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
    if let Some(ta_proto_ptr) = ctx.get_typed_array_prototype() {
        obj.prototype = Some(ta_proto_ptr);
    }

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

        let len = obj.get(ctx.common_atoms.length)
            .map(|v| v.get_int() as usize)
            .unwrap_or(0);

        let bytes_per_element = kind.bytes_per_element();
        let buffer = create_array_buffer(ctx, len * bytes_per_element);
        let result = create_typed_array(ctx, kind, buffer, 0, Some(len))?;

        if result.is_object() {
            let mut result_obj = result.as_object_mut();
            for i in 0..len {
                let val = if obj.is_array() && obj.is_dense_array() {
                    let ptr = obj as *const JSObject as usize;
                    let arr = unsafe { &*(ptr as *const crate::object::array_obj::JSArrayObject) };
                    arr.get(i)
                } else {
                    obj.get_indexed(i).or_else(|| {
                        let key = ctx.int_atom_mut(i);
                        obj.get(key)
                    })
                };
                if let Some(v) = val {
                    ta_set_element(ctx, &mut result_obj, i, v);
                }
            }
        }

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

fn typedarray_from(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let source = args.get(1).copied().unwrap_or(JSValue::undefined());
    let map_fn = args.get(2).copied().filter(|v| v.is_function());

    if source.is_null_or_undefined() {
        throw_type_error(ctx, "TypedArray.from requires an array-like object or iterable");
        return JSValue::undefined();
    }

    let mut items: Vec<JSValue> = Vec::new();

    if source.is_string() {
        let chars: Vec<char> = {
            let s = ctx.get_atom_str(source.get_atom());
            s.chars().collect()
        };
        for ch in chars {
            items.push(JSValue::new_string(ctx.intern(&ch.to_string())));
        }
    } else if source.is_object() {
        let obj = source.as_object();

        let sym_iter_atom = crate::builtins::symbol::get_symbol_iterator_atom(ctx);
        if let Some(iter_val) = obj.get(sym_iter_atom) {
            if iter_val.is_function() {
                match crate::builtins::array::call_callback_with_this(ctx, iter_val, source, &[]) {
                    Ok(iterator) => {
                        if iterator.is_object() {
                            let iter_obj = iterator.as_object();
                            let next_fn = iter_obj.get(ctx.intern("next"));
                            if let Some(next_val) = next_fn {
                                if next_val.is_function() {
                                    loop {
                                        match crate::builtins::array::call_callback_with_this(ctx, next_val, iterator, &[]) {
                                            Ok(result) => {
                                                if result.is_object() {
                                                    let robj = result.as_object();
                                                    let done = robj
                                                        .get(ctx.intern("done"))
                                                        .map(|v| v.is_truthy())
                                                        .unwrap_or(false);
                                                    if done {
                                                        break;
                                                    }
                                                    let value = robj
                                                        .get(ctx.intern("value"))
                                                        .unwrap_or(JSValue::undefined());
                                                    if let Some(mf) = map_fn {
                                                        match crate::builtins::array::call_callback(ctx, mf, &[value, JSValue::new_int(items.len() as i64), source]) {
                                                            Ok(v) => items.push(v),
                                                            Err(_) => return JSValue::undefined(),
                                                        }
                                                    } else {
                                                        items.push(value);
                                                    }
                                                } else {
                                                    break;
                                                }
                                            }
                                            Err(_) => return JSValue::undefined(),
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => return JSValue::undefined(),
                }
            }
        } else {
            let len = obj.get(ctx.common_atoms.length)
                .map(|v| {
                    if v.is_string() {
                        let s = ctx.get_atom_str(v.get_atom());
                        let n: f64 = s.parse().unwrap_or(f64::NAN);
                        if n.is_nan() || n <= 0.0 { 0usize }
                        else if n.is_infinite() { usize::MAX }
                        else { n as usize }
                    } else {
                        crate::builtins::string::js_to_length(&v) as usize
                    }
                })
                .unwrap_or(0);

            for i in 0..len {
                let val = crate::builtins::array::array_get(obj, i, ctx)
                    .unwrap_or(JSValue::undefined());
                if let Some(mf) = map_fn {
                    match crate::builtins::array::call_callback(ctx, mf, &[val, JSValue::new_int(i as i64), source]) {
                        Ok(v) => items.push(v),
                        Err(_) => return JSValue::undefined(),
                    }
                } else {
                    items.push(val);
                }
            }
        }
    }

    let len = items.len();
    let buffer = create_array_buffer(ctx, len * 8);
    match create_typed_array(ctx, TypedArrayKind::Float64, buffer, 0, Some(len)) {
        Ok(result) => {
            if result.is_object() {
                let mut result_obj = result.as_object_mut();
                for (i, &val) in items.iter().enumerate() {
                    ta_set_element(ctx, &mut result_obj, i, val);
                }
            }
            result
        }
        Err(e) => {
            throw_type_error(ctx, &e);
            JSValue::undefined()
        }
    }
}

fn typedarray_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let items: Vec<JSValue> = args[1..].to_vec();
    let len = items.len();
    let buffer = create_array_buffer(ctx, len * 8);
    match create_typed_array(ctx, TypedArrayKind::Float64, buffer, 0, Some(len)) {
        Ok(result) => {
            if result.is_object() {
                let mut result_obj = result.as_object_mut();
                for (i, &val) in items.iter().enumerate() {
                    ta_set_element(ctx, &mut result_obj, i, val);
                }
            }
            result
        }
        Err(e) => {
            throw_type_error(ctx, &e);
            JSValue::undefined()
        }
    }
}

fn typedarray_species_get(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if let Some(this_val) = args.first() {
        *this_val
    } else {
        JSValue::undefined()
    }
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

fn ta_check_this(args: &[JSValue], ctx: &mut JSContext) -> bool {
    if args.is_empty() || !args[0].is_object() {
        throw_type_error(ctx, "Method called on incompatible receiver");
        return false;
    }
    let obj = args[0].as_object();
    if obj.obj_type() != ObjectType::TypedArray {
        throw_type_error(ctx, "Method called on incompatible receiver");
        return false;
    }
    true
}

fn ta_get_length(ctx: &mut JSContext, obj: &JSObject) -> usize {
    obj.get(ctx.common_atoms.length)
        .map(|v| v.get_int() as usize)
        .unwrap_or(0)
}

pub fn ta_get_element(ctx: &mut JSContext, obj: &JSObject, index: usize) -> Option<JSValue> {
    let kind = obj.get_typed_array_kind()?;
    let bpe = kind.bytes_per_element();
    let byte_offset = obj.get(ctx.intern("byteOffset"))
        .map(|v| v.get_int() as usize)
        .unwrap_or(0);
    let buffer = obj.get(ctx.intern("buffer"))?;
    if !buffer.is_object() { return None; }
    let buf_obj = buffer.as_object();
    let data = get_array_buffer_data(&buf_obj)?;
    let byte_index = byte_offset + index * bpe;
    if byte_index + bpe > data.data.len() { return None; }
    let bytes = &data.data[byte_index..byte_index + bpe];
    Some(match kind {
        TypedArrayKind::Int8 => JSValue::new_int(bytes[0] as i8 as i64),
        TypedArrayKind::Uint8 | TypedArrayKind::Uint8Clamped => JSValue::new_int(bytes[0] as i64),
        TypedArrayKind::Int16 => JSValue::new_int(i16::from_le_bytes([bytes[0], bytes[1]]) as i64),
        TypedArrayKind::Uint16 => JSValue::new_int(u16::from_le_bytes([bytes[0], bytes[1]]) as i64),
        TypedArrayKind::Int32 => JSValue::new_int(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as i64),
        TypedArrayKind::Uint32 => JSValue::new_int(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as i64),
        TypedArrayKind::Float32 => JSValue::new_float(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64),
        TypedArrayKind::Float64 => JSValue::new_float(f64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]])),
        TypedArrayKind::BigInt64 => {
            let val = i64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]);
            let mut bi_obj = crate::object::object::JSObject::new_bigint();
            bi_obj.set_bigint_value(val as i128);
            JSValue::new_bigint(Box::into_raw(Box::new(bi_obj)) as usize)
        }
        TypedArrayKind::BigUint64 => {
            let val = u64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]);
            let mut bi_obj = crate::object::object::JSObject::new_bigint();
            bi_obj.set_bigint_value(val as i128);
            JSValue::new_bigint(Box::into_raw(Box::new(bi_obj)) as usize)
        }
    })
}

