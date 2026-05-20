use crate::runtime::context::JSContext;
use crate::value::JSValue;

pub fn number_to_exponential(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() {
        return JSValue::new_string(ctx.intern("0"));
    } else {
        &args[0]
    };

    let fraction_digits = if args.len() > 1 && !args[1].is_undefined() {
        let d = args[1].to_number() as i32;
        if d < 0 || d > 100 {
            let err = JSValue::new_string(
                ctx.intern("RangeError: toExponential() argument must be between 0 and 100"),
            );
            ctx.pending_exception = Some(err);
            return JSValue::undefined();
        }
        d as usize
    } else {
        usize::MAX
    };

    let val = if this.is_int() {
        this.get_int() as f64
    } else if this.is_float() {
        this.get_float()
    } else {
        return JSValue::new_string(ctx.intern("NaN"));
    };

    if val.is_nan() {
        return JSValue::new_string(ctx.intern("NaN"));
    }
    if val.is_infinite() {
        return JSValue::new_string(ctx.intern(if val.is_sign_positive() {
            "Infinity"
        } else {
            "-Infinity"
        }));
    }

    let result = if fraction_digits == usize::MAX {
        let s = format!("{:e}", val);

        fix_exponent_sign(&s)
    } else {
        let s = format!("{:.1$e}", val, fraction_digits);
        fix_exponent_sign(&s)
    };

    JSValue::new_string(ctx.intern(&result))
}

fn fix_exponent_sign(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 1);
    let mut chars = s.chars().peekable();
    let mut seen_e = false;
    while let Some(c) = chars.next() {
        if (c == 'e' || c == 'E') && !seen_e {
            seen_e = true;
            result.push(c);
            if let Some(&next) = chars.peek() {
                if next != '-' && next != '+' {
                    result.push('+');
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn number_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this = if args.is_empty() {
        return JSValue::new_string(ctx.intern("0"));
    } else {
        &args[0]
    };

    let radix = if args.len() > 1 {
        let r = args[1].to_number() as i32;
        if (2..=36).contains(&r) { r } else { 10 }
    } else {
        10
    };

    let result = if this.is_int() {
        let n = this.get_int();
        if radix == 10 {
            n.to_string()
        } else if n < 0 {
            format!("-{}", format_radix((-n) as u64, radix as u32))
        } else {
            format_radix(n as u64, radix as u32)
        }
    } else if this.is_float() {
        let f = this.get_float();
        if f.is_nan() {
            "NaN".to_string()
        } else if f.is_infinite() {
            if f.is_sign_positive() {
                "Infinity".to_string()
            } else {
                "-Infinity".to_string()
            }
        } else if radix != 10 {
            let n = f as i64;
            if n < 0 {
                format!("-{}", format_radix((-n) as u64, radix as u32))
            } else {
                format_radix(n as u64, radix as u32)
            }
        } else if f == f.floor() && f.is_finite() && f.abs() < 1e15 {
            format!("{}", f as i64)
        } else {
            format!("{}", f)
        }
    } else {
        "NaN".to_string()
    };
    JSValue::new_string(ctx.intern(&result))
}

pub(crate) fn format_radix(mut n: u64, radix: u32) -> String {
    if n == 0 {
        return "0".to_string();
    }
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut result = Vec::new();
    while n > 0 {
        result.push(DIGITS[(n % radix as u64) as usize]);
        n /= radix as u64;
    }
    result.reverse();
    String::from_utf8(result).unwrap()
}
