#![cfg(feature = "full_runtime_tests")]

use pipa::{JSRuntime, eval};

fn eval_bool(ctx: &mut pipa::JSContext, code: &str) -> bool {
    eval(ctx, code).unwrap().get_bool()
}

fn eval_int(ctx: &mut pipa::JSContext, code: &str) -> i64 {
    eval(ctx, code).unwrap().get_int()
}

#[test]
fn test_map_new_empty() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert!(eval(&mut ctx, "new Map()").is_ok());
}

#[test]
fn test_map_set_and_get() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let result = eval(
        &mut ctx,
        r#"
        const m = new Map();
        m.set("key", 42);
        m.get("key")
        "#,
    )
    .unwrap();
    assert_eq!(result.get_int(), 42);
}

#[test]
fn test_map_has() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert!(eval_bool(
        &mut ctx,
        r#"
        const m = new Map();
        m.set("a", 1);
        m.has("a")
        "#
    ));
    assert!(!eval_bool(
        &mut ctx,
        r#"
        const m = new Map();
        m.set("a", 1);
        m.has("b")
        "#
    ));
}

#[test]
fn test_map_delete() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert!(eval_bool(
        &mut ctx,
        r#"
        const m = new Map();
        m.set("x", 99);
        m.delete("x")
        "#
    ));
    assert!(!eval_bool(
        &mut ctx,
        r#"
        const m = new Map();
        m.set("x", 99);
        m.delete("x");
        m.has("x")
        "#
    ));
}

#[test]
fn test_map_size() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const m = new Map();
        m.set("a", 1);
        m.set("b", 2);
        m.set("c", 3);
        m.size
        "#
        ),
        3
    );
}

#[test]
fn test_map_size_after_delete() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const m = new Map();
        m.set("a", 1);
        m.set("b", 2);
        m.delete("a");
        m.size
        "#
        ),
        1
    );
}

#[test]
fn test_map_clear() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const m = new Map();
        m.set("a", 1);
        m.set("b", 2);
        m.clear();
        m.size
        "#
        ),
        0
    );
}

#[test]
fn test_map_set_returns_this() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const m = new Map();
        m.set("a", 1).set("b", 2);
        m.size
        "#
        ),
        2
    );
}

#[test]
fn test_map_get_missing_key_returns_undefined() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let result = eval(&mut ctx, r#"const m = new Map(); m.get("nope")"#).unwrap();
    assert!(result.is_undefined());
}

#[test]
fn test_map_update_existing_key() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const m = new Map();
        m.set("k", 1);
        m.set("k", 99);
        m.get("k")
        "#
        ),
        99
    );

    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const m = new Map();
        m.set("k", 1);
        m.set("k", 99);
        m.size
        "#
        ),
        1
    );
}

#[test]
fn test_map_constructor_with_iterable() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const m = new Map([["a", 1], ["b", 2], ["c", 3]]);
        m.size
        "#
        ),
        3
    );
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const m = new Map([["a", 1], ["b", 2]]);
        m.get("b")
        "#
        ),
        2
    );
}

#[test]
fn test_map_keys() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const m = new Map();
        m.set("a", 1);
        m.set("b", 2);
        m.keys().length
        "#
        ),
        2
    );
}

#[test]
fn test_map_values() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const m = new Map();
        m.set("a", 10);
        m.set("b", 20);
        m.values()[0]
        "#
        ),
        10
    );
}

#[test]
fn test_map_entries() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const m = new Map([["x", 5]]);
        m.entries().length
        "#
        ),
        1
    );
}

#[test]
fn test_map_foreach() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const m = new Map([["a", 1], ["b", 2], ["c", 3]]);
        let sum = 0;
        m.forEach((val, key) => { sum = sum + val; });
        sum
        "#
        ),
        6
    );
}

#[test]
fn test_set_new_empty() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert!(eval(&mut ctx, "new Set()").is_ok());
}

#[test]
fn test_set_add_and_has() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert!(eval_bool(
        &mut ctx,
        r#"
        const s = new Set();
        s.add(1);
        s.has(1)
        "#
    ));
    assert!(!eval_bool(
        &mut ctx,
        r#"
        const s = new Set();
        s.add(1);
        s.has(2)
        "#
    ));
}

#[test]
fn test_set_no_duplicates() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const s = new Set();
        s.add(1);
        s.add(1);
        s.add(1);
        s.size
        "#
        ),
        1
    );
}

#[test]
fn test_set_size() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const s = new Set();
        s.add(1);
        s.add(2);
        s.add(3);
        s.size
        "#
        ),
        3
    );
}

#[test]
fn test_set_delete() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert!(eval_bool(
        &mut ctx,
        r#"
        const s = new Set();
        s.add(42);
        s.delete(42)
        "#
    ));
    assert!(!eval_bool(
        &mut ctx,
        r#"
        const s = new Set();
        s.add(42);
        s.delete(42);
        s.has(42)
        "#
    ));
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const s = new Set();
        s.add(1);
        s.add(2);
        s.delete(1);
        s.size
        "#
        ),
        1
    );
}

#[test]
fn test_set_clear() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const s = new Set();
        s.add(1);
        s.add(2);
        s.clear();
        s.size
        "#
        ),
        0
    );
}

#[test]
fn test_set_add_returns_this() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const s = new Set();
        s.add(1).add(2).add(3);
        s.size
        "#
        ),
        3
    );
}

#[test]
fn test_set_constructor_with_array() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const s = new Set([1, 2, 3]);
        s.size
        "#
        ),
        3
    );
    assert!(eval_bool(
        &mut ctx,
        r#"
        const s = new Set([1, 2, 3]);
        s.has(2)
        "#
    ));

    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const s = new Set([1, 1, 2, 2, 3]);
        s.size
        "#
        ),
        3
    );
}

#[test]
fn test_set_values() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const s = new Set([10, 20, 30]);
        s.values().length
        "#
        ),
        3
    );
}

#[test]
fn test_set_keys_same_as_values() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const s = new Set([10, 20]);
        s.keys().length
        "#
        ),
        2
    );
}

#[test]
fn test_set_entries() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const s = new Set([1, 2]);
        s.entries().length
        "#
        ),
        2
    );
}

#[test]
fn test_set_foreach() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert_eq!(
        eval_int(
            &mut ctx,
            r#"
        const s = new Set([1, 2, 3, 4]);
        let sum = 0;
        s.forEach((val) => { sum = sum + val; });
        sum
        "#
        ),
        10
    );
}

#[test]
fn test_map_with_integer_keys() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert!(eval_bool(
        &mut ctx,
        r#"
        const m = new Map();
        m.set(1, "one");
        m.has(1)
        "#
    ));
}

#[test]
fn test_set_with_string_values() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert!(eval_bool(
        &mut ctx,
        r#"
        const s = new Set(["hello", "world"]);
        s.has("hello")
        "#
    ));
}

#[test]
fn test_map_delete_nonexistent_returns_false() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert!(!eval_bool(
        &mut ctx,
        r#"
        const m = new Map();
        m.delete("nonexistent")
        "#
    ));
}

#[test]
fn test_set_delete_nonexistent_returns_false() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    assert!(!eval_bool(
        &mut ctx,
        r#"
        const s = new Set();
        s.delete(999)
        "#
    ));
}
