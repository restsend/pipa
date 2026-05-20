use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub enum Connection {
    Plain(TcpStream),
    Tls {
        tls: rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
    },
}

impl Connection {
    pub fn connect_async(
        host: String,
        port: u16,
        use_tls: bool,
    ) -> Result<mpsc::Receiver<Result<Self, String>>, String> {
        Self::connect_async_with_roots(host, port, use_tls, Vec::new())
    }

    pub fn connect_async_with_roots(
        host: String,
        port: u16,
        use_tls: bool,
        extra_roots: Vec<Vec<u8>>,
    ) -> Result<mpsc::Receiver<Result<Self, String>>, String> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = Self::connect_blocking(&host, port, use_tls, &extra_roots);
            let _ = tx.send(result);
        });
        Ok(rx)
    }

    fn connect_blocking(
        host: &str,
        port: u16,
        use_tls: bool,
        extra_roots: &[Vec<u8>],
    ) -> Result<Self, String> {
        let addr = format!("{host}:{port}");
        let stream = TcpStream::connect(&addr)
            .map_err(|e| format!("connect to {addr} failed: {e}"))?;
        stream
            .set_read_timeout(None)
            .map_err(|e| format!("set_read_timeout failed: {e}"))?;
        stream
            .set_write_timeout(None)
            .map_err(|e| format!("set_write_timeout failed: {e}"))?;
        stream
            .set_nonblocking(true)
            .map_err(|e| format!("set_nonblocking failed: {e}"))?;

        if use_tls {
            Self::wrap_tls(host, stream, extra_roots)
        } else {
            Ok(Connection::Plain(stream))
        }
    }

    fn wrap_tls(
        host: &str,
        stream: TcpStream,
        extra_roots: &[Vec<u8>],
    ) -> Result<Self, String> {
        let mut root_certs = rustls::RootCertStore::from_iter(
            webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
        );
        for cert_der in extra_roots {
            root_certs
                .add(cert_der.clone().into())
                .map_err(|e| format!("add root cert failed: {e}"))?;
        }
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_certs)
            .with_no_client_auth();
        let server_name = rustls::pki_types::ServerName::try_from(host)
            .map_err(|e| format!("invalid server name: {e}"))?
            .to_owned();
        let tls_conn =
            rustls::ClientConnection::new(std::sync::Arc::new(config), server_name)
                .map_err(|e| format!("tls handshake failed: {e}"))?;
        let tls = rustls::StreamOwned::new(tls_conn, stream);
        Ok(Connection::Tls { tls })
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), String> {
        match self {
            Connection::Plain(stream) => stream
                .set_nonblocking(nonblocking)
                .map_err(|e| format!("set_nonblocking failed: {e}")),
            Connection::Tls { tls } => tls
                .sock
                .set_nonblocking(nonblocking)
                .map_err(|e| format!("set_nonblocking failed: {e}")),
        }
    }

    pub fn raw_fd(&self) -> RawFd {
        match self {
            Connection::Plain(stream) => stream.as_raw_fd(),
            Connection::Tls { tls } => tls.sock.as_raw_fd(),
        }
    }

    pub fn is_tls(&self) -> bool {
        matches!(self, Connection::Tls { .. })
    }

    pub fn set_read_timeout(&self, dur: Option<Duration>) -> Result<(), String> {
        match self {
            Connection::Plain(stream) => stream
                .set_read_timeout(dur)
                .map_err(|e| format!("set_read_timeout failed: {e}")),
            Connection::Tls { tls } => tls
                .sock
                .set_read_timeout(dur)
                .map_err(|e| format!("set_read_timeout failed: {e}")),
        }
    }

    pub fn try_clone(&self) -> Result<Self, String> {
        match self {
            Connection::Plain(stream) => stream
                .try_clone()
                .map(Connection::Plain)
                .map_err(|e| format!("try_clone failed: {e}")),
            Connection::Tls { .. } => Err("cannot clone TLS connection".into()),
        }
    }
}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Connection::Plain(stream) => stream.read(buf),
            Connection::Tls { tls } => tls.read(buf),
        }
    }
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Connection::Plain(stream) => stream.write(buf),
            Connection::Tls { tls } => tls.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Connection::Plain(stream) => stream.flush(),
            Connection::Tls { tls } => tls.sock.flush(),
        }
    }
}

impl AsRawFd for Connection {
    fn as_raw_fd(&self) -> RawFd {
        self.raw_fd()
    }
}
