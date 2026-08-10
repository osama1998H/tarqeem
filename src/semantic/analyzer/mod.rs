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
use super::linker::{link_program, unwrap_exported_decl};
use super::modules::ModuleLoader;
use super::prelude;
use super::scope::{normalize_name, Scope, ScopeKind, SymbolKind};
use super::types::{parse_type_name, Type};
use crate::error::codes::{
    ERR_CIRCULAR_DEPENDENCY, ERR_REDEFINE_PRELUDE_CLASS, WARN_UNUSED_FUNCTION, WARN_UNUSED_IMPORT,
    WARN_UNUSED_VARIABLE,
};
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
    /// Exports of each module bound by a wildcard import (`استورد * كـ`),
    /// keyed by the specifier `Type::Module` carries. Stdlib modules are
    /// absent: their members have no AST to enumerate and are looked up on
    /// demand through `Scope::get_stdlib_builtin`.
    pub(crate) module_namespaces: HashMap<String, HashMap<String, Type>>,
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
            module_namespaces: HashMap::new(),
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

    /// Fold every module reached by `analyze` into `main`, producing the single
    /// `Ast` that `IrBuilder::build` accepts.
    ///
    /// Must be called *after* `analyze`, which is what populates the module
    /// cache; on a fresh analyzer this returns a clone of `main`.
    ///
    /// `warnings` is an out-parameter because the `Err` arm carries fatal
    /// collisions only; callers emit the warnings and keep going.
    pub fn linked_ast(
        &self,
        main: &Ast,
        warnings: &mut Vec<Diagnostic>,
    ) -> Result<Ast, Vec<Diagnostic>> {
        link_program(
            main,
            &self.module_loader,
            self.current_file.as_deref(),
            warnings,
        )
    }

    /// Get the class resolver.
    pub fn class_resolver(&self) -> &ClassResolver {
        &self.class_resolver
    }

    /// Assignment-position type check: like `Type::is_compatible_with`, plus
    /// upcasting a class to one of its ancestors (issue #184). Use this for
    /// variable initialization, assignment, and call/constructor arguments —
    /// not for `==`, override checks, or generic constraints, which must
    /// keep their pre-existing exact-type semantics.
    pub(crate) fn is_assignable(&self, value: &Type, slot: &Type) -> bool {
        value.is_assignable(slot, &self.class_resolver)
    }

    /// Set the language for error messages.
    pub fn with_language(mut self, lang: Language) -> Self {
        self.language = lang;
        self
    }

    /// Analyze an AST and return any errors.
    pub fn analyze(&mut self, ast: &Ast) -> Result<(), Vec<Diagnostic>> {
        // Modules are loaded here rather than where `analyze_import` first
        // needs them, because that runs in the third pass — after
        // `build_vtables` below — so an imported class could never join the
        // hierarchy in time (issue #182). Rebuilding vtables afterwards is not
        // an option: `validate` drains its own diagnostics, so a second run
        // would report every main-file class error twice.
        // Before any module load, so `modules_in_load_order` yields the prelude
        // first and a user class can inherit from `استثناء` (issue #181).
        self.inject_prelude();

        let module_spans = self.preload_imported_modules(ast);

        // First pass: register all types (classes and interfaces)
        let module_type_spans = self.register_module_types(&module_spans);
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
        self.add_module_type_members();
        for stmt in &ast.statements {
            self.add_type_members(stmt);
        }

        // Build vtables for method dispatch
        self.class_resolver.build_vtables();

        // Validate class hierarchy.
        //
        // A module's classes are registered — `جديد نقطة()` on an imported
        // class needs them — but their hierarchy violations are not the user's
        // to fix and cannot even be shown: every one is anchored to main's
        // `استورد` (or to nothing at all, for a transitively loaded module), so
        // it renders against a line of main that has nothing to do with it.
        // Reporting them made a single `استورد { قائمة } من "مجموعات"` fail
        // every program that imported the stdlib collections. A violation
        // involving a *main* class — including one inheriting from a module
        // class — is anchored to that class and still reported.
        if let Err(diags) = self.class_resolver.validate() {
            self.diagnostics.extend(
                diags
                    .into_iter()
                    .filter(|diagnostic| !module_type_spans.contains(&diagnostic.span)),
            );
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
                // `register_class` is a `HashMap::insert`, so a user class of
                // the same name would replace the prelude's `استثناء` without a
                // word — and `link_program` would then merge two declarations of
                // it into the IR. Refuse instead; inheriting is the supported
                // way to add an exception type (issue #181).
                if normalize_name(name) == normalize_name(prelude::EXCEPTION_CLASS) {
                    self.error_with_code(
                        &format!(
                            "لا يمكن إعادة تعريف صنف الاستثناء الأساسي '{}'؛ ورّثه بدلاً من ذلك: صنف اسمك يرث {}",
                            prelude::EXCEPTION_CLASS, prelude::EXCEPTION_CLASS
                        ),
                        stmt.span,
                        &ERR_REDEFINE_PRELUDE_CLASS.to_string(),
                    );
                    return;
                }

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

    /// Seed the module cache with the implicit prelude, so `استثناء` reaches
    /// the class hierarchy and the merged AST like any other module's classes
    /// (see `super::prelude`).
    ///
    /// A parse failure here is a defect in the compiler's own source, not the
    /// user's, and `prelude::tests` guards it. Silently skipping beats failing
    /// the user's build with a diagnostic they cannot act on; the consequence is
    /// the pre-#181 behaviour, not a crash.
    fn inject_prelude(&mut self) {
        if let Ok((path, source, ast)) = prelude::prelude_ast() {
            self.module_loader
                .insert_synthetic_module(path, source, ast);
        }
    }

    /// Load every module `ast` imports, and map each one to the `استورد` span
    /// that pulled it in.
    ///
    /// Only top-level imports are walked, matching `link_program`: an import
    /// nested inside a block still resolves its own symbols in the third pass,
    /// but contributes no types to the hierarchy.
    fn preload_imported_modules(&mut self, ast: &Ast) -> HashMap<PathBuf, Span> {
        // The same fallback `analyze_import` uses. The two must resolve a
        // specifier to the same file, or the third pass would load and cache a
        // second copy of the module under a different path.
        let current_file = self
            .current_file
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));

        let imports: Vec<(String, Span)> = ast
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StmtKind::Import { from, .. } => Some((from.clone(), stmt.span)),
                _ => None,
            })
            .collect();

        let mut spans = HashMap::new();
        let circular = ERR_CIRCULAR_DEPENDENCY.to_string();

        for (from, span) in imports {
            // Stdlib names resolve against a builtin table and are never read
            // from disk; see `ModuleLoader::load_imported_modules`.
            if Scope::get_stdlib_modules().contains(&from.as_str()) {
                continue;
            }

            let Some(path) = self.module_loader.resolve_path(&current_file, &from) else {
                continue;
            };

            let loaded = self.module_loader.load_module(&path, span).is_ok();

            // Drained per import, because who reported what is only knowable
            // here: this batch is everything one direct import produced.
            let reported = self.module_loader.take_diagnostics();

            if loaded {
                spans.entry(path).or_insert(span);

                // The direct module is fine, so every diagnostic in the batch
                // came from something it pulled in. Nothing re-reports those:
                // `analyze_import` reloads the direct specifier in the third
                // pass and finds it cached, which never revisits its
                // dependencies. Dropping them let a project with a broken
                // transitive module pass `check` with "No errors found!" and
                // then fail at run time with a misleading undefined-function
                // error.
                self.diagnostics.extend(reported);
            } else {
                // The direct module itself failed, and `load_module_internal`
                // gives up before loading any dependency — so the batch is its
                // own failure alone. It is absent from the cache, so
                // `analyze_import` retries in the third pass and reports the
                // same failure there; keeping this copy would report it twice.
                //
                // A cycle is the exception: it is detected by a *nested* load,
                // and every module on it still lands in the cache, so the third
                // pass finds them present and never re-reports. Dropping it
                // here too let `أ` ⇄ `ب` compile and run silently (issue #182).
                self.diagnostics.extend(
                    reported
                        .into_iter()
                        .filter(|diagnostic| diagnostic.code.as_deref() == Some(circular.as_str())),
                );
            }
        }

        spans
    }

    /// Register the classes and interfaces declared by every loaded module, so
    /// that the first pass covers the whole program's hierarchy and not just
    /// main's.
    ///
    /// Diagnostics about these types are anchored to main's `استورد`, never to
    /// a span inside the module: `Span` carries no file identity and
    /// `Diagnostic::emit` renders every span against the main file's source
    /// (the rule `link_program` documents).
    ///
    /// Returns exactly those anchor spans, which is what lets `analyze` tell a
    /// module class's hierarchy diagnostic from a main class's. A `Vec` rather
    /// than a set because `Span` is not `Hash`, and a program imports few
    /// enough modules for the scan to be free.
    fn register_module_types(&mut self, module_spans: &HashMap<PathBuf, Span>) -> Vec<Span> {
        let main_path = self.main_module_path();
        let mut anchors = Vec::new();

        for module in self.module_loader.modules_in_load_order() {
            if main_path.as_deref() == Some(module.path.as_path()) {
                continue;
            }

            // A transitively loaded module has no `استورد` of its own in main.
            let span = module_spans
                .get(&module.path)
                .copied()
                .unwrap_or_else(Span::default);

            let mut registered_here = false;

            for stmt in &module.ast.statements {
                match &unwrap_exported_decl(stmt).kind {
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
                            span,
                        );
                        registered_here = true;
                    }
                    StmtKind::InterfaceDecl { name, .. } => {
                        self.class_resolver.register_interface(name, &[], span);
                        registered_here = true;
                    }
                    _ => {}
                }
            }

            if registered_here {
                anchors.push(span);
            }
        }

        anchors
    }

    /// Second-pass counterpart of `register_module_types`. Split for the same
    /// reason main's two passes are: a module class may inherit from one
    /// declared in main, so every name must be registered before any member is
    /// resolved.
    fn add_module_type_members(&mut self) {
        let main_path = self.main_module_path();

        for module in self.module_loader.modules_in_load_order() {
            if main_path.as_deref() == Some(module.path.as_path()) {
                continue;
            }

            for stmt in &module.ast.statements {
                match &unwrap_exported_decl(stmt).kind {
                    StmtKind::ClassDecl { name, members, .. } => {
                        self.class_resolver.add_class_members(
                            name,
                            members,
                            resolve_type_annotation,
                        );
                    }
                    StmtKind::InterfaceDecl { name, methods, .. } => {
                        self.class_resolver.add_interface_methods(
                            name,
                            methods,
                            resolve_type_annotation,
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    /// Main's own canonical path, when it has one.
    ///
    /// A module that imports the main file back puts main into the module
    /// cache. Its declarations must not be registered from there as well, or
    /// every main class would be built twice — `link_program` skips the same
    /// entry for the same reason.
    fn main_module_path(&self) -> Option<PathBuf> {
        self.current_file
            .as_ref()
            .and_then(|path| path.canonicalize().ok())
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
                // Bare `()` (no return type) resolves to Void — the same
                // idiom `func_signature_types` uses for a `دالة` with no
                // `-> نوع`.
                return_type: Box::new(
                    return_type
                        .as_ref()
                        .map(|t| self.resolve_type(t))
                        .unwrap_or(Type::Void),
                ),
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

    /// Push a lambda scope with return type. Mirrors `push_function_scope`,
    /// but tags the scope `ScopeKind::Lambda` (see `Scope::new_lambda`) so
    /// capture detection and return-type inference (issue #180) can tell an
    /// arrow lambda's body apart from a declared function's.
    pub(crate) fn push_lambda_scope(&mut self, return_type: Type) {
        let old_scope = std::mem::replace(&mut self.scope, Scope::new_global());
        self.scope = Scope::new_lambda(old_scope, return_type);
    }

    /// Runs `f` with `self.expected_type` temporarily set to `expected`,
    /// restoring whatever was there before on return. Unlike a raw
    /// assign-then-reset-to-`None`, this composes correctly when `f` itself
    /// reads or sets `expected_type` (e.g. a lambda argument whose own body
    /// contains a nested array/map literal or lambda that must not inherit
    /// the outer expectation — see `infer_lambda_expr`).
    pub(crate) fn with_expected<R>(
        &mut self,
        expected: Option<Type>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let saved = std::mem::replace(&mut self.expected_type, expected);
        let out = f(self);
        self.expected_type = saved;
        out
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

    /// `Exception`/`Error` are carried for the pre-#181 tests and sources that
    /// hand-declared a base class under an English name; `استثناء` is the one
    /// the prelude actually provides.
    const ERROR_BASE_CLASSES: &'static [&'static str] =
        &[prelude::EXCEPTION_CLASS, "Exception", "Error"];

    /// Whether `ty` may be thrown: `استثناء` itself, or any subclass of it.
    pub(crate) fn is_error_type(&self, ty: &Type) -> bool {
        match ty {
            // `is_subclass` is reflexive, so this covers the base class too.
            Type::Class(class_name) => Self::ERROR_BASE_CLASSES
                .iter()
                .any(|base| self.class_resolver.is_subclass(class_name, base)),
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
            return_type: Box::new(
                return_type
                    .as_ref()
                    .map(|t| resolve_type_annotation(t))
                    .unwrap_or(Type::Void),
            ),
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
    use crate::error::codes::{ERR_NOT_EXPORTED, ERR_THROW_NON_EXCEPTION};
    use crate::parser::Parser;
    use tempfile::TempDir;

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

    /// The class is *not* declared here: `استثناء` comes from the prelude
    /// (`semantic::prelude`). Before #181 every one of these tests had to
    /// hand-declare it, which is precisely why the base class being missing went
    /// unnoticed for the whole v1.0 release.
    #[test]
    fn test_throw_error_object() {
        let result = analyze(
            r#"
            دالة ف() {
                ارمِ جديد استثناء("حدث خطأ");
            }
        "#,
        );
        assert!(result.is_ok(), "{:?}", result.unwrap_err());
    }

    #[test]
    fn test_throw_error_subclass() {
        let result = analyze(
            r#"
            صنف استثناء_قيمة يرث استثناء {
                عام القيمة: عدد;
            }
            دالة ف() {
                ارمِ جديد استثناء_قيمة();
            }
        "#,
        );
        assert!(result.is_ok(), "{:?}", result.unwrap_err());
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
            .any(|e| e.code.as_deref() == Some(ERR_THROW_NON_EXCEPTION.to_string().as_str())));
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
        assert!(errors
            .iter()
            .any(|e| e.code.as_deref() == Some(ERR_THROW_NON_EXCEPTION.to_string().as_str())));
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
        assert!(errors
            .iter()
            .any(|e| e.code.as_deref() == Some(ERR_THROW_NON_EXCEPTION.to_string().as_str())));
    }

    #[test]
    fn test_catch_parameter_typed_as_error() {
        let result = analyze(
            r#"
            دالة ف() {
                حاول {
                    متغير س = 1;
                } التقط (خ) {
                    متغير م = خ.رسالة;
                }
            }
        "#,
        );
        assert!(result.is_ok(), "{:?}", result.unwrap_err());
    }

    #[test]
    fn test_try_catch_finally() {
        let result = analyze(
            r#"
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
        assert!(result.is_ok(), "{:?}", result.unwrap_err());
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

    /// Analyze `main` as a file on disk alongside `modules`, so that a relative
    /// `استورد` resolves the way it does for a real program.
    ///
    /// The `TempDir` travels back in the tuple because dropping it deletes the
    /// fixtures; a caller that discards it would race its own analysis.
    fn analyze_project(
        modules: &[(&str, &str)],
        main: &str,
    ) -> (Analyzer, Result<(), Vec<Diagnostic>>, TempDir) {
        let dir = TempDir::new().unwrap();

        for (name, body) in modules {
            std::fs::write(dir.path().join(name), wrap_with_markers(body)).unwrap();
        }

        let source = wrap_with_markers(main);
        let main_path = dir.path().join("رئيسي.ترقيم");
        std::fs::write(&main_path, &source).unwrap();

        let ast = Parser::new(&source).parse().expect("main must parse");
        let mut analyzer = Analyzer::for_file(main_path);
        let result = analyzer.analyze(&ast);

        (analyzer, result, dir)
    }

    // Issue #182: an imported class used to be missing from the hierarchy
    // entirely, because modules were loaded in the third pass — long after
    // `register_types`.
    #[test]
    fn test_imported_class_is_registered() {
        let (analyzer, result, _dir) = analyze_project(
            &[(
                "نقاط.ترقيم",
                "صدّر صنف نقطة {\n عام س: عدد\n منشئ(س: عدد) { هذا.س = س }\n}",
            )],
            "استورد { نقطة } من \"./نقاط\"\nمتغير ن = جديد نقطة(7)\nاطبع(ن.س)",
        );

        assert!(result.is_ok(), "expected no errors, got {:?}", result);
        assert!(analyzer.class_resolver().get_class("نقطة").is_some());
    }

    // The registration must land before `build_vtables`, not merely before the
    // statement that uses the class: an override only shares its parent's slot
    // if both classes were present when the vtables were built.
    #[test]
    fn test_imported_class_hierarchy_reaches_the_vtable() {
        let (analyzer, result, _dir) = analyze_project(
            &[(
                "أشكال.ترقيم",
                "صدّر صنف شكل {\n عام دالة اسم() -> نص { أرجع \"شكل\" }\n}\n\
                 صدّر صنف دائرة يرث شكل {\n عام دالة اسم() -> نص { أرجع \"دائرة\" }\n}",
            )],
            "استورد { دائرة } من \"./أشكال\"\nمتغير د = جديد دائرة()\nاطبع(د.اسم())",
        );

        assert!(result.is_ok(), "expected no errors, got {:?}", result);

        let resolver = analyzer.class_resolver();
        let parent = resolver.get_class("شكل").expect("شكل must be registered");
        let child = resolver
            .get_class("دائرة")
            .expect("دائرة must be registered");

        assert_eq!(child.vtable, vec!["اسم".to_string()]);
        assert_eq!(
            child.methods["اسم"].vtable_index, parent.methods["اسم"].vtable_index,
            "the override must occupy the inherited slot"
        );
    }

    // Preloading loads each module once, then `analyze_import` loads it again
    // in the third pass. A module that fails is absent from the cache, so the
    // second attempt repeats the failure — only one copy may reach the user.
    #[test]
    fn test_failing_module_is_reported_once() {
        let (_analyzer, result, _dir) = analyze_project(
            &[("معطوب.ترقيم", "صدّر دالة ناقصة( {")],
            "استورد { ناقصة } من \"./معطوب\"\nاطبع(1)",
        );

        let diagnostics = result.expect_err("a module that does not parse must fail analysis");
        let about_module = diagnostics
            .iter()
            .filter(|d| d.message.contains("معطوب"))
            .count();

        assert_eq!(
            about_module, 1,
            "one diagnostic per broken module, got {:?}",
            diagnostics
        );
    }

    // A failure inside a *transitively* imported module has nobody to
    // re-report it: the third pass reloads main's own specifiers only, and
    // finds them cached. Dropping it here let `check` announce "No errors
    // found!" for a project that then died at run time on a function whose
    // module never parsed.
    #[test]
    fn test_failing_transitive_module_is_reported() {
        let (_analyzer, result, _dir) = analyze_project(
            &[
                ("ج.ترقيم", "صدّر دالة ثلاثة() -> عدد { أرجع ((( }"),
                (
                    "ب.ترقيم",
                    "استورد { ثلاثة } من \"./ج\"\n\
                     صدّر دالة ضاعف(س: عدد) -> عدد { أرجع س * 2 }",
                ),
            ],
            "استورد { ضاعف } من \"./ب\"\nاطبع(ضاعف(21))",
        );

        let diagnostics =
            result.expect_err("a broken transitive module must fail the whole program");
        let about_broken = diagnostics
            .iter()
            .filter(|d| d.message.contains("ج.ترقيم"))
            .count();

        assert_eq!(
            about_broken, 1,
            "the transitive failure must be reported exactly once, got {:?}",
            diagnostics
        );
    }

    // Registering a module's classes also fed them to `ClassResolver::validate`,
    // so any hierarchy violation inside a library failed the importing program —
    // `استورد { قائمة } من "مجموعات"` alone was enough. The classes must stay
    // registered, since `جديد` on an imported class needs them.
    #[test]
    fn test_module_class_violation_does_not_fail_main() {
        let (analyzer, result, _dir) = analyze_project(
            &[(
                "أشكال.ترقيم",
                "ميثاق مرسوم {\n دالة ارسم() -> نص\n}\n\
                 صدّر صنف مربع يلتزم مرسوم {\n عام دالة ارسم() -> عدد { أرجع 1 }\n}",
            )],
            "استورد { مربع } من \"./أشكال\"\nمتغير م = جديد مربع()\nاطبع(\"مرحبا\")",
        );

        assert!(
            result.is_ok(),
            "a violation the user cannot see or fix must not fail their program: {:?}",
            result
        );
        assert!(
            analyzer.class_resolver().get_class("مربع").is_some(),
            "the module class must still be registered"
        );
    }

    // The other half of the same filter: a violation by a class the user did
    // write is anchored to that class, so it survives.
    #[test]
    fn test_main_class_violation_against_imported_interface_is_reported() {
        let (_analyzer, result, _dir) = analyze_project(
            &[("أشكال.ترقيم", "صدّر ميثاق مرسوم {\n دالة ارسم() -> نص\n}")],
            "استورد { مرسوم } من \"./أشكال\"\n\
             صنف مربع يلتزم مرسوم {\n عام دالة ارسم() -> عدد { أرجع 1 }\n}\n\
             اطبع(\"مرحبا\")",
        );

        let diagnostics = result.expect_err("a main-file class must still be validated");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("مربع") && d.message.contains("ارسم")),
            "expected the override violation, got {:?}",
            diagnostics
        );
    }

    // Deliberate degradation, not an oversight: an unresolved module exposes no
    // exports, so typing the alias as a module would turn every access through
    // it into an error.
    #[test]
    fn test_wildcard_of_missing_module_stays_any() {
        let (_analyzer, result, _dir) = analyze_project(
            &[],
            "استورد * كـ أدوات من \"./لا_يوجد\"\nاطبع(أدوات.أي_شيء())",
        );

        assert!(
            result.is_ok(),
            "access through an unresolved module must not error: {:?}",
            result
        );
    }

    #[test]
    fn test_wildcard_alias_uses_the_real_export_signature() {
        let (_analyzer, result, _dir) = analyze_project(
            &[(
                "أدوات.ترقيم",
                "صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد { أرجع أ + ب }",
            )],
            "استورد * كـ أدوات من \"./أدوات\"\nمتغير س: عدد = أدوات.جمع(2، 3)",
        );

        assert!(result.is_ok(), "expected no errors, got {:?}", result);
    }

    // The point of `Type::Module` over `أي`: the signature is enforced, not
    // waved through.
    #[test]
    fn test_wildcard_alias_checks_export_arity() {
        let (_analyzer, result, _dir) = analyze_project(
            &[(
                "أدوات.ترقيم",
                "صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد { أرجع أ + ب }",
            )],
            "استورد * كـ أدوات من \"./أدوات\"\nمتغير س: عدد = أدوات.جمع(2)",
        );

        assert!(result.is_err(), "a wrong argument count must be rejected");
    }

    #[test]
    fn test_wildcard_alias_rejects_unknown_export() {
        let (_analyzer, result, _dir) = analyze_project(
            &[("أدوات.ترقيم", "صدّر دالة جمع(أ: عدد) -> عدد { أرجع أ }")],
            "استورد * كـ أدوات من \"./أدوات\"\nمتغير س = أدوات.غير_موجود",
        );

        let diagnostics = result.expect_err("an export that does not exist must be reported");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code.as_deref() == Some(ERR_NOT_EXPORTED.to_string().as_str())),
            "expected و٠٠٠٢, got {:?}",
            diagnostics
        );
    }

    // Stdlib modules keep no AST, so their members resolve one at a time
    // through the builtin table instead of a recorded export map.
    #[test]
    fn test_stdlib_wildcard_alias_resolves_builtin() {
        let result =
            analyze("استورد * كـ رياضيات من \"رياضيات\"\nمتغير ج: عدد_عشري = رياضيات.جذر(16.0)");

        assert!(result.is_ok(), "expected no errors, got {:?}", result);
    }

    #[test]
    fn test_stdlib_wildcard_alias_rejects_unknown_builtin() {
        let result =
            analyze("استورد * كـ رياضيات من \"رياضيات\"\nمتغير ج = رياضيات.لا_وجود_لها(1.0)");

        let diagnostics = result.expect_err("an unknown stdlib member must be reported");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code.as_deref() == Some(ERR_NOT_EXPORTED.to_string().as_str())),
            "expected و٠٠٠٢, got {:?}",
            diagnostics
        );
    }
}
