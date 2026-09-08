//! Province zoning — engineered landform archetypes on the θ-torus.
//! LANDSCAPE_4200.md §BM. Query-time only; never a stored painted map.

use crate::habitat::Habitat;

/// Lattice resolution around / along the drum (integer so θ wraps exactly).
pub const NT_PROV: usize = 16;
pub const NZ_PROV: usize = 10;

/// Default blend width in metres.
pub const BLEND_M: f32 = 280.0;

pub mod id {
    pub const MASSIF: u8 = 0;
    pub const PLATEAU: u8 = 1;
    pub const BADLANDS: u8 = 2;
    pub const MEADOW: u8 = 3;
    pub const SWAMP_BASIN: u8 = 4;
    pub const DUNE_SEA: u8 = 5;
    pub const SEA_BASIN: u8 = 6;
    pub const KARST: u8 = 7;
    pub const ENDCAP_WALL: u8 = 8;
    /// Engineered alluvial farm ring — grid paddies, terrace growth.
    pub const FARMLAND: u8 = 9;
    /// Engineered urban corridor — stepped pads, block grid, townships.
    pub const CITY: u8 = 10;
    pub const COUNT: usize = 11;
}

#[derive(Clone, Copy, Debug)]
pub struct ProvinceSample {
    pub primary: u8,
    pub secondary: u8,
    pub w0: f32,
    pub w1: f32,
}

impl ProvinceSample {
    #[inline]
    pub fn weight(&self, arch: u8) -> f32 {
        if arch == self.primary {
            self.w0
        } else if arch == self.secondary {
            self.w1
        } else {
            0.0
        }
    }

    #[inline]
    pub fn blend2(&self, a: f32, b: f32) -> f32 {
        let sum = (self.w0 + self.w1).max(1e-6);
        let t = (self.w1 / sum).clamp(0.0, 0.5);
        a + (b - a) * t
    }
}

pub fn name(id: u8) -> &'static str {
    match id {
        id::MASSIF => "massif",
        id::PLATEAU => "plateau",
        id::BADLANDS => "badlands",
        id::MEADOW => "meadow",
        id::SWAMP_BASIN => "swamp basin",
        id::DUNE_SEA => "dune sea",
        id::SEA_BASIN => "sea basin",
        id::KARST => "karst",
        id::ENDCAP_WALL => "endcap wall",
        id::FARMLAND => "farmland",
        id::CITY => "city",
        _ => "unknown",
    }
}

pub fn color(id: u8) -> [f32; 3] {
    match id {
        id::MASSIF => [0.45, 0.42, 0.40],
        id::PLATEAU => [0.55, 0.48, 0.32],
        id::BADLANDS => [0.62, 0.40, 0.28],
        id::MEADOW => [0.28, 0.55, 0.24],
        id::SWAMP_BASIN => [0.18, 0.36, 0.28],
        id::DUNE_SEA => [0.72, 0.58, 0.34],
        id::SEA_BASIN => [0.12, 0.28, 0.42],
        id::KARST => [0.50, 0.52, 0.46],
        id::ENDCAP_WALL => [0.58, 0.56, 0.62],
        id::FARMLAND => [0.38, 0.52, 0.18],
        id::CITY => [0.72, 0.70, 0.66],
        _ => [0.5, 0.5, 0.5],
    }
}

/// Climate intent: 0=arid 1=mesic 2=humid 3=marine.
#[inline]
pub fn climate_intent(arch: u8) -> u8 {
    match arch {
        id::DUNE_SEA | id::BADLANDS => 0,
        id::MEADOW | id::PLATEAU | id::KARST | id::ENDCAP_WALL | id::FARMLAND | id::CITY => 1,
        id::SWAMP_BASIN | id::MASSIF => 2,
        id::SEA_BASIN => 3,
        _ => 1,
    }
}

