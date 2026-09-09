//! Live flow routing — the landscape stops being a fossil.
//!
//! LANDSCAPE_200.md §C + LANDSCAPE_800 verified blocker:
//!   Priority-flood seeded on the endcaps fills 97% of a sealed cylinder to the
//!   rim (no ocean outlet). Outlets are the engineered waterline instead —
//!   cells at/below `water_level` are free drains into the habitat's closed
//!   water budget. Digs then genuinely reroute.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::terrain::{idx, NT, NZ};

const N8: [(i32, i32); 8] = [
    (-1, 0),
    (1, 0),
    (0, -1),
    (0, 1),
    (-1, -1),
    (-1, 1),
    (1, -1),
    (1, 1),
];

/// Local repairs after which a full priority flood is *requested*, to be
/// spent on a tick where it will not be felt.
const SOFT_FULL: u32 = 24;
/// Local repairs after which one is taken regardless.
const HARD_FULL: u32 = 64;

#[derive(Clone)]
pub struct Flow {
    pub flux: Vec<f32>,
    pub filled: Vec<f32>,
    pub down: Vec<u32>,
    pub lake: Vec<u8>,
    pub discharge: Vec<f32>,
    /// CSR upstream graph: for cell i, ups are `up_idx[up_off[i]..up_off[i+1]]`.
    pub up_off: Vec<u32>,
    pub up_idx: Vec<u32>,
    /// Hash of the downstream-pointer field — changes when routing reroutes.
    pub route_sig: u64,
    pub lake_count: u32,
    pub mean_fill_depth: f32,
    dirty: bool,
    /// Dig-centred patch for fast rebuild (skip global priority-flood).
    dirty_t: i32,
    dirty_z: i32,
    dirty_r: i32,
    /// Full flood every N local rebuilds so ponds stay honest.
    local_rebuilds: u32,
    /// A player edit is waiting, and wants a much shorter deadline than the
    /// erosion cadence gives.
    urgent: bool,

    // ---- local-rebuild scratch ----
    /// Cells whose `filled` the current edit changed.
    patch: Vec<u32>,
    /// `patch` plus its eight-neighbour rim: every cell whose downstream
    /// pointer could therefore move, and no others.
    halo: Vec<u32>,
    /// Where each halo cell pointed before the edit, parallel to `halo`.
    was_down: Vec<u32>,
    /// Cells whose discharge changed, so flux can be renormalised over them
    /// instead of over the whole drum.
    touched: Vec<u32>,
    /// Reverse index for `halo`: `slot[cell] = k + 1` while cell is halo
    /// member `k`, and 0 otherwise.
    ///
    /// A hash map would do the same job without the 6 MB, but this sits in
    /// the innermost loop of the incremental accumulation and a lookup per
    /// edge is the whole cost. It is left zeroed at the end of every call —
    /// clearing costs one pass over the halo, not over the drum.
    slot: Vec<u32>,
    /// Largest discharge the last full accumulation saw. Flux is a fraction of
    /// it, so holding it lets a local edit renormalise only what it touched.
    flux_max: f32,
    /// How many local repairs have had to give up and redo the whole field.
    /// Expected to stay at zero: every known reason for it is either a cycle
    /// D8's strict descent forbids, or a patch too large to be local.
    fallbacks: u32,
    /// True when `up_off`/`up_idx` no longer match `down`. Rebuilding the CSR
    /// costs 5.6 ms and only the catchment overlay reads it, so it is rebuilt
    /// on demand rather than on every edit.
    up_stale: bool,
}

impl Default for Flow {
    fn default() -> Self {
        let n = NT * NZ;
        Self {
            flux: vec![0.0; n],
            filled: vec![0.0; n],
            down: vec![0; n],
            lake: vec![0; n],
            discharge: vec![0.0; n],
            up_off: vec![0; n + 1],
            up_idx: Vec::new(),
            route_sig: 0,
            lake_count: 0,
            mean_fill_depth: 0.0,
            dirty: true,
            dirty_t: -1,
            dirty_z: -1,
            dirty_r: 0,
            local_rebuilds: 0,
            urgent: false,
            patch: Vec::new(),
            halo: Vec::new(),
            was_down: Vec::new(),
            touched: Vec::new(),
            slot: vec![0u32; n],
            flux_max: 1e-6,
            fallbacks: 0,
            up_stale: false,
        }
    }
}

