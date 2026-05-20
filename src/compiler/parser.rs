use super::ast::*;
use crate::compiler::lexer::{Lexer, Token, TokenType};

pub struct Parser {
    pub lexer: Lexer,
    current: Option<Token>,
    peek_token: Option<Token>,

    saved_pos: usize,
    saved_line: u32,
    saved_col: u32,
    allow_in: bool,
}

impl Parser {
    pub fn new(source: &str) -> Self {
        let mut lexer = Lexer::new(source);
        let current = lexer.next_token();
        let saved_pos = lexer.pos;
        let saved_line = lexer.line;
        let saved_col = lexer.column;
        let peek_token = lexer.next_token();
        Parser {
            lexer,
            current,
            peek_token,
            saved_pos,
            saved_line,
            saved_col,
            allow_in: true,
        }
    }

    fn advance(&mut self) -> Option<Token> {
        let cur = self.current.take();
        if let Some(ref t) = cur {
            self.saved_line = t.line;
            self.saved_col = t.column;
        }
        self.current = self.peek_token.take();
        self.saved_pos = self.lexer.pos;
        self.peek_token = self.lexer.next_token();
        cur
    }

    fn expect(&mut self, value: &str) -> Result<Token, String> {
        match &self.current {
            Some(t) if t.value == value => {
                let tok = t.clone();
                self.advance();
                Ok(tok)
            }
            Some(t) => Err(self.error(&format!(
                "Expected '{}', but got '{}' at pos {}",
                value,
                t.value,
                self.lexer.pos()
            ))),
            None => Err(self.error(&format!("Expected '{}', but got EOF", value))),
        }
    }

    fn expect_semicolon(&mut self) -> Result<(), String> {
        if let Some(ref t) = self.current {
            if t.value == ";" {
                self.advance();
            }
        }
        Ok(())
    }

    fn at(&self, value: &str) -> bool {
        self.current.as_ref().map_or(false, |t| t.value == value)
    }

    fn at_keyword(&self, kw: &str) -> bool {
        self.current.as_ref().map_or(false, |t| {
            t.token_type == TokenType::Keyword && t.value == kw
        })
    }

    fn at_punct(&self, p: &str) -> bool {
        self.current.as_ref().map_or(false, |t| {
            t.token_type == TokenType::Punctuator && t.value == p
        })
    }

    fn at_template_start(&self) -> bool {
        self.current.as_ref().map_or(false, |t| {
            matches!(
                t.token_type,
                TokenType::NoSubstitutionTemplate | TokenType::TemplateHead
            )
        })
    }

    fn peek_at(&self, value: &str) -> bool {
        self.peek_token.as_ref().map_or(false, |t| t.value == value)
    }

    fn peek_value(&self) -> Option<&str> {
        self.peek_token.as_ref().map(|t| t.value.as_str())
    }

    fn cur_value(&self) -> &str {
        self.current
            .as_ref()
            .map(|t| t.value.as_str())
            .unwrap_or("")
    }

    fn cur_type(&self) -> Option<TokenType> {
        self.current.as_ref().map(|t| t.token_type)
    }

    fn is_eof(&self) -> bool {
        self.current.is_none()
    }

    fn error(&self, msg: &str) -> String {
        let line = self.current.as_ref().map(|t| t.line).unwrap_or(0);
        let col = self.current.as_ref().map(|t| t.column).unwrap_or(0);
        format!("Parse error ({}:{}): {}", line, col, msg)
    }
}

impl Parser {
    pub fn parse(&mut self) -> Result<Program, String> {
        if let Some(ref t) = self.current {
            if t.token_type == TokenType::Hashbang {
                self.advance();
            }
        }

        let mut body = Vec::new();
        let mut lines = Vec::new();
        while !self.is_eof() {
            let line = self.current.as_ref().map(|t| t.line).unwrap_or(0);
            body.push(self.parse_statement_list_item()?);
            lines.push(line);
        }

        Ok(Program {
            body,
            lines,
            source_type: SourceType::Script,
        })
    }

    fn parse_statement_list_item(&mut self) -> Result<ASTNode, String> {
        if self.at_keyword("import") && !self.peek_at("(") {
            return self
                .parse_import_declaration()
                .map(ASTNode::ImportDeclaration);
        }
        if self.at_keyword("export") {
            return self.parse_export_declaration();
        }
        self.parse_statement()
    }

    fn parse_statement(&mut self) -> Result<ASTNode, String> {
        if self.is_eof() {
            return Ok(ASTNode::EmptyStatement);
        }

        let tok = self.current.clone();

        match tok.as_ref().map(|t| (t.token_type, t.value.as_str())) {
            Some((TokenType::Punctuator, "{")) => self.parse_block_statement(),
            Some((TokenType::Punctuator, ";")) => {
                self.advance();
                Ok(ASTNode::EmptyStatement)
            }
            Some((TokenType::Keyword, "var"))
            | Some((TokenType::Keyword, "let"))
            | Some((TokenType::Keyword, "const")) => self.parse_variable_declaration(),
            Some((TokenType::Keyword, "function")) => {
                self.advance();
                let generator = self.at_punct("*");
                if generator {
                    self.advance();
                }
                self.parse_function_declaration(false, generator)
            }
            Some((TokenType::Keyword, "async")) => {
                self.advance();
                self.expect("function")?;
                let generator = self.at_punct("*");
                if generator {
                    self.advance();
                }
                self.parse_function_declaration(true, generator)
            }
            Some((TokenType::Keyword, "class")) => self.parse_class_declaration(),
            Some((TokenType::Keyword, "if")) => self.parse_if_statement(),
            Some((TokenType::Keyword, "for")) => self.parse_for_statement(),
            Some((TokenType::Keyword, "while")) => self.parse_while_statement(),
            Some((TokenType::Keyword, "do")) => self.parse_do_while_statement(),
            Some((TokenType::Keyword, "switch")) => self.parse_switch_statement(),
            Some((TokenType::Keyword, "try")) => self.parse_try_statement(),
            Some((TokenType::Keyword, "with")) => self.parse_with_statement(),
            Some((TokenType::Keyword, "return")) => self.parse_return_statement(),
            Some((TokenType::Keyword, "throw")) => self.parse_throw_statement(),
            Some((TokenType::Keyword, "break")) => self.parse_break_statement(),
            Some((TokenType::Keyword, "continue")) => self.parse_continue_statement(),
            Some((TokenType::Keyword, "debugger")) => {
                self.advance();
                self.expect_semicolon()?;
                Ok(ASTNode::DebuggerStatement)
            }
            Some((TokenType::Keyword, "import")) if self.peek_at("(") => {
                self.parse_expression_statement()
            }
            _ => self.parse_expression_statement_or_labelled(),
        }
    }

    fn parse_block_statement(&mut self) -> Result<ASTNode, String> {
        self.expect("{")?;
        let (body, lines) = self.parse_statement_list_with_lines()?;
        self.expect("}")?;
        Ok(ASTNode::BlockStatement(BlockStatement { body, lines }))
    }

    fn parse_statement_list_with_lines(&mut self) -> Result<(Vec<ASTNode>, Vec<u32>), String> {
        let mut stmts = Vec::new();
        let mut lines = Vec::new();
        while !self.is_eof() && !self.at_punct("}") {
            let line = self.current.as_ref().map(|t| t.line).unwrap_or(0);
            stmts.push(self.parse_statement()?);
            lines.push(line);
        }
        Ok((stmts, lines))
    }

    fn parse_variable_declaration(&mut self) -> Result<ASTNode, String> {
        let kind = match self.cur_value() {
            "var" => VariableKind::Var,
            "let" => VariableKind::Let,
            "const" => VariableKind::Const,
            _ => return Err(self.error("Expected var/let/const")),
        };
        self.advance();

        let mut declarations = Vec::new();
        loop {
            let id = self.parse_binding_pattern()?;
            let init = if self.at_punct("=") {
                self.advance();
                Some(self.parse_assignment_expression()?)
            } else {
                None
            };
            declarations.push(VariableDeclarator { id, init });
            if self.at_punct(",") {
                self.advance();
            } else {
                break;
            }
        }

        if !self.at_punct(")") && !self.is_eof() {
            self.expect_semicolon()?;
        }

        Ok(ASTNode::VariableDeclaration(VariableDeclaration {
            kind,
            declarations,
        }))
    }

