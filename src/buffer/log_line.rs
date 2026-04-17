//! Единичная строка лога с кэшируемым форматированным представлением.

use std::sync::Arc;

use chrono::{DateTime, Local};
use parking_lot::Mutex;

use crate::config::DisplayFormat;
use crate::formats;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Rx,
    Tx,
    Info,
    Error,
}

impl Direction {
    pub fn arrow(self) -> &'static str {
        match self {
            Self::Rx => "←",
            Self::Tx => "→",
            Self::Info => "·",
            Self::Error => "!",
        }
    }
}

/// Одна строка. `bytes` — `Arc<[u8]>` для дешёвого клонирования при поиске/копии.
#[derive(Clone)]
pub struct LogLine {
    pub timestamp: DateTime<Local>,
    pub direction: Direction,
    pub bytes: Arc<[u8]>,
    pub highlight: Option<[u8; 4]>,
    cache: Arc<Mutex<CacheSlot>>,
}

#[derive(Default)]
struct CacheSlot {
    fmt: Option<DisplayFormat>,
    escape_ctrl: bool,
    text: Arc<str>,
}

impl LogLine {
    pub fn new(direction: Direction, timestamp: DateTime<Local>, bytes: Arc<[u8]>) -> Self {
        Self {
            direction,
            timestamp,
            bytes,
            highlight: None,
            cache: Arc::new(Mutex::new(CacheSlot::default())),
        }
    }

    pub fn info(text: &str) -> Self {
        Self::new(
            Direction::Info,
            Local::now(),
            Arc::from(text.as_bytes()),
        )
    }

    pub fn error(text: &str) -> Self {
        Self::new(
            Direction::Error,
            Local::now(),
            Arc::from(text.as_bytes()),
        )
    }

    /// Получить отформатированное представление (с кэшированием по паре
    /// `(fmt, escape_ctrl)`).
    pub fn formatted(&self, fmt: DisplayFormat, escape_ctrl: bool) -> Arc<str> {
        let mut slot = self.cache.lock();
        if slot.fmt == Some(fmt) && slot.escape_ctrl == escape_ctrl && !slot.text.is_empty() {
            return Arc::clone(&slot.text);
        }
        let s = formats::format_bytes(&self.bytes, fmt, escape_ctrl);
        slot.fmt = Some(fmt);
        slot.escape_ctrl = escape_ctrl;
        slot.text = Arc::from(s.into_boxed_str());
        Arc::clone(&slot.text)
    }

    pub fn invalidate_cache(&self) {
        let mut slot = self.cache.lock();
        slot.fmt = None;
        slot.text = Arc::from("");
    }
}
