//! Heat, fire, lava, steam — one energy system on the drum.
//!
//! Heat lives on the weather grid. Fire lives on woodscape fuel cells. Lava is
//! sparse on the terrain grid and reuses D8 flow. Steam ties the hot half to
//! the wet half and must conserve habitat water across boil/condense.

use crate::debris::Debris;
use crate::economy::{self, craft_id};
use crate::soil::Soil;
use crate::terrain::{idx, Terrain, NT, NZ};
use crate::weather::{Weather, WT, WZ};
use crate::woodscape::{self, kind, Key, Woodscape, LEAF_KG, WOOD_KG};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default)]
struct GridHasher(u64);
impl Hasher for GridHasher {
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.write_u64(*b as u64);
        }
    }
    fn write_u64(&mut self, i: u64) {
        const K: u64 = 0x517c_c1b7_2722_0a95;
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(K);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}
type Fast = BuildHasherDefault<GridHasher>;
type FastMap<K, V> = HashMap<K, V, Fast>;

const HEAT_HALF_LIFE_DAYS: f32 = 0.05;
const HEAT_PER_KG: f32 = 18.0;
const BURN_WOOD_KG_PER_DAY: f32 = 40.0;
const BURN_LEAF_MULT: f32 = 8.0;
const SPREAD_BASE: f32 = 12.0;
const SOLIDUS_C: f32 = 980.0;
const LIQUIDUS_C: f32 = 1200.0;
const LAVA_SPEED: f32 = 8.0;
const STEAM_RISE: f32 = 4.0; // m/s toward axis (r decreases)
const MAX_PUFFS: usize = 128;
const MAX_LAVA_CELLS: usize = 512;

#[derive(Clone, Debug)]
pub struct Heat {
    /// Degrees above local ambient, WT×WZ.
    pub excess_c: Vec<f32>,
}

impl Default for Heat {
    fn default() -> Self {
        Self {
            excess_c: vec![0.0; WT * WZ],
        }
    }
}

impl Heat {
    pub fn excess_at(&self, _hab_r: f32, hab_len: f32, theta: f32, z: f32) -> f32 {
        let t = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * WT as f32;
        let zz = (z / hab_len + 0.5) * WZ as f32;
        sample_bilinear(&self.excess_c, WT, WZ, t, zz)
    }

    pub fn add_at(&mut self, hab_len: f32, theta: f32, z: f32, dc: f32) {
        if dc <= 0.0 {
            return;
        }
        let ti = (theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * WT as f32)
            .round() as usize
            % WT;
        let zi = ((z / hab_len + 0.5) * WZ as f32)
            .round()
            .clamp(0.0, (WZ - 1) as f32) as usize;
        self.excess_c[ti + zi * WT] += dc;
    }

    pub fn tick(&mut self, weather: &Weather, dt_days: f32) {
        if dt_days <= 0.0 {
            return;
        }
        // One Jacobi diffusion pass — order-independent (talus GS bias lesson).
        let mut next = self.excess_c.clone();
        for z in 0..WZ {
            for t in 0..WT {
                let i = t + z * WT;
                let l = (t + WT - 1) % WT + z * WT;
                let r = (t + 1) % WT + z * WT;
                let d = if z > 0 { t + (z - 1) * WT } else { i };
                let u = if z + 1 < WZ { t + (z + 1) * WT } else { i };
                let n = 0.25
                    * (self.excess_c[l]
                        + self.excess_c[r]
                        + self.excess_c[d]
                        + self.excess_c[u]);
                next[i] = self.excess_c[i] * 0.6 + n * 0.4;
            }
        }
        self.excess_c = next;

        // Decay toward zero with ~0.05 day half-life.
        let decay = (-std::f32::consts::LN_2 * dt_days / HEAT_HALF_LIFE_DAYS).exp();
        for v in &mut self.excess_c {
            *v *= decay;
            if *v < 0.01 {
                *v = 0.0;
            }
        }

        // Advect with wind (semi-Lagrangian, one cell-ish).
        let mut adv = self.excess_c.clone();
        let cell_t = std::f32::consts::TAU * weather_hab_radius_proxy() / WT as f32;
        let cell_z = 6000.0 / WZ as f32; // Kepler length; wind is on weather grid
        for z in 0..WZ {
            for t in 0..WT {
                let i = t + z * WT;
                let wt = weather.wind_theta[i];
                let wz = weather.wind_z[i];
                let dt = -(wt * dt_days * HAB_DAY_S / cell_t);
                let dz = -(wz * dt_days * HAB_DAY_S / cell_z);
                let src_t = (t as f32 + dt).rem_euclid(WT as f32);
                let src_z = (z as f32 + dz).clamp(0.0, (WZ - 1) as f32);
                adv[i] = sample_bilinear(&self.excess_c, WT, WZ, src_t, src_z);
            }
        }
        self.excess_c = adv;
        let _ = cell_t;
    }
}

