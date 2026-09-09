# Forest plan — stages 0 and 1 delivered

**Date:** 2026-09-08 · **Against:** `FOREST_IMPLEMENTATION_PLAN.md`
**Status:** Stage 0 (baseline) and stage 1 (identity and authority, in part) done.
Stages 2–6 not started. This is a report, not a second backlog.

## Why only two stages

The plan sequences six stages with gates between them and says outright: "prefer
the smallest coherent native model; do not complete a generalized ecology
framework before showing a better grove." Stage 0 is measurement and stage 1 is
identity/authority, both of which everything later depends on. Stage 1 also
contains two bugs the plan names explicitly, so it was the honest place to start.

## Stage 0 — baseline

New `sim/src/forest.rs` measures the standing population without changing it:
species counts, height classes, nearest-neighbour spacing classes, mean/max
height, how many organisms sit at their species height clamp, and how many
trunks intersect. `O(n)` plus a uniform neighbour grid — the plan forbids
all-pairs queries, and 22k plants all-pairs is a quarter of a billion pairs.

Exposed three ways: `forest::stats()` for tests, `RamaTerrain::forest_stats()`
for route captures in GDScript, and a `bench_forest` section in `bin/bench`.

**Measured on the shipped seed** (`biosphere::tests::forest_baseline_histograms`
prints this; `cargo run --release --bin bench` reproduces it):

```
alive 22000
species  [0, 6052, 18, 8605, 20, 1594, 4431, 1280]
height   [0, 1741, 6973, 1512, 1203, 9503, 1068]
spacing  [0, 0, 790, 4660, 11124, 4744, 0]
mean h 19.3 m, max 42.0 m, at-clamp 14675 (67%)
mean nearest neighbour 18.2 m, trunk overlaps 236
biosphere build 10.6 s, stats 26 ms
```

Read against the plan's own rules, that says five things:

1. **67% of the population sits at its species height clamp.** Section 2
   predicted exactly this — "seeded biomass combined with height clamps can push
   many trees toward the same maximum". It is the single biggest reason the drum
   reads as a plantation.
2. **There is no seedling class at all** — the first height bucket is empty and
   the largest single bucket is 26–40 m. Nothing is regenerating.
3. **Three of eight species are effectively absent** (0, 18 and 20 individuals)
   while two account for 67% of the drum. Section 5.2 asks for local continuity,
   not two species everywhere and six as rounding error.
4. **Spacing piles into one 12–25 m band.** That is the signature of
   fill-to-a-global-count placement; section 5.3 replaces it so density becomes
   an outcome of community cover rather than a target.
5. **236 trunks occupy one another**, which design rule 4 forbids outright.
   Crown overlap is wanted; trunk intersection is not.

The test **ratchets** these rather than asserting the goal, so nothing gets
worse before stage 2 lands: at-clamp ≤ 70% (baseline 67%, stage 2 target under
30%) and trunk overlaps ≤ 260 (baseline 236, rule 4 wants zero). Ratchets are
meant to move down. Do not raise them.

## Stage 1 — identity and authority

### One material ledger (the conservation gate)

The plan's gate: "chopping blocks and then pressing `H` cannot pay out the
original whole-tree biomass again." It could. `mine_trees_at` paid
`economy::harvest_plant(p)` — the intact organism's biomass — no matter how much
of that tree had already been chopped into blocks and banked.

There is now one ledger, `Woodscape::taken`, in kilograms per organism:

- `harvest_sphere` attributes every removed cell to the organism that owned it,
  at the same `WOOD_KG` / `LEAF_KG` rate the payout uses (now public constants,
  one place, because the block path and the fell path have to agree on a rate).
- `economy::harvest_plant_remaining(p, taken_kg)` pays what is left. Only the
  above-ground woody and leaf fractions scale down: roots and seed are not in
  the block grid, so chopping branches has not touched them.
