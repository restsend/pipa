use std::io::{Read, Write};

use crate::host::HostFunction;
use crate::http::conn::Connection;
use crate::http::url::Url;
use crate::object::function::JSFunction;
use crate::runtime::context::JSContext;
use crate::util::iomux::Poller;
use crate::value::JSValue;

pub fn register_eventsource(ctx: &mut JSContext) {
    ctx.register_builtin("EventSource", HostFunction::new("EventSource", 1, es_ctor));
    ctx.register_builtin("es_read", HostFunction::new("read", 1, es_read));
    ctx.register_builtin("es_close", HostFunction::new("close", 1, es_close));
}

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(name), 1);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn es_ctor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_string() {
        eprintln!("EventSource: url required");
        return JSValue::undefined();
    }

    let url_str = ctx.get_atom_str(args[0].get_atom()).to_string();
    let url = match Url::parse(&url_str) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("EventSource: invalid URL: {e}");
            return JSValue::undefined();
        }
    };

    let use_tls = url.is_tls();
    let rx = match Connection::connect_async(url.host.clone(), url.port, use_tls) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("EventSource: connect failed: {e}");
            return JSValue::undefined();
        }
    };

    let mut conn = match rx.recv() {
        Ok(Ok(c)) => c,
        _ => {
            eprintln!("EventSource: connection failed");
            return JSValue::undefined();
        }
    };

    let path = url.request_target();
    let host_header = format!("{}:{}", url.host, url.port);
    let request = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         Accept: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         Connection: keep-alive\r\n\r\n",
        host = host_header
    );

    conn.set_nonblocking(false).ok();
    if conn.write_all(request.as_bytes()).is_err() {
        eprintln!("EventSource: write failed");
        return JSValue::undefined();
    }

    conn.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok();
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
                eprintln!("EventSource: header read error: {e}");
                return JSValue::undefined();
            }
        }
    }

    let resp_str = String::from_utf8_lossy(&resp_buf);
    if !resp_str.contains("200") && !resp_str.contains("201") {
        eprintln!("EventSource: bad status: {}", &resp_str[..resp_str.find('\r').unwrap_or(resp_str.len())]);
        return JSValue::undefined();
    }

    let mut es_obj = crate::object::object::JSObject::new();
    es_obj.set(ctx.intern("url"), JSValue::new_string(ctx.intern(&url_str)));
    es_obj.set(ctx.intern("readyState"), JSValue::new_int(1));
    es_obj.set(ctx.intern("onopen"), JSValue::undefined());
    es_obj.set(ctx.intern("onmessage"), JSValue::undefined());
    es_obj.set(ctx.intern("onerror"), JSValue::undefined());

    let read_fn = create_builtin_function(ctx, "es_read");
    es_obj.set(ctx.intern("read"), read_fn);

    let close_fn = create_builtin_function(ctx, "es_close");
    es_obj.set(ctx.intern("close"), close_fn);

    let conn_idx = ctx.runtime_mut().add_connection(conn);
    es_obj.set(ctx.intern("__conn__"), JSValue::new_int(conn_idx as i64));

    let ptr = Box::into_raw(Box::new(es_obj)) as usize;
    JSValue::new_object(ptr)
}

fn get_conn_idx(ctx: &mut JSContext, args: &[JSValue]) -> usize {
    if args.is_empty() || !args[0].is_object() { return usize::MAX; }
    let obj = args[0].as_object();
    let val = obj.get(ctx.intern("__conn__")).unwrap_or(JSValue::undefined());
    if !val.is_int() { return usize::MAX; }
    val.get_int() as usize
}

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub data: String,
    pub event_type: String,
    pub last_event_id: String,
    pub is_closed: bool,
}

