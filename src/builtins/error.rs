use crate::host::HostFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

fn set_ctor_prototype(obj: &mut JSObject, key: crate::runtime::atom::Atom, val: JSValue) {
    obj.define_property(
        key,
        crate::object::object::PropertyDescriptor {
            value: Some(val),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );
}

fn set_own_ne(obj: &mut JSObject, key: crate::runtime::atom::Atom, val: JSValue) {
    obj.define_property(
        key,
        crate::object::object::PropertyDescriptor {
            value: Some(val),
            writable: true,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        },
    );
}

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let arity = ctx.get_builtin_arity(name).unwrap_or(1);
    let mut func = crate::object::function::JSFunction::new_builtin(ctx.intern(name), arity);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

pub fn init_error(ctx: &mut JSContext) {
    fn set_ne(
        obj: &mut crate::object::object::JSObject,
        key: crate::runtime::atom::Atom,
        val: crate::value::JSValue,
    ) {
        obj.define_property(
            key,
            crate::object::object::PropertyDescriptor {
                value: Some(val),
                writable: true,
                enumerable: false,
                configurable: true,
                get: None,
                set: None,
            },
        );
    }

    let error_atom = ctx.intern("Error");

    let mut error_func = crate::object::function::JSFunction::new_builtin(error_atom, 1);
    error_func.set_builtin_marker(ctx, "error_constructor");
    if let Some(func_proto) = ctx.get_function_prototype() {
        error_func.base.prototype = Some(func_proto);
    }
    let error_ptr = Box::into_raw(Box::new(error_func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(error_ptr);
    let error_value = JSValue::new_function(error_ptr);

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, error_atom, error_value);
    }

    let mut proto_obj = JSObject::new();
    set_ne(
        &mut proto_obj,
        ctx.intern("name"),
        JSValue::new_string(ctx.intern("Error")),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("message"),
        JSValue::new_string(ctx.intern("")),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("toString"),
        create_builtin_function(ctx, "error_toString"),
    );

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto_obj.prototype = Some(obj_proto_ptr);
    }

    set_ne(&mut proto_obj, ctx.intern("constructor"), error_value);

    let proto_ptr = Box::into_raw(Box::new(proto_obj)) as usize;
    ctx.set_error_prototype(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);
    if ctx.get_builtin_func("error_constructor").is_some() {
        let f = unsafe { JSValue::function_from_ptr_mut(error_ptr) };
        set_ctor_prototype(&mut f.base, ctx.intern("prototype"), proto_value);
    }

    let proto_atom = ctx.intern("ErrorPrototype");
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, proto_atom, proto_value);
    }

    init_type_error(ctx);
    init_reference_error(ctx);
    init_syntax_error(ctx);
    init_range_error(ctx);
    init_uri_error(ctx);
    init_eval_error(ctx);
}

fn init_type_error(ctx: &mut JSContext) {
    fn set_ne(
        obj: &mut crate::object::object::JSObject,
        key: crate::runtime::atom::Atom,
        val: crate::value::JSValue,
    ) {
        obj.define_property(
            key,
            crate::object::object::PropertyDescriptor {
                value: Some(val),
                writable: true,
                enumerable: false,
                configurable: true,
                get: None,
                set: None,
            },
        );
    }

    let atom = ctx.intern("TypeError");
    let mut ctor = crate::object::function::JSFunction::new_builtin(atom, 1);
    ctor.set_builtin_marker(ctx, "type_error_constructor");
    if let Some(func_proto) = ctx.get_function_prototype() {
        ctor.base.prototype = Some(func_proto);
    }
    let ptr = Box::into_raw(Box::new(ctor)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    let value = JSValue::new_function(ptr);

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, atom, value);
    }

    let mut proto = JSObject::new();
    set_ne(
        &mut proto,
        ctx.intern("name"),
        JSValue::new_string(ctx.intern("TypeError")),
    );
    set_ne(
        &mut proto,
        ctx.intern("message"),
        JSValue::new_string(ctx.intern("")),
    );

    if let Some(error_proto_ptr) = ctx.get_error_prototype() {
        proto.prototype = Some(error_proto_ptr);
    }
    set_ne(&mut proto, ctx.intern("constructor"), value);

    let proto_ptr = Box::into_raw(Box::new(proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);
    ctx.set_type_error_prototype(proto_ptr);
    let ctor = unsafe { JSValue::function_from_ptr_mut(ptr) };
    set_ctor_prototype(&mut ctor.base, ctx.intern("prototype"), proto_value);
}

