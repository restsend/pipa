use super::engine::Match;
use super::memchr::{find_digit, find_space, find_word_char};
use super::pattern_matcher::{CharClassType, PatternFingerprint, extract_fingerprint};
use crate::util::memchr::{memchr, memmem};

#[derive(Clone)]
pub struct SimpleDFA {
    pattern_type: PatternType,

    literal: Option<String>,

    finder_bytes: Option<Box<[u8]>>,
}

#[derive(Clone, Debug, PartialEq)]
enum PatternType {
    Literal,
    Digits,
    Words,
    Spaces,
    SingleChar,
    SingleCharStar(char),
    SingleCharPlus(char),

    StartAnchored(String),

    EndAnchored(String),

    EmailLike {
        local_class: CharClassType,
        domain_class: CharClassType,
        tld_class: CharClassType,
    },
}

impl SimpleDFA {
    pub fn new(pattern: &str) -> Option<Self> {
        match pattern {
            r"\d+" => Some(Self {
                pattern_type: PatternType::Digits,
                literal: None,
                finder_bytes: None,
            }),
            r"\w+" => Some(Self {
                pattern_type: PatternType::Words,
                literal: None,
                finder_bytes: None,
            }),
            r"\s+" => Some(Self {
                pattern_type: PatternType::Spaces,
                literal: None,
                finder_bytes: None,
            }),
            _ => {
                if is_pure_literal(pattern) {
                    let needle: Box<[u8]> = pattern.as_bytes().to_vec().into_boxed_slice();
                    return Some(Self {
                        pattern_type: PatternType::Literal,
                        literal: Some(pattern.to_string()),
                        finder_bytes: Some(needle),
                    });
                }

                if pattern.len() >= 1 && pattern.len() <= 2 {
                    let chars: Vec<char> = pattern.chars().collect();
                    if chars.len() == 1 && chars[0].is_ascii() {
                        return Some(Self {
                            pattern_type: PatternType::SingleChar,
                            literal: Some(pattern.to_string()),
                            finder_bytes: None,
                        });
                    } else if chars.len() == 2 && chars[0].is_ascii() {
                        match chars[1] {
                            '*' => {
                                return Some(Self {
                                    pattern_type: PatternType::SingleCharStar(chars[0]),
                                    literal: Some(pattern.to_string()),
                                    finder_bytes: None,
                                });
                            }
                            '+' => {
                                return Some(Self {
                                    pattern_type: PatternType::SingleCharPlus(chars[0]),
                                    literal: Some(pattern.to_string()),
                                    finder_bytes: None,
                                });
                            }
                            _ => {}
                        }
                    }
                }

                if pattern.starts_with('^') && pattern.len() > 1 {
                    let inner = &pattern[1..];
                    if is_pure_literal(inner) {
                        return Some(Self {
                            pattern_type: PatternType::StartAnchored(inner.to_string()),
                            literal: Some(inner.to_string()),
                            finder_bytes: None,
                        });
                    }
                }

                if pattern.ends_with('$') && pattern.len() > 1 {
                    let inner = &pattern[..pattern.len() - 1];
                    if is_pure_literal(inner) {
                        return Some(Self {
                            pattern_type: PatternType::EndAnchored(inner.to_string()),
                            literal: Some(inner.to_string()),
                            finder_bytes: None,
                        });
                    }
                }

                if let Ok(ast) = super::parser::parse(pattern, 0) {
                    let fingerprint = extract_fingerprint(&ast);

                    if let PatternFingerprint::EmailLike {
                        local_part,
                        domain_part,
                        tld_part,
                    } = fingerprint
                    {
                        return Some(Self {
                            pattern_type: PatternType::EmailLike {
                                local_class: local_part,
                                domain_class: domain_part,
                                tld_class: tld_part,
                            },
                            literal: None,
                            finder_bytes: None,
                        });
                    }
                }

                None
            }
        }
    }

