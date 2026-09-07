//! Live flow routing — the landscape stops being a fossil.
//!
//! LANDSCAPE_200.md §C + LANDSCAPE_800 verified blocker:
//!   Priority-flood seeded on the endcaps fills 97% of a sealed cylinder to the
//!   rim (no ocean outlet). Outlets are the engineered waterline instead —
//!   cells at/below `water_level` are free drains into the habitat's closed
//!   water budget. Digs then genuinely reroute.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::terrain::{NT, NZ, idx};

const N8: [(i32, i32); 8] = [
    (-1,  0), (1,  0), (0, -1), (0,  1),
    (-1, -1), (-1, 1), (1, -1), (1,  1),
];

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
        }
    }
}

impl Flow {
    pub fn mark_dirty(&mut self) { self.dirty = true; }

    /// Mark a cylindrical neighbourhood dirty (player dig / fill).
    pub fn mark_dirty_at(&mut self, ti: usize, zi: usize, radius_cells: i32) {
        self.dirty = true;
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

    pub fn is_dirty(&self) -> bool { self.dirty }

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
    }

    /// Fast path after a local dig: drop ponding in the patch, recompute D8 +
    /// accumulate globally. Skips priority-flood (~most of the 130 ms cost).
    /// Periodically falls back to full rebuild.
    pub fn rebuild_local(&mut self, elev: &[f32], water_level: f32) -> bool {
        if self.dirty_t < 0 || self.local_rebuilds >= 8 || self.dirty_r > 120 {
            self.rebuild(elev, water_level);
            return true;
        }
        let t0 = self.dirty_t;
        let z0 = self.dirty_z;
        let r = self.dirty_r;
        for dz in -r..=r {
            let zi = z0 + dz;
            if zi < 0 || zi >= NZ as i32 { continue; }
            for dt in -r..=r {
                let ti = (t0 + dt).rem_euclid(NT as i32) as usize;
                let i = idx(ti, zi as usize);
                // Dig opens drainage: collapse local ponding back to the surface.
                self.filled[i] = elev[i];
            }
        }
        // Walk a short downstream ribbon and clear ponding so trenches connect.
        let mut cur = idx(
            (t0.rem_euclid(NT as i32)) as usize,
            z0.clamp(0, NZ as i32 - 1) as usize,
        ) as u32;
        for _ in 0..256 {
            let i = cur as usize;
            if i >= elev.len() { break; }
            self.filled[i] = elev[i];
            let nxt = self.down[i];
            if nxt == cur { break; }
            cur = nxt;
        }
        self.finish_routing(elev, water_level);
        self.local_rebuilds += 1;
        self.dirty_t = -1;
        self.dirty_r = 0;
        true
    }

    fn finish_routing(&mut self, elev: &[f32], water_level: f32) {
        compute_d8(&self.filled, &mut self.down);
        accumulate(&self.down, &mut self.discharge);
        rebuild_upstream(&self.down, &mut self.up_off, &mut self.up_idx);

        let mut lakes = 0u32;
        let mut fill_sum = 0.0f32;
        for i in 0..elev.len() {
            let depth = self.filled[i] - elev[i];
            let is_lake = depth > 0.35 && elev[i] > water_level;
            self.lake[i] = if is_lake { 1 } else { 0 };
            if is_lake {
                lakes += 1;
                fill_sum += depth;
            }
        }
        self.lake_count = lakes;
        self.mean_fill_depth = if lakes > 0 { fill_sum / lakes as f32 } else { 0.0 };
        normalise_flux(&self.discharge, &mut self.flux);
        self.route_sig = hash_down(&self.down);
        self.dirty = false;
    }

    /// How many downstream pointers differ from `prev` (reroute metric).
    pub fn down_diff_count(&self, prev: &[u32]) -> u32 {
        if prev.len() != self.down.len() { return self.down.len() as u32; }
        let mut n = 0u32;
        for (a, b) in self.down.iter().zip(prev.iter()) {
            if a != b { n += 1; }
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
    pub fn catchment_local(&self, ti: usize, zi: usize, radius: i32, limit: usize) -> Vec<(usize, usize)> {
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
            if zz < 0 || zz >= NZ as i32 { continue; }
            for dt in (-r..=r).step_by(step) {
                let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                let i = idx(tt, zz as usize);
                if walk_reaches(&self.down, i as u32, target, &mut memo) {
                    out.push((tt, zz as usize));
                    if out.len() >= limit { return out; }
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
            if out.len() >= lim { break; }
            if i + 1 >= self.up_off.len() { continue; }
            let a = self.up_off[i] as usize;
            let b = self.up_off[i + 1] as usize;
            for &u in &self.up_idx[a..b.min(self.up_idx.len())] {
                let ui = u as usize;
                if ui >= seen.len() || seen[ui] { continue; }
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
        if i >= down.len() { return false; }
        match memo[i] {
            1 => { for &p in &path { memo[p as usize] = 1; } return true; }
            2 => { for &p in &path { memo[p as usize] = 2; } return false; }
            _ => {}
        }
        if cur == target {
            for &p in &path { memo[p as usize] = 1; }
            memo[i] = 1;
            return true;
        }
        path.push(cur);
        let nxt = down[i];
        if nxt == cur || path.len() > NT + NZ {
            for &p in &path { memo[p as usize] = 2; }
            return false;
        }
        cur = nxt;
    }
}

#[derive(Copy, Clone)]
struct Node { elev: f32, i: u32 }
impl Eq for Node {}
impl PartialEq for Node {
    fn eq(&self, o: &Self) -> bool { self.elev == o.elev && self.i == o.i }
}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.elev.partial_cmp(&self.elev).unwrap_or(Ordering::Equal)
            .then_with(|| self.i.cmp(&o.i))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) }
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
            open.push(Node { elev: elev[i], i: i as u32 });
            closed[i] = true;
        }
    }
    // Guarantee at least some outlets: lowest 0.5% of cells.
    if open.len() < NT {
        let mut order: Vec<u32> = (0..n as u32).collect();
        order.sort_by(|&a, &b| elev[a as usize].partial_cmp(&elev[b as usize]).unwrap_or(Ordering::Equal));
        let take = (n / 200).max(NT);
        for &i in order.iter().take(take) {
            if !closed[i as usize] {
                open.push(Node { elev: elev[i as usize], i });
                closed[i as usize] = true;
            }
        }
    }

