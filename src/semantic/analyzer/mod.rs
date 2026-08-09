//! Semantic analyzer for Tarqeem
//!
//! This module provides semantic analysis for Tarqeem programs, including
//! type checking, name resolution, and scope management.
//!
//! # Module Organization
//!
//! The analyzer is split into submodules for maintainability:
//! - `expr_analyzer`: Expression type inference
//! - `stmt_analyzer`: Statement analysis

mod expr_analyzer;
mod stmt_analyzer;

use super::class_resolver::ClassResolver;
use super::generics::{GenericContext, GenericParam, GenericResolver};
use super::modules::ModuleLoader;
use super::scope::{Scope, ScopeKind, SymbolKind};
use super::types::{parse_type_name, Type};
use crate::error::codes::{WARN_UNUSED_FUNCTION, WARN_UNUSED_IMPORT, WARN_UNUSED_VARIABLE};
use crate::error::{Diagnostic, Language, Span};
use crate::parser::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Information about an enum variant for semantic analysis.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EnumVariantInfo {
    pub name: String,
    pub discriminant: Option<i64>,
    pub fields: Vec<Type>,
}

/// Information about an enum type for semantic analysis.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EnumInfo {
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<EnumVariantInfo>,
}

/// Semantic analyzer for Tarqeem programs.
pub struct Analyzer {
    pub(crate) scope: Scope,
    pub(crate) class_resolver: ClassResolver,
    pub(crate) generic_resolver: GenericResolver,
    pub(crate) module_loader: ModuleLoader,
    pub(crate) current_file: Option<PathBuf>,
    pub(crate) exports: HashMap<String, Type>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) language: Language,
    pub(crate) current_class: Option<String>,
    pub(crate) expected_type: Option<Type>,
    /// Registry of enum types with their variants.
    pub(crate) enums: HashMap<String, EnumInfo>,
}

impl Analyzer {
    /// Create a new analyzer.
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

    /// Create an analyzer for a specific file.
    pub fn for_file(path: PathBuf) -> Self {
        let mut analyzer = Self::new();
        analyzer.current_file = Some(path);
        analyzer
    }

