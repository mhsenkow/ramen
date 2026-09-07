//! Trophic productivity — Miami NPP, Kleiber densities, carcasses, fear.
//! LANDSCAPE_1400.md §AA items 1011–1030.
//!
//! Limits (item 1080 / MODEL_LIMITS): NPP is the cited Miami empirical form;
//! transfer efficiency is a flat 10 %; fauna densities from Kleiber scaling,
//! not individual metabolism. Carcasses are discrete objects; fear is a field
//! prey read. Field resolution matches weather (WT×WZ).

use crate::economy::AmendEffect;
use crate::habitat::Habitat;
use crate::soil::Soil;
use crate::weather::{Weather, WT, WZ};

/// Transfer efficiency between trophic levels (item 1014). Authored flat.
pub const TRANSFER_EFF: f32 = 0.10;

/// Kleiber exponent (metabolic rate ∝ M^k). Cited; not invented.
pub const KLEIBER_EXP: f32 = 0.75;

pub const MAX_CARCASSES: usize = 48;

/// Miami-model net primary productivity (Lieth 1972), g dry matter / m² / year.
/// NPP = min(NPP_T, NPP_P) with T in °C and P in mm/year.
pub fn miami_npp(temp_c: f32, precip_mm_yr: f32) -> f32 {
    let t = temp_c;
    let p = precip_mm_yr.max(0.0);
    let npp_t = 3000.0 / (1.0 + (1.315 - 0.119 * t).exp());
    let npp_p = 3000.0 * (1.0 - (-0.000664 * p).exp());
    npp_t.min(npp_p).clamp(0.0, 3000.0)
}

/// Map our rain intensity field to annual mm. Invented scale — MODEL_LIMITS.
pub fn precip_mm_yr_from_rain(rain_intensity: f32) -> f32 {
    (rain_intensity.max(0.0) * 365.0 * 2.2).clamp(50.0, 2500.0)
}

/// Population density (individuals / km²) at this NPP for a body mass (1017).
pub fn kleiber_density(body_mass_kg: f32, npp_g_m2_yr: f32) -> f32 {
    let m = body_mass_kg.max(0.01);
    let metabolic = m.powf(KLEIBER_EXP);
    let support = npp_g_m2_yr * TRANSFER_EFF * 1.0e6; // g/km²/yr
    let per_capita = metabolic * 50.0 * 1000.0; // g/yr (invented)
    (support / per_capita.max(1.0)).max(0.0)
}

pub fn home_range_km2(body_mass_kg: f32) -> f32 {
    0.01 * body_mass_kg.max(0.01).powf(KLEIBER_EXP)
}

pub fn drum_area_km2(hab: &Habitat) -> f32 {
    (std::f32::consts::TAU * hab.radius * hab.length) / 1.0e6
}

