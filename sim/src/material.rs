//! Material substrate — "solid what", not just solid. LANDSCAPE_200.md §A.
//!
//! Materials are DERIVED from depth below the original surface + a few noise
//! fields, never stored. Same trick that keeps terrain free (item 1).
//! Ten materials still fit a nibble (item 2 budgeted eight; sand and coal each
//! earned their slot). Six free before the nibble has to grow.

use crate::noise::{fbm2, fbm3};
use crate::terrain::{Terrain, NT, NZ};

/// Material ids — stable for saves and HUD legends.
pub mod id {
    pub const REGOLITH: u8 = 0;
    pub const SEDIMENT: u8 = 1;
    pub const CLAY: u8 = 2;
    pub const SANDSTONE: u8 = 3;
    pub const BASALT: u8 = 4;
    pub const FERROUS: u8 = 5;
    pub const ICE: u8 = 6;
    pub const ALLOY: u8 = 7;
    pub const SAND: u8 = 8;
    /// Buried peat swamp, compressed. A reductant you dig instead of make.
    pub const COAL: u8 = 9;
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct MatInfo {
    pub name: &'static str,
    pub hardness: f32, // dig cost multiplier (item 6)
    pub cohesion: f32, // collapse resistance (item 7)
    pub albedo: [f32; 3],
    pub roughness: f32,
    /// Grain fineness, 0 coarse .. 1 rock flour. Drives how far material drifts
    /// when it moves (see `erosion::talus_relax_material`). Deliberately NOT
    /// derived from `cohesion`: clay is both the finest grain and the most
    /// cohesive, so the two are independent axes.
    pub fines: f32,
}

pub const PALETTE: [MatInfo; 10] = [
    MatInfo {
        name: "regolith",
        hardness: 1.0,
        cohesion: 0.35,
        albedo: [0.42, 0.34, 0.24],
        roughness: 0.85,
        fines: 0.35,
    },
    MatInfo {
        name: "sediment",
        hardness: 0.7,
        cohesion: 0.25,
        albedo: [0.38, 0.30, 0.18],
        roughness: 0.80,
        fines: 0.55,
    },
    MatInfo {
        name: "clay",
        hardness: 1.2,
        cohesion: 0.55,
        albedo: [0.40, 0.28, 0.20],
        roughness: 0.70,
        fines: 0.72,
    },
    MatInfo {
        name: "sandstone",
        hardness: 2.4,
        cohesion: 0.70,
        albedo: [0.55, 0.42, 0.28],
        roughness: 0.65,
        fines: 0.10,
    },
    MatInfo {
        name: "basalt",
        hardness: 4.0,
        cohesion: 0.90,
        albedo: [0.22, 0.22, 0.24],
        roughness: 0.55,
        fines: 0.05,
    },
    MatInfo {
        name: "ferrous ore",
        hardness: 3.2,
        cohesion: 0.80,
        albedo: [0.35, 0.22, 0.16],
        roughness: 0.50,
        fines: 0.08,
    },
    MatInfo {
        name: "ice",
        hardness: 1.5,
        cohesion: 0.40,
        albedo: [0.72, 0.80, 0.88],
        roughness: 0.25,
        fines: 0.12,
    },
    MatInfo {
        name: "structural alloy",
        hardness: 99.0,
        cohesion: 1.0,
        albedo: [0.48, 0.50, 0.54],
        roughness: 0.40,
        fines: 0.00,
    },
    MatInfo {
        name: "sand",
        hardness: 0.45,
        cohesion: 0.12,
        albedo: [0.72, 0.62, 0.42],
        roughness: 0.90,
        fines: 0.45,
    },
    MatInfo {
        name: "coal",
        hardness: 1.6,
        cohesion: 0.50,
        albedo: [0.09, 0.08, 0.09],
        roughness: 0.55,
        fines: 0.50,
    },
];

#[inline]
pub fn info(id: u8) -> MatInfo {
    PALETTE[(id as usize).min(PALETTE.len() - 1)]
}

/// Regolith depth from slope and drainage (item 3).
fn regolith_depth(slope: f32, flux: f32) -> f32 {
    // Thin on steep ground, thick where erosion deposited.
    let base = 1.8 + 4.5 * flux;
    base * (1.0 - slope * 0.85).max(0.15)
}

/// Sediment wedge thickness following basin depth (item 4).
fn sediment_depth(elev: f32, max_e: f32, flux: f32) -> f32 {
    let basin = (1.0 - (elev / (max_e * 0.55)).clamp(0.0, 1.0)).powf(1.4);
    basin * (3.0 + 8.0 * flux)
}

/// Material at a world point. Derived, not stored (item 1).
pub fn material_at(t: &Terrain, p: [f32; 3]) -> u8 {
    let hab = &t.hab;
    let (theta, z, r) = hab.to_cyl(p);
    let from_hull = hab.radius - r;

    // Structural alloy: bedrock shell + ribs (item 18).
    if from_hull < 8.0 {
        return id::ALLOY;
    }
    // Rib signature from generation — high angular frequency swell.
    let tf = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT as f32;
    let ribs = {
        let phase = (tf / NT as f32) * std::f32::consts::TAU * 9.0;
        phase.sin().abs().powf(3.0)
    };
    if ribs > 0.92 && from_hull < 22.0 {
        return id::ALLOY;
    }

    // Depth below the ORIGINAL surface — strata don't rewrite when you dig (item 1).
    let elev = t.elevation0(theta, z);
    let surf0 = hab.radius - elev;
    let below = (r - surf0).max(0.0);
    let flux = t.water_flux(theta, z);

    // Slope from elev neighbours.
    let d = 2.0;
    let slope = {
        let gx =
            (t.elevation(theta + d / hab.radius, z) - t.elevation(theta - d / hab.radius, z)).abs();
        let gz = (t.elevation(theta, z + d) - t.elevation(theta, z - d)).abs();
        ((gx + gz) / 4.0).clamp(0.0, 1.0)
    };

    // Beach / dune sand near waterline or in dune provinces (LANDSCAPE_4200).
    let prov = crate::province::province_at(hab, theta, z);
    let near_shore =
        elev > hab.water_level - 1.0 && elev < hab.water_level + 3.5 && slope < 0.32 && below < 2.5;
    let dune_w = prov.weight(crate::province::id::DUNE_SEA);
    if (near_shore || dune_w > 0.45) && below < 3.5 && from_hull > 8.0 {
        return id::SAND;
    }

    let reg_d = regolith_depth(slope, flux);
    let sed_d = sediment_depth(elev, hab.max_elevation, flux);

    // Ice lenses near endcaps at depth (item 11).
    let cap = {
        let d = (z / hab.length).abs() * 2.0;
        (d.max(0.72) - 0.72) / 0.28
    };
    if cap > 0.35 && below > 8.0 && below < 40.0 {
        let ice_n = fbm3(p[0] * 0.03, p[1] * 0.03, p[2] * 0.03, 3, hab.seed ^ 0x1CE);
        if ice_n > 0.62 - 0.2 * cap {
            return id::ICE;
        }
    }

    // Coal seams: buried peat. Coal wants the same ground clay does — a
    // drainage sink where organics settled — but *older*, so it sits under the
    // active sediment rather than in it. Flat-lying and banded, because a seam
    // is a bed: the depth test uses a low-frequency 2D field so it reads as a
    // layer you break into, not as blobs sprinkled through the rock.
    if below > sed_d + 1.5 && below < 46.0 && slope < 0.34 {
        let bed = fbm2(
            tf * 0.014,
            (z / hab.length) * NZ as f32 * 0.014,
            3,
            3,
            hab.seed ^ 0xC0A1,
        );
        // Paleo-drainage: today's flux is the best proxy the fields offer for
        // where water pooled long enough to bury a swamp.
        let swampy = flux * 0.6 + bed * 0.7;
        if swampy > 0.50 {
            let band = ((below - sed_d) * 0.42 + bed * 6.0).sin();
            if band > 0.05 {
                return id::COAL;
            }
        }
    }

    // Ore veins: capsules along a 3D noise flow field, biased to tunnels (items 9–10).
    if below > 4.0 {
        let s = 0.018;
        let v1 = fbm3(p[0] * s, p[1] * s, p[2] * s, 4, hab.seed ^ 0x0E01);
        let v2 = fbm3(
            p[0] * s + 17.0,
            p[1] * s - 9.0,
            p[2] * s + 3.0,
            4,
            hab.seed ^ 0x0E02,
        );
        let vein = ((v1 - 0.5).abs()).max((v2 - 0.5).abs());
        // Bias toward artifact tunnels: lower threshold near them.
        let near_tunnel = t.near_tunnel(p, 14.0);
        let thresh = if near_tunnel { 0.045 } else { 0.028 };
        if vein < thresh {
            return id::FERROUS;
        }
    }

    // Clay where drainage stalls: high flux + low slope (item 12).
    if below < reg_d + sed_d && flux > 0.55 && slope < 0.22 {
        let clay_n = fbm2(
            tf * 0.02,
            (z / hab.length) * NZ as f32 * 0.02,
            3,
            3,
            hab.seed ^ 0xC1A4,
        );
        if clay_n > 0.45 {
            return id::CLAY;
        }
    }

    if below < reg_d {
        return id::REGOLITH;
    }
    if below < reg_d + sed_d {
        return id::SEDIMENT;
    }

    // Bedrock banded by depth with low-frequency 3D warp (item 5).
    let warp = fbm3(
        p[0] * 0.008,
        p[1] * 0.008,
        p[2] * 0.008,
        3,
        hab.seed ^ 0x57A7,
    ) * 12.0;
    let band = below + warp;
    if band < reg_d + sed_d + 28.0 {
        id::SANDSTONE
    } else {
        id::BASALT
    }
}

/// Vertical column of materials for the geologist's readout (item 13).
pub fn strata_column(t: &Terrain, theta: f32, z: f32, steps: usize, step_m: f32) -> Vec<u8> {
    let surf = t.surface_radius(theta, z);
    let mut out = Vec::with_capacity(steps);
    for i in 0..steps {
        let r = surf + i as f32 * step_m;
        if r >= t.hab.radius {
            break;
        }
        let p = t.hab.to_world(theta, z, r);
        out.push(material_at(t, p));
    }
    out
}

pub fn name(id: u8) -> &'static str {
    info(id).name
}

