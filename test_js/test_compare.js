// Compare v-flag regex with non-v-flag
var r1 = /\p{Lu}/v;
var x1 = r1.test("A");
console.log("v-flag regex:", x1);

var r2 = /A/;
var x2 = r2.test("A");
console.log("non-v-flag regex:", x2);