- Felling pays the remainder, then records the whole organism as taken, so a
  second `H` or a re-sprout cannot bill it a third time.
- Player-placed material (`u32::MAX`) never enters an organism's ledger.
  Construction is not living tissue.

Durability came free: a touched stand is never pruned, and the ledger rides in
the save beside its cells.

### Save format: `RWOOD001` → `RWOOD002`

`encode_edits` now writes `RWOOD002`: the v1 payload byte-for-byte, then
`[u32 count]` and `count × (u32 plant, f32 kg)`. `decode_edits` accepts both —
v1 loads with an empty ledger, which is correct, since v1 predates the ledger
and nothing had been billed. Validation still happens before any mutation, so a
corrupt save leaves the session intact. Migration is therefore automatic and
lossless in the direction that matters; the plan's warning about never silently
relocating edits is respected because plant ownership is unchanged (see
deferred).

### Persistent species identity (the determinism gate)

`forest_kind` resolved species from **live weather and biome on every query**, at
two call sites. A drifting climate could turn a standing oak into a pine, and a
tree's identity depended on when the camera last asked — which violates design
rule 6, "camera movement cannot select new species."

Species is now resolved **once, at establishment**: `tree_form::species_at()`
(moved out of `lib.rs`, so it sits beside the shapes it selects), called from
`Biosphere::new` immediately after seeding, and stored as `Plant::species`.
Queries read the stored byte. `plant::UNASSIGNED` (255) marks a record that
predates seating; the one fallback branch keeps an older in-memory record
degrading to the old behaviour instead of panicking.

`biome_at_site()` in `biosphere.rs` makes the same biome reading
`Biosphere::biome_at` does, but callable during construction.

## Tests added

| Test | Gate it holds |
|---|---|
| `economy::chop_then_fell_conserves_material` | chop + fell = the tree, once; over-billing cannot go negative |
| `economy::chopping_branches_does_not_take_roots_or_seed` | only above-ground material scales down |
| `woodscape::harvest_attributes_material_to_its_organism` | ledger matches payout; construction is exempt |
| `woodscape::removing_a_stand_keeps_its_ledger` | felling leaves a tombstone, not a re-payable tree |
| `woodscape::ledger_survives_save_and_v1_still_loads` | RWOOD002 round-trips; RWOOD001 still loads; corruption is inert |
| `biosphere::species_is_seated_once_and_survives_a_changing_climate` | 25 world-days of drift changes no species |
| `biosphere::seated_species_are_mixed_but_not_uniform` | not a monoculture, not confetti |
| `biosphere::forest_baseline_histograms` | the stage 0 ratchets |

75 native tests pass. `cargo check --all-targets`, the four craft gates, the
Godot load gate and its self-check all pass. Extension rebuilt via `./build.sh`.

## Explicitly deferred

**Stable `PlantId` is not done.** Ownership is still the plant's index in
`PlantSim::plants`, which the plan rules out ("never a vector index"). The
ledger and the edited-cell map are both keyed by that index, so this is one
change touching organism identity, the save format and migration together. It
is the remaining stage 1 item and should land before stage 4's paging, because
paging edited chunks off disk multiplies the cost of getting identity wrong.
Nothing here compacts or reindexes the population, so current saves stay valid.

Also untouched, and still true of the code:

- `woodscape::tick` still skips edited stands, so a cut organism is
  biologically frozen. The plan says remove that shortcut (stage 5).
- `woodscape_mesh` still calls `woodscape_lod(..., 1)` for its side effect
  rather than an explicit realization call (stage 4, section 9).
- A single edit still rebuilds the whole resident mesh; dirty chunks are stage 4.
- No site/community/patch model yet — placement is still fill-to-22,000. That
  is stage 2, and it is what the stage 0 numbers above exist to justify.
- No visual route captures. The plan requires GPU captures for visual proof and
  those belong with stage 2–3, when there is a changed grove to photograph.
