mod cli;
mod formats;
mod gamut;

#[cfg(not(target_arch = "wasm32"))]
mod lua {
    use super::*;
    use bevy_color::{Color, ColorToPacked, Srgba};
    use clap::ValueEnum;
    use cli::CliColorFormat;
    use mlua::prelude::*;

    fn gamut_clip(color: Color) -> Color {
        if let Color::Oklcha(color) = color {
            gamut::gamut_clip_preserve_chroma(color.into()).into()
        } else {
            color
        }
    }

    fn version(_: &Lua, _: ()) -> LuaResult<&'static str> {
        Ok(env!("CARGO_PKG_VERSION"))
    }

    fn parse(_: &Lua, (color, fmt): (String, Option<String>)) -> LuaResult<Option<u32>> {
        let color = if let Some(fmt) = fmt {
            let parsed_fmt =
                CliColorFormat::from_str(&fmt, true).map_err(LuaError::RuntimeError)?;
            match formats::parse_color(&color, parsed_fmt.into()) {
                Some((c, _)) => c,
                None => return Ok(None),
            }
        } else {
            match formats::parse_color_unknown_format(&color) {
                Some((c, _, _)) => c,
                None => return Ok(None),
            }
        };

        let color = gamut_clip(color);

        let srgb = Srgba::from(color);
        let [r, g, b] = srgb.to_u8_array_no_alpha();

        Ok(Some((r as u32) << 16 | (g as u32) << 8 | b as u32))
    }

    #[mlua::lua_module(skip_memory_check)]
    fn parser_lua_module(lua: &Lua) -> LuaResult<LuaTable> {
        let exports = lua.create_table()?;
        exports.set("parse", lua.create_function(parse)?)?;
        exports.set("version", lua.create_function(version)?)?;
        Ok(exports)
    }
}
