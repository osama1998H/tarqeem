//! Hover information handler
//!
//! Provides type information and documentation when hovering over symbols.

use crate::error::Language;
use crate::lsp::analysis::{DocumentState, SymbolKind};
use crate::lsp::utils::{find_word_at_position, position_to_offset, span_to_range};
use crate::semantic::Type;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

/// Handle hover request
pub fn handle_hover(
    doc: &mut DocumentState,
    position: Position,
    language: Language,
) -> Option<Hover> {
    let _offset = position_to_offset(&doc.content, position);
    let content = doc.content.clone();

    // Find the word at the position
    let (start, end, word) = find_word_at_position(&content, position)?;

    let analysis = doc.get_analysis(language);

    // Look up the symbol
    let symbol_info = analysis.symbols.get(&word)?;

    // Format the hover content
    let (type_info, kind_label) = format_type_info(&symbol_info.ty, &symbol_info.kind, language);

    // Build markdown content
    let markdown = format!(
        "```tarqeem\n{}: {}\n```\n\n**{}**{}",
        word,
        type_info,
        kind_label,
        symbol_info
            .doc
            .as_ref()
            .map(|d| format!("\n\n{}", d))
            .unwrap_or_default()
    );

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: Some(span_to_range(
            &content,
            &crate::error::Span::new(start, end, 1, 1),
        )),
    })
}

/// Format type information for display
fn format_type_info(ty: &Type, kind: &SymbolKind, language: Language) -> (String, String) {
    let type_str = match language {
        Language::Arabic => ty.arabic_name(),
        Language::English => ty.to_string(),
    };

    let kind_label = match (kind, language) {
        (SymbolKind::Variable, Language::Arabic) => "متغير",
        (SymbolKind::Variable, Language::English) => "Variable",
        (SymbolKind::Function, Language::Arabic) => "دالة",
        (SymbolKind::Function, Language::English) => "Function",
        (SymbolKind::Class, Language::Arabic) => "صنف",
        (SymbolKind::Class, Language::English) => "Class",
        (SymbolKind::Interface, Language::Arabic) => "واجهة",
        (SymbolKind::Interface, Language::English) => "Interface",
        (SymbolKind::Parameter, Language::Arabic) => "معامل",
        (SymbolKind::Parameter, Language::English) => "Parameter",
        (SymbolKind::Field, Language::Arabic) => "حقل",
        (SymbolKind::Field, Language::English) => "Field",
        (SymbolKind::Method, Language::Arabic) => "دالة",
        (SymbolKind::Method, Language::English) => "Method",
    };

    (type_str, kind_label.to_string())
}

/// Get hover information for built-in functions
pub fn get_builtin_hover(name: &str, language: Language) -> Option<Hover> {
    let (signature, description_ar, description_en) = match name {
        // I/O Functions
        "اطبع" | "print" => (
            "دالة اطبع(قيمة: أي) -> فراغ",
            "طباعة قيمة إلى المخرج القياسي",
            "Print a value to standard output",
        ),
        "println" | "اطبع_سطر" => (
            "دالة اطبع_سطر(قيمة: أي) -> فراغ",
            "طباعة قيمة مع سطر جديد",
            "Print a value with a newline",
        ),
        "ادخل" | "input" => (
            "دالة ادخل() -> نص",
            "قراءة سطر من المدخل القياسي",
            "Read a line from standard input",
        ),

        // Introspection
        "طول" | "len" | "length" => (
            "دالة طول(قيمة: أي) -> عدد",
            "الحصول على طول المصفوفة أو النص",
            "Get the length of an array or string",
        ),
        "نوع" | "type" | "typeof" => (
            "دالة نوع(قيمة: أي) -> نص",
            "الحصول على نوع القيمة كنص",
            "Get the type of a value as a string",
        ),

        // Type conversion
        "عدد" | "int" => (
            "دالة عدد(قيمة: أي) -> عدد",
            "تحويل قيمة إلى عدد صحيح",
            "Convert a value to an integer",
        ),
        "عدد_عشري" | "float" => (
            "دالة عدد_عشري(قيمة: أي) -> عدد_عشري",
            "تحويل قيمة إلى عدد عشري",
            "Convert a value to a float",
        ),
        "نص" | "str" | "string" => (
            "دالة نص(قيمة: أي) -> نص",
            "تحويل قيمة إلى نص",
            "Convert a value to a string",
        ),
        "منطقي" | "bool" => (
            "دالة منطقي(قيمة: أي) -> منطقي",
            "تحويل قيمة إلى منطقي",
            "Convert a value to a boolean",
        ),

        // Math
        "مطلق" | "abs" => (
            "دالة مطلق(قيمة: عدد) -> عدد",
            "الحصول على القيمة المطلقة",
            "Get the absolute value",
        ),
        "جذر" | "sqrt" => (
            "دالة جذر(قيمة: عدد_عشري) -> عدد_عشري",
            "حساب الجذر التربيعي",
            "Calculate the square root",
        ),
        "قوة" | "pow" => (
            "دالة قوة(أساس: عدد_عشري، أس: عدد_عشري) -> عدد_عشري",
            "رفع عدد لقوة معينة",
            "Raise a number to a power",
        ),

        // File operations
        "اقرأ_ملف" | "read_file" => (
            "دالة اقرأ_ملف(مسار: نص) -> نص",
            "قراءة محتوى ملف",
            "Read the contents of a file",
        ),
        "اكتب_ملف" | "write_file" => (
            "دالة اكتب_ملف(مسار: نص، محتوى: نص) -> منطقي",
            "كتابة محتوى إلى ملف",
            "Write contents to a file",
        ),

        _ => return None,
    };

    let description = match language {
        Language::Arabic => description_ar,
        Language::English => description_en,
    };

    let markdown = format!("```tarqeem\n{}\n```\n\n{}", signature, description);

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_type_info() {
        let (type_str, kind) =
            format_type_info(&Type::Int, &SymbolKind::Variable, Language::Arabic);
        assert_eq!(type_str, "عدد");
        assert_eq!(kind, "متغير");

        let (type_str, kind) =
            format_type_info(&Type::Int, &SymbolKind::Variable, Language::English);
        assert_eq!(type_str, "int");
        assert_eq!(kind, "Variable");
    }

    #[test]
    fn test_builtin_hover() {
        let hover = get_builtin_hover("اطبع", Language::Arabic);
        assert!(hover.is_some());
        let hover = hover.unwrap();
        if let HoverContents::Markup(markup) = hover.contents {
            assert!(markup.value.contains("طباعة"));
        }

        let hover_en = get_builtin_hover("print", Language::English);
        assert!(hover_en.is_some());
    }
}
