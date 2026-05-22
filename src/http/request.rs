use std::io::{ErrorKind, Read, Write};
use std::os::unix::io::RawFd;

use crate::http::body_reader::{BodyMode, BodyReader};
use crate::http::compression::Decompressor;
use crate::http::conn::Connection;
use crate::http::headers::Headers;
use crate::http::method::HttpMethod;
use crate::http::response::HttpResponse;
use crate::http::status::HttpStatus;
use crate::http::url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestState {
    Connecting,
    WritingRequest,
    ReadingStatus,
    ReadingHeaders,
    ReadingBody,
    Done,
}

#[derive(Debug)]
pub enum RequestEvent {
    NeedRead,
    NeedWrite,
    Complete(HttpResponse),
    Error(String),
}

pub struct HttpRequest {
    pub url: Url,
    pub state: RequestState,
    pub method: HttpMethod,
    pub req_headers: Headers,
    pub body: Option<Vec<u8>>,
    pub conn: Option<Connection>,
    write_buf: Vec<u8>,
    write_pos: usize,
    read_buf: [u8; 8192],
    read_data: Vec<u8>,
    parse_buf: Vec<u8>,
    resp_headers: Option<Headers>,
    resp_status: Option<(u16, String)>,
    body_reader: Option<BodyReader>,
    decompressor: Option<Decompressor>,
    pub redirect_count: u32,
    max_redirects: u32,
}

impl HttpRequest {
    pub fn dummy() -> Self {
        HttpRequest {
            url: Url {
                scheme: String::new(),
                host: String::new(),
                port: 0,
                path: String::new(),
                query: String::new(),
                full: String::new(),
            },
            state: RequestState::Done,
            method: HttpMethod::GET,
            req_headers: Headers::new(),
            body: None,
            conn: None,
            write_buf: Vec::new(),
            write_pos: 0,
            read_buf: [0u8; 8192],
            read_data: Vec::new(),
            parse_buf: Vec::new(),
            resp_headers: None,
            resp_status: None,
            body_reader: None,
            decompressor: None,
            redirect_count: 0,
            max_redirects: 20,
        }
    }

    pub fn new(url: Url, method: HttpMethod, req_headers: Headers, body: Option<Vec<u8>>) -> Self {
        HttpRequest {
            url,
            state: RequestState::Connecting,
            method,
            req_headers,
            body,
            conn: None,
            write_buf: Vec::new(),
            write_pos: 0,
            read_buf: [0u8; 8192],
            read_data: Vec::new(),
            parse_buf: Vec::new(),
            resp_headers: None,
            resp_status: None,
            body_reader: None,
            decompressor: None,
            redirect_count: 0,
            max_redirects: 20,
        }
    }

    pub fn fd(&self) -> Option<RawFd> {
        self.conn.as_ref().map(|c| c.raw_fd())
    }

    pub fn max_redirects(&self) -> u32 {
        self.max_redirects
    }

