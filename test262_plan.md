# test262 Plan

## Principles

- bench-v8 每项不得回归：功能（所有8项必须通过）+ 性能（得分不允许明显下降，~±5% 正常波动）
- 代码不得包含注释（不增删任何注释）
- test262 不得有 crash：任何类别的运行不能产生 segfault
- 每个 fix/优化单独 commit

## Status (2026-05-27)

### Completes without crash or hang

| Module | Pass | Fail | Total | Pass Rate |
|---|---|---|---|---|
| Math | 282 | 42 | 324 | 87.0% |
| Number | 321 | 18 | 339 | 94.7% |
| Boolean | 47 | 3 | 50 | 94.0% |
| parseInt | 39 | 16 | 55 | 70.9% |
| parseFloat | 23 | 31 | 54 | 42.6% |
| Symbol | 44 | 54 | 98 | 44.9% |
| Error | 30 | 62 | 92 | 32.6% |
| JSON | 73 | 92 | 165 | 44.2% |
| Map | 44 | 159 | 203 | 21.7% |
| Set | 72 | 310 | 382 | 18.8% |
| Reflect | 33 | 120 | 153 | 21.6% |
| Proxy | 0 | 311 | 311 | 0.0% |
| Date | 177 | 417 | 594 | 29.8% |
| Iterator | 52 | 461 | 513 | 10.1% |
| RegExp | 769 | 1109 | 1878 | 40.9% |
| Promise | 50 | 626 | 676 | 7.4% |
| Object | 2025 | 554 | 2579 | 78.5% |
| String | 844 | 378 | 1222 | 69.1% |
| Array | 804 | 1533 | 2337 | 34.4% |

### Slow (very large array operations, not hangs)

| Module | Sub | Tests |
|---|---|---|
| Object | prototype/setPrototypeOf | 2 tests hang on Object.prototype mutation |
| Array | prototype/reverse,sort | Now fixed (completed without hang) |

## Fixes Applied

### exception_handlers leak (2026-05-27)

Root cause: When a function returned from inside a try block via Return/End opcode, the ExceptionHandler pushed by Try was not removed from the handler stack. A subsequent function call reusing the same frame_index would match the stale handler on exception, jumping to the old function's catch_pc in the new function's bytecode.

Fix: Added cleanup_handlers_for_frame() called before pop_frame in Return and End opcode handlers.

### String padEnd/padStart hang (2026-05-27)

Root cause: string_pad_end and string_pad_start used args[1].get_int() to get the target length. For object arguments, get_int() returns the raw object pointer as an integer (e.g. 94203228883904), causing the padding loop to iterate trillions of times.

Fix: Replaced get_int() with js_to_length() which properly converts to number first (NaN/objects -> 0). Also replaced direct string check with js_to_string_arg() for the filler argument.

### Array indexOf/lastIndexOf/includes/splice slow paths (2026-05-27)

Root causes:
1. indexOf/lastIndexOf: Used i32/u32 for length and fromIndex, truncating values > 2^31. Iterated all indices 0..len even for sparse arrays with len = 2^32.
2. includes: Same u32 length issue, iterated all indices.
3. splice: Materialized entire array into Vec before splicing, causing OOM for large sparse arrays.
4. SetField opcode: Extended dense arrays by pushing undefined values up to the index, causing billions of pushes for idx > 2^31.

Fixes:
- indexOf/lastIndexOf/includes: Use u64 for length, f64 for fromIndex, handle NaN, use sparse iteration for len > 10M.
- splice: Rewrite to shift elements in-place without materializing, use js_to_length for proper clamping.
- SetField: For dense arrays with idx > 100K, store as sparse property and clear dense flag instead of extending.
- js_to_length: Made pub, returns u64, handles Infinity.
- padEnd/padStart: Clamp target_len to 1<<30.

## Progress Log

- 2026-05-27: Fixed exception_handlers leak, all categories complete without segfault, bench-v8 score 1226
- 2026-05-27: Fixed padEnd/padStart infinite loop, String module now completes (844/1222 = 69.1%), bench-v8 score 1216
- 2026-05-27: Fixed Array indexOf/lastIndexOf/includes/splice/SetField slow paths for large sparse arrays, all Array sub-suites complete without hang, bench-v8 score 1218
- 2026-05-27: Fixed Function.prototype.apply hang on array-like with getter/non-int length (390/507 pass), bench-v8 score 1212
- 2026-05-27: Fixed Array sort/reverse/all-iterable-methods hang on negative/huge length, bench-v8 score 1243
- 2026-05-27: Fixed parseInt Infinity/bool/string-radix/large-number bugs (44/55 = 80%), bench-v8 score 1211
