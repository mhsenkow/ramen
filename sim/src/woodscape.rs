//! Connected wood & leaf voxels — Minecraft-style combining growth.
//!
//! Plants stamp timber/leaf cells onto a sparse world grid. Adjacent same-kind
//! cells share faces (they "combine"). Growth attaches new wood to existing
//! wood and leaf to wood tips; leaves without a wood neighbour decay.
//! Environment (moisture, aridity, shade) gates how fast a stand thickens.
//!
//! # Why this file is chunked and indexed
//!
//! The grid is a *streaming* structure: a drum with 22k plants can never hold
//! every stand at once, so cells live in 8³ chunks that load near the player
//! and unload behind. Three indexes keep every per-frame operation local:
//!
//! * `chunks` — spatial. `lod_near`, `harvest_sphere` and `prune_far` touch
//!   only the chunks their sphere overlaps, never the whole grid.
//! * `apex` — per plant. Growth needs the highest wood cell of one stand; a
//!   scan for it was O(all cells) *per plant per tick*, which at 140k cells and
//!   a 64-plant budget meant nine million map probes a tick.
//! * `leaf_checks` — dirty set. Leaf support is only re-tested where something
//!   actually changed, instead of sweeping every leaf in the world.

use crate::plant::{Plant, PlantSim};
use crate::soil::Soil;
use crate::terrain::Terrain;
use crate::weather::Weather;
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

/// Block edge length in metres — matches the tree mesh STEP vocabulary.
pub const CELL: f32 = 0.55;

/// Chunk edge in cells. 8 → 4.4 m, small enough that a 95 m LOD sphere
/// touches a few hundred chunks and large enough to keep the map shallow.
const CHUNK_BITS: i32 = 3;

pub mod kind {
    pub const EMPTY: u8 = 0;
    pub const WOOD: u8 = 1;
    pub const LEAF: u8 = 2;
}

pub type Key = (i32, i32, i32);

/// FxHash-style multiply-xor. The default SipHash is cryptographic overkill for
/// integer grid keys and dominated the profile at a few hundred thousand probes
/// per tick; this is the same trick rustc uses on its own interned maps.
#[derive(Default)]
pub struct GridHasher(u64);

impl Hasher for GridHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.write_u64(*b as u64);
        }
    }
    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.write_u64(i as u32 as u64);
    }
    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.write_u64(i as u64);
    }
    #[inline]
    fn write_u64(&mut self, i: u64) {
        const K: u64 = 0x517c_c1b7_2722_0a95;
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(K);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

type Fast = BuildHasherDefault<GridHasher>;
type FastMap<K, V> = HashMap<K, V, Fast>;
type FastSet<K> = HashSet<K, Fast>;

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub kind: u8,
    /// Height up the stand, 0–255 over `CROWN_SPAN` metres. The renderer needs
    /// it because a single instanced cube has no idea where it sits in a crown.
    pub up: u8,
    /// Owning plant index; u32::MAX = player-placed / orphan (still combines).
    pub plant: u32,
}

/// Metres of trunk that map onto the full 0–1 crown gradient.
pub const CROWN_SPAN: f32 = 22.0;

/// Highest wood cell of one stand, plus how much wood it owns. Maintained
/// incrementally so growth never scans the grid.
#[derive(Clone, Copy)]
struct Apex {
    key: Key,
    wood: u32,
}

#[derive(Default)]
pub struct Woodscape {
    chunks: FastMap<Key, FastMap<Key, Cell>>,
    count: usize,
    /// Plants stamped into the grid → the world position they were stamped at,
    /// so `prune_far` can unload a stand without consulting the plant list.
    sprouted: FastMap<u32, [f32; 3]>,
    apex: FastMap<u32, Apex>,
    /// Leaf cells to re-test for wood support (seeded where cells vanish).
    leaf_checks: Vec<Key>,
    /// Amortize growth across plants so a 20k stand doesn't hitch one frame.
    cursor: usize,
}

pub const MAX_CELLS: usize = 140_000;
/// Bound on the dirty-leaf queue; a chainsawed grove can't grow it forever.
const MAX_LEAF_CHECKS: usize = 8_192;

#[inline]
pub fn quantize(p: [f32; 3]) -> Key {
    (
        (p[0] / CELL).floor() as i32,
        (p[1] / CELL).floor() as i32,
        (p[2] / CELL).floor() as i32,
    )
}

