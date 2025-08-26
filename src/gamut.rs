#![allow(non_upper_case_globals)]

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

    result.red = result.red.clamp(0., 1.);
    result.green = result.green.clamp(0., 1.);
    result.blue = result.blue.clamp(0., 1.);

    // Don't bother if the result is very close
    if (rgba.red - result.red).abs() < 0.003
        && (rgba.green - result.green).abs() < 0.003
        && (rgba.blue - result.blue).abs() < 0.003
    {
        return rgba;
    }

    result
}

/// Toe function for L_r
pub fn lr_to_l(lr: f32) -> f32 {
    const k1: f32 = 0.206;
    const k2: f32 = 0.03;
    const k3: f32 = (1. + k1) / (1. + k2);
    (lr * (lr + k1)) / (k3 * (lr + k2))
}

/// Inverse toe function for L_r
pub fn l_to_lr(l: f32) -> f32 {
    const k1: f32 = 0.206;
    const k2: f32 = 0.03;
    const k3: f32 = (1. + k1) / (1. + k2);
    0.5 * (k3 * l - k1 + ((k3 * l - k1) * (k3 * l - k1) + 4. * k2 * k3 * l).sqrt())
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
            lr_to_l(oklrcha.lightness_r),
            oklrcha.chroma,
            oklrcha.hue,
            oklrcha.alpha,
        )
    }
}

impl From<Oklcha> for Oklrcha {
    fn from(oklcha: Oklcha) -> Self {
        Oklrcha {
            lightness_r: l_to_lr(oklcha.lightness),
            chroma: oklcha.chroma,
            hue: oklcha.hue,
            alpha: oklcha.alpha,
        }
    }
}
