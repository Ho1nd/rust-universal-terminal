//! Нижняя статусная строка.

use egui::Ui;

use crate::app::TerminalApp;
use crate::ui::theme_colors::{resolved_colors, rgba_to_color32};

pub fn render(app: &mut TerminalApp, ui: &mut Ui) {
    let colors = resolved_colors(&app.config);
    ui.horizontal(|ui| {
        let connected = app.is_connected();
        let dot = if connected { "●" } else { "○" };
        let color = if connected {
            egui::Color32::from_rgb(0x30, 0xc0, 0x60)
        } else {
            egui::Color32::from_rgb(0x90, 0x90, 0x90)
        };
        ui.colored_label(color, dot);

        if connected {
            let desc = app
                .connection
                .description
                .lock()
                .clone()
                .unwrap_or_else(|| "?".into());
            ui.label(format!("Подключено ({desc})"));
        } else {
            ui.colored_label(rgba_to_color32(colors.info), "Не подключено");
        }
        ui.separator();
        let (rx_b, tx_b) = {
            let store = app.log_store.lock();
            (store.rx_bytes_total, store.tx_bytes_total)
        };
        ui.label(format!(
            "RX: {} / всего {}",
            format_rate(app.rx_throughput.bytes_per_sec()),
            format_size(rx_b)
        ));
        ui.separator();
        ui.label(format!(
            "TX: {} / всего {}",
            format_rate(app.tx_throughput.bytes_per_sec()),
            format_size(tx_b)
        ));
        ui.separator();
        if let Some(t) = app.connected_since {
            let secs = t.elapsed().as_secs();
            let h = secs / 3600;
            let m = (secs / 60) % 60;
            let s = secs % 60;
            ui.label(format!("Время работы {h:02}:{m:02}:{s:02}"));
        }
        ui.separator();
        if app.scheduler.running {
            ui.colored_label(
                egui::Color32::from_rgb(0xff, 0xc0, 0x20),
                "Планировщик запущен",
            );
        }
    });
}

fn format_rate(bps: f64) -> String {
    if bps >= 1_048_576.0 {
        format!("{:.2} MB/s", bps / 1_048_576.0)
    } else if bps >= 1024.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else {
        format!("{bps:.0} B/s")
    }
}

fn format_size(b: u64) -> String {
    if b >= 1_048_576 {
        format!("{:.2} MB", b as f64 / 1_048_576.0)
    } else if b >= 1024 {
        format!("{:.1} KB", b as f64 / 1024.0)
    } else {
        format!("{b} B")
    }
}
