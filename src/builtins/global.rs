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
use crate::builtins::json::{json_is_raw_json, json_parse, json_raw_json, json_stringify};
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
use crate::runtime::atom::Atom;
use crate::runtime::context::JSContext;
use crate::value::JSValue;
use std::collections::HashSet;

pub fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let arity = ctx.get_builtin_arity(name).unwrap_or(1);
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
        HostFunction::new("parseInt", 2, global_parseint),
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

    if let Some(obj_proto) = ctx.get_object_prototype() {
        ctx.global().as_object_mut().prototype = Some(obj_proto);
    }

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
    symbol::fix_prototype_chain(ctx);
    date::init_date_to_primitive(ctx);
    bigint::init_bigint(ctx);
    weakref::init_weakref(ctx);
    generator::init_generator(ctx);
    typedarray::init_typed_array(ctx);
    intl::init_intl(ctx);

    if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
        let global = ctx.global();
        if global.is_object() {
            let global_obj = global.as_object();
            let mut values = Vec::new();
            global_obj.for_each_property(|_atom, value, _attrs| {
                values.push(value);
            });
            for value in values {
                if value.is_function() {
                    let func =
                        unsafe { crate::value::JSValue::function_from_ptr_mut(value.get_ptr()) };
                    if func.base.prototype.is_none() {
                        func.base.set_prototype_raw(fn_proto_ptr);
                    }
                }
            }
        }
    }

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
            {
                let mut desc =
                    crate::object::object::PropertyDescriptor::new_data(JSValue::new_int(1));
                desc.writable = false;
                desc.enumerable = false;
                desc.configurable = true;
                func.base
                    .define_property_ext(ctx.common_atoms.length, desc, true, true, true);
            }
            {
                let mut desc = crate::object::object::PropertyDescriptor::new_data(
                    JSValue::new_string(ctx.intern("[Symbol.hasInstance]")),
                );
                desc.writable = false;
                desc.enumerable = false;
                desc.configurable = true;
                func.base
                    .define_property_ext(ctx.common_atoms.name, desc, true, true, true);
            }
            let ptr = Box::into_raw(Box::new(func)) as usize;
            ctx.runtime_mut().gc_heap_mut().track_function(ptr);
            JSValue::new_function(ptr)
        };
        if has_instance_sym.is_symbol() {
            let sym_key = crate::runtime::atom::Atom(0x40000000 | has_instance_sym.get_symbol_id());
            let mut desc = crate::object::object::PropertyDescriptor::new_data(hi_value);
            desc.writable = false;
            desc.enumerable = false;
            desc.configurable = false;
            fn_proto.define_property_ext(sym_key, desc, true, true, true);
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

fn global_isnan(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::bool(true);
    }
    let n = match js_to_number_value(ctx, &args[0]) {
        Ok(n) => n,
        Err(()) => return JSValue::undefined(),
    };
    JSValue::bool(n.is_nan())
}

fn global_isfinite(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::bool(false);
    }
    let n = match js_to_number_value(ctx, &args[0]) {
        Ok(n) => n,
        Err(()) => return JSValue::undefined(),
    };
    JSValue::bool(n.is_finite())
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

fn extract_array_like(ctx: &mut JSContext, arg_list: &JSValue) -> Result<Vec<JSValue>, ()> {
    if !arg_list.is_object() {
        return Err(());
    }
    let mut call_args = Vec::new();
    let obj = arg_list.as_object();
    let len_val = obj.get(ctx.common_atoms.length);
    if ctx.pending_exception.is_some() { return Err(()); }
    let len = match len_val {
        Some(v) if v.is_int() => v.get_int() as usize,
        Some(v) if v.is_float() => v.get_float() as usize,
        _ => 0,
    };
    if len > 1000000 { return Err(()); }
    for i in 0..len {
        let key = ctx.int_atom_mut(i);
        if let Some(v) = obj.get(key) {
            if ctx.pending_exception.is_some() { return Err(()); }
            call_args.push(v);
        } else {
            call_args.push(JSValue::undefined());
        }
    }
    Ok(call_args)
}

fn reflect_get(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object_like() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.get requires an object")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    if args.len() < 2 { return JSValue::undefined(); }
    let key_val = &args[1];
    let key = if key_val.is_string() {
        key_val.get_atom()
    } else if key_val.is_symbol() {
        crate::runtime::atom::Atom(0x40000000 | key_val.get_symbol_id())
    } else if key_val.is_int() {
        let i = key_val.get_int();
        if i >= 0 && i < 100 { ctx.int_atom_mut(i as usize) } else { ctx.intern(&i.to_string()) }
    } else {
        let s = to_property_key_string(ctx, key_val);
        if ctx.pending_exception.is_some() { return JSValue::undefined(); }
        ctx.intern(&s)
    };
    let receiver = args.get(2).copied().unwrap_or(args[0]);
    let target = args[0].as_object();
    let mut current: Option<&JSObject> = Some(target);
    let mut depth = 0u32;
    while let Some(obj) = current {
        depth += 1;
        if depth > 1000 { break; }
        if let Some(desc) = obj.get_own_descriptor(key) {
            if let Some(value) = desc.value {
                return value;
            }
            if let Some(getter) = desc.get {
                if getter.is_function() {
                    if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
                        let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
                        if let Ok(result) = vm.call_function_with_this(ctx, getter, receiver, &[]) {
                            return result;
                        }
                    }
                }
            }
            return JSValue::undefined();
        }
        if let Some(proto_ptr) = obj.prototype_ptr() {
            if proto_ptr.is_null() { break; }
            current = Some(unsafe { &*proto_ptr });
        } else {
            break;
        }
    }
    JSValue::undefined()
}

