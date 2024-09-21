#version 330

out vec4 FragColor;

uniform vec2 size;
uniform uint supersample;

in vec2 uv;
in vec2 uv2;

const float PI = 3.14159265358979323846;
const float CHROMA_MAX = 0.33;

// Diamond
// const vec2 sample_positions[4] = vec2[4](
// 	vec2(0.5, 0.),
// 	vec2(0., 0.5),
// 	vec2(-0.5, 0.),
// 	vec2(0., -0.5)
// );


// Rotated grid (RGSS)
const vec2 sample_positions[4] = vec2[4](
	vec2(1./8., 3./8.),
	vec2(3./8., -1./8.),
	vec2(-1./8., -3./8.),
	vec2(-3./8., 1./8.)
);

vec3 to_srgb(vec3 c) {
	return mix(12.92 * c, 1.055 * pow(c, vec3(0.4166667)) - 0.055, step(0.0031308, c));
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

vec4 oklch_to_srgb(vec3 lch) {
	vec3 rgb = oklab_to_linear_srgb(oklch_to_oklab(lch));

	float a = 1.0 - float(any(lessThan(rgb, vec3(0.0))) || any(greaterThan(rgb, vec3(1.0))));

	return vec4(to_srgb(rgb), a);
}

vec4 premultiply(vec4 color) {
	return vec4(color.rgb * color.a, color.a);
}

vec4 blend_premultiplied(vec4 below, vec4 above) {
	return above + below * (1. - above.a);
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

vec4 checkerboard(float checker_size, float soften) {
	vec2 uv = uv / checker_size;
	uv.x *= size.x / size.y;

	vec2 a = abs(fract((2. * uv - soften) / 2.) - 0.5);
	vec2 b = abs(fract((2. * uv + soften) / 2.) - 0.5);
	vec2 mask = (a - b) / soften;

	return vec4(vec3(0.62), 0.5 - 0.5 * mask.x*mask.y);
}

// vec4 rounded(vec4 color, float border_radius, vec2 uv, vec2 size) {
// 	border_radius *= 2.;
// 	vec2 dist_edge = min(uv, size - uv);
// 	float dist_corner = length(max(border_radius - dist_edge, 0.));
// 	float f = smoothstep(border_radius - 0.5, border_radius, dist_corner);
// 	return mix(color, vec4(BG, 1.), f);
// }


