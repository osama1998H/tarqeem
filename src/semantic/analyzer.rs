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

/// Information about an enum variant for semantic analysis
#[derive(Debug, Clone)]
pub struct EnumVariantInfo {
    pub name: String,
    pub discriminant: Option<i64>,
    pub fields: Vec<Type>,
}

/// Information about an enum type for semantic analysis
#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<EnumVariantInfo>,
}

pub struct Analyzer {
    scope: Scope,
    class_resolver: ClassResolver,
    generic_resolver: GenericResolver,
    module_loader: ModuleLoader,
    current_file: Option<PathBuf>,
    exports: HashMap<String, Type>,
    diagnostics: Vec<Diagnostic>,
    language: Language,
    current_class: Option<String>,
    expected_type: Option<Type>,
    /// Registry of enum types with their variants
    enums: HashMap<String, EnumInfo>,
}

impl Analyzer {
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
            enums: HashMap::new(),
        }
    }

    pub fn for_file(path: PathBuf) -> Self {
        let mut analyzer = Self::new();
        analyzer.current_file = Some(path);
        analyzer
    }

    pub fn add_search_path(&mut self, path: PathBuf) {
        self.module_loader.add_search_path(path);
    }

    pub fn exports(&self) -> &HashMap<String, Type> {
        &self.exports
    }

    pub fn class_resolver(&self) -> &ClassResolver {
        &self.class_resolver
    }

    pub fn with_language(mut self, lang: Language) -> Self {
        self.language = lang;
        self
    }

    pub fn analyze(&mut self, ast: &Ast) -> Result<(), Vec<Diagnostic>> {
        for stmt in &ast.statements {
            self.register_types(stmt);
        }

        for stmt in &ast.statements {
            self.add_type_members(stmt);
        }

        self.class_resolver.build_vtables();

        if let Err(diags) = self.class_resolver.validate() {
            self.diagnostics.extend(diags);
        }

        for stmt in &ast.statements {
            self.analyze_stmt(stmt);
        }

        if self.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.diagnostics))
        }
    }

    fn register_types(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::ClassDecl {
                name,
                type_params,
                extends,
                implements,
                ..
            } => {
                self.class_resolver.register_class(
                    name,
                    type_params,
                    extends.as_deref(),
                    implements,
                    stmt.span,
                );
            }
            StmtKind::InterfaceDecl { name, .. } => {
                self.class_resolver.register_interface(name, &[], stmt.span);
            }
            _ => {}
        }
    }

    fn add_type_members(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::ClassDecl { name, members, .. } => {
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

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

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

            StmtKind::EnumDecl {
                name,
                variants,
                type_params,
                ..
            } => {
                self.analyze_enum_decl(name, type_params, variants, stmt.span);
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
        let declared_type = ty.map(|t| self.resolve_type(t));

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
        let param_types: Vec<Type> = params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(|t| self.resolve_type(t))
                    .unwrap_or(Type::Any)
            })
            .collect();

        let ret_type = return_type
            .map(|t| self.resolve_type(t))
            .unwrap_or(Type::Void);

        let symbol = Symbol::function(name, param_types.clone(), ret_type.clone());
        if !self.scope.define(symbol) {
            self.error(
                &format!("Function '{}' is already defined", name),
                &format!("الدالة '{}' معرّفة مسبقاً", name),
                span,
            );
        }

        self.push_function_scope(ret_type.clone());

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
        let has_generics = !type_params.is_empty();
        if has_generics {
            self.enter_generic_context(type_params);
        }

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

        for iface in implements {
            if self.class_resolver.get_interface(iface).is_none()
                && self.scope.lookup(iface).is_none()
            {
                self.error(
                    &format!("Unknown interface '{}'", iface),
                    &format!("ميثاق غير معروف '{}'", iface),
                    span,
                );
            }
        }

        let symbol = Symbol::class(name);
        if !self.scope.define(symbol) {
            self.error(
                &format!("Class '{}' is already defined", name),
                &format!("الصنف '{}' معرّف مسبقاً", name),
                span,
            );
        }

        let prev_class = self.current_class.take();
        self.current_class = Some(name.to_string());

        self.push_scope(ScopeKind::Class);

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
                    self.push_function_scope(Type::Void);

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

                ClassMember::Property {
                    name: prop_name,
                    ty,
                    accessors,
                    default_value,
                    ..
                } => {
                    let prop_type = self.resolve_type(ty);

                    if let Some(init_expr) = default_value {
                        let init_type = self.infer_type(init_expr);
                        if !init_type.is_compatible_with(&prop_type) {
                            self.error(
                                &format!(
                                    "Type mismatch in property '{}': expected {}, got {}",
                                    prop_name, prop_type, init_type
                                ),
                                &format!(
                                    "عدم تطابق الأنواع في الخاصية '{}': متوقع {}، وُجد {}",
                                    prop_name,
                                    prop_type.arabic_name(),
                                    init_type.arabic_name()
                                ),
                                init_expr.span,
                            );
                        }
                    }

                    for accessor in accessors {
                        match accessor {
                            crate::parser::PropertyAccessor::Get { body, .. } => {
                                self.push_function_scope(prop_type.clone());
                                match body {
                                    crate::parser::PropertyAccessorBody::Block(block) => {
                                        for stmt in &block.statements {
                                            self.analyze_stmt(stmt);
                                        }
                                    }
                                    crate::parser::PropertyAccessorBody::Expr(expr) => {
                                        let expr_type = self.infer_type(expr);
                                        if !expr_type.is_compatible_with(&prop_type) {
                                            self.error(
                                                &format!(
                                                    "Getter return type mismatch: expected {}, got {}",
                                                    prop_type, expr_type
                                                ),
                                                &format!(
                                                    "عدم تطابق نوع إرجاع القارئ: متوقع {}، وُجد {}",
                                                    prop_type.arabic_name(),
                                                    expr_type.arabic_name()
                                                ),
                                                expr.span,
                                            );
                                        }
                                    }
                                }
                                self.pop_scope();
                            }
                            crate::parser::PropertyAccessor::Set {
                                param_name, body, ..
                            } => {
                                self.push_function_scope(Type::Void);
                                self.scope.define(Symbol::variable(
                                    param_name,
                                    prop_type.clone(),
                                    false,
                                ));
                                for stmt in &body.statements {
                                    self.analyze_stmt(stmt);
                                }
                                self.pop_scope();
                            }
                        }
                    }

                    self.scope
                        .define(Symbol::variable(prop_name, prop_type, true));
                }
            }
        }

        self.pop_scope();

        if has_generics {
            self.exit_generic_context();
        }

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
                &format!("الميثاق '{}' معرّف مسبقاً", name),
                span,
            );
        }
    }

    fn analyze_enum_decl(
        &mut self,
        name: &str,
        type_params: &[String],
        variants: &[EnumVariant],
        span: Span,
    ) {
        // Define the enum as a symbol
        let symbol = Symbol {
            name: name.to_string(),
            kind: SymbolKind::Enum,
            ty: Type::Enum(name.to_string()),
            mutable: false,
            defined: true,
        };

        if !self.scope.define(symbol) {
            self.error(
                &format!("Enum '{}' is already defined", name),
                &format!("التعداد '{}' معرّف مسبقاً", name),
                span,
            );
            return;
        }

        // Store enum info with variants for later lookup
        let variant_infos: Vec<EnumVariantInfo> = variants
            .iter()
            .map(|v| {
                let fields = v
                    .fields
                    .iter()
                    .map(|f| self.resolve_type(&f.ty))
                    .collect();
                EnumVariantInfo {
                    name: v.name.clone(),
                    discriminant: v.discriminant,
                    fields,
                }
            })
            .collect();

        let enum_info = EnumInfo {
            name: name.to_string(),
            type_params: type_params.to_vec(),
            variants: variant_infos,
        };

        self.enums.insert(name.to_string(), enum_info);
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
            // Create a new scope for each arm to handle pattern bindings
            self.push_scope(ScopeKind::Block);

            for pattern in &arm.patterns {
                let pattern_type = self.infer_pattern_type(pattern, &match_type);
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

                // Add pattern bindings to scope
                self.add_pattern_bindings(pattern, &match_type);
            }

            // Analyze the arm body with bindings in scope
            for stmt in &arm.body.statements {
                self.analyze_stmt(stmt);
            }

            self.pop_scope();
        }
    }

    /// Infer the type of a pattern
    fn infer_pattern_type(&mut self, pattern: &Pattern, match_type: &Type) -> Type {
        match &pattern.kind {
            PatternKind::Literal(expr) => self.infer_type(expr),
            PatternKind::Identifier(_) => match_type.clone(),
            PatternKind::Wildcard => match_type.clone(),
            PatternKind::EnumVariant {
                enum_name,
                variant_name,
                ..
            } => {
                // Look up the enum and verify the variant exists
                if let Some(enum_info) = self.enums.get(enum_name) {
                    if enum_info.variants.iter().any(|v| &v.name == variant_name) {
                        Type::Enum(enum_name.clone())
                    } else {
                        self.error(
                            &format!("Variant '{}' not found in enum '{}'", variant_name, enum_name),
                            &format!("الحالة '{}' غير موجودة في التعداد '{}'", variant_name, enum_name),
                            pattern.span,
                        );
                        Type::Unknown
                    }
                } else {
                    self.error(
                        &format!("Enum '{}' not found", enum_name),
                        &format!("التعداد '{}' غير موجود", enum_name),
                        pattern.span,
                    );
                    Type::Unknown
                }
            }
        }
    }

    /// Add pattern bindings to the current scope
    fn add_pattern_bindings(&mut self, pattern: &Pattern, match_type: &Type) {
        match &pattern.kind {
            PatternKind::Identifier(name) => {
                // Bind the identifier to the match type
                self.scope.define(Symbol::variable(name, match_type.clone(), false));
            }
            PatternKind::EnumVariant {
                enum_name,
                variant_name,
                bindings,
            } => {
                // Look up the variant's field types and bind them
                if let Some(enum_info) = self.enums.get(enum_name) {
                    if let Some(variant) = enum_info.variants.iter().find(|v| &v.name == variant_name) {
                        for (i, binding) in bindings.iter().enumerate() {
                            let field_type = if i < variant.fields.len() {
                                variant.fields[i].clone()
                            } else {
                                Type::Unknown
                            };
                            self.scope.define(Symbol::variable(binding, field_type, false));
                        }
                    }
                }
            }
            PatternKind::Literal(_) | PatternKind::Wildcard => {
                // No bindings for literals or wildcards
            }
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

    fn analyze_throw(&mut self, expr: &Expr, span: Span) {
        let expr_type = self.analyze_expr(expr);

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
        let current_file = self
            .current_file
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));

        let module_path = match self.module_loader.resolve_path(&current_file, from) {
            Some(path) => path,
            None => {
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

        let module_exports = match self.module_loader.load_module(&module_path, span) {
            Ok(loaded_module) => loaded_module.exports.clone(),
            Err(()) => {
                let loader_diagnostics = self.module_loader.take_diagnostics();
                self.diagnostics.extend(loader_diagnostics);

                self.register_imports_as_any(items);
                return;
            }
        };

        match items {
            ImportItems::Named(imports) => {
                for import in imports {
                    let name = import.alias.as_ref().unwrap_or(&import.name);
                    if let Some(exported) = module_exports.get(&import.name) {
                        let ty = self.export_kind_to_type(&exported.kind, &import.name);
                        self.scope.define(Symbol::variable(name, ty, false));
                    } else {
                        self.error(
                            &format!("Module '{}' has no export named '{}'", from, import.name),
                            &format!("الوحدة '{}' لا تحتوي على تصدير باسم '{}'", from, import.name),
                            span,
                        );
                        self.scope.define(Symbol::variable(name, Type::Any, false));
                    }
                }
            }
            ImportItems::Wildcard(alias) => {
                self.scope.define(Symbol::variable(alias, Type::Any, false));
            }
            ImportItems::Default(name) => {
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

    fn warn(&mut self, message: &str, message_ar: &str, span: Span) {
        self.diagnostics
            .push(Diagnostic::warning(message, message_ar, span));
    }

    const ERROR_BASE_CLASSES: &'static [&'static str] = &["استثناء", "Exception", "Error"];

    fn is_error_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Class(class_name) => {
                if Self::ERROR_BASE_CLASSES.contains(&class_name.as_str()) {
                    return true;
                }
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

    fn analyze_super_constructor_call(&mut self, args: &[Expr], span: Span) -> Type {
        if !self.scope.is_in_class() {
            self.error(
                "'super()' can only be used inside a class constructor",
                "'الأصل()' يمكن استخدامه فقط داخل منشئ صنف",
                span,
            );
            return Type::Error;
        }

        let current_class_name = match &self.current_class {
            Some(name) => name.clone(),
            None => {
                self.error(
                    "'super()' can only be used inside a class",
                    "'الأصل()' يمكن استخدامه فقط داخل صنف",
                    span,
                );
                return Type::Error;
            }
        };

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
                            "لا يمكن استخدام 'الأصل()' في الصنف '{}' الذي ليس له صنف أب",
                            current_class_name
                        ),
                        span,
                    );
                    return Type::Error;
                }
            },
            None => {
                return Type::Error;
            }
        };

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

        match parent_constructor {
            Some(constructor) => {
                let params = &constructor.params;

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
                                "المعامل {} لـ الأصل() نوعه خاطئ: متوقع {}، وُجد {}",
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

        Type::Void
    }

    fn analyze_block(&mut self, block: &Block, kind: ScopeKind) {
        self.push_scope(kind);

        for stmt in &block.statements {
            self.analyze_stmt(stmt);
        }

        self.pop_scope();
    }

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
                if matches!(callee.kind, ExprKind::Super) {
                    return self.analyze_super_constructor_call(args, expr.span);
                }

                let callee_type = self.infer_type(callee);

                match callee_type {
                    Type::Function {
                        params,
                        return_type,
                    } => {
                        if args.len() != params.len() {
                            self.error(
                                &format!("Expected {} arguments, got {}", params.len(), args.len()),
                                &format!("متوقع {} معاملات، وُجد {}", params.len(), args.len()),
                                expr.span,
                            );
                        }

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

                match &target.kind {
                    ExprKind::Identifier(name) => {
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
                self.infer_type(target);
                self.infer_type(value);
                self.infer_type(target)
            }

            ExprKind::Array(elements) => {
                if elements.is_empty() {
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
                if pairs.is_empty() {
                    if let Some(Type::Map(key_ty, val_ty)) = &self.expected_type {
                        Type::Map(key_ty.clone(), val_ty.clone())
                    } else {
                        Type::Map(Box::new(Type::String), Box::new(Type::Any))
                    }
                } else {
                    let first_value_type = self.infer_type(&pairs[0].1);

                    let mut all_same = true;
                    for (_, value) in pairs.iter().skip(1) {
                        let value_type = self.infer_type(value);
                        if !value_type.is_compatible_with(&first_value_type) {
                            all_same = false;
                            break;
                        }
                    }

                    if all_same {
                        Type::Map(Box::new(Type::String), Box::new(first_value_type))
                    } else {
                        Type::Map(Box::new(Type::String), Box::new(Type::Any))
                    }
                }
            }

            ExprKind::Lambda { params, body } => {
                self.push_function_scope(Type::Any);

                let expected_param_types: Option<Vec<Type>> = match &self.expected_type {
                    Some(Type::Function {
                        params: expected_params,
                        ..
                    }) => Some(expected_params.clone()),
                    _ => None,
                };

                let param_types: Vec<Type> = params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let ty = if let Some(type_ann) = &p.ty {
                            self.resolve_type(type_ann)
                        } else {
                            expected_param_types
                                .as_ref()
                                .and_then(|expected| expected.get(i).cloned())
                                .unwrap_or(Type::Any)
                        };
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

            ExprKind::New {
                class,
                type_args,
                args,
            } => {
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

                let class_info = self.class_resolver.get_class(&class_name).cloned();

                if let Some(class_info) = class_info {
                    if class_info.is_generic() {
                        if type_args.is_empty() {
                            self.error(
                                &format!("Generic class '{}' requires type arguments", class_name),
                                &format!("الصنف المعمم '{}' يتطلب معاملات نوع", class_name),
                                expr.span,
                            );
                        } else if type_args.len() != class_info.type_params.len() {
                            self.error(
                                &format!(
                                    "Wrong number of type arguments: expected {}, got {}",
                                    class_info.type_params.len(),
                                    type_args.len()
                                ),
                                &format!(
                                    "عدد خاطئ لمعاملات النوع: متوقع {}، وُجد {}",
                                    class_info.type_params.len(),
                                    type_args.len()
                                ),
                                expr.span,
                            );
                        } else {
                            let resolved_args: Vec<Type> =
                                type_args.iter().map(|ta| self.resolve_type(ta)).collect();

                            use crate::semantic::generics::GenericParam;
                            let params: Vec<GenericParam> = class_info
                                .type_params
                                .iter()
                                .map(|name| GenericParam::new(name.clone()))
                                .collect();

                            if let Some(context) = self.generic_resolver.instantiate(
                                &params,
                                &resolved_args,
                                expr.span,
                            ) {
                                drop(context);
                            }

                            let diagnostics = self.generic_resolver.take_diagnostics();
                            for diag in diagnostics {
                                self.diagnostics.push(diag);
                            }
                        }
                    } else if !type_args.is_empty() {
                        self.error(
                            &format!(
                                "Class '{}' is not generic but type arguments were provided",
                                class_name
                            ),
                            &format!("الصنف '{}' ليس معمماً لكن تم تقديم معاملات نوع", class_name),
                            expr.span,
                        );
                    }

                    if let Some(ref ctor) = class_info.constructor {
                        let expected_params = &ctor.params;
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

            ExprKind::Await(inner) => self.infer_type(inner),

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
                    self.error("'super' outside of class", "'الأصل' خارج الصنف", expr.span);
                    Type::Error
                } else if let Some(ref class_name) = self.current_class {
                    if let Some(class) = self.class_resolver.get_class(class_name) {
                        if let Some(ref parent_name) = class.parent {
                            Type::Class(parent_name.clone())
                        } else {
                            self.error(
                                "Cannot use 'super' in a class without a parent",
                                "لا يمكن استخدام 'الأصل' في صنف بدون أب",
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

            ExprKind::EnumVariant {
                enum_name,
                variant_name,
                args,
                ..
            } => {
                // Analyze all arguments first
                let arg_types: Vec<Type> = args.iter().map(|a| self.analyze_expr(a)).collect();

                // Look up the enum in our registry
                if let Some(enum_info) = self.enums.get(enum_name).cloned() {
                    // Find the variant
                    if let Some(variant) = enum_info
                        .variants
                        .iter()
                        .find(|v| &v.name == variant_name)
                    {
                        // Check argument count matches
                        if args.len() != variant.fields.len() {
                            self.error(
                                &format!(
                                    "Variant '{}::{}' expects {} argument(s), got {}",
                                    enum_name,
                                    variant_name,
                                    variant.fields.len(),
                                    args.len()
                                ),
                                &format!(
                                    "الحالة '{}::{}' تتوقع {} معامل(ات)، وُجد {}",
                                    enum_name,
                                    variant_name,
                                    variant.fields.len(),
                                    args.len()
                                ),
                                expr.span,
                            );
                        } else {
                            // Check argument types match
                            for (i, (arg_ty, expected_ty)) in
                                arg_types.iter().zip(&variant.fields).enumerate()
                            {
                                if !arg_ty.is_compatible_with(expected_ty) {
                                    self.error(
                                        &format!(
                                            "Type mismatch in variant '{}::{}' argument {}: expected {}, got {}",
                                            enum_name,
                                            variant_name,
                                            i + 1,
                                            expected_ty,
                                            arg_ty
                                        ),
                                        &format!(
                                            "عدم تطابق النوع في معامل {} للحالة '{}::{}': متوقع {}، وُجد {}",
                                            i + 1,
                                            enum_name,
                                            variant_name,
                                            expected_ty.arabic_name(),
                                            arg_ty.arabic_name()
                                        ),
                                        args[i].span,
                                    );
                                }
                            }
                        }
                        Type::Enum(enum_name.clone())
                    } else {
                        // Variant not found
                        self.error(
                            &format!(
                                "Variant '{}' not found in enum '{}'",
                                variant_name, enum_name
                            ),
                            &format!(
                                "الحالة '{}' غير موجودة في التعداد '{}'",
                                variant_name, enum_name
                            ),
                            expr.span,
                        );
                        Type::Error
                    }
                } else {
                    // Enum not found - might be defined later or doesn't exist
                    // For now, just return the type and let it be resolved later
                    Type::Enum(enum_name.clone())
                }
            }
        }
    }

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

    fn resolve_type(&self, type_ann: &TypeAnnotation) -> Type {
        match &type_ann.kind {
            TypeKind::Simple(name) => {
                // Check if it's an enum type first
                if self.enums.contains_key(name) {
                    Type::Enum(name.clone())
                } else {
                    parse_type_name(name)
                }
            }
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
            TypeKind::Generic { base, args } => match base.as_str() {
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
                        parse_type_name(base)
                    }
                }
                _ => parse_type_name(base),
            },
            TypeKind::Optional(inner) => Type::Optional(Box::new(self.resolve_type(inner))),
        }
    }

    fn push_scope(&mut self, kind: ScopeKind) {
        let old_scope = std::mem::replace(&mut self.scope, Scope::new_global());
        self.scope = Scope::new_child(old_scope, kind);
    }

    fn push_function_scope(&mut self, return_type: Type) {
        let old_scope = std::mem::replace(&mut self.scope, Scope::new_global());
        self.scope = Scope::new_function(old_scope, return_type);
    }

    fn pop_scope(&mut self) {
        if let Some(parent) = std::mem::replace(&mut self.scope, Scope::new_global()).pop() {
            self.scope = parent;
        }
    }

    fn enter_generic_context(&mut self, type_params: &[String]) {
        use super::generics::{GenericContext, GenericParam};

        let params: Vec<GenericParam> = type_params
            .iter()
            .map(|name| GenericParam::new(name.clone()))
            .collect();
        self.generic_resolver
            .push_context(GenericContext::with_parameters(params));
    }

    fn exit_generic_context(&mut self) {
        self.generic_resolver.pop_context();
    }

    #[allow(dead_code)]
    fn is_generic_param(&self, name: &str) -> bool {
        self.generic_resolver.is_generic_param(name)
    }

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
        TypeKind::Generic { base, args } => match base.as_str() {
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
        },
        TypeKind::Optional(inner) => Type::Optional(Box::new(resolve_type_annotation(inner))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

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
            ميثاق قابل_للطباعة {
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
        let result = analyze(
            r#"
            الأصل;
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_super_constructor_call() {
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
                    الأصل(اسم);
                    هذا.عمر = عمر;
                }
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_super_constructor_call_wrong_args() {
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
                    الأصل();
                }
            }
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_super_constructor_call_no_parent() {
        let result = analyze(
            r#"
            صنف أ {
                منشئ() {
                    الأصل();
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

    #[test]
    fn test_throw_error_object() {
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
