use crate::runtime::context::JSContext;
use crate::value::JSValue;

fn coerce_uri_input(ctx: &mut JSContext, args: &[JSValue]) -> Result<String, JSValue> {
    if args.is_empty() {
        return Err(JSValue::new_string(ctx.intern("undefined")));
    }

    let input = if args[0].is_string() {
        ctx.get_atom_str(args[0].get_atom()).to_string()
    } else if args[0].is_undefined() {
        return Err(JSValue::new_string(ctx.intern("undefined")));
    } else if args[0].is_null() {
        return Err(JSValue::new_string(ctx.intern("null")));
    } else {
        crate::builtins::global::jsvalue_to_string(&args[0], ctx)
    };

    Ok(input)
}

fn decode_percent_escaped(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hex1 = chars.next();
            let hex2 = chars.next();

            if let (Some(h1), Some(h2)) = (hex1, hex2) {
                let hex_str = format!("{}{}", h1, h2);
                if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push(h1);
                    result.push(h2);
                }
            } else {
                result.push('%');
                if let Some(h1) = hex1 {
                    result.push(h1);
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

pub fn global_encodeuri(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let input = match coerce_uri_input(ctx, args) {
        Ok(v) => v,
        Err(v) => return v,
    };

    const UNENCODED: &str =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789;,/?:@&=+$-_.!~*'()#";

    let mut result = String::new();
    for ch in input.chars() {
        if UNENCODED.contains(ch) {
            result.push(ch);
        } else {
            let bytes = ch.encode_utf8(&mut [0; 4]).as_bytes().to_vec();
            for b in bytes {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }

    JSValue::new_string(ctx.intern(&result))
}

pub fn global_decodeuri(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let input = match coerce_uri_input(ctx, args) {
        Ok(v) => v,
        Err(v) => return v,
    };

    let result = decode_percent_escaped(&input);
    JSValue::new_string(ctx.intern(&result))
}

pub fn global_encodeuricomponent(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let input = match coerce_uri_input(ctx, args) {
        Ok(v) => v,
        Err(v) => return v,
    };

    const UNENCODED: &str =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.!~*'()";

    let mut result = String::new();
    for ch in input.chars() {
        if UNENCODED.contains(ch) {
            result.push(ch);
        } else {
            let bytes = ch.encode_utf8(&mut [0; 4]).as_bytes().to_vec();
            for b in bytes {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }

    JSValue::new_string(ctx.intern(&result))
}

pub fn global_decodeuricomponent(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let input = match coerce_uri_input(ctx, args) {
        Ok(v) => v,
        Err(v) => return v,
    };

    let result = decode_percent_escaped(&input);
    JSValue::new_string(ctx.intern(&result))
}