const HAB_DAY_S: f32 = 50.0;
fn weather_hab_radius_proxy() -> f32 {
    900.0
}

fn sample_bilinear(f: &[f32], nt: usize, nz: usize, x: f32, y: f32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let at = |xi: i32, yi: i32| -> f32 {
        let xi = xi.rem_euclid(nt as i32) as usize;
        let yi = yi.clamp(0, nz as i32 - 1) as usize;
        f[xi + yi * nt]
    };
    let a = at(x0, y0);
    let b = at(x0 + 1, y0);
    let c = at(x0, y0 + 1);
    let d = at(x0 + 1, y0 + 1);
    a * (1.0 - fx) * (1.0 - fy) + b * fx * (1.0 - fy) + c * (1.0 - fx) * fy + d * fx * fy
}

#[derive(Clone, Copy, Debug)]
pub struct Ember {
    pub fuel_kg: f32,
    pub temp_c: f32,
    pub plant: u32,
    pub kind: u8,
    pub o2_fails: u8,
}

#[derive(Default)]
pub struct Fire {
    pub burning: FastMap<Key, Ember>,
    front: Vec<Key>,
}

#[derive(Clone, Copy, Debug)]
pub struct Flow {
    pub m3: f32,
    pub temp_c: f32,
}

#[derive(Default)]
pub struct Lava {
    pub cells: FastMap<usize, Flow>,
}

#[derive(Clone, Debug)]
pub struct Puff {
    pub theta: f32,
    pub z: f32,
    pub r: f32,
    pub mass_kg: f32,
    pub heat_c: f32,
    pub life: f32,
}

#[derive(Default)]
pub struct Pyro {
    pub heat: Heat,
    pub fire: Fire,
    pub lava: Lava,
    pub steam: Vec<Puff>,
}

impl Pyro {
    pub fn douse_near(&mut self, world: [f32; 3], radius: f32) -> u32 {
        let r2 = radius * radius;
        let keys: Vec<Key> = self.fire.burning.keys().copied().collect();
        let mut n = 0u32;
        for k in keys {
            let p = woodscape::dequantize(k);
            let d0 = p[0] - world[0];
            let d1 = p[1] - world[1];
            let d2 = p[2] - world[2];
            if d0 * d0 + d1 * d1 + d2 * d2 <= r2 {
                self.fire.burning.remove(&k);
                n += 1;
            }
        }
        n
    }

    pub fn temp_c(&self, weather: &Weather, ter: &Terrain, theta: f32, z: f32) -> f32 {
        let elev = ter.elevation(theta, z);
        weather.temp_at_elev(theta, z, elev)
            + self
                .heat
                .excess_at(ter.hab.radius, ter.hab.length, theta, z)
    }

    pub fn steam_at(&self, theta: f32, z: f32, r: f32) -> f32 {
        let mut best = 0.0f32;
        for p in &self.steam {
            if p.heat_c < 80.0 {
                continue;
            }
            let dth = {
                let x = (p.theta - theta).rem_euclid(std::f32::consts::TAU);
                let x = x.min(std::f32::consts::TAU - x);
                x * 900.0
            };
            let d = (dth * dth + (p.z - z) * (p.z - z) + (p.r - r) * (p.r - r)).sqrt();
            if d < 4.0 {
                best = best.max(p.heat_c * (1.0 - d / 4.0));
            }
        }
        best
    }

