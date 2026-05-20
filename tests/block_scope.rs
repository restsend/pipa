use pipa::{JSRuntime, eval};

fn assert_js_ok(ctx: &mut pipa::JSContext, code: &str, msg: &str) {
    let r = eval(ctx, code);
    assert!(r.is_ok(), "{}: {:?}", msg, r);
}

fn assert_js_err(ctx: &mut pipa::JSContext, code: &str, msg: &str) {
    let r = eval(ctx, code);
    assert!(r.is_err(), "{}: expected error but got {:?}", msg, r);
}

#[test]
fn test_let_block_scope() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { let x = 0; { let x = 1; if (x !== 1) throw new Error('inner x'); } if (x !== 0) throw new Error('outer x'); return x; } if (f() !== 0) throw new Error('f()');",
        "let block scope shadowing",
    );
}

#[test]
fn test_var_function_scope() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { { var x = 1; } return x; } if (f() !== 1) throw new Error('var not function-scoped');",
        "var function scope",
    );
}

#[test]
fn test_tdz_let() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_err(&mut ctx, "function f() { x; let x = 1; } f();", "TDZ let");
}

#[test]
fn test_tdz_const() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_err(
        &mut ctx,
        "function f() { x; const x = 1; } f();",
        "TDZ const",
    );
}

#[test]
fn test_tdz_in_block() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_err(
        &mut ctx,
        "function f() { { x; let x = 1; } } f();",
        "TDZ in block",
    );
}

#[test]
fn test_const_assignment_error() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_err(
        &mut ctx,
        "function f() { const x = 1; x = 2; } f();",
        "const reassignment",
    );
}

#[test]
fn test_const_update_error() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_err(
        &mut ctx,
        "function f() { const x = 1; x++; } f();",
        "const update",
    );
}

#[test]
fn test_const_no_initializer_error() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_err(
        &mut ctx,
        "function f() { const x; } f();",
        "const no initializer",
    );
}

#[test]
fn test_let_in_for_loop() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { let sum = 0; for (let i = 0; i < 5; i++) { sum += i; } return sum; } if (f() !== 10) throw new Error('for-let sum');",
        "for-let scoping",
    );
}

#[test]
fn test_let_not_leaking_from_for() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { for (let i = 0; i < 3; i++) {} return typeof i; } if (f() !== 'undefined') throw new Error('for-let leaked');",
        "for-let not leaking",
    );
}

#[test]
fn test_var_in_for_loop() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { for (var i = 0; i < 3; i++) {} return i; } if (f() !== 3) throw new Error('for-var leak');",
        "for-var leaking",
    );
}

#[test]
fn test_nested_block_shadowing() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { let x = 'a'; { let x = 'b'; { let x = 'c'; if (x !== 'c') throw new Error('innermost'); } if (x !== 'b') throw new Error('middle'); } if (x !== 'a') throw new Error('outer'); return x; } if (f() !== 'a') throw new Error('return');",
        "nested block shadowing",
    );
}

#[test]
fn test_let_for_in() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { let keys = ''; for (let k in {a: 1, b: 2}) { keys += k; } return keys; } var r = f(); if (r !== 'ab') throw new Error('for-in let: ' + r);",
        "for-in let",
    );
}

#[test]
fn test_let_for_of() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { let sum = 0; for (let v of [1, 2, 3]) { sum += v; } return sum; } if (f() !== 6) throw new Error('for-of let');",
        "for-of let",
    );
}

#[test]
fn test_let_in_if_block() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { if (true) { let x = 1; } return typeof x; } if (f() !== 'undefined') throw new Error('let leaked from if');",
        "let in if block",
    );
}

#[test]
fn test_let_after_use_in_same_block() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { let x = 1; return x; } if (f() !== 1) throw new Error('let after use');",
        "let after use",
    );
}

#[test]
fn test_multiple_let_in_block() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { { let a = 1, b = 2; if (a + b !== 3) throw new Error('multi let'); } return 0; } f();",
        "multiple let declarations",
    );
}
