//! rama_sim — the Rust simulation core, exposed to Godot as a GDExtension.
//!
//! ARCHITECTURAL RULE (EM_BRIEF.md §2.3): the boundary is narrow and
//! data-oriented. A handful of calls that exchange flat buffers. If this file
//! starts growing chatty per-entity accessors, the design has gone wrong.

use godot::classes::RefCounted;
use godot::prelude::*;

mod agent;
mod biome;
mod biosphere;
mod chronicle;
mod chunker;
mod dwelling;
mod economy;
mod edits;
mod erosion;
mod flow;
mod habitat;
mod lakes;
mod material;
mod mesher;
mod noise;
mod paint;
mod persist;
mod plant;
mod province;
mod soil;
mod sph;
mod terrain;
mod trophic;
mod weather;

#[allow(unused_imports)]
use habitat::Habitat;

/// The global sample lattice every chunk shares. Tangential steps are chosen so
/// arc length at the hull is ~= the radial/axial cell size.
use chunker::{CHUNK_N, LATTICE_CELL, NT_LAT};
use biosphere::Biosphere;
use terrain::Terrain;
use weather::Weather;

/// Map biome + climate → plant species id (0–5) for Multimesh archetypes.
fn forest_kind(
    bid: u8,
    weather: &Weather,
    ter: &Terrain,
    theta: f32,
    z: f32,
    genome: u32,
) -> u8 {
    let elev = ter.elevation(theta, z);
    let arid = weather.aridity_at(theta, z);
    let temp = weather.temp_at_elev(theta, z, elev);
    let g = genome % 6;
    let prov = province::province_at(&ter.hab, theta, z);
    if prov.weight(province::id::CITY) > 0.45 {
        // Street trees / plaza orphans — scrub + rare orchard.
        return if g % 5 == 0 { 5 } else { 3 };
    }
    if prov.weight(province::id::FARMLAND) > 0.45 {
        return 5; // orchard / crop-edge trees
    }
    match bid {
        biome::id::FOREST => {
            if elev > ter.hab.max_elevation * 0.42 || temp < 9.0 {
                0 // montane conifer
            } else if arid > 0.48 {
                3 // dry scrub-forest edge
            } else if g % 3 == 0 {
                0
            } else {
                1 // mesic broadleaf
            }
        }
        biome::id::SWAMP | biome::id::WETLAND | biome::id::RIPARIAN => {
            if g % 5 == 0 {
                4
            } else {
                2
            } // reed pockets in willow
        }
        biome::id::MEADOW => {
            if g % 4 == 0 {
                5
            } else {
                1
            } // orchard-like meadow trees rare
        }
        biome::id::SCRUB | biome::id::ALPINE | biome::id::BARE_ROCK | biome::id::DESERT
        | biome::id::DUNE => 3,
        biome::id::SHORE => 4,
        biome::id::FARM => 5,
        _ => {
            if arid > 0.55 {
                3
            } else {
                1
            }
        }
    }
}

struct RamaSimExtension;

#[gdextension]
unsafe impl ExtensionLibrary for RamaSimExtension {}

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct RamaTerrain {
    t: Option<Terrain>,
    bio: Option<Biosphere>,
    /// Downstream pointers before last dig — for reroute metrics.
    prev_down: Vec<u32>,
    /// Colonist pack — mass and volume both bind (item 820).
    pack: economy::Inventory,
    /// Spoil heaps in the world (item 822).
    heaps: Vec<economy::Stockpile>,
    /// Placed craft stations (878 / kitchen).
    stations: Vec<CraftStation>,
    /// Last known player pose for agent social behaviour.
    player_theta: f32,
    player_z: f32,
    /// Satiety after eating ramen (presentation + mood stub).
    satiety: f32,
    /// LANDSCAPE_3200 Wave 1.6 — mesh on a scoped worker thread when true.
    threaded_meshing: bool,
    base: Base<RefCounted>,
}

#[derive(Clone, Debug)]
struct CraftStation {
    kind: String,
    theta: f32,
    z: f32,
}

#[godot_api]
impl IRefCounted for RamaTerrain {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            t: None,
            bio: None,
            prev_down: Vec::new(),
            pack: economy::Inventory::default(),
            heaps: Vec::new(),
            stations: Vec::new(),
            player_theta: 0.0,
            player_z: 0.0,
            satiety: 0.5,
            threaded_meshing: true,
            base,
        }
    }
}

impl RamaTerrain {
    fn ter(&self) -> &Terrain {
        self.t.as_ref().expect("call generate() first")
    }

    fn fallback_biome_id(t: &Terrain, theta: f32, z: f32) -> u8 {
        let h = t.hab;
        let water_r = h.radius - h.water_level;
        let e = t.elevation(theta, z);
        if t.surface_radius(theta, z) > water_r || t.in_lake(theta, z) {
            return biome::id::WATER;
        }
        let d = 2.0;
        let slope = ((t.elevation(theta + d / h.radius, z) - t.elevation(theta - d / h.radius, z))
            .abs()
            + (t.elevation(theta, z + d) - t.elevation(theta, z - d)).abs())
            / 4.0;
        let flux = t.water_flux(theta, z);
        let arid = {
            let p = province::province_at(&h, theta, z);
            let a0 = match province::climate_intent(p.primary) {
                0 => 0.78,
                1 => 0.38,
                2 => 0.18,
                3 => 0.12,
                _ => 0.4,
            };
            let a1 = match province::climate_intent(p.secondary) {
                0 => 0.78,
                1 => 0.38,
                2 => 0.18,
                3 => 0.12,
                _ => 0.4,
            };
            p.blend2(a0, a1)
        };
        let moisture = (flux * 0.55 + (1.0 - arid) * 0.50).clamp(0.0, 1.0);
        let axial = 1.0 - ((z / h.length).abs() * 2.0).clamp(0.0, 1.0).powf(1.4);
        let temp = 8.0 + 18.0 * axial - e * 0.0065;
        let soil_depth = (2.6 + flux * 5.0) * (1.0 - slope * 0.7).max(0.1);
        biome::classify_ex(moisture, temp, e, slope, flux, soil_depth, arid, h.water_level)
    }

    fn station_required(kind: &str) -> bool {
        matches!(kind, "kiln" | "smelter" | "kitchen")
    }

    fn station_in_range(&self, kind: &str, theta: f32, z: f32, radius_m: f32) -> bool {
        let hab_r = self.t.as_ref().map(|t| t.hab.radius).unwrap_or(900.0);
        let r2 = radius_m * radius_m;
        self.stations.iter().any(|s| {
            if s.kind != kind {
                return false;
            }
            let dth = {
                let x = (s.theta - theta).rem_euclid(std::f32::consts::TAU);
                let x = x.min(std::f32::consts::TAU - x);
                x * hab_r
            };
            let dz = s.z - z;
            dth * dth + dz * dz <= r2
        })
    }

    /// Dig/amend/harvest on a colonist's plot → Help event (1561).
    fn record_plot_helps(
        bio: &mut Biosphere,
        day: f32,
        theta: f32,
        z: f32,
        hab_r: f32,
        magnitude: f32,
        label: &str,
    ) {
        let hits: Vec<(u32, f32)> = bio
            .agents
            .agents
            .iter()
            .filter(|a| a.alive)
            .filter_map(|a| {
                let dth = {
                    let x = (a.plot_theta - theta).rem_euclid(std::f32::consts::TAU);
                    let x = x.min(std::f32::consts::TAU - x);
                    x * hab_r
                };
                let dz = a.plot_z - z;
                if dth * dth + dz * dz <= a.plot_radius * a.plot_radius {
                    Some((a.id, magnitude))
                } else {
                    None
                }
            })
            .collect();
        for (aid, mag) in hits {
            bio.chronicle.record_help(
                day,
                theta,
                z,
                chronicle::ACTOR_PLAYER,
                aid,
                mag,
                label.to_string(),
            );
        }
    }

    /// Bulk-convert to packed arrays. Pushing element-by-element crosses the
    /// FFI boundary a million times per mesh; this crosses it four times.
    /// (EM_BRIEF.md §2.3 — the boundary is flat buffers, not accessors.)
    fn pack(&self, m: mesher::Mesh) -> Dictionary {
        let n = m.verts.len();
        let mut vs: Vec<Vector3> = Vec::with_capacity(n);
        let mut ns: Vec<Vector3> = Vec::with_capacity(n);
        let mut cs: Vec<Color> = Vec::with_capacity(n);
        let has_ao = m.ao.len() == n;
        let has_col = m.cols.len() == n;
        for (i, (v, nn)) in m.verts.iter().zip(m.normals.iter()).enumerate() {
            vs.push(Vector3::new(v[0], v[1], v[2]));
            ns.push(Vector3::new(nn[0], nn[1], nn[2]));
            let c = if has_col {
                m.cols[i]
            } else {
                paint::vertex_color(self.ter(), self.bio.as_ref(), None, *v, *nn)
            };
            // Baked AO rides in alpha. The shader multiplies ambient by it.
            let a = if has_ao { m.ao[i] } else { 1.0 };
            cs.push(Color::from_rgba(c[0], c[1], c[2], a));
        }
        let mut d = Dictionary::new();
        let _ = d.insert("verts", PackedVector3Array::from(vs.as_slice()));
        let _ = d.insert("normals", PackedVector3Array::from(ns.as_slice()));
        let _ = d.insert("colors", PackedColorArray::from(cs.as_slice()));
        let _ = d.insert("indices", PackedInt32Array::from(m.indices.as_slice()));
        d
    }
}

#[godot_api]
impl RamaTerrain {
    /// Build the habitat. Deterministic: same seed, same world. (B8)
    #[func]
    fn generate(&mut self, seed: i64) {
        let mut hab = Habitat::kepler_drum();
        if seed != 0 {
            hab.seed = seed as u32;
        }
        let ter = Terrain::generate(hab);
        let bio = Biosphere::new(&ter);
        self.prev_down = ter.flow.down.clone();
        self.t = Some(ter);
        self.bio = Some(bio);
    }

    /// The decided settings, for display. Nothing here is a magic constant in
    /// the renderer — it all comes from habitat.rs. (REQUIREMENTS.md G)
    #[func]
    fn params(&self) -> Dictionary {
        let h = self.ter().hab;
        let mut d = Dictionary::new();
        let _ = d.insert("radius", h.radius as f64);
        let _ = d.insert("length", h.length as f64);
        let _ = d.insert("omega", h.omega as f64);
        let _ = d.insert("gravity", h.surface_gravity() as f64);
        let _ = d.insert("spin_period", h.spin_period() as f64);
        let _ = d.insert("circumference", (std::f32::consts::TAU * h.radius) as f64);
        let _ = d.insert("max_elevation", h.max_elevation as f64);
        let _ = d.insert("water_level", h.water_level as f64);
        let _ = d.insert("chunk_sagitta_32m", h.chunk_sagitta(32.0) as f64);
        let _ = d.insert("seed", h.seed as i64);
        let _ = d.insert("lattice_cell", LATTICE_CELL as f64);
        let _ = d.insert("chunks_around", (NT_LAT / CHUNK_N) as i64);
        let _ = d.insert("chunk_span", (CHUNK_N as f32 * LATTICE_CELL) as f64);
        d
    }

    /// Coarse whole-drum surface — the far field. Heightfield only, no caves.
    /// This is what curves overhead. (REQUIREMENTS.md A1, A6)
    #[func]
    fn far_mesh(&self, nt: i64, nz: i64, r_offset: f64) -> Dictionary {
        self.far_mesh_sector(nt, nz, r_offset, 0, 1)
    }

    /// One angular sector of the far field. Split the drum into `n_sectors`
    /// wedges so Godot's frustum cull can drop the ones behind the camera —
    /// a single full-drum mesh has an AABB the size of the habitat and never
    /// culls. Overlap one cell so seams stay watertight.
    #[func]
    fn far_mesh_sector(
        &self,
        nt: i64,
        nz: i64,
        r_offset: f64,
        sector: i64,
        n_sectors: i64,
    ) -> Dictionary {
        let t = self.ter();
        let h = t.hab;
        let (nt_full, nz) = (nt.max(8) as usize, nz.max(4) as usize);
        let n_sectors = n_sectors.max(1) as usize;
        let sector = (sector.rem_euclid(n_sectors as i64)) as usize;
        // Cells around the drum for this sector, plus one overlap on each side.
        let per = (nt_full + n_sectors - 1) / n_sectors;
        let t0 = sector * per;
        // Always overlap one cell into the next sector. The last wedge wraps
        // past nt_full so ti%nt_full meets sector 0 — without that the θ=0
        // join cracks into a black saw-tooth on the horizon.
        let t1 = if sector + 1 == n_sectors {
            nt_full + 1
        } else {
            ((sector + 1) * per + 1).min(nt_full + 1)
        };
        let nt_sec = t1 - t0; // vertices in theta (includes overlap)
        if nt_sec < 2 {
            return self.pack(mesher::Mesh::empty());
        }

        let mut m = mesher::Mesh::empty();
        // Mild 5-tap blur on far radius so silhouettes don't stairstep into
        // black saw-teeth, while still tracking ridges.
        let blur = |th: f32, z: f32| -> f32 {
            let e = 1.6f32;
            let a = t.surface_radius(th, z);
            let b = t.surface_radius(th + e / h.radius, z);
            let c = t.surface_radius(th - e / h.radius, z);
            let d = t.surface_radius(th, z + e);
            let f = t.surface_radius(th, z - e);
            (a * 0.40 + (b + c + d + f) * 0.15) + r_offset as f32
        };
        for zi in 0..=nz {
            let z = (zi as f32 / nz as f32 - 0.5) * h.length;
            for ti in t0..t1 {
                let th = (ti % nt_full) as f32 / nt_full as f32 * std::f32::consts::TAU;
                let r = blur(th, z);
                let p = h.to_world(th, z, r);
                m.verts.push(p);
                // No AO on the far field. Four extra bilinear samples per vertex
                // cost ~200 ms across a million vertices, and at 1.8 km under
                // 98 % haze it is invisible. AO stays where it reads: near chunks.
                m.ao.push(1.0);
                let e = 1.2f32;
                let dt = (blur(th + e / h.radius, z) - blur(th - e / h.radius, z)) / (2.0 * e);
                let dz = (blur(th, z + e) - blur(th, z - e)) / (2.0 * e);
                let up = h.up_at(p);
                let tang = [-th.sin(), th.cos(), 0.0];
                let axial = [0.0, 0.0, 1.0];
                let mut n = [0.0f32; 3];
                for i in 0..3 {
                    n[i] = up[i] + tang[i] * dt + axial[i] * dz;
                }
                let mag = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
                m.normals.push([n[0] / mag, n[1] / mag, n[2] / mag]);
            }
        }
        let vid = |ti: usize, zi: usize| (zi * nt_sec + ti) as i32;
        for zi in 0..nz {
            for ti in 0..nt_sec - 1 {
                let (a, b) = (vid(ti, zi), vid(ti + 1, zi));
                let (c, d2) = (vid(ti + 1, zi + 1), vid(ti, zi + 1));
                // Godot front faces are clockwise (ArrayMesh). CCW winding
                // made cull_back discard every triangle facing the camera —
                // the "far voids" were empty clear colour, not fog/haze.
                m.indices.extend_from_slice(&[a, b, c, a, c, d2]);
            }
        }
        self.pack(m)
    }

