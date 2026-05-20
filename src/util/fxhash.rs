use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default)]
pub struct FxHasher {
    hash: u64,
}

impl Hasher for FxHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut hash = self.hash;
        let mut i = 0;

        while i + 8 <= bytes.len() {
            let chunk = u64::from_ne_bytes(bytes[i..i + 8].try_into().unwrap());
            hash = hash.rotate_left(5) ^ chunk.wrapping_mul(0x517cc1b727220a95);
            i += 8;
        }

        if i < bytes.len() {
            let mut last = 0u64;
            let mut shift = 0;
            for &b in &bytes[i..] {
                last |= (b as u64) << shift;
                shift += 8;
            }
            hash = hash.rotate_left(5) ^ last.wrapping_mul(0x517cc1b727220a95);
        }
        self.hash = hash;
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.write(&[i])
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_ne_bytes())
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_ne_bytes())
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.hash = self.hash.rotate_left(5) ^ i.wrapping_mul(0x517cc1b727220a95);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }

    #[inline]
    fn write_i8(&mut self, i: i8) {
        self.write_u8(i as u8)
    }

    #[inline]
    fn write_i16(&mut self, i: i16) {
        self.write_u16(i as u16)
    }

    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.write_u32(i as u32)
    }

    #[inline]
    fn write_i64(&mut self, i: i64) {
        self.write_u64(i as u64)
    }

    #[inline]
    fn write_isize(&mut self, i: isize) {
        self.write_usize(i as usize)
    }
}

pub type FxBuildHasher = BuildHasherDefault<FxHasher>;

pub type FxHashMap<K, V> = HashMap<K, V, FxBuildHasher>;

pub type FxHashSet<K> = std::collections::HashSet<K, FxBuildHasher>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_insert_lookup() {
        let mut map: FxHashMap<u32, &str> = FxHashMap::default();
        map.insert(1, "one");
        map.insert(2, "two");
        map.insert(42, "answer");
        assert_eq!(map.get(&1), Some(&"one"));
        assert_eq!(map.get(&2), Some(&"two"));
        assert_eq!(map.get(&42), Some(&"answer"));
        assert_eq!(map.get(&99), None);
    }

    #[test]
    fn test_string_keys() {
        let mut map: FxHashMap<String, u32> = FxHashMap::default();
        map.insert("hello".to_string(), 1);
        map.insert("world".to_string(), 2);
        assert_eq!(map.get("hello"), Some(&1));
        assert_eq!(map.get("world"), Some(&2));
    }

    #[test]
    fn test_tuple_keys() {
        let mut map: FxHashMap<(u32, u32), &str> = FxHashMap::default();
        map.insert((1, 2), "a");
        map.insert((3, 4), "b");
        assert_eq!(map.get(&(1, 2)), Some(&"a"));
        assert_eq!(map.get(&(3, 4)), Some(&"b"));
    }

    #[test]
    fn test_overwrite() {
        let mut map: FxHashMap<u32, u32> = FxHashMap::default();
        map.insert(1, 10);
        map.insert(1, 20);
        assert_eq!(map.get(&1), Some(&20));
    }

    #[test]
    fn test_remove() {
        let mut map: FxHashMap<u32, u32> = FxHashMap::default();
        map.insert(1, 10);
        map.insert(2, 20);
        map.remove(&1);
        assert_eq!(map.get(&1), None);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_large_map() {
        let mut map: FxHashMap<u64, u64> = FxHashMap::default();
        for i in 0..10000u64 {
            map.insert(i, i * 2);
        }
        for i in 0..10000u64 {
            assert_eq!(map.get(&i), Some(&(i * 2)));
        }
    }
}