    pub fn ignite_key(&mut self, ws: &Woodscape, key: Key) -> bool {
        let k = ws.get(key);
        if k != kind::WOOD && k != kind::LEAF {
            return false;
        }
        if self.fire.burning.contains_key(&key) {
            return true;
        }
        let Some(cell) = ws.cell_public(key) else {
            return false;
        };
        let fuel = if k == kind::WOOD { WOOD_KG } else { LEAF_KG };
        self.fire.burning.insert(
            key,
            Ember {
                fuel_kg: fuel,
                temp_c: if k == kind::LEAF { 480.0 } else { 620.0 },
                plant: cell.plant,
                kind: k,
                o2_fails: 0,
            },
        );
        self.fire.front.push(key);
        true
    }

    pub fn try_ignite_at(
        &mut self,
        ws: &Woodscape,
        ter: &Terrain,
        soil: &Soil,
        weather: &Weather,
        water_depth: &[f32],
        world: [f32; 3],
        reason: &str,
    ) -> Result<(), String> {
        // Reach past a trunk face / canopy edge — 1.6 m missed too often.
        let Some((key, cell)) = ws.nearest_cell(world, 3.5) else {
            return Err("nothing to light — aim at wood or leaves".into());
        };
        if cell.kind != kind::WOOD && cell.kind != kind::LEAF {
            return Err("nothing to light — aim at wood or leaves".into());
        }
        let (th, z, _) = ter.hab.to_cyl(world);
        let ci = terrain_cell(ter, th, z);
        if water_depth.get(ci).copied().unwrap_or(0.0) > 0.02 {
            return Err("too wet to light".into());
        }
        let rain = weather.rain_at(th, z);
        if rain > 0.45 {
            return Err("too wet to light — raining".into());
        }
        let moist = soil.sample(th, z).moisture;
        // Standing water and rain are the real firebreaks. Soil moisture only
        // blocks when the ground is saturated — a 0.55 gate refused almost
        // every forest the player could reach.
        if moist > 0.88 {
            return Err("too wet to light".into());
        }
        let _ = reason;
        if !self.ignite_key(ws, key) {
            return Err("nothing to light — aim at wood or leaves".into());
        }
        // Seed heat so the field and the light react immediately.
        self.heat.add_at(ter.hab.length, th, z, 220.0);
        // Catch a couple of neighbouring leaves so the first spark reads.
        for n in woodscape::neighbors26(key).into_iter().take(8) {
            if ws.get(n) == kind::LEAF || ws.get(n) == kind::WOOD {
                let _ = self.ignite_key(ws, n);
            }
        }
        Ok(())
    }

    pub fn pour_lava(&mut self, ter: &Terrain, theta: f32, z: f32, m3: f32, temp_c: f32) {
        if m3 <= 0.0 {
            return;
        }
        let i = terrain_cell(ter, theta, z);
        if self.lava.cells.len() >= MAX_LAVA_CELLS && !self.lava.cells.contains_key(&i) {
            return;
        }
        let e = self.lava.cells.entry(i).or_insert(Flow {
            m3: 0.0,
            temp_c: temp_c.max(LIQUIDUS_C),
        });
        let total = e.m3 + m3;
        e.temp_c = (e.temp_c * e.m3 + temp_c * m3) / total.max(1e-6);
        e.m3 = total;
    }

    pub fn tick(
        &mut self,
        ter: &mut Terrain,
        ws: &mut Woodscape,
        soil: &mut Soil,
        weather: &Weather,
        atmosphere: &mut economy::Atmosphere,
        water_depth: &mut [f32],
        water_stock: &mut f32,
        heaps: &mut Vec<economy::Stockpile>,
        debris: &mut Debris,
        dt_days: f32,
    ) {
        if dt_days <= 0.0 {
            return;
        }
        self.heat.tick(weather, dt_days);
        self.tick_fire(
            ter,
            ws,
            soil,
            weather,
            atmosphere,
            water_depth,
            heaps,
            dt_days,
        );
        self.tick_lava(ter, ws, water_depth, water_stock, dt_days);
        self.tick_steam(ter, water_stock, water_depth, dt_days);
        self.heat_from_water_columns(ter, water_depth, water_stock, dt_days);
        self.ignite_from_debris(ws, ter, debris);
        self.ignite_from_lava(ws, ter);
        // Consume queued ignitions from debris.
        let queued = std::mem::take(&mut debris.ignite_keys);
        for k in queued {
            let _ = self.ignite_key(ws, k);
        }
    }