    /// The drum's endcaps. Without these you can see straight out of the open
    /// ends when you look up, because a line of sight across the axis travels
    /// hundreds of metres axially before it reaches the far side.
    #[func]
    fn endcap_mesh(&self, nt: i64) -> Dictionary {
        let t = self.ter();
        let h = t.hab;
        let nt = nt as usize;
        let mut m = mesher::Mesh::empty();
        for end in [-1.0f32, 1.0f32] {
            // Pull the cap 2 m inside the drum. Sitting exactly on the far
            // mesh's last row makes the two coplanar, and the irregular pale
            // patches on the cap were z-fighting between them.
            let z = end * (h.length * 0.5 - 2.0);
            let base = m.verts.len() as i32;
            m.verts.push([0.0, 0.0, z]);
            m.ao.push(1.0);
            m.normals.push([0.0, 0.0, -end]);
            for ti in 0..nt {
                let th = ti as f32 / nt as f32 * std::f32::consts::TAU;
                let r = t.surface_radius(th, z) + 1.2;
                m.verts.push(h.to_world(th, z, r));
                m.ao.push(1.0);
                m.normals.push([0.0, 0.0, -end]);
            }
            for ti in 0..nt {
                let a = base;
                let b = base + 1 + ti as i32;
                let c = base + 1 + ((ti + 1) % nt) as i32;
                // Clockwise when viewed from inside the drum.
                if end > 0.0 {
                    m.indices.extend_from_slice(&[a, c, b]);
                } else {
                    m.indices.extend_from_slice(&[a, b, c]);
                }
            }
        }
        self.pack(m)
    }

    // ---------------------------------------------------------- mining --

    #[func]
    fn raycast(&self, origin: Vector3, dir: Vector3, max: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let n = dir.normalized();
        match self
            .ter()
            .raycast([origin.x, origin.y, origin.z], [n.x, n.y, n.z], max as f32)
        {
            Some((hit, nrm, air)) => {
                let _ = d.insert("hit", true);
                let _ = d.insert("point", Vector3::new(hit[0], hit[1], hit[2]));
                let _ = d.insert("normal", Vector3::new(nrm[0], nrm[1], nrm[2]));
                let _ = d.insert("air", Vector3::new(air[0], air[1], air[2]));
                let dd = Vector3::new(hit[0], hit[1], hit[2]) - origin;
                let _ = d.insert("distance", dd.length() as f64);
            }
            None => {
                let _ = d.insert("hit", false);
            }
        }
        d
    }

    /// Carve. Returns a dictionary with `ok` and yield parts (items 801–805).
    /// Accepted mass is auto-added to the colonist pack when `auto_take` is true.
    #[func]
    fn dig(&mut self, p: Vector3, radius: f64, snap: f64, level: bool) -> Dictionary {
        self.dig_ex(p, radius, snap, level, true)
    }

    #[func]
    fn dig_ex(
        &mut self,
        p: Vector3,
        radius: f64,
        snap: f64,
        level: bool,
        auto_take: bool,
    ) -> Dictionary {
        let mut d = Dictionary::new();
        let y = {
            let Some(t) = self.t.as_mut() else {
                let _ = d.insert("ok", false);
                return d;
            };
            match t.dig([p.x, p.y, p.z], radius as f32, snap as f32, level) {
                Some(y) => y,
                None => {
                    let _ = d.insert("ok", false);
                    return d;
                }
            }
        };
        let mut accepted = 1.0f32;
        if auto_take {
            accepted = self.pack.try_add(&y);
            if accepted < 0.999 {
                let spill_frac = (1.0 - accepted).max(0.0);
                let (theta, z, _) = self.ter().hab.to_cyl([p.x, p.y, p.z]);
                let hab_r = self.ter().hab.radius;
                for part in &y.parts {
                    economy::deposit_heap(
                        &mut self.heaps,
                        hab_r,
                        theta,
                        z,
                        part.material_id,
                        part.mass_kg * spill_frac,
                        part.loose_m3 * spill_frac,
                        part.grade,
                    );
                }
            }
        }
        let _ = d.insert("ok", true);
        let _ = d.insert("mass_kg", y.total_mass_kg as f64);
        let _ = d.insert("loose_m3", y.total_loose_m3 as f64);
        let _ = d.insert("volume_m3", y.total_volume_m3 as f64);
        let _ = d.insert("accepted", accepted as f64);
        let _ = d.insert("parts", Self::yield_to_array(&y));
        if y.total_volume_m3 > 0.05 {
            let (theta, z, _) = self.ter().hab.to_cyl([p.x, p.y, p.z]);
            let hab_r = self.ter().hab.radius;
            if let Some(bio) = self.bio.as_mut() {
                let day = bio.day;
                bio.chronicle.record(
                    day,
                    theta,
                    z,
                    chronicle::EventKind::Dig,
                    chronicle::ACTOR_PLAYER,
                    y.total_volume_m3,
                    "you dug",
                );
                Self::record_plot_helps(
                    bio,
                    day,
                    theta,
                    z,
                    hab_r,
                    y.total_volume_m3,
                    "you dug on their plot",
                );
            }
        }
        d
    }

    fn yield_to_array(y: &economy::DigYield) -> VariantArray {
        let mut arr = VariantArray::new();
        for part in &y.parts {
            let mut pd = Dictionary::new();
            let _ = pd.insert("material_id", part.material_id as i64);
            let _ = pd.insert("name", economy::bio_name(part.material_id));
            let _ = pd.insert("mass_kg", part.mass_kg as f64);
            let _ = pd.insert("volume_m3", part.volume_m3 as f64);
            let _ = pd.insert("loose_m3", part.loose_m3 as f64);
            let _ = pd.insert("grade", part.grade as f64);
            let _ = arr.push(&pd.to_variant());
        }
        arr
    }

    #[func]
    fn inventory(&self) -> Dictionary {
        let mut d = Dictionary::new();
        let _ = d.insert("mass_kg", self.pack.mass_kg() as f64);
        let _ = d.insert("loose_m3", self.pack.volume_m3() as f64);
        let _ = d.insert("max_mass_kg", self.pack.max_mass_kg as f64);
        let _ = d.insert("max_volume_m3", self.pack.max_volume_m3 as f64);
        let _ = d.insert("mass_frac", self.pack.mass_frac() as f64);
        let _ = d.insert("volume_frac", self.pack.volume_frac() as f64);
        let g = self
            .t
            .as_ref()
            .map(|t| t.hab.surface_gravity())
            .unwrap_or(9.81);
        let _ = d.insert("encumbrance", self.pack.encumbrance(g) as f64);
        let mut stacks = VariantArray::new();
        for s in &self.pack.stacks {
            let mut sd = Dictionary::new();
            let _ = sd.insert("material_id", s.material_id as i64);
            let _ = sd.insert("name", economy::bio_name(s.material_id));
            let _ = sd.insert("mass_kg", s.mass_kg as f64);
            let _ = sd.insert("loose_m3", s.loose_m3 as f64);
            let _ = sd.insert("grade", s.grade as f64);
            let _ = stacks.push(&sd.to_variant());
        }
        let _ = d.insert("stacks", stacks);
        d
    }

    /// Pick the nearest heap back up. The inverse of drop_inventory, and the
    /// reason heaps are storage rather than a one-way loss.
    #[func]
    fn take_heap(&mut self, theta: f64, z: f64, reach: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let hab_r = self.ter().hab.radius;
        let got = economy::take_from_heap(
            &mut self.heaps,
            &mut self.pack,
            hab_r,
            theta as f32,
            z as f32,
            reach as f32,
        );
        match got {
            Some((mid, kg)) => {
                let _ = d.insert("ok", true);
                let _ = d.insert("material", material::name(mid));
                let _ = d.insert("kg", kg as f64);
            }
            None => {
                let _ = d.insert("ok", false);
            }
        }
        d
    }

    /// Mass of the nearest heap in reach, for the pickup prompt. 0 if none.
    #[func]
    fn heap_in_reach(&self, theta: f64, z: f64, reach: f64) -> Dictionary {
        let hab_r = self.ter().hab.radius;
        let (tc, zc, reach) = (theta as f32, z as f32, reach as f32);
        let mut d = Dictionary::new();
        let mut best: Option<(&economy::Stockpile, f32)> = None;
        for h in self.heaps.iter() {
            let dth = {
                let x = (h.theta - tc).rem_euclid(std::f32::consts::TAU);
                let x = x.min(std::f32::consts::TAU - x);
                x * hab_r
            };
            let dz = h.z - zc;
            let d2 = dth * dth + dz * dz;
            if d2 <= reach * reach && best.map_or(true, |(_, b)| d2 < b) {
                best = Some((h, d2));
            }
        }
        match best {
            Some((h, d2)) => {
                let _ = d.insert("ok", true);
                let _ = d.insert("material", material::name(h.material_id));
                let _ = d.insert("kg", h.mass_kg as f64);
                let _ = d.insert("distance", d2.sqrt() as f64);
            }
            None => {
                let _ = d.insert("ok", false);
            }
        }
        d
    }

    /// Drop the whole pack as spoil heaps at (theta, z) (item 822).
    #[func]
    fn drop_inventory(&mut self, theta: f64, z: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let y = self.pack.take_all();
        if y.total_mass_kg < 0.01 {
            let _ = d.insert("ok", false);
            return d;
        }
        let hab_r = self.t.as_ref().map(|t| t.hab.radius).unwrap_or(900.0);
        for part in &y.parts {
            economy::deposit_heap(
                &mut self.heaps,
                hab_r,
                theta as f32,
                z as f32,
                part.material_id,
                part.mass_kg,
                part.loose_m3,
                part.grade,
            );
        }
        let _ = d.insert("ok", true);
        let _ = d.insert("mass_kg", y.total_mass_kg as f64);
        let _ = d.insert("heaps", self.heaps.len() as i64);
        d
    }

    /// Stockpile buffer for Godot heaps: [theta, z, mat_id, mass, loose, grade, radius, ...]
    #[func]
    fn stockpiles(&self) -> PackedFloat32Array {
        let mut out = Vec::with_capacity(self.heaps.len() * 7);
        for h in &self.heaps {
            out.extend_from_slice(&[
                h.theta,
                h.z,
                h.material_id as f32,
                h.mass_kg,
                h.loose_m3,
                h.grade,
                h.radius(),
            ]);
        }
        PackedFloat32Array::from(out.as_slice())
    }

    /// Harvest the nearest living plant within `radius` m (item 813).
    #[func]
    fn harvest_near(&mut self, theta: f64, z: f64, radius: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let (hab_r, hab_len) = {
            let Some(t) = self.t.as_ref() else {
                let _ = d.insert("ok", false);
                return d;
            };
            (t.hab.radius, t.hab.length)
        };
        let y = {
            let Some(bio) = self.bio.as_mut() else {
                let _ = d.insert("ok", false);
                return d;
            };
            let th = theta as f32;
            let zz = z as f32;
            let r2 = (radius as f32).max(1.0).powi(2);
            let mut best: Option<usize> = None;
            let mut best_d = f32::MAX;
            for (i, p) in bio.plants.plants.iter().enumerate() {
                if !p.alive {
                    continue;
                }
                let dth = {
                    let x = (p.theta - th).rem_euclid(std::f32::consts::TAU);
                    let x = x.min(std::f32::consts::TAU - x);
                    x * hab_r
                };
                let dz = p.z - zz;
                let dist2 = dth * dth + dz * dz;
                if dist2 <= r2 && dist2 < best_d {
                    best_d = dist2;
                    best = Some(i);
                }
            }
            let Some(i) = best else {
                let _ = d.insert("ok", false);
                return d;
            };
            let mut y = economy::harvest_plant(&bio.plants.plants[i]);
            bio.plants.plants[i].alive = false;
            // Tag harvest quality from local Miami NPP — lush bands cook better.
            let npp = bio.trophic.npp_at(th, zz, hab_len);
            let grade = (npp / 1800.0).clamp(0.15, 1.0);
            for part in &mut y.parts {
                part.grade = grade;
            }
            y
        };
        let accepted = self.pack.try_add(&y);
        let grade = y.parts.first().map(|p| p.grade).unwrap_or(0.0);
        if y.total_mass_kg > 0.05 {
            let hab_r = self.ter().hab.radius;
            if let Some(bio) = self.bio.as_mut() {
                let day = bio.day;
                bio.chronicle.record(
                    day,
                    theta as f32,
                    z as f32,
                    chronicle::EventKind::Harvest,
                    chronicle::ACTOR_PLAYER,
                    y.total_mass_kg,
                    "you harvested",
                );
                Self::record_plot_helps(
                    bio,
                    day,
                    theta as f32,
                    z as f32,
                    hab_r,
                    y.total_mass_kg * 0.4,
                    "you harvested on their plot",
                );
            }
        }
        let _ = d.insert("ok", true);
        let _ = d.insert("mass_kg", y.total_mass_kg as f64);
        let _ = d.insert("accepted", accepted as f64);
        let _ = d.insert("grade", grade as f64);
        let _ = d.insert("parts", Self::yield_to_array(&y));
        d
    }

