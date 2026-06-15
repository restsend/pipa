#![cfg(feature = "full_runtime_tests")]

use pipa::{JSRuntime, eval};

#[test]
fn test_no_nursery_disable_under_survivors() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        const N = 20000;
        let keep = [];
        for (let i = 0; i < N; i++) {
            keep.push({ id: i, data: new Array(10).fill(i) });
        }
        keep.length;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 20000);

    let heap = runtime.gc_heap();
    assert!(heap.object_count() >= 20000);
}

#[test]
fn test_churn_high_mortality() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    ctx.runtime_mut().gc_heap_mut().set_threshold(1024 * 1024);

    let code = r#"
        const N = 50000;
        let sum = 0;
        for (let i = 0; i < N; i++) {
            let obj = { a: i, b: i + 1, c: String(i) };
            let arr = [i, i + 1, i + 2, obj];
            sum += arr[0] + arr[3].a;
        }
        sum;
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), (0..50000).map(|i| i + i).sum::<i64>());
}

#[test]
fn test_cross_gc_references_persist() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let code = r#"
        (function() {
            const N = 1000;
            let roots = [];
            for (let i = 0; i < N; i++) {
                let inner = { val: i };
                let outer = { child: inner, id: i };
                roots.push(outer);
            }
            for (let i = 0; i < N; i++) {
                if (roots[i].child.val !== i) return false;
            }
            return true;
        })();
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_bool(), true);
}

#[test]
fn test_gc_does_not_crash_under_allocation_pressure() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    ctx.runtime_mut().gc_heap_mut().set_threshold(1024 * 1024);

    let code = r#"
        (function() {
            const N = 10000;
            let objs = [];
            for (let i = 0; i < N; i++) {
                objs.push({
                    a: [1,2,3,4,5,6,7,8,9,10],
                    b: { x: 1, y: 2 },
                    c: String(i),
                    d: i * 2
                });
            }
            for (let i = 0; i < N - 1; i++) {
                objs[i].next = objs[i+1];
            }
            let count = 0;
            let cur = objs[0];
            while (cur) { count++; cur = cur.next; }
            return count;
        })();
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 10000);
}

#[test]
fn test_prototype_chain_survives_gc() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    ctx.runtime_mut().gc_heap_mut().set_threshold(1024 * 1024);

    let code = r#"
        (function() {
            const N = 1000;
            function Maker(name) {
                this.name = name;
            }
            Maker.prototype.hello = function() { return "hello " + this.name; };

            let instances = [];
            for (let i = 0; i < N; i++) {
                instances.push(new Maker("obj" + i));
            }
            for (let i = 0; i < N; i++) {
                let msg = instances[i].hello();
                if (msg !== "hello obj" + i) return false;
            }
            return true;
        })();
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_bool(), true);
}

#[test]
fn test_closure_captures_survive_gc() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    ctx.runtime_mut().gc_heap_mut().set_threshold(1024 * 1024);

    let code = r#"
        (function() {
            function makeClosure(val) {
                let captured = { value: val, tag: "captured" };
                return function() {
                    return captured.value;
                };
            }

            const N = 2000;
            let fns = [];
            for (let i = 0; i < N; i++) {
                fns.push(makeClosure(i));
            }
            for (let i = 0; i < N; i++) {
                let result = fns[i]();
                if (result !== i) return false;
            }
            return true;
        })();
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_bool(), true);
}

#[test]
fn test_array_elements_survive_gc() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    ctx.runtime_mut().gc_heap_mut().set_threshold(1024 * 1024);

    let code = r#"
        (function() {
            let big = [];
            for (let i = 0; i < 5000; i++) {
                let row = [];
                for (let j = 0; j < 10; j++) {
                    row.push({ r: i, c: j, val: i * 100 + j });
                }
                big.push(row);
            }
            let sum = 0;
            for (let i = 0; i < 5000; i++) {
                for (let j = 0; j < 10; j++) {
                    sum += big[i][j].val;
                }
            }
            return sum;
        })();
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), (0..5000).flat_map(|i| (0..10).map(move |j| i * 100 + j)).sum::<i64>());
}

#[test]
fn test_object_spread_and_gc() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    ctx.runtime_mut().gc_heap_mut().set_threshold(1024 * 1024);

    let code = r#"
        (function() {
            const N = 2000;
            let chain = { val: 0 };
            for (let i = 1; i < N; i++) {
                chain = { ...chain, val: i, next: chain };
            }
            let cur = chain;
            let seen = 0;
            while (cur && seen < 10) {
                seen++;
                cur = cur.next;
            }
            return chain.val;
        })();
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 1999);
}

#[test]
fn test_gc_pressure_with_strings() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    ctx.runtime_mut().gc_heap_mut().set_threshold(512 * 1024);

    let code = r#"
        (function() {
            const N = 3000;
            let strings = [];
            for (let i = 0; i < N; i++) {
                strings.push("prefix-" + i + "-suffix");
            }
            let obj = {};
            for (let i = 0; i < N; i++) {
                obj["key" + i] = strings[i];
            }
            let count = 0;
            for (let k in obj) {
                if (obj[k].indexOf("prefix-") === 0) count++;
            }
            return count;
        })();
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 3000);
}

#[test]
fn test_rapid_gc_cycles() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();
    ctx.runtime_mut().gc_heap_mut().set_threshold(65536);

    let code = r#"
        (function() {
            const OUTER = 10;
            const INNER = 5000;
            let accumulator = 0;
            for (let o = 0; o < OUTER; o++) {
                let temp = [];
                for (let i = 0; i < INNER; i++) {
                    temp.push({ num: i, str: String(i), arr: [i, i+1, i+2] });
                }
                for (let i = 0; i < INNER; i++) {
                    accumulator += temp[i].num;
                }
            }
            return accumulator;
        })();
    "#;

    let result = eval(&mut ctx, code).expect("Execution failed");
    assert_eq!(result.get_int(), 10 * (0..5000).sum::<i64>());
}
