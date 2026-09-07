shader_type spatial;
render_mode cull_disabled, unshaded, depth_draw_never, blend_mix;

// Clouds in a drum form AXIAL STRIPES, not Earth-like patches.
// Rossby number ~0.025: air cannot cross the drum; flow organises into
// bands along the axis. LANDSCAPE_200.md items 88–92.

global uniform float rama_day;
uniform vec3 tint : source_color = vec3(0.78, 0.84, 0.90);
uniform float cover = 0.60;

varying vec3 w_pos;
varying vec3 w_norm;

float h21(vec2 p) { return fract(sin(dot(p, vec2(41.3, 289.1))) * 43758.5453); }
float vnoise(vec2 p) {
    vec2 i = floor(p); vec2 f = fract(p); f = f * f * (3.0 - 2.0 * f);
    return mix(mix(h21(i), h21(i + vec2(1,0)), f.x),
               mix(h21(i + vec2(0,1)), h21(i + vec2(1,1)), f.x), f.y);
}

void vertex() {
    w_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
    w_norm = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
    float th = atan(w_pos.y, w_pos.x);
    float band = th * 5.5 + TIME * 0.012;
    float stripe = vnoise(vec2(band, 0.15)) * 0.70
                 + vnoise(vec2(band * 2.2 + 3.1, 0.4)) * 0.30;
    // 3rd octave for wispy edge texture — what makes clouds not look like
    // a cylinder with alpha.
    float wisp = vnoise(vec2(band * 4.4 + 7.2, w_pos.z * 0.003 + TIME * 0.008)) * 0.15;
    float axial = vnoise(vec2(th * 1.2, w_pos.z * 0.0018 + TIME * 0.004)) * 0.18;
    float n = stripe + axial + wisp;
    float a = smoothstep(cover, cover + 0.22, n);
    float d = length(w_pos - CAMERA_POSITION_WORLD);
    a *= clamp(d / 400.0, 0.0, 1.0);
    a *= clamp(1.0 - d / 3200.0, 0.0, 1.0);

    // Self-shadowing: top-lit faces are brighter, bottom faces darker.
    // Clouds that are the same brightness top and bottom look like decals.
    vec3 to_axis = normalize(vec3(-w_pos.x, -w_pos.y, 0.0));
    float self_shadow = 0.72 + 0.28 * max(dot(w_norm, to_axis), 0.0);

    // Dawn/dusk colour: clouds pick up the axis strip's warm colour.
    // During the day they're cool grey; at golden hour they're salmon/amber.
    vec3 warm_tint = mix(vec3(1.0, 0.65, 0.42), vec3(1.0, 0.92, 0.85), rama_day);
    vec3 cloud_col = mix(tint * warm_tint, tint, rama_day * 0.75 + 0.25);

    // Dim with the day schedule — bright white decks were bleaching the sky.
    ALBEDO = cloud_col * (0.45 + 0.40 * rama_day) * self_shadow;
    ALPHA = a * 0.28;
}

