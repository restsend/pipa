use crate::object::array_obj::JSArrayObject;
use crate::object::function::JSFunction;
use crate::object::object::JSObject;
use crate::object::object::SmallPropVec;
use crate::util::FxHashSet;
use crate::value::JSValue;
use std::env;

pub const TAG_OBJECT: u8 = 0;
pub const TAG_ARRAY: u8 = 1;
pub const TAG_FUNCTION: u8 = 2;

const COLOR_WHITE: u8 = 0;
const COLOR_GRAY: u8 = 1;
const COLOR_BLACK: u8 = 2;
const MIN_HEAP_THRESHOLD: usize = 1024 * 1024;
const NURSERY_PINNED_STREAK_LIMIT: usize = 10;

fn nursery_pinned_streak_limit() -> usize {
    env::var("PIPA_NURSERY_PINNED_STREAK_LIMIT")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .map(|value| value.max(1))
        .unwrap_or(NURSERY_PINNED_STREAK_LIMIT)
}

#[derive(Clone, Copy, Debug, Default)]
struct GcDebugConfig {
    enabled: bool,
    dump_roots: bool,
    dump_live: bool,
    dump_dead: bool,
    dump_alloc: bool,
    limit: usize,
}

impl GcDebugConfig {
    fn from_env() -> Self {
        let raw = match env::var("PIPA_GCDUMP") {
            Ok(value) => value,
            Err(_) => return Self::default(),
        };

        let normalized = raw.trim().to_ascii_lowercase();
        if normalized.is_empty()
            || normalized == "0"
            || normalized == "false"
            || normalized == "off"
        {
            return Self::default();
        }

        let limit = env::var("PIPA_GCDUMP_LIMIT")
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            .map(|value| value.max(1))
            .unwrap_or(16);

        let mut config = Self {
            enabled: true,
            limit,
            ..Self::default()
        };

        if normalized == "1" || normalized == "true" || normalized == "summary" {
            return config;
        }

        for token in normalized
            .split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
        {
            match token {
                "1" | "true" | "summary" => {}
                "all" => {
                    config.dump_roots = true;
                    config.dump_live = true;
                    config.dump_dead = true;
                    config.dump_alloc = true;
                }
                "roots" => config.dump_roots = true,
                "live" => config.dump_live = true,
                "dead" => config.dump_dead = true,
                "alloc" => config.dump_alloc = true,
                _ => {}
            }
        }

        config
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct GcDebugStats {
    collections: usize,
    full_collections: usize,
    alloc_events: usize,
    minor_freed: usize,
    full_freed: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct GcSnapshot {
    objects: usize,
    nursery: usize,
    bytes: usize,
    threshold: usize,
}

#[derive(Clone, Copy)]
struct GcDebugEntry {
    handle: usize,
    ptr: usize,
    tag: u8,
    in_nursery: bool,
}

pub struct Nursery {
    base: *mut u8,
    capacity: usize,
    offset: usize,
}

impl Nursery {
    pub fn new(capacity: usize) -> Self {
        let layout = std::alloc::Layout::from_size_align(capacity, 8).unwrap();
        let base = unsafe { std::alloc::alloc(layout) };
        if base.is_null() {
            panic!("failed to allocate nursery memory");
        }
        Self {
            base,
            capacity,
            offset: 0,
        }
    }

    #[inline]
    pub fn alloc<T>(&mut self, value: T) -> Option<*mut T> {
        let layout = std::alloc::Layout::new::<T>();
        let align = layout.align();
        let size = layout.size();
        let aligned = (self.offset + align - 1) & !(align - 1);
        if aligned + size > self.capacity {
            return None;
        }
        let ptr = unsafe { self.base.add(aligned) as *mut T };
        unsafe {
            std::ptr::write(ptr, value);
        }
        self.offset = aligned + size;
        Some(ptr)
    }

    #[inline]
    pub fn reset(&mut self) {
        self.offset = 0;
    }

    #[inline]
    pub fn can_fit<T>(&self) -> bool {
        let layout = std::alloc::Layout::new::<T>();
        let aligned = (self.offset + layout.align() - 1) & !(layout.align() - 1);
        aligned + layout.size() <= self.capacity
    }

    #[inline]
    pub fn usage(&self) -> usize {
        self.offset
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn contains<T>(&self, ptr: *const T) -> bool {
        let start = self.base as usize;
        let end = start + self.capacity;
        let p = ptr as usize;
        p >= start && p < end
    }
}

impl Drop for Nursery {
    fn drop(&mut self) {
        if !self.base.is_null() {
            let layout = std::alloc::Layout::from_size_align(self.capacity, 8).unwrap();
            unsafe {
                std::alloc::dealloc(self.base, layout);
            }
        }
    }
}

pub struct GcHeap {
    objects: Vec<usize>,
    marks: Vec<u8>,
    tags: Vec<u8>,
    free_list: Vec<usize>,
    gray_stack: Vec<usize>,
    total_allocated: usize,
    threshold: usize,

    live_count: usize,

    count_threshold: usize,
    pub nursery: Nursery,
    nursery_indices: Vec<usize>,
    nursery_enabled: bool,
    nursery_pinned_streak: usize,
    last_minor_survivor_count: usize,
    pub minor_collections: usize,

    pub deleted_props_count: usize,

    traced_functions: FxHashSet<usize>,
    nursery_pinned_streak_limit: usize,
    debug: GcDebugConfig,
    debug_stats: GcDebugStats,

    pub prop_pool: Vec<SmallPropVec>,

    pub extra_roots: Vec<JSValue>,

    pub gc_suppressed: bool,
}

impl GcHeap {
    pub fn new() -> Self {
        GcHeap {
            objects: Vec::new(),
            marks: Vec::new(),
            tags: Vec::new(),
            free_list: Vec::new(),
            gray_stack: Vec::new(),
            total_allocated: 0,
            threshold: MIN_HEAP_THRESHOLD,
            live_count: 0,
            count_threshold: 2_000_000,
            nursery: Nursery::new(16 * 1024 * 1024),
            nursery_indices: Vec::new(),
            nursery_enabled: true,
            nursery_pinned_streak: 0,
            nursery_pinned_streak_limit: nursery_pinned_streak_limit(),
            last_minor_survivor_count: 0,
            minor_collections: 0,
            deleted_props_count: 0,
            traced_functions: FxHashSet::default(),
            debug: GcDebugConfig::from_env(),
            debug_stats: GcDebugStats::default(),
            prop_pool: Vec::new(),
            extra_roots: Vec::new(),
            gc_suppressed: false,
        }
    }

    fn track_tenured(&mut self, ptr: usize, tag: u8) -> usize {
        let idx = if let Some(idx) = self.free_list.pop() {
            self.objects[idx] = ptr;
            self.marks[idx] = 0;
            self.tags[idx] = tag;
            idx
        } else {
            let idx = self.objects.len();
            self.objects.push(ptr);
            self.marks.push(0);
            self.tags.push(tag);
            idx
        };

        unsafe {
            (*(ptr as *mut JSObject)).gc_slot = idx as u32;
        }
        let size = if tag == TAG_ARRAY {
            std::mem::size_of::<JSArrayObject>()
        } else if tag == TAG_FUNCTION {
            std::mem::size_of::<JSFunction>()
        } else {
            std::mem::size_of::<JSObject>()
        };
        self.total_allocated += size;
        self.live_count += 1;
        self.debug_alloc(idx, ptr, tag, false);
        idx
    }

    fn track_nursery(&mut self, ptr: usize, tag: u8) -> usize {
        let idx = if let Some(idx) = self.free_list.pop() {
            self.objects[idx] = ptr;
            self.marks[idx] = 0;
            self.tags[idx] = tag;
            idx
        } else {
            let idx = self.objects.len();
            self.objects.push(ptr);
            self.marks.push(0);
            self.tags.push(tag);
            idx
        };

        unsafe {
            (*(ptr as *mut JSObject)).gc_slot = idx as u32;
        }
        self.nursery_indices.push(idx);
        self.debug_alloc(idx, ptr, tag, true);
        idx
    }

    pub fn track(&mut self, ptr: usize) -> usize {
        self.track_tenured(ptr, TAG_OBJECT)
    }

    pub fn track_array(&mut self, ptr: usize) -> usize {
        self.track_tenured(ptr, TAG_ARRAY)
    }

    pub fn track_function(&mut self, ptr: usize) -> usize {
        self.track_tenured(ptr, TAG_FUNCTION)
    }

    #[inline]
    pub fn take_prop_vec(&mut self) -> SmallPropVec {
        self.prop_pool.pop().unwrap_or_else(|| SmallPropVec::new())
    }

    pub fn alloc_object(&mut self) -> Option<*mut JSObject> {
        if !self.nursery_enabled {
            return None;
        }
        let props = self.take_prop_vec();
        self.nursery
            .alloc(JSObject::new_typed_from_pool(
                crate::object::object::ObjectType::Ordinary,
                props,
            ))
            .map(|ptr| {
                self.track_nursery(ptr as usize, TAG_OBJECT);
                ptr
            })
    }

    pub fn alloc_object_with_value(&mut self, obj: JSObject) -> Option<*mut JSObject> {
        if !self.nursery_enabled || !self.nursery.can_fit::<JSObject>() {
            return None;
        }
        self.nursery.alloc(obj).map(|ptr| {
            self.track_nursery(ptr as usize, TAG_OBJECT);
            ptr
        })
    }

    #[inline]
    pub fn nursery_enabled_and_can_fit_object(&self) -> bool {
        self.nursery_enabled && self.nursery.can_fit::<JSObject>()
    }

    pub fn alloc_array(&mut self) -> Option<*mut JSArrayObject> {
        if !self.nursery_enabled {
            return None;
        }
        self.nursery.alloc(JSArrayObject::new()).map(|ptr| {
            self.track_nursery(ptr as usize, TAG_ARRAY);
            ptr
        })
    }

    pub fn alloc_array_with_value(&mut self, arr: JSArrayObject) -> Option<*mut JSArrayObject> {
        if !self.nursery_enabled || !self.nursery.can_fit::<JSArrayObject>() {
            return None;
        }
        self.nursery.alloc(arr).map(|ptr| {
            self.track_nursery(ptr as usize, TAG_ARRAY);
            ptr
        })
    }

    pub fn nursery_is_full(&self) -> bool {
        !self.gc_suppressed && self.nursery_enabled && !self.nursery.can_fit::<JSObject>()
    }

    pub fn untrack(&mut self, ptr: usize) {
        let idx = unsafe { (*(ptr as *const JSObject)).gc_slot };
        if idx != u32::MAX {
            let idx = idx as usize;
            if self.objects.get(idx) == Some(&ptr) {
                self.objects[idx] = 0;
                self.marks[idx] = 0;
                self.tags[idx] = TAG_OBJECT;
                self.free_list.push(idx);
            }
        }
        self.total_allocated = self
            .total_allocated
            .saturating_sub(std::mem::size_of::<JSObject>());
        self.live_count = self.live_count.saturating_sub(1);
    }

    pub fn mark_value(&mut self, value: &JSValue) {
        if value.is_object() {
            let ptr = value.get_ptr();
            self.mark(ptr);
        } else if value.is_string() {
        } else if value.is_function() {
            let ptr = value.get_ptr();
            if self.mark(ptr) {
                let func = value.as_function();
                self.trace_function(func);
            }
        }
    }

    fn trace_function(&mut self, func: &JSFunction) {
        let ptr = func as *const JSFunction as usize;
        if !self.traced_functions.insert(ptr) {
            return;
        }

        func.base.for_each_property(|_atom, value, _attrs| {
            self.mark_value(&value);
        });

        func.base.for_each_accessor(|_atom, getter, setter| {
            if let Some(ref g) = getter {
                self.mark_value(g);
            }
            if let Some(ref s) = setter {
                self.mark_value(s);
            }
        });

        if let Some(items_to_mark) = func.base.get_private_accessors_for_gc() {
            for item in items_to_mark {
                self.mark_value(&item);
            }
        }

        func.base.for_each_private_field(|_atom, value| {
            self.mark_value(&value);
        });

        if let Some(proto_ptr) = func.base.prototype {
            self.mark(proto_ptr as usize);
        }

        if let Some(uv) = func.upvalues_ref() {
            for (_, upvalue) in &uv.upvalues {
                self.mark_value(upvalue);
            }
            for (_, upvalue_cell) in &uv.upvalue_cells {
                self.mark_value(&upvalue_cell.get());
            }
        }
    }

    #[inline(always)]
    pub fn is_array_ptr(&self, ptr: usize) -> bool {
        unsafe { (*(ptr as *const JSObject)).is_dense_array() }
    }

    #[inline]
    pub fn mark(&mut self, ptr: usize) -> bool {
        let idx = unsafe { (*(ptr as *const JSObject)).gc_slot };
        if idx == u32::MAX {
            return false;
        }
        let idx = idx as usize;
        if self.objects.get(idx) != Some(&ptr) {
            return false;
        }
        if self.marks[idx] == COLOR_WHITE {
            self.marks[idx] = COLOR_GRAY;
            self.gray_stack.push(idx);
        }
        true
    }

    fn trace_object(&mut self, idx: usize, ptr: usize) {
        if ptr == 0 {
            return;
        }
        self.marks[idx] = COLOR_BLACK;
        let tag = self.tags[idx];

        if tag == TAG_ARRAY {
            let arr = unsafe { &*(ptr as *const JSArrayObject) };
            arr.for_each_element(|value| {
                self.mark_value(value);
            });
            arr.header.for_each_property(|_atom, value, _attrs| {
                self.mark_value(&value);
            });
            arr.header.for_each_accessor(|_atom, getter, setter| {
                if let Some(ref g) = getter {
                    self.mark_value(g);
                }
                if let Some(ref s) = setter {
                    self.mark_value(s);
                }
            });

            if let Some(items_to_mark) = arr.header.get_private_accessors_for_gc() {
                for item in items_to_mark {
                    self.mark_value(&item);
                }
            }
            arr.header.for_each_private_field(|_atom, value| {
                self.mark_value(&value);
            });
            if let Some(proto_ptr) = arr.header.prototype {
                self.mark(proto_ptr as usize);
            }
        } else if tag == TAG_FUNCTION {
            let func = unsafe { &*(ptr as *const JSFunction) };
            self.trace_function(func);
        } else {
            let obj = unsafe { JSValue::object_from_ptr(ptr) };
            obj.for_each_property(|_atom, value, _attrs| {
                self.mark_value(&value);
            });
            obj.for_each_accessor(|_atom, getter, setter| {
                if let Some(ref g) = getter {
                    self.mark_value(g);
                }
                if let Some(ref s) = setter {
                    self.mark_value(s);
                }
            });

            if let Some(items_to_mark) = obj.get_private_accessors_for_gc() {
                for item in items_to_mark {
                    self.mark_value(&item);
                }
            }
            obj.for_each_private_field(|_atom, value| {
                self.mark_value(&value);
            });
            obj.for_each_array_element(|value| {
                self.mark_value(value);
            });
            if let Some(proto_ptr) = obj.prototype {
                self.mark(proto_ptr as usize);
            }

            if let Some(state) = obj.get_generator_state() {
                for v in &state.snapshot {
                    self.mark_value(v);
                }
            }
        }
    }

    pub fn mark_roots(&mut self, roots: &[JSValue]) {
        for value in roots {
            self.mark_value(value);
        }
    }

    #[inline(always)]
    unsafe fn drop_tracked(ptr: usize, tag: u8) {
        if tag == TAG_ARRAY {
            unsafe {
                let _ = Box::from_raw(ptr as *mut JSArrayObject);
            }
        } else if tag == TAG_FUNCTION {
            unsafe {
                let _ = Box::from_raw(ptr as *mut JSFunction);
            }
        } else {
            unsafe {
                let _ = Box::from_raw(ptr as *mut JSObject);
            }
        }
    }

    #[inline(always)]
    unsafe fn drop_in_place_tracked(ptr: usize, tag: u8) {
        if tag == TAG_ARRAY {
            unsafe {
                std::ptr::drop_in_place(ptr as *mut JSArrayObject);
            }
        } else if tag == TAG_FUNCTION {
            unsafe {
                std::ptr::drop_in_place(ptr as *mut JSFunction);
            }
        } else {
            unsafe {
                std::ptr::drop_in_place(ptr as *mut JSObject);
            }
        }
    }

    pub(crate) fn snapshot(&mut self) -> GcSnapshot {
        GcSnapshot {
            objects: self.object_count(),
            nursery: self.nursery_indices.len(),
            bytes: self.total_allocated,
            threshold: self.threshold,
        }
    }

    fn tag_name(tag: u8) -> &'static str {
        match tag {
            TAG_ARRAY => "array",
            TAG_OBJECT => "object",
            _ => "unknown",
        }
    }

    fn debug_alloc(&mut self, handle: usize, ptr: usize, tag: u8, in_nursery: bool) {
        self.debug_stats.alloc_events += 1;
        if self.debug.enabled && self.debug.dump_alloc {
            eprintln!(
                "[gcdump] alloc#{} handle={} ptr={:#x} tag={} nursery={}",
                self.debug_stats.alloc_events,
                handle,
                ptr,
                Self::tag_name(tag),
                in_nursery
            );
        }
    }

    fn describe_value(&self, value: &JSValue) -> String {
        if value.is_undefined() {
            "undefined".to_string()
        } else if value.is_null() {
            "null".to_string()
        } else if value.is_bool() {
            format!("bool({})", value.get_bool())
        } else if value.is_int() {
            format!("int({})", value.get_int())
        } else if value.is_float() {
            format!("float({})", value.get_float())
        } else if value.is_string() {
            format!("string(atom={})", value.get_atom().0)
        } else if value.is_symbol() {
            format!("symbol(atom={})", value.get_atom().0)
        } else if value.is_bigint() {
            format!("bigint(ptr={:#x})", value.get_ptr())
        } else if value.is_function() {
            let ptr = value.get_ptr();
            let handle = unsafe { (*(ptr as *const JSObject)).gc_slot };
            if handle != u32::MAX {
                format!("function(handle={} ptr={:#x})", handle, ptr)
            } else {
                format!("function(ptr={:#x})", ptr)
            }
        } else if value.is_object() {
            let ptr = value.get_ptr();
            let handle = unsafe { (*(ptr as *const JSObject)).gc_slot };
            if handle != u32::MAX {
                format!("object(handle={} ptr={:#x})", handle, ptr)
            } else {
                format!("object(ptr={:#x})", ptr)
            }
        } else if value.is_tdz() {
            "tdz".to_string()
        } else {
            format!("raw({:#x})", value.raw_bits())
        }
    }

    fn collect_entries_from_indices(&self, indices: &[usize]) -> Vec<GcDebugEntry> {
        indices
            .iter()
            .take(self.debug.limit)
            .filter_map(|&idx| {
                let ptr = self.objects.get(idx).copied().unwrap_or(0);
                if ptr == 0 {
                    return None;
                }
                Some(GcDebugEntry {
                    handle: idx,
                    ptr,
                    tag: self.tags[idx],
                    in_nursery: self.nursery.contains(ptr as *const u8),
                })
            })
            .collect()
    }

    fn collect_live_entries(&self) -> Vec<GcDebugEntry> {
        let mut entries = Vec::new();
        for (idx, &ptr) in self.objects.iter().enumerate() {
            if ptr == 0 {
                continue;
            }
            entries.push(GcDebugEntry {
                handle: idx,
                ptr,
                tag: self.tags[idx],
                in_nursery: self.nursery.contains(ptr as *const u8),
            });
            if entries.len() >= self.debug.limit {
                break;
            }
        }
        entries
    }

    fn debug_collection_start(&mut self, kind: &str, roots: &[JSValue]) -> usize {
        self.debug_stats.collections += 1;
        if kind == "full" {
            self.debug_stats.full_collections += 1;
        }
        let seq = self.debug_stats.collections;
        if !self.debug.enabled {
            return seq;
        }

        let snapshot = self.snapshot();
        eprintln!(
            "[gcdump] gc#{} {} start roots={} objects={} nursery={} bytes={} threshold={} nursery_enabled={} minor_count={} full_count={}",
            seq,
            kind,
            roots.len(),
            snapshot.objects,
            snapshot.nursery,
            snapshot.bytes,
            snapshot.threshold,
            self.nursery_enabled,
            self.minor_collections,
            self.debug_stats.full_collections
        );

        if self.debug.dump_roots {
            for (index, value) in roots.iter().take(self.debug.limit).enumerate() {
                eprintln!(
                    "[gcdump] gc#{} {} root[{}]={}",
                    seq,
                    kind,
                    index,
                    self.describe_value(value)
                );
            }
            if roots.len() > self.debug.limit {
                eprintln!(
                    "[gcdump] gc#{} {} roots omitted={}",
                    seq,
                    kind,
                    roots.len() - self.debug.limit
                );
            }
        }

        seq
    }

    fn debug_dump_entries(&self, seq: usize, kind: &str, label: &str, entries: &[GcDebugEntry]) {
        if !self.debug.enabled || entries.is_empty() {
            return;
        }
        for entry in entries {
            eprintln!(
                "[gcdump] gc#{} {} {} handle={} ptr={:#x} tag={} nursery={}",
                seq,
                kind,
                label,
                entry.handle,
                entry.ptr,
                Self::tag_name(entry.tag),
                entry.in_nursery
            );
        }
    }

    fn debug_collection_end(
        &mut self,
        seq: usize,
        kind: &str,
        before: GcSnapshot,
        freed: usize,
        dead_entries: &[GcDebugEntry],
    ) {
        if kind == "minor" {
            self.debug_stats.minor_freed += freed;
        } else {
            self.debug_stats.full_freed += freed;
        }

        if !self.debug.enabled {
            return;
        }

        if self.debug.dump_dead {
            self.debug_dump_entries(seq, kind, "dead", dead_entries);
            if freed > dead_entries.len() {
                eprintln!(
                    "[gcdump] gc#{} {} dead omitted={}",
                    seq,
                    kind,
                    freed - dead_entries.len()
                );
            }
        }

        if self.debug.dump_live {
            let live_entries = self.collect_live_entries();
            self.debug_dump_entries(seq, kind, "live", &live_entries);
            let live_total = self.object_count();
            if live_total > live_entries.len() {
                eprintln!(
                    "[gcdump] gc#{} {} live omitted={}",
                    seq,
                    kind,
                    live_total - live_entries.len()
                );
            }
        }

        let after = self.snapshot();
        eprintln!(
            "[gcdump] gc#{} {} end freed={} objects={}=>{} nursery={}=>{} bytes={}=>{} threshold={} nursery_enabled={} minor_freed_total={} full_freed_total={}",
            seq,
            kind,
            freed,
            before.objects,
            after.objects,
            before.nursery,
            after.nursery,
            before.bytes,
            after.bytes,
            after.threshold,
            self.nursery_enabled,
            self.debug_stats.minor_freed,
            self.debug_stats.full_freed
        );
    }

    fn refresh_threshold_after_full_gc(&mut self) {
        self.threshold = (self.total_allocated.saturating_mul(8)).max(MIN_HEAP_THRESHOLD);

        self.count_threshold = (self.live_count.saturating_mul(8)).max(2_000_000);
    }

    fn note_minor_gc_result(&mut self, freed: usize) {
        let survivor_count = self.nursery_indices.len();
        if survivor_count == 0 {
            self.nursery_enabled = true;
            self.nursery_pinned_streak = 0;
            self.last_minor_survivor_count = 0;
            return;
        }

        if freed == 0 && survivor_count == self.last_minor_survivor_count {
            self.nursery_pinned_streak += 1;
            if self.nursery_pinned_streak >= self.nursery_pinned_streak_limit {
                self.nursery_enabled = false;
            }
        } else {
            self.nursery_pinned_streak = 0;
        }

        self.last_minor_survivor_count = survivor_count;
    }

    pub fn minor_gc(&mut self, roots: &[JSValue]) -> usize {
        let before = self.snapshot();
        let seq = self.debug_collection_start("minor", roots);

        for m in self.marks.iter_mut() {
            *m = 0;
        }
        self.gray_stack.clear();
        self.traced_functions.clear();

        self.mark_roots(roots);

        while let Some(idx) = self.gray_stack.pop() {
            let ptr = self.objects[idx];
            self.trace_object(idx, ptr);
        }

        let mut dead = Vec::new();
        let mut new_nursery_indices = Vec::new();
        for &idx in &self.nursery_indices {
            if self.marks[idx] == COLOR_WHITE {
                dead.push(idx);
            } else {
                new_nursery_indices.push(idx);
            }
        }

        let dead_entries = if self.debug.enabled && self.debug.dump_dead {
            self.collect_entries_from_indices(&dead)
        } else {
            Vec::new()
        };

        for &idx in &dead {
            let ptr = self.objects[idx];
            let tag = self.tags[idx];
            unsafe {
                if tag == TAG_OBJECT {
                    let obj = &mut *(ptr as *mut JSObject);

                    let mut props = obj.take_props();
                    props.clear();

                    std::ptr::drop_in_place(ptr as *mut JSObject);

                    if props.capacity() <= 8 {
                        self.prop_pool.push(props);
                    }
                } else {
                    Self::drop_in_place_tracked(ptr, tag);
                }
            }
            self.objects[idx] = 0;
            self.marks[idx] = 0;
            self.tags[idx] = TAG_OBJECT;
            self.free_list.push(idx);
        }
        self.gray_stack.clear();
        self.clean_stale_properties_on_nursery_survivors();

        self.nursery_indices = new_nursery_indices;
        if self.nursery_indices.is_empty() {
            self.nursery.reset();
            self.nursery_enabled = true;
        }
        self.gray_stack.clear();
        self.minor_collections += 1;
        let freed = dead.len();
        self.note_minor_gc_result(freed);
        self.debug_collection_end(seq, "minor", before, freed, &dead_entries);
        freed
    }

    pub fn run_gc(&mut self, roots: &[JSValue]) -> usize {
        let before = self.snapshot();
        let seq = self.debug_collection_start("full", roots);

        for m in self.marks.iter_mut() {
            *m = 0;
        }
        self.gray_stack.clear();
        self.traced_functions.clear();

        self.mark_roots(roots);

        while let Some(idx) = self.gray_stack.pop() {
            let ptr = self.objects[idx];
            self.trace_object(idx, ptr);
        }

        let mut to_free: Vec<usize> = Vec::new();
        for (idx, &ptr) in self.objects.iter().enumerate() {
            if ptr != 0 && self.marks[idx] == COLOR_WHITE {
                to_free.push(idx);
            }
        }

        let dead_entries = if self.debug.enabled && self.debug.dump_dead {
            self.collect_entries_from_indices(&to_free)
        } else {
            Vec::new()
        };

        let freed = to_free.len();
        let mut live_nursery_indices = Vec::new();
        for idx in to_free {
            let ptr = self.objects[idx];
            let tag = self.tags[idx];
            let in_nursery = self.nursery.contains(ptr as *const u8);
            let size = if tag == TAG_ARRAY {
                std::mem::size_of::<JSArrayObject>()
            } else if tag == TAG_FUNCTION {
                std::mem::size_of::<JSFunction>()
            } else {
                std::mem::size_of::<JSObject>()
            };
            unsafe {
                if tag == TAG_OBJECT {
                    let obj = &mut *(ptr as *mut JSObject);
                    let mut props = obj.take_props();
                    props.clear();
                    if in_nursery {
                        std::ptr::drop_in_place(ptr as *mut JSObject);
                    } else {
                        let _ = Box::from_raw(ptr as *mut JSObject);
                    }
                    if props.capacity() <= 8 {
                        self.prop_pool.push(props);
                    }
                } else if in_nursery {
                    Self::drop_in_place_tracked(ptr, tag);
                } else {
                    Self::drop_tracked(ptr, tag);
                }
            }
            self.objects[idx] = 0;
            self.marks[idx] = 0;
            self.tags[idx] = TAG_OBJECT;
            self.free_list.push(idx);
            self.total_allocated = self.total_allocated.saturating_sub(size);
            self.live_count = self.live_count.saturating_sub(1);
        }

        for &idx in &self.nursery_indices {
            if self.objects[idx] != 0 {
                live_nursery_indices.push(idx);
            }
        }

        self.nursery_indices = live_nursery_indices;
        if self.nursery_indices.is_empty() {
            self.nursery.reset();
            self.nursery_enabled = true;
        } else {
            self.nursery_enabled = false;
        }
        self.nursery_pinned_streak = 0;
        self.last_minor_survivor_count = self.nursery_indices.len();
        self.nursery_pinned_streak = 0;
        self.last_minor_survivor_count = self.nursery_indices.len();
        self.gray_stack.clear();
        self.refresh_threshold_after_full_gc();
        self.debug_collection_end(seq, "full", before, freed, &dead_entries);
        freed
    }

    pub fn start_incremental_mark(&mut self, roots: &[JSValue]) {
        for m in self.marks.iter_mut() {
            *m = COLOR_WHITE;
        }
        self.gray_stack.clear();
        self.mark_roots(roots);
    }

    pub fn incremental_mark_step(&mut self, budget: usize) -> bool {
        for _ in 0..budget {
            if let Some(idx) = self.gray_stack.pop() {
                if self.marks[idx] == COLOR_GRAY {
                    let ptr = self.objects[idx];
                    self.trace_object(idx, ptr);
                }
            } else {
                return false;
            }
        }
        !self.gray_stack.is_empty()
    }

    pub fn write_barrier(&mut self, obj_ptr: usize, new_value: &JSValue) {
        let idx = unsafe { (*(obj_ptr as *const JSObject)).gc_slot };
        if idx != u32::MAX {
            let idx = idx as usize;
            if self.marks[idx] == COLOR_BLACK {
                self.mark_value(new_value);
            }
        }
    }

    pub fn finish_incremental_sweep(&mut self) -> usize {
        while let Some(idx) = self.gray_stack.pop() {
            if self.marks[idx] == COLOR_GRAY {
                let ptr = self.objects[idx];
                self.trace_object(idx, ptr);
            }
        }

        let mut to_free: Vec<usize> = Vec::new();
        for (idx, &ptr) in self.objects.iter().enumerate() {
            if ptr != 0 && self.marks[idx] == COLOR_WHITE {
                to_free.push(idx);
            }
        }

        let freed = to_free.len();
        for idx in to_free {
            let ptr = self.objects[idx];
            let tag = self.tags[idx];
            let in_nursery = self.nursery.contains(ptr as *const u8);
            unsafe {
                if in_nursery {
                    Self::drop_in_place_tracked(ptr, tag);
                } else {
                    Self::drop_tracked(ptr, tag);
                }
            }
            self.objects[idx] = 0;
            self.marks[idx] = 0;
            self.tags[idx] = TAG_OBJECT;
            self.free_list.push(idx);
            self.live_count = self.live_count.saturating_sub(1);
        }

        self.nursery_indices.clear();
        self.nursery.reset();
        self.gray_stack.clear();
        self.total_allocated = self
            .total_allocated
            .saturating_sub(freed * std::mem::size_of::<JSObject>());
        freed
    }

    pub fn for_each_live_object(&self, mut f: impl FnMut(usize, u8)) {
        for (idx, &ptr) in self.objects.iter().enumerate() {
            if ptr != 0 && self.marks[idx] != COLOR_WHITE {
                f(ptr, self.tags[idx]);
            }
        }
    }

    pub fn object_count(&self) -> usize {
        self.objects.iter().filter(|&&p| p != 0).count()
    }

    #[inline]
    pub fn tracked_ptr_at(&self, idx: usize) -> Option<usize> {
        self.objects.get(idx).copied().filter(|&p| p != 0)
    }

    #[inline]
    pub fn is_ptr_alive(&self, ptr: usize) -> bool {
        let slot = unsafe { (*(ptr as *const JSObject)).gc_slot };
        if slot == u32::MAX {
            return true;
        }
        self.objects.get(slot as usize) == Some(&ptr)
    }

    fn clean_stale_properties_on_nursery_survivors(&mut self) {
        for &idx in &self.nursery_indices {
            if self.marks[idx] == COLOR_WHITE {
                continue;
            }
            let tag = self.tags[idx];
            unsafe {
                match tag {
                    TAG_OBJECT | TAG_FUNCTION => {
                        let obj = &mut *(self.objects[idx] as *mut JSObject);
                        obj.clean_stale_properties(self);
                    }
                    TAG_ARRAY => {
                        let arr = &mut *(self.objects[idx] as *mut JSArrayObject);
                        arr.header.clean_stale_properties(self);
                    }
                    _ => {}
                }
            }
        }
    }

    fn clean_stale_properties_on_live_objects(&mut self) {
        for (idx, &ptr) in self.objects.iter().enumerate() {
            if ptr == 0 || self.marks[idx] == COLOR_WHITE {
                continue;
            }
            let tag = self.tags[idx];
            unsafe {
                match tag {
                    TAG_OBJECT | TAG_FUNCTION => {
                        let obj = &mut *(ptr as *mut JSObject);
                        obj.clean_stale_properties(self);
                    }
                    TAG_ARRAY => {
                        let arr = &mut *(ptr as *mut JSArrayObject);
                        arr.header.clean_stale_properties(self);
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn total_size(&self) -> usize {
        self.total_allocated
    }

    pub fn should_collect(&self) -> bool {
        if self.gc_suppressed {
            return false;
        }
        self.total_allocated > self.threshold || self.live_count > self.count_threshold
    }

    pub fn set_threshold(&mut self, threshold: usize) {
        self.threshold = threshold;
    }
}

impl Default for GcHeap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::object::JSObject;
    use crate::runtime::atom::Atom;
    use crate::value::JSValue;

    #[test]
    fn test_gc_track_untrack() {
        let mut heap = GcHeap::new();
        let obj = JSObject::new();
        let ptr = Box::into_raw(Box::new(obj)) as usize;

        heap.track(ptr);
        assert_eq!(heap.object_count(), 1);

        heap.untrack(ptr);
        assert_eq!(heap.object_count(), 0);

        unsafe {
            let _ = Box::from_raw(ptr as *mut JSObject);
        }
    }

    #[test]
    fn test_gc_mark_value() {
        let mut heap = GcHeap::new();
        let obj = JSObject::new();
        let ptr = Box::into_raw(Box::new(obj)) as usize;

        heap.track(ptr);

        let value = JSValue::new_object(ptr);
        heap.mark_value(&value);

        let idx = unsafe { (*(ptr as *const JSObject)).gc_slot } as usize;
        assert_eq!(heap.marks[idx], 1);

        unsafe {
            let _ = Box::from_raw(ptr as *mut JSObject);
        }
    }

    #[test]
    fn test_gc_run_gc_frees_unreachable() {
        let mut heap = GcHeap::new();

        let reachable_obj = JSObject::new();
        let reachable_ptr = Box::into_raw(Box::new(reachable_obj)) as usize;
        heap.track(reachable_ptr);

        let unreachable_obj = JSObject::new();
        let unreachable_ptr = Box::into_raw(Box::new(unreachable_obj)) as usize;
        heap.track(unreachable_ptr);

        assert_eq!(heap.object_count(), 2);

        let roots = vec![JSValue::new_object(reachable_ptr)];

        let freed = heap.run_gc(&roots);

        assert_eq!(freed, 1);
        assert_eq!(heap.object_count(), 1);
        assert!(heap.objects.contains(&reachable_ptr));
        assert!(!heap.objects.contains(&unreachable_ptr));

        unsafe {
            let _ = Box::from_raw(reachable_ptr as *mut JSObject);
        }
    }

    #[test]
    fn test_gc_trace_object_properties() {
        let mut heap = GcHeap::new();

        let child_obj = JSObject::new();
        let child_ptr = Box::into_raw(Box::new(child_obj)) as usize;
        heap.track(child_ptr);

        let mut parent_obj = JSObject::new();
        parent_obj.set(Atom(0), JSValue::new_object(child_ptr));
        let parent_ptr = Box::into_raw(Box::new(parent_obj)) as usize;
        heap.track(parent_ptr);

        let roots = vec![JSValue::new_object(parent_ptr)];

        let freed = heap.run_gc(&roots);

        assert_eq!(freed, 0);
        assert_eq!(heap.object_count(), 2);

        unsafe {
            let _ = Box::from_raw(parent_ptr as *mut JSObject);
            let _ = Box::from_raw(child_ptr as *mut JSObject);
        }
    }

    #[test]
    fn test_gc_reachable_through_nested_object() {
        let mut heap = GcHeap::new();

        let grandchild = JSObject::new();
        let grandchild_ptr = Box::into_raw(Box::new(grandchild)) as usize;
        heap.track(grandchild_ptr);

        let mut child = JSObject::new();
        child.set(Atom(0), JSValue::new_object(grandchild_ptr));
        let child_ptr = Box::into_raw(Box::new(child)) as usize;
        heap.track(child_ptr);

        let mut parent = JSObject::new();
        parent.set(Atom(0), JSValue::new_object(child_ptr));
        let parent_ptr = Box::into_raw(Box::new(parent)) as usize;
        heap.track(parent_ptr);

        let roots = vec![JSValue::new_object(parent_ptr)];

        let freed = heap.run_gc(&roots);

        assert_eq!(freed, 0);
        assert_eq!(heap.object_count(), 3);

        unsafe {
            let _ = Box::from_raw(parent_ptr as *mut JSObject);
            let _ = Box::from_raw(child_ptr as *mut JSObject);
            let _ = Box::from_raw(grandchild_ptr as *mut JSObject);
        }
    }

    #[test]
    fn test_gc_run_gc_compacts_live_object_props() {
        use crate::object::shape::ShapeCache;
        let mut heap = GcHeap::new();
        let mut cache = ShapeCache::new();

        let mut obj = JSObject::new();
        let a = Atom(1);
        let b = Atom(2);
        let c = Atom(3);
        obj.set(a, JSValue::new_int(10));
        obj.set(b, JSValue::new_int(20));
        obj.set(c, JSValue::new_int(30));
        obj.delete(b);

        assert_eq!(obj.get(a).unwrap().get_int(), 10);
        assert!(obj.get(b).is_none());
        assert_eq!(obj.get(c).unwrap().get_int(), 30);

        let ptr = Box::into_raw(Box::new(obj)) as usize;
        heap.track(ptr);

        let roots = vec![JSValue::new_object(ptr)];
        let freed = heap.run_gc(&roots);
        assert_eq!(freed, 0);
        assert_eq!(heap.object_count(), 1);

        heap.for_each_live_object(|live_ptr, tag| unsafe {
            if tag == TAG_OBJECT {
                let obj_ref = &mut *(live_ptr as *mut JSObject);
                obj_ref.compact_props(&mut cache);
            }
        });

        let obj_ref = unsafe { &*(ptr as *const JSObject) };
        assert_eq!(obj_ref.get(a).unwrap().get_int(), 10);
        assert!(obj_ref.get(b).is_none());
        assert_eq!(obj_ref.get(c).unwrap().get_int(), 30);

        let mut count = 0;
        obj_ref.for_each_property(|_, _, _| count += 1);
        assert_eq!(count, 2);

        unsafe {
            let _ = Box::from_raw(ptr as *mut JSObject);
        }
    }

    #[test]
    fn test_nursery_bump_alloc_basic() {
        let mut nursery = Nursery::new(1024);
        let ptr1 = nursery
            .alloc(JSObject::new())
            .expect("first alloc should succeed");
        let ptr2 = nursery
            .alloc(JSObject::new())
            .expect("second alloc should succeed");
        assert!(!ptr1.is_null());
        assert!(!ptr2.is_null());
        assert_ne!(ptr1, ptr2);

        assert!(nursery.usage() >= std::mem::size_of::<JSObject>() * 2);
    }

    #[test]
    fn test_nursery_alloc_respects_capacity() {
        let capacity = std::mem::size_of::<JSObject>() * 4;
        let mut nursery = Nursery::new(capacity);

        let _ = nursery.alloc(JSObject::new()).expect("should fit");

        let mut count = 1;
        while let Some(_ptr) = nursery.alloc(JSObject::new()) {
            count += 1;
            if count > 100 {
                panic!("nursery should have filled up by now");
            }
        }
        assert!(count >= 1);
    }

    #[test]
    fn test_gc_heap_alloc_object_in_nursery() {
        let mut heap = GcHeap::new();
        let ptr = heap.alloc_object().expect("nursery alloc should succeed");
        assert_eq!(heap.object_count(), 1);
        assert_eq!(heap.nursery_indices.len(), 1);

        unsafe {
            (*ptr).set(Atom(1), JSValue::new_int(42));
            assert_eq!((*ptr).get(Atom(1)).unwrap().get_int(), 42);
        }

        let freed = heap.minor_gc(&[]);
        assert_eq!(freed, 1);
        assert_eq!(heap.object_count(), 0);
        assert!(heap.nursery_indices.is_empty());
    }

    #[test]
    fn test_gc_heap_minor_gc_promotes_reachable() {
        let mut heap = GcHeap::new();
        let ptr = heap.alloc_object().expect("nursery alloc should succeed");
        unsafe {
            (*ptr).set(Atom(1), JSValue::new_int(99));
        }

        let roots = vec![JSValue::new_object(ptr as usize)];
        let freed = heap.minor_gc(&roots);
        assert_eq!(freed, 0);
        assert_eq!(heap.object_count(), 1);
        assert_eq!(heap.nursery_indices.len(), 1);

        let idx = unsafe { (*(ptr as usize as *const JSObject)).gc_slot } as usize;
        assert_eq!(heap.nursery_indices[0], idx);
        let new_ptr = heap.objects[idx];
        let obj_ref = unsafe { &*(new_ptr as *const JSObject) };
        assert_eq!(obj_ref.get(Atom(1)).unwrap().get_int(), 99);

        unsafe {
            std::ptr::drop_in_place(new_ptr as *mut JSObject);
        }
    }

    #[test]
    fn test_gc_heap_minor_gc_frees_unreachable_nursery() {
        let mut heap = GcHeap::new();
        let reachable = heap.alloc_object().expect("alloc should succeed");
        let _unreachable = heap.alloc_object().expect("alloc should succeed");

        let roots = vec![JSValue::new_object(reachable as usize)];
        let freed = heap.minor_gc(&roots);
        assert_eq!(freed, 1);
        assert_eq!(heap.object_count(), 1);

        let freed_full = heap.run_gc(&roots);
        assert_eq!(freed_full, 0);
        assert_eq!(heap.object_count(), 1);

        let idx = unsafe { (*(reachable as *const JSObject)).gc_slot } as usize;
        let new_ptr = heap.objects[idx];
        unsafe {
            std::ptr::drop_in_place(new_ptr as *mut JSObject);
        }
    }

    #[test]
    fn test_gc_heap_nursery_array_alloc_and_minor_gc() {
        let mut heap = GcHeap::new();
        let arr_ptr = heap
            .alloc_array()
            .expect("nursery array alloc should succeed");
        unsafe {
            (*arr_ptr).set(0, JSValue::new_int(1));
            (*arr_ptr).set(1, JSValue::new_int(2));
        }

        let roots = vec![JSValue::new_object(arr_ptr as usize)];
        let freed = heap.minor_gc(&roots);
        assert_eq!(freed, 0);
        assert_eq!(heap.object_count(), 1);
        assert_eq!(heap.nursery_indices.len(), 1);

        let idx = unsafe { (*(arr_ptr as *const JSObject)).gc_slot } as usize;
        assert_eq!(heap.nursery_indices[0], idx);
        let new_ptr = heap.objects[idx];
        let arr_ref = unsafe { &*(new_ptr as *const JSArrayObject) };
        assert_eq!(arr_ref.get(0).unwrap().get_int(), 1);
        assert_eq!(arr_ref.get(1).unwrap().get_int(), 2);

        unsafe {
            std::ptr::drop_in_place(new_ptr as *mut JSArrayObject);
        }
    }

    #[test]
    fn test_incremental_mark_frees_unreachable() {
        let mut heap = GcHeap::new();

        let reachable_obj = JSObject::new();
        let reachable_ptr = Box::into_raw(Box::new(reachable_obj)) as usize;
        heap.track(reachable_ptr);

        let unreachable_obj = JSObject::new();
        let unreachable_ptr = Box::into_raw(Box::new(unreachable_obj)) as usize;
        heap.track(unreachable_ptr);

        let roots = vec![JSValue::new_object(reachable_ptr)];
        heap.start_incremental_mark(&roots);

        let still_marking = heap.incremental_mark_step(1);
        assert!(still_marking || heap.gray_stack.is_empty());

        while heap.incremental_mark_step(10) {}

        let freed = heap.finish_incremental_sweep();
        assert_eq!(freed, 1);
        assert_eq!(heap.object_count(), 1);
        assert!(heap.objects.contains(&reachable_ptr));
        assert!(!heap.objects.contains(&unreachable_ptr));

        unsafe {
            let _ = Box::from_raw(reachable_ptr as *mut JSObject);
        }
    }

    #[test]
    fn test_write_barrier_protects_new_reference() {
        let mut heap = GcHeap::new();

        let parent = JSObject::new();
        let parent_ptr = Box::into_raw(Box::new(parent)) as usize;
        heap.track(parent_ptr);

        let child = JSObject::new();
        let child_ptr = Box::into_raw(Box::new(child)) as usize;
        heap.track(child_ptr);

        let roots = vec![JSValue::new_object(parent_ptr)];
        heap.start_incremental_mark(&roots);
        while heap.incremental_mark_step(1) {
            if heap.marks[unsafe { (*(parent_ptr as *const JSObject)).gc_slot } as usize]
                == COLOR_BLACK
            {
                break;
            }
        }

        let child_value = JSValue::new_object(child_ptr);
        heap.write_barrier(parent_ptr, &child_value);

        while heap.incremental_mark_step(10) {}
        let freed = heap.finish_incremental_sweep();
        assert_eq!(freed, 0);
        assert_eq!(heap.object_count(), 2);

        unsafe {
            let _ = Box::from_raw(parent_ptr as *mut JSObject);
            let _ = Box::from_raw(child_ptr as *mut JSObject);
        }
    }

    #[test]
    fn test_incremental_mark_step_budget() {
        let mut heap = GcHeap::new();
        for _ in 0..10 {
            let obj = JSObject::new();
            let ptr = Box::into_raw(Box::new(obj)) as usize;
            heap.track(ptr);
        }

        heap.start_incremental_mark(&[]);

        let still_marking = heap.incremental_mark_step(5);
        assert!(!still_marking);
        let freed = heap.finish_incremental_sweep();
        assert_eq!(freed, 10);
    }
}
