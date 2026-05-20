use pipa::{JSRuntime, eval};

fn assert_js_ok(ctx: &mut pipa::JSContext, code: &str, msg: &str) {
    let r = eval(ctx, code);
    assert!(r.is_ok(), "{}: {:?}", msg, r);
}

#[test]
fn test_user_function_return() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function getValue() { return 42; } if (getValue() !== 42) throw new Error('return mismatch');",
        "user function return failed",
    );
}

#[test]
fn test_user_function_with_params() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function add(a, b) { return a + b; } if (add(3, 4) !== 7) throw new Error('param mismatch');",
        "user function with params failed",
    );
}

#[test]
fn test_user_function_local_vars() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function compute() { var x = 10; var y = 20; return x + y; } if (compute() !== 30) throw new Error('local var mismatch');",
        "user function local vars failed",
    );
}

#[test]
fn test_function_with_if() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function abs(n) { if (n < 0) { return 0 - n; } return n; } if (abs(-5) !== 5) throw new Error('if mismatch');",
        "function with if failed",
    );
}

#[test]
fn test_recursive_function() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function factorial(n) { if (n <= 1) { return 1; } return n * factorial(n - 1); } if (factorial(5) !== 120) throw new Error('recursive mismatch');",
        "recursive function failed",
    );
}

#[test]
fn test_nested_function_calls() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function double(x) { return x * 2; } function quadruple(x) { return double(double(x)); } if (quadruple(3) !== 12) throw new Error('nested call mismatch');",
        "nested function calls failed",
    );
}

#[test]
fn test_multiple_functions() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function square(x) { return x * x; } function sumSquares(a, b) { return square(a) + square(b); } if (sumSquares(3, 4) !== 25) throw new Error('multiple functions mismatch');",
        "multiple functions failed",
    );
}

#[test]
fn test_arrow_expression_body() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var add = (a, b) => a + b; if (add(3, 4) !== 7) throw new Error('arrow add mismatch');",
        "arrow add failed",
    );
    assert_js_ok(
        &mut ctx,
        "var double = x => x * 2; if (double(5) !== 10) throw new Error('arrow double mismatch');",
        "arrow double failed",
    );
    assert_js_ok(
        &mut ctx,
        "var getN = () => 42; if (getN() !== 42) throw new Error('arrow zero-arg mismatch');",
        "arrow zero-arg failed",
    );
    
    assert_js_ok(
        &mut ctx,
        "var mul = (a) => (b) => a * b; if (mul(3)(4) !== 12) throw new Error('nested arrow mismatch');",
        "nested arrow failed",
    );
    
    assert_js_ok(
        &mut ctx,
        "var mkObj = (x) => ({ x: x }); if (mkObj(5).x !== 5) throw new Error('arrow object return mismatch');",
        "arrow object return failed",
    );
}

#[test]
fn test_arrow_block_body() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var abs = x => { if (x < 0) { return 0 - x; } return x; }; if (abs(-7) !== 7) throw new Error('arrow block mismatch');",
        "arrow block body failed",
    );
}

#[test]
fn test_arrow_with_higher_order() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var a = [1, 2, 3].map(x => x * x).reduce((acc, x) => acc + x, 0); if (a !== 14) throw new Error('higher-order reduce mismatch');",
        "arrow higher-order map/reduce failed",
    );
    assert_js_ok(
        &mut ctx,
        "var b = [1, 2, 3, 4, 5].filter(x => x % 2 === 0).length; if (b !== 2) throw new Error('higher-order filter mismatch');",
        "arrow higher-order filter failed",
    );
}

#[test]
fn test_arrow_closure() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var f = (x) => { var g = (y) => x + y; return g(10); }; if (f(5) !== 15) throw new Error('arrow closure mismatch');",
        "arrow closure failed",
    );
}

#[test]
fn test_arguments_object() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { return arguments.length; } if (f(1, 2, 3) !== 3) throw new Error('arguments.length mismatch');",
        "arguments object failed",
    );
}

#[test]
fn test_arguments_indexed_access() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { return arguments[0]; } if (f(42) !== 42) throw new Error('arguments[0] mismatch'); function g() { return arguments[1]; } if (g(10, 20) !== 20) throw new Error('arguments[1] mismatch');",
        "arguments indexed access failed",
    );
}

#[test]
fn test_arguments_callee() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function f() { return arguments.callee; } if (typeof f() !== 'function') throw new Error('arguments.callee mismatch');",
        "arguments.callee failed",
    );
}

#[test]
fn test_use_strict_directive() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "'use strict'; var x = 42; if (x !== 42) throw new Error('strict directive mismatch');",
        "use strict directive failed",
    );
}

#[test]
fn test_strict_mode_simple() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "'use strict'; function f() { return 42; } if (f() !== 42) throw new Error('strict mode function mismatch');",
        "strict mode simple failed",
    );
}

#[test]
fn test_async_function_basic() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(&mut ctx, "(async function() { return 42; })()");
    assert!(r.is_ok(), "async function failed: {:?}", r);
    assert!(
        r.unwrap().is_object(),
        "async function should return a Promise"
    );
}

#[test]
fn test_generator_function_basic() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function* gen() { yield 1; yield 2; return 3; } if (typeof gen !== 'function') throw new Error('generator not a function'); var g = gen(); if (typeof g.next !== 'function') throw new Error('generator object missing next');",
        "generator function failed",
    );
}

#[test]
fn test_async_generator_function() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(&mut ctx, "(async function*() { yield 1; })()");
    assert!(r.is_ok(), "async generator failed: {:?}", r);
    assert!(
        r.unwrap().is_object(),
        "async generator should return an object"
    );
}

#[test]
fn test_closure_upvalue_regular() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function makeCounter() { var count = 0; return function() { return ++count; }; } var c = makeCounter(); if (c() !== 1) throw new Error('closure 1 failed'); if (c() !== 2) throw new Error('closure 2 failed'); if (c() !== 3) throw new Error('closure 3 failed');",
        "closure upvalue regular failed",
    );
}

#[test]
fn test_closure_upvalue_async() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(
        &mut ctx,
        "function makeAsync() { var x = 10; return async function() { return x + 5; }; } makeAsync()()",
    );
    assert!(r.is_ok(), "async closure failed: {:?}", r);
}

#[test]
fn test_generator_function_object() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "function makeGen() { return function*() { yield 1; }; } var g = makeGen()(); if (typeof g.next !== 'function') throw new Error('generator object missing next');",
        "generator function object failed",
    );
}
