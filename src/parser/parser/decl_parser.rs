//! Declaration parsing for the Tarqeem parser.
//!
//! This module handles parsing of declarations including variables, functions,
//! classes, interfaces, enums, and imports/exports.

use crate::error::codes::{
    ERR_EXPECTED_CLASS_NAME, ERR_EXPECTED_FUNCTION_NAME, ERR_EXPECTED_VARIABLE_NAME,
    ERR_UNEXPECTED_TOKEN,
};
use crate::error::Diagnostic;
use crate::lexer::TokenKind;

use super::super::ast::*;
use super::{within_brackets, Parser};

impl Parser {
    /// Parse a declaration (variable, function, class, etc.).
    pub(crate) fn parse_declaration(&mut self) -> Result<Stmt, Diagnostic> {
        self.parse_declaration_with_doc(None)
    }

    /// Parse a declaration, optionally carrying a doc comment consumed by a
    /// caller that has already passed the point where it appeared.
    ///
    /// `صدّر` needs this: `parse_declaration` consumes the doc comment before it
    /// can tell which declaration follows, then recurses through
    /// `parse_export_statement` — by which point the token is gone and the inner
    /// declaration used to end up with `doc_comment: None`, silently dropping
    /// docs on every `صدّر دالة`/`صدّر صنف` (issue #204). Threading the value
    /// down keeps the AST built once, rather than patching it afterwards.
    pub(crate) fn parse_declaration_with_doc(
        &mut self,
        inherited_doc: Option<String>,
    ) -> Result<Stmt, Diagnostic> {
        // Consume the whole trivia run in one pass; the doc it holds is only
        // moved into `trivia.comments` if nothing below can carry it, so a
        // demotion lands back on the line the user wrote it on.
        let mut trivia = self.collect_leading_trivia();
        let pending_comments = self.take_pending_comments();

        // A doc comment written before the outer keyword documents the
        // declaration, so it wins over one found after it — and the loser is
        // demoted rather than dropped, since `fmt -w` would otherwise erase it.
        let inherited_wins = inherited_doc.is_some();
        let inherited_for_demotion = inherited_doc.clone();
        let doc_comment = if inherited_wins {
            trivia.demote_doc();
            inherited_doc
        } else {
            trivia.doc.clone()
        };
        // An inherited doc was written *above* this run, so it cannot simply be
        // appended to it — its provenance is what places it.
        let mut demoted_inherited: Option<String> = None;

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
            // Neither of these owns a `doc_comment` field, so the text would be
            // dropped — and since the doc token is consumed now, `fmt -w` would
            // erase it from the user's file rather than failing loudly. Demote it
            // to a leading comment so the formatter still writes it out.
            if inherited_wins {
                demoted_inherited = inherited_for_demotion;
            } else {
                trivia.demote_doc();
            }
            self.parse_import_statement()?
        } else if self.check(&TokenKind::Export) {
            // `صدّر <إعلان>` threads the doc into the inner declaration, but a
            // re-export (`صدّر *` / `صدّر { … }`) has nowhere to put it, so it
            // is demoted here for the same reason as `استورد` above — otherwise
            // `fmt -w` deletes the line from the user's file.
            if self.export_is_reexport() {
                if inherited_wins {
                    demoted_inherited = inherited_for_demotion;
                } else {
                    trivia.demote_doc();
                }
                self.parse_export_statement(None)?
            } else {
                self.parse_export_statement(doc_comment)?
            }
        } else {
            if inherited_wins {
                demoted_inherited = inherited_for_demotion;
            } else {
                trivia.demote_doc();
            }
            self.parse_statement()?
        };

        // Capture trailing comment (on same line after statement)
        stmt.trailing_comment = self.capture_trailing_comment();

