# Model Limits — Kepler Drum biosphere

**Status:** v0.4 · **Date:** 2026-09-07
Companion to `LANDSCAPE_200.md` item 199 / `LANDSCAPE_800.md` / `LANDSCAPE_3200.md` / ORRERY rule 4.
See also `CALIBRATION.md` for provenance-tagged constants.

| Topic | Limit |
|---|---|
| Hydrology | Waterline cells are outlets (sealed-cylinder fix). No Manning routing. Lakes are ponded cells ≤12 m deep, not dynamic entities yet. |
| Fluvial | Generation droplets + live rainfall-weighted tick. Toy networks. |
| Materials | Eight derived types + biomass + crafted + cooking ids. Dig yields integrated. Ore grade continuous. No vein depletion yet. |
| Economy | Pack inventory. Recipes mass-balanced with O₂/CO₂. Kitchen/kiln/smelter stations gate craft. Eat ramen → satiety. Materials ledger HUD. |
| Atmosphere | Lumped O₂/CO₂/N₂. Scrubbing is power→stoichiometry. |
| Trophic | Miami NPP (Lieth 1972). Flat 10 % transfer. Kleiber densities. Discrete carcasses → detritus/soil hotspots. Fear suppresses grazers / releases plants. |
| Agents | Utility AI farmers (≤12). Trait-differentiated dig spoil. Plot discs. Chronicle Help + citation affinity. |
| Sensory | Dawn/dusk via photothermal carriage; sunset-hour flare; mist/steam; fog banks; rain curtains (condensers + storms). |
| Soil | Full 768×512 SoA (not sparse deltas yet). Lateral moisture + N leaching. |
| Weather | Spine vapor → humidity → condensers → rain. FogBank / RainStorm events from humid air + phase. Day length is a lighting schedule. Fast-day preview (`F6`) densifies rolls via `spectacle_pace`. |
| Plants | Carbon allocation headless; Godot draws boxes. Light from shared spectacle schedule. |
| Bounce light | Far-side tint from opposite-sector biome albedo (`rama_bounce_tint`); still hemispherical, not a second bounce pass. |
| Lighting | **Photothermal Spine:** dim always-on core + traveling day-carriage + endcap rings. `RamaSun` is a local directional approx of axis→hull light within the focus band (**~240 m**), proximity-scaled to carriage z, and **aimed at the carriage** — so light rakes along the drum when the carriage is still down the habitat. Held ≥ ~21° above the local horizon. One directional light, not a line source: shadows are as sharp along the axis as across it, which a 600 m strip's would not be. No orbit. No second sun. |
| Coriolis on rivers | Negligible; omitted. |
| Max elevation | ≤ 450 m authored (**440 m**); summit g ≈ ω²(R−e). Raising requires same-PR radial-band update. | LANDSCAPE_4200 |
| Provinces | Derived Worley zoning + engineered farm ring / city helix — not tectonics; never a painted map | LANDSCAPE_4200 §BM |
| Climate | Banded + aridity + lapse + orographic rain scale; not a GCM | LANDSCAPE_4200 §BQ |
| Materials | Nine derived types (+ SAND) + biomass + crafted | LANDSCAPE_4200 |
| Biomes | 14 query-time ids incl. swamp/meadow/desert/dune/shore | LANDSCAPE_4200 |
| Mesh holes | Empty chunk meshes are soft-retried; permanent null is a bug | Wave 9 |
| Chunk radial band | Derived from a per-column elevation survey plus bounded carve/offset terms, not a defensive margin. Anything that can add or remove material outside `[surf − 3 m, surf + 26 m]` — artifact bores, player strokes — must be enumerated by `Terrain::features_near` or it will not be meshed | Wave 10 |
| Chunk field shortcuts | Deep rock and open air are written from arithmetic. Exact where the mesh can read them, and `chunker::tests::shortcuts_do_not_move_the_surface` holds them to it against a no-shortcut reference | Wave 10 |
| Swimming / boats | Out of scope; deep water blocks | LANDSCAPE_4200 §BR |

When a later system claims more precision than this page allows, update this page first.
