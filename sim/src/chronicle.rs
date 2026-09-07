//! Per-region chronicle — the world remembering. LANDSCAPE_1400 §AC / 2000 §AJ.
//!
//! Events are filtered for significance. Relationship affinity queries this
//! ledger; dialogue never invents facts that aren't here.

use std::collections::VecDeque;

pub const MAX_EVENTS: usize = 4_096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Dig,
    Harvest,
    Craft,
    Amend,
    Place,
    Flood,
    Fire,
    Help,   // agent/player helped another (Wave 2)
    Eat,
    Rest,
    Talk,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::Dig => "dig",
            EventKind::Harvest => "harvest",
            EventKind::Craft => "craft",
            EventKind::Amend => "amend",
            EventKind::Place => "place",
            EventKind::Flood => "flood",
            EventKind::Fire => "fire",
            EventKind::Help => "help",
            EventKind::Eat => "eat",
            EventKind::Rest => "rest",
            EventKind::Talk => "talk",
        }
    }
}

/// Actor id: 0 = player, 1.. = agent ids.
pub const ACTOR_PLAYER: u32 = 0;

#[derive(Clone, Debug)]
pub struct ChronicleEvent {
    pub day: f32,
    pub theta: f32,
    pub z: f32,
    pub kind: EventKind,
    pub actor: u32,
    pub target: u32, // 0 = none / world; else agent/player helped
    pub magnitude: f32,
    pub label: String,
}

#[derive(Default)]
pub struct Chronicle {
    events: VecDeque<ChronicleEvent>,
}

impl Chronicle {
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn push(&mut self, ev: ChronicleEvent) {
        // Significance filter (item 1144): skip tiny digs / trivial rests.
        if ev.kind == EventKind::Dig && ev.magnitude < 0.15 {
            return;
        }
        if ev.kind == EventKind::Rest && ev.magnitude < 0.5 {
            return;
        }
        if self.events.len() >= MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(ev);
    }

    pub fn record(
        &mut self,
        day: f32,
        theta: f32,
        z: f32,
        kind: EventKind,
        actor: u32,
        magnitude: f32,
        label: impl Into<String>,
    ) {
        self.push(ChronicleEvent {
            day,
            theta,
            z,
            kind,
            actor,
            target: 0,
            magnitude,
            label: label.into(),
        });
    }

    pub fn record_help(
        &mut self,
        day: f32,
        theta: f32,
        z: f32,
        actor: u32,
        target: u32,
        magnitude: f32,
        label: impl Into<String>,
    ) {
        self.push(ChronicleEvent {
            day,
            theta,
            z,
            kind: EventKind::Help,
            actor,
            target,
            magnitude,
            label: label.into(),
        });
    }

    pub fn latest(&self, n: usize) -> Vec<&ChronicleEvent> {
        self.events.iter().rev().take(n).collect()
    }

    /// Events near a point within `radius_m` (arc metres × axial).
    pub fn near(&self, theta: f32, z: f32, radius_m: f32, hab_r: f32) -> Vec<&ChronicleEvent> {
        let r2 = radius_m * radius_m;
        self.events
            .iter()
            .filter(|e| {
                let dth = angle_arc(e.theta, theta) * hab_r;
                let dz = e.z - z;
                dth * dth + dz * dz <= r2
            })
            .collect()
    }

    /// Affinity contribution from shared material history (item 1561).
    /// Positive help / joint work near `place`; digs that changed drainage count.
    pub fn affinity_from_history(
        &self,
        actor_a: u32,
        actor_b: u32,
        place_theta: f32,
        place_z: f32,
        hab_r: f32,
    ) -> f32 {
        let mut score = 0.0f32;
        let mut a_laboured = false;
        let mut b_laboured = false;
        for e in self.near(place_theta, place_z, 80.0, hab_r) {
            let involves = e.actor == actor_a
                || e.actor == actor_b
                || e.target == actor_a
                || e.target == actor_b;
            if !involves {
                continue;
            }
            match e.kind {
                EventKind::Help => {
                    if (e.actor == actor_a && e.target == actor_b)
                        || (e.actor == actor_b && e.target == actor_a)
                    {
                        // Cost-weighted (1564): magnitude is dig volume / amend kg.
                        score += 2.0 * e.magnitude.sqrt().max(0.25);
                    }
                }
                EventKind::Dig | EventKind::Amend | EventKind::Harvest => {
                    if e.actor == actor_a {
                        a_laboured = true;
                        score += 0.15 * e.magnitude.min(8.0);
                    } else if e.actor == actor_b {
                        b_laboured = true;
                        score += 0.15 * e.magnitude.min(8.0);
                    }
                }
                EventKind::Craft | EventKind::Eat => score += 0.05,
                _ => {}
            }
        }
        // Shared labour on the same ground (1572).
        if a_laboured && b_laboured {
            score += 1.5;
        }
        score.clamp(0.0, 100.0)
    }