    /// Add a search path for module resolution.
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.module_loader.add_search_path(path);
    }

    /// Get the exports from this module.
    pub fn exports(&self) -> &HashMap<String, Type> {
        &self.exports
    }

    /// Get the class resolver.
    pub fn class_resolver(&self) -> &ClassResolver {
        &self.class_resolver
    }

    /// Set the language for error messages.
    pub fn with_language(mut self, lang: Language) -> Self {
        self.language = lang;
        self
    }

    /// Analyze an AST and return any errors.
    pub fn analyze(&mut self, ast: &Ast) -> Result<(), Vec<Diagnostic>> {
        // First pass: register all types (classes and interfaces)
        for stmt in &ast.statements {
            self.register_types(stmt);
        }

        // Hoist top-level enums, then functions, so forward references
        // resolve (issue #186). Enums must come first: `resolve_type` only
        // produces Type::Enum for names already present in `self.enums`, so a
        // function signature mentioning a later enum would otherwise get an
        // incompatible class type.
        for stmt in &ast.statements {
            self.hoist_enum_decl(stmt);
        }
        for stmt in &ast.statements {
            self.hoist_func_decl(stmt);
        }

        // Second pass: add members to types
        for stmt in &ast.statements {
            self.add_type_members(stmt);
        }

        // Build vtables for method dispatch
        self.class_resolver.build_vtables();

        // Validate class hierarchy
        if let Err(diags) = self.class_resolver.validate() {
            self.diagnostics.extend(diags);
        }

        // Third pass: analyze statements
        for stmt in &ast.statements {
            self.analyze_stmt(stmt);
        }

        // Only return Err if there are actual errors (not just warnings)
        let has_errors = self
            .diagnostics
            .iter()
            .any(|d| d.level == crate::error::DiagnosticLevel::Error);

        if has_errors {
            Err(self.diagnostics.clone())
        } else {
            Ok(())
        }
    }

    /// Register types in the first pass.
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

    /// Add members to registered types in the second pass.
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

    /// Get the diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    // Type resolution

    /// Resolve a type annotation to a Type.
    pub(crate) fn resolve_type(&self, type_ann: &TypeAnnotation) -> Type {
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

    // Scope management

    /// Push a new scope.
    pub(crate) fn push_scope(&mut self, kind: ScopeKind) {
        let old_scope = std::mem::replace(&mut self.scope, Scope::new_global());
        self.scope = Scope::new_child(old_scope, kind);
    }

    /// Push a function scope with return type.
    pub(crate) fn push_function_scope(&mut self, return_type: Type) {
        let old_scope = std::mem::replace(&mut self.scope, Scope::new_global());
        self.scope = Scope::new_function(old_scope, return_type);
    }

    /// Pop the current scope.
    pub(crate) fn pop_scope(&mut self) {
        // Check for unused symbols before popping
        self.check_unused_symbols();

        if let Some(parent) = std::mem::replace(&mut self.scope, Scope::new_global()).pop() {
            self.scope = parent;
        }
    }

    /// Check for unused symbols in the current scope and emit warnings.
    fn check_unused_symbols(&mut self) {
        // Collect unused symbols first to avoid borrow issues
        let unused_symbols: Vec<(String, SymbolKind, Span)> = self
            .scope
            .symbols()
            .filter(|symbol| {
                // Skip if already used
                if symbol.used {
                    return false;
                }

                // Skip symbols without real source locations (built-ins use Span::default())
                if symbol.span == Span::default() {
                    return false;
                }

                // Skip symbols that start with underscore (intentionally unused)
                if symbol.name.starts_with('_') {
                    return false;
                }

                // Skip special names
                let name = &symbol.name;
                if name == "هذا"
                    || name == "this"
                    || name == "الأصل"
                    || name == "super"
                    || name == "رئيسية"
                    || name == "__main__"
                {
                    return false;
                }

                true
            })
            .map(|s| (s.name.clone(), s.kind.clone(), s.span))
            .collect();

        // Emit warnings for unused symbols
        for (name, kind, span) in unused_symbols {
            match kind {
                SymbolKind::Variable | SymbolKind::Parameter => {
                    self.warn_with_code(
                        &format!("المتغير '{}' مُعرَّف لكن غير مستخدم", name),
                        span,
                        &WARN_UNUSED_VARIABLE.to_string(),
                    );
                }
                SymbolKind::Function => {
                    self.warn_with_code(
                        &format!("الدالة '{}' مُعرَّفة لكن غير مستدعاة", name),
                        span,
                        &WARN_UNUSED_FUNCTION.to_string(),
                    );
                }
                SymbolKind::Import => {
                    self.warn_with_code(
                        &format!("الاستيراد '{}' غير مستخدم", name),
                        span,
                        &WARN_UNUSED_IMPORT.to_string(),
                    );
                }
                SymbolKind::Class | SymbolKind::Interface | SymbolKind::Enum => {
                    // Classes, interfaces, and enums are often defined for external use
                    // Skip warnings for these
                }
            }
        }
    }

    // Generic context management

    /// Enter a generic context with type parameters.
    pub(crate) fn enter_generic_context(&mut self, type_params: &[String]) {
        let params: Vec<GenericParam> = type_params
            .iter()
            .map(|name| GenericParam::new(name.clone()))
            .collect();
        self.generic_resolver
            .push_context(GenericContext::with_parameters(params));
    }

    /// Exit the current generic context.
    pub(crate) fn exit_generic_context(&mut self) {
        self.generic_resolver.pop_context();
    }

    /// Check if a name is a generic type parameter.
    #[allow(dead_code)]
    pub(crate) fn is_generic_param(&self, name: &str) -> bool {
        self.generic_resolver.is_generic_param(name)
    }

    // Error reporting

    /// Report an error (Arabic-only).
    pub(crate) fn error(&mut self, message: &str, span: Span) {
        self.diagnostics.push(Diagnostic::error(message, span));
    }

    /// Report a warning (Arabic-only).
    #[allow(dead_code)]
    pub(crate) fn warning(&mut self, message: &str, span: Span) {
        self.diagnostics.push(Diagnostic::warning(message, span));
    }

    /// Report a warning (alias, Arabic-only).
    pub(crate) fn warn(&mut self, message: &str, span: Span) {
        self.diagnostics.push(Diagnostic::warning(message, span));
    }

    /// Report an error with an error code (Arabic-only).
    pub(crate) fn error_with_code(&mut self, message: &str, span: Span, code: &str) {
        self.diagnostics
            .push(Diagnostic::error(message, span).with_code(code));
    }

    /// Report a warning with an error code (Arabic-only).
    pub(crate) fn warn_with_code(&mut self, message: &str, span: Span, code: &str) {
        self.diagnostics
            .push(Diagnostic::warning(message, span).with_code(code));
    }

    /// Report a type mismatch error with a conversion suggestion (Arabic-only).
    pub(crate) fn type_mismatch_error(
        &mut self,
        expected: &Type,
        found: &Type,
        span: Span,
        context_ar: &str,
        code: &str,
    ) {
        use crate::error::{Note, Suggestion};

        let message = format!(
            "عدم تطابق الأنواع{}: متوقع {}، وُجد {}",
            if context_ar.is_empty() {
                String::new()
            } else {
                format!(" في {}", context_ar)
            },
            expected.arabic_name(),
            found.arabic_name()
        );

        let mut diag = Diagnostic::error(&message, span).with_code(code);

        // Add conversion suggestion based on types
        if let Some((suggestion, replacement)) = Self::get_conversion_suggestion(expected, found) {
            diag = diag.with_suggestion(Suggestion::new(suggestion, replacement, span));
        }

        // Add note about type compatibility
        diag = diag.with_note(Note::new(format!(
            "النوع المتوقع '{}' لكن وُجد '{}'",
            expected.arabic_name(),
            found.arabic_name()
        )));

        self.diagnostics.push(diag);
    }

    /// Get a conversion suggestion for common type mismatches (Arabic-only).
    fn get_conversion_suggestion(expected: &Type, found: &Type) -> Option<(String, String)> {
        match (expected, found) {
            (Type::String, Type::Int) => Some((
                "حوّل العدد إلى نص باستخدام نص()".to_string(),
                "نص(<value>)".to_string(),
            )),
            (Type::String, Type::Float) => Some((
                "حوّل العدد العشري إلى نص باستخدام نص()".to_string(),
                "نص(<value>)".to_string(),
            )),
            (Type::String, Type::Bool) => Some((
                "حوّل القيمة المنطقية إلى نص باستخدام نص()".to_string(),
                "نص(<value>)".to_string(),
            )),
            (Type::Int, Type::Float) => Some((
                "استخدم عدد() لتحويل العدد العشري إلى صحيح (يحذف الكسر)".to_string(),
                "عدد(<value>)".to_string(),
            )),
            (Type::Float, Type::Int) => Some((
                "سيُرقّى العدد الصحيح تلقائياً إلى عشري".to_string(),
                "<value>.0".to_string(),
            )),
            (Type::Bool, Type::Int) => Some((
                "استخدم مقارنة للحصول على قيمة منطقية: <value> != 0".to_string(),
                "<value> != 0".to_string(),
            )),
            _ => None,
        }
    }

    /// Report an undefined identifier error with similar name suggestions (Arabic-only).
    pub(crate) fn undefined_error(
        &mut self,
        kind_ar: &str,
        name: &str,
        span: Span,
        similar_names: &[String],
        code: &str,
    ) {
        use crate::error::Suggestion;

        let message = format!("{} غير معروف '{}'", kind_ar, name);

        let mut diag = Diagnostic::error(&message, span).with_code(code);

        // Add suggestions for similar names
        if !similar_names.is_empty() {
            let closest = &similar_names[0];
            diag = diag.with_suggestion(Suggestion::new(
                format!("هل تقصد '{}'؟", closest),
                closest.clone(),
                span,
            ));
        }

        self.diagnostics.push(diag);
    }

    /// Report an "already defined" error with a note pointing to the original definition (Arabic-only).
    pub(crate) fn already_defined_error(
        &mut self,
        kind_ar: &str,
        name: &str,
        span: Span,
        original_span: Option<Span>,
        code: &str,
    ) {
        use crate::error::Note;

        let message = format!("{} '{}' معرّف مسبقاً", kind_ar, name);

        let mut diag = Diagnostic::error(&message, span).with_code(code);

        // Add note pointing to original definition
        if let Some(orig_span) = original_span {
            diag = diag
                .with_note(Note::new(format!("'{}' تم تعريفه هنا أولاً", name)).with_span(orig_span));
        }

        self.diagnostics.push(diag);
    }

    /// Check that a condition expression has boolean type.
    /// Reports an error if the condition is not compatible with Bool.
    pub(crate) fn check_condition_type(&mut self, condition: &Expr) {
        let cond_type = self.infer_type(condition);
        if !cond_type.is_compatible_with(&Type::Bool) {
            self.error(
                &format!("الشرط يجب أن يكون منطقياً، وُجد {}", cond_type.arabic_name()),
                condition.span,
            );
        }
    }

    /// Find similar names to the given name from a list of candidates.
    pub(crate) fn find_similar_names(&self, name: &str, max_results: usize) -> Vec<String> {
        let mut candidates: Vec<(String, usize)> = Vec::new();

        // Collect all defined symbols in scope
        for symbol in self.scope.all_symbols() {
            let distance = Self::levenshtein_distance(name, &symbol.name);
            // Only include if distance is reasonable (less than half the name length + 2)
            if distance <= (name.chars().count() / 2 + 2) {
                candidates.push((symbol.name.clone(), distance));
            }
        }

        // Sort by distance and take top results
        candidates.sort_by_key(|(_, d)| *d);
        candidates
            .into_iter()
            .take(max_results)
            .filter(|(_, d)| *d > 0) // Exclude exact matches
            .map(|(n, _)| n)
            .collect()
    }

    /// Calculate Levenshtein distance between two strings.
    fn levenshtein_distance(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let a_len = a_chars.len();
        let b_len = b_chars.len();

        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }

        let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

        for (i, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
            row[0] = i;
        }
        for (j, cell) in matrix[0].iter_mut().enumerate().take(b_len + 1) {
            *cell = j;
        }

        for i in 1..=a_len {
            for j in 1..=b_len {
                let cost = if a_chars[i - 1] == b_chars[j - 1] {
                    0
                } else {
                    1
                };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[a_len][b_len]
    }

    // Error type checking

    const ERROR_BASE_CLASSES: &'static [&'static str] = &["استثناء", "Exception", "Error"];

    /// Check if a type is an error type.
    pub(crate) fn is_error_type(&self, ty: &Type) -> bool {
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
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve a type annotation (standalone function).
pub fn resolve_type_annotation(type_ann: &TypeAnnotation) -> Type {
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
        assert!(errors
            .iter()
            .any(|e| e.message.contains("لا يمكن رمي نوع غير خطأ")));
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
        assert!(errors.iter().any(|e| e.message.contains("غير خطأ")));
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
        assert!(errors.iter().any(|e| e.message.contains("غير خطأ")));
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

    // ==========================================================================
    // Visibility Tests (ص٠٤٠١ - ص٠٤٠٢)
    // ==========================================================================

    #[test]
    fn test_public_field_access_allowed() {
        let result = analyze(
            r#"
            صنف شخص {
                عام الاسم: نص;
            }
            متغير ش = جديد شخص();
            متغير ا = ش.الاسم;
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_private_field_access_from_outside_fails() {
        let result = analyze(
            r#"
            صنف حساب {
                خاص الرصيد: عدد;
            }
            متغير ح = جديد حساب();
            متغير ر = ح.الرصيد;
        "#,
        );
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.code.as_ref().is_some_and(|c| c == "ص٠٤٠١")));
    }

    #[test]
    fn test_private_field_access_from_same_class_allowed() {
        let result = analyze(
            r#"
            صنف حساب {
                خاص الرصيد: عدد;

                عام دالة احصل_رصيد() -> عدد {
                    أرجع هذا.الرصيد;
                }
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_private_field_access_from_subclass_fails() {
        let result = analyze(
            r#"
            صنف أب {
                خاص سر: نص;
            }
            صنف ابن يرث أب {
                عام دالة اكشف() -> نص {
                    أرجع هذا.سر;
                }
            }
        "#,
        );
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.code.as_ref().is_some_and(|c| c == "ص٠٤٠١")));
    }

    #[test]
    fn test_protected_field_access_from_outside_fails() {
        let result = analyze(
            r#"
            صنف كائن_حي {
                محمي العمر: عدد;
            }
            متغير كائن = جديد كائن_حي();
            متغير عمر = كائن.العمر;
        "#,
        );
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.code.as_ref().is_some_and(|c| c == "ص٠٤٠٢")));
    }

    #[test]
    fn test_protected_field_access_from_same_class_allowed() {
        let result = analyze(
            r#"
            صنف كائن_حي {
                محمي العمر: عدد;

                عام دالة احصل_عمر() -> عدد {
                    أرجع هذا.العمر;
                }
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_protected_field_access_from_subclass_allowed() {
        let result = analyze(
            r#"
            صنف كائن_حي {
                محمي العمر: عدد;
            }
            صنف حيوان يرث كائن_حي {
                عام دالة تقدم_بالعمر() {
                    هذا.العمر = هذا.العمر + 1;
                }
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_protected_field_access_from_grandchild_allowed() {
        let result = analyze(
            r#"
            صنف جد {
                محمي القيمة: عدد;
            }
            صنف أب يرث جد {
            }
            صنف ابن يرث أب {
                عام دالة احصل_قيمة() -> عدد {
                    أرجع هذا.القيمة;
                }
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_private_method_access_from_outside_fails() {
        let result = analyze(
            r#"
            صنف معالج {
                خاص دالة تحقق_داخلي() -> منطقي {
                    أرجع صحيح;
                }
            }
            متغير م = جديد معالج();
            م.تحقق_داخلي();
        "#,
        );
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.code.as_ref().is_some_and(|c| c == "ص٠٤٠١")));
    }

    #[test]
    fn test_protected_method_access_from_subclass_allowed() {
        let result = analyze(
            r#"
            صنف آلة {
                محمي دالة صيانة() {
                    متغير س = 1;
                }
            }
            صنف سيارة يرث آلة {
                عام دالة فحص() {
                    هذا.صيانة();
                }
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_protected_method_access_from_outside_fails() {
        let result = analyze(
            r#"
            صنف آلة {
                محمي دالة صيانة() {
                    متغير س = 1;
                }
            }
            متغير آ = جديد آلة();
            آ.صيانة();
        "#,
        );
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.code.as_ref().is_some_and(|c| c == "ص٠٤٠٢")));
    }

    // Issue #186: top-level functions and enums are hoisted, so forward
    // references (including mutual recursion) must analyze cleanly.

    #[test]
    fn test_forward_function_call() {
        let result = analyze(
            r#"
            دالة أ() -> عدد {
                أرجع ب();
            }
            دالة ب() -> عدد {
                أرجع 1;
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_mutual_recursion_functions() {
        let result = analyze(
            r#"
            دالة زوجي(ن: عدد) -> منطقي {
                إذا (ن == 0) {
                    أرجع صحيح;
                }
                أرجع فردي(ن - 1);
            }
            دالة فردي(ن: عدد) -> منطقي {
                إذا (ن == 0) {
                    أرجع خطأ;
                }
                أرجع زوجي(ن - 1);
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_top_level_call_before_definition() {
        let result = analyze(
            r#"
            اطبع(ضعف(21));
            دالة ضعف(س: عدد) -> عدد {
                أرجع س * 2;
            }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_duplicate_function_still_errors_once() {
        let result = analyze(
            r#"
            دالة مكررة() {
                متغير س = 1;
            }
            دالة مكررة() {
                متغير س = 1;
            }
        "#,
        );
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(
            errors
                .iter()
                .filter(|e| e.code.as_ref().is_some_and(|c| c == "د٠١٠١"))
                .count(),
            1
        );
    }

    #[test]
    fn test_forward_enum_member_access() {
        let result = analyze(
            r#"
            متغير ل = لون.أحمر;
            تعداد لون { أحمر، أخضر، أزرق }
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_forward_enum_variant_with_annotation() {
        let result = analyze(
            r#"
            متغير ل: لون = لون::أحمر;
            تعداد لون { أحمر، أخضر }
        "#,
        );
        assert!(result.is_ok());
    }

    // Pins the hoist ordering: enums must be registered before function
    // signatures are resolved, or `ل: لون` would become an incompatible
    // class type.
    #[test]
    fn test_function_with_enum_param_before_enum_decl() {
        let result = analyze(
            r#"
            دالة صف(ل: لون) -> نص {
                أرجع "لون";
            }
            تعداد لون { أحمر }
            متغير الوصف = صف(لون::أحمر);
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_duplicate_enum_still_errors_once() {
        let result = analyze(
            r#"
            تعداد لون { أحمر }
            تعداد لون { أخضر }
        "#,
        );
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(
            errors
                .iter()
                .filter(|e| e.code.as_ref().is_some_and(|c| c == "د٠١٠١"))
                .count(),
            1
        );
    }

    // Only top-level declarations are hoisted; a nested function stays
    // invisible before its declaration, matching common language semantics.
    #[test]
    fn test_nested_function_forward_call_still_errors() {
        let result = analyze(
            r#"
            دالة خارجية() {
                داخلية();
                دالة داخلية() {
                    متغير س = 1;
                }
            }
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_nested_function_defined_then_called() {
        let result = analyze(
            r#"
            دالة خارجية() -> عدد {
                دالة داخلية() -> عدد {
                    أرجع 1;
                }
                أرجع داخلية();
            }
        "#,
        );
        assert!(result.is_ok());
    }

    // صدّر wraps the declaration in Export(Declaration(_)); the hoist pass
    // must unwrap it or exported functions would never be defined.
    #[test]
    fn test_exported_function_forward_call() {
        let result = analyze(
            r#"
            اطبع(مساعدة());
            صدّر دالة مساعدة() -> عدد {
                أرجع 5;
            }
        "#,
        );
        assert!(result.is_ok());
    }
}
