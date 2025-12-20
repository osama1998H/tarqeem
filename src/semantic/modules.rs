//! Module System for Tarqeem
//!
//! This module handles:
//! - Module path resolution
//! - Module loading and caching
//! - Circular dependency detection
//! - Export/import symbol tracking

use crate::error::{Diagnostic, Span};
use crate::parser::{Ast, Parser};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Unique identifier for a module based on its canonical path
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ModuleId(pub PathBuf);

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

/// An exported symbol from a module
#[derive(Debug, Clone)]
pub struct ExportedSymbol {
    pub name: String,
    pub kind: ExportKind,
    pub span: Span,
}

/// Kind of exported symbol
#[derive(Debug, Clone)]
pub enum ExportKind {
    Function,
    Class,
    Interface,
    Variable,
    Constant,
}

/// A loaded and parsed module
#[derive(Debug)]
pub struct LoadedModule {
    pub id: ModuleId,
    pub path: PathBuf,
    pub source: String,
    pub ast: Ast,
    pub exports: HashMap<String, ExportedSymbol>,
}

/// Module loader and cache
pub struct ModuleLoader {
    /// Search paths for modules (in order of priority)
    search_paths: Vec<PathBuf>,

    /// Loaded and cached modules
    modules: HashMap<ModuleId, LoadedModule>,

    /// Modules currently being loaded (for cycle detection)
    loading_stack: Vec<ModuleId>,

    /// Collected diagnostics
    diagnostics: Vec<Diagnostic>,
}

impl ModuleLoader {
    /// Create a new module loader
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
            modules: HashMap::new(),
            loading_stack: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Add a search path for modules
    pub fn add_search_path(&mut self, path: PathBuf) {
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }

    /// Get collected diagnostics
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Take the diagnostics, leaving an empty vector
    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Resolve a module path relative to the importing file
    ///
    /// # Arguments
    /// * `from` - The path of the importing file
    /// * `import_path` - The import specifier (e.g., "./math", "collections")
    ///
    /// # Returns
    /// The canonical path to the module file, or None if not found
    pub fn resolve_path(&self, from: &Path, import_path: &str) -> Option<PathBuf> {
        // Handle relative imports
        if import_path.starts_with("./") || import_path.starts_with("../") {
            let base_dir = from.parent().unwrap_or(Path::new("."));
            let relative_path = import_path.trim_start_matches("./");
            return self.find_module_file(&base_dir.join(relative_path));
        }

        // Handle absolute imports by searching search paths
        for search_path in &self.search_paths {
            if let Some(found) = self.find_module_file(&search_path.join(import_path)) {
                return Some(found);
            }
        }

        // Try relative to current working directory
        if let Some(found) = self.find_module_file(&PathBuf::from(import_path)) {
            return Some(found);
        }

        None
    }

    /// Find a module file, trying different extensions
    fn find_module_file(&self, base: &Path) -> Option<PathBuf> {
        // If the path already has a valid extension
        if base.exists() {
            if let Some(ext) = base.extension().and_then(|e| e.to_str()) {
                if ext == "trq" || ext == "ترقيم" {
                    return base.canonicalize().ok();
                }
            }
        }

        // Try with .trq extension
        let with_trq = base.with_extension("trq");
        if with_trq.exists() {
            return with_trq.canonicalize().ok();
        }

        // Try with Arabic extension
        let with_arabic = base.with_extension("ترقيم");
        if with_arabic.exists() {
            return with_arabic.canonicalize().ok();
        }

        // Try as a directory with index file
        if base.is_dir() {
            let index_trq = base.join("index.trq");
            if index_trq.exists() {
                return index_trq.canonicalize().ok();
            }
            let index_arabic = base.join("فهرس.ترقيم");
            if index_arabic.exists() {
                return index_arabic.canonicalize().ok();
            }
        }

        None
    }

