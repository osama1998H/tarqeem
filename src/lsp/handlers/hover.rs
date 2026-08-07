//! Hover information handler
//!
//! Provides type information and documentation when hovering over symbols.

use crate::error::Language;
use crate::lsp::analysis::{DocumentState, SymbolKind};
use crate::lsp::utils::{find_word_at_position, position_to_offset, span_to_range};
use crate::semantic::Type;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

pub fn handle_hover(
    doc: &mut DocumentState,
    position: Position,
    language: Language,
) -> Option<Hover> {
    let _offset = position_to_offset(&doc.content, position);
    let content = doc.content.clone();

    let (start, end, word) = find_word_at_position(&content, position)?;

    let analysis = doc.get_analysis(language);

    let symbol_info = analysis.symbols.get(&word)?;

    let (type_info, kind_label) = format_type_info(&symbol_info.ty, &symbol_info.kind, language);

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

fn format_type_info(ty: &Type, kind: &SymbolKind, _language: Language) -> (String, String) {
    // Arabic-only: ترقيم لغة برمجة عربية
    let type_str = ty.arabic_name();

    let kind_label = match kind {
        SymbolKind::Variable => "متغير",
        SymbolKind::Function => "دالة",
        SymbolKind::Class => "صنف",
        SymbolKind::Interface => "ميثاق",
        SymbolKind::Parameter => "معامل",
        SymbolKind::Field => "حقل",
        SymbolKind::Method => "دالة",
        SymbolKind::Property => "خاصية",
        SymbolKind::Enum => "تعداد",
        SymbolKind::EnumVariant => "حالة تعداد",
    };

    (type_str, kind_label.to_string())
}

#[allow(dead_code)]
pub fn get_builtin_hover(name: &str, _language: Language) -> Option<Hover> {
    // Arabic-only: ترقيم لغة برمجة عربية
    let (signature, description) = match name {
        "اطبع" => ("دالة اطبع(قيمة: أي)", "طباعة قيمة إلى المخرج القياسي"),
        "اطبع_سطر" => ("دالة اطبع_سطر(قيمة: أي)", "طباعة قيمة مع سطر جديد"),
        "ادخل" => ("دالة ادخل() -> نص", "قراءة سطر من المدخل القياسي"),
        "طول" => (
            "دالة طول(قيمة: أي) -> عدد",
            "الحصول على طول المصفوفة أو النص",
        ),
        "نوع" => ("دالة نوع(قيمة: أي) -> نص", "الحصول على نوع القيمة كنص"),
        "عدد" => ("دالة عدد(قيمة: أي) -> عدد", "تحويل قيمة إلى عدد صحيح"),
        "عدد_عشري" => (
            "دالة عدد_عشري(قيمة: أي) -> عدد_عشري",
            "تحويل قيمة إلى عدد عشري",
        ),
        "نص" => ("دالة نص(قيمة: أي) -> نص", "تحويل قيمة إلى نص"),
        "منطقي" => ("دالة منطقي(قيمة: أي) -> منطقي", "تحويل قيمة إلى منطقي"),
        "مطلق" => ("دالة مطلق(قيمة: عدد) -> عدد", "الحصول على القيمة المطلقة"),
        "جذر" => (
            "دالة جذر(قيمة: عدد_عشري) -> عدد_عشري",
            "حساب الجذر التربيعي",
        ),
        "قوة" => (
            "دالة قوة(أساس: عدد_عشري، أس: عدد_عشري) -> عدد_عشري",
            "رفع عدد لقوة معينة",
        ),
        "اقرأ_ملف" => ("دالة اقرأ_ملف(مسار: نص) -> نص", "قراءة محتوى ملف"),
        "اكتب_ملف" => (
            "دالة اكتب_ملف(مسار: نص، محتوى: نص) -> منطقي",
            "كتابة محتوى إلى ملف",
        ),
        _ => return None,
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
        // Arabic-only: ترقيم لغة برمجة عربية
        let (type_str, kind) =
            format_type_info(&Type::Int, &SymbolKind::Variable, Language::Arabic);
        assert_eq!(type_str, "عدد");
        assert_eq!(kind, "متغير");

        // Even with English language setting, output should be Arabic
        let (type_str, kind) =
            format_type_info(&Type::Int, &SymbolKind::Variable, Language::English);
        assert_eq!(type_str, "عدد");
        assert_eq!(kind, "متغير");
    }

    #[test]
    fn test_builtin_hover() {
        // Arabic-only: ترقيم لغة برمجة عربية
        let hover = get_builtin_hover("اطبع", Language::Arabic);
        assert!(hover.is_some());
        let hover = hover.unwrap();
        if let HoverContents::Markup(markup) = hover.contents {
            assert!(markup.value.contains("طباعة"));
        }

        // English names are no longer supported
        let hover_en = get_builtin_hover("print", Language::Arabic);
        assert!(hover_en.is_none());
    }
}
