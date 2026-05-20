use pipa::parse_to_ast;

#[test]
fn test_string_with_paren() {
    let code = r#"var x = "(";"#;
    let result = parse_to_ast(code);
    assert!(result.is_ok(), "Should parse string with (: {:?}", result);
}

#[test]
fn test_string_with_dash() {
    let code = r#"if (s == "-") { return true; }"#;
    let result = parse_to_ast(code);
    assert!(result.is_ok(), "Should parse string with -: {:?}", result);
}

#[test]
fn test_string_with_bracket() {
    let code = r#"var arr = "[";"#;
    let result = parse_to_ast(code);
    assert!(result.is_ok(), "Should parse string with [: {:?}", result);
}

#[test]
fn test_string_with_brace() {
    let code = r#"var obj = "{";"#;
    let result = parse_to_ast(code);
    assert!(result.is_ok(), "Should parse string with {{: {:?}", result);
}

#[test]
fn test_string_with_comparison_ops() {
    let code = r#"var a = "<", b = ">", c = "=";"#;
    let result = parse_to_ast(code);
    assert!(
        result.is_ok(),
        "Should parse string with comparison ops: {:?}",
        result
    );
}

#[test]
fn test_string_in_function_call() {
    let code = r#"console.log("(");"#;
    let result = parse_to_ast(code);
    assert!(
        result.is_ok(),
        "Should parse function call with string: {:?}",
        result
    );
}

#[test]
fn test_complex_case_from_combined_js() {
    let code = r#"
        sc_Pair.prototype.sc_toWriteOrDisplayString = function(writeOrDisplay) {
            var current = this;
            var res = "(";
            while(true) {
                res += writeOrDisplay(current.car);
                if (isPair(current.cdr)) {
                    res += " ";
                    current = current.cdr;
                }
            }
        }
    "#;
    let result = parse_to_ast(code);
    assert!(
        result.is_ok(),
        "Should parse complex case from combined.js: {:?}",
        result
    );
}
