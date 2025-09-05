struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct Uniforms {
    prev_color: vec4<f32>,
    color: vec4<f32>,
    values: vec3<f32>,
    size: vec2<f32>,
    kind: u32,
    mode: u32,
    supersample: u32,
};

@group(0) @binding(0)
var<uniform> u_prev_color: vec4<f32>;
@group(0) @binding(1)
var<uniform> u_color: vec4<f32>;
@group(0) @binding(2)
var<uniform> u_values: vec3<f32>;
@group(0) @binding(3)
var<uniform> u_size: vec2<f32>;
@group(0) @binding(4)
var<uniform> u_kind: u32;
@group(0) @binding(5)
var<uniform> u_mode: u32;
@group(0) @binding(6)
var<uniform> u_supersample: u32;

const CHROMA_MAX = 0.33;
const PI = 3.141592653589793;

// Rotated grid (RGSS)
var<private> sample_positions: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(1.0/8.0, 3.0/8.0),
    vec2<f32>(3.0/8.0, -1.0/8.0),
    vec2<f32>(-1.0/8.0, -3.0/8.0),
    vec2<f32>(-3.0/8.0, 1.0/8.0)
);

// vec3 to_srgb(vec3 c) {
// 	return mix(12.92 * c, 1.055 * pow(c, vec3(0.4166667)) - 0.055, step(0.0031308, c));
// }
//
// vec4 to_srgba(vec4 c) {
//     return vec4(to_srgb(c.rgb), c.a);
// }
fn to_srgb(c: vec3f) -> vec3f {
    return mix(12.92 * c, 1.055 * pow(c, vec3f(0.4166667)) - 0.055, step(vec3f(0.0031308), c));
}

fn to_srgba(c: vec4f) -> vec4f {
    return vec4f(to_srgb(c.rgb), c.a);
}

fn oklch_to_oklab(c: vec3f) -> vec3f {
    return vec3f(
        c.x,
        c.y * cos(c.z * PI * 2.0),
        c.y * sin(c.z * PI * 2.0)
    );
}

fn oklab_to_linear(c: vec3f) -> vec3f {
    let l_ = c.x + 0.3963377774 * c.y + 0.2158037573 * c.z;
    let m_ = c.x - 0.1055613458 * c.y - 0.0638541728 * c.z;
    let s_ = c.x - 0.0894841775 * c.y - 1.291485548 * c.z;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    return vec3f(
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s
    );
}

fn oklch_to_linear_clamped(lch: vec3f) -> vec4f {
    let rgb = oklab_to_linear(oklch_to_oklab(lch));

    let a = select(1.0, 0.0, any(rgb < vec3f(0.0)) || any(rgb > vec3f(1.0)));

    return vec4f(rgb, a);
}

fn toe_inv(lr: f32) -> f32 {
    let k1 = 0.206;
    let k2 = 0.03;
    let k3 = (1.0 + k1) / (1.0 + k2);
    return (lr * (lr + k1)) / (k3 * (lr + k2));
}

fn toe(l: f32) -> f32 {
    let k1 = 0.206;
    let k2 = 0.03;
    let k3 = (1.0 + k1) / (1.0 + k2);
    return 0.5 * (k3 * l - k1 + sqrt((k3 * l - k1) * (k3 * l - k1) + 4.0 * k2 * k3 * l));
}

fn compute_max_saturation(a: f32, b: f32) -> f32 {
    var k0: f32;
    var k1: f32;
    var k2: f32;
    var k3: f32;
    var k4: f32;
    var wl: f32;
    var wm: f32;
    var ws: f32;

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

    var ss = k0 + k1 * a + k2 * b + k3 * a * a + k4 * a * b;

    let k_l = 0.3963377774 * a + 0.2158037573 * b;
    let k_m = -0.1055613458 * a - 0.0638541728 * b;
    let k_s = -0.0894841775 * a - 1.2914855480 * b;

    {
        let l_ = 1.0 + ss * k_l;
        let m_ = 1.0 + ss * k_m;
        let s_ = 1.0 + ss * k_s;

        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;

        let l_d_s = 3.0 * k_l * l_ * l_;
        let m_d_s = 3.0 * k_m * m_ * m_;
        let s_d_s = 3.0 * k_s * s_ * s_;

        let l_d_s2 = 6.0 * k_l * k_l * l_;
        let m_d_s2 = 6.0 * k_m * k_m * m_;
        let s_d_s2 = 6.0 * k_s * k_s * s_;

        let f = wl * l + wm * m + ws * s;
        let f1 = wl * l_d_s + wm * m_d_s + ws * s_d_s;
        let f2 = wl * l_d_s2 + wm * m_d_s2 + ws * s_d_s2;

        ss = ss - f * f1 / (f1 * f1 - 0.5 * f * f2);
    }

    return ss;
}

fn find_cusp(a: f32, b: f32) -> vec2f {
    let s_cusp = compute_max_saturation(a, b);
    let rgb_at_max = oklab_to_linear(vec3f(1.0, s_cusp * a, s_cusp * b));
    let l_cusp = pow(1.0 / max(rgb_at_max.r, max(rgb_at_max.g, rgb_at_max.b)), 1.0/3.0);
    let c_cusp = l_cusp * s_cusp;
    return vec2f(l_cusp, c_cusp);
}

fn to_st(cusp: vec2f) -> vec2f {
    let l_cusp = cusp.x;
    let c_cusp = cusp.y;
    let s_cusp = c_cusp / l_cusp;
    let t_cusp = c_cusp / (1.0 - l_cusp);
    return vec2f(s_cusp, t_cusp);
}

fn okhsv_to_oklab(hsv: vec3f) -> vec3f {
    let h = hsv.x;
    let s = hsv.y;
    let v = hsv.z;

    let a_ = cos(2.0 * PI * h);
    let b_ = sin(2.0 * PI * h);

    let cusp = find_cusp(a_, b_);
    let st_max = to_st(cusp);
    let s_max = st_max.x;
    let t_max = st_max.y;

    let s_0 = 0.5;
    let k = 1.0 - s_0 / s_max;

    // L, C when v==1:
    let l_v = 1.0 - s * s_0 / (s_0 + t_max - t_max * k * s);
    let c_v = s * t_max * s_0 / (s_0 + t_max - t_max * k * s);

    var l = v * l_v;
    var c = v * c_v;

    // then we compensate for both toe and the curved top part of the triangle:
    let l_vt = toe_inv(l_v);
    let c_vt = c_v * l_vt / l_v;

    let l_new = toe_inv(l);
    c = c * l_new / l;
    l = l_new;

    let rgb_scale = oklab_to_linear(vec3f(l_vt, a_ * c_vt, b_ * c_vt));
    let scale_l = pow(1.0 / max(max(rgb_scale.r, rgb_scale.g), max(rgb_scale.b, 0.0)), 1.0/3.0);

    l = l * scale_l;
    c = c * scale_l;

    return vec3f(l, c * a_, c * b_);
}

fn okhsv_to_linear(hsv: vec3f) -> vec4f {
    return vec4f(oklab_to_linear(okhsv_to_oklab(hsv)), 1.0);
}
