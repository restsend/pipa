use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Atom(pub u32);

impl Atom {
    pub fn index(&self) -> u32 {
        self.0
    }

    pub fn empty() -> Self {
        Atom(0)
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

#[derive(Debug, Clone)]
struct AtomEntry {
    string: Arc<str>,
    ref_count: u32,
}

pub struct AtomTable {
    entries: FxHashMap<u32, AtomEntry>,
    index_map: FxHashMap<Arc<str>, u32>,
    next_index: u32,
    symbol_atoms: FxHashMap<u32, ()>,

    index_atoms: FxHashMap<u32, usize>,
}

impl AtomTable {
    pub fn new() -> Self {
        let mut table = AtomTable {
            entries: FxHashMap::with_capacity_and_hasher(4096, Default::default()),
            index_map: FxHashMap::with_capacity_and_hasher(4096, Default::default()),
            next_index: 0,
            symbol_atoms: FxHashMap::default(),
            index_atoms: FxHashMap::with_capacity_and_hasher(64, Default::default()),
        };
        table.intern("");
        table
    }

    pub fn intern(&mut self, s: &str) -> Atom {
        if let Some(&idx) = self.index_map.get(s) {
            if let Some(entry) = self.entries.get_mut(&idx) {
                entry.ref_count = entry.ref_count.saturating_add(1);
            }
            return Atom(idx);
        }

        let idx = self.next_index;
        self.next_index += 1;

        let arc: Arc<str> = Arc::from(s);
        self.index_map.insert(Arc::clone(&arc), idx);
        self.entries.insert(
            idx,
            AtomEntry {
                string: arc,
                ref_count: 1,
            },
        );

        if let Ok(array_idx) = s.parse::<usize>() {
            if array_idx < 1_000_000 {
                self.index_atoms.insert(idx, array_idx);
            }
        }

        Atom(idx)
    }

    #[inline]
    pub fn intern_fast(&mut self, s: &str) -> Atom {
        if let Some(&idx) = self.index_map.get(s) {
            return Atom(idx);
        }
        self.intern(s)
    }

    pub fn lookup(&self, s: &str) -> Option<Atom> {
        self.index_map.get(s).map(|&idx| Atom(idx))
    }

    pub fn intern_concat(&mut self, a: &str, b: &str) -> Atom {
        let total_len = a.len() + b.len();

        if total_len <= 64 {
            let mut buf = [0u8; 64];
            buf[..a.len()].copy_from_slice(a.as_bytes());
            buf[a.len()..total_len].copy_from_slice(b.as_bytes());
            let s = unsafe { std::str::from_utf8_unchecked(&buf[..total_len]) };
            return self.intern(s);
        }

        let mut combined = String::with_capacity(total_len);
        combined.push_str(a);
        combined.push_str(b);
        self.intern(&combined)
    }

    pub fn intern_concat_atoms(&mut self, a: Atom, b: Atom) -> Atom {
        let a_str = self.get(a);
        let b_str = self.get(b);
        let a_len = a_str.len();
        let b_len = b_str.len();
        let total = a_len + b_len;

        if total <= 128 {
            let mut buf = [0u8; 128];
            buf[..a_len].copy_from_slice(a_str.as_bytes());
            buf[a_len..total].copy_from_slice(b_str.as_bytes());

            let s = unsafe { std::str::from_utf8_unchecked(&buf[..total]) };
            self.intern(s)
        } else {
            let a_owned = a_str.to_string();
            let b_owned = b_str.to_string();
            let mut combined = String::with_capacity(total);
            combined.push_str(&a_owned);
            combined.push_str(&b_owned);
            self.intern(&combined)
        }
    }

    pub fn get(&self, atom: Atom) -> &str {
        if atom.0 == 0 {
            return "";
        }
        self.entries
            .get(&atom.0)
            .map(|e| e.string.as_ref())
            .unwrap_or("")
    }

    #[inline]
    pub fn char_count(&self, atom: Atom) -> usize {
        let s = self.get(atom);
        if s.is_ascii() {
            s.len()
        } else {
            s.chars().count()
        }
    }

    #[inline]
    pub fn char_code_at(&self, atom: Atom, index: usize) -> Option<u32> {
        let s = self.get(atom);
        if s.is_ascii() {
            s.as_bytes().get(index).map(|&b| b as u32)
        } else {
            s.chars().nth(index).map(|c| c as u32)
        }
    }

    pub fn retain(&mut self, atom: Atom) {
        if let Some(entry) = self.entries.get_mut(&atom.0) {
            entry.ref_count = entry.ref_count.saturating_add(1);
        }
    }

    pub fn release(&mut self, atom: Atom) {
        if let Some(entry) = self.entries.get_mut(&atom.0) {
            entry.ref_count = entry.ref_count.saturating_sub(1);
            if entry.ref_count == 0 {
                let arc = Arc::clone(&entry.string);
                self.index_map.remove(arc.as_ref());
                self.entries.remove(&atom.0);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn mark_symbol_atom(&mut self, idx: u32) {
        self.symbol_atoms.insert(idx, ());
    }

    pub fn is_symbol_atom(&self, idx: u32) -> bool {
        self.symbol_atoms.contains_key(&idx)
    }

    #[inline]
    pub fn get_array_index(&self, atom: Atom) -> Option<usize> {
        self.index_atoms.get(&atom.0).copied()
    }
}

impl Default for AtomTable {
    fn default() -> Self {
        Self::new()
    }
}

use crate::util::FxHashMap;
