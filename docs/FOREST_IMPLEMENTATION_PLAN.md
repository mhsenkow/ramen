# Forests as landscape — implementation plan

**Date:** 2026-09-08  
**Audience:** Cursor Composer implementing this repository  
**Status:** Proposed work, grounded in the current code; not an implementation report.

## 1. Outcome

Make forests feel like places that grew together. A player should recognize a
woodland from across the drum, approach an irregular edge, enter beneath a
shared canopy, find young growth in a clearing, follow wetter vegetation along
a drainage line, and emerge onto genuinely open ground. The trees, roots,
branches, fallen timber and undergrowth should belong to the same harvestable
material world.

The next pass should deliver **coherent forest communities and readable space
between trees**, not simply more trees, more random species, or more elaborate
individual models. A forest has both occupied volume and useful negative space.

Preserve the block/material vocabulary and the shared tree recipe already in
place. Biological records are useful simulation data; they must not make a tree
an indivisible gameplay prop. Rendering instances are caches, not another source
of matter. Cutting a branch changes the landscape itself.

This plan defines one implementation scope. It does not replace `NEXT.md` with a
second global backlog. Preserve unrelated working-tree changes, especially the
ongoing material, economy, terrain, UI and CI work.

## 2. Current code: what to keep and what to change

Read these files before implementation. Inspect executable paths rather than
assuming older comments and briefs describe current behavior.

| Area | Current entry points | Consequence for this work |
|---|---|---|
| Initial distribution | `sim/src/plant.rs`: `seed_stands`; `biosphere.rs`: `Biosphere::new` | Samples terrain lattice positions until a target of 22,000 stands, subject to elevation, flux, moisture and province checks. There is no explicit grove, gap, cohort or crown-spacing model. |
| Species selection | `sim/src/lib.rs`: `forest_kind` | Species form is inferred from current biome/climate and genome when queried. Establish persistent ecological identity so weather or a biome threshold cannot change an existing tree into another species. |
| Shape and size | `sim/src/tree_form.rs`: `recipe`, `dimensions`, `yaw`, `pigment` | One recipe per form, with shared material volumes. Preserve this unification, but introduce bounded, stable variation. Audit size distributions: seeded biomass combined with height clamps can push many trees toward the same maximum. |
| Plant simulation | `plant.rs`: `tick`, `allocation_step`, `build_leaf_grid` | Carbon allocation, stress and mortality exist. Shade is a coarse nearby leaf sum, without crown height or species response. There is no explicit recruitment/succession loop in this path. |
| Material realization | `sim/src/woodscape.rs` | Sparse 0.55 m cells, exposed-face meshing, resident budget, pruning, touched-stand persistence and material raycasts already exist. Keep these foundations. |
| Growth consistency | `woodscape.rs`: `tick`, `grow_one` | Material growth is resident-only and skips edited stands, while biological pools continue evolving separately. A forest must not grow differently because a camera visited it. |
| Representation | `lib.rs`: `plants_lod`, `woodscape_mesh`; `game/scripts/world.gd`: plant/woodscape functions | Per-stand residency and complementary 55–85 m fades exist. Queries still retain old ring caps; distant trees retire around 480 m. Across-drum forest cover needs a representation beyond individual near-range instances. |
| Harvesting and economy | `lib.rs`: `dig_ex`, `mine_trees_at`, `harvest_near`; `economy.rs` | Block excavation and whole-plant harvesting both exist. They must consume the same remaining material ledger, including after partial chopping. |
| Ground layer | `world.gd`: grass/litter builders; `lib.rs`: `grass_field`; `soil.rs` | Ground cover is largely placed independently of actual crowns. Couple its distribution to canopy, moisture, disturbance and litter; give gatherable coverage an authoritative quantity. |
| Persistence | `woodscape.rs`: edit encoding; `world.gd`: save/load | Current edited-cell retention is useful, but plant ownership uses array indices and saved edited cells share a runtime cell cap. Stable identity and disk-backed edited chunks are prerequisites for long-lived expanding forests. |

Companion documents: `TREES.md`, `HANDOFF_TREES.md`, `MATERIALS_BRIEF.md`,
`RENDER_CONTRACT.md`, `SIM_ARCH_BRIEF.md`. They are context, not permission to
rewrite unrelated systems.

## 3. Design rules

1. **Landscape first, community second, tree third.** Water, exposure, soil and
   land use determine where woodland is plausible. A community determines who
   grows together. Individual variation operates within that community.
