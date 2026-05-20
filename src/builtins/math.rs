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
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
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
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    JSValue::new_float(args[arg_idx].get_float().ceil())
}

fn math_round(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    JSValue::new_float(args[arg_idx].get_float().round())
}

fn math_sqrt(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    JSValue::new_float(args[arg_idx].get_float().sqrt())
}

fn math_max(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let args = if !args.is_empty() && args[0].is_object() {
        &args[1..]
    } else {
        args
    };
    if args.is_empty() {
        return JSValue::new_float(f64::NEG_INFINITY);
    }
    let mut max = args[0].get_float();
    for arg in args.iter().skip(1) {
        let v = arg.get_float();
        if v > max {
            max = v;
        }
    }
    JSValue::new_float(max)
}

fn math_min(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let args = if !args.is_empty() && args[0].is_object() {
        &args[1..]
    } else {
        args
    };
    if args.is_empty() {
        return JSValue::new_float(f64::INFINITY);
    }
    let mut min = args[0].get_float();
    for arg in args.iter().skip(1) {
        let v = arg.get_float();
        if v < min {
            min = v;
        }
    }
    JSValue::new_float(min)
}

fn math_pow(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.len() < 2 {
        return JSValue::new_float(f64::NAN);
    }
    JSValue::new_float(args[0].get_float().powf(args[1].get_float()))
}

fn math_random(_ctx: &mut JSContext, _args: &[JSValue]) -> JSValue {
    JSValue::new_float(rand_simple())
}

fn math_trunc(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.trunc())
}

fn math_sign(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
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
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.cbrt())
}

fn math_clz32(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_int(32);
    };
    let v = args[arg_idx].get_int();
    let bits = (v as u32).leading_zeros();
    JSValue::new_int(bits as i64)
}

fn math_imul(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let a = if args.len() >= 2 {
        args[1].get_int() as i32
    } else {
        0
    };
    let b = if args.len() >= 3 {
        args[2].get_int() as i32
    } else {
        0
    };
    let result = a.wrapping_mul(b);
    JSValue::new_int(result as i64)
}

fn math_fround(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float() as f32;
    JSValue::new_float(v as f64)
}

fn math_hypot(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let args = if !args.is_empty() && args[0].is_object() {
        &args[1..]
    } else {
        args
    };
    let mut sum_squares = 0.0;
    for arg in args {
        let v = arg.get_float();
        sum_squares += v * v;
    }
    JSValue::new_float(sum_squares.sqrt())
}

fn math_expm1(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
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
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float((1.0 + v).ln())
}

fn math_log10(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.log10())
}

fn math_log2(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.log2())
}

fn math_sinh(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.sinh())
}

fn math_cosh(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.cosh())
}

fn math_tanh(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.tanh())
}

fn math_asinh(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.asinh())
}

fn math_acos(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.acos())
}

fn math_acosh(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.acosh())
}

fn math_sin(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
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
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.cos())
}

fn math_tan(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.tan())
}

fn math_asin(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
    let v = args[arg_idx].get_float();
    JSValue::new_float(v.asin())
}

fn math_atan(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
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
    let arg_idx = if args.len() >= 2 {
        1
    } else if !args.is_empty() {
        0
    } else {
        return JSValue::new_float(f64::NAN);
    };
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
