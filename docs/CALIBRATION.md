# Calibration table — empirical constants with provenance

**Rule (LANDSCAPE_1400 items 1387–1390):** every constant that shapes
behaviour is listed here with a tag. Invented values are marked *invented*.
`MODEL_LIMITS.md` is updated before any system claims more precision than this
table allows.

| Symbol | Value | Provenance | Used by |
|---|---|---|---|
| Miami NPP_T | `3000 / (1 + exp(1.315 − 0.119 T))` | Lieth 1972 | `trophic::miami_npp` |
| Miami NPP_P | `3000 (1 − exp(−0.000664 P))` | Lieth 1972 | `trophic::miami_npp` |
| Transfer efficiency | 0.10 | Ecology textbook default; *flat* | producer→grazer→hunter |
| Kleiber exponent | 0.75 | Kleiber 1932 | density / home range |
| Rain→mm/yr scale | `rain × 365 × 2.2` | *invented* | Miami precip input |
| Per-capita metabolic | `M^0.75 × 50×1000 g/yr` | *invented* | Kleiber density |
| Fear decay | `0.12 / day` | *invented* | landscape of fear |
| Kill rate | `hunter × grazer × 0.55 / day` | *invented* | carcass spawn |
| Carcass stages | 1.8 / 5.5 / 14 days | *invented* | decay → soil hotspot |
| Arable yield | 0.45 kg dry / m² / yr | Wheat 3–5 t/ha; midpoint | `dwelling::capacity_of` |
| Ration | 0.60 kg dry / person / day | ~2200 kcal at 3.7 kcal/g | `dwelling` production/consumption |
| Labour ratio | 2.4 people fed per worker | Pre-industrial intensive farm ratios; order-of-magnitude | `dwelling::tick` |
| Claim radius | 90 m | *invented* — legibility, not agronomy | `dwelling::CLAIM_R` |
| Arable slope cut-off | 0.25 rise/run | *invented* | `dwelling::survey` |
| Structure labour | 16 fed labour-days | *invented* | `dwelling::WORK_DAYS` |
| People per structure | 1.5 | *invented* | `dwelling::PEOPLE_PER_WORK` |
| Delve capacity cap | 3 people | *authored* — a delve is cut rock, not a farm | `dwelling::site` |
| Drum diagonal | `sqrt(L² + (2R)²)` = 6264 m | *derived* | camera far, fog end, haze scale |
| Camera far plane | diagonal × 1.15 | *derived* | `player.cam_far` |
| Engine fog end | diagonal × 1.15 | *derived* | `_build_env` |
| Terrain haze scale | length × 0.40 | *invented* — Beer-Lambert e-fold | `terrain.gdshader` |
| Pack mass / volume | 60 kg / 55 L (player) | *authored* | encumbrance |
| Greenhouse glass | 12 kg | *authored* | place_greenhouse |
| Shadow max distance | tilt-shift focus ≈ **240 m** | *invented* — diorama band, not drum size | `RamaSun.directional_shadow_max_distance` |
| Shadow splits | 2 parallel; blend on; split₁≈0.12 split₂≈0.38 | *invented* — fade before far wall | `_build_rama_sun` |
| Sun noon energy | ≈ 0.42 + 1.35×day → ~1.77 noon | *invented* lux fiction | `_aim_rama_sun` |
| Sun **direction** | toward the nearest lit point of the day-carriage | *derived* — the carriage is where the light is; a dead-axis strip gives every hour the same overhead noon | `_aim_rama_sun` |
| Sun minimum elevation | sin 0.36 (≈ 21°) above the local horizon | *invented* — below it a directional light is all cascade artefact | `_aim_rama_sun` |
| Terrain shader sun_energy | 1.75 | *invented* — match RamaSun noon | `_terrain_material` |
| Bounce sector sample | 12 far sectors, opposite half | *derived* from `FAR_SECTORS` | `rama_bounce_tint` |
| Dawn-wave duration | `hab_len / 300` ≈ 20 s local edge; full day = `DAY_LENGTH` 420 s | *invented* — **implemented** as day-carriage transit | `_carriage_progress` / `_sync_photothermal_spine` |
| Day-carriage length | 10% of habitat length | *authored* | `CARRIAGE_FRAC` |
| Carriage→sun falloff | 900 m along z | *invented* | `CARRIAGE_PROX_M` / `_aim_rama_sun` |
| Spine vapor rate | 0.045 humidity / habitat-day | *invented* — evaporator half of water loop | `weather.spine_vapor_rate` |
| Endcap ring radii | 28–42 m about axis | *authored* | `_make_endcap_ring` |
| Spectacle→sim clock | Godot phase + `day` → `set_day_schedule` | *authored* — one schedule for picture + plants | `Weather::set_day_schedule` |
| Fast-day multiplier | `FAST_DAY_MULT` 18× (`F6` / `--fast-day`) | *authored* — preview dusk / fog / storms | `world.day_speed` |
| Spectacle pace | `√day_speed × 1.35` when fast | *invented* — denser sky-event rolls | `Weather::set_spectacle_pace` |
| Fog bank duration | ~0.16–0.30 habitat-days | *invented* — sudden cool-hour mist | `weather::tick_sky_events` |
| Rain storm duration | ~0.28–0.50 habitat-days | *invented* — mid-day wet-air dump | `weather::tick_sky_events` |
| Sunset air | copper→rose phase 0.68–0.88 | *authored* — carriage exit flare | `_tick_daylight` / spine rings |
| Water sheet near fade | 2.5–14 m (was 160–320) | *authored* — lake_mask clips dry land; seas must read underfoot | `water.gdshader` |
| Max elevation | **440 m** | *authored* — summit g ≈ 0.51× hull; **do not raise without radial-band PR** | `habitat::max_elevation` |
| Chunk radial lo margin | `SOLID_DEPTH` 26 m + 2, then bores/strokes | *derived* — max cave-tube carve is 22 m, so nothing below that is air | `chunker::chunk_mesh` |
| Chunk radial hi margin | 6 m over the highest lattice column | *derived* — strata offset is bounded at +2.22 m | `chunker::chunk_mesh` |
| Chunk elevation survey | every lattice column, not a 9×9 grid | *derived* — exact at the only points the mesher reads, so the band needs no defensive slack | `chunker::chunk_mesh` |
| Chunk lattice pad | 4 cells (5.6 m) each side | *derived* — widest AO probe is 4.8 m; occlusion must match across a seam | `chunker::AO_PAD` |
| Max cave-tube carve | 22 m | *derived* from `k²·22·fade` in `density_col` | `Terrain::MAX_TUBE_CARVE` |
| Chunk mesh threads | cores − 2, capped 6 | *invented* — leave Godot its render and physics threads | `chunker::worker_count` |
| Lapse rate | 0.0065 °C/m | Earth troposphere fiction | `weather::temp_at_elev` |
| Province blend | 280 m | *invented* | `province::BLEND_M` |
| Province lattice | 16 around × 10 along | *authored* | `province.rs` |
| Farmland band | axial ring ~1 cell (~10% length), full θ | *authored* — discoverable O'Neill ring | `province::engineered_band` |
| City corridor | helical strip 1/7–1/8 of θ | *authored* — cuts farm at crossings | `province::engineered_band` |
| Talus repose tan | 0.72 (~36°) gen pass | *invented* | `erosion::talus_relax` after recipes |
| Farprobe void gate | < 5% magenta empty | *authored* | `farprobe.gd` / `_selftest_far_coverage` |

When you change a row, bump `MODEL_LIMITS.md` the same commit.