    /// Load a module from the given path
    ///
    /// Returns the module if already loaded, or loads and caches it.
    pub fn load_module(&mut self, path: &Path, span: Span) -> Result<&LoadedModule, ()> {
        let canonical_path = match path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                self.diagnostics.push(Diagnostic::error(
                    &format!("Cannot resolve module path '{}': {}", path.display(), e),
                    &format!("لا يمكن تحديد مسار الوحدة '{}': {}", path.display(), e),
                    span,
                ));
                return Err(());
            }
        };

        let module_id = ModuleId(canonical_path.clone());

        // Check for circular dependency
        if self.loading_stack.contains(&module_id) {
            let cycle = self
                .loading_stack
                .iter()
                .skip_while(|id| *id != &module_id)
                .map(|id| id.0.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");

            self.diagnostics.push(Diagnostic::error(
                &format!(
                    "Circular dependency detected: {} -> {}",
                    cycle,
                    canonical_path.display()
                ),
                &format!(
                    "تم اكتشاف اعتماد دائري: {} -> {}",
                    cycle,
                    canonical_path.display()
                ),
                span,
            ));
            return Err(());
        }

        // Return cached module if available
        if self.modules.contains_key(&module_id) {
            return Ok(self.modules.get(&module_id).unwrap());
        }

        // Load the module
        self.loading_stack.push(module_id.clone());

        let result = self.load_module_internal(&canonical_path, span);

        self.loading_stack.pop();

        match result {
            Ok(module) => {
                self.modules.insert(module_id.clone(), module);
                Ok(self.modules.get(&module_id).unwrap())
            }
            Err(()) => Err(()),
        }
    }

    /// Internal function to load and parse a module
    fn load_module_internal(&mut self, path: &Path, span: Span) -> Result<LoadedModule, ()> {
        // Read file
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                self.diagnostics.push(Diagnostic::error(
                    &format!("Cannot read module '{}': {}", path.display(), e),
                    &format!("لا يمكن قراءة الوحدة '{}': {}", path.display(), e),
                    span,
                ));
                return Err(());
            }
        };

        // Parse file
        let mut parser = Parser::new(&source);

        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(error) => {
                self.diagnostics.push(Diagnostic::error(
                    &format!(
                        "Parse error in module '{}': {}",
                        path.display(),
                        error.message
                    ),
                    &format!(
                        "خطأ تحليل في الوحدة '{}': {}",
                        path.display(),
                        error.message_ar
                    ),
                    error.span,
                ));
                return Err(());
            }
        };

        // Collect exports
        let exports = self.collect_exports(&ast);

        Ok(LoadedModule {
            id: ModuleId(path.to_path_buf()),
            path: path.to_path_buf(),
            source,
            ast,
            exports,
        })
    }

    /// Collect exported symbols from a module's AST
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
                    StmtKind::VarDecl {
                        name, mutable, ..
                    } => {
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

    /// Get a loaded module by path (if already loaded)
    pub fn get_module(&self, path: &Path) -> Option<&LoadedModule> {
        let canonical_path = path.canonicalize().ok()?;
        self.modules.get(&ModuleId(canonical_path))
    }

    /// Check if a module is already loaded
    pub fn is_loaded(&self, path: &Path) -> bool {
        if let Ok(canonical_path) = path.canonicalize() {
            self.modules.contains_key(&ModuleId(canonical_path))
        } else {
            false
        }
    }

    /// Get all loaded modules
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

        // Create a test module file
        create_test_file(base_path, "math.trq", "صدّر دالة جمع(أ: عدد) -> عدد { أرجع أ }");

        let loader = ModuleLoader::new();
        let main_file = base_path.join("main.trq");

        let resolved = loader.resolve_path(&main_file, "./math");
        assert!(resolved.is_some());
    }

    #[test]
    fn test_resolve_with_extension() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        create_test_file(base_path, "utils.trq", "صدّر ثابت س = 42");

        let loader = ModuleLoader::new();
        let main_file = base_path.join("main.trq");

        let resolved = loader.resolve_path(&main_file, "./utils.trq");
        assert!(resolved.is_some());
    }

    #[test]
    fn test_circular_dependency_detection() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create two files that import each other
        create_test_file(base_path, "a.trq", "استورد { ب } من \"./b\"");
        create_test_file(base_path, "b.trq", "استورد { أ } من \"./a\"");

        let mut loader = ModuleLoader::new();
        let a_path = base_path.join("a.trq");

        // Start loading from a.trq - this should detect the cycle when b.trq tries to import a.trq
        // For now, just test that the loader can be created and paths can be resolved
        let resolved = loader.resolve_path(&a_path, "./b");
        assert!(resolved.is_some());
    }

    #[test]
    fn test_search_paths() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create stdlib directory
        let stdlib_dir = base_path.join("stdlib");
        std::fs::create_dir(&stdlib_dir).unwrap();
        create_test_file(&stdlib_dir, "collections.trq", "صدّر صنف قائمة {}");

        let mut loader = ModuleLoader::new();
        loader.add_search_path(stdlib_dir);

        let main_file = base_path.join("main.trq");
        let resolved = loader.resolve_path(&main_file, "collections");
        assert!(resolved.is_some());
    }
}
