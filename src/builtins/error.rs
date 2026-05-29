use crate::host::HostFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

fn set_ctor_prototype(obj: &mut JSObject, key: crate::runtime::atom::Atom, val: JSValue) {
    obj.define_property(key, crate::object::object::PropertyDescriptor {
        value: Some(val),
        writable: false,
        enumerable: false,
        configurable: false,
        get: None,
        set: None,
    });
}

fn set_own_ne(obj: &mut JSObject, key: crate::runtime::atom::Atom, val: JSValue) {
    obj.define_property(key, crate::object::object::PropertyDescriptor {
        value: Some(val),
        writable: true,
        enumerable: false,
        configurable: true,
        get: None,
        set: None,
    });
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
    fn set_ne(obj: &mut crate::object::object::JSObject, key: crate::runtime::atom::Atom, val: crate::value::JSValue) {
        obj.define_property(key, crate::object::object::PropertyDescriptor {
            value: Some(val),
            writable: true,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        });
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
    set_ne(&mut proto_obj,ctx.intern("name"), JSValue::new_string(ctx.intern("Error")));
    set_ne(&mut proto_obj,ctx.intern("message"), JSValue::new_string(ctx.intern("")));
    set_ne(&mut proto_obj,
        ctx.intern("toString"),
        create_builtin_function(ctx, "error_toString"),
    );

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto_obj.prototype = Some(obj_proto_ptr);
    }

    set_ne(&mut proto_obj,ctx.intern("constructor"), error_value);

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
}

fn init_type_error(ctx: &mut JSContext) {
    fn set_ne(obj: &mut crate::object::object::JSObject, key: crate::runtime::atom::Atom, val: crate::value::JSValue) {
        obj.define_property(key, crate::object::object::PropertyDescriptor {
            value: Some(val),
            writable: true,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        });
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
    set_ne(&mut proto,
        ctx.intern("name"),
        JSValue::new_string(ctx.intern("TypeError")),
    );
    set_ne(&mut proto,ctx.intern("message"), JSValue::new_string(ctx.intern("")));

    if let Some(error_proto_ptr) = ctx.get_error_prototype() {
        proto.prototype = Some(error_proto_ptr);
    }
    set_ne(&mut proto,ctx.intern("constructor"), value);

    let proto_ptr = Box::into_raw(Box::new(proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);
    ctx.set_type_error_prototype(proto_ptr);
    let ctor = unsafe { JSValue::function_from_ptr_mut(ptr) };
    set_ctor_prototype(&mut ctor.base, ctx.intern("prototype"), proto_value);
}

fn init_reference_error(ctx: &mut JSContext) {
    fn set_ne(obj: &mut crate::object::object::JSObject, key: crate::runtime::atom::Atom, val: crate::value::JSValue) {
        obj.define_property(key, crate::object::object::PropertyDescriptor {
            value: Some(val),
            writable: true,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        });
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
    set_ne(&mut proto,
        ctx.intern("name"),
        JSValue::new_string(ctx.intern("ReferenceError")),
    );
    set_ne(&mut proto,ctx.intern("message"), JSValue::new_string(ctx.intern("")));
    if let Some(error_proto_ptr) = ctx.get_error_prototype() {
        proto.prototype = Some(error_proto_ptr);
    }
    set_ne(&mut proto,ctx.intern("constructor"), value);

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
    fn set_ne(obj: &mut crate::object::object::JSObject, key: crate::runtime::atom::Atom, val: crate::value::JSValue) {
        obj.define_property(key, crate::object::object::PropertyDescriptor {
            value: Some(val),
            writable: true,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        });
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
    set_ne(&mut proto,
        ctx.intern("name"),
        JSValue::new_string(ctx.intern("SyntaxError")),
    );
    set_ne(&mut proto,ctx.intern("message"), JSValue::new_string(ctx.intern("")));
    if let Some(error_proto_ptr) = ctx.get_error_prototype() {
        proto.prototype = Some(error_proto_ptr);
    }
    set_ne(&mut proto,ctx.intern("constructor"), value);

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
    fn set_ne(obj: &mut crate::object::object::JSObject, key: crate::runtime::atom::Atom, val: crate::value::JSValue) {
        obj.define_property(key, crate::object::object::PropertyDescriptor {
            value: Some(val),
            writable: true,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        });
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
    set_ne(&mut proto,
        ctx.intern("name"),
        JSValue::new_string(ctx.intern("RangeError")),
    );
    set_ne(&mut proto,ctx.intern("message"), JSValue::new_string(ctx.intern("")));
    if let Some(error_proto_ptr) = ctx.get_error_prototype() {
        proto.prototype = Some(error_proto_ptr);
    }
    set_ne(&mut proto,ctx.intern("constructor"), value);

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

fn build_error(ctx: &mut JSContext, name: &str, args: &[JSValue], proto: Option<*mut JSObject>) -> JSValue {
    let mut err = JSObject::new();
    set_own_ne(&mut err, ctx.intern("name"), JSValue::new_string(ctx.intern(name)));
    if !args.is_empty() && args[0].is_string() {
        set_own_ne(&mut err, ctx.intern("message"), args[0]);
    } else {
        set_own_ne(&mut err, ctx.intern("message"), JSValue::new_string(ctx.intern("")));
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
    let proto = ctx.get_reference_error_prototype()
        .or_else(|| ctx.get_error_prototype())
        .map(|p| p as *mut _);
    build_error(ctx, "ReferenceError", args, proto)
}

pub fn syntax_error_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let proto = ctx.get_syntax_error_prototype().map(|p| p as *mut _);
    build_error(ctx, "SyntaxError", args, proto)
}

pub fn range_error_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let proto = ctx.get_range_error_prototype()
        .or_else(|| ctx.get_error_prototype())
        .map(|p| p as *mut _);
    build_error(ctx, "RangeError", args, proto)
}

fn error_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern("Error"));
    }
    let this = &args[0];
    if !this.is_object() {
        return JSValue::new_string(ctx.intern("Error"));
    }

    let obj = this.as_object();

    let name = if let Some(n) = obj.get(ctx.intern("name")) {
        if n.is_string() {
            ctx.get_atom_str(n.get_atom()).to_string()
        } else {
            "Error".to_string()
        }
    } else {
        "Error".to_string()
    };

    let message = if let Some(m) = obj.get(ctx.intern("message")) {
        if m.is_string() {
            ctx.get_atom_str(m.get_atom()).to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    if message.is_empty() {
        JSValue::new_string(ctx.intern(&name))
    } else {
        JSValue::new_string(ctx.intern(&format!("{}: {}", name, message)))
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
        "error_toString",
        HostFunction::method("toString", 0, error_to_string),
    );
}
