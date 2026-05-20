use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::io::RawFd;
use std::sync::mpsc;

use crate::http::conn::Connection;
use crate::http::headers::Headers;
use crate::http::status::HttpStatus;
use crate::http::url::Url;
use crate::http::ws::frame::{OpCode, WsFrame};
use crate::http::ws::handshake::WsHandshake;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsState {
    Connecting,
    Handshake,
    Open,
    Closing,
    Closed,
}

#[derive(Debug, Clone)]
pub enum WsEvent {
    Open,
    Message(Vec<u8>, bool),
    Close(u16, String),
    Error(String),
}

pub struct WsConnection {
    pub url: Url,
    pub state: WsState,
    conn: Option<Connection>,
    connect_rx: Option<mpsc::Receiver<Result<Connection, String>>>,
    write_buf: Vec<u8>,
    write_pos: usize,
    read_buf: [u8; 8192],
    read_data: Vec<u8>,
    key: String,
    pending_frames: VecDeque<WsFrame>,
    close_code: u16,
    close_reason: String,
    pub ready_state: u8,
}

impl WsConnection {
    pub fn new(url: Url) -> Self {
        WsConnection {
            url,
            state: WsState::Connecting,
            conn: None,
            connect_rx: None,
            write_buf: Vec::new(),
            write_pos: 0,
            read_buf: [0u8; 8192],
            read_data: Vec::new(),
            key: String::new(),
            pending_frames: VecDeque::new(),
            close_code: 0,
            close_reason: String::new(),
            ready_state: 0,
        }
    }

    pub fn fd(&self) -> Option<RawFd> {
        self.conn.as_ref().map(|c| c.raw_fd())
    }

    pub fn set_connect_rx(&mut self, rx: mpsc::Receiver<Result<Connection, String>>) {
        self.connect_rx = Some(rx);
        self.state = WsState::Connecting;
    }

    pub fn try_advance(&mut self) -> Result<Option<WsEvent>, String> {
        loop {
            match self.state {
                WsState::Connecting => match self.connect_rx.as_ref().unwrap().try_recv() {
                    Ok(result) => {
                        let conn = result?;
                        conn.set_nonblocking(true)?;
                        self.conn = Some(conn);
                        self.key = WsHandshake::generate_key();
                        self.build_handshake_request();
                        self.state = WsState::Handshake;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        return Ok(None);
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        return Err("connect thread disconnected".into());
                    }
                },

                WsState::Handshake => {
                    if !self.write_buf.is_empty() {
                        let conn = self.conn.as_mut().unwrap();
                        let remaining = &self.write_buf[self.write_pos..];
                        if !remaining.is_empty() {
                            match conn.write(remaining) {
                                Ok(n) => {
                                    self.write_pos += n;
                                    if self.write_pos >= self.write_buf.len() {
                                        self.write_buf.clear();
                                        self.write_pos = 0;
                                    } else {
                                        return Ok(None);
                                    }
                                }
                                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                                    return Ok(None);
                                }
                                Err(e) => return Err(format!("ws handshake write: {e}")),
                            }
                        }
                    }

                    let conn = self.conn.as_mut().unwrap();
                    match conn.read(&mut self.read_buf) {
                        Ok(0) => return Err("connection closed during handshake".into()),
                        Ok(n) => {
                            self.read_data.extend_from_slice(&self.read_buf[..n]);
                            if let Some(pos) =
                                self.read_data.windows(4).position(|w| w == b"\r\n\r\n")
                            {
                                let header_data = &self.read_data[..pos + 4];
                                let (headers, _) = Headers::from_bytes(header_data)?;
                                let status_line_end = self
                                    .read_data
                                    .windows(2)
                                    .position(|w| w == b"\r\n")
                                    .unwrap_or(0);
                                let status_line = &self.read_data[..status_line_end];
                                let code = if status_line.len() >= 12 {
                                    let s = &status_line[9..12];
                                    String::from_utf8_lossy(s).parse::<u16>().unwrap_or(0)
                                } else {
                                    0
                                };
                                let status = HttpStatus(code);
                                let accept = WsHandshake::validate_response(status, &headers)?;
                                if !WsHandshake::verify_accept(&self.key, &accept) {
                                    return Err("WebSocket accept mismatch".into());
                                }
                                self.state = WsState::Open;
                                self.ready_state = 1;
                                self.read_data.clear();
                                return Ok(Some(WsEvent::Open));
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            return Ok(None);
                        }
                        Err(e) => return Err(format!("ws handshake read: {e}")),
                    }
                }

                WsState::Open => {
                    if let Some(frame) = self.pending_frames.pop_front() {
                        self.write_buf = frame.encode();
                        self.write_pos = 0;
                    }

                    if !self.write_buf.is_empty() {
                        let conn = self.conn.as_mut().unwrap();
                        let remaining = &self.write_buf[self.write_pos..];
                        if !remaining.is_empty() {
                            match conn.write(remaining) {
                                Ok(n) => {
                                    self.write_pos += n;
                                }
                                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                                    return Ok(None);
                                }
                                Err(e) => return Err(format!("ws write error: {e}")),
                            }
                        }
                        if self.write_pos >= self.write_buf.len() {
                            self.write_buf.clear();
                            self.write_pos = 0;
                        }
                        if !self.pending_frames.is_empty() {
                            return Ok(None);
                        }
                    }

                    let conn = self.conn.as_mut().unwrap();
                    match conn.read(&mut self.read_buf) {
                        Ok(0) => {
                            self.state = WsState::Closed;
                            self.ready_state = 3;
                            return Ok(Some(WsEvent::Close(1006, "connection closed".into())));
                        }
                        Ok(n) => {
                            self.read_data.extend_from_slice(&self.read_buf[..n]);
                            let frames = WsFrame::parse_all(&self.read_data)?;
                            if !frames.is_empty() {
                                let consumed = self.calculate_consumed(&frames);
                                self.read_data.drain(..consumed);
                                for frame in frames {
                                    match frame.opcode {
                                        OpCode::Text | OpCode::Binary => {
                                            let is_text = frame.opcode == OpCode::Text;
                                            return Ok(Some(WsEvent::Message(
                                                frame.payload,
                                                is_text,
                                            )));
                                        }
                                        OpCode::Ping => {
                                            let pong = WsFrame::new_pong(frame.payload);
                                            self.pending_frames.push_back(pong);
                                            return Ok(None);
                                        }
                                        OpCode::Close => {
                                            let (code, reason) =
                                                Self::parse_close_payload(&frame.payload);
                                            self.close_code = code;
                                            self.close_reason = reason.clone();
                                            let close_frame = WsFrame::new_close(code, &reason);
                                            self.pending_frames.push_back(close_frame);
                                            self.state = WsState::Closing;
                                            self.ready_state = 2;
                                            return Ok(None);
                                        }
                                        OpCode::Pong => {}
                                        OpCode::Continuation => {}
                                    }
                                }
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            return Ok(None);
                        }
                        Err(e) => return Err(format!("ws read error: {e}")),
                    }
                }

                WsState::Closing => {
                    if let Some(frame) = self.pending_frames.pop_front() {
                        self.write_buf = frame.encode();
                        self.write_pos = 0;
                    }
                    if !self.write_buf.is_empty() {
                        let conn = self.conn.as_mut().unwrap();
                        let remaining = &self.write_buf[self.write_pos..];
                        if !remaining.is_empty() {
                            let _ = conn.write(remaining);
                        }
                        self.write_buf.clear();
                        self.write_pos = 0;
                    }
                    let conn = self.conn.as_mut().unwrap();
                    let _ = conn.read(&mut self.read_buf);
                    self.state = WsState::Closed;
                    self.ready_state = 3;
                    return Ok(Some(WsEvent::Close(
                        self.close_code,
                        self.close_reason.clone(),
                    )));
                }

                WsState::Closed => {
                    return Ok(None);
                }
            }
        }
    }

