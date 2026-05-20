use pipa::{JSRuntime, eval};

fn main() {
    let code = r#"
var print = console.log;
function BenchmarkSuite(name, reference, benchmarks) {
    this.name = name; this.reference = reference;
    this.benchmarks = benchmarks;
    BenchmarkSuite.suites.push(this);
}
BenchmarkSuite.suites = [];
BenchmarkSuite.scores = [];
BenchmarkSuite.GeometricMean = function (numbers) {
    var log = 0;
    for (var i = 0; i < numbers.length; i++) log += Math.log(numbers[i]);
    return Math.pow(Math.E, log / numbers.length);
};
BenchmarkSuite.FormatScore = function (value) {
    return value > 100 ? value.toFixed(0) : value.toPrecision(3);
};
BenchmarkSuite.RunSuites = function (runner) {
    var continuation = null, suites = BenchmarkSuite.suites, length = suites.length, index = 0;
    function RunStep() {
        print("RunStep start");
        while (continuation || index < length) {
            print("RunStep loop, continuation=" + continuation + " index=" + index);
            if (continuation) {
                print("RunStep calling continuation");
                continuation = continuation();
                print("RunStep continuation returned: " + continuation);
            } else {
                var suite = suites[index++];
                print("RunStep got suite, calling RunStep");
                continuation = suite.RunStep(runner);
                print("RunStep suite.RunStep returned: " + continuation);
            }
        }
        if (runner.NotifyScore) runner.NotifyScore(BenchmarkSuite.FormatScore(100 * BenchmarkSuite.GeometricMean(BenchmarkSuite.scores)));
    }
    print("RunSuites calling RunStep");
    RunStep();
    print("RunStep done");
};
BenchmarkSuite.prototype.RunStep = function (runner) {
    this.results = []; this.runner = runner;
    var length = this.benchmarks.length;
    var index = 0;
    var data;
    var suite = this;
    function RunNextSetup() {
        print("RunNextSetup");
        if (index < length) {
            try { suite.benchmarks[index].Setup(); }
            catch (e) { suite.NotifyError(e); return null; }
            return RunNextBenchmark;
        }
        suite.NotifyResult(); return null;
    }
    function RunNextBenchmark() {
        print("RunNextBenchmark");
        try { data = suite.RunSingleBenchmark(suite.benchmarks[index], data); }
        catch (e) { suite.NotifyError(e); return null; }
        return (data == null) ? RunNextTearDown : RunNextBenchmark();
    }
    function RunNextTearDown() {
        print("RunNextTearDown");
        try { suite.benchmarks[index++].TearDown(); }
        catch (e) { suite.NotifyError(e); return null; }
        return RunNextSetup;
    }
    return RunNextSetup;
};
BenchmarkSuite.prototype.NotifyResult = function () {
    var mean = BenchmarkSuite.GeometricMean(this.results);
    BenchmarkSuite.scores.push(this.reference / mean);
    if (this.runner.NotifyResult) this.runner.NotifyResult(this.name, BenchmarkSuite.FormatScore(100 * (this.reference / mean)));
};
BenchmarkSuite.prototype.NotifyError = function (error) {
    if (this.runner.NotifyError) this.runner.NotifyError(this.name, error);
};
BenchmarkSuite.prototype.RunSingleBenchmark = function (benchmark, data) {
    print("RunSingleBenchmark, data=" + data);
    function Measure(data) {
        var elapsed = 0;
        var start = {};
        for (var n = 0; n < 2; n++) {
            benchmark.run();
        }
        if (data != null) { data.runs += n; data.elapsed += elapsed; }
    }
    if (data == null) { Measure(null); return { runs: 0, elapsed: 0 }; }
    else {
        Measure(data);
        print("after Measure, data.runs=" + data.runs);
        if (data.runs < 32) return data;
        var usec = (data.elapsed * 1000) / data.runs;
        this.NotifyStep(new BenchmarkResult(benchmark, usec));
        return null;
    }
};
function BenchmarkResult(benchmark, time) {
    this.benchmark = benchmark;
    this.time = time;
}
BenchmarkResult.prototype.valueOf = function () { return this.time; };
BenchmarkSuite.prototype.NotifyStep = function (result) {
    this.results.push(result);
    if (this.runner.NotifyStep) this.runner.NotifyStep(result.benchmark.name);
};

function runRichards() { return 1; }
var Richards = new BenchmarkSuite('Richards', 35302, [{
    name: 'Richards',
    run: runRichards,
    Setup: function() { print("Setup"); },
    TearDown: function() { print("TearDown"); },
}]);

print("about to call RunSuites");
BenchmarkSuite.RunSuites({
    NotifyError: function(name, error) { print("ERROR", name, error); },
    NotifyResult: function(name, result) { print("RESULT", name, result); },
    NotifyScore: function(score) { print("SCORE", score); },
});
print("done");
"#;
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    match eval(&mut ctx, code) {
        Ok(v) => println!("OK: {:?}", v),
        Err(e) => println!("ERR: {}", e),
    }
}
