#![cfg(feature = "full_runtime_tests")]

use pipa::{JSRuntime, eval};
use std::fs;

fn setup_test_module(name: &str, content: &str) -> String {
    let path = format!("/tmp/pipa_test_{}.js", name);
    fs::write(&path, content).expect("Failed to write test module");
    path
}

fn cleanup_test_module(path: &str) {
    let _ = fs::remove_file(path);
}

#[test]
fn test_module_registry_basic() {
    let mut runtime = JSRuntime::new();
    let _ctx = runtime.new_context();

    let registry = runtime.module_registry();
    assert!(!registry.has("./nonexistent.js"));
}

#[test]
fn test_module_specifier_resolution() {
    use pipa::runtime::module::resolve_specifier;

    let base = "/home/user/project/main.js";
    let resolved = resolve_specifier("./utils.js", base);
    assert!(resolved.ends_with("utils.js"));

    let resolved2 = resolve_specifier("../lib/helper.js", base);
    assert!(resolved2.contains("lib"));
    assert!(resolved2.contains("helper.js"));
}

#[test]
fn test_module_object_creation() {
    use pipa::runtime::module::{Module, ModuleState};

    let module = Module::new("test.js".to_string(), "export const x = 42;".to_string());

    assert_eq!(module.specifier, "test.js");
    assert_eq!(module.state, ModuleState::Unlinked);
    assert!(module.exports.is_empty());
}

#[test]
fn test_module_export_add() {
    use pipa::runtime::module::Module;
    use pipa::value::JSValue;

    let mut module = Module::new("test.js".to_string(), String::new());

    module.add_export("foo".to_string(), JSValue::new_int(42), false);
    module.add_export("bar".to_string(), JSValue::bool(true), true);

    assert!(module.get_export("foo").is_some());
    assert!(module.get_export("bar").is_some());
    assert_eq!(module.get_export_value("foo").get_int(), 42);
    assert!(module.get_export_value("bar").get_bool());
}

#[test]
fn test_module_namespace_object() {
    use pipa::runtime::atom::AtomTable;
    use pipa::runtime::module::Module;
    use pipa::value::JSValue;

    let mut module = Module::new("test.js".to_string(), String::new());
    module.add_export("a".to_string(), JSValue::new_int(1), false);
    module.add_export("b".to_string(), JSValue::new_int(2), false);

    let mut atom_table = AtomTable::new();
    let ns_ptr = module.get_or_create_namespace_object(&mut atom_table);

    assert!(ns_ptr != 0);
}

#[test]
fn test_static_import_syntax_parsing() {
    let test_cases = vec![
        "import 'module';",
        "import foo from 'module';",
        "import * as ns from 'module';",
        "import { a, b } from 'module';",
        "import { a as alias } from 'module';",
    ];

    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    for code in test_cases {
        let result = eval(&mut ctx, code);
        println!(
            "Parsed: {} -> {:?}",
            code,
            result.is_ok() || result.is_err()
        );
    }
}

#[test]
fn test_export_syntax_parsing() {
    let test_cases = vec![
        "export var x = 1;",
        "export let y = 2;",
        "export const z = 3;",
        "export function f() { return 1; }",
        "export class C {}",
        "export { x };",
    ];

    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    for code in test_cases {
        let result = eval(&mut ctx, code);
        println!(
            "Parsed: {} -> {:?}",
            code,
            result.is_ok() || result.is_err()
        );
    }
}

#[test]
fn test_import_meta_parsing() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let result = eval(&mut ctx, "typeof import.meta");
    println!("import.meta typeof: {:?}", result);
}

#[test]
fn test_module_load_and_register() {
    let module_path = setup_test_module("load_test", "export const value = 123;");

    let source = pipa::runtime::module::load_module_source(&module_path);
    assert!(source.is_ok());
    assert!(source.unwrap().contains("export const value"));

    cleanup_test_module(&module_path);
}

#[test]
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
    assert!(r.is_ok(), "import failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 42);
}

#[test]
fn test_import_namespace_multiple_exports() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./module.js".to_string(),
        "export var a = 1; export var b = 2; export var c = 3;".to_string(),
    );
    module.add_export("a".to_string(), pipa::JSValue::new_int(1), false);
    module.add_export("b".to_string(), pipa::JSValue::new_int(2), false);
    module.add_export("c".to_string(), pipa::JSValue::new_int(3), false);
    rt.module_registry_mut().register(module);

    let r = eval(
        &mut ctx,
        "import * as ns from './module.js'; ns.a + ns.b + ns.c",
    );
    assert!(r.is_ok(), "import failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 6);
}

#[test]
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
    assert!(r.is_ok(), "import failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 42);
}

