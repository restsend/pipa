// Simple test - fetch blocks, so use chained .then
console.log("fetch test starting...");

function testBasicFetch() {
    console.log("--- Test 1: Basic GET ---");
    fetch("http://example.com/")
        .then(function(r) {
            console.log("Status: " + r.status);
            console.log("StatusText: " + r.statusText);
            console.log("OK: " + r.ok);
            return r.text();
        })
        .then(function(body) {
            console.log("Body length: " + body.length);
            if (body.length > 0) {
                console.log("First 100: " + body.substring(0, 100));
            }
            console.log("Test 1 PASSED");
        })
        .catch(function(e) {
            console.log("Test 1 FAILED: " + e);
        });
}

testBasicFetch();