pub fn ta_set_element(ctx: &mut JSContext, obj: &mut JSObject, index: usize, value: JSValue) -> bool {
    let kind = match obj.get_typed_array_kind() {
        Some(k) => k,
        None => return false,
    };
    let bpe = kind.bytes_per_element();
    let byte_offset = obj.get(ctx.intern("byteOffset"))
        .map(|v| v.get_int() as usize)
        .unwrap_or(0);
    let buffer = match obj.get(ctx.intern("buffer")) {
        Some(b) => b,
        None => return false,
    };
    if !buffer.is_object() { return false; }
    let buf_obj = buffer.as_object();
    let data = match get_array_buffer_data(&buf_obj) {
        Some(d) => d,
        None => return false,
    };
    let byte_index = byte_offset + index * bpe;
    if byte_index + bpe > data.data.len() { return false; }
    let num = value.to_number();
    match kind {
        TypedArrayKind::Int8 => {
            let v = num as i8;
            data.data[byte_index..byte_index + bpe].copy_from_slice(&v.to_le_bytes());
        }
        TypedArrayKind::Uint8 | TypedArrayKind::Uint8Clamped => {
            let v = num as u8;
            data.data[byte_index..byte_index + bpe].copy_from_slice(&v.to_le_bytes());
        }
        TypedArrayKind::Int16 => {
            let v = num as i16;
            data.data[byte_index..byte_index + bpe].copy_from_slice(&v.to_le_bytes());
        }
        TypedArrayKind::Uint16 => {
            let v = num as u16;
            data.data[byte_index..byte_index + bpe].copy_from_slice(&v.to_le_bytes());
        }
        TypedArrayKind::Int32 => {
            let v = num as i32;
            data.data[byte_index..byte_index + bpe].copy_from_slice(&v.to_le_bytes());
        }
        TypedArrayKind::Uint32 => {
            let v = num as u32;
            data.data[byte_index..byte_index + bpe].copy_from_slice(&v.to_le_bytes());
        }
        TypedArrayKind::Float32 => {
            let v = num as f32;
            data.data[byte_index..byte_index + bpe].copy_from_slice(&v.to_le_bytes());
        }
        TypedArrayKind::Float64 => {
            let v = num;
            data.data[byte_index..byte_index + bpe].copy_from_slice(&v.to_le_bytes());
        }
        TypedArrayKind::BigInt64 | TypedArrayKind::BigUint64 => {
            let v = num as i64;
            data.data[byte_index..byte_index + bpe].copy_from_slice(&v.to_le_bytes());
        }
    }
    true
}

