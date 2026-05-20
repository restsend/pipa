#[inline(always)]
pub fn memchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    let len = haystack.len();
    if len == 0 {
        return None;
    }

    if len <= 16 {
        return haystack.iter().position(|&b| b == needle);
    }

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { memchr_avx2(needle, haystack) };
        }
        if is_x86_feature_detected!("sse2") {
            return unsafe { memchr_sse2(needle, haystack) };
        }
    }

    memchr_swar(needle, haystack)
}

fn memchr_swar(needle: u8, haystack: &[u8]) -> Option<usize> {
    let len = haystack.len();
    let ptr = haystack.as_ptr();
    let mut i = 0usize;

    while i < len {
        if (ptr.wrapping_add(i) as usize) % 8 == 0 {
            break;
        }
        unsafe {
            if *ptr.add(i) == needle {
                return Some(i);
            }
        }
        i += 1;
    }

    let pattern = needle as u64 * 0x0101010101010101u64;
    let remaining = len - i;
    let chunks = remaining / 8;

    for _ in 0..chunks {
        let chunk = unsafe { (ptr.add(i) as *const u64).read_unaligned() };
        let xor = chunk ^ pattern;
        if has_zero_byte(xor) {
            let bytes = xor.to_le_bytes();
            for j in 0..8 {
                if bytes[j] == 0 {
                    return Some(i + j);
                }
            }
        }
        i += 8;
    }

    while i < len {
        unsafe {
            if *ptr.add(i) == needle {
                return Some(i);
            }
        }
        i += 1;
    }

    None
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn memchr_sse2(needle: u8, haystack: &[u8]) -> Option<usize> {
    use core::arch::x86_64::*;

    let len = haystack.len();
    let ptr = haystack.as_ptr();
    let mut i = 0usize;

    unsafe {
        let vec = _mm_set1_epi8(needle as i8);
        while i + 16 <= len {
            let chunk = _mm_loadu_si128(ptr.add(i) as *const __m128i);
            let cmp = _mm_cmpeq_epi8(chunk, vec);
            let mask = _mm_movemask_epi8(cmp) as u32;
            if mask != 0 {
                return Some(i + mask.trailing_zeros() as usize);
            }
            i += 16;
        }
    }

    while i < len {
        unsafe {
            if *ptr.add(i) == needle {
                return Some(i);
            }
        }
        i += 1;
    }

    None
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn memchr_avx2(needle: u8, haystack: &[u8]) -> Option<usize> {
    use core::arch::x86_64::*;

    let len = haystack.len();
    let ptr = haystack.as_ptr();
    let mut i = 0usize;

    unsafe {
        let vec = _mm256_set1_epi8(needle as i8);
        while i + 32 <= len {
            let chunk = _mm256_loadu_si256(ptr.add(i) as *const __m256i);
            let cmp = _mm256_cmpeq_epi8(chunk, vec);
            let mask = _mm256_movemask_epi8(cmp) as u32;
            if mask != 0 {
                return Some(i + mask.trailing_zeros() as usize);
            }
            i += 32;
        }
    }

    while i < len {
        unsafe {
            if *ptr.add(i) == needle {
                return Some(i);
            }
        }
        i += 1;
    }

    None
}

#[inline(always)]
fn has_zero_byte(x: u64) -> bool {
    let y = x.wrapping_sub(0x0101010101010101u64);
    let z = !x;
    (y & z & 0x8080808080808080u64) != 0
}

#[inline(always)]
pub fn memchr2(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
    let len = haystack.len();
    if len == 0 {
        return None;
    }

    if len < 32 {
        return haystack.iter().position(|&b| b == n1 || b == n2);
    }

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { memchr2_avx2(n1, n2, haystack) };
        }
        if is_x86_feature_detected!("sse2") {
            return unsafe { memchr2_sse2(n1, n2, haystack) };
        }
    }

    memchr2_swar(n1, n2, haystack)
}

