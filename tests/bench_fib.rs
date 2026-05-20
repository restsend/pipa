
use pipa::compiler::ast::BlockStatement;
use pipa::compiler::codegen::CodeGenerator;
use pipa::compiler::parser::Parser;
use pipa::runtime::vm::VM;
use pipa::{JSRuntime, eval};

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
fn test_fib_correctness() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function fib(n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}
fib(20)
"#;

    let result = eval(&mut ctx, code).expect("fib should succeed");
    assert_eq!(result.get_int(), 6765, "fib(20) should equal 6765");
}

#[test]
fn test_fib_register_vm_correctness() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function fib(n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}
return fib(20);
"#;

    let result = eval_register(&mut ctx, code).expect("Register VM fib should succeed");
    assert_eq!(
        result.get_int(),
        6765,
        "fib(20) should equal 6765 on register VM"
    );
}

#[test]
fn test_arithmetic_loop() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
var sum = 0;
for (var i = 0; i < 100; i++) {
    sum += i;
}
sum;
"#;

    let result = eval(&mut ctx, code).expect("arithmetic loop should succeed");
    assert_eq!(result.get_int(), 4950, "sum 0..99 should be 4950");
}

#[test]
fn test_arithmetic_loop_register_vm() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function run() {
    var sum = 0;
    for (var i = 0; i < 100; i++) {
        sum += i;
    }
    return sum;
}
return run();
"#;

    let result = eval_register(&mut ctx, code).expect("Register VM arithmetic loop should succeed");
    assert_eq!(
        result.get_int(),
        4950,
        "sum 0..99 should be 4950 on register VM"
    );
}

#[test]
fn test_nested_call_register_vm() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function add(a, b) { return a + b; }
function mul(a, b) { return a * b; }
function calc() {
    return add(mul(2, 3), mul(4, 5));
}
return calc();
"#;

    let result = eval_register(&mut ctx, code).expect("Register VM nested call should succeed");
    assert_eq!(
        result.get_int(),
        26,
        "2*3 + 4*5 should be 26 on register VM"
    );
}

#[test]
fn test_closure_register_vm() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function makeCounter() {
    var count = 0;
    return function() {
        count += 1;
        return count;
    };
}
var counter = makeCounter();
counter();
counter();
return counter();
"#;

    let result = eval_register(&mut ctx, code).expect("Register VM closure should succeed");
    assert_eq!(result.get_int(), 3, "Counter should be 3 on register VM");
}

#[test]
fn test_object_and_array_register_vm() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function test() {
    var arr = [1, 2, 3];
    var obj = { a: 10, b: 20 };
    return arr[0] + arr[1] + arr[2] + obj.a + obj.b;
}
return test();
"#;

    let result = eval_register(&mut ctx, code).expect("Register VM object/array should succeed");
    assert_eq!(
        result.get_int(),
        36,
        "1+2+3+10+20 should be 36 on register VM"
    );
}

#[test]
fn test_try_catch_register_vm() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
function test() {
    try {
        throw 42;
    } catch (e) {
        return e;
    }
}
return test();
"#;

    let result = eval_register(&mut ctx, code).expect("Register VM try/catch should succeed");
    assert_eq!(
        result.get_int(),
        42,
        "Caught value should be 42 on register VM"
    );
}