    #[func]
    fn recipe_count(&self) -> i64 {
        economy::RECIPES.len() as i64
    }

    #[func]
    fn recipe_at(&self, index: i64) -> Dictionary {
        let mut d = Dictionary::new();
        let Some(r) = economy::RECIPES.get(index as usize) else {
            let _ = d.insert("ok", false);
            return d;
        };
        let atmo = self.bio.as_ref().map(|b| &b.atmosphere);
        let scale = atmo
            .map(|a| economy::max_craft_scale(&self.pack, a, r))
            .unwrap_or(0.0);
        let _ = d.insert("ok", true);
        let _ = d.insert("id", r.id);
        let _ = d.insert("station", r.station);
        let _ = d.insert("energy_kj", r.energy_kj as f64);
        let _ = d.insert("time_s", r.time_s as f64);
        let _ = d.insert("o2_kg", r.o2_kg as f64);
        let _ = d.insert("co2_kg", r.co2_kg as f64);
        let inn: f32 = r.inputs.iter().map(|i| i.mass_kg).sum();
        let out: f32 = r.outputs.iter().map(|o| o.mass_kg).sum();
        let _ = d.insert("mass_in", inn as f64);
        let _ = d.insert("mass_out", out as f64);
        let _ = d.insert("balanced", economy::recipe_mass_ok(r, 0.05));
        let _ = d.insert("max_scale", scale as f64);
        let need = Self::station_required(r.station);
        let _ = d.insert("needs_station", need);
        let _ = d.insert(
            "station_near",
            !need || self.station_in_range(r.station, self.player_theta, self.player_z, 12.0),
        );
        let mut inputs = VariantArray::new();
        for io in r.inputs {
            let mut row = Dictionary::new();
            let _ = row.insert("material", io.material);
            let _ = row.insert("mass_kg", io.mass_kg as f64);
            let have = economy::material_id_by_name(io.material)
                .map(|id| self.pack.mass_of(id))
                .unwrap_or(0.0);
            let _ = row.insert("have_kg", have as f64);
            let _ = inputs.push(&row.to_variant());
        }
        let _ = d.insert("inputs", inputs);
        d
    }

    /// Craft a scaled batch of recipe `index` (items 839–855). Spills to heaps.
    #[func]
    fn craft(&mut self, index: i64, scale: f64, theta: f64, z: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let Some(r) = economy::RECIPES.get(index as usize) else {
            let _ = d.insert("ok", false);
            let _ = d.insert("error", "unknown");
            return d;
        };
        // Kiln / smelter / kitchen need a placed station in range (878–880).
        if Self::station_required(r.station)
            && !self.station_in_range(r.station, theta as f32, z as f32, 12.0)
        {
            let _ = d.insert("ok", false);
            let _ = d.insert("error", "need_station");
            let _ = d.insert("station", r.station);
            return d;
        }
        let Some(bio) = self.bio.as_mut() else {
            let _ = d.insert("ok", false);
            let _ = d.insert("error", "no_bio");
            return d;
        };
        let scale = (scale as f32).clamp(0.0, 1.0);
        let day = bio.day;
        match economy::craft(&mut self.pack, &mut bio.atmosphere, r, scale) {
            Ok(rep) => {
                let hab_r = self.t.as_ref().map(|t| t.hab.radius).unwrap_or(900.0);
                for part in &rep.spilled.parts {
                    economy::deposit_heap(
                        &mut self.heaps,
                        hab_r,
                        theta as f32,
                        z as f32,
                        part.material_id,
                        part.mass_kg,
                        part.loose_m3,
                        0.0,
                    );
                }
                bio.chronicle.record(
                    day,
                    theta as f32,
                    z as f32,
                    chronicle::EventKind::Craft,
                    chronicle::ACTOR_PLAYER,
                    rep.produced.total_mass_kg,
                    format!("you crafted {}", rep.recipe_id),
                );
                let _ = d.insert("ok", true);
                let _ = d.insert("id", rep.recipe_id);
                let _ = d.insert("scale", rep.scale as f64);
                let _ = d.insert("o2_used", rep.o2_used as f64);
                let _ = d.insert("co2_made", rep.co2_made as f64);
                let _ = d.insert("produced_kg", rep.produced.total_mass_kg as f64);
                let _ = d.insert("spilled_kg", rep.spilled.total_mass_kg as f64);
            }
            Err(e) => {
                let _ = d.insert("ok", false);
                let err = match e {
                    economy::CraftError::UnknownRecipe => "unknown",
                    economy::CraftError::MissingInputs => "missing_inputs",
                    economy::CraftError::InsufficientOxygen => "no_oxygen",
                    economy::CraftError::InvalidScale => "bad_scale",
                };
                let _ = d.insert("error", err);
            }
        }
        d
    }

    #[func]
    fn atmosphere(&self) -> Dictionary {
        let mut d = Dictionary::new();
        let Some(bio) = self.bio.as_ref() else {
            let _ = d.insert("ok", false);
            return d;
        };
        let a = &bio.atmosphere;
        let _ = d.insert("ok", true);
        let _ = d.insert("o2_kg", a.o2_kg as f64);
        let _ = d.insert("co2_kg", a.co2_kg as f64);
        let _ = d.insert("n2_kg", a.n2_kg as f64);
        let _ = d.insert("o2_frac", a.o2_frac() as f64);
        let _ = d.insert("co2_ppm", a.co2_ppm() as f64);
        let _ = d.insert("scrub_mw", a.scrub_mw as f64);
        d
    }

    #[func]
    fn set_scrub_mw(&mut self, mw: f64) {
        if let Some(bio) = self.bio.as_mut() {
            bio.atmosphere.scrub_mw = (mw as f32).clamp(0.0, 20.0);
        }
    }

    /// Spread amendment from the pack onto the soil (items 869–871, 855).
    #[func]
    fn amend_soil(&mut self, theta: f64, z: f64, material_id: i64, mass_kg: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let mid = material_id as u8;
        let Some(effect) = economy::amendment_effect(mid) else {
            let _ = d.insert("ok", false);
            let _ = d.insert("error", "not_amendment");
            return d;
        };
        let want = mass_kg as f32;
        let taken = self.pack.take_mass(mid, want);
        if taken < 0.05 {
            let _ = d.insert("ok", false);
            let _ = d.insert("error", "empty");
            return d;
        }
        let hab_r = self.ter().hab.radius;
        let Some(bio) = self.bio.as_mut() else {
            // Put it back if no soil.
            let ph = economy::bio_phys(mid);
            let vol = taken / ph.bulk_kg_m3.max(1.0);
            self.pack.add_stack(mid, taken, vol * ph.bulking, 0.0);
            let _ = d.insert("ok", false);
            return d;
        };
        let day = bio.day;
        bio.soil.amend(theta as f32, z as f32, effect, taken, 4.5);
        bio.chronicle.record(
            day,
            theta as f32,
            z as f32,
            chronicle::EventKind::Amend,
            chronicle::ACTOR_PLAYER,
            taken,
            format!("you amended with {}", economy::bio_name(mid)),
        );
        Self::record_plot_helps(
            bio,
            day,
            theta as f32,
            z as f32,
            hab_r,
            taken,
            "you amended their soil",
        );
        let _ = d.insert("ok", true);
        let _ = d.insert("mass_kg", taken as f64);
        let _ = d.insert("name", economy::bio_name(mid));
        d
    }

    /// Spend glass from the pack to register a greenhouse boost (item 854).
    #[func]
    fn place_greenhouse(&mut self, theta: f64, z: f64) -> Dictionary {
        self.place_greenhouse_ex(theta, z, true)
    }

    #[func]
    fn place_greenhouse_ex(&mut self, theta: f64, z: f64, consume_glass: bool) -> Dictionary {
        let mut d = Dictionary::new();
        let need = economy::GREENHOUSE_GLASS_KG;
        let mut taken = 0.0f32;
        if consume_glass {
            if self.pack.mass_of(economy::craft_id::GLASS) + 1e-3 < need {
                let _ = d.insert("ok", false);
                let _ = d.insert("error", "need_glass");
                let _ = d.insert("need_kg", need as f64);
                return d;
            }
            taken = self.pack.take_mass(economy::craft_id::GLASS, need);
        }
        let Some(bio) = self.bio.as_mut() else {
            if taken > 0.0 {
                let ph = economy::bio_phys(economy::craft_id::GLASS);
                let vol = taken / ph.bulk_kg_m3.max(1.0);
                self.pack
                    .add_stack(economy::craft_id::GLASS, taken, vol * ph.bulking, 0.0);
            }
            let _ = d.insert("ok", false);
            return d;
        };
        bio.greenhouses.push(economy::Greenhouse {
            theta: theta as f32,
            z: z as f32,
            radius: 14.0,
        });
        let _ = d.insert("ok", true);
        let _ = d.insert("glass_kg", taken as f64);
        let _ = d.insert("count", bio.greenhouses.len() as i64);
        d
    }

    #[func]
    fn greenhouse_count(&self) -> i64 {
        self.bio
            .as_ref()
            .map(|b| b.greenhouses.len() as i64)
            .unwrap_or(0)
    }

    #[func]
    fn fill(&mut self, p: Vector3, radius: f64, snap: f64, level: bool) {
        if let Some(t) = self.t.as_mut() {
            t.fill([p.x, p.y, p.z], radius as f32, snap as f32, level);
        }
    }

    #[func]
    fn edit_count(&self) -> i64 {
        self.ter().edits.len() as i64
    }

    /// Rebuild live flow accumulation if excavation dirtied the surface.
    /// Call after a dig burst so rivers reroute. Returns true if flux changed.
    /// How many cells the depression-fill currently treats as lake.
    #[func]
    fn lake_cells(&self) -> i64 {
        self.ter().flow.lake.iter().filter(|v| **v != 0).count() as i64
    }

    /// Mean absolute difference between the routing surface and the real one.
    /// If this rises after a dig, priority-flood is filling the excavation back
    /// in and the player's trench never reaches the router.
    #[func]
    fn fill_depth_mean(&self) -> f64 {
        let t = self.ter();
        let mut acc = 0.0f64;
        for i in 0..t.elev.len() {
            acc += (t.flow.filled[i] - t.elev[i]).max(0.0) as f64;
        }
        acc / t.elev.len() as f64
    }

    /// Hash of the whole downstream-pointer field. Segment counts are far too
    /// coarse to detect a local reroute — this changes if ANY cell now drains
    /// somewhere new, which is exactly the question item 182 asks.
    #[func]
    fn flow_signature(&self) -> i64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for d in &self.ter().flow.down {
            h ^= *d as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        (h & 0x7FFF_FFFF_FFFF_FFFF) as i64
    }

    /// How many cells changed their downstream neighbour, against a snapshot.
    #[func]
    fn flow_diff(&self, before: PackedInt32Array) -> i64 {
        let d = &self.ter().flow.down;
        if before.len() != d.len() {
            return -1;
        }
        let mut n = 0i64;
        for (i, v) in d.iter().enumerate() {
            if before[i] as u32 != *v {
                n += 1;
            }
        }
        n
    }

    /// Snapshot of the downstream field, for `flow_diff`.
    #[func]
    fn flow_snapshot(&self) -> PackedInt32Array {
        let v: Vec<i32> = self.ter().flow.down.iter().map(|x| *x as i32).collect();
        PackedInt32Array::from(v.as_slice())
    }

    #[func]
    fn refresh_flow(&mut self) -> bool {
        let changed = self.t.as_mut().map(|t| t.refresh_flow()).unwrap_or(false);
        if changed {
            if let Some(t) = self.t.as_ref() {
                self.prev_down = t.flow.down.clone();
            }
        }
        changed
    }

    #[func]
    fn flow_dirty(&self) -> bool {
        self.ter().flow.is_dirty()
    }

    /// Advance soil/weather/plants/agents/erosion by `dt_days`.
    #[func]
    fn sim_tick(&mut self, dt_days: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let Some(ter) = self.t.as_mut() else {
            return d;
        };
        let Some(bio) = self.bio.as_mut() else {
            return d;
        };
        let r = bio.tick(
            ter,
            dt_days as f32,
            self.player_theta,
            self.player_z,
            &mut self.heaps,
        );
        let _ = d.insert("day", r.day as f64);
        let _ = d.insert("plants", r.plants_alive as i64);
        let _ = d.insert("rain", r.rain_mean as f64);
        let _ = d.insert("moisture", r.moisture_mean as f64);
        let _ = d.insert("lakes", r.lake_cells as i64);
        let _ = d.insert("lake_entities", r.lake_entities as i64);
        let _ = d.insert("pools", r.pool_cells as i64);
        let _ = d.insert("pool_depth", r.pool_depth as f64);
        let _ = d.insert("sediment", r.sediment_moved as f64);
        let _ = d.insert("nitrogen", r.nitrogen as f64);
        let _ = d.insert("nitrogen_drift", r.nitrogen_drift as f64);
        let _ = d.insert("npp", r.mean_npp as f64);
        let _ = d.insert("max_fauna_kg", r.max_fauna_kg as f64);
        let _ = d.insert("agents", r.agents_alive as i64);
        let _ = d.insert("followers", r.followers as f64);
        let _ = d.insert("works", r.works as i64);
        let _ = d.insert("chronicle", r.chronicle_len as i64);
        let _ = d.insert("carcasses", r.carcasses as i64);
        let _ = d.insert("kills", r.kills as i64);
        let _ = d.insert("mean_fear", r.mean_fear as f64);
        let _ = d.insert("route_sig", (r.route_sig & 0x7FFF_FFFF_FFFF_FFFF) as i64);
        let water_stock = self.bio.as_ref().map(|b| b.water_stock).unwrap_or(0.0);
        let _ = d.insert("water_stock", water_stock as f64);
        d
    }

