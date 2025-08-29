//! Code adapted from
//! https://bottosson.github.io/posts/colorpicker
//! https://bottosson.github.io/posts/gamutclipping

#![allow(non_upper_case_globals)]

use std::f32::consts::PI;

use bevy_color::{LinearRgba, Oklaba, Oklcha};

#[allow(clippy::excessive_precision)]
pub fn compute_max_saturation(a: f32, b: f32) -> f32 {
    // Max saturation will be when one of r, g or b goes below zero.

    // Select different coefficients depending on which component goes below zero first
    let (k0, k1, k2, k3, k4, wl, wm, ws) = if -1.88170328 * a - 0.80936493 * b > 1.0 {
        // Red component
        (
            1.19086277,
            1.76576728,
            0.59662641,
            0.75515197,
            0.56771245,
            4.0767416621,
            -3.3077115913,
            0.2309699292,
        )
    } else if 1.81444104 * a - 1.19445276 * b > 1.0 {
        // Green component
        (
            0.73956515,
            -0.45954404,
            0.08285427,
            0.12541070,
            0.14503204,
            -1.2684380046,
            2.6097574011,
            -0.3413193965,
        )
    } else {
        // Blue component
        (
            1.35733652,
            -0.00915799,
            -1.15130210,
            -0.50559606,
            0.00692167,
            -0.0041960863,
            -0.7034186147,
            1.7076147010,
        )
    };

    // Approximate max saturation using a polynomial:
    let mut ss = k0 + k1 * a + k2 * b + k3 * a * a + k4 * a * b;

    // Do one step Halley's method to get closer
    // this gives an error less than 10e6, except for some blue hues where the dS/dh is close to infinite
    // this should be sufficient for most applications, otherwise do two/three steps

    let k_l = 0.3963377774 * a + 0.2158037573 * b;
    let k_m = -0.1055613458 * a - 0.0638541728 * b;
    let k_s = -0.0894841775 * a - 1.2914855480 * b;

    {
        let l_ = 1.0 + ss * k_l;
        let m_ = 1.0 + ss * k_m;
        let s_ = 1.0 + ss * k_s;

        let l = l_.powi(3);
        let m = m_.powi(3);
        let s = s_.powi(3);

        let l_d_s = 3.0 * k_l * l_ * l_;
        let m_d_s = 3.0 * k_m * m_ * m_;
        let s_d_s = 3.0 * k_s * s_ * s_;

        let l_d_s2 = 6.0 * k_l * k_l * l_;
        let m_d_s2 = 6.0 * k_m * k_m * m_;
        let s_d_s2 = 6.0 * k_s * k_s * s_;

        let f = wl * l + wm * m + ws * s;
        let f1 = wl * l_d_s + wm * m_d_s + ws * s_d_s;
        let f2 = wl * l_d_s2 + wm * m_d_s2 + ws * s_d_s2;

        ss -= f * f1 / (f1 * f1 - 0.5 * f * f2);
    }

    ss
}

pub fn find_cusp(a: f32, b: f32) -> (f32, f32) {
    let s_cusp = compute_max_saturation(a, b);

    let oklaba = Oklaba::new(1., s_cusp * a, s_cusp * b, 1.);

    let rgb_at_max = LinearRgba::from(oklaba);

    let l_cusp = (1. / rgb_at_max.red.max(rgb_at_max.green).max(rgb_at_max.blue)).cbrt();
    let c_cusp = l_cusp * s_cusp;

    (l_cusp, c_cusp)
}

