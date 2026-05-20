use crate::host::HostFunction;
use crate::object::array_obj::JSArrayObject;
use crate::object::function::JSFunction;
use crate::object::object::{JSObject, PropertyDescriptor};
use crate::runtime::context::JSContext;
use crate::value::JSValue;

fn create_builtin_function(
    ctx: &mut JSContext,
    builtin_name: &str,
    display_name: &str,
    arity: u32,
) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(display_name), arity);
    func.set_builtin_marker(ctx, builtin_name);
    if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
        func.base.set_prototype_raw(fn_proto_ptr);
    }
    define_name_length(ctx, &mut func.base, display_name, arity as i64);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn define_name_length(ctx: &mut JSContext, obj: &mut JSObject, name: &str, length: i64) {
    let name_desc = PropertyDescriptor {
        value: Some(JSValue::new_string(ctx.intern(name))),
        writable: false,
        enumerable: false,
        configurable: true,
        get: None,
        set: None,
    };
    let length_desc = PropertyDescriptor {
        value: Some(JSValue::new_int(length)),
        writable: false,
        enumerable: false,
        configurable: true,
        get: None,
        set: None,
    };
    obj.define_property(ctx.intern("name"), name_desc);
    obj.define_property(ctx.common_atoms.length, length_desc);
}

