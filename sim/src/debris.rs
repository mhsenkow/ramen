//! Rigid debris on the drum: fall, tumble, roll, float, come to rest.
//!
//! Hand-integrated capsules in cylindrical coordinates — not a general
//! rigid-body engine. Positions are `(theta, z, r)` with larger `r` downhill.
//! Tangential velocity is stored in m/s (not rad/s) so outward fall does not
//! silently accelerate the arc component.

use crate::economy::{self, bio_id};
use crate::habitat::Habitat;
use crate::material;
use crate::terrain::{idx, Terrain, NT, NZ};
use crate::woodscape::{self, Fall, FallVoxel, CELL};

/// Same exaggeration `player.gd` uses — true Coriolis is imperceptible here.
pub const CORIOLIS_FEEL: f32 = 3.4;
/// Habitat-day length in real seconds at 1× (sim_tick cadence ≈ 0.032 / 1.6 s).
pub const HABITAT_DAY_S: f32 = 50.0;

const SUB_DT: f32 = 1.0 / 45.0;
const MAX_SUBSTEPS: usize = 4;
const MAX_BODIES: usize = 256;
const REST_THRESHOLD_S: f32 = 2.5;
const MAX_EVENTS: usize = 64;
const ROLL_RESIST: f32 = 0.55;
const SLIDE_RESIST: f32 = 3.2;
const FLOAT_SPEED: f32 = 3.2;
const FLOAT_DRAG: f32 = 2.0;
/// Hard cap on voxels emitted to the client across all live pieces.
const MAX_LOD_VOXELS: usize = 6_000;

#[derive(Clone, Debug)]
pub struct Body {
    pub theta: f32,
    pub z: f32,
    pub r: f32,
    pub v_arc: f32,
    pub v_z: f32,
    pub v_r: f32,
    pub material: u8,
    pub mass_kg: f32,
    pub length: f32,
    pub girth: f32,
    pub spin: f32,
    pub yaw: f32,
    pub tumble: f32,
    pub afloat: bool,
    pub rest_s: f32,
    pub heat_c: f32,
    /// Local voxel offsets in the body's spawn frame (world axes at cut).
    /// Empty ⇒ render as a capsule (H-fell trunk path).
    pub voxels: Vec<FallVoxel>,
}

#[derive(Clone, Copy, Debug)]
pub struct DebrisEvent {
    pub kind: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub energy: f32,
}

pub mod event_kind {
    pub const IMPACT: u8 = 0;
    pub const SPLASH: u8 = 1;
    pub const BEACH: u8 = 2;
}

#[derive(Default)]
pub struct Debris {
    pub bodies: Vec<Body>,
    events: Vec<DebrisEvent>,
    /// Fuel cells a hot body wants lit — consumed by pyro each tick.
    pub ignite_keys: Vec<woodscape::Key>,
}

fn bulk_density(material: u8) -> f32 {
    if material >= 100 {
        economy::bio_phys(material).bulk_kg_m3
    } else {
        economy::phys(material).bulk_kg_m3
    }
}

fn bulking(material: u8) -> f32 {
    if material >= 100 {
        economy::bio_phys(material).bulking
    } else {
        economy::phys(material).bulking
    }
}

impl Debris {
    fn push_event(&mut self, ev: DebrisEvent) {
        if self.events.len() >= MAX_EVENTS {
            self.events.remove(0);
        }
        self.events.push(ev);
    }

    pub fn drain_events(&mut self) -> Vec<DebrisEvent> {
        std::mem::take(&mut self.events)
    }

    /// One place every spawn path goes through — mass accounting stays honest.
    pub fn spawn_log(
        &mut self,
        hab: &Habitat,
        theta: f32,
        z: f32,
        r: f32,
        material: u8,
        mass_kg: f32,
        length: f32,
        girth: f32,
        yaw: f32,
        v_arc: f32,
        v_z: f32,
        v_r: f32,
        tumble: f32,
        heat_c: f32,
        player_theta: f32,
        player_z: f32,
        heaps: &mut Vec<economy::Stockpile>,
    ) {
        if mass_kg < 0.05 {
            return;
        }
        while self.bodies.len() >= MAX_BODIES {
            if !self.retire_cap_victim(hab, player_theta, player_z, heaps) {
                return;
            }
        }
        self.bodies.push(Body {
            theta: theta.rem_euclid(std::f32::consts::TAU),
            z: z.clamp(-hab.length * 0.5 + 1.0, hab.length * 0.5 - 1.0),
            r,
            v_arc,
            v_z,
            v_r,
            material,
            mass_kg,
            length: length.max(0.4),
            girth: girth.max(0.15),
            spin: 0.0,
            yaw,
            tumble,
            afloat: false,
            rest_s: 0.0,
            heat_c,
            voxels: Vec::new(),
        });
    }

