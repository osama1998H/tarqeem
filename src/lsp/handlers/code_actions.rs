//! Code actions handler
//!
//! Provides quick fixes and refactorings for common issues.

use crate::error::Language;
use crate::lsp::analysis::DocumentState;
use crate::lsp::utils::span_to_range;
use std::collections::HashMap;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Range, TextEdit, Url, WorkspaceEdit,
};

/// Handle code actions request
pub fn handle_code_actions(
    doc: &mut DocumentState,
    range: Range,
    language: Language,
) -> Option<Vec<CodeActionOrCommand>> {
    let mut actions = Vec::new();
    let uri = doc.uri.clone();
    let content = doc.content.clone();

    let analysis = doc.get_analysis(language);

    // Check diagnostics for quick fixes
    for diagnostic in &analysis.diagnostics {
        let diag_range = span_to_range(&content, &diagnostic.span);

        // Check if diagnostic overlaps with request range
        if ranges_overlap(&diag_range, &range) {
            // Generate quick fixes based on diagnostic type
            if let Some(fix_actions) = generate_quick_fixes(&uri, diagnostic, &diag_range, language)
            {
                actions.extend(fix_actions);
            }
        }
    }

    // Add refactoring actions based on context
    if let Some(refactor_actions) =
        generate_refactorings(&uri, &content, &range, analysis, language)
    {
        actions.extend(refactor_actions);
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

/// Check if two ranges overlap
fn ranges_overlap(a: &Range, b: &Range) -> bool {
    !(a.end.line < b.start.line
        || (a.end.line == b.start.line && a.end.character < b.start.character)
        || b.end.line < a.start.line
        || (b.end.line == a.start.line && b.end.character < a.start.character))
}

/// Generate quick fixes for a diagnostic
fn generate_quick_fixes(
    uri: &Url,
    diagnostic: &crate::error::Diagnostic,
    range: &Range,
    language: Language,
) -> Option<Vec<CodeActionOrCommand>> {
    let mut actions = Vec::new();
    let message = &diagnostic.message;

    // Check for "undefined variable" errors - suggest declaration
    if message.contains("undefined") || message.contains("غير معرف") {
        let var_name = extract_identifier_from_message(message);
        if let Some(name) = var_name {
            let (title, new_text) = match language {
                Language::Arabic => (
                    format!("إضافة تعريف لـ '{}'", name),
                    format!("متغير {} = ", name),
                ),
                Language::English => (
                    format!("Add declaration for '{}'", name),
                    format!("let {} = ", name),
                ),
            };

            let insert_position = tower_lsp::lsp_types::Position {
                line: range.start.line,
                character: 0,
            };

            let mut changes = HashMap::new();
            changes.insert(
                uri.clone(),
                vec![TextEdit {
                    range: Range {
                        start: insert_position,
                        end: insert_position,
                    },
                    new_text: format!("{}\n", new_text),
                }],
            );

            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title,
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: None,
                edit: Some(WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                }),
                command: None,
                is_preferred: Some(false),
                disabled: None,
                data: None,
            }));
        }
    }

    // Check for "immutable" errors - suggest making mutable
    if message.contains("immutable") || message.contains("ثابت") || message.contains("غير قابل")
    {
        let (title, old_keyword, new_keyword) = match language {
            Language::Arabic => ("تحويل إلى متغير قابل للتعديل", "ثابت", "متغير"),
            Language::English => ("Convert to mutable variable", "const", "let"),
        };

        let mut changes = HashMap::new();
        changes.insert(
            uri.clone(),
            vec![TextEdit {
                range: *range,
                new_text: new_keyword.to_string(),
            }],
        );

        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: title.to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: None,
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }),
            command: None,
            is_preferred: Some(true),
            disabled: None,
            data: None,
        }));

        // Suppress unused variable warning
        let _ = old_keyword;
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

