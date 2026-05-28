use crate::host::HostFunction;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

fn create_builtin_method(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(name), 1);
    func.set_builtin_marker(ctx, name);
    if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
        func.base.set_prototype_raw(fn_proto_ptr);
    }
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn patch_function_values_with_function_prototype(obj: &mut JSObject, fn_proto_ptr: *mut JSObject) {
    let mut values = Vec::new();
    obj.for_each_property(|_atom, value, _attrs| {
        values.push(value);
    });

    for value in values {
        if value.is_function() {
            let func = unsafe { JSValue::function_from_ptr_mut(value.get_ptr()) };
            func.base.set_prototype_raw(fn_proto_ptr);
        }
    }
}

pub fn init_function(ctx: &mut JSContext) {
    let mut proto_obj = JSObject::new_function();
    if let Some(object_proto_ptr) = ctx.get_object_prototype() {
        proto_obj.prototype = Some(object_proto_ptr);
    }
    proto_obj.set(
        ctx.common_atoms.bind,
        create_builtin_method(ctx, "function_bind"),
    );
    proto_obj.set(
        ctx.common_atoms.call,
        create_builtin_method(ctx, "function_call"),
    );
    proto_obj.set(
        ctx.common_atoms.apply,
        create_builtin_method(ctx, "function_apply"),
    );
    proto_obj.set(
        ctx.common_atoms.to_string,
        create_builtin_method(ctx, "function_toString"),
    );
    proto_obj.define_accessor(
        ctx.common_atoms.length,
        Some(create_builtin_method(ctx, "function_length")),
        None,
    );
    proto_obj.define_accessor(
        ctx.common_atoms.name,
        Some(create_builtin_method(ctx, "function_name")),
        None,
    );
    // Throw TypeError accessors for .caller and .arguments.
    // Per spec, bound and strict mode functions throw when accessing these.
    let caller_getter = {
        let mut f = JSFunction::new_builtin(ctx.intern("caller"), 0);
        f.set_builtin_marker(ctx, "throw_type_error_caller");
        let ptr = Box::into_raw(Box::new(f)) as usize;
        ctx.runtime_mut().gc_heap_mut().track_function(ptr);
        JSValue::new_function(ptr)
    };
    let args_getter = {
        let mut f = JSFunction::new_builtin(ctx.intern("arguments"), 0);
        f.set_builtin_marker(ctx, "throw_type_error_caller");
        let ptr = Box::into_raw(Box::new(f)) as usize;
        ctx.runtime_mut().gc_heap_mut().track_function(ptr);
        JSValue::new_function(ptr)
    };
    proto_obj.define_non_enumerable_accessor(
        ctx.intern("caller"),
        Some(caller_getter),
        Some(caller_getter),
    );
    proto_obj.define_non_enumerable_accessor(
        ctx.intern("arguments"),
        Some(args_getter),
        Some(args_getter),
    );

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto_obj.prototype = Some(obj_proto_ptr);
    }

    let proto_ptr = Box::into_raw(Box::new(proto_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);

    ctx.set_function_prototype(proto_ptr);

    let proto_obj_mut = unsafe { &mut *(proto_ptr as *mut JSObject) };
    for atom in [
        ctx.common_atoms.bind,
        ctx.common_atoms.call,
        ctx.common_atoms.apply,
        ctx.common_atoms.to_string,
    ] {
        if let Some(v) = proto_obj_mut.get(atom) {
            if v.is_function() {
                let f = unsafe { JSValue::function_from_ptr_mut(v.get_ptr()) };
                f.base.set_prototype_raw(proto_ptr as *mut JSObject);
            }
        }
    }

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        patch_function_values_with_function_prototype(global_obj, proto_ptr as *mut JSObject);

        let mut nested = Vec::new();
        global_obj.for_each_property(|_atom, value, _attrs| {
            if value.is_object() || value.is_function() {
                nested.push(value);
            }
        });

        for value in nested {
            let nested_obj = value.as_object_mut();
            patch_function_values_with_function_prototype(nested_obj, proto_ptr as *mut JSObject);
        }
    }

    let mut function_ctor = JSFunction::new_builtin(ctx.common_atoms.function, 1);
    function_ctor.set_builtin_marker(ctx, "function_constructor");
    function_ctor.base.prototype = Some(proto_ptr as *mut JSObject);
    function_ctor
        .base
        .set(ctx.common_atoms.prototype, proto_value.clone());

    let function_ptr = Box::into_raw(Box::new(function_ctor)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(function_ptr);
    let function_value = JSValue::new_function(function_ptr);

    unsafe {
        let proto_obj_ptr = proto_ptr as *mut crate::object::object::JSObject;
        (*proto_obj_ptr).set(ctx.common_atoms.constructor, function_value);
    }

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, ctx.common_atoms.function, function_value);
        crate::builtins::global::set_non_enumerable(global_obj, ctx.intern("FunctionPrototype"), proto_value);
    }
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin("function_bind", HostFunction::method("bind", 1, function_bind));
    ctx.register_builtin("function_call", HostFunction::method("call", 1, function_call));
    ctx.register_builtin(
        "function_apply",
        HostFunction::method("apply", 2, function_apply),
    );
    ctx.register_builtin(
        "function_toString",
        HostFunction::method("toString", 0, function_to_string),
    );
    ctx.register_builtin(
        "function_length",
        HostFunction::method("length", 0, function_length),
    );
    ctx.register_builtin("function_name", HostFunction::method("name", 0, function_name));
    ctx.register_builtin(
        "function_constructor",
        HostFunction::ctor("Function", 1, function_constructor),
    );
    ctx.register_builtin(
        "function_has_instance",
        HostFunction::method(SYMBOL_HAS_INSTANCE_DISPLAY, 1, function_has_instance),
    );
    ctx.register_builtin(
        "throw_type_error_callee",
        HostFunction::new("callee", 0, throw_type_error_callee),
    );
    ctx.register_builtin(
        "throw_type_error_caller",
        HostFunction::new("caller", 0, throw_type_error_caller_args),
    );
}

