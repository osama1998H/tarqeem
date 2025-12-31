//! Declaration parsing for the Tarqeem parser.
//!
//! This module handles parsing of declarations including variables, functions,
//! classes, interfaces, enums, and imports/exports.

use crate::error::codes::{
    ERR_EXPECTED_CLASS_NAME, ERR_EXPECTED_FUNCTION_NAME, ERR_EXPECTED_VARIABLE_NAME,
};
use crate::error::Diagnostic;
use crate::lexer::TokenKind;

use super::super::ast::*;
use super::Parser;

impl Parser {
    /// Parse a declaration (variable, function, class, etc.).
    pub(crate) fn parse_declaration(&mut self) -> Result<Stmt, Diagnostic> {
        // Collect any line comments before the declaration
        self.collect_line_comments();
        let leading_comments = self.take_pending_comments();

        let doc_comment = self.consume_doc_comment();

        // Skip newlines after doc comment before the actual declaration
        self.skip_newlines();

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

        // Capture trailing comment (on same line after statement)
        stmt.trailing_comment = self.capture_trailing_comment();

        // Attach leading comments to the statement
        stmt.leading_comments = leading_comments;
        Ok(stmt)
    }

    /// Parse a variable declaration.
    pub(crate) fn parse_var_declaration(
        &mut self,
        doc_comment: Option<String>,
    ) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        let mutable = self.check(&TokenKind::Let);
        self.advance(); // consume 'let' or 'const'

        let name = self
            .expect_identifier("متوقع اسم المتغير")
            .map_err(|e| e.with_code(ERR_EXPECTED_VARIABLE_NAME.to_string()))?;

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

    /// Parse a function declaration.
    pub(crate) fn parse_function_declaration(
        &mut self,
        is_async: bool,
        doc_comment: Option<String>,
    ) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.expect(&TokenKind::Function, "متوقع 'دالة'")?;

        let name = self
            .expect_identifier("متوقع اسم الدالة")
            .map_err(|e| e.with_code(ERR_EXPECTED_FUNCTION_NAME.to_string()))?;

        self.expect(&TokenKind::LeftParen, "متوقع '('")?;
        let params = self.parse_parameters()?;
        self.expect(&TokenKind::RightParen, "متوقع ')'")?;

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

    /// Parse a class declaration.
    pub(crate) fn parse_class_declaration(
        &mut self,
        doc_comment: Option<String>,
    ) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'class'

        let name = self
            .expect_identifier("متوقع اسم الصنف")
            .map_err(|e| e.with_code(ERR_EXPECTED_CLASS_NAME.to_string()))?;

        let type_params = self.parse_type_parameters()?;

        let extends = if self.check(&TokenKind::Extends) {
            self.advance();
            Some(self.expect_identifier("متوقع اسم الصنف الأب")?)
        } else {
            None
        };