    /// Move the first farmer onto a plot beside the player spawn (Wave 1 visibility).
    #[func]
    fn seat_neighbour(&mut self, theta: f64, z: f64) {
        let Some(bio) = self.bio.as_mut() else {
            return;
        };
        let th = theta as f32 + 0.07;
        let zz = z as f32 + 55.0;
        if let Some(ag) = bio.agents.agents.first_mut() {
            ag.theta = th;
            ag.z = zz;
            ag.plot_theta = th;
            ag.plot_z = zz;
            ag.plot_radius = 45.0;
        } else {
            bio.agents
                .agents
                .push(crate::agent::Agent::farmer(1, "Ren", th, zz, 0xBEEF));
        }
    }

    #[func]
    fn set_player_pos(&mut self, theta: f64, z: f64) {
        self.player_theta = theta as f32;
        self.player_z = z as f32;
    }

    /// Spectacle clock → weather light schedule + carriage z (Photothermal Spine).
    #[func]
    fn set_day_schedule(&mut self, phase_01: f64, light: f64, carriage_z: f64) {
        if let Some(bio) = self.bio.as_mut() {
            bio.weather
                .set_day_schedule(phase_01 as f32, light as f32, carriage_z as f32);
        }
    }

    /// Photothermal spine readout for HUD / selftest.
    #[func]
    fn spine_status(&self) -> Dictionary {
        let mut d = Dictionary::new();
        let Some(bio) = self.bio.as_ref() else {
            let _ = d.insert("ok", false);
            return d;
        };
        let w = &bio.weather;
        let _ = d.insert("ok", true);
        let _ = d.insert("carriage_z", w.carriage_z as f64);
        let _ = d.insert("vapor_rate", w.spine_vapor_rate as f64);
        let _ = d.insert("light_now", w.light_now() as f64);
        let _ = d.insert("day_phase", w.day_phase as f64);
        let _ = d.insert("schedule_locked", w.schedule_locked);
        let _ = d.insert("sky_event", w.sky_event as i64);
        let _ = d.insert("sky_intensity", w.sky_intensity as f64);
        let _ = d.insert("fog_factor", w.fog_factor as f64);
        let _ = d.insert("mean_humidity", w.mean_humidity as f64);
        let _ = d.insert("spectacle_pace", w.spectacle_pace as f64);
        d
    }

    /// Fast-day / preview: raise sky-event roll rate (1 = normal).
    #[func]
    fn set_spectacle_pace(&mut self, pace: f64) {
        if let Some(bio) = self.bio.as_mut() {
            bio.weather.set_spectacle_pace(pace as f32);
        }
    }

    /// Agents for Godot: [theta, z, hunger, fatigue, mood, id, plot_th, plot_z, plot_r, ...]
    #[func]
    fn agents_lod(&self) -> PackedFloat32Array {
        let mut out = Vec::new();
        let Some(bio) = self.bio.as_ref() else {
            return PackedFloat32Array::from(out.as_slice());
        };
        for a in &bio.agents.agents {
            if !a.alive {
                continue;
            }
            out.extend_from_slice(&[
                a.theta,
                a.z,
                a.hunger,
                a.fatigue,
                a.mood,
                a.id as f32,
                a.plot_theta,
                a.plot_z,
                a.plot_radius,
            ]);
        }
        PackedFloat32Array::from(out.as_slice())
    }

    /// Dwellings for rendering: 9 floats each. Followers and works are the
    /// readout — six followers means thriving, alone in a delve means it is not.
    #[func]
    fn dwellings_lod(&self) -> PackedFloat32Array {
        let mut out = Vec::new();
        let Some(bio) = self.bio.as_ref() else {
            return PackedFloat32Array::from(out.as_slice());
        };
        for d in &bio.dwellings.list {
            out.extend_from_slice(&[
                d.theta,
                d.z,
                d.kind.code(),
                d.followers,
                d.works as f32,
                d.capacity,
                d.reach.code(),
                d.quality,
                d.agent_id as f32,
            ]);
        }
        PackedFloat32Array::from(out.as_slice())
    }

    /// Everything the survey panel needs about one dwelling, by agent id.
    #[func]
    fn dwelling_info(&self, agent_id: i64) -> Dictionary {
        let mut d = Dictionary::new();
        let Some(bio) = self.bio.as_ref() else {
            let _ = d.insert("ok", false);
            return d;
        };
        let Some(w) = bio.dwellings.get(agent_id as u32) else {
            let _ = d.insert("ok", false);
            return d;
        };
        let name = bio
            .agents
            .agents
            .iter()
            .find(|a| a.id == w.agent_id)
            .map(|a| a.name)
            .unwrap_or("colonist");
        let _ = d.insert("ok", true);
        let _ = d.insert("name", name);
        let _ = d.insert("kind", w.kind.as_str());
        let _ = d.insert("reach", w.reach.as_str());
        let _ = d.insert("theta", w.theta as f64);
        let _ = d.insert("z", w.z as f64);
        let _ = d.insert("capacity", w.capacity as f64);
        let _ = d.insert("followers", w.followers as f64);
        let _ = d.insert("works", w.works as i64);
        let _ = d.insert("stores_kg", w.stores_kg as f64);
        let _ = d.insert("days_of_food", w.days_of_food() as f64);
        let _ = d.insert("quality", w.quality as f64);
        let _ = d.insert("trend", w.trend() as f64);
        if let Some(r) = w.rival {
            let rname = bio
                .agents
                .agents
                .iter()
                .find(|a| a.id == r)
                .map(|a| a.name)
                .unwrap_or("someone");
            let _ = d.insert("rival", rname);
        }
        d
    }

    #[func]
    fn agent_line(&self, index: i64) -> GString {
        let Some(bio) = self.bio.as_ref() else {
            return GString::from("");
        };
        bio.agents
            .agents
            .get(index as usize)
            .map(|a| GString::from(a.last_line.as_str()))
            .unwrap_or_else(|| GString::from(""))
    }

    #[func]
    fn agent_name(&self, index: i64) -> GString {
        let Some(bio) = self.bio.as_ref() else {
            return GString::from("");
        };
        bio.agents
            .agents
            .get(index as usize)
            .map(|a| GString::from(a.name))
            .unwrap_or_else(|| GString::from(""))
    }

    #[func]
    fn chronicle_latest(&self, n: i64) -> PackedStringArray {
        let mut out = PackedStringArray::new();
        let Some(bio) = self.bio.as_ref() else {
            return out;
        };
        for e in bio.chronicle.latest(n.max(1) as usize) {
            let who = if e.actor == 0 {
                "you"
            } else {
                bio.agents
                    .agents
                    .iter()
                    .find(|a| a.id == e.actor)
                    .map(|a| a.name)
                    .unwrap_or("colonist")
            };
            let _ = out.push(&format!(
                "d{:.0} {} {}: {}",
                e.day,
                who,
                e.kind.as_str(),
                e.label
            ));
        }
        out
    }

    #[func]
    fn affinity_with(&self, agent_index: i64) -> f64 {
        let Some(bio) = self.bio.as_ref() else {
            return 0.0;
        };
        let Some(ag) = bio.agents.agents.get(agent_index as usize) else {
            return 0.0;
        };
        bio.chronicle.affinity_from_history(
            chronicle::ACTOR_PLAYER,
            ag.id,
            ag.plot_theta,
            ag.plot_z,
            self.ter().hab.radius,
        ) as f64
    }

    /// Citation for the affinity score — the beat you can re-read (1562).
    #[func]
    fn affinity_cite(&self, agent_index: i64) -> Dictionary {
        let mut d = Dictionary::new();
        let Some(bio) = self.bio.as_ref() else {
            let _ = d.insert("ok", false);
            return d;
        };
        let Some(ag) = bio.agents.agents.get(agent_index as usize) else {
            let _ = d.insert("ok", false);
            return d;
        };
        let score = bio.chronicle.affinity_from_history(
            chronicle::ACTOR_PLAYER,
            ag.id,
            ag.plot_theta,
            ag.plot_z,
            self.ter().hab.radius,
        );
        let _ = d.insert("ok", true);
        let _ = d.insert("score", score as f64);
        let _ = d.insert("name", ag.name);
        if let Some(e) = bio.chronicle.affinity_citation(
            chronicle::ACTOR_PLAYER,
            ag.id,
            ag.plot_theta,
            ag.plot_z,
            self.ter().hab.radius,
        ) {
            let _ = d.insert("has_cite", true);
            let _ = d.insert("day", e.day as f64);
            let _ = d.insert("kind", e.kind.as_str());
            let _ = d.insert("label", e.label.as_str());
        } else {
            let _ = d.insert("has_cite", false);
        }
        d
    }

    #[func]
    fn npp_at(&self, theta: f64, z: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let Some(bio) = self.bio.as_ref() else {
            let _ = d.insert("ok", false);
            return d;
        };
        let len = self.ter().hab.length;
        let th = theta as f32;
        let zz = z as f32;
        let npp = bio.trophic.npp_at(th, zz, len);
        let _ = d.insert("ok", true);
        let _ = d.insert("npp", npp as f64);
        let _ = d.insert("producer", bio.trophic.producer_at(th, zz, len) as f64);
        let _ = d.insert("grazer", bio.trophic.grazer_at(th, zz, len) as f64);
        let _ = d.insert("fear", bio.trophic.fear_at(th, zz, len) as f64);
        let dens = crate::trophic::kleiber_density(60.0, npp);
        let _ = d.insert("deer_per_km2", dens as f64);
        d
    }

    /// Carcasses for Godot: [theta, z, mass_kg, age, stage, ...]
    #[func]
    fn carcasses_lod(&self) -> PackedFloat32Array {
        let mut out = Vec::new();
        let Some(bio) = self.bio.as_ref() else {
            return PackedFloat32Array::from(out.as_slice());
        };
        for c in &bio.trophic.carcasses {
            out.extend_from_slice(&[c.theta, c.z, c.mass_kg, c.age_days, c.stage as f32]);
        }
        PackedFloat32Array::from(out.as_slice())
    }

    /// Free starter kitchen + kiln beside spawn so early craft isn't blocked.
    #[func]
    fn seed_starter_stations(&mut self, theta: f64, z: f64) {
        if self.stations.iter().any(|s| s.kind == "kitchen") {
            return;
        }
        self.stations.push(CraftStation {
            kind: "kitchen".into(),
            theta: theta as f32,
            z: z as f32 + 8.0,
        });
        self.stations.push(CraftStation {
            kind: "kiln".into(),
            theta: theta as f32 + 0.02,
            z: z as f32 - 12.0,
        });
    }

    /// Place a craft station (878). Costs timber + glass for kitchen; ore+timber kiln.
    #[func]
    fn place_station(&mut self, kind: GString, theta: f64, z: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let kind = kind.to_string();
        let (ok, err) = match kind.as_str() {
            "kitchen" => {
                let timber = self.pack.take_mass(economy::bio_id::WOOD, 8.0);
                let glass = self.pack.take_mass(economy::craft_id::GLASS, 4.0);
                if timber < 7.5 || glass < 3.5 {
                    if timber > 0.0 {
                        let ph = economy::bio_phys(economy::bio_id::WOOD);
                        self.pack.add_stack(
                            economy::bio_id::WOOD,
                            timber,
                            timber / ph.bulk_kg_m3.max(1.0) * ph.bulking,
                            0.0,
                        );
                    }
                    if glass > 0.0 {
                        let ph = economy::bio_phys(economy::craft_id::GLASS);
                        self.pack.add_stack(
                            economy::craft_id::GLASS,
                            glass,
                            glass / ph.bulk_kg_m3.max(1.0) * ph.bulking,
                            0.0,
                        );
                    }
                    (false, "need_wood_glass")
                } else {
                    (true, "")
                }
            }
            "kiln" | "smelter" => {
                let clay = self.pack.take_mass(material::id::CLAY, 12.0);
                let timber = self.pack.take_mass(economy::bio_id::WOOD, 6.0);
                if clay < 11.0 || timber < 5.5 {
                    if clay > 0.0 {
                        let ph = economy::bio_phys(material::id::CLAY);
                        self.pack.add_stack(
                            material::id::CLAY,
                            clay,
                            clay / ph.bulk_kg_m3.max(1.0) * ph.bulking,
                            0.0,
                        );
                    }
                    if timber > 0.0 {
                        let ph = economy::bio_phys(economy::bio_id::WOOD);
                        self.pack.add_stack(
                            economy::bio_id::WOOD,
                            timber,
                            timber / ph.bulk_kg_m3.max(1.0) * ph.bulking,
                            0.0,
                        );
                    }
                    (false, "need_clay_wood")
                } else {
                    (true, "")
                }
            }
            _ => (false, "unknown"),
        };
        if !ok {
            let _ = d.insert("ok", false);
            let _ = d.insert("error", err);
            return d;
        }
        self.stations.push(CraftStation {
            kind: kind.clone(),
            theta: theta as f32,
            z: z as f32,
        });
        let _ = d.insert("ok", true);
        let _ = d.insert("kind", kind);
        let _ = d.insert("count", self.stations.len() as i64);
        d
    }

    #[func]
    fn stations_lod(&self) -> PackedFloat32Array {
        let mut out = Vec::new();
        for (i, s) in self.stations.iter().enumerate() {
            let code = match s.kind.as_str() {
                "kitchen" => 1.0,
                "kiln" => 2.0,
                "smelter" => 3.0,
                _ => 0.0,
            };
            out.extend_from_slice(&[s.theta, s.z, code, i as f32]);
        }
        PackedFloat32Array::from(out.as_slice())
    }

    /// Eat ramen from the pack — calories + satiety (livelihood payoff).
    #[func]
    fn eat_meal(&mut self) -> Dictionary {
        let mut d = Dictionary::new();
        let mut grade = 0.4f32;
        let mut taken = 0.0f32;
        for id in [economy::craft_id::RICH_RAMEN, economy::craft_id::RAMEN] {
            // Peek grade from first matching stack.
            for s in &self.pack.stacks {
                if s.material_id == id && s.mass_kg > 0.1 {
                    grade = s.grade.max(0.2);
                    break;
                }
            }
            taken = self.pack.take_mass(id, 1.0);
            if taken > 0.1 {
                break;
            }
        }
        if taken < 0.1 {
            let _ = d.insert("ok", false);
            let _ = d.insert("error", "no_ramen");
            return d;
        }
        self.satiety = (self.satiety + 0.35 + grade * 0.25).clamp(0.0, 1.0);
        if let Some(bio) = self.bio.as_mut() {
            bio.chronicle.record(
                bio.day,
                self.player_theta,
                self.player_z,
                chronicle::EventKind::Eat,
                chronicle::ACTOR_PLAYER,
                taken * (0.5 + grade),
                format!("you ate ramen (grade {:.0}%)", grade * 100.0),
            );
        }
        let _ = d.insert("ok", true);
        let _ = d.insert("mass_kg", taken as f64);
        let _ = d.insert("grade", grade as f64);
        let _ = d.insert("satiety", self.satiety as f64);
        d
    }

