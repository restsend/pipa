use crate::runtime::context::JSContext;
use crate::value::JSValue;

pub fn global_parseint(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    }

    let input_val = &args[0];
    let s = if input_val.is_string() {
        ctx.get_atom_str(input_val.get_atom()).to_string()
    } else if input_val.is_int() {
        input_val.get_int().to_string()
    } else if input_val.is_float() {
        let f = input_val.get_float();
        if f.is_nan() || f.is_infinite() {
            return JSValue::new_float(f64::NAN);
        }
        let truncated = f.trunc();
        if truncated == 0.0 {
            return JSValue::new_int(0);
        }
        format!("{}", truncated as i64)
    } else if input_val.is_bool() {
        if input_val.get_bool() {
            "true".to_string()
        } else {
            "false".to_string()
        }
    } else if input_val.is_null() {
        "null".to_string()
    } else if input_val.is_undefined() {
        "undefined".to_string()
    } else if input_val.is_object() {
        let obj = input_val.as_object();
        let mut prim: Option<Option<String>> = None;
        if let Some(to_str) = obj.get(ctx.intern("toString")) {
            if to_str.is_function() {
                if let Some(ptr) = ctx.get_register_vm_ptr() {
                    let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                    match vm.call_function_with_this(ctx, to_str, *input_val, &[]) {
                        Ok(result) if result.is_string() => {
                            prim = Some(Some(ctx.get_atom_str(result.get_atom()).to_string()));
                        }
                        Ok(result) if result.is_int() => {
                            prim = Some(Some(result.get_int().to_string()));
                        }
                        Ok(result) if result.is_float() => {
                            prim = Some(Some(result.get_float().to_string()));
                        }
                        Ok(_) => {
                            prim = Some(None);
                        }
                        _ => {}
                    }
                }
            }
        }
        if let Some(Some(_)) = prim {
        } else {
            if let Some(val_of) = obj.get(ctx.intern("valueOf")) {
                if val_of.is_function() {
                    if let Some(ptr) = ctx.get_register_vm_ptr() {
                        let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                        match vm.call_function_with_this(ctx, val_of, *input_val, &[]) {
                            Ok(result) if result.is_string() => {
                                prim = Some(Some(ctx.get_atom_str(result.get_atom()).to_string()));
                            }
                            Ok(result) if result.is_int() => {
                                prim = Some(Some(result.get_int().to_string()));
                            }
                            Ok(result) if result.is_float() => {
                                prim = Some(Some(result.get_float().to_string()));
                            }
                            Ok(_) => {
                                prim = Some(None);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        match prim {
            Some(Some(s)) => s,
            _ => {
                return crate::builtins::global::throw_type_error(
                    ctx,
                    "Cannot convert object to primitive value",
                );
            }
        }
    } else {
        return JSValue::new_float(f64::NAN);
    };

    let mut input = s.trim_start();
    let mut sign = 1f64;
    if let Some(rest) = input.strip_prefix('-') {
        sign = -1.0;
        input = rest;
    } else if let Some(rest) = input.strip_prefix('+') {
        input = rest;
    }

    let radix_arg = args.get(1);
    let mut radix = if let Some(ra) = radix_arg {
        let n = if ra.is_int() {
            ra.get_int() as f64
        } else if ra.is_float() {
            ra.get_float()
        } else if ra.is_bool() {
            if ra.get_bool() { 1.0 } else { 0.0 }
        } else if ra.is_null() {
            0.0
        } else if ra.is_undefined() {
            0.0
        } else if ra.is_string() {
            let s = ctx.get_atom_str(ra.get_atom());
            if s.trim().is_empty() {
                0.0
            } else {
                match s.trim().parse::<f64>() {
                    Ok(v) if v.is_nan() => 0.0,
                    Ok(v) => v,
                    Err(_) => 0.0,
                }
            }
        } else if ra.is_object() {
            let obj = ra.as_object();
            let mut radix_num = None;
            if let Some(val_of) = obj.get(ctx.intern("valueOf")) {
                if val_of.is_function() {
                    if let Some(ptr) = ctx.get_register_vm_ptr() {
                        let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                        match vm.call_function_with_this(ctx, val_of, *ra, &[]) {
                            Ok(result) if result.is_int() => {
                                radix_num = Some(result.get_int() as f64)
                            }
                            Ok(result) if result.is_float() => radix_num = Some(result.get_float()),
                            Ok(result) if result.is_bool() => {
                                radix_num = Some(if result.get_bool() { 1.0 } else { 0.0 });
                            }
                            _ => {}
                        }
                        if ctx.pending_exception.is_some() {
                            return JSValue::undefined();
                        }
                    }
                }
            }
            if radix_num.is_none() {
                if let Some(to_str) = obj.get(ctx.intern("toString")) {
                    if to_str.is_function() {
                        if let Some(ptr) = ctx.get_register_vm_ptr() {
                            let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                            match vm.call_function_with_this(ctx, to_str, *ra, &[]) {
                                Ok(result) if result.is_string() => {
                                    let s = ctx.get_atom_str(result.get_atom());
                                    if let Ok(v) = s.trim().parse::<f64>() {
                                        radix_num = Some(v);
                                    }
                                }
                                Ok(result) if result.is_int() => {
                                    radix_num = Some(result.get_int() as f64)
                                }
                                Ok(result) if result.is_float() => {
                                    radix_num = Some(result.get_float())
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            if ctx.pending_exception.is_some() {
                return JSValue::undefined();
            }
            match radix_num {
                Some(v) => v,
                None => {
                    return crate::builtins::global::throw_type_error(
                        ctx,
                        "Cannot convert object to primitive value",
                    );
                }
            }
        } else {
            0.0
        };
        if n.is_nan() || n.is_infinite() {
            0
        } else {
            let r = (n.trunc() as i64 & 0xFFFFFFFF) as i32;
            if r == 1 {
                return JSValue::new_float(f64::NAN);
            }
            r
        }
    } else {
        0
    };

    if radix != 0 && !(2..=36).contains(&radix) {
        return JSValue::new_float(f64::NAN);
    }

    if radix == 0 {
        if input.starts_with("0x") || input.starts_with("0X") {
            radix = 16;
            input = &input[2..];
        } else {
            radix = 10;
        }
    } else if radix == 16 && (input.starts_with("0x") || input.starts_with("0X")) {
        input = &input[2..];
    }

    let mut result: f64 = 0.0;
    let mut has_digits = false;
    for ch in input.chars() {
        if let Some(d) = ch.to_digit(radix as u32) {
            result = result * (radix as f64) + (d as f64);
            has_digits = true;
        } else {
            break;
        }
    }

    if !has_digits {
        return JSValue::new_float(f64::NAN);
    }

    let final_val = sign * result;
    if final_val >= -(1i64 << 47) as f64
        && final_val < (1i64 << 47) as f64
        && final_val == final_val.trunc()
    {
        JSValue::new_int(final_val as i64)
    } else {
        JSValue::new_float(final_val)
    }
}

pub fn global_parsefloat(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    }

    let input = &args[0];

    if input.is_symbol() {
        if let Some(ptr) = ctx.get_register_vm_ptr() {
            let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
            let msg_atom = ctx.intern("TypeError: Cannot convert Symbol to string");
            vm.pending_throw = Some(JSValue::new_string(msg_atom));
        }
        return JSValue::undefined();
    }

    if input.is_float() {
        let f = input.get_float();
        if f == 0.0 {
            return JSValue::new_int(0);
        }
        return *input;
    }
    if input.is_int() {
        return *input;
    }

    let s = if input.is_string() {
        ctx.get_atom_str(input.get_atom()).to_string()
    } else if input.is_object() {
        let obj = input.as_object();
        let mut result_str: Option<String> = None;
        if let Some(to_str) = obj.get(ctx.intern("toString")) {
            if to_str.is_function() {
                if let Some(ptr) = ctx.get_register_vm_ptr() {
                    let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                    match vm.call_function_with_this(ctx, to_str, *input, &[]) {
                        Ok(result) => {
                            if result.is_string() {
                                result_str = Some(ctx.get_atom_str(result.get_atom()).to_string());
                            } else if result.is_int() {
                                result_str = Some(result.get_int().to_string());
                            } else if result.is_float() {
                                result_str = Some(result.get_float().to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        if result_str.is_none() {
            if let Some(value_of) = obj.get(ctx.intern("valueOf")) {
                if value_of.is_function() {
                    if let Some(ptr) = ctx.get_register_vm_ptr() {
                        let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                        match vm.call_function_with_this(ctx, value_of, *input, &[]) {
                            Ok(result) => {
                                if result.is_string() {
                                    result_str =
                                        Some(ctx.get_atom_str(result.get_atom()).to_string());
                                } else if result.is_int() {
                                    result_str = Some(result.get_int().to_string());
                                } else if result.is_float() {
                                    result_str = Some(result.get_float().to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        match result_str {
            Some(s) => s,
            None => {
                if let Some(ptr) = ctx.get_register_vm_ptr() {
                    let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                    let mut err = crate::object::object::JSObject::new_typed(
                        crate::object::object::ObjectType::Error,
                    );
                    err.set(
                        ctx.common_atoms.message,
                        JSValue::new_string(ctx.intern("Cannot convert object to primitive value")),
                    );
                    err.set(
                        ctx.common_atoms.name,
                        JSValue::new_string(ctx.intern("TypeError")),
                    );
                    if let Some(proto) = ctx.get_type_error_prototype() {
                        err.prototype = Some(proto);
                    }
                    let err_ptr = Box::into_raw(Box::new(err)) as usize;
                    ctx.runtime_mut().gc_heap_mut().track(err_ptr);
                    vm.pending_throw = Some(JSValue::new_object(err_ptr));
                }
                return JSValue::undefined();
            }
        }
    } else {
        return JSValue::new_float(f64::NAN);
    };

    let trimmed = s.trim_start();

    if trimmed.is_empty() {
        return JSValue::new_float(f64::NAN);
    }

    let bytes = trimmed.as_bytes();
    let mut pos = 0;

    if pos < bytes.len() && (bytes[pos] == b'+' || bytes[pos] == b'-') {
        pos += 1;
    }

    let start_num = pos;

    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        pos += 1;
    }

    if pos < bytes.len() && bytes[pos] == b'.' {
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
    }

    if pos < bytes.len() && (bytes[pos] == b'e' || bytes[pos] == b'E') {
        let e_pos = pos;
        pos += 1;
        if pos < bytes.len() && (bytes[pos] == b'+' || bytes[pos] == b'-') {
            pos += 1;
        }
        if pos < bytes.len() && bytes[pos].is_ascii_digit() {
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
        } else {
            pos = e_pos;
        }
    }

    if pos == start_num {
        let rest = trimmed;
        if rest.starts_with("Infinity") || rest.starts_with("+Infinity") {
            return JSValue::new_float(f64::INFINITY);
        }
        if rest.starts_with("-Infinity") {
            return JSValue::new_float(f64::NEG_INFINITY);
        }
        return JSValue::new_float(f64::NAN);
    }

    let num_str = &trimmed[..pos];

    if num_str == "Infinity" || num_str == "+Infinity" {
        return JSValue::new_float(f64::INFINITY);
    }
    if num_str == "-Infinity" {
        return JSValue::new_float(f64::NEG_INFINITY);
    }

    match num_str.parse::<f64>() {
        Ok(v) if v == 0.0 => JSValue::new_int(0),
        Ok(v) => JSValue::new_float(v),
        Err(_) => JSValue::new_float(f64::NAN),
    }
}
