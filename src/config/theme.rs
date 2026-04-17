//! Темы оформления. Цвета хранятся как RGBA-байты.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    #[default]
    Dark,
    Light,
    SolarizedDark,
    Dracula,
    Custom,
}

impl ThemeChoice {
    pub const ALL: [ThemeChoice; 5] = [
        Self::Dark,
        Self::Light,
        Self::SolarizedDark,
        Self::Dracula,
        Self::Custom,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::SolarizedDark => "Solarized Dark",
            Self::Dracula => "Dracula",
            Self::Custom => "Custom",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeColors {
    pub background: [u8; 4],
    pub text: [u8; 4],
    pub rx: [u8; 4],
    pub tx: [u8; 4],
    pub info: [u8; 4],
    pub error: [u8; 4],
    pub accent: [u8; 4],
    pub timestamp: [u8; 4],
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self::dark()
    }
}

impl ThemeColors {
    pub fn dark() -> Self {
        Self {
            background: [0x1e, 0x1e, 0x1e, 0xff],
            text: [0xd4, 0xd4, 0xd4, 0xff],
            rx: [0x4e, 0xc9, 0xb0, 0xff],
            tx: [0x56, 0x9c, 0xd6, 0xff],
            info: [0x80, 0x80, 0x80, 0xff],
            error: [0xf4, 0x47, 0x47, 0xff],
            accent: [0x00, 0x7a, 0xcc, 0xff],
            timestamp: [0x9c, 0x9c, 0x9c, 0xff],
        }
    }

    pub fn light() -> Self {
        Self {
            background: [0xff, 0xff, 0xff, 0xff],
            text: [0x1e, 0x1e, 0x1e, 0xff],
            rx: [0x09, 0x86, 0x58, 0xff],
            tx: [0x04, 0x51, 0xa5, 0xff],
            info: [0x60, 0x60, 0x60, 0xff],
            error: [0xcd, 0x31, 0x31, 0xff],
            accent: [0x00, 0x5c, 0xa6, 0xff],
            timestamp: [0x70, 0x70, 0x70, 0xff],
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            background: [0x00, 0x2b, 0x36, 0xff],
            text: [0x83, 0x94, 0x96, 0xff],
            rx: [0x85, 0x99, 0x00, 0xff],
            tx: [0x26, 0x8b, 0xd2, 0xff],
            info: [0x58, 0x6e, 0x75, 0xff],
            error: [0xdc, 0x32, 0x2f, 0xff],
            accent: [0xb5, 0x89, 0x00, 0xff],
            timestamp: [0x65, 0x7b, 0x83, 0xff],
        }
    }

    pub fn dracula() -> Self {
        Self {
            background: [0x28, 0x2a, 0x36, 0xff],
            text: [0xf8, 0xf8, 0xf2, 0xff],
            rx: [0x50, 0xfa, 0x7b, 0xff],
            tx: [0x8b, 0xe9, 0xfd, 0xff],
            info: [0x62, 0x72, 0xa4, 0xff],
            error: [0xff, 0x55, 0x55, 0xff],
            accent: [0xbd, 0x93, 0xf9, 0xff],
            timestamp: [0x6d, 0x7a, 0x9c, 0xff],
        }
    }

    pub fn for_choice(choice: ThemeChoice, custom: ThemeColors) -> Self {
        match choice {
            ThemeChoice::Dark => Self::dark(),
            ThemeChoice::Light => Self::light(),
            ThemeChoice::SolarizedDark => Self::solarized_dark(),
            ThemeChoice::Dracula => Self::dracula(),
            ThemeChoice::Custom => custom,
        }
    }
}
