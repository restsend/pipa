use crate::builtins::array;
use crate::builtins::base64::{global_atob, global_btoa};
use crate::builtins::bigint;
use crate::builtins::date;
use crate::builtins::error;
use crate::builtins::function;
use crate::builtins::generator;
use crate::builtins::json::{json_parse, json_stringify};
use crate::builtins::intl;
use crate::builtins::map_set;
use crate::builtins::math;
use crate::builtins::number::number_to_string;
use crate::builtins::object;
use crate::builtins::parse::{global_parsefloat, global_parseint};
use crate::builtins::promise;
use crate::builtins::regexp;
use crate::builtins::string;
use crate::builtins::symbol;
use crate::builtins::typedarray;
use crate::builtins::url::{
    global_decodeuri, global_decodeuricomponent, global_encodeuri, global_encodeuricomponent,
};
use crate::builtins::weakref;
#[cfg(feature = "fetch")]
use crate::builtins::eventsource;
#[cfg(feature = "fetch")]
use crate::builtins::fetch;
#[cfg(feature = "process")]
use crate::builtins::process;
#[cfg(feature = "fetch")]
use crate::builtins::websocket;
use crate::host::HostFunction;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;
use std::collections::HashSet;

pub fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(name), 1);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

pub fn init_globals(ctx: &mut JSContext) {
    ctx.register_builtin("console_log", HostFunction::new("log", 1, console_log));
    ctx.register_builtin(
        "console_error",
        HostFunction::new("error", 1, console_error),
    );
    ctx.register_builtin("console_warn", HostFunction::new("warn", 1, console_warn));
    ctx.register_builtin("console_info", HostFunction::new("info", 1, console_info));
    ctx.register_builtin(
        "setTimeout",
        HostFunction::new("setTimeout", 2, set_timeout),
    );
    ctx.register_builtin(
        "setInterval",
        HostFunction::new("setInterval", 2, set_interval),
    );
    ctx.register_builtin(
        "clearTimeout",
        HostFunction::new("clearTimeout", 1, clear_timeout),
    );
    ctx.register_builtin(
        "clearInterval",
        HostFunction::new("clearInterval", 1, clear_interval),
    );
    ctx.register_builtin(
        "queueMicrotask",
        HostFunction::new("queueMicrotask", 1, queue_microtask),
    );
    ctx.register_builtin(
        "requestAnimationFrame",
        HostFunction::new("requestAnimationFrame", 1, request_animation_frame),
    );
    ctx.register_builtin(
        "cancelAnimationFrame",
        HostFunction::new("cancelAnimationFrame", 1, cancel_animation_frame),
    );
    ctx.register_builtin("import", HostFunction::new("import", 1, js_import));
    ctx.register_builtin(
        "disassemble",
        HostFunction::new("disassemble", 1, js_disassemble),
    );
    math::register_math_builtins(ctx);
    ctx.register_builtin("global_isnan", HostFunction::new("isNaN", 1, global_isnan));
    ctx.register_builtin(
        "global_isfinite",
        HostFunction::new("isFinite", 1, global_isfinite),
    );
    ctx.register_builtin(
        "global_parseint",
        HostFunction::new("parseInt", 1, global_parseint),
    );
    ctx.register_builtin(
        "global_parsefloat",
        HostFunction::new("parseFloat", 1, global_parsefloat),
    );
    ctx.register_builtin("global_eval", HostFunction::new("eval", 1, global_eval));

    ctx.register_builtin("btoa", HostFunction::new("btoa", 1, global_btoa));
    ctx.register_builtin("atob", HostFunction::new("atob", 1, global_atob));

    ctx.register_builtin(
        "encodeURI",
        HostFunction::new("encodeURI", 1, global_encodeuri),
    );
    ctx.register_builtin(
        "decodeURI",
        HostFunction::new("decodeURI", 1, global_decodeuri),
    );
    ctx.register_builtin(
        "encodeURIComponent",
        HostFunction::new("encodeURIComponent", 1, global_encodeuricomponent),
    );
    ctx.register_builtin(
        "decodeURIComponent",
        HostFunction::new("decodeURIComponent", 1, global_decodeuricomponent),
    );

    array::register_builtins(ctx);
    string::register_builtins(ctx);
    object::register_builtins(ctx);
    function::register_builtins(ctx);
    map_set::register_builtins(ctx);
    symbol::register_builtins(ctx);
    error::register_builtins(ctx);
    bigint::register_builtins(ctx);
    weakref::register_builtins(ctx);
    date::register_builtins(ctx);
    typedarray::register_builtins(ctx);
    intl::register_builtins(ctx);

    init_console(ctx);
    init_math(ctx);
    init_reflect(ctx);
    init_json(ctx);
    init_global_funcs(ctx);

    object::init_object(ctx);
    array::init_array(ctx);
    string::init_string(ctx);

    init_number(ctx);
    init_boolean(ctx);
    function::init_function(ctx);
    init_date(ctx);
    init_regexp(ctx);
    init_promise(ctx);
    error::init_error(ctx);
    map_set::init_map_set(ctx);
    symbol::init_symbol(ctx);
    date::init_date_to_primitive(ctx);
    bigint::init_bigint(ctx);
    weakref::init_weakref(ctx);
    generator::init_generator(ctx);
    typedarray::init_typed_array(ctx);
    intl::init_intl(ctx);

    #[cfg(feature = "fetch")]
    {
        fetch::register_fetch(ctx);
        websocket::register_websocket(ctx);
        eventsource::register_eventsource(ctx);

        let reactor = crate::runtime::io_reactor::IoReactor::new()
            .expect("failed to create IoReactor");
        ctx.add_extension(Box::new(reactor));
    }

    #[cfg(feature = "process")]
    {
        process::init_process_module(ctx);

        let proc_ext = crate::runtime::process_task::ProcessExtension::new();
        ctx.add_extension(Box::new(proc_ext));
    }

    if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
        let fn_proto = unsafe { &mut *fn_proto_ptr };
        let has_instance_sym = crate::builtins::symbol::get_symbol_has_instance(ctx);
        let hi_func = crate::object::function::JSFunction::new_builtin(ctx.intern("[Symbol.hasInstance]"), 1);
        let hi_value = {
            let mut func = hi_func;
            if let Some(fn_proto_ptr2) = ctx.get_function_prototype() {
                func.base.set_prototype_raw(fn_proto_ptr2);
            }
            func.builtin_atom = Some(ctx.intern("function_has_instance"));
            func.builtin_func = ctx.get_builtin_func("function_has_instance");
            let ptr = Box::into_raw(Box::new(func)) as usize;
            ctx.runtime_mut().gc_heap_mut().track_function(ptr);
            JSValue::new_function(ptr)
        };
        if has_instance_sym.is_symbol() {
            let sym_key = crate::runtime::atom::Atom(0x40000000 | has_instance_sym.get_symbol_id());
            fn_proto.set_cached(sym_key, hi_value, ctx.shape_cache_mut());
        }
    }
}

