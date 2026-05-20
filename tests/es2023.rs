#![cfg(feature = "full_runtime_tests")]

use pipa::{JSContext, JSRuntime, eval, run_event_loop};
#[test]
fn test_exponentiation() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_eq!(eval(&mut ctx, "2 ** 10").unwrap().get_int(), 1024);
    assert_eq!(eval(&mut ctx, "2 ** 0").unwrap().get_int(), 1);
    assert_eq!(
        eval(&mut ctx, "var x = 3; x **= 2; x").unwrap().get_int(),
        9
    );
}

#[test]
fn test_number_static_methods() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert!(eval(&mut ctx, "Number.isNaN(NaN)").unwrap().get_bool());
    assert!(!eval(&mut ctx, "Number.isNaN(42)").unwrap().get_bool());
    assert!(eval(&mut ctx, "Number.isFinite(42)").unwrap().get_bool());
    assert!(
        !eval(&mut ctx, "Number.isFinite(Infinity)")
            .unwrap()
            .get_bool()
    );
    assert!(eval(&mut ctx, "Number.isInteger(42)").unwrap().get_bool());
    assert!(!eval(&mut ctx, "Number.isInteger(42.5)").unwrap().get_bool());
}

#[test]
fn test_array_es2023_methods() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    assert_eq!(eval(&mut ctx, "[1,2,3].at(-1)").unwrap().get_int(), 3);
    assert_eq!(eval(&mut ctx, "[1,2,3].at(0)").unwrap().get_int(), 1);

    let r = eval(&mut ctx, "[3,1,2].toSorted()");
    assert!(r.is_ok(), "toSorted failed: {:?}", r);

    let r = eval(&mut ctx, "[1,2,3].toReversed()");
    assert!(r.is_ok(), "toReversed failed: {:?}", r);

    let r = eval(&mut ctx, "[1,2,3,4,5].toSpliced(1, 2, 'x', 'y')");
    assert!(r.is_ok(), "toSpliced failed: {:?}", r);

    let r = eval(
        &mut ctx,
        "var arr = [1,2,3,4,5]; arr.toSpliced(1, 2); arr.length",
    );
    assert_eq!(
        r.unwrap().get_int(),
        5,
        "original array should not be modified"
    );

    assert_eq!(
        eval(&mut ctx, "[1,2,3,4].findLast(x => x % 2 === 0)")
            .unwrap()
            .get_int(),
        4
    );

    let r = eval(&mut ctx, "var arr = [1,2,3]; arr.with(1, 'x')");
    assert!(r.is_ok(), "with failed: {:?}", r);

    assert_eq!(
        eval(&mut ctx, "var arr = [1,2,3]; arr.with(1, 'x'); arr.length")
            .unwrap()
            .get_int(),
        3
    );

    let r = eval(&mut ctx, "[1,[2,3],[4,[5]]].flat()");
    assert!(r.is_ok(), "flat failed: {:?}", r);

    let r = eval(&mut ctx, "[1,2,3].flatMap(x => [x, x*2])");
    assert!(r.is_ok(), "flatMap failed: {:?}", r);

    let r = eval(&mut ctx, "var a = [3,1,2]; a.sort(); a[0]");
    assert!(r.is_ok(), "sort failed: {:?}", r);

    assert_eq!(
        eval(&mut ctx, "var a = [1,2,3]; a.reverse(); a[0]")
            .unwrap()
            .get_int(),
        3
    );
}

#[test]
fn test_object_new_methods() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    assert!(
        eval(
            &mut ctx,
            "var o = Object.fromEntries([['a',1],['b',2]]); o.a"
        )
        .is_ok()
    );

    assert!(
        eval(&mut ctx, "Object.hasOwn({x: 1}, 'x')")
            .unwrap()
            .get_bool()
    );
    assert!(
        !eval(&mut ctx, "Object.hasOwn({x: 1}, 'y')")
            .unwrap()
            .get_bool()
    );

    let r = eval(&mut ctx, "Object.is(NaN, NaN)");
    assert!(r.is_ok(), "Object.is failed: {:?}", r);
}

#[test]
fn test_string_new_methods() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "'hello world hello'.replaceAll('hello', 'hi')");
    assert!(r.is_ok(), "replaceAll failed: {:?}", r);

    let r = eval(&mut ctx, "'hello'.at(-1)");
    assert!(r.is_ok(), "string.at failed: {:?}", r);

    let r = eval(&mut ctx, "'hello'.isWellFormed()");
    assert!(r.is_ok(), "isWellFormed failed: {:?}", r);
    assert!(
        r.unwrap().get_bool(),
        "isWellFormed should return true for normal string"
    );

    let r = eval(&mut ctx, "'hello'.toWellFormed()");
    assert!(r.is_ok(), "toWellFormed failed: {:?}", r);
}

