// Colour-space contract (LANDSCAPE_2200 §2001–2002).
// Palettes are authored sRGB; ALBEDO is consumed linear. Feeding sRGB numbers
// straight in bleaches the world: 0.14 linear reads as pale sage, not deep green.

const float RAMA_SRGB_GAMMA = 1.95;

vec3 srgb_to_linear(vec3 c) {
	return pow(max(c, vec3(0.0)), vec3(RAMA_SRGB_GAMMA));
}