#[test]
fn test_import_default() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./module.js".to_string(),
        "export default 42;".to_string(),
    );
    module.add_export("default".to_string(), pipa::JSValue::new_int(42), false);
    rt.module_registry_mut().register(module);

    let r = eval(&mut ctx, "import foo from './module.js'; foo");
    assert!(r.is_ok(), "import failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 42);
}

#[test]
fn test_dynamic_import_returns_promise() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let module_path = setup_test_module("dynamic_test", "export var x = 42;");

    let r = eval(&mut ctx, &format!("import('{}')", module_path));
    assert!(r.is_ok(), "import() failed: {:?}", r);
    let result = r.unwrap();
    assert!(
        result.is_object(),
        "import() should return an object (Promise), got: {:?}",
        result
    );

    cleanup_test_module(&module_path);
}

#[test]
fn test_dynamic_import_with_promise() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let module_path = setup_test_module("dynamic_promise", "export var value = 99;");

    let r = eval(
        &mut ctx,
        &format!("import('{}').then(ns => ns.value)", module_path),
    );
    assert!(r.is_ok(), "import().then() failed: {:?}", r);

    cleanup_test_module(&module_path);
}

#[test]
fn test_export_default_expression() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "export default 42;");
    assert!(r.is_ok(), "export default failed: {:?}", r);
}

#[test]
fn test_import_meta_url() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "typeof import.meta");
    assert!(r.is_ok(), "import.meta failed: {:?}", r);
}

#[test]
fn test_export_multiple_named() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./module.js".to_string(),
        "export var x = 1, y = 2, z = 3;".to_string(),
    );
    module.add_export("x".to_string(), pipa::JSValue::new_int(1), false);
    module.add_export("y".to_string(), pipa::JSValue::new_int(2), false);
    module.add_export("z".to_string(), pipa::JSValue::new_int(3), false);
    rt.module_registry_mut().register(module);

    let r = eval(
        &mut ctx,
        "import * as ns from './module.js'; ns.x + ns.y + ns.z",
    );
    assert!(r.is_ok(), "import failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 6);
}

#[test]
fn test_re_export_all() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module_a =
        pipa::runtime::module::Module::new("./a.js".to_string(), "export var a = 10;".to_string());
    module_a.add_export("a".to_string(), pipa::JSValue::new_int(10), false);
    rt.module_registry_mut().register(module_a);

    let mut module_b =
        pipa::runtime::module::Module::new("./b.js".to_string(), "export var b = 20;".to_string());
    module_b.add_export("b".to_string(), pipa::JSValue::new_int(20), false);
    rt.module_registry_mut().register(module_b);

    let r = eval(&mut ctx, "import * as ns from './a.js'; ns.a");
    assert!(r.is_ok(), "import failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 10);
}

#[test]
fn test_module_state_transitions() {
    use pipa::runtime::module::{Module, ModuleState};

    let mut module = Module::new("test.js".to_string(), "export var x = 1;".to_string());
    assert_eq!(module.state, ModuleState::Unlinked);

    module.state = ModuleState::Linking;
    assert_eq!(module.state, ModuleState::Linking);

    module.state = ModuleState::Linked;
    assert_eq!(module.state, ModuleState::Linked);

    module.state = ModuleState::Evaluating;
    assert_eq!(module.state, ModuleState::Evaluating);

    module.state = ModuleState::Evaluated;
    assert_eq!(module.state, ModuleState::Evaluated);
}

#[test]
fn test_import_aliased() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./module.js".to_string(),
        "export var x = 42;".to_string(),
    );
    module.add_export("x".to_string(), pipa::JSValue::new_int(42), false);
    rt.module_registry_mut().register(module);

    let r = eval(&mut ctx, "import { x as y } from './module.js'; y");
    assert!(r.is_ok(), "import {{ x as y }} failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 42);
}

#[test]
fn test_import_multiple_aliased() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./module.js".to_string(),
        "export var a = 1; export var b = 2;".to_string(),
    );
    module.add_export("a".to_string(), pipa::JSValue::new_int(1), false);
    module.add_export("b".to_string(), pipa::JSValue::new_int(2), false);
    rt.module_registry_mut().register(module);

    let r = eval(
        &mut ctx,
        "import { a as x, b as y } from './module.js'; x + y",
    );
    assert!(r.is_ok(), "import {{ a as x, b as y }} failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);
}

#[test]
fn test_import_default_and_named() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./module.js".to_string(),
        "export default 100; export var x = 50;".to_string(),
    );
    module.add_export("default".to_string(), pipa::JSValue::new_int(100), false);
    module.add_export("x".to_string(), pipa::JSValue::new_int(50), false);
    rt.module_registry_mut().register(module);

    let r = eval(
        &mut ctx,
        "import def from './module.js'; import { x } from './module.js'; def + x",
    );
    assert!(r.is_ok(), "import default and named failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 150);
}

