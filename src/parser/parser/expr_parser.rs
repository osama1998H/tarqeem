//! Expression parsing for the Tarqeem parser.
//!
//! This module handles parsing of expressions using Pratt parsing
//! for operator precedence.

use crate::error::codes::ERR_UNEXPECTED_EXPRESSION;
use crate::error::{Diagnostic, Span};
use crate::lexer::TokenKind;

use super::super::ast::*;
use super::super::precedence::Precedence;
use super::{declaration_name, identifier_like_name, within_brackets, Parser};

impl Parser {
    /// Parse an expression.
    pub(crate) fn parse_expression(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_precedence(Precedence::Assignment)
    }

    /// Parse an expression with the given minimum precedence.
    pub(crate) fn parse_precedence(&mut self, precedence: Precedence) -> Result<Expr, Diagnostic> {
        // Inside an unclosed `(` or `[` a newline cannot be ending a statement,
        // so an operand may sit on the line after its operator (issue #255).
        // Needed on both sides of the loop: here for the right-hand operand
        // parse_infix recurses into, below for the operator itself. At depth 0
        // the newline still terminates the statement, which is what keeps
        // `متغير س = 1 +⏎ 2` an error.
        if self.bracket_depth > 0 {
            self.skip_newlines();
        }

        let mut left = self.parse_prefix()?;

        while !self.is_at_end() {
            if self.bracket_depth > 0 {
                self.skip_newlines();
            }

            let op_prec = Precedence::of(&self.peek().kind);
            if precedence > op_prec {
                break;
            }

            left = self.parse_infix(left)?;
        }

        Ok(left)
    }

