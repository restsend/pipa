use crate::object::array_obj::JSArrayObject;
use crate::object::object::JSObject;
use crate::runtime::atom::Atom;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

fn unwrap_wrapper_object(ctx: &mut JSContext, obj: &JSObject) -> Option<JSValue> {
    let v = obj.get(ctx.common_atoms.__value__);
    if let Some(val) = v {
        if val.is_bool() {
            return Some(val);
        }
        if val.is_int() || val.is_float() {
            let proto = obj.prototype;
            if let Some(pp) = proto {
                if let Some(ntp) = ctx.get_number_prototype() {
                    if pp == ntp as usize as *mut JSObject {
                        if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
                            let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
                            let obj_ptr = obj as *const _ as usize;
                            let obj_val = JSValue::new_object(obj_ptr);
                            if let Some(value_of_fn) = obj.get(ctx.common_atoms.value_of) {
                                if value_of_fn.is_function() {
                                    return vm.call_function_with_this(ctx, value_of_fn, obj_val, &[]).ok();
                                }
                            }
                        }
                        return Some(val);
                    }
                }
            }
        }
        if val.is_string() {
            let proto = obj.prototype;
            if let Some(pp) = proto {
                if let Some(stp) = ctx.get_string_prototype() {
                    if pp == stp as usize as *mut JSObject {
                        if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
                            let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
                            let obj_ptr = obj as *const _ as usize;
                            let obj_val = JSValue::new_object(obj_ptr);
                            if let Some(to_str_fn) = obj.get(ctx.common_atoms.to_string) {
                                if to_str_fn.is_function() {
                                    return vm.call_function_with_this(ctx, to_str_fn, obj_val, &[]).ok();
                                }
                            }
                        }
                        return Some(val);
                    }
                }
            }
        }
    }
    None
}

pub fn json_parse(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let text = if args.is_empty() {
        "undefined".to_string()
    } else if args[0].is_string() {
        ctx.get_atom_str(args[0].get_atom()).to_string()
    } else if args[0].is_undefined() {
        "undefined".to_string()
    } else if args[0].is_null() {
        "null".to_string()
    } else if args[0].is_bool() {
        if args[0].is_truthy() {
            "true".to_string()
        } else {
            "false".to_string()
        }
    } else if args[0].is_int() {
        format!("{}", args[0].get_int())
    } else if args[0].is_float() {
        let f = args[0].to_number();
        if f == 0.0 { "0".to_string() } else { format!("{}", f) }
    } else if args[0].is_symbol() {
        let mut err =
            crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
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
        return JSValue::undefined();
    } else if args[0].is_bigint() {
        return JSValue::undefined();
    } else if args[0].is_object() {
        let val = &args[0];
        let vm_opt = ctx.get_register_vm_ptr();
        let vm = vm_opt.map(|ptr| unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) });
        if let Some(vm) = vm {
            let obj = val.as_object();
            // Try toString first (hint: string)
            let mut got_primitive: Option<String> = None;
            let to_string_fn = obj.get(ctx.common_atoms.to_string);
            if let Some(f) = to_string_fn {
                if f.is_function() {
                    match vm.call_function_with_this(ctx, f, *val, &[]) {
                        Ok(r) => {
                            if ctx.pending_exception.is_some() {
                                return JSValue::undefined();
                            }
                            got_primitive = if r.is_string() {
                                Some(ctx.get_atom_str(r.get_atom()).to_string())
                            } else if !r.is_object() {
                                Some(crate::builtins::global::jsvalue_to_string(&r, ctx))
                            } else {
                                None
                            };
                        }
                        Err(_) => {
                            if ctx.pending_exception.is_some() {
                                return JSValue::undefined();
                            }
                        }
                    }
                }
            }
            // Try valueOf if toString didn't return a primitive
            if got_primitive.is_none() {
                let value_of_fn = obj.get(ctx.common_atoms.value_of);
                if let Some(f) = value_of_fn {
                    if f.is_function() {
                        match vm.call_function_with_this(ctx, f, *val, &[]) {
                            Ok(r) => {
                                if ctx.pending_exception.is_some() {
                                    return JSValue::undefined();
                                }
                                if !r.is_object() {
                                    let s = crate::builtins::global::jsvalue_to_string(&r, ctx);
                                    got_primitive = Some(s);
                                }
                            }
                            Err(_) => {
                                if ctx.pending_exception.is_some() {
                                    return JSValue::undefined();
                                }
                            }
                        }
                    }
                }
            }
            if let Some(s) = got_primitive {
                match super::json_parser::JsonParser::new(&s).parse_value(ctx) {
                    Ok(v) => return v,
                    Err(_) => {
                        let mut err = crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
                        if let Some(proto) = ctx.get_syntax_error_prototype() { err.prototype = Some(proto); }
                        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("JSON.parse: unexpected character")));
                        let ptr = Box::into_raw(Box::new(err)) as usize;
                        ctx.runtime_mut().gc_heap_mut().track(ptr);
                        ctx.pending_exception = Some(JSValue::new_object(ptr));
                        return JSValue::undefined();
                    }
                }
            }
        }
        // Fallback: throw TypeError when ToPrimitive fails
        let mut err = crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() { err.prototype = Some(proto); }
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Cannot convert object to primitive value")));
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    } else {
        String::new()
    };

    match super::json_parser::JsonParser::new(&text).parse_value(ctx) {
        Ok(root_val) => {
            if let Some(reviver) = args.get(1) {
                if reviver.is_function() {
                    // Create wrapper: { '': root_val } with Object.prototype
                    let mut wrapper = crate::object::object::JSObject::new();
                    if let Some(obj_proto) = ctx.get_object_prototype() {
                        wrapper.prototype = Some(obj_proto);
                    }
                    wrapper.set(ctx.common_atoms.empty, root_val);
                    let wptr = Box::into_raw(Box::new(wrapper)) as usize;
                    ctx.runtime_mut().gc_heap_mut().track(wptr);
                    let wrapper_val = JSValue::new_object(wptr);
                    // Walk the wrapper with reviver
                    return json_walk(ctx, *reviver, wrapper_val, ctx.common_atoms.empty);
                }
            }
            root_val
        }
        Err(_) => {
            let mut err = crate::object::object::JSObject::new_typed(
                crate::object::object::ObjectType::Error,
            );
            if let Some(proto) = ctx.get_syntax_error_prototype() {
                err.prototype = Some(proto);
            }
            err.set(
                ctx.common_atoms.message,
                JSValue::new_string(ctx.intern("JSON.parse: unexpected character")),
            );
            let ptr = Box::into_raw(Box::new(err)) as usize;
            ctx.runtime_mut().gc_heap_mut().track(ptr);
            ctx.pending_exception = Some(JSValue::new_object(ptr));
            JSValue::undefined()
        }
    }
}

