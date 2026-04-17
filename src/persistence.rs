//! Помощники для сохранения/загрузки логов и continuous-tee.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use parking_lot::Mutex;

use crate::buffer::{Direction, LogLine};
use crate::config::DisplayFormat;

/// Сохранить список строк в plain-text в выбранном формате.
pub fn save_log_txt(path: &Path, lines: &[LogLine], fmt: DisplayFormat, escape: bool) -> std::io::Result<()> {
    let f = File::create(path)?;
    let mut w = BufWriter::new(f);
    for line in lines {
        let arrow = match line.direction {
            Direction::Rx => "←",
            Direction::Tx => "→",
            Direction::Info => "·",
            Direction::Error => "!",
        };
        let ts = line.timestamp.format("%Y-%m-%d %H:%M:%S%.3f");
        let text = line.formatted(fmt, escape);
        writeln!(w, "{arrow} [{ts}] {text}")?;
    }
    w.flush()
}

/// Сохранить лог в CSV: timestamp, direction, hex, ascii.
pub fn save_log_csv(path: &Path, lines: &[LogLine]) -> std::io::Result<()> {
    let f = File::create(path)?;
    let mut w = BufWriter::new(f);
    writeln!(w, "timestamp,direction,hex,ascii")?;
    for line in lines {
        let dir = match line.direction {
            Direction::Rx => "RX",
            Direction::Tx => "TX",
            Direction::Info => "INFO",
            Direction::Error => "ERROR",
        };
        let ts = line.timestamp.format("%Y-%m-%d %H:%M:%S%.3f");
        let hex_s = line
            .bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii_s: String = line
            .bytes
            .iter()
            .map(|&b| if (0x20..=0x7e).contains(&b) { b as char } else { '.' })
            .collect();
        let escape_csv = |s: &str| -> String {
            if s.contains(',') || s.contains('"') || s.contains('\n') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        };
        writeln!(
            w,
            "{ts},{dir},{},{}",
            escape_csv(&hex_s),
            escape_csv(&ascii_s)
        )?;
    }
    w.flush()
}

/// Дебаунс-буфер для continuous-tee в файл. Открывает файл на append,
/// дописывает строки, flush после каждой.
pub struct ContinuousLogger {
    path: PathBuf,
    file: Mutex<Option<BufWriter<File>>>,
}

impl ContinuousLogger {
    pub fn open(path: PathBuf) -> std::io::Result<Self> {
        let f = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            path,
            file: Mutex::new(Some(BufWriter::new(f))),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn log(&self, line: &LogLine) {
        let mut guard = self.file.lock();
        if let Some(w) = guard.as_mut() {
            let dir = match line.direction {
                Direction::Rx => "RX",
                Direction::Tx => "TX",
                Direction::Info => "INFO",
                Direction::Error => "ERROR",
            };
            let ts = line.timestamp.format("%Y-%m-%d %H:%M:%S%.3f");
            let ascii: String = line
                .bytes
                .iter()
                .map(|&b| if (0x20..=0x7e).contains(&b) { b as char } else { '.' })
                .collect();
            let hex_s = line
                .bytes
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            let _ = writeln!(w, "[{ts}] {dir}: {hex_s} | {ascii}");
            let _ = w.flush();
        }
    }
}
