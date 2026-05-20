
use pipa::{JSRuntime, eval_via_ast, parse_to_ast};

#[test]
fn parse_empty_program() {
    let ast = parse_to_ast("").unwrap();
    assert!(ast.body.is_empty());
}

#[test]
fn parse_var_declaration() {
    let ast = parse_to_ast("var x = 42;").unwrap();
    assert_eq!(ast.body.len(), 1);
}

#[test]
fn parse_let_const() {
    parse_to_ast("let a = 1; const b = 2;").unwrap();
}

#[test]
fn parse_function_declaration() {
    parse_to_ast("function add(a, b) { return a + b; }").unwrap();
}

#[test]
fn parse_if_else() {
    parse_to_ast("if (true) { 1; } else { 2; }").unwrap();
}

#[test]
fn parse_for_loop() {
    parse_to_ast("for (var i = 0; i < 10; i++) { var x = i; }").unwrap();
}

#[test]
fn parse_while_loop() {
    parse_to_ast("while (true) { break; }").unwrap();
}

#[test]
fn parse_do_while() {
    parse_to_ast("do { x++; } while (x < 10);").unwrap();
}

#[test]
fn parse_for_in() {
    parse_to_ast("for (var k in obj) { count++; }").unwrap();
}

#[test]

fn parse_for_of() {
    parse_to_ast("for (var v of arr) { sum += v; }").unwrap();
}

#[test]
fn parse_switch() {
    parse_to_ast(
        r#"
        switch (x) {
            case 1: break;
            case 2: break;
            default: break;
        }
    "#,
    )
    .unwrap();
}

#[test]
fn parse_try_catch() {
    parse_to_ast("try { throw 1; } catch (e) { var x = e; }").unwrap();
}

#[test]
fn parse_try_finally() {
    parse_to_ast("try { 1; } finally { 2; }").unwrap();
}

#[test]
fn parse_try_catch_finally() {
    parse_to_ast("try { 1; } catch (e) { 2; } finally { 3; }").unwrap();
}

#[test]
fn parse_class_basic() {
    parse_to_ast(
        r#"
        class Point {
            constructor(x, y) { this.x = x; this.y = y; }
            sum() { return this.x + this.y; }
        }
    "#,
    )
    .unwrap();
}

#[test]
fn parse_class_extends() {
    parse_to_ast(
        r#"
        class Base {}
        class Child extends Base {}
    "#,
    )
    .unwrap();
}

#[test]
fn parse_array_destructuring() {
    parse_to_ast("var [a, b] = [1, 2];").unwrap();
}

#[test]
fn parse_object_destructuring() {
    parse_to_ast("var {x, y} = {x: 1, y: 2};").unwrap();
}

#[test]
fn parse_template_literal() {
    parse_to_ast("var s = `Hello ${name}!`;").unwrap();
}

#[test]
fn parse_spread() {
    parse_to_ast("var b = [...a, 3];").unwrap();
}

#[test]
fn parse_arrow_function() {
    parse_to_ast("var f = x => x * 2;").unwrap();
}

#[test]
fn parse_arrow_with_parens() {
    parse_to_ast("var f = (a, b) => a + b;").unwrap();
}

#[test]

fn parse_default_params() {
    parse_to_ast("function f(a, b = 10) { return a + b; }").unwrap();
}

#[test]

fn parse_rest_params() {
    parse_to_ast("function f(...args) { return args; }").unwrap();
}

#[test]
fn parse_nullish_coalescing() {
    parse_to_ast("var x = null ?? 42;").unwrap();
}

#[test]

fn parse_optional_chaining() {
    parse_to_ast("var x = o?.a?.b;").unwrap();
}

#[test]
fn parse_logical_assignment() {
    parse_to_ast("var a = null; a ??= 10; b ||= 5; c &&= 2;").unwrap();
}

#[test]
fn parse_compound_assignment() {
    parse_to_ast("var x = 1; x += 2; x -= 1; x *= 3; x /= 2;").unwrap();
}

#[test]
fn parse_bitwise_ops() {
    parse_to_ast("var x = 1 & 2 | 3 ^ 4; x = ~x; x = x << 1; x = x >> 1; x = x >>> 1;").unwrap();
}

