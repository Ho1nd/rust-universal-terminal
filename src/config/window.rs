//! Конфигурация окна (размер/позиция/максимизация).

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WindowConfig {
    pub width: f32,
    pub height: f32,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub maximized: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1400.0,
            height: 850.0,
            x: None,
            y: None,
            maximized: false,
        }
    }
}
