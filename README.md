# pipa (枇杷) - A fast, minimal ES2023 JavaScript runtime built in Rust.

## Features

- **ES2023 compliant** — implements the ECMAScript 2023 specification
- **Async/await built-in** — first-class async/await support without transpilation
- **Bytecode support** — compile JavaScript to `.jsc` bytecode files for fast loading and execution, with configurable optimization levels (`-O0` through `-O3`)
- **Fast** — outperforms QuickJS in benchmarks
- **Small** — ~5.2 MB binary (with `repl` feature)
- **Zero-dependency** built-in implementations for:
  - Regex/JSON/Base64/BigInt
  - Unicode
  - `fetch` (HTTP client), `rusttls` required 
  - WebSocket
  - Server-Sent Events (SSE)

No external C libraries or system dependencies for the above — everything is implemented from scratch in Rust.

## Benchmarks (2026-05-20)

V8 benchmark suite comparison (higher is better):

| Benchmark              |   qjs |   node |   boa |  pipa |
|------------------------|-------|--------|-------|-------|
| Richards               |   980 |  46634 |   139 |   977 |
| DeltaBlue              |   937 |  91283 |   143 |  1041 |
| Crypto                 |  1087 |  60024 |   123 |  1117 |
| RayTrace               |  1479 |  80881 |   315 |   994 |
| EarleyBoyer            |  2157 |  97221 |   373 |  1426 |
| RegExp                 |   338 |  13274 |  62.6 |  1014 |
| Splay                  |  2461 |  52843 |   554 |  2002 |
| NavierStokes           |  1837 |  56392 |   292 |  1901 |
| **SCORE (total)**      | **1219** | **54642** | **203** | **1256** |

Ranking: **#1 node** (54642) · **#2 pipa** (1256) · **#3 qjs** (1219) · **#4 boa** (203)

## Usage

```bash
cargo install pipa-js
# Run a script
pipa script.js

# Run precompiled bytecode
pipa script.jsc

# Compile JavaScript to bytecode
pipa -compile input.js output.jsc

# Disassemble bytecode (debugging)
pipa -diss script.jsc

# Specify optimization level (default: -O2)
pipa -O3 script.js

# Start REPL (requires the repl feature)
pipa
```

## Embedding in Rust

Use pipa-js as a library to embed JavaScript in your Rust project:

```toml
[dependencies]
pipa-js = "0.1.2"
```

### Evaluate JavaScript

```rust
use pipa::{JSRuntime, eval};

let mut rt = JSRuntime::new();
let mut ctx = rt.new_context();

let val = eval(&mut ctx, "1 + 2").unwrap();
assert_eq!(val.get_int(), 3);
```

### Read strings & values from JavaScript

```rust
use pipa::{JSRuntime, eval};

let mut rt = JSRuntime::new();
let mut ctx = rt.new_context();

eval(&mut ctx, r#"
    function greet(name) {
        return "Hello, " + name + "!";
    }
"#).unwrap();

let val = eval(&mut ctx, r#"greet("world")"#).unwrap();
assert!(val.is_string());
let s = ctx.get_atom_str(val.get_atom());
assert_eq!(s, "Hello, world!");
```

### Call custom Rust functions from JavaScript

```rust
use pipa::{JSRuntime, eval, JSValue};

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
```

### Async/await with event loop

```rust
use pipa::{JSRuntime, eval, run_event_loop};

let mut rt = JSRuntime::new();
let mut ctx = rt.new_context();

eval(&mut ctx, r#"
    var result = null;
    async function main() {
        var data = await fetch("https://httpbin.org/json");
        result = data;
    }
    main();
"#).unwrap();

run_event_loop(&mut ctx).unwrap();

let val = eval(&mut ctx, "JSON.stringify(result)").unwrap();
let s = ctx.get_atom_str(val.get_atom());
println!("{}", s);
```

> Requires the `fetch` feature (enabled by default).

### Bytecode compilation

```rust
use pipa::{JSRuntime, eval, compile_to_register_bytecode};

let mut rt = JSRuntime::new();
let mut ctx = rt.new_context();

// Compile JavaScript to register-based bytecode
let (code, constants) = compile_to_register_bytecode(
    &mut ctx,
    "function fib(n) { return n < 2 ? n : fib(n-1) + fib(n-2); } fib(20)",
).unwrap();

// code: Vec<u8>, constants: Vec<JSValue>
assert!(!code.is_empty());
```

## Build

```bash
# Default build (includes REPL, fetch, and process support)
cargo build --release

# Minimal build (no REPL, no fetch, no process)
cargo build --release --no-default-features
```

> If using pipa as a library dependency and you don't need REPL/fetch/process features, add it with `default-features = false`:
> ```toml
> [dependencies]
> pipa-js = { version = "0.1.1", default-features = false }
> ```

## License

MIT
