use pipa::{JSRuntime, eval};
fn main() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    let code = r#"
        var x = 32;
        x < 32 ? "less" : "not less";
    "#;
    match eval(&mut ctx, code) {
        Ok(v) => println!("result: {:?}", v),
        Err(e) => println!("ERR: {}", e),
    }
}
