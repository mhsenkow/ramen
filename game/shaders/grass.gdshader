shader_type spatial;
render_mode ambient_light_disabled, specular_disabled, cull_disabled, depth_draw_opaque;

#include "rama_light.gdshaderinc"

// Ground cover. One draw call, shadows off. Gust-driven sway so the field
// breathes as a single organism. Key from the local sun for cast-on contact.

global uniform float rama_day;
global uniform float rama_gust;
uniform vec3 sun_tint : source_color = vec3(1.0, 0.95, 0.86);
uniform vec3 night_tint : source_color = vec3(0.34, 0.44, 0.62);
uniform vec3 bounce_tint : source_color = vec3(0.38, 0.48, 0.36);
uniform vec3 haze_color : source_color = vec3(0.170, 0.225, 0.290);
uniform float fade_start = 22.0;
uniform float fade_end = 36.0;
uniform float sway = 0.34;

varying vec3 w_pos;
varying vec3 w_nrm;
varying vec4 v_col;
varying float v_h;
varying float v_haze_f;

void vertex() {
	v_h = clamp(VERTEX.y, 0.0, 1.0);
	vec3 local = VERTEX;
	vec3 wp = (MODEL_MATRIX * vec4(0.0, 0.0, 0.0, 1.0)).xyz;
	float ph = wp.x * 0.11 + wp.z * 0.09 + TIME * 1.35;
	float gust = 0.65 + 0.55 * rama_gust;
	float g = sin(ph) * gust + 0.35 * sin(ph * 2.7 + 1.7)
			+ 0.18 * sin(ph * 5.1 + wp.x * 0.3);
	local.x += g * sway * v_h * v_h;
	local.z += g * sway * 0.55 * v_h * v_h;
	local.x += sin(ph * 3.3 + v_h * 4.0) * sway * 0.12 * v_h;

	w_pos = (MODEL_MATRIX * vec4(local, 1.0)).xyz;
	w_nrm = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
	v_col = COLOR;
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(w_pos, 1.0);
}

void fragment() {
	float d = length(w_pos - CAMERA_POSITION_WORLD);
	float t = smoothstep(fade_start, fade_end, d);
	if (rama_dither_far(FRAGCOORD.xy, t)) { discard; }

	vec3 N = normalize(w_nrm);
	NORMAL = normalize((VIEW_MATRIX * vec4(N, 0.0)).xyz);

	vec3 albedo = srgb_to_linear(v_col.rgb);
	albedo *= 0.38 + 0.62 * v_h;
	albedo += vec3(0.04, 0.06, 0.01) * v_h * rama_day;

	float greenness = clamp(albedo.g - max(albedo.r, albedo.b), 0.0, 1.0);
	albedo = mix(albedo, vec3(dot(albedo, vec3(0.35, 0.50, 0.15))), greenness * 0.35);
	albedo.r += greenness * 0.03;

	vec3 bounce = rama_bounce(bounce_tint);
	vec3 to_axis = rama_to_axis(w_pos);
	float sss = pow(max(dot(N, -to_axis), 0.0), 1.6) * 0.22 * rama_day;
	vec3 fill = albedo * (bounce * 0.58 + rama_sky_fill(rama_day))
			  + vec3(0.18, 0.32, 0.08) * sss * rama_energy(2.15, rama_day);

	// feature: grass blade haze beyond fade band (~20–280 m)
	float f = clamp((d - 20.0) / 260.0, 0.0, 1.0) * 0.55;
	v_haze_f = f;

	ALBEDO = mix(albedo, vec3(0.0), f);
	EMISSION = mix(fill, rama_air(haze_color, rama_day), f);
}

void light() {
	vec3 sun = rama_sun(night_tint, sun_tint, rama_day);
	float energy = rama_energy(2.15, rama_day);
	float ndl = 0.42 + 0.58 * max(dot(NORMAL, LIGHT), 0.0);
	DIFFUSE_LIGHT += sun * ndl * energy * 0.62 * ATTENUATION * (1.0 - v_haze_f);
}
