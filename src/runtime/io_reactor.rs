use std::collections::{HashMap, VecDeque};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::io::RawFd;

use crate::http::conn::Connection;
use crate::http::connect_state::{ConnEvent, ConnectState};
use crate::http::headers::Headers;
use crate::http::method::HttpMethod;
use crate::http::request::{HttpRequest, RequestEvent, RequestState};
use crate::http::response::HttpResponse;
use crate::http::status::HttpStatus;
use crate::http::url::Url;
use crate::http::ws::frame::{OpCode, WsFrame};
use crate::http::ws::handshake::WsHandshake;
use crate::util::iomux::Poller;

use super::context::JSContext;
use super::extension::MacroTaskExtension;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollEvent {
    Readable,
    Writable,
    Error,
}

pub enum ReactorTask {
    Fetch(FetchTask),
    Ws(WsTask),
    Sse(SseTask),
}

pub struct FetchTask {
    phase: FetchPhase,
    promise_ptr: usize,
    url: Url,
    method: HttpMethod,
    req_headers: Headers,
    body: Option<Vec<u8>>,
}

pub enum FetchPhase {
    Connecting(ConnectState),
    Active(HttpRequest),
}

pub struct WsTask {
    phase: WsPhase,
    ws_obj_ptr: usize,
    promise_ptr: usize,
    url: Url,
}

pub enum WsPhase {
    Connecting(ConnectState),
    Handshake {
        conn: Connection,
        key: String,
        write_buf: Vec<u8>,
        write_pos: usize,
        read_data: Vec<u8>,
    },
    Open {
        conn: Connection,
        read_buf: [u8; 8192],
        read_data: Vec<u8>,
        pending_out: VecDeque<WsFrame>,
        closing: bool,
        close_code: u16,
        close_reason: String,
    },
}

pub struct SseTask {
    phase: SsePhase,
    es_obj_ptr: usize,
    promise_ptr: usize,
    url: Url,
}

pub enum SsePhase {
    Connecting(ConnectState),
    WritingRequest {
        conn: Connection,
        write_buf: Vec<u8>,
        write_pos: usize,
    },
    ReadingHeaders {
        conn: Connection,
        read_buf: [u8; 8192],
        read_data: Vec<u8>,
    },
    Streaming {
        conn: Connection,
        read_buf: [u8; 8192],
        read_data: Vec<u8>,
        sse_buf: Vec<u8>,
    },
}

impl ReactorTask {
    pub fn fd(&self) -> Option<RawFd> {
        match self {
            ReactorTask::Fetch(t) => match &t.phase {
                FetchPhase::Connecting(cs) => cs.fd(),
                FetchPhase::Active(req) => req.fd(),
            },
            ReactorTask::Ws(t) => match &t.phase {
                WsPhase::Connecting(cs) => cs.fd(),
                WsPhase::Handshake { conn, .. } => Some(conn.raw_fd()),
                WsPhase::Open { conn, .. } => Some(conn.raw_fd()),
            },
            ReactorTask::Sse(t) => match &t.phase {
                SsePhase::Connecting(cs) => cs.fd(),
                SsePhase::WritingRequest { conn, .. } => Some(conn.raw_fd()),
                SsePhase::ReadingHeaders { conn, .. } => Some(conn.raw_fd()),
                SsePhase::Streaming { conn, .. } => Some(conn.raw_fd()),
            },
        }
    }

    pub fn wants_read(&self) -> bool {
        match self {
            ReactorTask::Fetch(t) => match &t.phase {
                FetchPhase::Connecting(cs) => cs.wants_read(),
                FetchPhase::Active(req) => req.wants_read(),
            },
            ReactorTask::Ws(t) => match &t.phase {
                WsPhase::Connecting(cs) => cs.wants_read(),
                WsPhase::Handshake { .. } => true,
                WsPhase::Open { .. } => true,
            },
            ReactorTask::Sse(t) => match &t.phase {
                SsePhase::Connecting(cs) => cs.wants_read(),
                SsePhase::WritingRequest { .. } => false,
                SsePhase::ReadingHeaders { .. } => true,
                SsePhase::Streaming { .. } => true,
            },
        }
    }

    pub fn wants_write(&self) -> bool {
        match self {
            ReactorTask::Fetch(t) => match &t.phase {
                FetchPhase::Connecting(cs) => cs.wants_write(),
                FetchPhase::Active(req) => req.wants_write(),
            },
            ReactorTask::Ws(t) => match &t.phase {
                WsPhase::Connecting(cs) => cs.wants_write(),
                WsPhase::Handshake {
                    write_buf,
                    write_pos,
                    ..
                } => *write_pos < write_buf.len(),
                WsPhase::Open { pending_out, .. } => !pending_out.is_empty(),
            },
            ReactorTask::Sse(t) => match &t.phase {
                SsePhase::Connecting(cs) => cs.wants_write(),
                SsePhase::WritingRequest {
                    write_buf,
                    write_pos,
                    ..
                } => *write_pos < write_buf.len(),
                SsePhase::ReadingHeaders { .. } => false,
                SsePhase::Streaming { .. } => false,
            },
        }
    }