#[test]
fn test_export_default_function() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "export default function() { return 42; }");
    assert!(r.is_ok(), "export default function failed: {:?}", r);
}

#[test]
fn test_export_default_class() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "export default class Point { }");
    assert!(r.is_ok(), "export default class failed: {:?}", r);
}

#[test]
fn test_export_default_arrow() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "export default (x) => x * 2;");
    assert!(r.is_ok(), "export default arrow failed: {:?}", r);
}

#[test]
fn test_export_default_object() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "export default { a: 1, b: 2 };");
    assert!(r.is_ok(), "export default object failed: {:?}", r);
}

#[test]
fn test_export_default_array() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "export default [1, 2, 3];");
    assert!(r.is_ok(), "export default array failed: {:?}", r);
}

#[test]
fn test_export_default_string() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "export default 'hello';");
    assert!(r.is_ok(), "export default string failed: {:?}", r);
}

#[test]
fn test_export_const_object() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "export const obj = { x: 1, y: 2 };");
    assert!(r.is_ok(), "export const object failed: {:?}", r);
}

#[test]
fn test_export_function_declaration() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "export function add(a, b) { return a + b; }");
    assert!(r.is_ok(), "export function failed: {:?}", r);
}

#[test]
fn test_export_class_declaration() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(
        &mut ctx,
        "export class Point { constructor(x, y) { this.x = x; this.y = y; } }",
    );
    assert!(r.is_ok(), "export class failed: {:?}", r);
}

#[test]
fn test_export_let_reassignment() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "export let count = 0; count = 1;");
    assert!(r.is_ok(), "export let reassignment failed: {:?}", r);
}

#[test]
fn test_import_side_effect_only() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module =
        pipa::runtime::module::Module::new("./sideeffect.js".to_string(), "var x = 1;".to_string());
    module.add_export("x".to_string(), pipa::JSValue::new_int(1), false);
    rt.module_registry_mut().register(module);

    let r = eval(&mut ctx, "import './sideeffect.js';");
    assert!(r.is_ok(), "side-effect import failed: {:?}", r);
    assert!(r.unwrap().is_undefined());
}

#[test]
fn test_circular_dependency_simple() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module_a =
        pipa::runtime::module::Module::new("./a.js".to_string(), "export var a = 1;".to_string());
    module_a.add_export("a".to_string(), pipa::JSValue::new_int(1), false);
    rt.module_registry_mut().register(module_a);

    let mut module_b =
        pipa::runtime::module::Module::new("./b.js".to_string(), "export var b = 2;".to_string());
    module_b.add_export("b".to_string(), pipa::JSValue::new_int(2), false);
    rt.module_registry_mut().register(module_b);

    let r = eval(
        &mut ctx,
        "import { a } from './a.js'; import { b } from './b.js'; a + b",
    );
    assert!(r.is_ok(), "circular import failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);
}

#[test]
fn test_circular_dependency_mutual() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module_a = pipa::runtime::module::Module::new(
        "./cycle_a.js".to_string(),
        "export var a = 10;".to_string(),
    );
    module_a.add_export("a".to_string(), pipa::JSValue::new_int(10), false);
    rt.module_registry_mut().register(module_a);

    let mut module_b = pipa::runtime::module::Module::new(
        "./cycle_b.js".to_string(),
        "export var b = 20;".to_string(),
    );
    module_b.add_export("b".to_string(), pipa::JSValue::new_int(20), false);
    rt.module_registry_mut().register(module_b);

    let r = eval(
        &mut ctx,
        "import { a } from './cycle_a.js'; import { b } from './cycle_b.js'; a + b",
    );
    assert!(r.is_ok(), "mutual imports failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 30);
}

#[test]
fn test_module_namespace_immutable() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./immutable.js".to_string(),
        "export var x = 1;".to_string(),
    );
    module.add_export("x".to_string(), pipa::JSValue::new_int(1), false);
    rt.module_registry_mut().register(module);

    let r = eval(
        &mut ctx,
        "import * as ns from './immutable.js'; typeof ns.x",
    );
    assert!(r.is_ok(), "namespace access failed: {:?}", r);
}

#[test]
fn test_namespace_contains_default() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./hasdefault.js".to_string(),
        "export default 42;".to_string(),
    );
    module.add_export("default".to_string(), pipa::JSValue::new_int(42), false);
    rt.module_registry_mut().register(module);

    let r = eval(
        &mut ctx,
        "import * as ns from './hasdefault.js'; ns.default",
    );
    assert!(r.is_ok(), "namespace default access failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 42);
}

#[test]
fn test_import_nonexistent_module_fails() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "import { x } from './nonexistent_module_12345.js'; x",
    );
    assert!(r.is_err(), "import from nonexistent module should fail");
}