fn reflect_set(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 || !args[0].is_object_like() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.set requires an object")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let target = args[0];
    let key_val = &args[1];
    let value = args[2].clone();
    let key = if key_val.is_string() {
        key_val.get_atom()
    } else if key_val.is_symbol() {
        crate::runtime::atom::Atom(0x40000000 | key_val.get_symbol_id())
    } else if key_val.is_int() {
        let i = key_val.get_int();
        if i >= 0 && i < 100 { ctx.int_atom_mut(i as usize) } else { ctx.intern(&i.to_string()) }
    } else {
        let s = to_property_key_string(ctx, key_val);
        if ctx.pending_exception.is_some() { return JSValue::undefined(); }
        ctx.intern(&s)
    };
    target.as_object_mut().set(key, value);
    JSValue::bool(true)
}

fn reflect_has(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object_like() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.has requires an object")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    if args.len() < 2 { return JSValue::bool(false); }
    let key_val = &args[1];
    let key = if key_val.is_string() {
        key_val.get_atom()
    } else if key_val.is_symbol() {
        crate::runtime::atom::Atom(0x40000000 | key_val.get_symbol_id())
    } else if key_val.is_int() {
        let i = key_val.get_int();
        if i >= 0 && i < 100 { ctx.int_atom_mut(i as usize) } else { ctx.intern(&i.to_string()) }
    } else {
        let s = to_property_key_string(ctx, key_val);
        if ctx.pending_exception.is_some() { return JSValue::undefined(); }
        ctx.intern(&s)
    };
    JSValue::bool(args[0].as_object().has_property(key))
}

fn reflect_delete_property(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object_like() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.deleteProperty requires an object")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    if args.len() < 2 { return JSValue::bool(true); }
    let key_val = &args[1];
    let key = if key_val.is_string() {
        key_val.get_atom()
    } else if key_val.is_symbol() {
        crate::runtime::atom::Atom(0x40000000 | key_val.get_symbol_id())
    } else if key_val.is_int() {
        let i = key_val.get_int();
        if i >= 0 && i < 100 { ctx.int_atom_mut(i as usize) } else { ctx.intern(&i.to_string()) }
    } else {
        let s = to_property_key_string(ctx, key_val);
        if ctx.pending_exception.is_some() { return JSValue::undefined(); }
        ctx.intern(&s)
    };
    let result = args[0].as_object_mut().delete(key);
    JSValue::bool(result)
}

fn reflect_define_property(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 || !args[0].is_object_like() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.defineProperty requires an object")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let key_val = &args[1];
    let key = if key_val.is_string() {
        key_val.get_atom()
    } else if key_val.is_symbol() {
        crate::runtime::atom::Atom(0x40000000 | key_val.get_symbol_id())
    } else if key_val.is_int() {
        let i = key_val.get_int();
        if i >= 0 && i < 100 { ctx.int_atom_mut(i as usize) } else { ctx.intern(&i.to_string()) }
    } else {
        let s = to_property_key_string(ctx, key_val);
        if ctx.pending_exception.is_some() { return JSValue::undefined(); }
        ctx.intern(&s)
    };
    let attrs = &args[2];
    let mut desc = PropertyDescriptor::default();
    if attrs.is_object() {
        let obj = attrs.as_object();
        if let Some(v) = obj.get(ctx.intern("value")) { desc.value = Some(v); }
        if let Some(v) = obj.get(ctx.intern("writable")) { desc.writable = v.is_truthy(); }
        if let Some(v) = obj.get(ctx.intern("enumerable")) { desc.enumerable = v.is_truthy(); }
        if let Some(v) = obj.get(ctx.intern("configurable")) { desc.configurable = v.is_truthy(); }
        if let Some(v) = obj.get(ctx.intern("get")) { desc.get = Some(v); }
        if let Some(v) = obj.get(ctx.intern("set")) { desc.set = Some(v); }
    }
    if desc.get.is_some() || desc.set.is_some() {
        desc.value = None;
        desc.writable = false;
    }
    if let Some(v) = desc.value {
        if v.is_undefined() && desc.get.is_none() && desc.set.is_none() {
            desc.value = None;
        }
    }
    let result = args[0].as_object_mut().define_property(key, desc);
    JSValue::bool(result)
}

fn reflect_get_own_property_descriptor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object_like() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.getOwnPropertyDescriptor requires an object")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    if args.len() < 2 { return JSValue::undefined(); }
    let key_val = &args[1];
    let key = if key_val.is_string() {
        key_val.get_atom()
    } else if key_val.is_symbol() {
        crate::runtime::atom::Atom(0x40000000 | key_val.get_symbol_id())
    } else if key_val.is_int() {
        let i = key_val.get_int();
        if i >= 0 && i < 100 { ctx.int_atom_mut(i as usize) } else { ctx.intern(&i.to_string()) }
    } else {
        let s = to_property_key_string(ctx, key_val);
        if ctx.pending_exception.is_some() { return JSValue::undefined(); }
        ctx.intern(&s)
    };
    if let Some(desc) = args[0].as_object().get_own_descriptor(key) {
        let mut result = JSObject::new();
        if let Some(v) = desc.value {
            result.set(ctx.intern("value"), v);
        } else {
            result.set(ctx.intern("value"), JSValue::undefined());
        }
        result.set(ctx.intern("writable"), JSValue::bool(desc.writable));
        result.set(ctx.intern("enumerable"), JSValue::bool(desc.enumerable));
        result.set(ctx.intern("configurable"), JSValue::bool(desc.configurable));
        if let Some(g) = desc.get {
            result.set(ctx.intern("get"), g);
        } else {
            result.set(ctx.intern("get"), JSValue::undefined());
        }
        if let Some(s) = desc.set {
            result.set(ctx.intern("set"), s);
        } else {
            result.set(ctx.intern("set"), JSValue::undefined());
        }
        let ptr = Box::into_raw(Box::new(result)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        return JSValue::new_object(ptr);
    }
    JSValue::undefined()
}

