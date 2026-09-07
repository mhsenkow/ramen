# Simulation Architecture & Tech Stack Brief — *RAMA CYCLE*

**Status:** Draft v0.1 · **Owner:** Engineering · **Date:** 2026-09-06
**Companion to:** `PD_BRIEF.md`, `EM_BRIEF.md`
**Question this answers:** how do you get a fully mutable voxel world *and* an
ecosystem that behaves like a real biosphere *and* a legible farming/town game,
in one runtime, at 60fps?

---

## 0. Thesis

Three claims, and the whole document follows from them:

1. **The closed system is the cheat code.** Open-world ecosystems are hard to
   make convincing because there is always an "elsewhere" to launder
   inconsistency through. A rotating habitat has hard walls. Conservation of
   mass is *visible, auditable and dramatic* — every gram of nitrogen in the
   drum came from somewhere and is going somewhere. You get "this feels alive"
   almost free, from arithmetic that closes.
2. **Simulation is cheap; presentation is expensive.** People assume the biology
   is the performance problem. It isn't. Ten floats per plant, ticked hourly, is
   nothing. Meshing voxels and drawing them is the entire budget. Design
   accordingly: be generous with sim depth, miserly with sim *presence*.
3. **Depth and legibility are not in tension if you separate where each lives.**
   Wilderness runs the sim wild. Tended plots and the town are clamped and
   instrumented. The player learns the system in the safe place and then goes
   out into the place where it bites.

---

## 1. The core architectural move: simulation LOD

Everyone does level-of-detail for rendering. Almost nobody does it for
simulation, and it is the only way to get what you're describing.

| Tier | Scope | Representation | Tick rate |
|---|---|---|---|
| **T0 — Individual** | Player's plots, the town, ~200m radius around any player | Every plant, animal, soil cell modelled discretely | Per in-game 15 min |
| **T1 — Regional** | Each of ~50–200 named regions of the habitat | Continuous fields: biomass density, species mix, moisture, population counts | Per in-game 6 h |
| **T2 — Habitat** | The whole drum | ~20 scalars: atmosphere composition, water inventory, nutrient stocks, power, mean temperature | Per in-game day |

Each tier is a **budget** for the one below it. T2 says the drum holds X tonnes
of fixed nitrogen; T1 distributes it across regions; T0 spends it per plant.
Nothing is created at a lower tier. That constraint is what makes the world feel
like one system rather than three unrelated simulations.

### Promotion and demotion

The hard part, and the part to get right first.

- **Promotion (T1→T0):** when the player approaches a region, its statistics are
  *instantiated* into individuals by deterministic sampling — seeded on
  `(region_id, species, sim_time)`. The same region always resolves to the same
  forest.
- **Demotion (T0→T1):** individuals collapse back into statistics. Player-made
  changes (a felled grove, a dug pit, a planted field) are recorded as
  **persistent deltas** that survive the round trip.

This is what buys you "the wilderness kept living while I was away" without
simulating a million plants. It is also the single highest-risk piece of the
architecture — see spike S5.

---

## 2. The voxel layer

### 2.1 What "Minecraft realism" should mean here

Not cubes. A cubic-voxel look would fight the tilt-shift diorama camera and be
tonally incoherent next to live-action Archive footage. What you actually want
from Minecraft is the *property*, not the aesthetic: **the world is matter, all
of it mutable, all of it accounted for.**

**Recommendation: sparse voxel / signed-distance field, meshed with dual
contouring (surface nets), textured triplanar.** Smooth, fully diggable,
fully buildable. Proven at scale by Astroneer, Enshrouded, Deep Rock Galactic.
Chunked, streamed, LOD'd via transvoxel to avoid seams.

### 2.2 The curvature problem is smaller than it looks

The important number: for a 32m chunk on a 3km-radius drum, the sagitta —
how far the curved surface deviates from a flat plane across that chunk — is

```
s ≈ c² / 8r  =  32² / (8 × 3000)  ≈  0.043 m
```

**Four centimetres over a 32-metre chunk.** That is below terrain noise.

