# Falling bodies and the hot half of the world

**Status:** implemented — debris keeps severed voxel shape (rigid cluster), fire on `F`.
**Written:** 2026-09-08. **Updated:** 2026-09-09.
**For:** whoever implements it next (Cursor, Codex, me).
**Against:** the tree-felling work that landed today — severed wood now collapses
into a heap at the foot of the tree, which is where this picks up.

Two asks, in the user's words:

> I'd like to see trees fall with physics once part of it is cut down, roll to a
> river and then float on it.

> Can we also broach into fire, lava, heat and steam?

Those are one project, not two. A falling log needs to know about slope, water
and drag; a lava flow needs the same slope and the same water; steam is what
happens when the hot half meets the wet half. Build the shared plumbing once.

---

## 0. Scope, and what this is not

**In scope.** Rigid debris that falls, tumbles, rolls downhill and floats.
A sparse fire that spreads through fuel and eats the colony's oxygen. Lava that
flows, cools into new rock, and boils water. Steam that rises and condenses.

**Not in scope, deliberately.**

- **No general rigid-body engine.** No Jolt, no Bullet, no Godot physics bodies.
  A few hundred capsules on a heightfield is a closed-form problem and the drum's
  geometry (cylindrical, spinning, Coriolis) is not what a stock engine assumes.
  Everything here is a hand-integrated body in cylindrical coordinates.
- **No volcanism.** Nothing in an artificial drum melts on its own — `economy.rs`
  already says so where `LAVA` is defined. Lava comes out of a smelter, or out of
  a breach. That constraint is a feature: lava is something the colony *did*.
- **No voxel-accurate tree toppling.** A 62,000-cell tree does not become 62,000
  physics bodies. See §3.2 for what it becomes instead.
- **No fire spread through the terrain grid.** Fire lives on fuel, and fuel is the
  woodscape's sparse cells. A 1.57M-cell field is the wrong container.

---

## 1. Ground truth

Do not rediscover these. All verified in the current tree.

### Geometry and grids

| Fact | Value | Where |
|---|---|---|
| Drum radius | 900 m | `Habitat::kepler_drum` |
| Drum length | 6000 m | same |
| Surface gravity | 5.32 m/s² | `hab.surface_gravity()` |
| Gravity at radius r | `ω²r` — weakens as you climb | `hab.gravity_at(r)` |
| Terrain grid | `NT` 1536 × `NZ` 1024 = 1.57M | `terrain.rs` |
| Terrain cell | ~3.68 m arc × ~5.86 m axial | derived |
| Weather/wind grid | `WT` 192 × `WZ` 128 = 24,576 | `weather.rs` |
| Voxel cell | `CELL` = 0.55 m | `woodscape.rs` |
| Voxel budget | `MAX_CELLS` 140,000 | same |
| Canopy tree | ~62,000 cells (15,800 wood, 46,200 leaf) | measured today |

**Coordinates.** Positions are cylindrical `(theta, z, r)`. `r` is distance from
the spin axis, so **larger `r` is downhill** — the ground is at
`r = hab.radius - elevation(theta, z)`, and gravity increases `r`. `player.gd`
already uses exactly this convention (`var vr := 0.0  # positive = falling
outward`); match it rather than inventing another.

**Coriolis.** Radial motion deflects anti-spinward. `player.gd` uses
`d_arc = -2.0 * om * vr * feel * dt` then `theta += d_arc / max(r, 1.0)`, with
`feel = 3.4` because the true ~0.5 m miss is imperceptible. Debris should use the
same exaggeration and the same constant, from one shared place.

### APIs you will build against

```rust
// terrain.rs
ter.elevation(theta, z) -> f32          // metres above the hull
ter.elev: Vec<f32>                      // the raw field, idx(ti, zi)
ter.flow.down: Vec<u32>                 // D8 steepest descent, self = outlet
ter.flow.flux: Vec<f32>                 // 0..1, normalised (d/max)^0.28
ter.flow.filled: Vec<f32>               // ponded surface
ter.flow.mark_dirty_at(ti, zi, r)       // a local edit; see §6 for the cost
ter.hab.to_world(theta, z, r) -> [f32; 3]
ter.hab.to_cyl(p) -> (theta, z, r)
ter.hab.up_at(p) -> [f32; 3]            // inward radial unit vector

// biosphere.rs
bio.water.depth: Vec<f32>               // metres standing water, cap 28
bio.water_stock: f32                    // kg, closed habitat ledger
bio.pond_at(ter, theta, z, m3)          // put water on the ground (added today)
bio.weather.temp_at_elev(theta, z, elev) -> f32   // °C
bio.weather.wind_theta / wind_z: Vec<f32>         // m/s at WT×WZ
bio.soil.sample(theta, z)               // moisture, nutrients
bio.atmosphere.apply_combustion(o2_kg, co2_kg) -> bool   // FALSE if O2 short
bio.woodscape.*                         // see below

// woodscape.rs
ws.harvest_sphere(centre, radius) -> (wood_kg, leaf_kg, blocks)
ws.harvest_one(centre, r) -> Option<(kind, kg)>
ws.collapse_severed(pid, up) -> Option<Fall>     // added today
ws.cut_plants() -> Vec<u32>                      // stands that lost wood
ws.add_taken(pid, kg) / ws.taken_kg(pid)         // the conservation ledger
ws.get(key) -> u8                                // kind::EMPTY/WOOD/LEAF
woodscape::Key = (i32, i32, i32)                 // public
WOOD_KG = 2.8, LEAF_KG = 0.45                    // per cell
```

