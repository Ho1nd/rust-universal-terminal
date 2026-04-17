//! Корневой UI-рендер. Разбит на поддиректории.

pub mod center;
pub mod left_panel;
pub mod scheduler_panel;
pub mod search_bar;
pub mod send_bar;
pub mod statusbar;
pub mod terminal_view;
pub mod theme_colors;
pub mod strings;

use egui::{Context, Key, Modifiers, TopBottomPanel};

use crate::app::TerminalApp;
use crate::buffer::LogScope;
use crate::config::SendFormat;
use crate::formats;
use crate::macros as macros_mod;
use crate::search::SearchScope;

pub fn render_app(app: &mut TerminalApp, ctx: &Context) {
    handle_global_shortcuts(app, ctx);

    TopBottomPanel::top("top_menu").show(ctx, |ui| {
        top_menu(app, ui, ctx);
    });

    if app.config.left_panel_visible {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(app.config.left_panel_width)
            .width_range(240.0..=500.0)
            .show(ctx, |ui| {
                let w = ui.available_width();
                if (w - app.config.left_panel_width).abs() > 1.0 {
                    app.config.left_panel_width = w;
                    app.mark_config_dirty();
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    left_panel::render(app, ui);
                });
            });
    }

    TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
        statusbar::render(app, ui);
    });

    TopBottomPanel::bottom("send_bar")
        .resizable(false)
        .show(ctx, |ui| {
            send_bar::render(app, ui);
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        if app.search.open {
            search_bar::render(app, ui);
        }
        center::render(app, ui);

        if let Some(t) = &app.toast {
            let color = if t.is_error {
                egui::Color32::from_rgb(0xf4, 0x47, 0x47)
            } else {
                egui::Color32::from_rgb(0x30, 0xc0, 0x60)
            };
            let text = t.text.clone();
            egui::Area::new(egui::Id::new("toast"))
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -12.0))
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .fill(color)
                        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                        .show(ui, |ui| {
                            ui.colored_label(egui::Color32::WHITE, text);
                        });
                });
        }
    });
}

