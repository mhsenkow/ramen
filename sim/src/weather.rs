//! Engineered weather — condensers, humidity, banded Coriolis climate.
//! LANDSCAPE_200.md §D items 65–71, §E 87–92 + rain drift (item 70).

use crate::habitat::Habitat;
use crate::soil::{ST, SZ};

/// Coarse humidity / cloud field.
pub const WT: usize = 192;
pub const WZ: usize = 128;

/// Number of axial climate stripes around the drum (item 88–89).
pub const BAND_COUNT: u32 = 24;

#[derive(Clone, Copy, Debug)]
pub struct Condenser {
    pub theta: f32,
    pub z: f32,
    pub power: f32,
    pub radius: f32,
}

pub struct Weather {
    pub condensers: Vec<Condenser>,
    /// Humidity at WT×WZ — cloud opacity reads this (item 68).
    pub humidity: Vec<f32>,
    /// Rainfall intensity at soil resolution ST×SZ.
    pub rainfall: Vec<f32>,
    /// Temperature at WT×WZ — axial gradient: ends cold, middle warm (item 75).
    pub temperature: Vec<f32>,
    /// Wind (u_theta, u_z) at WT×WZ — mostly axial, weak cross-theta (item 88).
    pub wind_theta: Vec<f32>,
    pub wind_z: Vec<f32>,
    pub day_length_hours: f32,
    pub light_intensity: f32,
    pub reactor_power_budget: f32,
    pub power_used: f32,
    hab: Habitat,
    /// Accumulated habitat-day for light schedule phase.
    day_phase: f32,
}

impl Weather {
    pub fn new(hab: Habitat) -> Self {
        let n_w = WT * WZ;
        let mut humidity = vec![0.45f32; n_w];
        let mut temperature = vec![0.0f32; n_w];
        let mut wind_theta = vec![0.0f32; n_w];
        let mut wind_z = vec![0.0f32; n_w];

        for wz in 0..WZ {
            let z_norm = wz as f32 / (WZ - 1) as f32; // 0..1 along axis
            // Ends cold, middle warm (item 75).
            let axial = 1.0 - ((z_norm - 0.5).abs() * 2.0).powf(1.4);
            let t_c = 8.0 + 18.0 * axial; // °C-ish scalar
            for wt in 0..WT {
                let i = wt + wz * WT;
                temperature[i] = t_c;
                // Banded wind: strong along +z, weak cross-theta (items 88–91).
                let band = (wt as f32 / WT as f32) * BAND_COUNT as f32;
                let stripe = (band * std::f32::consts::TAU).sin() * 0.15;
                wind_z[i] = 3.5 + stripe;       // m/s along axis
                wind_theta[i] = 0.25 + stripe * 0.1; // weak Ekman cross
                humidity[i] = 0.40 + 0.10 * axial;
            }
        }

        // Six default condensers around mid-habitat spawn bands.
        let mut condensers = Vec::with_capacity(6);
        let mid_z = 0.0;
        for k in 0..6 {
            let theta = (k as f32 / 6.0) * std::f32::consts::TAU;
            condensers.push(Condenser {
                theta,
                z: mid_z + ((k as i32 % 3) - 1) as f32 * 180.0,
                power: 1.0,
                radius: 220.0,
            });
        }

        let power_used: f32 = condensers.iter().map(|c| c.power).sum();
        Self {
            condensers,
            humidity,
            rainfall: vec![0.0; ST * SZ],
            temperature,
            wind_theta,
            wind_z,
            day_length_hours: 14.0,
            light_intensity: 1.0,
            reactor_power_budget: 24.0,
            power_used,
            hab,
            day_phase: 0.0,
        }
    }

    pub fn add_condenser(&mut self, theta: f32, z: f32, power: f32) {
        let power = power.max(0.0);
        self.condensers.push(Condenser {
            theta: theta.rem_euclid(std::f32::consts::TAU),
            z,
            power,
            radius: 180.0 + 40.0 * power.min(2.0),
        });
        self.power_used = self.condensers.iter().map(|c| c.power).sum();
    }

