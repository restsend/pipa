use pipa::{JSRuntime, eval};

#[test]
fn test_array_creation() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var arr = [1, 2, 3]").is_ok());
    assert!(eval(&mut ctx, "var empty = []").is_ok());
    assert!(eval(&mut ctx, "var arr = new Array(5)").is_ok());
}

#[test]
fn test_array_push() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "var arr = [1]; arr.push(2)").is_ok());
}

#[test]
fn test_array_concat() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "[1, 2].concat([3, 4])").is_ok());
}

#[test]
fn test_array_index_of() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "[1, 2, 3].indexOf(2)").is_ok());
}

#[test]
fn test_array_length() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "[1, 2, 3].length").is_ok());
    assert!(
        eval(
            &mut ctx,
            "if ([1, 2, 3].length !== 3) { throw new Error('array length mismatch'); }"
        )
        .is_ok()
    );
}

#[test]
fn test_array_slice() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "[1, 2, 3, 4].slice(1, 3)").is_ok());
}

#[test]
fn test_array_push_pop() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(&mut ctx, "var arr = [1, 2]; arr.push(3); arr.length");
    assert!(result.is_ok());
}

#[test]
fn test_array_includes() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(&mut ctx, "[1, 2, 3].includes(2)").is_ok());
    assert!(eval(&mut ctx, "[1, 2, 3].includes(5)").is_ok());
}

#[test]
fn test_array_for_each() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        "var sum = 0; [1, 2, 3].forEach(function(x) { sum = sum + x; }); if (sum !== 6) { throw new Error('forEach sum mismatch'); }",
    );
    if let Err(ref e) = result {
        println!("DEBUG ERROR: {}", e);
    }
    assert!(result.is_ok());
}

#[test]
fn test_array_map() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(
        &mut ctx,
        "var arr = [1, 2, 3].map(function(x) { return x * 2; }); if (arr.length !== 3 || arr[0] !== 2 || arr[1] !== 4 || arr[2] !== 6) { throw new Error('map result mismatch'); }",
    )
    .is_ok());
}

#[test]
fn test_array_filter() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(
        &mut ctx,
        "var arr = [1, 2, 3, 4, 5].filter(function(x) { return x > 2; }); if (arr.length !== 3 || arr[0] !== 3 || arr[1] !== 4 || arr[2] !== 5) { throw new Error('filter result mismatch'); }",
    )
    .is_ok());
}

#[test]
fn test_array_reduce() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(
        &mut ctx,
        "var a = [1, 2, 3, 4].reduce(function(acc, x) { return acc + x; }, 0); var b = [1, 2, 3].reduce(function(acc, x) { return acc + x; }); if (a !== 10 || b !== 6) { throw new Error('reduce result mismatch'); }",
    )
    .is_ok());
}

#[test]
fn test_array_map_chain() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    let result = eval(
        &mut ctx,
        "[1, 2, 3].map(function(x) { return x * 2; }).map(function(x) { return x + 1; });",
    );
    assert!(result.is_ok());
}

#[test]
fn test_array_every() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(
        &mut ctx,
        "var a = [2, 4, 6].every(function(x) { return x % 2 == 0; }); var b = [2, 5, 6].every(function(x) { return x % 2 == 0; }); var c = [].every(function(x) { return false; }); if (a !== true || b !== false || c !== true) { throw new Error('every result mismatch'); }",
    )
    .is_ok());
}

#[test]
fn test_array_some() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(
        &mut ctx,
        "var a = [1, 4, 5].some(function(x) { return x % 2 == 1; }); var b = [2, 4, 6].some(function(x) { return x % 2 == 1; }); var c = [].some(function(x) { return true; }); if (a !== true || b !== false || c !== false) { throw new Error('some result mismatch'); }",
    )
    .is_ok());
}

#[test]
fn test_array_find() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(
        &mut ctx,
        "var a = [1, 3, 5, 7].find(function(x) { return x > 4; }); var b = [1, 2, 3].find(function(x) { return x > 10; }); var c = [].find(function(x) { return true; }); if (a !== 5 || b !== undefined || c !== undefined) { throw new Error('find result mismatch'); }",
    )
    .is_ok());
}

#[test]
fn test_array_find_index() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(
        &mut ctx,
        "var a = [1, 3, 5, 7].findIndex(function(x) { return x > 4; }); var b = [1, 2, 3].findIndex(function(x) { return x > 10; }); var c = [].findIndex(function(x) { return true; }); if (a !== 2 || b !== -1 || c !== -1) { throw new Error('findIndex result mismatch'); }",
    )
    .is_ok());
}

#[test]
fn test_array_reduce_right() {
    let mut runtime = JSRuntime::new();
    let mut ctx = runtime.new_context();

    assert!(eval(
        &mut ctx,
        "var a = [1, 2, 3, 4].reduceRight(function(acc, x) { return acc + x; }, 0); var b = [1, 2, 3].reduceRight(function(acc, x) { return acc - x; }); var c = [[1,2],[3,4],[5,6]].reduceRight(function(acc, x) { return acc.concat(x); }, []); if (a !== 10 || b !== 0 || c.length !== 6 || c[0] !== 5 || c[5] !== 2) { throw new Error('reduceRight result mismatch'); }",
    )
    .is_ok());
}