fn throw_type_error_callee(ctx: &mut JSContext, _args: &[JSValue]) -> JSValue {
    if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
        let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
        vm.set_pending_type_error(ctx, "'caller', 'callee', and 'arguments' properties may not be accessed on strict mode functions or the arguments objects for calls to them");
    }
    JSValue::undefined()
}

fn throw_type_error_caller_args(ctx: &mut JSContext, _args: &[JSValue]) -> JSValue {
    if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
        let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
        vm.set_pending_type_error(ctx, "'caller', 'callee', and 'arguments' properties may not be accessed on strict mode functions or the arguments objects for calls to them");
    }
    JSValue::undefined()
}

const SYMBOL_HAS_INSTANCE_DISPLAY: &str = "[Symbol.hasInstance]";

fn function_has_instance(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::bool(false);
    }
    let f = &args[0];
    let v = &args[1];
    if !f.is_function() && !f.is_object() {
        return JSValue::bool(false);
    }
    if !v.is_object() && !v.is_function() {
        return JSValue::bool(false);
    }
    let proto_atom = ctx.common_atoms.prototype;
    let proto_ptr = if f.is_function() {
        let js_func = f.as_function();
        if !js_func.cached_prototype_ptr.is_null() {
            js_func.cached_prototype_ptr as *const crate::object::object::JSObject
        } else {
            let pv = js_func.base.get(proto_atom).unwrap_or(JSValue::undefined());
            if pv.is_object() {
                pv.get_ptr() as *const crate::object::object::JSObject
            } else {
                return JSValue::bool(false);
            }
        }
    } else {
        let pv = f
            .as_object()
            .get(proto_atom)
            .unwrap_or(JSValue::undefined());
        if pv.is_object() {
            pv.get_ptr() as *const crate::object::object::JSObject
        } else {
            return JSValue::bool(false);
        }
    };
    let obj_ptr = v.get_ptr() as *const crate::object::object::JSObject;
    let mut current = unsafe { (*obj_ptr).prototype };
    while let Some(p) = current {
        if std::ptr::eq(p, proto_ptr) {
            return JSValue::bool(true);
        }
        current = unsafe { (*p).prototype };
    }
    JSValue::bool(false)
}