**Three things `pyro.rs` needs are currently private to `woodscape`.** Widen
them to `pub(crate)` as the first commit of Stage 5, rather than duplicating
them — a second copy of the adjacency rule is exactly the kind of drift that
makes a healthy tree collapse:

```rust
fn neighbors26(k: Key) -> Vec<Key>                     // → pub(crate)
fn wood_within(&self, k: Key, manhattan: i32) -> bool  // → pub(crate)
fn remove(&mut self, k: Key) -> bool                   // → pub(crate) fn burn_cell(...)
```

For removal specifically, do **not** expose `remove` raw. Add
`ws.burn_cell(key) -> Option<(u8, u32, f32)>` returning `(kind, plant, kg)`,
which bills `taken` and queues the leaf checks in one place, so a burned cell
cannot skip the ledger the way a raw `remove` would.

```rust

// economy.rs
bio_id::{GREEN 100, WOOD 101, FIBRE, SEED, WATER 104, ..., LAVA 109}
craft_id::{CHARCOAL 110, ..., ASH 115, ...}
material::id::{REGOLITH 0, SEDIMENT, CLAY, SANDSTONE, BASALT 4, FERROUS, ICE 6,
               ALLOY 7, SAND, COAL 9}
economy::bio_phys(id) -> Phys { bulk_kg_m3, bulking }   // WOOD 650, WATER 1000
economy::deposit_heap(&mut heaps, hab_r, theta, z, id, kg, loose_m3, grade)
economy::ice_melt_fraction(temp_c) -> f32                // added today
material::repose_tan(id) -> f32
material::info(id) -> MatInfo { hardness, cohesion, albedo, roughness, fines }
```

**Material id namespaces are disjoint and load-bearing:** terrain materials are
0–9, bio 100–109, crafted 110+. `weather_heaps` relies on `id >= 100` meaning
"not dug spoil". Keep that.

### Free input bindings

**Every letter A–Z is already bound** in `controls.gd`. New actions must use the
number row (`8`, `9`, `0` are free), an F-key, or a modifier. Add them to the
`controls.gd` table like any other so they stay rebindable from the menu — do not
hardcode `Input.is_key_pressed`.

---

## 2. Architecture

Two new modules. Register both in `lib.rs` **and** in `sim/src/bin/bench.rs`'s
`#[path]` list — omitting the second is a CI break that has happened twice.

```
sim/src/debris.rs   Rigid bodies: fall, tumble, roll, float, come to rest.
sim/src/pyro.rs     Heat, fire, lava, steam. One module: they are one energy
                    system, and splitting them duplicates the heat plumbing.
```

Both hang off `Biosphere` and tick from `Biosphere::tick`, after erosion and
before agents, in this order:

```
weather → soil → erosion → talus → flow rebuild → lakes
  → pyro.tick(...)        // heat moves, fire spreads, lava flows, steam rises
  → debris.tick(...)      // bodies fall, roll, float; lava may launch bodies
  → plants → agents → weather_heaps → dwellings
```

Pyro before debris so a log that lands in a fire ignites on the same tick it
lands, and lava that solidified this tick is already ground for a body to hit.

---

## 3. Part I — Falling bodies

### 3.1 The body

