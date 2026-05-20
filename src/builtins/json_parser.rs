use crate::object::array_obj::JSArrayObject;
use crate::object::object::JSObject;
use crate::runtime::context::JSContext;
use crate::util::memchr::memchr2;
use crate::value::JSValue;

#[inline(always)]
fn find_string_end(input: &[u8], pos: usize) -> usize {
    memchr2(b'"', b'\\', &input[pos..])
        .map(|o| pos + o)
        .unwrap_or(input.len())
}

#[inline(always)]
fn skip_ws_bulk(input: &[u8], mut pos: usize) -> usize {
    let len = input.len();
    loop {
        if pos >= len {
            return len;
        }
        match input[pos] {
            b' ' | b'\t' | b'\r' | b'\n' => pos += 1,
            _ => return pos,
        }
    }
}

pub struct JsonParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    pub fn new(input: &'a str) -> Self {
        JsonParser {
            input: input.as_bytes(),
            pos: 0,
        }
    }

    #[inline]
    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    #[inline]
    fn advance(&mut self) -> Option<u8> {
        let ch = self.input.get(self.pos).copied();
        self.pos += 1;
        ch
    }

    #[inline]
    fn skip_whitespace(&mut self) {
        self.pos = skip_ws_bulk(self.input, self.pos);
    }

    #[inline]
    fn expect_byte(&mut self, expected: u8) -> Result<(), String> {
        match self.advance() {
            Some(b) if b == expected => Ok(()),
            Some(b) => Err(format!(
                "JSON: expected '{}', got '{}' at pos {}",
                expected as char,
                b as char,
                self.pos - 1
            )),
            None => Err("JSON: unexpected end of input".to_string()),
        }
    }

    pub fn parse_value(&mut self, ctx: &mut JSContext) -> Result<JSValue, String> {
        self.skip_whitespace();
        let ch = self.peek().ok_or("JSON: unexpected end of input")?;
        match ch {
            b'"' => self.parse_string(ctx),
            b'{' => self.parse_object(ctx),
            b'[' => self.parse_array(ctx),
            b't' => self.parse_literal(b"true", JSValue::bool(true)),
            b'f' => self.parse_literal(b"false", JSValue::bool(false)),
            b'n' => self.parse_literal(b"null", JSValue::null()),
            b'-' | b'0'..=b'9' => self.parse_number(),
            _ => Err(format!(
                "JSON: unexpected character '{}' at pos {}",
                ch as char, self.pos
            )),
        }
    }

    fn parse_literal(&mut self, expected: &[u8], value: JSValue) -> Result<JSValue, String> {
        let end = self.pos + expected.len();
        if end <= self.input.len() && &self.input[self.pos..end] == expected {
            self.pos = end;
            Ok(value)
        } else {
            Err(format!("JSON: invalid literal at pos {}", self.pos))
        }
    }

    fn parse_number(&mut self) -> Result<JSValue, String> {
        let start = self.pos;

        if self.peek() == Some(b'-') {
            self.pos += 1;
        }

        if self.peek() == Some(b'0') {
            self.pos += 1;

            if let Some(b'0'..=b'9') = self.peek() {
                return Err("JSON: leading zeros not allowed".to_string());
            }
        } else {
            while let Some(b'0'..=b'9') = self.peek() {
                self.pos += 1;
            }
        }

        let has_fraction;
        let has_exponent;

        if self.peek() == Some(b'.') {
            self.pos += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err("JSON: expected digit after decimal point".to_string());
            }
            while let Some(b'0'..=b'9') = self.peek() {
                self.pos += 1;
            }
            has_fraction = true;
        } else {
            has_fraction = false;
        }

        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err("JSON: expected digit in exponent".to_string());
            }
            while let Some(b'0'..=b'9') = self.peek() {
                self.pos += 1;
            }
            has_exponent = true;
        } else {
            has_exponent = false;
        }

        if !has_fraction && !has_exponent {
            let bytes = &self.input[start..self.pos];
            let (negative, digits) = if bytes[0] == b'-' {
                (true, &bytes[1..])
            } else {
                (false, bytes)
            };

            if digits.len() <= 18 {
                let mut n: u64 = 0;
                for &b in digits {
                    n = n * 10 + (b - b'0') as u64;
                }
                let i = if negative {
                    if n <= i64::MAX as u64 + 1 {
                        n.wrapping_neg() as i64
                    } else {
                        let s = unsafe { std::str::from_utf8_unchecked(bytes) };
                        let f: f64 = s.parse().map_err(|_| "JSON: invalid number".to_string())?;
                        return Ok(JSValue::new_float(f));
                    }
                } else if n <= i64::MAX as u64 {
                    n as i64
                } else {
                    let s = unsafe { std::str::from_utf8_unchecked(bytes) };
                    let f: f64 = s.parse().map_err(|_| "JSON: invalid number".to_string())?;
                    return Ok(JSValue::new_float(f));
                };
                return Ok(JSValue::new_int(i));
            }

            let num_str = unsafe { std::str::from_utf8_unchecked(&self.input[start..self.pos]) };
            if let Ok(i) = num_str.parse::<i64>() {
                return Ok(JSValue::new_int(i));
            }
            let f: f64 = num_str
                .parse()
                .map_err(|_| "JSON: invalid number".to_string())?;
            return Ok(JSValue::new_float(f));
        }

        let num_str = unsafe { std::str::from_utf8_unchecked(&self.input[start..self.pos]) };
        let f: f64 = num_str
            .parse()
            .map_err(|_| "JSON: invalid number".to_string())?;
        Ok(JSValue::new_float(f))
    }

    fn parse_string(&mut self, ctx: &mut JSContext) -> Result<JSValue, String> {
        self.expect_byte(b'"')?;
        let start = self.pos;

        let first_special = find_string_end(self.input, self.pos);

        if first_special < self.input.len() && self.input[first_special] == b'"' {
            let slice = &self.input[start..first_special];
            let s = unsafe { std::str::from_utf8_unchecked(slice) };
            let atom = ctx.intern(s);
            self.pos = first_special + 1;
            return Ok(JSValue::new_string(atom));
        }

        let mut buf = if first_special > start {
            let slice = &self.input[start..first_special];
            let s = unsafe { std::str::from_utf8_unchecked(slice) };
            s.to_string()
        } else {
            String::new()
        };
        self.pos = first_special;

        loop {
            let b = self.advance().ok_or("JSON: unterminated string")?;
            match b {
                b'"' => {
                    let atom = ctx.intern(&buf);
                    return Ok(JSValue::new_string(atom));
                }
                b'\\' => {
                    let escaped = self.advance().ok_or("JSON: unterminated escape")?;
                    match escaped {
                        b'"' => buf.push('"'),
                        b'\\' => buf.push('\\'),
                        b'/' => buf.push('/'),
                        b'b' => buf.push('\x08'),
                        b'f' => buf.push('\x0c'),
                        b'n' => buf.push('\n'),
                        b'r' => buf.push('\r'),
                        b't' => buf.push('\t'),
                        b'u' => {
                            let hex = self.parse_hex_escape()?;
                            buf.push(hex);
                        }
                        _ => return Err(format!("JSON: invalid escape '\\{}'", escaped as char)),
                    }

                    let next_special = find_string_end(self.input, self.pos);
                    let span = &self.input[self.pos..next_special];

                    buf.push_str(unsafe { std::str::from_utf8_unchecked(span) });
                    self.pos = next_special;
                }
                _ => unreachable!(
                    "find_string_end guarantees self.pos stops at '\"' or '\\', got 0x{:02X}",
                    b
                ),
            }
        }
    }

    fn parse_hex_escape(&mut self) -> Result<char, String> {
        let mut code = 0u32;
        for _ in 0..4 {
            let b = self.advance().ok_or("JSON: unterminated unicode escape")?;
            let digit = match b {
                b'0'..=b'9' => (b - b'0') as u32,
                b'a'..=b'f' => (b - b'a') as u32 + 10,
                b'A'..=b'F' => (b - b'A') as u32 + 10,
                _ => return Err(format!("JSON: invalid hex digit '{}'", b as char)),
            };
            code = (code << 4) | digit;
        }

        if (0xD800..=0xDBFF).contains(&code) {
            if self.advance() != Some(b'\\') || self.advance() != Some(b'u') {
                return Err("JSON: expected low surrogate after high surrogate".to_string());
            }
            let mut low = 0u32;
            for _ in 0..4 {
                let b = self
                    .advance()
                    .ok_or("JSON: unterminated surrogate escape")?;
                let digit = match b {
                    b'0'..=b'9' => (b - b'0') as u32,
                    b'a'..=b'f' => (b - b'a') as u32 + 10,
                    b'A'..=b'F' => (b - b'A') as u32 + 10,
                    _ => {
                        return Err(format!(
                            "JSON: invalid hex digit in surrogate '{}'",
                            b as char
                        ));
                    }
                };
                low = (low << 4) | digit;
            }
            if !(0xDC00..=0xDFFF).contains(&low) {
                return Err("JSON: invalid low surrogate".to_string());
            }
            let combined = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
            char::from_u32(combined)
                .ok_or("JSON: invalid codepoint from surrogate pair".to_string())
        } else {
            char::from_u32(code).ok_or("JSON: invalid unicode codepoint".to_string())
        }
    }

    fn parse_array(&mut self, ctx: &mut JSContext) -> Result<JSValue, String> {
        self.expect_byte(b'[')?;
        self.skip_whitespace();

        let mut elements = Vec::new();

        if self.peek() != Some(b']') {
            loop {
                self.skip_whitespace();
                let val = self.parse_value(ctx)?;
                elements.push(val);
                self.skip_whitespace();
                match self.peek() {
                    Some(b',') => {
                        self.pos += 1;
                    }
                    Some(b']') => break,
                    _ => return Err("JSON: expected ',' or ']' in array".to_string()),
                }
            }
        }
        self.expect_byte(b']')?;

        let len = elements.len();
        let mut arr = JSArrayObject::from_elements(elements);
        if let Some(proto_ptr) = ctx.get_array_prototype() {
            arr.header.set_prototype_raw(proto_ptr);
        }
        let len_atom = ctx.common_atoms.length;
        arr.header.set(len_atom, JSValue::new_int(len as i64));

        let ptr = Box::into_raw(Box::new(arr)) as usize;
        ctx.runtime_mut().gc_heap_mut().track_array(ptr);
        Ok(JSValue::new_object(ptr))
    }

    fn parse_object(&mut self, ctx: &mut JSContext) -> Result<JSValue, String> {
        self.expect_byte(b'{')?;
        self.skip_whitespace();

        let mut obj = JSObject::new();

        if self.peek() != Some(b'}') {
            loop {
                self.skip_whitespace();

                if self.peek() != Some(b'"') {
                    return Err("JSON: expected string key in object".to_string());
                }
                let key_val = self.parse_string(ctx)?;
                let key_atom = key_val.get_atom();

                self.skip_whitespace();
                self.expect_byte(b':')?;

                self.skip_whitespace();
                let val = self.parse_value(ctx)?;
                obj.set(key_atom, val);

                self.skip_whitespace();
                match self.peek() {
                    Some(b',') => {
                        self.pos += 1;
                    }
                    Some(b'}') => break,
                    _ => return Err("JSON: expected ',' or '}' in object".to_string()),
                }
            }
        }
        self.expect_byte(b'}')?;

        let ptr = Box::into_raw(Box::new(obj)) as usize;
        Ok(JSValue::new_object(ptr))
    }
}