    /// The single chronicle beat that most explains current affinity (1562).
    pub fn affinity_citation(
        &self,
        actor_a: u32,
        actor_b: u32,
        place_theta: f32,
        place_z: f32,
        hab_r: f32,
    ) -> Option<&ChronicleEvent> {
        let mut best: Option<&ChronicleEvent> = None;
        let mut best_w = 0.0f32;
        for e in self.near(place_theta, place_z, 100.0, hab_r) {
            let w = match e.kind {
                EventKind::Help
                    if (e.actor == actor_a && e.target == actor_b)
                        || (e.actor == actor_b && e.target == actor_a) =>
                {
                    2.0 * e.magnitude.sqrt().max(0.25)
                }
                EventKind::Dig | EventKind::Amend | EventKind::Harvest
                    if e.actor == actor_a || e.actor == actor_b =>
                {
                    0.15 * e.magnitude.min(8.0)
                }
                _ => 0.0,
            };
            if w > best_w {
                best_w = w;
                best = Some(e);
            }
        }
        best
    }

    /// One grounded line an agent can say about the player's acts (Wave 1 test).
    pub fn remark_about_player(&self, near_theta: f32, near_z: f32, hab_r: f32) -> Option<String> {
        // Prefer Help citations — the romance beat (1563).
        let mut best_help: Option<&ChronicleEvent> = None;
        let mut best_help_m = 0.0f32;
        let mut best: Option<&ChronicleEvent> = None;
        let mut best_m = 0.0f32;
        for e in self.near(near_theta, near_z, 120.0, hab_r) {
            if e.kind == EventKind::Help && e.actor == ACTOR_PLAYER && e.magnitude >= best_help_m {
                best_help_m = e.magnitude;
                best_help = Some(e);
            }
            if e.actor != ACTOR_PLAYER {
                continue;
            }
            if matches!(
                e.kind,
                EventKind::Dig | EventKind::Amend | EventKind::Harvest | EventKind::Place
            ) && e.magnitude >= best_m
            {
                best_m = e.magnitude;
                best = Some(e);
            }
        }
        if let Some(e) = best_help {
            return Some(format!(
                "You worked my plot on day {:.0} — that mattered.",
                e.day
            ));
        }
        let e = best?;
        Some(match e.kind {
            EventKind::Dig => format!(
                "You dug near here on day {:.0} — I can still see the spoil.",
                e.day
            ),
            EventKind::Amend => format!(
                "You amended this ground on day {:.0}. The soil remembers.",
                e.day
            ),
            EventKind::Harvest => format!(
                "You harvested here on day {:.0}. Good hands.",
                e.day
            ),
            EventKind::Place => format!(
                "You built something here on day {:.0}.",
                e.day
            ),
            _ => format!("{} (day {:.0})", e.label, e.day),
        })
    }
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

    #[test]
    fn affinity_rises_with_shared_help() {
        let mut c = Chronicle::default();
        c.push(ChronicleEvent {
            day: 10.0,
            theta: 0.5,
            z: 0.0,
            kind: EventKind::Help,
            actor: ACTOR_PLAYER,
            target: 1,
            magnitude: 1.0,
            label: "dug channel".into(),
        });
        let a = c.affinity_from_history(ACTOR_PLAYER, 1, 0.5, 0.0, 900.0);
        assert!(a > 1.0, "help should raise affinity, got {a}");
        let cite = c
            .affinity_citation(ACTOR_PLAYER, 1, 0.5, 0.0, 900.0)
            .expect("citation");
        assert_eq!(cite.kind, EventKind::Help);
        assert!(cite.label.contains("channel"));
    }

    #[test]
    fn shared_labour_boosts_affinity() {
        let mut c = Chronicle::default();
        c.record(5.0, 0.2, 10.0, EventKind::Dig, ACTOR_PLAYER, 2.0, "you dug");
        c.record(6.0, 0.2, 10.0, EventKind::Harvest, 1, 3.0, "ren harvested");
        let a = c.affinity_from_history(ACTOR_PLAYER, 1, 0.2, 10.0, 900.0);
        assert!(a > 1.5, "shared labour should boost, got {a}");
    }

    #[test]
    fn remark_cites_player_dig() {
        let mut c = Chronicle::default();
        c.record(12.0, 1.0, 0.0, EventKind::Dig, ACTOR_PLAYER, 2.0, "trench");
        let line = c.remark_about_player(1.0, 0.0, 900.0).unwrap();
        assert!(line.contains("dug"));
        assert!(line.contains("12"));
    }

    #[test]
    fn remark_prefers_help_on_plot() {
        let mut c = Chronicle::default();
        c.record(8.0, 0.0, 0.0, EventKind::Dig, ACTOR_PLAYER, 1.0, "scratch");
        c.record_help(9.0, 0.0, 0.0, ACTOR_PLAYER, 1, 3.0, "you dug on their plot");
        let line = c.remark_about_player(0.0, 0.0, 900.0).unwrap();
        assert!(line.contains("plot") || line.contains("mattered"));
    }
}
