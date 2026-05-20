
use pipa::{JSRuntime, eval};
use std::fs;
use std::path::Path;

fn run_js_file(path: &Path) {
    let source =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("Failed to read {:?}: {}", path, e));
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    eval(&mut ctx, &source).unwrap_or_else(|e| {
        panic!("JS file {:?} threw an error:\n{}", path, e);
    });
}

fn for_each_test_js(f: impl Fn(&Path)) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_js");
    if !dir.exists() {
        eprintln!("test_js/ directory not found, skipping semantic tests");
        return;
    }
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "js").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in &entries {
        f(&entry.path());
    }
}

macro_rules! semantic_test {
    ($name:ident, $file:expr) => {
        #[test]
        fn $name() {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("test_js")
                .join($file);
            run_js_file(&path);
        }
    };
}

semantic_test!(sem_test_basic, "test_basic.js");
semantic_test!(sem_test_simple, "test_simple.js");
semantic_test!(sem_test_script, "test_script.js");
semantic_test!(sem_test_compare, "test_compare.js");
semantic_test!(sem_test_globals, "test_globals.js");
semantic_test!(sem_test_obj_prop, "test_obj_prop.js");
semantic_test!(sem_test_import, "test_import.js");

semantic_test!(sem_test_regex_basic, "test_regex_basic.js");
semantic_test!(sem_test_regex_simple, "test_regex_simple.js");
semantic_test!(sem_test_regex_only, "test_regex_only.js");
semantic_test!(sem_test_regex_only2, "test_regex_only2.js");
semantic_test!(sem_test_regex_nocall, "test_regex_nocall.js");
semantic_test!(sem_test_regex_direct, "test_regex_direct.js");
semantic_test!(sem_test_regex_literal, "test_regex_literal.js");
semantic_test!(sem_test_regex_paren, "test_regex_paren.js");
semantic_test!(sem_test_regex_var, "test_regex_var.js");
semantic_test!(sem_test_regex_return, "test_regex_return.js");
semantic_test!(sem_test_regex_access, "test_regex_access.js");
semantic_test!(sem_test_regex_obj, "test_regex_obj.js");
semantic_test!(sem_test_regex_debug, "test_regex_debug.js");
semantic_test!(sem_test_regex_debug2, "test_regex_debug2.js");
semantic_test!(sem_test_regex_debug3, "test_regex_debug3.js");
semantic_test!(sem_test_regex_proto, "test_regex_proto.js");
semantic_test!(sem_test_regex_props, "test_regex_props.js");
semantic_test!(sem_test_regex_4, "test_regex_4.js");
semantic_test!(sem_test_regex_5, "test_regex_5.js");
semantic_test!(sem_test_regex_6, "test_regex_6.js");

semantic_test!(sem_test_unicode, "test_unicode.js");
semantic_test!(sem_test_unicode2, "test_unicode2.js");
semantic_test!(sem_test_unicode3, "test_unicode3.js");

#[test]
fn all_test_js_files_pass() {
    for_each_test_js(|path| run_js_file(path));
}

fn js(code: &str) {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    eval(&mut ctx, code).unwrap_or_else(|e| panic!("JS error: {}", e));
}

fn js_eq(code: &str) {
    let full = format!(
        r#"
        function __assert_eq(a, b, msg) {{
            if (a !== b) throw new Error(msg + ': expected=' + b + ' got=' + a);
        }}
        {}
        "#,
        code
    );
    js(&full);
}

#[test]
fn sem_arithmetic_basic() {
    js_eq("__assert_eq(1 + 2, 3, 'add'); __assert_eq(10 - 3, 7, 'sub');");
}

#[test]
fn sem_arithmetic_precedence() {
    js_eq("__assert_eq(2 + 3 * 4, 14, 'precedence');");
}

#[test]
fn sem_arithmetic_div_mod() {
    js_eq("__assert_eq(10 / 5, 2, 'int div'); __assert_eq(10 % 3, 1, 'mod');");
}

#[test]
fn sem_arithmetic_pow() {
    js_eq("__assert_eq(2 ** 10, 1024, 'pow');");
}

#[test]
fn sem_arithmetic_negation() {
    js_eq("var x = 5; __assert_eq(-x, -5, 'negate');");
}

