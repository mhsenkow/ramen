# Core Requirements — *RAMA CYCLE*

**Status:** v1.0 · **Date:** 2026-09-06
Distilled from `PD_BRIEF.md`, `EM_BRIEF.md`, `SIM_ARCH_BRIEF.md`.
Each requirement is testable. If it can't be tested, it isn't here.

---

## A. World

| | Requirement | Test |
|---|---|---|
| **A1** | The world is the interior surface of a rotating habitat. Terrain curves up on both sides and closes overhead. | Stand at spawn, look up, see inhabited land above you. |
| **A2** | Gravity is radial-outward, magnitude `ω²r`. "Up" is toward the axis. | Walk 360° around the drum and return to spawn upright. |
| **A3** | Voxel chunks are flat Euclidean grids placed by rotation. No curved-space geometry. | Sagitta over a 32m chunk ≤ terrain noise amplitude. |
| **A4** | Terrain is a true 3D density field — caves, overhangs, tunnels. Smooth SDF, not cubes. Fully mutable. | Dig into a hillside and emerge in a cave system. |
| **A4b** | Two cave populations: eroded (drainage-biased) and artifact (tunnels, bores, collapsed infrastructure). | A straight passage is always artificial. |
| **A4c** | Relief is engineered-then-weathered: structural ribs, spoil heaps, centuries of hydraulic erosion. Ribs are undiggable and bound the world. | Mining toward the hull hits structure, not an invisible wall. |
| **A5** | Ballistic objects show Coriolis deflection (`-2Ω × v`). | Throw an object; it visibly curves. |
| **A6** | There is no sky. Overhead is the far side of the habitat. | No skybox in any camera orientation. |
| **A7** | Weather and daylight are *engineered* — outputs of habitat infrastructure the player can reallocate. | Change condenser power; rainfall changes. |

## B. Simulation

| | Requirement | Test |
|---|---|---|
| **B1** | Three simulation tiers (individual / regional / habitat), each a budget for the one below. Nothing is created at a lower tier. | Sum T0 nitrogen ≤ T1 allocation ≤ T2 stock, always. |
| **B2** | Promotion/demotion round-trips without loss. Player deltas survive. | Fell a grove, leave, return 30 days later: grove still felled, regrowth plausible. |
| **B3** | Plants grow by source–sink carbon allocation, not scripted stages. | A plant in shade goes leggy with no rule authored to make it. |
| **B4** | Soil is a real substrate (NPK, organic matter, moisture, pH, microbes) with diffusion. | Continuous cropping without amendment measurably degrades yield. |
| **B5** | The habitat is a closed system. No off-world inputs. Mass conserves. | Total habitat nitrogen is invariant across 100 simulated days. |
| **B6** | Crops carry heritable traits; crossbreeding recombines; radiation raises mutation rate. | Breed two strains; offspring traits fall between parents with variance. |
| **B7** | Fauna: agent-based near the player, population dynamics at range. Extinction is permanent. | Overhunt a species to zero; it does not return. |
| **B8** | The sim core is deterministic. Same seed + same inputs = bit-identical state. | 100 headless days, replayed, byte-identical. |

## C. Game

| | Requirement | Test |
|---|---|---|
| **C1** | Farm, mine, hunt, craft, build loops, all feeding one economy. | Playable start-to-harvest-to-sale in one session. |
| **C2** | The Crèche: farm output *and* bond depth both gate colony continuity. | Neither maxed alone produces a good ending. |
| **C3** | 6 romanceable characters (Option A) with affinity, a memory ledger, opinions of each other, and a Crèche stance. | An NPC references a specific player action 20 in-game days later. |
| **C4** | Intimacy peaks are live-action Archive footage, framed diegetically as ship records. | Cut to footage reads as intentional to a cold viewer (S4). |
| **C5** | The player can browse the Archive, including prior lineages. | Archive UI lists scenes with in-fiction metadata. |
| **C6** | Async Shared Habitat multiplayer (post-EA). No player↔player romance, ever. | — |

## D. Legibility

| | Requirement | Test |
|---|---|---|
| **D1** | Town and player plots are permanently T0. Never statistically approximated. | — |
| **D2** | Diegetic instrumentation: soil probes, agronomy console, overlays for moisture/N/light/yield. | Player diagnoses a nitrogen deficit unaided (S7). |
| **D3** | Every yield result carries its causal chain. | UI shows "−22%: nitrogen deficit from day 34". |
| **D4** | Tended plots have clamped variance; wilderness does not. | A bad harvest on a tended plot always has a visible prior cause. |

## E. Technical

| | Requirement | Test |
|---|---|---|
| **E1** | Godot 4.6 shell + Rust sim core via `gdext`. Boundary is narrow and data-oriented. | FFI surface is a countable list of flat-buffer calls. |
| **E2** | Sim core has zero engine dependencies and runs headless in CI. | `cargo test` simulates 100 days with no Godot present. |
| **E3** | 60fps target. Sim ≤ 3ms amortised; meshing never on the main thread. | Frame profiler across a 20k-plant scene. |
| **E4** | Deterministic worldgen. Untouched chunks are never stored — only deltas. | A 40-hour save is bounded by what the player touched. |
| **E5** | Save format versioned and migration-tested from commit #1. | Load a save from the previous schema version. |
| **E6** | Local-first. Fully playable with all services down. | Disconnect network; nothing degrades. |
| **E7** | Native desktop + Deck. Browser only for companion/demo/toy. | — |

## F. Explicit anti-scope

Cut, in writing: real fluid dynamics · per-leaf light transport · genome-level
biology · atmospheric chemistry · habitat-wide animal pathfinding · cubic-voxel
aesthetic · explicit sexual content · player↔player romance · always-online.

## G. MVP-0 — the world you can stand in

*The current build target. Proves A1, A2, A5, A6 and the art direction.*

- [x] Wrapped cylinder terrain from a 3D density field — mountains and caves — curving overhead
- [x] Radial gravity, third-person locomotion, full 360° traversal
- [x] Player house at spawn; farm plots; a town visible arcing above
- [x] Axis light strip as the sun; no skybox; distance haze
- [x] Tilt-shift post-process (the diorama read)
- [x] Coriolis throw demo
- [x] Wake-up sequence
- [x] Habitat parameters visible and driven by config, not constants
- [x] **SDF excavation** — CSG brush, 1 m grid snap, stroke-only storage
- [x] **Undiggable bedrock shell** bounding the world physically
- [x] **Prefab module placement** on terrain, grid-snapped
- [x] Faceted flat shading for the stylised low-poly read
- [x] Chunk streaming with unload; 900 m x 6000 m drum
- [x] **Watertight chunks** — one global cylindrical lattice, disjoint quad
      ownership, no seams
- [x] Articulated colonist with a procedural gait driven by distance travelled
- [x] Camera zoom to first person, smooth follow, radial ground clamp
- [x] Engineered day/night cycle, cloud band, dust
- [x] **Three views**: colonist, habitat survey (biome census), local plan —
      the latter two also live permanently on the HUD
- [x] Levelling brush (cylinder + plane) alongside the sphere brush
- [x] Undo, waypoints with bearing, plan-view zoom
- [x] All input via InputMap: rebindable keys, mouse and gamepad on one path
- [x] Pause menu with persisted settings (`user://controls.cfg`)
- [x] Procedural audio — excavation pitch tracks brush size
