use super::ast::{Ast, CharClass, ParseState, Quantifier};
use super::charclass::{CharRange, class_digit, class_space, class_word};

pub fn parse(pattern: &str, flags: u16) -> Result<Ast, String> {
    let mut state = ParseState::new(pattern, flags);
    let ast = parse_disjunction(&mut state)?;

    if !state.is_eof() {
        return Err(format!("Unexpected character at position {}", state.pos));
    }

    Ok(ast.simplify())
}

fn parse_disjunction(state: &mut ParseState) -> Result<Ast, String> {
    let mut alternatives = vec![parse_alternative(state)?];

    while state.peek() == Some('|') {
        state.next();
        alternatives.push(parse_alternative(state)?);
    }

    if alternatives.len() == 1 {
        Ok(alternatives.into_iter().next().unwrap())
    } else {
        Ok(Ast::Alt(alternatives))
    }
}

fn parse_alternative(state: &mut ParseState) -> Result<Ast, String> {
    let mut terms = Vec::new();

    while let Some(c) = state.peek() {
        if c == '|' || c == ')' {
            break;
        }
        terms.push(parse_term(state)?);
    }

    if terms.is_empty() {
        Ok(Ast::Empty)
    } else if terms.len() == 1 {
        Ok(terms.into_iter().next().unwrap())
    } else {
        Ok(Ast::Concat(terms))
    }
}

fn parse_term(state: &mut ParseState) -> Result<Ast, String> {
    let atom = parse_atom(state)?;

    if let Some(q) = parse_quantifier(state)? {
        Ok(Ast::Quant(Box::new(atom), q))
    } else {
        Ok(atom)
    }
}

fn parse_quantifier(state: &mut ParseState) -> Result<Option<Quantifier>, String> {
    let c = match state.peek() {
        Some(c) if "*+?{".contains(c) => c,
        _ => return Ok(None),
    };

    let q = match c {
        '*' => {
            state.next();
            Quantifier::star()
        }
        '+' => {
            state.next();
            Quantifier::plus()
        }
        '?' => {
            state.next();
            Quantifier::question()
        }
        '{' => parse_brace_quantifier(state)?,
        _ => unreachable!(),
    };

    let q = if state.peek() == Some('?') {
        state.next();
        Quantifier { greedy: false, ..q }
    } else {
        q
    };

    Ok(Some(q))
}

fn parse_brace_quantifier(state: &mut ParseState) -> Result<Quantifier, String> {
    state.expect('{')?;

    let min = state
        .parse_number()
        .ok_or_else(|| "Expected number in quantifier".to_string())?;

    let max = if state.peek() == Some(',') {
        state.next();
        if state.peek() == Some('}') {
            None
        } else {
            Some(
                state
                    .parse_number()
                    .ok_or_else(|| "Expected number in quantifier".to_string())?,
            )
        }
    } else {
        Some(min)
    };

    state.expect('}')?;

    if let Some(max_val) = max {
        if min > max_val {
            return Err("Invalid quantifier: min > max".to_string());
        }
    }

    Ok(Quantifier::range(min, max))
}

fn parse_atom(state: &mut ParseState) -> Result<Ast, String> {
    match state.peek() {
        None => Err("Unexpected end of pattern".to_string()),
        Some('^') => {
            state.next();
            Ok(Ast::StartOfLine)
        }
        Some('$') => {
            state.next();
            Ok(Ast::EndOfLine)
        }
        Some('.') => {
            state.next();
            if state.dot_all {
                Ok(Ast::AnyAll)
            } else {
                Ok(Ast::Any)
            }
        }
        Some('\\') => parse_escape(state),
        Some('(') => parse_group(state),
        Some('[') => parse_char_class(state),
        Some(c) if is_special_char(c) => Err(format!(
            "Unexpected special character '{}' at position {}",
            c, state.pos
        )),
        Some(_) => {
            let c = state.next().unwrap();
            Ok(Ast::Char(c))
        }
    }
}

