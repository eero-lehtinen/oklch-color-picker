#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env,
    process::ExitCode,
    sync::{Arc, Mutex},
    time::Instant,
};

use bevy_color::Oklcha;
use clap::Parser as _;
use cli::{Cli, CliColorFormat};
use eframe::egui::Vec2;
use egui::ViewportBuilder;
use formats::{parse_color, parse_color_unknown_format};
use once_cell::sync::Lazy;
use rand::{rngs::SmallRng, Rng, SeedableRng};

mod app;
mod cli;
mod formats;
mod gamut;
mod gl_programs;

fn main() -> ExitCode {
    log_startup_init();

    let cli = Cli::parse();
    log_startup_time("Cli parse");

    let (color, format, use_alpha) = match (cli.color, cli.format) {
        (Some(color_string), Some(cli_format)) => {
            let Some((color, use_alpha)) = parse_color(&color_string, cli_format.into()) else {
                eprintln!(
                    "Invalid color '{}' for specified format '{}'",
                    color_string, cli_format
                );
                return ExitCode::FAILURE;
            };

            (color.into(), cli_format.into(), use_alpha)
        }
        (Some(color_string), None) => {
            let Some((color, format, use_alpha)) = parse_color_unknown_format(&color_string) else {
                eprintln!("Could not detect format for color '{}'", color_string);
                return ExitCode::FAILURE;
            };
            (color.into(), format, use_alpha)
        }
        (None, Some(cli_format)) => (random_color(), cli_format.into(), true),
        (None, None) => (random_color(), CliColorFormat::default().into(), true),
    };
    log_startup_time("Color parse");

    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: ViewportBuilder::default().with_min_inner_size(Vec2::new(500., 400.)),
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

static INTERVAL_TIMER: Lazy<Mutex<Instant>> = Lazy::new(|| Mutex::new(Instant::now()));
static STARTUP_TIMER: Lazy<Instant> = Lazy::new(Instant::now);
static STARTUP_TIMER_ENABLED: Lazy<bool> = Lazy::new(|| env::var("STARTUP_TIMER").is_ok());
fn log_startup_init() {
    if *STARTUP_TIMER_ENABLED {
        _ = STARTUP_TIMER.elapsed();
        *INTERVAL_TIMER.lock().unwrap() = Instant::now();
    }
}
fn log_startup_time(name: &str) {
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

fn random_color() -> Oklcha {
    let mut rng = SmallRng::from_entropy();
    Oklcha::new(
        rng.gen_range(0.4..0.8),
        rng.gen_range(0.05..0.2),
        rng.gen_range(0.0..360.),
        1.,
    )
}

fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    (1. - t) * v0 + t * v1
}

fn map(input: f32, from: (f32, f32), to: (f32, f32)) -> f32 {
    ((to.1 - to.0) * (input - from.0) / (from.1 - from.0) + to.0)
        .clamp(to.0.min(to.1), to.1.max(to.0))
}
