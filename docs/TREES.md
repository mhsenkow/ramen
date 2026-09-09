# Trees as world material

Trees use `sim/src/tree_form.rs` as their shared construction recipe. Its eight
forms produce connected timber and leaf volumes: layered conifers, forked
broadleaf crowns, willow drapes, scrub, reeds, orchard trees, acacias and giants.

The distant instances and nearby material grid use the same recipe, dimensions,
orientation and foliage palette. Distance no longer changes the tree's scale or
selects a different generator. Each resident tree uses a complementary 55–85 m
pixel fade between its display cache and harvestable cells. An unstamped tree
keeps its display cache; a touched tree uses only its edited material geometry.

Near cells are 0.55 m world-grid blocks. Only exposed faces are meshed. Mesh data
crosses the native boundary only after a material revision. Whole stands stream
nearest first, under a 140,000-cell resident budget. Failed allocations are not
repeated until memory becomes available. Unedited stands can unload; harvested
stands and player-placed material stay authoritative. Their edits, including
completely removed stands, are saved in `user://rama_wood.bin`.

Aim traces the block grid before accepting a more distant terrain hit. Excavation
removes intersected wood/leaves instead of felling by horizontal proximity.
Placement charges only after a vacant cell is accepted, and block placement and
harvesting use matching material masses.

## Verification

- `cargo test --release --manifest-path sim/Cargo.toml --lib`
- The four shader, MultiMesh, and HUD contrast craft gates in `tools/`.
- `Godot --path game -- --shot --tree-study --shot-dir=/tmp/rama-trees`
  captures the same grove at 28, 65, 82, 110 and 220 m, plus separate material
  and proxy views, and prints streaming timings.

The surface mesh is still rebuilt for the resident set when material changes;
per-chunk mesh uploads would further reduce the cost of repeated harvesting in
dense stands. Quantizing a display volume can move its surface by roughly half a
cell; the shared distance fade conceals that small geometric difference.
