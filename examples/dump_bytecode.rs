use pipa::JSRuntime;
use pipa::compiler::ast::BlockStatement;
use pipa::compiler::codegen::CodeGenerator;
use pipa::compiler::parser::Parser;

fn main() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function isHeldOrSuspended() {
  return (this.state & 4) != 0 || (this.state == 2);
}
"#;
    let ast = Parser::new(code).parse().unwrap();
    let mut codegen = CodeGenerator::new();
    let block = BlockStatement {
        body: ast.body,
        lines: ast.lines,
    };
    let (rb, _) = codegen.compile_function(&[], &block, &mut ctx).unwrap();
    println!("{}", rb.disassemble());
}
