use pipa::{JSRuntime, eval};

fn js_eq(code: &str) {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    eval(&mut ctx, code).unwrap_or_else(|e| panic!("JS error: {}", e));
}

#[test]
fn json_parse_null() {
    js_eq("if (JSON.parse('null') !== null) throw new Error('fail');");
}

#[test]
fn json_parse_true() {
    js_eq("if (JSON.parse('true') !== true) throw new Error('fail');");
}

#[test]
fn json_parse_false() {
    js_eq("if (JSON.parse('false') !== false) throw new Error('fail');");
}

#[test]
fn json_parse_integer() {
    js_eq("if (JSON.parse('42') !== 42) throw new Error('fail');");
}

#[test]
fn json_parse_negative_integer() {
    js_eq("if (JSON.parse('-7') !== -7) throw new Error('fail');");
}

#[test]
fn json_parse_float() {
    js_eq("if (JSON.parse('3.14') !== 3.14) throw new Error('fail');");
}

#[test]
fn json_parse_exponent() {
    js_eq("if (JSON.parse('1e2') !== 100) throw new Error('fail');");
    js_eq("if (JSON.parse('1E+3') !== 1000) throw new Error('fail');");
    js_eq("if (JSON.parse('2.5e-1') !== 0.25) throw new Error('fail');");
}

#[test]
fn json_parse_string() {
    js_eq("if (JSON.parse('\"hello\"') !== 'hello') throw new Error('fail');");
}

#[test]
fn json_parse_empty_string() {
    js_eq("if (JSON.parse('\"\"') !== '') throw new Error('fail');");
}

#[test]
fn json_parse_string_escapes() {
    js_eq("if (JSON.parse('\"\\\\n\"') !== '\\n') throw new Error('newline');");

    js_eq("if (JSON.parse('\"\\\\t\"') !== '\\t') throw new Error('tab');");

    js_eq("if (JSON.parse('\"\\\\\\\\\"') !== '\\\\') throw new Error('backslash');");

    js_eq("if (JSON.parse('\"\\\\\"\"') !== '\"') throw new Error('quote');");

    js_eq("if (JSON.parse('\"\\\\/\"') !== '/') throw new Error('slash');");

    js_eq("if (JSON.parse('\"\\\\r\"') !== '\\r') throw new Error('cr');");

    js_eq("if (JSON.parse('\"\\\\b\"').charCodeAt(0) !== 8) throw new Error('bs');");

    js_eq("if (JSON.parse('\"\\\\f\"').charCodeAt(0) !== 12) throw new Error('ff');");
}

#[test]
fn json_parse_string_unicode_escape() {
    js_eq("if (JSON.parse('\"\\\\u0041\"') !== 'A') throw new Error('fail');");
}

#[test]
fn json_parse_empty_array() {
    js_eq("if (JSON.parse('[]').length !== 0) throw new Error('fail');");
}

#[test]
fn json_parse_simple_array() {
    js_eq(
        "var a = JSON.parse('[1,2,3]'); \
         if (a.length !== 3 || a[0] !== 1 || a[2] !== 3) throw new Error('fail');",
    );
}

#[test]
fn json_parse_mixed_array() {
    js_eq(
        "var a = JSON.parse('[1,\"two\",true,null]'); \
         if (a.length !== 4 || a[0] !== 1 || a[1] !== 'two' || a[2] !== true || a[3] !== null) throw new Error('fail');",
    );
}

#[test]
fn json_parse_nested_array() {
    js_eq(
        "var a = JSON.parse('[[1,2],[3,4]]'); \
         if (a.length !== 2 || a[0][0] !== 1 || a[1][1] !== 4) throw new Error('fail');",
    );
}

#[test]
fn json_parse_empty_object() {
    js_eq("var o = JSON.parse('{}'); if (typeof o !== 'object') throw new Error('fail');");
}

#[test]
fn json_parse_simple_object() {
    js_eq(
        "var o = JSON.parse('{\"a\":1,\"b\":2}'); \
         if (o.a !== 1 || o.b !== 2) throw new Error('fail');",
    );
}

#[test]
fn json_parse_nested_object() {
    js_eq(
        "var o = JSON.parse('{\"outer\":{\"inner\":42}}'); \
         if (o.outer.inner !== 42) throw new Error('fail');",
    );
}

#[test]
fn json_parse_complex() {
    js_eq(
        "var o = JSON.parse('{\"name\":\"test\",\"values\":[1,2,3],\"nested\":{\"key\":\"val\"}}'); \
         if (o.name !== 'test') throw new Error('name'); \
         if (o.values.length !== 3) throw new Error('values'); \
         if (o.nested.key !== 'val') throw new Error('nested');",
    );
}

