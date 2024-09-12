use clap::{Parser, ValueEnum};

use crate::formats::{ColorFormat, CssColorFormat, RawColorFormat};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The color format to use (default: try to auto detect)
    ///
    /// Note that the auto detection can't distinguish some raw formats from each other.
    /// Only raw_rgb and raw_rgb_float are attempted.
    #[arg(short, long)]
    pub format: Option<CliColorFormat>,

    /// Color to pre-select (default: get a random color)
    pub color: Option<String>,
}

#[derive(ValueEnum, Default, Clone, Copy, strum::Display)]
#[clap(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CliColorFormat {
    #[default]
    Hex,
    Rgb,
    Oklch,
    Hsl,
    RawRgb,
    RawRgbFloat,
    RawRgbLinear,
    RawOklch,
}

impl From<CliColorFormat> for ColorFormat {
    fn from(value: CliColorFormat) -> Self {
        match value {
            CliColorFormat::Hex => CssColorFormat::Hex.into(),
            CliColorFormat::Rgb => CssColorFormat::Rgb.into(),
            CliColorFormat::Oklch => CssColorFormat::Oklch.into(),
            CliColorFormat::Hsl => CssColorFormat::Hsl.into(),
            CliColorFormat::RawRgb => RawColorFormat::Rgb.into(),
            CliColorFormat::RawRgbFloat => RawColorFormat::RgbFloat.into(),
            CliColorFormat::RawRgbLinear => RawColorFormat::RgbLinear.into(),
            CliColorFormat::RawOklch => RawColorFormat::Oklch.into(),
        }
    }
}
