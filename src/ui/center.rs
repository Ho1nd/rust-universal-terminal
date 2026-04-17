//! Центральная область: split-view RX/TX или combined.

use egui::{Layout, Ui};

use crate::app::TerminalApp;
use crate::buffer::LogScope;
use crate::ui::terminal_view;

pub fn render(app: &mut TerminalApp, ui: &mut Ui) {
    if app.config.combined_view {
        render_single(app, ui, LogScope::Combined, "Объединённый (RX+TX)");
        return;
    }

    let total_width = ui.available_width();
    let split = app.config.split_ratio.clamp(0.1, 0.9);
    let left_w = (total_width * split).max(120.0);
    let right_w = (total_width - left_w - 6.0).max(120.0);

    let available_height = ui.available_height();
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, available_height),
            Layout::top_down(egui::Align::Min),
            |ui| {
                render_single(app, ui, LogScope::Rx, "RX");
            },
        );
        // Splitter
        let sep_rect = ui
            .allocate_exact_size(egui::vec2(6.0, available_height), egui::Sense::drag())
            .0;
        let sep_response =
            ui.interact(sep_rect, ui.id().with("splitter"), egui::Sense::drag());
        if sep_response.dragged() {
            let dx = sep_response.drag_delta().x;
            let new_ratio = (split + dx / total_width).clamp(0.15, 0.85);
            if (new_ratio - app.config.split_ratio).abs() > 0.001 {
                app.config.split_ratio = new_ratio;
                app.mark_config_dirty();
            }
        }
        if sep_response.hovered() {
            ui.ctx()
                .output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
        }
        ui.painter().line_segment(
            [
                egui::pos2(sep_rect.center().x, sep_rect.top() + 2.0),
                egui::pos2(sep_rect.center().x, sep_rect.bottom() - 2.0),
            ],
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        );
        ui.allocate_ui_with_layout(
            egui::vec2(right_w, available_height),
            Layout::top_down(egui::Align::Min),
            |ui| {
                render_single(app, ui, LogScope::Tx, "TX");
            },
        );
    });
}

fn render_single(app: &mut TerminalApp, ui: &mut Ui, scope: LogScope, title: &str) {
    let became_active = ui
        .interact(
            ui.available_rect_before_wrap(),
            egui::Id::new(("panel_focus", scope_id(scope))),
            egui::Sense::click(),
        )
        .clicked();
    if became_active {
        app.active_scope = scope;
    }
    header(app, ui, scope, title);
    terminal_view::render(app, ui, scope);
}

fn scope_id(scope: LogScope) -> &'static str {
    match scope {
        LogScope::Rx => "rx",
        LogScope::Tx => "tx",
        LogScope::Combined => "combined",
    }
}

fn header(app: &mut TerminalApp, ui: &mut Ui, scope: LogScope, title: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).strong());
        ui.separator();
        ui.label("Формат:");
        let mut fmt = app.default_format_for(scope);
        if super::format_combo(ui, &mut fmt) {
            app.set_format_for(scope, fmt);
            app.invalidate_format_cache();
        }
        ui.separator();
        let pause_label = if app.paused { "Продолжить" } else { "Пауза" };
        if ui
            .button(pause_label)
            .on_hover_text("Пауза/продолжение прокрутки")
            .clicked()
        {
            app.paused = !app.paused;
        }
        if ui.button("Очистить").clicked() {
            app.log_store.lock().clear(scope);
        }
        if ui.button("Сохранить").clicked() {
            let store = app.log_store.lock();
            let lines: Vec<_> = store.view(scope).iter().cloned().collect();
            drop(store);
            let path = rfd::FileDialog::new()
                .set_file_name(format!(
                    "{}.txt",
                    match scope {
                        LogScope::Rx => "rx_log",
                        LogScope::Tx => "tx_log",
                        LogScope::Combined => "combined_log",
                    }
                ))
                .add_filter("Текст", &["txt"])
                .add_filter("CSV", &["csv"])
                .save_file();
            if let Some(p) = path {
                let fmt = app.default_format_for(scope);
                let escape = app.config.display.escape_control_chars;
                let result = if p.extension().map(|s| s == "csv").unwrap_or(false) {
                    crate::persistence::save_log_csv(&p, &lines)
                } else {
                    crate::persistence::save_log_txt(&p, &lines, fmt, escape)
                };
                match result {
                    Ok(()) => app.show_toast(format!("Сохранено: {}", p.display()), false),
                    Err(e) => app.show_toast(format!("Ошибка: {e}"), true),
                }
            }
        }
    });
}