fn top_menu(app: &mut TerminalApp, ui: &mut egui::Ui, ctx: &Context) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button(strings::MENU_FILE, |ui| {
            if ui.button(strings::SAVE_RX_LOG).clicked() {
                ui.close_menu();
                save_log(app, LogScope::Rx);
            }
            if ui.button(strings::SAVE_TX_LOG).clicked() {
                ui.close_menu();
                save_log(app, LogScope::Tx);
            }
            if ui.button(strings::SAVE_COMBINED_LOG).clicked() {
                ui.close_menu();
                save_log(app, LogScope::Combined);
            }
            ui.separator();
            let cur = app
                .config
                .continuous_log_file
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "выключена".into());
            ui.label(format!("Непрерывная запись: {cur}"));
            if ui.button(strings::CONTINUOUS_LOG_SET).clicked() {
                ui.close_menu();
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Файлы логов", &["log", "txt"])
                    .save_file()
                {
                    app.set_continuous_log(Some(path));
                }
            }
            if ui.button(strings::CONTINUOUS_LOG_OFF).clicked() {
                ui.close_menu();
                app.set_continuous_log(None);
            }
            ui.separator();
            if ui.button(strings::SAVE_CONFIG).clicked() {
                ui.close_menu();
                app.config.save();
                app.show_toast("Конфигурация сохранена", false);
            }
            if ui.button(strings::EXIT).clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button(strings::MENU_EDIT, |ui| {
            if ui.button(strings::FIND).clicked() {
                ui.close_menu();
                app.search.open = true;
            }
            ui.separator();
            if ui.button(strings::CLEAR_RX).clicked() {
                app.log_store.lock().clear(LogScope::Rx);
                ui.close_menu();
            }
            if ui.button(strings::CLEAR_TX).clicked() {
                app.log_store.lock().clear(LogScope::Tx);
                ui.close_menu();
            }
            if ui.button(strings::CLEAR_ALL).clicked() {
                app.log_store.lock().clear(LogScope::Combined);
                ui.close_menu();
            }
        });
        ui.menu_button(strings::MENU_VIEW, |ui| {
            ui.menu_button(strings::THEME, |ui| {
                for c in crate::config::ThemeChoice::ALL {
                    let selected = app.config.theme == c;
                    if ui.selectable_label(selected, c.label()).clicked() {
                        app.config.theme = c;
                        theme_colors::apply_theme(ctx, &app.config);
                        app.mark_config_dirty();
                        ui.close_menu();
                    }
                }
            });
            if ui
                .checkbox(&mut app.config.left_panel_visible, strings::SHOW_LEFT_PANEL)
                .changed()
            {
                app.mark_config_dirty();
            }
            if ui
                .checkbox(&mut app.config.combined_view, strings::COMBINED_VIEW)
                .changed()
            {
                app.mark_config_dirty();
            }
            if ui
                .checkbox(&mut app.config.autoscroll, strings::AUTOSCROLL)
                .changed()
            {
                app.mark_config_dirty();
            }
            if ui
                .checkbox(&mut app.config.show_timestamps, strings::SHOW_TIMESTAMPS)
                .changed()
            {
                app.mark_config_dirty();
            }
            if ui
                .checkbox(
                    &mut app.config.show_direction_arrows,
                    strings::SHOW_ARROWS,
                )
                .changed()
            {
                app.mark_config_dirty();
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(strings::FONT_SIZE);
                let mut size = app.config.font_size;
                if ui.add(egui::DragValue::new(&mut size).range(9.0..=28.0)).changed() {
                    app.config.font_size = size;
                    theme_colors::apply_font(ctx, &app.config);
                    app.mark_config_dirty();
                }
            });
        });
        ui.menu_button(strings::MENU_TOOLS, |ui| {
            ui.menu_button(strings::PRESETS, |ui| {
                render_presets_menu(app, ui);
            });
        });
        ui.menu_button(strings::MENU_HELP, |ui| {
            if ui.button(strings::SHORTCUTS).clicked() {
                ui.close_menu();
                app.show_toast(
                    "Ctrl+F: поиск; Ctrl+L: очистить; Ctrl+S: сохранить лог; Ctrl+Enter: отправить; F1–F12: макросы; Ctrl+B: левая панель; Ctrl+M: объединённый вид",
                    false,
                );
            }
            if ui.button(strings::ABOUT).clicked() {
                ui.close_menu();
                app.show_toast(
                    format!("Rust-Terminal v{}", env!("CARGO_PKG_VERSION")),
                    false,
                );
            }
        });
    });
}

fn render_presets_menu(app: &mut TerminalApp, ui: &mut egui::Ui) {
    if app.config.presets.is_empty() {
        ui.label("(нет пресетов)");
    } else {
        let mut to_apply: Option<usize> = None;
        let mut to_delete: Option<usize> = None;
        for (i, p) in app.config.presets.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui.button(&p.name).clicked() {
                    to_apply = Some(i);
                }
                if ui.small_button("×").on_hover_text("Удалить").clicked() {
                    to_delete = Some(i);
                }
            });
        }
        if let Some(i) = to_apply {
            app.config.last_connection = app.config.presets[i].settings.clone();
            app.mark_config_dirty();
        }
        if let Some(i) = to_delete {
            app.config.presets.remove(i);
            app.mark_config_dirty();
        }
    }
    ui.separator();
    if ui.button("Сохранить как пресет…").clicked() {
        let name = default_preset_name(&app.config.last_connection);
        app.config.presets.push(crate::config::ConnectionPreset {
            name,
            settings: app.config.last_connection.clone(),
        });
        app.mark_config_dirty();
    }
}

fn default_preset_name(s: &crate::config::ConnectionSettings) -> String {
    match s.kind {
        crate::config::ConnectionType::Uart => {
            format!("{} @ {}", s.uart.port_name, s.uart.baud)
        }
        crate::config::ConnectionType::TcpClient => {
            format!("TCP {}:{}", s.net.host, s.net.port)
        }
        crate::config::ConnectionType::Udp => {
            format!("UDP {}:{}", s.net.host, s.net.port)
        }
    }
}

