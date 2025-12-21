//! Comprehensive tests for the Type system

use super::types::*;

// =============================================================================
// Type Basic Tests
// =============================================================================

#[test]
fn test_type_int() {
    let ty = Type::Int;
    assert!(ty.is_numeric());
    assert!(ty.is_primitive());
    assert_eq!(ty.to_string(), "int");
    assert_eq!(ty.arabic_name(), "عدد");
}

#[test]
fn test_type_float() {
    let ty = Type::Float;
    assert!(ty.is_numeric());
    assert!(ty.is_primitive());
    assert_eq!(ty.to_string(), "float");
    assert_eq!(ty.arabic_name(), "عدد_عشري");
}

#[test]
fn test_type_string() {
    let ty = Type::String;
    assert!(!ty.is_numeric());
    assert!(ty.is_primitive());
    assert_eq!(ty.to_string(), "string");
    assert_eq!(ty.arabic_name(), "نص");
}

#[test]
fn test_type_bool() {
    let ty = Type::Bool;
    assert!(!ty.is_numeric());
    assert!(ty.is_primitive());
    assert_eq!(ty.to_string(), "bool");
    assert_eq!(ty.arabic_name(), "منطقي");
}

#[test]
fn test_type_void() {
    let ty = Type::Void;
    assert!(!ty.is_numeric());
    assert!(ty.is_primitive());
    assert_eq!(ty.to_string(), "void");
    assert_eq!(ty.arabic_name(), "فراغ");
}

#[test]
fn test_type_null() {
    let ty = Type::Null;
    assert!(!ty.is_numeric());
    assert!(ty.is_primitive());
    assert_eq!(ty.to_string(), "null");
    assert_eq!(ty.arabic_name(), "عدم");
}

#[test]
fn test_type_any() {
    let ty = Type::Any;
    assert!(!ty.is_numeric());
    assert!(!ty.is_primitive());
    assert_eq!(ty.to_string(), "any");
    assert_eq!(ty.arabic_name(), "أي");
}

#[test]
fn test_type_never() {
    let ty = Type::Never;
    assert!(!ty.is_numeric());
    assert!(!ty.is_primitive());
    assert_eq!(ty.to_string(), "never");
    assert_eq!(ty.arabic_name(), "أبداً");
}

#[test]
fn test_type_unknown() {
    let ty = Type::Unknown;
    assert!(!ty.is_numeric());
    assert!(!ty.is_primitive());
    assert_eq!(ty.to_string(), "unknown");
    assert_eq!(ty.arabic_name(), "مجهول");
}

#[test]
fn test_type_error() {
    let ty = Type::Error;
    assert!(!ty.is_numeric());
    assert!(!ty.is_primitive());
    assert_eq!(ty.to_string(), "error");
    assert_eq!(ty.arabic_name(), "خطأ");
}

// =============================================================================
// Compound Type Tests
// =============================================================================

#[test]
fn test_type_array() {
    let ty = Type::Array(Box::new(Type::Int));
    assert!(!ty.is_numeric());
    assert!(!ty.is_primitive());
    assert_eq!(ty.to_string(), "array<int>");
    assert_eq!(ty.arabic_name(), "مصفوفة<عدد>");
}

#[test]
fn test_type_array_nested() {
    let ty = Type::Array(Box::new(Type::Array(Box::new(Type::String))));
    assert_eq!(ty.to_string(), "array<array<string>>");
    assert_eq!(ty.arabic_name(), "مصفوفة<مصفوفة<نص>>");
}

#[test]
fn test_type_map() {
    let ty = Type::Map(Box::new(Type::String), Box::new(Type::Int));
    assert!(!ty.is_numeric());
    assert!(!ty.is_primitive());
    assert_eq!(ty.to_string(), "map<string, int>");
    assert_eq!(ty.arabic_name(), "قاموس<نص، عدد>");
}

#[test]
fn test_type_optional() {
    let ty = Type::Optional(Box::new(Type::String));
    assert!(!ty.is_numeric());
    assert!(!ty.is_primitive());
    assert_eq!(ty.to_string(), "string?");
    assert_eq!(ty.arabic_name(), "نص?");
}

#[test]
fn test_type_function() {
    let ty = Type::Function {
        params: vec![Type::Int, Type::String],
        return_type: Box::new(Type::Bool),
    };
    assert!(!ty.is_numeric());
    assert!(!ty.is_primitive());
    assert_eq!(ty.to_string(), "(int, string) -> bool");
    assert_eq!(ty.arabic_name(), "(عدد، نص) -> منطقي");
}

