shader_type spatial;
render_mode unshaded, cull_back, depth_draw_opaque;

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
    float far_fade = 1.0 - smoothstep(120.0, 280.0, dist);
    if (far_fade * near_keep < 0.02) {
        discard;
    }

    vec3 N = normalize(w_nrm);
    vec3 to_axis = normalize(vec3(-w_pos.x, -w_pos.y, 0.0));
    float ndl = pow(max(dot(N, to_axis), 0.0), 0.72);
    float amb = 0.34 + 0.48 * (1.0 - ndl);
    vec3 sun = mix(night_tint, sun_tint, rama_day);
    float energy = 2.05 * (0.30 + 0.70 * rama_day);
    // Subtle mound mottling so it's not a plastic sphere.
    float mott = 0.92 + 0.08 * sin(w_pos.x * 3.1 + w_pos.z * 2.4)
               + 0.05 * sin(w_pos.y * 4.2 - w_pos.z * 1.7);
    // Same sRGB-as-linear bug the terrain had: instance colours are authored
    // as sRGB but ALBEDO is linear, which turned every spoil heap into a cream
    // blob. Convert, and pay for it with energy.
    vec3 body = pow(max(v_col.rgb, vec3(0.0)), vec3(1.95)) * mott;
    body = body * (sun * ndl * energy
                 + bounce_tint * amb * 0.60
                 + vec3(0.075, 0.082, 0.095) * (0.40 + 0.60 * rama_day));

    float f = clamp((dist - haze_start) / (haze_end - haze_start), 0.0, 1.0);
    f = pow(f, 1.1) * 0.75 * rama_haze;
    vec3 air = haze_color * (0.45 + 0.55 * rama_day);
    ALBEDO = mix(body, air, f);
    ALPHA = 1.0;
}
