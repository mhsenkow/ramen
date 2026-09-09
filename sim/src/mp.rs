//! Live co-op helpers — actor ids, mutation events, guest undo log.
//!
//! Host owns the authoritative `RamaTerrain`. Guests send intents; the host
//! applies them tagged by `ActorId` and broadcasts compact deltas (strokes,
//! inventory snapshots). Agent ids stay in `1..999`; network peers use
//! `ACTOR_NET_BASE + peer_id`.

use crate::chronicle::ACTOR_PLAYER;
use crate::economy::{Inventory, Stack};
use crate::edits::Stroke;
use godot::prelude::*;
use std::collections::HashMap;

/// Schema version for join handshake / version gate.
pub const SCHEMA_VERSION: u32 = 1;

/// Network human actors start here so they never collide with agents (1..).
pub const ACTOR_NET_BASE: u32 = 1000;

pub type ActorId = u32;

#[inline]
pub fn net_actor(peer_id: u32) -> ActorId {
    if peer_id <= 1 {
        ACTOR_PLAYER
    } else {
        ACTOR_NET_BASE + peer_id
    }
}

/// Compact mutation events drained by Godot for replication.
#[derive(Clone, Debug)]
pub enum MutationEvent {
    Stroke {
        actor: ActorId,
        stroke: Stroke,
        stroke_index: u32,
    },
    Inventory {
        actor: ActorId,
        inv: Inventory,
    },
    ActorPose {
        actor: ActorId,
        theta: f32,
        z: f32,
    },
    ActorGone {
        actor: ActorId,
    },
    WoodscapeOp {
        actor: ActorId,
        x: f32,
        y: f32,
        z: f32,
        as_leaf: bool,
        place: bool,
    },
    HeapTouch,
    Station {
        kind: String,
        theta: f32,
        z: f32,
    },
    DaySnapshot {
        day: f32,
        phase: f32,
    },
}

impl MutationEvent {
    pub fn to_dict(&self) -> Dictionary {
        let mut d = Dictionary::new();
        match self {
            MutationEvent::Stroke {
                actor,
                stroke,
                stroke_index,
            } => {
                let _ = d.insert("kind", "stroke");
                let _ = d.insert("actor", *actor as i64);
                let _ = d.insert("stroke_index", *stroke_index as i64);
                let _ = d.insert("cx", stroke.c[0] as f64);
                let _ = d.insert("cy", stroke.c[1] as f64);
                let _ = d.insert("cz", stroke.c[2] as f64);
                let _ = d.insert("radius", stroke.radius as f64);
                let _ = d.insert("dig", stroke.dig);
                let _ = d.insert("level", stroke.level);
                let _ = d.insert("ux", stroke.up[0] as f64);
                let _ = d.insert("uy", stroke.up[1] as f64);
                let _ = d.insert("uz", stroke.up[2] as f64);
            }
            MutationEvent::Inventory { actor, inv } => {
                let _ = d.insert("kind", "inventory");
                let _ = d.insert("actor", *actor as i64);
                let _ = d.insert("mass_kg", inv.mass_kg() as f64);
                let _ = d.insert("loose_m3", inv.volume_m3() as f64);
                let _ = d.insert("max_mass_kg", inv.max_mass_kg as f64);
                let _ = d.insert("max_volume_m3", inv.max_volume_m3 as f64);
                let mut stacks = VariantArray::new();
                for s in &inv.stacks {
                    let mut sd = Dictionary::new();
                    let _ = sd.insert("material_id", s.material_id as i64);
                    let _ = sd.insert("name", crate::economy::bio_name(s.material_id));
                    let _ = sd.insert("mass_kg", s.mass_kg as f64);
                    let _ = sd.insert("loose_m3", s.loose_m3 as f64);
                    let _ = sd.insert("grade", s.grade as f64);
                    let _ = stacks.push(&sd.to_variant());
                }
                let _ = d.insert("stacks", stacks);
            }
            MutationEvent::ActorPose { actor, theta, z } => {
                let _ = d.insert("kind", "pose");
                let _ = d.insert("actor", *actor as i64);
                let _ = d.insert("theta", *theta as f64);
                let _ = d.insert("z", *z as f64);
            }
            MutationEvent::ActorGone { actor } => {
                let _ = d.insert("kind", "gone");
                let _ = d.insert("actor", *actor as i64);
            }
            MutationEvent::WoodscapeOp {
                actor,
                x,
                y,
                z,
                as_leaf,
                place,
            } => {
                let _ = d.insert("kind", "woodscape");
                let _ = d.insert("actor", *actor as i64);
                let _ = d.insert("x", *x as f64);
                let _ = d.insert("y", *y as f64);
                let _ = d.insert("z", *z as f64);
                let _ = d.insert("as_leaf", *as_leaf);
                let _ = d.insert("place", *place);
            }
            MutationEvent::HeapTouch => {
                let _ = d.insert("kind", "heaps");
            }
            MutationEvent::Station { kind, theta, z } => {
                let _ = d.insert("kind", "station");
                let _ = d.insert("station", kind.clone());
                let _ = d.insert("theta", *theta as f64);
                let _ = d.insert("z", *z as f64);
            }
            MutationEvent::DaySnapshot { day, phase } => {
                let _ = d.insert("kind", "day");
                let _ = d.insert("day", *day as f64);
                let _ = d.insert("phase", *phase as f64);
            }
        }
        d
    }
}