#[test]
fn sem_var_declarations() {
    js_eq("var x = 1; let y = 2; const z = 3; __assert_eq(x + y + z, 6, 'var+let+const');");
}

#[test]
fn sem_var_shadowing() {
    
    js_eq("var x = 1; var x = 2; __assert_eq(x, 2, 'var redeclared');");
}

#[test]
fn sem_if_else() {
    js_eq("var x = 5 > 3 ? 10 : 20; __assert_eq(x, 10, 'ternary');");
}

#[test]
fn sem_for_loop() {
    js_eq("var s = 0; for (var i = 0; i < 10; i++) s += i; __assert_eq(s, 45, 'for sum');");
}

#[test]
fn sem_while_loop() {
    js_eq("var n = 0; while (n < 5) n++; __assert_eq(n, 5, 'while');");
}

#[test]
fn sem_do_while() {
    js_eq("var n = 0; do { n++; } while (n < 3); __assert_eq(n, 3, 'do-while');");
}

#[test]
fn sem_for_in() {
    
    js_eq(
        "var count = 0; var obj = {a:1,b:2,c:3}; for (var k in obj) count++; __assert_eq(count, 3, 'for-in');",
    );
}

#[test]
fn sem_for_of() {
    js_eq("var s = 0; for (var v of [1,2,3]) s += v; __assert_eq(s, 6, 'for-of');");
}

#[test]
fn sem_break_continue() {
    js_eq(
        r#"
        var s = 0;
        for (var i = 0; i < 10; i++) {
            if (i === 3) continue;
            if (i === 7) break;
            s += i;
        }
        __assert_eq(s, 0+1+2+4+5+6, 'break+continue');
    "#,
    );
}

#[test]
fn sem_switch() {
    js_eq(
        r#"
        var x = 0;
        switch (2) {
            case 1: x = 10; break;
            case 2: x = 20; break;
            default: x = 30;
        }
        __assert_eq(x, 20, 'switch');
    "#,
    );
}

#[test]
fn sem_function_declaration() {
    js_eq("function add(a, b) { return a + b; } __assert_eq(add(3, 4), 7, 'func decl');");
}

#[test]
fn sem_function_expression() {
    
    js_eq("var add = function(a, b) { return a + b; }; __assert_eq(add(3, 4), 7, 'func expr');");
}

#[test]
fn sem_closure() {
    js_eq(
        r#"
        function make() { var x = 42; return function() { return x; }; }
        __assert_eq(make()(), 42, 'closure');
    "#,
    );
}

#[test]
fn sem_default_params() {
    js_eq("function f(a, b = 10) { return a + b; } __assert_eq(f(1), 11, 'default param');");
}

#[test]
fn sem_rest_params() {
    js_eq("function f(...args) { return args.length; } __assert_eq(f(1,2,3), 3, 'rest params');");
}

#[test]
fn sem_recursive() {
    js_eq(
        "function fib(n) { return n <= 1 ? n : fib(n-1) + fib(n-2); } __assert_eq(fib(10), 55, 'fib');",
    );
}

#[test]
fn sem_object_literal() {
    js_eq("var o = {x: 1, y: 2}; __assert_eq(o.x + o.y, 3, 'obj literal');");
}

#[test]
fn sem_object_computed_key() {
    js_eq("var k = 'hello'; var o = {[k]: 42}; __assert_eq(o.hello, 42, 'computed key');");
}

#[test]
fn sem_object_method() {
    
    js_eq(
        "var o = { double: function(x) { return x * 2; } }; __assert_eq(o.double(21), 42, 'obj method');",
    );
}

#[test]
fn sem_array_basic() {
    js_eq(
        "var a = [1, 2, 3]; __assert_eq(a.length, 3, 'arr len'); __assert_eq(a[1], 2, 'arr idx');",
    );
}

#[test]
fn sem_array_methods() {
    js_eq(
        r#"
        var a = [3, 1, 2];
        a.sort();
        __assert_eq(a[0], 1, 'sort');
        __assert_eq(a.indexOf(2), 1, 'indexOf');
    "#,
    );
}

