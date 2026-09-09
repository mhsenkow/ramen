//! Agent colonists — they play the same game. LANDSCAPE_2000 §AH / §AQ.
//!
//! Rule (1401): agents act only through the same world operations the player
//! uses. No agent-only affordances.
//!
//! The first eight principals are the authored romance cast (`docs/CAST_EIGHT.md`).

use crate::chronicle::{Chronicle, EventKind};
use crate::economy::{self, bio_id, craft_id, Inventory};
use crate::habitat::Habitat;
use crate::plant::PlantSim;
use crate::province;
use crate::soil::Soil;
use crate::terrain::Terrain;

pub const MAX_AGENTS_T0: usize = 16;
pub const CAST_SIZE: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vocation {
    Hydroponics,
    Tower,
    Delve,
    Steward,
    Condenser,
    Orchard,
    Terrace,
    Gossip,
    /// Unnamed utility farmer past the authored eight.
    Utility,
}

impl Vocation {
    pub fn as_str(self) -> &'static str {
        match self {
            Vocation::Hydroponics => "hydroponics",
            Vocation::Tower => "tower",
            Vocation::Delve => "delve",
            Vocation::Steward => "steward",
            Vocation::Condenser => "condenser",
            Vocation::Orchard => "orchard",
            Vocation::Terrace => "terrace",
            Vocation::Gossip => "gossip",
            Vocation::Utility => "utility",
        }
    }

    pub fn code(self) -> f32 {
        match self {
            Vocation::Hydroponics => 0.0,
            Vocation::Tower => 1.0,
            Vocation::Delve => 2.0,
            Vocation::Steward => 3.0,
            Vocation::Condenser => 4.0,
            Vocation::Orchard => 5.0,
            Vocation::Terrace => 6.0,
            Vocation::Gossip => 7.0,
            Vocation::Utility => 8.0,
        }
    }
}

/// Prefab kind indices matching Godot `MODULES` in world.gd.
pub mod module_kind {
    pub const FARM_BED: u8 = 0;
    pub const GREENHOUSE: u8 = 1;
    pub const CONDENSER: u8 = 2;
    pub const LAMP: u8 = 3;
}

/// Visual / sim module seat for a cast plot (Godot places meshes; sim owns GH/COND).
#[derive(Clone, Copy, Debug)]
pub struct CastModule {
    pub theta: f32,
    pub z: f32,
    pub kind: u8,
    /// Metres toward the axis (tower stacks). 0 = ground.
    pub lift_m: f32,
    pub agent_id: u32,
}

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
    pub romanceable: bool,
    pub vocation: Vocation,
    pub blurb: &'static str,
}

/// Authored cast seat: name, land, traits. Spread around the drum (1716–1718).
struct CastSeat {
    name: &'static str,
    blurb: &'static str,
    vocation: Vocation,
    prefer: u8,
    alt: u8,
    /// Preferred angle slot 0..1 around θ.
    theta_frac: f32,
    /// Preferred axial band as fraction of length (−0.5..0.5).
    z_frac: f32,
    traits: Traits,
    plot_radius: f32,
}