    fn parse_binding_pattern(&mut self) -> Result<BindingPattern, String> {
        if self.at_punct("[") {
            self.advance();
            let mut elements = Vec::new();
            while !self.at_punct("]") && !self.is_eof() {
                if self.at_punct(",") {
                    elements.push(None);
                    self.advance();
                    continue;
                }
                let rest = if self.at_punct("...") {
                    self.advance();
                    let arg = self.parse_binding_pattern()?;
                    Some(Box::new(match arg {
                        BindingPattern::Identifier(name) => AssignmentTarget::Identifier(name),
                        other => AssignmentTarget::ArrayPattern(match other {
                            BindingPattern::ArrayPattern(p) => p,
                            BindingPattern::ObjectPattern(_) => {
                                return Err(self.error("rest element must be array/identifier"));
                            }
                            BindingPattern::AssignmentPattern(_) => {
                                return Err(self.error("rest element cannot be assignment pattern"));
                            }
                            BindingPattern::Identifier(_) => unreachable!(),
                        }),
                    }))
                } else {
                    None
                };
                if let Some(arg) = rest {
                    if self.at_punct(",") {
                        self.advance();
                    }
                    elements.push(Some(PatternElement::RestElement(RestElement {
                        argument: arg,
                    })));
                    continue;
                }
                let pattern = self.parse_binding_pattern()?;
                let elem = if self.at_punct("=") {
                    self.advance();
                    let right = self.parse_assignment_expression()?;
                    PatternElement::AssignmentPattern(AssignmentPattern {
                        left: Box::new(binding_to_target(pattern.clone())),
                        right: Box::new(right),
                    })
                } else {
                    PatternElement::Pattern(binding_to_target(pattern))
                };
                elements.push(Some(elem));
                if self.at_punct(",") {
                    self.advance();
                }
            }
            self.expect("]")?;
            return Ok(BindingPattern::ArrayPattern(ArrayPattern { elements }));
        }

        if self.at_punct("{") {
            self.advance();
            let mut properties = Vec::new();
            while !self.at_punct("}") && !self.is_eof() {
                if self.at_punct("...") {
                    self.advance();
                    let arg = self.parse_binding_pattern()?;
                    properties.push(ObjectPatternProperty::RestElement(RestElement {
                        argument: Box::new(binding_to_target(arg)),
                    }));
                    if self.at_punct(",") {
                        self.advance();
                    }
                    continue;
                }
                let (key, computed, shorthand) = self.parse_property_key()?;
                let value = if self.at_punct(":") {
                    self.advance();
                    binding_to_target(self.parse_binding_pattern()?)
                } else if shorthand
                    || (self.at_punct("=") && matches!(key, PropertyKey::Identifier(_)))
                {
                    match &key {
                        PropertyKey::Identifier(name) => AssignmentTarget::Identifier(name.clone()),
                        _ => return Err(self.error("invalid shorthand pattern")),
                    }
                } else {
                    return Err(self.error("expected ':' in object pattern"));
                };

                let value = if self.at_punct("=") {
                    self.advance();
                    let right = self.parse_assignment_expression()?;
                    AssignmentTarget::AssignmentPattern(AssignmentPattern {
                        left: Box::new(value),
                        right: Box::new(right),
                    })
                } else {
                    value
                };
                properties.push(ObjectPatternProperty::Property {
                    key,
                    value,
                    computed,
                    shorthand,
                });
                if self.at_punct(",") {
                    self.advance();
                }
            }
            self.expect("}")?;
            return Ok(BindingPattern::ObjectPattern(ObjectPattern { properties }));
        }

        let name = self.cur_value().to_string();
        if !self.is_identifier_name() {
            return Err(self.error("expected identifier"));
        }
        self.advance();
        Ok(BindingPattern::Identifier(name))
    }

    fn is_identifier_name(&self) -> bool {
        matches!(
            self.cur_type(),
            Some(TokenType::Identifier) | Some(TokenType::Keyword)
        ) && !matches!(self.cur_value(), "true" | "false" | "null")
    }

    fn parse_function_declaration(
        &mut self,
        is_async: bool,
        generator: bool,
    ) -> Result<ASTNode, String> {
        let name = match self.cur_type() {
            Some(TokenType::Identifier) | Some(TokenType::Keyword) => {
                let n = self.cur_value().to_string();
                self.advance();
                n
            }
            _ => return Err(self.error("expected function name")),
        };
        self.expect("(")?;
        let params = self.parse_parameter_list()?;
        self.expect(")")?;
        let body = self.parse_block_body()?;
        Ok(ASTNode::FunctionDeclaration(FunctionDeclaration {
            name,
            params,
            body,
            generator,
            is_async,
        }))
    }

    fn parse_function_expression(
        &mut self,
        is_async: bool,
        generator: bool,
    ) -> Result<Expression, String> {
        let name = if matches!(
            self.cur_type(),
            Some(TokenType::Identifier) | Some(TokenType::Keyword)
        ) && !self.at_punct("(")
        {
            let n = self.cur_value().to_string();
            self.advance();
            Some(n)
        } else {
            None
        };
        self.expect("(")?;
        let params = self.parse_parameter_list()?;
        self.expect(")")?;
        let body = self.parse_block_body()?;
        Ok(Expression::FunctionExpression(FunctionExpression {
            name,
            params,
            body,
            generator,
            is_async,
        }))
    }

    fn parse_parameter_list(&mut self) -> Result<Vec<Parameter>, String> {
        let mut params = Vec::new();
        while !self.at_punct(")") && !self.is_eof() {
            if self.at_punct("...") {
                self.advance();
                let arg = self.parse_binding_pattern()?;
                params.push(Parameter::RestElement(RestElement {
                    argument: Box::new(binding_to_target(arg)),
                }));
                break;
            }
            let pattern = self.parse_binding_pattern()?;
            if self.at_punct("=") {
                self.advance();
                let right = self.parse_assignment_expression()?;
                params.push(Parameter::AssignmentPattern(AssignmentPattern {
                    left: Box::new(binding_to_target(pattern)),
                    right: Box::new(right),
                }));
            } else {
                match pattern {
                    BindingPattern::Identifier(name) => params.push(Parameter::Identifier(name)),
                    other => params.push(Parameter::Pattern(other)),
                }
            }
            if self.at_punct(",") {
                self.advance();
            }
        }
        Ok(params)
    }

    fn parse_expression_no_in(&mut self) -> Result<Expression, String> {
        let prev_allow_in = self.allow_in;
        self.allow_in = false;
        let result = self.parse_expression();
        self.allow_in = prev_allow_in;
        result
    }

    fn parse_assignment_expression_no_in(&mut self) -> Result<Expression, String> {
        let prev_allow_in = self.allow_in;
        self.allow_in = false;
        let result = self.parse_assignment_expression();
        self.allow_in = prev_allow_in;
        result
    }

    fn parse_block_body(&mut self) -> Result<BlockStatement, String> {
        self.expect("{")?;
        let (body, lines) = self.parse_statement_list_with_lines()?;
        self.expect("}")?;
        Ok(BlockStatement { body, lines })
    }

    fn parse_class_declaration(&mut self) -> Result<ASTNode, String> {
        self.expect("class")?;
        let name = self.cur_value().to_string();
        self.advance();
        let super_class = if self.at_keyword("extends") {
            self.advance();
            Some(Box::new(self.parse_assignment_expression()?))
        } else {
            None
        };
        self.expect("{")?;
        let body = self.parse_class_body()?;
        self.expect("}")?;
        Ok(ASTNode::ClassDeclaration(ClassDeclaration {
            name,
            super_class,
            body,
        }))
    }

    fn parse_class_expression(&mut self) -> Result<Expression, String> {
        self.expect("class")?;
        let name = if self.is_identifier_name() && !self.at_keyword("extends") {
            let n = self.cur_value().to_string();
            self.advance();
            Some(n)
        } else {
            None
        };
        let super_class = if self.at_keyword("extends") {
            self.advance();
            Some(Box::new(self.parse_assignment_expression()?))
        } else {
            None
        };
        self.expect("{")?;
        let body = self.parse_class_body()?;
        self.expect("}")?;
        Ok(Expression::ClassExpression(ClassExpression {
            name,
            super_class,
            body,
        }))
    }

    fn parse_class_body(&mut self) -> Result<ClassBody, String> {
        let mut elements = Vec::new();
        while !self.at_punct("}") && !self.is_eof() {
            if self.at_punct(";") {
                self.advance();
                continue;
            }

            let is_static = if self.at_keyword("static") {
                self.advance();
                true
            } else {
                false
            };

            if is_static && self.at_punct("{") {
                self.expect("{")?;
                let (body, lines) = self.parse_statement_list_with_lines()?;
                self.expect("}")?;
                elements.push(ClassElement::StaticBlock(StaticBlock { body, lines }));
                continue;
            }

            let mut is_async = false;
            if self.at_keyword("async")
                && !self.peek_at(";")
                && !self.peek_at("=")
                && !self.peek_at("}")
            {
                self.advance();
                is_async = true;
            }

            let mut is_generator = false;
            if self.at_punct("*") {
                self.advance();
                is_generator = true;
            }

            let method_kind = if self.at_keyword("get") && !self.peek_at("(") && !self.peek_at(",")
            {
                self.advance();
                MethodKind::Get
            } else if self.at_keyword("set") && !self.peek_at("(") && !self.peek_at(",") {
                self.advance();
                MethodKind::Set
            } else if is_static && self.at_keyword("constructor") {
                MethodKind::Constructor
            } else {
                MethodKind::Method
            };

            let is_private = self.cur_type() == Some(TokenType::PrivateIdentifier);
            let (name, _computed, _shorthand) = self.parse_property_key()?;

            if self.at_punct("=") {
                self.advance();
                let value = Some(self.parse_assignment_expression()?);
                elements.push(ClassElement::Property(ClassProperty {
                    name,
                    value,
                    is_static,
                    is_private,
                }));
                if self.at_punct(";") {
                    self.advance();
                }
                continue;
            }

            if self.at_punct(";") {
                self.advance();
                elements.push(ClassElement::Property(ClassProperty {
                    name,
                    value: None,
                    is_static,
                    is_private,
                }));
                continue;
            }

            if !self.at_punct("(") {
                elements.push(ClassElement::Property(ClassProperty {
                    name,
                    value: None,
                    is_static,
                    is_private,
                }));
                continue;
            }

            self.expect("(")?;
            let params = self.parse_parameter_list()?;
            self.expect(")")?;
            let body = self.parse_block_body()?;

            let is_constructor_name =
                matches!(&name, PropertyKey::Identifier(n) if n == "constructor");
            let actual_kind = if !is_static && is_constructor_name {
                MethodKind::Constructor
            } else {
                method_kind
            };

            elements.push(ClassElement::Method(ClassMethod {
                name,
                params,
                body,
                kind: actual_kind,
                is_async,
                generator: is_generator,
                is_static,
                is_private,
            }));
        }
        Ok(ClassBody { elements })
    }

