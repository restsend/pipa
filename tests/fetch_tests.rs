#![cfg(feature = "fetch")]

use pipa::http::chunked::ChunkedDecoder;
use pipa::http::conn::Connection;
use pipa::http::headers::Headers;
use pipa::util::iomux::Poller;
use pipa::http::method::HttpMethod;
use pipa::http::status::HttpStatus;
use pipa::http::url::Url;
use pipa::http::ws::frame::{OpCode, WsFrame};
use pipa::http::ws::handshake::WsHandshake;

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

struct TestServer {
    port: u16,
    shutdown: Option<mpsc::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl TestServer {
    fn start_http() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel::<()>();

        let handle = thread::spawn(move || {
            listener
                .set_nonblocking(true)
                .unwrap();
            let _conns: Vec<TcpStream> = Vec::new();
            loop {
                if rx.try_recv().is_ok() {
                    return;
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut buf = [0u8; 4096];
                        match stream.read(&mut buf) {
                            Ok(n) => {
                                let req = String::from_utf8_lossy(&buf[..n]);
                                let status_line = if req.contains("GET /echo") {
                                    "HTTP/1.1 200 OK\r\n"
                                } else if req.contains("GET /redirect") {
                                    "HTTP/1.1 302 Found\r\nLocation: /echo\r\n"
                                } else if req.contains("GET /") {
                                    "HTTP/1.1 200 OK\r\n"
                                } else if req.contains("POST") {
                                    "HTTP/1.1 201 Created\r\n"
                                } else {
                                    "HTTP/1.1 404 Not Found\r\n"
                                };

                                let body = if req.contains("GET /echo") {
                                    req.as_bytes().to_vec()
                                } else if req.contains("GET /chunked") {
                                    build_chunked_response()
                                } else {
                                    b"OK".to_vec()
                                };

                                let mut response = status_line.as_bytes().to_vec();
                                response.extend_from_slice(b"Content-Length: ");
                                response.extend_from_slice(body.len().to_string().as_bytes());
                                response.extend_from_slice(b"\r\nContent-Type: text/plain\r\n\r\n");
                                response.extend_from_slice(&body);
                                let _ = stream.write_all(&response);
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                            Err(_) => {}
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => break,
                }
                thread::sleep(Duration::from_millis(10));
            }
        });

        TestServer {
            port,
            shutdown: Some(tx),
            thread: Some(handle),
        }
    }

    fn start_sse() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel::<()>();

        let handle = thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            loop {
                if rx.try_recv().is_ok() {
                    return;
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut buf = [0u8; 4096];
                        let _ = stream.read(&mut buf);
                        let resp = "HTTP/1.1 200 OK\r\n\
                                    Content-Type: text/event-stream\r\n\
                                    Cache-Control: no-cache\r\n\
                                    Connection: keep-alive\r\n\r\n";
                        let _ = stream.write_all(resp.as_bytes());

                        let events = [
                            "data: hello\n\n",
                            "data: world\n\n",
                            "event: custom\ndata: {\"key\":\"value\"}\n\n",
                            "data: line1\ndata: line2\n\n",
                            "data: goodbye\n\n",
                        ];
                        for evt in &events {
                            if rx.try_recv().is_ok() {
                                return;
                            }
                            let _ = stream.write_all(evt.as_bytes());
                            thread::sleep(Duration::from_millis(10));
                        }
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        TestServer {
            port,
            shutdown: Some(tx),
            thread: Some(handle),
        }
    }

    fn start_ws_echo() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel::<()>();

        let handle = thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            loop {
                if rx.try_recv().is_ok() {
                    return;
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(true).unwrap();
                        let mut buf = [0u8; 4096];
                        let n = loop {
                            match stream.read(&mut buf) {
                                Ok(n) => break n,
                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    thread::sleep(Duration::from_millis(10));
                                    continue;
                                }
                                Err(_) => return,
                            }
                        };
                        if n == 0 {
                            continue;
                        }
                        let req = String::from_utf8_lossy(&buf[..n]);
                        let key_line = req
                            .lines()
                            .find(|l| l.to_lowercase().starts_with("sec-websocket-key:"));
                        if let Some(kv) = key_line {
                            let key = kv
                                .splitn(2, ':')
                                .nth(1)
                                .map(|s| s.trim())
                                .unwrap_or("");
                            let accept = WsHandshake::compute_accept(key);
                            let resp = format!(
                                "HTTP/1.1 101 Switching Protocols\r\n\
                                 Upgrade: websocket\r\n\
                                 Connection: Upgrade\r\n\
                                 Sec-WebSocket-Accept: {accept}\r\n\r\n"
                            );
                            let _ = stream.write_all(resp.as_bytes());

                            let mut read_buf = [0u8; 8192];
                            loop {
                                if rx.try_recv().is_ok() {
                                    return;
                                }
                                match stream.read(&mut read_buf) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        let frames =
                                            WsFrame::parse_all(&read_buf[..n]).unwrap();
                                        for frame in &frames {
                                            let echo_frame = WsFrame {
                                                fin: frame.fin,
                                                opcode: frame.opcode,
                                                mask: None,
                                                payload: frame.payload.clone(),
                                            };
                                            let encoded = echo_frame.encode();
                                            if stream.write_all(&encoded).is_err() {
                                                break;
                                            }
                                        }
                                    }
                                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                        thread::sleep(Duration::from_millis(10));
                                    }
                                    Err(_) => break,
                                }
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        TestServer {
            port,
            shutdown: Some(tx),
            thread: Some(handle),
        }
    }

    fn stop(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

fn build_chunked_response() -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(b"5\r\nHello\r\n1\r\n \r\n6\r\nWorld!\r\n0\r\n\r\n");
    body
}

#[test]
fn test_method_parse() {
    assert_eq!(HttpMethod::from_bytes(b"GET"), Some(HttpMethod::GET));
    assert_eq!(HttpMethod::from_bytes(b"POST"), Some(HttpMethod::POST));
    assert_eq!(HttpMethod::from_bytes(b"PUT"), Some(HttpMethod::PUT));
    assert_eq!(HttpMethod::from_bytes(b"UNKNOWN"), None);
}

#[test]
fn test_status_code() {
    let s200 = HttpStatus(200);
    assert!(s200.is_success());
    assert!(!s200.is_redirect());
    assert_eq!(s200.reason_phrase(), "OK");

    let s302 = HttpStatus(302);
    assert!(s302.is_redirect());
    assert_eq!(s302.reason_phrase(), "Found");
}

#[test]
fn test_url_parse() {
    let u = Url::parse("https://example.com:8443/path?a=1").unwrap();
    assert_eq!(u.scheme, "https");
    assert_eq!(u.host, "example.com");
    assert_eq!(u.port, 8443);
    assert_eq!(u.path, "/path");
    assert_eq!(u.query, "?a=1");
    assert!(u.is_tls());
    assert_eq!(u.origin(), "https://example.com:8443");
}

#[test]
fn test_ws_frame_roundtrip() {
    let original = WsFrame::new_text(b"Hello, WebSocket!".to_vec());
    let encoded = original.encode();
    let decoded = WsFrame::parse_all(&encoded).unwrap();
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].opcode, OpCode::Text);
    assert!(decoded[0].fin);
    assert_eq!(decoded[0].payload, b"Hello, WebSocket!");
}

#[test]
fn test_ws_frame_binary() {
    let data = vec![0u8, 1, 2, 255, 128, 64];
    let frame = WsFrame::new_binary(data.clone());
    let encoded = frame.encode();
    let decoded = WsFrame::parse_all(&encoded).unwrap();
    assert_eq!(decoded[0].opcode, OpCode::Binary);
    assert_eq!(decoded[0].payload, data);
}

#[test]
fn test_ws_close_frame() {
    let frame = WsFrame::new_close(1000, "Normal closure");
    let encoded = frame.encode();
    let decoded = WsFrame::parse_all(&encoded).unwrap();
    assert_eq!(decoded[0].opcode, OpCode::Close);
    let code = u16::from_be_bytes([decoded[0].payload[0], decoded[0].payload[1]]);
    assert_eq!(code, 1000);
}

#[test]
fn test_ws_handshake_accept() {
    let key = "dGhlIHNhbXBsZSBub25jZQ==";
    let accept = WsHandshake::compute_accept(key);
    assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    assert!(WsHandshake::verify_accept(key, &accept));
}

#[test]
fn test_chunked_single() {
    let mut dec = ChunkedDecoder::new();
    let out = dec.feed(b"5\r\nHello\r\n0\r\n\r\n").unwrap();
    assert_eq!(out, b"Hello");
    assert!(dec.is_done());
}

#[test]
fn test_chunked_multi() {
    let mut dec = ChunkedDecoder::new();
    let out = dec.feed(b"6\r\nHello \r\n6\r\nWorld!\r\n0\r\n\r\n").unwrap();
    assert_eq!(out, b"Hello World!");
    assert!(dec.is_done());
}

#[test]
fn test_chunked_partial() {
    let mut dec = ChunkedDecoder::new();
    let out1 = dec.feed(b"5\r\nHel").unwrap();
    assert!(out1.is_empty());
    let out2 = dec.feed(b"lo\r\n0\r\n\r\n").unwrap();
    assert_eq!(out2, b"Hello");
    assert!(dec.is_done());
}

#[test]
fn test_headers_parse() {
    let data = b"Content-Type: application/json\r\nX-Custom: value\r\n\r\n";
    let (headers, _) = Headers::from_bytes(data).unwrap();
    assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
    assert_eq!(headers.get("x-custom").unwrap(), "value");
}

#[test]
fn test_headers_case_insensitive() {
    let mut h = Headers::new();
    h.set("Host", "example.com");
    assert_eq!(h.get("HOST").unwrap(), "example.com");
    assert_eq!(h.get("host").unwrap(), "example.com");
}

#[test]
fn test_headers_serialize() {
    let mut h = Headers::new();
    h.set("Accept", "text/html");
    h.set("Connection", "close");
    let bytes = h.to_request_bytes();
    let s = String::from_utf8_lossy(&bytes);
    assert!(s.contains("accept: text/html\r\n"));
    assert!(s.contains("connection: close\r\n"));
}

#[test]
fn test_e2e_http_get() {
    let server = TestServer::start_http();
    let port = server.port;
    let addr = format!("127.0.0.1:{port}");

    let mut stream = TcpStream::connect(&addr).unwrap();
    let request = format!("GET /echo HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    stream.flush().unwrap();

    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);

    assert!(response.contains("200 OK"), "expected 200 OK, got: {response}");
    assert!(response.contains("Content-Length:"), "missing Content-Length");
    assert!(response.contains("GET /echo"), "expected request echo in body");

    server.stop();
}

#[test]
fn test_e2e_http_chunked() {
    let mut dec = ChunkedDecoder::new();
    let data = b"5\r\nHello\r\n1\r\n \r\n6\r\nWorld!\r\n0\r\n\r\n";
    let out = dec.feed(data).unwrap();
    assert_eq!(out, b"Hello World!");
    assert!(dec.is_done());
}

#[test]
fn test_e2e_ws_handshake() {
    let server = TestServer::start_ws_echo();
    let port = server.port;
    let addr = format!("127.0.0.1:{port}");

    let mut stream = TcpStream::connect(&addr).unwrap();
    let key = WsHandshake::generate_key();
    let expected_accept = WsHandshake::compute_accept(&key);

    let request = format!(
        "GET /chat HTTP/1.1\r\n\
         Host: localhost:{port}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {key}\r\n\
         Sec-WebSocket-Version: 13\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).unwrap();
    stream.flush().unwrap();

    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);

    assert!(response.contains("101"), "expected 101, got: {response}");
    assert!(
        response.contains(&expected_accept),
        "expected accept {expected_accept}"
    );

    server.stop();
}

#[test]
fn test_iomux_create() {
    let poller = Poller::new();
    assert!(poller.is_ok(), "Poller::new() failed: {:?}", poller.err());
}

fn load_test_cert_der() -> Vec<u8> {
    fs::read("tests/test_cert.der").expect("test_cert.der not found; run: openssl x509 -in tests/test_cert.pem -outform DER -out tests/test_cert.der")
}

fn load_test_key_der() -> Vec<u8> {
    fs::read("tests/test_key_pk8.der").expect("test_key_pk8.der not found; run: openssl pkcs8 -in tests/test_key.pem -topk8 -nocrypt -outform DER -out tests/test_key_pk8.der")
}

struct TlsTestServer {
    port: u16,
    shutdown: Option<mpsc::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl TlsTestServer {
    fn start_https() -> Self {
        let cert_der = load_test_cert_der();
        let key_der = load_test_key_der();

        let certs = vec![rustls::pki_types::CertificateDer::from(cert_der)];
        let key = rustls::pki_types::PrivatePkcs8KeyDer::from(key_der);
        let key = rustls::pki_types::PrivateKeyDer::Pkcs8(key);

        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .expect("failed to build server config");

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel::<()>();

        let handle = thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            loop {
                if rx.try_recv().is_ok() {
                    return;
                }
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
                        let srv_conn = rustls::ServerConnection::new(
                            std::sync::Arc::new(server_config.clone()),
                        );
                        match srv_conn {
                            Ok(sc) => {
                                let mut tls = rustls::StreamOwned::new(sc, stream);
                                let mut buf = [0u8; 4096];
                                match tls.read(&mut buf) {
                                    Ok(0) => {}
                                    Ok(n) => {
                                        let req = String::from_utf8_lossy(&buf[..n]);
                                        let body = req.as_bytes().to_vec();
                                        let mut resp = Vec::new();
                                        if req.contains("Upgrade: websocket") {
                                            let key_line = req.lines().find(|l| {
                                                l.to_lowercase().starts_with("sec-websocket-key:")
                                            });
                                            let accept = key_line
                                                .and_then(|kv| kv.splitn(2, ':').nth(1))
                                                .map(|s| WsHandshake::compute_accept(s.trim()))
                                                .unwrap_or_default();
                                            resp.extend_from_slice(
                                                b"HTTP/1.1 101 Switching Protocols\r\n\
                                                  Upgrade: websocket\r\n\
                                                  Connection: Upgrade\r\n\
                                                  Sec-WebSocket-Accept: "
                                                    .as_ref(),
                                            );
                                            resp.extend_from_slice(accept.as_bytes());
                                            resp.extend_from_slice(b"\r\n\r\n");
                                            if tls.write_all(&resp).is_err() { return; }

                                            let mut ws_buf = [0u8; 8192];
                                            loop {
                                                if rx.try_recv().is_ok() {
                                                    return;
                                                }
                                                match tls.read(&mut ws_buf) {
                                                    Ok(0) => break,
                                                    Ok(n) => {
                                                        let frames = WsFrame::parse_all(
                                                            &ws_buf[..n],
                                                        )
                                                        .unwrap();
                                                        for frame in &frames {
                                                            let echo = WsFrame {
                                                                fin: frame.fin,
                                                                opcode: frame.opcode,
                                                                mask: None,
                                                                payload: frame.payload.clone(),
                                                            };
                                                            if tls.write_all(&echo.encode()).is_err() { break; }
                                                        }
                                                    }
                                                    Err(e)
                                                        if e.kind()
                                                            == std::io::ErrorKind::WouldBlock =>
                                                    {
                                                        thread::sleep(Duration::from_millis(10));
                                                    }
                                                    Err(_) => break,
                                                }
                                            }
                                        } else {
                                            resp.extend_from_slice(
                                                b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: "
                                                    .as_ref(),
                                            );
                                            resp.extend_from_slice(
                                                body.len().to_string().as_bytes(),
                                            );
                                            resp.extend_from_slice(b"\r\n\r\n");
                                            resp.extend_from_slice(&body);
                                            let _ = tls.write_all(&resp);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("TLS server read error: {e}");
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("TLS server connection error: {e}");
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        TlsTestServer {
            port,
            shutdown: Some(tx),
            thread: Some(handle),
        }
    }

    fn stop(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

#[test]
fn test_e2e_https_get() {
    let server = TlsTestServer::start_https();
    let port = server.port;
    let cert_der = load_test_cert_der();

    let rx = Connection::connect_async_with_roots(
        "localhost".into(),
        port,
        true,
        vec![cert_der],
    )
    .unwrap();

    let mut conn = rx.recv().unwrap().unwrap();
    conn.set_nonblocking(false).unwrap();

    let request = format!(
        "GET /echo HTTP/1.1\r\nHost: localhost:{port}\r\nConnection: close\r\n\r\n"
    );
    conn.write_all(request.as_bytes()).unwrap();
    conn.flush().unwrap();

    let mut buf = [0u8; 4096];
    let n = conn.read(&mut buf).unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);

    assert!(
        response.contains("200 OK"),
        "HTTPS: expected 200 OK, got: {response}"
    );
    assert!(response.contains("GET /echo"), "HTTPS: expected request echo");

    server.stop();
}

#[test]
fn test_e2e_https_post() {
    let server = TlsTestServer::start_https();
    let port = server.port;
    let cert_der = load_test_cert_der();

    let rx = Connection::connect_async_with_roots(
        "localhost".into(),
        port,
        true,
        vec![cert_der],
    )
    .unwrap();

    let mut conn = rx.recv().unwrap().unwrap();
    conn.set_nonblocking(false).unwrap();

    let body = "{\"key\":\"value\"}";
    let request = format!(
        "POST /api HTTP/1.1\r\nHost: localhost:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    conn.write_all(request.as_bytes()).unwrap();
    conn.flush().unwrap();

    let mut buf = [0u8; 4096];
    let n = conn.read(&mut buf).unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);

    assert!(
        response.contains("200 OK"),
        "HTTPS POST: expected 200 OK, got: {response}"
    );
    assert!(response.contains(body), "HTTPS POST: expected body echo");

    server.stop();
}

#[test]
fn test_e2e_wss_handshake() {
    let server = TlsTestServer::start_https();
    let port = server.port;
    let cert_der = load_test_cert_der();

    let rx = Connection::connect_async_with_roots(
        "localhost".into(),
        port,
        true,
        vec![cert_der],
    )
    .unwrap();

    let mut conn = rx.recv().unwrap().unwrap();
    conn.set_nonblocking(false).unwrap();

    let key = WsHandshake::generate_key();
    let expected_accept = WsHandshake::compute_accept(&key);

    let request = format!(
        "GET /chat HTTP/1.1\r\n\
         Host: localhost:{port}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {key}\r\n\
         Sec-WebSocket-Version: 13\r\n\r\n"
    );
    conn.write_all(request.as_bytes()).unwrap();
    conn.flush().unwrap();

    let mut buf = [0u8; 4096];
    let n = conn.read(&mut buf).unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);

    assert!(
        response.contains("101"),
        "WSS: expected 101, got: {response}"
    );
    assert!(
        response.contains(&expected_accept),
        "WSS: expected accept {expected_accept}"
    );

    server.stop();
}

#[test]
fn test_e2e_wss_frame_echo() {
    let server = TlsTestServer::start_https();
    let port = server.port;
    let cert_der = load_test_cert_der();

    let rx = Connection::connect_async_with_roots(
        "localhost".into(),
        port,
        true,
        vec![cert_der],
    )
    .unwrap();

    let mut conn = rx.recv().unwrap().unwrap();
    conn.set_nonblocking(false).unwrap();

    let key = WsHandshake::generate_key();
    let request = format!(
        "GET /chat HTTP/1.1\r\n\
         Host: localhost:{port}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {key}\r\n\
         Sec-WebSocket-Version: 13\r\n\r\n"
    );
    conn.write_all(request.as_bytes()).unwrap();

    let mut buf = [0u8; 4096];
    let n = conn.read(&mut buf).unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(
        response.contains("101"),
        "WSS echo: expected 101 upgrade, got: {response}"
    );

    let msg = b"Hello WSS!";
    let frame = WsFrame::new_text(msg.to_vec());
    conn.write_all(&frame.encode()).unwrap();

    conn.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
    loop {
        match conn.read(&mut buf) {
            Ok(n) if n > 0 => {
                let frames = WsFrame::parse_all(&buf[..n]).unwrap();
                if !frames.is_empty() {
                    assert_eq!(frames.len(), 1, "WSS: expected 1 echo frame");
                    assert_eq!(frames[0].opcode, OpCode::Text);
                    assert_eq!(frames[0].payload, msg);
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => panic!("WSS read error: {e}"),
            Ok(_) => break,
        }
    }
    server.stop();
}

#[test]
fn test_connection_tls_handshake() {
    let server = TlsTestServer::start_https();
    let port = server.port;
    let cert_der = load_test_cert_der();

    let rx = Connection::connect_async_with_roots(
        "localhost".into(),
        port,
        true,
        vec![cert_der],
    )
    .unwrap();

    let mut conn = rx.recv().expect("TLS connection failed");
    assert!(
        conn.is_ok(),
        "TLS connection error: {:?}",
        conn.as_ref().err()
    );
    let _conn = conn.unwrap();
    server.stop();
}

#[test]
fn test_connection_plain_connect() {
    let server = TestServer::start_http();
    let port = server.port;

    let rx = Connection::connect_async("127.0.0.1".into(), port, false).unwrap();
    let mut conn = rx.recv().unwrap().unwrap();
    conn.set_nonblocking(false).unwrap();

    let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    conn.write_all(request.as_bytes()).unwrap();
    conn.flush().unwrap();

    let mut buf = [0u8; 4096];
    let n = conn.read(&mut buf).unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("200 OK"));

    server.stop();
}

fn compile_and_run_js(js_code: &str) -> Result<String, String> {
    use pipa::runtime::context::JSContext;
    use pipa::runtime::runtime::JSRuntime;
    use pipa::compiler::codegen::OptLevel;

    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = pipa::eval(&mut ctx, js_code).map_err(|e| e.to_string())?;

    let output = if result.is_string() {
        ctx.get_atom_str(result.get_atom()).to_string()
    } else {
        format!("{result:?}")
    };
    Ok(output)
}

#[test]
fn test_js_fetch_basic() {
    let code = r#"
        let x = 1 + 2;
        x.toString();
    "#;
    let result = compile_and_run_js(code).unwrap();
    assert_eq!(result, "3");
}

#[test]
fn test_sse_parse_single() {
    let buf = &mut b"data: hello\n\n".to_vec();
    let event = pipa::builtins::eventsource::parse_sse_event(buf);
    assert!(event.is_some());
    assert_eq!(event.as_ref().unwrap().data, "hello");
    assert!(event.unwrap().event_type.is_empty());
}

#[test]
fn test_sse_parse_multiple_lines() {
    let buf = &mut b"data: line1\ndata: line2\n\n".to_vec();
    let event = pipa::builtins::eventsource::parse_sse_event(buf);
    assert!(event.is_some());
    assert_eq!(event.unwrap().data, "line1\nline2");
}

#[test]
fn test_sse_parse_custom_event() {
    let buf = &mut b"event: custom\ndata: {\"key\":\"value\"}\n\n".to_vec();
    let event = pipa::builtins::eventsource::parse_sse_event(buf);
    assert!(event.is_some());
    let e = event.unwrap();
    assert_eq!(e.data, "{\"key\":\"value\"}");
    assert_eq!(e.event_type, "custom");
}

#[test]
fn test_sse_parse_with_id() {
    let buf = &mut b"event: time\ndata: 2026-04-28T14:55:39Z\nid: 3\n\n".to_vec();
    let event = pipa::builtins::eventsource::parse_sse_event(buf);
    assert!(event.is_some());
    let e = event.unwrap();
    assert_eq!(e.data, "2026-04-28T14:55:39Z");
    assert_eq!(e.event_type, "time");
    assert_eq!(e.last_event_id, "3");
}

#[test]
fn test_sse_parse_incomplete() {
    let buf = &mut b"data: hello\n".to_vec();
    let event = pipa::builtins::eventsource::parse_sse_event(buf);
    assert!(event.is_none());
}

#[test]
fn test_sse_e2e() {
    let server = TestServer::start_sse();
    let port = server.port;

    let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    let req = format!(
        "GET /sse HTTP/1.1\r\nHost: localhost:{port}\r\nAccept: text/event-stream\r\n\r\n"
    );
    use std::io::Write;
    stream.write_all(req.as_bytes()).unwrap();
    stream.flush().unwrap();

    stream.set_read_timeout(Some(std::time::Duration::from_millis(100))).unwrap();
    let mut full_response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => full_response.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }

    let response = String::from_utf8_lossy(&full_response);

    assert!(response.contains("200 OK"), "SSE: expected 200, got: {response}");
    assert!(
        response.contains("text/event-stream"),
        "SSE: expected text/event-stream"
    );

    let body_start = response.find("\r\n\r\n").map(|p| p + 4).unwrap_or(0);
    let body = &response[body_start..];
    assert!(!body.is_empty(), "SSE: expected events in body");

    let mut events = Vec::new();
    let mut sse_buf = body.as_bytes().to_vec();
    while let Some(event) = pipa::builtins::eventsource::parse_sse_event(&mut sse_buf) {
        events.push(event);
    }
    assert_eq!(events.len(), 5, "SSE: expected 5 events, got {}", events.len());
    assert_eq!(events[0].data, "hello");
    assert_eq!(events[1].data, "world");
    assert_eq!(events[2].data, "{\"key\":\"value\"}");
    assert_eq!(events[2].event_type, "custom");
    assert_eq!(events[3].data, "line1\nline2");
    assert_eq!(events[4].data, "goodbye");

    server.stop();
}