pub fn albedo(id: u8) -> [f32; 3] {
    info(id).albedo
}

/// tan of the angle of repose — the steepest slope this material holds before
/// it slumps.
///
/// Derived from `cohesion`, which had been authored per material since item 7
/// and read by nothing. The mapping is `30° + cohesion · 58°`, which lands the
/// authored values on the real angles: sand 37°, sediment 44°, regolith 50°,
/// clay 62°, sandstone 71°, basalt 82°. Dry sand really does sit near 34° and
/// a basalt face really does stand near-vertical, so the ordering the table
/// already encoded turns out to be the physical one.
#[inline]
pub fn repose_tan(id: u8) -> f32 {
    let deg = 30.0 + info(id).cohesion.clamp(0.0, 1.0) * 58.0;
    deg.to_radians().tan()
}

/// How far material of this kind drifts when it moves, 0..1.
#[inline]
pub fn fines(id: u8) -> f32 {
    info(id).fines.clamp(0.0, 1.0)
}

/// Angle-of-repose proxy for a surface column, mirroring `surface_hardness`.
///
/// Talus runs over the whole elevation grid, so it cannot afford a
/// `material_at` world sample per cell. These are the same three fields the
/// hardness proxy reads, resolved to the material most likely exposed there.
pub fn surface_repose(elev0: f32, flux: f32, max_e: f32) -> f32 {
    let dry = repose_tan(surface_material(elev0, flux, max_e));
    // Saturated ground holds a SHALLOWER face, not a steeper one.
    //
    // Picking the material by drainage alone got this backwards: high flux
    // resolved to clay, and clay's cohesion is the highest of the soils, so
    // the wettest ground in the drum was modelled as the most stable. Water
    // fills the pore space, carries part of the load and cancels much of the
    // friction between grains — which is why saturated banks are exactly where
    // slumps happen. Cohesive soils lose the most, so the term scales with the
    // dry angle: clay at 62 degrees relaxes to about 40 when soaked, dry sand
    // at 37 barely moves.
    let wet = (flux - 0.35).max(0.0) / 0.65;
    dry * (1.0 - 0.42 * wet.clamp(0.0, 1.0))
}

