# Model Limits — Kepler Drum biosphere

**Status:** v0.3 · **Date:** 2026-09-07
Companion to `LANDSCAPE_200.md` item 199 / `LANDSCAPE_800.md` / ORRERY rule 4.
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
| Sensory | Dawn/dusk choreograph fog, ambient, steam, axis glow. No rain curtain yet. |
| Soil | Full 768×512 SoA (not sparse deltas yet). Lateral moisture + N leaching. |
| Weather | Condensers + humidity + axial temp + spinward rain drift. No storm failure modes. |
| Plants | Carbon allocation headless; Godot draws boxes. Grazing pressure from trophic fields. No L-systems. |
| Bounce light | Still a hemispherical term; far-side tint from biomass not yet. |
| Coriolis on rivers | Negligible; omitted. |

When a later system claims more precision than this page allows, update this page first.
