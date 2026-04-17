//! Панель планировщика (inline внутри левой панели).

use egui::Ui;

use crate::app::TerminalApp;
use crate::config::ScheduledCommand;

pub fn render(app: &mut TerminalApp, ui: &mut Ui) {
    let mut changed = false;
    let mut to_delete: Option<usize> = None;

    for (i, cmd) in app.config.scheduled_commands.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                changed |= ui.checkbox(&mut cmd.enabled, "Вкл.").changed();
                changed |= ui.text_edit_singleline(&mut cmd.name).changed();
                if ui.small_button("×").on_hover_text("Удалить").clicked() {
                    to_delete = Some(i);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Формат:");
                changed |= super::send_format_combo(ui, &mut cmd.format);
                changed |= ui.checkbox(&mut cmd.add_newline, "CR/LF").changed();
            });
            changed |= ui.text_edit_singleline(&mut cmd.payload).changed();
            ui.horizontal(|ui| {
                ui.label("Интервал, мс:");
                changed |= ui
                    .add(egui::DragValue::new(&mut cmd.interval_ms).range(10..=600_000))
                    .changed();
                ui.label("Повторы (0=∞):");
                changed |= ui
                    .add(egui::DragValue::new(&mut cmd.repeat).range(0..=1_000_000))
                    .changed();
            });
        });
    }

    if let Some(i) = to_delete {
        app.config.scheduled_commands.remove(i);
        changed = true;
    }

    ui.horizontal(|ui| {
        if ui.button("+ Команда").clicked() {
            app.config.scheduled_commands.push(ScheduledCommand::default());
            changed = true;
        }
        if app.scheduler.running {
            if ui.button("Остановить планировщик").clicked() {
                app.scheduler.stop();
            }
        } else if ui.button("Запустить планировщик").clicked() {
            if app.config.scheduled_commands.is_empty() {
                app.show_toast("Нет команд", true);
            } else if !app.is_connected() {
                app.show_toast("Подключение не установлено", true);
            } else {
                let list = app.config.scheduled_commands.clone();
                app.scheduler.start(&list);
            }
        }
    });

    if changed {
        app.refresh_scheduler_if_needed();
        app.mark_config_dirty();
    }
}
