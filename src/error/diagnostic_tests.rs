//! Comprehensive tests for the Diagnostic module
//!
//! These tests verify that diagnostics are created correctly with
//! Arabic-only messages and proper formatting.
//! ترقيم لغة برمجة عربية - جميع الرسائل بالعربية فقط

use super::diagnostic::*;
use super::span::Span;

#[test]
fn test_diagnostic_level_display() {
    assert_eq!(format!("{}", DiagnosticLevel::Error), "خطأ");
    assert_eq!(format!("{}", DiagnosticLevel::Warning), "تحذير");
    assert_eq!(format!("{}", DiagnosticLevel::Info), "معلومة");
    assert_eq!(format!("{}", DiagnosticLevel::Hint), "تلميح");
}

#[test]
fn test_note_creation() {
    let note = Note::new("هذه ملاحظة");
    assert_eq!(note.message, "هذه ملاحظة");
    assert!(note.span.is_none());
}

#[test]
fn test_note_with_span() {
    let span = Span::new(10, 20, 1, 5);
    let note = Note::new("ملاحظة مع موقع").with_span(span);

    assert_eq!(note.message, "ملاحظة مع موقع");
    assert!(note.span.is_some());
    assert_eq!(note.span.unwrap().line, 1);
}

#[test]
fn test_note_arabic_messages() {
    let note = Note::new("المتغير 'x' غير مستخدم");
    assert!(note.message.contains("غير مستخدم"));
}

#[test]
fn test_suggestion_creation() {
    let span = Span::new(0, 5, 1, 1);
    let suggestion = Suggestion::new("استخدم 'ثابت' بدلاً من ذلك", "ثابت", span);

    assert_eq!(suggestion.message, "استخدم 'ثابت' بدلاً من ذلك");
    assert_eq!(suggestion.replacement, "ثابت");
    assert_eq!(suggestion.span.start, 0);
    assert_eq!(suggestion.span.end, 5);
}

#[test]
fn test_suggestion_with_code_replacement() {
    let span = Span::new(10, 20, 2, 5);
    let suggestion = Suggestion::new("أضف الفاصلة المنقوطة المفقودة", ";", span);

    assert_eq!(suggestion.replacement, ";");
}

#[test]
fn test_diagnostic_error_creation() {
    let span = Span::new(0, 10, 1, 1);
    let diag = Diagnostic::error("متغير غير معرف 'x'", span);

    assert_eq!(diag.level, DiagnosticLevel::Error);
    assert_eq!(diag.message, "متغير غير معرف 'x'");
    assert_eq!(diag.span.line, 1);
    assert!(diag.notes.is_empty());
    assert!(diag.suggestions.is_empty());
    assert!(diag.code.is_none());
}

#[test]
fn test_diagnostic_warning_creation() {
    let span = Span::new(5, 15, 2, 3);
    let diag = Diagnostic::warning("متغير غير مستخدم 'y'", span);

    assert_eq!(diag.level, DiagnosticLevel::Warning);
    assert_eq!(diag.message, "متغير غير مستخدم 'y'");
}

#[test]
fn test_diagnostic_with_note() {
    let span = Span::new(0, 10, 1, 1);
    let note = Note::new("تم تعريفه سابقاً هنا");

    let diag = Diagnostic::error("تعريف مكرر", span).with_note(note);

    assert_eq!(diag.notes.len(), 1);
    assert_eq!(diag.notes[0].message, "تم تعريفه سابقاً هنا");
}

#[test]
fn test_diagnostic_with_multiple_notes() {
    let span = Span::new(0, 10, 1, 1);

    let diag = Diagnostic::error("رسالة الخطأ", span)
        .with_note(Note::new("الملاحظة الأولى"))
        .with_note(Note::new("الملاحظة الثانية"))
        .with_note(Note::new("الملاحظة الثالثة"));

    assert_eq!(diag.notes.len(), 3);
}

#[test]
fn test_diagnostic_with_suggestion() {
    let span = Span::new(0, 10, 1, 1);
    let suggestion = Suggestion::new("هل تقصد 'متغير'؟", "متغير", span);

    let diag =
        Diagnostic::error("كلمة مفتاحية غير معروفة 'متغبر'", span).with_suggestion(suggestion);

    assert_eq!(diag.suggestions.len(), 1);
    assert_eq!(diag.suggestions[0].replacement, "متغير");
}

#[test]
fn test_diagnostic_with_code() {
    let span = Span::new(0, 10, 1, 1);
    let diag = Diagnostic::error("عدم تطابق الأنماط", span).with_code("E0308");

    assert_eq!(diag.code, Some("E0308".to_string()));
}

#[test]
fn test_diagnostic_chained_builders() {
    let span = Span::new(0, 10, 1, 1);
    let note_span = Span::new(20, 30, 3, 1);

    let diag = Diagnostic::error("لا يمكن تعيين قيمة لمتغير ثابت", span)
        .with_code("E0384")
        .with_note(Note::new("تم إعلان المتغير هنا").with_span(note_span))
        .with_suggestion(Suggestion::new("اجعل المتغير قابلاً للتعديل", "متغير", span));

    assert!(diag.code.is_some());
    assert_eq!(diag.notes.len(), 1);
    assert_eq!(diag.suggestions.len(), 1);
}

#[test]
fn test_diagnostic_display() {
    let span = Span::new(0, 5, 1, 1);
    let diag = Diagnostic::error("خطأ اختباري", span);

    let display = format!("{}", diag);
    assert!(display.contains("خطأ"));
    assert!(display.contains("خطأ اختباري"));
}