fn init_reference_error(ctx: &mut JSContext) {
    fn set_ne(
        obj: &mut crate::object::object::JSObject,
        key: crate::runtime::atom::Atom,
        val: crate::value::JSValue,
    ) {
        obj.define_property(
            key,
            crate::object::object::PropertyDescriptor {
                value: Some(val),
                writable: true,
                enumerable: false,
                configurable: true,
                get: None,
                set: None,
            },
        );
    }

    let atom = ctx.intern("ReferenceError");
    let mut ctor = crate::object::function::JSFunction::new_builtin(atom, 1);
    ctor.set_builtin_marker(ctx, "reference_error_constructor");
    if let Some(func_proto) = ctx.get_function_prototype() {
        ctor.base.prototype = Some(func_proto);
    }
    let ptr = Box::into_raw(Box::new(ctor)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    let value = JSValue::new_function(ptr);

    let mut proto = JSObject::new();
    set_ne(
        &mut proto,
        ctx.intern("name"),
        JSValue::new_string(ctx.intern("ReferenceError")),
    );
    set_ne(
        &mut proto,
        ctx.intern("message"),
        JSValue::new_string(ctx.intern("")),
    );
    if let Some(error_proto_ptr) = ctx.get_error_prototype() {
        proto.prototype = Some(error_proto_ptr);
    }
    set_ne(&mut proto, ctx.intern("constructor"), value);

    let proto_ptr = Box::into_raw(Box::new(proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);
    ctx.set_reference_error_prototype(proto_ptr);
    let ctor = unsafe { JSValue::function_from_ptr_mut(ptr) };
    set_ctor_prototype(&mut ctor.base, ctx.intern("prototype"), proto_value);

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, atom, value);
    }
}

fn init_syntax_error(ctx: &mut JSContext) {
    fn set_ne(
        obj: &mut crate::object::object::JSObject,
        key: crate::runtime::atom::Atom,
        val: crate::value::JSValue,
    ) {
        obj.define_property(
            key,
            crate::object::object::PropertyDescriptor {
                value: Some(val),
                writable: true,
                enumerable: false,
                configurable: true,
                get: None,
                set: None,
            },
        );
    }

    let atom = ctx.intern("SyntaxError");
    let mut ctor = crate::object::function::JSFunction::new_builtin(atom, 1);
    ctor.set_builtin_marker(ctx, "syntax_error_constructor");
    if let Some(func_proto) = ctx.get_function_prototype() {
        ctor.base.prototype = Some(func_proto);
    }
    let ptr = Box::into_raw(Box::new(ctor)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    let value = JSValue::new_function(ptr);

    let mut proto = JSObject::new();
    set_ne(
        &mut proto,
        ctx.intern("name"),
        JSValue::new_string(ctx.intern("SyntaxError")),
    );
    set_ne(
        &mut proto,
        ctx.intern("message"),
        JSValue::new_string(ctx.intern("")),
    );
    if let Some(error_proto_ptr) = ctx.get_error_prototype() {
        proto.prototype = Some(error_proto_ptr);
    }
    set_ne(&mut proto, ctx.intern("constructor"), value);

    let proto_ptr = Box::into_raw(Box::new(proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);
    ctx.set_syntax_error_prototype(proto_ptr);
    let ctor = unsafe { JSValue::function_from_ptr_mut(ptr) };
    set_ctor_prototype(&mut ctor.base, ctx.intern("prototype"), proto_value);

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, atom, value);
    }
}

fn init_range_error(ctx: &mut JSContext) {
    fn set_ne(
        obj: &mut crate::object::object::JSObject,
        key: crate::runtime::atom::Atom,
        val: crate::value::JSValue,
    ) {
        obj.define_property(
            key,
            crate::object::object::PropertyDescriptor {
                value: Some(val),
                writable: true,
                enumerable: false,
                configurable: true,
                get: None,
                set: None,
            },
        );
    }

    let atom = ctx.intern("RangeError");
    let mut ctor = crate::object::function::JSFunction::new_builtin(atom, 1);
    ctor.set_builtin_marker(ctx, "range_error_constructor");
    if let Some(func_proto) = ctx.get_function_prototype() {
        ctor.base.prototype = Some(func_proto);
    }
    let ptr = Box::into_raw(Box::new(ctor)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    let value = JSValue::new_function(ptr);

    let mut proto = JSObject::new();
    set_ne(
        &mut proto,
        ctx.intern("name"),
        JSValue::new_string(ctx.intern("RangeError")),
    );
    set_ne(
        &mut proto,
        ctx.intern("message"),
        JSValue::new_string(ctx.intern("")),
    );
    if let Some(error_proto_ptr) = ctx.get_error_prototype() {
        proto.prototype = Some(error_proto_ptr);
    }
    set_ne(&mut proto, ctx.intern("constructor"), value);

    let proto_ptr = Box::into_raw(Box::new(proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);
    ctx.set_range_error_prototype(proto_ptr);
    let ctor = unsafe { JSValue::function_from_ptr_mut(ptr) };
    set_ctor_prototype(&mut ctor.base, ctx.intern("prototype"), proto_value);

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, atom, value);
    }
}

