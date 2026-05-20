use pipa::{JSRuntime, eval};

fn eval_int(code: &str) -> i64 {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let result = eval(&mut ctx, code);
    assert!(result.is_ok(), "JS error: {:?}", result);
    result.unwrap().get_int()
}

fn eval_ok(code: &str) {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let result = eval(&mut ctx, code);
    assert!(result.is_ok(), "JS error: {:?}", result);
}

#[test]
fn test_for_of_array_basic() {
    assert_eq!(
        eval_int("var sum = 0; for (var x of [1, 2, 3]) { sum += x; } sum"),
        6
    );
}

#[test]
fn test_for_of_array_empty() {
    assert_eq!(
        eval_int("var sum = 0; for (var x of []) { sum += x; } sum"),
        0
    );
}

#[test]
fn test_for_of_array_single() {
    assert_eq!(
        eval_int("var sum = 0; for (var x of [42]) { sum += x; } sum"),
        42
    );
}

#[test]
fn test_for_of_array_strings() {
    eval_ok(
        r#"
        var result = "";
        for (var s of ["hello", " ", "world"]) { result += s; }
        if (result !== "hello world") throw new Error("Expected 'hello world', got: " + result);
        "#,
    );
}

#[test]
fn test_for_of_array_with_break() {
    assert_eq!(
        eval_int(
            "var sum = 0; for (var x of [1, 2, 3, 4, 5]) { if (x === 3) break; sum += x; } sum"
        ),
        3 
    );
}

#[test]
fn test_for_of_array_with_continue() {
    assert_eq!(
        eval_int(
            "var sum = 0; for (var x of [1, 2, 3, 4, 5]) { if (x === 3) continue; sum += x; } sum"
        ),
        12 
    );
}

#[test]
fn test_for_of_array_let_binding() {
    
    eval_ok(
        r#"
        function test() {
            var results = [];
            for (let x of [10, 20, 30]) {
                results.push(x);
            }
            if (results.length !== 3) throw new Error("length: " + results.length);
            if (results[0] !== 10) throw new Error("results[0]: " + results[0]);
            if (results[1] !== 20) throw new Error("results[1]: " + results[1]);
            if (results[2] !== 30) throw new Error("results[2]: " + results[2]);
        }
        test();
        "#,
    );
}

#[test]
fn test_for_of_string_basic() {
    eval_ok(
        r#"
        var chars = "";
        for (var c of "abc") { chars += c; }
        if (chars !== "abc") throw new Error("Expected 'abc', got: " + chars);
        "#,
    );
}

#[test]
fn test_for_of_string_empty() {
    assert_eq!(
        eval_int(r#"var count = 0; for (var c of "") { count++; } count"#),
        0
    );
}

#[test]
fn test_for_of_string_count() {
    assert_eq!(
        eval_int(r#"var count = 0; for (var c of "hello") { count++; } count"#),
        5
    );
}

#[test]
fn test_for_of_nested() {
    assert_eq!(
        eval_int(
            r#"
        var sum = 0;
        for (var arr of [[1, 2], [3, 4]]) {
            for (var x of arr) {
                sum += x;
            }
        }
        sum
        "#
        ),
        10
    );
}

#[test]
fn test_for_of_existing_var() {
    assert_eq!(
        eval_int("var x = 0; var sum = 0; for (x of [10, 20, 30]) { sum += x; } sum"),
        60
    );
}

#[test]
fn test_for_of_var_after_loop() {
    
    assert_eq!(eval_int("var x = 0; for (x of [1, 2, 3]) {} x"), 3);
}

#[test]
fn test_super_this_binding() {
    eval_ok(
        r#"
        class Animal {
            constructor(name) { this.name = name; }
        }
        class Dog extends Animal {
            constructor(name) {
                super(name);
                this.type = "dog";
            }
        }
        var d = new Dog("Rex");
        if (d.name !== "Rex") throw new Error("name: " + d.name);
        if (d.type !== "dog") throw new Error("type: " + d.type);
        "#,
    );
}

#[test]
fn test_default_constructor_super() {
    eval_ok(
        r#"
        class Base {
            constructor(x) { this.x = x; }
        }
        class Derived extends Base {
            getX() { return this.x; }
        }
        var d = new Derived(42);
        if (d.getX() !== 42) throw new Error("getX: " + d.getX());
        "#,
    );
}

#[test]
fn test_default_constructor_multi_arg() {
    eval_ok(
        r#"
        class Base {
            constructor(a, b) { this.sum = a + b; }
        }
        class Derived extends Base {}
        var d = new Derived(3, 4);
        if (d.sum !== 7) throw new Error("sum: " + d.sum);
        "#,
    );
}

#[test]
fn test_generator_parses_and_creates_function() {
    
    eval_ok(
        "var gen = function*() { yield 1; }; if (typeof gen !== 'function') throw new Error('generator not a function');",
    );
}

#[test]
fn test_generator_object_has_next() {
    eval_ok(
        "var gen = function*() { yield 1; }; var g = gen(); if (typeof g.next !== 'function') throw new Error('generator object missing next');",
    );
}
