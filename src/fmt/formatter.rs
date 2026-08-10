//! AST-based code formatter
//!
//! Traverses the AST and generates formatted output.

use super::{FormatConfig, Printer};
use crate::parser::{
    Ast, BinaryOp, Block, ClassMember, EnumVariant, Expr, ExprKind, ImportItems, LambdaBody,
    Literal, MatchArm, MethodSignature, Param, Pattern, PatternKind, Stmt, StmtKind,
    TypeAnnotation, TypeKind, UnaryOp, Visibility,
};

pub struct Formatter {
    config: FormatConfig,
}

impl Formatter {
    pub fn new(config: FormatConfig) -> Self {
        Self { config }
    }

    pub fn format(&self, ast: &Ast) -> String {
        let mut printer = Printer::new(self.config.clone());
        self.format_ast(ast, &mut printer);
        printer.finish()
    }

    fn format_ast(&self, ast: &Ast, p: &mut Printer) {
        if ast.has_file_markers() {
            p.write("بسم_الله");
            p.newline();
            p.newline();
        }

        let mut prev_was_import = false;
        let mut first = true;

        for stmt in &ast.statements {
            if !first {
                let is_import = matches!(&stmt.kind, StmtKind::Import { .. });

                if prev_was_import && !is_import {
                    p.blank_lines(self.config.blank_lines_after_imports);
                } else if matches!(
                    &stmt.kind,
                    StmtKind::FuncDecl { .. }
                        | StmtKind::ClassDecl { .. }
                        | StmtKind::InterfaceDecl { .. }
                        | StmtKind::EnumDecl { .. }
                ) {
                    p.blank_lines(self.config.blank_lines_between_functions);
                }

                prev_was_import = is_import;
            }

            self.format_stmt(stmt, p);
            first = false;
        }

        if ast.has_file_markers() {
            p.newline();
            p.write("الحمد_لله");
            p.newline();
        }
    }

    fn format_stmt(&self, stmt: &Stmt, p: &mut Printer) {
        self.format_leading_trivia(stmt, p);
        self.format_stmt_no_leading_trivia(stmt, p);
    }

    /// Emits the comments that must sit on their own lines above a statement.
    ///
    /// Split out from `format_stmt` so a statement rendered mid-line (an
    /// exported declaration after `صدّر`, a `لكل` initializer) can skip it: a
    /// `//` or `///` emitted mid-line comments out the rest of that line, and
    /// the formatter's output would no longer parse.
    fn format_leading_trivia(&self, stmt: &Stmt, p: &mut Printer) {
        for comment in &stmt.leading_comments {
            p.write("//");
            p.write(comment);
            p.newline();
        }

        self.format_doc_comment_for_stmt(&stmt.kind, p);
    }

