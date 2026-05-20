use std::io::{Read, Write};

use crate::host::HostFunction;
use crate::http::conn::Connection;
use crate::http::url::Url;
use crate::http::ws::frame::WsFrame;
use crate::http::ws::handshake::WsHandshake;
use crate::object::function::JSFunction;
use crate::runtime::context::JSContext;
use crate::util::iomux::Poller;
use crate::value::JSValue;

pub fn register_websocket(ctx: &mut JSContext) {
    ctx.register_builtin("WebSocket", HostFunction::new("WebSocket", 1, ws_ctor));
    ctx.register_builtin("ws_send", HostFunction::new("send", 2, ws_send));
    ctx.register_builtin("ws_close", HostFunction::new("close", 1, ws_close));
    ctx.register_builtin("ws_recv", HostFunction::new("recv", 1, ws_recv));
}

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(name), 1);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn get_conn_idx(ctx: &mut JSContext, args: &[JSValue]) -> usize {
    if args.is_empty() || !args[0].is_object() {
        return usize::MAX;
    }
    let obj = args[0].as_object();
    let val = obj
        .get(ctx.intern("__conn__"))
        .unwrap_or(JSValue::undefined());
    if !val.is_int() {
        return usize::MAX;
    }
    val.get_int() as usize
}

fn ws_ctor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_string() {
        eprintln!("WebSocket: url required");
        return JSValue::undefined();
    }

    let url_str = ctx.get_atom_str(args[0].get_atom()).to_string();
    let url = match Url::parse(&url_str) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("WebSocket: invalid URL: {e}");
            return JSValue::undefined();
        }
    };

    let use_tls = url.is_tls();
    let rx = match Connection::connect_async(url.host.clone(), url.port, use_tls) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("WebSocket: connect failed: {e}");
            return JSValue::undefined();
        }
    };

    let mut conn = match rx.recv() {
        Ok(Ok(c)) => c,
        _ => {
            eprintln!("WebSocket: connection failed");
            return JSValue::undefined();
        }
    };

    let key = WsHandshake::generate_key();
    let path = url.request_target();
    let host = if url.port != 80 && url.port != 443 {
        format!("{}:{}", url.host, url.port)
    } else {
        url.host.clone()
    };
    let req_str = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    conn.set_nonblocking(false).ok();
    if conn.write_all(req_str.as_bytes()).is_err() {
        eprintln!("WebSocket: handshake write failed");
        return JSValue::undefined();
    }

    conn.set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .ok();
    let mut resp_buf = Vec::new();
    loop {
        let mut tmp = [0u8; 4096];
        match conn.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                resp_buf.extend_from_slice(&tmp[..n]);
                if resp_buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            Err(e) => {
                eprintln!("WebSocket: handshake read error: {e}");
                return JSValue::undefined();
            }
        }
    }
    let resp_str = String::from_utf8_lossy(&resp_buf);
    let accept_line = resp_str
        .lines()
        .find(|l| l.to_lowercase().starts_with("sec-websocket-accept:"));
    let accept = accept_line
        .and_then(|l| l.splitn(2, ':').nth(1))
        .map(|s| s.trim())
        .unwrap_or_default();
    if !WsHandshake::verify_accept(&key, accept) {
        eprintln!("WebSocket: accept mismatch");
        return JSValue::undefined();
    }
    if !resp_str.contains("101") {
        eprintln!("WebSocket: expected 101");
        return JSValue::undefined();
    }

    let conn_idx = ctx.runtime_mut().add_connection(conn);

    let mut ws_obj = crate::object::object::JSObject::new();
    ws_obj.set(ctx.intern("url"), JSValue::new_string(ctx.intern(&url_str)));
    ws_obj.set(ctx.intern("readyState"), JSValue::new_int(1));
    ws_obj.set(ctx.intern("onopen"), JSValue::undefined());
    ws_obj.set(ctx.intern("onmessage"), JSValue::undefined());
    ws_obj.set(ctx.intern("onerror"), JSValue::undefined());
    ws_obj.set(ctx.intern("onclose"), JSValue::undefined());
    ws_obj.set(ctx.intern("send"), create_builtin_function(ctx, "ws_send"));
    ws_obj.set(
        ctx.intern("close"),
        create_builtin_function(ctx, "ws_close"),
    );
    ws_obj.set(ctx.intern("recv"), create_builtin_function(ctx, "ws_recv"));
    ws_obj.set(ctx.intern("__conn__"), JSValue::new_int(conn_idx as i64));

    let ptr = Box::into_raw(Box::new(ws_obj)) as usize;
    JSValue::new_object(ptr)
}

