use pipa::{JSRuntime, eval};

fn assert_js_ok(ctx: &mut pipa::JSContext, code: &str, msg: &str) {
    let r = eval(ctx, code);
    assert!(r.is_ok(), "{}: {:?}", msg, r);
}

fn assert_bigint_eq(ctx: &mut pipa::JSContext, expr: &str, expected: i64) {
    let code = format!(
        "var __v = ({}); if (typeof __v !== 'bigint') throw new Error('not bigint'); if (__v !== ({}n)) throw new Error('bigint mismatch');",
        expr, expected
    );
    assert_js_ok(ctx, &code, "bigint assertion failed");
}

fn assert_bool_eq(ctx: &mut pipa::JSContext, expr: &str, expected: bool) {
    let expected_js = if expected { "true" } else { "false" };
    let code = format!(
        "var __v = ({}); if (__v !== {}) throw new Error('bool mismatch');",
        expr, expected_js
    );
    assert_js_ok(ctx, &code, "bool assertion failed");
}

#[test]
fn test_bigint_literal() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "123n", 123);
}

#[test]
fn test_bigint_zero() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "0n", 0);
}

#[test]
fn test_bigint_large() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "9007199254740993n", 9007199254740993i64);
}

#[test]
fn test_bigint_negative() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "-42n", -42);
}

#[test]
fn test_bigint_constructor_from_int() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "BigInt(123)", 123);
}

#[test]
fn test_bigint_constructor_from_string() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "BigInt('456')", 456);
}

#[test]
fn test_bigint_constructor_from_bigint() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "BigInt(789n)", 789);
}

#[test]
fn test_bigint_in_variable() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_js_ok(
        &mut ctx,
        "var x = 100n; if (typeof x !== 'bigint' || x !== 100n) throw new Error('var bigint mismatch');",
        "bigint in variable failed",
    );
}

#[test]
fn test_bigint_in_object() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_js_ok(
        &mut ctx,
        "var obj = {val: 999n}; if (typeof obj.val !== 'bigint' || obj.val !== 999n) throw new Error('object bigint mismatch');",
        "bigint in object failed",
    );
}

#[test]
fn test_bigint_in_array() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_js_ok(
        &mut ctx,
        "var arr = [1n, 2n, 3n]; if (typeof arr[1] !== 'bigint' || arr[1] !== 2n) throw new Error('array bigint mismatch');",
        "bigint in array failed",
    );
}

#[test]
fn test_bigint_add() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "10n + 20n", 30);
    assert_bigint_eq(&mut ctx, "100n + 200n", 300);
    assert_bigint_eq(&mut ctx, "0n + 0n", 0);
}

#[test]
fn test_bigint_sub() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "30n - 10n", 20);
    assert_bigint_eq(&mut ctx, "100n - 50n", 50);
}

#[test]
fn test_bigint_mul() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "6n * 7n", 42);
    assert_bigint_eq(&mut ctx, "10n * 10n", 100);
}

#[test]
fn test_bigint_div() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "42n / 6n", 7);
    assert_bigint_eq(&mut ctx, "100n / 10n", 10);
}

#[test]
fn test_bigint_mod() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "17n % 5n", 2);
    assert_bigint_eq(&mut ctx, "10n % 3n", 1);
}

#[test]
fn test_bigint_eq() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bool_eq(&mut ctx, "5n == 5n", true);
    assert_bool_eq(&mut ctx, "5n == 3n", false);
}

#[test]
fn test_bigint_neq() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bool_eq(&mut ctx, "5n != 3n", true);
    assert_bool_eq(&mut ctx, "5n != 5n", false);
}

#[test]
fn test_bigint_lt() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bool_eq(&mut ctx, "3n < 5n", true);
    assert_bool_eq(&mut ctx, "5n < 3n", false);
    assert_bool_eq(&mut ctx, "5n < 5n", false);
}

#[test]
fn test_bigint_lte() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bool_eq(&mut ctx, "3n <= 5n", true);
    assert_bool_eq(&mut ctx, "5n <= 5n", true);
    assert_bool_eq(&mut ctx, "5n <= 3n", false);
}

#[test]
fn test_bigint_gt() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bool_eq(&mut ctx, "7n > 5n", true);
    assert_bool_eq(&mut ctx, "3n > 5n", false);
    assert_bool_eq(&mut ctx, "5n > 5n", false);
}

#[test]
fn test_bigint_gte() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bool_eq(&mut ctx, "7n >= 5n", true);
    assert_bool_eq(&mut ctx, "5n >= 5n", true);
    assert_bool_eq(&mut ctx, "3n >= 5n", false);
}

#[test]
fn test_bigint_strict_eq() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bool_eq(&mut ctx, "5n === 5n", true);
    assert_bool_eq(&mut ctx, "5n === 3n", false);
}

#[test]
fn test_bigint_strict_neq() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bool_eq(&mut ctx, "5n !== 3n", true);
    assert_bool_eq(&mut ctx, "5n !== 5n", false);
}

#[test]
fn test_bigint_bitand() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "15n & 10n", 10);
    assert_bigint_eq(&mut ctx, "255n & 15n", 15);
    assert_bigint_eq(&mut ctx, "100n & 0n", 0);
}

#[test]
fn test_bigint_bitor() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "10n | 5n", 15);
    assert_bigint_eq(&mut ctx, "240n | 15n", 255);
    assert_bigint_eq(&mut ctx, "0n | 100n", 100);
}

#[test]
fn test_bigint_bitxor() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "15n ^ 10n", 5);
    assert_bigint_eq(&mut ctx, "255n ^ 15n", 240);
    assert_bigint_eq(&mut ctx, "100n ^ 100n", 0);
}

#[test]
fn test_bigint_bitnot() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "~0n", -1);
    assert_bigint_eq(&mut ctx, "~(-1n)", 0);
    assert_bigint_eq(&mut ctx, "~15n", -16);
}

#[test]
fn test_bigint_shl() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "1n << 4n", 16);
    assert_bigint_eq(&mut ctx, "7n << 3n", 56);
    assert_bigint_eq(&mut ctx, "1n << 0n", 1);
}

#[test]
fn test_bigint_shr() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "16n >> 2n", 4);
    assert_bigint_eq(&mut ctx, "56n >> 3n", 7);
    assert_bigint_eq(&mut ctx, "-8n >> 1n", -4);
}

#[test]
fn test_bigint_as_int_n() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "BigInt.asIntN(8, 255n)", -1);
    assert_bigint_eq(&mut ctx, "BigInt.asIntN(8, 128n)", -128);
    assert_bigint_eq(&mut ctx, "BigInt.asIntN(8, 127n)", 127);
    assert_bigint_eq(&mut ctx, "BigInt.asIntN(4, 25n)", -7);
}

#[test]
fn test_bigint_as_uint_n() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "BigInt.asUintN(8, -1n)", 255);
    assert_bigint_eq(&mut ctx, "BigInt.asUintN(8, 256n)", 0);
    assert_bigint_eq(&mut ctx, "BigInt.asUintN(8, 255n)", 255);
    assert_bigint_eq(&mut ctx, "BigInt.asUintN(4, -1n)", 15);
}

#[test]
fn test_bigint_as_int_n_zero_bits() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "BigInt.asIntN(0, 123n)", 0);
}

#[test]
fn test_bigint_as_uint_n_zero_bits() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    assert_bigint_eq(&mut ctx, "BigInt.asUintN(0, 123n)", 0);
}
