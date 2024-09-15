use bevy_color::{ColorToComponents, ColorToPacked, Gray, Hsla, LinearRgba, Oklcha, Srgba};
use once_cell::sync::Lazy;
use regex::Regex;
use strum::{EnumIter, EnumString, IntoEnumIterator};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorFormat {
    Css(CssColorFormat),
    Raw(RawColorFormat),
}

impl From<CssColorFormat> for ColorFormat {
    fn from(value: CssColorFormat) -> Self {
        ColorFormat::Css(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, EnumString, strum::Display, Default)]
#[strum(serialize_all = "snake_case")]
pub enum CssColorFormat {
    #[default]
    Hex,
    Rgb,
    Oklch,
    Hsl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, EnumString, strum::Display, Default)]
#[strum(serialize_all = "snake_case")]
pub enum RawColorFormat {
    #[default]
    Rgb,
    RgbFloat,
    RgbLinear,
    Oklch,
    // TODO: Octal hex
}

impl From<RawColorFormat> for ColorFormat {
    fn from(value: RawColorFormat) -> Self {
        ColorFormat::Raw(value)
    }
}

fn num(v: f32, decimals: i32) -> f32 {
    let factor = 10.0f32.powi(decimals);
    let n = (v * factor).round() / factor;

    if n < 0. || n.is_sign_negative() {
        0.
    } else {
        n
    }
}

fn css_alpha(alpha: f32) -> String {
    if alpha < 1. {
        format!(" / {}", num(alpha, 4))
    } else {
        String::new()
    }
}

fn raw_alpha(alpha: f32, use_alpha: bool) -> String {
    if use_alpha {
        format!(", {:?}", num(alpha, 4))
    } else {
        String::new()
    }
}

fn raw_alpha_u8(alpha: u8, use_alpha: bool) -> String {
    if use_alpha {
        format!(", {}", alpha)
    } else {
        String::new()
    }
}

pub fn format_color(fallback: LinearRgba, format: ColorFormat, use_alpha: bool) -> String {
    match format {
        ColorFormat::Css(format) => match format {
            CssColorFormat::Hex => Srgba::from(fallback).to_hex(),
            CssColorFormat::Rgb => {
                let c = Srgba::from(fallback).to_u8_array_no_alpha();
                format!(
                    "rgb({} {} {}{})",
                    c[0],
                    c[1],
                    c[2],
                    css_alpha(fallback.alpha)
                )
            }
            CssColorFormat::Oklch => {
                let c = Oklcha::from(fallback);
                format!(
                    "oklch({} {} {}{})",
                    num(c.lightness, 4),
                    num(c.chroma, 4),
                    num(c.hue, 2),
                    css_alpha(c.alpha)
                )
            }
            CssColorFormat::Hsl => {
                let c = Hsla::from(fallback);
                format!(
                    "hsl({} {} {}{})",
                    num(c.hue, 2),
                    num(c.saturation, 4),
                    num(c.lightness, 4),
                    css_alpha(c.alpha)
                )
            }
        },
        ColorFormat::Raw(format) => match format {
            RawColorFormat::Rgb => {
                let c = Srgba::from(fallback).to_u8_array();
                format!(
                    "{}, {}, {}{}",
                    c[0],
                    c[1],
                    c[2],
                    raw_alpha_u8(c[3], use_alpha)
                )
            }
            RawColorFormat::RgbFloat => {
                let c = Srgba::from(fallback);
                format!(
                    "{:?}, {:?}, {:?}{}",
                    num(c.red, 4),
                    num(c.green, 4),
                    num(c.blue, 4),
                    raw_alpha(c.alpha, use_alpha)
                )
            }
            RawColorFormat::RgbLinear => {
                let c = fallback;
                format!(
                    "{:?}, {:?}, {:?}{}",
                    num(c.red, 4),
                    num(c.green, 4),
                    num(c.blue, 4),
                    raw_alpha(c.alpha, use_alpha)
                )
            }
            RawColorFormat::Oklch => {
                let c = Oklcha::from(fallback);
                format!(
                    "{:?}, {:?}, {:?}{}",
                    num(c.lightness, 4),
                    num(c.chroma, 4),
                    num(c.hue, 2),
                    raw_alpha(c.alpha, use_alpha)
                )
            }
        },
    }
}

pub fn parse_js_float(s: &str) -> Option<f32> {
    lexical_parse_float::parse::parse_complete::<
        _,
        { lexical_parse_float::format::JAVASCRIPT_LITERAL },
    >(s.as_bytes(), &Default::default())
    .ok()
}

