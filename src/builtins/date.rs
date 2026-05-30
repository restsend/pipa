use crate::builtins::global::set_non_enumerable;
use crate::host::HostFunction;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;
use std::time::{SystemTime, UNIX_EPOCH};

fn current_timestamp_ms() -> f64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (now.as_secs() as f64 * 1000.0) + (now.subsec_nanos() as f64 / 1_000_000.0)
}

fn parse_date_string(s: &str) -> f64 {
    let s = s.trim();
    if s.starts_with('+') || s.starts_with('-') || s.contains('.') || s.contains('e') || s.contains('E') {
        if let Ok(ts) = s.parse::<f64>() {
            return ts;
        }
    }
    if let Some(rest) = s.strip_prefix("YYYY-") {
    }
    let parts: Vec<&str> = if s.contains('T') {
        s.splitn(2, 'T').collect()
    } else if s.contains(' ') {
        s.splitn(2, ' ').collect()
    } else {
        vec![s, ""]
    };
    let date_part = parts[0];
    let time_part = if parts.len() > 1 { parts[1] } else { "" };

    let (year, month, day) = if date_part.starts_with('-') {
        let after = &date_part[1..];
        let dp: Vec<&str> = after.splitn(3, '-').collect();
        let raw_y: i32 = dp[0].parse().unwrap_or(0);
        if raw_y == 0 { return f64::NAN; }
        let y = -raw_y;
        let m = if dp.len() > 1 { dp[1].parse::<u32>().unwrap_or(1) } else { 1 };
        let d = if dp.len() > 2 { dp[2].parse::<u32>().unwrap_or(1) } else { 1 };
        (y, m, d)
    } else if date_part.contains('-') {
        let dp: Vec<&str> = date_part.split('-').collect();
        if dp.len() < 1 {
            return f64::NAN;
        }
        let y = dp[0].parse::<i32>().unwrap_or(0);
        let m = if dp.len() > 1 { dp[1].parse::<u32>().unwrap_or(1) } else { 1 };
        let d = if dp.len() > 2 { dp[2].parse::<u32>().unwrap_or(1) } else { 1 };
        (y, m, d)
    } else {
        let y = date_part.parse::<i32>().unwrap_or(0);
        (y, 1, 1)
    };

    let (hour, minute, second, ms, tz_offset) = if time_part.is_empty() {
        (0u32, 0u32, 0u32, 0u32, 0i64)
    } else {
        let (time_str, tz) = if time_part.ends_with('Z') {
            (&time_part[..time_part.len() - 1], 0i64)
        } else if let Some(idx) = time_part.rfind('+') {
            let sign = 1i64;
            let offset_str = &time_part[idx + 1..];
            let off = parse_tz_offset(offset_str, sign);
            (&time_part[..idx], off)
        } else if let Some(idx) = time_part.rfind('-') {
            if idx > 0 && time_part.chars().nth(idx - 1) != Some(':') && idx > 5 {
                let sign = -1i64;
                let offset_str = &time_part[idx + 1..];
                let off = parse_tz_offset(offset_str, sign);
                (&time_part[..idx], off)
            } else {
                (time_part, 0)
            }
        } else {
            (time_part, 0)
        };

        let tp: Vec<&str> = time_str.split(':').collect();
        let h = tp.first().and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        let mi = tp.get(1).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        let sec_str = tp.get(2).map(|v| *v).unwrap_or("0");
        let (sec, msec) = if sec_str.contains('.') {
            let sp: Vec<&str> = sec_str.split('.').collect();
            let s = sp[0].parse::<u32>().unwrap_or(0);
            let ms = sp.get(1).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
            (s, ms)
        } else {
            (sec_str.parse::<u32>().unwrap_or(0), 0)
        };
        (h, mi, sec, msec, tz)
    };

    let ts = make_day_from_parts(year, month, day.max(1), hour, minute, second, ms);
    ts - (tz_offset as f64 * 60000.0)
}

fn parse_tz_offset(s: &str, sign: i64) -> i64 {
    let parts: Vec<&str> = s.split(':').collect();
    let h = parts.first().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    let m = parts.get(1).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    sign * (h * 60 + m)
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin("date_now", HostFunction::new("now", 0, date_now));
    ctx.register_builtin("date_parse", HostFunction::new("parse", 1, date_parse));
    ctx.register_builtin("date_utc", HostFunction::new("UTC", 7, date_utc));
    ctx.register_builtin(
        "date_constructor",
        HostFunction::ctor("Date", 7, date_constructor),
    );
    ctx.register_builtin(
        "date_getTime",
        HostFunction::method("getTime", 0, date_get_time),
    );
    ctx.register_builtin(
        "date_getFullYear",
        HostFunction::method("getFullYear", 0, date_get_full_year),
    );
    ctx.register_builtin(
        "date_getMonth",
        HostFunction::method("getMonth", 0, date_get_month),
    );
    ctx.register_builtin(
        "date_getDate",
        HostFunction::method("getDate", 0, date_get_date),
    );
    ctx.register_builtin("date_getDay", HostFunction::method("getDay", 0, date_get_day));
    ctx.register_builtin(
        "date_getHours",
        HostFunction::method("getHours", 0, date_get_hours),
    );
    ctx.register_builtin(
        "date_getMinutes",
        HostFunction::method("getMinutes", 0, date_get_minutes),
    );
    ctx.register_builtin(
        "date_getSeconds",
        HostFunction::method("getSeconds", 0, date_get_seconds),
    );
    ctx.register_builtin(
        "date_getMilliseconds",
        HostFunction::method("getMilliseconds", 0, date_get_milliseconds),
    );
    ctx.register_builtin(
        "date_getTimezoneOffset",
        HostFunction::method("getTimezoneOffset", 0, date_get_timezone_offset),
    );
    ctx.register_builtin(
        "date_getUTCFullYear",
        HostFunction::method("getUTCFullYear", 0, date_get_utc_full_year),
    );
    ctx.register_builtin(
        "date_getUTCMonth",
        HostFunction::method("getUTCMonth", 0, date_get_utc_month),
    );
    ctx.register_builtin(
        "date_getUTCDate",
        HostFunction::method("getUTCDate", 0, date_get_utc_date),
    );
    ctx.register_builtin(
        "date_getUTCDay",
        HostFunction::method("getUTCDay", 0, date_get_utc_day),
    );
    ctx.register_builtin(
        "date_getUTCHours",
        HostFunction::method("getUTCHours", 0, date_get_utc_hours),
    );
    ctx.register_builtin(
        "date_getUTCMinutes",
        HostFunction::method("getUTCMinutes", 0, date_get_utc_minutes),
    );
    ctx.register_builtin(
        "date_getUTCSeconds",
        HostFunction::method("getUTCSeconds", 0, date_get_utc_seconds),
    );
    ctx.register_builtin(
        "date_getUTCMilliseconds",
        HostFunction::method("getUTCMilliseconds", 0, date_get_utc_milliseconds),
    );
    ctx.register_builtin(
        "date_toString",
        HostFunction::method("toString", 0, date_to_string),
    );
    ctx.register_builtin(
        "date_toISOString",
        HostFunction::method("toISOString", 0, date_to_iso_string),
    );
    ctx.register_builtin(
        "date_toUTCString",
        HostFunction::method("toUTCString", 0, date_to_utc_string),
    );
    ctx.register_builtin(
        "date_toDateString",
        HostFunction::method("toDateString", 0, date_to_date_string),
    );
    ctx.register_builtin(
        "date_toTimeString",
        HostFunction::method("toTimeString", 0, date_to_time_string),
    );
    ctx.register_builtin(
        "date_valueOf",
        HostFunction::method("valueOf", 0, date_value_of),
    );
    ctx.register_builtin(
        "date_toJSON",
        HostFunction::method("toJSON", 1, date_to_json),
    );
}

