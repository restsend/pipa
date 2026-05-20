// Test script for Pipa eval binary
console.log("Hello from Pipa!");

let x = 42;
console.log("x =", x);

// Test BigInt
let big = 123456789012345678901234567890n;
console.log("BigInt:", big);

// Test setTimeout (will print after 100ms)
console.log("Starting timer...");
setTimeout(function() {
    console.log("Timer callback fired!");
}, 100);

console.log("Script loaded, waiting for timers...");