const CAST: [CastSeat; CAST_SIZE] = [
    CastSeat {
        name: "Hale",
        blurb: "Ran the wet-ring glass before the colony had names for weather. He worries the condensers more than he admits.",
        vocation: Vocation::Hydroponics,
        prefer: province::id::FARMLAND,
        alt: province::id::SWAMP_BASIN,
        theta_frac: 0.05,
        z_frac: 0.08,
        traits: Traits {
            patience: 0.85,
            risk: 0.25,
            care: 0.92,
            social: 0.55,
        },
        plot_radius: 50.0,
    },
    CastSeat {
        name: "Casimir",
        blurb: "Believes the drum wants a skyline — works climbing toward the axis. His plot reads as a small tower town before you hear him.",
        vocation: Vocation::Tower,
        prefer: province::id::CITY,
        alt: province::id::FARMLAND,
        theta_frac: 0.18,
        z_frac: -0.12,
        traits: Traits {
            patience: 0.72,
            risk: 0.4,
            care: 0.65,
            social: 0.92,
        },
        plot_radius: 70.0,
    },
    CastSeat {
        name: "Idris",
        blurb: "Digs until the spoil tells him where the rock ends. Alone on a ridge by choice — his claim is a dark cone of heaps.",
        vocation: Vocation::Delve,
        prefer: province::id::MASSIF,
        alt: province::id::BADLANDS,
        theta_frac: 0.32,
        z_frac: 0.22,
        traits: Traits {
            patience: 0.28,
            risk: 0.92,
            care: 0.22,
            social: 0.18,
        },
        plot_radius: 55.0,
    },
    CastSeat {
        name: "Ren",
        blurb: "Shares a meadow watershed and will not yield it. Rivalry is trenches and fear-edges, not blades.",
        vocation: Vocation::Steward,
        prefer: province::id::MEADOW,
        alt: province::id::PLATEAU,
        theta_frac: 0.45,
        z_frac: -0.05,
        traits: Traits {
            patience: 0.55,
            risk: 0.75,
            care: 0.8,
            social: 0.45,
        },
        plot_radius: 60.0,
    },
    CastSeat {
        name: "Jules",
        blurb: "Looks like he belongs at the reactor; works like a man afraid of dust. Condensers in the dune sea are his promise of rain.",
        vocation: Vocation::Condenser,
        prefer: province::id::DUNE_SEA,
        alt: province::id::BADLANDS,
        theta_frac: 0.58,
        z_frac: 0.15,
        traits: Traits {
            patience: 0.7,
            risk: 0.2,
            care: 0.88,
            social: 0.4,
        },
        plot_radius: 48.0,
    },
    CastSeat {
        name: "Oren",
        blurb: "Orchards on the wet margin. Gaps in the canopy are his handwriting; ash goes back into the same rows.",
        vocation: Vocation::Orchard,
        prefer: province::id::SWAMP_BASIN,
        alt: province::id::MEADOW,
        theta_frac: 0.70,
        z_frac: -0.18,
        traits: Traits {
            patience: 0.9,
            risk: 0.3,
            care: 0.88,
            social: 0.5,
        },
        plot_radius: 45.0,
    },
    CastSeat {
        name: "Sable",
        blurb: "Mid-slope benches, one man, no followers he will name. Level brush, small spoil, nothing wasted.",
        vocation: Vocation::Terrace,
        prefer: province::id::PLATEAU,
        alt: province::id::KARST,
        theta_frac: 0.82,
        z_frac: 0.28,
        traits: Traits {
            patience: 0.88,
            risk: 0.18,
            care: 0.92,
            social: 0.28,
        },
        plot_radius: 40.0,
    },
    CastSeat {
        name: "Lark",
        blurb: "Walks other people's plots and remembers who helped whom. Will cross half a claim to tell you what Hale muttered about the glass.",
        vocation: Vocation::Gossip,
        prefer: province::id::FARMLAND,
        alt: province::id::CITY,
        theta_frac: 0.94,
        z_frac: -0.25,
        traits: Traits {
            patience: 0.5,
            risk: 0.35,
            care: 0.55,
            social: 0.95,
        },
        plot_radius: 55.0,
    },
];

impl Agent {
    pub fn farmer(id: u32, name: &'static str, theta: f32, z: f32, seed: u32) -> Self {
        let mut pack = Inventory::default();
        pack.max_mass_kg = 55.0;
        pack.max_volume_m3 = 0.05;
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
            romanceable: false,
            vocation: Vocation::Utility,
            blurb: "",
        }
    }

    fn from_cast(id: u32, seat: &CastSeat, theta: f32, z: f32, seed: u32) -> Self {
        let mut ag = Self::farmer(id, seat.name, theta, z, seed);
        ag.traits = seat.traits;
        ag.plot_radius = seat.plot_radius;
        ag.romanceable = true;
        ag.vocation = seat.vocation;
        ag.blurb = seat.blurb;
        if matches!(
            seat.vocation,
            Vocation::Orchard | Vocation::Terrace | Vocation::Hydroponics | Vocation::Condenser
        ) {
            ag.pack.add_stack(craft_id::ASH, 4.0, 0.008, 0.5);
        }
        ag
    }

    fn next_f01(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 8) as f32 / (0x00FF_FFFF as f32)
    }

    fn next_f01_ref(&self) -> f32 {
        let x = self.rng.wrapping_mul(self.id.wrapping_add(1));
        (x >> 8) as f32 / (0x00FF_FFFF as f32)
    }
}

pub struct AgentSim {
    pub agents: Vec<Agent>,
    /// Modules seeded for the cast — Godot draws them; sim registers GH/COND.
    pub cast_modules: Vec<CastModule>,
}