fn ta_call_callback(
    ctx: &mut JSContext,
    callback: JSValue,
    this_value: JSValue,
    args: &[JSValue],
) -> Result<JSValue, String> {
    if let Some(ptr) = ctx.get_register_vm_ptr() {
        let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
        vm.call_function_with_this(ctx, callback, this_value, args)
    } else {
        Err("call_callback: VM not available".to_string())
    }
}

fn typed_array_for_each(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }
    let this_arg = args.get(2).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    for i in 0..len {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            let cb_args = vec![val, JSValue::new_int(i as i64), args[0]];
            let _ = ta_call_callback(ctx, callback, this_arg, &cb_args);
        }
    }
    JSValue::undefined()
}

fn typed_array_every(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }
    let this_arg = args.get(2).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    for i in 0..len {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            let cb_args = vec![val, JSValue::new_int(i as i64), args[0]];
            match ta_call_callback(ctx, callback, this_arg, &cb_args) {
                Ok(result) => {
                    if !result.is_truthy() {
                        return JSValue::new_int(0);
                    }
                }
                Err(_) => return JSValue::undefined(),
            }
        }
    }
    JSValue::new_int(1)
}

fn typed_array_some(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }
    let this_arg = args.get(2).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    for i in 0..len {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            let cb_args = vec![val, JSValue::new_int(i as i64), args[0]];
            match ta_call_callback(ctx, callback, this_arg, &cb_args) {
                Ok(result) => {
                    if result.is_truthy() {
                        return JSValue::bool(true);
                    }
                }
                Err(_) => return JSValue::undefined(),
            }
        }
    }
    JSValue::new_int(0)
}

fn typed_array_find(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }
    let this_arg = args.get(2).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    for i in 0..len {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            let cb_args = vec![val, JSValue::new_int(i as i64), args[0]];
            match ta_call_callback(ctx, callback, this_arg, &cb_args) {
                Ok(result) => {
                    if result.is_truthy() {
                        return val;
                    }
                }
                Err(_) => return JSValue::undefined(),
            }
        }
    }
    JSValue::undefined()
}

fn typed_array_find_index(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }
    let this_arg = args.get(2).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    for i in 0..len {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            let cb_args = vec![val, JSValue::new_int(i as i64), args[0]];
            match ta_call_callback(ctx, callback, this_arg, &cb_args) {
                Ok(result) => {
                    if result.is_truthy() {
                        return JSValue::new_int(i as i64);
                    }
                }
                Err(_) => return JSValue::new_int(-1),
            }
        }
    }
    JSValue::new_int(-1)
}

