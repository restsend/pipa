use pipa::{JSRuntime, eval};

#[test]
fn test_step_by_step() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "1+2");
    assert!(r.is_ok(), "step1 failed: {:?}", r);

    let r = eval(&mut ctx, "typeof eval");
    assert!(r.is_ok(), "step2 failed: {:?}", r);
    if let Ok(v) = &r {
        println!("typeof eval = {:?}", v);
    }

    let r = eval(&mut ctx, "eval('1+2')");
    assert!(r.is_ok(), "step3 failed: {:?}", r);

    println!("All steps passed!");
}
