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
uniform vec3 bark_tint : source_color = vec3(0.38, 0.29, 0.21);
uniform vec3 fruit_tint : source_color = vec3(0.66, 0.28, 0.18);
uniform float fade_start = 300.0;
uniform float fade_end = 450.0;
// Voxel stands are one instanced cube per block, so VERTEX.y says nothing about
// where the block sits in a crown. Those multimeshes pass the height fraction
// in CUSTOM.x instead; every authored tree mesh keeps reading its own vertices.
uniform bool up_from_instance = false;
// Complementary dither in the same distance band: never hide a whole tier.
uniform bool proxy = false;
uniform bool material_grid = false;
varying float v_resident;
varying float v_edited;
// Metres the 0-1 crown gradient spans — must match woodscape::CROWN_SPAN.
uniform float crown_span = 22.0;

varying vec3 w_pos;
varying vec3 w_nrm;
varying vec4 v_col;
varying float v_up;
varying float v_haze_f;
varying float v_greenish;
varying float v_contact;

void vertex() {
	v_resident = INSTANCE_CUSTOM.x;
	v_edited = material_grid ? UV.x : 0.0;
	v_up = up_from_instance
			? clamp(INSTANCE_CUSTOM.x, 0.0, 1.0)
			: clamp(VERTEX.y * length(MODEL_MATRIX[1].xyz) / crown_span, 0.0, 1.0);
	if (material_grid) { v_up = UV.y; }
	vec3 local = VERTEX;

	vec3 origin = (MODEL_MATRIX * vec4(0.0, 0.0, 0.0, 1.0)).xyz;
	float ph = origin.x * 0.085 + origin.z * 0.07 + TIME * 0.95;
	float gust = 0.55 + 0.70 * rama_gust;
	float g = (sin(ph) + 0.4 * sin(ph * 2.7 + 1.3)) * gust;
	float flutter = sin(ph * 6.2 + origin.z * 0.2) * 0.35 * v_up * v_up;
	// A block sways with the crown it belongs to, not with its own 0.55 m edge.
	float sway_span = up_from_instance ? crown_span : model_height;
	local.x += (g + flutter) * sway * v_up * v_up * sway_span;
	local.z += (g * 0.6 + flutter * 0.5) * sway * v_up * v_up * sway_span;

	w_pos = (MODEL_MATRIX * vec4(local, 1.0)).xyz;
	w_nrm = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
	v_col = COLOR;
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(w_pos, 1.0);
}

void fragment() {
	float dist = length(w_pos - CAMERA_POSITION_WORLD);
	float tf = smoothstep(fade_start, fade_end, dist);
	if (material_grid && v_edited > 0.5) { tf = smoothstep(400.0, 480.0, dist); } // feature: edited stands retire with the 400–480 m forest band
	if (proxy && v_resident > 0.5) {
		float incoming = smoothstep(55.0, 85.0, dist);
		if (rama_dither_near(FRAGCOORD.xy, incoming)) { discard; }
	}
	if (rama_dither_far(FRAGCOORD.xy, tf)) { discard; }

	vec3 N = normalize(w_nrm);
	NORMAL = normalize((VIEW_MATRIX * vec4(N, 0.0)).xyz);

	// COLOR.a is a material key baked into the source mesh: 0 wood, 0.5 fruit,
	// 1 leaf. A MultiMesh instance colour multiplies the mesh's vertex colour,
	// so wood and fruit — the two hues a per-instance LEAF colour can never
	// produce — are named here instead of baked in there, where they came out
	// as black sticks and black apples.
	vec3 pigment = v_col.rgb;
	float green_mask = smoothstep(0.02, 0.16, pigment.g - max(pigment.r, pigment.b));
	float leaf_value = dot(pigment, vec3(0.28, 0.58, 0.14));
	pigment = mix(pigment, leaf_value * vec3(0.90, 1.055, 0.77), green_mask * 0.62);
	vec3 leaf = srgb_to_linear(pigment);
	vec3 albedo = leaf;
	float key = v_col.a;
	if (key < 0.75) {
		// source_color uniforms already arrive in linear space. Converting them
		// again crushed the bark into black silhouettes.
		vec3 solid = mix(bark_tint, fruit_tint, smoothstep(0.15, 0.5, key)) * 0.90;
		albedo = mix(solid, leaf, smoothstep(0.5, 0.75, key));
	}
	float foliage = smoothstep(0.58, 0.82, key);
	float greenish = green_mask * foliage;
	vec3 to_axis = rama_to_axis(w_pos);
	float crown_light = clamp(dot(N, to_axis) * 0.5 + 0.5, 0.0, 1.0);
	// A crown is a shaded volume, with warm new growth on its exposed tips.
	albedo *= mix(1.0, mix(0.70, 1.08, v_up) * mix(0.78, 1.0, crown_light), foliage);
	albedo *= mix(vec3(1.0), mix(vec3(0.87, 0.97, 1.08), vec3(1.06, 1.015, 0.91), v_up), foliage);
	float contact = mix(0.60, 1.0, smoothstep(0.015, 0.30, v_up));

	vec3 bounce = rama_bounce(bounce_tint);
	vec3 fill = albedo * (bounce * (0.34 + 0.20 * (1.0 - v_up)) * 0.60
					+ rama_sky_fill(rama_day)) * contact;
	float sss = pow(max(dot(N, -to_axis), 0.0), 1.5) * greenish * 0.16 * rama_day;
	fill += srgb_to_linear(vec3(0.46, 0.52, 0.20)) * sss * rama_energy(1.65, rama_day) * v_up;

	vec3 view = normalize(CAMERA_POSITION_WORLD - w_pos);
	float rim = pow(1.0 - max(dot(N, view), 0.0), 2.2) * max(dot(view, to_axis), 0.0);
	fill += leaf * rama_sun(night_tint, sun_tint, rama_day) * rim * 0.18 * rama_day * foliage;

	float f = rama_haze_factor(dist, haze_start, haze_end, 0.72, rama_haze);
	v_haze_f = f;
	v_greenish = foliage;
	v_contact = contact;

	ALBEDO = mix(albedo, vec3(0.0), f);
	EMISSION = mix(fill, rama_air(haze_color, rama_day), f);
}

void light() {
	vec3 sun = rama_sun(night_tint, sun_tint, rama_day);
	float energy = rama_energy(1.65, rama_day);
	// A narrow wrap keeps thin leaves soft while wood retains a clear terminator.
	float direct = max(dot(NORMAL, LIGHT), 0.0);
	float ndl = mix(direct, 0.20 + 0.80 * direct, v_greenish);
	DIFFUSE_LIGHT += sun * ndl * energy * v_contact * ATTENUATION * (1.0 - v_haze_f);
}
