use crate::host::HostFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let arity = ctx.get_builtin_arity(name).unwrap_or(1);
    let mut func = crate::object::function::JSFunction::new_builtin(ctx.intern(name), arity);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

pub fn js_float_to_string(f: f64) -> String {
    if f.is_infinite() {
        return if f.is_sign_positive() { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    if f.is_nan() {
        return "NaN".to_string();
    }
    format!("{}", f)
}

fn this_to_string(ctx: &mut JSContext, this: &JSValue) -> Option<String> {
    if this.is_string() {
        return Some(ctx.get_atom_str(this.get_atom()).to_string());
    }
    if this.is_undefined() || this.is_null() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(
            ctx.common_atoms.message,
            JSValue::new_string(ctx.intern("this is not coercible")),
        );
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return None;
    }
    if this.is_bool() {
        return Some(if this.get_bool() { "true".to_string() } else { "false".to_string() });
    }
    if this.is_int() {
        return Some(format!("{}", this.get_int()));
    }
    if this.is_float() {
        return Some(js_float_to_string(this.get_float()));
    }
    if this.is_object() {
        let obj = this.as_object();
        if let Some(v) = obj.get(ctx.common_atoms.__value__) {
            if v.is_string() {
                return Some(ctx.get_atom_str(v.get_atom()).to_string());
            }
            if v.is_int() {
                return Some(format!("{}", v.get_int()));
            }
            if v.is_float() {
                return Some(js_float_to_string(v.get_float()));
            }
            if v.is_bool() {
                return Some(if v.get_bool() { "true".to_string() } else { "false".to_string() });
            }
        }
    }
    Some(format!("{}", this.to_number()))
}

fn require_string_coercible(ctx: &mut JSContext, this: &JSValue) -> Option<String> {
    if this.is_undefined() || this.is_null() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(
            ctx.common_atoms.message,
            JSValue::new_string(ctx.intern("this is not coercible")),
        );
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return None;
    }
    if this.is_string() {
        return Some(ctx.get_atom_str(this.get_atom()).to_string());
    }
    if this.is_bool() {
        return Some(if this.get_bool() { "true".to_string() } else { "false".to_string() });
    }
    if this.is_int() {
        return Some(format!("{}", this.get_int()));
    }
    if this.is_float() {
        return Some(js_float_to_string(this.get_float()));
    }
    if this.is_object() {
        let obj = this.as_object();
        if let Some(v) = obj.get(ctx.common_atoms.__value__) {
            if v.is_string() {
                return Some(ctx.get_atom_str(v.get_atom()).to_string());
            }
            if v.is_int() {
                return Some(format!("{}", v.get_int()));
            }
            if v.is_float() {
                return Some(js_float_to_string(v.get_float()));
            }
            if v.is_bool() {
                return Some(if v.get_bool() { "true".to_string() } else { "false".to_string() });
            }
        }
        return Some(ctx.get_atom_str(this.get_atom()).to_string());
    }
    Some(ctx.get_atom_str(this.get_atom()).to_string())
}

