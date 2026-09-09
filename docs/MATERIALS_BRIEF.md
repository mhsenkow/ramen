# Gather → combine: a materials and crafting brief

**Status:** v0.1 preliminary · **Date:** 2026-09-08
**Subject:** what the world gives you, and what you turn it into.
**Companion:** `REQUIREMENTS.md` · `NEXT.md` · `LANDSCAPE_1400.md` (items 813–871
authored the original economy — this brief extends it, it does not redo it).

## What already existed

Worth stating first, because the honest answer to "can we add a crafting and
inventory system" is that one is already here and it is good:

- **`Inventory`** with mass *and* volume caps, per-stack grade, encumbrance
  feeding movement speed, and spill to `Stockpile` heaps when the pack is full.
- **A mass-balanced recipe engine** — `station`, `energy_kj`, `time_s`, and an
  O₂/CO₂ ledger per batch. `max_craft_scale` crafts as much as the inputs allow.
  `recipes_balance_mass` fails the build if a recipe invents or destroys matter.
- **Three id namespaces**: geological 0–9 (derived from depth, never stored),
  biological 100–109, crafted 110–138.
- **A 16-recipe chain** ending in ramen, which is the game's actual livelihood
  loop: rock → gravel/ceramic/glass/iron/lime, greens → compost/bone meal, and
  garden → flour → oil → broth → noodles → tare → bowl.

So this pass is not a new system. It is the **raw end** of the existing one,
which was much thinner than the industrial end.

## The problem worth fixing

`bowl_ramen` takes `noodles + broth + greens`. `simmer_broth` takes
`greens + bone meal`. `reduce_tare` takes `greens + ash`.

`greens` is the leaf pool of *every* plant. One undifferentiated material was
the broth base, the tare base and the bowl topping simultaneously — so every
plant in the drum was interchangeable, and **no plant was worth walking to**.
Foraging was a chore with a single output, not a decision about where to go.

That is the socket the new gatherables fill. The fix is not "more materials";
it is that the thing you already gathered now depends on *what you gathered it
from*.

## What this pass adds

### Foraged produce — the leaf pool, divided

`harvest_plant_as(plant, form)` apportions the soft-tissue pool by `tree_form`
id. The shares are **divided, not multiplied**: every form returns the same
total leaf mass, split differently.

| form | greens | flowers | fruit | veg | grass |
|---|---|---|---|---|---|
| 0 conifer | 0.85 | — | — | — | 0.15 |
| 1 broadleaf | 0.55 | 0.10 | 0.25 | — | 0.10 |
| 2 willow | 0.70 | 0.10 | — | — | 0.20 |
| 3 scrub | 0.25 | 0.30 | 0.05 | — | 0.40 |
| 4 reed | 0.20 | 0.05 | — | — | 0.75 |
| 5 orchard / farm | 0.10 | 0.10 | 0.45 | 0.35 | — |
| 6 acacia | 0.45 | 0.35 | 0.05 | — | 0.15 |
| 7 giant | 0.75 | 0.05 | 0.10 | — | 0.10 |

`harvest_splits_preserve_leaf_mass` asserts every row sums to 1.0 and that a
real plant's soft tissue comes out equal to what went in. Without that test a
share table that stopped summing to one would quietly start minting mass.

Physically these are distinguished by **bulk, not weight**: flowers are
180 kg/m³ at 1.6 bulking and grass 120 kg/m³ at 1.8, so a pack of either fills
on volume long before mass. That is deliberate — it is the reason `thatch_bundle`
exists, and the reason carrying petals is a different problem from carrying ore.

### Coal — a reductant you dig instead of make

`material::id::COAL = 9`, placed in `material_at` as buried peat: below the
active sediment, on low slope, where a low-frequency 2D field and today's water
flux together suggest a drainage sink that once pooled. Banded via a `sin` on
depth so it reads as a **seam you break into**, not blobs sprinkled through rock.

Measured at **3.15% of sampled subsurface rock** against 2.60% for ferrous ore —
coal is slightly commoner than ore, because seams are extensive where veins are
narrow, and because coal being findable is the whole point of it.
`coal_seams_are_findable_but_not_everywhere` pins the band at 0.5–8% so a future
refactor that kills the seam or floods the drum with it fails loudly.

`smelt_ferrous_coal` then beats `smelt_ferrous`: 4 kg of coal where charcoal
needed 5, more iron, and no wood spent. It costs more atmosphere, which is the
trade the O₂/CO₂ ledger already knows how to charge for.

### Molten rock, and a note on lava