pub fn json_stringify(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }

    let value = &args[0];
    let replacer = args.get(1);
    let space = args.get(2);

    if value.is_function() || value.is_symbol() || value.is_undefined() {
        return JSValue::undefined();
    }

    if let Some(json_str) = jsvalue_to_json_with_options(ctx, value, replacer, space, &mut Vec::new()) {
        JSValue::new_string(ctx.intern(&json_str))
    } else {
        JSValue::undefined()
    }
}

fn apply_to_json(ctx: &mut JSContext, val: &mut JSValue, key: crate::runtime::atom::Atom) {
    let to_json_fn = get_to_json(ctx, *val);
    if let Some(f) = to_json_fn {
        if f.is_function() {
            if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
                let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
                let key_val = JSValue::new_string(key);
                let args = vec![key_val];
                if let Ok(r) = vm.call_function_with_this(ctx, f, *val, &args) {
                    *val = r;
                }
            }
        }
    }
}

fn get_to_json(ctx: &mut JSContext, val: JSValue) -> Option<JSValue> {
    if val.is_object() {
        return val.as_object().get(ctx.intern("toJSON"));
    }
    if val.is_bigint() {
        if let Some(bigint_proto) = ctx.get_bigint_prototype() {
            let proto_obj = unsafe { &*bigint_proto };
            return proto_obj.get(ctx.intern("toJSON"));
        }
    }
    None
}

