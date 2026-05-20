use std::io::{Read, Write};

use crate::builtins::promise;
use crate::host::HostFunction;
use crate::http::conn::Connection;
use crate::http::url::Url;
use crate::object::function::JSFunction;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

pub fn register_fetch(ctx: &mut JSContext) {
    ctx.register_builtin("fetch", HostFunction::new("fetch", 1, fetch_impl));
    ctx.register_builtin("response_text", HostFunction::new("text", 1, response_text));
    ctx.register_builtin("response_json", HostFunction::new("json", 1, response_json));
}

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(name), 1);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn reject(ctx: &mut JSContext, msg: &str) -> JSValue {
    let val = JSValue::new_string(ctx.intern(msg));
    promise::create_rejected_promise(ctx, val)
}

fn fetch_impl(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_string() {
        return reject(ctx, "fetch: url must be a string");
    }

    let url_str = ctx.get_atom_str(args[0].get_atom()).to_string();
    let url = match Url::parse(&url_str) {
        Ok(u) => u,
        Err(e) => return reject(ctx, &e),
    };

    let use_tls = url.is_tls();
    let rx = match Connection::connect_async(url.host.clone(), url.port, use_tls) {
        Ok(r) => r,
        Err(e) => return reject(ctx, &e),
    };

    let mut conn = match rx.recv() {
        Ok(Ok(c)) => c,
        _ => return reject(ctx, "connection failed"),
    };
    conn.set_nonblocking(false).ok();

    let path = url.request_target();
    let host_header = format!("{}:{}", url.host, url.port);
    let request = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         User-Agent: pipa/0.1\r\n\
         Accept: */*\r\n\
         Connection: close\r\n\r\n",
        host = host_header
    );

    if conn.write_all(request.as_bytes()).is_err() {
        return reject(ctx, "write failed");
    }

    let mut buf = [0u8; 65536];
    let mut response_data = Vec::new();
    loop {
        match conn.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response_data.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    let response = String::from_utf8_lossy(&response_data).to_string();

    let mut obj = crate::object::object::JSObject::new();

    let status = if response.len() > 12 {
        response[9..12].parse::<i64>().unwrap_or(0)
    } else {
        0
    };

    let status_text = if response.len() > 13 {
        let rest = &response[13..];
        let line_end = rest.find('\r').unwrap_or(0);
        rest[..line_end].to_string()
    } else {
        String::new()
    };

    let headers_end = response.find("\r\n\r\n").unwrap_or(response.len());
    let body_start = headers_end + 4;
    let body_text = String::from(&response[body_start..]);

    let body_atom = ctx.intern("__body__");
    let body_val = JSValue::new_string(ctx.intern(&body_text));
    let status_val = JSValue::new_int(status);
    let ok_val = JSValue::bool(status >= 200 && status < 300);
    let status_text_val = JSValue::new_string(ctx.intern(&status_text));
    let url_val = JSValue::new_string(ctx.intern(&url.full));

    obj.set(ctx.intern("status"), status_val);
    obj.set(ctx.intern("ok"), ok_val);
    obj.set(ctx.intern("statusText"), status_text_val);
    obj.set(ctx.intern("url"), url_val);
    obj.set(body_atom, body_val);

    let text_fn = create_builtin_function(ctx, "response_text");
    obj.set(ctx.intern("text"), text_fn);

    let json_fn = create_builtin_function(ctx, "response_json");
    obj.set(ctx.intern("json"), json_fn);

    let ptr = Box::into_raw(Box::new(obj)) as usize;
    let resp_val = JSValue::new_object(ptr);

    promise::create_resolved_promise(ctx, resp_val)
}

fn response_text(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object() {
        return reject(ctx, "no body");
    }
    let obj = args[0].as_object();
    let body_atom = ctx.intern("__body__");
    obj.get(body_atom).unwrap_or(JSValue::undefined())
}

fn response_json(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object() {
        return reject(ctx, "no body");
    }
    let obj = args[0].as_object();
    let body_atom = ctx.intern("__body__");
    let body = obj.get(body_atom).unwrap_or(JSValue::undefined());
    if body.is_undefined() {
        return reject(ctx, "no body");
    }
    crate::builtins::json::json_parse(ctx, &[body])
}