2. **Forest density is not uniform spacing.** Use dense pockets, lightly wooded
   margins, open interiors beneath mature crowns, regeneration thickets and gaps.
   Keep real meadows and exposed ridges open.
3. **Species mixes have local continuity.** A grove has a recognizable dominant
   form and a few compatible companions. Avoid a random eight-species sample at
   every point and avoid monocultures with identical sizes.
4. **Trees respond to their neighbors.** Open-grown crowns spread; crowded trees
   narrow and reach upward; edge trees grow toward open light. Nearby crowns may
   interleave. Trunks should not occupy one another.
5. **Matter and biological ownership are separate.** Touching wood shares a
   geometric boundary; adjacency does not silently merge two organisms or make
   player construction living tissue.
6. **Distance changes representation, never existence.** Camera movement cannot
   select new species, change mass, erase edits, trigger extra growth, or remove
   an otherwise visible forest because a rendering list filled.
7. **Quality settings change cost, not ecology.** The same seed and simulation
   time produce the same forest and harvestable material on every quality tier.

## 4. Proposed data model

Use compact native data and batched queries. These are responsibilities and
suggested names, not a mandate to add one class/file per item.

- `ForestTileId`: a stable tile in wrapped arc-metres × axial metres. Tile size
  should be configurable; start around 32–64 m and measure. Evaluate a halo for
  cross-border relationships, but give each organism exactly one owning tile.
- `ForestSite`: long-term moisture, flood exposure, slope, soil depth/fertility,
  light/exposure, land-use influence and disturbance history. Reuse existing
  fields and document missing approximations. Do not invent an independent
  weather or hydrology simulation.
- `CommunitySpec`: dominant/companion species weights, desired canopy cover,
  cohort mix, gap characteristics, edge response and understorey palette.
- `SpeciesSpec`: stable species ID, growth form, mature size range, crown/trunk
  allometry, shade/moisture tolerance, recruitment response, material palette and
  produce definitions. Form ID is a shape family, not a species or biome ID.
- `PlantId`: stable integer identity derived from seed, generator version,
  owning tile and candidate/recruitment identity. Never a vector index. Keep full
  integer IDs out of float32 buffers; use a parallel integer array or typed batch.
- `PlantState`: ID, species, root anchor, cohort/age, biomass, health, persistent
  morphology seed, growth stage, material revision and disturbance state.
- `MaterialChunk`: generated material plus durable edits and a revision. Track
  material kind, relevant provenance, live/dead status and support separately.
- `CanopyField`: coarse crown occupancy, height bands, transmitted light and
  litter influence. Shared by ecology, undergrowth and distant representation.

Separate three scales of state: habitat/community fields; organism records;
material chunks. Only the last needs dense detail near interaction. Existing
`Plant` pools can migrate incrementally into this model.

Make one native resolver produce the final morphology parameters used by both
proxies and voxelization. Remove duplicated size/palette logic in GDScript rather
than maintaining two formulas with comments promising they agree.

## 5. Generate forests, then populate them

### 5.1 Site suitability and continuity

Derive continuous suitability from current terrain, water, soil and province
fields. Include slope/root support, inundation duration or a documented initial
proxy, and exposure. High drainage flux does not mean every wet place is equally
suitable: distinguish banks, waterlogged basins and dry elevated terraces.

Create broad woodland patches using low-frequency, seam-safe deterministic
fields modulated by suitability. Add smaller patches and irregular gaps inside
those regions. Noise organizes plausible sites; it must not override water,
cliff, settlement or cultivation constraints.

Initial art-tuning ranges: broad patches roughly 80–300 m, local groves 15–60 m,
gaps 6–25 m. These are proposed visual scales, not scientific constants. Verify
that they read at the actual habitat scale and preserve traversable open areas.

Use periodic coordinates around the cylinder. In local neighbor queries, use
shortest wrapped arc distance. Clamp or explicitly mask the endcaps. Root world
positions use `radius - elevation`; `Habitat::to_world` takes radius, not height.

### 5.2 Community assignments

Choose a small weighted assemblage for each patch, then blend adjoining patches
through ecotones. Make site and disturbance history determine the mix.

