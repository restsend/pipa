use pipa::{JSRuntime, eval};

#[test]
fn debug_nested_var_resolution() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let r1 = eval(&mut ctx, "var x = 42; function f() { return x; } f();");
    if r1.is_ok() && r1.as_ref().unwrap().is_int() {
        println!("Test 1 (var before func): {}", r1.unwrap().get_int());
    } else {
        println!("Test 1 (var before func): {:?}", r1);
    }

    let r2 = eval(&mut ctx, "function f() { return x; } var x = 42; f();");
    if r2.is_ok() && r2.as_ref().unwrap().is_int() {
        println!("Test 2 (var after func): {}", r2.unwrap().get_int());
    } else {
        println!("Test 2 (var after func): {:?}", r2);
    }

    let r3 = eval(
        &mut ctx,
        "function outer() { var x = 42; function inner() { return x; } return inner(); } outer();",
    );
    if r3.is_ok() && r3.as_ref().unwrap().is_int() {
        println!("Test 3 (func in func): {}", r3.unwrap().get_int());
    } else {
        println!("Test 3 (func in func): {:?}", r3);
    }

    let r4 = eval(
        &mut ctx,
        "var STATE = 1; function Task() { this.state = STATE; } new Task().state;",
    );
    if r4.is_ok() && r4.as_ref().unwrap().is_int() {
        println!("Test 4 (top var in constructor): {}", r4.unwrap().get_int());
    } else {
        println!("Test 4 (top var in constructor): {:?}", r4);
    }

    let r5 = eval(
        &mut ctx,
        "function TaskControlBlock() { this.state = STATE_SUSPENDED; } var STATE_SUSPENDED = 2; new TaskControlBlock().state;",
    );
    if r5.is_ok() && r5.as_ref().unwrap().is_int() {
        println!("Test 5 (Richards pattern): {}", r5.unwrap().get_int());
    } else {
        println!("Test 5 (Richards pattern): {:?}", r5);
    }
}