Therefore: **voxel chunks are ordinary flat Euclidean grids.** They are indexed
by `(θ_index, z_index, r_layer)` and *placed* into world space with a rotation
about the habitat axis. Meshing is standard. Collision inside a chunk is
standard. Tools author in flat space. Only two things need to know about the
cylinder at all:

- the per-actor gravity vector, and
- the far-field render, where curvature is actually visible.

This substantially de-risks R1 in `EM_BRIEF.md`. It does not eliminate the
navmesh seam problem — pathing *across* chunk boundaries still has to handle a
rotating up-vector — but it means you are not writing curved-space geometry.

Note also that the habitable shell is thin: you care about maybe 100m of soil
and rock out of a 3000m radius. The voxel volume is a **wrapped slab**, not a
solid cylinder. Memory follows accordingly.

### 2.3 Matter and life are different layers

Minecraft's fatal limitation for your purposes is that a wheat plant *is a
block*. That conflation is precisely why Minecraft farming can never be deep.

**Voxels store matter only:** rock, ore, soil substrate, water, built structure.
**Plants, animals and agents are entities** with a position, anchored to the
voxel surface, carrying their own state. They query the voxel grid and the soil
grid; they are not stored in either.

This decoupling is what makes everything in §3 possible.

---

## 3. Algorithmic growth — the part you're actually excited about

### 3.1 The plant model: source–sink carbon allocation

Skip L-systems as the *simulation* (keep them for geometry — see §3.2). The
scientifically grounded and computationally trivial approach is a functional–
structural plant model:

```
   light × leaf_area × water_factor × temp_factor
                    ↓
            photosynthesis → carbon pool
                    ↓
     allocated to organs by demand and nutrient availability
                    ↓
        root   leaf   stem   flower   fruit
                    ↓
      morphology, which changes next tick's light capture
```

Per-plant state is roughly ten floats: `carbon_pool`, `biomass{root, leaf, stem,
reproductive}`, `water_status`, `nitrogen_status`, `age`, `stress_accumulator`,
`genome_ref`.

**Why this is the right choice: nothing is scripted, and the emergent behaviours
are the correct ones.**

- A plant in shade allocates to stem and gets leggy, reaching for light.
- Plenty of light but no nitrogen → small, pale, thick leaves.
- Crowd a plot and it self-thins; the losers were out-shaded, not flagged.
- Drought stress mid-fruiting → the plant aborts fruit to save the root.

No designer wrote any of those. They fall out of the allocation rule. **That is
the entire "feels real within its biosphere" requirement, and it is about three
hundred lines of arithmetic.**

### 3.2 Geometry from state

The carbon model drives a **parametric L-system** for visual form: internode
count and length from stem biomass, leaf size and count from leaf biomass, fruit
from reproductive biomass. Generate a mesh only for T0 plants the player can
actually see; everything else is state without geometry.

Instanced rendering with per-instance parameters; a small library of hand-tuned
L-system grammars per species, driven by simulated values. Don't generate
unique meshes per plant — generate a small number of morphology *bins* and
instance within them.

### 3.3 Soil is the substrate, and it is where the game lives

The layer most games skip, and the reason theirs feel fake.

A soil grid coarser than the voxel grid (1–2m cells, surface shell only), each
cell carrying: `nitrogen`, `phosphorus`, `potassium`, `organic_matter`,
`moisture`, `pH`, `compaction`, `microbial_biomass`. Diffusion between adjacent
cells; a slow ambient pass; a fast pass where the player is working.

In a closed habitat this is the whole economy: **there is no off-world
fertiliser.** Crops deplete. Legumes fix nitrogen from the drum's atmosphere.
Compost and waste return organic matter. Every harvest you export from a plot
is nutrient removed from that soil, and it has to come back or that soil dies.
The player is not "farming"; the player is **running a nutrient cycle**, and the
consequences of not closing it are visible over seasons.

Conservation of mass is doing all the work here. It is easy to implement and it
is unfakeable.

### 3.4 Crop genetics — which rhymes with the Crèche

Plants carry a small trait vector (yield, hardiness, maturation rate, nutrient
demand, flavour, radiation tolerance). Crossbreeding produces offspring by
recombination with mutation. Cosmic radiation raises the mutation rate — *which
is the game's founding premise applied to the plants.*