fn parse_escape(state: &mut ParseState) -> Result<Ast, String> {
    state.expect('\\')?;

    let c = state
        .next()
        .ok_or_else(|| "Unexpected end after \\".to_string())?;

    match c {
        'b' => Ok(Ast::Char('\x08')),
        'f' => Ok(Ast::Char('\x0C')),
        'n' => Ok(Ast::Char('\n')),
        'r' => Ok(Ast::Char('\r')),
        't' => Ok(Ast::Char('\t')),
        'v' => Ok(Ast::Char('\x0B')),
        'd' => Ok(Ast::Class(CharClass {
            negated: false,
            ranges: class_digit(),
        })),
        'D' => Ok(Ast::Class(CharClass {
            negated: true,
            ranges: class_digit(),
        })),
        's' => Ok(Ast::Class(CharClass {
            negated: false,
            ranges: class_space(),
        })),
        'S' => Ok(Ast::Class(CharClass {
            negated: true,
            ranges: class_space(),
        })),
        'w' => Ok(Ast::Class(CharClass {
            negated: false,
            ranges: class_word(),
        })),
        'W' => Ok(Ast::Class(CharClass {
            negated: true,
            ranges: class_word(),
        })),
        'B' => Ok(Ast::NotWordBoundary),
        'x' => parse_hex_escape(state, 2),
        'u' => parse_unicode_escape(state),
        '0' => {
            if state.is_unicode {
                if state.peek().map_or(false, |c| c.is_ascii_digit()) {
                    return Err("Invalid octal escape in unicode mode".to_string());
                }
                Ok(Ast::Char('\0'))
            } else {
                parse_octal_escape(state)
            }
        }
        '1'..='9' => {
            let start = state.pos - 1;
            let num = state.parse_number().unwrap_or(0);

            if num > 0 && num < state.capture_count as u32 {
                Ok(Ast::BackRef(num as usize))
            } else if !state.is_unicode {
                state.pos = start;
                parse_octal_escape(state)
            } else {
                Err(format!("Invalid backreference: {}", num))
            }
        }
        'p' | 'P' if state.is_unicode => parse_unicode_property(state, c == 'P'),
        'k' => parse_named_backref(state),
        c => Ok(Ast::Char(c)),
    }
}

fn parse_group(state: &mut ParseState) -> Result<Ast, String> {
    state.expect('(')?;

    if state.peek() == Some('?') {
        state.next();

        match state.peek() {
            Some(':') => {
                state.next();
                let inner = parse_disjunction(state)?;
                state.expect(')')?;
                Ok(inner)
            }
            Some('=') => {
                state.next();
                let inner = parse_disjunction(state)?;
                state.expect(')')?;
                Ok(Ast::Lookahead(Box::new(inner)))
            }
            Some('!') => {
                state.next();
                let inner = parse_disjunction(state)?;
                state.expect(')')?;
                Ok(Ast::NegativeLookahead(Box::new(inner)))
            }
            Some('<') => {
                state.next();
                let name = parse_group_name(state)?;
                state.expect('>')?;

                state.capture_count += 1;
                state.named_groups.push(name.clone());

                let inner = parse_disjunction(state)?;
                state.expect(')')?;

                Ok(Ast::Capture(Box::new(inner), Some(name)))
            }
            Some(c) => Err(format!("Unknown group extension: ?{}", c)),
            None => Err("Unexpected end after (?".to_string()),
        }
    } else {
        state.capture_count += 1;

        let inner = parse_disjunction(state)?;
        state.expect(')')?;

        Ok(Ast::Capture(Box::new(inner), None))
    }
}

