//! Coupled biosphere tick — soil, weather, plants, erosion, flow, lakes.
//! Owns everything that makes regions affect each other.

use crate::agent::AgentSim;
use crate::biome;
use crate::chronicle::Chronicle;
use crate::debris::Debris;
use crate::dwelling::Dwellings;
use crate::economy::{Atmosphere, Greenhouse};
use crate::erosion::{self, Erosion};
use crate::lakes::Lakes;
use crate::material;
use crate::plant::PlantSim;
use crate::pyro::Pyro;
use crate::soil::{Soil, ST, SZ};
use crate::sph::SurfaceWater;
use crate::terrain::{idx, Terrain, NT, NZ};
use crate::trophic::TrophicFields;
use crate::weather::Weather;
use crate::woodscape::{Sprout, Woodscape};

/// How long a player edit may wait for the water to answer, in habitat days.
/// 0.01 is half a real second at 1x. A local repair costs 1.4 ms, so this is
/// set by how long a burst of digging should coalesce for, not by what the
/// rebuild costs.
const FLOW_URGENT_DAYS: f32 = 0.01;

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
    /// Connected wood/leaf voxels — Minecraft combine + env growth.
    pub woodscape: Woodscape,
    /// Falling / rolling / floating debris bodies.
    pub debris: Debris,
    /// Heat, fire, lava, steam.
    pub pyro: Pyro,
}