    fn format_stmt_no_leading_trivia(&self, stmt: &Stmt, p: &mut Printer) {
        match &stmt.kind {
            StmtKind::VarDecl {
                name,
                mutable,
                ty,
                init,
                ..
            } => {
                if *mutable {
                    p.write("متغير");
                } else {
                    p.write("ثابت");
                }
                p.write_space();
                p.write(name);

                if let Some(ty) = ty {
                    p.write_colon();
                    self.format_type(ty, p);
                }

                if let Some(init) = init {
                    p.write_operator("=");
                    self.format_expr(init, p);
                }
            }

            StmtKind::FuncDecl {
                name,
                params,
                return_type,
                body,
                is_async,
                ..
            } => {
                if *is_async {
                    p.write("متوازي");
                    p.write_space();
                }
                p.write("دالة");
                p.write_space();
                p.write(name);

                p.write_parens(|p| {
                    self.format_params(params, p);
                });

                if let Some(ret) = return_type {
                    p.write_arrow();
                    self.format_type(ret, p);
                }

                p.write_block(|p| {
                    self.format_block(body, p);
                });
            }

            StmtKind::ClassDecl {
                name,
                type_params,
                extends,
                implements,
                members,
                ..
            } => {
                p.write("صنف");
                p.write_space();
                p.write(name);

                if !type_params.is_empty() {
                    p.write_char('<');
                    for (i, param) in type_params.iter().enumerate() {
                        if i > 0 {
                            p.write_comma();
                        }
                        p.write(param);
                    }
                    p.write_char('>');
                }

                if let Some(parent) = extends {
                    p.write_space();
                    p.write("يرث");
                    p.write_space();
                    p.write(parent);
                }

                if !implements.is_empty() {
                    p.write_space();
                    p.write("يلتزم");
                    p.write_space();
                    for (i, iface) in implements.iter().enumerate() {
                        if i > 0 {
                            p.write_comma();
                        }
                        p.write(iface);
                    }
                }

                p.write_block(|p| {
                    for member in members {
                        self.format_class_member(member, p);
                    }
                });
            }

            StmtKind::InterfaceDecl {
                name,
                type_params,
                methods,
                ..
            } => {
                p.write("ميثاق");
                p.write_space();
                p.write(name);

                if !type_params.is_empty() {
                    p.write_char('<');
                    for (i, param) in type_params.iter().enumerate() {
                        if i > 0 {
                            p.write_comma();
                        }
                        p.write(param);
                    }
                    p.write_char('>');
                }

                p.write_block(|p| {
                    for method in methods {
                        self.format_method_signature(method, p);
                    }
                });
            }

            StmtKind::EnumDecl {
                name,
                type_params,
                variants,
                ..
            } => {
                p.write("تعداد");
                p.write_space();
                p.write(name);

                if !type_params.is_empty() {
                    p.write_char('<');
                    for (i, param) in type_params.iter().enumerate() {
                        if i > 0 {
                            p.write_comma();
                        }
                        p.write(param);
                    }
                    p.write_char('>');
                }

                p.write_block(|p| {
                    for variant in variants {
                        self.format_enum_variant(variant, p);
                    }
                });
            }

            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                p.write("إذا");
                p.write_space();
                p.write_parens(|p| {
                    self.format_expr(condition, p);
                });

                p.write_block(|p| {
                    self.format_block(then_branch, p);
                });

                if let Some(else_block) = else_branch {
                    p.write_space();
                    p.write("وإلا");
                    p.write_block(|p| {
                        self.format_block(else_block, p);
                    });
                }
            }

            StmtKind::While { condition, body } => {
                p.write("طالما");
                p.write_space();
                p.write_parens(|p| {
                    self.format_expr(condition, p);
                });

                p.write_block(|p| {
                    self.format_block(body, p);
                });
            }

            StmtKind::DoWhile { body, condition } => {
                p.write("افعل");
                p.write_block(|p| {
                    self.format_block(body, p);
                });
                p.write_space();
                p.write("طالما");
                p.write_space();
                p.write_parens(|p| {
                    self.format_expr(condition, p);
                });
            }

            StmtKind::For {
                init,
                condition,
                update,
                body,
            } => {
                p.write("لكل");
                p.write_space();
                p.write_char('(');

                if let Some(init_stmt) = init {
                    self.format_stmt_inline(init_stmt, p);
                }
                p.write_semicolon();
                p.write_space();

                if let Some(cond) = condition {
                    self.format_expr(cond, p);
                }
                p.write_semicolon();
                p.write_space();

                if let Some(upd) = update {
                    self.format_expr(upd, p);
                }
                p.write_char(')');

                p.write_block(|p| {
                    self.format_block(body, p);
                });
            }

            StmtKind::ForIn {
                variable,
                iterable,
                body,
            } => {
                p.write("لكل");
                p.write_space();
                p.write(variable);
                p.write_space();
                p.write("في");
                p.write_space();
                self.format_expr(iterable, p);

                p.write_block(|p| {
                    self.format_block(body, p);
                });
            }

            StmtKind::Match { expr, arms } => {
                p.write("تطابق");
                p.write_space();
                p.write_parens(|p| {
                    self.format_expr(expr, p);
                });

                p.write_block(|p| {
                    for arm in arms {
                        self.format_match_arm(arm, p);
                    }
                });
            }

            StmtKind::Return(expr) => {
                p.write("أرجع");
                if let Some(e) = expr {
                    p.write_space();
                    self.format_expr(e, p);
                }
            }

            StmtKind::Break => {
                p.write("أوقف");
            }

            StmtKind::Continue => {
                p.write("استمر");
            }

            StmtKind::Try {
                body,
                catch,
                finally,
            } => {
                p.write("حاول");
                p.write_block(|p| {
                    self.format_block(body, p);
                });

                if let Some(catch_clause) = catch {
                    p.write_space();
                    p.write("التقط");
                    p.write_space();
                    p.write_parens(|p| {
                        p.write(&catch_clause.param);
                    });
                    p.write_block(|p| {
                        self.format_block(&catch_clause.body, p);
                    });
                }

                if let Some(finally_block) = finally {
                    p.write_space();
                    p.write("أخيراً");
                    p.write_block(|p| {
                        self.format_block(finally_block, p);
                    });
                }
            }

            StmtKind::Throw(expr) => {
                p.write("ارمِ");
                p.write_space();
                self.format_expr(expr, p);
            }

            StmtKind::Import { items, from } => {
                p.write("استورد");
                p.write_space();
                self.format_import_items(items, p);
                p.write_space();
                p.write("من");
                p.write_space();
                p.write_char('"');
                p.write(from);
                p.write_char('"');
            }

            StmtKind::Export(export_items) => {
                use crate::parser::ExportItems;

                // The doc comment belongs to the exported declaration but was
                // written above `صدّر`, and must be emitted there: letting the
                // inner declaration print it would place `/// ...` after `صدّر`
                // on the same line, commenting out the declaration itself. The
                // inner statement's `leading_comments` are always empty here —
                // `parse_declaration` attaches those to the outer `صدّر`
                // statement (decl_parser.rs) — so only the doc needs hoisting.
                if let ExportItems::Declaration(inner) = export_items {
                    self.format_doc_comment_for_stmt(&inner.kind, p);
                }

                p.write("صدّر");
                match export_items {
                    ExportItems::Declaration(inner) => {
                        p.write_space();
                        self.format_stmt_inline(inner, p);
                    }
                    ExportItems::Named(items) => {
                        p.write_space();
                        p.write("{");
                        p.write_space();
                        for (i, item) in items.iter().enumerate() {
                            if i > 0 {
                                p.write_comma();
                                p.write_space();
                            }
                            p.write(&item.name);
                            if let Some(alias) = &item.alias {
                                p.write_space();
                                p.write("كـ");
                                p.write_space();
                                p.write(alias);
                            }
                        }
                        p.write_space();
                        p.write("}");
                    }
                    ExportItems::Wildcard { from } => {
                        p.write_space();
                        p.write("*");
                        p.write_space();
                        p.write("من");
                        p.write_space();
                        p.write("\"");
                        p.write(from);
                        p.write("\"");
                    }
                    ExportItems::NamedReexport { items, from } => {
                        p.write_space();
                        p.write("{");
                        p.write_space();
                        for (i, item) in items.iter().enumerate() {
                            if i > 0 {
                                p.write_comma();
                                p.write_space();
                            }
                            p.write(&item.name);
                            if let Some(alias) = &item.alias {
                                p.write_space();
                                p.write("كـ");
                                p.write_space();
                                p.write(alias);
                            }
                        }
                        p.write_space();
                        p.write("}");
                        p.write_space();
                        p.write("من");
                        p.write_space();
                        p.write("\"");
                        p.write(from);
                        p.write("\"");
                    }
                }
            }

            StmtKind::Expr(expr) => {
                self.format_expr(expr, p);
            }

            StmtKind::Block(block) => {
                p.write_block(|p| {
                    self.format_block(block, p);
                });
            }
        }

        // Output trailing comment if present. `LineComment` content keeps its
        // leading space, but `DocComment`/`BlockDocComment` content has it
        // stripped by the lexer — write_comment_lines' per-line trim()
        // normalizes both to one space after `//`, and re-prefixes every
        // continuation line of a multi-line `/** */` so it can't corrupt
        // re-parsing (see finding #1 / write_comment_lines' doc comment).
        if let Some(trailing) = &stmt.trailing_comment {
            p.write("  ");
            self.write_comment_lines(trailing, p);
        }