fn function_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return match crate::eval(ctx, "(function(){})") {
            Ok(val) => val,
            Err(_) => JSValue::undefined(),
        };
    }
    let body_idx = args.len() - 1;
    let body_str = if args[body_idx].is_string() {
        ctx.get_atom_str(args[body_idx].get_atom()).to_string()
    } else {
        String::new()
    };
    let mut params = Vec::new();
    for i in 0..body_idx {
        let p = if args[i].is_string() {
            ctx.get_atom_str(args[i].get_atom()).to_string()
        } else {
            String::new()
        };
        for part in p.trim().split(',') {
            let part = part.trim();
            if !part.is_empty() {
                params.push(part.to_string());
            }
        }
    }
    let params_str = params.join(",");
    let source = format!("(function({}){{{}}})", params_str, body_str);
    match crate::eval(ctx, &source) {
        Ok(val) => val,
        Err(e) => {
            let mut err = crate::object::object::JSObject::new();
            err.set(
                ctx.intern("name"),
                JSValue::new_string(ctx.intern("SyntaxError")),
            );
            err.set(ctx.intern("message"), JSValue::new_string(ctx.intern(&e)));
            if let Some(proto) = ctx.get_syntax_error_prototype() {
                err.prototype = Some(proto);
            }
            let ptr = Box::into_raw(Box::new(err)) as usize;
            ctx.runtime_mut().gc_heap_mut().track(ptr);
            ctx.pending_exception = Some(JSValue::new_object(ptr));
            JSValue::undefined()
        }
    }
}

fn function_bind(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }

    let this_val = &args[0];
    if !this_val.is_function() {
        return JSValue::undefined();
    }

    let this_arg = if args.len() > 1 {
        args[1].clone()
    } else {
        JSValue::undefined()
    };

    let mut wrapper = JSObject::new();
    if let Some(proto_ptr) = ctx.get_function_prototype() {
        wrapper.prototype = Some(proto_ptr);
    }
    wrapper.set(ctx.common_atoms.__boundFn, *this_val);
    wrapper.set(ctx.common_atoms.__boundThis, this_arg);

    let mut args_arr = JSObject::new_array();
    let length_key = ctx.common_atoms.length;
    let bound_count = if args.len() > 2 {
        (args.len() - 2) as i64
    } else {
        0
    };
    args_arr.set(length_key, JSValue::new_int(bound_count));
    let mut idx = 0;
    for arg in args.iter().skip(2) {
        let key = ctx.intern(&idx.to_string());
        args_arr.set(key, arg.clone());
        idx += 1;
    }
    let boxed_args_arr = Box::new(args_arr);
    let args_ptr = Box::into_raw(boxed_args_arr) as usize;
    ctx.runtime_mut().gc_heap_mut().track(args_ptr);
    wrapper.set(ctx.common_atoms.__boundArgs, JSValue::new_object(args_ptr));

    let boxed_wrapper = Box::new(wrapper);
    let ptr = Box::into_raw(boxed_wrapper) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}

fn function_call(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this_val = &args[0];
    if !this_val.is_function() {
        return JSValue::undefined();
    }
    let this_arg = if args.len() > 1 {
        args[1].clone()
    } else {
        JSValue::undefined()
    };
    let mut call_args = Vec::new();
    for arg in args.iter().skip(2) {
        call_args.push(arg.clone());
    }

     if let Some(ptr) = ctx.get_register_vm_ptr() {
         let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
         let result = vm.call_function_with_this(ctx, *this_val, this_arg, &call_args);
         match result {
             Ok(val) => val,
             Err(msg) => {
                 let mut err = crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
                 let msg_only = if let Some(idx) = msg.find(": ") {
                     msg[idx + 2..].to_string()
                 } else {
                     msg.clone()
                 };
                 err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern(&msg_only)));
                 if msg.contains("TypeError") {
                     err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
                     if let Some(proto) = ctx.get_type_error_prototype() {
                         err.prototype = Some(proto);
                     }
                 } else if msg.contains("RangeError") {
                     err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("RangeError")));
                     if let Some(proto) = ctx.get_range_error_prototype() {
                         err.prototype = Some(proto);
                     }
                 } else {
                     err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("Error")));
                     if let Some(proto) = ctx.get_error_prototype() {
                         err.prototype = Some(proto);
                     }
                 }
                 let err_ptr = Box::into_raw(Box::new(err)) as usize;
                 ctx.runtime_mut().gc_heap_mut().track(err_ptr);
                 vm.pending_throw = Some(JSValue::new_object(err_ptr));
                 JSValue::undefined()
             }
         }
     } else {
         JSValue::undefined()
     }
}

