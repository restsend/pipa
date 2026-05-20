use crate::compiler::ast::*;
use crate::util::FxHashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirConst {
    NumberBits(u64),
    String(String),
    Boolean(bool),
    Null,
    Undefined,
    BigInt(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MirUnaryOp {
    Minus,
    Plus,
    Not,
    BitNot,
    TypeOf,
    Void,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MirBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Neq,
    StrictEq,
    StrictNeq,
    Lt,
    Lte,
    Gt,
    Gte,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UShr,
    In,
    InstanceOf,
}

impl From<&UnaryOp> for MirUnaryOp {
    fn from(value: &UnaryOp) -> Self {
        match value {
            UnaryOp::Minus => Self::Minus,
            UnaryOp::Plus => Self::Plus,
            UnaryOp::Not => Self::Not,
            UnaryOp::BitNot => Self::BitNot,
            UnaryOp::TypeOf => Self::TypeOf,
            UnaryOp::Void => Self::Void,
            UnaryOp::Delete => Self::Delete,
        }
    }
}

impl From<&BinaryOp> for MirBinaryOp {
    fn from(value: &BinaryOp) -> Self {
        match value {
            BinaryOp::Add => Self::Add,
            BinaryOp::Sub => Self::Sub,
            BinaryOp::Mul => Self::Mul,
            BinaryOp::Div => Self::Div,
            BinaryOp::Mod => Self::Mod,
            BinaryOp::Pow => Self::Pow,
            BinaryOp::Eq => Self::Eq,
            BinaryOp::Neq => Self::Neq,
            BinaryOp::StrictEq => Self::StrictEq,
            BinaryOp::StrictNeq => Self::StrictNeq,
            BinaryOp::Lt => Self::Lt,
            BinaryOp::Lte => Self::Lte,
            BinaryOp::Gt => Self::Gt,
            BinaryOp::Gte => Self::Gte,
            BinaryOp::BitAnd => Self::BitAnd,
            BinaryOp::BitOr => Self::BitOr,
            BinaryOp::BitXor => Self::BitXor,
            BinaryOp::Shl => Self::Shl,
            BinaryOp::Shr => Self::Shr,
            BinaryOp::UShr => Self::UShr,
            BinaryOp::In => Self::In,
            BinaryOp::InstanceOf => Self::InstanceOf,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ValueKey {
    Const(MirConst),
    Local {
        name: String,
        version: u32,
    },
    Unary {
        op: MirUnaryOp,
        arg: ValueId,
    },
    Binary {
        op: MirBinaryOp,
        lhs: ValueId,
        rhs: ValueId,
    },
}

#[derive(Debug, Clone)]
pub enum MirInstruction {
    Const {
        id: ValueId,
        value: MirConst,
    },
    ReadLocal {
        id: ValueId,
        name: String,
        version: u32,
    },
    Unary {
        id: ValueId,
        op: MirUnaryOp,
        arg: ValueId,
    },
    Binary {
        id: ValueId,
        op: MirBinaryOp,
        lhs: ValueId,
        rhs: ValueId,
    },
    WriteLocal {
        name: String,
        version: u32,
        value: ValueId,
    },
    Return {
        value: Option<ValueId>,
    },
    Unknown {
        id: ValueId,
    },
}

#[derive(Debug, Clone, Default)]
pub struct SsaStats {
    pub reused_values: u32,
    pub reused_unary: u32,
    pub reused_binary: u32,
}

#[derive(Debug, Clone)]
pub struct SsaFunction {
    pub instructions: Vec<MirInstruction>,
    pub current_versions: FxHashMap<String, u32>,
    pub stats: SsaStats,
}

pub fn build_ssa(block: &BlockStatement) -> SsaFunction {
    SsaBuilder::new().build_block(block)
}

struct SsaBuilder {
    next_value: u32,
    instructions: Vec<MirInstruction>,
    current_versions: FxHashMap<String, u32>,
    value_numbers: FxHashMap<ValueKey, ValueId>,
    stats: SsaStats,
}

impl SsaBuilder {
    fn new() -> Self {
        Self {
            next_value: 0,
            instructions: Vec::new(),
            current_versions: FxHashMap::default(),
            value_numbers: FxHashMap::default(),
            stats: SsaStats::default(),
        }
    }

    fn build_block(mut self, block: &BlockStatement) -> SsaFunction {
        for node in &block.body {
            self.build_node(node);
        }
        SsaFunction {
            instructions: self.instructions,
            current_versions: self.current_versions,
            stats: self.stats,
        }
    }

    fn build_node(&mut self, node: &ASTNode) {
        match node {
            ASTNode::VariableDeclaration(decl) => self.build_var_decl(decl),
            ASTNode::ExpressionStatement(stmt) => {
                self.build_expr(&stmt.expression);
            }
            ASTNode::ReturnStatement(stmt) => {
                let value = stmt.argument.as_ref().map(|expr| self.build_expr(expr));
                self.instructions.push(MirInstruction::Return { value });
            }
            _ => {
                self.invalidate_value_numbers();
            }
        }
    }

    fn build_var_decl(&mut self, decl: &VariableDeclaration) {
        for d in &decl.declarations {
            match &d.id {
                BindingPattern::Identifier(name) => {
                    let init = d
                        .init
                        .as_ref()
                        .map(|expr| self.build_expr(expr))
                        .unwrap_or_else(|| self.emit_const(MirConst::Undefined));
                    self.write_local(name, init);
                    if decl.kind == VariableKind::Const {}
                }
                _ => {
                    if let Some(init) = &d.init {
                        self.build_expr(init);
                    }
                    self.invalidate_value_numbers();
                }
            }
        }
    }

    fn build_expr(&mut self, expr: &Expression) -> ValueId {
        match expr {
            Expression::Literal(lit) => self.build_literal(lit),
            Expression::Identifier(id) => self.read_local(&id.name),
            Expression::UnaryExpression(unary) => {
                let arg = self.build_expr(&unary.argument);
                let op = MirUnaryOp::from(&unary.op);
                let key = ValueKey::Unary { op, arg };
                self.intern_value(key, |id| MirInstruction::Unary { id, op, arg })
            }
            Expression::BinaryExpression(bin) => {
                let lhs = self.build_expr(&bin.left);
                let rhs = self.build_expr(&bin.right);
                let op = MirBinaryOp::from(&bin.op);
                let key = ValueKey::Binary { op, lhs, rhs };
                self.intern_value(key, |id| MirInstruction::Binary { id, op, lhs, rhs })
            }
            Expression::AssignmentExpression(assign) => self.build_assignment(assign),
            Expression::UpdateExpression(update) => self.build_update(update),
            Expression::SequenceExpression(seq) => {
                let mut last = self.emit_const(MirConst::Undefined);
                for expr in &seq.expressions {
                    last = self.build_expr(expr);
                }
                last
            }
            _ => {
                self.invalidate_value_numbers();
                self.emit_unknown()
            }
        }
    }

    fn build_literal(&mut self, lit: &Literal) -> ValueId {
        let value = match lit {
            Literal::Number(n) => MirConst::NumberBits(n.to_bits()),
            Literal::String(s, _) => MirConst::String(s.clone()),
            Literal::Boolean(b) => MirConst::Boolean(*b),
            Literal::Null => MirConst::Null,
            Literal::Undefined => MirConst::Undefined,
            Literal::BigInt(s) => MirConst::BigInt(s.clone()),
            Literal::LegacyOctal(n) => MirConst::NumberBits((*n as f64).to_bits()),
        };
        self.emit_const(value)
    }

    fn build_assignment(&mut self, assign: &AssignmentExpression) -> ValueId {
        let rhs = self.build_expr(&assign.right);
        if let AssignmentTarget::Identifier(name) = &assign.left {
            let value = match assign.op {
                AssignOp::Assign => rhs,
                _ => {
                    if let Some(op) = Self::assign_op_to_binary(assign.op.clone()) {
                        let lhs = self.read_local(name);
                        let key = ValueKey::Binary { op, lhs, rhs };
                        self.intern_value(key, |id| MirInstruction::Binary { id, op, lhs, rhs })
                    } else {
                        self.invalidate_value_numbers();
                        rhs
                    }
                }
            };
            self.write_local(name, value);
            return value;
        }
        self.invalidate_value_numbers();
        self.emit_unknown()
    }

    fn build_update(&mut self, update: &UpdateExpression) -> ValueId {
        if let Expression::Identifier(id) = &*update.argument {
            let old = self.read_local(&id.name);
            let one = self.emit_const(MirConst::NumberBits(1.0f64.to_bits()));
            let op = match update.op {
                UpdateOp::Increment => MirBinaryOp::Add,
                UpdateOp::Decrement => MirBinaryOp::Sub,
            };
            let key = ValueKey::Binary {
                op,
                lhs: old,
                rhs: one,
            };
            let next = self.intern_value(key, |idv| MirInstruction::Binary {
                id: idv,
                op,
                lhs: old,
                rhs: one,
            });
            self.write_local(&id.name, next);
            return if update.prefix { next } else { old };
        }
        self.invalidate_value_numbers();
        self.emit_unknown()
    }

    fn assign_op_to_binary(op: AssignOp) -> Option<MirBinaryOp> {
        match op {
            AssignOp::AddAssign => Some(MirBinaryOp::Add),
            AssignOp::SubAssign => Some(MirBinaryOp::Sub),
            AssignOp::MulAssign => Some(MirBinaryOp::Mul),
            AssignOp::DivAssign => Some(MirBinaryOp::Div),
            AssignOp::ModAssign => Some(MirBinaryOp::Mod),
            AssignOp::PowAssign => Some(MirBinaryOp::Pow),
            AssignOp::BitAndAssign => Some(MirBinaryOp::BitAnd),
            AssignOp::BitOrAssign => Some(MirBinaryOp::BitOr),
            AssignOp::BitXorAssign => Some(MirBinaryOp::BitXor),
            AssignOp::ShlAssign => Some(MirBinaryOp::Shl),
            AssignOp::ShrAssign => Some(MirBinaryOp::Shr),
            AssignOp::UShrAssign => Some(MirBinaryOp::UShr),
            AssignOp::Assign
            | AssignOp::LogicalAndAssign
            | AssignOp::LogicalOrAssign
            | AssignOp::NullishAssign => None,
        }
    }

    fn read_local(&mut self, name: &str) -> ValueId {
        let version = self.current_versions.get(name).copied().unwrap_or(0);
        let key = ValueKey::Local {
            name: name.to_string(),
            version,
        };
        self.intern_value(key, |id| MirInstruction::ReadLocal {
            id,
            name: name.to_string(),
            version,
        })
    }

    fn write_local(&mut self, name: &str, value: ValueId) {
        let version = self.current_versions.get(name).copied().unwrap_or(0) + 1;
        self.current_versions.insert(name.to_string(), version);
        self.instructions.push(MirInstruction::WriteLocal {
            name: name.to_string(),
            version,
            value,
        });
    }

    fn emit_const(&mut self, value: MirConst) -> ValueId {
        let key = ValueKey::Const(value.clone());
        self.intern_value(key, |id| MirInstruction::Const { id, value })
    }

    fn emit_unknown(&mut self) -> ValueId {
        let id = self.alloc_value();
        self.instructions.push(MirInstruction::Unknown { id });
        id
    }

    fn intern_value<F>(&mut self, key: ValueKey, make_inst: F) -> ValueId
    where
        F: FnOnce(ValueId) -> MirInstruction,
    {
        if let Some(existing) = self.value_numbers.get(&key).copied() {
            self.stats.reused_values += 1;
            if matches!(key, ValueKey::Unary { .. }) {
                self.stats.reused_unary += 1;
            }
            if matches!(key, ValueKey::Binary { .. }) {
                self.stats.reused_binary += 1;
            }
            return existing;
        }

        let id = self.alloc_value();
        self.instructions.push(make_inst(id));
        self.value_numbers.insert(key, id);
        id
    }

    fn invalidate_value_numbers(&mut self) {
        self.value_numbers
            .retain(|key, _| matches!(key, ValueKey::Const(_)));
    }

    fn alloc_value(&mut self) -> ValueId {
        let id = ValueId(self.next_value);
        self.next_value += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_function_body(code: &str) -> BlockStatement {
        let program = crate::compiler::parser::Parser::new(code).parse().unwrap();
        match program.body.first() {
            Some(ASTNode::FunctionDeclaration(func)) => func.body.clone(),
            _ => panic!("expected function declaration"),
        }
    }

    #[test]
    fn ssa_versions_increment_on_reassignment() {
        let body = parse_function_body("function f() { let x = 1; x = x + 2; return x; }");
        let ssa = build_ssa(&body);
        assert_eq!(ssa.current_versions.get("x"), Some(&2));
    }

    #[test]
    fn lvn_reuses_identical_binary_expr() {
        let body = parse_function_body(
            "function f() { let a = 1; let b = 2; let x = a + b; let y = a + b; return y; }",
        );
        let ssa = build_ssa(&body);
        assert!(
            ssa.stats.reused_binary >= 1,
            "expected binary value reuse, got {:?}",
            ssa.stats
        );
    }

    #[test]
    fn lvn_does_not_reuse_after_version_change() {
        let body = parse_function_body(
            "function f() { let a = 1; let b = 2; let x = a + b; a = 3; let y = a + b; return y; }",
        );
        let ssa = build_ssa(&body);
        assert_eq!(ssa.stats.reused_binary, 0);
    }
}