fn init_uri_error(ctx: &mut JSContext) {
    fn set_ne(
        obj: &mut crate::object::object::JSObject,
        key: crate::runtime::atom::Atom,
        val: crate::value::JSValue,
    ) {
        obj.define_property(
            key,
            crate::object::object::PropertyDescriptor {
                value: Some(val),
                writable: true,
                enumerable: false,
                configurable: true,
                get: None,
                set: None,
            },
        );
    }

    let atom = ctx.intern("URIError");
    let mut ctor = crate::object::function::JSFunction::new_builtin(atom, 1);
    ctor.set_builtin_marker(ctx, "uri_error_constructor");
    if let Some(func_proto) = ctx.get_function_prototype() {
        ctor.base.prototype = Some(func_proto);
    }
    let ptr = Box::into_raw(Box::new(ctor)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    let value = JSValue::new_function(ptr);

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, atom, value);
    }

    let mut proto = JSObject::new();
    set_ne(
        &mut proto,
        ctx.intern("name"),
        JSValue::new_string(ctx.intern("URIError")),
    );
    set_ne(
        &mut proto,
        ctx.intern("message"),
        JSValue::new_string(ctx.intern("")),
    );

    if let Some(error_proto_ptr) = ctx.get_error_prototype() {
        proto.prototype = Some(error_proto_ptr);
    }
    set_ne(&mut proto, ctx.intern("constructor"), value);

    let proto_ptr = Box::into_raw(Box::new(proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);
    ctx.set_uri_error_prototype(proto_ptr);
    let ctor = unsafe { JSValue::function_from_ptr_mut(ptr) };
    set_ctor_prototype(&mut ctor.base, ctx.intern("prototype"), proto_value);
}

fn init_eval_error(ctx: &mut JSContext) {
    fn set_ne(
        obj: &mut crate::object::object::JSObject,
        key: crate::runtime::atom::Atom,
        val: crate::value::JSValue,
    ) {
        obj.define_property(
            key,
            crate::object::object::PropertyDescriptor {
                value: Some(val),
                writable: true,
                enumerable: false,
                configurable: true,
                get: None,
                set: None,
            },
        );
    }

    let atom = ctx.intern("EvalError");
    let mut ctor = crate::object::function::JSFunction::new_builtin(atom, 1);
    ctor.set_builtin_marker(ctx, "eval_error_constructor");
    if let Some(func_proto) = ctx.get_function_prototype() {
        ctor.base.prototype = Some(func_proto);
    }
    let ptr = Box::into_raw(Box::new(ctor)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    let value = JSValue::new_function(ptr);

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, atom, value);
    }

    let mut proto = JSObject::new();
    set_ne(
        &mut proto,
        ctx.intern("name"),
        JSValue::new_string(ctx.intern("EvalError")),
    );
    set_ne(
        &mut proto,
        ctx.intern("message"),
        JSValue::new_string(ctx.intern("")),
    );

    if let Some(error_proto_ptr) = ctx.get_error_prototype() {
        proto.prototype = Some(error_proto_ptr);
    }
    set_ne(&mut proto, ctx.intern("constructor"), value);

    let proto_ptr = Box::into_raw(Box::new(proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);
    ctx.set_eval_error_prototype(proto_ptr);
    let ctor = unsafe { JSValue::function_from_ptr_mut(ptr) };
    set_ctor_prototype(&mut ctor.base, ctx.intern("prototype"), proto_value);
}

fn get_property(
    ctx: &mut JSContext,
    obj: &crate::object::object::JSObject,
    prop: crate::runtime::atom::Atom,
    this: &JSValue,
) -> Option<JSValue> {
    if let Some(val) = obj.get_own_accessor_value(prop) {
        if val.is_function() {
            if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
                let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
                let saved_exception = ctx.pending_exception.take();
                let result = vm.call_function_with_this(ctx, val, this.clone(), &[]);
                if result.is_err() {
                    let exc = ctx
                        .pending_exception
                        .take()
                        .or_else(|| vm.last_caught_exception.take())
                        .or(saved_exception);
                    if let Some(e) = exc {
                        ctx.pending_exception = Some(e);
                    }
                    return None;
                }
                ctx.pending_exception = saved_exception;
                return result.ok();
            }
        }
        return Some(val);
    }
    obj.get(prop)
}