fn jsvalue_to_json_with_options(
    ctx: &mut JSContext,
    value: &JSValue,
    replacer: Option<&JSValue>,
    space: Option<&JSValue>,
    seen: &mut Vec<usize>,
) -> Option<String> {
    let mut val = *value;
    // Apply toJSON
    apply_to_json(ctx, &mut val, ctx.common_atoms.empty);
    // Apply replacer
    if let Some(repl) = replacer {
        if repl.is_function() {
            let vm_ptr = ctx.get_register_vm_ptr();
            if let Some(ptr) = vm_ptr {
                let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                let args = vec![JSValue::new_string(ctx.intern("")), val];
                // Create wrapper object as `this` for the replacer (spec step 9-10)
                let mut wrapper_obj = crate::object::object::JSObject::new();
                if let Some(proto_ptr) = ctx.get_object_prototype() {
                    wrapper_obj.prototype = Some(proto_ptr);
                }
                wrapper_obj.set(ctx.common_atoms.empty, *value);
                let wrapper_ptr = Box::into_raw(Box::new(wrapper_obj)) as usize;
                ctx.runtime_mut().gc_heap_mut().track(wrapper_ptr);
                let wrapper_val = JSValue::new_object(wrapper_ptr);
                if let Ok(result) = vm.call_function_with_this(ctx, *repl, wrapper_val, &args) {
                    val = result;
                }
            }
        }
    }

    let base_json = jsvalue_to_json_internal(ctx, &val, replacer, seen, ctx.common_atoms.empty)?;

    if let Some(sp) = space {
        let indent = get_indent_string_with_ctx(ctx, sp);
        if !indent.is_empty() {
            return Some(format_json(&base_json, &indent));
        }
    }

    Some(base_json)
}

