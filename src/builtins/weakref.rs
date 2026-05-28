use crate::host::HostFunction;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;
fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(name), 1);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin(
        "weakref_constructor",
        HostFunction::ctor("WeakRef", 1, weakref_constructor),
    );
    ctx.register_builtin(
        "weakref_deref",
        HostFunction::method("deref", 0, weakref_deref),
    );
    ctx.register_builtin(
        "finalization_registry_constructor",
        HostFunction::ctor("FinalizationRegistry", 1, finalization_registry_constructor),
    );
    ctx.register_builtin(
        "finalization_registry_register",
        HostFunction::method("register", 2, finalization_registry_register),
    );
    ctx.register_builtin(
        "finalization_registry_unregister",
        HostFunction::method("unregister", 1, finalization_registry_unregister),
    );
}

pub fn init_weakref(ctx: &mut JSContext) {
    let mut weakref_proto = JSObject::new();
    weakref_proto.set(
        ctx.intern("deref"),
        create_builtin_function(ctx, "weakref_deref"),
    );

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        weakref_proto.prototype = Some(obj_proto_ptr);
    }

    let weakref_proto_ptr = Box::into_raw(Box::new(weakref_proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(weakref_proto_ptr);
    ctx.set_weakref_prototype(weakref_proto_ptr);

    let weakref_ctor = create_builtin_function(ctx, "weakref_constructor");
    let weakref_atom = ctx.intern("WeakRef");
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, weakref_atom, weakref_ctor);
    }

    let mut fr_proto = JSObject::new();
    fr_proto.set(
        ctx.intern("register"),
        create_builtin_function(ctx, "finalization_registry_register"),
    );
    fr_proto.set(
        ctx.intern("unregister"),
        create_builtin_function(ctx, "finalization_registry_unregister"),
    );

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        fr_proto.prototype = Some(obj_proto_ptr);
    }

    let fr_proto_ptr = Box::into_raw(Box::new(fr_proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(fr_proto_ptr);
    ctx.set_finalization_registry_prototype(fr_proto_ptr);

    let fr_ctor = create_builtin_function(ctx, "finalization_registry_constructor");
    let fr_atom = ctx.intern("FinalizationRegistry");
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, fr_atom, fr_ctor);
    }
}

pub fn weakref_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let mut obj = JSObject::new();

    if let Some(proto_ptr) = ctx.get_weakref_prototype() {
        obj.prototype = Some(proto_ptr);
    }

    if args.is_empty() {
        obj.set(ctx.intern("__weakref_target__"), JSValue::undefined());
        let ptr = Box::into_raw(Box::new(obj)) as usize;
        return JSValue::new_object(ptr);
    }

    let target = &args[0];
    if !target.is_object() {
        obj.set(ctx.intern("__weakref_target__"), target.clone());
        let ptr = Box::into_raw(Box::new(obj)) as usize;
        return JSValue::new_object(ptr);
    }

    let target_ptr = target.get_ptr();

    obj.set(
        ctx.intern("__weakref_target__"),
        JSValue::new_int(target_ptr as i64),
    );
    obj.set(ctx.intern("__is_weakref__"), JSValue::bool(true));

    let ptr = Box::into_raw(Box::new(obj)) as usize;
    JSValue::new_object(ptr)
}

pub fn weakref_deref(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }

    let this = &args[0];
    if !this.is_object() {
        return JSValue::undefined();
    }

    let obj = this.as_object();

    let _is_weakref_check = obj.get(ctx.intern("__is_weakref__"));
    match _is_weakref_check {
        Some(v) if !v.is_undefined() && v.get_bool() => {}
        _ => return JSValue::undefined(),
    };

    let target_value = obj.get(ctx.intern("__weakref_target__"));
    let target_value = match target_value {
        Some(v) => v,
        None => return JSValue::undefined(),
    };

    if target_value.is_undefined() {
        return JSValue::undefined();
    }

    if target_value.is_int() {
        let target_ptr = target_value.get_int() as usize;

        if target_ptr == 0 {
            return JSValue::undefined();
        }

        return JSValue::new_object(target_ptr);
    }

    target_value
}

pub fn finalization_registry_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let mut obj = JSObject::new();

    if let Some(proto_ptr) = ctx.get_finalization_registry_prototype() {
        obj.prototype = Some(proto_ptr);
    }

    obj.set(
        ctx.intern("__is_finalization_registry__"),
        JSValue::bool(true),
    );

    let held_values_obj = JSObject::new();
    let held_values_ptr = Box::into_raw(Box::new(held_values_obj)) as usize;
    obj.set(
        ctx.intern("__held_values__"),
        JSValue::new_object(held_values_ptr),
    );

    if !args.is_empty() {
        let callback = &args[0];
        if callback.is_function() {
            obj.set(ctx.intern("__callback__"), callback.clone());
        }
    }

    let ptr = Box::into_raw(Box::new(obj)) as usize;
    let obj_value = JSValue::new_object(ptr);

    ctx.finalization_registries.borrow_mut().push(ptr);

    obj_value
}

pub fn finalization_registry_register(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }

    let this = &args[0];
    if !this.is_object() {
        return JSValue::undefined();
    }

    let obj = this.as_object_mut();

    let is_fr = obj.get(ctx.intern("__is_finalization_registry__"));
    match is_fr {
        Some(v) if !v.is_undefined() && v.get_bool() => {}
        _ => return JSValue::undefined(),
    };

    if args.len() < 3 {
        return JSValue::undefined();
    }

    let _target = &args[1];
    let held_value = &args[2];
    let unregister_token = if args.len() > 3 {
        &args[3]
    } else {
        &JSValue::undefined()
    };

    let held_values_atom = ctx.intern("__held_values__");
    let held_values_val = obj.get(held_values_atom);

    if let Some(hvv) = held_values_val {
        if hvv.is_object() {
            let held_obj = hvv.as_object_mut();

            let key = if unregister_token.is_object() {
                let token_ptr = unregister_token.get_ptr();
                ctx.intern(&format!("__token_{}__", token_ptr))
            } else {
                ctx.intern(&format!("__entry_{}__", held_obj.inline_values_len()))
            };

            held_obj.set(key, held_value.clone());
        }
    }

    JSValue::undefined()
}

pub fn finalization_registry_unregister(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::bool(false);
    }

    let this = &args[0];
    if !this.is_object() {
        return JSValue::bool(false);
    }

    let obj = this.as_object_mut();

    let is_fr = obj.get(ctx.intern("__is_finalization_registry__"));
    match is_fr {
        Some(v) if !v.is_undefined() && v.get_bool() => {}
        _ => return JSValue::bool(false),
    };

    if args.len() < 2 {
        return JSValue::bool(false);
    }

    let unregister_token = &args[1];

    if !unregister_token.is_object() {
        return JSValue::bool(false);
    }

    let token_ptr = unregister_token.get_ptr();
    let key = ctx.intern(&format!("__token_{}__", token_ptr));

    let held_values_atom = ctx.intern("__held_values__");
    let held_values_val = obj.get(held_values_atom);

    if let Some(hvv) = held_values_val {
        if hvv.is_object() {
            let held_obj = hvv.as_object_mut();

            if held_obj.has_own(key) {
                held_obj.delete(key);
                return JSValue::bool(true);
            }
        }
    }

    JSValue::bool(false)
}
