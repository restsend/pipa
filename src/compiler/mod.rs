pub mod ast;
pub mod codegen;
pub mod ic;
pub mod lexer;
pub mod location;
pub mod mir;
pub mod opcode;
pub mod parser;
pub mod peephole;

pub use codegen::{CodeGenerator, OptLevel};
pub use ic::{InlineCache, InlineCacheTable};
pub use lexer::{Lexer, Token, TokenType};
pub use location::{FrameInfo, LineNumberTable, SourceLocation, TracebackFormatter};
pub use opcode::{Bytecode, Opcode};

use crate::builtins::promise::run_microtasks_with_vm;
use crate::runtime::JSContext;
use crate::runtime::event_loop::EventLoopResult;
use crate::value::JSValue;

pub fn eval_code(ctx: &mut JSContext, source: &str) -> Result<JSValue, String> {
    eval_code_via_ast_with_opt_level(ctx, source, ctx.get_compiler_opt_level())
}

pub fn eval_code_via_ast(ctx: &mut JSContext, source: &str) -> Result<JSValue, String> {
    eval_code_via_ast_with_opt_level(ctx, source, ctx.get_compiler_opt_level())
}

pub fn compile_script_to_bytecode_with_opt_level(
    ctx: &mut JSContext,
    source: &str,
    opt_level: OptLevel,
    is_strict: bool,
    is_eval: bool,
) -> Result<Bytecode, String> {
    let ast = parser::Parser::new(source).parse()?;

    let mut func_codegen = CodeGenerator::with_opt_level(opt_level);
    func_codegen.is_strict = is_strict;
    func_codegen.is_eval = is_eval;
    let block = ast::BlockStatement {
        body: ast.body,
        lines: ast.lines,
    };
    let (mut rb, _) = func_codegen.compile_script(&block, ctx)?;

    peephole::optimize_with_level(&mut rb, opt_level);

    Ok(rb)
}

fn eval_bytecode_with_caller(
    ctx: &mut JSContext,
    rb: &Bytecode,
    caller_vm: Option<*mut crate::runtime::vm::VM>,
) -> Result<JSValue, String> {
    use crate::runtime::vm::{ExecutionOutcome, VM};

    let saved_register_vm_ptr = ctx.get_register_vm_ptr();

    let mut vm = VM::new();
    let vm_ptr = &mut vm as *mut _ as usize;
    ctx.set_register_vm_ptr(Some(vm_ptr));

    let caller_ptr = caller_vm.map(|p| p as usize);

    let caller_this = if let Some(ptr) = caller_vm {
        let caller = unsafe { &*ptr };
        let caller_frame = &caller.frames[caller.frame_index];
        let mut this_val = caller_frame.this_value;
        if !caller_frame.is_strict_frame && (this_val.is_undefined() || this_val.is_null()) {
            this_val = ctx.global();
        }
        this_val
    } else {
        ctx.global()
    };

    let result = match vm.execute_eval(ctx, rb, caller_this, caller_ptr) {
        Ok(ExecutionOutcome::Complete(v)) => Ok(v),
        Ok(ExecutionOutcome::Yield(v)) => Ok(v),
        Err(e) => Err(e),
    };

    run_microtasks_with_vm(ctx, &mut vm);

    ctx.set_register_vm_ptr(saved_register_vm_ptr);

    result
}

pub fn eval_code_via_ast_with_opt_level_as_eval_with_caller(
    ctx: &mut JSContext,
    source: &str,
    opt_level: OptLevel,
    is_strict: bool,
    caller_vm: *mut crate::runtime::vm::VM,
) -> Result<JSValue, String> {
    let rb = compile_script_to_bytecode_with_opt_level(ctx, source, opt_level, is_strict, true)?;
    eval_bytecode_with_caller(ctx, &rb, Some(caller_vm))
}

pub fn eval_code_via_ast_with_opt_level(
    ctx: &mut JSContext,
    source: &str,
    opt_level: OptLevel,
) -> Result<JSValue, String> {
    let rb = compile_script_to_bytecode_with_opt_level(ctx, source, opt_level, false, false)?;

    let saved_register_vm_ptr = ctx.get_register_vm_ptr();

    let mut vm = crate::runtime::vm::VM::new();
    let vm_ptr = &mut vm as *mut _ as usize;
    ctx.set_register_vm_ptr(Some(vm_ptr));

    let result = match vm.execute_preserving_registers(ctx, &rb) {
        Ok(crate::runtime::vm::ExecutionOutcome::Complete(v)) => Ok(v),
        Ok(crate::runtime::vm::ExecutionOutcome::Yield(v)) => Ok(v),
        Err(e) => Err(e),
    };

    run_microtasks_with_vm(ctx, &mut vm);

    ctx.set_register_vm_ptr(saved_register_vm_ptr);

    result
}

pub fn run_event_loop(ctx: &mut JSContext) -> Result<EventLoopResult, String> {
    ctx.run_event_loop()
}

pub fn run_event_loop_with_timeout(
    ctx: &mut JSContext,
    timeout_ms: u64,
) -> Result<EventLoopResult, String> {
    ctx.run_event_loop_with_timeout(timeout_ms)
}
