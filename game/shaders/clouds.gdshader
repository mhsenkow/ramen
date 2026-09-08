shader_type spatial;
render_mode cull_disabled, unshaded, depth_draw_never, blend_mix;

#include "rama_light.gdshaderinc"

// Condensation follows the drum's axial weather circulation, but individual
// banks have finite extent in both directions. The periodic angular domain
// closes the cylinder without a seam or the old full-length white curtains.
global uniform float rama_day;
global uniform float rama_haze;
uniform vec3 tint : source_color = vec3(0.77, 0.82, 0.83);
uniform float cover = 0.55;

varying vec3 w_pos;

float cloud_hash(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float cloud_noise(vec2 p, float period) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    vec2 a = vec2(mod(i.x, period), i.y);
    vec2 b = vec2(mod(i.x + 1.0, period), i.y);
    return mix(mix(cloud_hash(a), cloud_hash(b), f.x),
               mix(cloud_hash(a + vec2(0.0, 1.0)),
                   cloud_hash(b + vec2(0.0, 1.0)), f.x), f.y);
}

void vertex() {
    w_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
    float radius = max(length(w_pos.xy), 1.0);
    float dist = length(w_pos - CAMERA_POSITION_WORLD);
    vec3 toward_view = normalize(CAMERA_POSITION_WORLD - w_pos);
    vec3 up = rama_to_axis(w_pos);
    float facing = abs(dot(toward_view, up));
    // feature: banks ~235 m around / 300 m along a 600 m condensation shell;
    // continental breaks ~900 m, soft billows ~60 m. All scale with the shell.
    vec2 p = vec2(atan(w_pos.y, w_pos.x) * (16.0 / 6.28318530718),
                  w_pos.z / (radius * 0.50));
    p += vec2(TIME * 0.0015, TIME * 0.0030);
    float weather = cloud_noise(p * 0.25 + vec2(0.0, 7.4), 4.0);
    vec2 warp = vec2(sin(p.y * 0.82 + weather * 3.0) * 0.23,
                     sin(p.x * 0.3926990817) * 0.17);
    vec2 q = p + warp;
    float broad = cloud_noise(q, 16.0);
    float billow = cloud_noise(q * 2.0 + vec2(0.0, 17.0), 32.0);
    // feature: 60 m edge detail retires over 1500–2800 m; no fine screen noise.
    float detail = 1.0 - smoothstep(1500.0, 2800.0, dist);
    float fine = mix(0.5, cloud_noise(q * 4.0 + vec2(0.0, 31.0), 64.0), detail);
    float field = broad * 0.62 + billow * 0.27 + fine * 0.11;
    // `cover` is cloud amount from weather, so increasing it thickens the deck.
    float threshold = mix(0.64, 0.43, clamp(cover, 0.0, 1.0));
    float bank = smoothstep(threshold, threshold + 0.19, field);
    bank *= smoothstep(0.26, 0.61, weather);
    float veil = smoothstep(threshold - 0.045, threshold + 0.06, field)
               * (1.0 - bank) * 0.07;
    float density = bank * 0.47 + veil;
    // feature: extinction 140–3400 m; shell grazes dissolve before becoming ribbons.
    density *= smoothstep(140.0, 360.0, dist)
             * (1.0 - smoothstep(1900.0, 3400.0, dist));
    density *= smoothstep(0.07, 0.38, facing);
    if (density < 0.006) { discard; }

    // Rounded dense interiors retain a cool underside; thin edges catch the
    // warm axis. Optical scattering is approximated on one shell (no raymarch).
    float core = smoothstep(threshold + 0.07, threshold + 0.26, field);
    vec3 sun = rama_sun(vec3(0.47, 0.53, 0.65), vec3(1.0, 0.94, 0.82), rama_day);
    vec3 cloud = srgb_to_linear(tint) * mix(vec3(1.03, 1.03, 1.0),
                         vec3(0.65, 0.74, 0.79), core * 0.65);
    cloud *= sun * rama_energy(0.82, rama_day) + rama_sky_fill(rama_day) * 1.6;
    cloud += srgb_to_linear(vec3(0.92, 0.82, 0.64)) * (1.0 - core)
           * bank * 0.055 * rama_day;
    float air_mix = rama_haze_factor(dist, 650.0, 3400.0, 0.42, rama_haze); // feature: cloud air 650–3400 m
    ALBEDO = mix(cloud, rama_air(vec3(0.17, 0.225, 0.29), rama_day), air_mix);
    ALPHA = density;
}