#[test]
fn test_type_function_no_params() {
    let ty = Type::Function {
        params: vec![],
        return_type: Box::new(Type::Void),
    };
    assert_eq!(ty.to_string(), "() -> void");
    assert_eq!(ty.arabic_name(), "() -> فراغ");
}

#[test]
fn test_type_class() {
    let ty = Type::Class("شخص".to_string());
    assert!(!ty.is_numeric());
    assert!(!ty.is_primitive());
    assert_eq!(ty.to_string(), "شخص");
    assert_eq!(ty.arabic_name(), "شخص");
}

#[test]
fn test_type_interface() {
    let ty = Type::Interface("قابل_للمقارنة".to_string());
    assert!(!ty.is_numeric());
    assert!(!ty.is_primitive());
    assert_eq!(ty.to_string(), "قابل_للمقارنة");
}

#[test]
fn test_type_generic() {
    let ty = Type::Generic("T".to_string());
    assert!(!ty.is_numeric());
    assert!(!ty.is_primitive());
    assert_eq!(ty.to_string(), "T");
}

// =============================================================================
// Type Compatibility Tests
// =============================================================================

#[test]
fn test_compatible_same_types() {
    assert!(Type::Int.is_compatible_with(&Type::Int));
    assert!(Type::Float.is_compatible_with(&Type::Float));
    assert!(Type::String.is_compatible_with(&Type::String));
    assert!(Type::Bool.is_compatible_with(&Type::Bool));
}

#[test]
fn test_compatible_any() {
    assert!(Type::Any.is_compatible_with(&Type::Int));
    assert!(Type::Int.is_compatible_with(&Type::Any));
    assert!(Type::Any.is_compatible_with(&Type::String));
    assert!(Type::Any.is_compatible_with(&Type::Array(Box::new(Type::Int))));
}

#[test]
fn test_compatible_unknown() {
    assert!(Type::Unknown.is_compatible_with(&Type::Int));
    assert!(Type::String.is_compatible_with(&Type::Unknown));
}

#[test]
fn test_compatible_int_to_float() {
    assert!(Type::Int.is_compatible_with(&Type::Float));
    // Float cannot be narrowed to int
    assert!(!Type::Float.is_compatible_with(&Type::Int));
}

#[test]
fn test_compatible_null_optional() {
    let optional_int = Type::Optional(Box::new(Type::Int));
    assert!(Type::Null.is_compatible_with(&optional_int));
}

#[test]
fn test_compatible_type_with_optional() {
    let optional_int = Type::Optional(Box::new(Type::Int));
    assert!(Type::Int.is_compatible_with(&optional_int));
    assert!(optional_int.is_compatible_with(&Type::Int));
}

#[test]
fn test_compatible_arrays() {
    let arr1 = Type::Array(Box::new(Type::Int));
    let arr2 = Type::Array(Box::new(Type::Int));
    let arr3 = Type::Array(Box::new(Type::String));

    assert!(arr1.is_compatible_with(&arr2));
    assert!(!arr1.is_compatible_with(&arr3));
}

#[test]
fn test_compatible_maps() {
    let map1 = Type::Map(Box::new(Type::String), Box::new(Type::Int));
    let map2 = Type::Map(Box::new(Type::String), Box::new(Type::Int));
    let map3 = Type::Map(Box::new(Type::String), Box::new(Type::Float));

    assert!(map1.is_compatible_with(&map2));
    // Int compatible with Float for map values
    assert!(map1.is_compatible_with(&map3));
}

#[test]
fn test_compatible_functions() {
    let fn1 = Type::Function {
        params: vec![Type::Int],
        return_type: Box::new(Type::Bool),
    };
    let fn2 = Type::Function {
        params: vec![Type::Int],
        return_type: Box::new(Type::Bool),
    };
    let fn3 = Type::Function {
        params: vec![Type::String],
        return_type: Box::new(Type::Bool),
    };

    assert!(fn1.is_compatible_with(&fn2));
    assert!(!fn1.is_compatible_with(&fn3));
}

#[test]
fn test_incompatible_types() {
    assert!(!Type::Int.is_compatible_with(&Type::String));
    assert!(!Type::Bool.is_compatible_with(&Type::Int));
    assert!(!Type::String.is_compatible_with(&Type::Float));
}

// =============================================================================
// Binary Operation Result Type Tests
// =============================================================================

#[test]
fn test_binary_add_int() {
    let result = Type::Int.binary_result_type("+", &Type::Int);
    assert_eq!(result, Some(Type::Int));
}