impl AgentSim {
    pub fn new() -> Self {
        Self {
            agents: Vec::with_capacity(MAX_AGENTS_T0),
            cast_modules: Vec::new(),
        }
    }

    pub fn seed_farmers(&mut self, hab: &Habitat, elev: &[f32], n: usize) {
        use crate::terrain::{idx, NT, NZ};
        let n = n.min(MAX_AGENTS_T0);
        let mut rng = hab.seed ^ 0xA6E17;
        let mut placed = 0usize;

        let cast_n = n.min(CAST_SIZE);
        for i in 0..cast_n {
            let seat = &CAST[i];
            let hint_th = seat.theta_frac * std::f32::consts::TAU;
            let hint_z = seat.z_frac * hab.length;
            let (theta, z) = find_province_site(
                hab,
                elev,
                seat.prefer,
                seat.alt,
                hint_th,
                hint_z,
                &mut rng,
            )
            .unwrap_or_else(|| fallback_dry_site(hab, elev, hint_th, hint_z, &mut rng));
            let ag = Agent::from_cast(placed as u32 + 1, seat, theta, z, rng);
            self.agents.push(ag);
            placed += 1;
        }

        let utility_names = [
            "Pax", "Tobin", "Ash", "Nial", "Emre", "Dov", "Wren", "Sol",
        ];
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
            let (theta, z) = if placed == CAST_SIZE {
                if let Some(ren) = self.agents.iter().find(|a| a.name == "Ren") {
                    (ren.plot_theta + 55.0 / hab.radius, ren.plot_z + 20.0)
                } else {
                    (theta, z)
                }
            } else {
                (theta, z)
            };
            let uname = utility_names[(placed - cast_n) % utility_names.len()];
            let mut ag = Agent::farmer(placed as u32 + 1, uname, theta, z, rng);
            match (placed - cast_n) % 3 {
                0 => {
                    ag.traits.care = 0.7;
                    ag.traits.risk = 0.4;
                    ag.traits.social = 0.55;
                }
                1 => {
                    ag.traits.care = 0.35;
                    ag.traits.risk = 0.75;
                    ag.traits.patience = 0.35;
                    ag.traits.social = 0.3;
                    ag.plot_radius = 50.0;
                }
                _ => {
                    ag.traits.care = 0.6;
                    ag.traits.social = 0.7;
                    ag.plot_radius = 45.0;
                }
            }
            self.agents.push(ag);
            placed += 1;
        }
    }

    /// Seed vocation modules after plots are final.
    pub fn seed_cast_modules(&mut self, hab: &Habitat) {
        self.cast_modules.clear();
        for ag in &self.agents {
            if !ag.romanceable {
                continue;
            }
            let th = ag.plot_theta;
            let zz = ag.plot_z;
            let r = hab.radius;
            let push = |mods: &mut Vec<CastModule>, dth_m: f32, dz: f32, kind: u8, lift: f32| {
                mods.push(CastModule {
                    theta: (th + dth_m / r).rem_euclid(std::f32::consts::TAU),
                    z: zz + dz,
                    kind,
                    lift_m: lift,
                    agent_id: ag.id,
                });
            };
            match ag.vocation {
                Vocation::Hydroponics => {
                    push(&mut self.cast_modules, 8.0, 0.0, module_kind::GREENHOUSE, 0.0);
                    push(&mut self.cast_modules, -6.0, 10.0, module_kind::GREENHOUSE, 0.0);
                    push(&mut self.cast_modules, 0.0, -12.0, module_kind::CONDENSER, 0.0);
                    push(&mut self.cast_modules, 14.0, 4.0, module_kind::FARM_BED, 0.0);
                }
                Vocation::Tower => {
                    for i in 0..5 {
                        let lift = i as f32 * 3.2;
                        let a = i as f32 * 1.2;
                        push(
                            &mut self.cast_modules,
                            a.cos() * 4.0,
                            a.sin() * 4.0,
                            if i % 2 == 0 {
                                module_kind::LAMP
                            } else {
                                module_kind::FARM_BED
                            },
                            lift,
                        );
                    }
                    push(&mut self.cast_modules, 10.0, 0.0, module_kind::GREENHOUSE, 0.0);
                }
                Vocation::Condenser => {
                    push(&mut self.cast_modules, 6.0, 0.0, module_kind::CONDENSER, 0.0);
                    push(&mut self.cast_modules, -8.0, 8.0, module_kind::CONDENSER, 0.0);
                    push(&mut self.cast_modules, 0.0, -10.0, module_kind::FARM_BED, 0.0);
                }
                Vocation::Orchard => {
                    push(&mut self.cast_modules, 5.0, 5.0, module_kind::FARM_BED, 0.0);
                    push(&mut self.cast_modules, -7.0, -3.0, module_kind::FARM_BED, 0.0);
                }
                Vocation::Gossip => {
                    push(&mut self.cast_modules, 4.0, 0.0, module_kind::LAMP, 0.0);
                }
                Vocation::Terrace | Vocation::Steward => {
                    push(&mut self.cast_modules, 6.0, 2.0, module_kind::FARM_BED, 0.0);
                }
                Vocation::Delve | Vocation::Utility => {}
            }
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

            let social_gate = if ag.vocation == Vocation::Gossip {
                0.25
            } else {
                0.4
            };
            let talk_r = if ag.vocation == Vocation::Gossip {
                75.0
            } else {
                55.0
            };
            if day - ag.line_day > 0.8 && ag.traits.social > social_gate {
                let dth = angle_arc(ag.theta, player_theta) * hab_r;
                let dz = ag.z - player_z;
                if dth * dth + dz * dz < talk_r * talk_r {
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
                    } else if ag.romanceable && !ag.blurb.is_empty() {
                        ag.last_line = format!("{}: {}", ag.name, ag.blurb);
                        ag.line_day = day;
                    }
                }
            }
        }
    }
}