fn memchr2_swar(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
    let len = haystack.len();
    let ptr = haystack.as_ptr();
    let mut i = 0usize;

    let p1 = n1 as u64 * 0x0101010101010101u64;
    let p2 = n2 as u64 * 0x0101010101010101u64;

    while i + 8 <= len {
        let chunk = unsafe { (ptr.add(i) as *const u64).read_unaligned() };
        if has_zero_byte(chunk ^ p1) || has_zero_byte(chunk ^ p2) {
            break;
        }
        i += 8;
    }

    while i < len {
        let b = unsafe { *ptr.add(i) };
        if b == n1 || b == n2 {
            return Some(i);
        }
        i += 1;
    }

    None
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn memchr2_sse2(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
    use core::arch::x86_64::*;
    let len = haystack.len();
    let ptr = haystack.as_ptr();
    let mut i = 0usize;

    unsafe {
        let v1 = _mm_set1_epi8(n1 as i8);
        let v2 = _mm_set1_epi8(n2 as i8);
        while i + 16 <= len {
            let chunk = _mm_loadu_si128(ptr.add(i) as *const __m128i);
            let mask = _mm_movemask_epi8(_mm_or_si128(
                _mm_cmpeq_epi8(chunk, v1),
                _mm_cmpeq_epi8(chunk, v2),
            )) as u32;
            if mask != 0 {
                return Some(i + mask.trailing_zeros() as usize);
            }
            i += 16;
        }
    }

    while i < len {
        let b = unsafe { *ptr.add(i) };
        if b == n1 || b == n2 {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn memchr2_avx2(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
    use core::arch::x86_64::*;
    let len = haystack.len();
    let ptr = haystack.as_ptr();
    let mut i = 0usize;

    unsafe {
        let v1 = _mm256_set1_epi8(n1 as i8);
        let v2 = _mm256_set1_epi8(n2 as i8);
        while i + 32 <= len {
            let chunk = _mm256_loadu_si256(ptr.add(i) as *const __m256i);
            let mask = _mm256_movemask_epi8(_mm256_or_si256(
                _mm256_cmpeq_epi8(chunk, v1),
                _mm256_cmpeq_epi8(chunk, v2),
            )) as u32;
            if mask != 0 {
                return Some(i + mask.trailing_zeros() as usize);
            }
            i += 32;
        }
    }

    while i < len {
        let b = unsafe { *ptr.add(i) };
        if b == n1 || b == n2 {
            return Some(i);
        }
        i += 1;
    }
    None
}

pub mod memmem {
    use super::memchr;

    #[derive(Clone)]
    pub struct Finder<'a> {
        needle: &'a [u8],
        inner: Searcher<'a>,
    }

    #[derive(Clone)]
    enum Searcher<'a> {
        Trivial,
        TwoByte { first: u8, second: u8 },

        Short { pattern: &'a [u8] },

        Bmh { bmh: BmhSearcher<'a> },

        TwoWay { twoway: TwoWaySearcher },
    }

    impl<'a> Finder<'a> {
        pub fn new(needle: &'a [u8]) -> Self {
            let inner = match needle.len() {
                0 | 1 => Searcher::Trivial,
                2 => Searcher::TwoByte {
                    first: needle[0],
                    second: needle[1],
                },
                3..=4 => Searcher::Short { pattern: needle },
                5..=128 => Searcher::Bmh {
                    bmh: BmhSearcher::new(needle),
                },
                _ => Searcher::TwoWay {
                    twoway: TwoWaySearcher::new(needle),
                },
            };
            Self { needle, inner }
        }

        pub fn find(&self, haystack: &[u8]) -> Option<usize> {
            let n = self.needle.len();
            if n == 0 {
                return Some(0);
            }
            if n > haystack.len() {
                return None;
            }
            if n == 1 {
                return memchr(self.needle[0], haystack);
            }

            match &self.inner {
                Searcher::Trivial => unreachable!(),
                Searcher::TwoByte { first, second } => {
                    let max_pos = haystack.len().saturating_sub(2);
                    let mut pos = 0;
                    while pos <= max_pos {
                        if let Some(rel) = memchr(*first, &haystack[pos..=max_pos]) {
                            let candidate = pos + rel;
                            if haystack.get(candidate + 1) == Some(second) {
                                return Some(candidate);
                            }
                            pos = candidate + 1;
                        } else {
                            break;
                        }
                    }
                    None
                }
                Searcher::Short { pattern } => {
                    let pat_len = pattern.len();
                    let first = pattern[0];
                    let max_pos = haystack.len().saturating_sub(pat_len);
                    let mut pos = 0;
                    while pos <= max_pos {
                        if let Some(rel) = memchr(first, &haystack[pos..=max_pos]) {
                            let candidate = pos + rel;
                            if &haystack[candidate..candidate + pat_len] == *pattern {
                                return Some(candidate);
                            }
                            pos = candidate + 1;
                        } else {
                            break;
                        }
                    }
                    None
                }
                Searcher::Bmh { bmh } => bmh.find(haystack),
                Searcher::TwoWay { twoway } => twoway.find(haystack, self.needle),
            }
        }
    }

    pub fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        Finder::new(needle).find(haystack)
    }

    #[derive(Clone)]
    struct BmhSearcher<'a> {
        pattern: &'a [u8],
        pat_len: usize,
        skip_table: [usize; 256],
    }

    impl<'a> BmhSearcher<'a> {
        fn new(pattern: &'a [u8]) -> Self {
            let pat_len = pattern.len();
            let mut skip_table = [pat_len; 256];
            for i in 0..pat_len - 1 {
                skip_table[pattern[i] as usize] = pat_len - 1 - i;
            }
            Self {
                pattern,
                pat_len,
                skip_table,
            }
        }

        fn find(&self, text: &[u8]) -> Option<usize> {
            let pat = self.pattern;
            let pat_len = self.pat_len;
            let text_len = text.len();
            let mut pos = 0;
            let max_pos = text_len - pat_len;
            let last_byte = pat[pat_len - 1];

            while pos <= max_pos {
                if text[pos + pat_len - 1] == last_byte {
                    if &text[pos..pos + pat_len] == pat {
                        return Some(pos);
                    }
                }
                let bad_char = text[pos + pat_len - 1];
                pos += self.skip_table[bad_char as usize];
            }
            None
        }
    }

    #[derive(Clone)]
    struct TwoWaySearcher {
        critical_pos: usize,
        period: usize,
        pat_len: usize,
    }

    impl TwoWaySearcher {
        fn new(pattern: &[u8]) -> Self {
            let pat_len = pattern.len();
            let (critical_pos, period) = compute_two_way_params(pattern);
            Self {
                critical_pos,
                period,
                pat_len,
            }
        }

        fn find(&self, text: &[u8], pattern: &[u8]) -> Option<usize> {
            let pat_len = self.pat_len;
            let text_len = text.len();
            if pat_len > text_len {
                return None;
            }

            let mut pos = 0usize;
            let mut memory = 0usize;
            let max_pos = text_len - pat_len;

            while pos <= max_pos {
                let mut i = self.critical_pos.max(memory);
                while i < pat_len && pattern[i] == text[pos + i] {
                    i += 1;
                }
                if i < pat_len {
                    pos += i - self.critical_pos + 1;
                    if pos + pat_len > text_len {
                        break;
                    }
                    memory = 0;
                    continue;
                }

                let mut j = self.critical_pos;
                while j > memory && pattern[j - 1] == text[pos + j - 1] {
                    j -= 1;
                }
                if j <= memory {
                    return Some(pos);
                }

                pos += self.period;
                if pos + pat_len > text_len {
                    break;
                }
                memory = pat_len - self.period;
            }

            None
        }
    }

    fn compute_two_way_params(pattern: &[u8]) -> (usize, usize) {
        let pat_len = pattern.len();

        let (pos_ge, _) = maximal_suffix(pattern, false);
        let (pos_gt, _) = maximal_suffix(pattern, true);

        let pos = pos_ge.max(pos_gt);

        let mut p = 1;
        while p <= pat_len - pos {
            let mut ok = true;
            for i in (pos + p)..pat_len {
                if pattern[i] != pattern[i - p] {
                    ok = false;
                    break;
                }
            }
            if ok {
                break;
            }
            p += 1;
        }

        let mut period = p;
        for i in 0..pos {
            if i + period < pat_len && pattern[i] != pattern[i + period] {
                period = pat_len;
                break;
            }
        }

        (pos, period)
    }

    fn maximal_suffix(s: &[u8], strict: bool) -> (usize, usize) {
        let n = s.len();
        let mut i = 0usize;
        let mut j = 1usize;
        let mut k = 0usize;

        while j + k < n {
            let a = s[i + k];
            let b = s[j + k];
            if (strict && a < b) || (!strict && a <= b) {
                i += k + 1;
                k = 0;
                if i >= j {
                    j = i + 1;
                }
            } else {
                j += k + 1;
                k = 0;
            }
        }
        (i, j)
    }
}

#[cfg(test)]
mod tests {
    use super::memmem;
    use super::*;

    #[test]
    fn test_memchr_basic() {
        assert_eq!(memchr(b'x', b"hello world"), None);
        assert_eq!(memchr(b'o', b"hello world"), Some(4));
        assert_eq!(memchr(b'h', b"hello"), Some(0));
        assert_eq!(memchr(b'o', b"hello"), Some(4));
    }

    #[test]
    fn test_memchr_long() {
        let mut s = vec![b'a'; 1000];
        s.push(b'x');
        s.extend_from_slice(&[b'b'; 1000]);
        assert_eq!(memchr(b'x', &s), Some(1000));
    }

    #[test]
    fn test_memmem_basic() {
        assert_eq!(memmem::find(b"hello world", b"world"), Some(6));
        assert_eq!(memmem::find(b"hello world", b"xyz"), None);
        assert_eq!(memmem::find(b"abc", b"abc"), Some(0));
        assert_eq!(memmem::find(b"abc", b""), Some(0));
    }

    #[test]
    fn test_memmem_finder() {
        let finder = memmem::Finder::new(b"needle");
        assert_eq!(finder.find(b"hayneedlestack"), Some(3));
        assert_eq!(finder.find(b"none"), None);
    }

    #[test]
    fn test_memmem_periodic() {
        let text = b"ababababababababababababababababababababababababababababababababababababababc";
        let finder = memmem::Finder::new(
            b"ababababababababababababababababababababababababababababababababababababababc",
        );
        assert_eq!(finder.find(text), Some(0));
    }

    #[test]
    fn test_memmem_repeated() {
        let text = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab";
        let finder =
            memmem::Finder::new(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab");
        assert_eq!(finder.find(text), Some(0));
    }

    #[test]
    fn test_memmem_sunday_short() {
        let finder = memmem::Finder::new(b"abc");
        assert_eq!(finder.find(b"ababcabc"), Some(2));
        assert_eq!(finder.find(b"ababab"), None);
    }
}
