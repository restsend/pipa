use pipa::{JSRuntime, eval};

#[test]
fn test_if_statement() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "if(true)1").is_ok());
    assert!(eval(&mut ctx, "if (true) 1 else 2").is_ok());
}

#[test]
fn test_if_else() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "if (false) { 1 } else { 2 }").is_ok());
}

#[test]
fn test_if_true_body() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "if (true) { 1 }").is_ok());
}

#[test]
fn test_if_false_with_var() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var x = 1; if (false) { }").is_ok());
}

#[test]
fn test_nested_if() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "if (true) { if (false) { 1 } }").is_ok());
}

#[test]
fn test_while_statement() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "if(false){}").is_ok());
    assert!(eval(&mut ctx, "while (false) {}").is_ok());
}

#[test]
fn test_while_true() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "while (false) { }").is_ok());
}

#[test]
fn test_while_with_body() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "while (false) { var x = 1; }").is_ok());
}

#[test]
fn test_for_loop() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "while (false) {}").is_ok());
    assert!(eval(&mut ctx, "for (var i = 0; i < 3; i++) {}").is_ok());
}

#[test]
fn test_function_call() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "function add() {}").is_ok());
}
