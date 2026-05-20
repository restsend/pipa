pub static ASCII_CLASS_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];

    let mut i = 0u8;
    while i < 128 {
        let mut flags = 0u8;

        if i >= b'0' && i <= b'9' {
            flags |= CLASS_DIGIT;
        }

        if (i >= b'A' && i <= b'Z')
            || (i >= b'a' && i <= b'z')
            || (i >= b'0' && i <= b'9')
            || i == b'_'
        {
            flags |= CLASS_WORD;
        }

        if matches!(i, b'\t' | b'\n' | b'\r' | b'\x0C' | b'\x0B' | b' ') {
            flags |= CLASS_SPACE;
        }

        if matches!(i, b'\t' | b' ') {
            flags |= CLASS_HSPACE;
        }

        if matches!(i, b'\n' | b'\r' | b'\x0C' | b'\x0B') {
            flags |= CLASS_VSPACE;
        }

        if i >= b'a' && i <= b'z' {
            flags |= CLASS_LOWER;
        }

        if i >= b'A' && i <= b'Z' {
            flags |= CLASS_UPPER;
        }

        if (i >= b'A' && i <= b'Z') || (i >= b'a' && i <= b'z') {
            flags |= CLASS_ALPHA;
        }

        if matches!(i, b' ' | b'\t') {
            flags |= CLASS_HSPACE;
        }

        table[i as usize] = flags;
        i += 1;
    }

    let mut i: usize = 128;
    while i < 256 {
        table[i] = CLASS_WORD | CLASS_ALPHA;
        i += 1;
    }

    table
};

pub const CLASS_DIGIT: u8 = 1 << 0;
pub const CLASS_WORD: u8 = 1 << 1;
pub const CLASS_SPACE: u8 = 1 << 2;
pub const CLASS_HSPACE: u8 = 1 << 3;
pub const CLASS_VSPACE: u8 = 1 << 4;
pub const CLASS_LOWER: u8 = 1 << 5;
pub const CLASS_UPPER: u8 = 1 << 6;
pub const CLASS_ALPHA: u8 = 1 << 7;

#[inline(always)]
pub fn is_ascii_class(c: u8, class: u8) -> bool {
    ASCII_CLASS_TABLE[c as usize] & class != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digit_lookup() {
        assert!(is_ascii_class(b'0', CLASS_DIGIT));
        assert!(is_ascii_class(b'9', CLASS_DIGIT));
        assert!(!is_ascii_class(b'a', CLASS_DIGIT));
        assert!(!is_ascii_class(b' ', CLASS_DIGIT));
    }

    #[test]
    fn test_word_lookup() {
        assert!(is_ascii_class(b'a', CLASS_WORD));
        assert!(is_ascii_class(b'Z', CLASS_WORD));
        assert!(is_ascii_class(b'_', CLASS_WORD));
        assert!(is_ascii_class(b'5', CLASS_WORD));
        assert!(!is_ascii_class(b' ', CLASS_WORD));
        assert!(!is_ascii_class(b'-', CLASS_WORD));
    }

    #[test]
    fn test_space_lookup() {
        assert!(is_ascii_class(b' ', CLASS_SPACE));
        assert!(is_ascii_class(b'\t', CLASS_SPACE));
        assert!(is_ascii_class(b'\n', CLASS_SPACE));
        assert!(!is_ascii_class(b'a', CLASS_SPACE));
    }

    #[test]
    fn test_all_ascii_chars() {
        for i in b'0'..=b'9' {
            assert!(is_ascii_class(i, CLASS_DIGIT), "Digit {} not classified", i);
        }

        for i in b'A'..=b'Z' {
            assert!(is_ascii_class(i, CLASS_ALPHA), "Upper {} not classified", i);
        }
        for i in b'a'..=b'z' {
            assert!(is_ascii_class(i, CLASS_ALPHA), "Lower {} not classified", i);
        }

        assert!(is_ascii_class(b' ', CLASS_SPACE));
        assert!(is_ascii_class(b'\t', CLASS_SPACE));
    }
}
