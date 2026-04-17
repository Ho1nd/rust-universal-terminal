//! Левая панель: подключение, отображение, планировщик, макросы, триггеры, подсветка.

use egui::{CollapsingHeader, ComboBox, RichText, Ui};

use crate::app::TerminalApp;
use crate::config::{
    ConnectionType, DisplayMode, FlowControl, HighlightRule, MacroBinding, Parity,
    SendFormat, StopBits, TriggerRule,
};
use crate::ui::scheduler_panel;
use crate::ui::strings;
use crate::ui::theme_colors::rgba_to_color32;

pub fn render(app: &mut TerminalApp, ui: &mut Ui) {
    ui.add_space(4.0);

    // ——— Пресеты ———
    ui.horizontal(|ui| {
        ui.label("Пресет:");
        let mut current_label = "—".to_string();
        if !app.config.presets.is_empty() {
            current_label = app.config.presets[0].name.clone();
        }
        ComboBox::from_id_salt("preset_combo")
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                let mut to_apply: Option<usize> = None;
                for (i, p) in app.config.presets.iter().enumerate() {
                    if ui.selectable_label(false, &p.name).clicked() {
                        to_apply = Some(i);
                    }
                }
                if let Some(i) = to_apply {
                    app.config.last_connection = app.config.presets[i].settings.clone();
                    app.mark_config_dirty();
                }
            });
        if ui.small_button("+").on_hover_text("Сохранить как пресет").clicked() {
            let name = super::default_preset_name(&app.config.last_connection);
            app.config.presets.push(crate::config::ConnectionPreset {
                name,
                settings: app.config.last_connection.clone(),
            });
            app.mark_config_dirty();
        }
    });

    ui.separator();

    // ——— Подключение ———
    CollapsingHeader::new(strings::CONNECTION)
        .default_open(true)
        .show(ui, |ui| {
            render_connection(app, ui);
        });

    // ——— Отображение ———
    CollapsingHeader::new(strings::DISPLAY)
        .default_open(true)
        .show(ui, |ui| {
            render_display(app, ui);
        });

    // ——— Планировщик ———
    CollapsingHeader::new(strings::SCHEDULER)
        .default_open(false)
        .show(ui, |ui| {
            scheduler_panel::render(app, ui);
        });

    // ——— Макросы ———
    CollapsingHeader::new(strings::MACROS)
        .default_open(false)
        .show(ui, |ui| {
            render_macros(app, ui);
        });

    // ——— Триггеры ———
    CollapsingHeader::new(strings::TRIGGERS)
        .default_open(false)
        .show(ui, |ui| {
            render_triggers(app, ui);
        });

    // ——— Подсветка ———
    CollapsingHeader::new(strings::HIGHLIGHTS)
        .default_open(false)
        .show(ui, |ui| {
            render_highlights(app, ui);
        });

    ui.add_space(16.0);
}

