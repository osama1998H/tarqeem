//! Auto-completion handler
//!
//! Provides intelligent code completion suggestions.

use crate::error::Language;
use crate::lsp::analysis::{DocumentState, SymbolKind};
use crate::lsp::utils::position_to_offset;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Documentation, InsertTextFormat,
    MarkupContent, MarkupKind, Position,
};

/// Handle completion request
pub fn handle_completion(
    doc: &mut DocumentState,
    position: Position,
    language: Language,
) -> Option<CompletionResponse> {
    let offset = position_to_offset(&doc.content, position);
    let content = &doc.content;

    // Get context for completion
    let context = get_completion_context(content, offset);

    let mut items = Vec::new();

    match context {
        CompletionContext::TopLevel => {
            // Top-level keywords
            items.extend(get_keyword_completions(language));
            // Add user-defined symbols
            items.extend(get_symbol_completions(doc, language));
        }
        CompletionContext::InFunction => {
            // Keywords valid in function body
            items.extend(get_statement_keyword_completions(language));
            // Built-in functions
            items.extend(get_builtin_completions(language));
            // User-defined symbols
            items.extend(get_symbol_completions(doc, language));
        }
        CompletionContext::AfterDot(prefix) => {
            // Member completions
            items.extend(get_member_completions(doc, &prefix, language));
        }
        CompletionContext::AfterColon => {
            // Type completions
            items.extend(get_type_completions(language));
        }
        CompletionContext::InImport => {
            // Module completions (placeholder for now)
            items.extend(get_module_completions(language));
        }
    }

    if items.is_empty() {
        None
    } else {
        Some(CompletionResponse::Array(items))
    }
}

/// Completion context
enum CompletionContext {
    TopLevel,
    InFunction,
    AfterDot(String),
    AfterColon,
    InImport,
}

/// Determine the completion context from the cursor position
fn get_completion_context(content: &str, offset: usize) -> CompletionContext {
    // Get the text before the cursor
    let before = &content[..offset.min(content.len())];

    // Check for dot (member access)
    if let Some(dot_pos) = before.rfind('.') {
        // Get the identifier before the dot
        let prefix_start = before[..dot_pos]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && !is_arabic_letter(c))
            .map(|p| p + 1)
            .unwrap_or(0);
        let prefix = before[prefix_start..dot_pos].to_string();
        return CompletionContext::AfterDot(prefix);
    }

    // Check for colon (type annotation)
    if before.trim_end().ends_with(':') {
        return CompletionContext::AfterColon;
    }

    // Check for import context
    if before.contains("استورد") || before.contains("import") {
        return CompletionContext::InImport;
    }

    // Check if we're inside a function body
    let open_braces = before.matches('{').count();
    let close_braces = before.matches('}').count();

    if open_braces > close_braces {
        CompletionContext::InFunction
    } else {
        CompletionContext::TopLevel
    }
}

/// Check if a character is an Arabic letter
fn is_arabic_letter(c: char) -> bool {
    matches!(c, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}')
}

/// Get keyword completions
fn get_keyword_completions(language: Language) -> Vec<CompletionItem> {
    let keywords = match language {
        Language::Arabic => vec![
            ("متغير", "تعريف متغير قابل للتعديل", "متغير $1 = $2"),
            ("ثابت", "تعريف ثابت", "ثابت $1 = $2"),
            ("دالة", "تعريف دالة", "دالة $1($2) -> $3 {\n\t$0\n}"),
            ("صنف", "تعريف صنف", "صنف $1 {\n\t$0\n}"),
            ("واجهة", "تعريف واجهة", "واجهة $1 {\n\t$0\n}"),
            ("استورد", "استيراد وحدة", "استورد { $1 } من \"$2\""),
            ("صدّر", "تصدير", "صدّر "),
        ],
        Language::English => vec![
            ("let", "Define a mutable variable", "let $1 = $2"),
            ("const", "Define a constant", "const $1 = $2"),
            (
                "function",
                "Define a function",
                "function $1($2) -> $3 {\n\t$0\n}",
            ),
            ("class", "Define a class", "class $1 {\n\t$0\n}"),
            (
                "interface",
                "Define an interface",
                "interface $1 {\n\t$0\n}",
            ),
            ("import", "Import a module", "import { $1 } from \"$2\""),
            ("export", "Export", "export "),
        ],
    };

    keywords
        .into_iter()
        .map(|(label, detail, snippet)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(detail.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        })
        .collect()
}