#[inline]
pub fn dequantize(k: Key) -> [f32; 3] {
    [
        (k.0 as f32 + 0.5) * CELL,
        (k.1 as f32 + 0.5) * CELL,
        (k.2 as f32 + 0.5) * CELL,
    ]
}

#[inline]
fn chunk_of(k: Key) -> Key {
    // Arithmetic shift floors toward -inf, which is what a grid needs.
    (k.0 >> CHUNK_BITS, k.1 >> CHUNK_BITS, k.2 >> CHUNK_BITS)
}

#[inline]
fn chunk_center(c: Key) -> [f32; 3] {
    let span = (1 << CHUNK_BITS) as f32 * CELL;
    [
        (c.0 as f32 + 0.5) * span,
        (c.1 as f32 + 0.5) * span,
        (c.2 as f32 + 0.5) * span,
    ]
}

#[inline]
fn dist2(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (dx, dy, dz) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    dx * dx + dy * dy + dz * dz
}

#[inline]
fn neighbors6(k: Key) -> [Key; 6] {
    [
        (k.0 + 1, k.1, k.2),
        (k.0 - 1, k.1, k.2),
        (k.0, k.1 + 1, k.2),
        (k.0, k.1 - 1, k.2),
        (k.0, k.1, k.2 + 1),
        (k.0, k.1, k.2 - 1),
    ]
}

/// Inward radial unit vector at a world point — "up" on the drum's inner face.
#[inline]
fn drum_up(p: [f32; 3]) -> [f32; 3] {
    let len = (p[0] * p[0] + p[1] * p[1]).sqrt().max(1e-5);
    [-p[0] / len, -p[1] / len, 0.0]
}