fn json_walk(ctx: &mut JSContext, reviver: JSValue, holder: JSValue, key: crate::runtime::atom::Atom) -> JSValue {
    if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
        let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
        let val = if holder.is_object() {
            holder.as_object().get(key)
        } else {
            None
        };
        let Some(val) = val else { return JSValue::undefined() };

        if val.is_object() {
            let obj_ptr = val.get_ptr();
            if val.as_object().is_array() {
                let len_atom = ctx.common_atoms.length;
                let len = val.as_object().get(len_atom).map(|v| v.get_int() as usize).unwrap_or(0);
                for i in 0..len {
                    let i_atom = ctx.int_atom_mut(i);
                    let new_val = json_walk(ctx, reviver, val, i_atom);
                    if ctx.pending_exception.is_some() { return JSValue::undefined(); }
                    let obj_mut = unsafe { &mut *(obj_ptr as *mut crate::object::object::JSObject) };
                    if new_val.is_undefined() {
                        obj_mut.delete(i_atom);
                    } else {
                        obj_mut.set(i_atom, new_val);
                    }
                }
            } else {
                let mut props: Vec<(Atom, JSValue)> = val.as_object().own_properties();
                // Sort: numeric keys first (ascending), then string keys in insertion order
                props.sort_by(|a, b| {
                    let a_str = ctx.get_atom_str(a.0);
                    let b_str = ctx.get_atom_str(b.0);
                    let a_int: Option<i64> = a_str.parse().ok();
                    let b_int: Option<i64> = b_str.parse().ok();
                    match (a_int, b_int) {
                        (Some(ai), Some(bi)) => ai.cmp(&bi),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
                for (prop_atom, _) in props {
                    let new_val = json_walk(ctx, reviver, val, prop_atom);
                    if ctx.pending_exception.is_some() { return JSValue::undefined(); }
                    let obj_mut = unsafe { &mut *(obj_ptr as *mut crate::object::object::JSObject) };
                    if new_val.is_undefined() {
                        obj_mut.delete(prop_atom);
                    } else {
                        obj_mut.set(prop_atom, new_val);
                    }
                }
            }
        }

        let key_str = if key == ctx.common_atoms.empty {
            String::new()
        } else {
            ctx.get_atom_str(key).to_string()
        };
        let val2 = if holder.is_object() {
            holder.as_object().get(key)
        } else {
            None
        };
        let val = val2.unwrap_or(JSValue::undefined());
        // Create context object for reviver (3rd argument)
        let context_obj = crate::object::object::JSObject::new();
        let cptr = Box::into_raw(Box::new(context_obj)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(cptr);
        let context_val = JSValue::new_object(cptr);
        
        let args = vec![JSValue::new_string(ctx.intern(&key_str)), val, context_val];
        match vm.call_function_with_this(ctx, reviver, holder, &args) {
            Ok(r) => {
                if ctx.pending_exception.is_some() {
                    return JSValue::undefined();
                }
                r
            }
            Err(_) => {
                if ctx.pending_exception.is_some() {
                    return JSValue::undefined();
                }
                JSValue::undefined()
            }
        }
    } else {
        JSValue::undefined()
    }
}

fn get_indent_string_with_ctx(ctx: &mut JSContext, space: &JSValue) -> String {
    let effective_space = if space.is_object() {
        let obj = space.as_object();
        let inner = obj.get(ctx.common_atoms.__value__);
        // Only call valueOf/toString if the object has [[NumberData]] or [[StringData]]
        if let Some(v) = inner {
            if v.is_string() {
                // String wrapper: use ToString (toString first, then valueOf)
                if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
                    let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
                    let to_str_fn = obj.get(ctx.common_atoms.to_string);
                    if let Some(f) = to_str_fn {
                        if f.is_function() {
                            if let Ok(r) = vm.call_function_with_this(ctx, f, *space, &[]) {
                                if r.is_string() { return get_indent_string_with_ctx(ctx, &r); }
                            }
                        }
                    }
                    let val_fn = obj.get(ctx.common_atoms.value_of);
                    if let Some(f) = val_fn {
                        if f.is_function() {
                            if let Ok(r) = vm.call_function_with_this(ctx, f, *space, &[]) {
                                if r.is_string() { return get_indent_string_with_ctx(ctx, &r); }
                            }
                        }
                    }
                }
            } else if v.is_int() || v.is_float() {
                // Number wrapper: use ToNumber (valueOf first, then toString)
                if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
                    let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
                    let val_fn = obj.get(ctx.common_atoms.value_of);
                    if let Some(f) = val_fn {
                        if f.is_function() {
                            if let Ok(r) = vm.call_function_with_this(ctx, f, *space, &[]) {
                                if !r.is_object() { return get_indent_string_with_ctx(ctx, &r); }
                            }
                        }
                    }
                    let to_str_fn = obj.get(ctx.common_atoms.to_string);
                    if let Some(f) = to_str_fn {
                        if f.is_function() {
                            if let Ok(r) = vm.call_function_with_this(ctx, f, *space, &[]) {
                                if !r.is_object() { return get_indent_string_with_ctx(ctx, &r); }
                            }
                        }
                    }
                }
            }
            // Boolean and other wrappers: ignore (space = undefined per spec)
            return String::new();
        }
        // For generic objects or if conversion fails, space is ignored (no formatting)
        return String::new();
    } else {
        *space
    };
    if effective_space.is_int() {
        let n = effective_space.get_int().min(10).max(0) as usize;
        " ".repeat(n)
    } else if effective_space.is_float() {
        let f = effective_space.get_float();
        let n = if f.is_nan() { 0 } else { f.trunc() as i64 };
        let n = n.min(10).max(0) as usize;
        " ".repeat(n)
    } else if effective_space.is_string() {
        let atom = effective_space.get_atom();
        let s_str = ctx.get_atom_str(atom).to_string();
        s_str.chars().take(10).collect()
    } else {
        String::new()
    }
}

fn format_json(json: &str, indent: &str) -> String {
    let mut result = String::new();
    let mut level = 0;
    let mut in_string = false;
    let mut escape = false;
    let chars: Vec<char> = json.chars().collect();

    for i in 0..chars.len() {
        let c = chars[i];

        if escape {
            escape = false;
            result.push(c);
            continue;
        }

        if c == '\\' && in_string {
            escape = true;
            result.push(c);
            continue;
        }

        if c == '"' {
            in_string = !in_string;
            result.push(c);
            continue;
        }

        if in_string {
            result.push(c);
            continue;
        }

        match c {
            '{' | '[' => {
                result.push(c);
                level += 1;
                result.push('\n');
                result.push_str(&indent.repeat(level));
            }
            '}' | ']' => {
                level = level.saturating_sub(1);
                result.push('\n');
                result.push_str(&indent.repeat(level));
                result.push(c);
            }
            ',' => {
                result.push(c);
                result.push('\n');
                result.push_str(&indent.repeat(level));
            }
            ':' => {
                result.push(c);
                result.push(' ');
            }
            _ => {
                result.push(c);
            }
        }
    }

    result
}

fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0C' => out.push_str("\\f"),
            c if c.is_ascii_control() => {
                out.push_str(&format!("\\u{:04x}", c as u8));
            }
            c => out.push(c),
        }
    }
    out
}

