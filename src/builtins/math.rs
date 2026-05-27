use crate::host::HostFunction;
use crate::object::function::JSFunction;
use crate::runtime::context::JSContext;
use crate::runtime::vm::VM;
use crate::value::JSValue;

pub fn register_math_builtins(ctx: &mut JSContext) {
    ctx.register_builtin("math_abs", HostFunction::new("abs", 1, math_abs));
    ctx.register_builtin("math_floor", HostFunction::new("floor", 1, math_floor));
    ctx.register_builtin("math_ceil", HostFunction::new("ceil", 1, math_ceil));
    ctx.register_builtin("math_round", HostFunction::new("round", 1, math_round));
    ctx.register_builtin("math_sqrt", HostFunction::new("sqrt", 1, math_sqrt));
    ctx.register_builtin("math_max", HostFunction::new("max", 2, math_max));
    ctx.register_builtin("math_min", HostFunction::new("min", 2, math_min));
    ctx.register_builtin("math_pow", HostFunction::new("pow", 2, math_pow));
    ctx.register_builtin("math_log", HostFunction::new("log", 1, math_log));
    ctx.register_builtin("math_random", HostFunction::new("random", 0, math_random));
    ctx.register_builtin("math_trunc", HostFunction::new("trunc", 1, math_trunc));
    ctx.register_builtin("math_sign", HostFunction::new("sign", 1, math_sign));
    ctx.register_builtin("math_cbrt", HostFunction::new("cbrt", 1, math_cbrt));
    ctx.register_builtin("math_clz32", HostFunction::new("clz32", 1, math_clz32));
    ctx.register_builtin("math_imul", HostFunction::new("imul", 2, math_imul));
    ctx.register_builtin("math_fround", HostFunction::new("fround", 1, math_fround));
    ctx.register_builtin("math_hypot", HostFunction::new("hypot", 2, math_hypot));
    ctx.register_builtin("math_expm1", HostFunction::new("expm1", 1, math_expm1));
    ctx.register_builtin("math_log1p", HostFunction::new("log1p", 1, math_log1p));
    ctx.register_builtin("math_log10", HostFunction::new("log10", 1, math_log10));
    ctx.register_builtin("math_log2", HostFunction::new("log2", 1, math_log2));
    ctx.register_builtin("math_sin", HostFunction::new("sin", 1, math_sin));
    ctx.register_builtin("math_cos", HostFunction::new("cos", 1, math_cos));
    ctx.register_builtin("math_tan", HostFunction::new("tan", 1, math_tan));
    ctx.register_builtin("math_exp", HostFunction::new("exp", 1, math_exp));
    ctx.register_builtin("math_asin", HostFunction::new("asin", 1, math_asin));
    ctx.register_builtin("math_acos", HostFunction::new("acos", 1, math_acos));
    ctx.register_builtin("math_atan", HostFunction::new("atan", 1, math_atan));
    ctx.register_builtin("math_atan2", HostFunction::new("atan2", 2, math_atan2));
    ctx.register_builtin("math_f16round", HostFunction::new("f16round", 1, math_f16round));
    ctx.register_builtin("math_sumPrecise", HostFunction::new("sumPrecise", 1, math_sum_precise));
    ctx.register_builtin("math_sinh", HostFunction::new("sinh", 1, math_sinh));
    ctx.register_builtin("math_cosh", HostFunction::new("cosh", 1, math_cosh));
    ctx.register_builtin("math_tanh", HostFunction::new("tanh", 1, math_tanh));
    ctx.register_builtin("math_asinh", HostFunction::new("asinh", 1, math_asinh));
    ctx.register_builtin("math_acosh", HostFunction::new("acosh", 1, math_acosh));
    ctx.register_builtin("math_atanh", HostFunction::new("atanh", 1, math_atanh));
}

