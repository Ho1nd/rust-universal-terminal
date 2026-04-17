//! Менеджер активного подключения и его IO-потока.

use std::sync::Arc;
use std::thread::JoinHandle;

use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;

use crate::config::{ConnectionSettings, ConnectionType};

use super::{tcp::TcpConnection, udp::UdpConnection, uart::SerialConnection, Connection};

/// Сообщения UI → IO.
pub enum OutboundMessage {
    Send(Vec<u8>),
    Close,
}

/// Сообщения IO → UI.
#[derive(Debug, Clone)]
pub enum InboundMessage {
    RxBytes(Vec<u8>),
    TxEcho(Vec<u8>),
    Connected(String),
    Disconnected(Option<String>),
    Error(String),
}

pub struct ConnectionManager {
    out_tx: Option<Sender<OutboundMessage>>,
    in_rx: Receiver<InboundMessage>,
    in_tx: Sender<InboundMessage>,
    thread: Option<JoinHandle<()>>,
    pub description: Arc<Mutex<Option<String>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        let (in_tx, in_rx) = bounded::<InboundMessage>(1024);
        Self {
            out_tx: None,
            in_rx,
            in_tx,
            thread: None,
            description: Arc::new(Mutex::new(None)),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.out_tx.is_some()
    }

    pub fn inbound(&self) -> Receiver<InboundMessage> {
        self.in_rx.clone()
    }

    pub fn connect(&mut self, settings: &ConnectionSettings) -> Result<(), String> {
        if self.is_connected() {
            return Err("Уже подключено".into());
        }

        let conn: Box<dyn Connection> = match settings.kind {
            ConnectionType::Uart => Box::new(
                SerialConnection::open(&settings.uart).map_err(|e| e.to_string())?,
            ),
            ConnectionType::TcpClient => Box::new(
                TcpConnection::connect(&settings.net).map_err(|e| e.to_string())?,
            ),
            ConnectionType::Udp => Box::new(
                UdpConnection::open(&settings.net).map_err(|e| e.to_string())?,
            ),
        };
        let desc = conn.description();
        *self.description.lock() = Some(desc.clone());

        let (out_tx, out_rx) = bounded::<OutboundMessage>(256);
        let in_tx = self.in_tx.clone();
        let _ = in_tx.send(InboundMessage::Connected(desc));

        let handle = std::thread::spawn(move || {
            io_loop(conn, out_rx, in_tx);
        });

        self.out_tx = Some(out_tx);
        self.thread = Some(handle);
        Ok(())
    }

    pub fn disconnect(&mut self) {
        if let Some(tx) = self.out_tx.take() {
            let _ = tx.send(OutboundMessage::Close);
        }
        if let Some(h) = self.thread.take() {
            let _ = h.join();
        }
        *self.description.lock() = None;
    }

    pub fn send(&self, data: Vec<u8>) -> Result<(), String> {
        let tx = self
            .out_tx
            .as_ref()
            .ok_or_else(|| "Нет активного подключения".to_string())?;
        tx.send(OutboundMessage::Send(data))
            .map_err(|e| format!("channel: {e}"))
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

fn io_loop(
    mut conn: Box<dyn Connection>,
    out_rx: Receiver<OutboundMessage>,
    in_tx: Sender<InboundMessage>,
) {
    let mut buf = vec![0u8; 8192];
    loop {
        while let Ok(msg) = out_rx.try_recv() {
            match msg {
                OutboundMessage::Close => {
                    let _ = in_tx.send(InboundMessage::Disconnected(None));
                    return;
                }
                OutboundMessage::Send(data) => {
                    if let Err(e) = conn.write(&data) {
                        let _ = in_tx.send(InboundMessage::Error(format!("write: {e}")));
                        let _ = in_tx.send(InboundMessage::Disconnected(Some(e.to_string())));
                        return;
                    } else {
                        let _ = in_tx.send(InboundMessage::TxEcho(data));
                    }
                }
            }
        }

        match conn.read(&mut buf) {
            Ok(0) => {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Ok(n) => {
                let _ = in_tx.send(InboundMessage::RxBytes(buf[..n].to_vec()));
            }
            Err(e) => {
                let _ = in_tx.send(InboundMessage::Error(format!("read: {e}")));
                let _ = in_tx.send(InboundMessage::Disconnected(Some(e.to_string())));
                return;
            }
        }
    }
}
