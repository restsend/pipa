use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::sync::Arc;

#[derive(Debug)]
pub enum Connection {
    Plain(TcpStream),
    Tls {
        tls: rustls::ClientConnection,
        stream: TcpStream,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoHint {
    Read,
    Write,
    ReadWrite,
    Ready,
}

impl Connection {
    pub fn connect_nonblocking(host: &str, port: u16) -> Result<TcpStream, String> {
        let addr = format!("{host}:{port}");
        let addrs: Vec<SocketAddr> = std::net::ToSocketAddrs::to_socket_addrs(&addr)
            .map_err(|e| format!("dns resolve failed for {host}:{port}: {e}"))?
            .collect();

        for addr in addrs {
            let domain = if addr.is_ipv6() {
                libc::AF_INET6
            } else {
                libc::AF_INET
            };
            let sock = unsafe {
                libc::socket(
                    domain,
                    libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                    0,
                )
            };
            if sock < 0 {
                continue;
            }

            let (addr_ptr, addr_len) = socket_addr_to_raw(&addr);
            let ret = unsafe { libc::connect(sock, addr_ptr, addr_len) };

            if ret == 0 {
                return Ok(unsafe { TcpStream::from_raw_fd(sock) });
            }

            let errno = unsafe { *libc::__errno_location() };
            if errno == libc::EINPROGRESS {
                return Ok(unsafe { TcpStream::from_raw_fd(sock) });
            }

            unsafe { libc::close(sock) };
        }

        Err(format!("connect to {host}:{port} failed"))
    }

    pub fn check_connect(stream: &TcpStream) -> Result<(), String> {
        let mut err: i32 = 0;
        let mut err_len: u32 = std::mem::size_of::<i32>() as u32;
        let ret = unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_ERROR,
                &mut err as *mut _ as *mut _,
                &mut err_len,
            )
        };
        if ret < 0 {
            return Err("getsockopt failed".into());
        }
        if err != 0 {
            return Err(format!("connect failed: errno {err}"));
        }
        Ok(())
    }

    pub fn start_tls(
        host: &str,
        stream: TcpStream,
        extra_roots: &[Vec<u8>],
    ) -> Result<Self, String> {
        let mut root_certs =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
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
        let tls_conn = rustls::ClientConnection::new(Arc::new(config), server_name)
            .map_err(|e| format!("tls init failed: {e}"))?;
        Ok(Connection::Tls {
            tls: tls_conn,
            stream,
        })
    }

