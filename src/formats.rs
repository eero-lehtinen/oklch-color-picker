use std::sync::LazyLock;

use bevy_color::{Color, ColorToComponents, ColorToPacked, Hsla, LinearRgba, Oklcha, Srgba};
use clap::ValueEnum;
use strum::IntoEnumIterator;
use winnow::{
    ascii::{digit0, digit1, space0, space1},
    combinator::{alt, delimited, opt, separated, terminated},
    error::ParserError,
    PResult, Parser,
};

#[derive(ValueEnum, Default, Clone, Copy, strum::Display, strum::EnumIter, PartialEq, Eq)]
#[clap(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ColorFormat {
    #[default]
    Hex,
    Rgb,
    Oklch,
    Hsl,
    HexLiteral,
    RawRgb,
    RawRgbFloat,
    RawRgbLinear,
    RawOklch,
}

impl ColorFormat {
    fn is_auto_detectable(&self) -> bool {
        use ColorFormat as F;
        matches!(
            *self,
            F::Hex | F::Rgb | F::Oklch | F::Hsl | F::HexLiteral | F::RawRgb | F::RawRgbFloat
        )
    }

    // Not really dead but my lib system messes with compilation
    #[allow(dead_code)]
    pub fn needs_explicit_alpha(&self) -> bool {
        use ColorFormat as F;
        matches!(
            *self,
            F::HexLiteral | F::RawRgb | F::RawRgbFloat | F::RawRgbLinear | F::RawOklch
        )
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
        format!(" / {}%", num(alpha * 100., 1))
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

#[allow(unused)]
pub fn format_color(fallback: LinearRgba, format: ColorFormat, use_alpha: bool) -> String {
    match format {
        ColorFormat::Hex => {
            let arr = Srgba::from(fallback).to_u8_array();
            let short = arr.map(|c| (c / 17, c % 17));
            let is_short = short.iter().all(|(_, rem)| *rem == 0);

            let [r, g, b, a] = if is_short { short.map(|(d, _)| d) } else { arr };

            match (is_short, arr[3]) {
                (true, 255) => format!("#{:x}{:x}{:x}", r, g, b),
                (true, _) => format!("#{:x}{:x}{:x}{:x}", r, g, b, a),
                (false, 255) => format!("#{:02x}{:02x}{:02x}", r, g, b),
                _ => format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a),
            }
        }
        ColorFormat::Rgb => {
            let c = Srgba::from(fallback).to_u8_array_no_alpha();
            format!(
                "rgb({} {} {}{})",
                c[0],
                c[1],
                c[2],
                css_alpha(fallback.alpha)
            )
        }
        ColorFormat::Oklch => {
            let c = Oklcha::from(fallback);
            format!(
                "oklch({} {} {}{})",
                num(c.lightness, 4),
                num(c.chroma, 4),
                num(c.hue, 2),
                css_alpha(c.alpha)
            )
        }
        ColorFormat::Hsl => {
            let c = Hsla::from(fallback);
            format!(
                "hsl({} {} {}{})",
                num(c.hue, 2),
                num(c.saturation, 4),
                num(c.lightness, 4),
                css_alpha(c.alpha)
            )
        }
        ColorFormat::HexLiteral => {
            let [r, g, b, a] = Srgba::from(fallback).to_u8_array();
            if use_alpha {
                format!("0x{:02X}{:02X}{:02X}{:02X}", a, r, g, b)
            } else {
                format!("0x{:02X}{:02X}{:02X}", r, g, b)
            }
        }
        ColorFormat::RawRgb => {
            let c = Srgba::from(fallback).to_u8_array();
            format!(
                "{}, {}, {}{}",
                c[0],
                c[1],
                c[2],
                raw_alpha_u8(c[3], use_alpha)
            )
        }
        ColorFormat::RawRgbFloat => {
            let c = Srgba::from(fallback);
            format!(
                "{:?}, {:?}, {:?}{}",
                num(c.red, 4),
                num(c.green, 4),
                num(c.blue, 4),
                raw_alpha(c.alpha, use_alpha)
            )
        }
        ColorFormat::RawRgbLinear => {
            let c = fallback;
            format!(
                "{:?}, {:?}, {:?}{}",
                num(c.red, 4),
                num(c.green, 4),
                num(c.blue, 4),
                raw_alpha(c.alpha, use_alpha)
            )
        }
        ColorFormat::RawOklch => {
            let c = Oklcha::from(fallback);
            format!(
                "{:?}, {:?}, {:?}{}",
                num(c.lightness, 4),
                num(c.chroma, 4),
                num(c.hue, 2),
                raw_alpha(c.alpha, use_alpha)
            )
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
static UNKNOWN_FORMAT_CANDIDATES: LazyLock<Vec<ColorFormat>> = LazyLock::new(|| {
    ColorFormat::iter()
        .filter(ColorFormat::is_auto_detectable)
        .collect()
});

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_color_unknown_format(s: &str) -> Option<(Color, ColorFormat, bool)> {
    let s = s.trim();

    for format in UNKNOWN_FORMAT_CANDIDATES.iter().copied() {
        if let Some((parsed, use_alpha)) = parse_color_impl(s, format) {
            return Some((parsed, format, use_alpha));
        }
    }
    None
}

pub fn parse_color(s: &str, input_format: ColorFormat) -> Option<(Color, bool)> {
    let s = s.trim();
    parse_color_impl(s, input_format)
}

fn parse_color_impl(s: &str, input_format: ColorFormat) -> Option<(Color, bool)> {
    match input_format {
        ColorFormat::Hex => parse_hex(s.strip_prefix("#")?, true).map(|(c, _)| (c.into(), true)),
        ColorFormat::Oklch => oklch_parser.parse(s).ok().map(|c| (c.into(), true)),
        ColorFormat::Rgb => rgb_parser
            .parse(s)
            .or_else(|_| rgb_legacy_parser.parse(s))
            .ok()
            .map(|c| (c.into(), true)),
        ColorFormat::Hsl => hsl_parser
            .parse(s)
            .or_else(|_| hsl_legacy_parser.parse(s))
            .ok()
            .map(|c| (c.into(), true)),
        ColorFormat::HexLiteral => parse_hex(s.strip_prefix("0x")?, false)
            .map(|(c, has_alpha)| {
                let mut parts = c.to_f32_array();
                // Read as ARGB instead of RGBA
                if has_alpha {
                    parts.rotate_right(3);
                }
                (Srgba::from_f32_array(parts).into(), has_alpha)
            })?
            .into(),
        ColorFormat::RawRgb => color_components_u8_parser::<Srgba>.parse(s).ok()?.into(),
        ColorFormat::RawRgbFloat => color_components_parser::<Srgba>.parse(s).ok()?.into(),
        ColorFormat::RawRgbLinear => color_components_parser::<LinearRgba>.parse(s).ok()?.into(),
        ColorFormat::RawOklch => color_components_parser::<Oklcha>.parse(s).ok()?.into(),
    }
}

// Modified from bevy_color to be more general
pub fn parse_hex(hex: &str, allow_short: bool) -> Option<(Srgba, bool)> {
    match hex.len() {
        // RGB
        3 if allow_short => {
            let [l, b] = u16::from_str_radix(hex, 16).ok()?.to_be_bytes();
            let (r, g, b) = (l & 0x0F, (b & 0xF0) >> 4, b & 0x0F);
            (Srgba::rgb_u8(r << 4 | r, g << 4 | g, b << 4 | b), false)
        }
        // RGBA
        4 if allow_short => {
            let [l, b] = u16::from_str_radix(hex, 16).ok()?.to_be_bytes();
            let (r, g, b, a) = ((l & 0xF0) >> 4, l & 0xF, (b & 0xF0) >> 4, b & 0x0F);
            (
                Srgba::rgba_u8(r << 4 | r, g << 4 | g, b << 4 | b, a << 4 | a),
                true,
            )
        }
        // RRGGBB
        6 => {
            let [_, r, g, b] = u32::from_str_radix(hex, 16).ok()?.to_be_bytes();
            (Srgba::rgb_u8(r, g, b), false)
        }
        // RRGGBBAA
        8 => {
            let [r, g, b, a] = u32::from_str_radix(hex, 16).ok()?.to_be_bytes();
            (Srgba::rgba_u8(r, g, b, a), true)
        }
        _ => return None,
    }
    .into()
}

fn js_float_parser(input: &mut &str) -> PResult<f32> {
    alt(((digit1, opt(('.', digit0))).void(), ('.', digit1).void()))
        .take()
        .try_map(|s: &str| {
            lexical_parse_float::parse::parse_complete::<
                _,
                { lexical_parse_float::format::JAVASCRIPT_LITERAL },
            >(s.as_bytes(), &Default::default())
        })
        .parse_next(input)
}

fn color_components_parser<C: ColorToComponents + Into<Color>>(
    input: &mut &str,
) -> PResult<(Color, bool)> {
    separated(3..=4, js_float_parser, (space0, ',', space0))
        .map(|parts: Vec<f32>| {
            if parts.len() == 3 {
                (
                    C::from_f32_array_no_alpha(parts.try_into().unwrap()).into(),
                    false,
                )
            } else {
                (C::from_f32_array(parts.try_into().unwrap()).into(), true)
            }
        })
        .parse_next(input)
}

fn color_components_u8_parser<C: ColorToPacked + Into<Color>>(
    input: &mut &str,
) -> PResult<(Color, bool)> {
    separated(
        3..=4,
        digit1.try_map(|s: &str| s.parse::<u8>()),
        (space0, ',', space0),
    )
    .map(|parts: Vec<u8>| {
        if parts.len() == 3 {
            (
                C::from_u8_array_no_alpha(parts.try_into().unwrap()).into(),
                false,
            )
        } else {
            (C::from_u8_array(parts.try_into().unwrap()).into(), true)
        }
    })
    .parse_next(input)
}

enum CssNum {
    Num(f32),
    Percentage(CssPercentage),
}

impl CssNum {
    fn apply(self) -> f32 {
        self.apply_percent_max(1.)
    }

    fn apply_percent_max(self, max: f32) -> f32 {
        match self {
            Self::Num(n) => n,
            Self::Percentage(n) => n.apply_percent_max(max),
        }
    }

    fn as_u8(&self) -> f32 {
        match self {
            Self::Num(n) => n.round() / 255.,
            Self::Percentage(n) => n.as_u8(),
        }
    }
}

struct CssPercentage(f32);

impl CssPercentage {
    fn apply(self) -> f32 {
        self.apply_percent_max(1.)
    }

    fn apply_percent_max(self, max: f32) -> f32 {
        self.0 / 100. * max
    }

    fn as_u8(&self) -> f32 {
        self.0 / 100. * 255.
    }
}

fn css_percentage_parser(input: &mut &str) -> PResult<CssPercentage> {
    terminated(js_float_parser, "%")
        .map(CssPercentage)
        .parse_next(input)
}

fn css_legacy_num_parser(input: &mut &str) -> PResult<CssNum> {
    (js_float_parser, opt("%"))
        .map(|(n, p)| {
            if p.is_some() {
                CssNum::Percentage(CssPercentage(n))
            } else {
                CssNum::Num(n)
            }
        })
        .parse_next(input)
}

fn css_num_parser(input: &mut &str) -> PResult<CssNum> {
    alt((css_legacy_num_parser, "none".map(|_| CssNum::Num(0.)))).parse_next(input)
}

fn css_legacy_hue_parser(input: &mut &str) -> PResult<f32> {
    (js_float_parser, opt(alt(("deg", "rad", "grad", "turn"))))
        .map(|(n, unit)| {
            if let Some(unit) = unit {
                match unit {
                    "deg" => n,
                    "rad" => n.to_degrees(),
                    "grad" => (n / 400.) * 360.,
                    "turn" => n * 360.,
                    _ => unreachable!(),
                }
            } else {
                n
            }
            .rem_euclid(360.)
        })
        .parse_next(input)
}

fn css_hue_parser(input: &mut &str) -> PResult<f32> {
    alt((css_legacy_hue_parser, "none".map(|_| 0.))).parse_next(input)
}

fn css_alpha_parser(input: &mut &str) -> PResult<f32> {
    opt(delimited(
        (space0, '/', space0),
        css_num_parser.map(|n| n.apply()),
        space0,
    ))
    .map(|n| n.unwrap_or(1.))
    .parse_next(input)
}

fn css_legacy_alpha_parser(input: &mut &str) -> PResult<f32> {
    opt(delimited(
        (space0, ',', space0),
        css_legacy_num_parser.map(|n| n.apply()),
        space0,
    ))
    .map(|n| n.unwrap_or(1.))
    .parse_next(input)
}

fn color_read_parser<'a, F1, F2, C: ColorToComponents, E: ParserError<&'a str>>(
    name: F1,
    inner: F2,
) -> impl Parser<&'a str, C, E>
where
    F1: Parser<&'a str, (), E>,
    F2: Parser<&'a str, (f32, f32, f32, f32), E>,
{
    delimited(
        (name, "(", space0),
        inner.map(|arr| C::from_f32_array([arr.0, arr.1, arr.2, arr.3])),
        (space0, ")"),
    )
}

fn oklch_parser(input: &mut &str) -> PResult<Oklcha> {
    color_read_parser(
        "oklch".void(),
        (
            terminated(css_num_parser.map(|n| n.apply()), space1),
            terminated(css_num_parser.map(|n| n.apply_percent_max(0.4)), space1),
            css_hue_parser,
            css_alpha_parser,
        ),
    )
    .parse_next(input)
}

fn rgb_parser(input: &mut &str) -> PResult<Srgba> {
    color_read_parser(
        "rgb".void(),
        (
            terminated(css_num_parser.map(|n| n.as_u8()), space1),
            terminated(css_num_parser.map(|n| n.as_u8()), space1),
            css_num_parser.map(|n| n.as_u8()),
            css_alpha_parser,
        ),
    )
    .parse_next(input)
}

fn hsl_parser(input: &mut &str) -> PResult<Hsla> {
    color_read_parser(
        "hsl".void(),
        (
            terminated(css_hue_parser, space1),
            terminated(css_num_parser.map(|n| n.apply()), space1),
            css_num_parser.map(|n| n.apply()),
            css_alpha_parser,
        ),
    )
    .parse_next(input)
}

fn rgb_legacy_parser(input: &mut &str) -> PResult<Srgba> {
    color_read_parser(
        ("rgb", opt('a')).void(),
        (
            terminated(css_legacy_num_parser, (space0, ',', space0)),
            terminated(css_legacy_num_parser, (space0, ',', space0)),
            css_legacy_num_parser,
            css_legacy_alpha_parser,
        )
            .verify(|(r, g, b, _)| {
                matches!(
                    (r, g, b),
                    (
                        CssNum::Percentage(_),
                        CssNum::Percentage(_),
                        CssNum::Percentage(_)
                    ) | (CssNum::Num(_), CssNum::Num(_), CssNum::Num(_))
                )
            })
            .map(|(r, g, b, a)| (r.as_u8(), g.as_u8(), b.as_u8(), a)),
    )
    .parse_next(input)
}

fn hsl_legacy_parser(input: &mut &str) -> PResult<Hsla> {
    color_read_parser(
        ("hsl", opt('a')).void(),
        (
            terminated(css_legacy_hue_parser, (space0, ',', space0)),
            terminated(
                css_percentage_parser.map(|p| p.apply()),
                (space0, ',', space0),
            ),
            css_percentage_parser.map(|p| p.apply()),
            css_legacy_alpha_parser,
        ),
    )
    .parse_next(input)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn js_float() {
        let res = js_float_parser.parse("1.0");
        assert_eq!(res, Ok(1.0));
    }

    #[test]
    fn js_float2() {
        let res = js_float_parser.parse("1");
        assert_eq!(res, Ok(1.0));
    }

    #[test]
    fn js_float3() {
        let res = js_float_parser.parse(".1");
        assert_eq!(res, Ok(0.1));
    }

    #[test]
    fn js_float_fail() {
        let res = js_float_parser.parse(" 1.0");
        assert!(matches!(res, Err(..)))
    }

    #[test]
    fn js_float_fail2() {
        let res = js_float_parser.parse("1.0 ");
        assert!(matches!(res, Err(..)))
    }

    #[test]
    fn components() {
        let res = color_components_parser::<Srgba>.parse("1,0.5,1.");
        let color = Color::from(Srgba::rgb(1., 0.5, 1.));
        assert_eq!(res, Ok((color, false)));
    }

    #[test]
    fn components2() {
        let res = color_components_parser::<Srgba>.parse("1, 0.5 , 1. , 0.5");
        let color = Color::from(Srgba::new(1., 0.5, 1., 0.5));
        assert_eq!(res, Ok((color, true)));
    }

    #[test]
    fn hex1() {
        assert_eq!(
            parse_color("#aabbcc", ColorFormat::Hex).unwrap(),
            (Srgba::rgba_u8(170, 187, 204, 255).into(), true)
        );
    }

    #[test]
    fn hex2() {
        assert_eq!(
            parse_color("#aabbcc00", ColorFormat::Hex).unwrap(),
            (Srgba::rgba_u8(170, 187, 204, 0).into(), true)
        );
    }

    #[test]
    fn hex3() {
        assert_eq!(
            parse_color("#aaa", ColorFormat::Hex).unwrap(),
            (Srgba::rgba_u8(170, 170, 170, 255).into(), true)
        );
    }

    #[test]
    fn fail_hex1() {
        assert_eq!(parse_color("", ColorFormat::Hex), None);
    }

    #[test]
    fn fail_hex2() {
        assert_eq!(parse_color("#a", ColorFormat::Hex), None);
    }

    #[test]
    fn rgb1() {
        assert_eq!(
            parse_color("rgb(170 187 204)", ColorFormat::Rgb).unwrap(),
            (Srgba::rgba_u8(170, 187, 204, 255).into(), true)
        );
    }

    #[test]
    fn rgb2() {
        assert_eq!(
            parse_color("rgb(170 187 204 / 0.0)", ColorFormat::Rgb).unwrap(),
            (Srgba::rgba_u8(170, 187, 204, 0).into(), true)
        );
    }

    #[test]
    fn rgb3() {
        assert_eq!(
            parse_color("rgb(   170 187 204/.0%  )", ColorFormat::Rgb).unwrap(),
            (Srgba::rgba_u8(170, 187, 204, 0).into(), true)
        );
    }

    #[test]
    fn fail_rgb1() {
        assert_eq!(parse_color("170 187 204", ColorFormat::Rgb), None);
    }

    #[test]
    fn fail_rgb2() {
        assert_eq!(parse_color("rgb(1 2)", ColorFormat::Rgb), None);
    }
    #[test]
    fn fail_rgb3() {
        assert_eq!(parse_color("rgb()", ColorFormat::Rgb), None);
    }

    #[test]
    fn fail_rgb4() {
        assert_eq!(parse_color("rgb(x 1 1%)", ColorFormat::Rgb), None);
    }

    #[test]
    fn rgb_legacy() {
        assert_eq!(
            parse_color("rgba(255, 255, 255, 0.5)", ColorFormat::Rgb).unwrap(),
            (Srgba::new(1., 1., 1., 0.5).into(), true)
        );
    }

    #[test]
    fn fail_rgb_legacy_mixed_units() {
        assert_eq!(parse_color("rgb(1.0%, 1, 1)", ColorFormat::Rgb), None);
    }

    #[test]
    fn hsl_legacy() {
        assert_eq!(
            parse_color("hsla(50, 10%, 10%, 0.5)", ColorFormat::Hsl).unwrap(),
            (Hsla::new(50., 0.1, 0.1, 0.5).into(), true)
        );
    }

    #[test]
    fn oklch1() {
        assert_eq!(
            parse_color("oklch(0.5 0.4 0.2)", ColorFormat::Oklch).unwrap(),
            (Oklcha::new(0.5, 0.4, 0.2, 1.).into(), true)
        );
    }

    #[test]
    fn oklch2() {
        assert_eq!(
            parse_color("oklch( 10% 100% 150 / 20% )", ColorFormat::Oklch).unwrap(),
            (Oklcha::new(0.1, 0.4, 150., 0.2).into(), true)
        );
    }

    #[test]
    fn raw_rgb_float1() {
        assert_eq!(
            parse_color("0,.1,1.,0.2", ColorFormat::RawRgbFloat).unwrap(),
            (Srgba::new(0.0, 0.1, 1.0, 0.2).into(), true)
        );
    }

    #[test]
    fn raw_rgb_float2() {
        assert_eq!(
            parse_color("0,.1,1.", ColorFormat::RawRgbFloat).unwrap(),
            (Srgba::new(0.0, 0.1, 1.0, 1.0).into(), false)
        );
    }

    #[test]
    fn raw_rgb_float3() {
        assert_eq!(
            parse_color("0.0,     0.5,   0.8", ColorFormat::RawRgbFloat).unwrap(),
            (Srgba::new(0.0, 0.5, 0.8, 1.0).into(), false)
        );
    }

    #[test]
    fn fail_raw_rgb_float1() {
        assert_eq!(parse_color("0.0 0.5, 0.8", ColorFormat::RawRgbFloat), None);
    }

    #[test]
    fn fail_raw_rgb_float2() {
        assert_eq!(parse_color("0", ColorFormat::RawRgbFloat), None);
    }

    #[test]
    fn fail_raw_rgb_float3() {
        assert_eq!(parse_color("0, 0", ColorFormat::RawRgbFloat), None);
    }

    #[test]
    fn fail_raw_rgb_float4() {
        assert_eq!(parse_color("0, 0, 0, 0, 0", ColorFormat::RawRgbFloat), None);
    }

    #[test]
    fn raw_hex_literal() {
        assert_eq!(
            parse_color("0x001122", ColorFormat::HexLiteral),
            Some((Srgba::rgb_u8(0, 17, 34).into(), false))
        );
    }

    #[test]
    fn raw_hex_literal_alpha() {
        assert_eq!(
            parse_color("0x33001122", ColorFormat::HexLiteral),
            Some((Srgba::rgba_u8(0, 17, 34, 51).into(), true))
        );
    }
}