    /// Place only if reactor headroom remains (NEXT #4).
    pub fn try_add_condenser(&mut self, theta: f32, z: f32, power: f32) -> bool {
        let power = power.max(0.0);
        if self.power_used + power > self.reactor_power_budget + 1e-3 {
            return false;
        }
        self.add_condenser(theta, z, power);
        true
    }

    pub fn power_headroom(&self) -> f32 {
        (self.reactor_power_budget - self.power_used).max(0.0)
    }

    /// Axial climate stripe index from theta (items 88–89).
    #[inline]
    pub fn band_index(theta: f32) -> u32 {
        let u = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
        ((u * BAND_COUNT as f32).floor() as u32) % BAND_COUNT
    }

    pub fn rain_at(&self, theta: f32, z: f32) -> f32 {
        let (t, zz) = world_to_soil(&self.hab, theta, z);
        sample_wrap(&self.rainfall, ST, SZ, t, zz)
    }

    pub fn temp_at(&self, theta: f32, z: f32) -> f32 {
        let (t, zz) = world_to_weather(&self.hab, theta, z);
        sample_wrap(&self.temperature, WT, WZ, t, zz)
    }

    pub fn humidity_at(&self, theta: f32, z: f32) -> f32 {
        let (t, zz) = world_to_weather(&self.hab, theta, z);
        sample_wrap(&self.humidity, WT, WZ, t, zz)
    }

    /// Cloud opacity 0..1 from local humidity (item 68).
    pub fn cloud_opacity(&self, theta: f32, z: f32) -> f32 {
        ((self.humidity_at(theta, z) - 0.35) / 0.55).clamp(0.0, 1.0)
    }

    /// Current light schedule factor 0..1 from day length + intensity policy.
    pub fn light_now(&self) -> f32 {
        let day_frac = self.day_phase.fract();
        let daylight = (self.day_length_hours / 24.0).clamp(0.1, 0.95);
        let dawn = 0.05;
        let dusk_start = daylight;
        let lit = if day_frac < dawn {
            (day_frac / dawn) * self.light_intensity
        } else if day_frac < dusk_start {
            self.light_intensity
        } else if day_frac < dusk_start + dawn {
            (1.0 - (day_frac - dusk_start) / dawn) * self.light_intensity
        } else {
            0.0
        };
        lit.clamp(0.0, 1.0)
    }

