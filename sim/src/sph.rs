//! Surface water columns + settling particles.
//!
//! Minecraft pooling behaviour (flat free surfaces in pits, flow to lower
//! neighbours, drain at the waterline) on the elev lattice. Mass is carried by
//! lightweight particles that settle into columns — an SPH *idea* (discrete
//! mass parcels), not a continuum SPH solve (impossible at drum scale).

use crate::terrain::{NT, NZ, idx};

const N4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const MAX_DEPTH: f32 = 28.0;
const VISIBLE: f32 = 0.18;
const MAX_PARTICLES: usize = 12_000;

#[derive(Clone, Copy)]
pub struct WaterParticle {
    pub t: f32,
    pub z: f32,
    pub mass: f32,
    pub life: f32,
}

pub struct SurfaceWater {
    /// Metres of standing water above the bed (`elev`).
    pub depth: Vec<f32>,
    pub particles: Vec<WaterParticle>,
    /// Habitat closed-budget drain accumulator (diagnostic).
    pub drained: f32,
}

impl Default for SurfaceWater {
    fn default() -> Self {
        Self {
            depth: vec![0.0; NT * NZ],
            particles: Vec::with_capacity(4096),
            drained: 0.0,
        }
    }
}

impl SurfaceWater {
    /// Seed columns from depression fill (existing lakes) so basins start wet.
    pub fn seed_from_fill(&mut self, elev: &[f32], filled: &[f32], lake: &[u8], water_level: f32) {
        debug_assert_eq!(elev.len(), NT * NZ);
        for i in 0..elev.len() {
            if elev[i] <= water_level + 0.5 {
                self.depth[i] = 0.0;
                continue;
            }
            let d = (filled[i] - elev[i]).clamp(0.0, MAX_DEPTH);
            if lake[i] != 0 || d > 0.25 {
                // Fill most of the way to spill so pools read immediately.
                self.depth[i] = (d * 0.92).max(self.depth[i]);
            }
        }
    }

    /// After a dig: flood neighbours into the new pit (Minecraft fill-in).
    /// `invent_m3` is groundwater drawn from the closed habitat ledger (kg/1000).
    /// Returns cubic metres actually invented from that budget.
    pub fn rush_into_pit(
        &mut self,
        elev: &[f32],
        ti: usize,
        zi: usize,
        radius: i32,
        invent_m3: f32,
        cell_area: f32,
    ) -> f32 {
        let r = radius.max(3);
        let mut donated = 0.0f32;
        for dz in -r..=r {
            let zz = zi as i32 + dz;
            if zz < 1 || zz >= NZ as i32 - 1 { continue; }
            for dt in -r..=r {
                let dist = (dt.abs().max(dz.abs())) as i32;
                if dist < 2 || dist > r { continue; }
                let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                let i = idx(tt, zz as usize);
                let take = self.depth[i] * 0.45;
                self.depth[i] -= take;
                donated += take;
            }
        }
        let mut beds: Vec<(usize, f32)> = Vec::new();
        for dz in -2..=2 {
            let zz = zi as i32 + dz;
            if zz < 1 || zz >= NZ as i32 - 1 { continue; }
            for dt in -2..=2 {
                let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                let i = idx(tt, zz as usize);
                beds.push((i, elev[i]));
            }
        }
        beds.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let n_take = beds.len().min(12).max(1);
        let area = cell_area.max(1e-4) * n_take as f32;
        // Convert invent budget (m³) into depth metres over the pit cells.
        let want_invent = (2.5 + 0.15 * r as f32) * cell_area.max(1e-4) * n_take as f32 * 0.35;
        let used_m3 = invent_m3.max(0.0).min(want_invent);
        let invent_depth = used_m3 / area;
        donated += invent_depth * n_take as f32;
        let per = (donated / n_take as f32).min(MAX_DEPTH);
        for &(i, _) in beds.iter().take(n_take) {
            self.depth[i] = (self.depth[i] + per).min(MAX_DEPTH);
        }
        for _ in 0..14 {
            for dz in -r..=r {
                let zz = zi as i32 + dz;
                if zz < 1 || zz >= NZ as i32 - 1 { continue; }
                for dt in -r..=r {
                    let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                    self.equalize_cell(elev, tt, zz as usize, 0.65);
                }
            }
        }
        used_m3
    }

