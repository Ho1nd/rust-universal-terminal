//! Конфигурация приложения. Сериализация — serde_json (pretty).

pub mod presets;
pub mod theme;
pub mod window;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use self::presets::{
    ConnectionPreset, ConnectionSettings, ConnectionType, FlowControl, HighlightRule,
    MacroBinding, MacroKey, NetSettings, Parity, ScheduledCommand, SendFormat, StopBits,
    TriggerRule, UartSettings,
};
pub use self::theme::{ThemeChoice, ThemeColors};
pub use self::window::WindowConfig;

/// Режимы сборки строк из входящего потока.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DisplayMode {
    #[default]
    ByNewline,
    ByTimer,
    Raw,
}

/// Форматы отображения RX/TX.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DisplayFormat {
    #[default]
    Ascii,
    Hex,
    Decimal,
    Binary,
    Mixed,
}

impl DisplayFormat {
    pub const ALL: [DisplayFormat; 5] = [
        Self::Ascii,
        Self::Hex,
        Self::Decimal,
        Self::Binary,
        Self::Mixed,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Ascii => "ASCII",
            Self::Hex => "HEX",
            Self::Decimal => "Decimal",
            Self::Binary => "Binary",
            Self::Mixed => "Mixed",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DisplayConfig {
    pub mode: DisplayMode,
    pub idle_flush_ms: u32,
    pub max_line_bytes: usize,
    pub timer_interval_ms: u32,
    pub rx_format: DisplayFormat,
    pub tx_format: DisplayFormat,
    pub combined_format: DisplayFormat,
    pub escape_control_chars: bool,
    /// В режиме `ByNewline` считать разделителями строк не только реальные
    /// байты 0x0A/0x0D, но и их литеральные текстовые эквиваленты
    /// (`\n`, `\r`, `\r\n` как последовательности ASCII-символов
    /// `\` + `n`/`r`). Полезно когда удалённая сторона шлёт строки как
    /// JSON/логи с текстовым `\n` вместо реального перевода строки.
    #[serde(default = "default_true")]
    pub split_on_literal_escapes: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            mode: DisplayMode::ByNewline,
            idle_flush_ms: 200,
            max_line_bytes: 4096,
            timer_interval_ms: 50,
            rx_format: DisplayFormat::Ascii,
            tx_format: DisplayFormat::Ascii,
            combined_format: DisplayFormat::Ascii,
            escape_control_chars: true,
            split_on_literal_escapes: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    #[serde(default)]
    pub window: WindowConfig,
    #[serde(default)]
    pub theme: ThemeChoice,
    #[serde(default)]
    pub custom_colors: ThemeColors,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_font_family")]
    pub font_family: String,

    #[serde(default = "default_split")]
    pub split_ratio: f32,
    #[serde(default = "default_left_width")]
    pub left_panel_width: f32,
    #[serde(default = "default_true")]
    pub left_panel_visible: bool,
    #[serde(default)]
    pub combined_view: bool,

    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default = "default_true")]
    pub autoscroll: bool,
    #[serde(default = "default_true")]
    pub show_timestamps: bool,
    #[serde(default = "default_true")]
    pub show_direction_arrows: bool,
    #[serde(default = "default_ts_fmt")]
    pub timestamp_format: String,

    #[serde(default = "default_max_lines")]
    pub max_log_lines: usize,
    #[serde(default)]
    pub continuous_log_file: Option<PathBuf>,

    #[serde(default)]
    pub last_connection: ConnectionSettings,
    #[serde(default)]
    pub presets: Vec<ConnectionPreset>,
    #[serde(default)]
    pub scheduled_commands: Vec<ScheduledCommand>,
    #[serde(default)]
    pub macros: Vec<MacroBinding>,
    #[serde(default)]
    pub triggers: Vec<TriggerRule>,
    #[serde(default)]
    pub highlights: Vec<HighlightRule>,
    #[serde(default)]
    pub send_history: VecDeque<String>,
}