fn reflect_get_prototype_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object_like() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.getPrototypeOf requires an object")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    if let Some(proto_ptr) = args[0].as_object().prototype_ptr() {
        JSValue::new_object(proto_ptr as usize)
    } else {
        JSValue::null()
    }
}

fn reflect_set_prototype_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object_like() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.setPrototypeOf requires an object")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let proto_val = if args.len() >= 2 { &args[1] } else { &JSValue::null() };
    if !proto_val.is_object_like() && !proto_val.is_null() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.setPrototypeOf requires an object or null")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    if proto_val.is_null() {
        if let Some(proto_ptr) = ctx.get_object_prototype() {
            if obj as *const JSObject as *mut JSObject == proto_ptr {
                return JSValue::bool(false);
            }
        }
        let obj_mut = args[0].as_object_mut();
        obj_mut.set_prototype_raw(std::ptr::null_mut());
        return JSValue::bool(true);
    }
    if proto_val.is_object_like() {
        // Object.prototype is immutable
        if let Some(proto_ptr) = ctx.get_object_prototype() {
            if obj as *const JSObject as *mut JSObject == proto_ptr {
                return JSValue::bool(false);
            }
        }
        // Check for cycles
        let new_proto = proto_val.get_ptr() as *mut JSObject;
        let mut current = new_proto;
        let mut depth = 0u32;
        while let Some(ptr) = unsafe { current.as_ref() }.and_then(|o| o.prototype) {
            if ptr == obj as *const JSObject as *mut JSObject || depth > 1000 {
                return JSValue::bool(false);
            }
            current = ptr;
            depth += 1;
        }
        let obj_mut = args[0].as_object_mut();
        obj_mut.set_prototype_raw(proto_val.get_ptr() as *mut JSObject);
    }
    JSValue::bool(true)
}

fn reflect_is_extensible(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object_like() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.isExtensible requires an object")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    JSValue::bool(args[0].as_object().extensible())
}

fn reflect_prevent_extensions(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object_like() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.preventExtensions requires an object")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    args[0].as_object_mut().set_extensible(false);
    JSValue::bool(true)
}

fn reflect_own_keys(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object_like() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.ownKeys requires an object")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    let order: Vec<Atom> = if obj.has_property_order() {
        obj.property_order_keys()
    } else {
        Vec::new()
    };
    let has_order = !order.is_empty();
    let mut string_keys: Vec<(Atom, usize)> = Vec::new();
    let mut symbol_keys: Vec<Atom> = Vec::new();
    if has_order {
        for (ord_idx, atom) in order.iter().enumerate() {
            if obj.find_offset(*atom).is_none() {
                continue;
            }
            let is_symbol = atom.0 & 0x40000000 != 0;
            if is_symbol {
                symbol_keys.push(*atom);
            } else {
                string_keys.push((*atom, ord_idx));
            }
        }
    } else {
        for (idx, slot) in obj.iter_props().enumerate() {
            if obj.find_offset(slot.atom).is_none() {
                continue;
            }
            let is_symbol = slot.atom.0 & 0x40000000 != 0;
            if is_symbol {
                symbol_keys.push(slot.atom);
            } else {
                string_keys.push((slot.atom, idx));
            }
        }
    }
    let acc_keys = obj.accessor_keys();
    for (atom, _ord) in &acc_keys {
        let is_symbol = atom.0 & 0x40000000 != 0;
        if is_symbol {
            if !symbol_keys.contains(atom) {
                symbol_keys.push(*atom);
            }
        } else {
            if !string_keys.iter().any(|(a, _)| a == atom) {
                string_keys.push((*atom, string_keys.len()));
            }
        }
    }
    string_keys.sort_by(|a, b| {
        let a_str = ctx.get_atom_str(a.0);
        let b_str = ctx.get_atom_str(b.0);
        let a_int: Option<i64> = a_str.parse().ok();
        let b_int: Option<i64> = b_str.parse().ok();
        match (a_int, b_int) {
            (Some(ai), Some(bi)) => ai.cmp(&bi),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.1.cmp(&b.1),
        }
    });
    let mut result = crate::object::object::JSObject::new_array();
    let mut idx = 0usize;
    for (atom, _) in &string_keys {
        if atom.0 & 0x40000000 == 0 {
            let s = ctx.get_atom_str(*atom).to_string();
            result.set(ctx.int_atom_mut(idx), JSValue::new_string(ctx.intern(&s)));
            idx += 1;
        }
    }
    for atom in &symbol_keys {
        let sym_id = atom.0 & 0x7FFFFFFF;
        let sym = JSValue::new_symbol_with_id(crate::builtins::symbol::NO_DESCRIPTION, sym_id);
        result.set(ctx.int_atom_mut(idx), sym);
        idx += 1;
    }
    result.set(ctx.common_atoms.length, JSValue::new_int(idx as i64));
    let ptr = Box::into_raw(Box::new(result)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}