pub fn init_string(ctx: &mut JSContext) {
    let string_atom = ctx.common_atoms.string;

    let mut string_func = crate::object::function::JSFunction::new_builtin(string_atom, 1);
    string_func.set_builtin_marker(ctx, "string_constructor");

    string_func.base.set(
        ctx.intern("fromCharCode"),
        create_builtin_function(ctx, "string_fromCharCode"),
    );
    string_func.base.set(
        ctx.intern("fromCodePoint"),
        create_builtin_function(ctx, "string_fromCodePoint"),
    );

    let string_ptr = Box::into_raw(Box::new(string_func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(string_ptr);
    let string_value = JSValue::new_function(string_ptr);
    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(string_atom, string_value);
    }

    let proto_atom = ctx.intern("StringPrototype");
    let mut proto_obj = JSObject::new();
    proto_obj.set(
        ctx.intern("charAt"),
        create_builtin_function(ctx, "string_charAt"),
    );
    proto_obj.set(
        ctx.intern("charCodeAt"),
        create_builtin_function(ctx, "string_charCodeAt"),
    );
    proto_obj.set(
        ctx.intern("concat"),
        create_builtin_function(ctx, "string_concat"),
    );
    proto_obj.set(
        ctx.intern("indexOf"),
        create_builtin_function(ctx, "string_indexOf"),
    );
    proto_obj.set(
        ctx.intern("lastIndexOf"),
        create_builtin_function(ctx, "string_lastIndexOf"),
    );
    proto_obj.set(
        ctx.intern("slice"),
        create_builtin_function(ctx, "string_slice"),
    );
    proto_obj.set(
        ctx.intern("substring"),
        create_builtin_function(ctx, "string_substring"),
    );
    proto_obj.set(
        ctx.intern("toString"),
        create_builtin_function(ctx, "string_toString"),
    );
    proto_obj.set(
        ctx.intern("valueOf"),
        create_builtin_function(ctx, "string_toString"),
    );
    proto_obj.set(
        ctx.intern("toLowerCase"),
        create_builtin_function(ctx, "string_toLowerCase"),
    );
    proto_obj.set(
        ctx.intern("toUpperCase"),
        create_builtin_function(ctx, "string_toUpperCase"),
    );
    proto_obj.set(
        ctx.intern("split"),
        create_builtin_function(ctx, "string_split"),
    );
    proto_obj.set(
        ctx.common_atoms.length,
        create_builtin_function(ctx, "string_length"),
    );
    proto_obj.set(
        ctx.intern("trim"),
        create_builtin_function(ctx, "string_trim"),
    );
    proto_obj.set(
        ctx.intern("trimStart"),
        create_builtin_function(ctx, "string_trimStart"),
    );
    proto_obj.set(
        ctx.intern("trimLeft"),
        create_builtin_function(ctx, "string_trimStart"),
    );
    proto_obj.set(
        ctx.intern("trimEnd"),
        create_builtin_function(ctx, "string_trimEnd"),
    );
    proto_obj.set(
        ctx.intern("trimRight"),
        create_builtin_function(ctx, "string_trimEnd"),
    );
    proto_obj.set(
        ctx.intern("startsWith"),
        create_builtin_function(ctx, "string_startsWith"),
    );
    proto_obj.set(
        ctx.intern("endsWith"),
        create_builtin_function(ctx, "string_endsWith"),
    );
    proto_obj.set(
        ctx.intern("repeat"),
        create_builtin_function(ctx, "string_repeat"),
    );
    proto_obj.set(
        ctx.intern("includes"),
        create_builtin_function(ctx, "string_includes"),
    );
    proto_obj.set(
        ctx.intern("replace"),
        create_builtin_function(ctx, "string_replace"),
    );
    proto_obj.set(
        ctx.intern("padStart"),
        create_builtin_function(ctx, "string_padStart"),
    );
    proto_obj.set(
        ctx.intern("padEnd"),
        create_builtin_function(ctx, "string_padEnd"),
    );
    proto_obj.set(
        ctx.intern("replaceAll"),
        create_builtin_function(ctx, "string_replaceAll"),
    );
    proto_obj.set(
        ctx.intern("search"),
        create_builtin_function(ctx, "string_search"),
    );
    proto_obj.set(
        ctx.intern("match"),
        create_builtin_function(ctx, "string_match"),
    );
    proto_obj.set(ctx.intern("at"), create_builtin_function(ctx, "string_at"));
    proto_obj.set(
        ctx.intern("isWellFormed"),
        create_builtin_function(ctx, "string_isWellFormed"),
    );
    proto_obj.set(
        ctx.intern("toWellFormed"),
        create_builtin_function(ctx, "string_toWellFormed"),
    );
    proto_obj.set(
        ctx.intern("codePointAt"),
        create_builtin_function(ctx, "string_codePointAt"),
    );
    proto_obj.set(
        ctx.intern("matchAll"),
        create_builtin_function(ctx, "string_matchAll"),
    );
    proto_obj.set(
        ctx.intern("substr"),
        create_builtin_function(ctx, "string_substr"),
    );

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto_obj.prototype = Some(obj_proto_ptr);
    }

    let proto_ptr = Box::into_raw(Box::new(proto_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);

    ctx.set_string_prototype(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);

    let string_func_ptr = string_ptr as *mut crate::object::function::JSFunction;
    unsafe {
        (*string_func_ptr)
            .base
            .set(ctx.common_atoms.prototype, proto_value);
    }

    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(proto_atom, proto_value);
    }
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin(
        "string_constructor",
        HostFunction::ctor("String", 1, string_constructor),
    );
    ctx.register_builtin(
        "string_charAt",
        HostFunction::method("charAt", 1, string_char_at),
    );
    ctx.register_builtin(
        "string_charCodeAt",
        HostFunction::method("charCodeAt", 1, string_char_code_at),
    );
    ctx.register_builtin(
        "string_concat",
        HostFunction::method("concat", 1, string_concat),
    );
    ctx.register_builtin(
        "string_indexOf",
        HostFunction::method("indexOf", 1, string_index_of),
    );
    ctx.register_builtin(
        "string_lastIndexOf",
        HostFunction::method("lastIndexOf", 1, string_last_index_of),
    );
    ctx.register_builtin("string_slice", HostFunction::method("slice", 2, string_slice));
    ctx.register_builtin(
        "string_substring",
        HostFunction::method("substring", 2, string_substring),
    );
    ctx.register_builtin(
        "string_toString",
        HostFunction::method("toString", 0, string_to_string),
    );
    ctx.register_builtin(
        "string_toLowerCase",
        HostFunction::method("toLowerCase", 0, string_to_lower_case),
    );
    ctx.register_builtin(
        "string_toUpperCase",
        HostFunction::method("toUpperCase", 0, string_to_upper_case),
    );
    ctx.register_builtin("string_split", HostFunction::method("split", 1, string_split));
    ctx.register_builtin(
        "string_length",
        HostFunction::method("length", 0, string_length),
    );
    ctx.register_builtin(
        "string_fromCharCode",
        HostFunction::new("fromCharCode", 1, string_fromcharcode),
    );
    ctx.register_builtin(
        "string_fromCodePoint",
        HostFunction::new("fromCodePoint", 1, string_fromcodepoint),
    );
    ctx.register_builtin("string_trim", HostFunction::method("trim", 0, string_trim));
    ctx.register_builtin(
        "string_trimStart",
        HostFunction::method("trimStart", 0, string_trim_start),
    );
    ctx.register_builtin(
        "string_trimEnd",
        HostFunction::method("trimEnd", 0, string_trim_end),
    );
    ctx.register_builtin(
        "string_startsWith",
        HostFunction::method("startsWith", 1, string_starts_with),
    );
    ctx.register_builtin(
        "string_endsWith",
        HostFunction::method("endsWith", 1, string_ends_with),
    );
    ctx.register_builtin(
        "string_repeat",
        HostFunction::method("repeat", 1, string_repeat),
    );
    ctx.register_builtin(
        "string_includes",
        HostFunction::method("includes", 1, string_includes),
    );
    ctx.register_builtin(
        "string_replace",
        HostFunction::method("replace", 2, string_replace),
    );
    ctx.register_builtin(
        "string_padStart",
        HostFunction::method("padStart", 1, string_pad_start),
    );
    ctx.register_builtin(
        "string_padEnd",
        HostFunction::method("padEnd", 1, string_pad_end),
    );
    ctx.register_builtin(
        "string_replaceAll",
        HostFunction::method("replaceAll", 2, string_replace_all),
    );
    ctx.register_builtin(
        "string_search",
        HostFunction::method("search", 1, string_search),
    );
    ctx.register_builtin("string_match", HostFunction::method("match", 1, string_match));
    ctx.register_builtin("string_at", HostFunction::method("at", 1, string_at));
    ctx.register_builtin(
        "string_isWellFormed",
        HostFunction::method("isWellFormed", 0, string_is_well_formed),
    );
    ctx.register_builtin(
        "string_toWellFormed",
        HostFunction::method("toWellFormed", 0, string_to_well_formed),
    );
    ctx.register_builtin(
        "string_codePointAt",
        HostFunction::method("codePointAt", 1, string_code_point_at),
    );
    ctx.register_builtin(
        "string_matchAll",
        HostFunction::method("matchAll", 1, string_match_all),
    );
    ctx.register_builtin(
        "string_substr",
        HostFunction::method("substr", 2, string_substr),
    );
}