pub fn parse_components<C: ColorToComponents>(s: &str) -> Option<(C, bool)> {
    let mut components = [1.0f32; 4];

    let mut i = 0;
    for part in s.split(',') {
        *components.get_mut(i)? = parse_js_float(part.trim())?;
        i += 1;
    }
    if i < 3 {
        return None;
    }

    let use_alpha = i == 4;
    Some((C::from_f32_array(components), use_alpha))
}

pub fn parse_components_u8<C: ColorToPacked>(s: &str) -> Option<(C, bool)> {
    let mut components = [255u8; 4];

    let mut i = 0;
    for part in s.split(',') {
        *components.get_mut(i)? = part.trim().parse::<u8>().ok()?;
        i += 1;
    }
    if i < 3 {
        return None;
    }

    let use_alpha = i == 4;
    Some((C::from_u8_array(components), use_alpha))
}

pub fn css_named<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    s.strip_prefix(name)?.strip_prefix('(')?.strip_suffix(')')
}

static CSS_WORDS_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"[\d\.%]+|\/"#).unwrap());

pub fn css_words(s: &str) -> impl Iterator<Item = &str> {
    CSS_WORDS_REGEX
        .captures_iter(s)
        .map(|c| c.get(0).unwrap().as_str())
}

enum CssNum {
    Num(f32),
    Percent(f32),
}

impl CssNum {
    fn apply(self) -> f32 {
        match self {
            Self::Num(n) => n,
            Self::Percent(n) => n / 100.,
        }
    }
}

fn css_num<'a>(iter: &mut impl Iterator<Item = &'a str>) -> Option<CssNum> {
    let s = iter.next()?;
    if let Some(s) = s.strip_suffix('%') {
        return parse_js_float(s).map(CssNum::Percent);
    }
    parse_js_float(s).map(CssNum::Num)
}

fn css_num_255<'a>(iter: &mut impl Iterator<Item = &'a str>) -> Option<f32> {
    match css_num(iter)? {
        CssNum::Num(n) => n / 255.,
        CssNum::Percent(p) => p / 100.,
    }
    .into()
}

fn parse_alpha<'a>(iter: &mut impl Iterator<Item = &'a str>) -> Option<f32> {
    let Some(slash) = iter.next() else {
        // If there is not alpha, just return 1.
        return Some(1.);
    };
    if slash != "/" {
        return None;
    }
    Some(css_num(iter)?.apply())
}

pub fn parse_color_unknown_format(s: &str) -> Option<(Oklcha, ColorFormat, bool)> {
    let format_candidates = [RawColorFormat::Rgb, RawColorFormat::RgbFloat]
        .into_iter()
        .map(ColorFormat::Raw)
        .chain(CssColorFormat::iter().map(ColorFormat::Css));

    for format in format_candidates {
        if let Some((parsed, use_alpha)) = parse_color(s, format) {
            return Some((parsed, format, use_alpha));
        }
    }
    None
}