#[test]
fn test_binary_sub_int() {
    let result = Type::Int.binary_result_type("-", &Type::Int);
    assert_eq!(result, Some(Type::Int));
}

#[test]
fn test_binary_mul_int() {
    let result = Type::Int.binary_result_type("*", &Type::Int);
    assert_eq!(result, Some(Type::Int));
}

#[test]
fn test_binary_div_int() {
    let result = Type::Int.binary_result_type("/", &Type::Int);
    assert_eq!(result, Some(Type::Int));
}

#[test]
fn test_binary_mod_int() {
    let result = Type::Int.binary_result_type("%", &Type::Int);
    assert_eq!(result, Some(Type::Int));
}

#[test]
fn test_binary_add_float() {
    let result = Type::Float.binary_result_type("+", &Type::Float);
    assert_eq!(result, Some(Type::Float));
}

#[test]
fn test_binary_mixed_int_float() {
    assert_eq!(
        Type::Int.binary_result_type("+", &Type::Float),
        Some(Type::Float)
    );
    assert_eq!(
        Type::Float.binary_result_type("+", &Type::Int),
        Some(Type::Float)
    );
    assert_eq!(
        Type::Int.binary_result_type("-", &Type::Float),
        Some(Type::Float)
    );
    assert_eq!(
        Type::Int.binary_result_type("*", &Type::Float),
        Some(Type::Float)
    );
    assert_eq!(
        Type::Int.binary_result_type("/", &Type::Float),
        Some(Type::Float)
    );
}

#[test]
fn test_binary_power_int() {
    let result = Type::Int.binary_result_type("**", &Type::Int);
    assert_eq!(result, Some(Type::Int));
}

#[test]
fn test_binary_power_float() {
    assert_eq!(
        Type::Float.binary_result_type("**", &Type::Int),
        Some(Type::Float)
    );
    assert_eq!(
        Type::Int.binary_result_type("**", &Type::Float),
        Some(Type::Float)
    );
}

#[test]
fn test_binary_string_concat() {
    let result = Type::String.binary_result_type("+", &Type::String);
    assert_eq!(result, Some(Type::String));
}

#[test]
fn test_binary_string_concat_coercion() {
    assert_eq!(
        Type::String.binary_result_type("+", &Type::Int),
        Some(Type::String)
    );
    assert_eq!(
        Type::String.binary_result_type("+", &Type::Float),
        Some(Type::String)
    );
    assert_eq!(
        Type::String.binary_result_type("+", &Type::Bool),
        Some(Type::String)
    );
    assert_eq!(
        Type::Int.binary_result_type("+", &Type::String),
        Some(Type::String)
    );
    assert_eq!(
        Type::Float.binary_result_type("+", &Type::String),
        Some(Type::String)
    );
    assert_eq!(
        Type::Bool.binary_result_type("+", &Type::String),
        Some(Type::String)
    );
}

#[test]
fn test_binary_comparison_int() {
    assert_eq!(
        Type::Int.binary_result_type("<", &Type::Int),
        Some(Type::Bool)
    );
    assert_eq!(
        Type::Int.binary_result_type("<=", &Type::Int),
        Some(Type::Bool)
    );
    assert_eq!(
        Type::Int.binary_result_type(">", &Type::Int),
        Some(Type::Bool)
    );
    assert_eq!(
        Type::Int.binary_result_type(">=", &Type::Int),
        Some(Type::Bool)
    );
}

#[test]
fn test_binary_comparison_float() {
    assert_eq!(
        Type::Float.binary_result_type("<", &Type::Float),
        Some(Type::Bool)
    );
    assert_eq!(
        Type::Float.binary_result_type("<=", &Type::Float),
        Some(Type::Bool)
    );
    assert_eq!(
        Type::Float.binary_result_type(">", &Type::Float),
        Some(Type::Bool)
    );
    assert_eq!(
        Type::Float.binary_result_type(">=", &Type::Float),
        Some(Type::Bool)
    );
}

#[test]
fn test_binary_comparison_string() {
    assert_eq!(
        Type::String.binary_result_type("<", &Type::String),
        Some(Type::Bool)
    );
    assert_eq!(
        Type::String.binary_result_type("<=", &Type::String),
        Some(Type::Bool)
    );
    assert_eq!(
        Type::String.binary_result_type(">", &Type::String),
        Some(Type::Bool)
    );
    assert_eq!(
        Type::String.binary_result_type(">=", &Type::String),
        Some(Type::Bool)
    );
}

