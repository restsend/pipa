use super::ast::Ast;
use super::charclass::CharRange;

#[derive(Debug, Clone)]
pub enum OptimizedPattern {
    LiteralChar(char),

    LiteralString(String),

    CharClass(CharRange),

    Simple(Box<Ast>),

    Complex(Box<Ast>),
}

pub fn analyze_pattern(ast: &Ast) -> OptimizedPattern {
    match ast {
        Ast::Char(c) => OptimizedPattern::LiteralChar(*c),
        Ast::Concat(nodes) if nodes.len() == 1 => analyze_pattern(&nodes[0]),
        Ast::Concat(nodes) => {
            let mut literal = String::new();
            for node in nodes {
                match node {
                    Ast::Char(c) => literal.push(*c),
                    _ => return OptimizedPattern::Complex(Box::new(ast.clone())),
                }
            }
            if literal.len() == 1 {
                OptimizedPattern::LiteralChar(literal.chars().next().unwrap())
            } else {
                OptimizedPattern::LiteralString(literal)
            }
        }
        Ast::Class(class) => OptimizedPattern::CharClass(class.ranges.clone()),
        Ast::Any => OptimizedPattern::Simple(Box::new(ast.clone())),
        _ => OptimizedPattern::Complex(Box::new(ast.clone())),
    }
}

pub fn fast_literal_search(haystack: &str, needle: &str) -> Option<usize> {
    if needle.len() == 1 {
        let c = needle.chars().next().unwrap();
        fast_char_search(haystack, c)
    } else {
        boyer_moore_search(haystack, needle)
    }
}

#[inline(always)]
pub fn fast_char_search(haystack: &str, needle: char) -> Option<usize> {
    if needle.is_ascii() {
        let needle_byte = needle as u8;
        haystack.bytes().position(|b| b == needle_byte)
    } else {
        haystack.chars().position(|c| c == needle)
    }
}

pub fn boyer_moore_search(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if haystack.len() < needle.len() {
        return None;
    }

    if needle.len() < 4 {
        return naive_search(haystack, needle);
    }

    let mut bad_char: [usize; 256] = [0; 256];
    let needle_bytes = needle.as_bytes();
    let needle_len = needle_bytes.len();

    for (i, &b) in needle_bytes.iter().enumerate() {
        bad_char[b as usize] = i + 1;
    }

    let haystack_bytes = haystack.as_bytes();
    let mut i = 0;

    while i <= haystack_bytes.len() - needle_len {
        let mut j = needle_len;

        while j > 0 && haystack_bytes[i + j - 1] == needle_bytes[j - 1] {
            j -= 1;
        }

        if j == 0 {
            return Some(i);
        }

        let bad = bad_char[haystack_bytes[i + needle_len - 1] as usize];
        if bad == 0 {
            i += needle_len;
        } else {
            i += needle_len - bad + 1;
        }
    }

    None
}

#[inline(always)]
fn naive_search(haystack: &str, needle: &str) -> Option<usize> {
    let haystack_bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();

    haystack_bytes
        .windows(needle_bytes.len())
        .position(|window| window == needle_bytes)
}

pub fn fast_byte_search(haystack: &str, needle: u8) -> Option<usize> {
    haystack.bytes().position(|b| b == needle)
}

#[derive(Clone)]
pub struct SimpleDFA {
    literal: Option<String>,

    char_class: Option<CharRange>,
}

impl SimpleDFA {
    pub fn new(pattern: &Ast) -> Option<Self> {
        match analyze_pattern(pattern) {
            OptimizedPattern::LiteralChar(c) => Some(Self {
                literal: Some(c.to_string()),
                char_class: None,
            }),
            OptimizedPattern::LiteralString(s) => Some(Self {
                literal: Some(s),
                char_class: None,
            }),
            OptimizedPattern::CharClass(class) => Some(Self {
                literal: None,
                char_class: Some(class),
            }),
            _ => None,
        }
    }

    pub fn find(&self, input: &str) -> Option<(usize, usize)> {
        if let Some(ref literal) = self.literal {
            fast_literal_search(input, literal).map(|pos| (pos, pos + literal.len()))
        } else if let Some(ref class) = self.char_class {
            for (pos, c) in input.char_indices() {
                if class.contains(c as u32) {
                    let end = pos + c.len_utf8();
                    return Some((pos, end));
                }
            }
            None
        } else {
            None
        }
    }

    pub fn match_at(&self, input: &str, pos: usize) -> Option<usize> {
        if pos >= input.len() {
            return None;
        }

        if let Some(ref literal) = self.literal {
            let remaining = &input[pos..];
            if remaining.starts_with(literal) {
                Some(pos + literal.len())
            } else {
                None
            }
        } else if let Some(ref class) = self.char_class {
            let remaining = &input[pos..];
            if let Some(c) = remaining.chars().next() {
                if class.contains(c as u32) {
                    Some(pos + c.len_utf8())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub struct MatchStats {
    pub attempts: u64,

    pub successes: u64,

    pub backtracks: u64,
}

impl MatchStats {
    pub fn new() -> Self {
        Self {
            attempts: 0,
            successes: 0,
            backtracks: 0,
        }
    }

    pub fn should_use_dfa(&self) -> bool {
        if self.attempts < 100 {
            return false;
        }
        let backtrack_ratio = self.backtracks as f64 / self.attempts as f64;
        backtrack_ratio < 0.1
    }
}

impl Default for MatchStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_search() {
        assert_eq!(fast_char_search("hello", 'l'), Some(2));
        assert_eq!(fast_char_search("hello", 'x'), None);
    }

    #[test]
    fn test_boyer_moore() {
        let text = "The quick brown fox jumps over the lazy dog";
        assert_eq!(boyer_moore_search(text, "fox"), Some(16));
        assert_eq!(boyer_moore_search(text, "cat"), None);
    }

    #[test]
    fn test_naive_search() {
        assert_eq!(naive_search("hello", "ll"), Some(2));
        assert_eq!(naive_search("hello", "abc"), None);
    }

    #[test]
    fn test_dfa_literal() {
        let ast = Ast::Char('a');
        let dfa = SimpleDFA::new(&ast).unwrap();
        assert_eq!(dfa.find("abc"), Some((0, 1)));
        assert_eq!(dfa.find("bbc"), None);
    }

    #[test]
    fn test_dfa_match_at() {
        let ast = Ast::Char('a');
        let dfa = SimpleDFA::new(&ast).unwrap();
        assert_eq!(dfa.match_at("abc", 0), Some(1));
        assert_eq!(dfa.match_at("abc", 1), None);
    }
}
