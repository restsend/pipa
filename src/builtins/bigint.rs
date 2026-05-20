use crate::host::HostFunction;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

fn create_builtin_function(ctx: &mut JSContext, name: &str) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(name), 2);
    func.set_builtin_marker(ctx, name);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

pub fn init_bigint(ctx: &mut JSContext) {
    let bigint_atom = ctx.intern("BigInt");

    let mut func = JSFunction::new_builtin(bigint_atom, 1);
    func.set_builtin_marker(ctx, "bigint_constructor");
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    let bigint_value = JSValue::new_function(ptr);

    let bigint_func_obj = bigint_value.as_object_mut();
    bigint_func_obj.set(
        ctx.intern("asIntN"),
        create_builtin_function(ctx, "bigint_as_int_n"),
    );
    bigint_func_obj.set(
        ctx.intern("asUintN"),
        create_builtin_function(ctx, "bigint_as_uint_n"),
    );

    let proto_atom = ctx.intern("BigIntPrototype");
    let mut proto_obj = JSObject::new();
    proto_obj.set(
        ctx.intern("toString"),
        create_builtin_function(ctx, "bigint_to_string"),
    );
    let proto_ptr = Box::into_raw(Box::new(proto_obj)) as usize;
    let proto_value = JSValue::new_object(proto_ptr);

    let bigint_func = bigint_value.as_object_mut();
    bigint_func.set(ctx.intern("prototype"), proto_value);

    let global = ctx.global();
    if global.is_object() {
        let global_obj = global.as_object_mut();
        global_obj.set(bigint_atom, bigint_value);
        global_obj.set(proto_atom, proto_value);
    }
}

pub fn register_builtins(ctx: &mut JSContext) {
    ctx.register_builtin(
        "bigint_constructor",
        HostFunction::new("BigInt", 1, bigint_constructor),
    );
    ctx.register_builtin(
        "bigint_as_int_n",
        HostFunction::new("asIntN", 2, bigint_as_int_n),
    );
    ctx.register_builtin(
        "bigint_as_uint_n",
        HostFunction::new("asUintN", 2, bigint_as_uint_n),
    );
    ctx.register_builtin(
        "bigint_to_string",
        HostFunction::new("toString", 1, bigint_to_string),
    );
}

fn bigint_constructor(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let value = args.first().copied().unwrap_or(JSValue::new_int(0));

    if value.is_int() {
        let int_val = value.get_int() as i128;
        let mut bigint_obj = JSObject::new_bigint();
        bigint_obj.set_bigint_value(int_val);
        let obj_ptr = Box::into_raw(Box::new(bigint_obj)) as usize;
        return JSValue::new_bigint(obj_ptr);
    }

    if value.is_bigint() {
        return value;
    }

    if value.is_string() {
        let atom = value.get_atom();
        let s = ctx.get_atom_str(atom);
        let cleaned = s.trim_end_matches('n').replace('_', "");
        let c = cleaned.as_str();
        let (radix, num_str) = if c.starts_with("0x") || c.starts_with("0X") {
            (16, &c[2..])
        } else if c.starts_with("0b") || c.starts_with("0B") {
            (2, &c[2..])
        } else if c.starts_with("0o") || c.starts_with("0O") {
            (8, &c[2..])
        } else {
            (10, &c[..])
        };
        let val = i128::from_str_radix(num_str, radix).ok();
        if let Some(v) = val {
            let mut bigint_obj = JSObject::new_bigint();
            bigint_obj.set_bigint_value(v);
            let obj_ptr = Box::into_raw(Box::new(bigint_obj)) as usize;
            return JSValue::new_bigint(obj_ptr);
        }
    }

    let mut bigint_obj = JSObject::new_bigint();
    bigint_obj.set_bigint_value(0i128);
    let obj_ptr = Box::into_raw(Box::new(bigint_obj)) as usize;
    JSValue::new_bigint(obj_ptr)
}

fn bigint_as_int_n(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_offset = if args.len() >= 3 { 1 } else { 0 };
    let bits = args
        .get(arg_offset)
        .map(|v| v.get_int() as u32)
        .unwrap_or(0);
    let bigint_val = args
        .get(arg_offset + 1)
        .copied()
        .unwrap_or(JSValue::new_int(0));

    let value: i128 = if bigint_val.is_bigint() {
        bigint_val.as_object().get_bigint_value()
    } else if bigint_val.is_int() {
        bigint_val.get_int() as i128
    } else {
        0i128
    };

    if bits == 0 {
        let mut result = JSObject::new_bigint();
        result.set_bigint_value(0i128);
        return JSValue::new_bigint(Box::into_raw(Box::new(result)) as usize);
    }

    let wrapped = if bits >= 128 {
        value
    } else {
        let max = 1i128 << (bits - 1);
        let mask = (1i128 << bits) - 1;
        let masked = value & mask;
        if masked >= max {
            masked - (1i128 << bits)
        } else {
            masked
        }
    };

    let mut result = JSObject::new_bigint();
    result.set_bigint_value(wrapped);
    JSValue::new_bigint(Box::into_raw(Box::new(result)) as usize)
}

fn bigint_as_uint_n(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_offset = if args.len() >= 3 { 1 } else { 0 };
    let bits = args
        .get(arg_offset)
        .map(|v| v.get_int() as u32)
        .unwrap_or(0);
    let bigint_val = args
        .get(arg_offset + 1)
        .copied()
        .unwrap_or(JSValue::new_int(0));

    let value: i128 = if bigint_val.is_bigint() {
        bigint_val.as_object().get_bigint_value()
    } else if bigint_val.is_int() {
        bigint_val.get_int() as i128
    } else {
        0i128
    };

    if bits == 0 {
        let mut result = JSObject::new_bigint();
        result.set_bigint_value(0i128);
        return JSValue::new_bigint(Box::into_raw(Box::new(result)) as usize);
    }

    let wrapped = if bits >= 128 {
        value
    } else {
        let mask = (1u128 << bits) - 1;
        (value as u128 & mask) as i128
    };

    let mut result = JSObject::new_bigint();
    result.set_bigint_value(wrapped);
    JSValue::new_bigint(Box::into_raw(Box::new(result)) as usize)
}

fn bigint_to_string(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let this_val = args.first().copied().unwrap_or(JSValue::new_int(0));
    let radix = args.get(1).map(|v| v.get_int() as u32).unwrap_or(10);

    let value: i128 = if this_val.is_bigint() {
        this_val.as_object().get_bigint_value()
    } else if this_val.is_int() {
        this_val.get_int() as i128
    } else {
        0i128
    };

    let result = if radix == 16 {
        format!("{:x}", value)
    } else if radix == 8 {
        format!("{:o}", value)
    } else if radix == 2 {
        format!("{:b}", value)
    } else {
        value.to_string()
    };

    JSValue::new_string(ctx.intern(&result))
}