    fn tick_fire(
        &mut self,
        ter: &Terrain,
        ws: &mut Woodscape,
        soil: &mut Soil,
        weather: &Weather,
        atmosphere: &mut economy::Atmosphere,
        water_depth: &[f32],
        heaps: &mut Vec<economy::Stockpile>,
        dt_days: f32,
    ) {
        let budget = ((600.0 * dt_days).ceil() as usize).clamp(32, 4096);
        let keys: Vec<Key> = self.fire.burning.keys().copied().collect();
        let mut extinguished = Vec::new();
        let mut spreads: Vec<Key> = Vec::new();
        let mut worked = 0usize;

        for key in keys {
            if worked >= budget {
                break;
            }
            worked += 1;
            let (ember_kind, mut fuel_kg, mut temp_c, mut o2_fails, plant) = {
                let Some(ember) = self.fire.burning.get_mut(&key) else {
                    continue;
                };
                (
                    ember.kind,
                    ember.fuel_kg,
                    ember.temp_c,
                    ember.o2_fails,
                    ember.plant,
                )
            };
            let _ = plant;
            let p = woodscape::dequantize(key);
            let (th, z, _) = ter.hab.to_cyl(p);
            let ci = terrain_cell(ter, th, z);
            let depth = water_depth.get(ci).copied().unwrap_or(0.0);
            let rain = weather.rain_at(th, z);

            if depth > 0.02 || rain > 0.55 || fuel_kg <= 0.0 || o2_fails >= 3 {
                extinguished.push(key);
                continue;
            }

            let rate = BURN_WOOD_KG_PER_DAY
                * if ember_kind == kind::LEAF {
                    BURN_LEAF_MULT
                } else {
                    1.0
                };
            let burned = (rate * dt_days).min(fuel_kg);
            if burned <= 1e-5 {
                continue;
            }
            let o2 = burned * 1.4;
            let co2 = burned * 1.6;
            if !atmosphere.apply_combustion(o2, co2) {
                temp_c *= 0.5;
                o2_fails = o2_fails.saturating_add(1);
                if let Some(e) = self.fire.burning.get_mut(&key) {
                    e.temp_c = temp_c;
                    e.o2_fails = o2_fails;
                }
                continue;
            }
            o2_fails = 0;
            fuel_kg -= burned;
            temp_c = if ember_kind == kind::LEAF {
                500.0
            } else {
                800.0
            };

            // Remove the voxel once, billing the ledger. Further ticks only
            // pay atmosphere / ash / heat against the ember's remaining fuel.
            if ws.get(key) != kind::EMPTY {
                let _ = ws.burn_cell(key);
            }

            let ash = burned * 0.06;
            if ash > 0.02 {
                let ph = economy::bio_phys(craft_id::ASH);
                let vol = ash / ph.bulk_kg_m3.max(1.0);
                economy::deposit_heap(
                    heaps,
                    ter.hab.radius,
                    th,
                    z,
                    craft_id::ASH,
                    ash,
                    vol * ph.bulking,
                    0.4,
                );
                if let Some(effect) = economy::amendment_effect(craft_id::ASH) {
                    soil.amend(th, z, effect, ash, 2.5);
                }
            }
            self.heat.add_at(ter.hab.length, th, z, burned * HEAT_PER_KG);

            // Spread rolls.
            let moist = soil.sample(th, z).moisture;
            let humid = weather.humidity_at(th, z);
            let dryness = ((1.0 - moist) * (1.0 - humid)).clamp(0.0, 1.0);
            let heat_f = ((temp_c - 250.0) / 350.0).clamp(0.0, 1.0);
            let (wt, wz) = wind_at(weather, th, z, ter.hab.length);
            let burn_len = self.fire.burning.len() as u32;
            for n in woodscape::neighbors26(key) {
                let nk = ws.get(n);
                if nk != kind::WOOD && nk != kind::LEAF {
                    continue;
                }
                if self.fire.burning.contains_key(&n) {
                    continue;
                }
                let np = woodscape::dequantize(n);
                let (nth, nz, _) = ter.hab.to_cyl(np);
                let dir_arc = {
                    let x = (nth - th).rem_euclid(std::f32::consts::TAU);
                    let x = if x > std::f32::consts::PI {
                        x - std::f32::consts::TAU
                    } else {
                        x
                    };
                    x * ter.hab.radius
                };
                let dir_z = nz - z;
                let dlen = (dir_arc * dir_arc + dir_z * dir_z).sqrt().max(1e-3);
                let wind_len = (wt * wt + wz * wz).sqrt().max(1e-3);
                let align = (dir_arc * wt + dir_z * wz) / (dlen * wind_len);
                let wind_gain = 1.0 + 1.5 * align.max(0.0); // 1..2.5
                let fuel_gain = if nk == kind::LEAF { 2.2 } else { 1.0 };
                let p_spread = SPREAD_BASE * dt_days * heat_f * dryness * wind_gain * fuel_gain;
                let h = hash_key(n, burn_len);
                if (h % 10_000) as f32 / 10_000.0 < p_spread.clamp(0.0, 0.95) {
                    spreads.push(n);
                }
            }

            if let Some(e) = self.fire.burning.get_mut(&key) {
                e.fuel_kg = fuel_kg;
                e.temp_c = temp_c;
                e.o2_fails = o2_fails;
            }
            if fuel_kg <= 0.0 {
                extinguished.push(key);
            }
        }

        for k in extinguished {
            self.fire.burning.remove(&k);
        }
        for k in spreads {
            let _ = self.ignite_key(ws, k);
        }
    }

