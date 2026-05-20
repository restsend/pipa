#![cfg(feature = "full_runtime_tests")]

use pipa::{JSRuntime, eval};

#[test]
fn test_weakref_exists() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "typeof WeakRef");
    assert!(r.is_ok(), "WeakRef typeof failed: {:?}", r);
    let val = r.unwrap();
    assert!(val.is_string(), "WeakRef typeof should be string");
}

#[test]
fn test_weakref_constructor() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "var obj = {x: 1}; var wr = new WeakRef(obj); typeof wr",
    );
    assert!(r.is_ok(), "WeakRef constructor failed: {:?}", r);
    let val = r.unwrap();
    assert!(val.is_string(), "WeakRef typeof should be string");
}

#[test]
fn test_weakref_deref() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "var obj = {x: 42}; var wr = new WeakRef(obj); wr.deref()",
    );
    assert!(r.is_ok(), "WeakRef.deref failed: {:?}", r);
    let val = r.unwrap();
    assert!(val.is_object(), "WeakRef.deref should return object");
}

#[test]
fn test_weakref_deref_value() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "var obj = {x: 42}; var wr = new WeakRef(obj); wr.deref().x",
    );
    assert!(r.is_ok(), "WeakRef.deref().x failed: {:?}", r);
    let val = r.unwrap();
    assert_eq!(val.get_int(), 42, "WeakRef.deref().x should be 42");
}

#[test]
fn test_finalization_registry_exists() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "typeof FinalizationRegistry");
    assert!(r.is_ok(), "FinalizationRegistry typeof failed: {:?}", r);
    let val = r.unwrap();
    assert!(
        val.is_string(),
        "FinalizationRegistry typeof should be string"
    );
}

#[test]
fn test_finalization_registry_constructor() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "var fr = new FinalizationRegistry(v => {}); typeof fr",
    );
    assert!(
        r.is_ok(),
        "FinalizationRegistry constructor failed: {:?}",
        r
    );
    let val = r.unwrap();
    assert!(
        val.is_string(),
        "FinalizationRegistry typeof should be string"
    );
}

#[test]
fn test_finalization_registry_register() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var fr = new FinalizationRegistry(v => {});
        var obj = {x: 1};
        fr.register(obj, "held value");
        "ok"
    "#,
    );
    assert!(r.is_ok(), "FinalizationRegistry.register failed: {:?}", r);
}

#[test]
fn test_finalization_registry_unregister() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var fr = new FinalizationRegistry(v => {});
        var obj = {x: 1};
        var token = {};
        fr.register(obj, "held value", token);
        fr.unregister(token)
    "#,
    );
    assert!(r.is_ok(), "FinalizationRegistry.unregister failed: {:?}", r);
    let val = r.unwrap();
    assert!(
        val.get_bool(),
        "FinalizationRegistry.unregister should return true"
    );
}

#[test]
fn test_finalization_registry_unregister_not_found() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var fr = new FinalizationRegistry(v => {});
        var token = {};
        fr.unregister(token)
    "#,
    );
    assert!(r.is_ok(), "FinalizationRegistry.unregister failed: {:?}", r);
    let val = r.unwrap();
    assert!(
        !val.get_bool(),
        "FinalizationRegistry.unregister should return false for not found"
    );
}