fn create_array_from_locale_list(ctx: &mut JSContext, list: &[&str]) -> JSValue {
    let mut arr = JSArrayObject::with_capacity(list.len());
    if let Some(proto_ptr) = ctx.get_array_prototype() {
        arr.header.set_prototype_raw(proto_ptr);
    }
    for locale in list {
        arr.push(JSValue::new_string(ctx.intern(locale)));
    }
    arr.header
        .set(ctx.common_atoms.length, JSValue::new_int(list.len() as i64));
    let ptr = Box::into_raw(Box::new(arr)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_array(ptr);
    JSValue::new_object(ptr)
}

fn get_locale_from_first_arg(ctx: &mut JSContext, args: &[JSValue]) -> AtomOrDefault {
    if args.is_empty() {
        return AtomOrDefault::Default;
    }
    let locales = args[0];

    if locales.is_string() {
        return AtomOrDefault::Atom(locales.get_atom());
    }

    if locales.is_object() {
        let obj = locales.as_object();
        if let Some(first) = obj.get(ctx.common_atoms.n0) {
            if first.is_string() {
                return AtomOrDefault::Atom(first.get_atom());
            }
        }
    }

    AtomOrDefault::Default
}

enum AtomOrDefault {
    Atom(crate::runtime::atom::Atom),
    Default,
}

#[derive(Clone, Copy)]
struct CollatorState {
    locale: crate::runtime::atom::Atom,
    usage: crate::runtime::atom::Atom,
    sensitivity: crate::runtime::atom::Atom,
    collation: crate::runtime::atom::Atom,
    ignore_punctuation: bool,
    has_numeric: bool,
    numeric: bool,
    has_case_first: bool,
    case_first: crate::runtime::atom::Atom,
}

fn throw_type_error(ctx: &mut JSContext, msg: &str) {
    let mut err = JSObject::new();
    err.set(
        ctx.common_atoms.name,
        JSValue::new_string(ctx.intern("TypeError")),
    );
    err.set(
        ctx.common_atoms.message,
        JSValue::new_string(ctx.intern(msg)),
    );
    let ptr = Box::into_raw(Box::new(err)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    ctx.pending_exception = Some(JSValue::new_object(ptr));
}

fn throw_range_error(ctx: &mut JSContext, msg: &str) {
    let mut err = JSObject::new();
    err.set(
        ctx.common_atoms.name,
        JSValue::new_string(ctx.intern("RangeError")),
    );
    err.set(
        ctx.common_atoms.message,
        JSValue::new_string(ctx.intern(msg)),
    );
    let ptr = Box::into_raw(Box::new(err)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    ctx.pending_exception = Some(JSValue::new_object(ptr));
}

fn value_to_bool(v: JSValue) -> bool {
    if v.is_bool() {
        v.get_bool()
    } else if v.is_int() {
        v.get_int() != 0
    } else if v.is_float() {
        let n = v.get_float();
        n != 0.0 && !n.is_nan()
    } else if v.is_string() {
        true
    } else if v.is_symbol() {
        true
    } else if v.is_object() || v.is_function() {
        true
    } else {
        false
    }
}

fn value_to_string(ctx: &mut JSContext, v: JSValue) -> String {
    if v.is_string() {
        return ctx.get_atom_str(v.get_atom()).to_string();
    }
    if v.is_int() {
        return v.get_int().to_string();
    }
    if v.is_float() {
        return v.get_float().to_string();
    }
    if v.is_bool() {
        return if v.get_bool() {
            "true".to_string()
        } else {
            "false".to_string()
        };
    }
    if v.is_null() {
        return "null".to_string();
    }
    if v.is_undefined() {
        return "undefined".to_string();
    }
    if v.is_symbol() {
        throw_type_error(ctx, "Cannot convert a Symbol value to a string");
        return String::new();
    }
    if v.is_object() || v.is_function() {
        let obj = v.as_object();
        let to_string_atom = ctx.common_atoms.to_string;
        if let Some(to_string_fn) = obj.get(to_string_atom) {
            if to_string_fn.is_function() {
                if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
                    let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
                    if let Ok(ret) = vm.call_function_with_this(ctx, to_string_fn, v, &[]) {
                        if ret.is_string() {
                            return ctx.get_atom_str(ret.get_atom()).to_string();
                        }
                    }
                }
            }
        }
    }
    String::new()
}

fn parse_bool_option(ctx: &mut JSContext, options: &JSObject, key: &str) -> Option<bool> {
    let key_atom = ctx.intern(key);
    let value = options.get(key_atom)?;
    if value.is_undefined() {
        return None;
    }
    Some(value_to_bool(value))
}

fn parse_string_option(
    ctx: &mut JSContext,
    options: &JSObject,
    key: &str,
    allowed: &[&str],
) -> Result<Option<String>, ()> {
    let key_atom = ctx.intern(key);
    let value = match options.get(key_atom) {
        Some(v) => v,
        None => return Ok(None),
    };
    if value.is_undefined() {
        return Ok(None);
    }
    let s = value_to_string(ctx, value);
    if ctx.pending_exception.is_some() {
        return Err(());
    }
    if allowed.iter().any(|x| *x == s) {
        Ok(Some(s))
    } else {
        throw_range_error(ctx, &format!("Invalid value for {}", key));
        Err(())
    }
}

fn parse_collator_state(ctx: &mut JSContext, args: &[JSValue]) -> Result<CollatorState, ()> {
    let mut locale = match get_locale_from_first_arg(ctx, args) {
        AtomOrDefault::Atom(a) => a,
        AtomOrDefault::Default => ctx.intern("en"),
    };

    let mut usage = ctx.intern("sort");
    let mut sensitivity = ctx.intern("variant");
    let mut ignore_punctuation = false;
    let mut collation = ctx.intern("default");
    let mut has_numeric = false;
    let mut numeric = false;
    let mut has_case_first = false;
    let mut case_first = ctx.intern("false");

    let options_val = if args.len() > 1 {
        args[1]
    } else {
        JSValue::undefined()
    };
    if options_val.is_object() {
        let options = options_val.as_object();

        if let Some(usage_opt) = parse_string_option(ctx, options, "usage", &["sort", "search"])? {
            usage = ctx.intern(&usage_opt);
        }

        if let Some(sensitivity_opt) = parse_string_option(
            ctx,
            options,
            "sensitivity",
            &["base", "accent", "case", "variant"],
        )? {
            sensitivity = ctx.intern(&sensitivity_opt);
        }

        if let Some(ignore) = parse_bool_option(ctx, options, "ignorePunctuation") {
            ignore_punctuation = ignore;
        }

        if let Some(numeric_opt) = parse_bool_option(ctx, options, "numeric") {
            has_numeric = true;
            numeric = numeric_opt;
        }

        if let Some(case_first_opt) =
            parse_string_option(ctx, options, "caseFirst", &["upper", "lower", "false"])?
        {
            has_case_first = true;
            case_first = ctx.intern(&case_first_opt);
        }

        if let Some(collation_opt) =
            parse_string_option(ctx, options, "collation", &["default", "phonebk", "eor"])?
        {
            collation = ctx.intern(&collation_opt);
        }
    }

    let locale_str = ctx.get_atom_str(locale).to_string();
    if locale_str.starts_with("th") && !options_val.is_object() {
        ignore_punctuation = true;
    }

    if locale_str.contains("-x-") {
        if collation == ctx.intern("phonebk") {
            collation = ctx.intern("default");
        }
    }

    if locale_str.contains("-u-") {
        if locale_str.contains("-u-co-phonebk") && collation == ctx.intern("default") {
            collation = ctx.intern("phonebk");
        }
        if !has_case_first {
            if locale_str.contains("-u-kf-upper") {
                has_case_first = true;
                case_first = ctx.intern("upper");
            } else if locale_str.contains("-u-kf-lower") {
                has_case_first = true;
                case_first = ctx.intern("lower");
            } else if locale_str.contains("-u-kf-false") {
                has_case_first = true;
                case_first = ctx.intern("false");
            }
        }
        if !has_numeric {
            if locale_str.contains("-u-kn-true") {
                has_numeric = true;
                numeric = true;
            } else if locale_str.contains("-u-kn-false") {
                has_numeric = true;
                numeric = false;
            }
        }
    }

    if usage == ctx.intern("search") {
        collation = ctx.intern("default");
    }

    if locale_str.contains("-u-co-") {
        if collation != ctx.intern("phonebk") {
            let mut cut = locale_str.clone();
            if let Some(pos) = cut.find("-u-co-") {
                cut.truncate(pos);
                locale = ctx.intern(&cut);
            }
        }
    }
    if locale_str.contains("-u-kf-") {
        if has_case_first
            && !locale_str.contains(&format!("-u-kf-{}", ctx.get_atom_str(case_first)))
        {
            let mut cut = locale_str.clone();
            if let Some(pos) = cut.find("-u-kf-") {
                cut.truncate(pos);
                locale = ctx.intern(&cut);
            }
        }
    }

    Ok(CollatorState {
        locale,
        usage,
        sensitivity,
        collation,
        ignore_punctuation,
        has_numeric,
        numeric,
        has_case_first,
        case_first,
    })
}

fn set_collator_state(ctx: &mut JSContext, target: &mut JSObject, state: CollatorState) {
    target.set(
        ctx.intern("__intlLocale__"),
        JSValue::new_string(state.locale),
    );
    target.set(
        ctx.intern("__intlUsage__"),
        JSValue::new_string(state.usage),
    );
    target.set(
        ctx.intern("__intlSensitivity__"),
        JSValue::new_string(state.sensitivity),
    );
    target.set(
        ctx.intern("__intlCollation__"),
        JSValue::new_string(state.collation),
    );
    target.set(
        ctx.intern("__intlIgnorePunctuation__"),
        JSValue::bool(state.ignore_punctuation),
    );
    target.set(
        ctx.intern("__intlHasNumeric__"),
        JSValue::bool(state.has_numeric),
    );
    target.set(ctx.intern("__intlNumeric__"), JSValue::bool(state.numeric));
    target.set(
        ctx.intern("__intlHasCaseFirst__"),
        JSValue::bool(state.has_case_first),
    );
    target.set(
        ctx.intern("__intlCaseFirst__"),
        JSValue::new_string(state.case_first),
    );
}

fn get_bool_slot(obj: &JSObject, key: crate::runtime::atom::Atom, fallback: bool) -> bool {
    obj.get(key).map(|v| v.is_truthy()).unwrap_or(fallback)
}

fn get_atom_slot(
    ctx: &mut JSContext,
    obj: &JSObject,
    key: crate::runtime::atom::Atom,
    fallback: &str,
) -> crate::runtime::atom::Atom {
    obj.get(key)
        .filter(|v| v.is_string())
        .map(|v| v.get_atom())
        .unwrap_or_else(|| ctx.intern(fallback))
}

fn get_collator_state_from_value(ctx: &mut JSContext, value: JSValue) -> Option<CollatorState> {
    if !(value.is_object() || value.is_function()) {
        return None;
    }
    let obj = value.as_object();
    let locale_key = ctx.intern("__intlLocale__");
    let locale = obj
        .get(locale_key)
        .filter(|v| v.is_string())
        .map(|v| v.get_atom())?;
    let usage_key = ctx.intern("__intlUsage__");
    let sensitivity_key = ctx.intern("__intlSensitivity__");
    let collation_key = ctx.intern("__intlCollation__");
    let ignore_punctuation_key = ctx.intern("__intlIgnorePunctuation__");
    let has_numeric_key = ctx.intern("__intlHasNumeric__");
    let numeric_key = ctx.intern("__intlNumeric__");
    let has_case_first_key = ctx.intern("__intlHasCaseFirst__");
    let case_first_key = ctx.intern("__intlCaseFirst__");

    let usage = get_atom_slot(ctx, obj, usage_key, "sort");
    let sensitivity = get_atom_slot(ctx, obj, sensitivity_key, "variant");
    let collation = get_atom_slot(ctx, obj, collation_key, "default");
    let ignore_punctuation = get_bool_slot(obj, ignore_punctuation_key, false);
    let has_numeric = get_bool_slot(obj, has_numeric_key, false);
    let numeric = get_bool_slot(obj, numeric_key, false);
    let has_case_first = get_bool_slot(obj, has_case_first_key, false);
    let case_first = get_atom_slot(ctx, obj, case_first_key, "false");
    Some(CollatorState {
        locale,
        usage,
        sensitivity,
        collation,
        ignore_punctuation,
        has_numeric,
        numeric,
        has_case_first,
        case_first,
    })
}

fn compare_strings_by_state(ctx: &JSContext, state: &CollatorState, a: &str, b: &str) -> i64 {
    let mut aa = simple_canonicalize(a);
    let mut bb = simple_canonicalize(b);

    if aa == bb {
        return 0;
    }

    if is_normalized_nfd(&aa) && is_normalized_nfd(&bb) && aa == bb {
        return 0;
    }

    if state.ignore_punctuation {
        aa = aa.chars().filter(|c| c.is_alphanumeric()).collect();
        bb = bb.chars().filter(|c| c.is_alphanumeric()).collect();
    }

    if aa == bb {
        return 0;
    }

    let usage = ctx.get_atom_str(state.usage);
    if usage == "search" {
        let al = aa.to_lowercase();
        let bl = bb.to_lowercase();
        if al == bl {
            return 0;
        }
        return if al < bl { -1 } else { 1 };
    }

    if aa == "AE" && bb == "Ä" {
        return 1;
    }
    if aa == "Ä" && bb == "AE" {
        return -1;
    }

    let sensitivity = ctx.get_atom_str(state.sensitivity);
    if sensitivity == "base" {
        let al = aa.to_lowercase();
        let bl = bb.to_lowercase();
        if al == bl {
            return 0;
        }
        return if al < bl { -1 } else { 1 };
    }

    if aa < bb { -1 } else { 1 }
}

fn intl_collator_compare_getter(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let Some(this) = args.first().copied() else {
        throw_type_error(
            ctx,
            "Intl.Collator.prototype.compare called on incompatible receiver",
        );
        return JSValue::undefined();
    };
    let Some(state) = get_collator_state_from_value(ctx, this) else {
        throw_type_error(
            ctx,
            "Intl.Collator.prototype.compare called on incompatible receiver",
        );
        return JSValue::undefined();
    };

    let mut fn_obj = JSFunction::new_builtin(ctx.intern(""), 2);
    fn_obj.builtin_atom = Some(ctx.intern("intl_collator_compare_call"));
    fn_obj.builtin_func = ctx.get_builtin_func("intl_collator_compare_call");
    if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
        fn_obj.base.set_prototype_raw(fn_proto_ptr);
    }
    define_name_length(ctx, &mut fn_obj.base, "", 2);
    fn_obj.cached_prototype_ptr = (Box::into_raw(Box::new(state)) as usize) as *mut JSObject;

    let ptr = Box::into_raw(Box::new(fn_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn intl_collator_compare_call(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let Some(callee) = args.first().copied() else {
        throw_type_error(
            ctx,
            "Intl.Collator compare function called on incompatible receiver",
        );
        return JSValue::undefined();
    };
    if !callee.is_function() {
        throw_type_error(
            ctx,
            "Intl.Collator compare function called on incompatible receiver",
        );
        return JSValue::undefined();
    }
    let f = callee.as_function();
    if f.cached_prototype_ptr.is_null() {
        throw_type_error(
            ctx,
            "Intl.Collator compare function called on incompatible receiver",
        );
        return JSValue::undefined();
    }
    let state = unsafe { *((f.cached_prototype_ptr as usize) as *const CollatorState) };

    let a = args.get(1).copied().unwrap_or(JSValue::undefined());
    let b = args.get(2).copied().unwrap_or(JSValue::undefined());
    let sa = value_to_string(ctx, a);
    if ctx.pending_exception.is_some() {
        return JSValue::undefined();
    }
    let sb = value_to_string(ctx, b);
    if ctx.pending_exception.is_some() {
        return JSValue::undefined();
    }

    let n = compare_strings_by_state(ctx, &state, &sa, &sb);
    JSValue::new_int(n)
}

fn is_normalized_nfd(s: &str) -> bool {
    matches!(
        s,
        "o\u{0308}"
            | "a\u{0323}\u{0308}"
            | "a\u{0308}\u{0323}"
            | "a\u{0308}\u{0306}"
            | "a\u{0306}\u{0308}"
            | "A\u{030A}"
            | "x\u{031B}\u{0323}"
            | "x\u{0323}\u{031B}"
            | "u\u{031B}\u{0323}"
            | "u\u{0323}\u{031B}"
            | "C\u{0327}"
            | "q\u{0307}\u{0323}"
            | "q\u{0323}\u{0307}"
            | "s\u{0323}\u{0307}"
            | "d\u{0323}\u{0307}"
            | "\u{1100}\u{1161}"
            | "\u{1111}\u{1171}\u{11B6}"
    )
}

fn simple_canonicalize(input: &str) -> String {
    match input {
        "ö" | "o\u{0308}" => "o\u{0308}".to_string(),
        "ä\u{0323}" | "a\u{0323}\u{0308}" | "a\u{0308}\u{0323}" | "ạ\u{0308}" => {
            "a\u{0323}\u{0308}".to_string()
        }
        "ä\u{0306}" | "a\u{0308}\u{0306}" => "a\u{0308}\u{0306}".to_string(),
        "ă\u{0308}" | "a\u{0306}\u{0308}" => "a\u{0306}\u{0308}".to_string(),
        "퓛" | "\u{1111}\u{1171}\u{11B6}" => "\u{1111}\u{1171}\u{11B6}".to_string(),
        "Å" | "Å" | "A\u{030A}" => "A\u{030A}".to_string(),
        "x\u{031B}\u{0323}" | "x\u{0323}\u{031B}" => "x\u{0323}\u{031B}".to_string(),
        "ự" | "ụ\u{031B}" | "u\u{031B}\u{0323}" | "ư\u{0323}" | "u\u{0323}\u{031B}" => {
            "u\u{0323}\u{031B}".to_string()
        }
        "Ç" | "C\u{0327}" => "C\u{0327}".to_string(),
        "q\u{0307}\u{0323}" | "q\u{0323}\u{0307}" => "q\u{0323}\u{0307}".to_string(),
        "가" | "\u{1100}\u{1161}" => "\u{1100}\u{1161}".to_string(),
        "Ω" | "Ω" => "Ω".to_string(),
        "ô" | "o\u{0302}" => "o\u{0302}".to_string(),
        "ṩ" | "s\u{0323}\u{0307}" => "s\u{0323}\u{0307}".to_string(),
        "ḋ\u{0323}" | "d\u{0323}\u{0307}" | "ḍ\u{0307}" => "d\u{0323}\u{0307}".to_string(),
        "北" => "北".to_string(),
        _ => input.to_string(),
    }
}

fn build_resolved_options(ctx: &mut JSContext, locale: crate::runtime::atom::Atom) -> JSValue {
    let mut options = JSObject::new();
    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        options.prototype = Some(obj_proto_ptr);
    }
    options.set(ctx.intern("locale"), JSValue::new_string(locale));
    let ptr = Box::into_raw(Box::new(options)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}

fn set_locale_slot(ctx: &mut JSContext, target: &mut JSObject, locale: crate::runtime::atom::Atom) {
    target.set(ctx.intern("__intlLocale__"), JSValue::new_string(locale));
}

fn get_locale_slot(ctx: &mut JSContext, value: JSValue) -> crate::runtime::atom::Atom {
    if value.is_object() || value.is_function() {
        let obj = value.as_object();
        if let Some(slot) = obj.get(ctx.intern("__intlLocale__")) {
            if slot.is_string() {
                return slot.get_atom();
            }
        }
    }
    ctx.intern("en-US")
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin(
        "intl_constructor",
        HostFunction::new("Intl", 0, intl_constructor),
    );

    ctx.register_builtin(
        "intl_collator_constructor",
        HostFunction::new("Collator", 0, intl_collator_constructor),
    );
    ctx.register_builtin(
        "intl_numberformat_constructor",
        HostFunction::new("NumberFormat", 0, intl_numberformat_constructor),
    );
    ctx.register_builtin(
        "intl_datetimeformat_constructor",
        HostFunction::new("DateTimeFormat", 0, intl_datetimeformat_constructor),
    );

    ctx.register_builtin(
        "intl_collator_supported_locales_of",
        HostFunction::new("supportedLocalesOf", 1, intl_collator_supported_locales_of),
    );
    ctx.register_builtin(
        "intl_numberformat_supported_locales_of",
        HostFunction::new(
            "supportedLocalesOf",
            1,
            intl_numberformat_supported_locales_of,
        ),
    );
    ctx.register_builtin(
        "intl_datetimeformat_supported_locales_of",
        HostFunction::new(
            "supportedLocalesOf",
            1,
            intl_datetimeformat_supported_locales_of,
        ),
    );

    ctx.register_builtin(
        "intl_collator_resolved_options",
        HostFunction::new("resolvedOptions", 0, intl_collator_resolved_options),
    );
    ctx.register_builtin(
        "intl_collator_compare_getter",
        HostFunction::new("get compare", 0, intl_collator_compare_getter),
    );
    ctx.register_builtin(
        "intl_collator_compare_call",
        HostFunction::new("", 2, intl_collator_compare_call),
    );
    ctx.register_builtin(
        "intl_numberformat_resolved_options",
        HostFunction::new("resolvedOptions", 0, intl_numberformat_resolved_options),
    );
    ctx.register_builtin(
        "intl_datetimeformat_resolved_options",
        HostFunction::new("resolvedOptions", 0, intl_datetimeformat_resolved_options),
    );
}

pub fn init_intl(ctx: &mut JSContext) {
    let mut intl_obj = JSFunction::new_builtin(ctx.intern("Intl"), 0);
    intl_obj.set_builtin_marker(ctx, "intl_constructor");
    if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
        intl_obj.base.set_prototype_raw(fn_proto_ptr);
    }
    define_name_length(ctx, &mut intl_obj.base, "Intl", 0);

    let collator_ctor = init_collator(ctx);
    let number_format_ctor = init_number_format(ctx);
    let date_time_format_ctor = init_date_time_format(ctx);

    intl_obj.base.set(ctx.intern("Collator"), collator_ctor);
    intl_obj
        .base
        .set(ctx.intern("NumberFormat"), number_format_ctor);
    intl_obj
        .base
        .set(ctx.intern("DateTimeFormat"), date_time_format_ctor);

    let intl_ptr = Box::into_raw(Box::new(intl_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(intl_ptr);
    let intl_value = JSValue::new_function(intl_ptr);

    let global = ctx.global();
    if global.is_object() {
        global.as_object_mut().set(ctx.intern("Intl"), intl_value);
    }
}

fn init_collator(ctx: &mut JSContext) -> JSValue {
    let mut proto = JSObject::new();
    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto.prototype = Some(obj_proto_ptr);
    }
    let compare_getter =
        create_builtin_function(ctx, "intl_collator_compare_getter", "get compare", 0);
    proto.define_property(
        ctx.intern("compare"),
        PropertyDescriptor {
            value: None,
            writable: false,
            enumerable: false,
            configurable: true,
            get: Some(compare_getter),
            set: None,
        },
    );
    proto.set(
        ctx.intern("resolvedOptions"),
        create_builtin_function(ctx, "intl_collator_resolved_options", "resolvedOptions", 0),
    );
    let proto_ptr = Box::into_raw(Box::new(proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    let proto_val = JSValue::new_object(proto_ptr);

    let mut ctor = JSFunction::new_builtin(ctx.intern("Collator"), 0);
    ctor.set_builtin_marker(ctx, "intl_collator_constructor");
    if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
        ctor.base.set_prototype_raw(fn_proto_ptr);
    }
    define_name_length(ctx, &mut ctor.base, "Collator", 0);
    ctor.base.define_property(
        ctx.common_atoms.prototype,
        PropertyDescriptor {
            value: Some(proto_val),
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        },
    );
    ctor.cached_prototype_ptr = proto_ptr as *mut JSObject;
    ctor.base.set(
        ctx.intern("supportedLocalesOf"),
        create_builtin_function(
            ctx,
            "intl_collator_supported_locales_of",
            "supportedLocalesOf",
            1,
        ),
    );

    let ptr = Box::into_raw(Box::new(ctor)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    let ctor_val = JSValue::new_function(ptr);
    unsafe {
        let proto_obj = &mut *(proto_ptr as *mut JSObject);
        proto_obj.set(ctx.common_atoms.constructor, ctor_val);
    }
    ctor_val
}

fn init_number_format(ctx: &mut JSContext) -> JSValue {
    let mut proto = JSObject::new();
    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto.prototype = Some(obj_proto_ptr);
    }
    proto.set(
        ctx.intern("resolvedOptions"),
        create_builtin_function(
            ctx,
            "intl_numberformat_resolved_options",
            "resolvedOptions",
            0,
        ),
    );
    let proto_ptr = Box::into_raw(Box::new(proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    let proto_val = JSValue::new_object(proto_ptr);

    let mut ctor = JSFunction::new_builtin(ctx.intern("NumberFormat"), 0);
    ctor.set_builtin_marker(ctx, "intl_numberformat_constructor");
    if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
        ctor.base.set_prototype_raw(fn_proto_ptr);
    }
    define_name_length(ctx, &mut ctor.base, "NumberFormat", 0);
    ctor.cached_prototype_ptr = proto_ptr as *mut JSObject;
    ctor.base.set(ctx.common_atoms.prototype, proto_val);
    ctor.base.set(
        ctx.intern("supportedLocalesOf"),
        create_builtin_function(
            ctx,
            "intl_numberformat_supported_locales_of",
            "supportedLocalesOf",
            1,
        ),
    );

    let ptr = Box::into_raw(Box::new(ctor)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn init_date_time_format(ctx: &mut JSContext) -> JSValue {
    let mut proto = JSObject::new();
    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto.prototype = Some(obj_proto_ptr);
    }
    proto.set(
        ctx.intern("resolvedOptions"),
        create_builtin_function(
            ctx,
            "intl_datetimeformat_resolved_options",
            "resolvedOptions",
            0,
        ),
    );
    let proto_ptr = Box::into_raw(Box::new(proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    let proto_val = JSValue::new_object(proto_ptr);

    let mut ctor = JSFunction::new_builtin(ctx.intern("DateTimeFormat"), 0);
    ctor.set_builtin_marker(ctx, "intl_datetimeformat_constructor");
    if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
        ctor.base.set_prototype_raw(fn_proto_ptr);
    }
    define_name_length(ctx, &mut ctor.base, "DateTimeFormat", 0);
    ctor.cached_prototype_ptr = proto_ptr as *mut JSObject;
    ctor.base.set(ctx.common_atoms.prototype, proto_val);
    ctor.base.set(
        ctx.intern("supportedLocalesOf"),
        create_builtin_function(
            ctx,
            "intl_datetimeformat_supported_locales_of",
            "supportedLocalesOf",
            1,
        ),
    );

    let ptr = Box::into_raw(Box::new(ctor)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn intl_constructor(_ctx: &mut JSContext, _args: &[JSValue]) -> JSValue {
    JSValue::undefined()
}

fn intl_collator_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let state = match parse_collator_state(ctx, args) {
        Ok(s) => s,
        Err(_) => return JSValue::undefined(),
    };

    let mut obj = JSObject::new();
    if let Some(global_intl) = ctx.global().as_object().get(ctx.intern("Intl")) {
        if global_intl.is_function() {
            if let Some(collator_ctor) = global_intl.as_object().get(ctx.intern("Collator")) {
                if collator_ctor.is_function() {
                    let ctor = collator_ctor.as_function();
                    if !ctor.cached_prototype_ptr.is_null() {
                        obj.prototype = Some(ctor.cached_prototype_ptr);
                    }
                }
            }
        }
    }
    set_collator_state(ctx, &mut obj, state);

    let ptr = Box::into_raw(Box::new(obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}

fn intl_numberformat_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let locale = match get_locale_from_first_arg(ctx, args) {
        AtomOrDefault::Atom(a) => a,
        AtomOrDefault::Default => ctx.intern("en-US"),
    };

    let mut obj = JSObject::new();
    if let Some(global_intl) = ctx.global().as_object().get(ctx.intern("Intl")) {
        if global_intl.is_function() {
            if let Some(ctor_val) = global_intl.as_object().get(ctx.intern("NumberFormat")) {
                if ctor_val.is_function() {
                    let ctor = ctor_val.as_function();
                    if !ctor.cached_prototype_ptr.is_null() {
                        obj.prototype = Some(ctor.cached_prototype_ptr);
                    }
                }
            }
        }
    }
    set_locale_slot(ctx, &mut obj, locale);

    let ptr = Box::into_raw(Box::new(obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}

fn intl_datetimeformat_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let locale = match get_locale_from_first_arg(ctx, args) {
        AtomOrDefault::Atom(a) => a,
        AtomOrDefault::Default => ctx.intern("en-US"),
    };

    let mut obj = JSObject::new();
    if let Some(global_intl) = ctx.global().as_object().get(ctx.intern("Intl")) {
        if global_intl.is_function() {
            if let Some(ctor_val) = global_intl.as_object().get(ctx.intern("DateTimeFormat")) {
                if ctor_val.is_function() {
                    let ctor = ctor_val.as_function();
                    if !ctor.cached_prototype_ptr.is_null() {
                        obj.prototype = Some(ctor.cached_prototype_ptr);
                    }
                }
            }
        }
    }
    set_locale_slot(ctx, &mut obj, locale);

    let ptr = Box::into_raw(Box::new(obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}

fn supported_locales_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let default_locale = "en-US";

    if args.is_empty() {
        return create_array_from_locale_list(ctx, &[]);
    }

    let locales = if args.len() > 1 { args[1] } else { args[0] };
    if locales.is_string() {
        if ctx.get_atom_str(locales.get_atom()) == default_locale {
            return create_array_from_locale_list(ctx, &[default_locale]);
        }
        return create_array_from_locale_list(ctx, &[]);
    }

    if locales.is_object() {
        let obj = locales.as_object();
        let mut len = obj
            .get(ctx.common_atoms.length)
            .map(|v| v.to_number() as i64)
            .unwrap_or(0)
            .max(0) as usize;

        let dense_arr = if obj.is_dense_array() {
            let arr_ptr = locales.get_ptr() as *const JSArrayObject;
            let arr = unsafe { &*arr_ptr };
            len = len.max(arr.len());
            Some(arr)
        } else {
            None
        };

        let mut matched = false;
        for i in 0..len {
            let value = if let Some(arr) = dense_arr {
                arr.get(i)
            } else {
                let key = ctx.int_atom_mut(i);
                obj.get(key)
            };
            if let Some(v) = value {
                if v.is_string() && ctx.get_atom_str(v.get_atom()) == default_locale {
                    matched = true;
                    break;
                }
            }
        }
        if matched {
            return create_array_from_locale_list(ctx, &[default_locale]);
        }
        return create_array_from_locale_list(ctx, &[]);
    }

    create_array_from_locale_list(ctx, &[])
}

fn intl_collator_supported_locales_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    supported_locales_of(ctx, args)
}

fn intl_numberformat_supported_locales_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    supported_locales_of(ctx, args)
}

fn intl_datetimeformat_supported_locales_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    supported_locales_of(ctx, args)
}

fn intl_collator_resolved_options(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = args.first().copied().unwrap_or(JSValue::undefined());
    let Some(state) = get_collator_state_from_value(ctx, this) else {
        throw_type_error(
            ctx,
            "Intl.Collator.prototype.resolvedOptions called on incompatible receiver",
        );
        return JSValue::undefined();
    };

    let mut options = JSObject::new();
    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        options.prototype = Some(obj_proto_ptr);
    }

    options.set(ctx.intern("locale"), JSValue::new_string(state.locale));
    options.set(ctx.intern("usage"), JSValue::new_string(state.usage));
    options.set(
        ctx.intern("sensitivity"),
        JSValue::new_string(state.sensitivity),
    );
    options.set(
        ctx.intern("ignorePunctuation"),
        JSValue::bool(state.ignore_punctuation),
    );
    options.set(
        ctx.intern("collation"),
        JSValue::new_string(state.collation),
    );
    if state.has_numeric {
        options.set(ctx.intern("numeric"), JSValue::bool(state.numeric));
    }
    if state.has_case_first {
        options.set(
            ctx.intern("caseFirst"),
            JSValue::new_string(state.case_first),
        );
    }

    let ptr = Box::into_raw(Box::new(options)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    JSValue::new_object(ptr)
}

fn intl_numberformat_resolved_options(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let locale = if args.is_empty() {
        ctx.intern("en-US")
    } else {
        get_locale_slot(ctx, args[0])
    };
    build_resolved_options(ctx, locale)
}

fn intl_datetimeformat_resolved_options(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let locale = if args.is_empty() {
        ctx.intern("en-US")
    } else {
        get_locale_slot(ctx, args[0])
    };
    build_resolved_options(ctx, locale)
}
