//! Виртуализированный рендер строк лога.

use std::time::Instant;

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, ScrollArea, TextStyle, Ui};

use crate::app::TerminalApp;
use crate::buffer::{Direction, LogScope};
use crate::highlight::HighlightSpan;
use crate::ui::theme_colors::{resolved_colors, rgba_to_color32};

pub fn render(app: &mut TerminalApp, ui: &mut Ui, scope: LogScope) {
    let colors = resolved_colors(&app.config);
    let row_height = ui.text_style_height(&TextStyle::Monospace) + 2.0;
    let font_id: FontId = TextStyle::Monospace.resolve(ui.style());

    let should_flash_idx = match app.flash_line {
        Some((s, idx, until)) if s == scope && Instant::now() < until => Some(idx),
        _ => None,
    };

    let fmt = app.default_format_for(scope);
    let escape = app.config.display.escape_control_chars;
    let show_ts = app.config.show_timestamps;
    let show_arrow = app.config.show_direction_arrows;
    let ts_fmt = app.config.timestamp_format.clone();

    let lines_snapshot: Vec<_> = {
        let store = app.log_store.lock();
        store.view(scope).iter().cloned().collect()
    };
    let total = lines_snapshot.len();
    let highlight_rules = app.config.highlights.clone();

    // Явный запрос прокрутки (например, из поиска). Если совпадает со scope —
    // сбрасываем autoscroll-to-bottom на этот кадр и ставим нужный offset.
    let pending_scroll_idx: Option<usize> = match app.scroll_request {
        Some((s, idx)) if s == scope => Some(idx),
        _ => None,
    };
    if pending_scroll_idx.is_some() {
        app.scroll_request = None;
    }

    let stick = app.config.autoscroll && !app.paused && pending_scroll_idx.is_none();

    let mut sa = ScrollArea::vertical()
        .id_salt(("log_scroll", match scope {
            LogScope::Rx => "rx",
            LogScope::Tx => "tx",
            LogScope::Combined => "combined",
        }))
        .stick_to_bottom(stick)
        .auto_shrink([false, false]);

    if let Some(idx) = pending_scroll_idx {
        // Позиционируем целевую строку примерно по центру видимой области.
        let visible = ui.available_height().max(row_height);
        let target_offset =
            (idx as f32 * row_height - (visible * 0.5 - row_height * 0.5)).max(0.0);
        sa = sa.vertical_scroll_offset(target_offset);
    }

    sa.show_rows(ui, row_height, total, |ui, row_range| {
        for i in row_range {
            let Some(line) = lines_snapshot.get(i) else { continue; };
            let text_arc = line.formatted(fmt, escape);

            let base_color = match line.direction {
                Direction::Rx => rgba_to_color32(colors.rx),
                Direction::Tx => rgba_to_color32(colors.tx),
                Direction::Info => rgba_to_color32(colors.info),
                Direction::Error => rgba_to_color32(colors.error),
            };

            let mut job = LayoutJob::default();

            // Префикс (стрелка + таймстемп) — всегда базовым цветом.
            if show_arrow {
                append_plain(&mut job, line.direction.arrow(), base_color, &font_id);
                append_plain(&mut job, " ", base_color, &font_id);
            }
            if show_ts {
                let ts_prefix = format!("[{}] ", line.timestamp.format(&ts_fmt));
                append_plain(&mut job, &ts_prefix, base_color, &font_id);
            }

            // Тело строки с пофрагментной подсветкой.
            let body: &str = &text_arc;
            let spans: Vec<HighlightSpan> = app
                .highlight_engine
                .spans_for(&highlight_rules, line.direction, body);

            let mut cur = 0usize;
            for sp in &spans {
                if sp.start > cur {
                    append_plain(&mut job, &body[cur..sp.start], base_color, &font_id);
                }
                let bg = rgba_to_color32(sp.color);
                let fg = readable_fg_on(bg, base_color);
                let seg_fmt = TextFormat {
                    color: fg,
                    background: bg,
                    font_id: font_id.clone(),
                    italics: false,
                    underline: if sp.bold {
                        egui::Stroke::new(1.0, fg)
                    } else {
                        egui::Stroke::NONE
                    },
                    ..Default::default()
                };
                job.append(&body[sp.start..sp.end], 0.0, seg_fmt);
                cur = sp.end;
            }
            if cur < body.len() {
                append_plain(&mut job, &body[cur..], base_color, &font_id);
            }

            let flash = Some(i) == should_flash_idx;
            if flash {
                let flash_bg = Color32::from_rgba_unmultiplied(0xff, 0xe0, 0x80, 0x80);
                egui::Frame::none()
                    .fill(flash_bg)
                    .inner_margin(egui::Margin::ZERO)
                    .show(ui, |ui| {
                        ui.label(job);
                    });
            } else {
                ui.label(job);
            }
        }
    });
}

fn append_plain(job: &mut LayoutJob, text: &str, color: Color32, font_id: &FontId) {
    if text.is_empty() {
        return;
    }
    job.append(
        text,
        0.0,
        TextFormat {
            color,
            font_id: font_id.clone(),
            ..Default::default()
        },
    );
}

/// Выбирает читаемый цвет текста поверх заданного фона. Если контраст
/// `preferred` на фоне ≥ 3.0 — используем его, иначе переключаемся на
/// чёрный/белый.
fn readable_fg_on(bg: Color32, preferred: Color32) -> Color32 {
    if contrast_ratio(preferred, bg) >= 3.0 {
        return preferred;
    }
    let lum = relative_luminance(bg);
    if lum > 0.5 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

fn relative_luminance(c: Color32) -> f32 {
    let [r, g, b, _] = c.to_array();
    let srgb = |x: u8| {
        let v = x as f32 / 255.0;
        if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * srgb(r) + 0.7152 * srgb(g) + 0.0722 * srgb(b)
}

fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let la = relative_luminance(a);
    let lb = relative_luminance(b);
    let (l1, l2) = if la > lb { (la, lb) } else { (lb, la) };
    (l1 + 0.05) / (l2 + 0.05)
}
