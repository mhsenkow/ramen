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
| Pack mass / volume | 60 kg / 55 L (player) | *authored* | encumbrance |
| Greenhouse glass | 12 kg | *authored* | place_greenhouse |

When you change a row, bump `MODEL_LIMITS.md` the same commit.