fn console_log(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", jsvalue_to_string(arg, _ctx));
    }
    println!();
    JSValue::undefined()
}

fn console_error(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            eprint!(" ");
        }
        eprint!("{}", jsvalue_to_string(arg, _ctx));
    }
    eprintln!();
    JSValue::undefined()
}

fn console_warn(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", jsvalue_to_string(arg, _ctx));
    }
    println!();
    JSValue::undefined()
}

fn console_info(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", jsvalue_to_string(arg, _ctx));
    }
    println!();
    JSValue::undefined()
}

pub(crate) fn jsvalue_to_string(value: &JSValue, ctx: &JSContext) -> String {
    let mut visited = HashSet::new();
    jsvalue_to_string_internal(value, ctx, &mut visited)
}

fn jsvalue_to_string_internal(
    value: &JSValue,
    ctx: &JSContext,
    visited: &mut HashSet<usize>,
) -> String {
    if value.is_undefined() {
        "undefined".to_string()
    } else if value.is_null() {
        "null".to_string()
    } else if value.is_bool() {
        value.get_bool().to_string()
    } else if value.is_int() {
        value.get_int().to_string()
    } else if value.is_float() {
        value.get_float().to_string()
    } else if value.is_string() {
        let atom = value.get_atom();
        ctx.get_atom_str(atom).to_string()
    } else if value.is_bigint() {
        let obj = value.as_object();
        format!("{}n", obj.get_bigint_value())
    } else if value.is_object() {
        let ptr = value.get_ptr();

        if visited.contains(&ptr) {
            return "[Circular]".to_string();
        }
        visited.insert(ptr);

        let obj = value.as_object();
        let mut result = String::from("{");
        let mut first = true;
        obj.for_each_property(|key, val, _attrs| {
            if !first {
                result.push_str(", ");
            }
            first = false;
            result.push_str(ctx.get_atom_str(key));
            result.push_str(": ");
            result.push_str(&jsvalue_to_string_internal(&val, ctx, visited));
        });
        obj.for_each_accessor(|_key, getter, setter| {
            if !first {
                result.push_str(", ");
            }
            first = false;
            if getter.is_some() {
                result.push_str("[getter]");
            }
            if setter.is_some() {
                result.push_str("[setter]");
            }
        });
        result.push('}');
        result
    } else if value.is_function() {
        "[Function]".to_string()
    } else {
        "".to_string()
    }
}

