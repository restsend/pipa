use pipa::{JSRuntime, eval};

#[test]
fn test_inherited_upvalue() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    let code = r#"
        function outer() {
            var x = 42;
            function middle() {
                function inner() {
                    return x;
                }
                return inner();
            }
            return middle();
        }
        outer();
    "#;
    let r = eval(&mut ctx, code);
    println!("result: {:?}", r);
    assert!(r.is_ok() && r.as_ref().unwrap().is_int() && r.unwrap().get_int() == 42);
}
