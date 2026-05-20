use pipa::host::HostFunction;
use pipa::object::function::JSFunction;
use pipa::object::object::JSObject;
use pipa::runtime::context::JSContext;
use pipa::{
    JSRuntime, JSValue, OptLevel, compile_to_bytecode_with_opt_level, eval, eval_with_opt_level,
};
use std::fs;

#[cfg(feature = "repl")]
use rustyline::error::ReadlineError;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    #[cfg(feature = "repl")]
    if args.len() < 2 {
        start_repl();
        return;
    }

    #[cfg(not(feature = "repl"))]
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let mut arg_index = 1usize;
    let mut opt_level = OptLevel::default();
    if let Some(level) = args.get(arg_index).and_then(|arg| OptLevel::from_flag(arg)) {
        opt_level = level;
        arg_index += 1;
    }

    if args.len() <= arg_index {
        print_usage();
        std::process::exit(1);
    }

    let cmd = &args[arg_index];

    match cmd.as_str() {
        "-compile" => {
            if args.len() < arg_index + 3 {
                eprintln!("Usage: pipa [-O0|-O1|-O2|-O3] -compile <input.js> <output.jsc>");
                std::process::exit(1);
            }
            let input_path = &args[arg_index + 1];
            let output_path = &args[arg_index + 2];
            compile_js_to_jsc(input_path, output_path, opt_level);
        }
        "-diss" => {
            if args.len() < arg_index + 2 {
                eprintln!("Usage: pipa [-O0|-O1|-O2|-O3] -diss <input.jsc>");
                std::process::exit(1);
            }
            let input_path = &args[arg_index + 1];
            disassemble_jsc(input_path);
        }
        _ => {
            let script_path = cmd;
            if script_path.ends_with(".jsc") {
                execute_jsc(script_path);
            } else {
                execute_js(script_path, opt_level);
            }
        }
    }
}

fn print_usage() {
    eprintln!("Usage: pipa [-O0|-O1|-O2|-O3] <script.js|script.jsc>");
    eprintln!("       pipa [-O0|-O1|-O2|-O3] -compile <input.js> <output.jsc>");
    eprintln!("       pipa [-O0|-O1|-O2|-O3] -diss <input.jsc>");
    eprintln!();
    eprintln!("A simple JavaScript runtime with setTimeout, console, and import support.");
    eprintln!("GC debug: PIPA_GCDUMP=1|summary|roots|live|dead|alloc|all");
    eprintln!("          PIPA_GCDUMP_LIMIT=<n> to cap per-collection detail lines");
    eprintln!();
    eprintln!("Global functions:");
    eprintln!("  setTimeout(fn, ms)    - Execute fn after ms milliseconds");
    eprintln!("  setInterval(fn, ms)   - Execute fn every ms milliseconds");
    eprintln!("  clearTimeout(id)     - Cancel a timer");
    eprintln!("  import(specifier)    - Import a JavaScript module");
    eprintln!("  console.log(...)     - Log to stdout");
    eprintln!("  console.error(...)   - Log to stderr");
}

fn setup_context(ctx: &mut JSContext) {
    ctx.register_builtin("print", HostFunction::new("print", 1, print_fn));
    ctx.register_builtin("pipa_load", HostFunction::new("load", 1, load_fn));

    let global = ctx.global();
    let global_ptr = global.get_ptr();
    let global_obj = unsafe { &mut *(global_ptr as *mut JSObject) };

    let print_fn_val = {
        let mut func = JSFunction::new_builtin(ctx.intern("print"), 1);
        func.set_builtin_marker(ctx, "print");
        let ptr = Box::into_raw(Box::new(func)) as usize;
        JSValue::new_function(ptr)
    };
    global_obj.set(ctx.intern("print"), print_fn_val);

    let load_fn_val = {
        let mut func = JSFunction::new_builtin(ctx.intern("pipa_load"), 1);
        func.set_builtin_marker(ctx, "pipa_load");
        let ptr = Box::into_raw(Box::new(func)) as usize;
        JSValue::new_function(ptr)
    };
    global_obj.set(ctx.intern("load"), load_fn_val);

    if let Err(e) = eval(ctx, "function alert() {}") {
        eprintln!("Warning: Failed to define alert function: {}", e);
    }
}