fn set_timeout(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_int(0);
    }

    let callback = args[0].clone();
    let delay_ms = if args.len() > 1 {
        if args[1].is_int() {
            args[1].get_int() as u64
        } else if args[1].is_float() {
            args[1].get_float() as u64
        } else {
            0
        }
    } else {
        0
    };

    let callback_args: Vec<JSValue> = args.iter().skip(2).cloned().collect();

    let timer_id = ctx
        .event_loop_mut()
        .schedule_timer(callback, callback_args, delay_ms, false);

    JSValue::new_int(timer_id as i64)
}

fn set_interval(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_int(0);
    }

    let callback = args[0].clone();
    let interval_ms = if args.len() > 1 {
        if args[1].is_int() {
            args[1].get_int() as u64
        } else if args[1].is_float() {
            args[1].get_float() as u64
        } else {
            0
        }
    } else {
        0
    };

    let callback_args: Vec<JSValue> = args.iter().skip(2).cloned().collect();

    let timer_id = ctx
        .event_loop_mut()
        .schedule_timer(callback, callback_args, interval_ms, true);

    JSValue::new_int(timer_id as i64)
}

fn clear_timeout(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }

    let timer_id = if args[0].is_int() {
        args[0].get_int() as u32
    } else {
        return JSValue::undefined();
    };

    ctx.event_loop_mut().clear_timer(timer_id);

    JSValue::undefined()
}

fn clear_interval(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    clear_timeout(ctx, args)
}

fn queue_microtask(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }

    let callback = args[0].clone();
    let callback_args: Vec<JSValue> = args.iter().skip(1).cloned().collect();

    ctx.microtask_enqueue(crate::builtins::promise::Microtask::UserCallback(
        callback,
        callback_args,
    ));

    JSValue::undefined()
}

fn request_animation_frame(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_int(0);
    }

    let callback = args[0].clone();
    let id = ctx.event_loop_mut().schedule_animation_callback(callback);

    JSValue::new_int(id as i64)
}

fn cancel_animation_frame(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }

    let id = if args[0].is_int() {
        args[0].get_int() as u32
    } else {
        return JSValue::undefined();
    };

    ctx.event_loop_mut().cancel_animation_callback(id);

    JSValue::undefined()
}

fn js_disassemble(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_function() {
        return JSValue::new_string(ctx.intern("disassemble: expected a function argument"));
    }
    let js_func = args[0].as_function();
    let output = match js_func.bytecode.as_ref() {
        Some(bc) => bc.disassemble(),
        None => {
            let name = ctx.get_atom_str(js_func.name).to_string();
            format!(
                "disassemble: function '{}' has no register bytecode (native/builtin?)",
                name
            )
        }
    };
    JSValue::new_string(ctx.intern(&output))
}

fn js_import(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        let err_msg = JSValue::new_string(ctx.intern("Error: import requires a module specifier"));
        return promise::create_rejected_promise(ctx, err_msg);
    }

    let specifier = if args[0].is_string() {
        ctx.get_atom_str(args[0].get_atom()).to_string()
    } else {
        let err_msg = JSValue::new_string(ctx.intern("Error: import requires a string specifier"));
        return promise::create_rejected_promise(ctx, err_msg);
    };

    match crate::runtime::module::load_and_evaluate_module(ctx, &specifier) {
        Ok(ns_ptr) => {
            let ns_value = JSValue::new_object(ns_ptr);
            let result = promise::create_resolved_promise(ctx, ns_value);
            result
        }
        Err(e) => {
            let err_msg = JSValue::new_string(ctx.intern(&e));
            promise::create_rejected_promise(ctx, err_msg)
        }
    }
}

fn global_isnan(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::bool(true);
    }
    JSValue::bool(args[0].get_float().is_nan())
}

fn global_isfinite(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::bool(false);
    }
    let v = args[0].get_float();
    JSValue::bool(!v.is_nan() && v.is_finite())
}

fn global_eval(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }

    let source = if args[0].is_string() {
        ctx.get_atom_str(args[0].get_atom()).to_string()
    } else if args[0].is_object_like() {
        return args[0].clone();
    } else {
        return args[0].clone();
    };

    if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
        let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
        match vm.direct_eval(ctx, &source) {
            Ok(v) => return v,
            Err(_) => return JSValue::undefined(),
        }
    }

    match crate::eval(ctx, &source) {
        Ok(v) => v,
        Err(_e) => JSValue::undefined(),
    }
}