#[test]
fn sem_array_push_pop() {
    js_eq(
        r#"
        var a = [];
        a.push(10);
        a.push(20);
        __assert_eq(a.length, 2, 'push');
        var v = a.pop();
        __assert_eq(v, 20, 'pop');
    "#,
    );
}

#[test]
fn sem_class_basic() {
    js_eq(
        r#"
        class Point {
            constructor(x, y) { this.x = x; this.y = y; }
            sum() { return this.x + this.y; }
        }
        var p = new Point(3, 4);
        __assert_eq(p.sum(), 7, 'class');
    "#,
    );
}

#[test]
fn sem_class_inheritance() {
    js_eq(
        r#"
        class Animal {
            constructor(name) { this.name = name; }
            speak() { return this.name; }
        }
        class Dog extends Animal {
            speak() { return this.name + ' barks'; }
        }
        var d = new Dog('Rex');
        __assert_eq(d.speak(), 'Rex barks', 'extends');
    "#,
    );
}

#[test]
fn sem_array_destructuring() {
    js_eq("var [a, b] = [10, 20]; __assert_eq(a + b, 30, 'arr destr');");
}

#[test]
fn sem_object_destructuring() {
    js_eq("var {x, y} = {x: 1, y: 2}; __assert_eq(x + y, 3, 'obj destr');");
}

#[test]
fn sem_template_literal() {
    js_eq("var name = 'World'; __assert_eq(`Hello ${name}!`, 'Hello World!', 'template');");
}

#[test]
fn sem_spread_array() {
    js_eq("var a = [1, 2]; var b = [...a, 3]; __assert_eq(b.length, 3, 'spread');");
}

#[test]
fn sem_try_catch() {
    js_eq(
        r#"
        var caught = false;
        try { throw new Error('test'); }
        catch (e) { caught = true; }
        __assert_eq(caught, true, 'try-catch');
    "#,
    );
}

#[test]
fn sem_try_finally() {
    js_eq(
        r#"
        var cleaned = false;
        try { var x = 1; }
        finally { cleaned = true; }
        __assert_eq(cleaned, true, 'finally');
    "#,
    );
}

#[test]
fn sem_nullish_coalescing() {
    js_eq("__assert_eq(null ?? 42, 42, '?? null'); __assert_eq(0 ?? 99, 0, '?? 0');");
}

#[test]
fn sem_optional_chaining() {
    js_eq(
        "var o = {a: {b: 1}}; __assert_eq(o?.a?.b, 1, '?.'); __assert_eq(o?.c?.d, undefined, '?. miss');",
    );
}

#[test]
fn sem_logical_assignment() {
    js_eq(
        r#"
        var a = null; a ??= 10; __assert_eq(a, 10, '??=');
        var b = 0; b ||= 5; __assert_eq(b, 5, '||=');
        var c = 1; c &&= 2; __assert_eq(c, 2, '&&=');
    "#,
    );
}

#[test]
fn sem_typeof() {
    js_eq(
        r#"
        __assert_eq(typeof 42, 'number', 'typeof num');
        __assert_eq(typeof 'hi', 'string', 'typeof str');
        __assert_eq(typeof undefined, 'undefined', 'typeof undef');
        __assert_eq(typeof null, 'object', 'typeof null');
    "#,
    );
}

#[test]
fn sem_instanceof() {
    js_eq(
        r#"
        function Foo() {}
        var f = new Foo();
        __assert_eq(f instanceof Foo, true, 'instanceof');
    "#,
    );
}

#[test]
fn sem_string_methods() {
    js_eq(
        r#"
        __assert_eq('hello'.toUpperCase(), 'HELLO', 'toUpperCase');
        __assert_eq('abc'.split('').length, 3, 'split');
    "#,
    );
}

#[test]
fn sem_json() {
    js_eq(
        r#"
        var obj = JSON.parse('{"x":1}');
        __assert_eq(obj.x, 1, 'JSON.parse');
        __assert_eq(JSON.stringify({a:1}), '{"a":1}', 'JSON.stringify');
    "#,
    );
}

#[test]
fn sem_math() {
    js_eq(
        r#"
        __assert_eq(Math.max(1, 3, 2), 3, 'max');
        __assert_eq(Math.min(1, 3, 2), 1, 'min');
        __assert_eq(Math.abs(-7), 7, 'abs');
    "#,
    );
}