fn init_reflect(ctx: &mut JSContext) {
    ctx.register_builtin("reflect_construct", HostFunction::new("construct", 2, reflect_construct));
    ctx.register_builtin("reflect_apply", HostFunction::new("apply", 3, reflect_apply));
    ctx.register_builtin("reflect_get", HostFunction::new("get", 2, reflect_get));
    ctx.register_builtin("reflect_set", HostFunction::new("set", 3, reflect_set));
    ctx.register_builtin("reflect_has", HostFunction::new("has", 2, reflect_has));
    ctx.register_builtin("reflect_delete_property", HostFunction::new("deleteProperty", 2, reflect_delete_property));
    ctx.register_builtin("reflect_define_property", HostFunction::new("defineProperty", 3, reflect_define_property));
    ctx.register_builtin("reflect_get_own_property_descriptor", HostFunction::new("getOwnPropertyDescriptor", 2, reflect_get_own_property_descriptor));
    ctx.register_builtin("reflect_get_prototype_of", HostFunction::new("getPrototypeOf", 1, reflect_get_prototype_of));
    ctx.register_builtin("reflect_set_prototype_of", HostFunction::new("setPrototypeOf", 2, reflect_set_prototype_of));
    ctx.register_builtin("reflect_is_extensible", HostFunction::new("isExtensible", 1, reflect_is_extensible));
    ctx.register_builtin("reflect_prevent_extensions", HostFunction::new("preventExtensions", 1, reflect_prevent_extensions));
    ctx.register_builtin("reflect_own_keys", HostFunction::new("ownKeys", 1, reflect_own_keys));

    let reflect_atom = ctx.intern("Reflect");
    let mut reflect_obj = JSObject::new();
    let methods = [
        ("construct", "reflect_construct"),
        ("apply", "reflect_apply"),
        ("get", "reflect_get"),
        ("set", "reflect_set"),
        ("has", "reflect_has"),
        ("deleteProperty", "reflect_delete_property"),
        ("defineProperty", "reflect_define_property"),
        ("getOwnPropertyDescriptor", "reflect_get_own_property_descriptor"),
        ("getPrototypeOf", "reflect_get_prototype_of"),
        ("setPrototypeOf", "reflect_set_prototype_of"),
        ("isExtensible", "reflect_is_extensible"),
        ("preventExtensions", "reflect_prevent_extensions"),
        ("ownKeys", "reflect_own_keys"),
    ];
    for (name, builtin_name) in &methods {
        let val = create_builtin_function(ctx, builtin_name);
        let key = ctx.intern(name);
        reflect_obj.define_property(
            key,
            PropertyDescriptor {
                value: Some(val),
                writable: true,
                enumerable: false,
                configurable: true,
                get: None,
                set: None,
            },
        );
    }
    let tag_key = symbol::get_symbol_to_string_tag_prop_key(ctx);
    reflect_obj.define_property_ext(
        tag_key,
        PropertyDescriptor {
            value: Some(JSValue::new_string(ctx.intern("Reflect"))),
            writable: false,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        },
        true, true, true,
    );
    let reflect_ptr = Box::into_raw(Box::new(reflect_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(reflect_ptr);
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        set_non_enumerable(global_obj, reflect_atom, JSValue::new_object(reflect_ptr));
    }
}

fn reflect_construct(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_function() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.construct requires a constructor")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let target = args[0];
    let new_target = if args.len() >= 3 { args[2] } else { target };
    let arg_list = args.get(1).copied().unwrap_or(JSValue::undefined());

    if new_target.is_function() {
        let f = new_target.as_function();
        if f.is_builtin() {
            let builtin_name = f.builtin_atom
                .map(|ba| ctx.get_atom_str(ba).to_string())
                .unwrap_or_default();
            if !ctx.builtin_is_constructor(&builtin_name) {
                let mut err = JSObject::new();
                err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
                err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("newTarget is not a constructor")));
                if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
                let ptr = Box::into_raw(Box::new(err)) as usize;
                ctx.runtime_mut().gc_heap_mut().track(ptr);
                ctx.pending_exception = Some(JSValue::new_object(ptr));
                return JSValue::undefined();
            }
        }
    }

    if arg_list.is_undefined() || arg_list.is_null() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.construct requires an arguments list")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }

    let call_args = match extract_array_like(ctx, &arg_list) {
        Ok(v) => v,
        Err(()) => return JSValue::undefined(),
    };

    if target.is_function() {
        let f = target.as_function();
        if f.is_builtin() {
            let builtin_name = f.builtin_atom
                .map(|ba| ctx.get_atom_str(ba).to_string())
                .unwrap_or_default();
            if !ctx.builtin_is_constructor(&builtin_name) {
                let mut err = JSObject::new();
                err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
                err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("target is not a constructor")));
                if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
                let ptr = Box::into_raw(Box::new(err)) as usize;
                ctx.runtime_mut().gc_heap_mut().track(ptr);
                ctx.pending_exception = Some(JSValue::new_object(ptr));
                return JSValue::undefined();
            }
        }
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
    let obj_ptr = Box::into_raw(Box::new(obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(obj_ptr);
    let new_obj = JSValue::new_object(obj_ptr);

    if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
        let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
        match vm.call_function_with_this(ctx, target, new_obj, &call_args) {
            Ok(result) => {
                if result.is_object_like() {
                    return result;
                }
                return new_obj;
            }
            Err(_e) => {
                return JSValue::undefined();
            }
        }
    }
    new_obj
}

