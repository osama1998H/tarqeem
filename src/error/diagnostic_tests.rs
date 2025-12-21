//! Comprehensive tests for the Diagnostic module
//!
//! These tests verify that diagnostics are created correctly with
//! bilingual messages and proper formatting.

use super::diagnostic::*;
use super::span::Span;
use super::Language;

// =============================================================================
// DiagnosticLevel Tests
// =============================================================================

#[test]
fn test_diagnostic_level_error_english() {
    let level = DiagnosticLevel::Error;
    assert_eq!(level.english(), "error");
}

#[test]
fn test_diagnostic_level_error_arabic() {
    let level = DiagnosticLevel::Error;
    assert_eq!(level.arabic(), "خطأ");
}

#[test]
fn test_diagnostic_level_warning_english() {
    let level = DiagnosticLevel::Warning;
    assert_eq!(level.english(), "warning");
}

#[test]
fn test_diagnostic_level_warning_arabic() {
    let level = DiagnosticLevel::Warning;
    assert_eq!(level.arabic(), "تحذير");
}

#[test]
fn test_diagnostic_level_info_english() {
    let level = DiagnosticLevel::Info;
    assert_eq!(level.english(), "info");
}

#[test]
fn test_diagnostic_level_info_arabic() {
    let level = DiagnosticLevel::Info;
    assert_eq!(level.arabic(), "معلومة");
}

#[test]
fn test_diagnostic_level_hint_english() {
    let level = DiagnosticLevel::Hint;
    assert_eq!(level.english(), "hint");
}

#[test]
fn test_diagnostic_level_hint_arabic() {
    let level = DiagnosticLevel::Hint;
    assert_eq!(level.arabic(), "تلميح");
}

// =============================================================================
// Note Tests
// =============================================================================

#[test]
fn test_note_creation() {
    let note = Note::new("This is a note", "هذه ملاحظة");
    assert_eq!(note.message, "This is a note");
    assert_eq!(note.message_ar, "هذه ملاحظة");
    assert!(note.span.is_none());
}

#[test]
fn test_note_with_span() {
    let span = Span::new(10, 20, 1, 5);
    let note = Note::new("Note with span", "ملاحظة مع موقع").with_span(span);

    assert_eq!(note.message, "Note with span");
    assert!(note.span.is_some());
    assert_eq!(note.span.unwrap().line, 1);
}

#[test]
fn test_note_bilingual_messages() {
    let note = Note::new(
        "Variable 'x' is not used",
        "المتغير 'x' غير مستخدم",
    );

    assert!(note.message.contains("not used"));
    assert!(note.message_ar.contains("غير مستخدم"));
}

// =============================================================================
// Suggestion Tests
// =============================================================================

#[test]
fn test_suggestion_creation() {
    let span = Span::new(0, 5, 1, 1);
    let suggestion = Suggestion::new(
        "Consider using 'ثابت' instead",
        "استخدم 'ثابت' بدلاً من ذلك",
        "ثابت",
        span,
    );

    assert_eq!(suggestion.message, "Consider using 'ثابت' instead");
    assert_eq!(suggestion.message_ar, "استخدم 'ثابت' بدلاً من ذلك");
    assert_eq!(suggestion.replacement, "ثابت");
    assert_eq!(suggestion.span.start, 0);
    assert_eq!(suggestion.span.end, 5);
}

#[test]
fn test_suggestion_with_code_replacement() {
    let span = Span::new(10, 20, 2, 5);
    let suggestion = Suggestion::new(
        "Add missing semicolon",
        "أضف الفاصلة المنقوطة المفقودة",
        ";",
        span,
    );

    assert_eq!(suggestion.replacement, ";");
}

// =============================================================================
// Diagnostic Creation Tests
// =============================================================================

