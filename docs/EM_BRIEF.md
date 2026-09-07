# Engineering Brief — *RAMA CYCLE*

**Status:** Draft v0.1 · **Owner:** Engineering · **Date:** 2026-09-06
**Scopes:** Option A (Slice, Early Access) unless noted · See `PD_BRIEF.md`

---

## 1. What engineering is actually being asked for

Strip the genres away and there are four distinct technical products here:

1. A **third-person open sim on a non-planar, non-flat gravity manifold.**
2. A **deep simulation** (crops, ore, fauna, weather, economy, NPC schedules)
   that must be deterministic enough to save, load and eventually network.
3. A **narrative runtime** with branching, long-horizon memory and a content
   pipeline writers can actually work in without engineers.
4. **In-build video playback** of live-action Archive footage, cleanly
   interleaved with in-engine scenes.

(1) and (3) are the interesting ones. (2) is where the schedule actually dies.
(4) is now trivial engineering — a video player and a state machine — and its
remaining cost sits entirely in production and casting, not in this team.

## 2. Platform, engine and stack — decided

*Consolidates and supersedes the four separate decision passes in earlier drafts.*

### 2.1 Decisions

| | Decision | Status |
|---|---|---|
| **Platform** | Native desktop (Win/Mac/Linux + Steam Deck). Console post-EA. **Local-first, never always-online.** | Locked |
| **Engine** | **Godot 4.6** | Locked pending S4 |
| **Sim language** | **Rust**, via `gdext` GDExtension | Locked |
| **ECS** | **`bevy_ecs`** as a standalone crate inside the sim core | Locked |
| **Voxel** | `godot_voxel` (SDF/smooth, transvoxel LOD) | Locked |
| **Art target** | **Stylised**, not photoreal | **Pending S4** |
| **Narrative** | Ink | Locked |
| **Browser** | Companion + marketing toy only. Never the game. | Locked |

### 2.2 Why Godot over the alternatives

**Over UE5:** the only thing UE5 uniquely buys is photorealism, and §2.4 argues
photorealism is the wrong target for this game. Without it, UE5's costs (C ABI
friction, royalties, content terms, heavier pipeline) are unpaid-for. Godot's
`Control` UI system is also materially better for the overlay-heavy agronomy
interface, which is this game's densest UI surface.

**Over Bevy:** Bevy is the most modern option and philosophically the best match
— this game *is* a systems simulation. It has no editor. For a 12-person team,
that makes R3 (pipeline bottlenecks on engineers) permanent and structural rather
than a manageable risk, for two years, alongside per-release breaking changes.
**Bevy is the right answer to this question in about three years.** `bevy_ecs`
standalone gets us its best part today without that bet.

**Over browser/WebGL:** storage eviction and the wasm32 address ceiling
disqualify unwrapped browser for a 40-hour save-driven sim. A Tauri/Electron
wrapper solves both, but then the web buys nothing Godot doesn't already give.

**Over custom `wgpu`:** an engine is not a renderer. Animation, audio, UI,
cinematics, profiling and an editor designers can use is roughly a year of
rebuilding. That, not draw calls, is what decides it.

### 2.3 The rule that outlives the engine

**The sim core is Rust, deterministic, engine-agnostic, behind a narrow
data-oriented boundary. The engine is a shell that renders it.**

If the boundary starts growing chatty per-entity accessors, the design has gone
wrong. Held to, this makes the shell replaceable and the core permanent — the
browser companion, the marketing toy and the dev inspector all reuse the real
simulation, and a future engine migration is a project rather than a rewrite.

### 2.4 The open question: stylised vs photoreal (S4)

`PD_BRIEF.md` §8.2 casts one actor to be the photoscan, the voice and the
Archive footage — which asserts an exact-likeness claim, and any shortfall lands
in the uncanny valley on the character the player is meant to fall for.

A stylised cast sidesteps it and is thematically stronger: nobody reads a
stylised character as a fidelity claim, so the cut to live action stops being a
continuity break and becomes the point — **the Archive is where he is actually
real.** That is exactly the diegetic framing of `PD_BRIEF.md` §8.1.

S4 decides it. If photoreal wins, reopen the engine decision; UE5 returns.

## 3. Architecture

