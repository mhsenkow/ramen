shader_type spatial;
render_mode ambient_light_disabled, specular_disabled, cull_disabled, depth_draw_opaque;

#include "rama_light.gdshaderinc"

// Vegetation that reads as volume: canopy gradient, gust sway, wrap lighting,
// and a thin-leaf SSS term. Key from the local sun; fill from bounce.

global uniform float rama_day;
global uniform float rama_haze;
global uniform float rama_gust;
uniform vec3 sun_tint : source_color = vec3(1.0, 0.95, 0.86);
uniform vec3 night_tint : source_color = vec3(0.34, 0.44, 0.62);
uniform vec3 bounce_tint : source_color = vec3(0.38, 0.48, 0.36);
uniform vec3 haze_color : source_color = vec3(0.170, 0.225, 0.290);
uniform float haze_start = 90.0;
uniform float haze_end = 900.0;
uniform float sway = 0.085;
uniform float model_height = 2.2;
uniform vec3 bark_tint : source_color = vec3(0.33, 0.23, 0.14);
uniform vec3 fruit_tint : source_color = vec3(0.66, 0.28, 0.18);
uniform float fade_start = 300.0;
uniform float fade_end = 450.0;

varying vec3 w_pos;
varying vec3 w_nrm;
varying vec4 v_col;
varying float v_up;
varying float v_haze_f;
varying float v_greenish;

void vertex() {
	v_up = clamp(VERTEX.y / model_height, 0.0, 1.0);
	vec3 local = VERTEX;

	vec3 origin = (MODEL_MATRIX * vec4(0.0, 0.0, 0.0, 1.0)).xyz;
	float ph = origin.x * 0.085 + origin.z * 0.07 + TIME * 0.95;
	float gust = 0.55 + 0.70 * rama_gust;
	float g = (sin(ph) + 0.4 * sin(ph * 2.7 + 1.3)) * gust;
	float flutter = sin(ph * 6.2 + origin.z * 0.2) * 0.35 * v_up * v_up;
	local.x += (g + flutter) * sway * v_up * v_up * model_height;
	local.z += (g * 0.6 + flutter * 0.5) * sway * v_up * v_up * model_height;

	w_pos = (MODEL_MATRIX * vec4(local, 1.0)).xyz;
	w_nrm = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
	v_col = COLOR;
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(w_pos, 1.0);
}

void fragment() {
	float dist = length(w_pos - CAMERA_POSITION_WORLD);
	float tf = smoothstep(fade_start, fade_end, dist);
	if (rama_dither_far(FRAGCOORD.xy, tf)) { discard; }

	vec3 N = normalize(w_nrm);
	NORMAL = normalize((VIEW_MATRIX * vec4(N, 0.0)).xyz);

	// COLOR.a is a material key baked into the source mesh: 0 wood, 0.5 fruit,
	// 1 leaf. A MultiMesh instance colour multiplies the mesh's vertex colour,
	// so wood and fruit — the two hues a per-instance LEAF colour can never
	// produce — are named here instead of baked in there, where they came out
	// as black sticks and black apples.
	vec3 leaf = srgb_to_linear(v_col.rgb);
	vec3 albedo = leaf;
	float key = v_col.a;
	if (key < 0.75) {
		vec3 solid = srgb_to_linear(mix(bark_tint, fruit_tint, smoothstep(0.15, 0.5, key))
				* dot(v_col.rgb, vec3(0.35, 0.42, 0.23)) * 2.6);
		albedo = mix(solid, leaf, smoothstep(0.5, 0.75, key));
	}
	float greenish = clamp(v_col.g - max(v_col.r, v_col.b) + 0.12, 0.0, 1.0) * step(0.75, key);
	albedo *= mix(1.0, 0.38 + 0.78 * v_up, greenish);
	albedo = mix(albedo, vec3(dot(albedo, vec3(0.32, 0.52, 0.16))), greenish * 0.38);
	albedo.r += greenish * 0.04;

	vec3 bounce = rama_bounce(bounce_tint);
	vec3 fill = albedo * (bounce * (0.34 + 0.40 * (1.0 - v_up)) * 0.68
					+ rama_sky_fill(rama_day));
	vec3 to_axis = rama_to_axis(w_pos);
	float sss = pow(max(dot(N, -to_axis), 0.0), 1.5) * greenish * 0.28 * rama_day;
	fill += vec3(0.16, 0.30, 0.07) * sss * rama_energy(1.65, rama_day);

	vec3 view = normalize(CAMERA_POSITION_WORLD - w_pos);
	float rim = pow(1.0 - max(dot(N, view), 0.0), 2.2) * max(dot(view, to_axis), 0.0);
	fill += rama_sun(night_tint, sun_tint, rama_day) * rim * 0.12 * rama_day;

	float f = rama_haze_factor(dist, haze_start, haze_end, 0.72, rama_haze);
	v_haze_f = f;
	v_greenish = greenish;

	ALBEDO = mix(albedo, vec3(0.0), f);
	EMISSION = mix(fill, rama_air(haze_color, rama_day), f);
}

void light() {
	vec3 sun = rama_sun(night_tint, sun_tint, rama_day);
	float energy = rama_energy(1.65, rama_day);
	// Wrap lighting — thin leaves have no hard terminator (§2039).
	float ndl = 0.36 + 0.64 * max(dot(NORMAL, LIGHT), 0.0);
	DIFFUSE_LIGHT += sun * ndl * energy * ATTENUATION * (1.0 - v_haze_f);
}
