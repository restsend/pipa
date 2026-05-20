#[inline(always)]
pub fn find_digit(haystack: &[u8]) -> Option<usize> {
    let len = haystack.len();
    let mut i = 0;

    while i + 16 <= len {
        let chunk = &haystack[i..i + 16];

        let b0 = chunk[0];
        if b0 <= b'9' && b0 >= b'0' {
            return Some(i);
        }
        let b1 = chunk[1];
        if b1 <= b'9' && b1 >= b'0' {
            return Some(i + 1);
        }
        let b2 = chunk[2];
        if b2 <= b'9' && b2 >= b'0' {
            return Some(i + 2);
        }
        let b3 = chunk[3];
        if b3 <= b'9' && b3 >= b'0' {
            return Some(i + 3);
        }
        let b4 = chunk[4];
        if b4 <= b'9' && b4 >= b'0' {
            return Some(i + 4);
        }
        let b5 = chunk[5];
        if b5 <= b'9' && b5 >= b'0' {
            return Some(i + 5);
        }
        let b6 = chunk[6];
        if b6 <= b'9' && b6 >= b'0' {
            return Some(i + 6);
        }
        let b7 = chunk[7];
        if b7 <= b'9' && b7 >= b'0' {
            return Some(i + 7);
        }
        let b8 = chunk[8];
        if b8 <= b'9' && b8 >= b'0' {
            return Some(i + 8);
        }
        let b9 = chunk[9];
        if b9 <= b'9' && b9 >= b'0' {
            return Some(i + 9);
        }
        let b10 = chunk[10];
        if b10 <= b'9' && b10 >= b'0' {
            return Some(i + 10);
        }
        let b11 = chunk[11];
        if b11 <= b'9' && b11 >= b'0' {
            return Some(i + 11);
        }
        let b12 = chunk[12];
        if b12 <= b'9' && b12 >= b'0' {
            return Some(i + 12);
        }
        let b13 = chunk[13];
        if b13 <= b'9' && b13 >= b'0' {
            return Some(i + 13);
        }
        let b14 = chunk[14];
        if b14 <= b'9' && b14 >= b'0' {
            return Some(i + 14);
        }
        let b15 = chunk[15];
        if b15 <= b'9' && b15 >= b'0' {
            return Some(i + 15);
        }
        i += 16;
    }

    while i < len {
        let b = haystack[i];
        if b <= b'9' && b >= b'0' {
            return Some(i);
        }
        i += 1;
    }

    None
}

#[inline(always)]
pub fn find_word_char(haystack: &[u8]) -> Option<usize> {
    let len = haystack.len();
    let mut i = 0;

    while i + 16 <= len {
        let chunk = &haystack[i..i + 16];

        if is_word_byte(chunk[0]) {
            return Some(i);
        }
        if is_word_byte(chunk[1]) {
            return Some(i + 1);
        }
        if is_word_byte(chunk[2]) {
            return Some(i + 2);
        }
        if is_word_byte(chunk[3]) {
            return Some(i + 3);
        }
        if is_word_byte(chunk[4]) {
            return Some(i + 4);
        }
        if is_word_byte(chunk[5]) {
            return Some(i + 5);
        }
        if is_word_byte(chunk[6]) {
            return Some(i + 6);
        }
        if is_word_byte(chunk[7]) {
            return Some(i + 7);
        }
        if is_word_byte(chunk[8]) {
            return Some(i + 8);
        }
        if is_word_byte(chunk[9]) {
            return Some(i + 9);
        }
        if is_word_byte(chunk[10]) {
            return Some(i + 10);
        }
        if is_word_byte(chunk[11]) {
            return Some(i + 11);
        }
        if is_word_byte(chunk[12]) {
            return Some(i + 12);
        }
        if is_word_byte(chunk[13]) {
            return Some(i + 13);
        }
        if is_word_byte(chunk[14]) {
            return Some(i + 14);
        }
        if is_word_byte(chunk[15]) {
            return Some(i + 15);
        }
        i += 16;
    }

    while i < len {
        if is_word_byte(haystack[i]) {
            return Some(i);
        }
        i += 1;
    }

    None
}

#[inline(always)]
fn is_word_byte(b: u8) -> bool {
    (b >= b'0' && b <= b'9') || (b >= b'A' && b <= b'Z') || (b >= b'a' && b <= b'z') || b == b'_'
}

#[inline(always)]
pub fn find_space(haystack: &[u8]) -> Option<usize> {
    let len = haystack.len();
    let mut i = 0;

    while i + 16 <= len {
        let chunk = &haystack[i..i + 16];

        if is_space_byte(chunk[0]) {
            return Some(i);
        }
        if is_space_byte(chunk[1]) {
            return Some(i + 1);
        }
        if is_space_byte(chunk[2]) {
            return Some(i + 2);
        }
        if is_space_byte(chunk[3]) {
            return Some(i + 3);
        }
        if is_space_byte(chunk[4]) {
            return Some(i + 4);
        }
        if is_space_byte(chunk[5]) {
            return Some(i + 5);
        }
        if is_space_byte(chunk[6]) {
            return Some(i + 6);
        }
        if is_space_byte(chunk[7]) {
            return Some(i + 7);
        }
        if is_space_byte(chunk[8]) {
            return Some(i + 8);
        }
        if is_space_byte(chunk[9]) {
            return Some(i + 9);
        }
        if is_space_byte(chunk[10]) {
            return Some(i + 10);
        }
        if is_space_byte(chunk[11]) {
            return Some(i + 11);
        }
        if is_space_byte(chunk[12]) {
            return Some(i + 12);
        }
        if is_space_byte(chunk[13]) {
            return Some(i + 13);
        }
        if is_space_byte(chunk[14]) {
            return Some(i + 14);
        }
        if is_space_byte(chunk[15]) {
            return Some(i + 15);
        }
        i += 16;
    }

    while i < len {
        if is_space_byte(haystack[i]) {
            return Some(i);
        }
        i += 1;
    }

    None
}

#[inline(always)]
fn is_space_byte(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0C | 0x0B)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_digit() {
        assert_eq!(find_digit(b"abc123def"), Some(3));
        assert_eq!(find_digit(b"abcdef"), None);
    }

    #[test]
    fn test_find_word_char() {
        assert_eq!(find_word_char(b"!!!hello"), Some(3));
    }
}