| Community | Character and composition | Ground layer and spacing |
|---|---|---|
| Moist mixed woodland | Broadleaf canopy with compatible conifers and occasional larger elders | Irregular overlapping crowns, shaded litter floor, saplings concentrated in gaps |
| Cool upland grove | Conifer-dominated; broadleaf companions in sheltered pockets | More exposed rock, lower ground cover, broken edges at ridges |
| Riparian gallery | Moisture-tolerant broadleaf/willow along suitable banks | Long connected patches following drainage, reeds nearer water, walkable interruptions |
| Dry open woodland | Broad-spreading, drought-tolerant trees with scrub companions | Open sunlit spaces and patchy dry grass; fewer continuous crowns |
| Young disturbed stand | Locally compatible pioneer species and dense young cohorts | Clumps around an opening with surviving mature trees; gradually closing canopy |
| Managed orchard / settlement edge | Deliberate planted composition | Recognizable rows or setbacks where authored land use calls for them; natural woodland should not inherit these patterns |

Reeds and shrubs belong to appropriate vegetation layers, not merely to the same
random tree distribution scaled down. Forests should offer different useful
materials and produce through the existing economy, without making every green
surface interchangeable food.

### 5.3 Candidate placement and spacing

Replace fill-to-global-count placement with deterministic candidates inside
suitable patches. Density is an outcome of community cover, size and site
capacity; retain a global cap as a resource guard, not an artistic target.

Use a two-scale process: patch/cohort attraction plus local minimum trunk/root
spacing. A variable-radius inhibition rule with an inexpensive spatial hash is
sufficient. Crown overlap can be intentional; trunk intersection is not. Resolve
conflicts using stable candidate priority, independent of tile processing order.

Place mature framework trees first, then companions and gap cohorts. Exclude
understorey candidates where mature shade prevents establishment. Jitter within
terrain cells so there are no visible lattice rows. Sample root support over a
small footprint, not only the center. Reject unsuitable ground before building
any detailed tree geometry.

Validate the resulting size histogram. Seedlings, saplings, poles and mature
canopy should all be legible; elders are accents. Do not drive most generated
biomass values into `dimensions()`'s upper clamp.

## 6. Make individuals fit the stand

Extend the shared recipe with deterministic variants: branch hierarchy, fork
height, crown aspect, missing sectors, moderate lean, trunk taper and a bounded
palette shift. Derive variation from persistent morphology seeds, not query
order or frame time. Avoid making every tree a uniformly resized copy.

Resolve competition at coarse ecological intervals. Sample directional canopy
openness to bias branches into gaps; suppress lower branches under sustained
shade; let edge crowns spread outward. Root/trunk bases remain seated and
stable when a growth stage advances. Do not warp entire trees every frame or
rescale a harvested trunk back into an intact recipe.

Use a small quantized morphology library for distant instance batching, with
any residual local detail derived from the same parameters. If simplification
changes the envelope, measure the error instead of introducing an arbitrary
per-distance scale boost.

Trunks and major branches are connected wood. Leaf clusters are connected
material volumes with support metadata. Fine shading or leaf-pattern detail may
represent texture within existing material; it must not pretend to be a separate
harvestable object. Roots can initially be a compact support footprint with
visible buttresses; a detailed below-ground root voxel network is not required
for the first delivery.

## 7. Forest floor and combinations

Build undergrowth from actual canopy occupancy and site conditions. Reuse the
same fields for ground appearance, coverage quantities and placement decisions.

- Mature interiors: sparse sun-loving grass, patchy shade-tolerant growth,
  accumulated litter, visible roots and occasional fallen timber.
- Gaps: stronger grass/herb growth and cohorts of seedlings where seed sources
  and soil permit it. Clearing a crown should eventually change its light patch.
- Wet margins: localized reeds and moisture-tolerant growth following water,
  with interruptions caused by rocks, banks and inundation.
- Dry edges: discontinuous grass/scrub and exposed substrate; do not carpet every
  square metre with the same cone-shaped tuft.
- Fallen wood: starts where a tree or branch was lost, rests on terrain, and
  remains harvestable. It is not an unrelated random log decoration.

For grass, litter and tiny plants, use per-tile material coverage/quantity fields
with render instances sampling them. A brush harvest depletes a local quantity
and changes its visual cover. Do not create one simulated entity per blade.
Flowers and fruit should consume or transform a recorded reproductive pool and
match the actual plant/site identity; integrate `MATERIALS_BRIEF.md` rather than
creating duplicate material IDs or recipe systems.

## 8. Lifecycle and interaction

Run ecological updates on a deterministic, budgeted schedule independent of
visibility. Use existing simulation time and nearby spatial indices. Coarse
state should update whether the player is present; material chunks realize the
current state when needed.

