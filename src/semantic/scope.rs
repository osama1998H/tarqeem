//! Scope and symbol table management

use super::types::Type;
use indexmap::IndexMap;
use unicode_normalization::UnicodeNormalization;

/// Normalize a string to NFC form for consistent identifier comparison.
/// This ensures that Arabic identifiers with different Unicode representations
/// (e.g., composed vs decomposed forms) are treated as identical.
fn normalize_name(name: &str) -> String {
    name.nfc().collect()
}

/// A symbol in the symbol table
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub ty: Type,
    pub mutable: bool,
    pub defined: bool,
}

impl Symbol {
    pub fn new(name: impl Into<String>, kind: SymbolKind, ty: Type) -> Self {
        Self {
            name: name.into(),
            kind,
            ty,
            mutable: true,
            defined: true,
        }
    }

    pub fn variable(name: impl Into<String>, ty: Type, mutable: bool) -> Self {
        Self {
            name: name.into(),
            kind: SymbolKind::Variable,
            ty,
            mutable,
            defined: true,
        }
    }

    pub fn function(name: impl Into<String>, params: Vec<Type>, return_type: Type) -> Self {
        Self {
            name: name.into(),
            kind: SymbolKind::Function,
            ty: Type::Function {
                params,
                return_type: Box::new(return_type),
            },
            mutable: false,
            defined: true,
        }
    }

    pub fn class(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name: name.clone(),
            kind: SymbolKind::Class,
            ty: Type::Class(name),
            mutable: false,
            defined: true,
        }
    }
}

/// The kind of symbol
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Function,
    Class,
    Interface,
    Parameter,
}

/// A scope in the program
#[derive(Debug)]
pub struct Scope {
    /// Symbols defined in this scope
    symbols: IndexMap<String, Symbol>,
    /// Parent scope (if any)
    parent: Option<Box<Scope>>,
    /// Scope kind
    kind: ScopeKind,
    /// Return type for function scopes (used for return statement validation)
    return_type: Option<Type>,
}

/// The kind of scope
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScopeKind {
    Global,
    Function,
    Block,
    Class,
    Loop,
}

impl Scope {
    /// Create a new global scope
    pub fn new_global() -> Self {
        let mut scope = Self {
            symbols: IndexMap::new(),
            parent: None,
            kind: ScopeKind::Global,
            return_type: None,
        };

        // Register all built-in functions
        Self::register_builtins(&mut scope);

        scope
    }

