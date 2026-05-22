use crate::compiler::opcode::{Bytecode, Opcode};
use crate::runtime::context::JSContext;
#[cfg(test)]
use crate::runtime::runtime::JSRuntime;
use crate::util::FxHashMap;
use crate::value::JSValue;

const GC_CHECK_INTERVAL: u64 = 16384;
const INTERRUPT_POLL_EVERY_MASK: usize = 0xFFF;

const PRIM_STRING_SHAPE_ID: usize = usize::MAX - 10;
const PRIM_NUMBER_SHAPE_ID: usize = usize::MAX - 11;

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub return_pc: usize,
    pub registers_base: usize,
    pub registers_count: usize,
    pub locals_count: u32,
    pub bytecode_ptr: *const u8,
    pub bytecode_len: usize,
    pub constants_ptr: *const JSValue,
    pub constants_len: usize,
    pub function_ptr: Option<usize>,
    pub ic_table_ptr: *mut crate::compiler::InlineCacheTable,
    pub this_value: JSValue,
    pub saved_args: Vec<JSValue>,
    pub upvalue_sync_map: Option<Box<FxHashMap<u16, std::rc::Rc<std::cell::Cell<JSValue>>>>>,

    pub upvalue_sync_bitset: u64,
    pub dst_reg: u16,
    pub arg_count: u16,
    pub super_ctor: JSValue,
    pub is_constructor: bool,
    pub is_async: bool,
    pub uses_arguments: bool,
    pub current_pc: usize,
    pub is_strict_frame: bool,

    pub has_upvalues: bool,

    pub var_name_map: *const Vec<(u32, u16)>,
    pub eval_bindings: Option<Box<std::collections::HashMap<u32, JSValue>>>,
    pub cached_arguments: Option<usize>,
}

impl CallFrame {
    pub fn new() -> Self {
        Self {
            return_pc: 0,
            registers_base: 0,
            registers_count: 0,
            locals_count: 0,
            bytecode_ptr: std::ptr::null(),
            bytecode_len: 0,
            constants_ptr: std::ptr::null(),
            constants_len: 0,
            function_ptr: None,
            ic_table_ptr: std::ptr::null_mut(),
            this_value: JSValue::undefined(),
            saved_args: Vec::new(),
            upvalue_sync_map: None,
            upvalue_sync_bitset: 0,
            dst_reg: 0,
            arg_count: 0,
            super_ctor: JSValue::undefined(),
            is_constructor: false,
            is_async: false,
            uses_arguments: false,
            current_pc: 0,
            is_strict_frame: false,
            has_upvalues: false,
            var_name_map: std::ptr::null(),
            eval_bindings: None,
            cached_arguments: None,
        }
    }
}

#[derive(Clone)]
pub struct ExceptionHandler {
    pub frame_index: usize,
    pub catch_pc: usize,
    pub finally_pc: Option<usize>,
}

#[derive(Debug)]
pub enum ExecutionOutcome {
    Complete(JSValue),
    Yield(JSValue),
}

enum ThrowDispatch {
    Caught,

    Uncaught(String),

    AsyncComplete(ExecutionOutcome),
}

pub struct VM {
    pub registers: Vec<JSValue>,
    pub frames: Vec<CallFrame>,
    pub frame_index: usize,
    pub pc: usize,

    cached_code_ptr: *const u8,
    cached_code_len: usize,
    cached_const_ptr: *const JSValue,
    cached_registers_base: usize,
    cached_registers_ptr: *mut JSValue,
    cached_has_upvalue_sync: bool,

    cached_upvalue_sync_bitset: u64,

    cached_ic_table_ptr: *mut crate::compiler::InlineCacheTable,

    cached_upvalue_slot_ptr: *const std::rc::Rc<std::cell::Cell<JSValue>>,
    cached_upvalues_len: usize,

    exception_handlers: Vec<ExceptionHandler>,

    pending_throw: Option<JSValue>,

    finally_rethrow: Option<JSValue>,

    ctx_ptr: *mut crate::runtime::JSContext,

    allocation_count: u64,

    caller_vm: Option<usize>,

    eval_binding_frames: u32,

    cached_has_instance_atom: crate::runtime::atom::Atom,

    regex_lit_cache: std::collections::HashMap<usize, crate::regexp::Regex>,

    gc_roots: Vec<JSValue>,
}

impl VM {
    fn builtin_needs_this_for_call_with_this(name: &str) -> bool {
        matches!(
            name,
            "function_bind"
                | "function_call"
                | "function_apply"
                | "function_toString"
                | "function_length"
                | "function_name"
                | "function_has_instance"
                | "object_hasOwnProperty"
                | "object_valueOf"
                | "object_toString"
                | "object_isPrototypeOf"
                | "object_property_is_enumerable"
                | "object_to_locale_string"
                | "intl_collator_compare_getter"
                | "intl_collator_resolved_options"
                | "intl_numberformat_resolved_options"
                | "intl_datetimeformat_resolved_options"
                | "date_toPrimitive"
        )
    }

    fn builtin_needs_callee(name: &str) -> bool {
        matches!(name, "intl_collator_compare_call")
    }

    pub fn new() -> Self {
        let mut vm = Self {
            registers: Vec::with_capacity(4096),
            frames: vec![CallFrame::new(); 64],
            frame_index: 0,
            pc: 0,
            cached_code_ptr: std::ptr::null(),
            cached_code_len: 0,
            cached_const_ptr: std::ptr::null(),
            cached_registers_base: 0,
            cached_registers_ptr: std::ptr::null_mut(),
            cached_has_upvalue_sync: false,
            cached_upvalue_sync_bitset: 0,
            cached_ic_table_ptr: std::ptr::null_mut(),
            cached_upvalue_slot_ptr: std::ptr::null(),
            cached_upvalues_len: 0,
            exception_handlers: Vec::new(),
            pending_throw: None,
            finally_rethrow: None,
            ctx_ptr: std::ptr::null_mut(),
            allocation_count: 0,
            caller_vm: None,
            eval_binding_frames: 0,
            cached_has_instance_atom: crate::runtime::atom::Atom(0),
            regex_lit_cache: std::collections::HashMap::new(),
            gc_roots: Vec::with_capacity(512),
        };
        vm.frames[0] = CallFrame::new();
        vm
    }

    fn throw_reference_error(&mut self, ctx: &mut JSContext, msg: &str) -> Option<JSValue> {
        let mut err = crate::object::object::JSObject::new();
        err.set(
            ctx.intern("name"),
            JSValue::new_string(ctx.intern("ReferenceError")),
        );
        err.set(ctx.intern("message"), JSValue::new_string(ctx.intern(msg)));
        if let Some(proto) = ctx.get_reference_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        self.pending_throw = Some(JSValue::new_object(ptr));
        None
    }

