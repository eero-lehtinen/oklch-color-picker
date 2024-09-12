use bevy_color::{ColorToComponents, ColorToPacked, Gray, Hsla, LinearRgba, Oklcha, Srgba};
use once_cell::sync::Lazy;
use regex::Regex;
use strum::{EnumIter, EnumString, IntoEnumIterator};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorFormat {
    Css(CssColorFormat),
    Raw(RawColorFormat),
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

fn num(v: f32, decimals: i32) -> f32 {
    let factor = 10.0f32.powi(decimals);
    let n = (v * factor).round() / factor;

    if n == 0. && n.is_sign_negative() {
        -n
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

pub fn format_color(c: Oklcha, fallback: Srgba, format: ColorFormat, use_alpha: bool) -> String {
    match format {
        ColorFormat::Css(format) => match format {
            CssColorFormat::Hex => fallback.to_hex(),
            CssColorFormat::Rgb => {
                let c = fallback.to_u8_array_no_alpha();
                let a = css_alpha(fallback.alpha);
                format!("rgb({} {} {}{})", c[0], c[1], c[2], a)
            }
            CssColorFormat::Oklch => {
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
                let c = fallback.to_u8_array();
                format!(
                    "{}, {}, {}{}",
                    c[0],
                    c[1],
                    c[2],
                    raw_alpha_u8(c[3], use_alpha)
                )
            }
            RawColorFormat::RgbFloat => {
                let c = fallback;
                format!(
                    "{:?}, {:?}, {:?}{}",
                    num(c.red, 4),
                    num(c.green, 4),
                    num(c.blue, 4),
                    raw_alpha(c.alpha, use_alpha)
                )
            }
            RawColorFormat::RgbLinear => {
                let c = LinearRgba::from(fallback);
                format!(
                    "{:?}, {:?}, {:?}{}",
                    num(c.red, 4),
                    num(c.green, 4),
                    num(c.blue, 4),
                    raw_alpha(c.alpha, use_alpha)
                )
            }
            RawColorFormat::Oklch => {
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

pub fn parse_components<C: ColorToComponents>(s: &str, use_alpha: bool) -> Option<C> {
    let mut components = [1.0f32; 4];
    let max_component = if use_alpha { 3 } else { 2 };

    for (i, part) in s.split(',').enumerate() {
        if i > max_component {
            return None;
        }
        components[i] = part.trim().parse::<f32>().ok()?;
    }
    Some(C::from_f32_array(components))
}

pub fn parse_components_u8<C: ColorToPacked>(s: &str, use_alpha: bool) -> Option<C> {
    let mut components = [255u8; 4];
    let max_component = if use_alpha { 3 } else { 2 };

    for (i, part) in s.split(',').enumerate() {
        if i > max_component {
            return None;
        }
        components[i] = part.trim().parse::<u8>().ok()?;
    }
    Some(C::from_u8_array(components))
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
        return s.parse().ok().map(CssNum::Percent);
    }
    s.parse().ok().map(CssNum::Num)
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

pub fn parse_color_unknown_format(s: &str) -> Option<(Oklcha, ColorFormat)> {
    let format_candidates = [RawColorFormat::Rgb, RawColorFormat::RgbFloat]
        .into_iter()
        .map(ColorFormat::Raw)
        .chain(CssColorFormat::iter().map(ColorFormat::Css));

    for format in format_candidates {
        if let Some(parsed) = parse_color(s, format, true) {
            return Some((parsed, format));
        }
        if matches!(format, ColorFormat::Raw(_)) {
            if let Some(parsed) = parse_color(s, format, false) {
                return Some((parsed, format));
            }
        }
    }
    None
}

/// NOTE: use_alpha is ignored with css colors
pub fn parse_color(s: &str, input_format: ColorFormat, use_alpha: bool) -> Option<Oklcha> {
    let color: Oklcha = match input_format {
        ColorFormat::Css(css_format) => match css_format {
            CssColorFormat::Hex => Srgba::hex(s).ok()?.into(),
            CssColorFormat::Oklch => {
                let mut c = Oklcha::WHITE;
                let iter = &mut css_words(css_named(s, "oklch")?);
                c.lightness = css_num(iter)?.apply();
                c.chroma = match css_num(iter)? {
                    CssNum::Num(n) => n,
                    CssNum::Percent(p) => p / 100. * 0.4,
                };
                c.hue = iter.next()?.parse().ok()?;
                c.alpha = parse_alpha(iter)?;
                c
            }
            CssColorFormat::Rgb => {
                let mut c = Srgba::WHITE;
                let iter = &mut css_words(css_named(s, "oklch")?);
                c.red = css_num(iter)?.apply();
                c.green = css_num(iter)?.apply();
                c.blue = css_num(iter)?.apply();
                c.alpha = parse_alpha(iter)?;
                c.into()
            }
            CssColorFormat::Hsl => {
                let mut c = Hsla::WHITE;
                let iter = &mut css_words(css_named(s, "oklch")?);
                c.hue = iter.next()?.parse().ok()?;
                c.saturation = css_num(iter)?.apply();
                c.lightness = css_num(iter)?.apply();
                c.alpha = parse_alpha(iter)?;
                c.into()
            }
        },
        ColorFormat::Raw(format) => match format {
            RawColorFormat::Rgb => parse_components_u8::<Srgba>(s, use_alpha)?.into(),
            RawColorFormat::RgbFloat => parse_components::<Srgba>(s, use_alpha)?.into(),
            RawColorFormat::RgbLinear => parse_components::<LinearRgba>(s, use_alpha)?.into(),
            RawColorFormat::Oklch => parse_components::<Oklcha>(s, use_alpha)?,
        },
    };

    Some(color)
}
