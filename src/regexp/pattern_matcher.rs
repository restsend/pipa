use super::ast::Ast;
use super::engine::Match;
use crate::util::memchr::memchr;

#[derive(Debug, Clone, PartialEq)]
pub enum PatternFingerprint {
    Unknown,

    Literal(String),

    OneOrMoreCharClass(CharClassType),

    StartAnchored(Box<PatternFingerprint>),

    EndAnchored(Box<PatternFingerprint>),

    EmailLike {
        local_part: CharClassType,
        domain_part: CharClassType,
        tld_part: CharClassType,
    },

    UrlLike,

    DateLike,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CharClassType {
    Digits,
    Words,
    Spaces,
    Lowercase,
    Uppercase,
    Alpha,
    Alnum,
    Any,
}

pub fn extract_fingerprint(ast: &Ast) -> PatternFingerprint {
    match ast {
        Ast::Empty => PatternFingerprint::Literal(String::new()),
        Ast::Char(c) => PatternFingerprint::Literal(c.to_string()),
        Ast::Concat(nodes) => extract_concat_fingerprint(nodes),
        Ast::Quant(inner, q) if q.min >= 1 => {
            if let Some(class_type) = extract_char_class_type(inner) {
                PatternFingerprint::OneOrMoreCharClass(class_type)
            } else {
                PatternFingerprint::Unknown
            }
        }
        Ast::StartOfLine => PatternFingerprint::Unknown,
        Ast::EndOfLine => PatternFingerprint::Unknown,
        _ => PatternFingerprint::Unknown,
    }
}

fn extract_concat_fingerprint(nodes: &[Ast]) -> PatternFingerprint {
    if nodes.is_empty() {
        return PatternFingerprint::Literal(String::new());
    }

    if matches!(nodes.first(), Some(Ast::StartOfLine)) && nodes.len() > 1 {
        let inner_fp = extract_fingerprint(&Ast::Concat(nodes[1..].to_vec()));
        return PatternFingerprint::StartAnchored(Box::new(inner_fp));
    }

    if matches!(nodes.last(), Some(Ast::EndOfLine)) && nodes.len() > 1 {
        let inner_fp = extract_fingerprint(&Ast::Concat(nodes[..nodes.len() - 1].to_vec()));
        return PatternFingerprint::EndAnchored(Box::new(inner_fp));
    }

    if let Some(email_fp) = try_extract_email_fingerprint(nodes) {
        return email_fp;
    }

    let mut literal = String::new();
    for node in nodes {
        match node {
            Ast::Char(c) => literal.push(*c),
            _ => return PatternFingerprint::Unknown,
        }
    }
    PatternFingerprint::Literal(literal)
}

fn try_extract_email_fingerprint(nodes: &[Ast]) -> Option<PatternFingerprint> {
    if nodes.len() < 5 {
        return None;
    }

    for at_pos in 0..nodes.len() {
        for dot_pos in (at_pos + 1)..nodes.len() {
            let has_at = matches!(&nodes[at_pos], Ast::Char('@'));

            let has_dot = matches!(&nodes[dot_pos], Ast::Char('.'));

            if !has_at || !has_dot {
                continue;
            }

            let local_part = if at_pos == 1 {
                match &nodes[0] {
                    Ast::Quant(inner, q) if q.min >= 1 => extract_char_class_type(inner),
                    _ => None,
                }
            } else {
                None
            }?;

            let domain_part = if dot_pos == at_pos + 2 {
                match &nodes[at_pos + 1] {
                    Ast::Quant(inner, q) if q.min >= 1 => extract_char_class_type(inner),
                    _ => None,
                }
            } else {
                None
            }?;

            let tld_part = if dot_pos + 1 < nodes.len() {
                match &nodes[dot_pos + 1] {
                    Ast::Quant(inner, q) if q.min >= 1 => extract_char_class_type(inner),
                    _ => None,
                }
            } else {
                None
            }?;

            return Some(PatternFingerprint::EmailLike {
                local_part,
                domain_part,
                tld_part,
            });
        }
    }

    None
}

fn extract_char_class_type(ast: &Ast) -> Option<CharClassType> {
    match ast {
        Ast::Class(class) => {
            if class.negated {
                return None;
            }

            let ranges = class.ranges.as_slice();
            match ranges {
                [(48, 58)] => Some(CharClassType::Digits),
                [(97, 123)] => Some(CharClassType::Lowercase),
                [(65, 91)] => Some(CharClassType::Uppercase),
                [(97, 123), (65, 91)] => Some(CharClassType::Alpha),
                [(97, 123), (65, 91), (48, 58)] => Some(CharClassType::Alnum),

                [(48, 58), (65, 91), (95, 96), (97, 123)] => Some(CharClassType::Words),

                [(9, 14), (32, 33)] => Some(CharClassType::Spaces),

                [(48, 58), (97, 123)] => Some(CharClassType::Alnum),

                [(48, 58), (65, 91), (97, 123)] => Some(CharClassType::Alnum),
                _ => None,
            }
        }
        _ => None,
    }
}

pub struct FastPatternMatcher {
    fingerprint: PatternFingerprint,
}

impl FastPatternMatcher {
    pub fn new(fingerprint: PatternFingerprint) -> Option<Self> {
        match &fingerprint {
            PatternFingerprint::Unknown => None,
            _ => Some(Self { fingerprint }),
        }
    }

    pub fn find(&self, input: &str) -> Option<Match> {
        match &self.fingerprint {
            PatternFingerprint::EmailLike {
                local_part,
                domain_part,
                tld_part,
            } => self.find_email_like(input, *local_part, *domain_part, *tld_part),
            PatternFingerprint::StartAnchored(_inner) => None,
            PatternFingerprint::EndAnchored(_inner) => None,
            _ => None,
        }
    }

    fn find_email_like(
        &self,
        input: &str,
        _local_class: CharClassType,
        _domain_class: CharClassType,
        _tld_class: CharClassType,
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

                    if !is_valid_for_class(bytes[at_abs - 1], _local_class) {
                        search_start = at_abs + 1;
                        continue;
                    }

                    let mut found = false;
                    let mut dot_abs = at_abs + 2;
                    while dot_abs < len {
                        if bytes[dot_abs] == b'.' {
                            let mut valid = true;
                            for i in at_abs + 1..dot_abs {
                                if !is_valid_for_class(bytes[i], _domain_class) {
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

                    if dot_abs + 1 >= len || !is_valid_for_class(bytes[dot_abs + 1], _tld_class) {
                        search_start = at_abs + 1;
                        continue;
                    }

                    let mut start = at_abs;
                    while start > 0 && is_valid_for_class(bytes[start - 1], _local_class) {
                        start -= 1;
                    }

                    let mut end = dot_abs + 2;
                    while end < len && is_valid_for_class(bytes[end], _tld_class) {
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
}

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
