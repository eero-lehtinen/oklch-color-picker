use bevy_color::{Color, ColorToComponents, ColorToPacked, Hsla, LinearRgba, Oklcha, Srgba};
use strum::{EnumIter, EnumString, IntoEnumIterator};
use winnow::{
    ascii::{digit0, digit1, space0, space1},
    combinator::{alt, delimited, opt, separated, terminated},
    error::ParserError,
    PResult, Parser,
};

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

pub fn parse_color_unknown_format(s: &str) -> Option<(Color, ColorFormat, bool)> {
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

pub fn parse_color(s: &str, input_format: ColorFormat) -> Option<(Color, bool)> {
    let s = s.trim();

    match input_format {
        ColorFormat::Css(css_format) => {
            let color: Color = match css_format {
                CssColorFormat::Hex => Srgba::hex(s).ok()?.into(),
                CssColorFormat::Oklch => oklch_parser.parse(s).ok()?.into(),
                CssColorFormat::Rgb => rgb_parser.parse(s).ok()?.into(),
                CssColorFormat::Hsl => hsl_parser.parse(s).ok()?.into(),
            };

            Some((color, true))
        }
        ColorFormat::Raw(format) => match format {
            RawColorFormat::Rgb => color_components_u8_parser::<Srgba>.parse(s).ok()?.into(),
            RawColorFormat::RgbFloat => color_components_parser::<Srgba>.parse(s).ok()?.into(),
            RawColorFormat::RgbLinear => {
                color_components_parser::<LinearRgba>.parse(s).ok()?.into()
            }
            RawColorFormat::Oklch => color_components_parser::<Oklcha>.parse(s).ok()?.into(),
        },
    }
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
    Percent(f32),
}

impl CssNum {
    fn apply(self) -> f32 {
        self.apply_percent_max(1.)
    }

    fn apply_percent_max(self, max: f32) -> f32 {
        match self {
            Self::Num(n) => n,
            Self::Percent(n) => n / 100. * max,
        }
    }

    fn as_u8(&self) -> f32 {
        match self {
            Self::Num(n) => n.round() / 255.,
            Self::Percent(n) => n / 100. * 255.,
        }
    }
}

fn css_num_parser(input: &mut &str) -> PResult<CssNum> {
    (js_float_parser, opt("%"))
        .map(|(n, p)| {
            if p.is_some() {
                CssNum::Percent(n)
            } else {
                CssNum::Num(n)
            }
        })
        .parse_next(input)
}

fn css_angle_parser(input: &mut &str) -> PResult<f32> {
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

fn css_alpha_parser(input: &mut &str) -> PResult<f32> {
    opt(delimited(
        (space0, '/', space0),
        css_num_parser.map(|n| n.apply()),
        space0,
    ))
    .map(|n| n.unwrap_or(1.))
    .parse_next(input)
}

fn color_read_parser<'a, F, C: ColorToComponents, E: ParserError<&'a str>>(
    name: &'static str,
    inner: F,
) -> impl Parser<&'a str, C, E>
where
    F: Parser<&'a str, (f32, f32, f32, f32), E>,
{
    delimited(
        (name, space0),
        inner.map(|arr| C::from_f32_array([arr.0, arr.1, arr.2, arr.3])),
        (space0, ")"),
    )
}

fn oklch_parser(input: &mut &str) -> PResult<Oklcha> {
    color_read_parser(
        "oklch(",
        (
            terminated(css_num_parser.map(|n| n.apply()), space1),
            terminated(css_num_parser.map(|n| n.apply_percent_max(0.4)), space1),
            css_angle_parser,
            css_alpha_parser,
        ),
    )
    .parse_next(input)
}

fn rgb_parser(input: &mut &str) -> PResult<Srgba> {
    color_read_parser(
        "rgb(",
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
        "hsl(",
        (
            terminated(css_angle_parser, space1),
            terminated(css_num_parser.map(|n| n.apply()), space1),
            css_num_parser.map(|n| n.apply()),
            css_alpha_parser,
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
            (Oklcha::new(0.5, 0.4, 0.2, 1.).into(), true)
        );
    }

    #[test]
    fn oklch2() {
        assert_eq!(
            parse_color("oklch( 10% 100% 150 / 20% )", CssColorFormat::Oklch.into()).unwrap(),
            (Oklcha::new(0.1, 0.4, 150., 0.2).into(), true)
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