    while let Some(Node { elev: e, i }) = open.pop() {
        let (t, z) = (i as usize % NT, i as usize / NT);
        for &(dt, dz) in &N8 {
            let nt = (t as i32 + dt).rem_euclid(NT as i32) as usize;
            let nz = z as i32 + dz;
            if nz < 0 || nz >= NZ as i32 { continue; }
            let ni = idx(nt, nz as usize);
            if closed[ni] { continue; }
            closed[ni] = true;
            let fe = elev[ni].max(e).min(elev[ni] + MAX_POND);
            filled[ni] = fe;
            open.push(Node { elev: fe, i: ni as u32 });
        }
    }
    // Any unvisited cells (should be none) keep raw elev.
    for i in 0..n {
        if !closed[i] { filled[i] = elev[i]; }
    }
}

fn compute_d8(filled: &[f32], down: &mut [u32]) {
    for z in 0..NZ {
        for t in 0..NT {
            let i = idx(t, z);
            let h = filled[i];
            let mut best_dh = 0.0f32;
            let mut best = i as u32;
            for &(dt, dz) in &N8 {
                let nt = (t as i32 + dt).rem_euclid(NT as i32) as usize;
                let nz = z as i32 + dz;
                if nz < 0 || nz >= NZ as i32 { continue; }
                let ni = idx(nt, nz as usize);
                let dh = h - filled[ni];
                let dist = if dt != 0 && dz != 0 { std::f32::consts::SQRT_2 } else { 1.0 };
                let slope = dh / dist;
                if slope > best_dh {
                    best_dh = slope;
                    best = ni as u32;
                }
            }
            down[i] = best;
        }
    }
}

fn accumulate(down: &[u32], discharge: &mut [f32]) {
    let n = down.len();
    for d in discharge.iter_mut() { *d = 1.0; }
    let mut indeg = vec![0u32; n];
    for i in 0..n {
        let d = down[i] as usize;
        if d != i { indeg[d] += 1; }
    }
    let mut queue: Vec<u32> = Vec::with_capacity(n / 8);
    for i in 0..n {
        if indeg[i] == 0 { queue.push(i as u32); }
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
            if indeg[d] == 0 { queue.push(d as u32); }
        }
    }
    if order.len() < n {
        let mut seen = vec![false; n];
        for &o in &order { seen[o as usize] = true; }
        for i in 0..n {
            if !seen[i] { order.push(i as u32); }
        }
    }
    for &i in &order {
        let d = down[i as usize];
        if d != i {
            discharge[d as usize] += discharge[i as usize];
        }
    }
}

fn normalise_flux(discharge: &[f32], flux: &mut [f32]) {
    let fmax = discharge.iter().cloned().fold(1e-6f32, f32::max);
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
    let a = e[idx(x0, y0)]; let b = e[idx(x1, y0)];
    let c = e[idx(x0, y1)]; let d = e[idx(x1, y1)];
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
                if lm[i] != 0 { continue; }
            }
            if let Some(pd) = pool_depth {
                if pd[i] >= pool_mute { continue; }
            }
            let q = discharge[i];
            if q < thresh { continue; }
            let d = down[i] as usize;
            if d == i { continue; }
            if let Some(lm) = lake_mask {
                if lm[d] != 0 { continue; }
            }
            if let Some(pd) = pool_depth {
                if pd[d] >= pool_mute { continue; }
            }
            let mut lateral_max = 0.0f32;
            for &(dt, dz) in &N8 {
                let nt = (t as i32 + dt).rem_euclid(NT as i32) as usize;
                let nz = z as i32 + dz;
                if nz < 0 || nz >= NZ as i32 { continue; }
                let ni = idx(nt, nz as usize);
                if ni == d { continue; }
                lateral_max = lateral_max.max(discharge[ni]);
            }
            if q < lateral_max * 0.92 { continue; }

            let th0 = t as f32 / NT as f32 * std::f32::consts::TAU;
            let z0 = (z as f32 / NZ as f32 - 0.5) * hab.length;
            let th1 = (d % NT) as f32 / NT as f32 * std::f32::consts::TAU;
            let z1 = ((d / NT) as f32 / NZ as f32 - 0.5) * hab.length;
            let lift = 0.04 + 0.06 * (q / fmax).sqrt();
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
