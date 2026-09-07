shader_type spatial;
render_mode cull_disabled, blend_mix, depth_draw_opaque;

// Zelda-style standing water: axis-strip lighting (same contract as terrain),
// screen-space depth shore foam, sim depth (COLOR.a) for body colour.

uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;

uniform vec3 shallow : source_color = vec3(0.24, 0.50, 0.84);
uniform vec3 deep : source_color = vec3(0.10, 0.26, 0.52);
uniform vec3 foam_color : source_color = vec3(0.88, 0.94, 0.98);
uniform vec3 sparkle : source_color = vec3(0.78, 0.90, 1.0);
uniform vec3 sky_tint : source_color = vec3(0.42, 0.62, 0.88);
uniform vec3 sun_tint : source_color = vec3(1.0, 0.95, 0.86);
uniform vec3 night_tint : source_color = vec3(0.34, 0.44, 0.62);
uniform vec3 bounce_tint : source_color = vec3(0.38, 0.48, 0.36);
uniform vec3 haze_color : source_color = vec3(0.170, 0.225, 0.290);
uniform float haze_start = 140.0;
uniform float haze_end = 2600.0;
uniform float haze_max = 0.82;
uniform float shore_dist = 1.85;
uniform float foam_dist = 0.70;
uniform float alpha_base = 0.66;
// Beer-Lambert absorption per channel. Red dies first, which is why deep water
// goes blue-green and not just dark. One exp() beats any colour ramp.
uniform vec3 absorb : source_color = vec3(0.46, 0.14, 0.09);
uniform float wave_scale = 0.55;
uniform float wave_amp = 0.16;

global uniform float rama_day;
global uniform float rama_haze;
varying vec3 w_pos;
varying vec3 w_nrm;
varying float v_depth;
varying float v_view_z;

void vertex() {
    w_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
    w_nrm = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
    v_depth = COLOR.a;
    vec4 view_p = MODELVIEW_MATRIX * vec4(VERTEX, 1.0);
    v_view_z = view_p.z;
}