pub fn max_body_mass_kg(hab: &Habitat, npp_g_m2_yr: f32, mvp: f32) -> f32 {
    let area = drum_area_km2(hab);
    let mut lo = 0.01f32;
    let mut hi = 5000.0f32;
    for _ in 0..24 {
        let mid = (lo + hi) * 0.5;
        if kleiber_density(mid, npp_g_m2_yr) * area >= mvp {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Discrete kill site (item 1024). Stages: 0 fresh → 1 scavenged → 2 bones.
#[derive(Clone, Debug)]
pub struct Carcass {
    pub theta: f32,
    pub z: f32,
    pub mass_kg: f32,
    pub age_days: f32,
    pub stage: u8,
}

/// Producer / grazer / hunter / detritus / fear at weather resolution.
pub struct TrophicFields {
    pub npp: Vec<f32>,
    pub producer: Vec<f32>,
    pub grazer: Vec<f32>,
    pub hunter: Vec<f32>,
    pub detritus: Vec<f32>,
    pub fear: Vec<f32>,
    pub carcasses: Vec<Carcass>,
    mean_npp: f32,
    rng: u32,
    pub kills: u32,
}

impl TrophicFields {
    pub fn new() -> Self {
        let n = WT * WZ;
        Self {
            npp: vec![0.0; n],
            producer: vec![0.0; n],
            grazer: vec![0.0; n],
            hunter: vec![0.0; n],
            detritus: vec![0.0; n],
            fear: vec![0.0; n],
            carcasses: Vec::with_capacity(MAX_CARCASSES),
            mean_npp: 0.0,
            rng: 0xC4A55_u32,
            kills: 0,
        }
    }

    pub fn mean_npp(&self) -> f32 {
        self.mean_npp
    }

    fn next_f01(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 8) as f32 / (0x00FF_FFFF as f32)
    }

    pub fn tick(
        &mut self,
        dt_days: f32,
        weather: &Weather,
        soil: &mut Soil,
        length: f32,
        hab_r: f32,
    ) {
        let dt = dt_days.max(0.0);
        if dt <= 0.0 {
            return;
        }
        let length = length.max(1.0);
        let mut acc = 0.0f32;
        for wz in 0..WZ {
            for wt in 0..WT {
                let i = wt + wz * WT;
                let theta = wt as f32 / WT as f32 * std::f32::consts::TAU;
                let z = (wz as f32 / WZ as f32 - 0.5) * length;
                let temp = weather.temp_at(theta, z);
                let rain = weather.rain_at(theta, z);
                let moist = soil.sample(theta, z).moisture;
                let precip = precip_mm_yr_from_rain(rain.max(moist * 0.35));
                let npp = miami_npp(temp, precip);
                self.npp[i] = npp;
                acc += npp;

                // Fear decays with time since the predator passed (1030).
                self.fear[i] = (self.fear[i] * (1.0 - 0.12 * dt)).max(0.0);
                let fear_f = self.fear[i];

                let target = (npp / 1800.0).clamp(0.0, 1.0);
                // High fear → less grazing pressure → producers recover (1029).
                let prod_boost = 1.0 + 0.25 * fear_f;
                self.producer[i] +=
                    (target * prod_boost - self.producer[i]) * (0.15 * dt).min(1.0);
                self.producer[i] = self.producer[i].clamp(0.0, 1.4);

                // Grazers avoid high-fear cells (1028 landscape of fear).
                let g_t = self.producer[i] * TRANSFER_EFF * (1.0 - 0.75 * fear_f);
                self.grazer[i] += (g_t - self.grazer[i]) * (0.08 * dt).min(1.0);
                self.grazer[i] = self.grazer[i].clamp(0.0, 1.0);

                let h_t = self.grazer[i] * TRANSFER_EFF;
                self.hunter[i] += (h_t - self.hunter[i]) * (0.05 * dt).min(1.0);
                self.hunter[i] = self.hunter[i].clamp(0.0, 1.0);

                let fall = (self.producer[i] * 0.02 + self.grazer[i] * 0.04 + self.hunter[i] * 0.03)
                    * dt;
                self.detritus[i] =
                    (self.detritus[i] + fall - self.detritus[i] * 0.04 * dt).clamp(0.0, 2.0);

                // Predation event → discrete carcass + fear plume (1024 / 1028).
                let kill_p = self.hunter[i] * self.grazer[i] * dt * 0.55;
                if kill_p > 0.002
                    && self.grazer[i] > 0.04
                    && self.hunter[i] > 0.03
                    && self.next_f01() < kill_p
                {
                    let mass = (8.0 + self.next_f01() * 40.0) * (0.4 + self.grazer[i]);
                    self.spawn_carcass(theta, z, mass, length, hab_r);
                    self.grazer[i] = (self.grazer[i] * 0.82).max(0.0);
                    self.kills += 1;
                }
            }
        }
        self.mean_npp = acc / (WT * WZ) as f32;
        self.tick_carcasses(dt, soil, length, hab_r);
    }

    fn spawn_carcass(&mut self, theta: f32, z: f32, mass_kg: f32, length: f32, hab_r: f32) {
        if self.carcasses.len() >= MAX_CARCASSES {
            // Drop the oldest bones first.
            if let Some((i, _)) = self
                .carcasses
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.age_days.partial_cmp(&b.1.age_days).unwrap())
            {
                self.carcasses.swap_remove(i);
            }
        }
        self.carcasses.push(Carcass {
            theta,
            z,
            mass_kg,
            age_days: 0.0,
            stage: 0,
        });
        // Predation pressure radiates from the kill (1028).
        self.add_fear(theta, z, length, 0.65, 55.0 * (hab_r / 900.0).clamp(0.5, 1.5));
    }

    fn tick_carcasses(&mut self, dt: f32, soil: &mut Soil, length: f32, _hab_r: f32) {
        let mut i = 0;
        while i < self.carcasses.len() {
            let c = &mut self.carcasses[i];
            c.age_days += dt;
            let (th, zz, mass) = (c.theta, c.z, c.mass_kg);
            let stage = c.stage;
            let age = c.age_days;

            // Feed detritus at the weather cell under the carcass.
            let wi = cell_index(th, zz, length);
            self.detritus[wi] = (self.detritus[wi] + mass * 0.002 * dt).clamp(0.0, 2.5);

            if stage == 0 && age > 1.8 {
                c.stage = 1;
                // Scavenger pass — fear spike as they arrive (1027 lite).
                self.add_fear(th, zz, length, 0.25, 30.0);
            } else if stage == 1 && age > 5.5 {
                c.stage = 2;
                // Nutrient hotspot into soil (1026).
                let effect = AmendEffect {
                    n: 0.12,
                    p: 0.06,
                    k: 0.02,
                    organic: 0.18,
                    ph: -0.02,
                };
                soil.amend(th, zz, effect, mass * 0.35, 6.0);
            } else if stage == 2 && age > 14.0 {
                self.carcasses.swap_remove(i);
                continue;
            }
            i += 1;
        }
    }

    pub fn npp_at(&self, theta: f32, z: f32, length: f32) -> f32 {
        sample_field(&self.npp, theta, z, length)
    }

    pub fn producer_at(&self, theta: f32, z: f32, length: f32) -> f32 {
        sample_field(&self.producer, theta, z, length)
    }

    pub fn fear_at(&self, theta: f32, z: f32, length: f32) -> f32 {
        sample_field(&self.fear, theta, z, length)
    }

    pub fn grazer_at(&self, theta: f32, z: f32, length: f32) -> f32 {
        sample_field(&self.grazer, theta, z, length)
    }

    pub fn add_fear(&mut self, theta: f32, z: f32, length: f32, amount: f32, radius_m: f32) {
        let rad = radius_m.max(1.0);
        let cell_t = std::f32::consts::TAU * 900.0 / WT as f32;
        let cell_z = length / WZ as f32;
        let n_t = ((rad / cell_t).ceil() as i32 + 1).max(1);
        let n_z = ((rad / cell_z).ceil() as i32 + 1).max(1);
        let t0 = (theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * WT as f32)
            .round() as i32;
        let z0 = ((z / length + 0.5) * WZ as f32).round() as i32;
        for dz in -n_z..=n_z {
            let zz = z0 + dz;
            if zz < 0 || zz >= WZ as i32 {
                continue;
            }
            for dt in -n_t..=n_t {
                let tt = (t0 + dt).rem_euclid(WT as i32) as usize;
                let th = tt as f32 / WT as f32 * std::f32::consts::TAU;
                let zzf = (zz as f32 / WZ as f32 - 0.5) * length;
                let dth = angle_arc(th, theta) * 900.0;
                let d = (dth * dth + (zzf - z) * (zzf - z)).sqrt();
                if d > rad {
                    continue;
                }
                let w = (1.0 - d / rad).max(0.0);
                let i = tt + zz as usize * WT;
                self.fear[i] = (self.fear[i] + amount * w).clamp(0.0, 1.0);
            }
        }
    }
}

fn cell_index(theta: f32, z: f32, length: f32) -> usize {
    let t = (theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * WT as f32).floor()
        as usize
        % WT;
    let zz = (((z / length.max(1.0)) + 0.5).clamp(0.0, 0.999) * WZ as f32).floor() as usize;
    t + zz.min(WZ - 1) * WT
}

fn sample_field(f: &[f32], theta: f32, z: f32, length: f32) -> f32 {
    let t = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * WT as f32;
    let zz = ((z / length.max(1.0)) + 0.5).clamp(0.0, 0.999) * WZ as f32;
    let t0 = t.floor() as usize % WT;
    let z0 = (zz.floor() as usize).min(WZ - 1);
    let t1 = (t0 + 1) % WT;
    let z1 = (z0 + 1).min(WZ - 1);
    let ft = t - t.floor();
    let fz = zz - zz.floor();
    let a = f[t0 + z0 * WT];
    let b = f[t1 + z0 * WT];
    let c = f[t0 + z1 * WT];
    let d = f[t1 + z1 * WT];
    let u = a + (b - a) * ft;
    let v = c + (d - c) * ft;
    u + (v - u) * fz
}

fn angle_arc(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(std::f32::consts::TAU);
    if d > std::f32::consts::PI {
        d - std::f32::consts::TAU
    } else {
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habitat::Habitat;
    use crate::terrain::Terrain;

    #[test]
    fn miami_warm_wet_beats_cold_dry() {
        let lush = miami_npp(22.0, 1400.0);
        let arid = miami_npp(22.0, 120.0);
        let cold = miami_npp(2.0, 1400.0);
        assert!(lush > arid * 1.5, "lush {lush} vs arid {arid}");
        assert!(lush > cold * 1.2, "lush {lush} vs cold {cold}");
    }

    #[test]
    fn kleiber_makes_large_animals_rare() {
        let npp = 1200.0;
        let mice = kleiber_density(0.02, npp);
        let deer = kleiber_density(60.0, npp);
        assert!(mice > deer * 20.0, "mice {mice} vs deer {deer}");
    }

    #[test]
    fn drum_cannot_support_elephant() {
        let hab = Habitat::kepler_drum();
        let max_m = max_body_mass_kg(&hab, 1000.0, 50.0);
        assert!(max_m < 800.0, "drum max viable mass should be modest, got {max_m} kg");
    }

    #[test]
    fn kill_injects_fear_and_carcass() {
        let hab = Habitat::kepler_drum();
        let ter = Terrain::generate(hab);
        let mut soil = Soil::new(hab, &ter.elev, &ter.flow.flux, &ter.elev0);
        let weather = Weather::new(hab);
        let mut tf = TrophicFields::new();
        // Force a fertile predation cell.
        for v in tf.grazer.iter_mut() {
            *v = 0.6;
        }
        for v in tf.hunter.iter_mut() {
            *v = 0.5;
        }
        for _ in 0..40 {
            tf.tick(0.25, &weather, &mut soil, hab.length, hab.radius);
        }
        assert!(
            tf.kills > 0 || !tf.carcasses.is_empty(),
            "expected kills or carcasses"
        );
        let fear_max = tf.fear.iter().cloned().fold(0.0f32, f32::max);
        assert!(fear_max > 0.05, "fear should rise after kills, got {fear_max}");
    }

    #[test]
    fn fear_suppresses_grazer_target() {
        let hab = Habitat::kepler_drum();
        let ter = Terrain::generate(hab);
        let mut soil = Soil::new(hab, &ter.elev, &ter.flow.flux, &ter.elev0);
        let weather = Weather::new(hab);
        let mut calm = TrophicFields::new();
        let mut scared = TrophicFields::new();
        for v in scared.fear.iter_mut() {
            *v = 0.9;
        }
        for _ in 0..20 {
            calm.tick(0.2, &weather, &mut soil, hab.length, hab.radius);
            scared.tick(0.2, &weather, &mut soil, hab.length, hab.radius);
        }
        let g_calm: f32 = calm.grazer.iter().sum::<f32>() / calm.grazer.len() as f32;
        let g_fear: f32 = scared.grazer.iter().sum::<f32>() / scared.grazer.len() as f32;
        assert!(
            g_fear < g_calm * 0.85,
            "fear should suppress grazers: calm {g_calm} fear {g_fear}"
        );
    }
}