Implement growth in discrete, persistent stages. All growth consumes the same
biomass ledger and produces bounded geometry changes. Death changes living wood
into dead standing/fallen material; it does not simply toggle a mesh off. Leaf
loss transfers to litter/decomposition pools, with documented simplifications
for moisture and nutrient return.

Recruitment uses local seed availability, light, moisture, substrate and space.
Use deterministic time epochs and candidate IDs, not an unbounded per-tree
per-frame search. Gaps favor recruitment; established shade limits it. Initial
world generation may directly synthesize plausible cohorts from patch history;
do not require a long full-resolution simulation warm-up just to start a game.

Cutting, whole-tree harvesting, agent work and natural loss must use a shared
material-removal transaction. In particular, chopping blocks and then pressing
`H` cannot pay out the original whole-tree biomass again. Inventory overflow
must become accounted world material. Placed timber remains construction unless
an explicit planting/grafting mechanic says otherwise.

Remove the permanent “edited means biologically frozen” shortcut. A cut organism
can survive, die or produce new shoots according to its remaining support and
resources, while the cut remains a persistent historical edit. New growth gets
new material; it must not erase the edit mask or restore removed mass for free.

A bounded support graph can handle detached branches. For the first pass,
convert disconnected material into terrain-seated deadwood or an accounted
stockpile, preserving mass. A rigid-body simulation for every cell is not
required. Never delete a neighboring tree or an attached player structure merely
because cells touch a harvested trunk.

## 9. Streaming, meshing and distant forests

Make material realization an explicit operation rather than calling
`woodscape_lod(..., 1)` for an unused return value to trigger streaming.

Use dirty chunks and revisioned batched mesh results. A changed boundary cell
invalidates its chunk and the neighboring face-sharing chunks. A single branch
cut should not rebuild all 140,000 resident cells. Build meshes off the main
thread using immutable snapshots; reject stale results by revision. Commit a
replacement atomically and keep the previous valid mesh until ready.

Maintain a protected interaction neighborhood, including ray/brush reach and
prefetch margin. If material is not ready, prioritize realization before
accepting the edit. A visible nearby trunk must not be unharvestable because
other stands filled the cache. Evict reconstructible distant chunks before
refusing interaction; bound staging allocations and count them in memory use.

Split durable edits from active render residency. An edited forest kilometres
behind the player belongs on disk plus compact metadata, not permanently in the
resident mesh. Support incremental save/load, deletion tombstones, generator
versioning and checksums. Do not compact/reindex organisms in ways that change
saved ownership. Define migration for existing index-owned `RWOOD001` saves;
if deterministic mapping cannot be guaranteed, retain the old generator for
that save or require an explicit new world. Never silently relocate edits.

At medium distance use stable instances/cluster meshes sharing resolved
morphology. Beyond individual-tree range, represent canopy cover, height and
palette in coarse forest patches or far-field material/geometry, with bounds
for culling. Extend coverage to across-drum views without instantiating every
leaf. Match patch edges and clearings to the ecology; fog must not conceal a
camera-centered empty ring. Edited clearings propagate into this far field.

Keep complementary dither transitions and stable bounds. Do not “fix” the
existing overlap by hiding every resident proxy at all distances: the material
representation fades out, so doing that without revising both sides creates a
missing band. Test the complete handoff while moving in both directions.

## 10. Implementation sequence and completion gates

Deliver these as reviewable stages. Prefer the smallest coherent native model;
do not complete a generalized ecology framework before showing a better grove.

| Stage | Work and likely files | Must be true before continuing |
|---|---|---|
| 0. Baseline | Extend `tree_study.gd`/native benchmark with fixed grove, edge, wet-bank and across-drum routes; record current counters | Repeatable captures and timing distributions; existing tests still pass |
| 1. Identity and authority | Stable IDs/species; native resolved morphology; unify removal accounting in `plant.rs`, `tree_form.rs`, `woodscape.rs`, `lib.rs`, economy integration | Partial chop + H + reload conserves material; streaming cannot change identity or dimensions |
| 2. First forest slice | Site/community fields, deterministic patch candidates and local spacing; replace global-count placement in `seed_stands` | One mixed woodland has a convincing edge, interior, gap and open surrounding ground; different seeds retain these relationships |
| 3. Layered communities | Wet-bank, upland and dry mixes; cohorts; canopy-driven grass/litter; stable morphology variants | Communities read differently without labels; floor responds to crowns; no obvious rows, identical maximum-height population or species confetti |
| 4. Resident performance and persistence | Dirty chunk meshes, indexed queries, explicit realization, edited-chunk paging and save migration | Chopping is local work; walking away frees resident memory without losing edits; interaction remains available under pressure |
| 5. Lifecycle | Camera-independent staged growth, light competition, recruitment, deadwood and decomposition transfers | Clearing creates a persistent gap with plausible later regeneration; loaded/unloaded schedules agree within documented tolerances |
| 6. Whole-drum integration | Far canopy coverage, quality tiers, final route captures, regressions and documentation | Groves remain in the same places at all distances and quality settings; edited gaps remain visible from afar |