```
┌──────────────────────────────────────────────┐
│  PRESENTATION  (UE5: render, anim, audio, UI)│
└──────────────▲───────────────────────────────┘
               │ read-only view of state
┌──────────────┴───────────────────────────────┐
│  SIM CORE  — deterministic, fixed-tick, ECS  │
│  crops · ore · fauna · weather · economy     │
│  NPC schedules · habitat structural state    │
└──────────────▲───────────────────────────────┘
               │ events both ways
┌──────────────┴───────────────────────────────┐
│  NARRATIVE RUNTIME — Ink or Yarn Spinner     │
│  affinity · memory ledger · NPC↔NPC opinion  │
└──────────────▲───────────────────────────────┘
               │ queries only
┌──────────────┴───────────────────────────────┐
│  CRÈCHE — lineage resolution, ending state   │
└──────────────────────────────────────────────┘
   ┌────────────────────────────────────────┐
   │ SERVICES (out of process): async MP     │
   │ sync · entitlements · age verification  │
   │ · CDN video delivery                    │
   └────────────────────────────────────────┘
```

**The rule that matters:** sim core is deterministic and engine-independent,
with no UE types in it. This buys you: reliable saves, replay-based bug repro,
headless test runs in CI, and async multiplayer later without a rewrite. Every
sim game that skips this pays for it in year two, in the form of a save-corruption
bug they cannot reproduce.

## 4. Curved-world tech — how, specifically

Do **not** attempt real curved-space rendering or non-Euclidean physics. The
habitat interior is ordinary Euclidean geometry that happens to be wrapped.

- **Ground:** a heightfield in cylindrical coordinates (θ, z, r). Authored in a
  flat editor space, wrapped at load. Tools work in flat space; runtime is
  cylindrical. This is the single highest-leverage decision in the project.
- **Gravity:** per-actor radial vector, `-r̂ · g(r)`. For the Bernal sphere,
  `g` varies with latitude — that's a designed feature, not an edge case.
- **Coriolis:** apply `-2Ω × v` to unconstrained ballistic actors only. Cheap,
  and it is the game's signature.
- **Floating point:** the "up" axis rotates continuously, so world-space
  assumptions in third-party middleware will break. Origin rebasing plus an
  early audit of every plugin's up-vector assumptions.
- **Navmesh and AI is the real risk, not rendering.** UE's navmesh assumes a
  world up-vector. Expect to generate navmesh in unwrapped flat space and map
  paths back onto the cylinder — prototype this in week one, because if it
  can't be made to work the habitat topology has to change.

## 5. Ranked technical risks

| # | Risk | Impact | Mitigation |
|---|---|---|---|
| R1 | Navmesh / AI pathing / physics under rotating gravity | Could invalidate P1 | **Spike S1, week 1.** Fallback: segment the drum into locally-flat cells. |
| R2 | Save-state explosion — sim × narrative × habitat edits | Ships broken saves, kills EA reviews | Versioned, migratable save format from commit #1. Never ad-hoc serialise. |
| R3 | Narrative content pipeline bottlenecks on engineers | Silently caps cast size | Writers author in Ink; hot-reload; a CI validator for unreachable nodes and broken memory refs. |
| R4 | **Casting blocks character art** — if actors are also the 3D likenesses, casting is upstream of the art pipeline | Stalls all character work | Cast in M0/M1, not M3. This is now the Archive's only schedule risk, and it is a real one. |
| R5 | **Tonal seam** between stylised 3D and live-action video reads as broken | Undermines the emotional peak of every arc | **Spike S4** — one day, phone footage, rough cut. Answer it before it's expensive. |
| R6 | Async MP determinism drift across versions | Corrupt visits, rollbacks | Server-authoritative merge of sim deltas; version-gate visits. |
| R7 | Cast scope — each character is code + art + writing + VO + *now an actor* | Classic sim overrun | Hard-lock at 6 for Option A. Data-driven character template, not bespoke. |
| R8 | Console codec/playback matrix, if consoles in scope | Late cert surprise | Confirm during platform onboarding, not at cert. |

*Dropped from the previous draft: CDN video weight, age verification, adult
payment processors, engine licence exposure. All were consequences of P5.*

## 6. Spikes before any production commitment (6 weeks, 3 engineers)