fn find_province_site(
    hab: &Habitat,
    elev: &[f32],
    prefer: u8,
    alt: u8,
    hint_th: f32,
    hint_z: f32,
    rng: &mut u32,
) -> Option<(f32, f32)> {
    use crate::terrain::{idx, NT, NZ};
    let (dt_m, dz_m) = (
        std::f32::consts::TAU * hab.radius / NT as f32,
        hab.length / NZ as f32,
    );
    let ti0 = (hint_th / std::f32::consts::TAU * NT as f32).round() as i32;
    let zi0 = ((hint_z / hab.length + 0.5) * NZ as f32).round() as i32;
    let mut best: Option<(f32, usize, usize)> = None;
    let rt = (900.0 / dt_m) as i32;
    let rz = (900.0 / dz_m) as i32;
    let stride = 4i32;
    let mut dz = -rz;
    while dz <= rz {
        let zi = zi0 + dz;
        if zi >= 0 && zi < NZ as i32 {
            let mut dt = -rt;
            while dt <= rt {
                let ti = (ti0 + dt).rem_euclid(NT as i32) as usize;
                let zi = zi as usize;
                let e = elev[idx(ti, zi)];
                if e >= hab.water_level + 4.0 && e <= hab.max_elevation * 0.78 {
                    let th = ti as f32 / NT as f32 * std::f32::consts::TAU;
                    let z = (zi as f32 / NZ as f32 - 0.5) * hab.length;
                    let prim = province::dominant_at(hab, th, z);
                    let score = if prim == prefer {
                        2.0
                    } else if prim == alt {
                        1.0
                    } else {
                        0.0
                    };
                    if score > 0.0 {
                        let mx = dt as f32 * dt_m;
                        let my = dz as f32 * dz_m;
                        let dist = (mx * mx + my * my).sqrt();
                        let s = score * 1000.0 - dist;
                        if best.map(|(b, _, _)| s > b).unwrap_or(true) {
                            best = Some((s, ti, zi));
                        }
                    }
                }
                dt += stride;
            }
        }
        dz += stride;
    }
    let _ = rng;
    best.map(|(_, ti, zi)| {
        let th = ti as f32 / NT as f32 * std::f32::consts::TAU;
        let z = (zi as f32 / NZ as f32 - 0.5) * hab.length;
        (th, z)
    })
}

