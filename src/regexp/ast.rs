use super::charclass::CharRange;

#[derive(Debug, Clone, PartialEq)]
pub enum Ast {
    Empty,

    Char(char),

    Class(CharClass),

    Any,

    AnyAll,

    StartOfLine,

    EndOfLine,

    WordBoundary,

    NotWordBoundary,

    Concat(Vec<Ast>),

    Alt(Vec<Ast>),

    Quant(Box<Ast>, Quantifier),

    Capture(Box<Ast>, Option<String>),

    BackRef(usize),

    NamedBackRef(String),

    Lookahead(Box<Ast>),

    NegativeLookahead(Box<Ast>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharClass {
    pub negated: bool,

    pub ranges: CharRange,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantifier {
    pub min: u32,

    pub max: Option<u32>,

    pub greedy: bool,
}

impl Quantifier {
    pub fn star() -> Self {
        Self {
            min: 0,
            max: None,
            greedy: true,
        }
    }

    pub fn star_lazy() -> Self {
        Self {
            min: 0,
            max: None,
            greedy: false,
        }
    }

    pub fn plus() -> Self {
        Self {
            min: 1,
            max: None,
            greedy: true,
        }
    }

    pub fn plus_lazy() -> Self {
        Self {
            min: 1,
            max: None,
            greedy: false,
        }
    }

    pub fn question() -> Self {
        Self {
            min: 0,
            max: Some(1),
            greedy: true,
        }
    }

    pub fn question_lazy() -> Self {
        Self {
            min: 0,
            max: Some(1),
            greedy: false,
        }
    }

    pub fn range(min: u32, max: Option<u32>) -> Self {
        Self {
            min,
            max,
            greedy: true,
        }
    }

    pub fn range_lazy(min: u32, max: Option<u32>) -> Self {
        Self {
            min,
            max,
            greedy: false,
        }
    }

    pub fn is_fixed(&self) -> bool {
        self.max == Some(self.min)
    }
}

#[derive(Debug)]
pub struct ParseState {
    pub pos: usize,

    pub pattern: Vec<char>,

    pub capture_count: usize,

    pub named_groups: Vec<String>,

    pub is_unicode: bool,

    pub dot_all: bool,
}

impl ParseState {
    pub fn new(pattern: &str, flags: u16) -> Self {
        let is_unicode = (flags & super::opcode::FLAG_UNICODE) != 0
            || (flags & super::opcode::FLAG_UNICODE_SETS) != 0;
        Self {
            pos: 0,
            pattern: pattern.chars().collect(),
            capture_count: 1,
            named_groups: Vec::new(),
            is_unicode,
            dot_all: (flags & super::opcode::FLAG_DOT_ALL) != 0,
        }
    }

    pub fn peek(&self) -> Option<char> {
        self.pattern.get(self.pos).copied()
    }

    pub fn next(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= self.pattern.len()
    }

    pub fn expect(&mut self, expected: char) -> Result<(), String> {
        match self.next() {
            Some(c) if c == expected => Ok(()),
            Some(c) => Err(format!("Expected '{}', found '{}'", expected, c)),
            None => Err(format!("Expected '{}', found end of pattern", expected)),
        }
    }

    pub fn parse_number(&mut self) -> Option<u32> {
        let start = self.pos;
        let mut num: u32 = 0;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.next();
                num = num.wrapping_mul(10).wrapping_add(c.to_digit(10).unwrap());
                if num > i32::MAX as u32 {
                    return None;
                }
            } else {
                break;
            }
        }

        if self.pos == start { None } else { Some(num) }
    }
}

impl Ast {
    pub fn simplify(self) -> Self {
        match self {
            Ast::Concat(nodes) => {
                let mut flattened = Vec::new();
                for node in nodes {
                    match node.simplify() {
                        Ast::Concat(inner) => flattened.extend(inner),
                        Ast::Empty => {}
                        other => flattened.push(other),
                    }
                }
                if flattened.is_empty() {
                    Ast::Empty
                } else if flattened.len() == 1 {
                    flattened.into_iter().next().unwrap()
                } else {
                    Ast::Concat(flattened)
                }
            }
            Ast::Alt(nodes) => {
                let mut flattened = Vec::new();
                for node in nodes {
                    match node.simplify() {
                        Ast::Alt(inner) => flattened.extend(inner),
                        Ast::Empty => {}
                        other => flattened.push(other),
                    }
                }
                if flattened.is_empty() {
                    Ast::Empty
                } else if flattened.len() == 1 {
                    flattened.into_iter().next().unwrap()
                } else {
                    Ast::Alt(flattened)
                }
            }
            Ast::Quant(inner, q) => {
                let inner = inner.simplify();

                if q.min == 0 && q.max == Some(0) {
                    Ast::Empty
                } else if q.min == 1 && q.max == Some(1) {
                    inner
                } else {
                    Ast::Quant(Box::new(inner), q)
                }
            }
            Ast::Capture(inner, name) => Ast::Capture(Box::new(inner.simplify()), name),
            Ast::Lookahead(inner) => Ast::Lookahead(Box::new(inner.simplify())),
            Ast::NegativeLookahead(inner) => Ast::NegativeLookahead(Box::new(inner.simplify())),
            other => other,
        }
    }
}