fn call_js_function(
    ctx: &mut JSContext,
    func: JSValue,
    this_value: JSValue,
    args: &[JSValue],
) -> Result<JSValue, String> {
    if !func.is_function() {
        return Err("not a function".to_string());
    }
    let ptr = func.get_ptr();
    let js_func = unsafe { &*(ptr as *const JSFunction) };

    if js_func.is_builtin() {
        if let Some(builtin_fn) = js_func.builtin_func {
            return Ok(ctx.call_builtin_direct(builtin_fn, args));
        } else if let Some(builtin_atom) = js_func.builtin_atom {
            let builtin_name = ctx.get_atom_str(builtin_atom).to_string();
            return Ok(ctx.call_builtin(&builtin_name, args));
        }
        return Ok(JSValue::undefined());
    }

    if let Some(ref _rb) = js_func.bytecode {
        let vm_ptr = ctx
            .get_register_vm_ptr()
            .ok_or("No VM associated with context")?;
        let vm = unsafe { &mut *(vm_ptr as *mut VM) };
        vm.call_function_with_this(ctx, func, this_value, args)
    } else {
        Ok(JSValue::undefined())
    }
}

fn to_number_with_valueof(ctx: &mut JSContext, value: &JSValue) -> f64 {
    if value.is_int() {
        return value.get_int() as f64;
    }
    if value.is_float() {
        return value.get_float();
    }
    if value.is_bool() {
        return if value.get_bool() { 1.0 } else { 0.0 };
    }
    if value.is_null() {
        return 0.0;
    }
    if value.is_undefined() {
        return f64::NAN;
    }
    if value.is_object() {
        let obj = value.as_object();
        let value_of_atom = ctx.common_atoms.value_of;
        if let Some(func) = obj.get(value_of_atom) {
            if func.is_function() {
                let result = call_js_function(ctx, func, *value, &[]);
                if let Ok(result) = result {
                    return to_number_with_valueof(ctx, &result);
                }
            }
        }

        let to_string_atom = ctx.common_atoms.to_string;
        if let Some(func) = obj.get(to_string_atom) {
            if func.is_function() {
                let result = call_js_function(ctx, func, *value, &[]);
                if let Ok(result) = result {
                    if result.is_string() {
                        let s = ctx.get_atom_str(result.get_atom());
                        if let Ok(n) = s.parse::<f64>() {
                            return n;
                        }
                    }
                }
            }
        }
    }
    f64::NAN
}

fn math_abs(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg = if args.len() >= 2 {
        &args[1]
    } else if !args.is_empty() {
        &args[0]
    } else {
        return JSValue::new_float(f64::NAN);
    };
    if arg.is_int() {
        let v = arg.get_int();
        JSValue::new_int(if v < 0 { -v } else { v })
    } else {
        JSValue::new_float(arg.get_float().abs())
    }
}

fn math_floor(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let arg = &args[arg_idx];

    if arg.is_int() {
        return *arg;
    }
    if arg.is_bigint() {
        return *arg;
    }
    JSValue::new_float(arg.get_float().floor())
}

fn math_ceil(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    JSValue::new_float(args[arg_idx].get_float().ceil())
}

fn math_round(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    JSValue::new_float(args[arg_idx].get_float().round())
}

fn math_sqrt(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    JSValue::new_float(args[arg_idx].get_float().sqrt())
}

fn math_max(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NEG_INFINITY);
    }
    let mut max = args[0].to_number();
    for arg in args.iter().skip(1) {
        let v = arg.to_number();
        if v.is_nan() {
            max = v;
        } else if v > max {
            max = v;
        } else if v == 0.0 && max == 0.0 && !v.is_sign_negative() && max.is_sign_negative() {
            max = v;
        }
    }
    JSValue::new_float(max)
}

fn math_min(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::INFINITY);
    }
    let mut min = args[0].to_number();
    for arg in args.iter().skip(1) {
        let v = arg.to_number();
        if v.is_nan() {
            min = v;
        } else if v < min {
            min = v;
        } else if v == 0.0 && min == 0.0 && v.is_sign_negative() && !min.is_sign_negative() {
            min = v;
        }
    }
    JSValue::new_float(min)
}

fn math_pow(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::new_float(f64::NAN);
    }
    let x = args[0].get_float();
    let y = args[1].get_float();
    if y.is_nan() {
        return JSValue::new_float(f64::NAN);
    }
    if x.abs() == 1.0 && y.is_infinite() {
        return JSValue::new_float(f64::NAN);
    }
    JSValue::new_float(x.powf(y))
}

