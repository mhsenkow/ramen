shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_opaque;

#include "rama_light.gdshaderinc"

// The drum's end wall. Structural, not landscape — but it MUST share the same
// aerial perspective as the terrain. When terrain haze went white and the
// endcap stayed dark, you got a hard black disc cut out of the sky.
// That was the "weird cutoff."
// Opt-out of full key/fill contract: engineered hull, not ground (§2015).

global uniform float rama_day;
global uniform float rama_haze;
uniform vec3 base : source_color = vec3(0.34, 0.36, 0.40);
uniform vec3 haze_color : source_color = vec3(0.28, 0.36, 0.42);
uniform float haze_start = 140.0;
uniform float haze_end = 2600.0;
uniform float haze_max = 0.92;
uniform float hab_radius = 900.0;

varying vec3 w_pos;
varying vec3 w_norm;

float hash12(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

void vertex() {
    w_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
    w_norm = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
    float r = length(w_pos.xy);
    float cam_d = length(w_pos - CAMERA_POSITION_WORLD);
    // Panels are ~36 m. Across the drum that is ~20 px, so the per-panel
    // hash aliases into a checkerboard. Fade the panelling to a smooth hull
    // with range, like the terrain relief and ring band already do.
    float panel_detail = 1.0 - smoothstep(220.0, 850.0, cam_d);
    // Concentric structure + faint radial ribs so it reads as engineered hull.
    float band = 1.0 + 0.12 * sin(r * 0.055) * panel_detail;
    float ribs = 1.0 + 0.06 * sin(atan(w_pos.y, w_pos.x) * 36.0) * panel_detail;

    // Panel lines — procedural grid hash with slight view-dependent parallax.
    // A 900 m wall needs to read as engineered panels at distance.
    vec3 view = normalize(CAMERA_POSITION_WORLD - w_pos);
    vec2 panel_uv = w_pos.xy * 0.028 + view.xy * 0.35;  // parallax shift
    vec2 panel_cell = floor(panel_uv);
    vec2 panel_fract = fract(panel_uv);
    // Grid lines at cell edges.
    float edge = smoothstep(0.0, 0.06, min(panel_fract.x, panel_fract.y))
               * smoothstep(0.0, 0.06, min(1.0 - panel_fract.x, 1.0 - panel_fract.y));
    float panels = 0.92 + 0.08 * edge;
    // Per-panel value variation.
    float panel_hash = hash12(panel_cell);
    panels *= 0.95 + 0.10 * panel_hash;

    // Axis light falls on the endcap from the centre. Without this, grazing
    // views of the rim crush to black (the known MVP gap).
    vec3 to_axis = rama_to_axis(w_pos + vec3(0.0, 0.0, 0.001));
    // Endcap faces along ±Z; "facing the camera" and "lit from axis" both matter.
    float facing = abs(w_norm.z);
    float axis_lit = mix(0.45, 1.0, clamp(1.0 - r / hab_radius, 0.0, 1.0));
    float day = 0.30 + 0.70 * rama_day;
    vec3 c = srgb_to_linear(base) * band * ribs * panels * day * (0.55 + 0.45 * facing) * axis_lit;

    // Scattered port lights — tiny emissive windows on the end wall.
    // Increase at night; keep a few lit by day so habitation never vanishes.
    float port = hash12(panel_cell * 7.3 + 42.0);
    float port_chance = mix(0.72, 0.84, rama_day);
    float port_on = step(port_chance, port) * (0.35 + (1.0 - rama_day) * 0.65) * panel_detail;
    // Only show ports far enough from the hull centre to be "habitable".
    port_on *= smoothstep(hab_radius * 0.12, hab_radius * 0.42, r);
    c += vec3(1.0, 0.82, 0.52) * port_on * (0.28 + 0.22 * (1.0 - rama_day));
    // Sparse warmer "hab dome" clusters — denser along mid-radius rings.
    float cluster = hash12(panel_cell * 2.1 + 9.0);
    float cluster_on = step(0.91, cluster) * panel_detail
                     * smoothstep(hab_radius * 0.25, hab_radius * 0.55, r)
                     * (0.4 + 0.6 * (1.0 - rama_day));
    c += vec3(1.0, 0.70, 0.40) * cluster_on * 0.45;

    // Floor so the rim never punches a black hole through the haze.
    c = max(c, vec3(0.08, 0.10, 0.12) * day);

    float d = length(w_pos - CAMERA_POSITION_WORLD);
    float f = rama_haze_factor(d, haze_start, haze_end, haze_max, rama_haze);
    // Endcap air is darker than terrain inscatter — structural opt-out (§2015).
    vec3 air = haze_color * (0.22 + 0.55 * rama_day);
    // Outer rim dissolves aggressively into air — kills the hard black disc.
    float rim = smoothstep(hab_radius * 0.40, hab_radius * 0.95, r);
    f = clamp(f + rim * 0.40 * rama_haze, 0.0, 0.95);
    ALBEDO = mix(c, air, f);
}

