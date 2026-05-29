use crate::compiler::location::LineNumberTable;
use crate::host::func::HostFunc;
use crate::object::object::JSObject;
use crate::runtime::atom::Atom;
use crate::runtime::context::JSContext;
use crate::util::FxHashMap;
use crate::value::JSValue;
use std::cell::Cell;
use std::rc::Rc;

const FLAG_IS_ARROW: u8 = 1 << 0;
const FLAG_IS_ASYNC: u8 = 1 << 1;
const FLAG_IS_GENERATOR: u8 = 1 << 2;
const FLAG_IS_BUILTIN: u8 = 1 << 3;
const FLAG_USES_ARGUMENTS: u8 = 1 << 4;
const FLAG_IS_STRICT: u8 = 1 << 5;

const FLAG_HAS_SYMBOL_ON_BASE: u8 = 1 << 6;

#[derive(Debug, Clone, Default)]
pub struct UpvalueData {
    pub upvalues: FxHashMap<Atom, JSValue>,
    pub upvalue_cells: FxHashMap<Atom, Rc<Cell<JSValue>>>,
    pub upvalue_local_indices: FxHashMap<Atom, usize>,
    pub upvalue_frame_overrides: FxHashMap<Atom, (usize, u64)>,

    pub upvalue_slots: Vec<Rc<Cell<JSValue>>>,

    pub upvalue_slot_atoms: Vec<Atom>,

    pub env_frame: Option<usize>,

    pub env_frame_id: Option<u64>,
}

impl UpvalueData {
    pub fn new() -> Self {
        Self::default()
    }
}

#[repr(C)]
pub struct JSFunction {
    pub base: JSObject,
    pub name: Atom,
    pub param_count: u32,
    pub locals_count: u32,
    flags: u8,
    pub arity: u32,

    pub builtin_atom: Option<Atom>,

    pub builtin_func: Option<HostFunc>,
    pub upvalues: Option<Box<UpvalueData>>,

    pub bytecode: Option<Box<crate::compiler::opcode::Bytecode>>,

    pub shared_nb_for_ic: Option<std::sync::Arc<crate::compiler::opcode::NestedBytecode>>,

    pub cached_prototype_ptr: *mut crate::object::object::JSObject,

    pub source_filename: String,
    pub line_number_table: Option<LineNumberTable>,
    pub source_text: Option<String>,
}

impl JSFunction {
    pub fn new() -> Self {
        JSFunction {
            base: JSObject::new_function(),
            name: Atom(0),
            param_count: 0,
            locals_count: 256,
            flags: 0,
            arity: 0,
            builtin_atom: None,
            builtin_func: None,
            upvalues: None,
            bytecode: None,
            shared_nb_for_ic: None,
            cached_prototype_ptr: std::ptr::null_mut(),
            source_filename: "<anonymous>".to_string(),
            line_number_table: None,
            source_text: None,
        }
    }

    pub fn new_builtin(name: Atom, arity: u32) -> Self {
        let mut f = JSFunction::new();
        f.name = name;
        f.flags = FLAG_IS_BUILTIN;
        f.arity = arity;
        f
    }

    pub fn with_upvalues(mut self, upvalues_map: FxHashMap<Atom, JSValue>) -> Self {
        let mut data = UpvalueData::new();
        data.upvalues = upvalues_map;
        self.upvalues = Some(Box::new(data));
        self
    }

    pub fn is_callable(&self) -> bool {
        true
    }

    #[inline(always)]
    pub fn is_arrow(&self) -> bool {
        self.flags & FLAG_IS_ARROW != 0
    }
    #[inline(always)]
    pub fn set_is_arrow(&mut self, val: bool) {
        if val {
            self.flags |= FLAG_IS_ARROW;
        } else {
            self.flags &= !FLAG_IS_ARROW;
        }
    }

    #[inline(always)]
    pub fn is_async(&self) -> bool {
        self.flags & FLAG_IS_ASYNC != 0
    }
    #[inline(always)]
    pub fn set_is_async(&mut self, val: bool) {
        if val {
            self.flags |= FLAG_IS_ASYNC;
        } else {
            self.flags &= !FLAG_IS_ASYNC;
        }
    }

    #[inline(always)]
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
    pub fn is_builtin(&self) -> bool {
        self.flags & FLAG_IS_BUILTIN != 0
    }
    #[inline(always)]
    pub fn set_is_builtin(&mut self, val: bool) {
        if val {
            self.flags |= FLAG_IS_BUILTIN;
        } else {
            self.flags &= !FLAG_IS_BUILTIN;
        }
    }

    #[inline(always)]
    pub fn uses_arguments(&self) -> bool {
        self.flags & FLAG_USES_ARGUMENTS != 0
    }
    #[inline(always)]
    pub fn set_uses_arguments(&mut self, val: bool) {
        if val {
            self.flags |= FLAG_USES_ARGUMENTS;
        } else {
            self.flags &= !FLAG_USES_ARGUMENTS;
        }
    }

