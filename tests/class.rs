#![cfg(feature = "full_runtime_tests")]

use pipa::{JSRuntime, eval};

#[test]
fn test_this_in_method() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        "var obj = { x: 42, getX: function() { return this.x; } }; obj.getX()",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_int(), 42);
}

#[test]
fn test_new_basic() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        r#"
function Point(x, y) {
    this.x = x;
    this.y = y;
}
var p = new Point(3, 4);
p.x
"#,
    );
    assert!(result.is_ok(), "new Point failed: {:?}", result);
    assert_eq!(result.unwrap().get_int(), 3);
}

#[test]
fn test_new_method_access() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        r#"
function Counter(start) {
    this.count = start;
}
Counter.prototype.inc = function() { this.count = this.count + 1; };
Counter.prototype.get = function() { return this.count; };
var c = new Counter(0);
c.inc();
c.inc();
c.get()
"#,
    );
    assert!(result.is_ok(), "Counter test failed: {:?}", result);
    assert_eq!(
        result.unwrap().get_int(),
        2,
        "Counter should be 2 after two inc calls"
    );
}

#[test]
fn test_new_count_initial() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        r#"
function Counter(start) { this.count = start; }
var c = new Counter(5);
c.count
"#,
    );
    assert!(result.is_ok(), "initial count failed: {:?}", result);
    assert_eq!(result.unwrap().get_int(), 5, "initial count should be 5");
}

#[test]
fn test_inc_once() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        r#"
function Counter(start) { this.count = start; }
Counter.prototype.inc = function() { this.count = this.count + 1; };
var c = new Counter(0);
c.inc();
c.count
"#,
    );
    assert!(result.is_ok(), "inc_once failed: {:?}", result);
    assert_eq!(
        result.unwrap().get_int(),
        1,
        "count should be 1 after one inc"
    );
}

#[test]
fn test_class_basic() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        r#"
class Animal {
    constructor(name) {
        this.name = name;
    }
}
var a = new Animal("cat");
a.name
"#,
    );
    assert!(result.is_ok(), "class Animal failed: {:?}", result);
}

#[test]
fn test_class_method() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        r#"
class Counter {
    constructor(n) {
        this.n = n;
    }
    inc() {
        this.n = this.n + 1;
    }
    value() {
        return this.n;
    }
}
var c = new Counter(10);
c.inc();
c.inc();
c.value()
"#,
    );
    assert!(result.is_ok(), "class Counter failed: {:?}", result);
    assert_eq!(result.unwrap().get_int(), 12);
}

#[test]
fn test_class_static_method() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        r#"
class MathUtils {
    static add(a, b) {
        return a + b;
    }
}
MathUtils.add(3, 4)
"#,
    );
    assert!(result.is_ok(), "class static method failed: {:?}", result);
    assert_eq!(result.unwrap().get_int(), 7);
}

#[test]
fn test_class_no_constructor() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        r#"
class Noop {}
var n = new Noop();
typeof n
"#,
    );
    assert!(
        result.is_ok(),
        "class with no constructor failed: {:?}",
        result
    );
}

#[test]
fn test_multiple_instances() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        r#"
class Box {
    constructor(v) { this.v = v; }
    get() { return this.v; }
}
var a = new Box(1);
var b = new Box(2);
a.get() + b.get()
"#,
    );
    assert!(result.is_ok(), "multiple instances failed: {:?}", result);
    assert_eq!(result.unwrap().get_int(), 3);
}

#[test]
fn test_nullish_coalescing() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_eq!(eval(&mut ctx, "null ?? 42").unwrap().get_int(), 42);
    assert_eq!(eval(&mut ctx, "undefined ?? 42").unwrap().get_int(), 42);
    assert_eq!(eval(&mut ctx, "0 ?? 42").unwrap().get_int(), 0);
    assert_eq!(eval(&mut ctx, "false ?? 42").unwrap().get_bool(), false);
    assert_eq!(eval(&mut ctx, "1 ?? 42").unwrap().get_int(), 1);
}

#[test]
fn test_optional_chaining() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(&mut ctx, "var obj = null; obj?.foo");
    assert!(r.is_ok());
    assert!(r.unwrap().is_undefined());

    let r = eval(&mut ctx, "var obj = {x: 5}; obj?.x");
    assert!(r.is_ok());
    assert_eq!(r.unwrap().get_int(), 5);
}

#[test]
fn test_default_params() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(
        &mut ctx,
        "function greet(name = 'World') { return name; } greet()",
    );
    assert!(r.is_ok(), "default param failed: {:?}", r);

    let r = eval(
        &mut ctx,
        "function add(a = 0, b = 0) { return a + b; } add(3)",
    );
    assert!(r.is_ok(), "default param add failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);
}

#[test]
fn test_instanceof() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(
        &mut ctx,
        r#"
class Animal {}
var a = new Animal();
a instanceof Animal
"#,
    );
    assert!(r.is_ok(), "instanceof failed: {:?}", r);
    assert!(r.unwrap().get_bool(), "a should be instanceof Animal");

    let r = eval(
        &mut ctx,
        r#"
class Dog extends Animal {}
var d = new Dog();
d instanceof Dog
"#,
    );
    assert!(r.is_ok(), "instanceof Dog failed: {:?}", r);
}

#[test]
fn test_class_extends_basic() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(
        &mut ctx,
        r#"
class Animal {
    constructor(name) {
        this.name = name;
    }
    speak() {
        return this.name + " makes a noise";
    }
}
class Dog extends Animal {
    constructor(name) {
        super(name);
        this.type = "dog";
    }
}
var d = new Dog("Rex");
d.name
"#,
    );
    assert!(r.is_ok(), "extends failed: {:?}", r);
}

