use super::char_table::{CLASS_DIGIT, CLASS_SPACE, CLASS_WORD, is_ascii_class};
use super::engine::Match;

#[derive(Clone, Copy, Debug)]
pub enum FastClassMatcher {
    Digits,

    Words,

    Spaces,

    SingleDigit,

    SingleWord,

    SingleSpace,

    NotDigits,

    NotWords,

    NotSpaces,

    DigitsStar,

    WordsStar,

    SpacesStar,

    LiteralAlternation {
        options: &'static [&'static str],

        first_bytes: &'static [u8],
    },
}

impl FastClassMatcher {
    pub fn from_pattern(pattern: &str) -> Option<Self> {
        match pattern {
            r"\d+" => Some(Self::Digits),
            r"\w+" => Some(Self::Words),
            r"\s+" => Some(Self::Spaces),
            r"\d" => Some(Self::SingleDigit),
            r"\w" => Some(Self::SingleWord),
            r"\s" => Some(Self::SingleSpace),
            r"\D+" => Some(Self::NotDigits),
            r"\W+" => Some(Self::NotWords),
            r"\S+" => Some(Self::NotSpaces),
            r"\d*" => Some(Self::DigitsStar),
            r"\w*" => Some(Self::WordsStar),
            r"\s*" => Some(Self::SpacesStar),

            "foo|bar" => Some(Self::LiteralAlternation {
                options: &["foo", "bar"],
                first_bytes: &[b'f', b'b'],
            }),
            "yes|no" => Some(Self::LiteralAlternation {
                options: &["yes", "no"],
                first_bytes: &[b'y', b'n'],
            }),
            "true|false" => Some(Self::LiteralAlternation {
                options: &["true", "false"],
                first_bytes: &[b't', b'f'],
            }),
            _ => None,
        }
    }

    #[inline(always)]
    pub fn find(&self, input: &str) -> Option<Match> {
        match self {
            Self::Digits => Self::match_digits_fast(input),
            Self::Words => Self::match_words_fast(input),
            Self::Spaces => Self::match_spaces_fast(input),
            Self::SingleDigit => Self::match_single(input, CLASS_DIGIT),
            Self::SingleWord => Self::match_single(input, CLASS_WORD),
            Self::SingleSpace => Self::match_single(input, CLASS_SPACE),
            Self::NotDigits => Self::match_plus_negated(input, CLASS_DIGIT),
            Self::NotWords => Self::match_plus_negated(input, CLASS_WORD),
            Self::NotSpaces => Self::match_plus_negated(input, CLASS_SPACE),
            Self::DigitsStar => Self::match_star(input, CLASS_DIGIT),
            Self::WordsStar => Self::match_star(input, CLASS_WORD),
            Self::SpacesStar => Self::match_star(input, CLASS_SPACE),
            Self::LiteralAlternation {
                options,
                first_bytes,
            } => Self::match_literal_alternation(input, options, first_bytes),
        }
    }

    #[inline(always)]
    fn match_star(input: &str, class: u8) -> Option<Match> {
        let bytes = input.as_bytes();

        if bytes.is_empty() {
            return Some(Match {
                start: 0,
                end: 0,
                captures: vec![(Some(0), Some(0))],
            });
        }

        let start = find_first_class(bytes, class).unwrap_or(0);
        let end = find_last_class(bytes, start, class);

        Some(Match {
            start,
            end,
            captures: vec![(Some(start), Some(end))],
        })
    }

    #[inline(always)]
    fn match_plus_negated(input: &str, class: u8) -> Option<Match> {
        let bytes = input.as_bytes();

        let mut start = 0;
        while start < bytes.len() && is_ascii_class(bytes[start], class) {
            start += 1;
        }

        if start >= bytes.len() {
            return None;
        }

        let mut end = start + 1;
        while end < bytes.len() && !is_ascii_class(bytes[end], class) {
            end += 1;
        }

        Some(Match {
            start,
            end,
            captures: vec![(Some(start), Some(end))],
        })
    }