/// Get statement keyword completions
fn get_statement_keyword_completions(language: Language) -> Vec<CompletionItem> {
    let keywords = match language {
        Language::Arabic => vec![
            ("إذا", "جملة شرطية", "إذا ($1) {\n\t$0\n}"),
            ("وإلا", "فرع إذا فشل الشرط", "وإلا {\n\t$0\n}"),
            ("طالما", "حلقة طالما", "طالما ($1) {\n\t$0\n}"),
            ("لكل", "حلقة لكل", "لكل $1 في $2 {\n\t$0\n}"),
            ("أرجع", "إرجاع قيمة", "أرجع $0"),
            ("أوقف", "خروج من الحلقة", "أوقف"),
            ("استمر", "متابعة للتكرار التالي", "استمر"),
            (
                "حاول",
                "معالجة الأخطاء",
                "حاول {\n\t$1\n} التقط ($2) {\n\t$0\n}",
            ),
            ("ارمِ", "رمي خطأ", "ارمِ $0"),
            ("تطابق", "مطابقة الأنماط", "تطابق ($1) {\n\tحالة $2 => $0\n}"),
        ],
        Language::English => vec![
            ("if", "Conditional statement", "if ($1) {\n\t$0\n}"),
            ("else", "Else branch", "else {\n\t$0\n}"),
            ("while", "While loop", "while ($1) {\n\t$0\n}"),
            ("for", "For loop", "for $1 in $2 {\n\t$0\n}"),
            ("return", "Return a value", "return $0"),
            ("break", "Break from loop", "break"),
            ("continue", "Continue to next iteration", "continue"),
            (
                "try",
                "Error handling",
                "try {\n\t$1\n} catch ($2) {\n\t$0\n}",
            ),
            ("throw", "Throw an error", "throw $0"),
            (
                "match",
                "Pattern matching",
                "match ($1) {\n\tcase $2 => $0\n}",
            ),
        ],
    };

    keywords
        .into_iter()
        .map(|(label, detail, snippet)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(detail.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        })
        .collect()
}

/// Get built-in function completions
fn get_builtin_completions(language: Language) -> Vec<CompletionItem> {
    let builtins = match language {
        Language::Arabic => vec![
            ("اطبع", "طباعة قيمة", "اطبع($1)"),
            ("ادخل", "قراءة مدخل", "ادخل()"),
            ("طول", "طول المصفوفة أو النص", "طول($1)"),
            ("نوع", "نوع القيمة", "نوع($1)"),
            ("عدد", "تحويل إلى عدد صحيح", "عدد($1)"),
            ("نص", "تحويل إلى نص", "نص($1)"),
            ("جذر", "الجذر التربيعي", "جذر($1)"),
            ("مطلق", "القيمة المطلقة", "مطلق($1)"),
            ("اقرأ_ملف", "قراءة ملف", "اقرأ_ملف($1)"),
            ("اكتب_ملف", "كتابة ملف", "اكتب_ملف($1, $2)"),
        ],
        Language::English => vec![
            ("print", "Print a value", "print($1)"),
            ("input", "Read input", "input()"),
            ("len", "Length of array or string", "len($1)"),
            ("type", "Type of value", "type($1)"),
            ("int", "Convert to integer", "int($1)"),
            ("str", "Convert to string", "str($1)"),
            ("sqrt", "Square root", "sqrt($1)"),
            ("abs", "Absolute value", "abs($1)"),
            ("read_file", "Read a file", "read_file($1)"),
            ("write_file", "Write a file", "write_file($1, $2)"),
        ],
    };

    builtins
        .into_iter()
        .map(|(label, detail, snippet)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(detail.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        })
        .collect()
}

/// Get type completions
fn get_type_completions(language: Language) -> Vec<CompletionItem> {
    let types = match language {
        Language::Arabic => vec![
            ("عدد", "عدد صحيح"),
            ("عدد_عشري", "عدد عشري"),
            ("نص", "سلسلة نصية"),
            ("منطقي", "قيمة منطقية"),
            ("فراغ", "بدون قيمة"),
            ("مصفوفة", "مصفوفة"),
            ("قاموس", "قاموس"),
            ("أي", "أي نوع"),
        ],
        Language::English => vec![
            ("int", "Integer"),
            ("float", "Floating point"),
            ("string", "String"),
            ("bool", "Boolean"),
            ("void", "No value"),
            ("array", "Array"),
            ("map", "Map/Dictionary"),
            ("any", "Any type"),
        ],
    };

    types
        .into_iter()
        .map(|(label, detail)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some(detail.to_string()),
            ..Default::default()
        })
        .collect()
}

