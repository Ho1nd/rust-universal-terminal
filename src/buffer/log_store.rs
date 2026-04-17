//! Кольцевое хранилище строк: RX, TX и combined.

use std::collections::VecDeque;

use super::log_line::{Direction, LogLine};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogScope {
    Rx,
    Tx,
    Combined,
}

pub struct LogStore {
    pub rx: VecDeque<LogLine>,
    pub tx: VecDeque<LogLine>,
    pub combined: VecDeque<LogLine>,
    pub max_lines: usize,
    pub rx_bytes_total: u64,
    pub tx_bytes_total: u64,
    /// Увеличивается при любом push; UI использует для инвалидации кэшей поиска.
    pub revision: u64,
}

impl LogStore {
    pub fn new(max_lines: usize) -> Self {
        Self {
            rx: VecDeque::new(),
            tx: VecDeque::new(),
            combined: VecDeque::new(),
            max_lines: max_lines.max(1000),
            rx_bytes_total: 0,
            tx_bytes_total: 0,
            revision: 0,
        }
    }

    pub fn set_max_lines(&mut self, n: usize) {
        self.max_lines = n.max(1000);
        Self::trim_to(&mut self.rx, self.max_lines);
        Self::trim_to(&mut self.tx, self.max_lines);
        Self::trim_to(&mut self.combined, self.max_lines);
    }

    fn trim_to(q: &mut VecDeque<LogLine>, cap: usize) {
        while q.len() > cap {
            q.pop_front();
        }
    }

    pub fn push(&mut self, line: LogLine) {
        self.revision = self.revision.wrapping_add(1);
        match line.direction {
            Direction::Rx => {
                self.rx_bytes_total = self.rx_bytes_total.saturating_add(line.bytes.len() as u64);
                if self.rx.len() >= self.max_lines {
                    self.rx.pop_front();
                }
                self.rx.push_back(line.clone());
            }
            Direction::Tx => {
                self.tx_bytes_total = self.tx_bytes_total.saturating_add(line.bytes.len() as u64);
                if self.tx.len() >= self.max_lines {
                    self.tx.pop_front();
                }
                self.tx.push_back(line.clone());
            }
            Direction::Info | Direction::Error => {}
        }
        if self.combined.len() >= self.max_lines {
            self.combined.pop_front();
        }
        self.combined.push_back(line);
    }

    pub fn clear(&mut self, scope: LogScope) {
        self.revision = self.revision.wrapping_add(1);
        match scope {
            LogScope::Rx => self.rx.clear(),
            LogScope::Tx => self.tx.clear(),
            LogScope::Combined => {
                self.rx.clear();
                self.tx.clear();
                self.combined.clear();
            }
        }
    }

    pub fn view(&self, scope: LogScope) -> &VecDeque<LogLine> {
        match scope {
            LogScope::Rx => &self.rx,
            LogScope::Tx => &self.tx,
            LogScope::Combined => &self.combined,
        }
    }
}