    #[func]
    fn satiety(&self) -> f64 {
        self.satiety as f64
    }

    /// Materials ledger — every pack pool + atmosphere (886).
    #[func]
    fn materials_ledger(&self) -> Dictionary {
        let mut d = Dictionary::new();
        let mut rows = VariantArray::new();
        for s in &self.pack.stacks {
            if s.mass_kg < 0.05 {
                continue;
            }
            let mut row = Dictionary::new();
            let _ = row.insert("name", economy::bio_name(s.material_id));
            let _ = row.insert("mass_kg", s.mass_kg as f64);
            let _ = row.insert("loose_m3", s.loose_m3 as f64);
            let _ = row.insert("grade", s.grade as f64);
            let _ = rows.push(&row.to_variant());
        }
        let _ = d.insert("pack", rows);
        let _ = d.insert("pack_mass_kg", self.pack.mass_kg() as f64);
        let _ = d.insert("heap_count", self.heaps.len() as i64);
        let heap_mass: f32 = self.heaps.iter().map(|h| h.mass_kg).sum();
        let _ = d.insert("heap_mass_kg", heap_mass as f64);
        if let Some(bio) = self.bio.as_ref() {
            let _ = d.insert("o2_kg", bio.atmosphere.o2_kg as f64);
            let _ = d.insert("co2_kg", bio.atmosphere.co2_kg as f64);
            let _ = d.insert("detritus_mean", {
                let v = &bio.trophic.detritus;
                if v.is_empty() {
                    0.0
                } else {
                    (v.iter().sum::<f32>() / v.len() as f32) as f64
                }
            });
            let _ = d.insert("carcasses", bio.trophic.carcasses.len() as i64);
        }
        let _ = d.insert("stations", self.stations.len() as i64);
        let _ = d.insert("satiety", self.satiety as f64);
        d
    }

    /// Lake entities: flat [theta,z,level,area,volume, cells, ...]
    #[func]
    fn lakes_list(&self) -> PackedFloat32Array {
        let mut out = Vec::new();
        let Some(bio) = self.bio.as_ref() else {
            return PackedFloat32Array::from(out.as_slice());
        };
        for l in &bio.lakes.lakes {
            out.extend_from_slice(&[l.theta, l.z, l.level, l.area_m2, l.volume, l.cells as f32]);
        }
        PackedFloat32Array::from(out.as_slice())
    }

    /// Catchment of the aim cell: subsampled [theta, z, ...] points (NEXT #8).
    #[func]
    fn catchment_points(&self, theta: f64, z: f64, limit: i64) -> PackedFloat32Array {
        let t = self.ter();
        let th = theta as f32;
        let zz = z as f32;
        let ti = (th.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * terrain::NT as f32)
            .round() as usize
            % terrain::NT;
        let zi = ((zz / t.hab.length + 0.5) * terrain::NZ as f32)
            .round()
            .clamp(0.0, (terrain::NZ - 1) as f32) as usize;
        let lim = limit.max(8) as usize;
        let cells = t.flow.catchment_local(ti, zi, 90, lim);
        let mut out = Vec::with_capacity(cells.len() * 2);
        for (tti, zzi) in cells {
            let oth = tti as f32 / terrain::NT as f32 * std::f32::consts::TAU;
            let oz = (zzi as f32 / terrain::NZ as f32 - 0.5) * t.hab.length;
            out.push(oth);
            out.push(oz);
        }
        PackedFloat32Array::from(out.as_slice())
    }

    /// Soil plan RGB. `mode`: 0=org/N/wet composite, 1=organic, 2=N, 3=moisture.
    #[func]
    fn soil_map(&self, nt: i64, nz: i64, mode: i64) -> PackedByteArray {
        let mut out = Vec::new();
        let Some(bio) = self.bio.as_ref() else {
            return PackedByteArray::from(out.as_slice());
        };
        let h = self.ter().hab;
        let (nt, nz) = (nt.max(8) as usize, nz.max(8) as usize);
        let mode = mode.clamp(0, 3) as u8;
        out.reserve(nt * nz * 3);
        for zi in 0..nz {
            let z = (zi as f32 / nz as f32 - 0.5) * h.length * 0.995;
            for ti in 0..nt {
                let th = ti as f32 / nt as f32 * std::f32::consts::TAU;
                let s = bio.soil.sample(th, z);
                let (r, g, b) = match mode {
                    1 => {
                        let v = (s.organic * 255.0).clamp(0.0, 255.0) as u8;
                        (v, (v as f32 * 0.55) as u8, (v as f32 * 0.35) as u8)
                    }
                    2 => {
                        let v = (s.n.clamp(0.0, 1.0) * 255.0) as u8;
                        ((v as f32 * 0.35) as u8, v, (v as f32 * 0.45) as u8)
                    }
                    3 => {
                        let v = (s.moisture.clamp(0.0, 1.0) * 255.0) as u8;
                        ((v as f32 * 0.25) as u8, (v as f32 * 0.55) as u8, v)
                    }
                    _ => (
                        (s.organic * 255.0).clamp(0.0, 255.0) as u8,
                        (s.n.clamp(0.0, 1.0) * 255.0) as u8,
                        (s.moisture.clamp(0.0, 1.0) * 255.0) as u8,
                    ),
                };
                out.push(r);
                out.push(g);
                out.push(b);
            }
        }
        PackedByteArray::from(out.as_slice())
    }

    /// Recompute vertex colours for existing chunk verts (no remesh).
    #[func]
    fn colors_at(&self, verts: PackedVector3Array) -> PackedColorArray {
        let mut cols = Vec::with_capacity(verts.len());
        for v in verts.as_slice() {
            // Approximate normal as local up for colour shading.
            let p = [v.x, v.y, v.z];
            let up = self.ter().hab.up_at(p);
            let c = paint::vertex_color(self.ter(), self.bio.as_ref(), None, p, up);
            cols.push(Color::from_rgb(c[0], c[1], c[2]));
        }
        PackedColorArray::from(cols.as_slice())
    }

    /// Standing-water free surface: flat lake-entity basins plus dig ponds.
    #[func]
    fn lake_mesh(&self) -> Dictionary {
        let mut d = Dictionary::new();
        let Some(bio) = self.bio.as_ref() else {
            return d;
        };
        let t = self.ter();
        let (mut verts, mut normals, mut indices, mut colors) =
            bio.lakes.surface_mesh(&t.hab, &t.elev, 240_000);
        let base = verts.len() as i32;
        let (pv, pn, pi, pc) =
            bio.water
                .pool_mesh(&t.hab, &t.elev, Some(&bio.lakes.cell_lake), 140_000);
        verts.extend_from_slice(&pv);
        normals.extend_from_slice(&pn);
        colors.extend_from_slice(&pc);
        indices.extend(pi.into_iter().map(|i| i + base));
        let vs: Vec<Vector3> = verts
            .iter()
            .map(|v| Vector3::new(v[0], v[1], v[2]))
            .collect();
        let ns: Vec<Vector3> = normals
            .iter()
            .map(|n| Vector3::new(n[0], n[1], n[2]))
            .collect();
        let cs: Vec<Color> = colors
            .iter()
            .map(|c| Color::from_rgba(c[0], c[1], c[2], c[3]))
            .collect();
        let _ = d.insert("verts", PackedVector3Array::from(vs.as_slice()));
        let _ = d.insert("normals", PackedVector3Array::from(ns.as_slice()));
        let _ = d.insert("colors", PackedColorArray::from(cs.as_slice()));
        let _ = d.insert("indices", PackedInt32Array::from(indices.as_slice()));
        let _ = d.insert("wet", bio.water.wet_cells() as i64);
        let _ = d.insert("lakes", bio.lakes.lakes.len() as i64);
        d
    }

    #[func]
    fn water_depth_at(&self, theta: f64, z: f64) -> f64 {
        let Some(bio) = self.bio.as_ref() else {
            return 0.0;
        };
        let t = self.ter();
        let th = (theta as f32).rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU
            * terrain::NT as f32;
        let zz = ((z as f32) / t.hab.length + 0.5) * terrain::NZ as f32;
        bio.water.sample_depth(th, zz) as f64
    }

    /// Scoop standing water into the pack (item 817). `liters` is requested volume.
    #[func]
    fn scoop_water(&mut self, theta: f64, z: f64, liters: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let want_m3 = (liters as f32 / 1000.0).max(0.0);
        if want_m3 <= 1e-6 {
            let _ = d.insert("ok", false);
            return d;
        }
        let (ti, zi, cell_area, elev) = {
            let Some(ter) = self.t.as_ref() else {
                let _ = d.insert("ok", false);
                return d;
            };
            let th = (theta as f32).rem_euclid(std::f32::consts::TAU);
            let zz = z as f32;
            let ti =
                (th / std::f32::consts::TAU * terrain::NT as f32).round() as usize % terrain::NT;
            let zi = ((zz / ter.hab.length + 0.5) * terrain::NZ as f32)
                .round()
                .clamp(1.0, (terrain::NZ - 2) as f32) as usize;
            let cell_area = (std::f32::consts::TAU * ter.hab.radius / terrain::NT as f32)
                * (ter.hab.length / terrain::NZ as f32);
            (ti, zi, cell_area, ter.elev.clone())
        };
        let taken_m3 = {
            let Some(bio) = self.bio.as_mut() else {
                let _ = d.insert("ok", false);
                return d;
            };
            bio.water.scoop_at(&elev, ti, zi, 3, want_m3, cell_area)
        };
        if taken_m3 <= 1e-5 {
            let _ = d.insert("ok", false);
            let _ = d.insert("liters", 0.0);
            return d;
        }
        let mass = taken_m3 * 1000.0;
        let mut y = economy::DigYield::default();
        y.push(economy::YieldPart {
            material_id: economy::bio_id::WATER,
            volume_m3: taken_m3,
            mass_kg: mass,
            loose_m3: taken_m3,
            grade: 0.0,
        });
        let accepted = self.pack.try_add(&y);
        if accepted < 0.999 {
            let spill = taken_m3 * (1.0 - accepted);
            if let Some(bio) = self.bio.as_mut() {
                bio.water.pour_at(&elev, ti, zi, 3, spill, cell_area);
            }
        }
        let kept = taken_m3 * accepted;
        let _ = d.insert("ok", kept > 1e-5);
        let _ = d.insert("liters", (kept * 1000.0) as f64);
        let _ = d.insert("mass_kg", (kept * 1000.0) as f64);
        let _ = d.insert("accepted", accepted as f64);
        d
    }

    /// Pour carried water onto the ground / into a pit.
    #[func]
    fn pour_water(&mut self, theta: f64, z: f64, liters: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let want_m3 = (liters as f32 / 1000.0).max(0.0);
        let have = self.pack.mass_of(economy::bio_id::WATER);
        let take_kg = (want_m3 * 1000.0).min(have);
        if take_kg <= 0.05 {
            let _ = d.insert("ok", false);
            return d;
        }
        let taken = self.pack.take_mass(economy::bio_id::WATER, take_kg);
        let add_m3 = taken / 1000.0;
        let (placed, _ti, _zi) = {
            let Some(ter) = self.t.as_ref() else {
                self.pack
                    .add_stack(economy::bio_id::WATER, taken, add_m3, 0.0);
                let _ = d.insert("ok", false);
                return d;
            };
            let elev = ter.elev.clone();
            let cell_area = (std::f32::consts::TAU * ter.hab.radius / terrain::NT as f32)
                * (ter.hab.length / terrain::NZ as f32);
            let th = (theta as f32).rem_euclid(std::f32::consts::TAU);
            let zz = z as f32;
            let ti =
                (th / std::f32::consts::TAU * terrain::NT as f32).round() as usize % terrain::NT;
            let zi = ((zz / ter.hab.length + 0.5) * terrain::NZ as f32)
                .round()
                .clamp(1.0, (terrain::NZ - 2) as f32) as usize;
            let Some(bio) = self.bio.as_mut() else {
                self.pack
                    .add_stack(economy::bio_id::WATER, taken, add_m3, 0.0);
                let _ = d.insert("ok", false);
                return d;
            };
            let placed = bio.water.pour_at(&elev, ti, zi, 4, add_m3, cell_area);
            (placed, ti, zi)
        };
        if placed + 1e-5 < add_m3 {
            let back = (add_m3 - placed) * 1000.0;
            self.pack
                .add_stack(economy::bio_id::WATER, back, add_m3 - placed, 0.0);
        }
        let _ = d.insert("ok", placed > 1e-5);
        let _ = d.insert("liters", (placed * 1000.0) as f64);
        let _ = d.insert("mass_kg", (placed * 1000.0) as f64);
        d
    }

    /// Notify biosphere that a dig landed — refresh local hardness (strata).
    #[func]
    fn notify_dig(&mut self, p: Vector3, radius: f64) {
        let Some(ter) = self.t.as_ref() else {
            return;
        };
        let (theta, z, _) = ter.hab.to_cyl([p.x, p.y, p.z]);
        let ti = (theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU
            * terrain::NT as f32)
            .round() as usize
            % terrain::NT;
        let zi = ((z / ter.hab.length + 0.5) * terrain::NZ as f32)
            .round()
            .clamp(0.0, (terrain::NZ - 1) as f32) as usize;
        let cell = (std::f32::consts::TAU * ter.hab.radius / terrain::NT as f32)
            .min(ter.hab.length / terrain::NZ as f32);
        let r = ((radius as f32 / cell).ceil() as i32 + 2).max(4);
        if let Some(bio) = self.bio.as_mut() {
            bio.refresh_hardness_at(ter, ti, zi, r);
        }
    }

