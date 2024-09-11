#version 450

uniform float width;
in vec2 uv2;

const float PI = 3.14159265358979323846;
const vec3 BG = vec3(0.17, 0.17, 0.18);

float gamma_function_inverse(float v) {
	if (v <= 0.0) {
		return v;
	}
	if (v <= 0.0031308) {
		return 12.92 * v;
	}
	return 1.055 * pow(v, 1.0 / 2.4) - 0.055;
}

vec3 gamma_function_inverse(vec3 c) {
    return vec3(gamma_function_inverse(c.r), gamma_function_inverse(c.g), gamma_function_inverse(c.b));
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

	return gamma_function_inverse(rgb);
}

vec3 blend(vec3 below, vec4 above) {
	return above.a * above.rgb + (1.0 - above.a) * below;
}

