use crate::runtime::atom::Atom;

const QNAN_BASE: u64 = 0x7FF8_0000_0000_0000;

const TAG_UNDEFINED: u64 = 0x0;
const TAG_NULL: u64 = 0x1;
const TAG_BOOL: u64 = 0x2;
const TAG_INT: u64 = 0x3;
const TAG_STRING: u64 = 0x4;
const TAG_OBJECT: u64 = 0x5;
const TAG_SYMBOL: u64 = 0x6;
const TAG_BIGINT: u64 = 0x7;
const TAG_FUNC: u64 = 0x8;
const TAG_TDZ: u64 = 0x9;

const TAG_SHIFT: u64 = 47;
const PAYLOAD_MASK: u64 = 0x0000_7FFF_FFFF_FFFF;

const CANONICAL_NAN: u64 = QNAN_BASE | (0xF << TAG_SHIFT) | 0x1;

#[derive(Clone, Copy, Debug)]
pub struct JSValue(u64);

impl JSValue {
    pub fn raw_bits(&self) -> u64 {
        self.0
    }

    pub fn from_raw_bits(bits: u64) -> Self {
        JSValue(bits)
    }

    const fn is_tagged(bits: u64) -> bool {
        ((bits >> 48) & 0x7FF8) == 0x7FF8
    }

    const fn pack(tag: u64, payload: u64) -> u64 {
        QNAN_BASE | (tag << TAG_SHIFT) | (payload & PAYLOAD_MASK)
    }

    #[inline(always)]
    fn get_tag(&self) -> u64 {
        (self.0 >> TAG_SHIFT) & 0xF
    }

    #[inline(always)]
    fn get_payload(&self) -> u64 {
        self.0 & PAYLOAD_MASK
    }

    #[inline(always)]
    pub fn new_int(i: i64) -> Self {
        let sign = ((i as u64) >> 63) << 63;
        let payload = (i as u64) & PAYLOAD_MASK;
        JSValue(Self::pack(TAG_INT, payload) | sign)
    }

    #[inline]
    fn is_fast_int(f: f64) -> Option<i64> {
        let bits = f.to_bits();
        let exponent = ((bits >> 52) & 0x7FF) as i32;
        if exponent == 0 {
            if (bits & 0x000F_FFFF_FFFF_FFFF) == 0 {
                if bits & 0x8000_0000_0000_0000 != 0 {
                    return None;
                }
                return Some(0i64);
            }
            return None;
        }
        if exponent >= 0x7FF {
            return None;
        }

        if exponent < 1023 {
            return None;
        }

        let exp_shift = (exponent - 1023) as u32;
        if exp_shift >= 52 {
        } else {
            let frac_bits = 52 - exp_shift;
            let frac_mask = (1u64 << frac_bits) - 1;
            if (bits & frac_mask) != 0 {
                return None;
            }
        }

        let int_val = f as i64;
        const MAX_SAFE_INT: i64 = 1i64 << 47;
        if int_val >= -MAX_SAFE_INT && int_val < MAX_SAFE_INT {
            Some(int_val)
        } else {
            None
        }
    }

    pub fn new_float(f: f64) -> Self {
        if let Some(int_val) = Self::is_fast_int(f) {
            return JSValue::new_int(int_val);
        }

        let bits = f.to_bits();
        if !Self::is_tagged(bits) {
            return JSValue(bits);
        }

        JSValue(CANONICAL_NAN)
    }

    #[inline(always)]
    pub fn is_float(&self) -> bool {
        !Self::is_tagged(self.0) || self.0 == CANONICAL_NAN
    }

    #[inline(always)]
    pub fn new_string(atom: Atom) -> Self {
        JSValue(Self::pack(TAG_STRING, atom.0 as u64))
    }

    #[inline(always)]
    pub fn new_object(ptr: usize) -> Self {
        JSValue(Self::pack(TAG_OBJECT, Self::compress_ptr(ptr)))
    }

    #[inline(always)]
    pub fn new_function(ptr: usize) -> Self {
        JSValue(Self::pack(TAG_FUNC, Self::compress_ptr(ptr)))
    }

    pub fn new_symbol(atom: Atom) -> Self {
        JSValue(Self::pack(TAG_SYMBOL, atom.0 as u64))
    }

    pub fn new_symbol_with_id(atom: Atom, id: u32) -> Self {
        JSValue(Self::pack(TAG_SYMBOL, (id as u64) << 32 | atom.0 as u64))
    }

