use crate::util::memchr::memmem;

#[derive(Clone)]
pub struct StringSearcher {
    pattern: Vec<u8>,

    skip_table: [usize; 256],

    pattern_len: usize,
}

impl StringSearcher {
    pub fn new(pattern: &str) -> Option<Self> {
        let pattern = pattern.as_bytes();
        let pattern_len = pattern.len();

        if pattern_len == 0 {
            return None;
        }

        if pattern_len < 3 {
            return None;
        }

        let mut skip_table = [pattern_len; 256];

        for i in 0..pattern_len - 1 {
            skip_table[pattern[i] as usize] = pattern_len - 1 - i;
        }

        Some(Self {
            pattern: pattern.to_vec(),
            skip_table,
            pattern_len,
        })
    }

    #[inline(always)]
    pub fn find(&self, text: &str) -> Option<usize> {
        let text = text.as_bytes();
        let text_len = text.len();
        let pat = &self.pattern;
        let pat_len = self.pattern_len;

        if pat_len > text_len {
            return None;
        }

        let mut pos = 0;
        let max_pos = text_len - pat_len;

        while pos <= max_pos {
            let mut i = pat_len - 1;

            while text[pos + i] == pat[i] {
                if i == 0 {
                    return Some(pos);
                }
                i -= 1;
            }

            let bad_char = text[pos + pat_len - 1];
            pos += self.skip_table[bad_char as usize];
        }

        None
    }

    pub fn find_all(&self, text: &str) -> Vec<(usize, usize)> {
        let mut matches = Vec::new();
        let mut pos = 0;

        while let Some(m) = self.find(&text[pos..]) {
            let abs_pos = pos + m;
            matches.push((abs_pos, abs_pos + self.pattern_len));
            pos = abs_pos + self.pattern_len;

            if pos >= text.len() {
                break;
            }
        }

        matches
    }
}

pub fn fast_find(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    if needle.len() > haystack.len() {
        return None;
    }

    if needle.len() < 4 {
        return memmem::find(haystack.as_bytes(), needle.as_bytes());
    }

    if let Some(searcher) = StringSearcher::new(needle) {
        searcher.find(haystack)
    } else {
        memmem::find(haystack.as_bytes(), needle.as_bytes())
    }
}

pub fn fast_find_with(searcher: &StringSearcher, haystack: &str) -> Option<usize> {
    searcher.find(haystack)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bmh_basic() {
        let searcher = StringSearcher::new("world").unwrap();
        assert_eq!(searcher.find("hello world"), Some(6));
    }

    #[test]
    fn test_bmh_not_found() {
        let searcher = StringSearcher::new("xyz").unwrap();
        assert_eq!(searcher.find("hello world"), None);
    }

    #[test]
    fn test_bmh_multiple() {
        let searcher = StringSearcher::new("abc").unwrap();
        assert_eq!(searcher.find("abcabcabc"), Some(0));
    }

    #[test]
    fn test_bmh_long_pattern() {
        let text = "The quick brown fox jumps over the lazy dog. ";
        let pattern = "jumps over";
        let searcher = StringSearcher::new(pattern).unwrap();
        assert_eq!(searcher.find(text), Some(20));
    }

    #[test]
    fn test_fast_find_short() {
        assert_eq!(fast_find("hello", "ll"), Some(2));
        assert_eq!(fast_find("hello", "abc"), None);
    }

    #[test]
    fn test_fast_find_long() {
        let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ";
        assert_eq!(fast_find(text, "consectetur"), Some(28));
    }

    #[test]
    fn test_find_all() {
        let searcher = StringSearcher::new("abc").unwrap();
        let matches = searcher.find_all("abcxabcxabc");
        assert_eq!(matches, vec![(0, 3), (4, 7), (8, 11)]);
    }
}