    fn task_type(&self) -> &str {
        match self {
            ReactorTask::Fetch(_) => "fetch",
            ReactorTask::Ws(_) => "ws",
            ReactorTask::Sse(_) => "sse",
        }
    }
}

impl FetchTask {
    pub fn new(
        url: Url,
        method: HttpMethod,
        req_headers: Headers,
        body: Option<Vec<u8>>,
        promise_ptr: usize,
    ) -> Result<Self, String> {
        let use_tls = url.is_tls();
        let cs = ConnectState::new(&url.host, url.port, use_tls, Vec::new())?;
        Ok(FetchTask {
            phase: FetchPhase::Connecting(cs),
            promise_ptr,
            url,
            method,
            req_headers,
            body,
        })
    }
}

impl WsTask {
    pub fn new(url: Url, ws_obj_ptr: usize, promise_ptr: usize) -> Result<Self, String> {
        let use_tls = url.is_tls();
        let cs = ConnectState::new(&url.host, url.port, use_tls, Vec::new())?;
        Ok(WsTask {
            phase: WsPhase::Connecting(cs),
            ws_obj_ptr,
            promise_ptr,
            url,
        })
    }
}

impl SseTask {
    pub fn new(url: Url, es_obj_ptr: usize, promise_ptr: usize) -> Result<Self, String> {
        let use_tls = url.is_tls();
        let cs = ConnectState::new(&url.host, url.port, use_tls, Vec::new())?;
        Ok(SseTask {
            phase: SsePhase::Connecting(cs),
            es_obj_ptr,
            promise_ptr,
            url,
        })
    }
}

pub struct IoReactor {
    poller: Poller,
    tasks: HashMap<u64, ReactorTask>,
    next_id: u64,
}

impl IoReactor {
    pub fn new() -> Result<Self, String> {
        let poller = Poller::new()?;
        Ok(IoReactor {
            poller,
            tasks: HashMap::new(),
            next_id: 1,
        })
    }

    pub fn get_from_ctx(ctx: &mut JSContext) -> Option<&mut IoReactor> {
        let el = ctx.event_loop_mut();
        for ext in &mut el.extensions {
            let any = ext.as_any_mut();
            if any.is::<IoReactor>() {
                return any.downcast_mut::<IoReactor>();
            }
        }
        None
    }

    pub fn register(&mut self, task: ReactorTask) -> Result<u64, String> {
        let id = self.next_id;
        self.next_id += 1;

        if let Some(fd) = task.fd() {
            self.poller
                .register(fd, task.wants_read(), task.wants_write())?;
        }

        self.tasks.insert(id, task);
        Ok(id)
    }

    fn unregister(&mut self, id: u64) -> Option<ReactorTask> {
        let task = self.tasks.remove(&id);
        if let Some(ref t) = task {
            if let Some(fd) = t.fd() {
                let _ = self.poller.unregister(fd);
            }
        }
        task
    }

    fn reregister(&mut self, id: u64, task: ReactorTask) -> Result<(), String> {
        let old = self.tasks.remove(&id);
        let old_fd = old.as_ref().and_then(|t| t.fd());
        let new_fd = task.fd();

        if let Some(fd) = new_fd {
            let _ = self.poller.unregister(fd);
            self.poller
                .register(fd, task.wants_read(), task.wants_write())?;
        } else if let Some(ofd) = old_fd {
            let _ = self.poller.unregister(ofd);
        }

        self.tasks.insert(id, task);
        Ok(())
    }

    fn poll(&mut self, timeout_ms: i32) -> Result<Vec<(u64, String, PollEvent)>, String> {
        if self.tasks.is_empty() {
            return Ok(Vec::new());
        }

        let events = self.poller.wait(timeout_ms)?;
        let mut results = Vec::new();

        for event in &events {
            let ids_to_check: Vec<u64> = self
                .tasks
                .iter()
                .filter(|(_, t)| t.fd() == Some(event.fd))
                .map(|(id, _)| *id)
                .collect();

            for id in ids_to_check {
                let task_type = self.tasks.get(&id).map(|t| t.task_type().to_string());
                if event.error {
                    results.push((id, task_type.unwrap_or_default(), PollEvent::Error));
                } else if event.readable {
                    results.push((id, task_type.unwrap_or_default(), PollEvent::Readable));
                } else if event.writable {
                    results.push((id, task_type.unwrap_or_default(), PollEvent::Writable));
                }
            }
        }

        Ok(results)
    }