fn save_log(app: &mut TerminalApp, scope: LogScope) {
    let store = app.log_store.lock();
    let lines: Vec<_> = store.view(scope).iter().cloned().collect();
    drop(store);
    let default_name = match scope {
        LogScope::Rx => "rx_log",
        LogScope::Tx => "tx_log",
        LogScope::Combined => "combined_log",
    };
    let path = rfd::FileDialog::new()
        .set_file_name(format!("{}.txt", default_name))
        .add_filter("Текст", &["txt"])
        .add_filter("CSV", &["csv"])
        .save_file();
    let Some(path) = path else { return; };
    let fmt = app.default_format_for(scope);
    let escape = app.config.display.escape_control_chars;
    let result = if path.extension().map(|s| s == "csv").unwrap_or(false) {
        crate::persistence::save_log_csv(&path, &lines)
    } else {
        crate::persistence::save_log_txt(&path, &lines, fmt, escape)
    };
    match result {
        Ok(()) => app.show_toast(format!("Лог сохранён: {}", path.display()), false),
        Err(e) => app.show_toast(format!("Не удалось сохранить: {e}"), true),
    }
}

fn handle_global_shortcuts(app: &mut TerminalApp, ctx: &Context) {
    let input = ctx.input(|i| i.clone());
    let wants_text = ctx.wants_keyboard_input();

    if input.modifiers.command_only() && input.key_pressed(Key::F) {
        app.search.open = !app.search.open;
    }
    if input.modifiers.command_only() && input.key_pressed(Key::B) {
        app.config.left_panel_visible = !app.config.left_panel_visible;
        app.mark_config_dirty();
    }
    if input.modifiers.command_only() && input.key_pressed(Key::M) {
        app.config.combined_view = !app.config.combined_view;
        app.mark_config_dirty();
    }
    if input.modifiers.command_only() && input.key_pressed(Key::L) {
        let scope = app.active_scope;
        app.log_store.lock().clear(scope);
    }
    if input.modifiers.command_only() && input.key_pressed(Key::S) {
        let scope = app.active_scope;
        save_log(app, scope);
    }
    if input.key_pressed(Key::Escape) && app.search.open {
        app.search.open = false;
    }

    if !wants_text {
        for key in [
            Key::F1, Key::F2, Key::F3, Key::F4, Key::F5, Key::F6,
            Key::F7, Key::F8, Key::F9, Key::F10, Key::F11, Key::F12,
        ] {
            if input.key_pressed(key) {
                let Some(f) = macros_mod::f_key_from_egui(key) else { continue; };
                let m: Modifiers = input.modifiers;
                let bindings = app.config.macros.clone();
                if let Some(b) = crate::macros::find_macro(
                    &bindings,
                    f,
                    m.ctrl || m.command,
                    m.shift,
                    m.alt,
                ) {
                    match formats::parse_payload(&b.payload, b.format, b.add_newline) {
                        Ok(data) => {
                            if !app.is_connected() {
                                app.show_toast("Подключение не установлено", true);
                            } else {
                                app.send_raw(data);
                            }
                        }
                        Err(e) => {
                            app.show_toast(format!("Макрос {}: {e}", b.key.describe()), true);
                        }
                    }
                }
            }
        }
    }
}

// ——— переиспользуемое: переключатель формата ———

pub fn format_combo(ui: &mut egui::Ui, current: &mut crate::config::DisplayFormat) -> bool {
    let prev = *current;
    egui::ComboBox::from_id_salt(ui.next_auto_id())
        .selected_text(current.label())
        .show_ui(ui, |ui| {
            for f in crate::config::DisplayFormat::ALL {
                ui.selectable_value(current, f, f.label());
            }
        });
    prev != *current
}

pub fn send_format_combo(ui: &mut egui::Ui, current: &mut SendFormat) -> bool {
    let prev = *current;
    egui::ComboBox::from_id_salt(ui.next_auto_id())
        .selected_text(current.label())
        .show_ui(ui, |ui| {
            for f in SendFormat::ALL {
                ui.selectable_value(current, f, f.label());
            }
        });
    prev != *current
}

pub fn search_scope_to_log_scope(s: SearchScope) -> LogScope {
    s.to_log_scope()
}
