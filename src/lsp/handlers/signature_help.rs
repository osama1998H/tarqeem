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

/// Handle signature help request
///
/// This is called when the user is typing inside a function call,
/// typically after '(' or ',' characters.
pub fn handle_signature_help(
    doc: &mut DocumentState,
    position: Position,
    language: Language,
) -> Option<SignatureHelp> {
    let content = doc.content.clone();
    let offset = position_to_offset(&content, position);

    // Find the function call context
    let context = find_call_context(&content, offset)?;

    let analysis = doc.get_analysis(language);

    // Look up the function in symbols
    let symbol_info = analysis.symbols.get(&context.function_name)?;

    // Must be a function or method
    if !matches!(symbol_info.kind, SymbolKind::Function | SymbolKind::Method) {
        return None;
    }

    // Extract parameter types from the function type
    let (param_types, return_type) = match &symbol_info.ty {
        Type::Function {
            params,
            return_type,
        } => (params.clone(), return_type.as_ref().clone()),
        _ => return None,
    };

    // Parse doc comment for parameter descriptions
    let parsed_doc = symbol_info.doc.as_ref().map(|d| DocCommentParser::parse(d));

    // Build parameter information
    let parameters: Vec<ParameterInformation> = param_types
        .iter()
        .enumerate()
        .map(|(i, param_type)| {
            // Try to find parameter name and description from doc
            let param_doc = parsed_doc.as_ref().and_then(|pd| pd.params.get(i));

            let param_name = param_doc
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format_param_placeholder(i, language));

            let type_str = match language {
                Language::Arabic => param_type.arabic_name(),
                Language::English => param_type.to_string(),
            };

            let label = format!("{}: {}", param_name, type_str);

            // Build parameter documentation
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

    // Build signature label
    let params_str: Vec<String> = parameters
        .iter()
        .map(|p| match &p.label {
            ParameterLabel::Simple(s) => s.clone(),
            ParameterLabel::LabelOffsets(_) => String::new(),
        })
        .collect();

    let return_type_str = match language {
        Language::Arabic => return_type.arabic_name(),
        Language::English => return_type.to_string(),
    };

    let func_label = match language {
        Language::Arabic => "دالة",
        Language::English => "function",
    };

    let signature_label = format!(
        "{} {}({}) -> {}",
        func_label,
        context.function_name,
        params_str.join("، "),
        return_type_str
    );

    // Build main documentation from parsed doc comment
    let signature_documentation = parsed_doc.as_ref().and_then(|pd| {
        pd.description.as_ref().map(|desc| {
            let mut doc_text = desc.clone();

            // Add return description if available
            if let Some(returns) = &pd.returns {
                if let Some(ret_desc) = &returns.description {
                    let returns_label = match language {
                        Language::Arabic => "الإرجاع",
                        Language::English => "Returns",
                    };
                    doc_text.push_str(&format!("\n\n**{}**: {}", returns_label, ret_desc));
                }
            }

            // Add notes if available
            for note in &pd.notes {
                let note_label = match language {
                    Language::Arabic => "ملاحظة",
                    Language::English => "Note",
                };
                doc_text.push_str(&format!("\n\n**{}**: {}", note_label, note));
            }

            // Add warnings if available
            for warning in &pd.warnings {
                let warning_label = match language {
                    Language::Arabic => "تحذير",
                    Language::English => "Warning",
                };
                doc_text.push_str(&format!("\n\n**{}**: {}", warning_label, warning));
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

/// Context information about a function call
#[derive(Debug)]
struct CallContext {
    /// The name of the function being called
    function_name: String,
    /// The index of the parameter the cursor is on (0-indexed)
    active_parameter: usize,
}

/// Find the function call context at the given offset
///
/// This searches backwards from the cursor to find:
/// 1. The opening parenthesis of the function call
/// 2. The function name before the parenthesis
/// 3. How many commas are before the cursor (to determine active parameter)
fn find_call_context(content: &str, offset: usize) -> Option<CallContext> {
    let chars: Vec<char> = content.chars().collect();

    // Convert byte offset to char index
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

    // Search backwards to find the opening parenthesis
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

    // Find the function name before the parenthesis
    // Skip whitespace before '('
    let mut name_end = open_paren_pos;
    while name_end > 0 && chars[name_end - 1].is_whitespace() {
        name_end -= 1;
    }

    // Find the start of the identifier
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

/// Check if a character is valid in an identifier
fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || is_arabic_letter(c)
}

/// Check if a character is an Arabic letter
fn is_arabic_letter(c: char) -> bool {
    matches!(c, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}')
}

/// Format a placeholder parameter name when doc is not available
fn format_param_placeholder(index: usize, language: Language) -> String {
    match language {
        Language::Arabic => format!("معامل{}", index + 1),
        Language::English => format!("param{}", index + 1),
    }
}

/// Get signature help for built-in functions
pub fn get_builtin_signature_help(
    name: &str,
    active_parameter: usize,
    language: Language,
) -> Option<SignatureHelp> {
    let (label, params, doc_ar, doc_en) = match name {
        // I/O Functions
        "اطبع" | "print" => (
            match language {
                Language::Arabic => "دالة اطبع(قيمة: أي) -> فراغ",
                Language::English => "function print(value: any) -> void",
            },
            vec![ParameterInformation {
                label: ParameterLabel::Simple(match language {
                    Language::Arabic => "قيمة: أي".to_string(),
                    Language::English => "value: any".to_string(),
                }),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: match language {
                        Language::Arabic => "القيمة المراد طباعتها".to_string(),
                        Language::English => "The value to print".to_string(),
                    },
                })),
            }],
            "طباعة قيمة إلى المخرج القياسي",
            "Print a value to standard output",
        ),

        "ادخل" | "input" => (
            match language {
                Language::Arabic => "دالة ادخل() -> نص",
                Language::English => "function input() -> string",
            },
            vec![],
            "قراءة سطر من المدخل القياسي",
            "Read a line from standard input",
        ),

        "طول" | "len" | "length" => (
            match language {
                Language::Arabic => "دالة طول(قيمة: أي) -> عدد",
                Language::English => "function len(value: any) -> int",
            },
            vec![ParameterInformation {
                label: ParameterLabel::Simple(match language {
                    Language::Arabic => "قيمة: أي".to_string(),
                    Language::English => "value: any".to_string(),
                }),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: match language {
                        Language::Arabic => "المصفوفة أو النص المراد معرفة طوله".to_string(),
                        Language::English => "The array or string to get the length of".to_string(),
                    },
                })),
            }],
            "الحصول على طول المصفوفة أو النص",
            "Get the length of an array or string",
        ),

        "قوة" | "pow" => (
            match language {
                Language::Arabic => "دالة قوة(أساس: عدد_عشري، أس: عدد_عشري) -> عدد_عشري",
                Language::English => "function pow(base: float, exponent: float) -> float",
            },
            vec![
                ParameterInformation {
                    label: ParameterLabel::Simple(match language {
                        Language::Arabic => "أساس: عدد_عشري".to_string(),
                        Language::English => "base: float".to_string(),
                    }),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: match language {
                            Language::Arabic => "الأساس المراد رفعه".to_string(),
                            Language::English => "The base number".to_string(),
                        },
                    })),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple(match language {
                        Language::Arabic => "أس: عدد_عشري".to_string(),
                        Language::English => "exponent: float".to_string(),
                    }),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: match language {
                            Language::Arabic => "الأس".to_string(),
                            Language::English => "The exponent".to_string(),
                        },
                    })),
                },
            ],
            "رفع عدد لقوة معينة",
            "Raise a number to a power",
        ),

        "اقرأ_ملف" | "read_file" => (
            match language {
                Language::Arabic => "دالة اقرأ_ملف(مسار: نص) -> نص",
                Language::English => "function read_file(path: string) -> string",
            },
            vec![ParameterInformation {
                label: ParameterLabel::Simple(match language {
                    Language::Arabic => "مسار: نص".to_string(),
                    Language::English => "path: string".to_string(),
                }),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: match language {
                        Language::Arabic => "مسار الملف المراد قراءته".to_string(),
                        Language::English => "Path to the file to read".to_string(),
                    },
                })),
            }],
            "قراءة محتوى ملف",
            "Read the contents of a file",
        ),

        "اكتب_ملف" | "write_file" => (
            match language {
                Language::Arabic => "دالة اكتب_ملف(مسار: نص، محتوى: نص) -> منطقي",
                Language::English => "function write_file(path: string, content: string) -> bool",
            },
            vec![
                ParameterInformation {
                    label: ParameterLabel::Simple(match language {
                        Language::Arabic => "مسار: نص".to_string(),
                        Language::English => "path: string".to_string(),
                    }),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: match language {
                            Language::Arabic => "مسار الملف المراد الكتابة إليه".to_string(),
                            Language::English => "Path to the file to write".to_string(),
                        },
                    })),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple(match language {
                        Language::Arabic => "محتوى: نص".to_string(),
                        Language::English => "content: string".to_string(),
                    }),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: match language {
                            Language::Arabic => "المحتوى المراد كتابته".to_string(),
                            Language::English => "The content to write".to_string(),
                        },
                    })),
                },
            ],
            "كتابة محتوى إلى ملف",
            "Write contents to a file",
        ),

        _ => return None,
    };

    let description = match language {
        Language::Arabic => doc_ar,
        Language::English => doc_en,
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
    fn test_find_call_context_english() {
        let content = "print(add(1, 2), ";
        let offset = content.len();
        let ctx = find_call_context(content, offset);

        assert!(ctx.is_some());
        let ctx = ctx.unwrap();
        assert_eq!(ctx.function_name, "print");
        assert_eq!(ctx.active_parameter, 1);
    }

    #[test]
    fn test_builtin_signature_help() {
        let help = get_builtin_signature_help("اطبع", 0, Language::Arabic);
        assert!(help.is_some());
        let help = help.unwrap();
        assert!(!help.signatures.is_empty());
        assert!(help.signatures[0].label.contains("اطبع"));

        let help_en = get_builtin_signature_help("print", 0, Language::English);
        assert!(help_en.is_some());
        let help_en = help_en.unwrap();
        assert!(help_en.signatures[0].label.contains("print"));
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
