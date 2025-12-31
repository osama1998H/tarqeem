//! Scope and symbol table management

use super::types::Type;
use crate::error::Span;
use indexmap::IndexMap;
use unicode_normalization::UnicodeNormalization;

fn normalize_name(name: &str) -> String {
    name.nfc().collect()
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub ty: Type,
    pub mutable: bool,
    pub defined: bool,
    /// Whether the symbol has been accessed/used in the code.
    /// Used for unused variable/function warnings.
    pub used: bool,
    /// The source location where this symbol was declared.
    /// Used for warning messages to point to the declaration site.
    /// Built-in symbols use `Span::default()`.
    pub span: Span,
}

impl Symbol {
    pub fn new(name: impl Into<String>, kind: SymbolKind, ty: Type, span: Span) -> Self {
        Self {
            name: name.into(),
            kind,
            ty,
            mutable: true,
            defined: true,
            used: false,
            span,
        }
    }

    pub fn variable(name: impl Into<String>, ty: Type, mutable: bool, span: Span) -> Self {
        Self {
            name: name.into(),
            kind: SymbolKind::Variable,
            ty,
            mutable,
            defined: true,
            used: false,
            span,
        }
    }

    pub fn function(name: impl Into<String>, params: Vec<Type>, return_type: Type, span: Span) -> Self {
        Self {
            name: name.into(),
            kind: SymbolKind::Function,
            ty: Type::Function {
                params,
                return_type: Box::new(return_type),
            },
            mutable: false,
            defined: true,
            used: false,
            span,
        }
    }