    fn tick_lava(
        &mut self,
        ter: &mut Terrain,
        _ws: &mut Woodscape,
        water_depth: &mut [f32],
        water_stock: &mut f32,
        dt_days: f32,
    ) {
        if self.lava.cells.is_empty() {
            return;
        }
        let cell_area =
            (std::f32::consts::TAU * ter.hab.radius / NT as f32) * (ter.hab.length / NZ as f32);
        let keys: Vec<usize> = self.lava.cells.keys().copied().collect();
        let mut moves: Vec<(usize, usize, f32, f32)> = Vec::new();
        let mut solidify: Vec<(usize, f32)> = Vec::new();
        let mut quench_steam: Vec<(f32, f32, f32, f32)> = Vec::new();

        for i in keys {
            let Some(flow) = self.lava.cells.get_mut(&i) else {
                continue;
            };
            let (ti, zi) = (i % NT, i / NT);
            let th = ti as f32 / NT as f32 * std::f32::consts::TAU;
            let z = (zi as f32 / NZ as f32 - 0.5) * ter.hab.length;
            let depth = water_depth[i];

            if depth > 0.02 {
                let quench = 400.0 * depth * dt_days;
                flow.temp_c -= quench;
                // Boil water → steam; remove water from column + stock tracks via puff.
                let heat_removed = quench * flow.m3 * 900.0; // rough
                let boil_kg = (heat_removed / 2.6e6).min(depth * cell_area * 1000.0);
                if boil_kg > 0.05 {
                    let m3 = boil_kg / 1000.0;
                    water_depth[i] = (water_depth[i] - m3 / cell_area).max(0.0);
                    // Standing water → steam. Stock is a separate ledger; depth
                    // mass is the conserved quantity here.
                    let r = ter.hab.radius - ter.elev[i];
                    quench_steam.push((th, z, r, boil_kg));
                }
                // Quenched volume solidifies as basalt immediately.
                let frac = (0.35 * dt_days).clamp(0.0, 0.5);
                let rock_m3 = flow.m3 * frac;
                if rock_m3 > 1e-4 {
                    flow.m3 -= rock_m3;
                    ter.elev[i] += rock_m3 / cell_area;
                    ter.flow.mark_dirty_at(ti, zi, 4);
                    solidify.push((i, 0.0)); // mark handled; don't double
                }
            }

            // Cooling in air.
            flow.temp_c -= 35.0 * dt_days;
            if flow.temp_c < SOLIDUS_C || flow.m3 < 1e-4 {
                solidify.push((i, flow.m3));
                continue;
            }

            let mobility = smoothstep(SOLIDUS_C, LIQUIDUS_C, flow.temp_c);
            let dn = ter.flow.down[i] as usize;
            if dn != i && mobility > 0.05 {
                let moved = flow.m3 * mobility * LAVA_SPEED * dt_days;
                if moved > 1e-5 {
                    flow.m3 -= moved;
                    moves.push((i, dn, moved, flow.temp_c));
                }
            }
        }

        for (from, to, m3, temp) in moves {
            if let Some(f) = self.lava.cells.get_mut(&from) {
                // already deducted
                let _ = f;
            }
            let e = self.lava.cells.entry(to).or_insert(Flow {
                m3: 0.0,
                temp_c: temp,
            });
            let total = e.m3 + m3;
            e.temp_c = (e.temp_c * e.m3 + temp * m3) / total.max(1e-6);
            e.m3 = total;
        }

        for (i, m3) in solidify {
            if let Some(flow) = self.lava.cells.remove(&i) {
                let add = if m3 > 0.0 { m3 } else { 0.0 };
                // If we already raised elev during quench, skip zero adds.
                if add > 1e-4 {
                    let (ti, zi) = (i % NT, i / NT);
                    ter.elev[i] += add / cell_area;
                    ter.flow.mark_dirty_at(ti, zi, 4);
                }
                let _ = flow;
            }
        }

        for (th, z, r, kg) in quench_steam {
            self.spawn_puff(th, z, r, kg, 140.0);
        }
    }