impl Flow {
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Mark a cylindrical neighbourhood dirty (player dig / fill).
    ///
    /// This is a deliberate edit, not erosion nudging the field, so it also
    /// raises `urgent`: the routing behind it is something the player is
    /// standing over and waiting to see happen.
    pub fn mark_dirty_at(&mut self, ti: usize, zi: usize, radius_cells: i32) {
        self.dirty = true;
        self.urgent = true;
        let r = radius_cells.max(4);
        if self.dirty_t < 0 {
            self.dirty_t = ti as i32;
            self.dirty_z = zi as i32;
            self.dirty_r = r;
        } else {
            // Expand to cover both centres (θ wraps — approximate via larger radius).
            let dz = (zi as i32 - self.dirty_z).abs();
            let dt = {
                let a = (ti as i32 - self.dirty_t).rem_euclid(NT as i32);
                a.min(NT as i32 - a)
            };
            self.dirty_r = self.dirty_r.max(r).max(dt + r).max(dz + r);
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Is a player edit waiting on a rebuild?
    ///
    /// Erosion can wait — nobody is watching a hillside creep. A trench the
    /// player just cut cannot: the throttle that serves erosion is 0.7
    /// habitat days, which is 35 real seconds at 1x, and for 35 seconds after
    /// digging a channel out of a lake the water did not move at all.
    pub fn is_urgent(&self) -> bool {
        self.urgent
    }

    /// Has enough local repair accumulated that a full flood is due?
    ///
    /// The routing repair is exact, but the ponding step is not: it collapses
    /// `filled` to the ground in the patch rather than re-running the priority
    /// flood, so a freshly dug pit is not classified as a basin until a full
    /// rebuild. That used to self-correct every eighth local rebuild, which
    /// was every few minutes when digs were serviced 35 seconds apart. Now
    /// that they are serviced in half a second, every eighth would put a
    /// 240 ms hitch in the middle of the eighth swing.
    ///
    /// So the request is advisory: the caller is asked to spend it on a tick
    /// when the player is not mid-edit. `HARD_FULL` is the backstop for
    /// someone who never stops digging.
    pub fn wants_full(&self) -> bool {
        self.local_rebuilds >= SOFT_FULL
    }

    /// Rebuild from elevation. `water_level` cells are outlets (habitat drains).
    pub fn rebuild(&mut self, elev: &[f32], water_level: f32) {
        debug_assert_eq!(elev.len(), NT * NZ);
        // Finite-ish fill: flood only toward waterline outlets, and never raise
        // a cell more than MAX_POND above its true surface (caps absurd basins).
        priority_flood_waterline(elev, water_level, &mut self.filled);
        self.finish_routing(elev, water_level);
        self.dirty_t = -1;
        self.dirty_r = 0;
        self.local_rebuilds = 0;
        self.urgent = false;
    }

    /// Fast path after a local dig: drop ponding in the patch, then repair
    /// routing over that patch and its downstream consequences only.
    ///
    /// The global path costs ~44 ms on the 1536x1024 field — 20 ms of D8,
    /// 11 ms of accumulation, 6 ms of upstream CSR, 6 ms of flux — every bit
    /// of it over 1.57M cells, to service an edit that touched a few hundred.
    /// Skipping the priority flood alone was not enough to make a dig's water
    /// answer promptly without a hitch. This path is ~1.3 ms, of which 1.0 ms
    /// is the lake rescan that has to stay global; see `timing`.
    ///
    /// It is exact in `down`, `lake` and `discharge`, and within 0.06% in
    /// `flux`, which `local_rebuild::a_local_repair_matches_a_full_rebuild`
    /// holds it to. What it does *not* reproduce is the priority flood: the
    /// ponding step collapses `filled` to the ground in the patch instead, so
    /// a freshly dug pit is not classified as a basin until `wants_full` is
    /// honoured. That approximation predates this and is why the fallback
    /// exists at all.
    pub fn rebuild_local(&mut self, elev: &[f32], water_level: f32) -> bool {
        // A patch this large is not local in any useful sense, and a global
        // mark has no patch to work from at all.
        if self.dirty_t < 0 || self.dirty_r > 120 || self.local_rebuilds >= HARD_FULL {
            self.rebuild(elev, water_level);
            return true;
        }
        let t0 = self.dirty_t;
        let z0 = self.dirty_z;
        let r = self.dirty_r;
        self.patch.clear();
        for dz in -r..=r {
            let zi = z0 + dz;
            if zi < 0 || zi >= NZ as i32 {
                continue;
            }
            for dt in -r..=r {
                let ti = (t0 + dt).rem_euclid(NT as i32) as usize;
                let i = idx(ti, zi as usize);
                // Dig opens drainage: collapse local ponding back to the surface.
                self.filled[i] = elev[i];
                self.patch.push(i as u32);
            }
        }
        // Walk a short downstream ribbon and clear ponding so trenches connect.
        let mut cur = idx(
            (t0.rem_euclid(NT as i32)) as usize,
            z0.clamp(0, NZ as i32 - 1) as usize,
        ) as u32;
        for _ in 0..256 {
            let i = cur as usize;
            if i >= elev.len() {
                break;
            }
            self.filled[i] = elev[i];
            self.patch.push(cur);
            let nxt = self.down[i];
            if nxt == cur {
                break;
            }
            cur = nxt;
        }
        self.repair_routing(elev, water_level);
        self.local_rebuilds += 1;
        self.dirty_t = -1;
        self.dirty_r = 0;
        self.urgent = false;
        true
    }

    /// Re-route only what `self.patch` could have changed.
    fn repair_routing(&mut self, elev: &[f32], water_level: f32) {
        // A cell's D8 pointer is a function of its own `filled` and its eight
        // neighbours', so the patch and its rim bound everything that moved.
        self.halo.clear();
        for k in 0..self.patch.len() {
            let i = self.patch[k] as usize;
            let (t, z) = (i % NT, i / NT);
            for &(dt, dz) in std::iter::once(&(0i32, 0i32)).chain(N8.iter()) {
                let nz = z as i32 + dz;
                if nz < 0 || nz >= NZ as i32 {
                    continue;
                }
                let nt = (t as i32 + dt).rem_euclid(NT as i32) as usize;
                let ni = idx(nt, nz as usize);
                if self.slot[ni] == 0 {
                    self.halo.push(ni as u32);
                    self.slot[ni] = self.halo.len() as u32;
                }
            }
        }
        let seeds = self.halo.len();

        self.was_down.clear();
        let mut moved = false;
        for k in 0..seeds {
            let i = self.halo[k] as usize;
            self.was_down.push(self.down[i]);
            let nd = d8_at(&self.filled, i);
            moved |= nd != self.down[i];
            self.down[i] = nd;
        }

        let mut full_flux = false;
        if moved {
            // Routing moved, so the CSR and the accumulation are both stale.
            self.up_stale = true;
            if !self.close_downstream(seeds) || !self.accumulate_patch() {
                accumulate(&self.down, &mut self.discharge);
                self.fallbacks += 1;
                full_flux = true;
            }
        } else {
            // Ponding changed but every pointer held: the accumulation is a
            // pure function of the pointers, so it is still exact.
            self.touched.clear();
        }

        // Leave the reverse index clean for the next call.
        for &c in &self.halo {
            self.slot[c as usize] = 0;
        }

        // Lakes stay a global scan, unlike everything else here. They are the
        // one reading that depends on `elev`, and erosion rewrites `elev`
        // across the whole drum on the very ticks a local rebuild runs — so a
        // patch-local lake scan leaves stale flags wherever the ground moved
        // without anyone digging. It costs 1 ms of the 46, which is not where
        // the problem was.
        self.rescan_lakes(elev, water_level);
        if full_flux {
            self.flux_max = discharge_max(&self.discharge);
            normalise_flux(&self.discharge, &mut self.flux, self.flux_max);
        } else {
            self.normalise_touched();
        }
        self.route_sig = hash_down(&self.down);
        self.dirty = false;
    }

    /// Extend `halo` with everything downstream of it, so the set is closed
    /// under drainage. Returns false if that closure is too big to be worth
    /// repairing piecemeal.
    ///
    /// Without this the halo is a square patch on a hillside, and water that
    /// leaves one side can descend around it and re-enter somewhere lower.
    /// That breaks the reasoning `accumulate_patch` rests on — an outside cell
    /// draining into the halo is then itself downstream of the edit, so its
    /// stored accumulation is no longer something to trust. Measured, that
    /// happened on roughly one dig in seven, and each one cost the full 23 ms
    /// fallback.
    ///
    /// Closing the set removes the case rather than detecting it, and pays for
    /// itself twice: with no cells draining out of the halo, the separate pass
    /// that used to carry each exit's delta down to its outlet is gone too.
    fn close_downstream(&mut self, seeds: usize) -> bool {
        /// Beyond this the patch is not local in any useful sense and the
        /// global pass is both simpler and faster.
        const MAX_CLOSURE: usize = 60_000;
        let mut k = 0usize;
        while k < self.halo.len() {
            // Follow the new pointer, and for a cell that moved, the old one
            // too: the accumulation it used to feed needs correcting as much
            // as the one it feeds now.
            let mut cur = self.down[self.halo[k] as usize];
            if k < seeds && self.was_down[k] != cur {
                let old = self.was_down[k];
                if self.slot[old as usize] == 0 {
                    self.halo.push(old);
                    self.slot[old as usize] = self.halo.len() as u32;
                    // Beyond the seeds nothing moved, so old and new agree.
                    self.was_down.push(self.down[old as usize]);
                }
            }
            loop {
                if self.slot[cur as usize] != 0 {
                    break; // already in the set; its own walk continues it
                }
                if self.halo.len() >= MAX_CLOSURE {
                    return false;
                }
                self.halo.push(cur);
                self.slot[cur as usize] = self.halo.len() as u32;
                self.was_down.push(self.down[cur as usize]);
                let nx = self.down[cur as usize];
                if nx == cur {
                    break; // an outlet
                }
                cur = nx;
            }
            k += 1;
        }
        true
    }

    /// Recompute the flow accumulation over the closed halo.
    ///
    /// Accumulation is `A[i] = 1 + sum of A[j] over j draining into i`, so the
    /// inflow a halo cell receives from *outside* the halo need not be looked
    /// up: it is whatever the old `A[i]` holds once the cell's own unit and
    /// its old inside-the-halo tributaries are taken off. Those external
    /// inflows are exactly the ones that cannot have changed — the halo is
    /// closed under drainage, so a cell draining into it is not downstream of
    /// the edit, and nothing downstream of the edit is outside it.
    ///
    /// Returns false only if the pointers inside the halo somehow form a
    /// cycle, which strict-descent D8 should make impossible.
    fn accumulate_patch(&mut self) -> bool {
        let n = self.halo.len();
        let a_old: Vec<f32> = self
            .halo
            .iter()
            .map(|&c| self.discharge[c as usize])
            .collect();

        // Inflow from outside the halo, by subtraction.
        let mut a_new: Vec<f32> = a_old.clone();
        for k in 0..n {
            let j = self.halo[k];
            let o = self.was_down[k];
            if o == j {
                continue;
            }
            let s = self.slot[o as usize];
            if s != 0 {
                a_new[s as usize - 1] -= a_old[k];
            }
        }
        // `a_new` now holds 1 + external inflow, give or take float slack.
        for a in a_new.iter_mut() {
            *a = a.max(1.0);
        }

        // Kahn over the halo, under the NEW pointers.
        let mut indeg = vec![0u32; n];
        for k in 0..n {
            let j = self.halo[k];
            let d = self.down[j as usize];
            if d == j {
                continue;
            }
            let s = self.slot[d as usize];
            if s != 0 {
                indeg[s as usize - 1] += 1;
            }
        }
        let mut order: Vec<u32> = (0..n as u32).filter(|&k| indeg[k as usize] == 0).collect();
        let mut head = 0usize;
        while head < order.len() {
            let k = order[head] as usize;
            head += 1;
            let j = self.halo[k];
            let d = self.down[j as usize];
            if d == j {
                continue;
            }
            let s = self.slot[d as usize];
            if s == 0 {
                // Closure means this cannot happen; if it somehow does, the
                // global pass is the honest answer.
                return false;
            }
            let ks = s as usize - 1;
            a_new[ks] += a_new[k];
            indeg[ks] -= 1;
            if indeg[ks] == 0 {
                order.push(ks as u32);
            }
        }
        if order.len() != n {
            return false; // a cycle; let the global pass sort it out
        }

        self.touched.clear();
        for k in 0..n {
            let j = self.halo[k] as usize;
            self.discharge[j] = a_new[k];
            self.touched.push(j as u32);
        }
        true
    }

    /// Flux is `(discharge / max) ^ 0.28`, so every cell's value depends on a
    /// single drum-wide maximum and a change to it invalidates the whole
    /// field. Finding the maximum is a linear scan, but raising 1.57M cells to
    /// a fractional power is 6 ms — the expensive half — so the scan happens
    /// every time and the rewrite only when it has to.
    ///
    /// "Has to" is a relative threshold rather than equality. The maximum sits
    /// at the mouth of the largest river and a dig moves it by a few cells out
    /// of hundreds of thousands; insisting on exactness would repay the 6 ms
    /// for a difference of one part in 10^5. At the threshold below the error
    /// in flux is bounded by `1.002^0.28 - 1`, under 0.06%, and it cannot
    /// accumulate: the held maximum is only ever replaced by a true one.
    fn normalise_touched(&mut self) {
        const MAX_DRIFT: f32 = 0.002;
        let fmax = discharge_max(&self.discharge);
        if (fmax - self.flux_max).abs() > self.flux_max * MAX_DRIFT {
            self.flux_max = fmax;
            normalise_flux(&self.discharge, &mut self.flux, fmax);
            return;
        }
        for &c in &self.touched {
            let i = c as usize;
            self.flux[i] = (self.discharge[i] / self.flux_max).powf(0.28);
        }
    }

    /// Bring the upstream CSR back in step with `down`.
    ///
    /// Only the catchment overlay reads it, and that is debounced behind a
    /// cooldown, so the 5.5 ms is paid there rather than on every dig.
    pub fn refresh_upstream(&mut self) {
        if !self.up_stale {
            return;
        }
        rebuild_upstream(&self.down, &mut self.up_off, &mut self.up_idx);
        self.up_stale = false;
    }

    /// Lake flags and their reported statistics, over the whole field.
    fn rescan_lakes(&mut self, elev: &[f32], water_level: f32) {
        let mut lakes = 0u32;
        let mut fill_sum = 0.0f32;
        for i in 0..elev.len() {
            let depth = self.filled[i] - elev[i];
            let is_lake = depth > 0.35 && elev[i] > water_level;
            self.lake[i] = u8::from(is_lake);
            if is_lake {
                lakes += 1;
                fill_sum += depth;
            }
        }
        self.lake_count = lakes;
        self.mean_fill_depth = if lakes > 0 {
            fill_sum / lakes as f32
        } else {
            0.0
        };
    }

    fn finish_routing(&mut self, elev: &[f32], water_level: f32) {
        compute_d8(&self.filled, &mut self.down);
        accumulate(&self.down, &mut self.discharge);
        rebuild_upstream(&self.down, &mut self.up_off, &mut self.up_idx);
        self.up_stale = false;
        self.rescan_lakes(elev, water_level);
        self.flux_max = discharge_max(&self.discharge);
        normalise_flux(&self.discharge, &mut self.flux, self.flux_max);
        self.route_sig = hash_down(&self.down);
        self.dirty = false;
    }

    /// How many downstream pointers differ from `prev` (reroute metric).
    pub fn down_diff_count(&self, prev: &[u32]) -> u32 {
        if prev.len() != self.down.len() {
            return self.down.len() as u32;
        }
        let mut n = 0u32;
        for (a, b) in self.down.iter().zip(prev.iter()) {
            if a != b {
                n += 1;
            }
        }
        n
    }

    pub fn sample_flux(&self, t: f32, z: f32) -> f32 {
        sample_field(&self.flux, t, z)
    }

    pub fn sample_lake(&self, t: f32, z: f32) -> bool {
        let ti = (t.rem_euclid(NT as f32).round() as usize) % NT;
        let zi = z.round().clamp(0.0, (NZ - 1) as f32) as usize;
        self.lake[idx(ti, zi)] != 0
    }

    pub fn catchment_mask(&self, ti: usize, zi: usize) -> Vec<u8> {
        let target = idx(ti % NT, zi.min(NZ - 1)) as u32;
        let n = NT * NZ;
        let mut memo = vec![0u8; n];
        let mut out = vec![0u8; n];
        for i in 0..n {
            if walk_reaches(&self.down, i as u32, target, &mut memo) {
                out[i] = 1;
            }
        }
        out
    }

    /// Local catchment sample for HUD: cells within `radius` that drain to
    /// `(ti,zi)`, returned as grid indices. Avoids a full-grid walk.
    pub fn catchment_local(
        &self,
        ti: usize,
        zi: usize,
        radius: i32,
        limit: usize,
    ) -> Vec<(usize, usize)> {
        if !self.up_idx.is_empty() {
            return self.catchment_upstream(ti, zi, limit);
        }
        let target = idx(ti % NT, zi.min(NZ - 1)) as u32;
        let r = radius.max(8);
        let mut out = Vec::with_capacity(limit.min(512));
        let mut memo = vec![0u8; NT * NZ];
        let step = (r / 24).max(1) as usize;
        for dz in (-r..=r).step_by(step) {
            let zz = zi as i32 + dz;
            if zz < 0 || zz >= NZ as i32 {
                continue;
            }
            for dt in (-r..=r).step_by(step) {
                let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                let i = idx(tt, zz as usize);
                if walk_reaches(&self.down, i as u32, target, &mut memo) {
                    out.push((tt, zz as usize));
                    if out.len() >= limit {
                        return out;
                    }
                }
            }
        }
        out
    }

    /// BFS upstream via CSR graph — O(basin), not O(grid).
    pub fn catchment_upstream(&self, ti: usize, zi: usize, limit: usize) -> Vec<(usize, usize)> {
        let target = idx(ti % NT, zi.min(NZ - 1));
        let lim = limit.max(8).min(4096);
        let mut out = Vec::with_capacity(lim);
        let mut seen = vec![false; NT * NZ];
        let mut stack = vec![target as u32];
        seen[target] = true;
        while let Some(cur) = stack.pop() {
            let i = cur as usize;
            out.push((i % NT, i / NT));
            if out.len() >= lim {
                break;
            }
            if i + 1 >= self.up_off.len() {
                continue;
            }
            let a = self.up_off[i] as usize;
            let b = self.up_off[i + 1] as usize;
            for &u in &self.up_idx[a..b.min(self.up_idx.len())] {
                let ui = u as usize;
                if ui >= seen.len() || seen[ui] {
                    continue;
                }
                seen[ui] = true;
                stack.push(u);
            }
        }
        out
    }
}

fn rebuild_upstream(down: &[u32], up_off: &mut Vec<u32>, up_idx: &mut Vec<u32>) {
    let n = down.len();
    let mut counts = vec![0u32; n];
    for i in 0..n {
        let d = down[i] as usize;
        if d != i && d < n {
            counts[d] += 1;
        }
    }
    up_off.clear();
    up_off.resize(n + 1, 0);
    for i in 0..n {
        up_off[i + 1] = up_off[i] + counts[i];
    }
    up_idx.clear();
    up_idx.resize(up_off[n] as usize, 0);
    let mut cursor = up_off.clone();
    for i in 0..n {
        let d = down[i] as usize;
        if d != i && d < n {
            let slot = cursor[d] as usize;
            if slot < up_idx.len() {
                up_idx[slot] = i as u32;
            }
            cursor[d] += 1;
        }
    }
}

fn hash_down(down: &[u32]) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for &d in down.iter().step_by(17) {
        h ^= d as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn walk_reaches(down: &[u32], start: u32, target: u32, memo: &mut [u8]) -> bool {
    let mut path = Vec::with_capacity(64);
    let mut cur = start;
    loop {
        let i = cur as usize;
        if i >= down.len() {
            return false;
        }
        match memo[i] {
            1 => {
                for &p in &path {
                    memo[p as usize] = 1;
                }
                return true;
            }
            2 => {
                for &p in &path {
                    memo[p as usize] = 2;
                }
                return false;
            }
            _ => {}
        }
        if cur == target {
            for &p in &path {
                memo[p as usize] = 1;
            }
            memo[i] = 1;
            return true;
        }
        path.push(cur);
        let nxt = down[i];
        if nxt == cur || path.len() > NT + NZ {
            for &p in &path {
                memo[p as usize] = 2;
            }
            return false;
        }
        cur = nxt;
    }
}

#[derive(Copy, Clone)]
struct Node {
    elev: f32,
    i: u32,
}
impl Eq for Node {}
impl PartialEq for Node {
    fn eq(&self, o: &Self) -> bool {
        self.elev == o.elev && self.i == o.i
    }
}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.elev
            .partial_cmp(&self.elev)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.i.cmp(&o.i))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// Max ponding depth above true surface (metres). Caps absurd sealed basins.
const MAX_POND: f32 = 12.0;

fn priority_flood_waterline(elev: &[f32], water_level: f32, filled: &mut [f32]) {
    let n = elev.len();
    filled.copy_from_slice(elev);
    let mut closed = vec![false; n];
    let mut open = BinaryHeap::with_capacity(NT * 8);

    // Outlets: every cell at/below the engineered waterline drains into the
    // habitat water budget. Also seed a sparse set of the absolute lowest
    // cells in case the waterline is sparse.
    for i in 0..n {
        if elev[i] <= water_level + 0.5 {
            open.push(Node {
                elev: elev[i],
                i: i as u32,
            });
            closed[i] = true;
        }
    }
    // Guarantee at least some outlets: lowest 0.5% of cells.
    if open.len() < NT {
        let mut order: Vec<u32> = (0..n as u32).collect();
        order.sort_by(|&a, &b| {
            elev[a as usize]
                .partial_cmp(&elev[b as usize])
                .unwrap_or(Ordering::Equal)
        });
        let take = (n / 200).max(NT);
        for &i in order.iter().take(take) {
            if !closed[i as usize] {
                open.push(Node {
                    elev: elev[i as usize],
                    i,
                });
                closed[i as usize] = true;
            }
        }
    }

    while let Some(Node { elev: e, i }) = open.pop() {
        let (t, z) = (i as usize % NT, i as usize / NT);
        for &(dt, dz) in &N8 {
            let nt = (t as i32 + dt).rem_euclid(NT as i32) as usize;
            let nz = z as i32 + dz;
            if nz < 0 || nz >= NZ as i32 {
                continue;
            }
            let ni = idx(nt, nz as usize);
            if closed[ni] {
                continue;
            }
            closed[ni] = true;
            let fe = elev[ni].max(e).min(elev[ni] + MAX_POND);
            filled[ni] = fe;
            open.push(Node {
                elev: fe,
                i: ni as u32,
            });
        }
    }
    // Any unvisited cells (should be none) keep raw elev.
    for i in 0..n {
        if !closed[i] {
            filled[i] = elev[i];
        }
    }
}

/// Steepest-descent neighbour of one cell, or the cell itself at a pit.
///
/// Factored out so the local repair and the global pass cannot drift: a dig
/// that routed differently from a full rebuild would show up as water that
/// moved and then moved back.
fn d8_at(filled: &[f32], i: usize) -> u32 {
    let (t, z) = (i % NT, i / NT);
    let h = filled[i];
    let mut best_dh = 0.0f32;
    let mut best = i as u32;
    for &(dt, dz) in &N8 {
        let nz = z as i32 + dz;
        if nz < 0 || nz >= NZ as i32 {
            continue;
        }
        let nt = (t as i32 + dt).rem_euclid(NT as i32) as usize;
        let ni = idx(nt, nz as usize);
        let dh = h - filled[ni];
        let dist = if dt != 0 && dz != 0 {
            std::f32::consts::SQRT_2
        } else {
            1.0
        };
        let slope = dh / dist;
        if slope > best_dh {
            best_dh = slope;
            best = ni as u32;
        }
    }
    best
}

fn compute_d8(filled: &[f32], down: &mut [u32]) {
    for i in 0..down.len() {
        down[i] = d8_at(filled, i);
    }
}

fn accumulate(down: &[u32], discharge: &mut [f32]) {
    let n = down.len();
    for d in discharge.iter_mut() {
        *d = 1.0;
    }
    let mut indeg = vec![0u32; n];
    for i in 0..n {
        let d = down[i] as usize;
        if d != i {
            indeg[d] += 1;
        }
    }
    let mut queue: Vec<u32> = Vec::with_capacity(n / 8);
    for i in 0..n {
        if indeg[i] == 0 {
            queue.push(i as u32);
        }
    }
    let mut head = 0usize;
    let mut order = Vec::with_capacity(n);
    while head < queue.len() {
        let i = queue[head];
        head += 1;
        order.push(i);
        let d = down[i as usize] as usize;
        if d != i as usize {
            indeg[d] -= 1;
            if indeg[d] == 0 {
                queue.push(d as u32);
            }
        }
    }
    if order.len() < n {
        let mut seen = vec![false; n];
        for &o in &order {
            seen[o as usize] = true;
        }
        for i in 0..n {
            if !seen[i] {
                order.push(i as u32);
            }
        }
    }
    for &i in &order {
        let d = down[i as usize];
        if d != i {
            discharge[d as usize] += discharge[i as usize];
        }
    }
}

fn discharge_max(discharge: &[f32]) -> f32 {
    discharge.iter().cloned().fold(1e-6f32, f32::max)
}

fn normalise_flux(discharge: &[f32], flux: &mut [f32], fmax: f32) {
    for (f, &d) in flux.iter_mut().zip(discharge.iter()) {
        *f = (d / fmax).powf(0.28);
    }
}

fn sample_field(e: &[f32], x: f32, y: f32) -> f32 {
    let x = x.rem_euclid(NT as f32);
    let y = y.clamp(0.0, NZ as f32 - 1.001);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let x1 = (x0 + 1) % NT;
    let y1 = (y0 + 1).min(NZ - 1);
    let a = e[idx(x0, y0)];
    let b = e[idx(x1, y0)];
    let c = e[idx(x0, y1)];
    let d = e[idx(x1, y1)];
    let t = a + (b - a) * fx;
    let u = c + (d - c) * fx;
    t + (u - t) * fy
}

/// River skeleton segments: [x0,y0,z0, x1,y1,z1, width, ...].
/// Skip cells with standing pool depth above `pool_mute` or inside a lake
/// entity so ribbons don't draw through ponds.
pub fn river_segments(
    hab: &crate::habitat::Habitat,
    elev: &[f32],
    discharge: &[f32],
    down: &[u32],
    thresh_frac: f32,
    pool_depth: Option<&[f32]>,
    pool_mute: f32,
    lake_mask: Option<&[u16]>,
) -> Vec<f32> {
    let fmax = discharge.iter().cloned().fold(1e-6f32, f32::max);
    let thresh = fmax * thresh_frac;
    let mut out = Vec::new();
    for z in 1..NZ - 1 {
        for t in 0..NT {
            let i = idx(t, z);
            if let Some(lm) = lake_mask {
                if lm[i] != 0 {
                    continue;
                }
            }
            if let Some(pd) = pool_depth {
                if pd[i] >= pool_mute {
                    continue;
                }
            }
            let q = discharge[i];
            if q < thresh {
                continue;
            }
            let d = down[i] as usize;
            if d == i {
                continue;
            }
            if let Some(lm) = lake_mask {
                if lm[d] != 0 {
                    continue;
                }
            }
            if let Some(pd) = pool_depth {
                if pd[d] >= pool_mute {
                    continue;
                }
            }
            let mut lateral_max = 0.0f32;
            for &(dt, dz) in &N8 {
                let nt = (t as i32 + dt).rem_euclid(NT as i32) as usize;
                let nz = z as i32 + dz;
                if nz < 0 || nz >= NZ as i32 {
                    continue;
                }
                let ni = idx(nt, nz as usize);
                if ni == d {
                    continue;
                }
                lateral_max = lateral_max.max(discharge[ni]);
            }
            if q < lateral_max * 0.92 {
                continue;
            }

            let th0 = t as f32 / NT as f32 * std::f32::consts::TAU;
            let z0 = (z as f32 / NZ as f32 - 0.5) * hab.length;
            let th1 = (d % NT) as f32 / NT as f32 * std::f32::consts::TAU;
            let z1 = ((d / NT) as f32 / NZ as f32 - 0.5) * hab.length;
            let lift = 0.10 + 0.08 * (q / fmax).sqrt();
            let r0 = hab.radius - elev[i] - lift;
            let r1 = hab.radius - elev[d] - lift;
            let p0 = hab.to_world(th0, z0, r0);
            let p1 = hab.to_world(th1, z1, r1);
            // Only major channels — wide enough to read as rivers, not hairlines.
            let w = (2.2 + 6.5 * (q / fmax).sqrt()).min(16.0);
            out.extend_from_slice(&[p0[0], p0[1], p0[2], p1[0], p1[1], p1[2], w]);
        }
    }
    out
}

#[cfg(test)]
mod local_rebuild {
    use super::*;

    /// A ridged, tilted field with real basins, so the repair is exercised
    /// against pits and divides rather than a smooth ramp.
    pub(super) fn field() -> Vec<f32> {
        let mut e = vec![0.0f32; NT * NZ];
        for z in 0..NZ {
            for t in 0..NT {
                let a = t as f32 / NT as f32 * std::f32::consts::TAU;
                let zz = z as f32 / NZ as f32;
                e[idx(t, z)] = 70.0
                    + (a * 5.0).sin() * 30.0
                    + (zz * 19.0).cos() * 25.0
                    + (a * 23.0).sin() * (zz * 31.0).cos() * 6.0
                    + (a * 61.0).cos() * 1.5;
            }
        }
        e
    }

    pub(super) fn gouge(e: &mut [f32], t0: usize, z0: usize, r: i32, depth: f32) {
        for dz in -r..=r {
            let z = z0 as i32 + dz;
            if z < 1 || z >= NZ as i32 - 1 {
                continue;
            }
            for dt in -r..=r {
                let t = (t0 as i32 + dt).rem_euclid(NT as i32) as usize;
                let f = 1.0 - ((dt * dt + dz * dz) as f32).sqrt() / (r as f32 + 1.0);
                if f > 0.0 {
                    e[idx(t, z as usize)] -= depth * f;
                }
            }
        }
    }

    /// The whole point of the fast path: it has to agree with the slow one.
    ///
    /// The comparison is against `finish_routing` run on the *same* `filled`
    /// field, which is the actual contract. `rebuild_local` also differs from
    /// `rebuild` by skipping the priority flood — that is the older, deliberate
    /// approximation the every-eighth full rebuild exists to correct, and
    /// folding it into this test would only hide whether the routing repair
    /// itself is exact.
    ///
    /// And exact is the claim: `down` and `lake` are, and discharge is a count
    /// of upstream cells carried in an f32, so it is too until the counts grow
    /// past f32's integer range. The tolerance covers that and nothing else.
    #[test]
    fn a_local_repair_matches_a_full_rebuild() {
        let mut elev = field();
        let mut local = Flow::default();
        local.rebuild(&elev, 0.0);

        // Five separate digs, so `local_rebuilds` climbs but never trips the
        // every-eighth full fallback that would make this test vacuous.
        let sites = [
            (400usize, 300usize, 5i32),
            (401, 302, 3),
            (900, 700, 9),
            (12, 90, 4),
            (1530, 512, 6),
        ];
        for (t0, z0, r) in sites {
            gouge(&mut elev, t0, z0, r + 2, 9.0);
            local.mark_dirty_at(t0, z0, r);
            assert!(local.rebuild_local(&elev, 0.0));
        }
        assert_eq!(local.local_rebuilds, 5, "a full rebuild would hide bugs");
        assert!(
            !local.wants_full(),
            "five digs should not be asking for a flood"
        );
        assert_eq!(
            local.fallbacks, 0,
            "the local path gave up and redid the whole field, so this \
             compares the global pass against itself"
        );

        let mut full = local.clone();
        full.finish_routing(&elev, 0.0);

        let mut wrong_down = 0;
        let mut wrong_lake = 0;
        let mut worst = 0.0f32;
        let mut worst_flux = 0.0f32;
        for i in 0..NT * NZ {
            if local.down[i] != full.down[i] {
                wrong_down += 1;
            }
            if local.lake[i] != full.lake[i] {
                wrong_lake += 1;
            }
            worst = worst.max((local.discharge[i] - full.discharge[i]).abs());
            worst_flux = worst_flux.max((local.flux[i] - full.flux[i]).abs());
        }
        assert_eq!(wrong_down, 0, "{wrong_down} downstream pointers diverged");
        assert_eq!(wrong_lake, 0, "{wrong_lake} lake flags diverged");
        assert!(worst <= 1.0, "discharge diverged by {worst}");
        // Flux rides on a held drum-wide maximum; `normalise_touched` bounds
        // the resulting error to well under a tenth of a percent.
        assert!(worst_flux <= 0.001, "flux diverged by {worst_flux}");
    }

    /// The cheapest and commonest case: a dig that lowers ground without
    /// changing which way anything drains must not touch the accumulation.
    #[test]
    fn a_dig_that_reroutes_nothing_skips_the_accumulation() {
        let mut elev = vec![0.0f32; NT * NZ];
        for z in 0..NZ {
            for t in 0..NT {
                elev[idx(t, z)] = 400.0 - z as f32 * 0.3;
            }
        }
        let mut f = Flow::default();
        f.rebuild(&elev, 0.0);
        let before = f.discharge.clone();

        // Shave a wide, shallow, uniform slice: still a pure downhill slope.
        for z in 300..320 {
            for t in 500..520 {
                elev[idx(t, z)] -= 0.05;
            }
        }
        f.mark_dirty_at(510, 310, 12);
        f.rebuild_local(&elev, 0.0);
        assert!(
            f.touched.is_empty(),
            "pointers held, yet discharge was redone"
        );
        assert_eq!(before, f.discharge);
    }

    /// The upstream index is no longer maintained per dig, so anything that
    /// reads it has to ask for it. If this ever regresses, catchment overlays
    /// silently describe the terrain as it was before the player dug.
    #[test]
    fn the_upstream_index_is_stale_until_asked_for() {
        let mut elev = field();
        let mut f = Flow::default();
        f.rebuild(&elev, 0.0);
        gouge(&mut elev, 700, 400, 8, 12.0);
        f.mark_dirty_at(700, 400, 6);
        f.rebuild_local(&elev, 0.0);
        assert!(f.up_stale, "a reroute left the CSR looking current");
        f.refresh_upstream();
        assert!(!f.up_stale);

        let (mut want_off, mut want_idx) = (Vec::new(), Vec::new());
        rebuild_upstream(&f.down, &mut want_off, &mut want_idx);
        assert!(f.up_off == want_off, "CSR offsets do not match `down`");
        assert!(
            f.up_idx == want_idx,
            "CSR upstream lists do not match `down`"
        );
    }

    /// The full flood a run of local repairs eventually needs must be asked
    /// for, not taken, so the cost does not land in the middle of a swing.
    #[test]
    fn a_run_of_digs_asks_for_a_flood_rather_than_taking_one() {
        let mut elev = field();
        let mut f = Flow::default();
        f.rebuild(&elev, 0.0);
        for k in 0..SOFT_FULL {
            gouge(&mut elev, 300, 600, 4, 3.0);
            f.mark_dirty_at(300, 600, 5);
            f.rebuild_local(&elev, 0.0);
            assert_eq!(
                f.local_rebuilds,
                k + 1,
                "took a full rebuild at {k} instead of asking"
            );
        }
        assert!(f.wants_full(), "{SOFT_FULL} local repairs and no request");

        // Ignoring the request cannot be free forever.
        for _ in SOFT_FULL..HARD_FULL {
            gouge(&mut elev, 300, 600, 4, 0.5);
            f.mark_dirty_at(300, 600, 5);
            f.rebuild_local(&elev, 0.0);
        }
        f.mark_dirty_at(300, 600, 5);
        f.rebuild_local(&elev, 0.0);
        assert_eq!(f.local_rebuilds, 0, "the backstop never fired");
    }

    /// The reverse index has to be handed back clean, or the next call reads
    /// the previous call's halo as its own and repairs the wrong cells.
    #[test]
    fn the_reverse_index_is_left_clean() {
        let mut elev = field();
        let mut f = Flow::default();
        f.rebuild(&elev, 0.0);
        for k in 0..3 {
            gouge(&mut elev, 200 + k * 40, 500, 7, 8.0);
            f.mark_dirty_at(200 + k * 40, 500, 5);
            f.rebuild_local(&elev, 0.0);
            assert!(
                f.slot.iter().all(|&s| s == 0),
                "slot table dirty after rebuild {k}"
            );
        }
    }
}

#[cfg(test)]
/// Cost of a dig's reroute, against the global pass it replaced. Ignored by
/// default — it is a measurement, not an assertion, and the numbers it prints
/// are the ones quoted in `rebuild_local`'s own documentation.
#[cfg(test)]
mod timing {
    use super::*;
    use std::time::Instant;

    #[test]
    #[ignore]
    fn how_long_does_a_dig_take_to_reroute() {
        use super::local_rebuild as scene;
        let mut elev = scene::field();
        let mut f = Flow::default();
        f.rebuild(&elev, 0.0);

        // Typical: a 2.2 m brush marks a 6-cell patch.
        let mut local = std::time::Duration::ZERO;
        for k in 0..7 {
            scene::gouge(&mut elev, 600 + k * 3, 400, 3, 6.0);
            f.mark_dirty_at(600 + k * 3, 400, 6);
            let t = Instant::now();
            f.rebuild_local(&elev, 0.0);
            local += t.elapsed();
        }
        let per = local / 7;

        let mut g = f.clone();
        let t = Instant::now();
        g.finish_routing(&elev, 0.0);
        let global = t.elapsed();

        let t = Instant::now();
        f.rescan_lakes(&elev, 0.0);
        let lakes = t.elapsed();
        assert_eq!(f.fallbacks, 0, "timing is meaningless if it fell back");
        println!(
            "dig reroute: {per:?} local vs {global:?} global \
             (halo {}, touched {}, of which lake rescan {lakes:?})",
            f.halo.len(),
            f.touched.len()
        );
    }
}