#[allow(clippy::excessive_precision)]
fn find_gamut_intersection(a: f32, b: f32, ll1: f32, cc1: f32, ll0: f32) -> f32 {
    // Find the cusp of the gamut triangle
    let (ll, cc) = find_cusp(a, b);

    // Find the intersection for upper and lower half separately
    let mut t: f32;
    if ((ll1 - ll0) * cc - (ll - ll0) * cc1) <= 0.0 {
        // Lower half
        t = cc * ll0 / (cc1 * ll + cc * (ll0 - ll1));
    } else {
        // Upper half

        // First intersect with triangle
        t = cc * (ll0 - 1.0) / (cc1 * (ll - 1.0) + cc * (ll0 - ll1));

        // Then one step Halley's method
        {
            let dll = ll1 - ll0;
            let dcc = cc1;

            let k_l = 0.3963377774 * a + 0.2158037573 * b;
            let k_m = -0.1055613458 * a - 0.0638541728 * b;
            let k_s = -0.0894841775 * a - 1.2914855480 * b;

            let l_dt = dll + dcc * k_l;
            let m_dt = dll + dcc * k_m;
            let s_dt = dll + dcc * k_s;

            // If higher accuracy is required, 2 or 3 iterations of the following block can be used:
            {
                let ll = ll0 * (1.0 - t) + t * ll1;
                let cc = t * cc1;

                let l_ = ll + cc * k_l;
                let m_ = ll + cc * k_m;
                let s_ = ll + cc * k_s;

                let l = l_.powi(3);
                let m = m_.powi(3);
                let s = s_.powi(3);

                let l_dt = 3.0 * l_dt * l_ * l_;
                let m_dt = 3.0 * m_dt * m_ * m_;
                let s_dt = 3.0 * s_dt * s_ * s_;

                let l_dt2 = 6.0 * l_dt * l_dt * l_;
                let m_dt2 = 6.0 * m_dt * m_dt * m_;
                let s_dt2 = 6.0 * s_dt * s_dt * s_;

                let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s - 1.0;
                let r1 = 4.0767416621 * l_dt - 3.3077115913 * m_dt + 0.2309699292 * s_dt;
                let r2 = 4.0767416621 * l_dt2 - 3.3077115913 * m_dt2 + 0.2309699292 * s_dt2;

                let u_r = r1 / (r1 * r1 - 0.5 * r * r2);
                let mut t_r = -r * u_r;

                let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s - 1.0;
                let g1 = -1.2684380046 * l_dt + 2.6097574011 * m_dt - 0.3413193965 * s_dt;
                let g2 = -1.2684380046 * l_dt2 + 2.6097574011 * m_dt2 - 0.3413193965 * s_dt2;

                let u_g = g1 / (g1 * g1 - 0.5 * g * g2);
                let mut t_g = -g * u_g;

                let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s - 1.0;
                let b1 = -0.0041960863 * l_dt - 0.7034186147 * m_dt + 1.7076147010 * s_dt;
                let b2 = -0.0041960863 * l_dt2 - 0.7034186147 * m_dt2 + 1.7076147010 * s_dt2;

                let u_b = b1 / (b1 * b1 - 0.5 * b * b2);
                let mut t_b = -b * u_b;

                t_r = if u_r >= 0.0 { t_r } else { f32::MAX };
                t_g = if u_g >= 0.0 { t_g } else { f32::MAX };
                t_b = if u_b >= 0.0 { t_b } else { f32::MAX };

                t += t_r.min(t_g.min(t_b));
            }
        }
    }

    t
}

pub fn gamut_clip_preserve_chroma(rgba: LinearRgba) -> LinearRgba {
    if rgba.red <= 1.
        && rgba.green <= 1.
        && rgba.blue <= 1.
        && rgba.red >= 0.
        && rgba.green >= 0.
        && rgba.blue >= 0.
    {
        return rgba;
    }

    let laba = Oklaba::from(rgba);

    let ll = laba.lightness;
    let eps: f32 = 0.00001;
    let cc = eps.max((laba.a * laba.a + laba.b * laba.b).sqrt());
    let a_ = laba.a / cc;
    let b_ = laba.b / cc;

    let ll0 = ll.clamp(0., 1.);

    let t = find_gamut_intersection(a_, b_, ll, cc, ll0);
    let ll_clipped = ll0 * (1. - t) + t * ll;
    let cc_clipped = t * cc;

    let mut result = LinearRgba::from(Oklaba::new(
        ll_clipped,
        cc_clipped * a_,
        cc_clipped * b_,
        rgba.alpha,
    ));

    result = clamp_rgba(result);

    result
}

pub fn clamp_rgba(rgba: LinearRgba) -> LinearRgba {
    LinearRgba {
        red: rgba.red.clamp(0., 1.),
        green: rgba.green.clamp(0., 1.),
        blue: rgba.blue.clamp(0., 1.),
        alpha: rgba.alpha.clamp(0., 1.),
    }
}

const K1: f32 = 0.206;
const K2: f32 = 0.03;
const K3: f32 = (1. + K1) / (1. + K2);

/// L_r to L
pub fn toe_inv(lr: f32) -> f32 {
    (lr * (lr + K1)) / (K3 * (lr + K2))
}

/// L to L_r
pub fn toe(l: f32) -> f32 {
    0.5 * (K3 * l - K1 + ((K3 * l - K1) * (K3 * l - K1) + 4. * K2 * K3 * l).sqrt())
}