    /// Plants in LOD rings: flat [theta,z,stem,leaf,alive,lod,biome_id,genome_id, ...]
    /// lod 0 = near detail, 1 = mid, 2 = far billboard.
    #[func]
    fn plants_lod(&self, theta: f64, z: f64, limit: i64) -> PackedFloat32Array {
        let mut out = Vec::new();
        let Some(bio) = self.bio.as_ref() else {
            return PackedFloat32Array::from(out.as_slice());
        };
        let th = theta as f32;
        let zz = z as f32;
        let lim = limit.max(1) as usize;
        let r_near = 55.0f32;
        let r_mid = 140.0f32;
        let r_far = 320.0f32;
        let mut counts = [0usize; 3];
        let caps = [400usize, 500usize, 300usize];
        for p in &bio.plants.plants {
            if !p.alive {
                continue;
            }
            let dth = (p.theta - th).rem_euclid(std::f32::consts::TAU);
            let dth = dth.min(std::f32::consts::TAU - dth) * self.ter().hab.radius;
            let dz = (p.z - zz).abs();
            let dist = (dth * dth + dz * dz).sqrt();
            let lod = if dist <= r_near {
                0
            } else if dist <= r_mid {
                1
            } else if dist <= r_far {
                2
            } else {
                continue;
            };
            if counts[lod] >= caps[lod] {
                continue;
            }
            let bid = bio.biome_at(self.ter(), p.theta, p.z);
            // Forest kinds from climate: elev+aridity → conifer / broadleaf / willow /
            // scrub (Wave 5). Genome still jitters within kind.
            let kind = forest_kind(bid, &bio.weather, self.ter(), p.theta, p.z, p.genome_id);
            out.extend_from_slice(&[
                p.theta,
                p.z,
                p.stem,
                p.leaf,
                1.0,
                lod as f32,
                bid as f32,
                kind as f32,
            ]);
            counts[lod] += 1;
            if out.len() / 8 >= lim {
                break;
            }
        }
        PackedFloat32Array::from(out.as_slice())
    }

    #[func]
    fn save_strokes(&self) -> PackedByteArray {
        let bytes = persist::encode_strokes(&self.ter().edits);
        PackedByteArray::from(bytes.as_slice())
    }

    #[func]
    fn load_strokes(&mut self, data: PackedByteArray) -> bool {
        let bytes: Vec<u8> = data.to_vec();
        let Some(strokes) = persist::decode_strokes(&bytes) else {
            return false;
        };
        let Some(ter) = self.t.as_mut() else {
            return false;
        };
        ter.elev.copy_from_slice(&ter.elev0);
        ter.edits.clear();
        for s in &strokes {
            ter.edits.add(s.c, s.radius, s.dig, s.level, s.up);
            ter.apply_elev_stroke(s.c, s.radius, s.dig, s.level, s.up);
        }
        ter.flow.mark_dirty();
        let _ = ter.refresh_flow_full();
        if let Some(bio) = self.bio.as_mut() {
            bio.refresh_lakes(ter);
        }
        true
    }

    /// Persist soil N + moisture deltas (after strokes blob, optional second save).
    #[func]
    fn save_soil(&self) -> PackedByteArray {
        let Some(bio) = self.bio.as_ref() else {
            return PackedByteArray::from([].as_slice());
        };
        PackedByteArray::from(persist::encode_soil(&bio.soil).as_slice())
    }

    #[func]
    fn load_soil(&mut self, data: PackedByteArray) -> bool {
        let bytes: Vec<u8> = data.to_vec();
        let Some(bio) = self.bio.as_mut() else {
            return false;
        };
        persist::decode_soil_into(&bytes, &mut bio.soil)
    }

    #[func]
    fn save_dwellings(&self) -> PackedByteArray {
        let Some(bio) = self.bio.as_ref() else {
            return PackedByteArray::from([].as_slice());
        };
        PackedByteArray::from(persist::encode_dwellings(&bio.dwellings.list).as_slice())
    }

    #[func]
    fn load_dwellings(&mut self, data: PackedByteArray) -> bool {
        let bytes: Vec<u8> = data.to_vec();
        let Some(records) = persist::decode_dwellings(&bytes) else {
            return false;
        };
        let Some(bio) = self.bio.as_mut() else {
            return false;
        };
        let had_records = !records.is_empty();
        let mut matched = 0usize;
        let mut homes = Vec::new();
        for rec in records {
            if let Some(d) = bio
                .dwellings
                .list
                .iter_mut()
                .find(|d| d.agent_id == rec.agent_id)
            {
                d.kind = match rec.kind {
                    0 => dwelling::DwellKind::Delve,
                    1 => dwelling::DwellKind::Terrace,
                    2 => dwelling::DwellKind::Township,
                    _ => d.kind,
                };
                d.theta = rec.theta;
                d.z = rec.z;
                d.followers = rec.followers;
                d.works = rec.works;
                d.stores_kg = rec.stores_kg;
                d.build_days = rec.build_days;
                d.capacity = rec.capacity;
                d.quality = rec.quality;
                homes.push((rec.agent_id, rec.theta, rec.z));
                matched += 1;
            }
        }
        for (agent_id, theta, z) in homes {
            if let Some(ag) = bio.agents.agents.iter_mut().find(|a| a.id == agent_id) {
                ag.plot_theta = theta;
                ag.plot_z = z;
            }
        }
        matched > 0 || !had_records
    }

    #[func]
    fn soil_at(&self, theta: f64, z: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let Some(bio) = self.bio.as_ref() else {
            return d;
        };
        let s = bio.soil.sample(theta as f32, z as f32);
        let _ = d.insert("n", s.n as f64);
        let _ = d.insert("p", s.p as f64);
        let _ = d.insert("k", s.k as f64);
        let _ = d.insert("organic", s.organic as f64);
        let _ = d.insert("moisture", s.moisture as f64);
        let _ = d.insert("ph", s.ph as f64);
        d
    }

    #[func]
    fn weather_at(&self, theta: f64, z: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let Some(bio) = self.bio.as_ref() else {
            return d;
        };
        let th = theta as f32;
        let zz = z as f32;
        let _ = d.insert("rain", bio.weather.rain_at(th, zz) as f64);
        let _ = d.insert("temp", bio.weather.temp_at(th, zz) as f64);
        let _ = d.insert("humidity", bio.weather.humidity_at(th, zz) as f64);
        let _ = d.insert("band", Weather::band_index(th) as i64);
        let _ = d.insert("condensers", bio.weather.condensers.len() as i64);
        let _ = d.insert("power_used", bio.weather.power_used as f64);
        let _ = d.insert("sky_event", bio.weather.sky_event as i64);
        let _ = d.insert("sky_intensity", bio.weather.sky_intensity as f64);
        let _ = d.insert("fog_factor", bio.weather.fog_factor as f64);
        d
    }

    #[func]
    fn add_condenser(&mut self, theta: f64, z: f64, power: f64) {
        if let Some(bio) = self.bio.as_mut() {
            bio.weather
                .add_condenser(theta as f32, z as f32, power as f32);
        }
    }

    /// Returns false if reactor budget cannot cover the new condenser.
    #[func]
    fn try_add_condenser(&mut self, theta: f64, z: f64, power: f64) -> bool {
        self.bio
            .as_mut()
            .map(|b| {
                b.weather
                    .try_add_condenser(theta as f32, z as f32, power as f32)
            })
            .unwrap_or(false)
    }

    #[func]
    fn power_budget(&self) -> Dictionary {
        let mut d = Dictionary::new();
        let Some(bio) = self.bio.as_ref() else {
            return d;
        };
        let _ = d.insert("used", bio.weather.power_used as f64);
        let _ = d.insert("budget", bio.weather.reactor_power_budget as f64);
        let _ = d.insert("headroom", bio.weather.power_headroom() as f64);
        let _ = d.insert("condensers", bio.weather.condensers.len() as i64);
        d
    }

    /// Condensers for rain-device rendering: [theta, z, power, radius, ...].
    #[func]
    fn condensers_lod(&self) -> PackedFloat32Array {
        let mut out = Vec::new();
        let Some(bio) = self.bio.as_ref() else {
            return PackedFloat32Array::from(out.as_slice());
        };
        for c in &bio.weather.condensers {
            out.extend_from_slice(&[c.theta, c.z, c.power, c.radius]);
        }
        PackedFloat32Array::from(out.as_slice())
    }

    #[func]
    fn biome_at(&self, theta: f64, z: f64) -> Dictionary {
        let mut d = Dictionary::new();
        let Some(bio) = self.bio.as_ref() else {
            return d;
        };
        let id = bio.biome_at(self.ter(), theta as f32, z as f32);
        let _ = d.insert("id", id as i64);
        let _ = d.insert("name", biome::name(id));
        let c = biome::color(id);
        let _ = d.insert("color", Color::from_rgb(c[0], c[1], c[2]));
        d
    }

    /// Province zoning readout (LANDSCAPE_4200).
    #[func]
    fn province_at(&self, theta: f64, z: f64) -> Dictionary {
        let t = self.ter();
        let p = province::province_at(&t.hab, theta as f32, z as f32);
        let mut d = Dictionary::new();
        let _ = d.insert("id", p.primary as i64);
        let _ = d.insert("secondary", p.secondary as i64);
        let _ = d.insert("name", province::name(p.primary));
        let _ = d.insert("w0", p.w0 as f64);
        let _ = d.insert("w1", p.w1 as f64);
        let c = province::color(p.primary);
        let _ = d.insert("color", Color::from_rgb(c[0], c[1], c[2]));
        d
    }

    #[func]
    fn province_census(&self) -> Dictionary {
        let t = self.ter();
        let c = province::census(&t.hab, 64, 32);
        let mut d = Dictionary::new();
        for i in 0..province::id::COUNT {
            let _ = d.insert(province::name(i as u8), c[i] as i64);
        }
        d
    }

    /// Plant instances near the player for rendering: flat [theta,z,stem,leaf,alive, ...].
    #[func]
    fn plants_near(&self, theta: f64, z: f64, radius: f64, limit: i64) -> PackedFloat32Array {
        let mut out = Vec::new();
        let Some(bio) = self.bio.as_ref() else {
            return PackedFloat32Array::from(out.as_slice());
        };
        let th = theta as f32;
        let zz = z as f32;
        let rad = radius as f32;
        let lim = limit.max(1) as usize;
        let mut n = 0usize;
        for p in &bio.plants.plants {
            if !p.alive {
                continue;
            }
            let dth = (p.theta - th).rem_euclid(std::f32::consts::TAU);
            let dth = dth.min(std::f32::consts::TAU - dth) * self.ter().hab.radius;
            let dz = (p.z - zz).abs();
            if (dth * dth + dz * dz).sqrt() > rad {
                continue;
            }
            out.extend_from_slice(&[
                p.theta,
                p.z,
                p.stem,
                p.leaf,
                if p.alive { 1.0 } else { 0.0 },
            ]);
            n += 1;
            if n >= lim {
                break;
            }
        }
        PackedFloat32Array::from(out.as_slice())
    }

    #[func]
    fn plant_count(&self) -> i64 {
        self.bio
            .as_ref()
            .map(|b| b.plants.plants.iter().filter(|p| p.alive).count() as i64)
            .unwrap_or(0)
    }

    #[func]
    fn lake_count(&self) -> i64 {
        self.ter().flow.lake_count as i64
    }

    #[func]
    fn mean_fill_depth(&self) -> f64 {
        self.ter().flow.mean_fill_depth as f64
    }

    #[func]
    fn route_sig(&self) -> i64 {
        (self.ter().flow.route_sig & 0x7FFF_FFFF_FFFF_FFFF) as i64
    }

    #[func]
    fn reroute_count(&self) -> i64 {
        self.ter().flow.down_diff_count(&self.prev_down) as i64
    }

    /// Snapshot current routing before a dig burst, so reroute_count is meaningful.
    #[func]
    fn snapshot_routes(&mut self) {
        if let Some(t) = self.t.as_ref() {
            self.prev_down = t.flow.down.clone();
        }
    }

    /// Undo the last excavation. Returns where it was so the caller knows which
    /// chunks to remesh.
    #[func]
    fn undo_dig(&mut self) -> Dictionary {
        let mut d = Dictionary::new();
        let popped = self.t.as_mut().and_then(|t| t.undo_dig());
        match popped {
            Some(s) => {
                let _ = d.insert("ok", true);
                let _ = d.insert("point", Vector3::new(s.c[0], s.c[1], s.c[2]));
                let _ = d.insert(
                    "radius",
                    (s.radius * if s.level { 1.7 } else { 1.0 }) as f64,
                );
            }
            None => {
                let _ = d.insert("ok", false);
            }
        }
        d
    }

    /// What the player is looking at, in words the HUD can show.
    #[func]
    fn probe(&self, p: Vector3) -> Dictionary {
        let t = self.ter();
        let (theta, z, r) = t.hab.to_cyl([p.x, p.y, p.z]);
        let surf = t.surface_radius(theta, z);
        let mut d = Dictionary::new();
        let from_hull = t.hab.radius - r;
        let below = r - surf;
        let mat = material::material_at(t, [p.x, p.y, p.z]);
        let info = material::info(mat);
        let kind = if mat == material::id::ALLOY {
            "structural alloy — undiggable"
        } else if below > 2.0 {
            info.name
        } else if t.in_lake(theta, z) || t.hab.radius - r < t.hab.water_level {
            "lake / shoreline"
        } else if t.water_flux(theta, z) > 0.55 {
            "channel — live drainage"
        } else {
            info.name
        };
        let _ = d.insert("kind", kind);
        let _ = d.insert("material", info.name);
        let _ = d.insert("material_id", mat as i64);
        let _ = d.insert("hardness", info.hardness as f64);
        let moisture = self
            .bio
            .as_ref()
            .map(|b| b.soil.sample(theta, z).moisture)
            .unwrap_or(0.0);
        let _ = d.insert("moisture", moisture as f64);
        let grade = economy::ore_grade(t, [p.x, p.y, p.z]);
        let _ = d.insert("ore_grade", grade as f64);
        let ph = economy::phys(mat);
        let _ = d.insert("bulk_kg_m3", ph.bulk_kg_m3 as f64);
        let _ = d.insert("bulking", ph.bulking as f64);
        let _ = d.insert("depth", below as f64);
        let _ = d.insert("elevation", (t.hab.radius - r) as f64);
        let _ = d.insert("flux", t.water_flux(theta, z) as f64);
        let _ = d.insert("diggable", mat != material::id::ALLOY && from_hull >= 10.0);
        d
    }

