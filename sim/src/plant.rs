//! Headless carbon-allocation vegetation. LANDSCAPE_200.md §H items 137–142.
//!
//! First test claim (item 139): a shaded plant goes leggy — stem/leaf rises.
//! See `allocation_step` + unit test below.

use crate::habitat::Habitat;
use crate::soil::Soil;
use crate::weather::Weather;

pub const MAX_PLANTS: usize = 50_000;

#[derive(Clone, Debug)]
pub struct Plant {
    pub carbon: f32,
    pub root: f32,
    pub leaf: f32,
    pub stem: f32,
    pub repro: f32,
    pub water_status: f32,
    pub n_status: f32,
    pub age: f32,
    pub stress: f32,
    pub genome_id: u32,
    pub theta: f32,
    pub z: f32,
    pub alive: bool,
}

impl Default for Plant {
    fn default() -> Self {
        Self {
            carbon: 0.05,
            root: 0.08,
            leaf: 0.10,
            stem: 0.06,
            repro: 0.0,
            water_status: 1.0,
            n_status: 1.0,
            age: 0.0,
            stress: 0.0,
            genome_id: 0,
            theta: 0.0,
            z: 0.0,
            alive: true,
        }
    }
}

pub struct PlantSim {
    pub plants: Vec<Plant>,
}

impl PlantSim {
    pub fn new() -> Self {
        Self { plants: Vec::with_capacity(4096) }
    }

    /// Place stands on good ground: elevated above water, with drainage flux.
    pub fn seed_stands(
        &mut self,
        hab: &Habitat,
        elev: &[f32],
        flux: &[f32],
        soil: &Soil,
        n_stands: usize,
    ) {
        use crate::terrain::{NT, NZ, idx as tidx};
        let n_stands = n_stands.min(MAX_PLANTS);
        let mut rng = hab.seed ^ 0x51A47;
        let mut placed = 0usize;
        let mut attempts = 0usize;
        while placed < n_stands && attempts < n_stands * 40 {
            attempts += 1;
            rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let ti = ((rng >> 8) % NT as u32) as usize;
            rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let zi = ((rng >> 8) % NZ as u32) as usize;
            let i = tidx(ti, zi);
            let e = elev[i];
            let f = flux[i];
            if e <= hab.water_level + 2.0 { continue; }
            if f < 0.08 && e < hab.max_elevation * 0.15 { continue; }

            let theta = (ti as f32 / NT as f32) * std::f32::consts::TAU;
            let z = (zi as f32 / NZ as f32 - 0.5) * hab.length;
            let s = soil.sample(theta, z);
            if s.moisture < 0.12 { continue; }

            let mut p = Plant::default();
            p.theta = theta;
            p.z = z;
            p.genome_id = (rng >> 16) & 0xFF;
            p.n_status = s.n.clamp(0.1, 1.0);
            p.water_status = s.moisture.clamp(0.1, 1.0);
            // Slight size jitter from flux (riparian edge grows faster).
            let scale = 0.85 + 0.40 * f.clamp(0.0, 1.0);
            p.root *= scale;
            p.leaf *= scale;
            p.stem *= scale;
            self.plants.push(p);
            placed += 1;
            if self.plants.len() >= MAX_PLANTS { break; }
        }
    }

    pub fn tick(
        &mut self,
        dt_days: f32,
        weather: &Weather,
        soil: &Soil,
        hab: &Habitat,
        greenhouses: &[crate::economy::Greenhouse],
        trophic: &crate::trophic::TrophicFields,
    ) {
        let dt = dt_days.max(0.0);
        if dt <= 0.0 { return; }
        let light_sched = weather.light_now();

        // Coarse leaf grid once per tick — O(n). The old stand_leaf_near scan
        // was O(n²) and hitch'd the Godot frame every sim step (~2s).
        let leaf_grid = self.build_leaf_grid(hab);
        let n = self.plants.len();
        for i in 0..n {
            if !self.plants[i].alive { continue; }

            let (th, zz) = (self.plants[i].theta, self.plants[i].z);
            let local_leaf = sample_leaf_grid(&leaf_grid, hab, th, zz);
            let self_shade = (local_leaf / (local_leaf + 2.5)).clamp(0.0, 0.85);
            let mut light = light_sched * (1.0 - self_shade);
            let mut temp = weather.temp_at(th, zz);
            // Glass greenhouse: local light + warmth (item 854).
            for gh in greenhouses {
                let dth = angle_diff(gh.theta, th) * hab.radius;
                let dz = gh.z - zz;
                if dth * dth + dz * dz <= gh.radius * gh.radius {
                    light = (light * 1.28).min(1.35);
                    temp += 2.4;
                    break;
                }
            }

            let s = soil.sample(th, zz);
            let water = s.moisture.clamp(0.05, 1.0);
            let nitro = s.n.clamp(0.05, 1.0);
            let temp_f = ((temp - 5.0) / 20.0).clamp(0.15, 1.0);
            // Miami NPP scales carbon gain — productivity flows from climate (1012).
            let npp = trophic.npp_at(th, zz, hab.length);
            let npp_f = (npp / 1200.0).clamp(0.35, 1.45);
            // Grazing pressure released where fear is high (1029 cascade).
            let graze = trophic.grazer_at(th, zz, hab.length)
                * (1.0 - trophic.fear_at(th, zz, hab.length) * 0.85);
            let graze_f = (1.0 - 0.18 * graze).clamp(0.65, 1.15);

            let leaf_area = self.plants[i].leaf.max(0.01);
            let photo = light * leaf_area * water * temp_f * nitro * npp_f * graze_f * 0.35 * dt;
            self.plants[i].carbon += photo;
            self.plants[i].water_status = water;
            self.plants[i].n_status = nitro;
            self.plants[i].age += dt;

            // Transpiration tracked as water drawdown on status (humidity loop later).
            let transpire = leaf_area * light * 0.08 * dt;
            self.plants[i].water_status = (water - transpire * 0.15).clamp(0.05, 1.0);

            let shaded = light < 0.35;
            let drought = self.plants[i].water_status < 0.35;
            let n_limited = nitro < 0.35;

            allocation_step(&mut self.plants[i], dt, shaded, drought, n_limited);

            // Stress / death.
            if drought { self.plants[i].stress += 0.15 * dt; }
            if light < 0.12 { self.plants[i].stress += 0.08 * dt; }
            if self.plants[i].stress > 1.0 {
                self.plants[i].alive = false;
            } else {
                self.plants[i].stress = (self.plants[i].stress - 0.03 * dt).max(0.0);
            }
        }
    }

