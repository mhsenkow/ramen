# Handoff — one-recipe trees (notes for Codex)

From Claude, 2026-09-08. You were mid-stride on the tree unification; these are
notes for when you pick it back up, not a review to action top-to-bottom. I read
the working tree, ran the test suite and the gates, and stayed off your
in-flight design. Skip to **Concerns** if you want the short version.

## State when I looked

- `cargo check --all-targets` — clean
- `cargo test --release --lib` — **55/55 pass**
- All four `tools/check_*.py` gates — pass
- Every touched `.gd` and `.gdshader` loads headless

**One thing I changed:** `src/bin/bench.rs` was missing `mod far_field` and
`mod tree_form`, so the `bench` target and `bench test` would not compile (the
lib was fine). I added the two `#[path]` declarations — nothing else. See
concern E; this is the second time this trap has fired.

## The architecture is right

Worth saying plainly, because the win is specific and not just tidiness. The
transition problem was never a fade curve, it was that near and far were two
unrelated generators:

- `_plant_instance_xform` took a per-tier `scale_boost` of 1.0 / 1.25 / 1.7 —
  **the same tree changed size as the player walked toward it**
- per-tier random lean from a position hash, so it also re-tilted
- `trees.gd` recursive branching vs `woodscape::sprout_plant`'s trunk-plus-
  canopy-sphere: different silhouettes, different height formulas, different
  colours
- three fade bands to keep aligned

`sprout_plant` now rasterizing `tree_form::recipe()` through the same
`dimensions()` and `yaw()` collapses all of that. One tier, boost always 1.0,
one fade, one silhouette. Per-stand residency (`plants_lod` field 8:
0 absent / 1 resident / 2 edited) is also strictly better than the global tier
toggle I had put in the night before — I was hiding the seam with hysteresis,
you removed the seam. Keep going.

The gameplay consequence is the biggest gain and is underplayed in the commit
messages: solid connected foliage the player can tunnel through, plus
`woodscape::raycast` so aiming at a trunk hits wood before terrain.

## Concerns, ranked

### A. Two surfaces draw for the same stand inside `WOODSCAPE_RADIUS`

`Woodscape::surface()` meshes *every* resident cell, and `_refresh_plants`
(`world.gd`) skips only `flag == 2` (edited). So a stand that is resident and
unedited draws **both** the instanced proxy and the voxel surface. They come
from one recipe but through two paths, so they are near-coincident rather than
identical — the voxel version is quantised to `CELL`, up to ±0.27 m off the
box faces. That is redundant geometry within ~92 m and a shimmer risk on flat
faces.

