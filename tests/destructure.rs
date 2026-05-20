#![cfg(feature = "full_runtime_tests")]

use pipa::{JSRuntime, eval};

fn get_int(ctx: &mut pipa::JSContext, code: &str) -> i64 {
    let result = eval(ctx, code).unwrap();
    if result.is_int() {
        result.get_int()
    } else if result.is_float() {
        result.get_float() as i64
    } else {
        panic!("Expected number but got {:?}", result);
    }
}

#[test]
fn test_array_destructure_basic() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(&mut ctx, "var [a, b] = [1, 2]; a + b;");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 3);
}

#[test]
fn test_array_destructure_skip() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_eq!(get_int(&mut ctx, "var [a, , c] = [1, 2, 3]; c;"), 3);
}

#[test]
fn test_array_destructure_rest() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        "var [first, ...rest] = [1, 2, 3, 4]; rest.length;",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 3);

    assert_eq!(get_int(&mut ctx, "var [h, ...t] = [10, 20, 30]; t[0];"), 20);
}

#[test]
fn test_object_destructure_basic() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        "var obj = {x: 10, y: 20}; var {x, y} = obj; x + y;",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 30);
}

#[test]
fn test_object_destructure_rename() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        "var obj = {name: \"Alice\", age: 30}; var {name: n, age: a} = obj; a;",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 30);
}

#[test]
fn test_object_destructure_rename_string() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(&mut ctx, "var p = {name: \"Bob\"}; var {name: n} = p; n;");
    assert!(r.is_ok());
    let val = r.unwrap();
    assert!(val.is_string());
    assert_eq!(ctx.get_atom_str(val.get_atom()), "Bob");
}

#[test]
fn test_rest_params() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        "function sum(...args) { var total = 0; var i = 0; while (i < args.length) { total = total + args[i]; i = i + 1; } return total; } sum(1, 2, 3, 4, 5);",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 15);

    let result = eval(
        &mut ctx,
        "function first(a, b, ...rest) { return rest.length; } first(1, 2, 3, 4, 5);",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 3);
}

#[test]
fn test_rest_params_arrow() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        "var sum = (...args) => args.reduce((a, b) => a + b, 0); sum(1, 2, 3);",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 6);
}

#[test]
fn test_spread_array_literal() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(&mut ctx, "var a = [1, 2]; var b = [...a, 3, 4]; b.length;");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 4);

    assert_eq!(
        get_int(&mut ctx, "var a = [1, 2]; var b = [...a, 3, 4]; b[0];"),
        1
    );
    assert_eq!(
        get_int(&mut ctx, "var a = [1, 2]; var b = [...a, 3, 4]; b[2];"),
        3
    );

    let result = eval(
        &mut ctx,
        "var x = [1]; var y = [2]; var z = [...x, ...y]; z.length;",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 2);
}

#[test]
fn test_spread_function_call() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        "function add(a, b, c) { return a + b + c; } var args = [1, 2, 3]; add(...args);",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 6);

    let result = eval(&mut ctx, "var nums = [3, 1, 4, 1, 5]; Math.max(...nums);");
    assert!(result.is_ok());
    let val = result.unwrap();
    let max_val = if val.is_int() {
        val.get_int()
    } else {
        val.get_float() as i64
    };
    assert_eq!(max_val, 5);
}

#[test]
fn test_spread_object_literal() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        "var a = {x: 1, y: 2}; var b = {...a, z: 3}; b.x + b.y + b.z;",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 6);

    let result = eval(
        &mut ctx,
        "var a = {x: 1}; var b = {x: 10, y: 2}; var c = {...a, ...b}; c.x;",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 10);
}

#[test]
fn test_destructure_in_for_of() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(&mut ctx, "var [x, y] = [10, 20]; x + y;");
    assert!(r.is_ok(), "{:?}", r);
    assert_eq!(r.unwrap().get_int(), 30);

    let r2 = eval(
        &mut ctx,
        "var s2 = 0; var pp = [[1,2],[3,4]]; var j2 = 0; var [c, d] = pp[j2]; s2 = s2 + c + d; j2 = 1; var [c, d] = pp[j2]; s2 = s2 + c + d; s2;",
    );
    assert!(r2.is_ok(), "{:?}", r2);
    assert_eq!(r2.unwrap().get_int(), 10);

    let result = eval(
        &mut ctx,
        "var s3 = 0; var qq = [[1,2],[3,4],[5,6]]; for (var j3 = 0; j3 < qq.length; j3++) { var [e, f] = qq[j3]; s3 = s3 + e + f; } s3;",
    );
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap().get_int(), 21);
}

#[test]
fn test_spread_in_array_copy() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(&mut ctx, "var a = [1,2,3]; var b = [...a]; b.length;");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 3);
}