fn jsvalue_to_json_internal(
    ctx: &mut JSContext,
    value: &JSValue,
    replacer: Option<&JSValue>,
    seen: &mut Vec<usize>,
    key: crate::runtime::atom::Atom,
) -> Option<String> {
    if value.is_null() {
        return Some("null".to_string());
    }
    if value.is_bool() {
        return Some(value.get_bool().to_string());
    }
    if value.is_int() {
        return Some(value.get_int().to_string());
    }
    if value.is_float() {
        let f = value.get_float();
        if f.is_nan() || f.is_infinite() {
            return Some("null".to_string());
        }
        if f == 0.0 {
            return Some("0".to_string());
        }
        return Some(f.to_string());
    }
    if value.is_string() {
        let atom = value.get_atom();
        let s = ctx.get_atom_str(atom);

        return Some(format!("\"{}\"", escape_json_string(&s)));
    }
    if value.is_undefined() || value.is_function() || value.is_symbol() {
        return None;
    }

    if value.is_bigint() {
        let mut err =
            crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        err.set(
            ctx.common_atoms.message,
            JSValue::new_string(ctx.intern("BigInt value cannot be serialized in JSON")),
        );
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return Some(String::new());
    }

    if value.is_object() {
        let obj = value.as_object();

        let unwrapped = unwrap_wrapper_object(ctx, obj);
        if unwrapped.is_some() {
            return jsvalue_to_json_internal(ctx, &unwrapped.unwrap(), replacer, seen, key);
        }

        let ptr = value.get_ptr() as usize;

        if seen.contains(&ptr) {
            let mut err = crate::object::object::JSObject::new_typed(
                crate::object::object::ObjectType::Error,
            );
            if let Some(proto) = ctx.get_type_error_prototype() {
                err.prototype = Some(proto);
            }
            err.set(
                ctx.common_atoms.message,
                JSValue::new_string(ctx.intern("Converting circular structure to JSON")),
            );
            let eptr = Box::into_raw(Box::new(err)) as usize;
            ctx.runtime_mut().gc_heap_mut().track(eptr);
            ctx.pending_exception = Some(JSValue::new_object(eptr));
            return Some(String::new());
        }
        seen.push(ptr);

        let obj = value.as_object();

        if obj.is_array() {
            let result = array_to_json(ctx, obj, replacer, seen);
            seen.pop();
            return Some(result);
        } else {
            let result = object_to_json(ctx, obj, replacer, seen);
            seen.pop();
            return Some(result);
        }
    }

    Some("null".to_string())
}

fn array_to_json(
    ctx: &mut JSContext,
    arr: &JSObject,
    replacer: Option<&JSValue>,
    seen: &mut Vec<usize>,
) -> String {
    let len_atom = ctx.common_atoms.length;
    let len = arr.get(len_atom).map(|v| v.get_int() as usize).unwrap_or(0);

    let mut elements = Vec::new();
    let arr_ptr = arr as *const JSObject as usize;
    let is_jsarray = arr.is_dense_array();
    for i in 0..len {
        let i_key = ctx.int_atom_mut(i);
        let val = if is_jsarray {
            unsafe { &*(arr_ptr as *const JSArrayObject) }.get(i)
        } else if let Some(v) = arr.get_indexed(i) {
            Some(v)
        } else {
            arr.get(i_key)
        };
        if let Some(val) = val {
            let mut val = val;
            // Apply toJSON
            apply_to_json(ctx, &mut val, i_key);
            let processed = if let Some(repl) = replacer {
                if repl.is_function() {
                    let vm_ptr = ctx.get_register_vm_ptr();
                    if let Some(ptr) = vm_ptr {
                        let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                        let args = vec![JSValue::new_string(ctx.intern(&i.to_string())), val];
                        let arr_val = JSValue::new_object(arr as *const _ as usize);
                        if let Ok(result) = vm.call_function_with_this(ctx, *repl, arr_val, &args) {
                            result
                        } else {
                            val
                        }
                    } else {
                        val
                    }
                } else {
                    val
                }
            } else {
                val
            };
            elements.push(jsvalue_to_json_internal(ctx, &processed, replacer, seen, i_key).unwrap_or_else(|| "null".to_string()));
        } else {
            elements.push("null".to_string());
        }
    }
    format!("[{}]", elements.join(","))
}

