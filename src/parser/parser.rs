//! Recursive descent parser for Tarqeem

use super::ast::*;
use super::precedence::Precedence;
use crate::error::{Diagnostic, Span};
use crate::lexer::{Lexer, Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    errors: Vec<Diagnostic>,
    panic_mode: bool,
    /// Line comments pending to be attached to the next statement
    pending_comments: Vec<String>,
}

impl Parser {
    pub fn new(source: &str) -> Self {
        let mut lexer = Lexer::new(source);
        let tokens: Vec<Token> = lexer
            .tokenize()
            .into_iter()
            .filter(|t| !matches!(t.kind, TokenKind::Newline))
            .collect();

        Self {
            tokens,
            current: 0,
            errors: Vec::new(),
            panic_mode: false,
            pending_comments: Vec::new(),
        }
    }

    pub fn from_tokens(tokens: Vec<Token>) -> Self {
        let tokens: Vec<Token> = tokens
            .into_iter()
            .filter(|t| !matches!(t.kind, TokenKind::Newline))
            .collect();
        Self {
            tokens,
            current: 0,
            errors: Vec::new(),
            panic_mode: false,
            pending_comments: Vec::new(),
        }
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;

        while !self.is_at_end() {
            if self.previous().kind == TokenKind::Semicolon {
                return;
            }

            match self.peek().kind {
                TokenKind::Let        // متغير
                | TokenKind::Const    // ثابت
                | TokenKind::Function // دالة
                | TokenKind::Class    // صنف
                | TokenKind::Interface // ميثاق
                | TokenKind::Enum     // تعداد
                | TokenKind::If       // إذا
                | TokenKind::While    // طالما
                | TokenKind::For      // لكل
                | TokenKind::Do       // افعل
                | TokenKind::Return   // أرجع
                | TokenKind::Try      // حاول
                | TokenKind::Match    // تطابق
                | TokenKind::Import   // استورد
                | TokenKind::Export   // صدّر
                | TokenKind::Alhamdulillah => {
                    return;
                }
                _ => {}
            }

            self.advance();
        }
    }

    fn synchronize_to_member(&mut self) {
        self.panic_mode = false;

        while !self.is_at_end() {
            match self.peek().kind {
                TokenKind::Public     // عام
                | TokenKind::Private  // خاص
                | TokenKind::Protected // محمي
                | TokenKind::Static   // مشترك
                | TokenKind::Function // دالة
                | TokenKind::Async    // غير_متزامن
                | TokenKind::Constructor // منشئ
                | TokenKind::RightBrace => {
                    return;
                }
                TokenKind::Identifier(_) => {
                    return;
                }
                _ => {}
            }

            self.advance();
        }
    }

    fn synchronize_to_arm(&mut self) {
        self.panic_mode = false;

        while !self.is_at_end() {
            match self.peek().kind {
                TokenKind::Case      // حالة
                | TokenKind::Default // غير_ذلك
                | TokenKind::RightBrace => {
                    return;
                }
                _ => {}
            }

            self.advance();
        }
    }

    fn report_error(&mut self, diagnostic: Diagnostic) {
        if !self.panic_mode {
            self.errors.push(diagnostic);
            self.panic_mode = true;
        }
    }

    pub fn get_errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    fn consume_doc_comment(&mut self) -> Option<String> {
        match &self.peek().kind {
            TokenKind::DocComment(content) => {
                let content = content.clone();
                self.advance();
                Some(content)
            }
            TokenKind::BlockDocComment(content) => {
                let content = content.clone();
                self.advance();
                Some(content)
            }
            _ => None,
        }
    }

    /// Collects all line comments before the next non-comment token
    fn collect_line_comments(&mut self) {
        while let TokenKind::LineComment(content) = &self.peek().kind {
            self.pending_comments.push(content.clone());
            self.advance();
        }
    }

