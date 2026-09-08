//! Material substrate — "solid what", not just solid. LANDSCAPE_200.md §A.
//!
//! Materials are DERIVED from depth below the original surface + a few noise
//! fields, never stored. Same trick that keeps terrain free (item 1).
//! Eight materials fit a nibble and cover every gameplay need for a year (item 2).

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
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct MatInfo {
    pub name: &'static str,
    pub hardness: f32, // dig cost multiplier (item 6)
    pub cohesion: f32, // collapse resistance (item 7)
    pub albedo: [f32; 3],
    pub roughness: f32,
}

pub const PALETTE: [MatInfo; 9] = [
    MatInfo {
        name: "regolith",
        hardness: 1.0,
        cohesion: 0.35,
        albedo: [0.42, 0.34, 0.24],
        roughness: 0.85,
    },
    MatInfo {
        name: "sediment",
        hardness: 0.7,
        cohesion: 0.25,
        albedo: [0.38, 0.30, 0.18],
        roughness: 0.80,
    },
    MatInfo {
        name: "clay",
        hardness: 1.2,
        cohesion: 0.55,
        albedo: [0.40, 0.28, 0.20],
        roughness: 0.70,
    },
    MatInfo {
        name: "sandstone",
        hardness: 2.4,
        cohesion: 0.70,
        albedo: [0.55, 0.42, 0.28],
        roughness: 0.65,
    },
    MatInfo {
        name: "basalt",
        hardness: 4.0,
        cohesion: 0.90,
        albedo: [0.22, 0.22, 0.24],
        roughness: 0.55,
    },
    MatInfo {
        name: "ferrous ore",
        hardness: 3.2,
        cohesion: 0.80,
        albedo: [0.35, 0.22, 0.16],
        roughness: 0.50,
    },
    MatInfo {
        name: "ice",
        hardness: 1.5,
        cohesion: 0.40,
        albedo: [0.72, 0.80, 0.88],
        roughness: 0.25,
    },
    MatInfo {
        name: "structural alloy",
        hardness: 99.0,
        cohesion: 1.0,
        albedo: [0.48, 0.50, 0.54],
        roughness: 0.40,
    },
    MatInfo {
        name: "sand",
        hardness: 0.45,
        cohesion: 0.12,
        albedo: [0.72, 0.62, 0.42],
        roughness: 0.90,
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
    let near_shore = elev > hab.water_level - 1.0
        && elev < hab.water_level + 3.5
        && slope < 0.32
        && below < 2.5;
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