Stages 2–3 are the principal visual delivery. Stage 4 must precede increasing
forest density substantially. Support identity/save changes from stage 1 with
migration tests immediately, even if paging is delivered later.

## 11. Performance contracts

Measure on named hardware at fixed resolution/quality. The following are initial
acceptance targets to tune against the baseline, not claims about current speed:

- No full habitat scan or whole-forest mesh rebuild on an ordinary single cut.
- Stable standing still: no geometry uploads without material/representation
  revisions and no repeated failed voxelizations under unchanged memory pressure.
- Ecology stays amortized and approximately linear in active records/touched
  neighborhoods; avoid all-pairs crown/seed queries.
- Target forest-related main-thread work below 2 ms p95 during walking and
  4 ms p95 during continuous harvesting on the agreed desktop baseline. Report
  p99, maximum and cold-population cost too; do not hide stalls with averages.
- Low/deck settings reduce mesh detail, shadows and update/upload budgets while
  preserving canopy layout, material amounts, IDs and edit results.
- Track resident cells/bytes, dirty chunks, staging bytes, queued jobs, proxy
  counts, triangles, draw calls and saved-edit bytes separately.
- Repeated walking loops and visits to previously edited groves reach a stable
  resident working set. Saved world size may grow with actual edits; renderer
  memory must not grow with the total number of visited trees.

Do not raise caps, simplify whole forests into generic billboards, or reduce
material persistence merely to make the benchmark green.

## 12. Verification and definition of done

### Deterministic and material tests

- Same seed/version/time, different tile visitation and worker order: identical
  IDs, species, root anchors and community boundaries, including theta wrap.
- Community suitability, spacing, crown-cover and height histograms stay within
  authored bands over several seeds. Test deliberate clearing and open-meadow
  fixtures, not only the existence of some trees.
- Near material and proxy resolve the same morphology revision and bounds within
  explicit voxel quantization tolerance. Edited geometry and distant canopy agree
  on removed areas.
- Ray/brush interactions at chunk edges, partial harvest followed by H/agent
  harvest, full inventory spill, save/reload, growth and leaf loss conserve the
  relevant material ledger. Invalid saves fail without partially changing state.
- Edited, completely removed and player-built material survives eviction,
  restart, ID storage changes and supported save migration.
- Cut one of two touching trees: ownership/support changes are local; the other
  organism and player-built wood are not silently reclassified or deleted.
- Loaded and unloaded forest evolution agree for the same simulated duration;
  quality and camera path do not alter ecology.
- Dirty-face seam tests and out-of-order mesh completion tests prevent holes,
  duplicate faces and stale resurrection after edits.

### Visual routes

Capture fixed-seed wide views and walking sequences, not only isolated model
portraits: meadow → forest edge → mature interior → sunlit gap; stream bank →
wet margin; exposed ridge → sheltered grove; a cut grove before/after/reload;
slow forward/backward movement through every representation boundary; opposite
wall and plan/survey views. Include clear midday and low/raking light so both
structure and grounding can be judged. Include low/deck quality.

Approve the result when forests form connected, varied places; clearings and
open land remain legible; materials can be removed where they are seen; trees
retain identity and history across distance; and measured costs remain bounded.
A denser screenshot alone does not meet the goal.

### Repository checks and handoff

Run the relevant native tests, `cargo check --all-targets` (the benchmark has
its own module declarations), existing shader/MultiMesh/UI gates, and the Godot
load gate. Use `./build.sh` when rebuilding the extension so the macOS library
is installed and signed correctly. Use GPU captures for visual proof; a
headless dummy renderer cannot validate appearance.

Hand back changed files, the new data/schema version and migration behavior,
representative route images/video, before/after timing distributions, and any
explicitly deferred lifecycle approximation. Do not weaken existing assertions
to accommodate an unexplained ecological or conservation regression.