    pub fn send_text(&mut self, data: &str) {
        let frame = WsFrame::new_text(data.as_bytes().to_vec());
        self.pending_frames.push_back(frame);
    }

    pub fn send_binary(&mut self, data: &[u8]) {
        let frame = WsFrame::new_binary(data.to_vec());
        self.pending_frames.push_back(frame);
    }

    pub fn close(&mut self, code: u16, reason: &str) {
        if self.state == WsState::Open {
            let frame = WsFrame::new_close(code, reason);
            self.pending_frames.push_back(frame);
            self.state = WsState::Closing;
            self.ready_state = 2;
            self.close_code = code;
            self.close_reason = reason.to_string();
        }
    }

    pub fn wants_read(&self) -> bool {
        matches!(
            self.state,
            WsState::Connecting | WsState::Handshake | WsState::Open
        )
    }

    pub fn wants_write(&self) -> bool {
        matches!(
            self.state,
            WsState::Handshake | WsState::Open | WsState::Closing
        ) && (!self.write_buf.is_empty() || !self.pending_frames.is_empty())
    }

    fn build_handshake_request(&mut self) {
        let host = format!(
            "{}:{}",
            self.url.host,
            if self.url.port != 80 && self.url.port != 443 {
                self.url.port
            } else {
                0
            }
        );
        let host = if host.ends_with(":0") {
            self.url.host.clone()
        } else {
            host
        };
        let path = self.url.request_target();
        let mut headers = WsHandshake::build_request(&host, &path, &self.key);
        if !headers.contains("user-agent") {
            headers.set("User-Agent", "pipa/0.1");
        }
        let mut buf = Vec::new();
        buf.extend_from_slice(b"GET ");
        buf.extend_from_slice(path.as_bytes());
        buf.extend_from_slice(b" HTTP/1.1\r\n");
        buf.extend_from_slice(headers.to_request_bytes().as_ref());
        buf.extend_from_slice(b"\r\n");
        self.write_buf = buf;
        self.write_pos = 0;
    }

    fn parse_close_payload(payload: &[u8]) -> (u16, String) {
        if payload.len() >= 2 {
            let code = u16::from_be_bytes([payload[0], payload[1]]);
            let reason = if payload.len() > 2 {
                String::from_utf8_lossy(&payload[2..]).to_string()
            } else {
                String::new()
            };
            (code, reason)
        } else {
            (1005, String::new())
        }
    }

    fn calculate_consumed(&self, frames: &[WsFrame]) -> usize {
        let mut total = 0usize;
        for frame in frames {
            let mut frame_size = 2;
            let payload_len = frame.payload.len();
            if payload_len >= 126 && payload_len <= 0xFFFF {
                frame_size += 2;
            } else if payload_len > 0xFFFF {
                frame_size += 8;
            }
            if frame.mask.is_some() {
                frame_size += 4;
            }
            frame_size += payload_len;
            total += frame_size;
        }
        total
    }
}
