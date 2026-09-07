//! Terrain derived from process, not stacked noise. (SIM_ARCH_BRIEF.md §9)
//!
//! Two claims this world can honestly make:
//!   1. It was ENGINEERED  - structural ribs, spoil heaps, shaped watersheds.
//!   2. It has WEATHERED   - centuries of engineered rainfall doing hydraulic
//!                           erosion on regolith.
//! Relief comes from (1); valleys and drainage come from (2), simulated with
//! droplet erosion rather than approximated with noise that looks eroded.
//!
//! Live hydrology (LANDSCAPE_200.md Wave 1): after generation — and after every
//! excavation that moves the surface — flow is recomputed from the *current*
//! elevation. Dig a trench, the river moves.
//!
//! LIMITS: droplet erosion is a toy of real fluvial geomorphology. It produces
//! plausible drainage networks, not correct ones. No isostasy, no mass balance
//! against the habitat's sediment budget. Stated, per ORRERY's rule 4.

use crate::habitat::Habitat;
use crate::edits::Edits;
use crate::flow::Flow;
use crate::noise::*;

pub const NT: usize = 1536; // samples around the drum
pub const NZ: usize = 1024; // samples along the axis

pub struct Terrain {
    pub hab: Habitat,
    /// Elevation above hull floor, metres. Indexed [t + z * NT].
    pub elev: Vec<f32>,
    /// Original (pre-edit) elevation — material strata reference.
    pub elev0: Vec<f32>,
    /// Live flow routing. `flux` is the public drainage signal.
    pub flow: Flow,
    /// Artifact tunnel segments.
    tunnels: Vec<Tunnel>,
    /// Player excavation. Never the world — only the difference from it.
    pub edits: Edits,
}

impl Tunnel {
    /// Store endpoints in WORLD space: the density field is sampled millions of
    /// times and must not do trigonometry per tunnel per sample.
    fn new(hab: &Habitat, a_cyl: [f32; 3], b_cyl: [f32; 3], rad: f32) -> Self {
        let a = hab.to_world(a_cyl[0], a_cyl[1], a_cyl[2]);
        let b = hab.to_world(b_cyl[0], b_cyl[1], b_cyl[2]);
        let mid = [(a[0]+b[0])*0.5, (a[1]+b[1])*0.5, (a[2]+b[2])*0.5];
        let half = ((b[0]-a[0]).powi(2) + (b[1]-a[1]).powi(2) + (b[2]-a[2]).powi(2)).sqrt() * 0.5;
        Self { a, b, rad, mid, reach: half + rad + 3.0 }
    }
}

#[derive(Clone, Copy)]
struct Tunnel { a: [f32; 3], b: [f32; 3], rad: f32, mid: [f32; 3], reach: f32 }

#[inline]
pub fn idx(t: usize, z: usize) -> usize { t + z * NT }