        // Source order: comments collected before this call, then a doc written
        // above the outer keyword, then this run — whose own demoted doc is
        // already back in its slot.
        let mut leading_comments = pending_comments;
        leading_comments.extend(demoted_inherited);
        leading_comments.extend(trivia.comments);
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
            .expect_declaration_name("متوقع اسم المتغير")
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
            .expect_declaration_name("متوقع اسم الدالة")
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
            // A member-less tail of comments before '}' is not the start of
            // another member; break before parse_class_member() mistakes a
            // doc/block comment for a field name.
            if self.match_terminator_after_trivia(&TokenKind::RightBrace) {
                break;
            }
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
        let trivia = self.collect_leading_trivia();
        // Captured before the body is parsed so nothing downstream (e.g. the
        // method's own parse_block) can steal it — pending_comments is a
        // single shared buffer, not scoped to this member.
        let mut leading_comments = self.take_pending_comments();
        leading_comments.extend(trivia.comments);
        let member_doc = trivia.doc;

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
                leading_comments,
                doc_comment: member_doc,
            })
        } else if self.check(&TokenKind::Function) || self.check(&TokenKind::Async) {
            let is_async = self.match_token(&TokenKind::Async);
            self.expect(&TokenKind::Function, "متوقع 'دالة'")?;
            let name = self.expect_declaration_name("متوقع اسم الدالة")?;
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
                leading_comments,
                doc_comment: member_doc,
            })
        } else if self.check(&TokenKind::Property) {
            self.advance();
            let name = self.expect_declaration_name("متوقع اسم الخاصية")?;
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
                leading_comments,
                doc_comment: member_doc,
            })
        } else {
            let name = self.expect_declaration_name("متوقع اسم الحقل")?;
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
                leading_comments,
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
            // A trailing comment before '}' is not the start of another
            // accessor; break before parse_visibility()/match_token() below
            // mistake it for one and report a spurious "expected احصل/عيّن".
            if self.match_terminator_after_trivia(&TokenKind::RightBrace) {
                break;
            }
            self.collect_line_comments();
            // No field exists to preserve a leading comment here (0 real
            // occurrences in the corpus) — drain defensively so it can't
            // leak forward to an unrelated declaration.
            let _ = self.take_pending_comments();
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
                    let name = self.expect_declaration_name("متوقع اسم المعامل")?;
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
        match self.peek().kind {
            TokenKind::Public => {
                self.advance();
                Visibility::Public
            }
            TokenKind::Private => {
                self.advance();
                Visibility::Private
            }
            TokenKind::Protected => {
                self.advance();
                Visibility::Protected
            }
            _ => Visibility::Public, // default visibility
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
            // A trailing comment (line, doc, or block-doc) before '}' is not
            // the start of another method.
            if self.match_terminator_after_trivia(&TokenKind::RightBrace) {
                break;
            }
            // MethodSignature has no field for a leading comment, so the run's
            // line comments are discarded here exactly as they were before the
            // loop — but the run is consumed in one pass, so a `//` after a
            // `///` no longer hard-errors inside a ميثاق while the same shape
            // checks clean inside a صنف (issue #203).
            let trivia = self.collect_leading_trivia();
            let _ = self.take_pending_comments();
            let method_doc = trivia.doc;

            self.expect(&TokenKind::Function, "متوقع 'دالة'")?;
            let method_name = self.expect_declaration_name("متوقع اسم الدالة")?;
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
            // A trailing comment (line, doc, or block-doc) before '}' is not
            // the start of another variant.
            if self.match_terminator_after_trivia(&TokenKind::RightBrace) {
                break;
            }
            // Same as the interface-method loop: EnumVariant has no comment
            // field, so line comments are discarded as before, but the whole run
            // is consumed so the order inside it no longer matters.
            let trivia = self.collect_leading_trivia();
            let _ = self.take_pending_comments();
            let variant = self.parse_enum_variant_with_doc(trivia.doc)?;
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
    /// Parse an enum variant, carrying the doc comment its caller already took
    /// off the trivia run in front of it.
    pub(crate) fn parse_enum_variant_with_doc(
        &mut self,
        inherited_doc: Option<String>,
    ) -> Result<EnumVariant, Diagnostic> {
        let start = self.current_span();

        // A doc found here still wins for a caller that consumed none.
        let variant_doc = inherited_doc.or_else(|| self.consume_doc_comment());

        let name = self.expect_variant_name("متوقع اسم الحالة")?;

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
            let alias = self.expect_declaration_name("متوقع اسم مستعار")?;
            ImportItems::Wildcard(alias)
        } else if self.match_token(&TokenKind::LeftBrace) {
            let mut items = Vec::new();
            loop {
                let name = self.expect_declaration_name("متوقع اسم")?;
                let alias = if self.match_token(&TokenKind::As) {
                    Some(self.expect_declaration_name("متوقع اسم مستعار")?)
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
    ///
    /// Supports:
    /// - `صدّر دالة/صنف/ثابت...` - Declaration export
    /// - `صدّر { name1، name2 }` - Named exports
    /// - `صدّر { name1، name2 } من "module"` - Named re-exports
    /// - `صدّر * من "module"` - Wildcard re-export
    ///
    /// `doc_comment` is the doc comment `parse_declaration` already consumed
    /// before it knew a `صدّر` followed; it belongs to the exported declaration
    /// (issue #204). `ExportItems::Named`/`Wildcard`/`NamedReexport` have no
    /// field to hold it, so a doc comment on those forms is still dropped.
    pub(crate) fn parse_export_statement(
        &mut self,
        doc_comment: Option<String>,
    ) -> Result<Stmt, Diagnostic> {
        let start = self.current_span();
        self.advance(); // consume 'export' (صدّر)

        let items = if self.match_token(&TokenKind::Star) {
            // صدّر * من "module" - Wildcard re-export
            self.expect(&TokenKind::From, "متوقع 'من' بعد '*' في التصدير")?;
            let from = self.expect_string("متوقع مسار الوحدة")?;
            ExportItems::Wildcard { from }
        } else if self.match_token(&TokenKind::LeftBrace) {
            // صدّر { name1، name2 } [من "module"]
            let mut items = Vec::new();
            loop {
                let name = self.expect_declaration_name("متوقع اسم التصدير")?;
                let alias = if self.match_token(&TokenKind::As) {
                    Some(self.expect_identifier("متوقع الاسم المستعار")?)
                } else {
                    None
                };
                items.push(ExportItem { name, alias });

                if !self.match_token(&TokenKind::Comma)
                    && !self.match_token(&TokenKind::ArabicComma)
                {
                    break;
                }
            }
            self.expect(&TokenKind::RightBrace, "متوقع '}'")?;

            if self.match_token(&TokenKind::From) {
                // Re-export from another module
                let from = self.expect_string("متوقع مسار الوحدة")?;
                ExportItems::NamedReexport { items, from }
            } else {
                // Export from current module
                ExportItems::Named(items)
            }
        } else {
            // صدّر دالة/صنف/ثابت... - Declaration export
            let stmt = self.parse_declaration_with_doc(doc_comment)?;
            ExportItems::Declaration(Box::new(stmt))
        };

        self.consume_semicolon()?;

        let span = start.merge(&self.previous_span());
        Ok(Stmt::new(StmtKind::Export(items), span))
    }

    /// Parse a type annotation.
    pub(crate) fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, Diagnostic> {
        if self.check(&TokenKind::LeftParen) {
            return self.parse_function_type_annotation();
        }

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

    /// Parse a function-type annotation: `(T، U) -> R`.
    ///
    /// Grammar (LANGUAGE_SPEC.md §5.3):
    /// نمط_دالة := '(' [نمط {'،' نمط}] ')' ['->' نمط]
    ///
    /// The return type is parsed via a recursive `parse_type_annotation`
    /// call, which makes currying (`(عدد) -> (عدد) -> عدد`) right-associative
    /// for free. Omitting `->` is legal only with an empty parameter list,
    /// giving the spec's bare `()` sugar — a function returning nothing,
    /// represented as `return_type: None` (Tarqeem has no `فراغ` keyword).
    fn parse_function_type_annotation(&mut self) -> Result<TypeAnnotation, Diagnostic> {
        let start = self.current_span();
        self.expect(&TokenKind::LeftParen, "متوقع '('")?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                params.push(self.parse_type_annotation()?);
                if !self.match_token(&TokenKind::Comma)
                    && !self.match_token(&TokenKind::ArabicComma)
                {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RightParen, "متوقع ')'")?;

        let return_type = if self.match_token(&TokenKind::Arrow) {
            Some(Box::new(self.parse_type_annotation()?))
        } else if params.is_empty() {
            None
        } else {
            return Err(Diagnostic::error(
                "متوقع '->' بعد قائمة أنماط المعاملات",
                self.current_span(),
            )
            .with_code(ERR_UNEXPECTED_TOKEN.to_string()));
        };

        let span = start.merge(&self.previous_span());
        Ok(TypeAnnotation::new(
            TypeKind::Function {
                params,
                return_type,
            },
            span,
        ))
    }

    /// Parse function parameters.
    pub(crate) fn parse_parameters(&mut self) -> Result<Vec<Param>, Diagnostic> {
        // Newlines are trivia between the parentheses, so a long signature can be
        // wrapped (issue #255), matching parse_arguments at the call side.
        within_brackets(self, |parser| parser.parse_parameter_list())
    }

    fn parse_parameter_list(&mut self) -> Result<Vec<Param>, Diagnostic> {
        let mut params = Vec::new();

        self.skip_newlines();
        if !self.check(&TokenKind::RightParen) {
            loop {
                self.skip_newlines();
                let start = self.current_span();
                let name = self.expect_declaration_name("متوقع اسم المعامل")?;

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

                self.skip_newlines();
                if !self.match_token(&TokenKind::Comma)
                    && !self.match_token(&TokenKind::ArabicComma)
                {
                    break;
                }
            }
        }

        self.skip_newlines();
        Ok(params)
    }
}