fn es_read(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let parsed = match read_parsed(ctx, args) {
        Some(p) => p,
        None => return JSValue::undefined(),
    };

    if parsed.is_closed {
        return JSValue::new_string(ctx.intern("__CLOSED__"));
    }

    let mut evt = crate::object::object::JSObject::new();
    evt.set(ctx.intern("data"), JSValue::new_string(ctx.intern(&parsed.data)));
    if !parsed.event_type.is_empty() {
        evt.set(ctx.intern("event"), JSValue::new_string(ctx.intern(&parsed.event_type)));
    }
    if !parsed.last_event_id.is_empty() {
        evt.set(ctx.intern("lastEventId"), JSValue::new_string(ctx.intern(&parsed.last_event_id)));
    }

    let ptr = Box::into_raw(Box::new(evt)) as usize;
    let evt_val = JSValue::new_object(ptr);

    if args[0].is_object() {
        let cb = args[0].as_object().get(ctx.intern("onmessage")).unwrap_or(JSValue::undefined());
        if cb.is_function() {
            if let Some(vp) = ctx.get_register_vm_ptr() {
                let vm = unsafe { &mut *(vp as *mut crate::runtime::vm::VM) };
                let _ = vm.call_function(ctx, cb, &[evt_val]);
            }
        }
    }

    evt_val
}

fn read_parsed(ctx: &mut JSContext, args: &[JSValue]) -> Option<SseEvent> {
    let idx = get_conn_idx(ctx, args);
    if idx == usize::MAX { return None; }

    let timeout_ms = if args.len() > 1 && args[1].is_int() {
        args[1].get_int().max(0)
    } else { 0i64 };

    let conn = match ctx.runtime_mut().get_connection(idx) {
        Some(c) => c,
        None => return None,
    };

    conn.set_nonblocking(true).ok();
    let fd = conn.raw_fd();

    let mut poller = if timeout_ms > 0 {
        let mut p = Poller::new().ok()?;
        p.register(fd, true, false).ok()?;
        Some(p)
    } else {
        None
    };

    let start = std::time::Instant::now();
    let mut sse_buf = Vec::new();

    loop {
        let mut buf = [0u8; 16384];
        match conn.read(&mut buf) {
            Ok(0) => return Some(SseEvent { data: String::new(), event_type: String::new(), last_event_id: String::new(), is_closed: true }),
            Ok(n) => {
                sse_buf.extend_from_slice(&buf[..n]);
                if let Some(evt) = parse_sse_event(&mut sse_buf) {
                    return Some(evt);
                }
                if timeout_ms > 0 && start.elapsed().as_millis() as i64 >= timeout_ms {
                    return None;
                }
            }
            _ => {
                if !sse_buf.is_empty() {
                    if let Some(evt) = parse_sse_event(&mut sse_buf) {
                        return Some(evt);
                    }
                }
                if timeout_ms <= 0 {
                    return None;
                }
                let elapsed = start.elapsed().as_millis() as i64;
                if elapsed >= timeout_ms {
                    return None;
                }
                if let Some(ref mut p) = poller {
                    let remaining = (timeout_ms - elapsed) as i32;
                    let _ = p.wait(remaining.max(1));
                }
            }
        }
    }
}

fn es_close(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let idx = get_conn_idx(ctx, args);
    if idx != usize::MAX {
        ctx.runtime_mut().release_connection(idx);
    }
    JSValue::undefined()
}

pub fn parse_sse_event(buf: &mut Vec<u8>) -> Option<SseEvent> {
    let double_nl = buf.windows(2).position(|w| w == b"\n\n")?;

    let section = &buf[..double_nl];
    let mut data = String::new();
    let mut event_type = String::new();
    let mut last_event_id = String::new();

    for line in section.split(|&b| b == b'\n') {
        if line.is_empty() || line == b"\r" { continue; }
        if line.starts_with(b"data:") {
            let val = line[5..].strip_prefix(b" ").unwrap_or(&line[5..]);
            if !data.is_empty() { data.push('\n'); }
            data.push_str(&String::from_utf8_lossy(val));
        } else if line.starts_with(b"event:") {
            let val = line[6..].strip_prefix(b" ").unwrap_or(&line[6..]);
            event_type = String::from_utf8_lossy(val).to_string();
        } else if line.starts_with(b"id:") {
            let val = line[3..].strip_prefix(b" ").unwrap_or(&line[3..]);
            last_event_id = String::from_utf8_lossy(val).to_string();
        }
    }

    buf.drain(..double_nl + 2);

    if data.is_empty() { None }
    else { Some(SseEvent { data, event_type, last_event_id, is_closed: false }) }
}
