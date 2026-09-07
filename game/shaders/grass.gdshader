shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_opaque;

// Ground cover. One draw call, no shadows, and everything expensive is gated
// on distance so the far field pays nothing.

global uniform float rama_day;
uniform vec3 sun_tint : source_color = vec3(1.0, 0.95, 0.86);
uniform vec3 night_tint : source_color = vec3(0.34, 0.44, 0.62);
uniform vec3 bounce_tint : source_color = vec3(0.38, 0.48, 0.36);
uniform vec3 haze_color : source_color = vec3(0.170, 0.225, 0.290);
uniform float fade_start = 17.0;
uniform float fade_end = 27.0;
uniform float sway = 0.22;

varying vec3 w_pos;
varying vec3 w_nrm;
varying vec4 v_col;
varying float v_h;

void vertex() {
    // Height up the blade, from the instance-local Y. Drives sway and the
    // base-to-tip gradient without needing a second attribute.
    v_h = clamp(VERTEX.y, 0.0, 1.0);
    vec3 local = VERTEX;
    // Wind: a travelling wave over world position so the whole field moves as
    // one gust rather than each tuft wobbling independently.
    vec3 wp = (MODEL_MATRIX * vec4(0.0, 0.0, 0.0, 1.0)).xyz;
    float ph = wp.x * 0.11 + wp.z * 0.09 + TIME * 1.15;
    float g = sin(ph) + 0.45 * sin(ph * 2.3 + 1.7);
    local.x += g * sway * v_h * v_h;
    local.z += g * sway * 0.55 * v_h * v_h;

    w_pos = (MODEL_MATRIX * vec4(local, 1.0)).xyz;
    w_nrm = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
    v_col = COLOR;
    POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(w_pos, 1.0);
}

void fragment() {
    float d = length(w_pos - CAMERA_POSITION_WORLD);
    // Dither-discard rather than alpha blend: no sorting, no overdraw cost.
    float t = smoothstep(fade_start, fade_end, d);
    if (t > 0.001) {
        vec2 p = FRAGCOORD.xy;
        float n = fract(52.9829189 * fract(dot(p, vec2(0.06711056, 0.00583715))));
        if (t > n) { discard; }
    }

    vec3 to_axis = normalize(vec3(-w_pos.x, -w_pos.y, 0.0));
    // Blades are thin: light them mostly by wrap, not by a hard N.L.
    float ndl = 0.45 + 0.55 * max(dot(normalize(w_nrm), to_axis), 0.0);

    vec3 albedo = pow(max(v_col.rgb, vec3(0.0)), vec3(1.95));
    // Darker at the base, brighter at the tip: cheap self-shadowing that is
    // most of what makes grass read as a mass rather than as blades.
    albedo *= 0.42 + 0.58 * v_h;

    vec3 sun = mix(night_tint, sun_tint, rama_day);
    float energy = 2.05 * (0.28 + 0.72 * rama_day);
    vec3 lit = albedo * (sun * ndl * energy * 0.60
                       + bounce_tint * 0.55
                       + vec3(0.085, 0.094, 0.108) * (0.40 + 0.60 * rama_day));

    float f = clamp((d - 20.0) / 260.0, 0.0, 1.0) * 0.55;
    ALBEDO = mix(lit, haze_color * (0.45 + 0.55 * rama_day), f);
}
