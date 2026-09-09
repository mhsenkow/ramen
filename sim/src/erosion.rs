//! Live hydraulic erosion + talus. LANDSCAPE_200.md §F items 101–108.
//!
//! Generation already runs ~520k droplets once (`terrain.rs`). This tick is the
//! continuous sequel: a few thousand droplets per call, rainfall-weighted,
//! hardness-gated, deterministic from an LCG seed.

use crate::material;
use crate::terrain::{idx, NT, NZ};

/// Live erosion pass over the elevation grid.
pub struct Erosion;

impl Erosion {
    /// Run `droplets` hydraulic steps. Spawn sites are weighted by `rainfall`
    /// (empty → uniform). Harder material erodes less via `material_hardness_at`.
    /// Advances `seed` with the same LCG as terrain generation. Returns approx
    /// sediment mass moved (sum of absolute height transfers).
    pub fn tick(
        elev: &mut [f32],
        rainfall: &[f32],
        material_hardness_at: impl Fn(usize) -> f32,
        droplets: u32,
        seed: &mut u32,
    ) -> f32 {
        debug_assert_eq!(elev.len(), NT * NZ);
        let weighted = rainfall.len() == NT * NZ;
        let rmax = if weighted {
            rainfall.iter().cloned().fold(1e-6f32, f32::max)
        } else {
            1.0
        };

        let mut moved = 0.0f32;
        for _ in 0..droplets {
            let (mut px, mut py) = spawn(rainfall, weighted, rmax, seed);
            let (mut vx, mut vy, mut water, mut sed) = (0.0f32, 0.0f32, 1.0f32, 0.0f32);

            for _step in 0..36 {
                let (gx, gy, h) = grad(elev, px, py);
                vx = vx * 0.55 - gx * 1.4;
                vy = vy * 0.55 - gy * 1.4;
                let len = (vx * vx + vy * vy).sqrt();
                if len < 1e-4 {
                    break;
                }
                vx /= len;
                vy /= len;
                let (nx, ny) = (px + vx, py + vy);
                let nyc = ny.clamp(1.0, NZ as f32 - 2.0);
                let nh = sample(elev, nx, nyc);
                let dh = nh - h;

                let cell = idx(
                    (px.rem_euclid(NT as f32) as usize) % NT,
                    (py.clamp(0.0, (NZ - 1) as f32) as usize).min(NZ - 1),
                );
                let hard = material_hardness_at(cell).max(0.15);
                let erode_scale = 1.0 / hard;

                let capacity = (-dh).max(0.0) * water * 5.5 + 0.02;
                if sed > capacity || dh > 0.0 {
                    let drop = if dh > 0.0 {
                        sed.min(dh)
                    } else {
                        (sed - capacity) * 0.35
                    };
                    deposit(elev, px, py, drop);
                    sed -= drop;
                    moved += drop.abs();
                } else {
                    let take = ((capacity - sed) * 0.35 * erode_scale)
                        .min(-dh * 0.9 * erode_scale)
                        .max(0.0);
                    deposit(elev, px, py, -take);
                    sed += take;
                    moved += take;
                }
                water *= 0.985;
                px = nx.rem_euclid(NT as f32);
                py = nyc;
            }
        }
        moved
    }
}

/// Angle-of-repose slump (item 108, simplified). `angle_tan` is tan(repose);
/// each pass transfers excess height to lower 4-neighbours.
pub fn talus_relax(elev: &mut [f32], angle_tan: f32, passes: u32) {
    debug_assert_eq!(elev.len(), NT * NZ);
    let max_dh = angle_tan.max(1e-4);
    for _ in 0..passes {
        for z in 1..NZ - 1 {
            for t in 0..NT {
                let i = idx(t, z);
                let h = elev[i];
                for &(dt, dz) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let nt = (t as i32 + dt).rem_euclid(NT as i32) as usize;
                    let nz = (z as i32 + dz) as usize;
                    let ni = idx(nt, nz);
                    let diff = h - elev[ni];
                    if diff > max_dh {
                        let excess = (diff - max_dh) * 0.5;
                        elev[i] -= excess;
                        elev[ni] += excess;
                    }
                }
            }
        }
    }
}

