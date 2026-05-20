use pipa::{JSRuntime, eval};

fn main() {
    let code = r#"
function makeTree(depth) {
  if (depth == 0) return { val: 1 };
  return { left: makeTree(depth - 1), right: makeTree(depth - 1) };
}
makeTree(2);
"#;
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    println!("before eval");
    match eval(&mut ctx, code) {
        Ok(v) => println!("OK: {:?}", v),
        Err(e) => println!("ERR: {}", e),
    }
    println!("after eval");
}
