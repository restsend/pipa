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
    if args.is_empty() || !args[0].is_object() {
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    obj.get(ctx.intern("buffer"))
        .unwrap_or(JSValue::undefined())
}

fn data_view_get_byte_length(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object() {
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    obj.get(ctx.intern("byteLength"))
        .unwrap_or(JSValue::undefined())
}

fn data_view_get_byte_offset(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() || !args[0].is_object() {
        return JSValue::undefined();
    }
    let obj = args[0].as_object();
    obj.get(ctx.intern("byteOffset"))
        .unwrap_or(JSValue::undefined())
}

fn data_view_get_int8(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::undefined();
    }
    let view = &args[0];
    let offset = args[1].get_int() as usize;

    if !view.is_object() {
        return JSValue::undefined();
    }

    let view_obj = view.as_object();
    let buffer = view_obj.get(ctx.intern("buffer"));
    let byte_offset = view_obj
        .get(ctx.intern("byteOffset"))
        .map(|v| v.get_int() as usize)
        .unwrap_or(0);

    if let Some(buf) = buffer {
        if let Some(buf_obj) = if buf.is_object() {
            Some(buf.as_object())
        } else {
            None
        } {
            if let Some(data) = get_array_buffer_data(&buf_obj) {
                let idx = byte_offset + offset;
                if idx < data.data.len() {
                    return JSValue::new_int(data.data[idx] as i8 as i64);
                }
            }
        }
    }
    JSValue::undefined()
}

fn data_view_get_uint8(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::undefined();
    }
    let view = &args[0];
    let offset = args[1].get_int() as usize;

    if !view.is_object() {
        return JSValue::undefined();
    }

    let view_obj = view.as_object();
    let buffer = view_obj.get(ctx.intern("buffer"));
    let byte_offset = view_obj
        .get(ctx.intern("byteOffset"))
        .map(|v| v.get_int() as usize)
        .unwrap_or(0);

    if let Some(buf) = buffer {
        if let Some(buf_obj) = if buf.is_object() {
            Some(buf.as_object())
        } else {
            None
        } {
            if let Some(data) = get_array_buffer_data(&buf_obj) {
                let idx = byte_offset + offset;
                if idx < data.data.len() {
                    return JSValue::new_int(data.data[idx] as i64);
                }
            }
        }
    }
    JSValue::undefined()
}

fn data_view_set_int8(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 {
        return JSValue::undefined();
    }
    let view = &args[0];
    let offset = args[1].get_int() as usize;
    let value = args[2].get_int() as i8 as u8;

    if !view.is_object() {
        return JSValue::undefined();
    }

    let view_obj = view.as_object();
    let buffer = view_obj.get(ctx.intern("buffer"));
    let byte_offset = view_obj
        .get(ctx.intern("byteOffset"))
        .map(|v| v.get_int() as usize)
        .unwrap_or(0);

    if let Some(buf) = buffer {
        if let Some(buf_obj) = if buf.is_object() {
            Some(buf.as_object())
        } else {
            None
        } {
            if let Some(data) = get_array_buffer_data(&buf_obj) {
                let idx = byte_offset + offset;
                if idx < data.data.len() {
                    data.data[idx] = value;
                }
            }
        }
    }
    JSValue::undefined()
}

fn data_view_set_uint8(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 3 {
        return JSValue::undefined();
    }
    let view = &args[0];
    let offset = args[1].get_int() as usize;
    let value = args[2].get_int() as u8;

    if !view.is_object() {
        return JSValue::undefined();
    }

    let view_obj = view.as_object();
    let buffer = view_obj.get(ctx.intern("buffer"));
    let byte_offset = view_obj
        .get(ctx.intern("byteOffset"))
        .map(|v| v.get_int() as usize)
        .unwrap_or(0);

    if let Some(buf) = buffer {
        if let Some(buf_obj) = if buf.is_object() {
            Some(buf.as_object())
        } else {
            None
        } {
            if let Some(data) = get_array_buffer_data(&buf_obj) {
                let idx = byte_offset + offset;
                if idx < data.data.len() {
                    data.data[idx] = value;
                }
            }
        }
    }
    JSValue::undefined()
}

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = crate::object::function::JSFunction::new_builtin(ctx.intern(name), 1);
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

    global_obj.set(
        ctx.intern("ArrayBuffer"),
        create_builtin_function(ctx, "ArrayBuffer"),
    );

    global_obj.set(
        ctx.intern("DataView"),
        create_builtin_function(ctx, "DataView"),
    );

    global_obj.set(
        ctx.intern("Int8Array"),
        create_builtin_function(ctx, "Int8Array"),
    );
    global_obj.set(
        ctx.intern("Uint8Array"),
        create_builtin_function(ctx, "Uint8Array"),
    );
    global_obj.set(
        ctx.intern("Uint8ClampedArray"),
        create_builtin_function(ctx, "Uint8ClampedArray"),
    );
    global_obj.set(
        ctx.intern("Int16Array"),
        create_builtin_function(ctx, "Int16Array"),
    );
    global_obj.set(
        ctx.intern("Uint16Array"),
        create_builtin_function(ctx, "Uint16Array"),
    );
    global_obj.set(
        ctx.intern("Int32Array"),
        create_builtin_function(ctx, "Int32Array"),
    );
    global_obj.set(
        ctx.intern("Uint32Array"),
        create_builtin_function(ctx, "Uint32Array"),
    );
    global_obj.set(
        ctx.intern("Float32Array"),
        create_builtin_function(ctx, "Float32Array"),
    );
    global_obj.set(
        ctx.intern("Float64Array"),
        create_builtin_function(ctx, "Float64Array"),
    );
    global_obj.set(
        ctx.intern("BigInt64Array"),
        create_builtin_function(ctx, "BigInt64Array"),
    );
    global_obj.set(
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
}
