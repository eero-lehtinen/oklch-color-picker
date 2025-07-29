#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy_color::Oklcha;
use formats::ColorFormat;
use gamut::gamut_clip_preserve_chroma;
use rand::{Rng, SeedableRng, rngs::SmallRng};
#[cfg(not(target_arch = "wasm32"))]
use std::process::ExitCode;
use std::sync::Arc;

mod app;
mod cli;
mod formats;
mod gamut;
mod gl_programs;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> ExitCode {
    use clap::Parser as _;
    use cli::Cli;
    use egui::{Vec2, ViewportBuilder};
    use formats::{parse_color, parse_color_unknown_format};

    log_startup::init();

    let cli = Cli::parse();

    log_startup::log("Cli parse");

    let (color, format, use_alpha) = match (cli.color, cli.format) {
        (Some(color_string), Some(format)) => {
            let Some((color, use_alpha)) = parse_color(&color_string, format) else {
                eprintln!(
                    "Invalid color '{}' for specified format '{}'",
                    color_string, format
                );
                return ExitCode::FAILURE;
            };

            (color.into(), format, use_alpha)
        }
        (Some(color_string), None) => {
            let Some((color, format, use_alpha)) = parse_color_unknown_format(&color_string) else {
                eprintln!("Could not detect format for color '{}'", color_string);
                return ExitCode::FAILURE;
            };
            (color.into(), format, use_alpha)
        }
        (None, Some(format)) => (random_color(), format, true),
        (None, None) => (random_color(), ColorFormat::default(), true),
    };
    log_startup::log("Color parse");

    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: ViewportBuilder::default()
            .with_min_inner_size(Vec2::new(500., 400.))
            .with_icon(load_icon()),
        ..Default::default()
    };

    let data = Arc::new((color, format, use_alpha));

    eframe::run_native(
        "Oklch Color Picker",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc, data)))),
    )
    .unwrap();

    ExitCode::SUCCESS
}

#[cfg(not(target_arch = "wasm32"))]
fn load_icon() -> egui::IconData {
    let icon = include_bytes!("../assets/icon.png");
    let image = image::load_from_memory(icon).unwrap().into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    egui::IconData {
        rgba,
        width,
        height,
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let data = Arc::new((random_color(), ColorFormat::default(), true));

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(app::App::new(cc, data)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
mod log_startup {
    use std::{env, sync::Mutex, time::Instant};

    use once_cell::sync::Lazy;

    static INTERVAL_TIMER: Lazy<Mutex<Instant>> = Lazy::new(|| Mutex::new(Instant::now()));
    static STARTUP_TIMER: Lazy<Instant> = Lazy::new(Instant::now);
    static STARTUP_TIMER_ENABLED: Lazy<bool> = Lazy::new(|| env::var("STARTUP_TIMER").is_ok());
    pub fn init() {
        if *STARTUP_TIMER_ENABLED {
            _ = STARTUP_TIMER.elapsed();
            *INTERVAL_TIMER.lock().unwrap() = Instant::now();
        }
    }
    pub fn log(name: &str) {
        if *STARTUP_TIMER_ENABLED {
            let mut timer = INTERVAL_TIMER.lock().unwrap();
            println!(
                "{:<20}: {:>10.5}ms delta, {:>10.5}ms total",
                name,
                timer.elapsed().as_secs_f64() * 1000.,
                STARTUP_TIMER.elapsed().as_secs_f64() * 1000.,
            );
            *timer = Instant::now();
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod log_startup {
    pub fn log(_: &str) {}
}

fn random_color() -> Oklcha {
    let mut rng = SmallRng::from_os_rng();
    let color = Oklcha::new(
        rng.random_range(0.4..0.8),
        rng.random_range(0.05..0.2),
        rng.random_range(0.0..360.),
        1.,
    );
    gamut_clip_preserve_chroma(color.into()).into()
}

fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    (1. - t) * v0 + t * v1
}

fn map(input: f32, from: (f32, f32), to: (f32, f32)) -> f32 {
    ((to.1 - to.0) * (input - from.0) / (from.1 - from.0) + to.0)
        .clamp(to.0.min(to.1), to.1.max(to.0))
}