/// Target relief amplitude as a fraction of `max_elevation`.
pub fn relief_amp(arch: u8) -> f32 {
    match arch {
        id::MASSIF => 0.95,
        id::PLATEAU => 0.55,
        id::BADLANDS => 0.48,
        id::MEADOW => 0.18,
        id::SWAMP_BASIN => 0.08,
        id::DUNE_SEA => 0.12,
        id::SEA_BASIN => 0.05,
        id::KARST => 0.42,
        id::ENDCAP_WALL => 1.05,
        id::FARMLAND => 0.06,
        id::CITY => 0.14,
        _ => 0.4,
    }
}

#[inline]
fn hash_u(ci: i32, cj: i32, seed: u32) -> u32 {
    let mut h = seed
        .wrapping_add((ci as u32).wrapping_mul(0x9E3779B1))
        .wrapping_add((cj as u32).wrapping_mul(0x85EBCA77));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A2D39);
    h ^= h >> 15;
    h
}

#[inline]
fn hash01(ci: i32, cj: i32, seed: u32) -> f32 {
    (hash_u(ci, cj, seed) & 0x00FF_FFFF) as f32 / 16_777_215.0
}

/// Discoverable engineered bands (~12–20% of mid-habitat): one axial farm
/// ring wrapping θ, cut by a helical city corridor along z. City wins at
/// intersections so paddies and pads criss-cross like an O'Neill painting.
fn engineered_band(ci: usize, cj: usize, seed: u32) -> Option<u8> {
    let z_norm = cj as f32 / (NZ_PROV - 1).max(1) as f32;
    if z_norm < 0.16 || z_norm > 0.84 {
        return None;
    }

    // Axial farm ring — full circumference, ~1 cell thick (~10% of length).
    let farm_c = 0.36 + hash01(1, 1, seed ^ 0xFA12_B4D0) * 0.28;
    let in_farm = (z_norm - farm_c).abs() < 0.055;
    // Occasional second thinner ring farther along the drum.
    let farm2_on = hash01(1, 2, seed ^ 0xFA12_B4D0) > 0.42;
    let farm_c2 = 0.58 + hash01(1, 3, seed ^ 0xFA12_B4D0) * 0.16;
    let in_farm2 = farm2_on && (z_norm - farm_c2).abs() < 0.04;

    // Helical city corridor — one strip around θ that drifts with z.
    let period = 7 + (hash_u(2, 0, seed ^ 0xC170_00B4) % 2) as usize; // 7–8
    let pitch = 1 + (hash_u(3, 0, seed ^ 0xC170_00B4) % 2) as i32; // 1–2
    let phase = (hash_u(4, 0, seed ^ 0xC170_00B4) % period as u32) as i32;
    let helix = (ci as i32 + cj as i32 * pitch - phase).rem_euclid(period as i32);
    let in_city = helix == 0;

    // City cuts through farmland at crossings — the interesting intersection.
    if in_city {
        return Some(id::CITY);
    }
    if in_farm || in_farm2 {
        // Ribs still get massifs sometimes; leave those cells organic.
        let rib = (ci % 4) == 0;
        let r = hash01(ci as i32, cj as i32, seed ^ 0xFA12_B4D0);
        if rib && r > 0.55 {
            return None;
        }
        return Some(id::FARMLAND);
    }
    None
}

fn pick_archetype(ci: usize, cj: usize, seed: u32) -> u8 {
    let z_norm = cj as f32 / (NZ_PROV - 1).max(1) as f32;
    if z_norm < 0.12 || z_norm > 0.88 {
        return id::ENDCAP_WALL;
    }
    if let Some(eng) = engineered_band(ci, cj, seed) {
        return eng;
    }
    let mid = (z_norm - 0.5).abs() < 0.28;
    let r = hash01(ci as i32, cj as i32, seed ^ 0xA11CE5);
    // Rib-aligned massifs: every ~2 cells around.
    let rib = (ci % 4) == 0;
    if rib && r > 0.35 && r < 0.72 {
        return id::MASSIF;
    }
    if mid {
        if r < 0.22 {
            return id::SEA_BASIN;
        }
        if r < 0.34 {
            return id::SWAMP_BASIN;
        }
        if r < 0.46 {
            return id::DUNE_SEA;
        }
        if r < 0.58 {
            return id::MEADOW;
        }
        if r < 0.68 {
            return id::PLATEAU;
        }
        if r < 0.76 {
            return id::BADLANDS;
        }
        if r < 0.84 {
            return id::KARST;
        }
        return id::MEADOW;
    }
    // Between mid and endcap: more plateau / massif / meadow.
    if r < 0.18 {
        return id::MASSIF;
    }
    if r < 0.34 {
        return id::PLATEAU;
    }
    if r < 0.48 {
        return id::MEADOW;
    }
    if r < 0.58 {
        return id::BADLANDS;
    }
    if r < 0.68 {
        return id::DUNE_SEA;
    }
    if r < 0.78 {
        return id::KARST;
    }
    id::MEADOW
}

