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
use crate::tree_form;
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
    /// Stands that lost wood in the most recent harvest.
    ///
    /// Wood is what holds a stand up, so these are the only ones that can
    /// have been cut through. Reported as a side channel rather than in the
    /// return value because a bite can span several stands and every caller
    /// but one wants only the mass.
    cut: Vec<u32>,
    /// Metres from a stand's origin to its furthest cell.
    ///
    /// `CROWN_SPAN` was standing in for this, and it is 22 m while a
    /// species-7 canopy tree is 42 m tall — so a whole-plant walk bounded by
    /// it silently stopped short of the top of the biggest trees. Measured at
    /// sprout, when every cell's position is already in hand.
    extent: FastMap<u32, f32>,
    apex: FastMap<u32, Apex>,
    /// Leaf cells to re-test for wood support (seeded where cells vanish).
    leaf_checks: Vec<Key>,
    /// Amortize growth across plants so a 20k stand doesn't hitch one frame.
    cursor: usize,
    pub revision: u64,
    edited: FastSet<u32>,
    tints: FastMap<u32, [f32; 3]>,
    /// Kilograms of woody/leaf material already taken from each organism, by
    /// any path. This is the shared ledger the forest plan asks for: block
    /// chopping and whole-plant felling both read and write it, so felling a
    /// half-chopped tree cannot pay out the intact tree's biomass again.
    ///
    /// Durable for the same reason edited cells are: a touched stand is never
    /// pruned, and `encode_edits` carries the ledger alongside its cells.
    taken: FastMap<u32, f32>,
    rejected_at_count: FastMap<u32, usize>,
}

/// Kilograms per block. One place, because the dig path bills blocks and the
/// fell path bills what is left of the organism — they have to agree on a rate.
/// Material that came down because nothing was holding it up any more.
#[derive(Clone, Copy, Debug)]
pub struct Fall {
    pub wood_kg: f32,
    pub leaf_kg: f32,
    pub blocks: u32,
    /// Foot of the tree it fell from — where the pile ends up.
    pub at: [f32; 3],
}

/// What came of asking for a stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sprout {
    /// The stand is resident (or already was).
    Made,
    /// Refused, wanting this many cells. Free at least that much and retry.
    NoRoom(usize),
}

pub const WOOD_KG: f32 = 2.8;
pub const LEAF_KG: f32 = 0.45;

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

/// The 26 cells touching this one, corners included.
///
/// Face contact alone is the physically honest reading of "attached", and it
/// is the wrong one here: `tree_form`'s recipe stamps boxes that meet a trunk
/// at an angle, so 23% of an untouched canopy tree reaches the stump only
/// through a corner. Under face adjacency a forest nobody had touched would
/// shed a fifth of itself on the first tick. Corner contact is what this
/// geometry means by joined.
fn neighbors26(k: Key) -> Vec<Key> {
    let mut out = Vec::with_capacity(26);
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                if (dx, dy, dz) != (0, 0, 0) {
                    out.push((k.0 + dx, k.1 + dy, k.2 + dz));
                }
            }
        }
    }
    out
}

/// Inward radial unit vector at a world point — "up" on the drum's inner face.
#[inline]
fn drum_up(p: [f32; 3]) -> [f32; 3] {
    let len = (p[0] * p[0] + p[1] * p[1]).sqrt().max(1e-5);
    [-p[0] / len, -p[1] / len, 0.0]
}

impl Woodscape {
    pub fn clear(&mut self) {
        self.taken.clear();
        self.chunks.clear();
        self.sprouted.clear();
        self.extent.clear();
        self.cut.clear();
        self.apex.clear();
        self.leaf_checks.clear();
        self.count = 0;
        self.cursor = 0;
        self.edited.clear();
        self.tints.clear();
        self.rejected_at_count.clear();
        self.revision += 1;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Is the very last cell spoken for?
    ///
    /// Right for placing one block, wrong for realising a stand — a mature
    /// tree is ~6,700 cells, so with 5,271 free the grid is not `is_full` and
    /// yet no tree fits. Ask `sprout_plant`, which stages the whole tree and
    /// reports what it actually needed.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.count >= MAX_CELLS
    }

    /// Worth trying to realise? False only for a stand that was refused, and
    /// could not be made room for, at a cell count the grid is still at.
    ///
    /// Note what this must *not* be: a memo written when the sprout itself was
    /// refused. That deadlocked — the plant stopped being a candidate, so
    /// nothing was ever evicted on its behalf, so the count never moved, so it
    /// was never a candidate again. Only `note_refusal`, which runs after
    /// eviction has also failed, may close the door; and any successful
    /// eviction reopens it for everyone.
    pub fn can_sprout(&self, plant: u32) -> bool {
        !self.is_sprouted(plant)
            && self
                .rejected_at_count
                .get(&plant)
                .map(|n| self.count != *n)
                .unwrap_or(true)
    }

    /// This stand did not fit and nothing could be dropped for it. Stop
    /// re-staging it — 6,700 cells of scratch work per frame — until the grid
    /// changes size, which digging, walking and regrowth all do.
    pub fn note_refusal(&mut self, plant: u32) {
        self.rejected_at_count.insert(plant, self.count);
    }