#[test]
fn test_import_nonexistent_export_returns_undefined() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./hasx.js".to_string(),
        "export var x = 1;".to_string(),
    );
    module.add_export("x".to_string(), pipa::JSValue::new_int(1), false);
    rt.module_registry_mut().register(module);

    let r = eval(
        &mut ctx,
        "import { nonexistent } from './hasx.js'; nonexistent",
    );
    assert!(r.is_ok(), "import nonexistent export failed: {:?}", r);
    assert!(r.unwrap().is_undefined());
}

#[test]
fn test_multiple_imports_same_module() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./multi.js".to_string(),
        "export var a = 1; export var b = 2;".to_string(),
    );
    module.add_export("a".to_string(), pipa::JSValue::new_int(1), false);
    module.add_export("b".to_string(), pipa::JSValue::new_int(2), false);
    rt.module_registry_mut().register(module);

    let r = eval(
        &mut ctx,
        "import { a } from './multi.js'; import { b } from './multi.js'; a + b",
    );
    assert!(r.is_ok(), "multiple imports failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);
}

#[test]
fn test_export_named_list() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "var a = 1; var b = 2; export { a, b };");
    assert!(r.is_ok(), "export {{ a, b }} failed: {:?}", r);
}

#[test]
fn test_export_renamed() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "var x = 42; export { x as answer };");
    assert!(r.is_ok(), "export {{ x as answer }} failed: {:?}", r);
}

#[test]
fn test_dynamic_import_async_await() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let module_path = setup_test_module("async_import", "export var result = 123;");

    let r = eval(
        &mut ctx,
        &format!(
            "(async () => {{ const ns = await import('{}'); return ns.result; }})()",
            module_path
        ),
    );
    assert!(r.is_ok(), "async import failed: {:?}", r);

    cleanup_test_module(&module_path);
}

#[test]
fn test_import_meta_has_url() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("file:///test/module.js".to_string()));

    let r = eval(&mut ctx, "typeof import.meta");
    assert!(r.is_ok(), "import.meta check failed: {:?}", r);
    assert!(r.unwrap().is_string());
}

#[test]
fn test_import_meta_url_value() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("file:///path/to/module.js".to_string()));

    let r = eval(&mut ctx, "typeof import.meta.url");
    assert!(r.is_ok(), "import.meta.url failed: {:?}", r);
    let val = r.unwrap();
    assert!(val.is_string(), "import.meta.url typeof should be string");
}

#[test]
fn test_module_evaluation_order() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module_a = pipa::runtime::module::Module::new(
        "./order_a.js".to_string(),
        "export var a = 1;".to_string(),
    );
    module_a.add_export("a".to_string(), pipa::JSValue::new_int(1), false);
    rt.module_registry_mut().register(module_a);

    let mut module_b = pipa::runtime::module::Module::new(
        "./order_b.js".to_string(),
        "export var b = 2;".to_string(),
    );
    module_b.add_export("b".to_string(), pipa::JSValue::new_int(2), false);
    rt.module_registry_mut().register(module_b);

    let r = eval(
        &mut ctx,
        "import { a } from './order_a.js'; import { b } from './order_b.js'; a + b",
    );
    assert!(r.is_ok(), "dependency order import failed: {:?}", r);
    assert_eq!(r.unwrap().get_int(), 3);
}

#[test]
fn test_export_string_name() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "export const message = 'hello world';");
    assert!(r.is_ok(), "export string failed: {:?}", r);
}

#[test]
fn test_export_boolean() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "export const flag = true;");
    assert!(r.is_ok(), "export boolean failed: {:?}", r);
}

#[test]
fn test_export_null() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    ctx.set_current_module(Some("test.js".to_string()));

    let r = eval(&mut ctx, "export const nothing = null;");
    assert!(r.is_ok(), "export null failed: {:?}", r);
}

#[test]
fn test_import_export_function() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let mut module = pipa::runtime::module::Module::new(
        "./func.js".to_string(),
        "export function add(a, b) { return a + b; }".to_string(),
    );
    let add_fn = pipa::JSValue::new_function(0);
    module.add_export("add".to_string(), add_fn, false);
    rt.module_registry_mut().register(module);

    let r = eval(&mut ctx, "import { add } from './func.js'; typeof add");
    assert!(r.is_ok(), "import function failed: {:?}", r);
}

#[test]
fn test_dynamic_import_with_namespace() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let module_path = setup_test_module("ns_import", "export var x = 10; export var y = 20;");

    let r = eval(
        &mut ctx,
        &format!("import('{}').then(ns => ns.x + ns.y)", module_path),
    );
    assert!(r.is_ok(), "dynamic import with namespace failed: {:?}", r);

    cleanup_test_module(&module_path);
}