impl Terrain {
    pub fn generate(hab: Habitat) -> Self {
        let mut elev = vec![0.0f32; NT * NZ];
        let s = hab.seed;

        // ---- (1) ENGINEERED RELIEF -------------------------------------
        elev.chunks_mut(NT).enumerate().for_each(|(zi, row)| {
            let zf = zi as f32 / NZ as f32;
            let cap = {
                let d = (zf - 0.5).abs() * 2.0;
                (d.max(0.80) - 0.80) / 0.20
            };
            for (ti, cell) in row.iter_mut().enumerate() {
                let tf = ti as f32;
                let base = fbm2(tf * 0.0030, zi as f32 * 0.0026, 5, 5, s);
                let ridge = ridged2(tf * 0.0072, zi as f32 * 0.0060, 11, 5, s ^ 0xBEEF);
                let ribs = {
                    let phase = (tf / NT as f32) * std::f32::consts::TAU * 9.0;
                    let k = phase.sin().abs().powf(3.0);
                    k * (0.55 + 0.45 * fbm2(tf * 0.014, zi as f32 * 0.010, 21, 3, s ^ 0x21B))
                };
                let mut e = base * 0.40 + ridge.powf(1.30) * 0.74 + ribs * 0.20;
                e = e * hab.max_elevation * (1.0 - 0.55 * cap) + cap * hab.max_elevation * 1.30;
                *cell = e;
            }
        });

        // ---- (2) WEATHERING: droplet hydraulic erosion ------------------
        // Still runs once at generation to *shape* valleys. Live flow then
        // takes over for drainage that responds to the player.
        let mut rng = s ^ 0x1234_5678;
        let droplets = 520_000;
        for _ in 0..droplets {
            rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let mut px = ((rng >> 8) % NT as u32) as f32;
            rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let mut py = ((rng >> 8) % NZ as u32) as f32;
            let (mut vx, mut vy, mut water, mut sed) = (0.0f32, 0.0f32, 1.0f32, 0.0f32);

            for _step in 0..48 {
                let (gx, gy, h) = Self::grad(&elev, px, py);
                vx = vx * 0.55 - gx * 1.4;
                vy = vy * 0.55 - gy * 1.4;
                let len = (vx * vx + vy * vy).sqrt();
                if len < 1e-4 { break; }
                vx /= len; vy /= len;
                let (nx, ny) = (px + vx, py + vy);
                let nyc = ny.clamp(1.0, NZ as f32 - 2.0);
                let nh = Self::sample(&elev, nx, nyc);
                let dh = nh - h;

                let capacity = (-dh).max(0.0) * water * 5.5 + 0.02;
                if sed > capacity || dh > 0.0 {
                    let drop = if dh > 0.0 { sed.min(dh) } else { (sed - capacity) * 0.35 };
                    Self::deposit(&mut elev, px, py, drop);
                    sed -= drop;
                } else {
                    let take = ((capacity - sed) * 0.35).min(-dh * 0.9).max(0.0);
                    Self::deposit(&mut elev, px, py, -take);
                    sed += take;
                }
                water *= 0.985;
                px = nx.rem_euclid(NT as f32);
                py = nyc;
            }
        }

        // ---- ARTIFACT VOIDS: the tunnel graph ---------------------------
        let mut tunnels = Vec::new();
        let mut r2 = s ^ 0xA11C_E5;
        let mut rnd = || { r2 = r2.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                           ((r2 >> 8) & 0xFFFF) as f32 / 65535.0 };
        for _ in 0..70 {
            let t0 = rnd() * std::f32::consts::TAU;
            let z0 = (rnd() * 0.7 + 0.15) * hab.length - hab.length * 0.5;
            let depth0 = 12.0 + rnd() * 40.0;
            let dt = (rnd() - 0.5) * 0.55;
            let dz = (rnd() - 0.5) * 320.0;
            let a = [t0, z0, hab.radius - depth0];
            let b = [t0 + dt, z0 + dz, hab.radius - (10.0 + rnd() * 46.0)];
            tunnels.push(Tunnel::new(&hab, a, b, 3.0 + rnd() * 4.5));
            if rnd() > 0.45 {
                let m = [ (a[0]+b[0])*0.5, (a[1]+b[1])*0.5, (a[2]+b[2])*0.5 ];
                let c = [ m[0] + (rnd()-0.5)*0.30, m[1] + (rnd()-0.5)*160.0,
                          hab.radius - (8.0 + rnd()*30.0) ];
                tunnels.push(Tunnel::new(&hab, m, c, 2.2 + rnd() * 2.4));
            }
        }

        let elev0 = elev.clone();
        let mut flow = Flow::default();
        flow.rebuild(&elev, hab.water_level);

        Self { hab, elev, elev0, flow, tunnels, edits: Edits::default() }
    }

    #[inline]
    fn wrap_t(x: f32) -> f32 { x.rem_euclid(NT as f32) }

