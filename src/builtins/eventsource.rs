use crate::host::HostFunction;
use crate::http::url::Url;
use crate::object::function::JSFunction;
use crate::runtime::context::JSContext;
use crate::runtime::io_reactor::{IoReactor, ReactorTask, SseTask};
use crate::value::JSValue;

pub fn register_eventsource(ctx: &mut JSContext) {
    ctx.register_builtin("EventSource", HostFunction::new("EventSource", 1, es_ctor));
    ctx.register_builtin("es_close", HostFunction::method("close", 1, es_close));
}

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let arity = ctx.get_builtin_arity(name).unwrap_or(1);
    let mut func = JSFunction::new_builtin(ctx.intern(name), arity);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn es_ctor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_string() {
        return JSValue::undefined();
    }

    let url_str = ctx.get_atom_str(args[0].get_atom()).to_string();
    let url = match Url::parse(&url_str) {
        Ok(u) => u,
        Err(_) => {
            return JSValue::undefined();
        }
    };

    let mut es_obj = crate::object::object::JSObject::new();
    es_obj.set(ctx.intern("url"), JSValue::new_string(ctx.intern(&url_str)));
    es_obj.set(ctx.intern("readyState"), JSValue::new_int(0));
    es_obj.set(ctx.intern("onopen"), JSValue::undefined());
    es_obj.set(ctx.intern("onmessage"), JSValue::undefined());
    es_obj.set(ctx.intern("onerror"), JSValue::undefined());
    es_obj.set(ctx.intern("onclose"), JSValue::undefined());

    let close_fn = create_builtin_function(ctx, "es_close");
    es_obj.set(ctx.intern("close"), close_fn);

    let es_obj_ptr = Box::into_raw(Box::new(es_obj)) as usize;

    let promise_val = crate::builtins::promise::create_pending_promise(ctx);
    let promise_ptr = promise_val.get_ptr();

    let task = match SseTask::new(url, es_obj_ptr, promise_ptr) {
        Ok(t) => t,
        Err(_) => {
            unsafe {
                drop(Box::from_raw(
                    es_obj_ptr as *mut crate::object::object::JSObject,
                ));
            }
            return JSValue::undefined();
        }
    };

    let reactor = match IoReactor::get_from_ctx(ctx) {
        Some(r) => r,
        None => {
            return JSValue::undefined();
        }
    };

    if let Err(_) = reactor.register(ReactorTask::Sse(task)) {
        return JSValue::undefined();
    }

    JSValue::new_object(es_obj_ptr)
}

fn es_close(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let es_obj_ptr = if args.is_empty() || !args[0].is_object() {
        return JSValue::undefined();
    } else {
        args[0].get_ptr()
    };

    let reactor = match IoReactor::get_from_ctx(ctx) {
        Some(r) => r,
        None => return JSValue::undefined(),
    };
    reactor.sse_close(es_obj_ptr);

    if args[0].is_object() {
        args[0]
            .as_object_mut()
            .set(ctx.intern("readyState"), JSValue::new_int(2));
    }

    JSValue::undefined()
}

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub data: String,
    pub event_type: String,
    pub last_event_id: String,
    pub is_closed: bool,
}

pub fn parse_sse_event(buf: &mut Vec<u8>) -> Option<SseEvent> {
    let double_nl = buf.windows(2).position(|w| w == b"\n\n")?;

    let section = &buf[..double_nl];
    let mut data = String::new();
    let mut event_type = String::new();
    let mut last_event_id = String::new();

    for line in section.split(|&b| b == b'\n') {
        if line.is_empty() || line == b"\r" {
            continue;
        }
        if line.starts_with(b"data:") {
            let val = line[5..].strip_prefix(b" ").unwrap_or(&line[5..]);
            if !data.is_empty() {
                data.push('\n');
            }
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

    if data.is_empty() {
        None
    } else {
        Some(SseEvent {
            data,
            event_type,
            last_event_id,
            is_closed: false,
        })
    }
}