fn default_font_size() -> f32 { 13.0 }
fn default_font_family() -> String { "monospace".into() }
fn default_split() -> f32 { 0.5 }
fn default_left_width() -> f32 { 340.0 }
fn default_true() -> bool { true }
fn default_ts_fmt() -> String { "%H:%M:%S%.3f".into() }
fn default_max_lines() -> usize { 100_000 }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            theme: ThemeChoice::default(),
            custom_colors: ThemeColors::default(),
            font_size: default_font_size(),
            font_family: default_font_family(),
            split_ratio: default_split(),
            left_panel_width: default_left_width(),
            left_panel_visible: true,
            combined_view: false,
            display: DisplayConfig::default(),
            autoscroll: true,
            show_timestamps: true,
            show_direction_arrows: true,
            timestamp_format: default_ts_fmt(),
            max_log_lines: default_max_lines(),
            continuous_log_file: None,
            last_connection: ConnectionSettings::default(),
            presets: Vec::new(),
            scheduled_commands: Vec::new(),
            macros: Vec::new(),
            triggers: Vec::new(),
            highlights: Vec::new(),
            send_history: VecDeque::new(),
        }
    }
}

/// Путь к файлу конфигурации. Использует директорию из `directories`.
pub fn config_file_path() -> PathBuf {
    if let Some(pd) = directories::ProjectDirs::from("", "", "RustTerminal") {
        let dir = pd.config_dir().to_path_buf();
        let _ = std::fs::create_dir_all(&dir);
        return dir.join("config.json");
    }
    PathBuf::from("config.json")
}

impl AppConfig {
    pub fn load_or_default() -> Self {
        let path = config_file_path();
        Self::load_from(&path).unwrap_or_else(|e| {
            if path.exists() {
                log::warn!("failed to load config {}: {e}; backing up and using defaults", path.display());
                let backup = path.with_extension(format!(
                    "json.bak.{}",
                    chrono::Local::now().format("%Y%m%d%H%M%S")
                ));
                let _ = std::fs::rename(&path, &backup);
            }
            let mut cfg = Self::default();
            cfg.migrate_scheduled_commands_json();
            cfg
        })
    }

    pub fn load_from(path: &Path) -> anyhow::Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let cfg: Self = serde_json::from_str(&data)?;
        Ok(cfg)
    }

    pub fn save(&self) {
        let path = config_file_path();
        if let Err(e) = self.save_to(&path) {
            log::error!("failed to save config: {e}");
        }
    }

    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Импорт `scheduled_commands.json` из Python-версии, если он лежит в CWD.
    pub fn migrate_scheduled_commands_json(&mut self) {
        let candidate = PathBuf::from("scheduled_commands.json");
        if !candidate.exists() || !self.scheduled_commands.is_empty() {
            return;
        }
        let data = match std::fs::read_to_string(&candidate) {
            Ok(s) => s,
            Err(_) => return,
        };
        #[derive(Deserialize)]
        struct PyCmd {
            #[serde(default)]
            name: String,
            #[serde(default)]
            command: String,
            #[serde(default = "py_default_format")]
            format: String,
            #[serde(default = "py_default_interval")]
            interval: u32,
            #[serde(default)]
            repeat: u32,
            #[serde(default)]
            add_newline: bool,
        }
        fn py_default_format() -> String { "ASCII".into() }
        fn py_default_interval() -> u32 { 1000 }

        let parsed: Result<Vec<PyCmd>, _> = serde_json::from_str(&data);
        if let Ok(list) = parsed {
            for c in list {
                let fmt = match c.format.as_str() {
                    "HEX" => SendFormat::Hex,
                    "Decimal" => SendFormat::Decimal,
                    "Binary" => SendFormat::Binary,
                    _ => SendFormat::Ascii,
                };
                self.scheduled_commands.push(ScheduledCommand {
                    name: c.name,
                    payload: c.command,
                    format: fmt,
                    interval_ms: c.interval.max(10),
                    repeat: c.repeat,
                    add_newline: c.add_newline,
                    enabled: true,
                });
            }
            log::info!(
                "imported {} scheduled commands from scheduled_commands.json",
                self.scheduled_commands.len()
            );
        }
    }
}