#[test]
fn test_diagnostic_error_creation() {
    let span = Span::new(0, 10, 1, 1);
    let diag = Diagnostic::error(
        "Undefined variable 'x'",
        "متغير غير معرف 'x'",
        span,
    );

    assert_eq!(diag.level, DiagnosticLevel::Error);
    assert_eq!(diag.message, "Undefined variable 'x'");
    assert_eq!(diag.message_ar, "متغير غير معرف 'x'");
    assert_eq!(diag.span.line, 1);
    assert!(diag.notes.is_empty());
    assert!(diag.suggestions.is_empty());
    assert!(diag.code.is_none());
}

#[test]
fn test_diagnostic_warning_creation() {
    let span = Span::new(5, 15, 2, 3);
    let diag = Diagnostic::warning(
        "Unused variable 'y'",
        "متغير غير مستخدم 'y'",
        span,
    );

    assert_eq!(diag.level, DiagnosticLevel::Warning);
    assert_eq!(diag.message, "Unused variable 'y'");
}

#[test]
fn test_diagnostic_with_note() {
    let span = Span::new(0, 10, 1, 1);
    let note = Note::new("Previously defined here", "تم تعريفه سابقاً هنا");

    let diag = Diagnostic::error(
        "Duplicate definition",
        "تعريف مكرر",
        span,
    ).with_note(note);

    assert_eq!(diag.notes.len(), 1);
    assert_eq!(diag.notes[0].message, "Previously defined here");
}

#[test]
fn test_diagnostic_with_multiple_notes() {
    let span = Span::new(0, 10, 1, 1);

    let diag = Diagnostic::error("Error message", "رسالة الخطأ", span)
        .with_note(Note::new("First note", "الملاحظة الأولى"))
        .with_note(Note::new("Second note", "الملاحظة الثانية"))
        .with_note(Note::new("Third note", "الملاحظة الثالثة"));

    assert_eq!(diag.notes.len(), 3);
}

#[test]
fn test_diagnostic_with_suggestion() {
    let span = Span::new(0, 10, 1, 1);
    let suggestion = Suggestion::new(
        "Did you mean 'متغير'?",
        "هل تقصد 'متغير'؟",
        "متغير",
        span,
    );

    let diag = Diagnostic::error(
        "Unknown keyword 'متغبر'",
        "كلمة مفتاحية غير معروفة 'متغبر'",
        span,
    ).with_suggestion(suggestion);

    assert_eq!(diag.suggestions.len(), 1);
    assert_eq!(diag.suggestions[0].replacement, "متغير");
}

#[test]
fn test_diagnostic_with_code() {
    let span = Span::new(0, 10, 1, 1);
    let diag = Diagnostic::error("Type mismatch", "عدم تطابق الأنماط", span)
        .with_code("E0308");

    assert_eq!(diag.code, Some("E0308".to_string()));
}

#[test]
fn test_diagnostic_chained_builders() {
    let span = Span::new(0, 10, 1, 1);
    let note_span = Span::new(20, 30, 3, 1);

    let diag = Diagnostic::error(
        "Cannot assign to immutable variable",
        "لا يمكن تعيين قيمة لمتغير ثابت",
        span,
    )
    .with_code("E0384")
    .with_note(Note::new("Variable is declared here", "تم إعلان المتغير هنا").with_span(note_span))
    .with_suggestion(Suggestion::new(
        "Make the variable mutable",
        "اجعل المتغير قابلاً للتعديل",
        "متغير",
        span,
    ));

    assert!(diag.code.is_some());
    assert_eq!(diag.notes.len(), 1);
    assert_eq!(diag.suggestions.len(), 1);
}

// =============================================================================
// Diagnostic Display Tests
// =============================================================================

#[test]
fn test_diagnostic_display() {
    let span = Span::new(0, 5, 1, 1);
    let diag = Diagnostic::error(
        "Test error",
        "خطأ اختباري",
        span,
    );

    let display = format!("{}", diag);
    assert!(display.contains("error"));
    assert!(display.contains("Test error"));
    assert!(display.contains("خطأ"));
    assert!(display.contains("خطأ اختباري"));
}