    pub fn find(&self, input: &str) -> Option<Match> {
        match &self.pattern_type {
            PatternType::Literal => self.find_literal(input),
            PatternType::Digits => self.find_digits(input),
            PatternType::Words => self.find_words(input),
            PatternType::Spaces => self.find_spaces(input),
            PatternType::SingleChar => self.find_single_char(input),
            PatternType::SingleCharStar(c) => self.find_single_char_star(input, *c),
            PatternType::SingleCharPlus(c) => self.find_single_char_plus(input, *c),
            PatternType::StartAnchored(lit) => self.find_start_anchored(input, lit),
            PatternType::EndAnchored(lit) => self.find_end_anchored(input, lit),
            PatternType::EmailLike {
                local_class,
                domain_class,
                tld_class,
            } => self.find_email_like(input, *local_class, *domain_class, *tld_class),
        }
    }

    #[inline(always)]
    pub fn is_match(&self, input: &str) -> bool {
        match &self.pattern_type {
            PatternType::Literal => {
                let literal = match self.literal.as_ref() {
                    Some(l) => l,
                    None => return false,
                };
                let lit_bytes = literal.as_bytes();
                let input_bytes = input.as_bytes();
                let lit_len = lit_bytes.len();
                let input_len = input_bytes.len();

                if lit_len == 0 {
                    return true;
                }
                if lit_len > input_len {
                    return false;
                }

                if lit_len == 1 {
                    memchr(lit_bytes[0], input_bytes).is_some()
                } else if let Some(ref bytes) = self.finder_bytes {
                    memmem::find(input_bytes, bytes).is_some()
                } else {
                    memmem::find(input_bytes, lit_bytes).is_some()
                }
            }
            PatternType::StartAnchored(lit) => input.starts_with(lit),
            PatternType::EndAnchored(lit) => input.ends_with(lit),
            PatternType::SingleChar => {
                if let Some(lit) = self.literal.as_ref() {
                    if let Some(c) = lit.chars().next() {
                        return memchr(c as u8, input.as_bytes()).is_some();
                    }
                }
                false
            }
            PatternType::SingleCharStar(_) => true,
            PatternType::SingleCharPlus(c) => memchr(*c as u8, input.as_bytes()).is_some(),
            _ => self.find(input).is_some(),
        }
    }

    pub fn find_all(&self, input: &str) -> Vec<Match> {
        let mut matches = Vec::new();
        let mut pos = 0;
        let bytes = input.as_bytes();

        while pos < bytes.len() {
            if let Some(m) = self.find_from(input, pos) {
                let match_end = m.end;
                matches.push(m);
                if match_end <= pos {
                    pos += 1;
                } else {
                    pos = match_end;
                }
            } else {
                break;
            }
        }

        matches
    }

    #[inline(always)]
    fn find_literal(&self, input: &str) -> Option<Match> {
        let literal = self.literal.as_ref()?;
        let lit_bytes = literal.as_bytes();
        let input_bytes = input.as_bytes();
        let lit_len = lit_bytes.len();
        let input_len = input_bytes.len();

        if lit_len == 0 {
            return Some(Match {
                start: 0,
                end: 0,
                captures: vec![(Some(0), Some(0))],
            });
        }

        if lit_len > input_len {
            return None;
        }

        if lit_len == 1 {
            let b = lit_bytes[0];
            return memchr(b, input_bytes).map(|pos| Match {
                start: pos,
                end: pos + 1,
                captures: vec![(Some(pos), Some(pos + 1))],
            });
        }

        if let Some(ref bytes) = self.finder_bytes {
            return memmem::find(input_bytes, bytes).map(|pos| Match {
                start: pos,
                end: pos + lit_len,
                captures: vec![(Some(pos), Some(pos + lit_len))],
            });
        }

        let first = lit_bytes[0];
        let second = lit_bytes[1];
        let max_pos = input_len - lit_len;

        let mut pos = 0;
        while pos <= max_pos {
            if let Some(rel_pos) = memchr(first, &input_bytes[pos..max_pos + 1]) {
                let candidate = pos + rel_pos;

                if input_bytes[candidate + 1] == second {
                    if &input_bytes[candidate..candidate + lit_len] == lit_bytes {
                        return Some(Match {
                            start: candidate,
                            end: candidate + lit_len,
                            captures: vec![(Some(candidate), Some(candidate + lit_len))],
                        });
                    }
                }
                pos = candidate + 1;
            } else {
                break;
            }
        }

        None
    }