fn math_random(_ctx: &mut JSContext, _args: &[JSValue]) -> JSValue {
    JSValue::new_float(rand_simple())
}

fn math_trunc(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.trunc())
}

fn math_sign(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    if v.is_nan() || v == 0.0 {
        JSValue::new_float(v)
    } else if v > 0.0 {
        JSValue::new_int(1)
    } else {
        JSValue::new_int(-1)
    }
}

fn math_cbrt(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.cbrt())
}

fn math_clz32(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_int(32);
    }
    let uint32 = to_uint32(args[0].get_float());
    let bits = uint32.leading_zeros();
    JSValue::new_int(bits as i64)
}

fn to_uint32(f: f64) -> u32 {
    if f.is_nan() || f.is_infinite() || f == 0.0 {
        0
    } else {
        let int = f.trunc();
        let int128 = int as i128;
        let modulo = ((int128 % (1i128 << 32)) + (1i128 << 32)) % (1i128 << 32);
        modulo as u32
    }
}

fn to_int32(f: f64) -> i32 {
    let uint32 = to_uint32(f);
    if uint32 >= 0x80000000 {
        (uint32 as i64 - 0x100000000i64) as i32
    } else {
        uint32 as i32
    }
}

fn math_imul(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let a = if args.len() >= 1 {
        to_int32(args[0].get_float())
    } else {
        0i32
    };
    let b = if args.len() >= 2 {
        to_int32(args[1].get_float())
    } else {
        0i32
    };
    JSValue::new_int(a.wrapping_mul(b) as i64)
}

fn math_fround(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float() as f32;
    JSValue::new_float(v as f64)
}

fn math_f16round(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    }
    let x = args[0].to_number();
    if x.is_nan() {
        return JSValue::new_float(f64::NAN);
    }
    if x == 0.0 || x.is_infinite() {
        return JSValue::new_float(x);
    }
    let f16 = f16_from_f64(x);
    JSValue::new_float(f16)
}

fn f16_from_f64(x: f64) -> f64 {
    let sign_bit = if x < 0.0 { 0x8000u16 } else { 0u16 };
    let abs_x = x.abs();

    if abs_x < 2.0f64.powi(-25) {
        return if x < 0.0 { -0.0 } else { 0.0 };
    }
    if abs_x == 2.0f64.powi(-25) {
        return if x < 0.0 { -0.0 } else { 0.0 };
    }
    if abs_x < 2.0f64.powi(-24) {
        let result = 2.0f64.powi(-24);
        return if x < 0.0 { -result } else { result };
    }
    if abs_x > 65520.0 {
        return if x < 0.0 { f64::NEG_INFINITY } else { f64::INFINITY };
    }

    let bits = abs_x.to_bits();
    let exp64 = ((bits >> 52) & 0x7FF) as i32;
    let mant64 = bits & 0x000F_FFFF_FFFF_FFFF;

    let (exp_unbiased, full_mant): (i32, u64) = if exp64 == 0 {
        let lz = mant64.leading_zeros() as i32;
        let shift = lz - 11;
        (-1021 - shift, mant64 << (shift + 1))
    } else {
        (exp64 - 1023, (1u64 << 52) | mant64)
    };

    let f16_exp_biased = exp_unbiased + 15;

    if f16_exp_biased < 1 {
        let sub_shift = (1 - f16_exp_biased) as u32;
        if sub_shift > 10 {
            return if x < 0.0 { -0.0 } else { 0.0 };
        }
        let shift = 42 + sub_shift;
        let half_bit = 1u64 << (shift - 1);
        let lost = full_mant & ((1u64 << shift) - 1);
        let sig = full_mant >> shift;
        let rounded = sig
            + if lost > half_bit || (lost == half_bit && sig & 1 != 0) {
                1u64
            } else {
                0u64
            };
        let f16_bits = sign_bit | rounded as u16;
        f16_to_f64(f16_bits)
    } else {
        let shift = 42u32;
        let half_bit = 1u64 << (shift - 1);
        let lost = full_mant & ((1u64 << shift) - 1);
        let sig = (full_mant >> shift) as u16;
        let rounded = sig
            + if lost > half_bit || (lost == half_bit && sig & 1 != 0) {
                1u16
            } else {
                0u16
            };
        if rounded >= 0x800 {
            let new_exp = f16_exp_biased + 1;
            if new_exp >= 31 {
                let f16_bits = sign_bit | 0x7C00;
                return f16_to_f64(f16_bits);
            }
            let f16_bits = sign_bit | ((new_exp as u16) << 10);
            f16_to_f64(f16_bits)
        } else {
            let f16_bits = sign_bit | ((f16_exp_biased as u16) << 10) | (rounded & 0x3FF);
            f16_to_f64(f16_bits)
        }
    }
}

