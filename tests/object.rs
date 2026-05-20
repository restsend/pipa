use pipa::{JSRuntime, eval};

fn assert_js_ok(ctx: &mut pipa::JSContext, code: &str, msg: &str) {
    let r = eval(ctx, code);
    assert!(r.is_ok(), "{}: {:?}", msg, r);
}

#[test]
fn test_object_creation() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var obj = {a: 1, b: 2}").is_ok());
    assert!(eval(&mut ctx, "var empty = {}").is_ok());
}

#[test]
fn test_regular_for_loop() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var sum = 0; for (var i = 0; i < 3; i++) { sum = sum + 1; } if (sum !== 3) throw new Error('for loop sum mismatch');",
        "regular for loop failed",
    );
}

#[test]
fn test_object_property_access() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var obj = {a: 1}; obj.a").is_ok());
    assert_js_ok(
        &mut ctx,
        "var obj = {a: 1}; if (obj.a !== 1) throw new Error('obj.a mismatch'); var obj2 = {a: 1, b: 2}; if (obj2.a + obj2.b !== 3) throw new Error('obj sum mismatch');",
        "object property access failed",
    );
}

#[test]
fn test_object_nested() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var obj = {a: {b: 1}}; obj.a.b").is_ok());
}

#[test]
fn test_object_keys() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(&mut ctx, "var obj = {a: 1, b: 2}; Object.keys(obj).length");
    assert!(r.is_ok(), "Object.keys failed");

    let r2 = eval(&mut ctx, "Object.keys({}).length");
    assert!(r2.is_ok());
}

#[test]
fn test_object_values() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(
        &mut ctx,
        "var obj = {a: 1, b: 2}; Object.values(obj).length",
    );
    assert!(r.is_ok(), "Object.values failed");
}

#[test]
fn test_object_entries() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(&mut ctx, "var obj = {a: 1}; Object.entries(obj).length");
    assert!(r.is_ok());
}

#[test]
fn test_object_assign() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var target = {a: 1}; var src = {b: 2}; var result = Object.assign(target, src); if (result.b !== 2) throw new Error('Object.assign mismatch');",
        "object assign failed",
    );
}

#[test]
fn test_object_create() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(
        eval(
            &mut ctx,
            "var proto = {greet: 'hello'}; var obj = Object.create(proto); obj.greet"
        )
        .is_ok()
    );

    assert!(eval(&mut ctx, "var obj = Object.create(null)").is_ok());
}

#[test]
fn test_object_get_prototype_of() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(
        eval(
            &mut ctx,
            "var proto = {}; var obj = Object.create(proto); Object.getPrototypeOf(obj)"
        )
        .is_ok()
    );

    assert!(eval(&mut ctx, "Object.getPrototypeOf(null)").is_ok());
}

#[test]
fn test_object_set_prototype_of() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var proto1 = {a: 1}").is_ok());
    assert!(eval(&mut ctx, "var proto2 = {b: 2}").is_ok());
    assert!(eval(&mut ctx, "var obj = Object.create(proto1)").is_ok());
    let r = eval(&mut ctx, "Object.setPrototypeOf(obj, proto2)");
    assert!(r.is_ok());
    let r2 = eval(&mut ctx, "obj.b");
    assert!(r2.is_ok());
}

#[test]
fn test_object_prototype_has_own_property() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var obj = {a: 1}; obj.hasOwnProperty('a')").is_ok());
    assert_js_ok(
        &mut ctx,
        "var x1 = {a: 1}; if (x1.hasOwnProperty('a') !== true) throw new Error('hasOwnProperty a'); var x2 = {a: 1}; if (x2.hasOwnProperty('b') !== false) throw new Error('hasOwnProperty b'); var x3 = Object.create({a: 1}); if (x3.hasOwnProperty('a') !== false) throw new Error('proto own a'); var x4 = Object.create({a: 1}); if (x4.hasOwnProperty('b') !== false) throw new Error('proto own b');",
        "object prototype hasOwnProperty failed",
    );
}

#[test]
fn test_object_prototype_value_of() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var obj = {a: 1}; obj.valueOf()").is_ok());
}

#[test]
fn test_object_prototype_to_string() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var obj = {a: 1}; obj.toString()").is_ok());
}

#[test]
fn test_object_prototype_is_prototype_of() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let proto = eval(&mut ctx, "var proto = {}");
    assert!(proto.is_ok());
    let obj = eval(&mut ctx, "var obj = Object.create(proto)");
    assert!(obj.is_ok());
    assert_js_ok(
        &mut ctx,
        "if (proto.isPrototypeOf(obj) !== true) throw new Error('isPrototypeOf true mismatch'); if (obj.isPrototypeOf(proto) !== false) throw new Error('isPrototypeOf false mismatch');",
        "object prototype isPrototypeOf failed",
    );
}

#[test]
fn test_object_prototype_chain() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var obj = {}; Object.getPrototypeOf(obj)").is_ok());
    assert!(
        eval(
            &mut ctx,
            "var proto = {}; var obj = Object.create(proto); Object.getPrototypeOf(obj) === proto"
        )
        .is_ok()
    );
}

#[test]
fn test_function_bind() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "function add(a, b) { return a + b; }").is_ok());
    assert!(eval(&mut ctx, "var add5 = add.bind(null, 5)").is_ok());
    assert!(eval(&mut ctx, "add5(3)").is_ok());
    assert_js_ok(
        &mut ctx,
        "if (add5(3) !== 8) throw new Error('bind mismatch');",
        "function bind failed",
    );
}

#[test]
fn test_function_call() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(
        eval(
            &mut ctx,
            "function greet(greeting) { return greeting + ' ' + this.name; }"
        )
        .is_ok()
    );
    assert!(eval(&mut ctx, "var obj = {name: 'Alice'}").is_ok());
    assert!(eval(&mut ctx, "greet.call(obj, 'Hello')").is_ok());
}