#[test]
fn test_hashbang() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    let r = eval(&mut ctx, "#!/usr/bin/env node\n1 + 2");
    assert!(r.is_ok(), "hashbang failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);
}

#[test]
fn test_array_from_async() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    assert!(
        eval(&mut ctx, "Array.fromAsync").is_ok(),
        "Array.fromAsync should be accessible"
    );

    let r = eval(&mut ctx, "Array.fromAsync([1, 2, 3])");
    assert!(r.is_ok(), "fromAsync([1,2,3]) failed: {:?}", r);
    let result = r.unwrap();
    assert!(result.is_object(), "fromAsync should return an object");

    let r = eval(&mut ctx, "Array.fromAsync(null).length");
    assert!(r.is_ok(), "fromAsync(null) failed: {:?}", r);

    let r = eval(&mut ctx, "Array.fromAsync(undefined).length");
    assert!(r.is_ok(), "fromAsync(undefined) failed: {:?}", r);
}

#[test]
fn test_async_function_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "(async function() { return 42; })()");
    assert!(r.is_ok(), "async function failed: {:?}", r);
    let result = r.unwrap();
    assert!(
        result.is_object(),
        "async function should return an object (Promise)"
    );
}

#[test]
fn test_async_function_with_await() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "(async function() { return await Promise.resolve(42); })()",
    );
    assert!(r.is_ok(), "async function with await failed: {:?}", r);
    let result = r.unwrap();
    assert!(
        result.is_object(),
        "async function should return an object (Promise)"
    );
}

#[test]
fn test_promise_resolve() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "Promise.resolve(42)");
    assert!(r.is_ok(), "Promise.resolve failed: {:?}", r);
    let result = r.unwrap();
    assert!(
        result.is_object(),
        "Promise.resolve should return an object"
    );
}

#[test]
fn test_promise_all() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "Promise.all");
    assert!(r.is_ok(), "Promise.all should be accessible");
    assert!(r.unwrap().is_function(), "Promise.all should be a function");

    let r = eval(&mut ctx, "Promise.all([])");
    assert!(r.is_ok(), "Promise.all([]) failed: {:?}", r);
    assert!(
        r.unwrap().is_object(),
        "Promise.all([]) should return an object"
    );

    let r = eval(&mut ctx, "Promise.all([1, 2, 3])");
    assert!(r.is_ok(), "Promise.all([1,2,3]) failed: {:?}", r);
}

#[test]
fn test_promise_race() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "Promise.race");
    assert!(r.is_ok(), "Promise.race should be accessible");
    assert!(
        r.unwrap().is_function(),
        "Promise.race should be a function"
    );

    let r = eval(&mut ctx, "Promise.race([])");
    assert!(r.is_ok(), "Promise.race([]) failed: {:?}", r);

    let r = eval(&mut ctx, "Promise.race([1, 2, 3])");
    assert!(r.is_ok(), "Promise.race([1,2,3]) failed: {:?}", r);
}

#[test]
fn test_promise_all_settled() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "Promise.allSettled");
    assert!(r.is_ok(), "Promise.allSettled should be accessible");
    assert!(
        r.unwrap().is_function(),
        "Promise.allSettled should be a function"
    );

    let r = eval(&mut ctx, "Promise.allSettled([])");
    assert!(r.is_ok(), "Promise.allSettled([]) failed: {:?}", r);
    assert!(
        r.unwrap().is_object(),
        "Promise.allSettled([]) should return an object"
    );

    let r = eval(&mut ctx, "Promise.allSettled([1, 2, 3])");
    assert!(r.is_ok(), "Promise.allSettled([1,2,3]) failed: {:?}", r);
}

#[test]
fn test_promise_any() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "Promise.any");
    assert!(r.is_ok(), "Promise.any should be accessible");
    assert!(r.unwrap().is_function(), "Promise.any should be a function");

    let r = eval(&mut ctx, "Promise.any([])");
    assert!(r.is_ok(), "Promise.any([]) failed: {:?}", r);
    assert!(
        r.unwrap().is_object(),
        "Promise.any([]) should return an object"
    );

    let r = eval(&mut ctx, "Promise.any([1, 2, 3])");
    assert!(r.is_ok(), "Promise.any([1,2,3]) failed: {:?}", r);
}