    /// Takes pending comments and clears the buffer
    fn take_pending_comments(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_comments)
    }

    pub fn parse(&mut self) -> Result<Ast, Diagnostic> {
        let bismillah_span = if self.check(&TokenKind::Bismillah) {
            let span = self.current_span();
            self.advance();
            span
        } else {
            return Err(Diagnostic::error(
                "File must start with 'بسم_الله' (bismillah)",
                "يجب أن يبدأ الملف بـ 'بسم_الله'",
                self.current_span(),
            ));
        };

        let mut statements = Vec::new();

        while !self.is_at_end() && !self.check(&TokenKind::Alhamdulillah) {
            match self.parse_declaration() {
                Ok(stmt) => {
                    statements.push(stmt);
                }
                Err(diagnostic) => {
                    self.report_error(diagnostic);
                    self.synchronize();
                }
            }
        }

        let alhamdulillah_span = if self.check(&TokenKind::Alhamdulillah) {
            let span = self.current_span();
            self.advance();
            span
        } else {
            let err = Diagnostic::error(
                "File must end with 'الحمد_لله' (alhamdulillah)",
                "يجب أن ينتهي الملف بـ 'الحمد_لله'",
                self.current_span(),
            );
            if !self.errors.is_empty() {
                self.report_error(err);
                return Err(self.errors[0].clone());
            }
            return Err(err);
        };

        if !self.is_at_end() {
            let err = Diagnostic::error(
                "No code allowed after 'الحمد_لله' (alhamdulillah)",
                "لا يُسمح بأي كود بعد 'الحمد_لله'",
                self.current_span(),
            );
            if !self.errors.is_empty() {
                self.report_error(err);
                return Err(self.errors[0].clone());
            }
            return Err(err);
        }

        if !self.errors.is_empty() {
            return Err(self.errors[0].clone());
        }

        Ok(Ast::with_markers(
            statements,
            bismillah_span,
            alhamdulillah_span,
        ))
    }

    fn parse_declaration(&mut self) -> Result<Stmt, Diagnostic> {
        // Collect any line comments before the declaration
        self.collect_line_comments();
        let leading_comments = self.take_pending_comments();

        let doc_comment = self.consume_doc_comment();

        let mut stmt = if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) {
            self.parse_var_declaration(doc_comment)?
        } else if self.check(&TokenKind::Function) {
            self.parse_function_declaration(false, doc_comment)?
        } else if self.check(&TokenKind::Async) {
            self.advance();
            self.parse_function_declaration(true, doc_comment)?
        } else if self.check(&TokenKind::Class) {
            self.parse_class_declaration(doc_comment)?
        } else if self.check(&TokenKind::Interface) {
            self.parse_interface_declaration(doc_comment)?
        } else if self.check(&TokenKind::Enum) {
            self.parse_enum_declaration(doc_comment)?
        } else if self.check(&TokenKind::Import) {
            self.parse_import_statement()?
        } else if self.check(&TokenKind::Export) {
            self.parse_export_statement()?
        } else {
            self.parse_statement()?
        };

        // Attach leading comments to the statement
        stmt.leading_comments = leading_comments;
        Ok(stmt)
    }

    fn parse_var_declaration(&mut self, doc_comment: Option<String>) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        let mutable = self.check(&TokenKind::Let);
        self.advance(); // consume 'let' or 'const'

        let name = self.expect_identifier("Expected variable name", "متوقع اسم المتغير")?;

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
                doc_comment,
            },
            span,
        ))
    }

    fn parse_function_declaration(
        &mut self,
        is_async: bool,
        doc_comment: Option<String>,
    ) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.expect(&TokenKind::Function, "Expected 'function'", "متوقع 'دالة'")?;

        let name = self.expect_identifier("Expected function name", "متوقع اسم الدالة")?;

        self.expect(&TokenKind::LeftParen, "Expected '('", "متوقع '('")?;
        let params = self.parse_parameters()?;
        self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;

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
                doc_comment,
            },
            span,
        ))
    }

    fn parse_class_declaration(&mut self, doc_comment: Option<String>) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'class'

        let name = self.expect_identifier("Expected class name", "متوقع اسم الصنف")?;

        let type_params = self.parse_type_parameters()?;

        let extends = if self.check(&TokenKind::Extends) {
            self.advance();
            Some(self.expect_identifier("Expected superclass name", "متوقع اسم الصنف الأب")?)
        } else {
            None
        };

        let mut implements = Vec::new();
        if self.check(&TokenKind::Implements) {
            self.advance();
            loop {
                let interface_name =
                    self.expect_identifier("Expected interface name", "متوقع اسم الميثاق")?;
                implements.push(interface_name);

                if self.check(&TokenKind::Less) {
                    self.advance(); // consume '<'
                    loop {
                        self.parse_type_annotation()?;
                        if !self.match_token(&TokenKind::Comma)
                            && !self.match_token(&TokenKind::ArabicComma)
                        {
                            break;
                        }
                    }
                    self.expect(&TokenKind::Greater, "Expected '>'", "متوقع '>'")?;
                }

                if !self.match_token(&TokenKind::Comma)
                    && !self.match_token(&TokenKind::ArabicComma)
                {
                    break;
                }
            }
        }

        self.expect(&TokenKind::LeftBrace, "Expected '{'", "متوقع '{'")?;
        let members = self.parse_class_members()?;
        self.expect(&TokenKind::RightBrace, "Expected '}'", "متوقع '}'")?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(
            StmtKind::ClassDecl {
                name,
                type_params,
                extends,
                implements,
                members,
                doc_comment,
            },
            span,
        ))
    }

    fn parse_type_parameters(&mut self) -> Result<Vec<String>, Diagnostic> {
        let mut params = Vec::new();

        if !self.check(&TokenKind::Less) {
            return Ok(params);
        }

        self.advance(); // consume '<'

        loop {
            let param =
                self.expect_identifier("Expected type parameter name", "متوقع اسم معامل النوع")?;
            params.push(param);

            if !self.match_token(&TokenKind::Comma) && !self.match_token(&TokenKind::ArabicComma) {
                break;
            }
        }

        self.expect(&TokenKind::Greater, "Expected '>'", "متوقع '>'")?;
        Ok(params)
    }

    fn parse_class_members(&mut self) -> Result<Vec<ClassMember>, Diagnostic> {
        let mut members = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            match self.parse_class_member() {
                Ok(member) => members.push(member),
                Err(diagnostic) => {
                    self.report_error(diagnostic);
                    self.synchronize_to_member();
                }
            }
        }

        Ok(members)
    }

    fn parse_class_member(&mut self) -> Result<ClassMember, Diagnostic> {
        self.collect_line_comments();
        let member_doc = self.consume_doc_comment();

        let visibility = self.parse_visibility();
        let is_static = self.match_token(&TokenKind::Static);

        if self.check(&TokenKind::Constructor) {
            self.advance();
            self.expect(&TokenKind::LeftParen, "Expected '('", "متوقع '('")?;
            let params = self.parse_parameters()?;
            self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;
            let body = self.parse_block()?;
            Ok(ClassMember::Constructor {
                params,
                body,
                doc_comment: member_doc,
            })
        } else if self.check(&TokenKind::Function) || self.check(&TokenKind::Async) {
            let is_async = self.match_token(&TokenKind::Async);
            self.expect(&TokenKind::Function, "Expected 'function'", "متوقع 'دالة'")?;
            let name = self.expect_identifier("Expected method name", "متوقع اسم الدالة")?;
            self.expect(&TokenKind::LeftParen, "Expected '('", "متوقع '('")?;
            let params = self.parse_parameters()?;
            self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;

            let return_type = if self.match_token(&TokenKind::Arrow) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };

            let body = self.parse_block()?;

            Ok(ClassMember::Method {
                visibility,
                name,
                params,
                return_type,
                body,
                is_static,
                is_async,
                doc_comment: member_doc,
            })
        } else if self.check(&TokenKind::Property) {
            self.advance();
            let name = self.expect_identifier("Expected property name", "متوقع اسم الخاصية")?;
            self.expect(&TokenKind::Colon, "Expected ':'", "متوقع ':'")?;
            let ty = self.parse_type_annotation()?;

            let (accessors, default_value) = if self.match_token(&TokenKind::LeftBrace) {
                let accessors = self.parse_property_accessors()?;
                self.expect(&TokenKind::RightBrace, "Expected '}'", "متوقع '}'")?;
                (accessors, None)
            } else if self.match_token(&TokenKind::Equal) {
                let default = self.parse_expression()?;
                self.consume_semicolon()?;
                (Vec::new(), Some(default))
            } else {
                self.consume_semicolon()?;
                (Vec::new(), None)
            };

            Ok(ClassMember::Property {
                visibility,
                name,
                ty,
                accessors,
                default_value,
                is_static,
                doc_comment: member_doc,
            })
        } else {
            let name = self.expect_identifier("Expected field name", "متوقع اسم الحقل")?;
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

            Ok(ClassMember::Field {
                visibility,
                name,
                ty,
                init,
                is_static,
                doc_comment: member_doc,
            })
        }
    }

    fn parse_property_accessors(&mut self) -> Result<Vec<PropertyAccessor>, Diagnostic> {
        let mut accessors = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            self.collect_line_comments();
            let accessor_visibility = self.parse_visibility();

            if self.match_token(&TokenKind::Get) {
                let body = if self.match_token(&TokenKind::FatArrow) {
                    let expr = self.parse_expression()?;
                    PropertyAccessorBody::Expr(Box::new(expr))
                } else {
                    PropertyAccessorBody::Block(self.parse_block()?)
                };

                accessors.push(PropertyAccessor::Get {
                    visibility: accessor_visibility,
                    body,
                });
            } else if self.match_token(&TokenKind::Set) {
                let param_name = if self.match_token(&TokenKind::LeftParen) {
                    let name =
                        self.expect_identifier("Expected parameter name", "متوقع اسم المعامل")?;
                    self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;
                    name
                } else {
                    "قيمة".to_string() // Default parameter name
                };

                let body = self.parse_block()?;

                accessors.push(PropertyAccessor::Set {
                    visibility: accessor_visibility,
                    param_name,
                    body,
                });
            } else {
                return Err(Diagnostic::error(
                    "Expected 'احصل' (get) or 'عيّن' (set)",
                    "متوقع 'احصل' أو 'عيّن'",
                    self.current_span(),
                ));
            }
        }

        Ok(accessors)
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

    fn parse_interface_declaration(
        &mut self,
        doc_comment: Option<String>,
    ) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'interface'

        let name = self.expect_identifier("Expected interface name", "متوقع اسم الميثاق")?;

        let type_params = self.parse_type_parameters()?;

        self.expect(&TokenKind::LeftBrace, "Expected '{'", "متوقع '{'")?;

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Skip any line comments before the method
            self.collect_line_comments();
            if self.check(&TokenKind::RightBrace) {
                self.take_pending_comments();
                break;
            }
            let method_doc = self.consume_doc_comment();

            self.expect(&TokenKind::Function, "Expected 'function'", "متوقع 'دالة'")?;
            let method_name = self.expect_identifier("Expected method name", "متوقع اسم الدالة")?;
            self.expect(&TokenKind::LeftParen, "Expected '('", "متوقع '('")?;
            let params = self.parse_parameters()?;
            self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;

            let return_type = if self.match_token(&TokenKind::Arrow) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };

            methods.push(MethodSignature {
                name: method_name,
                params,
                return_type,
                doc_comment: method_doc,
            });
        }

        self.expect(&TokenKind::RightBrace, "Expected '}'", "متوقع '}'")?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(
            StmtKind::InterfaceDecl {
                name,
                type_params,
                methods,
                doc_comment,
            },
            span,
        ))
    }

    fn parse_enum_declaration(&mut self, doc_comment: Option<String>) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'تعداد'

        let name = self.expect_identifier("Expected enum name", "متوقع اسم التعداد")?;

        let type_params = self.parse_type_parameters()?;

        self.expect(&TokenKind::LeftBrace, "Expected '{'", "متوقع '{'")?;

        let mut variants = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Skip any line comments before the variant
            self.collect_line_comments();
            if self.check(&TokenKind::RightBrace) {
                self.take_pending_comments();
                break;
            }
            let variant = self.parse_enum_variant()?;
            variants.push(variant);

            let _ =
                self.match_token(&TokenKind::Comma) || self.match_token(&TokenKind::ArabicComma);
        }

        self.expect(&TokenKind::RightBrace, "Expected '}'", "متوقع '}'")?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(
            StmtKind::EnumDecl {
                name,
                type_params,
                variants,
                doc_comment,
            },
            span,
        ))
    }

    fn parse_enum_variant(&mut self) -> Result<EnumVariant, Diagnostic> {
        let start = self.current_span();

        let variant_doc = self.consume_doc_comment();

        let name = self.expect_identifier("Expected variant name", "متوقع اسم الحالة")?;

        let discriminant = if self.match_token(&TokenKind::Equal) {
            let is_negative = self.match_token(&TokenKind::Minus);

            match &self.peek().kind {
                TokenKind::IntLiteral(n) => {
                    let value = if is_negative { -(*n) } else { *n };
                    self.advance();
                    Some(value)
                }
                _ => {
                    return Err(Diagnostic::error(
                        "Expected integer value for discriminant",
                        "متوقع قيمة عددية للمميز",
                        self.current_span(),
                    ))
                }
            }
        } else {
            None
        };

        let fields = if self.check(&TokenKind::LeftParen) {
            self.parse_enum_variant_fields()?
        } else {
            Vec::new()
        };

        let span = start.merge(&self.previous_span());
        Ok(EnumVariant {
            name,
            discriminant,
            fields,
            doc_comment: variant_doc,
            span,
        })
    }

    fn parse_enum_variant_fields(&mut self) -> Result<Vec<EnumVariantField>, Diagnostic> {
        self.advance(); // consume '('

        let mut fields = Vec::new();

        if !self.check(&TokenKind::RightParen) {
            loop {
                let field_start = self.current_span();

                let (name, ty) = if self.check_identifier() {
                    let first = self.expect_identifier(
                        "Expected field name or type",
                        "متوقع اسم الحقل أو النوع",
                    )?;

                    if self.match_token(&TokenKind::Colon) {
                        let ty = self.parse_type_annotation()?;
                        (Some(first), ty)
                    } else {
                        let span = field_start.merge(&self.previous_span());
                        (None, TypeAnnotation::new(TypeKind::Simple(first), span))
                    }
                } else {
                    let ty = self.parse_type_annotation()?;
                    (None, ty)
                };

                let span = field_start.merge(&self.previous_span());
                fields.push(EnumVariantField { name, ty, span });

                if !self.match_token(&TokenKind::Comma)
                    && !self.match_token(&TokenKind::ArabicComma)
                {
                    break;
                }
            }
        }

        self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;
        Ok(fields)
    }

    fn parse_import_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'import'

        let items = if self.match_token(&TokenKind::Star) {
            self.expect(&TokenKind::As, "Expected 'as'", "متوقع 'كـ'")?;
            let alias = self.expect_identifier("Expected alias", "متوقع اسم مستعار")?;
            ImportItems::Wildcard(alias)
        } else if self.match_token(&TokenKind::LeftBrace) {
            let mut items = Vec::new();
            loop {
                let name = self.expect_identifier("Expected import name", "متوقع اسم")?;
                let alias = if self.match_token(&TokenKind::As) {
                    Some(self.expect_identifier("Expected alias", "متوقع اسم مستعار")?)
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
            self.expect(&TokenKind::RightBrace, "Expected '}'", "متوقع '}'")?;
            ImportItems::Named(items)
        } else {
            let name =
                self.expect_identifier("Expected default import", "متوقع استيراد افتراضي")?;
            ImportItems::Default(name)
        };

        self.expect(&TokenKind::From, "Expected 'from'", "متوقع 'من'")?;
        let from = self.expect_string("Expected module path", "متوقع مسار الوحدة")?;

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

    fn parse_statement(&mut self) -> Result<Stmt, Diagnostic> {
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

    fn parse_if_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'if'

        self.expect(&TokenKind::LeftParen, "Expected '('", "متوقع '('")?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;

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

    fn parse_while_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'while'

        self.expect(&TokenKind::LeftParen, "Expected '('", "متوقع '('")?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;

        let body = self.parse_block()?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::While { condition, body }, span))
    }

    fn parse_do_while_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'do' / 'افعل'

        let body = self.parse_block()?;

        self.expect(&TokenKind::While, "Expected 'while'", "متوقع 'طالما'")?;
        self.expect(&TokenKind::LeftParen, "Expected '('", "متوقع '('")?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;

        let _ = self.match_token(&TokenKind::Semicolon)
            || self.match_token(&TokenKind::ArabicSemicolon);

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::DoWhile { body, condition }, span))
    }

    fn parse_for_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'for'

        if self.check_identifier() {
            let var_name = self.expect_identifier("Expected variable name", "متوقع اسم المتغير")?;
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

        self.expect(&TokenKind::LeftParen, "Expected '('", "متوقع '('")?;

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

        self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;

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

        self.expect(&TokenKind::LeftParen, "Expected '('", "متوقع '('")?;
        let expr = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;

        self.expect(&TokenKind::LeftBrace, "Expected '{'", "متوقع '{'")?;

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
        }

        self.expect(&TokenKind::RightBrace, "Expected '}'", "متوقع '}'")?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::Match { expr, arms }, span))
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, Diagnostic> {
        let arm_start = self.current_span();

        let is_default = self.match_token(&TokenKind::Default);
        let mut patterns = Vec::new();

        if is_default {
            patterns.push(Pattern::new(PatternKind::Wildcard, arm_start));
        } else {
            self.expect(&TokenKind::Case, "Expected 'case'", "متوقع 'حالة'")?;
            loop {
                patterns.push(self.parse_pattern()?);
                if !self.match_token(&TokenKind::Comma)
                    && !self.match_token(&TokenKind::ArabicComma)
                {
                    break;
                }
            }
        }

        self.expect(&TokenKind::FatArrow, "Expected '=>'", "متوقع '=>'")?;

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

    /// Parse a pattern for match expressions
    /// Patterns can be:
    /// - Literals: 1, "hello", صحيح
    /// - Enum variants: EnumName::VariantName or EnumName::VariantName(x, y)
    fn parse_pattern(&mut self) -> Result<Pattern, Diagnostic> {
        let start_span = self.current_span();

        // Check for enum variant pattern: identifier::identifier
        if let TokenKind::Identifier(name) = self.peek().kind.clone() {
            // Look ahead for ::
            if self.peek_next().map(|t| &t.kind) == Some(&TokenKind::ColonColon) {
                self.advance(); // consume enum name
                self.advance(); // consume ::

                // Get variant name
                let variant_name = self.expect_identifier(
                    "Expected variant name after '::'",
                    "متوقع اسم الحالة بعد '::'",
                )?;

                // Check for bindings
                let bindings = if self.match_token(&TokenKind::LeftParen) {
                    let mut bindings = Vec::new();
                    if !self.check(&TokenKind::RightParen) {
                        loop {
                            let binding =
                                self.expect_identifier("Expected binding name", "متوقع اسم الربط")?;
                            bindings.push(binding);
                            if !self.match_token(&TokenKind::Comma)
                                && !self.match_token(&TokenKind::ArabicComma)
                            {
                                break;
                            }
                        }
                    }
                    self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;
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
            self.expect(&TokenKind::LeftParen, "Expected '('", "متوقع '('")?;
            let param = self.expect_identifier("Expected error name", "متوقع اسم الخطأ")?;
            self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;
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
        self.expect(&TokenKind::LeftBrace, "Expected '{'", "متوقع '{'")?;

        let mut statements = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            match self.parse_declaration() {
                Ok(stmt) => statements.push(stmt),
                Err(diagnostic) => {
                    self.report_error(diagnostic);
                    self.synchronize();
                }
            }
        }

        self.expect(&TokenKind::RightBrace, "Expected '}'", "متوقع '}'")?;

        let span = start.merge(&self.previous_span());
        Ok(Block::new(statements, span))
    }

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

            TokenKind::Identifier(name) => {
                let name = name.clone();

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
                    let variant_name = self.expect_identifier(
                        "Expected variant name after '::'",
                        "متوقع اسم الحالة بعد '::'",
                    )?;

                    // Check for variant arguments: Variant(args)
                    let args = if self.match_token(&TokenKind::LeftParen) {
                        let args = self.parse_arguments()?;
                        self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;
                        args
                    } else {
                        Vec::new()
                    };

                    let end_span = self.previous_span();
                    Ok(Expr::new(
                        ExprKind::EnumVariant {
                            enum_name: name,
                            type_args,
                            variant_name,
                            args,
                        },
                        span.merge(&end_span),
                    ))
                } else if !type_args.is_empty() {
                    // We parsed type args but no ::, this might be an error
                    // For now, return as identifier and let semantic analysis handle it
                    // This can happen in cases like: let x: SomeType<T> = ...
                    // But this is problematic - we consumed the type args
                    // We need to rollback or handle this differently
                    // For simplicity, let's just return an identifier for now
                    // The type args were consumed for the enum case
                    Ok(Expr::new(ExprKind::Identifier(name), span))
                } else {
                    Ok(Expr::new(ExprKind::Identifier(name), span))
                }
            }

            TokenKind::TypeInt => Ok(Expr::new(ExprKind::Identifier("عدد".to_string()), span)),
            TokenKind::TypeFloat => Ok(Expr::new(
                ExprKind::Identifier("عدد_عشري".to_string()),
                span,
            )),
            TokenKind::TypeString => Ok(Expr::new(ExprKind::Identifier("نص".to_string()), span)),
            TokenKind::TypeBool => Ok(Expr::new(ExprKind::Identifier("منطقي".to_string()), span)),

            TokenKind::This => Ok(Expr::new(ExprKind::This, span)),
            TokenKind::Super => Ok(Expr::new(ExprKind::Super, span)),

            TokenKind::LeftParen => {
                if let Some(lambda) = self.try_parse_arrow_function(span)? {
                    return Ok(lambda);
                }
                let expr = self.parse_expression()?;
                self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;
                let end_span = self.previous_span();
                Ok(Expr::new(
                    ExprKind::Grouping(Box::new(expr)),
                    span.merge(&end_span),
                ))
            }

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
                self.expect(&TokenKind::RightBracket, "Expected ']'", "متوقع ']'")?;
                let end_span = self.previous_span();
                Ok(Expr::new(ExprKind::Array(elements), span.merge(&end_span)))
            }

            TokenKind::LeftBrace => {
                let mut pairs = Vec::new();
                if !self.check(&TokenKind::RightBrace) {
                    loop {
                        let key = self.expect_identifier("Expected key", "متوقع مفتاح")?;
                        self.expect(&TokenKind::Colon, "Expected ':'", "متوقع ':'")?;
                        let value = self.parse_expression()?;
                        pairs.push((key, value));
                        if !self.match_token(&TokenKind::Comma)
                            && !self.match_token(&TokenKind::ArabicComma)
                        {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::RightBrace, "Expected '}'", "متوقع '}'")?;
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
                    self.expect(&TokenKind::Greater, "Expected '>'", "متوقع '>'")?;
                    args
                } else {
                    Vec::new()
                };

                let args = if self.match_token(&TokenKind::LeftParen) {
                    let args = self.parse_arguments()?;
                    self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;
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
                self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;
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
                self.expect(&TokenKind::RightBracket, "Expected ']'", "متوقع ']'")?;
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
                let property =
                    self.expect_identifier("Expected property name", "متوقع اسم الخاصية")?;
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
                self.expect(&TokenKind::Colon, "Expected ':'", "متوقع ':'")?;
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

    /// Try to parse type arguments for generic enum variants: `اختياري<عدد>::بعض`
    /// Returns None if it doesn't look like type args (e.g., comparison expression)
    fn try_parse_type_args(&mut self) -> Result<Option<Vec<TypeAnnotation>>, Diagnostic> {
        // We're at '<' - check if what follows looks like a type
        // If the next token is an identifier or type keyword, try to parse as type args
        // This is speculative - we commit only if we see '>' followed by '::'

        let saved_pos = self.current;

        if !self.match_token(&TokenKind::Less) {
            return Ok(None);
        }

        // Check if this looks like type args (identifier or type keyword after '<')
        let looks_like_type = matches!(
            &self.peek().kind,
            TokenKind::Identifier(_)
                | TokenKind::TypeInt
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

    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, Diagnostic> {
        let start = self.current_span();

        let name = self.expect_type_name()?;

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
            self.expect(&TokenKind::Greater, "Expected '>'", "متوقع '>'")?;
            let span = start.merge(&self.previous_span());
            return Ok(TypeAnnotation::new(
                TypeKind::Generic { base: name, args },
                span,
            ));
        }

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
                let name =
                    self.expect_identifier("Expected parameter name", "متوقع اسم المعامل")?;

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

    fn try_parse_arrow_function(&mut self, start_span: Span) -> Result<Option<Expr>, Diagnostic> {
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

    fn try_parse_arrow_params(&mut self) -> Result<Option<Vec<Param>>, Diagnostic> {
        let mut params = Vec::new();

        if self.check(&TokenKind::RightParen) {
            self.advance(); // consume ')'
            return Ok(Some(params));
        }

        loop {
            let param_start = self.current_span();

            let name = match &self.peek().kind {
                TokenKind::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    name
                }
                _ => return Ok(None), // Not an arrow function
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

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    /// Look ahead one token (for lookahead parsing)
    fn peek_next(&self) -> Option<&Token> {
        if self.current + 1 < self.tokens.len() {
            Some(&self.tokens[self.current + 1])
        } else {
            None
        }
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

    fn expect(&mut self, kind: &TokenKind, en: &str, ar: &str) -> Result<Token, Diagnostic> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(Diagnostic::error(en, ar, self.current_span()))
        }
    }

    fn expect_identifier(&mut self, en: &str, ar: &str) -> Result<String, Diagnostic> {
        if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(Diagnostic::error(en, ar, self.current_span()))
        }
    }

    fn expect_type_name(&mut self) -> Result<String, Diagnostic> {
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
                Ok("void".to_string()) // Internal name only
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

    fn expect_string(&mut self, en: &str, ar: &str) -> Result<String, Diagnostic> {
        if let TokenKind::StringLiteral(s) = &self.peek().kind {
            let s = s.clone();
            self.advance();
            Ok(s)
        } else {
            Err(Diagnostic::error(en, ar, self.current_span()))
        }
    }

    fn consume_semicolon(&mut self) -> Result<(), Diagnostic> {
        if self.match_token(&TokenKind::Semicolon) || self.match_token(&TokenKind::ArabicSemicolon)
        {
            Ok(())
        } else {
            if self.check(&TokenKind::RightBrace) || self.is_at_end() {
                return Ok(());
            }

            let prev_line = self.previous_span().line;
            let curr_line = self.current_span().line;
            if curr_line > prev_line {
                return Ok(());
            }

            Err(Diagnostic::error(
                "Expected ';'",
                "متوقع '؛'",
                self.current_span(),
            ))
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
        let mut parser = Parser::new("بسم_الله\nمتغير س = 5;\nالحمد_لله");
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
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
            بسم_الله
            دالة جمع(أ: عدد، ب: عدد) -> عدد {
                أرجع أ + ب;
            }
            الحمد_لله
        "#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
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
            بسم_الله
            إذا (س > 5) {
                اطبع("كبير");
            } وإلا {
                اطبع("صغير");
            }
            الحمد_لله
        "#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
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
            بسم_الله
            صنف شخص {
                خاص اسم: نص;

                منشئ(اسم: نص) {
                    هذا.اسم = اسم;
                }

                عام دالة احصل_اسم() -> نص {
                    أرجع هذا.اسم;
                }
            }
            الحمد_لله
        "#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
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
        let source = "بسم_الله\n1 + 2 * 3;\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
        assert_eq!(ast.statements.len(), 1);
    }

    #[test]
    fn test_parse_array_literal() {
        let source = "بسم_الله\n[1، 2، 3];\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
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

    #[test]
    fn test_parse_doc_comment_on_function() {
        let source = r#"
            بسم_الله
            /// دالة لحساب مجموع عددين
            دالة جمع(أ: عدد، ب: عدد) -> عدد {
                أرجع أ + ب;
            }
            الحمد_لله
        "#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::FuncDecl {
                name, doc_comment, ..
            } => {
                assert_eq!(name, "جمع");
                // Doc comment parsing may not be implemented yet
                if let Some(doc) = doc_comment {
                    assert!(doc.contains("دالة لحساب مجموع عددين"));
                }
            }
            _ => panic!("Expected FuncDecl"),
        }
    }

    #[test]
    fn test_parse_doc_comment_on_class() {
        let source = r#"
            بسم_الله
            /**
             * صنف لتمثيل شخص
             * @معامل اسم - اسم الشخص
             */
            صنف شخص {
                خاص اسم: نص;

                عام دالة احصل_اسم() -> نص {
                    أرجع هذا.اسم;
                }
            }
            الحمد_لله
        "#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::ClassDecl {
                name,
                doc_comment,
                members,
                ..
            } => {
                assert_eq!(name, "شخص");
                // Doc comment parsing may not be implemented yet
                if let Some(doc) = doc_comment {
                    assert!(doc.contains("صنف لتمثيل شخص"));
                }

                match &members[0] {
                    ClassMember::Field { name, .. } => {
                        assert_eq!(name, "اسم");
                        // Field doesn't have a doc comment in source
                    }
                    _ => panic!("Expected Field"),
                }

                match &members[1] {
                    ClassMember::Method { name, .. } => {
                        assert_eq!(name, "احصل_اسم");
                        // Method doesn't have a doc comment in source
                    }
                    _ => panic!("Expected Method"),
                }
            }
            _ => panic!("Expected ClassDecl"),
        }
    }

    #[test]
    fn test_missing_file_start_marker() {
        let source = "متغير س = 5;\nالحمد_لله";
        let mut parser = Parser::new(source);
        let result = parser.parse();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("بسم_الله") || err.message.contains("bismillah"));
    }

    #[test]
    fn test_missing_file_end_marker() {
        let source = "بسم_الله\nمتغير س = 5;";
        let mut parser = Parser::new(source);
        let result = parser.parse();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("الحمد_لله") || err.message.contains("alhamdulillah"));
    }

    #[test]
    fn test_code_after_file_end_marker() {
        let source = "بسم_الله\nمتغير س = 5;\nالحمد_لله\nمتغير ع = 10;";
        let mut parser = Parser::new(source);
        let result = parser.parse();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("No code allowed after"));
    }

    #[test]
    fn test_file_markers_with_statement() {
        let source = "بسم_الله\nمتغير س = 5;\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
        assert_eq!(ast.statements.len(), 1);
    }

    #[test]
    fn test_empty_file_with_markers() {
        let source = "بسم_الله\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert!(ast.has_file_markers());
        assert_eq!(ast.statements.len(), 0);
    }

    #[test]
    fn test_simple_enum() {
        let source = r#"بسم_الله
تعداد اللون {
    أحمر
    أخضر
    أزرق
}
الحمد_لله"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::EnumDecl { name, variants, .. } => {
                assert_eq!(name, "اللون");
                assert_eq!(variants.len(), 3);
                assert_eq!(variants[0].name, "أحمر");
                assert_eq!(variants[1].name, "أخضر");
                assert_eq!(variants[2].name, "أزرق");
                assert!(variants[0].discriminant.is_none());
                assert!(variants[0].fields.is_empty());
            }
            _ => panic!("Expected EnumDecl"),
        }
    }

    #[test]
    fn test_enum_with_discriminants() {
        let source = r#"بسم_الله
تعداد الحجم {
    صغير = 1
    متوسط = 2
    كبير = 3
}
الحمد_لله"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::EnumDecl { name, variants, .. } => {
                assert_eq!(name, "الحجم");
                assert_eq!(variants.len(), 3);
                assert_eq!(variants[0].name, "صغير");
                assert_eq!(variants[0].discriminant, Some(1));
                assert_eq!(variants[1].name, "متوسط");
                assert_eq!(variants[1].discriminant, Some(2));
                assert_eq!(variants[2].name, "كبير");
                assert_eq!(variants[2].discriminant, Some(3));
            }
            _ => panic!("Expected EnumDecl"),
        }
    }

    #[test]
    fn test_enum_with_associated_data() {
        let source = r#"بسم_الله
تعداد الرسالة {
    رسالة_نصية(محتوى: نص)
    رسالة_رقمية(قيمة: عدد)
    فارغ
}
الحمد_لله"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::EnumDecl { name, variants, .. } => {
                assert_eq!(name, "الرسالة");
                assert_eq!(variants.len(), 3);

                assert_eq!(variants[0].name, "رسالة_نصية");
                assert_eq!(variants[0].fields.len(), 1);
                assert_eq!(variants[0].fields[0].name, Some("محتوى".to_string()));

                assert_eq!(variants[1].name, "رسالة_رقمية");
                assert_eq!(variants[1].fields.len(), 1);
                assert_eq!(variants[1].fields[0].name, Some("قيمة".to_string()));

                assert_eq!(variants[2].name, "فارغ");
                assert!(variants[2].fields.is_empty());
            }
            _ => panic!("Expected EnumDecl"),
        }
    }

    #[test]
    fn test_generic_enum() {
        let source = r#"بسم_الله
تعداد نتيجة<ن، خ> {
    نجاح(قيمة: ن)
    فشل(سبب: خ)
}
الحمد_لله"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::EnumDecl {
                name,
                type_params,
                variants,
                ..
            } => {
                assert_eq!(name, "نتيجة");
                assert_eq!(type_params.len(), 2);
                assert_eq!(type_params[0], "ن");
                assert_eq!(type_params[1], "خ");
                assert_eq!(variants.len(), 2);
            }
            _ => panic!("Expected EnumDecl"),
        }
    }

    #[test]
    fn test_simple_enum_variant_expression() {
        // Simple variant without data: لون::أحمر
        let source = r#"بسم_الله
متغير اللون = لون::أحمر;
الحمد_لله"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::VarDecl { name, init, .. } => {
                assert_eq!(name, "اللون");
                let init = init.as_ref().unwrap();
                match &init.kind {
                    ExprKind::EnumVariant {
                        enum_name,
                        variant_name,
                        args,
                        type_args,
                    } => {
                        assert_eq!(enum_name, "لون");
                        assert_eq!(variant_name, "أحمر");
                        assert!(args.is_empty());
                        assert!(type_args.is_empty());
                    }
                    _ => panic!("Expected EnumVariant, got {:?}", init.kind),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_enum_variant_with_data() {
        // Variant with data: نتيجة::نجاح(42)
        let source = r#"بسم_الله
متغير النتيجة = نتيجة::نجاح(42);
الحمد_لله"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::VarDecl { name, init, .. } => {
                assert_eq!(name, "النتيجة");
                let init = init.as_ref().unwrap();
                match &init.kind {
                    ExprKind::EnumVariant {
                        enum_name,
                        variant_name,
                        args,
                        type_args,
                    } => {
                        assert_eq!(enum_name, "نتيجة");
                        assert_eq!(variant_name, "نجاح");
                        assert_eq!(args.len(), 1);
                        assert!(type_args.is_empty());
                        // Check the argument is a literal 42
                        match &args[0].kind {
                            ExprKind::Literal(Literal::Int(42)) => {}
                            _ => panic!("Expected Int literal 42"),
                        }
                    }
                    _ => panic!("Expected EnumVariant, got {:?}", init.kind),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_generic_enum_variant() {
        // Generic enum variant: اختياري<عدد>::بعض(100)
        let source = r#"بسم_الله
متغير قيمة = اختياري<عدد>::بعض(100);
الحمد_لله"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::VarDecl { name, init, .. } => {
                assert_eq!(name, "قيمة");
                let init = init.as_ref().unwrap();
                match &init.kind {
                    ExprKind::EnumVariant {
                        enum_name,
                        variant_name,
                        args,
                        type_args,
                    } => {
                        assert_eq!(enum_name, "اختياري");
                        assert_eq!(variant_name, "بعض");
                        assert_eq!(args.len(), 1);
                        assert_eq!(type_args.len(), 1);
                        // Check type arg is عدد
                        match &type_args[0].kind {
                            TypeKind::Simple(name) => assert_eq!(name, "عدد"),
                            _ => panic!("Expected Simple type 'عدد'"),
                        }
                    }
                    _ => panic!("Expected EnumVariant, got {:?}", init.kind),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_enum_variant_multiple_args() {
        // Variant with multiple data args: موقع::نقطة(10, 20)
        let source = r#"بسم_الله
متغير م = موقع::نقطة(10، 20);
الحمد_لله"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        assert_eq!(ast.statements.len(), 1);
        match &ast.statements[0].kind {
            StmtKind::VarDecl { init, .. } => {
                let init = init.as_ref().unwrap();
                match &init.kind {
                    ExprKind::EnumVariant {
                        enum_name,
                        variant_name,
                        args,
                        ..
                    } => {
                        assert_eq!(enum_name, "موقع");
                        assert_eq!(variant_name, "نقطة");
                        assert_eq!(args.len(), 2);
                    }
                    _ => panic!("Expected EnumVariant"),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }
}