    pub fn stand_biomass_near(&self, theta: f32, z: f32, radius: f32) -> f32 {
        let r2 = radius * radius;
        let mut sum = 0.0f32;
        for p in &self.plants {
            if !p.alive { continue; }
            let dth = angle_diff(p.theta, theta) * 900.0; // arc metres at ~hull radius
            let dz = p.z - z;
            if dth * dth + dz * dz <= r2 {
                sum += p.root + p.leaf + p.stem + p.repro;
            }
        }
        sum
    }

    fn build_leaf_grid(&self, hab: &Habitat) -> LeafGrid {
        let mut cells = vec![0.0f32; LEAF_GT * LEAF_GZ];
        let len = hab.length.max(1.0);
        for p in &self.plants {
            if !p.alive { continue; }
            let ti = ((p.theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU)
                * LEAF_GT as f32) as usize
                % LEAF_GT;
            let zi = (((p.z / len) + 0.5).clamp(0.0, 0.999) * LEAF_GZ as f32) as usize;
            cells[ti + zi * LEAF_GT] += p.leaf;
        }
        LeafGrid { cells }
    }
}

const LEAF_GT: usize = 192;
const LEAF_GZ: usize = 128;

struct LeafGrid {
    cells: Vec<f32>,
}

fn sample_leaf_grid(grid: &LeafGrid, hab: &Habitat, theta: f32, z: f32) -> f32 {
    let len = hab.length.max(1.0);
    let ti = ((theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU) * LEAF_GT as f32)
        as i32;
    let zi = (((z / len) + 0.5).clamp(0.0, 0.999) * LEAF_GZ as f32) as i32;
    // 3×3 neighbourhood ≈ plant-scale shade without an O(n) radius walk.
    let mut sum = 0.0f32;
    for dz in -1..=1 {
        let zz = zi + dz;
        if zz < 0 || zz >= LEAF_GZ as i32 {
            continue;
        }
        for dt in -1..=1 {
            let tt = (ti + dt).rem_euclid(LEAF_GT as i32) as usize;
            sum += grid.cells[tt + zz as usize * LEAF_GT];
        }
    }
    sum
}

#[inline]
fn angle_diff(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(std::f32::consts::TAU);
    if d > std::f32::consts::PI { d - std::f32::consts::TAU } else { d }
}

/// Allocate carbon by demand. Pure enough for unit tests (item 139).
///
/// - shaded → prefer stem (leggy)
/// - drought → abort repro, prefer root
/// - N limited → small leaves
pub fn allocation_step(
    p: &mut Plant,
    dt: f32,
    shaded: bool,
    drought: bool,
    n_limited: bool,
) {
    if !p.alive || p.carbon <= 0.0 || dt <= 0.0 { return; }

    let mut w_root = 0.25;
    let mut w_leaf = 0.35;
    let mut w_stem = 0.25;
    let mut w_repro = 0.15;

    if shaded {
        w_stem += 0.25;
        w_leaf -= 0.15;
    }
    if drought {
        w_root += 0.30;
        w_repro = 0.0;
        // Abort existing reproductive investment under drought (item 141).
        let abort = p.repro * (0.4 * dt).min(1.0);
        p.repro -= abort;
        p.root += abort * 0.5;
        p.carbon += abort * 0.25; // partial reclaim
    }
    if n_limited {
        w_leaf *= 0.45; // small leaves (item 142)
        w_root += 0.10;
    }

    let sum: f32 = w_root + w_leaf + w_stem + w_repro;
    let sum = sum.max(1e-6);
    let budget = p.carbon * (0.55 * dt).min(1.0);
    p.carbon -= budget;
    p.root += budget * (w_root / sum);
    p.leaf += budget * (w_leaf / sum);
    p.stem += budget * (w_stem / sum);
    p.repro += budget * (w_repro / sum);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shaded_plant_goes_leggy() {
        let mut sunny = Plant::default();
        sunny.carbon = 1.0;
        let mut shady = sunny.clone();

        for _ in 0..20 {
            allocation_step(&mut sunny, 1.0, false, false, false);
            sunny.carbon += 0.4;
            allocation_step(&mut shady, 1.0, true, false, false);
            shady.carbon += 0.4;
        }

        let sunny_ratio = sunny.stem / sunny.leaf.max(1e-6);
        let shady_ratio = shady.stem / shady.leaf.max(1e-6);
        assert!(
            shady_ratio > sunny_ratio * 1.15,
            "expected leggy shade ratio {shady_ratio} > sunny {sunny_ratio}"
        );
    }
}
