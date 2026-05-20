use pipa::{JSRuntime, eval};

fn assert_js_ok(ctx: &mut pipa::JSContext, code: &str, msg: &str) {
    let r = eval(ctx, code);
    assert!(r.is_ok(), "{}: {:?}", msg, r);
}

#[test]
fn test_variable_declaration() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var x = 1").is_ok());
    assert!(eval(&mut ctx, "var x = 1; x").is_ok());
    assert!(eval(&mut ctx, "var x = 5; var y = 3; x + y").is_ok());
}

#[test]
fn test_variable_return_values() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var x = 5; if (x !== 5) throw new Error('x mismatch'); var y = 3; if (x + y !== 8) throw new Error('x+y mismatch'); x = 20; if (x !== 20) throw new Error('assignment mismatch');",
        "variable return values failed",
    );
}

#[test]
fn test_variable_assignment() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var x = 1; x = 2; x").is_ok());
    assert!(eval(&mut ctx, "var x = 1; x += 5; x").is_ok());
}

#[test]
fn test_compound_assignment() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var x = 10; x += 3; if (x !== 13) throw new Error('+='); x = 10; x -= 3; if (x !== 7) throw new Error('-='); x = 10; x *= 3; if (x !== 30) throw new Error('*='); x = 10; x /= 2; if (x !== 5) throw new Error('/='); x = 10; x %= 3; if (x !== 1) throw new Error('%='); x = 5; x &= 3; if (x !== 1) throw new Error('&='); x = 5; x |= 3; if (x !== 7) throw new Error('|='); x = 5; x ^= 3; if (x !== 6) throw new Error('^='); x = 1; x <<= 3; if (x !== 8) throw new Error('<<='); x = 16; x >>= 2; if (x !== 4) throw new Error('>>=');",
        "compound assignment failed",
    );
}

#[test]
fn test_local_variable_assignment() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function foo() { var x = 1; x = 100; return x; } if (foo() !== 100) throw new Error('local assign'); function bar() { var x = 5; x += 3; return x; } if (bar() !== 8) throw new Error('local +=' ); function baz() { var x = 10; x -= 3; return x; } if (baz() !== 7) throw new Error('local -='); function qux() { var x = 3; x *= 4; return x; } if (qux() !== 12) throw new Error('local *='); function quux() { var x = 10; x /= 2; return x; } if (quux() !== 5) throw new Error('local /=');",
        "local variable assignment failed",
    );
}