fn reflect_apply(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 || !args[0].is_function() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.apply requires a function")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let target = args[0];
    let this_arg = args.get(1).copied().unwrap_or(JSValue::undefined());
    let arg_list = args.get(2).copied().unwrap_or(JSValue::undefined());
    if !arg_list.is_object() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Reflect.apply requires an array-like object")));
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let call_args = match extract_array_like(ctx, &arg_list) {
        Ok(v) => v,
        Err(()) => return JSValue::undefined(),
    };
    if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
        let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
        match vm.call_function_with_this(ctx, target, this_arg, &call_args) {
            Ok(result) => return result,
            Err(_e) => {
                return JSValue::undefined();
            }
        }
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
        set_non_enumerable(global_obj, console_atom, console_value);
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
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("abs"),
        create_builtin_function(ctx, "math_abs"),
    );
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
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("max"),
        create_builtin_function(ctx, "math_max"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("min"),
        create_builtin_function(ctx, "math_min"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("pow"),
        create_builtin_function(ctx, "math_pow"),
    );
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
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("log"),
        create_builtin_function(ctx, "math_log"),
    );
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
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("sin"),
        create_builtin_function(ctx, "math_sin"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("cos"),
        create_builtin_function(ctx, "math_cos"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("tan"),
        create_builtin_function(ctx, "math_tan"),
    );
    set_non_enumerable(
        &mut math_obj,
        ctx.intern("exp"),
        create_builtin_function(ctx, "math_exp"),
    );
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
    ctx.register_builtin("json_parse", HostFunction::new("parse", 2, json_parse));
    ctx.register_builtin(
        "json_stringify",
        HostFunction::new("stringify", 3, json_stringify),
    );
    ctx.register_builtin(
        "json_rawJSON",
        HostFunction::new("rawJSON", 1, json_raw_json),
    );
    ctx.register_builtin(
        "json_isRawJSON",
        HostFunction::new("isRawJSON", 1, json_is_raw_json),
    );

    let json_atom = ctx.intern("JSON");
    let tag_key = symbol::get_symbol_to_string_tag_prop_key(ctx);
    let mut json_obj = JSObject::new();
    let parse_fn = create_builtin_function(ctx, "json_parse");
    let stringify_fn = create_builtin_function(ctx, "json_stringify");
    let raw_json_fn = create_builtin_function(ctx, "json_rawJSON");
    let is_raw_json_fn = create_builtin_function(ctx, "json_isRawJSON");
    json_obj.set(ctx.intern("parse"), parse_fn);
    json_obj.set(ctx.intern("stringify"), stringify_fn);
    json_obj.set(ctx.intern("rawJSON"), raw_json_fn);
    json_obj.set(ctx.intern("isRawJSON"), is_raw_json_fn);
    let json_ptr = Box::into_raw(Box::new(json_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(json_ptr);
    let json_value = JSValue::new_object(json_ptr);
    let json_mut = json_value.as_object_mut();
    json_mut.define_property(
        tag_key,
        PropertyDescriptor {
            value: Some(JSValue::new_string(ctx.intern("JSON"))),
            writable: false,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        },
    );
    json_mut.define_property(
        ctx.intern("parse"),
        PropertyDescriptor {
            value: Some(parse_fn),
            writable: true,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        },
    );
    json_mut.define_property(
        ctx.intern("stringify"),
        PropertyDescriptor {
            value: Some(stringify_fn),
            writable: true,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        },
    );
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        set_non_enumerable(global_obj, json_atom, json_value);
    }
}

pub fn to_property_key_string(ctx: &mut JSContext, v: &JSValue) -> String {
    if v.is_string() {
        return ctx.get_atom_str(v.get_atom()).to_string();
    }
    if v.is_int() {
        return v.get_int().to_string();
    }
    if v.is_float() {
        return crate::builtins::string::js_float_to_string(v.get_float());
    }
    if v.is_undefined() {
        return "undefined".to_string();
    }
    if v.is_null() {
        return "null".to_string();
    }
    if v.is_bool() {
        return v.get_bool().to_string();
    }
    if v.is_symbol() {
        throw_type_error(ctx, "Cannot convert a Symbol value to a property key");
        return String::new();
    }
    if v.is_object() {
        if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
            let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
            let prim = vm.ordinary_to_primitive(v, "string", ctx);
            if ctx.pending_exception.is_some() {
                return String::new();
            }
            return to_property_key_string(ctx, &prim);
        }
    }
    String::new()
}

