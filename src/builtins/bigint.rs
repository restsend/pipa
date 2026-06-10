use crate::host::HostFunction;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::object::object::PropertyDescriptor;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(name), 2);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn set_ne(obj: &mut JSObject, key: crate::runtime::atom::Atom, val: JSValue) {
    obj.define_property(key, PropertyDescriptor {
        value: Some(val), writable: true, enumerable: false, configurable: true,
        get: None, set: None,
    });
}

fn to_bigint(ctx: &mut JSContext, value: JSValue) -> Option<i128> {
    if value.is_bigint() {
        return Some(value.as_object().get_bigint_value());
    }
    if value.is_int() {
        return Some(value.get_int() as i128);
    }
    if value.is_bool() {
        return Some(if value.get_bool() { 1i128 } else { 0i128 });
    }
    if value.is_null() {
        let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Cannot convert null to a BigInt")));
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return None;
    }
    if value.is_undefined() {
        let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Cannot convert undefined to a BigInt")));
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return None;
    }
    if value.is_symbol() {
        let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Cannot convert a Symbol to a BigInt")));
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return None;
    }
    if value.is_float() {
        let f = value.get_float();
        if f.is_nan() || f.is_infinite() {
            let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
            if let Some(proto) = ctx.get_range_error_prototype() { err.prototype = Some(proto); }
            err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("RangeError")));
            err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Cannot convert Infinity/NaN to a BigInt")));
            let ptr = Box::into_raw(Box::new(err)) as usize;
            ctx.runtime_mut().gc_heap_mut().track(ptr);
            ctx.pending_exception = Some(JSValue::new_object(ptr));
            return None;
        }
        return Some(f as i128);
    }
    if value.is_string() {
        let s = ctx.get_atom_str(value.get_atom());
        let cleaned = s.trim().replace('_', "");
        let c = cleaned.as_str();
        if c.is_empty() {
            return Some(0i128);
        }
        let (radix, num_str) = if c.starts_with("0x") || c.starts_with("0X") {
            (16, &c[2..])
        } else if c.starts_with("0b") || c.starts_with("0B") {
            (2, &c[2..])
        } else if c.starts_with("0o") || c.starts_with("0O") {
            (8, &c[2..])
        } else {
            let mut chars = c.chars();
            let first = chars.next().unwrap();
            if first == '+' || first == '-' {
                (10, c)
            } else {
                (10, c)
            }
        };
        if num_str.is_empty() || num_str == "+" || num_str == "-" {
            return Some(0i128);
        }
        if let Ok(val) = i128::from_str_radix(num_str, radix) {
            return Some(val);
        }
        let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_syntax_error_prototype() { err.prototype = Some(proto); }
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("SyntaxError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Cannot convert string to BigInt")));
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return None;
    }
    if value.is_object() {
        if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
            let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
            let to_prim_sym = crate::builtins::symbol::get_or_create_well_known_symbol(ctx, "Symbol.toPrimitive");
            let to_prim_key = crate::runtime::atom::Atom(0x40000000 | to_prim_sym.get_symbol_id());
            let to_primitive = value.as_object().get(to_prim_key);
            if let Some(f) = to_primitive {
                if f.is_function() {
                    let hint = JSValue::new_string(ctx.intern("number"));
                    if let Ok(r) = vm.call_function_with_this(ctx, f, value, &[hint]) {
                        if !r.is_object() {
                            return to_bigint(ctx, r);
                        }
                        let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
                        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
                        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
                        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Cannot convert object to a BigInt")));
                        let ptr = Box::into_raw(Box::new(err)) as usize;
                        ctx.runtime_mut().gc_heap_mut().track(ptr);
                        ctx.pending_exception = Some(JSValue::new_object(ptr));
                        return None;
                    }
                    return None;
                }
            }
            let value_of = value.as_object().get(ctx.common_atoms.value_of);
            if let Some(f) = value_of {
                if f.is_function() {
                    if let Ok(r) = vm.call_function_with_this(ctx, f, value, &[]) {
                        if !r.is_object() {
                            return to_bigint(ctx, r);
                        }
                    } else {
                        return None;
                    }
                }
            }
            let to_string = value.as_object().get(ctx.common_atoms.to_string);
            if let Some(f) = to_string {
                if f.is_function() {
                    if let Ok(r) = vm.call_function_with_this(ctx, f, value, &[]) {
                        if r.is_string() {
                            return to_bigint(ctx, r);
                        }
                    } else {
                        return None;
                    }
                }
            }
        }
        let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Cannot convert object to a BigInt")));
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return None;
    }
    Some(0i128)
}

