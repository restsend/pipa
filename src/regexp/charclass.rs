#[derive(Debug, Clone, Default, PartialEq)]
pub struct CharRange {
    ranges: Vec<(u32, u32)>,
}

impl CharRange {
    pub fn new() -> Self {
        Self { ranges: Vec::new() }
    }

    pub fn from_char(c: char) -> Self {
        let cp = c as u32;
        Self {
            ranges: vec![(cp, cp + 1)],
        }
    }

    pub fn from_range(start: char, end: char) -> Self {
        Self {
            ranges: vec![(start as u32, end as u32 + 1)],
        }
    }

    pub fn add_char(&mut self, c: char) {
        self.add_range(c as u32, c as u32);
    }

    pub fn add_range(&mut self, start: u32, end: u32) {
        if start > end {
            return;
        }
        let new_range = (start, end + 1);

        let mut i = 0;
        let mut merged_start = new_range.0;
        let mut merged_end = new_range.1;

        while i < self.ranges.len() && self.ranges[i].1 < merged_start {
            i += 1;
        }

        let mut j = i;
        while j < self.ranges.len() && self.ranges[j].0 <= merged_end {
            merged_start = merged_start.min(self.ranges[j].0);
            merged_end = merged_end.max(self.ranges[j].1);
            j += 1;
        }

        if i < j {
            self.ranges[i] = (merged_start, merged_end);
            self.ranges.drain(i + 1..j);
        } else {
            self.ranges.insert(i, (merged_start, merged_end));
        }
    }

    pub fn contains(&self, c: u32) -> bool {
        let mut left = 0;
        let mut right = self.ranges.len();

        while left < right {
            let mid = left + (right - left) / 2;
            let (start, end) = self.ranges[mid];
            if c < start {
                right = mid;
            } else if c >= end {
                left = mid + 1;
            } else {
                return true;
            }
        }
        false
    }

    pub fn contains_char(&self, c: char) -> bool {
        self.contains(c as u32)
    }

    pub fn union(&mut self, other: &Self) {
        for &(start, end) in &other.ranges {
            self.add_range(start, end - 1);
        }
    }

    pub fn intersect(&mut self, other: &Self) {
        let mut result = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < self.ranges.len() && j < other.ranges.len() {
            let (s1, e1) = self.ranges[i];
            let (s2, e2) = other.ranges[j];

            let start = s1.max(s2);
            let end = e1.min(e2);

            if start < end {
                result.push((start, end));
            }

            if e1 < e2 {
                i += 1;
            } else {
                j += 1;
            }
        }

        self.ranges = result;
    }

    pub fn subtract(&mut self, other: &Self) {
        let mut result = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < self.ranges.len() {
            let (mut s1, e1) = self.ranges[i];

            while j < other.ranges.len() && other.ranges[j].1 <= s1 {
                j += 1;
            }

            let mut k = j;
            while k < other.ranges.len() && other.ranges[k].0 < e1 {
                let (s2, e2) = other.ranges[k];

                if s2 > s1 {
                    result.push((s1, s2));
                }

                s1 = s1.max(e2);
                if s1 >= e1 {
                    break;
                }
                k += 1;
            }

            if s1 < e1 {
                result.push((s1, e1));
            }

            i += 1;
        }

        self.ranges = result;
    }

    pub fn invert(&mut self) {
        if self.ranges.is_empty() {
            self.ranges = vec![(0, 0x110000)];
            return;
        }

        let mut result = Vec::new();
        let mut prev_end = 0u32;

        for &(start, end) in &self.ranges {
            if prev_end < start {
                result.push((prev_end, start));
            }
            prev_end = end;
        }

        if prev_end < 0x110000 {
            result.push((prev_end, 0x110000));
        }

        self.ranges = result;
    }

    pub fn canonicalize(&mut self, is_unicode: bool) {
        let mut new_ranges = self.clone();

        for &(start, end) in &self.ranges {
            for c in start..end {
                let folded = if is_unicode {
                    unicode_fold(c)
                } else {
                    ascii_fold(c)
                };
                if folded != c {
                    new_ranges.add_range(folded, folded);
                }
            }
        }

        *self = new_ranges;
    }

