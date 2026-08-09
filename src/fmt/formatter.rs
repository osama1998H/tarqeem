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
        // Output leading line comments
        for comment in &stmt.leading_comments {
            p.write("//");
            p.write(comment);
            p.newline();
        }

        self.format_doc_comment_for_stmt(&stmt.kind, p);

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
        // stripped by the lexer — trim_start() normalizes both to one space
        // after `//` regardless of which comment kind produced the text.
        if let Some(trailing) = &stmt.trailing_comment {
            p.write("  // ");
            p.write(trailing.trim_start());
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
                self.format_stmt(stmt, p);
            }
        }
    }

    fn format_block(&self, block: &Block, p: &mut Printer) {
        for stmt in &block.statements {
            self.format_stmt(stmt, p);
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
                p.write_arrow();
                self.format_type(return_type, p);
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
                doc_comment,
            } => {
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
                doc_comment,
            } => {
                p.blank_line();

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
                doc_comment,
            } => {
                p.blank_line();

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
                doc_comment,
            } => {
                p.blank_line();

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
        p.write("حالة");
        p.write_space();

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

    fn format_doc_comment(&self, doc: &str, p: &mut Printer) {
        for line in doc.lines() {
            if !line.is_empty() {
                p.write_space();
                p.write(line);
            }
            p.newline();
        }
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
    // bundle). These pin formatter round-trip behavior for the parser fixes
    // landing elsewhere; both are expected to stay RED until the parser
    // accepts a comment-only function body / a trailing `///` respectively
    // (today, `format()`'s `.expect("Parse failed")` panics on both inputs).

    #[test]
    fn test_format_comment_only_function_body_does_not_panic() {
        // Known limitation: the comment is currently dropped from the
        // formatted output (tracked as a follow-up), so we only assert
        // that formatting a comment-only function body does not panic.
        let _ = format("دالة س() {\n    // تعليق\n}");
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
}