// =============================================================================
// Bilingual Message Tests
// =============================================================================

#[test]
fn test_bilingual_type_error() {
    let span = Span::new(0, 10, 5, 10);
    let diag = Diagnostic::error(
        "Expected 'عدد' but found 'نص'",
        "متوقع 'عدد' ولكن وُجد 'نص'",
        span,
    );

    assert!(diag.message.contains("Expected"));
    assert!(diag.message_ar.contains("متوقع"));
}

#[test]
fn test_bilingual_syntax_error() {
    let span = Span::new(15, 20, 3, 5);
    let diag = Diagnostic::error(
        "Missing closing parenthesis ')'",
        "قوس إغلاق مفقود ')'",
        span,
    );

    assert!(diag.message.contains("Missing"));
    assert!(diag.message_ar.contains("مفقود"));
}

#[test]
fn test_bilingual_undefined_variable() {
    let span = Span::new(0, 5, 1, 1);
    let diag = Diagnostic::error(
        "Undefined variable 'اسم'",
        "متغير غير معرف 'اسم'",
        span,
    );

    // Both messages should reference the Arabic variable name
    assert!(diag.message.contains("اسم"));
    assert!(diag.message_ar.contains("اسم"));
}

// =============================================================================
// Error Code Tests
// =============================================================================

#[test]
fn test_common_error_codes() {
    let span = Span::new(0, 5, 1, 1);

    // Type mismatch error
    let type_error = Diagnostic::error("Type mismatch", "عدم تطابق الأنماط", span)
        .with_code("E0308");
    assert_eq!(type_error.code, Some("E0308".to_string()));

    // Undefined variable
    let undef_error = Diagnostic::error("Undefined", "غير معرف", span)
        .with_code("E0425");
    assert_eq!(undef_error.code, Some("E0425".to_string()));

    // Parse error
    let parse_error = Diagnostic::error("Parse error", "خطأ تحليل", span)
        .with_code("E0001");
    assert_eq!(parse_error.code, Some("E0001".to_string()));
}

// =============================================================================
// Diagnostic Equality Tests
// =============================================================================

#[test]
fn test_diagnostic_level_equality() {
    assert_eq!(DiagnosticLevel::Error, DiagnosticLevel::Error);
    assert_eq!(DiagnosticLevel::Warning, DiagnosticLevel::Warning);
    assert_ne!(DiagnosticLevel::Error, DiagnosticLevel::Warning);
}

// =============================================================================
// Complex Diagnostic Scenarios
// =============================================================================

#[test]
fn test_function_signature_mismatch() {
    let call_span = Span::new(100, 120, 10, 5);
    let def_span = Span::new(20, 50, 3, 1);

    let diag = Diagnostic::error(
        "Function 'add' expects 2 arguments but 3 were provided",
        "الدالة 'add' تتوقع 2 معاملات ولكن تم توفير 3",
        call_span,
    )
    .with_code("E0061")
    .with_note(Note::new(
        "Function 'add' is defined here",
        "الدالة 'add' معرفة هنا",
    ).with_span(def_span));

    assert_eq!(diag.level, DiagnosticLevel::Error);
    assert!(diag.code.is_some());
    assert_eq!(diag.notes.len(), 1);
    assert!(diag.notes[0].span.is_some());
}

#[test]
fn test_class_inheritance_error() {
    let span = Span::new(50, 80, 5, 10);

    let diag = Diagnostic::error(
        "Class 'طالب' cannot inherit from sealed class 'شخص'",
        "الصنف 'طالب' لا يمكن أن يرث من الصنف المختوم 'شخص'",
        span,
    )
    .with_code("E0578")
    .with_note(Note::new(
        "Class 'شخص' is sealed",
        "الصنف 'شخص' مختوم",
    ))
    .with_suggestion(Suggestion::new(
        "Consider using composition instead",
        "فكر في استخدام التركيب بدلاً من الوراثة",
        "",
        span,
    ));

    assert_eq!(diag.level, DiagnosticLevel::Error);
    assert_eq!(diag.notes.len(), 1);
    assert_eq!(diag.suggestions.len(), 1);
}

