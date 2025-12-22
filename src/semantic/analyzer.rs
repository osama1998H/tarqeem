//! Semantic analyzer for Tarqeem

use super::class_resolver::ClassResolver;
use super::generics::GenericResolver;
use super::method_resolver::{MemberResolution, MethodResolver};
use super::modules::{ExportKind, ModuleLoader};
use super::scope::{Scope, ScopeKind, Symbol, SymbolKind};
use super::types::{parse_type_name, Type};
use crate::error::{Diagnostic, Language, Span};
use crate::parser::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// The semantic analyzer
pub struct Analyzer {
    /// Current scope
    scope: Scope,
    /// Class resolver for OOP type checking
    class_resolver: ClassResolver,
    /// Generic type resolver for handling generic types
    generic_resolver: GenericResolver,
    /// Module loader for handling imports
    module_loader: ModuleLoader,
    /// Current file being analyzed
    current_file: Option<PathBuf>,
    /// Exported symbols from this module
    exports: HashMap<String, Type>,
    /// Collected diagnostics
    diagnostics: Vec<Diagnostic>,
    /// Language for error messages
    language: Language,
    /// Current class being analyzed (if any)
    current_class: Option<String>,
    /// Expected type for context-aware inference (e.g., for empty arrays)
    /// This is set before calling infer_type to provide context
    expected_type: Option<Type>,
}

impl Analyzer {
    /// Create a new analyzer
    pub fn new() -> Self {
        Self {
            scope: Scope::new_global(),
            class_resolver: ClassResolver::new(),
            generic_resolver: GenericResolver::new(),
            module_loader: ModuleLoader::new(),
            current_file: None,
            exports: HashMap::new(),
            diagnostics: Vec::new(),
            language: Language::Arabic,
            current_class: None,
            expected_type: None,
        }
    }

    /// Create a new analyzer for a specific file
    pub fn for_file(path: PathBuf) -> Self {
        let mut analyzer = Self::new();
        analyzer.current_file = Some(path);
        analyzer
    }

