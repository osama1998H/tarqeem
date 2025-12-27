//! Auto-completion handler
//!
//! Provides intelligent code completion suggestions.

use crate::error::Language;
use crate::lsp::analysis::{DocumentState, SymbolKind};
use crate::lsp::utils::position_to_offset;
use crate::parser::{StmtKind, TypeAnnotation, TypeKind};
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Documentation, InsertTextFormat,
    MarkupContent, MarkupKind, Position,
};

pub fn handle_completion(
    doc: &mut DocumentState,
    position: Position,
    language: Language,
) -> Option<CompletionResponse> {
    let offset = position_to_offset(&doc.content, position);
    let content = &doc.content;

    let context = get_completion_context(content, offset);

    let mut items = Vec::new();

    match context {
        CompletionContext::TopLevel => {
            items.extend(get_keyword_completions(language));
            items.extend(get_symbol_completions(doc, language));
        }
        CompletionContext::InFunction => {
            items.extend(get_statement_keyword_completions(language));
            items.extend(get_builtin_completions(language));
            items.extend(get_symbol_completions(doc, language));
        }
        CompletionContext::AfterDot(prefix) => {
            items.extend(get_member_completions(doc, &prefix, language));
        }
        CompletionContext::AfterColon => {
            items.extend(get_type_completions(language));
        }
        CompletionContext::AfterColonColon(enum_name) => {
            items.extend(get_enum_variant_completions(doc, &enum_name, language));
        }
        CompletionContext::InImport => {
            items.extend(get_module_completions(language));
        }
    }

    if items.is_empty() {
        None
    } else {
        Some(CompletionResponse::Array(items))
    }
}

enum CompletionContext {
    TopLevel,
    InFunction,
    AfterDot(String),
    AfterColon,
    AfterColonColon(String), // For enum variant access: EnumName::
    InImport,
}

fn get_completion_context(content: &str, offset: usize) -> CompletionContext {
    let before = &content[..offset.min(content.len())];

    // Check for :: (enum variant access) first, before single :
    if let Some(stripped) = before.strip_suffix("::") {
        let prefix_start = stripped
            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && !is_arabic_letter(c))
            .map(|p| p + 1)
            .unwrap_or(0);
        let prefix = stripped[prefix_start..].to_string();
        return CompletionContext::AfterColonColon(prefix);
    }

    if let Some(dot_pos) = before.rfind('.') {
        let prefix_start = before[..dot_pos]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && !is_arabic_letter(c))
            .map(|p| p + 1)
            .unwrap_or(0);
        let prefix = before[prefix_start..dot_pos].to_string();
        return CompletionContext::AfterDot(prefix);
    }

    if before.trim_end().ends_with(':') && !before.trim_end().ends_with("::") {
        return CompletionContext::AfterColon;
    }

    if before.contains("استورد") || before.contains("import") {
        return CompletionContext::InImport;
    }

    let open_braces = before.matches('{').count();
    let close_braces = before.matches('}').count();

    if open_braces > close_braces {
        CompletionContext::InFunction
    } else {
        CompletionContext::TopLevel
    }
}

fn is_arabic_letter(c: char) -> bool {
    matches!(c, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}')
}