fn typed_array_find_last(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }
    let this_arg = args.get(2).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    let mut result = JSValue::undefined();
    for i in 0..len {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            let cb_args = vec![val, JSValue::new_int(i as i64), args[0]];
            match ta_call_callback(ctx, callback, this_arg, &cb_args) {
                Ok(res) => {
                    if res.is_truthy() {
                        result = val;
                    }
                }
                Err(_) => return JSValue::undefined(),
            }
        }
    }
    result
}

fn typed_array_find_last_index(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }
    let this_arg = args.get(2).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    let mut result = JSValue::new_int(-1);
    for i in 0..len {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            let cb_args = vec![val, JSValue::new_int(i as i64), args[0]];
            match ta_call_callback(ctx, callback, this_arg, &cb_args) {
                Ok(res) => {
                    if res.is_truthy() {
                        result = JSValue::new_int(i as i64);
                    }
                }
                Err(_) => return JSValue::new_int(-1),
            }
        }
    }
    result
}

fn typed_array_reduce(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    let mut acc = JSValue::undefined();
    let mut start = 0;
    if len == 0 {
        if args.len() < 3 {
            throw_type_error(ctx, "Reduce of empty array with no initial value");
            return JSValue::undefined();
        }
        acc = args[2];
    } else if args.len() >= 3 {
        acc = args[2];
    } else {
        start = 1;
        acc = ta_get_element(ctx, obj, 0).unwrap_or(JSValue::undefined());
    }
    for i in start..len {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            let cb_args = vec![acc, val, JSValue::new_int(i as i64), args[0]];
            match ta_call_callback(ctx, callback, JSValue::undefined(), &cb_args) {
                Ok(res) => { acc = res; }
                Err(_) => return JSValue::undefined(),
            }
        }
    }
    acc
}

fn typed_array_reduce_right(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    let mut acc = JSValue::undefined();
    let mut end = len;
    if len == 0 {
        if args.len() < 3 {
            throw_type_error(ctx, "Reduce of empty array with no initial value");
            return JSValue::undefined();
        }
        acc = args[2];
    } else if args.len() >= 3 {
        acc = args[2];
    } else {
        end -= 1;
        acc = ta_get_element(ctx, obj, end).unwrap_or(JSValue::undefined());
    }
    for i in (0..end).rev() {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            let cb_args = vec![acc, val, JSValue::new_int(i as i64), args[0]];
            match ta_call_callback(ctx, callback, JSValue::undefined(), &cb_args) {
                Ok(res) => { acc = res; }
                Err(_) => return JSValue::undefined(),
            }
        }
    }
    acc
}

fn typed_array_map(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }
    let this_arg = args.get(2).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    let kind = obj.get_typed_array_kind().unwrap_or(TypedArrayKind::Int32);
    let result = typed_array_from_args(ctx, kind, &[
        JSValue::new_int(len as i64),
    ]);
    match result {
        Ok(r) => {
            let result_obj = r.as_object_mut();
            for i in 0..len {
                if let Some(val) = ta_get_element(ctx, obj, i) {
                    let cb_args = vec![val, JSValue::new_int(i as i64), args[0]];
                    match ta_call_callback(ctx, callback, this_arg, &cb_args) {
                        Ok(mapped) => { ta_set_element(ctx, result_obj, i, mapped); }
                        Err(_) => return JSValue::undefined(),
                    }
                }
            }
            r
        }
        Err(_) => JSValue::undefined(),
    }
}

fn typed_array_filter(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let callback = args.get(1).copied().unwrap_or(JSValue::undefined());
    if !callback.is_function() {
        throw_type_error(ctx, "Callback is not a function");
        return JSValue::undefined();
    }
    let this_arg = args.get(2).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    let kind = obj.get_typed_array_kind().unwrap_or(TypedArrayKind::Int32);
    let mut result: Vec<JSValue> = Vec::new();
    for i in 0..len {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            let cb_args = vec![val, JSValue::new_int(i as i64), args[0]];
            match ta_call_callback(ctx, callback, this_arg, &cb_args) {
                Ok(res) => {
                    if res.is_truthy() {
                        result.push(val);
                    }
                }
                Err(_) => return JSValue::undefined(),
            }
        }
    }
    let result_val = typed_array_from_args(ctx, kind, &[
        JSValue::new_int(result.len() as i64),
    ]);
    match result_val {
        Ok(r) => {
            let result_obj = r.as_object_mut();
            for (i, val) in result.iter().enumerate() {
                ta_set_element(ctx, result_obj, i, *val);
            }
            r
        }
        Err(_) => JSValue::undefined(),
    }
}