#[derive(Default)]
pub struct MpState {
    /// Extra human packs keyed by ActorId (actor 0 uses RamaTerrain.pack).
    pub packs: HashMap<ActorId, Inventory>,
    /// Poses for non-primary actors. Primary (0) uses player_theta/z.
    pub poses: HashMap<ActorId, (f32, f32)>,
    pub pending: Vec<MutationEvent>,
    /// Guest stroke undo log: (actor, stroke copy) in apply order.
    pub guest_log: Vec<(ActorId, Stroke)>,
}

impl MpState {
    pub fn ensure_pack(&mut self, actor: ActorId) {
        if actor == ACTOR_PLAYER {
            return;
        }
        self.packs.entry(actor).or_insert_with(Inventory::default);
    }

    pub fn set_pose(&mut self, actor: ActorId, theta: f32, z: f32) {
        if actor == ACTOR_PLAYER {
            return;
        }
        self.poses.insert(actor, (theta, z));
        self.pending
            .push(MutationEvent::ActorPose { actor, theta, z });
    }

    pub fn clear_actor(&mut self, actor: ActorId) {
        if actor == ACTOR_PLAYER {
            return;
        }
        self.packs.remove(&actor);
        self.poses.remove(&actor);
        self.pending.push(MutationEvent::ActorGone { actor });
    }

    pub fn push(&mut self, ev: MutationEvent) {
        self.pending.push(ev);
    }

    pub fn drain_dicts(&mut self) -> VariantArray {
        let mut out = VariantArray::new();
        for ev in self.pending.drain(..) {
            let _ = out.push(&ev.to_dict().to_variant());
        }
        out
    }

    pub fn inventory_dict(inv: &Inventory, g: f32) -> Dictionary {
        let mut d = Dictionary::new();
        let _ = d.insert("mass_kg", inv.mass_kg() as f64);
        let _ = d.insert("loose_m3", inv.volume_m3() as f64);
        let _ = d.insert("max_mass_kg", inv.max_mass_kg as f64);
        let _ = d.insert("max_volume_m3", inv.max_volume_m3 as f64);
        let _ = d.insert("mass_frac", inv.mass_frac() as f64);
        let _ = d.insert("volume_frac", inv.volume_frac() as f64);
        let _ = d.insert("encumbrance", inv.encumbrance(g) as f64);
        let mut stacks = VariantArray::new();
        for s in &inv.stacks {
            let mut sd = Dictionary::new();
            let _ = sd.insert("material_id", s.material_id as i64);
            let _ = sd.insert("name", crate::economy::bio_name(s.material_id));
            let _ = sd.insert("mass_kg", s.mass_kg as f64);
            let _ = sd.insert("loose_m3", s.loose_m3 as f64);
            let _ = sd.insert("grade", s.grade as f64);
            let _ = stacks.push(&sd.to_variant());
        }
        let _ = d.insert("stacks", stacks);
        d
    }

    pub fn apply_inventory_stacks(inv: &mut Inventory, stacks: &[Stack]) {
        inv.stacks.clear();
        for s in stacks {
            if s.mass_kg > 1e-6 {
                inv.stacks.push(s.clone());
            }
        }
    }
}
