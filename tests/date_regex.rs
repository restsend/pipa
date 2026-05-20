use pipa::{JSRuntime, eval};

#[test]
fn test_date_now() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(
        eval(
            &mut ctx,
            "var ts = Date.now(); if (!(ts > 0)) throw new Error('Date.now should be positive');"
        )
        .is_ok()
    );
}

#[test]
fn test_date_constructor() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "new Date()").is_ok());
}

#[test]
fn test_regexp_literal() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "/abc/").is_ok());
}

#[test]
fn test_regexp_test() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "/abc/.test('abcdef')").is_ok());
}

#[test]
fn test_regexp_comprehensive() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var re = /abc/; re.test('abcdef')").is_ok());
}
