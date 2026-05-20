#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    Nop = 0,
    End = 1,
    Return = 2,

    LoadConst = 10,
    LoadInt = 11,
    LoadInt8 = 12,
    LoadTrue = 13,
    LoadFalse = 14,
    LoadNull = 15,
    LoadUndefined = 16,
    LoadTdz = 17,
    CheckTdz = 18,
    CheckRef = 19,
    CheckObjectCoercible = 91,

    Add = 20,
    Sub = 21,
    SubImm8 = 34,
    AddNum = 35,
    SubNum = 36,
    MulNum = 37,
    DivNum = 38,
    AddImm8 = 39,
    Mul = 22,
    Div = 23,
    Mod = 24,
    Pow = 25,
    Neg = 26,
    BitAnd = 27,
    BitOr = 28,
    BitXor = 29,
    BitNot = 30,
    Shl = 31,
    Shr = 32,
    UShr = 33,

    Lt = 50,
    Lte = 51,
    LteImm8 = 60,
    Gt = 52,
    Gte = 53,
    Eq = 54,
    Neq = 55,
    StrictEq = 56,
    StrictNeq = 57,
    InstanceOf = 58,
    NewRegExp = 59,

    Not = 70,
    TypeOf = 71,

    Jump = 80,
    JumpIf = 81,
    JumpIfNot = 82,
    JumpIfNullish = 176,
    Throw = 83,
    Try = 84,
    Catch = 85,
    Finally = 86,
    Jump8 = 87,
    JumpIf8 = 88,
    JumpIfNot8 = 89,

    Move = 90,

    GetLocal = 100,
    SetLocal = 101,
    GetGlobal = 102,
    SetGlobal = 103,
    GetUpvalue = 104,
    SetUpvalue = 105,
    IncLocal = 106,
    DecLocal = 107,

    NewObject = 120,
    NewArray = 121,
    GetField = 122,
    SetField = 123,
    GetProp = 124,
    SetProp = 125,
    Call = 126,
    CallMethod = 127,
    NewFunction = 128,
    CallNew = 129,
    DeleteProp = 130,
    HasProperty = 131,
    GatherRest = 132,
    ObjectSpread = 133,
    GetPropertyNames = 134,
    ArrayExtend = 135,
    SetProto = 136,
    GetSuper = 137,
    ArrayPush = 138,
    CallSpread = 139,
    CallMethodSpread = 140,
    CallNewSpread = 141,
    GetPrivate = 142,
    SetPrivate = 143,
    HasPrivate = 144,
    Yield = 145,
    NewGeneratorFunction = 146,
    NewAsyncFunction = 147,
    Await = 148,
    NewAsyncGeneratorFunction = 149,
    GetIterator = 150,
    IteratorNext = 151,
    GetArguments = 152,
    GetCurrentFunction = 153,
    CallCurrent = 154,
    CallCurrent1 = 155,
    GetNamedProp = 156,
    SetNamedProp = 157,
    Call0 = 174,
    Call1 = 175,
    Call2 = 177,
    Call3 = 178,
    CallNamedMethod = 179,

    MathSin = 180,
    MathCos = 181,
    MathSqrt = 182,
    MathAbs = 183,
    MathFloor = 184,
    MathCeil = 185,
    MathRound = 186,
    MathPow = 187,
    MathMin = 188,
    MathMax = 189,

    DefineAccessor = 190,

    DefinePrivateAccessor = 191,

    SetMethodProp = 192,

    Pos = 193,
    DefineGlobal = 194,
    DeleteGlobal = 195,
    ThrowReferenceError = 196,
    SetGlobalVar = 197,
    InitGlobalVar = 198,

    LtJumpIfNot = 158,
    LtJumpIf = 159,
    LteJumpIfNot = 160,
    LteJumpIf = 161,
    GtJumpIfNot = 162,
    GtJumpIf = 163,
    GteJumpIfNot = 164,
    GteJumpIf = 165,
    EqJumpIfNot = 166,
    EqJumpIf = 167,
    NeqJumpIfNot = 168,
    NeqJumpIf = 169,
    StrictEqJumpIfNot = 170,
    StrictEqJumpIf = 171,
    StrictNeqJumpIfNot = 172,
    StrictNeqJumpIf = 173,
}

