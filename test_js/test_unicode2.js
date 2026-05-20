// Test Unicode regex - simplified
console.log("Test 1: Uppercase A");
var r1 = /\p{Lu}/v;
if (r1.test("A")) { console.log("PASS"); } else { console.log("FAIL"); }

console.log("Test 2: Lowercase a");
var r2 = /\p{Ll}/v;
if (r2.test("a")) { console.log("PASS"); } else { console.log("FAIL"); }

console.log("Test 3: Digit 5");
var r3 = /\p{Nd}/v;
if (r3.test("5")) { console.log("PASS"); } else { console.log("FAIL"); }

console.log("Test 4: Not a number");
if (!r3.test("x")) { console.log("PASS"); } else { console.log("FAIL"); }

console.log("Done!");
