use super::promise;
use crate::host::HostFunction;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::value::JSValue;

pub fn init_process_module(ctx: &mut JSContext) {
    let specifier = "pipa:process";

    ctx.register_builtin("process_exit", HostFunction::new("exit", 1, process_exit));
    ctx.register_builtin("process_cwd", HostFunction::new("cwd", 0, process_cwd));
    ctx.register_builtin("process_exec", HostFunction::new("exec", 2, process_exec));

    let mut module = crate::runtime::module::Module::new(specifier.to_string(), String::new());

    module.add_export("argv".to_string(), make_argv(ctx), false);
    module.add_export(
        "argc".to_string(),
        JSValue::new_int(ctx.runtime().argv.len() as i64),
        false,
    );
    module.add_export("env".to_string(), make_env(ctx), false);

    let (cwd_fn, exit_fn, exec_fn) = make_functions(ctx);
    module.add_export("cwd".to_string(), cwd_fn, false);
    module.add_export("exit".to_string(), exit_fn, false);
    module.add_export("exec".to_string(), exec_fn, false);

    ctx.runtime_mut().module_registry_mut().register(module);
}

fn make_argv(ctx: &mut JSContext) -> JSValue {
    let args: Vec<String> = ctx.runtime().argv.clone();
    let mut arr = JSObject::new_array();
    if let Some(proto_ptr) = ctx.get_array_prototype() {
        arr.prototype = Some(proto_ptr);
    }
    arr.set(ctx.common_atoms.length, JSValue::new_int(args.len() as i64));

    for (i, arg) in args.iter().enumerate() {
        let val = JSValue::new_string(ctx.intern(arg));
        let atom = ctx.intern(&i.to_string());
        arr.set(atom, val);
    }

    let ptr = Box::into_raw(Box::new(arr)) as usize;
    JSValue::new_object(ptr)
}

fn make_env(ctx: &mut JSContext) -> JSValue {
    let mut obj = JSObject::new();
    for (key, val) in std::env::vars() {
        let key_atom = ctx.intern(&key);
        obj.set(key_atom, JSValue::new_string(ctx.intern(&val)));
    }
    let ptr = Box::into_raw(Box::new(obj)) as usize;
    JSValue::new_object(ptr)
}

fn make_functions(ctx: &mut JSContext) -> (JSValue, JSValue, JSValue) {
    let cwd = make_function_value(ctx, "cwd", 0, "process_cwd");
    let exit = make_function_value(ctx, "exit", 1, "process_exit");
    let exec = make_function_value(ctx, "exec", 2, "process_exec");
    (cwd, exit, exec)
}

fn make_function_value(ctx: &mut JSContext, name: &str, arity: u32, marker: &str) -> JSValue {
    let mut func = JSFunction::new_builtin(ctx.intern(name), arity);
    func.set_builtin_marker(ctx, marker);
    let ptr = Box::into_raw(Box::new(func)) as usize;
    ctx.runtime_mut().gc_heap_mut().track_function(ptr);
    JSValue::new_function(ptr)
}

fn process_exit(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let code = if args.is_empty() || !args[0].is_int() {
        0
    } else {
        args[0].get_int() as i32
    };
    std::process::exit(code);
}

fn process_cwd(ctx: &mut JSContext, _args: &[JSValue]) -> JSValue {
    match std::env::current_dir() {
        Ok(p) => JSValue::new_string(ctx.intern(&p.to_string_lossy().to_string())),
        Err(e) => {
            let s = format!("{}", e);
            let msg = JSValue::new_string(ctx.intern(&s));
            promise::create_rejected_promise(ctx, msg)
        }
    }
}

fn process_exec(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    let start_idx = if args.len() >= 3 { 1 } else { 0 };
    let real_args = &args[start_idx..];

    if real_args.is_empty() || !real_args[0].is_string() {
        let msg = JSValue::new_string(ctx.intern("exec: command must be a string"));
        return promise::create_rejected_promise(ctx, msg);
    }

    let command = ctx.get_atom_str(real_args[0].get_atom()).to_string();

    let cmd_args: Vec<String> = parse_js_array(ctx, real_args.get(1));

    let result = crate::runtime::process_task::run_command_sync(&command, &cmd_args);

    let mut promise_obj = JSObject::new_promise();
    if let Some(proto_ptr) = ctx.get_promise_prototype() {
        promise_obj.prototype = Some(proto_ptr);
    }
    promise_obj.set(ctx.intern("__promise_state__"), JSValue::new_int(0));
    promise_obj.set(ctx.intern("__promise_result__"), JSValue::undefined());
    promise_obj.set(ctx.intern("__promise_reactions__"), JSValue::null());
    let promise_ptr = Box::into_raw(Box::new(promise_obj)) as usize;
    let promise_val = JSValue::new_object(promise_ptr);

    if let Some(e) = result.error {
        let msg = JSValue::new_string(ctx.intern(&e));
        super::promise::reject_promise_with_value(ctx, promise_ptr, msg);
    } else {
        let mut result_obj = crate::object::object::JSObject::new();
        result_obj.set(
            ctx.intern("stdout"),
            JSValue::new_string(ctx.intern(&String::from_utf8_lossy(&result.stdout))),
        );
        result_obj.set(
            ctx.intern("stderr"),
            JSValue::new_string(ctx.intern(&String::from_utf8_lossy(&result.stderr))),
        );
        result_obj.set(ctx.intern("code"), JSValue::new_int(result.code as i64));
        result_obj.set(
            ctx.intern("signal"),
            if let Some(sig) = result.signal {
                JSValue::new_int(sig as i64)
            } else {
                JSValue::null()
            },
        );
        let result_ptr = Box::into_raw(Box::new(result_obj)) as usize;
        super::promise::fulfill_promise_with_value(
            ctx,
            promise_ptr,
            JSValue::new_object(result_ptr),
        );
    }

    promise_val
}

fn parse_js_array(ctx: &mut JSContext, arg: Option<&JSValue>) -> Vec<String> {
    let obj_val = match arg {
        Some(v) if v.is_object() => v,
        _ => return Vec::new(),
    };
    let obj = obj_val.as_object();

    let len = if obj.is_dense_array() {
        let arr_ptr = obj_val.get_ptr() as *const crate::object::array_obj::JSArrayObject;
        unsafe { &*arr_ptr }.len()
    } else {
        let len_val = obj.get(ctx.intern("length")).unwrap_or(JSValue::new_int(0));
        if len_val.is_int() {
            len_val.get_int() as usize
        } else {
            0
        }
    };

    let mut result = Vec::with_capacity(len);
    for i in 0..len {
        if obj.is_dense_array() {
            let arr_ptr = obj_val.get_ptr() as *const crate::object::array_obj::JSArrayObject;
            let arr = unsafe { &*arr_ptr };
            if let Some(val) = arr.get(i) {
                if val.is_string() {
                    result.push(ctx.get_atom_str(val.get_atom()).to_string());
                }
            }
        } else {
            let atom = ctx.intern(&i.to_string());
            if let Some(val) = obj.get(atom) {
                if val.is_string() {
                    result.push(ctx.get_atom_str(val.get_atom()).to_string());
                }
            }
        }
    }
    result
}
