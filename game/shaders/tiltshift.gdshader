shader_type canvas_item;

// The diorama read (PD_BRIEF.md §5). Also, conveniently, permission to drop
// distant LOD hard — the blur hides it.

uniform sampler2D screen_tex : hint_screen_texture, filter_linear_mipmap;
// The sharp core used to be one fifth of the screen height (band 0.10) with
// nearly two mip levels of blur outside it. In a drum that is the wrong shape:
// there is no horizon line to sit the band on, the ground under your own feet
// is at the bottom of the frame, and the inhabited far side is at the top. Both
// of those were mush. Wider core, gentler ramp, less blur at the extremes —
// still a diorama, but one you can read.
uniform float focus_y = 0.55;
uniform float band = 0.16;
uniform float falloff = 0.42;
uniform float max_blur = 1.25;
uniform float vignette = 0.22;
uniform float saturation = 0.96;
uniform float contrast = 1.05;
uniform float lift = 0.013;
uniform float grain = 0.018;
uniform float aberration = 0.75;
uniform float barrel = 0.25;           // radial warp % at edges
uniform float micro_contrast = 0.16;  // unsharp mask in focus band
uniform float bloom_threshold = 1.0; // highlight bloom in bokeh

float hash12(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

void fragment() {
    // --- Barrel distortion ---
    // Every real tilt-shift lens warps straight lines at the edges. Without
    // this, the post filter reads as "gaussian blur on a render", not
    // "photographed through glass".
    vec2 uv = SCREEN_UV;
    vec2 cent = uv - 0.5;
    float r2 = dot(cent, cent);
    uv = uv + cent * r2 * (barrel / 100.0);

    float d = abs(uv.y - focus_y);
    float k = smoothstep(band, band + falloff, d);
    vec2 v = (uv - 0.5) * 2.0;

    // Lateral chromatic aberration, scaled by blur and by distance from centre.
    // Tiny, but it is most of what separates "rendered" from "photographed".
    vec2 off = v * (aberration / 1000.0) * (0.35 + k);
    vec3 c;
    c.r = textureLod(screen_tex, uv + off, k * max_blur).r;
    c.g = textureLod(screen_tex, uv, k * max_blur).g;
    c.b = textureLod(screen_tex, uv - off, k * max_blur).b;

    // --- Highlight bloom in the bokeh zone ---
    // Bright pixels in the defocused region (axis strip, water sparkle, window
    // glow) bloom outward. This is what makes out-of-focus lights glow rather
    // than just smear.
    if (k > 0.05) {
        vec3 blurred = textureLod(screen_tex, uv, k * max_blur + 1.2).rgb;
        vec3 excess = max(blurred - vec3(bloom_threshold), vec3(0.0));
        c += excess * k * 0.55;
    }

    // --- Micro-contrast (unsharp mask) in the focus band ---
    // The focused region of a tilt-shift lens is always visually crunchier
    // than reality. A tiny sharpen makes voxel facets pop.
    if (k < 0.15) {
        vec3 soft = textureLod(screen_tex, uv, 1.2).rgb;
        float sharp_k = (1.0 - k / 0.15) * micro_contrast;
        c += (c - soft) * sharp_k;
    }

    // Filmic S-curve about mid grey. Restores the shoulder and toe that a
    // straight tonemap flattens, which is where "milky" comes from.
    c = clamp(c, 0.0, 4.0);
    c = (c - 0.5) * contrast + 0.5;
    c = clamp(c, 0.0, 4.0);
    c = c * c * (3.0 - 2.0 * clamp(c, 0.0, 1.0)) * 0.5 + c * 0.5;

    // Slight saturation lift: miniatures read as more colourful than reality.
    float l = dot(c, vec3(0.299, 0.587, 0.114));
    c = mix(vec3(l), c, saturation);

    // Print-black lift, so the darkest pixel is not pure zero.
    c += lift;

    c *= 1.0 - vignette * dot(v, v) * 0.5;

    // Film grain — luminance-aware: larger in shadows, warm-biased in highlights.
    // Real film grain varies with exposure; uniform grain is the computer tell.
    float g = hash12(FRAGCOORD.xy + fract(TIME) * 311.7) - 0.5;
    float shadow_grain = grain * (1.0 - smoothstep(0.0, 0.7, l)) * 1.4;
    float highlight_grain = grain * smoothstep(0.5, 1.0, l) * 0.6;
    c.r += g * (shadow_grain + highlight_grain * 1.15);
    c.g += g * (shadow_grain + highlight_grain * 1.0);
    c.b += g * (shadow_grain + highlight_grain * 0.85);

    COLOR = vec4(max(c, vec3(0.0)), 1.0);
}