    #[inline(always)]
    fn match_single(input: &str, class: u8) -> Option<Match> {
        let bytes = input.as_bytes();

        for (pos, &byte) in bytes.iter().enumerate() {
            if is_ascii_class(byte, class) {
                return Some(Match {
                    start: pos,
                    end: pos + 1,
                    captures: vec![(Some(pos), Some(pos + 1))],
                });
            }
        }

        None
    }

    #[inline(always)]
    fn match_digits_fast(input: &str) -> Option<Match> {
        let bytes = input.as_bytes();
        let len = bytes.len();

        let mut start = 0;
        while start < len {
            let b = bytes[start];
            if b <= b'9' && b >= b'0' {
                break;
            }
            start += 1;
        }

        if start >= len {
            return None;
        }

        let mut end = start + 1;
        while end < len {
            let b = bytes[end];
            if b > b'9' || b < b'0' {
                break;
            }
            end += 1;
        }

        Some(Match {
            start,
            end,
            captures: vec![(Some(start), Some(end))],
        })
    }

    #[inline(always)]
    fn match_words_fast(input: &str) -> Option<Match> {
        let bytes = input.as_bytes();
        let len = bytes.len();

        let mut start = 0;
        while start < len {
            let b = bytes[start];
            if (b >= b'0' && b <= b'9')
                || (b >= b'A' && b <= b'Z')
                || (b >= b'a' && b <= b'z')
                || b == b'_'
            {
                break;
            }
            start += 1;
        }

        if start >= len {
            return None;
        }

        let mut end = start + 1;
        while end < len {
            let b = bytes[end];
            if !((b >= b'0' && b <= b'9')
                || (b >= b'A' && b <= b'Z')
                || (b >= b'a' && b <= b'z')
                || b == b'_')
            {
                break;
            }
            end += 1;
        }

        Some(Match {
            start,
            end,
            captures: vec![(Some(start), Some(end))],
        })
    }

    #[inline(always)]
    fn match_spaces_fast(input: &str) -> Option<Match> {
        let bytes = input.as_bytes();
        let len = bytes.len();

        let mut start = 0;
        while start < len {
            let b = bytes[start];
            if matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0C | 0x0B) {
                break;
            }
            start += 1;
        }

        if start >= len {
            return None;
        }

        let mut end = start + 1;
        while end < len {
            let b = bytes[end];
            if !matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0C | 0x0B) {
                break;
            }
            end += 1;
        }

        Some(Match {
            start,
            end,
            captures: vec![(Some(start), Some(end))],
        })
    }

    #[inline(always)]
    fn match_literal_alternation(
        input: &str,
        options: &[&str],
        _first_bytes: &[u8],
    ) -> Option<Match> {
        let bytes = input.as_bytes();

        for start in 0..bytes.len() {
            for option in options {
                let opt_bytes = option.as_bytes();
                if start + opt_bytes.len() <= bytes.len() {
                    if &bytes[start..start + opt_bytes.len()] == opt_bytes {
                        return Some(Match {
                            start,
                            end: start + opt_bytes.len(),
                            captures: vec![(Some(start), Some(start + opt_bytes.len()))],
                        });
                    }
                }
            }
        }

        None
    }

    pub fn find_all(&self, input: &str) -> Vec<Match> {
        match self {
            Self::Digits => Self::find_all_digits(input),
            Self::Words => Self::find_all_words(input),
            _ => {
                let mut matches = Vec::new();
                let mut pos = 0;

                while pos < input.len() {
                    if let Some(m) = self.find(&input[pos..]) {
                        let abs_start = pos + m.start;
                        let abs_end = pos + m.end;
                        matches.push(Match {
                            start: abs_start,
                            end: abs_end,
                            captures: vec![(Some(abs_start), Some(abs_end))],
                        });
                        if m.end == 0 {
                            pos += 1;
                        } else {
                            pos = abs_end;
                        }
                    } else {
                        break;
                    }
                }

                matches
            }
        }
    }

    fn find_all_digits(input: &str) -> Vec<Match> {
        let mut matches = Vec::new();
        let bytes = input.as_bytes();
        let len = bytes.len();
        let mut pos = 0;

        while pos < len {
            while pos < len {
                let b = bytes[pos];
                if b <= b'9' && b >= b'0' {
                    break;
                }
                pos += 1;
            }

            if pos >= len {
                break;
            }

            let start = pos;
            pos += 1;
            while pos < len {
                let b = bytes[pos];
                if b > b'9' || b < b'0' {
                    break;
                }
                pos += 1;
            }

            matches.push(Match {
                start,
                end: pos,
                captures: vec![(Some(start), Some(pos))],
            });
        }

        matches
    }

    fn find_all_words(input: &str) -> Vec<Match> {
        let mut matches = Vec::new();
        let bytes = input.as_bytes();
        let len = bytes.len();
        let mut pos = 0;

        while pos < len {
            while pos < len {
                let b = bytes[pos];
                if (b >= b'0' && b <= b'9')
                    || (b >= b'A' && b <= b'Z')
                    || (b >= b'a' && b <= b'z')
                    || b == b'_'
                {
                    break;
                }
                pos += 1;
            }

            if pos >= len {
                break;
            }

            let start = pos;
            pos += 1;
            while pos < len {
                let b = bytes[pos];
                if !((b >= b'0' && b <= b'9')
                    || (b >= b'A' && b <= b'Z')
                    || (b >= b'a' && b <= b'z')
                    || b == b'_')
                {
                    break;
                }
                pos += 1;
            }

            matches.push(Match {
                start,
                end: pos,
                captures: vec![(Some(start), Some(pos))],
            });
        }

        matches
    }

    #[inline(always)]

    pub fn is_match(&self, input: &str) -> bool {
        self.find(input).is_some()
    }
}

