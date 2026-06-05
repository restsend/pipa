use crate::host::HostFunction;
use crate::http::url::Url;
use crate::object::function::JSFunction;
use crate::runtime::context::JSContext;
use crate::runtime::io_reactor::{IoReactor, ReactorTask, WsTask};
use crate::value::JSValue;

pub fn register_websocket(ctx: &mut JSContext) {
    ctx.register_builtin("WebSocket", HostFunction::ctor("WebSocket", 1, ws_ctor));
    ctx.register_builtin("ws_send", HostFunction::method("send", 2, ws_send));
    ctx.register_builtin("ws_close", HostFunction::method("close", 1, ws_close));
}

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let arity = ctx.get_builtin_arity(name).unwrap_or(1);
    let mut func = JSFunction::new_builtin(ctx.intern(name), arity);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn ws_ctor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
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

    let mut ws_obj = crate::object::object::JSObject::new();
    ws_obj.set(ctx.intern("url"), JSValue::new_string(ctx.intern(&url_str)));
    ws_obj.set(ctx.intern("readyState"), JSValue::new_int(0));
    ws_obj.set(ctx.intern("onopen"), JSValue::undefined());
    ws_obj.set(ctx.intern("onmessage"), JSValue::undefined());
    ws_obj.set(ctx.intern("onerror"), JSValue::undefined());
    ws_obj.set(ctx.intern("onclose"), JSValue::undefined());
    ws_obj.set(ctx.intern("send"), create_builtin_function(ctx, "ws_send"));
    ws_obj.set(
        ctx.intern("close"),
        create_builtin_function(ctx, "ws_close"),
    );

    let ws_obj_ptr = Box::into_raw(Box::new(ws_obj)) as usize;

    let promise_val = crate::builtins::promise::create_pending_promise(ctx);
    let promise_ptr = promise_val.get_ptr();

    let task = match WsTask::new(url, ws_obj_ptr, promise_ptr) {
        Ok(t) => t,
        Err(_) => {
            unsafe {
                drop(Box::from_raw(
                    ws_obj_ptr as *mut crate::object::object::JSObject,
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

    if let Err(_) = reactor.register(ReactorTask::Ws(task)) {
        return JSValue::undefined();
    }

    JSValue::new_object(ws_obj_ptr)
}

fn get_ws_obj_ptr(args: &[JSValue]) -> usize {
    if args.is_empty() || !args[0].is_object() {
        return 0;
    }
    args[0].get_ptr()
}

fn ws_send(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let ws_obj_ptr = get_ws_obj_ptr(args);
    if ws_obj_ptr == 0 {
        return JSValue::undefined();
    }

    let data = if args.len() > 1 && args[1].is_string() {
        ctx.get_atom_str(args[1].get_atom()).as_bytes().to_vec()
    } else {
        return JSValue::undefined();
    };

    let reactor = match IoReactor::get_from_ctx(ctx) {
        Some(r) => r,
        None => return JSValue::undefined(),
    };
    reactor.ws_send(ws_obj_ptr, data);
    JSValue::undefined()
}

fn ws_close(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let ws_obj_ptr = get_ws_obj_ptr(args);
    if ws_obj_ptr == 0 {
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

    let reactor = match IoReactor::get_from_ctx(ctx) {
        Some(r) => r,
        None => return JSValue::undefined(),
    };
    reactor.ws_close(ws_obj_ptr, code, &reason);

    if args[0].is_object() {
        args[0]
            .as_object_mut()
            .set(ctx.intern("readyState"), JSValue::new_int(2));
    }

    JSValue::undefined()
}
