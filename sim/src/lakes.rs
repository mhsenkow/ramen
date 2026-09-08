//! Finite-volume lake entities. LANDSCAPE_200 items 43–44 / NEXT #1.
//!
//! Lake cells come from the flow fill. We cluster them into basins with a
//! spill elevation, surface area, and a volume that rises/falls with the
//! closed water budget (inflow from discharge, loss to evaporation).

use crate::terrain::{idx, NT, NZ};

#[derive(Clone, Debug)]
pub struct Lake {
    pub id: u32,
    /// Representative cell (deepest).
    pub seed: u32,
    pub theta: f32,
    pub z: f32,
    pub cells: u32,
    pub area_m2: f32,
    pub spill_elev: f32,
    pub bed_elev: f32,
    /// Water surface elevation (between bed and spill).
    pub level: f32,
    pub volume: f32,
    pub inflow: f32,
}

pub struct Lakes {
    pub lakes: Vec<Lake>,
    /// Per-cell lake id (0 = none, else 1+index).
    pub cell_lake: Vec<u16>,
}

impl Default for Lakes {
    fn default() -> Self {
        Self {
            lakes: Vec::new(),
            cell_lake: vec![0; NT * NZ],
        }
    }
}

impl Lakes {
    /// Rebuild lake entities from the flow lake mask + elevations.
    pub fn extract(
        elev: &[f32],
        filled: &[f32],
        lake: &[u8],
        discharge: &[f32],
        _hab_radius: f32,
        length: f32,
        cell_area: f32,
    ) -> Self {
        let n = NT * NZ;
        let mut cell_lake = vec![0u16; n];
        let mut lakes = Vec::new();
        let mut visited = vec![false; n];

        for z in 1..NZ - 1 {
            for t in 0..NT {
                let i = idx(t, z);
                if lake[i] == 0 || visited[i] {
                    continue;
                }
                let mut stack = vec![i];
                visited[i] = true;
                let mut cells = Vec::new();
                let mut bed = elev[i];
                let mut spill = filled[i];
                let mut deep = i;
                let mut inflow = discharge[i];
                while let Some(cur) = stack.pop() {
                    cells.push(cur);
                    if elev[cur] < bed {
                        bed = elev[cur];
                        deep = cur;
                    }
                    spill = spill.min(filled[cur]);
                    inflow = inflow.max(discharge[cur]);
                    let (ct, cz) = (cur % NT, cur / NT);
                    for &(dt, dz) in &[(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                        let nt = (ct as i32 + dt).rem_euclid(NT as i32) as usize;
                        let nz = cz as i32 + dz;
                        if nz < 0 || nz >= NZ as i32 {
                            continue;
                        }
                        let ni = idx(nt, nz as usize);
                        if visited[ni] || lake[ni] == 0 {
                            continue;
                        }
                        visited[ni] = true;
                        stack.push(ni);
                    }
                }
                if cells.len() < 4 {
                    continue;
                }
                let id = (lakes.len() + 1) as u16;
                for &c in &cells {
                    cell_lake[c] = id;
                }
                let (dt, dz) = (deep % NT, deep / NT);
                let theta = dt as f32 / NT as f32 * std::f32::consts::TAU;
                let z_m = (dz as f32 / NZ as f32 - 0.5) * length;
                // Near-spill fill so basins read as proper ponds on load.
                let level = bed + (spill - bed).max(0.1) * 0.92;
                let depth = (level - bed).max(0.1);
                let area = cells.len() as f32 * cell_area;
                lakes.push(Lake {
                    id: id as u32,
                    seed: deep as u32,
                    theta,
                    z: z_m,
                    cells: cells.len() as u32,
                    area_m2: area,
                    spill_elev: spill,
                    bed_elev: bed,
                    level,
                    volume: area * depth * 0.35,
                    inflow: inflow * 0.001,
                });
            }
        }
        lakes.sort_by(|a, b| b.cells.cmp(&a.cells));
        if lakes.len() > 128 {
            let keep: std::collections::HashSet<u32> =
                lakes.iter().take(128).map(|l| l.id).collect();
            for c in cell_lake.iter_mut() {
                if *c != 0 && !keep.contains(&(*c as u32)) {
                    *c = 0;
                }
            }
            lakes.truncate(128);
        }
        Self { lakes, cell_lake }
    }

    /// Daily balance: inflow from catchment discharge, evaporation from area.
    pub fn tick(&mut self, dt_days: f32, rain_mean: f32, temp_mean: f32) {
        let evap = (0.002 + 0.004 * (temp_mean / 25.0).clamp(0.0, 1.5)) * dt_days;
        for lake in &mut self.lakes {
            let rain_in = rain_mean * lake.area_m2 * 0.001 * dt_days;
            let flow_in = lake.inflow * dt_days * 50.0;
            lake.volume = (lake.volume + rain_in + flow_in - lake.area_m2 * evap).max(0.0);
            let depth = (lake.volume / (lake.area_m2 * 0.35 + 1.0))
                .min((lake.spill_elev - lake.bed_elev).max(0.1));
            lake.level = lake.bed_elev + depth;
            if lake.level > lake.spill_elev {
                lake.level = lake.spill_elev;
                lake.volume = lake.area_m2 * 0.35 * (lake.spill_elev - lake.bed_elev).max(0.1);
            }
        }
    }

    pub fn at_cell(&self, ti: usize, zi: usize) -> Option<&Lake> {
        let id = self.cell_lake[idx(ti % NT, zi.min(NZ - 1))] as usize;
        if id == 0 {
            return None;
        }
        self.lakes.iter().find(|l| l.id == id as u32)
    }

    /// Free-surface mesh for lake entities: flat verts + depth colors.
    /// Level comes from the entity so basins are one shared plane.
    pub fn surface_mesh(
        &self,
        hab: &crate::habitat::Habitat,
        elev: &[f32],
        max_cells: usize,
    ) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<i32>, Vec<[f32; 4]>) {
        let mut verts = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        let mut colors = Vec::new();
        if self.lakes.is_empty() {
            return (verts, normals, indices, colors);
        }

        let id_to_level: std::collections::HashMap<u16, f32> =
            self.lakes.iter().map(|l| (l.id as u16, l.level)).collect();
        let cell_t = std::f32::consts::TAU / NT as f32;
        let cell_z = hab.length / NZ as f32;
        let mut emitted = 0usize;

        for z in 1..NZ - 1 {
            for t in 0..NT {
                let i = idx(t, z);
                let lid = self.cell_lake[i];
                if lid == 0 {
                    continue;
                }
                let Some(&level) = id_to_level.get(&lid) else {
                    continue;
                };
                let depth = level - elev[i];
                // Include a shallow fringe so the shore isn't a hard grid cut.
                if depth < 0.02 {
                    continue;
                }
                if depth < 0.10 && emitted > max_cells * 9 / 10 && (t + z) % 2 != 0 {
                    continue;
                }
                let th0 = t as f32 * cell_t;
                let th1 = (t + 1) as f32 * cell_t;
                let z0 = (z as f32 / NZ as f32 - 0.5) * hab.length;
                let z1 = z0 + cell_z;
                // Lift clear of faceted terrain — too low and screen-depth shore
                // collapses; too high and lakes float. ~22 cm reads as a sheet.
                let r = hab.radius - level - 0.22;
                let p00 = hab.to_world(th0, z0, r);
                let p10 = hab.to_world(th1, z0, r);
                let p01 = hab.to_world(th0, z1, r);
                let p11 = hab.to_world(th1, z1, r);
                let base = verts.len() as i32;
                let nrm = {
                    let u = [p10[0] - p00[0], p10[1] - p00[1], p10[2] - p00[2]];
                    let v = [p01[0] - p00[0], p01[1] - p00[1], p01[2] - p00[2]];
                    let cx = u[1] * v[2] - u[2] * v[1];
                    let cy = u[2] * v[0] - u[0] * v[2];
                    let cz = u[0] * v[1] - u[1] * v[0];
                    let len = (cx * cx + cy * cy + cz * cz).sqrt().max(1e-6);
                    let toward = [-p00[0], -p00[1], 0.0];
                    let flip = if cx * toward[0] + cy * toward[1] + cz * toward[2] < 0.0 {
                        -1.0
                    } else {
                        1.0
                    };
                    [cx / len * flip, cy / len * flip, cz / len * flip]
                };
                let col = [0.22, 0.52, 0.88, depth.min(10.0)];
                for p in [p00, p10, p11, p01] {
                    verts.push(p);
                    normals.push(nrm);
                    colors.push(col);
                }
                // Clockwise when viewed from the axis (Godot front faces).
                indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
                emitted += 1;
                if emitted >= max_cells {
                    return (verts, normals, indices, colors);
                }
            }
        }
        (verts, normals, indices, colors)
    }
}
