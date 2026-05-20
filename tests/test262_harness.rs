#![cfg(feature = "full_runtime_tests")]

use pipa::object::JSObject;
use pipa::value::JSValue;
use pipa::{JSRuntime, eval};

fn test262_create_realm(ctx: &mut pipa::JSContext, _args: &[JSValue]) -> JSValue {
    let global = ctx.global();
    let mut realm = JSObject::new();
    realm.set(ctx.intern("global"), global);
    let ptr = Box::into_raw(Box::new(realm)) as usize;
    JSValue::new_object(ptr)
}

fn test262_eval_script(ctx: &mut pipa::JSContext, args: &[JSValue]) -> JSValue {
    let script = if let Some(v) = args.get(1) {
        if v.is_string() {
            let atom = v.get_atom();
            ctx.get_atom_str(atom).to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    match eval(ctx, &script) {
        Ok(v) => v,
        Err(_) => JSValue::undefined(),
    }
}

fn test262_detach_array_buffer(_ctx: &mut pipa::JSContext, _args: &[JSValue]) -> JSValue {
    JSValue::undefined()
}

fn inject_test262_globals(ctx: &mut pipa::JSContext) {
    let global = ctx.global();
    if !global.is_object() {
        return;
    }
    let global_obj = global.as_object_mut();

    let print_func = {
        let mut f = pipa::object::function::JSFunction::new_builtin(ctx.intern("print"), 1);
        f.builtin_atom = Some(ctx.intern("console_log"));
        f.builtin_func = ctx.get_builtin_func("console_log");
        let ptr = Box::into_raw(Box::new(f)) as usize;
        JSValue::new_function(ptr)
    };
    global_obj.set(ctx.intern("print"), print_func);

    ctx.register_builtin(
        "test262_create_realm",
        pipa::host::HostFunction::new("createRealm", 0, test262_create_realm),
    );
    ctx.register_builtin(
        "test262_eval_script",
        pipa::host::HostFunction::new("evalScript", 1, test262_eval_script),
    );
    ctx.register_builtin(
        "test262_detach_array_buffer",
        pipa::host::HostFunction::new("detachArrayBuffer", 1, test262_detach_array_buffer),
    );

    let mut dollar_262 = JSObject::new();
    dollar_262.set(ctx.intern("global"), global);

    let create_realm_func = {
        let mut f = pipa::object::function::JSFunction::new_builtin(ctx.intern("createRealm"), 0);
        f.builtin_atom = Some(ctx.intern("test262_create_realm"));
        f.builtin_func = ctx.get_builtin_func("test262_create_realm");
        let ptr = Box::into_raw(Box::new(f)) as usize;
        JSValue::new_function(ptr)
    };
    dollar_262.set(ctx.intern("createRealm"), create_realm_func);

    let eval_script_func = {
        let mut f = pipa::object::function::JSFunction::new_builtin(ctx.intern("evalScript"), 1);
        f.builtin_atom = Some(ctx.intern("test262_eval_script"));
        f.builtin_func = ctx.get_builtin_func("test262_eval_script");
        let ptr = Box::into_raw(Box::new(f)) as usize;
        JSValue::new_function(ptr)
    };
    dollar_262.set(ctx.intern("evalScript"), eval_script_func);

    let detach_func = {
        let mut f =
            pipa::object::function::JSFunction::new_builtin(ctx.intern("detachArrayBuffer"), 1);
        f.builtin_atom = Some(ctx.intern("test262_detach_array_buffer"));
        f.builtin_func = ctx.get_builtin_func("test262_detach_array_buffer");
        let ptr = Box::into_raw(Box::new(f)) as usize;
        JSValue::new_function(ptr)
    };
    dollar_262.set(ctx.intern("detachArrayBuffer"), detach_func);

    let dollar_ptr = Box::into_raw(Box::new(dollar_262)) as usize;
    global_obj.set(ctx.intern("$262"), JSValue::new_object(dollar_ptr));
}

#[test]
fn test_dollar_262_exists() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    inject_test262_globals(&mut ctx);
    let result = eval(&mut ctx, "typeof $262");
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_string());
    let atom = val.get_atom();
    let s = ctx.get_atom_str(atom);
    assert_eq!(s, "object");
}

#[test]
fn test_dollar_262_global() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    inject_test262_globals(&mut ctx);
    let result = eval(&mut ctx, "typeof $262.global");
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_string());
    let atom = val.get_atom();
    let s = ctx.get_atom_str(atom);
    assert_eq!(s, "object");
}

#[test]
fn test_dollar_262_create_realm() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    inject_test262_globals(&mut ctx);
    let result = eval(&mut ctx, "typeof $262.createRealm");
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_string());
    let atom = val.get_atom();
    let s = ctx.get_atom_str(atom);
    assert_eq!(s, "function");
}

#[test]
fn test_dollar_262_eval_script() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    inject_test262_globals(&mut ctx);
    let result = eval(&mut ctx, "typeof $262.evalScript");
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_string());
    let atom = val.get_atom();
    let s = ctx.get_atom_str(atom);
    assert_eq!(s, "function");
}

#[test]
fn test_dollar_262_detach_array_buffer() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    inject_test262_globals(&mut ctx);
    let result = eval(&mut ctx, "typeof $262.detachArrayBuffer");
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_string());
    let atom = val.get_atom();
    let s = ctx.get_atom_str(atom);
    assert_eq!(s, "function");
}

#[test]
fn test_print_function() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    inject_test262_globals(&mut ctx);
    let result = eval(&mut ctx, "typeof print");
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_string());
    let atom = val.get_atom();
    let s = ctx.get_atom_str(atom);
    assert_eq!(s, "function");
}

#[test]
fn test_create_realm_returns_object() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    inject_test262_globals(&mut ctx);
    let result = eval(&mut ctx, "var realm = $262.createRealm(); typeof realm");
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_string());
    let atom = val.get_atom();
    let s = ctx.get_atom_str(atom);
    assert_eq!(s, "object");
}

#[test]
fn test_eval_script_returns_value() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    inject_test262_globals(&mut ctx);
    let result = eval(&mut ctx, "$262.evalScript('1 + 1')");
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_int());
    assert_eq!(val.get_int(), 2);
}

#[test]
fn test_dollar_262_global_is_global_this() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    inject_test262_globals(&mut ctx);
    let result = eval(&mut ctx, "$262.global === globalThis");
    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val.is_bool());
    assert!(val.get_bool());
}