The player spends the game selectively breeding crops for a hostile closed
environment while a committee does exactly that with the colony's own lineages.
One mechanic, two scales, no dialogue required to make the point. **This is the
strongest thematic hook in the simulation layer — build it early.**

### 3.5 Fauna

- **T0:** agent-based. Individual animals, needs-driven, foraging against real
  local biomass.
- **T1:** population dynamics. Predator–prey coupled to regional plant biomass.
  Overhunt and populations crash — genuinely, not on a scripted timer.
- **Closed system consequence:** extinction is permanent. There is no migration
  from elsewhere. Kill the last of something and it is gone for the rest of the
  playthrough, and the food web reorganises around the hole.

That stake does not exist in any Earth-set farming game. Take it.

### 3.6 Weather is engineered, not natural

The habitat has no weather. It has **infrastructure that produces weather.**
Rain falls because condensers run. Day length is a lighting schedule someone
set. "Seasons" are a policy the colony votes on. Temperature is a radiator
budget.

So the player can *change the climate* by reallocating power — and pays for it
somewhere else in a closed energy budget. This is a mechanic no terrestrial
farming sim can have, it costs almost nothing to implement (it's a handful of
scalars in T2), and it converts the setting into gameplay. **Highest
value-per-line-of-code item in this document.**

---

## 4. Keeping it legible

The failure mode is a Dwarf-Fortress-grade simulation the player cannot read,
where a bad harvest is indistinguishable from a bug. Four defences:

1. **The town and player plots are permanently T0.** Never demoted, never
   statistically approximated. The places the player reasons about are the
   places that are deterministic and inspectable.
2. **Diegetic instrumentation.** Soil probes, an agronomy console, overlay
   views for moisture / nitrogen / light / yield-forecast. The player reads the
   simulation through in-fiction tools rather than guessing. *Oxygen Not
   Included*'s overlay system is the model to copy — it is the single best
   solution anyone has shipped to this exact problem.
3. **Bounded chaos.** Tending clamps variance. A well-tended plot has a narrow
   outcome distribution; neglect widens it. A bad harvest on a tended plot must
   always have a legible cause the player could have seen coming. Wilderness is
   where the sim is allowed to be cruel.
4. **Always attribute.** Every yield result carries its causal chain — "−22%:
   nitrogen deficit from day 34". Store the reason, not just the number. This is
   also, not incidentally, your best debugging tool.

---

## 5. Recommended stack

| Layer | Choice | Rationale |
|---|---|---|
| **Presentation** | **Godot 4.6** | Per `EM_BRIEF.md` §2 |
| **Sim core** | **Rust**, `gdext` GDExtension | Determinism control, no GC, `rayon` parallelism, headless CI testing, survives an engine change |
| **Entity storage** | **`bevy_ecs`** standalone crate | Best ECS in Rust for this workload, without betting on Bevy the engine (`EM_BRIEF.md` §2.2) |
| **Voxel** | **`godot_voxel`** (SDF/smooth, transvoxel LOD) | Mature, exactly the stack §2.1 specifies. Months saved |
| **Meshing** | Worker threads → procedural mesh upload | Never on the game thread |
| **Scheduling** | Fixed ms budget per frame, regions round-robin | The Factorio model. Sim must never spike a frame |
| **Persistence** | SQLite for structured state + zstd chunk-delta blobs | See §5.2 |
| **Content data** | TOML, hot-reloadable — species, recipes, biomes, soil profiles | Designers must tune without a rebuild |
| **Narrative** | Ink | Per `EM_BRIEF.md` |

### 5.1 On Rust

The honest trade-off. **For:** the sim core is the game, it must be
deterministic across platforms and versions, and it must run headless in CI for
hundreds of simulated days. Rust enforces the discipline that makes that true,
and parallelising a hundred-thousand-entity tick is genuinely pleasant.
**Against:** FFI marshalling is real work, debugging across the boundary is
worse than debugging within it, and the hiring pool is smaller.

