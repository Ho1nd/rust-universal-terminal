//! Библиотечный корень крейта `rust_terminal`.
//!
//! Бинарь `rust-terminal` (`src/main.rs`) использует отсюда `run()`.

pub mod app;
pub mod buffer;
pub mod config;
pub mod connection;
pub mod formats;
pub mod highlight;
pub mod io_worker;
pub mod macros;
pub mod persistence;
pub mod scheduler;
pub mod search;
pub mod triggers;
pub mod ui;

use anyhow::Result;

use crate::app::TerminalApp;

/// Точка запуска. Вызывается из `main.rs`.
pub fn run() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init()
        .ok();

    let cfg = config::AppConfig::load_or_default();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([cfg.window.width.max(800.0), cfg.window.height.max(600.0)])
            .with_min_inner_size([800.0, 500.0])
            .with_title("Rust Terminal — UART/TCP/UDP"),
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "Rust Terminal",
        native_options,
        Box::new(|cc| Ok(Box::new(TerminalApp::new(cc, cfg)))),
    )
    .map_err(|e| anyhow::anyhow!("Ошибка eframe: {e}"))
}