/// Grain fineness proxy for a surface column.
pub fn surface_fines(elev0: f32, flux: f32, max_e: f32) -> f32 {
    fines(surface_material(elev0, flux, max_e))
}

/// The material a surface column most likely exposes, from fields alone.
///
/// Channels wash to clay, moderate drainage leaves sediment, high ground strips
/// to rock, and everything else is regolith — the same reading
/// `surface_hardness` has always made, named once so repose, fineness and
/// hardness cannot disagree about what is underfoot.
pub fn surface_material(elev0: f32, flux: f32, max_e: f32) -> u8 {
    let high = (elev0 / max_e.max(1.0)).clamp(0.0, 1.0);
    if flux > 0.55 {
        return id::CLAY;
    }
    if flux > 0.35 {
        return id::SEDIMENT;
    }
    if high > 0.55 {
        return id::SANDSTONE;
    }
    id::REGOLITH
}

/// Surface hardness proxy for live erosion (NEXT #7) — avoids per-droplet
/// `material_at` world samples. Soft in channels, harder on high ground.
pub fn surface_hardness(elev0: f32, flux: f32, max_e: f32) -> f32 {
    let high = (elev0 / max_e.max(1.0)).clamp(0.0, 1.0);
    if flux > 0.55 {
        return info(id::CLAY).hardness * 0.85 + info(id::SEDIMENT).hardness * 0.15;
    }
    if flux > 0.35 {
        return info(id::SEDIMENT).hardness;
    }
    if high > 0.55 {
        return info(id::SANDSTONE).hardness * 0.55 + info(id::REGOLITH).hardness * 0.45;
    }
    info(id::REGOLITH).hardness
}

