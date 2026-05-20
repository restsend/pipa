use super::charclass::{is_line_terminator, is_word_char};
use super::opcode::*;
use super::pool::BacktrackPool;

#[inline(always)]
fn likely(b: bool) -> bool {
    b
}

#[inline(always)]
fn unlikely(b: bool) -> bool {
    b
}

#[derive(Debug, Clone)]
pub struct Match {
    pub start: usize,

    pub end: usize,

    pub captures: Vec<(Option<usize>, Option<usize>)>,
}

impl Match {
    pub fn as_str<'a>(&self, input: &'a str) -> &'a str {
        &input[self.start..self.end]
    }
}

#[derive(Debug, Clone, Copy)]
struct RegisterFile {
    r: [usize; REG_COUNT],
}

impl Default for RegisterFile {
    fn default() -> Self {
        Self { r: [0; REG_COUNT] }
    }
}

pub struct ExecContext<'a> {
    input: &'a str,

    input_bytes: &'a [u8],

    char_positions: Vec<usize>,

    char_len: usize,

    is_ascii: bool,

    bytecode: &'a [u8],

    capture_count: usize,

    backtrack_pool: BacktrackPool,

    is_unicode: bool,

    char_ranges: &'a [super::charclass::CharRange],

    multi_line: bool,

    sticky: bool,
}

pub fn execute(prog: &super::compiler::Program, input: &str, start_pos: usize) -> Option<Match> {
    let ctx = ExecContext::new(prog, input);
    ctx.execute(start_pos)
}

pub fn find_all(prog: &super::compiler::Program, input: &str) -> Vec<Match> {
    let ctx = ExecContext::new(prog, input);
    ctx.find_all()
}

impl<'a> ExecContext<'a> {
    fn new(prog: &'a super::compiler::Program, input: &'a str) -> Self {
        let input_bytes = input.as_bytes();
        let flags = prog.flags;

        let is_ascii = input_bytes.iter().all(|&b| b < 0x80);

        let (char_positions, char_len) = if is_ascii {
            (Vec::new(), input_bytes.len())
        } else {
            let positions: Vec<usize> = input.char_indices().map(|(i, _)| i).collect();
            let len = positions.len();
            (positions, len)
        };

        Self {
            input,
            input_bytes,
            char_positions,
            char_len,
            is_ascii,
            bytecode: &prog.bytecode[HEADER_LEN..],
            capture_count: prog.capture_count,
            backtrack_pool: BacktrackPool::new(),
            is_unicode: (flags & FLAG_UNICODE) != 0 || (flags & FLAG_UNICODE_SETS) != 0,
            multi_line: (flags & FLAG_MULTI_LINE) != 0,
            sticky: (flags & FLAG_STICKY) != 0,
            char_ranges: &prog.char_ranges,
        }
    }

    #[inline(always)]
    fn char_to_byte_pos(&self, char_pos: usize) -> usize {
        if self.is_ascii {
            char_pos.min(self.input_bytes.len())
        } else {
            self.char_positions
                .get(char_pos)
                .copied()
                .unwrap_or(self.input_bytes.len())
        }
    }

    fn execute(mut self, start_pos: usize) -> Option<Match> {
        let is_sticky = self.sticky;

        let char_len = self.char_len;
        let mut pos = start_pos;

        let starts_with_anchor = self.pattern_starts_with_start_anchor();

        if starts_with_anchor && !is_sticky && start_pos == 0 {
            let mut regs = RegisterFile::default();
            regs.r[REG_POS] = 0;
            let mut captures = vec![(None, None); self.capture_count];

            if let Some(end_pos) = self.run(&mut regs, 0, &mut captures) {
                return Some(Match {
                    start: 0,
                    end: end_pos,
                    captures,
                });
            }
            return None;
        }

        let ends_with_anchor = self.pattern_ends_with_end_anchor();

        if ends_with_anchor && !is_sticky && char_len > 0 {
            return self.execute_end_anchor(start_pos);
        }

        let mut captures: Vec<(Option<usize>, Option<usize>)> =
            vec![(None, None); self.capture_count];

        while pos < char_len {
            let mut regs = RegisterFile::default();
            regs.r[REG_POS] = pos;

            for c in captures.iter_mut() {
                *c = (None, None);
            }

            if let Some(end_pos) = self.run(&mut regs, 0, &mut captures) {
                return Some(Match {
                    start: pos,
                    end: end_pos,
                    captures,
                });
            }

            if is_sticky {
                break;
            }
            pos += 1;
        }

        None
    }

