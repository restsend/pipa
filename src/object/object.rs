use super::shape::{Shape, ShapeCache};
use crate::runtime::atom::Atom;
use crate::util::FxHashMap;
use crate::value::JSValue;
use std::mem::MaybeUninit;

pub struct GeneratorState {
    pub bytecode: Box<crate::compiler::opcode::Bytecode>,

    pub snapshot: Vec<JSValue>,

    pub pc: usize,

    pub done: bool,
}
use std::ptr::NonNull;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedArrayKind {
    Int8,
    Uint8,
    Uint8Clamped,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
    BigInt64,
    BigUint64,
}

impl TypedArrayKind {
    pub fn bytes_per_element(&self) -> usize {
        match self {
            TypedArrayKind::Int8 | TypedArrayKind::Uint8 | TypedArrayKind::Uint8Clamped => 1,
            TypedArrayKind::Int16 | TypedArrayKind::Uint16 => 2,
            TypedArrayKind::Int32 | TypedArrayKind::Uint32 | TypedArrayKind::Float32 => 4,
            TypedArrayKind::Float64 | TypedArrayKind::BigInt64 | TypedArrayKind::BigUint64 => 8,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            TypedArrayKind::Int8 => "Int8Array",
            TypedArrayKind::Uint8 => "Uint8Array",
            TypedArrayKind::Uint8Clamped => "Uint8ClampedArray",
            TypedArrayKind::Int16 => "Int16Array",
            TypedArrayKind::Uint16 => "Uint16Array",
            TypedArrayKind::Int32 => "Int32Array",
            TypedArrayKind::Uint32 => "Uint32Array",
            TypedArrayKind::Float32 => "Float32Array",
            TypedArrayKind::Float64 => "Float64Array",
            TypedArrayKind::BigInt64 => "BigInt64Array",
            TypedArrayKind::BigUint64 => "BigUint64Array",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Ordinary,
    Array,
    Function,
    NativeFunction,
    Error,
    Date,
    RegExp,
    Promise,
    Proxy,
    BigInt,
    Boolean,
    ArrayBuffer,
    TypedArray,
    DataView,
    MappedArguments,
}

#[derive(Clone, Debug)]
pub struct PropertyDescriptor {
    pub value: Option<JSValue>,
    pub writable: bool,
    pub enumerable: bool,
    pub configurable: bool,
    pub get: Option<JSValue>,
    pub set: Option<JSValue>,
}

impl PropertyDescriptor {
    pub fn new_data(value: JSValue) -> Self {
        PropertyDescriptor {
            value: Some(value),
            writable: true,
            enumerable: true,
            configurable: true,
            get: None,
            set: None,
        }
    }

    pub fn new_accessor(get: Option<JSValue>, set: Option<JSValue>) -> Self {
        PropertyDescriptor {
            value: None,
            writable: false,
            enumerable: true,
            configurable: true,
            get,
            set,
        }
    }

    pub fn is_accessor(&self) -> bool {
        self.get.is_some() || self.set.is_some()
    }

    pub fn is_data_descriptor(&self) -> bool {
        self.value.is_some() || (!self.is_accessor() && (self.writable || !self.configurable))
    }

    pub fn default() -> Self {
        PropertyDescriptor {
            value: None,
            writable: false,
            enumerable: false,
            configurable: false,
            get: None,
            set: None,
        }
    }
}

const FLAG_EXTENSIBLE: u8 = 1 << 0;
const FLAG_SEALED: u8 = 1 << 1;
const FLAG_FROZEN: u8 = 1 << 2;
const FLAG_IS_GENERATOR: u8 = 1 << 3;

const FLAG_DENSE_ARRAY: u8 = 1 << 4;

const FLAG_HAS_DELETED_PROPS: u8 = 1 << 5;

pub const ATTR_WRITABLE: u8 = 1 << 0;
pub const ATTR_ENUMERABLE: u8 = 1 << 1;
pub const ATTR_CONFIGURABLE: u8 = 1 << 2;
const ATTR_DELETED: u8 = 1 << 7;
const ATTR_DEFAULT: u8 = ATTR_WRITABLE | ATTR_ENUMERABLE | ATTR_CONFIGURABLE;

#[derive(Clone, Debug)]
pub struct AccessorEntry {
    pub get: Option<JSValue>,
    pub set: Option<JSValue>,
    pub enumerable: bool,
    pub configurable: bool,
}

#[derive(Clone)]
pub struct PropSlot {
    pub value: JSValue,
    pub atom: Atom,
    pub attrs: u8,
    _pad: [u8; 3],
}

impl PropSlot {
    fn new(atom: Atom, value: JSValue, attrs: u8) -> Self {
        PropSlot {
            atom,
            value,
            attrs,
            _pad: [0; 3],
        }
    }
}

pub const INLINE_PROPS: usize = 6;

pub struct SmallPropVec {
    inline: [MaybeUninit<PropSlot>; INLINE_PROPS],
    len: u8,
    heap: Option<Vec<PropSlot>>,
}

impl SmallPropVec {
    #[inline(always)]
    pub fn new() -> Self {
        SmallPropVec {
            inline: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
            heap: None,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len as usize + self.heap.as_ref().map_or(0, |v| v.len())
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub fn push(&mut self, slot: PropSlot) {
        if (self.len as usize) < INLINE_PROPS {
            self.inline[self.len as usize].write(slot);
            self.len += 1;
        } else if let Some(ref mut v) = self.heap {
            v.push(slot);
        } else {
            let mut v: Vec<PropSlot> = Vec::with_capacity(4);
            v.push(slot);
            self.heap = Some(v);
        }
    }

    #[inline(always)]
    pub fn get(&self, idx: usize) -> Option<&PropSlot> {
        if idx < self.len as usize {
            Some(unsafe { self.inline[idx].assume_init_ref() })
        } else if let Some(ref v) = self.heap {
            v.get(idx - INLINE_PROPS)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut PropSlot> {
        if idx < self.len as usize {
            Some(unsafe { self.inline[idx].assume_init_mut() })
        } else if let Some(ref mut v) = self.heap {
            v.get_mut(idx - INLINE_PROPS)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn iter(&self) -> SmallPropIter<'_> {
        SmallPropIter { vec: self, idx: 0 }
    }

    #[inline(always)]
    pub fn iter_mut(&mut self) -> SmallPropIterMut<'_> {
        let len = self.len();
        SmallPropIterMut {
            vec: self,
            idx: 0,
            len,
        }
    }

    pub fn retain<F: FnMut(&mut PropSlot) -> bool>(&mut self, mut f: F) {
        let mut write = 0usize;
        let read_len = self.len as usize;
        for read in 0..read_len {
            let keep = unsafe { f(self.inline[read].assume_init_mut()) };
            if keep {
                if write != read {
                    let val = unsafe { self.inline[read].assume_init_read() };
                    self.inline[write].write(val);
                }
                write += 1;
            }
        }
        self.len = write as u8;

        if let Some(ref mut v) = self.heap {
            v.retain_mut(|s| f(s));
        }
    }

    pub fn truncate(&mut self, new_len: usize) {
        if new_len <= self.len as usize {
            self.len = new_len as u8;
            self.heap = None;
        } else if let Some(ref mut v) = self.heap {
            let heap_new = new_len - INLINE_PROPS;
            v.truncate(heap_new);
            if v.is_empty() {
                self.heap = None;
            }
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
        self.heap = None;
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        INLINE_PROPS + self.heap.as_ref().map_or(0, |v| v.capacity())
    }

    pub fn drain_to_vec(&mut self) -> Vec<PropSlot> {
        let total = self.len();
        let mut out = Vec::with_capacity(total);
        for i in 0..self.len as usize {
            out.push(unsafe { self.inline[i].assume_init_read() });
        }
        self.len = 0;
        if let Some(mut v) = self.heap.take() {
            out.append(&mut v);
        }
        out
    }

    pub fn from_vec(v: Vec<PropSlot>) -> Self {
        if v.len() <= INLINE_PROPS {
            let mut s = SmallPropVec::new();
            for slot in v {
                s.push(slot);
            }
            s
        } else {
            let mut s = SmallPropVec::new();
            let mut it = v.into_iter();
            for i in 0..INLINE_PROPS {
                s.inline[i].write(it.next().unwrap());
            }
            s.len = INLINE_PROPS as u8;
            s.heap = Some(it.collect());
            s
        }
    }
}

impl Default for SmallPropVec {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SmallPropVec {
    fn clone(&self) -> Self {
        let mut new = SmallPropVec::new();
        new.len = self.len;
        for i in 0..self.len as usize {
            unsafe {
                new.inline[i].write(self.inline[i].assume_init_ref().clone());
            }
        }
        new.heap = self.heap.clone();
        new
    }
}

pub struct SmallPropIter<'a> {
    vec: &'a SmallPropVec,
    idx: usize,
}

impl<'a> Iterator for SmallPropIter<'a> {
    type Item = &'a PropSlot;
    fn next(&mut self) -> Option<Self::Item> {
        let s = self.vec.get(self.idx)?;
        self.idx += 1;
        Some(s)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.vec.len().saturating_sub(self.idx);
        (rem, Some(rem))
    }
}

pub struct SmallPropIterMut<'a> {
    vec: &'a mut SmallPropVec,
    idx: usize,
    len: usize,
}

impl<'a> Iterator for SmallPropIterMut<'a> {
    type Item = &'a mut PropSlot;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.len {
            return None;
        }
        let idx = self.idx;
        self.idx += 1;

        unsafe {
            let ptr = self.vec.get_mut(idx)? as *mut PropSlot;
            Some(&mut *ptr)
        }
    }
}

impl std::ops::Index<usize> for SmallPropVec {
    type Output = PropSlot;
    #[inline(always)]
    fn index(&self, idx: usize) -> &PropSlot {
        self.get(idx).expect("SmallPropVec index out of bounds")
    }
}

impl std::ops::IndexMut<usize> for SmallPropVec {
    #[inline(always)]
    fn index_mut(&mut self, idx: usize) -> &mut PropSlot {
        self.get_mut(idx).expect("SmallPropVec index out of bounds")
    }
}
impl<'a> IntoIterator for &'a SmallPropVec {
    type Item = &'a PropSlot;
    type IntoIter = SmallPropIter<'a>;
    fn into_iter(self) -> SmallPropIter<'a> {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut SmallPropVec {
    type Item = &'a mut PropSlot;
    type IntoIter = SmallPropIterMut<'a>;
    fn into_iter(self) -> SmallPropIterMut<'a> {
        self.iter_mut()
    }
}

pub struct ObjectExtra {
    pub bigint_value: i128,
    pub private_fields: Option<Box<FxHashMap<Atom, JSValue>>>,
    pub private_accessors: Option<Box<FxHashMap<Atom, AccessorEntry>>>,
    pub array_elements: Option<Box<Vec<JSValue>>>,

    pub property_map: Option<Box<FxHashMap<Atom, u32>>>,
    pub accessors: Option<Box<FxHashMap<Atom, AccessorEntry>>>,
    pub property_order: Option<Box<Vec<Atom>>>,

    pub compiled_regex: Option<Box<crate::regexp::Regex>>,

    pub array_buffer_data: Option<usize>,

    pub typed_array_kind: Option<TypedArrayKind>,

    pub generator_state: Option<Box<GeneratorState>>,

    pub mapped_args_frame_index: usize,
    pub mapped_args_param_count: u32,
}

pub struct JSObject {
    obj_type: ObjectType,
    pub prototype: Option<*mut JSObject>,

    pub gc_slot: u32,
    flags: u8,
    shape: Option<NonNull<Shape>>,

    pub shape_id_cache: usize,

    props: SmallPropVec,

    extra: Option<Box<ObjectExtra>>,
}

const INLINE_THRESHOLD: usize = 16;

fn attrs_from_bools(writable: bool, enumerable: bool, configurable: bool) -> u8 {
    let mut a = 0u8;
    if writable {
        a |= ATTR_WRITABLE;
    }
    if enumerable {
        a |= ATTR_ENUMERABLE;
    }
    if configurable {
        a |= ATTR_CONFIGURABLE;
    }
    a
}

impl JSObject {
    pub fn ensure_extra(&mut self) -> &mut ObjectExtra {
        self.extra.get_or_insert_with(|| {
            Box::new(ObjectExtra {
                bigint_value: 0,
                private_fields: None,
                array_elements: None,
                private_accessors: None,
                property_map: None,
                accessors: None,
                property_order: None,
                compiled_regex: None,
                array_buffer_data: None,
                typed_array_kind: None,
                generator_state: None,
                mapped_args_frame_index: 0,
                mapped_args_param_count: 0,
            })
        })
    }

    pub fn new_typed(obj_type: ObjectType) -> Self {
        JSObject {
            obj_type,
            prototype: None,
            gc_slot: u32::MAX,
            flags: FLAG_EXTENSIBLE,
            shape: None,
            shape_id_cache: usize::MAX,
            props: SmallPropVec::new(),
            extra: None,
        }
    }

    #[inline]
    pub fn new_typed_from_pool(obj_type: ObjectType, props: SmallPropVec) -> Self {
        debug_assert!(props.is_empty());
        JSObject {
            obj_type,
            prototype: None,
            gc_slot: u32::MAX,
            flags: FLAG_EXTENSIBLE,
            shape: None,
            shape_id_cache: usize::MAX,
            props,
            extra: None,
        }
    }

    pub fn new() -> Self {
        Self::new_typed(ObjectType::Ordinary)
    }
    pub fn new_global() -> Self {
        let mut obj = Self::new();
        obj.prototype = None;
        obj
    }
    pub fn new_array() -> Self {
        Self::new_typed(ObjectType::Array)
    }
    pub fn new_function() -> Self {
        Self::new_typed(ObjectType::Function)
    }
    pub fn new_bigint() -> Self {
        Self::new_typed(ObjectType::BigInt)
    }
    pub fn new_regexp() -> Self {
        Self::new_typed(ObjectType::RegExp)
    }
    pub fn new_promise() -> Self {
        Self::new_typed(ObjectType::Promise)
    }
    pub fn new_error() -> Self {
        Self::new_typed(ObjectType::Error)
    }

    #[inline(always)]
    pub fn obj_type(&self) -> ObjectType {
        self.obj_type
    }

    #[inline(always)]
    pub fn set_obj_type(&mut self, t: ObjectType) {
        self.obj_type = t;
    }

    #[inline(always)]
    pub fn is_array(&self) -> bool {
        self.obj_type == ObjectType::Array
    }

    #[inline(always)]
    pub fn is_promise(&self) -> bool {
        self.obj_type == ObjectType::Promise
    }

    #[inline(always)]
    pub fn is_dense_array(&self) -> bool {
        self.flags & FLAG_DENSE_ARRAY != 0
    }

    #[inline(always)]
    pub fn set_dense_array_flag(&mut self) {
        self.flags |= FLAG_DENSE_ARRAY;
    }

    #[inline(always)]
    pub fn clear_dense_array_flag(&mut self) {
        self.flags &= !FLAG_DENSE_ARRAY;
    }

    #[inline(always)]
    pub fn set_prototype_raw(&mut self, ptr: *mut JSObject) {
        self.prototype = if ptr.is_null() { None } else { Some(ptr) }
    }

    #[inline(always)]
    pub fn prototype_ptr(&self) -> Option<*mut JSObject> {
        self.prototype
    }

    #[inline(always)]
    pub fn extensible(&self) -> bool {
        self.flags & FLAG_EXTENSIBLE != 0
    }
    #[inline(always)]
    pub fn set_extensible(&mut self, val: bool) {
        if val {
            self.flags |= FLAG_EXTENSIBLE;
        } else {
            self.flags &= !FLAG_EXTENSIBLE;
        }
    }
    #[inline(always)]
    pub fn sealed(&self) -> bool {
        self.flags & FLAG_SEALED != 0
    }
    #[inline(always)]
    pub fn set_sealed(&mut self, val: bool) {
        if val {
            self.flags |= FLAG_SEALED;
        } else {
            self.flags &= !FLAG_SEALED;
        }
    }
    #[inline(always)]
    pub fn frozen(&self) -> bool {
        self.flags & FLAG_FROZEN != 0
    }
    #[inline(always)]
    pub fn set_frozen(&mut self, val: bool) {
        if val {
            self.flags |= FLAG_FROZEN;
        } else {
            self.flags &= !FLAG_FROZEN;
        }
    }
    #[inline(always)]
    pub fn is_mapped_arguments(&self) -> bool {
        self.obj_type == ObjectType::MappedArguments
    }

    pub fn mapped_args_frame_index(&self) -> usize {
        self.extra.as_ref().map_or(0, |e| e.mapped_args_frame_index)
    }

    pub fn mapped_args_param_count(&self) -> u32 {
        self.extra.as_ref().map_or(0, |e| e.mapped_args_param_count)
    }

    pub fn is_generator(&self) -> bool {
        self.flags & FLAG_IS_GENERATOR != 0
    }
    #[inline(always)]
    pub fn set_is_generator(&mut self, val: bool) {
        if val {
            self.flags |= FLAG_IS_GENERATOR;
        } else {
            self.flags &= !FLAG_IS_GENERATOR;
        }
    }

    #[inline(always)]
    pub fn get_bigint_value(&self) -> i128 {
        self.extra.as_ref().map_or(0, |e| e.bigint_value)
    }
    #[inline(always)]
    pub fn set_bigint_value(&mut self, val: i128) {
        self.ensure_extra().bigint_value = val;
    }

    #[inline(always)]
    pub fn ensure_elements(&mut self) -> &mut Vec<JSValue> {
        self.ensure_extra()
            .array_elements
            .get_or_insert_with(|| Box::new(Vec::new()))
    }

    pub fn set_array_elements(&mut self, elements: Vec<JSValue>) {
        self.ensure_extra().array_elements = Some(Box::new(elements));
    }

    #[inline]
    pub fn get_compiled_regex(&self) -> Option<&crate::regexp::Regex> {
        self.extra
            .as_ref()
            .and_then(|e| e.compiled_regex.as_deref())
    }

    pub fn set_compiled_regex(&mut self, re: crate::regexp::Regex) {
        self.ensure_extra().compiled_regex = Some(Box::new(re));
    }

    #[inline]
    pub fn get_generator_state(&self) -> Option<&GeneratorState> {
        self.extra
            .as_ref()
            .and_then(|e| e.generator_state.as_deref())
    }

    #[inline]
    pub fn get_generator_state_mut(&mut self) -> Option<&mut GeneratorState> {
        self.extra
            .as_mut()
            .and_then(|e| e.generator_state.as_deref_mut())
    }

    #[inline]
    pub fn set_generator_state(&mut self, state: GeneratorState) {
        self.ensure_extra().generator_state = Some(Box::new(state));
    }

    #[inline]
    pub fn take_generator_state(&mut self) -> Option<Box<GeneratorState>> {
        self.extra.as_mut().and_then(|e| e.generator_state.take())
    }

    #[inline]
    pub fn get_array_buffer_data(&self) -> Option<usize> {
        self.extra.as_ref().and_then(|e| e.array_buffer_data)
    }

    pub fn set_array_buffer_data(&mut self, ptr: usize) {
        self.ensure_extra().array_buffer_data = Some(ptr);
    }

    #[inline]
    pub fn get_typed_array_kind(&self) -> Option<TypedArrayKind> {
        self.extra.as_ref().and_then(|e| e.typed_array_kind)
    }

    pub fn set_typed_array_kind(&mut self, kind: TypedArrayKind) {
        self.ensure_extra().typed_array_kind = Some(kind);
    }

    #[inline]
    pub fn array_elements_len(&self) -> usize {
        self.extra
            .as_ref()
            .and_then(|e| e.array_elements.as_ref())
            .map_or(0, |v| v.len())
    }

    #[inline]
    pub fn get_array_elements(&self) -> Option<&Vec<JSValue>> {
        self.extra
            .as_ref()
            .and_then(|e| e.array_elements.as_deref())
    }

    pub fn for_each_array_element(&self, mut f: impl FnMut(&JSValue)) {
        if let Some(ref extra) = self.extra {
            if let Some(ref elements) = extra.array_elements {
                for value in elements.iter() {
                    f(value);
                }
            }
        }
    }

    #[inline]
    pub fn get_private_field(&self, atom: Atom) -> Option<JSValue> {
        self.extra
            .as_ref()
            .and_then(|e| e.private_fields.as_ref())
            .and_then(|fields| fields.get(&atom).cloned())
    }

    #[inline]
    pub fn has_private_field(&self, atom: Atom) -> bool {
        self.extra
            .as_ref()
            .and_then(|e| e.private_fields.as_ref())
            .map_or(false, |fields| fields.contains_key(&atom))
    }

    pub fn set_private_field(&mut self, atom: Atom, value: JSValue) {
        self.ensure_extra()
            .private_fields
            .get_or_insert_with(|| Box::new(FxHashMap::default()))
            .insert(atom, value);
    }

    pub fn for_each_private_field(&self, mut f: impl FnMut(Atom, JSValue)) {
        if let Some(ref extra) = self.extra {
            if let Some(ref fields) = extra.private_fields {
                for (&atom, value) in fields.iter() {
                    f(atom, value.clone());
                }
            }
        }
    }

    #[inline(always)]
    fn shape_offset(&self, prop: Atom) -> Option<u32> {
        self.shape
            .and_then(|ptr| unsafe { (*ptr.as_ptr()).get_offset(prop) })
    }

    pub fn ensure_shape(&mut self, cache: &mut ShapeCache) -> NonNull<Shape> {
        self.shape.unwrap_or_else(|| {
            let r = cache.root_shape();
            self.shape = Some(r);
            self.shape_id_cache = unsafe { (*r.as_ptr()).id.0 };
            r
        })
    }

    #[inline]
    fn find_offset_linear(&self, prop: Atom) -> Option<usize> {
        let len = self.props.len();

        if len > 0 && self.props[len - 1].atom == prop {
            if self.props[len - 1].attrs != ATTR_DELETED {
                return Some(len - 1);
            }
        }
        for i in 0..len.saturating_sub(1) {
            if self.props[i].atom == prop {
                if self.props[i].attrs == ATTR_DELETED {
                    continue;
                }
                return Some(i);
            }
        }
        None
    }

    #[inline(always)]
    pub fn find_offset(&self, prop: Atom) -> Option<usize> {
        if let Some(offset) = self.shape_offset(prop) {
            let off = offset as usize;
            if off < self.props.len() && self.props[off].attrs == ATTR_DELETED {
            } else {
                return Some(off);
            }
        }

        if let Some(ref extra) = self.extra {
            if let Some(ref map) = extra.property_map {
                if let Some(&offset) = map.get(&prop) {
                    let off = offset as usize;
                    if off < self.props.len() && self.props[off].attrs == ATTR_DELETED {
                    } else {
                        return Some(off);
                    }
                }
            }
        }
        self.find_offset_linear(prop)
    }

    pub fn is_property_writable(&self, prop: Atom) -> bool {
        if let Some(offset) = self.find_offset(prop) {
            if offset < self.props.len() {
                return self.props[offset].attrs & ATTR_WRITABLE != 0;
            }
        }
        true
    }

    #[inline(always)]
    pub fn is_prop_writable_at(&self, offset: Option<usize>) -> bool {
        if let Some(offset) = offset {
            if offset < self.props.len() {
                return self.props[offset].attrs & ATTR_WRITABLE != 0;
            }
        }
        true
    }

    fn maybe_build_property_map(&mut self) {
        let needs_map = self
            .extra
            .as_ref()
            .map_or(true, |e| e.property_map.is_none());
        if needs_map && self.props.len() > INLINE_THRESHOLD {
            let mut map =
                FxHashMap::with_capacity_and_hasher(self.props.len() * 2, Default::default());
            for (i, slot) in self.props.iter().enumerate() {
                map.insert(slot.atom, i as u32);
            }
            self.ensure_extra().property_map = Some(Box::new(map));
        }
    }

    #[inline(always)]
    pub fn get_by_offset(&self, offset: usize) -> Option<JSValue> {
        if offset < self.props.len as usize {
            let slot = unsafe { self.props.inline[offset].assume_init_ref() };
            if slot.attrs != ATTR_DELETED {
                return Some(slot.value);
            }
            return None;
        }

        if let Some(ref v) = self.props.heap {
            let h_idx = offset - INLINE_PROPS;
            if let Some(slot) = v.get(h_idx) {
                if slot.attrs != ATTR_DELETED {
                    return Some(slot.value);
                }
            }
        }
        None
    }

    #[inline(always)]
    pub fn set_by_offset_fast(&mut self, offset: usize, value: JSValue) {
        unsafe { self.props.inline[offset].assume_init_mut() }.value = value;
    }

    #[inline(always)]
    pub fn get_by_offset_fast(&self, offset: usize) -> JSValue {
        debug_assert!(
            offset < INLINE_PROPS,
            "offset out of inline range in get_by_offset_fast"
        );
        debug_assert!(
            unsafe { self.props.inline[offset].assume_init_ref() }.attrs != ATTR_DELETED,
            "deleted prop in get_by_offset_fast"
        );
        unsafe { self.props.inline[offset].assume_init_ref() }.value
    }

    #[inline(always)]
    pub fn has_no_deleted_props(&self) -> bool {
        self.flags & FLAG_HAS_DELETED_PROPS == 0
    }

    #[inline(always)]
    pub fn shape_ptr(&self) -> Option<std::ptr::NonNull<super::shape::Shape>> {
        self.shape
    }

    #[inline(always)]
    pub fn set_by_offset(&mut self, offset: usize, value: JSValue) -> bool {
        if offset < self.props.len as usize {
            let slot = unsafe { self.props.inline[offset].assume_init_mut() };
            if slot.attrs == ATTR_DELETED {
                return false;
            }
            if slot.attrs & ATTR_WRITABLE == 0 {
                return false;
            }
            slot.value = value;
            return true;
        }

        if let Some(ref mut v) = self.props.heap {
            let h_idx = offset - INLINE_PROPS;
            if let Some(slot) = v.get_mut(h_idx) {
                if slot.attrs == ATTR_DELETED {
                    return false;
                }
                if slot.attrs & ATTR_WRITABLE == 0 {
                    return false;
                }
                slot.value = value;
                return true;
            }
        }
        false
    }

    #[inline(always)]
    pub fn push_prop_with_shape(
        &mut self,
        offset: usize,
        atom: Atom,
        value: JSValue,
        new_shape: std::ptr::NonNull<super::shape::Shape>,
    ) {
        debug_assert_eq!(offset, self.props.len());
        self.props.push(PropSlot::new(atom, value, ATTR_DEFAULT));
        self.shape = Some(new_shape);
        self.shape_id_cache = unsafe { (*new_shape.as_ptr()).id.0 };
        self.update_property_map(atom);
    }

    #[inline(always)]

    pub fn fast_init_from_simple_constructor<I>(
        &mut self,
        props: I,
        final_shape: std::ptr::NonNull<super::shape::Shape>,
    ) where
        I: Iterator<Item = (Atom, crate::value::JSValue)>,
    {
        for (atom, value) in props {
            self.props.push(PropSlot::new(atom, value, ATTR_DEFAULT));
        }
        self.shape = Some(final_shape);
        self.shape_id_cache = unsafe { (*final_shape.as_ptr()).id.0 };
    }

    pub fn batch_push_props(
        &mut self,
        atoms: &[Atom],
        values: &[JSValue],
        new_shape: std::ptr::NonNull<super::shape::Shape>,
    ) {
        for (&atom, &value) in atoms.iter().zip(values.iter()) {
            self.props.push(PropSlot::new(atom, value, ATTR_DEFAULT));
        }
        self.shape = Some(new_shape);
        self.shape_id_cache = unsafe { (*new_shape.as_ptr()).id.0 };
        if !atoms.is_empty() {
            self.maybe_build_property_map();
            if let Some(ref mut extra) = self.extra {
                if let Some(ref mut map) = extra.property_map {
                    for (i, &atom) in atoms.iter().enumerate() {
                        map.insert(atom, (self.props.len() - atoms.len() + i) as u32);
                    }
                }
            }
        }
    }

    #[inline(always)]
    pub fn get_own_accessor_value(&self, prop: Atom) -> Option<JSValue> {
        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                if let Some(entry) = accs.get(&prop) {
                    return entry.get;
                }
            }
        }
        None
    }

    pub fn get_own_accessor_entry(&self, prop: Atom) -> Option<&AccessorEntry> {
        self.extra.as_ref()?.accessors.as_ref()?.get(&prop)
    }

    pub fn define_accessor(
        &mut self,
        prop: Atom,
        getter: Option<JSValue>,
        setter: Option<JSValue>,
    ) {
        self.ensure_extra()
            .accessors
            .get_or_insert_with(|| Box::new(FxHashMap::default()))
            .insert(
                prop,
                AccessorEntry {
                    get: getter,
                    set: setter,
                    enumerable: true,
                    configurable: true,
                },
            );
    }

    pub fn define_non_enumerable_accessor(
        &mut self,
        prop: Atom,
        getter: Option<JSValue>,
        setter: Option<JSValue>,
    ) {
        self.ensure_extra()
            .accessors
            .get_or_insert_with(|| Box::new(FxHashMap::default()))
            .insert(
                prop,
                AccessorEntry {
                    get: getter,
                    set: setter,
                    enumerable: false,
                    configurable: false,
                },
            );
    }

    pub fn get_own_private_accessor_entry(&self, prop: Atom) -> Option<&AccessorEntry> {
        self.extra.as_ref()?.private_accessors.as_ref()?.get(&prop)
    }

    pub fn define_private_accessor(
        &mut self,
        prop: Atom,
        getter: Option<JSValue>,
        setter: Option<JSValue>,
    ) {
        let existing = self
            .extra
            .as_ref()
            .and_then(|e| e.private_accessors.as_ref())
            .and_then(|a| a.get(&prop))
            .cloned();
        let (final_getter, final_setter) = match existing {
            Some(e) => (getter.or(e.get), setter.or(e.set)),
            None => (getter, setter),
        };
        self.ensure_extra()
            .private_accessors
            .get_or_insert_with(|| Box::new(FxHashMap::default()))
            .insert(
                prop,
                AccessorEntry {
                    get: final_getter,
                    set: final_setter,
                    enumerable: false,
                    configurable: false,
                },
            );
    }

    #[inline(always)]
    pub fn get_own_value(&self, prop: Atom) -> Option<JSValue> {
        if let Some(offset) = self.find_offset(prop) {
            if offset < self.props.len() {
                return Some(self.props[offset].value);
            }
        }
        self.get_own_accessor_value(prop)
    }

    #[inline(always)]
    pub fn get(&self, prop: Atom) -> Option<JSValue> {
        if let Some(offset) = self.find_offset(prop) {
            if offset < self.props.len() {
                return Some(self.props[offset].value);
            }
        }
        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                if let Some(entry) = accs.get(&prop) {
                    return entry.get;
                }
            }
        }

