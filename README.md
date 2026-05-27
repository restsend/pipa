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

## Benchmarks (2026-05-26)

V8 benchmark suite comparison (higher is better):

| Benchmark              |   qjs |   node |   boa |  pipa | vs qjs |
|------------------------|-------|--------|-------|-------|--------|
| Richards               |   966 |  46846 |   133 |   967 |  +0.1% |
| DeltaBlue              |   948 |  94979 |   140 |   975 |  +2.8% |
| Crypto                 |  1097 |  60072 |   125 |  1073 |  -2.2% |
| RayTrace               |  1467 |  79697 |   315 |   896 | -38.9% |
| EarleyBoyer            |  2127 |  95129 |   281 |  1333 | -37.3% |
| RegExp                 |   330 |  12703 |  41.6 |   977 | +196.1% |
| Splay                  |  2428 |  48609 |   536 |  2901 | +19.5% |
| NavierStokes           |  1807 |  56392 |   288 |  1610 | -10.9% |
| **SCORE (total)**      | **1208** | **53836** | **184** | **1269** | **+5.0%** |

Ranking: **#1 node** (53836) · **#2 pipa** (1269) · **#3 qjs** (1208) · **#4 boa** (184)

## test262 Compatibility (2026-05-26)

Tested against [tc39/test262](https://github.com/tc39/test262) (excluding `intl402`).

| Category | Tests | Pass Rate | Notes |
|----------|-------|-----------|-------|
| **Core Builtins** | | | |
| Math | 324 | **98.8%** (320/324) | 4 edge cases in `sumPrecise` |
| Boolean | 50 | **98.0%** (49/50) | 1 cross-realm test skipped |
| Object.is | 21 | **100%** (21/21) | |
| Object.defineProperty | 1131 | **98.8%** (1118/1131) | |
| Object.create | 320 | **99.4%** (318/320) | |
| Object.getPrototypeOf | 39 | **100%** (39/39) | |
| Date | 594 | **31.8%** (189/594) | Partial date support |
| global | 29 | **96.6%** (28/29) | |
| Infinity | 6 | **100%** (6/6) | |
| eval | 10 | **80.0%** (8/10) | |
| URI encode/decode | 118 | **77.9%** (92/118) | |
| **Expressions** | | | |
| Addition | 48 | **97.9%** (47/48) | |
| Bitwise ops | 47 | **76.6%** (36/47) | |
| **Other Builtins** | | | |
| JSON | 165 | **42.4%** (70/165) | |
| Symbol | 98 | **30.6%** (30/98) | |
| Error | 92 | **26.1%** (24/92) | |
| Reflect | 153 | **20.9%** (32/153) | |
| Map | 203 | **21.2%** (43/203) | |
| Set | 382 | **18.3%** (70/382) | |
| BigInt | 77 | **23.4%** (18/77) | |
| Promise | 676 | **5.5%** (37/676) | Limited async support |
| Proxy | 311 | **0%** (0/311) | Not yet implemented |

**Sampled pass rate: ~55%** (across categories tested above, weighted by test count)

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