    fn spawn_body(
        &mut self,
        hab: &Habitat,
        theta: f32,
        z: f32,
        r: f32,
        material: u8,
        mass_kg: f32,
        length: f32,
        girth: f32,
        yaw: f32,
        v_arc: f32,
        v_z: f32,
        v_r: f32,
        tumble: f32,
        heat_c: f32,
        voxels: Vec<FallVoxel>,
        player_theta: f32,
        player_z: f32,
        heaps: &mut Vec<economy::Stockpile>,
    ) {
        if mass_kg < 0.05 {
            return;
        }
        while self.bodies.len() >= MAX_BODIES {
            if !self.retire_cap_victim(hab, player_theta, player_z, heaps) {
                return;
            }
        }
        self.bodies.push(Body {
            theta: theta.rem_euclid(std::f32::consts::TAU),
            z: z.clamp(-hab.length * 0.5 + 1.0, hab.length * 0.5 - 1.0),
            r,
            v_arc,
            v_z,
            v_r,
            material,
            mass_kg,
            length: length.max(0.4),
            girth: girth.max(0.15),
            spin: 0.0,
            yaw,
            tumble,
            afloat: false,
            rest_s: 0.0,
            heat_c,
            voxels,
        });
    }

    fn retire_body(&mut self, i: usize, hab: &Habitat, heaps: &mut Vec<economy::Stockpile>) {
        if i >= self.bodies.len() {
            return;
        }
        let b = self.bodies.swap_remove(i);
        let dens = bulk_density(b.material).max(1.0);
        let vol = b.mass_kg / dens;
        economy::deposit_heap(
            heaps,
            hab.radius,
            b.theta,
            b.z,
            b.material,
            b.mass_kg,
            vol * bulking(b.material),
            0.65,
        );
    }

    fn retire_cap_victim(
        &mut self,
        hab: &Habitat,
        player_theta: f32,
        player_z: f32,
        heaps: &mut Vec<economy::Stockpile>,
    ) -> bool {
        if self.bodies.is_empty() {
            return false;
        }
        let mut best = 0usize;
        let mut best_rest = -1.0f32;
        for (i, b) in self.bodies.iter().enumerate() {
            if b.rest_s > best_rest {
                best_rest = b.rest_s;
                best = i;
            }
        }
        if best_rest <= 0.05 {
            let mut best_d = -1.0f32;
            for (i, b) in self.bodies.iter().enumerate() {
                let dth = {
                    let x = (b.theta - player_theta).rem_euclid(std::f32::consts::TAU);
                    let x = x.min(std::f32::consts::TAU - x);
                    x * hab.radius
                };
                let d = dth * dth + (b.z - player_z) * (b.z - player_z);
                if d > best_d {
                    best_d = d;
                    best = i;
                }
            }
        }
        self.retire_body(best, hab, heaps);
        true
    }