/// Get symbol completions from the document
fn get_symbol_completions(doc: &mut DocumentState, language: Language) -> Vec<CompletionItem> {
    let analysis = doc.get_analysis(language);

    analysis
        .symbols
        .iter()
        .filter(|(name, _)| !name.contains('.')) // Skip class members
        .map(|(name, info)| {
            let kind = match info.kind {
                SymbolKind::Variable => CompletionItemKind::VARIABLE,
                SymbolKind::Function => CompletionItemKind::FUNCTION,
                SymbolKind::Class => CompletionItemKind::CLASS,
                SymbolKind::Interface => CompletionItemKind::INTERFACE,
                SymbolKind::Parameter => CompletionItemKind::VARIABLE,
                SymbolKind::Field => CompletionItemKind::FIELD,
                SymbolKind::Method => CompletionItemKind::METHOD,
            };

            let type_str = match language {
                Language::Arabic => info.ty.arabic_name(),
                Language::English => info.ty.to_string(),
            };

            CompletionItem {
                label: name.clone(),
                kind: Some(kind),
                detail: Some(type_str),
                documentation: info.doc.as_ref().map(|d| {
                    Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: d.clone(),
                    })
                }),
                ..Default::default()
            }
        })
        .collect()
}

/// Get member completions for a type
fn get_member_completions(
    _doc: &mut DocumentState,
    _prefix: &str,
    language: Language,
) -> Vec<CompletionItem> {
    // For now, provide common array and string methods
    let methods = match language {
        Language::Arabic => vec![
            // Array methods
            ("طول", "طول المصفوفة"),
            ("ألحق", "إضافة عنصر"),
            ("احذف", "حذف عنصر"),
            ("فارغة", "هل المصفوفة فارغة"),
            // String methods
            ("قص", "قص جزء من النص"),
            ("قسّم", "تقسيم النص"),
            ("استبدل", "استبدال نص"),
            ("يحتوي", "هل يحتوي على نص"),
            ("أحرف_كبيرة", "تحويل لأحرف كبيرة"),
            ("أحرف_صغيرة", "تحويل لأحرف صغيرة"),
        ],
        Language::English => vec![
            // Array methods
            ("length", "Array length"),
            ("push", "Add an element"),
            ("pop", "Remove last element"),
            ("isEmpty", "Is array empty"),
            // String methods
            ("slice", "Slice a portion"),
            ("split", "Split string"),
            ("replace", "Replace text"),
            ("contains", "Contains text"),
            ("toUpperCase", "Convert to uppercase"),
            ("toLowerCase", "Convert to lowercase"),
        ],
    };

    methods
        .into_iter()
        .map(|(label, detail)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(detail.to_string()),
            ..Default::default()
        })
        .collect()
}

/// Get module completions
fn get_module_completions(language: Language) -> Vec<CompletionItem> {
    let modules = match language {
        Language::Arabic => vec![
            ("مجموعات", "قائمة، مجموعة، خريطة"),
            ("رياضيات", "دوال رياضية"),
            ("نص", "أدوات النصوص"),
            ("ملفات", "عمليات الملفات"),
            ("شبكة", "عمليات الشبكة"),
            ("وقت", "دوال الوقت والتاريخ"),
        ],
        Language::English => vec![
            ("collections", "List, Set, Map"),
            ("math", "Math functions"),
            ("string", "String utilities"),
            ("files", "File operations"),
            ("network", "Network operations"),
            ("time", "Date and time functions"),
        ],
    };

    modules
        .into_iter()
        .map(|(label, detail)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some(detail.to_string()),
            insert_text: Some(format!("\"{}\"", label)),
            ..Default::default()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_completion_context_top_level() {
        let content = "متغير س = 5\n";
        let context = get_completion_context(content, content.len());
        assert!(matches!(context, CompletionContext::TopLevel));
    }

    #[test]
    fn test_get_completion_context_in_function() {
        let content = "دالة اختبار() {\n    ";
        let context = get_completion_context(content, content.len());
        assert!(matches!(context, CompletionContext::InFunction));
    }

    #[test]
    fn test_get_completion_context_after_dot() {
        let content = "س.";
        let context = get_completion_context(content, content.len());
        assert!(matches!(context, CompletionContext::AfterDot(_)));
    }

    #[test]
    fn test_get_completion_context_after_colon() {
        let content = "متغير س: ";
        let context = get_completion_context(content, content.len());
        assert!(matches!(context, CompletionContext::AfterColon));
    }

    #[test]
    fn test_keyword_completions() {
        let completions = get_keyword_completions(Language::Arabic);
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label == "متغير"));
        assert!(completions.iter().any(|c| c.label == "دالة"));
    }

    #[test]
    fn test_type_completions() {
        let completions = get_type_completions(Language::Arabic);
        assert!(completions.iter().any(|c| c.label == "عدد"));
        assert!(completions.iter().any(|c| c.label == "نص"));
    }
}