fn object_to_json(
    ctx: &mut JSContext,
    obj: &JSObject,
    replacer: Option<&JSValue>,
    seen: &mut Vec<usize>,
) -> String {
    let mut pairs = Vec::new();

    let filter_keys: Option<Vec<crate::runtime::atom::Atom>> = if let Some(repl) = replacer {
        if repl.is_object() {
            let repl_obj = repl.as_object();
            if repl_obj.is_array() {
                let len_atom = ctx.common_atoms.length;
                let len = repl_obj
                    .get(len_atom)
                    .map(|v| v.get_int() as usize)
                    .unwrap_or(0);
                let mut keys = Vec::new();
                let repl_ptr = repl_obj as *const JSObject as usize;
                let is_jsarray = repl_obj.is_dense_array();
                for i in 0..len {
                    let val = if is_jsarray {
                        unsafe { &*(repl_ptr as *const JSArrayObject) }.get(i)
                    } else if let Some(v) = repl_obj.get_indexed(i) {
                        Some(v)
                    } else {
                        let key = ctx.int_atom_mut(i);
                        repl_obj.get(key)
                    };
                    if let Some(k) = val {
                        if k.is_string() {
                            keys.push(k.get_atom());
                        } else if k.is_int() {
                            let s = k.get_int().to_string();
                            if !s.is_empty() {
                                keys.push(ctx.intern(&s));
                            }
                        } else if k.is_float() {
                            let f = k.get_float();
                            if f == 0.0 {
                                keys.push(ctx.intern("0"));
                            } else if f.is_infinite() {
                                if f.is_sign_positive() {
                                    keys.push(ctx.intern("Infinity"));
                                } else {
                                    keys.push(ctx.intern("-Infinity"));
                                }
                            } else if f.is_nan() {
                                keys.push(ctx.intern("NaN"));
                            } else {
                                keys.push(ctx.intern(&f.to_string()));
                            }
                        } else if k.is_object() {
                            if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
                                let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
                                let obj = k.as_object();
                                if let Some(to_str_fn) = obj.get(ctx.common_atoms.to_string) {
                                    if to_str_fn.is_function() {
                                        if let Ok(r) = vm.call_function_with_this(ctx, to_str_fn, k, &[]) {
                                            if r.is_string() {
                                                keys.push(r.get_atom());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Some(keys)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let mut properties: Vec<(crate::runtime::atom::Atom, JSValue)> = obj.own_properties();

    if let Some(ref filter) = filter_keys {
        // When a replacer array is specified, iterate in array order
        let mut filtered = Vec::new();
        for k in filter.iter() {
            if let Some(idx) = properties.iter().position(|(pk, _)| pk == k) {
                filtered.push(properties.swap_remove(idx));
            }
        }
        properties = filtered;
    } else {
        // Sort properties: integer array indices first (ascending), then string keys in insertion order
        properties.sort_by(|a, b| {
            let a_str = ctx.get_atom_str(a.0);
            let b_str = ctx.get_atom_str(b.0);
            let a_int: Option<i64> = a_str.parse().ok().filter(|n| *n >= 0);
            let b_int: Option<i64> = b_str.parse().ok().filter(|n| *n >= 0);
            match (a_int, b_int) {
                (Some(ai), Some(bi)) => ai.cmp(&bi),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
    }

    for (key, _prop_val) in properties {
        // Skip Symbol-keyed properties
        if key.0 & 0x40000000 != 0 {
            continue;
        }
        // Get the live value from the object
        let val = if let Some(v) = obj.get_own_value(key) {
            if v.is_function() {
                // Accessor property: call the getter
                if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
                    let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
                    let obj_ptr = obj as *const _ as usize;
                    let obj_val = JSValue::new_object(obj_ptr);
                    match vm.call_function_with_this(ctx, v, obj_val, &[]) {
                        Ok(r) => r,
                        Err(_) => continue,
                    }
                } else {
                    continue;
                }
            } else {
                v
            }
        } else {
            JSValue::undefined()
        };

        let mut val = val;
        // Apply toJSON
        apply_to_json(ctx, &mut val, key);

        let key_str = ctx.get_atom_str(key).to_string();

        let processed = if let Some(repl) = replacer {
            if repl.is_function() {
                let vm_ptr = ctx.get_register_vm_ptr();
                if let Some(ptr) = vm_ptr {
                    let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                    let args = vec![JSValue::new_string(ctx.intern(&key_str)), val];
                    let obj_ptr = obj as *const _ as usize;
                    let obj_val = JSValue::new_object(obj_ptr);
                    if let Ok(result) = vm.call_function_with_this(ctx, *repl, obj_val, &args) {
                        result
                    } else {
                        val
                    }
                } else {
                    val
                }
            } else {
                val
            }
        } else {
            val
        };

        if processed.is_undefined() || processed.is_function() || processed.is_symbol() {
            continue;
        }

        if let Some(value_str) = jsvalue_to_json_internal(ctx, &processed, replacer, seen, key) {
            let escaped_key = escape_json_string(&key_str);
            pairs.push(format!("\"{}\":{}", escaped_key, value_str));
        }
    }

    format!("{{{}}}", pairs.join(","))
}
