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
| Array | prototype/indexOf | 3 tests with 2^32-length arrays |
| Array | prototype/lastIndexOf | 3 tests with 2^32-length arrays |
| Array | prototype/splice | 1 test with large array |
| Function | prototype/apply | module timeout (unknown cause) |

## Fixes Applied

### exception_handlers leak (2026-05-27)

Root cause: When a function returned from inside a try block via Return/End opcode, the ExceptionHandler pushed by Try was not removed from the handler stack. A subsequent function call reusing the same frame_index would match the stale handler on exception, jumping to the old function's catch_pc in the new function's bytecode.

Fix: Added cleanup_handlers_for_frame() called before pop_frame in Return and End opcode handlers.

### String padEnd/padStart hang (2026-05-27)

Root cause: string_pad_end and string_pad_start used args[1].get_int() to get the target length. For object arguments, get_int() returns the raw object pointer as an integer (e.g. 94203228883904), causing the padding loop to iterate trillions of times.

Fix: Replaced get_int() with js_to_length() which properly converts to number first (NaN/objects -> 0). Also replaced direct string check with js_to_string_arg() for the filler argument.

## Progress Log

- 2026-05-27: Fixed exception_handlers leak, all categories complete without segfault, bench-v8 score 1226
- 2026-05-27: Fixed padEnd/padStart infinite loop, String module now completes (844/1222 = 69.1%), bench-v8 score 1216