    /// Register all built-in functions in the scope
    fn register_builtins(scope: &mut Scope) {
        // ==========================================================================
        // I/O Functions (الإدخال والإخراج)
        // ==========================================================================

        // Print functions
        scope.define(Symbol::function("اطبع", vec![Type::Any], Type::Void));
        scope.define(Symbol::function("print", vec![Type::Any], Type::Void));
        scope.define(Symbol::function("طباعة", vec![Type::Any], Type::Void));
        scope.define(Symbol::function("println", vec![Type::Any], Type::Void));
        scope.define(Symbol::function("اطبع_سطر", vec![Type::Any], Type::Void));

        // Input functions
        scope.define(Symbol::function("ادخل", vec![], Type::String));
        scope.define(Symbol::function("input", vec![], Type::String));
        scope.define(Symbol::function(
            "ادخل_رسالة",
            vec![Type::String],
            Type::String,
        ));
        scope.define(Symbol::function(
            "input_prompt",
            vec![Type::String],
            Type::String,
        ));
        scope.define(Symbol::function("ادخل_عدد", vec![], Type::Int));
        scope.define(Symbol::function("input_int", vec![], Type::Int));
        scope.define(Symbol::function("ادخل_عشري", vec![], Type::Float));
        scope.define(Symbol::function("input_float", vec![], Type::Float));

        // ==========================================================================
        // Introspection Functions (دوال الاستعلام)
        // ==========================================================================

        scope.define(Symbol::function("طول", vec![Type::Any], Type::Int));
        scope.define(Symbol::function("len", vec![Type::Any], Type::Int));
        scope.define(Symbol::function("length", vec![Type::Any], Type::Int));

        scope.define(Symbol::function("نوع", vec![Type::Any], Type::String));
        scope.define(Symbol::function("type", vec![Type::Any], Type::String));
        scope.define(Symbol::function("typeof", vec![Type::Any], Type::String));

        // ==========================================================================
        // Type Conversion Functions (دوال تحويل الأنماط)
        // ==========================================================================

        // To integer
        scope.define(Symbol::function("عدد", vec![Type::Any], Type::Int));
        scope.define(Symbol::function("int", vec![Type::Any], Type::Int));

        // To float
        scope.define(Symbol::function("عدد_عشري", vec![Type::Any], Type::Float));
        scope.define(Symbol::function("float", vec![Type::Any], Type::Float));

        // To string
        scope.define(Symbol::function("نص", vec![Type::Any], Type::String));
        scope.define(Symbol::function("str", vec![Type::Any], Type::String));
        scope.define(Symbol::function("string", vec![Type::Any], Type::String));

        // To boolean
        scope.define(Symbol::function("منطقي", vec![Type::Any], Type::Bool));
        scope.define(Symbol::function("bool", vec![Type::Any], Type::Bool));

        // ==========================================================================
        // Basic Math Functions (دوال رياضية أساسية)
        // ==========================================================================

        // Absolute value
        scope.define(Symbol::function("مطلق", vec![Type::Any], Type::Any));
        scope.define(Symbol::function("abs", vec![Type::Any], Type::Any));

        // Power
        scope.define(Symbol::function(
            "قوة",
            vec![Type::Float, Type::Float],
            Type::Float,
        ));
        scope.define(Symbol::function(
            "pow",
            vec![Type::Float, Type::Float],
            Type::Float,
        ));

        // Roots
        scope.define(Symbol::function("جذر", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("sqrt", vec![Type::Float], Type::Float));
        scope.define(Symbol::function(
            "جذر_تكعيبي",
            vec![Type::Float],
            Type::Float,
        ));
        scope.define(Symbol::function("cbrt", vec![Type::Float], Type::Float));

        // Logarithms
        scope.define(Symbol::function("لوغاريتم", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("log", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("لوغ10", vec![Type::Float], Type::Float));
        scope.define(Symbol::function(
            "لوغاريتم10",
            vec![Type::Float],
            Type::Float,
        ));
        scope.define(Symbol::function("log10", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("لوغ2", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("log2", vec![Type::Float], Type::Float));

        // Exponential
        scope.define(Symbol::function("أس", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("أسي", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("exp", vec![Type::Float], Type::Float));

        // Rounding
        scope.define(Symbol::function("أرضية", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("floor", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("سقف", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("ceil", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("قرب", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("تقريب", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("round", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("اقتطع", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("trunc", vec![Type::Float], Type::Float));

        // Min/Max
        scope.define(Symbol::function(
            "أقل",
            vec![Type::Any, Type::Any],
            Type::Any,
        ));
        scope.define(Symbol::function(
            "أدنى",
            vec![Type::Any, Type::Any],
            Type::Any,
        ));
        scope.define(Symbol::function(
            "min",
            vec![Type::Any, Type::Any],
            Type::Any,
        ));
        scope.define(Symbol::function(
            "أكبر",
            vec![Type::Any, Type::Any],
            Type::Any,
        ));
        scope.define(Symbol::function(
            "أقصى",
            vec![Type::Any, Type::Any],
            Type::Any,
        ));
        scope.define(Symbol::function(
            "max",
            vec![Type::Any, Type::Any],
            Type::Any,
        ));

        // Clamp
        scope.define(Symbol::function(
            "حصر",
            vec![Type::Any, Type::Any, Type::Any],
            Type::Any,
        ));
        scope.define(Symbol::function(
            "clamp",
            vec![Type::Any, Type::Any, Type::Any],
            Type::Any,
        ));

        // Sign and modulo
        scope.define(Symbol::function("علامة", vec![Type::Int], Type::Int));
        scope.define(Symbol::function("sign", vec![Type::Int], Type::Int));
        scope.define(Symbol::function(
            "باقي",
            vec![Type::Int, Type::Int],
            Type::Int,
        ));
        scope.define(Symbol::function(
            "mod",
            vec![Type::Int, Type::Int],
            Type::Int,
        ));

        // GCD/LCM
        scope.define(Symbol::function(
            "قاسم_مشترك",
            vec![Type::Int, Type::Int],
            Type::Int,
        ));
        scope.define(Symbol::function(
            "gcd",
            vec![Type::Int, Type::Int],
            Type::Int,
        ));
        scope.define(Symbol::function(
            "مضاعف_مشترك",
            vec![Type::Int, Type::Int],
            Type::Int,
        ));
        scope.define(Symbol::function(
            "lcm",
            vec![Type::Int, Type::Int],
            Type::Int,
        ));

        // Factorial
        scope.define(Symbol::function("عاملي", vec![Type::Int], Type::Int));
        scope.define(Symbol::function("factorial", vec![Type::Int], Type::Int));

        // ==========================================================================
        // Trigonometric Functions (دوال مثلثية)
        // ==========================================================================

        // Basic trig
        scope.define(Symbol::function("جا", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("sin", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("جتا", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("cos", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("ظا", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("tan", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("ظتا", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("cot", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("قا", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("sec", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("قتا", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("csc", vec![Type::Float], Type::Float));

        // Inverse trig
        scope.define(Symbol::function("جا_عكسي", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("asin", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("جتا_عكسي", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("acos", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("ظا_عكسي", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("atan", vec![Type::Float], Type::Float));
        scope.define(Symbol::function(
            "ظا_عكسي2",
            vec![Type::Float, Type::Float],
            Type::Float,
        ));
        scope.define(Symbol::function(
            "atan2",
            vec![Type::Float, Type::Float],
            Type::Float,
        ));

        // Hyperbolic
        scope.define(Symbol::function("جا_زائدي", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("sinh", vec![Type::Float], Type::Float));
        scope.define(Symbol::function(
            "جتا_زائدي",
            vec![Type::Float],
            Type::Float,
        ));
        scope.define(Symbol::function("cosh", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("ظا_زائدي", vec![Type::Float], Type::Float));
        scope.define(Symbol::function("tanh", vec![Type::Float], Type::Float));

        // Angle conversion
        scope.define(Symbol::function(
            "الى_راديان",
            vec![Type::Float],
            Type::Float,
        ));
        scope.define(Symbol::function("راديان", vec![Type::Float], Type::Float));
        scope.define(Symbol::function(
            "to_radians",
            vec![Type::Float],
            Type::Float,
        ));
        scope.define(Symbol::function(
            "الى_درجات",
            vec![Type::Float],
            Type::Float,
        ));
        scope.define(Symbol::function("درجات", vec![Type::Float], Type::Float));
        scope.define(Symbol::function(
            "to_degrees",
            vec![Type::Float],
            Type::Float,
        ));

        // ==========================================================================
        // Random Number Functions (دوال الأرقام العشوائية)
        // ==========================================================================

        scope.define(Symbol::function(
            "بذرة_عشوائية",
            vec![Type::Int],
            Type::Void,
        ));
        scope.define(Symbol::function("random_seed", vec![Type::Int], Type::Void));
        scope.define(Symbol::function("عشوائي", vec![], Type::Int));
        scope.define(Symbol::function("random", vec![], Type::Int));
        scope.define(Symbol::function("random_int", vec![], Type::Int));
        scope.define(Symbol::function(
            "عشوائي_بين",
            vec![Type::Int, Type::Int],
            Type::Int,
        ));
        scope.define(Symbol::function(
            "random_range",
            vec![Type::Int, Type::Int],
            Type::Int,
        ));
        scope.define(Symbol::function("عشوائي_عشري", vec![], Type::Float));
        scope.define(Symbol::function("random_float", vec![], Type::Float));
        scope.define(Symbol::function("عشوائي_منطقي", vec![], Type::Bool));
        scope.define(Symbol::function("random_bool", vec![], Type::Bool));

        // ==========================================================================
        // String Utility Functions (دوال النص)
        // ==========================================================================
        // Note: Most string methods are handled by method_resolver.rs
        // These are global utility functions

        scope.define(Symbol::function(
            "كرر",
            vec![Type::String, Type::Int],
            Type::String,
        ));
        scope.define(Symbol::function(
            "repeat",
            vec![Type::String, Type::Int],
            Type::String,
        ));

        // ==========================================================================
        // File System Functions (دوال نظام الملفات)
        // ==========================================================================

        // File existence/type checks
        scope.define(Symbol::function(
            "ملف_موجود",
            vec![Type::String],
            Type::Bool,
        ));
        scope.define(Symbol::function(
            "file_exists",
            vec![Type::String],
            Type::Bool,
        ));
        scope.define(Symbol::function("هل_ملف", vec![Type::String], Type::Bool));
        scope.define(Symbol::function("is_file", vec![Type::String], Type::Bool));
        scope.define(Symbol::function("هل_مجلد", vec![Type::String], Type::Bool));
        scope.define(Symbol::function("is_dir", vec![Type::String], Type::Bool));

        // File read/write
        scope.define(Symbol::function(
            "اقرأ_ملف",
            vec![Type::String],
            Type::String,
        ));
        scope.define(Symbol::function(
            "read_file",
            vec![Type::String],
            Type::String,
        ));
        scope.define(Symbol::function(
            "اكتب_ملف",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(Symbol::function(
            "write_file",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(Symbol::function(
            "الحق_ملف",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(Symbol::function(
            "append_file",
            vec![Type::String, Type::String],
            Type::Bool,
        ));

        // File operations
        scope.define(Symbol::function("احذف_ملف", vec![Type::String], Type::Bool));
        scope.define(Symbol::function(
            "delete_file",
            vec![Type::String],
            Type::Bool,
        ));
        scope.define(Symbol::function("حجم_ملف", vec![Type::String], Type::Int));
        scope.define(Symbol::function("file_size", vec![Type::String], Type::Int));

        // Directory operations
        scope.define(Symbol::function(
            "انشئ_مجلد",
            vec![Type::String],
            Type::Bool,
        ));
        scope.define(Symbol::function(
            "create_dir",
            vec![Type::String],
            Type::Bool,
        ));
        scope.define(Symbol::function("مجلد_حالي", vec![], Type::String));
        scope.define(Symbol::function("current_dir", vec![], Type::String));
        scope.define(Symbol::function("مجلد_مستخدم", vec![], Type::String));
        scope.define(Symbol::function("home_dir", vec![], Type::String));
        scope.define(Symbol::function("مجلد_مؤقت", vec![], Type::String));
        scope.define(Symbol::function("temp_dir", vec![], Type::String));

        // Path operations
        scope.define(Symbol::function(
            "ادمج_مسار",
            vec![Type::String, Type::String],
            Type::String,
        ));
        scope.define(Symbol::function(
            "path_join",
            vec![Type::String, Type::String],
            Type::String,
        ));
        scope.define(Symbol::function(
            "مسار_اب",
            vec![Type::String],
            Type::String,
        ));
        scope.define(Symbol::function(
            "path_parent",
            vec![Type::String],
            Type::String,
        ));
        scope.define(Symbol::function(
            "اسم_ملف",
            vec![Type::String],
            Type::String,
        ));
        scope.define(Symbol::function(
            "path_filename",
            vec![Type::String],
            Type::String,
        ));
        scope.define(Symbol::function(
            "امتداد_ملف",
            vec![Type::String],
            Type::String,
        ));
        scope.define(Symbol::function(
            "path_extension",
            vec![Type::String],
            Type::String,
        ));

        // ==========================================================================
        // Date/Time Functions (دوال التاريخ والوقت)
        // ==========================================================================

        scope.define(Symbol::function("نم", vec![Type::Int], Type::Void));
        scope.define(Symbol::function("sleep", vec![Type::Int], Type::Void));
        scope.define(Symbol::function("وقت_الآن", vec![], Type::Int));
        scope.define(Symbol::function("time_now", vec![], Type::Int));

        // ==========================================================================
        // Assertion/Panic Functions (دوال التأكيد)
        // ==========================================================================

        scope.define(Symbol::function("توقف", vec![Type::String], Type::Void));
        scope.define(Symbol::function("panic", vec![Type::String], Type::Void));
        scope.define(Symbol::function("تأكد", vec![Type::Bool], Type::Void));
        scope.define(Symbol::function("assert", vec![Type::Bool], Type::Void));
        scope.define(Symbol::function(
            "تأكد_رسالة",
            vec![Type::Bool, Type::String],
            Type::Void,
        ));
        scope.define(Symbol::function(
            "assert_msg",
            vec![Type::Bool, Type::String],
            Type::Void,
        ));
    }

    /// Create a new child scope
    pub fn new_child(parent: Scope, kind: ScopeKind) -> Self {
        Self {
            symbols: IndexMap::new(),
            parent: Some(Box::new(parent)),
            kind,
            return_type: None,
        }
    }

    /// Create a new function scope with a specified return type.
    /// This is used for function scopes to enable return type validation.
    pub fn new_function(parent: Scope, ret_type: Type) -> Self {
        Self {
            symbols: IndexMap::new(),
            parent: Some(Box::new(parent)),
            kind: ScopeKind::Function,
            return_type: Some(ret_type),
        }
    }

    /// Get the scope kind
    pub fn kind(&self) -> ScopeKind {
        self.kind
    }

    /// Define a new symbol in this scope.
    /// The symbol name is normalized to NFC form for consistent comparison.
    pub fn define(&mut self, symbol: Symbol) -> bool {
        let normalized = normalize_name(&symbol.name);
        if self.symbols.contains_key(&normalized) {
            false
        } else {
            let mut symbol = symbol;
            symbol.name = normalized.clone();
            self.symbols.insert(normalized, symbol);
            true
        }
    }

    /// Look up a symbol in this scope or parent scopes.
    /// The name is normalized to NFC form for consistent comparison.
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        let normalized = normalize_name(name);
        if let Some(symbol) = self.symbols.get(&normalized) {
            Some(symbol)
        } else if let Some(parent) = &self.parent {
            parent.lookup(name)
        } else {
            None
        }
    }

    /// Look up a symbol only in this scope (not parents).
    /// The name is normalized to NFC form for consistent comparison.
    pub fn lookup_local(&self, name: &str) -> Option<&Symbol> {
        let normalized = normalize_name(name);
        self.symbols.get(&normalized)
    }

    /// Get a mutable reference to a symbol.
    /// The name is normalized to NFC form for consistent comparison.
    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        let normalized = normalize_name(name);
        if self.symbols.contains_key(&normalized) {
            self.symbols.get_mut(&normalized)
        } else if let Some(parent) = &mut self.parent {
            parent.lookup_mut(name)
        } else {
            None
        }
    }

    /// Check if we're inside a loop
    pub fn is_in_loop(&self) -> bool {
        if self.kind == ScopeKind::Loop {
            true
        } else if let Some(parent) = &self.parent {
            parent.is_in_loop()
        } else {
            false
        }
    }

    /// Check if we're inside a function
    pub fn is_in_function(&self) -> bool {
        if self.kind == ScopeKind::Function {
            true
        } else if let Some(parent) = &self.parent {
            parent.is_in_function()
        } else {
            false
        }
    }

    /// Check if we're inside a class
    pub fn is_in_class(&self) -> bool {
        if self.kind == ScopeKind::Class {
            true
        } else if let Some(parent) = &self.parent {
            parent.is_in_class()
        } else {
            false
        }
    }

    /// Get the return type of the enclosing function
    pub fn get_function_return_type(&self) -> Option<Type> {
        if self.kind == ScopeKind::Function {
            // Return the stored return type if available
            self.return_type.clone()
        } else if let Some(parent) = &self.parent {
            parent.get_function_return_type()
        } else {
            None
        }
    }

    /// Pop this scope and return the parent
    pub fn pop(self) -> Option<Scope> {
        self.parent.map(|p| *p)
    }

    /// Get all symbols in this scope
    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_lookup() {
        let mut global = Scope::new_global();
        global.define(Symbol::variable("x", Type::Int, true));

        let child = Scope::new_child(global, ScopeKind::Block);

        assert!(child.lookup("x").is_some());
        assert!(child.lookup("اطبع").is_some());
        assert!(child.lookup("nonexistent").is_none());
    }

    #[test]
    fn test_scope_shadowing() {
        let mut global = Scope::new_global();
        global.define(Symbol::variable("x", Type::Int, true));

        let mut child = Scope::new_child(global, ScopeKind::Block);
        child.define(Symbol::variable("x", Type::String, true));

        let symbol = child.lookup("x").unwrap();
        assert_eq!(symbol.ty, Type::String);
    }

    #[test]
    fn test_loop_detection() {
        let global = Scope::new_global();
        let func = Scope::new_child(global, ScopeKind::Function);
        let loop_scope = Scope::new_child(func, ScopeKind::Loop);
        let block = Scope::new_child(loop_scope, ScopeKind::Block);

        assert!(block.is_in_loop());
        assert!(block.is_in_function());
    }

    #[test]
    fn test_function_return_type() {
        let global = Scope::new_global();
        // Create a function scope with Int return type
        let func = Scope::new_function(global, Type::Int);

        // The function scope should return the stored return type
        assert_eq!(func.get_function_return_type(), Some(Type::Int));
    }

    #[test]
    fn test_function_return_type_from_nested_scope() {
        let global = Scope::new_global();
        let func = Scope::new_function(global, Type::String);
        let block = Scope::new_child(func, ScopeKind::Block);
        let inner_block = Scope::new_child(block, ScopeKind::Block);

        // Nested scopes should be able to get the function return type
        assert_eq!(inner_block.get_function_return_type(), Some(Type::String));
    }

    #[test]
    fn test_function_return_type_void() {
        let global = Scope::new_global();
        let func = Scope::new_function(global, Type::Void);
        let loop_scope = Scope::new_child(func, ScopeKind::Loop);

        assert_eq!(loop_scope.get_function_return_type(), Some(Type::Void));
    }

    #[test]
    fn test_no_return_type_in_global_scope() {
        let global = Scope::new_global();
        let block = Scope::new_child(global, ScopeKind::Block);

        // No function scope, so no return type
        assert!(block.get_function_return_type().is_none());
    }
}
