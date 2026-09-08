# Handoff — far side of the drum renders as dark voids

**Status:** FIXED 2026-09-07 — root cause was triangle winding.
**Date:** 2026-09-07 · **Repo:** `/Users/powerox/ramen`

## The symptom

Standing on high ground and looking across or down the length of the drum,
large regions of the far wall read as flat dark navy — the user's words: *"that
dark blue in the back? that's surface not rendering."* Mountain ridges render as
thin tan ribbons; the land between them is void.

Worst-case vantage (use this to reproduce): `theta 3.063, z +2724, pitch 0.30,
yaw PI`, i.e. high ground near an endcap looking back down the long axis.

## Root cause (do not re-investigate)

Godot's ArrayMesh front faces are **clockwise**. Both hand-built grid meshers
(`far_mesh_sector` and `mid_mesh` in `sim/src/lib.rs`) emitted
`[a, c, b, a, d2, c]` — counter-clockwise when viewed from inside the drum.
`terrain.gdshader` uses `cull_back`, so every triangle facing the camera was
discarded. The ribbons were lee slopes (geometrically facing away) leaking
through; everything else was clear colour.

Measured at the vantage with a Camera3D-owned magenta clear colour and fog off:

| condition | empty screen |
|---|---|
| as shipped | **45.6%** |
| `FAR_NZ` doubled | 45.3% (no-op) |
| `FAR_OFFSET = 0` | 45.7% (no-op) |
| near_fade discard off | 45.5% (no-op) |
| un-sectored far mesh | 45.6% (not sector cull) |
| plain StandardMaterial3D | 44.4% (not the shader) |
| same shader, `cull_disabled` | 3.3% |
| same shader, `cull_front` | 3.3% |
| **far + mid indices rewound, real material** | **0.01%** |

Fix: `m.indices.extend_from_slice(&[a, b, c, a, c, d2]);` in both meshers.
`endcap_mesh` flipped the same way for consistency (its shader was
`cull_disabled`, so it already drew). Rebuild with `./build.sh`.

Near chunks were always fine — they come from the surface-nets mesher, which
winds correctly.

## What was fixed earlier (still stands)

Three distance cut-offs were tuned for a smaller habitat and each deleted part
of the world. Those remain corrected:

| file | was | now |
|---|---|---|
| `game/scripts/player.gd:20` `cam_far` | 5200 m hard clip | `drum_diagonal() * 1.15` |
| `game/scripts/world.gd:532` `env.fog_depth_end` | 1900 m | `drum_diagonal() * 1.15` |
| `game/shaders/terrain.gdshader` | `haze_end 1800`, `haze_max 0.98` | `haze_scale` + `haze_max 0.72` |

**Rule: every distance cut-off derives from `drum_diagonal()`.** Logged in
`CALIBRATION.md`.

After the winding fix the far wall has real coverage again — fog/haze/`haze_max`
were tuned against a mostly-empty wall and may need a pass. Prefer tuning cloud
density and the water sheet's `sheet_far` cut as the "stuff in the way" controls
rather than crushing haze.

## Instrumentation traps

1. `cull_disabled` did **not** inflate coverage 11x — it *revealed* the real
   coverage that back-face cull was deleting. That number was the answer.
2. Godot applies depth fog to unshaded surfaces too — disable fog before any
   flat-colour probe.
3. `RenderingServer.global_shader_parameter_get` is editor-only.
4. Isolating by visibility has a ~6% noise floor (glow/tonemap).
5. `func _set(...)` in a GDScript Node collides with Object's virtual `_set`.
6. `class_name` globals do not resolve inside `res://scripts/debug/*` — use
   `preload`.
7. zsh aborts a whole command line on an unmatched glob.
8. **Day cycle rewrites `env.background_color` / fog every frame**
   (`world.gd` ~2863). Setting clear colour on `world_env.environment` is
   clobbered before the frame renders. Put it on `Camera3D.environment`.
9. **`farprobe.gd` used `yaw = 0`**, which faces the *near* endcap ~274 m away.
   The endcap filled every tap. Long-axis view is `yaw = PI` (as `13_across`).
10. **Godot front faces are clockwise.** Check hand-built grids with a magenta
    clear colour, not with normals (normals were right the whole time; culling
    never looks at them).

Coverage regression lives in `_selftest`: magenta Camera3D env at the vantage,
fail if >5% of pixels are clear colour.

## Build and run

```bash
./build.sh                 # NEVER cargo+cp: overwriting a loaded dylib
                           # invalidates its ad-hoc signature and macOS SIGKILLs
                           # Godot with zero output (exit 137)
```

```bash
/Applications/Godot.app/Contents/MacOS/Godot --headless --path game --log-file /tmp/s.log -- --selftest
```