/// Material-aware angle-of-repose slump, with the drum's Coriolis drift.
///
/// `talus_relax` above relaxes the whole grid at one fixed angle. Generation
/// still uses it — changing that would move every landform — but at runtime a
/// sand dune and a basalt cliff have no business slumping identically, and
/// `MatInfo::cohesion` had been authored per material since item 7 and read by
/// nothing at all. This resolves the angle per column from the same three
/// fields `surface_hardness` reads: sand lets go near 37°, clay holds to 62°,
/// rock stands past 70°.
///
/// **Drift.** Material sliding off a face is falling — moving outward — and in
/// a drum spinning about +z an outward velocity is turned anti-spinward by
/// `-2ω × v`, the same term the thrown-object integrator already applies.
/// Integrating it over a fall of `h` metres gives `(ω·g/3)·(2h/g)^1.5` of
/// sideways travel: 35 cm off a 5 m face, 2.8 m off a 20 m one, against a
/// 3.7 m cell. So the bias is a real length over a real cell width rather than
/// a tuned constant, and fine material carries while gravel barely does.
/// Scree in this habitat should lean, and it should lean one way.
///
/// Returns metres of material moved, so a caller can tell a quiet pass from a
/// busy one. Mass is conserved exactly: every transfer takes from one cell and
/// gives the identical amount to another, drift included — the bias changes
/// *where* it lands, never how much there is.
pub fn talus_relax_material(
    elev: &mut [f32],
    flux: &[f32],
    max_elev: f32,
    hab_radius: f32,
    omega: f32,
    passes: u32,
) -> f32 {
    debug_assert_eq!(elev.len(), NT * NZ);
    debug_assert_eq!(flux.len(), NT * NZ);
    const G: f32 = 9.81;
    let cell_w = (std::f32::consts::TAU * hab_radius / NT as f32).max(1e-3);
    let mut moved = 0.0f32;
    const NB: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    // Jacobi, not Gauss-Seidel: every delta this pass is computed from the
    // heights the pass STARTED with, then applied together.
    //
    // Updating in place makes the sweep direction physical. A cell's -theta
    // neighbour has already been visited this pass and its +theta neighbour has
    // not, so material leans the way the loop runs — a 12% anti-spinward lean
    // with the spin switched off, which is indistinguishable from the Coriolis
    // term this function is trying to measure. (The fixed-angle `talus_relax`
    // above has the same bias; generation has been quietly leaning for a while.)
    let mut delta = vec![0.0f32; NT * NZ];
    for _ in 0..passes {
        delta.iter_mut().for_each(|d| *d = 0.0);
        for z in 1..NZ - 1 {
            for t in 0..NT {
                let i = idx(t, z);
                let f = flux[i];
                let h = elev[i];
                let repose = material::surface_repose(h, f, max_elev).max(1e-3);
                let fine = material::surface_fines(h, f, max_elev);

                let mut give = [0.0f32; 4];
                let mut total = 0.0f32;
                let mut lowest = h;
                for (k, &(dt, dz)) in NB.iter().enumerate() {
                    let nt = (t as i32 + dt).rem_euclid(NT as i32) as usize;
                    let nz = (z as i32 + dz) as usize;
                    let nh = elev[idx(nt, nz)];
                    let diff = h - nh;
                    if diff <= repose {
                        continue;
                    }
                    let mut excess = (diff - repose) * 0.5;
                    if dt != 0 {
                        let drift = (omega * G / 3.0) * (2.0 * diff / G).powf(1.5);
                        let bias = (fine * drift / cell_w).clamp(0.0, 0.9);
                        // -theta is anti-spinward: the side the drop favours.
                        excess *= if dt < 0 { 1.0 + bias } else { 1.0 - bias };
                    }
                    give[k] = excess;
                    total += excess;
                    lowest = lowest.min(nh);
                }
                if total <= 0.0 {
                    continue;
                }
                // Never shed so much that the cell drops under what it fed.
                let cap = ((h - lowest) * 0.5).max(0.0);
                let scale = if total > cap { cap / total } else { 1.0 };
                for (k, &(dt, dz)) in NB.iter().enumerate() {
                    if give[k] <= 0.0 {
                        continue;
                    }
                    let nt = (t as i32 + dt).rem_euclid(NT as i32) as usize;
                    let nz = (z as i32 + dz) as usize;
                    let amount = give[k] * scale;
                    delta[i] -= amount;
                    delta[idx(nt, nz)] += amount;
                    moved += amount;
                }
            }
        }
        for (e, d) in elev.iter_mut().zip(delta.iter()) {
            *e += *d;
        }
    }
    moved
}