    #[inline(always)]
    pub fn is_strict(&self) -> bool {
        self.flags & FLAG_IS_STRICT != 0
    }
    #[inline(always)]
    pub fn set_is_strict(&mut self, val: bool) {
        if val {
            self.flags |= FLAG_IS_STRICT;
        } else {
            self.flags &= !FLAG_IS_STRICT;
        }
    }

    #[inline(always)]
    pub fn has_symbol_on_base(&self) -> bool {
        self.flags & FLAG_HAS_SYMBOL_ON_BASE != 0
    }

    #[inline(always)]
    pub fn mark_has_symbol_prop(&mut self) {
        self.flags |= FLAG_HAS_SYMBOL_ON_BASE;
    }

    #[inline]
    pub fn upvalues_mut(&mut self) -> &mut UpvalueData {
        if self.upvalues.is_none() {
            self.upvalues = Some(Box::new(UpvalueData::new()));
        }
        self.upvalues.as_mut().unwrap()
    }

    #[inline]
    pub fn upvalues_ref(&self) -> Option<&UpvalueData> {
        self.upvalues.as_deref()
    }

    pub fn set_builtin_marker(&mut self, ctx: &mut JSContext, builtin_name: &str) {
        use crate::object::object::PropertyDescriptor;

        const BUILTIN_MARKER: &str = "__builtin__";
        let atom = ctx.intern(builtin_name);
        self.builtin_atom = Some(atom);

        self.builtin_func = ctx.get_builtin_func(builtin_name);
        if let Some(display_name) = ctx.get_builtin_name(builtin_name) {
            self.name = ctx.intern(display_name);
        }
        self.base
            .set(ctx.intern(BUILTIN_MARKER), JSValue::new_string(atom));

        let length_desc = PropertyDescriptor {
            value: Some(JSValue::new_int(self.arity as i64)),
            writable: false,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        };
        self.base.define_property(ctx.common_atoms.length, length_desc);

        let name_desc = PropertyDescriptor {
            value: Some(JSValue::new_string(self.name)),
            writable: false,
            enumerable: false,
            configurable: true,
            get: None,
            set: None,
        };
        self.base.define_property(ctx.common_atoms.name, name_desc);
    }
}

impl Default for JSFunction {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsfunction_flag_accessors() {
        let mut f = JSFunction::new();
        assert!(!f.is_arrow());
        assert!(!f.is_async());
        assert!(!f.is_generator());
        assert!(!f.is_builtin());

        f.set_is_arrow(true);
        assert!(f.is_arrow());

        f.set_is_async(true);
        assert!(f.is_async());

        f.set_is_generator(true);
        assert!(f.is_generator());

        f.set_is_builtin(true);
        assert!(f.is_builtin());

        f.set_is_arrow(false);
        assert!(!f.is_arrow());

        assert!(f.is_async());
        assert!(f.is_generator());
        assert!(f.is_builtin());
    }

    #[test]
    fn test_jsfunction_builtin_constructor() {
        let f = JSFunction::new_builtin(Atom(42), 3);
        assert!(f.is_builtin());
        assert_eq!(f.arity, 3);
        assert_eq!(f.name, Atom(42));
    }

    #[test]
    fn test_upvalue_data_lazily_allocated() {
        let mut f = JSFunction::new();
        assert!(f.upvalues.is_none());

        assert!(f.upvalues_ref().is_none());
        assert!(f.upvalues.is_none());

        f.upvalues_mut()
            .upvalues
            .insert(Atom(1), JSValue::new_int(42));
        assert!(f.upvalues.is_some());
        assert_eq!(
            f.upvalues_ref()
                .unwrap()
                .upvalues
                .get(&Atom(1))
                .unwrap()
                .get_int(),
            42
        );
    }

    #[test]
    fn test_upvalue_data_slots_and_atoms() {
        let mut f = JSFunction::new();
        let cell = Rc::new(Cell::new(JSValue::new_int(99)));
        f.upvalues_mut().upvalue_slots.push(cell.clone());
        f.upvalues_mut().upvalue_slot_atoms.push(Atom(7));

        let uv = f.upvalues_ref().unwrap();
        assert_eq!(uv.upvalue_slots.len(), 1);
        assert_eq!(uv.upvalue_slot_atoms[0], Atom(7));
        assert_eq!(uv.upvalue_slots[0].get().get_int(), 99);
    }

    #[test]
    fn test_with_upvalues() {
        let mut map = FxHashMap::default();
        map.insert(Atom(1), JSValue::new_int(10));
        let f = JSFunction::new().with_upvalues(map);
        assert!(f.upvalues.is_some());
        assert_eq!(
            f.upvalues_ref()
                .unwrap()
                .upvalues
                .get(&Atom(1))
                .unwrap()
                .get_int(),
            10
        );
    }
}