fn function_apply(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this_val = &args[0];
    if !this_val.is_function() {
        return JSValue::undefined();
    }

    let this_arg = if args.len() > 1 {
        args[1].clone()
    } else {
        JSValue::undefined()
    };

    let mut call_args_buf = [JSValue::undefined(); 16];
    let mut call_args_vec = Vec::new();
    let call_args: &[JSValue];

    if args.len() > 2 && args[2].is_object() {
        let arr_obj = args[2].as_object();
        let length_atom = ctx.common_atoms.length;

        let len_val = arr_obj.get(length_atom);
        if let Some(lv) = len_val {
            if !lv.is_int() {
                call_args = &[];
                if let Some(ptr) = ctx.get_register_vm_ptr() {
                    let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                    let result = vm.call_function_with_this(ctx, *this_val, this_arg, call_args);
                    return match result {
                        Ok(val) => val,
                        Err(msg) => {
                            let mut err = crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
                            let msg_only = if let Some(idx) = msg.find(": ") {
                                msg[idx + 2..].to_string()
                            } else {
                                msg.clone()
                            };
                            err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern(&msg_only)));
                            if msg.contains("TypeError") {
                                err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
                                if let Some(proto) = ctx.get_type_error_prototype() {
                                    err.prototype = Some(proto);
                                }
                            } else if msg.contains("RangeError") {
                                err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("RangeError")));
                                if let Some(proto) = ctx.get_range_error_prototype() {
                                    err.prototype = Some(proto);
                                }
                            } else {
                                err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("Error")));
                                if let Some(proto) = ctx.get_error_prototype() {
                                    err.prototype = Some(proto);
                                }
                            }
                            let err_ptr = Box::into_raw(Box::new(err)) as usize;
                            ctx.runtime_mut().gc_heap_mut().track(err_ptr);
                            vm.pending_throw = Some(JSValue::new_object(err_ptr));
                            JSValue::undefined()
                        }
                    };
                }
                return JSValue::undefined();
            }
            let len = lv.get_int() as usize;

            if arr_obj.is_mapped_arguments() {
                let fi = arr_obj.mapped_args_frame_index();
                let param_count = arr_obj.mapped_args_param_count() as usize;
                let vm_ptr = ctx.get_register_vm_ptr();
                if let Some(ptr) = vm_ptr {
                    let vm = unsafe { &*(ptr as *const crate::runtime::vm::VM) };
                    if fi < vm.frames.len() {
                        let frame = &vm.frames[fi];
                        let base = frame.registers_base;
                        let target = if len <= 16 {
                            &mut call_args_buf[..len]
                        } else {
                            call_args_vec.resize(len, JSValue::undefined());
                            &mut call_args_vec[..len]
                        };
                        let saved = &frame.saved_args;
                        for i in 0..len {
                            target[i] = if i < param_count {
                                let reg_idx = base + 1 + i;
                                if reg_idx < vm.registers.len() {
                                    vm.registers[reg_idx]
                                } else {
                                    JSValue::undefined()
                                }
                            } else if i < saved.len() {
                                saved[i]
                            } else {
                                arr_obj.get_indexed(i).unwrap_or(JSValue::undefined())
                            };
                        }
                        call_args = if len <= 16 {
                            &call_args_buf[..len]
                        } else {
                            &call_args_vec[..len]
                        };
                    } else {
                        call_args = &[];
                    }
                } else {
                    call_args = &[];
                }
            } else if len <= 16 {
                if arr_obj.is_array() {
                    let ptr = arr_obj as *const _ as usize;
                    if arr_obj.is_dense_array() {
                        let arr_ptr =
                            unsafe { &*(ptr as *const crate::object::array_obj::JSArrayObject) };
                        for i in 0..len {
                            call_args_buf[i] = arr_ptr.get(i).unwrap_or(JSValue::undefined());
                        }
                    } else {
                        for i in 0..len {
                            let key = ctx.int_atom_mut(i);
                            call_args_buf[i] = arr_obj.get(key).unwrap_or(JSValue::undefined());
                        }
                    }
                } else if let Some(slice) = arr_obj.get_dense_slice(len) {
                    call_args_buf[..len].copy_from_slice(slice);
                } else {
                    for i in 0..len {
                        if let Some(val) = arr_obj.get_indexed(i) {
                            call_args_buf[i] = val;
                        } else {
                            let key = ctx.int_atom_mut(i);
                            call_args_buf[i] = arr_obj.get(key).unwrap_or(JSValue::undefined());
                        }
                    }
                }
                call_args = &call_args_buf[..len];
            } else {
                if arr_obj.is_array() {
                    let ptr = arr_obj as *const _ as usize;
                    if arr_obj.is_dense_array() {
                        let arr_ptr =
                            unsafe { &*(ptr as *const crate::object::array_obj::JSArrayObject) };
                        for i in 0..len {
                            call_args_vec.push(arr_ptr.get(i).unwrap_or(JSValue::undefined()));
                        }
                    } else {
                        for i in 0..len {
                            let key = ctx.int_atom_mut(i);
                            call_args_vec.push(arr_obj.get(key).unwrap_or(JSValue::undefined()));
                        }
                    }
                } else if let Some(slice) = arr_obj.get_dense_slice(len) {
                    call_args_vec.extend_from_slice(slice);
                } else {
                    for i in 0..len {
                        if let Some(val) = arr_obj.get_indexed(i) {
                            call_args_vec.push(val);
                        } else {
                            let key = ctx.int_atom_mut(i);
                            if let Some(val) = arr_obj.get(key) {
                                call_args_vec.push(val);
                            } else {
                                call_args_vec.push(JSValue::undefined());
                            }
                        }
                    }
                }
                call_args = &call_args_vec;
            }
        } else {
            call_args = &[];
        }
    } else {
        call_args = &[];
    }

    if let Some(ptr) = ctx.get_register_vm_ptr() {
        let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
        let result = vm.call_function_with_this(ctx, *this_val, this_arg, call_args);
        match result {
            Ok(val) => val,
            Err(_) => {
                if let Some(exc) = ctx.pending_exception.take() {
                    vm.pending_throw = Some(exc);
                }
                JSValue::undefined()
            }
        }
    } else {
        JSValue::undefined()
    }
}