#[inline]
fn lcg(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed
}

fn spawn(rainfall: &[f32], weighted: bool, rmax: f32, seed: &mut u32) -> (f32, f32) {
    if !weighted {
        let px = ((lcg(seed) >> 8) % NT as u32) as f32;
        let py = ((lcg(seed) >> 8) % NZ as u32) as f32;
        return (px, py);
    }
    // Rejection sample against the rainfall field; fall back to uniform.
    for _ in 0..48 {
        let ti = ((lcg(seed) >> 8) % NT as u32) as usize;
        let zi = ((lcg(seed) >> 8) % NZ as u32) as usize;
        let r = rainfall[idx(ti, zi)] / rmax;
        let u = ((lcg(seed) >> 8) & 0xFFFF) as f32 / 65535.0;
        if u <= r {
            return (ti as f32, zi as f32);
        }
    }
    let px = ((lcg(seed) >> 8) % NT as u32) as f32;
    let py = ((lcg(seed) >> 8) % NZ as u32) as f32;
    (px, py)
}

#[inline]
fn wrap_t(x: f32) -> f32 {
    x.rem_euclid(NT as f32)
}

fn sample(e: &[f32], x: f32, y: f32) -> f32 {
    let x = wrap_t(x);
    let y = y.clamp(0.0, NZ as f32 - 1.001);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let x1 = (x0 + 1) % NT;
    let y1 = (y0 + 1).min(NZ - 1);
    let a = e[idx(x0, y0)];
    let b = e[idx(x1, y0)];
    let c = e[idx(x0, y1)];
    let d = e[idx(x1, y1)];
    let t = a + (b - a) * fx;
    let u = c + (d - c) * fx;
    t + (u - t) * fy
}

fn grad(e: &[f32], x: f32, y: f32) -> (f32, f32, f32) {
    let h = sample(e, x, y);
    let gx = (sample(e, x + 1.0, y) - sample(e, x - 1.0, y)) * 0.5;
    let gy = (sample(e, x, y + 1.0) - sample(e, x, y - 1.0)) * 0.5;
    (gx, gy, h)
}

fn deposit(e: &mut [f32], x: f32, y: f32, amt: f32) {
    let x = wrap_t(x);
    let y = y.clamp(0.0, NZ as f32 - 1.001);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let x1 = (x0 + 1) % NT;
    let y1 = (y0 + 1).min(NZ - 1);
    e[idx(x0, y0)] += amt * (1.0 - fx) * (1.0 - fy);
    e[idx(x1, y0)] += amt * fx * (1.0 - fy);
    e[idx(x0, y1)] += amt * (1.0 - fx) * fy;
    e[idx(x1, y1)] += amt * fx * fy;
}

#[cfg(test)]
mod talus_tests {
    use super::*;

    /// A cone of material on an otherwise flat grid, plus a uniform flux so the
    /// surface proxy resolves to a chosen material.
    fn cone(peak: f32, flux_val: f32) -> (Vec<f32>, Vec<f32>) {
        let mut elev = vec![0.0f32; NT * NZ];
        let flux = vec![flux_val; NT * NZ];
        let (ct, cz) = (NT / 2, NZ / 2);
        for dz in -6i32..=6 {
            for dt in -6i32..=6 {
                let d = ((dt * dt + dz * dz) as f32).sqrt();
                if d > 6.0 {
                    continue;
                }
                let t = (ct as i32 + dt) as usize;
                let z = (cz as i32 + dz) as usize;
                elev[idx(t, z)] = peak * (1.0 - d / 6.0);
            }
        }
        (elev, flux)
    }

