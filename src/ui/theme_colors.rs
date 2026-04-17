//! Применение темы и шрифта к `egui::Context`.

use egui::{Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Visuals};

use crate::config::{AppConfig, ThemeChoice, ThemeColors};

pub fn rgba_to_color32(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

pub fn resolved_colors(cfg: &AppConfig) -> ThemeColors {
    ThemeColors::for_choice(cfg.theme, cfg.custom_colors)
}

pub fn apply_theme(ctx: &Context, cfg: &AppConfig) {
    let mut visuals = match cfg.theme {
        ThemeChoice::Light => Visuals::light(),
        _ => Visuals::dark(),
    };
    let colors = resolved_colors(cfg);
    visuals.window_fill = rgba_to_color32(colors.background);
    visuals.panel_fill = rgba_to_color32(colors.background);
    visuals.extreme_bg_color = rgba_to_color32(colors.background);
    visuals.override_text_color = Some(rgba_to_color32(colors.text));
    visuals.hyperlink_color = rgba_to_color32(colors.accent);
    ctx.set_visuals(visuals);
}

pub fn apply_font(ctx: &Context, cfg: &AppConfig) {
    let mut style = (*ctx.style()).clone();
    for (_ts, fid) in style.text_styles.iter_mut() {
        if fid.family == FontFamily::Monospace {
            fid.size = cfg.font_size;
        } else {
            fid.size = (cfg.font_size - 1.0).max(10.0);
        }
    }
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::new(cfg.font_size - 1.0, FontFamily::Proportional),
    );
    ctx.set_style(style);
}

/// Регистрирует системные шрифты как fallback к дефолтным egui-шрифтам,
/// чтобы корректно отображались символы вне Latin-1 (иконки поиска,
/// геометрические фигуры ▲▼●○, × и т.д.). Вызывается один раз при старте.
pub fn install_system_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    let mut added: Vec<String> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        let candidates: &[(&str, &str)] = &[
            // Symbol/emoji шрифты — максимум покрытия по иконкам и геометрии.
            ("sys-seguisym", r"C:\Windows\Fonts\seguisym.ttf"),
            ("sys-seguiemj", r"C:\Windows\Fonts\seguiemj.ttf"),
            // Базовый UI-шрифт Windows с широким Latin/Cyrillic покрытием.
            ("sys-segoeui", r"C:\Windows\Fonts\segoeui.ttf"),
            ("sys-arial", r"C:\Windows\Fonts\arial.ttf"),
            // Моноширинный
            ("sys-consola", r"C:\Windows\Fonts\consola.ttf"),
        ];
        for (name, path) in candidates {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert((*name).to_string(), FontData::from_owned(data));
                added.push((*name).to_string());
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let candidates: &[(&str, &str)] = &[
            ("sys-noto", "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"),
            ("sys-dejavu", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
            (
                "sys-noto-sym",
                "/usr/share/fonts/truetype/noto/NotoSansSymbols-Regular.ttf",
            ),
        ];
        for (name, path) in candidates {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert((*name).to_string(), FontData::from_owned(data));
                added.push((*name).to_string());
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let candidates: &[(&str, &str)] = &[
            ("sys-appleemoji", "/System/Library/Fonts/Apple Color Emoji.ttc"),
            ("sys-helvetica", "/System/Library/Fonts/Helvetica.ttc"),
        ];
        for (name, path) in candidates {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert((*name).to_string(), FontData::from_owned(data));
                added.push((*name).to_string());
            }
        }
    }

    // Добавляем все найденные шрифты как fallback и в Proportional, и в Monospace.
    if !added.is_empty() {
        for fam in [FontFamily::Proportional, FontFamily::Monospace] {
            let list = fonts.families.entry(fam).or_default();
            for name in &added {
                if !list.contains(name) {
                    list.push(name.clone());
                }
            }
        }
    }

    ctx.set_fonts(fonts);
}