fn init_reflect(ctx: &mut JSContext) {
    let reflect_obj = JSObject::new();
    let reflect_ptr = Box::into_raw(Box::new(reflect_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(reflect_ptr);
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(ctx.intern("Reflect"), JSValue::new_object(reflect_ptr));
    }
}

fn init_console(ctx: &mut JSContext) {
    let console_atom = ctx.intern("console");
    let mut console_obj = JSObject::new();
    console_obj.set(
        ctx.intern("log"),
        create_builtin_function(ctx, "console_log"),
    );
    console_obj.set(
        ctx.intern("error"),
        create_builtin_function(ctx, "console_error"),
    );
    console_obj.set(
        ctx.intern("warn"),
        create_builtin_function(ctx, "console_warn"),
    );
    console_obj.set(
        ctx.intern("info"),
        create_builtin_function(ctx, "console_info"),
    );
    let console_ptr = Box::into_raw(Box::new(console_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(console_ptr);
    let console_value = JSValue::new_object(console_ptr);
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(console_atom, console_value);
    }
}

fn set_non_configurable(obj: &mut crate::object::object::JSObject, key: crate::runtime::atom::Atom, value: JSValue) {
    obj.define_property(key, crate::object::object::PropertyDescriptor {
        value: Some(value),
        writable: false,
        enumerable: false,
        configurable: false,
        get: None,
        set: None,
    });
}

fn init_math(ctx: &mut JSContext) {
    let math_atom = ctx.intern("Math");
    let mut math_obj = JSObject::new();
    let constants = [
        (ctx.intern("PI"), std::f64::consts::PI),
        (ctx.intern("E"), std::f64::consts::E),
        (ctx.intern("LN2"), std::f64::consts::LN_2),
        (ctx.intern("LN10"), std::f64::consts::LN_10),
        (ctx.intern("LOG2E"), std::f64::consts::LOG2_E),
        (ctx.intern("LOG10E"), std::f64::consts::LOG10_E),
        (ctx.intern("SQRT1_2"), std::f64::consts::FRAC_1_SQRT_2),
        (ctx.intern("SQRT2"), std::f64::consts::SQRT_2),
    ];
    for (key, val) in &constants {
        set_non_configurable(&mut math_obj, *key, JSValue::new_float(*val));
    }
    math_obj.set(ctx.intern("abs"), create_builtin_function(ctx, "math_abs"));
    math_obj.set(
        ctx.intern("floor"),
        create_builtin_function(ctx, "math_floor"),
    );
    math_obj.set(
        ctx.intern("ceil"),
        create_builtin_function(ctx, "math_ceil"),
    );
    math_obj.set(
        ctx.intern("round"),
        create_builtin_function(ctx, "math_round"),
    );
    math_obj.set(
        ctx.intern("sqrt"),
        create_builtin_function(ctx, "math_sqrt"),
    );
    math_obj.set(ctx.intern("max"), create_builtin_function(ctx, "math_max"));
    math_obj.set(ctx.intern("min"), create_builtin_function(ctx, "math_min"));
    math_obj.set(ctx.intern("pow"), create_builtin_function(ctx, "math_pow"));
    math_obj.set(
        ctx.intern("random"),
        create_builtin_function(ctx, "math_random"),
    );
    math_obj.set(
        ctx.intern("trunc"),
        create_builtin_function(ctx, "math_trunc"),
    );
    math_obj.set(
        ctx.intern("sign"),
        create_builtin_function(ctx, "math_sign"),
    );
    math_obj.set(
        ctx.intern("cbrt"),
        create_builtin_function(ctx, "math_cbrt"),
    );
    math_obj.set(
        ctx.intern("clz32"),
        create_builtin_function(ctx, "math_clz32"),
    );
    math_obj.set(
        ctx.intern("imul"),
        create_builtin_function(ctx, "math_imul"),
    );
    math_obj.set(
        ctx.intern("fround"),
        create_builtin_function(ctx, "math_fround"),
    );
    math_obj.set(
        ctx.intern("hypot"),
        create_builtin_function(ctx, "math_hypot"),
    );
    math_obj.set(
        ctx.intern("expm1"),
        create_builtin_function(ctx, "math_expm1"),
    );
    math_obj.set(ctx.intern("log"), create_builtin_function(ctx, "math_log"));
    math_obj.set(
        ctx.intern("log1p"),
        create_builtin_function(ctx, "math_log1p"),
    );
    math_obj.set(
        ctx.intern("log10"),
        create_builtin_function(ctx, "math_log10"),
    );
    math_obj.set(
        ctx.intern("log2"),
        create_builtin_function(ctx, "math_log2"),
    );
    math_obj.set(
        ctx.intern("sinh"),
        create_builtin_function(ctx, "math_sinh"),
    );
    math_obj.set(
        ctx.intern("cosh"),
        create_builtin_function(ctx, "math_cosh"),
    );
    math_obj.set(
        ctx.intern("tanh"),
        create_builtin_function(ctx, "math_tanh"),
    );
    math_obj.set(
        ctx.intern("asinh"),
        create_builtin_function(ctx, "math_asinh"),
    );
    math_obj.set(
        ctx.intern("acosh"),
        create_builtin_function(ctx, "math_acosh"),
    );
    math_obj.set(
        ctx.intern("atanh"),
        create_builtin_function(ctx, "math_atanh"),
    );
    math_obj.set(ctx.intern("sin"), create_builtin_function(ctx, "math_sin"));
    math_obj.set(ctx.intern("cos"), create_builtin_function(ctx, "math_cos"));
    math_obj.set(ctx.intern("tan"), create_builtin_function(ctx, "math_tan"));
    math_obj.set(ctx.intern("exp"), create_builtin_function(ctx, "math_exp"));
    math_obj.set(
        ctx.intern("asin"),
        create_builtin_function(ctx, "math_asin"),
    );
    math_obj.set(
        ctx.intern("acos"),
        create_builtin_function(ctx, "math_acos"),
    );
    math_obj.set(
        ctx.intern("atan"),
        create_builtin_function(ctx, "math_atan"),
    );
    math_obj.set(
        ctx.intern("atan2"),
        create_builtin_function(ctx, "math_atan2"),
    );
    let math_ptr = Box::into_raw(Box::new(math_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(math_ptr);
    let math_value = JSValue::new_object(math_ptr);
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(math_atom, math_value);
    }
}

fn init_json(ctx: &mut JSContext) {
    ctx.register_builtin("json_parse", HostFunction::new("parse", 1, json_parse));
    ctx.register_builtin(
        "json_stringify",
        HostFunction::new("stringify", 1, json_stringify),
    );

    let json_atom = ctx.intern("JSON");
    let mut json_obj = JSObject::new();
    json_obj.set(
        ctx.intern("parse"),
        create_builtin_function(ctx, "json_parse"),
    );
    json_obj.set(
        ctx.intern("stringify"),
        create_builtin_function(ctx, "json_stringify"),
    );
    let json_ptr = Box::into_raw(Box::new(json_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(json_ptr);
    let json_value = JSValue::new_object(json_ptr);
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(json_atom, json_value);
    }
}

fn number_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_int(0);
    }
    let v = &args[0];
    if v.is_int() {
        return *v;
    }
    if v.is_float() {
        return *v;
    }
    if v.is_bool() {
        return JSValue::new_int(if v.get_bool() { 1 } else { 0 });
    }
    if v.is_null() {
        return JSValue::new_int(0);
    }
    if v.is_undefined() {
        return JSValue::new_float(f64::NAN);
    }
    if v.is_string() {
        let s = ctx.get_atom_str(v.get_atom());
        if let Ok(n) = s.parse::<i64>() {
            return JSValue::new_int(n);
        }
        if let Ok(f) = s.parse::<f64>() {
            return JSValue::new_float(f);
        }
        return JSValue::new_float(f64::NAN);
    }
    if v.is_symbol() {
        return JSValue::new_float(f64::NAN);
    }
    if v.is_bigint() {
        if v.is_object() {
            return JSValue::new_int(v.as_object().get_bigint_value() as i64);
        }
        return JSValue::new_int(0);
    }
    JSValue::new_float(f64::NAN)
}

fn init_number(ctx: &mut JSContext) {
    ctx.register_builtin("number_constructor", HostFunction::new("Number", 1, number_constructor));
    ctx.register_builtin("number_isNaN", HostFunction::new("isNaN", 1, number_is_nan));
    ctx.register_builtin(
        "number_isFinite",
        HostFunction::new("isFinite", 1, number_is_finite),
    );
    ctx.register_builtin(
        "number_isInteger",
        HostFunction::new("isInteger", 1, number_is_integer),
    );
    ctx.register_builtin(
        "number_parseInt",
        HostFunction::new("parseInt", 1, global_parseint),
    );
    ctx.register_builtin(
        "number_parseFloat",
        HostFunction::new("parseFloat", 1, global_parsefloat),
    );
    ctx.register_builtin(
        "number_toFixed",
        HostFunction::new("toFixed", 1, number_to_fixed),
    );
    ctx.register_builtin(
        "number_toPrecision",
        HostFunction::new("toPrecision", 1, number_to_precision),
    );
    ctx.register_builtin(
        "number_toString",
        HostFunction::new("toString", 0, number_to_string),
    );
    ctx.register_builtin(
        "number_valueOf",
        HostFunction::new("valueOf", 0, number_value_of),
    );

    let number_atom = ctx.intern("Number");
    let mut num_func = JSFunction::new_builtin(number_atom, 1);
    num_func.set_builtin_marker(ctx, "number_constructor");
    num_func.base.set(ctx.intern("MAX_VALUE"), JSValue::new_float(f64::MAX));
    num_func.base.set(ctx.intern("MIN_VALUE"), JSValue::new_float(f64::MIN));
    num_func.base.set(
        ctx.intern("POSITIVE_INFINITY"),
        JSValue::new_float(f64::INFINITY),
    );
    num_func.base.set(
        ctx.intern("NEGATIVE_INFINITY"),
        JSValue::new_float(f64::NEG_INFINITY),
    );
    num_func.base.set(ctx.intern("NaN"), JSValue::new_float(f64::NAN));
    num_func.base.set(
        ctx.intern("isNaN"),
        create_builtin_function(ctx, "number_isNaN"),
    );
    num_func.base.set(
        ctx.intern("isFinite"),
        create_builtin_function(ctx, "number_isFinite"),
    );
    num_func.base.set(
        ctx.intern("isInteger"),
        create_builtin_function(ctx, "number_isInteger"),
    );
    num_func.base.set(
        ctx.intern("parseInt"),
        create_builtin_function(ctx, "number_parseInt"),
    );
    num_func.base.set(
        ctx.intern("parseFloat"),
        create_builtin_function(ctx, "number_parseFloat"),
    );
    let num_ptr = Box::into_raw(Box::new(num_func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(num_ptr);
    let num_value = JSValue::new_function(num_ptr);
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(number_atom, num_value);
    }

    let mut proto_obj = JSObject::new();
    proto_obj.set(
        ctx.intern("toFixed"),
        create_builtin_function(ctx, "number_toFixed"),
    );
    proto_obj.set(
        ctx.intern("toPrecision"),
        create_builtin_function(ctx, "number_toPrecision"),
    );
    proto_obj.set(
        ctx.intern("toString"),
        create_builtin_function(ctx, "number_toString"),
    );
    proto_obj.set(
        ctx.intern("valueOf"),
        create_builtin_function(ctx, "number_valueOf"),
    );
    proto_obj.set(ctx.common_atoms.constructor, num_value);

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto_obj.prototype = Some(obj_proto_ptr);
    }

    let proto_ptr = Box::into_raw(Box::new(proto_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    ctx.set_number_prototype(proto_ptr);

    let num_func_ptr = num_ptr as *mut crate::object::function::JSFunction;
    unsafe {
        (*num_func_ptr).base.set(ctx.common_atoms.prototype, JSValue::new_object(proto_ptr));
    }
}

fn number_value_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() { return JSValue::undefined(); }
    let this = &args[0];
    if this.is_int() || this.is_float() { return *this; }
    if this.is_object() {
        let obj = this.as_object();
        if let Some(v) = obj.get(ctx.common_atoms.__value__) {
            return v;
        }
    }
    JSValue::undefined()
}

fn number_is_nan(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::bool(false);
    }
    let v = &args[0];
    if v.is_float() {
        JSValue::bool(v.get_float().is_nan())
    } else {
        JSValue::bool(false)
    }
}

fn number_is_finite(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::bool(false);
    }
    let v = &args[0];
    if v.is_int() {
        JSValue::bool(true)
    } else if v.is_float() {
        JSValue::bool(v.get_float().is_finite())
    } else {
        JSValue::bool(false)
    }
}

fn number_is_integer(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::bool(false);
    }
    let v = &args[0];
    if v.is_int() {
        JSValue::bool(true)
    } else if v.is_float() {
        let f = v.get_float();
        JSValue::bool(f.is_finite() && f.fract() == 0.0)
    } else {
        JSValue::bool(false)
    }
}

