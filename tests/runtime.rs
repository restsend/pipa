use pipa::{JSRuntime, JSValue, eval, set_interrupt};

#[test]
fn test_runtime_creation() {
    let mut runtime = JSRuntime::new();
    let ctx = runtime.new_context();
    assert!(ctx.runtime().atom_count() > 0);
}

#[test]
fn test_jsvalue_undefined() {
    let val = JSValue::undefined();
    assert!(val.is_undefined());
    assert!(!val.is_null());
}

#[test]
fn test_jsvalue_null() {
    let val = JSValue::null();
    assert!(val.is_null());
    assert!(!val.is_undefined());
}

#[test]
fn test_jsvalue_bool() {
    let t = JSValue::bool(true);
    let f = JSValue::bool(false);
    assert!(t.is_bool());
    assert!(t.get_bool());
    assert!(!f.get_bool());
}

#[test]
fn test_jsvalue_int() {
    let val = JSValue::new_int(42);
    assert!(val.is_int());
    assert_eq!(val.get_int(), 42);
}

#[test]
fn test_jsvalue_float() {
    let val = JSValue::new_float(1.0);
    assert!(val.is_int(), "1.0 should be stored as int");
    assert_eq!(val.get_int(), 1);
}

#[test]
fn test_jsvalue_truthy() {
    assert!(!JSValue::undefined().is_truthy());
    assert!(!JSValue::null().is_truthy());
    assert!(JSValue::bool(true).is_truthy());
    assert!(!JSValue::bool(false).is_truthy());
    assert!(JSValue::new_int(1).is_truthy());
    assert!(!JSValue::new_int(0).is_truthy());
}

#[test]
fn test_jsvalue_strict_eq() {
    let a = JSValue::new_int(42);
    let b = JSValue::new_int(42);
    let c = JSValue::new_int(10);
    assert!(a.strict_eq(&b));
    assert!(!a.strict_eq(&c));
}

#[test]
fn test_atom_intern() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let atom1 = ctx.intern("hello");
    let atom2 = ctx.intern("hello");
    let atom3 = ctx.intern("world");

    assert_eq!(atom1, atom2);
    assert_ne!(atom1, atom3);
}

#[test]
fn test_atom_string() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let atom = ctx.intern("test");
    let s = ctx.get_atom_str(atom);
    assert_eq!(s, "test");
}

#[test]
fn test_interrupt() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    set_interrupt(&runtime, true);
    let result = eval(&mut ctx, "1 + 1");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "interrupted");
}

#[test]
fn test_no_interrupt() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    set_interrupt(&runtime, false);
    let result = eval(&mut ctx, "1 + 1");
    assert!(result.is_ok());
}
