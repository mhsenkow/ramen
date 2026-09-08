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
    /// Cached rain scale WT×WZ from province climate (rebuilt rarely).
    rain_scale: Vec<f32>,
    rain_scale_age: f32,
    pub day_length_hours: f32,
    pub light_intensity: f32,
    pub reactor_power_budget: f32,
    pub power_used: f32,
    hab: Habitat,
    /// Accumulated habitat-day / spectacle phase for light schedule.
    pub day_phase: f32,
    /// Photothermal spine vapor rate (humidity / habitat-day into WT×WZ).
    pub spine_vapor_rate: f32,
    /// Day-carriage z (metres); vapor peaks under the traveling segment.
    pub carriage_z: f32,
    /// When true, day_phase is driven by the Godot spectacle clock.
    pub schedule_locked: bool,
    /// Sky event: 0 clear, 1 fog bank, 2 rain storm.
    pub sky_event: u8,
    /// 0..1 how hard the current event is hitting.
    pub sky_intensity: f32,
    /// Presentation fog multiplier from fog banks (0..1).
    pub fog_factor: f32,
    sky_timer: f32,
    sky_cooldown: f32,
    pub mean_humidity: f32,
    /// >1 shortens quiet gaps / rolls events faster (fast-day preview).
    pub spectacle_pace: f32,
}

