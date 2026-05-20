
use pipa::{JSRuntime, eval};

const ASSERT_LIB: &str = r#"
function assert_eq(actual, expected, msg) {
    if (actual !== expected) {
        throw new Error((msg || 'assert_eq') + ': expected=' + expected + ' got=' + actual);
    }
}
function assert_ne(actual, unexpected, msg) {
    if (actual === unexpected) {
        throw new Error((msg || 'assert_ne') + ': should not equal ' + unexpected);
    }
}
function assert_true(v, msg) {
    if (!v) throw new Error((msg || 'assert_true') + ': value was falsy');
}
function assert_type(v, t, msg) {
    if (typeof v !== t) {
        throw new Error((msg || 'assert_type') + ': expected typeof=' + t + ' got=' + (typeof v));
    }
}
"#;

fn js(code: &str) {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    let src = format!("{}\n{}", ASSERT_LIB, code);
    eval(&mut ctx, &src).expect("JS threw an error");
}

#[test]
fn closure_read_outer_var() {
    js(r#"
        function outer() {
            var x = 42;
            function inner() { return x; }
            return inner();
        }
        assert_eq(outer(), 42, 'closure read outer var');
    "#);
}

#[test]
fn closure_returned_from_function() {
    js(r#"
        function make(n) {
            return function() { return n; };
        }
        var f = make(7);
        assert_eq(f(), 7, 'closure returned from function');
    "#);
}

#[test]
fn closure_multi_level_capture() {
    js(r#"
        function outer() {
            var x = 1;
            function middle() {
                var y = 10;
                function inner() { return x + y; }
                return inner;
            }
            return middle();
        }
        assert_eq(outer()(), 11, 'multi-level closure sum');
    "#);
}

#[test]
fn closure_outer_var_mutated_before_call() {
    js(r#"
        function outer() {
            var x = 1;
            function middle() {
                function inner() { return x; }
                return inner;
            }
            var g = middle();
            x = 5;
            return g();
        }
        assert_eq(outer(), 5, 'closure sees mutated outer var');
    "#);
}

#[test]
fn closure_self_ref_via_var_non_recursive() {
    js(r#"
        var f;
        f = function(x) { return x * 2; };
        assert_eq(f(21), 42, 'self-ref closure basic call');
    "#);
}

#[test]
fn closure_self_ref_via_var_recursive() {
    
    js(r#"
        var step = 0;
        var f;
        f = function(n) {
            if (n <= 0) return 0;
            step = step + 1;
            return step + f(n - 1);
        };
        assert_eq(f(3), 6, 'self-ref closure recursive');
    "#);
}

#[test]
fn closure_named_function_expr_recursion() {
    
    js(r#"
        var factorial = function fact(n) {
            return n <= 1 ? 1 : n * fact(n - 1);
        };
        assert_eq(factorial(5), 120, 'named function expression recursion');
    "#);
}

#[test]
fn closure_ctor_getter() {
    js(r#"
        function Foo() {
            var x = 10;
            this.get = function() { return x; };
        }
        var obj = new Foo();
        assert_eq(obj.get(), 10, 'ctor closure getter');
    "#);
}

#[test]
fn closure_ctor_parameterized() {
    js(r#"
        function Counter(start) {
            var n = start;
            this.get = function() { return n; };
        }
        var c = new Counter(99);
        assert_eq(c.get(), 99, 'ctor closure captures param');
    "#);
}

#[test]
fn closure_shared_upvalue_mutation() {
    js(r#"
        function make() {
            var x = 1;
            return {
                set: function(v) { x = v; },
                get: function() { return x; }
            };
        }
        var o = make();
        o.set(42);
        assert_eq(o.get(), 42, 'sibling closure shared upvalue');
    "#);
}

#[test]
fn closure_upvalue_replaced_via_setter() {
    js(r#"
        function Outer() {
            var cb = function(x) { return x * 2; };
            function callCb(val) { return cb(val); }
            this.setCb = function(f) { cb = f; };
            this.test  = function() { return callCb(10); };
        }
        var o = new Outer();
        o.setCb(function(x) { return x + 100; });
        assert_eq(o.test(), 110, 'setCb updates upvalue seen by callCb');
    "#);
}

#[test]
fn proto_method_reads_this() {
    js(r#"
        function Obj() { this.val = 7; }
        Obj.prototype.get = function() { return this.val; };
        var o = new Obj();
        assert_eq(o.get(), 7, 'proto method reads this.val');
    "#);
}

#[test]
fn proto_method_two_args() {
    js(r#"
        function Box() {}
        Box.prototype.set = function(a, b) { this.a = a; this.b = b; };
        var box = new Box();
        box.set(3, 4);
        assert_eq(box.a, 3, 'proto set arg a');
        assert_eq(box.b, 4, 'proto set arg b');
        assert_eq(box.a + box.b, 7, 'proto method two args sum');
    "#);
}

#[test]
fn proto_method_accumulate_via_loop() {
    js(r#"
        function Ctr() { this.n = 0; }
        Ctr.prototype.inc = function() { this.n = this.n + 1; };
        var c = new Ctr();
        for (var i = 0; i < 5; i++) { c.inc(); }
        assert_eq(c.n, 5, 'proto method loop accumulate');
    "#);
}

#[test]
fn proto_method_return_to_caller() {
    js(r#"
        function Foo() {}
        Foo.prototype.bar = function() { return 42; };
        var f = new Foo();
        var result = f.bar();
        assert_eq(result, 42, 'proto method return value stored in var');
    "#);
}

#[test]
fn control_while_break() {
    js(r#"
        function wb() { while (true) { break; } return 42; }
        assert_eq(wb(), 42, 'while(true)+break return value');
    "#);
}

#[test]
fn control_while_break_called_from_for() {
    js(r#"
        function wb() { while (true) { break; } return 42; }
        function fc() {
            var r;
            for (var i = 0; i < 1; i++) { r = wb(); }
            return r;
        }
        assert_eq(fc(), 42, 'while-break called from for');
    "#);
}

#[test]
fn control_nested_for_equality() {
    js(r#"
        var count = 0;
        for (var y = 0; y < 10; y++) {
            for (var x = 0; x < 10; x++) {
                if (x === y) count = count + 1;
            }
        }
        assert_eq(count, 10, 'nested for equality count');
    "#);
}

#[test]
fn control_do_while() {
    js(r#"
        var n = 0;
        do { n = n + 1; } while (n < 5);
        assert_eq(n, 5, 'do-while basic');
    "#);
}

#[test]
fn control_for_in_key_count() {
    js(r#"
        var obj = {a: 1, b: 2, c: 3};
        var count = 0;
        for (var k in obj) { count = count + 1; }
        assert_eq(count, 3, 'for-in key count');
    "#);
}

#[test]
fn control_ternary() {
    js(r#"
        var a = 5 > 3 ? 'yes' : 'no';
        assert_eq(a, 'yes', 'ternary true branch');
        var b = 1 > 3 ? 'yes' : 'no';
        assert_eq(b, 'no', 'ternary false branch');
    "#);
}

#[test]
fn array_element_from_method() {
    js(r#"
        function Foo() {}
        Foo.prototype.bar = function() { return 42; };
        var f = new Foo();
        var arr = [];
        arr[0] = f.bar();
        assert_eq(arr[0], 42, 'array element from method');
    "#);
}

#[test]
fn array_push_and_length() {
    js(r#"
        var arr = [];
        arr.push(1);
        arr.push(2);
        arr.push(3);
        assert_eq(arr.length, 3, 'array push length');
    "#);
}

#[test]
fn array_new_with_length() {
    js(r#"
        var a = new Array(2);
        a[0] = 10;
        a[1] = 20;
        assert_eq(a[0] + a[1], 30, 'new Array(2) slot access');
    "#);
}

#[test]
fn math_sqrt() {
    js("assert_eq(Math.sqrt(16), 4, 'sqrt(16)=4');");
}

#[test]
fn math_floor() {
    js("assert_eq(Math.floor(3.9), 3, 'floor(3.9)=3');");
}

#[test]
fn math_abs() {
    js("assert_eq(Math.abs(-5), 5, 'abs(-5)=5');");
}

#[test]
fn math_random_range() {
    js(r#"
        var r = Math.random();
        assert_true(r >= 0, 'random >= 0');
        assert_true(r < 1,  'random < 1');
    "#);
}

#[test]
fn builtin_parseint_decimal() {
    js("assert_eq(parseInt('42'), 42, 'parseInt decimal');");
}

#[test]
fn builtin_parseint_hex() {
    js("assert_eq(parseInt('ff', 16), 255, 'parseInt hex');");
}

#[test]
fn string_length() {
    js(r#"assert_eq('hello'.length, 5, 'string length');"#);
}

#[test]
fn string_concat() {
    js(r#"assert_eq('foo' + 'bar', 'foobar', 'string concat');"#);
}

#[test]
fn string_typeof() {
    js(r#"assert_type('hello', 'string', 'string typeof');"#);
}

#[test]
fn error_message_property() {
    js(r#"
        var e = new Error('oops');
        assert_eq(e.message, 'oops', 'Error.message');
    "#);
}

#[test]
fn typeof_undefined_is_string() {
    js(r#"assert_eq(typeof undefined, 'undefined', 'typeof undefined');"#);
}

#[test]
fn typeof_function_is_string() {
    js(r#"assert_eq(typeof function(){}, 'function', 'typeof function');"#);
}

#[test]
fn typeof_number_is_string() {
    js(r#"assert_eq(typeof 42, 'number', 'typeof number');"#);
}

#[test]
fn date_now_positive() {
    js(r#"assert_true(Date.now() > 0, 'Date.now() > 0');"#);
}

#[test]
fn runner_object_callbacks() {
    js(r#"
        var log = [];
        var runner = {
            NotifyStart:  function(name) { log.push('start:' + name); },
            NotifyResult: function(name, r) { log.push('result:' + name + '=' + r); },
            NotifyScore:  function(s) { log.push('score:' + s); }
        };
        runner.NotifyStart('Test');
        runner.NotifyResult('Test', 42);
        runner.NotifyScore(100);
        assert_eq(log.length, 3, 'runner callback count');
    "#);
}

#[test]
fn while_loop_not_false_guard() {
    js(r#"
        var data = 0;
        var i = 0;
        while (data !== false && i < 5) {
            data = i;
            i = i + 1;
        }
        assert_eq(i, 5, 'data-not-false while guard');
    "#);
}

#[test]
fn direct_method_call_pattern() {
    js(r#"
        function Suite(name) {
            this.name = name;
            this.ran = false;
        }
        Suite.prototype.RunStep = function() {
            this.ran = true;
            return 1;
        };
        var suite = new Suite('Demo');
        var result = suite.RunStep();
        assert_eq(result, 1, 'direct method call result');
        assert_eq(suite.ran, true, 'direct method call side effect');
    "#);
}
