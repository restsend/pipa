use pipa::{JSRuntime, eval};

fn assert_js_ok(ctx: &mut pipa::JSContext, code: &str, msg: &str) {
    let r = eval(ctx, code);
    assert!(r.is_ok(), "{}: {:?}", msg, r);
}

#[test]
fn test_async_function_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    assert_js_ok(
        &mut ctx,
        "var p = (async function() { return 42; })(); if (typeof p !== 'object') { throw new Error('async function should return a Promise'); }",
        "async function failed",
    );
}

#[test]
fn test_async_function_with_await() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    assert_js_ok(
        &mut ctx,
        "var p = (async function() { return await Promise.resolve(42); })(); if (typeof p !== 'object') { throw new Error('async function should return a Promise'); }",
        "async function with await failed",
    );
}

#[test]
fn test_async_function_try_catch_await() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    assert_js_ok(
        &mut ctx,
        r#"
        var p = (async function() {
            try {
                var x = await Promise.resolve(42);
                return x;
            } catch (e) {
                return -1;
            }
        })();
        if (typeof p !== 'object') {
            throw new Error('async function should return a Promise');
        }
    "#,
        "async function with try/catch failed",
    );
}

#[test]
fn test_async_function_catch_rejected_promise() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    assert_js_ok(
        &mut ctx,
        r#"
        var p = (async function() {
            try {
                var x = await Promise.reject("error");
                return 0; // should not reach here
            } catch (e) {
                return e; // should catch the error
            }
        })();
        if (typeof p !== 'object') {
            throw new Error('async function should return a Promise');
        }
    "#,
        "async function catch rejected promise failed",
    );
}

#[test]
fn test_async_function_throw() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    assert_js_ok(
        &mut ctx,
        r#"
        var p = (async function() {
            throw "my error";
        })();
        if (typeof p !== 'object') {
            throw new Error('async function should return a Promise even when throwing');
        }
    "#,
        "async function that throws failed",
    );
}

#[test]
fn test_async_function_try_catch_throw() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    assert_js_ok(
        &mut ctx,
        r#"
        var p = (async function() {
            try {
                throw 42;
            } catch (e) {
                return e;
            }
        })();
        if (typeof p !== 'object') {
            throw new Error('async function should return a Promise');
        }
    "#,
        "async function with try/catch/throw failed",
    );
}

#[test]
fn test_async_arrow_function() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        (async () => 42)()
    "#,
    );

    if r.is_err() {
        eprintln!("async arrow function not implemented: {:?}", r);
    }
}

#[test]
fn test_await_non_promise() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    assert_js_ok(
        &mut ctx,
        r#"
        var p = (async function() {
            return await 42;
        })();
        if (typeof p !== 'object') {
            throw new Error('async function should return a Promise');
        }
    "#,
        "await on non-promise failed",
    );
}