You asked for lava. Nothing in a built cylinder melts on its own — there is no
heat field, no magma, no volcanism, and inventing one would be a different and
much larger brief. So molten rock exists here as **industry, not geology**:

- `melt_basalt` (smelter, 1400 kJ) turns basalt into `molten rock`
- `cast_basalt` (kiln, 240 s) cools it back into placeable basalt

That gives it an honest source and an honest sink, and a real use: turning
ore-poor spoil into building stone instead of another heap. If natural lava is
wanted later it needs a heat source in the sim first — a geothermal gradient
near the hull, or a reactor fault — and that is a decision about what the
habitat *is*, not a material to add.

### Recipes

Eight, all mass-balanced, each giving every new raw at least one sink:

| recipe | station | takes | makes |
|---|---|---|---|
| `simmer_veg_broth` | kitchen | veg + bone meal | broth (better ratio than greens) |
| `fruit_tare` | kitchen | fruit + ash | tare (better than greens) |
| `bowl_veg_ramen` | kitchen | noodles, broth, veg, oil, tare | rich ramen |
| `steep_tea` | kitchen | flowers + water | tea |
| `thatch_bundle` | mill | grass + fibre | thatch |
| `smelt_ferrous_coal` | smelter | ferrous ore + coal | iron, slag |
| `melt_basalt` | smelter | basalt | molten rock |
| `cast_basalt` | kiln | molten rock | basalt |

`bowl_veg_ramen` is the load-bearing one. `bowl_rich_ramen` wants `greens`,
which every plant gives; this wants vegetables, which only a farm or orchard
gives. Two routes to the good bowl, one of which rewards having a plot.

## Bugs found on the way

Recorded because each is the kind that hides for months:

1. **`bio_name` masked missing entries.** Its fallback was `material::name(id)`,
   which *clamps* out-of-range ids onto the last stratum. Any craft id without
   a name arm reported as `"sand"` — and became `"coal"` the instant a stratum
   was appended. Now it only defers for ids inside the palette and returns
   `"?"` otherwise.
2. **`integrate_dig_yield` hardcoded nine materials** in three arrays, a
   `.min(8)` and a `0u8..9`. A tenth stratum would have been silently folded
   into sand's bucket. Now sized from `PHYS.len()`.
3. **`recipes.toml` exists twice** — `sim/data/` (embedded by `include_str!`)
   and `game/data/` (nothing in `game/scripts/` reads it). They were
   byte-identical, so editing one was invisible. `both_recipe_copies_agree`
   now guards it. The duplicate should probably just go, but it is not mine to
   delete.
4. **`Habitat::to_world`'s third argument is `r`, not elevation.** Noted here
   because `woodscape.rs` had been calling it with an elevation; Codex's rewrite
   fixed that, and this brief is where the convention gets written down.

## Not done — the next decisions

**The crafting UI is the real gap now.** `craft_prev`/`craft_next` cycle a bare
index and `K` crafts at max scale; the result goes to `print()`, not even the
HUD. Thirty-four materials and twenty-four recipes are unreachable behind blind
cycling. Nothing above is *findable* by a player until that changes — but it
lives in `player.gd` and `world.gd`, which are mid-refactor for the tree work,
so it is deliberately untouched here.

Also open:

- **Heap and stack colour** is a hand-written `match` in `world.gd` defaulting
  to dirt brown, which is why every material added since it was written piles up
  looking like spoil. `economy::display_albedo` now supplies the colour for all
  three namespaces so that table can be **deleted** rather than extended; the
  wiring is one line in a file currently in use elsewhere.
- **Grass and flowers are not yet gatherable from the ground** — only from
  plants. The drum has a grass shader and a grass multimesh; picking from those
  needs a sim-side representation of ground cover that does not exist yet.
- **Water and lava have no container model.** Water is a stack like any other,
  which means you can carry 90 kg of it in a pack with no vessel. A `bottle` or
  `crucible` item is the honest fix and would give `melt_basalt` a reason to
  need equipment.
- **Veg only comes from form 5.** `harvest_plant_as` takes a `tree_form` id, but
  vegetables really want the FARM *biome*, which the plant struct does not carry.
  Passing the real form and biome from the caller is a small change to
  `lib.rs` — left alone for the same reason as the UI.

---

# Addendum: material physics — repose, collapse and drift

**Date:** 2026-09-08 · extends the above with how material *behaves* rather than
what it is.

## The hull is already the end of the world

