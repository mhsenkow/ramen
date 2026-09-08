//! Soil chemistry grid — half-res SoA over the terrain lattice.
//! LANDSCAPE_200.md §B items 21–25, 35.
//!
//! Sparse deltas (item 35) are the long-term plan, matching `Edits` for terrain.
//! Full grid for MVP is fine: 8 × f32 × 768 × 512 ≈ 12 MB.

use crate::habitat::Habitat;
use crate::noise::fbm2;
use crate::terrain::{NT, NZ};

/// Half of terrain resolution (NT=1536, NZ=1024).
pub const ST: usize = 768;
pub const SZ: usize = 512;

#[inline]
pub fn idx(t: usize, z: usize) -> usize {
    t + z * ST
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SoilSample {
    pub n: f32,
    pub p: f32,
    pub k: f32,
    pub organic: f32,
    pub moisture: f32,
    pub ph: f32,
    pub compaction: f32,
    pub microbes: f32,
}

pub struct Soil {
    pub n: Vec<f32>,
    pub p: Vec<f32>,
    pub k: Vec<f32>,
    pub organic: Vec<f32>,
    pub moisture: Vec<f32>,
    pub ph: Vec<f32>,
    pub compaction: Vec<f32>,
    pub microbes: Vec<f32>,
    hab: Habitat,
}

impl Soil {
    /// Initialise from parent-material heuristics on the elevation/flux grids.
    pub fn new(hab: Habitat, elev: &[f32], flux: &[f32], elev0: &[f32]) -> Self {
        debug_assert_eq!(elev.len(), NT * NZ);
        debug_assert_eq!(flux.len(), NT * NZ);
        debug_assert_eq!(elev0.len(), NT * NZ);
        let n_cells = ST * SZ;
        let mut soil = Self {
            n: vec![0.0; n_cells],
            p: vec![0.0; n_cells],
            k: vec![0.0; n_cells],
            organic: vec![0.0; n_cells],
            moisture: vec![0.0; n_cells],
            ph: vec![0.0; n_cells],
            compaction: vec![0.35; n_cells],
            microbes: vec![0.0; n_cells],
            hab,
        };
        let s = hab.seed ^ 0x5011;
        let max_e = hab.max_elevation.max(1.0);

        for sz in 0..SZ {
            for st in 0..ST {
                let ti = (st * 2).min(NT - 1);
                let zi = (sz * 2).min(NZ - 1);
                let i_t = ti + zi * NT;
                let e = elev[i_t];
                let e0 = elev0[i_t];
                let f = flux[i_t].clamp(0.0, 1.0);
                // Low basins / thin overburden → basalt-ish parent → more P/K.
                let basin = (1.0 - (e0 / max_e).clamp(0.0, 1.0)).powf(1.2);
                let basalt = (basin * 0.65 + (1.0 - e / max_e).max(0.0) * 0.35).clamp(0.0, 1.0);

                let noise = fbm2(st as f32 * 0.04, sz as f32 * 0.04, 7, 3, s);
                let i = idx(st, sz);
                soil.n[i] = 0.25 + 0.35 * f + 0.15 * noise;
                soil.p[i] = 0.12 + 0.55 * basalt + 0.10 * f;
                soil.k[i] = 0.15 + 0.50 * basalt + 0.12 * f;
                soil.organic[i] = (0.05 + 0.55 * f).clamp(0.0, 1.0);
                // Moisture from drainage × climate (not flux alone). LANDSCAPE_4200 §BQ.
                let theta = ti as f32 / NT as f32 * std::f32::consts::TAU;
                let z_m = (zi as f32 / NZ as f32 - 0.5) * hab.length;
                let arid = {
                    let p = crate::province::province_at(&hab, theta, z_m);
                    let a0 = match crate::province::climate_intent(p.primary) {
                        0 => 0.78,
                        1 => 0.38,
                        2 => 0.18,
                        3 => 0.12,
                        _ => 0.4,
                    };
                    let a1 = match crate::province::climate_intent(p.secondary) {
                        0 => 0.78,
                        1 => 0.38,
                        2 => 0.18,
                        3 => 0.12,
                        _ => 0.4,
                    };
                    p.blend2(a0, a1)
                };
                let rain_proxy = (1.0 - arid) * 0.85;
                soil.moisture[i] =
                    (0.10 + 0.40 * f + 0.45 * rain_proxy - 0.15 * arid).clamp(0.04, 1.0);
                // Shore / sea floor wet.
                if e < hab.water_level + 1.5 {
                    soil.moisture[i] = soil.moisture[i].max(0.75);
                }
                soil.ph[i] = 6.5 + (noise - 0.5) * 0.35;
                soil.microbes[i] =
                    (0.20 + 0.40 * soil.organic[i] * soil.moisture[i]).clamp(0.0, 1.0);
            }
        }
        soil
    }

    /// Bilinear sample in world cylindrical coords (theta rad, z metres).
    pub fn sample(&self, theta: f32, z: f32) -> SoilSample {
        let (t, zz) = self.world_to_grid(theta, z);
        SoilSample {
            n: sample_field(&self.n, t, zz),
            p: sample_field(&self.p, t, zz),
            k: sample_field(&self.k, t, zz),
            organic: sample_field(&self.organic, t, zz),
            moisture: sample_field(&self.moisture, t, zz),
            ph: sample_field(&self.ph, t, zz),
            compaction: sample_field(&self.compaction, t, zz),
            microbes: sample_field(&self.microbes, t, zz),
        }
    }

    /// Apply an amendment in a small disc (LANDSCAPE_1400 items 855, 869–871).
    /// `mass_kg` scales the per-kg effect; returns false if unknown amendment.
    pub fn amend(
        &mut self,
        theta: f32,
        z: f32,
        effect: crate::economy::AmendEffect,
        mass_kg: f32,
        radius_m: f32,
    ) -> bool {
        let mass = mass_kg.max(0.0);
        if mass <= 1e-6 {
            return true;
        }
        let rad = radius_m.max(1.0);
        let cell_t = std::f32::consts::TAU * self.hab.radius / ST as f32;
        let cell_z = self.hab.length / SZ as f32;
        let n_t = ((rad / cell_t).ceil() as i32 + 1).max(1);
        let n_z = ((rad / cell_z).ceil() as i32 + 1).max(1);
        let t0 = (theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * ST as f32)
            .round() as i32;
        let z0 = ((z / self.hab.length + 0.5) * SZ as f32).round() as i32;
        let mut cells = 0i32;
        for dz in -n_z..=n_z {
            let zz = z0 + dz;
            if zz < 0 || zz >= SZ as i32 {
                continue;
            }
            for dt in -n_t..=n_t {
                let tt = (t0 + dt).rem_euclid(ST as i32) as usize;
                let th = tt as f32 / ST as f32 * std::f32::consts::TAU;
                let zzf = (zz as f32 / SZ as f32 - 0.5) * self.hab.length;
                let dth = {
                    let x = (th - theta).rem_euclid(std::f32::consts::TAU);
                    let x = x.min(std::f32::consts::TAU - x);
                    x * self.hab.radius
                };
                let d = (dth * dth + (zzf - z) * (zzf - z)).sqrt();
                if d > rad {
                    continue;
                }
                cells += 1;
            }
        }
        let cells = cells.max(1) as f32;
        let per = mass / cells;
        for dz in -n_z..=n_z {
            let zz = z0 + dz;
            if zz < 0 || zz >= SZ as i32 {
                continue;
            }
            for dt in -n_t..=n_t {
                let tt = (t0 + dt).rem_euclid(ST as i32) as usize;
                let th = tt as f32 / ST as f32 * std::f32::consts::TAU;
                let zzf = (zz as f32 / SZ as f32 - 0.5) * self.hab.length;
                let dth = {
                    let x = (th - theta).rem_euclid(std::f32::consts::TAU);
                    let x = x.min(std::f32::consts::TAU - x);
                    x * self.hab.radius
                };
                let d = (dth * dth + (zzf - z) * (zzf - z)).sqrt();
                if d > rad {
                    continue;
                }
                let w = (1.0 - d / rad).max(0.0);
                let i = idx(tt, zz as usize);
                self.n[i] = (self.n[i] + effect.n * per * w).clamp(0.0, 1.5);
                self.p[i] = (self.p[i] + effect.p * per * w).clamp(0.0, 1.5);
                self.k[i] = (self.k[i] + effect.k * per * w).clamp(0.0, 1.5);
                self.organic[i] = (self.organic[i] + effect.organic * per * w).clamp(0.0, 1.5);
                self.ph[i] = (self.ph[i] + effect.ph * per * w).clamp(4.5, 9.0);
                self.microbes[i] =
                    (self.microbes[i] + effect.organic * per * w * 0.35).clamp(0.0, 1.0);
            }
        }
        true
    }

    /// Moisture diffusion, rain, evaporation, and N leaching. `rainfall_map`
    /// is ST×SZ (or any length — upsampled bilinear from weather res if shorter).
    pub fn tick(&mut self, dt_days: f32, elev: &[f32], flux: &[f32], rainfall_map: &[f32]) {
        let _ = flux; // reserved for permeability weighting later
        let dt = dt_days.max(0.0);
        if dt <= 0.0 {
            return;
        }

        let mut rain = vec![0.0f32; ST * SZ];
        if rainfall_map.len() == ST * SZ {
            rain.copy_from_slice(rainfall_map);
        } else if rainfall_map.len() == 192 * 128 {
            // Upsample weather-res humidity/rain (WT×WZ) to soil grid.
            const WT: usize = 192;
            const WZ: usize = 128;
            for sz in 0..SZ {
                for st in 0..ST {
                    let tx = st as f32 * (WT as f32 / ST as f32);
                    let zy = sz as f32 * (WZ as f32 / SZ as f32);
                    rain[idx(st, sz)] = sample_rect(rainfall_map, WT, WZ, tx, zy);
                }
            }
        }

        // Lateral moisture + N along elevation gradient (downslope).
        let mut m_next = self.moisture.clone();
        let mut n_next = self.n.clone();
        let evap_base = 0.045 * dt;
        let leach_rate = 0.08 * dt;

        for sz in 0..SZ {
            for st in 0..ST {
                let i = idx(st, sz);
                let e = elev_at(elev, st, sz);
                let mut m = self.moisture[i];
                // Rain already carries province/orographic scale from weather.
                m += rain[i] * dt * 1.15;
                // Dry cells (little recent rain) evaporate faster — rain-shadow scrub.
                let dry = if rain[i] < 0.04 { 1.45 } else if rain[i] < 0.12 { 1.15 } else { 0.92 };
                m = (m - evap_base * dry * (0.45 + 0.55 * m)).clamp(0.02, 1.0);

                // Neighbours: theta wraps, z clamps.
                let nbrs: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                let mut outflow = 0.0f32;
                let mut n_out = 0.0f32;
                for &(dt_, dz_) in &nbrs {
                    let nt = (st as i32 + dt_).rem_euclid(ST as i32) as usize;
                    let nz = (sz as i32 + dz_).clamp(0, SZ as i32 - 1) as usize;
                    let j = idx(nt, nz);
                    let ej = elev_at(elev, nt, nz);
                    let dh = e - ej;
                    if dh > 0.02 {
                        let w = (dh * 0.15).clamp(0.0, 0.25) * m * dt;
                        outflow += w;
                        // Soluble N rides moisture downslope (item 25).
                        n_out += w * leach_rate * self.n[i];
                        m_next[j] = (m_next[j] + w).min(1.0);
                        // No upper clamp on N — clipping would destroy the ledger (NEXT #9).
                        n_next[j] += w * leach_rate * self.n[i];
                    }
                }
                m_next[i] = (m - outflow).clamp(0.02, 1.0);
                n_next[i] = (self.n[i] - n_out).max(0.0);
            }
        }
        self.moisture = m_next;
        self.n = n_next;

        // Slow microbial response to organic × moisture.
        for i in 0..self.microbes.len() {
            let target = (0.15 + 0.55 * self.organic[i] * self.moisture[i]).clamp(0.0, 1.0);
            self.microbes[i] += (target - self.microbes[i]) * (0.02 * dt).min(1.0);
        }
    }

    fn world_to_grid(&self, theta: f32, z: f32) -> (f32, f32) {
        let t = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * ST as f32;
        let zz = (z / self.hab.length + 0.5) * SZ as f32;
        (t, zz)
    }
}

#[inline]
fn elev_at(elev: &[f32], st: usize, sz: usize) -> f32 {
    let ti = (st * 2).min(NT - 1);
    let zi = (sz * 2).min(NZ - 1);
    elev[ti + zi * NT]
}

fn sample_field(f: &[f32], x: f32, y: f32) -> f32 {
    let x = x.rem_euclid(ST as f32);
    let y = y.clamp(0.0, SZ as f32 - 1.001);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let x1 = (x0 + 1) % ST;
    let y1 = (y0 + 1).min(SZ - 1);
    let a = f[idx(x0, y0)];
    let b = f[idx(x1, y0)];
    let c = f[idx(x0, y1)];
    let d = f[idx(x1, y1)];
    let t = a + (b - a) * fx;
    let u = c + (d - c) * fx;
    t + (u - t) * fy
}

fn sample_rect(f: &[f32], wt: usize, wz: usize, x: f32, y: f32) -> f32 {
    let x = x.rem_euclid(wt as f32);
    let y = y.clamp(0.0, wz as f32 - 1.001);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let x1 = (x0 + 1) % wt;
    let y1 = (y0 + 1).min(wz - 1);
    let a = f[x0 + y0 * wt];
    let b = f[x1 + y0 * wt];
    let c = f[x0 + y1 * wt];
    let d = f[x1 + y1 * wt];
    let t = a + (b - a) * fx;
    let u = c + (d - c) * fx;
    t + (u - t) * fy
}