```rust
/// One piece of debris under its own physics. Cylindrical, because the world is.
pub struct Body {
    pub theta: f32,
    pub z: f32,
    pub r: f32,          // distance from the spin axis; larger is downhill
    pub v_arc: f32,      // tangential velocity, m/s (NOT rad/s — see below)
    pub v_z: f32,
    pub v_r: f32,        // positive = falling outward
    pub material: u8,    // bio_id::WOOD, craft_id::ASH, material::id::BASALT...
    pub mass_kg: f32,
    pub length: f32,     // metres; a log's long axis
    pub girth: f32,      // metres; its diameter, and its draft when afloat
    pub spin: f32,       // radians, visual tumble about the long axis
    pub yaw: f32,        // radians, heading of the long axis
    pub tumble: f32,     // rad/s, decays on contact
    pub afloat: bool,
    pub rest_s: f32,     // seconds below the rest threshold
    pub heat_c: f32,     // °C; a burning log is a moving ignition source
}
```

**Store tangential velocity in metres per second, not radians per second.**
`theta += v_arc * dt / r` converts at use. Storing rad/s makes a body silently
speed up as it falls outward, which is wrong and is very hard to see in a test.

### 3.2 What a severed tree becomes

Not 15,800 bodies. `collapse_severed` already returns a `Fall { wood_kg,
leaf_kg, blocks, at }`. Replace the "deposit one heap at the foot" call in
`RamaTerrain::dig_as` with:

1. **Compute the severed part's shape** while the cells are still known — extend
   `collapse_severed` to also return the centroid and the principal axis of the
   fallen set, and its extent along that axis. That is one pass over cells you
   are already walking, and it is what makes the log look like the limb it was.
2. **Split into `n` logs**, `n = clamp(round(wood_kg / 400), 1, 12)`. A branch is
   one log; a whole canopy is a dozen. Distribute mass evenly, place them along
   the principal axis at the fall's centroid, `yaw` from that axis.
3. **Seed velocity** from the geometry: a severed limb rotates about the cut, so
   give each log an outward `v_r` of 0.5–1.5 m/s plus a tangential component
   away from the cut point, and `tumble` of 0.6–2.4 rad/s.
4. **Leaves do not become bodies.** They are already handled: `decay_leaves`
   retires them over the following ticks. Foliage mass goes to a heap as now.

This is the whole trick. The player sees a limb break off, tumble, hit the
slope, roll, and end up in the water. They do not see 15,800 cubes, and the
mass is conserved to the kilogram because it all came off one ledger.

### 3.3 Integration, per step

Use a fixed inner step of **1/45 s**, sub-stepped from `dt_days`, and cap the
sub-steps at 4 per tick. A body moving 12 m/s must not tunnel through a 3.68 m
cell, and the sim tick is coarse (§6).

```
for each body:
    g = hab.gravity_at(r)
    ground_r = hab.radius - ter.elevation(theta, z)
    water_m  = bio.water.depth[cell]
    surface_r = ground_r - water_m          // water surface is UPHILL of the bed

    if afloat:      → 3.6
    elif r < ground_r - 0.05:  → 3.4 (airborne)
    else:           → 3.5 (grounded)
```

#### 3.4 Airborne

```
v_r    += g * dt
v_arc  += -2.0 * hab.omega * v_r * CORIOLIS_FEEL * dt     // anti-spinward
theta  += v_arc * dt / max(r, 1.0)
z      += v_z * dt
r      += v_r * dt
spin   += tumble * dt
```

Air drag is negligible at these speeds and this scale; leave it out and say so
in a comment, so nobody adds it thinking it was forgotten.

**Landing.** When `r >= ground_r`: clamp `r = ground_r`, then

- keep a fraction of the incoming radial speed as a bounce:
  `v_r = -v_r * RESTITUTION` with `RESTITUTION = 0.18` for wood, 0.05 for rock —
  logs thud, they do not bounce like balls;
- if `|v_r| < 0.4` after the bounce, set `v_r = 0` and go grounded, or a body
  jitters on the ground forever;
- kill most of the tumble: `tumble *= 0.35`;
- **emit an impact event** (§3.8) so the client can play a thud and kick dust.

#### 3.5 Grounded — rolling

The interesting half. Downhill is the direction of **increasing r**, i.e.
decreasing elevation. Sample the elevation gradient in metres:

```
d = 3.0                                   // one cell-ish, in metres
dE_arc = (elev(theta + d/R, z) - elev(theta - d/R, z)) / (2d)
dE_z   = (elev(theta, z + d)   - elev(theta, z - d))   / (2d)
slope  = hypot(dE_arc, dE_z)              // rise over run, dimensionless
```

Acceleration along the surface, downhill (`-∇elev`), minus resistance:

```
a_arc = -g * dE_arc
a_z   = -g * dE_z
```

Then **rolling resistance**, which is what makes this read as a log and not a
puck. A log rolls freely across its length and resists along it:

