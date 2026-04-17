//! Настройки подключения и смежные модели данных (пресеты, планировщик,
//! макросы, триггеры, правила подсветки).

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ConnectionType {
    #[default]
    Uart,
    TcpClient,
    Udp,
}

impl ConnectionType {
    pub const ALL: [ConnectionType; 3] = [Self::Uart, Self::TcpClient, Self::Udp];
    pub fn label(self) -> &'static str {
        match self {
            Self::Uart => "UART",
            Self::TcpClient => "TCP",
            Self::Udp => "UDP",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StopBits {
    #[default]
    One,
    OnePointFive,
    Two,
}

impl StopBits {
    pub const ALL: [StopBits; 3] = [Self::One, Self::OnePointFive, Self::Two];
    pub fn label(self) -> &'static str {
        match self {
            Self::One => "1",
            Self::OnePointFive => "1.5",
            Self::Two => "2",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Parity {
    #[default]
    None,
    Even,
    Odd,
    Mark,
    Space,
}

impl Parity {
    pub const ALL: [Parity; 5] = [Self::None, Self::Even, Self::Odd, Self::Mark, Self::Space];
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Even => "Even",
            Self::Odd => "Odd",
            Self::Mark => "Mark",
            Self::Space => "Space",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FlowControl {
    #[default]
    None,
    XonXoff,
    RtsCts,
    DsrDtr,
}

impl FlowControl {
    pub const ALL: [FlowControl; 4] = [Self::None, Self::XonXoff, Self::RtsCts, Self::DsrDtr];
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::XonXoff => "Xon/Xoff",
            Self::RtsCts => "RTS/CTS",
            Self::DsrDtr => "DSR/DTR",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UartSettings {
    pub port_name: String,
    pub baud: u32,
    pub data_bits: u8,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow: FlowControl,
}

impl Default for UartSettings {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud: 115_200,
            data_bits: 8,
            stop_bits: StopBits::One,
            parity: Parity::None,
            flow: FlowControl::None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetSettings {
    pub host: String,
    pub port: u16,
    pub udp_bind_local: bool,
    pub udp_local_port: u16,
}

impl Default for NetSettings {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 5000,
            udp_bind_local: false,
            udp_local_port: 5000,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ConnectionSettings {
    pub kind: ConnectionType,
    pub uart: UartSettings,
    pub net: NetSettings,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionPreset {
    pub name: String,
    pub settings: ConnectionSettings,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SendFormat {
    #[default]
    Ascii,
    Hex,
    Decimal,
    Binary,
}

impl SendFormat {
    pub const ALL: [SendFormat; 4] = [Self::Ascii, Self::Hex, Self::Decimal, Self::Binary];
    pub fn label(self) -> &'static str {
        match self {
            Self::Ascii => "ASCII",
            Self::Hex => "HEX",
            Self::Decimal => "Decimal",
            Self::Binary => "Binary",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScheduledCommand {
    pub name: String,
    pub payload: String,
    pub format: SendFormat,
    pub interval_ms: u32,
    pub repeat: u32,
    pub add_newline: bool,
    pub enabled: bool,
}

impl Default for ScheduledCommand {
    fn default() -> Self {
        Self {
            name: "Новая команда".into(),
            payload: String::new(),
            format: SendFormat::Ascii,
            interval_ms: 1000,
            repeat: 0,
            add_newline: true,
            enabled: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MacroKey {
    pub f_number: u8, // 1..=12
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Default for MacroKey {
    fn default() -> Self {
        Self { f_number: 1, ctrl: false, shift: false, alt: false }
    }
}

impl MacroKey {
    pub fn describe(&self) -> String {
        let mut out = String::new();
        if self.ctrl { out.push_str("Ctrl+"); }
        if self.shift { out.push_str("Shift+"); }
        if self.alt { out.push_str("Alt+"); }
        out.push_str(&format!("F{}", self.f_number));
        out
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MacroBinding {
    pub key: MacroKey,
    pub payload: String,
    pub format: SendFormat,
    pub add_newline: bool,
    pub description: String,
}

impl Default for MacroBinding {
    fn default() -> Self {
        Self {
            key: MacroKey::default(),
            payload: String::new(),
            format: SendFormat::Ascii,
            add_newline: true,
            description: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TriggerRule {
    pub name: String,
    pub pattern: String,
    pub response: String,
    pub response_format: SendFormat,
    pub add_newline: bool,
    pub enabled: bool,
    pub cooldown_ms: u32,
    /// Срабатывать также на отправленные (TX) строки — удобно для тестов без
    /// физического loopback'а: послал сам «123» → сработал триггер → ушло «1000».
    #[serde(default)]
    pub apply_to_tx: bool,
}

impl Default for TriggerRule {
    fn default() -> Self {
        Self {
            name: "триггер".into(),
            pattern: String::new(),
            response: String::new(),
            response_format: SendFormat::Ascii,
            add_newline: true,
            enabled: true,
            cooldown_ms: 1000,
            apply_to_tx: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HighlightRule {
    pub name: String,
    pub pattern: String,
    pub color: [u8; 4],
    pub bold: bool,
    pub apply_to_rx: bool,
    pub apply_to_tx: bool,
    pub enabled: bool,
}

impl Default for HighlightRule {
    fn default() -> Self {
        Self {
            name: "подсветка".into(),
            pattern: String::new(),
            color: [0xff, 0xcc, 0x00, 0xff],
            bold: false,
            apply_to_rx: true,
            apply_to_tx: false,
            enabled: true,
        }
    }
}
