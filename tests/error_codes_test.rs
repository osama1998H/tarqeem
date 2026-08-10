//! Integration tests for the Arabic error code system
//!
//! Phase 10: Final verification that all error codes are properly formatted
//! and the error code system is complete.

use tarqeem::error::codes::*;
use tarqeem::error::{ErrorCategory, ErrorCode};

/// All defined error codes in the system
const ALL_ERROR_CODES: &[(ErrorCode, &str, &str)] = &[
    // Lexer errors (ق)
    (ERR_UNKNOWN_CHARACTER, "ق٠٠٠١", "Unknown character"),
    (ERR_UNEXPECTED_EOF, "ق٠٠٠٢", "Unexpected EOF"),
    (ERR_INVALID_NUMBER_FORMAT, "ق٠١٠١", "Invalid number format"),
    (ERR_UNCLOSED_STRING, "ق٠٢٠١", "Unclosed string"),
    (ERR_UNCLOSED_COMMENT, "ق٠٣٠١", "Unclosed comment"),
    // Parser errors (ب)
    (ERR_UNEXPECTED_EXPRESSION, "ب٠٠٠١", "Unexpected expression"),
    (ERR_UNEXPECTED_TOKEN, "ب٠٠٠٢", "Unexpected token"),
    (ERR_EXPECTED_SEMICOLON, "ب٠١٠١", "Expected semicolon"),
    (
        ERR_EXPECTED_VARIABLE_NAME,
        "ب٠٢٠١",
        "Expected variable name",
    ),
    (
        ERR_EXPECTED_FUNCTION_NAME,
        "ب٠٣٠١",
        "Expected function name",
    ),
    (ERR_EXPECTED_CLASS_NAME, "ب٠٤٠١", "Expected class name"),
    // Semantic errors (د)
    (ERR_UNDEFINED_VARIABLE, "د٠٠٠١", "Undefined variable"),
    (ERR_UNDEFINED_FUNCTION, "د٠٠٠٢", "Undefined function"),
    (ERR_UNDEFINED_CLASS, "د٠٠٠٣", "Undefined class"),
    (ERR_VARIABLE_REDEFINITION, "د٠١٠١", "Variable redefinition"),
    (ERR_CONST_ASSIGNMENT, "د٠١٠٢", "Assignment to constant"),
    (ERR_BREAK_OUTSIDE_LOOP, "د٠٣٠١", "Break outside loop"),
    (ERR_CONTINUE_OUTSIDE_LOOP, "د٠٣٠٢", "Continue outside loop"),
    (
        ERR_RETURN_OUTSIDE_FUNCTION,
        "د٠٣٠٣",
        "Return outside function",
    ),
    (ERR_THIS_OUTSIDE_CLASS, "د٠٣٠٤", "This outside class"),
    (ERR_SUPER_OUTSIDE_CLASS, "د٠٣٠٥", "Super outside class"),
    (
        ERR_LAMBDA_CAPTURE,
        "د٠٣٠٦",
        "Lambda captures outer variable",
    ),
    // Type errors (ن)
    (ERR_TYPE_MISMATCH, "ن٠٠٠١", "Type mismatch"),
    (ERR_CANNOT_INFER_TYPE, "ن٠٠٠٢", "Cannot infer type"),
    (ERR_INVALID_CONVERSION, "ن٠١٠١", "Invalid conversion"),
    (ERR_UNSPECIFIED_GENERIC, "ن٠٢٠١", "Unspecified generic"),
    // Class errors (ص)
    (ERR_CLASS_NOT_FOUND, "ص٠٠٠١", "Class not found"),
    (ERR_CIRCULAR_INHERITANCE, "ص٠١٠١", "Circular inheritance"),
    (
        ERR_INTERFACE_NOT_IMPLEMENTED,
        "ص٠٢٠١",
        "Interface not implemented",
    ),
    (ERR_PROPERTY_NOT_FOUND, "ص٠٣٠١", "Property not found"),
    (ERR_METHOD_NOT_FOUND, "ص٠٣٠٢", "Method not found"),
    (ERR_PRIVATE_ACCESS, "ص٠٤٠١", "Private access"),
    (ERR_PROTECTED_ACCESS, "ص٠٤٠٢", "Protected access"),
    (
        ERR_NONSTATIC_VIA_CLASS,
        "ص٠٥٠١",
        "Non-static member accessed via class name",
    ),
    (
        ERR_STATIC_VIA_INSTANCE,
        "ص٠٥٠٢",
        "Static member accessed via instance",
    ),
    (
        ERR_THROW_NON_EXCEPTION,
        "ص٠٦٠١",
        "Thrown value is not an exception",
    ),
    (
        ERR_REDEFINE_PRELUDE_CLASS,
        "ص٠٦٠٢",
        "Redefinition of the base exception class",
    ),
    // Module errors (و)
    (ERR_MODULE_NOT_FOUND, "و٠٠٠١", "Module not found"),
    (ERR_NOT_EXPORTED, "و٠٠٠٢", "Not exported"),
    (ERR_DUPLICATE_EXPORT, "و٠١٠١", "Duplicate export"),
    (ERR_CIRCULAR_DEPENDENCY, "و٠٣٠١", "Circular dependency"),
    // Codegen errors (ت)
    (ERR_LLVM_INTERNAL, "ت٠٠٠١", "LLVM internal error"),
    (ERR_LINKING_FAILED, "ت٠١٠١", "Linking failed"),
    (ERR_ENTRY_POINT_CONFLICT, "ت٠٢٠١", "Entry point conflict"),
    (
        ERR_UNTYPED_LAMBDA_PARAM,
        "ت٠٣٠١",
        "Types insufficient for native codegen",
    ),
    (
        ERR_UNTYPED_INDIRECT_CALL,
        "ت٠٣٠٢",
        "Indirect call without a known signature in native codegen",
    ),
    (
        ERR_NATIVE_EXCEPTIONS,
        "ت٠٣٠٣",
        "Throwing exceptions is unsupported in native codegen",
    ),
    // Warnings (ح)
    (WARN_UNUSED_VARIABLE, "ح٠٠٠١", "Unused variable"),
    (WARN_UNUSED_FUNCTION, "ح٠٠٠٢", "Unused function"),
    (WARN_UNUSED_IMPORT, "ح٠٠٠٣", "Unused import"),
    // Deprecated (م)
    (WARN_DEPRECATED_KEYWORD, "م٠٠٠١", "Deprecated keyword"),
];