```
roll_dir  = the horizontal direction perpendicular to the log's yaw
along      = component of velocity along yaw
across     = component perpendicular to yaw
across    *= (1 - ROLL_RESIST * dt)       // ROLL_RESIST ≈ 0.55 /s
along     *= (1 - SLIDE_RESIST * dt)      // SLIDE_RESIST ≈ 3.2 /s
```

so a log preferentially rolls sideways and slides very little — which also makes
it turn to lie across the fall line, exactly as real logs do. Recompose velocity
from `along`/`across`, and set `spin += (across / (girth * 0.5)) * dt` so the
roll rate matches the ground speed instead of being decorative.

**Coming to rest.** If `slope < repose_tan(material) * 0.45` and speed < 0.35
m/s, accumulate `rest_s`. Otherwise reset it to 0. Do not test speed alone: a log
crawling down a steep face is not at rest, and one motionless on a slope steeper
than its repose should creep (reuse the `weather_heaps` creep rule).

**Retiring.** At `rest_s > 2.5`, `deposit_heap` the body's mass at its position
and remove it. This is what bounds the system: bodies are a transient, heaps are
the durable form, and the pickup path (`L`) already works on heaps. Cap live
bodies at **256**; when full, retire the one with the largest `rest_s`, and if
none is resting, the one furthest from the player.

#### 3.6 Afloat — the river

Buoyancy from the material's own density, which the economy already knows:

```
draft = girth * (bio_phys(material).bulk_kg_m3 / 1000.0).min(1.0)
afloat = water_m > draft * 0.55
```

Wood at 650 kg/m³ floats with about two thirds submerged. Basalt does not float
at all, and that falls out of the same line rather than needing a special case.

While afloat:

- pin `r = surface_r + draft * 0.5` so the log sits *in* the surface;
- zero `v_r`;
- **advect along the flow**, not down the raw slope. The D8 field is already
  built and, since today, cheap to keep current:

```
dn = ter.flow.down[cell]
if dn != cell:
    aim = the world direction from this cell's centre to dn's centre
    speed_target = FLOAT_SPEED * (0.25 + 0.75 * ter.flow.flux[cell])
    v ← lerp(v, aim * speed_target, FLOAT_DRAG * dt)     // FLOAT_DRAG ≈ 2.0 /s
```

with `FLOAT_SPEED ≈ 3.2 m/s` in a full channel. Lerping rather than setting is
what makes a log entering a river swing round and pick up the current instead of
teleporting into it.

- **A floating log is not at rest.** Only accumulate `rest_s` while afloat if
  `flux` is negligible (a still pond) — otherwise a log would turn into a heap
  in the middle of a river.
- **Beaching.** When `water_m` drops below `draft * 0.35`, clear `afloat` and
  go grounded. Logs pile up on the inside of bends, which is free and correct.

#### 3.7 Where bodies come from

| Source | Mass | Note |
|---|---|---|
| `collapse_severed` | the fallen wood | §3.2 — the headline |
| `H` fell (`RamaTerrain::harvest_near`, `lib.rs:999`) | trunk fraction | so felling *looks* like felling |
| Talus / creep on a steep face | boulders | optional; reuses the repose rule |
| Lava launching a cooled crust | rock | §4.3, optional polish |

Route every one through a single `debris.spawn_log(...)`, so the mass accounting
happens in exactly one place.

#### 3.8 Client surface

```rust
#[func] fn debris_lod(&self, x: f64, y: f64, z: f64, radius: f64, limit: i64)
        -> PackedFloat32Array
// flat, stride 11: [x, y, z,  ax, ay, az (long-axis unit),  spin,
//                   length, girth, material, heat01]

#[func] fn debris_events(&mut self) -> VariantArray
// drains a queue: [{kind: "impact"|"splash"|"beach", x, y, z, energy}]
```

Follow `woodscape_lod`'s shape exactly — flat `PackedFloat32Array`, client
builds one `MultiMesh`. `world.gd` gets `_build_debris()` / `refresh_debris()`
mirroring `_build_woodscape` / `refresh_woodscape`, a capsule mesh, one draw
call, shadows off. Events drive audio — `audio.dig(brush, hardness)` at `audio.gd:240` for thuds,
`audio.splash(strength)` at `audio.gd:276` for water entry — and the existing
`world.spawn_dig_chips(p, normal, blocks, timber)` /
`world.spawn_dig_splash(p, strength)` bursts, both of which were generalised
today and share one MultiMesh via `_spawn_burst`. A new debris burst should be
a fourth caller of that, not a fifth particle system.

Draining the event queue in a getter means the client must call it every frame
or events pile up. Cap the queue at 64 and drop the oldest.

---

## 4. Part II — The hot half

### 4.0 The heat field