    pub fn try_advance(&mut self) -> Result<RequestEvent, String> {
        loop {
            match self.state {
                RequestState::WritingRequest => {
                    let conn = self.conn.as_mut().unwrap();
                    let remaining = &self.write_buf[self.write_pos..];
                    if remaining.is_empty() {
                        self.state = RequestState::ReadingStatus;
                        self.read_data.clear();
                        continue;
                    }
                    match conn.write(remaining) {
                        Ok(n) => {
                            self.write_pos += n;
                            if self.write_pos >= self.write_buf.len() {
                                self.state = RequestState::ReadingStatus;
                                self.read_data.clear();
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            return Ok(RequestEvent::NeedWrite);
                        }
                        Err(e) => {
                            return Err(format!("write error: {e}"));
                        }
                    }
                }

                RequestState::ReadingStatus => {
                    let conn = self.conn.as_mut().unwrap();
                    match conn.read(&mut self.read_buf) {
                        Ok(0) => {
                            return Err("connection closed while reading status".into());
                        }
                        Ok(n) => {
                            self.read_data.extend_from_slice(&self.read_buf[..n]);
                            if let Some(headers_end) =
                                self.read_data.windows(4).position(|w| w == b"\r\n\r\n")
                            {
                                let line_end = self
                                    .read_data
                                    .windows(2)
                                    .position(|w| w == b"\r\n")
                                    .unwrap_or(0);
                                let status =
                                    self.parse_status_from_bytes(&self.read_data[..line_end])?;
                                self.resp_status = Some(status);
                                let header_bytes = &self.read_data[line_end + 2..headers_end + 4];
                                let (headers, _) = Headers::from_bytes(header_bytes)?;
                                let body_mode = self.determine_body_mode(&headers);
                                self.resp_headers = Some(headers.clone());
                                let enc = headers.get("content-encoding").map(|s| s.to_string());
                                self.decompressor = Some(Decompressor::new(enc.as_deref()));
                                self.body_reader = Some(BodyReader::new(body_mode));
                                self.parse_buf = self.read_data[headers_end + 4..].to_vec();
                                self.state = RequestState::ReadingBody;
                                continue;
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            return Ok(RequestEvent::NeedRead);
                        }
                        Err(e) => {
                            return Err(format!("read error: {e}"));
                        }
                    }
                }

                RequestState::ReadingHeaders => {
                    let conn = self.conn.as_mut().unwrap();
                    match conn.read(&mut self.read_buf) {
                        Ok(0) => {
                            return Err("connection closed while reading headers".into());
                        }
                        Ok(n) => {
                            self.read_data.extend_from_slice(&self.read_buf[..n]);
                            if let Some(headers_end) =
                                self.read_data.windows(4).position(|w| w == b"\r\n\r\n")
                            {
                                let header_data = &self.read_data[..headers_end + 2];
                                let (headers, _) = Headers::from_bytes(header_data)?;
                                let body_mode = self.determine_body_mode(&headers);
                                self.resp_headers = Some(headers.clone());
                                let enc = headers.get("content-encoding").map(|s| s.to_string());
                                self.decompressor = Some(Decompressor::new(enc.as_deref()));
                                self.body_reader = Some(BodyReader::new(body_mode));
                                self.parse_buf = self.read_data[headers_end + 4..].to_vec();
                                self.state = RequestState::ReadingBody;
                                continue;
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            return Ok(RequestEvent::NeedRead);
                        }
                        Err(e) => {
                            return Err(format!("read error: {e}"));
                        }
                    }
                }

                RequestState::ReadingBody => {
                    let conn = self.conn.as_mut().unwrap();

                    if !self.parse_buf.is_empty() {
                        let data = std::mem::take(&mut self.parse_buf);
                        let _ = self.body_reader.as_mut().unwrap().feed(&data)?;
                    }

                    if self.body_reader.as_ref().unwrap().is_done() {
                        return self.finalize_response();
                    }

                    match conn.read(&mut self.read_buf) {
                        Ok(0) => {
                            self.body_reader.as_mut().unwrap().finish();
                            return self.finalize_response();
                        }
                        Ok(n) => {
                            let _ = self
                                .body_reader
                                .as_mut()
                                .unwrap()
                                .feed(&self.read_buf[..n])?;
                            if self.body_reader.as_ref().unwrap().is_done() {
                                return self.finalize_response();
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            return Ok(RequestEvent::NeedRead);
                        }
                        Err(e) => {
                            return Err(format!("read error: {e}"));
                        }
                    }
                }

                RequestState::Done => {
                    return Err("request already completed".into());
                }

                RequestState::Connecting => {
                    unreachable!("Connecting should have resolved to another state");
                }
            }
        }
    }

    fn parse_status_from_bytes(&self, line: &[u8]) -> Result<(u16, String), String> {
        if line.len() < 12 || !line.starts_with(b"HTTP/") {
            return Err(format!(
                "malformed status line: {:?}",
                String::from_utf8_lossy(line)
            ));
        }
        let http_end = line
            .iter()
            .position(|&b| b == b' ')
            .ok_or("no space after HTTP/x.y")?;
        let status_start = http_end + 1;
        if status_start >= line.len() {
            return Err("missing status code".into());
        }
        let status_end = line[status_start..]
            .iter()
            .position(|&b| b == b' ')
            .map(|p| status_start + p)
            .unwrap_or(line.len());
        let code_str = String::from_utf8_lossy(&line[status_start..status_end]);
        let code: u16 = code_str
            .parse()
            .map_err(|e| format!("invalid status code: {e}"))?;
        let reason = if status_end < line.len() {
            String::from_utf8_lossy(&line[status_end + 1..])
                .trim()
                .to_string()
        } else {
            String::new()
        };
        Ok((code, reason))
    }

    fn determine_body_mode(&self, headers: &Headers) -> BodyMode {
        if let Some(len_str) = headers.get("content-length") {
            if let Ok(len) = len_str.parse::<usize>() {
                return BodyMode::ContentLength(len);
            }
        }
        if let Some(te) = headers.get("transfer-encoding") {
            if te.contains("chunked") {
                return BodyMode::Chunked;
            }
        }
        BodyMode::ConnectionClose
    }

    pub fn build_request(&mut self) {
        let target = self.url.request_target();
        let mut buf = Vec::new();
        buf.extend_from_slice(self.method.as_str().as_bytes());
        buf.extend_from_slice(b" ");
        buf.extend_from_slice(target.as_bytes());
        buf.extend_from_slice(b" HTTP/1.1\r\n");
        buf.extend_from_slice(self.req_headers.to_request_bytes().as_ref());
        if !self.req_headers.contains("host") {
            buf.extend_from_slice(b"Host: ");
            buf.extend_from_slice(self.url.host.as_bytes());
            if (self.url.port != 80 && self.url.port != 443)
                || (self.url.port == 80 && self.url.is_tls())
                || (self.url.port == 443 && !self.url.is_tls())
            {
                buf.extend_from_slice(b":");
                buf.extend_from_slice(self.url.port.to_string().as_bytes());
            }
            buf.extend_from_slice(b"\r\n");
        }
        if !self.req_headers.contains("user-agent") {
            buf.extend_from_slice(b"User-Agent: pipa/0.1\r\n");
        }
        if !self.req_headers.contains("accept") {
            buf.extend_from_slice(b"Accept: */*\r\n");
        }
        if let Some(ref body) = self.body {
            if !self.req_headers.contains("content-length") {
                buf.extend_from_slice(b"Content-Length: ");
                buf.extend_from_slice(body.len().to_string().as_bytes());
                buf.extend_from_slice(b"\r\n");
            }
            buf.extend_from_slice(b"\r\n");
            buf.extend_from_slice(body);
        } else {
            buf.extend_from_slice(b"\r\n");
        }
        self.write_buf = buf;
        self.write_pos = 0;
    }

    fn finalize_response(&mut self) -> Result<RequestEvent, String> {
        self.state = RequestState::Done;
        let (code, status_text) = self.resp_status.take().unwrap_or((0, String::new()));
        let headers = self.resp_headers.take().unwrap_or_default();
        let mut body_reader = self
            .body_reader
            .take()
            .unwrap_or_else(|| BodyReader::new(BodyMode::None));
        let body = body_reader.take_body();

        let decompressed = if let Some(ref mut decomp) = self.decompressor {
            decomp.decompress(&body)?
        } else {
            body
        };

        let mut final_reader = BodyReader::new(BodyMode::ContentLength(decompressed.len()));
        let _ = final_reader.feed(&decompressed)?;

        let resp = HttpResponse::new(
            HttpStatus(code),
            status_text,
            headers,
            final_reader,
            self.url.full.clone(),
        );
        Ok(RequestEvent::Complete(resp))
    }

    pub fn wants_read(&self) -> bool {
        matches!(
            self.state,
            RequestState::ReadingStatus
                | RequestState::ReadingHeaders
                | RequestState::ReadingBody
                | RequestState::Connecting
        )
    }

    pub fn wants_write(&self) -> bool {
        matches!(self.state, RequestState::WritingRequest)
    }
}
