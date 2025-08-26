precision highp float;

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
	return vec4(above.rgb + below.rgb * (1. - above.a), above.a + below.a * (1. - above.a));
}

vec4 blend(vec4 below, vec4 above) {
	float a = above.a + below.a * (1. - above.a);
	return vec4((above.rgb * above.a + below.rgb * below.a * (1. - above.a)) / a, a);
}

float toe_inv(float lr) {
	float k1 = 0.206;
	float k2 = 0.03;
	float k3 = (1. + k1) / (1. + k2);
	return (lr * (lr + k1)) / (k3 * (lr + k2));
}

float toe(float l) {
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

	return vec4(vec3(1.), (0.5 - 0.5 * mask.x*mask.y) * 0.25);
}

// vec4 rounded(vec4 color, float border_radius, vec2 uv, vec2 size) {
// 	border_radius *= 2.;
// 	vec2 dist_edge = min(uv, size - uv);
// 	float dist_corner = length(max(border_radius - dist_edge, 0.));
// 	float f = smoothstep(border_radius - 0.5, border_radius, dist_corner);
// 	return mix(color, vec4(BG, 1.), f);
// }

float compute_max_saturation(float a, float b) {
    float k0, k1, k2, k3, k4, wl, wm, ws;

    if (-1.88170328 * a - 0.80936493 * b > 1.0) {
        // Red component
        k0 = 1.19086277;
        k1 = 1.76576728;
        k2 = 0.59662641;
        k3 = 0.75515197;
        k4 = 0.56771245;
        wl = 4.0767416621;
        wm = -3.3077115913;
        ws = 0.2309699292;
    } else if (1.81444104 * a - 1.19445276 * b > 1.0) {
        // Green component
        k0 = 0.73956515;
        k1 = -0.45954404;
        k2 = 0.08285427;
        k3 = 0.12541070;
        k4 = 0.14503204;
        wl = -1.2684380046;
        wm = 2.6097574011;
        ws = -0.3413193965;
    } else {
        // Blue component
        k0 = 1.35733652;
        k1 = -0.00915799;
        k2 = -1.15130210;
        k3 = -0.50559606;
        k4 = 0.00692167;
        wl = -0.0041960863;
        wm = -0.7034186147;
        ws = 1.7076147010;
    };

    float ss = k0 + k1 * a + k2 * b + k3 * a * a + k4 * a * b;

    float k_l = 0.3963377774 * a + 0.2158037573 * b;
    float k_m = -0.1055613458 * a - 0.0638541728 * b;
    float k_s = -0.0894841775 * a - 1.2914855480 * b;

    {
        float l_ = 1.0 + ss * k_l;
        float m_ = 1.0 + ss * k_m;
        float s_ = 1.0 + ss * k_s;

        float l = l_*l_*l_;
        float m = m_*m_*m_;
        float s = s_*s_*s_;

        float l_d_s = 3.0 * k_l * l_ * l_;
        float m_d_s = 3.0 * k_m * m_ * m_;
        float s_d_s = 3.0 * k_s * s_ * s_;

        float l_d_s2 = 6.0 * k_l * k_l * l_;
        float m_d_s2 = 6.0 * k_m * k_m * m_;
        float s_d_s2 = 6.0 * k_s * k_s * s_;

        float f = wl * l + wm * m + ws * s;
        float f1 = wl * l_d_s + wm * m_d_s + ws * s_d_s;
        float f2 = wl * l_d_s2 + wm * m_d_s2 + ws * s_d_s2;

        ss -= f * f1 / (f1 * f1 - 0.5 * f * f2);
    }

    return ss;
}

vec2 find_cusp(float a, float b) {
    float s_cusp = compute_max_saturation(a, b);
    vec3 rgb_at_max = oklab_to_linear_srgb(vec3(1.0, s_cusp * a, s_cusp * b));
    float l_cusp = pow(1.0 / max(rgb_at_max.r, max(rgb_at_max.g, rgb_at_max.b)), 1.0/3.0);
    float c_cusp = l_cusp * s_cusp;
    return vec2(l_cusp, c_cusp);
}

vec2 to_st(vec2 cusp) {
    float l_cusp = cusp.x;
    float c_cusp = cusp.y;
    float s_cusp = c_cusp / l_cusp;
    float t_cusp = c_cusp / (1.0 - l_cusp);
    return vec2(s_cusp, t_cusp);
}

vec3 okhsv_to_oklab(vec3 hsv) {
    float h = hsv.x;
    float s = hsv.y;
    float v = hsv.z;

    float a_ = cos(2.0 * PI * h);
    float b_ = sin(2.0 * PI * h);

    vec2 cusp = find_cusp(a_, b_);
    vec2 st_max = to_st(cusp);
    float s_max = st_max.x;
    float t_max = st_max.y;

    float s_0 = 0.5;
    float k = 1.0 - s_0 / s_max;

    // L, C when v==1:
    float l_v = 1.0 - s * s_0 / (s_0 + t_max - t_max * k * s);
    float c_v = s * t_max * s_0 / (s_0 + t_max - t_max * k * s);

    float l = v * l_v;
    float c = v * c_v;

    // then we compensate for both toe and the curved top part of the triangle:
    float l_vt = toe_inv(l_v);
    float c_vt = c_v * l_vt / l_v;

    float l_new = toe_inv(l);
    c = c * l_new / l;
    l = l_new;

    vec3 rgb_scale = oklab_to_linear_srgb(vec3(l_vt, a_ * c_vt, b_ * c_vt));
    float scale_l = pow(1.0 / max(max(rgb_scale.r, rgb_scale.g), max(rgb_scale.b, 0.0)), 1.0/3.0);

    l *= scale_l;
    c *= scale_l;

    return vec3(l, c * a_, c * b_);
}

vec3 oklab_to_okhsv(vec3 lab) {
    float l = lab.x;
    float a = lab.y;
    float b = lab.z;

    float c = sqrt(a * a + b * b);
    if (c == 0.0) {
        return vec3(0.0, 0.0, toe(l));
    }

    float a_ = a / c;
    float b_ = b / c;

    float h = 0.5 + 0.5 * atan(-b, -a) / PI;

    vec2 cusp = find_cusp(a_, b_);
    vec2 st_max = to_st(cusp);
    float s_max = st_max.x;
    float t_max = st_max.y;

    float s_0 = 0.5;
    float k = 1.0 - s_0 / s_max;

    // first we find L_v, C_v, L_vt and C_vt
    float t = t_max / (c + l * t_max);
    float l_v = t * l;
    float c_v = t * c;

    float l_vt = toe_inv(l_v);
    float c_vt = c_v * l_vt / l_v;

    // we can then use these to invert the step that compensates for the toe and the curved top part of the triangle:
    vec3 rgb_scale = oklab_to_linear_srgb(vec3(l_vt, a_ * c_vt, b_ * c_vt));
    float scale_l = pow(1.0 / max(max(rgb_scale.r, rgb_scale.g), max(rgb_scale.b, 0.0)), 1.0/3.0);

    l /= scale_l;

    l = toe(l);

    // we can now compute v and s:
    float v = l / l_v;
    float s = (s_0 + t_max) * c_v / ((t_max * s_0) + t_max * k * c_v);

    return vec3(h, s, v);
}


vec3 okhsv_to_srgb(vec3 hsv) {
	return to_srgb(oklab_to_linear_srgb(okhsv_to_oklab(hsv)));
}
