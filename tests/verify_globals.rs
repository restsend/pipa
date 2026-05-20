
use pipa::{JSRuntime, eval};

#[test]
fn test_global_var_returns_correct_value() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result1 = eval(&mut ctx, "this.x = 99; this.x");
    println!("Direct property access: {:?}", result1);

    let result = eval(&mut ctx, "var x = 42; x");
    println!("Var declaration result: {:?}", result);
    assert!(result.is_ok(), "Should not error: {:?}", result);

    let val = result.unwrap();
    if !val.is_int() {
        println!("Value is not int, raw: {:?}", val);
    }
    assert!(val.is_int(), "Should be int, got undefined or other type");
    assert_eq!(val.get_int(), 42, "Should be 42");
}

#[test]
fn test_global_var_modify_and_return() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "var x = 10; x = x + 5; x");
    assert!(result.is_ok(), "Should not error: {:?}", result);

    let val = result.unwrap();
    assert!(val.is_int(), "Should be int");
    assert_eq!(val.get_int(), 15, "Should be 15");
}