fn number_to_fixed(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() {
        return JSValue::new_string(_ctx.intern("0"));
    } else {
        &args[0]
    };
    let digits = if args.len() > 1 {
        args[1].get_int() as usize
    } else {
        0
    };

    let val = if this.is_int() {
        this.get_int() as f64
    } else if this.is_float() {
        this.get_float()
    } else {
        return JSValue::new_string(_ctx.intern("NaN"));
    };

    let result = format!("{:.1$}", val, digits);
    JSValue::new_string(_ctx.intern(&result))
}

fn number_to_precision(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() {
        return JSValue::new_string(_ctx.intern("0"));
    } else {
        &args[0]
    };
    let precision = if args.len() > 1 {
        args[1].get_int() as usize
    } else {
        0
    };

    let val = if this.is_int() {
        this.get_int() as f64
    } else if this.is_float() {
        this.get_float()
    } else {
        return JSValue::new_string(_ctx.intern("NaN"));
    };

    if val.is_nan() {
        return JSValue::new_string(_ctx.intern("NaN"));
    }
    if val.is_infinite() {
        return JSValue::new_string(_ctx.intern(if val.is_sign_positive() {
            "Infinity"
        } else {
            "-Infinity"
        }));
    }

    let result = format!("{:.1$}", val, precision);
    JSValue::new_string(_ctx.intern(&result))
}