#[test]
fn test_import_syntax() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./module.js".to_string(),
        "export var x = 42;".to_string(),
    );
    module.add_export("x".to_string(), pipa::JSValue::new_int(42), false);
    rt.module_registry_mut().register(module);

    let r = eval(&mut ctx, "import './module.js';");
    assert!(r.is_ok(), "import syntax failed: {:?}", r);
}

#[test]
#[ignore = "import system needs full implementation"]
fn test_import_namespace() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./module.js".to_string(),
        "export var x = 42;".to_string(),
    );
    module.add_export("x".to_string(), pipa::JSValue::new_int(42), false);
    rt.module_registry_mut().register(module);

    let r = eval(&mut ctx, "import * as ns from './module.js'; ns.x");
    assert!(r.is_ok(), "import * as ns failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 42);
}

#[test]
#[ignore = "import system needs full implementation"]
fn test_import_named() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./module.js".to_string(),
        "export var x = 42;".to_string(),
    );
    module.add_export("x".to_string(), pipa::JSValue::new_int(42), false);
    rt.module_registry_mut().register(module);

    let r = eval(&mut ctx, "import { x } from './module.js'; x");
    assert!(r.is_ok(), "import named failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 42);
}

#[test]
#[ignore = "import system needs full implementation"]
fn test_import_default() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./module.js".to_string(),
        "export default 42;".to_string(),
    );
    module.add_export("default".to_string(), pipa::JSValue::new_int(42), false);
    rt.module_registry_mut().register(module);

    let r = eval(&mut ctx, "import x from './module.js'; x");
    assert!(r.is_ok(), "import default failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 42);
}

#[test]
fn test_export_named() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "var x = 1; export { x };");
    assert!(r.is_ok(), "export named failed: {:?}", r);
}

#[test]
fn test_export_default() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "export default 42;");
    assert!(r.is_ok(), "export default failed: {:?}", r);
}

#[test]
fn test_export_var() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "export var y = 2;");
    assert!(r.is_ok(), "export var failed: {:?}", r);
}

#[test]
fn test_export_function() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "export function foo() { return 42; }");
    assert!(r.is_ok(), "export function failed: {:?}", r);
}

#[test]
fn test_for_of_array() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "var sum = 0; for (var x of [1, 2, 3]) { sum += x; } sum",
    );
    assert!(r.is_ok(), "for...of failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 6);
}

#[test]
fn test_for_of_string() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "var chars = ''; for (var c of 'ab') { chars += c; } chars",
    );
    assert!(r.is_ok(), "for...of string failed: {:?}", r);
}

#[test]
fn test_for_of_object() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "var m = new Map(); m.set('a', 1); m.set('b', 2); var sum = 0; for (var v of m.values()) { sum += v; } sum",
    );
    assert!(r.is_ok(), "for...of Map failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);
}

#[test]
fn test_generator_function() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "(function*() { yield 1; })");
    assert!(r.is_ok(), "function* failed: {:?}", r);
}

#[test]
fn test_error_cause() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "typeof Error");
    assert!(r.is_ok(), "Error typeof failed: {:?}", r);
    let typeof_err = r.unwrap();
    let type_str = ctx.get_atom_str(typeof_err.get_atom()).to_string();
    assert_eq!(
        type_str, "function",
        "Error should be a function, got: {}",
        type_str
    );

    let r = eval(&mut ctx, "var e = new Error('test'); typeof e");
    assert!(r.is_ok(), "Error constructor failed: {:?}", r);
    let typeof_e = r.unwrap();
    let e_type = ctx.get_atom_str(typeof_e.get_atom()).to_string();
    assert_eq!(
        e_type, "object",
        "new Error should return object, got: {}",
        e_type
    );

    let r = eval(&mut ctx, "var e2 = new Error('hello'); e2.message");
    assert!(r.is_ok(), "Error.message access failed: {:?}", r);
    let msg = r.unwrap();
    assert!(
        msg.is_string(),
        "message should be a string, got: {:?}",
        msg
    );

    let r = eval(
        &mut ctx,
        "var e3 = new Error('test', { cause: 'original' }); e3.cause",
    );
    assert!(r.is_ok(), "Error.cause access failed: {:?}", r);
    let cause = r.unwrap();
    assert!(
        cause.is_string(),
        "cause should be a string, got: {:?}",
        cause
    );
}