One coarse field, on the **weather grid** (`WT`×`WZ` = 24,576), not the terrain
grid. It is the resolution at which temperature and wind already exist, so
everything that reads heat can already sample there, and it is 64× cheaper than
the terrain grid.

```rust
pub struct Heat {
    /// Degrees above the local ambient, WT×WZ.
    pub excess_c: Vec<f32>,
}
```

Each tick: **diffuse** (one Jacobi pass — Gauss-Seidel is order-dependent and
that has already produced a visible directional bias in the talus work), then
**decay** toward zero with a half-life of about 0.05 habitat days, then **advect
with the wind** using `wind_theta` / `wind_z`.

Anything that wants a real temperature asks for
`weather.temp_at_elev(...) + heat.excess_at(theta, z)`. Wire that into
`ice_melt_fraction`'s call site immediately: **a fire next to an ice lens should
melt it.** That is a one-line payoff for having the field at all.

### 4.1 Fire

Fire lives on fuel, and fuel is sparse. Mirror `woodscape`'s own container:

```rust
pub struct Fire {
    /// Burning voxels. Key is a woodscape cell key.
    burning: FastMap<woodscape::Key, Ember>,
    /// Cells queued for an ignition roll — the same dirty-set discipline
    /// `leaf_checks` uses, so a big fire costs a bounded amount per tick.
    front: Vec<woodscape::Key>,
}

pub struct Ember {
    fuel_kg: f32,     // what is left to burn in this cell
    temp_c: f32,
    plant: u32,       // for the removal ledger
}
```

**Per tick, with a budget** (`budget = clamp(ceil(600 * dt_days), 32, 4096)`):

1. **Burn.** Each ember consumes `BURN_RATE * dt` kg. Wood burns slowly and hot;
   leaves flash off. Suggested: leaves at 8× the rate of wood, wood peaking near
   800 °C, leaves near 500 °C.
2. **Pay for it.** Combustion is not free in a sealed can:

```
o2  = burned_kg * 1.4           // rough CH2O + O2 stoichiometry
co2 = burned_kg * 1.6
if !bio.atmosphere.apply_combustion(o2, co2) {
    // The fire suffocates. This is the good case, not an error case.
    ember.temp_c *= 0.5;
    continue;
}
```

**This is the spine of the whole feature.** `apply_combustion` already returns
`false` when O₂ is short. A forest fire in a 180-tonne oxygen inventory is an
atmospheric emergency: CO₂ climbs, the scrubbers (`tick_scrub`) fall behind, and
the colony has a reason to care about a fire two kilometres away. Do not paper
over the `false` branch.

3. **Credit the ledger.** `ws.add_taken(plant, kg)` for everything burned, or
   `H` will still pay out for a tree that is now ash. Today's session closed
   exactly this hole for leaf decay; do not reopen it.
4. **Ash.** `deposit_heap(craft_id::ASH, burned_kg * 0.06, ...)` at the column,
   and enrich the ground through **`bio.soil.amend(theta, z, effect, kg,
   radius)`** — not by writing `soil.n`/`soil.k` directly. The effect already
   exists: `economy::amendment_effect(craft_id::ASH)` returns
   `{ n: 0.0, p: 0.02, k: 0.18, organic: 0.0, ph: 0.08 }` — potash and lime, no
   nitrogen. Burnt ground grows well afterwards, which closes the loop from
   fire back to regrowth, and it needs no new data.

   **Watch the nitrogen ledger.** `Biosphere` holds `nitrogen_stock` against a
   `nitrogen_initial` golden-invariant baseline with a test on it. Burning
   plant matter releases nitrogen to the air; if you add soil nitrogen from ash
   without taking it from somewhere, you will break that invariant. Ash
   carrying no `n` is the honest answer and it keeps the ledger closed.
5. **Spread.** For each ember, roll against `neighbors26` cells that are
   `WOOD`/`LEAF`:

```
p = SPREAD_BASE * dt
  * heat_factor(ember.temp_c)             // 0 below ~250 °C, 1 by ~600
  * dryness                               // 1 - soil moisture at the column,
                                          //   times (1 - weather humidity)
  * wind_gain(direction, wind_at(theta,z)) // 1.0 cross-wind, up to 2.5 downwind
  * fuel_gain(kind)                       // leaves catch far more readily
```

Wind alignment is what makes a fire a *shape* rather than a growing sphere, and
it is the single cheapest thing that makes it look real.

6. **Extinguish.** An ember dies if any of: `water.depth[column] > 0.02`;
   rainfall above a threshold at that column; `fuel_kg <= 0`; `apply_combustion`
   refused three ticks running. Water beats fire — a river must be a firebreak,
   or the drum burns down once and is never interesting again.

