#version 330

uniform float width;
in vec2 uv2;

const float PI = 3.14159265358979323846;

float to_srgb(float v) {
	if (v <= 0.0) {
		return v;
	}
	if (v <= 0.0031308) {
		return 12.92 * v;
	}
	return 1.055 * pow(v, 1.0 / 2.4) - 0.055;
}

vec3 to_srgb(vec3 c) {
	return vec3(to_srgb(c.r), to_srgb(c.g), to_srgb(c.b));
}


vec3 oklch_to_oklab(vec3 c) {
	return vec3 (
		c.x,
		c.y * cos(c.z * PI * 2.0),
		c.y * sin(c.z * PI * 2.0)
	);
}

vec3 oklab_to_linear_srgb(vec3 c) {
	float l_ = c.x + 0.3963377774f * c.y + 0.2158037573f * c.z;
	float m_ = c.x - 0.1055613458f * c.y - 0.0638541728f * c.z;
	float s_ = c.x - 0.0894841775f * c.y - 1.2914855480f * c.z;

	float l = l_*l_*l_;
	float m = m_*m_*m_;
	float s = s_*s_*s_;

	return vec3(
		+4.0767416621f * l - 3.3077115913f * m + 0.2309699292f * s,
		-1.2684380046f * l + 2.6097574011f * m - 0.3413193965f * s,
		-0.0041960863f * l - 0.7034186147f * m + 1.7076147010f * s
	);
}

vec3 oklch_to_srgb(vec3 lch, out bool valid) {
	vec3 rgb = oklab_to_linear_srgb(oklch_to_oklab(lch));

	valid = !(any(lessThan(rgb, vec3(0.0))) || any(greaterThan(rgb, vec3(1.0))));

	return to_srgb(rgb);
}

vec4 blend(vec3 below, vec4 above) {
	return vec4(above.a * above.rgb + (1.0 - above.a) * below, 1.0);
}

float lr_to_l(float lr) {
	float k1 = 0.206;
	float k2 = 0.03;
	float k3 = (1. + k1) / (1. + k2);
	return (lr * (lr + k1)) / (k3 * (lr + k2));
}

float l_to_lr(float l) {
	float k1 = 0.206;
	float k2 = 0.03;
	float k3 = (1. + k1) / (1. + k2);
	return 0.5 * (k3 * l - k1 + sqrt((k3 * l - k1) * (k3 * l - k1) + 4. * k2 * k3 * l));
}

vec3 screen_space_dither(vec2 frag_coord) {
    vec3 dither = vec3(dot(vec2(171.0, 231.0), frag_coord)).xxx;
    dither = fract(dither.rgb / vec3(103.0, 71.0, 97.0));
    return (dither - 0.5) / 255.0;
}

const vec3 BG = vec3(0.35, 0.35, 0.35);