#[test]
fn test_static_class_fields() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "class A { static x = 42; }; A.x");
    assert!(r.is_ok(), "static class field failed: {:?}", r);
    let val = r.unwrap();
    assert_eq!(val.get_int(), 42, "static field should be 42");

    let r = eval(&mut ctx, "class B { static y = 1 + 2; }; B.y");
    assert!(r.is_ok(), "static field expression failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);

    let r = eval(
        &mut ctx,
        "class C { static a = 1; static b = 2; }; C.a + C.b",
    );
    assert!(r.is_ok(), "multiple static fields failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);

    let r = eval(&mut ctx, "class D { static msg = 'hello'; }; D.msg");
    assert!(r.is_ok(), "static string field failed: {:?}", r);
    let val = r.unwrap();
    assert!(val.is_string(), "msg should be string");
}

#[test]
fn test_instance_class_fields() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "class A { x = 42; }; var a = new A(); a.x");
    assert!(r.is_ok(), "instance class field failed: {:?}", r);
    let val = r.unwrap();
    assert_eq!(val.get_int(), 42, "instance field should be 42");

    let r = eval(&mut ctx, "class B { y = 1 + 2; }; var b = new B(); b.y");
    assert!(r.is_ok(), "instance field expression failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);

    let r = eval(
        &mut ctx,
        "class C { a = 1; b = 2; }; var c = new C(); c.a + c.b",
    );
    assert!(r.is_ok(), "multiple instance fields failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);

    let r = eval(
        &mut ctx,
        "class D { msg = 'hello'; }; var d = new D(); d.msg",
    );
    assert!(r.is_ok(), "instance string field failed: {:?}", r);
    let val = r.unwrap();
    assert!(val.is_string(), "msg should be string");
}

#[test]
#[ignore = "static init block needs fixing"]
fn test_static_initialization_block() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "class A { static { A.x = 42; } }; A.x");
    assert!(r.is_ok(), "static block failed: {:?}", r);
    let val = r.unwrap();
    assert_eq!(val.get_int(), 42, "static block should set A.x = 42");

    let r = eval(
        &mut ctx,
        "class B { static y = 1; static { B.y += 10; } }; B.y",
    );
    assert!(r.is_ok(), "static block after field failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 11);

    let r = eval(
        &mut ctx,
        "class C { static { C.sum = 0; } static { C.sum += 5; } static { C.sum += 7; } }; C.sum",
    );
    assert!(r.is_ok(), "multiple static blocks failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 12);

    let r = eval(&mut ctx, "class D { static { this.value = 99; } }; D.value");
    assert!(r.is_ok(), "static block with this failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 99);
}

#[test]
fn test_private_instance_fields() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "class A { #x = 42; getX() { return this.#x; } }; var a = new A(); a.getX()",
    );
    assert!(r.is_ok(), "private field failed: {:?}", r);
    let val = r.unwrap();
    assert_eq!(val.get_int(), 42, "private field should be 42");

    let r = eval(
        &mut ctx,
        "class B { #y = 1 + 2; getY() { return this.#y; } }; var b = new B(); b.getY()",
    );
    assert!(r.is_ok(), "private field expression failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);

    let r = eval(
        &mut ctx,
        "class C { #a = 1; #b = 2; getSum() { return this.#a + this.#b; } }; var c = new C(); c.getSum()",
    );
    assert!(r.is_ok(), "multiple private fields failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);

    let r = eval(
        &mut ctx,
        "class D { #msg = 'hello'; getMsg() { return this.#msg; } }; var d = new D(); d.getMsg()",
    );
    assert!(r.is_ok(), "private string field failed: {:?}", r);
    let val = r.unwrap();
    assert!(val.is_string(), "msg should be string");
}

#[test]
fn test_private_static_fields() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "class A { static #x = 42; static getX() { return A.#x; } }; A.getX()",
    );
    assert!(r.is_ok(), "private static field failed: {:?}", r);
    let val = r.unwrap();
    assert_eq!(val.get_int(), 42, "private static field should be 42");

    let r = eval(
        &mut ctx,
        "class B { static #y = 1 + 2; static getY() { return B.#y; } }; B.getY()",
    );
    assert!(r.is_ok(), "private static field expression failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);

    let r = eval(
        &mut ctx,
        "class C { static #s = 10; #i = 5; getSum() { return C.#s + this.#i; } }; var c = new C(); c.getSum()",
    );
    assert!(r.is_ok(), "mixed private fields failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 15);
}

#[test]
fn test_private_field_in_check() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "class A { #x = 42; hasX(obj) { return #x in obj; } }; var a = new A(); a.hasX(a)",
    );
    assert!(r.is_ok(), "private field in check failed: {:?}", r);
    assert!(r.unwrap().get_bool(), "should have private field #x");

    let r = eval(
        &mut ctx,
        "class B { #y = 1; hasY(obj) { return #y in obj; } }; var b = new B(); b.hasY({})",
    );
    assert!(
        r.is_ok(),
        "private field in check (negative) failed: {:?}",
        r
    );
    assert!(!r.unwrap().get_bool(), "should not have private field #y");

    let r = eval(
        &mut ctx,
        "class C { #z = 1; check(a, b) { return #z in a && !(#z in b); } }; var c1 = new C(); var c2 = {}; c1.check(c1, c2)",
    );
    assert!(
        r.is_ok(),
        "private field in check with two objects failed: {:?}",
        r
    );
    assert!(r.unwrap().get_bool(), "c1 should have #z, c2 should not");
}