    /// Talus moves material; it must never create or destroy it.
    #[test]
    fn talus_conserves_material() {
        let (mut elev, flux) = cone(60.0, 0.1);
        let before: f64 = elev.iter().map(|&x| x as f64).sum();
        let moved = talus_relax_material(&mut elev, &flux, 440.0, 900.0, 0.104, 3);
        let after: f64 = elev.iter().map(|&x| x as f64).sum();
        assert!(moved > 0.0, "a 60 m cone should slump");
        assert!(
            (after - before).abs() / before.abs().max(1.0) < 1e-4,
            "talus changed total elevation by {:.4} m (from {before:.1})",
            after - before
        );
    }

    /// The point of the whole change: what the ground is made of decides how
    /// steeply it stands. Channel clay should hold a cone that high ground's
    /// sandstone holds even better, and loose sediment should hold least.
    #[test]
    fn softer_ground_slumps_further() {
        let spread = |flux_val: f32, max_e: f32| {
            let (mut elev, flux) = cone(60.0, flux_val);
            talus_relax_material(&mut elev, &flux, max_e, 900.0, 0.104, 4);
            // Peak height left standing after relaxation.
            elev.iter().cloned().fold(f32::MIN, f32::max)
        };
        // flux 0.4 -> sediment (repose ~44 deg); flux 0.7 -> clay (~62 deg).
        let sediment_peak = spread(0.40, 440.0);
        let clay_peak = spread(0.70, 440.0);
        assert!(
            clay_peak > sediment_peak,
            "clay ({clay_peak:.1} m) should hold a taller cone than sediment ({sediment_peak:.1} m)"
        );
    }

    /// Falling material is moving outward, and `-2w x v` turns outward motion
    /// anti-spinward. So a symmetric cone must shed asymmetrically: more
    /// material ends up on the -theta side than the +theta side.
    #[test]
    fn slump_drifts_anti_spinward() {
        let (mut elev, flux) = cone(60.0, 0.40);
        // A big omega exaggerates a real effect so the test is not reading noise.
        talus_relax_material(&mut elev, &flux, 440.0, 900.0, 0.6, 4);
        let (ct, cz) = (NT / 2, NZ / 2);
        let mut anti = 0.0f64; // -theta side
        let mut spin = 0.0f64; // +theta side
        for dz in -14i32..=14 {
            for dt in 1i32..=14 {
                let z = (cz as i32 + dz) as usize;
                let lo = ((ct as i32 - dt).rem_euclid(NT as i32)) as usize;
                let hi = ((ct as i32 + dt).rem_euclid(NT as i32)) as usize;
                anti += elev[idx(lo, z)] as f64;
                spin += elev[idx(hi, z)] as f64;
            }
        }
        assert!(
            anti > spin * 1.01,
            "expected an anti-spinward lean: -theta {anti:.2} vs +theta {spin:.2}"
        );
    }

    /// With no spin there is no Coriolis term, so the same cone must shed
    /// symmetrically. Guards against the bias leaking in from somewhere else.
    #[test]
    fn without_spin_the_slump_is_symmetric() {
        let (mut elev, flux) = cone(60.0, 0.40);
        talus_relax_material(&mut elev, &flux, 440.0, 900.0, 0.0, 4);
        let (ct, cz) = (NT / 2, NZ / 2);
        let mut anti = 0.0f64;
        let mut spin = 0.0f64;
        for dz in -14i32..=14 {
            for dt in 1i32..=14 {
                let z = (cz as i32 + dz) as usize;
                let lo = ((ct as i32 - dt).rem_euclid(NT as i32)) as usize;
                let hi = ((ct as i32 + dt).rem_euclid(NT as i32)) as usize;
                anti += elev[idx(lo, z)] as f64;
                spin += elev[idx(hi, z)] as f64;
            }
        }
        assert!(
            (anti - spin).abs() / anti.max(1.0) < 1e-3,
            "omega 0 should be symmetric: {anti:.3} vs {spin:.3}"
        );
    }
}