void fragment() {
    vec3 N = normalize(w_nrm);
    vec3 view = normalize(CAMERA_POSITION_WORLD - w_pos);
    float dist = length(w_pos - CAMERA_POSITION_WORLD);
    float near = clamp(1.0 - dist / 480.0, 0.0, 1.0);

    // Screen-space water thickness vs terrain behind the surface (BotW shore).
    float depth_raw = textureLod(depth_tex, SCREEN_UV, 0.0).r;
    vec3 ndc = vec3(SCREEN_UV * 2.0 - 1.0, depth_raw);
    vec4 view_pos = INV_PROJECTION_MATRIX * vec4(ndc, 1.0);
    view_pos.xyz /= max(view_pos.w, 1e-5);
    float scene_z = -view_pos.z;
    float water_z = -v_view_z;
    float thickness = max(scene_z - water_z, 0.0);

    float shore_fade = smoothstep(0.0, shore_dist, thickness);
    float foam_mask = 1.0 - smoothstep(0.0, foam_dist, thickness);
    float foam_noise = 0.55 + 0.45 * sin(w_pos.x * 2.4 + w_pos.z * 1.9 + TIME * 0.7)
                      + 0.25 * sin(w_pos.y * 3.1 - TIME * 1.1);
    foam_mask = clamp(foam_mask * foam_noise, 0.0, 1.0);
    foam_mask *= near;

    // Directional wave train with ANALYTIC derivatives, so the normal is the
    // real slope of the surface rather than a sine pushed into a normal. Three
    // octaves at different headings kills the axis-aligned corduroy look.
    // Surface-tangent basis on the drum: tangential and axial.
    vec3 up_ax = normalize(vec3(-w_pos.x, -w_pos.y, 0.0));
    vec3 tang = normalize(cross(up_ax, vec3(0.0, 0.0, 1.0)));
    vec3 axial = vec3(0.0, 0.0, 1.0);
    float u = dot(w_pos, tang);
    float v = dot(w_pos, axial);

    float amp = wave_amp * near;
    float du = 0.0;
    float dv = 0.0;
    float rip = 0.0;
    // heading, frequency, speed, weight
    vec4 w1 = vec4( 0.94,  0.34, 0.42, 1.00);
    vec4 w2 = vec4(-0.42,  0.91, 0.78, 0.55);
    vec4 w3 = vec4( 0.30, -0.95, 1.55, 0.28);
    for (int i = 0; i < 3; i++) {
        vec4 W = i == 0 ? w1 : (i == 1 ? w2 : w3);
        float k = wave_scale * W.z;
        float ph = (u * W.x + v * W.y) * k + TIME * (0.55 + 0.5 * float(i));
        float sw = sin(ph);
        float cw = cos(ph);
        rip += sw * W.w;
        du += cw * k * W.x * W.w;
        dv += cw * k * W.y * W.w;
    }
    vec3 Nw = normalize(N - (tang * du + axial * dv) * amp);

    float ndv = clamp(dot(Nw, view), 0.0, 1.0);
    float fres = pow(1.0 - ndv, 2.15);

    float dnorm = clamp(v_depth / 3.2, 0.0, 1.0);
    float thick_cue = clamp(thickness / 4.0, 0.0, 1.0);
    // Optical path length through the water, in metres.
    float path = max(max(v_depth, thickness), 0.0);
    // Beer-Lambert transmittance: what survives the trip down and back.
    vec3 transmit = exp(-absorb * path * 1.6);
    float depth_cue = mix(0.82, 0.16, max(dnorm, thick_cue * 0.85)) + rip * 0.02 * near;
    vec3 body = mix(deep, shallow, clamp(depth_cue, 0.0, 1.0));
    // Tint by transmittance so shallow edges keep the bed's warmth and depth
    // grades continuously instead of stepping between two palette colours.
    body *= mix(vec3(1.0), transmit * 1.9, 0.55);

    // Subsurface colour shift: deep water desaturates and shifts toward teal.
    // Flat blue at all depths is the CG tell.
    float deep_shift = clamp(1.0 - depth_cue, 0.0, 1.0);
    // Deep water is dark TEAL, not black. Mixing toward near-zero here was
    // what turned every lake into a flat black polygon once the colour space
    // was corrected on the terrain.
    body = mix(body, vec3(0.11, 0.24, 0.28), deep_shift * 0.30);

    // sRGB palette into a linear ALBEDO, matching the terrain pipeline.
    body = pow(max(body, vec3(0.0)), vec3(1.95));

    // Animated caustics — cheap sine-based pattern projected onto shallow regions.
    // Makes sunlit standing water shimmer instead of reading as a flat plane.
    float caustic = 0.0;
    if (near > 0.05 && thickness < 3.5) {
        float c1 = sin(w_pos.x * 1.8 + w_pos.z * 2.1 + TIME * 1.4);
        float c2 = sin(w_pos.y * 2.3 - w_pos.z * 1.6 + TIME * 1.1);
        float c3 = sin((w_pos.x + w_pos.y) * 1.4 + TIME * 0.9);
        caustic = clamp((c1 * c2 + c3 * 0.5) * 0.5 + 0.3, 0.0, 1.0);
        caustic *= (1.0 - clamp(thickness / 3.5, 0.0, 1.0)) * near * 0.18 * rama_day;
    }

    vec3 to_axis = normalize(vec3(-w_pos.x, -w_pos.y, 0.0));
    float ndl = max(dot(Nw, to_axis), 0.0);
    ndl = pow(ndl, 0.70);
    float amb = 0.32 + 0.50 * (1.0 - ndl);
    vec3 sun = mix(night_tint, sun_tint, rama_day);
    float energy = 2.15 * (0.28 + 0.72 * rama_day);
    body = body * (sun * ndl * energy
                 + bounce_tint * amb * 0.62
                 + vec3(0.085, 0.098, 0.125) * (0.40 + 0.60 * rama_day))
         + vec3(0.010, 0.016, 0.026) * (0.30 + 0.70 * rama_day);

    // Add caustic pattern — warm-tinted, axis-light-dependent.
    body += vec3(1.0, 0.95, 0.82) * caustic * energy;

    vec3 R = reflect(-view, Nw);
    float sky_amt = clamp(length(R.xy) * 0.55 + R.z * 0.1, 0.0, 1.0);
    vec3 refl = mix(sky_tint * (0.30 + 0.50 * rama_day), sparkle, ndl * 0.35);
    body = mix(body, refl, fres * (0.28 + 0.32 * near) * (0.50 + 0.50 * sky_amt));
    vec3 halfv = normalize(to_axis + view);
    body += sparkle * pow(max(dot(Nw, halfv), 0.0), 40.0) * 0.45 * near * rama_day;

    body = mix(body, foam_color * (0.55 + 0.45 * rama_day), foam_mask * 0.85);

    // Soften cell-edge tiles: shallow fringe + screen shore.
    float sim_shore = smoothstep(0.05, 0.55, v_depth);
    float edge_soft = shore_fade * sim_shore;

    float f = clamp((dist - haze_start) / (haze_end - haze_start), 0.0, 1.0);
    f = pow(f, 1.15) * haze_max * rama_haze;
    vec3 air = haze_color * (0.45 + 0.55 * rama_day);
    ALBEDO = mix(body, air, f);

    float alpha = mix(alpha_base, 0.92, fres * 0.35 + dnorm * 0.30);
    alpha *= edge_soft;
    alpha = mix(alpha, 0.92, foam_mask * 0.65);
    // Distant sheets thin out so blue quads don't tile the midground.
    alpha *= mix(0.55, 1.0, near);
    ALPHA = alpha * (1.0 - f * 0.35);
}