7. **Heat out.** Add `burned_kg * HEAT_PER_KG` into `Heat::excess_c` at the
   column. That is what melts ice, makes steam, and warms the field for spread.

**Ignition sources.** A body with `heat_c` above ignition rolling into fuel; a
lava cell within 1.5 m of fuel; a player action (§4.5); and lightning only if
the weather model ever grows it — do not add lightning for this.

**Rendering.** `fire_lod()` on the same flat-array pattern, stride 5:
`[x, y, z, temp01, size]`. Client draws an emissive `MultiMesh` plus **one**
`OmniLight3D` placed at the fire's centroid with energy from total burning mass.
Do not add a light per ember; a hundred lights will halve the frame rate and the
visual difference is nil. Smoke can reuse the burst system with a grey tint and
a positive `rise`.

### 4.2 Steam

The one that ties the halves together.

```rust
pub struct Puff { theta: f32, z: f32, r: f32, mass_kg: f32, heat_c: f32, life: f32 }
```

**Created by:** lava meeting water (violent — see §4.3); fire burning over
saturated ground; any `Heat::excess_c` above ~100 °C at a column with standing
water. In every case, take the water from `bio.water.depth` **and** account it,
so the closed water ledger stays closed:

```
water.depth[i] -= m3 / cell_area;
// mass moves from the ground to the air, and comes back in 4.2.2
```

**Per tick:** `r -= RISE * dt` (steam goes *toward the axis* — remember larger
`r` is down), mass spreads and thins, `heat_c` decays into `Heat::excess_c`
(steam is how heat travels), and `life` runs down.

**Condensation.** When `heat_c` falls below ~100 °C or `life` expires, return
the mass: `bio.water_stock += mass_kg`, or `bio.pond_at(...)` if it is still low
enough to rain out locally. **Total water must be conserved across the whole
cycle** — that is the test (§7).

**Scalding.** A puff is dangerous. Expose `steam_at(theta, z, r) -> f32` and let
`player.gd` take damage inside a hot one. This is the cheapest possible way to
make lava genuinely frightening rather than decorative.

### 4.3 Lava

Sparse cells on the terrain grid, flowing on the field that already exists.

```rust
pub struct Lava {
    /// Terrain cell index → molten rock sitting on it.
    cells: FastMap<usize, Flow>,
}
pub struct Flow { m3: f32, temp_c: f32 }
```

**Flow.** Reuse `ter.flow.down` — the D8 field. Each tick, move a fraction of
each cell's volume to its downstream neighbour, throttled by viscosity, which
rises steeply as it cools:

```
mobility = smoothstep(SOLIDUS_C, LIQUIDUS_C, temp_c)   // ~980 → ~1200 °C
moved    = m3 * mobility * LAVA_SPEED * dt
```

so a flow slows and thickens as it goes, and stops of its own accord. This is
why lava does not need a pressure solver to look right.

**Cooling.** `temp_c` falls with time, faster with contact area, *much* faster
against water. Below `SOLIDUS_C`:

- delete the lava cell;
- **raise the terrain**: `ter.elev[i] += m3 / cell_area` and
  `ter.flow.mark_dirty_at(ti, zi, 4)`;
- set the surface material to `BASALT` for that column.

**This is the most valuable thing in the whole document.** Everything else in
the game removes landscape. Lava *adds* it: you can pour a causeway across a
river, dam a valley, or build a pad. And because it marks the flow field dirty,
the water re-routes around your new rock within half a second — that path was
made fast this session precisely so this kind of thing can be responsive.

**Lava meets water.** At a cell with `water.depth > 0.02`:

- quench hard: `temp_c -= QUENCH * depth * dt`, so shallow water slows lava and
  deep water stops it dead;
- boil: convert water to steam at ~2.6 MJ/kg of heat removed, spawn puffs;
- **make rock, not mud**: the quenched volume solidifies immediately as basalt.
  Pouring lava into a lake should build a spit and throw up a steam plume.

**Lava meets vegetation.** Ignite any woodscape fuel within ~1.5 m.

**Sources.** `bio_id::LAVA` already exists as a smelter product. Give the player
a **pour** action (§4.5) that spends carried lava, and treat a smelter running
without cooling as a breach that spawns a flow. No natural volcanism.

### 4.4 Heat, the other consumers

Once `Heat` exists, wire the cheap payoffs immediately — each is a few lines and
each makes the field feel like part of the world rather than a fire subsystem:

