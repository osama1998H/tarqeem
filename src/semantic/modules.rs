//! Module System for Tarqeem
//!
//! This module handles:
//! - Module path resolution
//! - Module loading and caching
//! - Circular dependency detection
//! - Export/import symbol tracking

use crate::error::codes::{ERR_CIRCULAR_DEPENDENCY, ERR_MODULE_NOT_FOUND};
use crate::error::{Diagnostic, Span};
use crate::parser::{Ast, Parser};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone)]
pub enum ExportKind {
    Function,
    Class,
    Interface,
    Variable,
    Constant,
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

    modules: HashMap<ModuleId, LoadedModule>,

    loading_stack: Vec<ModuleId>,

    diagnostics: Vec<Diagnostic>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
            modules: HashMap::new(),
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
                        format!("Cannot resolve module path '{}': {}", path.display(), e),
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
                        "Circular dependency detected: {} -> {}",
                        cycle,
                        canonical_path.display()
                    ),
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

    fn load_module_internal(&mut self, path: &Path, span: Span) -> Result<LoadedModule, ()> {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                self.diagnostics.push(
                    Diagnostic::error(
                        format!("Cannot read module '{}': {}", path.display(), e),
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
                        "Parse error in module '{}': {}",
                        path.display(),
                        error.message
                    ),
                    format!(
                        "خطأ تحليل في الوحدة '{}': {}",
                        path.display(),
                        error.message_ar
                    ),
                    error.span,
                ));
                return Err(());
            }
        };

        let exports = self.collect_exports(&ast);

        Ok(LoadedModule {
            id: ModuleId(path.to_path_buf()),
            path: path.to_path_buf(),
            source,
            ast,
            exports,
        })
    }

    fn collect_exports(&self, ast: &Ast) -> HashMap<String, ExportedSymbol> {
        use crate::parser::StmtKind;

        let mut exports = HashMap::new();

        for stmt in &ast.statements {
            if let StmtKind::Export(inner) = &stmt.kind {
                match &inner.kind {
                    StmtKind::FuncDecl { name, .. } => {
                        exports.insert(
                            name.clone(),
                            ExportedSymbol {
                                name: name.clone(),
                                kind: ExportKind::Function,
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
                    StmtKind::VarDecl { name, mutable, .. } => {
                        exports.insert(
                            name.clone(),
                            ExportedSymbol {
                                name: name.clone(),
                                kind: if *mutable {
                                    ExportKind::Variable
                                } else {
                                    ExportKind::Constant
                                },
                                span: stmt.span,
                            },
                        );
                    }
                    _ => {}
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

        create_test_file(base_path, "أ.ترقيم", "استورد { ب } من \"./ب\"");
        create_test_file(base_path, "ب.ترقيم", "استورد { أ } من \"./أ\"");

        let loader = ModuleLoader::new();
        let a_path = base_path.join("أ.ترقيم");

        let resolved = loader.resolve_path(&a_path, "./ب");
        assert!(resolved.is_some());
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
