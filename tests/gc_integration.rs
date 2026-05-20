#![cfg(feature = "full_runtime_tests")]

use pipa::{JSRuntime, eval};

#[test]
fn test_gc_tracks_objects() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        var obj1 = { a: 1 };
        var obj2 = { b: 2 };
        var obj3 = { c: 3 };
        42;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 42);

    let object_count = runtime.gc_heap().object_count();
    assert!(
        object_count >= 3,
        "GC should have tracked at least 3 objects"
    );
}

#[test]
fn test_gc_heap_info() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let initial_count = runtime.gc_heap().object_count();

    let code = r#"
        var obj = { x: 1, y: 2 };
        1;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 1);

    let after_count = runtime.gc_heap().object_count();
    assert!(
        after_count > initial_count,
        "Object count should increase after creating objects"
    );
}

#[test]
fn test_gc_memory_size() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let initial_size = runtime.gc_heap().total_size();

    let code = r#"
        var arr = [1, 2, 3, 4, 5];
        var obj = { a: 1, b: 2 };
        1;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 1);

    let after_size = runtime.gc_heap().total_size();
    assert!(after_size > initial_size, "Memory size should increase");
}

#[test]
fn test_gc_nested_objects() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        var inner = { value: 42 };
        var outer = { nested: inner };
        outer.nested.value;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 42);
}

#[test]
fn test_gc_array_tracking() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let initial_count = runtime.gc_heap().object_count();

    let code = r#"
        var arr = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        arr.length;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 10);

    let after_count = runtime.gc_heap().object_count();
    assert!(after_count > initial_count);
}

#[test]
fn test_gc_function_tracking() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        function add(a, b) { return a + b; }
        add(2, 3);
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 5);
}

#[test]
fn test_gc_closure_tracking() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        function makeCounter() {
            var count = 0;
            return function() { return ++count; };
        }
        var counter = makeCounter();
        counter();
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 1);
}

#[test]
fn test_gc_object_with_methods() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        var obj = {
            x: 10,
            getY: function() { return 20; },
            sum: function() { return this.x + this.getY(); }
        };
        obj.sum();
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 30);
}

#[test]
fn test_gc_deep_nesting() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        var a = { v: 1 };
        var b = { a: a };
        var c = { b: b };
        var d = { c: c };
        d.c.b.a.v;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 1);
}

#[test]
fn test_gc_circular_reference() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        var a = {};
        var b = { ref: a };
        a.ref = b;
        'done';
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert!(result.is_string());
}

#[test]
fn test_gc_prototype_chain() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        var parent = { x: 1 };
        var child = Object.create(parent);
        child.y = 2;
        child.x + child.y;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 3);
}

#[test]
fn test_gc_class_instances() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        class Point {
            constructor(x, y) {
                this.x = x;
                this.y = y;
            }
            sum() { return this.x + this.y; }
        }
        var p = new Point(3, 4);
        p.sum();
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 7);
}

#[test]
fn test_gc_promise_tracking() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        var p = Promise.resolve(42);
        p;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert!(result.is_object());
}

#[test]
fn test_gc_map_tracking() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        var m = new Map();
        m.set('a', 1);
        m.set('b', 2);
        m.size;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 2);
}

#[test]
fn test_gc_set_tracking() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        var s = new Set();
        s.add(1);
        s.add(2);
        s.add(3);
        s.size;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 3);
}

#[test]
fn test_gc_threshold() {
    let mut runtime = JSRuntime::new();
    runtime.gc_heap_mut().set_threshold(100);
    let mut ctx = runtime.new_context();

    let code = r#"
        var arr = [];
        for (var i = 0; i < 50; i++) {
            arr.push({ value: i });
        }
        arr.length;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 50);
}

#[test]
fn test_gc_many_allocations() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let initial_count = runtime.gc_heap().object_count();

    let code = r#"
        var arr = [];
        for (var i = 0; i < 100; i++) {
            arr.push({ x: i, y: i * 2 });
        }
        arr[50].x + arr[50].y;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 150);

    let after_count = runtime.gc_heap().object_count();
    assert!(after_count > initial_count + 100);
}
