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
| `soil.n/p/k` | grid f32 ST×SZ | relative | soil | delta later |
| `soil.moisture` | grid f32 | 0..1 | soil | delta later |
| `soil.organic` | grid f32 | 0..1 | soil | delta later |
| `soil.ph` | grid f32 | pH | soil | delta later |
| `weather.humidity` | grid f32 WT×WZ | 0..1 | weather | no |
| `weather.rainfall` | grid f32 ST×SZ | intensity | weather | no |
| `weather.temperature` | grid f32 WT×WZ | °C proxy | weather | no |
| `weather.condensers` | list | entities | weather | **yes** |
| `plants[]` | entities | carbon pools | plant | **yes** T0 |
| `nitrogen_stock` | scalar | mass proxy | biosphere | **yes** |
| `water_stock` | scalar | mass proxy | biosphere | **yes** |
| `day` | scalar | habitat days | biosphere | **yes** |

Owners may not reach across layers except through `Biosphere::tick` and the
presentation/`RamaTerrain` FFI surface (item 798).
