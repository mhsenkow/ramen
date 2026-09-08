shader_type spatial;
render_mode ambient_light_disabled, specular_disabled, cull_back, depth_draw_opaque;

#include "rama_light.gdshaderinc"

// Light in an O'Neill cylinder comes from a strip along the axis. A local
// DirectionalLight3D approximates that strip within the focus band so
// ATTENUATION carries real shadows (LANDSCAPE_3200 §AX). Fill + haze stay
// emission so the contract survives past the shadow distance.

global uniform float rama_day;
global uniform float rama_haze;
uniform vec3 sun_tint : source_color = vec3(1.0, 0.95, 0.86);
uniform vec3 night_tint : source_color = vec3(0.34, 0.44, 0.62);
uniform vec3 bounce_tint : source_color = vec3(0.38, 0.48, 0.36);
uniform vec3 haze_color : source_color = vec3(0.170, 0.225, 0.290);
uniform float sun_energy = 2.05;
uniform float bounce_energy = 0.62;
uniform float haze_start = 140.0;
uniform float haze_scale = 2400.0;
uniform float haze_max = 0.72;
uniform float hab_radius = 900.0;
uniform float near_fade_start = 0.0;
uniform float near_fade_end = 0.0;
uniform float far_cut_start = 0.0;
uniform float far_cut_end = 0.0;
uniform vec3 shadow_tint : source_color = vec3(0.42, 0.52, 0.70);
uniform float ao_strength = 0.85;
uniform float face_jitter = 0.035;
uniform float mottle = 0.30;
uniform float relief = 0.24;
uniform int dbg = 0;
// Offset dither pattern per LOD tier so near/mid/far never discard the same
// pixel and punch a hole through to the void.
uniform float dither_seed = 0.0;

