//! Буферы лога: кольцевое хранилище `LogLine` + сборка строк.

pub mod line_assembler;
pub mod log_line;
pub mod log_store;

pub use line_assembler::LineAssembler;
pub use log_line::{Direction, LogLine};
pub use log_store::{LogScope, LogStore};
