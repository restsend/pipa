use crate::builtins::promise;
use crate::host::HostFunction;
use crate::http::method::HttpMethod;
use crate::http::url::Url;
use crate::runtime::context::JSContext;
use crate::runtime::io_reactor::{FetchTask, IoReactor, ReactorTask};
use crate::value::JSValue;

pub fn register_fetch(ctx: &mut JSContext) {
    ctx.register_builtin("fetch", HostFunction::new("fetch", 1, fetch_impl));
    ctx.register_builtin("response_text", HostFunction::new("text", 1, response_text));
    ctx.register_builtin("response_json", HostFunction::new("json", 1, response_json));
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

    let method = if args.len() > 1 && args[1].is_string() {
        let m = ctx.get_atom_str(args[1].get_atom());
        HttpMethod::from_bytes(m.as_bytes()).unwrap_or(HttpMethod::GET)
    } else {
        HttpMethod::GET
    };

    let headers = if args.len() > 2 && args[2].is_object() {
        let obj = args[2].as_object();
        let mut h = crate::http::headers::Headers::new();
        let keys = obj.keys();
        for key in keys {
            if let Some(val) = obj.get(key) {
                if val.is_string() {
                    h.set(
                        &ctx.get_atom_str(key).to_string(),
                        &ctx.get_atom_str(val.get_atom()).to_string(),
                    );
                }
            }
        }
        h
    } else {
        let mut h = crate::http::headers::Headers::new();
        h.set("User-Agent", "pipa/0.1");
        h.set("Accept", "*/*");
        h.set("Connection", "close");
        h
    };

    let body = if args.len() > 3 && args[3].is_string() {
        Some(ctx.get_atom_str(args[3].get_atom()).as_bytes().to_vec())
    } else {
        None
    };

    let promise_val = promise::create_pending_promise(ctx);
    let promise_ptr = promise_val.get_ptr();

    let task = match FetchTask::new(url, method, headers, body, promise_ptr) {
        Ok(t) => t,
        Err(e) => return reject(ctx, &e),
    };

    let reactor = match IoReactor::get_from_ctx(ctx) {
        Some(r) => r,
        None => return reject(ctx, "no io reactor"),
    };

    if let Err(e) = reactor.register(ReactorTask::Fetch(task)) {
        return reject(ctx, &e);
    }

    promise_val
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
