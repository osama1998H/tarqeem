//! Semantic analyzer for Tarqeem

use super::scope::{Scope, ScopeKind, Symbol, SymbolKind};
use super::types::{parse_type_name, Type};
use crate::error::{Diagnostic, Language, Span};
use crate::parser::*;

/// The semantic analyzer
pub struct Analyzer {
    /// Current scope
    scope: Scope,
    /// Collected diagnostics
    diagnostics: Vec<Diagnostic>,
    /// Language for error messages
    language: Language,
}

impl Analyzer {
    /// Create a new analyzer
    pub fn new() -> Self {
        Self {
            scope: Scope::new_global(),
            diagnostics: Vec::new(),
            language: Language::Arabic,
        }
    }

    /// Set the language for error messages
    pub fn with_language(mut self, lang: Language) -> Self {
        self.language = lang;
        self
    }

    /// Analyze a program
    pub fn analyze(&mut self, ast: &Ast) -> Result<(), Vec<Diagnostic>> {
        for stmt in &ast.statements {
            self.analyze_stmt(stmt);
        }

        if self.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.diagnostics))
        }
    }

    /// Get collected diagnostics
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    // ============ Statement Analysis ============

    fn analyze_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::VarDecl {
                name,
                mutable,
                ty,
                init,
            } => {
                self.analyze_var_decl(name, *mutable, ty.as_ref(), init.as_ref(), stmt.span);
            }

            StmtKind::FuncDecl {
                name,
                params,
                return_type,
                body,
                is_async,
            } => {
                self.analyze_func_decl(name, params, return_type.as_ref(), body, *is_async, stmt.span);
            }

            StmtKind::ClassDecl {
                name,
                extends,
                implements,
                members,
            } => {
                self.analyze_class_decl(name, extends.as_ref(), implements, members, stmt.span);
            }

            StmtKind::InterfaceDecl { name, methods } => {
                self.analyze_interface_decl(name, methods, stmt.span);
            }

            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.analyze_if(condition, then_branch, else_branch.as_ref());
            }

            StmtKind::While { condition, body } => {
                self.analyze_while(condition, body);
            }

            StmtKind::For {
                init,
                condition,
                update,
                body,
            } => {
                self.analyze_for(init.as_deref(), condition.as_ref(), update.as_ref(), body);
            }

            StmtKind::ForIn {
                variable,
                iterable,
                body,
            } => {
                self.analyze_for_in(variable, iterable, body, stmt.span);
            }

            StmtKind::Match { expr, arms } => {
                self.analyze_match(expr, arms);
            }

            StmtKind::Return(value) => {
                self.analyze_return(value.as_ref(), stmt.span);
            }

            StmtKind::Break => {
                if !self.scope.is_in_loop() {
                    self.error(
                        "'break' outside of loop",
                        "'أوقف' خارج الحلقة",
                        stmt.span,
                    );
                }
            }

            StmtKind::Continue => {
                if !self.scope.is_in_loop() {
                    self.error(
                        "'continue' outside of loop",
                        "'استمر' خارج الحلقة",
                        stmt.span,
                    );
                }
            }

            StmtKind::Try { body, catch, finally } => {
                self.analyze_try(body, catch.as_ref(), finally.as_ref());
            }

            StmtKind::Throw(expr) => {
                self.analyze_expr(expr);
            }

            StmtKind::Import { items, from } => {
                self.analyze_import(items, from, stmt.span);
            }

            StmtKind::Export(inner) => {
                self.analyze_stmt(inner);
            }

            StmtKind::Expr(expr) => {
                self.analyze_expr(expr);
            }

            StmtKind::Block(block) => {
                self.analyze_block(block, ScopeKind::Block);
            }
        }
    }

    fn analyze_var_decl(
        &mut self,
        name: &str,
        mutable: bool,
        ty: Option<&TypeAnnotation>,
        init: Option<&Expr>,
        span: Span,
    ) {
        // Infer or check type
        let var_type = if let Some(type_ann) = ty {
            self.resolve_type(type_ann)
        } else if let Some(init_expr) = init {
            self.infer_type(init_expr)
        } else {
            self.error(
                "Variable must have a type annotation or initializer",
                "المتغير يجب أن يحتوي على نوع أو قيمة ابتدائية",
                span,
            );
            Type::Error
        };

        // Check initializer type matches
        if let (Some(init_expr), Some(type_ann)) = (init, ty) {
            let init_type = self.infer_type(init_expr);
            let expected = self.resolve_type(type_ann);
            if !init_type.is_compatible_with(&expected) {
                self.error(
                    &format!(
                        "Type mismatch: expected {}, got {}",
                        expected, init_type
                    ),
                    &format!(
                        "عدم تطابق الأنواع: متوقع {}، وُجد {}",
                        expected.arabic_name(),
                        init_type.arabic_name()
                    ),
                    init_expr.span,
                );
            }
        }

        // Define the variable
        let symbol = Symbol::variable(name, var_type, mutable);
        if !self.scope.define(symbol) {
            self.error(
                &format!("Variable '{}' is already defined", name),
                &format!("المتغير '{}' معرّف مسبقاً", name),
                span,
            );
        }
    }

    fn analyze_func_decl(
        &mut self,
        name: &str,
        params: &[Param],
        return_type: Option<&TypeAnnotation>,
        body: &Block,
        _is_async: bool,
        span: Span,
    ) {
        // Resolve parameter types
        let param_types: Vec<Type> = params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(|t| self.resolve_type(t))
                    .unwrap_or(Type::Any)
            })
            .collect();

        // Resolve return type
        let ret_type = return_type
            .map(|t| self.resolve_type(t))
            .unwrap_or(Type::Void);

        // Define the function in current scope
        let symbol = Symbol::function(name, param_types.clone(), ret_type.clone());
        if !self.scope.define(symbol) {
            self.error(
                &format!("Function '{}' is already defined", name),
                &format!("الدالة '{}' معرّفة مسبقاً", name),
                span,
            );
        }

        // Create a new scope for the function body
        self.push_scope(ScopeKind::Function);

        // Define parameters in the function scope
        for (param, param_type) in params.iter().zip(param_types.iter()) {
            let symbol = Symbol {
                name: param.name.clone(),
                kind: SymbolKind::Parameter,
                ty: param_type.clone(),
                mutable: false,
                defined: true,
            };
            self.scope.define(symbol);
        }

        // Analyze function body
        for stmt in &body.statements {
            self.analyze_stmt(stmt);
        }

        self.pop_scope();
    }

    fn analyze_class_decl(
        &mut self,
        name: &str,
        extends: Option<&String>,
        implements: &[String],
        members: &[ClassMember],
        span: Span,
    ) {
        // Check parent class exists
        if let Some(parent_name) = extends {
            if self.scope.lookup(parent_name).is_none() {
                self.error(
                    &format!("Unknown superclass '{}'", parent_name),
                    &format!("صنف أب غير معروف '{}'", parent_name),
                    span,
                );
            }
        }

        // Check interfaces exist
        for iface in implements {
            if self.scope.lookup(iface).is_none() {
                self.error(
                    &format!("Unknown interface '{}'", iface),
                    &format!("واجهة غير معروفة '{}'", iface),
                    span,
                );
            }
        }

        // Define the class
        let symbol = Symbol::class(name);
        if !self.scope.define(symbol) {
            self.error(
                &format!("Class '{}' is already defined", name),
                &format!("الصنف '{}' معرّف مسبقاً", name),
                span,
            );
        }

        // Analyze class members in a class scope
        self.push_scope(ScopeKind::Class);

        for member in members {
            match member {
                ClassMember::Field { name, ty, init, .. } => {
                    let field_type = ty
                        .as_ref()
                        .map(|t| self.resolve_type(t))
                        .unwrap_or(Type::Any);

                    if let Some(init_expr) = init {
                        let init_type = self.infer_type(init_expr);
                        if !init_type.is_compatible_with(&field_type) {
                            self.error(
                                &format!(
                                    "Type mismatch in field '{}': expected {}, got {}",
                                    name, field_type, init_type
                                ),
                                &format!(
                                    "عدم تطابق الأنواع في الحقل '{}': متوقع {}، وُجد {}",
                                    name,
                                    field_type.arabic_name(),
                                    init_type.arabic_name()
                                ),
                                init_expr.span,
                            );
                        }
                    }

                    self.scope.define(Symbol::variable(name, field_type, true));
                }

                ClassMember::Method {
                    name,
                    params,
                    return_type,
                    body,
                    ..
                } => {
                    self.analyze_func_decl(
                        name,
                        params,
                        return_type.as_ref(),
                        body,
                        false,
                        body.span,
                    );
                }

                ClassMember::Constructor { params, body } => {
                    self.push_scope(ScopeKind::Function);

                    for param in params {
                        let param_type = param
                            .ty
                            .as_ref()
                            .map(|t| self.resolve_type(t))
                            .unwrap_or(Type::Any);
                        self.scope
                            .define(Symbol::variable(&param.name, param_type, false));
                    }

                    for stmt in &body.statements {
                        self.analyze_stmt(stmt);
                    }

                    self.pop_scope();
                }
            }
        }

        self.pop_scope();
    }

    fn analyze_interface_decl(&mut self, name: &str, _methods: &[MethodSignature], span: Span) {
        let symbol = Symbol {
            name: name.to_string(),
            kind: SymbolKind::Interface,
            ty: Type::Interface(name.to_string()),
            mutable: false,
            defined: true,
        };

        if !self.scope.define(symbol) {
            self.error(
                &format!("Interface '{}' is already defined", name),
                &format!("الواجهة '{}' معرّفة مسبقاً", name),
                span,
            );
        }
    }

    fn analyze_if(&mut self, condition: &Expr, then_branch: &Block, else_branch: Option<&Block>) {
        let cond_type = self.infer_type(condition);
        if !cond_type.is_compatible_with(&Type::Bool) {
            self.error(
                &format!("Condition must be boolean, got {}", cond_type),
                &format!(
                    "الشرط يجب أن يكون منطقياً، وُجد {}",
                    cond_type.arabic_name()
                ),
                condition.span,
            );
        }

        self.analyze_block(then_branch, ScopeKind::Block);

        if let Some(else_block) = else_branch {
            self.analyze_block(else_block, ScopeKind::Block);
        }
    }

    fn analyze_while(&mut self, condition: &Expr, body: &Block) {
        let cond_type = self.infer_type(condition);
        if !cond_type.is_compatible_with(&Type::Bool) {
            self.error(
                &format!("Condition must be boolean, got {}", cond_type),
                &format!(
                    "الشرط يجب أن يكون منطقياً، وُجد {}",
                    cond_type.arabic_name()
                ),
                condition.span,
            );
        }

        self.analyze_block(body, ScopeKind::Loop);
    }

    fn analyze_for(
        &mut self,
        init: Option<&Stmt>,
        condition: Option<&Expr>,
        update: Option<&Expr>,
        body: &Block,
    ) {
        self.push_scope(ScopeKind::Loop);

        if let Some(init_stmt) = init {
            self.analyze_stmt(init_stmt);
        }

        if let Some(cond_expr) = condition {
            let cond_type = self.infer_type(cond_expr);
            if !cond_type.is_compatible_with(&Type::Bool) {
                self.error(
                    &format!("Condition must be boolean, got {}", cond_type),
                    &format!(
                        "الشرط يجب أن يكون منطقياً، وُجد {}",
                        cond_type.arabic_name()
                    ),
                    cond_expr.span,
                );
            }
        }

        if let Some(update_expr) = update {
            self.analyze_expr(update_expr);
        }

        for stmt in &body.statements {
            self.analyze_stmt(stmt);
        }

        self.pop_scope();
    }

    fn analyze_for_in(&mut self, variable: &str, iterable: &Expr, body: &Block, _span: Span) {
        let iter_type = self.infer_type(iterable);

        // Determine element type
        let elem_type = match iter_type {
            Type::Array(inner) => *inner,
            Type::String => Type::String,
            Type::Map(_, v) => *v,
            _ => {
                self.error(
                    &format!("Cannot iterate over {}", iter_type),
                    &format!("لا يمكن التكرار على {}", iter_type.arabic_name()),
                    iterable.span,
                );
                Type::Error
            }
        };

        self.push_scope(ScopeKind::Loop);
        self.scope.define(Symbol::variable(variable, elem_type, false));

        for stmt in &body.statements {
            self.analyze_stmt(stmt);
        }

        self.pop_scope();
    }

    fn analyze_match(&mut self, expr: &Expr, arms: &[MatchArm]) {
        let match_type = self.infer_type(expr);

        for arm in arms {
            for pattern in &arm.patterns {
                let pattern_type = self.infer_type(pattern);
                if !pattern_type.is_compatible_with(&match_type) {
                    self.error(
                        &format!(
                            "Pattern type {} does not match {}",
                            pattern_type, match_type
                        ),
                        &format!(
                            "نوع النمط {} لا يتطابق مع {}",
                            pattern_type.arabic_name(),
                            match_type.arabic_name()
                        ),
                        pattern.span,
                    );
                }
            }

            self.analyze_block(&arm.body, ScopeKind::Block);
        }
    }

    fn analyze_return(&mut self, value: Option<&Expr>, span: Span) {
        if !self.scope.is_in_function() {
            self.error(
                "'return' outside of function",
                "'أرجع' خارج الدالة",
                span,
            );
            return;
        }

        if let Some(expr) = value {
            self.analyze_expr(expr);
        }
    }

    fn analyze_try(&mut self, body: &Block, catch: Option<&CatchClause>, finally: Option<&Block>) {
        self.analyze_block(body, ScopeKind::Block);

        if let Some(catch_clause) = catch {
            self.push_scope(ScopeKind::Block);
            self.scope
                .define(Symbol::variable(&catch_clause.param, Type::Any, false));

            for stmt in &catch_clause.body.statements {
                self.analyze_stmt(stmt);
            }

            self.pop_scope();
        }

        if let Some(finally_block) = finally {
            self.analyze_block(finally_block, ScopeKind::Block);
        }
    }

    fn analyze_import(&mut self, items: &ImportItems, _from: &str, _span: Span) {
        // For now, just register the imported names
        match items {
            ImportItems::Named(imports) => {
                for import in imports {
                    let name = import.alias.as_ref().unwrap_or(&import.name);
                    self.scope.define(Symbol::variable(name, Type::Any, false));
                }
            }
            ImportItems::Wildcard(alias) => {
                self.scope.define(Symbol::variable(alias, Type::Any, false));
            }
            ImportItems::Default(name) => {
                self.scope.define(Symbol::variable(name, Type::Any, false));
            }
        }
    }

    fn analyze_block(&mut self, block: &Block, kind: ScopeKind) {
        self.push_scope(kind);

        for stmt in &block.statements {
            self.analyze_stmt(stmt);
        }

        self.pop_scope();
    }

    // ============ Expression Analysis ============

    fn analyze_expr(&mut self, expr: &Expr) -> Type {
        self.infer_type(expr)
    }

    fn infer_type(&mut self, expr: &Expr) -> Type {
        match &expr.kind {
            ExprKind::Literal(lit) => match lit {
                Literal::Int(_) => Type::Int,
                Literal::Float(_) => Type::Float,
                Literal::String(_) => Type::String,
                Literal::Bool(_) => Type::Bool,
                Literal::Null => Type::Null,
            },

            ExprKind::Identifier(name) => {
                if let Some(symbol) = self.scope.lookup(name) {
                    symbol.ty.clone()
                } else {
                    self.error(
                        &format!("Unknown identifier '{}'", name),
                        &format!("معرّف غير معروف '{}'", name),
                        expr.span,
                    );
                    Type::Error
                }
            }

            ExprKind::Binary { left, op, right } => {
                let left_type = self.infer_type(left);
                let right_type = self.infer_type(right);

                let op_str = match op {
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
                };

                if let Some(result_type) = left_type.binary_result_type(op_str, &right_type) {
                    result_type
                } else {
                    self.error(
                        &format!(
                            "Cannot apply operator '{}' to {} and {}",
                            op_str, left_type, right_type
                        ),
                        &format!(
                            "لا يمكن تطبيق العامل '{}' على {} و {}",
                            op_str,
                            left_type.arabic_name(),
                            right_type.arabic_name()
                        ),
                        expr.span,
                    );
                    Type::Error
                }
            }

            ExprKind::Unary { op, operand } => {
                let operand_type = self.infer_type(operand);

                let op_str = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                    UnaryOp::PreInc | UnaryOp::PostInc => "++",
                    UnaryOp::PreDec | UnaryOp::PostDec => "--",
                };

                if let Some(result_type) = operand_type.unary_result_type(op_str) {
                    result_type
                } else {
                    self.error(
                        &format!("Cannot apply operator '{}' to {}", op_str, operand_type),
                        &format!(
                            "لا يمكن تطبيق العامل '{}' على {}",
                            op_str,
                            operand_type.arabic_name()
                        ),
                        expr.span,
                    );
                    Type::Error
                }
            }

            ExprKind::Call { callee, args } => {
                let callee_type = self.infer_type(callee);

                match callee_type {
                    Type::Function { params, return_type } => {
                        // Check argument count
                        if args.len() != params.len() {
                            self.error(
                                &format!(
                                    "Expected {} arguments, got {}",
                                    params.len(),
                                    args.len()
                                ),
                                &format!(
                                    "متوقع {} معاملات، وُجد {}",
                                    params.len(),
                                    args.len()
                                ),
                                expr.span,
                            );
                        }

                        // Check argument types
                        for (i, (arg, param_type)) in args.iter().zip(params.iter()).enumerate() {
                            let arg_type = self.infer_type(arg);
                            if !arg_type.is_compatible_with(param_type) {
                                self.error(
                                    &format!(
                                        "Argument {} has wrong type: expected {}, got {}",
                                        i + 1,
                                        param_type,
                                        arg_type
                                    ),
                                    &format!(
                                        "المعامل {} نوعه خاطئ: متوقع {}، وُجد {}",
                                        i + 1,
                                        param_type.arabic_name(),
                                        arg_type.arabic_name()
                                    ),
                                    arg.span,
                                );
                            }
                        }

                        *return_type
                    }
                    Type::Any => {
                        // Allow calling any
                        for arg in args {
                            self.infer_type(arg);
                        }
                        Type::Any
                    }
                    _ => {
                        self.error(
                            &format!("Cannot call non-function type {}", callee_type),
                            &format!(
                                "لا يمكن استدعاء نوع غير دالة {}",
                                callee_type.arabic_name()
                            ),
                            callee.span,
                        );
                        Type::Error
                    }
                }
            }

            ExprKind::Member { object, property: _ } => {
                let _object_type = self.infer_type(object);
                // For now, return Any for member access
                Type::Any
            }

            ExprKind::Index { object, index } => {
                let object_type = self.infer_type(object);
                let index_type = self.infer_type(index);

                match object_type {
                    Type::Array(inner) => {
                        if !index_type.is_compatible_with(&Type::Int) {
                            self.error(
                                "Array index must be an integer",
                                "فهرس المصفوفة يجب أن يكون عدداً صحيحاً",
                                index.span,
                            );
                        }
                        *inner
                    }
                    Type::Map(k, v) => {
                        if !index_type.is_compatible_with(&k) {
                            self.error(
                                &format!(
                                    "Map key has wrong type: expected {}, got {}",
                                    k, index_type
                                ),
                                &format!(
                                    "مفتاح القاموس نوعه خاطئ: متوقع {}، وُجد {}",
                                    k.arabic_name(),
                                    index_type.arabic_name()
                                ),
                                index.span,
                            );
                        }
                        *v
                    }
                    Type::String => {
                        if !index_type.is_compatible_with(&Type::Int) {
                            self.error(
                                "String index must be an integer",
                                "فهرس النص يجب أن يكون عدداً صحيحاً",
                                index.span,
                            );
                        }
                        Type::String
                    }
                    _ => {
                        self.error(
                            &format!("Cannot index into {}", object_type),
                            &format!("لا يمكن الفهرسة في {}", object_type.arabic_name()),
                            object.span,
                        );
                        Type::Error
                    }
                }
            }

            ExprKind::Assignment { target, value } => {
                let value_type = self.infer_type(value);

                // Check if target is assignable
                match &target.kind {
                    ExprKind::Identifier(name) => {
                        // Clone symbol info first to avoid borrow conflicts
                        let symbol_info = self.scope.lookup(name).map(|s| (s.mutable, s.ty.clone()));

                        if let Some((mutable, ty)) = symbol_info {
                            if !mutable {
                                self.error(
                                    &format!("Cannot assign to immutable variable '{}'", name),
                                    &format!("لا يمكن تعيين قيمة لمتغير ثابت '{}'", name),
                                    target.span,
                                );
                            }
                            if !value_type.is_compatible_with(&ty) {
                                self.error(
                                    &format!(
                                        "Type mismatch: expected {}, got {}",
                                        ty, value_type
                                    ),
                                    &format!(
                                        "عدم تطابق الأنواع: متوقع {}، وُجد {}",
                                        ty.arabic_name(),
                                        value_type.arabic_name()
                                    ),
                                    value.span,
                                );
                            }
                        } else {
                            self.error(
                                &format!("Unknown variable '{}'", name),
                                &format!("متغير غير معروف '{}'", name),
                                target.span,
                            );
                        }
                    }
                    ExprKind::Member { object, .. } | ExprKind::Index { object, .. } => {
                        self.infer_type(object);
                    }
                    _ => {
                        self.error(
                            "Invalid assignment target",
                            "هدف تعيين غير صالح",
                            target.span,
                        );
                    }
                }

                value_type
            }

            ExprKind::CompoundAssignment { target, op: _, value } => {
                // Similar to assignment but with operation
                self.infer_type(target);
                self.infer_type(value);
                self.infer_type(target)
            }

            ExprKind::Array(elements) => {
                if elements.is_empty() {
                    Type::Array(Box::new(Type::Unknown))
                } else {
                    let first_type = self.infer_type(&elements[0]);
                    for elem in elements.iter().skip(1) {
                        let elem_type = self.infer_type(elem);
                        if !elem_type.is_compatible_with(&first_type) {
                            self.error(
                                &format!(
                                    "Array element has wrong type: expected {}, got {}",
                                    first_type, elem_type
                                ),
                                &format!(
                                    "عنصر المصفوفة نوعه خاطئ: متوقع {}، وُجد {}",
                                    first_type.arabic_name(),
                                    elem_type.arabic_name()
                                ),
                                elem.span,
                            );
                        }
                    }
                    Type::Array(Box::new(first_type))
                }
            }

            ExprKind::Object(pairs) => {
                for (_, value) in pairs {
                    self.infer_type(value);
                }
                Type::Map(Box::new(Type::String), Box::new(Type::Any))
            }

            ExprKind::Lambda { params, body } => {
                self.push_scope(ScopeKind::Function);

                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| {
                        let ty = p
                            .ty
                            .as_ref()
                            .map(|t| self.resolve_type(t))
                            .unwrap_or(Type::Any);
                        self.scope.define(Symbol::variable(&p.name, ty.clone(), false));
                        ty
                    })
                    .collect();

                let return_type = match body {
                    LambdaBody::Expr(expr) => self.infer_type(expr),
                    LambdaBody::Block(block) => {
                        for stmt in &block.statements {
                            self.analyze_stmt(stmt);
                        }
                        Type::Void
                    }
                };

                self.pop_scope();

                Type::Function {
                    params: param_types,
                    return_type: Box::new(return_type),
                }
            }

            ExprKind::New { class, args } => {
                // Check class exists
                let class_type = self.infer_type(class);
                for arg in args {
                    self.infer_type(arg);
                }
                class_type
            }

            ExprKind::Await(inner) => {
                // Await unwraps a promise/future
                self.infer_type(inner)
            }

            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_type = self.infer_type(condition);
                if !cond_type.is_compatible_with(&Type::Bool) {
                    self.error(
                        "Ternary condition must be boolean",
                        "شرط العامل الثلاثي يجب أن يكون منطقياً",
                        condition.span,
                    );
                }

                let then_type = self.infer_type(then_expr);
                let else_type = self.infer_type(else_expr);

                if then_type.is_compatible_with(&else_type) {
                    then_type
                } else {
                    self.error(
                        &format!(
                            "Ternary branches have incompatible types: {} and {}",
                            then_type, else_type
                        ),
                        &format!(
                            "فروع العامل الثلاثي غير متوافقة: {} و {}",
                            then_type.arabic_name(),
                            else_type.arabic_name()
                        ),
                        expr.span,
                    );
                    Type::Error
                }
            }

            ExprKind::Grouping(inner) => self.infer_type(inner),

            ExprKind::This => {
                if !self.scope.is_in_class() {
                    self.error(
                        "'this' outside of class",
                        "'هذا' خارج الصنف",
                        expr.span,
                    );
                }
                Type::Any
            }

            ExprKind::Super => {
                if !self.scope.is_in_class() {
                    self.error(
                        "'super' outside of class",
                        "'أساس' خارج الصنف",
                        expr.span,
                    );
                }
                Type::Any
            }
        }
    }

    // ============ Type Resolution ============

    fn resolve_type(&self, type_ann: &TypeAnnotation) -> Type {
        match &type_ann.kind {
            TypeKind::Simple(name) => parse_type_name(name),
            TypeKind::Array(inner) => Type::Array(Box::new(self.resolve_type(inner))),
            TypeKind::Map(k, v) => {
                Type::Map(Box::new(self.resolve_type(k)), Box::new(self.resolve_type(v)))
            }
            TypeKind::Function { params, return_type } => Type::Function {
                params: params.iter().map(|p| self.resolve_type(p)).collect(),
                return_type: Box::new(self.resolve_type(return_type)),
            },
            TypeKind::Generic { base, args: _ } => {
                // For now, treat generics as the base type
                parse_type_name(base)
            }
            TypeKind::Optional(inner) => Type::Optional(Box::new(self.resolve_type(inner))),
        }
    }

    // ============ Scope Management ============

    fn push_scope(&mut self, kind: ScopeKind) {
        let old_scope = std::mem::replace(&mut self.scope, Scope::new_global());
        self.scope = Scope::new_child(old_scope, kind);
    }

    fn pop_scope(&mut self) {
        if let Some(parent) = std::mem::replace(&mut self.scope, Scope::new_global()).pop() {
            self.scope = parent;
        }
    }

    // ============ Error Reporting ============

    fn error(&mut self, message: &str, message_ar: &str, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(message, message_ar, span));
    }

    #[allow(dead_code)]
    fn warning(&mut self, message: &str, message_ar: &str, span: Span) {
        self.diagnostics
            .push(Diagnostic::warning(message, message_ar, span));
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn analyze(source: &str) -> Result<(), Vec<Diagnostic>> {
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();
        let mut analyzer = Analyzer::new();
        analyzer.analyze(&ast)
    }

    #[test]
    fn test_variable_declaration() {
        let result = analyze("متغير س = 5;");
        assert!(result.is_ok());
    }

    #[test]
    fn test_undefined_variable() {
        let result = analyze("اطبع(س);");
        assert!(result.is_err());
    }

    #[test]
    fn test_type_mismatch() {
        let result = analyze(r#"متغير س: عدد = "نص";"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_function_declaration() {
        let result = analyze(r#"
            دالة جمع(أ: عدد، ب: عدد) -> عدد {
                أرجع أ + ب;
            }
        "#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_break_outside_loop() {
        let result = analyze("أوقف;");
        assert!(result.is_err());
    }

    #[test]
    fn test_return_outside_function() {
        let result = analyze("أرجع 5;");
        assert!(result.is_err());
    }
}