#[test]
fn test_function_apply() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "function sum(a, b, c) { return a + b + c; }").is_ok());
    assert!(eval(&mut ctx, "sum.apply(null, [1, 2, 3])").is_ok());
    assert_js_ok(
        &mut ctx,
        "if (sum.apply(null, [1, 2, 3]) !== 6) throw new Error('apply mismatch');",
        "function apply failed",
    );
}

#[test]
fn test_for_in_loop() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r = eval(
        &mut ctx,
        "var keys = []; var obj = {a: 1, b: 2}; for (var k in obj) { keys.push(k); } keys.length",
    );
    assert!(r.is_ok());
    assert!(eval(
        &mut ctx,
        "var sum = 0; var obj = {a: 1, b: 2, c: 3}; for (var k in obj) { sum = sum + obj[k]; } sum"
    )
    .is_ok());
    assert_js_ok(
        &mut ctx,
        "var sum2 = 0; var obj2 = {a: 1, b: 2, c: 3}; for (var k2 in obj2) { sum2 = sum2 + obj2[k2]; } if (sum2 !== 6) throw new Error('for-in sum mismatch');",
        "for in loop failed",
    );
}

#[test]
fn test_for_in_debug() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r_outside = eval(&mut ctx, "var sum = 0; { sum = 5; } sum");
    println!("outside block sum = {:?}", r_outside.map(|v| v.get_int()));

    let r_block = eval(&mut ctx, "var sum = 0; { sum = 5; }");
    println!("inside block sum = {:?}", r_block.map(|v| v.get_int()));

    let r1 = eval(
        &mut ctx,
        "var sum = 0; for (var i = 0; i < 3; i++) { sum = 1; }",
    );
    println!("for with simple body = {:?}", r1.map(|v| v.get_int()));
}

#[test]
fn test_switch_statement() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var x = 2; switch(x) { case 1: x = 10; break; case 2: x = 20; break; case 3: x = 30; break; } if (x !== 20) throw new Error('switch case2 mismatch'); var y = 1; switch(y) { case 1: y = 10; break; default: y = 99; } if (y !== 10) throw new Error('switch case1 mismatch'); var z = 5; switch(z) { case 1: z = 10; break; default: z = 99; } if (z !== 99) throw new Error('switch default mismatch');",
        "switch statement failed",
    );
    assert!(eval(&mut ctx, "var x = 1; switch(x) { case 1: x = 10; } x").is_ok());
}

#[test]
fn test_switch_simple() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(
        &mut ctx,
        "var x = 2; switch(x) { case 1: x = 10; break; case 2: x = 20; break; } x",
    );
    match r {
        Ok(v) => println!("SIMPLE switch(2) => raw: {:?} int: {}", v, v.get_int()),
        Err(e) => println!("SIMPLE switch(2) error: {}", e),
    }
}

#[test]
fn test_switch_case1() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(
        &mut ctx,
        "var x = 1; switch(x) { case 1: x = 10; break; case 2: x = 20; break; } x",
    );
    match r {
        Ok(v) => println!("switch(1) => raw: {:?} int: {}", v, v.get_int()),
        Err(e) => println!("switch(1) error: {}", e),
    }
}

#[test]
fn test_switch_just_x() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(&mut ctx, "var x = 2; x");
    match r {
        Ok(v) => println!("just x => get_int: {}", v.get_int()),
        Err(e) => println!("error: {}", e),
    }
}

#[test]
fn test_switch_just_20() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(&mut ctx, "20");
    match r {
        Ok(v) => println!("just 20 => get_int: {}", v.get_int()),
        Err(e) => println!("error: {}", e),
    }
}

#[test]
fn test_switch_one_case() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(&mut ctx, "var x = 1; switch(x) { case 1: x = 100; } x");
    match r {
        Ok(v) => println!("one case x=1 => raw: {:?} int: {}", v, v.get_int()),
        Err(e) => println!("error: {}", e),
    }
}

#[test]
fn test_switch_zero() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let r = eval(&mut ctx, "var x = 0; switch(x) { case 0: x = 5; } x");
    match r {
        Ok(v) => println!("zero case x=0 => raw: {:?} int: {}", v, v.get_int()),
        Err(e) => println!("error: {}", e),
    }
}

#[test]
fn test_switch_fallthrough() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var x = 1; switch(x) { case 1: case 2: x = 100; break; } if (x !== 100) throw new Error('switch fallthrough mismatch');",
        "switch fallthrough failed",
    );
}

#[test]
fn test_switch_debug() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let result = eval(
        &mut ctx,
        "var x = 2; switch(x) { case 1: x = 10; break; case 2: x = 20; break; } x",
    );
    match result {
        Ok(v) => println!("OK: {:?}", v),
        Err(e) => println!("ERR: {}", e),
    }
}

#[test]
fn test_switch_debug2() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let result = eval(
        &mut ctx,
        "var x = 2; switch(x) { case 1: x = 10; break; case 2: x = 20; break; case 3: x = 30; break; } x",
    );
    match result {
        Ok(v) => {
            println!("OK: {:?}", v);
            println!("int: {}", v.get_int());
        }
        Err(e) => println!("ERR: {}", e),
    }
}

#[test]
fn test_switch_debug3() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    let result = eval(
        &mut ctx,
        "var x = 2; switch(x) { case 1: x = 10; break; case 2: x = 20; break; case 3: x = 30; break; } x",
    );
    match result {
        Ok(v) => {
            println!("OK: {:?}", v);
            println!("int: {}", v.get_int());
        }
        Err(e) => println!("ERR: {}", e),
    }
}