impl Opcode {
    #[inline(always)]
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Opcode::Nop),
            1 => Some(Opcode::End),
            2 => Some(Opcode::Return),
            10 => Some(Opcode::LoadConst),
            11 => Some(Opcode::LoadInt),
            12 => Some(Opcode::LoadInt8),
            13 => Some(Opcode::LoadTrue),
            14 => Some(Opcode::LoadFalse),
            15 => Some(Opcode::LoadNull),
            16 => Some(Opcode::LoadUndefined),
            17 => Some(Opcode::LoadTdz),
            18 => Some(Opcode::CheckTdz),
            19 => Some(Opcode::CheckRef),
            91 => Some(Opcode::CheckObjectCoercible),
            20 => Some(Opcode::Add),
            21 => Some(Opcode::Sub),
            34 => Some(Opcode::SubImm8),
            35 => Some(Opcode::AddNum),
            36 => Some(Opcode::SubNum),
            37 => Some(Opcode::MulNum),
            38 => Some(Opcode::DivNum),
            39 => Some(Opcode::AddImm8),
            22 => Some(Opcode::Mul),
            23 => Some(Opcode::Div),
            24 => Some(Opcode::Mod),
            25 => Some(Opcode::Pow),
            26 => Some(Opcode::Neg),
            27 => Some(Opcode::BitAnd),
            28 => Some(Opcode::BitOr),
            29 => Some(Opcode::BitXor),
            30 => Some(Opcode::BitNot),
            31 => Some(Opcode::Shl),
            32 => Some(Opcode::Shr),
            33 => Some(Opcode::UShr),
            50 => Some(Opcode::Lt),
            51 => Some(Opcode::Lte),
            60 => Some(Opcode::LteImm8),
            52 => Some(Opcode::Gt),
            53 => Some(Opcode::Gte),
            54 => Some(Opcode::Eq),
            55 => Some(Opcode::Neq),
            56 => Some(Opcode::StrictEq),
            57 => Some(Opcode::StrictNeq),
            58 => Some(Opcode::InstanceOf),
            59 => Some(Opcode::NewRegExp),
            70 => Some(Opcode::Not),
            71 => Some(Opcode::TypeOf),
            80 => Some(Opcode::Jump),
            81 => Some(Opcode::JumpIf),
            82 => Some(Opcode::JumpIfNot),
            176 => Some(Opcode::JumpIfNullish),
            83 => Some(Opcode::Throw),
            84 => Some(Opcode::Try),
            85 => Some(Opcode::Catch),
            86 => Some(Opcode::Finally),
            87 => Some(Opcode::Jump8),
            88 => Some(Opcode::JumpIf8),
            89 => Some(Opcode::JumpIfNot8),
            90 => Some(Opcode::Move),
            100 => Some(Opcode::GetLocal),
            101 => Some(Opcode::SetLocal),
            102 => Some(Opcode::GetGlobal),
            103 => Some(Opcode::SetGlobal),
            104 => Some(Opcode::GetUpvalue),
            105 => Some(Opcode::SetUpvalue),
            106 => Some(Opcode::IncLocal),
            107 => Some(Opcode::DecLocal),
            120 => Some(Opcode::NewObject),
            121 => Some(Opcode::NewArray),
            122 => Some(Opcode::GetField),
            123 => Some(Opcode::SetField),
            124 => Some(Opcode::GetProp),
            125 => Some(Opcode::SetProp),
            126 => Some(Opcode::Call),
            127 => Some(Opcode::CallMethod),
            128 => Some(Opcode::NewFunction),
            129 => Some(Opcode::CallNew),
            130 => Some(Opcode::DeleteProp),
            131 => Some(Opcode::HasProperty),
            132 => Some(Opcode::GatherRest),
            133 => Some(Opcode::ObjectSpread),
            134 => Some(Opcode::GetPropertyNames),
            135 => Some(Opcode::ArrayExtend),
            136 => Some(Opcode::SetProto),
            137 => Some(Opcode::GetSuper),
            138 => Some(Opcode::ArrayPush),
            139 => Some(Opcode::CallSpread),
            140 => Some(Opcode::CallMethodSpread),
            141 => Some(Opcode::CallNewSpread),
            142 => Some(Opcode::GetPrivate),
            143 => Some(Opcode::SetPrivate),
            144 => Some(Opcode::HasPrivate),
            145 => Some(Opcode::Yield),
            146 => Some(Opcode::NewGeneratorFunction),
            147 => Some(Opcode::NewAsyncFunction),
            148 => Some(Opcode::Await),
            149 => Some(Opcode::NewAsyncGeneratorFunction),
            150 => Some(Opcode::GetIterator),
            151 => Some(Opcode::IteratorNext),
            152 => Some(Opcode::GetArguments),
            153 => Some(Opcode::GetCurrentFunction),
            154 => Some(Opcode::CallCurrent),
            155 => Some(Opcode::CallCurrent1),
            156 => Some(Opcode::GetNamedProp),
            157 => Some(Opcode::SetNamedProp),
            174 => Some(Opcode::Call0),
            175 => Some(Opcode::Call1),
            177 => Some(Opcode::Call2),
            178 => Some(Opcode::Call3),
            179 => Some(Opcode::CallNamedMethod),
            180 => Some(Opcode::MathSin),
            181 => Some(Opcode::MathCos),
            182 => Some(Opcode::MathSqrt),
            183 => Some(Opcode::MathAbs),
            184 => Some(Opcode::MathFloor),
            185 => Some(Opcode::MathCeil),
            186 => Some(Opcode::MathRound),
            187 => Some(Opcode::MathPow),
            188 => Some(Opcode::MathMin),
            189 => Some(Opcode::MathMax),
            190 => Some(Opcode::DefineAccessor),
            191 => Some(Opcode::DefinePrivateAccessor),
            192 => Some(Opcode::SetMethodProp),
            193 => Some(Opcode::Pos),
            194 => Some(Opcode::DefineGlobal),
            195 => Some(Opcode::DeleteGlobal),
            196 => Some(Opcode::ThrowReferenceError),
            197 => Some(Opcode::SetGlobalVar),
            198 => Some(Opcode::InitGlobalVar),
            158 => Some(Opcode::LtJumpIfNot),
            159 => Some(Opcode::LtJumpIf),
            160 => Some(Opcode::LteJumpIfNot),
            161 => Some(Opcode::LteJumpIf),
            162 => Some(Opcode::GtJumpIfNot),
            163 => Some(Opcode::GtJumpIf),
            164 => Some(Opcode::GteJumpIfNot),
            165 => Some(Opcode::GteJumpIf),
            166 => Some(Opcode::EqJumpIfNot),
            167 => Some(Opcode::EqJumpIf),
            168 => Some(Opcode::NeqJumpIfNot),
            169 => Some(Opcode::NeqJumpIf),
            170 => Some(Opcode::StrictEqJumpIfNot),
            171 => Some(Opcode::StrictEqJumpIf),
            172 => Some(Opcode::StrictNeqJumpIfNot),
            173 => Some(Opcode::StrictNeqJumpIf),
            _ => None,
        }
    }

    #[inline(always)]
    pub fn from_u8_unchecked(v: u8) -> Self {
        Self::from_u8(v).unwrap_or(Opcode::Nop)
    }

    pub fn instruction_size(op: Opcode) -> usize {
        match op {
            Opcode::Nop | Opcode::End | Opcode::Catch | Opcode::Finally => 1,
            Opcode::Return => 3,
            Opcode::LoadTrue
            | Opcode::LoadFalse
            | Opcode::LoadNull
            | Opcode::LoadUndefined
            | Opcode::LoadTdz
            | Opcode::CheckTdz
            | Opcode::IncLocal
            | Opcode::DecLocal
            | Opcode::Throw
            | Opcode::GatherRest
            | Opcode::GetSuper
            | Opcode::Yield
            | Opcode::NewObject
            | Opcode::GetArguments
            | Opcode::GetCurrentFunction
            | Opcode::CheckObjectCoercible => 3,
            Opcode::Neg | Opcode::BitNot | Opcode::Not | Opcode::TypeOf | Opcode::Move => 5,
            Opcode::Add
            | Opcode::AddNum
            | Opcode::SubNum
            | Opcode::MulNum
            | Opcode::DivNum
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Mod
            | Opcode::Pow
            | Opcode::BitAnd
            | Opcode::BitOr
            | Opcode::BitXor
            | Opcode::Shl
            | Opcode::Shr
            | Opcode::UShr
            | Opcode::Lt
            | Opcode::Lte
            | Opcode::Gt
            | Opcode::Gte
            | Opcode::Eq
            | Opcode::Neq
            | Opcode::StrictEq
            | Opcode::StrictNeq
            | Opcode::InstanceOf => 7,
            Opcode::SubImm8 | Opcode::AddImm8 | Opcode::LteImm8 => 6,
            Opcode::NewRegExp => 11,
            Opcode::LoadConst | Opcode::LoadInt => 7,
            Opcode::LoadInt8 => 4,
            Opcode::Jump => 5,
            Opcode::JumpIf | Opcode::JumpIfNot | Opcode::JumpIfNullish => 7,
            Opcode::Jump8 => 2,
            Opcode::JumpIf8 | Opcode::JumpIfNot8 => 4,
            Opcode::Try => 9,
            Opcode::GetLocal
            | Opcode::SetLocal
            | Opcode::GetGlobal
            | Opcode::SetGlobal
            | Opcode::CheckRef => 7,
            Opcode::GetUpvalue | Opcode::SetUpvalue => 5,
            Opcode::NewArray => 5,
            Opcode::GetField | Opcode::GetProp => 7,
            Opcode::SetField | Opcode::SetProp => 7,
            Opcode::GetNamedProp | Opcode::SetNamedProp => 9,

            Opcode::LtJumpIfNot
            | Opcode::LtJumpIf
            | Opcode::LteJumpIfNot
            | Opcode::LteJumpIf
            | Opcode::GtJumpIfNot
            | Opcode::GtJumpIf
            | Opcode::GteJumpIfNot
            | Opcode::GteJumpIf
            | Opcode::EqJumpIfNot
            | Opcode::EqJumpIf
            | Opcode::NeqJumpIfNot
            | Opcode::NeqJumpIf
            | Opcode::StrictEqJumpIfNot
            | Opcode::StrictEqJumpIf
            | Opcode::StrictNeqJumpIfNot
            | Opcode::StrictNeqJumpIf => 9,
            Opcode::Call | Opcode::CallNew => 7,
            Opcode::Call0 => 5,
            Opcode::Call1 => 7,
            Opcode::Call2 => 9,
            Opcode::Call3 => 11,
            Opcode::CallCurrent => 5,
            Opcode::CallCurrent1 => 5,
            Opcode::CallMethod => 9,
            Opcode::CallNamedMethod => 11,
            Opcode::MathSin
            | Opcode::MathCos
            | Opcode::MathSqrt
            | Opcode::MathAbs
            | Opcode::MathFloor
            | Opcode::MathCeil
            | Opcode::MathRound => 5,
            Opcode::MathPow | Opcode::MathMin | Opcode::MathMax => 7,
            Opcode::DefineAccessor => 9,
            Opcode::DefinePrivateAccessor => 9,
            Opcode::SetMethodProp => 7,
            Opcode::Pos => 5,
            Opcode::DefineGlobal => 7,
            Opcode::DeleteGlobal => 7,
            Opcode::ThrowReferenceError => 5,
            Opcode::SetGlobalVar => 7,
            Opcode::InitGlobalVar => 7,
            Opcode::NewFunction => 1,
            Opcode::DeleteProp => 7,
            Opcode::HasProperty => 7,
            Opcode::ObjectSpread => 5,
            Opcode::GetPropertyNames => 5,
            Opcode::ArrayExtend => 5,
            Opcode::SetProto => 5,
            Opcode::ArrayPush => 5,
            Opcode::CallSpread => 7,
            Opcode::CallMethodSpread => 9,
            Opcode::CallNewSpread => 7,
            Opcode::GetPrivate => 7,
            Opcode::SetPrivate => 7,
            Opcode::HasPrivate => 7,
            Opcode::NewGeneratorFunction => 1,
            Opcode::NewAsyncFunction => 1,
            Opcode::NewAsyncGeneratorFunction => 1,
            Opcode::Await => 3,
            Opcode::GetIterator => 5,
            Opcode::IteratorNext => 7,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bytecode {
    pub code: Vec<u8>,
    pub constants: Vec<crate::value::JSValue>,
    pub locals_count: u32,
    pub param_count: u16,
    pub line_number_table: Option<crate::compiler::location::LineNumberTable>,
    pub ic_table: crate::compiler::InlineCacheTable,
    
    pub shared_ic_table_ptr: *mut crate::compiler::InlineCacheTable,
    
    pub shared_code_ptr: *const u8,
    pub shared_code_len: usize,
    pub shared_const_ptr: *const crate::value::JSValue,
    pub shared_const_len: usize,
    pub uses_arguments: bool,
    pub is_strict: bool,
    pub var_name_to_slot: std::rc::Rc<Vec<(u32, u16)>>,
    pub nested_bytecodes: std::collections::HashMap<u32, std::sync::Arc<NestedBytecode>>,
    pub is_simple_constructor: bool,
    pub simple_constructor_props: Vec<(crate::runtime::atom::Atom, u16, u16)>,
    pub cached_constructor_atoms: Vec<crate::runtime::atom::Atom>,
    pub cached_constructor_final_shape: Option<std::ptr::NonNull<crate::object::shape::Shape>>,
}

unsafe impl Send for Bytecode {}
unsafe impl Sync for Bytecode {}

pub struct NestedBytecode {
    pub code: Vec<u8>,
    pub constants: Vec<crate::value::JSValue>,
    pub locals_count: u32,
    pub param_count: u16,
    pub uses_arguments: bool,
    pub is_strict: bool,
    pub var_name_to_slot: std::rc::Rc<Vec<(u32, u16)>>,
    pub line_number_table: Option<crate::compiler::location::LineNumberTable>,

    pub parent_bytecode_span: u32,

    pub upvalue_count: u32,

    pub upvalue_descs: Vec<(u32, u32)>,

    pub func_name_atom: u32,

    pub ic_table: std::cell::UnsafeCell<crate::compiler::InlineCacheTable>,
}

unsafe impl Sync for NestedBytecode {}
unsafe impl Send for NestedBytecode {}

impl std::fmt::Debug for NestedBytecode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NestedBytecode")
            .field("code_len", &self.code.len())
            .field("locals_count", &self.locals_count)
            .field("param_count", &self.param_count)
            .finish()
    }
}