#[test]
fn test_warning_with_suggestions() {
    let span = Span::new(0, 15, 1, 1);

    let diag = Diagnostic::warning(
        "Variable 'count' is never used",
        "المتغير 'count' غير مستخدم أبداً",
        span,
    )
    .with_code("W0001")
    .with_suggestion(Suggestion::new(
        "Prefix with underscore to suppress warning",
        "أضف شرطة سفلية في البداية لإخفاء التحذير",
        "_count",
        span,
    ));

    assert_eq!(diag.level, DiagnosticLevel::Warning);
    assert_eq!(diag.suggestions[0].replacement, "_count");
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn test_empty_message_strings() {
    let span = Span::new(0, 0, 1, 1);
    let diag = Diagnostic::error("", "", span);

    assert!(diag.message.is_empty());
    assert!(diag.message_ar.is_empty());
}

#[test]
fn test_very_long_message() {
    let span = Span::new(0, 10, 1, 1);
    let long_message = "a".repeat(1000);
    let long_message_ar = "أ".repeat(1000);

    let diag = Diagnostic::error(&long_message, &long_message_ar, span);

    assert_eq!(diag.message.len(), 1000);
}

#[test]
fn test_message_with_special_characters() {
    let span = Span::new(0, 10, 1, 1);
    let diag = Diagnostic::error(
        "Error: Invalid character '؛' (Arabic semicolon)",
        "خطأ: حرف غير صالح '؛' (فاصلة منقوطة عربية)",
        span,
    );

    assert!(diag.message.contains("؛"));
    assert!(diag.message_ar.contains("؛"));
}

#[test]
fn test_message_with_unicode_normalization() {
    let span = Span::new(0, 10, 1, 1);
    // Arabic text with potential normalization issues
    let diag = Diagnostic::error(
        "Variable 'مُتَغَيِّر' not found",
        "المتغير 'مُتَغَيِّر' غير موجود",
        span,
    );

    // The message should preserve the diacritics
    assert!(diag.message.contains("مُتَغَيِّر"));
}

// =============================================================================
// Clone and Debug Tests
// =============================================================================

#[test]
fn test_diagnostic_clone() {
    let span = Span::new(0, 10, 1, 1);
    let original = Diagnostic::error("Test", "اختبار", span)
        .with_code("E0001")
        .with_note(Note::new("Note", "ملاحظة"));

    let cloned = original.clone();

    assert_eq!(original.message, cloned.message);
    assert_eq!(original.message_ar, cloned.message_ar);
    assert_eq!(original.code, cloned.code);
    assert_eq!(original.notes.len(), cloned.notes.len());
}

#[test]
fn test_diagnostic_debug() {
    let span = Span::new(0, 10, 1, 1);
    let diag = Diagnostic::error("Test", "اختبار", span);

    let debug_str = format!("{:?}", diag);
    assert!(debug_str.contains("Error"));
    assert!(debug_str.contains("Test"));
}

#[test]
fn test_note_clone() {
    let note = Note::new("Original", "أصلي");
    let cloned = note.clone();

    assert_eq!(note.message, cloned.message);
    assert_eq!(note.message_ar, cloned.message_ar);
}

#[test]
fn test_suggestion_clone() {
    let span = Span::new(0, 5, 1, 1);
    let suggestion = Suggestion::new("Fix", "إصلاح", "fixed", span);
    let cloned = suggestion.clone();

    assert_eq!(suggestion.replacement, cloned.replacement);
    assert_eq!(suggestion.span.start, cloned.span.start);
}
