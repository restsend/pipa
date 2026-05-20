// examples/process_chain.js
// Demonstrate chaining multiple exec calls, pipe-like workflow
//
// Run: pipa --features process examples/process_chain.js

import("pipa:process").then(function(p) {
    console.log("=== Chained exec workflow ===");

    // Step 1: get current directory
    p.exec("pwd", []).then(function(r1) {
        console.log("1. current directory:", r1.stdout.trim());

        // Step 2: list files (top 5 entries)
        return p.exec("ls", ["-1"]).then(function(r2) {
            console.log("2. directory listing:");
            var lines = r2.stdout.split("\n");
            for (var i = 0; i < 5 && i < lines.length; i++) {
                console.log("   " + lines[i]);
            }

            // Step 3: count files
            return p.exec("sh", ["-c", "ls -1 | wc -l"]).then(function(r3) {
                console.log("3. total files (wc -l):", r3.stdout.trim());
                console.log("Chained workflow OK");
            });
        });
    }).catch(function(e) {
        console.log("FAIL:", e);
    });
}).catch(function(e) {
    console.log("ERROR:", e);
});