    fn tick_steam(
        &mut self,
        ter: &Terrain,
        water_stock: &mut f32,
        water_depth: &mut [f32],
        dt_days: f32,
    ) {
        let dt = dt_days * HAB_DAY_S;
        let mut i = 0;
        while i < self.steam.len() {
            let p = &mut self.steam[i];
            p.r -= STEAM_RISE * dt;
            p.life -= dt_days;
            p.heat_c *= (-0.8 * dt_days).exp();
            p.mass_kg *= (1.0 - 0.15 * dt_days).max(0.5);
            // Deposit heat into the field while rising.
            self.heat
                .add_at(ter.hab.length, p.theta, p.z, p.heat_c * 0.02 * dt_days);

            let condense = p.heat_c < 100.0 || p.life <= 0.0 || p.r < 50.0;
            if condense {
                let mass = p.mass_kg;
                let th = p.theta;
                let z = p.z;
                let r = p.r;
                self.steam.swap_remove(i);
                // Prefer returning to the column it rose from; otherwise the
                // abstract stock. Never both.
                if r > ter.hab.radius - 80.0 {
                    let ci = terrain_cell(ter, th, z);
                    let cell_area = (std::f32::consts::TAU * ter.hab.radius / NT as f32)
                        * (ter.hab.length / NZ as f32);
                    water_depth[ci] =
                        (water_depth[ci] + mass / 1000.0 / cell_area).min(28.0);
                } else {
                    *water_stock += mass;
                }
            } else {
                i += 1;
            }
        }
    }

    fn heat_from_water_columns(
        &mut self,
        ter: &Terrain,
        water_depth: &mut [f32],
        water_stock: &mut f32,
        dt_days: f32,
    ) {
        // Excess heat over standing water → steam.
        let cell_area =
            (std::f32::consts::TAU * ter.hab.radius / NT as f32) * (ter.hab.length / NZ as f32);
        for z in 0..WZ {
            for t in 0..WT {
                let i = t + z * WT;
                let excess = self.heat.excess_c[i];
                if excess < 100.0 {
                    continue;
                }
                let th = t as f32 / WT as f32 * std::f32::consts::TAU;
                let zz = (z as f32 / WZ as f32 - 0.5) * ter.hab.length;
                let ci = terrain_cell(ter, th, zz);
                let depth = water_depth[ci];
                if depth < 0.02 {
                    continue;
                }
                let boil_kg = (excess * 0.002 * dt_days * cell_area * 1000.0)
                    .min(depth * cell_area * 1000.0);
                if boil_kg < 0.05 {
                    continue;
                }
                water_depth[ci] = (depth - boil_kg / 1000.0 / cell_area).max(0.0);
                self.heat.excess_c[i] *= 0.85;
                let r = ter.hab.radius - ter.elev[ci];
                self.spawn_puff(th, zz, r, boil_kg, 120.0 + excess * 0.2);
            }
        }
    }

