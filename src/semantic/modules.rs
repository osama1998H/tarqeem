//! Module System for Tarqeem
//!
//! This module handles:
//! - Module path resolution
//! - Module loading and caching
//! - Circular dependency detection
//! - Export/import symbol tracking

use crate::error::codes::{ERR_CIRCULAR_DEPENDENCY, ERR_MODULE_NOT_FOUND};
use crate::error::{Diagnostic, Span};
use crate::parser::{Ast, Expr, ExprKind, Literal, Parser, TypeAnnotation};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::analyzer::resolve_type_annotation;
use super::scope::Scope;
use super::types::Type;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ModuleId(pub PathBuf);

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

#[derive(Debug, Clone)]
pub struct ExportedSymbol {
    pub name: String,
    pub kind: ExportKind,
    pub span: Span,
}

/// What an exported name denotes, together with the type it resolves to.
///
/// The types live *inside* the variants so a new export kind cannot be added
/// without deciding what type an importer sees. Before this, importers of a
/// function got `() -> أي` and every call was rejected for arity (issue #182).
#[derive(Debug, Clone)]
pub enum ExportKind {
    Function {
        params: Vec<Type>,
        return_type: Type,
    },
    Class,
    Interface,
    Variable(Type),
    Constant(Type),
}

#[derive(Debug)]
pub struct LoadedModule {
    pub id: ModuleId,
    pub path: PathBuf,
    pub source: String,
    pub ast: Ast,
    pub exports: HashMap<String, ExportedSymbol>,
}

pub struct ModuleLoader {
    search_paths: Vec<PathBuf>,

    /// Insertion-ordered so that iteration is a stable topological order; see
    /// [`ModuleLoader::modules_in_load_order`].
    modules: IndexMap<ModuleId, LoadedModule>,

    loading_stack: Vec<ModuleId>,

    diagnostics: Vec<Diagnostic>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
            modules: IndexMap::new(),
            loading_stack: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn add_search_path(&mut self, path: PathBuf) {
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    pub fn resolve_path(&self, from: &Path, import_path: &str) -> Option<PathBuf> {
        if import_path.starts_with("./") || import_path.starts_with("../") {
            let base_dir = from.parent().unwrap_or(Path::new("."));
            let relative_path = import_path.trim_start_matches("./");
            return self.find_module_file(&base_dir.join(relative_path));
        }

        for search_path in &self.search_paths {
            if let Some(found) = self.find_module_file(&search_path.join(import_path)) {
                return Some(found);
            }
        }

        if let Some(found) = self.find_module_file(&PathBuf::from(import_path)) {
            return Some(found);
        }

        None
    }

    fn find_module_file(&self, base: &Path) -> Option<PathBuf> {
        if base.exists() {
            if let Some(ext) = base.extension().and_then(|e| e.to_str()) {
                if ext == "ترقيم" {
                    return base.canonicalize().ok();
                }
            }
        }

        let with_arabic = base.with_extension("ترقيم");
        if with_arabic.exists() {
            return with_arabic.canonicalize().ok();
        }

        if base.is_dir() {
            let index_arabic = base.join("فهرس.ترقيم");
            if index_arabic.exists() {
                return index_arabic.canonicalize().ok();
            }
        }

        None
    }

    #[allow(clippy::result_unit_err)]
    pub fn load_module(&mut self, path: &Path, span: Span) -> Result<&LoadedModule, ()> {
        let canonical_path = match path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                self.diagnostics.push(
                    Diagnostic::error(
                        format!("لا يمكن تحديد مسار الوحدة '{}': {}", path.display(), e),
                        span,
                    )
                    .with_code(ERR_MODULE_NOT_FOUND.to_string()),
                );
                return Err(());
            }
        };

        let module_id = ModuleId(canonical_path.clone());

        if self.loading_stack.contains(&module_id) {
            let cycle = self
                .loading_stack
                .iter()
                .skip_while(|id| *id != &module_id)
                .map(|id| id.0.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");

            self.diagnostics.push(
                Diagnostic::error(
                    format!(
                        "تم اكتشاف اعتماد دائري: {} -> {}",
                        cycle,
                        canonical_path.display()
                    ),
                    span,
                )
                .with_code(ERR_CIRCULAR_DEPENDENCY.to_string()),
            );
            return Err(());
        }

        if self.modules.contains_key(&module_id) {
            // Safe: we just checked the key exists
            return Ok(self.modules.get(&module_id).expect("key exists"));
        }

        self.loading_stack.push(module_id.clone());

        let result = self.load_module_internal(&canonical_path, span);

        self.loading_stack.pop();

        match result {
            Ok(module) => {
                self.modules.insert(module_id.clone(), module);
                // Safe: we just inserted this key
                Ok(self.modules.get(&module_id).expect("just inserted"))
            }
            Err(()) => Err(()),
        }
    }

