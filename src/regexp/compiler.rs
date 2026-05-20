use super::ast::{Ast, CharClass, Quantifier};

use super::opcode::*;

pub struct Compiler {
    bytecode: Vec<u8>,

    capture_count: usize,

    ignore_case: bool,

    char_ranges: Vec<super::charclass::CharRange>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub bytecode: Vec<u8>,

    pub capture_count: usize,

    pub flags: u16,

    pub char_ranges: Vec<super::charclass::CharRange>,
}

impl Program {
    pub fn flags(&self) -> u16 {
        u16::from_le_bytes([self.bytecode[HEADER_FLAGS], self.bytecode[HEADER_FLAGS + 1]])
    }

    pub fn capture_count(&self) -> usize {
        self.bytecode[HEADER_CAPTURE_COUNT] as usize
    }

    pub fn code(&self) -> &[u8] {
        &self.bytecode[HEADER_LEN..]
    }
}

pub fn compile(ast: &Ast, flags: u16) -> Result<Program, String> {
    let mut compiler = Compiler::new(flags);

    compiler.write_header_placeholder();

    compiler.emit_op_u8(OpCode::SaveStart, 0);

    compiler.compile_node(ast)?;

    compiler.emit_op_u8(OpCode::SaveEnd, 0);

    compiler.emit_op(OpCode::Success);

    compiler.update_header(flags)?;

    Ok(Program {
        bytecode: compiler.bytecode,
        capture_count: compiler.capture_count,
        flags,
        char_ranges: compiler.char_ranges,
    })
}

impl Compiler {
    fn new(flags: u16) -> Self {
        Self {
            bytecode: Vec::new(),
            capture_count: 1,
            ignore_case: (flags & FLAG_IGNORE_CASE) != 0,
            char_ranges: Vec::new(),
        }
    }

    fn write_header_placeholder(&mut self) {
        self.bytecode.extend_from_slice(&[0, 0]);

        self.bytecode.push(0);

        self.bytecode.push(REG_COUNT as u8);

        self.bytecode.extend_from_slice(&[0, 0, 0, 0]);
    }

    fn update_header(&mut self, flags: u16) -> Result<(), String> {
        let flag_bytes = flags.to_le_bytes();
        self.bytecode[HEADER_FLAGS] = flag_bytes[0];
        self.bytecode[HEADER_FLAGS + 1] = flag_bytes[1];

        if self.capture_count > MAX_CAPTURES {
            return Err(format!("Too many capture groups: {}", self.capture_count));
        }
        self.bytecode[HEADER_CAPTURE_COUNT] = self.capture_count as u8;

        let len = self.bytecode.len() - HEADER_LEN;
        let len_bytes = (len as u32).to_le_bytes();
        self.bytecode[HEADER_CODE_LEN..HEADER_CODE_LEN + 4].copy_from_slice(&len_bytes);

        Ok(())
    }

    fn compile_node(&mut self, node: &Ast) -> Result<(), String> {
        match node {
            Ast::Empty => Ok(()),
            Ast::Char(c) => self.compile_char(*c),
            Ast::Class(class) => self.compile_class(class),
            Ast::Any => {
                self.emit_op(OpCode::MatchDot);
                Ok(())
            }
            Ast::AnyAll => {
                self.emit_op(OpCode::MatchAny);
                Ok(())
            }
            Ast::StartOfLine => {
                self.emit_op(OpCode::CheckLineStart);
                Ok(())
            }
            Ast::EndOfLine => {
                self.emit_op(OpCode::CheckLineEnd);
                Ok(())
            }
            Ast::WordBoundary => {
                if self.ignore_case {
                    self.emit_op(OpCode::CheckWordBoundaryI);
                } else {
                    self.emit_op(OpCode::CheckWordBoundary);
                }
                Ok(())
            }
            Ast::NotWordBoundary => {
                if self.ignore_case {
                    self.emit_op(OpCode::CheckNotWordBoundaryI);
                } else {
                    self.emit_op(OpCode::CheckNotWordBoundary);
                }
                Ok(())
            }
            Ast::Concat(nodes) => {
                for node in nodes {
                    self.compile_node(node)?;
                }
                Ok(())
            }
            Ast::Alt(nodes) => self.compile_alt(nodes),
            Ast::Quant(inner, q) => self.compile_quant(inner, q),
            Ast::Capture(inner, _name) => self.compile_capture(inner),
            Ast::BackRef(idx) => self.compile_backref(*idx),
            Ast::NamedBackRef(name) => Err(format!("Named backref not yet implemented: {}", name)),
            Ast::Lookahead(inner) => self.compile_lookahead(inner, false),
            Ast::NegativeLookahead(inner) => self.compile_lookahead(inner, true),
        }
    }