    pub fn tls_handshake_step(&mut self) -> Result<IoHint, String> {
        match self {
            Connection::Plain(_) => Ok(IoHint::Ready),
            Connection::Tls { tls, stream } => {
                if !tls.is_handshaking() {
                    return Ok(IoHint::Ready);
                }

                let mut need_read = false;
                let mut need_write = false;

                if tls.wants_read() {
                    match tls.read_tls(stream) {
                        Ok(_) => {
                            tls.process_new_packets()
                                .map_err(|e| format!("tls process error: {e}"))?;
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            need_read = true;
                        }
                        Err(e) => return Err(format!("tls read error: {e}")),
                    }
                }

                if tls.wants_write() {
                    match tls.write_tls(stream) {
                        Ok(_) => {}
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            need_write = true;
                        }
                        Err(e) => return Err(format!("tls write error: {e}")),
                    }
                }

                if !tls.is_handshaking() {
                    Ok(IoHint::Ready)
                } else {
                    match (need_read, need_write) {
                        (true, true) => Ok(IoHint::ReadWrite),
                        (true, false) => Ok(IoHint::Read),
                        (false, true) => Ok(IoHint::Write),
                        (false, false) => {
                            if tls.wants_read() && tls.wants_write() {
                                Ok(IoHint::ReadWrite)
                            } else if tls.wants_read() {
                                Ok(IoHint::Read)
                            } else {
                                Ok(IoHint::Write)
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn tls_wants_read(&self) -> bool {
        match self {
            Connection::Plain(_) => false,
            Connection::Tls { tls, .. } => tls.wants_read() || tls.is_handshaking(),
        }
    }

    pub fn tls_wants_write(&self) -> bool {
        match self {
            Connection::Plain(_) => false,
            Connection::Tls { tls, .. } => tls.wants_write() || tls.is_handshaking(),
        }
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), String> {
        match self {
            Connection::Plain(stream) => stream
                .set_nonblocking(nonblocking)
                .map_err(|e| format!("set_nonblocking failed: {e}")),
            Connection::Tls { stream, .. } => stream
                .set_nonblocking(nonblocking)
                .map_err(|e| format!("set_nonblocking failed: {e}")),
        }
    }

    pub fn raw_fd(&self) -> RawFd {
        match self {
            Connection::Plain(stream) => stream.as_raw_fd(),
            Connection::Tls { stream, .. } => stream.as_raw_fd(),
        }
    }

    pub fn is_tls(&self) -> bool {
        matches!(self, Connection::Tls { .. })
    }

    pub fn set_read_timeout(&self, dur: Option<std::time::Duration>) -> Result<(), String> {
        let stream = match self {
            Connection::Plain(s) => s,
            Connection::Tls { stream, .. } => stream,
        };
        stream
            .set_read_timeout(dur)
            .map_err(|e| format!("set_read_timeout failed: {e}"))
    }
}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Connection::Plain(stream) => stream.read(buf),
            Connection::Tls { tls, stream } => loop {
                match tls.read_tls(stream) {
                    Ok(0) => {
                        tls.process_new_packets()
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                        return tls.reader().read(buf);
                    }
                    Ok(_) => {
                        tls.process_new_packets()
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                        match tls.reader().read(buf) {
                            Ok(n) => return Ok(n),
                            Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
                            Err(e) => return Err(e),
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => match tls.reader().read(buf) {
                        Ok(n) => return Ok(n),
                        Err(e2) if e2.kind() == ErrorKind::WouldBlock => return Err(e),
                        Err(e2) => return Err(e2),
                    },
                    Err(e) => return Err(e),
                }
            },
        }
    }
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Connection::Plain(stream) => stream.write(buf),
            Connection::Tls { tls, stream } => {
                let n = tls.writer().write(buf)?;
                let _ = tls.write_tls(stream);
                Ok(n)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Connection::Plain(stream) => stream.flush(),
            Connection::Tls { stream, .. } => stream.flush(),
        }
    }
}

impl AsRawFd for Connection {
    fn as_raw_fd(&self) -> RawFd {
        self.raw_fd()
    }
}

fn socket_addr_to_raw(addr: &SocketAddr) -> (*const libc::sockaddr, u32) {
    match addr {
        SocketAddr::V4(v4) => {
            let raw: libc::sockaddr_in = libc::sockaddr_in {
                sin_family: libc::AF_INET as u16,
                sin_port: v4.port().to_be(),
                sin_addr: libc::in_addr {
                    s_addr: u32::from_ne_bytes(v4.ip().octets()),
                },
                sin_zero: [0; 8],
            };
            (
                &raw as *const _ as *const libc::sockaddr,
                std::mem::size_of::<libc::sockaddr_in>() as u32,
            )
        }
        SocketAddr::V6(v6) => {
            let raw = libc::sockaddr_in6 {
                sin6_family: libc::AF_INET6 as u16,
                sin6_port: v6.port().to_be(),
                sin6_flowinfo: v6.flowinfo(),
                sin6_addr: libc::in6_addr {
                    s6_addr: v6.ip().octets(),
                },
                sin6_scope_id: v6.scope_id(),
            };
            (
                &raw as *const _ as *const libc::sockaddr,
                std::mem::size_of::<libc::sockaddr_in6>() as u32,
            )
        }
    }
}