pub mod sky {
    pub const CLEAR: u8 = 0;
    pub const FOG: u8 = 1;
    pub const STORM: u8 = 2;
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
                wind_z[i] = 3.5 + stripe; // m/s along axis
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
        let mut w = Self {
            condensers,
            humidity,
            rainfall: vec![0.0; ST * SZ],
            temperature,
            wind_theta,
            wind_z,
            rain_scale: vec![1.0; n_w],
            rain_scale_age: 0.0,
            day_length_hours: 14.0,
            light_intensity: 1.0,
            reactor_power_budget: 24.0,
            power_used,
            hab,
            day_phase: 0.0,
            spine_vapor_rate: 0.045,
            carriage_z: 0.0,
            schedule_locked: false,
            sky_event: sky::CLEAR,
            sky_intensity: 0.0,
            fog_factor: 0.0,
            sky_timer: 0.0,
            sky_cooldown: 0.15, // brief quiet after boot
            mean_humidity: 0.45,
            spectacle_pace: 1.0,
        };
        w.rebuild_rain_scale(&[]);
        w
    }

    /// Godot spectacle clock → sim light schedule (one clock for picture + plants).
    pub fn set_day_schedule(&mut self, phase_01: f32, light: f32, carriage_z: f32) {
        self.day_phase = phase_01.rem_euclid(1.0);
        self.light_intensity = light.clamp(0.0, 1.2);
        self.carriage_z = carriage_z;
        self.schedule_locked = true;
    }

    /// Fast-day preview: pace>1 rolls fog/storm more often without shrinking event length as hard.
    pub fn set_spectacle_pace(&mut self, pace: f32) {
        self.spectacle_pace = pace.clamp(1.0, 32.0);
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

    /// Temperature with elevation lapse rate (°C / m ≈ 0.0065). LANDSCAPE_4200 §BQ.
    pub fn temp_at_elev(&self, theta: f32, z: f32, elev_m: f32) -> f32 {
        self.temp_at(theta, z) - elev_m.max(0.0) * 0.0065
    }

    /// Aridity 0..1 from province climate intent + distance from condensers.
    pub fn aridity_at(&self, theta: f32, z: f32) -> f32 {
        let prov = crate::province::province_at(&self.hab, theta, z);
        let intent = crate::province::climate_intent(prov.primary);
        let base = match intent {
            0 => 0.78,
            1 => 0.38,
            2 => 0.18,
            3 => 0.12,
            _ => 0.40,
        };
        let base2 = match crate::province::climate_intent(prov.secondary) {
            0 => 0.78,
            1 => 0.38,
            2 => 0.18,
            3 => 0.12,
            _ => 0.40,
        };
        let mut arid = prov.blend2(base, base2);
        // Condensers reduce aridity.
        let mut near = 0.0f32;
        for c in &self.condensers {
            let dth = {
                let x = (c.theta - theta).rem_euclid(std::f32::consts::TAU);
                x.min(std::f32::consts::TAU - x) * self.hab.radius
            };
            let dz = (c.z - z).abs();
            let d = (dth * dth + dz * dz).sqrt();
            if d < c.radius * 1.4 {
                near = near.max(1.0 - d / (c.radius * 1.4));
            }
        }
        arid = (arid - near * 0.55).clamp(0.05, 0.95);
        // Sea basins stay marine-wet.
        arid = (arid - 0.35 * prov.weight(crate::province::id::SEA_BASIN)).clamp(0.05, 0.95);
        arid
    }

    /// Orographic rain factor: windward of high ground along +z gets a boost.
    pub fn orographic_factor(&self, _theta: f32, _z: f32, elev_here: f32, elev_upwind: f32) -> f32 {
        let rise = (elev_here - elev_upwind).max(0.0);
        1.0 + (rise / 80.0).clamp(0.0, 1.4) * 0.85
            - ((elev_upwind - elev_here).max(0.0) / 100.0).clamp(0.0, 1.0) * 0.55
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
        if self.schedule_locked {
            return self.light_intensity.clamp(0.0, 1.0);
        }
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

    pub fn tick(&mut self, dt_days: f32, omega: f32, elev: &[f32]) {
        let dt = dt_days.max(0.0);
        if dt <= 0.0 {
            return;
        }
        // Spectacle clock owns phase when Godot is driving; else free-run.
        if !self.schedule_locked {
            self.day_phase = (self.day_phase + dt).rem_euclid(1.0e6);
        }

        // Rebuild province→rain scale every ~2 habitat days (applied at deposit).
        self.rain_scale_age += dt;
        if self.rain_scale_age >= 2.0 {
            self.rain_scale_age = 0.0;
            self.rebuild_rain_scale(elev);
        }

        // Photothermal spine evaporates into the atmosphere — condensers recover it.
        // Stronger under the day-carriage and mid-habitat (LANDSCAPE Photothermal Spine).
        let half_l = (self.hab.length * 0.5).max(1.0);
        let vapor = self.spine_vapor_rate * dt;
        if vapor > 0.0 {
            for wz in 0..WZ {
                let z = (wz as f32 / (WZ - 1).max(1) as f32 - 0.5) * self.hab.length;
                let axial = 1.0 - ((z / half_l).abs()).powf(1.3);
                let near_car = 1.0 - ((z - self.carriage_z).abs() / 900.0).clamp(0.0, 1.0);
                let boost = (0.35 + 0.45 * axial + 0.55 * near_car * near_car) * vapor;
                for wt in 0..WT {
                    let i = wt + wz * WT;
                    self.humidity[i] = (self.humidity[i] + boost).min(1.0);
                }
            }
        }

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

        self.tick_sky_events(dt);

        // Clear rainfall; condensers produce rain where humidity is high enough.
        for r in self.rainfall.iter_mut() {
            *r = 0.0;
        }
        let budget_scale = if self.power_used > self.reactor_power_budget && self.power_used > 0.0 {
            self.reactor_power_budget / self.power_used
        } else {
            1.0
        };
        let storm_rain = if self.sky_event == sky::STORM {
            1.0 + 1.55 * self.sky_intensity
        } else {
            1.0
        };

        for c in &self.condensers {
            let pwr = c.power * budget_scale;
            if pwr <= 0.0 {
                continue;
            }
            let (ct, cz) = world_to_weather(&self.hab, c.theta, c.z);
            let r_cells_t =
                (c.radius / (std::f32::consts::TAU * self.hab.radius / WT as f32)).max(1.0);
            let r_cells_z = (c.radius / (self.hab.length / WZ as f32)).max(1.0);
            let rt = r_cells_t.ceil() as i32;
            let rz = r_cells_z.ceil() as i32;

            for dz in -rz..=rz {
                for dth in -rt..=rt {
                    let wt = (ct as i32 + dth).rem_euclid(WT as i32) as usize;
                    let wz = (cz as i32 + dz).clamp(0, WZ as i32 - 1) as usize;
                    let i = wt + wz * WT;
                    let dist =
                        ((dth as f32 / r_cells_t).powi(2) + (dz as f32 / r_cells_z).powi(2)).sqrt();
                    if dist > 1.0 {
                        continue;
                    }
                    let falloff = 1.0 - dist;
                    // Condensation needs humidity (item 67).
                    if self.humidity[i] < 0.38 {
                        continue;
                    }
                    let take = (0.12 * pwr * falloff * dt).min(self.humidity[i] - 0.25);
                    if take <= 0.0 {
                        continue;
                    }
                    self.humidity[i] -= take;
                    // Deposit rain onto soil grid under this weather cell.
                    let st0 = wt * ST / WT;
                    let sz0 = wz * SZ / WZ;
                    let st1 = ((wt + 1) * ST / WT).min(ST);
                    let sz1 = ((wz + 1) * SZ / WZ).min(SZ);
                    let rain_amt = take * 2.2 * falloff * self.rain_scale[i] * storm_rain;
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

    /// Sudden fog banks and rain storms from humid air + phase — “oh no” events.
    fn tick_sky_events(&mut self, dt: f32) {
        let n = self.humidity.len().max(1) as f32;
        let sum: f32 = self.humidity.iter().sum();
        self.mean_humidity = sum / n;

        let pace = self.spectacle_pace.max(1.0);
        // Pace burns quiet time faster; event length only mildly shortens so
        // fog/storm still reads as a beat during fast-day preview.
        let dt_wait = dt * pace;
        let dt_hold = dt * (1.0 + (pace - 1.0) * 0.25).min(3.0);

        let phase = if self.schedule_locked {
            self.day_phase
        } else {
            self.day_phase.rem_euclid(1.0)
        };
        let daylight = (self.day_length_hours / 24.0).clamp(0.1, 0.95);
        // Cool hours (night + dawn) favor fog; mid-day with wet air favors storms.
        let fog_hour = phase < 0.12 || phase > daylight + 0.02;
        let storm_hour = phase > 0.18 && phase < daylight - 0.05;

        if self.sky_event != sky::CLEAR {
            self.sky_timer -= dt_hold;
            // Fast onset, soft hold, then drop.
            let remaining = self.sky_timer.max(0.0);
            let peak = match self.sky_event {
                sky::FOG => 0.22,
                sky::STORM => 0.38,
                _ => 0.2,
            };
            let age_frac = if peak > 1e-4 {
                (1.0 - remaining / peak).clamp(0.0, 1.0)
            } else {
                1.0
            };
            // Envelope: rise quick (first 15%), hold, fall last 25%.
            let env = if age_frac < 0.15 {
                (age_frac / 0.15).clamp(0.0, 1.0)
            } else if age_frac < 0.75 {
                1.0
            } else {
                (1.0 - (age_frac - 0.75) / 0.25).clamp(0.0, 1.0)
            };
            let target = match self.sky_event {
                sky::FOG => 0.55 + 0.45 * (self.mean_humidity - 0.45).clamp(0.0, 1.0),
                sky::STORM => 0.65 + 0.35 * (self.mean_humidity - 0.5).clamp(0.0, 1.0),
                _ => 0.0,
            };
            self.sky_intensity = (target * env).clamp(0.0, 1.0);
            self.fog_factor = match self.sky_event {
                sky::FOG => self.sky_intensity,
                sky::STORM => 0.35 * self.sky_intensity,
                _ => 0.0,
            };
            if self.sky_timer <= 0.0 {
                self.sky_event = sky::CLEAR;
                self.sky_intensity = 0.0;
                self.fog_factor = 0.0;
                self.sky_cooldown = (0.28 + 0.22 * (1.0 - self.mean_humidity).clamp(0.0, 1.0)) / pace.sqrt();
            }
            return;
        }

        // Clear: bleed residual fog and wait for next roll.
        self.sky_intensity *= (1.0 - 8.0 * dt).max(0.0);
        self.fog_factor *= (1.0 - 6.0 * dt).max(0.0);
        if self.sky_intensity < 0.02 {
            self.sky_intensity = 0.0;
            self.fog_factor = 0.0;
        }
        self.sky_cooldown -= dt_wait;
        if self.sky_cooldown > 0.0 {
            return;
        }

        // Deterministic roll from phase + humidity (stable across ticks in a window).
        let seed = (phase * 9973.0 + self.mean_humidity * 431.0 + self.carriage_z * 0.001)
            .sin()
            .abs();
        let wet = (self.mean_humidity - 0.48).max(0.0);
        let chance_boost = (1.0 + (pace - 1.0) * 0.55).min(6.0);
        let fog_chance = if fog_hour {
            0.018 + wet * 0.09
        } else {
            0.004 + wet * 0.02
        } * chance_boost;
        let storm_chance = if storm_hour {
            0.012 + wet * 0.11
        } else {
            0.003 + wet * 0.025
        } * chance_boost;

        if seed < fog_chance.min(0.85) && fog_hour {
            self.sky_event = sky::FOG;
            self.sky_timer = 0.16 + 0.14 * wet; // ~habitat-day fractions
            self.sky_intensity = 0.25;
            self.fog_factor = 0.25;
            self.sky_cooldown = 0.0;
        } else if seed > 1.0 - storm_chance.min(0.85) && (storm_hour || wet > 0.12) {
            self.sky_event = sky::STORM;
            self.sky_timer = 0.28 + 0.22 * wet;
            self.sky_intensity = 0.3;
            self.fog_factor = 0.12;
            self.sky_cooldown = 0.0;
        } else {
            // Recheck soon — short quiet so weather stays lively.
            self.sky_cooldown = (0.04 + 0.06 * seed) / pace.sqrt();
        }
    }

    fn rebuild_rain_scale(&mut self, elev: &[f32]) {
        use crate::terrain::{idx as tidx, NT, NZ};
        let have_elev = elev.len() == NT * NZ;
        for wz in 0..WZ {
            let z = (wz as f32 / (WZ - 1).max(1) as f32 - 0.5) * self.hab.length;
            for wt in 0..WT {
                let theta = wt as f32 / WT as f32 * std::f32::consts::TAU;
                let p = crate::province::province_at(&self.hab, theta, z);
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
                let arid = p.blend2(a0, a1);
                let mut scale = (1.25 - arid * 0.95)
                    * (1.0
                        + 0.32 * p.weight(crate::province::id::MASSIF)
                        + 0.18 * p.weight(crate::province::id::SWAMP_BASIN)
                        - 0.48 * p.weight(crate::province::id::DUNE_SEA)
                        - 0.18 * p.weight(crate::province::id::SEA_BASIN));
                // Rain shadow: wind is mostly +z; upwind (−z) high ground sheds leeward.
                if have_elev {
                    let ti = (wt * NT / WT).min(NT - 1);
                    let zi = (wz * NZ / WZ).min(NZ - 1);
                    let elev_here = elev[tidx(ti, zi)];
                    let up = ((NZ as i32) / 48).max(2);
                    let zi_up = (zi as i32 - up).max(0) as usize;
                    let elev_up = elev[tidx(ti, zi_up)];
                    scale *= self.orographic_factor(theta, z, elev_here, elev_up);
                }
                self.rain_scale[wt + wz * WT] = scale.clamp(0.12, 2.1);
            }
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
    let a = f[x0 + y0 * nt];
    let b = f[x1 + y0 * nt];
    let c = f[x0 + y1 * nt];
    let d = f[x1 + y1 * nt];
    let t = a + (b - a) * fx;
    let u = c + (d - c) * fx;
    t + (u - t) * fy
}
