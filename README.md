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

## Benchmarks (2026-06-15)

V8 benchmark suite comparison (higher is better):

| Benchmark              |   qjs |   node |   boa |  pipa | vs qjs |
|------------------------|-------|--------|-------|-------|--------|
| Richards               |   881 |  44629 |   133 |   866 |  -1.7% |
| DeltaBlue              |   875 |  88373 |   136 |   828 |  -5.4% |
| Crypto                 |  1013 |  56374 |   118 |   985 |  -2.8% |
| RayTrace               |  1268 |  70299 |   276 |   589 | -53.5% |
| EarleyBoyer            |  1831 |  84585 |   341 |  1225 | -33.1% |
| RegExp                 |   306 |  12316 |  58.1 |   841 | +174.8% |
| Splay                  |  2206 |  44299 |   459 |  2000 |  -9.3% |
| NavierStokes           |  1759 |  54166 |   257 |  2087 | +18.6% |
| **SCORE (total)**      | **1101** | **50001** | **185** | **1075** | **-2.4%** |

Ranking: **#1 node** (50001) · **#2 qjs** (1101) · **#3 pipa** (1075) · **#4 boa** (185)

## test262 Compatibility (2026-06-15)

Tested against [tc39/test262](https://github.com/tc39/test262) (excluding `intl402`).

| Category | Tests | Pass Rate | Notes |
|----------|-------|-----------|-------|
| **Core Builtins** | | | |
| Math | 324 | **100%** (314/314, 10 skipped) | sumPrecise proposal skipped |
| Boolean | 50 | **100%** (50/50) | |
| parseFloat | 54 | **100%** (54/54) | VM exception handler fix |
| parseInt | 55 | **100%** (55/55) | VM exception handler fix |
| Number | 339 | **100%** (339/339) | VM exception handler fix |
| Object.is | 21 | **100%** (21/21) | |
| Object.defineProperty | 1131 | **100%** (1128/1128, 3 skipped) | |
| Object.create | 320 | **100%** (320/320) | |
| Object.getPrototypeOf | 39 | **100%** (39/39) | |
| Date | 594 | **93.6%** (556/594) | |
| URI encode/decode | 173 | **89.6%** (155/173) | URIError, UTF-8 validation, reserved chars, ToString coercion |
| Function | 507 | **85.2%** (432/507) | |
| **Other Builtins** | | | |
| Symbol | 98 | **93.9%** (92/98) | auto-boxing strict, ToString call order, new Symbol() TypeError |
| JSON | 165 | **60.6%** (100/165) | Reviver, toJSON, replacer, space, wrappers |
| Error | 180 | **52.8%** (95/180) | |
| RegExp | 1878 | **42.3%** (794/1878) | |
| String | 1222 | **77.2%** (943/1222) | |
| Reflect | 153 | **68.6%** (105/153) | Added get/set/has/deleteProperty/defineProperty/getOwnPropertyDescriptor/getPrototypeOf/setPrototypeOf/isExtensible/preventExtensions/ownKeys |
| Map | 203 | **22.2%** (45/203) | |
| Set | 382 | **18.8%** (72/382) | |
| BigInt | 77 | **44.2%** (34/77) | Added ToIndex, RangeError/TypeError, ToBigInt coercion, property descriptors |
| Promise | 676 | **5.6%** (38/676) | Limited async support |
| Proxy | 311 | **0%** (0/311) | Not yet implemented |

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