fn to_index(ctx: &mut JSContext, value: JSValue) -> Option<u64> {
    let int_val = if value.is_int() {
        value.get_int()
    } else if value.is_float() {
        let f = value.get_float();
        if f.is_nan() { return Some(0); }
        if f.is_infinite() || f < 0.0 {
            let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
            if let Some(proto) = ctx.get_range_error_prototype() { err.prototype = Some(proto); }
            err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("RangeError")));
            err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("The number is out of range")));
            let ptr = Box::into_raw(Box::new(err)) as usize;
            ctx.runtime_mut().gc_heap_mut().track(ptr);
            ctx.pending_exception = Some(JSValue::new_object(ptr));
            return None;
        }
        f.trunc() as i64
    } else if value.is_undefined() {
        return Some(0);
    } else if value.is_null() {
        return Some(0);
    } else if value.is_bool() {
        if value.get_bool() { 1 } else { 0 }
    } else if value.is_object() {
        if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
            let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
            let to_prim_sym = crate::builtins::symbol::get_or_create_well_known_symbol(ctx, "Symbol.toPrimitive");
            let to_prim_key = crate::runtime::atom::Atom(0x40000000 | to_prim_sym.get_symbol_id());
            let to_primitive = value.as_object().get(to_prim_key);
            if let Some(f) = to_primitive {
                if f.is_function() {
                    let hint = JSValue::new_string(ctx.intern("number"));
                    if let Ok(r) = vm.call_function_with_this(ctx, f, value, &[hint]) {
                        if !r.is_object() {
                            return to_index(ctx, r);
                        }
                        let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
                        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
                        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
                        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Cannot convert object to a number")));
                        let ptr = Box::into_raw(Box::new(err)) as usize;
                        ctx.runtime_mut().gc_heap_mut().track(ptr);
                        ctx.pending_exception = Some(JSValue::new_object(ptr));
                        return None;
                    }
                    return None;
                }
            }
            let value_of = value.as_object().get(ctx.common_atoms.value_of);
            if let Some(f) = value_of {
                if f.is_function() {
                    if let Ok(r) = vm.call_function_with_this(ctx, f, value, &[]) {
                        if !r.is_object() {
                            return to_index(ctx, r);
                        }
                        return None;
                    }
                    return None;
                }
            }
            let to_string = value.as_object().get(ctx.common_atoms.to_string);
            if let Some(f) = to_string {
                if f.is_function() {
                    if let Ok(r) = vm.call_function_with_this(ctx, f, value, &[]) {
                        if r.is_string() {
                            return to_index(ctx, r);
                        }
                        return None;
                    }
                    return None;
                }
            }
        }
        let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Cannot convert object to a number")));
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return None;
    } else if value.is_string() {
        let s = ctx.get_atom_str(value.get_atom());
        if let Ok(n) = s.trim().parse::<i64>() {
            n
        } else {
            return Some(0);
        }
    } else if value.is_bigint() {
        let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Cannot convert a BigInt to a number")));
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return None;
    } else {
        return None;
    };
    if int_val < 0 {
        let mut err = JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_range_error_prototype() { err.prototype = Some(proto); }
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("RangeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("The number is out of range")));
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return None;
    }
    Some(int_val as u64)
}

pub fn init_bigint(ctx: &mut JSContext) {
    let bigint_atom = ctx.intern("BigInt");

    let mut func = JSFunction::new_builtin(bigint_atom, 1);
    func.set_builtin_marker(ctx, "bigint_constructor");
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    let bigint_value = JSValue::new_function(ptr);

    let bigint_func_obj = bigint_value.as_object_mut();
    set_ne(bigint_func_obj, ctx.intern("asIntN"), create_builtin_function(ctx, "bigint_as_int_n"));
    set_ne(bigint_func_obj, ctx.intern("asUintN"), create_builtin_function(ctx, "bigint_as_uint_n"));

    let proto_atom = ctx.intern("BigIntPrototype");
    let mut proto_obj = JSObject::new();
    set_ne(&mut proto_obj, ctx.intern("toString"), create_builtin_function(ctx, "bigint_to_string"));
    let proto_ptr = Box::into_raw(Box::new(proto_obj)) as usize;
    let proto_value = JSValue::new_object(proto_ptr);
    ctx.set_bigint_prototype(proto_ptr);

    let bigint_func = bigint_value.as_object_mut();
    bigint_func.set(ctx.intern("prototype"), proto_value);

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, bigint_atom, bigint_value);
        crate::builtins::global::set_non_enumerable(global_obj, proto_atom, proto_value);
    }
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin("bigint_constructor", HostFunction::ctor("BigInt", 1, bigint_constructor));
    ctx.register_builtin("bigint_as_int_n", HostFunction::new("asIntN", 2, bigint_as_int_n));
    ctx.register_builtin("bigint_as_uint_n", HostFunction::new("asUintN", 2, bigint_as_uint_n));
    ctx.register_builtin("bigint_to_string", HostFunction::method("toString", 1, bigint_to_string));
}

