use crate::host::HostFunction;
use crate::object::object::{JSObject, ObjectType, TypedArrayKind};
use crate::runtime::context::JSContext;
use crate::value::JSValue;

#[derive(Debug)]
pub struct ArrayBufferData {
    pub data: Vec<u8>,
}

fn create_array_buffer(ctx: &mut JSContext, length: usize) -> JSValue {
    let mut obj = JSObject::new_typed(ObjectType::ArrayBuffer);
    let data = ArrayBufferData {
        data: vec![0; length],
    };

    obj.set(ctx.intern("byteLength"), JSValue::new_int(length as i64));

    let data_ptr = Box::into_raw(Box::new(data));
    obj.set_array_buffer_data(data_ptr as usize);
    let ptr = Box::into_raw(Box::new(obj)) as usize;
    JSValue::new_object(ptr)
}

fn get_array_buffer_data(obj: &JSObject) -> Option<&mut ArrayBufferData> {
    obj.get_array_buffer_data()
        .map(|ptr| unsafe { &mut *(ptr as *mut ArrayBufferData) })
}

fn create_typed_array(
    ctx: &mut JSContext,
    kind: TypedArrayKind,
    buffer: JSValue,
    byte_offset: usize,
    length: Option<usize>,
) -> Result<JSValue, String> {
    if !buffer.is_object() {
        return Err("TypedArray constructor requires ArrayBuffer".to_string());
    }

    let buffer_obj = buffer.as_object();
    let buffer_data = get_array_buffer_data(&buffer_obj).ok_or("Invalid ArrayBuffer")?;

    let bytes_per_element = kind.bytes_per_element();

    if byte_offset % bytes_per_element != 0 {
        return Err(format!(
            "byteOffset must be a multiple of {}",
            bytes_per_element
        ));
    }

    let byte_length = buffer_data.data.len();
    let remaining_bytes = byte_length.saturating_sub(byte_offset);

    let element_length = match length {
        Some(len) => len,
        None => remaining_bytes / bytes_per_element,
    };

    let required_bytes = byte_offset + element_length * bytes_per_element;
    if required_bytes > byte_length {
        return Err("TypedArray extends beyond ArrayBuffer bounds".to_string());
    }

    let mut obj = JSObject::new_typed(ObjectType::TypedArray);
    obj.set_typed_array_kind(kind);

    obj.set(ctx.intern("buffer"), buffer);
    obj.set(
        ctx.intern("byteOffset"),
        JSValue::new_int(byte_offset as i64),
    );
    obj.set(
        ctx.intern("byteLength"),
        JSValue::new_int((element_length * bytes_per_element) as i64),
    );
    obj.set(
        ctx.intern("length"),
        JSValue::new_int(element_length as i64),
    );

    let ptr = Box::into_raw(Box::new(obj)) as usize;
    Ok(JSValue::new_object(ptr))
}

fn typed_array_from_args(
    ctx: &mut JSContext,
    kind: TypedArrayKind,
    args: &[JSValue],
) -> Result<JSValue, String> {
    if args.is_empty() {
        let buffer = create_array_buffer(ctx, 0);
        return create_typed_array(ctx, kind, buffer, 0, Some(0));
    }

    let first_arg = &args[0];

    if first_arg.is_object() {
        let obj = first_arg.as_object();

        if obj.obj_type() == ObjectType::ArrayBuffer {
            let byte_offset = if args.len() > 1 {
                args[1].get_int() as usize
            } else {
                0
            };
            let length = if args.len() > 2 {
                Some(args[2].get_int() as usize)
            } else {
                None
            };
            return create_typed_array(ctx, kind, *first_arg, byte_offset, length);
        }

        if obj.obj_type() == ObjectType::TypedArray {
            let src_len = obj
                .get(ctx.intern("length"))
                .map(|v| v.get_int() as usize)
                .unwrap_or(0);

            let bytes_per_element = kind.bytes_per_element();
            let buffer = create_array_buffer(ctx, src_len * bytes_per_element);
            let result = create_typed_array(ctx, kind, buffer, 0, Some(src_len))?;

            return Ok(result);
        }

        let len = obj.get_array_elements().map(|e| e.len()).unwrap_or(0);

        let bytes_per_element = kind.bytes_per_element();
        let buffer = create_array_buffer(ctx, len * bytes_per_element);
        let result = create_typed_array(ctx, kind, buffer, 0, Some(len))?;

        return Ok(result);
    }

    if first_arg.is_int() || first_arg.is_float() {
        let length = first_arg.get_int() as usize;
        let bytes_per_element = kind.bytes_per_element();
        let buffer = create_array_buffer(ctx, length * bytes_per_element);
        return create_typed_array(ctx, kind, buffer, 0, Some(length));
    }

    Err("Invalid TypedArray constructor argument".to_string())
}