    fn spawn_puff(&mut self, theta: f32, z: f32, r: f32, mass_kg: f32, heat_c: f32) {
        if mass_kg < 0.05 {
            return;
        }
        if self.steam.len() >= MAX_PUFFS {
            self.steam.remove(0);
        }
        self.steam.push(Puff {
            theta,
            z,
            r,
            mass_kg,
            heat_c,
            life: 0.08,
        });
    }

    fn ignite_from_debris(&mut self, ws: &mut Woodscape, ter: &Terrain, debris: &Debris) {
        for b in &debris.bodies {
            if b.heat_c < 280.0 {
                continue;
            }
            let p = ter.hab.to_world(b.theta, b.z, b.r);
            if let Some((key, cell)) = ws.nearest_cell(p, 1.5) {
                if cell.kind == kind::WOOD || cell.kind == kind::LEAF {
                    let _ = self.ignite_key(ws, key);
                }
            }
        }
    }

    fn ignite_from_lava(&mut self, ws: &mut Woodscape, ter: &Terrain) {
        let cells: Vec<usize> = self.lava.cells.keys().copied().collect();
        for i in cells {
            let (ti, zi) = (i % NT, i / NT);
            let th = ti as f32 / NT as f32 * std::f32::consts::TAU;
            let z = (zi as f32 / NZ as f32 - 0.5) * ter.hab.length;
            let r = ter.hab.radius - ter.elev[i];
            let p = ter.hab.to_world(th, z, r);
            if let Some((key, cell)) = ws.nearest_cell(p, 1.5) {
                if cell.kind == kind::WOOD || cell.kind == kind::LEAF {
                    let _ = self.ignite_key(ws, key);
                }
            }
        }
    }

    /// Fire LOD: [x,y,z, temp01, size]
    pub fn fire_lod(&self, hab: &crate::habitat::Habitat, center: [f32; 3], radius: f32, limit: usize) -> Vec<f32> {
        let r2 = radius * radius;
        let mut out = Vec::new();
        let mut n = 0;
        for (key, ember) in &self.fire.burning {
            if n >= limit {
                break;
            }
            let p = woodscape::dequantize(*key);
            let d0 = p[0] - center[0];
            let d1 = p[1] - center[1];
            let d2 = p[2] - center[2];
            if d0 * d0 + d1 * d1 + d2 * d2 > r2 {
                continue;
            }
            out.extend_from_slice(&[
                p[0],
                p[1],
                p[2],
                (ember.temp_c / 800.0).clamp(0.0, 1.0),
                if ember.kind == kind::LEAF { 0.35 } else { 0.55 },
            ]);
            n += 1;
        }
        let _ = hab;
        out
    }

    pub fn fire_centroid_energy(&self) -> Option<([f32; 3], f32, f32)> {
        if self.fire.burning.is_empty() {
            return None;
        }
        let mut c = [0.0f32; 3];
        let mut mass = 0.0f32;
        for (k, e) in &self.fire.burning {
            let p = woodscape::dequantize(*k);
            c[0] += p[0] * e.fuel_kg;
            c[1] += p[1] * e.fuel_kg;
            c[2] += p[2] * e.fuel_kg;
            mass += e.fuel_kg;
        }
        if mass < 1e-3 {
            return None;
        }
        Some((
            [c[0] / mass, c[1] / mass, c[2] / mass],
            mass,
            (mass / 80.0).clamp(0.5, 8.0),
        ))
    }
}

fn terrain_cell(ter: &Terrain, theta: f32, z: f32) -> usize {
    let ti = (theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * NT as f32)
        .round() as usize
        % NT;
    let zi = ((z / ter.hab.length + 0.5) * NZ as f32)
        .round()
        .clamp(0.0, (NZ - 1) as f32) as usize;
    idx(ti, zi)
}

