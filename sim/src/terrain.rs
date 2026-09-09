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

use crate::edits::Edits;
use crate::flow::Flow;
use crate::habitat::Habitat;
use crate::noise::*;

pub const NT: usize = 1536; // samples around the drum
pub const NZ: usize = 1024; // samples along the axis

/// Worldgen knobs (droplet / tunnel counts).
#[derive(Clone, Copy, Debug)]
pub struct GenOpts {
    pub droplets: u32,
    pub tunnels: u32,
}

impl Default for GenOpts {
    fn default() -> Self {
        Self {
            droplets: 520_000,
            tunnels: 70,
        }
    }
}

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
        let mid = [
            (a[0] + b[0]) * 0.5,
            (a[1] + b[1]) * 0.5,
            (a[2] + b[2]) * 0.5,
        ];
        let half =
            ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt() * 0.5;
        Self {
            a,
            b,
            rad,
            mid,
            reach: half + rad + 3.0,
        }
    }
}

#[derive(Clone, Copy)]
struct Tunnel {
    a: [f32; 3],
    b: [f32; 3],
    rad: f32,
    mid: [f32; 3],
    reach: f32,
}

/// Per-(theta, z) cache for a radial column of density samples. See
/// `Terrain::column`.
pub struct Column {
    pub theta: f32,
    pub z: f32,
    /// Radius of the ground surface here — the same for every sample in the column.
    pub surf: f32,
    /// Whether an artifact bore or a player stroke can reach this column. When
    /// the caller can prove none can — the chunk builder enumerates them — the
    /// tunnel walk and the edit lookup are skipped for every sample in it,
    /// which is a hundred-odd segment tests and a hash probe per voxel.
    pub features: bool,
    strata: Option<Strata>,
    /// Lazily filled; NaN means "not yet read".
    drainage: f32,
}

/// The strata/ledge/col offset applied to the raw geometric distance. It is a
/// function of (theta, z) alone, so one value serves the whole column.
struct Strata {
    offset: f32,
}

impl Strata {
    fn inert() -> Self {
        Self { offset: 0.0 }
    }
}

#[inline]
pub fn idx(t: usize, z: usize) -> usize {
    t + z * NT
}

