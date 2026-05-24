use crate::compiler::ast::*;
use crate::compiler::location::LineNumberTable;
use crate::compiler::opcode::{Bytecode, Opcode};
use crate::runtime::context::JSContext;
use crate::util::FxHashMap;
use crate::value::JSValue;

#[derive(Debug)]
struct BreakableFrame {
    label: Option<String>,
    break_patches: Vec<usize>,
    continue_patches: Vec<usize>,
    continue_target: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
enum VarLocation {
    Local(u16),
    Upvalue(u16),
}

#[derive(Debug, Clone, Copy)]
struct ParentVar {
    local_idx: u16,
    is_inherited: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    O0,
    O1,
    O2,
    O3,
}

impl Default for OptLevel {
    fn default() -> Self {
        Self::O2
    }
}

impl OptLevel {
    pub fn from_flag(flag: &str) -> Option<Self> {
        match flag {
            "-O0" => Some(Self::O0),
            "-O1" => Some(Self::O1),
            "-O2" => Some(Self::O2),
            "-O3" => Some(Self::O3),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum P2FoldProfile {
    Raw,
    Ternary,
    Logical,
    Nullish,
    Stable,
    Aggressive,
}

impl P2FoldProfile {
    fn from_env_override() -> Option<Self> {
        let mode = std::env::var("PIPA_P2_PROFILE").ok()?;
        match mode.as_str() {
            "raw" => Some(Self::Raw),
            "ternary" => Some(Self::Ternary),
            "logical" => Some(Self::Logical),
            "nullish" => Some(Self::Nullish),
            "stable" => Some(Self::Stable),
            "aggressive" => Some(Self::Aggressive),
            _ => None,
        }
    }

    fn allow_ternary_bool(self) -> bool {
        matches!(
            self,
            Self::Ternary | Self::Logical | Self::Nullish | Self::Stable | Self::Aggressive
        )
    }

    fn allow_logical_bool_left(self) -> bool {
        matches!(
            self,
            Self::Logical | Self::Nullish | Self::Stable | Self::Aggressive
        )
    }

    fn allow_nullish_literal_left(self) -> bool {
        matches!(self, Self::Nullish | Self::Stable | Self::Aggressive)
    }

    fn allow_if_bool(self) -> bool {
        matches!(self, Self::Stable | Self::Aggressive)
    }

    fn allow_aggressive_truthy(self) -> bool {
        matches!(self, Self::Aggressive)
    }
}

#[derive(Debug, Clone, Copy)]
struct CodegenSettings {
    p2_fold_profile: P2FoldProfile,
    opt_jump_if_nullish: bool,
    opt_call23: bool,
    opt_fused_cmp_jump: bool,
    opt_fused_getprop_call: bool,
    opt_tiny_inline: bool,
    opt_branch_result_prealloc: bool,
}

impl CodegenSettings {
    fn for_opt_level(opt_level: OptLevel) -> Self {
        match opt_level {
            OptLevel::O0 => Self {
                p2_fold_profile: P2FoldProfile::Raw,
                opt_jump_if_nullish: false,
                opt_call23: false,
                opt_fused_cmp_jump: false,
                opt_fused_getprop_call: false,
                opt_tiny_inline: false,
                opt_branch_result_prealloc: false,
            },
            OptLevel::O1 => Self {
                p2_fold_profile: P2FoldProfile::Ternary,
                opt_jump_if_nullish: true,
                opt_call23: true,
                opt_fused_cmp_jump: false,
                opt_fused_getprop_call: true,
                opt_tiny_inline: false,
                opt_branch_result_prealloc: true,
            },
            OptLevel::O2 => Self {
                p2_fold_profile: P2FoldProfile::Stable,
                opt_jump_if_nullish: true,
                opt_call23: true,
                opt_fused_cmp_jump: false,
                opt_fused_getprop_call: true,
                opt_tiny_inline: true,
                opt_branch_result_prealloc: true,
            },
            OptLevel::O3 => Self {
                p2_fold_profile: P2FoldProfile::Aggressive,
                opt_jump_if_nullish: true,
                opt_call23: true,
                opt_fused_cmp_jump: false,
                opt_fused_getprop_call: true,
                opt_tiny_inline: true,
                opt_branch_result_prealloc: true,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VarEntry {
    pub(crate) slot: u16,
    pub(crate) kind: VariableKind,
    pub(crate) initialized: bool,
}

pub struct CodeGenerator {
    code: Vec<u8>,
    constants: Vec<JSValue>,
    next_register: u16,
    max_register: u16,
    free_list: Vec<u16>,
    scope_stack: Vec<FxHashMap<String, VarEntry>>,
    reserved_slots: std::collections::HashSet<u16>,
    breakable_frames: Vec<BreakableFrame>,

    parent_vars: FxHashMap<String, ParentVar>,
    upvalues: Vec<(u32, u16)>,
    upvalue_slots: FxHashMap<String, u16>,
    line_table: LineNumberTable,
    depth: u32,
    is_script_level: bool,
    scope_is_global: Vec<bool>,
    function_uses_arguments: bool,
    current_function_name: Option<String>,
    tiny_inline_functions: FxHashMap<String, FunctionDeclaration>,
    p2_fold_profile: P2FoldProfile,
    opt_jump_if_nullish: bool,
    opt_call23: bool,
    opt_fused_cmp_jump: bool,
    opt_fused_getprop_call: bool,
    opt_tiny_inline: bool,
    opt_branch_result_prealloc: bool,
    suppress_ref_error: bool,
    pub(crate) is_strict: bool,
    pub(crate) is_eval: bool,
    is_arrow: bool,
    in_static_method: bool,
    pending_label: Option<String>,
    all_var_names: Vec<(String, u16)>,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self::with_opt_level(OptLevel::default())
    }

    pub fn with_opt_level(opt_level: OptLevel) -> Self {
        let mut settings = CodegenSettings::for_opt_level(opt_level);
        if let Some(profile) = P2FoldProfile::from_env_override() {
            settings.p2_fold_profile = profile;
        }
        settings.opt_jump_if_nullish =
            Self::env_flag("PIPA_OPT_JUMP_NULLISH", settings.opt_jump_if_nullish);
        settings.opt_call23 = Self::env_flag("PIPA_OPT_CALL23", settings.opt_call23);
        settings.opt_fused_cmp_jump =
            Self::env_flag("PIPA_OPT_FUSED_CMP_JUMP", settings.opt_fused_cmp_jump);
        settings.opt_fused_getprop_call = Self::env_flag(
            "PIPA_OPT_FUSED_GETPROP_CALL",
            settings.opt_fused_getprop_call,
        );
        settings.opt_tiny_inline = Self::env_flag("PIPA_OPT_TINY_INLINE", settings.opt_tiny_inline);
        settings.opt_branch_result_prealloc = Self::env_flag(
            "PIPA_OPT_BRANCH_RESULT_PREALLOC",
            settings.opt_branch_result_prealloc,
        );
        Self::with_settings(settings)
    }

    fn with_settings(settings: CodegenSettings) -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            next_register: 0,
            max_register: 0,
            free_list: Vec::new(),
            scope_stack: vec![FxHashMap::default()],
            reserved_slots: std::collections::HashSet::new(),
            breakable_frames: Vec::new(),
            parent_vars: FxHashMap::default(),
            upvalues: Vec::new(),
            upvalue_slots: FxHashMap::default(),
            line_table: LineNumberTable::new(),
            depth: 0,
            is_script_level: true,
            scope_is_global: vec![false],
            function_uses_arguments: false,
            current_function_name: None,
            tiny_inline_functions: FxHashMap::default(),
            p2_fold_profile: settings.p2_fold_profile,
            opt_jump_if_nullish: settings.opt_jump_if_nullish,
            opt_call23: settings.opt_call23,
            opt_fused_cmp_jump: settings.opt_fused_cmp_jump,
            opt_fused_getprop_call: settings.opt_fused_getprop_call,
            opt_tiny_inline: settings.opt_tiny_inline,
            opt_branch_result_prealloc: settings.opt_branch_result_prealloc,
            suppress_ref_error: false,
            is_strict: false,
            is_eval: false,
            is_arrow: false,
            in_static_method: false,
            pending_label: None,
            all_var_names: Vec::new(),
        }
    }

    fn settings(&self) -> CodegenSettings {
        CodegenSettings {
            p2_fold_profile: self.p2_fold_profile,
            opt_jump_if_nullish: self.opt_jump_if_nullish,
            opt_call23: self.opt_call23,
            opt_fused_cmp_jump: self.opt_fused_cmp_jump,
            opt_fused_getprop_call: self.opt_fused_getprop_call,
            opt_tiny_inline: self.opt_tiny_inline,
            opt_branch_result_prealloc: self.opt_branch_result_prealloc,
        }
    }

    fn nested_codegen(&self) -> Self {
        let mut nested = Self::with_settings(self.settings());
        nested.tiny_inline_functions = self.tiny_inline_functions.clone();
        nested.is_strict = self.is_strict;
        nested.is_eval = self.is_eval;
        nested.in_static_method = self.in_static_method;
        nested
    }

    fn env_flag(name: &str, default: bool) -> bool {
        match std::env::var(name) {
            Ok(v) => match v.to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => true,
                "0" | "false" | "no" | "off" => false,
                _ => default,
            },
            Err(_) => default,
        }
    }

    fn literal_truthiness(lit: &Literal) -> Option<bool> {
        match lit {
            Literal::Boolean(b) => Some(*b),
            Literal::Null | Literal::Undefined => Some(false),
            Literal::Number(n) => Some(*n != 0.0 && !n.is_nan()),
            Literal::LegacyOctal(n) => Some(*n != 0),
            Literal::String(s, _) => Some(!s.is_empty()),
            Literal::BigInt(_) => None,
        }
    }

    fn detect_tiny_inline_function(func: &FunctionDeclaration) -> bool {
        if func.generator || func.is_async {
            return false;
        }
        if func.params.len() != 1 {
            return false;
        }
        let param_name = match &func.params[0] {
            Parameter::Identifier(name) => name,
            _ => return false,
        };
        if func.body.body.len() != 1 {
            return false;
        }
        let ret = match &func.body.body[0] {
            ASTNode::ReturnStatement(stmt) => stmt,
            _ => return false,
        };
        let expr = match &ret.argument {
            Some(expr) => expr,
            None => return false,
        };
        match expr {
            Expression::MemberExpression(member) => match member.object.as_ref() {
                Expression::Identifier(id) => {
                    id.name == *param_name
                        && matches!(member.property, MemberProperty::Identifier(_))
                }
                Expression::MemberExpression(inner) => {
                    if inner.computed || member.computed {
                        return false;
                    }
                    match inner.object.as_ref() {
                        Expression::Identifier(id) => {
                            id.name == *param_name
                                && matches!(inner.property, MemberProperty::Identifier(_))
                                && matches!(member.property, MemberProperty::Identifier(_))
                        }
                        _ => false,
                    }
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn try_emit_tiny_inline_call(
        &mut self,
        call: &CallExpression,
        ctx: &mut JSContext,
    ) -> Result<Option<u16>, String> {
        let callee_name = match call.callee.as_ref() {
            Expression::Identifier(id) => &id.name,
            _ => return Ok(None),
        };
        let func = match self.tiny_inline_functions.get(callee_name).cloned() {
            Some(func) => func,
            None => return Ok(None),
        };
        if call.arguments.len() != 1 {
            return Ok(None);
        }
        let arg_reg = match &call.arguments[0] {
            Argument::Expression(expr) => self.gen_expression(expr, ctx)?,
            Argument::Spread(_) => return Ok(None),
        };

        let ret = match &func.body.body[0] {
            ASTNode::ReturnStatement(stmt) => stmt,
            _ => return Ok(None),
        };
        let expr = match &ret.argument {
            Some(expr) => expr,
            None => return Ok(None),
        };

        let dst = self.alloc_register();
        match expr {
            Expression::MemberExpression(member) => match member.object.as_ref() {
                Expression::Identifier(_) => {
                    if let MemberProperty::Identifier(name) = &member.property {
                        let atom = ctx.intern(name);
                        self.emit(Opcode::GetNamedProp);
                        self.emit_u16(dst);
                        self.emit_u16(arg_reg);
                        self.emit_u32(atom.0);
                    } else {
                        self.free_register(arg_reg);
                        self.free_register(dst);
                        return Ok(None);
                    }
                }
                Expression::MemberExpression(inner) => {
                    let mid = self.alloc_register();
                    let atom1 = match &inner.property {
                        MemberProperty::Identifier(name) => ctx.intern(name),
                        _ => {
                            self.free_register(arg_reg);
                            self.free_register(mid);
                            self.free_register(dst);
                            return Ok(None);
                        }
                    };
                    let atom2 = match &member.property {
                        MemberProperty::Identifier(name) => ctx.intern(name),
                        _ => {
                            self.free_register(arg_reg);
                            self.free_register(mid);
                            self.free_register(dst);
                            return Ok(None);
                        }
                    };
                    self.emit(Opcode::GetNamedProp);
                    self.emit_u16(mid);
                    self.emit_u16(arg_reg);
                    self.emit_u32(atom1.0);
                    self.emit(Opcode::GetNamedProp);
                    self.emit_u16(dst);
                    self.emit_u16(mid);
                    self.emit_u32(atom2.0);
                    self.free_register(mid);
                }
                _ => {
                    self.free_register(arg_reg);
                    self.free_register(dst);
                    return Ok(None);
                }
            },
            _ => {
                self.free_register(arg_reg);
                self.free_register(dst);
                return Ok(None);
            }
        }

        self.free_register(arg_reg);
        Ok(Some(dst))
    }

    fn fused_op_from_binary_cmp(bin_op: &BinaryOp, jump_if_true: bool) -> Option<Opcode> {
        use BinaryOp::*;
        let op = match (bin_op, jump_if_true) {
            (Lt, true) => Opcode::LtJumpIf,
            (Lt, false) => Opcode::LtJumpIfNot,
            (Lte, true) => Opcode::LteJumpIf,
            (Lte, false) => Opcode::LteJumpIfNot,
            (Gt, true) => Opcode::GtJumpIf,
            (Gt, false) => Opcode::GtJumpIfNot,
            (Gte, true) => Opcode::GteJumpIf,
            (Gte, false) => Opcode::GteJumpIfNot,
            (Eq, true) => Opcode::EqJumpIf,
            (Eq, false) => Opcode::EqJumpIfNot,
            (Neq, true) => Opcode::NeqJumpIf,
            (Neq, false) => Opcode::NeqJumpIfNot,
            (StrictEq, true) => Opcode::StrictEqJumpIf,
            (StrictEq, false) => Opcode::StrictEqJumpIfNot,
            (StrictNeq, true) => Opcode::StrictNeqJumpIf,
            (StrictNeq, false) => Opcode::StrictNeqJumpIfNot,
            _ => return None,
        };
        Some(op)
    }

    fn emit(&mut self, op: Opcode) {
        self.code.push(op as u8);
    }

    fn emit_u8(&mut self, v: u8) {
        self.code.push(v);
    }

    fn emit_u16(&mut self, v: u16) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    fn emit_i32(&mut self, v: i32) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    fn emit_u32(&mut self, v: u32) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    fn emit_int_const(&mut self, dst: u16, val: i32) {
        if (i8::MIN as i32..=i8::MAX as i32).contains(&val) {
            self.emit(Opcode::LoadInt8);
            self.emit_u16(dst);
            self.emit_u8(val as i8 as u8);
        } else {
            self.emit(Opcode::LoadInt);
            self.emit_u16(dst);
            self.emit_i32(val);
        }
    }

    fn emit_bool_const(&mut self, dst: u16, b: bool) {
        if b {
            self.emit(Opcode::LoadTrue);
        } else {
            self.emit(Opcode::LoadFalse);
        }
        self.emit_u16(dst);
    }

    fn emit_backward_jump(&mut self, target: usize) {
        let compact_offset = (target as i64 - (self.code.len() as i64 + 2)) as i32;
        if compact_offset >= i8::MIN as i32 && compact_offset <= i8::MAX as i32 {
            self.emit(Opcode::Jump8);
            self.emit_u8(compact_offset as i8 as u8);
        } else {
            let full_offset = (target as i64 - (self.code.len() as i64 + 5)) as i32;
            self.emit(Opcode::Jump);
            self.emit_i32(full_offset);
        }
    }

    fn emit_conditional_backward_jump(&mut self, src: u16, target: usize, jump_if_true: bool) {
        let compact_offset = (target as i64 - (self.code.len() as i64 + 4)) as i32;
        if compact_offset >= i8::MIN as i32 && compact_offset <= i8::MAX as i32 {
            if jump_if_true {
                self.emit(Opcode::JumpIf8);
            } else {
                self.emit(Opcode::JumpIfNot8);
            }
            self.emit_u16(src);
            self.emit_u8(compact_offset as i8 as u8);
        } else {
            let full_offset = (target as i64 - (self.code.len() as i64 + 7)) as i32;
            if jump_if_true {
                self.emit(Opcode::JumpIf);
            } else {
                self.emit(Opcode::JumpIfNot);
            }
            self.emit_u16(src);
            self.emit_i32(full_offset);
        }
    }

    fn patch_jump(&mut self, patch_pos: usize, target_pos: usize) {
        if patch_pos >= 1 {
            let op_byte = self.code[patch_pos - 1];
            let is_uncond = op_byte == Opcode::Jump as u8;
            let is_cond = op_byte == Opcode::JumpIf as u8 || op_byte == Opcode::JumpIfNot as u8;

            if is_uncond || is_cond {
                let compact_offset = (target_pos as i64 - patch_pos as i64 - 1) as i32;
                if compact_offset >= i8::MIN as i32 && compact_offset <= i8::MAX as i32 {
                    if is_uncond {
                        self.code[patch_pos - 1] = Opcode::Jump8 as u8;
                    } else {
                        self.code[patch_pos - 3] = if op_byte == Opcode::JumpIf as u8 {
                            Opcode::JumpIf8 as u8
                        } else {
                            Opcode::JumpIfNot8 as u8
                        };
                    }
                    self.code[patch_pos] = compact_offset as i8 as u8;

                    let fill_end = patch_pos + 4;
                    for i in (patch_pos + 1)..fill_end {
                        self.code[i] = Opcode::Nop as u8;
                    }
                    return;
                }
            }
        }

        if patch_pos >= 6 {
            let fused_op = self.code[patch_pos - 5];
            let is_fused = fused_op == Opcode::LtJumpIfNot as u8
                || fused_op == Opcode::LtJumpIf as u8
                || fused_op == Opcode::LteJumpIfNot as u8
                || fused_op == Opcode::LteJumpIf as u8
                || fused_op == Opcode::GtJumpIfNot as u8
                || fused_op == Opcode::GtJumpIf as u8
                || fused_op == Opcode::GteJumpIfNot as u8
                || fused_op == Opcode::GteJumpIf as u8
                || fused_op == Opcode::EqJumpIfNot as u8
                || fused_op == Opcode::EqJumpIf as u8
                || fused_op == Opcode::NeqJumpIfNot as u8
                || fused_op == Opcode::NeqJumpIf as u8
                || fused_op == Opcode::StrictEqJumpIfNot as u8
                || fused_op == Opcode::StrictEqJumpIf as u8
                || fused_op == Opcode::StrictNeqJumpIfNot as u8
                || fused_op == Opcode::StrictNeqJumpIf as u8;
            if is_fused {
                let offset = (target_pos as i64 - patch_pos as i64 - 4) as i32;
                self.code[patch_pos..patch_pos + 4].copy_from_slice(&offset.to_le_bytes());
                return;
            }
        }

        let full_offset = (target_pos as i64 - patch_pos as i64 - 4) as i32;
        self.code[patch_pos..patch_pos + 4].copy_from_slice(&full_offset.to_le_bytes());
    }

    fn emit_line(&mut self, line: u32) {
        self.line_table.add_entry(self.code.len() as u32, line);
    }

    fn add_constant(&mut self, val: JSValue) -> u32 {
        let idx = self.constants.len() as u32;
        self.constants.push(val);
        idx
    }

    fn intern_and_add_const(&mut self, ctx: &mut JSContext, name: &str) -> u32 {
        let atom = ctx.atom_table_mut().intern(name);
        self.add_constant(JSValue::new_string(atom))
    }

    fn emit_embedded_function(
        &mut self,
        bytecode: &Bytecode,
        upvalues: &[(u32, u16)],
        func_name_atom: u32,
        dst: u16,
        is_generator: bool,
        is_async: bool,
    ) {
        if is_generator && is_async {
            self.emit(Opcode::NewAsyncGeneratorFunction);
        } else if is_generator {
            self.emit(Opcode::NewGeneratorFunction);
        } else if is_async {
            self.emit(Opcode::NewAsyncFunction);
        } else {
            self.emit(Opcode::NewFunction);
        }
        self.emit_u16(bytecode.param_count);
        self.emit_u8(if bytecode.uses_arguments { 1 } else { 0 });
        self.emit_u8(if bytecode.is_strict { 1 } else { 0 });
        let locals_bytes = (bytecode.locals_count as u16).to_le_bytes();
        self.code.extend_from_slice(&locals_bytes);
        self.emit_u32(bytecode.code.len() as u32);
        self.code.extend_from_slice(&bytecode.code);
        self.emit_u32(bytecode.constants.len() as u32);
        for c in &bytecode.constants {
            if c.is_undefined() {
                self.emit_u8(0);
            } else if c.is_null() {
                self.emit_u8(1);
            } else if c.is_bool() && c.get_bool() {
                self.emit_u8(2);
            } else if c.is_bool() && !c.get_bool() {
                self.emit_u8(3);
            } else if c.is_int() {
                self.emit_u8(4);
                self.code.extend_from_slice(&c.get_int().to_le_bytes());
            } else if c.is_float() {
                self.emit_u8(5);
                let bytes = c.get_float().to_le_bytes();
                self.code.extend_from_slice(&bytes);
            } else if c.is_string() {
                self.emit_u8(6);
                self.emit_u32(c.get_atom().0);
            } else {
                self.emit_u8(0);
            }
        }
        self.emit_u32(upvalues.len() as u32);
        for (atom_id, local_idx) in upvalues {
            self.emit_u32(*atom_id);
            self.emit_u32(*local_idx as u32);
        }

        if let Some(ref table) = bytecode.line_number_table {
            let entries = table.entries();
            self.emit_u32(entries.len() as u32);
            for (off, line) in entries {
                self.emit_u32(*off);
                self.emit_u32(*line);
            }
        } else {
            self.emit_u32(0);
        }
        self.emit_u32(func_name_atom);
        self.emit_u32(bytecode.var_name_to_slot.len() as u32);
        for &(atom_id, slot) in bytecode.var_name_to_slot.iter() {
            self.emit_u32(atom_id);
            self.emit_u16(slot);
        }
        self.emit_u16(dst);
    }

    fn alloc_register(&mut self) -> u16 {
        while let Some(free_r) = self.free_list.pop() {
            if !self.reserved_slots.contains(&free_r) {
                if free_r > self.max_register {
                    self.max_register = free_r;
                }
                return free_r;
            }
        }

        let mut attempts = 0u32;
        while self.reserved_slots.contains(&self.next_register) {
            self.next_register = self.next_register.wrapping_add(1);
            attempts += 1;
            if attempts >= 65536 {
                panic!("alloc_register: all u16 slots are reserved");
            }
        }
        let r = self.next_register;
        self.next_register = self.next_register.wrapping_add(1);
        if r > self.max_register {
            self.max_register = r;
        }
        r
    }

    fn free_register(&mut self, r: u16) {
        if !self.reserved_slots.contains(&r) && !self.free_list.contains(&r) {
            self.free_list.push(r);
        }
    }

    fn push_scope(&mut self) {
        self.scope_stack.push(FxHashMap::default());
        self.scope_is_global.push(false);
        self.depth += 1;
    }

    fn pop_scope(&mut self) {
        self.scope_stack.pop();
        self.scope_is_global.pop();
        self.depth = self.depth.saturating_sub(1);
    }

    fn declare_var(&mut self, name: &str, kind: VariableKind) -> u16 {
        if kind == VariableKind::Var {
            if let Some(entry) = self.scope_stack[0].get(name) {
                return entry.slot;
            }
        }
        let slot = self.alloc_register();
        let entry = VarEntry {
            slot,
            kind,
            initialized: kind == VariableKind::Var,
        };
        self.reserved_slots.insert(slot);
        self.all_var_names.push((name.to_string(), slot));
        match kind {
            VariableKind::Var => {
                self.scope_stack[0].insert(name.to_string(), entry);
            }
            VariableKind::Let | VariableKind::Const => {
                self.scope_stack
                    .last_mut()
                    .unwrap()
                    .insert(name.to_string(), entry);
            }
        }
        slot
    }

    fn is_strict_reserved_word(name: &str) -> bool {
        matches!(
            name,
            "implements"
                | "interface"
                | "let"
                | "package"
                | "private"
                | "protected"
                | "public"
                | "static"
                | "yield"
                | "eval"
                | "arguments"
        )
    }

    fn has_use_strict_directive(body: &BlockStatement) -> bool {
        for stmt in &body.body {
            match stmt {
                ASTNode::ExpressionStatement(es) => {
                    if let Expression::Literal(Literal::String(s, has_escape)) = &es.expression {
                        if !has_escape && s == "use strict" {
                            return true;
                        }
                    } else {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        false
    }

    fn is_global_scope(&self) -> bool {
        self.scope_is_global.first().copied().unwrap_or(false)
    }

    fn is_global_var(&self, name: &str) -> bool {
        if !self.is_global_scope() {
            return false;
        }
        self.scope_stack
            .first()
            .map_or(false, |s| s.contains_key(name))
    }

    fn lookup_var(&self, name: &str) -> Option<&VarEntry> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(entry) = scope.get(name) {
                return Some(entry);
            }
        }
        None
    }

    fn lookup_var_slot(&self, name: &str) -> Option<u16> {
        self.lookup_var(name).map(|e| e.slot)
    }

    fn resolve_var(&mut self, name: &str, ctx: &mut JSContext) -> Option<VarLocation> {
        if let Some(entry) = self.lookup_var(name) {
            return Some(VarLocation::Local(entry.slot));
        }
        if let Some(parent_var) = self.parent_vars.get(name) {
            if let Some(&upvalue_slot) = self.upvalue_slots.get(name) {
                return Some(VarLocation::Upvalue(upvalue_slot));
            }
            let upvalue_slot = self.upvalues.len() as u16;
            self.upvalue_slots.insert(name.to_string(), upvalue_slot);
            self.reserved_slots.insert(upvalue_slot);
            let atom = ctx.atom_table_mut().intern(name);
            let local_idx = if parent_var.is_inherited {
                u16::MAX
            } else {
                parent_var.local_idx
            };
            self.upvalues.push((atom.0, local_idx));
            return Some(VarLocation::Upvalue(upvalue_slot));
        }
        None
    }

    fn collect_inherited_upvalues_stmt(
        stmt: &ASTNode,
        locals: &mut std::collections::HashSet<String>,
        parent_vars: &FxHashMap<String, ParentVar>,
        result: &mut std::collections::HashSet<String>,
    ) {
        match stmt {
            ASTNode::ExpressionStatement(es) => {
                Self::collect_inherited_upvalues_expr(&es.expression, locals, parent_vars, result);
            }
            ASTNode::VariableDeclaration(decl) => {
                for d in &decl.declarations {
                    if let BindingPattern::Identifier(name) = &d.id {
                        if let Some(init) = &d.init {
                            Self::collect_inherited_upvalues_expr(
                                init,
                                locals,
                                parent_vars,
                                result,
                            );
                        }
                        locals.insert(name.clone());
                    }
                }
            }
            ASTNode::BlockStatement(block) => {
                let mut block_locals = locals.clone();
                for s in &block.body {
                    Self::collect_inherited_upvalues_stmt(
                        s,
                        &mut block_locals,
                        parent_vars,
                        result,
                    );
                }
            }
            ASTNode::IfStatement(stmt) => {
                Self::collect_inherited_upvalues_expr(&stmt.test, locals, parent_vars, result);
                let mut cons_locals = locals.clone();
                Self::collect_inherited_upvalues_stmt(
                    &stmt.consequent,
                    &mut cons_locals,
                    parent_vars,
                    result,
                );
                if let Some(alt) = &stmt.alternate {
                    let mut alt_locals = locals.clone();
                    Self::collect_inherited_upvalues_stmt(
                        alt,
                        &mut alt_locals,
                        parent_vars,
                        result,
                    );
                }
            }
            ASTNode::WhileStatement(stmt) => {
                Self::collect_inherited_upvalues_expr(&stmt.test, locals, parent_vars, result);
                let mut loop_locals = locals.clone();
                Self::collect_inherited_upvalues_stmt(
                    &stmt.body,
                    &mut loop_locals,
                    parent_vars,
                    result,
                );
            }
            ASTNode::DoWhileStatement(stmt) => {
                let mut loop_locals = locals.clone();
                Self::collect_inherited_upvalues_stmt(
                    &stmt.body,
                    &mut loop_locals,
                    parent_vars,
                    result,
                );
                Self::collect_inherited_upvalues_expr(&stmt.test, locals, parent_vars, result);
            }
            ASTNode::ForStatement(stmt) => {
                let mut for_locals = locals.clone();
                if let Some(init) = &stmt.init {
                    match init {
                        ForInit::VariableDeclaration(decl) => {
                            for d in &decl.declarations {
                                if let BindingPattern::Identifier(name) = &d.id {
                                    if let Some(init_expr) = &d.init {
                                        Self::collect_inherited_upvalues_expr(
                                            init_expr,
                                            &mut for_locals,
                                            parent_vars,
                                            result,
                                        );
                                    }
                                    for_locals.insert(name.clone());
                                }
                            }
                        }
                        ForInit::Expression(expr) => {
                            Self::collect_inherited_upvalues_expr(
                                expr,
                                &mut for_locals,
                                parent_vars,
                                result,
                            );
                        }
                    }
                }
                if let Some(test) = &stmt.test {
                    Self::collect_inherited_upvalues_expr(
                        test,
                        &mut for_locals,
                        parent_vars,
                        result,
                    );
                }
                Self::collect_inherited_upvalues_stmt(
                    &stmt.body,
                    &mut for_locals,
                    parent_vars,
                    result,
                );
                if let Some(update) = &stmt.update {
                    Self::collect_inherited_upvalues_expr(
                        update,
                        &mut for_locals,
                        parent_vars,
                        result,
                    );
                }
            }
            ASTNode::SwitchStatement(stmt) => {
                Self::collect_inherited_upvalues_expr(
                    &stmt.discriminant,
                    locals,
                    parent_vars,
                    result,
                );
                for case in &stmt.cases {
                    let mut case_locals = locals.clone();
                    if let Some(test) = &case.test {
                        Self::collect_inherited_upvalues_expr(
                            test,
                            &mut case_locals,
                            parent_vars,
                            result,
                        );
                    }
                    for s in &case.consequent {
                        Self::collect_inherited_upvalues_stmt(
                            s,
                            &mut case_locals,
                            parent_vars,
                            result,
                        );
                    }
                }
            }
            ASTNode::ReturnStatement(stmt) => {
                if let Some(expr) = &stmt.argument {
                    Self::collect_inherited_upvalues_expr(expr, locals, parent_vars, result);
                }
            }
            ASTNode::BreakStatement(_) | ASTNode::ContinueStatement(_) => {}
            ASTNode::FunctionDeclaration(func) => {
                let mut func_locals = std::collections::HashSet::new();
                func_locals.insert(func.name.clone());
                for param in &func.params {
                    Self::collect_parameter_bindings(param, &mut func_locals);
                }
                for s in &func.body.body {
                    Self::collect_inherited_upvalues_stmt(s, &mut func_locals, parent_vars, result);
                }
            }
            _ => {}
        }
    }

    fn collect_binding_pattern_bindings(
        pattern: &BindingPattern,
        out: &mut std::collections::HashSet<String>,
    ) {
        match pattern {
            BindingPattern::Identifier(name) => {
                out.insert(name.clone());
            }
            BindingPattern::ArrayPattern(arr) => {
                for elem in &arr.elements {
                    if let Some(elem) = elem {
                        match elem {
                            PatternElement::Pattern(target) => {
                                Self::collect_assignment_target_bindings(target, out);
                            }
                            PatternElement::RestElement(rest) => {
                                Self::collect_assignment_target_bindings(&rest.argument, out);
                            }
                            PatternElement::AssignmentPattern(ap) => {
                                Self::collect_assignment_target_bindings(&ap.left, out);
                            }
                        }
                    }
                }
            }
            BindingPattern::ObjectPattern(obj) => {
                for prop in &obj.properties {
                    match prop {
                        ObjectPatternProperty::Property { value, .. } => {
                            Self::collect_assignment_target_bindings(value, out);
                        }
                        ObjectPatternProperty::RestElement(rest) => {
                            Self::collect_assignment_target_bindings(&rest.argument, out);
                        }
                    }
                }
            }
            BindingPattern::AssignmentPattern(ap) => {
                Self::collect_assignment_target_bindings(&ap.left, out);
            }
        }
    }

    fn collect_assignment_target_bindings(
        target: &AssignmentTarget,
        out: &mut std::collections::HashSet<String>,
    ) {
        match target {
            AssignmentTarget::Identifier(name) => {
                out.insert(name.clone());
            }
            AssignmentTarget::ArrayPattern(arr) => {
                Self::collect_binding_pattern_bindings(
                    &BindingPattern::ArrayPattern(arr.clone()),
                    out,
                );
            }
            AssignmentTarget::ObjectPattern(obj) => {
                Self::collect_binding_pattern_bindings(
                    &BindingPattern::ObjectPattern(obj.clone()),
                    out,
                );
            }
            AssignmentTarget::AssignmentPattern(ap) => {
                Self::collect_assignment_target_bindings(&ap.left, out);
            }
            AssignmentTarget::RestElement(rest) => {
                Self::collect_assignment_target_bindings(&rest.argument, out);
            }
            _ => {}
        }
    }

    fn collect_parameter_bindings(param: &Parameter, out: &mut std::collections::HashSet<String>) {
        match param {
            Parameter::Identifier(name) => {
                out.insert(name.clone());
            }
            Parameter::Pattern(pattern) => {
                Self::collect_binding_pattern_bindings(pattern, out);
            }
            Parameter::AssignmentPattern(ap) => {
                Self::collect_assignment_target_bindings(&ap.left, out);
            }
            Parameter::RestElement(rest) => {
                Self::collect_assignment_target_bindings(&rest.argument, out);
            }
        }
    }

    fn collect_inherited_upvalues_expr(
        expr: &Expression,
        locals: &mut std::collections::HashSet<String>,
        parent_vars: &FxHashMap<String, ParentVar>,
        result: &mut std::collections::HashSet<String>,
    ) {
        match expr {
            Expression::Identifier(id) => {
                if !locals.contains(&id.name) && parent_vars.contains_key(&id.name) {
                    result.insert(id.name.clone());
                }
            }
            Expression::BinaryExpression(bin) => {
                Self::collect_inherited_upvalues_expr(&bin.left, locals, parent_vars, result);
                Self::collect_inherited_upvalues_expr(&bin.right, locals, parent_vars, result);
            }
            Expression::UnaryExpression(unary) => {
                Self::collect_inherited_upvalues_expr(&unary.argument, locals, parent_vars, result);
            }
            Expression::UpdateExpression(update) => {
                if let Expression::Identifier(id) = &*update.argument {
                    if !locals.contains(&id.name) && parent_vars.contains_key(&id.name) {
                        result.insert(id.name.clone());
                    }
                }
            }
            Expression::AssignmentExpression(assign) => {
                match &assign.left {
                    AssignmentTarget::Identifier(name) => {
                        if !locals.contains(name) && parent_vars.contains_key(name) {
                            result.insert(name.clone());
                        }
                    }
                    AssignmentTarget::MemberExpression(member) => {
                        Self::collect_inherited_upvalues_expr(
                            &member.object,
                            locals,
                            parent_vars,
                            result,
                        );
                        if let MemberProperty::Computed(e) = &member.property {
                            Self::collect_inherited_upvalues_expr(e, locals, parent_vars, result);
                        }
                    }
                    AssignmentTarget::ComputedMember(obj, key) => {
                        Self::collect_inherited_upvalues_expr(obj, locals, parent_vars, result);
                        Self::collect_inherited_upvalues_expr(key, locals, parent_vars, result);
                    }
                    _ => {}
                }
                Self::collect_inherited_upvalues_expr(&assign.right, locals, parent_vars, result);
            }
            Expression::ConditionalExpression(cond) => {
                Self::collect_inherited_upvalues_expr(&cond.test, locals, parent_vars, result);
                Self::collect_inherited_upvalues_expr(
                    &cond.consequent,
                    locals,
                    parent_vars,
                    result,
                );
                Self::collect_inherited_upvalues_expr(&cond.alternate, locals, parent_vars, result);
            }
            Expression::LogicalExpression(logical) => {
                Self::collect_inherited_upvalues_expr(&logical.left, locals, parent_vars, result);
                Self::collect_inherited_upvalues_expr(&logical.right, locals, parent_vars, result);
            }
            Expression::MemberExpression(member) => {
                Self::collect_inherited_upvalues_expr(&member.object, locals, parent_vars, result);
                if let MemberProperty::Computed(e) = &member.property {
                    Self::collect_inherited_upvalues_expr(e, locals, parent_vars, result);
                }
            }
            Expression::CallExpression(call) => {
                Self::collect_inherited_upvalues_expr(&call.callee, locals, parent_vars, result);
                for arg in &call.arguments {
                    if let Argument::Expression(e) = arg {
                        Self::collect_inherited_upvalues_expr(e, locals, parent_vars, result);
                    }
                }
            }
            Expression::NewExpression(new) => {
                Self::collect_inherited_upvalues_expr(&new.callee, locals, parent_vars, result);
                for arg in &new.arguments {
                    if let Argument::Expression(e) = arg {
                        Self::collect_inherited_upvalues_expr(e, locals, parent_vars, result);
                    }
                }
            }
            Expression::ArrayExpression(arr) => {
                for elem in &arr.elements {
                    if let Some(ArrayElement::Expression(e)) = elem {
                        Self::collect_inherited_upvalues_expr(e, locals, parent_vars, result);
                    }
                }
            }
            Expression::ObjectExpression(obj) => {
                for prop in &obj.properties {
                    if let Property::Property { value, .. } = prop {
                        Self::collect_inherited_upvalues_expr(value, locals, parent_vars, result);
                    }
                }
            }
            Expression::FunctionExpression(func) => {
                let mut func_locals = std::collections::HashSet::new();
                if let Some(ref name) = func.name {
                    func_locals.insert(name.clone());
                }
                for param in &func.params {
                    Self::collect_parameter_bindings(param, &mut func_locals);
                }
                for s in &func.body.body {
                    Self::collect_inherited_upvalues_stmt(s, &mut func_locals, parent_vars, result);
                }
            }
            Expression::ArrowFunction(arrow) => {
                let mut func_locals = std::collections::HashSet::new();
                for param in &arrow.params {
                    Self::collect_parameter_bindings(param, &mut func_locals);
                }
                let body = match &arrow.body {
                    ArrowBody::Expression(expr) => BlockStatement {
                        body: vec![ASTNode::ReturnStatement(ReturnStatement {
                            argument: Some((**expr).clone()),
                        })],
                        lines: vec![],
                    },
                    ArrowBody::Block(block) => block.clone(),
                };
                for s in &body.body {
                    Self::collect_inherited_upvalues_stmt(s, &mut func_locals, parent_vars, result);
                }
            }
            Expression::SequenceExpression(seq) => {
                for e in &seq.expressions {
                    Self::collect_inherited_upvalues_expr(e, locals, parent_vars, result);
                }
            }
            Expression::TemplateLiteral(tpl) => {
                for e in &tpl.expressions {
                    Self::collect_inherited_upvalues_expr(e, locals, parent_vars, result);
                }
            }
            Expression::TaggedTemplateExpression(tagged) => {
                Self::collect_inherited_upvalues_expr(&tagged.tag, locals, parent_vars, result);
                for e in &tagged.quasi.expressions {
                    Self::collect_inherited_upvalues_expr(e, locals, parent_vars, result);
                }
            }
            Expression::SpreadElement(spread) => {
                Self::collect_inherited_upvalues_expr(
                    &spread.argument,
                    locals,
                    parent_vars,
                    result,
                );
            }
            _ => {}
        }
    }

    pub fn compile_expression(
        &mut self,
        expr: &Expression,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        self.gen_expression(expr, ctx)
    }

    pub fn compile_function(
        &mut self,
        params: &[Parameter],
        body: &BlockStatement,
        ctx: &mut JSContext,
    ) -> Result<(Bytecode, Vec<(u32, u16)>), String> {
        self.compile_function_with_parent(params, body, ctx, FxHashMap::default(), None, false)
    }

    pub fn compile_script(
        &mut self,
        body: &BlockStatement,
        ctx: &mut JSContext,
    ) -> Result<(Bytecode, Vec<(u32, u16)>), String> {
        let saved_scope_is_global = self.scope_is_global.clone();
        self.scope_is_global = vec![true];
        let result =
            self.compile_function_with_parent(&[], body, ctx, FxHashMap::default(), None, true);
        self.scope_is_global = saved_scope_is_global;
        result
    }

    fn compile_function_with_parent(
        &mut self,
        params: &[Parameter],
        body: &BlockStatement,
        ctx: &mut JSContext,
        parent_vars: FxHashMap<String, ParentVar>,
        function_name: Option<&str>,
        return_last_expr: bool,
    ) -> Result<(Bytecode, Vec<(u32, u16)>), String> {
        self.is_strict = self.is_strict || Self::has_use_strict_directive(body);
        let saved_is_script_level = self.is_script_level;
        let saved_scope_is_global = self.scope_is_global.clone();
        self.next_register = 0;
        self.max_register = 0;
        self.free_list.clear();
        self.scope_stack = vec![FxHashMap::default()];

        if self.scope_is_global != vec![true] {
            self.scope_is_global = vec![false];
        }
        self.reserved_slots.clear();
        self.all_var_names.clear();
        self.code.clear();
        self.constants.clear();
        self.parent_vars = parent_vars.clone();
        self.upvalues.clear();
        self.upvalue_slots.clear();
        self.line_table = LineNumberTable::new();
        self.depth = 0;
        if return_last_expr {
            self.is_script_level = true;
        } else {
            self.is_script_level = false;
        }
        self.function_uses_arguments = false;
        self.current_function_name = function_name.map(|s| s.to_string());

        let this_slot = self.alloc_register();
        assert_eq!(this_slot, 0);
        self.reserved_slots.insert(this_slot);

        let mut fixed_param_count = 0u16;
        for param in params {
            if self.is_strict {
                match param {
                    Parameter::Identifier(name) if Self::is_strict_reserved_word(name) => {
                        return Err(format!(
                            "SyntaxError: '{}' cannot be declared in strict mode code",
                            name
                        ));
                    }
                    _ => {}
                }
            }
            match param {
                Parameter::Identifier(name) => {
                    self.declare_var(name, VariableKind::Var);
                    fixed_param_count += 1;
                }
                Parameter::Pattern(pattern) => {
                    self.predeclare_binding_pattern(pattern, VariableKind::Var);
                    fixed_param_count += 1;
                }
                Parameter::AssignmentPattern(ap) => {
                    self.predeclare_assignment_target(&ap.left, VariableKind::Var);
                    fixed_param_count += 1;
                }
                Parameter::RestElement(rest) => {
                    self.predeclare_assignment_target(&rest.argument, VariableKind::Var);
                }
            }
        }
        self.max_register = self.max_register.max(fixed_param_count);

        for (index, param) in params.iter().enumerate() {
            let value_reg = (index as u16) + 1;
            match param {
                Parameter::Identifier(name) => {
                    if let Some(slot) = self.lookup_var_slot(name) {
                        if slot != value_reg {
                            self.emit(Opcode::Move);
                            self.emit_u16(slot);
                            self.emit_u16(value_reg);
                        }
                    }
                }
                Parameter::Pattern(pattern) => {
                    self.gen_binding_pattern(pattern, value_reg, VariableKind::Var, ctx)?;
                }
                Parameter::AssignmentPattern(ap) => {
                    let binding_name = match &*ap.left {
                        AssignmentTarget::Identifier(name) => Some(name.as_str()),
                        _ => None,
                    };
                    self.gen_default_value_binding(&ap.right, value_reg, binding_name, ctx)?;
                    self.gen_assignment_target_binding(
                        &ap.left,
                        value_reg,
                        VariableKind::Var,
                        ctx,
                    )?;
                }
                Parameter::RestElement(_) => break,
            }
        }

        if params
            .iter()
            .any(|p| matches!(p, Parameter::RestElement(_)))
        {
            let rest_source_reg = fixed_param_count + 1;
            self.emit(Opcode::GatherRest);
            self.emit_u16(rest_source_reg);

            for param in params {
                if let Parameter::RestElement(rest) = param {
                    self.gen_assignment_target_binding(
                        &rest.argument,
                        rest_source_reg,
                        VariableKind::Var,
                        ctx,
                    )?;
                    break;
                }
            }
        }

        let mut locals: std::collections::HashSet<String> =
            self.scope_stack[0].keys().cloned().collect();
        let mut inherited = std::collections::HashSet::new();
        for stmt in &body.body {
            Self::collect_inherited_upvalues_stmt(stmt, &mut locals, &parent_vars, &mut inherited);
        }
        for name in inherited {
            if let Some(parent_var) = parent_vars.get(&name) {
                if self.upvalue_slots.contains_key(&name) {
                    continue;
                }
                let upvalue_slot = self.upvalues.len() as u16;
                self.upvalue_slots.insert(name.clone(), upvalue_slot);
                self.reserved_slots.insert(upvalue_slot);
                let atom = ctx.atom_table_mut().intern(name.as_str());
                let local_idx = if parent_var.is_inherited {
                    u16::MAX
                } else {
                    parent_var.local_idx
                };
                self.upvalues.push((atom.0, local_idx));
            }
        }

        for stmt in &body.body {
            if let ASTNode::FunctionDeclaration(func) = stmt {
                self.declare_var(&func.name, VariableKind::Var);
                if Self::detect_tiny_inline_function(func) {
                    self.tiny_inline_functions
                        .insert(func.name.clone(), func.clone());
                }
            }
        }

        for stmt in &body.body {
            if let ASTNode::VariableDeclaration(decl) = stmt {
                if decl.kind == VariableKind::Var {
                    for d in &decl.declarations {
                        self.predeclare_binding_pattern(&d.id, VariableKind::Var);
                    }
                }
            }
        }

        self.push_scope();
        self.pre_scan_let_const(&body.body)?;
        self.emit_tdz_for_block(&body.body)?;

        {
            for stmt in &body.body {
                if let ASTNode::FunctionDeclaration(func) = stmt {
                    let slot = self
                        .lookup_var_slot(&func.name)
                        .unwrap_or_else(|| self.declare_var(&func.name, VariableKind::Var));
                    let reg = self.gen_function_expression(
                        &func.params,
                        &func.body,
                        Some(&func.name),
                        ctx,
                        func.generator,
                        func.is_async,
                        false,
                    )?;
                    if slot != reg {
                        self.emit(Opcode::Move);
                        self.emit_u16(slot);
                        self.emit_u16(reg);
                        self.free_register(reg);
                    }

                    if self.is_global_scope() && !(self.is_strict && self.is_eval) {
                        let atom = ctx.intern(&func.name);
                        let idx = self.add_constant(JSValue::new_string(atom));
                        if self.is_eval {
                            self.emit(Opcode::SetGlobalVar);
                        } else {
                            self.emit(Opcode::SetGlobal);
                        }
                        self.emit_u32(idx);
                        self.emit_u16(slot);
                    }
                }
            }
        }

        let stmt_count = body.body.len();
        let mut last_expr_reg: Option<u16> = None;

        let mut last_nonempty_stmt_idx = stmt_count.wrapping_sub(1);
        while last_nonempty_stmt_idx < stmt_count {
            if !matches!(body.body[last_nonempty_stmt_idx], ASTNode::EmptyStatement) {
                break;
            }
            if last_nonempty_stmt_idx == 0 {
                last_nonempty_stmt_idx = stmt_count.wrapping_sub(1);
                break;
            }
            last_nonempty_stmt_idx -= 1;
        }

        for (i, stmt) in body.body.iter().enumerate() {
            if let Some(&line) = body.lines.get(i) {
                self.emit_line(line);
            }

            if return_last_expr && i == last_nonempty_stmt_idx {
                if let ASTNode::ExpressionStatement(es) = stmt {
                    last_expr_reg = Some(self.gen_expression(&es.expression, ctx)?);
                } else if matches!(stmt, ASTNode::FunctionDeclaration(_)) {
                } else if let Some(reg) = self.gen_statement_as_expression(stmt, ctx)? {
                    last_expr_reg = Some(reg);
                } else {
                    self.gen_statement(stmt, ctx)?;
                }
            } else if matches!(stmt, ASTNode::FunctionDeclaration(_)) {
            } else {
                self.gen_statement(stmt, ctx)?;
            }
        }

        self.pop_scope();

        let already_returns = body
            .body
            .last()
            .map_or(false, |s| matches!(s, ASTNode::ReturnStatement(_)));
        if !already_returns {
            let return_reg = if let Some(reg) = last_expr_reg {
                reg
            } else {
                self.emit(Opcode::LoadUndefined);
                self.emit_u16(0);
                0
            };
            self.emit(Opcode::Return);
            self.emit_u16(return_reg);
        }

        let line_table = std::mem::take(&mut self.line_table);
        let code = std::mem::take(&mut self.code);
        let mut constants = std::mem::take(&mut self.constants);
        if self.function_uses_arguments {
            let atom = if let Some(atom) = ctx.lookup_atom("__usesArguments__") {
                atom
            } else {
                ctx.atom_table_mut().intern("__usesArguments__")
            };
            if !constants
                .iter()
                .any(|v| v.is_string() && v.get_atom() == atom)
            {
                constants.push(JSValue::new_string(atom));
            }
        }
        self.is_script_level = saved_is_script_level;
        self.scope_is_global = saved_scope_is_global;
        let var_name_to_slot: std::rc::Rc<Vec<(u32, u16)>> = std::rc::Rc::new(
            self.all_var_names
                .iter()
                .map(|(name, slot)| {
                    let atom = ctx.intern(name);
                    (atom.0, *slot)
                })
                .collect(),
        );
        let mut bc = Bytecode {
            code,
            constants,
            locals_count: (self.max_register as u32) + 1,
            param_count: fixed_param_count,
            line_number_table: if line_table.entries().is_empty() {
                None
            } else {
                Some(line_table)
            },
            ic_table: crate::compiler::InlineCacheTable::new(),
            shared_ic_table_ptr: std::ptr::null_mut(),
            shared_code_ptr: std::ptr::null(),
            shared_code_len: 0,
            shared_const_ptr: std::ptr::null(),
            shared_const_len: 0,
            uses_arguments: self.function_uses_arguments,
            is_strict: self.is_strict,
            var_name_to_slot,
            nested_bytecodes: std::collections::HashMap::new(),
            is_simple_constructor: false,
            simple_constructor_props: Vec::new(),
            cached_constructor_final_shape: None,
            cached_constructor_atoms: Vec::new(),
        };
        bc.is_simple_constructor = detect_simple_constructor_props(&bc);
        if bc.is_simple_constructor {
            let code = &bc.code;
            let mut pc = 0usize;
            while pc < code.len() {
                let op = Opcode::from_u8_unchecked(code[pc]);
                let size = Opcode::instruction_size(op);
                if size == 0 || pc + size > code.len() {
                    break;
                }
                if op == Opcode::SetNamedProp {
                    let obj = u16::from_le_bytes([code[pc + 1], code[pc + 2]]);
                    let val = u16::from_le_bytes([code[pc + 3], code[pc + 4]]);
                    let atom = u32::from_le_bytes([
                        code[pc + 5],
                        code[pc + 6],
                        code[pc + 7],
                        code[pc + 8],
                    ]);
                    if obj == 0 && val >= 1 && val <= bc.param_count {
                        bc.simple_constructor_props.push((
                            crate::runtime::atom::Atom(atom),
                            val - 1,
                            (pc + size) as u16,
                        ));
                        bc.cached_constructor_atoms
                            .push(crate::runtime::atom::Atom(atom));
                    }
                }
                pc += size;
            }
        }
        Ok((bc, std::mem::take(&mut self.upvalues)))
    }

    pub fn take_bytecode(&mut self) -> (Vec<u8>, Vec<JSValue>, u32, u16) {
        let result = (
            std::mem::take(&mut self.code),
            std::mem::take(&mut self.constants),
            (self.max_register as u32) + 1,
            0,
        );
        self.next_register = 0;
        self.max_register = 0;
        self.free_list.clear();
        self.scope_stack = vec![FxHashMap::default()];
        self.reserved_slots.clear();
        self.upvalues.clear();
        self.upvalue_slots.clear();
        self.depth = 0;
        self.function_uses_arguments = false;

        result
    }

    fn pre_scan_let_const(&mut self, body: &[ASTNode]) -> Result<(), String> {
        for node in body {
            if let ASTNode::VariableDeclaration(decl) = node {
                if decl.kind != VariableKind::Var {
                    for d in &decl.declarations {
                        self.pre_scan_binding(&d.id, decl.kind);
                    }
                }
            } else if let ASTNode::ClassDeclaration(class) = node {
                if let Some(last) = self.scope_stack.last() {
                    if !last.contains_key(&class.name) {
                        self.declare_var(&class.name, VariableKind::Let);
                    }
                }
            }
        }
        Ok(())
    }

    fn pre_scan_binding(&mut self, pattern: &BindingPattern, kind: VariableKind) {
        match pattern {
            BindingPattern::Identifier(name) => {
                if let Some(last) = self.scope_stack.last() {
                    if !last.contains_key(name) {
                        self.declare_var(name, kind);
                    }
                }
            }
            BindingPattern::ArrayPattern(arr) => {
                for elem in &arr.elements {
                    if let Some(pat) = elem {
                        match pat {
                            PatternElement::Pattern(p) => {
                                self.pre_scan_assignment_target_binding(p, kind)
                            }
                            PatternElement::RestElement(r) => {
                                self.pre_scan_assignment_target_binding(&r.argument, kind)
                            }
                            PatternElement::AssignmentPattern(ap) => {
                                self.pre_scan_assignment_target_binding(&ap.left, kind)
                            }
                        }
                    }
                }
            }
            BindingPattern::ObjectPattern(obj) => {
                for prop in &obj.properties {
                    match prop {
                        ObjectPatternProperty::Property { value, .. } => {
                            self.pre_scan_assignment_target_binding(value, kind);
                        }
                        ObjectPatternProperty::RestElement(r) => {
                            self.pre_scan_assignment_target_binding(&r.argument, kind);
                        }
                    }
                }
            }
            BindingPattern::AssignmentPattern(ap) => {
                self.pre_scan_assignment_target_binding(&ap.left, kind);
            }
        }
    }

    fn pre_scan_assignment_target_binding(
        &mut self,
        target: &AssignmentTarget,
        kind: VariableKind,
    ) {
        match target {
            AssignmentTarget::Identifier(name) => {
                if let Some(last) = self.scope_stack.last() {
                    if !last.contains_key(name) {
                        self.declare_var(name, kind);
                    }
                }
            }
            AssignmentTarget::ArrayPattern(arr) => {
                for elem in &arr.elements {
                    if let Some(pat) = elem {
                        match pat {
                            PatternElement::Pattern(p) => {
                                self.pre_scan_assignment_target_binding(p, kind)
                            }
                            PatternElement::RestElement(r) => {
                                self.pre_scan_assignment_target_binding(&r.argument, kind)
                            }
                            PatternElement::AssignmentPattern(ap) => {
                                self.pre_scan_assignment_target_binding(&ap.left, kind)
                            }
                        }
                    }
                }
            }
            AssignmentTarget::ObjectPattern(obj) => {
                for prop in &obj.properties {
                    match prop {
                        ObjectPatternProperty::Property { value, .. } => {
                            self.pre_scan_assignment_target_binding(value, kind);
                        }
                        ObjectPatternProperty::RestElement(r) => {
                            self.pre_scan_assignment_target_binding(&r.argument, kind);
                        }
                    }
                }
            }
            AssignmentTarget::AssignmentPattern(ap) => {
                self.pre_scan_assignment_target_binding(&ap.left, kind);
            }
            _ => {}
        }
    }

    fn emit_tdz_for_block(&mut self, body: &[ASTNode]) -> Result<(), String> {
        for node in body {
            if let ASTNode::VariableDeclaration(decl) = node {
                if decl.kind != VariableKind::Var {
                    for d in &decl.declarations {
                        self.emit_tdz_for_binding(&d.id)?;
                    }
                }
            } else if let ASTNode::ClassDeclaration(class) = node {
                if let Some(slot) = self.lookup_var_slot(&class.name) {
                    self.emit(Opcode::LoadTdz);
                    self.emit_u16(slot);
                }
            }
        }
        Ok(())
    }

    fn emit_tdz_for_binding(&mut self, pattern: &BindingPattern) -> Result<(), String> {
        match pattern {
            BindingPattern::Identifier(name) => {
                if let Some(slot) = self.lookup_var_slot(name) {
                    self.emit(Opcode::LoadTdz);
                    self.emit_u16(slot);
                }
            }
            BindingPattern::ArrayPattern(arr) => {
                for elem in &arr.elements {
                    if let Some(pat) = elem {
                        match pat {
                            PatternElement::Pattern(p) => self.emit_tdz_for_assignment_target(p)?,
                            PatternElement::RestElement(r) => {
                                self.emit_tdz_for_assignment_target(&r.argument)?
                            }
                            PatternElement::AssignmentPattern(ap) => {
                                self.emit_tdz_for_assignment_target(&ap.left)?
                            }
                        }
                    }
                }
            }
            BindingPattern::ObjectPattern(obj) => {
                for prop in &obj.properties {
                    match prop {
                        ObjectPatternProperty::Property { value, .. } => {
                            self.emit_tdz_for_assignment_target(value)?;
                        }
                        ObjectPatternProperty::RestElement(r) => {
                            self.emit_tdz_for_assignment_target(&r.argument)?;
                        }
                    }
                }
            }
            BindingPattern::AssignmentPattern(ap) => {
                self.emit_tdz_for_assignment_target(&ap.left)?;
            }
        }
        Ok(())
    }

    fn emit_tdz_for_assignment_target(&mut self, target: &AssignmentTarget) -> Result<(), String> {
        match target {
            AssignmentTarget::Identifier(name) => {
                if let Some(slot) = self.lookup_var_slot(name) {
                    self.emit(Opcode::LoadTdz);
                    self.emit_u16(slot);
                }
            }
            AssignmentTarget::ArrayPattern(arr) => {
                for elem in &arr.elements {
                    if let Some(pat) = elem {
                        match pat {
                            PatternElement::Pattern(p) => self.emit_tdz_for_assignment_target(p)?,
                            PatternElement::RestElement(r) => {
                                self.emit_tdz_for_assignment_target(&r.argument)?
                            }
                            PatternElement::AssignmentPattern(ap) => {
                                self.emit_tdz_for_assignment_target(&ap.left)?
                            }
                        }
                    }
                }
            }
            AssignmentTarget::ObjectPattern(obj) => {
                for prop in &obj.properties {
                    match prop {
                        ObjectPatternProperty::Property { value, .. } => {
                            self.emit_tdz_for_assignment_target(value)?;
                        }
                        ObjectPatternProperty::RestElement(r) => {
                            self.emit_tdz_for_assignment_target(&r.argument)?;
                        }
                    }
                }
            }
            AssignmentTarget::AssignmentPattern(ap) => {
                self.emit_tdz_for_assignment_target(&ap.left)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn gen_statement(&mut self, stmt: &ASTNode, ctx: &mut JSContext) -> Result<(), String> {
        match stmt {
            ASTNode::ExpressionStatement(es) => {
                let r = self.gen_expression(&es.expression, ctx)?;

                self.free_register(r);
            }
            ASTNode::VariableDeclaration(decl) => {
                self.gen_variable_declaration(decl, ctx)?;
            }
            ASTNode::FunctionDeclaration(func) => {
                let slot = self
                    .lookup_var_slot(&func.name)
                    .unwrap_or_else(|| self.declare_var(&func.name, VariableKind::Var));
                let reg = self.gen_function_expression(
                    &func.params,
                    &func.body,
                    Some(&func.name),
                    ctx,
                    func.generator,
                    func.is_async,
                    false,
                )?;
                if slot != reg {
                    self.emit(Opcode::Move);
                    self.emit_u16(slot);
                    self.emit_u16(reg);
                }

                if self.is_global_scope() && !(self.is_strict && self.is_eval) {
                    let atom = ctx.intern(&func.name);
                    let idx = self.add_constant(JSValue::new_string(atom));
                    if self.is_eval {
                        self.emit(Opcode::SetGlobalVar);
                    } else {
                        self.emit(Opcode::SetGlobal);
                    }
                    self.emit_u32(idx);
                    self.emit_u16(slot);
                }
            }
            ASTNode::ClassDeclaration(class) => {
                self.gen_class_declaration(class, ctx)?;
            }
            ASTNode::BlockStatement(block) => {
                self.push_scope();
                self.pre_scan_let_const(&block.body)?;
                self.emit_tdz_for_block(&block.body)?;
                for (i, s) in block.body.iter().enumerate() {
                    if let Some(&line) = block.lines.get(i) {
                        self.emit_line(line);
                    }
                    self.gen_statement(s, ctx)?;
                }
                self.pop_scope();
            }
            ASTNode::IfStatement(stmt) => {
                self.gen_if_statement(stmt, ctx)?;
            }
            ASTNode::WhileStatement(stmt) => {
                self.gen_while_statement(stmt, ctx)?;
            }
            ASTNode::ForStatement(stmt) => {
                self.gen_for_statement(stmt, ctx)?;
            }
            ASTNode::ForInStatement(stmt) => {
                self.gen_for_in_statement(stmt, ctx)?;
            }
            ASTNode::ForOfStatement(stmt) => {
                self.gen_for_of_statement(stmt, ctx)?;
            }
            ASTNode::DoWhileStatement(stmt) => {
                self.gen_do_while_statement(stmt, ctx)?;
            }
            ASTNode::SwitchStatement(stmt) => {
                self.gen_switch_statement(stmt, ctx)?;
            }
            ASTNode::BreakStatement(stmt) => {
                if stmt.label.is_some() {
                    if !self
                        .breakable_frames
                        .iter()
                        .any(|f| f.label.as_deref() == stmt.label.as_deref())
                    {
                        return Err(format!(
                            "SyntaxError: Label '{}' not found",
                            stmt.label.as_deref().unwrap()
                        ));
                    }
                } else if self.breakable_frames.is_empty() {
                    return Err("SyntaxError: Illegal break statement".to_string());
                }
                self.emit(Opcode::Jump);
                let patch = self.code.len();
                self.emit_i32(0);
                if let Some(label) = &stmt.label {
                    if let Some(frame) = self
                        .breakable_frames
                        .iter_mut()
                        .rev()
                        .find(|f| f.label.as_deref() == Some(label))
                    {
                        frame.break_patches.push(patch);
                    }
                } else if let Some(frame) = self.breakable_frames.last_mut() {
                    frame.break_patches.push(patch);
                }
            }
            ASTNode::ContinueStatement(stmt) => {
                if stmt.label.is_some() {
                    let found = self.breakable_frames.iter().any(|f| {
                        f.label.as_deref() == stmt.label.as_deref() && f.continue_target.is_some()
                    });
                    if !found {
                        return Err(format!(
                            "SyntaxError: Continue target '{}' not found",
                            stmt.label.as_deref().unwrap()
                        ));
                    }
                } else if !self
                    .breakable_frames
                    .iter()
                    .any(|f| f.continue_target.is_some())
                {
                    return Err("SyntaxError: Illegal continue statement".to_string());
                }
                self.emit(Opcode::Jump);
                let patch = self.code.len();
                self.emit_i32(0);
                if let Some(label) = &stmt.label {
                    let found =
                        self.breakable_frames.iter_mut().rev().find(|f| {
                            f.label.as_deref() == Some(label) && f.continue_target.is_some()
                        });
                    if let Some(frame) = found {
                        frame.continue_patches.push(patch);
                    }
                } else if let Some(frame) = self
                    .breakable_frames
                    .iter_mut()
                    .rev()
                    .find(|f| f.continue_target.is_some())
                {
                    frame.continue_patches.push(patch);
                }
            }
            ASTNode::ReturnStatement(stmt) => {
                if self.is_script_level {
                    return Err("SyntaxError: Illegal return statement".to_string());
                }
                let return_reg = if let Some(expr) = &stmt.argument {
                    self.gen_expression(expr, ctx)?
                } else {
                    self.emit(Opcode::LoadUndefined);
                    self.emit_u16(0);
                    0
                };
                self.emit(Opcode::Return);
                self.emit_u16(return_reg);
            }
            ASTNode::ThrowStatement(stmt) => {
                self.gen_throw_statement(stmt, ctx)?;
            }
            ASTNode::TryStatement(stmt) => {
                self.gen_try_statement(stmt, ctx, None)?;
            }
            ASTNode::WithStatement(_) => {
                if self.is_strict {
                    return Err(
                        "SyntaxError: Strict mode code may not include a with statement"
                            .to_string(),
                    );
                }
                return Err("with statement is not supported".to_string());
            }
            ASTNode::EmptyStatement => {}
            ASTNode::LabelledStatement(stmt) => {
                let saved = self.pending_label.clone();
                self.pending_label = None;
                self.breakable_frames.push(BreakableFrame {
                    label: Some(stmt.label.clone()),
                    break_patches: Vec::new(),
                    continue_patches: Vec::new(),
                    continue_target: None,
                });
                let saved_label = self.pending_label.clone();
                self.pending_label = Some(stmt.label.clone());
                self.gen_statement(&stmt.body, ctx)?;
                self.pending_label = saved_label;
                let frame = self.breakable_frames.pop().unwrap();
                let end_pos = self.code.len();
                for patch in frame.break_patches {
                    self.patch_jump(patch, end_pos);
                }
                self.pending_label = saved;
            }
            _ => {}
        }
        Ok(())
    }

    fn gen_binding_pattern(
        &mut self,
        pattern: &BindingPattern,
        value_reg: u16,
        kind: VariableKind,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        match pattern {
            BindingPattern::Identifier(name) => {
                if self.is_strict && Self::is_strict_reserved_word(name) {
                    return Err(format!(
                        "SyntaxError: '{}' cannot be declared in strict mode code",
                        name
                    ));
                }
                let slot = if kind == VariableKind::Var {
                    self.declare_var(name, kind)
                } else {
                    self.lookup_var_slot(name)
                        .unwrap_or_else(|| self.declare_var(name, kind))
                };

                for scope in self.scope_stack.iter_mut().rev() {
                    if let Some(entry) = scope.get_mut(name) {
                        entry.initialized = true;
                        break;
                    }
                }
                if slot != value_reg {
                    self.emit(Opcode::Move);
                    self.emit_u16(slot);
                    self.emit_u16(value_reg);
                }

                if kind == VariableKind::Var
                    && self.is_global_scope()
                    && !(self.is_strict && self.is_eval)
                {
                    let atom = ctx.intern(name);
                    let idx = self.add_constant(JSValue::new_string(atom));
                    self.emit(Opcode::SetGlobalVar);
                    self.emit_u32(idx);
                    self.emit_u16(slot);
                }
                Ok(())
            }
            BindingPattern::ArrayPattern(arr) => {
                self.emit(Opcode::CheckObjectCoercible);
                self.emit_u16(value_reg);

                let iter_reg = self.alloc_register();
                self.emit(Opcode::GetIterator);
                self.emit_u16(iter_reg);
                self.emit_u16(value_reg);

                let len = arr.elements.len();
                for (i, elem) in arr.elements.iter().enumerate() {
                    if i == len - 1 {
                        if let Some(PatternElement::RestElement(rest)) = elem {
                            let rest_reg = self.alloc_register();
                            self.emit(Opcode::NewArray);
                            self.emit_u16(rest_reg);
                            self.emit_u16(0);
                            let loop_start = self.code.len();
                            let val_for_rest = self.alloc_register();
                            let done_for_rest = self.alloc_register();
                            self.emit(Opcode::IteratorNext);
                            self.emit_u16(val_for_rest);
                            self.emit_u16(done_for_rest);
                            self.emit_u16(iter_reg);
                            self.emit(Opcode::JumpIf);
                            self.emit_u16(done_for_rest);
                            let exit_patch = self.code.len();
                            self.emit_i32(0);
                            self.free_register(done_for_rest);
                            self.emit(Opcode::ArrayPush);
                            self.emit_u16(rest_reg);
                            self.emit_u16(val_for_rest);
                            self.free_register(val_for_rest);
                            self.emit_backward_jump(loop_start);
                            let end_pos = self.code.len();
                            self.patch_jump(exit_patch, end_pos);
                            self.gen_assignment_target_binding(
                                &rest.argument,
                                rest_reg,
                                kind,
                                ctx,
                            )?;
                            self.free_register(rest_reg);
                            self.free_register(iter_reg);
                            return Ok(());
                        }
                    }

                    let val_reg = self.alloc_register();
                    let done_reg = self.alloc_register();
                    self.emit(Opcode::IteratorNext);
                    self.emit_u16(val_reg);
                    self.emit_u16(done_reg);
                    self.emit_u16(iter_reg);

                    self.emit(Opcode::JumpIfNot);
                    self.emit_u16(done_reg);
                    let not_done_skip = self.code.len();
                    self.emit_i32(0);

                    let undef_reg = self.alloc_register();
                    self.emit(Opcode::LoadUndefined);
                    self.emit_u16(undef_reg);
                    self.emit(Opcode::Move);
                    self.emit_u16(val_reg);
                    self.emit_u16(undef_reg);
                    self.free_register(undef_reg);

                    let after_undef = self.code.len();
                    self.patch_jump(not_done_skip, after_undef);
                    self.free_register(done_reg);

                    if let Some(elem) = elem {
                        match elem {
                            PatternElement::Pattern(target) => {
                                self.gen_assignment_target_binding(target, val_reg, kind, ctx)?;
                            }
                            PatternElement::RestElement(_) => {
                                return Err("rest element not at end".to_string());
                            }
                            PatternElement::AssignmentPattern(ap) => {
                                let binding_name = match &*ap.left {
                                    AssignmentTarget::Identifier(name) => Some(name.as_str()),
                                    _ => None,
                                };
                                self.gen_default_value_binding(
                                    &ap.right,
                                    val_reg,
                                    binding_name,
                                    ctx,
                                )?;
                                self.gen_assignment_target_binding(&ap.left, val_reg, kind, ctx)?;
                            }
                        }
                    }
                    self.free_register(val_reg);
                }
                self.free_register(iter_reg);
                Ok(())
            }
            BindingPattern::ObjectPattern(obj) => {
                self.emit(Opcode::CheckObjectCoercible);
                self.emit_u16(value_reg);
                for prop in &obj.properties {
                    match prop {
                        ObjectPatternProperty::Property {
                            key,
                            value,
                            computed,
                            ..
                        } => {
                            let key_reg = if *computed {
                                match key {
                                    PropertyKey::Computed(expr) => {
                                        self.gen_expression(expr, ctx)?
                                    }
                                    _ => return Err("Expected computed key".to_string()),
                                }
                            } else {
                                match key {
                                    PropertyKey::Identifier(k)
                                    | PropertyKey::Literal(Literal::String(k, _)) => {
                                        let idx = self.intern_and_add_const(ctx, k);
                                        let kreg = self.alloc_register();
                                        self.emit(Opcode::LoadConst);
                                        self.emit_u16(kreg);
                                        self.emit_u32(idx);
                                        kreg
                                    }
                                    PropertyKey::Literal(Literal::Number(n)) => {
                                        let kreg = self.alloc_register();
                                        if *n >= i32::MIN as f64
                                            && *n <= i32::MAX as f64
                                            && n.fract() == 0.0
                                        {
                                            self.emit_int_const(kreg, *n as i32);
                                        } else {
                                            let cidx = self.add_constant(JSValue::new_float(*n));
                                            self.emit(Opcode::LoadConst);
                                            self.emit_u16(kreg);
                                            self.emit_u32(cidx);
                                        }
                                        kreg
                                    }
                                    _ => return Err("Invalid property key".to_string()),
                                }
                            };
                            let elem_reg = self.alloc_register();
                            self.emit(Opcode::GetProp);
                            self.emit_u16(elem_reg);
                            self.emit_u16(value_reg);
                            self.emit_u16(key_reg);
                            self.free_register(key_reg);
                            self.gen_assignment_target_binding(value, elem_reg, kind, ctx)?;
                            self.free_register(elem_reg);
                        }
                        ObjectPatternProperty::RestElement(rest) => {
                            let rest_reg = self.alloc_register();
                            self.emit(Opcode::NewObject);
                            self.emit_u16(rest_reg);
                            self.emit(Opcode::ObjectSpread);
                            self.emit_u16(rest_reg);
                            self.emit_u16(value_reg);
                            self.gen_assignment_target_binding(
                                &rest.argument,
                                rest_reg,
                                kind,
                                ctx,
                            )?;
                            self.free_register(rest_reg);
                        }
                    }
                }
                Ok(())
            }
            BindingPattern::AssignmentPattern(ap) => {
                let binding_name = match &*ap.left {
                    AssignmentTarget::Identifier(name) => Some(name.as_str()),
                    _ => None,
                };
                self.gen_default_value_binding(&ap.right, value_reg, binding_name, ctx)?;
                self.gen_assignment_target_binding(&ap.left, value_reg, kind, ctx)
            }
        }
    }

    fn gen_assignment_target_binding(
        &mut self,
        target: &AssignmentTarget,
        value_reg: u16,
        kind: VariableKind,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        match target {
            AssignmentTarget::Identifier(name) => {
                if self.is_strict && Self::is_strict_reserved_word(name) {
                    return Err(format!(
                        "SyntaxError: '{}' cannot be declared in strict mode code",
                        name
                    ));
                }
                let slot = if kind == VariableKind::Var {
                    self.declare_var(name, kind)
                } else {
                    self.lookup_var_slot(name)
                        .unwrap_or_else(|| self.declare_var(name, kind))
                };
                if slot != value_reg {
                    self.emit(Opcode::Move);
                    self.emit_u16(slot);
                    self.emit_u16(value_reg);
                }
                if kind == VariableKind::Var
                    && self.is_global_scope()
                    && !(self.is_strict && self.is_eval)
                {
                    let atom = ctx.intern(name);
                    let idx = self.add_constant(JSValue::new_string(atom));
                    self.emit(Opcode::SetGlobal);
                    self.emit_u32(idx);
                    self.emit_u16(slot);
                }
                Ok(())
            }
            AssignmentTarget::ArrayPattern(arr) => self.gen_binding_pattern(
                &BindingPattern::ArrayPattern(arr.clone()),
                value_reg,
                kind,
                ctx,
            ),
            AssignmentTarget::ObjectPattern(obj) => self.gen_binding_pattern(
                &BindingPattern::ObjectPattern(obj.clone()),
                value_reg,
                kind,
                ctx,
            ),
            AssignmentTarget::AssignmentPattern(ap) => {
                let binding_name = match &*ap.left {
                    AssignmentTarget::Identifier(name) => Some(name.as_str()),
                    _ => None,
                };
                self.gen_default_value_binding(&ap.right, value_reg, binding_name, ctx)?;
                self.gen_assignment_target_binding(&ap.left, value_reg, kind, ctx)
            }
            _ => Err("Unsupported target in binding pattern".to_string()),
        }
    }

    fn gen_default_value_binding(
        &mut self,
        default_expr: &Expression,
        value_reg: u16,
        binding_name: Option<&str>,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        let cmp_reg = self.alloc_register();
        self.emit(Opcode::LoadUndefined);
        self.emit_u16(cmp_reg);

        self.emit(Opcode::StrictNeq);
        self.emit_u16(cmp_reg);
        self.emit_u16(value_reg);
        self.emit_u16(cmp_reg);

        self.emit(Opcode::JumpIf);
        self.emit_u16(cmp_reg);
        let skip_patch = self.code.len();
        self.emit_i32(0);
        self.free_register(cmp_reg);

        let default_reg = if let Some(name) = binding_name {
            self.gen_expression_with_assign_name(default_expr, name, ctx)?
        } else {
            self.gen_expression(default_expr, ctx)?
        };
        if default_reg != value_reg {
            self.emit(Opcode::Move);
            self.emit_u16(value_reg);
            self.emit_u16(default_reg);
        }
        self.free_register(default_reg);

        let after_pos = self.code.len();
        self.patch_jump(skip_patch, after_pos);

        Ok(())
    }

    fn predeclare_binding_pattern(&mut self, pattern: &BindingPattern, kind: VariableKind) {
        match pattern {
            BindingPattern::Identifier(name) => {
                let _ = self.declare_var(name, kind);
            }
            BindingPattern::ArrayPattern(arr) => {
                for elem in &arr.elements {
                    if let Some(elem) = elem {
                        match elem {
                            PatternElement::Pattern(AssignmentTarget::Identifier(name)) => {
                                let _ = self.declare_var(name, kind);
                            }
                            PatternElement::Pattern(AssignmentTarget::ArrayPattern(arr)) => {
                                self.predeclare_binding_pattern(
                                    &BindingPattern::ArrayPattern(arr.clone()),
                                    kind,
                                );
                            }
                            PatternElement::Pattern(AssignmentTarget::ObjectPattern(obj)) => {
                                self.predeclare_binding_pattern(
                                    &BindingPattern::ObjectPattern(obj.clone()),
                                    kind,
                                );
                            }
                            PatternElement::Pattern(AssignmentTarget::AssignmentPattern(ap)) => {
                                self.predeclare_binding_pattern(
                                    &BindingPattern::AssignmentPattern(ap.clone()),
                                    kind,
                                );
                            }
                            PatternElement::RestElement(rest) => {
                                self.predeclare_assignment_target(&rest.argument, kind);
                            }
                            PatternElement::AssignmentPattern(ap) => {
                                self.predeclare_assignment_target(&ap.left, kind);
                            }
                            _ => {}
                        }
                    }
                }
            }
            BindingPattern::ObjectPattern(obj) => {
                for prop in &obj.properties {
                    match prop {
                        ObjectPatternProperty::Property { value, .. } => {
                            self.predeclare_assignment_target(value, kind);
                        }
                        ObjectPatternProperty::RestElement(rest) => {
                            self.predeclare_assignment_target(&rest.argument, kind);
                        }
                    }
                }
            }
            BindingPattern::AssignmentPattern(ap) => {
                self.predeclare_assignment_target(&ap.left, kind);
            }
        }
    }

    fn predeclare_assignment_target(&mut self, target: &AssignmentTarget, kind: VariableKind) {
        match target {
            AssignmentTarget::Identifier(name) => {
                let _ = self.declare_var(name, kind);
            }
            AssignmentTarget::ArrayPattern(arr) => {
                self.predeclare_binding_pattern(&BindingPattern::ArrayPattern(arr.clone()), kind);
            }
            AssignmentTarget::ObjectPattern(obj) => {
                self.predeclare_binding_pattern(&BindingPattern::ObjectPattern(obj.clone()), kind);
            }
            AssignmentTarget::AssignmentPattern(ap) => {
                self.predeclare_assignment_target(&ap.left, kind);
            }
            AssignmentTarget::RestElement(rest) => {
                self.predeclare_assignment_target(&rest.argument, kind);
            }
            _ => {}
        }
    }

    fn lookup_var_slot_for_pattern(&self, pattern: &BindingPattern) -> Option<u16> {
        match pattern {
            BindingPattern::Identifier(name) => self.lookup_var_slot(name),
            _ => None,
        }
    }

    fn gen_variable_declaration(
        &mut self,
        decl: &VariableDeclaration,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        for d in &decl.declarations {
            if decl.kind == VariableKind::Const && d.init.is_none() {
                return Err("SyntaxError: Missing initializer in const declaration".to_string());
            }

            if let Some(init) = &d.init {
                let reg = self.gen_expression_with_name(init, &d.id, ctx)?;
                self.gen_binding_pattern(&d.id, reg, decl.kind, ctx)?;
                self.free_register(reg);
            } else {
                if decl.kind == VariableKind::Var {
                    self.predeclare_binding_pattern(&d.id, VariableKind::Var);
                    if self.is_global_scope() && !(self.is_strict && self.is_eval) {
                        if let BindingPattern::Identifier(name) = &d.id {
                            let slot = self.lookup_var_slot(name).unwrap();
                            let atom = ctx.intern(name);
                            let idx = self.add_constant(JSValue::new_string(atom));
                            if self.is_eval {
                                self.emit(Opcode::InitGlobalVar);
                                self.emit_u32(idx);
                                self.emit_u16(slot);
                            } else {
                                self.emit(Opcode::DefineGlobal);
                                self.emit_u32(idx);
                                self.emit_u16(slot);
                            }
                        }
                    }
                } else {
                    if let Some(slot) = self.lookup_var_slot_for_pattern(&d.id) {
                        self.emit(Opcode::LoadUndefined);
                        self.emit_u16(slot);
                    }

                    if let BindingPattern::Identifier(name) = &d.id {
                        for scope in self.scope_stack.iter_mut().rev() {
                            if let Some(entry) = scope.get_mut(name) {
                                entry.initialized = true;
                                break;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn emit_load_undefined(&mut self) -> u16 {
        let r = self.alloc_register();
        self.emit(Opcode::LoadUndefined);
        self.emit_u16(r);
        r
    }

    fn unwrap_completion(&mut self, val: Option<u16>) -> u16 {
        match val {
            Some(r) => r,
            None => self.emit_load_undefined(),
        }
    }

    fn gen_statement_as_expression(
        &mut self,
        stmt: &ASTNode,
        ctx: &mut JSContext,
    ) -> Result<Option<u16>, String> {
        match stmt {
            ASTNode::ExpressionStatement(es) => {
                Ok(Some(self.gen_expression(&es.expression, ctx)?))
            }
            ASTNode::IfStatement(if_stmt) => {
                Ok(Some(self.gen_if_statement_as_expression(if_stmt, ctx)?))
            }
            ASTNode::BlockStatement(block) => {
                Ok(Some(self.gen_block_statement_as_expression(block, ctx)?))
            }
            ASTNode::TryStatement(try_stmt) => {
                Ok(Some(self.gen_try_statement_as_expression(try_stmt, ctx)?))
            }
            ASTNode::SwitchStatement(switch_stmt) => {
                Ok(Some(self.gen_switch_statement_as_expression(switch_stmt, ctx)?))
            }
            ASTNode::VariableDeclaration(_)
            | ASTNode::FunctionDeclaration(_)
            | ASTNode::ClassDeclaration(_) => {
                self.gen_statement(stmt, ctx)?;
                Ok(None)
            }
            ASTNode::ForStatement(f_stmt) => {
                let reg = self.alloc_register();
                self.emit(Opcode::LoadUndefined);
                self.emit_u16(reg);
                self.gen_for_statement_with_result(f_stmt, ctx, Some(reg))?;
                Ok(Some(reg))
            }
            ASTNode::ForInStatement(fi_stmt) => {
                let reg = self.alloc_register();
                self.emit(Opcode::LoadUndefined);
                self.emit_u16(reg);
                self.gen_for_in_statement_with_result(fi_stmt, ctx, Some(reg))?;
                Ok(Some(reg))
            }
            ASTNode::ForOfStatement(fo_stmt) => {
                let reg = self.alloc_register();
                self.emit(Opcode::LoadUndefined);
                self.emit_u16(reg);
                self.gen_for_of_statement_with_result(fo_stmt, ctx, Some(reg))?;
                Ok(Some(reg))
            }
            ASTNode::WhileStatement(w_stmt) => {
                let reg = self.alloc_register();
                self.emit(Opcode::LoadUndefined);
                self.emit_u16(reg);
                self.gen_while_statement_with_result(w_stmt, ctx, Some(reg))?;
                Ok(Some(reg))
            }
            ASTNode::DoWhileStatement(dw_stmt) => {
                let reg = self.alloc_register();
                self.emit(Opcode::LoadUndefined);
                self.emit_u16(reg);
                self.gen_do_while_statement_with_result(dw_stmt, ctx, Some(reg))?;
                Ok(Some(reg))
            }
            ASTNode::ReturnStatement(_) | ASTNode::ThrowStatement(_) | ASTNode::BreakStatement(_) | ASTNode::ContinueStatement(_) => {
                self.gen_statement(stmt, ctx)?;
                Ok(None)
            }
            ASTNode::LabelledStatement(labeled) => {
                self.pending_label = Some(labeled.label.clone());
                self.breakable_frames.push(BreakableFrame {
                    label: Some(labeled.label.clone()),
                    break_patches: Vec::new(),
                    continue_patches: Vec::new(),
                    continue_target: None,
                });
                let result = self.gen_statement_as_expression(&labeled.body, ctx);
                let frame = self.breakable_frames.pop().unwrap();
                let end_pos = self.code.len();
                for patch in frame.break_patches {
                    self.patch_jump(patch, end_pos);
                }
                for patch in frame.continue_patches {
                    if let Some(ct) = frame.continue_target {
                        self.patch_jump(patch, ct);
                    }
                }
                self.pending_label = None;
                result
            }
            ASTNode::WithStatement(_) | ASTNode::EmptyStatement | ASTNode::DebuggerStatement => Ok(None),
            _ => Ok(None),
        }
    }

    fn gen_block_statement_as_expression(
        &mut self,
        block: &BlockStatement,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        let result_reg = self.alloc_register();
        self.emit(Opcode::LoadUndefined);
        self.emit_u16(result_reg);
        self.gen_block_with_target(block, ctx, result_reg)?;
        Ok(result_reg)
    }

    fn gen_block_with_target(
        &mut self,
        block: &BlockStatement,
        ctx: &mut JSContext,
        target_reg: u16,
    ) -> Result<(), String> {
        self.push_scope();
        self.pre_scan_let_const(&block.body)?;
        self.emit_tdz_for_block(&block.body)?;

        for (i, s) in block.body.iter().enumerate() {
            if let Some(&line) = block.lines.get(i) {
                self.emit_line(line);
            }
            self.gen_statement_with_target(s, ctx, target_reg)?;
        }
        self.pop_scope();
        Ok(())
    }

    fn gen_statement_with_target(
        &mut self,
        stmt: &ASTNode,
        ctx: &mut JSContext,
        target_reg: u16,
    ) -> Result<(), String> {
        match stmt {
            ASTNode::BlockStatement(block) => {
                self.push_scope();
                self.pre_scan_let_const(&block.body)?;
                self.emit_tdz_for_block(&block.body)?;
                for (i, s) in block.body.iter().enumerate() {
                    if let Some(&line) = block.lines.get(i) {
                        self.emit_line(line);
                    }
                    self.gen_statement_with_target(s, ctx, target_reg)?;
                }
                self.pop_scope();
            }
            ASTNode::ExpressionStatement(es) => {
                let reg = self.gen_expression(&es.expression, ctx)?;
                if reg != target_reg {
                    self.emit(Opcode::Move);
                    self.emit_u16(target_reg);
                    self.emit_u16(reg);
                }
                self.free_register(reg);
            }
            ASTNode::IfStatement(if_stmt) => {
                self.gen_if_statement_with_target(if_stmt, ctx, target_reg)?;
            }
            ASTNode::BreakStatement(_) | ASTNode::ContinueStatement(_) => {
                self.gen_statement(stmt, ctx)?;
            }
            ASTNode::ReturnStatement(ret) => {
                if let Some(ref arg) = ret.argument {
                    let reg = self.gen_expression(arg, ctx)?;
                    self.emit(Opcode::Return);
                    self.emit_u16(reg);
                } else {
                    self.emit(Opcode::LoadUndefined);
                    self.emit_u16(0);
                    self.emit(Opcode::Return);
                    self.emit_u16(0);
                }
            }
            ASTNode::ThrowStatement(throw) => {
                let reg = self.gen_expression(&throw.argument, ctx)?;
                self.emit(Opcode::Throw);
                self.emit_u16(reg);
            }
            ASTNode::WhileStatement(w_stmt) => {
                self.gen_while_statement_with_result(w_stmt, ctx, Some(target_reg))?;
            }
            ASTNode::DoWhileStatement(dw_stmt) => {
                self.gen_do_while_statement_with_result(dw_stmt, ctx, Some(target_reg))?;
            }
            ASTNode::SwitchStatement(sw_stmt) => {
                self.emit(Opcode::LoadUndefined);
                self.emit_u16(target_reg);
                self.gen_switch_statement_with_result(sw_stmt, ctx, Some(target_reg))?;
            }
            _ => {
                // Variable declarations, function declarations, etc.
                if let Some(reg) = self.gen_statement_as_expression(stmt, ctx)? {
                    if reg != target_reg {
                        self.emit(Opcode::Move);
                        self.emit_u16(target_reg);
                        self.emit_u16(reg);
                        self.free_register(reg);
                    }
                } else {
                    self.gen_statement(stmt, ctx)?;
                }
            }
        }
        Ok(())
    }

    fn gen_if_statement_as_expression(
        &mut self,
        stmt: &IfStatement,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        if self.p2_fold_profile.allow_if_bool() {
            if let Expression::Literal(Literal::Boolean(test_bool)) = &stmt.test {
                if *test_bool {
                    return self.gen_statement_as_expression(&stmt.consequent, ctx).map(|r| self.unwrap_completion(r));
                } else if let Some(alt) = &stmt.alternate {
                    return self.gen_statement_as_expression(alt, ctx).map(|r| self.unwrap_completion(r));
                } else {
                    return Ok(self.emit_load_undefined());
                }
            }
        }

        let test_reg = self.gen_expression(&stmt.test, ctx)?;

        if self.opt_branch_result_prealloc {
            let result = if self.reserved_slots.contains(&test_reg) {
                let r = self.alloc_register();
                self.emit(Opcode::Move);
                self.emit_u16(r);
                self.emit_u16(test_reg);
                r
            } else {
                test_reg
            };

            self.emit(Opcode::JumpIfNot);
            self.emit_u16(test_reg);
            let else_jump = self.code.len();
            self.emit_i32(0);

            let cons_opt = self.gen_statement_as_expression(&stmt.consequent, ctx)?;
            let cons = self.unwrap_completion(cons_opt);
            if cons != result {
                self.emit(Opcode::Move);
                self.emit_u16(result);
                self.emit_u16(cons);
                self.free_register(cons);
            }

            self.emit(Opcode::Jump);
            let end_jump = self.code.len();
            self.emit_i32(0);

            let else_pos = self.code.len();
            self.patch_jump(else_jump, else_pos);

            let alt = if let Some(alt_stmt) = &stmt.alternate {
                let alt_opt = self.gen_statement_as_expression(alt_stmt, ctx)?;
                self.unwrap_completion(alt_opt)
            } else {
                self.emit_load_undefined()
            };
            if alt != result {
                self.emit(Opcode::Move);
                self.emit_u16(result);
                self.emit_u16(alt);
                self.free_register(alt);
            }

            let end_pos = self.code.len();
            self.patch_jump(end_jump, end_pos);
            return Ok(result);
        }

        self.emit(Opcode::JumpIfNot);
        self.emit_u16(test_reg);
        let else_jump = self.code.len();
        self.emit_i32(0);

        let cons_opt = self.gen_statement_as_expression(&stmt.consequent, ctx)?;
        let cons = self.unwrap_completion(cons_opt);

        let result = if self.reserved_slots.contains(&test_reg) && cons != test_reg {
            let r = self.alloc_register();
            self.emit(Opcode::Move);
            self.emit_u16(r);
            self.emit_u16(cons);
            self.free_register(cons);
            r
        } else if cons != test_reg {
            self.emit(Opcode::Move);
            self.emit_u16(test_reg);
            self.emit_u16(cons);
            self.free_register(cons);
            test_reg
        } else {
            test_reg
        };

        self.emit(Opcode::Jump);
        let end_jump = self.code.len();
        self.emit_i32(0);

        let else_pos = self.code.len();
        self.patch_jump(else_jump, else_pos);

        let alt = if let Some(alt_stmt) = &stmt.alternate {
            let alt_opt = self.gen_statement_as_expression(alt_stmt, ctx)?;
            self.unwrap_completion(alt_opt)
        } else {
            self.emit_load_undefined()
        };
        if self.reserved_slots.contains(&test_reg) && alt != result && alt != test_reg {
            self.emit(Opcode::Move);
            self.emit_u16(result);
            self.emit_u16(alt);
            self.free_register(alt);
        } else if alt != result && alt != test_reg {
            self.emit(Opcode::Move);
            self.emit_u16(result);
            self.emit_u16(alt);
            self.free_register(alt);
        }

        let end_pos = self.code.len();
        self.patch_jump(end_jump, end_pos);

        Ok(result)
    }

    fn gen_if_statement_with_target(
        &mut self,
        stmt: &IfStatement,
        ctx: &mut JSContext,
        target_reg: u16,
    ) -> Result<(), String> {
        if self.p2_fold_profile.allow_if_bool() {
            if let Expression::Literal(Literal::Boolean(test_bool)) = &stmt.test {
                if *test_bool {
                    self.emit(Opcode::LoadUndefined);
                    self.emit_u16(target_reg);
                    self.gen_statement_with_target(&stmt.consequent, ctx, target_reg)?;
                } else if let Some(alt) = &stmt.alternate {
                    self.emit(Opcode::LoadUndefined);
                    self.emit_u16(target_reg);
                    self.gen_statement_with_target(alt, ctx, target_reg)?;
                }
                return Ok(());
            }
        }
        if self.p2_fold_profile.allow_aggressive_truthy() {
            if let Expression::Literal(lit) = &stmt.test {
                if let Some(truthy) = Self::literal_truthiness(lit) {
                    if truthy {
                        self.emit(Opcode::LoadUndefined);
                        self.emit_u16(target_reg);
                        self.gen_statement_with_target(&stmt.consequent, ctx, target_reg)?;
                    } else if let Some(alt) = &stmt.alternate {
                        self.emit(Opcode::LoadUndefined);
                        self.emit_u16(target_reg);
                        self.gen_statement_with_target(alt, ctx, target_reg)?;
                    }
                    return Ok(());
                }
            }
        }
        let test_reg = self.gen_expression(&stmt.test, ctx)?;
        self.emit(Opcode::JumpIfNot);
        self.emit_u16(test_reg);
        self.free_register(test_reg);
        let else_jump = self.code.len();
        self.emit_i32(0);

        self.emit(Opcode::LoadUndefined);
        self.emit_u16(target_reg);
        self.gen_statement_with_target(&stmt.consequent, ctx, target_reg)?;

        let has_else = stmt.alternate.is_some();
        let end_jump = if has_else {
            self.emit(Opcode::Jump);
            let pos = self.code.len();
            self.emit_i32(0);
            Some(pos)
        } else {
            None
        };

        let else_pos = self.code.len();
        self.patch_jump(else_jump, else_pos);

        if let Some(alt) = &stmt.alternate {
            self.emit(Opcode::LoadUndefined);
            self.emit_u16(target_reg);
            self.gen_statement_with_target(alt, ctx, target_reg)?;
        }

        if let Some(end_jump_pos) = end_jump {
            let end_pos = self.code.len();
            self.patch_jump(end_jump_pos, end_pos);
        }
        Ok(())
    }

    fn gen_try_statement_as_expression(
        &mut self,
        stmt: &TryStatement,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        let reg = self.alloc_register();
        self.emit(Opcode::LoadUndefined);
        self.emit_u16(reg);
        self.gen_try_statement(stmt, ctx, Some(reg))?;
        Ok(reg)
    }

    fn gen_switch_statement_as_expression(
        &mut self,
        stmt: &SwitchStatement,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        let reg = self.alloc_register();
        self.emit(Opcode::LoadUndefined);
        self.emit_u16(reg);
        self.gen_switch_statement_with_result(stmt, ctx, Some(reg))?;
        Ok(reg)
    }

    fn gen_if_statement(&mut self, stmt: &IfStatement, ctx: &mut JSContext) -> Result<(), String> {
        if self.p2_fold_profile.allow_if_bool() {
            if let Expression::Literal(Literal::Boolean(test_bool)) = &stmt.test {
                if *test_bool {
                    self.gen_statement(&stmt.consequent, ctx)?;
                } else if let Some(alt) = &stmt.alternate {
                    self.gen_statement(alt, ctx)?;
                }
                return Ok(());
            }
        }

        if self.p2_fold_profile.allow_aggressive_truthy() {
            if let Expression::Literal(lit) = &stmt.test {
                if let Some(truthy) = Self::literal_truthiness(lit) {
                    if truthy {
                        self.gen_statement(&stmt.consequent, ctx)?;
                    } else if let Some(alt) = &stmt.alternate {
                        self.gen_statement(alt, ctx)?;
                    }
                    return Ok(());
                }
            }
        }

        let test_reg = self.gen_expression(&stmt.test, ctx)?;
        self.emit(Opcode::JumpIfNot);
        self.emit_u16(test_reg);
        self.free_register(test_reg);
        let else_jump = self.code.len();
        self.emit_i32(0);

        self.gen_statement(&stmt.consequent, ctx)?;

        let has_else = stmt.alternate.is_some();
        let end_jump = if has_else {
            self.emit(Opcode::Jump);
            let pos = self.code.len();
            self.emit_i32(0);
            Some(pos)
        } else {
            None
        };

        let else_pos = self.code.len();
        self.patch_jump(else_jump, else_pos);

        if let Some(alt) = &stmt.alternate {
            self.gen_statement(alt, ctx)?;
        }

        if let Some(end_jump_pos) = end_jump {
            let end_pos = self.code.len();
            self.patch_jump(end_jump_pos, end_pos);
        }

        Ok(())
    }

    fn gen_while_statement(
        &mut self,
        stmt: &WhileStatement,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        self.gen_while_statement_with_result(stmt, ctx, Some(0))
    }

    fn gen_while_statement_with_result(
        &mut self,
        stmt: &WhileStatement,
        ctx: &mut JSContext,
        result_reg: Option<u16>,
    ) -> Result<(), String> {
        let loop_start = self.code.len();

        let break_jump = if self.opt_fused_cmp_jump {
            if let Expression::BinaryExpression(bin) = &stmt.test {
                if let Some(fused) = Self::fused_op_from_binary_cmp(&bin.op, false) {
                    let left = self.gen_expression(&bin.left, ctx)?;
                    let right = self.gen_expression(&bin.right, ctx)?;
                    self.emit(fused);
                    self.emit_u16(left);
                    self.emit_u16(right);
                    self.free_register(left);
                    self.free_register(right);
                    let p = self.code.len();
                    self.emit_i32(0);
                    p
                } else {
                    let test_reg = self.gen_expression(&stmt.test, ctx)?;
                    self.emit(Opcode::JumpIfNot);
                    self.emit_u16(test_reg);
                    self.free_register(test_reg);
                    let p = self.code.len();
                    self.emit_i32(0);
                    p
                }
            } else {
                let test_reg = self.gen_expression(&stmt.test, ctx)?;
                self.emit(Opcode::JumpIfNot);
                self.emit_u16(test_reg);
                self.free_register(test_reg);
                let p = self.code.len();
                self.emit_i32(0);
                p
            }
        } else {
            let test_reg = self.gen_expression(&stmt.test, ctx)?;
            self.emit(Opcode::JumpIfNot);
            self.emit_u16(test_reg);
            self.free_register(test_reg);
            let p = self.code.len();
            self.emit_i32(0);
            p
        };

        self.breakable_frames.push(BreakableFrame {
            label: None,
            break_patches: vec![break_jump],
            continue_patches: Vec::new(),
            continue_target: Some(loop_start),
        });

        if let Some(v_reg) = result_reg {
            self.emit(Opcode::LoadUndefined);
            self.emit_u16(v_reg);
            self.gen_statement_with_target(&stmt.body, ctx, v_reg)?;
        } else {
            self.gen_statement(&stmt.body, ctx)?;
        }

        let frame = self.breakable_frames.pop().unwrap();

        self.emit_backward_jump(loop_start);

        let end_pos = self.code.len();
        for patch in frame.break_patches {
            self.patch_jump(patch, end_pos);
        }
        for patch in frame.continue_patches {
            self.patch_jump(patch, frame.continue_target.unwrap());
        }

        Ok(())
    }

    fn gen_for_statement(
        &mut self,
        stmt: &ForStatement,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        self.gen_for_statement_with_result(stmt, ctx, Some(0))
    }

    fn gen_for_statement_with_result(
        &mut self,
        stmt: &ForStatement,
        ctx: &mut JSContext,
        result_reg: Option<u16>,
    ) -> Result<(), String> {
        let needs_block_scope = if let Some(ForInit::VariableDeclaration(decl)) = &stmt.init {
            decl.kind != VariableKind::Var
        } else {
            false
        };

        if needs_block_scope {
            self.push_scope();
            if let Some(ForInit::VariableDeclaration(decl)) = &stmt.init {
                for d in &decl.declarations {
                    self.pre_scan_binding(&d.id, decl.kind);
                }
                for d in &decl.declarations {
                    self.emit_tdz_for_binding(&d.id)?;
                }
            }
        }

        if let Some(init) = &stmt.init {
            match init {
                ForInit::VariableDeclaration(decl) => {
                    self.gen_variable_declaration(decl, ctx)?;
                }
                ForInit::Expression(expr) => {
                    let r = self.gen_expression(expr, ctx)?;
                    self.free_register(r);
                }
            }
        }

        let loop_start = self.code.len();

        let break_jump = if let Some(test) = &stmt.test {
            if self.opt_fused_cmp_jump {
                if let Expression::BinaryExpression(bin) = test {
                    if let Some(fused) = Self::fused_op_from_binary_cmp(&bin.op, false) {
                        let left = self.gen_expression(&bin.left, ctx)?;
                        let right = self.gen_expression(&bin.right, ctx)?;
                        self.emit(fused);
                        self.emit_u16(left);
                        self.emit_u16(right);
                        self.free_register(left);
                        self.free_register(right);
                        let bj = self.code.len();
                        self.emit_i32(0);
                        bj
                    } else {
                        let test_reg = self.gen_expression(test, ctx)?;
                        self.emit(Opcode::JumpIfNot);
                        self.emit_u16(test_reg);
                        self.free_register(test_reg);
                        let bj = self.code.len();
                        self.emit_i32(0);
                        bj
                    }
                } else {
                    let test_reg = self.gen_expression(test, ctx)?;
                    self.emit(Opcode::JumpIfNot);
                    self.emit_u16(test_reg);
                    self.free_register(test_reg);
                    let bj = self.code.len();
                    self.emit_i32(0);
                    bj
                }
            } else {
                let test_reg = self.gen_expression(test, ctx)?;
                self.emit(Opcode::JumpIfNot);
                self.emit_u16(test_reg);
                self.free_register(test_reg);
                let bj = self.code.len();
                self.emit_i32(0);
                bj
            }
        } else {
            0
        };

        let loop_start_for_continue = loop_start;
        self.breakable_frames.push(BreakableFrame {
            label: self.pending_label.take(),
            break_patches: if stmt.test.is_some() {
                vec![break_jump]
            } else {
                Vec::new()
            },
            continue_patches: Vec::new(),
            continue_target: Some(loop_start_for_continue),
        });

        if let Some(v_reg) = result_reg {
            self.gen_statement_with_target(&stmt.body, ctx, v_reg)?;
        } else {
            self.gen_statement(&stmt.body, ctx)?;
        }

        let update_pos = self.code.len();
        if let Some(update) = &stmt.update {
            let r = self.gen_expression(update, ctx)?;
            self.free_register(r);
        }
        if let Some(frame) = self.breakable_frames.last_mut() {
            frame.continue_target = Some(update_pos);
        }

        let frame = self.breakable_frames.pop().unwrap();

        self.emit_backward_jump(loop_start);

        let end_pos = self.code.len();
        for patch in frame.break_patches {
            self.patch_jump(patch, end_pos);
        }
        for patch in frame.continue_patches {
            self.patch_jump(patch, frame.continue_target.unwrap());
        }

        if needs_block_scope {
            self.pop_scope();
        }

        Ok(())
    }

    fn gen_for_in_statement(
        &mut self,
        stmt: &ForInStatement,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        self.gen_for_in_statement_with_result(stmt, ctx, Some(0))
    }

    fn gen_for_in_statement_with_result(
        &mut self,
        stmt: &ForInStatement,
        ctx: &mut JSContext,
        result_reg: Option<u16>,
    ) -> Result<(), String> {
        let needs_block_scope = match &stmt.left {
            ForInOfLeft::VariableDeclaration(decl) => decl.kind != VariableKind::Var,
            _ => false,
        };

        if needs_block_scope {
            self.push_scope();
            if let ForInOfLeft::VariableDeclaration(decl) = &stmt.left {
                if let Some(first) = decl.declarations.first() {
                    self.pre_scan_binding(&first.id, decl.kind);
                    self.emit_tdz_for_binding(&first.id)?;
                }
            }
        }

        let has_complex_left = match &stmt.left {
            ForInOfLeft::VariableDeclaration(decl) => decl
                .declarations
                .first()
                .map_or(false, |d| !matches!(d.id, BindingPattern::Identifier(_))),
            ForInOfLeft::AssignmentTarget(at) => !matches!(at, AssignmentTarget::Identifier(_)),
        };

        let var_slot = if has_complex_left {
            let tmp_slot = self.alloc_register();
            self.free_register(tmp_slot);
            tmp_slot
        } else {
            match &stmt.left {
                ForInOfLeft::VariableDeclaration(decl) => {
                    if let Some(first) = decl.declarations.first() {
                        if let BindingPattern::Identifier(name) = &first.id {
                            self.lookup_var_slot(name).unwrap_or(u16::MAX)
                        } else {
                            u16::MAX
                        }
                    } else {
                        u16::MAX
                    }
                }
                ForInOfLeft::AssignmentTarget(AssignmentTarget::Identifier(name)) => {
                    if let Some(slot) = self.lookup_var_slot(name) {
                        slot
                    } else if !self.is_strict {
                        self.declare_var(name, VariableKind::Var)
                    } else {
                        return Err(format!("for-in: undefined variable {}", name));
                    }
                }
                _ => 255,
            }
        };

        let obj_reg = self.gen_expression(&stmt.right, ctx)?;

        let keys_reg = self.alloc_register();
        self.emit(Opcode::GetPropertyNames);
        self.emit_u16(keys_reg);
        self.emit_u16(obj_reg);
        self.free_register(obj_reg);

        let idx_reg = self.alloc_register();
        self.emit_int_const(idx_reg, 0);

        let loop_start = self.code.len();

        let len_key_reg = self.alloc_register();
        let len_atom = ctx.common_atoms.length;
        let len_const = self.add_constant(JSValue::new_string(len_atom));
        self.emit(Opcode::LoadConst);
        self.emit_u16(len_key_reg);
        self.emit_u32(len_const);
        let len_val_reg = self.alloc_register();
        self.emit(Opcode::GetProp);
        self.emit_u16(len_val_reg);
        self.emit_u16(keys_reg);
        self.emit_u16(len_key_reg);
        self.free_register(len_key_reg);

        let cmp_reg = self.alloc_register();
        self.emit(Opcode::Lt);
        self.emit_u16(cmp_reg);
        self.emit_u16(idx_reg);
        self.emit_u16(len_val_reg);
        self.free_register(len_val_reg);

        self.emit(Opcode::JumpIfNot);
        self.emit_u16(cmp_reg);
        let exit_patch = self.code.len();
        self.emit_i32(0);
        self.free_register(cmp_reg);

        self.breakable_frames.push(BreakableFrame {
            label: self.pending_label.take(),
            break_patches: Vec::new(),
            continue_patches: Vec::new(),
            continue_target: Some(0),
        });

        if let Some(v_reg) = result_reg {
            self.emit(Opcode::LoadUndefined);
            self.emit_u16(v_reg);
        }

        let key_val_reg = self.alloc_register();
        self.emit(Opcode::GetField);
        self.emit_u16(key_val_reg);
        self.emit_u16(keys_reg);
        self.emit_u16(idx_reg);
        if has_complex_left {
            match &stmt.left {
                ForInOfLeft::VariableDeclaration(decl) => {
                    if let Some(first) = decl.declarations.first() {
                        self.gen_binding_pattern(&first.id, key_val_reg, decl.kind, ctx)?;
                    } else {
                        return Err("for-in without variable".to_string());
                    }
                }
                ForInOfLeft::AssignmentTarget(at) => {
                    self.gen_assignment_target_destructure(at, key_val_reg, ctx)?;
                }
            };
        } else {
            let loop_var_name = match &stmt.left {
                ForInOfLeft::VariableDeclaration(decl) => decl.declarations.first().and_then(|d| {
                    if let BindingPattern::Identifier(n) = &d.id {
                        Some(n.clone())
                    } else {
                        None
                    }
                }),
                ForInOfLeft::AssignmentTarget(AssignmentTarget::Identifier(n)) => Some(n.clone()),
                _ => None,
            };
            if var_slot != key_val_reg {
                self.emit(Opcode::Move);
                self.emit_u16(var_slot);
                self.emit_u16(key_val_reg);
            }

            if let Some(ref name) = loop_var_name {
                if self.is_global_var(name) {
                    let atom = ctx.intern(name);
                    let idx = self.add_constant(JSValue::new_string(atom));
                    let src = if var_slot != key_val_reg {
                        var_slot
                    } else {
                        key_val_reg
                    };
                    self.emit(Opcode::SetGlobal);
                    self.emit_u32(idx);
                    self.emit_u16(src);
                }
            }
        }
        self.free_register(key_val_reg);

        if let Some(v_reg) = result_reg {
            self.gen_statement_with_target(&stmt.body, ctx, v_reg)?;
        } else {
            self.gen_statement(&stmt.body, ctx)?;
        }

        let continue_target = self.code.len();
        if let Some(frame) = self.breakable_frames.last_mut() {
            frame.continue_target = Some(continue_target);
        }

        self.emit(Opcode::IncLocal);
        self.emit_u16(idx_reg);

        self.emit_backward_jump(loop_start);

        let frame = self.breakable_frames.pop().unwrap();
        let end_pos = self.code.len();

        self.patch_jump(exit_patch, end_pos);

        for patch in frame.break_patches {
            self.patch_jump(patch, end_pos);
        }

        for patch in frame.continue_patches {
            self.patch_jump(patch, continue_target);
        }

        self.free_register(keys_reg);
        self.free_register(idx_reg);

        if needs_block_scope {
            self.pop_scope();
        }

        Ok(())
    }

    fn gen_for_of_statement(
        &mut self,
        stmt: &ForOfStatement,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        self.gen_for_of_statement_with_result(stmt, ctx, Some(0))
    }

    fn gen_for_of_statement_with_result(
        &mut self,
        stmt: &ForOfStatement,
        ctx: &mut JSContext,
        result_reg: Option<u16>,
    ) -> Result<(), String> {
        let needs_block_scope = match &stmt.left {
            ForInOfLeft::VariableDeclaration(decl) => decl.kind != VariableKind::Var,
            _ => false,
        };

        if needs_block_scope {
            self.push_scope();
            if let ForInOfLeft::VariableDeclaration(decl) = &stmt.left {
                if let Some(first) = decl.declarations.first() {
                    self.pre_scan_binding(&first.id, decl.kind);
                    self.emit_tdz_for_binding(&first.id)?;
                }
            }
        }

        let iterable_reg = self.gen_expression(&stmt.right, ctx)?;
        let iter_reg = self.alloc_register();
        self.emit(Opcode::GetIterator);
        self.emit_u16(iter_reg);
        self.emit_u16(iterable_reg);
        self.free_register(iterable_reg);

        let loop_start = self.code.len();

        let val_reg = self.alloc_register();
        let done_reg = self.alloc_register();
        self.emit(Opcode::IteratorNext);
        self.emit_u16(val_reg);
        self.emit_u16(done_reg);
        self.emit_u16(iter_reg);

        self.emit(Opcode::JumpIf);
        self.emit_u16(done_reg);
        let exit_patch = self.code.len();
        self.emit_i32(0);
        self.free_register(done_reg);

        self.breakable_frames.push(BreakableFrame {
            label: self.pending_label.take(),
            break_patches: Vec::new(),
            continue_patches: Vec::new(),
            continue_target: Some(0),
        });

        if let Some(v_reg) = result_reg {
            self.emit(Opcode::LoadUndefined);
            self.emit_u16(v_reg);
        }

        match &stmt.left {
            ForInOfLeft::VariableDeclaration(decl) => {
                if let Some(first) = decl.declarations.first() {
                    self.gen_binding_pattern(&first.id, val_reg, decl.kind, ctx)?;
                } else {
                    return Err("for-of without variable".to_string());
                }
            }
            ForInOfLeft::AssignmentTarget(target) => {
                self.gen_assignment_target_destructure(target, val_reg, ctx)?;
            }
        }
        self.free_register(val_reg);

        if let Some(v_reg) = result_reg {
            self.gen_statement_with_target(&stmt.body, ctx, v_reg)?;
        } else {
            self.gen_statement(&stmt.body, ctx)?;
        }

        let continue_target = self.code.len();
        if let Some(frame) = self.breakable_frames.last_mut() {
            frame.continue_target = Some(continue_target);
        }

        self.emit_backward_jump(loop_start);

        let frame = self.breakable_frames.pop().unwrap();
        let end_pos = self.code.len();

        self.patch_jump(exit_patch, end_pos);

        for patch in frame.break_patches {
            self.patch_jump(patch, end_pos);
        }
        for patch in frame.continue_patches {
            self.patch_jump(patch, continue_target);
        }

        self.free_register(iter_reg);

        if needs_block_scope {
            self.pop_scope();
        }

        Ok(())
    }

    fn gen_do_while_statement(
        &mut self,
        stmt: &DoWhileStatement,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        self.gen_do_while_statement_with_result(stmt, ctx, Some(0))
    }

    fn gen_do_while_statement_with_result(
        &mut self,
        stmt: &DoWhileStatement,
        ctx: &mut JSContext,
        result_reg: Option<u16>,
    ) -> Result<(), String> {
        let loop_start = self.code.len();

        self.breakable_frames.push(BreakableFrame {
            label: self.pending_label.take(),
            break_patches: Vec::new(),
            continue_patches: Vec::new(),
            continue_target: Some(0),
        });

        if let Some(v_reg) = result_reg {
            self.gen_statement_with_target(&stmt.body, ctx, v_reg)?;
        } else {
            self.gen_statement(&stmt.body, ctx)?;
        }

        let continue_target = self.code.len();
        if let Some(frame) = self.breakable_frames.last_mut() {
            frame.continue_target = Some(continue_target);
        }

        let test_reg = self.gen_expression(&stmt.test, ctx)?;
        self.emit_conditional_backward_jump(test_reg, loop_start, true);
        self.free_register(test_reg);

        let frame = self.breakable_frames.pop().unwrap();

        let end_pos = self.code.len();
        for patch in frame.break_patches {
            self.patch_jump(patch, end_pos);
        }
        for patch in frame.continue_patches {
            self.patch_jump(patch, frame.continue_target.unwrap());
        }

        Ok(())
    }

    fn gen_switch_statement(
        &mut self,
        stmt: &SwitchStatement,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        self.gen_switch_statement_with_result(stmt, ctx, None)
    }

    fn gen_switch_statement_with_result(
        &mut self,
        stmt: &SwitchStatement,
        ctx: &mut JSContext,
        result_reg: Option<u16>,
    ) -> Result<(), String> {
        let disc = self.gen_expression(&stmt.discriminant, ctx)?;

        let mut test_jumps: Vec<(usize, usize)> = Vec::new();
        for (case_idx, case) in stmt.cases.iter().enumerate() {
            if let Some(test_expr) = &case.test {
                let test_reg = self.gen_expression(test_expr, ctx)?;
                let eq_reg = self.alloc_register();
                self.emit(Opcode::StrictEq);
                self.emit_u16(eq_reg);
                self.emit_u16(disc);
                self.emit_u16(test_reg);
                self.free_register(test_reg);
                self.emit(Opcode::JumpIf);
                self.emit_u16(eq_reg);
                test_jumps.push((self.code.len(), case_idx));
                self.emit_i32(0);
                self.free_register(eq_reg);
            }
        }

        self.emit(Opcode::Jump);
        let default_jump_placeholder = self.code.len();
        self.emit_i32(0);

        self.breakable_frames.push(BreakableFrame {
            label: self.pending_label.take(),
            break_patches: Vec::new(),
            continue_patches: Vec::new(),
            continue_target: None,
        });

        self.push_scope();
        for case in &stmt.cases {
            self.pre_scan_let_const(&case.consequent)?;
            self.emit_tdz_for_block(&case.consequent)?;
        }

        let mut body_starts = vec![0usize; stmt.cases.len()];
        for (case_idx, case) in stmt.cases.iter().enumerate() {
            body_starts[case_idx] = self.code.len();
            if let Some(v_reg) = result_reg {
                for s in &case.consequent {
                    self.gen_statement_with_target(s, ctx, v_reg)?;
                }
            } else {
                for s in &case.consequent {
                    self.gen_statement(s, ctx)?;
                }
            }
        }
        self.pop_scope();

        let frame = self.breakable_frames.pop().unwrap();
        let end_pos = self.code.len();

        for patch in frame.break_patches {
            self.patch_jump(patch, end_pos);
        }

        let default_case_idx = stmt.cases.iter().position(|c| c.test.is_none());
        let default_target = default_case_idx
            .map(|idx| body_starts[idx])
            .unwrap_or(end_pos);
        self.patch_jump(default_jump_placeholder, default_target);

        for (placeholder, case_idx) in test_jumps {
            self.patch_jump(placeholder, body_starts[case_idx]);
        }

        self.free_register(disc);
        Ok(())
    }

    fn gen_throw_statement(
        &mut self,
        stmt: &ThrowStatement,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        let reg = self.gen_expression(&stmt.argument, ctx)?;
        self.emit(Opcode::Throw);
        self.emit_u16(reg);
        self.free_register(reg);
        Ok(())
    }

    fn gen_try_statement(
        &mut self,
        stmt: &TryStatement,
        ctx: &mut JSContext,
        result_reg: Option<u16>,
    ) -> Result<(), String> {
        self.emit(Opcode::Try);
        let catch_pc_pos = self.code.len();
        self.emit_i32(0);
        let finally_pc_pos = self.code.len();
        self.emit_i32(0);

        self.push_scope();
        self.pre_scan_let_const(&stmt.block.body)?;
        self.emit_tdz_for_block(&stmt.block.body)?;
        if let Some(v_reg) = result_reg {
            for s in &stmt.block.body {
                self.gen_statement_with_target(s, ctx, v_reg)?;
            }
        } else {
            for s in &stmt.block.body {
                self.gen_statement(s, ctx)?;
            }
        }
        self.pop_scope();

        self.emit(Opcode::Catch);

        self.emit(Opcode::Jump);
        let end_jump_pos = self.code.len();
        self.emit_i32(0);

        let has_catch = stmt.handler.is_some();
        let mut after_catch_jump_pos: Option<usize> = None;

        if let Some(handler) = &stmt.handler {
            let catch_start = self.code.len();

            let catch_bytes = (catch_start as i32).to_le_bytes();
            self.code[catch_pc_pos..catch_pc_pos + 4].copy_from_slice(&catch_bytes);

            self.push_scope();
            if let Some(param) = &handler.param {
                if self.is_strict {
                    if let BindingPattern::Identifier(name) = param {
                        if name == "eval" || name == "arguments" {
                            return Err(format!(
                                "SyntaxError: Catch variable may not be '{}' in strict mode",
                                name
                            ));
                        }
                    }
                }
                match param {
                    BindingPattern::Identifier(name) => {
                        self.pre_scan_binding(param, VariableKind::Let);
                        self.emit_tdz_for_binding(param)?;
                        let slot = self.declare_var(name, VariableKind::Let);
                        if slot != 0 {
                            self.emit(Opcode::Move);
                            self.emit_u16(slot);
                            self.emit_u16(0);
                        }

                        if self.is_global_var(name) {
                            let atom = ctx.intern(name);
                            let idx = self.add_constant(JSValue::new_string(atom));
                            self.emit(Opcode::SetGlobal);
                            self.emit_u32(idx);
                            self.emit_u16(0);
                        }
                    }
                    _ => self.gen_binding_pattern(param, 0, VariableKind::Let, ctx)?,
                }
            }
            self.pre_scan_let_const(&handler.body.body)?;
            self.emit_tdz_for_block(&handler.body.body)?;
            if let Some(v_reg) = result_reg {
                for s in &handler.body.body {
                    self.gen_statement_with_target(s, ctx, v_reg)?;
                }
            } else {
                for s in &handler.body.body {
                    self.gen_statement(s, ctx)?;
                }
            }
            self.pop_scope();

            self.emit(Opcode::Jump);
            after_catch_jump_pos = Some(self.code.len());
            self.emit_i32(0);
        }

        let has_finally = stmt.finalizer.is_some();
        let mut finally_start = 0usize;

        if let Some(finalizer) = &stmt.finalizer {
            finally_start = self.code.len();

            let finally_bytes = (finally_start as i32).to_le_bytes();
            self.code[finally_pc_pos..finally_pc_pos + 4].copy_from_slice(&finally_bytes);

            if !has_catch {
                self.code[catch_pc_pos..catch_pc_pos + 4].copy_from_slice(&finally_bytes);
            }

            self.patch_jump(end_jump_pos, finally_start);

            self.emit(Opcode::Finally);

            self.push_scope();
            self.pre_scan_let_const(&finalizer.body)?;
            self.emit_tdz_for_block(&finalizer.body)?;
            for s in &finalizer.body {
                self.gen_statement(s, ctx)?;
            }
            self.pop_scope();
        }

        let end_pos = self.code.len();

        if !has_finally {
            self.patch_jump(end_jump_pos, end_pos);
        }

        if let Some(pos) = after_catch_jump_pos {
            let catch_jump_target = if has_finally { finally_start } else { end_pos };
            self.patch_jump(pos, catch_jump_target);
        }

        Ok(())
    }

    fn gen_function_like_with_name(
        &mut self,
        expr: &Expression,
        name: &str,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        match expr {
            Expression::FunctionExpression(func) => self.gen_function_expression(
                &func.params,
                &func.body,
                Some(name),
                ctx,
                func.generator,
                func.is_async,
                false,
            ),
            Expression::ArrowFunction(arrow) => {
                let body = match &arrow.body {
                    ArrowBody::Expression(expr) => BlockStatement {
                        body: vec![ASTNode::ReturnStatement(ReturnStatement {
                            argument: Some((**expr).clone()),
                        })],
                        lines: vec![],
                    },
                    ArrowBody::Block(block) => block.clone(),
                };
                self.gen_function_expression(
                    &arrow.params,
                    &body,
                    Some(name),
                    ctx,
                    false,
                    arrow.is_async,
                    true,
                )
            }
            Expression::ClassExpression(ce) => {
                let class_slot = self.alloc_register();
                self.gen_class_body(class_slot, name, &ce.super_class, &ce.body, ctx)?;
                Ok(class_slot)
            }
            _ => unreachable!(),
        }
    }

    fn gen_expression_with_name(
        &mut self,
        expr: &Expression,
        binding: &BindingPattern,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        let needs_name = match binding {
            BindingPattern::Identifier(binding_name) => match expr {
                Expression::FunctionExpression(func) if func.name.is_none() => {
                    Some(binding_name.as_str())
                }
                Expression::ArrowFunction(_) => Some(binding_name.as_str()),
                Expression::ClassExpression(ce) if ce.name.is_none() => Some(binding_name.as_str()),
                _ => None,
            },
            _ => None,
        };
        if let Some(name) = needs_name {
            self.gen_function_like_with_name(expr, name, ctx)
        } else {
            self.gen_expression(expr, ctx)
        }
    }

    fn gen_expression_with_assign_name(
        &mut self,
        expr: &Expression,
        target_name: &str,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        let needs_name = match expr {
            Expression::FunctionExpression(func) if func.name.is_none() => Some(target_name),
            Expression::ArrowFunction(_) => Some(target_name),
            Expression::ClassExpression(ce) if ce.name.is_none() => Some(target_name),
            _ => None,
        };
        if let Some(name) = needs_name {
            self.gen_function_like_with_name(expr, name, ctx)
        } else {
            self.gen_expression(expr, ctx)
        }
    }

    fn gen_expression(&mut self, expr: &Expression, ctx: &mut JSContext) -> Result<u16, String> {
        match expr {
            Expression::Identifier(id) => {
                if id.name == "arguments"
                    && !self.is_script_level
                    && !self.is_arrow
                    && self.lookup_var("arguments").is_none()
                {
                    self.function_uses_arguments = true;
                    let r = self.alloc_register();
                    self.emit(Opcode::GetArguments);
                    self.emit_u16(r);
                    return Ok(r);
                }
                match self.resolve_var(&id.name, ctx) {
                    Some(VarLocation::Local(slot)) => {
                        if self.is_global_var(&id.name) {
                            let atom = ctx.intern(&id.name);
                            let idx = self.add_constant(JSValue::new_string(atom));
                            let r = self.alloc_register();
                            self.emit(Opcode::GetGlobal);
                            self.emit_u16(r);
                            self.emit_u32(idx);
                            return Ok(r);
                        }

                        if let Some(entry) = self.lookup_var(&id.name) {
                            if entry.kind != VariableKind::Var && !entry.initialized {
                                self.emit(Opcode::CheckTdz);
                                self.emit_u16(slot);
                            }
                        }
                        Ok(slot)
                    }
                    Some(VarLocation::Upvalue(slot)) => {
                        let r = self.alloc_register();
                        self.emit(Opcode::GetUpvalue);
                        self.emit_u16(r);
                        self.emit_u16(slot);

                        if let Some(entry) = self.lookup_var(&id.name) {
                            if entry.kind != VariableKind::Var && !entry.initialized {
                                self.emit(Opcode::CheckTdz);
                                self.emit_u16(r);
                            }
                        }
                        Ok(r)
                    }
                    None => {
                        let atom = ctx.intern(&id.name);
                        let idx = self.add_constant(JSValue::new_string(atom));
                        let r = self.alloc_register();
                        self.emit(Opcode::GetGlobal);
                        self.emit_u16(r);
                        self.emit_u32(idx);
                        if !self.suppress_ref_error && self.is_strict {
                            self.emit(Opcode::CheckRef);
                            self.emit_u16(r);
                            self.emit_u32(idx);
                        }
                        Ok(r)
                    }
                }
            }
            Expression::Literal(lit) => {
                let r = self.alloc_register();
                match lit {
                    Literal::Number(n) => {
                        if *n >= i32::MIN as f64 && *n <= i32::MAX as f64 && n.fract() == 0.0 {
                            let val = *n as i32;
                            if val >= i8::MIN as i32 && val <= i8::MAX as i32 {
                                self.emit(Opcode::LoadInt8);
                                self.emit_u16(r);
                                self.emit_u8(val as u8);
                            } else {
                                self.emit_int_const(r, val);
                            }
                        } else {
                            let idx = self.add_constant(JSValue::new_float(*n));
                            self.emit(Opcode::LoadConst);
                            self.emit_u16(r);
                            self.emit_u32(idx);
                        }
                    }
                    Literal::LegacyOctal(n) => {
                        if self.is_strict {
                            return Err(
                                "SyntaxError: Octal literals are not allowed in strict mode"
                                    .to_string(),
                            );
                        }
                        if *n >= i32::MIN as i64 && *n <= i32::MAX as i64 {
                            let val = *n as i32;
                            if val >= i8::MIN as i32 && val <= i8::MAX as i32 {
                                self.emit(Opcode::LoadInt8);
                                self.emit_u16(r);
                                self.emit_u8(val as u8);
                            } else {
                                self.emit_int_const(r, val);
                            }
                        } else {
                            let idx = self.add_constant(JSValue::new_float(*n as f64));
                            self.emit(Opcode::LoadConst);
                            self.emit_u16(r);
                            self.emit_u32(idx);
                        }
                    }
                    Literal::String(s, _) => {
                        let idx = self.intern_and_add_const(ctx, s);
                        self.emit(Opcode::LoadConst);
                        self.emit_u16(r);
                        self.emit_u32(idx);
                    }
                    Literal::Boolean(b) => {
                        if *b {
                            self.emit(Opcode::LoadTrue);
                        } else {
                            self.emit(Opcode::LoadFalse);
                        }
                        self.emit_u16(r);
                    }
                    Literal::Null => {
                        self.emit(Opcode::LoadNull);
                        self.emit_u16(r);
                    }
                    Literal::Undefined => {
                        self.emit(Opcode::LoadUndefined);
                        self.emit_u16(r);
                    }
                    Literal::BigInt(s) => {
                        let atom = ctx.atom_table_mut().intern(s);
                        let idx = self.add_constant(JSValue::new_string(atom));
                        self.emit(Opcode::LoadConst);
                        self.emit_u16(r);
                        self.emit_u32(idx);

                        let bigint_atom = ctx.common_atoms.bigint;
                        let bigint_idx = self.add_constant(JSValue::new_string(bigint_atom));
                        let func_reg = self.alloc_register();
                        self.emit(Opcode::GetGlobal);
                        self.emit_u16(func_reg);
                        self.emit_u32(bigint_idx);
                        self.emit(Opcode::Call);
                        self.emit_u16(r);
                        self.emit_u16(func_reg);
                        self.emit_u16(1);
                        self.emit_u16(r);
                        self.free_register(func_reg);
                    }
                }
                Ok(r)
            }
            Expression::BinaryExpression(bin) => self.gen_binary_expression(bin, ctx),
            Expression::LogicalExpression(logical) => self.gen_logical_expression(logical, ctx),
            Expression::UnaryExpression(unary) => self.gen_unary_expression(unary, ctx),
            Expression::UpdateExpression(update) => self.gen_update_expression(update, ctx),
            Expression::AssignmentExpression(assign) => self.gen_assignment_expression(assign, ctx),
            Expression::ConditionalExpression(cond) => self.gen_conditional_expression(cond, ctx),
            Expression::MemberExpression(member) => self.gen_member_expression(member, ctx),
            Expression::OptionalMemberExpression(opt) => {
                let obj_reg = self.gen_expression(&opt.object, ctx)?;
                let result = self.alloc_register();

                let undef_reg = self.alloc_register();
                self.emit(Opcode::LoadUndefined);
                self.emit_u16(undef_reg);
                let is_undef = self.alloc_register();
                self.emit(Opcode::StrictEq);
                self.emit_u16(is_undef);
                self.emit_u16(obj_reg);
                self.emit_u16(undef_reg);
                self.free_register(undef_reg);
                self.emit(Opcode::JumpIf);
                self.emit_u16(is_undef);
                let short_jump = self.code.len();
                self.emit_i32(0);
                self.free_register(is_undef);

                let null_reg = self.alloc_register();
                self.emit(Opcode::LoadNull);
                self.emit_u16(null_reg);
                let is_null = self.alloc_register();
                self.emit(Opcode::StrictEq);
                self.emit_u16(is_null);
                self.emit_u16(obj_reg);
                self.emit_u16(null_reg);
                self.free_register(null_reg);
                self.emit(Opcode::JumpIf);
                self.emit_u16(is_null);
                let short_jump_2 = self.code.len();
                self.emit_i32(0);
                self.free_register(is_null);

                match &opt.property {
                    MemberProperty::Identifier(name) => {
                        let atom = ctx.intern(name);
                        self.emit(Opcode::GetNamedProp);
                        self.emit_u16(result);
                        self.emit_u16(obj_reg);
                        self.emit_u32(atom.0);
                    }
                    MemberProperty::Computed(expr) => {
                        let key_reg = self.gen_expression(expr, ctx)?;
                        self.emit(Opcode::GetField);
                        self.emit_u16(result);
                        self.emit_u16(obj_reg);
                        self.emit_u16(key_reg);
                        self.free_register(key_reg);
                    }
                    MemberProperty::PrivateIdentifier(name) => {
                        let key_idx = self.intern_and_add_const(ctx, name);
                        let key_reg = self.alloc_register();
                        self.emit(Opcode::LoadConst);
                        self.emit_u16(key_reg);
                        self.emit_u32(key_idx);
                        self.emit(Opcode::GetPrivate);
                        self.emit_u16(result);
                        self.emit_u16(obj_reg);
                        self.emit_u16(key_reg);
                        self.free_register(key_reg);
                    }
                }
                self.emit(Opcode::Jump);
                let end_jump = self.code.len();
                self.emit_i32(0);

                let short_pos = self.code.len();
                self.patch_jump(short_jump, short_pos);
                self.patch_jump(short_jump_2, short_pos);
                self.emit(Opcode::LoadUndefined);
                self.emit_u16(result);

                let end_pos = self.code.len();
                self.patch_jump(end_jump, end_pos);
                self.free_register(obj_reg);
                Ok(result)
            }
            Expression::CallExpression(call) => self.gen_call_expression(call, ctx),
            Expression::NewExpression(new) => self.gen_new_expression(new, ctx),
            Expression::ArrayExpression(arr) => self.gen_array_expression(arr, ctx),
            Expression::ObjectExpression(obj) => self.gen_object_expression(obj, ctx),
            Expression::TemplateLiteral(tpl) => {
                let first_quasi = tpl
                    .quasis
                    .first()
                    .map(|quasi| quasi.value.as_str())
                    .unwrap_or("");
                let result = self.alloc_register();
                let first_idx = self.intern_and_add_const(ctx, first_quasi);
                self.emit(Opcode::LoadConst);
                self.emit_u16(result);
                self.emit_u32(first_idx);

                for (i, expr) in tpl.expressions.iter().enumerate() {
                    let expr_reg = self.gen_expression(expr, ctx)?;
                    self.emit(Opcode::Add);
                    self.emit_u16(result);
                    self.emit_u16(result);
                    self.emit_u16(expr_reg);
                    self.free_register(expr_reg);

                    if let Some(quasi) = tpl.quasis.get(i + 1) {
                        let quasi_idx = self.intern_and_add_const(ctx, &quasi.value);
                        let quasi_reg = self.alloc_register();
                        self.emit(Opcode::LoadConst);
                        self.emit_u16(quasi_reg);
                        self.emit_u32(quasi_idx);
                        self.emit(Opcode::Add);
                        self.emit_u16(result);
                        self.emit_u16(result);
                        self.emit_u16(quasi_reg);
                        self.free_register(quasi_reg);
                    }
                }

                Ok(result)
            }
            Expression::FunctionExpression(func) => self.gen_function_expression(
                &func.params,
                &func.body,
                func.name.as_deref(),
                ctx,
                func.generator,
                func.is_async,
                false,
            ),
            Expression::ArrowFunction(arrow) => {
                let body = match &arrow.body {
                    ArrowBody::Expression(expr) => BlockStatement {
                        body: vec![ASTNode::ReturnStatement(ReturnStatement {
                            argument: Some((**expr).clone()),
                        })],
                        lines: vec![],
                    },
                    ArrowBody::Block(block) => block.clone(),
                };
                self.gen_function_expression(
                    &arrow.params,
                    &body,
                    None,
                    ctx,
                    false,
                    arrow.is_async,
                    true,
                )
            }
            Expression::This => Ok(0),
            Expression::Super => {
                let r = self.alloc_register();
                self.emit(Opcode::GetSuper);
                self.emit_u16(r);
                Ok(r)
            }
            Expression::PrivateIdentifier(name) => {
                let idx = self.intern_and_add_const(ctx, name);
                let r = self.alloc_register();
                self.emit(Opcode::LoadConst);
                self.emit_u16(r);
                self.emit_u32(idx);
                Ok(r)
            }
            Expression::YieldExpression(yield_expr) => {
                if yield_expr.delegate {
                    return Err("yield* not yet supported in register VM".to_string());
                }
                let val_reg = if let Some(arg) = &yield_expr.argument {
                    self.gen_expression(arg, ctx)?
                } else {
                    let r = self.alloc_register();
                    self.emit(Opcode::LoadUndefined);
                    self.emit_u16(r);
                    r
                };
                self.emit(Opcode::Yield);
                self.emit_u16(val_reg);
                Ok(val_reg)
            }
            Expression::AwaitExpression(await_expr) => {
                let val_reg = self.gen_expression(&await_expr.argument, ctx)?;
                self.emit(Opcode::Await);
                self.emit_u16(val_reg);
                Ok(val_reg)
            }
            Expression::RegExpLiteral(re) => {
                let pattern_idx = self.intern_and_add_const(ctx, &re.pattern);
                let flags_idx = self.intern_and_add_const(ctx, &re.flags);
                let dst = self.alloc_register();
                self.emit(Opcode::NewRegExp);
                self.emit_u16(dst);
                self.emit_u32(pattern_idx);
                self.emit_u32(flags_idx);
                Ok(dst)
            }
            Expression::SequenceExpression(seq) => {
                let mut last_r = None;
                for (i, e) in seq.expressions.iter().enumerate() {
                    let r = self.gen_expression(e, ctx)?;
                    if i == seq.expressions.len() - 1 {
                        last_r = Some(r);
                    } else {
                        self.free_register(r);
                    }
                }

                Ok(last_r.unwrap_or_else(|| {
                    let r = self.alloc_register();
                    self.emit(Opcode::LoadUndefined);
                    self.emit_u16(r);
                    r
                }))
            }
            Expression::ClassExpression(ce) => {
                let class_name = ce.name.as_deref().unwrap_or("");
                let class_slot = self.alloc_register();
                self.gen_class_body(class_slot, class_name, &ce.super_class, &ce.body, ctx)?;
                Ok(class_slot)
            }
            _ => {
                let r = self.alloc_register();
                self.emit(Opcode::LoadUndefined);
                self.emit_u16(r);
                Ok(r)
            }
        }
    }

    fn gen_binary_expression(
        &mut self,
        bin: &BinaryExpression,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        let left = self.gen_expression(&bin.left, ctx)?;

        let right_imm8 = if let Expression::Literal(Literal::Number(n)) = &*bin.right {
            if n.fract() == 0.0 {
                let v = *n as i64;
                if v >= i8::MIN as i64 && v <= i8::MAX as i64 {
                    Some(v as i8)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(imm) = right_imm8 {
            match bin.op {
                BinaryOp::Sub => {
                    let dst = self.alloc_register();
                    self.emit(Opcode::SubImm8);
                    self.emit_u16(dst);
                    self.emit_u16(left);
                    self.emit_u8(imm as u8);
                    self.free_register(left);
                    return Ok(dst);
                }
                BinaryOp::Lte => {
                    let dst = self.alloc_register();
                    self.emit(Opcode::LteImm8);
                    self.emit_u16(dst);
                    self.emit_u16(left);
                    self.emit_u8(imm as u8);
                    self.free_register(left);
                    return Ok(dst);
                }
                _ => {}
            }
        }

        let right = self.gen_expression(&bin.right, ctx)?;

        let op = match bin.op {
            BinaryOp::Add => Opcode::Add,
            BinaryOp::Sub => Opcode::Sub,
            BinaryOp::Mul => Opcode::Mul,
            BinaryOp::Div => Opcode::Div,
            BinaryOp::Mod => Opcode::Mod,
            BinaryOp::Pow => Opcode::Pow,
            BinaryOp::BitAnd => Opcode::BitAnd,
            BinaryOp::BitOr => Opcode::BitOr,
            BinaryOp::BitXor => Opcode::BitXor,
            BinaryOp::Shl => Opcode::Shl,
            BinaryOp::Shr => Opcode::Shr,
            BinaryOp::UShr => Opcode::UShr,
            BinaryOp::Lt => Opcode::Lt,
            BinaryOp::Lte => Opcode::Lte,
            BinaryOp::Gt => Opcode::Gt,
            BinaryOp::Gte => Opcode::Gte,
            BinaryOp::Eq => Opcode::Eq,
            BinaryOp::Neq => Opcode::Neq,
            BinaryOp::StrictEq => Opcode::StrictEq,
            BinaryOp::StrictNeq => Opcode::StrictNeq,
            BinaryOp::In => {
                if matches!(&*bin.left, Expression::PrivateIdentifier(_)) {
                    Opcode::HasPrivate
                } else {
                    Opcode::HasProperty
                }
            }
            BinaryOp::InstanceOf => Opcode::InstanceOf,
        };

        let dst = self.alloc_register();
        self.emit(op);
        self.emit_u16(dst);

        if bin.op == BinaryOp::In {
            self.emit_u16(right);
            self.emit_u16(left);
        } else {
            self.emit_u16(left);
            self.emit_u16(right);
        }

        self.free_register(left);
        self.free_register(right);
        Ok(dst)
    }

    fn gen_logical_expression(
        &mut self,
        logical: &LogicalExpression,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        if self.p2_fold_profile.allow_logical_bool_left() {
            if let Expression::Literal(Literal::Boolean(left_bool)) = &*logical.left {
                match logical.op {
                    LogicalOp::And => {
                        if *left_bool {
                            return self.gen_expression(&logical.right, ctx);
                        }
                        let dst = self.alloc_register();
                        self.emit_bool_const(dst, false);
                        return Ok(dst);
                    }
                    LogicalOp::Or => {
                        if *left_bool {
                            let dst = self.alloc_register();
                            self.emit_bool_const(dst, true);
                            return Ok(dst);
                        }
                        return self.gen_expression(&logical.right, ctx);
                    }
                    LogicalOp::NullishCoalescing => {}
                }
            }
        }

        if self.p2_fold_profile.allow_aggressive_truthy() {
            if let Expression::Literal(lit) = &*logical.left {
                if let Some(left_truthy) = Self::literal_truthiness(lit) {
                    match logical.op {
                        LogicalOp::And => {
                            if left_truthy {
                                return self.gen_expression(&logical.right, ctx);
                            }
                            return self.gen_expression(&logical.left, ctx);
                        }
                        LogicalOp::Or => {
                            if left_truthy {
                                return self.gen_expression(&logical.left, ctx);
                            }
                            return self.gen_expression(&logical.right, ctx);
                        }
                        LogicalOp::NullishCoalescing => {}
                    }
                }
            }
        }

        if self.p2_fold_profile.allow_nullish_literal_left() {
            if let LogicalOp::NullishCoalescing = logical.op {
                if let Expression::Literal(lit) = &*logical.left {
                    return match lit {
                        Literal::Null | Literal::Undefined => {
                            self.gen_expression(&logical.right, ctx)
                        }
                        _ => self.gen_expression(&logical.left, ctx),
                    };
                }
            }
        }

        match logical.op {
            LogicalOp::And => {
                if self.opt_branch_result_prealloc {
                    let left = self.gen_expression(&logical.left, ctx)?;
                    let result = if self.reserved_slots.contains(&left) {
                        let dst = self.alloc_register();
                        self.emit(Opcode::Move);
                        self.emit_u16(dst);
                        self.emit_u16(left);
                        dst
                    } else {
                        left
                    };

                    self.emit(Opcode::JumpIfNot);
                    self.emit_u16(left);
                    let short_jump = self.code.len();
                    self.emit_i32(0);

                    let right = self.gen_expression(&logical.right, ctx)?;
                    if right != result {
                        self.emit(Opcode::Move);
                        self.emit_u16(result);
                        self.emit_u16(right);
                        self.free_register(right);
                    }

                    let end_pos = self.code.len();
                    self.patch_jump(short_jump, end_pos);
                    return Ok(result);
                }

                let left = self.gen_expression(&logical.left, ctx)?;
                self.emit(Opcode::JumpIfNot);
                self.emit_u16(left);
                let short_jump = self.code.len();
                self.emit_i32(0);

                let right = self.gen_expression(&logical.right, ctx)?;

                let result = if self.reserved_slots.contains(&left) && right != left {
                    let dst = self.alloc_register();
                    self.emit(Opcode::Move);
                    self.emit_u16(dst);
                    self.emit_u16(right);
                    self.free_register(right);
                    dst
                } else if right != left {
                    self.emit(Opcode::Move);
                    self.emit_u16(left);
                    self.emit_u16(right);
                    self.free_register(right);
                    left
                } else {
                    left
                };

                if result != left {
                    self.emit(Opcode::Jump);
                    let end_jump = self.code.len();
                    self.emit_i32(0);
                    let patch_pos = self.code.len();
                    self.patch_jump(short_jump, patch_pos);
                    self.emit(Opcode::Move);
                    self.emit_u16(result);
                    self.emit_u16(left);
                    let end_pos = self.code.len();
                    self.patch_jump(end_jump, end_pos);
                } else {
                    let end_pos = self.code.len();
                    self.patch_jump(short_jump, end_pos);
                }
                Ok(result)
            }
            LogicalOp::Or => {
                if self.opt_branch_result_prealloc {
                    let left = self.gen_expression(&logical.left, ctx)?;
                    let result = if self.reserved_slots.contains(&left) {
                        let dst = self.alloc_register();
                        self.emit(Opcode::Move);
                        self.emit_u16(dst);
                        self.emit_u16(left);
                        dst
                    } else {
                        left
                    };

                    self.emit(Opcode::JumpIf);
                    self.emit_u16(left);
                    let short_jump = self.code.len();
                    self.emit_i32(0);

                    let right = self.gen_expression(&logical.right, ctx)?;
                    if right != result {
                        self.emit(Opcode::Move);
                        self.emit_u16(result);
                        self.emit_u16(right);
                        self.free_register(right);
                    }

                    let end_pos = self.code.len();
                    self.patch_jump(short_jump, end_pos);
                    return Ok(result);
                }

                let left = self.gen_expression(&logical.left, ctx)?;
                self.emit(Opcode::JumpIf);
                self.emit_u16(left);
                let short_jump = self.code.len();
                self.emit_i32(0);

                let right = self.gen_expression(&logical.right, ctx)?;

                let result = if self.reserved_slots.contains(&left) && right != left {
                    let dst = self.alloc_register();
                    self.emit(Opcode::Move);
                    self.emit_u16(dst);
                    self.emit_u16(right);
                    self.free_register(right);
                    dst
                } else if right != left {
                    self.emit(Opcode::Move);
                    self.emit_u16(left);
                    self.emit_u16(right);
                    self.free_register(right);
                    left
                } else {
                    left
                };

                if result != left {
                    self.emit(Opcode::Jump);
                    let end_jump = self.code.len();
                    self.emit_i32(0);
                    let patch_pos = self.code.len();
                    self.patch_jump(short_jump, patch_pos);
                    self.emit(Opcode::Move);
                    self.emit_u16(result);
                    self.emit_u16(left);
                    let end_pos = self.code.len();
                    self.patch_jump(end_jump, end_pos);
                } else {
                    let end_pos = self.code.len();
                    self.patch_jump(short_jump, end_pos);
                }
                Ok(result)
            }
            LogicalOp::NullishCoalescing => {
                let left = self.gen_expression(&logical.left, ctx)?;
                let result = if self.reserved_slots.contains(&left) {
                    let dst = self.alloc_register();
                    self.emit(Opcode::Move);
                    self.emit_u16(dst);
                    self.emit_u16(left);
                    dst
                } else {
                    left
                };

                let eval_right_jump = if self.opt_jump_if_nullish {
                    self.emit(Opcode::JumpIfNullish);
                    self.emit_u16(left);
                    let patch = self.code.len();
                    self.emit_i32(0);
                    patch
                } else {
                    let check_reg = self.alloc_register();
                    self.emit(Opcode::LoadNull);
                    self.emit_u16(check_reg);
                    self.emit(Opcode::Eq);
                    self.emit_u16(check_reg);
                    self.emit_u16(left);
                    self.emit_u16(check_reg);
                    self.emit(Opcode::JumpIf);
                    self.emit_u16(check_reg);
                    let patch = self.code.len();
                    self.emit_i32(0);
                    self.free_register(check_reg);
                    patch
                };

                self.emit(Opcode::Jump);
                let end_jump = self.code.len();
                self.emit_i32(0);

                let eval_right_pos = self.code.len();
                self.patch_jump(eval_right_jump, eval_right_pos);
                let right = self.gen_expression(&logical.right, ctx)?;
                if right != result {
                    self.emit(Opcode::Move);
                    self.emit_u16(result);
                    self.emit_u16(right);
                    self.free_register(right);
                }

                let end_pos = self.code.len();
                self.patch_jump(end_jump, end_pos);
                Ok(result)
            }
        }
    }

    fn gen_unary_expression(
        &mut self,
        unary: &UnaryExpression,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        let saved = self.suppress_ref_error;
        if matches!(unary.op, UnaryOp::TypeOf) {
            if matches!(&*unary.argument, Expression::Identifier(_)) {
                self.suppress_ref_error = true;
            }
        }
        if matches!(unary.op, UnaryOp::Delete) {
            if matches!(&*unary.argument, Expression::Identifier(_)) {
                self.suppress_ref_error = true;
            }
        }
        let arg = self.gen_expression(&unary.argument, ctx)?;
        self.suppress_ref_error = saved;
        match unary.op {
            UnaryOp::Minus => {
                let dst = self.alloc_register();
                self.emit(Opcode::Neg);
                self.emit_u16(dst);
                self.emit_u16(arg);
                self.free_register(arg);
                return Ok(dst);
            }
            UnaryOp::Not => {
                let dst = self.alloc_register();
                self.emit(Opcode::Not);
                self.emit_u16(dst);
                self.emit_u16(arg);
                self.free_register(arg);
                return Ok(dst);
            }
            UnaryOp::TypeOf => {
                let dst = self.alloc_register();
                self.emit(Opcode::TypeOf);
                self.emit_u16(dst);
                self.emit_u16(arg);
                self.free_register(arg);
                return Ok(dst);
            }
            UnaryOp::BitNot => {
                let dst = self.alloc_register();
                self.emit(Opcode::BitNot);
                self.emit_u16(dst);
                self.emit_u16(arg);
                self.free_register(arg);
                return Ok(dst);
            }
            UnaryOp::Plus => {}
            UnaryOp::Void => {
                let dst = self.alloc_register();
                self.emit(Opcode::LoadUndefined);
                self.emit_u16(dst);
                self.free_register(arg);
                return Ok(dst);
            }
            UnaryOp::Delete => {
                return self.gen_delete_expression(arg, &unary.argument, ctx);
            }
        }
        Ok(arg)
    }

    fn gen_delete_expression(
        &mut self,
        arg_reg: u16,
        arg_expr: &Expression,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        match arg_expr {
            Expression::Identifier(id) => {
                self.free_register(arg_reg);
                let dst = self.alloc_register();
                if id.name == "arguments" && !self.is_script_level && !self.is_arrow {
                    self.emit(Opcode::LoadFalse);
                    self.emit_u16(dst);
                } else if self.resolve_var(&id.name, ctx).is_some() {
                    self.emit(Opcode::LoadFalse);
                    self.emit_u16(dst);
                } else {
                    let atom = ctx.intern(&id.name);
                    let idx = self.add_constant(JSValue::new_string(atom));
                    self.emit(Opcode::DeleteGlobal);
                    self.emit_u16(dst);
                    self.emit_u32(idx);
                }
                Ok(dst)
            }
            Expression::MemberExpression(member) => {
                if matches!(&*member.object, Expression::Super) {
                    if let MemberProperty::Computed(expr) = &member.property {
                        self.gen_expression(expr, ctx)?;
                    }
                    self.free_register(arg_reg);
                    let msg_idx = self.add_constant(JSValue::new_string(
                        ctx.intern("Cannot delete super reference"),
                    ));
                    self.emit(Opcode::ThrowReferenceError);
                    self.emit_u32(msg_idx);
                    let dst = self.alloc_register();
                    self.emit(Opcode::LoadUndefined);
                    self.emit_u16(dst);
                    return Ok(dst);
                }
                let obj = self.gen_expression(&member.object, ctx)?;
                let dst = self.alloc_register();
                match &member.property {
                    MemberProperty::Identifier(name) => {
                        let key_idx = self.intern_and_add_const(ctx, name);
                        let key = self.alloc_register();
                        self.emit(Opcode::LoadConst);
                        self.emit_u16(key);
                        self.emit_u32(key_idx);
                        self.emit(Opcode::DeleteProp);
                        self.emit_u16(dst);
                        self.emit_u16(obj);
                        self.emit_u16(key);
                        self.free_register(key);
                    }
                    MemberProperty::Computed(expr) => {
                        let key = self.gen_expression(expr, ctx)?;
                        self.emit(Opcode::DeleteProp);
                        self.emit_u16(dst);
                        self.emit_u16(obj);
                        self.emit_u16(key);
                    }
                    MemberProperty::PrivateIdentifier(name) => {
                        let key_idx = self.intern_and_add_const(ctx, name);
                        let key = self.alloc_register();
                        self.emit(Opcode::LoadConst);
                        self.emit_u16(key);
                        self.emit_u32(key_idx);
                        self.emit(Opcode::DeleteProp);
                        self.emit_u16(dst);
                        self.emit_u16(obj);
                        self.emit_u16(key);
                        self.free_register(key);
                    }
                }
                self.free_register(obj);
                Ok(dst)
            }
            Expression::Literal(Literal::Number(n)) if n.is_nan() || n.is_infinite() => {
                self.free_register(arg_reg);
                let name = if n.is_nan() { "NaN" } else { "Infinity" };
                let atom = ctx.intern(name);
                let idx = self.add_constant(JSValue::new_string(atom));
                let dst = self.alloc_register();
                self.emit(Opcode::DeleteGlobal);
                self.emit_u16(dst);
                self.emit_u32(idx);
                Ok(dst)
            }
            _ => {
                self.free_register(arg_reg);
                let dst = self.alloc_register();
                self.emit(Opcode::LoadTrue);
                self.emit_u16(dst);
                Ok(dst)
            }
        }
    }

    fn gen_update_expression(
        &mut self,
        update: &UpdateExpression,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        match &*update.argument {
            Expression::Identifier(id) => {
                if let Some(entry) = self.lookup_var(&id.name) {
                    if entry.kind == VariableKind::Const {
                        return Err(format!(
                            "TypeError: Assignment to constant variable '{}'",
                            id.name
                        ));
                    }
                }
                match self.resolve_var(&id.name, ctx) {
                    Some(VarLocation::Local(slot)) => {
                        if let Some(entry) = self.lookup_var(&id.name) {
                            if entry.kind != VariableKind::Var && !entry.initialized {
                                self.emit(Opcode::CheckTdz);
                                self.emit_u16(slot);
                            }
                        }
                        let op = match update.op {
                            UpdateOp::Increment => Opcode::IncLocal,
                            UpdateOp::Decrement => Opcode::DecLocal,
                        };

                        if update.prefix {
                            self.emit(op);
                            self.emit_u16(slot);
                            if self.is_global_var(&id.name) {
                                let atom = ctx.intern(&id.name);
                                let idx = self.add_constant(JSValue::new_string(atom));
                                self.emit(Opcode::SetGlobal);
                                self.emit_u32(idx);
                                self.emit_u16(slot);
                            }
                            Ok(slot)
                        } else {
                            let old_val = self.alloc_register();
                            self.emit(Opcode::Move);
                            self.emit_u16(old_val);
                            self.emit_u16(slot);
                            self.emit(Opcode::Pos);
                            self.emit_u16(old_val);
                            self.emit_u16(old_val);
                            self.emit(op);
                            self.emit_u16(slot);
                            if self.is_global_var(&id.name) {
                                let atom = ctx.intern(&id.name);
                                let idx = self.add_constant(JSValue::new_string(atom));
                                self.emit(Opcode::SetGlobal);
                                self.emit_u32(idx);
                                self.emit_u16(slot);
                            }
                            Ok(old_val)
                        }
                    }
                    Some(VarLocation::Upvalue(slot)) => {
                        let temp = self.alloc_register();
                        self.emit(Opcode::GetUpvalue);
                        self.emit_u16(temp);
                        self.emit_u16(slot);

                        if let Some(entry) = self.lookup_var(&id.name) {
                            if entry.kind != VariableKind::Var && !entry.initialized {
                                self.emit(Opcode::CheckTdz);
                                self.emit_u16(temp);
                            }
                        }

                        let op = match update.op {
                            UpdateOp::Increment => Opcode::IncLocal,
                            UpdateOp::Decrement => Opcode::DecLocal,
                        };

                        if update.prefix {
                            self.emit(op);
                            self.emit_u16(temp);
                            self.emit(Opcode::SetUpvalue);
                            self.emit_u16(slot);
                            self.emit_u16(temp);
                            Ok(temp)
                        } else {
                            let old_val = self.alloc_register();
                            self.emit(Opcode::Move);
                            self.emit_u16(old_val);
                            self.emit_u16(temp);
                            self.emit(op);
                            self.emit_u16(temp);
                            self.emit(Opcode::SetUpvalue);
                            self.emit_u16(slot);
                            self.emit_u16(temp);
                            Ok(old_val)
                        }
                    }
                    None => {
                        let atom = ctx.intern(&id.name);
                        let const_idx = self.add_constant(JSValue::new_string(atom));
                        let r = self.alloc_register();
                        self.emit(Opcode::GetGlobal);
                        self.emit_u16(r);
                        self.emit_u32(const_idx);
                        let op = match update.op {
                            UpdateOp::Increment => Opcode::IncLocal,
                            UpdateOp::Decrement => Opcode::DecLocal,
                        };
                        if update.prefix {
                            self.emit(op);
                            self.emit_u16(r);
                            self.emit(Opcode::SetGlobal);
                            self.emit_u32(const_idx);
                            self.emit_u16(r);
                            Ok(r)
                        } else {
                            let old = self.alloc_register();
                            self.emit(Opcode::Move);
                            self.emit_u16(old);
                            self.emit_u16(r);
                            self.emit(op);
                            self.emit_u16(r);
                            self.emit(Opcode::SetGlobal);
                            self.emit_u32(const_idx);
                            self.emit_u16(r);
                            self.free_register(r);
                            Ok(old)
                        }
                    }
                }
            }
            Expression::MemberExpression(member) => {
                let obj = self.gen_expression(&member.object, ctx)?;
                let op = match update.op {
                    UpdateOp::Increment => Opcode::IncLocal,
                    UpdateOp::Decrement => Opcode::DecLocal,
                };

                match &member.property {
                    MemberProperty::Identifier(name) => {
                        let atom = ctx.intern(name);
                        let tmp = self.alloc_register();
                        self.emit(Opcode::GetNamedProp);
                        self.emit_u16(tmp);
                        self.emit_u16(obj);
                        self.emit_u32(atom.0);

                        if update.prefix {
                            self.emit(op);
                            self.emit_u16(tmp);
                            self.emit(Opcode::SetNamedProp);
                            self.emit_u16(obj);
                            self.emit_u16(tmp);
                            self.emit_u32(atom.0);
                            self.free_register(obj);
                            Ok(tmp)
                        } else {
                            let old_val = self.alloc_register();
                            self.emit(Opcode::Move);
                            self.emit_u16(old_val);
                            self.emit_u16(tmp);
                            self.emit(Opcode::Pos);
                            self.emit_u16(old_val);
                            self.emit_u16(old_val);
                            self.emit(op);
                            self.emit_u16(tmp);
                            self.emit(Opcode::SetNamedProp);
                            self.emit_u16(obj);
                            self.emit_u16(tmp);
                            self.emit_u32(atom.0);
                            self.free_register(obj);
                            self.free_register(tmp);
                            Ok(old_val)
                        }
                    }
                    MemberProperty::Computed(expr) => {
                        let key = self.gen_expression(expr, ctx)?;

                        let tmp = self.alloc_register();
                        self.emit(Opcode::GetField);
                        self.emit_u16(tmp);
                        self.emit_u16(obj);
                        self.emit_u16(key);

                        if update.prefix {
                            self.emit(op);
                            self.emit_u16(tmp);
                            self.emit(Opcode::SetProp);
                            self.emit_u16(obj);
                            self.emit_u16(key);
                            self.emit_u16(tmp);
                            self.free_register(obj);
                            self.free_register(key);
                            Ok(tmp)
                        } else {
                            let old_val = self.alloc_register();
                            self.emit(Opcode::Move);
                            self.emit_u16(old_val);
                            self.emit_u16(tmp);
                            self.emit(op);
                            self.emit_u16(tmp);
                            self.emit(Opcode::SetField);
                            self.emit_u16(obj);
                            self.emit_u16(key);
                            self.emit_u16(tmp);
                            self.free_register(obj);
                            self.free_register(key);
                            self.free_register(tmp);
                            Ok(old_val)
                        }
                    }
                    MemberProperty::PrivateIdentifier(_) => {
                        self.free_register(obj);
                        let r = self.alloc_register();
                        self.emit(Opcode::LoadUndefined);
                        self.emit_u16(r);
                        Ok(r)
                    }
                }
            }
            _ => {
                let r = self.alloc_register();
                self.emit(Opcode::LoadUndefined);
                self.emit_u16(r);
                Ok(r)
            }
        }
    }

    fn assign_op_to_reg_opcode(op: &AssignOp) -> Option<Opcode> {
        match op {
            AssignOp::AddAssign => Some(Opcode::Add),
            AssignOp::SubAssign => Some(Opcode::Sub),
            AssignOp::MulAssign => Some(Opcode::Mul),
            AssignOp::DivAssign => Some(Opcode::Div),
            AssignOp::ModAssign => Some(Opcode::Mod),
            AssignOp::PowAssign => Some(Opcode::Pow),
            AssignOp::BitAndAssign => Some(Opcode::BitAnd),
            AssignOp::BitOrAssign => Some(Opcode::BitOr),
            AssignOp::BitXorAssign => Some(Opcode::BitXor),
            AssignOp::ShlAssign => Some(Opcode::Shl),
            AssignOp::ShrAssign => Some(Opcode::Shr),
            AssignOp::UShrAssign => Some(Opcode::UShr),
            _ => None,
        }
    }

    fn gen_assignment_expression(
        &mut self,
        assign: &AssignmentExpression,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        let is_compound = Self::assign_op_to_reg_opcode(&assign.op);

        match &assign.left {
            AssignmentTarget::Identifier(name) => {
                if self.is_strict && Self::is_strict_reserved_word(name) {
                    return Err(format!(
                        "SyntaxError: Assignment to '{}' in strict mode is not allowed",
                        name
                    ));
                }
                if let Some(entry) = self.lookup_var(name) {
                    if entry.kind == VariableKind::Const {
                        return Err(format!(
                            "TypeError: Assignment to constant variable '{}'",
                            name
                        ));
                    }
                }

                if matches!(
                    assign.op,
                    AssignOp::LogicalAndAssign
                        | AssignOp::LogicalOrAssign
                        | AssignOp::NullishAssign
                ) {
                    let resolved = self.resolve_var(name, ctx);
                    let global_atom = ctx.intern(name);
                    let global_idx = self.add_constant(JSValue::new_string(global_atom));

                    let current = match resolved {
                        Some(VarLocation::Local(slot)) => slot,
                        Some(VarLocation::Upvalue(slot)) => {
                            let tmp = self.alloc_register();
                            self.emit(Opcode::GetUpvalue);
                            self.emit_u16(tmp);
                            self.emit_u16(slot);
                            tmp
                        }
                        None => {
                            let tmp = self.alloc_register();
                            self.emit(Opcode::GetGlobal);
                            self.emit_u16(tmp);
                            self.emit_u32(global_idx);
                            tmp
                        }
                    };

                    let jump_to_end = match assign.op {
                        AssignOp::LogicalAndAssign => {
                            self.emit(Opcode::JumpIfNot);
                            self.emit_u16(current);
                            let patch = self.code.len();
                            self.emit_i32(0);
                            patch
                        }
                        AssignOp::LogicalOrAssign => {
                            self.emit(Opcode::JumpIf);
                            self.emit_u16(current);
                            let patch = self.code.len();
                            self.emit_i32(0);
                            patch
                        }
                        AssignOp::NullishAssign => {
                            let null_reg = self.alloc_register();
                            self.emit(Opcode::LoadNull);
                            self.emit_u16(null_reg);
                            let cmp_reg = self.alloc_register();
                            self.emit(Opcode::StrictEq);
                            self.emit_u16(cmp_reg);
                            self.emit_u16(current);
                            self.emit_u16(null_reg);
                            self.emit(Opcode::JumpIf);
                            self.emit_u16(cmp_reg);
                            let jump_to_assign = self.code.len();
                            self.emit_i32(0);

                            let undef_reg = self.alloc_register();
                            self.emit(Opcode::LoadUndefined);
                            self.emit_u16(undef_reg);
                            self.emit(Opcode::StrictEq);
                            self.emit_u16(cmp_reg);
                            self.emit_u16(current);
                            self.emit_u16(undef_reg);
                            self.emit(Opcode::JumpIfNot);
                            self.emit_u16(cmp_reg);
                            let jump_to_end = self.code.len();
                            self.emit_i32(0);

                            let assign_pos = self.code.len();
                            self.patch_jump(jump_to_assign, assign_pos);

                            self.free_register(undef_reg);
                            self.free_register(cmp_reg);
                            self.free_register(null_reg);
                            jump_to_end
                        }
                        _ => unreachable!(),
                    };

                    let rhs = self.gen_expression(&assign.right, ctx)?;
                    if current != rhs {
                        self.emit(Opcode::Move);
                        self.emit_u16(current);
                        self.emit_u16(rhs);
                    }

                    match resolved {
                        Some(VarLocation::Local(slot)) => {
                            if slot != current {
                                self.emit(Opcode::Move);
                                self.emit_u16(slot);
                                self.emit_u16(current);
                            }
                            if self.is_global_var(name) {
                                let idx = self.add_constant(JSValue::new_string(ctx.intern(name)));
                                if self.is_eval {
                                    self.emit(Opcode::SetGlobalVar);
                                } else {
                                    self.emit(Opcode::SetGlobal);
                                }
                                self.emit_u32(idx);
                                self.emit_u16(slot);
                            }
                        }
                        Some(VarLocation::Upvalue(slot)) => {
                            self.emit(Opcode::SetUpvalue);
                            self.emit_u16(slot);
                            self.emit_u16(current);
                        }
                        None => {
                            let idx = self.add_constant(JSValue::new_string(ctx.intern(name)));
                            if self.is_eval {
                                self.emit(Opcode::SetGlobalVar);
                            } else {
                                self.emit(Opcode::SetGlobal);
                            }
                            self.emit_u32(idx);
                            self.emit_u16(current);
                        }
                    }

                    let end_pos = self.code.len();
                    self.patch_jump(jump_to_end, end_pos);
                    if current != rhs {
                        self.free_register(rhs);
                    }
                    return Ok(current);
                }

                let val = self.gen_expression_with_assign_name(&assign.right, name, ctx)?;
                match self.resolve_var(name, ctx) {
                    Some(VarLocation::Local(slot)) => {
                        if let Some(opcode) = is_compound {
                            self.emit(opcode);
                            self.emit_u16(slot);
                            self.emit_u16(slot);
                            self.emit_u16(val);
                        } else if slot != val {
                            self.emit(Opcode::Move);
                            self.emit_u16(slot);
                            self.emit_u16(val);
                        }

                        if self.is_global_var(name) {
                            let atom = ctx.intern(name);
                            let idx = self.add_constant(JSValue::new_string(atom));
                            if self.is_eval {
                                self.emit(Opcode::SetGlobalVar);
                            } else {
                                self.emit(Opcode::SetGlobal);
                            }
                            self.emit_u32(idx);
                            self.emit_u16(slot);
                        }
                        Ok(slot)
                    }
                    Some(VarLocation::Upvalue(slot)) => {
                        if let Some(opcode) = is_compound {
                            let tmp = self.alloc_register();
                            self.emit(Opcode::GetUpvalue);
                            self.emit_u16(tmp);
                            self.emit_u16(slot);
                            self.emit(opcode);
                            self.emit_u16(tmp);
                            self.emit_u16(tmp);
                            self.emit_u16(val);
                            self.emit(Opcode::SetUpvalue);
                            self.emit_u16(slot);
                            self.emit_u16(tmp);
                            self.free_register(tmp);
                        } else {
                            self.emit(Opcode::SetUpvalue);
                            self.emit_u16(slot);
                            self.emit_u16(val);
                        }
                        Ok(val)
                    }
                    None => {
                        let atom = ctx.intern(name);
                        let idx = self.add_constant(JSValue::new_string(atom));
                        if let Some(opcode) = is_compound {
                            let tmp = self.alloc_register();
                            self.emit(Opcode::GetGlobal);
                            self.emit_u16(tmp);
                            self.emit_u32(idx);
                            if !self.suppress_ref_error {
                                self.emit(Opcode::CheckRef);
                                self.emit_u16(tmp);
                                self.emit_u32(idx);
                            }
                            self.emit(opcode);
                            self.emit_u16(tmp);
                            self.emit_u16(tmp);
                            self.emit_u16(val);
                            self.emit(Opcode::SetGlobal);
                            self.emit_u32(idx);
                            self.emit_u16(tmp);
                            self.free_register(tmp);
                            Ok(val)
                        } else {
                            self.emit(Opcode::SetGlobal);
                            self.emit_u32(idx);
                            self.emit_u16(val);
                            Ok(val)
                        }
                    }
                }
            }
            AssignmentTarget::MemberExpression(member) => {
                let obj = self.gen_expression(&member.object, ctx)?;
                let val = self.gen_expression(&assign.right, ctx)?;

                let mut result_reg = val;
                match &member.property {
                    MemberProperty::Identifier(name) => {
                        let atom = ctx.intern(name);
                        if let Some(opcode) = is_compound {
                            let tmp = self.alloc_register();
                            self.emit(Opcode::GetNamedProp);
                            self.emit_u16(tmp);
                            self.emit_u16(obj);
                            self.emit_u32(atom.0);
                            self.emit(opcode);
                            self.emit_u16(tmp);
                            self.emit_u16(tmp);
                            self.emit_u16(val);
                            self.emit(Opcode::SetNamedProp);
                            self.emit_u16(obj);
                            self.emit_u16(tmp);
                            self.emit_u32(atom.0);
                            self.free_register(val);
                            result_reg = tmp;
                        } else {
                            self.emit(Opcode::SetNamedProp);
                            self.emit_u16(obj);
                            self.emit_u16(val);
                            self.emit_u32(atom.0);
                        }
                    }
                    MemberProperty::Computed(expr) => {
                        let key = self.gen_expression(expr, ctx)?;
                        if let Some(opcode) = is_compound {
                            let tmp = self.alloc_register();
                            self.emit(Opcode::GetField);
                            self.emit_u16(tmp);
                            self.emit_u16(obj);
                            self.emit_u16(key);
                            self.emit(opcode);
                            self.emit_u16(tmp);
                            self.emit_u16(tmp);
                            self.emit_u16(val);
                            self.emit(Opcode::SetField);
                            self.emit_u16(obj);
                            self.emit_u16(key);
                            self.emit_u16(tmp);
                            self.free_register(val);
                            result_reg = tmp;
                        } else {
                            self.emit(Opcode::SetField);
                            self.emit_u16(obj);
                            self.emit_u16(key);
                            self.emit_u16(val);
                        }
                        self.free_register(key);
                    }
                    MemberProperty::PrivateIdentifier(name) => {
                        let key_idx = self.intern_and_add_const(ctx, name);
                        let key = self.alloc_register();
                        self.emit(Opcode::LoadConst);
                        self.emit_u16(key);
                        self.emit_u32(key_idx);
                        if let Some(opcode) = is_compound {
                            let tmp = self.alloc_register();
                            self.emit(Opcode::GetPrivate);
                            self.emit_u16(tmp);
                            self.emit_u16(obj);
                            self.emit_u16(key);
                            self.emit(opcode);
                            self.emit_u16(tmp);
                            self.emit_u16(tmp);
                            self.emit_u16(val);
                            self.emit(Opcode::SetPrivate);
                            self.emit_u16(obj);
                            self.emit_u16(key);
                            self.emit_u16(tmp);
                            self.free_register(val);
                            result_reg = tmp;
                        } else {
                            self.emit(Opcode::SetPrivate);
                            self.emit_u16(obj);
                            self.emit_u16(key);
                            self.emit_u16(val);
                        }
                        self.free_register(key);
                    }
                }
                self.free_register(obj);
                Ok(result_reg)
            }
            _ => {
                let val = self.gen_expression(&assign.right, ctx)?;
                self.gen_assignment_target_destructure(&assign.left, val, ctx)?;
                Ok(val)
            }
        }
    }

    fn gen_assignment_target_destructure(
        &mut self,
        target: &AssignmentTarget,
        value_reg: u16,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        match target {
            AssignmentTarget::Identifier(name) => match self.resolve_var(name, ctx) {
                Some(VarLocation::Local(slot)) => {
                    if slot != value_reg {
                        self.emit(Opcode::Move);
                        self.emit_u16(slot);
                        self.emit_u16(value_reg);
                    }
                    if self.is_global_var(name) {
                        let global_idx = self.intern_and_add_const(ctx, name);
                        self.emit(Opcode::SetGlobal);
                        self.emit_u32(global_idx);
                        self.emit_u16(slot);
                    }
                    Ok(())
                }
                Some(VarLocation::Upvalue(slot)) => {
                    self.emit(Opcode::SetUpvalue);
                    self.emit_u16(slot);
                    self.emit_u16(value_reg);
                    Ok(())
                }
                None => {
                    let global_idx = self.intern_and_add_const(ctx, name);
                    self.emit(Opcode::SetGlobal);
                    self.emit_u32(global_idx);
                    self.emit_u16(value_reg);
                    Ok(())
                }
            },
            AssignmentTarget::MemberExpression(member) => {
                let obj = self.gen_expression(&member.object, ctx)?;
                match &member.property {
                    MemberProperty::Identifier(name) => {
                        let atom = ctx.intern(name);
                        self.emit(Opcode::SetNamedProp);
                        self.emit_u16(obj);
                        self.emit_u16(value_reg);
                        self.emit_u32(atom.0);
                    }
                    MemberProperty::Computed(expr) => {
                        let key = self.gen_expression(expr, ctx)?;
                        self.emit(Opcode::SetField);
                        self.emit_u16(obj);
                        self.emit_u16(key);
                        self.emit_u16(value_reg);
                        self.free_register(key);
                    }
                    MemberProperty::PrivateIdentifier(name) => {
                        let key_idx = self.intern_and_add_const(ctx, name);
                        let key = self.alloc_register();
                        self.emit(Opcode::LoadConst);
                        self.emit_u16(key);
                        self.emit_u32(key_idx);
                        self.emit(Opcode::SetPrivate);
                        self.emit_u16(obj);
                        self.emit_u16(key);
                        self.emit_u16(value_reg);
                        self.free_register(key);
                    }
                }
                self.free_register(obj);
                Ok(())
            }
            AssignmentTarget::ComputedMember(obj_expr, key_expr) => {
                let obj = self.gen_expression(obj_expr, ctx)?;
                let key = self.gen_expression(key_expr, ctx)?;
                self.emit(Opcode::SetField);
                self.emit_u16(obj);
                self.emit_u16(key);
                self.emit_u16(value_reg);
                self.free_register(key);
                self.free_register(obj);
                Ok(())
            }
            AssignmentTarget::ArrayPattern(arr) => {
                self.gen_array_destructuring_target(arr, value_reg, ctx)
            }
            AssignmentTarget::ObjectPattern(obj) => {
                self.gen_object_destructuring_target(obj, value_reg, ctx)
            }
            AssignmentTarget::AssignmentPattern(ap) => {
                self.gen_default_value_assignment(&ap.right, value_reg, ctx)?;
                self.gen_assignment_target_destructure(&ap.left, value_reg, ctx)
            }
            AssignmentTarget::RestElement(_) => {
                Err("Rest element not valid in assignment target".to_string())
            }
            AssignmentTarget::OptionalMemberExpression(_) => {
                Err("Optional member expression not valid in assignment target".to_string())
            }
        }
    }

    fn gen_array_destructuring_target(
        &mut self,
        arr: &ArrayPattern,
        value_reg: u16,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        let iter_reg = self.alloc_register();
        self.emit(Opcode::GetIterator);
        self.emit_u16(iter_reg);
        self.emit_u16(value_reg);

        let len = arr.elements.len();
        for (i, elem) in arr.elements.iter().enumerate() {
            if i == len - 1 {
                if let Some(PatternElement::RestElement(rest)) = elem {
                    let rest_reg = self.alloc_register();
                    self.emit(Opcode::NewArray);
                    self.emit_u16(rest_reg);
                    self.emit_u16(0);
                    let loop_start = self.code.len();
                    let val_for_rest = self.alloc_register();
                    let done_for_rest = self.alloc_register();
                    self.emit(Opcode::IteratorNext);
                    self.emit_u16(val_for_rest);
                    self.emit_u16(done_for_rest);
                    self.emit_u16(iter_reg);
                    self.emit(Opcode::JumpIf);
                    self.emit_u16(done_for_rest);
                    let exit_patch = self.code.len();
                    self.emit_i32(0);
                    self.free_register(done_for_rest);
                    self.emit(Opcode::ArrayPush);
                    self.emit_u16(rest_reg);
                    self.emit_u16(val_for_rest);
                    self.free_register(val_for_rest);
                    self.emit_backward_jump(loop_start);
                    let end_pos = self.code.len();
                    self.patch_jump(exit_patch, end_pos);
                    self.gen_assignment_target_destructure(&rest.argument, rest_reg, ctx)?;
                    self.free_register(rest_reg);
                    self.free_register(iter_reg);
                    return Ok(());
                }
            }

            let val_reg = self.alloc_register();
            let done_reg = self.alloc_register();
            self.emit(Opcode::IteratorNext);
            self.emit_u16(val_reg);
            self.emit_u16(done_reg);
            self.emit_u16(iter_reg);

            self.emit(Opcode::JumpIfNot);
            self.emit_u16(done_reg);
            let not_done_skip = self.code.len();
            self.emit_i32(0);

            let undef_reg = self.alloc_register();
            self.emit(Opcode::LoadUndefined);
            self.emit_u16(undef_reg);
            self.emit(Opcode::Move);
            self.emit_u16(val_reg);
            self.emit_u16(undef_reg);
            self.free_register(undef_reg);

            let after_undef = self.code.len();
            self.patch_jump(not_done_skip, after_undef);
            self.free_register(done_reg);

            if let Some(elem) = elem {
                match elem {
                    PatternElement::Pattern(target) => {
                        self.gen_assignment_target_destructure(target, val_reg, ctx)?;
                    }
                    PatternElement::RestElement(_) => {
                        return Err("rest element not at end".to_string());
                    }
                    PatternElement::AssignmentPattern(ap) => {
                        self.gen_default_value_assignment(&ap.right, val_reg, ctx)?;
                        self.gen_assignment_target_destructure(&ap.left, val_reg, ctx)?;
                    }
                }
            }
            self.free_register(val_reg);
        }
        self.free_register(iter_reg);
        Ok(())
    }

    fn gen_object_destructuring_target(
        &mut self,
        obj: &ObjectPattern,
        value_reg: u16,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        for prop in &obj.properties {
            match prop {
                ObjectPatternProperty::Property {
                    key,
                    value,
                    computed,
                    ..
                } => {
                    let key_reg = if *computed {
                        match key {
                            PropertyKey::Computed(expr) => self.gen_expression(expr, ctx)?,
                            _ => return Err("Expected computed key".to_string()),
                        }
                    } else {
                        match key {
                            PropertyKey::Identifier(k)
                            | PropertyKey::Literal(Literal::String(k, _)) => {
                                let idx = self.intern_and_add_const(ctx, k);
                                let kreg = self.alloc_register();
                                self.emit(Opcode::LoadConst);
                                self.emit_u16(kreg);
                                self.emit_u32(idx);
                                kreg
                            }
                            PropertyKey::Literal(Literal::Number(n)) => {
                                let kreg = self.alloc_register();
                                if *n >= i32::MIN as f64
                                    && *n <= i32::MAX as f64
                                    && n.fract() == 0.0
                                {
                                    self.emit_int_const(kreg, *n as i32);
                                } else {
                                    let cidx = self.add_constant(JSValue::new_float(*n));
                                    self.emit(Opcode::LoadConst);
                                    self.emit_u16(kreg);
                                    self.emit_u32(cidx);
                                }
                                kreg
                            }
                            _ => return Err("Invalid property key".to_string()),
                        }
                    };
                    let elem_reg = self.alloc_register();
                    self.emit(Opcode::GetProp);
                    self.emit_u16(elem_reg);
                    self.emit_u16(value_reg);
                    self.emit_u16(key_reg);
                    self.free_register(key_reg);
                    self.gen_assignment_target_destructure(value, elem_reg, ctx)?;
                    self.free_register(elem_reg);
                }
                ObjectPatternProperty::RestElement(rest) => {
                    let rest_reg = self.alloc_register();
                    self.emit(Opcode::NewObject);
                    self.emit_u16(rest_reg);
                    self.emit(Opcode::ObjectSpread);
                    self.emit_u16(rest_reg);
                    self.emit_u16(value_reg);
                    self.gen_assignment_target_destructure(&rest.argument, rest_reg, ctx)?;
                    self.free_register(rest_reg);
                }
            }
        }
        Ok(())
    }

    fn gen_default_value_assignment(
        &mut self,
        default_expr: &Expression,
        value_reg: u16,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        let cmp_reg = self.alloc_register();
        self.emit(Opcode::LoadUndefined);
        self.emit_u16(cmp_reg);

        self.emit(Opcode::StrictNeq);
        self.emit_u16(cmp_reg);
        self.emit_u16(value_reg);
        self.emit_u16(cmp_reg);

        self.emit(Opcode::JumpIf);
        self.emit_u16(cmp_reg);
        let skip_patch = self.code.len();
        self.emit_i32(0);
        self.free_register(cmp_reg);

        let default_reg = self.gen_expression(default_expr, ctx)?;
        if default_reg != value_reg {
            self.emit(Opcode::Move);
            self.emit_u16(value_reg);
            self.emit_u16(default_reg);
        }
        self.free_register(default_reg);

        let after_pos = self.code.len();
        self.patch_jump(skip_patch, after_pos);

        Ok(())
    }

    fn gen_conditional_expression(
        &mut self,
        cond: &ConditionalExpression,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        if self.p2_fold_profile.allow_ternary_bool() {
            if let Expression::Literal(Literal::Boolean(test_bool)) = &*cond.test {
                if *test_bool {
                    return self.gen_expression(&cond.consequent, ctx);
                }
                return self.gen_expression(&cond.alternate, ctx);
            }
        }

        if self.p2_fold_profile.allow_aggressive_truthy() {
            if let Expression::Literal(lit) = &*cond.test {
                if let Some(truthy) = Self::literal_truthiness(lit) {
                    if truthy {
                        return self.gen_expression(&cond.consequent, ctx);
                    }
                    return self.gen_expression(&cond.alternate, ctx);
                }
            }
        }

        let test = self.gen_expression(&cond.test, ctx)?;

        if self.opt_branch_result_prealloc {
            let result = if self.reserved_slots.contains(&test) {
                let dst = self.alloc_register();
                self.emit(Opcode::Move);
                self.emit_u16(dst);
                self.emit_u16(test);
                dst
            } else {
                test
            };

            self.emit(Opcode::JumpIfNot);
            self.emit_u16(test);
            let else_jump = self.code.len();
            self.emit_i32(0);

            let cons = self.gen_expression(&cond.consequent, ctx)?;
            if cons != result {
                self.emit(Opcode::Move);
                self.emit_u16(result);
                self.emit_u16(cons);
                self.free_register(cons);
            }

            self.emit(Opcode::Jump);
            let end_jump = self.code.len();
            self.emit_i32(0);

            let else_pos = self.code.len();
            self.patch_jump(else_jump, else_pos);

            let alt = self.gen_expression(&cond.alternate, ctx)?;
            if alt != result {
                self.emit(Opcode::Move);
                self.emit_u16(result);
                self.emit_u16(alt);
                self.free_register(alt);
            }

            let end_pos = self.code.len();
            self.patch_jump(end_jump, end_pos);
            return Ok(result);
        }

        self.emit(Opcode::JumpIfNot);
        self.emit_u16(test);
        let else_jump = self.code.len();
        self.emit_i32(0);

        let cons = self.gen_expression(&cond.consequent, ctx)?;

        let result = if self.reserved_slots.contains(&test) && cons != test {
            let dst = self.alloc_register();
            self.emit(Opcode::Move);
            self.emit_u16(dst);
            self.emit_u16(cons);
            self.free_register(cons);
            dst
        } else if cons != test {
            self.emit(Opcode::Move);
            self.emit_u16(test);
            self.emit_u16(cons);
            self.free_register(cons);
            test
        } else {
            test
        };

        self.emit(Opcode::Jump);
        let end_jump = self.code.len();
        self.emit_i32(0);

        let else_pos = self.code.len();
        self.patch_jump(else_jump, else_pos);

        let alt = self.gen_expression(&cond.alternate, ctx)?;
        if self.reserved_slots.contains(&test) && alt != result && alt != test {
            self.emit(Opcode::Move);
            self.emit_u16(result);
            self.emit_u16(alt);
            self.free_register(alt);
        } else if alt != result && alt != test {
            self.emit(Opcode::Move);
            self.emit_u16(result);
            self.emit_u16(alt);
            self.free_register(alt);
        }

        let end_pos = self.code.len();
        self.patch_jump(end_jump, end_pos);

        Ok(result)
    }

    fn gen_member_expression(
        &mut self,
        member: &MemberExpression,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        let obj = self.gen_expression(&member.object, ctx)?;

        if matches!(&*member.object, Expression::Super) && !self.in_static_method {
            let proto_key = self.alloc_register();
            let proto_key_idx = self.add_constant(JSValue::new_string(ctx.common_atoms.prototype));
            self.emit(Opcode::LoadConst);
            self.emit_u16(proto_key);
            self.emit_u32(proto_key_idx);
            self.emit(Opcode::GetProp);
            self.emit_u16(obj);
            self.emit_u16(obj);
            self.emit_u16(proto_key);
            self.free_register(proto_key);
        }
        let dst = self.alloc_register();
        match &member.property {
            MemberProperty::Identifier(name) => {
                let atom = ctx.intern(name);
                self.emit(Opcode::GetNamedProp);
                self.emit_u16(dst);
                self.emit_u16(obj);
                self.emit_u32(atom.0);
            }
            MemberProperty::Computed(expr) => {
                let key = self.gen_expression(expr, ctx)?;
                self.emit(Opcode::GetField);
                self.emit_u16(dst);
                self.emit_u16(obj);
                self.emit_u16(key);
            }
            MemberProperty::PrivateIdentifier(name) => {
                let key_idx = self.intern_and_add_const(ctx, name);
                let key = self.alloc_register();
                self.emit(Opcode::LoadConst);
                self.emit_u16(key);
                self.emit_u32(key_idx);
                self.emit(Opcode::GetPrivate);
                self.emit_u16(dst);
                self.emit_u16(obj);
                self.emit_u16(key);
                self.free_register(key);
            }
        }
        self.free_register(obj);
        Ok(dst)
    }

    fn gen_call_expression(
        &mut self,
        call: &CallExpression,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        if self.opt_tiny_inline {
            if let Some(reg) = self.try_emit_tiny_inline_call(call, ctx)? {
                return Ok(reg);
            }
        }

        if self.opt_fused_getprop_call {
            if let Expression::MemberExpression(member) = &*call.callee {
                if !call
                    .arguments
                    .iter()
                    .any(|arg| matches!(arg, Argument::Spread(_)))
                {
                    if let MemberProperty::Identifier(name) = &member.property {
                        let is_namespace = if let Expression::Identifier(ident) = &*member.object {
                            matches!(
                                ident.name.as_str(),
                                "Math"
                                    | "Object"
                                    | "JSON"
                                    | "Number"
                                    | "Boolean"
                                    | "String"
                                    | "Array"
                                    | "RegExp"
                                    | "Error"
                                    | "Function"
                                    | "Promise"
                                    | "Symbol"
                                    | "Map"
                                    | "Set"
                                    | "WeakMap"
                                    | "WeakSet"
                                    | "BigInt"
                                    | "Date"
                                    | "Reflect"
                                    | "Proxy"
                                    | "console"
                            )
                        } else {
                            false
                        };

                        if !is_namespace {
                            let obj_reg = self.gen_expression(&member.object, ctx)?;
                            let atom = ctx.intern(name);

                            let mut arg_regs = Vec::new();
                            for arg in &call.arguments {
                                if let Argument::Expression(expr) = arg {
                                    arg_regs.push(self.gen_expression(expr, ctx)?);
                                }
                            }

                            let dst = self.alloc_register();
                            self.emit(Opcode::CallNamedMethod);
                            self.emit_u16(dst);
                            self.emit_u16(obj_reg);
                            self.emit_u32(atom.0);
                            self.emit_u16(arg_regs.len() as u16);
                            for r in &arg_regs {
                                self.emit_u16(*r);
                            }

                            self.free_register(obj_reg);
                            for r in arg_regs {
                                self.free_register(r);
                            }
                            return Ok(dst);
                        }
                    }
                }
            }
        }

        let is_self_recursive_call = matches!(&*call.callee, Expression::Identifier(id)
            if self.current_function_name.as_ref().map_or(false, |n| n == &id.name)
                && self.lookup_var_slot(&id.name).is_none()
                && !self.upvalue_slots.contains_key(&id.name));

        let is_namespace_call = if let Expression::MemberExpression(member) = &*call.callee {
            if let Expression::Identifier(ident) = &*member.object {
                matches!(
                    ident.name.as_str(),
                    "Math"
                        | "Object"
                        | "JSON"
                        | "Number"
                        | "Boolean"
                        | "String"
                        | "Array"
                        | "RegExp"
                        | "Error"
                        | "Function"
                        | "Promise"
                        | "Symbol"
                        | "Map"
                        | "Set"
                        | "WeakMap"
                        | "WeakSet"
                        | "BigInt"
                        | "Date"
                        | "Reflect"
                        | "Proxy"
                        | "console"
                )
            } else {
                false
            }
        } else {
            false
        };

        let has_spread = call
            .arguments
            .iter()
            .any(|arg| matches!(arg, Argument::Spread(_)));

        if is_self_recursive_call && !has_spread {
            let mut arg_regs = Vec::new();
            for arg in &call.arguments {
                if let Argument::Expression(expr) = arg {
                    arg_regs.push(self.gen_expression(expr, ctx)?);
                }
            }
            let dst = self.alloc_register();
            if arg_regs.len() == 1 {
                self.emit(Opcode::CallCurrent1);
                self.emit_u16(dst);
                self.emit_u16(arg_regs[0]);
            } else {
                self.emit(Opcode::CallCurrent);
                self.emit_u16(dst);
                self.emit_u16(arg_regs.len() as u16);
                for r in &arg_regs {
                    self.emit_u16(*r);
                }
            }
            for r in arg_regs {
                self.free_register(r);
            }
            return Ok(dst);
        }

        if !has_spread {
            if let Expression::MemberExpression(member) = &*call.callee {
                if let Expression::Identifier(ident) = &*member.object {
                    if ident.name == "Math" {
                        if let MemberProperty::Identifier(name) = &member.property {
                            let math_op = match name.as_str() {
                                "sin" if call.arguments.len() == 1 => Some(Opcode::MathSin),
                                "cos" if call.arguments.len() == 1 => Some(Opcode::MathCos),
                                "sqrt" if call.arguments.len() == 1 => Some(Opcode::MathSqrt),
                                "abs" if call.arguments.len() == 1 => Some(Opcode::MathAbs),
                                "floor" if call.arguments.len() == 1 => Some(Opcode::MathFloor),
                                "ceil" if call.arguments.len() == 1 => Some(Opcode::MathCeil),
                                "round" if call.arguments.len() == 1 => Some(Opcode::MathRound),
                                "pow" if call.arguments.len() == 2 => Some(Opcode::MathPow),
                                "min" if call.arguments.len() == 2 => Some(Opcode::MathMin),
                                "max" if call.arguments.len() == 2 => Some(Opcode::MathMax),
                                _ => None,
                            };
                            if let Some(op) = math_op {
                                let mut arg_regs = Vec::new();
                                for arg in &call.arguments {
                                    if let Argument::Expression(expr) = arg {
                                        arg_regs.push(self.gen_expression(expr, ctx)?);
                                    }
                                }
                                let dst = self.alloc_register();
                                self.emit(op);
                                self.emit_u16(dst);
                                for r in &arg_regs {
                                    self.emit_u16(*r);
                                }
                                for r in arg_regs {
                                    self.free_register(r);
                                }
                                return Ok(dst);
                            }
                        }
                    }
                }
            }
        }

        let (func, obj, namespace_obj_reg) = match &*call.callee {
            Expression::MemberExpression(member) => {
                let obj_reg = self.gen_expression(&member.object, ctx)?;
                let func_reg = match &member.property {
                    MemberProperty::Identifier(name) => {
                        let atom = ctx.intern(name);
                        let f = self.alloc_register();
                        self.emit(Opcode::GetNamedProp);
                        self.emit_u16(f);
                        self.emit_u16(obj_reg);
                        self.emit_u32(atom.0);
                        f
                    }
                    MemberProperty::PrivateIdentifier(name) => {
                        let key_idx = self.intern_and_add_const(ctx, name);
                        let key = self.alloc_register();
                        self.emit(Opcode::LoadConst);
                        self.emit_u16(key);
                        self.emit_u32(key_idx);
                        let f = self.alloc_register();
                        self.emit(Opcode::GetPrivate);
                        self.emit_u16(f);
                        self.emit_u16(obj_reg);
                        self.emit_u16(key);
                        self.free_register(key);
                        f
                    }
                    MemberProperty::Computed(expr) => {
                        let key = self.gen_expression(expr, ctx)?;
                        let f = self.alloc_register();
                        self.emit(Opcode::GetField);
                        self.emit_u16(f);
                        self.emit_u16(obj_reg);
                        self.emit_u16(key);
                        self.free_register(key);
                        f
                    }
                };
                if is_namespace_call {
                    (func_reg, None, Some(obj_reg))
                } else {
                    (func_reg, Some(obj_reg), None)
                }
            }
            other => {
                let func_reg = self.gen_expression(other, ctx)?;
                (func_reg, None, None)
            }
        };

        let dst = self.alloc_register();
        if has_spread {
            let arr_reg = self.alloc_register();
            self.emit(Opcode::NewArray);
            self.emit_u16(arr_reg);
            self.emit_u16(0);
            for arg in &call.arguments {
                match arg {
                    Argument::Expression(expr) => {
                        let val = self.gen_expression(expr, ctx)?;
                        self.emit(Opcode::ArrayPush);
                        self.emit_u16(arr_reg);
                        self.emit_u16(val);
                        self.free_register(val);
                    }
                    Argument::Spread(expr) => {
                        let val = self.gen_expression(expr, ctx)?;
                        self.emit(Opcode::ArrayExtend);
                        self.emit_u16(arr_reg);
                        self.emit_u16(val);
                        self.free_register(val);
                    }
                }
            }
            if let Some(obj_reg) = obj {
                self.emit(Opcode::CallMethodSpread);
                self.emit_u16(dst);
                self.emit_u16(obj_reg);
                self.emit_u16(func);
                self.emit_u16(arr_reg);
            } else {
                self.emit(Opcode::CallSpread);
                self.emit_u16(dst);
                self.emit_u16(func);
                self.emit_u16(arr_reg);
            }
            if let Some(obj_reg) = obj {
                self.free_register(obj_reg);
            }
            if let Some(reg) = namespace_obj_reg {
                self.free_register(reg);
            }
            self.free_register(func);
            self.free_register(arr_reg);
        } else {
            let mut arg_regs = Vec::new();
            for arg in &call.arguments {
                match arg {
                    Argument::Expression(expr) => {
                        arg_regs.push(self.gen_expression(expr, ctx)?);
                    }
                    _ => {}
                }
            }
            if let Some(obj_reg) = obj {
                self.emit(Opcode::CallMethod);
                self.emit_u16(dst);
                self.emit_u16(obj_reg);
                self.emit_u16(func);
                self.emit_u16(arg_regs.len() as u16);
                for r in &arg_regs {
                    self.emit_u16(*r);
                }
            } else {
                match arg_regs.len() {
                    0 => {
                        self.emit(Opcode::Call0);
                        self.emit_u16(dst);
                        self.emit_u16(func);
                    }
                    1 => {
                        self.emit(Opcode::Call1);
                        self.emit_u16(dst);
                        self.emit_u16(func);
                        self.emit_u16(arg_regs[0]);
                    }
                    2 => {
                        if self.opt_call23 {
                            self.emit(Opcode::Call2);
                            self.emit_u16(dst);
                            self.emit_u16(func);
                            self.emit_u16(arg_regs[0]);
                            self.emit_u16(arg_regs[1]);
                        } else {
                            self.emit(Opcode::Call);
                            self.emit_u16(dst);
                            self.emit_u16(func);
                            self.emit_u16(2);
                            self.emit_u16(arg_regs[0]);
                            self.emit_u16(arg_regs[1]);
                        }
                    }
                    3 => {
                        if self.opt_call23 {
                            self.emit(Opcode::Call3);
                            self.emit_u16(dst);
                            self.emit_u16(func);
                            self.emit_u16(arg_regs[0]);
                            self.emit_u16(arg_regs[1]);
                            self.emit_u16(arg_regs[2]);
                        } else {
                            self.emit(Opcode::Call);
                            self.emit_u16(dst);
                            self.emit_u16(func);
                            self.emit_u16(3);
                            self.emit_u16(arg_regs[0]);
                            self.emit_u16(arg_regs[1]);
                            self.emit_u16(arg_regs[2]);
                        }
                    }
                    _ => {
                        self.emit(Opcode::Call);
                        self.emit_u16(dst);
                        self.emit_u16(func);
                        self.emit_u16(arg_regs.len() as u16);
                        for r in &arg_regs {
                            self.emit_u16(*r);
                        }
                    }
                }
            }
            if let Some(obj_reg) = obj {
                self.free_register(obj_reg);
            }
            if let Some(reg) = namespace_obj_reg {
                self.free_register(reg);
            }
            self.free_register(func);
            for r in arg_regs {
                self.free_register(r);
            }
        }
        Ok(dst)
    }

    fn gen_new_expression(
        &mut self,
        new: &NewExpression,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        let func = self.gen_expression(&new.callee, ctx)?;
        let has_spread = new
            .arguments
            .iter()
            .any(|arg| matches!(arg, Argument::Spread(_)));
        let dst = self.alloc_register();
        if has_spread {
            let arr_reg = self.alloc_register();
            self.emit(Opcode::NewArray);
            self.emit_u16(arr_reg);
            self.emit_u16(0);
            for arg in &new.arguments {
                match arg {
                    Argument::Expression(expr) => {
                        let val = self.gen_expression(expr, ctx)?;
                        self.emit(Opcode::ArrayPush);
                        self.emit_u16(arr_reg);
                        self.emit_u16(val);
                        self.free_register(val);
                    }
                    Argument::Spread(expr) => {
                        let val = self.gen_expression(expr, ctx)?;
                        self.emit(Opcode::ArrayExtend);
                        self.emit_u16(arr_reg);
                        self.emit_u16(val);
                        self.free_register(val);
                    }
                }
            }
            self.emit(Opcode::CallNewSpread);
            self.emit_u16(dst);
            self.emit_u16(func);
            self.emit_u16(arr_reg);
            self.free_register(func);
            self.free_register(arr_reg);
        } else {
            let mut arg_regs = Vec::new();
            for arg in &new.arguments {
                match arg {
                    Argument::Expression(expr) => {
                        arg_regs.push(self.gen_expression(expr, ctx)?);
                    }
                    _ => {}
                }
            }
            self.emit(Opcode::CallNew);
            self.emit_u16(dst);
            self.emit_u16(func);
            self.emit_u16(arg_regs.len() as u16);
            for r in &arg_regs {
                self.emit_u16(*r);
            }
            self.free_register(func);
            for r in arg_regs {
                self.free_register(r);
            }
        }
        Ok(dst)
    }

    fn gen_array_expression(
        &mut self,
        arr: &ArrayExpression,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        let dst = self.alloc_register();
        let count = arr.elements.len() as u16;
        self.emit(Opcode::NewArray);
        self.emit_u16(dst);
        self.emit_u16(count);

        let has_holes = arr.elements.iter().any(|e| e.is_none());
        let has_spread = arr
            .elements
            .iter()
            .any(|e| matches!(e, Some(ArrayElement::Spread(_))));

        if !has_holes && !has_spread {
            for elem in &arr.elements {
                if let Some(ArrayElement::Expression(expr)) = elem {
                    let val = self.gen_expression(expr, ctx)?;
                    self.emit(Opcode::ArrayPush);
                    self.emit_u16(dst);
                    self.emit_u16(val);
                    self.free_register(val);
                }
            }
            return Ok(dst);
        }

        let idx_reg = self.alloc_register();
        self.emit_int_const(idx_reg, 0);

        for elem in &arr.elements {
            match elem {
                Some(ArrayElement::Expression(expr)) => {
                    let val = self.gen_expression(expr, ctx)?;
                    self.emit(Opcode::SetField);
                    self.emit_u16(dst);
                    self.emit_u16(idx_reg);
                    self.emit_u16(val);
                    self.free_register(val);
                    self.emit(Opcode::IncLocal);
                    self.emit_u16(idx_reg);
                }
                None => {
                    self.emit(Opcode::IncLocal);
                    self.emit_u16(idx_reg);
                }
                Some(ArrayElement::Spread(expr)) => {
                    let src = self.gen_expression(expr, ctx)?;
                    self.emit(Opcode::ArrayExtend);
                    self.emit_u16(dst);
                    self.emit_u16(src);
                    self.free_register(src);

                    let len_key = self.alloc_register();
                    let len_atom = ctx.common_atoms.length;
                    let len_const = self.add_constant(JSValue::new_string(len_atom));
                    self.emit(Opcode::LoadConst);
                    self.emit_u16(len_key);
                    self.emit_u32(len_const);
                    self.emit(Opcode::GetProp);
                    self.emit_u16(idx_reg);
                    self.emit_u16(dst);
                    self.emit_u16(len_key);
                    self.free_register(len_key);
                }
            }
        }
        let len_atom = ctx.common_atoms.length;
        let len_const = self.add_constant(JSValue::new_string(len_atom));
        let len_key = self.alloc_register();
        self.emit(Opcode::LoadConst);
        self.emit_u16(len_key);
        self.emit_u32(len_const);
        self.emit(Opcode::SetProp);
        self.emit_u16(dst);
        self.emit_u16(len_key);
        self.emit_u16(idx_reg);
        self.free_register(len_key);
        self.free_register(idx_reg);
        Ok(dst)
    }

    fn gen_object_expression(
        &mut self,
        obj: &ObjectExpression,
        ctx: &mut JSContext,
    ) -> Result<u16, String> {
        let dst = self.alloc_register();
        self.emit(Opcode::NewObject);
        self.emit_u16(dst);

        for prop in &obj.properties {
            match prop {
                Property::Property {
                    key,
                    value,
                    getter,
                    setter,
                    ..
                } => {
                    if *getter || *setter {
                        let func_reg = self.gen_expression(value, ctx)?;
                        let key_reg = match key {
                            PropertyKey::Identifier(name)
                            | PropertyKey::Literal(Literal::String(name, _)) => {
                                let atom = ctx.intern(name);
                                let idx = self.add_constant(JSValue::new_string(atom));
                                let k = self.alloc_register();
                                self.emit(Opcode::LoadConst);
                                self.emit_u16(k);
                                self.emit_u32(idx);
                                k
                            }
                            PropertyKey::Computed(expr) => self.gen_expression(expr, ctx)?,
                            _ => {
                                self.free_register(func_reg);
                                continue;
                            }
                        };
                        let getter_reg = if *getter { func_reg } else { u16::MAX };
                        let setter_reg = if *setter { func_reg } else { u16::MAX };
                        self.emit(Opcode::DefineAccessor);
                        self.emit_u16(dst);
                        self.emit_u16(key_reg);
                        self.emit_u16(getter_reg);
                        self.emit_u16(setter_reg);
                        self.free_register(key_reg);
                        self.free_register(func_reg);
                        continue;
                    }
                    match key {
                        PropertyKey::Identifier(name)
                        | PropertyKey::Literal(Literal::String(name, _)) => {
                            let val = self.gen_expression(value, ctx)?;
                            let atom = ctx.intern(name);
                            self.emit(Opcode::SetNamedProp);
                            self.emit_u16(dst);
                            self.emit_u16(val);
                            self.emit_u32(atom.0);
                            self.free_register(val);
                        }
                        _ => {
                            let key_reg = match key {
                                PropertyKey::Literal(Literal::Number(n)) => {
                                    let k = self.alloc_register();
                                    let nv = *n;
                                    if nv >= i32::MIN as f64
                                        && nv <= i32::MAX as f64
                                        && nv.fract() == 0.0
                                    {
                                        self.emit_int_const(k, nv as i32);
                                    } else {
                                        let idx = self.add_constant(JSValue::new_float(nv));
                                        self.emit(Opcode::LoadConst);
                                        self.emit_u16(k);
                                        self.emit_u32(idx);
                                    }
                                    k
                                }
                                PropertyKey::Computed(expr) => self.gen_expression(expr, ctx)?,
                                _ => self.alloc_register(),
                            };
                            let val = self.gen_expression(value, ctx)?;
                            self.emit(Opcode::SetField);
                            self.emit_u16(dst);
                            self.emit_u16(key_reg);
                            self.emit_u16(val);
                            self.free_register(key_reg);
                            self.free_register(val);
                        }
                    }
                }
                Property::SpreadElement(expr) => {
                    let src = self.gen_expression(expr, ctx)?;
                    self.emit(Opcode::ObjectSpread);
                    self.emit_u16(dst);
                    self.emit_u16(src);
                    self.free_register(src);
                }
            }
        }
        Ok(dst)
    }

    fn gen_function_expression(
        &mut self,
        params: &[Parameter],
        body: &BlockStatement,
        name: Option<&str>,
        ctx: &mut JSContext,
        is_generator: bool,
        is_async: bool,
        is_arrow: bool,
    ) -> Result<u16, String> {
        let mut nested_codegen = self.nested_codegen();
        nested_codegen.is_arrow = is_arrow;
        let mut parent_vars: FxHashMap<String, ParentVar> = FxHashMap::default();
        for (scope_idx, scope) in self.scope_stack.iter().enumerate() {
            if self
                .scope_is_global
                .get(scope_idx)
                .copied()
                .unwrap_or(false)
            {
                continue;
            }
            for (name, entry) in scope {
                parent_vars.insert(
                    name.clone(),
                    ParentVar {
                        local_idx: entry.slot,
                        is_inherited: false,
                    },
                );
            }
        }
        for (name, &slot) in &self.upvalue_slots {
            if !parent_vars.contains_key(name) {
                parent_vars.insert(
                    name.clone(),
                    ParentVar {
                        local_idx: slot,
                        is_inherited: true,
                    },
                );
            }
        }
        let (bytecode, upvalues) = nested_codegen
            .compile_function_with_parent(params, body, ctx, parent_vars, name, false)
            .map_err(|e| format!("nested function compile error: {}", e))?;

        let func_name_atom = name.map(|n| ctx.atom_table_mut().intern(n).0).unwrap_or(0);
        let dst = if let Some(name) = name {
            self.lookup_var_slot(name)
                .unwrap_or_else(|| self.alloc_register())
        } else {
            self.alloc_register()
        };
        self.emit_embedded_function(
            &bytecode,
            &upvalues,
            func_name_atom,
            dst,
            is_generator,
            is_async,
        );
        Ok(dst)
    }

    fn gen_class_declaration(
        &mut self,
        class: &ClassDeclaration,
        ctx: &mut JSContext,
    ) -> Result<(), String> {
        let class_name = &class.name;
        let class_slot = self
            .lookup_var_slot(class_name)
            .unwrap_or_else(|| self.declare_var(class_name, VariableKind::Let));
        let class_name_atom =
            self.gen_class_body(class_slot, class_name, &class.super_class, &class.body, ctx)?;
        if self.is_global_scope() && !self.is_eval {
            let class_idx = self.add_constant(JSValue::new_string(class_name_atom));
            self.emit(Opcode::SetGlobal);
            self.emit_u32(class_idx);
            self.emit_u16(class_slot);
        }
        Ok(())
    }

    fn gen_class_body(
        &mut self,
        class_slot: u16,
        class_name: &str,
        super_class: &Option<Box<Expression>>,
        body: &ClassBody,
        ctx: &mut JSContext,
    ) -> Result<crate::runtime::atom::Atom, String> {
        let has_super = super_class.is_some();

        let ctor_method = body.elements.iter().find(|e| {
            if let ClassElement::Method(m) = e {
                m.kind == MethodKind::Constructor && !m.is_static
            } else {
                false
            }
        });

        let instance_fields: Vec<_> = body
            .elements
            .iter()
            .filter_map(|e| {
                if let ClassElement::Property(p) = e {
                    if !p.is_static {
                        return Some(p.clone());
                    }
                }
                None
            })
            .collect();

        let field_init_stmts: Vec<ASTNode> = instance_fields
            .iter()
            .filter_map(|f| {
                let value = f
                    .value
                    .clone()
                    .unwrap_or(Expression::Literal(Literal::Undefined));
                let (property, computed) = match &f.name {
                    PropertyKey::Identifier(n) if f.is_private => {
                        (MemberProperty::PrivateIdentifier(n.clone()), false)
                    }
                    PropertyKey::Identifier(n) => (MemberProperty::Identifier(n.clone()), false),
                    PropertyKey::Literal(Literal::String(s, _)) => {
                        (MemberProperty::Identifier(s.clone()), false)
                    }
                    PropertyKey::PrivateIdentifier(n) => {
                        (MemberProperty::PrivateIdentifier(n.clone()), false)
                    }
                    PropertyKey::Computed(expr) => (MemberProperty::Computed(expr.clone()), true),
                    _ => return None,
                };
                Some(ASTNode::ExpressionStatement(ExpressionStatement {
                    expression: Expression::AssignmentExpression(AssignmentExpression {
                        op: AssignOp::Assign,
                        left: AssignmentTarget::MemberExpression(MemberExpression {
                            object: Box::new(Expression::This),
                            property,
                            computed,
                        }),
                        right: Box::new(value),
                    }),
                }))
            })
            .collect();

        let mut class_parent_vars = FxHashMap::default();

        for (scope_idx, scope) in self.scope_stack.iter().enumerate() {
            if self
                .scope_is_global
                .get(scope_idx)
                .copied()
                .unwrap_or(false)
            {
                continue;
            }
            for (var_name, entry) in scope {
                class_parent_vars.insert(
                    var_name.clone(),
                    ParentVar {
                        local_idx: entry.slot,
                        is_inherited: false,
                    },
                );
            }
        }

        for (var_name, &slot) in &self.upvalue_slots {
            if !class_parent_vars.contains_key(var_name) {
                class_parent_vars.insert(
                    var_name.clone(),
                    ParentVar {
                        local_idx: slot,
                        is_inherited: true,
                    },
                );
            }
        }

        if !class_name.is_empty() {
            class_parent_vars.insert(
                class_name.to_string(),
                ParentVar {
                    local_idx: class_slot,
                    is_inherited: false,
                },
            );
        }

        let (ctor_bytecode, ctor_upvalues) = if let Some(ClassElement::Method(m)) = ctor_method {
            let mut body = m.body.clone();
            let insert_pos = 0;
            for (i, stmt) in field_init_stmts.into_iter().enumerate() {
                body.body.insert(insert_pos + i, stmt);
                body.lines.insert(insert_pos + i, 0);
            }
            let mut nested = self.nested_codegen();
            nested.compile_function_with_parent(
                &m.params,
                &body,
                ctx,
                class_parent_vars.clone(),
                None,
                false,
            )?
        } else {
            let mut default_body = Vec::new();
            let mut default_lines = Vec::new();
            if has_super {
                let super_call = ASTNode::ExpressionStatement(ExpressionStatement {
                    expression: Expression::CallExpression(CallExpression {
                        callee: Box::new(Expression::Super),
                        arguments: vec![Argument::Spread(Expression::Identifier(Identifier {
                            name: "arguments".to_string(),
                        }))],
                    }),
                });
                default_body.push(super_call);
                default_lines.push(0);
            }
            default_body.extend(field_init_stmts);
            default_lines.extend(vec![0; instance_fields.len()]);
            let mut nested = self.nested_codegen();
            nested.compile_function_with_parent(
                &[],
                &BlockStatement {
                    body: default_body,
                    lines: default_lines,
                },
                ctx,
                class_parent_vars.clone(),
                None,
                false,
            )?
        };

        let mut instance_methods: Vec<(
            PropertyKey,
            Bytecode,
            Vec<(u32, u16)>,
            bool,
            bool,
            MethodKind,
            bool,
        )> = Vec::new();
        for element in &body.elements {
            if let ClassElement::Method(m) = element {
                if m.is_static || m.kind == MethodKind::Constructor {
                    continue;
                }
                let can_handle = match &m.name {
                    PropertyKey::Identifier(_)
                    | PropertyKey::Literal(Literal::String(_, _))
                    | PropertyKey::Literal(Literal::Number(_))
                    | PropertyKey::PrivateIdentifier(_)
                    | PropertyKey::Computed(_) => true,
                    _ => false,
                };
                if !can_handle {
                    continue;
                }
                let mut nested = self.nested_codegen();
                let (method_bc, method_upvalues) = nested.compile_function_with_parent(
                    &m.params,
                    &m.body,
                    ctx,
                    class_parent_vars.clone(),
                    None,
                    false,
                )?;
                instance_methods.push((
                    m.name.clone(),
                    method_bc,
                    method_upvalues,
                    m.generator,
                    m.is_async,
                    m.kind.clone(),
                    m.is_private,
                ));
            }
        }

        let mut static_methods: Vec<(
            PropertyKey,
            Bytecode,
            Vec<(u32, u16)>,
            bool,
            bool,
            MethodKind,
            bool,
        )> = Vec::new();
        for element in &body.elements {
            if let ClassElement::Method(m) = element {
                if !m.is_static {
                    continue;
                }
                let can_handle = match &m.name {
                    PropertyKey::Identifier(_)
                    | PropertyKey::Literal(Literal::String(_, _))
                    | PropertyKey::Literal(Literal::Number(_))
                    | PropertyKey::PrivateIdentifier(_)
                    | PropertyKey::Computed(_) => true,
                    _ => false,
                };
                if !can_handle {
                    continue;
                }
                let mut nested = self.nested_codegen();
                nested.in_static_method = true;
                let (method_bc, method_upvalues) = nested.compile_function_with_parent(
                    &m.params,
                    &m.body,
                    ctx,
                    class_parent_vars.clone(),
                    None,
                    false,
                )?;
                static_methods.push((
                    m.name.clone(),
                    method_bc,
                    method_upvalues,
                    m.generator,
                    m.is_async,
                    m.kind.clone(),
                    m.is_private,
                ));
            }
        }

        let mut static_blocks: Vec<(Bytecode, Vec<(u32, u16)>)> = Vec::new();
        for element in &body.elements {
            if let ClassElement::StaticBlock(block) = element {
                let mut nested = self.nested_codegen();
                let (block_bc, block_upvalues) = nested.compile_function_with_parent(
                    &[],
                    &BlockStatement {
                        body: block.body.clone(),
                        lines: block.lines.clone(),
                    },
                    ctx,
                    class_parent_vars.clone(),
                    None,
                    false,
                )?;
                static_blocks.push((block_bc, block_upvalues));
            }
        }

        let ctor_reg = self.alloc_register();
        let class_name_atom = ctx.atom_table_mut().intern(class_name);
        let ctor_name_atom = class_name_atom.0;
        self.emit_embedded_function(
            &ctor_bytecode,
            &ctor_upvalues,
            ctor_name_atom,
            ctor_reg,
            false,
            false,
        );

        if let Some(super_expr) = super_class {
            let super_key_idx = self.add_constant(JSValue::new_string(ctx.common_atoms.__super__));
            let super_key = self.alloc_register();
            self.emit(Opcode::LoadConst);
            self.emit_u16(super_key);
            self.emit_u32(super_key_idx);
            let super_val = self.gen_expression(super_expr, ctx)?;
            self.emit(Opcode::SetProp);
            self.emit_u16(ctor_reg);
            self.emit_u16(super_key);
            self.emit_u16(super_val);
            self.free_register(super_key);
            self.free_register(super_val);
        }

        if class_slot != ctor_reg {
            self.emit(Opcode::Move);
            self.emit_u16(class_slot);
            self.emit_u16(ctor_reg);
        }

        let proto_reg = self.alloc_register();
        self.emit(Opcode::NewObject);
        self.emit_u16(proto_reg);

        if let Some(super_expr) = super_class {
            let proto_key_idx = self.add_constant(JSValue::new_string(ctx.common_atoms.prototype));
            let proto_key = self.alloc_register();
            self.emit(Opcode::LoadConst);
            self.emit_u16(proto_key);
            self.emit_u32(proto_key_idx);
            let super_val = self.gen_expression(super_expr, ctx)?;
            let super_proto = self.alloc_register();
            self.emit(Opcode::GetProp);
            self.emit_u16(super_proto);
            self.emit_u16(super_val);
            self.emit_u16(proto_key);
            self.free_register(super_val);
            self.emit(Opcode::SetProto);
            self.emit_u16(proto_reg);
            self.emit_u16(super_proto);
            self.free_register(super_proto);
            self.free_register(proto_key);
        }

        for (method_key, method_bc, method_upvalues, is_gen, is_async, kind, is_private) in
            instance_methods
        {
            let key = self.alloc_register();
            let method_name_atom = match &method_key {
                PropertyKey::Identifier(n) | PropertyKey::Literal(Literal::String(n, _)) => {
                    let method_atom = ctx.atom_table_mut().intern(n.as_str());
                    let method_idx = self.add_constant(JSValue::new_string(method_atom));
                    self.emit(Opcode::LoadConst);
                    self.emit_u16(key);
                    self.emit_u32(method_idx);
                    method_atom.0
                }
                PropertyKey::Literal(Literal::Number(n)) => {
                    let s = if n.fract() == 0.0 && *n >= i32::MIN as f64 && *n <= i32::MAX as f64 {
                        (*n as i32).to_string()
                    } else {
                        f64_to_js_string(*n)
                    };
                    let method_atom = ctx.atom_table_mut().intern(&s);
                    let method_idx = self.add_constant(JSValue::new_string(method_atom));
                    self.emit(Opcode::LoadConst);
                    self.emit_u16(key);
                    self.emit_u32(method_idx);
                    method_atom.0
                }
                PropertyKey::PrivateIdentifier(n) => {
                    let method_atom = ctx.atom_table_mut().intern(n.as_str());
                    let method_idx = self.add_constant(JSValue::new_string(method_atom));
                    self.emit(Opcode::LoadConst);
                    self.emit_u16(key);
                    self.emit_u32(method_idx);
                    method_atom.0
                }
                PropertyKey::Computed(expr) => {
                    let k = self.gen_expression(expr, ctx)?;
                    self.emit(Opcode::Move);
                    self.emit_u16(key);
                    self.emit_u16(k);
                    self.free_register(k);
                    0
                }
                _ => {
                    self.free_register(key);
                    continue;
                }
            };
            let method_reg = self.alloc_register();
            self.emit_embedded_function(
                &method_bc,
                &method_upvalues,
                method_name_atom,
                method_reg,
                is_gen,
                is_async,
            );
            match kind {
                MethodKind::Get if is_private => {
                    self.emit(Opcode::DefinePrivateAccessor);
                    self.emit_u16(proto_reg);
                    self.emit_u16(key);
                    self.emit_u16(method_reg);
                    self.emit_u16(u16::MAX);
                }
                MethodKind::Set if is_private => {
                    self.emit(Opcode::DefinePrivateAccessor);
                    self.emit_u16(proto_reg);
                    self.emit_u16(key);
                    self.emit_u16(u16::MAX);
                    self.emit_u16(method_reg);
                }
                MethodKind::Get => {
                    self.emit(Opcode::DefineAccessor);
                    self.emit_u16(proto_reg);
                    self.emit_u16(key);
                    self.emit_u16(method_reg);
                    self.emit_u16(u16::MAX);
                }
                MethodKind::Set => {
                    self.emit(Opcode::DefineAccessor);
                    self.emit_u16(proto_reg);
                    self.emit_u16(key);
                    self.emit_u16(u16::MAX);
                    self.emit_u16(method_reg);
                }
                _ => {
                    if is_private {
                        self.emit(Opcode::SetPrivate);
                        self.emit_u16(proto_reg);
                        self.emit_u16(key);
                        self.emit_u16(method_reg);
                    } else {
                        self.emit(Opcode::SetMethodProp);
                        self.emit_u16(proto_reg);
                        self.emit_u16(key);
                        self.emit_u16(method_reg);
                    }
                }
            }
            self.free_register(key);
            self.free_register(method_reg);
        }

        for (method_key, method_bc, method_upvalues, is_gen, is_async, kind, is_private) in
            static_methods
        {
            let key = self.alloc_register();
            let method_name_atom = match &method_key {
                PropertyKey::Identifier(n) | PropertyKey::Literal(Literal::String(n, _)) => {
                    let method_atom = ctx.atom_table_mut().intern(n.as_str());
                    let method_idx = self.add_constant(JSValue::new_string(method_atom));
                    self.emit(Opcode::LoadConst);
                    self.emit_u16(key);
                    self.emit_u32(method_idx);
                    method_atom.0
                }
                PropertyKey::Literal(Literal::Number(n)) => {
                    let s = if n.fract() == 0.0 && *n >= i32::MIN as f64 && *n <= i32::MAX as f64 {
                        (*n as i32).to_string()
                    } else {
                        f64_to_js_string(*n)
                    };
                    let method_atom = ctx.atom_table_mut().intern(&s);
                    let method_idx = self.add_constant(JSValue::new_string(method_atom));
                    self.emit(Opcode::LoadConst);
                    self.emit_u16(key);
                    self.emit_u32(method_idx);
                    method_atom.0
                }
                PropertyKey::PrivateIdentifier(n) => {
                    let method_atom = ctx.atom_table_mut().intern(n.as_str());
                    let method_idx = self.add_constant(JSValue::new_string(method_atom));
                    self.emit(Opcode::LoadConst);
                    self.emit_u16(key);
                    self.emit_u32(method_idx);
                    method_atom.0
                }
                PropertyKey::Computed(expr) => {
                    let k = self.gen_expression(expr, ctx)?;
                    self.emit(Opcode::Move);
                    self.emit_u16(key);
                    self.emit_u16(k);
                    self.free_register(k);
                    0
                }
                _ => {
                    self.free_register(key);
                    continue;
                }
            };
            let method_reg = self.alloc_register();
            self.emit_embedded_function(
                &method_bc,
                &method_upvalues,
                method_name_atom,
                method_reg,
                is_gen,
                is_async,
            );
            match kind {
                MethodKind::Get if is_private => {
                    self.emit(Opcode::DefinePrivateAccessor);
                    self.emit_u16(class_slot);
                    self.emit_u16(key);
                    self.emit_u16(method_reg);
                    self.emit_u16(u16::MAX);
                }
                MethodKind::Set if is_private => {
                    self.emit(Opcode::DefinePrivateAccessor);
                    self.emit_u16(class_slot);
                    self.emit_u16(key);
                    self.emit_u16(u16::MAX);
                    self.emit_u16(method_reg);
                }
                MethodKind::Get => {
                    self.emit(Opcode::DefineAccessor);
                    self.emit_u16(class_slot);
                    self.emit_u16(key);
                    self.emit_u16(method_reg);
                    self.emit_u16(u16::MAX);
                }
                MethodKind::Set => {
                    self.emit(Opcode::DefineAccessor);
                    self.emit_u16(class_slot);
                    self.emit_u16(key);
                    self.emit_u16(u16::MAX);
                    self.emit_u16(method_reg);
                }
                _ => {
                    if is_private {
                        self.emit(Opcode::SetPrivate);
                        self.emit_u16(class_slot);
                        self.emit_u16(key);
                        self.emit_u16(method_reg);
                    } else {
                        self.emit(Opcode::SetMethodProp);
                        self.emit_u16(class_slot);
                        self.emit_u16(key);
                        self.emit_u16(method_reg);
                    }
                }
            }
            self.free_register(key);
            self.free_register(method_reg);
        }

        let proto_key_idx = self.add_constant(JSValue::new_string(ctx.common_atoms.prototype));
        let proto_key = self.alloc_register();
        self.emit(Opcode::LoadConst);
        self.emit_u16(proto_key);
        self.emit_u32(proto_key_idx);
        self.emit(Opcode::SetProp);
        self.emit_u16(class_slot);
        self.emit_u16(proto_key);
        self.emit_u16(proto_reg);
        self.free_register(proto_key);
        self.free_register(proto_reg);

        for element in &body.elements {
            if let ClassElement::Property(p) = element {
                if !p.is_static {
                    continue;
                }
                let val_reg = if let Some(value) = &p.value {
                    self.gen_expression(value, ctx)?
                } else {
                    let r = self.alloc_register();
                    self.emit(Opcode::LoadUndefined);
                    self.emit_u16(r);
                    r
                };
                if let PropertyKey::Computed(key_expr) = &p.name {
                    let key = self.gen_expression(key_expr, ctx)?;
                    self.emit(Opcode::SetField);
                    self.emit_u16(class_slot);
                    self.emit_u16(key);
                    self.emit_u16(val_reg);
                    self.free_register(key);
                    self.free_register(val_reg);
                } else {
                    let field_name = match &p.name {
                        PropertyKey::Identifier(n) => n.clone(),
                        PropertyKey::Literal(Literal::String(s, _)) => s.clone(),
                        PropertyKey::PrivateIdentifier(n) => n.clone(),
                        _ => {
                            self.free_register(val_reg);
                            continue;
                        }
                    };
                    let field_idx = self.intern_and_add_const(ctx, &field_name);
                    let key = self.alloc_register();
                    self.emit(Opcode::LoadConst);
                    self.emit_u16(key);
                    self.emit_u32(field_idx);
                    if p.is_private {
                        self.emit(Opcode::SetPrivate);
                    } else {
                        self.emit(Opcode::SetProp);
                    }
                    self.emit_u16(class_slot);
                    self.emit_u16(key);
                    self.emit_u16(val_reg);
                    self.free_register(key);
                    self.free_register(val_reg);
                }
            }
        }

        for (block_bc, block_upvalues) in static_blocks {
            let block_reg = self.alloc_register();
            self.emit_embedded_function(&block_bc, &block_upvalues, 0, block_reg, false, false);
            self.emit(Opcode::Call);
            self.emit_u16(block_reg);
            self.emit_u16(block_reg);
            self.emit_u16(0);
            self.free_register(block_reg);
        }

        Ok(class_name_atom)
    }
}

pub(crate) fn detect_simple_constructor_props(
    bytecode: &crate::compiler::opcode::Bytecode,
) -> bool {
    let code = &bytecode.code;
    if code.is_empty() {
        return false;
    }
    let mut pc = 0usize;
    let mut found_return = false;
    let mut has_simple_prop = false;
    let pc_limit = bytecode.param_count;
    while pc < code.len() {
        let op = crate::compiler::opcode::Opcode::from_u8_unchecked(code[pc]);
        let size = crate::compiler::opcode::Opcode::instruction_size(op);
        if size == 0 || pc + size > code.len() {
            break;
        }
        match op {
            crate::compiler::opcode::Opcode::SetNamedProp => {
                let obj = u16::from_le_bytes([code[pc + 1], code[pc + 2]]);
                let val = u16::from_le_bytes([code[pc + 3], code[pc + 4]]);
                if val > pc_limit {
                    return false;
                }
                if obj == 0 && val >= 1 {
                    has_simple_prop = true;
                } else {
                    return false;
                }
            }
            crate::compiler::opcode::Opcode::LoadUndefined => {
                let dst = u16::from_le_bytes([code[pc + 1], code[pc + 2]]);
                if dst != 0 {
                    return false;
                }
            }
            crate::compiler::opcode::Opcode::Return => {
                let src = u16::from_le_bytes([code[pc + 1], code[pc + 2]]);
                if src == 0 {
                    found_return = true;
                }
                break;
            }
            _ => {
                if op != crate::compiler::opcode::Opcode::Nop {
                    return false;
                }
            }
        }
        pc += size;
    }
    found_return && has_simple_prop
}

fn f64_to_js_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        if n.is_sign_negative() {
            return "-Infinity".to_string();
        }
        return "Infinity".to_string();
    }
    let abs = n.abs();
    if abs == 0.0 {
        if n.is_sign_negative() {
            return "0".to_string();
        }
        return "0".to_string();
    }
    if abs < 1e-6 || abs >= 1e21 {
        let s = format!("{:e}", n);
        s
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::vm::VM;
    use crate::runtime::{JSContext, JSRuntime};

    fn run_expr(code: &str) -> i64 {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);

        let ast = crate::compiler::parser::Parser::new(code).parse().unwrap();
        let expr = match ast.body.first() {
            Some(ASTNode::ExpressionStatement(es)) => es.expression.clone(),
            _ => panic!("expected expression"),
        };

        let mut codegen = CodeGenerator::with_opt_level(OptLevel::default());
        let reg = codegen.compile_expression(&expr, &mut ctx).unwrap();

        let mut bytecode = Bytecode::new();
        let (code, constants, locals_count, _) = codegen.take_bytecode();
        bytecode.code = code;
        bytecode.constants = constants;
        bytecode.locals_count = locals_count.max((reg + 1) as u32);

        if reg != 0 {
            bytecode.code.push(Opcode::Move as u8);
            bytecode.code.extend_from_slice(&0u16.to_le_bytes());
            bytecode.code.extend_from_slice(&reg.to_le_bytes());
        }
        bytecode.code.push(Opcode::End as u8);

        let mut vm = VM::new();
        let result = match vm.execute(&mut ctx, &bytecode).unwrap() {
            crate::runtime::vm::ExecutionOutcome::Complete(v) => v,
            crate::runtime::vm::ExecutionOutcome::Yield(v) => v,
        };
        result.get_int()
    }

    fn run_func(code: &str) -> i64 {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);

        let ast = crate::compiler::parser::Parser::new(code).parse().unwrap();
        let func = match ast.body.first() {
            Some(ASTNode::FunctionDeclaration(fd)) => fd.clone(),
            _ => panic!("expected function declaration"),
        };

        let mut codegen = CodeGenerator::with_opt_level(OptLevel::default());
        let (bytecode, _upvalues) = codegen
            .compile_function(&func.params, &func.body, &mut ctx)
            .unwrap();

        let mut vm = VM::new();
        let result = match vm.execute(&mut ctx, &bytecode).unwrap() {
            crate::runtime::vm::ExecutionOutcome::Complete(v) => v,
            crate::runtime::vm::ExecutionOutcome::Yield(v) => v,
        };
        result.get_int()
    }

    fn compile_func_bytecode(code: &str, opt_level: OptLevel) -> Bytecode {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);
        let ast = crate::compiler::parser::Parser::new(code).parse().unwrap();
        let func = match ast.body.first() {
            Some(ASTNode::FunctionDeclaration(fd)) => fd.clone(),
            _ => panic!("expected function declaration"),
        };
        let mut codegen = CodeGenerator::with_opt_level(opt_level);
        let (bc, _) = codegen
            .compile_function(&func.params, &func.body, &mut ctx)
            .unwrap();
        bc
    }

    fn compile_script_bytecode_len(code: &str, opt_level: OptLevel) -> usize {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);
        let ast = crate::compiler::parser::Parser::new(code).parse().unwrap();
        let block = crate::compiler::ast::BlockStatement {
            body: ast.body,
            lines: ast.lines,
        };
        let mut codegen = CodeGenerator::with_opt_level(opt_level);
        let (bc, _) = codegen.compile_script(&block, &mut ctx).unwrap();
        bc.code.len()
    }

    #[test]
    fn test_literal_int() {
        assert_eq!(run_expr("42"), 42);
    }

    #[test]
    fn test_add_sub() {
        assert_eq!(run_expr("1 + 2"), 3);
        assert_eq!(run_expr("10 - 3"), 7);
    }

    #[test]
    fn test_mul_div() {
        assert_eq!(run_expr("3 * 4"), 12);
        assert_eq!(run_expr("20 / 4"), 5);
    }

    #[test]
    fn test_complex_arithmetic() {
        assert_eq!(run_expr("1 + 2 * 3"), 7);
        assert_eq!(run_expr("(1 + 2) * 3"), 9);
    }

    #[test]
    fn test_variable_and_assignment() {
        assert_eq!(
            run_func("function f() { var x = 10; x = x + 5; return x; }"),
            15
        );
    }

    #[test]
    fn test_if_statement() {
        assert_eq!(
            run_func("function f() { var x = 3; if (x < 5) { return 1; } return 0; }"),
            1
        );
        assert_eq!(
            run_func("function f() { var x = 10; if (x < 5) { return 1; } return 0; }"),
            0
        );
    }

    #[test]
    fn test_if_else() {
        assert_eq!(
            run_func("function f() { var x = 2; if (x > 5) { return 10; } else { return 20; } }"),
            20
        );
    }

    #[test]
    fn test_while_loop() {
        assert_eq!(
            run_func(
                "function f() { var sum = 0; var i = 0; while (i < 10) { sum = sum + i; i = i + 1; } return sum; }"
            ),
            45
        );
    }

    #[test]
    fn test_for_loop() {
        assert_eq!(
            run_func(
                "function f() { var sum = 0; for (var i = 0; i <= 10; i = i + 1) { sum = sum + i; } return sum; }"
            ),
            55
        );
    }

    #[test]
    fn test_ternary() {
        assert_eq!(run_expr("1 < 2 ? 10 : 20"), 10);
        assert_eq!(run_expr("1 > 2 ? 10 : 20"), 20);
    }

    #[test]
    fn test_unary_neg() {
        assert_eq!(run_expr("-5"), -5);
        assert_eq!(run_expr("-(-3)"), 3);
    }

    #[test]
    fn test_logical_and() {
        assert_eq!(run_expr("1 && 1 ? 1 : 0"), 1);
    }

    #[test]
    fn test_p2_profile_raw_disables_all_literal_folds() {
        let code = "function f() { if (true) { return 1; } return 2; }";
        let bc = compile_func_bytecode(code, OptLevel::O0);
        assert!(bc.code.contains(&(Opcode::JumpIfNot as u8)));
    }

    #[test]
    fn test_p2_profile_ternary_keeps_if_unfolded() {
        let code = "function f() { if (true) { return 1; } return 2; }";
        let bc = compile_func_bytecode(code, OptLevel::O1);
        assert!(bc.code.contains(&(Opcode::JumpIfNot as u8)));
    }

    #[test]
    fn test_p2_profile_stable_folds_if_true() {
        let code = "function f() { if (true) { return 1; } return 2; }";
        let bc = compile_func_bytecode(code, OptLevel::O2);
        assert!(!bc.code.contains(&(Opcode::JumpIfNot as u8)));
    }

    #[test]
    fn test_p2_profile_aggressive_folds_string_truthy_conditional() {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);
        let ast = crate::compiler::parser::Parser::new("'x' ? 11 : 22")
            .parse()
            .unwrap();
        let expr = match ast.body.first() {
            Some(ASTNode::ExpressionStatement(es)) => es.expression.clone(),
            _ => panic!("expected expression"),
        };
        let mut codegen = CodeGenerator::with_opt_level(OptLevel::O3);
        let reg = codegen.compile_expression(&expr, &mut ctx).unwrap();

        let mut bytecode = Bytecode::new();
        let (code, constants, locals_count, _) = codegen.take_bytecode();
        bytecode.code = code;
        bytecode.constants = constants;
        bytecode.locals_count = locals_count.max((reg + 1) as u32);
        if reg != 0 {
            bytecode.code.push(Opcode::Move as u8);
            bytecode.code.extend_from_slice(&0u16.to_le_bytes());
            bytecode.code.extend_from_slice(&reg.to_le_bytes());
        }
        bytecode.code.push(Opcode::End as u8);

        let mut vm = VM::new();
        let result = match vm.execute(&mut ctx, &bytecode).unwrap() {
            crate::runtime::vm::ExecutionOutcome::Complete(v) => v,
            crate::runtime::vm::ExecutionOutcome::Yield(v) => v,
        };
        assert_eq!(result.get_int(), 11);
    }

    #[test]
    fn test_prefix_increment() {
        assert_eq!(run_func("function f() { var x = 5; return ++x; }"), 6);
        assert_eq!(
            run_func("function f() { var x = 5; var y = ++x; return x + y; }"),
            12
        );
    }

    #[test]
    fn test_postfix_increment() {
        assert_eq!(run_func("function f() { var x = 5; return x++; }"), 5);
        assert_eq!(
            run_func("function f() { var x = 5; var y = x++; return x + y; }"),
            11
        );
    }

    #[test]
    fn test_prefix_decrement() {
        assert_eq!(run_func("function f() { var x = 5; return --x; }"), 4);
        assert_eq!(
            run_func("function f() { var x = 5; var y = --x; return x + y; }"),
            8
        );
    }

    #[test]
    fn test_postfix_decrement() {
        assert_eq!(run_func("function f() { var x = 5; return x--; }"), 5);
        assert_eq!(
            run_func("function f() { var x = 5; var y = x--; return x + y; }"),
            9
        );
    }

    #[test]
    fn test_do_while_loop() {
        assert_eq!(
            run_func(
                "function f() { var sum = 0; var i = 0; do { sum = sum + i; i = i + 1; } while (i < 10); return sum; }"
            ),
            45
        );
    }

    #[test]
    fn test_break_in_while() {
        assert_eq!(
            run_func(
                "function f() { var sum = 0; var i = 0; while (i < 100) { sum = sum + i; i = i + 1; if (i > 5) break; } return sum; }"
            ),
            15
        );
    }

    #[test]
    fn test_break_in_for() {
        assert_eq!(
            run_func(
                "function f() { var sum = 0; for (var i = 0; i < 100; i = i + 1) { sum = sum + i; if (i == 5) break; } return sum; }"
            ),
            15
        );
    }

    #[test]
    fn test_break_in_do_while() {
        assert_eq!(
            run_func(
                "function f() { var sum = 0; var i = 0; do { sum = sum + i; i = i + 1; if (i > 5) break; } while (i < 100); return sum; }"
            ),
            15
        );
    }

    #[test]
    fn test_continue_in_while() {
        assert_eq!(
            run_func(
                "function f() { var sum = 0; var i = 0; while (i < 5) { i = i + 1; if (i == 3) continue; sum = sum + i; } return sum; }"
            ),
            12
        );
    }

    #[test]
    fn test_continue_in_for() {
        assert_eq!(
            run_func(
                "function f() { var sum = 0; for (var i = 0; i < 5; i = i + 1) { if (i == 3) continue; sum = sum + i; } return sum; }"
            ),
            7
        );
    }

    #[test]
    fn test_tiny_inline_getter_reduces_bytecode() {
        let code = r#"
            function car(p) { return p.car; }
            function main(x) { return car(x); }
        "#;

        let inline_len = compile_script_bytecode_len(code, OptLevel::O2);
        let noninline_len = compile_script_bytecode_len(code, OptLevel::O0);

        assert!(
            inline_len < noninline_len,
            "tiny getter inline should reduce bytecode: inline={}, noninline={}",
            inline_len,
            noninline_len
        );
    }

    #[test]
    fn test_tiny_inline_nested_getter_reduces_bytecode() {
        let code = r#"
            function cadr(p) { return p.cdr.car; }
            function main(x) { return cadr(x); }
        "#;

        let inline_len = compile_script_bytecode_len(code, OptLevel::O2);
        let noninline_len = compile_script_bytecode_len(code, OptLevel::O0);

        assert!(
            inline_len < noninline_len,
            "tiny nested getter inline should reduce bytecode: inline={}, noninline={}",
            inline_len,
            noninline_len
        );
    }

    #[test]
    fn test_continue_in_do_while() {
        assert_eq!(
            run_func(
                "function f() { var sum = 0; var i = 0; do { i = i + 1; if (i == 3) continue; sum = sum + i; } while (i < 5); return sum; }"
            ),
            12
        );
    }

    #[test]
    fn test_bitwise_codegen() {
        assert_eq!(run_expr("12 & 10"), 8);
        assert_eq!(run_expr("12 | 10"), 14);
        assert_eq!(run_expr("12 ^ 10"), 6);
        assert_eq!(run_expr("~8"), -9);
        assert_eq!(run_expr("1 << 4"), 16);
        assert_eq!(run_expr("16 >> 2"), 4);
        assert_eq!(run_expr("-1 >>> 1"), 2147483647);
    }

    #[test]
    fn test_mod_pow_codegen() {
        assert_eq!(run_expr("17 % 5"), 2);
        assert_eq!(run_expr("2 ** 10"), 1024);
    }

    #[test]
    fn test_switch_basic() {
        assert_eq!(
            run_func(
                "function f() { var x = 2; switch (x) { case 1: return 10; case 2: return 20; case 3: return 30; default: return 99; } }"
            ),
            20
        );
    }

    #[test]
    fn test_switch_default() {
        assert_eq!(
            run_func(
                "function f() { var x = 5; switch (x) { case 1: return 10; case 2: return 20; default: return 99; } }"
            ),
            99
        );
    }

    #[test]
    fn test_switch_fallthrough() {
        assert_eq!(
            run_func(
                "function f() { var x = 1; var r = 0; switch (x) { case 1: r = r + 1; case 2: r = r + 10; default: r = r + 100; } return r; }"
            ),
            111
        );
    }

    #[test]
    fn test_switch_break() {
        assert_eq!(
            run_func(
                "function f() { var x = 1; var r = 0; switch (x) { case 1: r = r + 1; break; case 2: r = r + 10; break; default: r = r + 100; } return r; }"
            ),
            1
        );
    }

    #[test]
    fn test_switch_break_in_nested_loop() {
        assert_eq!(
            run_func(
                "function f() { var r = 0; for (var i = 0; i < 3; i = i + 1) { switch (i) { case 0: r = r + 1; break; case 1: r = r + 10; break; default: r = r + 100; } } return r; }"
            ),
            111
        );
    }

    #[test]
    fn test_closure_basic() {
        assert_eq!(
            run_func(
                "function f() { var count = 0; function inc() { count = count + 1; return count; } return inc() + inc(); }"
            ),
            3
        );
    }

    #[test]
    fn test_closure_counter() {
        assert_eq!(
            run_func(
                "function f() { function makeCounter() { var count = 0; return function() { count = count + 1; return count; }; } var c = makeCounter(); return c() + c(); }"
            ),
            3
        );
    }

    #[test]
    fn test_closure_with_param() {
        assert_eq!(
            run_func(
                "function f() { function makeAdder(x) { return function(y) { return x + y; }; } var add5 = makeAdder(5); return add5(3); }"
            ),
            8
        );
    }

    #[test]
    fn test_nested_closure() {
        assert_eq!(
            run_func(
                "function f() { var a = 1; function g() { var b = 2; return function() { return a + b; }; } return g()(); }"
            ),
            3
        );
    }

    #[test]
    fn test_arrow_function() {
        assert_eq!(
            run_func(
                "function f() { var double = function(x) { return x * 2; }; return double(5); }"
            ),
            10
        );
    }

    #[test]
    fn test_delete_prop() {
        assert_eq!(
            run_func("function f() { var o = {x: 1, y: 2}; delete o.x; return o.y; }"),
            2
        );
    }

    #[test]
    fn test_delete_returns_true() {
        assert_eq!(
            run_func("function f() { var o = {x: 1}; return delete o.x; }"),
            1
        );
    }

    #[test]
    fn test_has_property_in() {
        assert_eq!(
            run_func("function f() { var o = {x: 1}; return 'x' in o; }"),
            1
        );
    }

    #[test]
    fn test_has_property_not_in() {
        assert_eq!(
            run_func("function f() { var o = {x: 1}; return 'y' in o; }"),
            0
        );
    }

    #[test]
    fn test_rest_params() {
        assert_eq!(
            run_func(
                "function outer() { function f(a, b, ...rest) { return rest.length; } return f(1, 2, 3, 4); }"
            ),
            2
        );
    }

    #[test]
    fn test_rest_params_values() {
        assert_eq!(
            run_func(
                "function outer() { function f(a, ...rest) { return rest[0] + rest[1]; } return f(1, 10, 20); }"
            ),
            30
        );
    }

    #[test]
    fn test_rest_params_empty() {
        assert_eq!(
            run_func(
                "function outer() { function f(a, b, ...rest) { return rest.length; } return f(1, 2); }"
            ),
            0
        );
    }

    #[test]
    fn test_object_spread() {
        assert_eq!(
            run_func(
                "function f() { var o = {a: 1, b: 2}; var n = {...o, c: 3}; return n.a + n.b + n.c; }"
            ),
            6
        );
    }

    #[test]
    fn test_array_spread() {
        assert_eq!(
            run_func(
                "function f() { var a = [1, 2]; var b = [...a, 3, 4]; return b[0] + b[1] + b[2] + b[3]; }"
            ),
            10
        );
    }

    #[test]
    fn test_array_spread_mixed() {
        assert_eq!(
            run_func("function f() { var a = [10, 20]; var b = [5, ...a]; return b.length; }"),
            3
        );
    }

    #[test]
    fn test_for_of_basic() {
        assert_eq!(
            run_func(
                "function f() { var s = 0; for (var x of [1, 2, 3]) { s = s + x; } return s; }"
            ),
            6
        );
    }

    #[test]
    fn test_for_of_destructuring_array() {
        assert_eq!(
            run_func(
                "function f() { var s = 0; for (var [a, b] of [[1,2],[3,4]]) { s = s + a + b; } return s; }"
            ),
            10
        );
    }

    #[test]
    fn test_for_of_destructuring_object() {
        assert_eq!(
            run_func(
                "function f() { var s = 0; for (var {x} of [{x:1},{x:2},{x:3}]) { s = s + x; } return s; }"
            ),
            6
        );
    }

    #[test]
    fn test_binding_destructuring_nested() {
        assert_eq!(
            run_func("function f() { var [[x], {y}] = [[1], {y: 2}]; return x + y; }"),
            3
        );
    }

    #[test]
    fn test_binding_destructuring_defaults() {
        assert_eq!(
            run_func("function f() { var [a = 10, b = 20] = [1]; return a + b; }"),
            21
        );
    }

    #[test]
    fn test_for_in_destructuring_array() {
        assert_eq!(
            run_func(
                "function f() { var o = {a:1,b:2}; var s = 0; for (var [k] in o) { s = s + 1; } return s; }"
            ),
            2
        );
    }

    #[test]
    fn test_class_basic() {
        assert_eq!(
            run_func(
                "function f() { class A { constructor() { this.x = 42; } getX() { return this.x; } } var a = new A(); return a.getX(); }"
            ),
            42
        );
    }

    #[test]
    fn test_class_with_static_method() {
        assert_eq!(
            run_func("function f() { class A { static foo() { return 10; } } return A.foo(); }"),
            10
        );
    }

    #[test]
    fn test_class_with_instance_field() {
        assert_eq!(
            run_func(
                "function f() { class A { x = 5; y = 7; sum() { return this.x + this.y; } } var a = new A(); return a.sum(); }"
            ),
            12
        );
    }

    #[test]
    fn test_class_inheritance() {
        assert_eq!(
            run_func(
                "function f() { class A { constructor() { this.x = 10; } foo() { return this.x; } } class B extends A { constructor() { super(); this.y = 20; } bar() { return this.y; } } var b = new B(); return b.foo() + b.bar(); }"
            ),
            30
        );
    }

    #[test]
    fn test_class_default_constructor_with_super() {
        assert_eq!(
            run_func(
                "function f() { class A { constructor() { this.x = 100; } } class B extends A {} var b = new B(); return b.x; }"
            ),
            100
        );
    }

    #[test]
    fn test_class_static_field() {
        assert_eq!(
            run_func("function f() { class A { static count = 3; } return A.count; }"),
            3
        );
    }

    #[test]
    fn test_for_in_basic() {
        assert_eq!(
            run_func(
                "function f() { var o = {a: 1, b: 2, c: 3}; var sum = 0; for (var k in o) { sum = sum + 1; } return sum; }"
            ),
            3
        );
    }

    #[test]
    fn test_for_in_sum_values() {
        assert_eq!(
            run_func(
                "function f() { var o = {a: 10, b: 20}; var sum = 0; for (var k in o) { sum = sum + o[k]; } return sum; }"
            ),
            30
        );
    }

    #[test]
    fn test_computed_prop_access() {
        assert_eq!(
            run_func("function f() { var o = {a: 10}; return o['a']; }"),
            10
        );
    }

    #[test]
    fn test_try_catch_throw() {
        assert_eq!(
            run_func("function f() { try { throw 42; return 0; } catch(e) { return e + 1; } }"),
            43
        );
    }

    #[test]
    fn test_try_no_throw() {
        assert_eq!(
            run_func("function f() { try { return 7; } catch(e) { return 99; } }"),
            7
        );
    }

    #[test]
    fn test_try_catch_finally() {
        assert_eq!(
            run_func(
                "function f() { var x = 0; try { throw 5; } catch(e) { x = e + 10; } finally { x = x + 1; } return x; }"
            ),
            16
        );
    }

    #[test]
    fn test_try_catch_nested() {
        assert_eq!(
            run_func(
                "function f() { try { try { throw 3; } catch(e) { throw e + 2; } } catch(e) { return e * 2; } }"
            ),
            10
        );
    }

    #[test]
    fn test_private_instance_field_get() {
        assert_eq!(
            run_func(
                "function f() { class A { #x = 42; getX() { return this.#x; } } return new A().getX(); }"
            ),
            42
        );
    }

    #[test]
    fn test_private_instance_field_set() {
        assert_eq!(
            run_func(
                "function f() { class A { #x = 10; setX(v) { this.#x = v; } getX() { return this.#x; } } var a = new A(); a.setX(99); return a.getX(); }"
            ),
            99
        );
    }

    #[test]
    fn test_private_instance_field_in() {
        assert_eq!(
            run_func(
                "function f() { class A { #x = 1; hasX() { return #x in this; } } return new A().hasX() ? 1 : 0; }"
            ),
            1
        );
    }

    #[test]
    fn test_private_static_field() {
        assert_eq!(
            run_func(
                "function f() { class A { static #x = 7; static getX() { return A.#x; } } return A.getX(); }"
            ),
            7
        );
    }

    #[test]
    fn test_generator_basic() {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);
        let code = "(function*() { yield 1; yield 2; })";
        let ast = crate::compiler::parser::Parser::new(code).parse().unwrap();
        let expr = match ast.body.first() {
            Some(ASTNode::ExpressionStatement(es)) => es.expression.clone(),
            _ => panic!("expected expression"),
        };
        let mut codegen = CodeGenerator::new();
        let func_reg = codegen.compile_expression(&expr, &mut ctx).unwrap();

        let call_dst = codegen.alloc_register();
        codegen.emit(Opcode::Call);
        codegen.emit_u16(call_dst);
        codegen.emit_u16(func_reg);
        codegen.emit_u16(0);

        let mut bytecode = Bytecode::new();
        let (code, constants, locals_count, _) = codegen.take_bytecode();
        bytecode.code = code;
        bytecode.constants = constants;
        bytecode.locals_count = locals_count.max((call_dst + 1) as u32);
        bytecode.code.push(Opcode::Move as u8);
        bytecode.code.extend_from_slice(&0u16.to_le_bytes());
        bytecode.code.extend_from_slice(&call_dst.to_le_bytes());
        bytecode.code.push(Opcode::End as u8);

        let mut vm = VM::new();
        let result = match vm.execute(&mut ctx, &bytecode).unwrap() {
            crate::runtime::vm::ExecutionOutcome::Complete(v) => v,
            crate::runtime::vm::ExecutionOutcome::Yield(v) => v,
        };
        assert!(result.is_object(), "expected generator object");
    }

    #[test]
    fn test_async_generator_basic() {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);
        let code = "(async function*() { yield 1; yield 2; })";
        let ast = crate::compiler::parser::Parser::new(code).parse().unwrap();
        let expr = match ast.body.first() {
            Some(ASTNode::ExpressionStatement(es)) => es.expression.clone(),
            _ => panic!("expected expression"),
        };
        let mut codegen = CodeGenerator::new();
        let func_reg = codegen.compile_expression(&expr, &mut ctx).unwrap();

        let call_dst = codegen.alloc_register();
        codegen.emit(Opcode::Call);
        codegen.emit_u16(call_dst);
        codegen.emit_u16(func_reg);
        codegen.emit_u16(0);

        let mut bytecode = Bytecode::new();
        let (code, constants, locals_count, _) = codegen.take_bytecode();
        bytecode.code = code;
        bytecode.constants = constants;
        bytecode.locals_count = locals_count.max((call_dst + 1) as u32);
        bytecode.code.push(Opcode::Move as u8);
        bytecode.code.extend_from_slice(&0u16.to_le_bytes());
        bytecode.code.extend_from_slice(&call_dst.to_le_bytes());
        bytecode.code.push(Opcode::End as u8);

        let mut vm = VM::new();
        let result = match vm.execute(&mut ctx, &bytecode).unwrap() {
            crate::runtime::vm::ExecutionOutcome::Complete(v) => v,
            crate::runtime::vm::ExecutionOutcome::Yield(v) => v,
        };
        assert!(result.is_object(), "expected async generator object");
        let obj = result.as_object();
        assert!(obj.is_generator(), "expected generator flag to be set");
    }

    #[test]
    fn test_promise_resolve_in_register_vm() {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);
        let code = "Promise.resolve(42)";
        let ast = crate::compiler::parser::Parser::new(code).parse().unwrap();
        let expr = match ast.body.first() {
            Some(ASTNode::ExpressionStatement(es)) => es.expression.clone(),
            _ => panic!("expected expression"),
        };
        let mut codegen = CodeGenerator::new();
        let dst = codegen.compile_expression(&expr, &mut ctx).unwrap();

        let mut bytecode = Bytecode::new();
        let (code, constants, locals_count, _) = codegen.take_bytecode();
        bytecode.code = code;
        bytecode.constants = constants;
        bytecode.locals_count = locals_count.max((dst + 1) as u32);
        bytecode.code.push(Opcode::Move as u8);
        bytecode.code.extend_from_slice(&0u16.to_le_bytes());
        bytecode.code.extend_from_slice(&dst.to_le_bytes());
        bytecode.code.push(Opcode::End as u8);

        let mut vm = VM::new();
        let result = match vm.execute(&mut ctx, &bytecode).unwrap() {
            crate::runtime::vm::ExecutionOutcome::Complete(v) => v,
            crate::runtime::vm::ExecutionOutcome::Yield(v) => v,
        };
        assert!(result.is_object(), "expected promise object");
        assert!(
            crate::builtins::promise::is_promise(&result),
            "expected a Promise"
        );
        let promise_obj = result.as_object();
        let state = promise_obj
            .get(ctx.common_atoms.__promise_state__)
            .unwrap_or(JSValue::new_int(0))
            .get_int();
        assert_eq!(state, 1, "expected resolved promise");
        let value = promise_obj
            .get(ctx.common_atoms.__promise_result__)
            .unwrap_or(JSValue::undefined());
        assert_eq!(value.get_int(), 42, "expected promise result to be 42");
    }

    #[test]
    fn test_async_function_await_resolved_promise() {
        let mut rt = JSRuntime::new();
        let mut ctx = JSContext::new(&mut rt);
        let code = "(async function() { return await Promise.resolve(42); })";
        let ast = crate::compiler::parser::Parser::new(code).parse().unwrap();
        let expr = match ast.body.first() {
            Some(ASTNode::ExpressionStatement(es)) => es.expression.clone(),
            _ => panic!("expected expression"),
        };
        let mut codegen = CodeGenerator::new();
        let func_reg = codegen.compile_expression(&expr, &mut ctx).unwrap();

        let call_dst = codegen.alloc_register();
        codegen.emit(Opcode::Call);
        codegen.emit_u16(call_dst);
        codegen.emit_u16(func_reg);
        codegen.emit_u16(0);

        let mut bytecode = Bytecode::new();
        let (code, constants, locals_count, _) = codegen.take_bytecode();
        bytecode.code = code;
        bytecode.constants = constants;
        bytecode.locals_count = locals_count.max((call_dst + 1) as u32);
        bytecode.code.push(Opcode::Move as u8);
        bytecode.code.extend_from_slice(&0u16.to_le_bytes());
        bytecode.code.extend_from_slice(&call_dst.to_le_bytes());
        bytecode.code.push(Opcode::End as u8);

        let mut vm = VM::new();
        let result = match vm.execute(&mut ctx, &bytecode).unwrap() {
            crate::runtime::vm::ExecutionOutcome::Complete(v) => v,
            crate::runtime::vm::ExecutionOutcome::Yield(v) => v,
        };
        assert!(
            result.is_object(),
            "expected promise object from async function call"
        );

        assert!(
            crate::builtins::promise::is_promise(&result),
            "expected a Promise"
        );
        let promise_obj = result.as_object();
        let state = promise_obj
            .get(ctx.common_atoms.__promise_state__)
            .unwrap_or(JSValue::new_int(0))
            .get_int();
        assert_eq!(state, 1, "expected resolved promise");
        let value = promise_obj
            .get(ctx.common_atoms.__promise_result__)
            .unwrap_or(JSValue::undefined());
        assert_eq!(value.get_int(), 42, "expected promise result to be 42");
    }

    #[test]
    fn test_deep_nesting_no_overflow() {
        let code = "1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1";
        assert_eq!(run_expr(code), 49);
    }

    #[test]
    fn test_very_deep_nesting_no_overflow() {
        let ones: Vec<&str> = std::iter::repeat("1").take(100).collect();
        let code = ones.join(" + ");
        assert_eq!(run_expr(&code), 100);
    }

    #[test]
    fn test_extreme_deep_nesting_no_overflow() {
        let ones: Vec<&str> = std::iter::repeat("1").take(300).collect();
        let code = ones.join(" + ");
        assert_eq!(run_expr(&code), 300);
    }
}