/// Jittered cell centre in continuous (t_cell, z_cell) space where
/// t ∈ [0, NT_PROV) wraps and z ∈ [0, NZ_PROV].
fn cell_centre(ci: usize, cj: usize, seed: u32) -> (f32, f32) {
    let jx = (hash01(ci as i32, cj as i32, seed ^ 0xC01) - 0.5) * 0.45;
    let jy = (hash01(ci as i32, cj as i32, seed ^ 0xC02) - 0.5) * 0.45;
    (ci as f32 + 0.5 + jx, cj as f32 + 0.5 + jy)
}

fn toroidal_dt(a: f32, b: f32, period: f32) -> f32 {
    let mut d = (a - b).rem_euclid(period);
    if d > period * 0.5 {
        d -= period;
    }
    d
}

/// Query province blend at world cylindrical coordinates.
pub fn province_at(hab: &Habitat, theta: f32, z: f32) -> ProvinceSample {
    province_at_seed(hab, theta, z, hab.seed)
}

pub fn province_at_seed(hab: &Habitat, theta: f32, z: f32, seed: u32) -> ProvinceSample {
    let seed = seed ^ 0xA11CE5;
    let t = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT_PROV as f32;
    let zz = ((z / hab.length) + 0.5).clamp(0.0, 0.999) * NZ_PROV as f32;

    let cell_m_t = std::f32::consts::TAU * hab.radius / NT_PROV as f32;
    let cell_m_z = hab.length / NZ_PROV as f32;
    let blend = BLEND_M * (hab.radius / 900.0).clamp(0.75, 1.5);
    let blend_cells_t = (blend / cell_m_t).max(0.55);
    let blend_cells_z = (blend / cell_m_z).max(0.55);

    // Soft endcap boost toward EndcapWall near seals.
    let z_norm = ((z / hab.length) + 0.5).clamp(0.0, 1.0);
    let end_w = {
        let d = (z_norm - 0.5).abs() * 2.0;
        ((d - 0.84) / 0.16).clamp(0.0, 1.0)
    };

    let mut best: [(f32, u8); 2] = [(f32::MAX, id::MEADOW), (f32::MAX, id::MEADOW)];

    // Search neighbourhood of cells.
    let ci0 = t.floor() as i32;
    let cj0 = zz.floor() as i32;
    for dj in -2..=2 {
        for di in -2..=2 {
            let ci = (ci0 + di).rem_euclid(NT_PROV as i32) as usize;
            let cj = (cj0 + dj).clamp(0, NZ_PROV as i32 - 1) as usize;
            let (cx, cy) = cell_centre(ci, cj, seed);
            let dt = toroidal_dt(t, cx, NT_PROV as f32) / blend_cells_t;
            let dz = (zz - cy) / blend_cells_z;
            let dist2 = dt * dt + dz * dz;
            let mut arch = pick_archetype(ci, cj, seed);
            if end_w > 0.55 && arch != id::ENDCAP_WALL && hash01(ci as i32, cj as i32, seed) < end_w
            {
                arch = id::ENDCAP_WALL;
            }
            if dist2 < best[0].0 {
                best[1] = best[0];
                best[0] = (dist2, arch);
            } else if dist2 < best[1].0 && arch != best[0].1 {
                best[1] = (dist2, arch);
            }
        }
    }

    // Inverse-distance weights (top two).
    let d0 = best[0].0.sqrt().max(1e-4);
    let d1 = best[1].0.sqrt().max(1e-4);
    let mut w0 = 1.0 / d0;
    let mut w1 = if best[1].1 == best[0].1 {
        0.0
    } else {
        1.0 / d1
    };
    if end_w > 0.01 {
        // Pull primary toward EndcapWall near seals.
        if best[0].1 != id::ENDCAP_WALL {
            w1 = w1.max(w0 * end_w * 0.85);
            // Keep primary; secondary becomes endcap if empty-ish.
            if best[1].1 != id::ENDCAP_WALL && end_w > 0.4 {
                best[1].1 = id::ENDCAP_WALL;
                w1 = w0 * end_w;
            }
        }
    }
    let sum = (w0 + w1).max(1e-6);
    w0 /= sum;
    w1 /= sum;
    // Ensure w0 >= w1.
    if w1 > w0 {
        let tmp_w = w0;
        w0 = w1;
        w1 = tmp_w;
        let tmp_a = best[0].1;
        best[0].1 = best[1].1;
        best[1].1 = tmp_a;
    }

    ProvinceSample {
        primary: best[0].1,
        secondary: if w1 > 1e-4 { best[1].1 } else { best[0].1 },
        w0,
        w1,
    }
}

