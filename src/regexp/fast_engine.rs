use super::compiler::{Program, compile};
use super::dfa::SimpleDFA;
use super::engine::{Match, execute as nfa_execute};
use super::fast_class::FastClassMatcher;
use super::optimizer::{OptimizedPattern, analyze_pattern};
use super::parser::parse;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecMode {
    LiteralFast,

    FastClass,

    LiteralDFA,

    ClassDFA,

    OptimizedNFA,

    FullNFA,
}

#[derive(Clone)]
pub struct FastRegex {
    mode: ExecMode,

    fast_class: Option<FastClassMatcher>,

    dfa: Option<SimpleDFA>,

    program: Option<Program>,
}

impl FastRegex {
    pub fn new(pattern: &str, flags: u16) -> Result<Self, String> {
        if let Some(fast_class) = FastClassMatcher::from_pattern(pattern) {
            return Ok(Self {
                mode: ExecMode::FastClass,
                fast_class: Some(fast_class),
                dfa: None,
                program: None,
            });
        }

        if is_pure_literal(pattern) {
            if let Some(dfa) = SimpleDFA::new(pattern) {
                return Ok(Self {
                    mode: ExecMode::LiteralFast,
                    fast_class: None,
                    dfa: Some(dfa),
                    program: None,
                });
            }
        }

        if let Some(dfa) = SimpleDFA::new(pattern) {
            return Ok(Self {
                mode: ExecMode::ClassDFA,
                fast_class: None,
                dfa: Some(dfa),
                program: None,
            });
        }

        let ast = parse(pattern, flags)?;

        let optimized = analyze_pattern(&ast);

        match optimized {
            OptimizedPattern::LiteralChar(c) => {
                let mut literal_str = String::new();
                literal_str.push(c);
                let dfa = SimpleDFA::new(&literal_str).ok_or("Failed to create literal DFA")?;
                Ok(Self {
                    mode: ExecMode::LiteralFast,
                    fast_class: None,
                    dfa: Some(dfa),
                    program: None,
                })
            }
            OptimizedPattern::LiteralString(s) => {
                let dfa = SimpleDFA::new(&s).ok_or("Failed to create literal DFA")?;
                Ok(Self {
                    mode: ExecMode::LiteralFast,
                    fast_class: None,
                    dfa: Some(dfa),
                    program: None,
                })
            }
            OptimizedPattern::CharClass(_) => {
                let program = compile(&ast, flags)?;
                Ok(Self {
                    mode: ExecMode::FullNFA,
                    fast_class: None,
                    dfa: None,
                    program: Some(program),
                })
            }
            OptimizedPattern::Simple(ast) => {
                let program = compile(&ast, flags).ok();
                Ok(Self {
                    mode: if program.is_some() {
                        ExecMode::OptimizedNFA
                    } else {
                        ExecMode::FullNFA
                    },
                    fast_class: None,
                    dfa: None,
                    program,
                })
            }
            OptimizedPattern::Complex(ast) => {
                let program = compile(&ast, flags)?;
                Ok(Self {
                    mode: ExecMode::FullNFA,
                    fast_class: None,
                    dfa: None,
                    program: Some(program),
                })
            }
        }
    }

    pub fn find(&self, input: &str) -> Option<Match> {
        match self.mode {
            ExecMode::LiteralFast => self.dfa.as_ref().and_then(|dfa| dfa.find(input)),
            ExecMode::FastClass => {
                if let Some(ref fast_class) = self.fast_class {
                    fast_class.find(input)
                } else {
                    None
                }
            }
            ExecMode::LiteralDFA | ExecMode::ClassDFA => {
                if let Some(ref dfa) = self.dfa {
                    dfa.find(input)
                } else {
                    None
                }
            }
            ExecMode::OptimizedNFA => {
                if let Some(ref dfa) = self.dfa {
                    if let Some(m) = dfa.find(input) {
                        return Some(m);
                    }
                }

                if let Some(ref program) = self.program {
                    nfa_execute(program, input, 0)
                } else {
                    None
                }
            }
            ExecMode::FullNFA => {
                if let Some(ref program) = self.program {
                    nfa_execute(program, input, 0)
                } else {
                    None
                }
            }
        }
    }

