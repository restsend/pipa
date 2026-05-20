// examples/process_basic.js
// Demonstrate process.argv, argc, env, cwd, exit
//
// Run: pipa --features process examples/process_basic.js

import("pipa:process").then(function(p) {
    console.log("=== process.argv ===");
    console.log("argc:", p.argc);
    console.log("argv:", JSON.stringify(p.argv));

    console.log();
    console.log("=== process.cwd() ===");
    console.log("cwd:", p.cwd());

    console.log();
    console.log("=== process.env ===");
    console.log("env.HOME:", p.env.HOME);
    console.log("env.PATH:", typeof p.env.PATH);
    console.log("env.USER:", p.env.USER || "(not set)");

    console.log();
    console.log("Basic process info OK");
}).catch(function(e) {
    console.log("ERROR:", e);
});