#[test]
fn json_parse_whitespace() {
    js_eq("if (JSON.parse('  42  ') !== 42) throw new Error('fail');");
    js_eq("var a = JSON.parse(' [ 1 , 2 ] '); if (a.length !== 2) throw new Error('fail');");
}

#[test]
fn json_parse_invalid_throws() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    let r = eval(&mut ctx, "JSON.parse('invalid')");
    assert!(r.is_err(), "expected SyntaxError for invalid JSON");
}

#[test]
fn json_stringify_null() {
    js_eq("if (JSON.stringify(null) !== 'null') throw new Error('fail');");
}

#[test]
fn json_stringify_bool() {
    js_eq("if (JSON.stringify(true) !== 'true') throw new Error('fail');");
    js_eq("if (JSON.stringify(false) !== 'false') throw new Error('fail');");
}

#[test]
fn json_stringify_number() {
    js_eq("if (JSON.stringify(42) !== '42') throw new Error('fail');");
    js_eq("if (JSON.stringify(3.14) !== '3.14') throw new Error('fail');");
}

#[test]
fn json_stringify_nan_null() {
    js_eq("if (JSON.stringify(NaN) !== 'null') throw new Error('fail');");
    js_eq("if (JSON.stringify(Infinity) !== 'null') throw new Error('fail');");
}

#[test]
fn json_stringify_string() {
    js_eq("if (JSON.stringify('hello') !== '\"hello\"') throw new Error('fail');");
}

#[test]
fn json_stringify_string_escapes() {
    js_eq("if (JSON.stringify('a\\nb') !== '\"a\\\\nb\"') throw new Error('fail');");
}

#[test]
fn json_stringify_undefined() {
    js_eq("var r = JSON.stringify(undefined); if (r !== undefined) throw new Error('fail');");
}

#[test]
fn json_stringify_empty_array() {
    js_eq("if (JSON.stringify([]) !== '[]') throw new Error('fail');");
}

#[test]
fn json_stringify_array() {
    js_eq("if (JSON.stringify([1,2,3]) !== '[1,2,3]') throw new Error('fail');");
}

#[test]
fn json_stringify_array_with_null() {
    js_eq("if (JSON.stringify([1,null,3]) !== '[1,null,3]') throw new Error('fail');");
}

#[test]
fn json_stringify_empty_object() {
    js_eq("if (JSON.stringify({}) !== '{}') throw new Error('fail');");
}

#[test]
fn json_stringify_simple_object() {
    js_eq("if (JSON.stringify({a:1}) !== '{\"a\":1}') throw new Error('fail');");
}

#[test]
fn json_stringify_nested() {
    js_eq(
        "var s = JSON.stringify({a:{b:1}}); \
         if (s !== '{\"a\":{\"b\":1}}') throw new Error('got: ' + s);",
    );
}

#[test]
fn json_stringify_skips_undefined() {
    js_eq(
        "var s = JSON.stringify({a:1,b:undefined}); \
         if (s !== '{\"a\":1}') throw new Error('got: ' + s);",
    );
}

#[test]
fn json_stringify_skips_functions() {
    js_eq(
        "var s = JSON.stringify({a:1,fn:function(){}}); \
         if (s !== '{\"a\":1}') throw new Error('got: ' + s);",
    );
}

#[test]
fn json_stringify_circular() {
    js_eq(
        "var obj = {a:1}; obj.self = obj; \
         try { JSON.stringify(obj); throw new Error('fail'); } catch(e) { if (e.message === 'fail') throw e; }",
    );
}

#[test]
fn json_stringify_space_number() {
    js_eq(
        "var s = JSON.stringify({a:1}, null, 2); \
         if (s.indexOf('\\n') < 0) throw new Error('expected newlines: ' + s);",
    );
}

#[test]
fn json_stringify_space_string() {
    js_eq(
        "var s = JSON.stringify([1], null, '\\t'); \
         if (s.indexOf('\\t') < 0) throw new Error('expected tabs: ' + s);",
    );
}

#[test]
fn json_roundtrip_object() {
    js_eq(
        "var obj = {x:1, y:'hello', z:true, w:null}; \
         var s = JSON.stringify(obj); \
         var parsed = JSON.parse(s); \
         if (parsed.x !== 1 || parsed.y !== 'hello' || parsed.z !== true || parsed.w !== null) throw new Error('roundtrip fail');",
    );
}

#[test]
fn json_roundtrip_nested() {
    js_eq(
        "var obj = {arr:[1,2,3], obj:{nested:true}}; \
         var s = JSON.stringify(obj); \
         var parsed = JSON.parse(s); \
         if (parsed.arr.length !== 3 || parsed.obj.nested !== true) throw new Error('roundtrip fail');",
    );
}

#[test]
fn json_roundtrip_large_number() {
    js_eq(
        "var n = JSON.parse('9999999999'); \
         if (n !== 9999999999) throw new Error('got: ' + n);",
    );
}