impl Bytecode {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            locals_count: 0,
            param_count: 0,
            uses_arguments: false,
            is_strict: false,
            var_name_to_slot: std::rc::Rc::new(Vec::new()),
            line_number_table: None,
            ic_table: crate::compiler::InlineCacheTable::new(),
            shared_ic_table_ptr: std::ptr::null_mut(),
            shared_code_ptr: std::ptr::null(),
            shared_code_len: 0,
            shared_const_ptr: std::ptr::null(),
            shared_const_len: 0,
            nested_bytecodes: std::collections::HashMap::new(),
            is_simple_constructor: false,
            simple_constructor_props: Vec::new(),
            cached_constructor_final_shape: None,
            cached_constructor_atoms: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn effective_ic_table_ptr(&self) -> *mut crate::compiler::InlineCacheTable {
        if !self.shared_ic_table_ptr.is_null() {
            self.shared_ic_table_ptr
        } else {
            &self.ic_table as *const crate::compiler::InlineCacheTable
                as *mut crate::compiler::InlineCacheTable
        }
    }

    #[inline(always)]
    pub fn effective_code_ptr(&self) -> *const u8 {
        if !self.shared_code_ptr.is_null() {
            self.shared_code_ptr
        } else {
            self.code.as_ptr()
        }
    }