pub fn throw_type_error(ctx: &mut JSContext, msg: &str) -> JSValue {
    let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
    err.set(
        ctx.common_atoms.name,
        JSValue::new_string(ctx.intern("TypeError")),
    );
    err.set(
        ctx.common_atoms.message,
        JSValue::new_string(ctx.intern(msg)),
    );
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

        let to_prim_atom = {
            let global = ctx.global();
            if global.is_object() {
                global
                    .as_object()
                    .get(ctx.intern("Symbol.toPrimitive"))
                    .filter(|v| v.is_symbol())
                    .map(|v| {
                        crate::runtime::atom::Atom(0x40000000 | v.get_symbol_id())
                    })
            } else {
                None
            }
        };
        if let Some(tp_atom) = to_prim_atom {
            let mut cur: Option<*mut crate::object::object::JSObject> = Some(v.get_ptr() as *mut crate::object::object::JSObject);
            let mut depth = 0u32;
            while let Some(p) = cur {
                if depth > 100 {
                    break;
                }
                unsafe {
                    let tp_val = (*p).get_own_value(tp_atom);
                    if let Some(f) = tp_val {
                        if f.is_null_or_undefined() {
                            break;
                        }
                        if !f.is_function() {
                            let mut err = crate::object::object::JSObject::new_typed(
                                crate::object::object::ObjectType::Error,
                            );
                            err.set(
                                ctx.common_atoms.name,
                                JSValue::new_string(ctx.intern("TypeError")),
                            );
                            err.set(
                                ctx.common_atoms.message,
                                JSValue::new_string(ctx.intern(
                                    "Symbol.toPrimitive is not a function",
                                )),
                            );
                            if let Some(proto) = ctx.get_type_error_prototype() {
                                err.prototype = Some(proto);
                            }
                            let ptr = Box::into_raw(Box::new(err)) as usize;
                            ctx.runtime_mut().gc_heap_mut().track(ptr);
                            ctx.pending_exception = Some(JSValue::new_object(ptr));
                            return None;
                        }
                        let hint = JSValue::new_string(ctx.intern("number"));
                        let r = vm.call_function_with_this(ctx, f, v.clone(), &[hint]);
                        if ctx.pending_exception.is_some() {
                            return None;
                        }
                        if let Ok(rv) = r {
                            if rv.is_object() || rv.is_function() {
                                let mut err = crate::object::object::JSObject::new_typed(
                                    crate::object::object::ObjectType::Error,
                                );
                                err.set(
                                    ctx.common_atoms.name,
                                    JSValue::new_string(ctx.intern("TypeError")),
                                );
                                err.set(
                                    ctx.common_atoms.message,
                                    JSValue::new_string(ctx.intern(
                                        "Cannot convert object to primitive value",
                                    )),
                                );
                                if let Some(proto) = ctx.get_type_error_prototype() {
                                    err.prototype = Some(proto);
                                }
                                let ptr = Box::into_raw(Box::new(err)) as usize;
                                ctx.runtime_mut().gc_heap_mut().track(ptr);
                                ctx.pending_exception = Some(JSValue::new_object(ptr));
                                return None;
                            }
                            return Some(rv);
                        }
                        return None;
                    }
                    cur = (*p).prototype;
                }
                depth += 1;
            }
        }

        let value_of_atom = ctx.intern("valueOf");
        let to_string_atom = ctx.intern("toString");
        if let Some(valueof_fn) = obj.get(value_of_atom) {
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
        let mut err = crate::object::object::JSObject::new_typed(
            crate::object::object::ObjectType::Error,
        );
        err.set(
            ctx.common_atoms.name,
            JSValue::new_string(ctx.intern("TypeError")),
        );
        err.set(
            ctx.common_atoms.message,
            JSValue::new_string(ctx.intern("Cannot convert a Symbol value to a number")),
        );
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return Err(());
    }
    if v.is_bigint() {
        let mut err = crate::object::object::JSObject::new_typed(
            crate::object::object::ObjectType::Error,
        );
        err.set(
            ctx.common_atoms.name,
            JSValue::new_string(ctx.intern("TypeError")),
        );
        err.set(
            ctx.common_atoms.message,
            JSValue::new_string(ctx.intern("Cannot convert a BigInt value to a number")),
        );
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return Err(());
    }
    if v.is_string() {
        let s = ctx.get_atom_str(v.get_atom());
        return Ok(crate::runtime::vm::VM::string_to_number(s));
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
                    parts.push(if elem.get_bool() {
                        "true".to_string()
                    } else {
                        "false".to_string()
                    });
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
    if args[0].is_bigint() {
        let bv = args[0].as_object().get_bigint_value();
        let f = bv as f64;
        if f.is_nan() {
            return JSValue::new_float(f64::NAN);
        }
        if f == 0.0 && f.is_sign_negative() {
            return JSValue::new_float(-0.0);
        }
        if f == f.floor() && f.is_finite() && f.abs() < 140737488355328.0 {
            return JSValue::new_int(f as i64);
        }
        return JSValue::new_float(f);
    }
    match js_to_number_value(ctx, &args[0]) {
        Ok(f) => {
            if f.is_nan() {
                return JSValue::new_float(f64::NAN);
            }
            if f == 0.0 && f.is_sign_negative() {
                return JSValue::new_float(-0.0);
            }
            if f == f.floor() && f.is_finite() && f.abs() < 140737488355328.0 {
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
        HostFunction::new("parseInt", 2, global_parseint),
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
        HostFunction::method("toString", 1, number_to_string),
    );
    ctx.register_builtin(
        "number_toLocaleString",
        HostFunction::method("toLocaleString", 0, number_to_string),
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
    set_non_enumerable(
        &mut num_func.base,
        ctx.intern("isNaN"),
        create_builtin_function(ctx, "number_isNaN"),
    );
    set_non_enumerable(
        &mut num_func.base,
        ctx.intern("isFinite"),
        create_builtin_function(ctx, "number_isFinite"),
    );
    set_non_enumerable(
        &mut num_func.base,
        ctx.intern("isInteger"),
        create_builtin_function(ctx, "number_isInteger"),
    );
    set_non_enumerable(
        &mut num_func.base,
        ctx.intern("isSafeInteger"),
        create_builtin_function(ctx, "number_isSafeInteger"),
    );
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
    set_non_enumerable(&mut number_proto, ctx.common_atoms.constructor, num_value);

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
        (*num_func_ptr)
            .base
            .define_property(ctx.common_atoms.prototype, desc);
    }
}

fn number_value_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Number.prototype.valueOf requires 'this' to be a Number")));
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let this = &args[0];
    if this.is_int() || this.is_float() {
        return *this;
    }
    if this.is_object() {
        let val = this_number_value(ctx, this);
        if let Some(f) = val {
            if f == f.floor() && f.is_finite() && f.abs() < 140737488355328.0 {
                return JSValue::new_int(f as i64);
            }
            return JSValue::new_float(f);
        }
    }
    let mut err = JSObject::new();
    err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
    err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Number.prototype.valueOf requires 'this' to be a Number")));
    if let Some(proto) = ctx.get_type_error_prototype() {
        err.prototype = Some(proto);
    }
    let ptr = Box::into_raw(Box::new(err)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    ctx.pending_exception = Some(JSValue::new_object(ptr));
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
                    parts.push(if elem.get_bool() {
                        "true".to_string()
                    } else {
                        "false".to_string()
                    });
                } else {
                    parts.push("".to_string());
                }
            }
            let joined = parts.join(",");
            let f = joined.trim().parse::<f64>().unwrap_or(f64::NAN);
            if f.is_nan() {
                return Ok(0.0);
            }
            if f.is_infinite() {
                return Ok(f);
            }
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
    if f.is_nan() {
        return Ok(0.0);
    }
    if f.is_infinite() {
        return Ok(f);
    }
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
        let number_proto = ctx.get_number_prototype();
        let mut is_number_wrapper = false;
        if let Some(np) = number_proto {
            let this_ptr = this.get_ptr();
            if this_ptr as usize == np as usize {
                is_number_wrapper = true;
            }
            if !is_number_wrapper {
                let mut current = this.as_object().prototype;
                let mut depth = 0u32;
                while let Some(p) = current {
                    if p.is_null() || depth > 10 {
                        break;
                    }
                    if p as usize == np as usize {
                        is_number_wrapper = true;
                        break;
                    }
                    unsafe {
                        current = (*p).prototype;
                    }
                    depth += 1;
                }
            }
        }
        if is_number_wrapper {
            if let Some(v) = obj.get(ctx.common_atoms.__value__) {
                if v.is_int() {
                    return Some(v.get_int() as f64);
                }
                if v.is_float() {
                    return Some(v.get_float());
                }
            }
        }
    }
    None
}

