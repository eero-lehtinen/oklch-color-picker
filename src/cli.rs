use clap::Parser;

use crate::formats::ColorFormat;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The color format to use (default: try to auto detect)
    ///
    /// Note that the auto detection can't distinguish some raw formats from each other.
    /// Only raw_rgb and raw_rgb_float are attempted.
    #[arg(short, long)]
    pub format: Option<ColorFormat>,

    /// Color to pre-select (default: get a random color)
    pub color: Option<String>,
}