    pub fn get_symbol_id(&self) -> u32 {
        debug_assert!(self.is_symbol());
        (self.get_payload() >> 32) as u32
    }

    pub fn new_bigint(ptr: usize) -> Self {
        JSValue(Self::pack(TAG_BIGINT, Self::compress_ptr(ptr)))
    }

    #[inline(always)]
    fn compress_ptr(ptr: usize) -> u64 {
        let compressed = (ptr >> 3) as u64;
        compressed & PAYLOAD_MASK
    }

    #[inline(always)]
    fn decompress_ptr(compressed: u64) -> usize {
        (compressed << 3) as usize
    }

    pub const fn undefined() -> Self {
        JSValue(Self::pack(TAG_UNDEFINED, 0))
    }

    pub const fn null() -> Self {
        JSValue(Self::pack(TAG_NULL, 1))
    }

    pub const fn bool(b: bool) -> Self {
        JSValue(Self::pack(TAG_BOOL, if b { 1 } else { 0 }))
    }

    #[inline(always)]
    pub fn is_undefined(&self) -> bool {
        Self::is_tagged(self.0) && self.get_tag() == TAG_UNDEFINED
    }

    #[inline(always)]
    pub fn is_null(&self) -> bool {
        Self::is_tagged(self.0) && self.get_tag() == TAG_NULL
    }

    #[inline(always)]
    pub fn is_null_or_undefined(&self) -> bool {
        Self::is_tagged(self.0) && self.get_tag() <= TAG_NULL
    }

    #[inline(always)]
    pub fn is_bool(&self) -> bool {
        Self::is_tagged(self.0) && self.get_tag() == TAG_BOOL
    }

    #[inline(always)]
    pub fn is_int(&self) -> bool {
        Self::is_tagged(self.0) && self.get_tag() == TAG_INT
    }

    #[inline(always)]
    pub fn both_int(a: &JSValue, b: &JSValue) -> bool {
        const INT_TAG_BITS: u64 = QNAN_BASE | (TAG_INT << TAG_SHIFT);
        const TAG_MASK: u64 = QNAN_BASE | (0xF << TAG_SHIFT);
        (a.0 & TAG_MASK) == INT_TAG_BITS && (b.0 & TAG_MASK) == INT_TAG_BITS
    }

    #[inline(always)]
    pub fn both_object(a: &JSValue, b: &JSValue) -> bool {
        const OBJ_TAG_BITS: u64 = QNAN_BASE | (TAG_OBJECT << TAG_SHIFT);
        const TAG_MASK: u64 = QNAN_BASE | (0xF << TAG_SHIFT);
        (a.0 & TAG_MASK) == OBJ_TAG_BITS && (b.0 & TAG_MASK) == OBJ_TAG_BITS
    }

    #[inline(always)]
    pub fn both_raw_float(a: &JSValue, b: &JSValue) -> bool {
        !Self::is_tagged(a.0) && !Self::is_tagged(b.0)
    }

    #[inline(always)]
    pub fn new_float_raw(f: f64) -> Self {
        let bits = f.to_bits();
        if !Self::is_tagged(bits) {
            JSValue(bits)
        } else {
            JSValue(CANONICAL_NAN)
        }
    }

    #[inline(always)]
    pub fn is_string(&self) -> bool {
        Self::is_tagged(self.0) && self.get_tag() == TAG_STRING
    }

    #[inline(always)]
    pub fn is_object(&self) -> bool {
        Self::is_tagged(self.0) && self.get_tag() == TAG_OBJECT
    }

    #[inline(always)]
    pub fn is_function(&self) -> bool {
        Self::is_tagged(self.0) && self.get_tag() == TAG_FUNC
    }

    #[inline(always)]
    pub fn is_symbol(&self) -> bool {
        Self::is_tagged(self.0) && self.get_tag() == TAG_SYMBOL
    }

    #[inline(always)]
    pub fn is_bigint(&self) -> bool {
        Self::is_tagged(self.0) && self.get_tag() == TAG_BIGINT
    }

    #[inline(always)]
    pub fn is_object_like(&self) -> bool {
        Self::is_tagged(self.0) && matches!(self.get_tag(), TAG_OBJECT | TAG_FUNC)
    }

    pub const fn new_tdz() -> Self {
        JSValue(Self::pack(TAG_TDZ, 0))
    }