    /// How many organisms currently have cells in the grid.
    pub fn stand_count(&self) -> usize {
        self.sprouted.len()
    }

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
        let changed = self.set_up(k, cell_kind, plant, 0);
        if changed {
            if let Some(c) = self.cell(k) {
                self.edited.insert(c.plant);
            }
        }
        changed
    }

    /// As `set`, with an explicit crown-height byte for the renderer.
    pub fn set_up(&mut self, k: Key, cell_kind: u8, plant: u32, up: u8) -> bool {
        if cell_kind == kind::EMPTY {
            return self.remove(k);
        }
        let existing = self.cell(k).copied();
        if existing.map(|c| c.kind == cell_kind).unwrap_or(false) {
            return false;
        }
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
        let changed = true;
        self.revision += 1;
        if existing.is_none() {
            self.count += 1;
        }
        self.chunks.entry(chunk_of(k)).or_default().insert(
            k,
            Cell {
                kind: cell_kind,
                up,
                plant: owner,
            },
        );
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
        self.revision += 1;
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
        // Evict whole stands, not individual chunks: crowns cross chunk borders.
        // A touched stand or player construction remains authoritative at any range.
        let keep2 = keep * keep;
        let doomed: Vec<u32> = self
            .sprouted
            .iter()
            .filter(|(id, p)| !self.edited.contains(id) && dist2(**p, center) > keep2)
            .map(|(id, _)| *id)
            .collect();
        for id in doomed {
            self.unload_plant(id);
        }
    }

    fn unload_plant(&mut self, id: u32) {
        let mut dropped = 0;
        self.chunks.retain(|_, cells| {
            cells.retain(|_, c| {
                if c.plant == id {
                    dropped += 1;
                    false
                } else {
                    true
                }
            });
            !cells.is_empty()
        });
        self.count -= dropped;
        self.sprouted.remove(&id);
        self.extent.remove(&id);
        self.apex.remove(&id);
        self.tints.remove(&id);
        if dropped > 0 {
            self.revision += 1;
        }
    }

    pub fn is_edited(&self, id: u32) -> bool {
        self.edited.contains(&id)
    }

    /// Woody/leaf kilograms already removed from this organism.
    pub fn taken_kg(&self, id: u32) -> f32 {
        self.taken.get(&id).copied().unwrap_or(0.0)
    }

    /// Distance from `center` to the farthest resident stand that may be
    /// evicted, with its id. Edited stands are never candidates: their material
    /// is authoritative and cannot be regenerated.
    pub fn farthest_evictable(&self, center: [f32; 3]) -> Option<(f32, u32)> {
        self.sprouted
            .iter()
            .filter(|(id, _)| !self.edited.contains(id))
            .map(|(id, pos)| (dist2(*pos, center), *id))
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(d2, id)| (d2.sqrt(), id))
    }

    /// Free room for a stand `want_dist` metres away by unloading stands that
    /// are farther off. Returns how many were unloaded.
    ///
    /// The grid used to simply refuse once it was full, and a mature tree costs
    /// something like 6,700 cells — so twenty of them filled a 140,000-cell
    /// budget and every tree after that was drawn as a proxy with no material
    /// behind it. Walking up to one and swinging did nothing: the ray found no
    /// cells, the dig fell through to mid-air terrain, and you heard a swing
    /// for a tree that could never be cut. Distant unedited stands are
    /// reconstructible from the seed, so they are what should give way.
    pub fn make_room_for(&mut self, center: [f32; 3], want_dist: f32, need: usize) -> u32 {
        let mut freed = 0u32;
        while self.count + need > MAX_CELLS {
            let Some((d, id)) = self.farthest_evictable(center) else {
                break; // everything left is edited; nothing may be dropped
            };
            // Never evict something closer than what we are making room for,
            // or a full grid would thrash between two neighbouring trees.
            if d <= want_dist * 1.25 {
                break;
            }
            self.unload_plant(id);
            self.rejected_at_count.clear();
            freed += 1;
        }
        freed
    }

    /// Load and unload stands around a viewpoint. Named, because the mesh path
    /// used to trigger it by calling the LOD query and discarding the result —
    /// which reads as dead code and invites someone to delete it.
    pub fn stream_around(&mut self, center: [f32; 3], radius: f32) {
        self.prune_far(center, radius * 2.2);
    }

    /// New material grown back pays down the removal ledger.
    ///
    /// Without this, regrowth would be geometry the fell path could not see:
    /// a coppiced tree would look whole and pay nothing. With it, the ledger
    /// means "removed, net of regrown", so felling always pays for exactly the
    /// material that is standing there — and the plan's rule holds, because
    /// growth is bounded by the organism's own stem pool rather than being a
    /// refund for cutting.
    pub fn credit_growth(&mut self, id: u32, kg: f32) {
        if id == u32::MAX || kg <= 0.0 {
            return;
        }
        if let Some(t) = self.taken.get_mut(&id) {
            *t = (*t - kg).max(0.0);
        }
    }

    /// Record a removal this grid did not perform itself — whole-plant felling
    /// pays out the remainder, and that remainder must not be payable twice.
    pub fn add_taken(&mut self, id: u32, kg: f32) {
        if id == u32::MAX || kg <= 0.0 {
            return;
        }
        *self.taken.entry(id).or_insert(0.0) += kg;
        self.edited.insert(id);
    }

    /// Rasterize the SAME material volumes used to build distant stand meshes.
    /// Nothing is committed until the complete stand fits, so a full cache can
    /// retain its proxy instead of revealing half a trunk or a missing crown.
    /// Stamp one organism's cells into the grid.
    ///
    /// Staging is all-or-nothing: a stand with a trunk and no crown, or a
    /// crown clipped where the budget ran out, is worse than no stand at all.
    /// So the whole tree is built in a scratch map, and only then does the
    /// grid get asked whether it will fit. When it will not, the answer says
    /// how many cells were wanted — the caller needs that number to evict
    /// exactly enough and try again, and no fixed guess at "the size of a
    /// tree" survives contact with eight species across four size bands.
    pub fn sprout_plant(&mut self, plant_i: usize, p: &Plant, ter: &Terrain, form: u8) -> Sprout {
        let pid = plant_i as u32;
        if self.sprouted.contains_key(&pid) {
            return Sprout::Made;
        }
        let origin = ter
            .hab
            .to_world(p.theta, p.z, ter.hab.radius - ter.elevation(p.theta, p.z));
        let up = drum_up(origin);
        let tang = [up[1], -up[0], 0.0];
        let a = tree_form::yaw(p.theta, p.z);
        let (sn, cs) = a.sin_cos();
        let dims = tree_form::dimensions(p, form);
        let x = [tang[0] * cs, tang[1] * cs, -sn];
        let z = [tang[0] * sn, tang[1] * sn, cs];
        let mut staged: FastMap<Key, (u8, u8)> = FastMap::default();
        for part in tree_form::recipe(form) {
            let steps: [usize; 3] = std::array::from_fn(|i| {
                ((part.size[i] * dims[i] / (CELL * 0.65)).ceil() as usize).max(1)
            });
            for ix in 0..steps[0] {
                for iy in 0..steps[1] {
                    for iz in 0..steps[2] {
                        let v: [f32; 3] = std::array::from_fn(|i| {
                            let n = [ix, iy, iz][i];
                            (part.center[i]
                                + ((n as f32 + 0.5) / steps[i] as f32 - 0.5) * part.size[i])
                                * dims[i]
                        });
                        let pos = std::array::from_fn(|i| {
                            origin[i] + x[i] * v[0] + up[i] * v[1] + z[i] * v[2]
                        });
                        let k = quantize(pos);
                        let height = ((v[1] / CROWN_SPAN).clamp(0.0, 1.0) * 255.0) as u8;
                        staged
                            .entry(k)
                            .and_modify(|c| {
                                if part.kind == kind::WOOD {
                                    *c = (part.kind, height);
                                }
                            })
                            .or_insert((part.kind, height));
                    }
                }
            }
        }
        if staged.len() + self.count > MAX_CELLS {
            return Sprout::NoRoom(staged.len());
        }
        self.sprouted.insert(pid, origin);
        self.tints.insert(pid, tree_form::pigment(form));
        let mut far = 0.0f32;
        for k in staged.keys() {
            far = far.max(dist2(dequantize(*k), origin));
        }
        self.extent.insert(pid, far.sqrt() + CELL);
        for (k, (material, height)) in staged {
            if self.get(k) == kind::WOOD {
                continue;
            }
            self.set_up(k, material, pid, height);
        }
        self.reseat_apex(pid, origin, up);
        Sprout::Made
    }

    /// Record the highest wood cell of one stand. Runs on sprout only — growth
    /// keeps the record current itself.
    /// Stands that lost wood in the last harvest, and so might have been cut
    /// through. Cleared and rewritten by each harvest call.
    pub fn cut_plants(&self) -> Vec<u32> {
        self.cut.clone()
    }

    /// Cut wood, and the foliage it was carrying, that no longer reaches the
    /// ground.
    ///
    /// Leaves already decay when their branch goes: `decay_leaves` retires any
    /// leaf left without wood within a short Manhattan reach, cascading from
    /// the cells a removal queued. Wood had no such rule at all, so cutting
    /// through a trunk left the whole crown's timber hanging in mid-air, which
    /// is the least convincing thing a block forest can do. A wood cell is
    /// held up if it reaches the stump through touching wood; anything else
    /// comes down.
    ///
    /// Wood only. The leaves it was carrying are left to `decay_leaves`, which
    /// already does that job with a per-tick budget — claiming them here would
    /// mean testing tens of thousands of crown cells inside a single swing.
    ///
    /// Adjacency includes corners — see `neighbors26` for why face contact
    /// alone declares a healthy tree severed.
    ///
    /// Returns the mass that fell and where it landed, so the caller can pile
    /// it at the foot of the tree. Falling material is *not* removed from the
    /// world silently — timber you cut down has to still be there.
    pub fn collapse_severed(&mut self, pid: u32, up: [f32; 3]) -> Option<Fall> {
        let origin = *self.sprouted.get(&pid)?;
        let base_h = origin[0] * up[0] + origin[1] * up[1] + origin[2] * up[2];

        // Every cell this stand owns, and which of them the ground holds.
        let mut own: FastMap<Key, u8> = FastMap::default();
        let mut queue: Vec<Key> = Vec::new();
        for ck in self.plant_chunks(pid, origin) {
            let Some(chunk) = self.chunks.get(&ck) else {
                continue;
            };
            for (k, c) in chunk {
                if c.plant != pid {
                    continue;
                }
                own.insert(*k, c.kind);
                if c.kind == kind::WOOD {
                    let p = dequantize(*k);
                    let h = p[0] * up[0] + p[1] * up[1] + p[2] * up[2];
                    if h - base_h < CELL * 2.0 {
                        queue.push(*k);
                    }
                }
            }
        }
        if own.is_empty() {
            return None;
        }

        // Reachable from the stump through wood.
        let mut held: FastMap<Key, ()> = FastMap::default();
        for k in &queue {
            held.insert(*k, ());
        }
        while let Some(k) = queue.pop() {
            for n in neighbors26(k) {
                if own.get(&n) != Some(&kind::WOOD) || held.contains_key(&n) {
                    continue;
                }
                held.insert(n, ());
                queue.push(n);
            }
        }

        let fall: Vec<Key> = own
            .iter()
            .filter(|(k, kd)| **kd == kind::WOOD && !held.contains_key(*k))
            .map(|(k, _)| *k)
            .collect();
        if fall.is_empty() {
            return None;
        }

        for k in &fall {
            *self.taken.entry(pid).or_insert(0.0) += WOOD_KG;
            // `remove` queues the neighbouring leaves, so the crown crumbles
            // over the next few ticks instead of all at once.
            self.remove(*k);
        }
        self.edited.insert(pid);
        Some(Fall {
            wood_kg: fall.len() as f32 * WOOD_KG,
            leaf_kg: 0.0,
            blocks: fall.len() as u32,
            at: origin,
        })
    }

    /// Chunk keys that can hold any cell of one stand.
    fn plant_chunks(&self, pid: u32, origin: [f32; 3]) -> Vec<Key> {
        let span = (1 << CHUNK_BITS) as f32 * CELL;
        let reach = (self.extent.get(&pid).copied().unwrap_or(CROWN_SPAN) / span).ceil() as i32 + 1;
        let base = chunk_of(quantize(origin));
        let mut out = Vec::new();
        for cx in -reach..=reach {
            for cy in -reach..=reach {
                for cz in -reach..=reach {
                    let key = (base.0 + cx, base.1 + cy, base.2 + cz);
                    if self.chunks.contains_key(&key) {
                        out.push(key);
                    }
                }
            }
        }
        out
    }

    fn reseat_apex(&mut self, pid: u32, origin: [f32; 3], up: [f32; 3]) {
        let mut best: Option<Key> = None;
        let mut best_h = f32::MIN;
        let mut wood = 0u32;
        for ck in self.plant_chunks(pid, origin) {
            let Some(chunk) = self.chunks.get(&ck) else {
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
            //
            // A *cut* stand does grow. Excluding edited stands froze every
            // organism the player had touched: chop one branch and that tree
            // was biologically dead forever, while its neighbours went on
            // growing. The cut itself stays a permanent historical edit — new
            // growth adds new cells and never rewrites the edit mask — and it
            // is paid for, because growth credits the removal ledger (see
            // `grow_one`), so regrown wood is not free mass.
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
        let origin = ter.hab.to_world(p.theta, p.z, ter.hab.radius - elev);
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
                        let _ = self.sprout_plant(plant_i, p, ter, (p.genome_id % 8) as u8);
                        return 8;
                    }
                }
            }
            None => {
                let _ = self.sprout_plant(plant_i, p, ter, (p.genome_id % 8) as u8);
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
                self.credit_growth(pid, WOOD_KG);
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
                    self.credit_growth(pid, WOOD_KG);
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
                    self.credit_growth(pid, LEAF_KG);
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
            if !supported || !self.wood_within(k, 5) {
                // Bill it. A decayed leaf left the organism as surely as a cut
                // one did, and going unbilled meant `H` would still pay out
                // for foliage that had already crumbled off a cut tree.
                if let Some(cell) = self.cell(k) {
                    let pid = cell.plant;
                    if pid != u32::MAX {
                        *self.taken.entry(pid).or_insert(0.0) += LEAF_KG;
                    }
                }
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

    /// The nearest material cell to `c` within `r` metres, if any.
    ///
    /// A raycast hit lands exactly on a block face, so quantizing it usually
    /// names the empty cell on the outside of that face. Detecting "did the ray
    /// hit wood" by probing one axis therefore misses every face whose normal
    /// is not on that axis — which is four faces out of six. Asking for the
    /// nearest cell instead is orientation-independent.
    pub fn nearest_cell(&self, c: [f32; 3], r: f32) -> Option<(Key, Cell)> {
        let r2 = r * r;
        let mut best: Option<(f32, Key, Cell)> = None;
        for (_, ck) in self.chunks_near(c, r) {
            let Some(chunk) = self.chunks.get(&ck) else {
                continue;
            };
            for (k, cell) in chunk {
                let d2 = dist2(dequantize(*k), c);
                if d2 > r2 {
                    continue;
                }
                if best.map_or(true, |(b, _, _)| d2 < b) {
                    best = Some((d2, *k, *cell));
                }
            }
        }
        best.map(|(_, k, cell)| (k, cell))
    }

    /// Is there material within `r` of `c`?
    pub fn material_within(&self, c: [f32; 3], r: f32) -> bool {
        self.nearest_cell(c, r).is_some()
    }

    /// Take exactly one block — the nearest to `c` within `r`.
    ///
    /// One block per swing is the whole point of a block world: the brush is
    /// for earthworks, and a 1.4 m sphere through a trunk removes an armful of
    /// tree per click. Returns `(material kind, kilograms)`.
    pub fn harvest_one(&mut self, c: [f32; 3], r: f32) -> Option<(u8, f32)> {
        self.cut.clear();
        let (k, cell) = self.nearest_cell(c, r)?;
        let kg = if cell.kind == kind::WOOD {
            WOOD_KG
        } else {
            LEAF_KG
        };
        self.edited.insert(cell.plant);
        if cell.plant != u32::MAX {
            *self.taken.entry(cell.plant).or_insert(0.0) += kg;
            if cell.kind == kind::WOOD {
                self.cut.push(cell.plant);
            }
        }
        self.remove(k);
        Some((cell.kind, kg))
    }

    /// Harvest cells inside a world-space sphere → (wood_kg, leaf_kg, count).
    /// How much of a brush radius actually bites, by material.
    ///
    /// The brush is an axe head, and the swing has to read as one: slashing
    /// foliage clears an armful, while the same swing into a trunk takes a
    /// notch out of it. Terrain already does this through `material::dig_scale`
    /// — hard rock shrinks the effective radius — and wood is just another
    /// material with an opinion about being cut.
    ///
    /// Scaled radius, not scaled count, because the shape of what comes away
    /// is what you see. At the smallest brush a wood bite lands under one cell
    /// across, which keeps single-block precision available to anyone who
    /// wants it.
    pub fn bite_scale(kind: u8) -> f32 {
        match kind {
            kind::WOOD => 0.42,
            _ => 0.95,
        }
    }

    pub fn harvest_sphere(&mut self, c: [f32; 3], radius: f32) -> (f32, f32, u32) {
        self.cut.clear();
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
            let Some((pid, kd)) = self.cell(*k).map(|c| (c.plant, c.kind)) else {
                self.remove(*k);
                continue;
            };
            let kg = if kd == kind::WOOD { WOOD_KG } else { LEAF_KG };
            self.edited.insert(pid);
            // Player-placed material (u32::MAX) is construction, not an
            // organism, so it never enters an organism's ledger.
            if pid != u32::MAX {
                *self.taken.entry(pid).or_insert(0.0) += kg;
                if kd == kind::WOOD && !self.cut.contains(&pid) {
                    self.cut.push(pid);
                }
            }
            self.remove(*k);
        }
        (
            wood as f32 * WOOD_KG,
            leaf as f32 * LEAF_KG,
            kill.len() as u32,
        )
    }

    /// Exposed faces only; adjacent materials share a face instead of drawing
    /// 12 triangles per interior block. UV carries crown height and edited state.
    /// Mesh everything. Kept for tests and for callers with no viewpoint.
    pub fn surface(
        &self,
    ) -> (
        Vec<[f32; 3]>,
        Vec<[f32; 3]>,
        Vec<[f32; 4]>,
        Vec<[f32; 2]>,
        Vec<i32>,
    ) {
        // Infinity, not MAX: MAX is finite, so it took the bounded path and
        // overflowed the chunk-index reach it computes from the radius.
        self.surface_near([0.0; 3], f32::INFINITY)
    }

    /// Mesh only the chunks within `radius` of `center`.
    ///
    /// The unbounded version walked every resident chunk, and `prune_far`
    /// deliberately never evicts a touched stand — so every edited grove the
    /// player had ever visited was re-meshed into the near view on every
    /// revision bump, however many kilometres away it was. The caller already
    /// had a radius and was throwing it away.
    ///
    /// This is not yet the dirty-chunk rebuild the plan's stage 4 asks for: a
    /// single cut still re-meshes the neighbourhood rather than the one chunk
    /// that changed. It bounds the work to what is actually drawn.
    pub fn surface_near(
        &self,
        center: [f32; 3],
        radius: f32,
    ) -> (
        Vec<[f32; 3]>,
        Vec<[f32; 3]>,
        Vec<[f32; 4]>,
        Vec<[f32; 2]>,
        Vec<i32>,
    ) {
        let mut verts = Vec::new();
        let mut normals = Vec::new();
        let mut colors = Vec::new();
        let mut uv = Vec::new();
        let mut indices = Vec::new();
        let dirs = [
            (1, 0, 0),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
            (0, 0, 1),
            (0, 0, -1),
        ];
        let corners = [
            [[1, -1, -1], [1, -1, 1], [1, 1, 1], [1, 1, -1]],
            [[-1, -1, 1], [-1, -1, -1], [-1, 1, -1], [-1, 1, 1]],
            [[-1, 1, -1], [1, 1, -1], [1, 1, 1], [-1, 1, 1]],
            [[-1, -1, 1], [1, -1, 1], [1, -1, -1], [-1, -1, -1]],
            [[1, -1, 1], [-1, -1, 1], [-1, 1, 1], [1, 1, 1]],
            [[-1, -1, -1], [1, -1, -1], [1, 1, -1], [-1, 1, -1]],
        ];
        let bounded = radius.is_finite();
        let in_range: Vec<Key> = if bounded {
            self.chunks_near(center, radius)
                .into_iter()
                .map(|(_, k)| k)
                .collect()
        } else {
            self.chunks.keys().copied().collect()
        };
        for ck in in_range {
            let Some(cells) = self.chunks.get(&ck) else {
                continue;
            };
            for (k, c) in cells {
                let p = dequantize(*k);
                let pigment = self
                    .tints
                    .get(&c.plant)
                    .copied()
                    .unwrap_or([0.28, 0.44, 0.23]);
                let col = if c.kind == kind::WOOD {
                    [0.30, 0.28, 0.26, 0.0]
                } else {
                    [pigment[0], pigment[1], pigment[2], 1.0]
                };
                for (face, dir) in dirs.iter().enumerate() {
                    if self.get((k.0 + dir.0, k.1 + dir.1, k.2 + dir.2)) != kind::EMPTY {
                        continue;
                    }
                    let base = verts.len() as i32;
                    for corner in corners[face] {
                        verts.push(std::array::from_fn(|i| {
                            p[i] + corner[i] as f32 * CELL * 0.5
                        }));
                        normals.push([dir.0 as f32, dir.1 as f32, dir.2 as f32]);
                        colors.push(col);
                        uv.push([
                            if self.is_edited(c.plant) { 1.0 } else { 0.0 },
                            c.up as f32 / 255.0,
                        ]);
                    }
                    indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,
                        base,
                        base + 2,
                        base + 3,
                    ]);
                }
            }
        }
        (verts, normals, colors, uv, indices)
    }

    /// Amanatides-Woo grid traversal: cost is crossed cells, not forest size.
    pub fn raycast(
        &self,
        origin: [f32; 3],
        dir: [f32; 3],
        max: f32,
    ) -> Option<([f32; 3], [f32; 3], u8)> {
        let mut k = quantize(origin);
        let mut t = 0.0;
        let mut normal = [0.0; 3];
        let step: [i32; 3] = std::array::from_fn(|i| if dir[i] >= 0.0 { 1 } else { -1 });
        let delta: [f32; 3] = std::array::from_fn(|i| {
            if dir[i].abs() > 1e-8 {
                CELL / dir[i].abs()
            } else {
                f32::INFINITY
            }
        });
        let mut next: [f32; 3] = std::array::from_fn(|i| {
            if dir[i].abs() < 1e-8 {
                return f32::INFINITY;
            }
            let v = [k.0, k.1, k.2][i];
            let edge = (v + if step[i] > 0 { 1 } else { 0 }) as f32 * CELL;
            ((edge - origin[i]) / dir[i]).max(0.0)
        });
        for _ in 0..((max.max(0.0) / CELL * 3.0) as usize + 6).min(100000) {
            if t > max {
                break;
            }
            let material = self.get(k);
            if material != kind::EMPTY {
                return Some((
                    std::array::from_fn(|i| origin[i] + dir[i] * t),
                    normal,
                    material,
                ));
            }
            let axis = if next[0] <= next[1] && next[0] <= next[2] {
                0
            } else if next[1] <= next[2] {
                1
            } else {
                2
            };
            t = next[axis];
            next[axis] += delta[axis];
            normal = [0.0; 3];
            normal[axis] = -step[axis] as f32;
            match axis {
                0 => k.0 += step[0],
                1 => k.1 += step[1],
                _ => k.2 += step[2],
            }
        }
        None
    }

    /// Save only touched material, including empty edited stands (tombstones).
    /// `RWOOD002` = the v1 payload plus the per-organism removal ledger.
    /// v1 saves still load (see `decode_edits`); they simply carry no ledger,
    /// which is correct — they predate it, so nothing had been billed.
    pub fn encode_edits(&self) -> Vec<u8> {
        let mut out = b"RWOOD002".to_vec();
        out.extend_from_slice(&(self.edited.len() as u32).to_le_bytes());
        for id in &self.edited {
            out.extend_from_slice(&id.to_le_bytes());
        }
        let cells: Vec<_> = self
            .chunks
            .values()
            .flat_map(|c| c.iter())
            .filter(|(_, c)| self.is_edited(c.plant))
            .collect();
        out.extend_from_slice(&(cells.len() as u32).to_le_bytes());
        for (k, c) in cells {
            for v in [k.0, k.1, k.2] {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out.extend_from_slice(&c.plant.to_le_bytes());
            out.push(c.kind);
            out.push(c.up);
            for v in self
                .tints
                .get(&c.plant)
                .copied()
                .unwrap_or([0.28, 0.44, 0.23])
            {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        // Ledger last, so the v1 reader's fixed-length payload is untouched.
        let led: Vec<(u32, f32)> = self
            .taken
            .iter()
            .filter(|(_, kg)| **kg > 0.0)
            .map(|(id, kg)| (*id, *kg))
            .collect();
        out.extend_from_slice(&(led.len() as u32).to_le_bytes());
        for (id, kg) in led {
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&kg.to_le_bytes());
        }
        out
    }
    pub fn decode_edits(&mut self, data: &[u8]) -> bool {
        if data.len() < 16 {
            return false;
        }
        let v2 = &data[..8] == b"RWOOD002";
        if !v2 && &data[..8] != b"RWOOD001" {
            return false;
        }
        let read = |i: usize| u32::from_le_bytes(data[i..i + 4].try_into().unwrap());
        let n = read(8) as usize;
        if n > 1_000_000 || data.len() < 16 + n * 4 {
            return false;
        }
        let base = 12 + n * 4;
        let count = read(base) as usize;
        let cells_end = base + 4 + count * 30;
        if count > MAX_CELLS || data.len() < cells_end {
            return false;
        }
        // v1 was exact-length; v2 appends the ledger after the cells.
        let mut ledger: Vec<(u32, f32)> = Vec::new();
        if v2 {
            if data.len() < cells_end + 4 {
                return false;
            }
            let m = read(cells_end) as usize;
            if m > 1_000_000 || data.len() != cells_end + 4 + m * 8 {
                return false;
            }
            for i in 0..m {
                let o = cells_end + 4 + i * 8;
                let kg = f32::from_le_bytes(data[o + 4..o + 8].try_into().unwrap());
                if !kg.is_finite() || kg < 0.0 {
                    return false;
                }
                ledger.push((read(o), kg));
            }
        } else if data.len() != cells_end {
            return false;
        }
        // Validate before mutating, so a corrupt save leaves the session intact.
        for row in data[base + 4..cells_end].chunks_exact(30) {
            if !matches!(row[16], 1 | 2) {
                return false;
            }
            for i in [18, 22, 26] {
                if !f32::from_le_bytes(row[i..i + 4].try_into().unwrap()).is_finite() {
                    return false;
                }
            }
        }
        self.clear();
        for (id, kg) in ledger {
            self.taken.insert(id, kg);
        }
        for i in 0..n {
            let id = read(12 + i * 4);
            self.edited.insert(id);
            self.sprouted.insert(id, [0.0; 3]);
        }
        for row in data[base + 4..cells_end].chunks_exact(30) {
            let v = |i| i32::from_le_bytes(row[i..i + 4].try_into().unwrap());
            let id = u32::from_le_bytes(row[12..16].try_into().unwrap());
            let tint = std::array::from_fn(|i| {
                f32::from_le_bytes(row[18 + i * 4..22 + i * 4].try_into().unwrap())
            });
            self.tints.insert(id, tint);
            let k = (v(0), v(4), v(8));
            if self
                .chunks
                .entry(chunk_of(k))
                .or_default()
                .insert(
                    k,
                    Cell {
                        kind: row[16],
                        up: row[17],
                        plant: id,
                    },
                )
                .is_none()
            {
                self.count += 1;
            }
        }
        true
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
            out.extend_from_slice(&[p[0], p[1], p[2], cell.kind as f32, cell.up as f32 / 255.0]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposed_faces_remove_shared_interiors() {
        let mut ws = Woodscape::default();
        ws.set((0, 0, 0), kind::WOOD, 1);
        ws.set((1, 0, 0), kind::LEAF, 1);
        let (v, n, _, _, i) = ws.surface();
        assert_eq!(v.len(), 40);
        assert_eq!(i.len(), 60);
        assert_eq!(n.len(), 40);
    }
    #[test]
    fn grid_raycast_hits_material_and_reports_outward_face() {
        let mut ws = Woodscape::default();
        ws.set((0, 0, 0), kind::WOOD, 1);
        let (hit, normal, kind) = ws.raycast([-2.0, 0.2, 0.2], [1.0, 0.0, 0.0], 4.0).unwrap();
        assert!(hit[0].abs() < 1e-5);
        assert_eq!(normal, [-1.0, 0.0, 0.0]);
        assert_eq!(kind, 1);
        assert!(ws.raycast([-2.0, 1.0, 0.2], [1.0, 0.0, 0.0], 4.0).is_none());
        assert!(ws.raycast([-2.0, 0.2, 0.2], [1.0, 0.0, 0.0], 1.0).is_none());
    }
    #[test]
    fn harvesting_and_placed_material_survive_streaming_and_save() {
        let mut ws = Woodscape::default();
        ws.set_up((0, 0, 0), kind::WOOD, 1, 100);
        ws.set_up((1, 0, 0), kind::LEAF, 1, 110);
        ws.sprouted.insert(1, [0.0; 3]);
        let (w, l, n) = ws.harvest_sphere(dequantize((0, 0, 0)), 0.1);
        assert_eq!((w, l, n), (2.8, 0.0, 1));
        ws.set((12, 0, 0), kind::WOOD, u32::MAX);
        ws.prune_far([1000.0; 3], 10.0);
        assert_eq!(ws.get((0, 0, 0)), kind::EMPTY);
        assert_eq!(ws.get((1, 0, 0)), kind::LEAF);
        assert_eq!(ws.get((12, 0, 0)), kind::WOOD);
        let bytes = ws.encode_edits();
        let mut restored = Woodscape::default();
        assert!(restored.decode_edits(&bytes));
        assert_eq!(restored.len(), 2);
        assert!(restored.is_sprouted(1));
        assert!(restored.is_edited(1));
        assert_eq!(restored.get((0, 0, 0)), kind::EMPTY);
        assert_eq!(restored.get((12, 0, 0)), kind::WOOD);
        let before = restored.len();
        assert!(!restored.decode_edits(&bytes[..bytes.len() - 1]));
        assert_eq!(restored.len(), before);
    }
    #[test]
    fn completely_harvested_stand_keeps_tombstone() {
        let mut ws = Woodscape::default();
        ws.set_up((0, 0, 0), kind::WOOD, 9, 0);
        ws.sprouted.insert(9, [0.0; 3]);
        ws.harvest_sphere([0.0; 3], 2.0);
        let mut restored = Woodscape::default();
        assert!(restored.decode_edits(&ws.encode_edits()));
        assert!(restored.is_edited(9));
        assert!(restored.is_sprouted(9));
        assert!(restored.is_empty());
    }

    #[test]
    fn adjacent_wood_combines_under_one_owner() {
        let mut ws = Woodscape::default();
        ws.set((0, 0, 0), kind::WOOD, 1);
        ws.set((1, 0, 0), kind::WOOD, 2); // grafts onto plant 1
        assert_eq!(ws.cell((1, 0, 0)).unwrap().plant, 1);
    }

    /// Blocks removed by the dig path must be billed to the organism they came
    /// from, so the fell path can pay only the remainder.
    #[test]
    fn harvest_attributes_material_to_its_organism() {
        let mut ws = Woodscape::default();
        for i in 0..10 {
            ws.set((i, 0, 0), kind::WOOD, 7);
        }
        for i in 0..4 {
            ws.set((i, 1, 0), kind::LEAF, 7);
        }
        // Player construction is not an organism and never enters a ledger.
        ws.set((50, 0, 0), kind::WOOD, u32::MAX);
        assert_eq!(ws.taken_kg(7), 0.0);

        let centre = dequantize((2, 0, 0));
        let (wood_kg, leaf_kg, n) = ws.harvest_sphere(centre, 1.6);
        assert!(n > 0, "the sphere should have taken something");
        assert!(
            (ws.taken_kg(7) - (wood_kg + leaf_kg)).abs() < 1e-3,
            "ledger {} does not match the {wood_kg:.2} wood + {leaf_kg:.2} leaf paid out",
            ws.taken_kg(7)
        );
        assert!(ws.is_edited(7), "a harvested stand is a touched stand");

        let before = ws.taken_kg(7);
        ws.harvest_sphere(dequantize((50, 0, 0)), 1.0);
        assert_eq!(
            ws.taken_kg(7),
            before,
            "player-placed material billed an organism"
        );
        assert_eq!(ws.taken_kg(u32::MAX), 0.0, "construction has no ledger");
    }

    /// Felling clears the cells but must leave the ledger standing, or walking
    /// away and back would make the tree payable again.
    #[test]
    fn removing_a_stand_keeps_its_ledger() {
        let mut ws = Woodscape::default();
        for i in 0..8 {
            ws.set((i, 0, 0), kind::WOOD, 3);
        }
        ws.harvest_sphere(dequantize((1, 0, 0)), 1.2);
        let billed = ws.taken_kg(3);
        assert!(billed > 0.0);
        ws.add_taken(3, 40.0);
        ws.remove_plant(3);
        assert!(
            (ws.taken_kg(3) - (billed + 40.0)).abs() < 1e-3,
            "remove_plant discarded the ledger"
        );
        assert!(ws.is_edited(3), "a felled stand stays a tombstone");
    }

    /// The ledger is only durable if it survives the save. And a v1 save must
    /// still load — it predates the ledger, so nothing had been billed.
    #[test]
    fn ledger_survives_save_and_v1_still_loads() {
        let mut ws = Woodscape::default();
        for i in 0..6 {
            ws.set((i, 2, -1), kind::WOOD, 11);
        }
        ws.harvest_sphere(dequantize((0, 2, -1)), 1.1);
        ws.add_taken(11, 12.5);
        let want = ws.taken_kg(11);
        let cells = ws.len();
        let blob = ws.encode_edits();
        assert_eq!(&blob[..8], b"RWOOD002");

        let mut other = Woodscape::default();
        assert!(other.decode_edits(&blob), "v2 save failed to load");
        assert!(
            (other.taken_kg(11) - want).abs() < 1e-3,
            "ledger lost across save: {} vs {want}",
            other.taken_kg(11)
        );
        assert_eq!(other.len(), cells, "cells lost across save");
        assert!(other.is_edited(11));

        // A v1 blob is the same payload with the old magic and no ledger.
        let mut v1 = blob.clone();
        // Strip the trailing ledger and relabel.
        v1.truncate(blob.len() - _ledger_bytes(&blob) - 4);
        v1[..8].copy_from_slice(b"RWOOD001");
        let mut old = Woodscape::default();
        assert!(old.decode_edits(&v1), "v1 save must still load");
        assert_eq!(old.taken_kg(11), 0.0, "v1 carries no ledger");
        assert_eq!(old.len(), cells, "v1 cells must still load");

        // Corrupt saves change nothing.
        let mut bad = blob.clone();
        let n = bad.len();
        bad[n - 1] = 0xFF;
        bad[n - 2] = 0xFF;
        bad[n - 3] = 0xFF;
        bad[n - 4] = 0x7F;
        let mut victim = Woodscape::default();
        victim.set((0, 0, 0), kind::WOOD, 1);
        let before = victim.len();
        let _ = victim.decode_edits(&bad);
        assert!(victim.len() == before || victim.len() == cells);
    }

    /// Bytes the trailing v2 ledger occupies in an encoded blob.
    fn _ledger_bytes(blob: &[u8]) -> usize {
        let read = |i: usize| u32::from_le_bytes(blob[i..i + 4].try_into().unwrap()) as usize;
        let n = read(8);
        let base = 12 + n * 4;
        let count = read(base);
        let cells_end = base + 4 + count * 30;
        read(cells_end) * 8
    }

    /// The bug this replaced: a raycast hit lands on a block face, and probing
    /// one axis to decide "is this wood" missed four faces out of six — so
    /// aiming at a trunk from most directions mined nothing at all.
    #[test]
    fn a_face_hit_is_detected_from_every_direction() {
        let mut ws = Woodscape::default();
        let k = (4, 5, -6);
        ws.set(k, kind::WOOD, 2);
        let c = dequantize(k);
        let half = CELL * 0.5;
        for (dx, dy, dz) in [
            (1.0, 0.0, 0.0),
            (-1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (0.0, -1.0, 0.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, -1.0),
        ] {
            // Exactly on the face, as a ray hit reports it.
            let hit = [c[0] + dx * half, c[1] + dy * half, c[2] + dz * half];
            assert!(
                ws.material_within(hit, 0.48),
                "face ({dx},{dy},{dz}) not detected"
            );
            assert_eq!(ws.nearest_cell(hit, 0.48).map(|(kk, _)| kk), Some(k));
        }
        // Ground a metre away is not the tree.
        assert!(!ws.material_within([c[0] + 1.2, c[1], c[2]], 0.48));
    }

    /// One block per swing, and it bills the ledger like any other removal.
    #[test]
    fn harvest_one_takes_a_single_block() {
        let mut ws = Woodscape::default();
        for i in 0..5 {
            ws.set((i, 0, 0), kind::WOOD, 9);
        }
        ws.set((0, 1, 0), kind::LEAF, 9);
        let before = ws.len();
        let c = dequantize((2, 0, 0));
        let got = ws.harvest_one([c[0], c[1] + CELL * 0.5, c[2]], 0.48);
        assert_eq!(got, Some((kind::WOOD, WOOD_KG)));
        assert_eq!(ws.len(), before - 1, "more than one block came away");
        assert!((ws.taken_kg(9) - WOOD_KG).abs() < 1e-4, "ledger not billed");

        // Leaf pays the leaf rate.
        let lc = dequantize((0, 1, 0));
        assert_eq!(ws.harvest_one(lc, 0.48), Some((kind::LEAF, LEAF_KG)));

        // Nothing in reach takes nothing.
        assert_eq!(ws.harvest_one([500.0, 500.0, 500.0], 0.48), None);
    }

    /// A cut tree must be able to recover, and the recovery must be paid for.
    ///
    /// Forest plan, stage 5: "a cut organism can survive, die or produce new
    /// shoots according to its remaining support and resources, while the cut
    /// remains a persistent historical edit... new growth gets new material; it
    /// must not erase the edit mask or restore removed mass for free."
    #[test]
    fn regrowth_pays_down_the_ledger_and_keeps_the_edit() {
        let mut ws = Woodscape::default();
        for i in 0..8 {
            ws.set((i, 0, 0), kind::WOOD, 4);
        }
        // Chop three blocks.
        for i in 0..3 {
            let c = dequantize((i, 0, 0));
            ws.harvest_one(c, 0.48);
        }
        let taken = ws.taken_kg(4);
        assert!((taken - 3.0 * WOOD_KG).abs() < 1e-3, "billed {taken}");
        assert!(ws.is_edited(4), "the cut must be recorded");

        // Two blocks of regrowth pays back two blocks' worth, no more.
        ws.credit_growth(4, WOOD_KG);
        ws.credit_growth(4, WOOD_KG);
        assert!(
            (ws.taken_kg(4) - WOOD_KG).abs() < 1e-3,
            "ledger should be one block down, is {}",
            ws.taken_kg(4)
        );
        // The edit itself is history and never goes away.
        assert!(ws.is_edited(4), "regrowth erased the edit mask");

        // Regrowth cannot turn the ledger into credit the player can spend.
        for _ in 0..20 {
            ws.credit_growth(4, WOOD_KG);
        }
        assert_eq!(ws.taken_kg(4), 0.0, "ledger went below zero");
    }

    /// Meshing must not drag in edited groves the player left kilometres back.
    /// `prune_far` keeps those resident on purpose, so the mesh has to be the
    /// thing that bounds them.
    #[test]
    fn surface_is_bounded_by_the_render_radius() {
        let mut ws = Woodscape::default();
        // A stand under the camera, and one far away that is edited (so it can
        // never be pruned).
        for i in 0..6 {
            ws.set((i, 0, 0), kind::WOOD, 1);
            ws.set((i + 4000, 0, 0), kind::WOOD, 2);
        }
        ws.harvest_one(dequantize((4000, 0, 0)), 0.48);
        assert!(ws.is_edited(2), "the distant stand must be a touched stand");

        let here = dequantize((2, 0, 0));
        let (near_v, _, _, _, near_i) = ws.surface_near(here, 60.0);
        let (all_v, _, _, _, all_i) = ws.surface();
        assert!(!near_v.is_empty(), "the stand underfoot should mesh");
        assert!(
            near_v.len() < all_v.len() && near_i.len() < all_i.len(),
            "bounded mesh ({} verts) should be smaller than everything ({})",
            near_v.len(),
            all_v.len()
        );
        // Nothing in the bounded mesh may come from the far stand.
        for v in &near_v {
            assert!(
                v[0] < 1000.0,
                "a cell from the distant grove leaked into the near mesh"
            );
        }
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
            ws.set_up((i * 40, 0, 0), kind::WOOD, i as u32, 0);
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

#[cfg(test)]
mod bite {
    use super::*;
    use crate::habitat::Habitat;
    use crate::terrain::Terrain;

    fn a_big_tree() -> (usize, crate::plant::Plant, Terrain) {
        let hab = Habitat::kepler_drum();
        let ter = Terrain::generate(hab);
        let bio = crate::biosphere::Biosphere::new(&ter);
        let (i, p) = bio
            .plants
            .plants
            .iter()
            .enumerate()
            .find(|(_, p)| p.alive && p.species == 7)
            .map(|(i, p)| (i, p.clone()))
            .expect("the drum has canopy trees");
        (i, p, ter)
    }

    /// Aim at the top of the crown, then at the trunk, so both materials get
    /// hit through the same path the dig uses.
    fn aim_at(w: &Woodscape, want: u8) -> [f32; 3] {
        let mut aim = [0.0f32; 3];
        let mut best = -1e30f32;
        for ch in w.chunks.values() {
            for (k, c) in ch {
                let p = dequantize(*k);
                let h = p[0] * p[0] + p[1] * p[1];
                if c.kind == want && h > best {
                    best = h;
                    aim = p;
                }
            }
        }
        aim
    }

    fn up_at(ter: &Terrain, p: &crate::plant::Plant) -> [f32; 3] {
        let o = ter
            .hab
            .to_world(p.theta, p.z, ter.hab.radius - ter.elevation(p.theta, p.z));
        let l = (o[0] * o[0] + o[1] * o[1]).sqrt().max(1e-6);
        [-o[0] / l, -o[1] / l, 0.0]
    }

    /// What an untouched tree's wood actually looks like as a graph.
    #[test]
    #[ignore]
    fn is_a_generated_tree_one_connected_piece() {
        let (pi, plant, ter) = a_big_tree();
        let mut w = Woodscape::default();
        w.sprout_plant(pi, &plant, &ter, 7);
        let pid = pi as u32;
        let origin = *w.sprouted.get(&pid).unwrap();
        let up = up_at(&ter, &plant);
        let base_h = origin[0] * up[0] + origin[1] * up[1] + origin[2] * up[2];
        let mut wood: FastMap<Key, ()> = FastMap::default();
        let mut leaf = 0usize;
        let mut lowest = f32::INFINITY;
        for ck in w.plant_chunks(pid, origin) {
            for (k, c) in w.chunks.get(&ck).unwrap() {
                if c.plant != pid {
                    continue;
                }
                if c.kind == kind::WOOD {
                    wood.insert(*k, ());
                    let p = dequantize(*k);
                    lowest = lowest.min(p[0] * up[0] + p[1] * up[1] + p[2] * up[2] - base_h);
                } else {
                    leaf += 1;
                }
            }
        }
        // Connected components of the wood, under corner adjacency.
        let mut seen: FastMap<Key, u32> = FastMap::default();
        let mut comps: Vec<usize> = Vec::new();
        for start in wood.keys() {
            if seen.contains_key(start) {
                continue;
            }
            let id = comps.len() as u32;
            let mut n = 0usize;
            let mut q = vec![*start];
            seen.insert(*start, id);
            while let Some(k) = q.pop() {
                n += 1;
                for nb in neighbors26(k) {
                    if wood.contains_key(&nb) && !seen.contains_key(&nb) {
                        seen.insert(nb, id);
                        q.push(nb);
                    }
                }
            }
            comps.push(n);
        }
        comps.sort_unstable_by(|a, b| b.cmp(a));
        println!(
            "wood {} leaf {leaf} | lowest wood cell sits {lowest:.2} m above the origin",
            wood.len()
        );
        println!(
            "wood forms {} connected pieces; largest ten: {:?}",
            comps.len(),
            &comps[..comps.len().min(10)]
        );
    }

    /// A tree nobody has touched must not fall over. This is the test that
    /// decides whether face adjacency is usable at all: if `sprout_plant`
    /// joins a branch to its trunk by a corner rather than a face, an
    /// untouched canopy reads as severed and the whole forest collapses on
    /// the first tick.
    #[test]
    fn an_untouched_tree_is_holding_itself_up() {
        let (pi, plant, ter) = a_big_tree();
        let mut w = Woodscape::default();
        w.sprout_plant(pi, &plant, &ter, 7);
        let before = w.len();
        let fell = w.collapse_severed(pi as u32, up_at(&ter, &plant));
        assert!(
            fell.is_none(),
            "an untouched tree shed {:?} of its {before} cells",
            fell.map(|f| f.blocks)
        );
    }

    /// Height of a cell up its own tree, in metres above the stump.
    fn height_of(k: Key, up: [f32; 3], base_h: f32) -> f32 {
        let p = dequantize(k);
        p[0] * up[0] + p[1] * up[1] + p[2] * up[2] - base_h
    }

    /// Take out every wood cell of one stand in a height band — a felling cut,
    /// rather than a single chop. A canopy tree's base is wider than the
    /// largest brush, so no one swing can sever it; that is the point.
    /// Returns kilograms removed, and the wood cells left above the band.
    fn cut_band(w: &mut Woodscape, pid: u32, up: [f32; 3], lo: f32, hi: f32) -> (f32, u32) {
        let origin = *w.sprouted.get(&pid).unwrap();
        let base_h = origin[0] * up[0] + origin[1] * up[1] + origin[2] * up[2];
        let mut band = Vec::new();
        let mut above = 0u32;
        for ck in w.plant_chunks(pid, origin) {
            let Some(chunk) = w.chunks.get(&ck) else {
                continue;
            };
            for (k, c) in chunk {
                if c.plant != pid || c.kind != kind::WOOD {
                    continue;
                }
                let h = height_of(*k, up, base_h);
                if h >= lo && h < hi {
                    band.push(dequantize(*k));
                } else if h >= hi {
                    above += 1;
                }
            }
        }
        // Sum what actually came away rather than inferring it from a count:
        // `nearest_cell` picks whatever is closest, so a count times WOOD_KG
        // is a guess and the ledger test is meant to catch guesses.
        let mut kg = 0.0f32;
        for c in band {
            if let Some((_, got)) = w.harvest_one(c, 0.1) {
                kg += got;
            }
        }
        (kg, above)
    }

    /// Sever the trunk halfway up and everything above the cut has to come
    /// down. This is the rule itself, isolated from how wide a bite one swing
    /// happens to take.
    #[test]
    fn wood_above_a_severed_trunk_comes_down() {
        let (pi, plant, ter) = a_big_tree();
        let mut w = Woodscape::default();
        w.sprout_plant(pi, &plant, &ter, 7);
        let pid = pi as u32;
        let up = up_at(&ter, &plant);
        let (cut, above) = cut_band(&mut w, pid, up, 12.0, 14.0);
        assert!(
            cut > 0.0 && above > 0,
            "cut {cut:.1} kg with {above} cells left above"
        );

        let fell = w
            .collapse_severed(pid, up)
            .expect("wood above a severed trunk should fall");
        assert!(
            fell.blocks >= above,
            "{above} wood cells were left above the cut but only {} fell",
            fell.blocks
        );
        assert!(
            fell.wood_kg > 1000.0,
            "only {:.0} kg of timber came down",
            fell.wood_kg
        );
    }

    /// Take the whole base out and the tree goes with it, as material — timber
    /// you felled cannot simply cease to exist.
    #[test]
    fn cutting_out_the_base_brings_the_whole_tree_down() {
        let (pi, plant, ter) = a_big_tree();
        let mut w = Woodscape::default();
        w.sprout_plant(pi, &plant, &ter, 7);
        let pid = pi as u32;
        let up = up_at(&ter, &plant);
        let total = w.len();
        cut_band(&mut w, pid, up, -3.0, 2.0);
        let fell = w.collapse_severed(pid, up).expect("it should come down");
        assert!(fell.blocks > 10_000, "only {} wood cells fell", fell.blocks);

        // And the crown follows, given ticks to crumble in.
        let mut guard = 0;
        while w.decay_leaves(20_000) > 0 && guard < 400 {
            guard += 1;
        }
        assert!(
            w.len() < total / 20,
            "{} of {total} cells still standing after the crown should have gone",
            w.len()
        );
    }

    /// Everything that comes down is billed to the tree, so felling by axe and
    /// felling with `H` cannot both be paid for.
    #[test]
    fn what_falls_is_still_on_the_ledger() {
        let (pi, plant, ter) = a_big_tree();
        let mut w = Woodscape::default();
        w.sprout_plant(pi, &plant, &ter, 7);
        let pid = pi as u32;
        let up = up_at(&ter, &plant);
        let (cut, _) = cut_band(&mut w, pid, up, 12.0, 14.0);
        let fell = w.collapse_severed(pid, up).expect("it should fall");
        let billed = w.taken_kg(pid);
        let accounted = cut + fell.wood_kg;
        // Relative, not absolute: this is 33 tonnes accumulated as ~12,000
        // f32 additions of 2.8, and the ledger adds them in a different order
        // from the test. At that magnitude f32 steps are ~0.002 kg, so the two
        // sums differ by a few kg without a single kilogram going astray.
        assert!(
            (billed - accounted).abs() < accounted * 5e-4,
            "ledger says {billed:.1} kg, cut plus fall is {accounted:.1} kg"
        );
    }

    /// The brush is an axe head, so a bigger brush has to take a bigger bite.
    /// It used to take exactly one block at every size, which made the whole
    /// size control cosmetic.
    #[test]
    fn a_bigger_brush_takes_a_bigger_bite() {
        let (pi, plant, ter) = a_big_tree();
        let mut last = 0u32;
        for brush in [1.2f32, 2.0, 2.6, 4.0, 7.0] {
            let mut w = Woodscape::default();
            w.sprout_plant(pi, &plant, &ter, 7);
            let aim = aim_at(&w, kind::LEAF);
            let (_, _, n) = w.harvest_sphere(aim, brush * Woodscape::bite_scale(kind::LEAF));
            assert!(
                n > last,
                "brush {brush} took {n} blocks, no more than the smaller brush's {last}"
            );
            last = n;
        }
    }

    /// Wood resists and foliage does not, the way rock resists and soil does
    /// not. Without this a swing into a trunk clears as much as a swing
    /// through leaves and nothing feels like timber.
    #[test]
    fn wood_gives_way_less_than_leaves() {
        let (pi, plant, ter) = a_big_tree();
        let brush = 2.6f32;
        let mut counts = Vec::new();
        for want in [kind::WOOD, kind::LEAF] {
            let mut w = Woodscape::default();
            w.sprout_plant(pi, &plant, &ter, 7);
            let aim = aim_at(&w, want);
            let (_, _, n) = w.harvest_sphere(aim, brush * Woodscape::bite_scale(want));
            counts.push(n);
        }
        assert!(
            counts[0] * 3 < counts[1],
            "a trunk bite of {} is not meaningfully tighter than a leaf bite of {}",
            counts[0],
            counts[1]
        );
    }

    /// The smallest brush against wood has to stay a single block, so anyone
    /// who wants to carve precisely still can.
    #[test]
    fn the_smallest_brush_still_takes_one_block() {
        let (pi, plant, ter) = a_big_tree();
        let mut w = Woodscape::default();
        w.sprout_plant(pi, &plant, &ter, 7);
        let aim = aim_at(&w, kind::WOOD);
        let r = 1.2 * Woodscape::bite_scale(kind::WOOD);
        assert!(r < CELL, "a minimum wood bite of {r} m spans a whole cell");
        let (_, _, n) = w.harvest_sphere(aim, r);
        assert_eq!(n, 1, "smallest brush took {n} blocks");
    }
}
