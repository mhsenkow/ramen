//! Live hydraulic erosion + talus. LANDSCAPE_200.md §F items 101–108.
//!
//! Generation already runs ~520k droplets once (`terrain.rs`). This tick is the
//! continuous sequel: a few thousand droplets per call, rainfall-weighted,
//! hardness-gated, deterministic from an LCG seed.

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