**Recommendation: Rust, on one condition** — that the FFI surface stays narrow
and data-oriented. A handful of calls that exchange flat buffers, not a chatty
object-graph API. If the boundary starts growing per-entity accessors, that is
the signal the design has gone wrong.

**Fallback:** C++ inside UE, in a plugin module with a hard rule of zero UObject
dependencies in sim code. Same architecture, less friction, weaker enforcement.
Choose this if the team is UE-native and the Rust tax looks like it will
dominate. The architecture in this document does not change either way.

### 5.2 Persistence — the sleeper risk

A mutable voxel world plus per-plant state plus narrative state produces very
large saves, and it is normally discovered too late.

The rule: **deterministic worldgen means untouched chunks are never stored.**
Store the seed and the *deltas from generation*. A player who has dug three
mines has a save containing three mines, not a habitat.

- Voxel: per-chunk delta blobs, zstd, only for modified chunks.
- Entities, soil, narrative, T1/T2 fields: SQLite. Queryable, transactional,
  survives a crash mid-write, and migratable — which matters enormously for
  Early Access, where the schema *will* change under live saves.
- Save format versioned and migration-tested from the first commit. This is
  `EM_BRIEF.md` R2 and it is the risk most likely to actually hurt you.

### 5.3 Rough budget (60fps → 16.6ms)

Targets, to be validated in spike S6:

| | Budget | Note |
|---|---|---|
| Sim tick (amortised) | 2–3 ms | T0 plants ~20k, hourly ticks spread across frames ≈ a few hundred plants/frame |
| Soil diffusion | < 1 ms | ~100k cells, SIMD-friendly, ticked every in-game 15 min |
| Voxel meshing | 0 ms game thread | Fully off-thread, budgeted queue |
| Everything else | ~13 ms | Rendering, animation, audio, UI |

The claim to test early: **the biology is not the bottleneck.** If S6 confirms
that, the design can be far more ambitious about sim depth than instinct
suggests.

---

## 6. Explicit anti-scope

Cut these now, in writing, or they will be argued for repeatedly:

- **No real fluid dynamics.** Water is cellular automata on the soil grid plus a
  coarse flow field. Not Navier–Stokes.
- **No per-leaf light transport.** A coarse light-accumulation grid, sampled.
- **No genome-level biology.** Traits are a short vector, not base pairs.
- **No atmospheric chemistry.** A handful of T2 scalars.
- **No habitat-wide animal pathfinding.** T1 populations move as fields;
  individuals path only within T0.
- **No cubic-voxel aesthetic.** Decided in §2.1; don't relitigate it in month
  ten.

---

## 7. Spikes (continues the S1–S4 series in `EM_BRIEF.md`)

| | Spike | Duration | Kill criterion |
|---|---|---|---|
| **S5** | **Promotion/demotion round-trip.** Instantiate a T1 region into individuals, modify it (fell trees, dig, plant), demote, advance 30 in-game days, re-promote. Result must be consistent and player deltas must survive. | 3 wk | Round-trip is lossy or inconsistent → the wilderness cannot be simulated; scope back to a hand-authored world |
| **S6** | **Sim load test.** 20k plants + 100k soil cells + 500 animals, headless, 100 in-game days, deterministic and bit-identical on replay. Measure. | 2 wk | Can't hit budget → reduce T0 radius before reducing model depth |
| **S7** | **The legibility test.** Give a player a plot, a soil probe and no tutorial. Have them diagnose a nitrogen-deficient harvest. | 1 wk | They can't → instrumentation is under-built; fix before content, not after |

S5 is the one that decides whether this architecture is real. Run it first.

---

## 8. What I'd build in what order

1. Soil grid + carbon-allocation plant model, headless, no rendering. Print
   numbers. Get a plant to grow leggy in shade with nobody having written a rule
   that says so.
2. Voxel chunks, flat-grid, wrapped and placed. Dig a hole. Save it. Load it.
3. T0/T1 promotion round-trip (S5).
4. Crop genetics — early, because it's the thematic spine (§3.4).
5. Overlays and instrumentation — *before* content, not after (S7).
6. Fauna, T1 populations, the town, everything else.

