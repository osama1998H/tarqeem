//! Statement analysis for the Tarqeem semantic analyzer.
//!
//! This module handles semantic analysis of statements including variable
//! declarations, function declarations, class declarations, control flow, etc.

use super::super::modules::ExportKind;
use super::super::prelude;
use super::super::scope::{Scope, ScopeKind, Symbol, SymbolKind};
use super::super::types::Type;
use super::{Analyzer, EnumInfo, EnumVariantInfo};
use crate::error::codes::{
    ERR_BREAK_OUTSIDE_LOOP, ERR_CLASS_NOT_FOUND, ERR_CONTINUE_OUTSIDE_LOOP, ERR_NOT_EXPORTED,
    ERR_RETURN_OUTSIDE_FUNCTION, ERR_THROW_NON_EXCEPTION, ERR_TYPE_MISMATCH, ERR_UNDEFINED_CLASS,
    ERR_VARIABLE_REDEFINITION,
};
use crate::error::Span;
use crate::parser::*;
use std::collections::HashMap;
use std::path::PathBuf;

impl Analyzer {
    /// Analyze a statement.
    pub(crate) fn analyze_stmt(&mut self, stmt: &Stmt) {
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
                    self.error_with_code(
                        "'أوقف' يمكن استخدامها فقط داخل حلقة",
                        stmt.span,
                        &ERR_BREAK_OUTSIDE_LOOP.to_string(),
                    );
                }
            }

            StmtKind::Continue => {
                if !self.scope.is_in_loop() {
                    self.error_with_code(
                        "'استمر' يمكن استخدامها فقط داخل حلقة",
                        stmt.span,
                        &ERR_CONTINUE_OUTSIDE_LOOP.to_string(),
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

            StmtKind::Export(export_items) => {
                self.analyze_export(export_items, stmt.span);
            }

            StmtKind::Expr(expr) => {
                self.analyze_expr(expr);
            }

            StmtKind::Block(block) => {
                self.analyze_block(block, ScopeKind::Block);
            }
        }
    }

    /// Analyze a variable declaration.
    pub(crate) fn analyze_var_decl(
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
            self.error("المتغير يجب أن يحتوي على نوع أو قيمة ابتدائية", span);
            Type::Error
        };

        if let (Some(init_expr), Some(ref expected)) = (init, &declared_type) {
            let init_type = self.with_expected(Some(expected.clone()), |a| a.infer_type(init_expr));

            if !self.is_assignable(&init_type, expected) {
                self.type_mismatch_error(
                    expected,
                    &init_type,
                    init_expr.span,
                    "تهيئة المتغير",
                    &ERR_TYPE_MISMATCH.to_string(),
                );
            }
        }

        let symbol = Symbol::variable(name, var_type, mutable, span);
        if !self.scope.define(symbol) {
            self.already_defined_error(
                "المتغير",
                name,
                span,
                None,
                &ERR_VARIABLE_REDEFINITION.to_string(),
            );
        }
    }

    /// Resolve a function's parameter and return types from its annotations.
    fn func_signature_types(
        &self,
        params: &[Param],
        return_type: Option<&TypeAnnotation>,
    ) -> (Vec<Type>, Type) {
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

        (param_types, ret_type)
    }

    /// Analyze a function declaration.
    pub(crate) fn analyze_func_decl(
        &mut self,
        name: &str,
        params: &[Param],
        return_type: Option<&TypeAnnotation>,
        body: &Block,
        _is_async: bool,
        span: Span,
    ) {
        let (param_types, ret_type) = self.func_signature_types(params, return_type);

        // Top-level functions were already declared by the hoisting pass in
        // `analyze` (issue #186); defining again would falsely report
        // redefinition (د٠١٠١). Nested functions are not hoisted and are
        // declared here as before.
        if self.scope.kind() != ScopeKind::Global {
            let symbol = Symbol::function(name, param_types.clone(), ret_type.clone(), span);
            if !self.scope.define(symbol) {
                self.already_defined_error(
                    "الدالة",
                    name,
                    span,
                    None,
                    &ERR_VARIABLE_REDEFINITION.to_string(),
                );
            }
        }

        self.push_function_scope(ret_type);

        for (param, param_type) in params.iter().zip(param_types.iter()) {
            let symbol = Symbol {
                name: param.name.clone(),
                kind: SymbolKind::Parameter,
                ty: param_type.clone(),
                mutable: false,
                defined: true,
                used: false,
                span: param.span,
            };
            self.scope.define(symbol);
        }

        for stmt in &body.statements {
            self.analyze_stmt(stmt);
        }

        self.pop_scope();
    }

    /// Analyze a class declaration.
    pub(crate) fn analyze_class_decl(
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
                // Try to find similar class names
                let all_classes = self.class_resolver.all_class_names();
                let similar: Vec<String> = all_classes
                    .iter()
                    .filter(|name| {
                        Self::levenshtein_distance(parent_name, name)
                            <= (parent_name.chars().count() / 2 + 2)
                    })
                    .filter(|name| *name != parent_name)
                    .take(3)
                    .cloned()
                    .collect();
                self.undefined_error(
                    "صنف أب",
                    parent_name,
                    span,
                    &similar,
                    &ERR_UNDEFINED_CLASS.to_string(),
                );
            }
        }

        for iface in implements {
            if self.class_resolver.get_interface(iface).is_none()
                && self.scope.lookup(iface).is_none()
            {
                // Try to find similar interface names
                let all_interfaces = self.class_resolver.all_interface_names();
                let similar: Vec<String> = all_interfaces
                    .iter()
                    .filter(|name| {
                        Self::levenshtein_distance(iface, name) <= (iface.chars().count() / 2 + 2)
                    })
                    .filter(|name| *name != iface)
                    .take(3)
                    .cloned()
                    .collect();
                self.undefined_error(
                    "ميثاق",
                    iface,
                    span,
                    &similar,
                    &ERR_UNDEFINED_CLASS.to_string(),
                );
            }
        }

        let symbol = Symbol::class(name, span);
        if !self.scope.define(symbol) {
            self.already_defined_error(
                "الصنف",
                name,
                span,
                None,
                &ERR_VARIABLE_REDEFINITION.to_string(),
            );
        }

        let prev_class = self.current_class.take();
        self.current_class = Some(name.to_string());

        self.push_scope(ScopeKind::Class);

        // هذا/this are synthetic symbols, so use default span
        self.scope.define(Symbol::variable(
            "هذا",
            Type::Class(name.to_string()),
            false,
            Span::default(),
        ));
        self.scope.define(Symbol::variable(
            "this",
            Type::Class(name.to_string()),
            false,
            Span::default(),
        ));

        for member in members {
            self.analyze_class_member(member, name);
        }

        self.pop_scope();

        if has_generics {
            self.exit_generic_context();
        }

        self.current_class = prev_class;
    }

    /// Analyze a single class member.
    fn analyze_class_member(&mut self, member: &ClassMember, _class_name: &str) {
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
                    if !self.is_assignable(&init_type, &field_type) {
                        self.type_mismatch_error(
                            &field_type,
                            &init_type,
                            init_expr.span,
                            &format!("الحقل '{}'", field_name),
                            &ERR_TYPE_MISMATCH.to_string(),
                        );
                    }
                }

                // Class fields use default span (internal to class analysis)
                self.scope.define(Symbol::variable(
                    field_name,
                    field_type,
                    true,
                    Span::default(),
                ));
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
                        .define(Symbol::variable(&param.name, param_type, false, param.span));
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
                self.analyze_property(prop_name, ty, accessors, default_value.as_ref());
            }
        }
    }

    /// Analyze a property with accessors.
    fn analyze_property(
        &mut self,
        prop_name: &str,
        ty: &TypeAnnotation,
        accessors: &[PropertyAccessor],
        default_value: Option<&Expr>,
    ) {
        let prop_type = self.resolve_type(ty);

        if let Some(init_expr) = default_value {
            let init_type = self.infer_type(init_expr);
            if !self.is_assignable(&init_type, &prop_type) {
                self.type_mismatch_error(
                    &prop_type,
                    &init_type,
                    init_expr.span,
                    &format!("الخاصية '{}'", prop_name),
                    &ERR_TYPE_MISMATCH.to_string(),
                );
            }
        }

        for accessor in accessors {
            match accessor {
                PropertyAccessor::Get { body, .. } => {
                    self.push_function_scope(prop_type.clone());
                    match body {
                        PropertyAccessorBody::Block(block) => {
                            for stmt in &block.statements {
                                self.analyze_stmt(stmt);
                            }
                        }
                        PropertyAccessorBody::Expr(expr) => {
                            let expr_type = self.infer_type(expr);
                            if !self.is_assignable(&expr_type, &prop_type) {
                                self.type_mismatch_error(
                                    &prop_type,
                                    &expr_type,
                                    expr.span,
                                    "نوع إرجاع القارئ",
                                    &ERR_TYPE_MISMATCH.to_string(),
                                );
                            }
                        }
                    }
                    self.pop_scope();
                }
                PropertyAccessor::Set {
                    param_name, body, ..
                } => {
                    self.push_function_scope(Type::Void);
                    // Setter param is internal, use default span
                    self.scope.define(Symbol::variable(
                        param_name,
                        prop_type.clone(),
                        false,
                        Span::default(),
                    ));
                    for stmt in &body.statements {
                        self.analyze_stmt(stmt);
                    }
                    self.pop_scope();
                }
            }
        }

        // Property is internal to class, use default span
        self.scope.define(Symbol::variable(
            prop_name,
            prop_type,
            true,
            Span::default(),
        ));
    }

    /// Analyze an interface declaration.
    pub(crate) fn analyze_interface_decl(
        &mut self,
        name: &str,
        _methods: &[MethodSignature],
        span: Span,
    ) {
        let symbol = Symbol {
            name: name.to_string(),
            kind: SymbolKind::Interface,
            ty: Type::Interface(name.to_string()),
            mutable: false,
            defined: true,
            used: false,
            span,
        };

        if !self.scope.define(symbol) {
            self.already_defined_error(
                "الميثاق",
                name,
                span,
                None,
                &ERR_VARIABLE_REDEFINITION.to_string(),
            );
        }
    }

    /// Analyze an enum declaration.
    pub(crate) fn analyze_enum_decl(
        &mut self,
        name: &str,
        type_params: &[String],
        variants: &[EnumVariant],
        span: Span,
    ) {
        // Top-level enums were already registered by the hoisting pass in
        // `analyze` (issue #186); re-declaring would falsely report د٠١٠١.
        if self.scope.kind() == ScopeKind::Global {
            return;
        }
        self.declare_enum(name, type_params, variants, span);
    }

    /// Define an enum's symbol and record its variant info.
    fn declare_enum(
        &mut self,
        name: &str,
        type_params: &[String],
        variants: &[EnumVariant],
        span: Span,
    ) {
        let symbol = Symbol {
            name: name.to_string(),
            kind: SymbolKind::Enum,
            ty: Type::Enum(name.to_string()),
            mutable: false,
            defined: true,
            used: false,
            span,
        };

        if !self.scope.define(symbol) {
            self.already_defined_error(
                "التعداد",
                name,
                span,
                None,
                &ERR_VARIABLE_REDEFINITION.to_string(),
            );
            return;
        }

        let variant_infos: Vec<EnumVariantInfo> = variants
            .iter()
            .map(|v| {
                let fields = v.fields.iter().map(|f| self.resolve_type(&f.ty)).collect();
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

    /// Hoist a top-level enum declaration so forward references resolve.
    ///
    /// Recurses into `صدّر` since exported declarations are analyzed with the
    /// global scope current and would otherwise never be defined.
    pub(crate) fn hoist_enum_decl(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::EnumDecl {
                name,
                type_params,
                variants,
                ..
            } => {
                self.declare_enum(name, type_params, variants, stmt.span);
            }
            StmtKind::Export(ExportItems::Declaration(inner)) => self.hoist_enum_decl(inner),
            _ => {}
        }
    }

    /// Hoist a top-level function declaration so forward references resolve.
    ///
    /// Duplicate detection lives here for top-level functions;
    /// `analyze_func_decl` skips re-defining them.
    pub(crate) fn hoist_func_decl(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::FuncDecl {
                name,
                params,
                return_type,
                ..
            } => {
                let (param_types, ret_type) =
                    self.func_signature_types(params, return_type.as_ref());
                let symbol = Symbol::function(name, param_types, ret_type, stmt.span);
                if !self.scope.define(symbol) {
                    self.already_defined_error(
                        "الدالة",
                        name,
                        stmt.span,
                        None,
                        &ERR_VARIABLE_REDEFINITION.to_string(),
                    );
                }
            }
            StmtKind::Export(ExportItems::Declaration(inner)) => self.hoist_func_decl(inner),
            _ => {}
        }
    }

    /// Analyze an if statement.
    pub(crate) fn analyze_if(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        else_branch: Option<&Block>,
    ) {
        self.check_condition_type(condition);

        self.analyze_block(then_branch, ScopeKind::Block);

        if let Some(else_block) = else_branch {
            self.analyze_block(else_block, ScopeKind::Block);
        }
    }

    /// Analyze a while loop.
    pub(crate) fn analyze_while(&mut self, condition: &Expr, body: &Block) {
        self.check_condition_type(condition);
        self.analyze_block(body, ScopeKind::Loop);
    }

    /// Analyze a do-while loop.
    pub(crate) fn analyze_do_while(&mut self, body: &Block, condition: &Expr) {
        self.analyze_block(body, ScopeKind::Loop);
        self.check_condition_type(condition);
    }

    /// Analyze a for loop.
    pub(crate) fn analyze_for(
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
            self.check_condition_type(cond_expr);
        }

        if let Some(update_expr) = update {
            self.analyze_expr(update_expr);
        }

        for stmt in &body.statements {
            self.analyze_stmt(stmt);
        }

        self.pop_scope();
    }

    /// Analyze a for-in loop.
    pub(crate) fn analyze_for_in(
        &mut self,
        variable: &str,
        iterable: &Expr,
        body: &Block,
        span: Span,
    ) {
        let iter_type = self.infer_type(iterable);

        let elem_type = match iter_type {
            Type::Array(inner) => *inner,
            Type::String => Type::String,
            Type::Map(_, v) => *v,
            _ => {
                self.error(
                    &format!("لا يمكن التكرار على {}", iter_type.arabic_name()),
                    iterable.span,
                );
                Type::Error
            }
        };

        self.push_scope(ScopeKind::Loop);
        self.scope
            .define(Symbol::variable(variable, elem_type, false, span));

        for stmt in &body.statements {
            self.analyze_stmt(stmt);
        }

        self.pop_scope();
    }

    /// Analyze a match statement.
    pub(crate) fn analyze_match(&mut self, expr: &Expr, arms: &[MatchArm]) {
        let match_type = self.infer_type(expr);

        for arm in arms {
            self.push_scope(ScopeKind::Block);

            for pattern in &arm.patterns {
                let pattern_type = self.infer_pattern_type(pattern, &match_type);
                if !pattern_type.is_compatible_with(&match_type) {
                    self.error(
                        &format!(
                            "نوع النمط {} لا يتطابق مع {}",
                            pattern_type.arabic_name(),
                            match_type.arabic_name()
                        ),
                        pattern.span,
                    );
                }

                self.add_pattern_bindings(pattern, &match_type);
            }

            for stmt in &arm.body.statements {
                self.analyze_stmt(stmt);
            }

            self.pop_scope();
        }
    }

    /// Infer the type of a pattern.
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
                if let Some(enum_info) = self.enums.get(enum_name) {
                    if enum_info.variants.iter().any(|v| &v.name == variant_name) {
                        Type::Enum(enum_name.clone())
                    } else {
                        self.error(
                            &format!(
                                "الحالة '{}' غير موجودة في التعداد '{}'",
                                variant_name, enum_name
                            ),
                            pattern.span,
                        );
                        Type::Unknown
                    }
                } else {
                    self.error(&format!("التعداد '{}' غير موجود", enum_name), pattern.span);
                    Type::Unknown
                }
            }
        }
    }

    /// Add pattern bindings to the current scope.
    fn add_pattern_bindings(&mut self, pattern: &Pattern, match_type: &Type) {
        match &pattern.kind {
            PatternKind::Identifier(name) => {
                self.scope.define(Symbol::variable(
                    name,
                    match_type.clone(),
                    false,
                    pattern.span,
                ));
            }
            PatternKind::EnumVariant {
                enum_name,
                variant_name,
                bindings,
            } => {
                if let Some(enum_info) = self.enums.get(enum_name) {
                    if let Some(variant) =
                        enum_info.variants.iter().find(|v| &v.name == variant_name)
                    {
                        for (i, binding) in bindings.iter().enumerate() {
                            let field_type = if i < variant.fields.len() {
                                variant.fields[i].clone()
                            } else {
                                Type::Unknown
                            };
                            // Bindings use pattern span (individual binding spans not available)
                            self.scope.define(Symbol::variable(
                                binding,
                                field_type,
                                false,
                                pattern.span,
                            ));
                        }
                    }
                }
            }
            PatternKind::Literal(_) | PatternKind::Wildcard => {}
        }
    }

    /// Analyze a return statement.
    pub(crate) fn analyze_return(&mut self, value: Option<&Expr>, span: Span) {
        if !self.scope.is_in_function() {
            self.error_with_code(
                "'أرجع' يمكن استخدامها فقط داخل دالة",
                span,
                &ERR_RETURN_OUTSIDE_FUNCTION.to_string(),
            );
            return;
        }

        let return_type = if let Some(expr) = value {
            self.analyze_expr(expr)
        } else {
            Type::Void
        };
        // Feeds block-bodied lambda return-type inference (issue #180); a
        // no-op for declared functions, which never read this back.
        self.scope.record_return(return_type);
    }

    /// Analyze a try-catch-finally statement.
    pub(crate) fn analyze_try(
        &mut self,
        body: &Block,
        catch: Option<&CatchClause>,
        finally: Option<&Block>,
    ) {
        self.analyze_block(body, ScopeKind::Block);

        if let Some(catch_clause) = catch {
            self.push_scope(ScopeKind::Block);
            // Catch param uses body span as its declaration location
            self.scope.define(Symbol::variable(
                &catch_clause.param,
                Type::Class("استثناء".to_string()),
                false,
                catch_clause.body.span,
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

    /// Analyze a throw statement.
    ///
    /// The message names `استثناء`, not `خطأ`: `خطأ` is the boolean-false
    /// literal, and calling it the base class is what sent readers looking for
    /// a class they could never declare (issue #181).
    pub(crate) fn analyze_throw(&mut self, expr: &Expr, span: Span) {
        let expr_type = self.analyze_expr(expr);

        // `Type::Error` means the operand was already diagnosed. Reporting it
        // again renders as `'خطأ'` — `Type::Error::arabic_name()` — reading as
        // if the user had thrown a boolean.
        if expr_type == Type::Error {
            return;
        }

        if !self.is_error_type(&expr_type) {
            self.error_with_code(
                &format!(
                    "لا يمكن رمي '{}': «ارمِ» تقبل نسخة من «{}» أو أحد أصنافه الفرعية فقط. / \
                     Cannot throw '{}': «ارمِ» accepts only an instance of «{}» \
                     or one of its subclasses.",
                    expr_type.arabic_name(),
                    prelude::EXCEPTION_CLASS,
                    expr_type.arabic_name(),
                    prelude::EXCEPTION_CLASS
                ),
                span,
                &ERR_THROW_NON_EXCEPTION.to_string(),
            );
        }
    }

    /// Analyze an import statement.
    pub(crate) fn analyze_import(&mut self, items: &ImportItems, from: &str, span: Span) {
        // First, check if this is a stdlib module import
        // Stdlib modules have builtin functions that don't require file loading
        let is_stdlib_module = Scope::get_stdlib_modules().contains(&from);

        if is_stdlib_module {
            // Handle stdlib module imports directly through the builtin map
            self.handle_stdlib_import(items, from, span);
            return;
        }

        let current_file = self
            .current_file
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));

        let module_path = match self.module_loader.resolve_path(&current_file, from) {
            Some(path) => path,
            None => {
                self.warn(
                    &format!(
                        "الوحدة '{}' غير موجودة، سيتم تصنيف الاستيرادات كـ 'أي'",
                        from
                    ),
                    span,
                );
                self.register_imports_as_any(items, span);
                return;
            }
        };

        let module_exports = match self.module_loader.load_module(&module_path, span) {
            Ok(loaded_module) => loaded_module.exports.clone(),
            Err(()) => {
                let loader_diagnostics = self.module_loader.take_diagnostics();
                self.diagnostics.extend(loader_diagnostics);

                self.register_imports_as_any(items, span);
                return;
            }
        };

        match items {
            ImportItems::Named(imports) => {
                for import in imports {
                    let name = import.alias.as_ref().unwrap_or(&import.name);
                    if let Some(exported) = module_exports.get(&import.name) {
                        let ty = self.export_kind_to_type(&exported.kind, &import.name);
                        self.scope.define(Symbol {
                            name: name.clone(),
                            kind: SymbolKind::Import,
                            ty,
                            mutable: false,
                            defined: true,
                            used: false,
                            span,
                        });
                    } else {
                        self.error_with_code(
                            &format!("الوحدة '{}' لا تحتوي على تصدير باسم '{}'", from, import.name),
                            span,
                            &ERR_NOT_EXPORTED.to_string(),
                        );
                        self.scope.define(Symbol {
                            name: name.clone(),
                            kind: SymbolKind::Import,
                            ty: Type::Any,
                            mutable: false,
                            defined: true,
                            used: false,
                            span,
                        });
                    }
                }
            }
            ImportItems::Wildcard(alias) => {
                let members: HashMap<String, Type> = module_exports
                    .iter()
                    .map(|(name, exported)| {
                        (name.clone(), self.export_kind_to_type(&exported.kind, name))
                    })
                    .collect();
                self.module_namespaces.insert(from.to_string(), members);

                self.scope.define(Symbol {
                    name: alias.clone(),
                    kind: SymbolKind::Import,
                    // Typing the alias `أي` made `أدوات.غير_موجود` type-check
                    // silently; `Type::Module` routes member access through the
                    // exports recorded above instead.
                    ty: Type::Module(from.to_string()),
                    mutable: false,
                    defined: true,
                    used: false,
                    span,
                });
            }
            ImportItems::Default(name) => {
                if let Some(exported) = module_exports.get("default") {
                    let ty = self.export_kind_to_type(&exported.kind, "default");
                    self.scope.define(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Import,
                        ty,
                        mutable: false,
                        defined: true,
                        used: false,
                        span,
                    });
                } else {
                    self.warn(
                        &format!("الوحدة '{}' لا تحتوي على تصدير افتراضي", from),
                        span,
                    );
                    self.scope.define(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Import,
                        ty: Type::Any,
                        mutable: false,
                        defined: true,
                        used: false,
                        span,
                    });
                }
            }
        }
    }

    /// Register imports as Any type when module is not found.
    ///
    /// The wildcard arm deliberately stays `أي` instead of `Type::Module`: with
    /// no exports to check against, every member access through the alias would
    /// become an error. Most of `stdlib/` does not parse yet, so tightening
    /// this would turn silent degradation into a wall of diagnostics.
    fn register_imports_as_any(&mut self, items: &ImportItems, span: Span) {
        match items {
            ImportItems::Named(imports) => {
                for import in imports {
                    let name = import.alias.as_ref().unwrap_or(&import.name);
                    self.scope.define(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Import,
                        ty: Type::Any,
                        mutable: false,
                        defined: true,
                        used: false,
                        span,
                    });
                }
            }
            ImportItems::Wildcard(alias) => {
                self.scope.define(Symbol {
                    name: alias.clone(),
                    kind: SymbolKind::Import,
                    ty: Type::Any,
                    mutable: false,
                    defined: true,
                    used: false,
                    span,
                });
            }
            ImportItems::Default(name) => {
                self.scope.define(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Import,
                    ty: Type::Any,
                    mutable: false,
                    defined: true,
                    used: false,
                    span,
                });
            }
        }
    }

    /// Analyze an export statement.
    pub(crate) fn analyze_export(&mut self, export_items: &ExportItems, span: Span) {
        match export_items {
            ExportItems::Declaration(inner) => {
                // صدّر دالة/صنف/ثابت... - analyze the inner declaration
                self.analyze_stmt(inner);
            }
            ExportItems::Named(items) => {
                // صدّر { name1، name2 } - verify names are defined
                for item in items {
                    if self.scope.lookup(&item.name).is_none() {
                        self.warn(&format!("الاسم '{}' غير معرّف للتصدير", item.name), span);
                    }
                }
            }
            ExportItems::Wildcard { from: _ } => {
                // صدّر * من "module" - re-export all from module
                // Full validation requires loading the source module
            }
            ExportItems::NamedReexport { items, from: _ } => {
                // صدّر { name1، name2 } من "module"
                // Full validation requires loading the source module
                // For now, just register that these are being re-exported
                let _ = items; // Suppress unused warning
            }
        }
    }

    /// Handle imports from stdlib modules.
    /// Stdlib modules have builtin functions that are resolved through Scope::get_stdlib_builtin.
    fn handle_stdlib_import(&mut self, items: &ImportItems, module: &str, span: Span) {
        match items {
            ImportItems::Named(imports) => {
                for import in imports {
                    let name = import.alias.as_ref().unwrap_or(&import.name);
                    // Check if this is a stdlib builtin function
                    if let Some(symbol) = Scope::get_stdlib_builtin(module, &import.name) {
                        self.scope.define(Symbol {
                            name: name.clone(),
                            kind: SymbolKind::Import,
                            ty: symbol.ty,
                            mutable: false,
                            defined: true,
                            used: false,
                            span,
                        });
                    } else {
                        self.error_with_code(
                            &format!(
                                "الوحدة '{}' لا تحتوي على تصدير باسم '{}'",
                                module, import.name
                            ),
                            span,
                            &ERR_NOT_EXPORTED.to_string(),
                        );
                        self.scope.define(Symbol {
                            name: name.clone(),
                            kind: SymbolKind::Import,
                            ty: Type::Any,
                            mutable: false,
                            defined: true,
                            used: false,
                            span,
                        });
                    }
                }
            }
            ImportItems::Wildcard(alias) => {
                self.scope.define(Symbol {
                    name: alias.clone(),
                    kind: SymbolKind::Import,
                    // No `module_namespaces` entry: a stdlib module has no AST
                    // to enumerate, so `resolve_member_type` asks
                    // `Scope::get_stdlib_builtin` for each member by name.
                    ty: Type::Module(module.to_string()),
                    mutable: false,
                    defined: true,
                    used: false,
                    span,
                });
            }
            ImportItems::Default(name) => {
                // Stdlib modules don't have default exports
                self.warn(
                    &format!("الوحدة '{}' لا تحتوي على تصدير افتراضي", module),
                    span,
                );
                self.scope.define(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Import,
                    ty: Type::Any,
                    mutable: false,
                    defined: true,
                    used: false,
                    span,
                });
            }
        }
    }

    /// Convert export kind to type.
    fn export_kind_to_type(&self, kind: &ExportKind, name: &str) -> Type {
        match kind {
            ExportKind::Function {
                params,
                return_type,
            } => Type::Function {
                params: params.clone(),
                return_type: Box::new(return_type.clone()),
            },
            ExportKind::Class => Type::Class(name.to_string()),
            ExportKind::Interface => Type::Interface(name.to_string()),
            ExportKind::Variable(ty) | ExportKind::Constant(ty) => ty.clone(),
        }
    }

    /// Analyze a super constructor call.
    pub(crate) fn analyze_super_constructor_call(&mut self, args: &[Expr], span: Span) -> Type {
        if !self.scope.is_in_class() {
            self.error("'الأصل()' يمكن استخدامه فقط داخل منشئ صنف", span);
            return Type::Error;
        }

        let current_class_name = match &self.current_class {
            Some(name) => name.clone(),
            None => {
                self.error("'الأصل()' يمكن استخدامه فقط داخل صنف", span);
                return Type::Error;
            }
        };

        let parent_name = match self.class_resolver.get_class(&current_class_name) {
            Some(class_info) => match &class_info.parent {
                Some(parent) => parent.clone(),
                None => {
                    self.error(
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
                self.error_with_code(
                    &format!("الصنف الأب '{}' غير موجود", parent_name),
                    span,
                    &ERR_CLASS_NOT_FOUND.to_string(),
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
                    if !self.is_assignable(&arg_type, param_type) {
                        self.type_mismatch_error(
                            param_type,
                            &arg_type,
                            arg.span,
                            &format!("معامل الأصل() {}", i + 1),
                            &ERR_TYPE_MISMATCH.to_string(),
                        );
                    }
                }
            }
            None => {
                if !args.is_empty() {
                    self.error(
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

    /// Analyze a block of statements.
    pub(crate) fn analyze_block(&mut self, block: &Block, kind: ScopeKind) {
        self.push_scope(kind);

        for stmt in &block.statements {
            self.analyze_stmt(stmt);
        }

        self.pop_scope();
    }
}