fn string_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let s = if args.is_empty() {
        JSValue::new_string(ctx.intern(""))
    } else {
        let val = &args[0];
        if val.is_string() {
            val.clone()
        } else if val.is_int() {
            JSValue::new_string(ctx.intern(&val.get_int().to_string()))
        } else if val.is_float() {
            JSValue::new_string(ctx.intern(&val.get_float().to_string()))
        } else if val.is_bigint() {
            let obj = unsafe { crate::value::JSValue::object_from_ptr(val.get_ptr()) };
            let n = obj.get_bigint_value();
            JSValue::new_string(ctx.intern(&n.to_string()))
        } else if val.is_bool() {
            JSValue::new_string(ctx.intern(&val.get_bool().to_string()))
        } else if val.is_null() {
            JSValue::new_string(ctx.common_atoms.null)
        } else if val.is_undefined() {
            JSValue::new_string(ctx.common_atoms.undefined)
        } else if val.is_object() {
            JSValue::new_string(ctx.intern("[object Object]"))
        } else {
            JSValue::new_string(ctx.intern(""))
        }
    };
    s
}

fn char_count(s: &str) -> usize {
    s.chars().count()
}

fn byte_index_for_char_pos(s: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    match s.char_indices().nth(pos) {
        Some((idx, _)) => idx,
        None => s.len(),
    }
}

fn string_char_at(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    let atom = ctx.intern(&s);

    let index = if args.len() > 1 {
        args[1].get_int() as usize
    } else {
        0
    };

    if index >= ctx.string_char_count(atom) {
        return JSValue::new_string(ctx.intern(""));
    }

    match ctx.string_char_code_at(atom, index) {
        Some(code) => {
            let c = char::from_u32(code).unwrap_or('\0');
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            JSValue::new_string(ctx.intern(s))
        }
        None => JSValue::new_string(ctx.intern("")),
    }
}

fn string_char_code_at(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    let atom = ctx.intern(&s);

    let index = if args.len() > 1 {
        let v = args[1];
        if v.is_int() {
            let i = v.get_int();
            if i < 0 {
                return JSValue::new_float(f64::NAN);
            }
            i as usize
        } else if v.is_float() {
            let f = v.get_float();
            if f < 0.0 || f.is_nan() || f.is_infinite() {
                return JSValue::new_float(f64::NAN);
            }
            f as usize
        } else {
            0
        }
    } else {
        0
    };

    match ctx.string_char_code_at(atom, index) {
        Some(code) => JSValue::new_int(code as i64),
        None => JSValue::new_float(f64::NAN),
    }
}

fn string_concat(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    let mut result = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };

    for arg in args.iter().skip(1) {
        if arg.is_string() {
            result.push_str(ctx.get_atom_str(arg.get_atom()));
        } else if arg.is_int() {
            result.push_str(&arg.get_int().to_string());
        } else if arg.is_float() {
            result.push_str(&arg.get_float().to_string());
        } else if arg.is_bool() {
            result.push_str(&arg.get_bool().to_string());
        }
    }

    JSValue::new_string(ctx.intern(&result))
}

