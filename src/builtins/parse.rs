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
        if input_val.get_bool() { "true".to_string() } else { "false".to_string() }
    } else if input_val.is_null() {
        "null".to_string()
    } else if input_val.is_undefined() {
        "undefined".to_string()
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
        } else {
            return JSValue::new_float(f64::NAN);
        };
        if n.is_nan() {
            return JSValue::new_float(f64::NAN);
        }
        let r = n.trunc() as i32;
        if r == 1 { return JSValue::new_float(f64::NAN); }
        r
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
    if final_val >= -(1i64 << 47) as f64 && final_val < (1i64 << 47) as f64 && final_val == final_val.trunc() {
        JSValue::new_int(final_val as i64)
    } else {
        JSValue::new_float(final_val)
    }
}

pub fn global_parsefloat(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    }
    let s = if args[0].is_string() {
        ctx.get_atom_str(args[0].get_atom())
    } else if args[0].is_float() || args[0].is_int() {
        return args[0];
    } else {
        return JSValue::new_float(f64::NAN);
    };
    match s.trim().parse::<f64>() {
        Ok(v) => JSValue::new_float(v),
        Err(_) => JSValue::new_float(f64::NAN),
    }
}