    #[inline(always)]
    pub fn effective_code_len(&self) -> usize {
        if !self.shared_code_ptr.is_null() {
            self.shared_code_len
        } else {
            self.code.len()
        }
    }

    #[inline(always)]
    pub fn effective_const_ptr(&self) -> *const crate::value::JSValue {
        if !self.shared_const_ptr.is_null() {
            self.shared_const_ptr
        } else {
            self.constants.as_ptr()
        }
    }

    #[inline(always)]
    pub fn effective_const_len(&self) -> usize {
        if !self.shared_const_ptr.is_null() {
            self.shared_const_len
        } else {
            self.constants.len()
        }
    }

    pub fn disassemble(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        writeln!(out, "=== Register Bytecode ===").unwrap();
        writeln!(
            out,
            "locals_count: {}, param_count: {}",
            self.locals_count, self.param_count
        )
        .unwrap();
        let mut pc = 0usize;
        while pc < self.code.len() {
            let op_val = self.code[pc];
            let op = Opcode::from_u8_unchecked(op_val);
            let size = Opcode::instruction_size(op);
            let remaining = self.code.len().saturating_sub(pc + 1);

            let read_u8 = |offset: usize| -> u8 {
                if offset <= remaining {
                    self.code.get(pc + offset).copied().unwrap_or(0)
                } else {
                    0
                }
            };
            let read_i32 = |offset: usize| -> i32 {
                let bytes = [
                    read_u8(offset),
                    read_u8(offset + 1),
                    read_u8(offset + 2),
                    read_u8(offset + 3),
                ];
                i32::from_le_bytes(bytes)
            };
            let read_u32 = |offset: usize| -> u32 { read_i32(offset) as u32 };
            let read_u16 = |offset: usize| -> u16 {
                u16::from_le_bytes([read_u8(offset), read_u8(offset + 1)])
            };
            let read_i8 = |offset: usize| -> i8 { read_u8(offset) as i8 };

            let mut operands = String::new();
            match op {
                Opcode::Nop | Opcode::End | Opcode::Catch | Opcode::Finally => {}
                Opcode::Return
                | Opcode::LoadTrue
                | Opcode::LoadFalse
                | Opcode::LoadNull
                | Opcode::LoadUndefined
                | Opcode::IncLocal
                | Opcode::DecLocal => {
                    let a = read_u16(1);
                    write!(operands, " r{}", a).unwrap();
                }
                Opcode::LoadConst => {
                    let dst = read_u16(1);
                    let idx = read_u32(3);
                    write!(operands, " r{}, const[{}]", dst, idx).unwrap();
                }
                Opcode::LoadInt => {
                    let dst = read_u16(1);
                    let val = read_i32(3);
                    write!(operands, " r{}, {}", dst, val).unwrap();
                }
                Opcode::LoadInt8 => {
                    let dst = read_u16(1);
                    let val = read_u8(3) as i8;
                    write!(operands, " r{}, {}", dst, val).unwrap();
                }
                Opcode::Neg
                | Opcode::BitNot
                | Opcode::Not
                | Opcode::TypeOf
                | Opcode::Throw
                | Opcode::GatherRest
                | Opcode::GetSuper
                | Opcode::Yield
                | Opcode::NewObject
                | Opcode::GetArguments
                | Opcode::GetCurrentFunction => {
                    let a = read_u16(1);
                    write!(operands, " r{}", a).unwrap();
                }
                Opcode::Add
                | Opcode::AddNum
                | Opcode::SubNum
                | Opcode::MulNum
                | Opcode::DivNum
                | Opcode::Sub
                | Opcode::Mul
                | Opcode::Div
                | Opcode::Mod
                | Opcode::Pow
                | Opcode::BitAnd
                | Opcode::BitOr
                | Opcode::BitXor
                | Opcode::Shl
                | Opcode::Shr
                | Opcode::UShr
                | Opcode::Lt
                | Opcode::Lte
                | Opcode::Gt
                | Opcode::Gte
                | Opcode::Eq
                | Opcode::Neq
                | Opcode::StrictEq
                | Opcode::StrictNeq => {
                    let dst = read_u16(1);
                    let src1 = read_u16(3);
                    let src2 = read_u16(5);
                    write!(operands, " r{}, r{}, r{}", dst, src1, src2).unwrap();
                }
                Opcode::InstanceOf => {
                    let dst = read_u16(1);
                    let obj = read_u16(3);
                    let ctor = read_u16(5);
                    write!(operands, " r{}, r{}, r{}", dst, obj, ctor).unwrap();
                }
                Opcode::NewRegExp => {
                    let dst = read_u16(1);
                    let pattern_idx = read_u32(3);
                    let flags_idx = read_u32(7);
                    write!(
                        operands,
                        " r{}, pattern={}, flags={}",
                        dst, pattern_idx, flags_idx
                    )
                    .unwrap();
                }
                Opcode::SubImm8 | Opcode::AddImm8 | Opcode::LteImm8 => {
                    let dst = read_u16(1);
                    let src = read_u16(3);
                    let imm = read_i8(5);
                    write!(operands, " r{}, r{}, {}", dst, src, imm).unwrap();
                }
                Opcode::Jump => {
                    let off = read_i32(1);
                    let size = Opcode::instruction_size(Opcode::Jump) as i32;
                    write!(operands, " {} (pc {})", off, pc as i32 + size + off).unwrap();
                }
                Opcode::JumpIf | Opcode::JumpIfNot | Opcode::JumpIfNullish => {
                    let src = read_u16(1);
                    let off = read_i32(3);
                    let size = Opcode::instruction_size(Opcode::JumpIf) as i32;
                    write!(
                        operands,
                        " r{}, {} (pc {})",
                        src,
                        off,
                        pc as i32 + size + off
                    )
                    .unwrap();
                }
                Opcode::Jump8 => {
                    let off = read_i8(1);
                    let size = Opcode::instruction_size(Opcode::Jump8) as i32;
                    write!(operands, " {} (pc {})", off, pc as i32 + size + off as i32).unwrap();
                }
                Opcode::JumpIf8 | Opcode::JumpIfNot8 => {
                    let src = read_u16(1);
                    let off = read_i8(3);
                    let size = Opcode::instruction_size(Opcode::JumpIf8) as i32;
                    write!(
                        operands,
                        " r{}, {} (pc {})",
                        src,
                        off,
                        pc as i32 + size + off as i32
                    )
                    .unwrap();
                }
                Opcode::Try => {
                    let catch = read_i32(1);
                    let finally = read_i32(5);
                    write!(operands, " catch={} finally={}", catch, finally).unwrap();
                }
                Opcode::Move => {
                    let dst = read_u16(1);
                    let src = read_u16(3);
                    write!(operands, " r{}, r{}", dst, src).unwrap();
                }
                Opcode::GetLocal | Opcode::GetGlobal => {
                    let dst = read_u16(1);
                    let idx = read_u32(3);
                    write!(operands, " r{}, {}", dst, idx).unwrap();
                }
                Opcode::SetLocal | Opcode::SetGlobal => {
                    let idx = read_u32(1);
                    let src = read_u16(5);
                    write!(operands, " {}, r{}", idx, src).unwrap();
                }
                Opcode::GetUpvalue | Opcode::SetUpvalue => {
                    let a = read_u16(1);
                    let b = read_u16(3);
                    write!(operands, " r{}, r{}", a, b).unwrap();
                }
                Opcode::NewArray => {
                    let dst = read_u16(1);
                    let count = read_u16(3);
                    write!(operands, " r{}, {}", dst, count).unwrap();
                }
                Opcode::GetField
                | Opcode::GetProp
                | Opcode::DeleteProp
                | Opcode::HasProperty
                | Opcode::GetPrivate
                | Opcode::HasPrivate => {
                    let dst = read_u16(1);
                    let obj = read_u16(3);
                    let key = read_u16(5);
                    write!(operands, " r{}, r{}, r{}", dst, obj, key).unwrap();
                }
                Opcode::GetNamedProp => {
                    let dst = read_u16(1);
                    let obj = read_u16(3);
                    let atom = read_u32(5);
                    write!(operands, " r{}, r{}, atom({})", dst, obj, atom).unwrap();
                }
                Opcode::SetField | Opcode::SetProp | Opcode::SetPrivate => {
                    let obj = read_u16(1);
                    let key = read_u16(3);
                    let val = read_u16(5);
                    write!(operands, " r{}, r{}, r{}", obj, key, val).unwrap();
                }
                Opcode::SetNamedProp => {
                    let obj = read_u16(1);
                    let val = read_u16(3);
                    let atom = read_u32(5);
                    write!(operands, " r{}, r{}, atom({})", obj, val, atom).unwrap();
                }
                Opcode::LtJumpIfNot
                | Opcode::LtJumpIf
                | Opcode::LteJumpIfNot
                | Opcode::LteJumpIf
                | Opcode::GtJumpIfNot
                | Opcode::GtJumpIf
                | Opcode::GteJumpIfNot
                | Opcode::GteJumpIf
                | Opcode::EqJumpIfNot
                | Opcode::EqJumpIf
                | Opcode::NeqJumpIfNot
                | Opcode::NeqJumpIf
                | Opcode::StrictEqJumpIfNot
                | Opcode::StrictEqJumpIf
                | Opcode::StrictNeqJumpIfNot
                | Opcode::StrictNeqJumpIf => {
                    let a = read_u16(3);
                    let b = read_u16(5);
                    let offset = read_i32(10);
                    let target = (pc as i64 + 14 + offset as i64) as usize;
                    write!(operands, " r{}, r{} -> {}", a, b, target).unwrap();
                }
                Opcode::Call | Opcode::CallNew => {
                    let dst = read_u16(1);
                    let func = read_u16(3);
                    let argc = read_u16(5);
                    write!(operands, " r{}, r{}, argc={}", dst, func, argc).unwrap();
                }
                Opcode::Call0 => {
                    let dst = read_u16(1);
                    let func = read_u16(3);
                    write!(operands, " r{}, r{}", dst, func).unwrap();
                }
                Opcode::Call1 => {
                    let dst = read_u16(1);
                    let func = read_u16(3);
                    let arg = read_u16(5);
                    write!(operands, " r{}, r{}, r{}", dst, func, arg).unwrap();
                }
                Opcode::Call2 => {
                    let dst = read_u16(1);
                    let func = read_u16(3);
                    let arg0 = read_u16(5);
                    let arg1 = read_u16(7);
                    write!(operands, " r{}, r{}, r{}, r{}", dst, func, arg0, arg1).unwrap();
                }
                Opcode::Call3 => {
                    let dst = read_u16(1);
                    let func = read_u16(3);
                    let arg0 = read_u16(5);
                    let arg1 = read_u16(7);
                    let arg2 = read_u16(9);
                    write!(
                        operands,
                        " r{}, r{}, r{}, r{}, r{}",
                        dst, func, arg0, arg1, arg2
                    )
                    .unwrap();
                }
                Opcode::CallCurrent => {
                    let dst = read_u16(1);
                    let argc = read_u16(3);
                    write!(operands, " r{}, argc={}", dst, argc).unwrap();
                }
                Opcode::CallCurrent1 => {
                    let dst = read_u16(1);
                    let arg = read_u16(3);
                    write!(operands, " r{}, r{}", dst, arg).unwrap();
                }
                Opcode::CallMethod => {
                    let dst = read_u16(1);
                    let obj = read_u16(3);
                    let func = read_u16(5);
                    let argc = read_u16(7);
                    write!(operands, " r{}, r{}, r{}, argc={}", dst, obj, func, argc).unwrap();
                }
                Opcode::CallNamedMethod => {
                    let dst = read_u16(1);
                    let obj = read_u16(3);
                    let atom = read_u32(5);
                    let argc = read_u16(9);
                    write!(
                        operands,
                        " r{}, r{}, atom={}, argc={}",
                        dst, obj, atom, argc
                    )
                    .unwrap();
                }
                Opcode::NewFunction
                | Opcode::NewGeneratorFunction
                | Opcode::NewAsyncFunction
                | Opcode::NewAsyncGeneratorFunction => {
                    write!(operands, " <embedded>").unwrap();
                }
                Opcode::ObjectSpread
                | Opcode::GetPropertyNames
                | Opcode::ArrayExtend
                | Opcode::SetProto
                | Opcode::ArrayPush => {
                    let a = read_u16(1);
                    let b = read_u16(3);
                    write!(operands, " r{}, r{}", a, b).unwrap();
                }
                Opcode::CallSpread | Opcode::CallNewSpread => {
                    let dst = read_u16(1);
                    let func = read_u16(3);
                    let arr = read_u16(5);
                    write!(operands, " r{}, r{}, r{}", dst, func, arr).unwrap();
                }
                Opcode::CallMethodSpread => {
                    let dst = read_u16(1);
                    let obj = read_u16(3);
                    let func = read_u16(5);
                    let arr = read_u16(7);
                    write!(operands, " r{}, r{}, r{}, r{}", dst, obj, func, arr).unwrap();
                }
                Opcode::MathSin
                | Opcode::MathCos
                | Opcode::MathSqrt
                | Opcode::MathAbs
                | Opcode::MathFloor
                | Opcode::MathCeil
                | Opcode::MathRound => {
                    let dst = read_u16(1);
                    let src = read_u16(3);
                    write!(operands, " r{}, r{}", dst, src).unwrap();
                }
                Opcode::MathPow | Opcode::MathMin | Opcode::MathMax => {
                    let dst = read_u16(1);
                    let a = read_u16(3);
                    let b = read_u16(5);
                    write!(operands, " r{}, r{}, r{}", dst, a, b).unwrap();
                }
                Opcode::DefineAccessor => {
                    let obj = read_u16(1);
                    let key = read_u16(3);
                    let getter = read_u16(5);
                    let setter = read_u16(7);
                    write!(operands, " r{}, r{}, r{}, r{}", obj, key, getter, setter).unwrap();
                }
                Opcode::DefinePrivateAccessor => {
                    let obj = read_u16(1);
                    let key = read_u16(3);
                    let getter = read_u16(5);
                    let setter = read_u16(7);
                    write!(operands, " r{}, r{}, r{}, r{}", obj, key, getter, setter).unwrap();
                }
                Opcode::SetMethodProp => {
                    let obj = read_u16(1);
                    let key = read_u16(3);
                    let val = read_u16(5);
                    write!(operands, " r{}, r{}, r{}", obj, key, val).unwrap();
                }
                Opcode::Pos => {
                    let dst = read_u16(1);
                    let src = read_u16(3);
                    write!(operands, " r{}, r{}", dst, src).unwrap();
                }
                Opcode::DefineGlobal => {
                    let idx = read_u32(1);
                    let src = read_u16(5);
                    write!(operands, " {}, r{}", idx, src).unwrap();
                }
                Opcode::DeleteGlobal => {
                    let dst = read_u16(1);
                    let idx = read_u32(3);
                    write!(operands, " r{}, {}", dst, idx).unwrap();
                }
                Opcode::ThrowReferenceError => {
                    let idx = read_u32(1);
                    write!(operands, " const[{}]", idx).unwrap();
                }
                Opcode::SetGlobalVar => {
                    let idx = read_u32(1);
                    let src = read_u16(5);
                    write!(operands, " {}, r{}", idx, src).unwrap();
                }
                Opcode::InitGlobalVar => {
                    let idx = read_u32(1);
                    let src = read_u16(5);
                    write!(operands, " {}, r{}", idx, src).unwrap();
                }
                Opcode::Await => {
                    let src = read_u16(1);
                    write!(operands, " r{}", src).unwrap();
                }
                Opcode::LoadTdz | Opcode::CheckTdz => {
                    let a = read_u16(1);
                    write!(operands, " r{}", a).unwrap();
                }
                Opcode::CheckRef => {
                    let a = read_u16(1);
                    let idx = read_u32(3);
                    write!(operands, " r{}, const[{}]", a, idx).unwrap();
                }
                Opcode::CheckObjectCoercible => {
                    let a = read_u16(1);
                    write!(operands, " r{}", a).unwrap();
                }
                Opcode::GetIterator => {
                    let dst = read_u16(1);
                    let src = read_u16(3);
                    write!(operands, " r{}, r{}", dst, src).unwrap();
                }
                Opcode::IteratorNext => {
                    let dst_val = read_u16(1);
                    let dst_done = read_u16(3);
                    let iter = read_u16(5);
                    write!(operands, " r{}, r{}, r{}", dst_val, dst_done, iter).unwrap();
                }
            }
            writeln!(out, "{:04}  {:?}{}", pc, op, operands).unwrap();
            pc += size.max(1);
        }
        out
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();

        out.extend_from_slice(b"SAFC");

        out.extend_from_slice(&1u32.to_le_bytes());

        out.extend_from_slice(&self.locals_count.to_le_bytes());

        out.extend_from_slice(&self.param_count.to_le_bytes());

        out.push(self.uses_arguments as u8);

        out.extend_from_slice(&(self.code.len() as u64).to_le_bytes());

        out.extend_from_slice(&self.code);

        out.extend_from_slice(&(self.constants.len() as u64).to_le_bytes());

        for c in &self.constants {
            out.extend_from_slice(&c.raw_bits().to_le_bytes());
        }
        out
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        if data.len() < 8 {
            return Err("Invalid .jsc: too short".to_string());
        }
        if &data[0..4] != b"SAFC" {
            return Err("Invalid .jsc: bad magic".to_string());
        }
        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        if version != 1 {
            return Err(format!("Unsupported .jsc version: {}", version));
        }
        let mut offset = 8usize;
        let read_u32 = |o: &mut usize| -> u32 {
            let v = u32::from_le_bytes([data[*o], data[*o + 1], data[*o + 2], data[*o + 3]]);
            *o += 4;
            v
        };
        let read_u16 = |o: &mut usize| -> u16 {
            let v = u16::from_le_bytes([data[*o], data[*o + 1]]);
            *o += 2;
            v
        };
        let read_u64 = |o: &mut usize| -> u64 {
            let v = u64::from_le_bytes([
                data[*o],
                data[*o + 1],
                data[*o + 2],
                data[*o + 3],
                data[*o + 4],
                data[*o + 5],
                data[*o + 6],
                data[*o + 7],
            ]);
            *o += 8;
            v
        };
        let locals_count = read_u32(&mut offset);
        let param_count = read_u16(&mut offset);
        let uses_arguments = data[offset] != 0;
        offset += 1;
        let code_len = read_u64(&mut offset) as usize;
        if offset + code_len > data.len() {
            return Err("Invalid .jsc: code truncated".to_string());
        }
        let code = data[offset..offset + code_len].to_vec();
        offset += code_len;
        let constants_count = read_u64(&mut offset) as usize;
        if offset + constants_count * 8 > data.len() {
            return Err("Invalid .jsc: constants truncated".to_string());
        }
        let mut constants = Vec::with_capacity(constants_count);
        for _ in 0..constants_count {
            let bits = read_u64(&mut offset);
            constants.push(crate::value::JSValue::from_raw_bits(bits));
        }
        Ok(Self {
            code,
            constants,
            locals_count,
            param_count,
            uses_arguments,
            is_strict: false,
            var_name_to_slot: std::rc::Rc::new(Vec::new()),
            line_number_table: None,
            ic_table: crate::compiler::InlineCacheTable::new(),
            shared_ic_table_ptr: std::ptr::null_mut(),
            shared_code_ptr: std::ptr::null(),
            shared_code_len: 0,
            shared_const_ptr: std::ptr::null(),
            shared_const_len: 0,
            nested_bytecodes: std::collections::HashMap::new(),
            is_simple_constructor: false,
            simple_constructor_props: Vec::new(),
            cached_constructor_final_shape: None,
            cached_constructor_atoms: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_roundtrip() {
        let ops = [
            Opcode::Nop,
            Opcode::End,
            Opcode::Return,
            Opcode::LoadConst,
            Opcode::LoadInt,
            Opcode::LoadTrue,
            Opcode::Add,
            Opcode::Sub,
            Opcode::Mul,
            Opcode::Div,
            Opcode::Jump,
            Opcode::JumpIf,
            Opcode::JumpIfNot,
            Opcode::GetLocal,
            Opcode::SetLocal,
            Opcode::IncLocal,
            Opcode::DecLocal,
            Opcode::Call,
            Opcode::CallMethod,
            Opcode::NewFunction,
            Opcode::JumpIfNullish,
            Opcode::Call2,
            Opcode::Call3,
            Opcode::Try,
        ];
        for op in ops {
            let byte = op as u8;
            let recovered = Opcode::from_u8(byte).expect("roundtrip failed");
            assert_eq!(op, recovered, "Opcode {:?} did not round-trip", op);
        }
    }

    #[test]
    fn test_disassemble_basic_program() {
        let bytecode = Bytecode {
            code: vec![
                Opcode::LoadInt as u8,
                0x00,
                0x00,
                0x0A,
                0x00,
                0x00,
                0x00,
                Opcode::LoadInt as u8,
                0x01,
                0x00,
                0x14,
                0x00,
                0x00,
                0x00,
                Opcode::Add as u8,
                0x02,
                0x00,
                0x00,
                0x00,
                0x01,
                0x00,
                Opcode::Return as u8,
                0x02,
                0x00,
            ],
            constants: vec![],
            locals_count: 3,
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
        };

        let output = bytecode.disassemble();
        assert!(output.contains("=== Register Bytecode ==="));
        assert!(
            output.contains("LoadInt r0, 10"),
            "Basic disasm wrong: {}",
            output
        );
        assert!(
            output.contains("LoadInt r1, 20"),
            "Basic disasm wrong: {}",
            output
        );
        assert!(
            output.contains("Add r2, r0, r1"),
            "Basic disasm wrong: {}",
            output
        );
        assert!(output.contains("Return"), "Basic disasm wrong: {}", output);
    }

    #[test]
    fn test_disassemble_jump_offsets() {
        let bytecode = Bytecode {
            code: vec![
                Opcode::LoadFalse as u8,
                0x00,
                0x00,
                Opcode::JumpIfNot as u8,
                0x00,
                0x00,
                0x05,
                0x00,
                0x00,
                0x00,
                Opcode::LoadTrue as u8,
                0x01,
                0x00,
                Opcode::Jump as u8,
                0x00,
                0x00,
                0x00,
                0x00,
            ],
            constants: vec![],
            locals_count: 1,
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
        };

        let output = bytecode.disassemble();

        assert!(
            output.contains("JumpIfNot r0, 5 (pc 15)"),
            "JumpIfNot disasm wrong: {}",
            output
        );

        assert!(
            output.contains("Jump 0 (pc 18)"),
            "Jump disasm wrong: {}",
            output
        );
    }

    #[test]
    fn test_disassemble_call_and_object_ops() {
        let bytecode = Bytecode {
            code: vec![
                Opcode::NewObject as u8,
                0x00,
                0x00,
                Opcode::Call as u8,
                0x01,
                0x00,
                0x02,
                0x00,
                0x00,
                0x00,
                Opcode::SetField as u8,
                0x00,
                0x00,
                0x01,
                0x00,
                0x02,
                0x00,
            ],
            constants: vec![],
            locals_count: 3,
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
        };

        let output = bytecode.disassemble();
        assert!(
            output.contains("NewObject r0"),
            "NewObject disasm wrong: {}",
            output
        );
        assert!(
            output.contains("Call r1, r2, argc=0"),
            "Call disasm wrong: {}",
            output
        );
        assert!(
            output.contains("SetField r0, r1, r2"),
            "SetField disasm wrong: {}",
            output
        );
    }
}