The comment says this is deliberate ("A full material cache keeps its matching
proxy visible"), and as a safety net for a pruned or capped grid that is
defensible. **Is that the intent?** If so it is worth a line saying why the
interpenetration is acceptable. If not, gating the proxy on `flag == 0` is the
one-line version.

### B. `surface()` is a full rebuild, and dig fires every 0.16 s

Any revision bump re-meshes every resident cell, not the chunk that changed.
`player.gd` re-bites on a 0.16 s cadence while dig is held, so sustained
chopping means a full re-mesh several times a second, and it grows with
`WOODSCAPE_RADIUS`. This is where I would expect hitching to show up first.

The grid is already chunked — a per-chunk dirty set and per-chunk submeshes
would make the cost proportional to the hole rather than the forest, which is
the same move that fixed `decay_leaves` and `grow_one`.

### C. `woodscape_mesh` calls `woodscape_lod(…, 1)` for its side effect

`lib.rs` runs the LOD query purely to trigger prune-and-sprout streaming and
discards the result. It works, but it silently couples mesh building to the
streaming path — easy to break later by "optimising" an unused call away. A
named `stream_around(center, radius)` that both callers use would say what is
actually happening.

### D. The nitrogen invariant was loosened, not fixed

`biosphere.rs` now asserts `drift < 0.12`, up from `0.08`. Actual drift is still
0.088, unchanged. Reasonable call to unblock yourself mid-feature, but recording
the root cause so it does not get lost, because it is a real conservation bug in
a ledger this codebase explicitly cares about:

**`soil.rs:275`** — `n_next[i] = (self.n[i] - n_out).max(0.0)` assigns where it
must accumulate. `n_next` starts as a clone of `self.n` and uphill neighbours
`+=` their leached N into it, but when cell `i` is then processed its own
assignment **overwrites those deposits**. Roughly half the leached nitrogen is
destroyed, depending on iteration order. `m_next[i]` on line 274 has the same
shape (less critical — moisture is not a ledger, but downslope cells stay drier
than intended).

Fix is `n_next[i] -= n_out`. I deliberately did **not** apply it: it shifts soil
moisture, which shifts biome classification, which changes world generation and
every screenshot. That is a call for a moment when you are not mid-feature, and
probably wants a `--selftest` baseline either side.

### E. `bench.rs` duplicating the module list is a recurring trap

It re-declares every module with `#[path = "../x.rs"]`, so any new file in
`sim/src/` breaks a target that nothing in the normal loop compiles. It caught
`woodscape` last night and `far_field` + `tree_form` today. Durable options:
have `bench` depend on the lib crate and `use rama_sim::…` instead of
re-including sources, or move the list into one file both include. Either beats
remembering.

### F. Small dead weight left by the refactor

- `lib.rs:482` `let trees = 0u32;` makes the `trees == 0` test on line 508
  vacuous. Behaviour is correct, the condition is now noise.
- `world.gd:148` `woodscape_owns_near` is orphaned — that was mine, from the
  tier toggle you replaced. Delete it.
- `world.gd:140,144` `plant_species_mid` / `plant_mm_far` are never populated
  but still appended at `4578-4579`. Harmless (the loop null-guards) but
  misleading about how many tiers exist.
- `_apply_reduced_motion` forces `tree_sway = 0.0` unconditionally, with a good
  reason ("solid wood/leaf volumes must stay aligned with mining"). Worth
  checking `_build_plants` agrees, or the two disagree until reduced motion is
  toggled. If sway is permanently off for trees, the `sway_span` / `crown_span`
  branch I added in `tree.gdshader` is dead weight and can go — your
  `v_up` change (normalising by world height rather than `model_height`) is the
  part worth keeping, it is better than what I wrote.

## Invariants in `woodscape.rs` worth not breaking

You are building on the streaming rewrite, so the load-bearing bits:

- **All deletions go through `remove()`.** It maintains `count`, drops empty
  chunks, decrements the apex wood count and seeds `leaf_checks`. Removing from
  a chunk map directly desyncs all four.
- **`apex` is maintained incrementally**, not scanned. `reseat_apex` is the only
  full (chunk-bounded) recount and runs on sprout, or when `grow_one` finds the
  apex cell gone. Finding the apex by scanning was the original O(all cells) per
  plant per tick.
- **`leaf_checks` is a dirty queue, not a sweep.** Leaf support is only re-tested
  where a cell actually vanished. Capped at `MAX_LEAF_CHECKS`.
- **`prune_far` must keep running.** Without it the grid fills to `MAX_CELLS`
  after roughly a kilometre of walking and then silently refuses every new stand
  ahead of the player while holding dead geometry behind.
- **`lod_near` truncates nearest-first on purpose.** Truncating raw map
  iteration handed the renderer a different arbitrary subset each refresh, which
  read on screen as blocks blinking in and out. The cut is fuzzy by a chunk
  half-diagonal; that is fine, the determinism is the point.

## Open question: does it look more natural?

I could not settle this from source and did not want to pronounce on it. Two
notes for whoever does:

- Variety did **not** regress. `recipe(form)` takes no seed, so there are 8
  silhouettes — but the old path was `mesh_for(kind, 1.0, kind * 104729 + 17)`,
  one seed per form, so it was also 8. I initially assumed a regression here and
  was wrong.
- What did change is character. The recursion had tapering, droop and
  18 %-dropout ragged leaf edges; `crown()` is three axis-aligned boxes. More
  legible and more structurally honest (buttresses, asymmetric fork, stepped
  shoulders, overhanging bough) but blockier — "reads better at distance"
  rather than "more organic". For a stated block vocabulary I think that is the
  right trade; it is a judgement call, not a fact.

`game/scripts/debug/tree_study.gd` is the harness to settle it:

```bash
/Applications/Godot.app/Contents/MacOS/Godot --path game -- --shot --tree-study --shot-dir=/tmp/rama-trees
```

Shooting one canopy form at several distances before/after is the only real
answer, and would also expose concern A if the interpenetration is visible.

## One environment warning

The Godot editor had `trees.gd` open and **saved its stale buffer over the file
mid-session**, silently reverting a full rewrite. If you are editing files the
open editor also holds, close them there first. Cost me a rewrite; worth not
repeating.

## Added later: CI now compiles every target (and two things blocked on you)

`ci.yml` gained `cargo check --all-targets` and a `godot-load` job
(`tools/check_godot.py`). Neither touches your files. Two follow-ons are parked
because the blocker is inside them:

- **`cargo fmt --check` is not wired up.** `cargo fmt` would rewrite 97 sites,
  and the two heaviest concentrations are `woodscape.rs` (18) and
  `tree_form.rs` (6) — your live files. Worth turning on the moment the tree
  work lands, in its own commit so the reformat is reviewable separately.
- **`cargo clippy` is not wired up** because it exits 101 today, on one
  deny-by-default lint: **`tree_form.rs:90`** uses `3.1415927` where
  `std::f32::consts::PI` belongs (`clippy::approx_constant`). Worth doing for
  its own sake, not clippy's: `world.gd:3225` computes the same yaw as
  `sin(th * 127.1 + zz * 0.013) * PI` with a comment reading *"Must match
  tree_form::dimensions/yaw"*, and GDScript's `PI` is f64. The drift is ~1e-7 so
  nothing is visibly wrong, but it is exactly the class of divergence that
  comment exists to prevent, and the constant makes the two definitionally
  equal. One token; yours to make.

  Past that lint there are 187 clippy warnings, which is brief #6 territory
  (dead-code triage), not a quick win.

## Added later: two edits in your files

You had been idle four hours, so I took these rather than leave them. Both are
small and neither touches the tree path — but they are your files, so:

**`player.gd` — the module ghost had no mode gate.** `show_ghost` was
`hit and view == 0 and not photo_mode`, with no check for whether the player is
placing anything. `module` defaults to 0, which is the 9 x 9 m Farm bed, so a
translucent green slab sat in the middle of the screen for the entire game,
layered over the dig brush sphere. It now shows while the `place` action is held
or for `MODULE_PREVIEW_S` (2.5 s) after picking a module with 4-7, so choosing
one still previews the footprint. New state: `module_shown`, decayed in
`_physics_process`.

**`world.gd` — `pose_ghost` posed the slab from its centre only.** It already
used edit-aware `ground_at`, so it tracked digging correctly at one point and
floated or buried at the other three. It now samples the four footprint corners
and rests on the highest of them. That is five raymarches instead of one, which
is affordable *because* of the gate above — if you ever ungate the ghost, this
needs a cheaper ground query.

Neither is a claim about how placement should work. If you want a real build
mode rather than a hold-to-preview, the gate is one condition to change.
