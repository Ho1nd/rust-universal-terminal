//! Обёртка над `serialport::SerialPort`.

use std::io;
use std::time::Duration;

use serialport::{DataBits, FlowControl as SpFlow, Parity as SpParity, SerialPort, StopBits as SpStop};

use crate::config::{FlowControl, Parity, StopBits, UartSettings};

use super::{Connection, ConnectionError};

pub struct SerialConnection {
    inner: Box<dyn SerialPort>,
    desc: String,
}

impl SerialConnection {
    pub fn open(settings: &UartSettings) -> Result<Self, ConnectionError> {
        if settings.port_name.is_empty() {
            return Err(ConnectionError::Other("COM-порт не выбран".into()));
        }
        let stop = match settings.stop_bits {
            StopBits::One | StopBits::OnePointFive => SpStop::One,
            StopBits::Two => SpStop::Two,
        };
        let parity = match settings.parity {
            Parity::None => SpParity::None,
            Parity::Even => SpParity::Even,
            Parity::Odd => SpParity::Odd,
            Parity::Mark | Parity::Space => SpParity::None,
        };
        let flow = match settings.flow {
            FlowControl::None => SpFlow::None,
            FlowControl::XonXoff => SpFlow::Software,
            FlowControl::RtsCts => SpFlow::Hardware,
            FlowControl::DsrDtr => SpFlow::Hardware,
        };
        let bits = match settings.data_bits {
            5 => DataBits::Five,
            6 => DataBits::Six,
            7 => DataBits::Seven,
            _ => DataBits::Eight,
        };

        let port = serialport::new(&settings.port_name, settings.baud)
            .data_bits(bits)
            .stop_bits(stop)
            .parity(parity)
            .flow_control(flow)
            .timeout(Duration::from_millis(100))
            .open()?;

        let desc = format!("{} @ {}", settings.port_name, settings.baud);
        Ok(Self { inner: port, desc })
    }
}

impl Connection for SerialConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.inner.read(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == io::ErrorKind::TimedOut => Ok(0),
            Err(e) => Err(e),
        }
    }

    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        self.inner.write_all(data)?;
        let _ = self.inner.flush();
        Ok(())
    }

    fn description(&self) -> String {
        self.desc.clone()
    }
}
