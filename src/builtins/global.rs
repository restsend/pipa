use crate::builtins::array;
use crate::builtins::base64::{global_atob, global_btoa};
use crate::builtins::bigint;
use crate::builtins::date;
use crate::builtins::error;
#[cfg(feature = "fetch")]
use crate::builtins::eventsource;
#[cfg(feature = "fetch")]
use crate::builtins::fetch;
use crate::builtins::function;
use crate::builtins::generator;
use crate::builtins::intl;
use crate::builtins::json::{json_parse, json_stringify};
use crate::builtins::map_set;
use crate::builtins::math;
use crate::builtins::number::number_to_string;
use crate::builtins::object;
use crate::builtins::parse::{global_parsefloat, global_parseint};
#[cfg(feature = "process")]
use crate::builtins::process;
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
use crate::builtins::websocket;
use crate::host::HostFunction;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::object::object::ObjectType;
use crate::object::object::PropertyDescriptor;
use crate::runtime::context::JSContext;
use crate::value::JSValue;
use std::collections::HashSet;

pub fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let arity = ctx
        .get_builtin_arity(name)
        .unwrap_or(1);
    let mut func = JSFunction::new_builtin(ctx.intern(name), arity);
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
    symbol::init_symbol(ctx);
    init_reflect(ctx);
    init_json(ctx);
    init_global_funcs(ctx);

    object::init_object(ctx);
    init_math(ctx);
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

        let reactor =
            crate::runtime::io_reactor::IoReactor::new().expect("failed to create IoReactor");
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
        let hi_func =
            crate::object::function::JSFunction::new_builtin(ctx.intern("[Symbol.hasInstance]"), 1);
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
    let val = args[0];
    if val.is_int() || val.is_float() {
        JSValue::bool(val.to_number().is_nan())
    } else {
        let n = val.to_number();
        JSValue::bool(n.is_nan())
    }
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
    ctx.register_builtin(
        "reflect_construct",
        HostFunction::new("construct", 2, reflect_construct),
    );
    ctx.register_builtin(
        "reflect_apply",
        HostFunction::new("apply", 3, reflect_apply),
    );

    let reflect_atom = ctx.intern("Reflect");
    let mut reflect_obj = JSObject::new();
    reflect_obj.set(
        ctx.intern("construct"),
        create_builtin_function(ctx, "reflect_construct"),
    );
    reflect_obj.set(
        ctx.intern("apply"),
        create_builtin_function(ctx, "reflect_apply"),
    );
    let reflect_ptr = Box::into_raw(Box::new(reflect_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(reflect_ptr);
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(reflect_atom, JSValue::new_object(reflect_ptr));
    }
}