fn f16_to_f64(bits: u16) -> f64 {
    let sign = (bits >> 15) & 1;
    let exp = ((bits >> 10) & 0x1F) as i32;
    let mant = (bits & 0x3FF) as f64;
    let val = if exp == 0 {
        if mant == 0.0 {
            0.0
        } else {
            2.0f64.powi(-14) * mant / 1024.0
        }
    } else if exp == 31 {
        if mant == 0.0 {
            f64::INFINITY
        } else {
            f64::NAN
        }
    } else {
        2.0f64.powi(exp - 15) * (1.0 + mant / 1024.0)
    };
    if sign != 0 {
        -val
    } else {
        val
    }
}

fn math_hypot(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(0.0);
    }
    let mut has_inf = false;
    let mut sum_squares = 0.0;
    for arg in args {
        let v = arg.to_number();
        if v.is_infinite() {
            has_inf = true;
        }
        sum_squares += v * v;
    }
    if has_inf {
        return JSValue::new_float(f64::INFINITY);
    }
    JSValue::new_float(sum_squares.sqrt())
}

fn math_expm1(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.exp_m1())
}

fn math_log(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    }
    let v = to_number_with_valueof(ctx, &args[0]);
    JSValue::new_float(v.ln())
}

fn math_log1p(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.ln_1p())
}

fn math_log10(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.log10())
}

fn math_log2(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.log2())
}

fn math_sinh(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.sinh())
}

fn math_cosh(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.cosh())
}

fn math_tanh(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.tanh())
}

fn math_asinh(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.asinh())
}

fn math_acos(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.acos())
}

fn math_acosh(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.acosh())
}

fn math_sin(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.sin())
}

fn math_exp(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    }
    let v = args[0].get_float();
    JSValue::new_float(v.exp())
}

fn math_cos(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.cos())
}

fn math_tan(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.tan())
}

fn math_asin(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.asin())
}

fn math_atan(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.atan())
}

fn math_atan2(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let args = if !args.is_empty() && args[0].is_object() {
        &args[1..]
    } else {
        args
    };
    if args.len() < 2 {
        return JSValue::new_float(f64::NAN);
    }
    let y = args[0].get_float();
    let x = args[1].get_float();
    JSValue::new_float(y.atan2(x))
}

fn math_atanh(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::new_float(f64::NAN);
    };
    let arg_idx = 0;
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.atanh())
}

fn rand_simple() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos as f64) / (u32::MAX as f64)
}

fn throw_type_error_math(ctx: &mut JSContext, msg: &str) -> JSValue {
    let mut err = crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
    err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
    err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern(msg)));
    if let Some(proto) = ctx.get_type_error_prototype() {
        err.prototype = Some(proto);
    }
    let ptr = Box::into_raw(Box::new(err)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(ptr);
    ctx.pending_exception = Some(JSValue::new_object(ptr));
    JSValue::undefined()
}