    fn compile_char(&mut self, c: char) -> Result<(), String> {
        let cp = c as u32;

        if self.ignore_case {
            let folded = unicode_fold_simple(cp);
            if folded > 0xFFFF {
                self.emit_match_char32_i(REG_POS, folded);
            } else {
                self.emit_match_char_i(REG_POS, folded as u16);
            }
        } else {
            if cp > 0xFFFF {
                self.emit_match_char32(REG_POS, cp);
            } else {
                self.emit_match_char(REG_POS, cp as u16);
            }
        }
        Ok(())
    }

    fn compile_class(&mut self, class: &CharClass) -> Result<(), String> {
        let range_idx = self.char_ranges.len();
        self.char_ranges.push(class.ranges.clone());

        if range_idx > u16::MAX as usize {
            return Err("Too many character classes".to_string());
        }

        let opcode = if self.ignore_case {
            OpCode::MatchClassI
        } else {
            OpCode::MatchClass
        };

        self.bytecode.push(opcode as u8);
        self.bytecode.push(REG_POS as u8);
        self.bytecode
            .extend_from_slice(&(range_idx as u16).to_le_bytes());

        Ok(())
    }

    fn compile_alt(&mut self, nodes: &[Ast]) -> Result<(), String> {
        if nodes.is_empty() {
            return Ok(());
        }
        if nodes.len() == 1 {
            return self.compile_node(&nodes[0]);
        }

        let mut jump_offsets = Vec::new();

        for (i, node) in nodes.iter().enumerate() {
            if i < nodes.len() - 1 {
                let push_pos = self.bytecode.len();
                self.bytecode.push(OpCode::PushBacktrack as u8);
                self.bytecode.extend_from_slice(&0i32.to_le_bytes());

                self.compile_node(node)?;

                let jmp_pos = self.bytecode.len();
                self.bytecode.push(OpCode::Jmp as u8);
                self.bytecode.extend_from_slice(&0i32.to_le_bytes());
                jump_offsets.push(jmp_pos);

                let fail_target = self.bytecode.len();
                let push_offset = (fail_target as i32 - push_pos as i32 - 5) as i32;
                self.bytecode[push_pos + 1..push_pos + 5]
                    .copy_from_slice(&push_offset.to_le_bytes());
            } else {
                self.compile_node(node)?;
            }
        }

        let end_pos = self.bytecode.len();
        for jmp_pos in jump_offsets {
            let offset = (end_pos as i32 - jmp_pos as i32 - 5) as i32;
            self.bytecode[jmp_pos + 1..jmp_pos + 5].copy_from_slice(&offset.to_le_bytes());
        }

        Ok(())
    }

    fn compile_quant(&mut self, inner: &Ast, q: &Quantifier) -> Result<(), String> {
        let min = q.min;
        let max = q.max.unwrap_or(usize::MAX as u32) as usize;
        let greedy = q.greedy;

        if min == 0 && max == 0 {
            return Ok(());
        }

        if min == 1 && max == 1 {
            return self.compile_node(inner);
        }

        if min == 0 && max == 1 {
            return self.compile_optional(inner, greedy);
        }

        if min == 0 && max == usize::MAX {
            return self.compile_star(inner, greedy);
        }

        if min == 1 && max == usize::MAX {
            return self.compile_plus(inner, greedy);
        }

        self.compile_repeat(inner, min as usize, max, greedy)
    }

    fn compile_optional(&mut self, inner: &Ast, greedy: bool) -> Result<(), String> {
        if greedy {
            let push_pos = self.bytecode.len();
            self.bytecode.push(OpCode::PushBacktrack as u8);
            self.bytecode.extend_from_slice(&0i32.to_le_bytes());

            self.compile_node(inner)?;

            let jmp_pos = self.bytecode.len();
            self.bytecode.push(OpCode::Jmp as u8);
            self.bytecode.extend_from_slice(&0i32.to_le_bytes());

            let skip_target = self.bytecode.len();
            let push_offset = (skip_target as i32 - push_pos as i32 - 5) as i32;
            self.bytecode[push_pos + 1..push_pos + 5].copy_from_slice(&push_offset.to_le_bytes());

            let done_target = self.bytecode.len();
            let jmp_offset = (done_target as i32 - jmp_pos as i32 - 5) as i32;
            self.bytecode[jmp_pos + 1..jmp_pos + 5].copy_from_slice(&jmp_offset.to_le_bytes());
        } else {
            let push_pos = self.bytecode.len();
            self.bytecode.push(OpCode::PushBacktrack as u8);
            self.bytecode.extend_from_slice(&0i32.to_le_bytes());

            let jmp_pos = self.bytecode.len();
            self.bytecode.push(OpCode::Jmp as u8);
            self.bytecode.extend_from_slice(&0i32.to_le_bytes());

            let match_target = self.bytecode.len();
            let push_offset = (match_target as i32 - push_pos as i32 - 5) as i32;
            self.bytecode[push_pos + 1..push_pos + 5].copy_from_slice(&push_offset.to_le_bytes());

            self.compile_node(inner)?;

            let done_target = self.bytecode.len();
            let jmp_offset = (done_target as i32 - jmp_pos as i32 - 5) as i32;
            self.bytecode[jmp_pos + 1..jmp_pos + 5].copy_from_slice(&jmp_offset.to_le_bytes());
        }

        Ok(())
    }

