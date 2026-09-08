# Field schema — simulation fields

LANDSCAPE_800 items 771–772. Every sim field that persists or crosses the FFI
boundary is listed here. Adding a field without registering it fails review.

| Field | Kind | Unit | Owner | Saved |
|---|---|---|---|---|
| `elev` | grid f32 NT×NZ | m above hull | terrain | no (derived+strokes) |
| `elev0` | grid f32 | m | terrain | no |
| `flow.flux` | grid f32 | 0..1 | flow | no (rebuild) |
| `flow.down` | grid u32 | index | flow | no |
| `flow.lake` | grid u8 | flag | flow | no |
| `flow.discharge` | grid f32 | cells | flow | no |
| `edits.strokes` | list | CSG | edits | **yes** |
| `dwelling.kind` | enum/site | delve/terrace/township | dwelling | derived from traits |
| `dwelling.reach` | enum/site | legibility rung | dwelling | derived (contested recomputed) |
| `dwelling.capacity` | f32/site | people | dwelling | no (resurveyed weekly) |
| `dwelling.followers` | f32/site | people | dwelling | *must* — not wired yet |
| `dwelling.works` | u32/site | structures | dwelling | *must* — not wired yet |
| `dwelling.stores_kg` | f32/site | kg dry food | dwelling | *must* — not wired yet |
| `dwelling.build_days` | f32/site | labour-days | dwelling | *must* — not wired yet |
| `dwelling.quality` | f32/site | 0.15..1.4 | dwelling | no (from soil) |
| `soil.n/p/k` | grid f32 ST×SZ | relative | soil | delta later |
| `soil.moisture` | grid f32 | 0..1 | soil | delta later |
| `soil.organic` | grid f32 | 0..1 | soil | delta later |
| `soil.ph` | grid f32 | pH | soil | delta later |
| `weather.humidity` | grid f32 WT×WZ | 0..1 | weather | no |
| `weather.rainfall` | grid f32 ST×SZ | intensity | weather | no |
| `weather.temperature` | grid f32 WT×WZ | °C proxy | weather | no |
| `weather.condensers` | list | entities (recovery / precip) | weather | **yes** |
| `weather.aridity` | derived query | 0..1 (province climate + condensers) | weather←province | **no** — never painted |
| `weather.rain_scale` | derived WT×WZ | rain multiplier | weather (rebuild ~2 d) | no |
| `weather.spine_vapor_rate` | scalar | humidity / habitat-day | weather | no |
| `weather.carriage_z` | scalar | m along axis | weather ← Godot | no |
| `province` | derived Worley NT_PROV×NZ_PROV | archetype weights (top-2 blend) | province | **no** — query-time only; never elev/soil |
| `Habitat.max_elevation` | scalar | m (authored **440**) | habitat | no (geometry constant) |
| `biome` (14 ids incl. swamp/meadow/desert/dune/shore) | derived | u8 id | biome | no |
| `material.SAND` | derived | u8=8 | material | no |
| `forest_kind` | derived in plants_lod | 0..5 species | lib←biome+climate | no |
| `plants[]` | entities | carbon pools | plant | **yes** T0 |
| `nitrogen_stock` | scalar | mass proxy | biosphere | **yes** |
| `water_stock` | scalar | mass proxy | biosphere | **yes** |
| `day` | scalar | habitat days | biosphere | **yes** |
| `grass_field` stride | LOD float×6 | θ,z,r,sc,lush,biome | presentation | no |
| `rama_bounce_tint` | global vec3 | linear albedo | world.gd ← biome_map | no (derived) |
| `RamaSun` | DirectionalLight3D | local axis→hull approx; shadows ≤240 m; prox to carriage | world.gd | no |
| `SpineCore` / `DayCarriage` / `EndcapRing*` | presentation meshes | Photothermal Spine | world.gd | no |

**Photothermal Spine:** dim always-on core evaporates vapor into humidity;
traveling day-carriage is the dawn wave; endcap rings are safety/horizon
lights; condensers precipitate. Spectacle clock pushes `set_day_schedule`.
Sky events (`fog` / `storm`) ride humidity + phase; `F6` / `--fast-day`
speeds the picture clock (~18×) and raises `spectacle_pace` so weather is
easy to audition.

**Province contract:** archetypes are Massif / Plateau / Badlands / Meadow /
SwampBasin / DuneSea / SeaBasin / Karst / EndcapWall / Farmland / City.
`BLEND_M = 280 m`. Farmland is an axial engineered ring; City is a helical
corridor that cuts across it — query-time only, never painted into elev/soil.
Raising `max_elevation` without updating `chunk_mesh_at` radial margins in the
same change is a regression (LANDSCAPE_4200 item 4184).

**Not persisted:** province maps, aridity grids, biome ids, rain_scale, spine
carriage — all recomputed or spectacle-driven. Do not add painted province
textures as source of truth.

Owners may not reach across layers except through `Biosphere::tick` and the
presentation/`RamaTerrain` FFI surface (item 798).

**Not yet persisted:** `persist.rs` covers strokes and soil only. Every
`dwelling` row marked *must* is population state that a reload currently resets
to day zero — a settlement you watched grow comes back empty. This is the next
thing the dwelling layer needs.
