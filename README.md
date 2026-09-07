# RAMA CYCLE — MVP-0

The world you can stand in: the inner surface of a rotating space habitat, with
mountains, caves, erosion-carved drainage, a homestead and towns overhead.

**Play:** [Download builds](https://mhsenkow.github.io/ramen/) · [Releases](https://github.com/mhsenkow/ramen/releases)

## Walk around in it

```bash
/Applications/Godot.app/Contents/MacOS/Godot --path game
```

**WASD** move · **mouse** look · **Space** jump · **Shift** run
**LEFT CLICK / F** excavate (hold) · **RIGHT CLICK / G** install module
**C** brush shape (sphere / levelling) · **R** add material · **Z** undo
**Q/E** or two-finger side-scroll: brush size · **scroll**: camera zoom
**1-4** module · **B** drop a waypoint · **+/-** plan-view zoom
**T** throw (Coriolis) · **P** tilt-shift · **TAB** switch view
**Esc** menu — rebind any key, look sensitivity, FOV, invert Y

Gamepad works throughout: left stick moves, right stick looks, triggers
excavate and install, shoulders size the brush, D-pad handles waypoints and
plan zoom, Start opens the menu.

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
| `game/scripts/controls.gd` | Every binding, in one table, registered into InputMap |
| `game/scripts/menu.gd` | Pause menu, rebinding, look settings |
| `game/scripts/audio.gd` | Procedurally generated sound — no assets to ship |
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
sampling at ~180 ns even with edits applied.

**Bedrock is a physical bound, not an invisible wall.** The 8 m shell against
the hull returns before edits are applied, so it is genuinely undiggable. You
cannot mine your way into vacuum.

## Tools

**Two brush shapes.** The sphere carves caves and tunnels. The **levelling**
brush is a cylinder cut off at a plane, which is the only way to make a flat
surface — building pads, terraces, floors. A sphere brush cannot produce a
flat anything, which is why most SDF sculpting feels mushy.

**Undo** pops the last stroke and remeshes. Because the world is stored as
strokes rather than as voxels, undo is exact and free.

**Waypoints.** Drop a mark with **B**; it appears on the habitat map and the
HUD gives you its bearing and distance, alongside the way home.

## Atmosphere

Daylight is a **schedule someone set**, not an orbit. The axis strip dims and
warms on a 7-minute cycle; nothing crosses a sky, because there is no sky. A
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
- Light comes from the axis strip, plus a green bounce term off the far side.
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

- **Do not use `rayon` inside the GDExtension.** Its thread pool gets the whole
  Godot process SIGKILLed here (exit 137, zero output) while the same code is
  fine in a standalone binary. Generation is 1.8 s single-threaded.
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
- **Never scale a tuned constant by absolute metres.** `prox` in the terrain
  shader had `900.0` baked in from when the drum was 600 m; growing the drum
  silently clamped it to its floor and crushed all lighting to near-black.
- **Keep angular noise frequencies low.** `theta` spans 2pi around the entire
  habitat, so `theta * 120` is ~750 cycles and aliases into radial streaks the
  moment you look across the drum.
