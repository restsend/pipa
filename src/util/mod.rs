pub mod fxhash;
pub mod iomux;
pub mod memchr;

pub use fxhash::FxHashMap;
pub use fxhash::FxHashSet;

#[inline]
pub fn write_usize_to_buf(mut n: usize, buf: &mut [u8]) -> &str {
    let len = buf.len();
    let mut pos = len;
    if n == 0 {
        buf[len - 1] = b'0';
        return unsafe { std::str::from_utf8_unchecked(&buf[len - 1..len]) };
    }
    while n > 0 {
        pos -= 1;
        buf[pos] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    unsafe { std::str::from_utf8_unchecked(&buf[pos..len]) }
}
