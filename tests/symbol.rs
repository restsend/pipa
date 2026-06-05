use pipa::{JSRuntime, eval};

#[test]
fn test_symbol_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "typeof Symbol()");
    assert!(r.is_ok(), "typeof Symbol() failed: {:?}", r);
}

#[test]
fn test_symbol_for() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "Symbol.for('foo')");
    assert!(r.is_ok(), "Symbol.for failed: {:?}", r);
}

#[test]
fn test_symbol_description() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        "var a = Symbol('test').description; if (a !== 'test') throw new Error('description mismatch'); var b = Symbol().description; if (b !== undefined) throw new Error('undefined description mismatch'); var c = Symbol('').description; if (c !== '') throw new Error('empty description mismatch');",
    );
    assert!(r.is_ok(), "Symbol.description failed: {:?}", r);
}
