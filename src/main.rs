//! Тонкая обёртка: делегирует в `rust_terminal::run`.

#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

fn main() -> anyhow::Result<()> {
    rust_terminal::run()
}
