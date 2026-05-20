pub mod builtins;
pub mod compiler;
pub mod host;
#[cfg(feature = "fetch")]
pub mod http;
pub mod object;
pub mod regexp;
pub mod runtime;
pub mod util;
pub mod value;

pub use compiler::OptLevel;
pub use object::JSObject;
pub use runtime::event_loop::EventLoopResult;
pub use runtime::{JSContext, JSRuntime};
pub use value::JSValue;

pub fn eval(ctx: &mut JSContext, code: &str) -> Result<JSValue, String> {
    compiler::eval_code(ctx, code)
}

pub fn eval_with_opt_level(
    ctx: &mut JSContext,
    code: &str,
    opt_level: OptLevel,
) -> Result<JSValue, String> {
    let saved_opt_level = ctx.get_compiler_opt_level();
    ctx.set_compiler_opt_level(opt_level);
    let result = compiler::eval_code_via_ast_with_opt_level(ctx, code, opt_level);
    ctx.set_compiler_opt_level(saved_opt_level);
    result
}

pub fn eval_via_ast(ctx: &mut JSContext, code: &str) -> Result<JSValue, String> {
    compiler::eval_code_via_ast(ctx, code)
}

pub fn eval_via_ast_with_opt_level(
    ctx: &mut JSContext,
    code: &str,
    opt_level: OptLevel,
) -> Result<JSValue, String> {
    compiler::eval_code_via_ast_with_opt_level(ctx, code, opt_level)
}

pub fn parse_to_ast(code: &str) -> Result<compiler::ast::Program, String> {
    compiler::parser::Parser::new(code).parse()
}

pub fn compile_to_register_bytecode(
    ctx: &mut JSContext,
    code: &str,
) -> Result<(Vec<u8>, Vec<JSValue>), String> {
    compile_to_register_bytecode_with_opt_level(ctx, code, OptLevel::default())
}

pub fn compile_to_register_bytecode_with_opt_level(
    ctx: &mut JSContext,
    code: &str,
    opt_level: OptLevel,
) -> Result<(Vec<u8>, Vec<JSValue>), String> {
    use compiler::codegen::CodeGenerator;
    use compiler::parser::Parser;
    use compiler::peephole;
    let program = Parser::new(code).parse()?;
    let mut func_codegen = CodeGenerator::with_opt_level(opt_level);
    let block = compiler::ast::BlockStatement {
        body: program.body,
        lines: program.lines,
    };
    let (mut rb, _) = func_codegen.compile_function(&[], &block, ctx)?;
    peephole::optimize_with_level(&mut rb, opt_level);
    Ok((rb.code, rb.constants))
}

pub fn compile_to_bytecode(ctx: &mut JSContext, code: &str) -> Result<compiler::Bytecode, String> {
    compile_to_bytecode_with_opt_level(ctx, code, OptLevel::default())
}

pub fn compile_to_bytecode_with_opt_level(
    ctx: &mut JSContext,
    code: &str,
    opt_level: OptLevel,
) -> Result<compiler::Bytecode, String> {
    compiler::compile_script_to_bytecode_with_opt_level(ctx, code, opt_level, false, false)
}

pub fn set_interrupt(runtime: &JSRuntime, flag: bool) {
    runtime.set_interrupt(flag);
}

pub fn run_event_loop(ctx: &mut JSContext) -> Result<EventLoopResult, String> {
    compiler::run_event_loop(ctx)
}

pub fn run_event_loop_with_timeout(
    ctx: &mut JSContext,
    timeout_ms: u64,
) -> Result<EventLoopResult, String> {
    compiler::run_event_loop_with_timeout(ctx, timeout_ms)
}