pub fn init_date(ctx: &mut JSContext) {
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

    let date_atom = ctx.intern("Date");

    let mut date_func = JSFunction::new_builtin(date_atom, 7);
    date_func.set_builtin_marker(ctx, "date_constructor");

    crate::builtins::global::set_non_enumerable(&mut date_func.base, ctx.intern("now"), create_builtin_function(ctx, "date_now", 0, None));
    crate::builtins::global::set_non_enumerable(&mut date_func.base, ctx.intern("parse"), create_builtin_function(ctx, "date_parse", 1, None));
    crate::builtins::global::set_non_enumerable(&mut date_func.base, ctx.intern("UTC"), create_builtin_function(ctx, "date_utc", 7, None));

    let date_ptr = Box::into_raw(Box::new(date_func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(date_ptr);
    let date_value = JSValue::new_function(date_ptr);

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, date_atom, date_value);
    }

    let proto_atom = ctx.intern("DatePrototype");
    let mut proto_obj = JSObject::new();

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        proto_obj.prototype = Some(obj_proto_ptr);
    }

    set_ne(&mut proto_obj,
        ctx.intern("constructor"),
        date_value,
    );

    set_ne(&mut proto_obj,
        ctx.intern("getTime"),
        create_builtin_function(ctx, "date_getTime", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("getFullYear"),
        create_builtin_function(ctx, "date_getFullYear", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("getMonth"),
        create_builtin_function(ctx, "date_getMonth", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("getDate"),
        create_builtin_function(ctx, "date_getDate", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("getDay"),
        create_builtin_function(ctx, "date_getDay", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("getHours"),
        create_builtin_function(ctx, "date_getHours", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("getMinutes"),
        create_builtin_function(ctx, "date_getMinutes", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("getSeconds"),
        create_builtin_function(ctx, "date_getSeconds", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("getMilliseconds"),
        create_builtin_function(ctx, "date_getMilliseconds", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("getTimezoneOffset"),
        create_builtin_function(ctx, "date_getTimezoneOffset", 0, None),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("getUTCFullYear"),
        create_builtin_function(ctx, "date_getUTCFullYear", 0, None),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("getUTCMonth"),
        create_builtin_function(ctx, "date_getUTCMonth", 0, None),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("getUTCDate"),
        create_builtin_function(ctx, "date_getUTCDate", 0, None),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("getUTCDay"),
        create_builtin_function(ctx, "date_getUTCDay", 0, None),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("getUTCHours"),
        create_builtin_function(ctx, "date_getUTCHours", 0, None),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("getUTCMinutes"),
        create_builtin_function(ctx, "date_getUTCMinutes", 0, None),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("getUTCSeconds"),
        create_builtin_function(ctx, "date_getUTCSeconds", 0, None),
    );
    set_ne(
        &mut proto_obj,
        ctx.intern("getUTCMilliseconds"),
        create_builtin_function(ctx, "date_getUTCMilliseconds", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setTime"),
        create_builtin_function(ctx, "date_setTime", 1, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setMilliseconds"),
        create_builtin_function(ctx, "date_setMilliseconds", 1, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setSeconds"),
        create_builtin_function(ctx, "date_setSeconds", 2, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setMinutes"),
        create_builtin_function(ctx, "date_setMinutes", 3, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setHours"),
        create_builtin_function(ctx, "date_setHours", 4, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setDate"),
        create_builtin_function(ctx, "date_setDate", 1, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setMonth"),
        create_builtin_function(ctx, "date_setMonth", 2, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setFullYear"),
        create_builtin_function(ctx, "date_setFullYear", 3, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setUTCMilliseconds"),
        create_builtin_function(ctx, "date_setUTCMilliseconds", 1, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setUTCSeconds"),
        create_builtin_function(ctx, "date_setUTCSeconds", 2, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setUTCMinutes"),
        create_builtin_function(ctx, "date_setUTCMinutes", 3, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setUTCHours"),
        create_builtin_function(ctx, "date_setUTCHours", 4, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setUTCDate"),
        create_builtin_function(ctx, "date_setUTCDate", 1, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setUTCMonth"),
        create_builtin_function(ctx, "date_setUTCMonth", 2, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("setUTCFullYear"),
        create_builtin_function(ctx, "date_setUTCFullYear", 3, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("toString"),
        create_builtin_function(ctx, "date_toString", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("toISOString"),
        create_builtin_function(ctx, "date_toISOString", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("toUTCString"),
        create_builtin_function(ctx, "date_toUTCString", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("toDateString"),
        create_builtin_function(ctx, "date_toDateString", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("toTimeString"),
        create_builtin_function(ctx, "date_toTimeString", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("valueOf"),
        create_builtin_function(ctx, "date_valueOf", 0, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("toJSON"),
        create_builtin_function(ctx, "date_toJSON", 1, None),
    );
    set_ne(&mut proto_obj,
        ctx.intern("toLocaleString"),
        create_builtin_function(ctx, "date_to_string", 0, Some("toLocaleString")),
    );
    set_ne(&mut proto_obj,
        ctx.intern("toLocaleDateString"),
        create_builtin_function(ctx, "date_to_date_string", 0, Some("toLocaleDateString")),
    );
    set_ne(&mut proto_obj,
        ctx.intern("toLocaleTimeString"),
        create_builtin_function(ctx, "date_to_time_string", 0, Some("toLocaleTimeString")),
    );

    let proto_ptr = Box::into_raw(Box::new(proto_obj)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
    ctx.set_date_prototype(proto_ptr);
    let proto_value = JSValue::new_object(proto_ptr);

    if global.is_object() {
        let global_obj = global.as_object_mut();
        crate::builtins::global::set_non_enumerable(global_obj, proto_atom, proto_value);
    }

    let date_func_ref = date_value.as_function_mut();
    date_func_ref.base.define_property(ctx.intern("prototype"), crate::object::object::PropertyDescriptor {
        value: Some(proto_value),
        writable: false,
        enumerable: false,
        configurable: false,
        get: None,
        set: None,
    });
}

fn create_builtin_function(ctx: &mut JSContext, internal_name: &str, arity: u32, display_name: Option<&str>) -> JSValue {
    let mut func = crate::object::function::JSFunction::new_builtin(ctx.intern(internal_name), arity);
    func.set_builtin_marker(ctx, internal_name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    if let Some(dname) = display_name {
        let func_obj = unsafe { &mut *(ptr as *mut crate::object::function::JSFunction) };
        func_obj.base.define_property(ctx.intern("name"), crate::object::object::PropertyDescriptor {
            value: Some(JSValue::new_string(ctx.intern(dname))),
            writable: false,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        });
    }
    JSValue::new_function(ptr)
}

pub fn date_now(_ctx: &mut JSContext, _args: &[JSValue]) -> JSValue {
    JSValue::new_float(current_timestamp_ms())
}

pub fn date_parse(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let args = if !args.is_empty() && args[0].is_function() { &args[1..] } else { args };
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    }

    let date_str = if args[0].is_string() {
        ctx.get_atom_str(args[0].get_atom()).to_string()
    } else {
        return JSValue::new_float(f64::NAN);
    };

    if let Ok(ts) = parse_iso_date(&date_str) {
        let clipped = time_clip(ts);
        return JSValue::new_float(if clipped.is_nan() { f64::NAN } else { ts });
    }

    if let Ok(ts) = parse_common_date(&date_str) {
        let clipped = time_clip(ts);
        return JSValue::new_float(if clipped.is_nan() { f64::NAN } else { ts });
    }

    let ts = parse_date_string(&date_str);
    if !ts.is_nan() {
        let clipped = time_clip(ts);
        return JSValue::new_float(if clipped.is_nan() { f64::NAN } else { ts });
    }

    JSValue::new_float(f64::NAN)
}

fn parse_iso_date(s: &str) -> Result<f64, ()> {
    let s = s.trim();
    let parts: Vec<&str> = s.split('T').collect();
    let date_part = parts[0];

    let (year, rest_date) = if date_part.starts_with('-') {
        let after_sign = &date_part[1..];
        let neg_parts: Vec<&str> = after_sign.splitn(2, '-').collect();
        if neg_parts.len() != 2 {
            let y: i32 = after_sign.parse().map_err(|_| ())?;
            if y == 0 { return Err(()); }
            return Ok(make_day_from_parts(-y, 1, 1, 0, 0, 0, 0));
        }
        let y: i32 = neg_parts[0].parse().map_err(|_| ())?;
        if y == 0 { return Err(()); }
        (-y, neg_parts[1])
    } else {
        let pos_parts: Vec<&str> = date_part.splitn(2, '-').collect();
        if pos_parts.len() != 2 {
            let y: i32 = date_part.parse().map_err(|_| ())?;
            return Ok(make_day_from_parts(y, 1, 1, 0, 0, 0, 0));
        }
        let y: i32 = pos_parts[0].parse().map_err(|_| ())?;
        (y, pos_parts[1])
    };

    let rest_parts: Vec<&str> = rest_date.split('-').collect();
    if rest_parts.len() != 2 {
        return Err(());
    }
    let month: u32 = rest_parts[0].parse().map_err(|_| ())?;
    let day: u32 = rest_parts[1].parse().map_err(|_| ())?;

    let mut hour: u32 = 0;
    let mut minute: u32 = 0;
    let mut second: u32 = 0;
    let mut ms: u32 = 0;

    if parts.len() > 1 {
        let time_part = parts[1].trim_end_matches('Z');
        let time_components: Vec<&str> = time_part.split(':').collect();
        if time_components.len() >= 2 {
            hour = time_components[0].parse().map_err(|_| ())?;
            minute = time_components[1].parse().map_err(|_| ())?;
            if time_components.len() >= 3 {
                let sec_str = time_components[2];
                let sec_parts: Vec<&str> = sec_str.split('.').collect();
                second = sec_parts[0].parse().map_err(|_| ())?;
                if sec_parts.len() > 1 {
                    let frac = sec_parts[1];
                    if frac.len() >= 3 {
                        ms = frac[..3].parse().map_err(|_| ())?;
                    } else {
                        let padded = format!("{:0<3}", frac);
                        ms = padded[..3].parse().map_err(|_| ())?;
                    }
                }
            }
        }
    }

    Ok(make_day_from_parts(year, month, day, hour, minute, second, ms))
}

fn parse_common_date(s: &str) -> Result<f64, ()> {
    let s = s.trim();
    let parts: Vec<&str> = s.split_whitespace().collect();

    if parts.len() >= 3 {
        let mut year: i32 = 0;
        let mut month: u32 = 0;
        let mut day: u32 = 0;
        let hour: u32 = 0;
        let minute: u32 = 0;
        let second: u32 = 0;

        for part in &parts {
            if let Ok(y) = part.parse::<i32>() {
                if y > 1000 {
                    year = y;
                } else if y > 0 && y <= 31 {
                    day = y as u32;
                }
            } else if let Ok(d) = part.parse::<u32>() {
                if d > 0 && d <= 31 {
                    day = d;
                }
            } else {
                month = month_from_name(part);
            }
        }

        if year > 0 && month > 0 && day > 0 {
            return Ok(make_day_from_parts(year, month, day, hour, minute, second, 0));
        }
    }

    Err(())
}

fn month_from_name(name: &str) -> u32 {
    match name.to_lowercase().as_str() {
        "jan" | "january" => 1,
        "feb" | "february" => 2,
        "mar" | "march" => 3,
        "apr" | "april" => 4,
        "may" => 5,
        "jun" | "june" => 6,
        "jul" | "july" => 7,
        "aug" | "august" => 8,
        "sep" | "september" => 9,
        "oct" | "october" => 10,
        "nov" | "november" => 11,
        "dec" | "december" => 12,
        _ => 0,
    }
}

fn day_from_year(y: i64) -> i64 {
    365 * (y - 1970) + (y - 1969) / 4 - (y - 1901) / 100 + (y - 1601) / 400
}

fn time_within_day(ms: i64) -> i64 {
    ms % (24 * 60 * 60 * 1000)
}

fn day_from_time(ms: i64) -> i64 {
    ms / (24 * 60 * 60 * 1000)
}

fn days_in_year(y: i32) -> i32 {
    if is_leap_year(y) { 366 } else { 365 }
}

fn day_within_year(y: i32, day: i64) -> i64 {
    day - day_from_year(y as i64)
}

fn month_from_time(ms: i64) -> u32 {
    let day = day_from_time(ms);
    let year = year_from_time(ms);
    let dwy = day_within_year(year, day);
    let leap = is_leap_year(year);
    let month_days: [i64; 12] = if leap {
        [31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335, 366]
    } else {
        [31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365]
    };
    for (i, &d) in month_days.iter().enumerate() {
        if dwy < d {
            return i as u32;
        }
    }
    11
}

fn year_from_time(ms: i64) -> i32 {
    let day = day_from_time(ms);
    let mut y = (day / 365).abs() + 1970;
    if day < 0 {
        y = 1970 - ((-day) / 365 + 1);
    }
    loop {
        let dy = day_from_year(y as i64);
        let dy_next = day_from_year((y + 1) as i64);
        if dy <= day && day < dy_next {
            return y as i32;
        }
        if day < dy {
            y -= 1;
        } else {
            y += 1;
        }
    }
}

fn make_day(year: f64, month: f64, date: f64) -> f64 {
    if year.is_nan() || month.is_nan() || date.is_nan() {
        return f64::NAN;
    }
    let y = year as i64;
    let m = month as i64;
    let d = date as i64;
    let ym = y + (m as f64 / 12.0).floor() as i64;
    let mn = m % 12;
    let mn = if mn < 0 { mn + 12 } else { mn };
    let day = day_from_year(ym) + month_to_day_in_year(mn as i32, is_leap_year(ym as i32)) as i64 + d - 1;
    day as f64
}

fn month_to_day_in_year(month: i32, leap: bool) -> i32 {
    let days: [i32; 12] = if leap {
        [0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335]
    } else {
        [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334]
    };
    if month < 12 && month >= 0 { days[month as usize] } else { 0 }
}

fn make_date(day: f64, time: f64) -> f64 {
    if day.is_nan() || time.is_nan() {
        return f64::NAN;
    }
    day * 24.0 * 60.0 * 60.0 * 1000.0 + time
}

fn make_time(hour: f64, min: f64, sec: f64, ms: f64) -> f64 {
    if hour.is_nan() || min.is_nan() || sec.is_nan() || ms.is_nan() {
        return f64::NAN;
    }
    if !hour.is_finite() || !min.is_finite() || !sec.is_finite() || !ms.is_finite() {
        return f64::NAN;
    }
    let h = hour as i64;
    let m = min as i64;
    let s = sec as i64;
    let milli = ms as i64;
    (h * 3600000 + m * 60000 + s * 1000 + milli) as f64
}

fn make_day_count(year: i32, month: u32, day: u32) -> i64 {
    let mut days: i64 = 0;
    if year >= 1970 {
        for y in 1970..year {
            days += days_in_year(y) as i64;
        }
    } else {
        for y in year..1970 {
            days -= days_in_year(y) as i64;
        }
    }
    let days_in_months = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    days += days_in_months[(month.saturating_sub(1)).min(11) as usize] as i64;
    if month > 2 && is_leap_year(year) {
        days += 1;
    }
    days += (day.saturating_sub(1)) as i64;
    days
}

fn to_integer_i64(val: f64) -> i64 {
    if val.is_nan() || val.is_infinite() {
        return 0;
    }
    val.trunc() as i64
}

fn to_integer_f64(val: f64) -> f64 {
    if val.is_nan() {
        return 0.0;
    }
    if val.is_infinite() {
        return val;
    }
    val.trunc()
}

fn make_time_ms(hour: f64, minute: f64, second: f64, ms: f64) -> f64 {
    let h = to_integer_f64(hour);
    let m = to_integer_f64(minute);
    let s = to_integer_f64(second);
    let milli = to_integer_f64(ms);
    h * 3600000.0 + m * 60000.0 + s * 1000.0 + milli
}

fn date_to_timestamp_fast(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32, ms: u32) -> f64 {
    let days = make_day_count(year, month, day);
    (days * 86400000 + hour as i64 * 3600000 + minute as i64 * 60000 + second as i64 * 1000 + ms as i64) as f64
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn coerce_utc_arg(ctx: &mut JSContext, val: Option<&JSValue>, default: f64) -> f64 {
    match val {
        None => default,
        Some(v) if v.is_undefined() => default,
        Some(v) => match to_number_arg(ctx, v) {
            Ok(n) => n,
            Err(_) => f64::NAN,
        },
    }
}

pub fn date_utc(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let args = if !args.is_empty() && args[0].is_function() { &args[1..] } else { args };
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    }

    let year_val = match to_number_arg(ctx, &args[0]) {
        Ok(n) => n,
        Err(_) => return JSValue::undefined(),
    };
    if year_val.is_nan() || year_val.is_infinite() {
        return JSValue::new_float(f64::NAN);
    }
    let year_int = to_integer_i64(year_val);
    let mut year = year_int as i32;
    if year_int >= 0 && year_int <= 99 {
        year += 1900;
    }

    let month_val = coerce_utc_arg(ctx, args.get(1), 0.0);
    if month_val.is_nan() || month_val.is_infinite() {
        return JSValue::new_float(f64::NAN);
    }
    let month_raw = month_val as i32;

    let day_val = coerce_utc_arg(ctx, args.get(2), 1.0);
    let hour_val = coerce_utc_arg(ctx, args.get(3), 0.0);
    let minute_val = coerce_utc_arg(ctx, args.get(4), 0.0);
    let second_val = coerce_utc_arg(ctx, args.get(5), 0.0);
    let ms_val = coerce_utc_arg(ctx, args.get(6), 0.0);
    if day_val.is_nan() || hour_val.is_nan() || minute_val.is_nan() || second_val.is_nan() || ms_val.is_nan() {
        return JSValue::new_float(f64::NAN);
    }
    if day_val.is_infinite() || hour_val.is_infinite() || minute_val.is_infinite() || second_val.is_infinite() || ms_val.is_infinite() {
        return JSValue::new_float(f64::NAN);
    }

    let mut y = year;
    let mut m = month_raw;
    if m >= 12 || m < 0 {
        let extra_years = m / 12;
        m = m % 12;
        if m < 0 {
            m += 12;
            y -= 1;
        }
        y += extra_years;
    }

    let day_int = to_integer_i64(day_val);
    let day = make_day_count(y, (m as u32) + 1, 1) + day_int - 1;
    let time = make_time_ms(hour_val, minute_val, second_val, ms_val);
    let ts = (day as f64) * 86400000.0 + time;
    JSValue::new_float(time_clip(ts))
}

pub fn date_get_time(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    }

    if let Some(_) = require_date_this(ctx, &args[0]) {
        return JSValue::undefined();
    }

    if args[0].is_object() {
        let obj = args[0].as_object();
        let key = ctx.intern("__dateValue__");
        if let Some(ts) = obj.get(key) {
            return ts;
        }
        let vkey = ctx.common_atoms.__value__;
        if let Some(ts) = obj.get(vkey) {
            return ts;
        }
    }

    JSValue::new_float(f64::NAN)
}

pub fn date_get_full_year(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let timestamp = get_date_timestamp_with_ctx(this, ctx);
    if timestamp.is_nan() {
        return JSValue::new_float(f64::NAN);
    }

    let (year, _, _, _, _, _, _, _) = timestamp_to_date(timestamp);
    JSValue::new_int(year as i64)
}

pub fn date_get_month(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let timestamp = get_date_timestamp_with_ctx(this, ctx);
    if timestamp.is_nan() {
        return JSValue::new_float(f64::NAN);
    }

    let (_, month, _, _, _, _, _, _) = timestamp_to_date(timestamp);
    JSValue::new_int(month as i64 - 1)
}

pub fn date_get_date(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let timestamp = get_date_timestamp_with_ctx(this, ctx);
    if timestamp.is_nan() {
        return JSValue::new_float(f64::NAN);
    }

    let (_, _, day, _, _, _, _, _) = timestamp_to_date(timestamp);
    JSValue::new_int(day as i64)
}

pub fn date_get_day(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let timestamp = get_date_timestamp_with_ctx(this, ctx);
    if timestamp.is_nan() {
        return JSValue::new_float(f64::NAN);
    }

    let (_, _, _, _, _, _, weekday, _) = timestamp_to_date(timestamp);
    JSValue::new_int(weekday as i64)
}

pub fn date_get_hours(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let timestamp = get_date_timestamp_with_ctx(this, ctx);
    if timestamp.is_nan() {
        return JSValue::new_float(f64::NAN);
    }

    let (_, _, _, hour, _, _, _, _) = timestamp_to_date(timestamp);
    JSValue::new_int(hour as i64)
}

pub fn date_get_minutes(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let timestamp = get_date_timestamp_with_ctx(this, ctx);
    if timestamp.is_nan() {
        return JSValue::new_float(f64::NAN);
    }

    let (_, _, _, _, minute, _, _, _) = timestamp_to_date(timestamp);
    JSValue::new_int(minute as i64)
}

pub fn date_get_seconds(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let timestamp = get_date_timestamp_with_ctx(this, ctx);
    if timestamp.is_nan() {
        return JSValue::new_float(f64::NAN);
    }

    let (_, _, _, _, _, second, _, _) = timestamp_to_date(timestamp);
    JSValue::new_int(second as i64)
}

pub fn date_get_milliseconds(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let timestamp = get_date_timestamp_with_ctx(this, ctx);
    if timestamp.is_nan() {
        return JSValue::new_float(f64::NAN);
    }

    let ms = (timestamp % 1000.0).abs();
    JSValue::new_int(ms as i64)
}

pub fn date_get_timezone_offset(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, &this) {
        return JSValue::undefined();
    }
    let ts = get_date_timestamp_with_ctx(&this, ctx);
    if ts.is_nan() {
        return JSValue::new_float(f64::NAN);
    }
    JSValue::new_int(0)
}

fn require_date_this(ctx: &mut JSContext, this: &JSValue) -> Option<JSValue> {
    if !this.is_object() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("this is not a Date object")));
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return Some(JSValue::undefined());
    }
    let obj = this.as_object();
    if obj.get(ctx.intern("__dateValue__")).is_none() {
        let has_value = if let Some(v) = obj.get(ctx.common_atoms.__value__) {
            v.is_float() || v.is_int()
        } else {
            false
        };
        if !has_value {
            let mut err = JSObject::new();
            err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
            err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("this is not a Date object")));
            if let Some(proto) = ctx.get_type_error_prototype() {
                err.prototype = Some(proto);
            }
            let ptr = Box::into_raw(Box::new(err)) as usize;
            ctx.runtime_mut().gc_heap_mut().track(ptr);
            ctx.pending_exception = Some(JSValue::new_object(ptr));
            return Some(JSValue::undefined());
        }
    }
    None
}

macro_rules! date_get_utc_field {
    ($name:ident, $field:expr) => {
        pub fn $name(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
            let this = args.get(0).cloned().unwrap_or_else(JSValue::undefined);
            if let Some(e) = require_date_this(ctx, &this) {
                return e;
            }
            let ts = get_date_timestamp_with_ctx(&this, ctx);
            if ts.is_nan() {
                return JSValue::new_float(f64::NAN);
            }
            let (year, month, day, hour, minute, second, weekday, ms_part) = timestamp_to_date(ts);
            JSValue::new_int($field(year, month, day, hour, minute, second, weekday, ms_part))
        }
    };
}

date_get_utc_field!(date_get_utc_full_year, |y,_,_,_,_,_,_,_| y as i64);
date_get_utc_field!(date_get_utc_month, |_,m,_,_,_,_,_,_| m as i64 - 1);
date_get_utc_field!(date_get_utc_date, |_,_,d,_,_,_,_,_| d as i64);
date_get_utc_field!(date_get_utc_day, |_,_,_,_,_,_,w,_| w as i64);
date_get_utc_field!(date_get_utc_hours, |_,_,_,h,_,_,_,_| h as i64);
date_get_utc_field!(date_get_utc_minutes, |_,_,_,_,m,_,_,_| m as i64);
date_get_utc_field!(date_get_utc_seconds, |_,_,_,_,_,s,_,_| s as i64);

pub fn date_get_utc_milliseconds(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(e) = require_date_this(ctx, &this) {
        return e;
    }
    let ts = get_date_timestamp_with_ctx(&this, ctx);
    if ts.is_nan() {
        return JSValue::new_float(f64::NAN);
    }
    let ms = (ts % 1000.0).abs();
    JSValue::new_int(ms as i64)
}

fn set_date_timestamp(ctx: &mut JSContext, this: &JSValue, ts: f64) -> JSValue {
    if this.is_object() {
        let obj = this.as_object_mut();
        obj.set(ctx.common_atoms.__value__, JSValue::new_float(ts));
    }
    JSValue::new_float(ts)
}

fn to_number_arg(ctx: &mut JSContext, val: &JSValue) -> Result<f64, ()> {
    if ctx.pending_exception.is_some() {
        return Err(());
    }
    if val.is_int() {
        return Ok(val.get_int() as f64);
    }
    if val.is_float() {
        return Ok(val.get_float());
    }
    if val.is_null() {
        return Ok(0.0);
    }
    if val.is_bool() {
        return Ok(if val.is_truthy() { 1.0 } else { 0.0 });
    }
    if val.is_undefined() {
        return Ok(f64::NAN);
    }
    if val.is_string() {
        let s = ctx.get_atom_str(val.get_atom()).to_string();
        let s = s.trim();
        if s.is_empty() {
            return Ok(0.0);
        }
        if let Ok(n) = s.parse::<i64>() {
            return Ok(n as f64);
        }
        if let Ok(f) = s.parse::<f64>() {
            return Ok(f);
        }
        return Ok(f64::NAN);
    }
    match crate::builtins::global::js_to_number_value(ctx, val) {
        Ok(n) => {
            if ctx.pending_exception.is_some() {
                return Err(());
            }
            Ok(n)
        }
        Err(_) => Err(()),
    }
}

fn coerce_arg(ctx: &mut JSContext, val: Option<&JSValue>) -> Result<Option<f64>, ()> {
    match val {
        None => Ok(None),
        Some(v) => {
            let n = to_number_arg(ctx, v)?;
            if n.is_nan() {
                Ok(Some(f64::NAN))
            } else {
                Ok(Some(n))
            }
        }
    }
}

pub fn date_set_time(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let t = match coerce_arg(ctx, args.get(1)) {
        Ok(Some(n)) => n,
        Ok(None) => f64::NAN,
        Err(()) => return JSValue::undefined(),
    };
    set_date_timestamp(ctx, this, time_clip(t))
}

pub fn date_set_milliseconds(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let ts = get_date_timestamp_with_ctx(this, ctx);
    let ms = match coerce_arg(ctx, args.get(1)) {
        Ok(Some(n)) => n,
        Ok(None) => f64::NAN,
        Err(()) => return JSValue::undefined(),
    };
    if ts.is_nan() { return JSValue::new_float(f64::NAN); }
    let new_ts = (ts / 1000.0).floor() * 1000.0 + ms;
    set_date_timestamp(ctx, this, time_clip(new_ts))
}

pub fn date_set_seconds(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let ts = get_date_timestamp_with_ctx(this, ctx);
    let sec = match coerce_arg(ctx, args.get(1)) {
        Ok(Some(n)) => n,
        Ok(None) => f64::NAN,
        Err(()) => return JSValue::undefined(),
    };
    let ms = match coerce_arg(ctx, args.get(2)) {
        Ok(Some(n)) => n,
        Ok(None) => if ts.is_nan() { 0.0 } else { ts % 1000.0 },
        Err(()) => return JSValue::undefined(),
    };
    if ts.is_nan() { return JSValue::new_float(f64::NAN); }
    let new_ts = make_date(day_from_time(ts as i64) as f64, make_time((ts as i64 / 3600000 % 24) as f64, (ts as i64 / 60000 % 60) as f64, sec, ms));
    set_date_timestamp(ctx, this, time_clip(new_ts))
}

pub fn date_set_minutes(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let ts = get_date_timestamp_with_ctx(this, ctx);
    let min = match coerce_arg(ctx, args.get(1)) {
        Ok(Some(n)) => n,
        Ok(None) => f64::NAN,
        Err(()) => return JSValue::undefined(),
    };
    let sec = match coerce_arg(ctx, args.get(2)) {
        Ok(Some(n)) => n,
        Ok(None) => if ts.is_nan() { 0.0 } else { ((ts as i64 / 1000) % 60) as f64 },
        Err(()) => return JSValue::undefined(),
    };
    let ms = match coerce_arg(ctx, args.get(3)) {
        Ok(Some(n)) => n,
        Ok(None) => if ts.is_nan() { 0.0 } else { ts % 1000.0 },
        Err(()) => return JSValue::undefined(),
    };
    if ts.is_nan() { return JSValue::new_float(f64::NAN); }
    let new_ts = make_date(day_from_time(ts as i64) as f64, make_time((ts as i64 / 3600000 % 24) as f64, min, sec, ms));
    set_date_timestamp(ctx, this, time_clip(new_ts))
}

pub fn date_set_hours(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let ts = get_date_timestamp_with_ctx(this, ctx);
    let hr = match coerce_arg(ctx, args.get(1)) {
        Ok(Some(n)) => n,
        Ok(None) => f64::NAN,
        Err(()) => return JSValue::undefined(),
    };
    let min = match coerce_arg(ctx, args.get(2)) {
        Ok(Some(n)) => n,
        Ok(None) => if ts.is_nan() { 0.0 } else { ((ts as i64 / 60000) % 60) as f64 },
        Err(()) => return JSValue::undefined(),
    };
    let sec = match coerce_arg(ctx, args.get(3)) {
        Ok(Some(n)) => n,
        Ok(None) => if ts.is_nan() { 0.0 } else { ((ts as i64 / 1000) % 60) as f64 },
        Err(()) => return JSValue::undefined(),
    };
    let ms = match coerce_arg(ctx, args.get(4)) {
        Ok(Some(n)) => n,
        Ok(None) => if ts.is_nan() { 0.0 } else { ts % 1000.0 },
        Err(()) => return JSValue::undefined(),
    };
    if ts.is_nan() { return JSValue::new_float(f64::NAN); }
    let new_ts = make_date(day_from_time(ts as i64) as f64, make_time(hr, min, sec, ms));
    set_date_timestamp(ctx, this, time_clip(new_ts))
}

pub fn date_set_date(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let ts = get_date_timestamp_with_ctx(this, ctx);
    let d = match coerce_arg(ctx, args.get(1)) {
        Ok(Some(n)) => n,
        Ok(None) => f64::NAN,
        Err(()) => return JSValue::undefined(),
    };
    if ts.is_nan() { return JSValue::new_float(f64::NAN); }
    let tsi = ts as i64;
    let year = year_from_time(tsi);
    let month = month_from_time(tsi);
    let new_ts = make_date(make_day(year as f64, month as f64, d), time_within_day(tsi) as f64);
    set_date_timestamp(ctx, this, time_clip(new_ts))
}

pub fn date_set_month(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let ts = get_date_timestamp_with_ctx(this, ctx);
    let mo = match coerce_arg(ctx, args.get(1)) {
        Ok(Some(n)) => n,
        Ok(None) => f64::NAN,
        Err(()) => return JSValue::undefined(),
    };
    let d_val = coerce_arg(ctx, args.get(2));
    if ts.is_nan() { return JSValue::new_float(f64::NAN); }
    let tsi = ts as i64;
    let year = year_from_time(tsi);
    let dt = match d_val {
        Ok(Some(n)) => n,
        Ok(None) => (day_within_year(year, day_from_time(tsi)) - month_to_day_in_year(month_from_time(tsi) as i32, is_leap_year(year)) as i64 + 1) as f64,
        Err(()) => return JSValue::undefined(),
    };
    let new_ts = make_date(make_day(year as f64, mo, dt), time_within_day(tsi) as f64);
    set_date_timestamp(ctx, this, time_clip(new_ts))
}

pub fn date_set_full_year(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let ts = get_date_timestamp_with_ctx(this, ctx);
    let y = match coerce_arg(ctx, args.get(1)) {
        Ok(Some(n)) => n,
        Ok(None) => f64::NAN,
        Err(()) => return JSValue::undefined(),
    };
    let mo_val = coerce_arg(ctx, args.get(2));
    let d_val = coerce_arg(ctx, args.get(3));
    if ts.is_nan() {
        let dt = match d_val {
            Ok(Some(n)) => n,
            Ok(None) => 1.0,
            Err(()) => return JSValue::undefined(),
        };
        let mo = match mo_val {
            Ok(Some(n)) => n,
            Ok(None) => 0.0,
            Err(()) => return JSValue::undefined(),
        };
        let new_ts = make_date(make_day(y, mo, dt), 0.0);
        return set_date_timestamp(ctx, this, time_clip(new_ts));
    }
    let tsi = ts as i64;
    let year = year_from_time(tsi);
    let month = month_from_time(tsi);
    let dt_from_ts = (day_within_year(year, day_from_time(tsi)) - month_to_day_in_year(month as i32, is_leap_year(year)) as i64 + 1) as f64;
    let mo = match mo_val {
        Ok(Some(n)) => n,
        Ok(None) => month as f64,
        Err(()) => return JSValue::undefined(),
    };
    let dt = match d_val {
        Ok(Some(n)) => n,
        Ok(None) => dt_from_ts,
        Err(()) => return JSValue::undefined(),
    };
    let new_ts = make_date(make_day(y, mo, dt), time_within_day(tsi) as f64);
    set_date_timestamp(ctx, this, time_clip(new_ts))
}

pub fn date_set_utc_milliseconds(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    date_set_milliseconds(ctx, args)
}

pub fn date_set_utc_seconds(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    date_set_seconds(ctx, args)
}

pub fn date_set_utc_minutes(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    date_set_minutes(ctx, args)
}

pub fn date_set_utc_hours(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    date_set_hours(ctx, args)
}

pub fn date_set_utc_date(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    date_set_date(ctx, args)
}

pub fn date_set_utc_month(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    date_set_month(ctx, args)
}

pub fn date_set_utc_full_year(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    date_set_full_year(ctx, args)
}

pub fn date_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let timestamp = get_date_timestamp_with_ctx(this, ctx);
    if timestamp.is_nan() {
        return JSValue::new_string(ctx.intern("Invalid Date"));
    }

    let (year, month, day, hour, minute, second, weekday, _) = timestamp_to_date(timestamp);
    let weekday_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let month_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let year_str = if year < 0 {
        if year > -10000 {
            format!("{:05}", year)
        } else {
            format!("{}", year)
        }
    } else if year < 10000 {
        format!("{:04}", year)
    } else {
        format!("{:+}", year)
    };

    let result = format!(
        "{} {} {:02} {} {:02}:{:02}:{:02} GMT+0000 (UTC)",
        weekday_names[weekday as usize],
        month_names[(month - 1) as usize],
        day,
        year_str,
        hour,
        minute,
        second
    );

    JSValue::new_string(ctx.intern(&result))
}

pub fn date_to_iso_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let timestamp = get_date_timestamp_with_ctx(this, ctx);
    if timestamp.is_nan() {
        let mut err = JSObject::new();
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("RangeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("Invalid time value")));
        if let Some(proto) = ctx.get_range_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }

    let (year, month, day, hour, minute, second, _, ms) = timestamp_to_date(timestamp);

    let year_str = if year < 0 {
        if year > -10000 {
            format!("-{:06}", -year)
        } else {
            format!("-{:06}", -year)
        }
    } else if year < 10000 {
        format!("{:04}", year)
    } else {
        format!("+{:06}", year)
    };

    let result = format!(
        "{}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year_str, month, day, hour, minute, second, ms
    );

    JSValue::new_string(ctx.intern(&result))
}

pub fn date_to_utc_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let timestamp = get_date_timestamp_with_ctx(this, ctx);
    if timestamp.is_nan() {
        return JSValue::new_string(ctx.intern("Invalid Date"));
    }

    let (year, month, day, hour, minute, second, weekday, _) = timestamp_to_date(timestamp);
    let weekday_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let month_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let year_str = if year < 0 {
        if year > -10000 {
            format!("{:05}", year)
        } else {
            format!("{}", year)
        }
    } else if year < 10000 {
        format!("{:04}", year)
    } else {
        format!("{:+}", year)
    };

    let result = format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        weekday_names[weekday as usize],
        day,
        month_names[(month - 1) as usize],
        year_str,
        hour,
        minute,
        second
    );

    JSValue::new_string(ctx.intern(&result))
}

pub fn date_to_date_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let timestamp = get_date_timestamp_with_ctx(this, ctx);
    if timestamp.is_nan() {
        return JSValue::new_string(ctx.intern("Invalid Date"));
    }

    let (year, month, day, _, _, _, weekday, _) = timestamp_to_date(timestamp);
    let weekday_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let month_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let year_str = if year < 0 {
        if year > -10000 {
            format!("{:05}", year)
        } else {
            format!("{}", year)
        }
    } else if year < 10000 {
        format!("{:04}", year)
    } else {
        format!("{:+}", year)
    };

    let result = format!(
        "{} {} {:02} {}",
        weekday_names[weekday as usize],
        month_names[(month - 1) as usize],
        day,
        year_str
    );

    JSValue::new_string(ctx.intern(&result))
}

pub fn date_to_time_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let timestamp = get_date_timestamp_with_ctx(this, ctx);
    if timestamp.is_nan() {
        return JSValue::new_string(ctx.intern("Invalid Date"));
    }

    let (_, _, _, hour, minute, second, _, _) = timestamp_to_date(timestamp);

    let result = format!("{:02}:{:02}:{:02} GMT+0000 (UTC)", hour, minute, second);

    JSValue::new_string(ctx.intern(&result))
}

pub fn date_value_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, &this) {
        return JSValue::undefined();
    }
    if !this.is_object() {
        return JSValue::new_float(f64::NAN);
    }
    let obj = this.as_object();
    let key = ctx.intern("__dateValue__");
    if let Some(ts) = obj.get(key) {
        return ts;
    }
    let vkey = ctx.common_atoms.__value__;
    if let Some(ts) = obj.get(vkey) {
        return ts;
    }
    JSValue::new_float(f64::NAN)
}

pub fn date_to_json(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object() {
        return JSValue::undefined();
    }
    let this_val = &args[0];
    let obj = this_val.as_object();
    let to_iso = obj.get(ctx.intern("toISOString"));
    if let Some(fn_val) = to_iso {
        if fn_val.is_function() {
            if let Some(vm_ptr) = ctx.get_register_vm_ptr() {
                let vm = unsafe { &mut *(vm_ptr as *mut crate::runtime::vm::VM) };
                match vm.call_function_with_this(ctx, fn_val, *this_val, &[]) {
                    Ok(val) => return val,
                    Err(_) => {
                        if let Some(exc) = vm.last_caught_exception.take() {
                            ctx.pending_exception = Some(exc);
                        }
                        return JSValue::undefined();
                    }
                }
            }
        }
    }
    JSValue::undefined()
}

pub fn init_date_to_primitive(ctx: &mut JSContext) {
    let global = ctx.global();
    if !global.is_object() {
        return;
    }
    let global_obj = global.as_object();

    let stp_key = ctx.intern("Symbol.toPrimitive");
    let sym_val = match global_obj.get(stp_key) {
        Some(s) if s.is_symbol() => s,
        _ => return,
    };
    let sym_atom = crate::runtime::atom::Atom(0x40000000 | sym_val.get_symbol_id());

    let date_key = ctx.intern("Date");
    let date_ctor = match global_obj.get(date_key) {
        Some(v) if v.is_function() => v,
        _ => return,
    };
    let proto_key = ctx.intern("prototype");
    let proto_val = date_ctor.as_object().get(proto_key);
    if let Some(pv) = proto_val {
        if pv.is_object() {
            let fn_val = create_builtin_function(ctx, "date_toPrimitive", 1, Some("[Symbol.toPrimitive]"));
            pv.as_object_mut().define_property(sym_atom, crate::object::object::PropertyDescriptor {
                value: Some(fn_val),
                writable: false,
                enumerable: false,
                configurable: true,
                get: None,
                set: None,
            });
        }
    }
}

pub fn date_to_primitive(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = &args.get(0).cloned().unwrap_or_else(JSValue::undefined);
    if let Some(_) = require_date_this(ctx, this) {
        return JSValue::undefined();
    }
    let hint_val = args.get(1).cloned().unwrap_or_else(|| JSValue::new_string(ctx.intern("default")));
    let hint = if hint_val.is_string() {
        ctx.get_atom_str(hint_val.get_atom())
    } else {
        "default"
    };
    match hint {
        "string" | "default" => date_to_string(ctx, args),
        "number" => date_get_time(ctx, args),
        _ => date_to_string(ctx, args),
    }
}

fn get_date_timestamp_with_ctx(this: &JSValue, ctx: &mut JSContext) -> f64 {
    if this.is_object() {
        let obj = this.as_object();

        if let Some(ts) = obj.get(ctx.intern("__dateValue__")) {
            if ts.is_int() {
                return ts.get_int() as f64;
            } else if ts.is_float() {
                return ts.get_float();
            }
        }

        if let Some(ts) = obj.get(ctx.common_atoms.__value__) {
            if ts.is_int() {
                return ts.get_int() as f64;
            } else if ts.is_float() {
                return ts.get_float();
            }
        }
    }
    f64::NAN
}

fn timestamp_to_date(ms: f64) -> (i32, u32, u32, u32, u32, u32, u32, u32) {
    if ms.is_nan() || ms.is_infinite() {
        return (0, 0, 0, 0, 0, 0, 0, 0);
    }

    let total_ms = ms as i64;
    let total_seconds = total_ms / 1000;
    let total_minutes = total_seconds / 60;
    let total_hours = total_minutes / 60;
    let total_days = total_hours / 24;

    let mut year = 1970;
    let mut remaining_days = total_days;

    if remaining_days >= 0 {
        while remaining_days >= days_in_year(year) as i64 {
            remaining_days -= days_in_year(year) as i64;
            year += 1;
        }
    } else {
        while remaining_days < 0 {
            year -= 1;
            remaining_days += days_in_year(year) as i64;
        }
    }

    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for &days in &days_in_months {
        if remaining_days < days as i64 {
            break;
        }
        remaining_days -= days as i64;
        month += 1;
    }

    let day = (remaining_days + 1) as u32;
    let hour = ((total_hours % 24 + 24) % 24) as u32;
    let minute = ((total_minutes % 60 + 60) % 60) as u32;
    let second = ((total_seconds % 60 + 60) % 60) as u32;
    let ms_part = (total_ms % 1000 + 1000) % 1000;
    let weekday = ((total_days % 7 + 4 + 7) % 7) as u32;

    (year, month, day, hour, minute, second, weekday, ms_part as u32)
}

fn make_day_from_parts(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32, ms: u32) -> f64 {
    date_to_timestamp_fast(year, month, day, hour, minute, second, ms)
}

fn time_clip(t: f64) -> f64 {
    if t.is_nan() || t.abs() > 8.64e15 {
        return f64::NAN;
    }
    t + 0.0
}

pub fn date_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let timestamp = if args.is_empty() {
        current_timestamp_ms()
    } else if args.len() == 1 {
        if args[0].is_string() {
            let s = ctx.get_atom_str(args[0].get_atom()).to_string();
            let parsed = parse_date_string(&s);
            if !parsed.is_nan() {
                time_clip(parsed)
            } else {
                match to_number_arg(ctx, &args[0]) {
                    Ok(n) => n,
                    Err(()) => f64::NAN,
                }
            }
        } else {
            match to_number_arg(ctx, &args[0]) {
                Ok(n) => n,
                Err(()) => return JSValue::undefined(),
            }
        }
    } else {
        let year_val = match to_number_arg(ctx, &args[0]) {
            Ok(n) => n,
            Err(()) => return JSValue::undefined(),
        };
        let mut year = to_integer_f64(year_val);
        if year >= 0.0 && year <= 99.0 {
            year += 1900.0;
        }
        let month_raw = match coerce_utc_arg(ctx, args.get(1), 0.0) {
            n if n.is_nan() => return JSValue::new_float(f64::NAN),
            n => n,
        };
        let day = coerce_utc_arg(ctx, args.get(2), 1.0);
        if day.is_nan() { return JSValue::new_float(f64::NAN); }
        let hour = coerce_utc_arg(ctx, args.get(3), 0.0);
        if hour.is_nan() { return JSValue::new_float(f64::NAN); }
        let minute = coerce_utc_arg(ctx, args.get(4), 0.0);
        if minute.is_nan() { return JSValue::new_float(f64::NAN); }
        let second = coerce_utc_arg(ctx, args.get(5), 0.0);
        if second.is_nan() { return JSValue::new_float(f64::NAN); }
        let ms = coerce_utc_arg(ctx, args.get(6), 0.0);
        if ms.is_nan() { return JSValue::new_float(f64::NAN); }

        let day_val = make_day(year, month_raw, day);
        let time_val = make_time_ms(hour, minute, second, ms);
        make_date(day_val, time_val)
    };

    if timestamp.is_nan() {
        return JSValue::new_float(f64::NAN);
    }

    let clipped = time_clip(timestamp);
    if clipped == 0.0 && clipped.is_sign_positive() {
        return JSValue::new_float(0.0);
    }
    JSValue::new_float(clipped)
}