    fn pattern_starts_with_start_anchor(&self) -> bool {
        let code = self.bytecode;
        if code.is_empty() {
            return false;
        }

        let mut i = 0;
        while i < code.len() && i < 20 {
            let op = code[i];
            if op == OpCode::CheckLineStart as u8 {
                return true;
            }

            match op {
                27 | 28 => i += 2,
                1 | 2 => i += 4,
                9..=14 => i += 1,
                _ => {
                    break;
                }
            }
        }
        false
    }

    fn pattern_ends_with_end_anchor(&self) -> bool {
        self.check_bytecode_has_end_anchor()
    }

    fn check_bytecode_has_end_anchor(&self) -> bool {
        let code = self.bytecode;
        let len = code.len();

        if len < 2 {
            return false;
        }

        for &op in code.iter().rev().take(50) {
            if op == OpCode::CheckLineEnd as u8 {
                return true;
            }
        }

        false
    }

    fn execute_end_anchor(mut self, start_pos: usize) -> Option<Match> {
        let char_len = self.char_len;

        if self.multi_line {
            let end_positions = self.find_all_line_ends(start_pos);
            for end_pos in end_positions {
                if let Some(m) = self.try_match_ending_at(end_pos, start_pos) {
                    return Some(m);
                }
            }
        } else {
            if let Some(m) = self.try_match_ending_at(char_len, start_pos) {
                return Some(m);
            }
        }

        None
    }

    fn try_match_ending_at(&mut self, end_pos: usize, start_pos: usize) -> Option<Match> {
        let max_lookback = 100;
        let search_start = start_pos.max(end_pos.saturating_sub(max_lookback));

        for pos in search_start..=end_pos {
            let mut regs = RegisterFile::default();
            regs.r[REG_POS] = pos;
            let mut captures = vec![(None, None); self.capture_count];

            if let Some(match_end) = self.run(&mut regs, 0, &mut captures) {
                if match_end == end_pos {
                    return Some(Match {
                        start: pos,
                        end: match_end,
                        captures,
                    });
                }
            }
        }

        None
    }

    fn find_all_line_ends(&self, start_pos: usize) -> Vec<usize> {
        let mut ends = vec![self.char_len];

        if !self.multi_line {
            return ends;
        }

        for (i, c) in self.input.char_indices() {
            if is_line_terminator(c as u32) {
                let char_pos = self.byte_to_char_pos(i);
                if char_pos >= start_pos && char_pos < self.char_len && !ends.contains(&char_pos) {
                    ends.push(char_pos);
                }
            }
        }

        ends.sort_unstable();
        ends
    }

    fn byte_to_char_pos(&self, byte_pos: usize) -> usize {
        if self.is_ascii {
            byte_pos
        } else {
            match self.char_positions.binary_search(&byte_pos) {
                Ok(i) => i,
                Err(i) => i.saturating_sub(1),
            }
        }
    }

    fn find_all(mut self) -> Vec<Match> {
        let mut matches = Vec::new();
        let mut pos = 0;

        let char_len = self.char_len;

        let mut captures: Vec<(Option<usize>, Option<usize>)> =
            vec![(None, None); self.capture_count];

        while pos < char_len {
            let mut regs = RegisterFile::default();
            regs.r[REG_POS] = pos;

            self.backtrack_pool.clear();

            for c in captures.iter_mut() {
                *c = (None, None);
            }

            if let Some(end_pos) = self.run(&mut regs, 0, &mut captures) {
                let match_start = pos;
                let match_end = end_pos;

                matches.push(Match {
                    start: match_start,
                    end: match_end,
                    captures: captures.clone(),
                });

                if match_end <= match_start {
                    pos += 1;
                } else {
                    pos = match_end;
                }
            } else {
                pos += 1;
            }
        }

        matches
    }