#[test]
fn test_error_code_count() {
    // Phase 10 verification: 43 + ص٠٥٠١/ص٠٥٠٢ (issue #184, static-member access)
    // + د٠٣٠٦/ت٠٣٠١/ت٠٣٠٢ (issue #180: lambda capture, untyped native lambda
    // param, untyped native indirect call)
    // + ص٠٦٠١/ص٠٦٠٢/ت٠٣٠٣ (issue #181: non-exception throw, redefinition of the
    // base exception class, `ارمِ` under native codegen)
    assert_eq!(
        ALL_ERROR_CODES.len(),
        51,
        "Expected 51 error codes, found {}",
        ALL_ERROR_CODES.len()
    );
}

#[test]
fn test_all_error_codes_have_arabic_format() {
    for (code, expected_str, description) in ALL_ERROR_CODES {
        let actual = code.to_string();

        // Verify the string matches expected
        assert_eq!(
            actual, *expected_str,
            "Error code for '{}' should be '{}', got '{}'",
            description, expected_str, actual
        );

        // Verify format: starts with Arabic letter, followed by 4 Arabic-Indic digits
        let first_char = actual.chars().next().unwrap();
        assert!(
            matches!(
                first_char,
                'ق' | 'ب' | 'د' | 'ن' | 'ص' | 'و' | 'ت' | 'ح' | 'م'
            ),
            "Error code '{}' should start with Arabic category letter",
            actual
        );

        // Verify length is 5 characters (1 letter + 4 digits)
        assert_eq!(
            actual.chars().count(),
            5,
            "Error code '{}' should be 5 characters",
            actual
        );
    }
}

