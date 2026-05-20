use pipa::{JSRuntime, eval};

fn assert_js_ok(ctx: &mut pipa::JSContext, code: &str, msg: &str) {
    let r = eval(ctx, code);
    assert!(r.is_ok(), "{}: {:?}", msg, r);
}

#[test]
fn test_string_length() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "'hello'.length").is_ok());
    assert_js_ok(
        &mut ctx,
        "if ('hello'.length !== 5) throw new Error('length hello mismatch'); if (''.length !== 0) throw new Error('length empty mismatch');",
        "string length failed",
    );
}

#[test]
fn test_string_char_at() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "'hello'.charAt(1)").is_ok());
}

#[test]
fn test_string_concat() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "'hello'.concat(' ', 'world')").is_ok());
}

#[test]
fn test_string_index_of() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "'hello'.indexOf('l')").is_ok());
}

#[test]
fn test_string_to_uppercase() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "'hello'.toUpperCase()").is_ok());
}

#[test]
fn test_string_to_lowercase() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "'HELLO'.toLowerCase()").is_ok());
}

#[test]
fn test_string_substring() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "'hello'.substring(1, 4)").is_ok());
}

#[test]
fn test_string_methods() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "'hello'.charAt(0)").is_ok());
    assert!(eval(&mut ctx, "'hello'.indexOf('l')").is_ok());
    assert!(eval(&mut ctx, "'HELLO'.toLowerCase()").is_ok());
    assert!(eval(&mut ctx, "'hello'.lastIndexOf('l')").is_ok());
    assert!(eval(&mut ctx, "'hello'.split('')").is_ok());
}

#[test]
fn test_string_trim() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "'  hello  '.trim()").is_ok());
}

#[test]
fn test_string_starts_with() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "if ('hello'.startsWith('hel') !== true) throw new Error('startsWith true mismatch'); if ('hello'.startsWith('world') !== false) throw new Error('startsWith false mismatch');",
        "string startsWith failed",
    );
}

#[test]
fn test_string_ends_with() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "if ('hello'.endsWith('llo') !== true) throw new Error('endsWith true mismatch'); if ('hello'.endsWith('world') !== false) throw new Error('endsWith false mismatch');",
        "string endsWith failed",
    );
}

#[test]
fn test_string_repeat() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(&mut ctx, "'abc'.repeat(3)");
    assert!(result.is_ok());
}

#[test]
fn test_string_includes() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "if ('hello'.includes('ell') !== true) throw new Error('includes true mismatch'); if ('hello'.includes('world') !== false) throw new Error('includes false mismatch');",
        "string includes failed",
    );
}

#[test]
fn test_string_replace() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(&mut ctx, "'hello world'.replace('world', 'rust')");
    assert!(result.is_ok());
}

#[test]
fn test_string_pad_start() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(&mut ctx, "'abc'.padStart(5)");
    assert!(result.is_ok());

    let result = eval(&mut ctx, "'abc'.padStart(5, 'x')");
    assert!(result.is_ok());

    let result = eval(&mut ctx, "'abc'.padStart(10, '123')");
    assert!(result.is_ok());
}

#[test]
fn test_string_pad_end() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(&mut ctx, "'abc'.padEnd(5)");
    assert!(result.is_ok());

    let result = eval(&mut ctx, "'abc'.padEnd(5, 'x')");
    assert!(result.is_ok());

    let result = eval(&mut ctx, "'abc'.padEnd(10, '123')");
    assert!(result.is_ok());
}

#[test]
fn test_template_literal_basic() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var name = 'World'; var s = `Hello ${name}!`; if (s !== 'Hello World!') throw new Error('template basic mismatch');",
        "template literal basic failed",
    );
}

#[test]
fn test_template_literal_expression() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var x = 3; var y = 4; var s = `sum is ${x + y}`; if (s !== 'sum is 7') throw new Error('template expression mismatch');",
        "template literal expression failed",
    );
}

#[test]
fn test_template_literal_no_interpolation() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var s = `hello world`; if (s !== 'hello world') throw new Error('template plain mismatch');",
        "template literal no interpolation failed",
    );
}