fn render_connection(app: &mut TerminalApp, ui: &mut Ui) {
    let mut changed = false;

    ui.horizontal(|ui| {
        for ct in ConnectionType::ALL {
            if ui
                .selectable_label(app.config.last_connection.kind == ct, ct.label())
                .clicked()
            {
                app.config.last_connection.kind = ct;
                changed = true;
            }
        }
    });

    match app.config.last_connection.kind {
        ConnectionType::Uart => {
            ui.horizontal(|ui| {
                ui.label("Порт:");
                let mut cur = app.config.last_connection.uart.port_name.clone();
                ComboBox::from_id_salt("com_port_combo")
                    .selected_text(if cur.is_empty() { "—".into() } else { cur.clone() })
                    .show_ui(ui, |ui| {
                        for p in &app.com_ports {
                            if ui.selectable_label(&cur == p, p).clicked() {
                                cur = p.clone();
                            }
                        }
                    });
                if cur != app.config.last_connection.uart.port_name {
                    app.config.last_connection.uart.port_name = cur;
                    changed = true;
                }
                if ui.small_button("↻").on_hover_text("Обновить список портов").clicked() {
                    app.refresh_ports();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Скорость:");
                let mut baud = app.config.last_connection.uart.baud;
                let presets = [
                    300, 600, 1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400,
                    460800, 921600,
                ];
                ComboBox::from_id_salt("baud_combo")
                    .selected_text(baud.to_string())
                    .show_ui(ui, |ui| {
                        for p in presets {
                            ui.selectable_value(&mut baud, p, p.to_string());
                        }
                    });
                if ui
                    .add(egui::DragValue::new(&mut baud).range(300..=4_000_000))
                    .changed()
                {
                    // no-op, изменение попадёт в проверку ниже
                }
                if baud != app.config.last_connection.uart.baud {
                    app.config.last_connection.uart.baud = baud;
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Биты данных:");
                let mut db = app.config.last_connection.uart.data_bits;
                ComboBox::from_id_salt("data_bits")
                    .selected_text(db.to_string())
                    .show_ui(ui, |ui| {
                        for b in [5u8, 6, 7, 8] {
                            ui.selectable_value(&mut db, b, b.to_string());
                        }
                    });
                if db != app.config.last_connection.uart.data_bits {
                    app.config.last_connection.uart.data_bits = db;
                    changed = true;
                }

                ui.label("Стоп:");
                let mut sb = app.config.last_connection.uart.stop_bits;
                ComboBox::from_id_salt("stop_bits")
                    .selected_text(sb.label())
                    .show_ui(ui, |ui| {
                        for v in StopBits::ALL {
                            ui.selectable_value(&mut sb, v, v.label());
                        }
                    });
                if sb != app.config.last_connection.uart.stop_bits {
                    app.config.last_connection.uart.stop_bits = sb;
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Чётность:");
                let mut par = app.config.last_connection.uart.parity;
                ComboBox::from_id_salt("parity")
                    .selected_text(par.label())
                    .show_ui(ui, |ui| {
                        for v in Parity::ALL {
                            ui.selectable_value(&mut par, v, v.label());
                        }
                    });
                if par != app.config.last_connection.uart.parity {
                    app.config.last_connection.uart.parity = par;
                    changed = true;
                }

                ui.label("Поток:");
                let mut fc = app.config.last_connection.uart.flow;
                ComboBox::from_id_salt("flow")
                    .selected_text(fc.label())
                    .show_ui(ui, |ui| {
                        for v in FlowControl::ALL {
                            ui.selectable_value(&mut fc, v, v.label());
                        }
                    });
                if fc != app.config.last_connection.uart.flow {
                    app.config.last_connection.uart.flow = fc;
                    changed = true;
                }
            });
        }
        ConnectionType::TcpClient | ConnectionType::Udp => {
            ui.horizontal(|ui| {
                ui.label("Хост:");
                if ui
                    .text_edit_singleline(&mut app.config.last_connection.net.host)
                    .changed()
                {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Порт:");
                if ui
                    .add(egui::DragValue::new(&mut app.config.last_connection.net.port).range(1..=65535))
                    .changed()
                {
                    changed = true;
                }
            });
            if app.config.last_connection.kind == ConnectionType::Udp {
                ui.horizontal(|ui| {
                    if ui
                        .checkbox(
                            &mut app.config.last_connection.net.udp_bind_local,
                            "Слушать локальный порт",
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    if app.config.last_connection.net.udp_bind_local
                        && ui
                            .add(
                                egui::DragValue::new(
                                    &mut app.config.last_connection.net.udp_local_port,
                                )
                                .range(1..=65535),
                            )
                            .changed()
                    {
                        changed = true;
                    }
                });
            }
        }
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let connected = app.is_connected();
        let btn_label = if connected { "Отключиться" } else { "Подключиться" };
        let btn = egui::Button::new(RichText::new(btn_label).strong());
        if ui.add(btn).clicked() {
            if connected {
                app.disconnect();
            } else {
                app.try_connect();
            }
        }
        let dot = if connected { "●" } else { "○" };
        let color = if connected {
            egui::Color32::from_rgb(0x30, 0xc0, 0x60)
        } else {
            egui::Color32::from_rgb(0xaa, 0xaa, 0xaa)
        };
        ui.colored_label(color, dot);
    });

    if changed {
        app.mark_config_dirty();
    }
}

fn render_display(app: &mut TerminalApp, ui: &mut Ui) {
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label("Сборка строк:");
        for (m, label, hover) in [
            (
                DisplayMode::ByNewline,
                "По \\n / \\r / \\r\\n",
                "Разделять входящий поток по переводам строки (Unix, Windows, старый macOS)",
            ),
            (
                DisplayMode::ByTimer,
                "По таймеру",
                "Эмитить одну строку каждые timer_interval_ms",
            ),
            (
                DisplayMode::Raw,
                "Сырой",
                "Каждый пришедший чанк — отдельная строка",
            ),
        ] {
            if ui
                .selectable_label(app.config.display.mode == m, label)
                .on_hover_text(hover)
                .clicked()
            {
                app.config.display.mode = m;
                changed = true;
            }
        }
    });

    match app.config.display.mode {
        DisplayMode::ByNewline => {
            ui.horizontal(|ui| {
                ui.label("Пауза сброса, мс:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut app.config.display.idle_flush_ms)
                            .range(10..=5000),
                    )
                    .changed();
                ui.label("Макс. длина строки:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut app.config.display.max_line_bytes)
                            .range(64..=65536),
                    )
                    .changed();
            });
            changed |= ui
                .checkbox(
                    &mut app.config.display.split_on_literal_escapes,
                    "Резать и по литеральным \\n / \\r / \\r\\n",
                )
                .on_hover_text(
                    "Если удалённая сторона шлёт строки в текстовом виде\n\
                     (как в JSON-логах: два байта — обратный слэш и 'n'),\n\
                     считать такие последовательности разделителями строк\n\
                     наравне с реальными 0x0A / 0x0D.",
                )
                .changed();
        }
        DisplayMode::ByTimer => {
            ui.horizontal(|ui| {
                ui.label("Интервал, мс:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut app.config.display.timer_interval_ms)
                            .range(10..=2000),
                    )
                    .changed();
            });
        }
        DisplayMode::Raw => {}
    }

    changed |= ui
        .checkbox(
            &mut app.config.display.escape_control_chars,
            "Экранировать управляющие символы",
        )
        .changed();
    changed |= ui.checkbox(&mut app.config.autoscroll, "Автопрокрутка").changed();
    changed |= ui
        .checkbox(&mut app.config.show_timestamps, "Таймстемпы")
        .changed();
    changed |= ui
        .checkbox(&mut app.config.show_direction_arrows, "Стрелки ←/→")
        .changed();

    ui.horizontal(|ui| {
        ui.label("Макс. строк в буфере:");
        let mut n = app.config.max_log_lines;
        if ui
            .add(egui::DragValue::new(&mut n).range(1000..=2_000_000))
            .changed()
        {
            app.config.max_log_lines = n;
            app.log_store.lock().set_max_lines(n);
            changed = true;
        }
    });

    if changed {
        app.mark_config_dirty();
    }
}

fn render_macros(app: &mut TerminalApp, ui: &mut Ui) {
    let mut to_delete: Option<usize> = None;
    let mut changed = false;
    for (i, m) in app.config.macros.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(m.key.describe()).strong());
                ui.text_edit_singleline(&mut m.description);
                if ui.small_button("×").on_hover_text("Удалить").clicked() {
                    to_delete = Some(i);
                }
            });
            ui.horizontal(|ui| {
                ui.label("F#:");
                changed |= ui
                    .add(egui::DragValue::new(&mut m.key.f_number).range(1..=12))
                    .changed();
                changed |= ui.checkbox(&mut m.key.ctrl, "Ctrl").changed();
                changed |= ui.checkbox(&mut m.key.shift, "Shift").changed();
                changed |= ui.checkbox(&mut m.key.alt, "Alt").changed();
            });
            ui.horizontal(|ui| {
                ui.label("Формат:");
                changed |= super::send_format_combo(ui, &mut m.format);
                changed |= ui.checkbox(&mut m.add_newline, "CR/LF").changed();
            });
            changed |= ui.text_edit_singleline(&mut m.payload).changed();
        });
    }
    if let Some(i) = to_delete {
        app.config.macros.remove(i);
        changed = true;
    }
    if ui.button("+ Добавить макрос").clicked() {
        app.config.macros.push(MacroBinding::default());
        changed = true;
    }
    if changed {
        app.mark_config_dirty();
    }
}

