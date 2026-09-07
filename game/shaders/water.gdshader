shader_type spatial;
render_mode cull_disabled, unshaded, blend_mix, depth_draw_opaque;

// Habitat waterline cylinder — distant sea only. Near field is local pools.

uniform vec3 shallow : source_color = vec3(0.18, 0.40, 0.72);
uniform vec3 deep : source_color = vec3(0.05, 0.14, 0.34);
uniform vec3 haze_color : source_color = vec3(0.170, 0.225, 0.290);
uniform float haze_start = 320.0;
uniform float haze_end = 3400.0;
uniform float near_hide = 160.0;
uniform float near_soft = 320.0;

global uniform float rama_day;
global uniform float rama_haze;
// Coverage mask: red channel is standing-water presence, theta across and z
// down. Without this the sheet paints every square metre below the waterline,
// which from across the drum reads as half the world being ocean.
uniform sampler2D lake_mask : filter_linear, repeat_enable;
uniform float hab_len = 6000.0;

varying vec3 w_pos;

void vertex() { w_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz; }

void fragment() {
    float th = atan(w_pos.y, w_pos.x);
    vec2 muv = vec2(fract(th / 6.28318530718 + 1.0), clamp(w_pos.z / hab_len + 0.5, 0.0, 1.0));
    float cover = texture(lake_mask, muv).r;
    if (cover < 0.06) { discard; }

    float dist = length(w_pos - CAMERA_POSITION_WORLD);
    // Kill the dark underfoot plane — waterline is a far backdrop only.
    float near_fade = smoothstep(near_hide, near_soft, dist);
    if (near_fade < 0.02) {
        discard;
    }

    float rfade = clamp(1.0 - dist / 1400.0, 0.0, 1.0);
    float rip = sin(w_pos.z * 0.10 + TIME * 0.22)
              + sin(w_pos.x * 0.08 - TIME * 0.16);
    vec3 c = mix(deep, shallow, clamp(0.42 + rip * 0.08 * rfade, 0.0, 1.0));

    vec3 to_axis = normalize(vec3(-w_pos.x, -w_pos.y, 0.0));
    float ndl = max(dot(to_axis, normalize(CAMERA_POSITION_WORLD - w_pos)), 0.0);
    c *= 0.72 + 0.38 * ndl;
    c *= 0.40 + 0.60 * rama_day;

    // Distant shimmer — micro-specular that catches the axis strip as a pixel-
    // frequency sparkle on the far waterline. One pow() term.
    float shimmer = pow(ndl, 24.0) * 0.08 * rama_day * (1.0 - rfade * 0.5);
    c += vec3(1.0, 0.95, 0.82) * shimmer;

    float f = clamp((dist - haze_start) / (haze_end - haze_start), 0.0, 1.0);
    f = pow(f, 1.15) * 0.80 * rama_haze;
    vec3 air = haze_color * (0.22 + 0.55 * rama_day);
    ALBEDO = mix(c, air, f);
    ALPHA = mix(0.35, 0.62, rfade) * (1.0 - f * 0.55) * near_fade;
}