        let mut implements = Vec::new();
        if self.check(&TokenKind::Implements) {
            self.advance();
            loop {
                let interface_name = self.expect_identifier("متوقع اسم الميثاق")?;
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
                    self.expect(&TokenKind::Greater, "متوقع '>'")?;
                }

                if !self.match_token(&TokenKind::Comma)
                    && !self.match_token(&TokenKind::ArabicComma)
                {
                    break;
                }
            }
        }

        self.expect(&TokenKind::LeftBrace, "متوقع '{'")?;
        let members = self.parse_class_members()?;
        self.expect(&TokenKind::RightBrace, "متوقع '}'")?;

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

    /// Parse type parameters (e.g., <T, U>).
    pub(crate) fn parse_type_parameters(&mut self) -> Result<Vec<String>, Diagnostic> {
        let mut params = Vec::new();

        if !self.check(&TokenKind::Less) {
            return Ok(params);
        }

        self.advance(); // consume '<'

        loop {
            let param = self.expect_identifier("متوقع اسم معامل النوع")?;
            params.push(param);

            if !self.match_token(&TokenKind::Comma) && !self.match_token(&TokenKind::ArabicComma) {
                break;
            }
        }

        self.expect(&TokenKind::Greater, "متوقع '>'")?;
        Ok(params)
    }

    /// Parse class members.
    pub(crate) fn parse_class_members(&mut self) -> Result<Vec<ClassMember>, Diagnostic> {
        let mut members = Vec::new();

        // Skip initial newlines
        self.skip_newlines();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            match self.parse_class_member() {
                Ok(member) => members.push(member),
                Err(diagnostic) => {
                    self.report_error(diagnostic);
                    self.synchronize_to_member();
                }
            }
            // Skip newlines after each member
            self.skip_newlines();
        }

        Ok(members)
    }

    /// Parse a single class member.
    pub(crate) fn parse_class_member(&mut self) -> Result<ClassMember, Diagnostic> {
        self.collect_line_comments();
        let member_doc = self.consume_doc_comment();

        let visibility = self.parse_visibility();
        let is_static = self.match_token(&TokenKind::Static);

        if self.check(&TokenKind::Constructor) {
            self.advance();
            self.expect(&TokenKind::LeftParen, "متوقع '('")?;
            let params = self.parse_parameters()?;
            self.expect(&TokenKind::RightParen, "متوقع ')'")?;
            let body = self.parse_block()?;
            Ok(ClassMember::Constructor {
                params,
                body,
                doc_comment: member_doc,
            })
        } else if self.check(&TokenKind::Function) || self.check(&TokenKind::Async) {
            let is_async = self.match_token(&TokenKind::Async);
            self.expect(&TokenKind::Function, "متوقع 'دالة'")?;
            let name = self.expect_identifier("متوقع اسم الدالة")?;
            self.expect(&TokenKind::LeftParen, "متوقع '('")?;
            let params = self.parse_parameters()?;
            self.expect(&TokenKind::RightParen, "متوقع ')'")?;

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
            let name = self.expect_identifier("متوقع اسم الخاصية")?;
            self.expect(&TokenKind::Colon, "متوقع ':'")?;
            let ty = self.parse_type_annotation()?;

            let (accessors, default_value) = if self.match_token(&TokenKind::LeftBrace) {
                let accessors = self.parse_property_accessors()?;
                self.expect(&TokenKind::RightBrace, "متوقع '}'")?;
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
            let name = self.expect_identifier("متوقع اسم الحقل")?;
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

    /// Parse property accessors (get/set).
    pub(crate) fn parse_property_accessors(&mut self) -> Result<Vec<PropertyAccessor>, Diagnostic> {
        let mut accessors = Vec::new();

        // Skip initial newlines
        self.skip_newlines();

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
                    let name = self.expect_identifier("متوقع اسم المعامل")?;
                    self.expect(&TokenKind::RightParen, "متوقع ')'")?;
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
                    "متوقع 'احصل' أو 'عيّن'",
                    self.current_span(),
                ));
            }
            // Skip newlines after each accessor
            self.skip_newlines();
        }

        Ok(accessors)
    }

    /// Parse visibility modifier.
    pub(crate) fn parse_visibility(&mut self) -> Visibility {
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

    /// Parse an interface declaration.
    pub(crate) fn parse_interface_declaration(
        &mut self,
        doc_comment: Option<String>,
    ) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'interface'

        let name = self.expect_identifier("متوقع اسم الميثاق")?;

        let type_params = self.parse_type_parameters()?;

        self.expect(&TokenKind::LeftBrace, "متوقع '{'")?;

        // Skip initial newlines
        self.skip_newlines();

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Skip any line comments before the method
            self.collect_line_comments();
            if self.check(&TokenKind::RightBrace) {
                self.take_pending_comments();
                break;
            }
            let method_doc = self.consume_doc_comment();

            self.expect(&TokenKind::Function, "متوقع 'دالة'")?;
            let method_name = self.expect_identifier("متوقع اسم الدالة")?;
            self.expect(&TokenKind::LeftParen, "متوقع '('")?;
            let params = self.parse_parameters()?;
            self.expect(&TokenKind::RightParen, "متوقع ')'")?;

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
            // Skip newlines after each method
            self.skip_newlines();
        }

        self.expect(&TokenKind::RightBrace, "متوقع '}'")?;

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

    /// Parse an enum declaration.
    pub(crate) fn parse_enum_declaration(
        &mut self,
        doc_comment: Option<String>,
    ) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'تعداد'

        let name = self.expect_identifier("متوقع اسم التعداد")?;

        let type_params = self.parse_type_parameters()?;

        self.expect(&TokenKind::LeftBrace, "متوقع '{'")?;

        // Skip initial newlines
        self.skip_newlines();

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
            // Skip newlines after each variant
            self.skip_newlines();
        }

        self.expect(&TokenKind::RightBrace, "متوقع '}'")?;

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

    /// Parse an enum variant.
    pub(crate) fn parse_enum_variant(&mut self) -> Result<EnumVariant, Diagnostic> {
        let start = self.current_span();

        let variant_doc = self.consume_doc_comment();

        let name = self.expect_identifier("متوقع اسم الحالة")?;

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

    /// Parse enum variant fields.
    pub(crate) fn parse_enum_variant_fields(
        &mut self,
    ) -> Result<Vec<EnumVariantField>, Diagnostic> {
        self.advance(); // consume '('

        let mut fields = Vec::new();

        if !self.check(&TokenKind::RightParen) {
            loop {
                let field_start = self.current_span();

                let (name, ty) = if self.check_identifier() {
                    let first = self.expect_identifier("متوقع اسم الحقل أو النوع")?;

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

        self.expect(&TokenKind::RightParen, "متوقع ')'")?;
        Ok(fields)
    }

    /// Parse an import statement.
    pub(crate) fn parse_import_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'import'

        let items = if self.match_token(&TokenKind::Star) {
            self.expect(&TokenKind::As, "متوقع 'كـ'")?;
            let alias = self.expect_identifier("متوقع اسم مستعار")?;
            ImportItems::Wildcard(alias)
        } else if self.match_token(&TokenKind::LeftBrace) {
            let mut items = Vec::new();
            loop {
                let name = self.expect_identifier("متوقع اسم")?;
                let alias = if self.match_token(&TokenKind::As) {
                    Some(self.expect_identifier("متوقع اسم مستعار")?)
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
            self.expect(&TokenKind::RightBrace, "متوقع '}'")?;
            ImportItems::Named(items)
        } else {
            let name = self.expect_identifier("متوقع استيراد افتراضي")?;
            ImportItems::Default(name)
        };

        self.expect(&TokenKind::From, "متوقع 'من'")?;
        let from = self.expect_string("متوقع مسار الوحدة")?;

        self.consume_semicolon()?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::Import { items, from }, span))
    }

    /// Parse an export statement.
    pub(crate) fn parse_export_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'export'

        let stmt = self.parse_declaration()?;
        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::Export(Box::new(stmt)), span))
    }

    /// Parse a type annotation.
    pub(crate) fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, Diagnostic> {
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
            self.expect(&TokenKind::Greater, "متوقع '>'")?;
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

    /// Parse function parameters.
    pub(crate) fn parse_parameters(&mut self) -> Result<Vec<Param>, Diagnostic> {
        let mut params = Vec::new();

        if !self.check(&TokenKind::RightParen) {
            loop {
                let start = self.current_span();
                let name = self.expect_identifier("متوقع اسم المعامل")?;

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
}