fn string_index_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::new_int(-1);
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };

    let search = if args[1].is_string() {
        ctx.get_atom_str(args[1].get_atom()).to_string()
    } else {
        return JSValue::new_int(-1);
    };

    let from_index = if args.len() > 2 {
        args[2].get_int().max(0) as usize
    } else {
        0
    };

    let s_char_len = char_count(&s);
    if from_index > s_char_len {
        return JSValue::new_int(-1);
    }

    let from_byte = byte_index_for_char_pos(&s, from_index);

    match s[from_byte..].find(&search) {
        Some(byte_pos) => {
            let byte_idx = from_byte + byte_pos;
            let char_idx = s[..byte_idx].chars().count();
            JSValue::new_int(char_idx as i64)
        }
        None => JSValue::new_int(-1),
    }
}

fn string_last_index_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::new_int(-1);
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };

    let search = if args[1].is_string() {
        ctx.get_atom_str(args[1].get_atom()).to_string()
    } else {
        return JSValue::new_int(-1);
    };

    match s.rfind(&search) {
        Some(byte_pos) => {
            let char_idx = s[..byte_pos].chars().count();
            JSValue::new_int(char_idx as i64)
        }
        None => JSValue::new_int(-1),
    }
}

fn string_substring(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };

    let len = char_count(&s);
    let start = if args.len() > 1 {
        let v = args[1].get_int().max(0) as usize;
        v.min(len)
    } else {
        0
    };

    let end = if args.len() > 2 {
        let v = args[2].get_int().max(0) as usize;
        v.min(len)
    } else {
        len
    };

    let (start, end) = if start > end {
        (end, start)
    } else {
        (start, end)
    };

    let start_b = byte_index_for_char_pos(&s, start);
    let end_b = byte_index_for_char_pos(&s, end);
    JSValue::new_string(ctx.intern(&s[start_b..end_b]))
}

fn string_slice(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };

    let len = char_count(&s);
    let start = if args.len() > 1 {
        let v = args[1].get_int();
        if v < 0 {
            (len as i64 + v).max(0) as usize
        } else {
            (v as usize).min(len)
        }
    } else {
        0
    };

    let end = if args.len() > 2 {
        let v = args[2].get_int();
        if v < 0 {
            (len as i64 + v).max(0) as usize
        } else {
            (v as usize).min(len)
        }
    } else {
        len
    };

    if start >= end {
        return JSValue::new_string(ctx.intern(""));
    }

    let start_b = byte_index_for_char_pos(&s, start);
    let end_b = byte_index_for_char_pos(&s, end);
    JSValue::new_string(ctx.intern(&s[start_b..end_b]))
}

fn string_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    match require_string_coercible(ctx, &args[0]) {
        Some(s) => JSValue::new_string(ctx.intern(&s)),
        None => JSValue::undefined(),
    }
}

fn string_to_lower_case(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s.to_lowercase(),
        None => return JSValue::undefined(),
    };

    JSValue::new_string(ctx.intern(&s))
}

fn string_to_upper_case(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s.to_uppercase(),
        None => return JSValue::undefined(),
    };

    JSValue::new_string(ctx.intern(&s))
}

fn string_split(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        let mut result = JSObject::new_array();
        result.set(ctx.common_atoms.length, JSValue::new_int(0));
        let ptr = Box::into_raw(Box::new(result)) as usize;
        return JSValue::new_object(ptr);
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    let s_atom = ctx.intern(&s);

    let length_atom = ctx.common_atoms.length;

    if args.len() > 1 && args[1].is_object() {
        let sep_obj = args[1].as_object();
        let pattern_atom = ctx.common_atoms.__pattern__;
        if sep_obj.get(pattern_atom).is_some() {
            let limit = if args.len() > 2 && args[2].is_int() {
                args[2].get_int() as usize
            } else {
                usize::MAX
            };

            let mut parts: Vec<String> = Vec::new();
            let compiled_re = args[1]
                .as_object()
                .get_compiled_regex()
                .map(|re| re as *const crate::regexp::Regex);
            if let Some(re_ptr) = compiled_re {
                let re = unsafe { &*re_ptr };
                let mut last = 0usize;
                let mut search_start = 0usize;
                loop {
                    if parts.len() >= limit {
                        break;
                    }
                    if search_start > s.len() {
                        break;
                    }
                    let sub = &s[search_start..];
                    if let Some(m) = re.find(sub) {
                        let match_start = search_start + m.start();
                        let match_end = search_start + m.end();
                        if match_end == search_start && match_start == last {
                            search_start += s[search_start..]
                                .chars()
                                .next()
                                .map(|c| c.len_utf8())
                                .unwrap_or(1);
                            continue;
                        }
                        parts.push(s[last..match_start].to_string());

                        for cap in m.iter().skip(1) {
                            parts.push(cap.unwrap_or("").to_string());
                        }
                        last = match_end;
                        search_start = match_end;
                        if match_end == match_start {
                            search_start += s[search_start..]
                                .chars()
                                .next()
                                .map(|c| c.len_utf8())
                                .unwrap_or(1);
                        }
                    } else {
                        break;
                    }
                }
                if parts.len() < limit {
                    parts.push(s[last..].to_string());
                }
            } else {
                parts.push(s);
            }

            let mut result = JSObject::new_array();
            for (i, part) in parts.iter().enumerate() {
                let key = ctx.int_atom_mut(i);
                let part_atom = ctx.intern(part);
                result.set(key, JSValue::new_string(part_atom));
            }
            result.set(length_atom, JSValue::new_int(parts.len() as i64));
            let ptr = Box::into_raw(Box::new(result)) as usize;
            return JSValue::new_object(ptr);
        }
    }

    let sep_atom = if args.len() > 1 && args[1].is_string() {
        Some(args[1].get_atom())
    } else {
        None
    };

    let s = ctx.get_atom_str(s_atom);
    let separator = sep_atom.map(|a| ctx.get_atom_str(a)).unwrap_or("");

    let mut result = JSObject::new_array();

    if separator.is_empty() {
        let chars: Vec<char> = s.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            let key = ctx.int_atom_mut(i);
            let c_str = c.to_string();
            let c_atom = ctx.intern(&c_str);
            result.set(key, JSValue::new_string(c_atom));
        }
        result.set(length_atom, JSValue::new_int(chars.len() as i64));
    } else {
        let parts: Vec<String> = s.split(separator).map(|s| s.to_string()).collect();
        for (i, part) in parts.iter().enumerate() {
            let key = ctx.int_atom_mut(i);
            let part_atom = ctx.intern(part);
            result.set(key, JSValue::new_string(part_atom));
        }
        result.set(length_atom, JSValue::new_int(parts.len() as i64));
    }

    let ptr = Box::into_raw(Box::new(result)) as usize;
    JSValue::new_object(ptr)
}