    /// Scoop standing water near (ti,zi). Returns cubic metres removed.
    pub fn scoop_at(
        &mut self,
        elev: &[f32],
        ti: usize,
        zi: usize,
        radius: i32,
        want_m3: f32,
        cell_area: f32,
    ) -> f32 {
        let r = radius.max(1);
        let ca = cell_area.max(1e-4);
        let mut cells = Vec::new();
        for dz in -r..=r {
            let zz = zi as i32 + dz;
            if zz < 1 || zz >= NZ as i32 - 1 { continue; }
            for dt in -r..=r {
                let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                let i = idx(tt, zz as usize);
                if self.depth[i] >= VISIBLE {
                    cells.push(i);
                }
            }
        }
        if cells.is_empty() || want_m3 <= 1e-6 { return 0.0; }
        let mut left = want_m3;
        let mut taken = 0.0f32;
        // Several passes so we drain evenly rather than emptying one cell.
        for _ in 0..6 {
            if left <= 1e-6 { break; }
            let wet: Vec<usize> = cells.iter().copied().filter(|&i| self.depth[i] >= 0.05).collect();
            if wet.is_empty() { break; }
            let share = (left / (wet.len() as f32 * ca)).min(0.45);
            for i in wet {
                let take_d = share.min(self.depth[i]);
                self.depth[i] -= take_d;
                let m3 = take_d * ca;
                taken += m3;
                left -= m3;
            }
        }
        // Flatten the scooped basin.
        for _ in 0..8 {
            for dz in -r..=r {
                let zz = zi as i32 + dz;
                if zz < 1 || zz >= NZ as i32 - 1 { continue; }
                for dt in -r..=r {
                    let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                    self.equalize_cell(elev, tt, zz as usize, 0.55);
                }
            }
        }
        taken
    }

    /// Pour volume into a neighbourhood. Returns cubic metres accepted.
    pub fn pour_at(
        &mut self,
        elev: &[f32],
        ti: usize,
        zi: usize,
        radius: i32,
        add_m3: f32,
        cell_area: f32,
    ) -> f32 {
        let r = radius.max(1);
        let ca = cell_area.max(1e-4);
        if add_m3 <= 1e-6 { return 0.0; }
        let mut beds: Vec<(usize, f32)> = Vec::new();
        for dz in -r..=r {
            let zz = zi as i32 + dz;
            if zz < 1 || zz >= NZ as i32 - 1 { continue; }
            for dt in -r..=r {
                let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                let i = idx(tt, zz as usize);
                beds.push((i, elev[i]));
            }
        }
        if beds.is_empty() { return 0.0; }
        beds.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let n = beds.len().min(16).max(1);
        let per_d = (add_m3 / (ca * n as f32)).min(MAX_DEPTH);
        let mut accepted = 0.0f32;
        for &(i, _) in beds.iter().take(n) {
            let room = (MAX_DEPTH - self.depth[i]).max(0.0);
            let add = per_d.min(room);
            self.depth[i] += add;
            accepted += add * ca;
        }
        for _ in 0..12 {
            for dz in -r..=r {
                let zz = zi as i32 + dz;
                if zz < 1 || zz >= NZ as i32 - 1 { continue; }
                for dt in -r..=r {
                    let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                    self.equalize_cell(elev, tt, zz as usize, 0.60);
                }
            }
        }
        accepted
    }

