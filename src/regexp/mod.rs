mod ast;
mod char_table;
mod charclass;
pub mod compiler;
mod dfa;
mod engine;
mod fast_class;
mod fast_engine;
mod memchr;
pub mod opcode;
pub mod optimizer;
pub mod parser;
pub mod pattern_matcher;
mod pool;

pub mod string_search;
#[cfg(test)]
mod tests;

use std::fmt;

pub use engine::Match;
pub use fast_engine::FastRegex;
pub use optimizer::OptimizedPattern;

#[derive(Clone)]
pub struct Regex {
    fast_engine: Option<fast_engine::FastRegex>,

    program: Option<compiler::Program>,

    pattern: String,

    flags: RegexFlags,
}

#[derive(Debug, Clone)]
pub struct MatchResult<'a> {
    text: &'a str,

    start: usize,

    end: usize,

    captures: Vec<Option<&'a str>>,

    capture_positions: Vec<(Option<usize>, Option<usize>)>,
}

impl<'a> MatchResult<'a> {
    pub fn as_str(&self) -> &'a str {
        &self.text[self.start..self.end]
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }

    pub fn get(&self, i: usize) -> Option<&'a str> {
        self.captures.get(i).copied().flatten()
    }

    pub fn len(&self) -> usize {
        self.captures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.captures.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = Option<&'a str>> + '_ {
        self.captures.iter().copied()
    }

    pub fn positions(&self) -> &[(Option<usize>, Option<usize>)] {
        &self.capture_positions
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RegexFlags {
    pub global: bool,

    pub ignore_case: bool,

    pub multi_line: bool,

    pub dot_all: bool,

    pub unicode: bool,

    pub sticky: bool,
}

impl RegexFlags {
    pub fn from_str(s: &str) -> Result<Self, String> {
        let mut flags = Self::default();
        for c in s.chars() {
            match c {
                'g' => flags.global = true,
                'i' => flags.ignore_case = true,
                'm' => flags.multi_line = true,
                's' => flags.dot_all = true,
                'u' => flags.unicode = true,
                'y' => flags.sticky = true,
                _ => return Err(format!("Invalid flag: {}", c)),
            }
        }
        Ok(flags)
    }

    fn to_u16(&self) -> u16 {
        let mut f = 0u16;
        if self.global {
            f |= opcode::FLAG_GLOBAL;
        }
        if self.ignore_case {
            f |= opcode::FLAG_IGNORE_CASE;
        }
        if self.multi_line {
            f |= opcode::FLAG_MULTI_LINE;
        }
        if self.dot_all {
            f |= opcode::FLAG_DOT_ALL;
        }
        if self.unicode {
            f |= opcode::FLAG_UNICODE;
        }
        if self.sticky {
            f |= opcode::FLAG_STICKY;
        }
        f
    }
}

impl fmt::Display for RegexFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.global {
            write!(f, "g")?;
        }
        if self.ignore_case {
            write!(f, "i")?;
        }
        if self.multi_line {
            write!(f, "m")?;
        }
        if self.dot_all {
            write!(f, "s")?;
        }
        if self.unicode {
            write!(f, "u")?;
        }
        if self.sticky {
            write!(f, "y")?;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct RegexBuilder {
    pattern: String,
    flags: RegexFlags,
}

impl RegexBuilder {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            flags: RegexFlags::default(),
        }
    }

    pub fn global(mut self, value: bool) -> Self {
        self.flags.global = value;
        self
    }

    pub fn ignore_case(mut self, value: bool) -> Self {
        self.flags.ignore_case = value;
        self
    }

    pub fn multi_line(mut self, value: bool) -> Self {
        self.flags.multi_line = value;
        self
    }

    pub fn dot_all(mut self, value: bool) -> Self {
        self.flags.dot_all = value;
        self
    }

    pub fn unicode(mut self, value: bool) -> Self {
        self.flags.unicode = value;
        self
    }

    pub fn sticky(mut self, value: bool) -> Self {
        self.flags.sticky = value;
        self
    }

    pub fn build(self) -> Result<Regex, RegexError> {
        Regex::new_with_flags(&self.pattern, self.flags)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegexError {
    Parse(String),

    Compile(String),

    Other(String),
}

impl fmt::Display for RegexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegexError::Parse(msg) => write!(f, "Parse error: {}", msg),
            RegexError::Compile(msg) => write!(f, "Compile error: {}", msg),
            RegexError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for RegexError {}

impl Regex {
    pub fn new(pattern: &str) -> Result<Self, RegexError> {
        Self::new_with_flags(pattern, RegexFlags::default())
    }

    pub fn new_with_flags(pattern: &str, flags: RegexFlags) -> Result<Self, RegexError> {
        let flag_bits = flags.to_u16();

        match fast_engine::FastRegex::new(pattern, flag_bits) {
            Ok(fast) => {
                return Ok(Self {
                    fast_engine: Some(fast),
                    program: None,
                    pattern: pattern.to_string(),
                    flags,
                });
            }
            Err(_) => {
                let ast = parser::parse(pattern, flag_bits).map_err(RegexError::Parse)?;

                let program = compiler::compile(&ast, flag_bits).map_err(RegexError::Compile)?;

                Ok(Self {
                    fast_engine: None,
                    program: Some(program),
                    pattern: pattern.to_string(),
                    flags,
                })
            }
        }
    }

    pub fn with_flags(pattern: &str, flags: &str) -> Result<Self, RegexError> {
        let flags = RegexFlags::from_str(flags).map_err(|e| RegexError::Parse(e))?;
        Self::new_with_flags(pattern, flags)
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn flags(&self) -> RegexFlags {
        self.flags
    }

    pub fn find<'a>(&self, input: &'a str) -> Option<MatchResult<'a>> {
        if let Some(ref fast) = self.fast_engine {
            if let Some(m) = fast.find(input) {
                return self.match_result_from_engine(input, m);
            }
        }

        if let Some(ref program) = self.program {
            let m = engine::execute(program, input, 0)?;
            return self.match_result_from_engine(input, m);
        }

        None
    }

    pub fn find_at<'a>(&self, input: &'a str, start: usize) -> Option<MatchResult<'a>> {
        if let Some(ref program) = self.program {
            let m = engine::execute(program, input, start)?;
            return self.match_result_from_engine(input, m);
        }

        if let Some(ref fast) = self.fast_engine {
            if let Some(m) = fast.find(&input[start..]) {
                let shifted = Match {
                    start: m.start + start,
                    end: m.end + start,
                    captures: m.captures,
                };
                return self.match_result_from_engine(input, shifted);
            }
        }

        None
    }

    pub fn find_all<'a>(&self, input: &'a str) -> Vec<MatchResult<'a>> {
        let is_ascii = is_ascii_fast(input);

        if let Some(ref fast) = self.fast_engine {
            fast.find_all(input)
                .into_iter()
                .filter_map(|m| self.match_result_from_engine_fast(input, m, is_ascii))
                .collect()
        } else if let Some(ref program) = self.program {
            engine::find_all(program, input)
                .into_iter()
                .filter_map(|m| self.match_result_from_engine_fast(input, m, is_ascii))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn match_result_from_engine_fast<'a>(
        &self,
        input: &'a str,
        m: engine::Match,
        is_ascii: bool,
    ) -> Option<MatchResult<'a>> {
        let (start_byte, end_byte) = if is_ascii {
            (m.start, m.end)
        } else {
            let char_positions: Vec<usize> = input.char_indices().map(|(i, _)| i).collect();
            let start_byte = char_positions.get(m.start).copied().unwrap_or(0);
            let end_byte = char_positions.get(m.end).copied().unwrap_or(input.len());
            (start_byte, end_byte)
        };

        let mut captures = Vec::with_capacity(m.captures.len());
        for (start, end) in &m.captures {
            let cap = match (start, end) {
                (Some(s), Some(e)) => {
                    if is_ascii {
                        Some(&input[*s..*e])
                    } else {
                        let start_byte = input.char_indices().nth(*s).map(|(i, _)| i).unwrap_or(0);
                        let end_byte = input
                            .char_indices()
                            .nth(*e)
                            .map(|(i, _)| i)
                            .unwrap_or(input.len());
                        Some(&input[start_byte..end_byte])
                    }
                }
                _ => None,
            };
            captures.push(cap);
        }

        Some(MatchResult {
            text: input,
            start: start_byte,
            end: end_byte,
            captures,
            capture_positions: m.captures,
        })
    }

    pub fn is_match(&self, input: &str) -> bool {
        if let Some(ref fast) = self.fast_engine {
            if let Some(ref dfa) = fast.dfa() {
                return dfa.is_match(input);
            }
            return fast.is_match(input);
        }

        self.find(input).is_some()
    }

    pub fn is_full_match(&self, input: &str) -> bool {
        if let Some(m) = self.find(input) {
            m.start() == 0 && m.end() == input.len()
        } else {
            false
        }
    }

    pub fn replace<'a>(&self, input: &'a str, replacement: &str) -> String {
        self.replace_n(input, replacement, 1)
    }

    pub fn replace_all<'a>(&self, input: &'a str, replacement: &str) -> String {
        self.replace_n(input, replacement, usize::MAX)
    }

    pub fn replace_n<'a>(&self, input: &'a str, replacement: &str, n: usize) -> String {
        let mut result = String::new();
        let mut last_end = 0;
        let mut count = 0;

        for m in self.find_all(input) {
            if count >= n {
                break;
            }
            result.push_str(&input[last_end..m.start()]);
            result.push_str(replacement);
            last_end = m.end();
            count += 1;
        }

        result.push_str(&input[last_end..]);
        result
    }

    pub fn replace_fn<'a, F>(&self, input: &'a str, f: F) -> String
    where
        F: Fn(&MatchResult) -> String,
    {
        let mut result = String::new();
        let mut last_end = 0;

        for m in self.find_all(input) {
            result.push_str(&input[last_end..m.start()]);
            result.push_str(&f(&m));
            last_end = m.end();
        }

        result.push_str(&input[last_end..]);
        result
    }

    pub fn capture_count(&self) -> usize {
        if let Some(ref program) = self.program {
            program.capture_count
        } else {
            1
        }
    }

    fn match_result_from_engine<'a>(
        &self,
        input: &'a str,
        m: engine::Match,
    ) -> Option<MatchResult<'a>> {
        let is_ascii = is_ascii_fast(input);
        self.match_result_from_engine_fast(input, m, is_ascii)
    }
}

#[inline(always)]
fn is_ascii_fast(s: &str) -> bool {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i + 8 <= len {
        let chunk = &bytes[i..i + 8];
        if (chunk[0] | chunk[1] | chunk[2] | chunk[3] | chunk[4] | chunk[5] | chunk[6] | chunk[7])
            >= 0x80
        {
            return false;
        }
        i += 8;
    }

    while i < len {
        if bytes[i] >= 0x80 {
            return false;
        }
        i += 1;
    }

    true
}

impl fmt::Debug for Regex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Regex(/{}/{}", self.pattern, self.flags)
    }
}

impl fmt::Display for Regex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/{}/{}", self.pattern, self.flags)
    }
}

pub fn find(pattern: &str, input: &str) -> Result<Option<String>, RegexError> {
    let re = Regex::new(pattern)?;
    Ok(re.find(input).map(|m| m.as_str().to_string()))
}

pub fn is_match(pattern: &str, input: &str) -> Result<bool, RegexError> {
    let re = Regex::new(pattern)?;
    Ok(re.is_match(input))
}

pub fn replace(pattern: &str, input: &str, replacement: &str) -> Result<String, RegexError> {
    let re = Regex::new(pattern)?;
    Ok(re.replace_all(input, replacement))
}
