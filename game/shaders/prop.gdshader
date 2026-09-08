shader_type spatial;
render_mode ambient_light_disabled, specular_disabled, cull_back, depth_draw_opaque;

#include "rama_light.gdshaderinc"

// Placed props — stations, carcasses, dwellings. Same light contract as
// terrain so they cast and receive shadows instead of floating as bright blocks.

global uniform float rama_day;
global uniform float rama_haze;
uniform vec3 sun_tint : source_color = vec3(1.0, 0.95, 0.86);
uniform vec3 night_tint : source_color = vec3(0.34, 0.44, 0.62);
uniform vec3 bounce_tint : source_color = vec3(0.38, 0.48, 0.36);
uniform vec3 haze_color : source_color = vec3(0.170, 0.225, 0.290);
uniform float albedo_scale = 0.86;
uniform float haze_start = 100.0;
uniform float haze_end = 1200.0;
uniform float emission_boost = 0.0;

varying vec3 w_pos;
varying vec3 w_nrm;
varying vec4 v_col;
varying float v_haze_f;

void vertex() {
	w_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	w_nrm = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
	v_col = COLOR;
}

void fragment() {
	vec3 N = normalize(w_nrm);
	NORMAL = normalize((VIEW_MATRIX * vec4(N, 0.0)).xyz);

	vec3 pigment = v_col.rgb * albedo_scale;
	float value = dot(pigment, vec3(0.28, 0.58, 0.14));
	vec3 body = srgb_to_linear(mix(pigment, vec3(value), 0.08));
	vec3 bounce = rama_bounce(bounce_tint);
	float facing = max(dot(N, rama_to_axis(w_pos)), 0.0);
	float amb = 0.32 + 0.34 * (1.0 - facing);
	// Cool bounced light separates shaded planes from the warm sunlit faces.
	vec3 fill_tint = mix(bounce, vec3(0.32, 0.40, 0.49), 0.32);
	vec3 fill = body * (fill_tint * amb * 0.56 + rama_sky_fill(rama_day) * 0.86);
	// Dusk settlement lights — boosted by world.gd via emission_boost / colour.
	fill += body * emission_boost * (1.0 - rama_day);

	float dist = length(w_pos - CAMERA_POSITION_WORLD);
	float f = rama_haze_factor(dist, haze_start, haze_end, 0.70, rama_haze);
	v_haze_f = f;

	ALBEDO = mix(body, vec3(0.0), f);
	EMISSION = mix(fill, rama_air(haze_color, rama_day), f);
}

void light() {
	vec3 sun = rama_sun(night_tint, sun_tint, rama_day);
	float energy = rama_energy(1.32, rama_day);
	float ndl = pow(max(dot(NORMAL, LIGHT), 0.0), 0.86);
	DIFFUSE_LIGHT += sun * ndl * energy * ATTENUATION * (1.0 - v_haze_f);
}