    /// Add a module search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.module_loader.add_search_path(path);
    }

    /// Get exported symbols from this module
    pub fn exports(&self) -> &HashMap<String, Type> {
        &self.exports
    }

    /// Get the class resolver
    pub fn class_resolver(&self) -> &ClassResolver {
        &self.class_resolver
    }

    /// Set the language for error messages
    pub fn with_language(mut self, lang: Language) -> Self {
        self.language = lang;
        self
    }

    /// Analyze a program
    pub fn analyze(&mut self, ast: &Ast) -> Result<(), Vec<Diagnostic>> {
        // First pass: Register all classes and interfaces
        for stmt in &ast.statements {
            self.register_types(stmt);
        }

        // Second pass: Add members to classes and interfaces
        for stmt in &ast.statements {
            self.add_type_members(stmt);
        }

        // Build vtables for class hierarchy
        self.class_resolver.build_vtables();

        // Validate class hierarchy (inheritance, interfaces, etc.)
        if let Err(diags) = self.class_resolver.validate() {
            self.diagnostics.extend(diags);
        }

        // Third pass: Full semantic analysis
        for stmt in &ast.statements {
            self.analyze_stmt(stmt);
        }

        if self.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.diagnostics))
        }
    }

    /// First pass: Register type declarations
    fn register_types(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::ClassDecl {
                name,
                extends,
                implements,
                ..
            } => {
                self.class_resolver
                    .register_class(name, extends.as_deref(), implements, stmt.span);
            }
            StmtKind::InterfaceDecl { name, .. } => {
                self.class_resolver.register_interface(name, &[], stmt.span);
            }
            _ => {}
        }
    }

    /// Second pass: Add members to types
    fn add_type_members(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::ClassDecl { name, members, .. } => {
                // We need to use a standalone function to avoid borrow conflicts
                self.class_resolver
                    .add_class_members(name, members, resolve_type_annotation);
            }
            StmtKind::InterfaceDecl { name, methods, .. } => {
                self.class_resolver
                    .add_interface_methods(name, methods, resolve_type_annotation);
            }
            _ => {}
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
                ..
            } => {
                self.analyze_var_decl(name, *mutable, ty.as_ref(), init.as_ref(), stmt.span);
            }

            StmtKind::FuncDecl {
                name,
                params,
                return_type,
                body,
                is_async,
                ..
            } => {
                self.analyze_func_decl(
                    name,
                    params,
                    return_type.as_ref(),
                    body,
                    *is_async,
                    stmt.span,
                );
            }

            StmtKind::ClassDecl {
                name,
                type_params,
                extends,
                implements,
                members,
                ..
            } => {
                self.analyze_class_decl(
                    name,
                    type_params,
                    extends.as_ref(),
                    implements,
                    members,
                    stmt.span,
                );
            }

            StmtKind::InterfaceDecl { name, methods, .. } => {
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

            StmtKind::DoWhile { body, condition } => {
                self.analyze_do_while(body, condition);
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
                    self.error("'break' outside of loop", "'أوقف' خارج الحلقة", stmt.span);
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

            StmtKind::Try {
                body,
                catch,
                finally,
            } => {
                self.analyze_try(body, catch.as_ref(), finally.as_ref());
            }

            StmtKind::Throw(expr) => {
                self.analyze_throw(expr, stmt.span);
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
        // Resolve type annotation first (if present) for context-aware inference
        let declared_type = ty.map(|t| self.resolve_type(t));

        // Infer or check type
        let var_type = if let Some(ref declared) = declared_type {
            declared.clone()
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
        // Set expected_type for context-aware inference (e.g., empty arrays)
        if let (Some(init_expr), Some(ref expected)) = (init, &declared_type) {
            self.expected_type = Some(expected.clone());
            let init_type = self.infer_type(init_expr);
            self.expected_type = None; // Clear after inference

            if !init_type.is_compatible_with(expected) {
                self.error(
                    &format!("Type mismatch: expected {}, got {}", expected, init_type),
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
        type_params: &[String],
        extends: Option<&String>,
        implements: &[String],
        members: &[ClassMember],
        span: Span,
    ) {
        // Push generic context if this is a generic class
        let has_generics = !type_params.is_empty();
        if has_generics {
            self.enter_generic_context(type_params);
        }

        // Check parent class exists (using class resolver)
        if let Some(parent_name) = extends {
            if self.class_resolver.get_class(parent_name).is_none()
                && self.scope.lookup(parent_name).is_none()
            {
                self.error(
                    &format!("Unknown superclass '{}'", parent_name),
                    &format!("صنف أب غير معروف '{}'", parent_name),
                    span,
                );
            }
        }

        // Check interfaces exist
        for iface in implements {
            if self.class_resolver.get_interface(iface).is_none()
                && self.scope.lookup(iface).is_none()
            {
                self.error(
                    &format!("Unknown interface '{}'", iface),
                    &format!("واجهة غير معروفة '{}'", iface),
                    span,
                );
            }
        }

        // Define the class in scope
        let symbol = Symbol::class(name);
        if !self.scope.define(symbol) {
            self.error(
                &format!("Class '{}' is already defined", name),
                &format!("الصنف '{}' معرّف مسبقاً", name),
                span,
            );
        }

        // Set current class context
        let prev_class = self.current_class.take();
        self.current_class = Some(name.to_string());

        // Analyze class members in a class scope
        self.push_scope(ScopeKind::Class);

        // Define 'this' in class scope
        self.scope.define(Symbol::variable(
            "هذا",
            Type::Class(name.to_string()),
            false,
        ));
        self.scope.define(Symbol::variable(
            "this",
            Type::Class(name.to_string()),
            false,
        ));

        for member in members {
            match member {
                ClassMember::Field {
                    name: field_name,
                    ty,
                    init,
                    ..
                } => {
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
                                    field_name, field_type, init_type
                                ),
                                &format!(
                                    "عدم تطابق الأنواع في الحقل '{}': متوقع {}، وُجد {}",
                                    field_name,
                                    field_type.arabic_name(),
                                    init_type.arabic_name()
                                ),
                                init_expr.span,
                            );
                        }
                    }

                    self.scope
                        .define(Symbol::variable(field_name, field_type, true));
                }

                ClassMember::Method {
                    name: method_name,
                    params,
                    return_type,
                    body,
                    ..
                } => {
                    self.analyze_func_decl(
                        method_name,
                        params,
                        return_type.as_ref(),
                        body,
                        false,
                        body.span,
                    );
                }

                ClassMember::Constructor { params, body, .. } => {
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

        // Pop generic context if this was a generic class
        if has_generics {
            self.exit_generic_context();
        }

        // Restore previous class context
        self.current_class = prev_class;
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
                &format!("الشرط يجب أن يكون منطقياً، وُجد {}", cond_type.arabic_name()),
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
                &format!("الشرط يجب أن يكون منطقياً، وُجد {}", cond_type.arabic_name()),
                condition.span,
            );
        }

        self.analyze_block(body, ScopeKind::Loop);
    }

    fn analyze_do_while(&mut self, body: &Block, condition: &Expr) {
        // For do-while, body is analyzed first (it executes at least once)
        self.analyze_block(body, ScopeKind::Loop);

        let cond_type = self.infer_type(condition);
        if !cond_type.is_compatible_with(&Type::Bool) {
            self.error(
                &format!("Condition must be boolean, got {}", cond_type),
                &format!("الشرط يجب أن يكون منطقياً، وُجد {}", cond_type.arabic_name()),
                condition.span,
            );
        }
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
                    &format!("الشرط يجب أن يكون منطقياً، وُجد {}", cond_type.arabic_name()),
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
        self.scope
            .define(Symbol::variable(variable, elem_type, false));

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
            self.error("'return' outside of function", "'أرجع' خارج الدالة", span);
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
            // The catch parameter is typed as the base error class (استثناء)
            // This ensures .رسالة (message) property access works correctly
            self.scope.define(Symbol::variable(
                &catch_clause.param,
                Type::Class("استثناء".to_string()),
                false,
            ));

            for stmt in &catch_clause.body.statements {
                self.analyze_stmt(stmt);
            }

            self.pop_scope();
        }

        if let Some(finally_block) = finally {
            self.analyze_block(finally_block, ScopeKind::Block);
        }
    }

    /// Analyze a throw statement - requires an error object
    fn analyze_throw(&mut self, expr: &Expr, span: Span) {
        let expr_type = self.analyze_expr(expr);

        // Check that the thrown expression is an error type
        if !self.is_error_type(&expr_type) {
            self.error(
                &format!(
                    "Cannot throw non-error type '{}'. Only error objects (خطأ or subclasses) can be thrown",
                    expr_type
                ),
                &format!(
                    "لا يمكن رمي نوع غير خطأ '{}'. يمكن رمي كائنات الخطأ (خطأ أو أصنافه الفرعية) فقط",
                    expr_type.arabic_name()
                ),
                span,
            );
        }
    }

    fn analyze_import(&mut self, items: &ImportItems, from: &str, span: Span) {
        // Try to resolve and load the module
        let current_file = self
            .current_file
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));

        let module_path = match self.module_loader.resolve_path(&current_file, from) {
            Some(path) => path,
            None => {
                // Module not found - fall back to registering names as Any
                // This allows compilation to proceed with unknown modules
                self.warn(
                    &format!(
                        "Module '{}' not found, imports will be typed as 'any'",
                        from
                    ),
                    &format!(
                        "الوحدة '{}' غير موجودة، سيتم تصنيف الاستيرادات كـ 'أي'",
                        from
                    ),
                    span,
                );
                self.register_imports_as_any(items);
                return;
            }
        };

        // Load the module and clone exports to avoid borrow issues
        let module_exports = match self.module_loader.load_module(&module_path, span) {
            Ok(loaded_module) => {
                // Clone the exports we need
                loaded_module.exports.clone()
            }
            Err(()) => {
                // Module loading failed - diagnostics already added by loader
                // Merge loader diagnostics
                let loader_diagnostics = self.module_loader.take_diagnostics();
                self.diagnostics.extend(loader_diagnostics);

                // Fall back to registering names as Any
                self.register_imports_as_any(items);
                return;
            }
        };

        // Import symbols from the loaded module
        match items {
            ImportItems::Named(imports) => {
                for import in imports {
                    let name = import.alias.as_ref().unwrap_or(&import.name);
                    if let Some(exported) = module_exports.get(&import.name) {
                        // Convert ExportKind to Type
                        let ty = self.export_kind_to_type(&exported.kind, &import.name);
                        self.scope.define(Symbol::variable(name, ty, false));
                    } else {
                        self.error(
                            &format!("Module '{}' has no export named '{}'", from, import.name),
                            &format!("الوحدة '{}' لا تحتوي على تصدير باسم '{}'", from, import.name),
                            span,
                        );
                        // Still register as Any to allow compilation to continue
                        self.scope.define(Symbol::variable(name, Type::Any, false));
                    }
                }
            }
            ImportItems::Wildcard(alias) => {
                // Create a namespace object with all exports
                // For now, just register as Any
                self.scope.define(Symbol::variable(alias, Type::Any, false));
            }
            ImportItems::Default(name) => {
                // Look for default export
                if let Some(exported) = module_exports.get("default") {
                    let ty = self.export_kind_to_type(&exported.kind, "default");
                    self.scope.define(Symbol::variable(name, ty, false));
                } else {
                    self.warn(
                        &format!("Module '{}' has no default export", from),
                        &format!("الوحدة '{}' لا تحتوي على تصدير افتراضي", from),
                        span,
                    );
                    self.scope.define(Symbol::variable(name, Type::Any, false));
                }
            }
        }
    }

    /// Helper to register imports as Type::Any when module loading fails
    fn register_imports_as_any(&mut self, items: &ImportItems) {
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

    /// Convert ExportKind to Type
    fn export_kind_to_type(&self, kind: &ExportKind, name: &str) -> Type {
        match kind {
            ExportKind::Function => Type::Function {
                params: vec![],
                return_type: Box::new(Type::Any),
            },
            ExportKind::Class => Type::Class(name.to_string()),
            ExportKind::Interface => Type::Interface(name.to_string()),
            ExportKind::Variable | ExportKind::Constant => Type::Any,
        }
    }

    /// Add a warning diagnostic
    fn warn(&mut self, message: &str, message_ar: &str, span: Span) {
        self.diagnostics
            .push(Diagnostic::warning(message, message_ar, span));
    }

    // ============ Error Type Checking ============

    /// Base error class names (Arabic and English)
    /// Note: "خطأ" is reserved for the `false` boolean, so we use "استثناء" (exception)
    const ERROR_BASE_CLASSES: &'static [&'static str] = &["استثناء", "Exception", "Error"];

    /// Check if a type is an error type (is استثناء or inherits from it)
    fn is_error_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Class(class_name) => {
                // Check if it's a base error class
                if Self::ERROR_BASE_CLASSES.contains(&class_name.as_str()) {
                    return true;
                }
                // Check if it inherits from a base error class
                for base_error in Self::ERROR_BASE_CLASSES {
                    if self.class_resolver.is_subclass(class_name, base_error) {
                        return true;
                    }
                }
                false
            }
            Type::Any => true, // Allow Any for backwards compatibility
            _ => false,
        }
    }

    /// Analyze a super constructor call: أساس(args)
    fn analyze_super_constructor_call(&mut self, args: &[Expr], span: Span) -> Type {
        // Must be inside a class
        if !self.scope.is_in_class() {
            self.error(
                "'super()' can only be used inside a class constructor",
                "'أساس()' يمكن استخدامه فقط داخل منشئ صنف",
                span,
            );
            return Type::Error;
        }

        // Get current class
        let current_class_name = match &self.current_class {
            Some(name) => name.clone(),
            None => {
                self.error(
                    "'super()' can only be used inside a class",
                    "'أساس()' يمكن استخدامه فقط داخل صنف",
                    span,
                );
                return Type::Error;
            }
        };

        // Get current class info
        let parent_name = match self.class_resolver.get_class(&current_class_name) {
            Some(class_info) => match &class_info.parent {
                Some(parent) => parent.clone(),
                None => {
                    self.error(
                        &format!(
                            "Cannot use 'super()' in class '{}' which has no parent class",
                            current_class_name
                        ),
                        &format!(
                            "لا يمكن استخدام 'أساس()' في الصنف '{}' الذي ليس له صنف أب",
                            current_class_name
                        ),
                        span,
                    );
                    return Type::Error;
                }
            },
            None => {
                // This shouldn't happen if we're inside a class
                return Type::Error;
            }
        };

        // Get parent class constructor
        let parent_constructor = match self.class_resolver.get_class(&parent_name) {
            Some(parent_info) => parent_info.constructor.clone(),
            None => {
                self.error(
                    &format!("Parent class '{}' not found", parent_name),
                    &format!("الصنف الأب '{}' غير موجود", parent_name),
                    span,
                );
                return Type::Error;
            }
        };

        // Check if parent has a constructor
        match parent_constructor {
            Some(constructor) => {
                let params = &constructor.params;

                // Check argument count
                if args.len() != params.len() {
                    self.error(
                        &format!(
                            "Parent constructor expects {} arguments, got {}",
                            params.len(),
                            args.len()
                        ),
                        &format!(
                            "منشئ الصنف الأب يتوقع {} معاملات، وُجد {}",
                            params.len(),
                            args.len()
                        ),
                        span,
                    );
                }

                // Check argument types
                for (i, (arg, (_param_name, param_type))) in
                    args.iter().zip(params.iter()).enumerate()
                {
                    let arg_type = self.infer_type(arg);
                    if !arg_type.is_compatible_with(param_type) {
                        self.error(
                            &format!(
                                "Argument {} to super() has wrong type: expected {}, got {}",
                                i + 1,
                                param_type,
                                arg_type
                            ),
                            &format!(
                                "المعامل {} لـ أساس() نوعه خاطئ: متوقع {}، وُجد {}",
                                i + 1,
                                param_type.arabic_name(),
                                arg_type.arabic_name()
                            ),
                            arg.span,
                        );
                    }
                }
            }
            None => {
                // Parent has no explicit constructor, check that no args are passed
                if !args.is_empty() {
                    self.error(
                        &format!(
                            "Parent class '{}' has no constructor, but {} arguments were passed",
                            parent_name,
                            args.len()
                        ),
                        &format!(
                            "الصنف الأب '{}' ليس له منشئ، لكن تم تمرير {} معاملات",
                            parent_name,
                            args.len()
                        ),
                        span,
                    );
                }
            }
        }

        // Super constructor calls don't return a value
        Type::Void
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
                // Special case: super constructor call (أساس(...))
                if matches!(callee.kind, ExprKind::Super) {
                    return self.analyze_super_constructor_call(args, expr.span);
                }

                let callee_type = self.infer_type(callee);

                match callee_type {
                    Type::Function {
                        params,
                        return_type,
                    } => {
                        // Check argument count
                        if args.len() != params.len() {
                            self.error(
                                &format!("Expected {} arguments, got {}", params.len(), args.len()),
                                &format!("متوقع {} معاملات، وُجد {}", params.len(), args.len()),
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
                            &format!("لا يمكن استدعاء نوع غير دالة {}", callee_type.arabic_name()),
                            callee.span,
                        );
                        Type::Error
                    }
                }
            }

            ExprKind::Member { object, property } => {
                let object_type = self.infer_type(object);
                self.resolve_member_type(&object_type, property, expr.span)
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
                        let symbol_info =
                            self.scope.lookup(name).map(|s| (s.mutable, s.ty.clone()));

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
                                    &format!("Type mismatch: expected {}, got {}", ty, value_type),
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

            ExprKind::CompoundAssignment {
                target,
                op: _,
                value,
            } => {
                // Similar to assignment but with operation
                self.infer_type(target);
                self.infer_type(value);
                self.infer_type(target)
            }

            ExprKind::Array(elements) => {
                if elements.is_empty() {
                    // Use expected type for context-aware inference
                    if let Some(Type::Array(elem_ty)) = &self.expected_type {
                        Type::Array(elem_ty.clone())
                    } else {
                        Type::Array(Box::new(Type::Unknown))
                    }
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
                        let ty =
                            p.ty.as_ref()
                                .map(|t| self.resolve_type(t))
                                .unwrap_or(Type::Any);
                        self.scope
                            .define(Symbol::variable(&p.name, ty.clone(), false));
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
                // Extract class name from identifier
                let class_name = match &class.kind {
                    ExprKind::Identifier(name) => name.clone(),
                    _ => {
                        self.error(
                            "New expression requires a class name",
                            "تعبير جديد يتطلب اسم صنف",
                            class.span,
                        );
                        return Type::Error;
                    }
                };

                // Extract constructor info from class resolver (clone to avoid borrow issues)
                let ctor_info = self
                    .class_resolver
                    .get_class(&class_name)
                    .and_then(|ci| ci.constructor.as_ref())
                    .map(|ctor| ctor.params.clone());
                let class_exists = self.class_resolver.get_class(&class_name).is_some();

                if class_exists {
                    // Validate constructor arguments
                    if let Some(expected_params) = ctor_info {
                        // Check argument count
                        if args.len() != expected_params.len() {
                            self.error(
                                &format!(
                                    "Constructor expects {} arguments, got {}",
                                    expected_params.len(),
                                    args.len()
                                ),
                                &format!(
                                    "المنشئ يتوقع {} معاملات، وُجد {}",
                                    expected_params.len(),
                                    args.len()
                                ),
                                expr.span,
                            );
                        }

                        // Type-check each argument
                        for (arg, (_, param_type)) in args.iter().zip(expected_params.iter()) {
                            let arg_type = self.infer_type(arg);
                            if !arg_type.is_compatible_with(param_type) {
                                self.error(
                                    &format!(
                                        "Wrong argument type: expected {}, got {}",
                                        param_type, arg_type
                                    ),
                                    &format!(
                                        "نوع المعامل خاطئ: متوقع {}، وُجد {}",
                                        param_type.arabic_name(),
                                        arg_type.arabic_name()
                                    ),
                                    arg.span,
                                );
                            }
                        }
                    } else if !args.is_empty() {
                        self.error(
                            &format!("Class '{}' has no constructor", class_name),
                            &format!("الصنف '{}' ليس له منشئ", class_name),
                            expr.span,
                        );
                    }

                    Type::Class(class_name)
                } else {
                    self.error(
                        &format!("Unknown class '{}'", class_name),
                        &format!("صنف غير معروف '{}'", class_name),
                        class.span,
                    );
                    Type::Error
                }
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
                    self.error("'this' outside of class", "'هذا' خارج الصنف", expr.span);
                    Type::Error
                } else if let Some(ref class_name) = self.current_class {
                    Type::Class(class_name.clone())
                } else {
                    Type::Any
                }
            }

            ExprKind::Super => {
                if !self.scope.is_in_class() {
                    self.error("'super' outside of class", "'أساس' خارج الصنف", expr.span);
                    Type::Error
                } else if let Some(ref class_name) = self.current_class {
                    // Get the parent class type
                    if let Some(class) = self.class_resolver.get_class(class_name) {
                        if let Some(ref parent_name) = class.parent {
                            Type::Class(parent_name.clone())
                        } else {
                            self.error(
                                "Cannot use 'super' in a class without a parent",
                                "لا يمكن استخدام 'أساس' في صنف بدون أب",
                                expr.span,
                            );
                            Type::Error
                        }
                    } else {
                        Type::Any
                    }
                } else {
                    Type::Any
                }
            }
        }
    }

    // ============ Member Resolution ============

    /// Resolve the type of a member access
    fn resolve_member_type(&mut self, object_type: &Type, property: &str, span: Span) -> Type {
        let mut method_resolver = MethodResolver::new(&self.class_resolver);

        match method_resolver.resolve_member(object_type, property) {
            MemberResolution::Field(field) => field.ty.clone(),
            MemberResolution::Method(method) => Type::Function {
                params: method.params.iter().map(|(_, ty)| ty.clone()).collect(),
                return_type: Box::new(method.return_type.clone()),
            },
            MemberResolution::BuiltinProperty { ty, .. } => ty,
            MemberResolution::NotFound => {
                // Check if it's a valid class type with unknown member
                if let Type::Class(class_name) = object_type {
                    if self.class_resolver.get_class(class_name).is_some() {
                        self.error(
                            &format!(
                                "Property '{}' not found on class '{}'",
                                property, class_name
                            ),
                            &format!(
                                "الخاصية '{}' غير موجودة في الصنف '{}'",
                                property, class_name
                            ),
                            span,
                        );
                    }
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
            TypeKind::Map(k, v) => Type::Map(
                Box::new(self.resolve_type(k)),
                Box::new(self.resolve_type(v)),
            ),
            TypeKind::Function {
                params,
                return_type,
            } => Type::Function {
                params: params.iter().map(|p| self.resolve_type(p)).collect(),
                return_type: Box::new(self.resolve_type(return_type)),
            },
            TypeKind::Generic { base, args } => {
                // Handle built-in generic types like مصفوفة<عدد> (Array<Int>)
                match base.as_str() {
                    "مصفوفة" | "array" | "Array" => {
                        if let Some(elem_type) = args.first() {
                            Type::Array(Box::new(self.resolve_type(elem_type)))
                        } else {
                            Type::Array(Box::new(Type::Unknown))
                        }
                    }
                    "قاموس" | "map" | "Map" | "dict" | "Dict" => {
                        if args.len() >= 2 {
                            Type::Map(
                                Box::new(self.resolve_type(&args[0])),
                                Box::new(self.resolve_type(&args[1])),
                            )
                        } else {
                            // Fallback to class type
                            parse_type_name(base)
                        }
                    }
                    _ => {
                        // For other generics, treat as the base type for now
                        parse_type_name(base)
                    }
                }
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

    // ============ Generic Context Management ============

    /// Enter a generic context for a class or function with type parameters.
    /// This registers the type parameters so they can be recognized during type resolution.
    fn enter_generic_context(&mut self, type_params: &[String]) {
        use super::generics::{GenericContext, GenericParam};

        let params: Vec<GenericParam> = type_params
            .iter()
            .map(|name| GenericParam::new(name.clone()))
            .collect();
        self.generic_resolver
            .push_context(GenericContext::with_parameters(params));
    }

    /// Exit the current generic context.
    fn exit_generic_context(&mut self) {
        self.generic_resolver.pop_context();
    }

    /// Check if a name is a generic type parameter in the current context.
    #[allow(dead_code)]
    fn is_generic_param(&self, name: &str) -> bool {
        self.generic_resolver.is_generic_param(name)
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

/// Standalone function for type resolution (used to avoid borrow conflicts)
fn resolve_type_annotation(type_ann: &TypeAnnotation) -> Type {
    match &type_ann.kind {
        TypeKind::Simple(name) => parse_type_name(name),
        TypeKind::Array(inner) => Type::Array(Box::new(resolve_type_annotation(inner))),
        TypeKind::Map(k, v) => Type::Map(
            Box::new(resolve_type_annotation(k)),
            Box::new(resolve_type_annotation(v)),
        ),
        TypeKind::Function {
            params,
            return_type,
        } => Type::Function {
            params: params.iter().map(resolve_type_annotation).collect(),
            return_type: Box::new(resolve_type_annotation(return_type)),
        },
        TypeKind::Generic { base, args } => {
            // Handle built-in generic types like مصفوفة<عدد> (Array<Int>)
            match base.as_str() {
                "مصفوفة" | "array" | "Array" => {
                    if let Some(elem_type) = args.first() {
                        Type::Array(Box::new(resolve_type_annotation(elem_type)))
                    } else {
                        Type::Array(Box::new(Type::Unknown))
                    }
                }
                "قاموس" | "map" | "Map" | "dict" | "Dict" => {
                    if args.len() >= 2 {
                        Type::Map(
                            Box::new(resolve_type_annotation(&args[0])),
                            Box::new(resolve_type_annotation(&args[1])),
                        )
                    } else {
                        parse_type_name(base)
                    }
                }
                _ => parse_type_name(base),
            }
        }
        TypeKind::Optional(inner) => Type::Optional(Box::new(resolve_type_annotation(inner))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    /// Helper to wrap source code with required file markers
    fn wrap_with_markers(source: &str) -> String {
        format!("بسم_الله\n{}\nالحمد_لله", source.trim())
    }

    fn analyze(source: &str) -> Result<(), Vec<Diagnostic>> {
        let wrapped = wrap_with_markers(source);
        let mut parser = Parser::new(&wrapped);
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
        let result = analyze(
            r#"
            دالة جمع(أ: عدد، ب: عدد) -> عدد {
                أرجع أ + ب;
            }
        "#,
        );
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

    // ============ OOP Tests ============

    #[test]
    fn test_class_declaration() {
        let result = analyze(
            r#"
            صنف شخص {
                عام الاسم: نص;
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_class_with_methods() {
        let result = analyze(
            r#"
            صنف حساب {
                خاص رصيد: عدد;

                عام دالة أودع(مبلغ: عدد) {
                    متغير س = مبلغ;
                }
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_class_inheritance() {
        let result = analyze(
            r#"
            صنف حيوان {
                عام الاسم: نص;
            }

            صنف قط يرث حيوان {
                عام اللون: نص;
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_interface_declaration() {
        let result = analyze(
            r#"
            واجهة قابل_للطباعة {
                دالة اطبع() -> نص
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_this_outside_class() {
        let result = analyze(
            r#"
            متغير س = هذا.الاسم;
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_super_outside_class() {
        // Test super usage outside class
        let result = analyze(
            r#"
            أساس;
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_super_constructor_call() {
        // Test super constructor call in child class
        let result = analyze(
            r#"
            صنف أب {
                خاص اسم: نص;
                منشئ(اسم: نص) {
                    هذا.اسم = اسم;
                }
            }
            صنف ابن يرث أب {
                خاص عمر: عدد;
                منشئ(اسم: نص، عمر: عدد) {
                    أساس(اسم);
                    هذا.عمر = عمر;
                }
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_super_constructor_call_wrong_args() {
        // Test super constructor call with wrong number of arguments
        let result = analyze(
            r#"
            صنف أب {
                خاص اسم: نص;
                منشئ(اسم: نص) {
                    هذا.اسم = اسم;
                }
            }
            صنف ابن يرث أب {
                منشئ() {
                    أساس();
                }
            }
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_super_constructor_call_no_parent() {
        // Test super constructor call in class without parent
        let result = analyze(
            r#"
            صنف أ {
                منشئ() {
                    أساس();
                }
            }
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_array_length_property() {
        let result = analyze(
            r#"
            متغير أرقام = [1, 2, 3];
            متغير ط = أرقام.طول;
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_string_length_property() {
        let result = analyze(
            r#"
            متغير كلمة = "مرحبا";
            متغير ط = كلمة.طول;
        "#,
        );
        assert!(result.is_ok());
    }

    // ============ Error Handling Tests ============

    #[test]
    fn test_throw_error_object() {
        // Throwing an error object should succeed
        // Note: "خطأ" is reserved for `false`, so we use "استثناء" (exception)
        let result = analyze(
            r#"
            صنف استثناء {
                عام رسالة: نص;
                منشئ(رسالة: نص) {
                    هذا.رسالة = رسالة;
                }
            }
            دالة ف() {
                ارمِ جديد استثناء("حدث خطأ");
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_throw_error_subclass() {
        // Throwing a subclass of استثناء should succeed
        let result = analyze(
            r#"
            صنف استثناء {
                عام رسالة: نص;
            }
            صنف استثناء_قيمة يرث استثناء {
                عام القيمة: عدد;
            }
            دالة ف() {
                ارمِ جديد استثناء_قيمة();
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_throw_string_fails() {
        // Throwing a string should fail
        let result = analyze(
            r#"
            دالة ف() {
                ارمِ "رسالة";
            }
        "#,
        );
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("non-error type")));
    }

    #[test]
    fn test_throw_number_fails() {
        // Throwing a number should fail
        let result = analyze(
            r#"
            دالة ف() {
                ارمِ 42;
            }
        "#,
        );
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("non-error type")));
    }

    #[test]
    fn test_throw_non_error_class_fails() {
        // Throwing a non-error class should fail
        let result = analyze(
            r#"
            صنف شخص {
                عام الاسم: نص;
            }
            دالة ف() {
                ارمِ جديد شخص();
            }
        "#,
        );
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("non-error type")));
    }

    #[test]
    fn test_catch_parameter_typed_as_error() {
        // Catch parameter should be typed as استثناء
        // This test validates that we can access .رسالة on the catch parameter
        let result = analyze(
            r#"
            صنف استثناء {
                عام رسالة: نص;
            }
            دالة ف() {
                حاول {
                    متغير س = 1;
                } التقط (خ) {
                    متغير م = خ.رسالة;
                }
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_catch_finally() {
        // Complete try-catch-finally block should work
        let result = analyze(
            r#"
            صنف استثناء {
                عام رسالة: نص;
                منشئ(رسالة: نص) {
                    هذا.رسالة = رسالة;
                }
            }
            دالة ف() {
                حاول {
                    ارمِ جديد استثناء("حدث استثناء");
                } التقط (خ) {
                    متغير م = خ.رسالة;
                } أخيراً {
                    متغير ن = 1;
                }
            }
        "#,
        );
        assert!(result.is_ok());
    }
}
