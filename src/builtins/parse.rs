use crate::runtime::context::JSContext;
use crate::value::JSValue;

pub fn global_parseint(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    }
    let s = if args[0].is_string() {
        ctx.get_atom_str(args[0].get_atom())
    } else if args[0].is_int() {
        return args[0];
    } else if args[0].is_float() {
        return JSValue::new_int(args[0].get_float().trunc() as i64);
    } else {
        return JSValue::new_float(f64::NAN);
    };

    let mut input = s.trim_start();
    let mut sign = 1i64;
    if let Some(rest) = input.strip_prefix('-') {
        sign = -1;
        input = rest;
    } else if let Some(rest) = input.strip_prefix('+') {
        input = rest;
    }

    let mut radix = if args.len() > 1 {
        if args[1].is_int() {
            args[1].get_int() as i32
        } else if args[1].is_float() {
            args[1].get_float() as i32
        } else {
            0
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

    let mut digits = String::new();
    for ch in input.chars() {
        if ch.to_digit(radix as u32).is_some() {
            digits.push(ch);
        } else {
            break;
        }
    }

    if digits.is_empty() {
        return JSValue::new_float(f64::NAN);
    }

    match i64::from_str_radix(&digits, radix as u32) {
        Ok(v) => JSValue::new_int(sign.saturating_mul(v)),
        Err(_) => JSValue::new_float(f64::NAN),
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
