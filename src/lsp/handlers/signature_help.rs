//! Signature help handler
//!
//! Provides function signature information with parameter documentation
//! when the user is typing a function call.

use crate::doc::comment::DocCommentParser;
use crate::error::Language;
use crate::lsp::analysis::{DocumentState, SymbolKind};
use crate::lsp::utils::position_to_offset;
use crate::semantic::Type;
use tower_lsp::lsp_types::{
    Documentation, MarkupContent, MarkupKind, ParameterInformation, ParameterLabel, Position,
    SignatureHelp, SignatureInformation,
};

pub fn handle_signature_help(
    doc: &mut DocumentState,
    position: Position,
    language: Language,
) -> Option<SignatureHelp> {
    let content = doc.content.clone();
    let offset = position_to_offset(&content, position);

    let context = find_call_context(&content, offset)?;

    let analysis = doc.get_analysis(language);

    let symbol_info = analysis.symbols.get(&context.function_name)?;

    if !matches!(symbol_info.kind, SymbolKind::Function | SymbolKind::Method) {
        return None;
    }

    let (param_types, return_type) = match &symbol_info.ty {
        Type::Function {
            params,
            return_type,
        } => (params.clone(), return_type.as_ref().clone()),
        _ => return None,
    };

    let parsed_doc = symbol_info.doc.as_ref().map(|d| DocCommentParser::parse(d));

    // Arabic-only: ترقيم لغة برمجة عربية
    let parameters: Vec<ParameterInformation> = param_types
        .iter()
        .enumerate()
        .map(|(i, param_type)| {
            let param_doc = parsed_doc.as_ref().and_then(|pd| pd.params.get(i));

            let param_name = param_doc
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format_param_placeholder(i));

            let type_str = param_type.arabic_name();

            let label = format!("{}: {}", param_name, type_str);

            let documentation = param_doc.and_then(|p| {
                p.description.as_ref().map(|desc| {
                    Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: desc.clone(),
                    })
                })
            });

            ParameterInformation {
                label: ParameterLabel::Simple(label),
                documentation,
            }
        })
        .collect();

    let params_str: Vec<String> = parameters
        .iter()
        .map(|p| match &p.label {
            ParameterLabel::Simple(s) => s.clone(),
            ParameterLabel::LabelOffsets(_) => String::new(),
        })
        .collect();

    // Arabic-only: ترقيم لغة برمجة عربية
    let return_type_str = return_type.arabic_name();

    let func_label = "دالة";

    let signature_label = format!(
        "{} {}({}) -> {}",
        func_label,
        context.function_name,
        params_str.join("، "),
        return_type_str
    );

    // Arabic-only: ترقيم لغة برمجة عربية
    let signature_documentation = parsed_doc.as_ref().and_then(|pd| {
        pd.description.as_ref().map(|desc| {
            let mut doc_text = desc.clone();

            if let Some(returns) = &pd.returns {
                if let Some(ret_desc) = &returns.description {
                    doc_text.push_str(&format!("\n\n**الإرجاع**: {}", ret_desc));
                }
            }

            for note in &pd.notes {
                doc_text.push_str(&format!("\n\n**ملاحظة**: {}", note));
            }

            for warning in &pd.warnings {
                doc_text.push_str(&format!("\n\n**تحذير**: {}", warning));
            }

            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: doc_text,
            })
        })
    });

    Some(SignatureHelp {
        signatures: vec![SignatureInformation {
            label: signature_label,
            documentation: signature_documentation,
            parameters: Some(parameters),
            active_parameter: Some(context.active_parameter as u32),
        }],
        active_signature: Some(0),
        active_parameter: Some(context.active_parameter as u32),
    })
}

#[derive(Debug)]
struct CallContext {
    function_name: String,
    active_parameter: usize,
}

fn find_call_context(content: &str, offset: usize) -> Option<CallContext> {
    let chars: Vec<char> = content.chars().collect();

    let mut char_offset = 0;
    let mut byte_count = 0;
    for (i, c) in content.chars().enumerate() {
        if byte_count >= offset {
            char_offset = i;
            break;
        }
        byte_count += c.len_utf8();
        char_offset = i + 1;
    }

    let mut paren_depth = 0;
    let mut comma_count = 0;
    let mut open_paren_pos = None;

    for i in (0..char_offset).rev() {
        let c = chars[i];

        match c {
            ')' => paren_depth += 1,
            '(' => {
                if paren_depth == 0 {
                    open_paren_pos = Some(i);
                    break;
                }
                paren_depth -= 1;
            }
            '،' | ',' => {
                if paren_depth == 0 {
                    comma_count += 1;
                }
            }
            _ => {}
        }
    }

    let open_paren_pos = open_paren_pos?;

    let mut name_end = open_paren_pos;
    while name_end > 0 && chars[name_end - 1].is_whitespace() {
        name_end -= 1;
    }

    let mut name_start = name_end;
    while name_start > 0 && is_identifier_char(chars[name_start - 1]) {
        name_start -= 1;
    }

    if name_start == name_end {
        return None;
    }

    let function_name: String = chars[name_start..name_end].iter().collect();

    Some(CallContext {
        function_name,
        active_parameter: comma_count,
    })
}

fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || is_arabic_letter(c)
}

fn is_arabic_letter(c: char) -> bool {
    matches!(c, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}')
}

fn format_param_placeholder(index: usize) -> String {
    // Arabic-only: ترقيم لغة برمجة عربية
    format!("معامل{}", index + 1)
}

#[allow(dead_code)]
pub fn get_builtin_signature_help(
    name: &str,
    active_parameter: usize,
    _language: Language,
) -> Option<SignatureHelp> {
    // Arabic-only: ترقيم لغة برمجة عربية
    let (label, params, description) = match name {
        "اطبع" => (
            "دالة اطبع(قيمة: أي)",
            vec![ParameterInformation {
                label: ParameterLabel::Simple("قيمة: أي".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "القيمة المراد طباعتها".to_string(),
                })),
            }],
            "طباعة قيمة إلى المخرج القياسي",
        ),

        "ادخل" => ("دالة ادخل() -> نص", vec![], "قراءة سطر من المدخل القياسي"),

        "طول" => (
            "دالة طول(قيمة: أي) -> عدد",
            vec![ParameterInformation {
                label: ParameterLabel::Simple("قيمة: أي".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "المصفوفة أو النص المراد معرفة طوله".to_string(),
                })),
            }],
            "الحصول على طول المصفوفة أو النص",
        ),

        "قوة" => (
            "دالة قوة(أساس: عدد_عشري، أس: عدد_عشري) -> عدد_عشري",
            vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("أساس: عدد_عشري".to_string()),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: "الأساس المراد رفعه".to_string(),
                    })),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("أس: عدد_عشري".to_string()),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: "الأس".to_string(),
                    })),
                },
            ],
            "رفع عدد لقوة معينة",
        ),

        "اقرأ_ملف" => (
            "دالة اقرأ_ملف(مسار: نص) -> نص",
            vec![ParameterInformation {
                label: ParameterLabel::Simple("مسار: نص".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "مسار الملف المراد قراءته".to_string(),
                })),
            }],
            "قراءة محتوى ملف",
        ),

        "اكتب_ملف" => (
            "دالة اكتب_ملف(مسار: نص، محتوى: نص) -> منطقي",
            vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("مسار: نص".to_string()),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: "مسار الملف المراد الكتابة إليه".to_string(),
                    })),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("محتوى: نص".to_string()),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: "المحتوى المراد كتابته".to_string(),
                    })),
                },
            ],
            "كتابة محتوى إلى ملف",
        ),

        _ => return None,
    };

    Some(SignatureHelp {
        signatures: vec![SignatureInformation {
            label: label.to_string(),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: description.to_string(),
            })),
            parameters: Some(params),
            active_parameter: Some(active_parameter as u32),
        }],
        active_signature: Some(0),
        active_parameter: Some(active_parameter as u32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_call_context_simple() {
        let content = "جمع(5، ";
        let offset = content.len();
        let ctx = find_call_context(content, offset);

        assert!(ctx.is_some());
        let ctx = ctx.unwrap();
        assert_eq!(ctx.function_name, "جمع");
        assert_eq!(ctx.active_parameter, 1);
    }

    #[test]
    fn test_find_call_context_first_param() {
        let content = "اطبع(";
        let offset = content.len();
        let ctx = find_call_context(content, offset);

        assert!(ctx.is_some());
        let ctx = ctx.unwrap();
        assert_eq!(ctx.function_name, "اطبع");
        assert_eq!(ctx.active_parameter, 0);
    }

    #[test]
    fn test_find_call_context_nested() {
        let content = "اطبع(جمع(1، 2)، ";
        let offset = content.len();
        let ctx = find_call_context(content, offset);

        assert!(ctx.is_some());
        let ctx = ctx.unwrap();
        assert_eq!(ctx.function_name, "اطبع");
        assert_eq!(ctx.active_parameter, 1);
    }

    #[test]
    fn test_builtin_signature_help() {
        // Arabic-only: ترقيم لغة برمجة عربية
        let help = get_builtin_signature_help("اطبع", 0, Language::Arabic);
        assert!(help.is_some());
        let help = help.unwrap();
        assert!(!help.signatures.is_empty());
        assert!(help.signatures[0].label.contains("اطبع"));

        // English names are no longer supported
        let help_en = get_builtin_signature_help("print", 0, Language::Arabic);
        assert!(help_en.is_none());
    }

    #[test]
    fn test_builtin_signature_help_multi_param() {
        let help = get_builtin_signature_help("قوة", 1, Language::Arabic);
        assert!(help.is_some());
        let help = help.unwrap();
        assert_eq!(help.active_parameter, Some(1));
        assert_eq!(help.signatures[0].parameters.as_ref().unwrap().len(), 2);
    }
}