pub fn to_st((l_cusp, c_cusp): (f32, f32)) -> (f32, f32) {
    let s_cusp = c_cusp / l_cusp;
    let t_cusp = c_cusp / (1. - l_cusp);
    (s_cusp, t_cusp)
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Oklrcha {
    pub lightness_r: f32,
    pub chroma: f32,
    pub hue: f32,
    pub alpha: f32,
}

impl Oklrcha {
    pub fn new(lightness_r: f32, chroma: f32, hue: f32, alpha: f32) -> Self {
        Self {
            lightness_r,
            chroma,
            hue,
            alpha,
        }
    }
}

impl From<Oklrcha> for Oklcha {
    fn from(oklrcha: Oklrcha) -> Self {
        Oklcha::new(
            toe_inv(oklrcha.lightness_r),
            oklrcha.chroma,
            oklrcha.hue,
            oklrcha.alpha,
        )
    }
}

impl From<Oklcha> for Oklrcha {
    fn from(oklcha: Oklcha) -> Self {
        Oklrcha {
            lightness_r: toe(oklcha.lightness),
            chroma: oklcha.chroma,
            hue: oklcha.hue,
            alpha: oklcha.alpha,
        }
    }
}

impl From<Oklrcha> for LinearRgba {
    fn from(oklrcha: Oklrcha) -> Self {
        LinearRgba::from(Oklcha::from(oklrcha))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Okhsva {
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
    pub alpha: f32,
}

impl Okhsva {
    pub fn new(hue: f32, saturation: f32, value: f32, alpha: f32) -> Self {
        Self {
            hue,
            saturation,
            value,
            alpha,
        }
    }
}

impl From<Okhsva> for Oklaba {
    fn from(okhsv: Okhsva) -> Self {
        let h = okhsv.hue / 360.;
        let s = okhsv.saturation;
        let v = okhsv.value;

        if v == 0. {
            return Oklaba::new(0., 0., 0., okhsv.alpha);
        }

        let a_ = (2. * PI * h).cos();
        let b_ = (2. * PI * h).sin();

        let (l_cusp, c_cusp) = find_cusp(a_, b_);

        let (s_max, t_max) = to_st((l_cusp, c_cusp));

        let s_0 = 0.5;
        let k = 1. - s_0 / s_max;

        // L, C when v==1:
        let l_v = 1. - s * s_0 / (s_0 + t_max - t_max * k * s);
        let c_v = s * t_max * s_0 / (s_0 + t_max - t_max * k * s);

        let mut l = v * l_v;
        let mut c = v * c_v;

        // then we compensate for both toe and the curved top part of the triangle:
        let l_vt = toe_inv(l_v);
        let c_vt = c_v * l_vt / l_v;

        let l_new = toe_inv(l);
        c = c * l_new / l;
        l = l_new;

        let rgb_scale = LinearRgba::from(Oklaba::new(l_vt, a_ * c_vt, b_ * c_vt, 1.0));
        let scale_l = (1.
            / rgb_scale
                .red
                .max(rgb_scale.green)
                .max(rgb_scale.blue.max(0.)))
        .cbrt();

        l *= scale_l;
        c *= scale_l;

        Oklaba::new(l, c * a_, c * b_, okhsv.alpha)
    }
}

impl From<Oklaba> for Okhsva {
    fn from(oklaba: Oklaba) -> Self {
        let c = (oklaba.a * oklaba.a + oklaba.b * oklaba.b).sqrt();
        if c == 0. {
            return Okhsva::new(0., 0., 0., oklaba.alpha);
        }

        let a_ = oklaba.a / c;
        let b_ = oklaba.b / c;

        let mut l = oklaba.lightness;
        let h = 0.5 + 0.5 * (-oklaba.b).atan2(-oklaba.a) / PI;

        let (l_cusp, c_cusp) = find_cusp(a_, b_);
        let (s_max, t_max) = to_st((l_cusp, c_cusp));
        let s_0 = 0.5;
        let k = 1.0 - s_0 / s_max;

        // first we find L_v, C_v, L_vt and C_vt
        let t = t_max / (c + l * t_max);
        let l_v = t * l;
        let c_v = t * c;

        let l_vt = toe_inv(l_v);
        let c_vt = c_v * l_vt / l_v;

        // we can then use these to invert the step that compensates for the toe and the curved top part of the triangle:
        let rgb_scale = LinearRgba::from(Oklaba::new(l_vt, a_ * c_vt, b_ * c_vt, 1.0));
        let scale_l = (1.0
            / rgb_scale
                .red
                .max(rgb_scale.green)
                .max(rgb_scale.blue.max(0.0)))
        .cbrt();

        l /= scale_l;

        // These calculations exist in the source but aren't used for some reason in the end.
        // c /= scale_l;
        // c = c * toe(l) / l;

        l = toe(l);

        // we can now compute v and s:
        let v = l / l_v;
        let s = (s_0 + t_max) * c_v / ((t_max * s_0) + t_max * k * c_v);

        Okhsva::new(h * 360., s, v, oklaba.alpha)
    }
}

impl From<Okhsva> for LinearRgba {
    fn from(okhsv: Okhsva) -> Self {
        Oklaba::from(okhsv).into()
    }
}

impl From<Oklcha> for Okhsva {
    fn from(oklcha: Oklcha) -> Self {
        Oklaba::from(oklcha).into()
    }
}

impl From<Oklrcha> for Okhsva {
    fn from(oklrcha: Oklrcha) -> Self {
        Okhsva::from(Oklaba::from(Oklcha::from(oklrcha)))
    }
}

impl From<Okhsva> for Oklrcha {
    fn from(okhsv: Okhsva) -> Self {
        Oklrcha::from(Oklcha::from(Oklaba::from(okhsv)))
    }
}
