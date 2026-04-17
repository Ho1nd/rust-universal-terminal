//! Сборка входящего потока байт в логические строки.
//!
//! Поддерживает три режима (`DisplayMode`):
//! * `ByNewline` — разбивает поток по **универсальным переводам строки**:
//!   `\n` (Unix/LF), `\r\n` (Windows/CRLF) и одиночный `\r` (старый macOS/CR).
//!   Таймаут простоя (`idle_flush_ms`) выплёвывает накопленный хвост как строку.
//!   При превышении `max_line_bytes` принудительно эмитит строку.
//! * `ByTimer` — копит всё в буфер и эмитит одну строку по таймеру
//!   (`timer_interval_ms`), либо на явный `flush_now`.
//! * `Raw` — каждый чанк сразу отдельная строка.
//!
//! **Таймстемп** — время первого байта в строке, не последнего.

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};

use crate::buffer::log_line::{Direction, LogLine};
use crate::config::DisplayMode;

pub struct LineAssembler {
    direction: Direction,
    mode: DisplayMode,
    pub idle_flush_ms: u32,
    pub max_line_bytes: usize,
    pub timer_interval_ms: u32,
    pub split_on_literal_escapes: bool,

    acc: Vec<u8>,
    first_byte_wall: Option<DateTime<Local>>,
    last_activity: Option<Instant>,
    last_timer_flush: Instant,
}

impl LineAssembler {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            mode: DisplayMode::ByNewline,
            idle_flush_ms: 200,
            max_line_bytes: 4096,
            timer_interval_ms: 50,
            split_on_literal_escapes: true,
            acc: Vec::with_capacity(4096),
            first_byte_wall: None,
            last_activity: None,
            last_timer_flush: Instant::now(),
        }
    }

    pub fn set_mode(&mut self, mode: DisplayMode) {
        if self.mode != mode {
            self.mode = mode;
        }
    }

    pub fn set_limits(&mut self, idle_flush_ms: u32, max_line_bytes: usize, timer_interval_ms: u32) {
        self.idle_flush_ms = idle_flush_ms;
        self.max_line_bytes = max_line_bytes.max(1);
        self.timer_interval_ms = timer_interval_ms.max(1);
    }

    pub fn set_split_on_literal_escapes(&mut self, enabled: bool) {
        self.split_on_literal_escapes = enabled;
    }

    /// Сбросить внутреннее состояние (например, при reconnect).
    pub fn reset(&mut self) {
        self.acc.clear();
        self.first_byte_wall = None;
        self.last_activity = None;
        self.last_timer_flush = Instant::now();
    }

    /// Обработать входящий чанк байт. Вернёт набор готовых строк.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<LogLine> {
        if chunk.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let now_wall = Local::now();
        let now_mono = Instant::now();
        self.last_activity = Some(now_mono);

        match self.mode {
            DisplayMode::Raw => {
                out.push(self.make_line(now_wall, chunk.to_vec()));
            }
            DisplayMode::ByNewline => {
                if self.first_byte_wall.is_none() {
                    self.first_byte_wall = Some(now_wall);
                }
                self.acc.extend_from_slice(chunk);

                loop {
                    // Найти следующий разделитель строк: \n (с опциональным
                    // предшествующим \r → CRLF), одиночный \r или, если
                    // включено, литеральные `\\n`/`\\r`/`\\r\\n`.
                    let Some((line_end_excl, consume_up_to_incl)) =
                        find_line_break(&self.acc, self.split_on_literal_escapes)
                    else {
                        break;
                    };
                    let line_bytes = self.acc[..line_end_excl].to_vec();
                    let ts = self.first_byte_wall.take().unwrap_or(now_wall);
                    out.push(self.make_line(ts, line_bytes));
                    self.acc.drain(..=consume_up_to_incl);
                    if !self.acc.is_empty() {
                        self.first_byte_wall = Some(Local::now());
                    }
                }

                if self.acc.len() >= self.max_line_bytes {
                    let ts = self.first_byte_wall.take().unwrap_or(now_wall);
                    let bytes = std::mem::take(&mut self.acc);
                    out.push(self.make_line(ts, bytes));
                }
            }
            DisplayMode::ByTimer => {
                if self.first_byte_wall.is_none() {
                    self.first_byte_wall = Some(now_wall);
                }
                self.acc.extend_from_slice(chunk);
            }
        }
        out
    }

    /// Периодически вызываемый метод — эмитит хвост по таймауту/таймеру.
    pub fn poll_timeout(&mut self) -> Vec<LogLine> {
        if self.acc.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        match self.mode {
            DisplayMode::ByNewline => {
                if let Some(last) = self.last_activity {
                    if Instant::now().duration_since(last)
                        >= Duration::from_millis(self.idle_flush_ms as u64)
                    {
                        let ts = self.first_byte_wall.take().unwrap_or_else(Local::now);
                        let bytes = std::mem::take(&mut self.acc);
                        out.push(self.make_line(ts, bytes));
                    }
                }
            }
            DisplayMode::ByTimer => {
                let since = Instant::now().duration_since(self.last_timer_flush);
                if since >= Duration::from_millis(self.timer_interval_ms as u64) {
                    let ts = self.first_byte_wall.take().unwrap_or_else(Local::now);
                    let bytes = std::mem::take(&mut self.acc);
                    out.push(self.make_line(ts, bytes));
                    self.last_timer_flush = Instant::now();
                }
            }
            DisplayMode::Raw => {}
        }
        out
    }

    /// Принудительный flush текущего накопителя (при disconnect).
    pub fn flush_now(&mut self) -> Option<LogLine> {
        if self.acc.is_empty() {
            return None;
        }
        let ts = self.first_byte_wall.take().unwrap_or_else(Local::now);
        let bytes = std::mem::take(&mut self.acc);
        Some(self.make_line(ts, bytes))
    }

    fn make_line(&self, ts: DateTime<Local>, bytes: Vec<u8>) -> LogLine {
        LogLine::new(self.direction, ts, Arc::from(bytes.into_boxed_slice()))
    }
}

