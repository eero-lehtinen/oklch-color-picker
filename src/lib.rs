use bevy_color::Color;
use clap::ValueEnum;
use cli::CliColorFormat;
use mlua::prelude::*;

pub mod cli;
pub mod formats;
pub mod gamut;

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

fn color_to_hex(_: &Lua, (color, fmt): (String, Option<String>)) -> LuaResult<Option<String>> {
    let color = if let Some(fmt) = fmt {
        let parsed_fmt = CliColorFormat::from_str(&fmt, true).map_err(LuaError::RuntimeError)?;
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

    Ok(Some(formats::format_normalized_hex_no_alpha(color.into())))
}

#[mlua::lua_module(skip_memory_check)]
fn parser_lua_module(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("color_to_hex", lua.create_function(color_to_hex)?)?;
    exports.set("version", lua.create_function(version)?)?;
    Ok(exports)
}
