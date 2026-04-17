//! Оверлей поиска (Ctrl+F).

use std::time::{Duration, Instant};

use egui::{Key, Ui};

use crate::app::TerminalApp;
use crate::search::SearchScope;

pub fn render(app: &mut TerminalApp, ui: &mut Ui) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Поиск:");
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut app.search.query)
                        .desired_width(220.0)
                        .hint_text("текст или регулярное выражение…"),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    let dir_back = ui.input(|i| i.modifiers.shift);
                    recompute_and_navigate(app, !dir_back);
                    resp.request_focus();
                }

                ui.toggle_value(&mut app.search.case_sensitive, "Aa")
                    .on_hover_text("Учитывать регистр");
                ui.toggle_value(&mut app.search.regex, ".*")
                    .on_hover_text("Регулярное выражение");

                egui::ComboBox::from_id_salt("search_scope")
                    .selected_text(match app.search.scope {
                        SearchScope::Rx => "RX",
                        SearchScope::Tx => "TX",
                        SearchScope::Combined => "Объединённый",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.search.scope, SearchScope::Rx, "RX");
                        ui.selectable_value(&mut app.search.scope, SearchScope::Tx, "TX");
                        ui.selectable_value(&mut app.search.scope, SearchScope::Combined, "Объединённый");
                    });

                if ui.small_button("<").on_hover_text("Назад (Shift+Enter)").clicked() {
                    recompute_and_navigate(app, false);
                }
                if ui.small_button(">").on_hover_text("Далее (Enter)").clicked() {
                    recompute_and_navigate(app, true);
                }

                let count = app.search.matches.len();
                let cur = app.search.current.map(|i| i + 1).unwrap_or(0);
                ui.label(format!("{cur}/{count}"));

                if ui.small_button("×").on_hover_text("Закрыть").clicked() {
                    app.search.open = false;
                }
            });

            if !app.search.query.is_empty() {
                let scope = app.search.scope.to_log_scope();
                let revision;
                let lines_snapshot: Vec<_> = {
                    let store = app.log_store.lock();
                    revision = store.revision;
                    store.view(scope).iter().cloned().collect()
                };
                let mut vec_lines = std::collections::VecDeque::from(lines_snapshot);
                let fmt = app.default_format_for(scope);
                let escape = app.config.display.escape_control_chars;
                app.search.recompute(&vec_lines, fmt, escape, revision);
                vec_lines.clear();
            }
        });
}

fn recompute_and_navigate(app: &mut TerminalApp, forward: bool) {
    let scope = app.search.scope.to_log_scope();
    let revision;
    let lines: std::collections::VecDeque<_> = {
        let store = app.log_store.lock();
        revision = store.revision;
        store.view(scope).iter().cloned().collect()
    };
    let fmt = app.default_format_for(scope);
    let escape = app.config.display.escape_control_chars;
    app.search.recompute(&lines, fmt, escape, revision);

    let target = if forward {
        app.search.go_next()
    } else {
        app.search.go_prev()
    };
    if let Some(idx) = target {
        app.flash_line = Some((scope, idx, Instant::now() + Duration::from_secs(2)));
        app.scroll_request = Some((scope, idx));
        // При навигации по поиску переключаемся на ту панель, в которой ищем,
        // и ставим на паузу автоскролл, чтобы найденная строка не уезжала вниз.
        app.active_scope = scope;
        app.paused = true;
    }
}