The first item is a week and it will tell you more about whether this game works
than the next three months of anything else.

---

## 9. Terrain: caves and mountains, derived from process

*Added after reviewing ORRERY (`~/simearth`), which derives Earth's hypsometry
from plate boundaries, isostasy, crustal age and strain rather than stacking
noise for looks. That discipline — **honesty of mechanism over cosmetic
spectacle** — transfers directly, and its prioritisation rule should be adopted
verbatim (see §10).*

### 9.1 The problem the setting hands us

Dramatic terrain wants a process, and a habitat has no plate tectonics. Nobody
built subduction into a can. Generic noise-caves would be exactly the
"cosmetic spectacle" ORRERY refuses.

**But the habitat has two processes it can honestly claim, and they're better
than tectonics:**

**(1) It was engineered.** Mountains are structural ribs under thin regolith,
spoil heaps from the original excavation, deliberate windbreaks and watershed
features the builders shaped to make the drum habitable. Relief is *designed*,
and reads as designed once you know to look.

**(2) It has been weathering for centuries.** Engineered rainfall (§3.6) has
been running hydraulic erosion on that regolith the whole voyage. Valleys
incise, sediment moves downslope, drainage concentrates, soil depth varies with
slope and deposition — which feeds directly into §3.3's soil grid rather than
sitting beside it.

### 9.2 Two populations of cave

This is the payoff, and it's better than Minecraft's single noise field:

- **Eroded voids** — water working through regolith and fill over centuries.
  Follow the drainage; they are where the water went.
- **Artifact voids** — service tunnels, mining bores, structural cavities,
  buried machinery, collapsed infrastructure from construction and from
  whatever went wrong two hundred years ago.

They generate differently, look different, and mean different things. Finding a
straight cave is a *story event*. Nothing in a natural cave system is straight.

### 9.2b Chunk seams: the lattice rule

Learned the hard way in MVP-0. If each chunk builds its own flat sample grid
rotated to its own angle, neighbours never sample the same points, surface nets
skips its outermost quad row, and you get **a one-cell crack at every chunk
boundary** — visible as repeated curved lines across the terrain.

**Rule: all chunks sample ONE global cylindrical lattice, and each owns a
disjoint set of quads.** Identical samples produce identical boundary vertices,
so the world is watertight; disjoint ownership means nothing is drawn twice.
This does not contradict §2.2 — chunks are still effectively flat at their own
scale — but the *sample positions* must come from the shared lattice, not from a
per-chunk basis.

### 9.3 Technical consequence

**Terrain must be a true 3D density field, not a heightfield.** Caves,
overhangs and tunnels are volumetric by definition. This supersedes any
heightfield shortcut:

```
density(p) = regolith_surface(θ, z)        // engineered relief + erosion
           − eroded_caves(p)               // 3D fbm/worley, drainage-biased
           − artifact_voids(p)             // authored + procedural tunnel graph
           + structural_ribs(p)            // the hull's bones, undiggable
```

Meshed with marching cubes / surface nets, chunked per §2.2's flat-grid rule.
Player collision samples the field analytically rather than against mesh.

**`structural_ribs` is load-bearing in both senses.** It is the undiggable
layer, and it is what stops a player mining through the hull into vacuum —
a physical bound expressed as terrain rather than as an invisible wall.

## 10. Adopted from ORRERY

Its prioritisation rule, applied to this project when pillars conflict:

1. **Honesty of mechanism** beats cosmetic spectacle.
2. **Legibility** beats completeness.
3. **Delight in the first ninety seconds** beats feature count.
4. **Stated limits** beat implied precision.

Plus three practices worth copying outright:

- **A published model-limits doc.** Say what the biosphere sim does *not*
  claim. It buys enormous credibility and costs a page.
- **Determinism lint + golden tests in CI**, not aspiration. ORRERY already
  runs `determinism-lint`, `golden`, `parity` and `calibrate` gates; requirement
  B8 should be enforced the same way from commit #1.
- **A curated field schema** (ORRERY's `fields.js`) — every simulation state
  field registered with owner, unit and whether it's saved. This is the
  discipline that makes §5.2's save-migration requirement survivable.