fn typed_array_index_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let search = args.get(1).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    let from_index = if args.len() >= 3 {
        let v = args[2].to_number() as i64;
        if v < 0 { (len as i64 + v).max(0) as usize } else { (v as usize).min(len) }
    } else {
        0
    };
    for i in from_index..len {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            if val.strict_eq(&search) {
                return JSValue::new_int(i as i64);
            }
        }
    }
    JSValue::new_int(-1)
}

fn typed_array_last_index_of(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let search = args.get(1).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    let from_index = if args.len() >= 3 {
        let v = args[2].to_number() as i64;
        if v < 0 { 0 } else { (v as usize).min(len.saturating_sub(1)) }
    } else {
        len.saturating_sub(1)
    };
    for i in (0..=from_index).rev() {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            if val.strict_eq(&search) {
                return JSValue::new_int(i as i64);
            }
        }
    }
    JSValue::new_int(-1)
}

fn typed_array_includes(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let search = args.get(1).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    let from_index = if args.len() >= 3 {
        let v = args[2].to_number() as i64;
        if v < 0 { (len as i64 + v).max(0) as usize } else { (v as usize).min(len) }
    } else {
        0
    };
    for i in from_index..len {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            if val.strict_eq(&search) {
                return JSValue::bool(true);
            }
            if search.is_float() && val.is_float() {
                let sv = search.to_number();
                let vv = val.to_number();
                if sv.is_nan() && vv.is_nan() {
                    return JSValue::bool(true);
                }
            }
        }
    }
    JSValue::new_int(0)
}

fn typed_array_join(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let separator = args.get(1).copied().unwrap_or(JSValue::undefined());
    let sep_str = if separator.is_undefined() {
        ",".to_string()
    } else if separator.is_string() {
        ctx.get_atom_str(separator.get_atom()).to_string()
    } else if separator.is_int() {
        separator.get_int().to_string()
    } else if separator.is_float() {
        separator.get_float().to_string()
    } else if separator.is_bool() {
        separator.get_bool().to_string()
    } else if separator.is_null() {
        "null".to_string()
    } else {
        ",".to_string()
    };
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj);
    let mut parts = Vec::new();
    for i in 0..len {
        if let Some(val) = ta_get_element(ctx, obj, i) {
            if val.is_undefined() || val.is_null() {
                parts.push(String::new());
            } else if val.is_string() {
                parts.push(ctx.get_atom_str(val.get_atom()).to_string());
            } else if val.is_int() {
                parts.push(val.get_int().to_string());
            } else if val.is_float() {
                parts.push(val.get_float().to_string());
            } else if val.is_bool() {
                parts.push(val.get_bool().to_string());
            } else {
                parts.push(String::new());
            }
        } else {
            parts.push(String::new());
        }
    }
    JSValue::new_string(ctx.intern(&parts.join(&sep_str)))
}

fn typed_array_at(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj) as i64;
    let relative_index = args.get(1).copied().unwrap_or(JSValue::new_int(0)).to_number() as i64;
    let k = if relative_index >= 0 {
        relative_index
    } else {
        len + relative_index
    };
    if k < 0 || k >= len {
        return JSValue::undefined();
    }
    ta_get_element(ctx, obj, k as usize).unwrap_or(JSValue::undefined())
}

fn typed_array_values(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    JSValue::undefined()
}

fn typed_array_keys(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    JSValue::undefined()
}

fn typed_array_entries(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    JSValue::undefined()
}

fn typed_array_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    typed_array_join(ctx, &[
        args[0],
        JSValue::undefined(),
    ])
}

fn typed_array_reverse(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let obj = args[0].as_object_mut();
    let len = ta_get_length(ctx, obj);
    let mut values: Vec<JSValue> = Vec::with_capacity(len);
    for i in 0..len {
        values.push(ta_get_element(ctx, obj, i).unwrap_or(JSValue::undefined()));
    }
    values.reverse();
    for i in 0..len {
        ta_set_element(ctx, obj, i, values[i]);
    }
    args[0]
}

fn typed_array_slice(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let obj = args[0].as_object();
    let len = ta_get_length(ctx, obj) as i64;
    let start = {
        let v = args.get(1).copied().unwrap_or(JSValue::new_int(0)).to_number() as i64;
        if v < 0 { (len + v).max(0) } else { v.min(len) }
    };
    let end = {
        let v = args.get(2).copied().unwrap_or(JSValue::new_int(len)).to_number() as i64;
        if v < 0 { (len + v).max(0) } else { v.min(len) }
    };
    let new_len = (end - start).max(0) as usize;
    let kind = obj.get_typed_array_kind().unwrap_or(TypedArrayKind::Int32);
    let result = typed_array_from_args(ctx, kind, &[
        JSValue::new_int(new_len as i64),
    ]);
    match result {
        Ok(r) => {
            let result_obj = r.as_object_mut();
            for i in 0..new_len {
                if let Some(val) = ta_get_element(ctx, obj, (start as usize) + i) {
                    ta_set_element(ctx, result_obj, i, val);
                }
            }
            r
        }
        Err(_) => JSValue::undefined(),
    }
}