    #[inline(always)]
    fn find_digits(&self, input: &str) -> Option<Match> {
        let bytes = input.as_bytes();

        let start = find_digit(bytes)?;

        let len = bytes.len();
        let mut end = start + 1;

        while end + 8 <= len {
            let b0 = bytes[end];
            let b1 = bytes[end + 1];
            let b2 = bytes[end + 2];
            let b3 = bytes[end + 3];
            let b4 = bytes[end + 4];
            let b5 = bytes[end + 5];
            let b6 = bytes[end + 6];
            let b7 = bytes[end + 7];
            if Self::is_digit_byte(b0)
                && Self::is_digit_byte(b1)
                && Self::is_digit_byte(b2)
                && Self::is_digit_byte(b3)
                && Self::is_digit_byte(b4)
                && Self::is_digit_byte(b5)
                && Self::is_digit_byte(b6)
                && Self::is_digit_byte(b7)
            {
                end += 8;
            } else {
                break;
            }
        }

        while end < len && Self::is_digit_byte(bytes[end]) {
            end += 1;
        }

        Some(Match {
            start,
            end,
            captures: vec![(Some(start), Some(end))],
        })
    }

    #[inline(always)]
    fn is_digit_byte(b: u8) -> bool {
        b >= b'0' && b <= b'9'
    }

    #[inline(always)]
    fn find_words(&self, input: &str) -> Option<Match> {
        let bytes = input.as_bytes();

        let start = find_word_char(bytes)?;

        let len = bytes.len();
        let mut end = start + 1;

        while end + 8 <= len {
            let b0 = bytes[end];
            let b1 = bytes[end + 1];
            let b2 = bytes[end + 2];
            let b3 = bytes[end + 3];
            let b4 = bytes[end + 4];
            let b5 = bytes[end + 5];
            let b6 = bytes[end + 6];
            let b7 = bytes[end + 7];
            if Self::is_word_byte(b0)
                && Self::is_word_byte(b1)
                && Self::is_word_byte(b2)
                && Self::is_word_byte(b3)
                && Self::is_word_byte(b4)
                && Self::is_word_byte(b5)
                && Self::is_word_byte(b6)
                && Self::is_word_byte(b7)
            {
                end += 8;
            } else {
                break;
            }
        }

        while end < len && Self::is_word_byte(bytes[end]) {
            end += 1;
        }

        Some(Match {
            start,
            end,
            captures: vec![(Some(start), Some(end))],
        })
    }

    #[inline(always)]
    fn find_spaces(&self, input: &str) -> Option<Match> {
        let bytes = input.as_bytes();

        let start = find_space(bytes)?;

        let len = bytes.len();
        let mut end = start + 1;

        while end + 8 <= len {
            let b0 = bytes[end];
            let b1 = bytes[end + 1];
            let b2 = bytes[end + 2];
            let b3 = bytes[end + 3];
            let b4 = bytes[end + 4];
            let b5 = bytes[end + 5];
            let b6 = bytes[end + 6];
            let b7 = bytes[end + 7];
            if Self::is_space_byte(b0)
                && Self::is_space_byte(b1)
                && Self::is_space_byte(b2)
                && Self::is_space_byte(b3)
                && Self::is_space_byte(b4)
                && Self::is_space_byte(b5)
                && Self::is_space_byte(b6)
                && Self::is_space_byte(b7)
            {
                end += 8;
            } else {
                break;
            }
        }

        while end < len && Self::is_space_byte(bytes[end]) {
            end += 1;
        }

        Some(Match {
            start,
            end,
            captures: vec![(Some(start), Some(end))],
        })
    }

    #[inline(always)]
    fn is_word_byte(b: u8) -> bool {
        (b >= b'0' && b <= b'9')
            || (b >= b'A' && b <= b'Z')
            || (b >= b'a' && b <= b'z')
            || b == b'_'
    }

