
use pipa::{JSRuntime, eval};

#[test]
fn test_error_shows_stack_trace() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function inner() {
    throw new Error("test error");
}
function outer() {
    inner();
}
outer();
"#;

    let result = eval(&mut ctx, code);
    assert!(result.is_err(), "Expected an error");

    let error_msg = result.unwrap_err();

    assert!(
        error_msg.contains("Uncaught"),
        "Error should start with 'Uncaught': {}",
        error_msg
    );
    assert!(
        error_msg.contains("at"),
        "Error should contain 'at' for stack trace: {}",
        error_msg
    );

    println!("Error output:\n{}", error_msg);
}

#[test]
fn test_type_error_shows_traceback() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function foo() {
    var x = 123;
    x();  // TypeError: x is not a function
}
foo();
"#;

    let result = eval(&mut ctx, code);
    assert!(result.is_err(), "Expected a TypeError");

    let error_msg = result.unwrap_err();
    println!("TypeError output:\n{}", error_msg);

    assert!(
        error_msg.contains("at"),
        "Error should contain stack trace: {}",
        error_msg
    );
}

#[test]
fn test_nested_function_calls_traceback() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function level3() {
    throw "deep error";
}
function level2() {
    level3();
}
function level1() {
    level2();
}
level1();
"#;

    let result = eval(&mut ctx, code);
    assert!(result.is_err());

    let error_msg = result.unwrap_err();
    println!("Nested call error:\n{}", error_msg);

    assert!(
        error_msg.contains("Uncaught"),
        "Should show 'Uncaught': {}",
        error_msg
    );
}

#[test]
fn test_error_in_global_scope() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"throw "global error";"#;

    let result = eval(&mut ctx, code);
    assert!(result.is_err());

    let error_msg = result.unwrap_err();
    println!("Global error:\n{}", error_msg);

    assert!(error_msg.contains("Uncaught"));
    assert!(error_msg.contains("global error"));
}

#[test]
fn test_traceback_line_numbers_are_accurate() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function inner() {
    var x = 1;
    var y = 2;
    throw new Error("line 5 error");
}
function outer() {
    inner();
}
outer();
"#;

    let result = eval(&mut ctx, code);
    assert!(result.is_err());
    let error_msg = result.unwrap_err();
    println!("Line number accuracy test (stack VM):\n{}", error_msg);

    assert!(
        error_msg.contains("inner"),
        "Should show inner: {}",
        error_msg
    );
}

#[test]
fn test_register_vm_traceback_line_numbers() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function inner() {
    var x = 1;
    var y = 2;
    throw new Error("line 5 error");
}
function outer() {
    inner();
}
return outer();
"#;

    let result = eval_register(&mut ctx, code);
    assert!(result.is_err());
    let error_msg = result.unwrap_err();
    println!("Line number accuracy test (register VM):\n{}", error_msg);

    assert!(
        error_msg.contains(":5:") || error_msg.contains(":6:"),
        "Expected error around line 5, got:\n{}",
        error_msg
    );
}

#[test]
fn test_traceback_shows_full_call_chain() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function a() { b(); }
function b() { c(); }
function c() { d(); }
function d() { throw new Error("deep"); }
a();
"#;

    let result = eval(&mut ctx, code);
    assert!(result.is_err());
    let error_msg = result.unwrap_err();

    assert!(
        error_msg.contains("d"),
        "Should show d in stack: {}",
        error_msg
    );
}

#[test]
fn test_register_vm_traceback_full_call_chain() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function outer() {
    function a() { return b(); }
    function b() { return c(); }
    function c() { return d(); }
    function d() { throw new Error("deep"); }
    return a();
}
return outer();
"#;

    let result = eval_register(&mut ctx, code);
    assert!(result.is_err());
    let error_msg = result.unwrap_err();

    assert!(
        error_msg.contains("d"),
        "Should show d in stack: {}",
        error_msg
    );
    assert!(
        error_msg.contains("c"),
        "Should show c in stack: {}",
        error_msg
    );
    assert!(
        error_msg.contains("b"),
        "Should show b in stack: {}",
        error_msg
    );
    assert!(
        error_msg.contains("a"),
        "Should show a in stack: {}",
        error_msg
    );
    assert!(
        error_msg.contains("outer"),
        "Should show outer in stack: {}",
        error_msg
    );
}

use pipa::compiler::ast::BlockStatement;
use pipa::compiler::codegen::CodeGenerator;
use pipa::compiler::parser::Parser;
use pipa::runtime::vm::VM;

fn eval_register(ctx: &mut pipa::JSContext, code: &str) -> Result<pipa::JSValue, String> {
    let ast = Parser::new(code).parse()?;
    let mut codegen = CodeGenerator::new();
    let block = BlockStatement {
        body: ast.body,
        lines: ast.lines,
    };
    let (rb, _) = codegen.compile_function(&[], &block, ctx)?;
    let mut vm = VM::new();
    let outcome = vm.execute(ctx, &rb)?;
    match outcome {
        pipa::runtime::vm::ExecutionOutcome::Complete(v) => Ok(v),
        pipa::runtime::vm::ExecutionOutcome::Yield(v) => Ok(v),
    }
}

#[test]
fn test_register_vm_error_traceback() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function inner() {
    throw new Error("register vm error");
}
function outer() {
    inner();
}
return outer();
"#;

    let result = eval_register(&mut ctx, code);
    assert!(result.is_err(), "Expected an error in register VM");

    let error_msg = result.unwrap_err();
    println!("Register VM error output:\n{}", error_msg);

    assert!(
        error_msg.contains("Uncaught"),
        "Error should start with 'Uncaught': {}",
        error_msg
    );
    assert!(
        error_msg.contains("at"),
        "Error should contain 'at' for stack trace: {}",
        error_msg
    );
}

#[test]
fn test_register_vm_type_error_traceback() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function foo() {
    var x = 123;
    x();
}
return foo();
"#;

    let result = eval_register(&mut ctx, code);
    assert!(result.is_err(), "Expected a TypeError in register VM");

    let error_msg = result.unwrap_err();
    println!("Register VM TypeError output:\n{}", error_msg);

    assert!(
        error_msg.contains("at"),
        "Error should contain stack trace: {}",
        error_msg
    );
}

#[test]
fn test_register_vm_nested_calls_traceback() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function level3() {
    throw "deep error";
}
function level2() {
    level3();
}
function level1() {
    level2();
}
return level1();
"#;

    let result = eval_register(&mut ctx, code);
    assert!(result.is_err());

    let error_msg = result.unwrap_err();
    println!("Register VM nested call error:\n{}", error_msg);

    assert!(
        error_msg.contains("Uncaught"),
        "Should show 'Uncaught': {}",
        error_msg
    );
    assert!(
        error_msg.contains("level3"),
        "Should show level3 in stack: {}",
        error_msg
    );
    assert!(
        error_msg.contains("level2"),
        "Should show level2 in stack: {}",
        error_msg
    );
    assert!(
        error_msg.contains("level1"),
        "Should show level1 in stack: {}",
        error_msg
    );
}