fn throw_range_error(ctx: &mut JSContext, msg: &str) -> JSValue {
    let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
    err.set(
        ctx.common_atoms.name,
        JSValue::new_string(ctx.intern("RangeError")),
    );
    err.set(
        ctx.common_atoms.message,
        JSValue::new_string(ctx.intern(msg)),
    );
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
        let parts: Vec<&str> = high_prec.splitn(2, 'e').collect();
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
        let parts: Vec<&str> = rust_fmt.splitn(2, 'e').collect();
        let m = parts[0].trim_end_matches('0');
        let m = if m.ends_with('.') {
            &m[..m.len() - 1]
        } else {
            m
        };
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
            Err(()) => {
                return throw_type_error_if_no_exception(
                    ctx,
                    "toFixed() argument cannot be converted to a number",
                );
            }
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
    if val.abs() >= 1e21 {
        let s = format!("{}", val);
        return JSValue::new_string(ctx.intern(&s));
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
            Err(()) => {
                return throw_type_error_if_no_exception(
                    ctx,
                    "toExponential() argument cannot be converted to a number",
                );
            }
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
                return throw_range_error(
                    ctx,
                    "toExponential() fractionDigits must be between 0 and 100",
                );
            }
            Some(d as usize)
        }
        None => None,
    };
    if let Some(d) = fraction_digits {
        if d > 100 {
            return throw_range_error(
                ctx,
                "toExponential() fractionDigits must be between 0 and 100",
            );
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
        Err(()) => {
            return throw_type_error_if_no_exception(
                ctx,
                "toPrecision() argument cannot be converted to a number",
            );
        }
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
    let _exp = e + 1;
    if e >= -6 && e < p as i32 {
        let exp_str = format_exponential(abs, Some(p - 1));
        let parts: Vec<&str> = exp_str.splitn(2, 'e').collect();
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
    let val = if !args.is_empty() {
        args[0].is_truthy()
    } else {
        false
    };
    JSValue::bool(val)
}

fn boolean_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        let mut err = JSObject::new();
        err.set(
            ctx.common_atoms.name,
            JSValue::new_string(ctx.intern("TypeError")),
        );
        err.set(
            ctx.common_atoms.message,
            JSValue::new_string(
                ctx.intern("Boolean.prototype.toString requires 'this' to be a Boolean"),
            ),
        );
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
    let mut err = JSObject::new();
    err.set(
        ctx.common_atoms.name,
        JSValue::new_string(ctx.intern("TypeError")),
    );
    err.set(
        ctx.common_atoms.message,
        JSValue::new_string(
            ctx.intern("Boolean.prototype.toString requires 'this' to be a Boolean"),
        ),
    );
    if let Some(proto) = ctx.get_type_error_prototype() {
        err.prototype = Some(proto);
    }
    let ptr = Box::into_raw(Box::new(err)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    ctx.pending_exception = Some(JSValue::new_object(ptr));
    JSValue::undefined()
}

fn boolean_value_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        let mut err = JSObject::new();
        err.set(
            ctx.common_atoms.name,
            JSValue::new_string(ctx.intern("TypeError")),
        );
        err.set(
            ctx.common_atoms.message,
            JSValue::new_string(
                ctx.intern("Boolean.prototype.valueOf requires 'this' to be a Boolean"),
            ),
        );
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
    err.set(
        ctx.common_atoms.name,
        JSValue::new_string(ctx.intern("TypeError")),
    );
    err.set(
        ctx.common_atoms.message,
        JSValue::new_string(
            ctx.intern("Boolean.prototype.valueOf requires 'this' to be a Boolean"),
        ),
    );
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
    ctx.set_bool_prototype(proto_ptr);

    let bool_func_ptr = bool_ptr as *mut crate::object::function::JSFunction;
    unsafe {
        (*bool_func_ptr).base.define_property(
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
        (*bool_func_ptr).base.define_property(
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
    ctx.register_builtin(
        "date_getUTCFullYear",
        HostFunction::method("getUTCFullYear", 0, date::date_get_utc_full_year),
    );
    ctx.register_builtin(
        "date_getUTCMonth",
        HostFunction::method("getUTCMonth", 0, date::date_get_utc_month),
    );
    ctx.register_builtin(
        "date_getUTCDate",
        HostFunction::method("getUTCDate", 0, date::date_get_utc_date),
    );
    ctx.register_builtin(
        "date_getUTCDay",
        HostFunction::method("getUTCDay", 0, date::date_get_utc_day),
    );
    ctx.register_builtin(
        "date_getUTCHours",
        HostFunction::method("getUTCHours", 0, date::date_get_utc_hours),
    );
    ctx.register_builtin(
        "date_getUTCMinutes",
        HostFunction::method("getUTCMinutes", 0, date::date_get_utc_minutes),
    );
    ctx.register_builtin(
        "date_getUTCSeconds",
        HostFunction::method("getUTCSeconds", 0, date::date_get_utc_seconds),
    );
    ctx.register_builtin(
        "date_getUTCMilliseconds",
        HostFunction::method("getUTCMilliseconds", 0, date::date_get_utc_milliseconds),
    );
    ctx.register_builtin(
        "date_setTime",
        HostFunction::method("setTime", 1, date::date_set_time),
    );
    ctx.register_builtin(
        "date_setMilliseconds",
        HostFunction::method("setMilliseconds", 1, date::date_set_milliseconds),
    );
    ctx.register_builtin(
        "date_setSeconds",
        HostFunction::method("setSeconds", 2, date::date_set_seconds),
    );
    ctx.register_builtin(
        "date_setMinutes",
        HostFunction::method("setMinutes", 3, date::date_set_minutes),
    );
    ctx.register_builtin(
        "date_setHours",
        HostFunction::method("setHours", 4, date::date_set_hours),
    );
    ctx.register_builtin(
        "date_setDate",
        HostFunction::method("setDate", 1, date::date_set_date),
    );
    ctx.register_builtin(
        "date_setMonth",
        HostFunction::method("setMonth", 2, date::date_set_month),
    );
    ctx.register_builtin(
        "date_setFullYear",
        HostFunction::method("setFullYear", 3, date::date_set_full_year),
    );
    ctx.register_builtin(
        "date_setUTCMilliseconds",
        HostFunction::method("setUTCMilliseconds", 1, date::date_set_utc_milliseconds),
    );
    ctx.register_builtin(
        "date_setUTCSeconds",
        HostFunction::method("setUTCSeconds", 2, date::date_set_utc_seconds),
    );
    ctx.register_builtin(
        "date_setUTCMinutes",
        HostFunction::method("setUTCMinutes", 3, date::date_set_utc_minutes),
    );
    ctx.register_builtin(
        "date_setUTCHours",
        HostFunction::method("setUTCHours", 4, date::date_set_utc_hours),
    );
    ctx.register_builtin(
        "date_setUTCDate",
        HostFunction::method("setUTCDate", 1, date::date_set_utc_date),
    );
    ctx.register_builtin(
        "date_setUTCMonth",
        HostFunction::method("setUTCMonth", 2, date::date_set_utc_month),
    );
    ctx.register_builtin(
        "date_setUTCFullYear",
        HostFunction::method("setUTCFullYear", 3, date::date_set_utc_full_year),
    );
    ctx.register_builtin(
        "date_toJSON",
        HostFunction::method("toJSON", 1, date::date_to_json),
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
    promise::register_builtins(ctx);
    promise::init_promise(ctx);
}

fn init_global_funcs(ctx: &mut JSContext) {
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        set_non_enumerable(
            global_obj,
            ctx.intern("isNaN"),
            create_builtin_function(ctx, "global_isnan"),
        );
        set_non_enumerable(
            global_obj,
            ctx.intern("isFinite"),
            create_builtin_function(ctx, "global_isfinite"),
        );
        set_non_enumerable(
            global_obj,
            ctx.intern("parseInt"),
            create_builtin_function(ctx, "global_parseint"),
        );
        set_non_enumerable(
            global_obj,
            ctx.intern("parseFloat"),
            create_builtin_function(ctx, "global_parsefloat"),
        );
        set_non_enumerable(
            global_obj,
            ctx.intern("eval"),
            create_builtin_function(ctx, "global_eval"),
        );

        set_non_enumerable(
            global_obj,
            ctx.intern("btoa"),
            create_builtin_function(ctx, "btoa"),
        );
        set_non_enumerable(
            global_obj,
            ctx.intern("atob"),
            create_builtin_function(ctx, "atob"),
        );

        set_non_enumerable(
            global_obj,
            ctx.intern("encodeURI"),
            create_builtin_function(ctx, "encodeURI"),
        );
        set_non_enumerable(
            global_obj,
            ctx.intern("decodeURI"),
            create_builtin_function(ctx, "decodeURI"),
        );
        set_non_enumerable(
            global_obj,
            ctx.intern("encodeURIComponent"),
            create_builtin_function(ctx, "encodeURIComponent"),
        );
        set_non_enumerable(
            global_obj,
            ctx.intern("decodeURIComponent"),
            create_builtin_function(ctx, "decodeURIComponent"),
        );

        set_non_enumerable(
            global_obj,
            ctx.intern("setTimeout"),
            create_builtin_function(ctx, "setTimeout"),
        );
        set_non_enumerable(
            global_obj,
            ctx.intern("setInterval"),
            create_builtin_function(ctx, "setInterval"),
        );
        set_non_enumerable(
            global_obj,
            ctx.intern("clearTimeout"),
            create_builtin_function(ctx, "clearTimeout"),
        );
        set_non_enumerable(
            global_obj,
            ctx.intern("clearInterval"),
            create_builtin_function(ctx, "clearInterval"),
        );

        set_non_enumerable(
            global_obj,
            ctx.intern("queueMicrotask"),
            create_builtin_function(ctx, "queueMicrotask"),
        );

        set_non_enumerable(
            global_obj,
            ctx.intern("requestAnimationFrame"),
            create_builtin_function(ctx, "requestAnimationFrame"),
        );
        set_non_enumerable(
            global_obj,
            ctx.intern("cancelAnimationFrame"),
            create_builtin_function(ctx, "cancelAnimationFrame"),
        );

        set_non_enumerable(global_obj, ctx.intern("globalThis"), global);

        set_non_enumerable(
            global_obj,
            ctx.intern("import"),
            create_builtin_function(ctx, "import"),
        );
        set_non_enumerable(
            global_obj,
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
            set_non_enumerable(
                global_obj,
                ctx.intern("fetch"),
                create_builtin_function(ctx, "fetch"),
            );
            set_non_enumerable(
                global_obj,
                ctx.intern("WebSocket"),
                create_builtin_function(ctx, "WebSocket"),
            );
            set_non_enumerable(
                global_obj,
                ctx.intern("EventSource"),
                create_builtin_function(ctx, "EventSource"),
            );
        }
    }
}