    fn run(
        &mut self,
        regs: &mut RegisterFile,
        mut pc: usize,
        captures: &mut [(Option<usize>, Option<usize>)],
    ) -> Option<usize> {
        loop {
            if unlikely(pc >= self.bytecode.len()) {
                return self.fail_or_backtrack(regs, pc, captures);
            }

            let opcode_byte = self.bytecode[pc];

            let opcode = if likely(opcode_byte <= OpCode::Halt as u8) {
                unsafe { std::mem::transmute(opcode_byte) }
            } else {
                return None;
            };

            match opcode {
                OpCode::Success => {
                    return Some(regs.r[REG_POS]);
                }

                OpCode::Fail => {
                    return self.fail_or_backtrack(regs, pc, captures);
                }

                OpCode::Halt => {
                    return None;
                }

                OpCode::MatchChar => {
                    let expected =
                        ((self.bytecode[pc + 3] as u16) << 8 | self.bytecode[pc + 2] as u16) as u32;
                    let pos = regs.r[REG_POS];

                    if likely(pos < self.char_len) {
                        let byte_pos = if self.is_ascii {
                            pos
                        } else {
                            self.char_positions[pos]
                        };
                        let b = self.input_bytes[byte_pos];
                        if likely(b < 0x80 && b as u32 == expected) {
                            regs.r[REG_POS] = pos + 1;
                            pc += 4;
                            continue;
                        }

                        if let Some(c) = self.get_char_fast(pos) {
                            if c as u32 == expected {
                                regs.r[REG_POS] = pos + 1;
                                pc += 4;
                                continue;
                            }
                        }
                    }
                    return self.fail_or_backtrack(regs, pc, captures);
                }

                OpCode::MatchCharI => {
                    let expected = self.read_u16(pc + 2) as u32;
                    let pos = regs.r[REG_POS];

                    if likely(pos < self.char_len) {
                        if let Some(c) = self.get_char_fast(pos) {
                            if canonicalize(c as u32, self.is_unicode) == expected {
                                regs.r[REG_POS] = pos + 1;
                                pc += opcode.size();
                                continue;
                            }
                        }
                    }
                    return self.fail_or_backtrack(regs, pc, captures);
                }

                OpCode::MatchChar32 => {
                    let reg = self.bytecode[pc + 1] as usize;
                    let expected = self.read_u32(pc + 2);
                    let pos = regs.r[reg];

                    if let Some(c) = self.get_char(pos) {
                        if c as u32 == expected {
                            regs.r[reg] = pos + 1;
                            pc += opcode.size();
                            continue;
                        }
                    }
                    return self.fail_or_backtrack(regs, pc, captures);
                }

                OpCode::MatchChar32I => {
                    let reg = self.bytecode[pc + 1] as usize;
                    let expected = self.read_u32(pc + 2);
                    let pos = regs.r[reg];

                    if let Some(c) = self.get_char(pos) {
                        if canonicalize(c as u32, self.is_unicode) == expected {
                            regs.r[reg] = pos + 1;
                            pc += opcode.size();
                            continue;
                        }
                    }
                    return self.fail_or_backtrack(regs, pc, captures);
                }

                OpCode::MatchDot => {
                    let pos = regs.r[REG_POS];
                    if let Some(c) = self.get_char(pos) {
                        if !is_line_terminator(c as u32) {
                            regs.r[REG_POS] = pos + 1;
                            pc += 1;
                            continue;
                        }
                    }
                    return self.fail_or_backtrack(regs, pc, captures);
                }

                OpCode::MatchAny => {
                    let pos = regs.r[REG_POS];
                    if self.get_char(pos).is_some() {
                        regs.r[REG_POS] = pos + 1;
                        pc += 1;
                        continue;
                    }
                    return self.fail_or_backtrack(regs, pc, captures);
                }

                OpCode::MatchClass => {
                    let range_idx = self.read_u16(pc + 2) as usize;
                    let pos = regs.r[REG_POS];
                    if let Some(c) = self.get_char(pos) {
                        if let Some(range) = self.char_ranges.get(range_idx) {
                            if range.contains(c as u32) {
                                regs.r[REG_POS] = pos + 1;
                                pc += 4;
                                continue;
                            }
                        }
                    }
                    return self.fail_or_backtrack(regs, pc, captures);
                }

                OpCode::MatchClassI => {
                    let range_idx = self.read_u16(pc + 2) as usize;
                    let pos = regs.r[REG_POS];
                    if let Some(c) = self.get_char(pos) {
                        let c_upper = canonicalize(c as u32, self.is_unicode);
                        if let Some(range) = self.char_ranges.get(range_idx) {
                            if range.contains(c as u32) || range.contains(c_upper) {
                                regs.r[REG_POS] = pos + 1;
                                pc += 4;
                                continue;
                            }
                        }
                    }
                    return self.fail_or_backtrack(regs, pc, captures);
                }

                OpCode::CheckLineStart => {
                    let pos = regs.r[REG_POS];
                    let at_start = pos == 0;
                    let after_newline = if self.multi_line {
                        pos > 0 && is_line_terminator(self.get_char(pos - 1).unwrap_or('\0') as u32)
                    } else {
                        false
                    };

                    if at_start || after_newline {
                        pc += 1;
                        continue;
                    }
                    return self.fail_or_backtrack(regs, pc, captures);
                }

                OpCode::CheckLineEnd => {
                    let pos = regs.r[REG_POS];
                    let at_end = pos >= self.char_len;
                    let before_newline = if self.multi_line {
                        self.get_char(pos)
                            .map_or(false, |c| is_line_terminator(c as u32))
                    } else {
                        false
                    };

                    if at_end || before_newline {
                        pc += 1;
                        continue;
                    }
                    return self.fail_or_backtrack(regs, pc, captures);
                }

                OpCode::CheckWordBoundary | OpCode::CheckWordBoundaryI => {
                    let ignore_case = opcode == OpCode::CheckWordBoundaryI;
                    if self.check_word_boundary(regs.r[REG_POS], ignore_case) {
                        pc += 1;
                        continue;
                    }
                    return self.fail_or_backtrack(regs, pc, captures);
                }

                OpCode::CheckNotWordBoundary | OpCode::CheckNotWordBoundaryI => {
                    let ignore_case = opcode == OpCode::CheckNotWordBoundaryI;
                    if !self.check_word_boundary(regs.r[REG_POS], ignore_case) {
                        pc += 1;
                        continue;
                    }
                    return self.fail_or_backtrack(regs, pc, captures);
                }

                OpCode::Jmp => {
                    let offset = i32::from_le_bytes([
                        self.bytecode[pc + 1],
                        self.bytecode[pc + 2],
                        self.bytecode[pc + 3],
                        self.bytecode[pc + 4],
                    ]);
                    pc = (pc as i32 + 5 + offset) as usize;
                    continue;
                }

                OpCode::JmpMatch => {
                    let offset = i32::from_le_bytes([
                        self.bytecode[pc + 1],
                        self.bytecode[pc + 2],
                        self.bytecode[pc + 3],
                        self.bytecode[pc + 4],
                    ]);
                    pc = (pc as i32 + 5 + offset) as usize;
                    continue;
                }

                OpCode::JmpFail => {
                    let offset = i32::from_le_bytes([
                        self.bytecode[pc + 1],
                        self.bytecode[pc + 2],
                        self.bytecode[pc + 3],
                        self.bytecode[pc + 4],
                    ]);
                    pc = (pc as i32 + 5 + offset) as usize;
                    continue;
                }

                OpCode::JmpEq => {
                    let reg = self.bytecode[pc + 1] as usize;
                    let imm = self.read_u32(pc + 2) as usize;
                    let offset = self.read_i32(pc + 6);

                    if regs.r[reg] == imm {
                        pc = (pc as i32 + 10 + offset) as usize;
                    } else {
                        pc += 10;
                    }
                    continue;
                }

                OpCode::JmpNe => {
                    let reg = self.bytecode[pc + 1] as usize;
                    let imm = self.read_u32(pc + 2) as usize;
                    let offset = self.read_i32(pc + 6);

                    if regs.r[reg] != imm {
                        pc = (pc as i32 + 10 + offset) as usize;
                    } else {
                        pc += 10;
                    }
                    continue;
                }

                OpCode::JmpLt => {
                    let reg = self.bytecode[pc + 1] as usize;
                    let imm = self.read_u32(pc + 2) as usize;
                    let offset = self.read_i32(pc + 6);

                    if regs.r[reg] < imm {
                        pc = (pc as i32 + 10 + offset) as usize;
                    } else {
                        pc += 10;
                    }
                    continue;
                }

                OpCode::MovImm => {
                    let reg = self.bytecode[pc + 1] as usize;
                    let imm = self.read_u32(pc + 2);
                    regs.r[reg] = imm as usize;
                    pc += 6;
                    continue;
                }

                OpCode::MovReg => {
                    let dst = self.bytecode[pc + 1] as usize;
                    let src = self.bytecode[pc + 2] as usize;
                    regs.r[dst] = regs.r[src];
                    pc += 3;
                    continue;
                }

                OpCode::Inc => {
                    let reg = self.bytecode[pc + 1] as usize;
                    regs.r[reg] = regs.r[reg].wrapping_add(1);
                    pc += 2;
                    continue;
                }

                OpCode::Dec => {
                    let reg = self.bytecode[pc + 1] as usize;
                    regs.r[reg] = regs.r[reg].wrapping_sub(1);
                    pc += 2;
                    continue;
                }

                OpCode::AddImm => {
                    let reg = self.bytecode[pc + 1] as usize;
                    let imm = self.read_u32(pc + 2);
                    regs.r[reg] = regs.r[reg].wrapping_add(imm as usize);
                    pc += 6;
                    continue;
                }

                OpCode::SaveStart => {
                    let idx = self.bytecode[pc + 1] as usize;
                    if idx < captures.len() {
                        captures[idx].0 = Some(regs.r[REG_POS]);
                    }
                    pc += 2;
                    continue;
                }

                OpCode::SaveEnd => {
                    let idx = self.bytecode[pc + 1] as usize;
                    if idx < captures.len() {
                        captures[idx].1 = Some(regs.r[REG_POS]);
                    }
                    pc += 2;
                    continue;
                }

                OpCode::ResetCaptures => {
                    let start = self.bytecode[pc + 1] as usize;
                    let end = self.bytecode[pc + 2] as usize;
                    for i in start..=end {
                        if i < captures.len() {
                            captures[i] = (None, None);
                        }
                    }
                    pc += 3;
                    continue;
                }

                OpCode::PushBacktrack => {
                    let offset = self.read_i32(pc + 1);
                    let fail_target = (pc as i32 + 5 + offset) as usize;

                    self.backtrack_pool.push(super::pool::BacktrackState {
                        pc: fail_target as u32,
                        pos: regs.r[REG_POS] as u32,
                        counter: regs.r[REG_COUNTER] as u32,
                        capture_start: captures
                            .get(0)
                            .and_then(|c| c.0)
                            .unwrap_or(u32::MAX as usize)
                            as u32,
                        capture_end: captures
                            .get(0)
                            .and_then(|c| c.1)
                            .unwrap_or(u32::MAX as usize)
                            as u32,
                    });

                    pc += 5;
                    continue;
                }

                OpCode::PopBacktrack => {
                    self.backtrack_pool.pop();
                    pc += 1;
                    continue;
                }

                OpCode::InitCounter => {
                    let reg = self.bytecode[pc + 1] as usize;
                    let min = self.read_u32(pc + 2);
                    let max = self.read_u32(pc + 6);

                    regs.r[reg] = 0;

                    if reg + 1 < REG_COUNT {
                        regs.r[reg + 1] = min as usize;
                    }
                    if reg + 2 < REG_COUNT {
                        regs.r[reg + 2] = max as usize;
                    }

                    pc += 10;
                    continue;
                }

                OpCode::CheckCounter => {
                    let reg = self.bytecode[pc + 1] as usize;
                    let fail_offset = self.read_i32(pc + 2);

                    let count = regs.r[reg];
                    let max = if reg + 2 < REG_COUNT {
                        regs.r[reg + 2]
                    } else {
                        usize::MAX
                    };

                    if count >= max {
                        pc = (pc as i32 + 6 + fail_offset) as usize;
                    } else {
                        regs.r[reg] = count + 1;
                        pc += 6;
                    }
                    continue;
                }

                OpCode::Invalid => {
                    panic!("Invalid opcode at pc={}", pc);
                }

                _ => {
                    return self.fail_or_backtrack(regs, pc, captures);
                }
            }
        }
    }

