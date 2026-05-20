use pipa::{JSRuntime, eval};

fn assert_js_ok(ctx: &mut pipa::JSContext, code: &str, msg: &str) {
    let r = eval(ctx, code);
    assert!(r.is_ok(), "{}: {:?}", msg, r);
}

#[test]
fn test_arithmetic_operations() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "1 + 2").is_ok());
    assert!(eval(&mut ctx, "10 - 3").is_ok());
    assert!(eval(&mut ctx, "4 * 5").is_ok());
    assert!(eval(&mut ctx, "10 / 2").is_ok());
}

#[test]
fn test_comparison_operations() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "1 < 2").is_ok());
    assert!(eval(&mut ctx, "3 > 2").is_ok());
    assert!(eval(&mut ctx, "2 == 2").is_ok());
    assert!(eval(&mut ctx, "1 != 2").is_ok());
}

#[test]
fn test_logical_operations() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "true && true").is_ok());
    assert!(eval(&mut ctx, "false || true").is_ok());
    assert!(eval(&mut ctx, "!false").is_ok());
}

#[test]
fn test_bitwise_operations() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "5 & 3").is_ok());
    assert!(eval(&mut ctx, "5 | 3").is_ok());
    assert!(eval(&mut ctx, "5 ^ 3").is_ok());
    assert!(eval(&mut ctx, "~5").is_ok());
}

#[test]
fn test_arithmetic_return_values() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "if ((1 + 2) !== 3) throw new Error('add'); if ((5 - 3) !== 2) throw new Error('sub'); if ((2 * 3) !== 6) throw new Error('mul'); if ((10 / 2) !== 5) throw new Error('div'); if ((10 % 3) !== 1) throw new Error('mod');",
        "arithmetic return values failed",
    );
}

#[test]
fn test_comparison_return_values() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "if ((1 < 2) !== true) throw new Error('<'); if ((1 > 2) !== false) throw new Error('>'); if ((1 == 1) !== true) throw new Error('=='); if ((1 === 1) !== true) throw new Error('==='); if ((1 != 2) !== true) throw new Error('!='); if ((1 !== 2) !== true) throw new Error('!==');",
        "comparison return values failed",
    );
}

#[test]
fn test_bitwise_return_values() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "if ((5 & 3) !== 1) throw new Error('&&'); if ((5 | 3) !== 7) throw new Error('||'); if ((5 ^ 3) !== 6) throw new Error('^^'); if ((~5) !== -6) throw new Error('~~');",
        "bitwise return values failed",
    );
}

#[test]
fn test_logical_assignment_operators() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var x = 1; x ||= 5; if (x !== 1) throw new Error('||= 1'); x = 0; x ||= 5; if (x !== 5) throw new Error('||= 0'); x = false; x ||= 5; if (x !== 5) throw new Error('||= false'); x = ''; x ||= 5; if (x !== 5) throw new Error('||= empty'); x = 1; x &&= 5; if (x !== 5) throw new Error('&&= 1'); x = 0; x &&= 5; if (x !== 0) throw new Error('&&= 0'); x = false; x &&= 5; if (x !== false) throw new Error('&&= false'); x = 1; x ??= 5; if (x !== 1) throw new Error('??= 1'); x = null; x ??= 5; if (x !== 5) throw new Error('??= null'); x = undefined; x ??= 5; if (x !== 5) throw new Error('??= undef'); x = 0; x ??= 5; if (x !== 0) throw new Error('??= zero');",
        "logical assignment operators failed",
    );
}
