use pipa::{JSRuntime, eval};
fn main() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    match eval(&mut ctx, "console.log('hello from console');") {
        Ok(_) => println!("OK"),
        Err(e) => println!("ERR: {}", e),
    }
}
