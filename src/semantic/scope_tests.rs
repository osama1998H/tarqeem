//! Comprehensive tests for the Scope and Symbol system

use super::scope::*;
use super::types::Type;
use crate::error::Span;

#[test]
fn test_symbol_new() {
    let symbol = Symbol::new("test", SymbolKind::Variable, Type::Int, Span::default());
    assert_eq!(symbol.name, "test");
    assert_eq!(symbol.kind, SymbolKind::Variable);
    assert_eq!(symbol.ty, Type::Int);
    assert!(symbol.mutable);
    assert!(symbol.defined);
}

#[test]
fn test_symbol_variable() {
    let symbol = Symbol::variable("x", Type::Float, false, Span::default());
    assert_eq!(symbol.name, "x");
    assert_eq!(symbol.kind, SymbolKind::Variable);
    assert_eq!(symbol.ty, Type::Float);
    assert!(!symbol.mutable);
    assert!(symbol.defined);
}

#[test]
fn test_symbol_variable_mutable() {
    let symbol = Symbol::variable("count", Type::Int, true, Span::default());
    assert!(symbol.mutable);
}

#[test]
fn test_symbol_function() {
    let symbol = Symbol::function(
        "add",
        vec![Type::Int, Type::Int],
        Type::Int,
        Span::default(),
    );
    assert_eq!(symbol.name, "add");
    assert_eq!(symbol.kind, SymbolKind::Function);
    assert!(!symbol.mutable);

    match symbol.ty {
        Type::Function {
            params,
            return_type,
        } => {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], Type::Int);
            assert_eq!(params[1], Type::Int);
            assert_eq!(*return_type, Type::Int);
        }
        _ => panic!("Expected function type"),
    }
}

#[test]
fn test_symbol_function_no_params() {
    let symbol = Symbol::function("greet", vec![], Type::Void, Span::default());
    match symbol.ty {
        Type::Function {
            params,
            return_type,
        } => {
            assert!(params.is_empty());
            assert_eq!(*return_type, Type::Void);
        }
        _ => panic!("Expected function type"),
    }
}

#[test]
fn test_symbol_class() {
    let symbol = Symbol::class("Person", Span::default());
    assert_eq!(symbol.name, "Person");
    assert_eq!(symbol.kind, SymbolKind::Class);
    assert_eq!(symbol.ty, Type::Class("Person".to_string()));
    assert!(!symbol.mutable);
}

#[test]
fn test_symbol_class_arabic() {
    let symbol = Symbol::class("شخص", Span::default());
    assert_eq!(symbol.name, "شخص");
    assert_eq!(symbol.ty, Type::Class("شخص".to_string()));
}

#[test]
fn test_symbol_kind_equality() {
    assert_eq!(SymbolKind::Variable, SymbolKind::Variable);
    assert_eq!(SymbolKind::Function, SymbolKind::Function);
    assert_eq!(SymbolKind::Class, SymbolKind::Class);
    assert_eq!(SymbolKind::Interface, SymbolKind::Interface);
    assert_eq!(SymbolKind::Parameter, SymbolKind::Parameter);

    assert_ne!(SymbolKind::Variable, SymbolKind::Function);
    assert_ne!(SymbolKind::Class, SymbolKind::Interface);
}

#[test]
fn test_symbol_kind_clone() {
    let kind = SymbolKind::Function;
    let cloned = kind.clone();
    assert_eq!(kind, cloned);
}

#[test]
fn test_scope_kind_equality() {
    assert_eq!(ScopeKind::Global, ScopeKind::Global);
    assert_eq!(ScopeKind::Function, ScopeKind::Function);
    assert_eq!(ScopeKind::Block, ScopeKind::Block);
    assert_eq!(ScopeKind::Class, ScopeKind::Class);
    assert_eq!(ScopeKind::Loop, ScopeKind::Loop);

    assert_ne!(ScopeKind::Global, ScopeKind::Function);
}