impl Woodscape {
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.sprouted.clear();
        self.apex.clear();
        self.leaf_checks.clear();
        self.count = 0;
        self.cursor = 0;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.count >= MAX_CELLS
    }

    #[inline]
    pub fn is_sprouted(&self, plant: u32) -> bool {
        self.sprouted.contains_key(&plant)
    }

    #[inline]
    pub fn get(&self, k: Key) -> u8 {
        self.chunks
            .get(&chunk_of(k))
            .and_then(|c| c.get(&k))
            .map(|c| c.kind)
            .unwrap_or(kind::EMPTY)
    }

    #[inline]
    fn cell(&self, k: Key) -> Option<&Cell> {
        self.chunks.get(&chunk_of(k)).and_then(|c| c.get(&k))
    }

    /// Place a cell. Same-kind neighbours already "combine" — one grid, shared
    /// faces. Wood placed against foreign wood grafts (adopts its plant id).
    pub fn set(&mut self, k: Key, cell_kind: u8, plant: u32) -> bool {
        self.set_up(k, cell_kind, plant, 0)
    }

    /// As `set`, with an explicit crown-height byte for the renderer.
    pub fn set_up(&mut self, k: Key, cell_kind: u8, plant: u32, up: u8) -> bool {
        if cell_kind == kind::EMPTY {
            return self.remove(k);
        }
        let existing = self.cell(k).copied();
        if self.count >= MAX_CELLS && existing.is_none() {
            return false;
        }
        let mut owner = plant;
        if cell_kind == kind::WOOD {
            for n in neighbors6(k) {
                if let Some(c) = self.cell(n) {
                    if c.kind == kind::WOOD && c.plant != u32::MAX {
                        owner = c.plant; // graft onto the stand
                        break;
                    }
                }
            }
        }
        let changed = existing.map(|c| c.kind != cell_kind).unwrap_or(true);
        if existing.is_none() {
            self.count += 1;
        }
        self.chunks
            .entry(chunk_of(k))
            .or_default()
            .insert(k, Cell { kind: cell_kind, up, plant: owner });
        if cell_kind == kind::WOOD && existing.map(|c| c.kind) != Some(kind::WOOD) {
            if let Some(a) = self.apex.get_mut(&owner) {
                a.wood += 1;
            }
        }
        changed
    }

    /// Remove one cell, keeping counts, the chunk map and the dirty-leaf queue
    /// straight. Every deletion path goes through here.
    fn remove(&mut self, k: Key) -> bool {
        let ck = chunk_of(k);
        let Some(chunk) = self.chunks.get_mut(&ck) else {
            return false;
        };
        let Some(old) = chunk.remove(&k) else {
            return false;
        };
        if chunk.is_empty() {
            self.chunks.remove(&ck);
        }
        self.count -= 1;
        if old.kind == kind::WOOD {
            if let Some(a) = self.apex.get_mut(&old.plant) {
                a.wood = a.wood.saturating_sub(1);
            }
        }
        // Anything that leaned on this cell must re-prove it still has wood.
        if self.leaf_checks.len() < MAX_LEAF_CHECKS {
            for n in neighbors6(k) {
                if self.get(n) == kind::LEAF {
                    self.leaf_checks.push(n);
                }
            }
        }
        true
    }

    pub fn remove_plant(&mut self, plant: u32) {
        let mut doomed: Vec<Key> = Vec::new();
        for chunk in self.chunks.values() {
            for (k, c) in chunk {
                if c.plant == plant {
                    doomed.push(*k);
                }
            }
        }
        for k in doomed {
            self.remove(k);
        }
        self.sprouted.remove(&plant);
        self.apex.remove(&plant);
    }

    /// Unload chunks and sprout marks beyond `keep` metres of `center`.
    ///
    /// Without this the grid fills to MAX_CELLS after roughly a kilometre of
    /// walking and then refuses every new stand — the forest simply stops
    /// appearing ahead of you while dead geometry sits behind.
    pub fn prune_far(&mut self, center: [f32; 3], keep: f32) {
        let keep2 = keep * keep;
        let mut dropped = 0usize;
        self.chunks.retain(|c, cells| {
            if dist2(chunk_center(*c), center) <= keep2 {
                return true;
            }
            dropped += cells.len();
            false
        });
        self.count -= dropped.min(self.count);
        self.sprouted
            .retain(|_, pos| dist2(*pos, center) <= keep2);
        let live = &self.sprouted;
        self.apex.retain(|pid, _| live.contains_key(pid));
        if dropped > 0 {
            // Queued leaves may have been in the unloaded region.
            self.leaf_checks.clear();
        }
    }

    /// Stamp an initial trunk + canopy from plant pools / genome (form).
    pub fn sprout_plant(&mut self, plant_i: usize, p: &Plant, ter: &Terrain, form: u8) {
        let elev = ter.elevation(p.theta, p.z);
        let origin = ter.hab.to_world(p.theta, p.z, elev);
        let up = drum_up(origin);
        let tang = [-up[1], up[0], 0.0];
        let axial = [0.0, 0.0, 1.0];
        let pid = plant_i as u32;
        self.sprouted.insert(pid, origin);

        let wood_n = ((2.0 + p.stem * 18.0).clamp(3.0, 28.0) / CELL) as i32;
        let canopy_r = ((0.9 + p.leaf * 3.2).clamp(1.0, 5.0) / CELL) as i32;
        let lean = match form {
            6 => 0.12, // twisted / spinward
            2 => 0.06, // willow
            _ => 0.02,
        };
        // Crown byte from metres above the plant's own base, so a sapling and a
        // giant both read 0 at the root and ~1 at the tip.
        let up_of = |t: f32| ((t / CROWN_SPAN).clamp(0.0, 1.0) * 255.0) as u8;

        // Trunk — stacked wood; adjacent stacks combine into one column.
        for i in 0..wood_n {
            let t = i as f32 * CELL;
            let lean_off = lean * t;
            let pos = [
                origin[0] + up[0] * t + tang[0] * lean_off,
                origin[1] + up[1] * t + tang[1] * lean_off,
                origin[2] + up[2] * t + axial[2] * lean_off * 0.3,
            ];
            self.set_up(quantize(pos), kind::WOOD, pid, up_of(t));
            // Thick trunk for giants / mature stems.
            if p.stem > 0.45 && i < wood_n * 2 / 3 {
                for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let pos2 = [
                        pos[0] + tang[0] * CELL * dx as f32 + axial[0] * CELL * dz as f32,
                        pos[1] + tang[1] * CELL * dx as f32 + axial[1] * CELL * dz as f32,
                        pos[2] + tang[2] * CELL * dx as f32 + axial[2] * CELL * dz as f32,
                    ];
                    self.set_up(quantize(pos2), kind::WOOD, pid, up_of(t));
                }
            }
        }

        // Branches — wood steps sideways then up (decurrent / pine whorls).
        let branch_n = match form {
            0 => 4, // excurrent whorls
            1 | 7 => 5,
            2 => 5,
            3 | 4 => 0,
            5 => 3,
            _ => 4,
        };
        let crown_base = wood_n as f32 * CELL * 0.45;
        for b in 0..branch_n {
            let ang =
                b as f32 * std::f32::consts::TAU / branch_n as f32 + (p.genome_id as f32) * 0.07;
            let dir = [
                tang[0] * ang.cos() + axial[0] * ang.sin(),
                tang[1] * ang.cos() + axial[1] * ang.sin(),
                tang[2] * ang.cos() + axial[2] * ang.sin(),
            ];
            let elev_f = match form {
                0 => 0.15,
                2 => -0.45,
                _ => 0.35,
            };
            let steps = ((1.5 + p.stem * 3.0) / CELL) as i32;
            for s in 1..=steps {
                let t = s as f32 * CELL;
                let h = crown_base + t * elev_f;
                let pos = [
                    origin[0] + up[0] * h + dir[0] * t,
                    origin[1] + up[1] * h + dir[1] * t,
                    origin[2] + up[2] * h + dir[2] * t,
                ];
                self.set_up(quantize(pos), kind::WOOD, pid, up_of(h));
            }
        }

        // Canopy leaf blob around the upper trunk — fills combine into a crown.
        let cy = wood_n as f32 * CELL * 0.78;
        let leaf_center = [
            origin[0] + up[0] * cy,
            origin[1] + up[1] * cy,
            origin[2] + up[2] * cy,
        ];
        let r2 = canopy_r * canopy_r;
        let core2 = (canopy_r / 2 + 1).pow(2);
        for dx in -canopy_r..=canopy_r {
            for dy in -canopy_r / 2..=canopy_r {
                for dz in -canopy_r..=canopy_r {
                    let rad2 = dx * dx + dy * dy + dz * dz;
                    if rad2 > r2 {
                        continue;
                    }
                    let pos = [
                        leaf_center[0]
                            + tang[0] * dx as f32 * CELL
                            + up[0] * dy as f32 * CELL
                            + axial[0] * dz as f32 * CELL,
                        leaf_center[1]
                            + tang[1] * dx as f32 * CELL
                            + up[1] * dy as f32 * CELL
                            + axial[1] * dz as f32 * CELL,
                        leaf_center[2]
                            + tang[2] * dx as f32 * CELL
                            + up[2] * dy as f32 * CELL
                            + axial[2] * dz as f32 * CELL,
                    ];
                    let k = quantize(pos);
                    if self.get(k) == kind::WOOD {
                        continue;
                    }
                    // Only place leaf if a wood neighbour exists (or crown core).
                    let near_wood = rad2 <= core2
                        || neighbors6(k).iter().any(|n| self.get(*n) == kind::WOOD);
                    if near_wood {
                        self.set_up(k, kind::LEAF, pid, up_of(cy + dy as f32 * CELL));
                    }
                }
            }
        }
        self.reseat_apex(pid, origin, up);
    }

    /// Record the highest wood cell of one stand. Runs on sprout only — growth
    /// keeps the record current itself.
    fn reseat_apex(&mut self, pid: u32, origin: [f32; 3], up: [f32; 3]) {
        let mut best: Option<Key> = None;
        let mut best_h = f32::MIN;
        let mut wood = 0u32;
        // A stand lives inside a handful of chunks around its own base; walk
        // those rather than the whole grid.
        let span = (1 << CHUNK_BITS) as f32 * CELL;
        let reach = (CROWN_SPAN / span).ceil() as i32 + 1;
        let base = chunk_of(quantize(origin));
        for cx in -reach..=reach {
            for cy in -reach..=reach {
                for cz in -reach..=reach {
                    let Some(chunk) = self.chunks.get(&(base.0 + cx, base.1 + cy, base.2 + cz))
                    else {
                        continue;
                    };
                    for (k, c) in chunk {
                        if c.plant != pid || c.kind != kind::WOOD {
                            continue;
                        }
                        wood += 1;
                        let pos = dequantize(*k);
                        let h = pos[0] * up[0] + pos[1] * up[1] + pos[2] * up[2];
                        if h > best_h {
                            best_h = h;
                            best = Some(*k);
                        }
                    }
                }
            }
        }
        match best {
            Some(key) => {
                self.apex.insert(pid, Apex { key, wood });
            }
            None => {
                self.apex.remove(&pid);
            }
        }
    }

    /// Grow / decay a slice of plants. Returns cells changed.
    pub fn tick(
        &mut self,
        plants: &PlantSim,
        ter: &Terrain,
        soil: &Soil,
        weather: &Weather,
        budget: usize,
    ) -> u32 {
        let n = plants.plants.len();
        if n == 0 {
            return 0;
        }
        let mut changed = 0u32;
        let take = budget.min(n).max(1);
        for _ in 0..take {
            let i = self.cursor % n;
            self.cursor = self.cursor.wrapping_add(1);
            let pid = i as u32;
            // Only resident stands grow. An unloaded plant has no cells to
            // attach to, and sprouting it here would stamp geometry nobody can
            // see into a grid that then refuses the stand in front of you.
            if !self.sprouted.contains_key(&pid) {
                continue;
            }
            let p = &plants.plants[i];
            if !p.alive {
                continue;
            }
            changed += self.grow_one(i, p, ter, soil, weather);
        }
        changed += self.decay_leaves(budget * 2 + 16);
        changed
    }

    fn grow_one(
        &mut self,
        plant_i: usize,
        p: &Plant,
        ter: &Terrain,
        soil: &Soil,
        weather: &Weather,
    ) -> u32 {
        let s = soil.sample(p.theta, p.z);
        let arid = weather.aridity_at(p.theta, p.z);
        let moist = (s.moisture * (1.15 - arid * 0.7)).clamp(0.0, 1.0);
        if moist < 0.08 {
            return 0;
        }
        let pid = plant_i as u32;
        let elev = ter.elevation(p.theta, p.z);
        let origin = ter.hab.to_world(p.theta, p.z, elev);
        let up = drum_up(origin);

        // A stand chopped down to nothing but still alive resprouts from the
        // stump; one chopped part-way regrows from whatever wood survived.
        let mut rec = match self.apex.get(&pid).copied() {
            Some(a) if self.get(a.key) == kind::WOOD => a,
            Some(_) => {
                self.reseat_apex(pid, origin, up);
                match self.apex.get(&pid).copied() {
                    Some(a) => a,
                    None => {
                        self.sprout_plant(plant_i, p, ter, (p.genome_id % 8) as u8);
                        return 8;
                    }
                }
            }
            None => {
                self.sprout_plant(plant_i, p, ter, (p.genome_id % 8) as u8);
                return 8;
            }
        };

        let target_wood = ((2.0 + p.stem * 20.0).clamp(3.0, 32.0) / CELL) as u32;
        let mut changed = 0u32;
        let base_h = origin[0] * up[0] + origin[1] * up[1] + origin[2] * up[2];
        let up_of = |pos: [f32; 3]| {
            let h = pos[0] * up[0] + pos[1] * up[1] + pos[2] * up[2] - base_h;
            ((h / CROWN_SPAN).clamp(0.0, 1.0) * 255.0) as u8
        };

        // Extend trunk if the stem pool wants more wood — attaches to the apex.
        if rec.wood < target_wood && moist > 0.18 {
            let pos = dequantize(rec.key);
            let next = [
                pos[0] + up[0] * CELL,
                pos[1] + up[1] * CELL,
                pos[2] + up[2] * CELL,
            ];
            let nk = quantize(next);
            if self.get(nk) == kind::EMPTY {
                self.set_up(nk, kind::WOOD, pid, up_of(next));
                rec.key = nk;
                self.apex.insert(pid, rec);
                changed += 1;
            }
            // Occasional side shoot toward light (less arid → more branching).
            if moist > 0.4 && (plant_i.wrapping_mul(17) + rec.wood as usize) % 5 == 0 {
                let tang = [-up[1], up[0], 0.0];
                let side = [
                    pos[0] + tang[0] * CELL + up[0] * CELL * 0.5,
                    pos[1] + tang[1] * CELL + up[1] * CELL * 0.5,
                    pos[2] + tang[2] * CELL + up[2] * CELL * 0.5,
                ];
                let sk = quantize(side);
                if self.get(sk) == kind::EMPTY {
                    self.set_up(sk, kind::WOOD, pid, up_of(side));
                    changed += 1;
                }
            }
        }

        // Leaf flush on wood tips when the leaf pool is healthy.
        if (p.leaf * moist * 1.4).clamp(0.2, 1.5) > 0.35 {
            let pos = dequantize(rec.key);
            for (dx, dy, dz) in [
                (1, 0, 0),
                (-1, 0, 0),
                (0, 1, 0),
                (0, 0, 1),
                (0, 0, -1),
                (1, 1, 0),
                (-1, 1, 0),
            ] {
                let lp = [
                    pos[0] + dx as f32 * CELL,
                    pos[1] + dy as f32 * CELL,
                    pos[2] + dz as f32 * CELL,
                ];
                let lk = quantize(lp);
                if self.get(lk) == kind::EMPTY {
                    self.set_up(lk, kind::LEAF, pid, up_of(lp));
                    changed += 1;
                }
            }
        }
        changed
    }

    /// Retire leaves that lost their wood. Only cells queued by an actual
    /// change are examined, so a felled tree costs work proportional to the
    /// hole it left rather than to the size of the world.
    fn decay_leaves(&mut self, budget: usize) -> u32 {
        if self.leaf_checks.is_empty() {
            return 0;
        }
        let take = budget.min(self.leaf_checks.len());
        let batch: Vec<Key> = self.leaf_checks.drain(..take).collect();
        let mut n = 0u32;
        for k in batch {
            if self.get(k) != kind::LEAF {
                continue;
            }
            let supported = neighbors6(k)
                .iter()
                .any(|nb| matches!(self.get(*nb), kind::WOOD | kind::LEAF));
            // Strict Minecraft-ish: leaf needs wood within a small Manhattan range.
            if !supported || !self.wood_within(k, 3) {
                self.remove(k);
                n += 1;
            }
        }
        n
    }

    fn wood_within(&self, k: Key, manhattan: i32) -> bool {
        for dx in -manhattan..=manhattan {
            let rx = manhattan - dx.abs();
            for dy in -rx..=rx {
                let rz = rx - dy.abs();
                for dz in -rz..=rz {
                    if self.get((k.0 + dx, k.1 + dy, k.2 + dz)) == kind::WOOD {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Chunk keys whose cube can reach within `radius` of `c`, each with the
    /// distance to its centre.
    ///
    /// Two ways to find them, and which is cheaper flips with the query: a
    /// small harvest sphere spans a handful of chunk slots, while a 92 m LOD
    /// sphere spans a 45³ index cube — far more probes than the whole grid
    /// holds chunks once `prune_far` is keeping it local. Take the smaller.
    fn chunks_near(&self, c: [f32; 3], radius: f32) -> Vec<(f32, Key)> {
        let span = (1 << CHUNK_BITS) as f32 * CELL;
        // Half-diagonal of a chunk: a chunk whose centre is further than this
        // beyond the sphere cannot hold a cell inside it.
        let slack = span * 0.8661;
        let cut = (radius + slack) * (radius + slack);
        let reach = ((radius + slack) / span).ceil() as i64;
        let mut out = Vec::new();
        let span_slots = (2 * reach + 1).pow(3);
        if span_slots > self.chunks.len() as i64 {
            for key in self.chunks.keys() {
                let d2 = dist2(chunk_center(*key), c);
                if d2 <= cut {
                    out.push((d2, *key));
                }
            }
            return out;
        }
        let base = chunk_of(quantize(c));
        let reach = reach as i32;
        for cx in -reach..=reach {
            for cy in -reach..=reach {
                for cz in -reach..=reach {
                    let key = (base.0 + cx, base.1 + cy, base.2 + cz);
                    if !self.chunks.contains_key(&key) {
                        continue;
                    }
                    let d2 = dist2(chunk_center(key), c);
                    if d2 <= cut {
                        out.push((d2, key));
                    }
                }
            }
        }
        out
    }

    /// Harvest cells inside a world-space sphere → (wood_kg, leaf_kg, count).
    pub fn harvest_sphere(&mut self, c: [f32; 3], radius: f32) -> (f32, f32, u32) {
        let r2 = radius * radius;
        let mut kill: Vec<Key> = Vec::new();
        let mut wood = 0u32;
        let mut leaf = 0u32;
        for (_, ck) in self.chunks_near(c, radius) {
            let Some(chunk) = self.chunks.get(&ck) else {
                continue;
            };
            for (k, cell) in chunk {
                if dist2(dequantize(*k), c) > r2 {
                    continue;
                }
                match cell.kind {
                    kind::WOOD => wood += 1,
                    kind::LEAF => leaf += 1,
                    _ => {}
                }
                kill.push(*k);
            }
        }
        for k in &kill {
            self.remove(*k);
        }
        // ~kg per block — chunky Minecraft bites.
        const WOOD_KG: f32 = 2.8;
        const LEAF_KG: f32 = 0.45;
        (
            wood as f32 * WOOD_KG,
            leaf as f32 * LEAF_KG,
            kill.len() as u32,
        )
    }

    /// Near-player dump: flat [x, y, z, kind, up01, ...].
    ///
    /// Ordered nearest-first and truncated from the far end. Iterating the map
    /// directly instead handed the renderer a different arbitrary subset every
    /// refresh, which read on screen as blocks blinking in and out.
    pub fn lod_near(&self, center: [f32; 3], radius: f32, limit: usize) -> Vec<f32> {
        let r2 = radius * radius;
        let mut near = self.chunks_near(center, radius);
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut picked: Vec<(f32, Key, Cell)> = Vec::new();
        for (_, ck) in near {
            // Nearest-first chunk order, so once `limit` cells are in hand the
            // rest are further out. The cut lands within a chunk half-diagonal
            // (3.8 m) of a true distance sort — invisible at a 92 m radius, and
            // stable between calls, which is the property that matters.
            if picked.len() >= limit {
                break;
            }
            let Some(chunk) = self.chunks.get(&ck) else {
                continue;
            };
            for (k, cell) in chunk {
                let d2 = dist2(dequantize(*k), center);
                if d2 > r2 {
                    continue;
                }
                picked.push((d2, *k, *cell));
            }
        }
        // Stable tie-break on the key so equidistant cells keep their order
        // between refreshes; distance alone still ties inside a chunk.
        picked.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        picked.truncate(limit);
        let mut out = Vec::with_capacity(picked.len() * 5);
        for (_, k, cell) in picked {
            let p = dequantize(k);
            out.extend_from_slice(&[
                p[0],
                p[1],
                p[2],
                cell.kind as f32,
                cell.up as f32 / 255.0,
            ]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_wood_combines_under_one_owner() {
        let mut ws = Woodscape::default();
        ws.set((0, 0, 0), kind::WOOD, 1);
        ws.set((1, 0, 0), kind::WOOD, 2); // grafts onto plant 1
        assert_eq!(ws.cell((1, 0, 0)).unwrap().plant, 1);
    }

    #[test]
    fn leaf_without_wood_decays() {
        let mut ws = Woodscape::default();
        ws.set((5, 5, 5), kind::WOOD, 0);
        ws.set((5, 6, 5), kind::LEAF, 0);
        assert_eq!(ws.len(), 2);
        // Losing the wood queues the leaf above it for a support re-test.
        ws.remove((5, 5, 5));
        assert!(ws.decay_leaves(32) >= 1);
        assert!(ws.is_empty());
    }

    #[test]
    fn counts_and_chunks_stay_consistent() {
        let mut ws = Woodscape::default();
        for i in 0..40 {
            ws.set((i, i / 3, -i), kind::WOOD, 7);
        }
        assert_eq!(ws.len(), 40);
        let live: usize = ws.chunks.values().map(|c| c.len()).sum();
        assert_eq!(live, 40);
        ws.remove_plant(7);
        assert_eq!(ws.len(), 0);
        assert!(ws.chunks.is_empty(), "empty chunks must be dropped");
    }

    #[test]
    fn lod_is_nearest_first_and_stable() {
        let mut ws = Woodscape::default();
        for i in 0..60 {
            ws.set((i, 0, 0), kind::WOOD, 1);
        }
        let a = ws.lod_near(dequantize((0, 0, 0)), 40.0, 12);
        let b = ws.lod_near(dequantize((0, 0, 0)), 40.0, 12);
        assert_eq!(a, b, "same query must give the same subset");
        assert_eq!(a.len(), 12 * 5);
        // Nearest cell first.
        assert!((a[0] - dequantize((0, 0, 0))[0]).abs() < 1e-4);
    }

    #[test]
    fn prune_far_frees_cells_and_sprout_marks() {
        let mut ws = Woodscape::default();
        for i in 0..30 {
            ws.set((i * 40, 0, 0), kind::WOOD, i as u32);
            ws.sprouted.insert(i as u32, dequantize((i * 40, 0, 0)));
        }
        assert_eq!(ws.len(), 30);
        ws.prune_far(dequantize((0, 0, 0)), 60.0);
        assert!(ws.len() < 30 && ws.len() > 0);
        assert!(ws.sprouted.len() < 30);
        let live: usize = ws.chunks.values().map(|c| c.len()).sum();
        assert_eq!(live, ws.len(), "count must match what the chunks hold");
    }
}
