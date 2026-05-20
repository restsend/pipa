pub const REG_COUNT: usize = 8;

pub const REG_PC: usize = 0;
pub const REG_POS: usize = 0;
pub const REG_COUNTER: usize = 1;
pub const REG_TEMP: usize = 2;
pub const REG_SAVE0: usize = 3;
pub const REG_SAVE1: usize = 4;
pub const REG_SAVE2: usize = 5;
pub const REG_SAVE3: usize = 6;
pub const REG_MARK: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    Invalid = 0,

    MatchChar = 1,

    MatchCharI = 2,

    MatchChar32 = 3,

    MatchChar32I = 4,

    MatchDot = 5,

    MatchAny = 6,

    MatchClass = 7,
    MatchClassI = 8,

    CheckLineStart = 9,

    CheckLineEnd = 10,

    CheckWordBoundary = 11,
    CheckWordBoundaryI = 12,
    CheckNotWordBoundary = 13,
    CheckNotWordBoundaryI = 14,

    Jmp = 15,

    JmpMatch = 16,

    JmpFail = 17,

    JmpEq = 18,

    JmpNe = 19,

    JmpLt = 20,

    MovImm = 21,

    MovReg = 22,

    Inc = 23,

    Dec = 24,

    AddImm = 25,

    CmpImm = 26,

    SaveStart = 27,

    SaveEnd = 28,

    ResetCaptures = 29,

    CheckBackref = 30,
    CheckBackrefI = 31,

    Lookahead = 32,

    NegLookahead = 33,

    PushBacktrack = 34,

    PopBacktrack = 35,

    Mark = 36,

    Restore = 37,

    InitCounter = 38,

    CheckCounter = 39,

    Success = 40,

    Fail = 41,

    Halt = 42,
}

impl OpCode {
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= 42 {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }

    pub fn size(&self) -> usize {
        match self {
            OpCode::Invalid => 1,
            OpCode::MatchChar => 4,
            OpCode::MatchCharI => 4,
            OpCode::MatchChar32 => 6,
            OpCode::MatchChar32I => 6,
            OpCode::MatchDot => 1,
            OpCode::MatchAny => 1,
            OpCode::MatchClass => 4,
            OpCode::MatchClassI => 4,
            OpCode::CheckLineStart => 1,
            OpCode::CheckLineEnd => 1,
            OpCode::CheckWordBoundary => 1,
            OpCode::CheckWordBoundaryI => 1,
            OpCode::CheckNotWordBoundary => 1,
            OpCode::CheckNotWordBoundaryI => 1,
            OpCode::Jmp => 5,
            OpCode::JmpMatch => 5,
            OpCode::JmpFail => 5,
            OpCode::JmpEq => 10,
            OpCode::JmpNe => 10,
            OpCode::JmpLt => 10,
            OpCode::MovImm => 6,
            OpCode::MovReg => 3,
            OpCode::Inc => 2,
            OpCode::Dec => 2,
            OpCode::AddImm => 6,
            OpCode::CmpImm => 6,
            OpCode::SaveStart => 2,
            OpCode::SaveEnd => 2,
            OpCode::ResetCaptures => 3,
            OpCode::CheckBackref => 2,
            OpCode::CheckBackrefI => 2,
            OpCode::Lookahead => 5,
            OpCode::NegLookahead => 5,
            OpCode::PushBacktrack => 5,
            OpCode::PopBacktrack => 1,
            OpCode::Mark => 2,
            OpCode::Restore => 2,
            OpCode::InitCounter => 10,
            OpCode::CheckCounter => 6,
            OpCode::Success => 1,
            OpCode::Fail => 1,
            OpCode::Halt => 1,
        }
    }
}

pub const HEADER_FLAGS: usize = 0;
pub const HEADER_CAPTURE_COUNT: usize = 2;
pub const HEADER_REG_COUNT: usize = 3;
pub const HEADER_CODE_LEN: usize = 4;
pub const HEADER_LEN: usize = 8;

pub const FLAG_GLOBAL: u16 = 1 << 0;
pub const FLAG_IGNORE_CASE: u16 = 1 << 1;
pub const FLAG_MULTI_LINE: u16 = 1 << 2;
pub const FLAG_DOT_ALL: u16 = 1 << 3;
pub const FLAG_UNICODE: u16 = 1 << 4;
pub const FLAG_STICKY: u16 = 1 << 5;
pub const FLAG_INDICES: u16 = 1 << 6;
pub const FLAG_NAMED_GROUPS: u16 = 1 << 7;
pub const FLAG_UNICODE_SETS: u16 = 1 << 8;

pub const MAX_CAPTURES: usize = 255;
pub const MAX_BACKTRACK: usize = 10000;