/// Ищет первый разделитель строк в буфере.
///
/// Возвращает пару `(line_end_excl, consume_up_to_incl)`:
/// * `line_end_excl` — длина байтов, которые войдут в текущую строку;
/// * `consume_up_to_incl` — индекс последнего байта разделителя, который нужно
///   удалить вместе со строкой (`drain(..=consume_up_to_incl)`).
///
/// Всегда поддерживаются LF (`\n`), CRLF (`\r\n`) и одиночный CR (`\r`).
/// При `literal_escapes = true` дополнительно ищутся литеральные ASCII-
/// последовательности `\n`, `\r`, `\r\n` (два или три байта: `\\ n`,
/// `\\ r`, `\\ r \\ n`) — это частый случай, когда удалённая сторона
/// шлёт строки в текстовом формате без реальных управляющих байт.
///
/// Если буфер оканчивается на `\r` — возвращает `None`, чтобы дождаться
/// следующего байта (возможно CRLF). Аналогично удерживается хвост
/// `\\ r`, который может превратиться в `\\ r \\ n`.
pub(crate) fn find_line_break(buf: &[u8], literal_escapes: bool) -> Option<(usize, usize)> {
    let mut i = 0;
    while i < buf.len() {
        let b = buf[i];
        match b {
            b'\n' => {
                let end = if i > 0 && buf[i - 1] == b'\r' { i - 1 } else { i };
                return Some((end, i));
            }
            b'\r' => {
                if i + 1 < buf.len() {
                    if buf[i + 1] == b'\n' {
                        i += 1;
                        continue;
                    }
                    return Some((i, i));
                }
                return None;
            }
            b'\\' if literal_escapes && i + 1 < buf.len() => {
                match buf[i + 1] {
                    b'n' => {
                        // `\\ n` — два байта разделителя.
                        let end = if i >= 2 && buf[i - 2] == b'\\' && buf[i - 1] == b'r' {
                            i - 2
                        } else {
                            i
                        };
                        return Some((end, i + 1));
                    }
                    b'r' => {
                        // Литеральный `\\ r`: может оказаться `\\ r \\ n`
                        // (CRLF в текстовой форме). Нужно дождаться минимум
                        // двух следующих байт после него.
                        if i + 3 < buf.len() {
                            if buf[i + 2] == b'\\' && buf[i + 3] == b'n' {
                                i += 2;
                                continue;
                            }
                            return Some((i, i + 1));
                        }
                        if i + 2 < buf.len() && buf[i + 2] != b'\\' {
                            return Some((i, i + 1));
                        }
                        // Неполно — ждём следующий чанк.
                        return None;
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}