fn render_triggers(app: &mut TerminalApp, ui: &mut Ui) {
    let mut changed = false;
    let mut to_delete: Option<usize> = None;

    ui.label(
        egui::RichText::new(
            "Триггер срабатывает на принятой (RX) строке: при совпадении regex \
             отправляется заданный ответ. Если нужна проверка без физического \
             loopback — включи «TX тоже», и триггер будет ловить свои же \
             отправленные сообщения.",
        )
        .weak()
        .small(),
    );

    let triggers_len = app.config.triggers.len();
    for i in 0..triggers_len {
        let fire_count = app.triggers.fire_count(i);
        let last_fire = app.triggers.last_fire_instant(i);
        let pattern_snapshot = app.config.triggers[i].pattern.clone();
        let regex_err = app.triggers.regex_error(&pattern_snapshot);
        let test_id = ui.id().with(("trigger_test", i));
        let mut sample = ui
            .memory_mut(|m| m.data.get_persisted::<String>(test_id))
            .unwrap_or_default();
        let test_match_ok = if !sample.is_empty() && !pattern_snapshot.is_empty() {
            Some(app.triggers.test_match(&pattern_snapshot, &sample))
        } else {
            None
        };

        let t = &mut app.config.triggers[i];
        ui.group(|ui| {
            ui.horizontal(|ui| {
                changed |= ui.checkbox(&mut t.enabled, "Вкл.").changed();
                changed |= ui.text_edit_singleline(&mut t.name).changed();
                if ui.small_button("×").on_hover_text("Удалить").clicked() {
                    to_delete = Some(i);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Regex:");
                if ui.text_edit_singleline(&mut t.pattern).changed() {
                    changed = true;
                }
                if let Some(err) = &regex_err {
                    ui.colored_label(
                        egui::Color32::from_rgb(0xf4, 0x47, 0x47),
                        format!("Ошибка: {err}"),
                    );
                }
            });
            ui.horizontal(|ui| {
                ui.label("Ответ:");
                changed |= super::send_format_combo(ui, &mut t.response_format);
                changed |= ui.checkbox(&mut t.add_newline, "CR/LF").changed();
            });
            changed |= ui
                .text_edit_singleline(&mut t.response)
                .on_hover_text("Что отправить обратно при совпадении (поддерживаются \\n, \\r, \\xHH)")
                .changed();
            ui.horizontal(|ui| {
                ui.label("Задержка, мс:");
                changed |= ui
                    .add(egui::DragValue::new(&mut t.cooldown_ms).range(0..=600_000))
                    .on_hover_text("Минимальный интервал между повторными срабатываниями")
                    .changed();
                changed |= ui
                    .checkbox(&mut t.apply_to_tx, "TX тоже")
                    .on_hover_text(
                        "Срабатывать также на отправленные (TX) строки.\n\
                         Полезно для тестов без физической петли RX↔TX:\n\
                         отправил сам «123» → сработал триггер → ушло «1000».",
                    )
                    .changed();
            });
            let last_txt = match last_fire {
                Some(t) => format!("{:.1} с назад", t.elapsed().as_secs_f32()),
                None => "—".to_string(),
            };
            ui.label(
                egui::RichText::new(format!(
                    "Срабатываний: {fire_count}    Последнее: {last_txt}"
                ))
                .small()
                .weak(),
            );
            ui.horizontal(|ui| {
                ui.label("Проверить на:");
                if ui.text_edit_singleline(&mut sample).changed() {
                    ui.memory_mut(|m| m.data.insert_persisted(test_id, sample.clone()));
                }
                match test_match_ok {
                    Some(true) => {
                        ui.colored_label(
                            egui::Color32::from_rgb(0x30, 0xc0, 0x60),
                            "совпало",
                        );
                    }
                    Some(false) => {
                        ui.colored_label(
                            egui::Color32::from_rgb(0xf4, 0x47, 0x47),
                            "не совпало",
                        );
                    }
                    None => {}
                }
            });
        });
    }
    if let Some(i) = to_delete {
        app.config.triggers.remove(i);
        changed = true;
    }
    if ui.button("+ Добавить триггер").clicked() {
        app.config.triggers.push(TriggerRule::default());
        changed = true;
    }
    if changed {
        app.triggers.clear_cache();
        app.mark_config_dirty();
    }
}

fn render_highlights(app: &mut TerminalApp, ui: &mut Ui) {
    let mut changed = false;
    let mut to_delete: Option<usize> = None;
    for (i, h) in app.config.highlights.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                changed |= ui.checkbox(&mut h.enabled, "Вкл.").changed();
                changed |= ui.text_edit_singleline(&mut h.name).changed();
                if ui.small_button("×").on_hover_text("Удалить").clicked() {
                    to_delete = Some(i);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Regex:");
                changed |= ui.text_edit_singleline(&mut h.pattern).changed();
            });
            ui.horizontal(|ui| {
                let mut c = rgba_to_color32(h.color);
                if ui.color_edit_button_srgba(&mut c).changed() {
                    let [r, g, b, a] = c.to_array();
                    h.color = [r, g, b, a];
                    changed = true;
                }
                changed |= ui.checkbox(&mut h.apply_to_rx, "RX").changed();
                changed |= ui.checkbox(&mut h.apply_to_tx, "TX").changed();
                changed |= ui.checkbox(&mut h.bold, "жирный").changed();
            });
        });
    }
    if let Some(i) = to_delete {
        app.config.highlights.remove(i);
        changed = true;
    }
    if ui.button("+ Добавить правило").clicked() {
        app.config.highlights.push(HighlightRule::default());
        changed = true;
    }
    if changed {
        app.highlight_engine.clear_cache();
        app.mark_config_dirty();
    }
}

// Хак: формат-комбо для приватных энамов в этом модуле (публичный — в mod.rs).
#[allow(dead_code)]
fn _fmt_combo(ui: &mut Ui, f: &mut SendFormat) {
    ComboBox::from_id_salt(ui.next_auto_id())
        .selected_text(f.label())
        .show_ui(ui, |ui| {
            for v in SendFormat::ALL {
                ui.selectable_value(f, v, v.label());
            }
        });
}