    /// Parse a prefix expression.
    pub(crate) fn parse_prefix(&mut self) -> Result<Expr, Diagnostic> {
        let token = self.advance();
        let span = token.span;

        // Identifiers, including the contextual keywords احصل/عيّن/حالة, are
        // handled before the match so the token-kind set lives only in
        // identifier_like_name (same early-return shape as expect_type_name).
        if let Some(name) = identifier_like_name(&token) {
            let name = name.to_string();

            // Check for generic type args: Identifier<T>
            let type_args = if self.check(&TokenKind::Less) {
                // Try to parse type args - this is speculative
                // We need to check if this is actually a generic type or a comparison
                self.try_parse_type_args()?.unwrap_or_default()
            } else {
                Vec::new()
            };

            // Check for enum variant access: Identifier::Variant or Identifier<T>::Variant
            if self.check(&TokenKind::ColonColon) {
                self.advance(); // consume '::'
                let variant_name = self.expect_variant_name("متوقع اسم الحالة بعد '::'")?;

                // Check for variant arguments: Variant(args)
                let args = if self.match_token(&TokenKind::LeftParen) {
                    let args = self.parse_arguments()?;
                    self.expect(&TokenKind::RightParen, "متوقع ')'")?;
                    args
                } else {
                    Vec::new()
                };

                let end_span = self.previous_span();
                return Ok(Expr::new(
                    ExprKind::EnumVariant {
                        enum_name: name,
                        type_args,
                        variant_name,
                        args,
                    },
                    span.merge(&end_span),
                ));
            }
            // If type args were parsed without a following '::', return as a
            // plain identifier and let semantic analysis handle it.
            return Ok(Expr::new(ExprKind::Identifier(name), span));
        }

        match &token.kind {
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

            TokenKind::TypeInt => Ok(Expr::new(ExprKind::Identifier("عدد".to_string()), span)),
            TokenKind::TypeFloat => Ok(Expr::new(
                ExprKind::Identifier("عدد_عشري".to_string()),
                span,
            )),
            TokenKind::TypeString => Ok(Expr::new(ExprKind::Identifier("نص".to_string()), span)),
            TokenKind::TypeBool => Ok(Expr::new(ExprKind::Identifier("منطقي".to_string()), span)),
            // مصفوفة/قاموس/أي complete the set: عدد and friends were already here
            // because they double as builtin conversion functions, but a
            // parameter named مصفوفة (stdlib_trq/اختبار/توكيدات.ترقيم:391) was
            // declarable and then unreadable in its own body.
            TokenKind::TypeArray | TokenKind::TypeMap | TokenKind::TypeAny => {
                Ok(Expr::new(ExprKind::Identifier(token.lexeme.clone()), span))
            }

            TokenKind::This => Ok(Expr::new(ExprKind::This, span)),
            TokenKind::Super => Ok(Expr::new(ExprKind::Super, span)),

            TokenKind::LeftParen => {
                if let Some(lambda) = self.try_parse_arrow_function(span)? {
                    return Ok(lambda);
                }
                let expr = within_brackets(self, |parser| {
                    parser.skip_newlines();
                    let expr = parser.parse_expression()?;
                    parser.skip_newlines();
                    Ok::<_, Diagnostic>(expr)
                })?;
                self.expect(&TokenKind::RightParen, "متوقع ')'")?;
                let end_span = self.previous_span();
                Ok(Expr::new(
                    ExprKind::Grouping(Box::new(expr)),
                    span.merge(&end_span),
                ))
            }

            TokenKind::LeftBracket => {
                let elements = within_brackets(self, |parser| {
                    let mut elements = Vec::new();
                    parser.skip_newlines();
                    if !parser.check(&TokenKind::RightBracket) {
                        loop {
                            parser.skip_newlines();
                            elements.push(parser.parse_expression()?);
                            parser.skip_newlines();
                            if !parser.match_token(&TokenKind::Comma)
                                && !parser.match_token(&TokenKind::ArabicComma)
                            {
                                break;
                            }
                        }
                    }
                    parser.skip_newlines();
                    Ok::<_, Diagnostic>(elements)
                })?;
                self.expect(&TokenKind::RightBracket, "متوقع ']'")?;
                let end_span = self.previous_span();
                Ok(Expr::new(ExprKind::Array(elements), span.merge(&end_span)))
            }

            TokenKind::LeftBrace => {
                let mut pairs = Vec::new();
                self.skip_newlines();
                if !self.check(&TokenKind::RightBrace) {
                    loop {
                        self.skip_newlines();
                        let key = self.expect_identifier("متوقع مفتاح")?;
                        self.expect(&TokenKind::Colon, "متوقع ':'")?;
                        let value = self.parse_expression()?;
                        pairs.push((key, value));
                        self.skip_newlines();
                        if !self.match_token(&TokenKind::Comma)
                            && !self.match_token(&TokenKind::ArabicComma)
                        {
                            break;
                        }
                    }
                }
                self.skip_newlines();
                self.expect(&TokenKind::RightBrace, "متوقع '}'")?;
                let end_span = self.previous_span();
                Ok(Expr::new(ExprKind::Object(pairs), span.merge(&end_span)))
            }

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

            TokenKind::New => {
                let class = self.parse_precedence(Precedence::Primary)?;

                let type_args = if self.check(&TokenKind::Less) {
                    self.advance(); // consume '<'
                    let mut args = Vec::new();
                    loop {
                        args.push(self.parse_type_annotation()?);
                        if !self.match_token(&TokenKind::Comma)
                            && !self.match_token(&TokenKind::ArabicComma)
                        {
                            break;
                        }
                    }
                    self.expect(&TokenKind::Greater, "متوقع '>'")?;
                    args
                } else {
                    Vec::new()
                };

                let args = if self.match_token(&TokenKind::LeftParen) {
                    let args = self.parse_arguments()?;
                    self.expect(&TokenKind::RightParen, "متوقع ')'")?;
                    args
                } else {
                    Vec::new()
                };
                let end_span = self.previous_span();
                Ok(Expr::new(
                    ExprKind::New {
                        class: Box::new(class),
                        type_args,
                        args,
                    },
                    span.merge(&end_span),
                ))
            }

            TokenKind::Await => {
                let expr = self.parse_precedence(Precedence::Unary)?;
                let end_span = expr.span;
                Ok(Expr::new(
                    ExprKind::Await(Box::new(expr)),
                    span.merge(&end_span),
                ))
            }

            _ => Err(
                Diagnostic::error(format!("رمز غير متوقع: {:?}", token.kind), span)
                    .with_code(ERR_UNEXPECTED_EXPRESSION.to_string()),
            ),
        }
    }

