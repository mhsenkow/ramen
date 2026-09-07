shader_type spatial;
render_mode unshaded, cull_back, depth_draw_opaque;

// Light in an O'Neill cylinder comes from a strip running along the axis.
// There is no sun overhead — "up" IS the light. Bounce off the far side is
// farmland-green. Distance haze must stay COLOURED and darker than midday
// terrain, or the far field dissolves into white and the endcaps look like
// holes punched in the sky.

global uniform float rama_day;
global uniform float rama_haze;
uniform vec3 sun_tint : source_color = vec3(1.0, 0.95, 0.86);
uniform vec3 night_tint : source_color = vec3(0.34, 0.44, 0.62);
uniform vec3 bounce_tint : source_color = vec3(0.38, 0.48, 0.36);
// Habitat air is a cool teal-grey, never near-white — white haze is what
// made the distance read as overexposed snow.
uniform vec3 haze_color : source_color = vec3(0.170, 0.225, 0.290);
uniform float sun_energy = 2.05;
uniform float bounce_energy = 0.62;
uniform float haze_start = 140.0;
uniform float haze_end = 2600.0;
uniform float haze_max = 0.90;
uniform float hab_radius = 900.0;
// Far-field only: dither-discard under the near chunk ring so the coarse LOD
// never fights the fine mesh at the streaming boundary.
uniform float near_fade_start = 0.0;
uniform float near_fade_end = 0.0;
// Warm key, cool fill. The single cheapest trick that makes flat-shaded
// geometry read as lit by something real rather than tinted by a constant.
uniform vec3 shadow_tint : source_color = vec3(0.42, 0.52, 0.70);
uniform float ao_strength = 0.85;
uniform float face_jitter = 0.045;
uniform float mottle = 0.30;
uniform float relief = 0.34;