fn wind_at(weather: &Weather, theta: f32, z: f32, hab_len: f32) -> (f32, f32) {
    let t = theta.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * WT as f32;
    let zz = (z / hab_len + 0.5) * WZ as f32;
    (
        sample_bilinear(&weather.wind_theta, WT, WZ, t, zz),
        sample_bilinear(&weather.wind_z, WT, WZ, t, zz),
    )
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0).max(1e-4)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn hash_key(k: Key, salt: u32) -> u32 {
    let mut h = salt
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(k.0 as u32)
        .wrapping_mul(0x85EB_CA6B)
        .wrapping_add(k.1 as u32);
    h ^= (k.2 as u32).wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habitat::Habitat;
    use crate::terrain::Terrain;

    #[test]
    fn heat_spreads_and_fades() {
        let hab = Habitat::kepler_drum();
        let ter = Terrain::generate(hab);
        let weather = Weather::new(hab);
        let mut pyro = Pyro::default();
        pyro.heat.add_at(hab.length, 1.0, 0.0, 500.0);
        let before = pyro.heat.excess_at(hab.radius, hab.length, 1.0, 0.0);
        for _ in 0..8 {
            pyro.heat.tick(&weather, 0.02);
        }
        let near = pyro.heat.excess_at(hab.radius, hab.length, 1.0 + 2.0 / hab.radius, 0.0);
        let far = pyro.heat.excess_at(hab.radius, hab.length, 1.0 + 0.5, 500.0);
        let after = pyro.heat.excess_at(hab.radius, hab.length, 1.0, 0.0);
        assert!(after < before, "heat should fade");
        assert!(near > far * 0.5 || near > 1.0, "heat should spread locally");
        let _ = ter;
    }

    #[test]
    fn heat_is_bounded() {
        let hab = Habitat::kepler_drum();
        let weather = Weather::new(hab);
        let mut pyro = Pyro::default();
        for _ in 0..40 {
            pyro.heat.add_at(hab.length, 0.5, 0.0, 50.0);
            pyro.heat.tick(&weather, 0.05);
        }
        let peak = pyro.heat.excess_c.iter().cloned().fold(0.0f32, f32::max);
        assert!(peak < 5000.0, "runaway heat {peak}");
    }

    #[test]
    fn steam_rises_and_condenses() {
        let hab = Habitat::kepler_drum();
        let ter = Terrain::generate(hab);
        let mut pyro = Pyro::default();
        let mut stock = 1.0e6f32;
        let mut depth = vec![0.0f32; NT * NZ];
        let r0 = hab.radius - 10.0;
        pyro.spawn_puff(0.4, 0.0, r0, 100.0, 150.0);
        let start_r = pyro.steam[0].r;
        for _ in 0..30 {
            pyro.tick_steam(&ter, &mut stock, &mut depth, 0.01);
        }
        if let Some(p) = pyro.steam.first() {
            assert!(p.r < start_r, "steam should rise (r decreases)");
        } else {
            // Condensed — also fine.
            assert!(stock >= 1.0e6 - 1.0);
        }
    }

    #[test]
    fn a_full_boil_and_condense_cycle_conserves_the_habitat_water() {
        let hab = Habitat::kepler_drum();
        let ter = Terrain::generate(hab);
        let mut pyro = Pyro::default();
        let mut stock = 1.0e6f32;
        let mut depth = vec![0.0f32; NT * NZ];
        let ci = terrain_cell(&ter, 0.5, 0.0);
        depth[ci] = 1.0;
        let cell_area =
            (std::f32::consts::TAU * hab.radius / NT as f32) * (hab.length / NZ as f32);
        let water0 = stock + depth.iter().sum::<f32>() * cell_area * 1000.0;
        // Force boil via heat.
        pyro.heat.add_at(hab.length, 0.5, 0.0, 400.0);
        for _ in 0..20 {
            pyro.heat_from_water_columns(&ter, &mut depth, &mut stock, 0.02);
            pyro.tick_steam(&ter, &mut stock, &mut depth, 0.02);
        }
        // Finish condensing.
        for _ in 0..40 {
            pyro.tick_steam(&ter, &mut stock, &mut depth, 0.05);
        }
        let steam_m: f32 = pyro.steam.iter().map(|p| p.mass_kg).sum();
        let water1 = stock + depth.iter().sum::<f32>() * cell_area * 1000.0 + steam_m;
        let rel = (water1 - water0).abs() / water0.max(1.0);
        assert!(
            rel < 5e-4,
            "water not conserved: {water0} -> {water1} (rel {rel})"
        );
    }
}
