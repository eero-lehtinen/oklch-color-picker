use clap::Parser;

use crate::formats::ColorFormat;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The input color format (default: try to auto detect)
    ///
    /// Note that the auto detection can't distinguish some raw formats from each other.
    /// Only raw_rgb and raw_rgb_float are attempted.
    #[arg(short, long)]
    pub format: Option<ColorFormat>,

    /// Convert the input to this format, print it, and exit without opening the picker
    #[arg(long, requires = "color")]
    pub convert_to: Option<ColorFormat>,

    /// Color to pre-select (default: get a random color)
    pub color: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_conversion_options() {
        let cli = Cli::try_parse_from([
            "oklch-color-picker",
            "0.72, 0.13, 225, 0.8",
            "--format",
            "raw_oklch",
            "--convert-to",
            "rgb",
        ])
        .unwrap();

        assert!(cli.format == Some(ColorFormat::RawOklch));
        assert!(cli.convert_to == Some(ColorFormat::Rgb));
        assert_eq!(cli.color.as_deref(), Some("0.72, 0.13, 225, 0.8"));
    }

    #[test]
    fn conversion_requires_input_color() {
        assert!(Cli::try_parse_from(["oklch-color-picker", "--convert-to", "hex"]).is_err());
    }
}
