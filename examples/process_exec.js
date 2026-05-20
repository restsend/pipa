// examples/process_exec.js
// Demonstrate process.exec() - spawn commands and capture output
//
// Run: pipa --features process examples/process_exec.js

import("pipa:process").then(function(p) {
    // Simple command
    return p.exec("echo", ["hello from pipa"]).then(function(r) {
        console.log("=== exec echo ===");
        console.log("stdout:", JSON.stringify(r.stdout));
        console.log("stderr:", JSON.stringify(r.stderr));
        console.log("code:", r.code);

        // Command with stderr output
        return p.exec("sh", ["-c", "echo stdout_line; echo stderr_line >&2"]).then(function(r2) {
            console.log();
            console.log("=== exec sh (stdout + stderr) ===");
            console.log("stdout:", JSON.stringify(r2.stdout));
            console.log("stderr:", JSON.stringify(r2.stderr));
            console.log("code:", r2.code);

            // Command that fails (use .then with error handler)
            return p.exec("nonexistent_command_xyz", []).then(
                function(r3) {
                    console.log();
                    console.log("=== exec nonexistent (unexpected success) ===");
                    console.log("unexpected:", JSON.stringify(r3));
                },
                function(err) {
                    console.log();
                    console.log("=== exec nonexistent (expected failure) ===");
                    console.log("error:", typeof err === "object" ? err.message || JSON.stringify(err) : err);
                    console.log("All exec tests OK");
                }
            );
        });
    });
}).catch(function(e) {
    console.log("ERROR:", e);
});