    pub fn tick(
        &mut self,
        elev: &[f32],
        down: &[u32],
        rain: &[f32],
        water_level: f32,
        dt_days: f32,
        seed: &mut u32,
    ) {
        let dt = dt_days.max(0.0);
        if dt <= 0.0 { return; }

        // --- rain into columns + spawn a few particles ---
        let rain_scale = 2.8 * dt;
        for z in 0..NZ {
            for t in 0..NT {
                let i = idx(t, z);
                let r = if rain.len() == NT * NZ { rain[i] } else { 0.15 };
                self.depth[i] = (self.depth[i] + r * rain_scale).min(MAX_DEPTH);
            }
        }
        self.spawn_particles(elev, rain, seed, dt);

        // --- settle particles into columns ---
        self.integrate_particles(elev, down, dt);

        // --- Minecraft-style neighbour equalize (flat pool tops) ---
        let passes = ((6.0 * dt).ceil() as u32).clamp(4, 12);
        for _ in 0..passes {
            for z in 1..NZ - 1 {
                for t in (0..NT).step_by(2) {
                    self.equalize_cell(elev, t, z, 0.55);
                }
            }
            for z in 1..NZ - 1 {
                for t in (1..NT).step_by(2) {
                    self.equalize_cell(elev, t, z, 0.55);
                }
            }
        }

        // --- Gentle D8 downhill — keep basins pooled, only bleed overflow ---
        let mut next = self.depth.clone();
        for z in 1..NZ - 1 {
            for t in 0..NT {
                let i = idx(t, z);
                let d = self.depth[i];
                if d < 0.05 { continue; }
                let di = down[i] as usize;
                if di == i || di >= elev.len() { continue; }
                let surf = elev[i] + d;
                let nsurf = elev[di] + self.depth[di];
                // Need a real head before draining — stops flat pools from
                // collapsing into thin downhill ribbons every tick.
                if surf <= nsurf + 0.12 { continue; }
                let send = ((surf - nsurf - 0.08) * 0.18 * dt.min(1.0)).min(d * 0.28);
                next[i] -= send;
                next[di] = (next[di] + send).min(MAX_DEPTH);
            }
        }
        self.depth = next;

        // Re-flatten after channel bleed so free surfaces stay Minecraft-flat.
        for _ in 0..passes {
            for z in 1..NZ - 1 {
                for t in (0..NT).step_by(2) {
                    self.equalize_cell(elev, t, z, 0.50);
                }
            }
            for z in 1..NZ - 1 {
                for t in (1..NT).step_by(2) {
                    self.equalize_cell(elev, t, z, 0.50);
                }
            }
        }

        // --- drain at engineered waterline ---
        for i in 0..elev.len() {
            if elev[i] <= water_level + 0.4 && self.depth[i] > 0.0 {
                self.drained += self.depth[i];
                self.depth[i] = 0.0;
            }
            // Only evaporate hairline sheets — keep puddles.
            if self.depth[i] > 0.0 && self.depth[i] < 0.18 {
                self.depth[i] = (self.depth[i] - 0.008 * dt).max(0.0);
            }
        }
    }

    fn equalize_cell(&mut self, elev: &[f32], t: usize, z: usize, rate: f32) {
        let i = idx(t, z);
        let surf_i = elev[i] + self.depth[i];
        for &(dt, dz) in &N4 {
            let nt = (t as i32 + dt).rem_euclid(NT as i32) as usize;
            let nz = z as i32 + dz;
            if nz < 0 || nz >= NZ as i32 { continue; }
            let j = idx(nt, nz as usize);
            let surf_j = elev[j] + self.depth[j];
            let diff = surf_i - surf_j;
            if diff.abs() < 0.04 { continue; }
            // Transfer so free surfaces meet (Minecraft flat water).
            let xfer = diff * 0.5 * rate;
            if xfer > 0.0 {
                let send = xfer.min(self.depth[i]);
                self.depth[i] -= send;
                self.depth[j] = (self.depth[j] + send).min(MAX_DEPTH);
            } else {
                let send = (-xfer).min(self.depth[j]);
                self.depth[j] -= send;
                self.depth[i] = (self.depth[i] + send).min(MAX_DEPTH);
            }
        }
    }

    fn spawn_particles(&mut self, elev: &[f32], rain: &[f32], seed: &mut u32, dt: f32) {
        let n = ((180.0 * dt).ceil() as usize).clamp(20, 400);
        for _ in 0..n {
            if self.particles.len() >= MAX_PARTICLES { break; }
            *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let t = ((*seed >> 8) % NT as u32) as f32;
            *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let z = (((*seed >> 8) % (NZ as u32 - 2)) + 1) as f32;
            let i = idx(t as usize % NT, (z as usize).min(NZ - 1));
            let r = if rain.len() == NT * NZ { rain[i] } else { 0.2 };
            // Prefer wet / channel cells.
            *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let u = ((*seed >> 8) & 0xFFFF) as f32 / 65535.0;
            if u > r * 1.4 + 0.15 { continue; }
            let _ = elev;
            self.particles.push(WaterParticle {
                t,
                z,
                mass: 0.08 + 0.12 * r,
                life: 2.5 + 2.0 * u,
            });
        }
    }

