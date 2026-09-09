//! One construction recipe for both distant tree meshes and harvestable cells.
//! Boxes are material volumes in a 14 m local frame, never decorative entities.
use crate::biome;
use crate::plant::Plant;
use crate::province;
use crate::terrain::Terrain;
use crate::weather::Weather;

pub const HEIGHT: f32 = 14.0;
#[derive(Clone, Copy, Debug)]
pub struct Part {
    pub center: [f32; 3],
    pub size: [f32; 3],
    pub kind: u8,
}
fn block(out: &mut Vec<Part>, center: [f32; 3], size: [f32; 3], kind: u8) {
    out.push(Part { center, size, kind });
}
fn limb(out: &mut Vec<Part>, a: [f32; 3], b: [f32; 3], width: f32) {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let n = ((d[0].abs().max(d[1].abs()).max(d[2].abs()) / (width * 0.7)).ceil() as usize).max(1);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        block(
            out,
            [a[0] + d[0] * t, a[1] + d[1] * t, a[2] + d[2] * t],
            [width; 3],
            1,
        );
    }
}
fn crown(out: &mut Vec<Part>, c: [f32; 3], w: f32, h: f32) {
    // Stepped shoulders, an overhanging lower bough and an offset new-growth cap.
    // Solid connected foliage gives the player blocks to cut tunnels through.
    block(out, c, [w, h * 0.58, w * 0.88], 2);
    block(
        out,
        [c[0] - w * 0.08, c[1] + h * 0.32, c[2] + w * 0.04],
        [w * 0.76, h * 0.40, w * 0.70],
        2,
    );
    block(
        out,
        [c[0] + w * 0.08, c[1] - h * 0.28, c[2] - w * 0.06],
        [w * 0.78, h * 0.32, w * 0.72],
        2,
    );
}
pub fn recipe(form: u8) -> Vec<Part> {
    let mut out = Vec::new();
    match form {
        0 => {
            // Layered spruce, exposed lower trunk and distinct whorls.
            block(&mut out, [0.0, 6.6, 0.0], [0.75, 13.2, 0.75], 1);
            for i in 0..6 {
                let y = 4.0 + i as f32 * 1.65;
                let w = 6.0 - i as f32 * 0.78;
                crown(&mut out, [0.12 * (i as f32).sin(), y, 0.0], w, 1.8);
                if i < 4 {
                    limb(
                        &mut out,
                        [0.0, y - 0.5, 0.0],
                        [w * 0.40, y - 0.25, 0.0],
                        0.42,
                    );
                }
            }
        }
        4 => {
            // Reed thicket; all stems are still material.
            for i in 0..5 {
                let a = i as f32 * 2.4;
                let x = a.cos() * 1.2;
                let z = a.sin() * 1.2;
                let h = 9.0 + (i % 3) as f32 * 1.7;
                block(&mut out, [x, h * 0.5, z], [0.30, h, 0.30], 1);
                block(&mut out, [x + 0.45, h * 0.7, z], [1.2, 2.8, 0.65], 2);
                block(&mut out, [x, h - 0.2, z], [0.55, 1.2, 0.55], 2);
            }
        }
        _ => {
            let (trunk, spread, top, branches) = match form {
                2 => (1.05, 4.0, 10.5, 6), // willow
                3 => (0.85, 3.5, 8.8, 4),  // scrub
                5 => (1.1, 3.1, 10.0, 5),  // orchard
                6 => (0.9, 4.8, 11.5, 5),  // umbrella / acacia
                7 => (1.65, 3.9, 11.8, 6),
                _ => (1.1, 3.3, 10.8, 5),
            };
            block(&mut out, [0.0, 3.4, 0.0], [trunk, 6.8, trunk], 1);
            // Buttresses and an asymmetric fork avoid the lollipop silhouette.
            for i in 0..3 {
                let a = i as f32 * 2.094;
                limb(
                    &mut out,
                    [a.cos() * trunk, 0.3, a.sin() * trunk],
                    [0.0, 2.3, 0.0],
                    trunk * 0.65,
                );
            }
            for i in 0..branches {
                let a = i as f32 * 2.399 + form as f32 * 0.31;
                let reach = spread * (0.72 + (i % 3) as f32 * 0.13);
                let end = [
                    a.cos() * reach,
                    top - 1.0 + (i % 3) as f32 * 0.8,
                    a.sin() * reach,
                ];
                limb(
                    &mut out,
                    [0.0, 4.8 + i as f32 * 0.20, 0.0],
                    end,
                    trunk * 0.60,
                );
                let width = if form == 6 {
                    4.8
                } else {
                    3.8 + (i % 2) as f32 * 0.55
                };
                crown(&mut out, end, width, if form == 6 { 1.8 } else { 3.5 });
                if form == 2 {
                    // Draping willow fingers, connected to each bough.
                    for j in 0..3 {
                        let q = a + j as f32 * 1.8;
                        block(
                            &mut out,
                            [end[0] + q.cos() * 1.4, end[1] - 2.0, end[2] + q.sin() * 1.4],
                            [0.9, 4.0, 0.9],
                            2,
                        );
                    }
                }
            }
            limb(
                &mut out,
                [0.0, 5.0, 0.0],
                [0.6, top + 0.8, -0.3],
                trunk * 0.65,
            );
            crown(&mut out, [0.6, top + 0.8, -0.3], 4.0, 2.8);
        }
    }
    out
}