- **Ice** — `ice_melt_fraction` reads ambient + excess (§4.0).
- **Plants** — scorch or kill vegetation above ~120 °C; regrowth on ash is fast.
- **Comfort** — the player's own temperature, if that exists; else HUD only.
- **Agents** — colonists should not path through fire. `agent.rs` already has a
  fear field (`mean_fear` is in `TickReport`); add heat to it rather than
  building a second avoidance system.
- **Snow/frost** — negative excess near the endcaps, if you want it later.

### 4.5 Player actions

Add to `controls.gd` (all letters are taken — use the number row):

| Action | Key | Effect |
|---|---|---|
| `ignite` | `8` | Light the aimed fuel cell, if dry enough. Should fail loudly on wet wood. |
| `douse` | `9` | Spend carried `bio_id::WATER` to extinguish; `pond_at` at the aim point. |
| `pour` | `0` | Spend carried `bio_id::LAVA` into a lava cell at the aim point. |

Each returns a dict like `dig` does, and `player.gd` reports it with
`world.note(...)`. Refusals need to say *why* — "too wet to light", "no lava in
pack" — because a silent no-op is the bug the block-mining work spent a whole
session chasing.

---

## 5. Sequencing, with gates

Ship each stage. Do not start the next until its gate passes — and do not leave
a half-built stage in the tree, because a GDScript file with a half-written
function is a **parse error that takes the whole script down**, which has already
cost this project a session.

**Stage 1 — bodies fall and roll.** `debris.rs`, spawn from
`collapse_severed`, integrate, land, roll, retire to a heap. No water yet.
*Gate:* a log dropped on a slope ends up downhill of where it started; on flat
ground it stays put; mass in equals mass in the heap out.

**Stage 2 — bodies float.** Buoyancy, flow advection, beaching.
*Gate:* a log spawned above a river ends up in the channel and moves along it;
a basalt block in the same river does not float.

**Stage 3 — client.** `debris_lod`, `debris_events`, the MultiMesh, thuds and
splashes.
*Gate:* the user can fell a tree and watch the limb roll into water. **This is
the ask; everything before it is scaffolding.**

**Stage 4 — heat.** `Heat` field, diffusion, decay, wind advection, and the
`ice_melt_fraction` hookup.
*Gate:* heat put in one place shows up nearby and fades; total is bounded; ice
near a heat source melts more than ice far from one.

**Stage 5 — fire.** Embers, spread, the oxygen gate, ash, extinguishing.
*Gate:* fire spreads downwind faster than upwind; water stops it; burning a
tree bills its ledger so `H` pays nothing afterwards; CO₂ rises and O₂ falls by
stoichiometric amounts.

**Stage 6 — lava and steam.** Flow, cooling to basalt, quenching, puffs,
condensation.
*Gate:* poured lava runs downhill, stops, and leaves higher ground than it
found; water re-routes around it; lava into water makes steam and rock; **the
habitat's total water is unchanged across a full boil-and-condense cycle.**

Stages 1–3 are the user's actual request. 4–6 are the second half of the ask and
depend on nothing from each other except `Heat`.

---

## 6. Traps that have already cost time

Read this section. Every item is something that has actually bitten in this
repository.

1. **The dylib must be re-signed.** Overwriting `librama_sim.dylib` in place
   invalidates its ad-hoc signature and the kernel then `SIGKILL`s Godot **with
   no output whatsoever**. Always `./build.sh` (which removes, copies, and
   `codesign --force --sign -`). Never `cp` it yourself. This cost most of an
   afternoon and looked exactly like an unrelated crash.

2. **A sim tick is expensive and rare.** `sim_tick` costs **~270 ms** on an idle
   tick and is called with `dt_days` up to 0.032, roughly every 1.6 real
   seconds. Debris integrated at that cadence will visibly teleport. Sub-step
   inside the tick (§3.3), and consider exposing
   `debris_step(dt_seconds)` for the client to call every frame independently of
   the biosphere tick. Falling is the one thing here that must run at frame rate.

3. **`ter.flow.mark_dirty_at` is now cheap but not free** — ~1.3 ms for a local
   repair, and it defers a ~240 ms full priority flood via `wants_full()`. A
   lava flow that marks dirty every cell every tick will thrash it. Mark once
   per solidified cell, not per flow step.

4. **Register new modules in `bench.rs` too.** `sim/src/bin/bench.rs` declares
   every module by `#[path]`. Adding a module to `lib.rs` alone compiles locally
   and fails CI. Run `cargo check --all-targets`.

5. **`f32` sums drift.** A conservation test over ~12,000 additions at 33-tonne
   magnitude differs by several kg purely from summation order. Use a *relative*
   tolerance (5e-4 worked) and say why in the assertion, or you will hunt a
   phantom leak.

