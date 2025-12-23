//! AST-based code formatter
//!
//! Traverses the AST and generates formatted output.

use super::{FormatConfig, Printer};
use crate::parser::{
    Ast, BinaryOp, Block, ClassMember, Expr, ExprKind, ImportItems, LambdaBody, Literal, MatchArm,
    MethodSignature, Param, Stmt, StmtKind, TypeAnnotation, TypeKind, UnaryOp, Visibility,
};

/// Code formatter that traverses the AST
pub struct Formatter {
    config: FormatConfig,
}

impl Formatter {
    /// Create a new formatter with the given configuration
    pub fn new(config: FormatConfig) -> Self {
        Self { config }
    }

    /// Format an AST into a string
    pub fn format(&self, ast: &Ast) -> String {
        let mut printer = Printer::new(self.config.clone());
        self.format_ast(ast, &mut printer);
        printer.finish()
    }

    /// Format the AST
    fn format_ast(&self, ast: &Ast, p: &mut Printer) {
        // Output file start marker if present
        if ast.has_file_markers() {
            p.write("بسم_الله");
            p.newline();
            p.newline();
        }

        let mut prev_was_import = false;
        let mut first = true;

        for stmt in &ast.statements {
            // Add blank lines between declarations
            if !first {
                let is_import = matches!(&stmt.kind, StmtKind::Import { .. });

                if prev_was_import && !is_import {
                    // Blank line after imports
                    p.blank_lines(self.config.blank_lines_after_imports);
                } else if matches!(
                    &stmt.kind,
                    StmtKind::FuncDecl { .. }
                        | StmtKind::ClassDecl { .. }
                        | StmtKind::InterfaceDecl { .. }
                ) {
                    // Blank line between top-level declarations
                    p.blank_lines(self.config.blank_lines_between_functions);
                }

                prev_was_import = is_import;
            }

            self.format_stmt(stmt, p);
            first = false;
        }

        // Output file end marker if present
        if ast.has_file_markers() {
            p.newline();
            p.write("الحمد_لله");
            p.newline();
        }
    }

    /// Format a statement
    fn format_stmt(&self, stmt: &Stmt, p: &mut Printer) {
        // Format doc comment if present
        self.format_doc_comment_for_stmt(&stmt.kind, p);

        match &stmt.kind {
            StmtKind::VarDecl {
                name,
                mutable,
                ty,
                init,
                ..
            } => {
                // متغير/ثابت keyword
                if *mutable {
                    p.write("متغير");
                } else {
                    p.write("ثابت");
                }
                p.write_space();
                p.write(name);

                // Type annotation
                if let Some(ty) = ty {
                    p.write_colon();
                    self.format_type(ty, p);
                }

                // Initializer
                if let Some(init) = init {
                    p.write_operator("=");
                    self.format_expr(init, p);
                }

                p.newline();
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

                // Parameters
                p.write_parens(|p| {
                    self.format_params(params, p);
                });

                // Return type
                if let Some(ret) = return_type {
                    p.write_arrow();
                    self.format_type(ret, p);
                }

                // Body
                p.write_block(|p| {
                    self.format_block(body, p);
                });
                p.newline();
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

                // Generic type parameters
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

                // Extends
                if let Some(parent) = extends {
                    p.write_space();
                    p.write("يرث");
                    p.write_space();
                    p.write(parent);
                }

                // Implements
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

                // Body
                p.write_block(|p| {
                    for member in members {
                        self.format_class_member(member, p);
                    }
                });
                p.newline();
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

                // Generic type parameters
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

                // Body
                p.write_block(|p| {
                    for method in methods {
                        self.format_method_signature(method, p);
                    }
                });
                p.newline();
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
                p.newline();
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
                p.newline();
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
                p.newline();
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
                p.newline();
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
                p.newline();
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
                p.newline();
            }

            StmtKind::Return(expr) => {
                p.write("أرجع");
                if let Some(e) = expr {
                    p.write_space();
                    self.format_expr(e, p);
                }
                p.newline();
            }

            StmtKind::Break => {
                p.write("أوقف");
                p.newline();
            }

            StmtKind::Continue => {
                p.write("استمر");
                p.newline();
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
                p.newline();
            }

            StmtKind::Throw(expr) => {
                p.write("ارمِ");
                p.write_space();
                self.format_expr(expr, p);
                p.newline();
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
                p.newline();
            }

            StmtKind::Export(inner) => {
                p.write("صدّر");
                p.write_space();
                self.format_stmt_inline(inner, p);
                p.newline();
            }

            StmtKind::Expr(expr) => {
                self.format_expr(expr, p);
                p.newline();
            }

            StmtKind::Block(block) => {
                p.write_block(|p| {
                    self.format_block(block, p);
                });
                p.newline();
            }
        }
    }

