shader_type spatial;
render_mode cull_disabled, blend_mix, depth_draw_opaque, unshaded;

// Major flowing channels — same blue language + axis lighting as pools.

uniform vec3 shallow : source_color = vec3(0.24, 0.50, 0.84);
uniform vec3 deep : source_color = vec3(0.05, 0.16, 0.44);
uniform vec3 sparkle : source_color = vec3(0.78, 0.90, 1.0);
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
    // Channels only read up close; far field is pool sheets + haze.
    float rfade = smoothstep(220.0, 40.0, dist);
    if (rfade < 0.03) {
        discard;
    }
    // Flow-aligned ripple: downstream frequency compresses for a visible
    // current read. Cross-flow stays slower for surface tension.
    float flow_along = w_pos.z * 0.72 + TIME * 1.45;  // primary downstream
    float flow_cross = w_pos.x * 0.38 + w_pos.y * 0.32;
    float rip = sin(flow_along)
              + sin(flow_along * 1.7 - TIME * 0.4) * 0.65
              + sin(flow_cross - TIME * 0.55) * 0.45;
    vec3 c = mix(deep, shallow, clamp(0.50 + rip * 0.16 * rfade, 0.0, 1.0));

    vec3 to_axis = normalize(vec3(-w_pos.x, -w_pos.y, 0.0));
    vec3 view = normalize(CAMERA_POSITION_WORLD - w_pos);
    float ndl = max(dot(to_axis, view), 0.0);
    ndl = pow(ndl, 0.70);
    float amb = 0.32 + 0.50 * (1.0 - ndl);
    vec3 sun = mix(night_tint, sun_tint, rama_day);
    float energy = 1.15 * (0.28 + 0.72 * rama_day);
    c = c * (sun * ndl * energy
           + bounce_tint * amb * 0.32
           + vec3(0.16, 0.18, 0.22) * (0.40 + 0.60 * rama_day));
    // Sharper specular glint — channels catch the axis strip as bright lines.
    vec3 halfv = normalize(to_axis + view);
    float spec = pow(max(dot(normalize(vec3(-w_pos.x, -w_pos.y, 0.0) + vec3(rip * 0.06 * rfade)), halfv), 0.0), 28.0);
    c += sparkle * (pow(ndl, 18.0) * 0.14 + spec * 0.22) * rfade * rama_day;

    float f = clamp((dist - haze_start) / (haze_end - haze_start), 0.0, 1.0);
    f = pow(f, 0.95) * 0.80 * rama_haze;
    vec3 air = haze_color * (0.22 + 0.55 * rama_day);
    ALBEDO = mix(c, air, f);
    ALPHA = mix(0.42, 0.72, rfade) * (1.0 - f * 0.65) * rfade;
}
