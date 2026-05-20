use pipa::compiler::parser::Parser;

fn parse_ok(src: &str) {
    Parser::new(src)
        .parse()
        .unwrap_or_else(|e| panic!("parse failed: {e}\nsource:\n{src}"));
}

#[test]
fn parses_arrow_param_object_destructuring() {
    parse_ok(
        r#"
const actualComponents = [{ type: "year" }];
actualComponents.find(({ type }) => type === "year");
"#,
    );
}

#[test]
fn parses_for_of_left_side_without_in_operator() {
    parse_ok(
        r#"
const testData = [1, 2, 3];
for (let currency of testData) {
  currency;
}
"#,
    );
}

#[test]
fn parses_trailing_comment_at_eof() {
    parse_ok(
        r#"
const x = 1;
// trailing comment without newline"#,
    );
}
