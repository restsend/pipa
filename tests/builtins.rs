use pipa::{JSRuntime, eval};

fn assert_js_ok(ctx: &mut pipa::JSContext, code: &str, msg: &str) {
    let r = eval(ctx, code);
    assert!(r.is_ok(), "{}: {:?}", msg, r);
}

#[test]
fn test_console_log() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "console.log(1)").is_ok());
}

#[test]
fn test_math_abs() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "Math.abs(-5)").is_ok());
}

#[test]
fn test_math_floor() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "Math.floor(3.7)").is_ok());
}

#[test]
fn test_math_return_values() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "if (Math.abs(-5) !== 5) throw new Error('abs mismatch'); if (Math.max(1, 5, 3) !== 5) throw new Error('max mismatch'); if (Math.min(1, 5, 3) !== 1) throw new Error('min mismatch');",
        "math return values failed",
    );
}

#[test]
fn test_json_parse() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "JSON.parse('{\"a\":1}')").is_ok());
}

#[test]
fn test_math_trigonometric_functions() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "if (Math.sin(0) !== 0) throw new Error('sin(0)');",
        "math sin failed",
    );
    assert_js_ok(
        &mut ctx,
        "if (Math.cos(0) !== 1) throw new Error('cos(0)');",
        "math cos failed",
    );
    assert_js_ok(
        &mut ctx,
        "if (Math.tan(0) !== 0) throw new Error('tan(0)');",
        "math tan failed",
    );
    assert_js_ok(
        &mut ctx,
        "if (Math.asin(0) !== 0) throw new Error('asin(0)');",
        "math asin failed",
    );
    assert_js_ok(
        &mut ctx,
        "if (Math.acos(1) !== 0) throw new Error('acos(1)');",
        "math acos failed",
    );
    assert_js_ok(
        &mut ctx,
        "if (Math.atan(0) !== 0) throw new Error('atan(0)');",
        "math atan failed",
    );
    assert_js_ok(
        &mut ctx,
        "if (Math.atan2(0, 1) !== 0) throw new Error('atan2(0,1)');",
        "math atan2 failed",
    );
}

#[test]
fn test_math_trigonometric_approximations() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var pi = Math.PI; if (Math.abs(Math.sin(pi / 2) - 1) > 1e-10) throw new Error('sin(pi/2)');",
        "math sin(pi/2) failed",
    );
    assert_js_ok(
        &mut ctx,
        "var pi = Math.PI; if (Math.abs(Math.cos(pi) - (-1)) > 1e-10) throw new Error('cos(pi)');",
        "math cos(pi) failed",
    );
    assert_js_ok(
        &mut ctx,
        "var pi = Math.PI; if (Math.abs(Math.tan(pi / 4) - 1) > 1e-10) throw new Error('tan(pi/4)');",
        "math tan(pi/4) failed",
    );
    assert_js_ok(
        &mut ctx,
        "if (Math.abs(Math.asin(1) - Math.PI / 2) > 1e-10) throw new Error('asin(1)');",
        "math asin(1) failed",
    );
    assert_js_ok(
        &mut ctx,
        "if (Math.abs(Math.acos(0) - Math.PI / 2) > 1e-10) throw new Error('acos(0)');",
        "math acos(0) failed",
    );
    assert_js_ok(
        &mut ctx,
        "if (Math.abs(Math.atan(1) - Math.PI / 4) > 1e-10) throw new Error('atan(1)');",
        "math atan(1) failed",
    );
    assert_js_ok(
        &mut ctx,
        "if (Math.abs(Math.atan2(1, 1) - Math.PI / 4) > 1e-10) throw new Error('atan2(1,1)');",
        "math atan2(1,1) failed",
    );
}

#[test]
fn test_global_parseint() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "parseInt('42')").is_ok());
}

#[test]
fn test_global_isnan() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "isNaN(NaN)").is_ok());
}

#[test]
fn test_global_functions() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "isNaN(5)").is_ok());
    assert!(eval(&mut ctx, "isFinite(5)").is_ok());
    assert!(eval(&mut ctx, "parseInt('42')").is_ok());
}

#[test]
fn test_global_this() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "if (typeof globalThis !== 'object') throw new Error('globalThis type mismatch');",
        "globalThis failed",
    );

    let r = eval(&mut ctx, "globalThis.isNaN(5)");
    assert!(r.is_ok(), "globalThis.isNaN failed: {:?}", r);

    let r = eval(&mut ctx, "globalThis.parseInt('42')");
    assert!(r.is_ok(), "globalThis.parseInt failed: {:?}", r);
}