fn get_keyword_completions(language: Language) -> Vec<CompletionItem> {
    let keywords = match language {
        Language::Arabic => vec![
            ("متغير", "تعريف متغير قابل للتعديل", "متغير $1 = $2"),
            ("ثابت", "تعريف ثابت", "ثابت $1 = $2"),
            ("دالة", "تعريف دالة", "دالة $1($2) -> $3 {\n\t$0\n}"),
            ("صنف", "تعريف صنف", "صنف $1 {\n\t$0\n}"),
            ("ميثاق", "تعريف ميثاق", "ميثاق $1 {\n\t$0\n}"),
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

fn get_type_completions(language: Language) -> Vec<CompletionItem> {
    let types = match language {
        Language::Arabic => vec![
            ("عدد", "عدد صحيح"),
            ("عدد_عشري", "عدد عشري"),
            ("نص", "سلسلة نصية"),
            ("منطقي", "قيمة منطقية"),
            ("مصفوفة", "مصفوفة"),
            ("قاموس", "قاموس"),
            ("أي", "أي نوع"),
        ],
        Language::English => vec![
            ("int", "Integer"),
            ("float", "Floating point"),
            ("string", "String"),
            ("bool", "Boolean"),
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
                SymbolKind::Property => CompletionItemKind::PROPERTY,
                SymbolKind::Enum => CompletionItemKind::ENUM,
                SymbolKind::EnumVariant => CompletionItemKind::ENUM_MEMBER,
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

fn get_member_completions(
    _doc: &mut DocumentState,
    _prefix: &str,
    language: Language,
) -> Vec<CompletionItem> {
    let methods = match language {
        Language::Arabic => vec![
            ("طول", "طول المصفوفة"),
            ("ألحق", "إضافة عنصر"),
            ("احذف", "حذف عنصر"),
            ("فارغة", "هل المصفوفة فارغة"),
            ("قص", "قص جزء من النص"),
            ("قسّم", "تقسيم النص"),
            ("استبدل", "استبدال نص"),
            ("يحتوي", "هل يحتوي على نص"),
            ("أحرف_كبيرة", "تحويل لأحرف كبيرة"),
            ("أحرف_صغيرة", "تحويل لأحرف صغيرة"),
        ],
        Language::English => vec![
            ("length", "Array length"),
            ("push", "Add an element"),
            ("pop", "Remove last element"),
            ("isEmpty", "Is array empty"),
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

/// Get enum variant completions for a given enum type
fn get_enum_variant_completions(
    doc: &mut DocumentState,
    enum_name: &str,
    language: Language,
) -> Vec<CompletionItem> {
    let analysis = doc.get_analysis(language);

    // Look for enum declarations in the AST
    let mut items = Vec::new();

    // Check for enums in the current document's AST
    if let Some(ast) = &analysis.ast {
        for stmt in &ast.statements {
            if let StmtKind::EnumDecl { name, variants, .. } = &stmt.kind {
                if name == enum_name {
                    for variant in variants {
                        let detail = if variant.fields.is_empty() {
                            match language {
                                Language::Arabic => "حالة بسيطة".to_string(),
                                Language::English => "Simple variant".to_string(),
                            }
                        } else {
                            let field_types: Vec<String> =
                                variant.fields.iter().map(|f| format_type(&f.ty)).collect();
                            format!("({})", field_types.join("، "))
                        };

                        let insert_text = if variant.fields.is_empty() {
                            variant.name.clone()
                        } else {
                            format!("{}(${{1}})", variant.name)
                        };

                        items.push(CompletionItem {
                            label: variant.name.clone(),
                            kind: Some(CompletionItemKind::ENUM_MEMBER),
                            detail: Some(detail),
                            insert_text: Some(insert_text),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }

    items
}

fn format_type(ty: &TypeAnnotation) -> String {
    match &ty.kind {
        TypeKind::Simple(name) => name.clone(),
        TypeKind::Array(inner) => format!("مصفوفة<{}>", format_type(inner)),
        TypeKind::Map(key, value) => format!("قاموس<{}، {}>", format_type(key), format_type(value)),
        TypeKind::Optional(inner) => format!("{}?", format_type(inner)),
        TypeKind::Function {
            params,
            return_type,
        } => {
            let params_str: Vec<String> = params.iter().map(format_type).collect();
            format!(
                "({}) -> {}",
                params_str.join("، "),
                format_type(return_type)
            )
        }
        TypeKind::Generic { base, args } => {
            let args_str: Vec<String> = args.iter().map(format_type).collect();
            format!("{}<{}>", base, args_str.join("، "))
        }
    }
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
