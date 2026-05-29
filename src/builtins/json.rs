use crate::object::array_obj::JSArrayObject;
use crate::object::object::JSObject;
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
                unsafe {
                    if let Some(ntp) = ctx.get_number_prototype() {
                        if pp == ntp as usize as *mut JSObject {
                            return Some(val);
                        }
                    }
                }
            }
        }
        if val.is_string() {
            let proto = obj.prototype;
            if let Some(pp) = proto {
                unsafe {
                    if let Some(stp) = ctx.get_string_prototype() {
                        if pp == stp as usize as *mut JSObject {
                            return Some(val);
                        }
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
        if args[0].is_truthy() { "true".to_string() } else { "false".to_string() }
    } else if args[0].is_int() {
        format!("{}", args[0].get_int())
    } else if args[0].is_float() {
        format!("{}", args[0].to_number())
    } else if args[0].is_symbol() {
        let mut err = crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Cannot convert a Symbol value to a string")));
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    } else {
        String::new()
    };

    match super::json_parser::JsonParser::new(&text).parse_value(ctx) {
        Ok(v) => v,
        Err(_) => {
            let mut err = crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
            if let Some(proto) = ctx.get_syntax_error_prototype() {
                err.prototype = Some(proto);
            }
            err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("JSON.parse: unexpected character")));
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

    let json_str = jsvalue_to_json_with_options(ctx, value, replacer, space, &mut Vec::new());

    JSValue::new_string(ctx.intern(&json_str))
}

fn jsvalue_to_json_with_options(
    ctx: &mut JSContext,
    value: &JSValue,
    replacer: Option<&JSValue>,
    space: Option<&JSValue>,
    seen: &mut Vec<usize>,
) -> String {
    let processed_value = if let Some(repl) = replacer {
        if repl.is_function() {
            let vm_ptr = ctx.get_register_vm_ptr();
            if let Some(ptr) = vm_ptr {
                let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                let args = vec![JSValue::new_string(ctx.intern("")), *value];
                if let Ok(result) = vm.call_function(ctx, *repl, &args) {
                    result
                } else {
                    *value
                }
            } else {
                *value
            }
        } else {
            *value
        }
    } else {
        *value
    };

    let base_json = jsvalue_to_json_internal(ctx, &processed_value, replacer, seen);

    if let Some(sp) = space {
        let indent = get_indent_string_with_ctx(ctx, sp);
        if !indent.is_empty() {
            return format_json(&base_json, &indent);
        }
    }

    base_json
}

fn get_indent_string_with_ctx(ctx: &mut JSContext, space: &JSValue) -> String {
    if space.is_int() {
        let n = space.get_int().min(10).max(0) as usize;
        " ".repeat(n)
    } else if space.is_string() {
        let atom = space.get_atom();
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

fn jsvalue_to_json_internal(
    ctx: &mut JSContext,
    value: &JSValue,
    replacer: Option<&JSValue>,
    seen: &mut Vec<usize>,
) -> String {
    if value.is_null() {
        return "null".to_string();
    }
    if value.is_bool() {
        return value.get_bool().to_string();
    }
    if value.is_int() {
        return value.get_int().to_string();
    }
    if value.is_float() {
        let f = value.get_float();
        if f.is_nan() || f.is_infinite() {
            return "null".to_string();
        }
        return f.to_string();
    }
    if value.is_string() {
        let atom = value.get_atom();
        let s = ctx.get_atom_str(atom);

        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        return format!("\"{}\"", escaped);
    }
    if value.is_undefined() || value.is_function() || value.is_symbol() {
        return "null".to_string();
    }

    if value.is_bigint() {
        let mut err = crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("BigInt value cannot be serialized in JSON")));
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return String::new();
    }

    if value.is_object() {
        let obj = value.as_object();

        let unwrapped = unwrap_wrapper_object(ctx, obj);
        if unwrapped.is_some() {
            return jsvalue_to_json_internal(ctx, &unwrapped.unwrap(), replacer, seen);
        }

        let ptr = value.get_ptr() as usize;

        if seen.contains(&ptr) {
            let mut err = crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
            if let Some(proto) = ctx.get_type_error_prototype() {
                err.prototype = Some(proto);
            }
            err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Converting circular structure to JSON")));
            let eptr = Box::into_raw(Box::new(err)) as usize;
            ctx.runtime_mut().gc_heap_mut().track(eptr);
            ctx.pending_exception = Some(JSValue::new_object(eptr));
            return String::new();
        }
        seen.push(ptr);

        let obj = value.as_object();

        if obj.is_array() {
            let result = array_to_json(ctx, obj, replacer, seen);
            seen.pop();
            return result;
        } else {
            let result = object_to_json(ctx, obj, replacer, seen);
            seen.pop();
            return result;
        }
    }

    "null".to_string()
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
        let val = if is_jsarray {
            unsafe { &*(arr_ptr as *const JSArrayObject) }.get(i)
        } else if let Some(v) = arr.get_indexed(i) {
            Some(v)
        } else {
            let key = ctx.int_atom_mut(i);
            arr.get(key)
        };
        if let Some(val) = val {
            let processed = if let Some(repl) = replacer {
                if repl.is_function() {
                    let vm_ptr = ctx.get_register_vm_ptr();
                    if let Some(ptr) = vm_ptr {
                        let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                        let args = vec![JSValue::new_string(ctx.intern(&i.to_string())), val];
                        if let Ok(result) = vm.call_function(ctx, *repl, &args) {
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
            elements.push(jsvalue_to_json_internal(ctx, &processed, replacer, seen));
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

    let properties: Vec<(crate::runtime::atom::Atom, JSValue)> = obj.own_properties();

    for (key, val) in properties {
        if let Some(ref filter) = filter_keys {
            if !filter.contains(&key) {
                continue;
            }
        }

        let key_str = ctx.get_atom_str(key).to_string();

        let processed = if let Some(repl) = replacer {
            if repl.is_function() {
                let vm_ptr = ctx.get_register_vm_ptr();
                if let Some(ptr) = vm_ptr {
                    let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                    let args = vec![JSValue::new_string(ctx.intern(&key_str)), val];
                    if let Ok(result) = vm.call_function(ctx, *repl, &args) {
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

        let value_str = jsvalue_to_json_internal(ctx, &processed, replacer, seen);
        let escaped_key = key_str.replace('\\', "\\\\").replace('"', "\\\"");
        pairs.push(format!("\"{}\":{}", escaped_key, value_str));
    }

    format!("{{{}}}", pairs.join(","))
}