    /// Format a statement inline (without trailing newline)
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
                // For other statements, format normally but skip newline
                self.format_stmt(stmt, p);
            }
        }
    }

    /// Format a block of statements
    fn format_block(&self, block: &Block, p: &mut Printer) {
        for stmt in &block.statements {
            self.format_stmt(stmt, p);
        }
    }

    /// Format an expression
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
                // Format type arguments if present
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
        }
    }

    /// Format a literal value
    fn format_literal(&self, lit: &Literal, p: &mut Printer) {
        match lit {
            Literal::Int(n) => p.write(&n.to_string()),
            Literal::Float(f) => p.write(&f.to_string()),
            Literal::String(s) => {
                p.write_char('"');
                // Escape special characters
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

    /// Format a type annotation
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

    /// Format function/method parameters
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

    /// Format a class member
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
                // Doc comment
                if let Some(doc) = doc_comment.as_ref() {
                    self.format_doc_comment(doc, p);
                }

                // Visibility
                self.format_visibility(*visibility, p);

                // Static
                if *is_static {
                    p.write("مشترك");
                    p.write_space();
                }

                // Name and type
                p.write(name);
                if let Some(ty) = ty {
                    p.write_colon();
                    self.format_type(ty, p);
                }

                // Initializer
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
                // Blank line before methods
                p.blank_line();

                // Doc comment
                if let Some(doc) = doc_comment.as_ref() {
                    self.format_doc_comment(doc, p);
                }

                // Visibility
                self.format_visibility(*visibility, p);

                // Static
                if *is_static {
                    p.write("مشترك");
                    p.write_space();
                }

                // Async
                if *is_async {
                    p.write("متوازي");
                    p.write_space();
                }

                p.write("دالة");
                p.write_space();
                p.write(name);

                // Parameters
                p.write_parens(|p| {
                    self.format_params(params, p);
                });

                // Return type
                if let Some(ret) = return_type {
                    p.write_arrow();
                    self.format_type(ret, p);
                }

                // Body
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
                // Blank line before constructor
                p.blank_line();

                // Doc comment
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
        }
    }

    /// Format a method signature (for interfaces)
    fn format_method_signature(&self, sig: &MethodSignature, p: &mut Printer) {
        // Doc comment
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

    /// Format a match arm
    fn format_match_arm(&self, arm: &MatchArm, p: &mut Printer) {
        p.write("حالة");
        p.write_space();

        for (i, pattern) in arm.patterns.iter().enumerate() {
            if i > 0 {
                p.write_comma();
            }
            self.format_expr(pattern, p);
        }

        p.write_fat_arrow();
        p.write_block(|p| {
            self.format_block(&arm.body, p);
        });
        p.newline();
    }

    /// Format import items
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

    /// Format visibility modifier
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

    /// Format a documentation comment
    fn format_doc_comment(&self, doc: &str, p: &mut Printer) {
        for line in doc.lines() {
            p.write("///");
            if !line.is_empty() {
                p.write_space();
                p.write(line);
            }
            p.newline();
        }
    }

    /// Format doc comment for a statement if present
    fn format_doc_comment_for_stmt(&self, kind: &StmtKind, p: &mut Printer) {
        let doc = match kind {
            StmtKind::VarDecl { doc_comment, .. } => doc_comment.as_ref(),
            StmtKind::FuncDecl { doc_comment, .. } => doc_comment.as_ref(),
            StmtKind::ClassDecl { doc_comment, .. } => doc_comment.as_ref(),
            StmtKind::InterfaceDecl { doc_comment, .. } => doc_comment.as_ref(),
            _ => None,
        };

        if let Some(doc) = doc {
            self.format_doc_comment(doc, p);
        }
    }

    /// Get string representation of binary operator
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

    /// Helper to wrap source code with required file markers
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
        // Output includes file markers
        assert!(result.starts_with("بسم_الله\n"));
        assert!(result.contains("متغير س = 5"));
        assert!(result.ends_with("الحمد_لله\n"));
    }

    #[test]
    fn test_format_constant() {
        let result = format("ثابت ط = 3.14");
        // Output includes file markers
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
}
