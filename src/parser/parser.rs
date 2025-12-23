//! Recursive descent parser for Tarqeem

use super::ast::*;
use super::precedence::Precedence;
use crate::error::{Diagnostic, Span};
use crate::lexer::{Lexer, Token, TokenKind};

/// The Tarqeem parser
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    /// Collected errors during parsing (for error recovery)
    errors: Vec<Diagnostic>,
    /// Whether we're in panic mode (recovering from an error)
    panic_mode: bool,
}

impl Parser {
    /// Create a new parser from source code
    pub fn new(source: &str) -> Self {
        let mut lexer = Lexer::new(source);
        let tokens: Vec<Token> = lexer
            .tokenize()
            .into_iter()
            // Filter out newlines but keep doc comments
            .filter(|t| !matches!(t.kind, TokenKind::Newline))
            .collect();

        Self {
            tokens,
            current: 0,
            errors: Vec::new(),
            panic_mode: false,
        }
    }

    /// Create a new parser from pre-lexed tokens
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
        }
    }

    /// Synchronize after a parse error by skipping to the next statement boundary.
    /// This enables parsing to continue and collect multiple errors.
    fn synchronize(&mut self) {
        self.panic_mode = false;

        while !self.is_at_end() {
            // If we just passed a semicolon, we're at a statement boundary
            if self.previous().kind == TokenKind::Semicolon {
                return;
            }

            // If we see a statement-starting keyword, we're at a statement boundary
            match self.peek().kind {
                // Declaration keywords
                TokenKind::Let        // متغير
                | TokenKind::Const    // ثابت
                | TokenKind::Function // دالة
                | TokenKind::Class    // صنف
                | TokenKind::Interface // ميثاق
                | TokenKind::Enum     // تعداد
                // Control flow
                | TokenKind::If       // إذا
                | TokenKind::While    // طالما
                | TokenKind::For      // لكل
                | TokenKind::Do       // افعل
                | TokenKind::Return   // أرجع
                | TokenKind::Try      // حاول
                | TokenKind::Match    // تطابق
                // Module
                | TokenKind::Import   // استورد
                | TokenKind::Export   // صدّر
                // File markers
                | TokenKind::Alhamdulillah => {
                    return;
                }
                _ => {}
            }

            self.advance();
        }
    }

    /// Synchronize to the next class member boundary.
    /// Used for error recovery within class declarations.
    fn synchronize_to_member(&mut self) {
        self.panic_mode = false;

        while !self.is_at_end() {
            // Stop at class member boundaries
            match self.peek().kind {
                // Visibility modifiers start a new member
                TokenKind::Public     // عام
                | TokenKind::Private  // خاص
                | TokenKind::Protected // محمي
                // Static modifier
                | TokenKind::Static   // مشترك
                // Member declarations
                | TokenKind::Function // دالة
                | TokenKind::Async    // غير_متزامن
                | TokenKind::Constructor // منشئ
                // End of class
                | TokenKind::RightBrace => {
                    return;
                }
                // Identifier could be a field declaration
                TokenKind::Identifier(_) => {
                    return;
                }
                _ => {}
            }

            self.advance();
        }
    }

    /// Synchronize to the next match arm boundary.
    /// Used for error recovery within match statements.
    fn synchronize_to_arm(&mut self) {
        self.panic_mode = false;

        while !self.is_at_end() {
            // Stop at match arm boundaries
            match self.peek().kind {
                // Case/default start a new arm
                TokenKind::Case      // حالة
                | TokenKind::Default // غير_ذلك
                // End of match
                | TokenKind::RightBrace => {
                    return;
                }
                _ => {}
            }

            self.advance();
        }
    }

    /// Report an error and enter panic mode.
    /// The error is collected for later reporting.
    fn report_error(&mut self, diagnostic: Diagnostic) {
        if !self.panic_mode {
            self.errors.push(diagnostic);
            self.panic_mode = true;
        }
    }

    /// Get all collected errors
    pub fn get_errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    /// Consume any doc comment token and return its content
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

    /// Parse the entire program
    /// Files must start with بسم_الله (bismillah) and end with الحمد_لله (alhamdulillah)
    /// Uses error recovery to collect multiple errors.
    pub fn parse(&mut self) -> Result<Ast, Diagnostic> {
        // Require file start marker: بسم_الله
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

        // Parse all declarations until we hit الحمد_لله or EOF
        // Use error recovery to collect multiple errors
        while !self.is_at_end() && !self.check(&TokenKind::Alhamdulillah) {
            match self.parse_declaration() {
                Ok(stmt) => {
                    statements.push(stmt);
                }
                Err(diagnostic) => {
                    // Report error and try to recover
                    self.report_error(diagnostic);
                    self.synchronize();
                }
            }
        }

        // Require file end marker: الحمد_لله
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
            // If we have other errors, add this to them
            if !self.errors.is_empty() {
                self.report_error(err);
                // Clone the first error so get_errors() still has all errors
                return Err(self.errors[0].clone());
            }
            return Err(err);
        };

        // Ensure nothing comes after الحمد_لله
        if !self.is_at_end() {
            let err = Diagnostic::error(
                "No code allowed after 'الحمد_لله' (alhamdulillah)",
                "لا يُسمح بأي كود بعد 'الحمد_لله'",
                self.current_span(),
            );
            if !self.errors.is_empty() {
                self.report_error(err);
                // Clone the first error so get_errors() still has all errors
                return Err(self.errors[0].clone());
            }
            return Err(err);
        }

        // If we collected errors during parsing, return the first one
        // All errors remain available via get_errors()
        if !self.errors.is_empty() {
            // Clone the first error so get_errors() still has all errors
            return Err(self.errors[0].clone());
        }

        Ok(Ast::with_markers(
            statements,
            bismillah_span,
            alhamdulillah_span,
        ))
    }

    // ============ Declaration Parsing ============

    fn parse_declaration(&mut self) -> Result<Stmt, Diagnostic> {
        // Capture any doc comment before the declaration
        let doc_comment = self.consume_doc_comment();

        let result = if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) {
            self.parse_var_declaration(doc_comment)
        } else if self.check(&TokenKind::Function) {
            self.parse_function_declaration(false, doc_comment)
        } else if self.check(&TokenKind::Async) {
            self.advance();
            self.parse_function_declaration(true, doc_comment)
        } else if self.check(&TokenKind::Class) {
            self.parse_class_declaration(doc_comment)
        } else if self.check(&TokenKind::Interface) {
            self.parse_interface_declaration(doc_comment)
        } else if self.check(&TokenKind::Enum) {
            self.parse_enum_declaration(doc_comment)
        } else if self.check(&TokenKind::Import) {
            self.parse_import_statement()
        } else if self.check(&TokenKind::Export) {
            self.parse_export_statement()
        } else {
            self.parse_statement()
        };

        result
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

        // Parse optional generic type parameters: <T, U, ...>
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

                // Skip generic type arguments on interface: <T, U, ...>
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

    /// Parse generic type parameters: <T, U, V>
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
            // Use error recovery to collect multiple errors within a class
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

    /// Parse a single class member (field, method, or constructor)
    fn parse_class_member(&mut self) -> Result<ClassMember, Diagnostic> {
        // Capture any doc comment before the member
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
            // Property: خاصية name: type { احصل { ... } عيّن { ... } }
            self.advance();
            let name = self.expect_identifier("Expected property name", "متوقع اسم الخاصية")?;
            self.expect(&TokenKind::Colon, "Expected ':'", "متوقع ':'")?;
            let ty = self.parse_type_annotation()?;

            // Check for accessor block, default value, or auto-property
            let (accessors, default_value) = if self.match_token(&TokenKind::LeftBrace) {
                // Property with accessor block
                let accessors = self.parse_property_accessors()?;
                self.expect(&TokenKind::RightBrace, "Expected '}'", "متوقع '}'")?;
                (accessors, None)
            } else if self.match_token(&TokenKind::Equal) {
                // Auto-property with default value
                let default = self.parse_expression()?;
                self.consume_semicolon()?;
                (Vec::new(), Some(default))
            } else {
                // Auto-property without default
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
            // Field
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

    /// Parse property accessors (احصل and عيّن blocks)
    fn parse_property_accessors(&mut self) -> Result<Vec<PropertyAccessor>, Diagnostic> {
        let mut accessors = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Optional visibility modifier for accessor
            let accessor_visibility = self.parse_visibility();

            if self.match_token(&TokenKind::Get) {
                // احصل block
                let body = if self.match_token(&TokenKind::FatArrow) {
                    // Short form: احصل => expression
                    let expr = self.parse_expression()?;
                    PropertyAccessorBody::Expr(Box::new(expr))
                } else {
                    // Full form: احصل { ... }
                    PropertyAccessorBody::Block(self.parse_block()?)
                };

                accessors.push(PropertyAccessor::Get {
                    visibility: accessor_visibility,
                    body,
                });
            } else if self.match_token(&TokenKind::Set) {
                // عيّن block
                // Optional parameter name: عيّن(قيمة) or just عيّن
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

        // Parse optional generic type parameters: <T, U, ...>
        let type_params = self.parse_type_parameters()?;

        self.expect(&TokenKind::LeftBrace, "Expected '{'", "متوقع '{'")?;

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Capture any doc comment before the method signature
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

    /// Parse enum declaration: تعداد Color { Red, Green = 1, Blue(r: عدد) }
    fn parse_enum_declaration(&mut self, doc_comment: Option<String>) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'تعداد'

        let name = self.expect_identifier("Expected enum name", "متوقع اسم التعداد")?;

        // Parse optional generic type parameters: <T, U, ...>
        let type_params = self.parse_type_parameters()?;

        self.expect(&TokenKind::LeftBrace, "Expected '{'", "متوقع '{'")?;

        let mut variants = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let variant = self.parse_enum_variant()?;
            variants.push(variant);

            // Optional comma between variants
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

    /// Parse a single enum variant
    fn parse_enum_variant(&mut self) -> Result<EnumVariant, Diagnostic> {
        let start = self.current_span();

        // Capture any doc comment before the variant
        let variant_doc = self.consume_doc_comment();

        let name = self.expect_identifier("Expected variant name", "متوقع اسم الحالة")?;

        // Check for explicit discriminant: = 1
        let discriminant = if self.match_token(&TokenKind::Equal) {
            match &self.peek().kind {
                TokenKind::IntLiteral(n) => {
                    let value = *n;
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

        // Check for associated data: (field: type, ...)
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

    /// Parse enum variant fields: (name: type, ...) or (type, ...)
    fn parse_enum_variant_fields(&mut self) -> Result<Vec<EnumVariantField>, Diagnostic> {
        self.advance(); // consume '('

        let mut fields = Vec::new();

        if !self.check(&TokenKind::RightParen) {
            loop {
                let field_start = self.current_span();

                // Check if this is a named field (name: type) or positional (type)
                let (name, ty) = if self.check_identifier() {
                    let first = self.expect_identifier(
                        "Expected field name or type",
                        "متوقع اسم الحقل أو النوع",
                    )?;

                    if self.match_token(&TokenKind::Colon) {
                        // Named field: name: type
                        let ty = self.parse_type_annotation()?;
                        (Some(first), ty)
                    } else {
                        // Positional field: just a type (the first token was the type name)
                        let span = field_start.merge(&self.previous_span());
                        (None, TypeAnnotation::new(TypeKind::Simple(first), span))
                    }
                } else {
                    // Parse a complex type directly
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

    // ============ Statement Parsing ============

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

        // Optional semicolon at end
        let _ = self.match_token(&TokenKind::Semicolon)
            || self.match_token(&TokenKind::ArabicSemicolon);

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::DoWhile { body, condition }, span))
    }

    fn parse_for_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'for'

        // Check for for-in loop
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
                // Backtrack - this is a regular for loop
                self.current -= 1;
            }
        }

        self.expect(&TokenKind::LeftParen, "Expected '('", "متوقع '('")?;

        // Parse init
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
            // Use error recovery to collect multiple errors within a match
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

    /// Parse a single match arm (case or default)
    fn parse_match_arm(&mut self) -> Result<MatchArm, Diagnostic> {
        let arm_start = self.current_span();

        let is_default = self.match_token(&TokenKind::Default);
        let mut patterns = Vec::new();

        if is_default {
            // Default case - no patterns needed
        } else {
            self.expect(&TokenKind::Case, "Expected 'case'", "متوقع 'حالة'")?;
            loop {
                patterns.push(self.parse_expression()?);
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
        } else {
            // Allow single statements (return, break, continue) without braces
            // Check for statement keywords first
            if self.check(&TokenKind::Return)
                || self.check(&TokenKind::Break)
                || self.check(&TokenKind::Continue)
            {
                let stmt = self.parse_statement()?;
                Block::new(vec![stmt], self.previous_span())
            } else {
                // Otherwise parse as expression statement
                let expr = self.parse_expression()?;
                Block::new(
                    vec![Stmt::new(StmtKind::Expr(expr), self.previous_span())],
                    self.previous_span(),
                )
            }
        };

        Ok(MatchArm {
            patterns,
            body,
            span: arm_start.merge(&self.previous_span()),
        })
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
            // Use error recovery to collect multiple errors within a block
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

            // Type keywords as identifiers (for type conversion functions like نص(42), منطقي(1))
            TokenKind::TypeInt => Ok(Expr::new(ExprKind::Identifier("عدد".to_string()), span)),
            TokenKind::TypeFloat => Ok(Expr::new(
                ExprKind::Identifier("عدد_عشري".to_string()),
                span,
            )),
            TokenKind::TypeString => Ok(Expr::new(ExprKind::Identifier("نص".to_string()), span)),
            TokenKind::TypeBool => Ok(Expr::new(ExprKind::Identifier("منطقي".to_string()), span)),

            // this/super
            TokenKind::This => Ok(Expr::new(ExprKind::This, span)),
            TokenKind::Super => Ok(Expr::new(ExprKind::Super, span)),

            // Grouping or arrow function parameters
            TokenKind::LeftParen => {
                // Try to parse as arrow function first
                if let Some(lambda) = self.try_parse_arrow_function(span)? {
                    return Ok(lambda);
                }
                // Fall back to grouping
                let expr = self.parse_expression()?;
                self.expect(&TokenKind::RightParen, "Expected ')'", "متوقع ')'")?;
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
                self.expect(&TokenKind::RightBracket, "Expected ']'", "متوقع ']'")?;
                let end_span = self.previous_span();
                Ok(Expr::new(ExprKind::Array(elements), span.merge(&end_span)))
            }

            // Object literal
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

            // new expression: جديد ClassName(args) or جديد ClassName<T>(args)
            TokenKind::New => {
                // Parse class name at Primary level to avoid parsing args as a call
                let class = self.parse_precedence(Precedence::Primary)?;

                // Parse generic type arguments if present: <T, U, ...>
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

                // Args must follow immediately with parentheses
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

            // Index
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

            // Member access
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

            // Ternary
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
            self.expect(&TokenKind::Greater, "Expected '>'", "متوقع '>'")?;
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

    // ============ Arrow Function Parsing ============

    /// Try to parse an arrow function starting after '('.
    /// Returns Some(Lambda) if successful, None if this is not an arrow function.
    /// The caller has already consumed the '('.
    fn try_parse_arrow_function(&mut self, start_span: Span) -> Result<Option<Expr>, Diagnostic> {
        // Save position for backtracking
        let saved_pos = self.current;

        // Try to parse arrow function parameters
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

        // Check for '=>'
        if !self.check(&TokenKind::FatArrow) {
            self.current = saved_pos;
            return Ok(None);
        }
        self.advance(); // consume '=>'

        // Parse body: either block or expression
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

    /// Try to parse arrow function parameter list.
    /// Returns Some(params) if this looks like arrow function params followed by ')'.
    /// Returns None if this doesn't look like arrow function params.
    fn try_parse_arrow_params(&mut self) -> Result<Option<Vec<Param>>, Diagnostic> {
        let mut params = Vec::new();

        // Empty params: () => ...
        if self.check(&TokenKind::RightParen) {
            self.advance(); // consume ')'
            return Ok(Some(params));
        }

        // Parse comma-separated parameters
        loop {
            let param_start = self.current_span();

            // Parameter must start with an identifier
            let name = match &self.peek().kind {
                TokenKind::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    name
                }
                _ => return Ok(None), // Not an arrow function
            };

            // Optional type annotation
            let ty = if self.check(&TokenKind::Colon) {
                self.advance(); // consume ':'
                match self.parse_type_annotation() {
                    Ok(ty) => Some(ty),
                    Err(_) => return Ok(None),
                }
            } else {
                None
            };

            // Optional default value
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

            // Check for comma or end
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
            // Note: TypeVoid has no keyword - functions default to no return
            // This case kept for internal/future use but lexer won't produce it
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
            // Semicolons are optional at end of blocks
            if self.check(&TokenKind::RightBrace) || self.is_at_end() {
                return Ok(());
            }

            // Automatic semicolon insertion: if current token is on a new line,
            // treat newline as statement terminator (like Go, Kotlin, Swift)
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

        // Should parse as 1 + (2 * 3) due to precedence
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
                assert!(doc_comment.is_some());
                assert!(doc_comment
                    .as_ref()
                    .unwrap()
                    .contains("دالة لحساب مجموع عددين"));
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
                /// اسم الشخص
                خاص اسم: نص;

                /// دالة للحصول على الاسم
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
                assert!(doc_comment.is_some());
                assert!(doc_comment.as_ref().unwrap().contains("صنف لتمثيل شخص"));

                // Check field has doc comment
                match &members[0] {
                    ClassMember::Field {
                        name, doc_comment, ..
                    } => {
                        assert_eq!(name, "اسم");
                        assert!(doc_comment.is_some());
                    }
                    _ => panic!("Expected Field"),
                }

                // Check method has doc comment
                match &members[1] {
                    ClassMember::Method {
                        name, doc_comment, ..
                    } => {
                        assert_eq!(name, "احصل_اسم");
                        assert!(doc_comment.is_some());
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

    // ============ Enum Tests ============

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

                // First variant: رسالة_نصية(محتوى: نص)
                assert_eq!(variants[0].name, "رسالة_نصية");
                assert_eq!(variants[0].fields.len(), 1);
                assert_eq!(variants[0].fields[0].name, Some("محتوى".to_string()));

                // Second variant: رسالة_رقمية(قيمة: عدد)
                assert_eq!(variants[1].name, "رسالة_رقمية");
                assert_eq!(variants[1].fields.len(), 1);
                assert_eq!(variants[1].fields[0].name, Some("قيمة".to_string()));

                // Third variant: فارغ (unit variant)
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
}