fn ws_send(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let idx = get_conn_idx(ctx, args);
    if idx == usize::MAX {
        return JSValue::undefined();
    }

    let data = if args.len() > 1 && args[1].is_string() {
        ctx.get_atom_str(args[1].get_atom()).as_bytes().to_vec()
    } else {
        return JSValue::undefined();
    };

    let conn = match ctx.runtime_mut().get_connection(idx) {
        Some(c) => c,
        None => return JSValue::undefined(),
    };
    let frame = WsFrame::new_text(data);
    conn.set_nonblocking(false).ok();
    let _ = conn.write_all(&frame.encode());
    JSValue::undefined()
}

fn ws_close(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let idx = get_conn_idx(ctx, args);
    if idx == usize::MAX {
        return JSValue::undefined();
    }

    let code = if args.len() > 1 && args[1].is_int() {
        args[1].get_int() as u16
    } else {
        1000
    };
    let reason = if args.len() > 2 && args[2].is_string() {
        ctx.get_atom_str(args[2].get_atom()).to_string()
    } else {
        String::new()
    };

    if let Some(conn) = ctx.runtime_mut().get_connection(idx) {
        let frame = WsFrame::new_close(code, &reason);
        conn.set_nonblocking(false).ok();
        let _ = conn.write_all(&frame.encode());
    }

    if args[0].is_object() {
        args[0]
            .as_object_mut()
            .set(ctx.intern("readyState"), JSValue::new_int(3));
    }

    ctx.runtime_mut().release_connection(idx);
    JSValue::undefined()
}

fn ws_recv(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let idx = get_conn_idx(ctx, args);
    if idx == usize::MAX {
        return JSValue::undefined();
    }

    let timeout_ms = if args.len() > 1 && args[1].is_int() {
        args[1].get_int().max(0)
    } else {
        0i64
    };

    let conn = match ctx.runtime_mut().get_connection(idx) {
        Some(c) => c,
        None => return JSValue::undefined(),
    };

    conn.set_nonblocking(true).ok();
    let fd = conn.raw_fd();

    let mut poller = if timeout_ms > 0 {
        match Poller::new().and_then(|mut p| p.register(fd, true, false).map(|_| p)) {
            Ok(p) => Some(p),
            Err(_) => return JSValue::undefined(),
        }
    } else {
        None
    };

    let start = std::time::Instant::now();
    loop {
        let mut buf = [0u8; 16384];
        match conn.read(&mut buf) {
            Ok(n) if n > 0 => {
                if let Ok(frames) = WsFrame::parse_all(&buf[..n]) {
                    for frame in &frames {
                        match frame.opcode {
                            crate::http::ws::frame::OpCode::Text => {
                                let text = String::from_utf8_lossy(&frame.payload);
                                return JSValue::new_string(ctx.intern(&text));
                            }
                            crate::http::ws::frame::OpCode::Close => {
                                return JSValue::new_string(ctx.intern("__CLOSED__"));
                            }
                            _ => {}
                        }
                    }
                }
                return JSValue::undefined();
            }
            _ => {
                if timeout_ms <= 0 {
                    return JSValue::undefined();
                }
                let elapsed = start.elapsed().as_millis() as i64;
                if elapsed >= timeout_ms {
                    return JSValue::undefined();
                }
                if let Some(ref mut p) = poller {
                    let remaining = (timeout_ms - elapsed) as i32;
                    let _ = p.wait(remaining.max(1));
                }
            }
        }
    }
}