fn math_sum_precise(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        let mut err = crate::object::object::JSObject::new_typed(crate::object::object::ObjectType::Error);
        err.set(ctx.common_atoms.name, JSValue::new_string(ctx.intern("TypeError")));
        err.set(ctx.common_atoms.message, JSValue::new_string(ctx.intern("sumPrecise requires 1 argument")));
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        ctx.pending_exception = Some(JSValue::new_object(ptr));
        return JSValue::undefined();
    }
    let iterable = &args[0];
    let mut values: Vec<f64> = Vec::new();

    let sym_iter_val = crate::builtins::symbol::get_or_create_well_known_symbol(ctx, "Symbol.iterator");
    let has_iterator = iterable.is_object() && {
        let sym_key = crate::runtime::atom::Atom(0x40000000 | sym_iter_val.get_symbol_id());
        let obj = unsafe { &*(iterable.get_ptr() as *const crate::object::JSObject) };
        obj.get(sym_key).map_or(false, |v| v.is_function())
    };

     if has_iterator {
         let vm_ptr = match ctx.get_register_vm_ptr() {
             Some(p) => p,
             None => return JSValue::new_float(f64::NAN),
         };
         let vm = unsafe { &mut *(vm_ptr as *mut VM) };

        let iter_fn = {
            let sym_key = crate::runtime::atom::Atom(0x40000000 | sym_iter_val.get_symbol_id());
            let obj = unsafe { &*(iterable.get_ptr() as *const crate::object::JSObject) };
            obj.get(sym_key).unwrap()
        };

        let iterator = match vm.call_function_with_this(ctx, iter_fn, iterable.clone(), &[]) {
            Ok(v) => v,
            Err(_) => return JSValue::undefined(),
        };
        if ctx.pending_exception.is_some() {
            return JSValue::undefined();
        }
        if !iterator.is_object() {
            return throw_type_error_math(ctx, "iterator is not an object");
        }

        let next_atom = ctx.intern("next");
        let next_fn = {
            let obj = unsafe { &*(iterator.get_ptr() as *const crate::object::JSObject) };
            match obj.get(next_atom) {
                Some(v) if v.is_function() => v,
                _ => return throw_type_error_math(ctx, "iterator has no next method"),
            }
        };

        loop {
            let result = match vm.call_function_with_this(ctx, next_fn, iterator.clone(), &[]) {
                Ok(v) => v,
                Err(_) => return JSValue::undefined(),
            };
            if ctx.pending_exception.is_some() {
                return JSValue::undefined();
            }
            if !result.is_object() {
                return throw_type_error_math(ctx, "iterator result is not an object");
            }
            let result_obj = unsafe { &*(result.get_ptr() as *const crate::object::JSObject) };
            let done_atom = ctx.intern("done");
            let value_atom = ctx.intern("value");
            let done = result_obj.get(done_atom).map_or(false, |v| v.is_bool() && v.get_bool());
            if done {
                break;
            }
            let value = match result_obj.get(value_atom) {
                Some(v) => v,
                None => JSValue::undefined(),
            };

            if value.is_object() || value.is_bigint() || value.is_symbol() || value.is_function() {
                let return_atom = ctx.intern("return");
                let iter_obj = unsafe { &*(iterator.get_ptr() as *const crate::object::JSObject) };
                if let Some(ret_fn) = iter_obj.get(return_atom) {
                    if ret_fn.is_function() {
                        let _ = vm.call_function_with_this(ctx, ret_fn, iterator.clone(), &[]);
                    }
                }
                return throw_type_error_math(ctx, "element is not a number");
            }
            values.push(value.to_number());
        }
    } else if iterable.is_object() {
        let obj = unsafe { &*(iterable.get_ptr() as *const crate::object::JSObject) };
        let length_prop = obj.get(ctx.common_atoms.length);
        let len = if let Some(v) = length_prop {
            if v.is_int() {
                v.get_int() as usize
            } else if v.is_float() {
                v.get_float() as usize
            } else {
                0
            }
        } else {
            return throw_type_error_math(ctx, "argument is not iterable");
        };
        for i in 0..len {
            let val = if obj.is_dense_array() {
                use crate::object::array_obj::JSArrayObject;
                let arr = unsafe { &*(iterable.get_ptr() as *const JSArrayObject) };
                if i < arr.elements.len() {
                    arr.elements[i]
                } else {
                    JSValue::undefined()
                }
            } else {
                let atom = crate::runtime::atom::Atom(i as u32);
                obj.get(atom).unwrap_or(JSValue::undefined())
            };
            if val.is_object() || val.is_bigint() {
                return throw_type_error_math(ctx, "element is not a number");
            }
            values.push(val.to_number());
        }
    } else {
        return throw_type_error_math(ctx, "argument is not iterable");
    }
    if values.is_empty() {
        return JSValue::new_float(-0.0);
    }
    let mut has_nan = false;
    let mut pos_inf = false;
    let mut neg_inf = false;
    let mut finite_values: Vec<f64> = Vec::new();
    for &n in &values {
        if n.is_nan() {
            has_nan = true;
        } else if n == f64::INFINITY {
            pos_inf = true;
        } else if n == f64::NEG_INFINITY {
            neg_inf = true;
        } else if n > 0.0 || (n == 0.0 && !n.is_sign_negative()) {
            finite_values.push(n);
        } else {
            finite_values.push(n);
        }
    }
    if has_nan { return JSValue::new_float(f64::NAN); }
    if pos_inf && neg_inf { return JSValue::new_float(f64::NAN); }
    if pos_inf { return JSValue::new_float(f64::INFINITY); }
    if neg_inf { return JSValue::new_float(f64::NEG_INFINITY); }
    if finite_values.is_empty() {
        return JSValue::new_float(-0.0);
    }
    if finite_values.len() == 1 {
        return JSValue::new_float(finite_values[0]);
    }
    let result = exact_float_sum(&finite_values);
    if result == 0.0 {
        let has_positive = finite_values.iter().any(|&x| x > 0.0 || (x == 0.0 && !x.is_sign_negative()));
        let has_negative = finite_values.iter().any(|&x| x < 0.0 || (x == 0.0 && x.is_sign_negative()));
        if has_positive && has_negative {
            return JSValue::new_float(0.0);
        }
        if finite_values.iter().all(|&x| x == 0.0 && x.is_sign_negative()) {
            return JSValue::new_float(-0.0);
        }
        let has_pos_zero = finite_values.iter().any(|&x| x == 0.0 && !x.is_sign_negative());
        if has_pos_zero {
            return JSValue::new_float(0.0);
        }
        return JSValue::new_float(-0.0);
    }
    JSValue::new_float(result)
}

