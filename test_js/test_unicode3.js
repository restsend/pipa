// Debug Unicode regex
var r = /\p{Lu}/v;
var result = r.test("A");
console.log("Type of result:", typeof result);
console.log("Result value:", result);
