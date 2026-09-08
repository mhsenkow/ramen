// Shared light contract for the O'Neill cylinder (LANDSCAPE_2200 §2014,
// LANDSCAPE_3200 §AX / §BI bounce).
// Axis strip = warm key; bounce = cool fill tinted by the far wall; haze =
// additive inscatter. A shader that opts out must say why in a comment (§2015).

#include "rama_color.gdshaderinc"

global uniform vec3 rama_bounce_tint;

vec3 rama_to_axis(vec3 w_pos) {
	return normalize(vec3(-w_pos.x, -w_pos.y, 0.0));
}

vec3 rama_sun(vec3 night_tint, vec3 sun_tint, float day) {
	return mix(night_tint, sun_tint, day);
}

float rama_energy(float base, float day) {
	return base * (0.28 + 0.72 * day);
}

vec3 rama_sky_fill(float day) {
	return vec3(0.085, 0.094, 0.108) * (0.40 + 0.60 * day);
}

// Far-side bounce: falls back to a farm-green default until world.gd publishes
// the opposite-sector mean albedo (§691 / Wave 1.4).
vec3 rama_bounce(vec3 fallback) {
	float w = rama_bounce_tint.r + rama_bounce_tint.g + rama_bounce_tint.b;
	return w > 0.01 ? rama_bounce_tint : fallback;
}

// Additive aerial perspective: lifts distant blacks, does not crush them.
vec3 rama_air(vec3 haze_color, float day) {
	return haze_color * (0.45 + 0.55 * day);
}

vec3 rama_air_toward(vec3 haze_color, float day, float toward) {
	return rama_air(haze_color, day) * (1.0 + 0.45 * pow(toward, 2.0));
}

float rama_haze_factor(float dist, float start, float end, float hazemax, float haze) {
	float f = clamp((dist - start) / max(end - start, 1.0), 0.0, 1.0);
	return pow(f, 1.15) * hazemax * haze;
}

// Beer-Lambert extinction — the same law pool.gdshader already uses for water.
float rama_haze_exp(float dist, float start, float scale, float hazemax, float haze) {
	float d = max(dist - start, 0.0);
	return hazemax * (1.0 - exp(-d / max(scale, 1.0))) * haze;
}

// Dither-discard LOD crossfade — no sorting cost, no overdraw (§2020).
float rama_dither_noise(vec2 fragcoord) {
	return fract(52.9829189 * fract(dot(fragcoord, vec2(0.06711056, 0.00583715))));
}

bool rama_dither_far(vec2 fragcoord, float t) {
	return t > 0.001 && t > rama_dither_noise(fragcoord);
}

bool rama_dither_near(vec2 fragcoord, float t) {
	return t < 0.999 && t < rama_dither_noise(fragcoord);
}