        p.newline();
    }

    fn format_stmt_inline(&self, stmt: &Stmt, p: &mut Printer) {
        match &stmt.kind {
            StmtKind::VarDecl {
                name,
                mutable,
                ty,
                init,
                ..
            } => {
                if *mutable {
                    p.write("متغير");
                } else {
                    p.write("ثابت");
                }
                p.write_space();
                p.write(name);

                if let Some(ty) = ty {
                    p.write_colon();
                    self.format_type(ty, p);
                }

                if let Some(init) = init {
                    p.write_operator("=");
                    self.format_expr(init, p);
                }
            }
            StmtKind::Expr(expr) => {
                self.format_expr(expr, p);
            }
            _ => {
                // Leading trivia is the caller's job here — this statement is
                // being rendered mid-line, where a comment would swallow it.
                self.format_stmt_no_leading_trivia(stmt, p);
            }
        }
    }

    fn format_block(&self, block: &Block, p: &mut Printer) {
        for stmt in &block.statements {
            self.format_stmt(stmt, p);
        }
        for comment in &block.dangling_comments {
            self.write_comment_lines(comment, p);
            p.newline();
        }
    }

    fn format_expr(&self, expr: &Expr, p: &mut Printer) {
        match &expr.kind {
            ExprKind::Literal(lit) => self.format_literal(lit, p),

            ExprKind::Identifier(name) => {
                p.write(name);
            }

            ExprKind::Binary { left, op, right } => {
                self.format_expr(left, p);
                p.write_operator(self.binary_op_str(*op));
                self.format_expr(right, p);
            }

            ExprKind::Unary { op, operand } => match op {
                UnaryOp::Neg => {
                    p.write_char('-');
                    self.format_expr(operand, p);
                }
                UnaryOp::Not => {
                    p.write_char('!');
                    self.format_expr(operand, p);
                }
                UnaryOp::PreInc => {
                    p.write("++");
                    self.format_expr(operand, p);
                }
                UnaryOp::PreDec => {
                    p.write("--");
                    self.format_expr(operand, p);
                }
                UnaryOp::PostInc => {
                    self.format_expr(operand, p);
                    p.write("++");
                }
                UnaryOp::PostDec => {
                    self.format_expr(operand, p);
                    p.write("--");
                }
            },

            ExprKind::Call { callee, args } => {
                self.format_expr(callee, p);
                p.write_parens(|p| {
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            p.write_comma();
                        }
                        self.format_expr(arg, p);
                    }
                });
            }

            ExprKind::Member { object, property } => {
                self.format_expr(object, p);
                p.write_char('.');
                p.write(property);
            }

            ExprKind::Index { object, index } => {
                self.format_expr(object, p);
                p.write_brackets(|p| {
                    self.format_expr(index, p);
                });
            }

            ExprKind::Assignment { target, value } => {
                self.format_expr(target, p);
                p.write_operator("=");
                self.format_expr(value, p);
            }

            ExprKind::CompoundAssignment { target, op, value } => {
                self.format_expr(target, p);
                p.write_operator(&format!("{}=", self.binary_op_str(*op)));
                self.format_expr(value, p);
            }

            ExprKind::Array(elements) => {
                p.write_brackets(|p| {
                    for (i, elem) in elements.iter().enumerate() {
                        if i > 0 {
                            p.write_comma();
                        }
                        self.format_expr(elem, p);
                    }
                });
            }

            ExprKind::Object(pairs) => {
                p.write_char('{');
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        p.write_comma();
                    }
                    p.write_space();
                    p.write(key);
                    p.write_colon();
                    self.format_expr(value, p);
                }
                if !pairs.is_empty() {
                    p.write_space();
                }
                p.write_char('}');
            }

            ExprKind::Lambda { params, body } => {
                p.write_parens(|p| {
                    self.format_params(params, p);
                });
                p.write_fat_arrow();
                match body {
                    LambdaBody::Expr(e) => self.format_expr(e, p),
                    LambdaBody::Block(b) => {
                        p.write_block(|p| {
                            self.format_block(b, p);
                        });
                    }
                }
            }

            ExprKind::New {
                class,
                type_args,
                args,
            } => {
                p.write("جديد");
                p.write_space();
                self.format_expr(class, p);
                if !type_args.is_empty() {
                    p.write_char('<');
                    for (i, ta) in type_args.iter().enumerate() {
                        if i > 0 {
                            p.write("، ");
                        }
                        self.format_type(ta, p);
                    }
                    p.write_char('>');
                }
                p.write_parens(|p| {
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            p.write_comma();
                        }
                        self.format_expr(arg, p);
                    }
                });
            }

            ExprKind::Await(inner) => {
                p.write("انتظر");
                p.write_space();
                self.format_expr(inner, p);
            }

            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                self.format_expr(condition, p);
                p.write_operator("?");
                self.format_expr(then_expr, p);
                p.write_operator(":");
                self.format_expr(else_expr, p);
            }

            ExprKind::Grouping(inner) => {
                p.write_parens(|p| {
                    self.format_expr(inner, p);
                });
            }

            ExprKind::This => {
                p.write("هذا");
            }

            ExprKind::Super => {
                p.write("الأصل");
            }

            ExprKind::EnumVariant {
                enum_name,
                type_args,
                variant_name,
                args,
            } => {
                // Format: EnumName::VariantName or EnumName<T>::VariantName(args)
                p.write(enum_name);
                if !type_args.is_empty() {
                    p.write_char('<');
                    for (i, ta) in type_args.iter().enumerate() {
                        if i > 0 {
                            p.write("، ");
                        }
                        self.format_type(ta, p);
                    }
                    p.write_char('>');
                }
                p.write("::");
                p.write(variant_name);
                if !args.is_empty() {
                    p.write_parens(|p| {
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 {
                                p.write_comma();
                            }
                            self.format_expr(arg, p);
                        }
                    });
                }
            }
        }
    }

    fn format_literal(&self, lit: &Literal, p: &mut Printer) {
        match lit {
            Literal::Int(n) => p.write(&n.to_string()),
            Literal::Float(f) => p.write(&f.to_string()),
            Literal::String(s) => {
                p.write_char('"');
                for c in s.chars() {
                    match c {
                        '"' => p.write("\\\""),
                        '\\' => p.write("\\\\"),
                        '\n' => p.write("\\n"),
                        '\r' => p.write("\\r"),
                        '\t' => p.write("\\t"),
                        _ => p.write_char(c),
                    }
                }
                p.write_char('"');
            }
            Literal::Bool(b) => {
                if *b {
                    p.write("صحيح");
                } else {
                    p.write("خطأ");
                }
            }
            Literal::Null => p.write("لا_شيء"),
        }
    }

    fn format_type(&self, ty: &TypeAnnotation, p: &mut Printer) {
        match &ty.kind {
            TypeKind::Simple(name) => {
                p.write(name);
            }
            TypeKind::Array(inner) => {
                p.write("مصفوفة");
                p.write_char('<');
                self.format_type(inner, p);
                p.write_char('>');
            }
            TypeKind::Map(key, value) => {
                p.write("قاموس");
                p.write_char('<');
                self.format_type(key, p);
                p.write_comma();
                self.format_type(value, p);
                p.write_char('>');
            }
            TypeKind::Function {
                params,
                return_type,
            } => {
                p.write_parens(|p| {
                    for (i, param) in params.iter().enumerate() {
                        if i > 0 {
                            p.write_comma();
                        }
                        self.format_type(param, p);
                    }
                });
                // Bare `()` has no return type and must print as `()` —
                // there is no surface spelling for "returns nothing".
                if let Some(rt) = return_type {
                    p.write_arrow();
                    self.format_type(rt, p);
                }
            }
            TypeKind::Generic { base, args } => {
                p.write(base);
                if !args.is_empty() {
                    p.write_char('<');
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            p.write_comma();
                        }
                        self.format_type(arg, p);
                    }
                    p.write_char('>');
                }
            }
            TypeKind::Optional(inner) => {
                self.format_type(inner, p);
                p.write_char('?');
            }
        }
    }

    fn format_params(&self, params: &[Param], p: &mut Printer) {
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                p.write_comma();
            }
            p.write(&param.name);
            if let Some(ty) = &param.ty {
                p.write_colon();
                self.format_type(ty, p);
            }
            if let Some(default) = &param.default {
                p.write_operator("=");
                self.format_expr(default, p);
            }
        }
    }

    fn format_class_member(&self, member: &ClassMember, p: &mut Printer) {
        match member {
            ClassMember::Field {
                visibility,
                name,
                ty,
                init,
                is_static,
                leading_comments,
                doc_comment,
            } => {
                for comment in leading_comments {
                    p.write("//");
                    p.write(comment);
                    p.newline();
                }

                if let Some(doc) = doc_comment.as_ref() {
                    self.format_doc_comment(doc, p);
                }

                self.format_visibility(*visibility, p);

                if *is_static {
                    p.write("مشترك");
                    p.write_space();
                }

                p.write(name);
                if let Some(ty) = ty {
                    p.write_colon();
                    self.format_type(ty, p);
                }

                if let Some(init) = init {
                    p.write_operator("=");
                    self.format_expr(init, p);
                }

                p.newline();
            }

            ClassMember::Method {
                visibility,
                name,
                params,
                return_type,
                body,
                is_static,
                is_async,
                leading_comments,
                doc_comment,
            } => {
                p.blank_line();

                for comment in leading_comments {
                    p.write("//");
                    p.write(comment);
                    p.newline();
                }

                if let Some(doc) = doc_comment.as_ref() {
                    self.format_doc_comment(doc, p);
                }

                self.format_visibility(*visibility, p);

                if *is_static {
                    p.write("مشترك");
                    p.write_space();
                }

                if *is_async {
                    p.write("متوازي");
                    p.write_space();
                }

                p.write("دالة");
                p.write_space();
                p.write(name);

                p.write_parens(|p| {
                    self.format_params(params, p);
                });

                if let Some(ret) = return_type {
                    p.write_arrow();
                    self.format_type(ret, p);
                }

                p.write_block(|p| {
                    self.format_block(body, p);
                });
                p.newline();
            }

            ClassMember::Constructor {
                params,
                body,
                leading_comments,
                doc_comment,
            } => {
                p.blank_line();

                for comment in leading_comments {
                    p.write("//");
                    p.write(comment);
                    p.newline();
                }

                if let Some(doc) = doc_comment.as_ref() {
                    self.format_doc_comment(doc, p);
                }

                p.write("منشئ");
                p.write_parens(|p| {
                    self.format_params(params, p);
                });

                p.write_block(|p| {
                    self.format_block(body, p);
                });
                p.newline();
            }

            ClassMember::Property {
                visibility,
                name,
                ty,
                accessors,
                default_value,
                is_static,
                leading_comments,
                doc_comment,
            } => {
                p.blank_line();

                for comment in leading_comments {
                    p.write("//");
                    p.write(comment);
                    p.newline();
                }

                if let Some(doc) = doc_comment.as_ref() {
                    self.format_doc_comment(doc, p);
                }

                self.format_visibility(*visibility, p);

                if *is_static {
                    p.write("مشترك");
                    p.write_space();
                }

                p.write("خاصية");
                p.write_space();
                p.write(name);
                p.write_colon();
                self.format_type(ty, p);

                if accessors.is_empty() {
                    if let Some(init) = default_value {
                        p.write_operator("=");
                        self.format_expr(init, p);
                    }
                    p.newline();
                } else {
                    p.write_block(|p| {
                        for accessor in accessors {
                            match accessor {
                                crate::parser::PropertyAccessor::Get {
                                    visibility: acc_vis,
                                    body,
                                } => {
                                    self.format_visibility(*acc_vis, p);
                                    p.write("احصل");
                                    match body {
                                        crate::parser::PropertyAccessorBody::Expr(expr) => {
                                            p.write_operator("=>");
                                            self.format_expr(expr, p);
                                            p.newline();
                                        }
                                        crate::parser::PropertyAccessorBody::Block(block) => {
                                            p.write_block(|p| {
                                                self.format_block(block, p);
                                            });
                                            p.newline();
                                        }
                                    }
                                }
                                crate::parser::PropertyAccessor::Set {
                                    visibility: acc_vis,
                                    param_name,
                                    body,
                                } => {
                                    self.format_visibility(*acc_vis, p);
                                    p.write("عيّن");
                                    p.write_parens(|p| {
                                        p.write(param_name);
                                    });
                                    p.write_block(|p| {
                                        self.format_block(body, p);
                                    });
                                    p.newline();
                                }
                            }
                        }
                    });
                    p.newline();
                }
            }
        }
    }

    fn format_method_signature(&self, sig: &MethodSignature, p: &mut Printer) {
        if let Some(doc) = &sig.doc_comment {
            self.format_doc_comment(doc, p);
        }

        p.write("دالة");
        p.write_space();
        p.write(&sig.name);

        p.write_parens(|p| {
            self.format_params(&sig.params, p);
        });

        if let Some(ret) = &sig.return_type {
            p.write_arrow();
            self.format_type(ret, p);
        }

        p.newline();
    }

    fn format_enum_variant(&self, variant: &EnumVariant, p: &mut Printer) {
        if let Some(doc) = &variant.doc_comment {
            self.format_doc_comment(doc, p);
        }

        p.write(&variant.name);

        if let Some(disc) = variant.discriminant {
            p.write_operator("=");
            p.write(&disc.to_string());
        }

        if !variant.fields.is_empty() {
            p.write_parens(|p| {
                for (i, field) in variant.fields.iter().enumerate() {
                    if i > 0 {
                        p.write_comma();
                    }
                    if let Some(name) = &field.name {
                        p.write(name);
                        p.write_colon();
                    }
                    self.format_type(&field.ty, p);
                }
            });
        }

        p.newline();
    }

    fn format_match_arm(&self, arm: &MatchArm, p: &mut Printer) {
        // `غير_ذلك` is its own arm production, not a pattern introduced by
        // `حالة` (LANGUAGE_SPEC §15.6: ذراع_تطابق := 'حالة' تعبير … | 'غير_ذلك'
        // '=>' …). Writing `حالة` unconditionally produced `حالة غير_ذلك`, which
        // the parser rejects with `رمز غير متوقع: Default` (ب٠٠٠١) — output the
        // formatter itself could not re-read.
        let is_wildcard_arm =
            arm.patterns.len() == 1 && matches!(arm.patterns[0].kind, PatternKind::Wildcard);

        if !is_wildcard_arm {
            p.write("حالة");
            p.write_space();
        }

        for (i, pattern) in arm.patterns.iter().enumerate() {
            if i > 0 {
                p.write_comma();
            }
            self.format_pattern(pattern, p);
        }

        p.write_fat_arrow();
        p.write_block(|p| {
            self.format_block(&arm.body, p);
        });
        p.newline();
    }

    fn format_pattern(&self, pattern: &Pattern, p: &mut Printer) {
        match &pattern.kind {
            PatternKind::Literal(expr) => {
                self.format_expr(expr, p);
            }
            PatternKind::Identifier(name) => {
                p.write(name);
            }
            PatternKind::Wildcard => {
                p.write("غير_ذلك");
            }
            PatternKind::EnumVariant {
                enum_name,
                variant_name,
                bindings,
            } => {
                p.write(enum_name);
                p.write("::");
                p.write(variant_name);
                if !bindings.is_empty() {
                    p.write_char('(');
                    for (i, binding) in bindings.iter().enumerate() {
                        if i > 0 {
                            p.write_comma();
                        }
                        p.write(binding);
                    }
                    p.write_char(')');
                }
            }
        }
    }

    fn format_import_items(&self, items: &ImportItems, p: &mut Printer) {
        match items {
            ImportItems::Named(imports) => {
                p.write_char('{');
                p.write_space();
                for (i, item) in imports.iter().enumerate() {
                    if i > 0 {
                        p.write_comma();
                    }
                    p.write(&item.name);
                    if let Some(alias) = &item.alias {
                        p.write_space();
                        p.write("كـ");
                        p.write_space();
                        p.write(alias);
                    }
                }
                p.write_space();
                p.write_char('}');
            }
            ImportItems::Wildcard(alias) => {
                p.write_char('*');
                p.write_space();
                p.write("كـ");
                p.write_space();
                p.write(alias);
            }
            ImportItems::Default(name) => {
                p.write(name);
            }
        }
    }

    fn format_visibility(&self, vis: Visibility, p: &mut Printer) {
        match vis {
            Visibility::Public => {
                p.write("عام");
                p.write_space();
            }
            Visibility::Private => {
                p.write("خاص");
                p.write_space();
            }
            Visibility::Protected => {
                p.write("محمي");
                p.write_space();
            }
        }
    }

    /// Writes `text` as `//` comment lines, re-prefixing every continuation line.
    /// A multi-line `/** */` (scan_block_doc_comment, lexer.rs:458-471) joins its
    /// lines with embedded `\n`; a single `// ` prefix would leave continuation
    /// lines as bare code that fails to re-parse. Emits no leading/trailing
    /// newline — the caller owns those.
    fn write_comment_lines(&self, text: &str, p: &mut Printer) {
        let mut wrote_any = false;
        for line in text.lines() {
            if wrote_any {
                p.newline();
            }
            let line = line.trim();
            if line.is_empty() {
                p.write("//");
            } else {
                p.write("// ");
                p.write(line);
            }
            wrote_any = true;
        }
        if !wrote_any {
            p.write("//"); // /** */ lexes to BlockDocComment("")
        }
    }

    /// Writes `text` as `///` doc-comment lines, re-prefixing every line.
    ///
    /// Deliberately a sibling of `write_comment_lines` rather than a shared
    /// helper with a swappable marker, because the two differ in how much of
    /// each line they may touch. Doc *content* must survive verbatim: the lexer
    /// strips exactly one leading space per `///` line (lexer.rs:385-387), so
    /// indentation inside a doc block is content, not layout. Trimming line
    /// starts here — as the trailing-comment path does — would silently flatten
    /// it, which is the same data loss this function exists to prevent.
    /// `trim_end` alone round-trips exactly: the emitted `/// ` + line re-lexes
    /// back to `line`. Emits no leading/trailing newline — the caller owns those.
    fn write_doc_comment_lines(&self, text: &str, p: &mut Printer) {
        let mut wrote_any = false;
        for line in text.lines() {
            if wrote_any {
                p.newline();
            }
            let line = line.trim_end();
            p.write("///");
            if !line.is_empty() {
                p.write_char(' ');
                p.write(line);
            }
            wrote_any = true;
        }
        if !wrote_any {
            p.write("///"); // /** */ lexes to BlockDocComment("")
        }
    }

    /// Emits a declaration's doc comment as `///` lines, one per line of `doc`,
    /// followed by the newline every call site expects before the declaration.
    ///
    /// The `///` marker is not decoration: without it the doc text is emitted as
    /// bare words and the formatter's own output no longer parses (issue #201).
    fn format_doc_comment(&self, doc: &str, p: &mut Printer) {
        self.write_doc_comment_lines(doc, p);
        p.newline();
    }

    fn format_doc_comment_for_stmt(&self, kind: &StmtKind, p: &mut Printer) {
        let doc = match kind {
            StmtKind::VarDecl { doc_comment, .. } => doc_comment.as_ref(),
            StmtKind::FuncDecl { doc_comment, .. } => doc_comment.as_ref(),
            StmtKind::ClassDecl { doc_comment, .. } => doc_comment.as_ref(),
            StmtKind::InterfaceDecl { doc_comment, .. } => doc_comment.as_ref(),
            StmtKind::EnumDecl { doc_comment, .. } => doc_comment.as_ref(),
            _ => None,
        };

        if let Some(doc) = doc {
            self.format_doc_comment(doc, p);
        }
    }

    fn binary_op_str(&self, op: BinaryOp) -> &'static str {
        match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Pow => "**",
            BinaryOp::Eq => "==",
            BinaryOp::NotEq => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap_with_markers(source: &str) -> String {
        format!("بسم_الله\n{}\nالحمد_لله", source.trim())
    }

    fn format(source: &str) -> String {
        let config = FormatConfig::default();
        let wrapped = wrap_with_markers(source);
        let mut parser = crate::parser::Parser::new(&wrapped);
        let ast = parser.parse().expect("Parse failed");
        let formatter = Formatter::new(config);
        formatter.format(&ast)
    }

    /// Like `format`, but does not wrap the input with بسم_الله/الحمد_لله
    /// markers — needed for round-tripping output that already carries them.
    fn format_raw(source: &str) -> String {
        let config = FormatConfig::default();
        let mut parser = crate::parser::Parser::new(source);
        let ast = parser.parse().expect("Parse failed");
        let formatter = Formatter::new(config);
        formatter.format(&ast)
    }

    #[test]
    fn test_format_variable() {
        let result = format("متغير س=5");
        assert!(result.starts_with("بسم_الله\n"));
        assert!(result.contains("متغير س = 5"));
        assert!(result.ends_with("الحمد_لله\n"));
    }

    #[test]
    fn test_format_constant() {
        let result = format("ثابت ط = 3.14");
        assert!(result.starts_with("بسم_الله\n"));
        assert!(result.contains("ثابت ط = 3.14"));
        assert!(result.ends_with("الحمد_لله\n"));
    }

    #[test]
    fn test_format_function() {
        let result = format("دالة اختبار(أ:عدد)->عدد{أرجع أ}");
        assert!(result.contains("دالة اختبار(أ: عدد) -> عدد {"));
        assert!(result.contains("    أرجع أ"));
    }

    #[test]
    fn test_format_if_else() {
        let result = format("إذا(س>0){اطبع(س)}وإلا{اطبع(0)}");
        assert!(result.contains("إذا (س > 0) {"));
        assert!(result.contains("} وإلا {"));
    }

    #[test]
    fn test_format_class() {
        let result = format("صنف شخص{عام اسم:نص}");
        assert!(result.contains("صنف شخص {"));
        assert!(result.contains("    عام اسم: نص"));
    }

    #[test]
    fn test_format_array() {
        let result = format("متغير أ = [1،2،3]");
        assert!(result.contains("[1, 2, 3]"));
    }

    #[test]
    fn test_format_binary_ops() {
        let result = format("متغير س=1+2*3-4/2");
        assert!(result.contains("1 + 2 * 3 - 4 / 2"));
    }

    #[test]
    fn test_format_import() {
        let result = format("استورد { قائمة } من \"مجموعات\"");
        assert!(result.contains("استورد { قائمة } من \"مجموعات\""));
    }

    #[test]
    fn test_format_preserves_line_comments() {
        let result = format("// مثال على استخدام التعداد في ترقيم\nمتغير س = 5");
        assert!(result.contains("// مثال على استخدام التعداد في ترقيم"));
        assert!(result.contains("متغير س = 5"));
    }

    #[test]
    fn test_format_preserves_multiple_comments() {
        let result = format("// التعليق الأول\n// التعليق الثاني\nمتغير س = 5");
        assert!(result.contains("// التعليق الأول"));
        assert!(result.contains("// التعليق الثاني"));
        assert!(result.contains("متغير س = 5"));
    }

    #[test]
    fn test_format_comment_not_broken() {
        // This test ensures // doesn't become / /
        let result = format("// تعليق\nمتغير س = 5");
        // Should contain "//" not "/ /"
        assert!(result.contains("//"));
        assert!(!result.contains("/ /"));
    }

    #[test]
    fn test_format_trailing_comment() {
        let result = format("متغير س = 5  // تعليق نهائي");
        assert!(result.contains("متغير س = 5  // تعليق نهائي"));
    }

    #[test]
    fn test_format_trailing_comment_on_multiple_statements() {
        let result = format("متغير س = 5  // المتغير الأول\nمتغير ص = 10  // المتغير الثاني");
        assert!(result.contains("متغير س = 5  // المتغير الأول"));
        assert!(result.contains("متغير ص = 10  // المتغير الثاني"));
    }

    #[test]
    fn test_format_mixed_leading_and_trailing_comments() {
        let result = format("// تعليق قبلي\nمتغير س = 5  // تعليق بعدي");
        assert!(result.contains("// تعليق قبلي"));
        assert!(result.contains("متغير س = 5  // تعليق بعدي"));
    }

    #[test]
    fn test_format_trailing_comment_in_function() {
        let result = format("دالة اختبار() {\n    متغير س = 5  // داخل الدالة\n}");
        assert!(result.contains("متغير س = 5  // داخل الدالة"));
    }

    // Regression tests for #193/#194/#198 (comment handling & error-masking
    // bundle). The parser fixes have landed: a comment-only function body and
    // a trailing `///` both parse, and these tests are what keep the comment
    // surviving the formatter round trip instead of being dropped or
    // corrupted.

    #[test]
    fn test_format_comment_only_function_body_preserves_comment() {
        let result = format("دالة س() {\n    // تعليق\n}");
        assert!(result.contains("// تعليق"));
    }

    #[test]
    fn test_format_trailing_doc_comment_normalized() {
        // A trailing `///` is semantically meaningless as a doc comment
        // (it doesn't precede a declaration), so on round-trip it must be
        // normalized to a plain `//` line comment.
        let result = format("متغير س = 5 /// تعليق");
        assert!(result.contains("// تعليق"));
        assert!(!result.contains("/// تعليق"));
    }

    // Regression tests for finding #1: a multi-line `/** */` trailing comment
    // (scan_block_doc_comment joins its lines with embedded `\n`) must have
    // every continuation line re-prefixed with `//`, or the continuation
    // lands as bare, uncommented code that fails to re-parse.

    #[test]
    fn test_format_multiline_trailing_block_doc_comment() {
        let result = format("متغير س = 5 /** سطر واحد\nسطر آخر */\nمتغير ص = 6");
        assert!(result.contains("متغير س = 5  // سطر واحد"));
        assert!(
            result.lines().any(|l| l.trim() == "// سطر آخر"),
            "expected a standalone '// سطر آخر' line in:\n{result}"
        );
        // Every occurrence of the continuation text must be preceded by
        // `//` on its own line — never a bare, uncommented line.
        assert!(!result.contains("\nسطر آخر\n"));
    }

    #[test]
    fn test_format_multiline_trailing_comment_is_idempotent() {
        let once = format("متغير س = 5 /** سطر واحد\nسطر آخر */\nمتغير ص = 6");
        let twice = format_raw(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn test_format_multiline_trailing_comment_output_reparses() {
        let once = format("متغير س = 5 /** سطر واحد\nسطر آخر */\nمتغير ص = 6");
        // Automated form of the manual `fmt -w` → `check` repro from the
        // review: formatting once must never produce output that fails to
        // parse (previously: خطأ[ب٠١٠١] "متوقع '؛'").
        crate::parser::Parser::new(&once)
            .parse()
            .expect("re-parse must succeed");
    }

    #[test]
    fn test_format_multiline_trailing_comment_indented_in_block() {
        let result = format("دالة س() {\n    متغير س = 5 /** سطر واحد\nسطر آخر */\n}");
        assert!(
            result.contains("    // سطر آخر"),
            "expected 4-space-indented continuation line in:\n{result}"
        );
    }

    // Regression tests for finding #2: a leading `//` comment on a class
    // member must stay attached to that member, not leak past the class's
    // closing brace to whatever statement follows it.

    #[test]
    fn test_format_class_member_leading_comment_stays_inside_class() {
        let result = format("صنف س {\n    // تعليق\n    عام دالة م() {}\n}\nمتغير ص = 5");

        let comment_pos = result
            .find("// تعليق")
            .expect("expected the leading comment to appear in the output");
        let method_pos = result
            .find("عام دالة م")
            .expect("expected the method declaration to appear in the output");
        let var_pos = result
            .find("متغير ص")
            .expect("expected the trailing statement to appear in the output");
        let class_close_pos = result[..var_pos]
            .rfind('}')
            .expect("expected a closing brace for the class before the next statement");

        assert!(
            comment_pos < method_pos,
            "comment must appear before the method it documents"
        );
        assert!(
            comment_pos < class_close_pos,
            "comment must stay inside the class body, not leak past its closing brace"
        );
        assert!(
            comment_pos < var_pos,
            "comment must not leak to the unrelated statement after the class"
        );
    }

    #[test]
    fn test_format_stack_module_banner_stays_above_method() {
        // Direct repro of the 92-real-line corpus bug found during code
        // review: a `//` banner comment placed between two class members
        // must stay above the member that follows it, not be relocated
        // into that member's body (or past it).
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/stdlib_trq/مجموعات/مكدس.ترقيم"
        ));
        let result = format_raw(source);

        let banner_pos = result
            .find("العمليات الأساسية")
            .expect("expected the banner comment to appear in the output");
        let method_pos = result
            .find("دالة ادفع")
            .expect("expected the دالة ادفع method to appear in the output");

        assert!(
            banner_pos < method_pos,
            "banner comment must stay above 'دالة ادفع', not be relocated into its body"
        );
    }

    // Function-type annotation formatting (issue #180): the formatter's
    // `TypeKind::Function` arm must produce output that re-parses —
    // including the bare-`()` sugar, which has no return type to print.

    #[test]
    fn test_format_function_type_annotation() {
        let result = format("ثابت جمع: (عدد, عدد) -> عدد = (أ, ب) => أ + ب;");
        // ASCII comma: FormatConfig::default() has arabic_comma = false.
        assert!(
            result.contains("(عدد, عدد) -> عدد"),
            "expected function-type annotation in output:\n{result}"
        );
    }

    #[test]
    fn test_format_function_type_annotation_is_idempotent() {
        let once = format("ثابت جمع: (عدد, عدد) -> عدد = (أ, ب) => أ + ب;");
        let twice = format_raw(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn test_format_bare_unit_function_type_round_trips() {
        // A `return_type: None` prints as bare `()` — there is no surface
        // spelling for "returns nothing", so emitting any `-> X` here would
        // produce output that fails to re-parse (an issue-#201-class
        // regression).
        let once = format("متغير ف: ();");
        crate::parser::Parser::new(&once)
            .parse()
            .expect("formatted output must re-parse");
    }

    #[test]
    fn test_format_curried_function_type_is_idempotent() {
        let once = format("متغير ف: (عدد) -> (عدد) -> عدد;");
        let twice = format_raw(&once);
        assert_eq!(once, twice);
    }

    // Regression tests for #201 / #204. Every pre-existing doc-comment test
    // above covers a *trailing* `///`/`/** */`, which routes through
    // `write_comment_lines`; nothing exercised a *leading* doc comment, which is
    // how #201 shipped. Asserting the marker is present is the whole point —
    // without it the doc text is emitted as bare words and the output is no
    // longer a program.

    /// Every line of a doc comment must carry its own `///`. The lexer merges a
    /// consecutive `///` run into one token joined by `\n`, so a single prefix
    /// would leave line 2 onward as bare text.
    #[test]
    fn test_format_leading_doc_comment_keeps_marker_on_every_line() {
        let result = format("/// وثيقة الدالة\n/// سطر ثان\nدالة س() {}");
        assert!(result.contains("/// وثيقة الدالة"));
        assert!(result.contains("/// سطر ثان"));
        assert!(
            !result.contains("\nوثيقة الدالة"),
            "doc text must never appear unprefixed: {result}"
        );
        assert!(
            !result.contains("\nسطر ثان"),
            "continuation line must be re-prefixed: {result}"
        );
    }

    /// The exact repro from issue #201's body.
    #[test]
    fn test_format_leading_doc_comment_output_reparses() {
        let once = format("/// وثيقة الدالة\n/// سطر ثان\nدالة س() {}");
        crate::parser::Parser::new(&once)
            .parse()
            .expect("re-parse must succeed (was خطأ[ب٠١٠١] 'متوقع ؛')");
    }

    #[test]
    fn test_format_leading_doc_comment_is_idempotent() {
        let once = format("/// وثيقة الدالة\n/// سطر ثان\nدالة س() {}");
        let twice = format_raw(&once);
        assert_eq!(once, twice);
    }

    /// Indentation *inside* a doc block is content, not layout: the lexer strips
    /// exactly one leading space per line, so trimming line starts here would
    /// silently flatten an indented example. Hence `trim_end` only.
    #[test]
    fn test_format_leading_doc_comment_preserves_interior_indentation() {
        let once = format("/// مثال:\n///     س = ٥\nدالة س() {}");
        assert!(
            once.contains("///     س = ٥"),
            "interior indentation must survive verbatim: {once}"
        );
        // Round-trips exactly: the emitted `/// ` re-lexes back to the same
        // content, so the indentation cannot erode over repeated formatting.
        assert_eq!(once, format_raw(&once));
    }

    /// A blank `///` line inside a doc block stays a `///` line rather than
    /// becoming a genuinely empty line.
    #[test]
    fn test_format_leading_doc_comment_blank_line_keeps_marker() {
        let once = format("/// أول\n///\n/// ثالث\nدالة س() {}");
        assert!(once.contains("/// أول"));
        assert!(once.contains("/// ثالث"));
        assert_eq!(once, format_raw(&once));
    }

    /// The second half of #201: a `/** */` before a declaration used to be a
    /// hard parse error. It is now attached and normalized to `///`, matching
    /// the existing precedent that a trailing `/** */` normalizes to `//`.
    #[test]
    fn test_format_leading_block_doc_comment_normalized_to_slashes() {
        let once = format("/** وثيقة */\nدالة س() {}");
        assert!(once.contains("/// وثيقة"), "got: {once}");
        assert!(!once.contains("/**"));
        assert_eq!(once, format_raw(&once));
    }

    #[test]
    fn test_format_leading_multiline_block_doc_comment_reparses() {
        let once = format("/** سطر واحد\nسطر آخر */\nدالة س() {}");
        assert!(once.contains("/// سطر واحد"));
        assert!(once.contains("/// سطر آخر"));
        crate::parser::Parser::new(&once)
            .parse()
            .expect("re-parse must succeed");
    }

    /// `/** */` lexes to `BlockDocComment("")`; emitting nothing would silently
    /// delete it, so an empty doc still produces a bare `///`.
    #[test]
    fn test_format_empty_block_doc_comment_is_not_dropped() {
        let once = format("/** */\nدالة س() {}");
        assert!(once.contains("///"), "empty doc must not vanish: {once}");
        assert_eq!(once, format_raw(&once));
    }

    #[test]
    fn test_format_class_member_doc_comments_keep_markers() {
        let once = format(
            "صنف ش {\n\
             /// وثيقة الحقل\n\
             خاص اسم: نص\n\
             /// وثيقة المنشئ\n\
             منشئ() {}\n\
             /// وثيقة الدالة\n\
             عام دالة تحية() {}\n\
             }",
        );
        assert!(once.contains("/// وثيقة الحقل"), "got: {once}");
        assert!(once.contains("/// وثيقة المنشئ"), "got: {once}");
        assert!(once.contains("/// وثيقة الدالة"), "got: {once}");
        crate::parser::Parser::new(&once)
            .parse()
            .expect("re-parse must succeed");
        assert_eq!(once, format_raw(&once));
    }

    #[test]
    fn test_format_interface_method_doc_comment_keeps_marker() {
        let once = format("ميثاق م {\n/// وثيقة الدالة\nدالة تحية()\n}");
        assert!(once.contains("/// وثيقة الدالة"), "got: {once}");
        crate::parser::Parser::new(&once)
            .parse()
            .expect("re-parse must succeed");
        assert_eq!(once, format_raw(&once));
    }

    #[test]
    fn test_format_enum_variant_doc_comment_keeps_marker() {
        let once = format("تعداد ل {\n/// وثيقة الحالة\nأحمر\n}");
        assert!(once.contains("/// وثيقة الحالة"), "got: {once}");
        crate::parser::Parser::new(&once)
            .parse()
            .expect("re-parse must succeed");
        assert_eq!(once, format_raw(&once));
    }

    /// #204: the doc belongs to the exported declaration but must be printed
    /// *above* `صدّر`. Emitting it after `صدّر` would comment out the
    /// declaration itself.
    #[test]
    fn test_format_exported_declaration_doc_comment_sits_above_export_keyword() {
        for source in [
            "/// وثيقة\nصدّر دالة س() {}",
            "/// وثيقة\nصدّر صنف ن { عام س: عدد }",
            "/// وثيقة\nصدّر ثابت الإصدار = \"١.٠.٠\"",
        ] {
            let once = format(source);
            let doc_pos = once
                .find("/// وثيقة")
                .unwrap_or_else(|| panic!("doc comment must survive: {once}"));
            let export_pos = once.find("صدّر").expect("صدّر must be present");
            assert!(
                doc_pos < export_pos,
                "doc must precede صدّر, not sit mid-line after it: {once}"
            );
            crate::parser::Parser::new(&once)
                .parse()
                .expect("re-parse must succeed");
            assert_eq!(once, format_raw(&once));
        }
    }

    /// `غير_ذلك` is its own arm production (LANGUAGE_SPEC §15.6); prefixing it
    /// with `حالة` produced `حالة غير_ذلك`, rejected as `رمز غير متوقع: Default`.
    #[test]
    fn test_format_match_wildcard_arm_has_no_case_keyword() {
        let once = format("تطابق (س) {\nحالة ١ => اطبع(\"واحد\")\nغير_ذلك => اطبع(\"آخر\")\n}");
        assert!(
            !once.contains("حالة غير_ذلك"),
            "wildcard arm must not be prefixed with حالة: {once}"
        );
        assert!(once.contains("غير_ذلك"), "got: {once}");
        crate::parser::Parser::new(&once)
            .parse()
            .expect("re-parse must succeed");
        assert_eq!(once, format_raw(&once));
    }

    /// Corpus guard: `fmt` must never produce output it cannot read back, and
    /// must be a fixed point, across every real program in the repo. Walks the
    /// tree at runtime rather than `include_str!`ing a single file, so new
    /// stdlib/example files are covered automatically.
    #[test]
    fn test_format_repo_corpus_is_reparsable_and_idempotent() {
        fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect(&path, out);
                } else if path.extension().and_then(|e| e.to_str()) == Some("ترقيم") {
                    out.push(path);
                }
            }
        }

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        collect(&root.join("stdlib_trq"), &mut files);
        collect(&root.join("examples"), &mut files);
        files.sort();

        let config = FormatConfig::default();
        let mut parseable = 0;
        let mut failures = Vec::new();

        for path in &files {
            let Ok(source) = std::fs::read_to_string(path) else {
                continue;
            };
            // Files that do not parse as *input* are a separate concern
            // (#197/#202/#203) and are skipped, not asserted on.
            if crate::parser::Parser::new(&source).parse().is_err() {
                continue;
            }
            parseable += 1;

            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .display()
                .to_string();
            let once = match crate::fmt::format_source(&source, &config) {
                Ok(once) => once,
                Err(e) => {
                    failures.push(format!("{rel}: first pass failed: {e}"));
                    continue;
                }
            };
            match crate::fmt::format_source(&once, &config) {
                Ok(twice) if twice == once => {}
                Ok(_) => failures.push(format!("{rel}: fmt(fmt(x)) != fmt(x)")),
                Err(e) => failures.push(format!("{rel}: second pass failed: {e}")),
            }
        }

        assert!(
            failures.is_empty(),
            "{} of {parseable} parseable corpus files are not fmt-stable:\n{}",
            failures.len(),
            failures.join("\n")
        );

        // Measured on this corpus at the time of the #201 fix. A parser
        // regression that made files unparseable would otherwise silently skip
        // them and leave this test vacuously green.
        assert!(
            parseable >= 33,
            "corpus coverage shrank: only {parseable} of {} files parse (expected >= 33) \
             — a parser regression is hiding behind the skip branch",
            files.len()
        );
    }
}