fn array_buffer_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let length = if args.is_empty() {
        0
    } else {
        args[0].get_int() as usize
    };
    create_array_buffer(ctx, length)
}

fn data_view_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object() {
        return JSValue::undefined();
    }

    let buffer = &args[0];
    let byte_offset = if args.len() > 1 {
        args[1].get_int() as usize
    } else {
        0
    };

    let mut obj = JSObject::new_typed(ObjectType::DataView);
    if let Some(dv_proto_ptr) = ctx.get_dataview_prototype() {
        obj.prototype = Some(dv_proto_ptr);
    }
    obj.set(ctx.intern("buffer"), *buffer);
    obj.set(
        ctx.intern("byteOffset"),
        JSValue::new_int(byte_offset as i64),
    );

    if let Some(buffer_obj) = if buffer.is_object() {
        Some(buffer.as_object())
    } else {
        None
    } {
        if let Some(data) = get_array_buffer_data(&buffer_obj) {
            let byte_length = if args.len() > 2 {
                args[2].get_int() as usize
            } else {
                data.data.len().saturating_sub(byte_offset)
            };
            obj.set(
                ctx.intern("byteLength"),
                JSValue::new_int(byte_length as i64),
            );
        }
    }

    let ptr = Box::into_raw(Box::new(obj)) as usize;
    JSValue::new_object(ptr)
}

fn int8_array_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    typed_array_from_args(ctx, TypedArrayKind::Int8, args).unwrap_or(JSValue::undefined())
}

fn uint8_array_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    typed_array_from_args(ctx, TypedArrayKind::Uint8, args).unwrap_or(JSValue::undefined())
}

fn uint8_clamped_array_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    typed_array_from_args(ctx, TypedArrayKind::Uint8Clamped, args).unwrap_or(JSValue::undefined())
}

fn int16_array_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    typed_array_from_args(ctx, TypedArrayKind::Int16, args).unwrap_or(JSValue::undefined())
}

fn uint16_array_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    typed_array_from_args(ctx, TypedArrayKind::Uint16, args).unwrap_or(JSValue::undefined())
}

fn int32_array_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    typed_array_from_args(ctx, TypedArrayKind::Int32, args).unwrap_or(JSValue::undefined())
}

fn uint32_array_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    typed_array_from_args(ctx, TypedArrayKind::Uint32, args).unwrap_or(JSValue::undefined())
}

fn float32_array_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    typed_array_from_args(ctx, TypedArrayKind::Float32, args).unwrap_or(JSValue::undefined())
}

fn float64_array_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    typed_array_from_args(ctx, TypedArrayKind::Float64, args).unwrap_or(JSValue::undefined())
}

fn bigint64_array_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    typed_array_from_args(ctx, TypedArrayKind::BigInt64, args).unwrap_or(JSValue::undefined())
}

fn biguint64_array_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    typed_array_from_args(ctx, TypedArrayKind::BigUint64, args).unwrap_or(JSValue::undefined())
}

