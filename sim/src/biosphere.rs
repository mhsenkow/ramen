//! Coupled biosphere tick — soil, weather, plants, erosion, flow, lakes.
//! Owns everything that makes regions affect each other.

use crate::agent::AgentSim;
use crate::biome;
use crate::chronicle::Chronicle;
use crate::dwelling::Dwellings;
use crate::economy::{Atmosphere, Greenhouse};
use crate::erosion::{self, Erosion};
use crate::lakes::Lakes;
use crate::material;
use crate::plant::PlantSim;
use crate::soil::{Soil, ST, SZ};
use crate::sph::SurfaceWater;
use crate::terrain::{idx, Terrain, NT, NZ};
use crate::trophic::TrophicFields;
use crate::weather::Weather;

pub struct Biosphere {
    pub soil: Soil,
    pub weather: Weather,
    pub plants: PlantSim,
    pub lakes: Lakes,
    pub water: SurfaceWater,
    pub trophic: TrophicFields,
    pub agents: AgentSim,
    /// Where each principal lives, and what that ground can feed.
    pub dwellings: Dwellings,
    pub chronicle: Chronicle,
    pub day: f32,
    pub water_stock: f32,
    pub nitrogen_stock: f32,
    pub carbon_stock: f32,
    pub energy_stock: f32,
    /// Closed breathable ledger (item 843).
    pub atmosphere: Atmosphere,
    pub greenhouses: Vec<Greenhouse>,
    /// Nitrogen at day 0 — golden invariant baseline.
    pub nitrogen_initial: f32,
    erosion_seed: u32,
    water_seed: u32,
    hardness: Vec<f32>,
    /// Last habitat-day we paid for a flow rebuild. Debounced so sim_tick
    /// doesn't full-flood the drum every few real seconds (look-hitch).
    last_flow_day: f32,
    last_survey_day: f32,
    /// Talus once per interval — `(day as u32) % 3 == 0` used to run every
    /// tick for an entire habitat day and hitch the client every ~3 s.
    last_talus_day: f32,
}

impl Biosphere {
    pub fn new(ter: &Terrain) -> Self {
        let hab = ter.hab;
        let soil = Soil::new(hab, &ter.elev, &ter.flow.flux, &ter.elev0);
        let mut weather = Weather::new(hab);
        for i in 0..8 {
            let th = (i as f32 + 0.5) / 8.0 * std::f32::consts::TAU;
            weather.add_condenser(th, 0.0, 1.0);
            weather.add_condenser(th + 0.15, hab.length * 0.18, 0.7);
        }
        let mut plants = PlantSim::new();
        plants.seed_stands(&hab, &ter.elev, &ter.flow.flux, &soil, 12_000);
        let mut agents = AgentSim::new();
        agents.seed_farmers(&hab, &ter.elev, 10);
        // Site each principal on the ground their traits reach for, then move
        // them to it — a man lives where he chose to live, not where the
        // scatter dropped him. Capacity is surveyed against the real
        // heightfield and the real soil, so a bad site simply cannot grow.
        let mut dwellings = Dwellings::default();
        for ag in &agents.agents {
            dwellings.site(
                &hab,
                &ter.elev,
                Some(&soil),
                ag.id,
                ag.theta,
                ag.z,
                ag.traits.patience,
                ag.traits.risk,
                ag.traits.care,
                ag.traits.social,
                0.0,
            );
        }
        dwellings.mark_contested(hab.radius);
        for ag in &mut agents.agents {
            if let Some(d) = dwellings.get(ag.id) {
                ag.theta = d.theta;
                ag.z = d.z;
                ag.plot_theta = d.theta;
                ag.plot_z = d.z;
            }
        }
        let n_sum: f32 = soil.n.iter().sum();
        let energy = weather.reactor_power_budget;
        let hardness = build_hardness(ter);
        let cell_area = (std::f32::consts::TAU * hab.radius / NT as f32) * (hab.length / NZ as f32);
        let lakes = Lakes::extract(
            &ter.elev,
            &ter.flow.filled,
            &ter.flow.lake,
            &ter.flow.discharge,
            hab.radius,
            hab.length,
            cell_area,
        );
        let mut water = SurfaceWater::default();
        water.seed_from_fill(&ter.elev, &ter.flow.filled, &ter.flow.lake, hab.water_level);
        // A few warm-up ticks so basins equalize into flat free surfaces
        // before the first frame (Minecraft "already pooled" look).
        let mut wseed = hab.seed ^ 0xA7E1;
        let rain0 = upsample_rain(&weather.rainfall);
        for _ in 0..14 {
            water.tick(
                &ter.elev,
                &ter.flow.down,
                &rain0,
                hab.water_level,
                0.40,
                &mut wseed,
            );
        }
        Self {
            soil,
            weather,
            plants,
            lakes,
            water,
            trophic: TrophicFields::new(),
            agents,
            dwellings,
            chronicle: Chronicle::default(),
            day: 0.0,
            water_stock: 1.0e6,
            nitrogen_stock: n_sum,
            nitrogen_initial: n_sum,
            carbon_stock: 5.0e5,
            energy_stock: energy,
            atmosphere: Atmosphere::default(),
            greenhouses: Vec::new(),
            erosion_seed: hab.seed ^ 0xE20D,
            water_seed: wseed,
            hardness,
            last_flow_day: -10.0,
            last_survey_day: -10.0,
            last_talus_day: -10.0,
        }
    }