    pub fn tick(&mut self, dt_days: f32, omega: f32) {
        let dt = dt_days.max(0.0);
        if dt <= 0.0 { return; }
        self.day_phase = (self.day_phase + dt).rem_euclid(1.0e6);

        // Evaporation into humidity — moisture proxy from last rainfall + baseline.
        for wz in 0..WZ {
            for wt in 0..WT {
                let i = wt + wz * WT;
                let st = wt * ST / WT;
                let sz = wz * SZ / WZ;
                let moist_proxy = 0.25 + 0.75 * self.rainfall[st + sz * ST].min(1.0);
                let evap = 0.03 * moist_proxy * dt;
                self.humidity[i] = (self.humidity[i] + evap).min(1.0);
                // Weak along-band advection (item 88): humidity smears in z.
                if wz + 1 < WZ {
                    let j = wt + (wz + 1) * WT;
                    let flow = 0.02 * dt * self.wind_z[i].abs();
                    let xfer = (self.humidity[i] - self.humidity[j]) * flow;
                    self.humidity[i] -= xfer;
                    self.humidity[j] += xfer;
                }
            }
        }

        // Clear rainfall; condensers produce rain where humidity is high enough.
        for r in self.rainfall.iter_mut() { *r = 0.0; }
        let budget_scale = if self.power_used > self.reactor_power_budget && self.power_used > 0.0 {
            self.reactor_power_budget / self.power_used
        } else {
            1.0
        };

        for c in &self.condensers {
            let pwr = c.power * budget_scale;
            if pwr <= 0.0 { continue; }
            let (ct, cz) = world_to_weather(&self.hab, c.theta, c.z);
            let r_cells_t = (c.radius / (std::f32::consts::TAU * self.hab.radius / WT as f32)).max(1.0);
            let r_cells_z = (c.radius / (self.hab.length / WZ as f32)).max(1.0);
            let rt = r_cells_t.ceil() as i32;
            let rz = r_cells_z.ceil() as i32;

            for dz in -rz..=rz {
                for dth in -rt..=rt {
                    let wt = (ct as i32 + dth).rem_euclid(WT as i32) as usize;
                    let wz = (cz as i32 + dz).clamp(0, WZ as i32 - 1) as usize;
                    let i = wt + wz * WT;
                    let dist = ((dth as f32 / r_cells_t).powi(2) + (dz as f32 / r_cells_z).powi(2)).sqrt();
                    if dist > 1.0 { continue; }
                    let falloff = 1.0 - dist;
                    // Condensation needs humidity (item 67).
                    if self.humidity[i] < 0.38 { continue; }
                    let take = (0.12 * pwr * falloff * dt).min(self.humidity[i] - 0.25);
                    if take <= 0.0 { continue; }
                    self.humidity[i] -= take;
                    // Deposit rain onto soil grid under this weather cell.
                    let st0 = wt * ST / WT;
                    let sz0 = wz * SZ / WZ;
                    let st1 = ((wt + 1) * ST / WT).min(ST);
                    let sz1 = ((wz + 1) * SZ / WZ).min(SZ);
                    let rain_amt = take * 2.2 * falloff;
                    for sz in sz0..sz1 {
                        for st in st0..st1 {
                            self.rainfall[st + sz * ST] += rain_amt;
                        }
                    }
                }
            }
        }

        // Rain drifts spinward by Coriolis (item 70): tens of metres in +theta.
        // Drift ≈ 40 m × (ω / ω_kepler); ω_kepler ≈ sqrt(0.95g/900) ≈ 0.102.
        let omega_ref = 0.102;
        let drift_m = 40.0 * (omega / omega_ref).clamp(0.25, 3.0);
        let cell_m = std::f32::consts::TAU * self.hab.radius / ST as f32;
        let shift = (drift_m / cell_m).round() as i32;
        if shift != 0 {
            let mut shifted = vec![0.0f32; ST * SZ];
            for sz in 0..SZ {
                for st in 0..ST {
                    let src = (st as i32 - shift).rem_euclid(ST as i32) as usize;
                    shifted[st + sz * ST] = self.rainfall[src + sz * ST];
                }
            }
            self.rainfall = shifted;
        }

        // Soft rain decay so maps don't accumulate forever between soil ticks.
        for r in self.rainfall.iter_mut() {
            *r = (*r).clamp(0.0, 2.0);
        }
    }
}

fn world_to_soil(hab: &Habitat, theta: f32, z: f32) -> (f32, f32) {
    let t = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * ST as f32;
    let zz = (z / hab.length + 0.5) * SZ as f32;
    (t, zz)
}

fn world_to_weather(hab: &Habitat, theta: f32, z: f32) -> (f32, f32) {
    let t = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * WT as f32;
    let zz = (z / hab.length + 0.5) * WZ as f32;
    (t, zz)
}

fn sample_wrap(f: &[f32], nt: usize, nz: usize, x: f32, y: f32) -> f32 {
    let x = x.rem_euclid(nt as f32);
    let y = y.clamp(0.0, nz as f32 - 1.001);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let x1 = (x0 + 1) % nt;
    let y1 = (y0 + 1).min(nz - 1);
    let a = f[x0 + y0 * nt]; let b = f[x1 + y0 * nt];
    let c = f[x0 + y1 * nt]; let d = f[x1 + y1 * nt];
    let t = a + (b - a) * fx;
    let u = c + (d - c) * fx;
    t + (u - t) * fy
}