    fn compile_star(&mut self, inner: &Ast, _greedy: bool) -> Result<(), String> {
        let start_pos = self.bytecode.len();

        let push_pos = self.bytecode.len();
        self.bytecode.push(OpCode::PushBacktrack as u8);
        self.bytecode.extend_from_slice(&0i32.to_le_bytes());

        self.compile_node(inner)?;

        self.bytecode.push(OpCode::Jmp as u8);
        let loop_offset = (start_pos as i32 - self.bytecode.len() as i32 - 5) as i32;
        self.bytecode.extend_from_slice(&loop_offset.to_le_bytes());

        let done_pos = self.bytecode.len();
        let push_offset = (done_pos as i32 - push_pos as i32 - 5) as i32;
        self.bytecode[push_pos + 1..push_pos + 5].copy_from_slice(&push_offset.to_le_bytes());

        Ok(())
    }

    fn compile_plus(&mut self, inner: &Ast, greedy: bool) -> Result<(), String> {
        self.compile_node(inner)?;

        self.compile_star(inner, greedy)
    }

    fn compile_repeat(
        &mut self,
        inner: &Ast,
        min: usize,
        max: usize,
        _greedy: bool,
    ) -> Result<(), String> {
        let counter_reg = REG_COUNTER;

        self.emit_mov_imm(counter_reg, 0);

        let min_start = self.bytecode.len();

        self.bytecode.push(OpCode::CmpImm as u8);
        self.bytecode.push(counter_reg as u8);
        self.bytecode.extend_from_slice(&(min as u32).to_le_bytes());

        let cmp_pos = self.bytecode.len();
        self.bytecode.push(OpCode::JmpNe as u8);
        self.bytecode.push(counter_reg as u8);
        self.bytecode.extend_from_slice(&(min as u32).to_le_bytes());
        self.bytecode.extend_from_slice(&0i32.to_le_bytes());

        self.compile_node(inner)?;

        self.bytecode.push(OpCode::Inc as u8);
        self.bytecode.push(counter_reg as u8);

        self.bytecode.push(OpCode::Jmp as u8);
        let loop_offset = (min_start as i32 - self.bytecode.len() as i32 - 5) as i32;
        self.bytecode.extend_from_slice(&loop_offset.to_le_bytes());

        let opt_start = self.bytecode.len();
        let jmp_offset = (opt_start as i32 - cmp_pos as i32 - 10) as i32;
        self.bytecode[cmp_pos + 6..cmp_pos + 10].copy_from_slice(&jmp_offset.to_le_bytes());

        if max > min && max < usize::MAX {
            self.emit_mov_imm(counter_reg, 0);

            let opt_loop_start = self.bytecode.len();

            self.bytecode.push(OpCode::CmpImm as u8);
            self.bytecode.push(counter_reg as u8);
            self.bytecode
                .extend_from_slice(&((max - min) as u32).to_le_bytes());

            let cmp2_pos = self.bytecode.len();
            self.bytecode.push(OpCode::JmpNe as u8);
            self.bytecode.push(counter_reg as u8);
            self.bytecode
                .extend_from_slice(&((max - min) as u32).to_le_bytes());
            self.bytecode.extend_from_slice(&0i32.to_le_bytes());

            let push_pos = self.bytecode.len();
            self.bytecode.push(OpCode::PushBacktrack as u8);
            self.bytecode.extend_from_slice(&0i32.to_le_bytes());

            self.compile_node(inner)?;

            self.bytecode.push(OpCode::Inc as u8);
            self.bytecode.push(counter_reg as u8);

            self.bytecode.push(OpCode::Jmp as u8);
            let loop2_offset = (opt_loop_start as i32 - self.bytecode.len() as i32 - 5) as i32;
            self.bytecode.extend_from_slice(&loop2_offset.to_le_bytes());

            let end_pos = self.bytecode.len();
            let push2_offset = (end_pos as i32 - push_pos as i32 - 5) as i32;
            self.bytecode[push_pos + 1..push_pos + 5].copy_from_slice(&push2_offset.to_le_bytes());

            let jmp2_offset = (end_pos as i32 - cmp2_pos as i32 - 10) as i32;
            self.bytecode[cmp2_pos + 6..cmp2_pos + 10].copy_from_slice(&jmp2_offset.to_le_bytes());
        }

        Ok(())
    }