fn compile_js_to_jsc(input_path: &str, output_path: &str, opt_level: OptLevel) {
    let code = match fs::read_to_string(input_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {}: {}", input_path, e);
            std::process::exit(1);
        }
    };

    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    setup_context(&mut ctx);

    let bytecode_obj = match compile_to_bytecode_with_opt_level(&mut ctx, &code, opt_level) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("[DEBUG binary] eval ERROR: {}", e);
            std::process::exit(1);
        }
    };

    let serialized = bytecode_obj.serialize();
    if let Err(e) = fs::write(output_path, serialized) {
        eprintln!("Error writing {}: {}", output_path, e);
        std::process::exit(1);
    }
    println!("Compiled {} -> {}", input_path, output_path);
}

fn disassemble_jsc(input_path: &str) {
    let data = match fs::read(input_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading {}: {}", input_path, e);
            std::process::exit(1);
        }
    };

    let bytecode = match pipa::compiler::opcode::Bytecode::deserialize(&data) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Deserialization error: {}", e);
            std::process::exit(1);
        }
    };

    println!("{}", bytecode.disassemble());
}

fn execute_jsc(script_path: &str) {
    let data = match fs::read(script_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading {}: {}", script_path, e);
            std::process::exit(1);
        }
    };

    let bytecode = match pipa::compiler::opcode::Bytecode::deserialize(&data) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Deserialization error: {}", e);
            std::process::exit(1);
        }
    };

    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    setup_context(&mut ctx);

    let script_dir = std::path::Path::new(script_path)
        .canonicalize()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let script_dir_str = script_dir.to_string_lossy().to_string();
    let global = ctx.global();
    let global_ptr = global.get_ptr();
    let global_obj = unsafe { &mut *(global_ptr as *mut JSObject) };
    global_obj.set(
        ctx.intern("__scriptDir"),
        JSValue::new_string(ctx.intern(&script_dir_str)),
    );

    let mut vm = pipa::runtime::vm::VM::new();
    match vm.execute(&mut ctx, &bytecode) {
        Ok(pipa::runtime::vm::ExecutionOutcome::Complete(result)) => {
            if !result.is_undefined() {
                println!("{:?}", js_value_to_string(&result, &ctx));
            }
        }
        Ok(pipa::runtime::vm::ExecutionOutcome::Yield(result)) => {
            if !result.is_undefined() {
                println!("{:?}", js_value_to_string(&result, &ctx));
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn execute_js(script_path: &str, opt_level: OptLevel) {
    let mut runtime = JSRuntime::new();
    runtime.set_argv(std::env::args().collect());
    let mut ctx = runtime.new_context();
    setup_context(&mut ctx);

    let script_dir = std::path::Path::new(script_path)
        .canonicalize()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let script_dir_str = script_dir.to_string_lossy().to_string();
    let global = ctx.global();
    let global_ptr = global.get_ptr();
    let global_obj = unsafe { &mut *(global_ptr as *mut JSObject) };
    global_obj.set(
        ctx.intern("__scriptDir"),
        JSValue::new_string(ctx.intern(&script_dir_str)),
    );

    let code = match fs::read_to_string(script_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {}: {}", script_path, e);
            std::process::exit(1);
        }
    };

    match eval_with_opt_level(&mut ctx, &code, opt_level) {
        Ok(result) => {
            let _ = pipa::run_event_loop(&mut ctx);
            if !result.is_undefined() {
                println!("{:?}", js_value_to_string(&result, &ctx));
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn js_value_to_string(value: &JSValue, ctx: &JSContext) -> String {
    if value.is_undefined() {
        "undefined".to_string()
    } else if value.is_null() {
        "null".to_string()
    } else if value.is_bool() {
        value.get_bool().to_string()
    } else if value.is_int() {
        value.get_int().to_string()
    } else if value.is_float() {
        value.get_float().to_string()
    } else if value.is_string() {
        ctx.get_atom_str(value.get_atom()).to_string()
    } else {
        "[object]".to_string()
    }
}

fn print_fn(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        println!();
        return JSValue::undefined();
    }

    let output: Vec<String> = args.iter().map(|v| js_value_to_string(v, _ctx)).collect();
    let line = output.join(" ");

    let trimmed = line.trim_start();
    if trimmed.starts_with("DEBUG")
        || trimmed.starts_with("[DEBUG]")
        || trimmed.starts_with("checkNumber after=")
        || trimmed.starts_with("WARNING: Invalid checkNumber")
    {
        return JSValue::undefined();
    }

    println!("{}", line);
    JSValue::undefined()
}

fn load_fn(ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
    if args.is_empty() {
        return JSValue::undefined();
    }

    let filename = if args[0].is_string() {
        ctx.get_atom_str(args[0].get_atom()).to_string()
    } else {
        return JSValue::undefined();
    };

    use std::path::PathBuf;

    let base_dir = {
        let global = ctx.global();
        let global_ptr = global.get_ptr();
        let global_obj = unsafe { &*(global_ptr as *const JSObject) };
        match global_obj.get(ctx.intern("__scriptDir")) {
            Some(v) if v.is_string() => PathBuf::from(ctx.get_atom_str(v.get_atom()).to_string()),
            _ => std::env::current_dir().unwrap_or_default(),
        }
    };

    let full_path = if PathBuf::from(&filename).is_absolute() {
        PathBuf::from(&filename)
    } else {
        base_dir.join(&filename)
    };

    match fs::read_to_string(&full_path) {
        Ok(code) => match eval(ctx, &code) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Error executing {:?}: {}", full_path, e);
                JSValue::undefined()
            }
        },
        Err(e) => {
            eprintln!("Error loading {:?}: {}", full_path, e);
            JSValue::undefined()
        }
    }
}

#[cfg(feature = "repl")]
fn count_bracket_delta(line: &str) -> i32 {
    let chars: Vec<char> = line.chars().collect();
    let mut delta = 0i32;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '{' | '(' | '[' => delta += 1,
            '}' | ')' | ']' => delta -= 1,
            '\'' | '"' | '`' => {
                let quote = chars[i];
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\\' {
                        i += 1;
                    } else if chars[i] == quote {
                        break;
                    }
                    i += 1;
                }
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '/' => {
                return delta;
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '*' => {
                i += 2;
                while i + 1 < chars.len() {
                    if chars[i] == '*' && chars[i + 1] == '/' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    delta
}

#[cfg(feature = "repl")]
fn start_repl() {
    let history_path = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".pipa_history"))
        .ok();

    let mut rl = rustyline::DefaultEditor::new().expect("Failed to create REPL editor");
    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    setup_context(&mut ctx);

    println!("Pipa REPL  (type .exit or press Ctrl+D to quit)");

    let mut buffer = String::new();
    let mut depth = 0i32;

    loop {
        let prompt = if buffer.is_empty() { "> " } else { "... " };
        match rl.readline(prompt) {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if buffer.is_empty() && trimmed == ".exit" {
                    break;
                }

                buffer.push_str(&line);
                depth += count_bracket_delta(&line);

                if depth > 0 {
                    buffer.push('\n');
                    continue;
                }

                let _ = rl.add_history_entry(buffer.trim().to_string());
                if let Some(ref path) = history_path {
                    let _ = rl.save_history(path);
                }

                {
                    let result = eval_repl(&mut ctx, &buffer);
                    js_value_to_string_safe(&result, &mut ctx);
                }

                buffer.clear();
                depth = 0;
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                buffer.clear();
                depth = 0;
            }
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    println!();
    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }
}

#[cfg(feature = "repl")]
fn eval_repl(ctx: &mut JSContext, code: &str) -> Result<JSValue, String> {
    use pipa::builtins::promise::run_microtasks_with_vm;
    use pipa::runtime::vm::{ExecutionOutcome, VM};

    let rb = pipa::compile_to_bytecode_with_opt_level(ctx, code, ctx.get_compiler_opt_level())?;

    let saved_register_vm_ptr = ctx.get_register_vm_ptr();
    let mut vm = VM::new();
    let vm_ptr = &mut vm as *mut _ as usize;
    ctx.set_register_vm_ptr(Some(vm_ptr));

    let result = match vm.execute_preserving_registers(ctx, &rb) {
        Ok(ExecutionOutcome::Complete(v)) => Ok(v),
        Ok(ExecutionOutcome::Yield(v)) => Ok(v),
        Err(e) => Err(e),
    };

    run_microtasks_with_vm(ctx, &mut vm);
    let _ = ctx.run_event_loop_with_timeout(30000);

    ctx.set_register_vm_ptr(saved_register_vm_ptr);
    result
}

#[cfg(feature = "repl")]
fn js_value_to_string_safe(result: &Result<JSValue, String>, ctx: &mut JSContext) {
    match result {
        Ok(val) => {
            let global = ctx.global();
            let global_ptr = global.get_ptr();
            let global_obj = unsafe { &mut *(global_ptr as *mut JSObject) };
            global_obj.set(ctx.intern("_"), *val);

            let s = js_value_to_string(val, ctx);
            if s != "undefined" {
                println!("{}", s);
            }
        }
        Err(e) => {
            eprintln!("Uncaught {}", e);
        }
    }
}