    #[inline(always)]
    fn is_space_byte(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0C | 0x0B)
    }

    #[inline(always)]
    fn find_single_char(&self, input: &str) -> Option<Match> {
        let literal = self.literal.as_ref()?;
        let c = literal.chars().next()?;
        let c_byte = c as u8;
        let bytes = input.as_bytes();

        memchr(c_byte, bytes).map(|pos| Match {
            start: pos,
            end: pos + 1,
            captures: vec![(Some(pos), Some(pos + 1))],
        })
    }

    #[inline(always)]
    fn find_single_char_star(&self, input: &str, c: char) -> Option<Match> {
        let bytes = input.as_bytes();
        let c_byte = c as u8;

        let len = bytes.len();
        let mut pos = 0;

        while pos < len && bytes[pos] == c_byte {
            pos += 1;
        }

        Some(Match {
            start: 0,
            end: pos,
            captures: vec![(Some(0), Some(pos))],
        })
    }

    #[inline(always)]
    fn find_single_char_plus(&self, input: &str, c: char) -> Option<Match> {
        let bytes = input.as_bytes();
        let c_byte = c as u8;
        let len = bytes.len();

        if len == 0 {
            return None;
        }

        let start = match memchr(c_byte, bytes) {
            Some(pos) => pos,
            None => return None,
        };

        let mut end = start + 1;

        while end + 8 <= len {
            if bytes[end] == c_byte
                && bytes[end + 1] == c_byte
                && bytes[end + 2] == c_byte
                && bytes[end + 3] == c_byte
                && bytes[end + 4] == c_byte
                && bytes[end + 5] == c_byte
                && bytes[end + 6] == c_byte
                && bytes[end + 7] == c_byte
            {
                end += 8;
            } else {
                break;
            }
        }

        while end < len && bytes[end] == c_byte {
            end += 1;
        }

        Some(Match {
            start,
            end,
            captures: vec![(Some(start), Some(end))],
        })
    }

    #[inline(always)]
    fn find_start_anchored(&self, input: &str, literal: &str) -> Option<Match> {
        let lit_bytes = literal.as_bytes();
        let input_bytes = input.as_bytes();

        if input_bytes.starts_with(lit_bytes) {
            Some(Match {
                start: 0,
                end: lit_bytes.len(),
                captures: vec![(Some(0), Some(lit_bytes.len()))],
            })
        } else {
            None
        }
    }

    #[inline(always)]
    fn find_end_anchored(&self, input: &str, literal: &str) -> Option<Match> {
        let lit_bytes = literal.as_bytes();
        let input_bytes = input.as_bytes();

        if lit_bytes.len() > input_bytes.len() {
            return None;
        }

        if input_bytes.ends_with(lit_bytes) {
            let start = input_bytes.len() - lit_bytes.len();
            Some(Match {
                start,
                end: input_bytes.len(),
                captures: vec![(Some(start), Some(input_bytes.len()))],
            })
        } else {
            None
        }
    }

    #[inline(always)]
    fn find_email_like(
        &self,
        input: &str,
        local_class: CharClassType,
        domain_class: CharClassType,
        tld_class: CharClassType,
    ) -> Option<Match> {
        let bytes = input.as_bytes();
        let len = bytes.len();

        let mut search_start = 0;
        while search_start < len {
            match memchr(b'@', &bytes[search_start..]) {
                Some(rel_at) => {
                    let at_abs = search_start + rel_at;

                    if at_abs < 1 || at_abs + 3 >= len {
                        search_start = at_abs + 1;
                        continue;
                    }

                    if !Self::is_valid_for_class(bytes[at_abs - 1], local_class) {
                        search_start = at_abs + 1;
                        continue;
                    }

                    let mut found = false;
                    let mut dot_abs = at_abs + 2;
                    while dot_abs < len {
                        if bytes[dot_abs] == b'.' {
                            let mut valid = true;
                            for i in at_abs + 1..dot_abs {
                                if !Self::is_valid_for_class(bytes[i], domain_class) {
                                    valid = false;
                                    break;
                                }
                            }
                            if valid {
                                found = true;
                                break;
                            }
                        }
                        dot_abs += 1;
                    }

                    if !found {
                        search_start = at_abs + 1;
                        continue;
                    }

                    if dot_abs + 1 >= len
                        || !Self::is_valid_for_class(bytes[dot_abs + 1], tld_class)
                    {
                        search_start = at_abs + 1;
                        continue;
                    }

                    let mut start = at_abs;
                    while start > 0 && Self::is_valid_for_class(bytes[start - 1], local_class) {
                        start -= 1;
                    }

                    let mut end = dot_abs + 2;
                    while end < len && Self::is_valid_for_class(bytes[end], tld_class) {
                        end += 1;
                    }

                    return Some(Match {
                        start,
                        end,
                        captures: vec![(Some(start), Some(end))],
                    });
                }
                None => break,
            }
        }

        None
    }

    #[inline(always)]
    fn is_valid_for_class(byte: u8, class: CharClassType) -> bool {
        match class {
            CharClassType::Digits => byte.is_ascii_digit(),
            CharClassType::Words => byte.is_ascii_alphanumeric() || byte == b'_',
            CharClassType::Spaces => matches!(byte, b' ' | b'\t' | b'\n' | b'\r'),
            CharClassType::Lowercase => byte.is_ascii_lowercase(),
            CharClassType::Uppercase => byte.is_ascii_uppercase(),
            CharClassType::Alpha => byte.is_ascii_alphabetic(),
            CharClassType::Alnum => byte.is_ascii_alphanumeric(),
            CharClassType::Any => true,
        }
    }

    fn find_from(&self, input: &str, start: usize) -> Option<Match> {
        if start >= input.len() {
            return None;
        }

        match self.find(&input[start..]) {
            Some(mut m) => {
                m.start += start;
                m.end += start;
                for cap in &mut m.captures {
                    if let Some(s) = cap.0 {
                        cap.0 = Some(s + start);
                    }
                    if let Some(e) = cap.1 {
                        cap.1 = Some(e + start);
                    }
                }
                Some(m)
            }
            None => None,
        }
    }
}