/// Shared dimensions. No distance-dependent boost, lean, or random new seed.
pub fn dimensions(p: &Plant, form: u8) -> [f32; 3] {
    let h = match form {
        3 | 4 => (0.9 + p.stem * 3.0 + p.leaf * 1.5).clamp(0.7, 3.8),
        5 => (3.5 + p.stem * 14.0 + p.leaf * 4.0).clamp(3.0, 12.0),
        6 => (7.0 + p.stem * 28.0 + p.leaf * 8.0).clamp(6.0, 28.0),
        7 => (12.0 + p.stem * 42.0 + p.leaf * 12.0).clamp(10.0, 42.0),
        _ => (6.0 + p.stem * 38.0 + p.leaf * 10.0).clamp(5.5, 36.0),
    };
    let w = (0.80 + p.leaf * 0.35).clamp(0.75, 1.25);
    [h / HEIGHT * w, h / HEIGHT, h / HEIGHT * w]
}
/// Deterministic rotation from position.
///
/// `std::f32::consts::PI`, not a typed-out approximation: `world.gd` computes
/// the same yaw with GDScript's `PI` (an f64), under a comment promising the
/// two agree. A hand-written 3.1415927 makes that agreement a coincidence, and
/// it is the one deny-by-default clippy lint standing between this crate and a
/// clippy gate in CI.
pub fn yaw(theta: f32, z: f32) -> f32 {
    (theta * 127.1 + z * 0.013).sin() * std::f32::consts::PI
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_forms_have_both_materials_and_bounded_cost() {
        for form in 0..8 {
            let p = recipe(form);
            assert!(p.iter().any(|b| b.kind == 1));
            assert!(p.iter().any(|b| b.kind == 2));
            assert!(p.len() < 180, "recipe cost: {}", p.len());
            for b in p {
                assert!(b.size.iter().all(|v| v.is_finite() && *v > 0.0));
            }
        }
    }
}

pub fn pigment(form: u8) -> [f32; 3] {
    [
        [0.22, 0.36, 0.27],
        [0.34, 0.48, 0.23],
        [0.34, 0.46, 0.29],
        [0.42, 0.44, 0.26],
        [0.40, 0.48, 0.26],
        [0.35, 0.48, 0.24],
        [0.40, 0.44, 0.25],
        [0.24, 0.40, 0.29],
    ][form.min(7) as usize]
}

/// Which species establishes on this ground.
///
/// Resolved **once, at establishment**, and then stored on the organism (see
/// `Plant::species`). It used to be recomputed on every query, so a drifting
/// climate or a biome threshold could turn a standing oak into a pine — and a
/// tree's identity depended on when the camera last asked. The forest plan
/// requires persistent ecological identity: "weather or a biome threshold
/// cannot change an existing tree into another species."
///
/// The returned id doubles as the `recipe` shape family for now. Species and
/// form are separate concepts and will want separating later.
/// Map biome + climate + genome → plant form id (0–7) for Multimesh archetypes.
/// 0 conifer, 1 broadleaf, 2 willow, 3 scrub, 4 reed, 5 orchard, 6 acacia, 7 giant.
/// The species an organism actually is.
///
/// Seated once at establishment and stored, never re-derived from live
/// weather: a drifting climate must not turn a standing oak into a pine, and
/// a tree's identity must not depend on when the camera last asked (forest
/// plan, design rule 6). The fallback is for in-memory records that predate
/// seating; it degrades to the old query rather than panicking.
pub fn species_of(
    p: &crate::plant::Plant,
    bid: u8,
    weather: &crate::weather::Weather,
    ter: &crate::terrain::Terrain,
) -> u8 {
    if p.species != crate::plant::UNASSIGNED {
        return p.species;
    }
    species_at(bid, weather, ter, p.theta, p.z, p.genome_id)
}

pub fn species_at(
    bid: u8,
    weather: &Weather,
    ter: &Terrain,
    theta: f32,
    z: f32,
    genome: u32,
) -> u8 {
    let elev = ter.elevation(theta, z);
    let arid = weather.aridity_at(theta, z);
    let temp = weather.temp_at_elev(theta, z, elev);
    let g = genome;
    let form = (g % 8) as u8;
    let prov = province::province_at(&ter.hab, theta, z);
    if prov.weight(province::id::CITY) > 0.45 {
        // Street trees / plaza orphans — scrub + rare orchard.
        return if g % 5 == 0 { 5 } else { 3 };
    }
    if prov.weight(province::id::FARMLAND) > 0.45 {
        return 5; // orchard / crop-edge trees
    }
    match bid {
        biome::id::FOREST => {
            if elev > ter.hab.max_elevation * 0.42 || temp < 9.0 {
                0 // montane conifer
            } else if arid > 0.48 {
                if form % 3 == 0 {
                    6
                } else {
                    3
                } // dry acacia / scrub-forest edge
            } else {
                // Mesic stand: pine, oak, gallery, giant — genome picks the silhouette.
                match form {
                    0 | 1 => 0,
                    2 | 3 => 1,
                    4 => 6,
                    5 => 7,
                    6 => 0,
                    _ => 1,
                }
            }
        }
        biome::id::SWAMP | biome::id::WETLAND | biome::id::RIPARIAN => {
            if form % 5 == 0 {
                4
            } else if form % 3 == 0 {
                1
            } else {
                2
            }
        }
        biome::id::MEADOW => match form % 5 {
            0 => 5,
            1 => 6,
            2 => 0,
            _ => 1,
        },
        biome::id::SCRUB
        | biome::id::ALPINE
        | biome::id::BARE_ROCK
        | biome::id::DESERT
        | biome::id::DUNE => {
            if form == 7 {
                6
            } else {
                3
            }
        }
        biome::id::SHORE => 4,
        biome::id::FARM => 5,
        _ => {
            if arid > 0.55 {
                3
            } else if form == 5 {
                7
            } else if form % 4 == 0 {
                6
            } else {
                1
            }
        }
    }
}