fn typed_array_fill(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let value = args.get(1).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object_mut();
    let len = ta_get_length(ctx, obj) as i64;
    let start = {
        let v = args.get(2).copied().unwrap_or(JSValue::new_int(0)).to_number() as i64;
        if v < 0 { (len + v).max(0) } else { v.min(len) }
    };
    let end = {
        let v = args.get(3).copied().unwrap_or(JSValue::new_int(len)).to_number() as i64;
        if v < 0 { (len + v).max(0) } else { v.min(len) }
    };
    for i in start..end {
        ta_set_element(ctx, obj, i as usize, value);
    }
    args[0]
}

fn typed_array_copy_within(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let obj = args[0].as_object_mut();
    let len = ta_get_length(ctx, obj) as i64;
    let target = {
        let v = args.get(1).copied().unwrap_or(JSValue::new_int(0)).to_number() as i64;
        if v < 0 { (len + v).max(0) } else { v.min(len) }
    };
    let start = {
        let v = args.get(2).copied().unwrap_or(JSValue::new_int(0)).to_number() as i64;
        if v < 0 { (len + v).max(0) } else { v.min(len) }
    };
    let end = {
        let v = args.get(3).copied().unwrap_or(JSValue::new_int(len)).to_number() as i64;
        if v < 0 { (len + v).max(0) } else { v.min(len) }
    };
    let count = (end - start).min(len - target).max(0) as usize;
    let mut values = Vec::with_capacity(count);
    for i in 0..count {
        values.push(ta_get_element(ctx, obj, (start as usize) + i).unwrap_or(JSValue::undefined()));
    }
    for (i, val) in values.into_iter().enumerate() {
        ta_set_element(ctx, obj, (target as usize) + i, val);
    }
    args[0]
}

fn typed_array_sort(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if !ta_check_this(args, ctx) { return JSValue::undefined(); }
    let compare_fn = args.get(1).copied().unwrap_or(JSValue::undefined());
    let obj = args[0].as_object_mut();
    let len = ta_get_length(ctx, obj);
    if len <= 1 {
        return args[0];
    }
    let mut values: Vec<JSValue> = Vec::with_capacity(len);
    for i in 0..len {
        values.push(ta_get_element(ctx, obj, i).unwrap_or(JSValue::undefined()));
    }
    let compare_is_fn = compare_fn.is_function();
    values.sort_by(|a, b| {
        if compare_is_fn {
            let cb_args = vec![*a, *b];
            if let Some(ptr) = ctx.get_register_vm_ptr() {
                let vm = unsafe { &mut *(ptr as *mut crate::runtime::vm::VM) };
                if let Ok(result) = vm.call_function_with_this(ctx, compare_fn, JSValue::undefined(), &cb_args) {
                    let n = result.to_number();
                    if n < 0.0 { std::cmp::Ordering::Less }
                    else if n > 0.0 { std::cmp::Ordering::Greater }
                    else { std::cmp::Ordering::Equal }
                } else {
                    std::cmp::Ordering::Equal
                }
            } else {
                std::cmp::Ordering::Equal
            }
        } else {
            let a_str = a.to_number();
            let b_str = b.to_number();
            a_str.partial_cmp(&b_str).unwrap_or(std::cmp::Ordering::Equal)
        }
    });
    for (i, val) in values.into_iter().enumerate() {
        ta_set_element(ctx, obj, i, val);
    }
    args[0]
}