    fn create_iter_object(&mut self, ctx: &mut JSContext, iterable: JSValue) -> JSValue {
        let mut iter_obj = crate::object::object::JSObject::new();
        let arr_atom = ctx.common_atoms.__iter_arr__;
        let idx_atom = ctx.common_atoms.__iter_idx__;
        iter_obj.set_cached(arr_atom, iterable, ctx.shape_cache_mut());
        iter_obj.set_cached(idx_atom, JSValue::new_int(0), ctx.shape_cache_mut());
        let iter_ptr = Box::into_raw(Box::new(iter_obj)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(iter_ptr);
        self.allocation_count += 1;
        JSValue::new_object(iter_ptr)
    }

    #[inline(always)]
    fn refresh_cache(&mut self) {
        let frame = &self.frames[self.frame_index];
        self.cached_code_ptr = frame.bytecode_ptr;
        self.cached_code_len = frame.bytecode_len;
        self.cached_const_ptr = frame.constants_ptr;
        self.cached_registers_base = frame.registers_base;
        self.cached_has_upvalue_sync = frame.upvalue_sync_map.is_some();
        self.cached_upvalue_sync_bitset = frame.upvalue_sync_bitset;
        self.cached_registers_ptr =
            unsafe { self.registers.as_mut_ptr().add(self.cached_registers_base) };
        self.cached_ic_table_ptr = frame.ic_table_ptr;

        if frame.has_upvalues {
            if let Some(func_ptr) = frame.function_ptr {
                let func_val = JSValue::new_function(func_ptr);
                let func = func_val.as_function();
                if let Some(uv) = func.upvalues.as_ref() {
                    let slots = &uv.upvalue_slots;
                    self.cached_upvalue_slot_ptr = slots.as_ptr();
                    self.cached_upvalues_len = slots.len();
                } else {
                    self.cached_upvalue_slot_ptr = std::ptr::null();
                    self.cached_upvalues_len = 0;
                }
            } else {
                self.cached_upvalue_slot_ptr = std::ptr::null();
                self.cached_upvalues_len = 0;
            }
        } else {
            self.cached_upvalue_slot_ptr = std::ptr::null();
            self.cached_upvalues_len = 0;
        }
    }

    fn push_frame(
        &mut self,
        bytecode: &Bytecode,
        return_pc: usize,
        function_ptr: Option<usize>,
        this_value: JSValue,
        dst_reg: u16,
        arg_count: u16,
        is_constructor: bool,
        is_async: bool,
        args: &[JSValue],
        save_args: bool,
    ) {
        self.push_frame_raw(
            bytecode.locals_count,
            bytecode.effective_code_ptr(),
            bytecode.effective_code_len(),
            bytecode.effective_const_ptr(),
            bytecode.effective_const_len(),
            return_pc,
            function_ptr,
            this_value,
            dst_reg,
            arg_count,
            is_constructor,
            is_async,
            args,
            save_args,
        );
    }

    fn push_frame_raw(
        &mut self,
        locals_count: u32,
        bytecode_ptr: *const u8,
        bytecode_len: usize,
        constants_ptr: *const JSValue,
        constants_len: usize,
        return_pc: usize,
        function_ptr: Option<usize>,
        this_value: JSValue,
        dst_reg: u16,
        arg_count: u16,
        is_constructor: bool,
        is_async: bool,
        args: &[JSValue],
        save_args: bool,
    ) {
        let needed = (locals_count as usize).max(arg_count as usize + 1);
        let prev = &self.frames[self.frame_index];
        let base = prev.registers_base + prev.registers_count;
        let total = base + needed;
        if total > self.registers.len() {
            if total > self.registers.capacity() {
                let new_cap = (self.registers.capacity() * 2 + 64).max(total);
                self.registers.reserve(new_cap - self.registers.len());
            }

            unsafe {
                self.registers.set_len(total);
            }
            self.cached_registers_ptr =
                unsafe { self.registers.as_mut_ptr().add(self.cached_registers_base) };
        }

        unsafe {
            let ptr = self.registers.as_mut_ptr().add(base);
            ptr.write(this_value);
            for (i, arg) in args.iter().enumerate() {
                ptr.add(i + 1).write(*arg);
            }
            let undef = JSValue::undefined();
            for i in (args.len() + 1)..needed {
                ptr.add(i).write(undef);
            }
        }

        self.frame_index += 1;
        if self.frame_index >= self.frames.len() {
            self.frames.resize(self.frames.len() * 2, CallFrame::new());
        }

        let frame = &mut self.frames[self.frame_index];
        frame.return_pc = return_pc;
        frame.registers_base = base;
        frame.registers_count = needed;
        frame.locals_count = locals_count;
        frame.bytecode_ptr = bytecode_ptr;
        frame.bytecode_len = bytecode_len;
        frame.constants_ptr = constants_ptr;
        frame.constants_len = constants_len;
        frame.function_ptr = function_ptr;
        frame.ic_table_ptr = function_ptr
            .and_then(|fptr| {
                let js_func = unsafe { JSValue::function_from_ptr(fptr) };
                js_func
                    .bytecode
                    .as_ref()
                    .map(|rb| rb.effective_ic_table_ptr())
            })
            .unwrap_or(std::ptr::null_mut());
        frame.this_value = this_value;
        frame.dst_reg = dst_reg;
        frame.arg_count = arg_count;
        frame.super_ctor = JSValue::undefined();
        frame.is_constructor = is_constructor;
        frame.is_async = is_async;
        frame.var_name_map = function_ptr.map_or(std::ptr::null(), |fptr| {
            let func = unsafe { &*(fptr as *const crate::object::function::JSFunction) };
            func.bytecode.as_ref().map_or(std::ptr::null(), |bc| {
                std::rc::Rc::as_ptr(&bc.var_name_to_slot)
            })
        });
        frame.cached_arguments = None;
        frame.uses_arguments = save_args;
        if save_args {
            frame.saved_args.clear();
            frame.saved_args.extend_from_slice(args);
        } else if !frame.saved_args.is_empty() {
            frame.saved_args.clear();
        }
        if frame.upvalue_sync_map.is_some() {
            frame.upvalue_sync_map = None;
            frame.upvalue_sync_bitset = 0;
        }
        self.pc = 0;
        self.refresh_cache();
    }

    #[inline(always)]
    fn push_frame_from_arg_regs_raw(
        &mut self,
        _ctx: &mut JSContext,
        locals_count: u32,
        bytecode_ptr: *const u8,
        bytecode_len: usize,
        constants_ptr: *const JSValue,
        constants_len: usize,
        return_pc: usize,
        function_ptr: Option<usize>,
        ic_table_ptr: *mut crate::compiler::InlineCacheTable,
        this_value: JSValue,
        dst_reg: u16,
        arg_count: u16,
        is_constructor: bool,
        is_async: bool,
        _caller_base: usize,
        arg_regs: &[u16],
        save_args: bool,
    ) {
        let needed = (locals_count as usize).max(arg_count as usize + 1);
        let prev = &self.frames[self.frame_index];
        let base = prev.registers_base + prev.registers_count;
        let total = base + needed;

        if total > self.registers.len() {
            if total > self.registers.capacity() {
                let new_cap = (self.registers.capacity() * 2 + 64).max(total);
                self.registers.reserve(new_cap - self.registers.len());
            }

            unsafe {
                self.registers.set_len(total);
            }
            self.cached_registers_ptr =
                unsafe { self.registers.as_mut_ptr().add(self.cached_registers_base) };
        }

        unsafe {
            let ptr = self.registers.as_mut_ptr().add(base);
            ptr.write(this_value);

            let caller_ptr = self.cached_registers_ptr;
            for (i, r) in arg_regs.iter().enumerate() {
                ptr.add(i + 1).write(*caller_ptr.add(*r as usize));
            }
            let fill_start = arg_regs.len() + 1;
            if fill_start < needed {
                let undef = JSValue::undefined();
                for i in fill_start..needed {
                    ptr.add(i).write(undef);
                }
            }
        }

        self.frame_index += 1;
        let is_new_frame = self.frame_index >= self.frames.len();
        if is_new_frame {
            self.frames.resize(self.frames.len() * 2, CallFrame::new());
        }

        let frame = &mut self.frames[self.frame_index];
        frame.return_pc = return_pc;
        frame.registers_base = base;
        frame.registers_count = needed;
        frame.locals_count = locals_count;
        frame.bytecode_ptr = bytecode_ptr;
        frame.bytecode_len = bytecode_len;
        frame.constants_ptr = constants_ptr;
        frame.constants_len = constants_len;
        frame.function_ptr = function_ptr;
        frame.ic_table_ptr = ic_table_ptr;
        frame.this_value = this_value;
        frame.dst_reg = dst_reg;
        frame.arg_count = arg_count;
        frame.super_ctor = JSValue::undefined();
        frame.is_constructor = is_constructor;
        frame.is_async = is_async;
        if let Some(fptr) = function_ptr {
            let func = unsafe { &*(fptr as *const crate::object::function::JSFunction) };
            frame.is_strict_frame = func.is_strict();
            frame.var_name_map = func.bytecode.as_ref().map_or(std::ptr::null(), |bc| {
                std::rc::Rc::as_ptr(&bc.var_name_to_slot)
            });
            frame.has_upvalues = func
                .upvalues
                .as_ref()
                .map_or(false, |uv| !uv.upvalue_slots.is_empty());
        } else {
            frame.is_strict_frame = false;
            frame.var_name_map = std::ptr::null();
            frame.has_upvalues = false;
        }
        frame.cached_arguments = None;
        frame.uses_arguments = save_args;
        if save_args {
            if !is_new_frame && !frame.saved_args.is_empty() {
                frame.saved_args.clear();
            }
            frame.saved_args.reserve(arg_regs.len());
            let caller_ptr = self.cached_registers_ptr;
            for r in arg_regs.iter() {
                frame
                    .saved_args
                    .push(unsafe { *caller_ptr.add(*r as usize) });
            }
        } else if !is_new_frame && !frame.saved_args.is_empty() {
            frame.saved_args.clear();
        }
        if !is_new_frame && frame.upvalue_sync_map.is_some() {
            frame.upvalue_sync_map = None;
            frame.upvalue_sync_bitset = 0;
        }
        self.pc = 0;
        self.refresh_cache();
    }

    #[inline(never)]
    fn scan_eval_bindings(
        frames: &[CallFrame],
        frame_index: usize,
        atom_id: u32,
    ) -> Option<JSValue> {
        for fi in (0..=frame_index).rev() {
            if let Some(ref eb) = frames[fi].eval_bindings {
                if let Some(val) = eb.get(&atom_id) {
                    return Some(*val);
                }
            }
        }
        None
    }

    #[inline(always)]
    fn pop_frame(&mut self, return_value: JSValue) {
        if self.frame_index == 0 {
            return;
        }

        if self.eval_binding_frames > 0 && self.frames[self.frame_index].eval_bindings.is_some() {
            self.frames[self.frame_index].eval_bindings = None;
            self.eval_binding_frames -= 1;
        }
        let frame = &self.frames[self.frame_index];
        let dst_reg = frame.dst_reg;
        let return_pc = frame.return_pc;

        self.frame_index -= 1;
        self.pc = return_pc;
        self.refresh_cache();
        unsafe {
            *self.cached_registers_ptr.add(dst_reg as usize) = return_value;
        }

        let caller = &self.frames[self.frame_index];
        if let Some(ref sync_map) = caller.upvalue_sync_map {
            if let Some(cell) = sync_map.get(&dst_reg) {
                cell.set(return_value);
            }
        }
    }

    pub fn find_var_in_frame_stack(&self, name_atom: u32) -> Option<(usize, u16)> {
        for fi in (0..=self.frame_index).rev() {
            let frame = &self.frames[fi];
            if !frame.var_name_map.is_null() {
                let map = unsafe { &*frame.var_name_map };
                for &(an, slot) in map.iter().rev() {
                    if an == name_atom {
                        return Some((fi, slot));
                    }
                }
            }
        }
        None
    }

    #[inline(always)]
    pub fn get_var_in_caller_vm(&self, name_atom: u32) -> Option<JSValue> {
        if let Some(ptr) = self.caller_vm {
            let caller = unsafe { &*(ptr as *const VM) };
            for fi in (0..=caller.frame_index).rev() {
                let frame = &caller.frames[fi];
                if let Some(ref eb) = frame.eval_bindings {
                    if let Some(val) = eb.get(&name_atom) {
                        return Some(*val);
                    }
                }
                if !frame.var_name_map.is_null() {
                    let map = unsafe { &*frame.var_name_map };
                    for &(an, slot) in map.iter().rev() {
                        if an == name_atom {
                            let base = frame.registers_base;
                            let val = caller.registers[base + slot as usize];
                            return Some(val);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn set_var_in_caller_vm(
        &mut self,
        ctx: &mut JSContext,
        name_atom: u32,
        value: JSValue,
    ) -> bool {
        if let Some(ptr) = self.caller_vm {
            let caller = unsafe { &mut *(ptr as *mut VM) };
            for fi in (0..=caller.frame_index).rev() {
                let in_eb = caller.frames[fi]
                    .eval_bindings
                    .as_ref()
                    .map_or(false, |eb| eb.contains_key(&name_atom));
                if in_eb {
                    caller.frames[fi]
                        .eval_bindings
                        .as_mut()
                        .unwrap()
                        .insert(name_atom, value);

                    if fi == 0 {
                        let global = ctx.global();
                        if global.is_object() {
                            global.as_object_mut().set_cached(
                                crate::runtime::atom::Atom(name_atom),
                                value,
                                ctx.shape_cache_mut(),
                            );
                        }
                    }
                    return true;
                }
                if !caller.frames[fi].var_name_map.is_null() {
                    let map = unsafe { &*caller.frames[fi].var_name_map };
                    for &(an, slot) in map.iter().rev() {
                        if an == name_atom {
                            let base = caller.frames[fi].registers_base;
                            if fi == caller.frame_index {
                                caller.registers[base + slot as usize] = value;
                            } else {
                                let saved = caller.frame_index;
                                caller.frame_index = fi;
                                caller.refresh_cache();
                                caller.set_reg(slot, value);
                                caller.frame_index = saved;
                                caller.refresh_cache();
                            }

                            if fi == 0 {
                                let global = ctx.global();
                                if global.is_object() {
                                    global.as_object_mut().set_cached(
                                        crate::runtime::atom::Atom(name_atom),
                                        value,
                                        ctx.shape_cache_mut(),
                                    );
                                }
                            }
                            return true;
                        }
                    }
                }
            }

            for fi in (0..=caller.frame_index).rev() {
                if caller.frames[fi].function_ptr.is_some() || fi == 0 {
                    if caller.frames[fi].eval_bindings.is_none() {
                        caller.frames[fi].eval_bindings =
                            Some(Box::new(std::collections::HashMap::new()));
                        caller.eval_binding_frames += 1;
                    }
                    caller.frames[fi]
                        .eval_bindings
                        .as_mut()
                        .unwrap()
                        .insert(name_atom, value);

                    if fi == 0 {
                        let global = ctx.global();
                        if global.is_object() {
                            global.as_object_mut().set_cached(
                                crate::runtime::atom::Atom(name_atom),
                                value,
                                ctx.shape_cache_mut(),
                            );
                        }
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn init_var_in_caller_vm(
        &mut self,
        ctx: &mut JSContext,
        name_atom: u32,
        value: JSValue,
    ) -> bool {
        if let Some(ptr) = self.caller_vm {
            let caller = unsafe { &mut *(ptr as *mut VM) };
            for fi in (0..=caller.frame_index).rev() {
                if let Some(ref eb) = caller.frames[fi].eval_bindings {
                    if eb.contains_key(&name_atom) {
                        return true;
                    }
                }
                if !caller.frames[fi].var_name_map.is_null() {
                    let map = unsafe { &*caller.frames[fi].var_name_map };
                    if map.iter().any(|&(an, _)| an == name_atom) {
                        return true;
                    }
                }
            }

            for fi in (0..=caller.frame_index).rev() {
                if caller.frames[fi].function_ptr.is_some() || fi == 0 {
                    if caller.frames[fi].eval_bindings.is_none() {
                        caller.frames[fi].eval_bindings =
                            Some(Box::new(std::collections::HashMap::new()));
                        caller.eval_binding_frames += 1;
                    }
                    caller.frames[fi]
                        .eval_bindings
                        .as_mut()
                        .unwrap()
                        .insert(name_atom, value);
                    if fi == 0 {
                        let global = ctx.global();
                        if global.is_object() {
                            global.as_object_mut().set_cached(
                                crate::runtime::atom::Atom(name_atom),
                                value,
                                ctx.shape_cache_mut(),
                            );
                        }
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn set_var_in_frame_stack(&mut self, ctx: &mut JSContext, name: &str, value: JSValue) {
        let atom = ctx.intern(name);
        for fi in (0..=self.frame_index).rev() {
            if !self.frames[fi].var_name_map.is_null() {
                let map = unsafe { &*self.frames[fi].var_name_map };
                for &(an, slot) in map.iter().rev() {
                    if an == atom.0 {
                        let base = self.frames[fi].registers_base;
                        if fi == self.frame_index {
                            self.registers[base + slot as usize] = value;
                        } else {
                            let saved_fi = self.frame_index;
                            self.frame_index = fi;
                            self.refresh_cache();
                            self.set_reg(slot, value);
                            self.frame_index = saved_fi;
                            self.refresh_cache();
                        }
                        return;
                    }
                }
            }
        }
        let global = ctx.global();
        if global.is_object() {
            global
                .as_object_mut()
                .set_cached(atom, value, ctx.shape_cache_mut());
        }
    }

    #[inline(always)]
    fn execute_call(
        &mut self,
        ctx: &mut JSContext,
        func_val: JSValue,
        this_val: JSValue,
        dst: u16,
        argc: u16,
        arg_regs: &[u16],
        obj_reg: u16,
        is_call_new: bool,
        is_call_method: bool,
    ) -> Result<bool, String> {
        if func_val.is_function() {
            let ptr = func_val.get_ptr();
            let js_func = unsafe { JSValue::function_from_ptr(ptr) };
            if let Some(ref rb) = js_func.bytecode {
                let return_pc = self.pc;
                let caller_base = self.cached_registers_base;
                let uses_arguments = js_func.uses_arguments();

                if js_func.is_generator() {
                    let mut args_buf = [JSValue::undefined(); 16];
                    let mut args_vec = Vec::new();
                    let args: &[JSValue] = if argc as usize <= 16 {
                        for (i, r) in arg_regs.iter().enumerate() {
                            args_buf[i] = self.registers[caller_base + *r as usize];
                        }
                        &args_buf[..argc as usize]
                    } else {
                        args_vec.reserve(argc as usize);
                        for r in arg_regs {
                            args_vec.push(self.registers[caller_base + *r as usize]);
                        }
                        &args_vec
                    };

                    let saved_frame_index = self.frame_index;

                    let snapshot_this = if !js_func.is_strict()
                        && (this_val.is_undefined() || this_val.is_null())
                    {
                        ctx.global()
                    } else {
                        this_val
                    };
                    let min_slots = 1 + args.len();
                    let snapshot_len = (rb.locals_count as usize).max(min_slots);
                    let mut snapshot = vec![JSValue::undefined(); snapshot_len];
                    snapshot[0] = snapshot_this;
                    for (i, arg) in args.iter().enumerate() {
                        snapshot[i + 1] = *arg;
                    }

                    let has_params = argc < rb.param_count;

                    let saved_handlers = self.exception_handlers.clone();

                    let (result_snapshot, result_pc, result_done) = if has_params {
                        match self.execute_generator_step(ctx, rb, &snapshot, 0) {
                            Ok((_val, new_snapshot, new_pc, done)) => {
                                self.exception_handlers = saved_handlers;
                                self.frame_index = saved_frame_index;
                                (new_snapshot, new_pc, done)
                            }
                            Err(e) => {
                                self.exception_handlers = saved_handlers;
                                self.frame_index = saved_frame_index;
                                if self.pending_throw.is_some() {
                                    return Ok(false);
                                }
                                if e.contains("SyntaxError") || e.contains("Uncaught") {
                                    return Err(e);
                                }
                                (snapshot, 0, false)
                            }
                        }
                    } else {
                        (snapshot, 0, false)
                    };

                    let mut gen_obj = crate::object::object::JSObject::new();
                    gen_obj.set_is_generator(true);
                    if let Some(proto_ptr) = ctx.get_generator_prototype() {
                        gen_obj.prototype = Some(proto_ptr);
                    }
                    gen_obj.set_generator_state(crate::object::object::GeneratorState {
                        bytecode: Box::new((**rb).clone()),
                        snapshot: result_snapshot,
                        pc: result_pc,
                        done: result_done,
                    });
                    let gen_ptr = Box::into_raw(Box::new(gen_obj)) as usize;
                    ctx.runtime_mut().gc_heap_mut().track(gen_ptr);
                    self.set_reg(dst, JSValue::new_object(gen_ptr));

                    self.frame_index = saved_frame_index;

                    return Ok(false);
                }

                if is_call_new && rb.is_simple_constructor {
                    if let Some(cached_shape) = rb.cached_constructor_final_shape {
                        if this_val.is_object() {
                            let obj = unsafe { JSValue::object_from_ptr_mut(this_val.get_ptr()) };
                            let cached_regs_ptr = self.cached_registers_ptr;
                            obj.fast_init_from_simple_constructor(
                                rb.simple_constructor_props
                                    .iter()
                                    .map(|&(atom, arg_idx, _)| {
                                        let value = if (arg_idx as usize) < arg_regs.len() {
                                            unsafe {
                                                *cached_regs_ptr
                                                    .add(arg_regs[arg_idx as usize] as usize)
                                            }
                                        } else {
                                            JSValue::undefined()
                                        };
                                        (atom, value)
                                    }),
                                cached_shape,
                            );
                        }
                        self.set_reg(dst, this_val);
                        return Ok(false);
                    }
                }
                let fn_this =
                    if !js_func.is_strict() && (this_val.is_undefined() || this_val.is_null()) {
                        ctx.global()
                    } else {
                        this_val
                    };
                self.push_frame_from_arg_regs_raw(
                    ctx,
                    rb.locals_count,
                    rb.effective_code_ptr(),
                    rb.effective_code_len(),
                    rb.effective_const_ptr(),
                    rb.effective_const_len(),
                    return_pc,
                    Some(ptr),
                    rb.effective_ic_table_ptr(),
                    fn_this,
                    dst,
                    argc,
                    is_call_new,
                    js_func.is_async(),
                    caller_base,
                    arg_regs,
                    uses_arguments,
                );
                if is_call_new {
                    let super_key = ctx.common_atoms.__super__;
                    if let Some(super_val) = js_func.base.get(super_key) {
                        self.frames[self.frame_index].super_ctor = super_val;
                    }
                }
                Ok(true)
            } else if js_func.is_builtin() {
                let caller_base = self.cached_registers_base;
                let mut args_buf = [JSValue::undefined(); 17];
                let builtin_name = js_func
                    .builtin_atom
                    .map(|ba| ctx.get_atom_str(ba).to_string())
                    .unwrap_or_default();
                let needs_callee = Self::builtin_needs_callee(&builtin_name);
                let pass_this = is_call_method && !needs_callee;
                let builtin_arg_count = argc as usize
                    + if pass_this { 1 } else { 0 }
                    + if needs_callee { 1 } else { 0 };
                let mut args_vec = Vec::new();
                let args: &[JSValue] = if builtin_arg_count <= 16 {
                    let mut idx = 0;
                    if needs_callee {
                        args_buf[idx] = func_val;
                        idx += 1;
                    }
                    if pass_this {
                        args_buf[0] = self.registers[caller_base + obj_reg as usize];
                        idx += 1;
                    }
                    for (i, r) in arg_regs.iter().enumerate() {
                        args_buf[idx + i] = self.registers[caller_base + *r as usize];
                    }
                    &args_buf[..builtin_arg_count]
                } else {
                    args_vec.reserve(builtin_arg_count);
                    if needs_callee {
                        args_vec.push(func_val);
                    }
                    if pass_this {
                        args_vec.push(self.registers[caller_base + obj_reg as usize]);
                    }
                    for r in arg_regs {
                        args_vec.push(self.registers[caller_base + *r as usize]);
                    }
                    &args_vec
                };
                let result = if let Some(bf) = js_func.builtin_func {
                    ctx.call_builtin_direct(bf, args)
                } else if let Some(ba) = js_func.builtin_atom {
                    let name = ctx.get_atom_str(ba).to_string();
                    ctx.call_builtin(&name, args)
                } else {
                    JSValue::undefined()
                };
                if let Some(exc) = ctx.pending_exception.take() {
                    match self.dispatch_throw_value(ctx, exc) {
                        ThrowDispatch::Caught => return Ok(false),
                        ThrowDispatch::Uncaught(e) => return Err(e),
                        ThrowDispatch::AsyncComplete(o) => match o {
                            ExecutionOutcome::Complete(v) => {
                                self.set_reg(dst, v);
                                return Ok(false);
                            }
                            ExecutionOutcome::Yield(v) => {
                                self.set_reg(dst, v);
                                return Ok(false);
                            }
                        },
                    }
                }
                if is_call_new && !result.is_object_like() {
                    if this_val.is_object() {
                        this_val
                            .as_object_mut()
                            .set(ctx.common_atoms.__value__, result);
                    }
                    self.set_reg(dst, this_val);
                } else {
                    self.set_reg(dst, result);
                }
                Ok(false)
            } else {
                Ok(false)
            }
        } else if func_val.is_object() && !is_call_new {
            let caller_base = self.cached_registers_base;
            let mut args_buf = [JSValue::undefined(); 16];
            let mut args_vec = Vec::new();
            let call_args: &[JSValue] = if argc as usize <= 16 {
                for (i, r) in arg_regs.iter().enumerate() {
                    args_buf[i] = self.registers[caller_base + *r as usize];
                }
                &args_buf[..argc as usize]
            } else {
                args_vec.reserve(argc as usize);
                for r in arg_regs.iter() {
                    args_vec.push(self.registers[caller_base + *r as usize]);
                }
                &args_vec
            };
            match self.call_function_with_this(ctx, func_val, this_val, call_args) {
                Ok(result) => {
                    self.set_reg(dst, result);
                    Ok(false)
                }
                Err(_) => {
                    let msg = format!(
                        "{} is not a function",
                        self.format_thrown_value(&func_val, ctx)
                    );
                    self.set_pending_type_error(ctx, &msg);
                    if let Some(exc) = self.pending_throw.take() {
                        let disp = self.dispatch_throw_value(ctx, exc);
                        match disp {
                            ThrowDispatch::Caught => Ok(false),
                            ThrowDispatch::Uncaught(e) => Err(e),
                            ThrowDispatch::AsyncComplete(o) => match o {
                                ExecutionOutcome::Complete(v) => {
                                    self.set_reg(dst, v);
                                    Ok(false)
                                }
                                _ => Err("call error".to_string()),
                            },
                        }
                    } else {
                        Ok(false)
                    }
                }
            }
        } else {
            let msg = format!(
                "{} is not a function",
                self.format_thrown_value(&func_val, ctx)
            );
            self.set_pending_type_error(ctx, &msg);
            Ok(false)
        }
    }

    #[inline(always)]
    fn read_u8(&mut self) -> u8 {
        let pc = self.pc;
        let val = unsafe { *self.cached_code_ptr.add(pc) };
        self.pc = pc + 1;
        val
    }

    #[inline(always)]
    fn read_i64(&mut self) -> i64 {
        let val =
            unsafe { std::ptr::read_unaligned(self.cached_code_ptr.add(self.pc) as *const i64) };
        self.pc += 8;
        val
    }

    #[inline(always)]
    fn read_i32(&mut self) -> i32 {
        let val =
            unsafe { std::ptr::read_unaligned(self.cached_code_ptr.add(self.pc) as *const i32) };
        self.pc += 4;
        val
    }

    #[inline(always)]
    fn read_u32(&mut self) -> u32 {
        self.read_i32() as u32
    }

    #[inline(always)]
    fn get_reg(&self, idx: u16) -> JSValue {
        if self.cached_has_upvalue_sync {
            return self.get_reg_upvalue_slow(idx);
        }
        unsafe { *self.cached_registers_ptr.add(idx as usize) }
    }

    #[cold]
    #[inline(never)]
    fn get_reg_upvalue_slow(&self, idx: u16) -> JSValue {
        let captured = if idx < 64 {
            self.cached_upvalue_sync_bitset & (1u64 << idx) != 0
        } else {
            true
        };
        if captured {
            let frame = &self.frames[self.frame_index];
            if let Some(cell) = frame.upvalue_sync_map.as_ref().and_then(|m| m.get(&idx)) {
                return cell.get();
            }
        }
        unsafe { *self.cached_registers_ptr.add(idx as usize) }
    }

    #[inline(always)]
    fn set_reg(&mut self, idx: u16, val: JSValue) {
        unsafe {
            *self.cached_registers_ptr.add(idx as usize) = val;
        }
        if self.cached_has_upvalue_sync {
            self.set_reg_upvalue_slow(idx, val);
        }
    }

    #[cold]
    #[inline(never)]
    fn set_reg_upvalue_slow(&mut self, idx: u16, val: JSValue) {
        let captured = if idx < 64 {
            self.cached_upvalue_sync_bitset & (1u64 << idx) != 0
        } else {
            true
        };
        if captured {
            let frame = &self.frames[self.frame_index];
            if let Some(cell) = frame.upvalue_sync_map.as_ref().and_then(|m| m.get(&idx)) {
                cell.set(val);
            }
        }
    }

    fn format_stack_trace(&self, ctx: &JSContext) -> String {
        let mut frames = Vec::new();
        for fi in (0..=self.frame_index).rev() {
            let frame = &self.frames[fi];
            let func_name = if let Some(fptr) = frame.function_ptr {
                let func_val = JSValue::new_function(fptr);
                let js_func = func_val.as_function();
                ctx.get_atom_str(js_func.name).to_string()
            } else {
                "<top-level>".to_string()
            };
            let mut location = crate::compiler::location::SourceLocation::unknown();
            if let Some(fptr) = frame.function_ptr {
                let func_val = JSValue::new_function(fptr);
                let js_func = func_val.as_function();
                location.filename = js_func.source_filename.clone();
                if let Some(ref table) = js_func.line_number_table {
                    if let Some(line) = table.lookup_line(frame.current_pc as u32) {
                        location.line = line;
                    }
                }
            }
            frames.push(crate::compiler::location::FrameInfo::new(
                func_name, location,
            ));
        }
        let mut result = String::new();
        for frame in frames.iter().rev() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&format!(
                "  at {} ({}:{}:{})",
                frame.function_name,
                frame.location.filename,
                frame.location.line,
                frame.location.column
            ));
        }
        result
    }

    #[cold]
    fn set_pending_type_error(&mut self, ctx: &mut JSContext, msg: &str) {
        use crate::object::object::JSObject;
        let mut err = JSObject::new();
        let name_atom = ctx.intern("name");
        let msg_atom = ctx.intern("message");
        let type_error_atom = ctx.intern("TypeError");
        let msg_str_atom = ctx.intern(msg);
        err.set(name_atom, JSValue::new_string(type_error_atom));
        err.set(msg_atom, JSValue::new_string(msg_str_atom));
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        self.pending_throw = Some(JSValue::new_object(ptr));
    }

    #[cold]
    fn dispatch_throw_value(&mut self, ctx: &mut JSContext, value: JSValue) -> ThrowDispatch {
        let find_async_frame = |frames: &[CallFrame], frame_index: usize| {
            for i in (0..=frame_index).rev() {
                if frames[i].is_async {
                    return Some(i);
                }
            }
            None
        };

        if let Some(handler) = self.exception_handlers.last().cloned() {
            self.exception_handlers.pop();

            if self.frame_index != handler.frame_index {
                if let Some(async_idx) = find_async_frame(&self.frames, self.frame_index) {
                    if async_idx > handler.frame_index {
                        while self.frame_index > async_idx {
                            if self.frame_index == 0 {
                                break;
                            }
                            self.frame_index -= 1;
                        }
                        let rejected = ctx.call_builtin("promise_reject", &[value]);
                        if self.frame_index == 0 {
                            return ThrowDispatch::AsyncComplete(ExecutionOutcome::Complete(
                                rejected,
                            ));
                        }
                        let return_pc = self.frames[self.frame_index].return_pc;
                        let dst_reg = self.frames[self.frame_index].dst_reg;
                        self.frame_index -= 1;
                        self.pc = return_pc;
                        self.refresh_cache();
                        self.set_reg(dst_reg, rejected);
                        return ThrowDispatch::Caught;
                    }
                }
                while self.frame_index > handler.frame_index {
                    if self.frame_index == 0 {
                        break;
                    }
                    self.frame_index -= 1;
                }
            }

            if handler
                .finally_pc
                .map_or(false, |fp| fp == handler.catch_pc)
            {
                self.finally_rethrow = Some(value);
            }
            self.pc = handler.catch_pc;
            self.refresh_cache();
            self.set_reg(0, value);
            ThrowDispatch::Caught
        } else {
            if let Some(async_idx) = find_async_frame(&self.frames, self.frame_index) {
                let rejected = ctx.call_builtin("promise_reject", &[value]);
                if async_idx == 0 {
                    return ThrowDispatch::AsyncComplete(ExecutionOutcome::Complete(rejected));
                }
                while self.frame_index > async_idx {
                    self.frame_index -= 1;
                }
                let return_pc = self.frames[self.frame_index].return_pc;
                let dst_reg = self.frames[self.frame_index].dst_reg;
                self.frame_index -= 1;
                self.pc = return_pc;
                self.refresh_cache();
                self.set_reg(dst_reg, rejected);
                return ThrowDispatch::Caught;
            }
            let msg = self.format_thrown_value(&value, ctx);
            let trace = self.format_stack_trace(ctx);
            ThrowDispatch::Uncaught(format!("Uncaught: {}\nStack trace:\n{}", msg, trace))
        }
    }

    fn format_thrown_value(&self, value: &JSValue, ctx: &JSContext) -> String {
        if value.is_string() {
            return ctx.get_atom_str(value.get_atom()).to_string();
        }
        if value.is_int() {
            return value.get_int().to_string();
        }
        if value.is_float() {
            return value.get_float().to_string();
        }
        if value.is_bool() {
            return value.get_bool().to_string();
        }
        if value.is_null() {
            return "null".to_string();
        }
        if value.is_undefined() {
            return "undefined".to_string();
        }
        if value.is_object() {
            let ptr = value.get_ptr();
            let obj = unsafe { &*(ptr as *const crate::object::JSObject) };
            let name = obj.get(ctx.common_atoms.name).and_then(|v| {
                if v.is_string() {
                    Some(ctx.get_atom_str(v.get_atom()).to_string())
                } else {
                    None
                }
            });
            let message = obj.get(ctx.common_atoms.message).and_then(|v| {
                if v.is_string() {
                    Some(ctx.get_atom_str(v.get_atom()).to_string())
                } else {
                    None
                }
            });
            match (name, message) {
                (Some(n), Some(m)) => {
                    if m.is_empty() {
                        return n;
                    }
                    return format!("{}: {}", n, m);
                }
                (Some(n), _) => return n,
                (None, Some(m)) => {
                    if m.is_empty() {
                        return "Error".to_string();
                    }
                    return m;
                }
                _ => {}
            }
            return "[object Object]".to_string();
        }
        if value.is_function() {
            return "function".to_string();
        }
        if value.is_symbol() {
            return "Symbol".to_string();
        }
        if value.is_bigint() {
            return "BigInt".to_string();
        }
        "<value>".to_string()
    }

    #[inline(never)]
    fn fill_gc_roots(&mut self, ctx: &JSContext) {
        let roots = &mut self.gc_roots;
        roots.clear();
        let global = ctx.global();
        if global.is_object() || global.is_function() {
            roots.push(global);
        }

        for fi in 0..=self.frame_index {
            let frame = &self.frames[fi];
            let base = frame.registers_base;
            let count = frame.registers_count;
            for i in 0..count {
                let v = self.registers[base + i];
                if v.is_object() || v.is_function() {
                    roots.push(v);
                }
            }
            if let Some(ptr) = frame.function_ptr {
                roots.push(JSValue::new_function(ptr));
            }
            let tv = frame.this_value;
            if tv.is_object() || tv.is_function() {
                roots.push(tv);
            }
            let sv = frame.super_ctor;
            if sv.is_object() || sv.is_function() {
                roots.push(sv);
            }
        }

        macro_rules! push_proto {
            ($getter:ident) => {
                if let Some(ptr) = ctx.$getter() {
                    roots.push(JSValue::new_object(ptr as usize));
                }
            };
        }
        push_proto!(get_string_prototype);
        push_proto!(get_number_prototype);
        push_proto!(get_array_prototype);
        push_proto!(get_regexp_prototype);
        push_proto!(get_object_prototype);
        push_proto!(get_function_prototype);
        push_proto!(get_map_prototype);
        push_proto!(get_set_prototype);
        push_proto!(get_weakmap_prototype);
        push_proto!(get_weakset_prototype);
        push_proto!(get_error_prototype);
        push_proto!(get_type_error_prototype);
        push_proto!(get_weakref_prototype);
        push_proto!(get_finalization_registry_prototype);
        push_proto!(get_generator_prototype);
        push_proto!(get_async_generator_prototype);
        push_proto!(get_promise_prototype);

        for &v in &ctx.runtime().gc_heap().extra_roots {
            if v.is_object() || v.is_function() {
                roots.push(v);
            }
        }
    }

    pub fn collect_roots(&self, ctx: &JSContext) -> Vec<JSValue> {
        let mut roots = Vec::with_capacity(256);
        let global = ctx.global();
        if global.is_object() || global.is_function() {
            roots.push(global);
        }

        for fi in 0..=self.frame_index {
            let frame = &self.frames[fi];
            let base = frame.registers_base;
            let count = frame.registers_count;
            for i in 0..count {
                let v = self.registers[base + i];
                if v.is_object() || v.is_function() {
                    roots.push(v);
                }
            }
            if let Some(ptr) = frame.function_ptr {
                roots.push(JSValue::new_function(ptr));
            }
            let tv = frame.this_value;
            if tv.is_object() || tv.is_function() {
                roots.push(tv);
            }
            let sv = frame.super_ctor;
            if sv.is_object() || sv.is_function() {
                roots.push(sv);
            }
        }

        macro_rules! push_proto {
            ($getter:ident) => {
                if let Some(ptr) = ctx.$getter() {
                    roots.push(JSValue::new_object(ptr as usize));
                }
            };
        }
        push_proto!(get_string_prototype);
        push_proto!(get_number_prototype);
        push_proto!(get_array_prototype);
        push_proto!(get_regexp_prototype);
        push_proto!(get_object_prototype);
        push_proto!(get_function_prototype);
        push_proto!(get_map_prototype);
        push_proto!(get_set_prototype);
        push_proto!(get_weakmap_prototype);
        push_proto!(get_weakset_prototype);
        push_proto!(get_error_prototype);
        push_proto!(get_type_error_prototype);
        push_proto!(get_weakref_prototype);
        push_proto!(get_finalization_registry_prototype);
        push_proto!(get_generator_prototype);
        push_proto!(get_async_generator_prototype);
        push_proto!(get_promise_prototype);

        for &v in &ctx.runtime().gc_heap().extra_roots {
            if v.is_object() || v.is_function() {
                roots.push(v);
            }
        }
        roots
    }

    #[inline(always)]
    pub fn maybe_gc(&mut self, ctx: &mut JSContext) {
        if self.allocation_count >= GC_CHECK_INTERVAL {
            self.allocation_count = 0;
            let do_minor = ctx.runtime().gc_heap().nursery_is_full();
            let do_full = ctx.runtime().gc_heap().should_collect();
            if do_minor || do_full {
                self.fill_gc_roots(ctx);

                let roots_ptr = self.gc_roots.as_ptr();
                let roots_len = self.gc_roots.len();
                let roots = unsafe { std::slice::from_raw_parts(roots_ptr, roots_len) };
                if do_minor {
                    let _ = ctx.runtime_mut().minor_gc(roots);
                }

                if do_full || ctx.runtime().gc_heap().should_collect() {
                    self.run_gc(ctx);
                }
            }
        }
    }

    pub fn minor_gc(&mut self, ctx: &mut JSContext) -> usize {
        self.fill_gc_roots(ctx);
        let roots_ptr = self.gc_roots.as_ptr();
        let roots_len = self.gc_roots.len();
        let roots = unsafe { std::slice::from_raw_parts(roots_ptr, roots_len) };
        ctx.runtime_mut().minor_gc(roots)
    }

    pub fn run_gc(&mut self, ctx: &mut JSContext) -> usize {
        self.fill_gc_roots(ctx);
        let roots_ptr = self.gc_roots.as_ptr();
        let roots_len = self.gc_roots.len();
        let roots = unsafe { std::slice::from_raw_parts(roots_ptr, roots_len) };
        let freed = ctx.runtime_mut().run_gc(roots);

        if ctx.runtime().gc_heap().deleted_props_count > 0 {
            let live_objects: Vec<(usize, u8)> = {
                let heap = ctx.runtime().gc_heap();
                let mut objs = Vec::new();
                heap.for_each_live_object(|ptr, tag| objs.push((ptr, tag)));
                objs
            };
            {
                let cache = ctx.shape_cache_mut();
                for (ptr, tag) in live_objects {
                    unsafe {
                        if tag == crate::runtime::gc::TAG_ARRAY {
                            let arr = &mut *(ptr as *mut crate::object::array_obj::JSArrayObject);
                            arr.header.compact_props(cache);
                        } else {
                            let obj = &mut *(ptr as *mut crate::object::object::JSObject);
                            obj.compact_props(cache);
                        }
                    }
                }
            }

            ctx.runtime_mut().gc_heap_mut().deleted_props_count = 0;
        }
        freed
    }

    pub fn call_function(
        &mut self,
        ctx: &mut JSContext,
        func: JSValue,
        args: &[JSValue],
    ) -> Result<JSValue, String> {
        self.call_function_with_this(ctx, func, JSValue::undefined(), args)
    }

    pub fn call_function_with_this(
        &mut self,
        ctx: &mut JSContext,
        func: JSValue,
        this_value: JSValue,
        args: &[JSValue],
    ) -> Result<JSValue, String> {
        if func.is_object() {
            let obj = func.as_object();
            if let Some(bound_fn) = obj.get(ctx.common_atoms.__boundFn) {
                let bound_this = obj.get(ctx.common_atoms.__boundThis).unwrap_or(this_value);
                let bound_args_val = obj
                    .get(ctx.common_atoms.__boundArgs)
                    .filter(|val| val.is_object());

                let bound_len = bound_args_val
                    .and_then(|v| v.as_object().get(ctx.common_atoms.length))
                    .map(|v| v.get_int() as usize)
                    .unwrap_or(0);

                let mut actual_args = Vec::with_capacity(bound_len.saturating_add(args.len()));
                if let Some(bound_args_val) = bound_args_val {
                    let bound_args_obj = bound_args_val.as_object();
                    for i in 0..bound_len {
                        let key = self.int_atom(i, ctx);
                        if let Some(arg_val) = bound_args_obj.get(key) {
                            actual_args.push(arg_val);
                        }
                    }
                }
                actual_args.extend_from_slice(args);
                return self.call_function_with_this(ctx, bound_fn, bound_this, &actual_args);
            }
        }

        if !func.is_function() {
            return Err("call_function: not a function".to_string());
        }

        let ptr = func.get_ptr();
        let js_func = func.as_function();

        if js_func.is_builtin() {
            let builtin_name = js_func
                .builtin_atom
                .map(|ba| ctx.get_atom_str(ba).to_string())
                .unwrap_or_default();
            let needs_this = Self::builtin_needs_this_for_call_with_this(&builtin_name);

            let mut call_args_vec: Vec<JSValue> = Vec::new();
            let call_args: &[JSValue] = if needs_this {
                call_args_vec.reserve(args.len() + 1);
                call_args_vec.push(this_value);
                call_args_vec.extend_from_slice(args);
                &call_args_vec
            } else {
                args
            };

            if let Some(bf) = js_func.builtin_func {
                let result = ctx.call_builtin_direct(bf, call_args);
                if let Some(exc) = ctx.pending_exception.take() {
                    let msg = if exc.is_string() {
                        ctx.get_atom_str(exc.get_atom()).to_string()
                    } else if exc.is_object() {
                        let obj = exc.as_object();
                        let m = obj.get(ctx.common_atoms.message);
                        let n = obj.get(ctx.intern("name"));
                        match (n, m) {
                            (Some(nv), Some(mv)) if nv.is_string() && mv.is_string() => {
                                format!(
                                    "{}: {}",
                                    ctx.get_atom_str(nv.get_atom()),
                                    ctx.get_atom_str(mv.get_atom())
                                )
                            }
                            _ => "builtin error".to_string(),
                        }
                    } else {
                        "builtin error".to_string()
                    };
                    return Err(msg);
                }
                return Ok(result);
            } else if let Some(ba) = js_func.builtin_atom {
                let result = ctx.call_builtin(&ctx.get_atom_str(ba).to_string(), call_args);
                if let Some(exc) = ctx.pending_exception.take() {
                    let msg = if exc.is_string() {
                        ctx.get_atom_str(exc.get_atom()).to_string()
                    } else if exc.is_object() {
                        let obj = exc.as_object();
                        let m = obj.get(ctx.common_atoms.message);
                        let n = obj.get(ctx.intern("name"));
                        match (n, m) {
                            (Some(nv), Some(mv)) if nv.is_string() && mv.is_string() => {
                                format!(
                                    "{}: {}",
                                    ctx.get_atom_str(nv.get_atom()),
                                    ctx.get_atom_str(mv.get_atom())
                                )
                            }
                            _ => "builtin error".to_string(),
                        }
                    } else {
                        "builtin error".to_string()
                    };
                    return Err(msg);
                }
                return Ok(result);
            }
            return Ok(JSValue::undefined());
        }

        if let Some(ref rb) = js_func.bytecode {
            let saved_frame_index = self.frame_index;
            let saved_pc = self.pc;
            let saved_r0 = self.get_reg(0);

            let fn_this =
                if !js_func.is_strict() && (this_value.is_undefined() || this_value.is_null()) {
                    ctx.global()
                } else {
                    this_value
                };
            self.push_frame_raw(
                rb.locals_count,
                rb.effective_code_ptr(),
                rb.effective_code_len(),
                rb.effective_const_ptr(),
                rb.effective_const_len(),
                self.frames[self.frame_index].bytecode_len,
                Some(ptr),
                fn_this,
                0,
                args.len() as u16,
                false,
                js_func.is_async(),
                args,
                js_func.uses_arguments(),
            );

            let result = self.execute_inner(ctx, rb, false, 0, false);
            let return_value = self.get_reg(0);

            while self.frame_index > saved_frame_index {
                self.pop_frame(JSValue::undefined());
            }
            self.pc = saved_pc;
            self.refresh_cache();
            self.set_reg(0, saved_r0);

            return match result {
                Ok(_) => Ok(return_value),
                Err(e) => Err(e),
            };
        }

        Ok(JSValue::undefined())
    }

    pub fn execute(
        &mut self,
        ctx: &mut JSContext,
        bytecode: &Bytecode,
    ) -> Result<ExecutionOutcome, String> {
        let result = self.execute_inner(ctx, bytecode, false, 0, true);

        if !self.ctx_ptr.is_null() {
            unsafe {
                let count = self.frames[0].registers_count;
                for i in 0..count {
                    self.registers[i].release_atoms_in(&mut *self.ctx_ptr);
                    self.registers[i] = JSValue::undefined();
                }
            }
        }
        self.ctx_ptr = std::ptr::null_mut();
        result
    }

    pub fn execute_preserving_registers(
        &mut self,
        ctx: &mut JSContext,
        bytecode: &Bytecode,
    ) -> Result<ExecutionOutcome, String> {
        let result = self.execute_inner(ctx, bytecode, true, 0, true);
        self.ctx_ptr = std::ptr::null_mut();
        result
    }

    pub fn execute_eval(
        &mut self,
        ctx: &mut JSContext,
        bytecode: &Bytecode,
        this_value: JSValue,
        caller_vm_ptr: Option<usize>,
    ) -> Result<ExecutionOutcome, String> {
        self.ctx_ptr = ctx;
        self.caller_vm = caller_vm_ptr;
        ctx.set_register_vm_ptr(Some(self as *mut VM as usize));

        let needed = (bytecode.locals_count as usize).max(1);
        if needed > self.registers.len() {
            self.registers.resize(needed, JSValue::undefined());
        }
        self.registers[0] = this_value;

        let frame = &mut self.frames[0];
        frame.registers_base = 0;
        frame.registers_count = needed;
        frame.locals_count = bytecode.locals_count;
        frame.bytecode_ptr = bytecode.effective_code_ptr();
        frame.bytecode_len = bytecode.effective_code_len();
        frame.constants_ptr = bytecode.effective_const_ptr();
        frame.constants_len = bytecode.effective_const_len();
        frame.return_pc = 0;
        frame.function_ptr = None;
        frame.this_value = this_value;
        frame.saved_args.clear();
        frame.upvalue_sync_map = None;
        frame.upvalue_sync_bitset = 0;
        frame.uses_arguments = false;
        frame.var_name_map = std::rc::Rc::as_ptr(&bytecode.var_name_to_slot);
        frame.ic_table_ptr = bytecode.effective_ic_table_ptr();

        self.frame_index = 0;
        self.exception_handlers.clear();
        self.frames[0].is_strict_frame = bytecode.is_strict;
        self.refresh_cache();

        let result = self.execute_inner(ctx, bytecode, true, 0, false);
        self.ctx_ptr = std::ptr::null_mut();
        result
    }

    pub fn direct_eval(&mut self, ctx: &mut JSContext, source: &str) -> Result<JSValue, String> {
        let is_strict_caller = self.frames[self.frame_index].is_strict_frame;

        let trimmed = source.trim();
        if trimmed.starts_with("var ")
            || trimmed.starts_with("let ")
            || trimmed.starts_with("const ")
        {
            let decl_kw = if trimmed.starts_with("var ") {
                "var "
            } else if trimmed.starts_with("let ") {
                "let "
            } else {
                "const "
            };
            let after_kw = &trimmed[decl_kw.len()..].trim_start();
            let var_name = after_kw
                .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '$')
                .next()
                .unwrap_or("");
            if var_name == "arguments" {
                let mut err = crate::object::object::JSObject::new();
                err.set(
                    ctx.intern("name"),
                    JSValue::new_string(ctx.intern("SyntaxError")),
                );
                err.set(
                    ctx.intern("message"),
                    JSValue::new_string(ctx.intern("arguments not allowed in direct eval")),
                );
                if let Some(proto) = ctx.get_syntax_error_prototype() {
                    err.prototype = Some(proto);
                }
                let ptr = Box::into_raw(Box::new(err)) as usize;
                ctx.runtime_mut().gc_heap_mut().track(ptr);
                let exc = JSValue::new_object(ptr);
                match self.dispatch_throw_value(ctx, exc) {
                    ThrowDispatch::Caught => return Ok(JSValue::undefined()),
                    ThrowDispatch::Uncaught(e) => return Err(e),
                    ThrowDispatch::AsyncComplete(o) => match o {
                        ExecutionOutcome::Complete(v) => return Ok(v),
                        _ => return Err("async eval error".to_string()),
                    },
                }
            }
        }

        let caller_vm_ptr = self as *mut VM;
        match crate::compiler::eval_code_via_ast_with_opt_level_as_eval_with_caller(
            ctx,
            source,
            ctx.get_compiler_opt_level(),
            is_strict_caller,
            caller_vm_ptr,
        ) {
            Ok(v) => Ok(v),
            Err(e) => self.convert_eval_error_to_exc(ctx, e),
        }
    }

    fn convert_eval_error_to_exc(
        &mut self,
        ctx: &mut JSContext,
        e: String,
    ) -> Result<JSValue, String> {
        if e.starts_with("Uncaught:") {
            let value_str = e[9..].split('\n').next().unwrap_or("").trim();
            let is_error = value_str.contains("Error:")
                || value_str.starts_with("[object ")
                || value_str.starts_with("function ");
            if is_error {
                let mut err = crate::object::object::JSObject::new();
                let name = if value_str.starts_with("SyntaxError") {
                    "SyntaxError"
                } else if value_str.starts_with("ReferenceError") {
                    "ReferenceError"
                } else if value_str.starts_with("TypeError") {
                    "TypeError"
                } else if value_str.starts_with("RangeError") {
                    "RangeError"
                } else {
                    "Error"
                };
                err.set(ctx.intern("name"), JSValue::new_string(ctx.intern(name)));
                err.set(ctx.intern("message"), JSValue::new_string(ctx.intern(&e)));
                match name {
                    "TypeError" => {
                        if let Some(proto) = ctx.get_type_error_prototype() {
                            err.prototype = Some(proto);
                        }
                    }
                    "SyntaxError" => {
                        if let Some(proto) = ctx.get_syntax_error_prototype() {
                            err.prototype = Some(proto);
                        }
                    }
                    "ReferenceError" => {
                        if let Some(proto) = ctx.get_reference_error_prototype() {
                            err.prototype = Some(proto);
                        }
                    }
                    _ => {
                        if let Some(proto) = ctx.get_error_prototype() {
                            err.prototype = Some(proto);
                        }
                    }
                }
                let ptr = Box::into_raw(Box::new(err)) as usize;
                ctx.runtime_mut().gc_heap_mut().track(ptr);
                let exc = JSValue::new_object(ptr);
                match self.dispatch_throw_value(ctx, exc) {
                    ThrowDispatch::Caught => Ok(JSValue::undefined()),
                    ThrowDispatch::Uncaught(e) => Err(e),
                    ThrowDispatch::AsyncComplete(o) => match o {
                        ExecutionOutcome::Complete(v) => Ok(v),
                        _ => Err("async eval error".to_string()),
                    },
                }
            } else {
                let raw_value = if let Ok(n) = value_str.parse::<i64>() {
                    JSValue::new_int(n)
                } else if let Ok(f) = value_str.parse::<f64>() {
                    JSValue::new_float(f)
                } else if value_str == "true" {
                    JSValue::bool(true)
                } else if value_str == "false" {
                    JSValue::bool(false)
                } else if value_str == "null" {
                    JSValue::null()
                } else if value_str == "undefined" {
                    JSValue::undefined()
                } else {
                    JSValue::undefined()
                };
                match self.dispatch_throw_value(ctx, raw_value) {
                    ThrowDispatch::Caught => Ok(JSValue::undefined()),
                    ThrowDispatch::Uncaught(e) => Err(e),
                    ThrowDispatch::AsyncComplete(o) => match o {
                        ExecutionOutcome::Complete(v) => Ok(v),
                        _ => Err("async eval error".to_string()),
                    },
                }
            }
        } else if e.starts_with("Parse error") || e.starts_with("SyntaxError") {
            let mut err = crate::object::object::JSObject::new();
            err.set(
                ctx.intern("name"),
                JSValue::new_string(ctx.intern("SyntaxError")),
            );
            err.set(ctx.intern("message"), JSValue::new_string(ctx.intern(&e)));
            if let Some(proto) = ctx.get_syntax_error_prototype() {
                err.prototype = Some(proto);
            }
            let ptr = Box::into_raw(Box::new(err)) as usize;
            ctx.runtime_mut().gc_heap_mut().track(ptr);
            let exc = JSValue::new_object(ptr);
            match self.dispatch_throw_value(ctx, exc) {
                ThrowDispatch::Caught => Ok(JSValue::undefined()),
                ThrowDispatch::Uncaught(e) => Err(e),
                ThrowDispatch::AsyncComplete(o) => match o {
                    ExecutionOutcome::Complete(v) => Ok(v),
                    _ => Err("async eval error".to_string()),
                },
            }
        } else if e.starts_with("ReferenceError") {
            let mut err = crate::object::object::JSObject::new();
            err.set(
                ctx.intern("name"),
                JSValue::new_string(ctx.intern("ReferenceError")),
            );
            err.set(ctx.intern("message"), JSValue::new_string(ctx.intern(&e)));
            if let Some(proto) = ctx.get_reference_error_prototype() {
                err.prototype = Some(proto);
            }
            let ptr = Box::into_raw(Box::new(err)) as usize;
            ctx.runtime_mut().gc_heap_mut().track(ptr);
            let exc = JSValue::new_object(ptr);
            match self.dispatch_throw_value(ctx, exc) {
                ThrowDispatch::Caught => Ok(JSValue::undefined()),
                ThrowDispatch::Uncaught(e) => Err(e),
                ThrowDispatch::AsyncComplete(o) => match o {
                    ExecutionOutcome::Complete(v) => Ok(v),
                    _ => Err("async eval error".to_string()),
                },
            }
        } else if e.starts_with("TypeError") {
            let mut err = crate::object::object::JSObject::new();
            err.set(
                ctx.intern("name"),
                JSValue::new_string(ctx.intern("TypeError")),
            );
            err.set(ctx.intern("message"), JSValue::new_string(ctx.intern(&e)));
            if let Some(proto) = ctx.get_type_error_prototype() {
                err.prototype = Some(proto);
            }
            let ptr = Box::into_raw(Box::new(err)) as usize;
            ctx.runtime_mut().gc_heap_mut().track(ptr);
            let exc = JSValue::new_object(ptr);
            match self.dispatch_throw_value(ctx, exc) {
                ThrowDispatch::Caught => Ok(JSValue::undefined()),
                ThrowDispatch::Uncaught(e) => Err(e),
                ThrowDispatch::AsyncComplete(o) => match o {
                    ExecutionOutcome::Complete(v) => Ok(v),
                    _ => Err("async eval error".to_string()),
                },
            }
        } else {
            Err(e)
        }
    }

    fn execute_inner(
        &mut self,
        ctx: &mut JSContext,
        bytecode: &Bytecode,
        preserve_registers: bool,
        start_pc: usize,
        setup_frame: bool,
    ) -> Result<ExecutionOutcome, String> {
        self.ctx_ptr = ctx;

        if setup_frame {
            let needed = bytecode.locals_count as usize;
            if needed > self.registers.len() {
                self.registers.resize(needed, JSValue::undefined());
            }
            let global_val = ctx.global();
            if !preserve_registers {
                for i in 0..needed {
                    self.registers[i] = if i == 0 {
                        global_val
                    } else {
                        JSValue::undefined()
                    };
                }
            } else {
                self.registers[0] = global_val;
            }

            let frame = &mut self.frames[0];
            frame.registers_base = 0;
            frame.registers_count = needed;
            frame.locals_count = bytecode.locals_count;
            frame.bytecode_ptr = bytecode.effective_code_ptr();
            frame.bytecode_len = bytecode.effective_code_len();
            frame.constants_ptr = bytecode.effective_const_ptr();
            frame.constants_len = bytecode.effective_const_len();
            frame.return_pc = 0;
            frame.function_ptr = None;
            frame.this_value = global_val;
            frame.saved_args.clear();
            frame.upvalue_sync_map = None;
            frame.upvalue_sync_bitset = 0;
            frame.uses_arguments = false;
            frame.var_name_map = std::rc::Rc::as_ptr(&bytecode.var_name_to_slot);
            frame.ic_table_ptr = bytecode.effective_ic_table_ptr();

            self.frame_index = 0;
            self.exception_handlers.clear();
            self.frames[0].is_strict_frame = bytecode.is_strict;
        }
        self.pc = start_pc;
        self.refresh_cache();
        ctx.reset_interrupt_counter();

        loop {
            if self.pending_throw.is_some() {
                let exc = unsafe { self.pending_throw.take().unwrap_unchecked() };
                match self.dispatch_throw_value(ctx, exc) {
                    ThrowDispatch::Caught => {}
                    ThrowDispatch::Uncaught(e) => return Err(e),
                    ThrowDispatch::AsyncComplete(o) => return Ok(o),
                }
            }
            if (self.pc & INTERRUPT_POLL_EVERY_MASK) == 0 {
                ctx.check_interrupt()?;
            }
            if self.pc >= self.cached_code_len {
                return Ok(ExecutionOutcome::Complete(JSValue::undefined()));
            }

            let instr_pc = self.pc;
            let op_val = unsafe { *self.cached_code_ptr.add(instr_pc) };
            self.pc = instr_pc + 1;
            if op_val == 90 {
                let dst = unsafe {
                    std::ptr::read_unaligned(self.cached_code_ptr.add(self.pc) as *const u16)
                };
                self.pc += 2;
                let src = unsafe {
                    std::ptr::read_unaligned(self.cached_code_ptr.add(self.pc) as *const u16)
                };
                self.pc += 2;
                let val = self.get_reg(src);
                self.set_reg(dst, val);
                continue;
            }
            let op = Opcode::from_u8_unchecked(op_val);

            match op {
                Opcode::Nop => {}
                Opcode::End => {
                    if self.frame_index == 0 {
                        return Ok(ExecutionOutcome::Complete(self.get_reg(0)));
                    }
                    self.pop_frame(self.get_reg(0));
                }
                Opcode::Return => {
                    let src = self.read_u16_pc();
                    let mut ret = self.get_reg(src);
                    let frame = &self.frames[self.frame_index];
                    if frame.is_constructor && !ret.is_object() {
                        ret = frame.this_value;
                    }

                    if frame.is_constructor {
                        if let Some(fptr) = frame.function_ptr {
                            let func =
                                unsafe { &*(fptr as *const crate::object::function::JSFunction) };
                            if let Some(ref bc) = func.bytecode {
                                if bc.is_simple_constructor
                                    && bc.cached_constructor_final_shape.is_none()
                                {
                                    if ret.is_object() {
                                        let obj =
                                            unsafe { JSValue::object_from_ptr(ret.get_ptr()) };
                                        if let Some(shape) = obj.shape_ptr() {
                                            let func_mut = unsafe {
                                                &mut *(fptr
                                                    as *mut crate::object::function::JSFunction)
                                            };
                                            if let Some(ref mut bc_mut) = func_mut.bytecode {
                                                bc_mut.cached_constructor_final_shape = Some(shape);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if self.frame_index == 0 {
                        if self.frames[0].is_async {
                            let result = ctx.call_builtin("promise_resolve", &[ret]);
                            return Ok(ExecutionOutcome::Complete(result));
                        }
                        return Ok(ExecutionOutcome::Complete(ret));
                    }
                    let is_async = frame.is_async;
                    if is_async {
                        ret = ctx.call_builtin("promise_resolve", &[ret]);
                    }
                    self.pop_frame(ret);
                }
                Opcode::Yield => {
                    let src = self.read_u16_pc();
                    let val = self.get_reg(src);
                    return Ok(ExecutionOutcome::Yield(val));
                }
                Opcode::Await => {
                    let src = self.read_u16_pc();
                    let val = self.get_reg(src);
                    if crate::builtins::promise::is_promise(&val) {
                        let ptr = val.get_ptr();
                        let promise_obj =
                            unsafe { &*(ptr as *mut crate::object::object::JSObject) };
                        let state_atom = ctx.common_atoms.__promise_state__;
                        let result_atom = ctx.common_atoms.__promise_result__;
                        let state = promise_obj
                            .get(state_atom)
                            .unwrap_or(JSValue::new_int(0))
                            .get_int();
                        let result = promise_obj.get(result_atom).unwrap_or(JSValue::undefined());
                        match state {
                            1 => {
                                self.set_reg(src, result);
                            }
                            2 => {
                                if self.frame_index == 0 {
                                    let rejected = ctx.call_builtin("promise_reject", &[result]);
                                    return Ok(ExecutionOutcome::Complete(rejected));
                                }
                                let mut found_async = false;
                                for i in (0..=self.frame_index).rev() {
                                    if self.frames[i].is_async {
                                        found_async = true;
                                        while self.frame_index > i {
                                            self.pop_frame(JSValue::undefined());
                                        }
                                        let rejected =
                                            ctx.call_builtin("promise_reject", &[result]);
                                        self.set_reg(0, rejected);
                                        break;
                                    }
                                }
                                if !found_async {
                                    return Err(format!("Uncaught (in promise): {:?}", result));
                                }
                            }
                            _ => {
                                self.set_reg(src, JSValue::undefined());
                            }
                        }
                    } else {
                        self.set_reg(src, val);
                    }
                }

                Opcode::LoadConst => {
                    let dst = self.read_u16_pc();
                    let idx = self.read_u32() as usize;
                    let val = if idx < self.frames[self.frame_index].constants_len {
                        unsafe { *self.cached_const_ptr.add(idx) }
                    } else {
                        JSValue::undefined()
                    };
                    self.set_reg(dst, val);
                }
                Opcode::LoadInt => {
                    let dst = self.read_u16_pc();
                    let val = self.read_i32() as i64;
                    self.set_reg(dst, JSValue::new_int(val));
                }
                Opcode::LoadInt8 => {
                    let dst = self.read_u16_pc();
                    let val = self.read_u8() as i8 as i64;
                    self.set_reg(dst, JSValue::new_int(val));
                }
                Opcode::LoadTrue => {
                    let dst = self.read_u16_pc();
                    self.set_reg(dst, JSValue::bool(true));
                }
                Opcode::LoadFalse => {
                    let dst = self.read_u16_pc();
                    self.set_reg(dst, JSValue::bool(false));
                }
                Opcode::LoadNull => {
                    let dst = self.read_u16_pc();
                    self.set_reg(dst, JSValue::null());
                }
                Opcode::LoadUndefined => {
                    let dst = self.read_u16_pc();
                    self.set_reg(dst, JSValue::undefined());
                }

                Opcode::Move => {
                    let dst = self.read_u16_pc();
                    let src = self.read_u16_pc();
                    let val = self.get_reg(src);
                    self.set_reg(dst, val);
                }

                Opcode::Add => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        JSValue::new_int(a.get_int() + b.get_int())
                    } else if a.is_float() && b.is_float() {
                        JSValue::new_float_raw(a.get_float() + b.get_float())
                    } else {
                        self.add_slow(&a, &b, ctx)
                    };
                    if let Some(exc) = self.pending_throw.take() {
                        match self.dispatch_throw_value(ctx, exc) {
                            ThrowDispatch::Caught => {
                                self.set_reg(dst, self.get_reg(0));
                                return Ok(ExecutionOutcome::Complete(self.get_reg(dst)));
                            }
                            ThrowDispatch::Uncaught(e) => return Err(e),
                            ThrowDispatch::AsyncComplete(o) => match o {
                                ExecutionOutcome::Complete(v) => {
                                    self.set_reg(dst, v);
                                    return Ok(ExecutionOutcome::Complete(v));
                                }
                                _ => {
                                    self.set_reg(dst, JSValue::undefined());
                                    return Ok(ExecutionOutcome::Complete(JSValue::undefined()));
                                }
                            },
                        }
                    }
                    self.set_reg(dst, result);
                }
                Opcode::AddNum => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        JSValue::new_int(a.get_int() + b.get_int())
                    } else {
                        let fa = if a.is_float() {
                            a.get_float()
                        } else if a.is_int() {
                            a.get_int() as f64
                        } else {
                            Self::js_to_number(&a, ctx)
                        };
                        let fb = if b.is_float() {
                            b.get_float()
                        } else if b.is_int() {
                            b.get_int() as f64
                        } else {
                            Self::js_to_number(&b, ctx)
                        };
                        JSValue::new_float(fa + fb)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::AddImm8 => {
                    let dst = self.read_u16_pc();
                    let src = self.read_u16_pc();
                    let imm = self.read_u8() as i8 as i64;
                    let val = self.get_reg(src);
                    let result = if val.is_int() {
                        JSValue::new_int(val.get_int() + imm)
                    } else if val.is_float() {
                        JSValue::new_float_raw(val.get_float() + imm as f64)
                    } else {
                        let b = JSValue::new_int(imm);
                        self.add_slow(&val, &b, ctx)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::SubNum => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        JSValue::new_int(a.get_int() - b.get_int())
                    } else {
                        let fa = if a.is_float() {
                            a.get_float()
                        } else if a.is_int() {
                            a.get_int() as f64
                        } else {
                            Self::js_to_number(&a, ctx)
                        };
                        let fb = if b.is_float() {
                            b.get_float()
                        } else if b.is_int() {
                            b.get_int() as f64
                        } else {
                            Self::js_to_number(&b, ctx)
                        };
                        JSValue::new_float(fa - fb)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::MulNum => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        let ai = a.get_int();
                        let bi = b.get_int();
                        match ai.checked_mul(bi) {
                            Some(prod) if prod >= -(1i64 << 46) && prod < (1i64 << 46) => {
                                JSValue::new_int(prod)
                            }
                            _ => JSValue::new_float(ai as f64 * bi as f64),
                        }
                    } else if a.is_float() && b.is_float() {
                        JSValue::new_float_raw(a.get_float() * b.get_float())
                    } else {
                        let fa = if a.is_float() {
                            a.get_float()
                        } else if a.is_int() {
                            a.get_int() as f64
                        } else {
                            Self::js_to_number(&a, ctx)
                        };
                        let fb = if b.is_float() {
                            b.get_float()
                        } else if b.is_int() {
                            b.get_int() as f64
                        } else {
                            Self::js_to_number(&b, ctx)
                        };
                        JSValue::new_float(fa * fb)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::DivNum => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        let ai = a.get_int();
                        let bi = b.get_int();
                        if bi != 0 && ai % bi == 0 {
                            JSValue::new_int(ai / bi)
                        } else if bi != 0 {
                            JSValue::new_float(ai as f64 / bi as f64)
                        } else {
                            JSValue::new_float(f64::NAN)
                        }
                    } else if a.is_float() && b.is_float() {
                        JSValue::new_float_raw(a.get_float() / b.get_float())
                    } else {
                        let fa = if a.is_float() {
                            a.get_float()
                        } else if a.is_int() {
                            a.get_int() as f64
                        } else {
                            Self::js_to_number(&a, ctx)
                        };
                        let fb = if b.is_float() {
                            b.get_float()
                        } else if b.is_int() {
                            b.get_int() as f64
                        } else {
                            Self::js_to_number(&b, ctx)
                        };
                        if fb != 0.0 {
                            JSValue::new_float(fa / fb)
                        } else {
                            JSValue::new_float(f64::NAN)
                        }
                    };
                    self.set_reg(dst, result);
                }
                Opcode::Sub => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        let v = a.get_int() - b.get_int();
                        if v == 0 && a.get_int() < b.get_int() {
                            JSValue::new_float_raw(-0.0f64)
                        } else {
                            JSValue::new_int(v)
                        }
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        Self::create_bigint(a_int - b_int)
                    } else if a.is_float() && b.is_float() {
                        JSValue::new_float_raw(a.get_float() - b.get_float())
                    } else if a.is_int() && b.is_float() {
                        JSValue::new_float_raw(a.get_int() as f64 - b.get_float())
                    } else if a.is_float() && b.is_int() {
                        JSValue::new_float_raw(a.get_float() - b.get_int() as f64)
                    } else {
                        let fa = Self::js_to_number(&a, ctx);
                        let fb = Self::js_to_number(&b, ctx);
                        JSValue::new_float_raw(fa - fb)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::SubImm8 => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let imm = self.read_u8() as i8 as i64;
                    let a = self.get_reg(a_reg);
                    let result = if a.is_int() {
                        JSValue::new_int(a.get_int() - imm)
                    } else if a.is_float() {
                        JSValue::new_float(a.get_float() - imm as f64)
                    } else {
                        JSValue::new_float(Self::js_to_number(&a, ctx) - imm as f64)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::Mul => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        let ai = a.get_int();
                        let bi = b.get_int();

                        match ai.checked_mul(bi) {
                            Some(prod) if prod >= -(1i64 << 46) && prod < (1i64 << 46) => {
                                JSValue::new_int(prod)
                            }
                            _ => JSValue::new_float(ai as f64 * bi as f64),
                        }
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        Self::create_bigint(a_int * b_int)
                    } else if a.is_float() && b.is_float() {
                        JSValue::new_float(a.get_float() * b.get_float())
                    } else if a.is_int() && b.is_float() {
                        JSValue::new_float(a.get_int() as f64 * b.get_float())
                    } else if a.is_float() && b.is_int() {
                        JSValue::new_float(a.get_float() * b.get_int() as f64)
                    } else {
                        let fa = Self::js_to_number(&a, ctx);
                        let fb = Self::js_to_number(&b, ctx);
                        JSValue::new_float(fa * fb)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::Div => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        let ai = a.get_int();
                        let bi = b.get_int();
                        if bi != 0 && ai % bi == 0 {
                            JSValue::new_int(ai / bi)
                        } else if bi != 0 {
                            JSValue::new_float(ai as f64 / bi as f64)
                        } else {
                            JSValue::new_float(ai as f64 / 0.0f64)
                        }
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        if b_int != 0 {
                            Self::create_bigint(a_int / b_int)
                        } else {
                            JSValue::new_float(f64::NAN)
                        }
                    } else if a.is_float() && b.is_float() {
                        JSValue::new_float(a.get_float() / b.get_float())
                    } else if a.is_int() && b.is_float() {
                        JSValue::new_float(a.get_int() as f64 / b.get_float())
                    } else if a.is_float() && b.is_int() {
                        let bi = b.get_int();
                        if bi != 0 {
                            JSValue::new_float(a.get_float() / bi as f64)
                        } else {
                            JSValue::new_float(a.get_float() / 0.0f64)
                        }
                    } else {
                        let fa = Self::js_to_number(&a, ctx);
                        let fb = Self::js_to_number(&b, ctx);
                        JSValue::new_float(fa / fb)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::Mod => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        let bi = b.get_int();
                        if bi != 0 {
                            JSValue::new_int(a.get_int() % bi)
                        } else {
                            JSValue::new_float(f64::NAN)
                        }
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        if b_int != 0 {
                            Self::create_bigint(a_int % b_int)
                        } else {
                            JSValue::new_float(f64::NAN)
                        }
                    } else {
                        let bf = Self::js_to_number(&b, ctx);
                        if bf != 0.0 {
                            JSValue::new_float(Self::js_to_number(&a, ctx) % bf)
                        } else {
                            JSValue::new_float(f64::NAN)
                        }
                    };
                    self.set_reg(dst, result);
                }
                Opcode::Pow => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = JSValue::new_float(a.to_number().powf(b.to_number()));
                    self.set_reg(dst, result);
                }
                Opcode::BitAnd => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        Self::create_bigint(a_int & b_int)
                    } else if JSValue::both_int(&a, &b) {
                        JSValue::new_int((a.get_int() as i32 & b.get_int() as i32) as i64)
                    } else {
                        let na = if a.is_int() {
                            a.get_int() as f64
                        } else {
                            Self::js_to_number(&a, ctx)
                        };
                        let nb = if b.is_int() {
                            b.get_int() as f64
                        } else {
                            Self::js_to_number(&b, ctx)
                        };
                        let ia = Self::to_int32(na) as i64;
                        let ib = Self::to_int32(nb) as i64;
                        JSValue::new_int(ia & ib)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::BitOr => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        Self::create_bigint(a_int | b_int)
                    } else if JSValue::both_int(&a, &b) {
                        JSValue::new_int((a.get_int() as i32 | b.get_int() as i32) as i64)
                    } else {
                        let na = if a.is_int() {
                            a.get_int() as f64
                        } else {
                            Self::js_to_number(&a, ctx)
                        };
                        let nb = if b.is_int() {
                            b.get_int() as f64
                        } else {
                            Self::js_to_number(&b, ctx)
                        };
                        let ia = Self::to_int32(na) as i64;
                        let ib = Self::to_int32(nb) as i64;
                        JSValue::new_int(ia | ib)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::BitXor => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        Self::create_bigint(a_int ^ b_int)
                    } else if JSValue::both_int(&a, &b) {
                        JSValue::new_int((a.get_int() as i32 ^ b.get_int() as i32) as i64)
                    } else {
                        let na = if a.is_int() {
                            a.get_int() as f64
                        } else {
                            Self::js_to_number(&a, ctx)
                        };
                        let nb = if b.is_int() {
                            b.get_int() as f64
                        } else {
                            Self::js_to_number(&b, ctx)
                        };
                        let ia = Self::to_int32(na) as i64;
                        let ib = Self::to_int32(nb) as i64;
                        JSValue::new_int(ia ^ ib)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::BitNot => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let mut a = self.get_reg(a_reg);
                    if a.is_object() || a.is_function() {
                        a = self.ordinary_to_primitive(&a, "number", ctx);
                        if let Some(exc) = self.pending_throw.take() {
                            match self.dispatch_throw_value(ctx, exc) {
                                ThrowDispatch::Caught => continue,
                                ThrowDispatch::Uncaught(e) => return Err(e),
                                ThrowDispatch::AsyncComplete(_) => continue,
                            }
                        }
                    }
                    let result = if a.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        Self::create_bigint(!a_int)
                    } else {
                        let na = if a.is_int() {
                            a.get_int() as f64
                        } else {
                            Self::js_to_number(&a, ctx)
                        };
                        let i32_val = Self::to_int32(na);
                        JSValue::new_int(!(i32_val as i64))
                    };
                    self.set_reg(dst, result);
                }
                Opcode::Shl => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        let shift = b.get_int() & 0x1f;
                        JSValue::new_int(a.get_int() << shift)
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        Self::create_bigint(a_int << b_int)
                    } else {
                        let nb = Self::js_to_number(&b, ctx);
                        let shift = if nb.is_nan() || nb.is_infinite() {
                            0
                        } else {
                            (nb as i64) & 0x1f
                        };
                        let na = Self::js_to_number(&a, ctx);
                        let ia = if na.is_nan() || na.is_infinite() {
                            0
                        } else {
                            na as i64
                        };
                        JSValue::new_int(ia << shift)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::Shr => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        let shift = b.get_int() & 0x1f;
                        JSValue::new_int(a.get_int() >> shift)
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        Self::create_bigint(a_int >> b_int)
                    } else {
                        let nb = Self::js_to_number(&b, ctx);
                        let shift = if nb.is_nan() || nb.is_infinite() {
                            0
                        } else {
                            (nb as i64) & 0x1f
                        };
                        let na = Self::js_to_number(&a, ctx);
                        let ia = if na.is_nan() || na.is_infinite() {
                            0
                        } else {
                            na as i64
                        };
                        JSValue::new_int(ia >> shift)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::UShr => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);

                    if JSValue::both_int(&a, &b) {
                        let a_u32 = (a.get_int() as u64 & 0xffffffff) as u32;
                        let shift = (b.get_int() & 0x1f) as u32;
                        self.set_reg(dst, JSValue::new_int((a_u32 >> shift) as i64));
                        continue;
                    }
                    let nb = Self::js_to_number(&b, ctx);
                    let shift = if nb.is_nan() || nb.is_infinite() {
                        0
                    } else {
                        (nb as i64) & 0x1f
                    };
                    let a_u32 = if a.is_int() {
                        (a.get_int() as u64 & 0xffffffff) as u32
                    } else {
                        let n = Self::js_to_number(&a, ctx);
                        if n.is_nan() || n.is_infinite() {
                            0u32
                        } else {
                            let n = n.trunc();
                            if n >= 0.0 {
                                (n as u64 % (1u64 << 32)) as u32
                            } else {
                                let m = (-n) as u64 % (1u64 << 32);
                                if m == 0 {
                                    0u32
                                } else {
                                    ((1u64 << 32) - m) as u32
                                }
                            }
                        }
                    };
                    let result = JSValue::new_int((a_u32 >> shift as u32) as i64);
                    self.set_reg(dst, result);
                }
                Opcode::Neg => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let result = if a.is_int() {
                        let v = a.get_int();
                        if v == 0 {
                            JSValue::new_float_raw(-0.0f64)
                        } else {
                            JSValue::new_int(-v)
                        }
                    } else if a.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        Self::create_bigint(-a_int)
                    } else if a.is_float() {
                        JSValue::new_float(-a.get_float())
                    } else {
                        JSValue::new_float(f64::NAN)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::Pos => {
                    let dst = self.read_u16_pc();
                    let src = self.read_u16_pc();
                    let val = self.get_reg(src);

                    if val.is_int() || val.is_float() {
                        self.set_reg(dst, val);
                    } else {
                        self.set_reg(dst, JSValue::new_float(val.get_float()));
                    }
                }

                Opcode::Lt => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        JSValue::bool(a.get_int() < b.get_int())
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        JSValue::bool(a_int < b_int)
                    } else if a.is_string() && b.is_string() {
                        let sa = ctx.get_atom_str(a.get_atom());
                        let sb = ctx.get_atom_str(b.get_atom());
                        JSValue::bool(sa < sb)
                    } else {
                        JSValue::bool(a.to_number() < b.to_number())
                    };
                    self.set_reg(dst, result);
                }
                Opcode::Lte => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        JSValue::bool(a.get_int() <= b.get_int())
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        JSValue::bool(a_int <= b_int)
                    } else if a.is_string() && b.is_string() {
                        JSValue::bool(a.get_int() <= b.get_int())
                    } else {
                        JSValue::bool(a.to_number() <= b.to_number())
                    };
                    self.set_reg(dst, result);
                }
                Opcode::LteImm8 => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let imm = self.read_u8() as i8 as i64;
                    let a = self.get_reg(a_reg);
                    let result = if a.is_int() {
                        JSValue::bool(a.get_int() <= imm)
                    } else if a.is_float() {
                        JSValue::bool(a.get_float() <= imm as f64)
                    } else {
                        JSValue::bool(Self::js_to_number(&a, ctx) <= imm as f64)
                    };
                    self.set_reg(dst, result);
                }
                Opcode::Gt => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        JSValue::bool(a.get_int() > b.get_int())
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        JSValue::bool(a_int > b_int)
                    } else if a.is_string() && b.is_string() {
                        let sa = ctx.get_atom_str(a.get_atom());
                        let sb = ctx.get_atom_str(b.get_atom());
                        JSValue::bool(sa > sb)
                    } else {
                        JSValue::bool(a.to_number() > b.to_number())
                    };
                    self.set_reg(dst, result);
                }
                Opcode::Gte => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let result = if JSValue::both_int(&a, &b) {
                        JSValue::bool(a.get_int() >= b.get_int())
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        JSValue::bool(a_int >= b_int)
                    } else if a.is_string() && b.is_string() {
                        let sa = ctx.get_atom_str(a.get_atom());
                        let sb = ctx.get_atom_str(b.get_atom());
                        JSValue::bool(sa >= sb)
                    } else {
                        JSValue::bool(a.to_number() >= b.to_number())
                    };
                    self.set_reg(dst, result);
                }
                Opcode::Eq => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    self.set_reg(dst, JSValue::bool(loose_equal(ctx, a, b)));
                }
                Opcode::Neq => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    self.set_reg(dst, JSValue::bool(!loose_equal(ctx, a, b)));
                }
                Opcode::StrictEq => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    self.set_reg(dst, JSValue::bool(a.strict_eq(&b)));
                }
                Opcode::StrictNeq => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    self.set_reg(dst, JSValue::bool(!a.strict_eq(&b)));
                }
                Opcode::Not => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    self.set_reg(dst, JSValue::bool(!a.is_truthy()));
                }
                Opcode::TypeOf => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let ca = &ctx.common_atoms;
                    let atom = if a.is_undefined() {
                        ca.typeof_undefined
                    } else if a.is_null() {
                        ca.typeof_object
                    } else if a.is_bool() {
                        ca.typeof_boolean
                    } else if a.is_int() || a.is_float() {
                        ca.typeof_number
                    } else if a.is_string() {
                        ca.typeof_string
                    } else if a.is_symbol() {
                        ca.typeof_symbol
                    } else if a.is_bigint() {
                        ca.typeof_bigint
                    } else if a.is_function() {
                        ca.typeof_function
                    } else if a.is_object() {
                        let obj = a.as_object();
                        if obj.get(ctx.common_atoms.__boundFn).is_some() {
                            ca.typeof_function
                        } else {
                            ca.typeof_object
                        }
                    } else {
                        ca.typeof_object
                    };
                    self.set_reg(dst, JSValue::new_string(atom));
                }

                Opcode::MathSin
                | Opcode::MathCos
                | Opcode::MathSqrt
                | Opcode::MathAbs
                | Opcode::MathFloor
                | Opcode::MathCeil
                | Opcode::MathRound => {
                    let dst = self.read_u16_pc();
                    let src = self.read_u16_pc();
                    let a = self.get_reg(src);

                    if a.is_int()
                        && matches!(
                            op,
                            Opcode::MathFloor
                                | Opcode::MathCeil
                                | Opcode::MathRound
                                | Opcode::MathAbs
                        )
                    {
                        let result = if matches!(op, Opcode::MathAbs) {
                            let i = a.get_int();
                            if i == i64::MIN {
                                JSValue::new_float(-(i64::MIN as f64))
                            } else if i < 0 {
                                JSValue::new_int(-i)
                            } else {
                                a
                            }
                        } else {
                            a
                        };
                        self.set_reg(dst, result);
                        continue;
                    }
                    let f = if a.is_int() {
                        a.get_int() as f64
                    } else if a.is_float() {
                        a.get_float()
                    } else {
                        Self::js_to_number(&a, ctx)
                    };
                    let result = match op {
                        Opcode::MathSin => f.sin(),
                        Opcode::MathCos => f.cos(),
                        Opcode::MathSqrt => f.sqrt(),
                        Opcode::MathAbs => f.abs(),
                        Opcode::MathFloor => {
                            let floored = f.floor();

                            const MAX_SAFE: f64 = (1i64 << 47) as f64;
                            if floored >= -MAX_SAFE && floored <= MAX_SAFE {
                                self.set_reg(dst, JSValue::new_int(floored as i64));
                                continue;
                            }
                            floored
                        }
                        Opcode::MathCeil => {
                            let ceiled = f.ceil();
                            const MAX_SAFE: f64 = (1i64 << 47) as f64;
                            if ceiled >= -MAX_SAFE && ceiled <= MAX_SAFE {
                                self.set_reg(dst, JSValue::new_int(ceiled as i64));
                                continue;
                            }
                            ceiled
                        }
                        Opcode::MathRound => {
                            let rounded = f.round();
                            const MAX_SAFE: f64 = (1i64 << 47) as f64;
                            if rounded >= -MAX_SAFE && rounded <= MAX_SAFE {
                                self.set_reg(dst, JSValue::new_int(rounded as i64));
                                continue;
                            }
                            rounded
                        }
                        _ => unreachable!("unhandled math op"),
                    };
                    self.set_reg(dst, JSValue::new_float(result));
                }
                Opcode::MathPow => {
                    let dst = self.read_u16_pc();
                    let base_reg = self.read_u16_pc();
                    let exp_reg = self.read_u16_pc();
                    let base = self.get_reg(base_reg);
                    let exp = self.get_reg(exp_reg);
                    let b = if base.is_int() {
                        base.get_int() as f64
                    } else if base.is_float() {
                        base.get_float()
                    } else {
                        Self::js_to_number(&base, ctx)
                    };
                    let e = if exp.is_int() {
                        exp.get_int() as f64
                    } else if exp.is_float() {
                        exp.get_float()
                    } else {
                        Self::js_to_number(&exp, ctx)
                    };
                    self.set_reg(dst, JSValue::new_float(b.powf(e)));
                }
                Opcode::MathMin | Opcode::MathMax => {
                    let dst = self.read_u16_pc();
                    let a_reg = self.read_u16_pc();
                    let b_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b = self.get_reg(b_reg);
                    let fa = if a.is_int() {
                        a.get_int() as f64
                    } else if a.is_float() {
                        a.get_float()
                    } else {
                        Self::js_to_number(&a, ctx)
                    };
                    let fb = if b.is_int() {
                        b.get_int() as f64
                    } else if b.is_float() {
                        b.get_float()
                    } else {
                        Self::js_to_number(&b, ctx)
                    };
                    let result = if op == Opcode::MathMin {
                        fa.min(fb)
                    } else {
                        fa.max(fb)
                    };
                    self.set_reg(dst, JSValue::new_float(result));
                }

                Opcode::Jump => {
                    let offset = self.read_i32();
                    self.pc = (self.pc as i64 + offset as i64) as usize;
                }
                Opcode::JumpIf => {
                    let src = self.read_u16_pc();
                    let offset = self.read_i32();
                    if self.get_reg(src).is_truthy() {
                        self.pc = (self.pc as i64 + offset as i64) as usize;
                    }
                }
                Opcode::JumpIfNot => {
                    let src = self.read_u16_pc();
                    let offset = self.read_i32();
                    if !self.get_reg(src).is_truthy() {
                        self.pc = (self.pc as i64 + offset as i64) as usize;
                    }
                }
                Opcode::JumpIfNullish => {
                    let src = self.read_u16_pc();
                    let offset = self.read_i32();
                    let v = self.get_reg(src);
                    if v.is_null() || v.is_undefined() {
                        self.pc = (self.pc as i64 + offset as i64) as usize;
                    }
                }
                Opcode::Jump8 => {
                    let offset = self.read_u8() as i8;
                    self.pc = (self.pc as i64 + offset as i64) as usize;
                }
                Opcode::JumpIf8 => {
                    let src = self.read_u16_pc();
                    let offset = self.read_u8() as i8;
                    if self.get_reg(src).is_truthy() {
                        self.pc = (self.pc as i64 + offset as i64) as usize;
                    }
                }
                Opcode::JumpIfNot8 => {
                    let src = self.read_u16_pc();
                    let offset = self.read_u8() as i8;
                    if !self.get_reg(src).is_truthy() {
                        self.pc = (self.pc as i64 + offset as i64) as usize;
                    }
                }
                Opcode::Throw => {
                    let src = self.read_u16_pc();
                    let value = self.get_reg(src);

                    let find_async_frame = |frames: &[CallFrame], frame_index: usize| {
                        for i in (0..=frame_index).rev() {
                            if frames[i].is_async {
                                return Some(i);
                            }
                        }
                        None
                    };

                    if let Some(handler) = self.exception_handlers.last().cloned() {
                        self.exception_handlers.pop();

                        if self.frame_index != handler.frame_index {
                            if let Some(async_idx) =
                                find_async_frame(&self.frames, self.frame_index)
                            {
                                if async_idx > handler.frame_index {
                                    while self.frame_index > async_idx {
                                        if self.frame_index == 0 {
                                            break;
                                        }
                                        self.frame_index -= 1;
                                    }

                                    let rejected = ctx.call_builtin("promise_reject", &[value]);
                                    if self.frame_index == 0 {
                                        return Ok(ExecutionOutcome::Complete(rejected));
                                    }
                                    let return_pc = self.frames[self.frame_index].return_pc;
                                    let dst_reg = self.frames[self.frame_index].dst_reg;
                                    self.frame_index -= 1;
                                    self.pc = return_pc;
                                    self.refresh_cache();
                                    self.set_reg(dst_reg, rejected);
                                    continue;
                                }
                            }
                            while self.frame_index > handler.frame_index {
                                if self.frame_index == 0 {
                                    break;
                                }
                                let return_pc = self.frames[self.frame_index].return_pc;
                                let _ = return_pc;
                                self.frame_index -= 1;
                            }
                        }

                        self.pc = handler.catch_pc;
                        self.refresh_cache();
                        self.set_reg(0, value);
                    } else {
                        if let Some(async_idx) = find_async_frame(&self.frames, self.frame_index) {
                            let rejected = ctx.call_builtin("promise_reject", &[value]);
                            if async_idx == 0 {
                                return Ok(ExecutionOutcome::Complete(rejected));
                            }
                            while self.frame_index > async_idx {
                                self.frame_index -= 1;
                            }
                            let return_pc = self.frames[self.frame_index].return_pc;
                            let dst_reg = self.frames[self.frame_index].dst_reg;
                            self.frame_index -= 1;
                            self.pc = return_pc;
                            self.refresh_cache();
                            self.set_reg(dst_reg, rejected);
                            continue;
                        }
                        self.pending_throw = Some(value);
                        let msg = self.format_thrown_value(&value, ctx);
                        let trace = self.format_stack_trace(ctx);
                        return Err(format!("Uncaught: {}\nStack trace:\n{}", msg, trace));
                    }
                }
                Opcode::Try => {
                    let catch_pc = self.read_i32() as usize;
                    let finally_pc = self.read_i32() as usize;
                    self.exception_handlers.push(ExceptionHandler {
                        frame_index: self.frame_index,
                        catch_pc,
                        finally_pc: if finally_pc > 0 {
                            Some(finally_pc)
                        } else {
                            None
                        },
                    });
                }
                Opcode::Catch => {
                    self.exception_handlers.pop();
                }
                Opcode::Finally => {
                    if let Some(exc) = self.finally_rethrow.take() {
                        let value = exc;
                        if let Some(handler) = self.exception_handlers.last().cloned() {
                            self.exception_handlers.pop();
                            if self.frame_index != handler.frame_index {
                                while self.frame_index > handler.frame_index {
                                    if self.frame_index == 0 {
                                        break;
                                    }
                                    self.frame_index -= 1;
                                }
                            }
                            self.pc = handler.catch_pc;
                            self.refresh_cache();
                            self.set_reg(0, value);
                            continue;
                        }
                        self.pending_throw = Some(value);
                        let msg = self.format_thrown_value(&value, ctx);
                        let trace = self.format_stack_trace(ctx);
                        return Err(format!("Uncaught: {}\nStack trace:\n{}", msg, trace));
                    }
                }

                Opcode::GetLocal => {
                    let dst = self.read_u16_pc();
                    let idx = self.read_u32() as u16;
                    self.set_reg(dst, self.get_reg(idx));
                }
                Opcode::SetLocal => {
                    let idx = self.read_u32() as u16;
                    let src = self.read_u16_pc();
                    let val = self.get_reg(src);
                    self.set_reg(idx, val);
                }
                Opcode::IncLocal => {
                    let slot = self.read_u16_pc();
                    let val = self.get_reg(slot);
                    let result = if val.is_int() {
                        JSValue::new_int(val.get_int() + 1)
                    } else if val.is_bigint() {
                        let big = Self::get_bigint_int(&val).unwrap_or(0);
                        Self::create_bigint(big + 1)
                    } else if val.is_float() {
                        JSValue::new_float(val.get_float() + 1.0)
                    } else {
                        JSValue::new_float(Self::js_to_number(&val, ctx) + 1.0)
                    };
                    self.set_reg(slot, result);
                }
                Opcode::DecLocal => {
                    let slot = self.read_u16_pc();
                    let val = self.get_reg(slot);
                    let result = if val.is_int() {
                        JSValue::new_int(val.get_int() - 1)
                    } else if val.is_bigint() {
                        let big = Self::get_bigint_int(&val).unwrap_or(0);
                        Self::create_bigint(big - 1)
                    } else if val.is_float() {
                        JSValue::new_float(val.get_float() - 1.0)
                    } else {
                        JSValue::new_float(Self::js_to_number(&val, ctx) - 1.0)
                    };
                    self.set_reg(slot, result);
                }
                Opcode::GetGlobal => {
                    let ic_pc = self.pc - 1;
                    let dst = self.read_u16_pc();
                    let idx = self.read_u32() as usize;
                    let name = if idx < self.frames[self.frame_index].constants_len {
                        unsafe { *self.cached_const_ptr.add(idx) }
                    } else {
                        JSValue::undefined()
                    };
                    let atom = name.get_atom();
                    let mut resolved = false;
                    if self.eval_binding_frames > 0 {
                        if let Some(val) =
                            Self::scan_eval_bindings(&self.frames, self.frame_index, atom.0)
                        {
                            self.set_reg(dst, val);
                            resolved = true;
                        }
                    }
                    if !resolved {
                        if let Some(val) = self.get_var_in_caller_vm(atom.0) {
                            self.set_reg(dst, val);
                        } else {
                            let global = ctx.global();
                            let mut result = JSValue::undefined();
                            if global.is_object() {
                                let global_obj = global.as_object();

                                let global_shape_id = global_obj.shape_id_cache;
                                let ic_table_ptr = self.cached_ic_table_ptr;
                                let mut ic_hit = false;
                                if !ic_table_ptr.is_null() {
                                    if let Some(offset) = unsafe {
                                        (*ic_table_ptr).get_global_cache(
                                            ic_pc,
                                            global_shape_id,
                                            atom.0,
                                        )
                                    } {
                                        result = global_obj
                                            .get_by_offset(offset as usize)
                                            .unwrap_or(JSValue::undefined());
                                        ic_hit = true;
                                    }
                                }
                                if !ic_hit {
                                    if let Some(accessor) = global_obj.get_own_accessor_entry(atom)
                                    {
                                        if let Some(getter) = accessor.get {
                                            if getter.is_function() {
                                                match self.call_function_with_this(
                                                    ctx,
                                                    getter,
                                                    global,
                                                    &[],
                                                ) {
                                                    Ok(v) => result = v,
                                                    Err(msg) => {
                                                        self.set_pending_type_error(ctx, &msg);
                                                        if let Some(exc) = self.pending_throw.take()
                                                        {
                                                            match self
                                                                .dispatch_throw_value(ctx, exc)
                                                            {
                                                                ThrowDispatch::Caught => {}
                                                                ThrowDispatch::Uncaught(e) => {
                                                                    return Err(e);
                                                                }
                                                                ThrowDispatch::AsyncComplete(o) => {
                                                                    return Ok(o);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if let Some(value) = global_obj.get(atom) {
                                        result = value;

                                        if !ic_table_ptr.is_null() {
                                            if let Some(offset) = global_obj.find_offset(atom) {
                                                unsafe {
                                                    (*ic_table_ptr).insert_global_cache(
                                                        ic_pc,
                                                        global_shape_id,
                                                        offset as u32,
                                                        atom.0,
                                                    );
                                                }
                                            }
                                        }
                                    } else if let Some(func_ptr) =
                                        self.frames[self.frame_index].function_ptr
                                    {
                                        let func_val = JSValue::new_function(func_ptr);
                                        let js_func = func_val.as_function();
                                        if js_func.name == atom {
                                            result = JSValue::new_function(func_ptr);
                                        }
                                    }
                                }
                            }
                            self.set_reg(dst, result);
                        }
                    }
                }
                Opcode::SetGlobal => {
                    let idx = self.read_u32() as usize;
                    let src = self.read_u16_pc();
                    let val = self.get_reg(src);
                    let name = if idx < self.frames[self.frame_index].constants_len {
                        unsafe { *self.cached_const_ptr.add(idx) }
                    } else {
                        JSValue::undefined()
                    };
                    let atom = name.get_atom();
                    let is_strict = self.frames[self.frame_index].is_strict_frame;

                    if self.caller_vm.is_some() {
                        if !self.set_var_in_caller_vm(ctx, atom.0, val) {
                            if is_strict {
                                let name_str = ctx.get_atom_str(atom).to_string();
                                let err_msg = format!("{} is not defined", name_str);
                                let mut err = crate::object::object::JSObject::new();
                                err.set(
                                    ctx.intern("name"),
                                    JSValue::new_string(ctx.intern("ReferenceError")),
                                );
                                err.set(
                                    ctx.intern("message"),
                                    JSValue::new_string(ctx.intern(&err_msg)),
                                );
                                if let Some(proto) = ctx.get_reference_error_prototype() {
                                    err.prototype = Some(proto);
                                }
                                let ptr = Box::into_raw(Box::new(err)) as usize;
                                ctx.runtime_mut().gc_heap_mut().track(ptr);
                                self.pending_throw = Some(JSValue::new_object(ptr));
                                if let Some(exc) = self.pending_throw.take() {
                                    match self.dispatch_throw_value(ctx, exc) {
                                        ThrowDispatch::Caught => continue,
                                        ThrowDispatch::Uncaught(e) => return Err(e),
                                        ThrowDispatch::AsyncComplete(o) => return Ok(o),
                                    }
                                }
                            }
                            let global = ctx.global();
                            if global.is_object() {
                                let global_obj = global.as_object_mut();
                                global_obj.set_cached(atom, val, ctx.shape_cache_mut());
                            }
                        }
                    } else {
                        if is_strict {
                            let mut found = false;

                            if self.eval_binding_frames == 0 && self.caller_vm.is_none() {
                                let global = ctx.global();
                                if global.is_object() {
                                    found = global.as_object().get_own(atom).is_some();
                                }

                                if !found && !self.frames[0].var_name_map.is_null() {
                                    let vnm = unsafe { &*self.frames[0].var_name_map };
                                    found = vnm.iter().any(|&(an, _)| an == atom.0);
                                }
                            } else if self.eval_binding_frames > 0 {
                                for fi in (0..=self.frame_index).rev() {
                                    if let Some(ref eb) = self.frames[fi].eval_bindings {
                                        if eb.contains_key(&atom.0) {
                                            found = true;
                                            break;
                                        }
                                    }
                                    if !self.frames[fi].var_name_map.is_null() {
                                        let vnm = unsafe { &*self.frames[fi].var_name_map };
                                        if vnm.iter().any(|&(an, _)| an == atom.0) {
                                            found = true;
                                            break;
                                        }
                                    }
                                }
                                if !found {
                                    let global = ctx.global();
                                    if global.is_object() {
                                        found = global.as_object().get_own(atom).is_some();
                                    }
                                }
                            } else {
                                if self.get_var_in_caller_vm(atom.0).is_some() {
                                    found = true;
                                }
                                if !found {
                                    let global = ctx.global();
                                    if global.is_object() {
                                        found = global.as_object().get_own(atom).is_some();
                                    }
                                }
                            }
                            if !found {
                                let name_str = ctx.get_atom_str(atom).to_string();
                                let err_msg = format!("{} is not defined", name_str);
                                let mut err = crate::object::object::JSObject::new();
                                err.set(
                                    ctx.intern("name"),
                                    JSValue::new_string(ctx.intern("ReferenceError")),
                                );
                                err.set(
                                    ctx.intern("message"),
                                    JSValue::new_string(ctx.intern(&err_msg)),
                                );
                                if let Some(proto) = ctx.get_reference_error_prototype() {
                                    err.prototype = Some(proto);
                                }
                                let ptr = Box::into_raw(Box::new(err)) as usize;
                                ctx.runtime_mut().gc_heap_mut().track(ptr);
                                self.pending_throw = Some(JSValue::new_object(ptr));
                                if let Some(exc) = self.pending_throw.take() {
                                    match self.dispatch_throw_value(ctx, exc) {
                                        ThrowDispatch::Caught => continue,
                                        ThrowDispatch::Uncaught(e) => return Err(e),
                                        ThrowDispatch::AsyncComplete(o) => return Ok(o),
                                    }
                                }
                            }
                        }
                        let global = ctx.global();
                        if global.is_object() {
                            let global_obj = global.as_object_mut();
                            global_obj.set_cached(atom, val, ctx.shape_cache_mut());
                        }
                    }
                }
                Opcode::SetGlobalVar => {
                    let idx = self.read_u32() as usize;
                    let src = self.read_u16_pc();
                    let val = self.get_reg(src);
                    let name = if idx < self.frames[self.frame_index].constants_len {
                        unsafe { *self.cached_const_ptr.add(idx) }
                    } else {
                        JSValue::undefined()
                    };
                    let atom = name.get_atom();
                    let is_strict = self.frames[self.frame_index].is_strict_frame;
                    if self.caller_vm.is_some() {
                        if !self.set_var_in_caller_vm(ctx, atom.0, val) {
                            if is_strict {
                                let name_str = ctx.get_atom_str(atom).to_string();
                                let err_msg = format!("{} is not defined", name_str);
                                let mut err = crate::object::object::JSObject::new();
                                err.set(
                                    ctx.intern("name"),
                                    JSValue::new_string(ctx.intern("ReferenceError")),
                                );
                                err.set(
                                    ctx.intern("message"),
                                    JSValue::new_string(ctx.intern(&err_msg)),
                                );
                                if let Some(proto) = ctx.get_reference_error_prototype() {
                                    err.prototype = Some(proto);
                                }
                                let ptr = Box::into_raw(Box::new(err)) as usize;
                                ctx.runtime_mut().gc_heap_mut().track(ptr);
                                self.pending_throw = Some(JSValue::new_object(ptr));
                                if let Some(exc) = self.pending_throw.take() {
                                    match self.dispatch_throw_value(ctx, exc) {
                                        ThrowDispatch::Caught => continue,
                                        ThrowDispatch::Uncaught(e) => return Err(e),
                                        ThrowDispatch::AsyncComplete(o) => return Ok(o),
                                    }
                                }
                            }
                            let global = ctx.global();
                            if global.is_object() {
                                let global_obj = global.as_object_mut();
                                global_obj.set_cached_non_configurable(
                                    atom,
                                    val,
                                    ctx.shape_cache_mut(),
                                );
                            }
                        }
                    } else {
                        if is_strict {
                            let mut found = false;

                            if self.eval_binding_frames == 0 && self.caller_vm.is_none() {
                                let global = ctx.global();
                                if global.is_object() {
                                    found = global.as_object().get_own(atom).is_some();
                                }
                                if !found && !self.frames[0].var_name_map.is_null() {
                                    let vnm = unsafe { &*self.frames[0].var_name_map };
                                    found = vnm.iter().any(|&(an, _)| an == atom.0);
                                }
                            } else if self.eval_binding_frames > 0 {
                                for fi in (0..=self.frame_index).rev() {
                                    if let Some(ref eb) = self.frames[fi].eval_bindings {
                                        if eb.contains_key(&atom.0) {
                                            found = true;
                                            break;
                                        }
                                    }
                                    if !self.frames[fi].var_name_map.is_null() {
                                        let vnm = unsafe { &*self.frames[fi].var_name_map };
                                        if vnm.iter().any(|&(an, _)| an == atom.0) {
                                            found = true;
                                            break;
                                        }
                                    }
                                }
                                if !found {
                                    let global = ctx.global();
                                    if global.is_object() {
                                        found = global.as_object().get_own(atom).is_some();
                                    }
                                }
                            } else {
                                if self.get_var_in_caller_vm(atom.0).is_some() {
                                    found = true;
                                }
                                if !found {
                                    let global = ctx.global();
                                    if global.is_object() {
                                        found = global.as_object().get_own(atom).is_some();
                                    }
                                }
                            }
                            if !found {
                                let name_str = ctx.get_atom_str(atom).to_string();
                                let err_msg = format!("{} is not defined", name_str);
                                let mut err = crate::object::object::JSObject::new();
                                err.set(
                                    ctx.intern("name"),
                                    JSValue::new_string(ctx.intern("ReferenceError")),
                                );
                                err.set(
                                    ctx.intern("message"),
                                    JSValue::new_string(ctx.intern(&err_msg)),
                                );
                                if let Some(proto) = ctx.get_reference_error_prototype() {
                                    err.prototype = Some(proto);
                                }
                                let ptr = Box::into_raw(Box::new(err)) as usize;
                                ctx.runtime_mut().gc_heap_mut().track(ptr);
                                self.pending_throw = Some(JSValue::new_object(ptr));
                                if let Some(exc) = self.pending_throw.take() {
                                    match self.dispatch_throw_value(ctx, exc) {
                                        ThrowDispatch::Caught => continue,
                                        ThrowDispatch::Uncaught(e) => return Err(e),
                                        ThrowDispatch::AsyncComplete(o) => return Ok(o),
                                    }
                                }
                            }
                        }
                        let global = ctx.global();
                        if global.is_object() {
                            let global_obj = global.as_object_mut();
                            global_obj.set_cached_non_configurable(
                                atom,
                                val,
                                ctx.shape_cache_mut(),
                            );
                        }
                    }
                }
                Opcode::InitGlobalVar => {
                    let idx = self.read_u32() as usize;
                    let src = self.read_u16_pc();
                    let val = self.get_reg(src);
                    let name = if idx < self.frames[self.frame_index].constants_len {
                        unsafe { *self.cached_const_ptr.add(idx) }
                    } else {
                        JSValue::undefined()
                    };
                    let atom = name.get_atom();
                    if self.caller_vm.is_some() {
                        self.init_var_in_caller_vm(ctx, atom.0, val);
                    } else {
                        let global = ctx.global();
                        if global.is_object() {
                            let global_obj = global.as_object_mut();
                            if !global_obj.has_own(atom) {
                                global_obj.define_cached(atom, val, ctx.shape_cache_mut());
                            }
                        }
                    }
                }
                Opcode::DefineGlobal => {
                    let idx = self.read_u32() as usize;
                    let src = self.read_u16_pc();
                    let val = self.get_reg(src);
                    let name = if idx < self.frames[self.frame_index].constants_len {
                        unsafe { *self.cached_const_ptr.add(idx) }
                    } else {
                        JSValue::undefined()
                    };
                    let atom = name.get_atom();
                    let global = ctx.global();
                    if global.is_object() {
                        let global_obj = global.as_object_mut();
                        global_obj.define_cached(atom, val, ctx.shape_cache_mut());
                    }
                }
                Opcode::DeleteGlobal => {
                    let dst = self.read_u16_pc();
                    let idx = self.read_u32() as usize;
                    let name = if idx < self.frames[self.frame_index].constants_len {
                        unsafe { *self.cached_const_ptr.add(idx) }
                    } else {
                        JSValue::undefined()
                    };
                    let atom = name.get_atom();
                    let global = ctx.global();
                    let result = if global.is_object() {
                        let global_obj = global.as_object_mut();
                        let deleted = global_obj.delete(atom);
                        if deleted {
                            ctx.runtime_mut().gc_heap_mut().deleted_props_count += 1;
                        }
                        deleted
                    } else {
                        true
                    };
                    let is_strict = self.frames[self.frame_index].is_strict_frame;
                    if is_strict && !result {
                        self.set_pending_type_error(
                            ctx,
                            "Delete of unqualified identifier in strict mode",
                        );
                        if let Some(exc) = self.pending_throw.take() {
                            match self.dispatch_throw_value(ctx, exc) {
                                ThrowDispatch::Caught => {}
                                ThrowDispatch::Uncaught(e) => return Err(e),
                                ThrowDispatch::AsyncComplete(o) => return Ok(o),
                            }
                        }
                    }
                    self.set_reg(dst, JSValue::bool(result));
                }
                Opcode::ThrowReferenceError => {
                    let msg_idx = self.read_u32() as usize;
                    let msg_const = if msg_idx < self.frames[self.frame_index].constants_len {
                        unsafe { *self.cached_const_ptr.add(msg_idx) }
                    } else {
                        JSValue::undefined()
                    };
                    let err_msg = if msg_const.is_string() {
                        ctx.get_atom_str(msg_const.get_atom()).to_string()
                    } else {
                        String::new()
                    };
                    let mut err = crate::object::object::JSObject::new();
                    err.set(
                        ctx.intern("name"),
                        JSValue::new_string(ctx.intern("ReferenceError")),
                    );
                    err.set(
                        ctx.intern("message"),
                        JSValue::new_string(ctx.intern(&err_msg)),
                    );
                    if let Some(proto) = ctx.get_reference_error_prototype() {
                        err.prototype = Some(proto);
                    }
                    let ptr = Box::into_raw(Box::new(err)) as usize;
                    ctx.runtime_mut().gc_heap_mut().track(ptr);
                    self.pending_throw = Some(JSValue::new_object(ptr));
                    if let Some(exc) = self.pending_throw.take() {
                        match self.dispatch_throw_value(ctx, exc) {
                            ThrowDispatch::Caught => continue,
                            ThrowDispatch::Uncaught(e) => return Err(e),
                            ThrowDispatch::AsyncComplete(o) => return Ok(o),
                        }
                    }
                    continue;
                }
                Opcode::GetUpvalue => {
                    let dst = self.read_u16_pc();
                    let slot = self.read_u16_pc() as usize;
                    let result = if !self.cached_upvalue_slot_ptr.is_null()
                        && slot < self.cached_upvalues_len
                    {
                        let rc = unsafe { &*self.cached_upvalue_slot_ptr.add(slot) };
                        unsafe { *(*std::rc::Rc::as_ptr(rc)).as_ptr() }
                    } else {
                        JSValue::undefined()
                    };
                    self.set_reg(dst, result);
                }
                Opcode::SetUpvalue => {
                    let slot = self.read_u16_pc() as usize;
                    let src = self.read_u16_pc();
                    let val = self.get_reg(src);
                    if !self.cached_upvalue_slot_ptr.is_null() && slot < self.cached_upvalues_len {
                        let rc = unsafe { &*self.cached_upvalue_slot_ptr.add(slot) };
                        unsafe { (*std::rc::Rc::as_ptr(rc)).as_ptr().write(val) };
                    }
                }

                Opcode::NewObject => {
                    let dst = self.read_u16_pc();
                    let ptr = if let Some(proto_ptr) = ctx.get_object_prototype() {
                        if let Some(nursery_ptr) = ctx.runtime_mut().gc_heap_mut().alloc_object() {
                            unsafe {
                                (*nursery_ptr).prototype = Some(proto_ptr);
                                (*nursery_ptr).ensure_shape(ctx.shape_cache_mut());
                            }
                            nursery_ptr as usize
                        } else {
                            let mut obj = crate::object::object::JSObject::new();
                            obj.prototype = Some(proto_ptr);
                            obj.ensure_shape(ctx.shape_cache_mut());
                            let heap_ptr = Box::into_raw(Box::new(obj)) as usize;
                            ctx.runtime_mut().gc_heap_mut().track(heap_ptr);
                            heap_ptr
                        }
                    } else {
                        let mut obj = crate::object::object::JSObject::new();
                        obj.ensure_shape(ctx.shape_cache_mut());
                        let heap_ptr = Box::into_raw(Box::new(obj)) as usize;
                        ctx.runtime_mut().gc_heap_mut().track(heap_ptr);
                        heap_ptr
                    };
                    self.allocation_count += 1;
                    self.set_reg(dst, JSValue::new_object(ptr));
                    self.maybe_gc(ctx);
                }
                Opcode::NewArray => {
                    let dst = self.read_u16_pc();
                    let count = self.read_u16_pc();
                    let ptr = if let Some(proto_ptr) = ctx.get_array_prototype() {
                        if let Some(nursery_ptr) = ctx.runtime_mut().gc_heap_mut().alloc_array() {
                            unsafe {
                                (*nursery_ptr).header.set_prototype_raw(proto_ptr);
                                (*nursery_ptr).elements.reserve(count as usize);
                                (*nursery_ptr).header.ensure_shape(ctx.shape_cache_mut());
                            }
                            nursery_ptr as usize
                        } else {
                            let mut arr = crate::object::array_obj::JSArrayObject::with_capacity(
                                count as usize,
                            );
                            arr.header.set_prototype_raw(proto_ptr);
                            arr.header.ensure_shape(ctx.shape_cache_mut());
                            let heap_ptr = Box::into_raw(Box::new(arr)) as usize;
                            ctx.runtime_mut().gc_heap_mut().track_array(heap_ptr);
                            heap_ptr
                        }
                    } else {
                        let mut arr =
                            crate::object::array_obj::JSArrayObject::with_capacity(count as usize);
                        arr.header.ensure_shape(ctx.shape_cache_mut());
                        let heap_ptr = Box::into_raw(Box::new(arr)) as usize;
                        ctx.runtime_mut().gc_heap_mut().track_array(heap_ptr);
                        heap_ptr
                    };
                    self.allocation_count += 1;
                    self.set_reg(dst, JSValue::new_object(ptr));
                    self.maybe_gc(ctx);
                }
                Opcode::GetField => {
                    let dst = self.read_u16_pc();
                    let obj_reg = self.read_u16_pc();
                    let key_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let key_val = self.get_reg(key_reg);
                    let result = if obj_val.is_object_like() && key_val.is_int() {
                        if obj_val.is_object() {
                            let js_obj_check =
                                unsafe { JSValue::object_from_ptr(obj_val.get_ptr()) };
                            if js_obj_check.is_mapped_arguments() {
                                let fi = js_obj_check.mapped_args_frame_index();
                                let param_count = js_obj_check.mapped_args_param_count();
                                let idx = key_val.get_int();
                                if idx >= 0 && fi < self.frames.len() {
                                    let idx_u = idx as usize;
                                    if (idx as u32) < param_count {
                                        let base = self.frames[fi].registers_base;
                                        let reg_idx = base + 1 + idx_u;
                                        if reg_idx < self.registers.len() {
                                            self.set_reg(dst, self.registers[reg_idx]);
                                            continue;
                                        }
                                    } else {
                                        let saved = &self.frames[fi].saved_args;
                                        if idx_u < saved.len() {
                                            self.set_reg(dst, saved[idx_u]);
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                        let ptr = obj_val.get_ptr();
                        let js_obj = unsafe { JSValue::object_from_ptr(ptr) };
                        if js_obj.is_dense_array() {
                            let idx = key_val.get_int() as usize;
                            let arr = unsafe {
                                &*(ptr as *const crate::object::array_obj::JSArrayObject)
                            };
                            if idx < arr.elements.len() {
                                arr.elements[idx]
                            } else {
                                JSValue::undefined()
                            }
                        } else {
                            let idx = key_val.get_int();
                            if idx >= 0 {
                                if let Some(val) = js_obj.get_indexed(idx as usize) {
                                    self.set_reg(dst, val);
                                    continue;
                                }
                            }
                            let atom = self.int_atom(idx as usize, ctx);
                            js_obj.get(atom).unwrap_or(JSValue::undefined())
                        }
                    } else if obj_val.is_string() && key_val.is_int() {
                        let s = ctx.get_atom_str(obj_val.get_atom());
                        let idx = key_val.get_int();
                        if idx < 0 {
                            JSValue::undefined()
                        } else if let Some(ch) = s.chars().nth(idx as usize) {
                            let chs = ch.to_string();
                            JSValue::new_string(ctx.intern(&chs))
                        } else {
                            JSValue::undefined()
                        }
                    } else if obj_val.is_string() && key_val.is_string() {
                        let str_atom = obj_val.get_atom();
                        let key = ctx.get_atom_str(key_val.get_atom());
                        if key == "length" {
                            JSValue::new_int(ctx.string_char_count(str_atom) as i64)
                        } else if let Ok(idx) = key.parse::<usize>() {
                            let s = ctx.get_atom_str(str_atom);
                            if let Some(ch) = s.chars().nth(idx) {
                                let chs = ch.to_string();
                                JSValue::new_string(ctx.intern(&chs))
                            } else {
                                JSValue::undefined()
                            }
                        } else if let Some(proto_ptr) = ctx.get_string_prototype() {
                            let proto_obj = unsafe { &*proto_ptr };
                            proto_obj
                                .get(key_val.get_atom())
                                .unwrap_or(JSValue::undefined())
                        } else {
                            JSValue::undefined()
                        }
                    } else if obj_val.is_object_like() && key_val.is_string() {
                        let js_obj = unsafe { JSValue::object_from_ptr(obj_val.get_ptr()) };
                        let atom = key_val.get_atom();

                        if let Some(getter) = js_obj.get_own_accessor_value(atom) {
                            if getter.is_function() {
                                match self.call_function_with_this(ctx, getter, obj_val, &[]) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        let mut err = crate::object::object::JSObject::new();
                                        err.set(
                                            ctx.intern("name"),
                                            JSValue::new_string(ctx.intern("ReferenceError")),
                                        );
                                        err.set(
                                            ctx.intern("message"),
                                            JSValue::new_string(ctx.intern(&e)),
                                        );
                                        if let Some(proto) = ctx.get_reference_error_prototype() {
                                            err.prototype = Some(proto);
                                        }
                                        let ptr = Box::into_raw(Box::new(err)) as usize;
                                        ctx.runtime_mut().gc_heap_mut().track(ptr);
                                        let exc = JSValue::new_object(ptr);
                                        match self.dispatch_throw_value(ctx, exc) {
                                            ThrowDispatch::Caught => JSValue::undefined(),
                                            ThrowDispatch::Uncaught(_) => {
                                                return Err(e);
                                            }
                                            ThrowDispatch::AsyncComplete(o) => {
                                                return Ok(o);
                                            }
                                        }
                                    }
                                }
                            } else {
                                JSValue::undefined()
                            }
                        } else {
                            js_obj.get(atom).unwrap_or(JSValue::undefined())
                        }
                    } else if obj_val.is_object_like() && key_val.is_symbol() {
                        let js_obj = unsafe { JSValue::object_from_ptr(obj_val.get_ptr()) };
                        let sym_key =
                            crate::runtime::atom::Atom(0x40000000 | key_val.get_symbol_id());
                        js_obj.get(sym_key).unwrap_or(JSValue::undefined())
                    } else if obj_val.is_object_like() && key_val.is_float() {
                        let js_obj = unsafe { JSValue::object_from_ptr(obj_val.get_ptr()) };
                        let s = VM::js_to_string(&key_val, ctx);
                        let atom = s.get_atom();
                        js_obj.get(atom).unwrap_or(JSValue::undefined())
                    } else {
                        JSValue::undefined()
                    };
                    self.set_reg(dst, result);
                }
                Opcode::SetField => {
                    let obj_reg = self.read_u16_pc();
                    let key_reg = self.read_u16_pc();
                    let val_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let key_val = self.get_reg(key_reg);
                    let value = self.get_reg(val_reg);
                    if obj_val.is_object_like() && key_val.is_int() {
                        let ptr = obj_val.get_ptr();
                        let js_obj = unsafe { JSValue::object_from_ptr_mut(ptr) };
                        if js_obj.is_dense_array() {
                            let idx = key_val.get_int() as usize;
                            let arr = unsafe {
                                &mut *(ptr as *mut crate::object::array_obj::JSArrayObject)
                            };
                            if idx < arr.elements.len() {
                                arr.elements[idx] = value;
                            } else {
                                while arr.elements.len() < idx {
                                    arr.elements.push(JSValue::undefined());
                                }
                                arr.elements.push(value);
                                let len_atom = ctx.common_atoms.length;

                                let new_elements_len = arr.elements.len();
                                let old_len = arr
                                    .header
                                    .get(len_atom)
                                    .map(|v| if v.is_int() { v.get_int() as usize } else { 0 })
                                    .unwrap_or(0);
                                let new_len = new_elements_len.max(old_len);
                                if new_len != old_len {
                                    arr.header.set_length_ic(
                                        len_atom,
                                        JSValue::new_int(new_len as i64),
                                        ctx.shape_cache_mut(),
                                    );
                                }
                            }
                        } else {
                            let idx = key_val.get_int();
                            if idx >= 0 && js_obj.maybe_set_indexed(idx as usize, value) {
                            } else {
                                let atom = self.int_atom(idx as usize, ctx);
                                js_obj.set_cached(atom, value, ctx.shape_cache_mut());
                            }
                        }
                    } else if obj_val.is_object_like() && key_val.is_string() {
                        let js_obj = unsafe { JSValue::object_from_ptr_mut(obj_val.get_ptr()) };
                        let atom = key_val.get_atom();

                        let setter = js_obj.get_own_accessor_entry(atom).and_then(|e| e.set);
                        if let Some(s) = setter {
                            if s.is_function() {
                                match self.call_function_with_this(ctx, s, obj_val, &[value]) {
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                            }
                        } else {
                            js_obj.set_cached(atom, value, ctx.shape_cache_mut());
                        }
                    } else if obj_val.is_object_like() && key_val.is_float() {
                        let js_obj = unsafe { JSValue::object_from_ptr_mut(obj_val.get_ptr()) };
                        let s = VM::js_to_string(&key_val, ctx);
                        let atom = s.get_atom();

                        let setter = js_obj.get_own_accessor_entry(atom).and_then(|e| e.set);
                        if let Some(s) = setter {
                            if s.is_function() {
                                match self.call_function_with_this(ctx, s, obj_val, &[value]) {
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                            }
                        } else {
                            js_obj.set_cached(atom, value, ctx.shape_cache_mut());
                        }
                    } else if obj_val.is_object_like() && key_val.is_symbol() {
                        let js_obj = unsafe { JSValue::object_from_ptr_mut(obj_val.get_ptr()) };
                        let sym_key =
                            crate::runtime::atom::Atom(0x40000000 | key_val.get_symbol_id());

                        let setter = js_obj.get_own_accessor_entry(sym_key).and_then(|e| e.set);
                        if let Some(s) = setter {
                            if s.is_function() {
                                match self.call_function_with_this(ctx, s, obj_val, &[value]) {
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                            }
                        } else {
                            js_obj.set_cached(sym_key, value, ctx.shape_cache_mut());
                        }
                    }
                    self.set_reg(obj_reg, obj_val);
                }
                Opcode::GetProp => {
                    let dst = self.read_u16_pc();
                    let obj_reg = self.read_u16_pc();
                    let key_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let key_val = self.get_reg(key_reg);

                    if !obj_val.is_object_like()
                        && !obj_val.is_function()
                        && (obj_val.is_int()
                            || obj_val.is_float()
                            || obj_val.is_string()
                            || obj_val.is_bool())
                        && key_val.is_string()
                    {
                        let atom = key_val.get_atom();
                        let start_proto = if obj_val.is_string() {
                            ctx.get_string_prototype()
                        } else if obj_val.is_int() || obj_val.is_float() {
                            ctx.get_number_prototype()
                        } else {
                            ctx.get_object_prototype()
                        };
                        let mut current = start_proto;
                        let mut found = false;
                        while let Some(ptr) = current {
                            let pobj = unsafe { &*ptr };
                            if let Some(getter) = pobj.get_own_accessor_value(atom) {
                                if getter.is_function() {
                                    let r = self
                                        .call_function_with_this(ctx, getter, obj_val, &[])
                                        .unwrap_or(JSValue::undefined());
                                    self.set_reg(dst, r);
                                } else {
                                    self.set_reg(dst, JSValue::undefined());
                                }
                                found = true;
                                break;
                            }
                            if let Some(val) = pobj.get_own(atom) {
                                self.set_reg(dst, val);
                                found = true;
                                break;
                            }
                            current = pobj.prototype;
                        }
                        if !found {
                            self.set_reg(dst, JSValue::undefined());
                        }
                        continue;
                    }
                    let result = if key_val.is_string() {
                        if let Some(result) = self.get_named_prop_result(
                            ctx,
                            dst,
                            obj_val,
                            key_val.get_atom(),
                            instr_pc,
                        ) {
                            result
                        } else {
                            continue;
                        }
                    } else if key_val.is_symbol() {
                        if obj_val.is_object_like() {
                            let js_obj = unsafe { JSValue::object_from_ptr(obj_val.get_ptr()) };
                            let sym_key =
                                crate::runtime::atom::Atom(0x40000000 | key_val.get_symbol_id());
                            js_obj.get(sym_key).unwrap_or(JSValue::undefined())
                        } else {
                            JSValue::undefined()
                        }
                    } else if key_val.is_int() && obj_val.is_object_like() {
                        if obj_val.is_object() {
                            let idx = key_val.get_int();
                            let js_obj_check =
                                unsafe { JSValue::object_from_ptr(obj_val.get_ptr()) };
                            if js_obj_check.is_mapped_arguments() {
                                let fi = js_obj_check.mapped_args_frame_index();
                                let param_count = js_obj_check.mapped_args_param_count();
                                if idx >= 0 && fi < self.frames.len() {
                                    let idx_u = idx as usize;
                                    if (idx as u32) < param_count {
                                        let base = self.frames[fi].registers_base;
                                        let reg_idx = base + 1 + idx_u;
                                        if reg_idx < self.registers.len() {
                                            self.set_reg(dst, self.registers[reg_idx]);
                                            continue;
                                        }
                                    } else {
                                        let saved = &self.frames[fi].saved_args;
                                        if idx_u < saved.len() {
                                            self.set_reg(dst, saved[idx_u]);
                                            continue;
                                        }
                                    }
                                }
                            }

                            if idx >= 0 {
                                if let Some(val) = js_obj_check.get_indexed(idx as usize) {
                                    self.set_reg(dst, val);
                                    continue;
                                }
                            }
                        }
                        let atom = self.int_atom(key_val.get_int() as usize, ctx);
                        if let Some(result) =
                            self.get_named_prop_result(ctx, dst, obj_val, atom, instr_pc)
                        {
                            result
                        } else {
                            continue;
                        }
                    } else if key_val.is_float() && obj_val.is_object_like() {
                        let s = VM::js_to_string(&key_val, ctx);
                        let atom = s.get_atom();
                        if let Some(result) =
                            self.get_named_prop_result(ctx, dst, obj_val, atom, instr_pc)
                        {
                            result
                        } else {
                            continue;
                        }
                    } else {
                        JSValue::undefined()
                    };
                    self.set_reg(dst, result);
                }
                Opcode::GetNamedProp => {
                    let dst = self.read_u16_pc();
                    let obj_reg = self.read_u16_pc();
                    let atom = crate::runtime::atom::Atom(self.read_u32_pc());
                    if !self.get_named_prop_fast(ctx, dst, obj_reg, atom, instr_pc) {
                        if let Some(exc) = self.pending_throw.take() {
                            match self.dispatch_throw_value(ctx, exc) {
                                ThrowDispatch::Caught => continue,
                                ThrowDispatch::Uncaught(e) => return Err(e),
                                ThrowDispatch::AsyncComplete(o) => return Ok(o),
                            }
                        }
                        continue;
                    }
                }
                Opcode::SetProp => {
                    let obj_reg = self.read_u16_pc();
                    let key_reg = self.read_u16_pc();
                    let val_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let key_val = self.get_reg(key_reg);
                    let value = self.get_reg(val_reg);

                    if key_val.is_string() {
                        self.set_named_prop(ctx, obj_val, value, key_val.get_atom(), usize::MAX);
                    } else if key_val.is_symbol() && obj_val.is_object_like() {
                        let js_obj = unsafe { JSValue::object_from_ptr_mut(obj_val.get_ptr()) };
                        let sym_key =
                            crate::runtime::atom::Atom(0x40000000 | key_val.get_symbol_id());
                        js_obj.set_cached(sym_key, value, ctx.shape_cache_mut());
                    } else if key_val.is_int() && obj_val.is_object_like() {
                        if obj_val.is_object() {
                            let js_obj_check =
                                unsafe { JSValue::object_from_ptr(obj_val.get_ptr()) };
                            if js_obj_check.is_mapped_arguments() {
                                let fi = js_obj_check.mapped_args_frame_index();
                                let param_count = js_obj_check.mapped_args_param_count();
                                let idx = key_val.get_int();
                                if idx >= 0 && (idx as u32) < param_count && fi < self.frames.len()
                                {
                                    let base = self.frames[fi].registers_base;
                                    let reg_idx = base + 1 + idx as usize;
                                    if reg_idx < self.registers.len() {
                                        self.registers[reg_idx] = value;
                                    }
                                }
                            }
                        }
                        let atom = self.int_atom(key_val.get_int() as usize, ctx);
                        self.set_named_prop(ctx, obj_val, value, atom, usize::MAX);
                    } else if key_val.is_float() && obj_val.is_object_like() {
                        let s = VM::js_to_string(&key_val, ctx);
                        let atom = s.get_atom();
                        self.set_named_prop(ctx, obj_val, value, atom, usize::MAX);
                    }
                    self.set_reg(obj_reg, obj_val);
                    if let Some(exc) = self.pending_throw.take() {
                        match self.dispatch_throw_value(ctx, exc) {
                            ThrowDispatch::Caught => continue,
                            ThrowDispatch::Uncaught(e) => return Err(e),
                            ThrowDispatch::AsyncComplete(_) => continue,
                        }
                    }
                }
                Opcode::SetNamedProp => {
                    let obj_reg = self.read_u16_pc();
                    let val_reg = self.read_u16_pc();
                    let atom = crate::runtime::atom::Atom(self.read_u32_pc());
                    self.set_named_prop_fast(ctx, obj_reg, val_reg, atom, instr_pc);
                    if let Some(exc) = self.pending_throw.take() {
                        match self.dispatch_throw_value(ctx, exc) {
                            ThrowDispatch::Caught => continue,
                            ThrowDispatch::Uncaught(e) => return Err(e),
                            ThrowDispatch::AsyncComplete(_) => continue,
                        }
                    }
                }

                Opcode::LtJumpIfNot => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    let cmp = if JSValue::both_int(&a, &b) {
                        a.get_int() < b.get_int()
                    } else if a.is_float() && b.is_float() {
                        a.get_float() < b.get_float()
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        a_int < b_int
                    } else {
                        a.to_number() < b.to_number()
                    };
                    if !cmp {
                        self.pc = (self.pc as i64 + offset as i64) as usize;
                    }
                }
                Opcode::LtJumpIf => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    let cmp = if JSValue::both_int(&a, &b) {
                        a.get_int() < b.get_int()
                    } else if a.is_float() && b.is_float() {
                        a.get_float() < b.get_float()
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        a_int < b_int
                    } else {
                        a.to_number() < b.to_number()
                    };
                    if cmp {
                        self.pc = (self.pc as i64 + offset as i64) as usize;
                    }
                }
                Opcode::LteJumpIfNot => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    let cmp = if JSValue::both_int(&a, &b) {
                        a.get_int() <= b.get_int()
                    } else if a.is_float() && b.is_float() {
                        a.get_float() <= b.get_float()
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        a_int <= b_int
                    } else {
                        a.to_number() <= b.to_number()
                    };
                    if !cmp {
                        self.pc = (self.pc as i64 + offset as i64) as usize;
                    }
                }
                Opcode::LteJumpIf => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    let cmp = if JSValue::both_int(&a, &b) {
                        a.get_int() <= b.get_int()
                    } else if a.is_float() && b.is_float() {
                        a.get_float() <= b.get_float()
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        a_int <= b_int
                    } else {
                        a.to_number() <= b.to_number()
                    };
                    if cmp {
                        self.pc = (self.pc as i64 + offset as i64) as usize;
                    }
                }
                Opcode::GtJumpIfNot => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    let cmp = if JSValue::both_int(&a, &b) {
                        a.get_int() > b.get_int()
                    } else if a.is_float() && b.is_float() {
                        a.get_float() > b.get_float()
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        a_int > b_int
                    } else {
                        a.to_number() > b.to_number()
                    };
                    if !cmp {
                        self.pc = (self.pc as i64 + offset as i64) as usize;
                    }
                }
                Opcode::GtJumpIf => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    let cmp = if JSValue::both_int(&a, &b) {
                        a.get_int() > b.get_int()
                    } else if a.is_float() && b.is_float() {
                        a.get_float() > b.get_float()
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        a_int > b_int
                    } else {
                        a.to_number() > b.to_number()
                    };
                    if cmp {
                        self.pc = (self.pc as i64 + offset as i64) as usize;
                    }
                }
                Opcode::GteJumpIfNot => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    let cmp = if JSValue::both_int(&a, &b) {
                        a.get_int() >= b.get_int()
                    } else if a.is_float() && b.is_float() {
                        a.get_float() >= b.get_float()
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        a_int >= b_int
                    } else {
                        a.to_number() >= b.to_number()
                    };
                    if !cmp {
                        self.pc = (self.pc as i64 + offset as i64) as usize;
                    }
                }
                Opcode::GteJumpIf => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    let cmp = if JSValue::both_int(&a, &b) {
                        a.get_int() >= b.get_int()
                    } else if a.is_float() && b.is_float() {
                        a.get_float() >= b.get_float()
                    } else if a.is_bigint() && b.is_bigint() {
                        let a_int = Self::get_bigint_int(&a).unwrap_or(0);
                        let b_int = Self::get_bigint_int(&b).unwrap_or(0);
                        a_int >= b_int
                    } else {
                        a.to_number() >= b.to_number()
                    };
                    if cmp {
                        self.pc = (self.pc as i64 + offset as i64) as usize;
                    }
                }
                Opcode::EqJumpIfNot => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    if !loose_equal(ctx, a, b) {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }
                Opcode::EqJumpIf => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    if loose_equal(ctx, a, b) {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }
                Opcode::NeqJumpIfNot => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    if loose_equal(ctx, a, b) {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }
                Opcode::NeqJumpIf => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    if !loose_equal(ctx, a, b) {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }
                Opcode::StrictEqJumpIfNot => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    if !a.strict_eq(&b) {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }
                Opcode::StrictEqJumpIf => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    if a.strict_eq(&b) {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }
                Opcode::StrictNeqJumpIfNot => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    if a.strict_eq(&b) {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }
                Opcode::StrictNeqJumpIf => {
                    let a_reg = self.read_u16_pc();
                    let a = self.get_reg(a_reg);
                    let b_reg = self.read_u16_pc();
                    let b = self.get_reg(b_reg);
                    let offset = self.read_i32_pc();
                    if !a.strict_eq(&b) {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }

                Opcode::DeleteProp => {
                    let dst = self.read_u16_pc();
                    let obj_reg = self.read_u16_pc();
                    let key_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let key_val = self.get_reg(key_reg);
                    if obj_val.is_null() || obj_val.is_undefined() {
                        self.set_pending_type_error(
                            ctx,
                            "Cannot delete property from null or undefined",
                        );
                        if let Some(exc) = self.pending_throw.take() {
                            match self.dispatch_throw_value(ctx, exc) {
                                ThrowDispatch::Caught => {}
                                ThrowDispatch::Uncaught(e) => return Err(e),
                                ThrowDispatch::AsyncComplete(o) => return Ok(o),
                            }
                        }
                        continue;
                    }
                    let atom = if key_val.is_string() {
                        Some(key_val.get_atom())
                    } else if key_val.is_int() && (obj_val.is_object() || obj_val.is_function()) {
                        Some(self.int_atom(key_val.get_int() as usize, ctx))
                    } else if key_val.is_float() && (obj_val.is_object() || obj_val.is_function()) {
                        let s = VM::js_to_string(&key_val, ctx);
                        Some(s.get_atom())
                    } else {
                        None
                    };
                    let result = if let Some(atom) = atom {
                        if obj_val.is_object() || obj_val.is_function() {
                            let js_obj = obj_val.as_object_mut();
                            let mut deleted = false;
                            if js_obj.is_dense_array() && key_val.is_int() {
                                let idx = key_val.get_int();
                                if idx >= 0 {
                                    let ptr = obj_val.get_ptr();
                                    let arr = unsafe {
                                        &mut *(ptr as *mut crate::object::array_obj::JSArrayObject)
                                    };
                                    if (idx as usize) < arr.elements.len() {
                                        arr.elements[idx as usize] = JSValue::undefined();
                                        deleted = true;
                                    }
                                }
                            }
                            if !deleted {
                                deleted = js_obj.delete(atom);
                            }
                            if deleted {
                                ctx.runtime_mut().gc_heap_mut().deleted_props_count += 1;
                            }
                            deleted
                        } else {
                            true
                        }
                    } else {
                        true
                    };
                    let is_strict = self.frames[self.frame_index].is_strict_frame;
                    if is_strict && !result {
                        self.set_pending_type_error(ctx, "Cannot delete property");
                        if let Some(exc) = self.pending_throw.take() {
                            match self.dispatch_throw_value(ctx, exc) {
                                ThrowDispatch::Caught => {}
                                ThrowDispatch::Uncaught(e) => return Err(e),
                                ThrowDispatch::AsyncComplete(o) => return Ok(o),
                            }
                        }
                        continue;
                    }
                    self.set_reg(dst, JSValue::bool(result));
                }
                Opcode::HasProperty => {
                    let dst = self.read_u16_pc();
                    let obj_reg = self.read_u16_pc();
                    let key_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let key_val = self.get_reg(key_reg);
                    let result = if (obj_val.is_object() || obj_val.is_function())
                        && key_val.is_string()
                    {
                        let js_obj = obj_val.as_object();
                        let atom = key_val.get_atom();
                        js_obj.has_property(atom)
                    } else if (obj_val.is_object() || obj_val.is_function()) && key_val.is_int() {
                        let js_obj = obj_val.as_object();
                        let atom = self.int_atom(key_val.get_int() as usize, ctx);
                        js_obj.has_property(atom)
                    } else if (obj_val.is_object() || obj_val.is_function()) && key_val.is_float() {
                        let js_obj = obj_val.as_object();
                        let s = VM::js_to_string(&key_val, ctx);
                        let atom = s.get_atom();
                        js_obj.has_property(atom)
                    } else if (obj_val.is_object() || obj_val.is_function()) && key_val.is_symbol()
                    {
                        let js_obj = obj_val.as_object();
                        let sym_key =
                            crate::runtime::atom::Atom(0x40000000 | key_val.get_symbol_id());
                        js_obj.has_property(sym_key)
                    } else {
                        false
                    };
                    self.set_reg(dst, JSValue::bool(result));
                }
                Opcode::InstanceOf => {
                    let dst = self.read_u16_pc();
                    let obj_reg = self.read_u16_pc();
                    let ctor_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let ctor_val = self.get_reg(ctor_reg);
                    if !ctor_val.is_object() && !ctor_val.is_function() {
                        self.set_pending_type_error(
                            ctx,
                            "Right-hand side of 'instanceof' is not an object",
                        );
                        if let Some(exc) = self.pending_throw.take() {
                            match self.dispatch_throw_value(ctx, exc) {
                                ThrowDispatch::Caught => {
                                    self.set_reg(dst, JSValue::undefined());
                                }
                                ThrowDispatch::Uncaught(e) => return Err(e),
                                ThrowDispatch::AsyncComplete(o) => return Ok(o),
                            }
                        }
                        continue;
                    }

                    let hi_atom = if self.cached_has_instance_atom.0 != 0 {
                        self.cached_has_instance_atom
                    } else {
                        let sym = crate::builtins::symbol::get_symbol_has_instance(ctx);
                        let a = if sym.is_symbol() {
                            crate::runtime::atom::Atom(0x40000000 | sym.get_symbol_id())
                        } else {
                            unreachable!()
                        };
                        self.cached_has_instance_atom = a;
                        a
                    };
                    let has_instance_handler = if ctor_val.is_function() {
                        let jf = ctor_val.as_function();

                        if jf.has_symbol_on_base() {
                            jf.base.get_own(hi_atom)
                        } else {
                            None
                        }
                    } else {
                        ctor_val.as_object().get_own(hi_atom)
                    };
                    let has_instance_handler = has_instance_handler;
                    if let Some(handler) = has_instance_handler {
                        if handler.is_function() || handler.is_object() {
                            if handler.is_function() {
                                match self.call_function_with_this(
                                    ctx,
                                    handler,
                                    ctor_val,
                                    &[obj_val],
                                ) {
                                    Ok(v) => {
                                        self.set_reg(dst, JSValue::bool(v.is_truthy()));
                                        continue;
                                    }
                                    Err(_) => {
                                        self.set_reg(dst, JSValue::undefined());
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                    if !ctor_val.is_function() {
                        self.set_pending_type_error(
                            ctx,
                            "Right-hand side of 'instanceof' is not callable",
                        );
                        if let Some(exc) = self.pending_throw.take() {
                            match self.dispatch_throw_value(ctx, exc) {
                                ThrowDispatch::Caught => {
                                    self.set_reg(dst, JSValue::undefined());
                                }
                                ThrowDispatch::Uncaught(e) => return Err(e),
                                ThrowDispatch::AsyncComplete(o) => return Ok(o),
                            }
                        }
                        continue;
                    }

                    let result = if obj_val.is_object() || obj_val.is_function() {
                        let ctor_proto_opt = if ctor_val.is_function() {
                            let js_func = ctor_val.as_function();
                            if !js_func.cached_prototype_ptr.is_null() {
                                Some(
                                    js_func.cached_prototype_ptr
                                        as *const crate::object::object::JSObject,
                                )
                            } else {
                                let proto_atom = ctx.common_atoms.prototype;
                                js_func.base.get(proto_atom).and_then(|v| {
                                    if v.is_object() {
                                        let ptr =
                                            v.get_ptr() as *mut crate::object::object::JSObject;

                                        ctor_val.as_function_mut().cached_prototype_ptr = ptr;
                                        Some(ptr as *const crate::object::object::JSObject)
                                    } else {
                                        None
                                    }
                                })
                            }
                        } else {
                            None
                        };
                        if let Some(ctor_proto_ptr) = ctor_proto_opt {
                            let obj_ptr = obj_val.get_ptr();
                            let mut proto_opt = unsafe {
                                (*(obj_ptr as *const crate::object::object::JSObject)).prototype
                            };
                            let mut found = false;
                            let mut limit = 0;
                            while let Some(proto_ptr) = proto_opt {
                                if std::ptr::eq(proto_ptr, ctor_proto_ptr) {
                                    found = true;
                                    break;
                                }
                                proto_opt = unsafe { (*proto_ptr).prototype };
                                limit += 1;
                                if limit > 1000 {
                                    break;
                                }
                            }
                            found
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    self.set_reg(dst, JSValue::bool(result));
                }
                Opcode::NewRegExp => {
                    let cache_key = self.cached_code_ptr as usize + self.pc;
                    let dst = self.read_u16_pc();
                    let pattern_idx = self.read_u32() as usize;
                    let flags_idx = self.read_u32() as usize;
                    let constants_len = self.frames[self.frame_index].constants_len;
                    let pattern_val = if pattern_idx < constants_len {
                        unsafe { *self.cached_const_ptr.add(pattern_idx) }
                    } else {
                        JSValue::undefined()
                    };
                    let flags_val = if flags_idx < constants_len {
                        unsafe { *self.cached_const_ptr.add(flags_idx) }
                    } else {
                        JSValue::undefined()
                    };
                    let pattern = if pattern_val.is_string() {
                        ctx.get_atom_str(pattern_val.get_atom()).to_string()
                    } else {
                        String::new()
                    };
                    let flags = if flags_val.is_string() {
                        ctx.get_atom_str(flags_val.get_atom()).to_string()
                    } else {
                        String::new()
                    };

                    if let Some(cached_re) = self.regex_lit_cache.get(&cache_key) {
                        let cloned_re = cached_re.clone();
                        let re_val = crate::builtins::regexp::create_regexp_object_precompiled(
                            ctx, &pattern, &flags, cloned_re,
                        );
                        self.set_reg(dst, re_val);
                    } else {
                        let pattern_atom = ctx.intern(&pattern);
                        let flags_atom = ctx.intern(&flags);
                        let re_val = crate::builtins::regexp::regexp_constructor(
                            ctx,
                            &[
                                JSValue::new_string(pattern_atom),
                                JSValue::new_string(flags_atom),
                            ],
                        );

                        if re_val.is_object() {
                            if let Some(compiled) = re_val.as_object().get_compiled_regex() {
                                self.regex_lit_cache.insert(cache_key, compiled.clone());
                            }
                        }
                        self.set_reg(dst, re_val);
                    }
                }
                Opcode::GetPrivate => {
                    let dst = self.read_u16_pc();
                    let obj_reg = self.read_u16_pc();
                    let key_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let key_val = self.get_reg(key_reg);
                    let result = if (obj_val.is_object() || obj_val.is_function())
                        && key_val.is_string()
                    {
                        let js_obj = obj_val.as_object();
                        let atom = key_val.get_atom();
                        let atom_str = ctx.get_atom_str(atom).to_string();
                        let alt_atom = if let Some(stripped) = atom_str.strip_prefix('#') {
                            Some(ctx.intern(stripped))
                        } else {
                            None
                        };

                        let own_accessor =
                            js_obj.get_own_private_accessor_entry(atom).or_else(|| {
                                alt_atom.and_then(|a| js_obj.get_own_private_accessor_entry(a))
                            });
                        if let Some(entry) = own_accessor {
                            if let Some(getter) = entry.get {
                                self.call_function_with_this(ctx, getter, obj_val, &[])
                                    .unwrap_or(JSValue::undefined())
                            } else {
                                JSValue::undefined()
                            }
                        } else if let Some(val) = js_obj
                            .get_private_field(atom)
                            .or_else(|| alt_atom.and_then(|a| js_obj.get_private_field(a)))
                        {
                            val
                        } else {
                            let mut getter_fn: Option<JSValue> = None;
                            let mut inherited_value: Option<JSValue> = None;
                            let mut cur = js_obj.prototype;
                            let mut depth = 0u32;
                            while let Some(p) = cur {
                                if p.is_null() || depth > 100 {
                                    break;
                                }
                                let proto_obj = unsafe { &*p };
                                let accessor_entry =
                                    proto_obj.get_own_private_accessor_entry(atom).or_else(|| {
                                        alt_atom.and_then(|a| {
                                            proto_obj.get_own_private_accessor_entry(a)
                                        })
                                    });
                                if let Some(entry) = accessor_entry {
                                    getter_fn = entry.get;
                                    break;
                                }
                                if let Some(v) = proto_obj.get_private_field(atom).or_else(|| {
                                    alt_atom.and_then(|a| proto_obj.get_private_field(a))
                                }) {
                                    inherited_value = Some(v);
                                    break;
                                }
                                cur = proto_obj.prototype;
                                depth += 1;
                            }
                            if let Some(getter) = getter_fn {
                                self.call_function_with_this(ctx, getter, obj_val, &[])
                                    .unwrap_or(JSValue::undefined())
                            } else if let Some(v) = inherited_value {
                                v
                            } else {
                                JSValue::undefined()
                            }
                        }
                    } else {
                        JSValue::undefined()
                    };
                    self.set_reg(dst, result);
                }
                Opcode::SetPrivate => {
                    let obj_reg = self.read_u16_pc();
                    let key_reg = self.read_u16_pc();
                    let val_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let key_val = self.get_reg(key_reg);
                    let val = self.get_reg(val_reg);
                    if (obj_val.is_object() || obj_val.is_function()) && key_val.is_string() {
                        let atom = key_val.get_atom();
                        let atom_str = ctx.get_atom_str(atom).to_string();
                        let alt_atom = if let Some(stripped) = atom_str.strip_prefix('#') {
                            Some(ctx.intern(stripped))
                        } else {
                            None
                        };

                        let own_accessor_entry = {
                            let js_obj = obj_val.as_object();
                            js_obj
                                .get_own_private_accessor_entry(atom)
                                .or_else(|| {
                                    alt_atom.and_then(|a| js_obj.get_own_private_accessor_entry(a))
                                })
                                .cloned()
                        };
                        if let Some(entry) = own_accessor_entry {
                            if let Some(setter) = entry.set {
                                let _ = self.call_function_with_this(ctx, setter, obj_val, &[val]);
                            } else {
                                self.set_pending_type_error(
                                    ctx,
                                    "Cannot set private property without setter",
                                );
                                if let Some(exc) = self.pending_throw.take() {
                                    match self.dispatch_throw_value(ctx, exc) {
                                        ThrowDispatch::Caught => continue,
                                        ThrowDispatch::Uncaught(e) => return Err(e),
                                        ThrowDispatch::AsyncComplete(o) => return Ok(o),
                                    }
                                }
                            }
                        } else {
                            let mut setter_fn: Option<JSValue> = None;
                            let mut accessor_found = false;
                            {
                                let js_obj = obj_val.as_object();
                                let mut cur = js_obj.prototype;
                                let mut depth = 0u32;
                                while let Some(p) = cur {
                                    if p.is_null() || depth > 100 {
                                        break;
                                    }
                                    let proto_obj = unsafe { &*p };
                                    let accessor_entry = proto_obj
                                        .get_own_private_accessor_entry(atom)
                                        .or_else(|| {
                                            alt_atom.and_then(|a| {
                                                proto_obj.get_own_private_accessor_entry(a)
                                            })
                                        });
                                    if let Some(entry) = accessor_entry {
                                        accessor_found = true;
                                        setter_fn = entry.set;
                                        break;
                                    }
                                    cur = proto_obj.prototype;
                                    depth += 1;
                                }
                            }
                            if let Some(setter) = setter_fn {
                                let _ = self.call_function_with_this(ctx, setter, obj_val, &[val]);
                            } else if accessor_found {
                                self.set_pending_type_error(
                                    ctx,
                                    "Cannot set private property without setter",
                                );
                                if let Some(exc) = self.pending_throw.take() {
                                    match self.dispatch_throw_value(ctx, exc) {
                                        ThrowDispatch::Caught => continue,
                                        ThrowDispatch::Uncaught(e) => return Err(e),
                                        ThrowDispatch::AsyncComplete(o) => return Ok(o),
                                    }
                                }
                            } else {
                                let js_obj = obj_val.as_object_mut();
                                js_obj.set_private_field(atom, val);
                            }
                        }
                    }
                }
                Opcode::HasPrivate => {
                    let dst = self.read_u16_pc();
                    let obj_reg = self.read_u16_pc();
                    let key_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let key_val = self.get_reg(key_reg);
                    let result =
                        if (obj_val.is_object() || obj_val.is_function()) && key_val.is_string() {
                            let js_obj = obj_val.as_object();
                            let atom = key_val.get_atom();
                            js_obj.has_private_field(atom)
                        } else {
                            false
                        };
                    self.set_reg(dst, JSValue::bool(result));
                }
                Opcode::GatherRest => {
                    let dst = self.read_u16_pc();
                    let base = self.frames[self.frame_index].registers_base;
                    let arg_count = self.frames[self.frame_index].arg_count as usize;
                    let rest_start = dst as usize;

                    let rest_count = if arg_count + 1 > rest_start {
                        arg_count + 1 - rest_start
                    } else {
                        0
                    };
                    let mut arr =
                        crate::object::array_obj::JSArrayObject::with_capacity(rest_count);
                    if let Some(proto_ptr) = ctx.get_array_prototype() {
                        arr.header.set_prototype_raw(proto_ptr);
                    }
                    for i in 0..rest_count {
                        let val = self.registers[base + rest_start + i];
                        arr.push(val);
                    }
                    let len_atom = ctx.common_atoms.length;
                    arr.header
                        .set(len_atom, JSValue::new_int(rest_count as i64));
                    let ptr = Box::into_raw(Box::new(arr)) as usize;
                    ctx.runtime_mut().gc_heap_mut().track_array(ptr);
                    self.set_reg(dst, JSValue::new_object(ptr));
                }
                Opcode::ObjectSpread => {
                    let dst = self.read_u16_pc();
                    let src = self.read_u16_pc();
                    let dst_val = self.get_reg(dst);
                    let src_val = self.get_reg(src);
                    if (dst_val.is_object() || dst_val.is_function())
                        && (src_val.is_object() || src_val.is_function())
                    {
                        let dst_obj = dst_val.as_object_mut();
                        let src_obj = src_val.as_object();

                        for (atom, value) in src_obj.own_properties() {
                            dst_obj.set_cached(atom, value, ctx.shape_cache_mut());
                        }
                    }
                }
                Opcode::GetPropertyNames => {
                    let dst = self.read_u16_pc();
                    let obj_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let mut arr = crate::object::array_obj::JSArrayObject::new();
                    if let Some(proto_ptr) = ctx.get_array_prototype() {
                        arr.header.set_prototype_raw(proto_ptr);
                    }
                    if obj_val.is_object() || obj_val.is_function() {
                        let js_obj = obj_val.as_object();
                        if js_obj.is_dense_array() {
                            let arr_obj = unsafe {
                                &*(obj_val.get_ptr()
                                    as *mut crate::object::array_obj::JSArrayObject)
                            };
                            for i in 0..arr_obj.elements.len() {
                                let idx_atom = ctx.intern(i.to_string().as_str());
                                arr.push(JSValue::new_string(idx_atom));
                            }
                        }
                        for (atom, _value) in js_obj.own_properties() {
                            arr.push(JSValue::new_string(atom));
                        }
                    }
                    let len_atom = ctx.common_atoms.length;
                    arr.header
                        .set(len_atom, JSValue::new_int(arr.elements.len() as i64));
                    let ptr = Box::into_raw(Box::new(arr)) as usize;
                    ctx.runtime_mut().gc_heap_mut().track_array(ptr);
                    self.allocation_count += 1;
                    self.set_reg(dst, JSValue::new_object(ptr));
                    self.maybe_gc(ctx);
                }
                Opcode::ArrayExtend => {
                    let dst = self.read_u16_pc();
                    let src = self.read_u16_pc();
                    let dst_val = self.get_reg(dst);
                    let src_val = self.get_reg(src);
                    if dst_val.is_object() && src_val.is_object() {
                        let dst_ptr = dst_val.get_ptr();
                        let src_ptr = src_val.get_ptr();
                        let is_src_array = src_val.as_object().is_dense_array();
                        let is_dst_array = dst_val.as_object().is_dense_array();
                        if is_src_array && is_dst_array {
                            let dst_arr = unsafe {
                                &mut *(dst_ptr as *mut crate::object::array_obj::JSArrayObject)
                            };
                            let src_arr = unsafe {
                                &*(src_ptr as *const crate::object::array_obj::JSArrayObject)
                            };
                            for val in src_arr.elements.iter() {
                                dst_arr.push(*val);
                            }
                            let len_atom = ctx.common_atoms.length;
                            dst_arr
                                .header
                                .set(len_atom, JSValue::new_int(dst_arr.elements.len() as i64));
                        } else if is_dst_array {
                            let dst_arr = unsafe {
                                &mut *(dst_ptr as *mut crate::object::array_obj::JSArrayObject)
                            };
                            let src_obj =
                                unsafe { &*(src_ptr as *const crate::object::object::JSObject) };
                            let len_atom = ctx.common_atoms.length;
                            let arr_len = src_obj
                                .get(len_atom)
                                .map(|v| v.get_int() as usize)
                                .unwrap_or(0);
                            if let Some(elems) = src_obj.get_array_elements() {
                                for val in elems.iter() {
                                    dst_arr.push(*val);
                                }
                            } else {
                                for i in 0..arr_len {
                                    let key = ctx.intern(&i.to_string());
                                    if let Some(val) = src_obj.get(key) {
                                        dst_arr.push(val);
                                    }
                                }
                            }
                            dst_arr
                                .header
                                .set(len_atom, JSValue::new_int(dst_arr.elements.len() as i64));
                        }
                    }
                }
                Opcode::ArrayPush => {
                    let arr_reg = self.read_u16_pc();
                    let val_reg = self.read_u16_pc();
                    let arr_val = self.get_reg(arr_reg);
                    let val = self.get_reg(val_reg);
                    if arr_val.is_object() {
                        let arr_ptr = arr_val.get_ptr();
                        let arr = unsafe {
                            &mut *(arr_ptr as *mut crate::object::array_obj::JSArrayObject)
                        };
                        arr.push(val);
                        let len_atom = ctx.common_atoms.length;
                        let new_len = arr.elements.len() as i64;
                        if !arr.header.has_own(len_atom) {
                            arr.header.define_property(
                                len_atom,
                                crate::object::object::PropertyDescriptor {
                                    value: Some(JSValue::new_int(new_len)),
                                    writable: true,
                                    enumerable: false,
                                    configurable: false,
                                    get: None,
                                    set: None,
                                },
                            );
                        } else {
                            arr.header.set(len_atom, JSValue::new_int(new_len));
                        }
                    }
                }
                Opcode::SetProto => {
                    let obj_reg = self.read_u16_pc();
                    let proto_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let proto_val = self.get_reg(proto_reg);
                    if obj_val.is_object() && proto_val.is_object() {
                        let obj_ref = obj_val.as_object_mut();
                        obj_ref.prototype =
                            Some(proto_val.get_ptr() as *mut crate::object::object::JSObject);
                    }
                }
                Opcode::GetSuper => {
                    let dst = self.read_u16_pc();
                    self.set_reg(dst, self.frames[self.frame_index].super_ctor);
                }

                Opcode::DefineAccessor => {
                    let obj_reg = self.read_u16_pc();
                    let key_reg = self.read_u16_pc();
                    let getter_reg = self.read_u16_pc();
                    let setter_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let key_val = self.get_reg(key_reg);
                    if obj_val.is_object_like() {
                        let prop_key = if key_val.is_string() {
                            key_val
                        } else {
                            let prim = self.ordinary_to_primitive(&key_val, "string", ctx);
                            if self.pending_throw.is_some() {
                                if let Some(exc) = self.pending_throw.take() {
                                    let disp = self.dispatch_throw_value(ctx, exc);
                                    match disp {
                                        ThrowDispatch::Caught => {}
                                        ThrowDispatch::Uncaught(e) => return Err(e),
                                        ThrowDispatch::AsyncComplete(o) => match o {
                                            ExecutionOutcome::Complete(v) => {
                                                self.set_reg(obj_reg, v);
                                            }
                                            _ => {}
                                        },
                                    }
                                }
                                continue;
                            }
                            if prim.is_string() {
                                prim
                            } else {
                                VM::js_to_string(&prim, ctx)
                            }
                        };
                        if prop_key.is_string() {
                            let atom = prop_key.get_atom();
                            let obj = obj_val.as_object_mut();
                            let getter = if getter_reg != u16::MAX {
                                Some(self.get_reg(getter_reg))
                            } else {
                                None
                            };
                            let setter = if setter_reg != u16::MAX {
                                Some(self.get_reg(setter_reg))
                            } else {
                                None
                            };
                            let existing = obj.get_own_accessor_entry(atom);
                            let final_getter = getter.or_else(|| existing.and_then(|e| e.get));
                            let final_setter = setter.or_else(|| existing.and_then(|e| e.set));
                            obj.define_accessor(atom, final_getter, final_setter);
                        } else if self.pending_throw.is_none() {
                            self.set_pending_type_error(ctx, "Invalid property key");
                        }
                    }
                }

                Opcode::DefinePrivateAccessor => {
                    let obj_reg = self.read_u16_pc();
                    let key_reg = self.read_u16_pc();
                    let getter_reg = self.read_u16_pc();
                    let setter_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let key_val = self.get_reg(key_reg);
                    if (obj_val.is_object() || obj_val.is_function()) && key_val.is_string() {
                        let atom = key_val.get_atom();
                        let obj = obj_val.as_object_mut();
                        let getter = if getter_reg != u16::MAX {
                            Some(self.get_reg(getter_reg))
                        } else {
                            None
                        };
                        let setter = if setter_reg != u16::MAX {
                            Some(self.get_reg(setter_reg))
                        } else {
                            None
                        };
                        obj.define_private_accessor(atom, getter, setter);
                    }
                }

                Opcode::SetMethodProp => {
                    let obj_reg = self.read_u16_pc();
                    let key_reg = self.read_u16_pc();
                    let val_reg = self.read_u16_pc();
                    let obj_val = self.get_reg(obj_reg);
                    let key_val = self.get_reg(key_reg);
                    let value = self.get_reg(val_reg);
                    if obj_val.is_object_like() {
                        let js_obj = unsafe { JSValue::object_from_ptr_mut(obj_val.get_ptr()) };
                        let atom = if key_val.is_string() {
                            key_val.get_atom()
                        } else if key_val.is_symbol() {
                            crate::runtime::atom::Atom(0x40000000 | key_val.get_symbol_id())
                        } else if key_val.is_float() {
                            let s = VM::js_to_string(&key_val, ctx);
                            s.get_atom()
                        } else {
                            self.int_atom(key_val.get_int() as usize, ctx)
                        };
                        js_obj.set_cached_non_enumerable(atom, value, ctx.shape_cache_mut());
                    }
                    self.set_reg(obj_reg, obj_val);
                }

                Opcode::CallCurrent1 => {
                    unsafe {
                        (*self.frames.as_mut_ptr().add(self.frame_index)).current_pc = instr_pc;
                    }
                    let dst = self.read_u16_pc();
                    let arg_reg = self.read_u16_pc();
                    let caller = &self.frames[self.frame_index];
                    let function_ptr = caller.function_ptr;
                    if function_ptr.is_none() {
                        self.set_pending_type_error(ctx, "undefined is not a function");
                        if let Some(exc) = self.pending_throw.take() {
                            match self.dispatch_throw_value(ctx, exc) {
                                ThrowDispatch::Caught => continue,
                                ThrowDispatch::Uncaught(e) => return Err(e),
                                ThrowDispatch::AsyncComplete(o) => return Ok(o),
                            }
                        }
                        continue;
                    }
                    let one_arg = [arg_reg];
                    self.push_frame_from_arg_regs_raw(
                        ctx,
                        caller.locals_count,
                        caller.bytecode_ptr,
                        caller.bytecode_len,
                        caller.constants_ptr,
                        caller.constants_len,
                        self.pc,
                        function_ptr,
                        caller.ic_table_ptr,
                        JSValue::undefined(),
                        dst,
                        1,
                        false,
                        caller.is_async,
                        self.cached_registers_base,
                        &one_arg,
                        caller.uses_arguments,
                    );
                    continue;
                }

                Opcode::Call
                | Opcode::Call0
                | Opcode::Call1
                | Opcode::Call2
                | Opcode::Call3
                | Opcode::CallMethod
                | Opcode::CallNew
                | Opcode::CallCurrent => {
                    unsafe {
                        (*self.frames.as_mut_ptr().add(self.frame_index)).current_pc = instr_pc;
                    }
                    let is_call_current = op == Opcode::CallCurrent;
                    let is_call0 = op == Opcode::Call0;
                    let is_call1 = op == Opcode::Call1;
                    let is_call2 = op == Opcode::Call2;
                    let is_call3 = op == Opcode::Call3;
                    let is_call_method = op == Opcode::CallMethod;
                    let is_call_new = op == Opcode::CallNew;
                    let dst = self.read_u16_pc();
                    let obj_reg = if is_call_method {
                        self.read_u16_pc()
                    } else {
                        0
                    };
                    let func_reg = if is_call_current {
                        0
                    } else {
                        self.read_u16_pc()
                    };
                    let mut one_arg = [0u16; 1];
                    let mut two_args = [0u16; 2];
                    let mut three_args = [0u16; 3];
                    let argc = if is_call0 {
                        0
                    } else if is_call1 {
                        one_arg[0] = self.read_u16_pc();
                        1
                    } else if is_call2 {
                        two_args[0] = self.read_u16_pc();
                        two_args[1] = self.read_u16_pc();
                        2
                    } else if is_call3 {
                        three_args[0] = self.read_u16_pc();
                        three_args[1] = self.read_u16_pc();
                        three_args[2] = self.read_u16_pc();
                        3
                    } else {
                        self.read_u16_pc()
                    };

                    let mut arg_regs_buf = [0u16; 16];
                    let mut arg_regs_vec = Vec::new();
                    let arg_regs: &[u16] = if is_call1 {
                        &one_arg
                    } else if is_call2 {
                        &two_args
                    } else if is_call3 {
                        &three_args
                    } else if argc as usize <= 16 {
                        for i in 0..argc {
                            arg_regs_buf[i as usize] = self.read_u16_pc();
                        }
                        &arg_regs_buf[..argc as usize]
                    } else {
                        arg_regs_vec.reserve(argc as usize);
                        for _ in 0..argc {
                            arg_regs_vec.push(self.read_u16_pc());
                        }
                        &arg_regs_vec
                    };

                    if is_call_current {
                        let caller = &self.frames[self.frame_index];
                        let function_ptr = caller.function_ptr;
                        if function_ptr.is_none() {
                            self.set_pending_type_error(ctx, "undefined is not a function");
                            if let Some(exc) = self.pending_throw.take() {
                                match self.dispatch_throw_value(ctx, exc) {
                                    ThrowDispatch::Caught => continue,
                                    ThrowDispatch::Uncaught(e) => return Err(e),
                                    ThrowDispatch::AsyncComplete(o) => return Ok(o),
                                }
                            }
                            continue;
                        }

                        self.push_frame_from_arg_regs_raw(
                            ctx,
                            caller.locals_count,
                            caller.bytecode_ptr,
                            caller.bytecode_len,
                            caller.constants_ptr,
                            caller.constants_len,
                            self.pc,
                            function_ptr,
                            caller.ic_table_ptr,
                            JSValue::undefined(),
                            dst,
                            argc,
                            false,
                            caller.is_async,
                            self.cached_registers_base,
                            arg_regs,
                            caller.uses_arguments,
                        );
                        continue;
                    }

                    let func_val = self.get_reg(func_reg);
                    let this_val = if is_call_method {
                        self.get_reg(obj_reg)
                    } else if is_call_new {
                        let props = ctx.runtime_mut().gc_heap_mut().take_prop_vec();
                        let mut obj = crate::object::object::JSObject::new_typed_from_pool(
                            crate::object::object::ObjectType::Ordinary,
                            props,
                        );
                        obj.ensure_shape(ctx.shape_cache_mut());
                        if let Some(proto_ptr) = ctx.get_object_prototype() {
                            obj.prototype = Some(proto_ptr);
                        }
                        let func_val = if is_call_current {
                            if let Some(ptr) = self.frames[self.frame_index].function_ptr {
                                JSValue::new_function(ptr)
                            } else {
                                JSValue::undefined()
                            }
                        } else {
                            self.get_reg(func_reg)
                        };
                        if func_val.is_function() {
                            let js_func = func_val.as_function();

                            if !js_func.cached_prototype_ptr.is_null() {
                                obj.prototype = Some(js_func.cached_prototype_ptr);
                            } else {
                                let proto_key = ctx.common_atoms.prototype;
                                if let Some(proto_val) = js_func.base.get(proto_key) {
                                    if proto_val.is_object() {
                                        let cptr = proto_val.get_ptr()
                                            as *mut crate::object::object::JSObject;
                                        func_val.as_function_mut().cached_prototype_ptr = cptr;
                                        obj.prototype = Some(cptr);
                                    }
                                } else if !js_func.is_builtin() {
                                    let mut pobj = crate::object::object::JSObject::new();
                                    pobj.set(ctx.common_atoms.constructor, func_val);
                                    if let Some(opp) = ctx.get_object_prototype() {
                                        pobj.prototype = Some(opp);
                                    }
                                    let pp = Box::into_raw(Box::new(pobj)) as usize;
                                    ctx.runtime_mut().gc_heap_mut().track(pp);
                                    let func_mut = func_val.as_function_mut();
                                    func_mut.base.set(proto_key, JSValue::new_object(pp));
                                    let cptr = pp as *mut crate::object::object::JSObject;
                                    func_mut.cached_prototype_ptr = cptr;
                                    obj.prototype = Some(cptr);
                                }
                            }
                        }
                        let ptr = if ctx.runtime().gc_heap().nursery_enabled_and_can_fit_object() {
                            ctx.runtime_mut()
                                .gc_heap_mut()
                                .alloc_object_with_value(obj)
                                .map(|p| p as usize)
                                .unwrap_or_else(|| {
                                    let hp = Box::into_raw(Box::new(
                                        crate::object::object::JSObject::new(),
                                    )) as usize;
                                    ctx.runtime_mut().gc_heap_mut().track(hp);
                                    hp
                                })
                        } else {
                            let heap_ptr = Box::into_raw(Box::new(obj)) as usize;
                            ctx.runtime_mut().gc_heap_mut().track(heap_ptr);
                            heap_ptr
                        };
                        self.allocation_count += 1;
                        JSValue::new_object(ptr)
                    } else {
                        let frame = &self.frames[self.frame_index];
                        if frame.is_constructor
                            && !frame.super_ctor.is_undefined()
                            && func_val.is_function()
                            && frame.super_ctor.is_function()
                        {
                            let sctor_ptr = frame.super_ctor.get_ptr();
                            let fptr = func_val.get_ptr();
                            if fptr == sctor_ptr {
                                frame.this_value
                            } else {
                                JSValue::undefined()
                            }
                        } else {
                            JSValue::undefined()
                        }
                    };

                    if self.execute_call(
                        ctx,
                        func_val,
                        this_val,
                        dst,
                        argc,
                        arg_regs,
                        obj_reg,
                        is_call_new,
                        is_call_method,
                    )? {
                        continue;
                    }
                    if let Some(exc) = self.pending_throw.take() {
                        match self.dispatch_throw_value(ctx, exc) {
                            ThrowDispatch::Caught => continue,
                            ThrowDispatch::Uncaught(e) => return Err(e),
                            ThrowDispatch::AsyncComplete(o) => return Ok(o),
                        }
                    }
                }

                Opcode::CallNamedMethod => {
                    unsafe {
                        (*self.frames.as_mut_ptr().add(self.frame_index)).current_pc = instr_pc;
                    }
                    let dst = self.read_u16_pc();
                    let obj_reg = self.read_u16_pc();
                    let atom = crate::runtime::atom::Atom(self.read_u32_pc());
                    let argc = self.read_u16_pc();

                    let mut arg_regs_buf = [0u16; 16];
                    let mut arg_regs_vec = Vec::new();
                    let arg_regs: &[u16] = if argc as usize <= 16 {
                        for i in 0..argc {
                            arg_regs_buf[i as usize] = self.read_u16_pc();
                        }
                        &arg_regs_buf[..argc as usize]
                    } else {
                        arg_regs_vec.reserve(argc as usize);
                        for _ in 0..argc {
                            arg_regs_vec.push(self.read_u16_pc());
                        }
                        &arg_regs_vec
                    };

                    let obj_val = self.get_reg(obj_reg);
                    if !self.get_named_prop_fast(ctx, obj_reg, obj_reg, atom, instr_pc) {}
                    let func_val = self.get_reg(obj_reg);

                    self.set_reg(obj_reg, obj_val);
                    let this_val = obj_val;

                    if self.execute_call(
                        ctx, func_val, this_val, dst, argc, arg_regs, obj_reg, false, true,
                    )? {
                        continue;
                    }
                    if let Some(exc) = self.pending_throw.take() {
                        match self.dispatch_throw_value(ctx, exc) {
                            ThrowDispatch::Caught => continue,
                            ThrowDispatch::Uncaught(e) => return Err(e),
                            ThrowDispatch::AsyncComplete(o) => return Ok(o),
                        }
                    }
                }

                Opcode::CallSpread | Opcode::CallMethodSpread | Opcode::CallNewSpread => {
                    let is_call_method = op == Opcode::CallMethodSpread;
                    let is_call_new = op == Opcode::CallNewSpread;
                    let dst = self.read_u16_pc();
                    let obj_reg = if is_call_method {
                        self.read_u16_pc()
                    } else {
                        0
                    };
                    let func_reg = self.read_u16_pc();
                    let arr_reg = self.read_u16_pc();

                    let func_val = self.get_reg(func_reg);
                    let arr_val = self.get_reg(arr_reg);
                    let mut args = Vec::new();
                    if arr_val.is_object() {
                        let arr_ptr = arr_val.get_ptr();
                        let is_jsarray = arr_val.as_object().is_dense_array();
                        if is_jsarray {
                            let arr = unsafe {
                                &*(arr_ptr as *const crate::object::array_obj::JSArrayObject)
                            };
                            for val in arr.elements.iter() {
                                args.push(*val);
                            }
                        } else {
                            let arr = arr_val.as_object();
                            let len_atom = ctx.common_atoms.length;
                            let arr_len =
                                arr.get(len_atom).map(|v| v.get_int() as usize).unwrap_or(0);
                            if let Some(elems) = arr.get_array_elements() {
                                for val in elems.iter() {
                                    args.push(*val);
                                }
                            } else {
                                for i in 0..arr_len {
                                    let key = ctx.intern(&i.to_string());
                                    if let Some(val) = arr.get(key) {
                                        args.push(val);
                                    }
                                }
                            }
                        }
                    }
                    let argc = args.len() as u16;

                    let this_val = if is_call_method {
                        self.get_reg(obj_reg)
                    } else if is_call_new {
                        let props = ctx.runtime_mut().gc_heap_mut().take_prop_vec();
                        let mut obj = crate::object::object::JSObject::new_typed_from_pool(
                            crate::object::object::ObjectType::Ordinary,
                            props,
                        );
                        obj.ensure_shape(ctx.shape_cache_mut());
                        if let Some(proto_ptr) = ctx.get_object_prototype() {
                            obj.prototype = Some(proto_ptr);
                        }
                        if func_val.is_function() {
                            let js_func = func_val.as_function();

                            if !js_func.cached_prototype_ptr.is_null() {
                                obj.prototype = Some(js_func.cached_prototype_ptr);
                            } else {
                                let proto_key = ctx.common_atoms.prototype;
                                if let Some(proto_val) = js_func.base.get(proto_key) {
                                    if proto_val.is_object() {
                                        let cptr = proto_val.get_ptr()
                                            as *mut crate::object::object::JSObject;
                                        func_val.as_function_mut().cached_prototype_ptr = cptr;
                                        obj.prototype = Some(cptr);
                                    }
                                } else if !js_func.is_builtin() {
                                    let mut pobj = crate::object::object::JSObject::new();
                                    pobj.set(ctx.common_atoms.constructor, func_val);
                                    if let Some(opp) = ctx.get_object_prototype() {
                                        pobj.prototype = Some(opp);
                                    }
                                    let pp = Box::into_raw(Box::new(pobj)) as usize;
                                    ctx.runtime_mut().gc_heap_mut().track(pp);
                                    let func_mut = func_val.as_function_mut();
                                    func_mut.base.set(proto_key, JSValue::new_object(pp));
                                    let cptr = pp as *mut crate::object::object::JSObject;
                                    func_mut.cached_prototype_ptr = cptr;
                                    obj.prototype = Some(cptr);
                                }
                            }
                        }
                        let ptr = if ctx.runtime().gc_heap().nursery_enabled_and_can_fit_object() {
                            ctx.runtime_mut()
                                .gc_heap_mut()
                                .alloc_object_with_value(obj)
                                .map(|p| p as usize)
                                .unwrap_or_else(|| {
                                    let hp = Box::into_raw(Box::new(
                                        crate::object::object::JSObject::new(),
                                    )) as usize;
                                    ctx.runtime_mut().gc_heap_mut().track(hp);
                                    hp
                                })
                        } else {
                            let heap_ptr = Box::into_raw(Box::new(obj)) as usize;
                            ctx.runtime_mut().gc_heap_mut().track(heap_ptr);
                            heap_ptr
                        };
                        self.allocation_count += 1;
                        JSValue::new_object(ptr)
                    } else {
                        let frame = &self.frames[self.frame_index];
                        if frame.is_constructor
                            && !frame.super_ctor.is_undefined()
                            && func_val.is_function()
                            && frame.super_ctor.is_function()
                        {
                            let sctor_ptr = frame.super_ctor.get_ptr();
                            let fptr = func_val.get_ptr();
                            if fptr == sctor_ptr {
                                frame.this_value
                            } else {
                                JSValue::undefined()
                            }
                        } else {
                            JSValue::undefined()
                        }
                    };

                    if func_val.is_function() {
                        let ptr = func_val.get_ptr();
                        let js_func = func_val.as_function();
                        if let Some(ref rb) = js_func.bytecode {
                            if js_func.is_generator() {
                                let mut snapshot =
                                    vec![JSValue::undefined(); rb.locals_count as usize];
                                if !snapshot.is_empty() {
                                    snapshot[0] = this_val;
                                    for (i, arg) in args.iter().enumerate() {
                                        if i + 1 < snapshot.len() {
                                            snapshot[i + 1] = *arg;
                                        }
                                    }
                                }
                                let mut gen_obj = crate::object::object::JSObject::new();
                                gen_obj.set_is_generator(true);
                                if let Some(proto_ptr) = ctx.get_generator_prototype() {
                                    gen_obj.prototype = Some(proto_ptr);
                                }
                                gen_obj.set_generator_state(
                                    crate::object::object::GeneratorState {
                                        bytecode: Box::new((**rb).clone()),
                                        snapshot,
                                        pc: 0,
                                        done: false,
                                    },
                                );
                                let gen_ptr = Box::into_raw(Box::new(gen_obj)) as usize;
                                ctx.runtime_mut().gc_heap_mut().track(gen_ptr);
                                self.set_reg(dst, JSValue::new_object(gen_ptr));
                                continue;
                            }
                            let return_pc = self.pc;
                            self.push_frame(
                                rb,
                                return_pc,
                                Some(ptr),
                                this_val,
                                dst,
                                argc,
                                is_call_new,
                                js_func.is_async(),
                                &args,
                                js_func.uses_arguments(),
                            );
                            if is_call_new {
                                let super_key = ctx.common_atoms.__super__;
                                if let Some(super_val) = js_func.base.get(super_key) {
                                    self.frames[self.frame_index].super_ctor = super_val;
                                }
                            }
                            continue;
                        } else if js_func.is_builtin() {
                            let caller_base = self.cached_registers_base;
                            let mut builtin_args = Vec::with_capacity(args.len() + 1);
                            if is_call_method {
                                builtin_args.push(self.registers[caller_base + obj_reg as usize]);
                            }
                            builtin_args.extend(args);
                            let result = if let Some(bf) = js_func.builtin_func {
                                ctx.call_builtin_direct(bf, &builtin_args)
                            } else if let Some(ba) = js_func.builtin_atom {
                                let name = ctx.get_atom_str(ba).to_string();
                                ctx.call_builtin(&name, &builtin_args)
                            } else {
                                JSValue::undefined()
                            };
                            self.set_reg(dst, result);
                        }
                    } else if func_val.is_object() && !is_call_new {
                        let result =
                            self.call_function_with_this(ctx, func_val, this_val, &args)?;
                        self.set_reg(dst, result);
                    } else {
                        let msg = format!(
                            "{} is not a function",
                            self.format_thrown_value(&func_val, ctx)
                        );
                        self.set_pending_type_error(ctx, &msg);
                        if let Some(exc) = self.pending_throw.take() {
                            match self.dispatch_throw_value(ctx, exc) {
                                ThrowDispatch::Caught => continue,
                                ThrowDispatch::Uncaught(e) => return Err(e),
                                ThrowDispatch::AsyncComplete(o) => return Ok(o),
                            }
                        }
                    }
                }

                Opcode::NewFunction
                | Opcode::NewGeneratorFunction
                | Opcode::NewAsyncFunction
                | Opcode::NewAsyncGeneratorFunction => {
                    let is_generator = op == Opcode::NewGeneratorFunction
                        || op == Opcode::NewAsyncGeneratorFunction;
                    let is_async =
                        op == Opcode::NewAsyncFunction || op == Opcode::NewAsyncGeneratorFunction;

                    let newfunc_start_pc = self.pc;

                    let cached: Option<std::sync::Arc<crate::compiler::opcode::NestedBytecode>> =
                        if let Some(parent_ptr) = self.frames[self.frame_index].function_ptr {
                            let parent_func = unsafe {
                                &*(parent_ptr as *const crate::object::function::JSFunction)
                            };
                            if let Some(bc) = parent_func.bytecode.as_ref() {
                                bc.nested_bytecodes.get(&(newfunc_start_pc as u32)).cloned()
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                    let mut nb_for_ic: Option<
                        std::sync::Arc<crate::compiler::opcode::NestedBytecode>,
                    > = None;

                    let (
                        fb_code,
                        fb_constants,
                        locals_count,
                        param_count,
                        uses_arguments,
                        is_strict,
                        func_var_name_to_slot,
                        upvalue_descs,
                        line_number_table_opt,
                        func_name_atom,
                    ) = if let Some(ref nb) = cached {
                        self.pc = newfunc_start_pc + nb.parent_bytecode_span as usize;
                        let uvdescs: Vec<(u32, u32)> = nb.upvalue_descs.clone();

                        (
                            Vec::new(),
                            Vec::new(),
                            nb.locals_count,
                            nb.param_count,
                            nb.uses_arguments,
                            nb.is_strict,
                            nb.var_name_to_slot.clone(),
                            uvdescs,
                            nb.line_number_table.clone(),
                            nb.func_name_atom,
                        )
                    } else {
                        let param_count = self.read_u16_pc();
                        let uses_arguments = self.read_u8() != 0;
                        let is_strict = self.read_u8() != 0;
                        let locals_count = self.read_u16_pc() as u32;
                        let bytecode_len = self.read_u32();
                        let mut fb_code = Vec::with_capacity(bytecode_len as usize);

                        unsafe {
                            let src = self.cached_code_ptr.add(self.pc);
                            fb_code.set_len(bytecode_len as usize);
                            std::ptr::copy_nonoverlapping(
                                src,
                                fb_code.as_mut_ptr(),
                                bytecode_len as usize,
                            );
                        }
                        self.pc += bytecode_len as usize;
                        let constants_len = self.read_u32();
                        let mut fb_constants = Vec::with_capacity(constants_len as usize);
                        for _ in 0..constants_len {
                            let const_type = self.read_u8();
                            match const_type {
                                0 => fb_constants.push(JSValue::undefined()),
                                1 => fb_constants.push(JSValue::null()),
                                2 => fb_constants.push(JSValue::bool(true)),
                                3 => fb_constants.push(JSValue::bool(false)),
                                4 => {
                                    let v = self.read_i64();
                                    fb_constants.push(JSValue::new_int(v));
                                }
                                5 => {
                                    let bytes = [
                                        self.read_u8(),
                                        self.read_u8(),
                                        self.read_u8(),
                                        self.read_u8(),
                                        self.read_u8(),
                                        self.read_u8(),
                                        self.read_u8(),
                                        self.read_u8(),
                                    ];
                                    let v = f64::from_le_bytes(bytes);
                                    fb_constants.push(JSValue::new_float(v));
                                }
                                6 => {
                                    let atom_id = self.read_u32() as u32;
                                    fb_constants.push(JSValue::new_string(
                                        crate::runtime::atom::Atom(atom_id),
                                    ));
                                }
                                _ => fb_constants.push(JSValue::undefined()),
                            }
                        }
                        let upvalue_count = self.read_u32();
                        let mut upvalue_descs = Vec::with_capacity(upvalue_count as usize);
                        for _ in 0..upvalue_count {
                            let atom_id = self.read_u32();
                            let local_idx = self.read_u32();
                            upvalue_descs.push((atom_id, local_idx));
                        }

                        let line_table_entry_count = self.read_u32();
                        let mut line_table = crate::compiler::location::LineNumberTable::new();
                        for _ in 0..line_table_entry_count {
                            let off = self.read_u32();
                            let line = self.read_u32();
                            line_table.add_entry(off, line);
                        }
                        let line_number_table_opt = if line_table_entry_count > 0 {
                            Some(line_table)
                        } else {
                            None
                        };
                        let func_name_atom = self.read_u32() as u32;
                        let var_count = self.read_u32();
                        let func_var_name_to_slot = std::rc::Rc::new({
                            let mut v = Vec::new();
                            for _ in 0..var_count {
                                let atom_id = self.read_u32();
                                let slot = self.read_u16_pc();
                                v.push((atom_id, slot));
                            }
                            v
                        });

                        let parent_bytecode_span = (self.pc - newfunc_start_pc) as u32;

                        if let Some(parent_ptr) = self.frames[self.frame_index].function_ptr {
                            let parent_func = unsafe {
                                &mut *(parent_ptr as *mut crate::object::function::JSFunction)
                            };
                            if let Some(bc) = parent_func.bytecode.as_mut() {
                                let nb = if let Some(existing) =
                                    bc.nested_bytecodes.get(&(newfunc_start_pc as u32))
                                {
                                    let arc = existing.clone();
                                    nb_for_ic = Some(arc.clone());
                                    arc
                                } else {
                                    let nb = std::sync::Arc::new(
                                        crate::compiler::opcode::NestedBytecode {
                                            code: fb_code.clone(),
                                            constants: fb_constants.clone(),
                                            locals_count,
                                            param_count,
                                            uses_arguments,
                                            is_strict,
                                            var_name_to_slot: func_var_name_to_slot.clone(),
                                            line_number_table: line_number_table_opt.clone(),
                                            parent_bytecode_span,
                                            upvalue_count: upvalue_descs.len() as u32,
                                            upvalue_descs: upvalue_descs.clone(),
                                            func_name_atom,
                                            ic_table: std::cell::UnsafeCell::new(
                                                crate::compiler::InlineCacheTable::new(),
                                            ),
                                        },
                                    );
                                    nb_for_ic = Some(nb.clone());
                                    bc.nested_bytecodes
                                        .insert(newfunc_start_pc as u32, nb.clone());
                                    nb
                                };
                                let _ = nb;
                            }
                        }
                        (
                            fb_code,
                            fb_constants,
                            locals_count,
                            param_count,
                            uses_arguments,
                            is_strict,
                            func_var_name_to_slot,
                            upvalue_descs,
                            line_number_table_opt,
                            func_name_atom,
                        )
                    };

                    let mut func = crate::object::function::JSFunction::new();

                    if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
                        func.base.prototype = Some(fn_proto_ptr);
                    }
                    func.param_count = param_count as u32;
                    func.arity = param_count as u32;
                    func.locals_count = locals_count;
                    func.set_is_generator(is_generator);
                    func.set_is_async(is_async);
                    func.set_uses_arguments(uses_arguments);
                    func.set_is_strict(is_strict);
                    func.line_number_table = line_number_table_opt.clone();
                    func.bytecode = Some(Box::new(Bytecode {
                        code: fb_code,
                        constants: fb_constants,
                        locals_count,
                        param_count,
                        line_number_table: line_number_table_opt,
                        ic_table: crate::compiler::InlineCacheTable::new(),
                        shared_ic_table_ptr: std::ptr::null_mut(),
                        shared_code_ptr: std::ptr::null(),
                        shared_code_len: 0,
                        shared_const_ptr: std::ptr::null(),
                        shared_const_len: 0,
                        uses_arguments,
                        is_strict,
                        var_name_to_slot: func_var_name_to_slot,
                        nested_bytecodes: std::collections::HashMap::new(),
                        is_simple_constructor: false,
                        simple_constructor_props: Vec::new(),
                        cached_constructor_final_shape: None,
                        cached_constructor_atoms: Vec::new(),
                    }));

                    let effective_nb = nb_for_ic.or_else(|| cached.clone());
                    if let Some(ref nb_arc) = effective_nb {
                        if let Some(bc) = func.bytecode.as_mut() {
                            bc.shared_ic_table_ptr = nb_arc.ic_table.get();

                            if bc.code.is_empty() {
                                bc.shared_code_ptr = nb_arc.code.as_ptr();
                                bc.shared_code_len = nb_arc.code.len();
                            }
                            if bc.constants.is_empty() {
                                bc.shared_const_ptr = nb_arc.constants.as_ptr();
                                bc.shared_const_len = nb_arc.constants.len();
                            }
                        }
                    }
                    func.shared_nb_for_ic = effective_nb;

                    let inherited_sentinel = u16::MAX as usize;
                    let current_frame_base = self.frames[self.frame_index].registers_base;
                    for (atom_id, local_idx_raw) in &upvalue_descs {
                        let atom = crate::runtime::atom::Atom(*atom_id);
                        let local_idx = *local_idx_raw as usize;
                        func.upvalues_mut().upvalue_slot_atoms.push(atom);

                        let cell = if local_idx != inherited_sentinel {
                            func.upvalues_mut()
                                .upvalue_local_indices
                                .insert(atom, local_idx);
                            let initial_value =
                                if local_idx < self.frames[self.frame_index].registers_count {
                                    self.registers[current_frame_base + local_idx]
                                } else {
                                    JSValue::undefined()
                                };

                            let current_frame = &mut self.frames[self.frame_index];
                            let local_idx_u16 = local_idx as u16;
                            if let Some(existing) = current_frame
                                .upvalue_sync_map
                                .as_ref()
                                .and_then(|m| m.get(&local_idx_u16))
                            {
                                existing.clone()
                            } else {
                                let new_cell =
                                    std::rc::Rc::new(std::cell::Cell::new(initial_value));
                                current_frame
                                    .upvalue_sync_map
                                    .get_or_insert_with(|| Box::new(FxHashMap::default()))
                                    .insert(local_idx_u16, new_cell.clone());
                                if local_idx_u16 < 64 {
                                    current_frame.upvalue_sync_bitset |= 1u64 << local_idx_u16;
                                    self.cached_upvalue_sync_bitset |= 1u64 << local_idx_u16;
                                }
                                self.cached_has_upvalue_sync = true;
                                new_cell
                            }
                        } else if let Some(parent_ptr) = self.frames[self.frame_index].function_ptr
                        {
                            let parent_func = unsafe {
                                &*(parent_ptr as *const crate::object::function::JSFunction)
                            };

                            if let Some(parent_cell) = parent_func
                                .upvalues_ref()
                                .and_then(|u| u.upvalue_cells.get(&atom))
                            {
                                parent_cell.clone()
                            } else {
                                std::rc::Rc::new(std::cell::Cell::new(JSValue::undefined()))
                            }
                        } else {
                            std::rc::Rc::new(std::cell::Cell::new(JSValue::undefined()))
                        };
                        func.upvalues_mut().upvalue_slots.push(cell.clone());
                        func.upvalues_mut().upvalue_cells.insert(atom, cell);
                    }

                    if func_name_atom != 0 {
                        func.name = crate::runtime::atom::Atom(func_name_atom);
                    }

                    let is_closure = !upvalue_descs.is_empty();

                    let proto_atom = ctx.common_atoms.prototype;
                    let func_ptr = Box::into_raw(Box::new(func)) as usize;
                    ctx.runtime_mut().gc_heap_mut().track_function(func_ptr);
                    let unboxed_func =
                        unsafe { &mut *(func_ptr as *mut crate::object::function::JSFunction) };
                    if !is_closure {
                        let proto_obj = crate::object::object::JSObject::new();
                        let proto_ptr = Box::into_raw(Box::new(proto_obj)) as usize;
                        ctx.runtime_mut().gc_heap_mut().track(proto_ptr);
                        let func_value = JSValue::new_function(func_ptr);
                        unsafe {
                            let proto_obj_mut =
                                &mut *(proto_ptr as *mut crate::object::object::JSObject);
                            proto_obj_mut.set(ctx.common_atoms.constructor, func_value);
                        }
                        unboxed_func
                            .base
                            .set(proto_atom, JSValue::new_object(proto_ptr));
                        unboxed_func.cached_prototype_ptr =
                            proto_ptr as *mut crate::object::object::JSObject;
                    }
                    if let Some(fn_pp) = ctx.get_function_prototype() {
                        unboxed_func.base.prototype = Some(fn_pp);
                    } else if let Some(obj_pp) = ctx.get_object_prototype() {
                        unboxed_func.base.prototype = Some(obj_pp);
                    }
                    self.allocation_count += 1;
                    let dst_reg = self.read_u16_pc();
                    self.set_reg(dst_reg, JSValue::new_function(func_ptr));
                    self.maybe_gc(ctx);
                }
                Opcode::LoadTdz => {
                    let dst = self.read_u16_pc();
                    self.set_reg(dst, JSValue::new_tdz());
                }
                Opcode::CheckTdz => {
                    let reg = self.read_u16_pc();
                    if self.get_reg(reg).is_tdz() {
                        return Err(
                            "ReferenceError: Cannot access variable before initialization"
                                .to_string(),
                        );
                    }
                }
                Opcode::CheckRef => {
                    let reg = self.read_u16_pc();
                    let idx = self.read_u32() as usize;

                    let reg_val = self.get_reg(reg);
                    if !reg_val.is_undefined()
                        && !reg_val.is_tdz()
                        && self.eval_binding_frames == 0
                        && self.caller_vm.is_none()
                    {
                        continue;
                    }
                    let name = if idx < self.frames[self.frame_index].constants_len {
                        unsafe { *self.cached_const_ptr.add(idx) }
                    } else {
                        JSValue::undefined()
                    };
                    let atom = name.get_atom();
                    let mut exists = false;
                    let has_eval = self.eval_binding_frames > 0 || self.caller_vm.is_some();
                    if has_eval {
                        for fi in (0..=self.frame_index).rev() {
                            if let Some(ref eb) = self.frames[fi].eval_bindings {
                                if eb.contains_key(&atom.0) {
                                    exists = true;
                                    break;
                                }
                            }
                        }
                        if !exists && self.get_var_in_caller_vm(atom.0).is_some() {
                            exists = true;
                        }
                    }
                    if !exists {
                        let global = ctx.global();
                        if global.is_object() {
                            exists = global.as_object().get_own(atom).is_some();
                        }
                    }
                    if !exists {
                        let ref_err_atom = ctx.intern("ReferenceError");
                        let name_str = ctx.get_atom_str(atom).to_string();
                        let err_msg = format!("{} is not defined", name_str);
                        let msg_atom = ctx.intern(&err_msg);
                        let mut err = crate::object::object::JSObject::new();
                        err.set(ctx.intern("name"), JSValue::new_string(ref_err_atom));
                        err.set(ctx.intern("message"), JSValue::new_string(msg_atom));
                        if let Some(proto) = ctx.get_reference_error_prototype() {
                            err.prototype = Some(proto as *mut _);
                        } else if let Some(proto) = ctx.get_error_prototype() {
                            err.prototype = Some(proto as *mut _);
                        }
                        let ptr = Box::into_raw(Box::new(err)) as usize;
                        ctx.runtime_mut().gc_heap_mut().track(ptr);
                        self.pending_throw = Some(JSValue::new_object(ptr));
                        if let Some(exc) = self.pending_throw.take() {
                            match self.dispatch_throw_value(ctx, exc) {
                                ThrowDispatch::Caught => continue,
                                ThrowDispatch::Uncaught(e) => return Err(e),
                                ThrowDispatch::AsyncComplete(_) => continue,
                            }
                        }
                    }
                }
                Opcode::CheckObjectCoercible => {
                    let reg = self.read_u16_pc();
                    let val = self.get_reg(reg);
                    if val.is_null() || val.is_undefined() {
                        let type_err_atom = ctx.intern("TypeError");
                        let msg = "Cannot destructure 'undefined' or 'null'.";
                        let msg_atom = ctx.intern(msg);
                        let mut err = crate::object::object::JSObject::new();
                        err.set(ctx.intern("name"), JSValue::new_string(type_err_atom));
                        err.set(ctx.intern("message"), JSValue::new_string(msg_atom));
                        if let Some(proto) = ctx.get_type_error_prototype() {
                            err.prototype = Some(proto as *mut _);
                        } else if let Some(proto) = ctx.get_error_prototype() {
                            err.prototype = Some(proto as *mut _);
                        }
                        let ptr = Box::into_raw(Box::new(err)) as usize;
                        ctx.runtime_mut().gc_heap_mut().track(ptr);
                        let exc = JSValue::new_object(ptr);
                        self.pending_throw = Some(exc);
                        if let Some(thrown) = self.pending_throw.take() {
                            match self.dispatch_throw_value(ctx, thrown) {
                                ThrowDispatch::Caught => continue,
                                ThrowDispatch::Uncaught(e) => {
                                    self.pending_throw = Some(exc);
                                    return Err(e);
                                }
                                ThrowDispatch::AsyncComplete(_) => continue,
                            }
                        }
                    }
                }
                Opcode::GetIterator => {
                    let dst = self.read_u16_pc();
                    let src = self.read_u16_pc();
                    let iterable = self.get_reg(src);
                    let result = if iterable.is_object() || iterable.is_function() {
                        let arr_ptr = iterable.get_ptr();
                        let is_array = iterable.as_object().is_dense_array();
                        if is_array {
                            self.create_iter_object(ctx, iterable)
                        } else if iterable.is_string() {
                            self.create_iter_object(ctx, iterable)
                        } else {
                            let obj: &crate::object::object::JSObject = if iterable.is_function() {
                                let func = iterable.as_function();
                                &func.base
                            } else {
                                iterable.as_object()
                            };

                            let mut sym_iter_atom = None;
                            let global = ctx.global();
                            if global.is_object() {
                                if let Some(sym_val) =
                                    global.as_object().get(ctx.intern("Symbol.iterator"))
                                {
                                    if sym_val.is_symbol() {
                                        sym_iter_atom = Some(crate::runtime::atom::Atom(
                                            0x40000000 | sym_val.get_symbol_id(),
                                        ));
                                    }
                                }
                            }
                            let iter_fn = sym_iter_atom.and_then(|a| obj.get(a)).or_else(|| {
                                let mut current = obj.prototype;
                                while let Some(p) = current {
                                    let pobj = unsafe { &*p };
                                    if let Some(v) = sym_iter_atom.and_then(|a| pobj.get(a)) {
                                        return Some(v);
                                    }
                                    current = pobj.prototype;
                                }
                                None
                            });
                            if let Some(iter_fn) = iter_fn.or_else(|| {
                                let str_atom = ctx.intern("Symbol.iterator");
                                obj.get(str_atom).or_else(|| {
                                    let mut proto = obj.prototype;
                                    while let Some(p) = proto {
                                        let pobj = unsafe { &*p };
                                        if let Some(v) = pobj.get(str_atom) {
                                            return Some(v);
                                        }
                                        proto = pobj.prototype;
                                    }
                                    None
                                })
                            }) {
                                if iter_fn.is_function() {
                                    let func_ptr = iter_fn.get_ptr();
                                    let js_func = unsafe {
                                        &*(func_ptr as *const crate::object::function::JSFunction)
                                    };
                                    if js_func.is_builtin() {
                                        if let Some(builtin_fn) = js_func.builtin_func {
                                            ctx.call_builtin_direct(builtin_fn, &[iterable])
                                        } else {
                                            JSValue::undefined()
                                        }
                                    } else {
                                        let obj2 = iterable.as_object();
                                        let len_atom = ctx.common_atoms.length;
                                        if obj2.get(len_atom).is_some() {
                                            let mut iter_obj =
                                                crate::object::object::JSObject::new();
                                            let arr_atom = ctx.common_atoms.__iter_arr__;
                                            let idx_atom = ctx.common_atoms.__iter_idx__;
                                            iter_obj.set_cached(
                                                arr_atom,
                                                iterable,
                                                ctx.shape_cache_mut(),
                                            );
                                            iter_obj.set_cached(
                                                idx_atom,
                                                JSValue::new_int(0),
                                                ctx.shape_cache_mut(),
                                            );
                                            let iter_ptr =
                                                Box::into_raw(Box::new(iter_obj)) as usize;
                                            ctx.runtime_mut().gc_heap_mut().track(iter_ptr);
                                            self.allocation_count += 1;

                                            JSValue::new_object(iter_ptr)
                                        } else {
                                            JSValue::undefined()
                                        }
                                    }
                                } else {
                                    let obj2: &crate::object::object::JSObject = unsafe {
                                        &*(arr_ptr as *const crate::object::object::JSObject)
                                    };
                                    let len_atom = ctx.common_atoms.length;
                                    if obj2.get(len_atom).is_some() {
                                        let mut iter_obj = crate::object::object::JSObject::new();
                                        let arr_atom = ctx.common_atoms.__iter_arr__;
                                        let idx_atom = ctx.common_atoms.__iter_idx__;
                                        iter_obj.set_cached(
                                            arr_atom,
                                            iterable,
                                            ctx.shape_cache_mut(),
                                        );
                                        iter_obj.set_cached(
                                            idx_atom,
                                            JSValue::new_int(0),
                                            ctx.shape_cache_mut(),
                                        );
                                        let iter_ptr = Box::into_raw(Box::new(iter_obj)) as usize;
                                        ctx.runtime_mut().gc_heap_mut().track(iter_ptr);
                                        self.allocation_count += 1;

                                        JSValue::new_object(iter_ptr)
                                    } else {
                                        JSValue::undefined()
                                    }
                                }
                            } else {
                                let obj2: &crate::object::object::JSObject = unsafe {
                                    &*(arr_ptr as *const crate::object::object::JSObject)
                                };
                                let len_atom = ctx.common_atoms.length;
                                if obj2.get(len_atom).is_some() {
                                    let mut iter_obj = crate::object::object::JSObject::new();
                                    let arr_atom = ctx.common_atoms.__iter_arr__;
                                    let idx_atom = ctx.common_atoms.__iter_idx__;
                                    iter_obj.set_cached(arr_atom, iterable, ctx.shape_cache_mut());
                                    iter_obj.set_cached(
                                        idx_atom,
                                        JSValue::new_int(0),
                                        ctx.shape_cache_mut(),
                                    );
                                    let iter_ptr = Box::into_raw(Box::new(iter_obj)) as usize;
                                    ctx.runtime_mut().gc_heap_mut().track(iter_ptr);
                                    self.allocation_count += 1;

                                    JSValue::new_object(iter_ptr)
                                } else {
                                    JSValue::undefined()
                                }
                            }
                        }
                    } else if iterable.is_string() {
                        self.create_iter_object(ctx, iterable)
                    } else {
                        JSValue::undefined()
                    };
                    self.set_reg(dst, result);
                }
                Opcode::IteratorNext => {
                    let dst_val = self.read_u16_pc();
                    let dst_done = self.read_u16_pc();
                    let iter_reg = self.read_u16_pc();
                    let iter_val = self.get_reg(iter_reg);

                    if !iter_val.is_object() {
                        self.set_reg(dst_val, JSValue::undefined());
                        self.set_reg(dst_done, JSValue::bool(true));
                        continue;
                    }

                    let iter_ptr = iter_val.get_ptr();
                    let iter_obj =
                        unsafe { &mut *(iter_ptr as *mut crate::object::object::JSObject) };
                    let arr_atom = ctx.common_atoms.__iter_arr__;

                    if let Some(arr_val) = iter_obj.get(arr_atom) {
                        let idx_atom = ctx.common_atoms.__iter_idx__;
                        let idx = iter_obj.get(idx_atom).map(|v| v.get_int()).unwrap_or(0) as usize;

                        if arr_val.is_string() {
                            let atom = arr_val.get_atom();
                            let s = ctx.atom_table().get(atom);
                            let chars: Vec<char> = s.chars().collect();
                            if idx < chars.len() {
                                let ch = chars[idx].to_string();
                                let ch_atom = ctx.atom_table_mut().intern(&ch);
                                let ch_val = JSValue::new_string(ch_atom);
                                iter_obj.set_cached(
                                    idx_atom,
                                    JSValue::new_int((idx + 1) as i64),
                                    ctx.shape_cache_mut(),
                                );
                                self.set_reg(dst_val, ch_val);
                                self.set_reg(dst_done, JSValue::bool(false));
                            } else {
                                self.set_reg(dst_val, JSValue::undefined());
                                self.set_reg(dst_done, JSValue::bool(true));
                            }
                        } else if arr_val.is_object() {
                            let arr_ptr = arr_val.get_ptr();
                            let is_jsarray =
                                unsafe { &*(arr_ptr as *const crate::object::object::JSObject) }
                                    .is_dense_array();
                            let length = if is_jsarray {
                                let arr = unsafe {
                                    &*(arr_ptr as *const crate::object::array_obj::JSArrayObject)
                                };
                                arr.len()
                            } else {
                                let obj = unsafe {
                                    &*(arr_ptr as *const crate::object::object::JSObject)
                                };
                                let len_atom = ctx.common_atoms.length;
                                obj.get(len_atom).map(|v| v.get_int() as usize).unwrap_or(0)
                            };

                            if idx < length {
                                let value = if is_jsarray {
                                    let arr = unsafe {
                                        &*(arr_ptr
                                            as *const crate::object::array_obj::JSArrayObject)
                                    };
                                    arr.get(idx).unwrap_or(JSValue::undefined())
                                } else {
                                    let obj = unsafe {
                                        &*(arr_ptr as *const crate::object::object::JSObject)
                                    };
                                    let key = self.int_atom(idx, ctx);
                                    obj.get(key).unwrap_or(JSValue::undefined())
                                };
                                iter_obj.set_cached(
                                    idx_atom,
                                    JSValue::new_int((idx + 1) as i64),
                                    ctx.shape_cache_mut(),
                                );
                                self.set_reg(dst_val, value);
                                self.set_reg(dst_done, JSValue::bool(false));
                            } else {
                                self.set_reg(dst_val, JSValue::undefined());
                                self.set_reg(dst_done, JSValue::bool(true));
                            }
                        } else {
                            self.set_reg(dst_val, JSValue::undefined());
                            self.set_reg(dst_done, JSValue::bool(true));
                        }
                    } else {
                        self.set_reg(dst_val, JSValue::undefined());
                        self.set_reg(dst_done, JSValue::bool(true));
                    }
                }
                Opcode::GetArguments => {
                    let dst = self.read_u16_pc();
                    let fi = self.frame_index;
                    if let Some(cached_ptr) = self.frames[fi].cached_arguments {
                        self.set_reg(dst, JSValue::new_object(cached_ptr));
                    } else {
                        let frame = &self.frames[fi];
                        let func_ptr = frame.function_ptr;
                        let saved_args = &frame.saved_args;
                        let arg_count = saved_args.len();
                        let is_strict = frame.is_strict_frame;
                        let use_mapped = !is_strict && func_ptr.is_some();
                        use crate::object::object::{
                            ATTR_CONFIGURABLE, ATTR_WRITABLE, PropertyDescriptor,
                        };
                        let mut args_obj = crate::object::object::JSObject::new();
                        if let Some(obj_proto) = ctx.get_object_prototype() {
                            args_obj.prototype = Some(obj_proto);
                        }

                        let args_shape = ctx.get_or_create_args_length_shape();
                        args_obj.set_first_prop_with_shape(
                            ctx.common_atoms.length,
                            JSValue::new_int(arg_count as i64),
                            ATTR_WRITABLE | ATTR_CONFIGURABLE,
                            args_shape,
                        );
                        let declared_param_count = if use_mapped {
                            if let Some(ptr) = func_ptr {
                                let js_func = unsafe { JSValue::function_from_ptr(ptr) };
                                js_func.param_count
                            } else {
                                0
                            }
                        } else {
                            0
                        };
                        if use_mapped {
                            args_obj
                                .set_obj_type(crate::object::object::ObjectType::MappedArguments);
                            {
                                let extra = args_obj.ensure_extra();
                                extra.mapped_args_frame_index = fi;
                                extra.mapped_args_param_count = declared_param_count;
                            }
                            let start = declared_param_count as usize;
                            if start < arg_count {
                                args_obj
                                    .ensure_elements()
                                    .resize(arg_count, JSValue::undefined());
                                for i in start..arg_count {
                                    args_obj.set_indexed(i, saved_args[i]);
                                }
                            }
                        } else {
                            if arg_count > 0 {
                                args_obj
                                    .ensure_elements()
                                    .resize(arg_count, JSValue::undefined());
                                for (i, val) in saved_args.iter().enumerate() {
                                    args_obj.set_indexed(i, *val);
                                }
                            }
                        }
                        let callee_atom = ctx.common_atoms.callee;
                        if let Some(ptr) = func_ptr {
                            if is_strict {
                                let mut thrower = crate::object::function::JSFunction::new_builtin(
                                    ctx.intern("callee"),
                                    0,
                                );
                                thrower.builtin_atom = Some(ctx.intern("throw_type_error_callee"));
                                thrower.builtin_func =
                                    ctx.get_builtin_func("throw_type_error_callee");
                                let thrower_ptr = Box::into_raw(Box::new(thrower)) as usize;
                                ctx.runtime_mut().gc_heap_mut().track_function(thrower_ptr);
                                args_obj.define_accessor(
                                    callee_atom,
                                    Some(JSValue::new_function(thrower_ptr)),
                                    None,
                                );
                            } else {
                                args_obj.define_property(
                                    callee_atom,
                                    PropertyDescriptor {
                                        value: Some(JSValue::new_function(ptr)),
                                        writable: true,
                                        enumerable: false,
                                        configurable: true,
                                        get: None,
                                        set: None,
                                    },
                                );
                            }
                        }

                        let ptr = if ctx
                            .runtime_mut()
                            .gc_heap_mut()
                            .nursery_enabled_and_can_fit_object()
                        {
                            ctx.runtime_mut()
                                .gc_heap_mut()
                                .alloc_object_with_value(args_obj)
                                .expect("nursery alloc failed despite can_fit")
                                as usize
                        } else {
                            let hp = Box::into_raw(Box::new(args_obj)) as usize;
                            ctx.runtime_mut().gc_heap_mut().track(hp);
                            hp
                        };
                        self.frames[fi].cached_arguments = Some(ptr);
                        self.set_reg(dst, JSValue::new_object(ptr));
                        self.allocation_count += 1;
                        self.maybe_gc(ctx);
                    }
                }
                Opcode::GetCurrentFunction => {
                    let dst = self.read_u16_pc();
                    let value = if let Some(ptr) = self.frames[self.frame_index].function_ptr {
                        JSValue::new_function(ptr)
                    } else {
                        JSValue::undefined()
                    };
                    self.set_reg(dst, value);
                }
            }
        }
    }

    pub fn execute_generator_step(
        &mut self,
        ctx: &mut JSContext,
        bytecode: &Bytecode,
        registers_snapshot: &[JSValue],
        start_pc: usize,
    ) -> Result<(JSValue, Vec<JSValue>, usize, bool), String> {
        self.ctx_ptr = ctx;

        let needed = bytecode.locals_count as usize;
        if needed > self.registers.len() {
            self.registers.resize(needed, JSValue::undefined());
        }
        for (i, val) in registers_snapshot.iter().enumerate() {
            if i < self.registers.len() {
                self.registers[i] = *val;
            }
        }
        for i in registers_snapshot.len()..needed {
            if i < self.registers.len() {
                self.registers[i] = JSValue::undefined();
            }
        }

        let result = self.execute_inner(ctx, bytecode, true, start_pc, true)?;
        match result {
            ExecutionOutcome::Complete(val) => Ok((val, self.registers.clone(), self.pc, true)),
            ExecutionOutcome::Yield(val) => Ok((val, self.registers.clone(), self.pc, false)),
        }
    }

    #[inline(always)]
    fn read_u16_pc(&mut self) -> u16 {
        let val =
            unsafe { std::ptr::read_unaligned(self.cached_code_ptr.add(self.pc) as *const u16) };
        self.pc += 2;
        val
    }

    #[inline(always)]
    fn read_u32_pc(&mut self) -> u32 {
        let val =
            unsafe { std::ptr::read_unaligned(self.cached_code_ptr.add(self.pc) as *const u32) };
        self.pc += 4;
        val
    }

    #[inline(always)]
    fn read_i32_pc(&mut self) -> i32 {
        let val =
            unsafe { std::ptr::read_unaligned(self.cached_code_ptr.add(self.pc) as *const i32) };
        self.pc += 4;
        val
    }

    #[inline(always)]
    fn get_named_prop_result(
        &mut self,
        ctx: &mut JSContext,
        dst: u16,
        obj_val: JSValue,
        atom: crate::runtime::atom::Atom,
        ic_pc: usize,
    ) -> Option<JSValue> {
        if !obj_val.is_object_like()
            && !obj_val.is_function()
            && (obj_val.is_int() || obj_val.is_float() || obj_val.is_string() || obj_val.is_bool())
        {
            if obj_val.is_string() && atom == ctx.common_atoms.length {
                let len = ctx.string_char_count(obj_val.get_atom()) as i64;
                let v = JSValue::new_int(len);
                self.set_reg(dst, v);
                return Some(v);
            }
            let start_proto = if obj_val.is_string() {
                ctx.get_string_prototype()
            } else if obj_val.is_int() || obj_val.is_float() {
                ctx.get_number_prototype()
            } else {
                ctx.get_object_prototype()
            };
            let mut current = start_proto;
            while let Some(ptr) = current {
                let pobj = unsafe { &*ptr };
                if let Some(getter) = pobj.get_own_accessor_value(atom) {
                    if getter.is_function() {
                        let js_func = getter.as_function();
                        let fn_this = if !js_func.is_strict()
                            && (obj_val.is_int()
                                || obj_val.is_float()
                                || obj_val.is_string()
                                || obj_val.is_bool())
                        {
                            let mut wrapper = crate::object::object::JSObject::new();
                            if let Some(opp) = ctx.get_object_prototype() {
                                wrapper.set_prototype_raw(opp);
                            }
                            if obj_val.is_string() || obj_val.is_int() || obj_val.is_float() {
                                wrapper.set_cached(
                                    crate::runtime::atom::Atom(0),
                                    obj_val,
                                    ctx.shape_cache_mut(),
                                );
                            }
                            JSValue::new_object(Box::into_raw(Box::new(wrapper)) as usize)
                        } else {
                            obj_val
                        };
                        if let Ok(v) = self.call_function_with_this(ctx, getter, fn_this, &[]) {
                            self.set_reg(dst, v);
                            return Some(v);
                        }
                    }
                    self.set_reg(dst, JSValue::undefined());
                    return Some(JSValue::undefined());
                }
                if let Some(val) = pobj.get_own(atom) {
                    self.set_reg(dst, val);

                    let ic_table_ptr = self.cached_ic_table_ptr;
                    if !ic_table_ptr.is_null() {
                        let pc = ic_pc;
                        let pseudo_id = if obj_val.is_string() {
                            PRIM_STRING_SHAPE_ID
                        } else {
                            PRIM_NUMBER_SHAPE_ID
                        };
                        if let Some(offset) = pobj.find_offset(atom) {
                            unsafe {
                                (*ic_table_ptr).ensure_capacity(pc + 1);
                                if let Some(ic) = (*ic_table_ptr).get_mut(pc) {
                                    ic.insert(
                                        crate::object::shape::ShapeId(pseudo_id),
                                        offset as u32,
                                        Some(ptr as usize),
                                    );
                                }
                            }
                        }
                    }
                    return Some(val);
                }
                current = pobj.prototype;
            }
            self.set_reg(dst, JSValue::undefined());
            return Some(JSValue::undefined());
        }
        if obj_val.is_object_like() {
            let ptr = obj_val.get_ptr();
            let js_obj = unsafe { JSValue::object_from_ptr(ptr) };
            let ic_table_ptr = self.cached_ic_table_ptr;

            let pc = ic_pc;
            let mut ic_hit = false;
            if let Some(shape_id) = js_obj.get_shape_id() {
                if !ic_table_ptr.is_null() {
                    let ic_table = unsafe { &*ic_table_ptr };
                    if let Some(ic) = ic_table.get(pc) {
                        if let Some((offset, proto_ptr)) = ic.get(shape_id) {
                            if offset == u32::MAX && proto_ptr.is_none() {
                                self.set_reg(dst, JSValue::undefined());
                                ic_hit = true;
                            } else {
                                let val = if let Some(proto) = proto_ptr {
                                    let mut proto_matches = js_obj
                                        .prototype
                                        .map(|p| !p.is_null() && p as usize == proto)
                                        .unwrap_or(false);
                                    if !proto_matches {
                                        let mut cur = js_obj.prototype;
                                        let mut depth = 0u32;
                                        while let Some(p) = cur {
                                            if p.is_null() || depth > 1000 {
                                                break;
                                            }
                                            if p as usize == proto {
                                                proto_matches = true;
                                                break;
                                            }
                                            depth += 1;
                                            unsafe {
                                                cur = (*p).prototype;
                                            }
                                        }
                                    }
                                    if proto_matches {
                                        let proto_obj = unsafe {
                                            &*(proto as *const crate::object::object::JSObject)
                                        };
                                        proto_obj.get_by_offset(offset as usize)
                                    } else {
                                        None
                                    }
                                } else {
                                    js_obj.get_by_offset(offset as usize)
                                };
                                if let Some(v) = val {
                                    self.set_reg(dst, v);
                                    ic_hit = true;
                                }
                            }
                        }
                    }
                }
            }

            if !ic_hit {
                if let Some(shape_id) = js_obj.get_shape_id() {
                    if let Some(offset) = js_obj.find_offset(atom) {
                        let v = js_obj.get_by_offset(offset).unwrap_or(JSValue::undefined());
                        self.set_reg(dst, v);
                        if !ic_table_ptr.is_null() {
                            unsafe {
                                (*ic_table_ptr).ensure_capacity(pc + 1);
                                if let Some(ic) = (*ic_table_ptr).get_mut(pc) {
                                    ic.insert(shape_id, offset as u32, None);
                                }
                            }
                        }
                    } else {
                        if let Some(getter) = js_obj.get_own_accessor_value(atom) {
                            if getter.is_function() {
                                match self.call_function_with_this(ctx, getter, obj_val, &[]) {
                                    Ok(ret) => {
                                        self.set_reg(dst, ret);
                                    }
                                    Err(msg) => {
                                        return self.throw_reference_error(ctx, &msg);
                                    }
                                }
                            } else {
                                self.set_pending_type_error(
                                    ctx,
                                    "Property getter is not a function",
                                );
                            }
                        } else {
                            let mut current = js_obj.prototype;
                            let mut depth = 0u32;
                            let mut found = false;
                            while let Some(proto_ptr) = current {
                                if proto_ptr.is_null() || depth > 1000 {
                                    break;
                                }
                                depth += 1;
                                unsafe {
                                    let proto = &*proto_ptr;
                                    if let Some(offset) = proto.find_offset(atom) {
                                        let v = proto
                                            .get_by_offset(offset)
                                            .unwrap_or(JSValue::undefined());
                                        self.set_reg(dst, v);
                                        if !ic_table_ptr.is_null() {
                                            (*ic_table_ptr).ensure_capacity(pc + 1);
                                            if let Some(ic) = (*ic_table_ptr).get_mut(pc) {
                                                ic.insert(
                                                    shape_id,
                                                    offset as u32,
                                                    Some(proto_ptr as usize),
                                                );
                                            }
                                        }
                                        found = true;
                                        break;
                                    }
                                    if let Some(getter) = proto.get_own_accessor_value(atom) {
                                        if getter.is_function() {
                                            match self.call_function_with_this(
                                                ctx,
                                                getter,
                                                obj_val,
                                                &[],
                                            ) {
                                                Ok(ret) => {
                                                    self.set_reg(dst, ret);
                                                }
                                                Err(msg) => {
                                                    let mut err =
                                                        crate::object::object::JSObject::new();
                                                    err.set(
                                                        ctx.intern("name"),
                                                        JSValue::new_string(
                                                            ctx.intern("ReferenceError"),
                                                        ),
                                                    );
                                                    err.set(
                                                        ctx.intern("message"),
                                                        JSValue::new_string(ctx.intern(&msg)),
                                                    );
                                                    if let Some(proto) =
                                                        ctx.get_reference_error_prototype()
                                                    {
                                                        err.prototype = Some(proto);
                                                    }
                                                    let ptr = Box::into_raw(Box::new(err)) as usize;
                                                    ctx.runtime_mut().gc_heap_mut().track(ptr);
                                                    self.pending_throw =
                                                        Some(JSValue::new_object(ptr));
                                                    return None;
                                                }
                                            }
                                        } else {
                                            self.set_pending_type_error(
                                                ctx,
                                                "Property getter is not a function",
                                            );
                                        }
                                        found = true;
                                        break;
                                    }
                                    current = proto.prototype;
                                }
                            }
                            if !found && obj_val.is_function() && js_obj.prototype.is_none() {
                                if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
                                    let fn_proto = unsafe { &*fn_proto_ptr };
                                    if let Some(offset) = fn_proto.find_offset(atom) {
                                        let v = fn_proto
                                            .get_by_offset(offset)
                                            .unwrap_or(JSValue::undefined());
                                        self.set_reg(dst, v);
                                        found = true;
                                    } else if let Some(getter) =
                                        fn_proto.get_own_accessor_value(atom)
                                    {
                                        if getter.is_function() {
                                            match self.call_function_with_this(
                                                ctx,
                                                getter,
                                                obj_val,
                                                &[],
                                            ) {
                                                Ok(ret) => {
                                                    self.set_reg(dst, ret);
                                                }
                                                Err(msg) => {
                                                    let mut err =
                                                        crate::object::object::JSObject::new();
                                                    err.set(
                                                        ctx.intern("name"),
                                                        JSValue::new_string(
                                                            ctx.intern("ReferenceError"),
                                                        ),
                                                    );
                                                    err.set(
                                                        ctx.intern("message"),
                                                        JSValue::new_string(ctx.intern(&msg)),
                                                    );
                                                    if let Some(proto) =
                                                        ctx.get_reference_error_prototype()
                                                    {
                                                        err.prototype = Some(proto);
                                                    }
                                                    let ptr = Box::into_raw(Box::new(err)) as usize;
                                                    ctx.runtime_mut().gc_heap_mut().track(ptr);
                                                    self.pending_throw =
                                                        Some(JSValue::new_object(ptr));
                                                    return None;
                                                }
                                            }
                                        } else {
                                            self.set_pending_type_error(
                                                ctx,
                                                "Property getter is not a function",
                                            );
                                        }
                                        found = true;
                                    }
                                }
                            }
                            if !found {
                                self.set_reg(dst, JSValue::undefined());
                                if !ic_table_ptr.is_null() {
                                    unsafe {
                                        (*ic_table_ptr).ensure_capacity(pc + 1);
                                        if let Some(ic) = (*ic_table_ptr).get_mut(pc) {
                                            ic.insert(shape_id, u32::MAX, None);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    let mut value = JSValue::undefined();
                    let mut found = false;

                    if let Some(offset) = js_obj.find_offset(atom) {
                        value = js_obj.get_by_offset(offset).unwrap_or(JSValue::undefined());
                        found = true;
                    } else if let Some(getter) = js_obj.get_own_accessor_value(atom) {
                        value = if getter.is_function() {
                            match self.call_function_with_this(ctx, getter, obj_val, &[]) {
                                Ok(ret) => ret,
                                Err(msg) => {
                                    self.set_pending_type_error(ctx, &msg);
                                    JSValue::undefined()
                                }
                            }
                        } else {
                            self.set_pending_type_error(ctx, "Property getter is not a function");
                            JSValue::undefined()
                        };
                        found = true;
                    } else {
                        let mut current = js_obj.prototype;
                        let mut depth = 0u32;
                        while let Some(proto_ptr) = current {
                            if proto_ptr.is_null() || depth > 1000 {
                                break;
                            }
                            depth += 1;
                            unsafe {
                                let proto = &*proto_ptr;
                                if let Some(offset) = proto.find_offset(atom) {
                                    value =
                                        proto.get_by_offset(offset).unwrap_or(JSValue::undefined());
                                    found = true;
                                    break;
                                }
                                if let Some(getter) = proto.get_own_accessor_value(atom) {
                                    value = if getter.is_function() {
                                        match self.call_function_with_this(
                                            ctx,
                                            getter,
                                            obj_val,
                                            &[],
                                        ) {
                                            Ok(ret) => ret,
                                            Err(msg) => {
                                                self.set_pending_type_error(ctx, &msg);
                                                JSValue::undefined()
                                            }
                                        }
                                    } else {
                                        self.set_pending_type_error(
                                            ctx,
                                            "Property getter is not a function",
                                        );
                                        JSValue::undefined()
                                    };
                                    found = true;
                                    break;
                                }
                                current = proto.prototype;
                            }
                        }
                    }

                    if !found && obj_val.is_function() && js_obj.prototype.is_none() {
                        if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
                            let fn_proto = unsafe { &*fn_proto_ptr };
                            if let Some(offset) = fn_proto.find_offset(atom) {
                                value = fn_proto
                                    .get_by_offset(offset)
                                    .unwrap_or(JSValue::undefined());
                                found = true;
                            } else if let Some(getter) = fn_proto.get_own_accessor_value(atom) {
                                value = if getter.is_function() {
                                    match self.call_function_with_this(ctx, getter, obj_val, &[]) {
                                        Ok(ret) => ret,
                                        Err(msg) => {
                                            self.set_pending_type_error(ctx, &msg);
                                            JSValue::undefined()
                                        }
                                    }
                                } else {
                                    self.set_pending_type_error(
                                        ctx,
                                        "Property getter is not a function",
                                    );
                                    JSValue::undefined()
                                };
                                found = true;
                            }
                        }
                    }

                    if found {
                        self.set_reg(dst, value);
                    } else {
                        self.set_reg(dst, JSValue::undefined());
                    }
                }
            }

            None
        } else if obj_val.is_string() {
            if atom == ctx.common_atoms.length {
                Some(JSValue::new_int(
                    ctx.string_char_count(obj_val.get_atom()) as i64
                ))
            } else if let Some(proto_ptr) = ctx.get_string_prototype() {
                let proto_obj = unsafe { &*proto_ptr };
                Some(proto_obj.get(atom).unwrap_or(JSValue::undefined()))
            } else {
                Some(JSValue::undefined())
            }
        } else if obj_val.is_int() || obj_val.is_float() || obj_val.is_bool() {
            if let Some(proto_ptr) = ctx.get_number_prototype() {
                let proto_obj = unsafe { &*proto_ptr };
                Some(proto_obj.get(atom).unwrap_or(JSValue::undefined()))
            } else {
                Some(JSValue::undefined())
            }
        } else if obj_val.is_symbol() {
            let prop_str = ctx.get_atom_str(atom);
            if prop_str == "description" {
                let desc_atom = obj_val.get_atom();
                if desc_atom.0 == ctx.common_atoms.empty.0 {
                    Some(JSValue::undefined())
                } else {
                    Some(JSValue::new_string(desc_atom))
                }
            } else if let Some(proto_ptr) = ctx.get_symbol_prototype() {
                let proto = unsafe { &*proto_ptr };
                Some(proto.get(atom).unwrap_or(JSValue::undefined()))
            } else {
                Some(JSValue::undefined())
            }
        } else {
            Some(JSValue::undefined())
        }
    }

    #[inline(always)]
    fn get_named_prop_fast(
        &mut self,
        ctx: &mut JSContext,
        dst: u16,
        obj_reg: u16,
        atom: crate::runtime::atom::Atom,
        instr_pc: usize,
    ) -> bool {
        let obj_val = self.get_reg(obj_reg);

        if obj_val.is_object_like() {
            let js_obj = unsafe { crate::value::JSValue::object_from_ptr(obj_val.get_ptr()) };

            if js_obj.is_dense_array() && atom == ctx.common_atoms.length {
                if js_obj.props_len() > 0 {
                    self.set_reg(dst, js_obj.get_by_offset_fast(0));
                } else {
                    self.set_reg(dst, crate::value::JSValue::new_int(0));
                }
                return true;
            }
            let shape_id = js_obj.shape_id_cache;
            if shape_id != usize::MAX {
                let ic_table_ptr = self.cached_ic_table_ptr;
                if !ic_table_ptr.is_null() {
                    let (ic_hit, r0_offset, r0_proto) =
                        unsafe { (*ic_table_ptr).get_reads0_values(instr_pc, shape_id) };
                    if ic_hit {
                        if r0_proto == 0 {
                            let val = if r0_offset == u32::MAX {
                                crate::value::JSValue::undefined()
                            } else {
                                let off = r0_offset as usize;

                                if off < crate::object::object::INLINE_PROPS
                                    && js_obj.has_no_deleted_props()
                                {
                                    js_obj.get_by_offset_fast(off)
                                } else if let Some(v) = js_obj.get_by_offset(off) {
                                    v
                                } else {
                                    return self
                                        .get_named_prop_slow(ctx, dst, obj_val, atom, instr_pc);
                                }
                            };
                            self.set_reg(dst, val);
                            return true;
                        } else {
                            if let Some(proto_raw) = js_obj.prototype {
                                if proto_raw as usize == r0_proto {
                                    let proto_obj = unsafe { &*proto_raw };
                                    let off = r0_offset as usize;
                                    let v = if off < crate::object::object::INLINE_PROPS
                                        && proto_obj.has_no_deleted_props()
                                    {
                                        Some(proto_obj.get_by_offset_fast(off))
                                    } else {
                                        proto_obj.get_by_offset(off)
                                    };
                                    if let Some(v) = v {
                                        self.set_reg(dst, v);
                                        return true;
                                    }
                                    return self
                                        .get_named_prop_slow(ctx, dst, obj_val, atom, instr_pc);
                                }
                            }
                            return self.get_inherited_fast(
                                ctx, dst, obj_val, atom, instr_pc, r0_offset, r0_proto,
                            );
                        }
                    } else {
                        return self
                            .get_named_prop_poly_hit(ctx, dst, obj_val, atom, instr_pc, shape_id);
                    }
                }
            }
        }
        self.get_named_prop_slow(ctx, dst, obj_val, atom, instr_pc)
    }

    #[inline(never)]
    fn get_named_prop_poly_hit(
        &mut self,
        ctx: &mut JSContext,
        dst: u16,
        obj_val: JSValue,
        atom: crate::runtime::atom::Atom,
        instr_pc: usize,
        shape_id: usize,
    ) -> bool {
        let ic_table_ptr = self.cached_ic_table_ptr;
        if !ic_table_ptr.is_null() {
            let (ic_hit, r_offset, r_proto) =
                unsafe { (*ic_table_ptr).get_reads123_values(instr_pc, shape_id) };
            if ic_hit {
                let js_obj = unsafe { JSValue::object_from_ptr(obj_val.get_ptr()) };
                if r_proto == 0 {
                    let val = if r_offset == u32::MAX {
                        JSValue::undefined()
                    } else {
                        let off = r_offset as usize;
                        if off < crate::object::object::INLINE_PROPS
                            && js_obj.has_no_deleted_props()
                        {
                            js_obj.get_by_offset_fast(off)
                        } else if let Some(v) = js_obj.get_by_offset(off) {
                            v
                        } else {
                            return self.get_named_prop_slow(ctx, dst, obj_val, atom, instr_pc);
                        }
                    };
                    self.set_reg(dst, val);
                    return true;
                } else {
                    if let Some(p) = js_obj.prototype {
                        if p as usize == r_proto {
                            let proto_obj = unsafe { &*p };
                            let off = r_offset as usize;
                            let v = if off < crate::object::object::INLINE_PROPS
                                && proto_obj.has_no_deleted_props()
                            {
                                Some(proto_obj.get_by_offset_fast(off))
                            } else {
                                proto_obj.get_by_offset(off)
                            };
                            if let Some(v) = v {
                                self.set_reg(dst, v);
                                return true;
                            }
                        }
                    }

                    return self
                        .get_inherited_fast(ctx, dst, obj_val, atom, instr_pc, r_offset, r_proto);
                }
            }
        }
        self.get_named_prop_slow(ctx, dst, obj_val, atom, instr_pc)
    }

    #[inline(never)]
    fn get_inherited_fast(
        &mut self,
        ctx: &mut JSContext,
        dst: u16,
        obj_val: crate::value::JSValue,
        atom: crate::runtime::atom::Atom,
        instr_pc: usize,
        offset: u32,
        proto_ptr: usize,
    ) -> bool {
        let js_obj = unsafe { crate::value::JSValue::object_from_ptr(obj_val.get_ptr()) };
        let mut cur = js_obj.prototype;
        let mut depth = 0u32;
        while let Some(p) = cur {
            if depth > 1000 {
                break;
            }
            if p as usize == proto_ptr {
                let proto_obj = unsafe { &*p };
                if let Some(v) = proto_obj.get_by_offset(offset as usize) {
                    self.set_reg(dst, v);
                    return true;
                }

                break;
            }
            depth += 1;
            cur = unsafe { (*p).prototype };
        }

        self.get_named_prop_slow(ctx, dst, obj_val, atom, instr_pc)
    }

    #[inline(never)]
    fn get_named_prop_slow(
        &mut self,
        ctx: &mut JSContext,
        dst: u16,
        obj_val: JSValue,
        atom: crate::runtime::atom::Atom,
        instr_pc: usize,
    ) -> bool {
        if (obj_val.is_string() && atom != ctx.common_atoms.length)
            || obj_val.is_int()
            || obj_val.is_float()
        {
            let pseudo_id = if obj_val.is_string() {
                PRIM_STRING_SHAPE_ID
            } else {
                PRIM_NUMBER_SHAPE_ID
            };
            let ic_table_ptr = self.cached_ic_table_ptr;
            if !ic_table_ptr.is_null() {
                let (ic_hit, r0_offset, r0_proto) =
                    unsafe { (*ic_table_ptr).get_reads0_values(instr_pc, pseudo_id) };
                if ic_hit {
                    if r0_proto != 0 {
                        let proto_obj =
                            unsafe { &*(r0_proto as *const crate::object::object::JSObject) };
                        if let Some(v) = proto_obj.get_by_offset(r0_offset as usize) {
                            self.set_reg(dst, v);
                            return true;
                        }
                    } else if r0_offset == u32::MAX {
                        self.set_reg(dst, JSValue::undefined());
                        return true;
                    }
                }
            }
        }
        if let Some(result) = self.get_named_prop_result(ctx, dst, obj_val, atom, instr_pc) {
            self.set_reg(dst, result);
            return true;
        }
        false
    }

    #[inline(always)]
    fn set_named_prop_fast(
        &mut self,
        ctx: &mut JSContext,
        obj_reg: u16,
        val_reg: u16,
        atom: crate::runtime::atom::Atom,
        instr_pc: usize,
    ) {
        let obj_val = self.get_reg(obj_reg);
        let value = self.get_reg(val_reg);
        self.set_named_prop(ctx, obj_val, value, atom, instr_pc);
    }

    #[inline(always)]
    fn proto_chain_has_accessors(
        &self,
        obj_val: JSValue,
        atom: crate::runtime::atom::Atom,
    ) -> (bool, Option<JSValue>) {
        if obj_val.is_object_like() {
            let js_obj = unsafe { JSValue::object_from_ptr(obj_val.get_ptr()) };
            if let Some(entry) = js_obj.get_own_accessor_entry(atom) {
                return (true, entry.set);
            }
            let mut proto = js_obj.prototype;
            while let Some(proto_ptr) = proto {
                if proto_ptr.is_null() {
                    break;
                }
                let proto_obj = unsafe { &*proto_ptr };
                if let Some(entry) = proto_obj.get_own_accessor_entry(atom) {
                    return (true, entry.set);
                }
                proto = proto_obj.prototype;
            }
        }
        (false, None)
    }

    #[inline(always)]
    fn set_named_prop(
        &mut self,
        ctx: &mut JSContext,
        obj_val: JSValue,
        value: JSValue,
        atom: crate::runtime::atom::Atom,
        ic_pc: usize,
    ) {
        if obj_val.is_object_like() {
            let ptr = obj_val.get_ptr();
            let js_obj = unsafe { JSValue::object_from_ptr_mut(ptr) };
            let ic_table_ptr = self.cached_ic_table_ptr;

            if let Some(shape_id) = js_obj.get_shape_id() {
                if !ic_table_ptr.is_null() {
                    let ic_table = unsafe { &*ic_table_ptr };
                    if let Some(ic) = ic_table.get(ic_pc) {
                        if let Some((offset, new_shape_ptr)) = ic.get_transition(shape_id) {
                            if new_shape_ptr.is_null() {
                                js_obj.set_by_offset(offset as usize, value);
                            } else {
                                let new_shape = unsafe {
                                    std::ptr::NonNull::new_unchecked(
                                        new_shape_ptr as *mut crate::object::shape::Shape,
                                    )
                                };
                                js_obj.push_prop_with_shape(
                                    offset as usize,
                                    atom,
                                    value,
                                    new_shape,
                                );
                            }
                            return;
                        }
                    }
                }
            }

            let pre_offset = js_obj.find_offset(atom);

            if !js_obj.is_prop_writable_at(pre_offset) {
                let msg = format!(
                    "Cannot assign to read only property '{}'",
                    ctx.get_atom_str(atom)
                );
                self.set_pending_type_error(ctx, &msg);
                return;
            }

            let (has_accessor, setter) = self.proto_chain_has_accessors(obj_val, atom);
            if has_accessor {
                if let Some(setter_fn) = setter {
                    let _ = self.call_function_with_this(ctx, setter_fn, obj_val, &[value]);
                }
                return;
            }

            if pre_offset.is_none() && !js_obj.extensible() {
                let msg = format!(
                    "Cannot define property '{}', object is not extensible",
                    ctx.get_atom_str(atom)
                );
                self.set_pending_type_error(ctx, &msg);
                return;
            }

            self.set_named_prop_slow(ctx, obj_val, ptr, js_obj, value, atom, ic_pc);
        }
    }

    #[cold]
    fn set_named_prop_slow(
        &mut self,
        ctx: &mut JSContext,
        obj_val: JSValue,
        ptr: usize,
        js_obj: &mut crate::object::object::JSObject,
        value: JSValue,
        atom: crate::runtime::atom::Atom,
        ic_pc: usize,
    ) {
        let ic_table_ptr = self.cached_ic_table_ptr;

        let pre_shape_id = js_obj.get_shape_id();
        let pre_props_len = js_obj.props_len();
        let pre_offset = js_obj.find_offset(atom);
        js_obj.set_cached_with_offset(atom, value, ctx.shape_cache_mut(), pre_offset);
        if let Some(shape_id) = js_obj.get_shape_id() {
            let offset = pre_offset.or_else(|| js_obj.find_offset(atom));
            if let Some(offset) = offset {
                if !ic_table_ptr.is_null() && ic_pc != usize::MAX {
                    unsafe {
                        (*ic_table_ptr).ensure_capacity((ic_pc) + 1);
                        if let Some(ic) = (*ic_table_ptr).get_mut(ic_pc) {
                            let was_transition = offset == pre_props_len;
                            if was_transition {
                                if let Some(pre_id) = pre_shape_id {
                                    if let Some(new_shape_ptr) = js_obj.get_shape_ptr() {
                                        ic.insert_transition(pre_id, offset as u32, new_shape_ptr);
                                    } else {
                                        ic.insert(shape_id, offset as u32, None);
                                    }
                                } else {
                                    ic.insert(shape_id, offset as u32, None);
                                }
                            } else {
                                ic.insert_transition_null(shape_id, offset as u32);
                            }
                        }
                    }
                }
            }
        }

        if obj_val.is_function() && atom == ctx.common_atoms.prototype {
            let js_func = unsafe { JSValue::function_from_ptr_mut(ptr) };
            if value.is_object() {
                js_func.cached_prototype_ptr =
                    value.get_ptr() as *mut crate::object::object::JSObject;
            } else {
                js_func.cached_prototype_ptr = std::ptr::null_mut();
            }
        }

        if obj_val.is_function() && atom.0 & 0x40000000 != 0 {
            let js_func = unsafe { JSValue::function_from_ptr_mut(ptr) };
            js_func.mark_has_symbol_prop();
        }
    }

    fn int_atom(&self, idx: usize, ctx: &mut JSContext) -> crate::runtime::atom::Atom {
        ctx.int_atom_mut(idx)
    }

    #[cold]
    fn add_slow(&mut self, a: &JSValue, b: &JSValue, ctx: &mut JSContext) -> JSValue {
        if a.is_int() && b.is_float() {
            return JSValue::new_float_raw(a.get_int() as f64 + b.get_float());
        }
        if a.is_float() && b.is_int() {
            return JSValue::new_float_raw(a.get_float() + b.get_int() as f64);
        }

        let a = self.ordinary_to_primitive(a, "default", ctx);
        if self.pending_throw.is_some() {
            return JSValue::undefined();
        }
        let b = self.ordinary_to_primitive(b, "default", ctx);
        if self.pending_throw.is_some() {
            return JSValue::undefined();
        }

        if a.is_bigint() && b.is_bigint() {
            let a_int = Self::get_bigint_int(&a).unwrap_or(0);
            let b_int = Self::get_bigint_int(&b).unwrap_or(0);
            Self::create_bigint(a_int + b_int)
        } else if a.is_symbol() || b.is_symbol() {
            self.set_pending_type_error(ctx, "Cannot convert a Symbol value to a string");
            JSValue::undefined()
        } else if b.is_string() || a.is_string() {
            let a_str = if a.is_object() || a.is_function() {
                self.object_to_string(&a, ctx)
            } else {
                Self::js_to_string(&a, ctx)
            };
            let b_str = if b.is_object() || b.is_function() {
                self.object_to_string(&b, ctx)
            } else {
                Self::js_to_string(&b, ctx)
            };
            Self::js_add_string(&a_str, &b_str, ctx)
        } else if a.is_bigint() || b.is_bigint() {
            self.set_pending_type_error(ctx, "Cannot mix BigInt and other types");
            JSValue::undefined()
        } else if a.is_float() && b.is_float() {
            JSValue::new_float_raw(a.get_float() + b.get_float())
        } else if a.is_int() && b.is_float() {
            JSValue::new_float_raw(a.get_int() as f64 + b.get_float())
        } else if a.is_float() && b.is_int() {
            JSValue::new_float_raw(a.get_float() + b.get_int() as f64)
        } else {
            let fa = Self::js_to_number(&a, ctx);
            let fb = Self::js_to_number(&b, ctx);
            JSValue::new_float_raw(fa + fb)
        }
    }

    fn get_method_for_primitive(
        &self,
        obj: &crate::object::JSObject,
        method_atom: crate::runtime::atom::Atom,
        ctx: &JSContext,
    ) -> Option<JSValue> {
        if let Some(v) = obj.get(method_atom) {
            return Some(v);
        }

        if obj.obj_type() == crate::object::object::ObjectType::Function {
            if let Some(fn_proto_ptr) = ctx.get_function_prototype() {
                unsafe {
                    if let Some(v) = (*fn_proto_ptr).get(method_atom) {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    fn get_symbol_to_primitive_atom(
        &self,
        ctx: &mut JSContext,
    ) -> Option<crate::runtime::atom::Atom> {
        let global = ctx.global();
        if !global.is_object() {
            return None;
        }
        let sym_val = global.as_object().get(ctx.intern("Symbol.toPrimitive"))?;
        if !sym_val.is_symbol() {
            return None;
        }
        let sym_id = sym_val.get_symbol_id();
        Some(crate::runtime::atom::Atom(0x40000000 | sym_id))
    }

    fn call_function_safe(
        &mut self,
        ctx: &mut JSContext,
        func: JSValue,
        this_val: JSValue,
        args: &[JSValue],
    ) -> Result<JSValue, String> {
        let saved_handlers = std::mem::take(&mut self.exception_handlers);
        let result = self.call_function_with_this(ctx, func, this_val, args);
        self.exception_handlers = saved_handlers;
        if let Some(exc) = ctx.pending_exception.take() {
            self.pending_throw = Some(exc);
            return Err("exception propagated".to_string());
        }
        result
    }

    fn ordinary_to_primitive(&mut self, v: &JSValue, hint: &str, ctx: &mut JSContext) -> JSValue {
        if !v.is_object() && !v.is_function() {
            return *v;
        }
        let obj: &crate::object::JSObject = if v.is_object() {
            v.as_object()
        } else if v.is_function() {
            let func = v.as_function();

            if let Some(custom_fn) = func.base.get_own_value(ctx.common_atoms.value_of) {
                if custom_fn.is_function() {
                    if let Ok(r) = self.call_function_with_this(ctx, custom_fn, *v, &[]) {
                        if !r.is_object() {
                            return r;
                        }
                    }
                }
            }

            if let Some(custom_fn) = func.base.get_own_value(ctx.common_atoms.to_string) {
                if custom_fn.is_function() {
                    if let Ok(r) = self.call_function_with_this(ctx, custom_fn, *v, &[]) {
                        if !r.is_object() {
                            return r;
                        }
                    }
                }
            }
            let name = ctx.get_atom_str(func.name);
            let params: Vec<String> = (0..func.arity as usize)
                .map(|i| format!("a{}", i))
                .collect();
            let param_str = params.join(", ");
            let s = if name.is_empty() {
                format!("function({}) {{ [user code] }}", param_str)
            } else {
                format!("function {}({}) {{ [user code] }}", name, param_str)
            };
            return JSValue::new_string(ctx.intern(&s));
        } else {
            return *v;
        };

        let mut has_tp = false;

        let has_own_tp = self.get_symbol_to_primitive_atom(ctx).map_or(false, |a| {
            obj.get_own_value(a)
                .or_else(|| {
                    let mut cur = obj.prototype;
                    while let Some(p) = cur {
                        unsafe {
                            if let Some(v) = (*p).get_own_value(a) {
                                return Some(v);
                            }
                            cur = (*p).prototype;
                        }
                    }
                    None
                })
                .map_or(false, |v| v.is_function())
        });
        if has_own_tp {
            has_tp = true;
        } else if let Some(tp_atom) = self.get_symbol_to_primitive_atom(ctx) {
            let tp_val = {
                let saved_tp = std::mem::take(&mut self.exception_handlers);
                let result = obj
                    .get_own_descriptor(tp_atom)
                    .and_then(|desc| {
                        if let Some(getter) = desc.get {
                            self.call_function_with_this(ctx, getter, *v, &[]).ok()
                        } else {
                            desc.value
                        }
                    })
                    .or_else(|| {
                        let mut current = obj.prototype;
                        while let Some(proto_ptr) = current {
                            unsafe {
                                let proto = &*proto_ptr;
                                if let Some(desc) = proto.get_own_descriptor(tp_atom) {
                                    if let Some(getter) = desc.get {
                                        return self
                                            .call_function_with_this(ctx, getter, *v, &[])
                                            .ok();
                                    }
                                    return desc.value;
                                }
                                current = proto.prototype;
                            }
                        }
                        None
                    });
                self.exception_handlers = saved_tp;
                result
            };
            if self.pending_throw.is_some() {
                return JSValue::undefined();
            }
            has_tp = tp_val.map_or(false, |v| v.is_function());
        }
        if !has_tp {
            if let Some(prim) = obj.get(ctx.common_atoms.__value__) {
                if !prim.is_object() {
                    return prim;
                }
            }
        }

        if let Some(tp_atom) = self.get_symbol_to_primitive_atom(ctx) {
            let tp_method = (|| -> Option<JSValue> {
                let from_own = obj.get_own_descriptor(tp_atom).and_then(|desc| {
                    if let Some(getter) = desc.get {
                        let saved = std::mem::take(&mut self.exception_handlers);
                        let result = self.call_function_with_this(ctx, getter, *v, &[]).ok();
                        self.exception_handlers = saved;
                        result
                    } else {
                        desc.value
                    }
                });
                if from_own.is_some() {
                    return from_own;
                }

                let mut current = obj.prototype;
                while let Some(proto_ptr) = current {
                    unsafe {
                        let proto = &*proto_ptr;
                        if let Some(desc) = proto.get_own_descriptor(tp_atom) {
                            if let Some(getter) = desc.get {
                                let saved = std::mem::take(&mut self.exception_handlers);
                                let result =
                                    self.call_function_with_this(ctx, getter, *v, &[]).ok();
                                self.exception_handlers = saved;
                                return result;
                            }
                            return desc.value;
                        }
                        current = proto.prototype;
                    }
                }
                None
            })();
            if let Some(tp_fn) = tp_method {
                if tp_fn.is_function() {
                    let hint_atom = ctx.intern(hint);
                    let result =
                        self.call_function_safe(ctx, tp_fn, *v, &[JSValue::new_string(hint_atom)]);
                    match result {
                        Ok(r) if !r.is_object() => return r,
                        Ok(_) => {
                            self.set_pending_type_error(
                                ctx,
                                "Cannot convert object to primitive value",
                            );
                            return JSValue::undefined();
                        }
                        Err(_) => {
                            return JSValue::undefined();
                        }
                    }
                }
            }
        }

        let hint = if hint == "default" { "number" } else { hint };
        let to_string_atom = ctx.common_atoms.to_string;
        let value_of_atom = ctx.common_atoms.value_of;
        let (first_try, second_try) = if hint == "string" {
            (to_string_atom, value_of_atom)
        } else {
            (value_of_atom, to_string_atom)
        };
        for &method_atom in &[first_try, second_try] {
            if let Some(f) = self.get_method_for_primitive(obj, method_atom, ctx) {
                if f.is_function() {
                    let result = self.call_function_safe(ctx, f, *v, &[]);
                    match result {
                        Ok(r) if !r.is_object() => return r,
                        Ok(_) => {}
                        Err(_) => {
                            return JSValue::undefined();
                        }
                    }
                }
            }
        }

        if v.is_function() {
            let func = v.as_function();
            let name = ctx.get_atom_str(func.name);
            if !name.is_empty() {
                return JSValue::new_string(
                    ctx.intern(&format!("function {}() {{ [native code] }}", name)),
                );
            }
            return JSValue::new_string(ctx.intern(&format!("function() {{ [native code] }}")));
        }

        let name_atom = ctx.common_atoms.name;
        let message_atom = ctx.common_atoms.message;
        let mut err = crate::object::object::JSObject::new();
        err.set(name_atom, JSValue::new_string(ctx.intern("TypeError")));
        err.set(
            message_atom,
            JSValue::new_string(ctx.intern("Cannot convert object to primitive value")),
        );
        if let Some(proto) = ctx.get_type_error_prototype() {
            err.prototype = Some(proto);
        }
        let ptr = Box::into_raw(Box::new(err)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        self.pending_throw = Some(JSValue::new_object(ptr));
        JSValue::undefined()
    }

    fn js_to_string(v: &JSValue, ctx: &mut JSContext) -> JSValue {
        if v.is_string() {
            return *v;
        }
        if v.is_int() {
            return JSValue::new_string(ctx.intern(&v.get_int().to_string()));
        }
        if v.is_float() {
            return JSValue::new_string(ctx.intern(&v.get_float().to_string()));
        }
        if v.is_bool() {
            return JSValue::new_string(ctx.intern(&v.get_bool().to_string()));
        }
        if v.is_null() {
            return JSValue::new_string(ctx.common_atoms.null);
        }
        if v.is_undefined() {
            return JSValue::new_string(ctx.common_atoms.undefined);
        }
        if v.is_bigint() {
            let val = VM::get_bigint_int(v).unwrap_or(0);
            return JSValue::new_string(ctx.intern(&val.to_string()));
        }
        if v.is_symbol() {
            return JSValue::new_string(ctx.intern("Symbol()"));
        }
        JSValue::new_string(ctx.intern(""))
    }

    fn object_to_string(&mut self, v: &JSValue, ctx: &mut JSContext) -> JSValue {
        if !v.is_object() && !v.is_function() {
            return Self::js_to_string(v, ctx);
        }
        let obj: &crate::object::JSObject = if v.is_object() {
            v.as_object()
        } else if v.is_function() {
            &v.as_function().base
        } else {
            return Self::js_to_string(v, ctx);
        };
        let to_str_atom = ctx.common_atoms.to_string;

        if v.is_function() {
            let fn_builtin = ctx.get_builtin_func("function_toString");
            if let Some(f) = fn_builtin {
                let result = f(ctx, &[*v]);
                if result.is_string() {
                    return result;
                }
            }

            let func = v.as_function();
            let name = ctx.get_atom_str(func.name);
            if name.is_empty() {
                return JSValue::new_string(ctx.intern("function () { [native code] }"));
            } else {
                return JSValue::new_string(
                    ctx.intern(&format!("function {}() {{ [native code] }}", name)),
                );
            }
        }
        if let Some(to_str_fn) = obj.get(to_str_atom) {
            if to_str_fn.is_function() {
                match self.call_function_with_this(ctx, to_str_fn, *v, &[]) {
                    Ok(r) if r.is_string() => return r,
                    Ok(r) => return Self::js_to_string(&r, ctx),
                    Err(_) => {}
                }
            }
        }

        let obj_proto = if let Some(p) = ctx.get_object_prototype() {
            p
        } else {
            return Self::js_to_string(v, ctx);
        };
        let to_str = unsafe { (*obj_proto).get(to_str_atom) };
        if let Some(f) = to_str {
            if f.is_function() {
                match self.call_function_with_this(ctx, f, *v, &[]) {
                    Ok(r) => return r,
                    Err(_) => {}
                }
            }
        }
        Self::js_to_string(v, ctx)
    }

    fn js_to_number(v: &JSValue, ctx: &mut JSContext) -> f64 {
        if v.is_int() {
            return v.get_int() as f64;
        } else if v.is_float() {
            return v.get_float();
        } else if v.is_bool() {
            return if v.get_bool() { 1.0 } else { 0.0 };
        } else if v.is_null() {
            return 0.0;
        } else if v.is_undefined() {
            return f64::NAN;
        } else if v.is_string() {
            let s = ctx.get_atom_str(v.get_atom());
            return s.trim().parse::<f64>().unwrap_or(f64::NAN);
        } else if v.is_bigint() {
            let val = VM::get_bigint_int(v).unwrap_or(0);
            return val as f64;
        } else if v.is_symbol() {
            return f64::NAN;
        } else if v.is_object() {
            if let Some(prim) = v.as_object().get(ctx.common_atoms.__value__) {
                return Self::js_to_number(&prim, ctx);
            }
        }
        f64::NAN
    }

    fn to_int32(na: f64) -> i32 {
        if na.is_nan() || na.is_infinite() || na == 0.0 {
            return 0;
        }
        let pos = na.abs().floor() as u64;
        let u32_val = if na > 0.0 {
            pos % 0x1_0000_0000
        } else {
            let rem = pos % 0x1_0000_0000;
            if rem == 0 { 0 } else { 0x1_0000_0000 - rem }
        };
        u32_val as i32
    }

    fn get_bigint_int(v: &JSValue) -> Option<i128> {
        if v.is_bigint() {
            Some(v.as_object().get_bigint_value())
        } else {
            None
        }
    }

    fn create_bigint(value: i128) -> JSValue {
        let mut bigint_obj = crate::object::object::JSObject::new_bigint();
        bigint_obj.set_bigint_value(value);
        let ptr = Box::into_raw(Box::new(bigint_obj)) as usize;
        JSValue::new_bigint(ptr)
    }

    fn js_add_string(b: &JSValue, a: &JSValue, ctx: &mut JSContext) -> JSValue {
        if b.is_string() && a.is_string() {
            let atom = ctx.intern_concat_atoms(b.get_atom(), a.get_atom());
            return JSValue::new_string(atom);
        }

        fn stringify(v: &JSValue, ctx: &JSContext) -> (Option<[u8; 24]>, usize, Option<String>) {
            if v.is_string() {
                let s = ctx.get_atom_str(v.get_atom());
                let bytes = s.as_bytes();
                let len = bytes.len();
                if len <= 24 {
                    let mut buf = [0u8; 24];
                    buf[..len].copy_from_slice(bytes);
                    return (Some(buf), len, None);
                } else {
                    return (None, len, Some(s.to_string()));
                }
            } else if v.is_int() {
                let n = v.get_int();
                let mut buf = [0u8; 24];
                let mut tmp = n;
                let mut len = 0;
                let negative = tmp < 0;
                if negative {
                    tmp = -tmp;
                }
                if tmp == 0 {
                    buf[0] = b'0';
                    len = 1;
                } else {
                    while tmp > 0 {
                        buf[len] = (tmp % 10) as u8 + b'0';
                        len += 1;
                        tmp /= 10;
                    }
                }
                if negative {
                    buf[len] = b'-';
                    len += 1;
                }

                for i in 0..len / 2 {
                    buf.swap(i, len - 1 - i);
                }
                return (Some(buf), len, None);
            } else if v.is_float() {
                let f = v.get_float();
                let s = if f.fract() == 0.0 && f.abs() < 1e15 {
                    format!("{}", f as i64)
                } else {
                    format!("{}", f)
                };
                let bytes = s.as_bytes();
                let len = bytes.len();
                if len <= 24 {
                    let mut buf = [0u8; 24];
                    buf[..len].copy_from_slice(bytes);
                    return (Some(buf), len, None);
                } else {
                    return (None, len, Some(s));
                }
            } else if v.is_bool() {
                if v.get_bool() {
                    let mut buf = [0u8; 24];
                    buf[..4].copy_from_slice(b"true");
                    return (Some(buf), 4, None);
                } else {
                    let mut buf = [0u8; 24];
                    buf[..5].copy_from_slice(b"false");
                    return (Some(buf), 5, None);
                }
            } else if v.is_null() {
                let mut buf = [0u8; 24];
                buf[..4].copy_from_slice(b"null");
                return (Some(buf), 4, None);
            } else if v.is_object() || v.is_function() {
                let mut buf = [0u8; 24];
                let s = b"[object Object]";
                buf[..s.len()].copy_from_slice(s);
                return (Some(buf), s.len(), None);
            } else if v.is_bigint() {
                let val = crate::runtime::vm::VM::get_bigint_int(v);
                let n = val.unwrap_or(0);
                let mut buf = [0u8; 24];
                let s = n.to_string();
                let bytes = s.as_bytes();
                let len = bytes.len();
                if len <= 24 {
                    buf[..len].copy_from_slice(bytes);
                    (Some(buf), len, None)
                } else {
                    (None, len, Some(s))
                }
            } else {
                let mut buf = [0u8; 24];
                let s = b"undefined";
                buf[..s.len()].copy_from_slice(s);
                return (Some(buf), s.len(), None);
            }
        }

        let (buf_b, len_b, str_b) = stringify(b, ctx);
        let (buf_a, len_a, str_a) = stringify(a, ctx);

        let total = len_b + len_a;
        if total <= 128 {
            let mut combined = [0u8; 128];
            if let Some(buf) = buf_b {
                combined[..len_b].copy_from_slice(&buf[..len_b]);
            } else if let Some(ref s) = str_b {
                combined[..len_b].copy_from_slice(s.as_bytes());
            }
            if let Some(buf) = buf_a {
                combined[len_b..total].copy_from_slice(&buf[..len_a]);
            } else if let Some(ref s) = str_a {
                combined[len_b..total].copy_from_slice(s.as_bytes());
            }
            let atom = ctx.intern(unsafe { std::str::from_utf8_unchecked(&combined[..total]) });
            JSValue::new_string(atom)
        } else {
            let mut combined = String::with_capacity(total);
            if let Some(buf) = buf_b {
                combined.push_str(unsafe { std::str::from_utf8_unchecked(&buf[..len_b]) });
            } else if let Some(ref s) = str_b {
                combined.push_str(s);
            }
            if let Some(buf) = buf_a {
                combined.push_str(unsafe { std::str::from_utf8_unchecked(&buf[..len_a]) });
            } else if let Some(ref s) = str_a {
                combined.push_str(s);
            }
            let atom = ctx.intern(&combined);
            JSValue::new_string(atom)
        }
    }
}

#[inline(always)]
fn loose_equal(ctx: &JSContext, a: JSValue, b: JSValue) -> bool {
    if a.raw_bits() == b.raw_bits() {
        return if a.is_float() {
            !a.get_float().is_nan()
        } else {
            true
        };
    }

    if JSValue::both_int(&a, &b) {
        return false;
    }

    if JSValue::both_object(&a, &b) {
        return false;
    }

    if (a.is_object() || a.is_function()) && b.is_null_or_undefined() {
        return false;
    }
    if (b.is_object() || b.is_function()) && a.is_null_or_undefined() {
        return false;
    }

    if a.is_null_or_undefined() {
        return b.is_null_or_undefined();
    }
    if b.is_null() || b.is_undefined() {
        return false;
    }

    loose_equal_slow(ctx, a, b)
}

#[cold]
fn loose_equal_slow(ctx: &JSContext, a: JSValue, b: JSValue) -> bool {
    if a.is_undefined() && b.is_undefined() {
        return true;
    }
    if a.is_null() && b.is_null() {
        return true;
    }
    if a.is_bool() && b.is_bool() {
        return a.get_bool() == b.get_bool();
    }
    if a.is_string() && b.is_string() {
        return a.get_atom().0 == b.get_atom().0;
    }
    if a.is_bigint() && b.is_bigint() {
        return a.strict_eq(&b);
    }
    if a.is_null() && b.is_undefined() || a.is_undefined() && b.is_null() {
        return true;
    }

    if a.is_object() || a.is_function() {
        if (a.is_object() && b.is_object()) || (a.is_function() && b.is_function()) {
            return a.strict_eq(&b);
        }

        return false;
    }
    if b.is_object() || b.is_function() {
        return false;
    }

    if a.is_float() && b.is_float() {
        return a.get_float() == b.get_float();
    }
    if a.is_int() && b.is_float() {
        return (a.get_int() as f64) == b.get_float();
    }
    if a.is_float() && b.is_int() {
        return a.get_float() == (b.get_int() as f64);
    }

    if a.is_bool() && (b.is_int() || b.is_float()) {
        return loose_equal(ctx, JSValue::new_int(if a.get_bool() { 1 } else { 0 }), b);
    }
    if (a.is_int() || a.is_float()) && b.is_bool() {
        return loose_equal(ctx, a, JSValue::new_int(if b.get_bool() { 1 } else { 0 }));
    }

    if a.is_bool() && b.is_string() {
        let n = JSValue::new_int(if a.get_bool() { 1 } else { 0 });
        return loose_equal(ctx, n, b);
    }
    if a.is_string() && b.is_bool() {
        return loose_equal(ctx, b, a);
    }

    if a.is_string() && (b.is_int() || b.is_float()) {
        let s = ctx.get_atom_str(a.get_atom());
        if let Ok(n) = s.parse::<f64>() {
            if n.is_nan() {
                return false;
            }
            if b.is_int() {
                return n == (b.get_int() as f64);
            }
            return n == b.get_float();
        }
        return false;
    }
    if (a.is_int() || a.is_float()) && b.is_string() {
        return loose_equal(ctx, b, a);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bytecode(code: Vec<u8>, constants: Vec<JSValue>, locals_count: u32) -> Bytecode {
        Bytecode {
            code,
            constants,
            locals_count,
            param_count: 0,
            line_number_table: None,
            ic_table: crate::compiler::InlineCacheTable::new(),
            shared_ic_table_ptr: std::ptr::null_mut(),
            uses_arguments: false,
            is_strict: false,
            var_name_to_slot: std::rc::Rc::new(Vec::new()),
            nested_bytecodes: std::collections::HashMap::new(),
            is_simple_constructor: false,
            simple_constructor_props: Vec::new(),
            cached_constructor_final_shape: None,
            cached_constructor_atoms: Vec::new(),
            shared_code_ptr: std::ptr::null(),
            shared_code_len: 0,
            shared_const_ptr: std::ptr::null(),
            shared_const_len: 0,
        }
    }

    fn emit_u16(code: &mut Vec<u8>, v: u16) {
        code.extend_from_slice(&v.to_le_bytes());
    }

    fn emit_u32(code: &mut Vec<u8>, v: u32) {
        code.extend_from_slice(&v.to_le_bytes());
    }

    fn emit_i32(code: &mut Vec<u8>, v: i32) {
        code.extend_from_slice(&v.to_le_bytes());
    }

    fn run_bytecode(code: Vec<u8>, constants: Vec<JSValue>, locals_count: u32) -> JSValue {
        let bc = make_bytecode(code, constants, locals_count);
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);
        let mut vm = VM::new();
        match vm.execute(&mut ctx, &bc).unwrap() {
            ExecutionOutcome::Complete(v) => v,
            ExecutionOutcome::Yield(v) => v,
        }
    }

    #[test]
    fn test_vm_new_encoding_add() {
        let mut code = Vec::new();

        code.push(Opcode::LoadInt as u8);
        emit_u16(&mut code, 1);
        emit_i32(&mut code, 40);

        code.push(Opcode::LoadInt as u8);
        emit_u16(&mut code, 2);
        emit_i32(&mut code, 2);

        code.push(Opcode::Add as u8);
        emit_u16(&mut code, 3);
        emit_u16(&mut code, 1);
        emit_u16(&mut code, 2);

        code.push(Opcode::Move as u8);
        emit_u16(&mut code, 0);
        emit_u16(&mut code, 3);

        code.push(Opcode::End as u8);

        let result = run_bytecode(code, vec![], 4);
        assert_eq!(result.get_int(), 42);
    }

    #[test]
    fn test_vm_new_encoding_set_get_prop() {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);
        let atom_x = ctx.atom_table_mut().intern("x");

        let mut code = Vec::new();

        code.push(Opcode::NewObject as u8);
        emit_u16(&mut code, 1);

        code.push(Opcode::LoadConst as u8);
        emit_u16(&mut code, 2);
        emit_u32(&mut code, 0);

        code.push(Opcode::LoadInt as u8);
        emit_u16(&mut code, 3);
        emit_i32(&mut code, 42);

        code.push(Opcode::SetProp as u8);
        emit_u16(&mut code, 1);
        emit_u16(&mut code, 2);
        emit_u16(&mut code, 3);

        code.push(Opcode::GetProp as u8);
        emit_u16(&mut code, 4);
        emit_u16(&mut code, 1);
        emit_u16(&mut code, 2);

        code.push(Opcode::Move as u8);
        emit_u16(&mut code, 0);
        emit_u16(&mut code, 4);

        code.push(Opcode::End as u8);

        let bc = make_bytecode(code, vec![JSValue::new_string(atom_x)], 5);
        let mut vm = VM::new();
        let result = match vm.execute(&mut ctx, &bc).unwrap() {
            ExecutionOutcome::Complete(v) => v,
            ExecutionOutcome::Yield(v) => v,
        };
        assert_eq!(result.get_int(), 42);
    }

    fn builtin_const_99(_ctx: &mut JSContext, _args: &[JSValue]) -> JSValue {
        JSValue::new_int(99)
    }

    fn builtin_inc(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
        let v = args.first().copied().unwrap_or_else(JSValue::undefined);
        if v.is_int() {
            JSValue::new_int(v.get_int() + 1)
        } else {
            JSValue::new_int(1)
        }
    }

    #[test]
    fn test_vm_call0_new_encoding() {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);

        let mut func = crate::object::function::JSFunction::new_builtin(ctx.intern("f0"), 0);
        func.builtin_func = Some(builtin_const_99);
        let func_ptr = Box::into_raw(Box::new(func)) as usize;
        ctx.runtime_mut().gc_heap_mut().track_function(func_ptr);

        let mut code = Vec::new();
        code.push(Opcode::LoadConst as u8);
        emit_u16(&mut code, 1);
        emit_u32(&mut code, 0);

        code.push(Opcode::Call0 as u8);
        emit_u16(&mut code, 2);
        emit_u16(&mut code, 1);

        code.push(Opcode::Move as u8);
        emit_u16(&mut code, 0);
        emit_u16(&mut code, 2);

        code.push(Opcode::End as u8);

        let bc = make_bytecode(code, vec![JSValue::new_function(func_ptr)], 3);
        let mut vm = VM::new();
        let result = match vm.execute(&mut ctx, &bc).unwrap() {
            ExecutionOutcome::Complete(v) => v,
            ExecutionOutcome::Yield(v) => v,
        };
        assert_eq!(result.get_int(), 99);
    }

    #[test]
    fn test_vm_call1_new_encoding() {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);

        let mut func = crate::object::function::JSFunction::new_builtin(ctx.intern("f1"), 1);
        func.builtin_func = Some(builtin_inc);
        let func_ptr = Box::into_raw(Box::new(func)) as usize;

        let mut code = Vec::new();
        code.push(Opcode::LoadConst as u8);
        emit_u16(&mut code, 1);
        emit_u32(&mut code, 0);

        code.push(Opcode::LoadInt as u8);
        emit_u16(&mut code, 2);
        emit_i32(&mut code, 41);

        code.push(Opcode::Call1 as u8);
        emit_u16(&mut code, 3);
        emit_u16(&mut code, 1);
        emit_u16(&mut code, 2);

        code.push(Opcode::Move as u8);
        emit_u16(&mut code, 0);
        emit_u16(&mut code, 3);

        code.push(Opcode::End as u8);

        let bc = make_bytecode(code, vec![JSValue::new_function(func_ptr)], 4);
        let mut vm = VM::new();
        let result = match vm.execute(&mut ctx, &bc).unwrap() {
            ExecutionOutcome::Complete(v) => v,
            ExecutionOutcome::Yield(v) => v,
        };
        assert_eq!(result.get_int(), 42);
    }

    #[test]
    fn test_vm_call2_new_encoding() {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);

        fn builtin_add2(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
            let a = args.first().copied().unwrap_or_else(JSValue::undefined);
            let b = args.get(1).copied().unwrap_or_else(JSValue::undefined);
            JSValue::new_int(a.get_int() + b.get_int())
        }

        let mut func = crate::object::function::JSFunction::new_builtin(ctx.intern("f2"), 2);
        func.builtin_func = Some(builtin_add2);
        let func_ptr = Box::into_raw(Box::new(func)) as usize;

        let mut code = Vec::new();
        code.push(Opcode::LoadConst as u8);
        emit_u16(&mut code, 1);
        emit_u32(&mut code, 0);

        code.push(Opcode::LoadInt as u8);
        emit_u16(&mut code, 2);
        emit_i32(&mut code, 40);

        code.push(Opcode::LoadInt as u8);
        emit_u16(&mut code, 3);
        emit_i32(&mut code, 2);

        code.push(Opcode::Call2 as u8);
        emit_u16(&mut code, 4);
        emit_u16(&mut code, 1);
        emit_u16(&mut code, 2);
        emit_u16(&mut code, 3);

        code.push(Opcode::Move as u8);
        emit_u16(&mut code, 0);
        emit_u16(&mut code, 4);

        code.push(Opcode::End as u8);

        let bc = make_bytecode(code, vec![JSValue::new_function(func_ptr)], 5);
        let mut vm = VM::new();
        let result = match vm.execute(&mut ctx, &bc).unwrap() {
            ExecutionOutcome::Complete(v) => v,
            ExecutionOutcome::Yield(v) => v,
        };
        assert_eq!(result.get_int(), 42);
    }

    #[test]
    fn test_vm_call3_new_encoding() {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);

        fn builtin_add3(_ctx: &mut JSContext, args: &[JSValue]) -> JSValue {
            let a = args.first().copied().unwrap_or_else(JSValue::undefined);
            let b = args.get(1).copied().unwrap_or_else(JSValue::undefined);
            let c = args.get(2).copied().unwrap_or_else(JSValue::undefined);
            JSValue::new_int(a.get_int() + b.get_int() + c.get_int())
        }

        let mut func = crate::object::function::JSFunction::new_builtin(ctx.intern("f3"), 3);
        func.builtin_func = Some(builtin_add3);
        let func_ptr = Box::into_raw(Box::new(func)) as usize;

        let mut code = Vec::new();
        code.push(Opcode::LoadConst as u8);
        emit_u16(&mut code, 1);
        emit_u32(&mut code, 0);

        code.push(Opcode::LoadInt as u8);
        emit_u16(&mut code, 2);
        emit_i32(&mut code, 39);

        code.push(Opcode::LoadInt as u8);
        emit_u16(&mut code, 3);
        emit_i32(&mut code, 2);

        code.push(Opcode::LoadInt as u8);
        emit_u16(&mut code, 4);
        emit_i32(&mut code, 1);

        code.push(Opcode::Call3 as u8);
        emit_u16(&mut code, 5);
        emit_u16(&mut code, 1);
        emit_u16(&mut code, 2);
        emit_u16(&mut code, 3);
        emit_u16(&mut code, 4);

        code.push(Opcode::Move as u8);
        emit_u16(&mut code, 0);
        emit_u16(&mut code, 5);

        code.push(Opcode::End as u8);

        let bc = make_bytecode(code, vec![JSValue::new_function(func_ptr)], 6);
        let mut vm = VM::new();
        let result = match vm.execute(&mut ctx, &bc).unwrap() {
            ExecutionOutcome::Complete(v) => v,
            ExecutionOutcome::Yield(v) => v,
        };
        assert_eq!(result.get_int(), 42);
    }

    #[test]
    fn test_vm_inc_local_new_encoding() {
        let mut code = Vec::new();

        code.push(Opcode::LoadInt as u8);
        emit_u16(&mut code, 1);
        emit_i32(&mut code, 41);

        code.push(Opcode::IncLocal as u8);
        emit_u16(&mut code, 1);

        code.push(Opcode::Move as u8);
        emit_u16(&mut code, 0);
        emit_u16(&mut code, 1);

        code.push(Opcode::End as u8);

        let result = run_bytecode(code, vec![], 2);
        assert_eq!(result.get_int(), 42);
    }

    #[test]
    fn test_vm_dec_local_new_encoding() {
        let mut code = Vec::new();

        code.push(Opcode::LoadInt as u8);
        emit_u16(&mut code, 1);
        emit_i32(&mut code, 41);

        code.push(Opcode::DecLocal as u8);
        emit_u16(&mut code, 1);

        code.push(Opcode::Move as u8);
        emit_u16(&mut code, 0);
        emit_u16(&mut code, 1);

        code.push(Opcode::End as u8);

        let result = run_bytecode(code, vec![], 2);
        assert_eq!(result.get_int(), 40);
    }
}