#[test]
fn test_binary_equality() {
    assert_eq!(
        Type::Int.binary_result_type("==", &Type::Int),
        Some(Type::Bool)
    );
    assert_eq!(
        Type::String.binary_result_type("==", &Type::String),
        Some(Type::Bool)
    );
    assert_eq!(
        Type::Bool.binary_result_type("!=", &Type::Bool),
        Some(Type::Bool)
    );
}

#[test]
fn test_binary_logical_and() {
    assert_eq!(
        Type::Bool.binary_result_type("&&", &Type::Bool),
        Some(Type::Bool)
    );
}

#[test]
fn test_binary_logical_or() {
    assert_eq!(
        Type::Bool.binary_result_type("||", &Type::Bool),
        Some(Type::Bool)
    );
}

#[test]
fn test_binary_invalid_operations() {
    assert_eq!(Type::String.binary_result_type("-", &Type::String), None);
    assert_eq!(Type::Bool.binary_result_type("+", &Type::Bool), None);
    assert_eq!(Type::Int.binary_result_type("&&", &Type::Int), None);
}

// =============================================================================
// Unary Operation Result Type Tests
// =============================================================================

#[test]
fn test_unary_neg_int() {
    assert_eq!(Type::Int.unary_result_type("-"), Some(Type::Int));
}

#[test]
fn test_unary_neg_float() {
    assert_eq!(Type::Float.unary_result_type("-"), Some(Type::Float));
}

#[test]
fn test_unary_not_bool() {
    assert_eq!(Type::Bool.unary_result_type("!"), Some(Type::Bool));
}

#[test]
fn test_unary_increment() {
    assert_eq!(Type::Int.unary_result_type("++"), Some(Type::Int));
    assert_eq!(Type::Int.unary_result_type("--"), Some(Type::Int));
}

#[test]
fn test_unary_invalid() {
    assert_eq!(Type::String.unary_result_type("-"), None);
    assert_eq!(Type::Int.unary_result_type("!"), None);
    assert_eq!(Type::Bool.unary_result_type("-"), None);
}

// =============================================================================
// Type Name Parsing Tests
// =============================================================================

#[test]
fn test_parse_type_name_arabic() {
    assert_eq!(parse_type_name("عدد"), Type::Int);
    assert_eq!(parse_type_name("عدد_عشري"), Type::Float);
    assert_eq!(parse_type_name("نص"), Type::String);
    assert_eq!(parse_type_name("منطقي"), Type::Bool);
    assert_eq!(parse_type_name("فراغ"), Type::Void);
    assert_eq!(parse_type_name("عدم"), Type::Null);
    assert_eq!(parse_type_name("أي"), Type::Any);
    assert_eq!(parse_type_name("اي"), Type::Any);
}

#[test]
fn test_parse_type_name_english() {
    assert_eq!(parse_type_name("int"), Type::Int);
    assert_eq!(parse_type_name("float"), Type::Float);
    assert_eq!(parse_type_name("string"), Type::String);
    assert_eq!(parse_type_name("bool"), Type::Bool);
    assert_eq!(parse_type_name("void"), Type::Void);
    assert_eq!(parse_type_name("null"), Type::Null);
    assert_eq!(parse_type_name("none"), Type::Null);
    assert_eq!(parse_type_name("any"), Type::Any);
}

#[test]
fn test_parse_type_name_class() {
    assert_eq!(
        parse_type_name("MyClass"),
        Type::Class("MyClass".to_string())
    );
    assert_eq!(
        parse_type_name("صنف_مخصص"),
        Type::Class("صنف_مخصص".to_string())
    );
}

// =============================================================================
// Type Clone and Equality Tests
// =============================================================================

#[test]
fn test_type_clone() {
    let ty = Type::Array(Box::new(Type::Map(
        Box::new(Type::String),
        Box::new(Type::Int),
    )));
    let cloned = ty.clone();
    assert_eq!(ty, cloned);
}

#[test]
fn test_type_equality() {
    assert_eq!(Type::Int, Type::Int);
    assert_ne!(Type::Int, Type::Float);

    let arr1 = Type::Array(Box::new(Type::Int));
    let arr2 = Type::Array(Box::new(Type::Int));
    let arr3 = Type::Array(Box::new(Type::String));

    assert_eq!(arr1, arr2);
    assert_ne!(arr1, arr3);
}

#[test]
fn test_type_debug() {
    let ty = Type::Int;
    let debug_str = format!("{:?}", ty);
    assert!(debug_str.contains("Int"));
}