pub fn parse_color(s: &str, input_format: ColorFormat) -> Option<(Oklcha, bool)> {
    match input_format {
        ColorFormat::Css(css_format) => {
            let color = match css_format {
                CssColorFormat::Hex => Srgba::hex(s).ok()?.into(),
                CssColorFormat::Oklch => {
                    let mut c = Oklcha::WHITE;
                    let iter = &mut css_words(css_named(s, "oklch")?);
                    c.lightness = css_num(iter)?.apply();
                    c.chroma = match css_num(iter)? {
                        CssNum::Num(n) => n,
                        CssNum::Percent(p) => p / 100. * 0.4,
                    };
                    c.hue = parse_js_float(iter.next()?)?;
                    c.alpha = parse_alpha(iter)?;
                    c
                }
                CssColorFormat::Rgb => {
                    let mut c = Srgba::WHITE;
                    let iter = &mut css_words(css_named(s, "rgb")?);
                    c.red = css_num_255(iter)?;
                    c.green = css_num_255(iter)?;
                    c.blue = css_num_255(iter)?;
                    c.alpha = parse_alpha(iter)?;
                    c.into()
                }
                CssColorFormat::Hsl => {
                    let mut c = Hsla::WHITE;
                    let iter = &mut css_words(css_named(s, "hsl")?);
                    c.hue = parse_js_float(iter.next()?)?;
                    c.saturation = css_num(iter)?.apply();
                    c.lightness = css_num(iter)?.apply();
                    c.alpha = parse_alpha(iter)?;
                    c.into()
                }
            };

            (color, true)
        }
        ColorFormat::Raw(format) => match format {
            RawColorFormat::Rgb => parse_components_u8::<Srgba>(s).map(|(c, a)| (c.into(), a))?,
            RawColorFormat::RgbFloat => parse_components::<Srgba>(s).map(|(c, a)| (c.into(), a))?,
            RawColorFormat::RgbLinear => {
                parse_components::<LinearRgba>(s).map(|(c, a)| (c.into(), a))?
            }
            RawColorFormat::Oklch => parse_components::<Oklcha>(s)?,
        },
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex1() {
        assert_eq!(
            parse_color("#aabbcc", CssColorFormat::Hex.into()).unwrap(),
            (Srgba::rgba_u8(170, 187, 204, 255).into(), true)
        );
    }

    #[test]
    fn hex2() {
        assert_eq!(
            parse_color("#aabbcc00", CssColorFormat::Hex.into()).unwrap(),
            (Srgba::rgba_u8(170, 187, 204, 0).into(), true)
        );
    }

    #[test]
    fn hex3() {
        assert_eq!(
            parse_color("#aaa", CssColorFormat::Hex.into()).unwrap(),
            (Srgba::rgba_u8(170, 170, 170, 255).into(), true)
        );
    }

    #[test]
    fn fail_hex1() {
        assert_eq!(parse_color("", CssColorFormat::Hex.into()), None);
    }

    #[test]
    fn fail_hex2() {
        assert_eq!(parse_color("#a", CssColorFormat::Hex.into()), None);
    }

    #[test]
    fn rgb1() {
        assert_eq!(
            parse_color("rgb(170 187 204)", CssColorFormat::Rgb.into()).unwrap(),
            (Srgba::rgba_u8(170, 187, 204, 255).into(), true)
        );
    }

    #[test]
    fn rgb2() {
        assert_eq!(
            parse_color("rgb(170 187 204 / 0.0)", CssColorFormat::Rgb.into()).unwrap(),
            (Srgba::rgba_u8(170, 187, 204, 0).into(), true)
        );
    }

    #[test]
    fn rgb3() {
        assert_eq!(
            parse_color("rgb(   170 187 204/.0%  )", CssColorFormat::Rgb.into()).unwrap(),
            (Srgba::rgba_u8(170, 187, 204, 0).into(), true)
        );
    }

    #[test]
    fn fail_rgb1() {
        assert_eq!(parse_color("170 187 204", CssColorFormat::Rgb.into()), None);
    }

    #[test]
    fn fail_rgb2() {
        assert_eq!(parse_color("rgb(1 2)", CssColorFormat::Rgb.into()), None);
    }

    #[test]
    fn fail_rgb3() {
        assert_eq!(parse_color("rgb()", CssColorFormat::Rgb.into()), None);
    }

    #[test]
    fn fail_rgb4() {
        assert_eq!(parse_color("rgb(x 1 1%)", CssColorFormat::Rgb.into()), None);
    }

    #[test]
    fn oklch1() {
        assert_eq!(
            parse_color("oklch(0.5 0.4 0.2)", CssColorFormat::Oklch.into()).unwrap(),
            (Oklcha::new(0.5, 0.4, 0.2, 1.), true)
        );
    }

    #[test]
    fn oklch2() {
        assert_eq!(
            parse_color("oklch(10% 100% 150 / 20%)", CssColorFormat::Oklch.into()).unwrap(),
            (Oklcha::new(0.1, 0.4, 150., 0.2), true)
        );
    }

    #[test]
    fn raw_rgb_float1() {
        assert_eq!(
            parse_color("0,.1,1.,0.2", RawColorFormat::RgbFloat.into()).unwrap(),
            (Srgba::new(0.0, 0.1, 1.0, 0.2).into(), true)
        );
    }

    #[test]
    fn raw_rgb_float2() {
        assert_eq!(
            parse_color("0,.1,1.", RawColorFormat::RgbFloat.into()).unwrap(),
            (Srgba::new(0.0, 0.1, 1.0, 1.0).into(), false)
        );
    }

    #[test]
    fn raw_rgb_float3() {
        assert_eq!(
            parse_color("0.0,     0.5,   0.8", RawColorFormat::RgbFloat.into()).unwrap(),
            (Srgba::new(0.0, 0.5, 0.8, 1.0).into(), false)
        );
    }

    #[test]
    fn fail_raw_rgb_float1() {
        assert_eq!(
            parse_color("0.0 0.5, 0.8", RawColorFormat::RgbFloat.into()),
            None
        );
    }

    #[test]
    fn fail_raw_rgb_float2() {
        assert_eq!(parse_color("0", RawColorFormat::RgbFloat.into()), None);
    }

    #[test]
    fn fail_raw_rgb_float3() {
        assert_eq!(parse_color("0, 0", RawColorFormat::RgbFloat.into()), None);
    }

    #[test]
    fn fail_raw_rgb_float4() {
        assert_eq!(
            parse_color("0, 0, 0, 0, 0", RawColorFormat::RgbFloat.into()),
            None
        );
    }
}
