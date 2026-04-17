//! UDP: клиент+сервер в одном. При наличии RX с нового `peer` — обновляет
//! `last_peer` и шлёт ответ туда.

use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

use crate::config::NetSettings;

use super::{Connection, ConnectionError};

pub struct UdpConnection {
    socket: UdpSocket,
    default_target: Option<SocketAddr>,
    last_peer: Option<SocketAddr>,
    desc: String,
}

impl UdpConnection {
    pub fn open(settings: &NetSettings) -> Result<Self, ConnectionError> {
        let bind_addr = if settings.udp_bind_local {
            format!("0.0.0.0:{}", settings.udp_local_port)
        } else {
            "0.0.0.0:0".into()
        };
        let socket = UdpSocket::bind(&bind_addr)
            .map_err(|e| ConnectionError::Other(format!("bind {bind_addr}: {e}")))?;
        socket.set_read_timeout(Some(Duration::from_millis(200)))?;

        let mut default_target = None;
        if !settings.host.is_empty() && settings.port != 0 {
            let addr_str = format!("{}:{}", settings.host, settings.port);
            if let Ok(mut it) = addr_str.to_socket_addrs() {
                default_target = it.next();
            }
        }
        let desc = match default_target {
            Some(a) => format!("UDP local {bind_addr} ↔ {a}"),
            None => format!("UDP listen {bind_addr}"),
        };
        Ok(Self {
            socket,
            default_target,
            last_peer: None,
            desc,
        })
    }
}

impl Connection for UdpConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.socket.recv_from(buf) {
            Ok((n, addr)) => {
                self.last_peer = Some(addr);
                Ok(n)
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => Ok(0),
            Err(e) => Err(e),
        }
    }

    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        let target = self
            .last_peer
            .or(self.default_target)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "UDP: нет адресата"))?;
        self.socket.send_to(data, target).map(|_| ())
    }

    fn description(&self) -> String {
        self.desc.clone()
    }
}