#[test]
fn test_scope_kind_copy() {
    let kind = ScopeKind::Loop;
    let copied = kind;
    assert_eq!(kind, copied);
}

#[test]
fn test_global_scope_creation() {
    let scope = Scope::new_global();
    assert_eq!(scope.kind(), ScopeKind::Global);
}

#[test]
fn test_global_scope_has_core_builtins() {
    // Only 21 core builtins should be available globally without imports
    let scope = Scope::new_global();

    // I/O (6)
    assert!(scope.lookup("اطبع").is_some());
    assert!(scope.lookup("طباعة").is_some());
    assert!(scope.lookup("اطبع_سطر").is_some());
    assert!(scope.lookup("اطبع_خطأ").is_some());
    assert!(scope.lookup("ادخل").is_some());
    assert!(scope.lookup("ادخل_رسالة").is_some());

    // Type introspection (2)
    assert!(scope.lookup("طول").is_some());
    assert!(scope.lookup("نوع").is_some());

    // Type conversion (4)
    assert!(scope.lookup("عدد").is_some());
    assert!(scope.lookup("عدد_عشري").is_some());
    assert!(scope.lookup("نص").is_some());
    assert!(scope.lookup("منطقي").is_some());

    // Control (4)
    assert!(scope.lookup("تأكد").is_some());
    assert!(scope.lookup("تأكد_رسالة").is_some());
    assert!(scope.lookup("توقف").is_some());
    assert!(scope.lookup("نم").is_some());

    // Array (2)
    assert!(scope.lookup("طول_مصفوفة").is_some());
    assert!(scope.lookup("الحق").is_some());

    // Bitwise (7)
    assert!(scope.lookup("بتات_و").is_some());
    assert!(scope.lookup("بتات_أو").is_some());
    assert!(scope.lookup("بتات_أو_حصري").is_some());
    assert!(scope.lookup("بتات_نفي").is_some());
    assert!(scope.lookup("بتات_إزاحة_يسار").is_some());
    assert!(scope.lookup("بتات_إزاحة_يمين").is_some());
    assert!(scope.lookup("بتات_إزاحة_يمين_منطقية").is_some());

    // Strings (2)
    assert!(scope.lookup("حرف_إلى_رمز").is_some());
    assert!(scope.lookup("رمز_إلى_حرف").is_some());

    // Verify stdlib functions are NOT in global scope
    assert!(scope.lookup("مطلق").is_none());
    assert!(scope.lookup("جذر").is_none());
    assert!(scope.lookup("قوة").is_none());
}

#[test]
fn test_stdlib_builtins_available_via_import() {
    // Math functions should be available via get_stdlib_builtin
    assert!(Scope::get_stdlib_builtin("رياضيات", "جا").is_some());
    assert!(Scope::get_stdlib_builtin("رياضيات", "جتا").is_some());
    assert!(Scope::get_stdlib_builtin("رياضيات", "ظا").is_some());
    assert!(Scope::get_stdlib_builtin("رياضيات", "جا_عكسي").is_some());
    assert!(Scope::get_stdlib_builtin("رياضيات", "جذر").is_some());
    assert!(Scope::get_stdlib_builtin("رياضيات", "قوة").is_some());
    assert!(Scope::get_stdlib_builtin("رياضيات", "مطلق").is_some());
}

#[test]
fn test_stdlib_file_functions_require_import() {
    // File functions should NOT be in global scope
    let scope = Scope::new_global();
    assert!(scope.lookup("ملف_موجود").is_none());
    assert!(scope.lookup("اقرأ_ملف").is_none());
    assert!(scope.lookup("اكتب_ملف").is_none());

    // But should be available via import
    assert!(Scope::get_stdlib_builtin("ملفات", "ملف_موجود").is_some());
    assert!(Scope::get_stdlib_builtin("ملفات", "اقرأ_ملف").is_some());
    assert!(Scope::get_stdlib_builtin("ملفات", "اكتب_ملف").is_some());
}