fn decompose_float(f: f64) -> (i64, i32) {
    let bits = f.to_bits();
    let sign = if (bits >> 63) != 0 { -1i64 } else { 1i64 };
    let exp_bits = ((bits >> 52) & 0x7FF) as i32;
    let mant = (bits & 0xFFFFFFFFFFFFF) as i64;
    if exp_bits == 0 {
        (sign * mant, -1074)
    } else {
        (sign * (mant + (1i64 << 52)), exp_bits - 1023 - 52)
    }
}

fn exact_float_sum(values: &[f64]) -> f64 {
    let comps: Vec<(i64, i32)> = values.iter().map(|&v| decompose_float(v)).collect();
    let min_exp = comps.iter().map(|&(_, e)| e).min().unwrap_or(0);
    let max_exp = comps.iter().map(|&(_, e)| e).max().unwrap_or(0);
    let total_shift = (max_exp - min_exp) as usize;
    let limbs_needed = (total_shift + 63) / 64 + 2;
    let mut pos_limbs = vec![0u64; limbs_needed];
    let mut neg_limbs = vec![0u64; limbs_needed];
    for &(mant, exp) in &comps {
        let shift = (exp - min_exp) as usize;
        let limb_shift = shift / 64;
        let bit_shift = shift % 64;
        let abs_mant = mant.unsigned_abs();
        if mant >= 0 {
            add_to_bigint(&mut pos_limbs, abs_mant, limb_shift, bit_shift);
        } else {
            add_to_bigint(&mut neg_limbs, abs_mant, limb_shift, bit_shift);
        }
    }
    let (result_sign, result_limbs) = if cmp_bigint(&pos_limbs, &neg_limbs) >= 0 {
        (false, sub_bigint(&pos_limbs, &neg_limbs))
    } else {
        (true, sub_bigint(&neg_limbs, &pos_limbs))
    };
    let first_nonzero = result_limbs.iter().rposition(|&l| l != 0).unwrap_or(0);
    if first_nonzero == 0 && result_limbs[0] == 0 {
        return if result_sign { -0.0 } else { 0.0 };
    }
    let total_bits = first_nonzero * 64 + (64 - result_limbs[first_nonzero].leading_zeros() as usize);
    let exp = min_exp + total_bits as i32 - 1;
    let biased_exp = exp + 1023;
    if biased_exp >= 0x7FF {
        return if result_sign { f64::NEG_INFINITY } else { f64::INFINITY };
    }
    if biased_exp <= 0 {
        return if result_sign { -0.0 } else { 0.0 };
    }
    let top_limb = first_nonzero;
    let bits_in_top = 64 - result_limbs[top_limb].leading_zeros() as usize;
    let top_val = result_limbs[top_limb];
    let next_val = if top_limb > 0 { result_limbs[top_limb - 1] } else { 0 };
    let next_next_val = if top_limb > 1 { result_limbs[top_limb - 2] } else { 0 };
    let result = if bits_in_top >= 53 {
        let shift = bits_in_top - 53;
        let mant = (top_val >> shift) as u64;
        let round_bit = if shift > 0 { (top_val >> (shift - 1)) & 1 } else { 0 };
        let sticky = if shift > 1 {
            (top_val & ((1u64 << (shift - 1)) - 1)) != 0
        } else if shift == 1 {
            false
        } else {
            (next_val >> 63) != 0
        };
        round_mantissa_to_float(mant, round_bit != 0, sticky, biased_exp, result_sign)
    } else if bits_in_top > 0 && bits_in_top + 64 >= 53 {
        let shift = 53 - bits_in_top;
        let mut mant = (top_val << shift) as u128;
        if shift <= 64 {
            mant |= (next_val >> (64 - shift)) as u128;
        }
        let round_bit = if shift < 64 { ((next_val >> (64 - shift - 1)) & 1) != 0 } else { false };
        let sticky = if shift < 63 {
            (next_val & ((1u64 << (64 - shift - 1)) - 1)) != 0 || next_next_val != 0
        } else {
            next_next_val != 0
        };
        round_mantissa_to_float(mant as u64, round_bit, sticky, biased_exp, result_sign)
    } else {
        if result_sign { -0.0 } else { 0.0 }
    };
    result
}