fn parse_char_class(state: &mut ParseState) -> Result<Ast, String> {
    state.expect('[')?;

    let negated = if state.peek() == Some('^') {
        state.next();
        true
    } else {
        false
    };

    let mut ranges = CharRange::new();
    let mut prev_char: Option<char> = None;

    loop {
        match state.peek() {
            None => return Err("Unterminated character class".to_string()),
            Some(']') if prev_char.is_some() => {
                state.next();
                break;
            }
            Some(']') if ranges.is_empty() => {
                state.next();
                ranges.add_char(']');
                prev_char = Some(']');
            }
            Some(']') => {
                state.next();
                break;
            }
            Some('-') if prev_char.is_some() && state.peek_at(1) != Some(']') => {
                state.next();
                let end = parse_class_atom(state)?;
                if let Some(start) = prev_char {
                    ranges.add_range(start as u32, end as u32);
                }
                prev_char = None;
            }
            Some('-') => {
                state.next();
                ranges.add_char('-');
                prev_char = Some('-');
            }
            Some('\\') => {
                let atom = parse_class_escape(state)?;
                if let Some(c) = atom {
                    ranges.add_char(c);
                    prev_char = Some(c);
                } else {
                    prev_char = None;
                }
            }
            Some(c) => {
                state.next();
                ranges.add_char(c);
                prev_char = Some(c);
            }
        }
    }

    if negated {
        ranges.invert();
    }

    Ok(Ast::Class(CharClass { negated, ranges }))
}

fn parse_class_atom(state: &mut ParseState) -> Result<char, String> {
    match state.peek() {
        None => Err("Unexpected end in character class".to_string()),
        Some('\\') => parse_class_escape(state)?
            .ok_or_else(|| "Class escapes not allowed in range".to_string()),
        Some(c) => {
            state.next();
            Ok(c)
        }
    }
}

fn parse_class_escape(state: &mut ParseState) -> Result<Option<char>, String> {
    state.expect('\\')?;

    let c = state
        .next()
        .ok_or_else(|| "Unexpected end after \\".to_string())?;

    match c {
        'b' => Ok(Some('\x08')),
        'f' => Ok(Some('\x0C')),
        'n' => Ok(Some('\n')),
        'r' => Ok(Some('\r')),
        't' => Ok(Some('\t')),
        'v' => Ok(Some('\x0B')),
        'x' => parse_hex_escape_char(state, 2),
        'u' => parse_unicode_escape_char(state),
        '0' if state.is_unicode => {
            if state.peek().map_or(false, |c| c.is_ascii_digit()) {
                Err("Invalid octal escape".to_string())
            } else {
                Ok(Some('\0'))
            }
        }

        'd' | 'D' | 's' | 'S' | 'w' | 'W' => Ok(None),
        c => Ok(Some(c)),
    }
}

fn parse_hex_escape(state: &mut ParseState, digits: usize) -> Result<Ast, String> {
    let c = parse_hex_escape_char(state, digits)?.ok_or("Invalid hex escape")?;
    Ok(Ast::Char(c))
}

fn parse_hex_escape_char(state: &mut ParseState, digits: usize) -> Result<Option<char>, String> {
    let mut val: u32 = 0;
    for _ in 0..digits {
        let c = state
            .next()
            .ok_or_else(|| "Unexpected end in hex escape".to_string())?;
        let digit = c
            .to_digit(16)
            .ok_or_else(|| format!("Invalid hex digit: {}", c))?;
        val = val * 16 + digit;
    }
    if (0xD800..=0xDFFF).contains(&val) {
        return Ok(Some('\u{FFFD}'));
    }
    char::from_u32(val)
        .map(Some)
        .ok_or_else(|| "Invalid Unicode code point".to_string())
}

fn parse_unicode_escape(state: &mut ParseState) -> Result<Ast, String> {
    let c = parse_unicode_escape_char(state)?;
    Ok(Ast::Char(c.unwrap_or('\u{FFFD}')))
}

fn parse_unicode_escape_char(state: &mut ParseState) -> Result<Option<char>, String> {
    if state.peek() == Some('{') {
        state.next();
        let mut val: u32 = 0;
        loop {
            match state.next() {
                Some('}') => break,
                Some(c) => {
                    let digit = c
                        .to_digit(16)
                        .ok_or_else(|| format!("Invalid hex digit: {}", c))?;
                    val = val * 16 + digit;
                    if val > 0x10FFFF {
                        return Err("Unicode code point out of range".to_string());
                    }
                }
                None => return Err("Unterminated Unicode escape".to_string()),
            }
        }
        char::from_u32(val)
            .map(Some)
            .ok_or_else(|| "Invalid Unicode code point".to_string())
    } else {
        parse_hex_escape_char(state, 4)
    }
}