#[test]
fn test_stdlib_random_functions_require_import() {
    // Random functions should NOT be in global scope
    let scope = Scope::new_global();
    assert!(scope.lookup("عشوائي").is_none());
    assert!(scope.lookup("عشوائي_عدد").is_none());

    // But should be available via import from رياضيات
    assert!(Scope::get_stdlib_builtin("رياضيات", "عشوائي").is_some());
    assert!(Scope::get_stdlib_builtin("رياضيات", "عشوائي_عدد").is_some());
    assert!(Scope::get_stdlib_builtin("رياضيات", "عشوائي_عشري").is_some());
    assert!(Scope::get_stdlib_builtin("رياضيات", "عشوائي_منطقي").is_some());
}

#[test]
fn test_get_stdlib_modules() {
    let modules = Scope::get_stdlib_modules();
    assert!(modules.contains(&"رياضيات"));
    assert!(modules.contains(&"نص"));
    assert!(modules.contains(&"ملفات"));
    assert!(modules.contains(&"وقت"));
    assert!(modules.contains(&"تشفير"));
    assert!(modules.contains(&"ضغط"));
    assert!(modules.contains(&"شبكة"));
}

#[test]
fn test_get_stdlib_module_exports() {
    // Math module should have many functions
    let math_exports = Scope::get_stdlib_module_exports("رياضيات");
    assert!(!math_exports.is_empty());
    assert!(math_exports.contains(&"جذر"));
    assert!(math_exports.contains(&"قوة"));

    // String module should have string functions
    let string_exports = Scope::get_stdlib_module_exports("نص");
    assert!(string_exports.contains(&"قص_نص"));
    assert!(string_exports.contains(&"كبير"));

    // Unknown module should return empty
    let unknown_exports = Scope::get_stdlib_module_exports("غير_موجود");
    assert!(unknown_exports.is_empty());
}

#[test]
fn test_scope_define() {
    let mut scope = Scope::new_global();
    let symbol = Symbol::variable("myVar", Type::Int, true, Span::default());

    assert!(scope.define(symbol));
    assert!(scope.lookup("myVar").is_some());
}

#[test]
fn test_scope_define_duplicate() {
    let mut scope = Scope::new_global();

    let symbol1 = Symbol::variable("x", Type::Int, true, Span::default());
    let symbol2 = Symbol::variable("x", Type::String, true, Span::default());

    assert!(scope.define(symbol1));
    assert!(!scope.define(symbol2)); // Should fail - duplicate
}

#[test]
fn test_scope_lookup_nonexistent() {
    let scope = Scope::new_global();
    assert!(scope.lookup("nonexistent_variable").is_none());
}

#[test]
fn test_scope_lookup_local() {
    let mut scope = Scope::new_global();
    scope.define(Symbol::variable("local", Type::Int, true, Span::default()));

    assert!(scope.lookup_local("local").is_some());
    assert!(scope.lookup_local("اطبع").is_some()); // builtin is also local to global
    assert!(scope.lookup_local("undefined").is_none());
}

#[test]
fn test_child_scope_creation() {
    let global = Scope::new_global();
    let child = Scope::new_child(global, ScopeKind::Function);

    assert_eq!(child.kind(), ScopeKind::Function);
}

#[test]
fn test_child_scope_inherits_parent() {
    let mut global = Scope::new_global();
    global.define(Symbol::variable(
        "parentVar",
        Type::Int,
        true,
        Span::default(),
    ));

    let child = Scope::new_child(global, ScopeKind::Block);

    assert!(child.lookup("parentVar").is_some());
    assert!(child.lookup("اطبع").is_some());
}

#[test]
fn test_child_scope_local_shadows_parent() {
    let mut global = Scope::new_global();
    global.define(Symbol::variable("x", Type::Int, true, Span::default()));

    let mut child = Scope::new_child(global, ScopeKind::Block);
    child.define(Symbol::variable("x", Type::String, true, Span::default()));

    let symbol = child.lookup("x").unwrap();
    assert_eq!(symbol.ty, Type::String); // Child's version shadows parent
}