        let mut current = self.prototype;
        let mut depth = 0u32;
        while let Some(proto_ptr) = current {
            if proto_ptr.is_null() || depth > 1000 {
                break;
            }
            depth += 1;
            unsafe {
                let proto = &*proto_ptr;
                if let Some(offset) = proto.find_offset(prop) {
                    if offset < proto.props.len() {
                        return Some(proto.props[offset].value);
                    }
                }
                if let Some(ref extra) = proto.extra {
                    if let Some(ref accs) = extra.accessors {
                        if let Some(entry) = accs.get(&prop) {
                            return entry.get;
                        }
                    }
                }
                current = proto.prototype;
            }
        }
        None
    }

    #[inline(always)]
    pub fn set(&mut self, prop: Atom, value: JSValue) {
        if let Some(offset) = self.find_offset(prop) {
            if offset < self.props.len() {
                if self.props[offset].attrs & ATTR_WRITABLE == 0 {
                    return;
                }
                self.props[offset].value = value;
                return;
            }
        }

        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                if accs.contains_key(&prop) {
                    return;
                }
            }
        }
        if !self.extensible() {
            return;
        }

        self.props.push(PropSlot::new(prop, value, ATTR_DEFAULT));
        self.track_property_order(prop);
        self.update_property_map(prop);
    }

    pub fn set_length(&mut self, prop: Atom, value: JSValue) {
        if self.has_own(prop) {
            self.set(prop, value);
        } else {
            self.props.push(PropSlot::new(prop, value, ATTR_WRITABLE));
            self.update_property_map(prop);
        }
    }

    pub fn set_length_ic(
        &mut self,
        prop: Atom,
        value: JSValue,
        cache: &mut crate::object::shape::ShapeCache,
    ) {
        if let Some(shape) = self.shape {
            if let Some(offset) = unsafe { (*shape.as_ptr()).get_offset(prop) } {
                let off = offset as usize;
                if off < self.props.len() {
                    self.props[off].value = value;
                    return;
                }
            }

            let new_shape = cache.transition(shape, prop);
            self.shape = Some(new_shape);
            self.shape_id_cache = unsafe { (*new_shape.as_ptr()).id.0 };
            self.props.push(PropSlot::new(prop, value, ATTR_WRITABLE));
            self.update_property_map(prop);
        } else {
            self.set_length(prop, value);
        }
    }

    pub fn set_cached_non_configurable(
        &mut self,
        prop: Atom,
        value: JSValue,
        cache: &mut ShapeCache,
    ) {
        if self.has_own(prop) {
            if let Some(offset) = self.find_offset(prop) {
                if offset < self.props.len() {
                    if self.props[offset].attrs & ATTR_WRITABLE == 0 {
                        return;
                    }
                    self.props[offset].value = value;
                    return;
                }
            }
        }
        let _ = cache;
        self.props.push(PropSlot::new(prop, value, ATTR_WRITABLE));
        self.update_property_map(prop);
    }

    #[inline(always)]
    pub fn set_with_cache(&mut self, prop: Atom, value: JSValue, cache: &mut ShapeCache) {
        if let Some(offset) = self.find_offset(prop) {
            if offset < self.props.len() {
                if self.props[offset].attrs & ATTR_WRITABLE == 0 {
                    return;
                }
                self.props[offset].value = value;
                return;
            }
        }

        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                if accs.contains_key(&prop) {
                    return;
                }
            }
        }
        if !self.extensible() {
            return;
        }

        if self.shape.is_some() {
            let current = self.shape.unwrap();
            let new_shape = cache.transition(current, prop);
            self.shape = Some(new_shape);
            self.shape_id_cache = unsafe { (*new_shape.as_ptr()).id.0 };
        } else if self.props.is_empty() {
            let current = self.ensure_shape(cache);
            let new_shape = cache.transition(current, prop);
            self.shape = Some(new_shape);
            self.shape_id_cache = unsafe { (*new_shape.as_ptr()).id.0 };
        }
        self.props.push(PropSlot::new(prop, value, ATTR_DEFAULT));
        self.update_property_map(prop);
    }

    #[inline(always)]
    pub fn set_cached(&mut self, prop: Atom, value: JSValue, cache: &mut ShapeCache) {
        if let Some(offset) = self.shape_offset(prop) {
            let off = offset as usize;
            if off < self.props.len() {
                if self.props[off].attrs & ATTR_WRITABLE == 0 {
                    return;
                }
                self.props[off].value = value;
                return;
            }
        } else {
            let found = if let Some(ref extra) = self.extra {
                if let Some(ref map) = extra.property_map {
                    map.get(&prop)
                        .copied()
                        .map(|o| o as usize)
                        .filter(|&o| o < self.props.len() && self.props[o].attrs != ATTR_DELETED)
                } else {
                    self.find_offset_linear(prop)
                }
            } else {
                self.find_offset_linear(prop)
            };
            if let Some(offset) = found {
                if self.props[offset].attrs & ATTR_WRITABLE == 0 {
                    return;
                }
                self.props[offset].value = value;
                return;
            }
        }

        self.set_cached_new(prop, value, cache);
    }

    pub fn set_cached_with_offset(
        &mut self,
        prop: Atom,
        value: JSValue,
        cache: &mut ShapeCache,
        pre_offset: Option<usize>,
    ) {
        if let Some(offset) = pre_offset {
            if offset < self.props.len() {
                if self.props[offset].attrs & ATTR_WRITABLE == 0 {
                    return;
                }
                self.props[offset].value = value;
                return;
            }
        }
        self.set_cached_new(prop, value, cache);
    }

    fn set_cached_new(&mut self, prop: Atom, value: JSValue, cache: &mut ShapeCache) {
        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                if accs.contains_key(&prop) {
                    return;
                }
            }
        }
        if !self.extensible() {
            return;
        }

        if self.shape.is_none() && !self.props.is_empty() {
        } else if self.shape.is_some() {
            let current = self.shape.unwrap();
            let shape_count = unsafe { (*current.as_ptr()).property_count } as usize;
            if shape_count == self.props.len() {
                let new_shape = cache.transition(current, prop);
                self.shape = Some(new_shape);
                self.shape_id_cache = unsafe { (*new_shape.as_ptr()).id.0 };
            }
        } else {
            let current = self.ensure_shape(cache);
            let new_shape = cache.transition(current, prop);
            self.shape = Some(new_shape);
            self.shape_id_cache = unsafe { (*new_shape.as_ptr()).id.0 };
        }
        self.props.push(PropSlot::new(prop, value, ATTR_DEFAULT));
        self.track_property_order(prop);
        self.update_property_map(prop);
    }

    pub fn set_cached_non_enumerable(
        &mut self,
        prop: Atom,
        value: JSValue,
        cache: &mut ShapeCache,
    ) {
        if let Some(offset) = self.shape_offset(prop) {
            let off = offset as usize;
            if off < self.props.len() {
                if self.props[off].attrs & ATTR_WRITABLE == 0 {
                    return;
                }
                self.props[off].value = value;
                return;
            }
        } else {
            let found = self.find_offset(prop);
            if let Some(offset) = found {
                if self.props[offset].attrs & ATTR_WRITABLE == 0 {
                    return;
                }
                self.props[offset].value = value;
                return;
            }
        }

        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                if accs.contains_key(&prop) {
                    return;
                }
            }
        }
        if !self.extensible() {
            return;
        }

        if self.shape.is_none() && !self.props.is_empty() {
        } else if self.shape.is_some() {
            let current = self.shape.unwrap();
            let new_shape = cache.transition(current, prop);
            self.shape = Some(new_shape);
            self.shape_id_cache = unsafe { (*new_shape.as_ptr()).id.0 };
        } else {
            let current = self.ensure_shape(cache);
            let new_shape = cache.transition(current, prop);
            self.shape = Some(new_shape);
            self.shape_id_cache = unsafe { (*new_shape.as_ptr()).id.0 };
        }
        self.props.push(PropSlot::new(
            prop,
            value,
            ATTR_WRITABLE | ATTR_CONFIGURABLE,
        ));

        self.update_property_map(prop);
    }

    pub fn define_cached(&mut self, prop: Atom, value: JSValue, cache: &mut ShapeCache) {
        if let Some(offset) = self.shape_offset(prop) {
            let off = offset as usize;
            if off < self.props.len() {
                self.props[off].value = value;
                return;
            }
        } else {
            let found = if let Some(ref extra) = self.extra {
                if let Some(ref map) = extra.property_map {
                    map.get(&prop)
                        .copied()
                        .map(|o| o as usize)
                        .filter(|&o| o < self.props.len() && self.props[o].attrs != ATTR_DELETED)
                } else {
                    self.find_offset_linear(prop)
                }
            } else {
                self.find_offset_linear(prop)
            };
            if let Some(offset) = found {
                self.props[offset].value = value;
                return;
            }
        }

        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                if accs.contains_key(&prop) {
                    return;
                }
            }
        }

        if !self.extensible() {
            return;
        }

        if self.shape.is_none() && !self.props.is_empty() {
        } else if self.shape.is_some() {
            let current = self.shape.unwrap();
            let new_shape = cache.transition(current, prop);
            self.shape = Some(new_shape);
            self.shape_id_cache = unsafe { (*new_shape.as_ptr()).id.0 };
        } else {
            let current = self.ensure_shape(cache);
            let new_shape = cache.transition(current, prop);
            self.shape = Some(new_shape);
            self.shape_id_cache = unsafe { (*new_shape.as_ptr()).id.0 };
        }
        self.props.push(PropSlot::new(prop, value, ATTR_DEFAULT));
        self.update_property_map(prop);
    }

    #[inline]
    fn update_property_map(&mut self, prop: Atom) {
        if let Some(ref mut extra) = self.extra {
            if let Some(ref mut map) = extra.property_map {
                map.insert(prop, (self.props.len() - 1) as u32);
                return;
            }
        }
        self.maybe_build_property_map();
    }

    fn track_property_order(&mut self, prop: Atom) {
        if let Some(ref mut extra) = self.extra {
            let order = extra
                .property_order
                .get_or_insert_with(|| Box::new(Vec::new()));
            if !order.contains(&prop) {
                order.push(prop);
            }
        }
    }

    pub fn get_own(&self, prop: Atom) -> Option<JSValue> {
        if let Some(offset) = self.find_offset(prop) {
            if offset < self.props.len() {
                return Some(self.props[offset].value);
            }
        }
        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                if let Some(entry) = accs.get(&prop) {
                    return entry.get;
                }
            }
        }
        None
    }

    pub fn own_properties(&self) -> Vec<(Atom, JSValue)> {
        let mut result = Vec::new();
        // Use the property order vector if available, otherwise fall back to props order
        if let Some(ref extra) = self.extra {
            if let Some(ref order) = extra.property_order {
                for atom in order.iter() {
                    // Check if it's a data property
                    if let Some(offset) = self.find_offset(*atom) {
                        if offset < self.props.len() {
                            let slot = &self.props[offset];
                            if slot.attrs == ATTR_DELETED {
                                continue;
                            }
                            if slot.attrs & ATTR_ENUMERABLE != 0 {
                                result.push((slot.atom, slot.value));
                            }
                            continue;
                        }
                    }
                    // Check if it's an accessor property
                    if let Some(ref accs) = extra.accessors {
                        if let Some(entry) = accs.get(atom) {
                            if entry.enumerable {
                                if let Some(getter) = entry.get {
                                    result.push((*atom, getter));
                                }
                            }
                        }
                    }
                }
                return result;
            }
        }
        // Fallback: iterate props then accessors (for objects without property_order)
        for slot in &self.props {
            if slot.attrs == ATTR_DELETED {
                continue;
            }
            if slot.attrs & ATTR_ENUMERABLE != 0 {
                result.push((slot.atom, slot.value));
            }
        }
        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                for (atom, entry) in accs.iter() {
                    if entry.enumerable {
                        if let Some(getter) = entry.get {
                            result.push((*atom, getter));
                        }
                    }
                }
            }
        }
        result
    }

    pub fn non_enumerable_property_atoms(&self) -> Vec<Atom> {
        let mut result = Vec::new();
        for slot in &self.props {
            if slot.attrs == ATTR_DELETED {
                continue;
            }
            if slot.attrs & ATTR_ENUMERABLE == 0 {
                result.push(slot.atom);
            }
        }
        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                for (atom, entry) in accs.iter() {
                    if !entry.enumerable {
                        result.push(*atom);
                    }
                }
            }
        }
        result
    }

    pub fn get_own_descriptor(&self, prop: Atom) -> Option<PropertyDescriptor> {
        if let Some(offset) = self.find_offset(prop) {
            if offset < self.props.len() {
                let a = self.props[offset].attrs;
                let (w, e, c) = (
                    a & ATTR_WRITABLE != 0,
                    a & ATTR_ENUMERABLE != 0,
                    a & ATTR_CONFIGURABLE != 0,
                );
                return Some(PropertyDescriptor {
                    value: Some(self.props[offset].value),
                    writable: w,
                    enumerable: e,
                    configurable: c,
                    get: None,
                    set: None,
                });
            }
        }
        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                if let Some(entry) = accs.get(&prop) {
                    return Some(PropertyDescriptor {
                        value: None,
                        writable: false,
                        enumerable: entry.enumerable,
                        configurable: entry.configurable,
                        get: entry.get,
                        set: entry.set,
                    });
                }
            }
        }
        None
    }

    pub fn define_property(&mut self, prop: Atom, desc: PropertyDescriptor) -> bool {
        self.define_property_ext(prop, desc, true, true, true)
    }

    pub fn define_property_ext(
        &mut self,
        prop: Atom,
        mut desc: PropertyDescriptor,
        has_writable: bool,
        has_enumerable: bool,
        has_configurable: bool,
    ) -> bool {
        if let Some(existing) = self.get_own_descriptor(prop) {
            if !existing.configurable {
                if has_writable && desc.writable != existing.writable {
                    return false;
                }
                if has_enumerable && desc.enumerable != existing.enumerable {
                    return false;
                }
                if has_configurable && desc.configurable != existing.configurable {
                    return false;
                }
                if let Some(value) = desc.value {
                    if !existing.writable {
                        return false;
                    }
                    self.set(prop, value);
                }
                if desc.get.is_some() || desc.set.is_some() {
                    return false;
                }
                return true;
            }
            // Preserve existing attributes for any fields not specified in the descriptor
            if !has_enumerable {
                desc.enumerable = existing.enumerable;
            }
            if !has_writable {
                desc.writable = existing.writable;
            }
            if !has_configurable {
                desc.configurable = existing.configurable;
            }
            // Update the existing property in-place to preserve creation order
            if desc.is_accessor() {
                let extra = self.ensure_extra();
                let accs = extra
                    .accessors
                    .get_or_insert_with(|| Box::new(FxHashMap::default()));
                let is_new = !accs.contains_key(&prop);
                accs.insert(
                    prop,
                    AccessorEntry {
                        get: desc.get,
                        set: desc.set,
                        enumerable: desc.enumerable,
                        configurable: desc.configurable,
                    },
                );
                if is_new {
                    let order = extra
                        .property_order
                        .get_or_insert_with(|| Box::new(Vec::new()));
                    order.push(prop);
                }
            } else if let Some(value) = desc.value {
                if let Some(offset) = self.find_offset(prop) {
                    if offset < self.props.len() {
                        self.props[offset].value = value;
                        self.props[offset].attrs = attrs_from_bools(
                            desc.writable, desc.enumerable, desc.configurable,
                        );
                    }
                } else {
                    self.props.push(PropSlot::new(
                        prop,
                        value,
                        attrs_from_bools(desc.writable, desc.enumerable, desc.configurable),
                    ));
                }
            }
            return true;
        }
        if desc.is_accessor() {
            let extra = self.ensure_extra();
            let accs = extra
                .accessors
                .get_or_insert_with(|| Box::new(FxHashMap::default()));
            accs.insert(
                prop,
                AccessorEntry {
                    get: desc.get,
                    set: desc.set,
                    enumerable: desc.enumerable,
                    configurable: desc.configurable,
                },
            );
            let order = extra
                .property_order
                .get_or_insert_with(|| Box::new(Vec::new()));
            order.push(prop);
        } else if let Some(value) = desc.value {
            self.props.push(PropSlot::new(
                prop,
                value,
                attrs_from_bools(desc.writable, desc.enumerable, desc.configurable),
            ));
            self.track_property_order(prop);
            self.update_property_map(prop);
        }
        true
    }

    pub fn delete(&mut self, prop: Atom) -> bool {
        if let Some(offset) = self.find_offset(prop) {
            if offset < self.props.len() {
                if self.props[offset].attrs & ATTR_CONFIGURABLE == 0 {
                    return false;
                }
                self.props[offset].value = JSValue::undefined();
                self.props[offset].attrs = ATTR_DELETED;
                self.flags |= FLAG_HAS_DELETED_PROPS;
                return true;
            }
        }
        if let Some(ref mut extra) = self.extra {
            if let Some(ref mut accs) = extra.accessors {
                if let Some(entry) = accs.get(&prop) {
                    if !entry.configurable {
                        return false;
                    }
                }
                if accs.remove(&prop).is_some() {
                    return true;
                }
            }
        }
        true
    }

    #[inline(always)]
    pub fn add_property_ic(
        &mut self,
        atom: Atom,
        value: JSValue,
        offset: usize,
        new_shape: NonNull<crate::object::shape::Shape>,
    ) {
        debug_assert!(offset == self.props.len());
        self.props.push(PropSlot::new(atom, value, ATTR_DEFAULT));
        self.shape = Some(new_shape);
        self.shape_id_cache = unsafe { (*new_shape.as_ptr()).id.0 };

        self.update_property_map(atom);
    }

    pub fn compact_props(&mut self, cache: &mut ShapeCache) {
        if self.flags & FLAG_HAS_DELETED_PROPS == 0 {
            return;
        }
        let has_deleted = self.props.iter().any(|p| p.attrs == ATTR_DELETED);
        if !has_deleted {
            self.flags &= !FLAG_HAS_DELETED_PROPS;
            return;
        }

        let mut new_props: SmallPropVec = SmallPropVec::new();
        let mut property_atoms = Vec::new();

        let old_len = self.props.len();
        for i in 0..old_len {
            let slot = self.props.get(i).expect("valid index");
            if slot.attrs != ATTR_DELETED {
                property_atoms.push(slot.atom);
                new_props.push(slot.clone());
            }
        }

        self.props = new_props;

        if !property_atoms.is_empty() {
            let new_shape = cache.create_shape_for_properties(&property_atoms);
            self.shape = Some(new_shape);
            self.shape_id_cache = unsafe { (*new_shape.as_ptr()).id.0 };
        } else {
            self.shape = None;
            self.shape_id_cache = usize::MAX;
        }

        if let Some(ref mut extra) = self.extra {
            if let Some(ref mut map) = extra.property_map {
                let mut new_map = FxHashMap::default();
                for (i, atom) in property_atoms.iter().enumerate() {
                    new_map.insert(*atom, i as u32);
                }
                *map = Box::new(new_map);
            }
        }

        self.flags &= !FLAG_HAS_DELETED_PROPS;
    }

    pub fn has_property(&self, prop: Atom) -> bool {
        if self.find_offset(prop).is_some() {
            return true;
        }
        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                if accs.contains_key(&prop) {
                    return true;
                }
            }
        }
        let mut current = self.prototype;
        let mut depth = 0u32;
        while let Some(proto_ptr) = current {
            if proto_ptr.is_null() || depth > 1000 {
                break;
            }
            depth += 1;
            unsafe {
                let proto = &*proto_ptr;
                if proto.find_offset(prop).is_some() {
                    return true;
                }
                if let Some(ref extra) = proto.extra {
                    if let Some(ref accs) = extra.accessors {
                        if accs.contains_key(&prop) {
                            return true;
                        }
                    }
                }
                current = proto.prototype;
            }
        }
        false
    }

    pub fn has_own(&self, prop: Atom) -> bool {
        if self.find_offset(prop).is_some() {
            return true;
        }
        self.extra
            .as_ref()
            .and_then(|e| e.accessors.as_ref())
            .map_or(false, |a| a.contains_key(&prop))
    }

    pub fn keys(&self) -> Vec<Atom> {
        let mut result = Vec::new();
        for slot in &self.props {
            if slot.attrs != ATTR_DELETED && slot.atom.0 & 0x40000000 == 0 {
                result.push(slot.atom);
            }
        }
        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                for atom in accs.keys() {
                    if atom.0 & 0x40000000 == 0 {
                        result.push(*atom);
                    }
                }
            }
        }
        result
    }

    pub fn all_keys(&self) -> Vec<Atom> {
        let mut result: Vec<Atom> = Vec::new();
        for slot in &self.props {
            if slot.attrs != ATTR_DELETED {
                result.push(slot.atom);
            }
        }
        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                for atom in accs.keys() {
                    result.push(*atom);
                }
            }
        }
        result
    }

    pub fn symbol_keys(&self) -> Vec<JSValue> {
        let mut result = Vec::new();
        for i in 0..self.props.len() {
            let slot = self.props.get(i).expect("valid index");
            if slot.attrs != ATTR_DELETED && slot.atom.0 & 0x40000000 != 0 {
                let symbol_id = slot.atom.0 & 0x3FFFFFFF;
                result.push(JSValue::new_symbol_with_id(
                    crate::runtime::atom::Atom(0),
                    symbol_id,
                ));
            }
        }
        result
    }

    pub fn enumerable_keys(&self) -> Vec<Atom> {
        let mut result = Vec::new();
        for slot in &self.props {
            if slot.attrs != ATTR_DELETED
                && slot.attrs & ATTR_ENUMERABLE != 0
                && slot.atom.0 & 0x40000000 == 0
            {
                result.push(slot.atom);
            }
        }
        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                for (atom, entry) in accs.iter() {
                    if entry.enumerable && atom.0 & 0x40000000 == 0 {
                        result.push(*atom);
                    }
                }
            }
        }
        result
    }

    pub fn for_each_property<F: FnMut(Atom, JSValue, u8)>(&self, mut f: F) {
        for slot in &self.props {
            if slot.attrs == ATTR_DELETED {
                continue;
            }
            f(slot.atom, slot.value, slot.attrs);
        }
    }

    pub fn for_each_accessor<F: FnMut(Atom, Option<JSValue>, Option<JSValue>)>(&self, mut f: F) {
        if let Some(ref extra) = self.extra {
            if let Some(ref accs) = extra.accessors {
                for (atom, entry) in accs.iter() {
                    f(*atom, entry.get, entry.set);
                }
            }
        }
    }

    pub fn get_private_accessors_for_gc(&self) -> Option<Vec<JSValue>> {
        if let Some(ref extra) = self.extra {
            if let Some(ref priv_accs) = extra.private_accessors {
                let mut values = Vec::new();
                for (_atom, entry) in priv_accs.iter() {
                    if let Some(g) = entry.get {
                        values.push(g);
                    }
                    if let Some(s) = entry.set {
                        values.push(s);
                    }
                }
                if values.is_empty() {
                    None
                } else {
                    Some(values)
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn for_each_property_attrs_mut<F: FnMut(Atom, JSValue, &mut u8)>(&mut self, mut f: F) {
        for slot in &mut self.props {
            f(slot.atom, slot.value, &mut slot.attrs);
        }
    }

    #[inline]
    pub fn get_indexed(&self, index: usize) -> Option<JSValue> {
        self.extra
            .as_ref()
            .and_then(|e| e.array_elements.as_ref())
            .and_then(|elements| elements.get(index).copied())
    }

    #[inline]
    pub fn get_dense_slice(&self, len: usize) -> Option<&[JSValue]> {
        self.extra
            .as_ref()
            .and_then(|e| e.array_elements.as_ref())
            .filter(|elems| elems.len() >= len)
            .map(|elems| &elems[..len])
    }

    #[inline]
    pub fn set_first_prop_with_shape(
        &mut self,
        atom: Atom,
        value: JSValue,
        attrs: u8,
        shape: std::ptr::NonNull<crate::object::shape::Shape>,
    ) {
        debug_assert!(
            self.props.is_empty(),
            "set_first_prop_with_shape: object must be empty"
        );
        self.props.push(PropSlot::new(atom, value, attrs));
        self.shape = Some(shape);
        self.shape_id_cache = unsafe { (*shape.as_ptr()).id.0 };
    }

    #[inline]
    pub fn set_indexed(&mut self, index: usize, value: JSValue) {
        let elements = self.ensure_elements();
        if index < elements.len() {
            elements[index] = value;
        } else if index < 100000 {
            elements.resize(index + 1, JSValue::undefined());
            elements[index] = value;
        }
    }

    #[inline]
    pub fn maybe_set_indexed(&mut self, index: usize, value: JSValue) -> bool {
        if let Some(extra) = &mut self.extra {
            if let Some(elements) = &mut extra.array_elements {
                if index < elements.len() {
                    elements[index] = value;
                    return true;
                }
            }
        }
        false
    }

    #[inline]
    pub fn array_len(&self) -> usize {
        self.extra
            .as_ref()
            .and_then(|e| e.array_elements.as_ref())
            .map_or(0, |v| v.len())
    }

    #[inline]
    pub fn has_dense_storage(&self) -> bool {
        self.extra
            .as_ref()
            .and_then(|e| e.array_elements.as_ref())
            .map_or(false, |v| !v.is_empty())
    }

    #[inline(always)]
    pub fn get_shape_id(&self) -> Option<super::shape::ShapeId> {
        if self.shape_id_cache == usize::MAX {
            None
        } else {
            Some(super::shape::ShapeId(self.shape_id_cache))
        }
    }

    #[inline(always)]
    pub fn get_shape_ptr(&self) -> Option<std::ptr::NonNull<super::shape::Shape>> {
        self.shape
    }

    #[inline(always)]
    pub fn props_len(&self) -> usize {
        self.props.len()
    }

    pub fn iter_props(&self) -> SmallPropIter<'_> {
        self.props.iter()
    }

    #[inline]
    pub fn take_props(&mut self) -> SmallPropVec {
        std::mem::take(&mut self.props)
    }

    pub fn clean_stale_properties(&mut self, heap: &crate::runtime::gc::GcHeap) {
        for slot in &mut self.props {
            if slot.attrs == ATTR_DELETED {
                continue;
            }
            if slot.value.is_object() || slot.value.is_function() {
                let ptr = slot.value.get_ptr();
                if !heap.is_ptr_alive(ptr) {
                    slot.value = JSValue::undefined();
                }
            }
        }
    }

    #[inline(always)]
    pub fn get_inline_value_at(&self, offset: usize) -> Option<JSValue> {
        self.props.get(offset).map(|s| s.value)
    }

    #[inline(always)]
    pub fn get_shape(&self) -> Option<NonNull<Shape>> {
        self.shape
    }

    pub fn assign_shape_from_existing_props(&mut self, cache: &mut ShapeCache) {
        if self.shape.is_some() {
            return;
        }
        let mut current = cache.root_shape();

        let inline_len = self.props.len().min(INLINE_PROPS);
        for i in 0..inline_len {
            let slot = unsafe { self.props.inline[i].assume_init_ref() };
            if slot.attrs != ATTR_DELETED {
                current = cache.transition(current, slot.atom);
            }
        }

        if let Some(ref heap) = self.props.heap {
            for slot in heap.iter() {
                if slot.attrs != ATTR_DELETED {
                    current = cache.transition(current, slot.atom);
                }
            }
        }
        self.shape = Some(current);
        self.shape_id_cache = unsafe { (*current.as_ptr()).id.0 };
    }

    #[inline(always)]
    pub fn inline_values_len(&self) -> usize {
        self.props.len()
    }
}

impl Default for JSObject {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prop_slot_size() {
        assert_eq!(std::mem::size_of::<PropSlot>(), 16);
    }

    #[test]
    fn test_prop_slot_new() {
        let slot = PropSlot::new(Atom(42), JSValue::new_int(100), ATTR_DEFAULT);
        assert_eq!(slot.atom, Atom(42));
        assert_eq!(slot.value.get_int(), 100);
        assert_eq!(
            slot.attrs,
            ATTR_WRITABLE | ATTR_ENUMERABLE | ATTR_CONFIGURABLE
        );
    }

    #[test]
    fn test_object_new_has_no_properties() {
        let obj = JSObject::new();
        assert_eq!(obj.inline_values_len(), 0);
        assert_eq!(obj.keys().len(), 0);
    }

    #[test]
    fn test_object_set_and_get() {
        let mut obj = JSObject::new();
        let atom_a = Atom(1);
        obj.set(atom_a, JSValue::new_int(42));
        assert_eq!(obj.inline_values_len(), 1);

        let val = obj.get(atom_a);
        assert!(val.is_some());
        assert_eq!(val.unwrap().get_int(), 42);
    }

    #[test]
    fn test_object_set_multiple_properties() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));
        obj.set(Atom(2), JSValue::new_int(20));
        obj.set(Atom(3), JSValue::new_int(30));

        assert_eq!(obj.inline_values_len(), 3);
        assert_eq!(obj.get(Atom(1)).unwrap().get_int(), 10);
        assert_eq!(obj.get(Atom(2)).unwrap().get_int(), 20);
        assert_eq!(obj.get(Atom(3)).unwrap().get_int(), 30);
    }

    #[test]
    fn test_object_set_overwrites_existing() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));
        obj.set(Atom(1), JSValue::new_int(20));

        assert_eq!(obj.inline_values_len(), 1);
        assert_eq!(obj.get(Atom(1)).unwrap().get_int(), 20);
    }

    #[test]
    fn test_object_get_nonexistent_returns_none() {
        let obj = JSObject::new();
        assert!(obj.get(Atom(999)).is_none());
    }

    #[test]
    fn test_object_delete_property() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(42));
        assert!(obj.get(Atom(1)).is_some());

        let deleted = obj.delete(Atom(1));
        assert!(deleted);
        assert!(obj.get(Atom(1)).is_none());
    }

    #[test]
    fn test_object_delete_nonexistent_returns_true() {
        let mut obj = JSObject::new();
        assert!(obj.delete(Atom(999)));
    }

    #[test]
    fn test_object_delete_then_readd() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));
        obj.delete(Atom(1));
        obj.set(Atom(1), JSValue::new_int(20));

        assert_eq!(obj.get(Atom(1)).unwrap().get_int(), 20);
    }

    #[test]
    fn test_object_flags_default() {
        let obj = JSObject::new();
        assert!(obj.extensible());
        assert!(!obj.sealed());
        assert!(!obj.frozen());
        assert!(!obj.is_generator());
    }

    #[test]
    fn test_object_seal() {
        let mut obj = JSObject::new();
        obj.set_sealed(true);
        assert!(obj.sealed());
        obj.set_sealed(false);
        assert!(!obj.sealed());
    }

    #[test]
    fn test_object_freeze() {
        let mut obj = JSObject::new();
        obj.set_frozen(true);
        assert!(obj.frozen());
    }

    #[test]
    fn test_object_not_extensible_rejects_new_props() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));
        obj.set_extensible(false);

        obj.set(Atom(2), JSValue::new_int(20));
        assert!(obj.get(Atom(2)).is_none());

        obj.set(Atom(1), JSValue::new_int(30));
        assert_eq!(obj.get(Atom(1)).unwrap().get_int(), 30);
    }

    #[test]
    fn test_object_types() {
        let ordinary = JSObject::new();
        assert_eq!(ordinary.obj_type(), ObjectType::Ordinary);

        let array = JSObject::new_array();
        assert_eq!(array.obj_type(), ObjectType::Array);

        let func = JSObject::new_function();
        assert_eq!(func.obj_type(), ObjectType::Function);
    }

    #[test]
    fn test_object_keys() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));
        obj.set(Atom(2), JSValue::new_int(20));

        let keys = obj.keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&Atom(1)));
        assert!(keys.contains(&Atom(2)));
    }

    #[test]
    fn test_object_keys_excludes_deleted() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));
        obj.set(Atom(2), JSValue::new_int(20));
        obj.delete(Atom(1));

        let keys = obj.keys();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&Atom(2)));
    }

    #[test]
    fn test_object_own_properties() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));
        obj.set(Atom(2), JSValue::new_int(20));

        let props = obj.own_properties();
        assert_eq!(props.len(), 2);
    }

    #[test]
    fn test_object_has_own() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));
        assert!(obj.has_own(Atom(1)));
        assert!(!obj.has_own(Atom(2)));
    }

    #[test]
    fn test_object_get_own() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(42));
        assert_eq!(obj.get_own(Atom(1)).unwrap().get_int(), 42);
        assert!(obj.get_own(Atom(2)).is_none());
    }

    #[test]
    fn test_object_get_own_descriptor() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(42));

        let desc = obj.get_own_descriptor(Atom(1)).unwrap();
        assert_eq!(desc.value.unwrap().get_int(), 42);
        assert!(desc.writable);
        assert!(desc.enumerable);
        assert!(desc.configurable);
    }

    #[test]
    fn test_object_define_property_data() {
        let mut obj = JSObject::new();
        let desc = PropertyDescriptor::new_data(JSValue::new_int(99));
        let ok = obj.define_property(Atom(1), desc);
        assert!(ok);
        assert_eq!(obj.get(Atom(1)).unwrap().get_int(), 99);
    }

    #[test]
    fn test_object_define_property_accessor() {
        let mut obj = JSObject::new();
        let getter = JSValue::new_int(0);
        let desc = PropertyDescriptor::new_accessor(Some(getter), None);
        let ok = obj.define_property(Atom(1), desc);
        assert!(ok);

        assert!(obj.get(Atom(1)).is_some());
    }

    #[test]
    fn test_object_for_each_property() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));
        obj.set(Atom(2), JSValue::new_int(20));

        let mut collected = Vec::new();
        obj.for_each_property(|atom, value, attrs| {
            collected.push((atom, value.get_int(), attrs));
        });

        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].0, Atom(1));
        assert_eq!(collected[0].1, 10);
        assert_eq!(collected[1].0, Atom(2));
        assert_eq!(collected[1].1, 20);
    }

    #[test]
    fn test_object_for_each_property_skips_deleted() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));
        obj.set(Atom(2), JSValue::new_int(20));
        obj.delete(Atom(1));

        let mut count = 0;
        obj.for_each_property(|_, _, _| {
            count += 1;
        });
        assert_eq!(count, 1);
    }

    #[test]
    fn test_object_for_each_property_attrs_mut() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));

        obj.for_each_property_attrs_mut(|atom, _value, attrs| {
            assert_eq!(atom, Atom(1));
            *attrs = ATTR_DELETED;
        });

        assert!(obj.get(Atom(1)).is_none());
    }

    #[test]
    fn test_object_get_inline_value_at() {
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(42));

        assert_eq!(obj.get_inline_value_at(0).unwrap().get_int(), 42);
        assert!(obj.get_inline_value_at(1).is_none());
    }

    #[test]
    fn test_object_array_elements() {
        let mut obj = JSObject::new_array();
        obj.set_array_elements(vec![
            JSValue::new_int(1),
            JSValue::new_int(2),
            JSValue::new_int(3),
        ]);

        assert_eq!(obj.array_elements_len(), 3);
        assert_eq!(obj.get_indexed(0).unwrap().get_int(), 1);
        assert_eq!(obj.get_indexed(2).unwrap().get_int(), 3);
        assert!(obj.get_indexed(3).is_none());
    }

    #[test]
    fn test_object_set_indexed() {
        let mut obj = JSObject::new_array();
        obj.set_array_elements(vec![JSValue::new_int(0); 5]);
        obj.set_indexed(2, JSValue::new_int(99));

        assert_eq!(obj.get_indexed(2).unwrap().get_int(), 99);
    }

    #[test]
    fn test_object_ensure_elements() {
        let mut obj = JSObject::new_array();
        let elements = obj.ensure_elements();
        elements.push(JSValue::new_int(1));
        assert_eq!(obj.array_elements_len(), 1);
    }

    #[test]
    fn test_object_has_dense_storage() {
        let mut obj = JSObject::new_array();
        assert!(!obj.has_dense_storage());
        obj.set_array_elements(vec![JSValue::new_int(1)]);
        assert!(obj.has_dense_storage());
    }

    #[test]
    fn test_object_for_each_array_element() {
        let mut obj = JSObject::new_array();
        obj.set_array_elements(vec![JSValue::new_int(10), JSValue::new_int(20)]);

        let mut sum = 0i64;
        obj.for_each_array_element(|v| {
            sum += v.get_int();
        });
        assert_eq!(sum, 30);
    }

    #[test]
    fn test_object_bigint_value() {
        let mut obj = JSObject::new();
        assert_eq!(obj.get_bigint_value(), 0);
        obj.set_bigint_value(123456789);
        assert_eq!(obj.get_bigint_value(), 123456789);
    }

    #[test]
    fn test_object_private_fields() {
        let mut obj = JSObject::new();
        assert!(!obj.has_private_field(Atom(1)));
        assert!(obj.get_private_field(Atom(1)).is_none());

        obj.set_private_field(Atom(1), JSValue::new_int(42));
        assert!(obj.has_private_field(Atom(1)));
        assert_eq!(obj.get_private_field(Atom(1)).unwrap().get_int(), 42);
    }

    #[test]
    fn test_object_prototype_chain_get() {
        let mut child_obj = JSObject::new();
        let mut parent_obj = JSObject::new();
        parent_obj.set(Atom(1), JSValue::new_int(42));

        let parent_ptr = Box::into_raw(Box::new(parent_obj)) as *mut JSObject;
        child_obj.prototype = Some(parent_ptr);

        assert_eq!(child_obj.get(Atom(1)).unwrap().get_int(), 42);

        child_obj.set(Atom(1), JSValue::new_int(99));
        assert_eq!(child_obj.get(Atom(1)).unwrap().get_int(), 99);

        unsafe {
            let _ = Box::from_raw(parent_ptr);
        }
    }

    #[test]
    fn test_object_has_property_inherits() {
        let mut child_obj = JSObject::new();
        let mut parent_obj = JSObject::new();
        parent_obj.set(Atom(1), JSValue::new_int(42));

        let parent_ptr = Box::into_raw(Box::new(parent_obj)) as *mut JSObject;
        child_obj.prototype = Some(parent_ptr);

        assert!(child_obj.has_property(Atom(1)));
        assert!(!child_obj.has_own(Atom(1)));

        unsafe {
            let _ = Box::from_raw(parent_ptr);
        }
    }

    #[test]
    fn test_object_property_map_built_after_threshold() {
        let mut obj = JSObject::new();

        for i in 0..=INLINE_THRESHOLD {
            obj.set(Atom(i as u32), JSValue::new_int(i as i64));
        }

        for i in 0..=INLINE_THRESHOLD {
            assert_eq!(obj.get(Atom(i as u32)).unwrap().get_int(), i as i64);
        }
    }

    #[test]
    fn test_object_set_cached_creates_shape() {
        let mut cache = ShapeCache::new();
        let mut obj = JSObject::new();
        obj.set_cached(Atom(1), JSValue::new_int(10), &mut cache);
        obj.set_cached(Atom(2), JSValue::new_int(20), &mut cache);

        assert_eq!(obj.get(Atom(1)).unwrap().get_int(), 10);
        assert_eq!(obj.get(Atom(2)).unwrap().get_int(), 20);
        assert!(obj.shape.is_some());
    }

    #[test]
    fn test_jsobject_size() {
        let size = std::mem::size_of::<JSObject>();

        assert!(
            size <= 192,
            "JSObject size should be <= 192 bytes, got {}",
            size
        );
    }

    #[test]
    fn test_compact_props_removes_deleted_slots() {
        let mut cache = ShapeCache::new();
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));
        obj.set(Atom(2), JSValue::new_int(20));
        obj.set(Atom(3), JSValue::new_int(30));
        obj.delete(Atom(2));

        assert_eq!(obj.inline_values_len(), 3);
        obj.compact_props(&mut cache);
        assert_eq!(obj.inline_values_len(), 2);

        assert_eq!(obj.get(Atom(1)).unwrap().get_int(), 10);
        assert_eq!(obj.get(Atom(3)).unwrap().get_int(), 30);
        assert!(obj.get(Atom(2)).is_none());
    }

    #[test]
    fn test_compact_props_rebuilds_shape_offsets() {
        let mut cache = ShapeCache::new();
        let mut obj = JSObject::new();
        obj.set_cached(Atom(1), JSValue::new_int(10), &mut cache);
        obj.set_cached(Atom(2), JSValue::new_int(20), &mut cache);
        obj.set_cached(Atom(3), JSValue::new_int(30), &mut cache);
        obj.delete(Atom(2));

        let old_shape = obj.get_shape_id().unwrap();
        obj.compact_props(&mut cache);
        let new_shape = obj.get_shape_id().unwrap();

        assert_ne!(old_shape, new_shape);

        let shape = obj.get_shape().unwrap();
        unsafe {
            assert_eq!((*shape.as_ptr()).get_offset(Atom(1)), Some(0));
            assert_eq!((*shape.as_ptr()).get_offset(Atom(3)), Some(1));
            assert_eq!((*shape.as_ptr()).get_offset(Atom(2)), None);
        }
    }

    #[test]
    fn test_compact_props_all_deleted() {
        let mut cache = ShapeCache::new();
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));
        obj.set(Atom(2), JSValue::new_int(20));
        obj.delete(Atom(1));
        obj.delete(Atom(2));

        obj.compact_props(&mut cache);
        assert_eq!(obj.inline_values_len(), 0);
        assert!(obj.get_shape().is_none());
        assert!(obj.get(Atom(1)).is_none());
        assert!(obj.get(Atom(2)).is_none());
    }

    #[test]
    fn test_compact_props_no_deleted_early_return() {
        let mut cache = ShapeCache::new();
        let mut obj = JSObject::new();
        obj.set(Atom(1), JSValue::new_int(10));
        obj.set(Atom(2), JSValue::new_int(20));

        let shape_before = obj.get_shape_id();
        obj.compact_props(&mut cache);
        let shape_after = obj.get_shape_id();

        assert_eq!(shape_before, shape_after);
        assert_eq!(obj.inline_values_len(), 2);
    }
}
