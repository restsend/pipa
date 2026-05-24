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

## Benchmarks (2026-05-24)

V8 benchmark suite comparison (higher is better):

| Benchmark              |   qjs |   node |   boa |  pipa | vs qjs |
|------------------------|-------|--------|-------|-------|--------|
| Richards               |   966 |  46846 |   133 |   967 |  +0.1% |
| DeltaBlue              |   948 |  94979 |   140 |   975 |  +2.8% |
| Crypto                 |  1097 |  60072 |   125 |  1073 |  -2.2% |
| RayTrace               |  1467 |  79697 |   315 |   896 | -38.9% |
| EarleyBoyer            |  2127 |  95129 |   281 |  1333 | -37.3% |
| RegExp                 |   330 |  12703 |  41.6 |   934 | +183.0% |
| Splay                  |  2428 |  48609 |   536 |  2901 | +19.5% |
| NavierStokes           |  1807 |  56392 |   288 |  1866 |  +3.3% |
| **SCORE (total)**      | **1208** | **53836** | **184** | **1254** | **+3.8%** |

Ranking: **#1 node** (53836) · **#2 pipa** (1254) · **#3 qjs** (1208) · **#4 boa** (184)

## test262 Compatibility (2026-05-24)

Tested against [tc39/test262](https://github.com/tc39/test262) (excluding `intl402` and `annexB`).

| Category | Pass Rate | Notes |
|----------|-----------|-------|
| **Core Operators** | | |
| Addition, Coalesce, Comma, Grouping, Logical-And/Or, Strict-Equals/Not-Equals, Void, Bitwise-Not, Relational | **100%** | Fully compliant |
| Subtraction, Division, Multiplication, Modulus, Exponentiation | **64–82%** | Mostly compliant |
| **Control Flow** | | |
| `if` | **100%** (69/69) | ✅ |
| `switch` | **100%** (111/111) | ✅ |
| `while` | **100%** (38/38) | ✅ |
| `do-while` | **100%** (36/36) | ✅ |
| `for-in` | **94%** (108/115) | ✅ `let` as identifier, TDZ, completion values, enumeration order, `Object.defineProperty` attribute preservation<br>⏳ scope/lexical environment, fresh per-iteration bindings |
| `for` | **~99%** | ✅ iterator-based array destructuring, `let` as identifier<br>⏳ 2 scope/lexical failures |
| `try/catch/finally` | **~97%** | ✅ break/continue through finally, error propagation<br>⏳ completion values, scope, optional catch |
| `const`/`let` | **73–74%** | |
| **Functions** | | |
| Function declarations/expressions | **66–73%** | |
| Arrow functions | **80%** | |
| Generators | **65–67%** | |
| **Builtins** | | |
| Math | **49%** | |
| JSON | **23%** | |
| Boolean | **59%** | |
| Promise | **7%** | Limited async support |
| Proxy/Reflect | **0–19%** | Not yet implemented |
| Set/Map/WeakMap/WeakSet | **6–23%** | Partial implementation |

**Overall sampled pass rate: ~52%** (5327 tests sampled across all categories above)

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
use pipa::{JSRuntime, eval, eval_async};

let mut rt = JSRuntime::new();
let mut ctx = rt.new_context();

eval_async(&mut ctx, r#"
    var result = null;
    (async () => {
        result = await fetch("https://httpbin.org/json");
    })();
"#).unwrap();

let val = eval(&mut ctx, "JSON.stringify(result)").unwrap();
println!("{}", ctx.get_atom_str(val.get_atom()));
```

> Requires the `fetch` feature (enabled by default). `eval_async` is `eval` + `run_event_loop` in one call.

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