    fn fail_or_backtrack(
        &mut self,
        regs: &mut RegisterFile,
        _pc: usize,
        captures: &mut [(Option<usize>, Option<usize>)],
    ) -> Option<usize> {
        while let Some(state) = self.backtrack_pool.pop() {
            regs.r[REG_POS] = state.pos as usize;
            regs.r[REG_COUNTER] = state.counter as usize;

            if state.capture_start != u32::MAX && state.capture_end != u32::MAX {
                if !captures.is_empty() {
                    captures[0] = (
                        Some(state.capture_start as usize),
                        Some(state.capture_end as usize),
                    );
                }
            }

            if let Some(result) = self.run(regs, state.pc as usize, captures) {
                return Some(result);
            }
        }
        None
    }

    #[inline(always)]
    fn get_char_fast(&self, pos: usize) -> Option<char> {
        if unlikely(pos >= self.char_len) {
            return None;
        }

        let byte_pos = self.char_to_byte_pos(pos);
        let b = self.input_bytes[byte_pos];

        if likely(b < 0x80) {
            Some(b as char)
        } else {
            self.get_char_utf8(pos)
        }
    }

    #[inline(never)]
    fn get_char_utf8(&self, pos: usize) -> Option<char> {
        if pos >= self.char_len {
            return None;
        }
        let byte_pos = self.char_to_byte_pos(pos);
        let bytes = &self.input_bytes[byte_pos..];
        let first = *bytes.first()?;

        let len = if first < 0xE0 {
            2
        } else if first < 0xF0 {
            3
        } else {
            4
        };

        if bytes.len() < len {
            return None;
        }

        std::str::from_utf8(&bytes[..len]).ok()?.chars().next()
    }

