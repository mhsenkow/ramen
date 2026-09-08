shader_type spatial;
render_mode cull_disabled, blend_mix, depth_draw_always, unshaded;

#include "rama_light.gdshaderinc"

// Major flowing channels — same blue language + axis lighting as pools.

uniform vec3 shallow : source_color = vec3(0.38, 0.68, 0.92);
uniform vec3 deep : source_color = vec3(0.10, 0.34, 0.55);
uniform vec3 sparkle : source_color = vec3(0.85, 0.94, 1.0);
uniform vec3 sun_tint : source_color = vec3(1.0, 0.95, 0.86);
uniform vec3 night_tint : source_color = vec3(0.34, 0.44, 0.62);
uniform vec3 bounce_tint : source_color = vec3(0.38, 0.48, 0.36);
uniform vec3 haze_color : source_color = vec3(0.170, 0.225, 0.290);
uniform float haze_start = 70.0;
uniform float haze_end = 900.0;

global uniform float rama_day;
global uniform float rama_haze;
varying vec3 w_pos;

void vertex() { w_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz; }

void fragment() {
    float dist = length(w_pos - CAMERA_POSITION_WORLD);
    // Channels read close; a thin glint survives farther so rivers mark the far wall.
    // feature: channel body ~220 m; glint ribbon ~retire past 900 m
    float rfade = smoothstep(220.0, 40.0, dist);
    float glint_fade = smoothstep(900.0, 120.0, dist);
    if (glint_fade < 0.02) {
        discard;
    }
    // Flow-aligned ripple: downstream frequency compresses for a visible
    // current read. Cross-flow stays slower for surface tension.
    float flow_along = w_pos.z * 0.72 + TIME * 1.45;  // primary downstream
    float flow_cross = w_pos.x * 0.38 + w_pos.y * 0.32;
    float rip = sin(flow_along)
              + sin(flow_along * 1.7 - TIME * 0.4) * 0.65
              + sin(flow_cross - TIME * 0.55) * 0.45;
    vec3 c = mix(deep, shallow, clamp(0.55 + rip * 0.18 * rfade, 0.0, 1.0));
    c = srgb_to_linear(c);
    c = max(c, vec3(0.08, 0.18, 0.26));

    vec3 to_axis = rama_to_axis(w_pos);
    vec3 view = normalize(CAMERA_POSITION_WORLD - w_pos);
    float ndl = max(dot(to_axis, view), 0.0);
    ndl = pow(ndl, 0.70);
    float amb = 0.36 + 0.48 * (1.0 - ndl);
    vec3 sun = rama_sun(night_tint, sun_tint, rama_day);
    float energy = rama_energy(1.45, rama_day);
    c = c * (sun * ndl * energy
           + bounce_tint * amb * 0.40
           + rama_sky_fill(rama_day) * 2.1);
    // Sharper specular glint — channels catch the axis strip as bright lines.
    vec3 halfv = normalize(to_axis + view);
    float spec = pow(max(dot(normalize(vec3(-w_pos.x, -w_pos.y, 0.0) + vec3(rip * 0.10 * rfade)), halfv), 0.0), 42.0);
    c += sparkle * (pow(ndl, 16.0) * 0.30 + spec * 0.55) * max(rfade, glint_fade * 0.65) * rama_day;

    float f = rama_haze_factor(dist, haze_start, haze_end, 0.80, rama_haze);
    vec3 air = rama_air(haze_color, rama_day);
    ALBEDO = mix(c, air, f);
    // Far: keep a bright thin stroke; near: fuller water body.
    float body_a = mix(0.48, 0.78, rfade) * rfade;
    float glint_a = 0.62 * glint_fade * (1.0 - rfade);
    ALPHA = min(0.85, (body_a + glint_a) * (1.0 - f * 0.35));
}