fn recipe_elev(arch: u8, tf: f32, zf: f32, hab: &Habitat, s: u32, wl: f32) -> f32 {
    use crate::province::id;
    let max_e = hab.max_elevation;
    let amp = crate::province::relief_amp(arch) * max_e;
    match arch {
        id::MASSIF => {
            let ridge = ridged2(tf * 0.0065, zf * 0.0055, 11, 5, s ^ 0xBEEF);
            let ridge2 = ridged2(tf * 0.014, zf * 0.012, 17, 4, s ^ 0xBEE2);
            let e = ridge.powf(1.45) * 0.78 + ridge2.powf(1.6) * 0.22;
            e * amp
        }
        id::PLATEAU => {
            let base = fbm2(tf * 0.0028, zf * 0.0024, 5, 4, s ^ 0x51A7);
            let stepped = ((base * 4.0).floor() / 4.0) * amp * 0.85 + base * amp * 0.15;
            let canyon = ridged2(tf * 0.02, zf * 0.018, 9, 3, s ^ 0x0CA1).powf(2.0);
            (stepped - canyon * 28.0).max(wl + 4.0)
        }
        id::BADLANDS => {
            let r = ridged2(tf * 0.018, zf * 0.016, 13, 5, s ^ 0x0BAD);
            let gully = fbm2(tf * 0.04, zf * 0.035, 19, 3, s ^ 0x0BAD2);
            (r * 0.7 + gully * 0.3) * amp
        }
        id::MEADOW => {
            let base = fbm2(tf * 0.0022, zf * 0.0020, 5, 3, s ^ 0xEAD0);
            wl + 18.0 + base * amp.min(60.0)
        }
        id::SWAMP_BASIN => {
            let hum = fbm2(tf * 0.05, zf * 0.045, 23, 2, s ^ 0x5A70);
            wl + 2.0 + 4.0 * hum + hum * 1.2
        }
        id::DUNE_SEA => {
            // Anisotropic ridges along +z (wind).
            let phase = zf * 0.085 + fbm2(tf * 0.01, zf * 0.008, 7, 2, s ^ 0x0D01) * 3.0;
            let dune = (phase.sin() * 0.5 + 0.5).powf(1.35);
            // Asymmetric slip: steeper when derivative positive in z.
            let slip = (phase.cos()).abs().powf(0.7);
            let h = 12.0 + dune * 28.0 * slip + amp * 0.35 * dune;
            wl + 8.0 + h
        }
        id::SEA_BASIN => {
            let floor = 9.0 + 6.0 * fbm2(tf * 0.003, zf * 0.0025, 5, 3, s ^ 0x05EA);
            let island = ridged2(tf * 0.012, zf * 0.010, 11, 4, s ^ 0x015E).powf(1.8);
            // Residual knobs poke above waterline as islands.
            if island > 0.62 {
                wl + 8.0 + (island - 0.62) / 0.38 * 55.0
            } else {
                floor
            }
        }
        id::KARST => {
            let base = fbm2(tf * 0.003, zf * 0.0026, 5, 4, s ^ 0x0CA5) * amp * 0.7 + amp * 0.25;
            let pit = {
                // Worley-like pit from hashed cell.
                let cx = (tf * 0.04).floor();
                let cy = (zf * 0.04).floor();
                let mut dmin = 1e9f32;
                for oy in -1..=1 {
                    for ox in -1..=1 {
                        let jx = hash01_local(cx as i32 + ox, cy as i32 + oy, s ^ 0x0CA51);
                        let jy = hash01_local(cx as i32 + ox, cy as i32 + oy, s ^ 0x0CA52);
                        let px = cx + ox as f32 + jx;
                        let py = cy + oy as f32 + jy;
                        let dx = tf * 0.04 - px;
                        let dy = zf * 0.04 - py;
                        dmin = dmin.min(dx * dx + dy * dy);
                    }
                }
                (1.0 - dmin.sqrt().min(1.0)).powf(2.0)
            };
            (base - pit * 35.0).max(wl + 1.0)
        }
        id::ENDCAP_WALL => {
            let z_norm = zf / NZ as f32;
            let cap = {
                let d = (z_norm - 0.5).abs() * 2.0;
                (d.max(0.80) - 0.80) / 0.20
            };
            let ridge = ridged2(tf * 0.007, zf * 0.006, 11, 5, s ^ 0x0E1D);
            ridge.powf(1.35) * amp * (0.55 + 0.75 * cap) + cap * max_e * 0.35
        }
        id::FARMLAND => {
            // Near-flat alluvial benches slightly above waterline — field
            // terraces with subtle 8–12 m steps, not organic hills.
            let base = fbm2(tf * 0.0018, zf * 0.0016, 3, 2, s ^ 0xFA12);
            let terrace = ((base * 5.0).floor() / 5.0) * 10.0;
            let micro = fbm2(tf * 0.08, zf * 0.07, 29, 2, s ^ 0xFA13) * 1.6;
            (wl + 6.0 + terrace + micro + amp * 0.35).clamp(wl + 3.0, wl + 42.0)
        }
        id::CITY => {
            // Graded urban pads — sharper terrace quantisation than farmland,
            // with occasional canal cuts so blocks read as engineered.
            let base = fbm2(tf * 0.0024, zf * 0.0020, 5, 3, s ^ 0xC170);
            let stepped = ((base * 6.0).floor() / 6.0) * amp * 0.9 + base * amp * 0.1;
            let canal = ridged2(tf * 0.028, zf * 0.024, 9, 2, s ^ 0xC171).powf(2.4);
            (wl + 10.0 + stepped - canal * 14.0).clamp(wl + 4.0, wl + 70.0)
        }
        _ => {
            let base = fbm2(tf * 0.003, zf * 0.0026, 5, 5, s);
            base * amp
        }
    }
}

#[inline]
fn hash01_local(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = seed
        .wrapping_add((x as u32).wrapping_mul(0x9E3779B1))
        .wrapping_add((y as u32).wrapping_mul(0x85EBCA77));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A2D39);
    h ^= h >> 15;
    (h & 0x00FF_FFFF) as f32 / 16_777_215.0
}

impl Terrain {
    pub fn generate(hab: Habitat) -> Self {
        Self::generate_ex(hab, GenOpts::default())
    }