fn function_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern("function () { [native code] }"));
    }
    let this_val = &args[0];
    if this_val.is_function() {
        let js_func = this_val.as_function();

        let func_name = ctx.get_atom_str(js_func.name).to_string();
        let arity = js_func.arity as usize;

        let params: Vec<String> = (0..arity).map(|i| format!("a{}", i)).collect();
        let param_str = params.join(", ");

        if js_func.is_builtin() {
            if func_name.is_empty() {
                JSValue::new_string(
                    ctx.intern(&format!("function({}) {{ [native code] }}", param_str)),
                )
            } else {
                JSValue::new_string(ctx.intern(&format!(
                    "function {}({}) {{ [native code] }}",
                    func_name, param_str
                )))
            }
        } else {
            let prefix = if js_func.is_async() { "async " } else { "" };
            let suffix = if js_func.is_generator() { "*" } else { "" };
            if func_name.is_empty() {
                JSValue::new_string(ctx.intern(&format!(
                    "{}function{}({}) {{ [user code] }}",
                    prefix, suffix, param_str
                )))
            } else {
                JSValue::new_string(ctx.intern(&format!(
                    "{}function{} {}({}) {{ [user code] }}",
                    prefix, suffix, func_name, param_str
                )))
            }
        }
    } else {
        JSValue::new_string(ctx.intern("[object Function]"))
    }
}

fn function_length(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this_val = &args[0];
    if !this_val.is_function() {
        return JSValue::undefined();
    }
    let js_func = this_val.as_function();
    JSValue::new_int(js_func.arity as i64)
}

fn function_name(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let this_val = &args[0];
    if !this_val.is_function() {
        return JSValue::undefined();
    }
    let js_func = this_val.as_function();
    let name_str = ctx.get_atom_str(js_func.name).to_string();
    JSValue::new_string(ctx.intern(&name_str))
}