    /// Seed the cache with an already-parsed module that has no file behind it.
    ///
    /// Used for the implicit prelude (`super::prelude`). Insert order matters:
    /// `modules_in_load_order` is what `register_module_types` and
    /// `link_program` walk, and the prelude must come first so a user class can
    /// inherit from `استثناء`. It carries no exports — nothing imports it, and
    /// leaving it unexported keeps it out of import resolution entirely.
    pub(crate) fn insert_synthetic_module(&mut self, path: PathBuf, source: String, ast: Ast) {
        let id = ModuleId(path.clone());
        if self.modules.contains_key(&id) {
            return;
        }

        self.modules.insert(
            id.clone(),
            LoadedModule {
                id,
                path,
                source,
                ast,
                exports: HashMap::new(),
            },
        );
    }

    fn load_module_internal(&mut self, path: &Path, span: Span) -> Result<LoadedModule, ()> {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                self.diagnostics.push(
                    Diagnostic::error(
                        format!("لا يمكن قراءة الوحدة '{}': {}", path.display(), e),
                        span,
                    )
                    .with_code(ERR_MODULE_NOT_FOUND.to_string()),
                );
                return Err(());
            }
        };

        let mut parser = Parser::new(&source);

        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(error) => {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "خطأ تحليل في الوحدة '{}': {}",
                        path.display(),
                        error.message
                    ),
                    error.span,
                ));
                return Err(());
            }
        };

        self.load_imported_modules(&ast, path);

        let exports = self.collect_exports(&ast, path);

        Ok(LoadedModule {
            id: ModuleId(path.to_path_buf()),
            path: path.to_path_buf(),
            source,
            ast,
            exports,
        })
    }

    /// Load the modules that `ast` imports, so the cache holds the whole
    /// dependency graph and not just re-exported modules.
    ///
    /// Until this existed only `صدّر * من` / `صدّر {..} من` recursed, so
    /// `أ` importing `ب` importing `ج` never loaded `ج` — and the
    /// `loading_stack` cycle detector could never fire.
    fn load_imported_modules(&mut self, ast: &Ast, current_path: &Path) {
        use crate::parser::StmtKind;

        // Collect first: load_module needs &mut self. Same borrow dance as the
        // wildcard re-export pass in collect_exports.
        let imports: Vec<(String, Span)> = ast
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StmtKind::Import { from, .. } => Some((from.clone(), stmt.span)),
                _ => None,
            })
            .collect();

        for (from, span) in imports {
            // Stdlib names are short-circuited to a builtin table by the
            // analyzer and never read from disk; most files under stdlib_trq/
            // do not parse yet, so following them here would drag unrelated
            // parse errors into every program that imports رياضيات.
            if Scope::get_stdlib_modules().contains(&from.as_str()) {
                continue;
            }

            if let Some(target) = self.resolve_path(current_path, &from) {
                // A dependency that fails to load simply stays out of the
                // cache; the importer's symbols then degrade to `أي`.
                let _ = self.load_module(&target, span);
            }
        }
    }

    fn collect_exports(
        &mut self,
        ast: &Ast,
        current_path: &Path,
    ) -> HashMap<String, ExportedSymbol> {
        use crate::parser::{ExportItems, StmtKind};

        let mut exports = HashMap::new();

        // First pass: collect direct exports and prepare re-export info
        let mut wildcard_reexports: Vec<(String, Span)> = Vec::new();
        let mut named_reexports: Vec<(Vec<crate::parser::ExportItem>, String, Span)> = Vec::new();

        for stmt in &ast.statements {
            if let StmtKind::Export(export_items) = &stmt.kind {
                match export_items {
                    ExportItems::Declaration(inner) => {
                        // Handle صدّر دالة/صنف/ثابت...
                        match &inner.kind {
                            StmtKind::FuncDecl {
                                name,
                                params,
                                return_type,
                                ..
                            } => {
                                // Mirrors `func_signature_types` in
                                // stmt_analyzer: unannotated parameter → أي,
                                // missing `-> نوع` → Void. The two must agree
                                // or the importer and the IR builder disagree
                                // about the callee's signature.
                                let param_types = params
                                    .iter()
                                    .map(|p| {
                                        p.ty.as_ref()
                                            .map(resolve_type_annotation)
                                            .unwrap_or(Type::Any)
                                    })
                                    .collect();
                                let ret_type = return_type
                                    .as_ref()
                                    .map(resolve_type_annotation)
                                    .unwrap_or(Type::Void);

                                exports.insert(
                                    name.clone(),
                                    ExportedSymbol {
                                        name: name.clone(),
                                        kind: ExportKind::Function {
                                            params: param_types,
                                            return_type: ret_type,
                                        },
                                        span: stmt.span,
                                    },
                                );
                            }
                            StmtKind::ClassDecl { name, .. } => {
                                exports.insert(
                                    name.clone(),
                                    ExportedSymbol {
                                        name: name.clone(),
                                        kind: ExportKind::Class,
                                        span: stmt.span,
                                    },
                                );
                            }
                            StmtKind::InterfaceDecl { name, .. } => {
                                exports.insert(
                                    name.clone(),
                                    ExportedSymbol {
                                        name: name.clone(),
                                        kind: ExportKind::Interface,
                                        span: stmt.span,
                                    },
                                );
                            }
                            StmtKind::VarDecl {
                                name,
                                mutable,
                                ty,
                                init,
                                ..
                            } => {
                                let value_type = exported_value_type(ty.as_ref(), init.as_ref());
                                exports.insert(
                                    name.clone(),
                                    ExportedSymbol {
                                        name: name.clone(),
                                        kind: if *mutable {
                                            ExportKind::Variable(value_type)
                                        } else {
                                            ExportKind::Constant(value_type)
                                        },
                                        span: stmt.span,
                                    },
                                );
                            }
                            StmtKind::EnumDecl { name, .. } => {
                                exports.insert(
                                    name.clone(),
                                    ExportedSymbol {
                                        name: name.clone(),
                                        kind: ExportKind::Class, // Enums are treated like classes
                                        span: stmt.span,
                                    },
                                );
                            }
                            _ => {}
                        }
                    }
                    ExportItems::Named(items) => {
                        // Handle صدّر { name1، name2 }
                        // Store for later pass to resolve after direct exports are collected
                        for item in items {
                            let export_name = item.alias.as_ref().unwrap_or(&item.name);
                            // Look up the symbol in already collected exports (from Declaration exports)
                            let kind = exports
                                .get(&item.name)
                                .map(|s| s.kind.clone())
                                .unwrap_or(ExportKind::Variable(Type::Any));
                            exports.insert(
                                export_name.clone(),
                                ExportedSymbol {
                                    name: item.name.clone(),
                                    kind,
                                    span: stmt.span,
                                },
                            );
                        }
                    }
                    ExportItems::Wildcard { from } => {
                        // Handle صدّر * من "module"
                        // Store for second pass to resolve after all modules loaded
                        wildcard_reexports.push((from.clone(), stmt.span));
                    }
                    ExportItems::NamedReexport { items, from } => {
                        // Handle صدّر { name1، name2 } من "module"
                        // Store for second pass
                        named_reexports.push((items.clone(), from.clone(), stmt.span));
                    }
                }
            }
        }

        // Second pass: resolve wildcard re-exports
        for (from, span) in wildcard_reexports {
            if let Some(target_path) = self.resolve_path(current_path, &from) {
                // Load the target module (circular deps handled by load_module)
                if let Ok(target_module) = self.load_module(&target_path, span) {
                    // Merge all exports from target module
                    for (name, symbol) in target_module.exports.clone() {
                        exports.entry(name).or_insert(symbol);
                    }
                }
            }
        }

        // Third pass: resolve named re-exports with correct types
        for (items, from, span) in named_reexports {
            if let Some(target_path) = self.resolve_path(current_path, &from) {
                if let Ok(target_module) = self.load_module(&target_path, span) {
                    for item in items {
                        let export_name = item.alias.as_ref().unwrap_or(&item.name);
                        // Try to get the correct type from target module
                        if let Some(target_symbol) = target_module.exports.get(&item.name) {
                            exports.insert(
                                export_name.clone(),
                                ExportedSymbol {
                                    name: item.name.clone(),
                                    kind: target_symbol.kind.clone(),
                                    span,
                                },
                            );
                        } else {
                            // Symbol not found in target, use Variable as fallback
                            exports.insert(
                                export_name.clone(),
                                ExportedSymbol {
                                    name: item.name.clone(),
                                    kind: ExportKind::Variable(Type::Any),
                                    span,
                                },
                            );
                        }
                    }
                }
            }
        }

        exports
    }

    pub fn get_module(&self, path: &Path) -> Option<&LoadedModule> {
        let canonical_path = path.canonicalize().ok()?;
        self.modules.get(&ModuleId(canonical_path))
    }

    pub fn is_loaded(&self, path: &Path) -> bool {
        if let Ok(canonical_path) = path.canonicalize() {
            self.modules.contains_key(&ModuleId(canonical_path))
        } else {
            false
        }
    }

    pub fn loaded_modules(&self) -> impl Iterator<Item = &LoadedModule> {
        self.modules.values()
    }

    /// Loaded modules in dependency-first order.
    ///
    /// `load_module` recurses into a module's dependencies (via
    /// `load_imported_modules` and the re-export passes) *before* inserting the
    /// module itself, and the `contains_key` guard means every `ModuleId` is
    /// inserted at most once — so `IndexMap` insertion order is a post-order
    /// walk of the dependency graph: a dependency always precedes the module
    /// that pulled it in. Merged output depends on this being a valid
    /// topological order, and on it being deterministic across runs.
    pub fn modules_in_load_order(&self) -> impl Iterator<Item = &LoadedModule> {
        self.modules.values()
    }
}