#[test]
fn test_class_extends_instanceof() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(
        &mut ctx,
        r#"
class A {}
class B extends A {}
var b = new B();
b instanceof B
"#,
    );
    assert!(r.is_ok(), "instanceof B failed: {:?}", r);
    assert!(r.unwrap().get_bool());

    let r = eval(
        &mut ctx,
        r#"
b instanceof A
"#,
    );
    assert!(r.is_ok(), "instanceof A failed: {:?}", r);
    assert!(
        r.unwrap().get_bool(),
        "b should be instanceof A (prototype chain)"
    );
}

#[test]
fn test_class_extends_method_inherit() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(
        &mut ctx,
        r#"
class Shape {
    area() { return 0; }
    describe() { return "shape"; }
}
class Circle extends Shape {
    constructor(r) {
        this.r = r;
    }
    area() { return this.r * this.r; }
}
var c = new Circle(5);
c.area()
"#,
    );
    assert!(r.is_ok(), "extends method override failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 25);
}

#[test]
fn test_class_field_basic() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(
        &mut ctx,
        r#"
class Foo {
    x = 42;
}
var f = new Foo();
f.x
"#,
    );
    assert!(r.is_ok(), "class field basic failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 42);
}

#[test]
fn test_class_field_no_initializer() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(
        &mut ctx,
        r#"
class Foo {
    x;
}
var f = new Foo();
f.x === undefined
"#,
    );
    assert!(r.is_ok(), "class field no init failed: {:?}", r);
    assert_eq!(r.unwrap().is_truthy(), true);
}

#[test]
fn test_class_field_multiple() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(
        &mut ctx,
        r#"
class Point {
    x = 10;
    y = 20;
}
var p = new Point();
p.x + p.y
"#,
    );
    assert!(r.is_ok(), "class field multiple failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 30);
}

#[test]
fn test_class_field_with_constructor() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(
        &mut ctx,
        r#"
class Foo {
    x = 1;
    y = 2;
    constructor(z) {
        this.z = z;
    }
}
var f = new Foo(3);
f.x + f.y + f.z
"#,
    );
    assert!(r.is_ok(), "class field with constructor failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 6);
}

#[test]
fn test_class_field_constructor_overrides() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(
        &mut ctx,
        r#"
class Foo {
    x = 10;
    constructor() {
        this.x = 99;
    }
}
var f = new Foo();
f.x
"#,
    );
    assert!(r.is_ok(), "class field override failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 99);
}

#[test]
fn test_class_field_expression_initializer() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(
        &mut ctx,
        r#"
class Foo {
    x = 3 + 4;
    y = "hello";
}
var f = new Foo();
f.x
"#,
    );
    assert!(r.is_ok(), "class field expr init failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 7);
}

#[test]
fn test_class_field_per_instance() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(
        &mut ctx,
        r#"
class Counter {
    count = 0;
    inc() { this.count = this.count + 1; }
}
var a = new Counter();
var b = new Counter();
a.inc();
a.inc();
b.inc();
a.count - b.count
"#,
    );
    assert!(r.is_ok(), "class field per-instance failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 1);
}

#[test]
fn test_class_field_with_method() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(
        &mut ctx,
        r#"
class Rect {
    width = 5;
    height = 10;
    area() { return this.width * this.height; }
}
var r = new Rect();
r.area()
"#,
    );
    assert!(r.is_ok(), "class field with method failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 50);
}

#[test]
fn test_private_instance_field_basic() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(
        &mut ctx,
        "class A { #x = 42; getX() { return this.#x; } }; new A().getX()",
    );
    assert!(r.is_ok(), "private instance field basic failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 42);
}

#[test]
fn test_private_instance_field_expression() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(
        &mut ctx,
        "class B { #y = 1 + 2; getY() { return this.#y; } }; new B().getY()",
    );
    assert!(
        r.is_ok(),
        "private instance field expression failed: {:?}",
        r
    );
    assert_eq!(r.unwrap().get_int(), 3);
}

#[test]
fn test_private_instance_field_multiple() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(
        &mut ctx,
        "class C { #a = 1; #b = 2; getSum() { return this.#a + this.#b; } }; new C().getSum()",
    );
    assert!(
        r.is_ok(),
        "multiple private instance fields failed: {:?}",
        r
    );
    assert_eq!(r.unwrap().get_int(), 3);
}

#[test]
fn test_private_static_field_basic() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(
        &mut ctx,
        "class D { static #x = 42; static getX() { return D.#x; } }; D.getX()",
    );
    assert!(r.is_ok(), "private static field basic failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 42);
}

#[test]
fn test_private_field_in_check() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(
        &mut ctx,
        "class A { #x = 42; hasX(obj) { return #x in obj; } }; var a = new A(); a.hasX(a)",
    );
    assert!(r.is_ok(), "private field in check failed: {:?}", r);
    assert!(r.unwrap().get_bool(), "should have private field #x");

    let r2 = eval(
        &mut ctx,
        "class B { #y = 1; hasY(obj) { return #y in obj; } }; var b = new B(); b.hasY({})",
    );
    assert!(
        r2.is_ok(),
        "private field in check (negative) failed: {:?}",
        r2
    );
    assert!(!r2.unwrap().get_bool(), "should not have private field #y");
}