fn js_to_string_value(ctx: &mut JSContext, val: &JSValue) -> Result<String, ()> {
    if val.is_string() {
        return Ok(ctx.get_atom_str(val.get_atom()).to_string());
    }
    if val.is_undefined() {
        return Ok("undefined".to_string());
    }
    if val.is_null() {
        return Ok("null".to_string());
    }
    if val.is_bool() {
        if val.is_truthy() {
            return Ok("true".to_string());
        }
        return Ok("false".to_string());
    }
    if val.is_int() {
        let n = val.get_int();
        return Ok(format!("{}", n));
    }
    if val.is_float() {
        let n = val.to_number();
        if n == n.floor() && n.is_finite() && n.abs() < 1e15 {
            return Ok(format!("{}", n as i64));
        }
        return Ok(format!("{}", n));
    }
    if val.is_symbol() {
        let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        err.set(
            ctx.common_atoms.message,
            JSValue::new_string(ctx.intern("Cannot convert a Symbol value to a string")),
        );
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return Err(());
    }
    if val.is_object() {
        let obj = val.as_object();
        if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
            let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
            let to_string_atom = ctx.intern("toString");
            let value_of_atom = ctx.intern("valueOf");
            let hint_string = ctx.intern("string");
            if let Some(to_prim) = obj.get(ctx.intern("__toPrimitive__hook__")) {
                let _ = obj;
                if to_prim.is_function() {
                    let result = vm.call_function_with_this(
                        ctx,
                        to_prim,
                        val.clone(),
                        &[JSValue::new_string(hint_string)],
                    );
                    if ctx.pending_exception.is_some() {
                        return Err(());
                    }
                    if let Ok(prim) = result {
                        if !prim.is_object() {
                            return js_to_string_value(ctx, &prim);
                        }
                    }
                }
            }
            let to_string_fn = obj.get(to_string_atom);
            let _ = obj;
            if let Some(fn_val) = to_string_fn {
                if fn_val.is_function() {
                    let result = vm.call_function_with_this(ctx, fn_val, val.clone(), &[]);
                    if ctx.pending_exception.is_some() {
                        return Err(());
                    }
                    if let Ok(r) = result {
                        if !r.is_object() {
                            return js_to_string_value(ctx, &r);
                        }
                    }
                }
            }
            if ctx.pending_exception.is_some() {
                return Err(());
            }
            let obj2 = val.as_object();
            let value_of_fn = obj2.get(value_of_atom);
            let _ = obj2;
            if let Some(fn_val) = value_of_fn {
                if fn_val.is_function() {
                    let result = vm.call_function_with_this(ctx, fn_val, val.clone(), &[]);
                    if ctx.pending_exception.is_some() {
                        return Err(());
                    }
                    if let Ok(r) = result {
                        if !r.is_object() {
                            return js_to_string_value(ctx, &r);
                        }
                    }
                }
            }
            if ctx.pending_exception.is_some() {
                return Err(());
            }
        }
        let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        err.set(
            ctx.common_atoms.message,
            JSValue::new_string(ctx.intern("Cannot convert object to primitive value")),
        );
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return Err(());
    }
    Ok(String::new())
}

fn build_error(
    ctx: &mut JSContext,
    name: &str,
    args: &[JSValue],
    proto: Option<*mut JSObject>,
) -> JSValue {
    let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
    set_own_ne(
        &mut err,
        ctx.intern("name"),
        JSValue::new_string(ctx.intern(name)),
    );
    if !args.is_empty() && !args[0].is_undefined() {
        match js_to_string_value(ctx, &args[0]) {
            Ok(s) => {
                set_own_ne(
                    &mut err,
                    ctx.intern("message"),
                    JSValue::new_string(ctx.intern(&s)),
                );
            }
            Err(()) => return JSValue::undefined(),
        }
    }
    if args.len() > 1 && args[1].is_object() {
        let opts = args[1].as_object();
        if let Some(cause) = opts.get(ctx.intern("cause")) {
            set_own_ne(&mut err, ctx.intern("cause"), cause);
        }
    }
    if let Some(p) = proto {
        err.prototype = Some(p);
    }
    let ptr = Box::into_raw(Box::new(err)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}

pub fn error_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let proto = ctx.get_error_prototype().map(|p| p as *mut _);
    build_error(ctx, "Error", args, proto)
}

