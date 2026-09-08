shader_type spatial;
render_mode cull_disabled, unshaded, blend_mix, depth_draw_always;

#include "rama_light.gdshaderinc"

// Habitat waterline cylinder — masked seas (lake_mask). Near + far; local
// lake/pool quads still own smaller basins and shore foam detail.

uniform vec3 shallow : source_color = vec3(0.34, 0.62, 0.88);
uniform vec3 deep : source_color = vec3(0.10, 0.30, 0.48);
uniform vec3 haze_color : source_color = vec3(0.170, 0.225, 0.290);
uniform float haze_start = 320.0;
uniform float haze_end = 3400.0;
// Soft underfoot fade only — the lake_mask already clips dry land. The old
// 160–320 m hard hide left seas as bare bed under the player (near field was
// supposed to be pools, but waterline basins never got pool quads).
uniform float near_hide = 2.5;
uniform float near_soft = 14.0;

global uniform float rama_day;
global uniform float rama_haze;
// Coverage mask: red channel is standing-water presence, theta across and z
// down. Without this the sheet paints every square metre below the waterline,
// which from across the drum reads as half the world being ocean.
uniform sampler2D lake_mask : filter_linear_mipmap, repeat_enable;
uniform float hab_len = 6000.0;

varying vec3 w_pos;

void vertex() { w_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz; }

void fragment() {
    float th = atan(w_pos.y, w_pos.x);
    vec2 muv = vec2(fract(th / 6.28318530718 + 1.0), clamp(w_pos.z / hab_len + 0.5, 0.0, 1.0));
    float cover = texture(lake_mask, muv).r;
    if (cover < 0.06) { discard; }

    float dist = length(w_pos - CAMERA_POSITION_WORLD);
    // Soften only the camera-intersection band so the sheet doesn't clip
    // through the near plane as a hard disc.
    float near_fade = smoothstep(near_hide, near_soft, dist);
    if (near_fade < 0.02) {
        discard;
    }

    // Bathymetry proxy: neighbour cover ≈ basin extent; edge cover ≈ shelf.
    float cover_n = texture(lake_mask, muv + vec2(0.012, 0.0)).r;
    float cover_z = texture(lake_mask, muv + vec2(0.0, 0.009)).r;
    float depth_proxy = clamp(cover * 0.55 + cover_n * 0.25 + cover_z * 0.20, 0.0, 1.0);
    float shelf = 1.0 - smoothstep(0.25, 0.85, depth_proxy);
    float edge = smoothstep(0.08, 0.28, cover) * (1.0 - smoothstep(0.45, 0.95, cover));

    // Near seas need body chroma; far keeps the old soft backdrop look.
    float near_body = 1.0 - smoothstep(40.0, 180.0, dist);
    float rfade = clamp(1.0 - dist / 1400.0, 0.0, 1.0); // metre-ok: backdrop scale
    float rip = sin(w_pos.z * 0.10 + TIME * 0.22)
              + sin(w_pos.x * 0.08 - TIME * 0.16);
    // Near: add a second ripple so underfoot water isn't a flat paint swipe.
    rip += near_body * (sin(w_pos.z * 0.55 + TIME * 1.1) * 0.35
         + sin(th * 40.0 + w_pos.z * 0.2 - TIME * 0.7) * 0.25);
    vec3 c = mix(deep, shallow, clamp(0.35 + shelf * 0.40 + rip * 0.08 * (0.35 + 0.65 * rfade), 0.0, 1.0));
    // Deep basins go inkier; shelves stay turquoise.
    c = mix(c, deep * 0.72, (1.0 - shelf) * 0.45);
    // Near water reads as a body, not a washed distant sheet.
    c = mix(c, mix(shallow, deep, 0.35 + shelf * 0.25), near_body * 0.45);
    c = srgb_to_linear(c);

    vec3 to_axis = rama_to_axis(w_pos);
    float ndl = max(dot(to_axis, normalize(CAMERA_POSITION_WORLD - w_pos)), 0.0);
    c *= 0.78 + 0.42 * ndl;
    c *= 0.45 + 0.55 * rama_day;

    // Distant shimmer + flow-aligned glints that survive at range (Wave 2/6).
    float shimmer = pow(ndl, 20.0) * 0.14 * rama_day * (1.0 - rfade * 0.4);
    // River strokes: wind-phase sparkle along z so far water reads as living lines.
    float flow_glint = pow(max(sin(w_pos.z * 0.035 + TIME * 0.55) * 0.5
                     + sin(th * 3.0 - TIME * 0.2) * 0.5, 0.0), 5.0);
    float strip_sparkle = pow(ndl, 14.0) * flow_glint * 0.32 * rama_day * cover;
    // Soft fresnel lift so the sheet isn't matte paint.
    float fres = pow(1.0 - clamp(ndl, 0.0, 1.0), 2.0);
    c += vec3(1.0, 0.95, 0.82) * (shimmer + strip_sparkle);
    c = mix(c, vec3(0.75, 0.88, 1.0) * (0.35 + 0.65 * rama_day), fres * 0.22 * cover);
    // Shore foam ribbon on the waterline edge (Wave 7).
    vec3 foam = vec3(0.92, 0.95, 0.98);
    c = mix(c, foam * (0.55 + 0.45 * rama_day), edge * 0.55 * rfade);

    // Basin-scale tone variation. A single flat blue across the far side is
    // the giveaway; real water bodies differ by depth, fetch and turbidity.
    float basin = texture(lake_mask, muv + vec2(0.021, 0.013)).r;
    float wide = 0.5 + 0.5 * sin(muv.x * 41.0) * sin(muv.y * 33.0);
    c *= 0.88 + 0.22 * basin + 0.10 * wide;
    c = mix(c, vec3(0.12, 0.24, 0.30), (1.0 - basin) * 0.28);
    c = max(c, vec3(0.05, 0.11, 0.16));

    float f = rama_haze_factor(dist, haze_start, haze_end, 0.80, rama_haze);
    // Water holds chroma far longer than land, which is what made the far side
    // read as a flat saturated sheet. Bleed it toward the air colour with range.
    float lum = dot(c, vec3(0.299, 0.587, 0.114));
    c = mix(c, vec3(lum) * vec3(0.86, 0.94, 1.04), f * 0.55);

    vec3 air = rama_air(haze_color, rama_day);
    ALBEDO = mix(c, air, f);
    // The mask is ~5.5 m per texel; across the drum those texels subtend tens
    // of pixels and paint the far wall in blocks. Retire the sheet before that
    // — the far-field terrain already carries distant water colour.
    float sheet_far = 1.0 - smoothstep(700.0, 1150.0, dist);
    // Near seas need opaque-enough body; far sheet stays soft backdrop.
    float a_near = mix(0.55, 0.78, fres * 0.35 + (1.0 - shelf) * 0.25);
    float a_far = mix(0.38, 0.68, rfade);
    ALPHA = mix(a_far, a_near, near_body) * (1.0 - f * 0.50) * near_fade * sheet_far;
    if (ALPHA < 0.008) { discard; }
}
