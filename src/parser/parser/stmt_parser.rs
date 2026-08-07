//! Statement parsing for the Tarqeem parser.
//!
//! This module handles parsing of statements including control flow,
//! loops, blocks, and match expressions.

use crate::error::Diagnostic;
use crate::lexer::TokenKind;

use super::super::ast::*;
use super::Parser;

impl Parser {
    /// Parse a statement (not a declaration).
    pub(crate) fn parse_statement(&mut self) -> Result<Stmt, Diagnostic> {
        if self.check(&TokenKind::If) {
            self.parse_if_statement()
        } else if self.check(&TokenKind::While) {
            self.parse_while_statement()
        } else if self.check(&TokenKind::Do) {
            self.parse_do_while_statement()
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

    /// Parse an if statement.
    pub(crate) fn parse_if_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'if'

        self.expect(&TokenKind::LeftParen, "متوقع '('")?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "متوقع ')'")?;

        let then_branch = self.parse_block()?;

        let else_branch = if self.match_token(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
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

    /// Parse a while statement.
    pub(crate) fn parse_while_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'while'

        self.expect(&TokenKind::LeftParen, "متوقع '('")?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "متوقع ')'")?;

        let body = self.parse_block()?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::While { condition, body }, span))
    }

    /// Parse a do-while statement.
    pub(crate) fn parse_do_while_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'do' / 'افعل'

        let body = self.parse_block()?;

        self.expect(&TokenKind::While, "متوقع 'طالما'")?;
        self.expect(&TokenKind::LeftParen, "متوقع '('")?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "متوقع ')'")?;

        let _ = self.match_token(&TokenKind::Semicolon)
            || self.match_token(&TokenKind::ArabicSemicolon);

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::DoWhile { body, condition }, span))
    }

    /// Parse a for statement (C-style or for-in).
    pub(crate) fn parse_for_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'for'

        if self.check_identifier() {
            let var_name = self.expect_identifier("متوقع اسم المتغير")?;
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
                self.current -= 1;
            }
        }

        self.expect(&TokenKind::LeftParen, "متوقع '('")?;

        let init = if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::ArabicSemicolon) {
            None
        } else if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) {
            Some(Box::new(self.parse_var_declaration(None)?))
        } else {
            let expr = self.parse_expression()?;
            self.consume_semicolon()?;
            Some(Box::new(Stmt::new(
                StmtKind::Expr(expr),
                self.previous_span(),
            )))
        };

        if init.is_none() {
            self.consume_semicolon()?;
        }

        let condition =
            if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::ArabicSemicolon) {
                None
            } else {
                Some(self.parse_expression()?)
            };
        self.consume_semicolon()?;

        let update = if self.check(&TokenKind::RightParen) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.expect(&TokenKind::RightParen, "متوقع ')'")?;

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

    /// Parse a match statement.
    pub(crate) fn parse_match_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'match'

        self.expect(&TokenKind::LeftParen, "متوقع '('")?;
        let expr = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "متوقع ')'")?;

        self.expect(&TokenKind::LeftBrace, "متوقع '{'")?;

        // Skip initial newlines
        self.skip_newlines();

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Skip any line comments before the arm
            self.collect_line_comments();
            if self.check(&TokenKind::RightBrace) {
                self.take_pending_comments();
                break;
            }
            match self.parse_match_arm() {
                Ok(arm) => arms.push(arm),
                Err(diagnostic) => {
                    self.report_error(diagnostic);
                    self.synchronize_to_arm();
                }
            }
            // Skip newlines after each arm
            self.skip_newlines();
        }

        self.expect(&TokenKind::RightBrace, "متوقع '}'")?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::Match { expr, arms }, span))
    }

    /// Parse a match arm.
    pub(crate) fn parse_match_arm(&mut self) -> Result<MatchArm, Diagnostic> {
        let arm_start = self.current_span();

        let is_default = self.match_token(&TokenKind::Default);
        let mut patterns = Vec::new();

        if is_default {
            patterns.push(Pattern::new(PatternKind::Wildcard, arm_start));
        } else {
            self.expect(&TokenKind::Case, "متوقع 'حالة'")?;
            loop {
                patterns.push(self.parse_pattern()?);
                if !self.match_token(&TokenKind::Comma)
                    && !self.match_token(&TokenKind::ArabicComma)
                {
                    break;
                }
            }
        }

        self.expect(&TokenKind::FatArrow, "متوقع '=>'")?;

        let body = if self.check(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else if self.check(&TokenKind::Return)
            || self.check(&TokenKind::Break)
            || self.check(&TokenKind::Continue)
        {
            let stmt = self.parse_statement()?;
            Block::new(vec![stmt], self.previous_span())
        } else {
            let expr = self.parse_expression()?;
            Block::new(
                vec![Stmt::new(StmtKind::Expr(expr), self.previous_span())],
                self.previous_span(),
            )
        };

        Ok(MatchArm {
            patterns,
            body,
            span: arm_start.merge(&self.previous_span()),
        })
    }

    /// Parse a pattern for match expressions.
    /// Patterns can be:
    /// - Literals: 1, "hello", صحيح
    /// - Enum variants: EnumName::VariantName or EnumName::VariantName(x, y)
    pub(crate) fn parse_pattern(&mut self) -> Result<Pattern, Diagnostic> {
        let start_span = self.current_span();

        // Check for enum variant pattern: identifier::identifier
        if let TokenKind::Identifier(name) = self.peek().kind.clone() {
            // Look ahead for ::
            if self.peek_next().map(|t| &t.kind) == Some(&TokenKind::ColonColon) {
                self.advance(); // consume enum name
                self.advance(); // consume ::

                // Get variant name
                let variant_name = self.expect_identifier("متوقع اسم الحالة بعد '::'")?;

                // Check for bindings
                let bindings = if self.match_token(&TokenKind::LeftParen) {
                    let mut bindings = Vec::new();
                    if !self.check(&TokenKind::RightParen) {
                        loop {
                            let binding = self.expect_identifier("متوقع اسم الربط")?;
                            bindings.push(binding);
                            if !self.match_token(&TokenKind::Comma)
                                && !self.match_token(&TokenKind::ArabicComma)
                            {
                                break;
                            }
                        }
                    }
                    self.expect(&TokenKind::RightParen, "متوقع ')'")?;
                    bindings
                } else {
                    Vec::new()
                };

                let end_span = self.previous_span();
                return Ok(Pattern::new(
                    PatternKind::EnumVariant {
                        enum_name: name,
                        variant_name,
                        bindings,
                    },
                    start_span.merge(&end_span),
                ));
            }
        }

        // Otherwise, parse as a literal/expression pattern
        let expr = self.parse_expression()?;
        let expr_span = expr.span;
        Ok(Pattern::new(PatternKind::Literal(expr), expr_span))
    }

    /// Parse a return statement.
    pub(crate) fn parse_return_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'return'

        // A newline (or trailing comment) after أرجع ends the statement,
        // otherwise a bare return swallows the next line as its expression
        let value = if self.check(&TokenKind::Semicolon)
            || self.check(&TokenKind::ArabicSemicolon)
            || self.check(&TokenKind::RightBrace)
            || self.check(&TokenKind::Newline)
            || matches!(self.peek().kind, TokenKind::LineComment(_))
            || self.is_at_end()
        {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.consume_semicolon()?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::Return(value), span))
    }

    /// Parse a break statement.
    pub(crate) fn parse_break_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let span = self.current_span();
        self.advance(); // consume 'break'
        self.consume_semicolon()?;
        Ok(Stmt::new(StmtKind::Break, span))
    }

    /// Parse a continue statement.
    pub(crate) fn parse_continue_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let span = self.current_span();
        self.advance(); // consume 'continue'
        self.consume_semicolon()?;
        Ok(Stmt::new(StmtKind::Continue, span))
    }

    /// Parse a try-catch-finally statement.
    pub(crate) fn parse_try_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'try'

        let body = self.parse_block()?;

        let catch = if self.match_token(&TokenKind::Catch) {
            self.expect(&TokenKind::LeftParen, "متوقع '('")?;
            let param = self.expect_identifier("متوقع اسم الخطأ")?;
            self.expect(&TokenKind::RightParen, "متوقع ')'")?;
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

    /// Parse a throw statement.
    pub(crate) fn parse_throw_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'throw'
        let expr = self.parse_expression()?;
        self.consume_semicolon()?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::Throw(expr), span))
    }

    /// Parse an expression statement.
    pub(crate) fn parse_expression_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let expr = self.parse_expression()?;
        self.consume_semicolon()?;
        let span = expr.span;
        Ok(Stmt::new(StmtKind::Expr(expr), span))
    }

    /// Parse a block of statements.
    pub(crate) fn parse_block(&mut self) -> Result<Block, Diagnostic> {
        let start = self.current_span();
        self.expect(&TokenKind::LeftBrace, "متوقع '{'")?;

        // Skip newlines after opening brace
        self.skip_newlines();

        let mut statements = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            match self.parse_declaration() {
                Ok(stmt) => statements.push(stmt),
                Err(diagnostic) => {
                    self.report_error(diagnostic);
                    self.synchronize();
                }
            }
            // Skip newlines after each statement
            self.skip_newlines();
        }

        self.expect(&TokenKind::RightBrace, "متوقع '}'")?;

        let span = start.merge(&self.previous_span());
        Ok(Block::new(statements, span))
    }
}