    fn parse_if_statement(&mut self) -> Result<ASTNode, String> {
        self.expect("if")?;
        self.expect("(")?;
        let test = self.parse_expression()?;
        self.expect(")")?;
        let consequent = self.parse_statement()?;
        let alternate = if self.at_keyword("else") {
            self.advance();
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };
        Ok(ASTNode::IfStatement(IfStatement {
            test,
            consequent: Box::new(consequent),
            alternate,
        }))
    }

    fn parse_for_statement(&mut self) -> Result<ASTNode, String> {
        self.expect("for")?;
        self.expect("(")?;

        if self.at_punct(";") {
            self.advance();
            return self.parse_for_rest(None);
        }

        let is_await = if self.at_keyword("await") {
            self.advance();
            true
        } else {
            false
        };

        let is_var = matches!(self.cur_value(), "var" | "let" | "const");
        if is_var {
            let kind = match self.cur_value() {
                "var" => VariableKind::Var,
                "let" => VariableKind::Let,
                "const" => VariableKind::Const,
                _ => unreachable!(),
            };
            self.advance();
            let mut declarations = Vec::new();
            let id = self.parse_binding_pattern()?;

            if self.at_keyword("in") {
                self.advance();
                let right = self.parse_expression()?;
                self.expect(")")?;
                let body = self.parse_statement()?;
                let decl = VariableDeclaration {
                    kind,
                    declarations: vec![VariableDeclarator { id, init: None }],
                };
                return Ok(ASTNode::ForInStatement(ForInStatement {
                    left: ForInOfLeft::VariableDeclaration(decl),
                    right,
                    body: Box::new(body),
                }));
            }
            if self.at_keyword("of") {
                self.advance();
                let right = self.parse_expression()?;
                self.expect(")")?;
                let body = self.parse_statement()?;
                let decl = VariableDeclaration {
                    kind,
                    declarations: vec![VariableDeclarator { id, init: None }],
                };
                return Ok(ASTNode::ForOfStatement(ForOfStatement {
                    left: ForInOfLeft::VariableDeclaration(decl),
                    right,
                    body: Box::new(body),
                    is_await,
                }));
            }
            let init = if self.at_punct("=") {
                self.advance();
                Some(self.parse_assignment_expression_no_in()?)
            } else {
                None
            };
            declarations.push(VariableDeclarator { id, init });
            while self.at_punct(",") {
                self.advance();
                let id = self.parse_binding_pattern()?;
                let init = if self.at_punct("=") {
                    self.advance();
                    Some(self.parse_assignment_expression_no_in()?)
                } else {
                    None
                };
                declarations.push(VariableDeclarator { id, init });
            }
            let decl = VariableDeclaration { kind, declarations };
            self.expect(";")?;
            return self.parse_for_rest(Some(ForInit::VariableDeclaration(decl)));
        }

        let expr = self.parse_expression_no_in()?;
        if self.at_keyword("in") {
            self.advance();
            let right = self.parse_expression()?;
            self.expect(")")?;
            let body = self.parse_statement()?;
            return Ok(ASTNode::ForInStatement(ForInStatement {
                left: ForInOfLeft::AssignmentTarget(expr_to_target(expr)?),
                right,
                body: Box::new(body),
            }));
        }
        if self.at_keyword("of") {
            self.advance();
            let right = self.parse_expression()?;
            self.expect(")")?;
            let body = self.parse_statement()?;
            return Ok(ASTNode::ForOfStatement(ForOfStatement {
                left: ForInOfLeft::AssignmentTarget(expr_to_target(expr)?),
                right,
                body: Box::new(body),
                is_await,
            }));
        }
        self.expect(";")?;
        self.parse_for_rest(Some(ForInit::Expression(expr)))
    }

    fn parse_for_rest(&mut self, init: Option<ForInit>) -> Result<ASTNode, String> {
        let test = if self.at_punct(";") {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(";")?;
        let update = if self.at_punct(")") {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(")")?;
        let body = self.parse_statement()?;
        Ok(ASTNode::ForStatement(ForStatement {
            init,
            test,
            update,
            body: Box::new(body),
        }))
    }

    fn parse_while_statement(&mut self) -> Result<ASTNode, String> {
        self.expect("while")?;
        self.expect("(")?;
        let test = self.parse_expression()?;
        self.expect(")")?;
        let body = self.parse_statement()?;
        Ok(ASTNode::WhileStatement(WhileStatement {
            test,
            body: Box::new(body),
        }))
    }

    fn parse_do_while_statement(&mut self) -> Result<ASTNode, String> {
        self.expect("do")?;
        let body = self.parse_statement()?;
        self.expect_keyword("while")?;
        self.expect("(")?;
        let test = self.parse_expression()?;
        self.expect(")")?;
        self.expect_semicolon()?;
        Ok(ASTNode::DoWhileStatement(DoWhileStatement {
            body: Box::new(body),
            test,
        }))
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<(), String> {
        if self.at_keyword(kw) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!("Expected keyword '{}'", kw)))
        }
    }

    fn parse_switch_statement(&mut self) -> Result<ASTNode, String> {
        self.expect("switch")?;
        self.expect("(")?;
        let discriminant = self.parse_expression()?;
        self.expect(")")?;
        self.expect("{")?;
        let mut cases = Vec::new();
        while !self.at_punct("}") && !self.is_eof() {
            if self.at_keyword("default") {
                self.advance();
                self.expect(":")?;
                let consequent = self.parse_statement_list_until_case()?;
                cases.push(SwitchCase {
                    test: None,
                    consequent,
                });
            } else {
                self.expect_keyword("case")?;
                let test = self.parse_expression()?;
                self.expect(":")?;
                let consequent = self.parse_statement_list_until_case()?;
                cases.push(SwitchCase {
                    test: Some(test),
                    consequent,
                });
            }
        }
        self.expect("}")?;
        Ok(ASTNode::SwitchStatement(SwitchStatement {
            discriminant,
            cases,
        }))
    }

    fn parse_statement_list_until_case(&mut self) -> Result<Vec<ASTNode>, String> {
        let (stmts, _) = self.parse_statement_list_until_case_with_lines()?;
        Ok(stmts)
    }

    fn parse_statement_list_until_case_with_lines(
        &mut self,
    ) -> Result<(Vec<ASTNode>, Vec<u32>), String> {
        let mut stmts = Vec::new();
        let mut lines = Vec::new();
        while !self.is_eof()
            && !self.at_punct("}")
            && !self.at_keyword("case")
            && !self.at_keyword("default")
        {
            let line = self.current.as_ref().map(|t| t.line).unwrap_or(0);
            stmts.push(self.parse_statement()?);
            lines.push(line);
        }
        Ok((stmts, lines))
    }

    fn parse_try_statement(&mut self) -> Result<ASTNode, String> {
        self.expect("try")?;
        let block = self.parse_block_body()?;
        let handler = if self.at_keyword("catch") {
            self.advance();
            let param = if self.at_punct("(") {
                self.advance();
                let p = self.parse_binding_pattern()?;
                self.expect(")")?;
                Some(p)
            } else {
                None
            };
            let body = self.parse_block_body()?;
            Some(CatchClause { param, body })
        } else {
            None
        };
        let finalizer = if self.at_keyword("finally") {
            self.advance();
            Some(self.parse_block_body()?)
        } else {
            None
        };
        Ok(ASTNode::TryStatement(TryStatement {
            block,
            handler,
            finalizer,
        }))
    }

    fn parse_with_statement(&mut self) -> Result<ASTNode, String> {
        self.expect("with")?;
        self.expect("(")?;
        let object = self.parse_expression()?;
        self.expect(")")?;
        let body = self.parse_statement()?;
        Ok(ASTNode::WithStatement(WithStatement {
            object,
            body: Box::new(body),
        }))
    }

