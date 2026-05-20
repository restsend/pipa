function GlobalFunc() {
    return "success";
}

var direct = GlobalFunc();
console.log("direct:", direct);
console.log("result:", direct === "success");
var result = direct === "success";
result;
