# Oklch Color Picker

![picker](https://github.com/user-attachments/assets/e75bd890-2833-4c40-ab80-fee55ef21db3)

## Features
- Takes an input color from stdin and ouputs the edited color to stdout
- Uses a perceptual colorspace (Oklch) to allow intuitive editing
  - Consists of lightness, chroma and hue
  - Motivation: [An article by the Oklab creator](https://bottosson.github.io/posts/oklab/)
  - Oklch uses the same theory as Oklab, but uses parameters that are easier to understand
  - L<sub>r</sub> estimate is used instead of L as specified in [another article by the same guy](https://bottosson.github.io/posts/colorpicker/#intermission---a-new-lightness-estimate-for-oklab)
- Supports many color formats for input and output (editing uses only Oklch):
  - Hex (`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`)
  - Other common CSS formats (`rgb(..)`, `hsl(..)`, `oklch(..)`)
  - Any list of 3 or 4 numbers can be used as a color (e.g. `0.5, 0.5, 0.5` or `120, 120, 120, 255`)
- Hardware accelerated for maximum smoothness and high resolutions

## Installation

Download from [Releases](https://github.com/eero-lehtinen/oklch-color-picker/releases).

You can also clone the repo, run `cargo build -r` and copy the binary from `target/release/oklch-color-picker` to your PATH.

___
 
Check out the neovim plugin that this picker was made for [eero-lehtinen/oklch-color-picker.nvim](https://github.com/eero-lehtinen/oklch-color-picker.nvim).

Inspired by https://oklch.com/.
