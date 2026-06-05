use crate::compiler::codegen::OptLevel;
use crate::compiler::opcode::{Bytecode, Opcode};

pub fn optimize(bytecode: &mut Bytecode) {
    optimize_with_level(bytecode, OptLevel::O2)
}

pub fn optimize_with_level(bytecode: &mut Bytecode, opt_level: OptLevel) {
    if matches!(opt_level, OptLevel::O0) {
        return;
    }
    let mut _total_changed = 0usize;
    for _ in 0..3 {
        let changed = run_pass(bytecode);
        if changed {
            _total_changed += 1;
        } else {
            break;
        }
    }
}

fn embedded_function_size(code: &[u8], start: usize) -> usize {
    let mut pc = start;
    if pc >= code.len() {
        return 1;
    }
    pc += 1; // opcode byte
    if pc + 2 > code.len() {
        return pc - start;
    }
    pc += 2; // param_count u16
    if pc + 1 > code.len() {
        return pc - start;
    }
    pc += 1; // uses_arguments u8
    if pc + 1 > code.len() {
        return pc - start;
    }
    pc += 1; // is_strict u8
    if pc + 2 > code.len() {
        return pc - start;
    }
    pc += 2; // locals_count u16
    if pc + 4 > code.len() {
        return pc - start;
    }
    let bytecode_len = read_i32(code, pc) as usize;
    pc += 4;
    pc += bytecode_len;
    if pc + 4 > code.len() {
        return pc - start;
    }
    let constants_len = read_i32(code, pc) as usize;
    pc += 4;
    for _ in 0..constants_len {
        if pc >= code.len() {
            return pc - start;
        }
        let tag = code[pc];
        pc += 1;
        match tag {
            4 => {
                pc += 8;
            }
            5 => {
                pc += 8;
            }
            6 => {
                pc += 4;
            }
            _ => {}
        }
    }
    if pc + 4 > code.len() {
        return pc - start;
    }
    let upvalue_count = read_i32(code, pc) as usize;
    pc += 4;
    pc += upvalue_count * 8;
    if pc + 4 > code.len() {
        return pc - start;
    }
    let line_entry_count = read_i32(code, pc) as usize;
    pc += 4;
    pc += line_entry_count * 8;
    if pc + 4 > code.len() {
        return pc - start;
    }
    pc += 4; // func_name_atom
    if pc + 4 > code.len() {
        return pc - start;
    }
    let var_count = read_i32(code, pc) as usize;
    pc += 4;
    pc += var_count * 6;
    if pc + 2 > code.len() {
        return pc - start;
    }
    pc += 2; // dst register
    pc - start
}

