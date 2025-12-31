//! Code actions handler
//!
//! Provides quick fixes and refactorings for common issues.

use crate::error::codes::{ERR_CONST_ASSIGNMENT, ERR_UNDEFINED_VARIABLE, WARN_UNUSED_VARIABLE};
use crate::error::Language;
use crate::lsp::analysis::DocumentState;
use crate::lsp::utils::span_to_range;
use std::collections::HashMap;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Range, TextEdit, Url, WorkspaceEdit,
};

pub fn handle_code_actions(
    doc: &mut DocumentState,
    range: Range,
    language: Language,
) -> Option<Vec<CodeActionOrCommand>> {
    let mut actions = Vec::new();
    let uri = doc.uri.clone();
    let content = doc.content.clone();

    let analysis = doc.get_analysis(language);

    for diagnostic in &analysis.diagnostics {
        let diag_range = span_to_range(&content, &diagnostic.span);

        if ranges_overlap(&diag_range, &range) {
            if let Some(fix_actions) = generate_quick_fixes(&uri, diagnostic, &diag_range, language)
            {
                actions.extend(fix_actions);
            }
        }
    }

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

fn ranges_overlap(a: &Range, b: &Range) -> bool {
    !(a.end.line < b.start.line
        || (a.end.line == b.start.line && a.end.character < b.start.character)
        || b.end.line < a.start.line
        || (b.end.line == a.start.line && b.end.character < a.start.character))
}

fn generate_quick_fixes(
    uri: &Url,
    diagnostic: &crate::error::Diagnostic,
    range: &Range,
    language: Language,
) -> Option<Vec<CodeActionOrCommand>> {
    let mut actions = Vec::new();
    let message = &diagnostic.message;
    let _ = language; // Mark as used

    // Match on error code first for precise action matching
    if let Some(code) = &diagnostic.code {
        match code.as_str() {
            // د٠٠٠١: Undefined variable
            c if c == ERR_UNDEFINED_VARIABLE.to_string() => {
                if let Some(name) = extract_identifier_from_message(message) {
                    actions.push(create_declare_variable_action(uri, &name, range));
                }
            }
            // د٠١٠٢: Assignment to constant
            c if c == ERR_CONST_ASSIGNMENT.to_string() => {
                actions.push(create_change_to_mutable_action(uri, range));
            }
            // ح٠٠٠١: Unused variable
            c if c == WARN_UNUSED_VARIABLE.to_string() => {
                if let Some(name) = extract_identifier_from_message(message) {
                    actions.push(create_prefix_underscore_action(uri, &name, range));
                }
            }
            _ => {}
        }
    }

    // Fallback to message-based matching for backward compatibility
    if actions.is_empty() {
        if message.contains("غير معرف") {
            if let Some(name) = extract_identifier_from_message(message) {
                actions.push(create_declare_variable_action(uri, &name, range));
            }
        }

        if message.contains("ثابت") || message.contains("غير قابل") {
            actions.push(create_change_to_mutable_action(uri, range));
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

fn create_declare_variable_action(uri: &Url, name: &str, range: &Range) -> CodeActionOrCommand {
    let title = format!("إضافة تعريف لـ '{}'", name);
    let new_text = format!("متغير {} = ", name);

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

    CodeActionOrCommand::CodeAction(CodeAction {
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
    })
}

fn create_change_to_mutable_action(uri: &Url, range: &Range) -> CodeActionOrCommand {
    let title = "تحويل إلى متغير قابل للتعديل";
    let new_keyword = "متغير";

    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![TextEdit {
            range: *range,
            new_text: new_keyword.to_string(),
        }],
    );

    CodeActionOrCommand::CodeAction(CodeAction {
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
    })
}

fn create_prefix_underscore_action(uri: &Url, name: &str, range: &Range) -> CodeActionOrCommand {
    let title = format!("إضافة بادئة '_' لـ '{}'", name);
    let new_name = format!("_{}", name);

    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![TextEdit {
            range: *range,
            new_text: new_name,
        }],
    );

    CodeActionOrCommand::CodeAction(CodeAction {
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
    })
}

fn generate_refactorings(
    _uri: &Url,
    content: &str,
    range: &Range,
    analysis: &crate::lsp::analysis::AnalysisResult,
    _language: Language,
) -> Option<Vec<CodeActionOrCommand>> {
    use crate::semantic::Type;
    let mut actions = Vec::new();

    // Arabic-only: ترقيم لغة برمجة عربية
    for (name, info) in &analysis.symbols {
        let symbol_range = span_to_range(content, &info.definition_span);

        if ranges_overlap(&symbol_range, range) {
            if !matches!(info.ty, Type::Unknown) {
                let type_str = info.ty.arabic_name();
                let title = format!("إضافة تحديد النوع: {}", type_str);

                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title,
                    kind: Some(CodeActionKind::REFACTOR),
                    diagnostics: None,
                    edit: None, // Would need more precise position calculation
                    command: None,
                    is_preferred: Some(false),
                    disabled: Some(tower_lsp::lsp_types::CodeActionDisabled {
                        reason: "يتطلب تحليل موقع دقيق".to_string(),
                    }),
                    data: None,
                }));
            }

            let _ = name;
        }
    }

    if range.start != range.end {
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: "استخراج إلى متغير".to_string(),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            diagnostics: None,
            edit: None, // Would need expression analysis
            command: None,
            is_preferred: Some(false),
            disabled: Some(tower_lsp::lsp_types::CodeActionDisabled {
                reason: "يتطلب تحليل التعبير".to_string(),
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

fn extract_identifier_from_message(message: &str) -> Option<String> {
    if let Some(start_byte) = message.find('\'') {
        let after_quote = start_byte + '\''.len_utf8();
        if let Some(end_offset) = message[after_quote..].find('\'') {
            return Some(message[after_quote..after_quote + end_offset].to_string());
        }
    }

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

    #[test]
    fn test_code_action_for_undefined_variable() {
        use crate::error::{Diagnostic, Span};

        let uri = Url::parse("file:///test.ترقيم").unwrap();
        let range = Range::default();
        let mut diag = Diagnostic::error(
            "Undefined variable 'س'",
            "المتغير 'س' غير معرف",
            Span::new(0, 1, 1, 1),
        );
        diag.code = Some(ERR_UNDEFINED_VARIABLE.to_string());

        let actions = generate_quick_fixes(&uri, &diag, &range, Language::Arabic);
        assert!(actions.is_some());
        let actions = actions.unwrap();
        assert!(!actions.is_empty());

        // Check action title contains variable name
        if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
            assert!(action.title.contains("س"));
        }
    }

    #[test]
    fn test_code_action_for_const_assignment() {
        use crate::error::{Diagnostic, Span};

        let uri = Url::parse("file:///test.ترقيم").unwrap();
        let range = Range::default();
        let mut diag = Diagnostic::error(
            "Cannot assign to constant",
            "لا يمكن تعيين قيمة لثابت",
            Span::new(0, 5, 1, 1),
        );
        diag.code = Some(ERR_CONST_ASSIGNMENT.to_string());

        let actions = generate_quick_fixes(&uri, &diag, &range, Language::Arabic);
        assert!(actions.is_some());
        let actions = actions.unwrap();
        assert!(!actions.is_empty());

        if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
            assert!(action.title.contains("متغير"));
        }
    }

    #[test]
    fn test_code_action_for_unused_variable() {
        use crate::error::{Diagnostic, Span};

        let uri = Url::parse("file:///test.ترقيم").unwrap();
        let range = Range::default();
        let mut diag = Diagnostic::warning(
            "Unused variable 'س'",
            "المتغير 'س' غير مستخدم",
            Span::new(0, 1, 1, 1),
        );
        diag.code = Some(WARN_UNUSED_VARIABLE.to_string());

        let actions = generate_quick_fixes(&uri, &diag, &range, Language::Arabic);
        assert!(actions.is_some());
        let actions = actions.unwrap();
        assert!(!actions.is_empty());

        // Check action suggests adding underscore prefix
        if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
            assert!(action.title.contains("_"));
        }
    }

    #[test]
    fn test_code_action_fallback_without_code() {
        use crate::error::{Diagnostic, Span};

        let uri = Url::parse("file:///test.ترقيم").unwrap();
        let range = Range::default();
        // No error code set - should fall back to Arabic message matching
        let diag = Diagnostic::error(
            "المتغير 'س' غير معرف",
            "المتغير 'س' غير معرف",
            Span::new(0, 1, 1, 1),
        );

        let actions = generate_quick_fixes(&uri, &diag, &range, Language::Arabic);
        assert!(actions.is_some());
    }
}