fn typed_array_subarray(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object() {
        throw_type_error(ctx, "Method called on incompatible receiver");
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    if obj.obj_type() != ObjectType::TypedArray {
        throw_type_error(ctx, "Method called on incompatible receiver");
        return JSValue::undefined();
    }
    let len = ta_get_length(ctx, obj) as i64;
    let buffer = obj.get(ctx.intern("buffer")).unwrap_or(JSValue::undefined());
    let byte_offset = obj.get(ctx.intern("byteOffset"))
        .map(|v| v.get_int())
        .unwrap_or(0);
    let element_size = obj.get_typed_array_kind()
        .map(|k| k.bytes_per_element() as i64)
        .unwrap_or(1);
    let start = {
        let v = args.get(1).copied().unwrap_or(JSValue::new_int(0)).to_number() as i64;
        if v < 0 { (len + v).max(0) } else { v.min(len) }
    };
    let end = {
        let v = args.get(2).copied().unwrap_or(JSValue::new_int(len)).to_number() as i64;
        if v < 0 { (len + v).max(0) } else { v.min(len) }
    };
    let new_len = (end - start).max(0) as usize;
    let new_byte_offset = byte_offset + start * element_size;
    let kind = obj.get_typed_array_kind().unwrap_or(TypedArrayKind::Int32);
    match create_typed_array(ctx, kind, buffer, new_byte_offset as usize, Some(new_len)) {
        Ok(v) => v,
        Err(_) => JSValue::undefined(),
    }
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

    let mut ta_proto = JSObject::new();
    ta_proto.set(ctx.intern("forEach"), create_builtin_function(ctx, "ta_forEach"));
    ta_proto.set(ctx.intern("every"), create_builtin_function(ctx, "ta_every"));
    ta_proto.set(ctx.intern("some"), create_builtin_function(ctx, "ta_some"));
    ta_proto.set(ctx.intern("find"), create_builtin_function(ctx, "ta_find"));
    ta_proto.set(ctx.intern("findIndex"), create_builtin_function(ctx, "ta_findIndex"));
    ta_proto.set(ctx.intern("findLast"), create_builtin_function(ctx, "ta_findLast"));
    ta_proto.set(ctx.intern("findLastIndex"), create_builtin_function(ctx, "ta_findLastIndex"));
    ta_proto.set(ctx.intern("reduce"), create_builtin_function(ctx, "ta_reduce"));
    ta_proto.set(ctx.intern("reduceRight"), create_builtin_function(ctx, "ta_reduceRight"));
    ta_proto.set(ctx.intern("map"), create_builtin_function(ctx, "ta_map"));
    ta_proto.set(ctx.intern("filter"), create_builtin_function(ctx, "ta_filter"));
    ta_proto.set(ctx.intern("indexOf"), create_builtin_function(ctx, "ta_indexOf"));
    ta_proto.set(ctx.intern("lastIndexOf"), create_builtin_function(ctx, "ta_lastIndexOf"));
    ta_proto.set(ctx.intern("includes"), create_builtin_function(ctx, "ta_includes"));
    ta_proto.set(ctx.intern("join"), create_builtin_function(ctx, "ta_join"));
    ta_proto.set(ctx.intern("at"), create_builtin_function(ctx, "ta_at"));
    ta_proto.set(ctx.intern("values"), create_builtin_function(ctx, "ta_values"));
    ta_proto.set(ctx.intern("keys"), create_builtin_function(ctx, "ta_keys"));
    ta_proto.set(ctx.intern("entries"), create_builtin_function(ctx, "ta_entries"));
    ta_proto.set(ctx.intern("toString"), create_builtin_function(ctx, "ta_toString"));
    ta_proto.set(ctx.intern("reverse"), create_builtin_function(ctx, "ta_reverse"));
    ta_proto.set(ctx.intern("slice"), create_builtin_function(ctx, "ta_slice"));
    ta_proto.set(ctx.intern("fill"), create_builtin_function(ctx, "ta_fill"));
    ta_proto.set(ctx.intern("copyWithin"), create_builtin_function(ctx, "ta_copyWithin"));
    ta_proto.set(ctx.intern("sort"), create_builtin_function(ctx, "ta_sort"));
    ta_proto.set(ctx.intern("subarray"), create_builtin_function(ctx, "ta_subarray"));

    let to_string_tag_key = crate::builtins::symbol::get_symbol_to_string_tag_prop_key(ctx);
    ta_proto.set(to_string_tag_key, JSValue::new_string(ctx.intern("TypedArray")));

    if let Some(obj_proto_ptr) = ctx.get_object_prototype() {
        ta_proto.prototype = Some(obj_proto_ptr);
    }
    let ta_proto_ptr = Box::into_raw(Box::new(ta_proto)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ta_proto_ptr);
    ctx.set_typed_array_prototype(ta_proto_ptr);

    let ta_names = [
        "Int8Array", "Uint8Array", "Uint8ClampedArray",
        "Int16Array", "Uint16Array", "Int32Array", "Uint32Array",
        "Float32Array", "Float64Array", "BigInt64Array", "BigUint64Array",
    ];
    for name in &ta_names {
        let atom = ctx.intern(name);
        if let Some(ctor_val) = global_obj.get(atom) {
            if ctor_val.is_function() {
                let ctor_obj = ctor_val.as_object_mut();
                ctor_obj.set(ctx.intern("prototype"), JSValue::new_object(ta_proto_ptr));
                ctor_obj.set(ctx.intern("from"), create_builtin_function(ctx, "typedarray_from"));
                ctor_obj.set(ctx.intern("of"), create_builtin_function(ctx, "typedarray_of"));
                ctor_obj.set(ctx.intern("name"), JSValue::new_string(ctx.intern(name)));
                ctor_obj.set(ctx.intern("length"), JSValue::new_int(1));
                let species_fn = create_builtin_function(ctx, "typedarray_species_get");
                let species_sym = crate::builtins::symbol::get_symbol_species(ctx);
                if !species_sym.is_undefined() {
                    ctor_obj.set(species_sym.get_atom(), species_fn);
                }
            }
        }
    }

    let ta_sym_tag = crate::builtins::symbol::get_symbol_to_string_tag(ctx);
    if !ta_sym_tag.is_undefined() {
        if let Some(proto_obj) = ctx.get_typed_array_prototype() {
            unsafe {
                (*proto_obj).set(ta_sym_tag.get_atom(), JSValue::new_string(ctx.intern("TypedArray")));
            }
        }
    }
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
    ctx.register_builtin(
        "ta_forEach",
        HostFunction::method("forEach", 1, typed_array_for_each),
    );
    ctx.register_builtin(
        "ta_every",
        HostFunction::method("every", 1, typed_array_every),
    );
    ctx.register_builtin(
        "ta_some",
        HostFunction::method("some", 1, typed_array_some),
    );
    ctx.register_builtin(
        "ta_find",
        HostFunction::method("find", 1, typed_array_find),
    );
    ctx.register_builtin(
        "ta_findIndex",
        HostFunction::method("findIndex", 1, typed_array_find_index),
    );
    ctx.register_builtin(
        "ta_findLast",
        HostFunction::method("findLast", 1, typed_array_find_last),
    );
    ctx.register_builtin(
        "ta_findLastIndex",
        HostFunction::method("findLastIndex", 1, typed_array_find_last_index),
    );
    ctx.register_builtin(
        "ta_reduce",
        HostFunction::method("reduce", 1, typed_array_reduce),
    );
    ctx.register_builtin(
        "ta_reduceRight",
        HostFunction::method("reduceRight", 1, typed_array_reduce_right),
    );
    ctx.register_builtin(
        "ta_map",
        HostFunction::method("map", 1, typed_array_map),
    );
    ctx.register_builtin(
        "ta_filter",
        HostFunction::method("filter", 1, typed_array_filter),
    );
    ctx.register_builtin(
        "ta_indexOf",
        HostFunction::method("indexOf", 1, typed_array_index_of),
    );
    ctx.register_builtin(
        "ta_lastIndexOf",
        HostFunction::method("lastIndexOf", 1, typed_array_last_index_of),
    );
    ctx.register_builtin(
        "ta_includes",
        HostFunction::method("includes", 1, typed_array_includes),
    );
    ctx.register_builtin(
        "ta_join",
        HostFunction::method("join", 1, typed_array_join),
    );
    ctx.register_builtin(
        "ta_at",
        HostFunction::method("at", 1, typed_array_at),
    );
    ctx.register_builtin(
        "ta_values",
        HostFunction::method("values", 0, typed_array_values),
    );
    ctx.register_builtin(
        "ta_keys",
        HostFunction::method("keys", 0, typed_array_keys),
    );
    ctx.register_builtin(
        "ta_entries",
        HostFunction::method("entries", 0, typed_array_entries),
    );
    ctx.register_builtin(
        "ta_toString",
        HostFunction::method("toString", 0, typed_array_to_string),
    );
    ctx.register_builtin(
        "ta_reverse",
        HostFunction::method("reverse", 0, typed_array_reverse),
    );
    ctx.register_builtin(
        "ta_slice",
        HostFunction::method("slice", 2, typed_array_slice),
    );
    ctx.register_builtin(
        "ta_fill",
        HostFunction::method("fill", 1, typed_array_fill),
    );
    ctx.register_builtin(
        "ta_copyWithin",
        HostFunction::method("copyWithin", 2, typed_array_copy_within),
    );
    ctx.register_builtin(
        "ta_sort",
        HostFunction::method("sort", 1, typed_array_sort),
    );
    ctx.register_builtin(
        "ta_subarray",
        HostFunction::method("subarray", 2, typed_array_subarray),
    );
    ctx.register_builtin(
        "typedarray_from",
        HostFunction::method("from", 1, typedarray_from),
    );
    ctx.register_builtin(
        "typedarray_of",
        HostFunction::method("of", 0, typedarray_of),
    );
    ctx.register_builtin(
        "typedarray_species_get",
        HostFunction::new("get [Symbol.species]", 0, typedarray_species_get),
    );
}