#[test]
fn test_arabic_type_error() {
    let span = Span::new(0, 10, 5, 10);
    let diag = Diagnostic::error("متوقع 'عدد' ولكن وُجد 'نص'", span);

    assert!(diag.message.contains("متوقع"));
    assert!(diag.message.contains("عدد"));
    assert!(diag.message.contains("نص"));
}

#[test]
fn test_arabic_syntax_error() {
    let span = Span::new(15, 20, 3, 5);
    let diag = Diagnostic::error("قوس إغلاق مفقود ')'", span);

    assert!(diag.message.contains("مفقود"));
}

#[test]
fn test_arabic_undefined_variable() {
    let span = Span::new(0, 5, 1, 1);
    let diag = Diagnostic::error("متغير غير معرف 'اسم'", span);

    assert!(diag.message.contains("اسم"));
    assert!(diag.message.contains("غير معرف"));
}

#[test]
fn test_common_error_codes() {
    let span = Span::new(0, 5, 1, 1);

    let type_error = Diagnostic::error("عدم تطابق الأنماط", span).with_code("E0308");
    assert_eq!(type_error.code, Some("E0308".to_string()));

    let undef_error = Diagnostic::error("غير معرف", span).with_code("E0425");
    assert_eq!(undef_error.code, Some("E0425".to_string()));

    let parse_error = Diagnostic::error("خطأ تحليل", span).with_code("E0001");
    assert_eq!(parse_error.code, Some("E0001".to_string()));
}

#[test]
fn test_diagnostic_level_equality() {
    assert_eq!(DiagnosticLevel::Error, DiagnosticLevel::Error);
    assert_eq!(DiagnosticLevel::Warning, DiagnosticLevel::Warning);
    assert_ne!(DiagnosticLevel::Error, DiagnosticLevel::Warning);
}

#[test]
fn test_function_signature_mismatch() {
    let call_span = Span::new(100, 120, 10, 5);
    let def_span = Span::new(20, 50, 3, 1);

    let diag = Diagnostic::error("الدالة 'add' تتوقع 2 معاملات ولكن تم توفير 3", call_span)
        .with_code("E0061")
        .with_note(Note::new("الدالة 'add' معرفة هنا").with_span(def_span));

    assert_eq!(diag.level, DiagnosticLevel::Error);
    assert!(diag.code.is_some());
    assert_eq!(diag.notes.len(), 1);
    assert!(diag.notes[0].span.is_some());
}

#[test]
fn test_class_inheritance_error() {
    let span = Span::new(50, 80, 5, 10);

    let diag = Diagnostic::error("الصنف 'طالب' لا يمكن أن يرث من الصنف المختوم 'شخص'", span)
        .with_code("E0578")
        .with_note(Note::new("الصنف 'شخص' مختوم"))
        .with_suggestion(Suggestion::new(
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

    let diag = Diagnostic::warning("المتغير 'count' غير مستخدم أبداً", span)
        .with_code("W0001")
        .with_suggestion(Suggestion::new(
            "أضف شرطة سفلية في البداية لإخفاء التحذير",
            "_count",
            span,
        ));

    assert_eq!(diag.level, DiagnosticLevel::Warning);
    assert_eq!(diag.suggestions[0].replacement, "_count");
}

#[test]
fn test_empty_message_strings() {
    let span = Span::new(0, 0, 1, 1);
    let diag = Diagnostic::error("", span);

    assert!(diag.message.is_empty());
}

#[test]
fn test_very_long_message() {
    let span = Span::new(0, 10, 1, 1);
    let long = "أ".repeat(1000);

    let diag = Diagnostic::error(&long, span);

    // Arabic characters are multi-byte in UTF-8
    assert!(diag.message.chars().count() >= 1000);
}

#[test]
fn test_message_with_special_characters() {
    let span = Span::new(0, 10, 1, 1);
    let diag = Diagnostic::error("خطأ: حرف غير صالح '؛' (فاصلة منقوطة عربية)", span);

    assert!(diag.message.contains("؛"));
}

#[test]
fn test_message_with_unicode_normalization() {
    let span = Span::new(0, 10, 1, 1);
    let diag = Diagnostic::error("المتغير 'مُتَغَيِّر' غير موجود", span);

    assert!(diag.message.contains("مُتَغَيِّر"));
}

#[test]
fn test_diagnostic_clone() {
    let span = Span::new(0, 10, 1, 1);
    let original = Diagnostic::error("اختبار", span)
        .with_code("E0001")
        .with_note(Note::new("ملاحظة"));

    let cloned = original.clone();

    assert_eq!(original.message, cloned.message);
    assert_eq!(original.code, cloned.code);
    assert_eq!(original.notes.len(), cloned.notes.len());
}

#[test]
fn test_diagnostic_debug() {
    let span = Span::new(0, 10, 1, 1);
    let diag = Diagnostic::error("اختبار", span);

    let debug_str = format!("{:?}", diag);
    assert!(debug_str.contains("Error"));
}

#[test]
fn test_note_clone() {
    let note = Note::new("أصلي");
    let cloned = note.clone();

    assert_eq!(note.message, cloned.message);
}

#[test]
fn test_suggestion_clone() {
    let span = Span::new(0, 5, 1, 1);
    let suggestion = Suggestion::new("إصلاح", "fixed", span);
    let cloned = suggestion.clone();

    assert_eq!(suggestion.replacement, cloned.replacement);
    assert_eq!(suggestion.span.start, cloned.span.start);
}
