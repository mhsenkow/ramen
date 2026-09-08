# NEXT — capped backlog

**Rule (LANDSCAPE_800 item 799):** this file holds the **only** prioritised
work queue, capped at ten. The catalogues are a register, not a queue.

Last updated: 2026-09-07 · Wave 11 (avatars: builds and gaits)

| # | Item | Why now |
|---|---|---|
| 1 | `nitrogen_holds_over_100_days` drifts 0.088 vs 0.08 | A conservation invariant is failing; predates the perf wave |
| 2 | Faces beyond eyes + nose: brows, expression, gaze | These are dating options; the face is where that happens |
| 3 | Idle personality — stance, weight shift, what he does with his hands | Standing still is most of the time you look at someone |
| 4 | Sit / lean / talk poses off the same gait axes | Conversation needs somewhere to put the body |
| 5 | Clothing as a layer over the build, not baked colour | A build should be able to change his shirt |
| 6 | Contact darkening under huts and trunks (baked) | §AX 2227 — raking light exposed how much props float |
| 7 | Tilt-shift by depth, not screen Y | A drum has no horizon to hang the band on |
| 8 | Async chunk meshing (request / poll) | 2.6 ms is fine; off-frame is better |
| 9 | Golden stills: sunset / fog bank / rain storm | Visual proof of sky events |
| 10 | Playtest seed walk: name five landforms | Success metric 4199 |

**Done (Wave 11 — avatars):**
- `avatar/body.gd`: eleven build axes, eight archetypes, blend, per-id variation,
  and one `measure()` every proportion derives from
- `avatar/gait.gd`: thirty gait axes, eight walks, phase advanced by DISTANCE,
  every event placed in the cycle by `_bump` rather than layered sines
- `gait.for_body()` — a hand-built body gets a walk derived from its shape
- Player rig rebuilt from the system; **Esc → Build** is the creator; F2/F3 cycle
- Colonists at two tiers: six near articulated rigs on sticky slots that chase
  their sim position, one baked MultiMesh per build for everyone else
- `--parade` contact sheets; selftest asserts silhouette spread and the creator
- Fixed: arm swing was ipsilateral, the body basis was left-handed (he walked
  backwards), forearm pronation crossed both hands over the crotch

**Done (Wave 10 — cost and colour):**
- Near chunk **19.6 ms → 2.6 ms**: paint before flat-shade, column-major field
  fill, exact deep-rock / open-air shortcuts, scoped threads. `bench` prices
  mesh, paint and flat-shade separately so the next regression is obvious.
- Radial band derived from a per-column elevation survey plus
  `Terrain::features_near`, so a 440 m massif no longer triples the lattice
- `bin/bench` compiles again and measures the code the game actually runs
- Near ring filled before the first frame (316 ms) — the opening shot used to be
  the mid tier dithering through holes in it
- Sun aimed at the day-carriage: raking light, hillsides with two sides
- Far field fades in *before* mid dissolves — the checkerboard along every
  distant shoreline was ninety metres where neither tier was solid
- MultiMesh instance × vertex colour crush fixed (black pebbles, black sticks);
  the rule is now written into `RENDER_CONTRACT.md`
- Cloud-shadow hash → smooth noise; cloud bands meander instead of projecting
  into venetian blinds up the axis
- Claim stain is a ring near you, not a green veil across the valley

**Done — Photothermal Spine + sky weather:** SpineCore + DayCarriage + EndcapRings;
RamaSun proximity; spine vapor → humidity → condensers; spectacle clock; fog banks
+ rain storms; sunset-hour air/ring flare; **F6 / `--fast-day`** preview (×18).

**Thesis reminder:** honesty of mechanism beats cosmetic spectacle; legibility
beats completeness; delight in the first ninety seconds beats feature count.