    #[inline(always)]
    pub fn is_tdz(&self) -> bool {
        Self::is_tagged(self.0) && self.get_tag() == TAG_TDZ
    }

    #[inline(always)]
    pub fn get_int(&self) -> i64 {
        let payload = self.get_payload();
        let sign = (self.0 >> 63) as i64;
        let extend = (-sign as u64) & !PAYLOAD_MASK;
        (payload | extend) as i64
    }

    pub fn to_number(&self) -> f64 {
        if !Self::is_tagged(self.0) {
            return f64::from_bits(self.0);
        }
        if self.get_tag() == TAG_INT {
            return self.get_int() as f64;
        }
        if self.get_tag() == TAG_BOOL {
            return if self.get_bool() { 1.0 } else { 0.0 };
        }
        if self.get_tag() == TAG_NULL {
            return 0.0;
        }
        f64::NAN
    }

    pub fn get_float(&self) -> f64 {
        if !Self::is_tagged(self.0) {
            return f64::from_bits(self.0);
        }
        if self.0 == CANONICAL_NAN {
            return f64::NAN;
        }
        if self.get_tag() == TAG_INT {
            return self.get_int() as f64;
        }
        if self.get_tag() == TAG_NULL {
            return 0.0;
        }
        f64::NAN
    }

    #[inline(always)]
    pub fn get_atom(&self) -> Atom {
        Atom(self.get_payload() as u32)
    }

    #[inline(always)]
    pub fn get_ptr(&self) -> usize {
        Self::decompress_ptr(self.get_payload())
    }