pub fn dominant_at(hab: &Habitat, theta: f32, z: f32) -> u8 {
    province_at(hab, theta, z).primary
}

/// Count archetype hits over a regular sample grid.
pub fn census(hab: &Habitat, samples_t: usize, samples_z: usize) -> [u32; id::COUNT] {
    let mut counts = [0u32; id::COUNT];
    let st = samples_t.max(8);
    let sz = samples_z.max(4);
    for jz in 0..sz {
        let z = (jz as f32 / (sz - 1).max(1) as f32 - 0.5) * hab.length;
        for jt in 0..st {
            let theta = jt as f32 / st as f32 * std::f32::consts::TAU;
            let a = dominant_at(hab, theta, z) as usize;
            counts[a.min(id::COUNT - 1)] += 1;
        }
    }
    counts
}

/// Ensure the default seed has the landform set we care about (soft check).
pub fn guaranteed_present(counts: &[u32; id::COUNT]) -> bool {
    counts[id::MASSIF as usize] > 0
        && counts[id::MEADOW as usize] > 0
        && counts[id::SEA_BASIN as usize] > 0
        && counts[id::DUNE_SEA as usize] > 0
        && counts[id::SWAMP_BASIN as usize] > 0
        && counts[id::FARMLAND as usize] > 0
        && counts[id::CITY as usize] > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habitat::Habitat;

    #[test]
    fn wrap_continuous() {
        let hab = Habitat::kepler_drum();
        let a = province_at(&hab, 0.01, 0.0);
        let b = province_at(&hab, std::f32::consts::TAU - 0.01, 0.0);
        // Same or neighbouring archetypes near the seam.
        assert!(a.primary == b.primary || a.secondary == b.primary || a.primary == b.secondary);
    }

    #[test]
    fn determinism() {
        let hab = Habitat::kepler_drum();
        let c1 = census(&hab, 48, 24);
        let c2 = census(&hab, 48, 24);
        assert_eq!(c1, c2);
    }

    #[test]
    fn default_seed_has_variety() {
        let hab = Habitat::kepler_drum();
        let c = census(&hab, 64, 32);
        assert!(
            guaranteed_present(&c),
            "province census missing required archetypes: {:?}",
            c
        );
    }

    #[test]
    fn engineered_bands_discoverable_share() {
        let hab = Habitat::kepler_drum();
        let c = census(&hab, 96, 48);
        let total: u32 = c.iter().sum();
        let eng = c[id::FARMLAND as usize] + c[id::CITY as usize];
        let share = eng as f32 / total as f32;
        assert!(
            share > 0.06 && share < 0.28,
            "engineered share {:.1}% out of discoverable band (got farm={} city={})",
            share * 100.0,
            c[id::FARMLAND as usize],
            c[id::CITY as usize]
        );
        assert!(c[id::FARMLAND as usize] > 0 && c[id::CITY as usize] > 0);
    }
}