    #[inline(always)]
    pub fn is_match(&self, input: &str) -> bool {
        match self.mode {
            ExecMode::LiteralFast => self.dfa.as_ref().map_or(false, |dfa| dfa.is_match(input)),
            ExecMode::FastClass => {
                if let Some(ref fast_class) = self.fast_class {
                    fast_class.is_match(input)
                } else {
                    false
                }
            }
            ExecMode::LiteralDFA | ExecMode::ClassDFA => {
                if let Some(ref dfa) = self.dfa {
                    dfa.is_match(input)
                } else {
                    false
                }
            }
            ExecMode::OptimizedNFA | ExecMode::FullNFA => self.find(input).is_some(),
        }
    }

    pub fn find_all(&self, input: &str) -> Vec<Match> {
        if let Some(ref dfa) = self.dfa {
            return dfa.find_all(input);
        }

        if let Some(ref fast_class) = self.fast_class {
            return fast_class.find_all(input);
        }

        let mut matches = Vec::new();
        let mut pos = 0;

        while pos <= input.len() {
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

    fn find_from(&self, input: &str, start: usize) -> Option<Match> {
        if start >= input.len() {
            return None;
        }

        let slice = &input[start..];

        match self.mode {
            ExecMode::LiteralDFA | ExecMode::ClassDFA => {
                if let Some(ref dfa) = self.dfa {
                    dfa.find(slice).map(|m| Match {
                        start: m.start + start,
                        end: m.end + start,
                        captures: m
                            .captures
                            .iter()
                            .map(|(s, e)| (s.map(|x| x + start), e.map(|x| x + start)))
                            .collect(),
                    })
                } else {
                    None
                }
            }
            _ => {
                if let Some(ref program) = self.program {
                    nfa_execute(program, input, start)
                } else {
                    None
                }
            }
        }
    }

    pub fn mode(&self) -> ExecMode {
        self.mode
    }

    pub fn dfa(&self) -> Option<&SimpleDFA> {
        self.dfa.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_literal() {
        let re = FastRegex::new("hello", 0).unwrap();

        assert_eq!(re.mode(), ExecMode::LiteralFast);
        assert!(re.is_match("hello world"));
        assert!(!re.is_match("goodbye"));
    }

    #[test]
    fn test_fast_literal_find() {
        let re = FastRegex::new("world", 0).unwrap();
        let m = re.find("hello world").unwrap();
        assert_eq!(m.start, 6);
        assert_eq!(m.end, 11);
    }

    #[test]
    fn test_fast_char_class() {
        let re = FastRegex::new(r"\w+", 0).unwrap();
        assert_eq!(
            re.mode(),
            ExecMode::FastClass,
            "\\w+ should use FastClass mode"
        );

        let m = re.find("hello world").unwrap();
        assert_eq!(m.start, 0);
        assert_eq!(m.end, 5);

        let re2 = FastRegex::new(r"\d+", 0).unwrap();
        assert_eq!(
            re2.mode(),
            ExecMode::FastClass,
            "\\d+ should use FastClass mode"
        );

        let m2 = re2.find("abc123def").unwrap();
        assert_eq!(m2.start, 3);
        assert_eq!(m2.end, 6);

        let re3 = FastRegex::new(r"\s+", 0).unwrap();
        assert_eq!(
            re3.mode(),
            ExecMode::FastClass,
            "\\s+ should use FastClass mode"
        );

        let m3 = re3.find("hello   world").unwrap();
        assert_eq!(m3.start, 5);
        assert_eq!(m3.end, 8);
    }

    #[test]
    fn test_find_all() {
        let re = FastRegex::new("a", 0).unwrap();
        let matches = re.find_all("banana");
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_is_match_fast() {
        let re = FastRegex::new(r"\d+", 0).unwrap();
        assert!(re.is_match("abc123def"));
        assert!(!re.is_match("abcdef"));
    }

    #[test]
    fn test_fast_literal_is_match() {
        let re = FastRegex::new("hello", 0).unwrap();
        assert!(re.is_match("hello world"));
        assert!(re.is_match("say hello"));
        assert!(!re.is_match("goodbye"));
    }

    #[test]
    fn test_fast_empty_pattern() {
        let re = FastRegex::new("", 0).unwrap();
        assert!(re.is_match("anything"));
        assert!(re.is_match(""));
    }

    #[test]
    fn test_fast_single_char() {
        let re = FastRegex::new("x", 0).unwrap();
        assert!(re.is_match("xyz"));
        assert!(re.is_match("abc x def"));
        assert!(!re.is_match("abc"));
    }

    #[test]
    fn test_fast_anchors() {
        let re_start = FastRegex::new("^hello", 0).unwrap();
        assert!(re_start.is_match("hello world"));
        assert!(!re_start.is_match("say hello"));

        let re_end = FastRegex::new("world$", 0).unwrap();
        assert!(re_end.is_match("hello world"));
        assert!(!re_end.is_match("worldly"));
    }

    #[test]
    fn test_fast_star_plus() {
        let re_star = FastRegex::new("a*", 0).unwrap();
        assert!(re_star.is_match(""));
        assert!(re_star.is_match("a"));
        assert!(re_star.is_match("aaa"));
        assert!(re_star.is_match("baaa"));

        let re_plus = FastRegex::new("a+", 0).unwrap();
        assert!(!re_plus.is_match(""));
        assert!(!re_plus.is_match("bbb"));
        assert!(re_plus.is_match("a"));
        assert!(re_plus.is_match("aaa"));
    }

    #[test]
    fn test_find_all_words() {
        let re = FastRegex::new(r"\w+", 0).unwrap();
        let matches = re.find_all("hello world test");
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[0].end, 5);
        assert_eq!(matches[1].start, 6);
        assert_eq!(matches[1].end, 11);
        assert_eq!(matches[2].start, 12);
        assert_eq!(matches[2].end, 16);
    }

    #[test]
    fn test_find_all_digits() {
        let re = FastRegex::new(r"\d+", 0).unwrap();
        let matches = re.find_all("a1b22c333d");
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].as_str("a1b22c333d"), "1");
        assert_eq!(matches[1].as_str("a1b22c333d"), "22");
        assert_eq!(matches[2].as_str("a1b22c333d"), "333");
    }

    #[test]
    fn test_find_all_no_matches() {
        let re = FastRegex::new("xyz", 0).unwrap();
        let matches = re.find_all("abc");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_fast_unicode() {
        let re = FastRegex::new("hello", 0).unwrap();
        assert!(re.is_match("你好 hello 世界"));

        let m = re.find("你好 hello 世界").unwrap();
        assert_eq!(m.as_str("你好 hello 世界"), "hello");
    }

    #[test]
    fn test_fast_pattern_with_special_chars() {
        let re_dot = FastRegex::new("a.b", 0).unwrap();
        assert!(re_dot.is_match("axb"));
        assert!(!re_dot.is_match("ab"));

        let re_alt = FastRegex::new("a|b", 0).unwrap();
        assert!(re_alt.is_match("a"));
        assert!(re_alt.is_match("b"));
        assert!(!re_alt.is_match("c"));
    }

    #[test]
    fn test_exec_mode_display() {
        assert_eq!(format!("{:?}", ExecMode::LiteralFast), "LiteralFast");
        assert_eq!(format!("{:?}", ExecMode::FastClass), "FastClass");
        assert_eq!(format!("{:?}", ExecMode::ClassDFA), "ClassDFA");
    }
}