    pub fn ws_send(&mut self, ws_obj_ptr: usize, data: Vec<u8>) {
        let frame = WsFrame::new_text(data);
        for (_, task) in &mut self.tasks {
            if let ReactorTask::Ws(ws_task) = task {
                if ws_task.ws_obj_ptr == ws_obj_ptr {
                    if let WsPhase::Open { pending_out, .. } = &mut ws_task.phase {
                        pending_out.push_back(frame);
                    }
                    break;
                }
            }
        }
    }

    pub fn ws_close(&mut self, ws_obj_ptr: usize, code: u16, reason: &str) {
        for (_, task) in &mut self.tasks {
            if let ReactorTask::Ws(ws_task) = task {
                if ws_task.ws_obj_ptr == ws_obj_ptr {
                    if let WsPhase::Open {
                        pending_out,
                        closing,
                        close_code,
                        close_reason,
                        ..
                    } = &mut ws_task.phase
                    {
                        if !*closing {
                            let frame = WsFrame::new_close(code, reason);
                            pending_out.push_back(frame);
                            *closing = true;
                            *close_code = code;
                            *close_reason = reason.to_string();
                        }
                    }
                }
            }
        }
    }

    pub fn sse_close(&mut self, es_obj_ptr: usize) {
        let ids_to_remove: Vec<u64> = self
            .tasks
            .iter()
            .filter(|(_, t)| {
                if let ReactorTask::Sse(sse_task) = t {
                    sse_task.es_obj_ptr == es_obj_ptr
                } else {
                    false
                }
            })
            .map(|(id, _)| *id)
            .collect();
        for id in ids_to_remove {
            self.unregister(id);
        }
    }

    fn advance_task(&mut self, ctx: &mut JSContext, id: u64) -> Result<(), String> {
        let task_type = self
            .tasks
            .get(&id)
            .map(|t| t.task_type().to_string())
            .unwrap_or_default();

        match task_type.as_str() {
            "fetch" => self.advance_fetch(ctx, id),
            "ws" => self.advance_ws(ctx, id),
            "sse" => self.advance_sse(ctx, id),
            _ => Ok(()),
        }
    }

    fn advance_fetch(&mut self, ctx: &mut JSContext, id: u64) -> Result<(), String> {
        let task = self.tasks.remove(&id);
        let mut task = match task {
            Some(t) => t,
            None => return Ok(()),
        };

        if let ReactorTask::Fetch(fetch_task) = &mut task {
            let phase = std::mem::replace(
                &mut fetch_task.phase,
                FetchPhase::Active(HttpRequest::dummy()),
            );
            match phase {
                FetchPhase::Connecting(mut cs) => match cs.try_advance() {
                    ConnEvent::Connected(conn) => {
                        let conn = set_conn_nonblocking(conn)?;
                        let mut req = HttpRequest::new(
                            fetch_task.url.clone(),
                            fetch_task.method,
                            fetch_task.req_headers.clone(),
                            fetch_task.body.clone(),
                        );
                        req.conn = Some(conn);
                        req.state = RequestState::WritingRequest;
                        req.build_request();
                        fetch_task.phase = FetchPhase::Active(req);
                        self.reregister(id, task)?;
                        let _ = self.advance_task(ctx, id);
                        return Ok(());
                    }
                    ConnEvent::Error(e) => {
                        let val = crate::value::JSValue::new_string(ctx.intern(&e));
                        crate::builtins::promise::reject_promise_with_value(
                            ctx,
                            fetch_task.promise_ptr,
                            val,
                        );
                        return Ok(());
                    }
                    _ => {
                        fetch_task.phase = FetchPhase::Connecting(cs);
                        self.reregister(id, task)?;
                        return Ok(());
                    }
                },
                FetchPhase::Active(mut req) => match req.try_advance() {
                    Ok(RequestEvent::Complete(resp)) => {
                        let resp_val = build_response_js_object(ctx, resp, &fetch_task.url.full);
                        crate::builtins::promise::fulfill_promise_with_value(
                            ctx,
                            fetch_task.promise_ptr,
                            resp_val,
                        );
                    }
                    Ok(RequestEvent::Error(e)) => {
                        let val = crate::value::JSValue::new_string(ctx.intern(&e));
                        crate::builtins::promise::reject_promise_with_value(
                            ctx,
                            fetch_task.promise_ptr,
                            val,
                        );
                    }
                    Ok(_) => {
                        fetch_task.phase = FetchPhase::Active(req);
                        self.reregister(id, task)?;
                    }
                    Err(e) => {
                        let val = crate::value::JSValue::new_string(ctx.intern(&e));
                        crate::builtins::promise::reject_promise_with_value(
                            ctx,
                            fetch_task.promise_ptr,
                            val,
                        );
                    }
                },
            }
        }
        Ok(())
    }

