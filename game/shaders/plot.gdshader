shader_type spatial;
render_mode unshaded, cull_disabled, blend_mix, depth_draw_opaque;

// Colonist territory — soft radial fill that dies toward the centre so a
// 40 m claim reads as a ground stain / ring, not a green glass dome.

global uniform float rama_day;
varying vec3 w_pos;
varying float v_radial;
varying vec4 v_col;

void vertex() {
    w_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
    // CylinderMesh radius 1 → length(xz) is 0 at axis, 1 at rim.
    v_radial = length(VERTEX.xz);
    v_col = COLOR;
}

void fragment() {
    float dist = length(w_pos - CAMERA_POSITION_WORLD);
    // feature: claim ring readable ~8–120 m
    //
    // It used to carry to 220 m, and at that range a 40 m claim on a hillside
    // is a green veil across half the valley — the picture repainted by a UI
    // affordance. The RING is what says "someone's ground"; the fill only has
    // to whisper.
    float near = smoothstep(8.0, 28.0, dist);
    float far = 1.0 - smoothstep(45.0, 120.0, dist); // feature: claim ring 45-120 m
    float range = near * far;
    if (range < 0.02) {
        discard;
    }

    float r = clamp(v_radial, 0.0, 1.2);
    float fill = smoothstep(0.0, 0.55, r) * (1.0 - smoothstep(0.72, 1.02, r));
    float rim = smoothstep(0.78, 0.90, r) * (1.0 - smoothstep(0.96, 1.05, r));
    float mask = fill * 0.10 + rim * 1.0;
    if (mask < 0.02) {
        discard;
    }

    vec3 grass = mix(vec3(0.22, 0.38, 0.20), vec3(0.40, 0.62, 0.34), rim);
    grass *= 0.55 + 0.45 * rama_day;
    ALBEDO = mix(v_col.rgb, grass, 0.65);
    ALPHA = mask * v_col.a * range * (0.55 + 0.20 * rim);
}
