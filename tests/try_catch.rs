use pipa::{JSContext, JSRuntime, eval};

fn assert_js_ok(ctx: &mut JSContext, code: &str, msg: &str) {
    let r = eval(ctx, code);
    assert!(r.is_ok(), "{}: {:?}", msg, r);
}

#[test]
fn test_try_catch_basic() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        r#"
            var result = 0;
            try {
                result = 1;
            } catch (e) {
                result = 2;
            }
            if (result !== 1) throw new Error('try/catch basic mismatch');
        "#,
        "try catch basic failed",
    );
}

#[test]
fn test_try_catch_throw() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        r#"
            var result = 0;
            try {
                throw 42;
            } catch (e) {
                result = e;
            }
            if (result !== 42) throw new Error('try/catch throw mismatch');
        "#,
        "try catch throw failed",
    );
}

#[test]
fn test_try_catch_throw_string() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        r#"
        var caught = "none";
        try {
            throw "error message";
        } catch (e) {
            caught = e;
        }
        if (caught !== 'error message') throw new Error('try/catch string mismatch');
    "#,
        "try catch throw string failed",
    );
}

#[test]
fn test_try_catch_nested() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        r#"
            var result = 0;
            try {
                try {
                    throw 1;
                } catch (e1) {
                    throw e1 + 1;
                }
            } catch (e2) {
                result = e2;
            }
            if (result !== 2) throw new Error('nested try/catch mismatch');
        "#,
        "try catch nested failed",
    );
}

#[test]
fn test_try_finally() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        r#"
            var result = 0;
            try {
                result = 1;
            } finally {
                result = result + 10;
            }
            if (result !== 11) throw new Error('try/finally mismatch');
        "#,
        "try finally failed",
    );
}

#[test]
fn test_try_catch_in_function() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        r#"
            function mayThrow(shouldThrow) {
                try {
                    if (shouldThrow) {
                        throw 100;
                    }
                    return 0;
                } catch (e) {
                    return e;
                }
            }
            if (mayThrow(true) !== 100) throw new Error('mayThrow(true) mismatch');
        "#,
        "try catch in function failed",
    );
}
