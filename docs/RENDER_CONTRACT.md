# Render contract

**Authority:** `LANDSCAPE_2200.md` §AR (2001–2050).  
**Includes:** `game/shaders/rama_color.gdshaderinc`, `rama_light.gdshaderinc`.

## Colour space (2001–2003)

| Authored as | Consumed as | Conversion |
|---|---|---|
| Palette / vertex colour (sRGB) | `ALBEDO` (linear) | `srgb_to_linear(c)` → `pow(c, 1.95)` |
| Prop instance colour (display 0.5–0.7) | albedo via `albedo_scale` then sRGB→linear | see `prop.gdshader` |

No shader may feed an authored palette into `ALBEDO` without that conversion.

## Light (2014–2015)

One contract for anything that sits in the habitat:

1. **Key** — warm strip along the axis (`rama_to_axis` · `rama_sun`)
2. **Fill** — cool bounce + sky term (`rama_sky_fill`)
3. **Haze** — additive inscatter (`rama_air` / `rama_air_toward`), never a near-white tint
4. **Energy** — scales with `rama_day` via `rama_energy`

Opt-outs need an inline comment naming the reason (e.g. endcap structural lighting).

## Detail falloff (2004–2007)

Every noise / panel / mask / billboard term states its feature size in metres and retires before it goes sub-pixel. Metre values that depend on habitat size scale by `hab_radius` / `hab_len`, not a baked 900.

## MultiMesh colours (2009–2011)

**An instance colour MULTIPLIES the source mesh's vertex colour, and the shader
then raises the product to 1.95.** A source mesh that bakes its own final colour
therefore lands at roughly a hundredth of the value it was authored at. Litter
authored at 0.3 under an instance at 0.3 rendered at 0.008 linear — the black
pebbles that used to be scattered over every grassland; a brown trunk under a
green leaf rendered at zero.

So for any mesh used as a MultiMesh source with `use_colors`:

- **Vertex RGB is a luminance ratio around 1.0.** The instance colour carries hue.
- **A hue the instance colour can never produce needs its own channel.**
  `tree.gdshader` reads vertex ALPHA as a material key — 0 wood, 0.5 fruit,
  1 leaf — and names those two colours as uniforms. See `world._leaf` /
  `_wood` / `_fruit`.


If `instance_count > 0` at build with `use_colors`, set transforms and colours in the same function (transparent / zero-scale is fine). Instanced props use `prop.gdshader`, never a bare unshaded `StandardMaterial3D` with default white albedo.