#[inline(always)]
fn find_first_class(bytes: &[u8], class: u8) -> Option<usize> {
    for (pos, &byte) in bytes.iter().enumerate() {
        if is_ascii_class(byte, class) {
            return Some(pos);
        }
    }
    None
}

#[inline(always)]
fn find_last_class(bytes: &[u8], start: usize, class: u8) -> usize {
    let mut end = start + 1;
    while end < bytes.len() && is_ascii_class(bytes[end], class) {
        end += 1;
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_digits() {
        let matcher = FastClassMatcher::Digits;
        let m = matcher.find("abc123def").unwrap();
        assert_eq!(m.start, 3);
        assert_eq!(m.end, 6);
    }

    #[test]
    fn test_fast_words() {
        let matcher = FastClassMatcher::Words;
        let m = matcher.find("hello world").unwrap();
        assert_eq!(m.start, 0);
        assert_eq!(m.end, 5);
    }

    #[test]
    fn test_fast_spaces() {
        let matcher = FastClassMatcher::Spaces;
        let m = matcher.find("hello   world").unwrap();
        assert_eq!(m.start, 5);
        assert_eq!(m.end, 8);
    }

    #[test]
    fn test_single_digit() {
        let matcher = FastClassMatcher::SingleDigit;
        let m = matcher.find("abc1def").unwrap();
        assert_eq!(m.start, 3);
        assert_eq!(m.end, 4);
    }

    #[test]
    fn test_find_all_digits() {
        let matcher = FastClassMatcher::Digits;
        let matches = matcher.find_all("a1b22c333d");
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].as_str("a1b22c333d"), "1");
        assert_eq!(matches[1].as_str("a1b22c333d"), "22");
        assert_eq!(matches[2].as_str("a1b22c333d"), "333");
    }

    #[test]
    fn test_find_all_words() {
        let matcher = FastClassMatcher::Words;
        let matches = matcher.find_all("hello world test");
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_not_digits() {
        let matcher = FastClassMatcher::NotDigits;
        let m = matcher.find("123abc456").unwrap();
        assert_eq!(m.start, 3);
        assert_eq!(m.end, 6);
    }

    #[test]
    fn test_digits_star() {
        let matcher = FastClassMatcher::DigitsStar;
        let m = matcher.find("abc123").unwrap();

        assert_eq!(m.start, 3);
        assert_eq!(m.end, 6);
    }
}