/// Dig cost scale from hardness (item 6). Soft materials carve in one pass.
pub fn dig_scale(id: u8) -> f32 {
    let h = info(id).hardness;
    if h >= 50.0 {
        0.0
    } else {
        1.0 / h
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habitat::Habitat;
    use crate::terrain::Terrain;

    /// The drum is a shell with dirt on the inside of it, and the shell is the
    /// end of the world. Dig everything away and you should be standing on
    /// structural alloy, not falling through a hole into space.
    #[test]
    fn the_hull_cannot_be_dug_through() {
        let hab = Habitat::kepler_drum();
        let mut t = Terrain::generate(hab);
        let (th, z) = (0.7f32, 120.0f32);
        let surf = t.surface_radius(th, z);

        // Excavate the whole column, well past the hull, with a fat brush.
        let mut r = surf;
        while r < hab.radius + 20.0 {
            let p = hab.to_world(th, z, r);
            let up = hab.up_at(p);
            t.edits.add(p, 12.0, true, false, up);
            r += 6.0;
        }

        // Somewhere at or before the shell, the world must go solid again.
        let mut first_solid = None;
        let mut rr = surf - 5.0;
        while rr < hab.radius + 30.0 {
            if t.density(hab.to_world(th, z, rr)) > 0.0 {
                first_solid = Some(rr);
                break;
            }
            rr += 0.25;
        }
        let floor = first_solid.expect("dug clean through the hull into space");
        let from_hull = hab.radius - floor;
        assert!(
            from_hull <= 8.5,
            "floor is {from_hull:.2} m from the hull; the shell should stop a dig by 8 m"
        );
        assert!(
            from_hull > 0.0,
            "floor is outside the hull radius, which is not a place"
        );
        // And it must be the alloy that stopped it, not a stray rock.
        let p = hab.to_world(th, z, floor + 0.5);
        assert_eq!(
            material_at(&t, p),
            id::ALLOY,
            "the thing you cannot dig should be the structural shell"
        );
    }

    /// Repose has to order the way real materials do, because the whole point
    /// of deriving it from `cohesion` is that the authored ordering was already
    /// the physical one.
    #[test]
    fn repose_orders_sand_below_soil_below_rock() {
        let sand = repose_tan(id::SAND);
        let sediment = repose_tan(id::SEDIMENT);
        let regolith = repose_tan(id::REGOLITH);
        let clay = repose_tan(id::CLAY);
        let sandstone = repose_tan(id::SANDSTONE);
        let basalt = repose_tan(id::BASALT);
        assert!(
            sand < sediment
                && sediment < regolith
                && regolith < clay
                && clay < sandstone
                && sandstone < basalt,
            "repose out of order: sand {sand:.2} sed {sediment:.2} reg {regolith:.2} \
             clay {clay:.2} sst {sandstone:.2} bas {basalt:.2}"
        );
        // Dry sand really does sit near 34-37 degrees.
        let deg = sand.atan().to_degrees();
        assert!(
            (33.0..40.0).contains(&deg),
            "sand repose is {deg:.1} degrees; expected roughly 34-38"
        );
        // And a basalt face should be effectively a wall.
        assert!(
            repose_tan(id::BASALT).atan().to_degrees() > 75.0,
            "basalt should stand near-vertical"
        );
    }

    /// Wet ground slumps further than dry ground. The old reading did the
    /// opposite: drainage chose the material, high drainage chose clay, and
    /// clay is the most cohesive soil on the table — so the wettest banks were
    /// the most stable ground in the habitat.
    ///
    /// Compared within one material band on purpose. Flux does two things at
    /// once — it decides *what* the surface is (regolith, then sediment, then
    /// clay) and *how wet* it is — so comparing across a threshold measures
    /// both and proves neither. A clay channel bank legitimately holds steeper
    /// than a loose sediment slope even when the clay is the wetter of the two.
    #[test]
    fn saturated_ground_holds_a_shallower_face() {
        let max_e = 440.0;
        let e = 40.0;

        // Clay band (flux > 0.55): same material, more water.
        let clay_damp = surface_repose(e, 0.56, max_e);
        let clay_soaked = surface_repose(e, 1.00, max_e);
        assert!(
            clay_soaked < clay_damp,
            "soaked clay {clay_soaked:.3} should slump further than damp {clay_damp:.3}"
        );
        assert!(
            clay_soaked < repose_tan(id::CLAY),
            "soaked clay is not below dry clay's angle"
        );

        // Sediment band (0.35 < flux <= 0.55): same again.
        let sed_low = surface_repose(e, 0.36, max_e);
        let sed_high = surface_repose(e, 0.54, max_e);
        assert!(
            sed_high < sed_low,
            "wetter sediment {sed_high:.3} should slump further than {sed_low:.3}"
        );

        // A soaked bank should read as a slumping angle, not a cliff.
        let deg = clay_soaked.atan().to_degrees();
        assert!(
            (25.0..50.0).contains(&deg),
            "soaked clay sits at {deg:.1} degrees"
        );

        // Below the threshold nothing changes, so dry ground is untouched.
        assert!(
            (surface_repose(e, 0.0, max_e) - surface_repose(e, 0.3, max_e)).abs() < 1e-6,
            "dry ground should be unaffected by the wetness term"
        );
        assert!(
            (surface_repose(e, 0.0, max_e) - repose_tan(id::REGOLITH)).abs() < 1e-6,
            "dry ground should equal its material's own angle"
        );
    }

    /// Fineness is independent of cohesion — clay is the finest grain *and* the
    /// most cohesive. If these ever collapse into one axis, drift stops making
    /// sense.
    #[test]
    fn fineness_is_not_cohesion() {
        assert!(fines(id::CLAY) > fines(id::SAND), "clay is finer than sand");
        assert!(
            repose_tan(id::CLAY) > repose_tan(id::SAND),
            "clay also holds a steeper face than sand"
        );
        assert!(fines(id::BASALT) < 0.1, "basalt does not travel as dust");
        assert_eq!(fines(id::ALLOY), 0.0, "the hull does not erode");
    }
}