Worth settling first, because it was raised as a bug and is not one.
`density_col` returns solid for anything within 8 m of the hull, and that early
return sits **before** `edits.apply` — so no stroke can reach it. Measured:
excavate a whole column with a 12 m brush past the hull radius and solid still
begins **7.77 m from the hull**, and `material_at` there is `ALLOY`.
`the_hull_cannot_be_dug_through` now pins that.

So the drum is already dirt on the inside of an impenetrable shell, and digging
everything away already leaves you standing on structural alloy. What is missing
is that **it does not look like it**. `paint.rs` is rule-based on fields, not on
material id, so the alloy floor is painted like the dirt above it — you reach
the end of the world and it looks like more ground. That is a presentation gap,
not a physics one, and it is the next thing worth doing here.

## Angle of repose, from a field that had no job

`MatInfo::cohesion` has been authored per material since item 7 and was read by
**nothing**. Meanwhile `talus_relax` relaxed the entire drum at one fixed angle,
so a sand dune and a basalt cliff slumped identically.

`material::repose_tan` now derives the angle from cohesion as
`30° + cohesion · 58°`, which lands the already-authored ordering on the real
angles:

| material | cohesion | repose |
|---|---|---|
| sand | 0.12 | 37° |
| sediment | 0.25 | 44° |
| ice | 0.40 | 53° |
| regolith | 0.35 | 50° |
| coal | 0.50 | 59° |
| clay | 0.55 | 62° |
| sandstone | 0.70 | 71° |
| ferrous ore | 0.80 | 76° |
| basalt | 0.90 | 82° |

Dry sand really does sit near 34°, and a basalt face really does stand
near-vertical — so the ordering the table already encoded turns out to be the
physical one. No new authored data was needed for repose at all.

## Drift is Coriolis, not a tuned constant

Material sliding off a face is *falling* — moving outward — and in a drum
spinning about +z an outward velocity is turned anti-spinward by `-2ω × v`.
That is the same term `player.gd`'s thrown-object integrator already applies
(`Vector3(2ω·v.y, -2ω·v.x, 0)`), so the direction is the game's own convention,
not a new one.

Integrated over a fall of `h` metres it gives `(ω·g/3)·(2h/g)^1.5`:

| face height | lateral drift |
|---|---|
| 1 m | 3 cm |
| 5 m | 35 cm |
| 20 m | 2.8 m |

Against a 3.68 m cell (2π·900 / NT=1536) that is 10% of a cell off a 5 m face
and three quarters of one off a 20 m face. So the bias is **a real length over a
real cell width**, scaled by grain fineness — not a magic number. Scree in this
habitat should lean, and it should lean one way.

`fines` is the one genuinely new piece of material data, because it is not
derivable from cohesion: clay is simultaneously the finest grain and the most
cohesive. Cohesion decides whether it moves; fineness decides how far it goes.

## The bug the symmetry test found

`talus_relax_material` is a **Jacobi** pass — all deltas computed from the
heights the pass started with, then applied together. That is not fussiness.
Written the obvious in-place way it failed
`without_spin_the_slump_is_symmetric` with a **25% anti-spinward lean at ω = 0**,
from two stacked order dependencies:

1. the fixed neighbour order `[-θ, +θ, -z, +z]` meant whichever neighbour was
   visited first was sized against the full height and the rest against an
   already-reduced one;
2. the `for t in 0..NT` sweep meant a cell's −θ neighbour had always already
   been updated this pass and its +θ neighbour had not.

Either alone is indistinguishable from the Coriolis term this function exists to
model. **The fixed-angle `talus_relax` still used by generation has the same
bias**, so the drum's generated landforms have been quietly leaning
anti-spinward for real — by accident, in the same direction physics would have
leaned them. Not changed here: generation is not the place for a surprise.

## What "stuff falling" did and did not get

Delivered: *material* falling — repose per material, collapse toward it, mass
conserved exactly, and Coriolis drift on what slumps. Tests cover conservation,
material ordering, drift direction, and symmetry without spin.

Not delivered:

- **Loose debris as objects.** Thrown items already integrate centrifugal and
  Coriolis; excavated spoil does not become a falling entity, it becomes a heap.
- **Water against fresh cuts.** `flow.rs` collapses local ponding when a dig
  opens drainage, but there is no wet-collapse coupling — saturated ground
  should have a *lower* repose than dry, and `surface_repose` already receives
  `flux`, so the hook is sitting there unused.
- **Undercut collapse.** Repose is a height-difference rule on a heightfield; it
  cannot see an overhang carved into a cliff, which is exactly the shape a
  player digs. Real undercut collapse needs the density field, not the grid.