fn parse_octal_escape(state: &mut ParseState) -> Result<Ast, String> {
    let mut val: u32 = 0;
    let mut count = 0;

    while count < 3 {
        match state.peek() {
            Some(c) if c >= '0' && c <= '7' => {
                state.next();
                val = val * 8 + (c as u32 - '0' as u32);
                count += 1;
            }
            _ => break,
        }
    }

    if count == 0 {
        Ok(Ast::Char('\0'))
    } else {
        char::from_u32(val)
            .map(Ast::Char)
            .ok_or_else(|| "Invalid octal escape".to_string())
    }
}

fn parse_unicode_property(state: &mut ParseState, negated: bool) -> Result<Ast, String> {
    state.expect('{')?;

    let mut name = String::new();
    loop {
        match state.next() {
            Some('}') => break,
            Some(c) => name.push(c),
            None => return Err("Unterminated property name".to_string()),
        }
    }

    let ranges = match name.as_str() {
        "ASCII" => CharRange::from_range('\0', '\x7F'),
        "ASCII_Hex_Digit" => {
            let mut r = CharRange::from_range('0', '9');
            r.add_range('A' as u32, 'F' as u32);
            r.add_range('a' as u32, 'f' as u32);
            r
        }
        _ => CharRange::new(),
    };

    Ok(Ast::Class(CharClass { negated, ranges }))
}

fn parse_named_backref(state: &mut ParseState) -> Result<Ast, String> {
    state.expect('<')?;
    let name = parse_group_name(state)?;
    state.expect('>')?;

    Ok(Ast::NamedBackRef(name))
}

fn parse_group_name(state: &mut ParseState) -> Result<String, String> {
    let mut name = String::new();

    loop {
        match state.peek() {
            Some('>') | Some(')') => break,
            Some(c) if c.is_alphanumeric() || c == '_' => {
                state.next();
                name.push(c);
            }
            Some(c) => return Err(format!("Invalid character in group name: {}", c)),
            None => return Err("Unexpected end in group name".to_string()),
        }
    }

    if name.is_empty() {
        Err("Empty group name".to_string())
    } else {
        Ok(name)
    }
}

fn is_special_char(c: char) -> bool {
    matches!(
        c,
        '^' | '$' | '\\' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
    )
}

trait PeekAt {
    fn peek_at(&self, offset: usize) -> Option<char>;
}

impl PeekAt for ParseState {
    fn peek_at(&self, offset: usize) -> Option<char> {
        self.pattern.get(self.pos + offset).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let ast = parse("abc", 0).unwrap();
        match ast {
            Ast::Concat(nodes) => {
                assert_eq!(nodes.len(), 3);
            }
            _ => panic!("Expected concat"),
        }
    }

    #[test]
    fn test_parse_alt() {
        let ast = parse("a|b|c", 0).unwrap();
        match ast {
            Ast::Alt(nodes) => {
                assert_eq!(nodes.len(), 3);
            }
            _ => panic!("Expected alt"),
        }
    }

    #[test]
    fn test_parse_quantifier() {
        let ast = parse("a*", 0).unwrap();
        match ast {
            Ast::Quant(_inner, q) => {
                assert_eq!(q.min, 0);
                assert_eq!(q.max, None);
                assert!(q.greedy);
            }
            _ => panic!("Expected quant"),
        }
    }

    #[test]
    fn test_parse_capture() {
        let ast = parse("(ab)+", 0).unwrap();
        match ast {
            Ast::Quant(inner, _) => match *inner {
                Ast::Capture(_, None) => {}
                _ => panic!("Expected capture"),
            },
            _ => panic!("Expected quant"),
        }
    }

    #[test]
    fn test_parse_char_class() {
        let ast = parse("[a-z]", 0).unwrap();
        match ast {
            Ast::Class(cc) => {
                assert!(!cc.negated);
                assert!(cc.ranges.contains_char('a'));
                assert!(cc.ranges.contains_char('z'));
                assert!(!cc.ranges.contains_char('A'));
            }
            _ => panic!("Expected class"),
        }
    }
}