    pub fn as_slice(&self) -> &[(u32, u32)] {
        &self.ranges
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend_from_slice(&(self.ranges.len() as u16).to_le_bytes());

        for &(start, end) in &self.ranges {
            let end_val = if end > 0xFFFF { 0xFFFF } else { end as u16 };
            result.extend_from_slice(&(start as u16).to_le_bytes());
            result.extend_from_slice(&end_val.to_le_bytes());
        }
        result
    }

    pub fn to_bytes32(&self) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend_from_slice(&(self.ranges.len() as u16).to_le_bytes());

        for &(start, end) in &self.ranges {
            result.extend_from_slice(&start.to_le_bytes());
            result.extend_from_slice(&end.to_le_bytes());
        }
        result
    }
}

fn ascii_fold(c: u32) -> u32 {
    if c < 128 {
        if c >= b'a' as u32 && c <= b'z' as u32 {
            c - 32
        } else {
            c
        }
    } else {
        c
    }
}

fn unicode_fold(c: u32) -> u32 {
    if c < 128 {
        if c >= b'A' as u32 && c <= b'Z' as u32 {
            c + 32
        } else {
            c
        }
    } else {
        c
    }
}

pub fn class_digit() -> CharRange {
    CharRange::from_range('0', '9')
}

pub fn class_word() -> CharRange {
    let mut cr = CharRange::new();
    cr.add_range('0' as u32, '9' as u32);
    cr.add_range('A' as u32, 'Z' as u32);
    cr.add_char('_');
    cr.add_range('a' as u32, 'z' as u32);
    cr
}

pub fn class_space() -> CharRange {
    let mut cr = CharRange::new();

    cr.add_char('\t');
    cr.add_char('\n');
    cr.add_char('\r');
    cr.add_char(' ');

    cr.add_char('\x0B');
    cr.add_char('\x0C');

    cr.add_char('\u{00A0}');
    cr.add_char('\u{1680}');
    cr.add_range(0x2000, 0x200A);
    cr.add_char('\u{2028}');
    cr.add_char('\u{2029}');
    cr.add_char('\u{202F}');
    cr.add_char('\u{205F}');
    cr.add_char('\u{3000}');
    cr.add_char('\u{FEFF}');
    cr
}

pub fn is_word_char(c: u32) -> bool {
    (c >= b'0' as u32 && c <= b'9' as u32)
        || (c >= b'a' as u32 && c <= b'z' as u32)
        || (c >= b'A' as u32 && c <= b'Z' as u32)
        || c == b'_' as u32
}

pub fn is_line_terminator(c: u32) -> bool {
    c == b'\n' as u32 || c == b'\r' as u32 || c == 0x2028 || c == 0x2029
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_range_basic() {
        let mut cr = CharRange::new();
        cr.add_char('a');
        cr.add_char('b');
        cr.add_char('c');

        assert!(cr.contains_char('a'));
        assert!(cr.contains_char('b'));
        assert!(cr.contains_char('c'));
        assert!(!cr.contains_char('d'));
    }

    #[test]
    fn test_char_range_merge() {
        let mut cr = CharRange::new();
        cr.add_range('a' as u32, 'c' as u32);
        cr.add_range('d' as u32, 'f' as u32);

        assert!(cr.contains_char('a'));
        assert!(cr.contains_char('c'));
        assert!(cr.contains_char('d'));
        assert!(cr.contains_char('f'));
        assert!(!cr.contains_char('g'));
    }

    #[test]
    fn test_char_range_invert() {
        let mut cr = CharRange::from_range('a', 'z');
        cr.invert();

        assert!(!cr.contains_char('a'));
        assert!(!cr.contains_char('z'));
        assert!(cr.contains_char('A'));
        assert!(cr.contains_char('0'));
    }

    #[test]
    fn test_word_char() {
        assert!(is_word_char(b'0' as u32));
        assert!(is_word_char(b'a' as u32));
        assert!(is_word_char(b'Z' as u32));
        assert!(is_word_char(b'_' as u32));
        assert!(!is_word_char(b' ' as u32));
        assert!(!is_word_char(b'-' as u32));
    }
}
