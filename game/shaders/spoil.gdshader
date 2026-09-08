shader_type spatial;
render_mode unshaded, cull_back, depth_draw_opaque;

#include "rama_light.gdshaderinc"

// Spoil mounds — same axis-strip light as terrain so dig piles belong.

global uniform float rama_day;
global uniform float rama_haze;
uniform vec3 sun_tint : source_color = vec3(1.0, 0.95, 0.86);
uniform vec3 night_tint : source_color = vec3(0.34, 0.44, 0.62);
uniform vec3 bounce_tint : source_color = vec3(0.38, 0.48, 0.36);
uniform vec3 haze_color : source_color = vec3(0.170, 0.225, 0.290);
uniform float haze_start = 80.0;
uniform float haze_end = 900.0;

varying vec3 w_pos;
varying vec3 w_nrm;
varying vec4 v_col;

void vertex() {
    w_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
    w_nrm = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
    v_col = COLOR;
}

void fragment() {
    float dist = length(w_pos - CAMERA_POSITION_WORLD);
    // Soft near/far so heaps don't read as hard debug blobs.
    float near_keep = smoothstep(1.2, 4.0, dist);
    // feature: spoil heap ~3–8 m → retire 120–280 m
    float far_fade = 1.0 - smoothstep(120.0, 280.0, dist);
    if (far_fade * near_keep < 0.02) {
        discard;
    }

    vec3 N = normalize(w_nrm);
    vec3 to_axis = rama_to_axis(w_pos);
    float ndl = pow(max(dot(N, to_axis), 0.0), 0.72);
    float amb = 0.34 + 0.48 * (1.0 - ndl);
    vec3 sun = rama_sun(night_tint, sun_tint, rama_day);
    float energy = rama_energy(2.05, rama_day);
    // Subtle mound mottling so it's not a plastic sphere.
    float mott = 0.92 + 0.08 * sin(w_pos.x * 3.1 + w_pos.z * 2.4)
               + 0.05 * sin(w_pos.y * 4.2 - w_pos.z * 1.7);
    vec3 body = srgb_to_linear(v_col.rgb) * mott;
    body = body * (sun * ndl * energy
                 + bounce_tint * amb * 0.60
                 + rama_sky_fill(rama_day) * 0.9);

    float f = rama_haze_factor(dist, haze_start, haze_end, 0.75, rama_haze);
    ALBEDO = mix(body, rama_air(haze_color, rama_day), f);
    ALPHA = 1.0;
}
