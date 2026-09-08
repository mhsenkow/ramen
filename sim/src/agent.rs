//! Agent colonists — they play the same game. LANDSCAPE_2000 §AH / §AQ.
//!
//! Rule (1401): agents act only through the same world operations the player
//! uses. No agent-only affordances.

use crate::chronicle::{Chronicle, EventKind};
use crate::economy::{self, bio_id, craft_id, Inventory};
use crate::habitat::Habitat;
use crate::plant::PlantSim;
use crate::soil::Soil;
use crate::terrain::Terrain;

pub const MAX_AGENTS_T0: usize = 16;

#[derive(Clone, Copy, Debug)]
pub struct Traits {
    /// Time preference: high → orchards / long projects (item 1513).
    pub patience: f32,
    /// Risk on steep ground (item 1512).
    pub risk: f32,
    /// Maintenance / care (item 1515).
    pub care: f32,
    /// How often they seek company (item 1516).
    pub social: f32,
}

impl Default for Traits {
    fn default() -> Self {
        Self {
            patience: 0.55,
            risk: 0.4,
            care: 0.6,
            social: 0.5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goal {
    Idle,
    Harvest,
    Dig,
    Amend,
    CraftFood,
    Eat,
    Rest,
    GoHome,
    ApproachPlayer,
}

#[derive(Clone, Debug)]
pub struct Agent {
    pub id: u32,
    pub name: &'static str,
    pub theta: f32,
    pub z: f32,
    pub pack: Inventory,
    pub hunger: f32,
    pub fatigue: f32,
    pub mood: f32,
    pub traits: Traits,
    pub goal: Goal,
    pub plot_theta: f32,
    pub plot_z: f32,
    pub plot_radius: f32,
    pub rng: u32,
    pub alive: bool,
    /// Last grounded line they said (from chronicle).
    pub last_line: String,
    pub line_day: f32,
}

impl Agent {
    pub fn farmer(id: u32, name: &'static str, theta: f32, z: f32, seed: u32) -> Self {
        let mut pack = Inventory::default();
        pack.max_mass_kg = 55.0;
        pack.max_volume_m3 = 0.05;
        // Starter tools of the trade — a little seed of their own.
        pack.add_stack(bio_id::SEED, 3.0, 0.006, 0.4);
        pack.add_stack(bio_id::GREEN, 2.0, 0.005, 0.4);
        Self {
            id,
            name,
            theta,
            z,
            pack,
            hunger: 0.35,
            fatigue: 0.2,
            mood: 0.6,
            traits: Traits {
                patience: 0.7,
                risk: 0.35,
                care: 0.75,
                social: 0.55,
            },
            goal: Goal::Idle,
            plot_theta: theta,
            plot_z: z,
            plot_radius: 35.0,
            rng: seed ^ (id.wrapping_mul(0x9E37_79B9)),
            alive: true,
            last_line: String::new(),
            line_day: -99.0,
        }
    }

    fn next_f01(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 8) as f32 / (0x00FF_FFFF as f32)
    }
}

pub struct AgentSim {
    pub agents: Vec<Agent>,
}

impl AgentSim {
    pub fn new() -> Self {
        Self {
            agents: Vec::with_capacity(MAX_AGENTS_T0),
        }
    }

    pub fn seed_farmers(&mut self, hab: &Habitat, elev: &[f32], n: usize) {
        use crate::terrain::{idx, NT, NZ};
        let n = n.min(MAX_AGENTS_T0);
        // Sixteen principals — the cast you can actually know. Followers are
        // counted statistically by their dwelling, never named here.
        let names = [
            "Ren", "Jules", "Oren", "Sable", "Pax", "Idris", "Casimir", "Tobin", "Ash", "Nial",
            "Emre", "Dov", "Lark", "Hale", "Wren", "Sol",
        ];
        let mut rng = hab.seed ^ 0xA6E17;
        let mut placed = 0usize;
        let mut attempts = 0usize;
        while placed < n && attempts < n * 80 {
            attempts += 1;
            rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let ti = ((rng >> 8) % NT as u32) as usize;
            rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let zi = ((rng >> 8) % NZ as u32) as usize;
            let e = elev[idx(ti, zi)];
            if e < hab.water_level + 4.0 || e > hab.max_elevation * 0.78 {
                continue;
            }
            let theta = ti as f32 / NT as f32 * std::f32::consts::TAU;
            let z = (zi as f32 / NZ as f32 - 0.5) * hab.length;
            let name = names[placed % names.len()];
            let mut ag = Agent::farmer(placed as u32 + 1, name, theta, z, rng);
            // Trait differentiation (1512–1520): two farmers treat land differently.
            // Four ways to be, which `dwelling::kind_for` reads to decide how
            // each of them chooses to live. Social is the axis that separates
            // a town founder from a man who goes to ground.
            match placed % 4 {
                0 => {
                    // Terracer — careful, patient, keeps to himself.
                    ag.traits.care = 0.9;
                    ag.traits.risk = 0.2;
                    ag.traits.patience = 0.85;
                    ag.traits.social = 0.35;
                    ag.plot_radius = 40.0;
                }
                1 => {
                    // Strip-miner energy — risky, impatient, goes to ground.
                    ag.traits.care = 0.25;
                    ag.traits.risk = 0.9;
                    ag.traits.patience = 0.3;
                    ag.traits.social = 0.2;
                    ag.plot_radius = 55.0;
                }
                2 => {
                    // Founder — gathers people, feeds them.
                    ag.traits.care = 0.7;
                    ag.traits.risk = 0.35;
                    ag.traits.patience = 0.6;
                    ag.traits.social = 0.9;
                    ag.plot_radius = 65.0;
                }
                _ => {
                    ag.traits.care = 0.55;
                    ag.traits.risk = 0.45;
                    ag.traits.patience = 0.55;
                    ag.traits.social = 0.6;
                }
            }
            self.agents.push(ag);
            placed += 1;
        }
    }

    /// Pick utility goals, then execute through world ops (1401).
    pub fn tick(
        &mut self,
        dt_days: f32,
        day: f32,
        ter: &mut Terrain,
        plants: &mut PlantSim,
        soil: &mut Soil,
        chronicle: &mut Chronicle,
        heaps: &mut Vec<economy::Stockpile>,
        player_theta: f32,
        player_z: f32,
    ) {
        let dt = dt_days.max(0.0);
        if dt <= 0.0 {
            return;
        }
        let hab_r = ter.hab.radius;
        let light = {
            let phase = (day.fract() - 0.25).rem_euclid(1.0);
            let ang = phase * std::f32::consts::TAU;
            ((ang.sin() * 0.5 + 0.5) * 1.1).clamp(0.05, 1.0)
        };

        for ag in &mut self.agents {
            if !ag.alive {
                continue;
            }
            ag.hunger = (ag.hunger + 0.12 * dt).clamp(0.0, 1.0);
            ag.fatigue = (ag.fatigue + 0.08 * dt * (0.5 + (1.0 - light))).clamp(0.0, 1.0);
            ag.mood = (ag.mood + (light - 0.5) * 0.03 * dt).clamp(0.0, 1.0);

            ag.goal = choose_goal(ag, light, player_theta, player_z, hab_r);
            execute_goal(
                ag,
                dt,
                day,
                ter,
                plants,
                soil,
                chronicle,
                heaps,
                player_theta,
                player_z,
                light,
            );

            if day - ag.line_day > 0.8 && ag.traits.social > 0.4 {
                let dth = angle_arc(ag.theta, player_theta) * hab_r;
                let dz = ag.z - player_z;
                if dth * dth + dz * dz < 55.0 * 55.0 {
                    if let Some(line) = chronicle.remark_about_player(ag.theta, ag.z, hab_r) {
                        ag.last_line = format!("{}: {}", ag.name, line);
                        ag.line_day = day;
                        chronicle.record(
                            day,
                            ag.theta,
                            ag.z,
                            EventKind::Talk,
                            ag.id,
                            1.0,
                            ag.last_line.clone(),
                        );
                    }
                }
            }
        }
    }
}

fn choose_goal(ag: &Agent, light: f32, player_theta: f32, player_z: f32, hab_r: f32) -> Goal {
    if ag.hunger > 0.7
        && (ag.pack.mass_of(craft_id::RAMEN) > 0.5
            || ag.pack.mass_of(craft_id::RICH_RAMEN) > 0.5
            || ag.pack.mass_of(bio_id::GREEN) > 0.5
            || ag.pack.mass_of(bio_id::SEED) > 0.5)
    {
        return Goal::Eat;
    }
    let home_dth = angle_arc(ag.theta, ag.plot_theta) * hab_r;
    let home_dz = ag.z - ag.plot_z;
    let home_far = home_dth * home_dth + home_dz * home_dz > 12.0 * 12.0;
    if (ag.fatigue > 0.75 || light < 0.25) && home_far {
        return Goal::GoHome;
    }
    if ag.fatigue > 0.75 || light < 0.25 {
        return Goal::Rest;
    }
    // Social: approach player if nearby and mood ok.
    let dth = angle_arc(ag.theta, player_theta) * hab_r;
    let dz = ag.z - player_z;
    if ag.traits.social > 0.5
        && ag.mood > 0.4
        && dth * dth + dz * dz < 90.0 * 90.0
        && dth * dth + dz * dz > 12.0 * 12.0
    {
        return Goal::ApproachPlayer;
    }
    if ag.pack.mass_of(bio_id::SEED) > 4.0 && ag.pack.mass_of(craft_id::FLOUR) < 2.0 {
        return Goal::CraftFood;
    }
    if ag.traits.care > 0.5 && ag.pack.mass_of(craft_id::ASH) > 0.5 {
        return Goal::Amend;
    }
    if ag.next_f01_ref() < 0.35 {
        return Goal::Harvest;
    }
    if ag.traits.risk > 0.3 && ag.next_f01_ref() < 0.25 {
        return Goal::Dig;
    }
    Goal::Harvest
}

// Helper without mut — use a cheap hash of state instead for choose_goal branch.
impl Agent {
    fn next_f01_ref(&self) -> f32 {
        let x = self.rng.wrapping_mul(self.id.wrapping_add(1));
        (x >> 8) as f32 / (0x00FF_FFFF as f32)
    }
}

fn execute_goal(
    ag: &mut Agent,
    dt: f32,
    day: f32,
    ter: &mut Terrain,
    plants: &mut PlantSim,
    soil: &mut Soil,
    chronicle: &mut Chronicle,
    heaps: &mut Vec<economy::Stockpile>,
    player_theta: f32,
    player_z: f32,
    light: f32,
) {
    let hab = ter.hab;
    match ag.goal {
        Goal::Rest => {
            ag.fatigue = (ag.fatigue - 0.35 * dt).max(0.0);
            ag.mood = (ag.mood + 0.05 * dt).min(1.0);
            chronicle.record(day, ag.theta, ag.z, EventKind::Rest, ag.id, dt, "rest");
        }
        Goal::GoHome => {
            step_toward(
                ag,
                ag.plot_theta,
                ag.plot_z,
                hab.radius,
                16.0 * dt * (0.45 + light),
            );
            let dth = angle_arc(ag.theta, ag.plot_theta) * hab.radius;
            let dz = ag.z - ag.plot_z;
            if dth * dth + dz * dz <= 12.0 * 12.0 {
                ag.goal = Goal::Rest;
                ag.fatigue = (ag.fatigue - 0.18 * dt).max(0.0);
                ag.mood = (ag.mood + 0.03 * dt).min(1.0);
                chronicle.record(day, ag.theta, ag.z, EventKind::Rest, ag.id, dt, "home rest");
            }
        }
        Goal::Eat => {
            let mut ate = 0.0f32;
            for id in [
                craft_id::RICH_RAMEN,
                craft_id::RAMEN,
                bio_id::GREEN,
                bio_id::SEED,
            ] {
                let take = ag.pack.take_mass(id, 1.2);
                if take > 0.05 {
                    ate += take;
                    break;
                }
            }
            if ate > 0.05 {
                ag.hunger = (ag.hunger - 0.45).max(0.0);
                ag.mood = (ag.mood + 0.1).min(1.0);
                chronicle.record(day, ag.theta, ag.z, EventKind::Eat, ag.id, ate, "meal");
            }
        }
        Goal::ApproachPlayer => {
            step_toward(
                ag,
                player_theta,
                player_z,
                hab.radius,
                14.0 * dt * (0.5 + light),
            );
        }
        Goal::Harvest => {
            wander_in_plot(ag, dt, light);
            // Perception bound to plot (1491 lite): only crops they can "see" on their land.
            if let Some(i) = nearest_plant(plants, ag.theta, ag.z, ag.plot_radius, hab.radius) {
                let y = economy::harvest_plant(&plants.plants[i]);
                plants.plants[i].alive = false;
                let got = ag.pack.try_add(&y);
                ag.fatigue = (ag.fatigue + 0.04).min(1.0);
                if got > 0.1 {
                    chronicle.record(
                        day,
                        ag.theta,
                        ag.z,
                        EventKind::Harvest,
                        ag.id,
                        y.total_mass_kg,
                        format!("{} harvested", ag.name),
                    );
                }
            }
        }
        Goal::Dig => {
            wander_in_plot(ag, dt * 0.5, light);
            let surf = ter.surface_radius(ag.theta, ag.z);
            let p = hab.to_world(ag.theta, ag.z, surf - 0.4);
            if let Some(y) = ter.dig(p, 1.8, 1.0, ag.traits.care > 0.65) {
                let accepted = ag.pack.try_add(&y);
                // Leave spoil so their work is visible (1418 / 1491 neighbour read).
                let spoil_frac = (1.0 - accepted).max(0.18);
                // High-risk agents dig harder and leave more spoil (1540).
                let spoil_frac = if ag.traits.risk > 0.6 {
                    spoil_frac.max(0.35)
                } else {
                    spoil_frac
                };
                for part in &y.parts {
                    economy::deposit_heap(
                        heaps,
                        hab.radius,
                        ag.theta,
                        ag.z,
                        part.material_id,
                        part.mass_kg * spoil_frac,
                        part.loose_m3 * spoil_frac,
                        part.grade,
                    );
                }
                ag.fatigue = (ag.fatigue + 0.08).min(1.0);
                chronicle.record(
                    day,
                    ag.theta,
                    ag.z,
                    EventKind::Dig,
                    ag.id,
                    y.total_volume_m3,
                    format!("{} dug", ag.name),
                );
            }
        }
        Goal::Amend => {
            wander_in_plot(ag, dt * 0.3, light);
            let taken = ag.pack.take_mass(craft_id::ASH, 1.0);
            if taken > 0.1 {
                if let Some(effect) = economy::amendment_effect(craft_id::ASH) {
                    soil.amend(ag.theta, ag.z, effect, taken, 4.0);
                }
                chronicle.record(
                    day,
                    ag.theta,
                    ag.z,
                    EventKind::Amend,
                    ag.id,
                    taken,
                    format!("{} amended", ag.name),
                );
                ag.fatigue = (ag.fatigue + 0.03).min(1.0);
            }
        }
        Goal::CraftFood => {
            let mut atmo = economy::Atmosphere::default();
            if let Some(r) = economy::recipe_by_id("mill_flour") {
                let scale = economy::max_craft_scale(&ag.pack, &atmo, r).min(0.5);
                if scale >= 0.1 {
                    if let Ok(rep) = economy::craft(&mut ag.pack, &mut atmo, r, scale) {
                        chronicle.record(
                            day,
                            ag.theta,
                            ag.z,
                            EventKind::Craft,
                            ag.id,
                            rep.produced.total_mass_kg,
                            format!("{} milled flour", ag.name),
                        );
                    }
                }
            }
        }
        Goal::Idle => {
            wander_in_plot(ag, dt * 0.4, light);
        }
    }
}

fn wander_in_plot(ag: &mut Agent, dt: f32, light: f32) {
    let ang = ag.next_f01() * std::f32::consts::TAU;
    let step = (6.0 + ag.traits.risk * 4.0) * dt * (0.4 + 0.6 * light);
    let th2 = ag.theta + ang.cos() * step / 900.0;
    let z2 = ag.z + ang.sin() * step;
    // Soft clamp to plot.
    let dth = angle_arc(th2, ag.plot_theta) * 900.0;
    let dz = z2 - ag.plot_z;
    if dth * dth + dz * dz < ag.plot_radius * ag.plot_radius {
        ag.theta = th2.rem_euclid(std::f32::consts::TAU);
        ag.z = z2;
    } else {
        step_toward(ag, ag.plot_theta, ag.plot_z, 900.0, step);
    }
}

fn step_toward(ag: &mut Agent, th: f32, z: f32, hab_r: f32, dist: f32) {
    let dth = angle_arc(th, ag.theta);
    let dz = z - ag.z;
    let len = ((dth * hab_r).powi(2) + dz * dz).sqrt().max(1e-3);
    let k = (dist / len).min(1.0);
    ag.theta = (ag.theta + dth * k).rem_euclid(std::f32::consts::TAU);
    ag.z += dz * k;
}

fn nearest_plant(plants: &PlantSim, theta: f32, z: f32, radius: f32, hab_r: f32) -> Option<usize> {
    let r2 = radius * radius;
    let mut best = None;
    let mut best_d = f32::MAX;
    for (i, p) in plants.plants.iter().enumerate() {
        if !p.alive {
            continue;
        }
        let dth = angle_arc(p.theta, theta) * hab_r;
        let dz = p.z - z;
        let d2 = dth * dth + dz * dz;
        if d2 <= r2 && d2 < best_d {
            best_d = d2;
            best = Some(i);
        }
    }
    best
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
    fn agent_farms_through_dig_api() {
        let hab = Habitat::kepler_drum();
        let mut ter = Terrain::generate(hab);
        let mut plants = PlantSim::new();
        plants.seed_stands(
            &hab,
            &ter.elev,
            &ter.flow.flux,
            &crate::soil::Soil::new(hab, &ter.elev, &ter.flow.flux, &ter.elev0),
            200,
        );
        let mut chron = Chronicle::default();
        let mut sim = AgentSim::new();
        sim.seed_farmers(&hab, &ter.elev, 1);
        assert_eq!(sim.agents.len(), 1);
        let strokes0 = ter.edits.len();
        let mut soil = crate::soil::Soil::new(hab, &ter.elev, &ter.flow.flux, &ter.elev0);
        let mut heaps = Vec::new();
        for _ in 0..30 {
            sim.tick(
                0.2,
                1.0,
                &mut ter,
                &mut plants,
                &mut soil,
                &mut chron,
                &mut heaps,
                0.0,
                0.0,
            );
            if let Some(ag) = sim.agents.get_mut(0) {
                ag.goal = Goal::Dig;
                ag.fatigue = 0.1;
            }
        }
        assert!(
            ter.edits.len() > strokes0 || chron.len() > 0 || !heaps.is_empty(),
            "agent should dig or leave chronicle / spoil traces"
        );
    }
}