    pub fn class(name: impl Into<String>, span: Span) -> Self {
        let name = name.into();
        Self {
            name: name.clone(),
            kind: SymbolKind::Class,
            ty: Type::Class(name),
            mutable: false,
            defined: true,
            used: false,
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Function,
    Class,
    Interface,
    Enum,
    Parameter,
    Import,
}

#[derive(Debug)]
pub struct Scope {
    symbols: IndexMap<String, Symbol>,
    parent: Option<Box<Scope>>,
    kind: ScopeKind,
    return_type: Option<Type>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScopeKind {
    Global,
    Function,
    Block,
    Class,
    Loop,
}

impl Scope {
    pub fn new_global() -> Self {
        let mut scope = Self {
            symbols: IndexMap::new(),
            parent: None,
            kind: ScopeKind::Global,
            return_type: None,
        };

        Self::register_builtins(&mut scope);

        scope
    }

    fn register_builtins(scope: &mut Scope) {
        // Helper for creating builtin function symbols (no source location)
        fn builtin(name: &str, params: Vec<Type>, return_type: Type) -> Symbol {
            Symbol::function(name, params, return_type, Span::default())
        }

        scope.define(builtin("اطبع", vec![Type::Any], Type::Void));
        scope.define(builtin("طباعة", vec![Type::Any], Type::Void));
        scope.define(builtin("اطبع_سطر", vec![Type::Any], Type::Void));
        scope.define(builtin("اطبع_خطأ", vec![Type::Any], Type::Void));

        scope.define(builtin("ادخل", vec![], Type::String));
        scope.define(builtin("ادخل_رسالة", vec![Type::String], Type::String));
        scope.define(builtin("ادخل_عدد", vec![], Type::Int));
        scope.define(builtin("ادخل_عشري", vec![], Type::Float));

        scope.define(builtin("طول", vec![Type::Any], Type::Int));
        scope.define(builtin("نوع", vec![Type::Any], Type::String));

        scope.define(builtin("عدد", vec![Type::Any], Type::Int));
        scope.define(builtin("عدد_عشري", vec![Type::Any], Type::Float));
        scope.define(builtin("نص", vec![Type::Any], Type::String));
        scope.define(builtin("منطقي", vec![Type::Any], Type::Bool));

        scope.define(builtin("مطلق", vec![Type::Any], Type::Any));
        scope.define(builtin("مطلق_عدد", vec![Type::Int], Type::Int));

        scope.define(builtin("قوة", vec![Type::Float, Type::Float], Type::Float));
        scope.define(builtin("قوة_عدد", vec![Type::Int, Type::Int], Type::Int));

        scope.define(builtin("جذر", vec![Type::Float], Type::Float));
        scope.define(builtin("جذر_تكعيبي", vec![Type::Float], Type::Float));

        scope.define(builtin("لوغاريتم", vec![Type::Float], Type::Float));
        scope.define(builtin("لوغ10", vec![Type::Float], Type::Float));
        scope.define(builtin("لوغاريتم10", vec![Type::Float], Type::Float));
        scope.define(builtin("لوغ2", vec![Type::Float], Type::Float));

        scope.define(builtin("أس", vec![Type::Float], Type::Float));
        scope.define(builtin("أسي", vec![Type::Float], Type::Float));

        scope.define(builtin("أرضية", vec![Type::Float], Type::Float));
        scope.define(builtin("سقف", vec![Type::Float], Type::Float));
        scope.define(builtin("قرّب", vec![Type::Float], Type::Float));
        scope.define(builtin("تقريب", vec![Type::Float], Type::Float));
        scope.define(builtin("اقتطع", vec![Type::Float], Type::Float));

        scope.define(builtin("أقل", vec![Type::Any, Type::Any], Type::Any));
        scope.define(builtin("أدنى", vec![Type::Any, Type::Any], Type::Any));
        scope.define(builtin("أقل_عدد", vec![Type::Int, Type::Int], Type::Int));
        scope.define(builtin("أكبر", vec![Type::Any, Type::Any], Type::Any));
        scope.define(builtin("أقصى", vec![Type::Any, Type::Any], Type::Any));
        scope.define(builtin("أكبر_عدد", vec![Type::Int, Type::Int], Type::Int));

        scope.define(builtin(
            "حصر",
            vec![Type::Any, Type::Any, Type::Any],
            Type::Any,
        ));
        scope.define(builtin(
            "حصر_عدد",
            vec![Type::Int, Type::Int, Type::Int],
            Type::Int,
        ));

        scope.define(builtin("علامة", vec![Type::Int], Type::Int));
        scope.define(builtin("باقي", vec![Type::Int, Type::Int], Type::Int));

        scope.define(builtin("قاسم_مشترك", vec![Type::Int, Type::Int], Type::Int));
        scope.define(builtin("مضاعف_مشترك", vec![Type::Int, Type::Int], Type::Int));

        scope.define(builtin("عاملي", vec![Type::Int], Type::Int));

        scope.define(builtin("جا", vec![Type::Float], Type::Float));
        scope.define(builtin("جيب", vec![Type::Float], Type::Float));
        scope.define(builtin("جتا", vec![Type::Float], Type::Float));
        scope.define(builtin("جيب_التمام", vec![Type::Float], Type::Float));
        scope.define(builtin("ظا", vec![Type::Float], Type::Float));
        scope.define(builtin("ظل", vec![Type::Float], Type::Float));
        scope.define(builtin("ظتا", vec![Type::Float], Type::Float));
        scope.define(builtin("ظل_التمام", vec![Type::Float], Type::Float));
        scope.define(builtin("قا", vec![Type::Float], Type::Float));
        scope.define(builtin("قاطع", vec![Type::Float], Type::Float));
        scope.define(builtin("قتا", vec![Type::Float], Type::Float));
        scope.define(builtin("قاطع_التمام", vec![Type::Float], Type::Float));

        scope.define(builtin("جا_عكسي", vec![Type::Float], Type::Float));
        scope.define(builtin("جيب_عكسي", vec![Type::Float], Type::Float));
        scope.define(builtin("جتا_عكسي", vec![Type::Float], Type::Float));
        scope.define(builtin("جيب_تمام_عكسي", vec![Type::Float], Type::Float));
        scope.define(builtin("ظا_عكسي", vec![Type::Float], Type::Float));
        scope.define(builtin(
            "ظا_عكسي2",
            vec![Type::Float, Type::Float],
            Type::Float,
        ));

        scope.define(builtin("جا_زائدي", vec![Type::Float], Type::Float));
        scope.define(builtin("جتا_زائدي", vec![Type::Float], Type::Float));
        scope.define(builtin("ظا_زائدي", vec![Type::Float], Type::Float));

        scope.define(builtin("الى_راديان", vec![Type::Float], Type::Float));
        scope.define(builtin("راديان", vec![Type::Float], Type::Float));
        scope.define(builtin("الى_درجات", vec![Type::Float], Type::Float));
        scope.define(builtin("درجات", vec![Type::Float], Type::Float));

        scope.define(builtin("بذرة_عشوائية", vec![Type::Int], Type::Void));
        scope.define(builtin("بذرة_عشوائي", vec![Type::Int], Type::Void));
        scope.define(builtin("عشوائي", vec![], Type::Int));
        scope.define(builtin("عشوائي_عدد", vec![], Type::Int));
        scope.define(builtin("عشوائي_بين", vec![Type::Int, Type::Int], Type::Int));
        scope.define(builtin(
            "عشوائي_عدد_بين",
            vec![Type::Int, Type::Int],
            Type::Int,
        ));
        scope.define(builtin("عشوائي_عشري", vec![], Type::Float));
        scope.define(builtin(
            "عشوائي_عشري_بين",
            vec![Type::Float, Type::Float],
            Type::Float,
        ));
        scope.define(builtin("عشوائي_منطقي", vec![], Type::Bool));

        scope.define(builtin(
            "قص_نص",
            vec![Type::String, Type::Int, Type::Int],
            Type::String,
        ));
        scope.define(builtin(
            "قص_حروف",
            vec![Type::String, Type::Int, Type::Int],
            Type::String,
        ));
        scope.define(builtin("حرف_في", vec![Type::String, Type::Int], Type::String));

        scope.define(builtin(
            "يحتوي",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(builtin(
            "يبدأ_بـ",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(builtin(
            "ينتهي_بـ",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(builtin("موضع", vec![Type::String, Type::String], Type::Int));
        scope.define(builtin(
            "موضع_اخير",
            vec![Type::String, Type::String],
            Type::Int,
        ));
        scope.define(builtin(
            "عدد_مرات",
            vec![Type::String, Type::String],
            Type::Int,
        ));

        scope.define(builtin("كبير", vec![Type::String], Type::String));
        scope.define(builtin("صغير", vec![Type::String], Type::String));
        scope.define(builtin("عنوان", vec![Type::String], Type::String));
        scope.define(builtin("اعكس_نص", vec![Type::String], Type::String));

        scope.define(builtin("ازل_فراغات", vec![Type::String], Type::String));
        scope.define(builtin("ازل_فراغات_يسار", vec![Type::String], Type::String));
        scope.define(builtin("ازل_فراغات_يمين", vec![Type::String], Type::String));

        scope.define(builtin(
            "قسّم",
            vec![Type::String, Type::String],
            Type::Array(Box::new(Type::String)),
        ));
        scope.define(builtin(
            "ادمج",
            vec![Type::Array(Box::new(Type::String)), Type::String],
            Type::String,
        ));

        scope.define(builtin(
            "استبدل",
            vec![Type::String, Type::String, Type::String],
            Type::String,
        ));
        scope.define(builtin(
            "استبدل_كل",
            vec![Type::String, Type::String, Type::String],
            Type::String,
        ));

        scope.define(builtin("كرر_نص", vec![Type::String, Type::Int], Type::String));
        scope.define(builtin("كرر", vec![Type::String, Type::Int], Type::String));
        scope.define(builtin(
            "احشو_يسار",
            vec![Type::String, Type::Int, Type::String],
            Type::String,
        ));
        scope.define(builtin(
            "احشو_يمين",
            vec![Type::String, Type::Int, Type::String],
            Type::String,
        ));

        scope.define(builtin("طول_نص", vec![Type::String], Type::Int));
        scope.define(builtin("طول_حروف", vec![Type::String], Type::Int));

        scope.define(builtin("رقمي", vec![Type::String], Type::Bool));
        scope.define(builtin("حروف_فقط", vec![Type::String], Type::Bool));
        scope.define(builtin("عربي", vec![Type::String], Type::Bool));

        scope.define(builtin("قارن_نص", vec![Type::String, Type::String], Type::Int));
        scope.define(builtin(
            "نصوص_متساوية",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(builtin(
            "نص_يحتوي",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(builtin(
            "نص_يبدأ_بـ",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(builtin(
            "نص_ينتهي_بـ",
            vec![Type::String, Type::String],
            Type::Bool,
        ));

        scope.define(builtin("عدد_لنص", vec![Type::Int], Type::String));
        scope.define(builtin("عشري_لنص", vec![Type::Float], Type::String));
        scope.define(builtin("منطقي_لنص", vec![Type::Bool], Type::String));
        scope.define(builtin("نص_لعدد", vec![Type::String], Type::Int));
        scope.define(builtin("نص_لعشري", vec![Type::String], Type::Float));

        scope.define(builtin("طول_مصفوفة", vec![Type::Any], Type::Int));
        scope.define(builtin("الحق", vec![Type::Any, Type::Any], Type::Void));

        scope.define(builtin("ملف_موجود", vec![Type::String], Type::Bool));
        scope.define(builtin("هل_ملف", vec![Type::String], Type::Bool));
        scope.define(builtin("هل_مجلد", vec![Type::String], Type::Bool));
        scope.define(builtin("اقرأ_ملف", vec![Type::String], Type::String));
        scope.define(builtin(
            "اكتب_ملف",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(builtin(
            "الحق_ملف",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(builtin("احذف_ملف", vec![Type::String], Type::Bool));
        scope.define(builtin(
            "انسخ_ملف",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(builtin(
            "انقل_ملف",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(builtin("حجم_ملف", vec![Type::String], Type::Int));
        scope.define(builtin("انشئ_مجلد", vec![Type::String], Type::Bool));
        scope.define(builtin(
            "قائمة_مجلد",
            vec![Type::String],
            Type::Array(Box::new(Type::String)),
        ));
        scope.define(builtin("احذف_مجلد", vec![Type::String], Type::Bool));
        scope.define(builtin("مجلد_حالي", vec![], Type::String));
        scope.define(builtin("مجلد_مستخدم", vec![], Type::String));
        scope.define(builtin("مجلد_مؤقت", vec![], Type::String));

        scope.define(builtin(
            "ادمج_مسار",
            vec![Type::String, Type::String],
            Type::String,
        ));
        scope.define(builtin("مسار_اب", vec![Type::String], Type::String));
        scope.define(builtin("اسم_ملف", vec![Type::String], Type::String));
        scope.define(builtin("امتداد_ملف", vec![Type::String], Type::String));
        scope.define(builtin("فاصل_مسار", vec![], Type::String));

        scope.define(builtin("نم", vec![Type::Int], Type::Void));
        scope.define(builtin("وقت_الآن", vec![], Type::Int));
        scope.define(builtin("وقت_أداء", vec![], Type::Int));

        scope.define(builtin("توقف", vec![Type::String], Type::Void));
        scope.define(builtin("تأكد", vec![Type::Bool], Type::Void));
        scope.define(builtin(
            "تأكد_رسالة",
            vec![Type::Bool, Type::String],
            Type::Void,
        ));

        // Cryptography - SHA-256 (بصمة = fingerprint)
        scope.define(builtin("احسب_بصمة", vec![Type::String], Type::String));
        scope.define(builtin("بصمة_ملف", vec![Type::String], Type::String));
        scope.define(builtin(
            "بصمة_ثنائي",
            vec![Type::Array(Box::new(Type::Int))],
            Type::String,
        ));
        scope.define(builtin(
            "طابق_بصمة",
            vec![Type::String, Type::String],
            Type::Bool,
        ));

        // Hex encoding (ست_عشري = hexadecimal)
        scope.define(builtin("إلى_ست_عشري", vec![Type::String], Type::String));
        scope.define(builtin("من_ست_عشري", vec![Type::String], Type::String));
        scope.define(builtin(
            "ثنائي_إلى_ست_عشري",
            vec![Type::Array(Box::new(Type::Int))],
            Type::String,
        ));
        scope.define(builtin(
            "ست_عشري_إلى_ثنائي",
            vec![Type::String],
            Type::Array(Box::new(Type::Int)),
        ));

        // Compression (اضغط = compress, فك_الضغط = decompress)
        scope.define(builtin(
            "اضغط",
            vec![Type::String],
            Type::Array(Box::new(Type::Int)),
        ));
        scope.define(builtin(
            "فك_الضغط",
            vec![Type::Array(Box::new(Type::Int))],
            Type::String,
        ));
        scope.define(builtin(
            "اضغط_ثنائي",
            vec![Type::Array(Box::new(Type::Int))],
            Type::Array(Box::new(Type::Int)),
        ));
        scope.define(builtin(
            "فك_ضغط_ثنائي",
            vec![Type::Array(Box::new(Type::Int))],
            Type::Array(Box::new(Type::Int)),
        ));
        scope.define(builtin(
            "اضغط_ملف",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
        scope.define(builtin(
            "فك_ضغط_ملف",
            vec![Type::String, Type::String],
            Type::Bool,
        ));
    }

    pub fn new_child(parent: Scope, kind: ScopeKind) -> Self {
        Self {
            symbols: IndexMap::new(),
            parent: Some(Box::new(parent)),
            kind,
            return_type: None,
        }
    }

    pub fn new_function(parent: Scope, ret_type: Type) -> Self {
        Self {
            symbols: IndexMap::new(),
            parent: Some(Box::new(parent)),
            kind: ScopeKind::Function,
            return_type: Some(ret_type),
        }
    }

    pub fn kind(&self) -> ScopeKind {
        self.kind
    }

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

    pub fn lookup_local(&self, name: &str) -> Option<&Symbol> {
        let normalized = normalize_name(name);
        self.symbols.get(&normalized)
    }

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

    /// Mark a symbol as used. Traverses parent scopes if not found locally.
    /// Returns true if the symbol was found and marked, false otherwise.
    pub fn mark_used(&mut self, name: &str) -> bool {
        let normalized = normalize_name(name);
        if let Some(symbol) = self.symbols.get_mut(&normalized) {
            symbol.used = true;
            true
        } else if let Some(parent) = &mut self.parent {
            parent.mark_used(name)
        } else {
            false
        }
    }

    pub fn is_in_loop(&self) -> bool {
        if self.kind == ScopeKind::Loop {
            true
        } else if let Some(parent) = &self.parent {
            parent.is_in_loop()
        } else {
            false
        }
    }

    pub fn is_in_function(&self) -> bool {
        if self.kind == ScopeKind::Function {
            true
        } else if let Some(parent) = &self.parent {
            parent.is_in_function()
        } else {
            false
        }
    }

    pub fn is_in_class(&self) -> bool {
        if self.kind == ScopeKind::Class {
            true
        } else if let Some(parent) = &self.parent {
            parent.is_in_class()
        } else {
            false
        }
    }

    pub fn get_function_return_type(&self) -> Option<Type> {
        if self.kind == ScopeKind::Function {
            self.return_type.clone()
        } else if let Some(parent) = &self.parent {
            parent.get_function_return_type()
        } else {
            None
        }
    }

    pub fn pop(self) -> Option<Scope> {
        self.parent.map(|p| *p)
    }

    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }

    /// Get all symbols from this scope and all parent scopes.
    pub fn all_symbols(&self) -> Vec<&Symbol> {
        let mut all: Vec<&Symbol> = self.symbols.values().collect();
        if let Some(ref parent) = self.parent {
            all.extend(parent.all_symbols());
        }
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_lookup() {
        let mut global = Scope::new_global();
        global.define(Symbol::variable("x", Type::Int, true, Span::default()));

        let child = Scope::new_child(global, ScopeKind::Block);

        assert!(child.lookup("x").is_some());
        assert!(child.lookup("اطبع").is_some());
        assert!(child.lookup("nonexistent").is_none());
    }

    #[test]
    fn test_scope_shadowing() {
        let mut global = Scope::new_global();
        global.define(Symbol::variable("x", Type::Int, true, Span::default()));

        let mut child = Scope::new_child(global, ScopeKind::Block);
        child.define(Symbol::variable("x", Type::String, true, Span::default()));

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
        let func = Scope::new_function(global, Type::Int);

        assert_eq!(func.get_function_return_type(), Some(Type::Int));
    }

    #[test]
    fn test_function_return_type_from_nested_scope() {
        let global = Scope::new_global();
        let func = Scope::new_function(global, Type::String);
        let block = Scope::new_child(func, ScopeKind::Block);
        let inner_block = Scope::new_child(block, ScopeKind::Block);

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

        assert!(block.get_function_return_type().is_none());
    }
}