    fn compile_capture(&mut self, inner: &Ast) -> Result<(), String> {
        let capture_idx = self.capture_count;
        self.capture_count += 1;

        self.emit_op_u8(OpCode::SaveStart, capture_idx as u8);
        self.compile_node(inner)?;
        self.emit_op_u8(OpCode::SaveEnd, capture_idx as u8);

        Ok(())
    }

    fn compile_backref(&mut self, idx: usize) -> Result<(), String> {
        if idx >= MAX_CAPTURES {
            return Err(format!("Backreference index too large: {}", idx));
        }

        let opcode = if self.ignore_case {
            OpCode::CheckBackrefI
        } else {
            OpCode::CheckBackref
        };

        self.emit_op_u8(opcode, idx as u8);
        Ok(())
    }

    fn compile_lookahead(&mut self, inner: &Ast, negative: bool) -> Result<(), String> {
        self.bytecode.push(OpCode::Mark as u8);
        self.bytecode.push(REG_MARK as u8);

        let push_pos = self.bytecode.len();
        self.bytecode.push(OpCode::PushBacktrack as u8);
        self.bytecode.extend_from_slice(&0i32.to_le_bytes());

        self.compile_node(inner)?;

        self.bytecode.push(OpCode::PopBacktrack as u8);

        if negative {
            self.bytecode.push(OpCode::Restore as u8);
            self.bytecode.push(REG_MARK as u8);
            self.bytecode.push(OpCode::Fail as u8);
        }

        self.bytecode.push(OpCode::Restore as u8);
        self.bytecode.push(REG_MARK as u8);

        let end_pos = self.bytecode.len();
        let offset = (end_pos as i32 - push_pos as i32 - 5) as i32;
        self.bytecode[push_pos + 1..push_pos + 5].copy_from_slice(&offset.to_le_bytes());

        if !negative {
            self.bytecode.push(OpCode::Fail as u8);
        }

        Ok(())
    }

    fn emit_op(&mut self, op: OpCode) {
        self.bytecode.push(op as u8);
    }

    fn emit_op_u8(&mut self, op: OpCode, val: u8) {
        self.bytecode.push(op as u8);
        self.bytecode.push(val);
    }

    fn emit_match_char(&mut self, reg: usize, ch: u16) {
        self.bytecode.push(OpCode::MatchChar as u8);
        self.bytecode.push(reg as u8);
        self.bytecode.extend_from_slice(&ch.to_le_bytes());
    }

    fn emit_match_char_i(&mut self, reg: usize, ch: u16) {
        self.bytecode.push(OpCode::MatchCharI as u8);
        self.bytecode.push(reg as u8);
        self.bytecode.extend_from_slice(&ch.to_le_bytes());
    }

    fn emit_match_char32(&mut self, reg: usize, ch: u32) {
        self.bytecode.push(OpCode::MatchChar32 as u8);
        self.bytecode.push(reg as u8);
        self.bytecode.extend_from_slice(&ch.to_le_bytes());
    }

    fn emit_match_char32_i(&mut self, reg: usize, ch: u32) {
        self.bytecode.push(OpCode::MatchChar32I as u8);
        self.bytecode.push(reg as u8);
        self.bytecode.extend_from_slice(&ch.to_le_bytes());
    }

    fn emit_mov_imm(&mut self, reg: usize, imm: usize) {
        self.bytecode.push(OpCode::MovImm as u8);
        self.bytecode.push(reg as u8);
        self.bytecode.extend_from_slice(&(imm as u32).to_le_bytes());
    }
}

fn unicode_fold_simple(c: u32) -> u32 {
    if c < 128 {
        if c >= b'A' as u32 && c <= b'Z' as u32 {
            c + 32
        } else {
            c
        }
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::super::parser::parse;
    use super::*;

    #[test]
    fn test_compile_simple() {
        let ast = parse("abc", 0).unwrap();
        let prog = compile(&ast, 0).unwrap();
        assert!(prog.bytecode.len() > HEADER_LEN);
    }

    #[test]
    fn test_compile_capture() {
        let ast = parse("(a)", 0).unwrap();
        let prog = compile(&ast, 0).unwrap();
        assert_eq!(prog.capture_count, 2);
    }

    #[test]
    fn test_compile_alt() {
        let ast = parse("a|b", 0).unwrap();
        let prog = compile(&ast, 0).unwrap();
        assert!(prog.bytecode.len() > HEADER_LEN);
    }

    #[test]
    fn test_compile_quant() {
        let ast = parse("a*", 0).unwrap();
        let prog = compile(&ast, 0).unwrap();
        assert!(prog.bytecode.len() > HEADER_LEN);
    }
}
