use pipa::JSRuntime;
use pipa::compiler::ast::BlockStatement;
use pipa::compiler::codegen::CodeGenerator;
use pipa::compiler::parser::Parser;

fn main() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function fib(n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}
return fib(20);
"#;

    let ast = Parser::new(code).parse().unwrap();
    let mut codegen = CodeGenerator::new();
    let block = BlockStatement {
        body: ast.body,
        lines: ast.lines,
    };
    let (rb, _) = codegen.compile_function(&[], &block, &mut ctx).unwrap();

    println!("Constants count: {}", rb.constants.len());
    for (i, c) in rb.constants.iter().enumerate() {
        println!("Constant {}: {:?}", i, c);
    }
}
