use std::net::TcpStream;
use std::os::unix::io::{AsRawFd, RawFd};

use crate::http::conn::{Connection, IoHint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnPhase {
    Connecting,
    TlsHandshaking,
    Ready,
    Failed,
}

#[derive(Debug)]
pub enum ConnEvent {
    NeedRead,
    NeedWrite,
    NeedReadWrite,
    Connected(Connection),
    Error(String),
}

pub struct ConnectState {
    phase: ConnPhase,
    stream: Option<TcpStream>,
    conn: Option<Connection>,
    use_tls: bool,
    tls_host: String,
    extra_roots: Vec<Vec<u8>>,
}

impl ConnectState {
    pub fn dummy() -> Self {
        ConnectState {
            phase: ConnPhase::Failed,
            stream: None,
            conn: None,
            use_tls: false,
            tls_host: String::new(),
            extra_roots: Vec::new(),
        }
    }

    pub fn new(
        host: &str,
        port: u16,
        use_tls: bool,
        extra_roots: Vec<Vec<u8>>,
    ) -> Result<Self, String> {
        let stream = Connection::connect_nonblocking(host, port)?;
        Ok(ConnectState {
            phase: ConnPhase::Connecting,
            stream: Some(stream),
            conn: None,
            use_tls,
            tls_host: host.to_string(),
            extra_roots,
        })
    }

    pub fn phase(&self) -> ConnPhase {
        self.phase
    }

    pub fn fd(&self) -> Option<RawFd> {
        self.stream
            .as_ref()
            .map(|s| s.as_raw_fd())
            .or_else(|| self.conn.as_ref().map(|c| c.raw_fd()))
    }

    pub fn wants_read(&self) -> bool {
        match self.phase {
            ConnPhase::Connecting => false,
            ConnPhase::TlsHandshaking => self
                .conn
                .as_ref()
                .map_or(false, |c| c.tls_wants_read()),
            _ => false,
        }
    }

    pub fn wants_write(&self) -> bool {
        match self.phase {
            ConnPhase::Connecting => true,
            ConnPhase::TlsHandshaking => self
                .conn
                .as_ref()
                .map_or(false, |c| c.tls_wants_write()),
            _ => false,
        }
    }

    pub fn try_advance(&mut self) -> ConnEvent {
        match self.phase {
            ConnPhase::Connecting => {
                let stream = match self.stream.as_ref() {
                    Some(s) => s,
                    None => {
                        self.phase = ConnPhase::Failed;
                        return ConnEvent::Error("no stream".into());
                    }
                };
                match Connection::check_connect(stream) {
                    Ok(()) => {
                        let stream = self.stream.take().unwrap();
                        if self.use_tls {
                            match Connection::start_tls(
                                &self.tls_host,
                                stream,
                                &self.extra_roots,
                            ) {
                                Ok(conn) => {
                                    self.conn = Some(conn);
                                    self.phase = ConnPhase::TlsHandshaking;
                                    self.try_advance()
                                }
                                Err(e) => {
                                    self.phase = ConnPhase::Failed;
                                    ConnEvent::Error(e)
                                }
                            }
                        } else {
                            let conn = Connection::Plain(stream);
                            self.phase = ConnPhase::Ready;
                            ConnEvent::Connected(conn)
                        }
                    }
                    Err(e) => {
                        self.phase = ConnPhase::Failed;
                        ConnEvent::Error(e)
                    }
                }
            }
            ConnPhase::TlsHandshaking => {
                let conn = match self.conn.as_mut() {
                    Some(c) => c,
                    None => {
                        self.phase = ConnPhase::Failed;
                        return ConnEvent::Error("no connection for tls".into());
                    }
                };
                match conn.tls_handshake_step() {
                    Ok(IoHint::Ready) => {
                        self.phase = ConnPhase::Ready;
                        let conn = self.conn.take().unwrap();
                        ConnEvent::Connected(conn)
                    }
                    Ok(IoHint::Read) => ConnEvent::NeedRead,
                    Ok(IoHint::Write) => ConnEvent::NeedWrite,
                    Ok(IoHint::ReadWrite) => ConnEvent::NeedReadWrite,
                    Err(e) => {
                        self.phase = ConnPhase::Failed;
                        ConnEvent::Error(e)
                    }
                }
            }
            _ => ConnEvent::Error("invalid state for advance".into()),
        }
    }
}