    fn get_char(&self, pos: usize) -> Option<char> {
        if pos >= self.char_len {
            return None;
        }
        self.get_char_fast(pos)
    }

    #[inline(always)]
    fn read_u16(&self, pos: usize) -> u16 {
        let bytes = &self.bytecode[pos..pos + 2];
        u16::from_le_bytes([bytes[0], bytes[1]])
    }

    #[inline(always)]
    fn read_u32(&self, pos: usize) -> u32 {
        let bytes = &self.bytecode[pos..pos + 4];
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    #[inline(always)]
    fn read_i32(&self, pos: usize) -> i32 {
        self.read_u32(pos) as i32
    }

    fn check_word_boundary(&self, pos: usize, ignore_case: bool) -> bool {
        let prev_is_word = if pos == 0 {
            false
        } else {
            self.get_char(pos - 1).map_or(false, |c| {
                let cp = if ignore_case {
                    canonicalize(c as u32, self.is_unicode)
                } else {
                    c as u32
                };
                is_word_char(cp)
            })
        };

        let next_is_word = self.get_char(pos).map_or(false, |c| {
            let cp = if ignore_case {
                canonicalize(c as u32, self.is_unicode)
            } else {
                c as u32
            };
            is_word_char(cp)
        });

        prev_is_word != next_is_word
    }
}

#[inline(always)]
fn canonicalize(c: u32, is_unicode: bool) -> u32 {
    if c < 128 {
        if is_unicode {
            if c >= b'A' as u32 && c <= b'Z' as u32 {
                c + 32
            } else {
                c
            }
        } else {
            if c >= b'a' as u32 && c <= b'z' as u32 {
                c - 32
            } else {
                c
            }
        }
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::super::compiler::compile;
    use super::super::parser::parse;
    use super::*;

    #[test]
    fn test_execute_simple() {
        let ast = parse("abc", 0).unwrap();
        let prog = compile(&ast, 0).unwrap();

        let m = execute(&prog, "abc", 0).unwrap();
        assert_eq!(m.start, 0);
        assert_eq!(m.end, 3);
    }

    #[test]
    fn test_execute_literal() {
        let ast = parse("hello", 0).unwrap();
        let prog = compile(&ast, 0).unwrap();

        let m = execute(&prog, "hello world", 0).unwrap();
        assert_eq!(m.start, 0);
        assert_eq!(m.end, 5);
    }

    #[test]
    fn test_no_match() {
        let ast = parse("xyz", 0).unwrap();
        let prog = compile(&ast, 0).unwrap();

        assert!(execute(&prog, "abc", 0).is_none());
    }

    #[test]
    fn test_ascii_fast_path() {
        let ast = parse("test", 0).unwrap();
        let prog = compile(&ast, 0).unwrap();

        let m = execute(&prog, "this is a test", 0).unwrap();
        assert_eq!(m.start, 10);
        assert_eq!(m.end, 14);
    }
}