fn string_length(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_int(0);
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    JSValue::new_int(char_count(&s) as i64)
}

fn from_str_or_hex(s: &str) -> u32 {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).unwrap_or(0)
    } else {
        s.parse::<f64>().unwrap_or(0.0) as u32
    }
}

fn string_fromcharcode(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let mut result = String::new();
    for arg in args {
        let code = if arg.is_string() {
            from_str_or_hex(ctx.get_atom_str(arg.get_atom()))
        } else if arg.is_int() {
            arg.get_int() as u32
        } else {
            0
        };
        if let Some(c) = char::from_u32(code & 0xFFFF) {
            result.push(c);
        }
    }
    JSValue::new_string(ctx.intern(&result))
}

fn string_fromcodepoint(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let mut result = String::new();
    for arg in args {
        let code = arg.get_int() as u32;
        if let Some(c) = char::from_u32(code) {
            result.push(c);
        }
    }
    JSValue::new_string(ctx.intern(&result))
}

fn string_trim(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    let trimmed = s.trim();
    JSValue::new_string(ctx.intern(trimmed))
}

fn string_trim_start(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    match this_to_string(ctx, &args[0]) {
        Some(s) => JSValue::new_string(ctx.intern(s.trim_start())),
        None => JSValue::undefined(),
    }
}

fn string_trim_end(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    match this_to_string(ctx, &args[0]) {
        Some(s) => JSValue::new_string(ctx.intern(s.trim_end())),
        None => JSValue::undefined(),
    }
}

fn string_starts_with(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::bool(false);
    }
    let s = match this_to_string(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    let prefix = ctx.get_atom_str(args[1].get_atom()).to_string();
    JSValue::bool(s.starts_with(&prefix))
}

fn string_ends_with(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::bool(false);
    }
    let s = match this_to_string(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    let suffix = ctx.get_atom_str(args[1].get_atom()).to_string();
    JSValue::bool(s.ends_with(&suffix))
}