    pub fn tick(
        &mut self,
        ter: &mut Terrain,
        dt_days: f32,
        player_theta: f32,
        player_z: f32,
        heaps: &mut Vec<crate::economy::Stockpile>,
    ) -> TickReport {
        if dt_days <= 0.0 {
            return TickReport::default();
        }
        self.day += dt_days;

        self.weather.tick(dt_days, ter.hab.omega, &ter.elev);
        self.soil
            .tick(dt_days, &ter.elev, &ter.flow.flux, &self.weather.rainfall);
        self.trophic.tick(
            dt_days,
            &self.weather,
            &mut self.soil,
            ter.hab.length,
            ter.hab.radius,
        );

        let rain_full = upsample_rain(&self.weather.rainfall);
        // Keep droplet count gentle on realtime steps — large bursts hitch.
        let droplets = ((140.0 * dt_days).ceil() as u32).clamp(2, 80);
        let hard = &self.hardness;
        let moved = Erosion::tick(
            &mut ter.elev,
            &rain_full,
            |i| hard.get(i).copied().unwrap_or(1.2),
            droplets,
            &mut self.erosion_seed,
        );
        if moved > 1.0 {
            ter.flow.mark_dirty();
        }
        // One talus pass every ~2 habitat days — keep it off the soft sim cadence.
        let mut did_talus = false;
        if self.day - self.last_talus_day >= 2.0 {
            self.last_talus_day = self.day;
            erosion::talus_relax(&mut ter.elev, 0.85, 1);
            ter.flow.mark_dirty();
            did_talus = true;
        }

        // mark_dirty() clears the local patch → rebuild_local always fell through
        // to full priority-flood. Only pay that cost every ~0.7 habitat days,
        // and never on the same tick as talus (stacking was the soft hitch).
        if ter.flow.is_dirty() && !did_talus && (self.day - self.last_flow_day) >= 0.7 {
            self.last_flow_day = self.day;
            if (self.day as u32) % 11 == 0 {
                ter.flow.rebuild(&ter.elev, ter.hab.water_level);
            } else {
                ter.flow.rebuild_local(&ter.elev, ter.hab.water_level);
            }
            self.refresh_lakes(ter);
            if (self.day as u32) % 17 == 0 {
                self.hardness = build_hardness(ter);
            }
        }

        self.plants.tick(
            dt_days,
            &self.weather,
            &self.soil,
            &ter.hab,
            &self.greenhouses,
            &self.trophic,
        );

        // Agents after plants (item 1988) — they harvest what grew.
        self.agents.tick(
            dt_days,
            self.day,
            ter,
            &mut self.plants,
            &mut self.soil,
            &mut self.chronicle,
            heaps,
            player_theta,
            player_z,
        );

        // Dwellings after agents: population follows the land, and the land
        // moved this tick. Re-read the ground weekly — that is what makes
        // fixing a river a social act and not a landscaping one.
        self.dwellings.tick(dt_days);
        if (self.day as u32) % 7 == 3 && self.day - self.last_survey_day > 1.0 {
            self.last_survey_day = self.day;
            self.dwellings
                .resurvey(&ter.hab, &ter.elev, Some(&self.soil));
            self.dwellings.mark_contested(ter.hab.radius);
        }

        self.water.tick(
            &ter.elev,
            &ter.flow.down,
            &rain_full,
            ter.hab.water_level,
            dt_days,
            &mut self.water_seed,
        );
        // Waterline drain returns depth-metres; credit kg back to the ledger.
        let cell_area =
            (std::f32::consts::TAU * ter.hab.radius / NT as f32) * (ter.hab.length / NZ as f32);
        if self.water.drained > 0.0 {
            self.water_stock += self.water.drained * cell_area * 1000.0;
            self.water.drained = 0.0;
        }

        // Scrubbers burn reactor headroom to claw CO₂ back to O₂ (item 844).
        let scrub = self
            .atmosphere
            .scrub_mw
            .min(self.weather.power_headroom().max(0.0));
        if scrub > 1e-4 {
            self.atmosphere.tick_scrub(dt_days, scrub);
        } else {
            self.atmosphere.scrub_mw = self.atmosphere.scrub_mw.min(0.0);
        }

        let rain_mean = mean(&self.weather.rainfall);
        let temp_mean = mean_temp(&self.weather);
        self.lakes.tick(dt_days, rain_mean, temp_mean);

        // Closed N ledger: soil pool only. Plant tissue N is drawn from soil
        // when uptake is wired; inventing leaf×0.02 broke the golden gate.
        let n_now: f32 = self.soil.n.iter().sum();
        self.nitrogen_stock = n_now;
        self.energy_stock =
            (self.weather.reactor_power_budget - self.weather.power_used - scrub).max(0.0);

        TickReport {
            day: self.day,
            plants_alive: self.plants.plants.iter().filter(|p| p.alive).count() as u32,
            rain_mean,
            moisture_mean: mean(&self.soil.moisture),
            lake_cells: ter.flow.lake_count,
            lake_entities: self.lakes.lakes.len() as u32,
            pool_cells: self.water.wet_cells(),
            pool_depth: self.water.mean_depth(),
            route_sig: ter.flow.route_sig,
            sediment_moved: moved,
            nitrogen: self.nitrogen_stock,
            nitrogen_drift: (self.nitrogen_stock - self.nitrogen_initial)
                / self.nitrogen_initial.max(1.0),
            mean_npp: self.trophic.mean_npp(),
            max_fauna_kg: crate::trophic::max_body_mass_kg(&ter.hab, self.trophic.mean_npp(), 50.0),
            agents_alive: self.agents.agents.iter().filter(|a| a.alive).count() as u32,
            followers: self.dwellings.list.iter().map(|d| d.followers).sum(),
            works: self.dwellings.list.iter().map(|d| d.works).sum(),
            chronicle_len: self.chronicle.len() as u32,
            carcasses: self.trophic.carcasses.len() as u32,
            kills: self.trophic.kills,
            mean_fear: mean(&self.trophic.fear),
        }
    }

