//! Recursive descent parser for Tarqeem

use super::ast::*;
use super::precedence::Precedence;
use crate::error::{Diagnostic, Span};
use crate::lexer::{Lexer, Token, TokenKind};

/// The Tarqeem parser
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    /// Create a new parser from source code
    pub fn new(source: &str) -> Self {
        let mut lexer = Lexer::new(source);
        let tokens: Vec<Token> = lexer
            .tokenize()
            .into_iter()
            .filter(|t| !matches!(t.kind, TokenKind::Newline))
            .collect();

        Self { tokens, current: 0 }
    }

    /// Create a new parser from pre-lexed tokens
    pub fn from_tokens(tokens: Vec<Token>) -> Self {
        let tokens: Vec<Token> = tokens
            .into_iter()
            .filter(|t| !matches!(t.kind, TokenKind::Newline))
            .collect();
        Self { tokens, current: 0 }
    }

    /// Parse the entire program
    pub fn parse(&mut self) -> Result<Ast, Diagnostic> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            statements.push(self.parse_declaration()?);
        }

        Ok(Ast::new(statements))
    }

    // ============ Declaration Parsing ============

    fn parse_declaration(&mut self) -> Result<Stmt, Diagnostic> {
        let result = if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) {
            self.parse_var_declaration()
        } else if self.check(&TokenKind::Function) {
            self.parse_function_declaration(false)
        } else if self.check(&TokenKind::Async) {
            self.advance();
            self.parse_function_declaration(true)
        } else if self.check(&TokenKind::Class) {
            self.parse_class_declaration()
        } else if self.check(&TokenKind::Interface) {
            self.parse_interface_declaration()
        } else if self.check(&TokenKind::Import) {
            self.parse_import_statement()
        } else if self.check(&TokenKind::Export) {
            self.parse_export_statement()
        } else {
            self.parse_statement()
        };

        result
    }

    fn parse_var_declaration(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        let mutable = self.check(&TokenKind::Let);
        self.advance(); // consume 'let' or 'const'

        let name = self.expect_identifier("Expected variable name / متوقع اسم المتغير")?;

        let ty = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let init = if self.match_token(&TokenKind::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume_semicolon()?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(
            StmtKind::VarDecl {
                name,
                mutable,
                ty,
                init,
            },
            span,
        ))
    }

    fn parse_function_declaration(&mut self, is_async: bool) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.expect(&TokenKind::Function, "Expected 'function' / متوقع 'دالة'")?;

        let name = self.expect_identifier("Expected function name / متوقع اسم الدالة")?;

        self.expect(&TokenKind::LeftParen, "Expected '(' / متوقع '('")?;
        let params = self.parse_parameters()?;
        self.expect(&TokenKind::RightParen, "Expected ')' / متوقع ')'")?;

        let return_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(
            StmtKind::FuncDecl {
                name,
                params,
                return_type,
                body,
                is_async,
            },
            span,
        ))
    }

    fn parse_class_declaration(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'class'

        let name = self.expect_identifier("Expected class name / متوقع اسم الصنف")?;

        let extends = if self.check(&TokenKind::Extends) {
            self.advance();
            Some(self.expect_identifier("Expected superclass name / متوقع اسم الصنف الأب")?)
        } else {
            None
        };

        let mut implements = Vec::new();
        if self.check(&TokenKind::Implements) {
            self.advance();
            loop {
                implements
                    .push(self.expect_identifier("Expected interface name / متوقع اسم الواجهة")?);
                if !self.match_token(&TokenKind::Comma)
                    && !self.match_token(&TokenKind::ArabicComma)
                {
                    break;
                }
            }
        }

        self.expect(&TokenKind::LeftBrace, "Expected '{' / متوقع '{'")?;
        let members = self.parse_class_members()?;
        self.expect(&TokenKind::RightBrace, "Expected '}' / متوقع '}'")?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(
            StmtKind::ClassDecl {
                name,
                extends,
                implements,
                members,
            },
            span,
        ))
    }

    fn parse_class_members(&mut self) -> Result<Vec<ClassMember>, Diagnostic> {
        let mut members = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let visibility = self.parse_visibility();
            let is_static = self.match_token(&TokenKind::Static);

            if self.check(&TokenKind::Constructor) {
                self.advance();
                self.expect(&TokenKind::LeftParen, "Expected '(' / متوقع '('")?;
                let params = self.parse_parameters()?;
                self.expect(&TokenKind::RightParen, "Expected ')' / متوقع ')'")?;
                let body = self.parse_block()?;
                members.push(ClassMember::Constructor { params, body });
            } else if self.check(&TokenKind::Function) || self.check(&TokenKind::Async) {
                let is_async = self.match_token(&TokenKind::Async);
                self.expect(&TokenKind::Function, "Expected 'function' / متوقع 'دالة'")?;
                let name = self.expect_identifier("Expected method name / متوقع اسم الدالة")?;
                self.expect(&TokenKind::LeftParen, "Expected '(' / متوقع '('")?;
                let params = self.parse_parameters()?;
                self.expect(&TokenKind::RightParen, "Expected ')' / متوقع ')'")?;

                let return_type = if self.match_token(&TokenKind::Arrow) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };

                let body = self.parse_block()?;

                members.push(ClassMember::Method {
                    visibility,
                    name,
                    params,
                    return_type,
                    body,
                    is_static,
                    is_async,
                });
            } else {
                // Field
                let name = self.expect_identifier("Expected field name / متوقع اسم الحقل")?;
                let ty = if self.match_token(&TokenKind::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };
                let init = if self.match_token(&TokenKind::Equal) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                self.consume_semicolon()?;

                members.push(ClassMember::Field {
                    visibility,
                    name,
                    ty,
                    init,
                    is_static,
                });
            }
        }

        Ok(members)
    }

    fn parse_visibility(&mut self) -> Visibility {
        if self.match_token(&TokenKind::Public) {
            Visibility::Public
        } else if self.match_token(&TokenKind::Private) {
            Visibility::Private
        } else if self.match_token(&TokenKind::Protected) {
            Visibility::Protected
        } else {
            Visibility::Public
        }
    }

    fn parse_interface_declaration(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'interface'

        let name = self.expect_identifier("Expected interface name / متوقع اسم الواجهة")?;

        self.expect(&TokenKind::LeftBrace, "Expected '{' / متوقع '{'")?;

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            self.expect(&TokenKind::Function, "Expected 'function' / متوقع 'دالة'")?;
            let method_name = self.expect_identifier("Expected method name / متوقع اسم الدالة")?;
            self.expect(&TokenKind::LeftParen, "Expected '(' / متوقع '('")?;
            let params = self.parse_parameters()?;
            self.expect(&TokenKind::RightParen, "Expected ')' / متوقع ')'")?;

            let return_type = if self.match_token(&TokenKind::Arrow) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };

            methods.push(MethodSignature {
                name: method_name,
                params,
                return_type,
            });
        }

        self.expect(&TokenKind::RightBrace, "Expected '}' / متوقع '}'")?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::InterfaceDecl { name, methods }, span))
    }

    fn parse_import_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'import'

        let items = if self.match_token(&TokenKind::Star) {
            self.expect(&TokenKind::As, "Expected 'as' / متوقع 'كـ'")?;
            let alias = self.expect_identifier("Expected alias / متوقع اسم مستعار")?;
            ImportItems::Wildcard(alias)
        } else if self.match_token(&TokenKind::LeftBrace) {
            let mut items = Vec::new();
            loop {
                let name = self.expect_identifier("Expected import name / متوقع اسم")?;
                let alias = if self.match_token(&TokenKind::As) {
                    Some(self.expect_identifier("Expected alias / متوقع اسم مستعار")?)
                } else {
                    None
                };
                items.push(ImportItem { name, alias });

                if !self.match_token(&TokenKind::Comma)
                    && !self.match_token(&TokenKind::ArabicComma)
                {
                    break;
                }
            }
            self.expect(&TokenKind::RightBrace, "Expected '}' / متوقع '}'")?;
            ImportItems::Named(items)
        } else {
            let name = self.expect_identifier("Expected default import / متوقع استيراد افتراضي")?;
            ImportItems::Default(name)
        };

        self.expect(&TokenKind::From, "Expected 'from' / متوقع 'من'")?;
        let from = self.expect_string("Expected module path / متوقع مسار الوحدة")?;

        self.consume_semicolon()?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::Import { items, from }, span))
    }

    fn parse_export_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'export'

        let stmt = self.parse_declaration()?;
        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::Export(Box::new(stmt)), span))
    }

    // ============ Statement Parsing ============

    fn parse_statement(&mut self) -> Result<Stmt, Diagnostic> {
        if self.check(&TokenKind::If) {
            self.parse_if_statement()
        } else if self.check(&TokenKind::While) {
            self.parse_while_statement()
        } else if self.check(&TokenKind::For) {
            self.parse_for_statement()
        } else if self.check(&TokenKind::Match) {
            self.parse_match_statement()
        } else if self.check(&TokenKind::Return) {
            self.parse_return_statement()
        } else if self.check(&TokenKind::Break) {
            self.parse_break_statement()
        } else if self.check(&TokenKind::Continue) {
            self.parse_continue_statement()
        } else if self.check(&TokenKind::Try) {
            self.parse_try_statement()
        } else if self.check(&TokenKind::Throw) {
            self.parse_throw_statement()
        } else if self.check(&TokenKind::LeftBrace) {
            let block = self.parse_block()?;
            Ok(Stmt::new(StmtKind::Block(block.clone()), block.span))
        } else {
            self.parse_expression_statement()
        }
    }

    fn parse_if_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'if'

        self.expect(&TokenKind::LeftParen, "Expected '(' / متوقع '('")?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "Expected ')' / متوقع ')'")?;

        let then_branch = self.parse_block()?;

        let else_branch = if self.match_token(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                // else if
                let else_if = self.parse_if_statement()?;
                Some(Block::new(vec![else_if], self.previous_span()))
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            },
            span,
        ))
    }

    fn parse_while_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'while'

        self.expect(&TokenKind::LeftParen, "Expected '(' / متوقع '('")?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "Expected ')' / متوقع ')'")?;

        let body = self.parse_block()?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::While { condition, body }, span))
    }

    fn parse_for_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'for'

        // Check for for-in loop
        if self.check_identifier() {
            let var_name = self.expect_identifier("")?;
            if self.check(&TokenKind::In) {
                self.advance();
                let iterable = self.parse_expression()?;
                let body = self.parse_block()?;

                let span = start.merge(&self.previous_span());
                return Ok(Stmt::new(
                    StmtKind::ForIn {
                        variable: var_name,
                        iterable,
                        body,
                    },
                    span,
                ));
            } else {
                // Backtrack - this is a regular for loop
                self.current -= 1;
            }
        }

        self.expect(&TokenKind::LeftParen, "Expected '(' / متوقع '('")?;

        // Parse init
        let init = if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::ArabicSemicolon) {
            None
        } else if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) {
            Some(Box::new(self.parse_var_declaration()?))
        } else {
            let expr = self.parse_expression()?;
            self.consume_semicolon()?;
            Some(Box::new(Stmt::new(
                StmtKind::Expr(expr),
                self.previous_span(),
            )))
        };

        // For var declarations, semicolon is already consumed
        if init.is_none() {
            self.consume_semicolon()?;
        }

        // Parse condition
        let condition =
            if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::ArabicSemicolon) {
                None
            } else {
                Some(self.parse_expression()?)
            };
        self.consume_semicolon()?;

        // Parse update
        let update = if self.check(&TokenKind::RightParen) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.expect(&TokenKind::RightParen, "Expected ')' / متوقع ')'")?;

        let body = self.parse_block()?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(
            StmtKind::For {
                init,
                condition,
                update,
                body,
            },
            span,
        ))
    }

    fn parse_match_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'match'

        self.expect(&TokenKind::LeftParen, "Expected '(' / متوقع '('")?;
        let expr = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "Expected ')' / متوقع ')'")?;

        self.expect(&TokenKind::LeftBrace, "Expected '{' / متوقع '{'")?;

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let arm_start = self.current_span();

            let is_default = self.match_token(&TokenKind::Default);
            let mut patterns = Vec::new();

            if is_default {
                // Default case - no patterns needed
            } else {
                self.expect(&TokenKind::Case, "Expected 'case' / متوقع 'حالة'")?;
                loop {
                    patterns.push(self.parse_expression()?);
                    if !self.match_token(&TokenKind::Comma)
                        && !self.match_token(&TokenKind::ArabicComma)
                    {
                        break;
                    }
                }
            }

            self.expect(&TokenKind::FatArrow, "Expected '=>' / متوقع '=>'")?;

            let body = if self.check(&TokenKind::LeftBrace) {
                self.parse_block()?
            } else {
                let expr = self.parse_expression()?;
                Block::new(
                    vec![Stmt::new(StmtKind::Expr(expr), self.previous_span())],
                    self.previous_span(),
                )
            };

            arms.push(MatchArm {
                patterns,
                body,
                span: arm_start.merge(&self.previous_span()),
            });
        }

        self.expect(&TokenKind::RightBrace, "Expected '}' / متوقع '}'")?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::Match { expr, arms }, span))
    }

    fn parse_return_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'return'

        let value = if self.check(&TokenKind::Semicolon)
            || self.check(&TokenKind::ArabicSemicolon)
            || self.check(&TokenKind::RightBrace)
        {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.consume_semicolon()?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::Return(value), span))
    }

    fn parse_break_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let span = self.current_span();
        self.advance(); // consume 'break'
        self.consume_semicolon()?;
        Ok(Stmt::new(StmtKind::Break, span))
    }

    fn parse_continue_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let span = self.current_span();
        self.advance(); // consume 'continue'
        self.consume_semicolon()?;
        Ok(Stmt::new(StmtKind::Continue, span))
    }

    fn parse_try_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'try'

        let body = self.parse_block()?;

        let catch = if self.match_token(&TokenKind::Catch) {
            self.expect(&TokenKind::LeftParen, "Expected '(' / متوقع '('")?;
            let param = self.expect_identifier("Expected error name / متوقع اسم الخطأ")?;
            self.expect(&TokenKind::RightParen, "Expected ')' / متوقع ')'")?;
            let catch_body = self.parse_block()?;
            Some(CatchClause {
                param,
                body: catch_body,
                span: self.previous_span(),
            })
        } else {
            None
        };

        let finally = if self.match_token(&TokenKind::Finally) {
            Some(self.parse_block()?)
        } else {
            None
        };

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(
            StmtKind::Try {
                body,
                catch,
                finally,
            },
            span,
        ))
    }

    fn parse_throw_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'throw'
        let expr = self.parse_expression()?;
        self.consume_semicolon()?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::Throw(expr), span))
    }

    fn parse_expression_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let expr = self.parse_expression()?;
        self.consume_semicolon()?;
        let span = expr.span;
        Ok(Stmt::new(StmtKind::Expr(expr), span))
    }

    fn parse_block(&mut self) -> Result<Block, Diagnostic> {
        let start = self.current_span();
        self.expect(&TokenKind::LeftBrace, "Expected '{' / متوقع '{'")?;

        let mut statements = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_declaration()?);
        }

        self.expect(&TokenKind::RightBrace, "Expected '}' / متوقع '}'")?;

        let span = start.merge(&self.previous_span());
        Ok(Block::new(statements, span))
    }

    // ============ Expression Parsing (Pratt Parser) ============

    fn parse_expression(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_prefix()?;

        while !self.is_at_end() {
            let op_prec = Precedence::of(&self.peek().kind);
            if precedence > op_prec {
                break;
            }

            left = self.parse_infix(left)?;
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, Diagnostic> {
        let token = self.advance();
        let span = token.span;

        match &token.kind {
            // Literals
            TokenKind::IntLiteral(n) => Ok(Expr::new(ExprKind::Literal(Literal::Int(*n)), span)),
            TokenKind::FloatLiteral(n) => {
                Ok(Expr::new(ExprKind::Literal(Literal::Float(*n)), span))
            }
            TokenKind::StringLiteral(s) => Ok(Expr::new(
                ExprKind::Literal(Literal::String(s.clone())),
                span,
            )),
            TokenKind::True => Ok(Expr::new(ExprKind::Literal(Literal::Bool(true)), span)),
            TokenKind::False => Ok(Expr::new(ExprKind::Literal(Literal::Bool(false)), span)),
            TokenKind::Null => Ok(Expr::new(ExprKind::Literal(Literal::Null), span)),

            // Identifiers
            TokenKind::Identifier(name) => Ok(Expr::new(ExprKind::Identifier(name.clone()), span)),

            // this/super
            TokenKind::This => Ok(Expr::new(ExprKind::This, span)),
            TokenKind::Super => Ok(Expr::new(ExprKind::Super, span)),

            // Grouping
            TokenKind::LeftParen => {
                let expr = self.parse_expression()?;
                self.expect(&TokenKind::RightParen, "Expected ')' / متوقع ')'")?;
                let end_span = self.previous_span();
                Ok(Expr::new(
                    ExprKind::Grouping(Box::new(expr)),
                    span.merge(&end_span),
                ))
            }

            // Array literal
            TokenKind::LeftBracket => {
                let mut elements = Vec::new();
                if !self.check(&TokenKind::RightBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.match_token(&TokenKind::Comma)
                            && !self.match_token(&TokenKind::ArabicComma)
                        {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::RightBracket, "Expected ']' / متوقع ']'")?;
                let end_span = self.previous_span();
                Ok(Expr::new(ExprKind::Array(elements), span.merge(&end_span)))
            }

            // Object literal
            TokenKind::LeftBrace => {
                let mut pairs = Vec::new();
                if !self.check(&TokenKind::RightBrace) {
                    loop {
                        let key = self.expect_identifier("Expected key / متوقع مفتاح")?;
                        self.expect(&TokenKind::Colon, "Expected ':' / متوقع ':'")?;
                        let value = self.parse_expression()?;
                        pairs.push((key, value));
                        if !self.match_token(&TokenKind::Comma)
                            && !self.match_token(&TokenKind::ArabicComma)
                        {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::RightBrace, "Expected '}' / متوقع '}'")?;
                let end_span = self.previous_span();
                Ok(Expr::new(ExprKind::Object(pairs), span.merge(&end_span)))
            }

            // Unary operators
            TokenKind::Minus => {
                let operand = self.parse_precedence(Precedence::Unary)?;
                let end_span = operand.span;
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::Neg,
                        operand: Box::new(operand),
                    },
                    span.merge(&end_span),
                ))
            }
            TokenKind::Bang => {
                let operand = self.parse_precedence(Precedence::Unary)?;
                let end_span = operand.span;
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::Not,
                        operand: Box::new(operand),
                    },
                    span.merge(&end_span),
                ))
            }
            TokenKind::PlusPlus => {
                let operand = self.parse_precedence(Precedence::Unary)?;
                let end_span = operand.span;
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::PreInc,
                        operand: Box::new(operand),
                    },
                    span.merge(&end_span),
                ))
            }
            TokenKind::MinusMinus => {
                let operand = self.parse_precedence(Precedence::Unary)?;
                let end_span = operand.span;
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::PreDec,
                        operand: Box::new(operand),
                    },
                    span.merge(&end_span),
                ))
            }

            // new expression: جديد ClassName(args)
            TokenKind::New => {
                // Parse class name at Primary level to avoid parsing args as a call
                let class = self.parse_precedence(Precedence::Primary)?;
                // Args must follow immediately with parentheses
                let args = if self.match_token(&TokenKind::LeftParen) {
                    let args = self.parse_arguments()?;
                    self.expect(&TokenKind::RightParen, "Expected ')' / متوقع ')'")?;
                    args
                } else {
                    Vec::new()
                };
                let end_span = self.previous_span();
                Ok(Expr::new(
                    ExprKind::New {
                        class: Box::new(class),
                        args,
                    },
                    span.merge(&end_span),
                ))
            }

            // await expression
            TokenKind::Await => {
                let expr = self.parse_precedence(Precedence::Unary)?;
                let end_span = expr.span;
                Ok(Expr::new(
                    ExprKind::Await(Box::new(expr)),
                    span.merge(&end_span),
                ))
            }

            _ => Err(Diagnostic::error(
                format!("Unexpected token: {:?}", token.kind),
                format!("رمز غير متوقع: {:?}", token.kind),
                span,
            )),
        }
    }

    fn parse_infix(&mut self, left: Expr) -> Result<Expr, Diagnostic> {
        let token = self.peek().clone();
        let op_prec = Precedence::of(&token.kind);

        match &token.kind {
            // Binary operators
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::StarStar
            | TokenKind::EqualEqual
            | TokenKind::BangEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::And
            | TokenKind::Or => {
                self.advance();
                let op = self.token_to_binary_op(&token.kind);
                let next_prec = if Precedence::is_right_associative(&token.kind) {
                    op_prec
                } else {
                    op_prec.next()
                };
                let right = self.parse_precedence(next_prec)?;
                let span = left.span.merge(&right.span);
                Ok(Expr::new(
                    ExprKind::Binary {
                        left: Box::new(left),
                        op,
                        right: Box::new(right),
                    },
                    span,
                ))
            }

            // Assignment
            TokenKind::Equal => {
                self.advance();
                let right = self.parse_precedence(Precedence::Assignment)?;
                let span = left.span.merge(&right.span);
                Ok(Expr::new(
                    ExprKind::Assignment {
                        target: Box::new(left),
                        value: Box::new(right),
                    },
                    span,
                ))
            }

            // Compound assignment
            TokenKind::PlusEqual
            | TokenKind::MinusEqual
            | TokenKind::StarEqual
            | TokenKind::SlashEqual
            | TokenKind::PercentEqual => {
                self.advance();
                let op = self.compound_to_binary_op(&token.kind);
                let right = self.parse_precedence(Precedence::Assignment)?;
                let span = left.span.merge(&right.span);
                Ok(Expr::new(
                    ExprKind::CompoundAssignment {
                        target: Box::new(left),
                        op,
                        value: Box::new(right),
                    },
                    span,
                ))
            }

            // Call
            TokenKind::LeftParen => {
                self.advance();
                let args = self.parse_arguments()?;
                self.expect(&TokenKind::RightParen, "Expected ')' / متوقع ')'")?;
                let span = left.span.merge(&self.previous_span());
                Ok(Expr::new(
                    ExprKind::Call {
                        callee: Box::new(left),
                        args,
                    },
                    span,
                ))
            }

            // Index
            TokenKind::LeftBracket => {
                self.advance();
                let index = self.parse_expression()?;
                self.expect(&TokenKind::RightBracket, "Expected ']' / متوقع ']'")?;
                let span = left.span.merge(&self.previous_span());
                Ok(Expr::new(
                    ExprKind::Index {
                        object: Box::new(left),
                        index: Box::new(index),
                    },
                    span,
                ))
            }

            // Member access
            TokenKind::Dot => {
                self.advance();
                let property =
                    self.expect_identifier("Expected property name / متوقع اسم الخاصية")?;
                let span = left.span.merge(&self.previous_span());
                Ok(Expr::new(
                    ExprKind::Member {
                        object: Box::new(left),
                        property,
                    },
                    span,
                ))
            }

            // Ternary
            TokenKind::Question => {
                self.advance();
                let then_expr = self.parse_expression()?;
                self.expect(&TokenKind::Colon, "Expected ':' / متوقع ':'")?;
                let else_expr = self.parse_precedence(Precedence::Ternary)?;
                let span = left.span.merge(&else_expr.span);
                Ok(Expr::new(
                    ExprKind::Ternary {
                        condition: Box::new(left),
                        then_expr: Box::new(then_expr),
                        else_expr: Box::new(else_expr),
                    },
                    span,
                ))
            }

            // Postfix increment/decrement
            TokenKind::PlusPlus => {
                self.advance();
                let span = left.span.merge(&self.previous_span());
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::PostInc,
                        operand: Box::new(left),
                    },
                    span,
                ))
            }
            TokenKind::MinusMinus => {
                self.advance();
                let span = left.span.merge(&self.previous_span());
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::PostDec,
                        operand: Box::new(left),
                    },
                    span,
                ))
            }

            _ => Ok(left),
        }
    }

    fn token_to_binary_op(&self, kind: &TokenKind) -> BinaryOp {
        match kind {
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Sub,
            TokenKind::Star => BinaryOp::Mul,
            TokenKind::Slash => BinaryOp::Div,
            TokenKind::Percent => BinaryOp::Mod,
            TokenKind::StarStar => BinaryOp::Pow,
            TokenKind::EqualEqual => BinaryOp::Eq,
            TokenKind::BangEqual => BinaryOp::NotEq,
            TokenKind::Less => BinaryOp::Lt,
            TokenKind::LessEqual => BinaryOp::LtEq,
            TokenKind::Greater => BinaryOp::Gt,
            TokenKind::GreaterEqual => BinaryOp::GtEq,
            TokenKind::And => BinaryOp::And,
            TokenKind::Or => BinaryOp::Or,
            _ => unreachable!(),
        }
    }

    fn compound_to_binary_op(&self, kind: &TokenKind) -> BinaryOp {
        match kind {
            TokenKind::PlusEqual => BinaryOp::Add,
            TokenKind::MinusEqual => BinaryOp::Sub,
            TokenKind::StarEqual => BinaryOp::Mul,
            TokenKind::SlashEqual => BinaryOp::Div,
            TokenKind::PercentEqual => BinaryOp::Mod,
            _ => unreachable!(),
        }
    }

    fn parse_arguments(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        let mut args = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                args.push(self.parse_expression()?);
                if !self.match_token(&TokenKind::Comma)
                    && !self.match_token(&TokenKind::ArabicComma)
                {
                    break;
                }
            }
        }
        Ok(args)
    }

    // ============ Type Parsing ============

    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, Diagnostic> {
        let start = self.current_span();

        // Simple type or generic base - can be identifier or type keyword
        let name = self.expect_type_name()?;

        // Check for generics
        if self.match_token(&TokenKind::Less) {
            let mut args = Vec::new();
            loop {
                args.push(self.parse_type_annotation()?);
                if !self.match_token(&TokenKind::Comma)
                    && !self.match_token(&TokenKind::ArabicComma)
                {
                    break;
                }
            }
            self.expect(&TokenKind::Greater, "Expected '>' / متوقع '>'")?;
            let span = start.merge(&self.previous_span());
            return Ok(TypeAnnotation::new(
                TypeKind::Generic { base: name, args },
                span,
            ));
        }

        // Check for optional
        let kind = if self.match_token(&TokenKind::Question) {
            TypeKind::Optional(Box::new(TypeAnnotation::new(TypeKind::Simple(name), start)))
        } else {
            TypeKind::Simple(name)
        };

        let span = start.merge(&self.previous_span());
        Ok(TypeAnnotation::new(kind, span))
    }

    fn parse_parameters(&mut self) -> Result<Vec<Param>, Diagnostic> {
        let mut params = Vec::new();

        if !self.check(&TokenKind::RightParen) {
            loop {
                let start = self.current_span();
                let name = self.expect_identifier("Expected parameter name / متوقع اسم المعامل")?;

                let ty = if self.match_token(&TokenKind::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };

                let default = if self.match_token(&TokenKind::Equal) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };

                params.push(Param {
                    name,
                    ty,
                    default,
                    span: start.merge(&self.previous_span()),
                });

                if !self.match_token(&TokenKind::Comma)
                    && !self.match_token(&TokenKind::ArabicComma)
                {
                    break;
                }
            }
        }

        Ok(params)
    }

    // ============ Helper Methods ============

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous().clone()
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn check_identifier(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Identifier(_))
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind, message: &str) -> Result<Token, Diagnostic> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(Diagnostic::error(
                message.split(" / ").next().unwrap_or(message),
                message.split(" / ").nth(1).unwrap_or(message),
                self.current_span(),
            ))
        }
    }

    fn expect_identifier(&mut self, message: &str) -> Result<String, Diagnostic> {
        if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(Diagnostic::error(
                message.split(" / ").next().unwrap_or(message),
                message.split(" / ").nth(1).unwrap_or(message),
                self.current_span(),
            ))
        }
    }

    fn expect_type_name(&mut self) -> Result<String, Diagnostic> {
        // Accept identifiers or type keywords
        let token = self.peek().clone();
        match &token.kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            TokenKind::TypeInt => {
                self.advance();
                Ok("عدد".to_string())
            }
            TokenKind::TypeFloat => {
                self.advance();
                Ok("عدد_عشري".to_string())
            }
            TokenKind::TypeString => {
                self.advance();
                Ok("نص".to_string())
            }
            TokenKind::TypeBool => {
                self.advance();
                Ok("منطقي".to_string())
            }
            TokenKind::TypeArray => {
                self.advance();
                Ok("مصفوفة".to_string())
            }
            TokenKind::TypeMap => {
                self.advance();
                Ok("قاموس".to_string())
            }
            TokenKind::TypeVoid => {
                self.advance();
                Ok("فراغ".to_string())
            }
            TokenKind::TypeAny => {
                self.advance();
                Ok("أي".to_string())
            }
            _ => Err(Diagnostic::error(
                "Expected type name",
                "متوقع اسم النوع",
                self.current_span(),
            )),
        }
    }

    fn expect_string(&mut self, message: &str) -> Result<String, Diagnostic> {
        if let TokenKind::StringLiteral(s) = &self.peek().kind {
            let s = s.clone();
            self.advance();
            Ok(s)
        } else {
            Err(Diagnostic::error(
                message.split(" / ").next().unwrap_or(message),
                message.split(" / ").nth(1).unwrap_or(message),
                self.current_span(),
            ))
        }
    }

    fn consume_semicolon(&mut self) -> Result<(), Diagnostic> {
        if self.match_token(&TokenKind::Semicolon) || self.match_token(&TokenKind::ArabicSemicolon)
        {
            Ok(())
        } else {
            // Semicolons are optional at end of blocks
            if self.check(&TokenKind::RightBrace) || self.is_at_end() {
                Ok(())
            } else {
                Err(Diagnostic::error(
                    "Expected ';'",
                    "متوقع '؛'",
                    self.current_span(),
                ))
            }
        }
    }

    fn current_span(&self) -> Span {
        self.peek().span
    }

    fn previous_span(&self) -> Span {
        self.previous().span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_variable_declaration() {
        let mut parser = Parser::new("متغير س = 5;");
        let ast = parser.parse().unwrap();

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::VarDecl {
                name,
                mutable,
                init,
                ..
            } => {
                assert_eq!(name, "س");
                assert!(*mutable);
                assert!(init.is_some());
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_parse_function_declaration() {
        let source = r#"
            دالة جمع(أ: عدد، ب: عدد) -> عدد {
                أرجع أ + ب;
            }
        "#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::FuncDecl { name, params, .. } => {
                assert_eq!(name, "جمع");
                assert_eq!(params.len(), 2);
            }
            _ => panic!("Expected FuncDecl"),
        }
    }

    #[test]
    fn test_parse_if_statement() {
        let source = r#"
            إذا (س > 5) {
                اطبع("كبير");
            } وإلا {
                اطبع("صغير");
            }
        "#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::If { else_branch, .. } => {
                assert!(else_branch.is_some());
            }
            _ => panic!("Expected If"),
        }
    }

    #[test]
    fn test_parse_class_declaration() {
        let source = r#"
            صنف شخص {
                خاص اسم: نص;

                منشئ(اسم: نص) {
                    هذا.اسم = اسم;
                }

                عام دالة احصل_اسم() -> نص {
                    أرجع هذا.اسم;
                }
            }
        "#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::ClassDecl { name, members, .. } => {
                assert_eq!(name, "شخص");
                assert_eq!(members.len(), 3); // field, constructor, method
            }
            _ => panic!("Expected ClassDecl"),
        }
    }

    #[test]
    fn test_parse_expressions() {
        let source = "1 + 2 * 3;";
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        // Should parse as 1 + (2 * 3) due to precedence
        assert_eq!(ast.statements.len(), 1);
    }

    #[test]
    fn test_parse_array_literal() {
        let source = "[1، 2، 3];";
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        match &ast.statements[0].kind {
            StmtKind::Expr(expr) => match &expr.kind {
                ExprKind::Array(elements) => {
                    assert_eq!(elements.len(), 3);
                }
                _ => panic!("Expected Array"),
            },
            _ => panic!("Expected Expr"),
        }
    }
}