    pub fn generate_ex(hab: Habitat, opts: GenOpts) -> Self {
        let mut elev = vec![0.0f32; NT * NZ];
        let s = hab.seed;
        let wl = hab.water_level;

        // ---- (1) ENGINEERED RELIEF (province-blended recipes) -----------
        // LANDSCAPE_4200 §BN: each archetype has its own recipe; top-two blend.
        elev.chunks_mut(NT).enumerate().for_each(|(zi, row)| {
            let zf = zi as f32 / NZ as f32;
            let z_m = (zf - 0.5) * hab.length;
            for (ti, cell) in row.iter_mut().enumerate() {
                let tf = ti as f32;
                let theta = tf / NT as f32 * std::f32::consts::TAU;
                let prov = crate::province::province_at(&hab, theta, z_m);
                let e0 = recipe_elev(prov.primary, tf, zi as f32, &hab, s, wl);
                let e1 = recipe_elev(prov.secondary, tf, zi as f32, &hab, s ^ 0xB17, wl);
                let mut e = prov.blend2(e0, e1);
                // Shared structural ribs — stronger under massifs.
                let ribs = {
                    let phase = (tf / NT as f32) * std::f32::consts::TAU * 9.0;
                    let k = phase.sin().abs().powf(3.0);
                    k * (0.55 + 0.45 * fbm2(tf * 0.014, zi as f32 * 0.010, 21, 3, s ^ 0x21B))
                };
                let rib_w = 0.08
                    + 0.22 * prov.weight(crate::province::id::MASSIF)
                    + 0.12 * prov.weight(crate::province::id::ENDCAP_WALL);
                e += ribs * rib_w * hab.max_elevation;
                // Keep sea basins honestly below the waterline after blend/ribs.
                if prov.weight(crate::province::id::SEA_BASIN) > 0.55 && e > wl - 1.0 {
                    // Preserve island knobs (already above WL from recipe).
                    if e < wl + 6.0 {
                        e = (wl - 8.0 - fbm2(tf * 0.01, zi as f32 * 0.008, 5, 2, s ^ 0xF100) * 5.0)
                            .max(9.0);
                    }
                }
                *cell = e.clamp(8.5, hab.max_elevation);
            }
        });

        // ---- (2) WEATHERING: droplet hydraulic erosion ------------------
        // Still runs once at generation to *shape* valleys. Live flow then
        // takes over for drainage that responds to the player.
        let mut rng = s ^ 0x1234_5678;
        let droplets = opts.droplets;
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
                if len < 1e-4 {
                    break;
                }
                vx /= len;
                vy /= len;
                let (nx, ny) = (px + vx, py + vy);
                let nyc = ny.clamp(1.0, NZ as f32 - 2.0);
                let nh = Self::sample(&elev, nx, nyc);
                let dh = nh - h;

                let capacity = (-dh).max(0.0) * water * 5.5 + 0.02;
                if sed > capacity || dh > 0.0 {
                    let drop = if dh > 0.0 {
                        sed.min(dh)
                    } else {
                        (sed - capacity) * 0.35
                    };
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

        // Province-aware thermal settle (sand / soft slopes).
        crate::erosion::talus_relax(&mut elev, 0.72, 2);

        // Re-assert sea basin floors after hydraulic/talus (erosion fills them).
        for zi in 0..NZ {
            let z_m = (zi as f32 / NZ as f32 - 0.5) * hab.length;
            for ti in 0..NT {
                let theta = ti as f32 / NT as f32 * std::f32::consts::TAU;
                let w = crate::province::province_at(&hab, theta, z_m)
                    .weight(crate::province::id::SEA_BASIN);
                if w > 0.55 {
                    let i = idx(ti, zi);
                    if elev[i] < wl + 5.0 {
                        elev[i] = elev[i].min(wl - 4.0).max(8.5);
                    }
                }
            }
        }

        // ---- ARTIFACT VOIDS: the tunnel graph ---------------------------
        let mut tunnels = Vec::new();
        let mut r2 = s ^ 0xA11C_E5;
        let mut rnd = || {
            r2 = r2.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((r2 >> 8) & 0xFFFF) as f32 / 65535.0
        };
        for _ in 0..opts.tunnels {
            let t0 = rnd() * std::f32::consts::TAU;
            let z0 = (rnd() * 0.7 + 0.15) * hab.length - hab.length * 0.5;
            let depth0 = 12.0 + rnd() * 40.0;
            let dt = (rnd() - 0.5) * 0.55;
            let dz = (rnd() - 0.5) * (hab.length * 0.053).clamp(80.0, 320.0);
            let a = [t0, z0, hab.radius - depth0];
            let b = [t0 + dt, z0 + dz, hab.radius - (10.0 + rnd() * 46.0)];
            tunnels.push(Tunnel::new(&hab, a, b, 3.0 + rnd() * 4.5));
            if rnd() > 0.45 {
                let m = [
                    (a[0] + b[0]) * 0.5,
                    (a[1] + b[1]) * 0.5,
                    (a[2] + b[2]) * 0.5,
                ];
                let c = [
                    m[0] + (rnd() - 0.5) * 0.30,
                    m[1] + (rnd() - 0.5) * (hab.length * 0.027).clamp(40.0, 160.0),
                    hab.radius - (8.0 + rnd() * 30.0),
                ];
                tunnels.push(Tunnel::new(&hab, m, c, 2.2 + rnd() * 2.4));
            }
        }

        let elev0 = elev.clone();
        let mut flow = Flow::default();
        flow.rebuild(&elev, hab.water_level);

        Self {
            hab,
            elev,
            elev0,
            flow,
            tunnels,
            edits: Edits::default(),
        }
    }

    #[inline]
    fn wrap_t(x: f32) -> f32 {
        x.rem_euclid(NT as f32)
    }

    pub fn sample(e: &[f32], x: f32, y: f32) -> f32 {
        let x = Self::wrap_t(x);
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

    fn grad(e: &[f32], x: f32, y: f32) -> (f32, f32, f32) {
        let h = Self::sample(e, x, y);
        let gx = (Self::sample(e, x + 1.0, y) - Self::sample(e, x - 1.0, y)) * 0.5;
        let gy = (Self::sample(e, x, y + 1.0) - Self::sample(e, x, y - 1.0)) * 0.5;
        (gx, gy, h)
    }

    fn deposit(e: &mut [f32], x: f32, y: f32, amt: f32) {
        let x = Self::wrap_t(x);
        let y = y.clamp(0.0, NZ as f32 - 1.001);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let x1 = (x0 + 1) % NT;
        let y1 = (y0 + 1).min(NZ - 1);
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
            if dm[0] * dm[0] + dm[1] * dm[1] + dm[2] * dm[2] > (tn.reach + rad) * (tn.reach + rad) {
                continue;
            }
            if seg_dist(p, tn.a, tn.b) < tn.rad + rad {
                return true;
            }
        }
        false
    }

    /// The cave-tube term can never subtract more than this. Anything deeper
    /// than it is unambiguously solid, so the two 4-octave 3D noises that shape
    /// tubes can be skipped outright — the dominant cost in chunk meshing.
    pub const MAX_TUBE_CARVE: f32 = 22.0;
    /// Depth past which `density` may return the plain geometric distance.
    /// One cell of margin over MAX_TUBE_CARVE.
    pub const SOLID_DEPTH: f32 = 26.0;
    /// Above the surface by more than this, nothing in the field adds material,
    /// so the geometric distance is already exact.
    pub const AIR_DEPTH: f32 = -3.0;

    /// The 3D density field. > 0 solid, < 0 air.
    pub fn density(&self, p: [f32; 3]) -> f32 {
        let (theta, z, r) = self.hab.to_cyl(p);
        let mut col = self.column(theta, z);
        self.density_col(&mut col, p, r)
    }

    /// Everything in `density` that depends only on (theta, z), hoisted out.
    ///
    /// A radial column of lattice samples shares one surface radius, one slope,
    /// one strata phase and one drainage reading. Computing them per SAMPLE was
    /// ten grid lookups and a flux lookup per voxel; computing them per COLUMN
    /// is ten per thirty voxels. The strata and drainage terms stay lazy so a
    /// stray single `density()` (raycasts, physics) pays for nothing it skips.
    #[inline]
    pub fn column(&self, theta: f32, z: f32) -> Column {
        Column {
            theta,
            z,
            surf: self.surface_radius(theta, z),
            features: true,
            strata: None,
            drainage: f32::NAN,
        }
    }

    fn fill_strata(&self, c: &mut Column) {
        let (theta, z) = (c.theta, c.z);
        let dd = 2.0;
        let gx = (self.elevation(theta + dd / self.hab.radius, z)
            - self.elevation(theta - dd / self.hab.radius, z))
        .abs();
        let gz = (self.elevation(theta, z + dd) - self.elevation(theta, z - dd)).abs();
        let slope = ((gx + gz) / 4.0).clamp(0.0, 1.0);
        if slope <= 0.42 {
            c.strata = Some(Strata::inert());
            return;
        }
        let e0 = self.elevation0(theta, z);
        let band = (e0 * 0.11).sin() * 0.5 + 0.5;
        let hard = (e0 * 0.037).sin() * 0.5 + 0.5;
        let steep = ((slope - 0.42) / 0.58).clamp(0.0, 1.0);
        let ledge = ((e0 * 0.11).fract() - 0.5).abs();
        // Col: local elev saddle (lower than θ± and z± neighbours) carves a notch.
        let e = self.elevation(theta, z);
        let e_t = self
            .elevation(theta + 6.0 / self.hab.radius, z)
            .min(self.elevation(theta - 6.0 / self.hab.radius, z));
        let e_z = self
            .elevation(theta, z + 6.0)
            .min(self.elevation(theta, z - 6.0));
        let saddle = ((e_t.min(e_z) - e) / 8.0).clamp(0.0, 1.0);
        // Hard bands stick out; soft bands recess — bounded overhang.
        // Secondary ledge every ~9 m of strata.
        let offset = (band - 0.5) * 1.8 * steep
            + (0.55 - ledge).max(0.0) * 2.4 * steep * (0.55 + 0.45 * hard);
        let notch = if saddle > 0.15 && slope > 0.35 {
            saddle * 3.2 * steep
        } else {
            0.0
        };
        c.strata = Some(Strata {
            offset: offset - notch,
        });
    }

    /// Density for a point already resolved to cylindrical coordinates, reusing
    /// a column cache. `p` is the same point in world space.
    #[inline]
    pub fn density_col(&self, c: &mut Column, p: [f32; 3], r: f32) -> f32 {
        let from_hull = self.hab.radius - r;
        if from_hull < 8.0 {
            return (8.0 - from_hull) * 10.0 + 1.0;
        }

        let mut d = r - c.surf;

        // Soft cliff terrace: hard strata resist, soft undercuts slightly
        // (LANDSCAPE_4200 §BO). Plus col dips on ridge saddles so routes exist
        // between faces.
        if d > Self::AIR_DEPTH && d < 14.0 {
            if c.strata.is_none() {
                self.fill_strata(c);
            }
            d += c.strata.as_ref().map_or(0.0, |s| s.offset);
        }

        if d > Self::AIR_DEPTH - 5.0 {
            // Keep tubes deeper underground. Opening them ~2.5 m below the
            // surface daylighted as black rectangular holes on valley walls
            // and steep grassy banks (drainage valleys especially).
            if d > 7.0 && d <= Self::SOLID_DEPTH {
                if c.drainage.is_nan() {
                    c.drainage = self.water_flux(c.theta, c.z);
                }
                let thresh = 0.052 + 0.030 * c.drainage;
                let s = 0.020;
                // Both ridges have to be inside the threshold for a tube to
                // exist, and most rock is nowhere near one — so ask the first
                // and only pay for the second when the answer is still open.
                if let Some(a) =
                    tube_dev(p[0] * s, p[1] * s, p[2] * s, self.hab.seed ^ 0xCAFE, thresh)
                {
                    if let Some(b) = tube_dev(
                        p[0] * s + 31.7,
                        p[1] * s - 12.3,
                        p[2] * s + 5.1,
                        self.hab.seed ^ 0xF00D,
                        thresh,
                    ) {
                        let tube = a.max(b);
                        let k = 1.0 - tube / thresh;
                        let fade = ((d - 7.0) / 16.0).clamp(0.0, 1.0);
                        d -= k * k * Self::MAX_TUBE_CARVE * fade;
                    }
                }
            }
            if c.features {
                for tn in &self.tunnels {
                    let dm = [p[0] - tn.mid[0], p[1] - tn.mid[1], p[2] - tn.mid[2]];
                    if dm[0] * dm[0] + dm[1] * dm[1] + dm[2] * dm[2] > tn.reach * tn.reach {
                        continue;
                    }
                    let dist = seg_dist(p, tn.a, tn.b);
                    if dist < tn.rad + 2.0 {
                        d -= (tn.rad + 2.0 - dist) * 9.0;
                    }
                }
            }
        }

        if c.features {
            self.edits.apply(p, d)
        } else {
            d
        }
    }

    /// A carve or fill the procedural band alone would miss.
    ///
    /// The ground band a chunk has to mesh is decided by its own elevations,
    /// which is right for hillsides and caves but blind to artifact bores and
    /// to the shaft a player sank last night. Those are enumerated instead.
    pub fn features_near(&self, th_c: f32, z_c: f32, half_arc: f32, half_z: f32) -> Vec<Feature> {
        let mut out: Vec<Feature> = Vec::new();
        let r0 = self.hab.radius;
        let mut push = |p: [f32; 3], reach: f32, out: &mut Vec<Feature>| {
            let (th, z, r) = self.hab.to_cyl(p);
            let dth = wrap_pi(th - th_c).abs() * r0;
            if dth > half_arc + reach || (z - z_c).abs() > half_z + reach {
                return;
            }
            out.push(Feature {
                theta: th,
                z,
                elev: r0 - r,
                reach,
            });
        };
        for tn in &self.tunnels {
            let len = ((tn.b[0] - tn.a[0]).powi(2)
                + (tn.b[1] - tn.a[1]).powi(2)
                + (tn.b[2] - tn.a[2]).powi(2))
            .sqrt();
            // The bore is a segment; walk it in bites smaller than its own radius
            // so nothing between two samples can be missed.
            let steps = ((len / (tn.rad.max(1.0))).ceil() as usize).clamp(1, 1024);
            let reach = tn.rad + 3.0;
            for s in 0..=steps {
                let f = s as f32 / steps as f32;
                let p = [
                    tn.a[0] + (tn.b[0] - tn.a[0]) * f,
                    tn.a[1] + (tn.b[1] - tn.a[1]) * f,
                    tn.a[2] + (tn.b[2] - tn.a[2]) * f,
                ];
                push(p, reach, &mut out);
            }
        }
        for st in self.edits.strokes_slice() {
            let reach = if st.level {
                st.radius * 1.7 + 2.5
            } else {
                st.radius + 2.5
            };
            push(st.c, reach, &mut out);
        }
        out
    }

    /// The near-chunk index a point falls in. Mirrors `chunker`'s lattice.
    pub fn chunk_of(&self, theta: f32, z: f32) -> (i64, i64) {
        let cell = crate::chunker::LATTICE_CELL;
        let nt = crate::chunker::NT_LAT as i64;
        let n = crate::chunker::CHUNK_N as i64;
        let ti = (theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * nt as f32)
            .floor() as i64;
        let zj = ((z + self.hab.length * 0.5) / cell).floor() as i64;
        (ti.div_euclid(n), zj.div_euclid(n))
    }

    pub fn normal(&self, p: [f32; 3]) -> [f32; 3] {
        let h = 0.35;
        let g = [
            self.density([p[0] + h, p[1], p[2]]) - self.density([p[0] - h, p[1], p[2]]),
            self.density([p[0], p[1] + h, p[2]]) - self.density([p[0], p[1] - h, p[2]]),
            self.density([p[0], p[1], p[2] + h]) - self.density([p[0], p[1], p[2] - h]),
        ];
        let m = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt().max(1e-6);
        [-g[0] / m, -g[1] / m, -g[2] / m]
    }

    pub fn raycast(
        &self,
        o: [f32; 3],
        dir: [f32; 3],
        max: f32,
    ) -> Option<([f32; 3], [f32; 3], [f32; 3])> {
        let step = 0.22;
        let mut t = 0.0f32;
        let mut prev = o;
        while t < max {
            let p = [o[0] + dir[0] * t, o[1] + dir[1] * t, o[2] + dir[2] * t];
            if self.density(p) > 0.0 {
                let (mut a, mut b) = (prev, p);
                for _ in 0..8 {
                    let m = [
                        (a[0] + b[0]) * 0.5,
                        (a[1] + b[1]) * 0.5,
                        (a[2] + b[2]) * 0.5,
                    ];
                    if self.density(m) > 0.0 {
                        b = m;
                    } else {
                        a = m;
                    }
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
        &mut self,
        p: [f32; 3],
        radius: f32,
        snap: f32,
        level: bool,
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
            c,
            radius: r_eff,
            dig: true,
            level,
            up,
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
        let ti = (theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT as f32)
            .round() as usize
            % NT;
        let zi = ((z / self.hab.length + 0.5) * NZ as f32)
            .round()
            .clamp(0.0, (NZ - 1) as f32) as usize;
        let cell_t = std::f32::consts::TAU * self.hab.radius / NT as f32;
        let cell_z = self.hab.length / NZ as f32;
        let r = ((radius / cell_t.min(cell_z)).ceil() as i32 + 3).max(6);
        self.flow.mark_dirty_at(ti, zi, r);
    }

    pub fn mark_flow_dirty_public(&mut self, c: [f32; 3], radius: f32) {
        self.mark_flow_dirty_at(c, radius);
    }

    /// Lower or raise the elevation grid under a brush. This is what makes
    /// digging a watershed tool rather than a cosmetic hole.
    pub fn apply_elev_stroke(
        &mut self,
        c: [f32; 3],
        radius: f32,
        dig: bool,
        level: bool,
        up: [f32; 3],
    ) {
        let (theta, z, _cr) = self.hab.to_cyl(c);
        let cell_t = std::f32::consts::TAU * self.hab.radius / NT as f32;
        let cell_z = self.hab.length / NZ as f32;
        let reach = if level { radius * 1.85 } else { radius * 1.15 };
        let n_t = ((reach / cell_t).ceil() as i32 + 2).max(2);
        let n_z = ((reach / cell_z).ceil() as i32 + 2).max(2);
        let t0 = (theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT as f32)
            .round() as i32;
        let z0 = ((z / self.hab.length + 0.5) * NZ as f32).round() as i32;
        // Depth of cut into the elevation field — always meaningful for watersheds.
        let dig_depth = if dig { radius * 1.35 } else { radius * 0.55 };

        for dz in -n_z..=n_z {
            for dt in -n_t..=n_t {
                let ti = (t0 + dt).rem_euclid(NT as i32) as usize;
                let zi = z0 + dz;
                if zi < 0 || zi >= NZ as i32 {
                    continue;
                }
                let zi = zi as usize;
                let th = ti as f32 / NT as f32 * std::f32::consts::TAU;
                let zz = (zi as f32 / NZ as f32 - 0.5) * self.hab.length;
                let surf_r = self.hab.radius - self.elev[idx(ti, zi)];
                let pw = self.hab.to_world(th, zz, surf_r);
                let rel = [pw[0] - c[0], pw[1] - c[1], pw[2] - c[2]];
                let h = rel[0] * up[0] + rel[1] * up[1] + rel[2] * up[2];
                let ax = [rel[0] - up[0] * h, rel[1] - up[1] * h, rel[2] - up[2] * h];
                let rad = (ax[0] * ax[0] + ax[1] * ax[1] + ax[2] * ax[2]).sqrt();
                let foot = if level { radius } else { reach };
                if rad > foot {
                    continue;
                }
                let w = (1.0 - rad / foot).powf(1.15).max(0.0);

                if level {
                    if dig {
                        let cut = dig_depth * w + h.max(0.0) * w;
                        self.elev[idx(ti, zi)] = (self.elev[idx(ti, zi)] - cut).max(8.5);
                    } else {
                        let raise = dig_depth * w + (-h).max(0.0) * w * 0.5;
                        self.elev[idx(ti, zi)] =
                            (self.elev[idx(ti, zi)] + raise).min(self.hab.max_elevation);
                    }
                } else if dig {
                    self.elev[idx(ti, zi)] = (self.elev[idx(ti, zi)] - dig_depth * w).max(8.5);
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
        if !self.flow.is_dirty() {
            return false;
        }
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
            [
                (p[0] / snap).round() * snap,
                (p[1] / snap).round() * snap,
                (p[2] / snap).round() * snap,
            ]
        } else {
            p
        }
    }

    /// FNV-like hash of quantised elevation — determinism gate (LANDSCAPE_4200 §BY).
    pub fn elev_hash(&self) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        for (i, &e) in self.elev.iter().enumerate().step_by(11) {
            let q = (e * 4.0).round() as i32 as u32;
            h ^= (q as u64).wrapping_add(i as u64);
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    /// Hypsometric buckets: below water, low, mid, high peak fraction.
    pub fn hypsometry(&self) -> (f32, f32, f32, f32) {
        let n = self.elev.len().max(1) as f32;
        let wl = self.hab.water_level;
        let max_e = self.hab.max_elevation.max(1.0);
        let (mut below, mut low, mut mid, mut peak) = (0u32, 0u32, 0u32, 0u32);
        for &e in &self.elev {
            if e < wl {
                below += 1;
            } else if e < max_e * 0.3 {
                low += 1;
            } else if e < max_e * 0.7 {
                mid += 1;
            } else {
                peak += 1;
            }
        }
        (
            below as f32 / n,
            low as f32 / n,
            mid as f32 / n,
            peak as f32 / n,
        )
    }
}

/// A tunnel bite or an edit stroke, reduced to "there is something at this
/// (theta, z, elevation) with this reach". See `Terrain::features_near`.
pub struct Feature {
    pub theta: f32,
    pub z: f32,
    /// Metres above the hull floor.
    pub elev: f32,
    /// Metres it can reach, in any direction.
    pub reach: f32,
}

#[inline]
pub fn wrap_pi(a: f32) -> f32 {
    let t = std::f32::consts::TAU;
    let x = (a + std::f32::consts::PI).rem_euclid(t);
    x - std::f32::consts::PI
}

/// `|fbm3(..., 4 octaves) - 0.5|`, but only when it lands inside `thresh`.
///
/// Returns `None` the moment the octaves still to come cannot carry the sum
/// back across the threshold. The answer is identical to computing the whole
/// sum and comparing; this just declines to finish sums already decided, and
/// cave tubes are thin, so most rock decides after two octaves instead of four.
#[inline]
fn tube_dev(x: f32, y: f32, z: f32, seed: u32, thresh: f32) -> Option<f32> {
    const OCT: u32 = 4;
    // 1 + 1/2 + 1/4 + 1/8 — matches `fbm3`'s normalisation exactly.
    const NORM: f32 = 1.875;
    let (mut f, mut a, mut sum, mut rem) = (1.0f32, 1.0f32, 0.0f32, NORM);
    for o in 0..OCT {
        sum += value3(x * f, y * f, z * f, seed.wrapping_add(o * 6271)) * a;
        rem -= a;
        // value3 is in 0..1, so the final value lies in [sum, sum + rem] / NORM.
        if sum >= NORM * (0.5 + thresh) || sum + rem <= NORM * (0.5 - thresh) {
            return None;
        }
        f *= 2.0;
        a *= 0.5;
    }
    let dev = (sum / NORM - 0.5).abs();
    if dev < thresh {
        Some(dev)
    } else {
        None
    }
}

fn seg_dist(p: [f32; 3], a: [f32; 3], b: [f32; 3]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let ll = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    let t = if ll < 1e-6 {
        0.0
    } else {
        ((ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2]) / ll).clamp(0.0, 1.0)
    };
    let c = [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t];
    let d = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `tube_dev` bails early; it must never bail on an answer that differs
    /// from the full four-octave sum.
    #[test]
    fn tube_dev_matches_full_fbm() {
        let mut r = 0x1234_5678u32;
        let mut rnd = || {
            r = r.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((r >> 8) & 0xFFFF) as f32 / 65535.0
        };
        for _ in 0..200_000 {
            let (x, y, z) = (rnd() * 40.0, rnd() * 40.0, rnd() * 40.0);
            let thresh = 0.052 + 0.030 * rnd();
            let full = fbm3(x, y, z, 4, 0xCAFE);
            let want = (full - 0.5).abs();
            match tube_dev(x, y, z, 0xCAFE, thresh) {
                Some(got) => assert!(
                    want < thresh && (got - want).abs() < 1e-6,
                    "claimed inside: {got} vs {want} (thresh {thresh})"
                ),
                None => assert!(
                    want >= thresh,
                    "bailed out on an inside value: {want} < {thresh}"
                ),
            }
        }
    }

    use crate::habitat::Habitat;

    #[test]
    fn elev_hash_stable() {
        let hab = Habitat::kepler_drum();
        let a = Terrain::generate(hab);
        let b = Terrain::generate(hab);
        assert_eq!(a.elev_hash(), b.elev_hash());
    }

    #[test]
    fn hypsometry_has_sea_and_peaks() {
        let t = Terrain::generate(Habitat::kepler_drum());
        let (below, _low, _mid, peak) = t.hypsometry();
        assert!(
            below > 0.008,
            "expected sea/basin cells below waterline, got {below}"
        );
        assert!(
            peak > 0.02,
            "expected high peaks after max_elev raise, got {peak}"
        );
        let emax = t.elev.iter().cloned().fold(0.0f32, f32::max);
        assert!(
            emax > t.hab.max_elevation * 0.7,
            "max elev {emax} too low vs {}",
            t.hab.max_elevation
        );
    }
}