    /// One rigid piece that keeps the severed voxel shape.
    pub fn spawn_from_fall(
        &mut self,
        hab: &Habitat,
        fall: &Fall,
        player_theta: f32,
        player_z: f32,
        heaps: &mut Vec<economy::Stockpile>,
    ) {
        let mass = fall.wood_kg + fall.leaf_kg;
        if mass < 0.5 {
            return;
        }
        let axis = fall.axis;
        let axis_len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        let ax = if axis_len > 1e-4 {
            [axis[0] / axis_len, axis[1] / axis_len, axis[2] / axis_len]
        } else {
            [0.0, 0.0, 1.0]
        };
        let (cth, z_c, cr) = hab.to_cyl(fall.centroid);
        let (cut_th, cut_z, _) = hab.to_cyl(fall.at);
        let d_arc_axis = {
            let x = (cth - cut_th).rem_euclid(std::f32::consts::TAU);
            let x = if x > std::f32::consts::PI {
                x - std::f32::consts::TAU
            } else {
                x
            };
            x * hab.radius
        };
        let yaw = ax[2].atan2(d_arc_axis);
        let extent = fall.extent.max(1.0);
        // Bounding capsule for contact / rolling — the eye sees the voxels.
        let girth = (mass / (650.0 * extent.max(1.0) * 0.55))
            .sqrt()
            .clamp(0.25, 2.2);
        let length = extent.clamp(0.8, 14.0).max(girth * 2.0);

        let d_arc = d_arc_axis;
        let d_z = z_c - cut_z;
        let horiz = (d_arc * d_arc + d_z * d_z).sqrt().max(0.5);
        let v_arc = (d_arc / horiz) * 1.0;
        let v_z = (d_z / horiz) * 1.0;
        let v_r = 0.9;
        let tumble = 1.1;
        let r0 = cr - 0.35;

        let voxels = if fall.voxels.is_empty() {
            let mut v = Vec::new();
            let n = ((extent / CELL).ceil() as i32).clamp(2, 16);
            for i in 0..n {
                let t = (i as f32 / (n as f32 - 1.0).max(1.0) - 0.5) * extent;
                v.push(FallVoxel {
                    lx: ax[0] * t,
                    ly: ax[1] * t,
                    lz: ax[2] * t,
                    kind: woodscape::kind::WOOD,
                });
            }
            v
        } else {
            fall.voxels.clone()
        };

        self.spawn_body(
            hab,
            cth,
            z_c,
            r0,
            bio_id::WOOD,
            mass,
            length,
            girth,
            yaw,
            v_arc,
            v_z,
            v_r,
            tumble,
            0.0,
            voxels,
            player_theta,
            player_z,
            heaps,
        );
    }

    /// Felled trunk fraction — a few logs so H-felling reads as felling.
    pub fn spawn_fell_trunk(
        &mut self,
        hab: &Habitat,
        theta: f32,
        z: f32,
        elev: f32,
        wood_kg: f32,
        player_theta: f32,
        player_z: f32,
        heaps: &mut Vec<economy::Stockpile>,
    ) {
        if wood_kg < 1.0 {
            return;
        }
        let ground_r = hab.radius - elev;
        let n = ((wood_kg / 400.0).round() as i32).clamp(1, 6) as usize;
        let mass_each = wood_kg / n as f32;
        for i in 0..n {
            let yaw = i as f32 * 0.9;
            self.spawn_log(
                hab,
                theta + (i as f32 - n as f32 * 0.5) * 0.002,
                z + (i as f32 - n as f32 * 0.5) * 1.2,
                ground_r - 0.3,
                bio_id::WOOD,
                mass_each,
                (mass_each / 80.0).clamp(1.2, 6.0),
                0.45,
                yaw,
                0.4,
                0.2,
                1.0,
                1.2,
                0.0,
                player_theta,
                player_z,
                heaps,
            );
        }
    }

    /// Frame-rate step. `dt_seconds` is wall / game seconds, not habitat days.
    pub fn step(
        &mut self,
        ter: &Terrain,
        water_depth: &[f32],
        heaps: &mut Vec<economy::Stockpile>,
        dt_seconds: f32,
        player_theta: f32,
        player_z: f32,
    ) {
        if dt_seconds <= 0.0 || self.bodies.is_empty() {
            return;
        }
        let steps = ((dt_seconds / SUB_DT).ceil() as usize).clamp(1, MAX_SUBSTEPS);
        let dt = dt_seconds / steps as f32;
        for _ in 0..steps {
            self.integrate_once(ter, water_depth, heaps, dt, player_theta, player_z);
        }
    }

    pub fn tick(
        &mut self,
        ter: &Terrain,
        water_depth: &[f32],
        heaps: &mut Vec<economy::Stockpile>,
        dt_days: f32,
        player_theta: f32,
        player_z: f32,
    ) {
        self.step(
            ter,
            water_depth,
            heaps,
            dt_days * HABITAT_DAY_S,
            player_theta,
            player_z,
        );
    }