#[test]
fn test_nested_child_scopes() {
    let mut global = Scope::new_global();
    global.define(Symbol::variable("a", Type::Int, true, Span::default()));

    let mut level1 = Scope::new_child(global, ScopeKind::Function);
    level1.define(Symbol::variable("b", Type::String, true, Span::default()));

    let mut level2 = Scope::new_child(level1, ScopeKind::Block);
    level2.define(Symbol::variable("c", Type::Bool, true, Span::default()));

    assert!(level2.lookup("a").is_some());
    assert!(level2.lookup("b").is_some());
    assert!(level2.lookup("c").is_some());
}

#[test]
fn test_is_in_loop_direct() {
    let global = Scope::new_global();
    let loop_scope = Scope::new_child(global, ScopeKind::Loop);

    assert!(loop_scope.is_in_loop());
}

#[test]
fn test_is_in_loop_nested() {
    let global = Scope::new_global();
    let func = Scope::new_child(global, ScopeKind::Function);
    let loop_scope = Scope::new_child(func, ScopeKind::Loop);
    let block = Scope::new_child(loop_scope, ScopeKind::Block);

    assert!(block.is_in_loop());
}

#[test]
fn test_is_not_in_loop() {
    let global = Scope::new_global();
    let func = Scope::new_child(global, ScopeKind::Function);

    assert!(!func.is_in_loop());
}

#[test]
fn test_is_in_function_direct() {
    let global = Scope::new_global();
    let func = Scope::new_child(global, ScopeKind::Function);

    assert!(func.is_in_function());
}

#[test]
fn test_is_in_function_nested() {
    let global = Scope::new_global();
    let func = Scope::new_child(global, ScopeKind::Function);
    let block = Scope::new_child(func, ScopeKind::Block);
    let loop_scope = Scope::new_child(block, ScopeKind::Loop);

    assert!(loop_scope.is_in_function());
}

#[test]
fn test_is_not_in_function() {
    let global = Scope::new_global();
    assert!(!global.is_in_function());
}

#[test]
fn test_is_in_class_direct() {
    let global = Scope::new_global();
    let class = Scope::new_child(global, ScopeKind::Class);

    assert!(class.is_in_class());
}

#[test]
fn test_is_in_class_nested() {
    let global = Scope::new_global();
    let class = Scope::new_child(global, ScopeKind::Class);
    let method = Scope::new_child(class, ScopeKind::Function);

    assert!(method.is_in_class());
}

#[test]
fn test_is_not_in_class() {
    let global = Scope::new_global();
    let func = Scope::new_child(global, ScopeKind::Function);

    assert!(!func.is_in_class());
}

#[test]
fn test_lookup_mut_local() {
    let mut scope = Scope::new_global();
    scope.define(Symbol::variable("x", Type::Int, true, Span::default()));

    let symbol = scope.lookup_mut("x").unwrap();
    symbol.ty = Type::String;

    let updated = scope.lookup("x").unwrap();
    assert_eq!(updated.ty, Type::String);
}

#[test]
fn test_lookup_mut_parent() {
    let mut global = Scope::new_global();
    global.define(Symbol::variable("x", Type::Int, true, Span::default()));

    let mut child = Scope::new_child(global, ScopeKind::Block);

    let symbol = child.lookup_mut("x").unwrap();
    symbol.mutable = false;

    let updated = child.lookup("x").unwrap();
    assert!(!updated.mutable);
}

#[test]
fn test_lookup_mut_nonexistent() {
    let mut scope = Scope::new_global();
    assert!(scope.lookup_mut("nonexistent").is_none());
}

#[test]
fn test_scope_pop() {
    let mut global = Scope::new_global();
    global.define(Symbol::variable("x", Type::Int, true, Span::default()));

    let child = Scope::new_child(global, ScopeKind::Block);

    let parent = child.pop().unwrap();
    assert_eq!(parent.kind(), ScopeKind::Global);
    assert!(parent.lookup("x").is_some());
}

