use crate::runtime::context::JSContext;
use crate::value::JSValue;

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub(crate) fn base64_encode_standard(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        let i0 = (b0 >> 2) as usize;
        let i1 = (((b0 & 0x03) << 4) | (b1 >> 4)) as usize;
        let i2 = (((b1 & 0x0f) << 2) | (b2 >> 6)) as usize;
        let i3 = (b2 & 0x3f) as usize;

        out.push(BASE64_ALPHABET[i0] as char);
        out.push(BASE64_ALPHABET[i1] as char);
        if chunk.len() > 1 {
            out.push(BASE64_ALPHABET[i2] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(BASE64_ALPHABET[i3] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub(crate) fn base64_decode_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

pub(crate) fn base64_decode_standard(input: &[u8]) -> Option<Vec<u8>> {
    if !input.len().is_multiple_of(4) {
        return None;
    }

    let mut out = Vec::with_capacity(input.len() / 4 * 3);

    for chunk in input.chunks(4) {
        let c0 = chunk[0];
        let c1 = chunk[1];
        let c2 = chunk[2];
        let c3 = chunk[3];

        let v0 = base64_decode_value(c0)?;
        let v1 = base64_decode_value(c1)?;

        let p2 = c2 == b'=';
        let p3 = c3 == b'=';

        let v2 = if p2 {
            if !p3 {
                return None;
            }
            0
        } else {
            base64_decode_value(c2)?
        };

        let v3 = if p3 { 0 } else { base64_decode_value(c3)? };

        out.push((v0 << 2) | (v1 >> 4));
        if !p2 {
            out.push(((v1 & 0x0f) << 4) | (v2 >> 2));
        }
        if !p3 {
            out.push(((v2 & 0x03) << 6) | v3);
        }
    }

    Some(out)
}

pub fn global_btoa(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }

    let input = if args[0].is_string() {
        ctx.get_atom_str(args[0].get_atom())
    } else {
        return JSValue::new_string(ctx.intern(""));
    };

    let encoded = base64_encode_standard(input.as_bytes());
    JSValue::new_string(ctx.intern(&encoded))
}

pub fn global_atob(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_string(ctx.intern(""));
    }

    let input = if args[0].is_string() {
        ctx.get_atom_str(args[0].get_atom())
    } else {
        return JSValue::new_string(ctx.intern(""));
    };

    match base64_decode_standard(input.as_bytes()) {
        Some(decoded) => {
            let result = String::from_utf8_lossy(&decoded);
            JSValue::new_string(ctx.intern(&result))
        }
        None => JSValue::new_string(ctx.intern("")),
    }
}
