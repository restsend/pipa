use pipa::{JSRuntime, eval};

fn eval_ok(ctx: &mut pipa::JSContext, code: &str) {
    let r = eval(ctx, code);
    assert!(r.is_ok(), "eval failed: {:?}", r);
}

#[test]
fn test_eval_read_callers_var() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    eval_ok(
        &mut ctx,
        r#"
var x = 23;
var result = eval('x');
if (result !== 23) throw new Error("result should be 23, got: " + result);
"#,
    );
}

#[test]
fn test_eval_read_callers_var_then_declare() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    eval_ok(
        &mut ctx,
        r#"
var x = 23;
var initial;
eval('initial = x; var x = 45;');
if (initial !== 23) throw new Error("initial should be 23, got: " + initial);
if (x !== 45) throw new Error("x should be 45, got: " + x);
"#,
    );
}

#[test]
fn test_eval_var_no_init_keeps_callers_value() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    eval_ok(
        &mut ctx,
        r#"
var x = 23;
eval('var x;');
if (x !== 23) throw new Error("x should be 23, got: " + x);
"#,
    );
}

#[test]
fn test_eval_var_no_init_creates_undefined() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    eval_ok(
        &mut ctx,
        r#"
eval('var x;');
if (x !== undefined) throw new Error("x should be undefined, got: " + x);
"#,
    );
}

#[test]
fn test_eval_var_no_init_after_assignment() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    eval_ok(
        &mut ctx,
        r#"
eval('x = 4; var x;');
if (x !== 4) throw new Error("x should be 4, got: " + x);
"#,
    );
}

#[test]
fn test_eval_var_init_global_new() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    eval_ok(
        &mut ctx,
        r#"
var initial = null;
var postAssignment;
eval('initial = x; x = 4; postAssignment = x; var x;');
if (initial !== undefined) throw new Error("initial should be undefined, got: " + initial);
if (postAssignment !== 4) throw new Error("postAssignment should be 4, got: " + postAssignment);
if (x !== 4) throw new Error("x should be 4, got: " + x);
"#,
    );
}