    /// Parse an infix expression.
    pub(crate) fn parse_infix(&mut self, left: Expr) -> Result<Expr, Diagnostic> {
        let token = self.peek().clone();
        let op_prec = Precedence::of(&token.kind);

        match &token.kind {
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

            TokenKind::LeftParen => {
                self.advance();
                let args = self.parse_arguments()?;
                self.expect(&TokenKind::RightParen, "متوقع ')'")?;
                let span = left.span.merge(&self.previous_span());
                Ok(Expr::new(
                    ExprKind::Call {
                        callee: Box::new(left),
                        args,
                    },
                    span,
                ))
            }

            TokenKind::LeftBracket => {
                self.advance();
                let index = self.parse_expression()?;
                self.expect(&TokenKind::RightBracket, "متوقع ']'")?;
                let span = left.span.merge(&self.previous_span());
                Ok(Expr::new(
                    ExprKind::Index {
                        object: Box::new(left),
                        index: Box::new(index),
                    },
                    span,
                ))
            }

            TokenKind::Dot => {
                self.advance();
                let property = self.expect_declaration_name("متوقع اسم الخاصية")?;
                let span = left.span.merge(&self.previous_span());
                Ok(Expr::new(
                    ExprKind::Member {
                        object: Box::new(left),
                        property,
                    },
                    span,
                ))
            }

            TokenKind::Question => {
                self.advance();
                let then_expr = self.parse_expression()?;
                self.expect(&TokenKind::Colon, "متوقع ':'")?;
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

    /// Convert a token to a binary operator.
    pub(crate) fn token_to_binary_op(&self, kind: &TokenKind) -> BinaryOp {
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

    /// Convert a compound assignment token to a binary operator.
    pub(crate) fn compound_to_binary_op(&self, kind: &TokenKind) -> BinaryOp {
        match kind {
            TokenKind::PlusEqual => BinaryOp::Add,
            TokenKind::MinusEqual => BinaryOp::Sub,
            TokenKind::StarEqual => BinaryOp::Mul,
            TokenKind::SlashEqual => BinaryOp::Div,
            TokenKind::PercentEqual => BinaryOp::Mod,
            _ => unreachable!(),
        }
    }

    /// Parse function call arguments.
    ///
    /// Newlines are trivia inside the parentheses, so a call can be wrapped over
    /// several lines (issue #255). Statements are still newline-terminated —
    /// only an unclosed bracket suspends that, the same way the array-literal
    /// loop in `parse_prefix` already did.
    pub(crate) fn parse_arguments(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        within_brackets(self, |parser| {
            let mut args = Vec::new();
            parser.skip_newlines();
            if !parser.check(&TokenKind::RightParen) {
                loop {
                    parser.skip_newlines();
                    args.push(parser.parse_expression()?);
                    parser.skip_newlines();
                    if !parser.match_token(&TokenKind::Comma)
                        && !parser.match_token(&TokenKind::ArabicComma)
                    {
                        break;
                    }
                }
            }
            parser.skip_newlines();
            Ok(args)
        })
    }

    /// Try to parse type arguments for generic enum variants: `اختياري<عدد>::بعض`
    /// Returns None if it doesn't look like type args (e.g., comparison expression)
    pub(crate) fn try_parse_type_args(
        &mut self,
    ) -> Result<Option<Vec<TypeAnnotation>>, Diagnostic> {
        // We're at '<' - check if what follows looks like a type
        // If the next token is an identifier or type keyword, try to parse as type args
        // This is speculative - we commit only if we see '>' followed by '::'

        let saved_pos = self.current;

        if !self.match_token(&TokenKind::Less) {
            return Ok(None);
        }

        // Check if this looks like type args (identifier, contextual keyword,
        // or type keyword after '<')
        let looks_like_type = self.check_identifier()
            || matches!(
                &self.peek().kind,
                TokenKind::TypeInt
                    | TokenKind::TypeFloat
                    | TokenKind::TypeString
                    | TokenKind::TypeBool
                    | TokenKind::TypeArray
                    | TokenKind::TypeMap
                    | TokenKind::TypeAny
            );

        if !looks_like_type {
            // Not type args, rollback
            self.current = saved_pos;
            return Ok(None);
        }

        // Try to parse type annotations
        let mut args = Vec::new();
        loop {
            match self.parse_type_annotation() {
                Ok(ty) => args.push(ty),
                Err(_) => {
                    // Failed to parse type, rollback
                    self.current = saved_pos;
                    return Ok(None);
                }
            }
            if !self.match_token(&TokenKind::Comma) && !self.match_token(&TokenKind::ArabicComma) {
                break;
            }
        }

        // Expect '>'
        if !self.match_token(&TokenKind::Greater) {
            // Not type args, rollback
            self.current = saved_pos;
            return Ok(None);
        }

        // Check if followed by '::' - if not, it might be a comparison
        // For enum variants, we need '::'
        if !self.check(&TokenKind::ColonColon) {
            // Not followed by ::, rollback (it's a comparison like x < y > z)
            self.current = saved_pos;
            return Ok(None);
        }

        Ok(Some(args))
    }

    /// Try to parse an arrow function.
    pub(crate) fn try_parse_arrow_function(
        &mut self,
        start_span: Span,
    ) -> Result<Option<Expr>, Diagnostic> {
        let saved_pos = self.current;

        let params = match self.try_parse_arrow_params() {
            Ok(Some(params)) => params,
            Ok(None) => {
                self.current = saved_pos;
                return Ok(None);
            }
            Err(_) => {
                self.current = saved_pos;
                return Ok(None);
            }
        };

        if !self.check(&TokenKind::FatArrow) {
            self.current = saved_pos;
            return Ok(None);
        }
        self.advance(); // consume '=>'

        let body = if self.check(&TokenKind::LeftBrace) {
            LambdaBody::Block(self.parse_block()?)
        } else {
            LambdaBody::Expr(Box::new(self.parse_precedence(Precedence::Assignment)?))
        };

        let end_span = self.previous_span();
        Ok(Some(Expr::new(
            ExprKind::Lambda { params, body },
            start_span.merge(&end_span),
        )))
    }

    /// Try to parse arrow function parameters.
    pub(crate) fn try_parse_arrow_params(&mut self) -> Result<Option<Vec<Param>>, Diagnostic> {
        let mut params = Vec::new();

        if self.check(&TokenKind::RightParen) {
            self.advance(); // consume ')'
            return Ok(Some(params));
        }

        loop {
            let param_start = self.current_span();

            let name = match declaration_name(self.peek()).map(str::to_string) {
                Some(name) => {
                    self.advance();
                    name
                }
                None => return Ok(None), // Not an arrow function
            };

            let ty = if self.check(&TokenKind::Colon) {
                self.advance(); // consume ':'
                match self.parse_type_annotation() {
                    Ok(ty) => Some(ty),
                    Err(_) => return Ok(None),
                }
            } else {
                None
            };

            let default = if self.check(&TokenKind::Equal) {
                self.advance(); // consume '='
                match self.parse_expression() {
                    Ok(expr) => Some(expr),
                    Err(_) => return Ok(None),
                }
            } else {
                None
            };

            params.push(Param {
                name,
                ty,
                default,
                span: param_start.merge(&self.previous_span()),
            });

            if self.check(&TokenKind::Comma) || self.check(&TokenKind::ArabicComma) {
                self.advance(); // consume comma
            } else if self.check(&TokenKind::RightParen) {
                self.advance(); // consume ')'
                return Ok(Some(params));
            } else {
                return Ok(None); // Not a valid parameter list
            }
        }
    }
}