fn is_pure_literal(pattern: &str) -> bool {
    for c in pattern.chars() {
        match c {
            '\\' | '.' | '+' | '*' | '?' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' => {
                return false;
            }
            _ => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dfa_literal() {
        let dfa = SimpleDFA::new("hello").unwrap();
        let m = dfa.find("hello world").unwrap();
        assert_eq!(m.start, 0);
        assert_eq!(m.end, 5);
    }

    #[test]
    fn test_dfa_digits() {
        let dfa = SimpleDFA::new(r"\d+").unwrap();
        let m = dfa.find("abc123def").unwrap();
        assert_eq!(m.start, 3);
        assert_eq!(m.end, 6);
    }

    #[test]
    fn test_dfa_words() {
        let dfa = SimpleDFA::new(r"\w+").unwrap();
        let m = dfa.find("hello world").unwrap();
        assert_eq!(m.start, 0);
        assert_eq!(m.end, 5);
    }

    #[test]
    fn test_dfa_spaces() {
        let dfa = SimpleDFA::new(r"\s+").unwrap();
        let m = dfa.find("hello   world").unwrap();
        assert_eq!(m.start, 5);
        assert_eq!(m.end, 8);
    }

    #[test]
    fn test_dfa_not_found() {
        let dfa = SimpleDFA::new("xyz").unwrap();
        assert!(dfa.find("abc").is_none());
    }

    #[test]
    fn test_dfa_find_all() {
        let dfa = SimpleDFA::new(r"\d+").unwrap();
        let matches = dfa.find_all("a1b22c333d");
        assert_eq!(matches.len(), 3);
        assert_eq!(&"a1b22c333d"[matches[0].start..matches[0].end], "1");
        assert_eq!(&"a1b22c333d"[matches[1].start..matches[1].end], "22");
        assert_eq!(&"a1b22c333d"[matches[2].start..matches[2].end], "333");
    }

    #[test]
    fn test_dfa_is_match() {
        let dfa = SimpleDFA::new("hello").unwrap();
        assert!(dfa.is_match("hello world"));
        assert!(dfa.is_match("say hello"));
        assert!(!dfa.is_match("goodbye"));
    }

    #[test]
    fn test_dfa_is_match_char_classes() {
        let digits = SimpleDFA::new(r"\d+").unwrap();
        assert!(digits.is_match("abc123"));
        assert!(!digits.is_match("abcdef"));

        let words = SimpleDFA::new(r"\w+").unwrap();
        assert!(words.is_match("hello"));
        assert!(words.is_match("hello_world"));
        assert!(!words.is_match("   "));

        let spaces = SimpleDFA::new(r"\s+").unwrap();
        assert!(spaces.is_match("hello   world"));
        assert!(!spaces.is_match("helloworld"));
    }

    #[test]
    fn test_dfa_start_anchor() {
        let dfa = SimpleDFA::new("^hello").unwrap();
        assert!(dfa.is_match("hello world"));
        assert!(!dfa.is_match("say hello"));

        let m = dfa.find("hello world").unwrap();
        assert_eq!(m.start, 0);
        assert_eq!(m.end, 5);
    }

    #[test]
    fn test_dfa_end_anchor() {
        let dfa = SimpleDFA::new("world$").unwrap();
        assert!(dfa.is_match("hello world"));
        assert!(!dfa.is_match("worldly"));
    }

    #[test]
    fn test_dfa_single_char_star() {
        let dfa = SimpleDFA::new("a*").unwrap();
        assert!(dfa.is_match(""));
        assert!(dfa.is_match("a"));
        assert!(dfa.is_match("aaa"));
        assert!(dfa.is_match("baaa"));
    }

    #[test]
    fn test_dfa_single_char_plus() {
        let dfa = SimpleDFA::new("a+").unwrap();
        assert!(!dfa.is_match(""));
        assert!(!dfa.is_match("bbb"));
        assert!(dfa.is_match("a"));
        assert!(dfa.is_match("aaa"));

        assert!(dfa.is_match("baaa"));

        let m = dfa.find("baaa").unwrap();
        assert_eq!(m.start, 1);
        assert_eq!(m.end, 4);
    }

    #[test]
    fn test_dfa_single_char_literal() {
        let dfa = SimpleDFA::new("x").unwrap();
        assert!(dfa.is_match("x"));
        assert!(dfa.is_match("xyz"));
        assert!(!dfa.is_match("abc"));
    }

    #[test]
    fn test_dfa_empty_input() {
        let dfa = SimpleDFA::new("test").unwrap();
        assert!(!dfa.is_match(""));
        assert!(dfa.find("").is_none());
    }

    #[test]
    fn test_dfa_empty_pattern() {
        let dfa = SimpleDFA::new("").unwrap();
        assert!(dfa.is_match("anything"));
        let m = dfa.find("").unwrap();
        assert_eq!(m.start, 0);
        assert_eq!(m.end, 0);
    }

    #[test]
    fn test_dfa_pattern_longer_than_input() {
        let dfa = SimpleDFA::new("verylongpattern").unwrap();
        assert!(!dfa.is_match("short"));
    }

    #[test]
    fn test_dfa_unicode() {
        let dfa = SimpleDFA::new("hello").unwrap();
        assert!(dfa.is_match("你好 hello 世界"));

        let m = dfa.find("你好 hello 世界").unwrap();
        assert_eq!(m.start, 7);
        assert_eq!(m.end, 12);
    }

    #[test]
    fn test_dfa_multiple_matches() {
        let dfa = SimpleDFA::new(r"\d+").unwrap();
        let matches = dfa.find_all("12 34 56");
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[1].start, 3);
        assert_eq!(matches[2].start, 6);
    }

    #[test]
    fn test_dfa_adjacent_matches() {
        let dfa = SimpleDFA::new("ab").unwrap();
        let matches = dfa.find_all("ababab");
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[1].start, 2);
        assert_eq!(matches[2].start, 4);
    }
}