// Cheap 3D hash for per-face variation.
float hash13(vec3 p3) {
    p3 = fract(p3 * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// Value noise for ground mottling. Real ground is patchy at several scales at
// once; a single flat albedo per facet is the tell that it is not.
float vnoise(vec3 p) {
    vec3 i = floor(p);
    vec3 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    float n000 = hash13(i);
    float n100 = hash13(i + vec3(1.0, 0.0, 0.0));
    float n010 = hash13(i + vec3(0.0, 1.0, 0.0));
    float n110 = hash13(i + vec3(1.0, 1.0, 0.0));
    float n001 = hash13(i + vec3(0.0, 0.0, 1.0));
    float n101 = hash13(i + vec3(1.0, 0.0, 1.0));
    float n011 = hash13(i + vec3(0.0, 1.0, 1.0));
    float n111 = hash13(i + vec3(1.0, 1.0, 1.0));
    return mix(mix(mix(n000, n100, f.x), mix(n010, n110, f.x), f.y),
               mix(mix(n001, n101, f.x), mix(n011, n111, f.x), f.y), f.z);
}

varying vec3 w_pos;
varying vec3 w_norm;
varying vec4 v_col;

void vertex() {
    w_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
    w_norm = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
    v_col = COLOR;
}

void fragment() {
    float d = length(w_pos - CAMERA_POSITION_WORLD);
    if (near_fade_end > near_fade_start) {
        float t = smoothstep(near_fade_start, near_fade_end, d);
        // Interleaved gradient noise — soft LOD handoff without alpha blend.
        vec2 p = FRAGCOORD.xy;
        float n = fract(52.9829189 * fract(dot(p, vec2(0.06711056, 0.00583715))));
        if (t < n) {
            discard;
        }
    }

    vec3 to_axis = normalize(vec3(-w_pos.x, -w_pos.y, 0.0));

    // Micro-relief: perturb the SHADING normal with cheap noise so smooth
    // surface-nets ground reads as a surface rather than as plastic. No extra
    // geometry, and gated to the near field so distant pixels pay nothing.
    vec3 sn = w_norm;
    float near_relief = 1.0 - smoothstep(35.0, 110.0, d);
    if (near_relief > 0.004) {
        float e = 0.9;
        float h0 = vnoise(w_pos * 0.42);
        float hx = vnoise((w_pos + vec3(e, 0.0, 0.0)) * 0.42);
        float hz = vnoise((w_pos + vec3(0.0, 0.0, e)) * 0.42);
        vec3 t1 = normalize(cross(w_norm, vec3(0.0, 0.0, 1.0)) + vec3(1e-5));
        vec3 t2 = cross(w_norm, t1);
        sn = normalize(w_norm - (t1 * (hx - h0) + t2 * (hz - h0))
                                * relief * near_relief * 6.0);
    }
    float ndl = max(dot(sn, to_axis), 0.0);

    float r = length(w_pos.xy);
    float prox = clamp(0.58 + 0.42 * (1.0 - r / hab_radius), 0.5, 1.05);

    // Height-band tinting: ridgelines warm (exposed, dry), valleys cool (wet,
    // shadowed). Uses radial position which is altitude in a drum.
    float elev_frac = clamp((hab_radius - r) / 120.0, 0.0, 1.0);

    float amb = 0.32 + 0.50 * (1.0 - ndl);
    ndl = pow(ndl, 0.70);

    // Baked ambient occlusion from the mesher, in the vertex alpha.
    float ao = mix(1.0, v_col.a, ao_strength);
    // Contact darkening bites hardest on the indirect terms; direct light is
    // only lightly occluded, which is how real AO behaves.
    float ao_dir = mix(1.0, ao, 0.35);

    // The palette is authored as sRGB values but ALBEDO is consumed linear.
    // Feeding sRGB numbers straight in is what bleached the whole world: 0.14
    // linear is a pale sage, not the deep green the table says. Convert, and
    // pay for the darkening with light energy instead.
    vec3 albedo = pow(max(v_col.rgb, vec3(0.0)), vec3(1.95));

    // Height-band colour shift: high ground is warmer and less saturated
    // (exposed rock/scrub), low ground is cooler and greener (wet alluvial).
    albedo.r *= 1.0 + elev_frac * 0.08;
    albedo.b *= 1.0 - elev_frac * 0.05;
    // Valleys get a touch more green saturation — they're wetter.
    albedo.g *= 1.0 + (1.0 - elev_frac) * 0.04;

    // Per-face value and hue break-up. Flat shading gives one normal per
    // triangle, so hashing the normal with a coarse position varies each facet
    // independently and kills the "one flat colour" look on open ground.
    // Multi-scale mottling — tightened to 240 m because the tilt-shift blur
    // hides everything past ~200 m; saves ALU on far fragments.
    float detail = 1.0 - smoothstep(55.0, 240.0, d);
    if (detail > 0.001) {
        float m1 = vnoise(w_pos * 0.055);
        float m2 = vnoise(w_pos * 0.21);
        float m3 = vnoise(w_pos * 0.52);
        float m = (m1 * 0.50 + m2 * 0.32 + m3 * 0.18) - 0.5;
        albedo *= 1.0 + m * mottle * detail;
        // Drier patches skew warm, damper ones cool. Two-tone, not just value.
        albedo.r *= 1.0 + m * mottle * 0.55 * detail;
        albedo.b *= 1.0 - m * mottle * 0.40 * detail;
    }

    float fh = hash13(floor(w_pos * 0.6) + w_norm * 37.0);
    albedo *= 1.0 + (fh - 0.5) * 2.0 * face_jitter;
    albedo.r *= 1.0 + (fh - 0.5) * face_jitter * 0.6;
    albedo.b *= 1.0 - (fh - 0.5) * face_jitter * 0.6;

    vec3 sun = mix(night_tint, sun_tint, rama_day);
    float energy = sun_energy * (0.28 + 0.72 * rama_day);

    // Warm key from the strip, cool fill from everything else. Splitting the
    // colour temperature between lit and unlit is what makes a surface read as
    // having a light source rather than a brightness value.
    vec3 fill = mix(shadow_tint, bounce_tint, 0.55);

    // AO contact darkening shifts cool-violet rather than neutral grey.
    // Real radiosity from green/brown ground bouncing into crevices reads cool.
    vec3 ao_col = mix(vec3(1.0), vec3(0.82, 0.78, 0.92), (1.0 - ao) * 0.55);

    vec3 lit = albedo * (sun * ndl * energy * prox * ao_dir
                       + fill * amb * bounce_energy * ao * ao_col
                       + vec3(0.085, 0.094, 0.108) * (0.40 + 0.60 * rama_day) * ao * ao_col)
              + vec3(0.006, 0.008, 0.011) * (0.30 + 0.70 * rama_day) * ao;

    // --- Rim light / back-scatter ---
    // Surfaces grazing the view toward the axis strip catch a warm edge.
    // Real miniatures always show this; flat-shaded voxels need it more.
    vec3 view = normalize(w_pos - CAMERA_POSITION_WORLD);
    float rim_dot = 1.0 - max(dot(w_norm, -view), 0.0);
    float rim_axis = max(dot(-view, to_axis), 0.0);
    float rim = pow(rim_dot, 2.5) * rim_axis * 0.18 * rama_day;
    lit += sun * rim * prox;

    // --- Subsurface scatter hint on green/grassy faces ---
    // Grass glows when back-lit. One transmitted-light term on green faces.
    float greenness = clamp(lit.g - max(lit.r, lit.b), 0.0, 1.0);
    float sss_dot = max(dot(w_norm, view), 0.0);  // face away from camera = toward light
    float sss = pow(sss_dot, 1.8) * greenness * 0.22 * rama_day * detail;
    lit += vec3(0.15, 0.28, 0.06) * sss * energy * 0.5;

    // Nature is less saturated than a palette suggests, and greens are the
    // worst offender — real vegetation sits olive, never emerald.
    greenness = clamp(lit.g - max(lit.r, lit.b), 0.0, 1.0);
    lit = mix(lit, vec3(dot(lit, vec3(0.35, 0.50, 0.15))), greenness * 0.42);
    lit.r += greenness * 0.045;

    // Lit surfaces gain a little saturation, shadowed ones lose it. Real film
    // does this and the eye reads it as depth.
    float sat = mix(0.86, 1.02, ndl);
    float ll = dot(lit, vec3(0.299, 0.587, 0.114));
    lit = mix(vec3(ll), lit, sat);

    // Aerial perspective: mix toward a DARKER cool air colour, and hold a
    // little of the original chroma so ridges still silhouette.
    float f = clamp((d - haze_start) / (haze_end - haze_start), 0.0, 1.0);
    f = pow(f, 1.15) * haze_max * rama_haze;
    // Aerial perspective is brighter looking toward the light strip, because
    // that is where the air is scattering from.
    vec3 cam_to_axis = normalize(vec3(-CAMERA_POSITION_WORLD.x, -CAMERA_POSITION_WORLD.y, 0.0));
    float toward = max(dot(-view, cam_to_axis), 0.0);
    // Aerial perspective is ADDITIVE inscatter: it lifts distant blacks, it
    // does not just tint them. Crushing the far side to pure black was the
    // giveaway that this was being treated as a tint.
    vec3 air = haze_color * (0.45 + 0.55 * rama_day) * (1.0 + 0.45 * pow(toward, 2.0));
    // Desaturate slightly with range, but don't lift value — lifting is bleach.
    float lum = dot(lit, vec3(0.299, 0.587, 0.114));
    vec3 faded = mix(lit, vec3(lum) * vec3(0.82, 0.90, 1.02), f * 0.55);
    ALBEDO = mix(faded, air, f);
}