    fn advance_ws(&mut self, ctx: &mut JSContext, id: u64) -> Result<(), String> {
        let task = self.tasks.remove(&id);
        let mut task = match task {
            Some(t) => t,
            None => return Ok(()),
        };

        if let ReactorTask::Ws(ws_task) = &mut task {
            let phase = std::mem::replace(
                &mut ws_task.phase,
                WsPhase::Connecting(ConnectState::dummy()),
            );

            match phase {
                WsPhase::Connecting(mut cs) => match cs.try_advance() {
                    ConnEvent::Connected(conn) => {
                        let conn = set_conn_nonblocking(conn)?;
                        let key = WsHandshake::generate_key();
                        let path = ws_task.url.request_target();
                        let host = if ws_task.url.port != 80 && ws_task.url.port != 443 {
                            format!("{}:{}", ws_task.url.host, ws_task.url.port)
                        } else {
                            ws_task.url.host.clone()
                        };
                        let mut hdrs = WsHandshake::build_request(&host, &path, &key);
                        if !hdrs.contains("user-agent") {
                            hdrs.set("User-Agent", "pipa/0.1");
                        }
                        let mut write_buf = Vec::new();
                        write_buf.extend_from_slice(b"GET ");
                        write_buf.extend_from_slice(path.as_bytes());
                        write_buf.extend_from_slice(b" HTTP/1.1\r\n");
                        write_buf.extend_from_slice(hdrs.to_request_bytes().as_ref());
                        write_buf.extend_from_slice(b"\r\n");
                        ws_task.phase = WsPhase::Handshake {
                            conn,
                            key,
                            write_buf,
                            write_pos: 0,
                            read_data: Vec::new(),
                        };
                        self.reregister(id, task)?;
                        let _ = self.advance_task(ctx, id);
                        return Ok(());
                    }
                    ConnEvent::Error(e) => {
                        let val = crate::value::JSValue::new_string(ctx.intern(&e));
                        crate::builtins::promise::reject_promise_with_value(
                            ctx,
                            ws_task.promise_ptr,
                            val,
                        );
                        return Ok(());
                    }
                    _ => {
                        ws_task.phase = WsPhase::Connecting(cs);
                        self.reregister(id, task)?;
                        return Ok(());
                    }
                },
                WsPhase::Handshake {
                    mut conn,
                    key,
                    write_buf,
                    mut write_pos,
                    mut read_data,
                } => {
                    if write_pos < write_buf.len() {
                        let remaining = &write_buf[write_pos..];
                        match conn.write(remaining) {
                            Ok(n) => {
                                write_pos += n;
                            }
                            Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                            Err(e) => {
                                let val = crate::value::JSValue::new_string(
                                    ctx.intern(&format!("ws write: {e}")),
                                );
                                crate::builtins::promise::reject_promise_with_value(
                                    ctx,
                                    ws_task.promise_ptr,
                                    val,
                                );
                                return Ok(());
                            }
                        }
                    }
                    if write_pos >= write_buf.len() {
                        let mut tmp = [0u8; 8192];
                        match conn.read(&mut tmp) {
                            Ok(0) => {
                                let val = crate::value::JSValue::new_string(
                                    ctx.intern("ws handshake closed"),
                                );
                                crate::builtins::promise::reject_promise_with_value(
                                    ctx,
                                    ws_task.promise_ptr,
                                    val,
                                );
                                return Ok(());
                            }
                            Ok(n) => {
                                read_data.extend_from_slice(&tmp[..n]);
                                if let Some(pos) =
                                    read_data.windows(4).position(|w| w == b"\r\n\r\n")
                                {
                                    let status_line_end = read_data
                                        .windows(2)
                                        .position(|w| w == b"\r\n")
                                        .unwrap_or(0);
                                    let status_line = &read_data[..status_line_end];
                                    let code = if status_line.len() >= 12 {
                                        String::from_utf8_lossy(&status_line[9..12])
                                            .parse::<u16>()
                                            .unwrap_or(0)
                                    } else {
                                        0
                                    };
                                    let status = HttpStatus(code);
                                    let header_bytes = &read_data[status_line_end + 2..pos + 4];
                                    let (headers, _) = Headers::from_bytes(header_bytes)?;
                                    let accept = WsHandshake::validate_response(status, &headers)?;
                                    if !WsHandshake::verify_accept(&key, &accept) {
                                        let val = crate::value::JSValue::new_string(
                                            ctx.intern("ws accept mismatch"),
                                        );
                                        crate::builtins::promise::reject_promise_with_value(
                                            ctx,
                                            ws_task.promise_ptr,
                                            val,
                                        );
                                        return Ok(());
                                    }

                                    let ws_obj = unsafe {
                                        &mut *(ws_task.ws_obj_ptr
                                            as *mut crate::object::object::JSObject)
                                    };
                                    ws_obj.set(
                                        ctx.intern("readyState"),
                                        crate::value::JSValue::new_int(1),
                                    );

                                    let val = crate::value::JSValue::new_object(ws_task.ws_obj_ptr);
                                    crate::builtins::promise::fulfill_promise_with_value(
                                        ctx,
                                        ws_task.promise_ptr,
                                        val,
                                    );

                                    let onopen = ws_obj
                                        .get(ctx.intern("onopen"))
                                        .unwrap_or(crate::value::JSValue::undefined());
                                    if onopen.is_function() {
                                        ctx.event_loop_mut().schedule_macrotask(onopen, vec![]);
                                    }

                                    ws_task.phase = WsPhase::Open {
                                        conn,
                                        read_buf: [0u8; 8192],
                                        read_data: Vec::new(),
                                        pending_out: VecDeque::new(),
                                        closing: false,
                                        close_code: 0,
                                        close_reason: String::new(),
                                    };
                                    self.reregister(id, task)?;
                                    return Ok(());
                                }
                            }
                            Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                            Err(e) => {
                                let val = crate::value::JSValue::new_string(
                                    ctx.intern(&format!("ws handshake read: {e}")),
                                );
                                crate::builtins::promise::reject_promise_with_value(
                                    ctx,
                                    ws_task.promise_ptr,
                                    val,
                                );
                                return Ok(());
                            }
                        }
                    }
                    ws_task.phase = WsPhase::Handshake {
                        conn,
                        key,
                        write_buf,
                        write_pos,
                        read_data,
                    };
                    self.reregister(id, task)?;
                    return Ok(());
                }
                WsPhase::Open {
                    mut conn,
                    mut read_buf,
                    mut read_data,
                    mut pending_out,
                    closing,
                    close_code,
                    close_reason,
                } => {
                    while let Some(frame) = pending_out.pop_front() {
                        let encoded = frame.encode();
                        match conn.write_all(&encoded) {
                            Ok(_) => {}
                            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                                pending_out.push_front(frame);
                                break;
                            }
                            Err(e) => {
                                dispatch_ws_error(
                                    ctx,
                                    ws_task.ws_obj_ptr,
                                    &format!("ws write: {e}"),
                                );
                                return Ok(());
                            }
                        }
                    }

                    if closing && pending_out.is_empty() {
                        let ws_obj = unsafe {
                            &mut *(ws_task.ws_obj_ptr as *mut crate::object::object::JSObject)
                        };
                        ws_obj.set(ctx.intern("readyState"), crate::value::JSValue::new_int(3));
                        dispatch_ws_close(ctx, ws_task.ws_obj_ptr, close_code, close_reason);
                        return Ok(());
                    }

                    match conn.read(&mut read_buf) {
                        Ok(0) => {
                            let ws_obj = unsafe {
                                &mut *(ws_task.ws_obj_ptr as *mut crate::object::object::JSObject)
                            };
                            ws_obj.set(ctx.intern("readyState"), crate::value::JSValue::new_int(3));
                            dispatch_ws_close(
                                ctx,
                                ws_task.ws_obj_ptr,
                                1006,
                                "connection closed".into(),
                            );
                            return Ok(());
                        }
                        Ok(n) => {
                            read_data.extend_from_slice(&read_buf[..n]);
                            if let Ok(frames) = WsFrame::parse_all(&read_data) {
                                let consumed = calculate_consumed(&frames);
                                read_data.drain(..consumed);
                                for frame in frames {
                                    match frame.opcode {
                                        OpCode::Text => {
                                            let text = String::from_utf8_lossy(&frame.payload);
                                            dispatch_ws_message(
                                                ctx,
                                                ws_task.ws_obj_ptr,
                                                &text,
                                                true,
                                            );
                                        }
                                        OpCode::Binary => {
                                            let text = String::from_utf8_lossy(&frame.payload);
                                            dispatch_ws_message(
                                                ctx,
                                                ws_task.ws_obj_ptr,
                                                &text,
                                                false,
                                            );
                                        }
                                        OpCode::Ping => {
                                            let pong = WsFrame::new_pong(frame.payload);
                                            pending_out.push_back(pong);
                                        }
                                        OpCode::Close => {
                                            let (code, reason) =
                                                parse_close_payload(&frame.payload);
                                            let close_frame = WsFrame::new_close(code, &reason);
                                            pending_out.push_back(close_frame);
                                            ws_task.phase = WsPhase::Open {
                                                conn,
                                                read_buf,
                                                read_data,
                                                pending_out,
                                                closing: true,
                                                close_code: code,
                                                close_reason: reason,
                                            };
                                            self.reregister(id, task)?;
                                            return Ok(());
                                        }
                                        OpCode::Pong | OpCode::Continuation => {}
                                    }
                                }
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                        Err(e) => {
                            dispatch_ws_error(ctx, ws_task.ws_obj_ptr, &format!("ws read: {e}"));
                            return Ok(());
                        }
                    }
                    ws_task.phase = WsPhase::Open {
                        conn,
                        read_buf,
                        read_data,
                        pending_out,
                        closing,
                        close_code,
                        close_reason,
                    };
                    self.reregister(id, task)?;
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn advance_sse(&mut self, ctx: &mut JSContext, id: u64) -> Result<(), String> {
        let task = self.tasks.remove(&id);
        let mut task = match task {
            Some(t) => t,
            None => return Ok(()),
        };

        if let ReactorTask::Sse(sse_task) = &mut task {
            let phase = std::mem::replace(
                &mut sse_task.phase,
                SsePhase::Connecting(ConnectState::dummy()),
            );

            match phase {
                SsePhase::Connecting(mut cs) => match cs.try_advance() {
                    ConnEvent::Connected(conn) => {
                        let conn = set_conn_nonblocking(conn)?;
                        let path = sse_task.url.request_target();
                        let host_header = format!("{}:{}", sse_task.url.host, sse_task.url.port);
                        let mut write_buf = Vec::new();
                        write_buf.extend_from_slice(b"GET ");
                        write_buf.extend_from_slice(path.as_bytes());
                        write_buf.extend_from_slice(b" HTTP/1.1\r\nHost: ");
                        write_buf.extend_from_slice(host_header.as_bytes());
                        write_buf.extend_from_slice(b"\r\nAccept: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n");
                        sse_task.phase = SsePhase::WritingRequest {
                            conn,
                            write_buf,
                            write_pos: 0,
                        };
                        self.reregister(id, task)?;
                        let _ = self.advance_task(ctx, id);
                        return Ok(());
                    }
                    ConnEvent::Error(e) => {
                        let val = crate::value::JSValue::new_string(ctx.intern(&e));
                        crate::builtins::promise::reject_promise_with_value(
                            ctx,
                            sse_task.promise_ptr,
                            val,
                        );
                        return Ok(());
                    }
                    _ => {
                        sse_task.phase = SsePhase::Connecting(cs);
                        self.reregister(id, task)?;
                        return Ok(());
                    }
                },
                SsePhase::WritingRequest {
                    mut conn,
                    write_buf,
                    mut write_pos,
                } => {
                    if write_pos < write_buf.len() {
                        let remaining = &write_buf[write_pos..];
                        match conn.write(remaining) {
                            Ok(n) => {
                                write_pos += n;
                            }
                            Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                            Err(e) => {
                                let val = crate::value::JSValue::new_string(
                                    ctx.intern(&format!("sse write: {e}")),
                                );
                                crate::builtins::promise::reject_promise_with_value(
                                    ctx,
                                    sse_task.promise_ptr,
                                    val,
                                );
                                return Ok(());
                            }
                        }
                    }
                    if write_pos >= write_buf.len() {
                        sse_task.phase = SsePhase::ReadingHeaders {
                            conn,
                            read_buf: [0u8; 8192],
                            read_data: Vec::new(),
                        };
                        self.reregister(id, task)?;
                        return Ok(());
                    }
                    sse_task.phase = SsePhase::WritingRequest {
                        conn,
                        write_buf,
                        write_pos,
                    };
                    self.reregister(id, task)?;
                    return Ok(());
                }
                SsePhase::ReadingHeaders {
                    mut conn,
                    mut read_buf,
                    mut read_data,
                } => {
                    match conn.read(&mut read_buf) {
                        Ok(0) => {
                            let val = crate::value::JSValue::new_string(
                                ctx.intern("sse closed during headers"),
                            );
                            crate::builtins::promise::reject_promise_with_value(
                                ctx,
                                sse_task.promise_ptr,
                                val,
                            );
                            return Ok(());
                        }
                        Ok(n) => {
                            read_data.extend_from_slice(&read_buf[..n]);
                            if let Some(pos) = read_data.windows(4).position(|w| w == b"\r\n\r\n") {
                                let header_str = String::from_utf8_lossy(&read_data[..pos]);
                                if !header_str.contains("200") && !header_str.contains("201") {
                                    let val =
                                        crate::value::JSValue::new_string(ctx.intern(&format!(
                                            "sse bad status: {}",
                                            header_str.lines().next().unwrap_or("")
                                        )));
                                    crate::builtins::promise::reject_promise_with_value(
                                        ctx,
                                        sse_task.promise_ptr,
                                        val,
                                    );
                                    return Ok(());
                                }

                                let val = crate::value::JSValue::new_object(sse_task.es_obj_ptr);
                                crate::builtins::promise::fulfill_promise_with_value(
                                    ctx,
                                    sse_task.promise_ptr,
                                    val,
                                );

                                let es_obj = unsafe {
                                    &mut *(sse_task.es_obj_ptr
                                        as *mut crate::object::object::JSObject)
                                };
                                es_obj.set(
                                    ctx.intern("readyState"),
                                    crate::value::JSValue::new_int(1),
                                );
                                let onopen = es_obj
                                    .get(ctx.intern("onopen"))
                                    .unwrap_or(crate::value::JSValue::undefined());
                                if onopen.is_function() {
                                    ctx.event_loop_mut().schedule_macrotask(onopen, vec![]);
                                }

                                let remaining = read_data[pos + 4..].to_vec();
                                sse_task.phase = SsePhase::Streaming {
                                    conn,
                                    read_buf: [0u8; 8192],
                                    read_data: Vec::new(),
                                    sse_buf: remaining,
                                };
                                self.reregister(id, task)?;
                                return Ok(());
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                        Err(e) => {
                            let val = crate::value::JSValue::new_string(
                                ctx.intern(&format!("sse header read: {e}")),
                            );
                            crate::builtins::promise::reject_promise_with_value(
                                ctx,
                                sse_task.promise_ptr,
                                val,
                            );
                            return Ok(());
                        }
                    }
                    sse_task.phase = SsePhase::ReadingHeaders {
                        conn,
                        read_buf,
                        read_data,
                    };
                    self.reregister(id, task)?;
                    return Ok(());
                }
                SsePhase::Streaming {
                    mut conn,
                    mut read_buf,
                    read_data: _,
                    mut sse_buf,
                } => {
                    match conn.read(&mut read_buf) {
                        Ok(0) => {
                            let es_obj = unsafe {
                                &mut *(sse_task.es_obj_ptr as *mut crate::object::object::JSObject)
                            };
                            es_obj.set(ctx.intern("readyState"), crate::value::JSValue::new_int(2));
                            let onclose = es_obj
                                .get(ctx.intern("onclose"))
                                .unwrap_or(crate::value::JSValue::undefined());
                            if onclose.is_function() {
                                ctx.event_loop_mut().schedule_macrotask(onclose, vec![]);
                            }
                            return Ok(());
                        }
                        Ok(n) => {
                            sse_buf.extend_from_slice(&read_buf[..n]);
                            while let Some(event) =
                                crate::builtins::eventsource::parse_sse_event(&mut sse_buf)
                            {
                                if event.is_closed {
                                    let es_obj = unsafe {
                                        &mut *(sse_task.es_obj_ptr
                                            as *mut crate::object::object::JSObject)
                                    };
                                    es_obj.set(
                                        ctx.intern("readyState"),
                                        crate::value::JSValue::new_int(2),
                                    );
                                    let onclose = es_obj
                                        .get(ctx.intern("onclose"))
                                        .unwrap_or(crate::value::JSValue::undefined());
                                    if onclose.is_function() {
                                        ctx.event_loop_mut().schedule_macrotask(onclose, vec![]);
                                    }
                                    return Ok(());
                                }
                                dispatch_sse_event(ctx, sse_task.es_obj_ptr, &event);
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                        Err(e) => {
                            let es_obj = unsafe {
                                &mut *(sse_task.es_obj_ptr as *mut crate::object::object::JSObject)
                            };
                            let onerror = es_obj
                                .get(ctx.intern("onerror"))
                                .unwrap_or(crate::value::JSValue::undefined());
                            if onerror.is_function() {
                                let err_val =
                                    crate::value::JSValue::new_string(ctx.intern(&format!("{e}")));
                                ctx.event_loop_mut()
                                    .schedule_macrotask(onerror, vec![err_val]);
                            }
                            return Ok(());
                        }
                    }
                    sse_task.phase = SsePhase::Streaming {
                        conn,
                        read_buf,
                        read_data: Vec::new(),
                        sse_buf,
                    };
                    self.reregister(id, task)?;
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl MacroTaskExtension for IoReactor {
    fn tick(&mut self, ctx: &mut JSContext) -> Result<bool, String> {
        if self.is_empty() {
            return Ok(false);
        }

        let events = self.poll(1)?;
        let ids: Vec<u64> = events.iter().map(|(id, _, _)| *id).collect();
        for id in ids {
            let _ = self.advance_task(ctx, id);
        }

        Ok(!events.is_empty())
    }

    fn has_pending(&self) -> bool {
        !self.is_empty()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn build_response_js_object(
    ctx: &mut JSContext,
    mut resp: HttpResponse,
    url: &str,
) -> crate::value::JSValue {
    let body = resp.body_reader.take_body();
    let body_text = String::from_utf8_lossy(&body).to_string();

    let status = resp.status.0 as i64;
    let status_text = resp.status_text;

    let mut obj = crate::object::object::JSObject::new();
    obj.set(ctx.intern("status"), crate::value::JSValue::new_int(status));
    obj.set(
        ctx.intern("ok"),
        crate::value::JSValue::bool(status >= 200 && status < 300),
    );
    obj.set(
        ctx.intern("statusText"),
        crate::value::JSValue::new_string(ctx.intern(&status_text)),
    );
    obj.set(
        ctx.intern("url"),
        crate::value::JSValue::new_string(ctx.intern(url)),
    );
    obj.set(
        ctx.intern("__body__"),
        crate::value::JSValue::new_string(ctx.intern(&body_text)),
    );

    let text_fn = create_builtin_function(ctx, "response_text");
    obj.set(ctx.intern("text"), text_fn);

    let json_fn = create_builtin_function(ctx, "response_json");
    obj.set(ctx.intern("json"), json_fn);

    let ptr = Box::into_raw(Box::new(obj)) as usize;
    crate::value::JSValue::new_object(ptr)
}

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> crate::value::JSValue {
    let mut func = crate::object::function::JSFunction::new_builtin(ctx.intern(name), 1);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    crate::value::JSValue::new_function(ptr)
}

fn dispatch_ws_message(ctx: &mut JSContext, ws_obj_ptr: usize, data: &str, _is_text: bool) {
    let ws_obj = unsafe { &*(ws_obj_ptr as *const crate::object::object::JSObject) };
    let on_msg = ws_obj
        .get(ctx.intern("onmessage"))
        .unwrap_or(crate::value::JSValue::undefined());
    if on_msg.is_function() {
        let msg_val = crate::value::JSValue::new_string(ctx.intern(data));
        ctx.event_loop_mut()
            .schedule_macrotask(on_msg, vec![msg_val]);
    }
}

fn dispatch_ws_close(ctx: &mut JSContext, ws_obj_ptr: usize, code: u16, reason: String) {
    let ws_obj = unsafe { &*(ws_obj_ptr as *const crate::object::object::JSObject) };
    let on_close = ws_obj
        .get(ctx.intern("onclose"))
        .unwrap_or(crate::value::JSValue::undefined());
    if on_close.is_function() {
        let mut evt = crate::object::object::JSObject::new();
        evt.set(
            ctx.intern("code"),
            crate::value::JSValue::new_int(code as i64),
        );
        evt.set(
            ctx.intern("reason"),
            crate::value::JSValue::new_string(ctx.intern(&reason)),
        );
        let ptr = Box::into_raw(Box::new(evt)) as usize;
        let evt_val = crate::value::JSValue::new_object(ptr);
        ctx.event_loop_mut()
            .schedule_macrotask(on_close, vec![evt_val]);
    }
}

fn dispatch_ws_error(ctx: &mut JSContext, ws_obj_ptr: usize, error: &str) {
    let ws_obj = unsafe { &*(ws_obj_ptr as *const crate::object::object::JSObject) };
    let on_error = ws_obj
        .get(ctx.intern("onerror"))
        .unwrap_or(crate::value::JSValue::undefined());
    if on_error.is_function() {
        let err_val = crate::value::JSValue::new_string(ctx.intern(error));
        ctx.event_loop_mut()
            .schedule_macrotask(on_error, vec![err_val]);
    }
}

fn dispatch_sse_event(
    ctx: &mut JSContext,
    es_obj_ptr: usize,
    event: &crate::builtins::eventsource::SseEvent,
) {
    let es_obj = unsafe { &*(es_obj_ptr as *const crate::object::object::JSObject) };
    let on_msg = es_obj
        .get(ctx.intern("onmessage"))
        .unwrap_or(crate::value::JSValue::undefined());
    if on_msg.is_function() {
        let mut evt = crate::object::object::JSObject::new();
        evt.set(
            ctx.intern("data"),
            crate::value::JSValue::new_string(ctx.intern(&event.data)),
        );
        if !event.event_type.is_empty() {
            evt.set(
                ctx.intern("event"),
                crate::value::JSValue::new_string(ctx.intern(&event.event_type)),
            );
        }
        if !event.last_event_id.is_empty() {
            evt.set(
                ctx.intern("lastEventId"),
                crate::value::JSValue::new_string(ctx.intern(&event.last_event_id)),
            );
        }
        let ptr = Box::into_raw(Box::new(evt)) as usize;
        let evt_val = crate::value::JSValue::new_object(ptr);
        ctx.event_loop_mut()
            .schedule_macrotask(on_msg, vec![evt_val]);
    }
}

fn set_conn_nonblocking(conn: Connection) -> Result<Connection, String> {
    conn.set_nonblocking(true)?;
    Ok(conn)
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

fn calculate_consumed(frames: &[WsFrame]) -> usize {
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