#[test]
fn test_scope_pop_global() {
    let global = Scope::new_global();
    assert!(global.pop().is_none());
}

#[test]
fn test_scope_symbols_iterator() {
    let mut scope = Scope::new_global();
    scope.define(Symbol::variable("a", Type::Int, true, Span::default()));
    scope.define(Symbol::variable("b", Type::String, true, Span::default()));

    let symbols: Vec<_> = scope.symbols().collect();

    assert!(symbols.len() > 2);

    let names: Vec<_> = symbols.iter().map(|s| &s.name).collect();
    assert!(names.contains(&&"a".to_string()));
    assert!(names.contains(&&"b".to_string()));
}

#[test]
fn test_symbol_clone() {
    let symbol = Symbol::function("test", vec![Type::Int], Type::Bool, Span::default());
    let cloned = symbol.clone();

    assert_eq!(symbol.name, cloned.name);
    assert_eq!(symbol.kind, cloned.kind);
    assert_eq!(symbol.ty, cloned.ty);
    assert_eq!(symbol.mutable, cloned.mutable);
}

#[test]
fn test_symbol_debug() {
    let symbol = Symbol::variable("debug_test", Type::Float, true, Span::default());
    let debug_str = format!("{:?}", symbol);

    assert!(debug_str.contains("debug_test"));
    assert!(debug_str.contains("Variable"));
    assert!(debug_str.contains("Float"));
}

#[test]
fn test_unicode_normalization_lookup() {
    use unicode_normalization::UnicodeNormalization;

    let mut scope = Scope::new_global();

    let nfc_name: String = "أحمد".nfc().collect();

    let nfd_name: String = "أحمد".nfd().collect();

    assert_ne!(nfc_name.as_bytes(), nfd_name.as_bytes());

    scope.define(Symbol::variable(
        &nfc_name,
        Type::Int,
        true,
        Span::default(),
    ));

    assert!(scope.lookup(&nfd_name).is_some());
}

#[test]
fn test_unicode_normalization_define() {
    use unicode_normalization::UnicodeNormalization;

    let mut scope = Scope::new_global();

    let nfc_name: String = "متغير".nfc().collect();
    let nfd_name: String = "متغير".nfd().collect();

    scope.define(Symbol::variable(
        &nfd_name,
        Type::Int,
        true,
        Span::default(),
    ));

    assert!(scope.lookup(&nfc_name).is_some());
}

#[test]
fn test_unicode_normalization_prevents_duplicate() {
    use unicode_normalization::UnicodeNormalization;

    let mut scope = Scope::new_global();

    let nfc_name: String = "س".nfc().collect();
    let nfd_name: String = "س".nfd().collect();

    assert!(scope.define(Symbol::variable(
        &nfc_name,
        Type::Int,
        true,
        Span::default()
    )));

    assert!(!scope.define(Symbol::variable(
        &nfd_name,
        Type::String,
        true,
        Span::default()
    )));
}

#[test]
fn test_unicode_normalization_lookup_local() {
    use unicode_normalization::UnicodeNormalization;

    let mut scope = Scope::new_global();

    let nfc_name: String = "محلي".nfc().collect();
    let nfd_name: String = "محلي".nfd().collect();

    scope.define(Symbol::variable(
        &nfc_name,
        Type::Bool,
        true,
        Span::default(),
    ));

    assert!(scope.lookup_local(&nfd_name).is_some());
}

#[test]
fn test_unicode_normalization_lookup_mut() {
    use unicode_normalization::UnicodeNormalization;

    let mut scope = Scope::new_global();

    let nfc_name: String = "قابل_للتغيير".nfc().collect();
    let nfd_name: String = "قابل_للتغيير".nfd().collect();

    scope.define(Symbol::variable(
        &nfc_name,
        Type::Int,
        true,
        Span::default(),
    ));

    let symbol = scope.lookup_mut(&nfd_name).unwrap();
    symbol.ty = Type::String;

    let updated = scope.lookup(&nfc_name).unwrap();
    assert_eq!(updated.ty, Type::String);
}
