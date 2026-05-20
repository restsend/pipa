// Test Unicode regex with v flag
console.log("Testing Unicode regex...");

// Test uppercase letter matching
var upperRx = /\p{General_Category=Uppercase_Letter}/v;
console.log("Uppercase regex test:");
console.log("  'A' matches:", upperRx.test("A"));
console.log("  'z' matches:", upperRx.test("z"));
console.log("  '1' matches:", upperRx.test("1"));

// Test lowercase letter matching
var lowerRx = /\p{Ll}/v;
console.log("\nLowercase regex test:");
console.log("  'a' matches:", lowerRx.test("a"));
console.log("  'Z' matches:", lowerRx.test("Z"));

// Test decimal number
var numRx = /\p{Nd}/v;
console.log("\nDecimal number regex test:");
console.log("  '0' matches:", numRx.test("0"));
console.log("  '9' matches:", numRx.test("9"));
console.log("  'a' matches:", numRx.test("a"));

// Test short form
var shortUpper = /\p{Lu}/v;
console.log("\nShort form (Lu) test:");
console.log("  'B' matches:", shortUpper.test("B"));

// Test Letter (L)
var letterRx = /\p{Letter}/v;
console.log("\nLetter property test:");
console.log("  'A' matches:", letterRx.test("A"));
console.log("  'a' matches:", letterRx.test("a"));

console.log("\nAll tests completed!");