    fn has_line_terminator(&self) -> bool {
        self.current
            .as_ref()
            .map_or(false, |t| t.line > self.saved_line)
    }

    fn parse_return_statement(&mut self) -> Result<ASTNode, String> {
        self.expect("return")?;
        let argument = if self.is_eof()
            || self.at_punct(";")
            || self.at_punct("}")
            || self.has_line_terminator()
        {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect_semicolon()?;
        Ok(ASTNode::ReturnStatement(ReturnStatement { argument }))
    }

    fn parse_throw_statement(&mut self) -> Result<ASTNode, String> {
        self.expect("throw")?;
        let argument = self.parse_expression()?;
        self.expect_semicolon()?;
        Ok(ASTNode::ThrowStatement(ThrowStatement { argument }))
    }

    fn parse_break_statement(&mut self) -> Result<ASTNode, String> {
        self.expect("break")?;
        let label = if !self.has_line_terminator()
            && self.is_identifier_name()
            && !self.at_punct(";")
            && !self.is_eof()
        {
            Some(self.cur_value().to_string())
        } else {
            None
        };
        if label.is_some() {
            self.advance();
        }
        self.expect_semicolon()?;
        Ok(ASTNode::BreakStatement(BreakStatement { label }))
    }

    fn parse_continue_statement(&mut self) -> Result<ASTNode, String> {
        self.expect("continue")?;
        let label = if !self.has_line_terminator()
            && self.is_identifier_name()
            && !self.at_punct(";")
            && !self.is_eof()
        {
            Some(self.cur_value().to_string())
        } else {
            None
        };
        if label.is_some() {
            self.advance();
        }
        self.expect_semicolon()?;
        Ok(ASTNode::ContinueStatement(ContinueStatement { label }))
    }

    fn parse_expression_statement_or_labelled(&mut self) -> Result<ASTNode, String> {
        if self.is_identifier_name() && self.peek_at(":") {
            let label = self.cur_value().to_string();
            self.advance();
            self.advance();
            let body = self.parse_statement()?;
            return Ok(ASTNode::LabelledStatement(LabelledStatement {
                label,
                body: Box::new(body),
            }));
        }
        self.parse_expression_statement()
    }

    fn parse_expression_statement(&mut self) -> Result<ASTNode, String> {
        let expr = self.parse_expression()?;
        self.expect_semicolon()?;
        Ok(ASTNode::ExpressionStatement(ExpressionStatement {
            expression: expr,
        }))
    }

    fn parse_import_declaration(&mut self) -> Result<ImportDeclaration, String> {
        self.expect("import")?;
        let mut specifiers = Vec::new();

        if self.cur_type() == Some(TokenType::String) {
            let source = self.cur_value().to_string();
            self.advance();
            self.expect_semicolon()?;
            return Ok(ImportDeclaration { specifiers, source });
        }

        if self.is_identifier_name() {
            let local = self.cur_value().to_string();
            self.advance();
            specifiers.push(ImportSpecifier::Default { local });

            if self.at_punct(",") {
                self.advance();
            }
        }

        if self.at_punct("*") {
            self.advance();
            self.expect_keyword("as")?;
            let local = self.cur_value().to_string();
            self.advance();
            specifiers.push(ImportSpecifier::Namespace { local });
        }

        if self.at_punct("{") {
            self.advance();
            while !self.at_punct("}") && !self.is_eof() && !self.at_keyword("from") {
                if self.at_punct(",") {
                    self.advance();
                    continue;
                }
                let imported = self.cur_value().to_string();
                self.advance();
                let local = if self.at_keyword("as") {
                    self.advance();
                    self.cur_value().to_string()
                } else {
                    imported.clone()
                };
                if !self.at_punct(",") && !self.at_punct("}") && !self.at_keyword("from") {
                    self.advance();
                }
                specifiers.push(ImportSpecifier::Named { imported, local });
            }
            self.expect("}")?;
        }

        self.expect_keyword("from")?;
        let source = self.cur_value().to_string();
        self.advance();
        self.expect_semicolon()?;
        Ok(ImportDeclaration { specifiers, source })
    }

    fn parse_export_declaration(&mut self) -> Result<ASTNode, String> {
        self.expect("export")?;

        if self.at_keyword("default") {
            self.advance();
            let decl = if self.at_keyword("function") {
                self.advance();
                let generator = self.at_punct("*");
                if generator {
                    self.advance();
                }
                let func = self.parse_function_declaration_like(false, generator)?;
                ExportDefaultDeclarationKind::Function(func)
            } else if self.at_keyword("class") {
                self.advance();
                let cls = self.parse_class_declaration_inner()?;
                ExportDefaultDeclarationKind::Class(cls)
            } else if self.at_keyword("async") {
                self.advance();
                self.expect("function")?;
                let generator = self.at_punct("*");
                if generator {
                    self.advance();
                }
                let func = self.parse_function_declaration_like(true, generator)?;
                ExportDefaultDeclarationKind::Function(func)
            } else {
                let expr = self.parse_assignment_expression()?;
                ExportDefaultDeclarationKind::Expression(expr)
            };
            self.expect_semicolon()?;
            return Ok(ASTNode::ExportDefaultDeclaration(
                ExportDefaultDeclaration { declaration: decl },
            ));
        }

        if self.at_punct("*") {
            self.advance();
            self.expect_keyword("from")?;
            let source = self.cur_value().to_string();
            self.advance();
            self.expect_semicolon()?;
            return Ok(ASTNode::ExportAllDeclaration(ExportAllDeclaration {
                source,
                exported: None,
            }));
        }

        if self.at_punct("{") {
            self.advance();
            let mut specifiers = Vec::new();
            while !self.at_punct("}") && !self.is_eof() {
                let local = self.cur_value().to_string();
                self.advance();
                let exported = if self.at_keyword("as") {
                    self.advance();
                    let name = self.cur_value().to_string();
                    self.advance();
                    name
                } else {
                    local.clone()
                };
                specifiers.push(ExportSpecifier { exported, local });
                if self.at_punct(",") {
                    self.advance();
                }
            }
            self.expect("}")?;
            let source = if self.at_keyword("from") {
                self.advance();
                let s = self.cur_value().to_string();
                self.advance();
                Some(s)
            } else {
                None
            };
            self.expect_semicolon()?;
            return Ok(ASTNode::ExportNamedDeclaration(ExportNamedDeclaration {
                declaration: None,
                specifiers,
                source,
            }));
        }

        if self.at_keyword("var") || self.at_keyword("let") || self.at_keyword("const") {
            let decl = self.parse_variable_declaration()?;
            match decl {
                ASTNode::VariableDeclaration(v) => {
                    return Ok(ASTNode::ExportNamedDeclaration(ExportNamedDeclaration {
                        declaration: Some(ExportDeclaration::Variable(v)),
                        specifiers: vec![],
                        source: None,
                    }));
                }
                _ => unreachable!(),
            }
        }
        if self.at_keyword("function") {
            self.advance();
            let generator = self.at_punct("*");
            if generator {
                self.advance();
            }
            let func = self.parse_function_declaration_like(false, generator)?;
            return Ok(ASTNode::ExportNamedDeclaration(ExportNamedDeclaration {
                declaration: Some(ExportDeclaration::Function(func)),
                specifiers: vec![],
                source: None,
            }));
        }
        if self.at_keyword("async") {
            self.advance();
            self.expect("function")?;
            let generator = self.at_punct("*");
            if generator {
                self.advance();
            }
            let func = self.parse_function_declaration_like(true, generator)?;
            return Ok(ASTNode::ExportNamedDeclaration(ExportNamedDeclaration {
                declaration: Some(ExportDeclaration::Function(func)),
                specifiers: vec![],
                source: None,
            }));
        }
        if self.at_keyword("class") {
            self.advance();
            let cls = self.parse_class_declaration_inner()?;
            return Ok(ASTNode::ExportNamedDeclaration(ExportNamedDeclaration {
                declaration: Some(ExportDeclaration::Class(cls)),
                specifiers: vec![],
                source: None,
            }));
        }

        Err(self.error("expected export declaration"))
    }

    fn parse_function_declaration_like(
        &mut self,
        is_async: bool,
        generator: bool,
    ) -> Result<FunctionDeclaration, String> {
        let name = if matches!(
            self.cur_type(),
            Some(TokenType::Identifier) | Some(TokenType::Keyword)
        ) && !self.at_punct("(")
        {
            let n = self.cur_value().to_string();
            self.advance();
            n
        } else {
            String::new()
        };
        self.expect("(")?;
        let params = self.parse_parameter_list()?;
        self.expect(")")?;
        let body = self.parse_block_body()?;
        Ok(FunctionDeclaration {
            name,
            params,
            body,
            generator,
            is_async,
        })
    }

    fn parse_class_declaration_inner(&mut self) -> Result<ClassDeclaration, String> {
        let name = self.cur_value().to_string();
        self.advance();
        let super_class = if self.at_keyword("extends") {
            self.advance();
            Some(Box::new(self.parse_assignment_expression()?))
        } else {
            None
        };
        self.expect("{")?;
        let body = self.parse_class_body()?;
        self.expect("}")?;
        Ok(ClassDeclaration {
            name,
            super_class,
            body,
        })
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        let expr = self.parse_assignment_expression()?;
        if self.at_punct(",") {
            let mut exprs = vec![expr];
            while self.at_punct(",") {
                self.advance();
                exprs.push(self.parse_assignment_expression()?);
            }
            return Ok(Expression::SequenceExpression(SequenceExpression {
                expressions: exprs,
            }));
        }
        Ok(expr)
    }

    fn parse_assignment_expression(&mut self) -> Result<Expression, String> {
        if self.at_keyword("async") && self.peek_at("function") {
            self.advance();
            self.expect("function")?;
            let generator = self.at_punct("*");
            if generator {
                self.advance();
            }
            return self.parse_function_expression(true, generator);
        }

        if self.at_keyword("async") {
            if self.peek_at("(") || (self.peek_value().map_or(false, |v| v != "function")) {
                let saved_pos = self.lexer.pos();
                let saved_kind = self.lexer.last_token_kind();
                self.advance();
                if self.at_keyword("function") {
                    self.lexer.set_pos(saved_pos);
                    self.lexer.set_last_token_kind(saved_kind);
                    self.current = self.lexer.next_token();
                    self.peek_token = self.lexer.next_token();
                } else if let Some(arrow) = self.try_parse_arrow_function(true)? {
                    return Ok(arrow);
                } else {
                    self.lexer.set_pos(saved_pos);
                    self.lexer.set_last_token_kind(saved_kind);
                    self.current = self.lexer.next_token();
                    self.peek_token = self.lexer.next_token();
                }
            }
        }

        if self.at_punct("(") || (self.is_identifier_name() && self.peek_at("=>")) {
            if let Some(arrow) = self.try_parse_arrow_function(false)? {
                return Ok(arrow);
            }
        }

        let expr = self.parse_conditional_expression()?;

        let assign_ops = [
            ("=", AssignOp::Assign),
            ("+=", AssignOp::AddAssign),
            ("-=", AssignOp::SubAssign),
            ("*=", AssignOp::MulAssign),
            ("/=", AssignOp::DivAssign),
            ("%=", AssignOp::ModAssign),
            ("**=", AssignOp::PowAssign),
            ("&=", AssignOp::BitAndAssign),
            ("|=", AssignOp::BitOrAssign),
            ("^=", AssignOp::BitXorAssign),
            ("<<=", AssignOp::ShlAssign),
            (">>=", AssignOp::ShrAssign),
            (">>>=", AssignOp::UShrAssign),
            ("&&=", AssignOp::LogicalAndAssign),
            ("||=", AssignOp::LogicalOrAssign),
            ("??=", AssignOp::NullishAssign),
        ];
        for (op_str, op) in &assign_ops {
            if self.at_punct(op_str) {
                self.advance();
                let right = self.parse_assignment_expression()?;
                let left = expr_to_target(expr)?;
                return Ok(Expression::AssignmentExpression(AssignmentExpression {
                    op: op.clone(),
                    left,
                    right: Box::new(right),
                }));
            }
        }

        Ok(expr)
    }

    fn try_parse_arrow_function(&mut self, is_async: bool) -> Result<Option<Expression>, String> {
        let saved_pos = self.lexer.pos();
        let saved_kind = self.lexer.last_token_kind();
        let saved_cur = self.current.clone();
        let saved_peek = self.peek_token.clone();

        let params = if self.at_punct("(") {
            self.advance();
            let p = match self.parse_parameter_list() {
                Ok(p) => p,
                Err(_) => {
                    self.restore(saved_pos, saved_kind, saved_cur, saved_peek);
                    return Ok(None);
                }
            };
            if !self.at_punct(")") {
                self.restore(saved_pos, saved_kind, saved_cur, saved_peek);
                return Ok(None);
            }
            self.advance();
            p
        } else if self.is_identifier_name() {
            let name = self.cur_value().to_string();
            self.advance();
            vec![Parameter::Identifier(name)]
        } else {
            self.restore(saved_pos, saved_kind, saved_cur, saved_peek);
            return Ok(None);
        };

        if !self.at_punct("=>") {
            self.restore(saved_pos, saved_kind, saved_cur, saved_peek);
            return Ok(None);
        }
        self.advance();

        let body = if self.at_punct("{") {
            ArrowBody::Block(self.parse_block_body()?)
        } else {
            ArrowBody::Expression(Box::new(self.parse_assignment_expression()?))
        };

        Ok(Some(Expression::ArrowFunction(ArrowFunction {
            params,
            body,
            is_async,
        })))
    }

    fn restore(
        &mut self,
        pos: usize,
        kind: crate::compiler::lexer::LastTokenKind,
        cur: Option<Token>,
        peek: Option<Token>,
    ) {
        self.lexer.set_pos(pos);
        self.lexer.set_last_token_kind(kind);
        self.current = cur;
        self.peek_token = peek;
    }

    fn parse_conditional_expression(&mut self) -> Result<Expression, String> {
        let expr = self.parse_logical_or()?;
        if self.at_punct("?") {
            self.advance();
            let consequent = self.parse_assignment_expression()?;
            self.expect(":")?;
            let alternate = self.parse_assignment_expression()?;
            return Ok(Expression::ConditionalExpression(ConditionalExpression {
                test: Box::new(expr),
                consequent: Box::new(consequent),
                alternate: Box::new(alternate),
            }));
        }
        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_logical_and()?;
        while self.at_punct("||") || self.at_punct("??") {
            let op = if self.at_punct("||") {
                self.advance();
                LogicalOp::Or
            } else {
                self.advance();
                LogicalOp::NullishCoalescing
            };
            let right = self.parse_logical_and()?;
            left = Expression::LogicalExpression(LogicalExpression {
                op,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_bitwise_or()?;
        while self.at_punct("&&") {
            self.advance();
            let right = self.parse_bitwise_or()?;
            left = Expression::LogicalExpression(LogicalExpression {
                op: LogicalOp::And,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_bitwise_xor()?;
        while self.at_punct("|") {
            self.advance();
            let right = self.parse_bitwise_xor()?;
            left = Expression::BinaryExpression(BinaryExpression {
                op: BinaryOp::BitOr,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_bitwise_and()?;
        while self.at_punct("^") {
            self.advance();
            let right = self.parse_bitwise_and()?;
            left = Expression::BinaryExpression(BinaryExpression {
                op: BinaryOp::BitXor,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_equality()?;
        while self.at_punct("&") {
            self.advance();
            let right = self.parse_equality()?;
            left = Expression::BinaryExpression(BinaryExpression {
                op: BinaryOp::BitAnd,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_relational()?;
        loop {
            let op = if self.at_punct("==") {
                Some(BinaryOp::Eq)
            } else if self.at_punct("!=") {
                Some(BinaryOp::Neq)
            } else if self.at_punct("===") {
                Some(BinaryOp::StrictEq)
            } else if self.at_punct("!==") {
                Some(BinaryOp::StrictNeq)
            } else {
                None
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_relational()?;
                left = Expression::BinaryExpression(BinaryExpression {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                });
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<Expression, String> {
        if self.cur_type() == Some(TokenType::PrivateIdentifier) {
            let name = self.cur_value().to_string();
            self.advance();
            if self.at_keyword("in") {
                self.advance();
                let right = self.parse_shift()?;
                return Ok(Expression::BinaryExpression(BinaryExpression {
                    op: BinaryOp::In,
                    left: Box::new(Expression::PrivateIdentifier(name)),
                    right: Box::new(right),
                }));
            } else {
                return Err(self.error("Unexpected private identifier"));
            }
        }

        let mut left = self.parse_shift()?;
        loop {
            let op = if self.at_punct("<=") {
                Some(BinaryOp::Lte)
            } else if self.at_punct(">=") {
                Some(BinaryOp::Gte)
            } else if self.at_punct("<") {
                Some(BinaryOp::Lt)
            } else if self.at_punct(">") {
                Some(BinaryOp::Gt)
            } else if self.allow_in && self.at_keyword("in") {
                Some(BinaryOp::In)
            } else if self.at_keyword("instanceof") {
                Some(BinaryOp::InstanceOf)
            } else {
                None
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_shift()?;
                left = Expression::BinaryExpression(BinaryExpression {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                });
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_additive()?;
        loop {
            let op = if self.at_punct(">>>") {
                Some(BinaryOp::UShr)
            } else if self.at_punct(">>") {
                Some(BinaryOp::Shr)
            } else if self.at_punct("<<") {
                Some(BinaryOp::Shl)
            } else {
                None
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_additive()?;
                left = Expression::BinaryExpression(BinaryExpression {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                });
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = if self.at_punct("+") {
                Some(BinaryOp::Add)
            } else if self.at_punct("-") {
                Some(BinaryOp::Sub)
            } else {
                None
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_multiplicative()?;
                left = Expression::BinaryExpression(BinaryExpression {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                });
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_exponentiation()?;
        loop {
            let op = if self.at_punct("*") {
                Some(BinaryOp::Mul)
            } else if self.at_punct("/") {
                Some(BinaryOp::Div)
            } else if self.at_punct("%") {
                Some(BinaryOp::Mod)
            } else {
                None
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_exponentiation()?;
                left = Expression::BinaryExpression(BinaryExpression {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                });
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_exponentiation(&mut self) -> Result<Expression, String> {
        let left = self.parse_unary()?;
        if self.at_punct("**") {
            self.advance();
            let right = self.parse_exponentiation()?;
            return Ok(Expression::BinaryExpression(BinaryExpression {
                op: BinaryOp::Pow,
                left: Box::new(left),
                right: Box::new(right),
            }));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, String> {
        if self.at_punct("!") {
            self.advance();
            let arg = self.parse_unary()?;
            return Ok(Expression::UnaryExpression(UnaryExpression {
                op: UnaryOp::Not,
                argument: Box::new(arg),
                prefix: true,
            }));
        }

        if self.at_punct("~") {
            self.advance();
            let arg = self.parse_unary()?;
            return Ok(Expression::UnaryExpression(UnaryExpression {
                op: UnaryOp::BitNot,
                argument: Box::new(arg),
                prefix: true,
            }));
        }

        if self.at_punct("-") {
            self.advance();
            let arg = self.parse_unary()?;
            return Ok(Expression::UnaryExpression(UnaryExpression {
                op: UnaryOp::Minus,
                argument: Box::new(arg),
                prefix: true,
            }));
        }

        if self.at_punct("+") && self.cur_type() == Some(TokenType::Punctuator) {
            self.advance();
            let arg = self.parse_unary()?;
            return Ok(Expression::UnaryExpression(UnaryExpression {
                op: UnaryOp::Plus,
                argument: Box::new(arg),
                prefix: true,
            }));
        }
        if self.at_keyword("typeof") {
            self.advance();
            let arg = self.parse_unary()?;
            return Ok(Expression::UnaryExpression(UnaryExpression {
                op: UnaryOp::TypeOf,
                argument: Box::new(arg),
                prefix: true,
            }));
        }
        if self.at_keyword("void") {
            self.advance();
            let arg = self.parse_unary()?;
            return Ok(Expression::UnaryExpression(UnaryExpression {
                op: UnaryOp::Void,
                argument: Box::new(arg),
                prefix: true,
            }));
        }
        if self.at_keyword("delete") {
            self.advance();
            let arg = self.parse_unary()?;
            return Ok(Expression::UnaryExpression(UnaryExpression {
                op: UnaryOp::Delete,
                argument: Box::new(arg),
                prefix: true,
            }));
        }

        if self.at_punct("++") {
            self.advance();
            let arg = self.parse_unary()?;
            return Ok(Expression::UpdateExpression(UpdateExpression {
                op: UpdateOp::Increment,
                argument: Box::new(arg),
                prefix: true,
            }));
        }
        if self.at_punct("--") {
            self.advance();
            let arg = self.parse_unary()?;
            return Ok(Expression::UpdateExpression(UpdateExpression {
                op: UpdateOp::Decrement,
                argument: Box::new(arg),
                prefix: true,
            }));
        }

        if self.at_keyword("await") {
            self.advance();
            let arg = self.parse_unary()?;
            return Ok(Expression::AwaitExpression(AwaitExpression {
                argument: Box::new(arg),
            }));
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expression, String> {
        let expr = self.parse_member_expression()?;

        if self.at_punct("++") && !self.has_line_terminator() {
            self.advance();
            return Ok(Expression::UpdateExpression(UpdateExpression {
                op: UpdateOp::Increment,
                argument: Box::new(expr),
                prefix: false,
            }));
        }
        if self.at_punct("--") && !self.has_line_terminator() {
            self.advance();
            return Ok(Expression::UpdateExpression(UpdateExpression {
                op: UpdateOp::Decrement,
                argument: Box::new(expr),
                prefix: false,
            }));
        }

        Ok(expr)
    }

    fn parse_member_expression(&mut self) -> Result<Expression, String> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.at_punct(".") {
                self.advance();

                if self.cur_type() == Some(TokenType::PrivateIdentifier) {
                    let name = self.cur_value().to_string();
                    self.advance();
                    expr = Expression::MemberExpression(MemberExpression {
                        object: Box::new(expr),
                        property: MemberProperty::PrivateIdentifier(name),
                        computed: false,
                    });
                } else {
                    let name = self.cur_value().to_string();
                    self.advance();
                    expr = Expression::MemberExpression(MemberExpression {
                        object: Box::new(expr),
                        property: MemberProperty::Identifier(name),
                        computed: false,
                    });
                }
            } else if self.at_punct("?.") {
                self.advance();
                if self.at_punct("[") {
                    self.advance();
                    let prop = self.parse_expression()?;
                    self.expect("]")?;
                    expr = Expression::OptionalMemberExpression(OptionalMemberExpression {
                        object: Box::new(expr),
                        property: MemberProperty::Computed(Box::new(prop)),
                        computed: true,
                        optional: true,
                    });
                } else if self.at_punct("(") {
                    let args = self.parse_arguments()?;
                    expr = Expression::OptionalCallExpression(OptionalCallExpression {
                        callee: Box::new(expr),
                        arguments: args,
                        optional: true,
                    });
                } else {
                    let name = self.cur_value().to_string();
                    self.advance();
                    expr = Expression::OptionalMemberExpression(OptionalMemberExpression {
                        object: Box::new(expr),
                        property: MemberProperty::Identifier(name),
                        computed: false,
                        optional: true,
                    });
                }
            } else if self.at_punct("[") {
                self.advance();
                let prop = self.parse_expression()?;
                self.expect("]")?;
                expr = Expression::MemberExpression(MemberExpression {
                    object: Box::new(expr),
                    property: MemberProperty::Computed(Box::new(prop)),
                    computed: true,
                });
            } else if self.at_punct("(") {
                let args = self.parse_arguments()?;
                expr = Expression::CallExpression(CallExpression {
                    callee: Box::new(expr),
                    arguments: args,
                });
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_arguments(&mut self) -> Result<Vec<Argument>, String> {
        self.expect("(")?;
        let mut args = Vec::new();
        while !self.at_punct(")") && !self.is_eof() {
            if self.at_punct("...") {
                self.advance();
                args.push(Argument::Spread(self.parse_assignment_expression()?));
            } else {
                args.push(Argument::Expression(self.parse_assignment_expression()?));
            }
            if self.at_punct(",") {
                self.advance();
            }
        }
        self.expect(")")?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        if self.at_punct("(") {
            self.advance();
            let expr = self.parse_expression()?;
            self.expect(")")?;
            return Ok(expr);
        }

        if self.at_keyword("new") {
            return self.parse_new_expression();
        }

        if self.at_keyword("import") {
            if self.peek_at(".") {
                self.advance();
                self.expect(".")?;
                self.expect_keyword("meta")?;
                return Ok(Expression::MetaProperty(MetaProperty {
                    meta: "import".to_string(),
                    property: "meta".to_string(),
                }));
            }
            if self.peek_at("(") {
                self.advance();
                self.expect("(")?;
                let arg = self.parse_assignment_expression()?;
                self.expect(")")?;

                return Ok(Expression::CallExpression(CallExpression {
                    callee: Box::new(Expression::Identifier(Identifier {
                        name: "import".to_string(),
                    })),
                    arguments: vec![Argument::Expression(arg)],
                }));
            }
        }

        if self.cur_type() == Some(TokenType::Number) {
            let raw = self.cur_value().to_string();
            let lit = parse_number_literal(&raw);
            self.advance();
            return Ok(match lit {
                NumberLit::Normal(v) => Expression::Literal(Literal::Number(v)),
                NumberLit::LegacyOctal(v) => Expression::Literal(Literal::LegacyOctal(v)),
            });
        }

        if self.cur_type() == Some(TokenType::BigInt) {
            let val = self.cur_value().to_string();
            self.advance();
            return Ok(Expression::Literal(Literal::BigInt(val)));
        }

        if self.cur_type() == Some(TokenType::String) {
            let val = self.cur_value().to_string();
            self.advance();
            let has_escape = self.lexer.last_string_had_escape;
            return Ok(Expression::Literal(Literal::String(val, has_escape)));
        }

        if self.at_keyword("true") {
            self.advance();
            return Ok(Expression::Literal(Literal::Boolean(true)));
        }
        if self.at_keyword("false") {
            self.advance();
            return Ok(Expression::Literal(Literal::Boolean(false)));
        }
        if self.at_keyword("null") {
            self.advance();
            return Ok(Expression::Literal(Literal::Null));
        }
        if self.at_keyword("undefined")
            || (self.cur_type() == Some(TokenType::Identifier) && self.cur_value() == "undefined")
        {
            self.advance();
            return Ok(Expression::Literal(Literal::Undefined));
        }
        if self.cur_type() == Some(TokenType::Identifier) && self.cur_value() == "NaN" {
            self.advance();
            return Ok(Expression::Literal(Literal::Number(f64::NAN)));
        }
        if self.cur_type() == Some(TokenType::Identifier) && self.cur_value() == "Infinity" {
            self.advance();
            return Ok(Expression::Literal(Literal::Number(f64::INFINITY)));
        }
        if self.at_keyword("this") {
            self.advance();
            return Ok(Expression::This);
        }
        if self.at_keyword("super") {
            self.advance();
            return Ok(Expression::Super);
        }

        if self.cur_type() == Some(TokenType::Regex) {
            let val = self.cur_value().to_string();
            self.advance();
            let (pattern, flags) = parse_regex_value(&val);
            return Ok(Expression::RegExpLiteral(RegExpLiteral { pattern, flags }));
        }

        if self.at_keyword("function") {
            self.advance();
            let generator = self.at_punct("*");
            if generator {
                self.advance();
            }
            return self.parse_function_expression(false, generator);
        }

        if self.at_keyword("class") {
            return self.parse_class_expression();
        }

        if self.at_punct("[") {
            return self.parse_array_expression();
        }

        if self.at_punct("{") {
            return self.parse_object_expression();
        }

        if self.at_template_start() {
            return self.parse_template_literal();
        }

        if self.at_keyword("yield") {
            self.advance();
            let delegate = if self.at_punct("*") {
                self.advance();
                true
            } else {
                false
            };
            let argument =
                if self.is_eof() || self.at_punct(";") || self.at_punct("}") || self.at(")") {
                    None
                } else {
                    Some(Box::new(self.parse_assignment_expression()?))
                };
            return Ok(Expression::YieldExpression(YieldExpression {
                argument,
                delegate,
            }));
        }

        if self.cur_type() == Some(TokenType::Identifier)
            || self.cur_type() == Some(TokenType::Keyword)
        {
            let name = self.cur_value().to_string();
            match name.as_str() {
                "NaN" => {
                    self.advance();
                    return Ok(Expression::Literal(Literal::Number(f64::NAN)));
                }
                "Infinity" => {
                    self.advance();
                    return Ok(Expression::Literal(Literal::Number(f64::INFINITY)));
                }
                _ => {
                    self.advance();
                    return Ok(Expression::Identifier(Identifier { name }));
                }
            }
        }

        Err(self.error(&format!("Unexpected token: '{}'", self.cur_value())))
    }

    fn parse_new_expression(&mut self) -> Result<Expression, String> {
        self.expect("new")?;

        let callee = if self.at_keyword("new") {
            Box::new(self.parse_new_expression()?)
        } else {
            Box::new(self.parse_primary()?)
        };

        let mut callee = callee;
        loop {
            if self.at_punct(".") {
                self.advance();
                let name = self.cur_value().to_string();
                self.advance();
                callee = Box::new(Expression::MemberExpression(MemberExpression {
                    object: callee,
                    property: MemberProperty::Identifier(name),
                    computed: false,
                }));
            } else if self.at_punct("[") {
                self.advance();
                let prop = self.parse_expression()?;
                self.expect("]")?;
                callee = Box::new(Expression::MemberExpression(MemberExpression {
                    object: callee,
                    property: MemberProperty::Computed(Box::new(prop)),
                    computed: true,
                }));
            } else {
                break;
            }
        }

        let arguments = if self.at_punct("(") {
            self.parse_arguments()?
        } else {
            vec![]
        };

        Ok(Expression::NewExpression(NewExpression {
            callee,
            arguments,
        }))
    }

    fn parse_array_expression(&mut self) -> Result<Expression, String> {
        self.expect("[")?;
        let mut elements = Vec::new();
        while !self.at_punct("]") && !self.is_eof() {
            if self.at_punct(",") {
                elements.push(None);
                self.advance();
                continue;
            }
            if self.at_punct("...") {
                self.advance();
                elements.push(Some(ArrayElement::Spread(
                    self.parse_assignment_expression()?,
                )));
            } else {
                elements.push(Some(ArrayElement::Expression(
                    self.parse_assignment_expression()?,
                )));
            }
            if self.at_punct(",") {
                self.advance();
            }
        }
        self.expect("]")?;
        Ok(Expression::ArrayExpression(ArrayExpression { elements }))
    }

    fn parse_object_expression(&mut self) -> Result<Expression, String> {
        self.expect("{")?;
        let mut properties = Vec::new();
        while !self.at_punct("}") && !self.is_eof() {
            if self.at_punct("...") {
                self.advance();
                properties.push(Property::SpreadElement(self.parse_assignment_expression()?));
                if self.at_punct(",") {
                    self.advance();
                }
                continue;
            }

            let mut is_async = false;
            if self.at_keyword("async")
                && !self.peek_at(";")
                && !self.peek_at(":")
                && !self.peek_at(",")
                && !self.peek_at("}")
                && !self.peek_at("=")
            {
                self.advance();
                is_async = true;
            }

            let mut is_generator = false;
            if self.at_punct("*") {
                self.advance();
                is_generator = true;
            }

            let (key, computed, shorthand) = self.parse_property_key()?;

            if self.at_punct("(") && !shorthand {
                self.expect("(")?;
                let params = self.parse_parameter_list()?;
                self.expect(")")?;
                let body = self.parse_block_body()?;
                properties.push(Property::Property {
                    key,
                    value: Box::new(Expression::FunctionExpression(FunctionExpression {
                        name: None,
                        params,
                        body,
                        generator: is_generator,
                        is_async,
                    })),
                    computed,
                    shorthand: false,
                    method: true,
                    getter: false,
                    setter: false,
                });
                if self.at_punct(",") {
                    self.advance();
                }
                continue;
            }

            if !computed {
                let accessor = match &key {
                    PropertyKey::Identifier(name) if name == "get" || name == "set" => {
                        Some(name.clone())
                    }
                    _ => None,
                };

                if let Some(accessor_kind) = accessor {
                    if self.at_punct(":") {
                    } else {
                        let (acc_key, _acc_computed, _acc_shorthand) = self.parse_property_key()?;
                        if self.at_punct("(") {
                            self.expect("(")?;
                            let params = self.parse_parameter_list()?;
                            self.expect(")")?;
                            let body = self.parse_block_body()?;

                            if accessor_kind == "get" && !params.is_empty() {
                                return Err(self.error("getter must not have parameters"));
                            }
                            if accessor_kind == "set" && params.len() != 1 {
                                return Err(self.error("setter must have exactly one parameter"));
                            }

                            let is_getter = accessor_kind == "get";
                            properties.push(Property::Property {
                                key: acc_key,
                                value: Box::new(Expression::FunctionExpression(
                                    FunctionExpression {
                                        name: None,
                                        params,
                                        body,
                                        generator: false,
                                        is_async: false,
                                    },
                                )),
                                computed: false,
                                shorthand: false,
                                method: true,
                                getter: is_getter,
                                setter: !is_getter,
                            });

                            if self.at_punct(",") {
                                self.advance();
                            }
                            continue;
                        }
                    }
                }
            }

            if self.at_punct(":") {
                self.advance();
                let value = self.parse_assignment_expression()?;
                properties.push(Property::Property {
                    key,
                    value: Box::new(value),
                    computed,
                    shorthand: false,
                    method: false,
                    getter: false,
                    setter: false,
                });
            } else if shorthand {
                let name = match &key {
                    PropertyKey::Identifier(n) => n.clone(),
                    _ => return Err(self.error("invalid shorthand property")),
                };
                properties.push(Property::Property {
                    key,
                    value: Box::new(Expression::Identifier(Identifier { name })),
                    computed: false,
                    shorthand: true,
                    method: false,
                    getter: false,
                    setter: false,
                });
            } else {
                return Err(self.error("expected ':' or '(' in object literal"));
            }
            if self.at_punct(",") {
                self.advance();
            }
        }
        self.expect("}")?;
        Ok(Expression::ObjectExpression(ObjectExpression {
            properties,
        }))
    }

    fn parse_property_key(&mut self) -> Result<(PropertyKey, bool, bool), String> {
        if self.at_punct("[") {
            self.advance();
            let expr = self.parse_assignment_expression()?;
            self.expect("]")?;
            return Ok((PropertyKey::Computed(Box::new(expr)), true, false));
        }
        if self.cur_type() == Some(TokenType::PrivateIdentifier) {
            let name = self.cur_value().to_string();
            self.advance();
            return Ok((PropertyKey::PrivateIdentifier(name), false, false));
        }
        if self.cur_type() == Some(TokenType::String) {
            let val = self.cur_value().to_string();
            self.advance();
            let has_escape = self.lexer.last_string_had_escape;
            return Ok((
                PropertyKey::Literal(Literal::String(val, has_escape)),
                false,
                false,
            ));
        }
        if self.cur_type() == Some(TokenType::Number) {
            let raw = self.cur_value().to_string();
            let lit = parse_number_literal(&raw);
            self.advance();
            return Ok(match lit {
                NumberLit::Normal(v) => (PropertyKey::Literal(Literal::Number(v)), false, false),
                NumberLit::LegacyOctal(v) => {
                    (PropertyKey::Literal(Literal::LegacyOctal(v)), false, false)
                }
            });
        }
        if self.is_identifier_name()
            || matches!(
                self.cur_value(),
                "true" | "false" | "null" | "undefined" | "NaN" | "Infinity"
            )
        {
            let name = self.cur_value().to_string();
            self.advance();

            let shorthand = self.at_punct(":") || self.at_punct(",") || self.at_punct("}");
            return Ok((PropertyKey::Identifier(name), false, shorthand));
        }
        Err(self.error("expected property key"))
    }

    fn parse_template_literal(&mut self) -> Result<Expression, String> {
        let mut quasis = Vec::new();
        let mut expressions = Vec::new();

        let tok = self
            .current
            .take()
            .ok_or_else(|| self.error("Expected template literal"))?;

        match tok.token_type {
            TokenType::NoSubstitutionTemplate => {
                quasis.push(TemplateElement {
                    value: tok.value,
                    tail: true,
                });

                self.current = self.peek_token.take();
                self.saved_pos = self.lexer.pos;
                self.saved_line = self.lexer.line;
                self.saved_col = self.lexer.column;
                self.peek_token = self.lexer.next_token();
                return Ok(Expression::TemplateLiteral(TemplateLiteral {
                    quasis,
                    expressions,
                }));
            }
            TokenType::TemplateHead => {
                quasis.push(TemplateElement {
                    value: tok.value,
                    tail: false,
                });

                self.current = self.peek_token.take();
                self.saved_pos = self.lexer.pos;
                self.saved_line = self.lexer.line;
                self.saved_col = self.lexer.column;
                self.peek_token = self.lexer.next_token();
            }
            _ => return Err(self.error("Expected template literal")),
        }

        loop {
            let expr = self.parse_expression()?;
            expressions.push(expr);

            if !self.at("}") {
                return Err(self.error("Expected '}' after template expression"));
            }

            self.lexer.pos = self.saved_pos;
            self.lexer.line = self.saved_line;
            self.lexer.column = self.saved_col;
            self.peek_token = None;

            let segment = self
                .lexer
                .scan_template_continuation()
                .ok_or_else(|| self.error("Unterminated template literal"))?;

            match segment.token_type {
                TokenType::TemplateTail => {
                    quasis.push(TemplateElement {
                        value: segment.value,
                        tail: true,
                    });

                    self.current = self.lexer.next_token();
                    self.saved_pos = self.lexer.pos;
                    self.saved_line = self.lexer.line;
                    self.saved_col = self.lexer.column;
                    self.peek_token = self.lexer.next_token();
                    break;
                }
                TokenType::TemplateMiddle => {
                    quasis.push(TemplateElement {
                        value: segment.value,
                        tail: false,
                    });

                    self.current = self.lexer.next_token();
                    self.saved_pos = self.lexer.pos;
                    self.saved_line = self.lexer.line;
                    self.saved_col = self.lexer.column;
                    self.peek_token = self.lexer.next_token();
                }
                _ => return Err(self.error("Unexpected token in template literal")),
            }
        }

        Ok(Expression::TemplateLiteral(TemplateLiteral {
            quasis,
            expressions,
        }))
    }
}

fn binding_to_target(bp: BindingPattern) -> AssignmentTarget {
    match bp {
        BindingPattern::Identifier(name) => AssignmentTarget::Identifier(name),
        BindingPattern::ArrayPattern(p) => AssignmentTarget::ArrayPattern(p),
        BindingPattern::ObjectPattern(p) => AssignmentTarget::ObjectPattern(p),
        BindingPattern::AssignmentPattern(p) => AssignmentTarget::AssignmentPattern(p),
    }
}

fn expr_to_target(expr: Expression) -> Result<AssignmentTarget, String> {
    match expr {
        Expression::Identifier(id) => Ok(AssignmentTarget::Identifier(id.name)),
        Expression::MemberExpression(m) => Ok(AssignmentTarget::MemberExpression(m)),
        Expression::ArrayPattern(p) => Ok(AssignmentTarget::ArrayPattern(p)),
        Expression::ObjectPattern(p) => Ok(AssignmentTarget::ObjectPattern(p)),
        Expression::ArrayExpression(arr) => {
            let mut elements = Vec::new();
            for elem in arr.elements {
                match elem {
                    Some(ArrayElement::Expression(e)) => {
                        elements.push(Some(PatternElement::Pattern(expr_to_target(e)?)));
                    }
                    Some(ArrayElement::Spread(e)) => {
                        elements.push(Some(PatternElement::RestElement(RestElement {
                            argument: Box::new(expr_to_target(e)?),
                        })));
                    }
                    None => {
                        elements.push(None);
                    }
                }
            }
            Ok(AssignmentTarget::ArrayPattern(ArrayPattern { elements }))
        }
        Expression::ObjectExpression(obj) => {
            let mut properties = Vec::new();
            for prop in obj.properties {
                match prop {
                    Property::SpreadElement(arg) => {
                        properties.push(ObjectPatternProperty::RestElement(RestElement {
                            argument: Box::new(expr_to_target(arg)?),
                        }));
                    }
                    Property::Property {
                        key,
                        value,
                        computed,
                        shorthand,
                        ..
                    } => {
                        if shorthand {
                            let name = match &key {
                                PropertyKey::Identifier(n) => n.clone(),
                                _ => {
                                    return Err(
                                        "invalid shorthand property in destructuring".to_string()
                                    );
                                }
                            };
                            properties.push(ObjectPatternProperty::Property {
                                key,
                                value: AssignmentTarget::Identifier(name),
                                computed: false,
                                shorthand: true,
                            });
                        } else {
                            let target = expr_to_target(*value)?;
                            properties.push(ObjectPatternProperty::Property {
                                key,
                                value: target,
                                computed,
                                shorthand: false,
                            });
                        }
                    }
                }
            }
            Ok(AssignmentTarget::ObjectPattern(ObjectPattern {
                properties,
            }))
        }
        Expression::AssignmentExpression(a) => {
            Ok(AssignmentTarget::AssignmentPattern(AssignmentPattern {
                left: Box::new(a.left),
                right: a.right,
            }))
        }
        _ => Err(format!("invalid assignment target: {:?}", expr)),
    }
}

fn parse_regex_value(value: &str) -> (String, String) {
    if let Some(idx) = value.rfind('/') {
        let pattern = value[0..idx].to_string();
        let flags = value[idx + 1..].to_string();
        (pattern, flags)
    } else {
        (value.to_string(), String::new())
    }
}

enum NumberLit {
    Normal(f64),
    LegacyOctal(i64),
}

fn parse_number_literal(raw: &str) -> NumberLit {
    if raw.starts_with("0x") || raw.starts_with("0X") {
        let clean: String = raw[2..].chars().filter(|&c| c != '_').collect();
        NumberLit::Normal(
            i64::from_str_radix(&clean, 16)
                .map(|n| n as f64)
                .unwrap_or(0.0),
        )
    } else if raw.starts_with("0o") || raw.starts_with("0O") {
        let clean: String = raw[2..].chars().filter(|&c| c != '_').collect();
        NumberLit::Normal(
            i64::from_str_radix(&clean, 8)
                .map(|n| n as f64)
                .unwrap_or(0.0),
        )
    } else if raw.starts_with("0b") || raw.starts_with("0B") {
        let clean: String = raw[2..].chars().filter(|&c| c != '_').collect();
        NumberLit::Normal(
            i64::from_str_radix(&clean, 2)
                .map(|n| n as f64)
                .unwrap_or(0.0),
        )
    } else if raw.len() > 1 && raw.as_bytes()[0] == b'0' && raw.as_bytes()[1].is_ascii_digit() {
        let chars: Vec<char> = raw.chars().collect();
        if chars[1] == '8' || chars[1] == '9' {
            let clean: String = raw.chars().filter(|&c| c != '_').collect();
            NumberLit::Normal(clean.parse::<f64>().unwrap_or(0.0))
        } else if chars[1..]
            .iter()
            .all(|&c| c == '_' || (c.is_ascii_digit() && c <= '7'))
        {
            let clean: String = raw[1..].chars().filter(|&c| c != '_').collect();
            NumberLit::LegacyOctal(i64::from_str_radix(&clean, 8).unwrap_or(0))
        } else {
            let clean: String = raw.chars().filter(|&c| c != '_').collect();
            NumberLit::Normal(clean.parse::<f64>().unwrap_or(0.0))
        }
    } else {
        let clean: String = raw.chars().filter(|&c| c != '_').collect();
        NumberLit::Normal(clean.parse::<f64>().unwrap_or(0.0))
    }
}