    fn cell_of(hab: &Habitat, theta: f32, z: f32) -> usize {
        let ti = (theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT as f32)
            .round() as usize
            % NT;
        let zi = ((z / hab.length + 0.5) * NZ as f32)
            .round()
            .clamp(0.0, (NZ - 1) as f32) as usize;
        idx(ti, zi)
    }

    fn integrate_once(
        &mut self,
        ter: &Terrain,
        water_depth: &[f32],
        heaps: &mut Vec<economy::Stockpile>,
        dt: f32,
        player_theta: f32,
        player_z: f32,
    ) {
        let hab = ter.hab;
        let mut events: Vec<DebrisEvent> = Vec::new();
        let mut retire: Vec<usize> = Vec::new();

        for (bi, body) in self.bodies.iter_mut().enumerate() {
            if body.rest_s > REST_THRESHOLD_S {
                retire.push(bi);
                continue;
            }
            let g = hab.gravity_at(body.r.max(1.0));
            let ground_r = hab.radius - ter.elevation(body.theta, body.z);
            let ci = Self::cell_of(&hab, body.theta, body.z);
            let water_m = water_depth.get(ci).copied().unwrap_or(0.0);
            let surface_r = ground_r - water_m;
            let dens = bulk_density(body.material);
            let draft = body.girth * (dens / 1000.0).min(1.0);
            let was_afloat = body.afloat;

            if water_m > draft * 0.55 && dens < 950.0 {
                if !body.afloat {
                    let p = hab.to_world(body.theta, body.z, body.r);
                    events.push(DebrisEvent {
                        kind: event_kind::SPLASH,
                        x: p[0],
                        y: p[1],
                        z: p[2],
                        energy: body.v_r.abs().max(0.4),
                    });
                }
                body.afloat = true;
            } else if body.afloat && water_m < draft * 0.35 {
                body.afloat = false;
                let p = hab.to_world(body.theta, body.z, body.r);
                events.push(DebrisEvent {
                    kind: event_kind::BEACH,
                    x: p[0],
                    y: p[1],
                    z: p[2],
                    energy: 0.5,
                });
            }
            let _ = was_afloat;

            if body.afloat {
                body.r = surface_r + draft * 0.5;
                body.v_r = 0.0;
                let dn = ter.flow.down[ci] as usize;
                if dn != ci {
                    let (dti, dzi) = (dn % NT, dn / NT);
                    let dth = dti as f32 / NT as f32 * std::f32::consts::TAU;
                    let dz = (dzi as f32 / NZ as f32 - 0.5) * hab.length;
                    let aim_arc = {
                        let x = (dth - body.theta).rem_euclid(std::f32::consts::TAU);
                        let x = if x > std::f32::consts::PI {
                            x - std::f32::consts::TAU
                        } else {
                            x
                        };
                        x * hab.radius
                    };
                    let aim_z = dz - body.z;
                    let len = (aim_arc * aim_arc + aim_z * aim_z).sqrt().max(1e-3);
                    let speed_target =
                        FLOAT_SPEED * (0.25 + 0.75 * ter.flow.flux[ci].clamp(0.0, 1.0));
                    let tx = aim_arc / len * speed_target;
                    let tz = aim_z / len * speed_target;
                    let a = (FLOAT_DRAG * dt).clamp(0.0, 1.0);
                    body.v_arc += (tx - body.v_arc) * a;
                    body.v_z += (tz - body.v_z) * a;
                }
                body.theta += body.v_arc * dt / body.r.max(1.0);
                body.z += body.v_z * dt;
                body.spin += body.tumble * dt;
                body.tumble *= (1.0 - 0.4 * dt).max(0.0);
                let flux = ter.flow.flux[ci];
                let speed = (body.v_arc * body.v_arc + body.v_z * body.v_z).sqrt();
                if flux < 0.04 && speed < 0.25 {
                    body.rest_s += dt;
                } else {
                    body.rest_s = 0.0;
                }
            } else if body.r < ground_r - 0.05 {
                // Airborne. Air drag omitted on purpose at these speeds/scales.
                body.v_r += g * dt;
                body.v_arc += -2.0 * hab.omega * body.v_r * CORIOLIS_FEEL * dt;
                body.theta += body.v_arc * dt / body.r.max(1.0);
                body.z += body.v_z * dt;
                body.r += body.v_r * dt;
                body.spin += body.tumble * dt;
                if body.r >= ground_r {
                    let impact = body.v_r.abs();
                    body.r = ground_r;
                    let rest = if body.material == bio_id::WOOD {
                        0.18
                    } else {
                        0.05
                    };
                    body.v_r = -body.v_r * rest;
                    if body.v_r.abs() < 0.4 {
                        body.v_r = 0.0;
                    }
                    body.tumble *= 0.35;
                    let p = hab.to_world(body.theta, body.z, body.r);
                    events.push(DebrisEvent {
                        kind: event_kind::IMPACT,
                        x: p[0],
                        y: p[1],
                        z: p[2],
                        energy: impact,
                    });
                }
                body.rest_s = 0.0;
            } else {
                body.r = ground_r;
                body.v_r = 0.0;
                let d = 3.0;
                let r_hab = hab.radius.max(1.0);
                let d_e_arc = (ter.elevation(body.theta + d / r_hab, body.z)
                    - ter.elevation(body.theta - d / r_hab, body.z))
                    / (2.0 * d);
                let d_e_z = (ter.elevation(body.theta, body.z + d)
                    - ter.elevation(body.theta, body.z - d))
                    / (2.0 * d);
                let slope = (d_e_arc * d_e_arc + d_e_z * d_e_z).sqrt();
                body.v_arc += -g * d_e_arc * dt;
                body.v_z += -g * d_e_z * dt;

                let cy = body.yaw.cos();
                let sy = body.yaw.sin();
                let along = body.v_arc * cy + body.v_z * sy;
                let across = -body.v_arc * sy + body.v_z * cy;
                let along = along * (1.0 - SLIDE_RESIST * dt).max(0.0);
                let across = across * (1.0 - ROLL_RESIST * dt).max(0.0);
                body.v_arc = along * cy - across * sy;
                body.v_z = along * sy + across * cy;
                let half_g = (body.girth * 0.5).max(0.05);
                body.spin += (across / half_g) * dt;

                let mat_for_repose = if body.material >= 100 {
                    material::id::REGOLITH
                } else {
                    body.material
                };
                let repose = material::repose_tan(mat_for_repose);
                let speed = (body.v_arc * body.v_arc + body.v_z * body.v_z).sqrt();
                if slope < repose * 0.45 && speed < 0.35 {
                    body.rest_s += dt;
                } else {
                    body.rest_s = 0.0;
                    if slope > repose && speed < 0.15 {
                        let inv = slope.max(1e-4);
                        body.v_arc += -d_e_arc / inv * 0.4 * dt;
                        body.v_z += -d_e_z / inv * 0.4 * dt;
                    }
                }

                body.theta += body.v_arc * dt / body.r.max(1.0);
                body.z += body.v_z * dt;
                body.r = hab.radius - ter.elevation(body.theta, body.z);
            }

            body.theta = body.theta.rem_euclid(std::f32::consts::TAU);
            body.z = body.z.clamp(-hab.length * 0.5 + 0.5, hab.length * 0.5 - 0.5);

            if body.rest_s > REST_THRESHOLD_S {
                retire.push(bi);
            }
        }

        for ev in events {
            self.push_event(ev);
        }
        retire.sort_unstable();
        retire.dedup();
        for &i in retire.iter().rev() {
            self.retire_body(i, &hab, heaps);
        }
        while self.bodies.len() > MAX_BODIES {
            if !self.retire_cap_victim(&hab, player_theta, player_z, heaps) {
                break;
            }
        }
    }

