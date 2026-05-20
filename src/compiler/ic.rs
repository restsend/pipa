use crate::object::shape::{Shape, ShapeId};
use std::ptr::NonNull;

#[derive(Clone, Copy)]
struct InlineCacheRead {
    shape_id: u32,
    offset: u32,
    proto_ptr: usize,
}

impl InlineCacheRead {
    #[inline(always)]
    const fn empty() -> Self {
        Self {
            shape_id: u32::MAX,
            offset: 0,
            proto_ptr: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct InlineCacheWrite {
    shape_id: u32,
    offset: u32,
    new_shape: usize,
}

impl InlineCacheWrite {
    #[inline(always)]
    const fn empty() -> Self {
        Self {
            shape_id: u32::MAX,
            offset: 0,
            new_shape: 0,
        }
    }
}

unsafe impl Send for InlineCacheWrite {}
unsafe impl Sync for InlineCacheWrite {}

const IC_POLY: usize = 2;

#[derive(Clone, Debug)]
pub struct InlineCache {
    
    reads: [InlineCacheRead; IC_POLY],
    
    write: InlineCacheWrite,
}

impl std::fmt::Debug for InlineCacheRead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ICRead(shape={}, off={}, proto={})",
            self.shape_id as usize, self.offset, self.proto_ptr
        )
    }
}

impl std::fmt::Debug for InlineCacheWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ICWrite(shape={}, off={}, new_shape={})",
            self.shape_id as usize, self.offset, self.new_shape
        )
    }
}

impl InlineCache {
    pub fn new() -> Self {
        InlineCache {
            reads: [InlineCacheRead::empty(); IC_POLY],
            write: InlineCacheWrite::empty(),
        }
    }

    #[inline(always)]
    pub fn get(&self, shape_id: ShapeId) -> Option<(u32, Option<usize>)> {
        let sid = shape_id.0 as u32;
        for r in &self.reads {
            if r.shape_id == sid {
                return Some((
                    r.offset,
                    if r.proto_ptr == 0 {
                        None
                    } else {
                        Some(r.proto_ptr)
                    },
                ));
            }
        }
        None
    }

    #[inline(always)]
    pub fn get_transition(&self, shape_id: ShapeId) -> Option<(u32, *const Shape)> {
        let w = &self.write;
        if w.shape_id == shape_id.0 as u32 {
            Some((w.offset, w.new_shape as *const Shape))
        } else {
            None
        }
    }

    pub fn insert(&mut self, shape_id: ShapeId, offset: u32, proto_ptr: Option<usize>) {
        
        self.reads[1] = self.reads[0];
        self.reads[0] = InlineCacheRead {
            shape_id: shape_id.0 as u32,
            offset,
            proto_ptr: proto_ptr.unwrap_or(0),
        };
    }

    pub fn insert_transition_null(&mut self, shape_id: ShapeId, offset: u32) {
        self.write = InlineCacheWrite {
            shape_id: shape_id.0 as u32,
            offset,
            new_shape: 0,
        };
    }

    pub fn insert_transition(
        &mut self,
        pre_shape_id: ShapeId,
        offset: u32,
        new_shape: NonNull<Shape>,
    ) {
        self.write = InlineCacheWrite {
            shape_id: pre_shape_id.0 as u32,
            offset,
            new_shape: new_shape.as_ptr() as usize,
        };
    }
}

impl Default for InlineCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct InlineCacheTable {
    caches: Vec<InlineCache>,
}

impl InlineCacheTable {
    pub fn new() -> Self {
        InlineCacheTable { caches: Vec::new() }
    }

    #[inline(always)]
    pub fn ensure_capacity(&mut self, len: usize) {
        if self.caches.len() < len {
            
            let new_len = (self.caches.len() * 2).max(len);
            self.caches.resize(new_len, InlineCache::new());
        }
    }

    pub fn preallocate(&mut self, len: usize) {
        if self.caches.len() < len {
            self.caches.resize(len, InlineCache::new());
        }
    }

    #[inline(always)]
    pub fn get(&self, pc: usize) -> Option<&InlineCache> {
        self.caches.get(pc)
    }

    #[inline(always)]
    pub fn get_mut(&mut self, pc: usize) -> Option<&mut InlineCache> {
        self.caches.get_mut(pc)
    }

    #[inline(always)]
    pub fn get_own_fast(&self, pc: usize, shape_id: usize) -> Option<u32> {
        if let Some(ic) = self.caches.get(pc) {
            let r = ic.reads[0];
            if r.shape_id == shape_id as u32 && r.proto_ptr == 0 {
                return Some(r.offset);
            }
        }
        None
    }