    #[inline(always)]
    pub unsafe fn object_from_ptr(ptr: usize) -> &'static crate::object::object::JSObject {
        unsafe { &*(ptr as *const crate::object::object::JSObject) }
    }

    #[inline(always)]
    pub unsafe fn object_from_ptr_mut(ptr: usize) -> &'static mut crate::object::object::JSObject {
        unsafe { &mut *(ptr as *mut crate::object::object::JSObject) }
    }

    #[inline(always)]
    pub unsafe fn function_from_ptr(ptr: usize) -> &'static crate::object::function::JSFunction {
        unsafe { &*(ptr as *const crate::object::function::JSFunction) }
    }

    #[inline(always)]
    pub unsafe fn function_from_ptr_mut(
        ptr: usize,
    ) -> &'static mut crate::object::function::JSFunction {
        unsafe { &mut *(ptr as *mut crate::object::function::JSFunction) }
    }

    #[inline(always)]
    pub fn as_object(&self) -> &crate::object::object::JSObject {
        unsafe { &*(self.get_ptr() as *const crate::object::object::JSObject) }
    }

    #[inline(always)]
    pub fn as_object_mut(&self) -> &mut crate::object::object::JSObject {
        unsafe { &mut *(self.get_ptr() as *mut crate::object::object::JSObject) }
    }

    #[inline(always)]
    pub fn as_function(&self) -> &crate::object::function::JSFunction {
        unsafe { &*(self.get_ptr() as *const crate::object::function::JSFunction) }
    }

    #[inline(always)]
    pub fn as_function_mut(&self) -> &mut crate::object::function::JSFunction {
        unsafe { &mut *(self.get_ptr() as *mut crate::object::function::JSFunction) }
    }

    #[inline(always)]
    pub fn get_bool(&self) -> bool {
        self.get_payload() != 0
    }

    pub fn is_truthy(&self) -> bool {
        if !Self::is_tagged(self.0) {
            let f = f64::from_bits(self.0);
            return f != 0.0 && !f.is_nan();
        }
        if self.is_float() {
            let f = self.get_float();
            return f != 0.0 && !f.is_nan();
        }
        match self.get_tag() {
            TAG_UNDEFINED | TAG_NULL => false,
            TAG_BOOL => self.get_bool(),
            TAG_INT => self.get_int() != 0,
            TAG_STRING => self.get_payload() != 0,
            TAG_OBJECT | TAG_FUNC | TAG_SYMBOL | TAG_BIGINT => true,
            TAG_TDZ => false,
            _ => true,
        }
    }

    #[inline(always)]
    pub fn strict_eq(&self, other: &JSValue) -> bool {
        if self.0 == other.0 {
            if self.is_float() {
                return !self.get_float().is_nan();
            }
            return true;
        }

        if self.is_bigint() && other.is_bigint() {
            let obj_a = self.as_object();
            let obj_b = other.as_object();
            return obj_a.get_bigint_value() == obj_b.get_bigint_value();
        }

        if self.is_int() && other.is_float() {
            let fv = other.get_float();
            return !fv.is_nan() && (self.get_int() as f64) == fv;
        }
        if self.is_float() && other.is_int() {
            let fv = self.get_float();
            return !fv.is_nan() && fv == (other.get_int() as f64);
        }

        if self.is_float() && other.is_float() {
            let a = self.get_float();
            let b = other.get_float();
            return !a.is_nan() && !b.is_nan() && a == b;
        }

        if self.is_string() && other.is_string() {
            return self.get_atom().0 == other.get_atom().0;
        }

        false
    }

    pub fn debug_raw(&self) -> u64 {
        self.0
    }

    pub fn get_data(&self) -> u64 {
        self.get_payload()
    }

    #[inline]
    pub fn retain_atoms_in(&self, ctx: &mut crate::runtime::JSContext) {
        if self.is_string() || self.is_symbol() {
            ctx.atom_table_mut().retain(self.get_atom());
        }
    }

    #[inline]
    pub fn release_atoms_in(&self, ctx: &mut crate::runtime::JSContext) {
        if self.is_string() || self.is_symbol() {
            ctx.atom_table_mut().release(self.get_atom());
        }
    }

    #[inline]
    pub fn release_atoms_in_table(&self, table: &mut crate::runtime::atom::AtomTable) {
        if self.is_string() || self.is_symbol() {
            table.release(self.get_atom());
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JSValueType {
    Undefined,
    Null,
    Boolean,
    Number,
    BigInt,
    Symbol,
    String,
    Object,
    Function,
}

impl JSValue {
    pub fn get_type(&self) -> JSValueType {
        if !Self::is_tagged(self.0) {
            return JSValueType::Number;
        }
        match self.get_tag() {
            TAG_UNDEFINED => JSValueType::Undefined,
            TAG_NULL => JSValueType::Null,
            TAG_BOOL => JSValueType::Boolean,
            TAG_INT => JSValueType::Number,
            TAG_BIGINT => JSValueType::BigInt,
            TAG_SYMBOL => JSValueType::Symbol,
            TAG_STRING => JSValueType::String,
            TAG_OBJECT => JSValueType::Object,
            TAG_FUNC => JSValueType::Function,
            TAG_TDZ => JSValueType::Undefined,
            _ => JSValueType::Undefined,
        }
    }
}

pub trait IntoJSValue {
    fn into_jsvalue(self, ctx: &mut crate::runtime::JSContext) -> JSValue;
}

pub trait FromJSValue: Sized {
    fn from_jsvalue(value: &JSValue, ctx: &crate::runtime::JSContext) -> Option<Self>
    where
        Self: Sized;
}

impl IntoJSValue for () {
    fn into_jsvalue(self, _ctx: &mut crate::runtime::JSContext) -> JSValue {
        JSValue::undefined()
    }
}

impl IntoJSValue for bool {
    fn into_jsvalue(self, _ctx: &mut crate::runtime::JSContext) -> JSValue {
        JSValue::bool(self)
    }
}

impl IntoJSValue for i32 {
    fn into_jsvalue(self, _ctx: &mut crate::runtime::JSContext) -> JSValue {
        JSValue::new_int(self as i64)
    }
}

impl IntoJSValue for i64 {
    fn into_jsvalue(self, _ctx: &mut crate::runtime::JSContext) -> JSValue {
        JSValue::new_int(self)
    }
}

impl IntoJSValue for f64 {
    fn into_jsvalue(self, _ctx: &mut crate::runtime::JSContext) -> JSValue {
        JSValue::new_float(self)
    }
}

impl IntoJSValue for String {
    fn into_jsvalue(self, ctx: &mut crate::runtime::JSContext) -> JSValue {
        let atom = ctx.atom_table_mut().intern(&self);
        JSValue::new_string(atom)
    }
}

impl IntoJSValue for &str {
    fn into_jsvalue(self, ctx: &mut crate::runtime::JSContext) -> JSValue {
        let atom = ctx.atom_table_mut().intern(self);
        JSValue::new_string(atom)
    }
}

impl<T: IntoJSValue> IntoJSValue for Option<T> {
    fn into_jsvalue(self, ctx: &mut crate::runtime::JSContext) -> JSValue {
        match self {
            Some(v) => v.into_jsvalue(ctx),
            None => JSValue::null(),
        }
    }
}