fn string_repeat(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    let s = match this_to_string(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    if args.len() < 2 {
        return JSValue::new_string(ctx.intern(""));
    }
    let count = args[1].get_int();
    if count < 0 {
        return JSValue::new_string(ctx.intern(""));
    }
    let count = count as usize;
    let repeated = s.repeat(count);
    JSValue::new_string(ctx.intern(&repeated))
}

fn string_includes(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::bool(false);
    }
    let s = match this_to_string(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    let search = ctx.get_atom_str(args[1].get_atom()).to_string();
    JSValue::bool(s.contains(&search))
}

fn string_replace(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    if args.len() < 3 {
        return JSValue::new_string(ctx.intern(&s));
    }

    if args[1].is_object() {
        let regexp_obj = args[1].as_object();

        let pattern_atom = ctx.common_atoms.__pattern__;
        let flags_atom = ctx.common_atoms.__flags__;

        let pattern = if let Some(p) = regexp_obj.get(pattern_atom) {
            if p.is_string() {
                ctx.get_atom_str(p.get_atom()).to_string()
            } else {
                return JSValue::new_string(ctx.intern(&s));
            }
        } else {
            return JSValue::new_string(ctx.intern(&s));
        };

        let flags = if let Some(f) = regexp_obj.get(flags_atom) {
            if f.is_string() {
                ctx.get_atom_str(f.get_atom()).to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let replacement = if args[2].is_string() {
            ctx.get_atom_str(args[2].get_atom()).to_string()
        } else {
            String::new()
        };

        let ignore_case = flags.contains('i');
        let is_global = flags.contains('g');

        if is_global {
            let mut result = String::new();
            let mut last_end = 0;
            let mut search_from = 0;
            let pat_lower = if ignore_case {
                pattern.to_lowercase()
            } else {
                String::new()
            };
            let s_lower = if ignore_case {
                s.to_lowercase()
            } else {
                String::new()
            };

            loop {
                let found = if ignore_case {
                    s_lower[search_from..]
                        .find(&pat_lower)
                        .map(|i| i + search_from)
                } else {
                    s[search_from..].find(&pattern).map(|i| i + search_from)
                };

                if let Some(pos) = found {
                    result.push_str(&s[last_end..pos]);
                    result.push_str(&replacement);
                    last_end = pos + pattern.len();
                    search_from = last_end;
                    if pattern.is_empty() {
                        if search_from < s.len() {
                            result.push_str(&s[search_from..search_from + 1]);
                            search_from += 1;
                            last_end = search_from;
                        } else {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
            result.push_str(&s[last_end..]);
            JSValue::new_string(ctx.intern(&result))
        } else {
            let found = if ignore_case {
                s.to_lowercase().find(&pattern.to_lowercase())
            } else {
                s.find(&pattern)
            };

            if let Some(pos) = found {
                let mut result = String::new();
                result.push_str(&s[..pos]);
                result.push_str(&replacement);
                result.push_str(&s[pos + pattern.len()..]);
                JSValue::new_string(ctx.intern(&result))
            } else {
                JSValue::new_string(ctx.intern(&s))
            }
        }
    } else {
        let search = if args[1].is_string() {
            ctx.get_atom_str(args[1].get_atom()).to_string()
        } else {
            return JSValue::new_string(ctx.intern(&s));
        };
        let replacement = if args[2].is_string() {
            ctx.get_atom_str(args[2].get_atom()).to_string()
        } else if args[2].is_function() {
            if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
                let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
                let result = vm.call_function_with_this(
                    ctx,
                    args[2],
                    JSValue::undefined(),
                    &[args[1].clone(), JSValue::new_int(0), args[0].clone()],
                );
                match result {
                    Ok(v) => {
                        if v.is_string() {
                            ctx.get_atom_str(v.get_atom()).to_string()
                        } else if v.is_undefined() {
                            "undefined".to_string()
                        } else if v.is_null() {
                            "null".to_string()
                        } else {
                            format!("{}", v.get_int())
                        }
                    }
                    Err(_) => String::new(),
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let result = s.replace(&search, &replacement);
        JSValue::new_string(ctx.intern(&result))
    }
}

fn js_to_length(val: &JSValue) -> usize {
    let n = val.to_number();
    if n.is_nan() || n.is_infinite() || n <= 0.0 {
        0
    } else {
        n as usize
    }
}

fn js_to_string_arg(val: &JSValue, ctx: &mut JSContext) -> String {
    if val.is_string() {
        ctx.get_atom_str(val.get_atom()).to_string()
    } else if val.is_int() {
        format!("{}", val.get_int())
    } else if val.is_float() {
        js_float_to_string(val.get_float())
    } else if val.is_bool() {
        if val.get_bool() { "true".to_string() } else { "false".to_string() }
    } else if val.is_undefined() {
        "undefined".to_string()
    } else if val.is_null() {
        "null".to_string()
    } else {
        "[object Object]".to_string()
    }
}

fn string_pad_start(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    let target_len = if args.len() > 1 {
        js_to_length(&args[1])
    } else {
        0
    };
    let pad_str = if args.len() > 2 {
        js_to_string_arg(&args[2], ctx)
    } else {
        " ".to_string()
    };

    let s_len = s.chars().count();
    if target_len <= s_len {
        return JSValue::new_string(ctx.intern(&s));
    }

    let mut pad_count = target_len - s_len;
    let mut prepend = String::new();
    let pad_chars: Vec<char> = pad_str.chars().collect();
    let pad_char_len = pad_chars.len();
    if pad_char_len == 0 {
        return JSValue::new_string(ctx.intern(&s));
    }
    let mut i = 0;
    while pad_count > 0 {
        prepend.push(pad_chars[i % pad_char_len]);
        pad_count -= 1;
        i += 1;
    }

    JSValue::new_string(ctx.intern(&(prepend + &s)))
}

fn string_pad_end(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    let target_len = if args.len() > 1 {
        js_to_length(&args[1])
    } else {
        0
    };
    let pad_str = if args.len() > 2 {
        js_to_string_arg(&args[2], ctx)
    } else {
        " ".to_string()
    };

    let s_len = s.chars().count();
    if target_len <= s_len {
        return JSValue::new_string(ctx.intern(&s));
    }

    let mut pad_count = target_len - s_len;
    let mut append = String::new();
    let pad_chars: Vec<char> = pad_str.chars().collect();
    let pad_char_len = pad_chars.len();
    if pad_char_len == 0 {
        return JSValue::new_string(ctx.intern(&s));
    }
    let mut i = 0;
    while pad_count > 0 {
        append.push(pad_chars[i % pad_char_len]);
        pad_count -= 1;
        i += 1;
    }

    JSValue::new_string(ctx.intern(&(s + &append)))
}

fn string_replace_all(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    if args.len() < 3 {
        return JSValue::new_string(ctx.intern(&s));
    }
    let search = if args[1].is_string() {
        ctx.get_atom_str(args[1].get_atom()).to_string()
    } else {
        return args[0];
    };
    let replacement = if args[2].is_string() {
        ctx.get_atom_str(args[2].get_atom()).to_string()
    } else {
        String::new()
    };

    let result = s.replace(&search as &str, &replacement as &str);
    JSValue::new_string(ctx.intern(&result))
}

fn string_at(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let idx = if args.len() > 1 { args[1].get_int() } else { 0 };
    let actual_idx = if idx < 0 { len as i64 + idx } else { idx };
    if actual_idx < 0 || actual_idx as usize >= len {
        return JSValue::undefined();
    }
    let c = chars[actual_idx as usize];
    JSValue::new_string(ctx.intern(&c.to_string()))
}

fn string_is_well_formed(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::bool(false);
    }
    let _s = match require_string_coercible(_ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    JSValue::bool(true)
}

fn string_to_well_formed(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    JSValue::new_string(ctx.intern(&s))
}

fn string_code_point_at(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let index = if args.len() > 1 {
        args[1].get_int() as i64
    } else {
        0
    };
    let actual_index = if index < 0 { len as i64 + index } else { index };
    if actual_index < 0 || actual_index as usize >= len {
        return JSValue::undefined();
    }
    let c = chars[actual_index as usize];
    if (c as u32) >= 0xD800 && (c as u32) <= 0xDBFF && actual_index as usize + 1 < len {
        let next = chars[actual_index as usize + 1];
        if (next as u32) >= 0xDC00 && (next as u32) <= 0xDFFF {
            let cp = 0x10000 + (((c as u32) - 0xD800) << 10) + (next as u32) - 0xDC00;
            return JSValue::new_int(cp as i64);
        }
    }
    JSValue::new_int(c as u32 as i64)
}

fn string_match_all(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        let mut result = JSObject::new_array();
        result.set(ctx.common_atoms.length, JSValue::new_int(0));
        let ptr = Box::into_raw(Box::new(result)) as usize;
        return JSValue::new_object(ptr);
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };

    let pattern = if args.len() > 1 {
        if args[1].is_object() {
            let regexp_obj = args[1].as_object();

            let flags_atom = ctx.common_atoms.__flags__;
            let flags = if let Some(f) = regexp_obj.get(flags_atom) {
                if f.is_string() {
                    ctx.get_atom_str(f.get_atom()).to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            if !flags.contains('g') {
                let mut err_obj = JSObject::new();
                err_obj.set(
                    ctx.common_atoms.name,
                    JSValue::new_string(ctx.intern("TypeError")),
                );
                err_obj.set(
                    ctx.common_atoms.message,
                    JSValue::new_string(ctx.intern("matchAll requires a global RegExp")),
                );
                let ptr = Box::into_raw(Box::new(err_obj)) as usize;
                return JSValue::new_object(ptr);
            }

            let pattern_atom = ctx.common_atoms.__pattern__;
            if let Some(p) = regexp_obj.get(pattern_atom) {
                if p.is_string() {
                    ctx.get_atom_str(p.get_atom()).to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else if args[1].is_string() {
            ctx.get_atom_str(args[1].get_atom()).to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let mut result_array = JSObject::new_array();
    let length_atom = ctx.common_atoms.length;
    let mut match_count = 0;

    if pattern.is_empty() {
        result_array.set(length_atom, JSValue::new_int(0));
        let ptr = Box::into_raw(Box::new(result_array)) as usize;
        return JSValue::new_object(ptr);
    }

    let mut search_pos = 0;
    while search_pos <= s.len() {
        let max_start = if pattern.len() <= s.len() {
            s.len() - pattern.len()
        } else {
            break;
        };

        let mut found = false;
        for i in search_pos..=max_start {
            if i + pattern.len() <= s.len() {
                let slice = &s[i..i + pattern.len()];
                if slice == pattern {
                    found = true;
                    let mut match_obj = JSObject::new();
                    match_obj.set(ctx.intern("0"), JSValue::new_string(ctx.intern(slice)));
                    match_obj.set(ctx.common_atoms.index, JSValue::new_int(i as i64));
                    match_obj.set(ctx.common_atoms.input, JSValue::new_string(ctx.intern(&s)));
                    match_obj.set(ctx.common_atoms.length, JSValue::new_int(1));

                    let key = ctx.int_atom_mut(match_count);
                    result_array.set(
                        key,
                        JSValue::new_object(Box::into_raw(Box::new(match_obj)) as usize),
                    );

                    match_count += 1;
                    search_pos = i + pattern.len();
                    if pattern.len() == 0 {
                        search_pos += 1;
                    }
                    break;
                }
            }
        }

        if !found {
            break;
        }
    }

    result_array.set(length_atom, JSValue::new_int(match_count as i64));
    let ptr = Box::into_raw(Box::new(result_array)) as usize;
    JSValue::new_object(ptr)
}

fn string_search(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::new_int(-1);
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };

    if args[1].is_object() {
        let regexp_obj = args[1].as_object();
        let pattern_atom = ctx.common_atoms.__pattern__;
        let flags_atom = ctx.common_atoms.__flags__;

        let pattern = if let Some(p) = regexp_obj.get(pattern_atom) {
            if p.is_string() {
                ctx.get_atom_str(p.get_atom()).to_string()
            } else {
                return JSValue::new_int(-1);
            }
        } else {
            return JSValue::new_int(-1);
        };

        let flags = if let Some(f) = regexp_obj.get(flags_atom) {
            if f.is_string() {
                ctx.get_atom_str(f.get_atom()).to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let ignore_case = flags.contains('i');
        let s_lower = if ignore_case {
            s.to_lowercase()
        } else {
            String::new()
        };
        let p_lower = if ignore_case {
            pattern.to_lowercase()
        } else {
            String::new()
        };

        let found = if ignore_case {
            s_lower.find(&p_lower)
        } else {
            s.find(&pattern)
        };
        JSValue::new_int(found.map(|i| i as i64).unwrap_or(-1))
    } else if args[1].is_string() {
        let search = ctx.get_atom_str(args[1].get_atom()).to_string();
        JSValue::new_int(s.find(&search).map(|i| i as i64).unwrap_or(-1))
    } else {
        JSValue::new_int(-1)
    }
}

fn string_match(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::null();
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };

    if args[1].is_object() {
        let regexp_obj = args[1].as_object();
        let pattern_atom = ctx.common_atoms.__pattern__;
        let flags_atom = ctx.common_atoms.__flags__;

        let pattern = if let Some(p) = regexp_obj.get(pattern_atom) {
            if p.is_string() {
                ctx.get_atom_str(p.get_atom()).to_string()
            } else {
                return JSValue::null();
            }
        } else {
            return JSValue::null();
        };

        let flags = if let Some(f) = regexp_obj.get(flags_atom) {
            if f.is_string() {
                ctx.get_atom_str(f.get_atom()).to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let is_global = flags.contains('g');
        let ignore_case = flags.contains('i');
        let s_lower = if ignore_case {
            s.to_lowercase()
        } else {
            String::new()
        };
        let p_lower = if ignore_case {
            pattern.to_lowercase()
        } else {
            String::new()
        };

        if is_global {
            let mut results: Vec<String> = Vec::new();
            let mut search_from = 0;
            loop {
                let found = if ignore_case {
                    s_lower[search_from..]
                        .find(&p_lower)
                        .map(|i| i + search_from)
                } else {
                    s[search_from..].find(&pattern).map(|i| i + search_from)
                };
                if let Some(pos) = found {
                    let end = (pos + pattern.len()).min(s.len());
                    results.push(s[pos..end].to_string());
                    search_from = end;
                    if pattern.is_empty() {
                        if search_from < s.len() {
                            search_from += 1;
                        } else {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
            if results.is_empty() {
                return JSValue::null();
            }
            let mut arr = JSObject::new_array();
            let length_atom = ctx.common_atoms.length;
            for (i, m) in results.iter().enumerate() {
                let key = ctx.int_atom_mut(i);
                arr.set(key, JSValue::new_string(ctx.intern(m)));
            }
            arr.set(length_atom, JSValue::new_int(results.len() as i64));
            let ptr = Box::into_raw(Box::new(arr)) as usize;
            JSValue::new_object(ptr)
        } else {
            let found = if ignore_case {
                s_lower.find(&p_lower)
            } else {
                s.find(&pattern)
            };
            if let Some(pos) = found {
                let end = (pos + pattern.len()).min(s.len());
                let match_str = &s[pos..end];
                let mut result = JSObject::new_array();
                result.set(ctx.intern("0"), JSValue::new_string(ctx.intern(match_str)));
                result.set(ctx.common_atoms.index, JSValue::new_int(pos as i64));
                result.set(ctx.common_atoms.input, JSValue::new_string(ctx.intern(&s)));
                result.set(ctx.common_atoms.length, JSValue::new_int(1));
                let ptr = Box::into_raw(Box::new(result)) as usize;
                JSValue::new_object(ptr)
            } else {
                JSValue::null()
            }
        }
    } else {
        JSValue::null()
    }
}

fn string_substr(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }
    let s = match require_string_coercible(ctx, &args[0]) {
        Some(s) => s,
        None => return JSValue::undefined(),
    };
    let len = char_count(&s);

    let start = if args.len() > 1 {
        let v = args[1].get_int();
        if v < 0 {
            (len as i64 + v).max(0) as usize
        } else {
            (v as usize).min(len)
        }
    } else {
        0
    };

    let end = if args.len() > 2 {
        let v = args[2].get_int();
        if v <= 0 {
            return JSValue::new_string(ctx.intern(""));
        }
        (start + v as usize).min(len)
    } else {
        len
    };
    let start_b = byte_index_for_char_pos(&s, start);
    let end_b = byte_index_for_char_pos(&s, end);
    JSValue::new_string(ctx.intern(&s[start_b..end_b]))
}
