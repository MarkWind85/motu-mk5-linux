use std::fmt;
use std::net::UdpSocket;
use std::time::Duration;

use anyhow::{Context, Result};
use log::{info, debug};
use tungstenite::{connect, Message};
use tungstenite::stream::MaybeTlsStream;

const DISCOVERY_PORT: u16 = 1280;
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);
const WS_READ_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub enum ConnectionError {
    Disconnected(String),
    Protocol(String),
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionError::Disconnected(msg) => write!(f, "device disconnected: {msg}"),
            ConnectionError::Protocol(msg) => write!(f, "protocol error: {msg}"),
        }
    }
}

impl std::error::Error for ConnectionError {}

pub struct DeviceConnection {
    ws: tungstenite::WebSocket<MaybeTlsStream<std::net::TcpStream>>,
    pub device_ip: String,
}

impl DeviceConnection {
    pub fn open() -> Result<Self> {
        let ip = discover_device()?;
        info!("discovered MOTU at {ip}");

        let url = format!("ws://{}:{}", ip, DISCOVERY_PORT);
        let (mut ws, _response) = connect(&url)
            .with_context(|| format!("failed to connect to {url}"))?;

        match ws.get_mut() {
            MaybeTlsStream::Plain(s) => {
                s.set_read_timeout(Some(WS_READ_TIMEOUT)).ok();
                s.set_nodelay(true).ok();
            }
            _ => {}
        }

        let first = ws.read()
            .context("no initial message from device")?;

        match &first {
            Message::Binary(data) if data.len() >= 4 => {
                let prop_id = (data[0] as u16) << 8 | data[1] as u16;
                info!("connected to MOTU via WebSocket (first prop={prop_id:#06x})");
            }
            _ => {
                info!("connected to MOTU via WebSocket");
            }
        }

        Ok(DeviceConnection { ws, device_ip: ip })
    }

    pub fn send_property(&mut self, prop_id: u16, index: u16, data: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(4 + data.len());
        buf.extend_from_slice(&prop_id.to_be_bytes());
        buf.extend_from_slice(&index.to_be_bytes());
        buf.extend_from_slice(data);

        self.ws.send(Message::Binary(buf.into()))
            .context("failed to send property")?;
        Ok(())
    }

    pub fn recv(&mut self) -> std::result::Result<Option<(u16, u16, Vec<u8>)>, ConnectionError> {
        match self.ws.read() {
            Ok(Message::Binary(data)) if data.len() >= 4 => {
                let prop_id = (data[0] as u16) << 8 | data[1] as u16;
                let index = (data[2] as u16) << 8 | data[3] as u16;
                let payload = data[4..].to_vec();
                Ok(Some((prop_id, index, payload)))
            }
            Ok(Message::Ping(data)) => {
                let _ = self.ws.send(Message::Pong(data));
                Ok(None)
            }
            Ok(_) => Ok(None),
            Err(tungstenite::Error::Io(ref e))
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => Ok(None),
            Err(tungstenite::Error::ConnectionClosed) =>
                Err(ConnectionError::Disconnected("connection closed".into())),
            Err(tungstenite::Error::AlreadyClosed) =>
                Err(ConnectionError::Disconnected("already closed".into())),
            Err(tungstenite::Error::Io(e)) =>
                Err(ConnectionError::Disconnected(e.to_string())),
            Err(tungstenite::Error::Protocol(e)) =>
                Err(ConnectionError::Protocol(e.to_string())),
            Err(e) => {
                debug!("ws read error: {e}");
                Err(ConnectionError::Protocol(e.to_string()))
            }
        }
    }

    pub fn recv_timeout(&mut self, timeout: Duration) -> std::result::Result<Option<(u16, u16, Vec<u8>)>, ConnectionError> {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            match self.recv()? {
                Some(msg) => return Ok(Some(msg)),
                None => {}
            }
        }
        Ok(None)
    }
}

fn discover_device() -> Result<String> {
    use std::net::SocketAddr;

    let addr: SocketAddr = format!("0.0.0.0:{DISCOVERY_PORT}").parse().unwrap();
    let sock = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::DGRAM,
        Some(socket2::Protocol::UDP),
    ).context("failed to create discovery socket")?;
    sock.set_reuse_address(true).ok();
    sock.bind(&addr.into()).context("failed to bind discovery socket")?;
    sock.set_read_timeout(Some(DISCOVERY_TIMEOUT)).ok();

    let listen: UdpSocket = sock.into();

    info!("listening for MOTU discovery on UDP {DISCOVERY_PORT}...");

    let mut buf = [0u8; 4096];
    loop {
        let (len, addr) = listen.recv_from(&mut buf)
            .context("discovery timed out — is the MOTU connected?")?;

        let text = std::str::from_utf8(&buf[..len]).unwrap_or("");
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            if json.get("model").and_then(|m| m.as_str()) == Some("UltraLite-mk5") {
                let ip = json.get("ip")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&addr.ip().to_string())
                    .to_string();
                return Ok(ip);
            }
        }
    }
}
