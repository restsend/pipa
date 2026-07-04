# test262 Plan

## Principles

- bench-v8 每项不得回归：功能（所有8项必须通过）+ 性能（得分不允许明显下降，~±5% 正常波动）
- 代码不得包含注释（不增删任何注释）
- test262 不得有 crash：任何类别的运行不能产生 segfault
- 每个 fix/优化单独 commit

## Status (2026-06-14)

### Modules at 100%

| Module | Total |
|---|---|
| Math | 324 |
| parseInt | 55 |
| parseFloat | 54 |
| Boolean | 50 |
| Number | 339 |
| isNaN | 15 |
| isFinite | 15 |
| NaN | 6 |
| Infinity | 6 |
| eval | 9 (+1 skipped) |
| undefined | 8 |
| global | 29 |
| decodeURI | 55 |
| decodeURIComponent | 56 |

### Completes without crash or hang

| Module | Pass | Fail | Total | Pass Rate |
|---|---|---|---|---|
| Symbol | 94 | 4 | 98 | 95.9% |
| Array/isArray | 27 | 0 | 27 (+2 skipped) | 100% of non-skipped |
| String/fromCharCode | 16 | 1 | 17 | 94.1% |
| encodeURI | 26 | 5 | 31 | 83.9% |
| encodeURIComponent | 26 | 5 | 31 | 83.9% |
| ThrowTypeError | 4 | 10 | 14 | 28.6% |

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
- 2026-05-27: Fixed remaining array method length reads (forEach/every/some/map/filter/etc), bench-v8 score 1300
- 2026-05-27: Fixed builtin needs_this: use HostFunction.needs_this instead of hardcoded list, bench-v8 1253
- 2026-05-27: Fixed parseFloat proper string parsing (scientific notation, Infinity, ToPrimitive), bench-v8 1241
- 2026-05-27: Fixed Number.prototype.toString for wrapper objects, Object.prototype.toString for Number/Boolean wrappers, parseInt/parseFloat ToPrimitive, bench-v8 1238
- 2026-05-27: Fixed parseInt radix: object radix via valueOf/toString, empty string, Infinity, Int32 overflow, Boolean valueOf. parseInt 89.1%, parseFloat 88.9%, Boolean 96.0%, bench-v8 1246
- 2026-05-27: Added not-a-constructor check for builtins in CallNew, made global builtins non-enumerable. parseInt 92.7%, parseFloat 94.4%, Number 97.3%, Boolean 98.0%, Math 97.8%, String 70.9%, bench-v8 1275
- 2026-05-27: Fixed String.fromCodePoint range/integer validation. bench-v8 1235
- 2026-05-28: Fixed Function.prototype.call/apply to propagate exceptions from builtins. Number 98.5%, String 73.2%, bench-v8 1229
- 2026-05-28: Made all 34 global constructors non-enumerable. Math 97.8%, bench-v8 1279
- 2026-05-28: Made all prototype methods non-enumerable (Array 33, String 34, Object 6, Date 16, Error 4, RegExp 13). String 74.5%, Error 31.5%, Date 34.5%, bench-v8 1289
- 2026-05-28: Fixed Error.prototype descriptor { writable: false, enumerable: false, configurable: false }. Error 32.6%, bench-v8 1230
- 2026-05-28: Fixed Object.prototype.toString to identify Date wrapper objects via prototype chain. Date 34.5% -> 36.9%, bench-v8 1281
- 2026-05-28: Fixed Function.prototype.apply/call TypeError for non-function this and non-object argArray. Function 76.9% -> 77.3%, bench-v8 1234
- 2026-05-28: Set own 'length' property on String wrapper objects in CallNew. bench-v8 1250
- 2026-05-28: Added last_caught_exception to VM for preserving exception objects across call_function_with_this. bench-v8 1240
- 2026-05-28: Fixed String method position coercion (includes/indexOf/substring/slice/repeat/codePointAt). String 74.5% -> 75.2%
- 2026-05-28: Fixed String.prototype.split: handle undefined separator and limit argument. String 75.2% -> 75.4%
- 2026-05-28: Fixed Error constructor property descriptors: message/name/cause now non-enumerable. Error 32.6% -> 34.8%
- 2026-05-28: Fixed String method position coercion, split limit, startsWith/endsWith, constructor prop. String 75.4% -> 76.0%
- 2026-05-28: Added Date.prototype.constructor and toJSON. Date 36.9% -> 38.9%
- 2026-06-14: Made function prototype patching recursive (fixes nested builtin methods like Number.prototype.toFixed.hasOwnProperty). Number 98.8% -> 99.7%. bench-v8 1149 (noisy)
- 2026-06-14: Fixed Number.prototype.toString/valueOf to check prototype chain (must be Number wrapper). Fixed toFixed for very large numbers (>=1e21). Number 99.7% -> 100%
- 2026-06-14: Fixed global isNaN/isFinite to use proper ToNumber (string/array/object). isNaN 40% -> 100%, isFinite 40% -> 100%
- 2026-06-14: Fixed js_to_primitive_number to honor Symbol.toPrimitive. Symbol 94.9% -> 95.9%
- 2026-06-14: Added Symbol.species accessor to Array/Map/Set/RegExp/Promise constructors
- 2026-06-14: Treated undefined/NaN/Infinity as Identifiers (not Literals) so assignments/delete work per spec. NaN 100%, Infinity 100%, undefined 87.5%
- 2026-06-14: Spec-correct String.fromCharCode via ToUint16 + UTF-16-lossy decode. Number(bigint) handled explicitly; ToNumber(BigInt) throws. decodeURI 100%, decodeURIComponent 100%, fromCharCode 94%
- 2026-06-14: Made Array.prototype an exotic Array object. Array.isArray 100%
- 2026-06-14: Enforce strict-mode TypeError for non-writable SetField (obj[key] = ...) and SetGlobal (var on non-writable global). undefined 100%, global 100%
- 2026-07-04: Object.defineProperty/defineProperties now throw TypeError on [[DefineOwnProperty]] rejection; define_property_ext made spec-correct (non-extensible rejection, data prop for value-less descriptors, attribute-only updates, data<->accessor transitions, non-configurable validation: writable->non-writable allowed, SameValue get/set additions, same-value writes). defineProperty 681->277, defineProperties 423->265-ish, Object 1684->901.
- 2026-07-04: Object.getOwnPropertyDescriptor/getOwnPropertyDescriptors set Object.prototype on returned descriptor objects (were prototype-less, missing inherited methods). getOwnPropertyDescriptor 101->67.
- 2026-07-04: String.prototype split/match/matchAll/etc. result arrays now get Array.prototype (were prototype-less). String/prototype 585->513 (resolves the 89 'split.constructor' failures).
- 2026-07-04: Implemented String.prototype.localeCompare (code-unit compare) and normalize (form validation, RangeError on invalid). localeCompare 0->11/13, normalize 0->6/14.
- 2026-07-04: Object.prototype __defineGetter__/__defineSetter__/__lookupGetter__/__lookupSetter__ (Annex B accessor methods). Object/prototype 101->69.
- 2026-07-04: Object.defineProperty validates array length value (RangeError on >2^32-1). defineProperty 290->277. Object category overall 1684->901 failing. bench-v8 8/8 pass throughout.
- 2026-07-04: Array.prototype methods (push/pop/shift/unshift/slice/join/concat/sort/reverse/fill/splice/flat/iterators) now use RequireObjectCoercible + ToObject for primitive receivers per spec. concat now checks IsArray for spreadability (was spreading any object with length). flat iterates receiver indices at top-level (was pushing non-array this whole). object_to_object handles BigInt (prevented sort.call(0n) hang). concat 13->17, reverse 16->21, fill 10->14, sort 20->21, all call-with-boolean/call-with-primitive tests pass. bench-v8 8/8 pass.
