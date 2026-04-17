//! Тонкий ре-экспорт: логика IO-потока живёт в `connection::manager`.

pub use crate::connection::{InboundMessage, OutboundMessage};