pub fn type_error_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let proto = ctx.get_type_error_prototype().map(|p| p as *mut _);
    build_error(ctx, "TypeError", args, proto)
}

pub fn reference_error_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let proto = ctx
        .get_reference_error_prototype()
        .or_else(|| ctx.get_error_prototype())
        .map(|p| p as *mut _);
    build_error(ctx, "ReferenceError", args, proto)
}

pub fn syntax_error_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let proto = ctx.get_syntax_error_prototype().map(|p| p as *mut _);
    build_error(ctx, "SyntaxError", args, proto)
}

pub fn range_error_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let proto = ctx
        .get_range_error_prototype()
        .or_else(|| ctx.get_error_prototype())
        .map(|p| p as *mut _);
    build_error(ctx, "RangeError", args, proto)
}

pub fn uri_error_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let proto = ctx
        .get_uri_error_prototype()
        .or_else(|| ctx.get_error_prototype())
        .map(|p| p as *mut _);
    build_error(ctx, "URIError", args, proto)
}

pub fn eval_error_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let proto = ctx
        .get_eval_error_prototype()
        .or_else(|| ctx.get_error_prototype())
        .map(|p| p as *mut _);
    build_error(ctx, "EvalError", args, proto)
}

fn error_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        let mut err =
            crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        err.set(
            ctx.common_atoms.message,
            JSValue::new_string(ctx.intern("Error.prototype.toString called on undefined")),
        );
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let this = &args[0];
    if !this.is_object() {
        let mut err =
            crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        err.set(
            ctx.common_atoms.message,
            JSValue::new_string(ctx.intern("Error.prototype.toString called on non-object")),
        );
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }

    let obj = this.as_object();
    let name_atom = ctx.intern("name");
    let name_val = get_property(ctx, &obj, name_atom, this);
    let _ = obj;
    if ctx.pending_exception.is_some() {
        return JSValue::undefined();
    }
    let name: Result<String, ()> = match name_val {
        Some(n) => {
            if n.is_undefined() {
                Ok("Error".to_string())
            } else {
                match js_to_string_value(ctx, &n) {
                    Ok(s) => Ok(s),
                    Err(()) => return JSValue::undefined(),
                }
            }
        }
        None => Ok("Error".to_string()),
    };
    if ctx.pending_exception.is_some() {
        return JSValue::undefined();
    }

    let obj2 = this.as_object();
    let msg_atom = ctx.intern("message");
    let message_val = get_property(ctx, &obj2, msg_atom, this);
    let _ = obj2;
    if ctx.pending_exception.is_some() {
        return JSValue::undefined();
    }
    let message: Result<String, ()> = match message_val {
        Some(m) => {
            if m.is_undefined() {
                Ok(String::new())
            } else {
                match js_to_string_value(ctx, &m) {
                    Ok(s) => Ok(s),
                    Err(()) => return JSValue::undefined(),
                }
            }
        }
        None => Ok(String::new()),
    };

    match (name, message) {
        (Ok(n), Ok(m)) => {
            if m.is_empty() {
                JSValue::new_string(ctx.intern(&n))
            } else {
                JSValue::new_string(ctx.intern(&format!("{}: {}", n, m)))
            }
        }
        _ => JSValue::undefined(),
    }
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin(
        "error_constructor",
        HostFunction::ctor("Error", 1, error_constructor),
    );
    ctx.register_builtin(
        "type_error_constructor",
        HostFunction::ctor("TypeError", 1, type_error_constructor),
    );
    ctx.register_builtin(
        "reference_error_constructor",
        HostFunction::ctor("ReferenceError", 1, reference_error_constructor),
    );
    ctx.register_builtin(
        "syntax_error_constructor",
        HostFunction::ctor("SyntaxError", 1, syntax_error_constructor),
    );
    ctx.register_builtin(
        "range_error_constructor",
        HostFunction::ctor("RangeError", 1, range_error_constructor),
    );
    ctx.register_builtin(
        "uri_error_constructor",
        HostFunction::ctor("URIError", 1, uri_error_constructor),
    );
    ctx.register_builtin(
        "eval_error_constructor",
        HostFunction::ctor("EvalError", 1, eval_error_constructor),
    );
    ctx.register_builtin(
        "error_toString",
        HostFunction::method("toString", 0, error_to_string),
    );
}
