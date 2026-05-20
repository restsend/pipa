
use pipa::{JSRuntime, eval};

#[test]
fn test_global_function_accessible_from_nested_call() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = std::fs::read_to_string("tests/global_var_resolution_test.js")
        .expect("Failed to read test file");

    let result = eval(&mut ctx, &code);
    println!("Result: {:?}", result);
    if let Ok(val) = &result {
        println!("Value: {:?}", val);
        println!("is_bool: {}", val.is_bool());
        println!("is_undefined: {}", val.is_undefined());
    }
    assert!(result.is_ok(), "Test should complete: {:?}", result);
    let val = result.unwrap();
    assert!(
        val.is_bool() && val.get_bool(),
        "Global function call should work, got {:?}",
        val
    );
}

#[test]
fn test_global_function_through_continuation() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
        "use strict";

        // Define global function
        function GlobalFunc() {
            return "success";
        }

        // Simulate a continuation pattern like in BenchmarkSuite
        function createContinuation() {
            var called = false;
            var result = null;

            function run() {
                // This should be able to access GlobalFunc
                result = GlobalFunc();
                called = true;
                return null; // No more continuations
            }

            function getResult() {
                return result;
            }

            return { run, getResult };
        }

        // Execute continuation
        var state = createContinuation();
        var cont = state.run;
        while (cont) {
            cont = cont();
        }

        state.getResult() === "success";
    "#;

    let result = eval(&mut ctx, code);
    assert!(result.is_ok(), "Test should complete: {:?}", result);
    assert!(
        result.unwrap().get_bool(),
        "Global function should be accessible from continuation"
    );
}

#[test]
fn test_combined_js_scenario() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
        "use strict";

        // Minimal Benchmark framework
        function Benchmark(name, run, setup, tearDown) {
            this.name = name;
            this.run = run;
            this.Setup = setup || function(){};
            this.TearDown = tearDown || function(){};
        }

        function BenchmarkSuite(name, reference, benchmarks) {
            this.name = name;
            this.reference = reference;
            this.benchmarks = benchmarks;
        }

        BenchmarkSuite.prototype.RunStep = function(runner) {
            var suite = this;
            var length = this.benchmarks.length;
            var index = 0;
            var data;

            function RunNextSetup() {
                if (index < length) {
                    suite.benchmarks[index].Setup();
                    return RunNextBenchmark;
                }
                return null;
            }

            function RunNextBenchmark() {
                try {
                    suite.benchmarks[index].run();
                } catch (e) {
                    return null;
                }
                return RunNextTearDown;
            }

            function RunNextTearDown() {
                suite.benchmarks[index++].TearDown();
                return RunNextSetup;
            }

            return RunNextSetup;
        };

        // Richards-like benchmark
        function runRichards() {
            var x = 0;
            for (var i = 0; i < 1000; i++) {
                x += i;
            }
            return x;
        }

        var Richards = new BenchmarkSuite('Richards', 35302, [
            new Benchmark("Richards", runRichards)
        ]);

        // DeltaBlue-like
        Object.prototype.inheritsFrom = function (shuper) {
            function Inheriter() { }
            Inheriter.prototype = shuper.prototype;
            this.prototype = new Inheriter();
            this.superConstructor = shuper;
        };

        function UnaryConstraint(v, strength) {
            this.myOutput = v;
            this.strength = strength;
        }

        function EditConstraint(v, str) {
            EditConstraint.superConstructor.call(this, v, str);
        }
        EditConstraint.inheritsFrom(UnaryConstraint);

        function chainTest(n) {
            // This is the critical access
            var edit = new EditConstraint("test", "preferred");
            return edit;
        }

        function deltaBlue() {
            return chainTest(100);
        }

        var DeltaBlue = new BenchmarkSuite('DeltaBlue', 66118, [
            new Benchmark('DeltaBlue', deltaBlue)
        ]);

        // Run Richards
        var rCont = Richards.RunStep({});
        while (rCont) {
            rCont = rCont();
        }

        // Run DeltaBlue
        var dCont = DeltaBlue.RunStep({});
        while (dCont) {
            dCont = dCont();
        }

        "success";
    "#;

    let result = eval(&mut ctx, code);
    assert!(
        result.is_ok(),
        "Combined.js scenario should work: {:?}",
        result
    );
    
    let val = result.unwrap();
    assert!(val.is_string() && val.get_atom() == ctx.intern("success"), "Result should be 'success'");
}
