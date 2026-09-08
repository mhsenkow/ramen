shader_type spatial;
render_mode cull_disabled, blend_mix, depth_draw_never, unshaded;

#include "rama_light.gdshaderinc"

// Current ribbons share the lake's mineral blue-green body and quiet reflections.
uniform vec3 shallow : source_color = vec3(0.29, 0.48, 0.44);
uniform vec3 deep : source_color = vec3(0.10, 0.28, 0.30);
uniform vec3 sparkle : source_color = vec3(0.83, 0.87, 0.82);
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
    // feature: 6–15 m current folds retire by 220 m; whole ribbons by 900 m.
    float detail = 1.0 - smoothstep(50.0, 220.0, dist);
    float visible = 1.0 - smoothstep(300.0, 900.0, dist);
    if (visible < 0.02) { discard; }
    vec3 up = rama_to_axis(w_pos);
    vec3 tang = vec3(-up.y, up.x, 0.0);
    float u = atan(w_pos.y, w_pos.x) * length(w_pos.xy);
    vec2 p = vec2(u, w_pos.z);
    float p1 = dot(p, vec2(0.42, 0.28)) - TIME * 1.10;
    float p2 = dot(p, vec2(-0.26, 0.77)) - TIME * 1.68;
    vec2 slope = (vec2(0.42, 0.28) * cos(p1) * 0.11
                + vec2(-0.26, 0.77) * cos(p2) * 0.035) * detail;
    vec3 N = normalize(up - tang * slope.x - vec3(0.0, 0.0, slope.y));
    vec3 view = normalize(CAMERA_POSITION_WORLD - w_pos);
    float ndv = abs(dot(N, view));
    float fresnel = 0.025 + 0.975 * pow(1.0 - ndv, 5.0);
    vec3 body = srgb_to_linear(mix(deep, shallow,
                       0.50 + sin(p1) * sin(p2) * detail * 0.04));
    vec3 sun = rama_sun(night_tint, sun_tint, rama_day);
    float energy = rama_energy(1.20, rama_day);
    body *= sun * max(dot(N, up), 0.0) * energy
          + rama_bounce(bounce_tint) * 0.23 + rama_sky_fill(rama_day);
    vec3 reflection = srgb_to_linear(vec3(0.46, 0.56, 0.55))
                    * (0.36 + 0.64 * rama_day);
    body = mix(body, reflection, fresnel * 0.70);
    vec3 halfv = normalize(up + view);
    float highlight = pow(max(dot(N, halfv), 0.0), 92.0);
    body += srgb_to_linear(sparkle) * highlight * detail * 0.20 * rama_day;
    float f = rama_haze_factor(dist, haze_start, haze_end, 0.80, rama_haze);
    ALBEDO = mix(body, rama_air(haze_color, rama_day), f);
    ALPHA = (0.60 + fresnel * 0.18) * visible * (1.0 - f * 0.40);
}