    pub fn sample(e: &[f32], x: f32, y: f32) -> f32 {
        let x = Self::wrap_t(x);
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

    fn grad(e: &[f32], x: f32, y: f32) -> (f32, f32, f32) {
        let h = Self::sample(e, x, y);
        let gx = (Self::sample(e, x + 1.0, y) - Self::sample(e, x - 1.0, y)) * 0.5;
        let gy = (Self::sample(e, x, y + 1.0) - Self::sample(e, x, y - 1.0)) * 0.5;
        (gx, gy, h)
    }

    fn deposit(e: &mut [f32], x: f32, y: f32, amt: f32) {
        let x = Self::wrap_t(x); let y = y.clamp(0.0, NZ as f32 - 1.001);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let x1 = (x0 + 1) % NT; let y1 = (y0 + 1).min(NZ - 1);
        e[idx(x0, y0)] += amt * (1.0 - fx) * (1.0 - fy);
        e[idx(x1, y0)] += amt * fx * (1.0 - fy);
        e[idx(x0, y1)] += amt * (1.0 - fx) * fy;
        e[idx(x1, y1)] += amt * fx * fy;
    }

    /// Elevation above hull floor at world angle/axial position.
    pub fn elevation(&self, theta: f32, z: f32) -> f32 {
        let t = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT as f32;
        let zz = (z / self.hab.length + 0.5) * NZ as f32;
        Self::sample(&self.elev, t, zz)
    }

    /// Pre-edit elevation — strata parent for materials.
    pub fn elevation0(&self, theta: f32, z: f32) -> f32 {
        let t = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT as f32;
        let zz = (z / self.hab.length + 0.5) * NZ as f32;
        Self::sample(&self.elev0, t, zz)
    }

    pub fn water_flux(&self, theta: f32, z: f32) -> f32 {
        let t = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT as f32;
        let zz = (z / self.hab.length + 0.5) * NZ as f32;
        self.flow.sample_flux(t, zz)
    }

    pub fn in_lake(&self, theta: f32, z: f32) -> bool {
        let t = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT as f32;
        let zz = (z / self.hab.length + 0.5) * NZ as f32;
        self.flow.sample_lake(t, zz)
    }

    /// Radius of the ground surface (ignoring caves) at this angle/axial pos.
    #[inline]
    pub fn surface_radius(&self, theta: f32, z: f32) -> f32 {
        self.hab.radius - self.elevation(theta, z)
    }

    pub fn near_tunnel(&self, p: [f32; 3], rad: f32) -> bool {
        for tn in &self.tunnels {
            let dm = [p[0] - tn.mid[0], p[1] - tn.mid[1], p[2] - tn.mid[2]];
            if dm[0]*dm[0] + dm[1]*dm[1] + dm[2]*dm[2] > (tn.reach + rad) * (tn.reach + rad) {
                continue;
            }
            if seg_dist(p, tn.a, tn.b) < tn.rad + rad {
                return true;
            }
        }
        false
    }

    /// The 3D density field. > 0 solid, < 0 air.
    pub fn density(&self, p: [f32; 3]) -> f32 {
        let (theta, z, r) = self.hab.to_cyl(p);

        let from_hull = self.hab.radius - r;
        if from_hull < 8.0 {
            return (8.0 - from_hull) * 10.0 + 1.0;
        }

        let surf = self.surface_radius(theta, z);
        let mut d = r - surf;

        if d > -8.0 {
            if d > 2.5 {
                let s = 0.020;
                let w1 = fbm3(p[0] * s, p[1] * s, p[2] * s, 4, self.hab.seed ^ 0xCAFE);
                let w2 = fbm3(p[0] * s + 31.7, p[1] * s - 12.3, p[2] * s + 5.1, 4,
                              self.hab.seed ^ 0xF00D);
                let tube = ((w1 - 0.5).abs()).max((w2 - 0.5).abs());
                let drainage = self.water_flux(theta, z);
                let thresh = 0.058 + 0.048 * drainage;
                if tube < thresh {
                    let k = 1.0 - tube / thresh;
                    let fade = ((d - 2.5) / 12.0).clamp(0.0, 1.0);
                    d -= k * k * 30.0 * fade;
                }
            }
            for tn in &self.tunnels {
                let dm = [p[0] - tn.mid[0], p[1] - tn.mid[1], p[2] - tn.mid[2]];
                if dm[0]*dm[0] + dm[1]*dm[1] + dm[2]*dm[2] > tn.reach * tn.reach { continue; }
                let dist = seg_dist(p, tn.a, tn.b);
                if dist < tn.rad + 2.0 {
                    d -= (tn.rad + 2.0 - dist) * 9.0;
                }
            }
        }

        self.edits.apply(p, d)
    }

    pub fn normal(&self, p: [f32; 3]) -> [f32; 3] {
        let h = 0.35;
        let g = [
            self.density([p[0]+h, p[1], p[2]]) - self.density([p[0]-h, p[1], p[2]]),
            self.density([p[0], p[1]+h, p[2]]) - self.density([p[0], p[1]-h, p[2]]),
            self.density([p[0], p[1], p[2]+h]) - self.density([p[0], p[1], p[2]-h]),
        ];
        let m = (g[0]*g[0] + g[1]*g[1] + g[2]*g[2]).sqrt().max(1e-6);
        [-g[0]/m, -g[1]/m, -g[2]/m]
    }

    pub fn raycast(&self, o: [f32; 3], dir: [f32; 3], max: f32)
        -> Option<([f32; 3], [f32; 3], [f32; 3])> {
        let step = 0.22;
        let mut t = 0.0f32;
        let mut prev = o;
        while t < max {
            let p = [o[0] + dir[0]*t, o[1] + dir[1]*t, o[2] + dir[2]*t];
            if self.density(p) > 0.0 {
                let (mut a, mut b) = (prev, p);
                for _ in 0..8 {
                    let m = [(a[0]+b[0])*0.5, (a[1]+b[1])*0.5, (a[2]+b[2])*0.5];
                    if self.density(m) > 0.0 { b = m; } else { a = m; }
                }
                return Some((b, self.normal(b), a));
            }
            prev = p;
            t += step;
        }
        None
    }

    /// Carve. Updates the elevation grid so drainage can reroute (items 181–182),
    /// then marks flow dirty for a deferred rebuild. Returns dig yield (mass)
    /// integrated against the density field *before* the stroke (items 801–805).
    pub fn dig(
        &mut self, p: [f32; 3], radius: f32, snap: f32, level: bool,
    ) -> Option<crate::economy::DigYield> {
        let c = Self::snap_to(p, snap);
        let up = self.hab.up_at(c);
        let mat = crate::material::material_at(self, c);
        let scale = crate::material::dig_scale(mat);
        if scale <= 0.0 {
            return None;
        }
        let r_eff = radius * scale.sqrt().max(0.45);
        let stroke = crate::edits::Stroke {
            c, radius: r_eff, dig: true, level, up,
        };
        let yield_ = crate::economy::integrate_dig_yield(self, &stroke);
        self.edits.add(c, r_eff, true, level, up);
        self.apply_elev_stroke(c, r_eff, true, level, up);
        self.mark_flow_dirty_at(c, r_eff);
        Some(yield_)
    }

    pub fn fill(&mut self, p: [f32; 3], radius: f32, snap: f32, level: bool) {
        let c = Self::snap_to(p, snap);
        let up = self.hab.up_at(c);
        self.edits.add(c, radius, false, level, up);
        self.apply_elev_stroke(c, radius, false, level, up);
        self.mark_flow_dirty_at(c, radius);
    }

    fn mark_flow_dirty_at(&mut self, c: [f32; 3], radius: f32) {
        let (theta, z, _) = self.hab.to_cyl(c);
        let ti = (theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU
            * NT as f32).round() as usize % NT;
        let zi = ((z / self.hab.length + 0.5) * NZ as f32).round()
            .clamp(0.0, (NZ - 1) as f32) as usize;
        let cell_t = std::f32::consts::TAU * self.hab.radius / NT as f32;
        let cell_z = self.hab.length / NZ as f32;
        let r = ((radius / cell_t.min(cell_z)).ceil() as i32 + 3).max(6);
        self.flow.mark_dirty_at(ti, zi, r);
    }

    /// Lower or raise the elevation grid under a brush. This is what makes
    /// digging a watershed tool rather than a cosmetic hole.
    pub fn apply_elev_stroke(
        &mut self, c: [f32; 3], radius: f32, dig: bool, level: bool, up: [f32; 3],
    ) {
        let (theta, z, _cr) = self.hab.to_cyl(c);
        let cell_t = std::f32::consts::TAU * self.hab.radius / NT as f32;
        let cell_z = self.hab.length / NZ as f32;
        let reach = if level { radius * 1.85 } else { radius * 1.15 };
        let n_t = ((reach / cell_t).ceil() as i32 + 2).max(2);
        let n_z = ((reach / cell_z).ceil() as i32 + 2).max(2);
        let t0 = (theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT as f32).round() as i32;
        let z0 = ((z / self.hab.length + 0.5) * NZ as f32).round() as i32;
        // Depth of cut into the elevation field — always meaningful for watersheds.
        let dig_depth = if dig { radius * 1.35 } else { radius * 0.55 };

        for dz in -n_z..=n_z {
            for dt in -n_t..=n_t {
                let ti = (t0 + dt).rem_euclid(NT as i32) as usize;
                let zi = z0 + dz;
                if zi < 0 || zi >= NZ as i32 { continue; }
                let zi = zi as usize;
                let th = ti as f32 / NT as f32 * std::f32::consts::TAU;
                let zz = (zi as f32 / NZ as f32 - 0.5) * self.hab.length;
                let surf_r = self.hab.radius - self.elev[idx(ti, zi)];
                let pw = self.hab.to_world(th, zz, surf_r);
                let rel = [pw[0] - c[0], pw[1] - c[1], pw[2] - c[2]];
                let h = rel[0]*up[0] + rel[1]*up[1] + rel[2]*up[2];
                let ax = [rel[0] - up[0]*h, rel[1] - up[1]*h, rel[2] - up[2]*h];
                let rad = (ax[0]*ax[0] + ax[1]*ax[1] + ax[2]*ax[2]).sqrt();
                let foot = if level { radius } else { reach };
                if rad > foot { continue; }
                let w = (1.0 - rad / foot).powf(1.15).max(0.0);

                if level {
                    if dig {
                        let cut = dig_depth * w + h.max(0.0) * w;
                        self.elev[idx(ti, zi)] =
                            (self.elev[idx(ti, zi)] - cut).max(8.5);
                    } else {
                        let raise = dig_depth * w + (-h).max(0.0) * w * 0.5;
                        self.elev[idx(ti, zi)] =
                            (self.elev[idx(ti, zi)] + raise).min(self.hab.max_elevation);
                    }
                } else if dig {
                    self.elev[idx(ti, zi)] =
                        (self.elev[idx(ti, zi)] - dig_depth * w).max(8.5);
                } else {
                    self.elev[idx(ti, zi)] =
                        (self.elev[idx(ti, zi)] + dig_depth * w).min(self.hab.max_elevation);
                }
            }
        }
    }

    /// Rebuild live flow if excavation dirtied it. Returns true if flux changed.
    /// Prefers a local (no priority-flood) rebuild when the dig patch is small.
    pub fn refresh_flow(&mut self) -> bool {
        if !self.flow.is_dirty() { return false; }
        self.flow.rebuild_local(&self.elev, self.hab.water_level);
        true
    }

    /// Force a full priority-flood rebuild (ponds / lake entities).
    pub fn refresh_flow_full(&mut self) -> bool {
        self.flow.rebuild(&self.elev, self.hab.water_level);
        true
    }

    pub fn undo_dig(&mut self) -> Option<crate::edits::Stroke> {
        let s = self.edits.pop()?;
        // Elevation undo is approximate: reverse the stroke once.
        self.apply_elev_stroke(s.c, s.radius, !s.dig, s.level, s.up);
        self.mark_flow_dirty_at(s.c, s.radius);
        Some(s)
    }

    fn snap_to(p: [f32; 3], snap: f32) -> [f32; 3] {
        if snap > 0.0 {
            [(p[0]/snap).round()*snap, (p[1]/snap).round()*snap, (p[2]/snap).round()*snap]
        } else { p }
    }
}

fn seg_dist(p: [f32; 3], a: [f32; 3], b: [f32; 3]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let ll = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    let t = if ll < 1e-6 { 0.0 } else {
        ((ap[0]*ab[0] + ap[1]*ab[1] + ap[2]*ab[2]) / ll).clamp(0.0, 1.0)
    };
    let c = [a[0] + ab[0]*t, a[1] + ab[1]*t, a[2] + ab[2]*t];
    let d = [p[0]-c[0], p[1]-c[1], p[2]-c[2]];
    (d[0]*d[0] + d[1]*d[1] + d[2]*d[2]).sqrt()
}
