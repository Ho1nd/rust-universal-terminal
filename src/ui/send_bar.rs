//! Нижняя строка: ввод, формат, кнопка, CR/LF, история.

use egui::{Key, Ui};

use crate::app::TerminalApp;
use crate::config::SendFormat;

pub fn render(app: &mut TerminalApp, ui: &mut Ui) {
    let mut want_send = false;
    let mut refocus = false;

    ui.horizontal(|ui| {
        ui.label("Отправить:");
        let text_edit = egui::TextEdit::singleline(&mut app.send_input)
            .hint_text("Введите данные и нажмите Enter…")
            .desired_width(f32::INFINITY);
        let resp = ui.add(text_edit);

        if resp.has_focus() {
            let (up, down) = ui.input(|i| {
                (i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown))
            });
            if up {
                history_prev(app);
            }
            if down {
                history_next(app);
            }
        }

        // Enter в однострочном TextEdit вызывает потерю фокуса — ловим именно так.
        let enter_pressed = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if enter_pressed {
            want_send = true;
            refocus = true;
        }

        super::send_format_combo(ui, &mut app.send_format);
        ui.checkbox(&mut app.add_newline, "CR/LF")
            .on_hover_text("Добавлять \\r\\n в конец отправляемых данных");
        ui.checkbox(&mut app.interpret_escapes, "\\n \\r \\t")
            .on_hover_text(
                "Обрабатывать escape-последовательности в ASCII:\n\
                 \\n → перевод строки (0x0A)\n\
                 \\r → возврат каретки (0x0D)\n\
                 \\t → табуляция (0x09)\n\
                 \\0 → нулевой байт\n\
                 \\\\ → обратная косая черта\n\
                 \\xHH → произвольный байт по HEX-коду",
            );
        ui.checkbox(&mut app.show_sent, "лог TX")
            .on_hover_text("Показывать отправленные данные в панели TX");
        if ui.button("Отправить").clicked() {
            want_send = true;
            refocus = true;
        }

        if refocus {
            resp.request_focus();
        }
    });

    if want_send {
        send_now(app);
    }
}

fn send_now(app: &mut TerminalApp) {
    if app.send_input.is_empty() {
        return;
    }
    if !app.is_connected() {
        app.show_toast("Подключение не установлено", true);
        return;
    }
    let text = app.send_input.clone();
    let fmt = app.send_format;
    let nl = app.add_newline;
    let escapes = app.interpret_escapes;
    app.send_text(&text, fmt, nl, escapes);
    app.push_history(text);
    app.history_index = None;
    app.history_draft.clear();
    // Текст намеренно не очищается — удобно отправлять повторно.
    app.mark_config_dirty();
}

fn history_prev(app: &mut TerminalApp) {
    if app.config.send_history.is_empty() {
        return;
    }
    let new_idx = match app.history_index {
        None => {
            app.history_draft = app.send_input.clone();
            app.config.send_history.len() - 1
        }
        Some(0) => 0,
        Some(i) => i - 1,
    };
    app.history_index = Some(new_idx);
    if let Some(s) = app.config.send_history.get(new_idx) {
        app.send_input = s.clone();
    }
}

fn history_next(app: &mut TerminalApp) {
    if app.config.send_history.is_empty() {
        return;
    }
    match app.history_index {
        None => {}
        Some(i) if i + 1 >= app.config.send_history.len() => {
            app.history_index = None;
            app.send_input = std::mem::take(&mut app.history_draft);
        }
        Some(i) => {
            app.history_index = Some(i + 1);
            if let Some(s) = app.config.send_history.get(i + 1) {
                app.send_input = s.clone();
            }
        }
    }
}

#[allow(dead_code)]
fn _noop(_f: SendFormat) {}