/// Resolve the type of an exported `متغير`/`ثابت`.
///
/// Inference deliberately stops at literals: anything richer would duplicate
/// `expr_analyzer` and drag `Analyzer` state into the module loader. Falling
/// back to `أي` costs analyzer precision only — the IR builder never reads
/// `ExportedSymbol`, since a merged `VarDecl` is typed by its own globals pass.
fn exported_value_type(ty: Option<&TypeAnnotation>, init: Option<&Expr>) -> Type {
    if let Some(annotation) = ty {
        return resolve_type_annotation(annotation);
    }

    match init.map(|expr| &expr.kind) {
        Some(ExprKind::Literal(Literal::Int(_))) => Type::Int,
        Some(ExprKind::Literal(Literal::Float(_))) => Type::Float,
        Some(ExprKind::Literal(Literal::String(_))) => Type::String,
        Some(ExprKind::Literal(Literal::Bool(_))) => Type::Bool,
        _ => Type::Any,
    }
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    /// The parser rejects any file lacking the بسم_الله / الحمد_لله markers, so
    /// every fixture that reaches `load_module` must go through this.
    fn create_module(dir: &Path, name: &str, body: &str) -> PathBuf {
        create_test_file(dir, name, &format!("بسم_الله\n{}\nالحمد_لله", body))
    }

    fn has_code(loader: &ModuleLoader, code: &str) -> bool {
        loader
            .diagnostics()
            .iter()
            .any(|d| d.code.as_deref() == Some(code))
    }

    fn file_names_in_load_order(loader: &ModuleLoader) -> Vec<String> {
        loader
            .modules_in_load_order()
            .map(|m| {
                m.path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn test_resolve_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        create_test_file(
            base_path,
            "رياضيات.ترقيم",
            "صدّر دالة جمع(أ: عدد) -> عدد { أرجع أ }",
        );

        let loader = ModuleLoader::new();
        let main_file = base_path.join("رئيسي.ترقيم");

        let resolved = loader.resolve_path(&main_file, "./رياضيات");
        assert!(resolved.is_some());
    }

    #[test]
    fn test_resolve_with_extension() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        create_test_file(base_path, "أدوات.ترقيم", "صدّر ثابت س = 42");

        let loader = ModuleLoader::new();
        let main_file = base_path.join("رئيسي.ترقيم");

        let resolved = loader.resolve_path(&main_file, "./أدوات.ترقيم");
        assert!(resolved.is_some());
    }

    #[test]
    fn test_circular_dependency_detection() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        let a_path = create_module(base_path, "أ.ترقيم", "استورد { ب } من \"./ب\"");
        create_module(base_path, "ب.ترقيم", "استورد { أ } من \"./أ\"");

        let mut loader = ModuleLoader::new();
        let _ = loader.load_module(&a_path, Span::empty());

        // The cycle is reported by the *nested* load; the outer load still
        // succeeds, so the diagnostic — not the Result — is the evidence.
        assert!(
            has_code(&loader, &ERR_CIRCULAR_DEPENDENCY.to_string()),
            "expected و٠٣٠١, got {:?}",
            loader.diagnostics()
        );
    }

    #[test]
    fn test_transitive_imports_are_loaded() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        create_module(base_path, "ج.ترقيم", "صدّر ثابت س = 42");
        create_module(
            base_path,
            "ب.ترقيم",
            "استورد { س } من \"./ج\"\nصدّر ثابت ص = 1",
        );
        let a_path = create_module(base_path, "أ.ترقيم", "استورد { ص } من \"./ب\"");

        let mut loader = ModuleLoader::new();
        assert!(loader.load_module(&a_path, Span::empty()).is_ok());

        assert!(loader.is_loaded(&base_path.join("ب.ترقيم")));
        assert!(loader.is_loaded(&base_path.join("ج.ترقيم")));
    }

    #[test]
    fn test_load_order_is_dependency_first() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        create_module(base_path, "ج.ترقيم", "صدّر ثابت س = 42");
        create_module(
            base_path,
            "ب.ترقيم",
            "استورد { س } من \"./ج\"\nصدّر ثابت ص = 1",
        );
        let a_path = create_module(base_path, "أ.ترقيم", "استورد { ص } من \"./ب\"");

        let mut loader = ModuleLoader::new();
        assert!(loader.load_module(&a_path, Span::empty()).is_ok());

        assert_eq!(
            file_names_in_load_order(&loader),
            vec!["ج.ترقيم", "ب.ترقيم", "أ.ترقيم"]
        );
    }

    #[test]
    fn test_stdlib_imports_are_not_loaded_from_disk() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // A real file named after a stdlib module, reachable through the search
        // path: without the stdlib skip the transitive pass would load it.
        create_module(base_path, "رياضيات.ترقيم", "صدّر ثابت ط = 3");
        let main_path = create_module(base_path, "رئيسي.ترقيم", "استورد { جذر } من \"رياضيات\"");

        let mut loader = ModuleLoader::new();
        loader.add_search_path(base_path.to_path_buf());
        assert!(loader.load_module(&main_path, Span::empty()).is_ok());

        assert!(!loader.is_loaded(&base_path.join("رياضيات.ترقيم")));
    }

    #[test]
    fn test_exported_function_captures_signature() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        let path = create_module(
            base_path,
            "أدوات.ترقيم",
            "صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد { أرجع أ + ب }",
        );

        let mut loader = ModuleLoader::new();
        let module = loader.load_module(&path, Span::empty()).unwrap();

        match &module.exports.get("جمع").expect("جمع is exported").kind {
            ExportKind::Function {
                params,
                return_type,
            } => {
                assert_eq!(params, &vec![Type::Int, Type::Int]);
                assert_eq!(return_type, &Type::Int);
            }
            other => panic!("expected a function export, got {:?}", other),
        }
    }

    #[test]
    fn test_exported_function_without_return_type_is_void() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        let path = create_module(base_path, "أدوات.ترقيم", "صدّر دالة سجّل(ر) { أرجع }");

        let mut loader = ModuleLoader::new();
        let module = loader.load_module(&path, Span::empty()).unwrap();

        match &module.exports.get("سجّل").expect("سجّل is exported").kind {
            ExportKind::Function {
                params,
                return_type,
            } => {
                // Must match `func_signature_types`: unannotated param → أي,
                // absent `-> نوع` → Void.
                assert_eq!(params, &vec![Type::Any]);
                assert_eq!(return_type, &Type::Void);
            }
            other => panic!("expected a function export, got {:?}", other),
        }
    }

    #[test]
    fn test_exported_value_types() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        let path = create_module(
            base_path,
            "قيم.ترقيم",
            "صدّر ثابت س: عدد_عشري = 1\nصدّر ثابت ص = \"مرحبا\"\nصدّر متغير ع = 7",
        );

        let mut loader = ModuleLoader::new();
        let module = loader.load_module(&path, Span::empty()).unwrap();

        match &module.exports.get("س").expect("س is exported").kind {
            ExportKind::Constant(ty) => assert_eq!(ty, &Type::Float),
            other => panic!("expected an annotated constant, got {:?}", other),
        }
        match &module.exports.get("ص").expect("ص is exported").kind {
            ExportKind::Constant(ty) => assert_eq!(ty, &Type::String),
            other => panic!("expected a literal-inferred constant, got {:?}", other),
        }
        match &module.exports.get("ع").expect("ع is exported").kind {
            ExportKind::Variable(ty) => assert_eq!(ty, &Type::Int),
            other => panic!("expected a literal-inferred variable, got {:?}", other),
        }
    }

    #[test]
    fn test_search_paths() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        let stdlib_dir = base_path.join("مكتبة");
        std::fs::create_dir(&stdlib_dir).unwrap();
        create_test_file(&stdlib_dir, "مجموعات.ترقيم", "صدّر صنف قائمة {}");

        let mut loader = ModuleLoader::new();
        loader.add_search_path(stdlib_dir);

        let main_file = base_path.join("رئيسي.ترقيم");
        let resolved = loader.resolve_path(&main_file, "مجموعات");
        assert!(resolved.is_some());
    }
}