    /// Vertical material column at a surface point (item 13).
    #[func]
    fn strata_at(&self, theta: f64, z: f64, steps: i64) -> PackedByteArray {
        let col = material::strata_column(
            self.ter(),
            theta as f32,
            z as f32,
            steps.max(1) as usize,
            1.5,
        );
        PackedByteArray::from(col.as_slice())
    }

    /// River ribbon segments: flat f32 buffer of [x0,y0,z0, x1,y1,z1, width, ...].
    /// Mutes channels that sit under standing pools or lake basins.
    #[func]
    fn river_segments(&self, thresh: f64) -> PackedFloat32Array {
        let t = self.ter();
        let depth = self.bio.as_ref().map(|b| b.water.depth.as_slice());
        let lake = self.bio.as_ref().map(|b| b.lakes.cell_lake.as_slice());
        let segs = flow::river_segments(
            &t.hab,
            &t.elev,
            &t.flow.discharge,
            &t.flow.down,
            thresh as f32,
            depth,
            0.10,
            lake,
        );
        PackedFloat32Array::from(segs.as_slice())
    }

    /// Shoreline foam seeds near the player: [theta, z, depth, ...]
    #[func]
    fn shore_points(&self, theta: f64, z: f64, radius: f64, limit: i64) -> PackedFloat32Array {
        let mut out = Vec::new();
        let Some(bio) = self.bio.as_ref() else {
            return PackedFloat32Array::from(out.as_slice());
        };
        let t = self.ter();
        let th = theta as f32;
        let zz = z as f32;
        let rad = radius as f32;
        let lim = limit.max(8) as usize;
        let cell_t = std::f32::consts::TAU * t.hab.radius / terrain::NT as f32;
        let cell_z = t.hab.length / terrain::NZ as f32;
        let ti0 = (th.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU
            * terrain::NT as f32)
            .round() as i32;
        let zi0 = ((zz / t.hab.length + 0.5) * terrain::NZ as f32).round() as i32;
        let rt = ((rad / cell_t).ceil() as i32).clamp(4, 80);
        let rz = ((rad / cell_z).ceil() as i32).clamp(4, 80);
        let step = ((rt + rz) / 40).max(1);
        for dz in (-rz..=rz).step_by(step as usize) {
            let zi = zi0 + dz;
            if zi < 1 || zi >= terrain::NZ as i32 - 1 {
                continue;
            }
            for dt in (-rt..=rt).step_by(step as usize) {
                let ti = (ti0 + dt).rem_euclid(terrain::NT as i32) as usize;
                let i = terrain::idx(ti, zi as usize);
                let d = bio.water.depth[i];
                // Prefer the thin contact band the depth-shore foam paints.
                if d < 0.10 || d > 1.1 {
                    continue;
                }
                // Edge: a dry neighbour.
                let mut edge = false;
                for &(adt, adz) in &[(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                    let nt = (ti as i32 + adt).rem_euclid(terrain::NT as i32) as usize;
                    let nz = zi + adz;
                    if nz < 0 || nz >= terrain::NZ as i32 {
                        continue;
                    }
                    if bio.water.depth[terrain::idx(nt, nz as usize)] < 0.08 {
                        edge = true;
                        break;
                    }
                }
                if !edge {
                    continue;
                }
                let oth = ti as f32 / terrain::NT as f32 * std::f32::consts::TAU;
                let oz = (zi as f32 / terrain::NZ as f32 - 0.5) * t.hab.length;
                out.extend_from_slice(&[oth, oz, d]);
                if out.len() / 3 >= lim {
                    break;
                }
            }
            if out.len() / 3 >= lim {
                break;
            }
        }
        PackedFloat32Array::from(out.as_slice())
    }

    /// The whole drum unrolled into an image: circumference across, length
    /// down. Generated once from the same fields the sim runs on, so the map
    /// cannot drift from the world it describes.
    #[func]
    fn biome_map(&self, nt: i64, nz: i64) -> PackedByteArray {
        let t = self.ter();
        let h = t.hab;
        let (nt, nz) = (nt.max(8) as usize, nz.max(8) as usize);
        let mut out = Vec::with_capacity(nt * nz * 3);
        for zi in 0..nz {
            let z = (zi as f32 / nz as f32 - 0.5) * h.length * 0.995;
            for ti in 0..nt {
                let th = ti as f32 / nt as f32 * std::f32::consts::TAU;
                let e = t.elevation(th, z);
                let fx = t.water_flux(th, z);
                let d = 3.0;
                let slope = (t.elevation(th + d / h.radius, z) - t.elevation(th - d / h.radius, z))
                    .abs()
                    + (t.elevation(th, z + d) - t.elevation(th, z - d)).abs();
                // Hillshade from the elevation gradient, lit from -z. Without
                // it the map is a flat colour key and you cannot read terrain.
                let gx = t.elevation(th + d / h.radius, z) - t.elevation(th - d / h.radius, z);
                let gz = t.elevation(th, z + d) - t.elevation(th, z - d);
                let relief = ((-gz * 0.55 - gx * 0.35) / 6.0).clamp(-0.45, 0.45);
                let shade = (0.80 + 0.20 * (e / h.max_elevation).clamp(0.0, 1.0)) * (1.0 + relief);
                let (r, g, b) = if e < h.water_level || t.in_lake(th, z) {
                    (0.13, 0.30, 0.44)
                } else if slope > 6.5 {
                    (0.46 * shade, 0.44 * shade, 0.40 * shade)
                } else if e > 150.0 {
                    (0.64 * shade, 0.60 * shade, 0.40 * shade)
                } else if fx > 0.55 {
                    // Live channel — brighter blue so reroutes read on the map.
                    (0.18 * shade, 0.36 * shade, 0.52 * shade)
                } else if fx > 0.40 {
                    (0.44 * shade, 0.34 * shade, 0.19 * shade)
                } else {
                    (0.26 * shade, 0.50 * shade, 0.21 * shade)
                };
                out.push((r.clamp(0.0, 1.0) * 255.0) as u8);
                out.push((g.clamp(0.0, 1.0) * 255.0) as u8);
                out.push((b.clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
        PackedByteArray::from(out.as_slice())
    }

    /// Biome ids for tint maps: one u8 per unrolled cell.
    #[func]
    fn biome_id_map(&self, nt: i64, nz: i64) -> PackedByteArray {
        let t = self.ter();
        let h = t.hab;
        let (nt, nz) = (nt.max(8) as usize, nz.max(8) as usize);
        let mut out = Vec::with_capacity(nt * nz);
        for zi in 0..nz {
            let z = (zi as f32 / nz as f32 - 0.5) * h.length * 0.995;
            for ti in 0..nt {
                let th = ti as f32 / nt as f32 * std::f32::consts::TAU;
                let bid = if let Some(bio) = self.bio.as_ref() {
                    bio.biome_at(t, th, z)
                } else {
                    Self::fallback_biome_id(t, th, z)
                };
                out.push(bid);
            }
        }
        PackedByteArray::from(out.as_slice())
    }

    /// Ground-cover placements around a point, as one flat buffer.
    ///
    /// Deliberately ONE call: three thousand separate `ground_radius` calls
    /// across the FFI boundary would cost more than the grass is worth.
    /// Returns [theta, z, r, scale, lush, biome_id] per tuft (Wave 1.2).
    #[func]
    fn grass_field(&self, theta: f64, z: f64, radius: f64, max_n: i64) -> PackedFloat32Array {
        let t = self.ter();
        let h = t.hab;
        let (tc, zc, rad) = (theta as f32, z as f32, radius as f32);
        let want = max_n.clamp(0, 8000) as usize;
        if want == 0 {
            return PackedFloat32Array::new();
        }

        let side = (want as f32).sqrt().ceil() as i32;
        let step = (rad * 2.0) / side as f32;
        let mut out: Vec<f32> = Vec::with_capacity(want * 6);
        let water_r = h.radius - h.water_level;

        for a in 0..side {
            for b in 0..side {
                let gx = ((tc * h.radius / step).floor() as i32) + a - side / 2;
                let gz = ((zc / step).floor() as i32) + b - side / 2;
                let hh = {
                    let mut x =
                        (gx as u32).wrapping_mul(0x9E3779B1) ^ (gz as u32).wrapping_mul(0x85EBCA77);
                    x ^= x >> 15;
                    x = x.wrapping_mul(0x2C1B3C6D);
                    x ^= x >> 13;
                    x
                };
                let j0 = ((hh & 0xFFFF) as f32 / 65535.0 - 0.5) * step * 0.9;
                let j1 = (((hh >> 16) & 0xFFFF) as f32 / 65535.0 - 0.5) * step * 0.9;
                let arc = gx as f32 * step + j0;
                let zz = gz as f32 * step + j1;
                if zz.abs() > h.length * 0.49 {
                    continue;
                }
                let th = arc / h.radius;

                let surf = t.surface_radius(th, zz);
                if surf > water_r {
                    continue;
                }
                let e = 2.2;
                let slope =
                    (t.elevation(th + e / h.radius, zz) - t.elevation(th - e / h.radius, zz)).abs()
                        + (t.elevation(th, zz + e) - t.elevation(th, zz - e)).abs();
                if slope > 4.2 {
                    continue;
                }
                let elev = h.radius - surf;
                if elev > 165.0 {
                    continue;
                }

                let bid = if let Some(bio) = self.bio.as_ref() {
                    bio.biome_at(t, th, zz)
                } else {
                    biome::id::GRASSLAND
                };
                // Bare rock / open water: no grass.
                if bid == biome::id::BARE_ROCK || bid == biome::id::WATER {
                    continue;
                }

                let flux = t.water_flux(th, zz);
                let mut lush = (0.35 + flux * 1.25).clamp(0.0, 1.0);
                // Biome density bias.
                lush *= match bid {
                    biome::id::WETLAND | biome::id::RIPARIAN => 1.30,
                    biome::id::SWAMP => 1.40,
                    biome::id::MEADOW => 1.20,
                    biome::id::FOREST => 0.50,
                    biome::id::SCRUB | biome::id::ALPINE => 0.38,
                    biome::id::DESERT => 0.12,
                    biome::id::DUNE => 0.06,
                    biome::id::SHORE => 0.35,
                    biome::id::FARM => 0.85,
                    _ => 1.0,
                };
                let pw = province::province_at(&h, th, zz);
                // Farm rings stay grassy; city pads sparse street lawn.
                lush *= 1.0 + 0.25 * pw.weight(province::id::FARMLAND)
                    - 0.55 * pw.weight(province::id::CITY);
                lush = lush.clamp(0.0, 1.0);
                let keep = ((hh >> 8) & 0xFF) as f32 / 255.0;
                if keep > lush {
                    continue;
                }

                let sc = 0.55 + lush * 0.75 + (((hh >> 20) & 0x3F) as f32 / 63.0) * 0.45;
                out.push(th);
                out.push(zz);
                out.push(surf);
                out.push(sc);
                out.push(lush);
                out.push(bid as f32);
                if out.len() / 6 >= want {
                    break;
                }
            }
            if out.len() / 6 >= want {
                break;
            }
        }
        PackedFloat32Array::from(out.as_slice())
    }

    /// Grazers near a point: [theta, z, density, fear, mass_proxy, ...].
    #[func]
    fn grazer_field(&self, theta: f64, z: f64, radius: f64, max_n: i64) -> PackedFloat32Array {
        let t = self.ter();
        let h = t.hab;
        let Some(bio) = self.bio.as_ref() else {
            return PackedFloat32Array::new();
        };
        let (tc, zc, rad) = (theta as f32, z as f32, (radius as f32).max(1.0));
        let want = max_n.clamp(0, 400) as usize;
        if want == 0 {
            return PackedFloat32Array::new();
        }

        let side = ((want as f32) * 2.0).sqrt().ceil().max(1.0) as i32;
        let step = (rad * 2.0) / side as f32;
        let mut out: Vec<f32> = Vec::with_capacity(want * 5);
        let water_r = h.radius - h.water_level;

        for a in 0..side {
            for b in 0..side {
                let gx = ((tc * h.radius / step).floor() as i32) + a - side / 2;
                let gz = ((zc / step).floor() as i32) + b - side / 2;
                let hh = {
                    let mut x = (gx as u32).wrapping_mul(0x9E3779B1)
                        ^ (gz as u32).wrapping_mul(0x85EBCA77)
                        ^ 0x6A09_E667;
                    x ^= x >> 15;
                    x = x.wrapping_mul(0x2C1B3C6D);
                    x ^= x >> 13;
                    x
                };
                let j0 = ((hh & 0xFFFF) as f32 / 65535.0 - 0.5) * step * 0.9;
                let j1 = (((hh >> 16) & 0xFFFF) as f32 / 65535.0 - 0.5) * step * 0.9;
                let arc = gx as f32 * step + j0;
                let zz = gz as f32 * step + j1;
                if zz.abs() > h.length * 0.49 {
                    continue;
                }
                let dth = (arc - tc * h.radius).abs();
                let dz = zz - zc;
                if dth * dth + dz * dz > rad * rad {
                    continue;
                }

                let th = arc / h.radius;
                if t.surface_radius(th, zz) > water_r {
                    continue;
                }
                let density = bio.trophic.grazer_at(th, zz, h.length).clamp(0.0, 1.0);
                let fear = bio.trophic.fear_at(th, zz, h.length).clamp(0.0, 1.0);
                if density < 0.06 || fear > 0.62 {
                    continue;
                }
                let keep = ((hh >> 8) & 0xFF) as f32 / 255.0;
                if keep > (density * (1.0 - fear * 0.8)).clamp(0.0, 1.0) {
                    continue;
                }

                let npp = bio.trophic.npp_at(th, zz, h.length).clamp(0.0, 3000.0);
                let mass_proxy = (2.0 + npp / 3000.0 * 80.0) * (0.5 + density * 0.8);
                out.extend_from_slice(&[th, zz, density, fear, mass_proxy]);
                if out.len() / 5 >= want {
                    break;
                }
            }
            if out.len() / 5 >= want {
                break;
            }
        }
        PackedFloat32Array::from(out.as_slice())
    }

    #[func]
    fn set_threaded_meshing(&mut self, on: bool) {
        self.threaded_meshing = on;
        godot::global::godot_print!("[rama] threaded meshing: {}", on);
    }

    #[func]
    fn threaded_meshing(&self) -> bool {
        self.threaded_meshing
    }

    /// Threads to spread one chunk over. One = everything on the calling thread.
    fn workers(&self) -> usize {
        if self.threaded_meshing {
            chunker::worker_count()
        } else {
            1
        }
    }

    /// Smoke that `std::thread` inside the GDExtension does not SIGKILL the
    /// process (LANDSCAPE_3200 Wave 1.6). Exit-137 historically was codesign.
    #[func]
    fn selftest_threaded(&self) -> bool {
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut s = 0u64;
            for i in 0..50_000u64 {
                s = s.wrapping_add(i.wrapping_mul(0x9E37));
            }
            let _ = tx.send(s);
        });
        match (handle.join(), rx.recv()) {
            (Ok(_), Ok(v)) => v != 0,
            _ => false,
        }
    }

    /// Where standing water actually exists, as an R8 coverage mask.
    ///
    /// The legacy global water cylinder draws a waterline sheet across the
    /// ENTIRE drum, so every square metre below the waterline reads as ocean
    /// whether or not a basin holds water there. Looking across the habitat
    /// that paints half the far side blue. This mask is what the sheet should
    /// have been clipped by all along.
    #[func]
    fn lake_mask(&self, nt: i64, nz: i64) -> PackedByteArray {
        let t = self.ter();
        let h = t.hab;
        let (nt, nz) = (nt.max(8) as usize, nz.max(8) as usize);
        let water_r = h.radius - h.water_level;
        let mut raw = vec![0u8; nt * nz];
        for zi in 0..nz {
            let z = (zi as f32 / nz as f32 - 0.5) * h.length * 0.999;
            for ti in 0..nt {
                let th = ti as f32 / nt as f32 * std::f32::consts::TAU;
                // Two independent authorities: the flow fill's lake flag, and
                // the plain test that the ground sits below the waterline.
                let wet = t.in_lake(th, z) || t.surface_radius(th, z) > water_r;
                if wet {
                    raw[ti + zi * nt] = 255;
                }
            }
        }
        // One box blur so the shore fades instead of stepping at cell edges.
        let mut out = vec![0u8; nt * nz];
        for zi in 0..nz {
            for ti in 0..nt {
                let mut acc = 0u32;
                let mut n = 0u32;
                for dz in -1i32..=1 {
                    let z2 = zi as i32 + dz;
                    if z2 < 0 || z2 >= nz as i32 {
                        continue;
                    }
                    for dt in -1i32..=1 {
                        let t2 = (ti as i32 + dt).rem_euclid(nt as i32) as usize;
                        acc += raw[t2 + z2 as usize * nt] as u32;
                        n += 1;
                    }
                }
                out[ti + zi * nt] = (acc / n.max(1)) as u8;
            }
        }
        PackedByteArray::from(out.as_slice())
    }

    // ----------------------------------------------------------- survey --

    /// Classify the whole drum into biomes. This is the habitat "reading" its
    /// own state — the same numbers the sim runs on, not a decorative legend.
    #[func]
    fn biome_census(&self, samples: i64) -> Dictionary {
        let t = self.ter();
        let h = t.hab;
        let n = samples.max(8) as usize;
        let mut counts = [0u32; 14];
        let (mut e_sum, mut e_min, mut e_max) = (0.0f64, f32::MAX, f32::MIN);
        let mut total = 0u32;
        for zi in 0..n {
            let z = (zi as f32 / n as f32 - 0.5) * h.length * 0.985;
            for ti in 0..n {
                let th = ti as f32 / n as f32 * std::f32::consts::TAU;
                let e = t.elevation(th, z);
                total += 1;
                e_sum += e as f64;
                e_min = e_min.min(e);
                e_max = e_max.max(e);
                let bid = if let Some(bio) = self.bio.as_ref() {
                    bio.biome_at(t, th, z)
                } else {
                    Self::fallback_biome_id(t, th, z)
                };
                let bi = (bid as usize).min(counts.len() - 1);
                counts[bi] += 1;
            }
        }
        let f = |c: u32| (c as f64) / (total.max(1) as f64);
        let area = std::f64::consts::TAU * h.radius as f64 * h.length as f64;
        let frac = |id: u8| f(counts[id as usize]);
        let wetland = frac(biome::id::WETLAND) + frac(biome::id::SWAMP);
        let riparian = frac(biome::id::RIPARIAN);
        let grassland = frac(biome::id::GRASSLAND) + frac(biome::id::MEADOW);
        let scrub = frac(biome::id::SCRUB) + frac(biome::id::DESERT) + frac(biome::id::DUNE);
        let alpine = frac(biome::id::ALPINE);
        let bare_rock = frac(biome::id::BARE_ROCK);
        let farm = frac(biome::id::FARM);
        let mut d = Dictionary::new();
        for id in 0u8..=13 {
            let key = if id == biome::id::BARE_ROCK {
                "bare_rock"
            } else {
                biome::name(id)
            };
            let _ = d.insert(key, frac(id));
        }
        let _ = d.insert("alluvial", riparian + wetland);
        let _ = d.insert("grass", grassland + farm);
        let _ = d.insert("upland", alpine + scrub);
        let _ = d.insert("rock", bare_rock);
        let (below, _, _, peak) = t.hypsometry();
        let _ = d.insert("hypso_below_water", below as f64);
        let _ = d.insert("hypso_peak", peak as f64);
        let _ = d.insert("elev_hash", t.elev_hash() as i64);
        let pc = province::census(&h, 48, 24);
        for i in 0..province::id::COUNT {
            let _ = d.insert(format!("prov_{}", province::name(i as u8)), pc[i] as i64);
        }
        let _ = d.insert("mean_elevation", e_sum / total.max(1) as f64);
        let _ = d.insert("min_elevation", e_min as f64);
        let _ = d.insert("max_elevation", e_max as f64);
        let _ = d.insert("surface_area_km2", area / 1.0e6);
        let _ = d.insert(
            "arable_km2",
            area * (grassland + farm + riparian + wetland) / 1.0e6,
        );
        d
    }

    /// High-resolution near-field chunk WITH caves. See `chunker::chunk_mesh`.
    #[func]
    fn chunk_mesh_at(&self, ci: i64, cj: i64) -> Dictionary {
        let workers = self.workers();
        let mut m = chunker::chunk_mesh(self.ter(), ci, cj, workers);
        // Paint BEFORE flat-shading. Painting costs more than meshing does, and
        // flat shading triples the vertex count — colouring afterwards paid for
        // every corner of every triangle instead of every surface point once.
        let (t, bio) = (self.ter(), self.bio.as_ref());
        let span = (CHUNK_N as f32 * 0.5 + chunker::AO_PAD as f32) * LATTICE_CELL;
        let (th_c, z_c) = chunker::chunk_centre(t, ci, cj);
        // Ecotone edges are probed 8 m out from each vertex, so the biome window
        // has to reach past the chunk by that much.
        let half = span + 10.0;
        m.cols = vec![[0.0f32; 3]; m.verts.len()];
        let paint_run = |out: &mut [[f32; 3]], from: usize| {
            let mut win = paint::BiomeWindow::new(t.hab.radius, th_c, z_c, half);
            for (i, c) in out.iter_mut().enumerate() {
                *c = paint::vertex_color(t, bio, Some(&mut win), m.verts[from + i], m.normals[from + i]);
            }
        };
        if workers > 1 && m.verts.len() > 512 {
            let per = m.verts.len().div_ceil(workers);
            std::thread::scope(|sc| {
                for (w, block) in m.cols.chunks_mut(per).enumerate() {
                    let paint_run = &paint_run;
                    sc.spawn(move || paint_run(block, w * per));
                }
            });
        } else {
            paint_run(&mut m.cols, 0);
        }
        self.pack(mesher::flat_shade(m))
    }

    /// A mid-detail heightfield window around the player.
    ///
    /// The second LOD tier. Near chunks give 1.4 m detail out to ~275 m; the
    /// far field is 6-10 m cells across the whole drum. Everything between was
    /// therefore flat. This covers that band at ~2.5 m in ONE mesh rather than
    /// hundreds more surface-nets chunks.
    #[func]
    fn mid_mesh(&self, theta_c: f64, z_c: f64, span: f64, n: i64) -> Dictionary {
        let t = self.ter();
        let h = t.hab;
        let (tc, zc, span) = (theta_c as f32, z_c as f32, span as f32);
        let n = n.clamp(16, 512) as usize;
        let step = span / n as f32;
        let half = span * 0.5;

        let mut m = mesher::Mesh::empty();
        // Vertices on a regular (arc, z) grid, projected onto the drum.
        for j in 0..=n {
            let z = zc - half + j as f32 * step;
            let zc_ok = z.abs() <= h.length * 0.5;
            for i in 0..=n {
                let arc = -half + i as f32 * step;
                let th = tc + arc / h.radius;
                let r = if zc_ok {
                    t.surface_radius(th, z) + 0.22
                } else {
                    h.radius
                };
                let p = h.to_world(th, z, r);
                m.verts.push(p);

                // Normal from the elevation gradient, same construction the far
                // field uses so the two tiers shade identically at the handoff.
                let e = step * 0.75;
                let dt = (t.surface_radius(th + e / h.radius, z)
                    - t.surface_radius(th - e / h.radius, z))
                    / (2.0 * e);
                let dz = (t.surface_radius(th, z + e) - t.surface_radius(th, z - e)) / (2.0 * e);
                let up = h.up_at(p);
                let tang = [-th.sin(), th.cos(), 0.0];
                let mut nn = [0.0f32; 3];
                for k in 0..3 {
                    nn[k] = up[k] + tang[k] * dt;
                }
                nn[2] += dz;
                let mag = (nn[0] * nn[0] + nn[1] * nn[1] + nn[2] * nn[2])
                    .sqrt()
                    .max(1e-6);
                m.normals.push([nn[0] / mag, nn[1] / mag, nn[2] / mag]);
                m.ao.push(1.0);
            }
        }
        let w = n + 1;
        for j in 0..n {
            for i in 0..n {
                let a = (j * w + i) as i32;
                let b = (j * w + i + 1) as i32;
                let c = ((j + 1) * w + i + 1) as i32;
                let d2 = ((j + 1) * w + i) as i32;
                // Same clockwise convention as far_mesh_sector.
                m.indices.extend_from_slice(&[a, b, c, a, c, d2]);
            }
        }
        self.pack(m)
    }

    /// Which chunk a world position falls in.
    #[func]
    fn chunk_index(&self, theta: f64, z: f64) -> Vector2i {
        let h = self.ter().hab;
        let ti = (theta.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU * NT_LAT as f64)
            .floor() as i64;
        let zj = (((z as f32 + h.length * 0.5) / LATTICE_CELL).floor()) as i64;
        Vector2i::new(
            (ti.div_euclid(CHUNK_N as i64)) as i32,
            (zj.div_euclid(CHUNK_N as i64)) as i32,
        )
    }

    #[func]
    fn ground_radius(&self, theta: f64, z: f64) -> f64 {
        self.ter().surface_radius(theta as f32, z as f32) as f64
    }
    #[func]
    fn elevation(&self, theta: f64, z: f64) -> f64 {
        self.ter().elevation(theta as f32, z as f32) as f64
    }
    #[func]
    fn water_flux(&self, theta: f64, z: f64) -> f64 {
        self.ter().water_flux(theta as f32, z as f32) as f64
    }
    #[func]
    fn density_at(&self, p: Vector3) -> f64 {
        self.ter().density([p.x, p.y, p.z]) as f64
    }

    /// Ray-march inward/outward to find the true ground radius including caves.
    #[func]
    fn ground_below(&self, theta: f64, z: f64, from_r: f64) -> f64 {
        let t = self.ter();
        let (th, zz) = (theta as f32, z as f32);
        let mut r = from_r as f32;
        let step = 0.35f32;
        let limit = t.hab.radius;
        while r < limit {
            let p = t.hab.to_world(th, zz, r);
            if t.density(p) > 0.0 {
                return r as f64;
            }
            r += step;
        }
        limit as f64
    }

    /// A spawn site the game can justify: gentle ground, above water, near
    /// where drainage collected — i.e. where a colonist would actually farm.
    #[func]
    fn find_spawn(&self) -> Dictionary {
        let t = self.ter();
        let h = t.hab;
        let (mut best, mut bt, mut bz) = (f32::MIN, 0.0f32, 0.0f32);
        for zi in 0..120 {
            let z = (zi as f32 / 120.0 - 0.5) * h.length * 0.68;
            for ti in 0..320 {
                let th = ti as f32 / 320.0 * std::f32::consts::TAU;
                let e = t.elevation(th, z);
                if e < h.water_level + 4.0 || e > 90.0 {
                    continue;
                }
                let d = 1.4;
                let sl = (t.elevation(th + d / h.radius, z) - t.elevation(th - d / h.radius, z))
                    .abs()
                    + (t.elevation(th, z + d) - t.elevation(th, z - d)).abs();
                let flux = t.water_flux(th, z);
                let meadow = province::province_at(&h, th, z).weight(province::id::MEADOW);
                // Prefer meadow province, gentle farm flats, shy of dunes/seas.
                let score = flux * 2.0 - sl * 3.2 - (e - 36.0).abs() * 0.02 + meadow * 4.5;
                if score > best {
                    best = score;
                    bt = th;
                    bz = z;
                }
            }
        }
        let r = self.ground_below(
            bt as f64,
            bz as f64,
            (h.radius - h.max_elevation - 20.0) as f64,
        );
        let mut d = Dictionary::new();
        let _ = d.insert("theta", bt as f64);
        let _ = d.insert("z", bz as f64);
        let _ = d.insert("radius", r);
        let _ = d.insert("elevation", (h.radius - r as f32) as f64);
        d
    }
}
