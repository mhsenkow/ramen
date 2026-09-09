//! Forest measurement — the baseline stage 0 of `FOREST_IMPLEMENTATION_PLAN.md`.
//!
//! Nothing here changes the world. It reports what the current generator
//! actually produces, so the later stages have numbers to move rather than
//! impressions to argue about. The plan asks specifically for a size histogram
//! and for spacing to be an outcome rather than an assumption, and it warns
//! that "a denser screenshot alone does not meet the goal".
//!
//! These same readings are the gate for stages 2–3: community suitability,
//! spacing, crown cover and height histograms have to stay inside authored
//! bands across several seeds.

use crate::habitat::Habitat;
use crate::plant::PlantSim;
use crate::tree_form;

/// Height classes, metres. Chosen to match the plan's vocabulary: seedlings,
/// saplings, poles, mature canopy, elders.
pub const HEIGHT_BANDS: [f32; 6] = [1.0, 3.0, 7.0, 14.0, 26.0, 40.0];

/// Nearest-neighbour distance classes, metres. Trunk collision, thicket,
/// closed stand, open woodland, scattered, isolated.
pub const SPACING_BANDS: [f32; 6] = [1.0, 3.0, 6.0, 12.0, 25.0, 60.0];

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub alive: u32,
    /// Count per species/form id 0–7.
    pub species: [u32; 8],
    /// Count per `HEIGHT_BANDS` bucket, plus one overflow bucket.
    pub height: [u32; 7],
    /// Count per `SPACING_BANDS` bucket, plus one overflow bucket.
    pub spacing: [u32; 7],
    pub mean_height: f32,
    pub max_height: f32,
    /// How many organisms sit within 2% of their species' height clamp. The
    /// plan calls this out: seeded biomass plus a clamp can pile most of the
    /// population onto one maximum, which reads as a plantation.
    pub at_clamp: u32,
    /// Nearest-neighbour trunk distance below the sum of the two trunk radii —
    /// i.e. trunks occupying one another. Crown overlap is fine; this is not.
    pub trunk_overlaps: u32,
    pub mean_spacing_m: f32,
}

fn band(v: f32, bands: &[f32; 6]) -> usize {
    for (i, b) in bands.iter().enumerate() {
        if v < *b {
            return i;
        }
    }
    6
}

/// Resolved height of one organism, through the same path the renderer uses.
pub fn height_of(p: &crate::plant::Plant, species: u8) -> f32 {
    tree_form::dimensions(p, species)[1] * tree_form::HEIGHT
}

/// Measure the standing population. `O(n)` plus a uniform grid for neighbours,
/// because 22k plants all-pairs is a quarter of a billion comparisons and the
/// plan forbids all-pairs queries in the ecology path.
pub fn stats(plants: &PlantSim, hab: &Habitat) -> Stats {
    let mut st = Stats::default();

    // Bucket by arc-metres x axial-metres so wrap is a simple modulo.
    const CELL: f32 = 24.0;
    let arc = std::f32::consts::TAU * hab.radius;
    let nt = (arc / CELL).ceil().max(1.0) as i32;
    let nz = (hab.length / CELL).ceil().max(1.0) as i32;
    let key = |th: f32, z: f32| -> (i32, i32) {
        let a = (th.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * arc / CELL).floor()
            as i32;
        let b = ((z / hab.length + 0.5) * nz as f32).floor() as i32;
        (a.rem_euclid(nt), b.clamp(0, nz - 1))
    };

    let mut grid: std::collections::HashMap<(i32, i32), Vec<usize>> =
        std::collections::HashMap::new();
    for (i, p) in plants.plants.iter().enumerate() {
        if !p.alive {
            continue;
        }
        grid.entry(key(p.theta, p.z)).or_default().push(i);
    }

    let mut h_sum = 0.0f64;
    let mut s_sum = 0.0f64;
    let mut s_n = 0u32;
    for (i, p) in plants.plants.iter().enumerate() {
        if !p.alive {
            continue;
        }
        st.alive += 1;
        let sp = (p.species as usize).min(7);
        st.species[sp] += 1;

        let h = height_of(p, sp as u8);
        h_sum += h as f64;
        st.max_height = st.max_height.max(h);
        st.height[band(h, &HEIGHT_BANDS)] += 1;
        // Within 2% of what this species can reach at full biomass.
        let ceiling = species_height_ceiling(sp as u8);
        if h >= ceiling * 0.98 {
            st.at_clamp += 1;
        }

        // Nearest neighbour over the 3x3 cell neighbourhood.
        let (a, b) = key(p.theta, p.z);
        let mut best = f32::MAX;
        let mut best_i = usize::MAX;
        for da in -1..=1 {
            for db in -1..=1 {
                let k = ((a + da).rem_euclid(nt), (b + db).clamp(0, nz - 1));
                let Some(list) = grid.get(&k) else { continue };
                for &j in list {
                    if j == i {
                        continue;
                    }
                    let q = &plants.plants[j];
                    let dth = {
                        let x = (q.theta - p.theta).rem_euclid(std::f32::consts::TAU);
                        x.min(std::f32::consts::TAU - x) * hab.radius
                    };
                    let dz = q.z - p.z;
                    let d2 = dth * dth + dz * dz;
                    if d2 < best {
                        best = d2;
                        best_i = j;
                    }
                }
            }
        }
        if best_i != usize::MAX {
            let d = best.sqrt();
            st.spacing[band(d, &SPACING_BANDS)] += 1;
            s_sum += d as f64;
            s_n += 1;
            let need = crate::plant::trunk_radius_m(p)
                + crate::plant::trunk_radius_m(&plants.plants[best_i]);
            if d < need {
                st.trunk_overlaps += 1;
            }
        }
    }
    if st.alive > 0 {
        st.mean_height = (h_sum / st.alive as f64) as f32;
    }
    if s_n > 0 {
        st.mean_spacing_m = (s_sum / s_n as f64) as f32;
    }
    st
}

/// Tallest this species can get, from `tree_form::dimensions`' own clamps.
fn species_height_ceiling(species: u8) -> f32 {
    match species {
        3 | 4 => 3.8,
        5 => 12.0,
        6 => 28.0,
        7 => 42.0,
        _ => 36.0,
    }
}

/// One-line summary for benchmarks and route captures.
pub fn summary(st: &Stats) -> String {
    format!(
        "alive {} | species {:?} | height {:?} | spacing {:?} | mean h {:.1} m \
         max {:.1} m | at-clamp {} ({:.0}%) | mean nn {:.1} m | trunk overlaps {}",
        st.alive,
        st.species,
        st.height,
        st.spacing,
        st.mean_height,
        st.max_height,
        st.at_clamp,
        if st.alive > 0 {
            st.at_clamp as f32 / st.alive as f32 * 100.0
        } else {
            0.0
        },
        st.mean_spacing_m,
        st.trunk_overlaps,
    )
}