fn add_to_bigint(limbs: &mut [u64], val: u64, limb_idx: usize, bit_shift: usize) {
    if limb_idx >= limbs.len() || val == 0 { return; }
    let (lo, hi) = if bit_shift == 0 {
        (val, 0u64)
    } else {
        (val << bit_shift, val >> (64 - bit_shift))
    };
    let mut carry = 0u64;
    {
        let sum = limbs[limb_idx] as u128 + lo as u128;
        limbs[limb_idx] = sum as u64;
        carry = (sum >> 64) as u64;
    }
    if hi != 0 || carry != 0 {
        let idx = limb_idx + 1;
        if idx < limbs.len() {
            let sum = limbs[idx] as u128 + hi as u128 + carry as u128;
            limbs[idx] = sum as u64;
            carry = (sum >> 64) as u64;
            let mut cidx = idx + 1;
            while carry != 0 && cidx < limbs.len() {
                let sum = limbs[cidx] as u128 + carry as u128;
                limbs[cidx] = sum as u64;
                carry = (sum >> 64) as u64;
                cidx += 1;
            }
        }
    }
}

fn cmp_bigint(a: &[u64], b: &[u64]) -> i32 {
    for i in (0..a.len()).rev() {
        if a[i] > b[i] { return 1; }
        if a[i] < b[i] { return -1; }
    }
    0
}

fn sub_bigint(a: &[u64], b: &[u64]) -> Vec<u64> {
    let mut result = vec![0u64; a.len()];
    let mut borrow = 0u64;
    for i in 0..a.len() {
        let (diff1, borrow1) = a[i].overflowing_sub(b[i]);
        let (diff2, borrow2) = diff1.overflowing_sub(borrow);
        result[i] = diff2;
        borrow = borrow1 as u64 + borrow2 as u64;
    }
    result
}