| | Spike | Kill criterion |
|---|---|---|
| **S1** | Cylinder locomotion: walk the full 360°, throw an object and see Coriolis, path an AI across the seam, place a building | Pathing or physics unfixable at reasonable cost → change topology or drop P1 |
| **S2** | Sim core headless: 100 in-game days of crops+NPC schedules in CI, deterministic, save/load/resume bit-identical | Non-determinism → renegotiate async MP out of scope |
| **S3** | Narrative vertical: one character, 3 arc beats, memory ledger referenced 20 in-game days later, authored entirely by a writer with zero engineer involvement | Writer can't work unaided → pipeline is the schedule |
| **S4** | **Medium-shift test.** Grey-box 3D scene → cut to live-action footage shot on a phone → cut back. Grade it as Archive playback: aspect change, grain, timecode. Show it to ten people cold. | Reads as broken rather than intentional → fall back to in-engine intimacy scenes. Costs one day; decides an entire pillar. |

S1–S3 gate production. S4 costs a day and should happen in week one regardless,
because its answer determines whether you're casting actors at all.

## 7. Team shape — Option A

| Role | Count | Notes |
|---|---|---|
| Engineering lead | 1 | Owns sim-core boundary discipline |
| Gameplay engineer | 2 | Locomotion, build, farm/mine/hunt loops |
| Systems/sim engineer | 1 | Sim core, save, determinism |
| Tools/pipeline engineer | 1 | Habitat authoring, narrative tooling, CI |
| Backend engineer | 0.25 | Telemetry and entitlements only until MP is funded |
| Technical artist | 1 | Curved-world shaders, tilt-shift, LOD |
| Art | 2–3 | Environment + character (scan-based) |
| Design | 2 | Systems + narrative design |
| Writing | 1 + contract | Ink authoring |
| Producer / QA | 1 + contract QA | |

~11–12 FTE — down about one FTE of backend from the previous draft.

**Plus, as a discrete contracted production (not engineering headcount):** a
short-film unit for the Archive shoot — director, DP, intimacy coordinator,
casting, 6 actors, 1–2 shoot days, editorial. Budget and schedule it alongside
VO recording, and cast early (see R4).

## 8. Milestones

| | Milestone | Duration | Exit criteria |
|---|---|---|---|
| **M0** | Spikes + S4 + casting kickoff | 6 wk | S1–S3 pass; S4 answers the medium question; casting brief out |
| **M1** | Vertical slice | 4 mo | One valley of Kepler Drum. Farm→sell→gift→one romance beat→**one real Archive scene**→one Crèche resolution. Playable end to end with one actor cast, scanned and shot. |
| **M2** | Systems complete | 5 mo | All loops in, 6 cast stubbed, save/load stable, designers authoring habitats unaided |
| **M3** | Content complete | 5 mo | Full drum, 6 arcs written + VO, **principal photography wrapped and cut** |
| **M4** | Early Access ship | 3 mo | Perf targets, localisation-ready, store compliance, telemetry live |

~23 months to Early Access, unchanged. The Archive shoot moves *earlier* (a real
scene in the vertical slice), which is deliberate: it de-risks the tonal question
while it's still cheap to reverse.

## 9. Build vs buy

| | Decision |
|---|---|
| Archive footage | **Contract a film production.** Not an ML pipeline, not a game team task. |
| Narrative runtime | **Buy** (Ink). Do not write a dialogue engine. |
| Video playback | **Buy** (Electra / platform-native). |
| Sim core | **Build.** This is the game. |
| Habitat authoring tools | **Build.** Nothing off the shelf edits a cylinder. |
| Character pipeline | **Buy** (photoscan → MetaHuman), **build** the clothing/variation layer. |
| Netcode | **Buy** the transport, **build** the merge semantics — post-EA. |

*Dropped: age verification, adult payments, video generation infrastructure.*

## 10. What I'd push back on

- **Cast the actors early or the Archive becomes a schedule risk instead of a
  feature.** If the same men are the 3D likenesses, the VO and the footage, then
  casting sits upstream of character art. That inversion is easy to miss and
  expensive to discover in month fourteen.
- **Five habitat topologies is five sets of locomotion, navmesh and tools edge
  cases**, not five art passes. One at a time, each proven.
- **Multiplayer in a farming/relationship sim is a year of work minimum** even
  async. Post-EA, funded on real retention data, not a launch commitment.
- **If S4 says the medium shift doesn't land, believe it.** In-engine intimacy
  scenes are a perfectly good game. Live-action that reads as a glitch is worse
  than no live-action.

## 11. Immediate next actions

1. Run S4 (one day, phone footage). It decides whether you cast actors at all.
2. Staff and run S1–S3.
3. If S4 passes: casting brief out in M0, because it gates character art.
4. Lock Option A scope in writing; treat B and C as unfunded.