#[test]
fn test_error_categories_are_correct() {
    let category_checks: Vec<(ErrorCode, ErrorCategory, char)> = vec![
        (ERR_UNKNOWN_CHARACTER, ErrorCategory::Qiraa, 'ق'),
        (ERR_UNEXPECTED_TOKEN, ErrorCategory::Binaa, 'ب'),
        (ERR_UNDEFINED_VARIABLE, ErrorCategory::Dalala, 'د'),
        (ERR_TYPE_MISMATCH, ErrorCategory::Naw, 'ن'),
        (ERR_CLASS_NOT_FOUND, ErrorCategory::Sinf, 'ص'),
        (ERR_MODULE_NOT_FOUND, ErrorCategory::Wahda, 'و'),
        (ERR_LLVM_INTERNAL, ErrorCategory::Tawleed, 'ت'),
        (WARN_UNUSED_VARIABLE, ErrorCategory::Tahdheer, 'ح'),
        (WARN_DEPRECATED_KEYWORD, ErrorCategory::Muhmal, 'م'),
    ];

    for (code, expected_category, expected_char) in category_checks {
        assert_eq!(
            code.category, expected_category,
            "Error code {} should have category {:?}",
            code, expected_category
        );
        assert_eq!(
            code.category.arabic_char(),
            expected_char,
            "Category {:?} should have Arabic char '{}'",
            expected_category,
            expected_char
        );
    }
}

#[test]
fn test_error_code_parsing() {
    // Test parsing Arabic error codes
    let test_cases = vec![
        ("د٠٣٠١", ErrorCategory::Dalala, 301),
        ("ق٠٠٠١", ErrorCategory::Qiraa, 1),
        ("ح٠٠٠١", ErrorCategory::Tahdheer, 1),
        ("ص٠٤٠٢", ErrorCategory::Sinf, 402),
    ];

    for (code_str, expected_category, expected_number) in test_cases {
        let parsed = ErrorCode::from_arabic_string(code_str);
        assert!(parsed.is_some(), "Failed to parse error code: {}", code_str);

        let parsed = parsed.unwrap();
        assert_eq!(parsed.category, expected_category);
        assert_eq!(parsed.number, expected_number);
    }
}

#[test]
fn test_arabic_numeral_conversion() {
    // Test Arabic-Indic numeral conversion
    assert_eq!(to_arabic_numerals(1), "٠٠٠١");
    assert_eq!(to_arabic_numerals(301), "٠٣٠١");
    assert_eq!(to_arabic_numerals(9999), "٩٩٩٩");

    // Test parsing
    assert_eq!(from_arabic_numerals("٠٠٠١"), Some(1));
    assert_eq!(from_arabic_numerals("٠٣٠١"), Some(301));
    assert_eq!(from_arabic_numerals("٩٩٩٩"), Some(9999));
}

#[test]
fn test_all_categories_have_codes() {
    // Verify each category has at least one error code
    let categories = vec![
        ErrorCategory::Qiraa,
        ErrorCategory::Binaa,
        ErrorCategory::Dalala,
        ErrorCategory::Naw,
        ErrorCategory::Sinf,
        ErrorCategory::Wahda,
        ErrorCategory::Tawleed,
        ErrorCategory::Tahdheer,
        ErrorCategory::Muhmal,
    ];

    for category in categories {
        let has_code = ALL_ERROR_CODES
            .iter()
            .any(|(code, _, _)| code.category == category);
        assert!(
            has_code,
            "Category {:?} should have at least one error code",
            category
        );
    }
}