fn reflect_construct(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_function() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.construct requires a constructor")));
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let target = args[0];
    let new_target = if args.len() >= 3 && args[2].is_function() {
        args[2]
    } else {
        target
    };
    let is_ctor = if new_target.is_function() {
        let f = new_target.as_function();
        if f.is_builtin() {
            ctx.builtin_is_constructor(
                &f.builtin_atom.map(|ba| ctx.get_atom_str(ba).to_string()).unwrap_or_default()
            )
        } else {
            true
        }
    } else {
        false
    };
    if !is_ctor {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("newTarget is not a constructor")));
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let proto_key = ctx.common_atoms.prototype;
    let proto_val = if new_target.is_function() {
        new_target.as_function().base.get(proto_key)
    } else {
        None
    };
    let mut obj = JSObject::new();
    if let Some(pv) = proto_val {
        if pv.is_object() {
            obj.prototype = Some(pv.get_ptr() as *mut JSObject);
        }
    } else if let Some(op) = ctx.get_object_prototype() {
        obj.prototype = Some(op);
    }
    let ptr = Box::into_raw(Box::new(obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}

fn reflect_apply(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 || !args[0].is_function() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.apply requires a function")));
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    JSValue::undefined()
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

fn set_non_configurable(
    obj: &mut crate::object::object::JSObject,
    key: crate::runtime::atom::Atom,
    value: JSValue,
) {
    obj.define_property(
        key,
        crate::object::object::PropertyDescriptor {
            value: Some(value),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );
}

pub fn set_non_enumerable(
    obj: &mut crate::object::object::JSObject,
    key: crate::runtime::atom::Atom,
    value: JSValue,
) {
    obj.define_property(
        key,
        crate::object::object::PropertyDescriptor {
            value: Some(value),
            writable: true,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        },
    );
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
    {
        let key = symbol::get_symbol_to_string_tag_prop_key(ctx);
        let val = JSValue::new_string(ctx.intern("Math"));
        let mut desc = PropertyDescriptor::new_data(val);
        desc.writable = false;
        desc.enumerable = false;
        desc.configurable = true;
        math_obj.define_property_ext(key, desc, true, true, true);
    }
    set_non_enumerable(&mut math_obj, ctx.intern("abs"), create_builtin_function(ctx, "math_abs"));
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("floor"),
        create_builtin_function(ctx, "math_floor"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("ceil"),
        create_builtin_function(ctx, "math_ceil"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("round"),
        create_builtin_function(ctx, "math_round"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("sqrt"),
        create_builtin_function(ctx, "math_sqrt"),
    );
    set_non_enumerable(&mut math_obj, ctx.intern("max"), create_builtin_function(ctx, "math_max"));
    set_non_enumerable(&mut math_obj, ctx.intern("min"), create_builtin_function(ctx, "math_min"));
    set_non_enumerable(&mut math_obj, ctx.intern("pow"), create_builtin_function(ctx, "math_pow"));
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("random"),
        create_builtin_function(ctx, "math_random"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("trunc"),
        create_builtin_function(ctx, "math_trunc"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("sign"),
        create_builtin_function(ctx, "math_sign"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("cbrt"),
        create_builtin_function(ctx, "math_cbrt"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("clz32"),
        create_builtin_function(ctx, "math_clz32"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("imul"),
        create_builtin_function(ctx, "math_imul"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("fround"),
        create_builtin_function(ctx, "math_fround"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("f16round"),
        create_builtin_function(ctx, "math_f16round"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("sumPrecise"),
        create_builtin_function(ctx, "math_sumPrecise"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("hypot"),
        create_builtin_function(ctx, "math_hypot"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("expm1"),
        create_builtin_function(ctx, "math_expm1"),
    );
    set_non_enumerable(&mut math_obj, ctx.intern("log"), create_builtin_function(ctx, "math_log"));
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("log1p"),
        create_builtin_function(ctx, "math_log1p"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("log10"),
        create_builtin_function(ctx, "math_log10"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("log2"),
        create_builtin_function(ctx, "math_log2"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("sinh"),
        create_builtin_function(ctx, "math_sinh"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("cosh"),
        create_builtin_function(ctx, "math_cosh"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("tanh"),
        create_builtin_function(ctx, "math_tanh"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("asinh"),
        create_builtin_function(ctx, "math_asinh"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("acosh"),
        create_builtin_function(ctx, "math_acosh"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("atanh"),
        create_builtin_function(ctx, "math_atanh"),
    );
    set_non_enumerable(&mut math_obj, ctx.intern("sin"), create_builtin_function(ctx, "math_sin"));
    set_non_enumerable(&mut math_obj, ctx.intern("cos"), create_builtin_function(ctx, "math_cos"));
    set_non_enumerable(&mut math_obj, ctx.intern("tan"), create_builtin_function(ctx, "math_tan"));
    set_non_enumerable(&mut math_obj, ctx.intern("exp"), create_builtin_function(ctx, "math_exp"));
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("asin"),
        create_builtin_function(ctx, "math_asin"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("acos"),
        create_builtin_function(ctx, "math_acos"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("atan"),
        create_builtin_function(ctx, "math_atan"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("atan2"),
        create_builtin_function(ctx, "math_atan2"),
    );
    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        math_obj.set_prototype_raw(obj_proto_ptr);
    }
    let math_ptr = Box::into_raw(Box::new(math_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(math_ptr);
    let math_value = JSValue::new_object(math_ptr);
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        set_non_enumerable(global_obj, math_atom, math_value);
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

pub fn throw_type_error(ctx: &mut JSContext, msg: &str) -> JSValue {
    let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
    err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
    err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern(msg)));
    if let Some(proto) = ctx.get_type_error_prototype() {
        err.prototype = Some(proto);
    }
    let ptr = Box::into_raw(Box::new(err)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    ctx.pending_exception = Some(JSValue::new_object(ptr));
    JSValue::undefined()
}

pub fn throw_type_error_if_no_exception(ctx: &mut JSContext, msg: &str) -> JSValue {
    if ctx.pending_exception.is_some() {
        return JSValue::undefined();
    }
    throw_type_error(ctx, msg)
}

fn js_to_primitive_number(ctx: &mut JSContext, v: &JSValue) -> Option<JSValue> {
    if !v.is_object() {
        return Some(v.clone());
    }
    let obj = v.as_object();
    if let Some(val) = obj.get(ctx.common_atoms.__value__) {
        if !val.is_object() && !val.is_function() {
            return Some(val);
        }
    }
    if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
        let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
        let valueOf_atom = ctx.intern("valueOf");
        let to_string_atom = ctx.intern("toString");
        if let Some(valueof_fn) = obj.get(valueOf_atom) {
            if valueof_fn.is_function() {
                let result = vm.call_function_with_this(ctx, valueof_fn, v.clone(), &[]);
                if ctx.pending_exception.is_some() {
                    return None;
                }
                if let Ok(val) = result {
                    if !val.is_object() && !val.is_function() {
                        return Some(val);
                    }
                }
            }
        }
        if ctx.pending_exception.is_some() {
            return None;
        }
        if let Some(tostring_fn) = obj.get(to_string_atom) {
            if tostring_fn.is_function() {
                let result = vm.call_function_with_this(ctx, tostring_fn, v.clone(), &[]);
                if ctx.pending_exception.is_some() {
                    return None;
                }
                if let Ok(val) = result {
                    if !val.is_object() && !val.is_function() {
                        return Some(val);
                    }
                }
            }
        }
        if ctx.pending_exception.is_some() {
            return None;
        }
    }
    None
}

pub fn js_to_number_value(ctx: &mut JSContext, v: &JSValue) -> Result<f64, ()> {
    if v.is_int() {
        return Ok(v.get_int() as f64);
    }
    if v.is_float() {
        return Ok(v.get_float());
    }
    if v.is_bool() {
        return Ok(if v.get_bool() { 1.0 } else { 0.0 });
    }
    if v.is_null() {
        return Ok(0.0);
    }
    if v.is_undefined() {
        return Ok(f64::NAN);
    }
    if v.is_symbol() {
        return Err(());
    }
    if v.is_bigint() {
        let bv = v.as_object().get_bigint_value();
        return Ok(bv as f64);
    }
    if v.is_string() {
        let s = ctx.get_atom_str(v.get_atom());
        let s = s.trim();
        if s.starts_with("0b") || s.starts_with("0B") {
            return Ok(u64::from_str_radix(&s[2..], 2).map(|n| n as f64).unwrap_or(f64::NAN));
        }
        if s.starts_with("0o") || s.starts_with("0O") {
            return Ok(u64::from_str_radix(&s[2..], 8).map(|n| n as f64).unwrap_or(f64::NAN));
        }
        if s.starts_with("0x") || s.starts_with("0X") {
            return Ok(u64::from_str_radix(&s[2..], 16).map(|n| n as f64).unwrap_or(f64::NAN));
        }
        if let Ok(n) = s.parse::<i64>() {
            return Ok(n as f64);
        }
        if let Ok(f) = s.parse::<f64>() {
            return Ok(f);
        }
        return Ok(f64::NAN);
    }
    if v.is_object() {
        let obj = v.as_object();
        let obj_type = obj.obj_type();
        if obj_type == crate::object::object::ObjectType::Array {
            let elements: Vec<JSValue> = if obj.is_dense_array() {
                use crate::object::array_obj::JSArrayObject;
                let arr = unsafe { &*(v.get_ptr() as *const JSArrayObject) };
                arr.elements.clone()
            } else if let Some(elems) = obj.get_array_elements() {
                elems.clone()
            } else {
                return Ok(0.0);
            };
            if elements.is_empty() {
                return Ok(0.0);
            }
            let mut parts = Vec::new();
            for elem in elements.iter() {
                if elem.is_undefined() || elem.is_null() {
                    parts.push("".to_string());
                } else if elem.is_string() {
                    parts.push(ctx.get_atom_str(elem.get_atom()).to_string());
                } else if elem.is_int() {
                    parts.push(format!("{}", elem.get_int()));
                } else if elem.is_float() {
                    parts.push(string::js_float_to_string(elem.get_float()));
                } else if elem.is_bool() {
                    parts.push(if elem.get_bool() { "true".to_string() } else { "false".to_string() });
                } else {
                    parts.push("".to_string());
                }
            }
            let joined = parts.join(",");
            return Ok(joined.trim().parse::<f64>().unwrap_or(f64::NAN));
        }
        if let Some(prim) = js_to_primitive_number(ctx, v) {
            return js_to_number_value(ctx, &prim);
        }
        return Err(());
    }
    Ok(f64::NAN)
}

fn number_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_int(0);
    }
    match js_to_number_value(ctx, &args[0]) {
        Ok(f) => {
            if f.is_nan() {
                return JSValue::new_float(f64::NAN);
            }
            if f == f.floor() && f.is_finite()
                && f.abs() < 140737488355328.0
            {
                return JSValue::new_int(f as i64);
            }
            JSValue::new_float(f)
        }
        Err(()) => {
            if ctx.pending_exception.is_some() {
                return JSValue::undefined();
            }
            throw_type_error(ctx, "Cannot convert Symbol to a number")
        }
    }
}

fn init_number(ctx: &mut JSContext) {
    ctx.register_builtin(
        "number_constructor",
        HostFunction::ctor("Number", 1, number_constructor),
    );
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
        "number_isSafeInteger",
        HostFunction::new("isSafeInteger", 1, number_is_safe_integer),
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
        HostFunction::method("toFixed", 1, number_to_fixed),
    );
    ctx.register_builtin(
        "number_toExponential",
        HostFunction::method("toExponential", 1, number_to_exponential),
    );
    ctx.register_builtin(
        "number_toPrecision",
        HostFunction::method("toPrecision", 1, number_to_precision),
    );
    ctx.register_builtin(
        "number_toString",
        HostFunction::new("toString", 1, number_to_string),
    );
    ctx.register_builtin(
        "number_toLocaleString",
        HostFunction::new("toLocaleString", 0, number_to_string),
    );
    ctx.register_builtin(
        "number_valueOf",
        HostFunction::method("valueOf", 0, number_value_of),
    );

    let number_atom = ctx.intern("Number");
    let mut num_func = JSFunction::new_builtin(number_atom, 1);
    num_func.set_builtin_marker(ctx, "number_constructor");
    set_non_configurable(
        &mut num_func.base,
        ctx.intern("MAX_VALUE"),
        JSValue::new_float(1.7976931348623157e+308),
    );
    set_non_configurable(
        &mut num_func.base,
        ctx.intern("MIN_VALUE"),
        JSValue::new_float(5e-324),
    );
    set_non_configurable(
        &mut num_func.base,
        ctx.intern("POSITIVE_INFINITY"),
        JSValue::new_float(f64::INFINITY),
    );
    set_non_configurable(
        &mut num_func.base,
        ctx.intern("NEGATIVE_INFINITY"),
        JSValue::new_float(f64::NEG_INFINITY),
    );
    set_non_configurable(
        &mut num_func.base,
        ctx.intern("NaN"),
        JSValue::new_float(f64::NAN),
    );
    set_non_configurable(
        &mut num_func.base,
        ctx.intern("MAX_SAFE_INTEGER"),
        JSValue::new_float(9007199254740991.0),
    );
    set_non_configurable(
        &mut num_func.base,
        ctx.intern("MIN_SAFE_INTEGER"),
        JSValue::new_float(-9007199254740991.0),
    );
    set_non_configurable(
        &mut num_func.base,
        ctx.intern("EPSILON"),
        JSValue::new_float(f64::EPSILON),
    );
    set_non_enumerable(&mut num_func.base, ctx.intern("isNaN"), create_builtin_function(ctx, "number_isNaN"));
    set_non_enumerable(&mut num_func.base, ctx.intern("isFinite"), create_builtin_function(ctx, "number_isFinite"));
    set_non_enumerable(&mut num_func.base, ctx.intern("isInteger"), create_builtin_function(ctx, "number_isInteger"));
    set_non_enumerable(&mut num_func.base, ctx.intern("isSafeInteger"), create_builtin_function(ctx, "number_isSafeInteger"));
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object();
        if let Some(parse_int_fn) = global_obj.get(ctx.intern("parseInt")) {
            set_non_enumerable(&mut num_func.base, ctx.intern("parseInt"), parse_int_fn);
        }
        if let Some(parse_float_fn) = global_obj.get(ctx.intern("parseFloat")) {
            set_non_enumerable(&mut num_func.base, ctx.intern("parseFloat"), parse_float_fn);
        }
    }
    let num_ptr = Box::into_raw(Box::new(num_func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(num_ptr);
    let num_value = JSValue::new_function(num_ptr);
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        set_non_enumerable(global_obj, number_atom, num_value);
    }

    let mut number_proto = JSObject::new();
    number_proto.set(ctx.common_atoms.__value__, JSValue::new_int(0));
    set_non_enumerable(
        &mut number_proto,
        ctx.intern("toFixed"),
        create_builtin_function(ctx, "number_toFixed"),
    );
    set_non_enumerable(
        &mut number_proto,
        ctx.intern("toExponential"),
        create_builtin_function(ctx, "number_toExponential"),
    );
    set_non_enumerable(
        &mut number_proto,
        ctx.intern("toPrecision"),
        create_builtin_function(ctx, "number_toPrecision"),
    );
    set_non_enumerable(
        &mut number_proto,
        ctx.intern("toString"),
        create_builtin_function(ctx, "number_toString"),
    );
    set_non_enumerable(
        &mut number_proto,
        ctx.intern("valueOf"),
        create_builtin_function(ctx, "number_valueOf"),
    );
    set_non_enumerable(
        &mut number_proto,
        ctx.intern("toLocaleString"),
        create_builtin_function(ctx, "number_toLocaleString"),
    );
    set_non_enumerable(
        &mut number_proto,
        ctx.common_atoms.constructor,
        num_value,
    );

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        number_proto.prototype = Some(obj_proto_ptr);
    }

    let proto_ptr = Box::into_raw(Box::new(number_proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    ctx.set_number_prototype(proto_ptr);

    let num_func_ptr = num_ptr as *mut crate::object::function::JSFunction;
    unsafe {
        use crate::object::object::PropertyDescriptor;
        let mut desc = PropertyDescriptor::new_data(JSValue::new_object(proto_ptr));
        desc.writable = false;
        desc.enumerable = false;
        desc.configurable = false;
        (*num_func_ptr).base.define_property(ctx.common_atoms.prototype, desc);
    }
}

fn number_value_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this = &args[0];
    if this.is_int() || this.is_float() {
        return *this;
    }
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

fn number_is_safe_integer(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::bool(false);
    }
    let v = &args[0];
    if v.is_int() {
        JSValue::bool(true)
    } else if v.is_float() {
        let f = v.get_float();
        if !f.is_finite() || f.fract() != 0.0 {
            return JSValue::bool(false);
        }
        let abs = f.abs();
        JSValue::bool(abs <= 9007199254740991.0)
    } else {
        JSValue::bool(false)
    }
}

fn to_integer_or_nan(ctx: &mut JSContext, v: &JSValue) -> Result<f64, ()> {
    if v.is_symbol() {
        return Err(());
    }
    if v.is_bigint() {
        return Err(());
    }
    if v.is_object() {
        let obj = v.as_object();
        let obj_type = obj.obj_type();
        if obj_type == crate::object::object::ObjectType::Array {
            let elements: Vec<JSValue> = if obj.is_dense_array() {
                use crate::object::array_obj::JSArrayObject;
                let arr = unsafe { &*(v.get_ptr() as *const JSArrayObject) };
                arr.elements.clone()
            } else if let Some(elems) = obj.get_array_elements() {
                elems.clone()
            } else {
                return Ok(0.0);
            };
            if elements.is_empty() {
                    return Ok(0.0);
                }
                let mut parts = Vec::new();
                for elem in elements.iter() {
                    if elem.is_undefined() || elem.is_null() {
                        parts.push("".to_string());
                    } else if elem.is_string() {
                        parts.push(ctx.get_atom_str(elem.get_atom()).to_string());
                    } else if elem.is_int() {
                        parts.push(format!("{}", elem.get_int()));
                    } else if elem.is_float() {
                        parts.push(string::js_float_to_string(elem.get_float()));
                    } else if elem.is_bool() {
                        parts.push(if elem.get_bool() { "true".to_string() } else { "false".to_string() });
                    } else {
                        parts.push("".to_string());
                    }
                }
                let joined = parts.join(",");
                let f = joined.trim().parse::<f64>().unwrap_or(f64::NAN);
                if f.is_nan() { return Ok(0.0); }
                if f.is_infinite() { return Ok(f); }
                return Ok(f.trunc());
        }
        if let Some(prim) = js_to_primitive_number(ctx, v) {
            if prim.is_symbol() {
                return Err(());
            }
            return to_integer_or_nan(ctx, &prim);
        }
        return Err(());
    }
    let f = match js_to_number_value(ctx, v) {
        Ok(f) => f,
        Err(()) => return Err(()),
    };
    if f.is_nan() { return Ok(0.0); }
    if f.is_infinite() { return Ok(f); }
    Ok(f.trunc())
}

fn this_number_value(ctx: &mut JSContext, this: &JSValue) -> Option<f64> {
    if this.is_int() {
        return Some(this.get_int() as f64);
    }
    if this.is_float() {
        return Some(this.get_float());
    }
    if this.is_object() {
        let obj = this.as_object();
        if let Some(v) = obj.get(ctx.common_atoms.__value__) {
            if v.is_int() {
                return Some(v.get_int() as f64);
            }
            if v.is_float() {
                return Some(v.get_float());
            }
        }
    }
    None
}

fn throw_range_error(ctx: &mut JSContext, msg: &str) -> JSValue {
    let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
    err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("RangeError")));
    err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern(msg)));
    if let Some(proto) = ctx.get_range_error_prototype() {
        err.prototype = Some(proto);
    }
    let ptr = Box::into_raw(Box::new(err)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    ctx.pending_exception = Some(JSValue::new_object(ptr));
    JSValue::undefined()
}

fn throw_type_error_number(ctx: &mut JSContext) -> JSValue {
    let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
    err.set(
        ctx.common_atoms.name,
        JSValue::new_string(ctx.intern("TypeError")),
    );
    err.set(
        ctx.common_atoms.message,
        JSValue::new_string(ctx.intern("this is not a number")),
    );
    if let Some(proto) = ctx.get_type_error_prototype() {
        err.prototype = Some(proto);
    }
    let ptr = Box::into_raw(Box::new(err)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    ctx.pending_exception = Some(JSValue::new_object(ptr));
    JSValue::undefined()
}

fn format_exponential(val: f64, fraction_digits: Option<usize>) -> String {
    if val == 0.0 {
        return if let Some(digits) = fraction_digits {
            if digits == 0 {
                "0e+0".to_string()
            } else {
                format!("0.{:0<width$}e+0", "", width = digits)
            }
        } else {
            "0e+0".to_string()
        };
    }
    let sign = if val < 0.0 { "-" } else { "" };
    let abs = val.abs();
    if let Some(digits) = fraction_digits {
        let high_prec = format!("{:.20e}", abs);
        let mut parts: Vec<&str> = high_prec.splitn(2, 'e').collect();
        let m_str = parts[0];
        let e_val: i32 = parts.get(1).unwrap_or(&"0").parse().unwrap_or(0);
        let m_digits: Vec<char> = m_str.replace(".", "").chars().collect();
        let total_digits = digits + 1;
        if total_digits < m_digits.len() {
            let mut result_digits: Vec<char> = m_digits[..total_digits].to_vec();
            let next = m_digits[total_digits];
            if next >= '5' {
                let mut carry = true;
                for i in (0..result_digits.len()).rev() {
                    if carry {
                        let d = result_digits[i].to_digit(10).unwrap();
                        if d == 9 {
                            result_digits[i] = '0';
                        } else {
                            result_digits[i] = char::from_digit(d + 1, 10).unwrap();
                            carry = false;
                        }
                    }
                }
                if carry {
                    result_digits.insert(0, '1');
                    let m = if digits == 0 {
                        "1".to_string()
                    } else {
                        let mut s = "1.".to_string();
                        for _ in 0..digits {
                            s.push('0');
                        }
                        s
                    };
                    return format!("{}{}e{:+}", sign, m, e_val + 1);
                }
            }
            let m = if digits == 0 {
                result_digits.iter().collect::<String>()
            } else {
                let mut s = result_digits[0].to_string();
                s.push('.');
                for &d in &result_digits[1..] {
                    s.push(d);
                }
                s
            };
            format!("{}{}e{:+}", sign, m, e_val)
        } else {
            let m = if digits == 0 {
                m_digits.iter().collect::<String>()
            } else {
                let mut s = m_digits[0].to_string();
                s.push('.');
                for &d in &m_digits[1..] {
                    s.push(d);
                }
                while s.len() < 2 + digits {
                    s.push('0');
                }
                s
            };
            format!("{}{}e{:+}", sign, m, e_val)
        }
    } else {
        let rust_fmt = format!("{:.15e}", abs);
        let mut parts: Vec<&str> = rust_fmt.splitn(2, 'e').collect();
        let m = parts[0].trim_end_matches('0');
        let m = if m.ends_with('.') { &m[..m.len()-1] } else { m };
        let e: i32 = parts.get(1).unwrap_or(&"0").parse().unwrap_or(0);
        format!("{}{}e{:+}", sign, m, e)
    }
}

fn number_to_fixed(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() {
        return throw_type_error_number(ctx);
    } else {
        &args[0]
    };
    let val = match this_number_value(ctx, this) {
        Some(v) => v,
        None => return throw_type_error_number(ctx),
    };
    let digits = if args.len() > 1 && !args[1].is_undefined() {
        let d = match to_integer_or_nan(ctx, &args[1]) {
            Ok(d) => d,
            Err(()) => return throw_type_error_if_no_exception(ctx, "toFixed() argument cannot be converted to a number"),
        };
        if d < 0.0 || d > 100.0 {
            return throw_range_error(ctx, "toFixed() digits argument must be between 0 and 100");
        }
        d as usize
    } else {
        0
    };
    if val.is_nan() {
        return JSValue::new_string(ctx.intern("NaN"));
    }
    if val.is_infinite() {
        return JSValue::new_string(ctx.intern(if val.is_sign_positive() {
            "Infinity"
        } else {
            "-Infinity"
        }));
    }
    let result = format!("{:.1$}", val, digits);
    JSValue::new_string(ctx.intern(&result))
}

fn number_to_exponential(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() {
        return throw_type_error_number(ctx);
    } else {
        &args[0]
    };
    let val = match this_number_value(ctx, this) {
        Some(v) => v,
        None => return throw_type_error_number(ctx),
    };
    let fraction_digits = if args.len() > 1 && !args[1].is_undefined() {
        let d = match to_integer_or_nan(ctx, &args[1]) {
            Ok(d) => d,
            Err(()) => return throw_type_error_if_no_exception(ctx, "toExponential() argument cannot be converted to a number"),
        };
        Some(d)
    } else {
        None
    };
    if val.is_nan() {
        return JSValue::new_string(ctx.intern("NaN"));
    }
    if val.is_infinite() {
        return JSValue::new_string(ctx.intern(if val.is_sign_positive() {
            "Infinity"
        } else {
            "-Infinity"
        }));
    }
    let fraction_digits = match fraction_digits {
        Some(d) => {
            if d < 0.0 || d > 100.0 {
                return throw_range_error(ctx, "toExponential() fractionDigits must be between 0 and 100");
            }
            Some(d as usize)
        }
        None => None,
    };
    if let Some(d) = fraction_digits {
        if d > 100 {
            return throw_range_error(ctx, "toExponential() fractionDigits must be between 0 and 100");
        }
    }
    let result = format_exponential(val, fraction_digits);
    JSValue::new_string(ctx.intern(&result))
}

fn number_to_precision(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() {
        return throw_type_error_number(ctx);
    } else {
        &args[0]
    };
    let val = match this_number_value(ctx, this) {
        Some(v) => v,
        None => return throw_type_error_number(ctx),
    };
    if args.len() <= 1 || args[1].is_undefined() {
        if val.is_nan() {
            return JSValue::new_string(ctx.intern("NaN"));
        }
        if val.is_infinite() {
            return JSValue::new_string(ctx.intern(if val.is_sign_positive() {
                "Infinity"
            } else {
                "-Infinity"
            }));
        }
        return JSValue::new_string(ctx.intern(&format!("{}", val)));
    }
    let precision = match to_integer_or_nan(ctx, &args[1]) {
        Ok(p) => p,
        Err(()) => return throw_type_error_if_no_exception(ctx, "toPrecision() argument cannot be converted to a number"),
    };
    if val.is_nan() {
        return JSValue::new_string(ctx.intern("NaN"));
    }
    if val.is_infinite() {
        return JSValue::new_string(ctx.intern(if val.is_sign_positive() {
            "Infinity"
        } else {
            "-Infinity"
        }));
    }
    if precision < 1.0 || precision > 100.0 {
        return throw_range_error(ctx, "toPrecision() precision must be between 1 and 100");
    }
    let p = precision as usize;
    let sign = if val < 0.0 { "-" } else { "" };
    let abs = val.abs();
    if abs == 0.0 {
        let s = if p == 1 {
            "0".to_string()
        } else {
            format!("0.{:0<width$}", "", width = p - 1)
        };
        return JSValue::new_string(ctx.intern(&format!("{}{}", sign, s)));
    }
    let e = abs.log10().floor() as i32;
    let exp = e + 1;
    if e >= -6 && e < p as i32 {
        let exp_str = format_exponential(abs, Some(p - 1));
        let mut parts: Vec<&str> = exp_str.splitn(2, 'e').collect();
        let mantissa = parts[0].replace(".", "");
        let e_val: i32 = parts.get(1).unwrap_or(&"+0").parse().unwrap_or(0);
        let n_digits = mantissa.len();
        let dot_pos = e_val + 1;
        let mut s = String::new();
        if dot_pos <= 0 {
            s.push_str("0.");
            for _ in 0..(-dot_pos) {
                s.push('0');
            }
            s.push_str(&mantissa);
        } else if dot_pos >= n_digits as i32 {
            s.push_str(&mantissa);
            for _ in 0..(dot_pos - n_digits as i32) {
                s.push('0');
            }
        } else {
            s.push_str(&mantissa[..dot_pos as usize]);
            s.push('.');
            s.push_str(&mantissa[dot_pos as usize..]);
        }
        return JSValue::new_string(ctx.intern(&format!("{}{}", sign, s)));
    }
    let m_digits = p - 1;
    let result = format_exponential(abs, Some(m_digits));
    JSValue::new_string(ctx.intern(&format!("{}{}", sign, result)))
}

fn boolean_constructor(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let val = if !args.is_empty() { args[0].is_truthy() } else { false };
    JSValue::bool(val)
}

fn boolean_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern("false"));
    }
    let this = &args[0];
    if this.is_bool() {
        return JSValue::new_string(ctx.intern(if this.get_bool() { "true" } else { "false" }));
    }
    if this.is_object() {
        let obj = this.as_object();
        if let Some(v) = obj.get(ctx.common_atoms.__value__) {
            if v.is_bool() {
                return JSValue::new_string(ctx.intern(if v.get_bool() {
                    "true"
                } else {
                    "false"
                }));
            }
        }
    }
    JSValue::new_string(ctx.intern("false"))
}

fn boolean_value_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Boolean.prototype.valueOf requires 'this' to be a Boolean")));
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let this = &args[0];
    if this.is_bool() {
        return *this;
    }
    if this.is_object() {
        let obj = this.as_object();
        if let Some(v) = obj.get(ctx.common_atoms.__value__) {
            if v.is_bool() {
                return v;
            }
        }
    }
    let mut err = JSObject::new();
    err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
    err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Boolean.prototype.valueOf requires 'this' to be a Boolean")));
    if let Some(proto) = ctx.get_type_error_prototype() {
        err.prototype = Some(proto);
    }
    let ptr = Box::into_raw(Box::new(err)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    ctx.pending_exception = Some(JSValue::new_object(ptr));
    JSValue::undefined()
}

fn init_boolean(ctx: &mut JSContext) {
    ctx.register_builtin(
        "boolean_constructor",
        HostFunction::ctor("Boolean", 1, boolean_constructor),
    );
    ctx.register_builtin(
        "boolean_toString",
        HostFunction::method("toString", 0, boolean_to_string),
    );
    ctx.register_builtin(
        "boolean_valueOf",
        HostFunction::method("valueOf", 0, boolean_value_of),
    );

    let boolean_atom = ctx.intern("Boolean");
    let mut bool_func = JSFunction::new_builtin(boolean_atom, 1);
    bool_func.set_builtin_marker(ctx, "boolean_constructor");
    if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
        bool_func.base.set_prototype_raw(fn_proto_ptr);
    }
    let bool_ptr = Box::into_raw(Box::new(bool_func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(bool_ptr);
    let bool_value = JSValue::new_function(bool_ptr);

    let mut proto_obj = JSObject::new_typed(ObjectType::Boolean);
    proto_obj.set(ctx.common_atoms.__value__, JSValue::bool(false));
    proto_obj.set(
        ctx.intern("toString"),
        create_builtin_function(ctx, "boolean_toString"),
    );
    proto_obj.set(
        ctx.intern("valueOf"),
        create_builtin_function(ctx, "boolean_valueOf"),
    );
    proto_obj.define_property(
        ctx.common_atoms.constructor,
        PropertyDescriptor {
            value: Some(bool_value),
            writable: true,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        },
    );
    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto_obj.prototype = Some(obj_proto_ptr);
    }
    let proto_ptr = Box::into_raw(Box::new(proto_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);

    let bool_func_ptr = bool_ptr as *mut crate::object::function::JSFunction;
    unsafe {
        (*bool_func_ptr)
            .base
            .define_property(
                ctx.common_atoms.prototype,
                PropertyDescriptor {
                    value: Some(JSValue::new_object(proto_ptr)),
                    writable: false,
                    enumerable: false,
                    configurable: false,
                    get: None,
                    set: None,
                },
            );
        (*bool_func_ptr)
            .base
            .define_property(
                ctx.intern("length"),
                PropertyDescriptor {
                    value: Some(JSValue::new_float(1.0)),
                    writable: false,
                    enumerable: false,
                    configurable: true,
                    get: None,
                    set: None,
                },
            );
    }

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        set_non_enumerable(global_obj, boolean_atom, bool_value);
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
        HostFunction::method("getTime", 0, date::date_get_time),
    );
    ctx.register_builtin(
        "date_getFullYear",
        HostFunction::method("getFullYear", 0, date::date_get_full_year),
    );
    ctx.register_builtin(
        "date_getMonth",
        HostFunction::method("getMonth", 0, date::date_get_month),
    );
    ctx.register_builtin(
        "date_getDate",
        HostFunction::method("getDate", 0, date::date_get_date),
    );
    ctx.register_builtin(
        "date_getDay",
        HostFunction::method("getDay", 0, date::date_get_day),
    );
    ctx.register_builtin(
        "date_getHours",
        HostFunction::method("getHours", 0, date::date_get_hours),
    );
    ctx.register_builtin(
        "date_getMinutes",
        HostFunction::method("getMinutes", 0, date::date_get_minutes),
    );
    ctx.register_builtin(
        "date_getSeconds",
        HostFunction::method("getSeconds", 0, date::date_get_seconds),
    );
    ctx.register_builtin(
        "date_getMilliseconds",
        HostFunction::method("getMilliseconds", 0, date::date_get_milliseconds),
    );
    ctx.register_builtin(
        "date_getTimezoneOffset",
        HostFunction::method("getTimezoneOffset", 0, date::date_get_timezone_offset),
    );
    ctx.register_builtin(
        "date_toString",
        HostFunction::method("toString", 0, date::date_to_string),
    );
    ctx.register_builtin(
        "date_toISOString",
        HostFunction::method("toISOString", 0, date::date_to_iso_string),
    );
    ctx.register_builtin(
        "date_toUTCString",
        HostFunction::method("toUTCString", 0, date::date_to_utc_string),
    );
    ctx.register_builtin(
        "date_toDateString",
        HostFunction::method("toDateString", 0, date::date_to_date_string),
    );
    ctx.register_builtin(
        "date_toTimeString",
        HostFunction::method("toTimeString", 0, date::date_to_time_string),
    );
    ctx.register_builtin(
        "date_valueOf",
        HostFunction::method("valueOf", 0, date::date_value_of),
    );
    ctx.register_builtin(
        "date_toPrimitive",
        HostFunction::method("toPrimitive", 1, date::date_to_primitive),
    );

    date::init_date(ctx);
}

fn init_regexp(ctx: &mut JSContext) {
    ctx.register_builtin(
        "regexp_test",
        HostFunction::method("test", 1, regexp::regexp_test),
    );
    ctx.register_builtin(
        "regexp_exec",
        HostFunction::method("exec", 1, regexp::regexp_exec),
    );
    ctx.register_builtin(
        "regexp_toString",
        HostFunction::method("toString", 0, regexp::regexp_to_string),
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
        global_obj.define_property(
            ctx.intern("undefined"),
            PropertyDescriptor {
                value: Some(JSValue::undefined()),
                writable: false,
                enumerable: false,
                configurable: false,
                get: None,
                set: None,
            },
        );
        global_obj.define_property(
            ctx.intern("NaN"),
            PropertyDescriptor {
                value: Some(JSValue::new_float(f64::NAN)),
                writable: false,
                enumerable: false,
                configurable: false,
                get: None,
                set: None,
            },
        );
        global_obj.define_property(
            ctx.intern("Infinity"),
            PropertyDescriptor {
                value: Some(JSValue::new_float(f64::INFINITY)),
                writable: false,
                enumerable: false,
                configurable: false,
                get: None,
                set: None,
            },
        );
        #[cfg(feature = "fetch")]
        {
            global_obj.set(ctx.intern("fetch"), create_builtin_function(ctx, "fetch"));
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