    fn integrate_particles(&mut self, elev: &[f32], down: &[u32], dt: f32) {
        let mut i = 0;
        while i < self.particles.len() {
            let p = &mut self.particles[i];
            p.life -= dt;
            let ti = (p.t.rem_euclid(NT as f32) as usize) % NT;
            let zi = p.z.clamp(1.0, (NZ - 2) as f32) as usize;
            let cell = idx(ti, zi);
            // Follow D8 downhill.
            let dest = down[cell] as usize;
            if dest != cell && dest < elev.len() {
                let dt_ = (dest % NT) as f32 - ti as f32;
                let dz_ = (dest / NT) as f32 - zi as f32;
                p.t = (p.t + dt_.clamp(-1.0, 1.0) * 6.0 * dt).rem_euclid(NT as f32);
                p.z = (p.z + dz_.clamp(-1.0, 1.0) * 6.0 * dt).clamp(1.0, (NZ - 2) as f32);
            }
            // Deposit into column — particles become pool mass.
            let di = idx(
                (p.t as usize) % NT,
                (p.z as usize).min(NZ - 1),
            );
            let deposit = p.mass * (0.55 * dt).min(1.0);
            self.depth[di] = (self.depth[di] + deposit).min(MAX_DEPTH);
            p.mass -= deposit;

            if p.life <= 0.0 || p.mass < 0.01 {
                self.particles.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn sample_depth(&self, t: f32, z: f32) -> f32 {
        let t = t.rem_euclid(NT as f32);
        let z = z.clamp(0.0, (NZ - 1) as f32 - 0.001);
        let (t0, z0) = (t.floor() as usize, z.floor() as usize);
        let (ft, fz) = (t - t0 as f32, z - z0 as f32);
        let t1 = (t0 + 1) % NT;
        let z1 = (z0 + 1).min(NZ - 1);
        let a = self.depth[idx(t0, z0)];
        let b = self.depth[idx(t1, z0)];
        let c = self.depth[idx(t0, z1)];
        let d = self.depth[idx(t1, z1)];
        let u = a + (b - a) * ft;
        let v = c + (d - c) * ft;
        u + (v - u) * fz
    }

    /// Pool free-surface mesh: true Minecraft flat tops via connected-component
    /// leveling. Returns verts, normals, indices, and colors (a = depth metres).
    /// When `skip_lake` is set, lake-entity cells are omitted.
    pub fn pool_mesh(
        &self,
        hab: &crate::habitat::Habitat,
        elev: &[f32],
        skip_lake: Option<&[u16]>,
        max_cells: usize,
    ) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<i32>, Vec<[f32; 4]>) {
        let mut verts = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        let mut colors = Vec::new();
        let cell_t = std::f32::consts::TAU / NT as f32;
        let cell_z = hab.length / NZ as f32;

        // Mark wet candidates.
        let mut wet = vec![false; NT * NZ];
        for z in 1..NZ - 1 {
            for t in 0..NT {
                let i = idx(t, z);
                if let Some(mask) = skip_lake {
                    if mask[i] != 0 { continue; }
                }
                if self.depth[i] >= VISIBLE {
                    wet[i] = true;
                }
            }
        }

        // Connected-component flood: one shared free-surface level per basin.
        let mut comp = vec![-1i32; NT * NZ];
        let mut levels: Vec<f32> = Vec::new();
        let mut stack = Vec::new();
        for z in 1..NZ - 1 {
            for t in 0..NT {
                let seed = idx(t, z);
                if !wet[seed] || comp[seed] >= 0 { continue; }
                let cid = levels.len() as i32;
                let mut level = elev[seed] + self.depth[seed];
                stack.clear();
                stack.push(seed);
                comp[seed] = cid;
                let mut members = Vec::new();
                while let Some(cur) = stack.pop() {
                    members.push(cur);
                    level = level.max(elev[cur] + self.depth[cur]);
                    let (ct, cz) = (cur % NT, cur / NT);
                    for &(dt, dz) in &N4 {
                        let nt = (ct as i32 + dt).rem_euclid(NT as i32) as usize;
                        let nz = cz as i32 + dz;
                        if nz < 1 || nz >= NZ as i32 - 1 { continue; }
                        let ni = idx(nt, nz as usize);
                        if !wet[ni] || comp[ni] >= 0 { continue; }
                        comp[ni] = cid;
                        stack.push(ni);
                    }
                }
                // Also claim dry fringe cells whose bed sits under this level
                // so the sheet fills the whole depression (Minecraft cell fill).
                for &cur in &members {
                    let (ct, cz) = (cur % NT, cur / NT);
                    for &(dt, dz) in &N4 {
                        let nt = (ct as i32 + dt).rem_euclid(NT as i32) as usize;
                        let nz = cz as i32 + dz;
                        if nz < 1 || nz >= NZ as i32 - 1 { continue; }
                        let ni = idx(nt, nz as usize);
                        if comp[ni] >= 0 { continue; }
                        if let Some(mask) = skip_lake {
                            if mask[ni] != 0 { continue; }
                        }
                        if elev[ni] < level - 0.04 {
                            comp[ni] = cid;
                            wet[ni] = true;
                        }
                    }
                }
                levels.push(level);
            }
        }

        let mut emitted = 0usize;
        for z in 1..NZ - 1 {
            for t in 0..NT {
                let i = idx(t, z);
                let cid = comp[i];
                if cid < 0 { continue; }
                let level = levels[cid as usize];
                let depth = (level - elev[i]).max(0.0);
                if depth < 0.04 { continue; }
                // Dense emission — only thin fringe subsamples under budget pressure.
                if depth < 0.25 && emitted > max_cells * 3 / 4 && (t + z) % 2 != 0 {
                    continue;
                }

                let th0 = t as f32 * cell_t;
                let th1 = (t + 1) as f32 * cell_t;
                let z0 = (z as f32 / NZ as f32 - 0.5) * hab.length;
                let z1 = z0 + cell_z;
                let r = hab.radius - level - 0.03;
                let p00 = hab.to_world(th0, z0, r);
                let p10 = hab.to_world(th1, z0, r);
                let p01 = hab.to_world(th0, z1, r);
                let p11 = hab.to_world(th1, z1, r);
                let base = verts.len() as i32;
                let nrm = {
                    let toward = [-p00[0], -p00[1], 0.0f32];
                    let len = (toward[0]*toward[0] + toward[1]*toward[1]).sqrt().max(1e-6);
                    [toward[0] / len, toward[1] / len, 0.0]
                };
                // COLOR.a = water depth — shore softens in the shader.
                let col = [0.18, 0.42, 0.78, depth.min(8.0)];
                for p in [p00, p10, p11, p01] {
                    verts.push(p);
                    normals.push(nrm);
                    colors.push(col);
                }
                indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                emitted += 1;
                if emitted >= max_cells {
                    return (verts, normals, indices, colors);
                }
            }
        }
        (verts, normals, indices, colors)
    }

    pub fn wet_cells(&self) -> u32 {
        self.depth.iter().filter(|&&d| d >= VISIBLE).count() as u32
    }

    pub fn mean_depth(&self) -> f32 {
        let mut s = 0.0f32;
        let mut n = 0u32;
        for &d in &self.depth {
            if d >= VISIBLE {
                s += d;
                n += 1;
            }
        }
        if n == 0 { 0.0 } else { s / n as f32 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rush_fills_depression() {
        let mut elev = vec![40.0f32; NT * NZ];
        let ti = NT / 2;
        let zi = NZ / 2;
        for dz in -4i32..=4 {
            for dt in -4i32..=4 {
                let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                let zz = (zi as i32 + dz) as usize;
                let dist = ((dt * dt + dz * dz) as f32).sqrt();
                elev[idx(tt, zz)] = 40.0 - (4.0 - dist).max(0.0) * 2.5;
            }
        }
        let mut w = SurfaceWater::default();
        for dz in -8i32..=8 {
            for dt in -8i32..=8 {
                if dt.abs() <= 4 && dz.abs() <= 4 { continue; }
                let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                let zz = (zi as i32 + dz) as usize;
                w.depth[idx(tt, zz)] = 2.0;
            }
        }
        w.rush_into_pit(&elev, ti, zi, 8, 40.0, 20.0);
        let d = w.depth[idx(ti, zi)];
        assert!(d > 0.5, "bowl should pond, depth={d}");
    }
}
