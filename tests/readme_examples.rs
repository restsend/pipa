use pipa::{JSRuntime, JSValue, eval};

#[test]
fn test_example_eval_js() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let val = eval(&mut ctx, "1 + 2").unwrap();
    assert_eq!(val.get_int(), 3);
}

#[test]
fn test_example_read_strings() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    eval(
        &mut ctx,
        r#"
        function greet(name) {
            return "Hello, " + name + "!";
        }
    "#,
    )
    .unwrap();

    let val = eval(&mut ctx, r#"greet("world")"#).unwrap();
    assert!(val.is_string());
    let s = ctx.get_atom_str(val.get_atom());
    assert_eq!(s, "Hello, world!");
}

#[test]
fn test_example_register_builtin() {
    fn js_print(ctx: &mut pipa::JSContext, args: &[JSValue]) -> JSValue {
        for arg in args {
            if arg.is_string() {
                print!("{}", ctx.get_atom_str(arg.get_atom()));
            } else if arg.is_int() {
                print!("{}", arg.get_int());
            }
        }
        println!();
        JSValue::undefined()
    }

    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    ctx.register_global_builtin("print", 1, js_print);

    eval(&mut ctx, r#"print("hello from Rust!")"#).unwrap();
}

#[test]
fn test_example_bytecode_compilation() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let (code, _constants) = pipa::compile_to_register_bytecode(
        &mut ctx,
        "function fib(n) { return n < 2 ? n : fib(n-1) + fib(n-2); } fib(20)",
    )
    .unwrap();

    assert!(!code.is_empty());
    // fib(20) = 6765
    let val = eval(
        &mut ctx,
        "function fib(n) { return n < 2 ? n : fib(n-1) + fib(n-2); } fib(20)",
    )
    .unwrap();
    assert_eq!(val.get_int(), 6765);
}