fn boolean_constructor(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::bool(false);
    }
    JSValue::bool(args[0].is_truthy())
}

fn boolean_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() { return JSValue::new_string(ctx.intern("false")); }
    let this = &args[0];
    if this.is_bool() {
        return JSValue::new_string(ctx.intern(if this.get_bool() { "true" } else { "false" }));
    }
    if this.is_object() {
        let obj = this.as_object();
        if let Some(v) = obj.get(ctx.common_atoms.__value__) {
            if v.is_bool() {
                return JSValue::new_string(ctx.intern(if v.get_bool() { "true" } else { "false" }));
            }
        }
    }
    JSValue::new_string(ctx.intern("false"))
}

fn boolean_value_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() { return JSValue::bool(false); }
    let this = &args[0];
    if this.is_bool() { return *this; }
    if this.is_object() {
        let obj = this.as_object();
        if let Some(v) = obj.get(ctx.common_atoms.__value__) {
            return v;
        }
    }
    JSValue::bool(false)
}

fn init_boolean(ctx: &mut JSContext) {
    ctx.register_builtin(
        "boolean_constructor",
        HostFunction::new("Boolean", 1, boolean_constructor),
    );
    ctx.register_builtin(
        "boolean_toString",
        HostFunction::new("toString", 0, boolean_to_string),
    );
    ctx.register_builtin(
        "boolean_valueOf",
        HostFunction::new("valueOf", 0, boolean_value_of),
    );

    let boolean_atom = ctx.intern("Boolean");
    let mut bool_func = JSFunction::new_builtin(boolean_atom, 1);
    bool_func.set_builtin_marker(ctx, "boolean_constructor");
    let bool_ptr = Box::into_raw(Box::new(bool_func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(bool_ptr);
    let bool_value = JSValue::new_function(bool_ptr);

    let mut proto_obj = JSObject::new();
    proto_obj.set(
        ctx.intern("toString"),
        create_builtin_function(ctx, "boolean_toString"),
    );
    proto_obj.set(
        ctx.intern("valueOf"),
        create_builtin_function(ctx, "boolean_valueOf"),
    );
    proto_obj.set(ctx.common_atoms.constructor, bool_value);
    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto_obj.prototype = Some(obj_proto_ptr);
    }
    let proto_ptr = Box::into_raw(Box::new(proto_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);

    let bool_func_ptr = bool_ptr as *mut crate::object::function::JSFunction;
    unsafe {
        (*bool_func_ptr).base.set(ctx.common_atoms.prototype, JSValue::new_object(proto_ptr));
    }

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(boolean_atom, bool_value);
    }
}

fn init_date(ctx: &mut JSContext) {
    ctx.register_builtin("date_now", HostFunction::new("now", 0, date::date_now));
    ctx.register_builtin(
        "date_parse",
        HostFunction::new("parse", 1, date::date_parse),
    );
    ctx.register_builtin("date_utc", HostFunction::new("UTC", 7, date::date_utc));
    ctx.register_builtin(
        "date_getTime",
        HostFunction::new("getTime", 0, date::date_get_time),
    );
    ctx.register_builtin(
        "date_getFullYear",
        HostFunction::new("getFullYear", 0, date::date_get_full_year),
    );
    ctx.register_builtin(
        "date_getMonth",
        HostFunction::new("getMonth", 0, date::date_get_month),
    );
    ctx.register_builtin(
        "date_getDate",
        HostFunction::new("getDate", 0, date::date_get_date),
    );
    ctx.register_builtin(
        "date_getDay",
        HostFunction::new("getDay", 0, date::date_get_day),
    );
    ctx.register_builtin(
        "date_getHours",
        HostFunction::new("getHours", 0, date::date_get_hours),
    );
    ctx.register_builtin(
        "date_getMinutes",
        HostFunction::new("getMinutes", 0, date::date_get_minutes),
    );
    ctx.register_builtin(
        "date_getSeconds",
        HostFunction::new("getSeconds", 0, date::date_get_seconds),
    );
    ctx.register_builtin(
        "date_getMilliseconds",
        HostFunction::new("getMilliseconds", 0, date::date_get_milliseconds),
    );
    ctx.register_builtin(
        "date_getTimezoneOffset",
        HostFunction::new("getTimezoneOffset", 0, date::date_get_timezone_offset),
    );
    ctx.register_builtin(
        "date_toString",
        HostFunction::new("toString", 0, date::date_to_string),
    );
    ctx.register_builtin(
        "date_toISOString",
        HostFunction::new("toISOString", 0, date::date_to_iso_string),
    );
    ctx.register_builtin(
        "date_toUTCString",
        HostFunction::new("toUTCString", 0, date::date_to_utc_string),
    );
    ctx.register_builtin(
        "date_toDateString",
        HostFunction::new("toDateString", 0, date::date_to_date_string),
    );
    ctx.register_builtin(
        "date_toTimeString",
        HostFunction::new("toTimeString", 0, date::date_to_time_string),
    );
    ctx.register_builtin(
        "date_valueOf",
        HostFunction::new("valueOf", 0, date::date_value_of),
    );
    ctx.register_builtin(
        "date_toPrimitive",
        HostFunction::new("toPrimitive", 1, date::date_to_primitive),
    );

    date::init_date(ctx);
}

fn init_regexp(ctx: &mut JSContext) {
    ctx.register_builtin(
        "regexp_test",
        HostFunction::new("test", 1, regexp::regexp_test),
    );
    ctx.register_builtin(
        "regexp_exec",
        HostFunction::new("exec", 1, regexp::regexp_exec),
    );
    ctx.register_builtin(
        "regexp_toString",
        HostFunction::new("toString", 0, regexp::regexp_to_string),
    );

    regexp::init_regexp(ctx);
}

fn init_promise(ctx: &mut JSContext) {
    promise::init_promise(ctx);
    promise::register_builtins(ctx);
}

fn init_global_funcs(ctx: &mut JSContext) {
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(
            ctx.intern("isNaN"),
            create_builtin_function(ctx, "global_isnan"),
        );
        global_obj.set(
            ctx.intern("isFinite"),
            create_builtin_function(ctx, "global_isfinite"),
        );
        global_obj.set(
            ctx.intern("parseInt"),
            create_builtin_function(ctx, "global_parseint"),
        );
        global_obj.set(
            ctx.intern("parseFloat"),
            create_builtin_function(ctx, "global_parsefloat"),
        );
        global_obj.set(
            ctx.intern("eval"),
            create_builtin_function(ctx, "global_eval"),
        );

        global_obj.set(ctx.intern("btoa"), create_builtin_function(ctx, "btoa"));
        global_obj.set(ctx.intern("atob"), create_builtin_function(ctx, "atob"));

        global_obj.set(
            ctx.intern("encodeURI"),
            create_builtin_function(ctx, "encodeURI"),
        );
        global_obj.set(
            ctx.intern("decodeURI"),
            create_builtin_function(ctx, "decodeURI"),
        );
        global_obj.set(
            ctx.intern("encodeURIComponent"),
            create_builtin_function(ctx, "encodeURIComponent"),
        );
        global_obj.set(
            ctx.intern("decodeURIComponent"),
            create_builtin_function(ctx, "decodeURIComponent"),
        );

        global_obj.set(
            ctx.intern("setTimeout"),
            create_builtin_function(ctx, "setTimeout"),
        );
        global_obj.set(
            ctx.intern("setInterval"),
            create_builtin_function(ctx, "setInterval"),
        );
        global_obj.set(
            ctx.intern("clearTimeout"),
            create_builtin_function(ctx, "clearTimeout"),
        );
        global_obj.set(
            ctx.intern("clearInterval"),
            create_builtin_function(ctx, "clearInterval"),
        );

        global_obj.set(
            ctx.intern("queueMicrotask"),
            create_builtin_function(ctx, "queueMicrotask"),
        );

        global_obj.set(
            ctx.intern("requestAnimationFrame"),
            create_builtin_function(ctx, "requestAnimationFrame"),
        );
        global_obj.set(
            ctx.intern("cancelAnimationFrame"),
            create_builtin_function(ctx, "cancelAnimationFrame"),
        );

        global_obj.set(ctx.intern("globalThis"), global);

        global_obj.set(ctx.intern("import"), create_builtin_function(ctx, "import"));
        global_obj.set(
            ctx.intern("disassemble"),
            create_builtin_function(ctx, "disassemble"),
        );

        use crate::object::object::PropertyDescriptor;
        global_obj.define_property(ctx.intern("undefined"), PropertyDescriptor {
            value: Some(JSValue::undefined()),
            writable: false, enumerable: false, configurable: false,
            get: None, set: None,
        });
        global_obj.define_property(ctx.intern("NaN"), PropertyDescriptor {
            value: Some(JSValue::new_float(f64::NAN)),
            writable: false, enumerable: false, configurable: false,
            get: None, set: None,
        });
        global_obj.define_property(ctx.intern("Infinity"), PropertyDescriptor {
            value: Some(JSValue::new_float(f64::INFINITY)),
            writable: false, enumerable: false, configurable: false,
            get: None, set: None,
        });
        #[cfg(feature = "fetch")]
        {
            global_obj.set(
                ctx.intern("fetch"),
                create_builtin_function(ctx, "fetch"),
            );
            global_obj.set(
                ctx.intern("WebSocket"),
                create_builtin_function(ctx, "WebSocket"),
            );
            global_obj.set(
                ctx.intern("EventSource"),
                create_builtin_function(ctx, "EventSource"),
            );
        }
    }
}