    #[inline(always)]
    pub fn get_reads0_fast(&self, pc: usize, shape_id: usize) -> Option<(u32, usize)> {
        if let Some(ic) = self.caches.get(pc) {
            let r = ic.reads[0];
            if r.shape_id == shape_id as u32 {
                return Some((r.offset, r.proto_ptr));
            }
        }
        None
    }

    #[inline(always)]
    pub fn get_reads0_values(&self, pc: usize, shape_id: usize) -> (bool, u32, usize) {
        if let Some(ic) = self.caches.get(pc) {
            let r = &ic.reads[0];
            if r.shape_id == shape_id as u32 {
                return (true, r.offset, r.proto_ptr);
            }
        }
        (false, 0, 0)
    }

    #[inline(always)]
    pub fn get_reads123_values(&self, pc: usize, shape_id: usize) -> (bool, u32, usize) {
        if let Some(ic) = self.caches.get(pc) {
            let sid = shape_id as u32;
            let r1 = ic.reads[1];
            if r1.shape_id == sid {
                return (true, r1.offset, r1.proto_ptr);
            }
        }
        (false, 0, 0)
    }

    #[inline(always)]
    pub fn set_own_fast(&self, pc: usize, shape_id: usize) -> Option<u32> {
        if let Some(ic) = self.caches.get(pc) {
            let w = &ic.write;
            if w.shape_id == shape_id as u32 && w.new_shape == 0 {
                return Some(w.offset);
            }
        }
        None
    }

    #[inline(always)]
    pub fn get_global_cache(&self, pc: usize, global_shape_id: usize, atom_id: u32) -> Option<u32> {
        if let Some(ic) = self.caches.get(pc) {
            let r = &ic.reads[0];
            
            if r.shape_id == global_shape_id as u32 && r.proto_ptr == atom_id as usize + 1 {
                return Some(r.offset);
            }
        }
        None
    }

    #[inline(always)]
    pub fn insert_global_cache(
        &mut self,
        pc: usize,
        global_shape_id: usize,
        offset: u32,
        atom_id: u32,
    ) {
        self.ensure_capacity(pc + 1);
        if let Some(ic) = self.caches.get_mut(pc) {
            ic.reads[0] = InlineCacheRead {
                shape_id: global_shape_id as u32,
                offset,
                proto_ptr: atom_id as usize + 1, 
            };
        }
    }
}

impl Default for InlineCacheTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::shape::ShapeId;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn test_shape_id() -> ShapeId {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
        ShapeId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    #[test]
    fn test_inline_cache_basic() {
        let mut ic = InlineCache::new();
        let shape_id = test_shape_id();

        assert!(ic.get(shape_id).is_none());

        ic.insert(shape_id, 42, None);
        assert_eq!(ic.get(shape_id), Some((42, None)));
    }

    #[test]
    fn test_inline_cache_multiple_shapes() {
        let mut ic = InlineCache::new();
        let shape1 = test_shape_id();
        let shape2 = test_shape_id();

        ic.insert(shape1, 10, None);
        assert_eq!(ic.get(shape1), Some((10, None)));

        ic.insert(shape2, 20, None);
        assert_eq!(ic.get(shape1), Some((10, None)));
        assert_eq!(ic.get(shape2), Some((20, None)));
    }

    #[test]
    fn test_inline_cache_eviction() {
        let mut ic = InlineCache::new();
        let mut shapes = Vec::new();

        for i in 0..IC_POLY {
            let s = test_shape_id();
            shapes.push(s);
            ic.insert(s, i as u32, None);
        }
        
        for (i, &s) in shapes.iter().enumerate() {
            assert_eq!(ic.get(s), Some((i as u32, None)));
        }

        let new_shape = test_shape_id();
        ic.insert(new_shape, 100, None);
        assert_eq!(ic.get(new_shape), Some((100, None)));
        
        assert_eq!(ic.get(shapes[0]), None);
        
        for i in 1..IC_POLY {
            assert_eq!(ic.get(shapes[i]), Some((i as u32, None)));
        }
    }

    #[test]
    fn test_inline_cache_prototype() {
        let mut ic = InlineCache::new();
        let shape_id = test_shape_id();
        let proto = 0x1234usize;

        ic.insert(shape_id, 7, Some(proto));
        assert_eq!(ic.get(shape_id), Some((7, Some(proto))));
    }
}