/// Generate refactoring actions
fn generate_refactorings(
    uri: &Url,
    content: &str,
    range: &Range,
    analysis: &crate::lsp::analysis::AnalysisResult,
    language: Language,
) -> Option<Vec<CodeActionOrCommand>> {
    use crate::semantic::Type;
    let mut actions = Vec::new();

    // Check if selection is a variable declaration without type annotation
    for (name, info) in &analysis.symbols {
        let symbol_range = span_to_range(content, &info.definition_span);

        if ranges_overlap(&symbol_range, range) {
            // Offer to add type annotation if none exists
            if !matches!(info.ty, Type::Unknown) {
                let type_str = match language {
                    Language::Arabic => info.ty.arabic_name(),
                    Language::English => info.ty.to_string(),
                };

                let title = match language {
                    Language::Arabic => format!("إضافة تحديد النوع: {}", type_str),
                    Language::English => format!("Add type annotation: {}", type_str),
                };

                // This is a simplified version - real implementation would need
                // to find the exact position after the variable name
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title,
                    kind: Some(CodeActionKind::REFACTOR),
                    diagnostics: None,
                    edit: None, // Would need more precise position calculation
                    command: None,
                    is_preferred: Some(false),
                    disabled: Some(tower_lsp::lsp_types::CodeActionDisabled {
                        reason: match language {
                            Language::Arabic => "يتطلب تحليل موقع دقيق".to_string(),
                            Language::English => "Requires precise position analysis".to_string(),
                        },
                    }),
                    data: None,
                }));
            }

            // Suppress unused variable warnings
            let _ = name;
        }
    }

    // Offer to extract expression to variable if selection spans an expression
    if range.start != range.end {
        let title = match language {
            Language::Arabic => "استخراج إلى متغير",
            Language::English => "Extract to variable",
        };

        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: title.to_string(),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            diagnostics: None,
            edit: None, // Would need expression analysis
            command: None,
            is_preferred: Some(false),
            disabled: Some(tower_lsp::lsp_types::CodeActionDisabled {
                reason: match language {
                    Language::Arabic => "يتطلب تحليل التعبير".to_string(),
                    Language::English => "Requires expression analysis".to_string(),
                },
            }),
            data: None,
        }));
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

/// Extract identifier name from error message
fn extract_identifier_from_message(message: &str) -> Option<String> {
    // Look for patterns like "'name'" or "«name»" in the message
    // Use char_indices for proper UTF-8 handling

    // Check for single quotes
    if let Some(start_byte) = message.find('\'') {
        let after_quote = start_byte + '\''.len_utf8();
        if let Some(end_offset) = message[after_quote..].find('\'') {
            return Some(message[after_quote..after_quote + end_offset].to_string());
        }
    }

    // Check for Arabic guillemets «»
    if let Some(start_byte) = message.find('«') {
        let after_open = start_byte + '«'.len_utf8();
        if let Some(end_offset) = message[after_open..].find('»') {
            return Some(message[after_open..after_open + end_offset].to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ranges_overlap() {
        let a = Range {
            start: tower_lsp::lsp_types::Position {
                line: 0,
                character: 0,
            },
            end: tower_lsp::lsp_types::Position {
                line: 0,
                character: 10,
            },
        };
        let b = Range {
            start: tower_lsp::lsp_types::Position {
                line: 0,
                character: 5,
            },
            end: tower_lsp::lsp_types::Position {
                line: 0,
                character: 15,
            },
        };
        let c = Range {
            start: tower_lsp::lsp_types::Position {
                line: 1,
                character: 0,
            },
            end: tower_lsp::lsp_types::Position {
                line: 1,
                character: 10,
            },
        };

        assert!(ranges_overlap(&a, &b));
        assert!(ranges_overlap(&b, &a));
        assert!(!ranges_overlap(&a, &c));
    }

    #[test]
    fn test_extract_identifier_from_message() {
        assert_eq!(
            extract_identifier_from_message("Variable 'foo' is undefined"),
            Some("foo".to_string())
        );
        assert_eq!(
            extract_identifier_from_message("المتغير «س» غير معرف"),
            Some("س".to_string())
        );
        assert_eq!(extract_identifier_from_message("Some other message"), None);
    }
}
