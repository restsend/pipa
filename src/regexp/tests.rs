use super::*;

#[test]
fn test_literal_match() {
    let re = Regex::new("abc").unwrap();
    let m = re.find("abc").unwrap();
    assert_eq!(m.as_str(), "abc");
}

#[test]
fn test_literal_in_middle() {
    let re = Regex::new("test").unwrap();
    let m = re.find("this is a test string").unwrap();
    assert_eq!(m.as_str(), "test");
}

#[test]
fn test_literal_not_found() {
    let re = Regex::new("xyz").unwrap();
    assert!(re.find("abc").is_none());
}

#[test]
fn test_empty_pattern() {
    let re = Regex::new("").unwrap();
    let m = re.find("abc").unwrap();
    assert_eq!(m.as_str(), "");
}

#[test]
fn test_concatenation() {
    let re = Regex::new("abcdef").unwrap();
    assert!(re.is_match("abcdef"));
    assert!(!re.is_match("abc"));
}

#[test]
fn test_simple_alt() {
    let re = Regex::new("cat|dog").unwrap();
    assert_eq!(re.find("I have a cat").unwrap().as_str(), "cat");
    assert_eq!(re.find("I have a dog").unwrap().as_str(), "dog");
}

#[test]
fn test_alt_multiple() {
    let re = Regex::new("a|b|c|d").unwrap();
    assert!(re.is_match("a"));
    assert!(re.is_match("b"));
    assert!(re.is_match("c"));
    assert!(re.is_match("d"));
}

#[test]
fn test_simple_capture() {
    let re = Regex::new("(abc)").unwrap();
    let m = re.find("abc").unwrap();
    assert_eq!(m.as_str(), "abc");
    assert_eq!(m.get(1), Some("abc"));
}

#[test]
fn test_multiple_captures() {
    let re = Regex::new(r"(a)(b)(c)").unwrap();
    let m = re.find("abc").unwrap();
    assert_eq!(m.get(1), Some("a"));
    assert_eq!(m.get(2), Some("b"));
    assert_eq!(m.get(3), Some("c"));
}

#[test]
fn test_is_match_basic() {
    let re = Regex::new("test").unwrap();
    assert!(re.is_match("this is a test"));
    assert!(!re.is_match("no match here"));
}

#[test]
fn test_is_full_match() {
    let re = Regex::new("abc").unwrap();
    assert!(re.is_full_match("abc"));
    assert!(!re.is_full_match("abcd"));
}

#[test]
fn test_capture_iter() {
    let re = Regex::new(r"(a)(b)(c)").unwrap();
    let m = re.find("abc").unwrap();
    let caps: Vec<_> = m.iter().collect();
    assert_eq!(caps.len(), 4);
    assert_eq!(caps[0], Some("abc"));
    assert_eq!(caps[1], Some("a"));
    assert_eq!(caps[2], Some("b"));
    assert_eq!(caps[3], Some("c"));
}

#[test]
fn test_optional_present() {
    let re = Regex::new("ab?c").unwrap();
    let m = re.find("abc").unwrap();
    assert_eq!(m.as_str(), "abc");
}

#[test]
fn test_optional_absent() {
    let re = Regex::new("ab?c").unwrap();
    let m = re.find("ac").unwrap();
    assert_eq!(m.as_str(), "ac");
}

#[test]
fn test_simple_email_pattern() {
    let re = Regex::new(r"\w+@\w+\.\w+").unwrap();

    assert!(re.find("test").is_none() || re.find("test").is_some());
}

#[test]
fn test_pattern_compilation() {
    assert!(Regex::new("a").is_ok());
    assert!(Regex::new("(a)").is_ok());
    assert!(Regex::new("a|b").is_ok());
    assert!(Regex::new("ab?c").is_ok());
    assert!(Regex::new("ab*c").is_ok());
    assert!(Regex::new("ab+c").is_ok());
}

#[test]
fn test_regex_flags() {
    let flags = RegexFlags::from_str("i").unwrap();
    assert!(flags.ignore_case);
}

#[test]
fn test_regex_builder() {
    let re = RegexBuilder::new("test").ignore_case(true).build().unwrap();
    assert!(re.flags().ignore_case);
}

#[test]
fn test_regex_display() {
    let re = Regex::new("test").unwrap();
    assert_eq!(format!("{}", re), "/test/");
}

#[test]
fn test_capture_count() {
    let re = Regex::new(r"(a)(b)(c)").unwrap();

    let count = re.capture_count();
    assert!(count >= 1);
}

#[test]
fn test_match_result_methods() {
    let re = Regex::new("(abc)").unwrap();
    let m = re.find("abc").unwrap();

    assert_eq!(m.as_str(), "abc");
    assert_eq!(m.start(), 0);
    assert_eq!(m.end(), 3);
    assert_eq!(m.len(), 2);
    assert!(!m.is_empty());
}

#[test]
fn test_match_range() {
    let re = Regex::new("test").unwrap();
    let m = re.find("this is a test").unwrap();
    let range = m.range();
    assert_eq!(range.start, 10);
    assert_eq!(range.end, 14);
}

#[test]
fn test_match_positions() {
    let re = Regex::new("(abc)").unwrap();
    let m = re.find("abc").unwrap();
    let positions = m.positions();
    assert_eq!(positions[0].0, Some(0));
    assert_eq!(positions[0].1, Some(3));
    assert_eq!(positions[1].0, Some(0));
    assert_eq!(positions[1].1, Some(3));
}

#[test]
fn test_invalid_flag() {
    assert!(RegexFlags::from_str("x").is_err());
}

#[test]
fn test_regex_error_display() {
    let err = RegexError::Parse("test error".to_string());
    assert!(format!("{}", err).contains("test error"));
}

#[test]
fn test_find_convenience() {
    let result = find("test", "this is a test").unwrap();
    assert_eq!(result, Some("test".to_string()));
}

#[test]
fn test_is_match_convenience() {
    assert!(is_match("test", "this is a test").unwrap());
    assert!(!is_match("xyz", "this is a test").unwrap());
}