6. **Corner adjacency, not face, for anything structural in the woodscape.**
   `tree_form` stamps boxes that meet a trunk at an angle: 23% of an untouched
   canopy tree reaches its stump only through a corner. See `neighbors26` and
   the test that pins it.

7. **`is_full()` is the wrong question for anything stand-sized.** It asks
   whether the very last cell is spoken for. Ask `sprout_plant` what it needs.

8. **`Input.is_action_just_pressed` polling ignores `set_input_as_handled()`.**
   The same keystroke fires in both the consumer and the poller. If `ignite`
   ever gets a confirm dialog, expect a double fire.

9. **`Control.position` is parent-space and not idempotent** for anchored
   controls. Any new HUD readout should set anchors and offsets explicitly.

10. **Close the ledgers.** Water, nitrogen, carbon and now removed plant mass are
    all closed loops with tests. Fire, steam and lava each move mass between
    ledgers, and each needs a conservation test — that is the house style and it
    is what catches the real bugs.

11. **`f32::MAX` is finite.** Use `INFINITY` for sentinels; the bounded path in
    `surface()` overflowed because of this.

12. **Close files in the Godot editor before editing them from a tool.** The
    editor has saved a stale buffer over a rewritten `trees.gd` once already.

---

## 7. Test list

Native, in-module, no engine. These are the gates from §5 written out.

**debris**
- `a_log_on_a_slope_ends_up_downhill`
- `a_log_on_flat_ground_stays_put`
- `a_log_does_not_tunnel_through_the_ground` — spawn at 20 m/s, assert it lands
- `mass_is_conserved_from_fall_to_heap`
- `wood_floats_and_basalt_does_not`
- `a_floating_log_follows_the_channel`
- `a_log_beaches_when_the_water_runs_out`
- `a_body_at_rest_becomes_a_heap_and_stops_costing_anything`
- `the_body_count_is_capped`

**heat**
- `heat_spreads_and_fades`
- `heat_is_bounded` — no runaway from repeated input
- `heat_advects_downwind`
- `ice_near_a_fire_melts_faster_than_ice_far_from_one`

**fire**
- `fire_spreads_downwind_faster_than_upwind`
- `water_stops_a_fire`
- `wet_wood_does_not_light`
- `burning_a_tree_bills_its_ledger` — then `H` pays ~nothing
- `a_fire_consumes_oxygen_and_emits_carbon` — stoichiometric, both ledgers
- `a_fire_suffocates_when_the_oxygen_runs_out` — the `false` branch
- `fire_leaves_ash_and_richer_soil`

**lava**
- `lava_flows_downhill_and_stops`
- `cooled_lava_raises_the_ground` — and marks flow dirty
- `water_reroutes_around_new_rock`
- `lava_in_water_makes_steam_and_rock`
- `lava_ignites_nearby_fuel`

**steam**
- `steam_rises_and_condenses`
- `a_full_boil_and_condense_cycle_conserves_the_habitat_water` — the big one

Plus the five existing gates (`tools/check_*.py`), `cargo check --all-targets`,
`cargo fmt --check`, and
`cargo clippy --all-targets --release -- -D clippy::correctness -D clippy::suspicious`.

---

## 8. Leave alone

- **`game/scripts/coop_lobby.gd`** — a teammate is actively writing it.
- **`sim/src/flow.rs`'s local repair.** It is exact in `down`, `lake` and
  `discharge` and within 0.06% in `flux`, with tests pinning that. If lava needs
  something new from it, add a method; do not loosen the repair.
- **The stage-0 forest ratchets** in `docs/FOREST_STAGE_0_1.md` (at-clamp ≤ 70%,
  trunk overlaps ≤ 260). They are meant to go down, never up. Fire that kills
  trees will move these numbers — re-measure and tighten, do not raise.
- **`bite_scale`'s wood value (0.42)** is tuned so the default brush takes a
  visible chunk and the minimum brush still takes exactly one block. Changing it
  changes how felling feels.

---

## 9. Open questions for the user

1. **Should fire be able to burn the colony down?** The oxygen gate makes a big
   fire genuinely dangerous. That is either the best thing here or a source of
   unrecoverable saves. Suggest: fire cannot start in rain, cannot cross water,
   and dies below a spread threshold — but a fire the player *starts* in a dry
   province in a high wind should be able to take a whole grove.
2. **Do felled logs need to be shapeable?** A log that can be dragged, stacked,
   or split into planks is a much bigger feature than a log that becomes a heap
   after 2.5 s. Stage 1–3 assume the heap.
3. **Is lava a player tool or a hazard?** §4.3 assumes both. If it is only a
   hazard, the pour action and half the appeal go away.