    /// Flat LOD of debris voxels (and capsule fallbacks).
    /// Stride 5: `[x, y, z, kind, heat01]` — kind 1 wood, 2 leaf, 0 capsule proxy.
    pub fn lod_near(&self, hab: &Habitat, center: [f32; 3], radius: f32, limit: usize) -> Vec<f32> {
        let r2 = radius * radius;
        let mut out = Vec::with_capacity(4096);
        let mut n = 0usize;
        let limit = limit.max(1).min(MAX_LOD_VOXELS);
        for b in &self.bodies {
            if n >= limit {
                break;
            }
            let p = hab.to_world(b.theta, b.z, b.r);
            let d0 = p[0] - center[0];
            let d1 = p[1] - center[1];
            let d2 = p[2] - center[2];
            if d0 * d0 + d1 * d1 + d2 * d2 > r2 {
                continue;
            }
            let heat01 = (b.heat_c / 800.0).clamp(0.0, 1.0);
            if b.voxels.is_empty() {
                out.extend_from_slice(&[p[0], p[1], p[2], 0.0, heat01]);
                n += 1;
                continue;
            }
            let up = hab.up_at(p);
            let axis_hint = [0.0, 0.0, 1.0];
            let right = [
                up[1] * axis_hint[2] - up[2] * axis_hint[1],
                up[2] * axis_hint[0] - up[0] * axis_hint[2],
                up[0] * axis_hint[1] - up[1] * axis_hint[0],
            ];
            let rl = (right[0] * right[0] + right[1] * right[1] + right[2] * right[2]).sqrt();
            let right = if rl > 1e-5 {
                [right[0] / rl, right[1] / rl, right[2] / rl]
            } else {
                [1.0, 0.0, 0.0]
            };
            let fwd = [
                up[1] * right[2] - up[2] * right[1],
                up[2] * right[0] - up[0] * right[2],
                up[0] * right[1] - up[1] * right[0],
            ];
            let cy = b.yaw.cos();
            let sy = b.yaw.sin();
            let ax = [
                right[0] * cy + fwd[0] * sy,
                right[1] * cy + fwd[1] * sy,
                right[2] * cy + fwd[2] * sy,
            ];
            let (s, c) = (b.spin.sin(), b.spin.cos());
            for v in &b.voxels {
                if n >= limit {
                    break;
                }
                let lx = v.lx;
                let ly = v.ly;
                let lz = v.lz;
                let dot = ax[0] * lx + ax[1] * ly + ax[2] * lz;
                let cxv = [
                    ax[1] * lz - ax[2] * ly,
                    ax[2] * lx - ax[0] * lz,
                    ax[0] * ly - ax[1] * lx,
                ];
                let rx = lx * c + cxv[0] * s + ax[0] * dot * (1.0 - c);
                let ry = ly * c + cxv[1] * s + ax[1] * dot * (1.0 - c);
                let rz = lz * c + cxv[2] * s + ax[2] * dot * (1.0 - c);
                out.extend_from_slice(&[p[0] + rx, p[1] + ry, p[2] + rz, v.kind as f32, heat01]);
                n += 1;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::Terrain;

    fn tiny_world() -> (Terrain, Vec<f32>) {
        let hab = Habitat::kepler_drum();
        let ter = Terrain::generate(hab);
        let water = vec![0.0f32; NT * NZ];
        (ter, water)
    }

    #[test]
    fn a_log_on_a_slope_ends_up_downhill() {
        let (ter, water) = tiny_world();
        let mut heaps = Vec::new();
        let mut debris = Debris::default();
        // Find a cell with meaningful slope.
        let mut th0 = 1.0f32;
        let mut z0 = 0.0f32;
        let mut best_slope = 0.0f32;
        for k in 0..400 {
            let th = (k as f32) * 0.02;
            let z = ((k % 40) as f32 - 20.0) * 30.0;
            let d = 3.0;
            let d_e_arc = (ter.elevation(th + d / ter.hab.radius, z)
                - ter.elevation(th - d / ter.hab.radius, z))
                / (2.0 * d);
            let d_e_z =
                (ter.elevation(th, z + d) - ter.elevation(th, z - d)) / (2.0 * d);
            let s = (d_e_arc * d_e_arc + d_e_z * d_e_z).sqrt();
            if s > best_slope {
                best_slope = s;
                th0 = th;
                z0 = z;
            }
        }
        assert!(best_slope > 0.05, "need a slope to test rolling");
        let elev0 = ter.elevation(th0, z0);
        let r0 = ter.hab.radius - elev0;
        debris.spawn_log(
            &ter.hab,
            th0,
            z0,
            r0,
            bio_id::WOOD,
            200.0,
            2.0,
            0.4,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            th0,
            z0,
            &mut heaps,
        );
        for _ in 0..90 {
            debris.step(&ter, &water, &mut heaps, 0.2, th0, z0);
        }
        let (th1, z1) = if let Some(b) = debris.bodies.first() {
            (b.theta, b.z)
        } else if let Some(h) = heaps.last() {
            (h.theta, h.z)
        } else {
            panic!("body vanished without a heap");
        };
        let elev1 = ter.elevation(th1, z1);
        assert!(
            elev1 < elev0 - 0.15,
            "expected downhill move: elev {elev0} -> {elev1} (slope {best_slope})"
        );
    }

    #[test]
    fn a_log_on_flat_ground_stays_put() {
        let (ter, water) = tiny_world();
        let mut heaps = Vec::new();
        let mut debris = Debris::default();
        // Meadow-ish mid-drum flats are common near z=0 in some provinces;
        // search for near-zero slope.
        let mut th0 = 0.5f32;
        let mut z0 = 100.0f32;
        let mut best = f32::MAX;
        for k in 0..500 {
            let th = (k as f32) * 0.015;
            let z = ((k % 50) as f32 - 25.0) * 40.0;
            let d = 3.0;
            let d_e_arc = (ter.elevation(th + d / ter.hab.radius, z)
                - ter.elevation(th - d / ter.hab.radius, z))
                / (2.0 * d);
            let d_e_z =
                (ter.elevation(th, z + d) - ter.elevation(th, z - d)) / (2.0 * d);
            let s = (d_e_arc * d_e_arc + d_e_z * d_e_z).sqrt();
            if s < best {
                best = s;
                th0 = th;
                z0 = z;
            }
        }
        let elev0 = ter.elevation(th0, z0);
        debris.spawn_log(
            &ter.hab,
            th0,
            z0,
            ter.hab.radius - elev0,
            bio_id::WOOD,
            120.0,
            1.5,
            0.35,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            th0,
            z0,
            &mut heaps,
        );
        for _ in 0..40 {
            debris.step(&ter, &water, &mut heaps, 0.15, th0, z0);
        }
        let (th1, z1) = debris
            .bodies
            .first()
            .map(|b| (b.theta, b.z))
            .or_else(|| heaps.last().map(|h| (h.theta, h.z)))
            .expect("body or heap");
        let dth = {
            let x = (th1 - th0).rem_euclid(std::f32::consts::TAU);
            let x = x.min(std::f32::consts::TAU - x);
            x * ter.hab.radius
        };
        let dist = (dth * dth + (z1 - z0) * (z1 - z0)).sqrt();
        assert!(
            dist < 4.0,
            "flat log drifted {dist} m (slope {best})"
        );
    }

    #[test]
    fn a_log_does_not_tunnel_through_the_ground() {
        let (ter, water) = tiny_world();
        let mut heaps = Vec::new();
        let mut debris = Debris::default();
        let th = 0.8f32;
        let z = 50.0f32;
        let ground = ter.hab.radius - ter.elevation(th, z);
        debris.spawn_log(
            &ter.hab,
            th,
            z,
            ground - 8.0,
            bio_id::WOOD,
            80.0,
            1.2,
            0.3,
            0.0,
            0.0,
            0.0,
            20.0,
            0.0,
            0.0,
            th,
            z,
            &mut heaps,
        );
        for _ in 0..120 {
            debris.step(&ter, &water, &mut heaps, SUB_DT, th, z);
        }
        let b = debris
            .bodies
            .first()
            .expect("still live or just landed");
        let ground_now = ter.hab.radius - ter.elevation(b.theta, b.z);
        assert!(
            b.r <= ground_now + 0.25,
            "tunneled: r={} ground={}",
            b.r,
            ground_now
        );
        assert!(
            b.r >= ground_now - 2.5,
            "still high above ground: r={} ground={}",
            b.r,
            ground_now
        );
    }

    #[test]
    fn mass_is_conserved_from_fall_to_heap() {
        let (ter, water) = tiny_world();
        let mut heaps = Vec::new();
        let mut debris = Debris::default();
        let th = 1.2f32;
        let z = -80.0f32;
        let mass = 350.0f32;
        debris.spawn_log(
            &ter.hab,
            th,
            z,
            ter.hab.radius - ter.elevation(th, z),
            bio_id::WOOD,
            mass,
            2.0,
            0.4,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            th,
            z,
            &mut heaps,
        );
        // Force rest.
        if let Some(b) = debris.bodies.first_mut() {
            b.rest_s = REST_THRESHOLD_S + 1.0;
            b.v_arc = 0.0;
            b.v_z = 0.0;
        }
        debris.step(&ter, &water, &mut heaps, 0.1, th, z);
        let heap_m: f32 = heaps
            .iter()
            .filter(|h| h.material_id == bio_id::WOOD)
            .map(|h| h.mass_kg)
            .sum();
        let live: f32 = debris.bodies.iter().map(|b| b.mass_kg).sum();
        let total = heap_m + live;
        let rel = (total - mass).abs() / mass;
        assert!(
            rel < 5e-4,
            "mass drift {rel}: in={mass} out={total} (f32 relative tol)"
        );
    }

    #[test]
    fn wood_floats_and_basalt_does_not() {
        let (mut ter, mut water) = tiny_world();
        // Put a pond under a known cell.
        let th = 0.3f32;
        let z = 200.0f32;
        let ti = (th.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT as f32)
            .round() as usize
            % NT;
        let zi = ((z / ter.hab.length + 0.5) * NZ as f32)
            .round()
            .clamp(0.0, (NZ - 1) as f32) as usize;
        water[idx(ti, zi)] = 2.5;
        // Flatten a bit so they don't immediately roll away.
        let i = idx(ti, zi);
        let e = ter.elev[i];
        for dt in -2..=2i32 {
            for dz in -2..=2i32 {
                let tt = (ti as i32 + dt).rem_euclid(NT as i32) as usize;
                let zz = (zi as i32 + dz).clamp(0, NZ as i32 - 1) as usize;
                ter.elev[idx(tt, zz)] = e;
            }
        }
        let mut heaps = Vec::new();
        let mut debris = Debris::default();
        let r = ter.hab.radius - e;
        debris.spawn_log(
            &ter.hab, th, z, r, bio_id::WOOD, 100.0, 1.5, 0.4, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, th,
            z, &mut heaps,
        );
        debris.spawn_log(
            &ter.hab,
            th,
            z,
            r,
            material::id::BASALT,
            100.0,
            1.0,
            0.4,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            th,
            z,
            &mut heaps,
        );
        debris.step(&ter, &water, &mut heaps, 0.5, th, z);
        let wood = debris
            .bodies
            .iter()
            .find(|b| b.material == bio_id::WOOD)
            .expect("wood");
        let rock = debris
            .bodies
            .iter()
            .find(|b| b.material == material::id::BASALT)
            .expect("basalt");
        assert!(wood.afloat, "wood should float");
        assert!(!rock.afloat, "basalt should not float");
    }

    #[test]
    fn a_body_at_rest_becomes_a_heap_and_stops_costing_anything() {
        let (ter, water) = tiny_world();
        let mut heaps = Vec::new();
        let mut debris = Debris::default();
        let th = 2.0f32;
        let z = 10.0f32;
        debris.spawn_log(
            &ter.hab,
            th,
            z,
            ter.hab.radius - ter.elevation(th, z),
            bio_id::WOOD,
            90.0,
            1.0,
            0.3,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            th,
            z,
            &mut heaps,
        );
        if let Some(b) = debris.bodies.first_mut() {
            b.rest_s = REST_THRESHOLD_S + 0.5;
        }
        debris.step(&ter, &water, &mut heaps, 0.05, th, z);
        assert!(debris.bodies.is_empty());
        assert!(!heaps.is_empty());
    }

    #[test]
    fn the_body_count_is_capped() {
        let (ter, water) = tiny_world();
        let mut heaps = Vec::new();
        let mut debris = Debris::default();
        let th = 0.1f32;
        let z = 0.0f32;
        for i in 0..(MAX_BODIES + 20) {
            debris.spawn_log(
                &ter.hab,
                th + i as f32 * 0.001,
                z,
                ter.hab.radius - 5.0,
                bio_id::WOOD,
                40.0,
                1.0,
                0.25,
                0.0,
                0.0,
                0.0,
                0.5,
                0.0,
                0.0,
                th,
                z,
                &mut heaps,
            );
        }
        assert!(debris.bodies.len() <= MAX_BODIES);
        let _ = water;
    }
}
