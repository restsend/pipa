use pipa::{JSRuntime, eval};

fn main() {
    let code = r#"
var print = console.log;
function makeTree(depth) {
  if (depth == 0) return { val: 1 };
  return { left: makeTree(depth - 1), right: makeTree(depth - 1) };
}
print("starting");
makeTree(5);
print("done");
"#;
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    match eval(&mut ctx, code) {
        Ok(v) => println!("OK: {:?}", v),
        Err(e) => println!("ERR: {}", e),
    }
}