float hash13(vec3 p3) {
	p3 = fract(p3 * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

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
varying float v_ao_dir;
varying float v_prox;
varying float v_haze_f;
varying vec3 v_albedo;

void vertex() {
	w_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	w_norm = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
	v_col = COLOR;
}

void fragment() {
	float d = length(w_pos - CAMERA_POSITION_WORLD);
	vec2 dith = FRAGCOORD.xy + vec2(dither_seed * 13.7, dither_seed * 7.1);
	if (far_cut_end > far_cut_start) {
		float tf = smoothstep(far_cut_start, far_cut_end, d);
		if (rama_dither_far(dith, tf)) {
			discard;
		}
	}
	if (near_fade_end > near_fade_start) {
		float t = smoothstep(near_fade_start, near_fade_end, d);
		if (rama_dither_near(dith, t)) {
			discard;
		}
	}

	vec3 to_axis = rama_to_axis(w_pos);
	vec3 surface_normal = normalize(w_norm);
	vec3 sn = surface_normal;
	float slope = 1.0 - max(dot(surface_normal, to_axis), 0.0);
	// feature: relief ~2.4 m — retire before ~170 m
	float near_relief = 1.0 - smoothstep(45.0, 170.0, d);
	if (near_relief > 0.004) {
		float e = 0.9;
		vec3 t1 = normalize(cross(surface_normal, vec3(0.0, 0.0, 1.0)) + vec3(1e-5));
		vec3 t2 = cross(surface_normal, t1);
		float h0 = vnoise(w_pos * 0.42);
		float hx = vnoise((w_pos + t1 * e) * 0.42);
		float hz = vnoise((w_pos + t2 * e) * 0.42);
		// Sample in the surface tangent plane, so relief follows the cylinder.
		// Rock gets more grain; a meadow keeps its broad sculpted silhouette.
		sn = normalize(surface_normal - (t1 * (hx - h0) + t2 * (hz - h0))
								* relief * near_relief * mix(2.2, 4.2, slope));
	}
	NORMAL = normalize((VIEW_MATRIX * vec4(sn, 0.0)).xyz);

	float r = length(w_pos.xy);
	float prox = clamp(0.58 + 0.42 * (1.0 - r / hab_radius), 0.5, 1.05);
	float elev_frac = clamp((hab_radius - r) / 120.0, 0.0, 1.0);

	float slope_ao = 1.0 - 0.30 * (1.0 - max(dot(w_norm, to_axis), 0.0));
	float ao = mix(1.0, min(v_col.a, slope_ao), ao_strength);
	float ao_dir = mix(1.0, ao, 0.35);

	// Authored grass palettes used to become lime under the warm axis light.
	// Retain their biome values while compressing chroma toward meadow sage.
	vec3 pigment = v_col.rgb;
	float vegetation = smoothstep(0.018, 0.12, pigment.g - max(pigment.r, pigment.b));
	float pigment_value = dot(pigment, vec3(0.28, 0.58, 0.14));
	pigment = mix(pigment, pigment_value * vec3(0.92, 1.05, 0.79), vegetation * 0.66);
	vec3 albedo = srgb_to_linear(pigment);

	// Slope-split cliff tone (§AZ / Wave 1.2): steep faces read as rock, not clay.
	float cliff = smoothstep(0.36, 0.76, slope);
	// feature: mineral colour beds 35–65 m; haze retires these across the drum.
	float mineral = vnoise(w_pos * vec3(0.017, 0.023, 0.029));
	vec3 cliff_col = srgb_to_linear(mix(vec3(0.39, 0.43, 0.44), vec3(0.55, 0.49, 0.40), mineral));
	albedo = mix(albedo, cliff_col, cliff * 0.64);

	albedo.r *= 1.0 + elev_frac * 0.08;
	albedo.b *= 1.0 - elev_frac * 0.05;
	albedo.g *= 1.0 + (1.0 - elev_frac) * 0.04;

	float detail = 1.0 - smoothstep(55.0, 240.0, d); // feature: mottling retires ~240 m
	if (detail > 0.001) {
		float m1 = vnoise(w_pos * 0.055);
		float m2 = vnoise(w_pos * 0.21);
		float m3 = vnoise(w_pos * 0.52);
		// feature: 4.8 m and 1.9 m grain retire at 140 m and 65 m respectively.
		float grain_mid = 1.0 - smoothstep(35.0, 140.0, d);
		float grain_near = 1.0 - smoothstep(18.0, 65.0, d);
		float m = (m1 - 0.5) * 0.62 + (m2 - 0.5) * 0.28 * grain_mid
				+ (m3 - 0.5) * 0.10 * grain_near;
		albedo *= 1.0 + m * mottle * detail;
		albedo.r *= 1.0 + m * mottle * 0.55 * detail;
		albedo.b *= 1.0 - m * mottle * 0.40 * detail;
		// feature: 0.56 m soil granules retire by 35 m, before they can shimmer.
		float litter = vnoise(w_pos * 1.8) - 0.5;
		albedo *= 1.0 + litter * 0.08 * detail * (1.0 - smoothstep(12.0, 35.0, d));

		// Dune ripples — sand-warm albedo + gentle slope; wind along +z (Wave 6).
		float sandness = clamp(albedo.r - albedo.g * 0.55 - albedo.b * 0.35, 0.0, 1.0);
		sandness *= 1.0 - cliff;
		if (sandness > 0.04 && near_relief > 0.02) {
			float sand_th = atan(w_pos.y, w_pos.x);
			float arc = sand_th * hab_radius;
			float rip = sin(w_pos.z * 0.55 + arc * 0.08) * 0.55
					  + sin(w_pos.z * 1.35 - arc * 0.03) * 0.28
					  + sin(w_pos.z * 2.8 + TIME * 0.02) * 0.12;
			float crest = smoothstep(0.15, 0.85, rip * 0.5 + 0.5);
			albedo *= 1.0 + (crest - 0.5) * 0.22 * sandness * near_relief;
			albedo.r += crest * 0.04 * sandness * near_relief;
			albedo.b -= crest * 0.03 * sandness * near_relief;
		}
		// Peat mottling — dark olive wet greens.
		float peat = clamp(albedo.g - albedo.r * 0.9 - albedo.b * 0.4, 0.0, 1.0);
		if (peat > 0.05) {
			float bog = vnoise(w_pos * 0.9) * vnoise(w_pos * 0.18);
			albedo = mix(albedo, srgb_to_linear(vec3(0.12, 0.22, 0.14)), bog * peat * 0.35 * detail);
		}
	}

	// Broad patchiness — 40-120 m features. Everything above is metre-scale and
	// retires by 240 m, which left every field past that one flat value of
	// green: the single flattest thing in the picture. This one survives the
	// distance, and haze takes it before it goes sub-pixel.
	// feature: ground patches 40-120 m
	float patch = vnoise(w_pos * 0.0115) * 0.66 + vnoise(w_pos * 0.031) * 0.34 - 0.5;
	albedo *= 1.0 + patch * 0.42;
	// Dry ochre and cool green patches give the meadow a readable larger scale.
	vec3 meadow_tone = mix(vec3(0.88, 1.025, 1.04), vec3(1.13, 0.965, 0.80),
			smoothstep(-0.22, 0.23, patch));
	albedo *= mix(vec3(1.0), meadow_tone, vegetation * (1.0 - cliff));

	// feature: 1.7 m pigment flecks; smooth samples and retirement prevent
	// the old cell hash from becoming pixel-sized sparkle on the far wall.
	float fleck_fade = 1.0 - smoothstep(24.0, 90.0, d);
	float fh = vnoise(w_pos * 0.6 + surface_normal * 3.7) - 0.5;
	albedo *= 1.0 + fh * 2.0 * face_jitter * fleck_fade;

	float greenness = clamp(albedo.g - max(albedo.r, albedo.b), 0.0, 1.0);

	// Cloud shadows crawl across the far wall (Wave 2). Arc metres — never
	// high-frequency theta bands (LANDSCAPE_3200 §2286 / Wave 2 anti-streak).
	//
	// And SMOOTH noise, never a raw hash. `fract(sin(dot(p, k)) * 43758.0)` of a
	// continuous coordinate has no feature size at all: every pixel is an
	// independent sample, so across the drum it aliased into exactly the radial
	// streaks the arc-metre rule exists to prevent. feature: shadow decks ~220 m
	float th = atan(w_pos.y, w_pos.x);
	float arc_m = th * hab_radius; // metre-ok: circumference scale
	float csh = vnoise(vec3(arc_m * 0.0045 + TIME * 0.012, w_pos.z * 0.0018, 3.7));
	csh = smoothstep(0.46, 0.70, csh);
	float cloud_shadow = mix(1.0, 0.82, csh * rama_day * (1.0 - near_relief * 0.5));
	albedo *= cloud_shadow;

	vec3 bounce = rama_bounce(bounce_tint);
	vec3 fill = mix(shadow_tint, bounce, 0.55);
	vec3 ao_col = mix(vec3(1.0), vec3(0.82, 0.78, 0.92), (1.0 - ao) * 0.55);
	float amb = 0.32 + 0.50 * cliff;

	vec3 fill_lit = albedo * (fill * amb * bounce_energy * ao * ao_col
					+ rama_sky_fill(rama_day) * ao * ao_col)
				  + vec3(0.010, 0.012, 0.016) * (0.35 + 0.65 * rama_day) * ao;

	vec3 view = normalize(w_pos - CAMERA_POSITION_WORLD);
	float rim_dot = 1.0 - max(dot(w_norm, -view), 0.0);
	float rim_axis = max(dot(-view, to_axis), 0.0);
	float rim = pow(rim_dot, 2.5) * rim_axis * 0.11 * rama_day;
	vec3 sun = rama_sun(night_tint, sun_tint, rama_day);
	fill_lit += albedo * sun * rim * prox;

	float sss_dot = max(dot(w_norm, view), 0.0);
	float sss = pow(sss_dot, 1.8) * greenness * 0.22 * rama_day * detail;
	fill_lit += srgb_to_linear(vec3(0.42, 0.48, 0.20)) * sss * rama_energy(sun_energy, rama_day) * 0.35;

	float f = rama_haze_exp(d, haze_start, haze_scale, haze_max, rama_haze);
	vec3 cam_to_axis = rama_to_axis(CAMERA_POSITION_WORLD);
	float toward = max(dot(-view, cam_to_axis), 0.0);
	vec3 air = rama_air_toward(haze_color, rama_day, toward);

	v_albedo = albedo;
	v_ao_dir = ao_dir * prox;
	v_prox = prox;
	v_haze_f = f;

	// Key arrives via light()*ATTENUATION; fill+haze via emission.
	ALBEDO = mix(albedo, vec3(0.0), f);
	EMISSION = mix(fill_lit, air, f);

	if (dbg != 0) {
		if (dbg == 1) { ALBEDO = vec3(0.0); EMISSION = vec3(v_col.a); }
		else if (dbg == 2) { ALBEDO = vec3(0.0); EMISSION = vec3(prox); }
		else if (dbg == 4) { ALBEDO = vec3(0.0); EMISSION = vec3(cliff); }
		else if (dbg == 5) { ALBEDO = vec3(0.0); EMISSION = vec3(ao); }
		else if (dbg == 6) { ALBEDO = vec3(0.0); EMISSION = vec3(f); }
		else if (dbg == 7) { ALBEDO = vec3(0.0); EMISSION = albedo; }
		else if (dbg == 8) { ALBEDO = vec3(0.0); EMISSION = air; }
	}
}

void light() {
	vec3 sun = rama_sun(night_tint, sun_tint, rama_day);
	float energy = rama_energy(sun_energy, rama_day);
	float ndl = pow(max(dot(NORMAL, LIGHT), 0.0), 0.70);
	DIFFUSE_LIGHT += sun * ndl * energy * v_ao_dir * ATTENUATION * (1.0 - v_haze_f);
}