    pub fn refresh_lakes(&mut self, ter: &Terrain) {
        let cell_area =
            (std::f32::consts::TAU * ter.hab.radius / NT as f32) * (ter.hab.length / NZ as f32);
        self.lakes = Lakes::extract(
            &ter.elev,
            &ter.flow.filled,
            &ter.flow.lake,
            &ter.flow.discharge,
            ter.hab.radius,
            ter.hab.length,
            cell_area,
        );
        // Keep columns honest with new depression fill without wiping dig ponds.
        for i in 0..ter.elev.len() {
            if ter.flow.lake[i] != 0 {
                let d = (ter.flow.filled[i] - ter.elev[i]).clamp(0.0, 28.0) * 0.85;
                if d > self.water.depth[i] {
                    self.water.depth[i] = d;
                }
            }
        }
    }

    /// Refresh hardness in a dig neighbourhood (exposes strata proxy).
    pub fn refresh_hardness_at(&mut self, ter: &Terrain, ti: usize, zi: usize, radius: i32) {
        let max_e = ter.hab.max_elevation.max(1.0);
        let r = radius.max(4);
        for dz in -r..=r {
            let zz = zi as i32 + dz;
            if zz < 0 || zz >= NZ as i32 {
                continue;
            }
            for dt in -r..=r {
                let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                let i = idx(tt, zz as usize);
                let cut = (ter.elev0[i] - ter.elev[i]).max(0.0);
                let base = material::surface_hardness(ter.elev0[i], ter.flow.flux[i], max_e);
                let exposed = if cut > 28.0 {
                    material::info(material::id::BASALT).hardness
                } else if cut > 8.0 {
                    material::info(material::id::SANDSTONE).hardness
                } else {
                    base
                };
                if i < self.hardness.len() {
                    self.hardness[i] = exposed;
                }
            }
        }
        let cell_area =
            (std::f32::consts::TAU * ter.hab.radius / NT as f32) * (ter.hab.length / NZ as f32);
        let stock_m3 = (self.water_stock / 1000.0).max(0.0);
        let invent_budget = stock_m3.min(18.0);
        let used = self
            .water
            .rush_into_pit(&ter.elev, ti, zi, r, invent_budget, cell_area);
        self.water_stock = (self.water_stock - used * 1000.0).max(0.0);
    }