#[test]
fn test_template_literal_multiple_interpolations() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var a = 1; var b = 2; var c = 3; var s = `${a} + ${b} = ${c}`; if (s !== '1 + 2 = 3') throw new Error('template multi mismatch');",
        "template literal multiple interpolations failed",
    );
}

#[test]
fn test_template_escape_sequences() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        r#"var s = `tab:\there`; if (s !== 'tab:\there') throw new Error('template escape tab mismatch: ' + s);"#,
        "template escape tab",
    );
}

#[test]
fn test_template_escape_newline() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        r#"var s = `line1\nline2`; if (s !== 'line1\nline2') throw new Error('newline mismatch: ' + s);"#,
        "template escape newline",
    );
}

#[test]
fn test_template_escape_backtick() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        r#"var s = `a\`b`; if (s !== 'a`b') throw new Error('backtick escape mismatch: ' + s);"#,
        "template escape backtick",
    );
}

#[test]
fn test_template_escape_dollar() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        r#"var s = `price: \${42}`; if (s !== 'price: ${42}') throw new Error('dollar escape mismatch: ' + s);"#,
        "template escape dollar",
    );
}

#[test]
fn test_template_multiline() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var s = `line1\nline2`; if (s.length < 10) throw new Error('multiline length');",
        "template multiline",
    );
}

#[test]
fn test_template_adjacent_interpolations() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var a = 'x'; var b = 'y'; var s = `${a}${b}`; if (s !== 'xy') throw new Error('adjacent interp mismatch: ' + s);",
        "template adjacent interpolations",
    );
}

#[test]
fn test_template_nested() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var x = 1; var s = `outer ${`inner ${x}`} end`; if (s !== 'outer inner 1 end') throw new Error('nested template mismatch: ' + s);",
        "template nested",
    );
}

#[test]
fn test_template_with_object_literal() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var obj = {a: 1}; var s = `val: ${obj.a}`; if (s !== 'val: 1') throw new Error('obj interp mismatch: ' + s);",
        "template with object member",
    );
}

#[test]
fn test_template_with_ternary() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var s = `${true ? 'yes' : 'no'}`; if (s !== 'yes') throw new Error('ternary mismatch: ' + s);",
        "template with ternary",
    );
}

#[test]
fn test_template_empty() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var s = ``; if (s !== '') throw new Error('empty template mismatch');",
        "template empty",
    );
}

#[test]
fn test_string_concatenation() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var a = 'hello' + ' ' + 'world'; if (a !== 'hello world') throw new Error('concat mismatch'); var b = 'num: ' + 42; if (b !== 'num: 42') throw new Error('concat num mismatch');",
        "string concatenation failed",
    );
}

#[test]
fn test_string_from_code_point() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "var s = String.fromCodePoint(42); if (s !== '*') throw new Error('fromCodePoint mismatch');",
        "string fromCodePoint failed",
    );
}

#[test]
fn test_string_code_point_at() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "if ('hello'.codePointAt(1) !== 101) throw new Error('cp1'); if ('hello'.codePointAt(0) !== 104) throw new Error('cp0'); if ('hello'.codePointAt(4) !== 111) throw new Error('cp4'); if ('😀'.codePointAt(0) !== 128512) throw new Error('cp emoji'); if ('hello'.codePointAt(-1) !== 111) throw new Error('cp neg'); if ('hello'.codePointAt(10) !== undefined) throw new Error('cp oob');",
        "string codePointAt failed",
    );
}

#[test]
fn test_string_match_all() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert_js_ok(
        &mut ctx,
        "if ('hello hello'.matchAll('hello').length !== 2) throw new Error('matchAll len'); if ('ab ab'.matchAll('ab')[0][0] !== 'ab') throw new Error('matchAll token'); if ('ab ab'.matchAll('ab')[0].index !== 0) throw new Error('matchAll index'); if ('test'.matchAll('t')[0].input !== 'test') throw new Error('matchAll input'); if ('hello'.matchAll('').length !== 0) throw new Error('matchAll empty');",
        "string matchAll failed",
    );
}