fn typed_array_get_buffer(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object() {
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    obj.get(ctx.intern("buffer"))
        .unwrap_or(JSValue::undefined())
}

fn typed_array_get_byte_length(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object() {
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    obj.get(ctx.intern("byteLength"))
        .unwrap_or(JSValue::undefined())
}

fn typed_array_get_byte_offset(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object() {
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    obj.get(ctx.intern("byteOffset"))
        .unwrap_or(JSValue::undefined())
}

fn typed_array_get_length(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object() {
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    obj.get(ctx.intern("length"))
        .unwrap_or(JSValue::undefined())
}

fn array_buffer_get_byte_length(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object() {
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    obj.get(ctx.intern("byteLength"))
        .unwrap_or(JSValue::undefined())
}

fn data_view_get_buffer(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !dv_check_this(args, ctx) {
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    obj.get(ctx.intern("buffer"))
        .unwrap_or(JSValue::undefined())
}

fn data_view_get_byte_length(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !dv_check_this(args, ctx) {
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    obj.get(ctx.intern("byteLength"))
        .unwrap_or(JSValue::undefined())
}

fn data_view_get_byte_offset(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !dv_check_this(args, ctx) {
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    obj.get(ctx.intern("byteOffset"))
        .unwrap_or(JSValue::undefined())
}

fn data_view_get_int8(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    match dv_read_bytes_checked(args, ctx, 1) {
        Some(b) => JSValue::new_int(b[0] as i8 as i64),
        None => JSValue::undefined(),
    }
}

fn data_view_get_uint8(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    match dv_read_bytes_checked(args, ctx, 1) {
        Some(b) => JSValue::new_int(b[0] as i64),
        None => JSValue::undefined(),
    }
}

fn f16_to_f32(val: u16) -> f32 {
    let sign = (val >> 15) as u32;
    let exp = (val >> 10) & 0x1F;
    let mantissa = val & 0x3FF;

    if exp == 0 {
        if mantissa == 0 {
            f32::from_bits(sign << 31)
        } else {
            let mut m = mantissa as u32;
            let mut e = 1;
            while (m & 0x400) == 0 {
                m <<= 1;
                e -= 1;
            }
            m &= 0x3FF;
            f32::from_bits((sign << 31) | (((112 + e) as u32) << 23) | (m << 13))
        }
    } else if exp == 31 {
        f32::from_bits((sign << 31) | 0x7F800000 | ((mantissa as u32) << 13))
    } else {
        f32::from_bits((sign << 31) | (((exp as u32) + 112) << 23) | ((mantissa as u32) << 13))
    }
}

fn f32_to_f16(val: f32) -> u16 {
    let bits = val.to_bits();
    let sign = (bits >> 16) as u16;
    let exp = ((bits >> 23) & 0xFF) as i32;
    let mantissa = bits & 0x7FFFFF;

    if exp == 0xFF {
        return sign | 0x7C00 | if mantissa != 0 { 0x200 } else { 0 };
    }
    if exp == 0 {
        return sign | (mantissa >> 13) as u16;
    }
    let new_exp = exp - 127 + 15;
    if new_exp >= 31 {
        return sign | 0x7C00;
    }
    if new_exp <= 0 {
        return sign | (mantissa >> (14 - new_exp)) as u16;
    }
    sign | ((new_exp as u16) << 10) | ((mantissa >> 13) as u16)
}

fn dv_extract_buffer_info(args: &[JSValue], ctx: &mut JSContext) -> Option<(Vec<u8>, usize)> {
    if args.is_empty() || !args[0].is_object() {
        return None;
    }
    let view_obj = args[0].as_object();
    let byte_offset = view_obj
        .get(ctx.intern("byteOffset"))
        .map(|v| v.to_number() as usize)
        .unwrap_or(0);
    let buffer = view_obj.get(ctx.intern("buffer"))?;
    if !buffer.is_object() { return None; }
    let buf_obj = buffer.as_object();
    let data = get_array_buffer_data(&buf_obj)?;
    Some((data.data.clone(), byte_offset))
}

fn throw_type_error(ctx: &mut JSContext, msg: &str) {
    let mut err = JSObject::new();
    if let Some(proto_ptr) = ctx.get_type_error_prototype() {
        err.prototype = Some(proto_ptr);
    }
    err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
    err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern(msg)));
    let ptr = Box::into_raw(Box::new(err)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    ctx.pending_exception = Some(JSValue::new_object(ptr));
}

fn throw_range_error(ctx: &mut JSContext, msg: &str) {
    let mut err = JSObject::new();
    if let Some(proto_ptr) = ctx.get_range_error_prototype() {
        err.prototype = Some(proto_ptr);
    }
    err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("RangeError")));
    err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern(msg)));
    let ptr = Box::into_raw(Box::new(err)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    ctx.pending_exception = Some(JSValue::new_object(ptr));
}

fn dv_check_this(args: &[JSValue], ctx: &mut JSContext) -> bool {
    if args.is_empty() || !args[0].is_object() {
        throw_type_error(ctx, "Method called on incompatible receiver");
        return false;
    }
    let view_obj = args[0].as_object();
    if view_obj.obj_type() != crate::object::object::ObjectType::DataView {
        throw_type_error(ctx, "Method called on incompatible receiver");
        return false;
    }
    true
}

fn dv_to_index(val: f64) -> Option<usize> {
    if val.is_nan() || val < 0.0 || !val.is_finite() {
        return None;
    }
    Some(val as usize)
}

fn dv_read_bytes_checked(args: &[JSValue], ctx: &mut JSContext, n: usize) -> Option<[u8; 8]> {
    if !dv_check_this(args, ctx) { return None; }
    if args.len() < 2 { return None; }
    let raw_offset = args[1].to_number();
    let offset = match dv_to_index(raw_offset) {
        Some(o) => o,
        None => {
            throw_range_error(ctx, &format!("Offset is out of range: {}", raw_offset));
            return None;
        }
    };
    let little_endian = args.get(2).map_or(false, |v| v.is_truthy());
    let (data, byte_offset) = dv_extract_buffer_info(args, ctx)?;
    let idx = match byte_offset.checked_add(offset) {
        Some(i) => i,
        None => {
            throw_range_error(ctx, "Offset is out of range");
            return None;
        }
    };
    if idx + n > data.len() {
        throw_range_error(ctx, "Offset is out of range for DataView");
        return None;
    }
    let mut buf = [0u8; 8];
    buf[..n].copy_from_slice(&data[idx..idx+n]);
    if little_endian {
        buf[..n].reverse();
    }
    Some(buf)
}

fn dv_write_bytes_checked(args: &[JSValue], ctx: &mut JSContext, bytes: &[u8]) -> bool {
    if !dv_check_this(args, ctx) { return false; }
    if args.len() < 3 {
        throw_type_error(ctx, "Not enough arguments");
        return false;
    }
    let raw_offset = args[1].to_number();
    let offset = match dv_to_index(raw_offset) {
        Some(o) => o,
        None => {
            throw_range_error(ctx, &format!("Offset is out of range: {}", raw_offset));
            return false;
        }
    };
    let little_endian = args.get(3).map_or(false, |v| v.is_truthy());
    let view_obj = args[0].as_object();
    let byte_offset = view_obj
        .get(ctx.intern("byteOffset"))
        .map(|v| v.to_number() as usize)
        .unwrap_or(0);
    let buffer = match view_obj.get(ctx.intern("buffer")) {
        Some(b) => b,
        None => {
            throw_type_error(ctx, "Detached buffer");
            return false;
        }
    };
    if !buffer.is_object() {
        throw_type_error(ctx, "Detached buffer");
        return false;
    }
    let buf_obj = buffer.as_object();
    let data = match get_array_buffer_data(&buf_obj) {
        Some(d) => d,
        None => {
            throw_type_error(ctx, "Detached buffer");
            return false;
        }
    };
    let idx = match byte_offset.checked_add(offset) {
        Some(i) => i,
        None => {
            throw_range_error(ctx, "Offset is out of range");
            return false;
        }
    };
    if idx + bytes.len() > data.data.len() {
        throw_range_error(ctx, "Offset is out of range for DataView");
        return false;
    }
    if little_endian {
        let mut rev = bytes.to_vec();
        rev.reverse();
        data.data[idx..idx+bytes.len()].copy_from_slice(&rev);
    } else {
        data.data[idx..idx+bytes.len()].copy_from_slice(bytes);
    }
    true
}

fn data_view_set_int8(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 { return JSValue::undefined(); }
    let val = args[2].get_int() as i8 as u8;
    let bytes = [val];
    dv_write_bytes_checked(args, ctx, &bytes);
    JSValue::undefined()
}

fn data_view_set_uint8(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 { return JSValue::undefined(); }
    let val = args[2].get_int() as u8;
    let bytes = [val];
    dv_write_bytes_checked(args, ctx, &bytes);
    JSValue::undefined()
}

fn data_view_set_int16(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 { return JSValue::undefined(); }
    let val = args[2].to_number() as i16;
    let bytes = val.to_be_bytes();
    dv_write_bytes_checked(args, ctx, &bytes);
    JSValue::undefined()
}

fn data_view_set_uint16(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 { return JSValue::undefined(); }
    let val = args[2].to_number() as u16;
    let bytes = val.to_be_bytes();
    dv_write_bytes_checked(args, ctx, &bytes);
    JSValue::undefined()
}

fn data_view_set_int32(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 { return JSValue::undefined(); }
    let val = args[2].to_number() as i32;
    let bytes = val.to_be_bytes();
    dv_write_bytes_checked(args, ctx, &bytes);
    JSValue::undefined()
}

fn data_view_set_uint32(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 { return JSValue::undefined(); }
    let val = args[2].to_number() as u32;
    let bytes = val.to_be_bytes();
    dv_write_bytes_checked(args, ctx, &bytes);
    JSValue::undefined()
}

fn data_view_set_float32(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 { return JSValue::undefined(); }
    let val = args[2].to_number() as f32;
    let bytes = val.to_be_bytes();
    dv_write_bytes_checked(args, ctx, &bytes);
    JSValue::undefined()
}

fn data_view_set_float64(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 { return JSValue::undefined(); }
    let val = args[2].to_number();
    let bytes = val.to_be_bytes();
    dv_write_bytes_checked(args, ctx, &bytes);
    JSValue::undefined()
}

fn data_view_set_float16(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 { return JSValue::undefined(); }
    let val = args[2].to_number() as f32;
    let h = f32_to_f16(val);
    let bytes = h.to_be_bytes();
    dv_write_bytes_checked(args, ctx, &bytes);
    JSValue::undefined()
}

fn data_view_get_int16(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    match dv_read_bytes_checked(args, ctx, 2) {
        Some(b) => JSValue::new_int(i16::from_be_bytes([b[0], b[1]]) as i64),
        None => JSValue::undefined(),
    }
}

fn data_view_get_uint16(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    match dv_read_bytes_checked(args, ctx, 2) {
        Some(b) => JSValue::new_int(u16::from_be_bytes([b[0], b[1]]) as i64),
        None => JSValue::undefined(),
    }
}

fn data_view_get_int32(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    match dv_read_bytes_checked(args, ctx, 4) {
        Some(b) => JSValue::new_int(i32::from_be_bytes([b[0], b[1], b[2], b[3]]) as i64),
        None => JSValue::undefined(),
    }
}

fn data_view_get_uint32(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    match dv_read_bytes_checked(args, ctx, 4) {
        Some(b) => {
            let val = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
            JSValue::new_int(val as i64)
        }
        None => JSValue::undefined(),
    }
}

fn data_view_get_float32(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    match dv_read_bytes_checked(args, ctx, 4) {
        Some(b) => JSValue::new_float(f32::from_be_bytes([b[0], b[1], b[2], b[3]]) as f64),
        None => JSValue::undefined(),
    }
}

fn data_view_get_float64(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    match dv_read_bytes_checked(args, ctx, 8) {
        Some(b) => JSValue::new_float(f64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ])),
        None => JSValue::undefined(),
    }
}

fn data_view_get_float16(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    match dv_read_bytes_checked(args, ctx, 2) {
        Some(b) => {
            let val = u16::from_be_bytes([b[0], b[1]]);
            JSValue::new_float(f16_to_f32(val) as f64)
        }
        None => JSValue::undefined(),
    }
}

fn data_view_get_bigint64(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    match dv_read_bytes_checked(args, ctx, 8) {
        Some(b) => {
            let val = i64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
            let mut obj = crate::object::object::JSObject::new_bigint();
            obj.set_bigint_value(val as i128);
            JSValue::new_bigint(Box::into_raw(Box::new(obj)) as usize)
        }
        None => JSValue::undefined(),
    }
}

fn data_view_get_biguint64(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    match dv_read_bytes_checked(args, ctx, 8) {
        Some(b) => {
            let val = u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
            let mut obj = crate::object::object::JSObject::new_bigint();
            obj.set_bigint_value(val as i128);
            JSValue::new_bigint(Box::into_raw(Box::new(obj)) as usize)
        }
        None => JSValue::undefined(),
    }
}

fn data_view_set_bigint64(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 { return JSValue::undefined(); }
    let val = if args[2].is_bigint() {
        let obj = args[2].as_object();
        obj.get_bigint_value() as i64
    } else {
        args[2].to_number() as i64
    };
    let bytes = val.to_be_bytes();
    dv_write_bytes_checked(args, ctx, &bytes);
    JSValue::undefined()
}

fn data_view_set_biguint64(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 { return JSValue::undefined(); }
    let val = if args[2].is_bigint() {
        let obj = args[2].as_object();
        obj.get_bigint_value() as u64
    } else {
        args[2].to_number() as u64
    };
    let bytes = val.to_be_bytes();
    dv_write_bytes_checked(args, ctx, &bytes);
    JSValue::undefined()
}

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let arity = ctx.get_builtin_arity(name).unwrap_or(1);
    let mut func = crate::object::function::JSFunction::new_builtin(ctx.intern(name), arity);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

pub fn init_typed_array(ctx: &mut JSContext) {
    let global = ctx.global();
    if !global.is_object() {
        return;
    }
    let global_obj = global.as_object_mut();

    crate::builtins::global::set_non_enumerable(
        global_obj,
        ctx.intern("ArrayBuffer"),
        create_builtin_function(ctx, "ArrayBuffer"),
    );

    let mut dv_proto = JSObject::new();
    dv_proto.set(ctx.intern("buffer"), create_builtin_function(ctx, "dataview_buffer"));
    dv_proto.set(
        ctx.intern("byteLength"),
        create_builtin_function(ctx, "dataview_byteLength"),
    );
    dv_proto.set(
        ctx.intern("byteOffset"),
        create_builtin_function(ctx, "dataview_byteOffset"),
    );
    dv_proto.set(
        ctx.intern("getInt8"),
        create_builtin_function(ctx, "dataview_getInt8"),
    );
    dv_proto.set(
        ctx.intern("getUint8"),
        create_builtin_function(ctx, "dataview_getUint8"),
    );
    dv_proto.set(
        ctx.intern("setInt8"),
        create_builtin_function(ctx, "dataview_setInt8"),
    );
    dv_proto.set(
        ctx.intern("setUint8"),
        create_builtin_function(ctx, "dataview_setUint8"),
    );
    dv_proto.set(
        ctx.intern("getInt16"),
        create_builtin_function(ctx, "dataview_getInt16"),
    );
    dv_proto.set(
        ctx.intern("getUint16"),
        create_builtin_function(ctx, "dataview_getUint16"),
    );
    dv_proto.set(
        ctx.intern("getInt32"),
        create_builtin_function(ctx, "dataview_getInt32"),
    );
    dv_proto.set(
        ctx.intern("getUint32"),
        create_builtin_function(ctx, "dataview_getUint32"),
    );
    dv_proto.set(
        ctx.intern("getFloat32"),
        create_builtin_function(ctx, "dataview_getFloat32"),
    );
    dv_proto.set(
        ctx.intern("getFloat64"),
        create_builtin_function(ctx, "dataview_getFloat64"),
    );
    dv_proto.set(
        ctx.intern("getFloat16"),
        create_builtin_function(ctx, "dataview_getFloat16"),
    );
    dv_proto.set(
        ctx.intern("setInt16"),
        create_builtin_function(ctx, "dataview_setInt16"),
    );
    dv_proto.set(
        ctx.intern("setUint16"),
        create_builtin_function(ctx, "dataview_setUint16"),
    );
    dv_proto.set(
        ctx.intern("setInt32"),
        create_builtin_function(ctx, "dataview_setInt32"),
    );
    dv_proto.set(
        ctx.intern("setUint32"),
        create_builtin_function(ctx, "dataview_setUint32"),
    );
    dv_proto.set(
        ctx.intern("setFloat32"),
        create_builtin_function(ctx, "dataview_setFloat32"),
    );
    dv_proto.set(
        ctx.intern("setFloat64"),
        create_builtin_function(ctx, "dataview_setFloat64"),
    );
    dv_proto.set(
        ctx.intern("setFloat16"),
        create_builtin_function(ctx, "dataview_setFloat16"),
    );
    dv_proto.set(
        ctx.intern("getBigInt64"),
        create_builtin_function(ctx, "dataview_getBigInt64"),
    );
    dv_proto.set(
        ctx.intern("getBigUint64"),
        create_builtin_function(ctx, "dataview_getBigUint64"),
    );
    dv_proto.set(
        ctx.intern("setBigInt64"),
        create_builtin_function(ctx, "dataview_setBigInt64"),
    );
    dv_proto.set(
        ctx.intern("setBigUint64"),
        create_builtin_function(ctx, "dataview_setBigUint64"),
    );

    let to_string_tag_key = crate::builtins::symbol::get_symbol_to_string_tag_prop_key(ctx);
    dv_proto.set(to_string_tag_key, JSValue::new_string(ctx.intern("DataView")));

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        dv_proto.prototype = Some(obj_proto_ptr);
    }

    let dv_proto_ptr = Box::into_raw(Box::new(dv_proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(dv_proto_ptr);
    ctx.set_dataview_prototype(dv_proto_ptr);

    let dv_ctor = create_builtin_function(ctx, "DataView");
    if dv_ctor.is_function() {
        let dv_ctor_obj = dv_ctor.as_object_mut();
        dv_ctor_obj.set(ctx.intern("prototype"), JSValue::new_object(dv_proto_ptr));
    }
    crate::builtins::global::set_non_enumerable(
        global_obj,
        ctx.intern("DataView"),
        dv_ctor,
    );

    crate::builtins::global::set_non_enumerable(
        global_obj,
        ctx.intern("Int8Array"),
        create_builtin_function(ctx, "Int8Array"),
    );
    crate::builtins::global::set_non_enumerable(
        global_obj,
        ctx.intern("Uint8Array"),
        create_builtin_function(ctx, "Uint8Array"),
    );
    crate::builtins::global::set_non_enumerable(
        global_obj,
        ctx.intern("Uint8ClampedArray"),
        create_builtin_function(ctx, "Uint8ClampedArray"),
    );
    crate::builtins::global::set_non_enumerable(
        global_obj,
        ctx.intern("Int16Array"),
        create_builtin_function(ctx, "Int16Array"),
    );
    crate::builtins::global::set_non_enumerable(
        global_obj,
        ctx.intern("Uint16Array"),
        create_builtin_function(ctx, "Uint16Array"),
    );
    crate::builtins::global::set_non_enumerable(
        global_obj,
        ctx.intern("Int32Array"),
        create_builtin_function(ctx, "Int32Array"),
    );
    crate::builtins::global::set_non_enumerable(
        global_obj,
        ctx.intern("Uint32Array"),
        create_builtin_function(ctx, "Uint32Array"),
    );
    crate::builtins::global::set_non_enumerable(
        global_obj,
        ctx.intern("Float32Array"),
        create_builtin_function(ctx, "Float32Array"),
    );
    crate::builtins::global::set_non_enumerable(
        global_obj,
        ctx.intern("Float64Array"),
        create_builtin_function(ctx, "Float64Array"),
    );
    crate::builtins::global::set_non_enumerable(
        global_obj,
        ctx.intern("BigInt64Array"),
        create_builtin_function(ctx, "BigInt64Array"),
    );
    crate::builtins::global::set_non_enumerable(
        global_obj,
        ctx.intern("BigUint64Array"),
        create_builtin_function(ctx, "BigUint64Array"),
    );
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin(
        "ArrayBuffer",
        HostFunction::ctor("ArrayBuffer", 1, array_buffer_constructor),
    );
    ctx.register_builtin(
        "DataView",
        HostFunction::ctor("DataView", 1, data_view_constructor),
    );

    ctx.register_builtin(
        "Int8Array",
        HostFunction::ctor("Int8Array", 1, int8_array_constructor),
    );
    ctx.register_builtin(
        "Uint8Array",
        HostFunction::ctor("Uint8Array", 1, uint8_array_constructor),
    );
    ctx.register_builtin(
        "Uint8ClampedArray",
        HostFunction::ctor("Uint8ClampedArray", 1, uint8_clamped_array_constructor),
    );
    ctx.register_builtin(
        "Int16Array",
        HostFunction::ctor("Int16Array", 1, int16_array_constructor),
    );
    ctx.register_builtin(
        "Uint16Array",
        HostFunction::ctor("Uint16Array", 1, uint16_array_constructor),
    );
    ctx.register_builtin(
        "Int32Array",
        HostFunction::ctor("Int32Array", 1, int32_array_constructor),
    );
    ctx.register_builtin(
        "Uint32Array",
        HostFunction::ctor("Uint32Array", 1, uint32_array_constructor),
    );
    ctx.register_builtin(
        "Float32Array",
        HostFunction::ctor("Float32Array", 1, float32_array_constructor),
    );
    ctx.register_builtin(
        "Float64Array",
        HostFunction::ctor("Float64Array", 1, float64_array_constructor),
    );
    ctx.register_builtin(
        "BigInt64Array",
        HostFunction::ctor("BigInt64Array", 1, bigint64_array_constructor),
    );
    ctx.register_builtin(
        "BigUint64Array",
        HostFunction::ctor("BigUint64Array", 1, biguint64_array_constructor),
    );

    ctx.register_builtin(
        "typedarray_buffer",
        HostFunction::method("buffer", 0, typed_array_get_buffer),
    );
    ctx.register_builtin(
        "typedarray_byteLength",
        HostFunction::method("byteLength", 0, typed_array_get_byte_length),
    );
    ctx.register_builtin(
        "typedarray_byteOffset",
        HostFunction::method("byteOffset", 0, typed_array_get_byte_offset),
    );
    ctx.register_builtin(
        "typedarray_length",
        HostFunction::method("length", 0, typed_array_get_length),
    );

    ctx.register_builtin(
        "arraybuffer_byteLength",
        HostFunction::method("byteLength", 0, array_buffer_get_byte_length),
    );

    ctx.register_builtin(
        "dataview_buffer",
        HostFunction::method("buffer", 0, data_view_get_buffer),
    );
    ctx.register_builtin(
        "dataview_byteLength",
        HostFunction::method("byteLength", 0, data_view_get_byte_length),
    );
    ctx.register_builtin(
        "dataview_byteOffset",
        HostFunction::method("byteOffset", 0, data_view_get_byte_offset),
    );
    ctx.register_builtin(
        "dataview_getInt8",
        HostFunction::method("getInt8", 1, data_view_get_int8),
    );
    ctx.register_builtin(
        "dataview_getUint8",
        HostFunction::method("getUint8", 1, data_view_get_uint8),
    );
    ctx.register_builtin(
        "dataview_setInt8",
        HostFunction::method("setInt8", 2, data_view_set_int8),
    );
    ctx.register_builtin(
        "dataview_setUint8",
        HostFunction::method("setUint8", 2, data_view_set_uint8),
    );
    ctx.register_builtin(
        "dataview_getInt16",
        HostFunction::method("getInt16", 2, data_view_get_int16),
    );
    ctx.register_builtin(
        "dataview_getUint16",
        HostFunction::method("getUint16", 2, data_view_get_uint16),
    );
    ctx.register_builtin(
        "dataview_getInt32",
        HostFunction::method("getInt32", 2, data_view_get_int32),
    );
    ctx.register_builtin(
        "dataview_getUint32",
        HostFunction::method("getUint32", 2, data_view_get_uint32),
    );
    ctx.register_builtin(
        "dataview_getFloat32",
        HostFunction::method("getFloat32", 2, data_view_get_float32),
    );
    ctx.register_builtin(
        "dataview_getFloat64",
        HostFunction::method("getFloat64", 2, data_view_get_float64),
    );
    ctx.register_builtin(
        "dataview_getFloat16",
        HostFunction::method("getFloat16", 2, data_view_get_float16),
    );
    ctx.register_builtin(
        "dataview_setFloat16",
        HostFunction::method("setFloat16", 3, data_view_set_float16),
    );
    ctx.register_builtin(
        "dataview_setInt16",
        HostFunction::method("setInt16", 3, data_view_set_int16),
    );
    ctx.register_builtin(
        "dataview_setUint16",
        HostFunction::method("setUint16", 3, data_view_set_uint16),
    );
    ctx.register_builtin(
        "dataview_setInt32",
        HostFunction::method("setInt32", 3, data_view_set_int32),
    );
    ctx.register_builtin(
        "dataview_setUint32",
        HostFunction::method("setUint32", 3, data_view_set_uint32),
    );
    ctx.register_builtin(
        "dataview_setFloat32",
        HostFunction::method("setFloat32", 3, data_view_set_float32),
    );
    ctx.register_builtin(
        "dataview_setFloat64",
        HostFunction::method("setFloat64", 3, data_view_set_float64),
    );
    ctx.register_builtin(
        "dataview_getBigInt64",
        HostFunction::method("getBigInt64", 2, data_view_get_bigint64),
    );
    ctx.register_builtin(
        "dataview_getBigUint64",
        HostFunction::method("getBigUint64", 2, data_view_get_biguint64),
    );
    ctx.register_builtin(
        "dataview_setBigInt64",
        HostFunction::method("setBigInt64", 3, data_view_set_bigint64),
    );
    ctx.register_builtin(
        "dataview_setBigUint64",
        HostFunction::method("setBigUint64", 3, data_view_set_biguint64),
    );
}
