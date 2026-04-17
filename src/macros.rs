//! Резолвер горячих клавиш → макрос.

use crate::config::{MacroBinding, MacroKey};

pub fn find_macro(
    bindings: &[MacroBinding],
    f_number: u8,
    ctrl: bool,
    shift: bool,
    alt: bool,
) -> Option<&MacroBinding> {
    let wanted = MacroKey { f_number, ctrl, shift, alt };
    bindings.iter().find(|m| m.key == wanted)
}

pub fn f_key_from_egui(key: egui::Key) -> Option<u8> {
    match key {
        egui::Key::F1 => Some(1),
        egui::Key::F2 => Some(2),
        egui::Key::F3 => Some(3),
        egui::Key::F4 => Some(4),
        egui::Key::F5 => Some(5),
        egui::Key::F6 => Some(6),
        egui::Key::F7 => Some(7),
        egui::Key::F8 => Some(8),
        egui::Key::F9 => Some(9),
        egui::Key::F10 => Some(10),
        egui::Key::F11 => Some(11),
        egui::Key::F12 => Some(12),
        _ => None,
    }
}