#[test]
fn test_symbol_weakmap_keys() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "var wm = new WeakMap(); var sym = Symbol('key'); wm.set(sym, 'value')",
    );
    assert!(r.is_ok(), "WeakMap.set with symbol key failed: {:?}", r);

    let r = eval(
        &mut ctx,
        "var wm = new WeakMap(); var sym = Symbol('key'); wm.set(sym, 'value'); wm.get(sym)",
    );
    assert!(r.is_ok(), "WeakMap.get with symbol key failed: {:?}", r);

    let r = eval(
        &mut ctx,
        "var wm = new WeakMap(); var sym = Symbol('key'); wm.set(sym, 'value'); wm.has(sym)",
    );
    assert!(r.is_ok(), "WeakMap.has with symbol key failed: {:?}", r);
    assert!(r.unwrap().get_bool(), "WeakMap should have symbol key");
}

#[test]
fn test_symbol_weakset_values() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "var ws = new WeakSet(); var sym = Symbol('val'); ws.add(sym)",
    );
    assert!(r.is_ok(), "WeakSet.add with symbol failed: {:?}", r);

    let r = eval(
        &mut ctx,
        "var ws = new WeakSet(); var sym = Symbol('val'); ws.add(sym); ws.has(sym)",
    );
    assert!(r.is_ok(), "WeakSet.has with symbol failed: {:?}", r);
    assert!(r.unwrap().get_bool(), "WeakSet should have symbol");
}

#[test]
fn test_bigint_as_int_n() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "BigInt.asIntN(8, 255n)");
    assert!(r.is_ok(), "BigInt.asIntN failed: {:?}", r);
    assert!(r.unwrap().is_bigint(), "BigInt.asIntN should return BigInt");

    let r = eval(&mut ctx, "BigInt.asIntN(8, 128n)");
    assert!(r.is_ok(), "BigInt.asIntN(8, 128n) failed: {:?}", r);

    let r = eval(&mut ctx, "BigInt.asIntN(8, 127n)");
    assert!(r.is_ok(), "BigInt.asIntN(8, 127n) failed: {:?}", r);
}

#[test]
fn test_bigint_as_uint_n() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "BigInt.asUintN(8, -1n)");
    assert!(r.is_ok(), "BigInt.asUintN failed: {:?}", r);
    assert!(
        r.unwrap().is_bigint(),
        "BigInt.asUintN should return BigInt"
    );

    let r = eval(&mut ctx, "BigInt.asUintN(8, 256n)");
    assert!(r.is_ok(), "BigInt.asUintN(8, 256n) failed: {:?}", r);

    let r = eval(&mut ctx, "BigInt.asUintN(8, 255n)");
    assert!(r.is_ok(), "BigInt.asUintN(8, 255n) failed: {:?}", r);
}

#[test]
fn test_weakref_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "var obj = {x: 42}; var wr = new WeakRef(obj); wr.deref().x",
    );
    assert!(r.is_ok(), "WeakRef failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 42);
}

#[test]
fn test_finalization_registry_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "typeof FinalizationRegistry");
    assert!(r.is_ok(), "FinalizationRegistry typeof failed: {:?}", r);

    let r = eval(
        &mut ctx,
        r#"
        var fr = new FinalizationRegistry(v => {});
        var obj = {x: 1};
        var token = {};
        fr.register(obj, "held", token);
        fr.unregister(token)
    "#,
    );
    assert!(r.is_ok(), "FinalizationRegistry failed: {:?}", r);
    assert!(r.unwrap().get_bool(), "unregister should return true");
}
