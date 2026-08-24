# OKLCH Color Picker

[![Crates.io](https://img.shields.io/crates/v/oklch-color-picker)](https://crates.io/crates/oklch-color-picker)

<img src="https://github.com/user-attachments/assets/e7752d50-4e68-4aab-990a-ff3126952783" width="100%" alt="screenshot">

Try the web demo: https://oklch.eerolehtinen.fi/

**NOTE:** This is an application, even though crates.io detects it as a library. The "library" part only exposes lua bindings for color parsing in Neovim.

## Features

- Takes an input color as a cli argument and outputs the edited color to stdout
- Uses a perceptual colorspace (OKLCH) to allow intuitive editing
  - Consists of lightness, chroma and hue
  - Motivation: [An article by the OKLab creator](https://bottosson.github.io/posts/oklab/)
  - OKLCH uses the same theory as OKLab, but uses parameters that are easier to understand
  - L<sub>r</sub> estimate is used instead of L as specified in [another article by the same guy](https://bottosson.github.io/posts/colorpicker/#intermission---a-new-lightness-estimate-for-oklab)
- Supports many color formats for input and output (editing uses only OKLCH):
  - Hex (`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`)
  - Other common CSS formats (`rgb(..)`, `hsl(..)`, `oklch(..)`)
  - Hex literal (`0xRRGGBB`, `0xAARRGGBB`)
  - Any list of 3 or 4 numbers can be used as a color (e.g. `0.5, 0.5, 0.5` or `120, 120, 120, 255`)
- Hardware accelerated for maximum smoothness and high resolutions

**COLOR ACCURACY NOTE:** 
This application doesn't properly interpret display color gamuts wider than sRGB. E.g., if your display uses [Display P3](https://en.wikipedia.org/wiki/DCI-P3) (common with Apple devices) or [Adobe RGB](https://en.wikipedia.org/wiki/Adobe_RGB_color_space), this application shows colors more vibrantly than intended.

## Installation

Download from [Releases](https://github.com/eero-lehtinen/oklch-color-picker/releases).

If you have **cargo**, you can also install with:

```sh
cargo install oklch-color-picker --locked
```

Convert a color without opening the picker with `--convert-to`:

```sh
oklch-color-picker "oklch(62% 0.2 250)" --convert-to hex
```

The input format is auto-detected when possible. Use `--format` for ambiguous
raw values:

```sh
oklch-color-picker "0.62, 0.2, 250" --format raw_oklch --convert-to rgb
```

---

Check out the neovim plugin that this picker was made for [eero-lehtinen/oklch-color-picker.nvim](https://github.com/eero-lehtinen/oklch-color-picker.nvim).

Inspired by https://oklch.com/.


