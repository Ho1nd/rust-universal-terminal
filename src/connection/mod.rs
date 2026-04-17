//! Абстракция подключения (UART / TCP / UDP).

pub mod manager;
pub mod tcp;
pub mod udp;
pub mod uart;

use std::io;

pub use manager::{ConnectionManager, InboundMessage, OutboundMessage};

/// Универсальный интерфейс, который реализуется всеми транспортами.
/// `read` должен быть неблокирующим с таймаутом ~100-200мс.
pub trait Connection: Send {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    fn write(&mut self, data: &[u8]) -> io::Result<()>;
    fn description(&self) -> String;
}

#[derive(thiserror::Error, Debug)]
pub enum ConnectionError {
    #[error("Ошибка ввода-вывода: {0}")]
    Io(#[from] io::Error),
    #[error("Ошибка COM-порта: {0}")]
    Serial(#[from] serialport::Error),
    #[error("{0}")]
    Other(String),
}