#[test]
fn parse_typeof_instanceof() {
    parse_to_ast("typeof 42; o instanceof Object; 'x' in obj;").unwrap();
}

#[test]

fn parse_new_expression() {
    parse_to_ast("function F() {} var f = new F();").unwrap();
}

#[test]
fn parse_regex() {
    parse_to_ast("var r = /test/gi;").unwrap();
}

#[test]
fn parse_async_function() {
    parse_to_ast("async function f() { return await 1; }").unwrap();
}

#[test]

fn parse_generator() {
    parse_to_ast("function* g() { yield 1; yield* gen; }").unwrap();
}

#[test]

fn parse_import() {
    parse_to_ast("import { x, y as z } from 'module';").unwrap();
}

#[test]

fn parse_import_default() {
    parse_to_ast("import foo from 'module';").unwrap();
}

#[test]

fn parse_import_namespace() {
    parse_to_ast("import * as ns from 'module';").unwrap();
}

#[test]
fn parse_export() {
    parse_to_ast("export var x = 1;").unwrap();
}

#[test]
fn parse_export_function() {
    parse_to_ast("export function f() {}").unwrap();
}

#[test]
fn parse_export_class() {
    parse_to_ast("export class Foo {}").unwrap();
}

#[test]

fn parse_export_default() {
    parse_to_ast("export default function() {}").unwrap();
}

#[test]

fn parse_export_list() {
    parse_to_ast("export { a, b as c };").unwrap();
}

#[test]

fn parse_export_all() {
    parse_to_ast("export * from 'module';").unwrap();
}

#[test]

fn parse_complex_expression() {
    parse_to_ast("var x = (1 + 2) * 3 - 4 / 2 ** 3 % 2;").unwrap();
}

#[test]
fn parse_object_literal() {
    parse_to_ast("var o = {a: 1, b: 'hello', [key]: value, ...spread};").unwrap();
}

#[test]
fn parse_array_literal() {
    parse_to_ast("var a = [1, 'two', true, null, undefined];").unwrap();
}

#[test]
fn parse_labelled_statement() {
    parse_to_ast("outer: for (var i = 0; i < 10; i++) { break outer; }").unwrap();
}

#[test]
fn parse_with_statement() {
    parse_to_ast("with (obj) { x = 1; }").unwrap();
}

const ASSERT_LIB: &str = r#"
function __assert_eq(a, b, msg) {
    if (a !== b) throw new Error(msg + ': expected=' + b + ' got=' + a);
}
function __assert_true(v, msg) {
    if (!v) throw new Error(msg + ': value was falsy');
}
"#;

fn js_ast(code: &str) {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    let src = format!("{}\n{}", ASSERT_LIB, code);
    eval_via_ast(&mut ctx, &src).expect("JS threw an error");
}

#[test]
fn ast_eval_arithmetic() {
    js_ast("__assert_eq(1 + 2 * 3, 7, 'add precedence');");
}

#[test]
fn ast_eval_variable() {
    js_ast("var x = 42; __assert_eq(x, 42, 'var assign');");
}

#[test]
fn ast_eval_function() {
    js_ast("function add(a, b) { return a + b; } __assert_eq(add(3, 4), 7, 'func call');");
}

#[test]
fn ast_eval_closure() {
    js_ast(
        r#"
        function make(x) {
            return function() { return x; };
        }
        __assert_eq(make(42)(), 42, 'closure');
    "#,
    );
}

#[test]
fn ast_eval_for_loop() {
    js_ast("var s = 0; for (var i = 0; i < 10; i++) s += i; __assert_eq(s, 45, 'for sum');");
}

#[test]
fn ast_eval_if_else() {
    js_ast("var x = 5 > 3 ? 10 : 20; __assert_eq(x, 10, 'ternary');");
}

#[test]
fn ast_eval_object() {
    js_ast("var o = {x: 1, y: 2}; __assert_eq(o.x + o.y, 3, 'obj prop');");
}

#[test]
fn ast_eval_try_catch() {
    js_ast(
        r#"
        var caught = false;
        try { throw new Error('test'); }
        catch (e) { caught = true; }
        __assert_true(caught, 'try-catch');
    "#,
    );
}

#[test]
fn ast_eval_string_concat() {
    js_ast("__assert_eq('hello' + ' ' + 'world', 'hello world', 'concat');");
}