/// Biome from the site fields alone — the same reading `Biosphere::biome_at`
/// makes, but callable before the biosphere exists (species seating runs
/// during construction).
fn biome_at_site(soil: &Soil, weather: &Weather, ter: &Terrain, theta: f32, z: f32) -> u8 {
    let s = soil.sample(theta, z);
    let elev = ter.elevation(theta, z);
    let flux = ter.water_flux(theta, z);
    let temp = weather.temp_at_elev(theta, z, elev);
    let arid = weather.aridity_at(theta, z);
    let d = 2.0;
    let slope = (ter.elevation(theta + d / ter.hab.radius, z)
        - ter.elevation(theta - d / ter.hab.radius, z))
    .abs()
        + (ter.elevation(theta, z + d) - ter.elevation(theta, z - d)).abs();
    let soil_depth = (1.0 - slope / 8.0).clamp(0.05, 1.0) * (0.4 + 0.6 * s.organic);
    biome::classify_ex(
        s.moisture,
        temp,
        elev,
        slope / 6.0,
        flux,
        soil_depth,
        arid,
        ter.hab.water_level,
    )
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
        plants.seed_stands(&hab, &ter.elev, &ter.flow.flux, &soil, 22_000);
        // Seat every species once, here, from the site as generated. After this
        // point a tree's identity is a stored fact, not a query against the
        // current weather (forest plan, stage 1: identity and authority).
        for i in 0..plants.plants.len() {
            let (th, z) = (plants.plants[i].theta, plants.plants[i].z);
            let g = plants.plants[i].genome_id;
            let bid = biome_at_site(&soil, &weather, ter, th, z);
            plants.plants[i].species = crate::tree_form::species_at(bid, &weather, ter, th, z, g);
        }
        let mut agents = AgentSim::new();
        agents.seed_farmers(&hab, &ter.elev, 10);
        // Site each principal on the ground their traits reach for, then move
        // them to it — a man lives where he chose to live, not where the
        // scatter dropped him. Authored cast keep a tight seat so province
        // identity survives (`docs/CAST_EIGHT.md`).
        let mut dwellings = Dwellings::default();
        for ag in &agents.agents {
            // Authored cast + Ren's meadow neighbour stay local so watershed
            // contest and province seats survive.
            let tight = ag.romanceable || ag.name == "Pax";
            if tight {
                dwellings.site_near(
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
            } else {
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
        }
        // Casimir's skyline: start with a tower of works.
        if let Some(cas) = agents.agents.iter().find(|a| a.name == "Casimir") {
            if let Some(d) = dwellings.list.iter_mut().find(|d| d.agent_id == cas.id) {
                d.works = 8;
                d.followers = 4.0;
            }
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
        agents.seed_cast_modules(&hab);
        let mut greenhouses = Vec::new();
        for m in &agents.cast_modules {
            match m.kind {
                crate::agent::module_kind::GREENHOUSE => {
                    greenhouses.push(Greenhouse {
                        theta: m.theta,
                        z: m.z,
                        radius: 12.0,
                    });
                }
                crate::agent::module_kind::CONDENSER => {
                    let _ = weather.try_add_condenser(m.theta, m.z, 1.0);
                }
                _ => {}
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
            greenhouses,
            erosion_seed: hab.seed ^ 0xE20D,
            water_seed: wseed,
            hardness,
            last_flow_day: -10.0,
            last_survey_day: -10.0,
            last_talus_day: -10.0,
            woodscape: Woodscape::default(),
            debris: Debris::default(),
            pyro: Pyro::default(),
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
            // Material-aware: sand lets go, rock holds, and what slumps
            // drifts anti-spinward with the drum's Coriolis term.
            let max_e = ter.hab.max_elevation;
            let (rad, om) = (ter.hab.radius, ter.hab.omega);
            let flux = std::mem::take(&mut ter.flow.flux);
            erosion::talus_relax_material(&mut ter.elev, &flux, max_e, rad, om, 1);
            ter.flow.flux = flux;
            ter.flow.mark_dirty();
            did_talus = true;
        }

        // mark_dirty() clears the local patch → rebuild_local always fell through
        // to full priority-flood. Rebuilding costs ~50 ms local, ~195 ms full,
        // plus 7 ms to re-extract lakes, so it cannot run every tick.
        //
        // But erosion and a player's shovel do not deserve the same deadline.
        // The erosion cadence is 0.7 habitat days, which at 1x is 35 real
        // seconds — so for over half a minute after cutting a channel out of a
        // lake, the water simply sat there, which is exactly what it looked
        // like. A deliberate edit gets FLOW_URGENT_DAYS instead: long enough
        // that holding the dig button coalesces one rebuild rather than one
        // per bite, short enough that the water answers while you are still
        // looking at the hole. It also overrides the talus exclusion, because
        // putting off a slump by one tick is invisible and putting off the
        // water is the bug.
        let mut water_rebuilt = false;
        let urgent = ter.flow.is_urgent();
        let waited = self.day - self.last_flow_day;
        if ter.flow.is_dirty()
            && (!did_talus || urgent)
            && waited >= if urgent { FLOW_URGENT_DAYS } else { 0.7 }
        {
            self.last_flow_day = self.day;
            // A player edit takes the cheap local path even when a full flood
            // is due: the 240 ms belongs on a tick the player is not mid-swing
            // for, and `wants_full` keeps asking until it gets one.
            if !urgent && ((self.day as u32) % 11 == 0 || ter.flow.wants_full()) {
                ter.flow.rebuild(&ter.elev, ter.hab.water_level);
            } else {
                ter.flow.rebuild_local(&ter.elev, ter.hab.water_level);
            }
            self.refresh_lakes(ter);
            water_rebuilt = true;
            if (self.day as u32) % 17 == 0 {
                self.hardness = build_hardness(ter);
            }
        }

        // Hot half, then debris — a log that lands in fire ignites this tick,
        // and lava that solidified is already ground under a body.
        self.pyro.tick(
            ter,
            &mut self.woodscape,
            &mut self.soil,
            &self.weather,
            &mut self.atmosphere,
            &mut self.water.depth,
            &mut self.water_stock,
            heaps,
            &mut self.debris,
            dt_days,
        );
        self.debris.tick(
            ter,
            &self.water.depth,
            heaps,
            dt_days,
            player_theta,
            player_z,
        );

        self.plants.tick(
            dt_days,
            &self.weather,
            &self.soil,
            &ter.hab,
            &self.greenhouses,
            &self.trophic,
        );
        // Wood/leaf voxels grow onto each other; moisture/aridity gate flush.
        self.woodscape.tick(
            &self.plants,
            ter,
            &self.soil,
            &self.weather,
            ((48.0 * dt_days).ceil() as usize).clamp(8, 64),
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

        // Spoil answers to the weather like everything else on the ground.
        self.weather_heaps(ter, heaps, dt_days);

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
            water_rebuilt,
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
        // Keep columns honest with the new depression fill, in both
        // directions, without wiping dig ponds.
        //
        // This used to raise depth only. That made a drained basin permanent:
        // cut an outlet, the fill level drops, and the column kept the water it
        // had because nothing here would take it away. The pool mesh reads
        // `water.depth`, so the lake you had just emptied went on being drawn.
        //
        // Falling is deliberately gentler than rising, and never goes below
        // what the basin now holds: draining reads as a level going down over a
        // few seconds rather than water vanishing between two frames, and a
        // cell that has left its basin altogether is left to the surface-water
        // tick, which runs it downhill like any other puddle.
        for i in 0..ter.elev.len() {
            if ter.flow.lake[i] == 0 {
                continue;
            }
            let d = (ter.flow.filled[i] - ter.elev[i]).clamp(0.0, 28.0) * 0.85;
            let have = self.water.depth[i];
            self.water.depth[i] = if d > have {
                d
            } else {
                have + (d - have) * 0.35
            };
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

    /// Bring nearby stands into the voxel grid and drop ones left behind.
    ///
    /// Lives here rather than in the Godot binding because this is the rule
    /// that decides whether a tree you can see is a thing you can cut, and
    /// that rule wants a test that does not need an engine to run.
    ///
    /// Returns how many stands were realised. `budget` caps the work per call
    /// so arriving at a dense grove costs several frames of stamping rather
    /// than one visible hitch.
    pub fn realise_stands(
        &mut self,
        ter: &Terrain,
        center: [f32; 3],
        rad: f32,
        budget: usize,
    ) -> u32 {
        // Unload first: the grid is capped, and a full grid would otherwise
        // refuse the stand you are walking toward while still holding one a
        // kilometre behind.
        self.woodscape.stream_around(center, rad);

        let picks: Vec<(usize, crate::plant::Plant, u8, f32)> = {
            let r2 = (rad + 24.0) * (rad + 24.0);
            let hab_r = ter.hab.radius;
            let (cth, cz, _) = ter.hab.to_cyl(center);
            let mut candidates: Vec<_> = self
                .plants
                .plants
                .iter()
                .enumerate()
                .filter(|(i, p)| p.alive && self.woodscape.can_sprout(*i as u32))
                .map(|(i, p)| {
                    let arc = (p.theta - cth).rem_euclid(std::f32::consts::TAU);
                    let arc = arc.min(std::f32::consts::TAU - arc) * hab_r;
                    (arc * arc + (p.z - cz).powi(2), i, p)
                })
                .filter(|(d, _, _)| *d <= r2)
                .collect();
            candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            candidates
                .into_iter()
                .take(budget)
                .map(|(d2, i, p)| {
                    let bid = self.biome_at(ter, p.theta, p.z);
                    let form = crate::tree_form::species_of(p, bid, &self.weather, ter);
                    (i, p.clone(), form, d2.sqrt())
                })
                .collect()
        };

        let mut made = 0u32;
        for (i, p, form, dist) in picks {
            // Evict before refusing. A stand you can walk up to has to have
            // material even when the budget is spent on stands behind you;
            // distant unedited stands are reconstructible from the seed, so
            // they are what gives way.
            match self.woodscape.sprout_plant(i, &p, ter, form) {
                Sprout::Made => made += 1,
                Sprout::NoRoom(need) => {
                    if self.woodscape.make_room_for(center, dist, need) == 0
                        || self.woodscape.sprout_plant(i, &p, ter, form) != Sprout::Made
                    {
                        // Nothing droppable is farther away than this stand,
                        // so it genuinely does not fit. Remember that, at this
                        // cell count, so the next frame does not re-stage it.
                        self.woodscape.note_refusal(i as u32);
                        continue;
                    }
                    made += 1;
                }
            }
        }
        made
    }

    /// Weather the spoil the player left lying about.
    ///
    /// A heap was permanent and inert: tip a tonne of silt into a stream bed
    /// and it sat there for good, which made the landscape's own rules stop at
    /// the edge of anything you had touched. Two things happen to a real pile.
    ///
    /// Running water takes the fines out of it — `material::info().fines` is
    /// already the fraction of a material that behaves as silt — and carries
    /// them to the cell downstream, where they pile up. So a heap in a channel
    /// migrates down the channel and coarsens as it goes, and one on dry
    /// ground stays put.
    ///
    /// Bare spoil sheds fines everywhere, not only in channels — it is the
    /// most erodible thing in a landscape, which is why real spoil heaps get
    /// seeded — but a heap in a channel loses them about eight times faster.
    ///
    /// And a pile steeper than its own angle of repose creeps downhill.
    /// `material::repose_tan` is the same function the talus pass uses on the
    /// terrain itself, so spoil and bedrock answer to one rule.
    ///
    /// Only dug material weathers. Bio ids (100 and up) are timber and
    /// foliage: a log pile does not lose fines, it floats, and floating is a
    /// different model than this one.
    pub fn weather_heaps(
        &mut self,
        ter: &Terrain,
        heaps: &mut Vec<crate::economy::Stockpile>,
        dt_days: f32,
    ) {
        if heaps.is_empty() || dt_days <= 0.0 {
            return;
        }
        let tau = std::f32::consts::TAU;
        let cell_t = tau * ter.hab.radius / NT as f32;
        let cell_z = ter.hab.length / NZ as f32;
        let cell_of = |theta: f32, z: f32| -> usize {
            let ti = (theta.rem_euclid(tau) / tau * NT as f32).round() as usize % NT;
            let zi = ((z / ter.hab.length + 0.5) * NZ as f32)
                .round()
                .clamp(0.0, (NZ - 1) as f32) as usize;
            idx(ti, zi)
        };
        let pos_of = |i: usize| -> (f32, f32) {
            let (ti, zi) = (i % NT, i / NT);
            (
                ti as f32 / NT as f32 * tau,
                (zi as f32 / NZ as f32 - 0.5) * ter.hab.length,
            )
        };

        let mut washed: Vec<(f32, f32, u8, f32, f32, f32)> = Vec::new();
        for h in heaps.iter_mut() {
            if h.material_id >= 100 {
                continue;
            }
            let i = cell_of(h.theta, h.z);
            let dn = ter.flow.down[i] as usize;
            if dn == i {
                continue; // a pit or an outlet: nowhere for it to go
            }
            let (dth, dz) = pos_of(dn);

            // --- fines washed out by running water
            let flux = ter.flow.flux[i];
            let depth = self.water.depth[i];
            let wet = (flux * 1.4 + depth.min(1.0) * 0.6).min(1.0);
            let fines = material::info(h.material_id).fines;
            let rate = (wet * fines * 0.6 * dt_days).clamp(0.0, 0.4);
            if rate > 1e-4 && h.mass_kg > 0.5 {
                let lost = h.mass_kg * rate;
                let frac = lost / h.mass_kg;
                let vol = h.loose_m3 * frac;
                h.mass_kg -= lost;
                h.loose_m3 -= vol;
                washed.push((dth, dz, h.material_id, lost, vol, h.grade));
            }

            // --- creep, when the ground under it is steeper than it can hold
            let drop = ter.elev[i] - ter.elev[dn];
            let run = (cell_t.min(cell_z)).max(0.1);
            if drop / run > material::repose_tan(h.material_id) {
                let step = (2.5 * dt_days).min(1.0);
                h.theta += (dth - h.theta) * step;
                h.z += (dz - h.z) * step;
            }
        }
        heaps.retain(|h| h.mass_kg > 0.5);
        for (theta, z, id, kg, vol, grade) in washed {
            crate::economy::deposit_heap(heaps, ter.hab.radius, theta, z, id, kg, vol, grade);
        }
    }

    /// Put `m3` of water on the ground at one spot.
    ///
    /// Used by ice melt: the water a cut releases has to appear where the cut
    /// is, not as an abstract rise in the ledger.
    pub fn pond_at(&mut self, ter: &Terrain, theta: f32, z: f32, m3: f32) {
        if m3 <= 0.0 {
            return;
        }
        let ti = (theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT as f32)
            .round() as usize
            % NT;
        let zi = ((z / ter.hab.length + 0.5) * NZ as f32)
            .round()
            .clamp(0.0, (NZ - 1) as f32) as usize;
        let cell_area =
            (std::f32::consts::TAU * ter.hab.radius / NT as f32) * (ter.hab.length / NZ as f32);
        let i = idx(ti, zi);
        self.water.depth[i] = (self.water.depth[i] + m3 / cell_area.max(1e-4)).min(6.0);
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
    /// Routing and lakes were re-extracted on this tick, so anything drawn
    /// from them — the pool mesh, the river ribbons — is now stale. The client
    /// cannot infer this from `route_sig`: a dig in the middle of a slope
    /// moves water without changing a single downstream pointer.
    pub water_rebuilt: bool,
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

    /// Forest plan, stage 1: "streaming cannot change identity or dimensions",
    /// and "weather or a biome threshold cannot change an existing tree into
    /// another species."
    #[test]
    fn species_is_seated_once_and_survives_a_changing_climate() {
        let hab = Habitat::kepler_drum();
        let mut ter = Terrain::generate(hab);
        let mut bio = Biosphere::new(&ter);

        let sample: Vec<(usize, u8)> = bio
            .plants
            .plants
            .iter()
            .enumerate()
            .filter(|(_, p)| p.alive)
            .step_by(211)
            .map(|(i, p)| (i, p.species))
            .collect();
        assert!(
            sample.len() > 20,
            "expected a decent sample, got {}",
            sample.len()
        );
        for (i, sp) in &sample {
            assert!(
                *sp != crate::plant::UNASSIGNED,
                "plant {i} never had a species seated"
            );
            assert!(
                *sp < 8,
                "plant {i} has species {sp}, outside the form family"
            );
        }

        // Run the world long enough for weather, soil and biomes to move.
        for _ in 0..25 {
            bio.tick(&mut ter, 2.0, 0.0, 0.0, &mut Vec::new());
        }

        for (i, sp) in &sample {
            assert_eq!(
                bio.plants.plants[*i].species, *sp,
                "plant {i} changed species while standing there"
            );
        }
    }

    /// The whole population should not be one species, nor a uniform sample of
    /// all eight — the plan asks for local continuity, not confetti.
    #[test]
    fn seated_species_are_mixed_but_not_uniform() {
        let hab = Habitat::kepler_drum();
        let ter = Terrain::generate(hab);
        let bio = Biosphere::new(&ter);
        let mut hist = [0u32; 8];
        let mut n = 0u32;
        for p in bio.plants.plants.iter().filter(|p| p.alive) {
            if (p.species as usize) < 8 {
                hist[p.species as usize] += 1;
                n += 1;
            }
        }
        assert!(n > 1000, "too few living plants to judge: {n}");
        let present = hist.iter().filter(|c| **c > 0).count();
        assert!(present >= 3, "only {present} species present: {hist:?}");
        let biggest = *hist.iter().max().unwrap() as f32 / n as f32;
        assert!(
            biggest < 0.90,
            "one species is {:.0}% of the drum: {hist:?}",
            biggest * 100.0
        );
    }

    /// Stage 0 baseline of `FOREST_IMPLEMENTATION_PLAN.md`. Records what the
    /// current generator produces and ratchets it, so stages 2-3 have numbers
    /// to move and nothing silently gets worse first.
    ///
    /// Measured on the shipped seed, 2026-09-08:
    ///
    /// ```text
    /// alive 22000
    /// species  [0, 6052, 18, 8605, 20, 1594, 4431, 1280]
    /// height   [0, 1741, 6973, 1512, 1203, 9503, 1068]
    /// spacing  [0, 0, 790, 4660, 11124, 4744, 0]
    /// mean h 19.3 m, max 42.0 m, at-clamp 14675 (67%)
    /// mean nearest neighbour 18.2 m, trunk overlaps 236
    /// ```
    ///
    /// What those numbers say, against the plan's own design rules:
    ///
    /// * **67% of the population sits at its species height clamp** — exactly
    ///   the failure section 2 predicted ("seeded biomass combined with height
    ///   clamps can push many trees toward the same maximum"). Stage 2's gate
    ///   is to bring this under 30% with all size classes legible.
    /// * **No seedling class at all** (first height bucket is empty) and the
    ///   largest single bucket is 26-40 m. There is no regeneration to see.
    /// * **Three of eight species are effectively absent** (0, 18 and 20
    ///   individuals) while two make up 67% of the drum. Section 5.2 wants
    ///   local continuity, not two species everywhere and six as rounding.
    /// * **Spacing clusters in one 12-25 m band** — the signature of
    ///   fill-to-a-global-count placement. Section 5.3 replaces that with
    ///   patch candidates, so density becomes an outcome.
    /// * **236 trunks occupy one another**, which design rule 4 forbids
    ///   outright. Crown overlap is wanted; trunk intersection is not.
    #[test]
    fn forest_baseline_histograms() {
        let hab = Habitat::kepler_drum();
        let ter = Terrain::generate(hab);
        let bio = Biosphere::new(&ter);
        let st = crate::forest::stats(&bio.plants, &ter.hab);
        println!("BASELINE {}", crate::forest::summary(&st));
        assert!(st.alive > 1000, "too few plants to measure: {}", st.alive);

        // Ratchets, set just above the measurements above. These are allowed to
        // move DOWN as the forest work lands; they must not move up.
        let clamped = st.at_clamp as f32 / st.alive as f32;
        assert!(
            clamped <= 0.70,
            "{:.0}% of the forest is at its height clamp (baseline 67%, stage 2 \
             target under 30%). histogram {:?}",
            clamped * 100.0,
            st.height
        );
        assert!(
            st.trunk_overlaps <= 260,
            "{} trunks intersect (baseline 236, design rule 4 wants zero)",
            st.trunk_overlaps
        );
        let present = st.height.iter().filter(|c| **c > 0).count();
        assert!(
            present >= 3,
            "only {present} height classes exist: {:?}",
            st.height
        );
    }

    /// Impact probe for the `soil.rs` leach fix: how much the biome mix and
    /// the nitrogen ledger move. Printed, not asserted — the point is evidence
    /// for a decision about world generation, not a threshold.
    #[test]
    #[ignore = "diagnostic: cargo test -- --ignored soil_leach_impact"]
    fn soil_leach_impact() {
        let hab = Habitat::kepler_drum();
        let mut ter = Terrain::generate(hab);
        let mut bio = Biosphere::new(&ter);
        let n0 = bio.nitrogen_initial;
        let mut census = std::collections::BTreeMap::new();
        for zi in 0..64 {
            for ti in 0..64 {
                let th = ti as f32 / 64.0 * std::f32::consts::TAU;
                let z = (zi as f32 / 64.0 - 0.5) * hab.length * 0.9;
                *census.entry(bio.biome_at(&ter, th, z)).or_insert(0u32) += 1;
            }
        }
        for _ in 0..50 {
            bio.tick(&mut ter, 2.0, 0.0, 0.0, &mut Vec::new());
        }
        let drift = (bio.nitrogen_stock - n0).abs() / n0;
        println!(
            "LEACH n0 {n0:.0} -> {:.0}  drift {drift:.4}",
            bio.nitrogen_stock
        );
        println!("LEACH biomes {census:?}");
    }

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
        // Coarse 2-day steps + plant/agent uptake leave a few percent of float
        // and pool churn; this gate catches catastrophic leaks, not daily fidelity.
        assert!(
            // Was relaxed to 0.12 to accommodate a leak; `soil.rs` now
            // accumulates leached nitrogen instead of overwriting it, and the
            // measured drift is 0.0000. Tightened rather than left slack.
            drift < 0.02,
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

    /// Forest plan, section 9: "A visible nearby trunk must not be
    /// unharvestable because other stands filled the cache. Evict
    /// reconstructible distant chunks before refusing interaction."
    ///
    /// This is the regression for the bug where you could walk up to a tree,
    /// swing, hear the swing, and take nothing. Twenty stands at ~6,700 cells
    /// each filled the 140,000-cell budget, and every stand after that was
    /// drawn as an instanced proxy with no material behind it. The ray found
    /// no cells, the dig fell through to open air, and the tree could never
    /// be cut.
    #[test]
    fn a_tree_you_walk_to_is_realised_even_on_a_full_grid() {
        let hab = Habitat::kepler_drum();
        let ter = Terrain::generate(hab);
        let mut bio = Biosphere::new(&ter);

        let feet = |bio: &Biosphere, i: usize| {
            let p = &bio.plants.plants[i];
            let r = ter.hab.radius - ter.elevation(p.theta, p.z);
            ter.hab.to_world(p.theta, p.z, r)
        };

        // Spend the budget standing in one place, as walking around does.
        let home = feet(&bio, 0);
        for _ in 0..200 {
            bio.realise_stands(&ter, home, 92.0, 3);
        }
        let spent = bio.woodscape.len();
        assert!(
            spent > MAX_CELLS_TEST_FLOOR,
            "budget should be near spent, was {spent}"
        );

        // Now pick a live plant well away from there and stand at its foot.
        let (target, tpos) = bio
            .plants
            .plants
            .iter()
            .enumerate()
            .find(|(i, p)| {
                p.alive && !bio.woodscape.is_sprouted(*i as u32) && {
                    let w = {
                        let r = ter.hab.radius - ter.elevation(p.theta, p.z);
                        ter.hab.to_world(p.theta, p.z, r)
                    };
                    let d2: f32 = (0..3).map(|k| (w[k] - home[k]).powi(2)).sum();
                    d2 > 300.0 * 300.0
                }
            })
            .map(|(i, _)| (i, feet(&bio, i)))
            .expect("some unrealised plant 300 m away");

        for _ in 0..8 {
            bio.realise_stands(&ter, tpos, 92.0, 3);
        }

        assert!(
            bio.woodscape.is_sprouted(target as u32),
            "walked to plant {target} and it still has no material: grid holds \
             {} cells across {} stands",
            bio.woodscape.len(),
            bio.woodscape.stand_count(),
        );
        assert!(
            bio.woodscape.material_within(tpos, 12.0),
            "plant {target} is resident but there is nothing to hit within 12 m"
        );
    }

    /// Two neighbouring trees must not take turns evicting each other: a full
    /// grid that thrashes costs a mesh rebuild every frame and shows trees
    /// blinking in and out.
    #[test]
    fn a_full_grid_settles_instead_of_thrashing() {
        let hab = Habitat::kepler_drum();
        let ter = Terrain::generate(hab);
        let mut bio = Biosphere::new(&ter);
        let p = &bio.plants.plants[0];
        let home = ter
            .hab
            .to_world(p.theta, p.z, ter.hab.radius - ter.elevation(p.theta, p.z));

        for _ in 0..200 {
            bio.realise_stands(&ter, home, 92.0, 3);
        }
        // Standing still, nothing more should come or go.
        let before = (bio.woodscape.len(), bio.woodscape.stand_count());
        for _ in 0..20 {
            bio.realise_stands(&ter, home, 92.0, 3);
        }
        assert_eq!(
            before,
            (bio.woodscape.len(), bio.woodscape.stand_count()),
            "a stationary viewer churned the grid"
        );
    }

    /// Half the budget, so "near spent" is a real claim rather than a
    /// tautology if `MAX_CELLS` ever changes.
    const MAX_CELLS_TEST_FLOOR: usize = 70_000;

    /// "Water isn't really updating after I dig either."
    ///
    /// The routing rebuild sat behind the erosion cadence — 0.7 habitat days,
    /// 35 real seconds at 1x — so a freshly cut trench changed nothing you
    /// could see for over half a minute.
    #[test]
    fn a_dig_gets_the_water_rebuilt_in_seconds_not_a_habitat_day() {
        let hab = Habitat::kepler_drum();
        let mut ter = Terrain::generate(hab);
        let mut bio = Biosphere::new(&ter);

        // Settle first. `is_dirty` is no use as the observable here: erosion
        // sets it on almost every tick, which is exactly why a player edit
        // needs a flag of its own to be distinguishable from background creep.
        for _ in 0..40 {
            bio.tick(&mut ter, 0.03, 1.0, 0.0, &mut Vec::new());
        }
        // Colonists can dig during settle and raise urgent; drain that so the
        // assertion below is about *our* dig, not theirs.
        let mut drain = 0.0f32;
        while ter.flow.is_urgent() && drain < 1.0 {
            bio.tick(&mut ter, 0.01, 1.0, 0.0, &mut Vec::new());
            drain += 0.01;
        }
        assert!(!ter.flow.is_urgent(), "nothing has been dug yet");

        let c = ter
            .hab
            .to_world(1.0, 0.0, ter.hab.radius - ter.elevation(1.0, 0.0));
        ter.dig(c, 1.6, 1.0, false);
        assert!(ter.flow.is_urgent(), "a dig is a deliberate edit");

        // Two real seconds is 0.04 habitat days at 1x.
        let mut days = 0.0f32;
        while ter.flow.is_urgent() && days < 0.2 {
            bio.tick(&mut ter, 0.01, 1.0, 0.0, &mut Vec::new());
            days += 0.01;
        }
        assert!(
            !ter.flow.is_urgent(),
            "water still had not been rebuilt after {days:.2} habitat days"
        );
        assert!(
            days <= FLOW_URGENT_DAYS + 0.02,
            "took {days:.2} habitat days; urgent budget is {FLOW_URGENT_DAYS}"
        );
    }

    /// A basin you drain has to actually empty. `refresh_lakes` only ever
    /// raised column depth, so cutting an outlet dropped the fill level and
    /// the pool mesh — which reads `water.depth` — went on drawing the lake.
    #[test]
    fn draining_a_basin_lowers_the_water_it_used_to_hold() {
        let hab = Habitat::kepler_drum();
        let mut ter = Terrain::generate(hab);
        let mut bio = Biosphere::new(&ter);
        ter.flow.rebuild(&ter.elev, ter.hab.water_level);
        bio.refresh_lakes(&ter);

        // A cell inside a real depression, holding real water.
        let deep = (0..ter.elev.len())
            .filter(|&i| ter.flow.lake[i] != 0)
            .max_by(|&a, &b| {
                (ter.flow.filled[a] - ter.elev[a]).total_cmp(&(ter.flow.filled[b] - ter.elev[b]))
            })
            .expect("the drum has depressions");
        let held = bio.water.depth[deep];
        assert!(held > 0.5, "picked a dry cell: depth {held}");

        // Take the basin away underneath it: no fill, no lake.
        for i in 0..ter.elev.len() {
            ter.flow.filled[i] = ter.elev[i];
            ter.flow.lake[i] = 1;
        }
        for _ in 0..12 {
            bio.refresh_lakes(&ter);
        }
        assert!(
            bio.water.depth[deep] < held * 0.25,
            "drained basin still holds {:.2} m of the original {held:.2} m",
            bio.water.depth[deep]
        );
    }

    /// Spoil tipped into a channel has to move; spoil on dry ground has to
    /// stay. Before this, a heap was permanent and inert wherever you left it.
    #[test]
    fn water_carries_a_spoil_heap_downstream() {
        let hab = Habitat::kepler_drum();
        let mut ter = Terrain::generate(hab);
        let mut bio = Biosphere::new(&ter);
        ter.flow.rebuild(&ter.elev, ter.hab.water_level);

        // The wettest cell that still has somewhere to drain to. The very
        // wettest is the drum's outlet, which is a self-loop: a heap there has
        // nowhere downstream to go and is left alone by design.
        let wet = (0..ter.elev.len())
            .filter(|&i| ter.flow.down[i] as usize != i)
            .max_by(|&a, &b| ter.flow.flux[a].total_cmp(&ter.flow.flux[b]))
            .unwrap();
        let dry = (0..ter.elev.len())
            .filter(|&i| ter.flow.down[i] as usize != i)
            .min_by(|&a, &b| ter.flow.flux[a].total_cmp(&ter.flow.flux[b]))
            .unwrap();
        let pos = |i: usize| {
            let (ti, zi) = (i % NT, i / NT);
            (
                ti as f32 / NT as f32 * std::f32::consts::TAU,
                (zi as f32 / NZ as f32 - 0.5) * ter.hab.length,
            )
        };

        let mut heaps = Vec::new();
        for (i, id) in [(wet, material::id::SEDIMENT), (dry, material::id::SEDIMENT)] {
            let (th, z) = pos(i);
            crate::economy::deposit_heap(&mut heaps, ter.hab.radius, th, z, id, 900.0, 0.6, 0.0);
        }
        assert_eq!(heaps.len(), 2);
        let (wet_th, wet_z) = pos(wet);

        for _ in 0..12 {
            bio.weather_heaps(&ter, &mut heaps, 0.05);
        }

        let on_wet: f32 = heaps
            .iter()
            .filter(|h| {
                let d = (h.theta - wet_th).abs() * ter.hab.radius + (h.z - wet_z).abs();
                d < 1.0
            })
            .map(|h| h.mass_kg)
            .sum();
        let (dry_th, dry_z) = pos(dry);
        let on_dry: f32 = heaps
            .iter()
            .filter(|h| {
                let d = (h.theta - dry_th).abs() * ter.hab.radius + (h.z - dry_z).abs();
                d < 1.0
            })
            .map(|h| h.mass_kg)
            .sum();

        // Measured: a sediment heap in a strong channel sheds ~18% over 0.6
        // habitat days, so it halves in about three. The claim under test is
        // that it moves at all and that the dry one does not.
        assert!(
            on_wet < 850.0,
            "the heap in the channel kept {on_wet:.0} of its 900 kg"
        );
        // Bare spoil erodes everywhere — it is the most erodible thing in a
        // landscape, which is why real spoil heaps get seeded — but a channel
        // has to dominate. Measured: ~8x.
        assert!(
            (900.0 - on_dry) * 4.0 < 900.0 - on_wet,
            "dry ground lost {:.0} kg against the channel's {:.0} kg; \
             the two are not meaningfully different",
            900.0 - on_dry,
            900.0 - on_wet
        );
        // Nothing vanished — the fines are downstream, not gone.
        let total: f32 = heaps.iter().map(|h| h.mass_kg).sum();
        assert!(
            (total - 1800.0).abs() < 2.0,
            "1800 kg of spoil became {total:.0} kg"
        );
        assert!(
            heaps.len() > 2,
            "the fines washed off but no downstream pile appeared"
        );
    }

    /// Timber piles are not silt. A felled crown sitting in a stream must not
    /// dissolve into it.
    #[test]
    fn timber_does_not_wash_away_as_fines() {
        let hab = Habitat::kepler_drum();
        let mut ter = Terrain::generate(hab);
        let mut bio = Biosphere::new(&ter);
        ter.flow.rebuild(&ter.elev, ter.hab.water_level);
        let wet = (0..ter.elev.len())
            .max_by(|&a, &b| ter.flow.flux[a].total_cmp(&ter.flow.flux[b]))
            .unwrap();
        let (th, z) = {
            let (ti, zi) = (wet % NT, wet / NT);
            (
                ti as f32 / NT as f32 * std::f32::consts::TAU,
                (zi as f32 / NZ as f32 - 0.5) * ter.hab.length,
            )
        };
        let mut heaps = Vec::new();
        crate::economy::deposit_heap(
            &mut heaps,
            ter.hab.radius,
            th,
            z,
            crate::economy::bio_id::WOOD,
            900.0,
            1.4,
            0.7,
        );
        for _ in 0..12 {
            bio.weather_heaps(&ter, &mut heaps, 0.05);
        }
        assert_eq!(heaps.len(), 1);
        assert!(
            (heaps[0].mass_kg - 900.0).abs() < 1.0,
            "timber washed away: {:.0} kg left",
            heaps[0].mass_kg
        );
    }

    /// Cutting into a buried ice lens in a warm province releases water, and
    /// that water is a deposit into the habitat's ledger — buried ice is
    /// stored water, not a new invention.
    #[test]
    fn exposing_ice_releases_meltwater_and_cold_ice_does_not() {
        assert_eq!(crate::economy::ice_melt_fraction(-4.0), 0.0);
        assert_eq!(crate::economy::ice_melt_fraction(0.0), 0.0);
        let warm = crate::economy::ice_melt_fraction(26.0);
        let cool = crate::economy::ice_melt_fraction(3.0);
        assert!(warm > 0.85, "26 C melted only {warm:.2} of the ice");
        assert!(
            cool < 0.3 && cool > 0.0,
            "3 C melted {cool:.2}, which is not 'most of it survives'"
        );

        let hab = Habitat::kepler_drum();
        let ter = Terrain::generate(hab);
        let mut bio = Biosphere::new(&ter);
        let before = bio.water_stock;
        bio.pond_at(&ter, 1.0, 0.0, 4.0);
        assert!(
            bio.water.depth[{
                let ti = (1.0f32 / std::f32::consts::TAU * NT as f32).round() as usize % NT;
                let zi = ((0.0f32 / ter.hab.length + 0.5) * NZ as f32).round() as usize;
                idx(ti, zi)
            }] > 0.0,
            "ponding put no water on the ground"
        );
        assert_eq!(before, bio.water_stock, "pond_at must not invent stock");
    }
}
