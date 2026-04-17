//! TCP-клиент. Блокирующий сокет с read-таймаутом.

use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::config::NetSettings;

use super::{Connection, ConnectionError};

pub struct TcpConnection {
    stream: TcpStream,
    desc: String,
}

impl TcpConnection {
    pub fn connect(settings: &NetSettings) -> Result<Self, ConnectionError> {
        let addr_str = format!("{}:{}", settings.host, settings.port);
        let mut addrs = addr_str
            .to_socket_addrs()
            .map_err(|e| ConnectionError::Other(format!("resolve {addr_str}: {e}")))?;
        let first = addrs
            .next()
            .ok_or_else(|| ConnectionError::Other(format!("no address for {addr_str}")))?;
        let stream = TcpStream::connect_timeout(&first, Duration::from_secs(3))?;
        stream.set_read_timeout(Some(Duration::from_millis(200)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        stream.set_nodelay(true).ok();
        Ok(Self {
            stream,
            desc: format!("TCP {addr_str}"),
        })
    }
}

impl Connection for TcpConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.stream.read(buf) {
            Ok(0) => Err(io::Error::new(io::ErrorKind::ConnectionAborted, "peer closed")),
            Ok(n) => Ok(n),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => Ok(0),
            Err(e) => Err(e),
        }
    }

    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        self.stream.write_all(data)?;
        self.stream.flush()
    }

    fn description(&self) -> String {
        self.desc.clone()
    }
}
