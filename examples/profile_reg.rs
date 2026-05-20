use pipa::JSRuntime;
use pipa::compiler::ast::BlockStatement;
use pipa::compiler::codegen::CodeGenerator;
use pipa::compiler::parser::Parser;
use pipa::runtime::vm::VM;

fn main() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    let code = r#"
function fib(n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}
fib(20)
"#;
    let ast = Parser::new(code).parse().unwrap();
    let mut codegen = CodeGenerator::new();
    let block = BlockStatement {
        body: ast.body,
        lines: ast.lines,
    };
    let (rb, _) = codegen.compile_script(&block, &mut ctx).unwrap();
    let mut vm = VM::new();

    for _ in 0..100 {
        let _ = vm.execute(&mut ctx, &rb);
    }

    for _ in 0..100000 {
        let _ = vm.execute(&mut ctx, &rb);
    }
}