fn run_pass(bytecode: &mut Bytecode) -> bool {
    let mut changed = false;
    let mut pc = 0usize;
    let code = &mut bytecode.code;

    while pc < code.len() {
        let op = Opcode::from_u8_unchecked(code[pc]);
        let size = if op == Opcode::NewFunction
            || op == Opcode::NewAsyncFunction
            || op == Opcode::NewGeneratorFunction
            || op == Opcode::NewAsyncGeneratorFunction
        {
            embedded_function_size(code, pc)
        } else {
            Opcode::instruction_size(op)
        };
        if size == 0 || pc + size > code.len() {
            break;
        }

        if op == Opcode::Move {
            let dst = read_u16(code, pc + 1);
            let src = read_u16(code, pc + 3);
            if dst == src {
                code[pc] = Opcode::Nop as u8;
                code[pc + 1] = 0;
                code[pc + 2] = 0;
                code[pc + 3] = 0;
                code[pc + 4] = 0;
                changed = true;
            }
        }

        if op == Opcode::LoadInt {
            let dst = read_u16(code, pc + 1);
            let val = read_i32(code, pc + 3);
            if val == 0 {
                let next_pc = pc + size;
                if next_pc < code.len() {
                    let next_op = Opcode::from_u8_unchecked(code[next_pc]);
                    if next_op == Opcode::Sub || next_op == Opcode::SubNum {
                        let sub_dst = read_u16(code, next_pc + 1);
                        let sub_a = read_u16(code, next_pc + 3);
                        let sub_b = read_u16(code, next_pc + 5);
                        if sub_b == dst && sub_dst == sub_a {
                            code[next_pc] = Opcode::Move as u8;
                            write_i32(code, next_pc + 1, 0);
                            code[next_pc + 5] = 0;
                            code[next_pc + 6] = 0;
                            code[pc] = Opcode::Nop as u8;
                            for i in 1..size {
                                code[pc + i] = 0;
                            }
                            changed = true;
                        } else if sub_a == dst && sub_dst == sub_b {
                            code[next_pc] = Opcode::Move as u8;
                            code[next_pc + 1] = code[next_pc + 5];
                            code[next_pc + 2] = code[next_pc + 6];
                            code[next_pc + 3] = 0;
                            code[next_pc + 4] = 0;
                            code[next_pc + 5] = 0;
                            code[next_pc + 6] = 0;
                            code[pc] = Opcode::Nop as u8;
                            for i in 1..size {
                                code[pc + i] = 0;
                            }
                            changed = true;
                        }
                    } else if next_op == Opcode::Add || next_op == Opcode::AddNum {
                        let add_a = read_u16(code, next_pc + 3);
                        let add_b = read_u16(code, next_pc + 5);
                        if add_b == dst {
                            code[next_pc] = Opcode::Move as u8;
                            code[next_pc + 1] = code[next_pc + 1];
                            code[next_pc + 2] = code[next_pc + 2];
                            code[next_pc + 3] = code[next_pc + 3];
                            code[next_pc + 4] = code[next_pc + 4];
                            code[next_pc + 5] = 0;
                            code[next_pc + 6] = 0;

                            code[pc] = Opcode::Nop as u8;
                            for i in 1..size {
                                code[pc + i] = 0;
                            }
                            changed = true;
                        } else if add_a == dst {
                            code[next_pc] = Opcode::Move as u8;
                            code[next_pc + 1] = code[next_pc + 1];
                            code[next_pc + 2] = code[next_pc + 2];
                            code[next_pc + 3] = code[next_pc + 5];
                            code[next_pc + 4] = code[next_pc + 6];
                            code[next_pc + 5] = 0;
                            code[next_pc + 6] = 0;
                            code[pc] = Opcode::Nop as u8;
                            for i in 1..size {
                                code[pc + i] = 0;
                            }
                            changed = true;
                        }
                    }
                }
            }
        }

        if op == Opcode::LoadInt {
            let dst = read_u16(code, pc + 1);
            let val = read_i32(code, pc + 3);
            if val == 1 {
                let next_pc = pc + size;
                if next_pc < code.len() {
                    let next_op = Opcode::from_u8_unchecked(code[next_pc]);
                    if next_op == Opcode::Div || next_op == Opcode::DivNum {
                        let div_dst = read_u16(code, next_pc + 1);
                        let div_a = read_u16(code, next_pc + 3);
                        let div_b = read_u16(code, next_pc + 5);
                        if div_b == dst && div_dst == div_a {
                            code[next_pc] = Opcode::Move as u8;
                            write_i32(code, next_pc + 1, 0);
                            code[next_pc + 5] = 0;
                            code[next_pc + 6] = 0;
                            code[pc] = Opcode::Nop as u8;
                            for i in 1..size {
                                code[pc + i] = 0;
                            }
                            changed = true;
                        } else if div_a == dst && div_dst == div_b {
                            code[next_pc] = Opcode::Move as u8;
                            code[next_pc + 1] = code[next_pc + 5];
                            code[next_pc + 2] = code[next_pc + 6];
                            code[next_pc + 3] = 0;
                            code[next_pc + 4] = 0;
                            code[next_pc + 5] = 0;
                            code[next_pc + 6] = 0;
                            code[pc] = Opcode::Nop as u8;
                            for i in 1..size {
                                code[pc + i] = 0;
                            }
                            changed = true;
                        }
                    } else if next_op == Opcode::Mul || next_op == Opcode::MulNum {
                        let mul_a = read_u16(code, next_pc + 3);
                        let mul_b = read_u16(code, next_pc + 5);
                        if mul_b == dst {
                            code[next_pc] = Opcode::Move as u8;
                            code[next_pc + 3] = code[next_pc + 3];
                            code[next_pc + 4] = code[next_pc + 4];
                            code[next_pc + 5] = 0;
                            code[next_pc + 6] = 0;
                            code[pc] = Opcode::Nop as u8;
                            for i in 1..size {
                                code[pc + i] = 0;
                            }
                            changed = true;
                        } else if mul_a == dst {
                            code[next_pc] = Opcode::Move as u8;
                            code[next_pc + 3] = code[next_pc + 5];
                            code[next_pc + 4] = code[next_pc + 6];
                            code[next_pc + 5] = 0;
                            code[next_pc + 6] = 0;
                            code[pc] = Opcode::Nop as u8;
                            for i in 1..size {
                                code[pc + i] = 0;
                            }
                            changed = true;
                        }
                    }
                }
            }
        }

        if op == Opcode::LoadInt {
            let dst = read_u16(code, pc + 1);
            let val = read_i32(code, pc + 3);
            let next_pc = pc + size;
            if next_pc < code.len() {
                let next_op = Opcode::from_u8_unchecked(code[next_pc]);
                if next_op == Opcode::Move {
                    let move_src = read_u16(code, next_pc + 3);
                    if move_src == dst && (i8::MIN as i32..=i8::MAX as i32).contains(&val) {
                        code[next_pc] = Opcode::LoadInt8 as u8;
                        code[next_pc + 1] = code[next_pc + 1];
                        code[next_pc + 2] = code[next_pc + 2];
                        code[next_pc + 3] = val as i8 as u8;

                        code[next_pc + 4] = 0;
                        code[pc] = Opcode::Nop as u8;
                        for i in 1..size {
                            code[pc + i] = 0;
                        }
                        changed = true;
                    }
                }
            }
        }

        if op == Opcode::Jump {
            let offset = read_i32(code, pc + 1);
            if offset == size as i32 {
                code[pc] = Opcode::Nop as u8;
                for i in 1..size {
                    code[pc + i] = 0;
                }
                changed = true;
            } else {
                let target = (pc as i32 + size as i32 + offset) as usize;
                if target < code.len() {
                    let target_op = Opcode::from_u8_unchecked(code[target]);
                    if target_op == Opcode::Jump {
                        let target_offset = read_i32(code, target + 1);
                        let final_target = (target as i32
                            + Opcode::instruction_size(Opcode::Jump) as i32
                            + target_offset) as i32;
                        let new_offset = final_target - (pc as i32 + size as i32);
                        write_i32(code, pc + 1, new_offset);
                        changed = true;
                    }
                }
            }
        }

        if op == Opcode::Neg || op == Opcode::BitNot || op == Opcode::Not {
            let dst1 = read_u16(code, pc + 1);
            let next_pc = pc + size;
            if next_pc < code.len() {
                let next_op = Opcode::from_u8_unchecked(code[next_pc]);
                if next_op == op {
                    let _dst2 = read_u16(code, next_pc + 1);
                    let src2 = read_u16(code, next_pc + 3);
                    if src2 == dst1 {
                        code[next_pc] = Opcode::Move as u8;
                        code[next_pc + 3] = code[pc + 3];
                        code[next_pc + 4] = code[pc + 4];
                        code[pc] = Opcode::Nop as u8;
                        for i in 1..size {
                            code[pc + i] = 0;
                        }
                        changed = true;
                    }
                }
            }
        }

        if op == Opcode::LoadInt {
            let dst1 = read_u16(code, pc + 1);
            let a = read_i32(code, pc + 3);
            let next1 = pc + size;
            if next1 < code.len() && Opcode::from_u8_unchecked(code[next1]) == Opcode::LoadInt {
                let dst2 = read_u16(code, next1 + 1);
                let b = read_i32(code, next1 + 3);
                let next2 = next1 + size;
                if next2 < code.len() {
                    let op3 = Opcode::from_u8_unchecked(code[next2]);
                    let add_size = Opcode::instruction_size(op3);
                    let _dst3 = read_u16(code, next2 + 1);
                    let src1 = read_u16(code, next2 + 3);
                    let src2 = read_u16(code, next2 + 5);
                    if src1 == dst1 && src2 == dst2 {
                        let result = match op3 {
                            Opcode::Add => a.checked_add(b),
                            Opcode::Sub => a.checked_sub(b),
                            Opcode::Mul => a.checked_mul(b),
                            Opcode::BitAnd => Some(a & b),
                            Opcode::BitOr => Some(a | b),
                            Opcode::BitXor => Some(a ^ b),
                            _ => None,
                        };
                        if let Some(r) = result {
                            code[pc] = Opcode::Nop as u8;
                            for i in 1..size {
                                code[pc + i] = 0;
                            }
                            code[next1] = Opcode::LoadInt as u8;
                            code[next1 + 1] = code[next2 + 1];
                            code[next1 + 2] = code[next2 + 2];
                            write_i32(code, next1 + 3, r as i32);

                            if add_size > size {
                                for i in size..add_size {
                                    code[next1 + i] = 0;
                                }
                            }
                            changed = true;
                            pc = next2;
                            continue;
                        }
                    }
                }
            }
        }

        if op == Opcode::LoadTrue || op == Opcode::LoadFalse {
            let is_true = op == Opcode::LoadTrue;
            let dst = read_u16(code, pc + 1);
            let next_pc = pc + size;
            if next_pc < code.len() {
                let next_op = Opcode::from_u8_unchecked(code[next_pc]);
                if next_op == Opcode::JumpIf || next_op == Opcode::JumpIfNot {
                    let src = read_u16(code, next_pc + 1);
                    let offset = read_i32(code, next_pc + 3);
                    if src == dst {
                        let jump_taken = if next_op == Opcode::JumpIf {
                            is_true
                        } else {
                            !is_true
                        };
                        if jump_taken {
                            let new_offset = offset + 2;
                            code[next_pc] = Opcode::Jump as u8;
                            let bytes = new_offset.to_le_bytes();
                            code[next_pc + 1] = bytes[0];
                            code[next_pc + 2] = bytes[1];
                            code[next_pc + 3] = bytes[2];
                            code[next_pc + 4] = bytes[3];

                            code[next_pc + 5] = 0;
                            code[next_pc + 6] = 0;
                        } else {
                            code[next_pc] = Opcode::Nop as u8;
                            for i in 1..7 {
                                code[next_pc + i] = 0;
                            }
                        }

                        code[pc] = Opcode::Nop as u8;
                        for i in 1..size {
                            code[pc + i] = 0;
                        }
                        changed = true;
                    }
                } else if next_op == Opcode::JumpIf8 || next_op == Opcode::JumpIfNot8 {
                    let src = read_u16(code, next_pc + 1);
                    let jif8_offset = code[next_pc + 3] as i8;
                    if src == dst {
                        let jump_taken = if next_op == Opcode::JumpIf8 {
                            is_true
                        } else {
                            !is_true
                        };
                        if jump_taken {
                            let adjusted = jif8_offset as i32 + 2;
                            if adjusted >= i8::MIN as i32 && adjusted <= i8::MAX as i32 {
                                code[next_pc] = Opcode::Jump8 as u8;
                                code[next_pc + 1] = adjusted as i8 as u8;
                                code[next_pc + 2] = Opcode::Nop as u8;
                                code[next_pc + 3] = 0;
                                code[pc] = Opcode::Nop as u8;
                                for i in 1..size {
                                    code[pc + i] = 0;
                                }
                                changed = true;
                            }
                        } else {
                            code[pc] = Opcode::Nop as u8;
                            for i in 1..size {
                                code[pc + i] = 0;
                            }
                            code[next_pc] = Opcode::Nop as u8;
                            for i in 1..4 {
                                code[next_pc + i] = 0;
                            }
                            changed = true;
                        }
                    }
                }
            }
        }

        pc += size;
    }

    changed
}

#[inline(always)]
fn write_i32(code: &mut [u8], offset: usize, val: i32) {
    let bytes = val.to_le_bytes();
    if offset + 3 >= code.len() {
        return;
    }
    code[offset] = bytes[0];
    code[offset + 1] = bytes[1];
    code[offset + 2] = bytes[2];
    code[offset + 3] = bytes[3];
}

#[inline(always)]
fn read_u16(code: &[u8], offset: usize) -> u16 {
    if offset + 1 >= code.len() {
        return 0;
    }
    u16::from_le_bytes([code[offset], code[offset + 1]])
}

#[inline(always)]
fn read_i32(code: &[u8], offset: usize) -> i32 {
    if offset + 3 >= code.len() {
        return 0;
    }
    i32::from_le_bytes([
        code[offset],
        code[offset + 1],
        code[offset + 2],
        code[offset + 3],
    ])
}