fn round_mantissa_to_float(mant: u64, round_bit: bool, sticky: bool, mut biased_exp: i32, negative: bool) -> f64 {
    let mant_53_mask = (1u64 << 53) - 1;
    let mut mant = mant & mant_53_mask;
    if mant == 0 && !round_bit {
        return if negative { -0.0 } else { 0.0 };
    }
    if round_bit && (sticky || (mant & 1) != 0) {
        mant = mant.wrapping_add(1);
        if mant > mant_53_mask {
            mant >>= 1;
            biased_exp += 1;
        }
    }
    if biased_exp >= 0x7FF {
        return if negative { f64::NEG_INFINITY } else { f64::INFINITY };
    }
    if biased_exp <= 0 {
        return if negative { -0.0 } else { 0.0 };
    }
    let mant_bits = mant & ((1u64 << 52) - 1);
    let sign_bit = if negative { 1u64 << 63 } else { 0 };
    let exp_bits = (biased_exp as u64) << 52;
    f64::from_bits(sign_bit | exp_bits | mant_bits)
}

fn naive_sum(values: &[f64]) -> f64 {
    let mut sum = 0.0f64;
    for &x in values { sum += x; }
    sum
}

fn kbn_sum(values: &[f64]) -> f64 {
    let mut sum = 0.0f64;
    let mut c = 0.0f64;
    for &x in values {
        let y = x - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
    }
    sum
}

fn precise_sum(values: &[f64]) -> f64 {
    let mut merged: Vec<f64> = Vec::new();
    let mut positives: Vec<f64> = values.iter().filter(|&&x| x > 0.0).copied().collect::<Vec<_>>();
    let mut negatives: Vec<f64> = values.iter().filter(|&&x| x < 0.0).copied().collect::<Vec<_>>();
    positives.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    negatives.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let (mut pi, mut ni) = (0usize, 0usize);
    let mut last_was_neg = false;
    while pi < positives.len() || ni < negatives.len() {
        let p_abs = if pi < positives.len() { positives[pi].abs() } else { 0.0 };
        let n_abs = if ni < negatives.len() { negatives[ni].abs() } else { 0.0 };
        let take_neg = if ni >= negatives.len() { false }
            else if pi >= positives.len() { true }
            else if n_abs > p_abs { true }
            else if p_abs > n_abs { false }
            else { !last_was_neg };
        if take_neg {
            merged.push(negatives[ni]);
            ni += 1;
            last_was_neg = true;
        } else {
            merged.push(positives[pi]);
            pi += 1;
            last_was_neg = false;
        }
    }
    let mut sum = 0.0f64;
    let mut c = 0.0f64;
    for &x in &merged {
        let y = x + c;
        let t = sum + y;
        if sum.abs() >= y.abs() {
            c = (sum - t) + y;
        } else {
            c = (y - t) + sum;
        }
        sum = t;
    }
    sum + c
}

fn shewchuk_sum_finite(values: &[f64]) -> f64 {
    let mut partials: Vec<f64> = Vec::new();
    for &x in values {
        let mut y = x;
        let mut new_partials = Vec::with_capacity(partials.len() + 1);
        for &p in &partials {
            let (hi, lo) = two_sum(p, y);
            y = lo;
            if hi != 0.0 { new_partials.push(hi); }
        }
        if y != 0.0 { new_partials.push(y); }
        partials = new_partials;
    }
    let mut sum = 0.0f64;
    for &p in &partials { sum += p; }
    sum
}

fn shewchuk_sum(values: &[f64]) -> f64 {
    let mut partials: Vec<f64> = Vec::new();
    for &x in values {
        let mut y = x;
        let mut j = 0;
        for i in 0..partials.len() {
            let p = partials[i];
            if y.abs() > p.abs() {
                let (hi, lo) = two_sum(y, p);
                y = lo;
                partials[j] = hi;
            } else {
                let (hi, lo) = two_sum(p, y);
                y = lo;
                partials[j] = hi;
            }
            j += 1;
        }
        partials.truncate(j);
        if y != 0.0 {
            partials.push(y);
        }
    }
    let mut sum = 0.0f64;
    let mut err = 0.0f64;
    for &p in &partials {
        let new_sum = sum + p;
        if sum.abs() >= p.abs() {
            err += (sum - new_sum) + p;
        } else {
            err += (p - new_sum) + sum;
        }
        sum = new_sum;
    }
    sum + err
}

fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let v = s - a;
    let lo = (a - (s - v)) + (b - v);
    (s, lo)
}