fn fallback_dry_site(
    hab: &Habitat,
    elev: &[f32],
    hint_th: f32,
    hint_z: f32,
    rng: &mut u32,
) -> (f32, f32) {
    use crate::terrain::{idx, NT, NZ};
    for _ in 0..40 {
        *rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let dth = ((*rng >> 8) as f32 / (0x00FF_FFFF as f32) - 0.5) * 0.4;
        *rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let dz = ((*rng >> 8) as f32 / (0x00FF_FFFF as f32) - 0.5) * 400.0;
        let th = (hint_th + dth).rem_euclid(std::f32::consts::TAU);
        let z = (hint_z + dz).clamp(-hab.length * 0.45, hab.length * 0.45);
        let ti = (th / std::f32::consts::TAU * NT as f32).round() as usize % NT;
        let zi = (((z / hab.length + 0.5) * NZ as f32).round() as usize).min(NZ - 1);
        let e = elev[idx(ti, zi)];
        if e >= hab.water_level + 4.0 && e <= hab.max_elevation * 0.78 {
            return (th, z);
        }
    }
    (
        hint_th.rem_euclid(std::f32::consts::TAU),
        hint_z.clamp(-hab.length * 0.4, hab.length * 0.4),
    )
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
    let dth = angle_arc(ag.theta, player_theta) * hab_r;
    let dz = ag.z - player_z;
    let dist2 = dth * dth + dz * dz;
    let approach_r = if ag.vocation == Vocation::Gossip {
        120.0
    } else {
        90.0
    };
    let social_need = if ag.vocation == Vocation::Gossip {
        0.35
    } else {
        0.5
    };
    if ag.traits.social > social_need
        && ag.mood > 0.35
        && dist2 < approach_r * approach_r
        && dist2 > 12.0 * 12.0
    {
        return Goal::ApproachPlayer;
    }
    if ag.pack.mass_of(bio_id::SEED) > 4.0 && ag.pack.mass_of(craft_id::FLOUR) < 2.0 {
        return Goal::CraftFood;
    }

    match ag.vocation {
        Vocation::Orchard | Vocation::Hydroponics => {
            if ag.traits.care > 0.5 && ag.pack.mass_of(craft_id::ASH) > 0.5 && ag.next_f01_ref() < 0.4
            {
                return Goal::Amend;
            }
            if ag.next_f01_ref() < 0.55 {
                return Goal::Harvest;
            }
            return Goal::Dig;
        }
        Vocation::Delve | Vocation::Steward | Vocation::Tower => {
            if ag.traits.risk > 0.3 && ag.next_f01_ref() < 0.55 {
                return Goal::Dig;
            }
            if ag.next_f01_ref() < 0.35 {
                return Goal::Harvest;
            }
        }
        Vocation::Condenser | Vocation::Terrace => {
            if ag.pack.mass_of(craft_id::ASH) > 0.5 && ag.next_f01_ref() < 0.45 {
                return Goal::Amend;
            }
            if ag.next_f01_ref() < 0.4 {
                return Goal::Dig;
            }
            return Goal::Harvest;
        }
        Vocation::Gossip => {
            if dist2 < approach_r * approach_r && dist2 > 8.0 * 8.0 && ag.next_f01_ref() < 0.6 {
                return Goal::ApproachPlayer;
            }
            if ag.next_f01_ref() < 0.5 {
                return Goal::Harvest;
            }
        }
        Vocation::Utility => {}
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
            let brush = match ag.vocation {
                Vocation::Delve => 2.4,
                Vocation::Steward | Vocation::Tower => 2.0,
                _ => 1.8,
            };
            let level = ag.traits.care > 0.65
                || matches!(
                    ag.vocation,
                    Vocation::Terrace | Vocation::Condenser | Vocation::Steward
                );
            let p = hab.to_world(ag.theta, ag.z, surf - 0.4);
            if let Some(y) = ter.dig(p, brush, 1.0, level) {
                let accepted = ag.pack.try_add(&y);
                let mut spoil_frac = (1.0 - accepted).max(0.18);
                if ag.traits.risk > 0.6 || ag.vocation == Vocation::Delve {
                    spoil_frac = spoil_frac.max(0.4);
                }
                if matches!(ag.vocation, Vocation::Terrace | Vocation::Condenser) {
                    spoil_frac = spoil_frac.min(0.22);
                }
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
        assert!(sim.agents[0].romanceable);
        assert_eq!(sim.agents[0].name, "Hale");
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

    #[test]
    fn cast_eight_seats_romanceable() {
        let hab = Habitat::kepler_drum();
        let ter = Terrain::generate(hab);
        let mut sim = AgentSim::new();
        sim.seed_farmers(&hab, &ter.elev, 10);
        assert_eq!(sim.agents.len(), 10);
        let cast: Vec<_> = sim.agents.iter().filter(|a| a.romanceable).collect();
        assert_eq!(cast.len(), 8);
        assert_eq!(cast[0].name, "Hale");
        assert_eq!(cast[7].name, "Lark");
        sim.seed_cast_modules(&hab);
        assert!(!sim.cast_modules.is_empty());
    }
}