    pub fn biome_at(&self, ter: &Terrain, theta: f32, z: f32) -> u8 {
        let s = self.soil.sample(theta, z);
        let elev = ter.elevation(theta, z);
        let flux = ter.water_flux(theta, z);
        let temp = self.weather.temp_at_elev(theta, z, elev);
        let arid = self.weather.aridity_at(theta, z);
        let d = 2.0;
        let slope = (ter.elevation(theta + d / ter.hab.radius, z)
            - ter.elevation(theta - d / ter.hab.radius, z))
        .abs()
            + (ter.elevation(theta, z + d) - ter.elevation(theta, z - d)).abs();
        let soil_depth = (1.0 - slope / 8.0).clamp(0.05, 1.0) * (0.4 + 0.6 * s.organic);
        let mut bid = biome::classify_ex(
            s.moisture,
            temp,
            elev,
            slope / 6.0,
            flux,
            soil_depth,
            arid,
            ter.hab.water_level,
        );
        // Soft prior: engineered farmland reads as FARM when the ground is
        // still ploughable — does not override water / rock / shore.
        let prov = crate::province::province_at(&ter.hab, theta, z);
        let farm_w = prov.weight(crate::province::id::FARMLAND);
        if farm_w > 0.48
            && !matches!(
                bid,
                biome::id::WATER | biome::id::BARE_ROCK | biome::id::SHORE | biome::id::ALPINE
            )
            && slope / 6.0 < 0.28
        {
            bid = biome::id::FARM;
        }
        bid
    }
}

#[derive(Clone, Copy, Default)]
pub struct TickReport {
    pub day: f32,
    pub plants_alive: u32,
    pub rain_mean: f32,
    pub moisture_mean: f32,
    pub lake_cells: u32,
    pub lake_entities: u32,
    pub pool_cells: u32,
    pub pool_depth: f32,
    pub route_sig: u64,
    pub sediment_moved: f32,
    pub nitrogen: f32,
    pub nitrogen_drift: f32,
    pub mean_npp: f32,
    pub max_fauna_kg: f32,
    pub agents_alive: u32,
    pub followers: f32,
    pub works: u32,
    pub chronicle_len: u32,
    pub carcasses: u32,
    pub kills: u32,
    pub mean_fear: f32,
}