fn bigint_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let value = args.first().copied().unwrap_or(JSValue::new_int(0));
    if let Some(bi) = to_bigint(ctx, value) {
        let mut bigint_obj = JSObject::new_bigint();
        bigint_obj.set_bigint_value(bi);
        let obj_ptr = Box::into_raw(Box::new(bigint_obj)) as usize;
        return JSValue::new_bigint(obj_ptr);
    }
    JSValue::undefined()
}

fn bigint_as_int_n(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_offset = if args.len() >= 3 { 1 } else { 0 };
    let bits_val = args.get(arg_offset).copied().unwrap_or(JSValue::new_int(0));
    let bits = match to_index(ctx, bits_val) {
        Some(b) => b,
        None => return JSValue::undefined(),
    };
    let bigint_val = args.get(arg_offset + 1).copied().unwrap_or(JSValue::undefined());
    let value = match to_bigint(ctx, bigint_val) {
        Some(v) => v,
        None => return JSValue::undefined(),
    };

    if bits == 0 {
        let mut result = JSObject::new_bigint();
        result.set_bigint_value(0i128);
        return JSValue::new_bigint(Box::into_raw(Box::new(result)) as usize);
    }

    let wrapped = if bits >= 128 {
        value
    } else {
        let shift_bits = bits as u32;
        let max = 1i128 << (shift_bits - 1);
        let mask = (1i128 << shift_bits) - 1;
        let masked = value & mask;
        if masked >= max {
            masked - (1i128 << shift_bits)
        } else {
            masked
        }
    };

    let mut result = JSObject::new_bigint();
    result.set_bigint_value(wrapped);
    JSValue::new_bigint(Box::into_raw(Box::new(result)) as usize)
}

fn bigint_as_uint_n(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_offset = if args.len() >= 3 { 1 } else { 0 };
    let bits_val = args.get(arg_offset).copied().unwrap_or(JSValue::new_int(0));
    let bits = match to_index(ctx, bits_val) {
        Some(b) => b,
        None => return JSValue::undefined(),
    };
    let bigint_val = args.get(arg_offset + 1).copied().unwrap_or(JSValue::undefined());
    let value = match to_bigint(ctx, bigint_val) {
        Some(v) => v,
        None => return JSValue::undefined(),
    };

    if bits == 0 {
        let mut result = JSObject::new_bigint();
        result.set_bigint_value(0i128);
        return JSValue::new_bigint(Box::into_raw(Box::new(result)) as usize);
    }

    let wrapped = if bits >= 128 {
        value
    } else {
        let mask = (1u128 << bits as u32) - 1;
        (value as u128 & mask) as i128
    };

    let mut result = JSObject::new_bigint();
    result.set_bigint_value(wrapped);
    JSValue::new_bigint(Box::into_raw(Box::new(result)) as usize)
}

fn bigint_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this_val = args.first().copied().unwrap_or(JSValue::new_int(0));
    let radix = args.get(1).map(|v| v.get_int() as u32).unwrap_or(10);

    let value: i128 = if this_val.is_bigint() {
        this_val.as_object().get_bigint_value()
    } else if this_val.is_int() {
        this_val.get_int() as i128
    } else {
        0i128
    };

    let result = if radix == 16 {
        format!("{:x}", value)
    } else if radix == 8 {
        format!("{:o}", value)
    } else if radix == 2 {
        format!("{:b}", value)
    } else {
        value.to_string()
    };

    JSValue::new_string(ctx.intern(&result))
}
