# RAMA CYCLE — MVP-0

The world you can stand in: the inner surface of a rotating space habitat, with
mountains, caves, erosion-carved drainage, a homestead and towns overhead.

**Play:** [Download builds](https://mhsenkow.github.io/ramen/) · [Releases](https://github.com/mhsenkow/ramen/releases)

Desktop builds ship for **macOS**, **Windows**, **Linux**, **Steam Deck**, and **Low Spec** Linux via GitHub Releases. The Pages site wires download buttons to the latest release assets.

## Walk around in it

```bash
/Applications/Godot.app/Contents/MacOS/Godot --path game
```

**WASD** move · **mouse** look · **Space** jump · **Shift** run
**LEFT CLICK / F** excavate (hold) — digs rock *and* mines wood/leaf blocks for timber
**RIGHT CLICK / R** fill — places timber/leaf from your pack if you carry any (blocks combine);
otherwise adds dirt · **H** fell nearest plant · **L** take spoil/timber heap
**C** brush shape (sphere / levelling) · **Z** undo
**Q/E** or two-finger side-scroll: brush size · **scroll**: camera zoom
**1-4** module · **B** drop a waypoint · **+/-** plan-view zoom
**I** field book — every readout on one tabbed page, big enough to read
**O** whole-ship map: unrolled ↔ 3D drum · **,** / **.** map size
(←/→ tab · home/end first/last · ↑/↓ scroll · **I** again to close)
**T** throw (Coriolis) · **P** tilt-shift · **TAB** switch view
**F2/F3** cycle your build · **Esc → Build** the full character creator
**Esc** menu — rebind any key, look sensitivity, FOV, invert Y, reduced motion, HUD density

**Live co-op:** Esc → **Invite friend** opens a lobby that checks for
[Tailscale](https://tailscale.com/download) (needed for different cities), then
**Host** → invite auto-copies → friend **Joins**. Same Wi‑Fi works without Tailscale.
Voice on Discord.

Gamepad works throughout: left stick moves, right stick looks, triggers
excavate and install, shoulders size the brush, D-pad handles waypoints and
plan zoom, Start opens the menu.

Headless checks:

```bash
Godot --path game -- --selftest          # sim + mesh smoke
Godot --path game -- --bisect            # hide groups; print brightest culprit
Godot --path game -- --bisect-metric=var # luminance variance (endcap patterns)
Godot --path game -- --playtest          # time first look-up / walk / dig → clipboard
Godot --path game -- --photo             # hide HUD
Godot --path game -- --quality=low       # foliage/LOD cut (§2195)
Godot --path game -- --no-threaded       # mesh chunks on the main thread
Godot --path game -- --parade            # build + gait contact sheets
python3 tools/check_shader_includes.py
python3 tools/check_shader_literals.py
python3 tools/check_multimesh_init.py
python3 tools/check_ui_contrast.py
python3 tools/check_godot.py            # every script/shader/scene loads headless
python3 tools/check_godot.py --verify   # prove that gate still catches breakage
cd sim && cargo check --all-targets     # compiles bin/bench too, which tests skip
```

Kepler Drum is **900 m radius x 6000 m long** — 5.65 km around, ~9 minutes to
run a full lap.

You wake up on the ground outside your house. Walk far enough in any direction
around the drum and you come back to where you started.

## Layout

| Path | What |
|---|---|
| `sim/` | Rust simulation core — terrain, erosion, density field, surface nets |
| `game/` | Godot 4.6 project — rendering, camera, input |
| `docs/` | Product, engineering, simulation-architecture briefs and requirements |
| `docs/LANDSCAPE_200.md` | 200 steps to a coupled, material-driven biosphere (items 1-200) |
| `docs/LANDSCAPE_800.md` | 600 more: forests, rivers, rain, mountains, graphics (items 201-800) |
| `docs/LANDSCAPE_1400.md` | 600 more: material economy, food web, evolution, memory, magic (items 801-1400) |
| `docs/LANDSCAPE_2000.md` | 600 more: agent colonists, romance from shared work, multiplayer (items 1401-2000) |
| `docs/LANDSCAPE_2200.md` | 200 more: rendering discipline, debugging, UI, audio, shipping (items 2001-2200) |
| `docs/LANDSCAPE_3200.md` | 1000 more: light, far side, biomes, species, water, weather, performance (items 2201-3200) |
| `docs/RENDER_CONTRACT.md` | Colour-space + light contract enforced by shared shader includes |
| `docs/AVATARS.md` | Bodies and gaits — the axes, the archetypes, what makes each walk read |
| `game/scripts/avatar/` | Parametric bodies (`body.gd`) and locomotion (`gait.gd`) |
| `game/scripts/controls.gd` | Every binding, in one table, registered into InputMap |
| `game/scripts/menu.gd` | Pause menu, rebinding, look + accessibility settings |
| `game/scripts/audio.gd` | Procedurally generated sound — no assets to ship |
| `game/scripts/debug/bisect.gd` | Visual bisect harness (`--bisect`) |
| `tools/check_*.py` | CI craft gates (shader includes, metre literals, MultiMesh colours) |
| `shots/` | Rendered stills |

## Rebuild the Rust core after editing `sim/`

```bash
./build.sh
```

Use the script, not a bare `cargo build` + `cp`. See the codesign gotcha below.

## Headless checks (no window, no GPU)

```bash
./sim/target/release/bench
```
```bash
/Applications/Godot.app/Contents/MacOS/Godot --path game --headless -- --selftest
```

## Mining and building

**Terrain is SDF and you sculpt it. Construction is separate prefab modules
placed on the terrain, not built from it.** That is the Astroneer/Deep Rock
model, and it fits the fiction better than blocks: a colonist excavates regolith
and installs modules, they do not stack dirt.

Digging is CSG against the procedural field:

```
dig  =  A \ sphere   ->  d = min(d, dist - radius)
fill =  A U sphere   ->  d = max(d, radius - dist)
```

The brush centre snaps to a 1 m grid, so each bite reads as a discrete chunk of
rock while the field underneath stays smooth. Only the strokes are stored —
20 bytes each — never the world. A bucketed spatial index keeps density
sampling at ~130 ns even with edits applied — and a chunk skips that index
entirely for the columns it can prove no stroke reaches.

**Bedrock is a physical bound, not an invisible wall.** The 8 m shell against
the hull returns before edits are applied, so it is genuinely undiggable. You
cannot mine your way into vacuum.

## What a chunk costs

A near chunk is the only thing on a frame that costs milliseconds rather than
microseconds, so it is the whole performance story. It used to cost **~19.6 ms**
— every frame that streamed one was a dropped frame. It now costs **~2.6 ms**.

```bash
./sim/target/release/bench     # prints mesh / paint / flat-shade per chunk
```

Four things got it there, in order of size:

- **Paint before flat-shading.** Flat shading triples the vertex count, and
  colours were being computed after it — so every corner of every triangle was
  painted, three times over the same surface point. Painting is the *expensive*
  half of a chunk, more than meshing.
- **Fill the density lattice column-major.** Surface radius, slope, strata phase
  and drainage depend only on (θ, z), so a radial column of ~30 voxels shares
  one set of grid lookups instead of paying for its own thirty times.
- **Skip the rock and the air.** The cave-tube term can never carve more than
  22 m, so anything deeper than that is unambiguously solid and is written as
  plain `r − surf`; anything more than 3 m above the surface is unambiguously
  air. Both shortcuts are *exact where the mesh can see them* — the radial band
  is derived from a per-column elevation survey, and
  `chunker::tests::shortcuts_do_not_move_the_surface` checks the mesh against a
  no-shortcut reference vertex for vertex.
- **Spread one chunk over threads.** `std::thread::scope` over disjoint slices
  of the field buffer and of the vertex array — everything read is shared
  immutably, so it is the borrow the compiler already checks. `--no-threaded`
  falls back to one thread, and so does a machine with two cores.

The **radial band is now derived, not defensive**, which is what stopped tall
relief from exploding the lattice. The consequence: anything that adds or
removes material outside `[surf − 3 m, surf + 26 m]` — an artifact bore, a shaft
you sank last night — **must** be enumerated by `Terrain::features_near`, or it
will simply not be meshed.

## Tools

**Two brush shapes.** The sphere carves caves and tunnels. The **levelling**
brush is a cylinder cut off at a plane, which is the only way to make a flat
surface — building pads, terraces, floors. A sphere brush cannot produce a
flat anything, which is why most SDF sculpting feels mushy.

**Undo** pops the last stroke and remeshes. Because the world is stored as
strokes rather than as voxels, undo is exact and free.

**Waypoints.** Drop a mark with **B**; it appears on the habitat map and the
HUD gives you its bearing and distance, alongside the way home.

## Who is in here

Colonists are **parametric**, not eight prefabs. A body is a dictionary of
numbers and so is a gait; an archetype is a named point in those spaces, and
nothing downstream knows the names.

    twink · lanky · otter · jock · daddy · muscle daddy · cub · bear

Any two of them blend, so the spread is a spectrum rather than a menu, and a
body built by hand still moves like the body it is — `gait.for_body()` derives a
walk from the shape. Mass widens the stance and shortens the stride. Muscle
locks the thorax and holds the arms off the ribs, which is why a big man's arms
swing a little and a slight man's swing a lot. Limb length lengthens the stride
and slows the cadence.

Gait phase advances with **distance travelled**, never with time, so feet cannot
skate at any speed. Each movement is placed at a phase — heel strike, toe-off,
knee peak, the dip of arriving weight — rather than layered out of sines, which
is what makes a waddle read as a waddle instead of a bigger sway.

You are one of them, and so is everyone you will meet:

```bash
Godot --path game -- --parade
```

The near six colonists get real articulated rigs and chase their sim position at
their own walking speed; everyone further out is one MultiMesh per build, baked
from the same construction, so a bear reads as a bear across a field. A
colonist's build comes from his id alone, varied per man — stable for the life
of the save with nothing to persist.

Details, and what makes each walk recognisable, in `docs/AVATARS.md`.

## Atmosphere

Daylight is a **schedule someone set**, not an orbit. A lit carriage runs the
length of the axis on a 7-minute cycle, and the sun is aimed *at it* — so when
it is still down the habitat the light rakes along the drum and hillsides have
a lit side and a dark one. Nothing crosses a sky, because there is no sky. A
cloud band sits at one radius — in a drum, "altitude" is a radius and the
condensers run at a fixed level. Dust drifts near the player.

## What is real here

- Terrain is a **3D density field**, not a heightfield — so caves, overhangs and
  tunnels exist. Two cave populations: eroded voids that follow drainage, and
  straight artifact bores. Anything straight is artificial.
- Relief is **engineered then weathered**: structural ribs and shaped high
  ground, then 140k droplets of hydraulic erosion cutting valleys and depositing
  the alluvial flats you farm on.
- Gravity is radial. There is no world "up" anywhere in the code.
- Voxel chunks are **flat Euclidean grids placed by rotation** — sagitta over a
  32 m chunk is 0.21 m, below terrain noise.
- Light comes from the day-carriage on the axis, plus a green bounce term off
  the far side. Its direction is where the carriage actually is, which is why
  morning rakes and midday does not.
- Structural ribs are undiggable and are why you cannot mine into vacuum.

## Known MVP gaps

- Far field is smooth-shaded while near chunks are faceted — the transition is
  visible if you look for it.
- Crops are static geometry — the growth model in `SIM_ARCH_BRIEF.md` §3 is
  specified but not implemented.
- Edits are in memory only; no save/load yet.
- Waypoints can only be dropped where you stand — not yet placed by clicking
  the habitat map.
- Sound is procedurally generated; ambience bed is thin.
- Soil chemistry, condensers, and the carbon plant model are next
  (`docs/LANDSCAPE_200.md` Waves 2–4).

## Live landscape

Per `docs/LANDSCAPE_200.md` + `docs/LANDSCAPE_800.md`:

- **Live drainage** with waterline outlets (sealed-cylinder fix). Dig a trench;
  routing signature changes.
- **Soil** NPK/moisture grid, **weather** condensers + Coriolis rain bands,
  **plants** with carbon allocation (shaded → leggy), **live erosion** tick.
- Paint table colours, emergent biomes, far-field frustum sectors, depth fog.
- Backlog capped in `docs/NEXT.md`. Field register in `docs/FIELDS.md`.


## Two environment gotchas, hard-won

- **Exit 137 on load was almost always codesign**, not threading. Overwriting
  the dylib in place invalidates its ad-hoc signature and macOS SIGKILLs Godot
  with zero output — use `./build.sh`. A corrupt `game/.godot/` import cache
  produces the same silent kill. Threading is being retested behind
  `--threaded` (`LANDSCAPE_3200` §BI); keep generation single-threaded until
  that gate passes.
- **If Godot dies instantly with exit 137, delete `game/.godot/` and run
  twice.** A corrupt import cache produces exactly the same silent kill. The
  extension registers on the second load; `game/.godot/extension_list.cfg` must
  contain `res://rama_sim.gdextension`.
- **Overwriting the dylib in place invalidates its ad-hoc code signature and
  macOS then SIGKILLs Godot at load with zero output.** This was the cause of
  every mystery exit-137 in this project. `build.sh` does `rm` + `cp` +
  `codesign --force --sign -`, which is why it exists.
- GDScript parse errors leave Godot sitting on an empty scene forever with
  stdout buffered, so a hang usually means a parse error. `--log-file` is the
  only reliable way to see it.
- **A MultiMesh instance colour multiplies the source mesh's vertex colour**,
  and the prop shaders then raise the product to 1.95. Two mid-tone colours
  multiply to a hundredth of what either was authored at. That was the black
  pebbles on every grassland and the black sticks in every meadow. Source
  meshes carry a luminance *ratio*; the instance carries the hue.
  (`RENDER_CONTRACT.md` §2009.)
- **A hash is not noise.** `fract(sin(dot(p, k)) * 43758.0)` of a continuous
  coordinate has no feature size, so every pixel is an independent sample and it
  aliases into radial streaks across the drum — the exact artefact the
  arc-metre rule exists to prevent. Smooth-interpolate, or do not sample it in
  world space.
- **Never scale a tuned constant by absolute metres.** `prox` in the terrain
  shader had `900.0` baked in from when the drum was 600 m; growing the drum
  silently clamped it to its floor and crushed all lighting to near-black.
- **Keep angular noise frequencies low.** `theta` spans 2pi around the entire
  habitat, so `theta * 120` is ~750 cycles and aliases into radial streaks the
  moment you look across the drum.