fn build_hardness(ter: &Terrain) -> Vec<f32> {
    let max_e = ter.hab.max_elevation.max(1.0);
    let mut h = vec![1.2f32; NT * NZ];
    for z in 0..NZ {
        for t in 0..NT {
            let i = idx(t, z);
            h[i] = material::surface_hardness(ter.elev0[i], ter.flow.flux[i], max_e);
        }
    }
    h
}

fn mean(v: &[f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.iter().sum::<f32>() / v.len() as f32
}

fn mean_temp(w: &Weather) -> f32 {
    // Weather stores band temps; sample a few longitudes.
    let mut s = 0.0f32;
    for i in 0..8 {
        let th = i as f32 / 8.0 * std::f32::consts::TAU;
        s += w.temp_at(th, 0.0);
    }
    s / 8.0
}

fn upsample_rain(half: &[f32]) -> Vec<f32> {
    let mut full = vec![0.0f32; NT * NZ];
    if half.len() != ST * SZ {
        return full;
    }
    for z in 0..NZ {
        for t in 0..NT {
            let st = (t / 2).min(ST - 1);
            let sz = (z / 2).min(SZ - 1);
            full[t + z * NT] = half[st + sz * ST];
        }
    }
    full
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habitat::Habitat;
    use crate::terrain::Terrain;

    #[test]
    fn nitrogen_holds_over_100_days() {
        let hab = Habitat::kepler_drum();
        let mut ter = Terrain::generate(hab);
        let mut bio = Biosphere::new(&ter);
        let n0 = bio.nitrogen_initial;
        assert!(n0 > 1000.0, "expected substantial N stock, got {n0}");
        // Coarse steps — invariant is conservation, not daily fidelity.
        for _ in 0..50 {
            bio.tick(&mut ter, 2.0, 0.0, 0.0, &mut Vec::new());
        }
        let drift = (bio.nitrogen_stock - n0).abs() / n0;
        assert!(
            drift < 0.08,
            "nitrogen drifted {drift:.3} over 100 days (stock {} vs {})",
            bio.nitrogen_stock,
            n0
        );
        assert!(bio.day >= 99.0);
    }

    #[test]
    fn dig_pit_fills_with_water() {
        let hab = Habitat::kepler_drum();
        let mut ter = Terrain::generate(hab);
        let mut bio = Biosphere::new(&ter);
        let th = 1.2f32;
        let z = 80.0f32;
        let surf = ter.hab.radius - ter.elevation(th, z);
        let p = ter.hab.to_world(th, z, surf);
        assert!(ter.dig(p, 8.0, 0.0, false).is_some());
        let ti = (th.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT as f32).round()
            as usize
            % NT;
        let zi = ((z / ter.hab.length + 0.5) * NZ as f32)
            .round()
            .clamp(0.0, (NZ - 1) as f32) as usize;
        for dz in -6i32..=6 {
            for dt in -6i32..=6 {
                if dt.abs() < 2 && dz.abs() < 2 {
                    continue;
                }
                let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                let zz = (zi as i32 + dz).clamp(1, NZ as i32 - 2) as usize;
                bio.water.depth[idx(tt, zz)] = 1.5;
            }
        }
        bio.water.rush_into_pit(&ter.elev, ti, zi, 10, 40.0, 20.0);
        let mut best = 0.0f32;
        for dz in -4i32..=4 {
            for dt in -4i32..=4 {
                let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                let zz = (zi as i32 + dz).clamp(1, NZ as i32 - 2) as usize;
                best = best.max(bio.water.depth[idx(tt, zz)]);
            }
        }
        assert!(
            best > 0.25,
            "expected pit neighbourhood to flood, best depth {best}"
        );
    }
}
