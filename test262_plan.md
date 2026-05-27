# test262 Plan

## Principles

- bench-v8 每项不得回归：功能（所有8项必须通过）+ 性能（得分不允许明显下降，~±5% 正常波动）
- 代码不得包含注释（不增删任何注释）
- test262 不得有 crash：任何类别的运行不能产生 segfault
- 每个 fix/优化单独 commit

## Status (2026-05-27)

### Completes without crash

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
| String/proto | 715 | 325 | 1040 | 68.7% |
| String other | 19 | 28 | 47 | 40.4% |

### Hangs (infinite loop)

| Module | Sub | Tests Affected |
|---|---|---|
| String | prototype/padEnd | max-length-not-greater-than-string.js and others |
| String | prototype/padStart | similar |
| Array | prototype/indexOf | 28 tests timeout |
| Array | prototype/lastIndexOf | timeout |
| Array | prototype/reverse | timeout |
| Array | prototype/sort | timeout |
| Array | prototype/splice | timeout |
| Function | prototype/apply | entire module timeout |

## Fixes Applied

### exception_handlers leak (2026-05-27)

Root cause: When a function returned from inside a try block via Return/End opcode, the ExceptionHandler pushed by Try was not removed from the handler stack. A subsequent function call reusing the same frame_index would match the stale handler on exception, jumping to the old function's catch_pc in the new function's bytecode.

Fix: Added cleanup_handlers_for_frame() called before pop_frame in Return and End opcode handlers.

## Progress Log

- 2026-05-27: Fixed exception_handlers leak, all categories complete without segfault, bench-v8 score 1226
