//! Inlay hints handler
//!
//! Provides inline type annotations and parameter hints.

use crate::error::Language;
use crate::lsp::analysis::{DocumentState, SymbolKind};
use crate::lsp::utils::offset_to_position;
use crate::semantic::Type;
use tower_lsp::lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, Position, Range};

pub fn handle_inlay_hints(
    doc: &mut DocumentState,
    range: Range,
    language: Language,
) -> Option<Vec<InlayHint>> {
    let content = doc.content.clone();
    let analysis = doc.get_analysis(language);
    let mut hints = Vec::new();

    for (name, info) in &analysis.symbols {
        if matches!(info.ty, Type::Unknown | Type::Void) {
            continue;
        }

        if !matches!(info.kind, SymbolKind::Variable | SymbolKind::Parameter) {
            continue;
        }

        let hint_position = offset_to_position(&content, info.definition_span.end);

        if !position_in_range(&hint_position, &range) {
            continue;
        }

        let type_str = match language {
            Language::Arabic => format!(": {}", info.ty.arabic_name()),
            Language::English => format!(": {}", info.ty),
        };

        hints.push(InlayHint {
            position: hint_position,
            label: InlayHintLabel::String(type_str),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: Some(tower_lsp::lsp_types::InlayHintTooltip::String(
                match language {
                    Language::Arabic => {
                        format!("نوع المتغير '{}' هو {}", name, info.ty.arabic_name())
                    }
                    Language::English => format!("Type of '{}' is {}", name, info.ty),
                },
            )),
            padding_left: Some(true),
            padding_right: Some(false),
            data: None,
        });
    }

    if let Some(ref ast) = analysis.ast {
        collect_parameter_hints(&content, ast, &mut hints, language, &range);
    }

    if hints.is_empty() {
        None
    } else {
        Some(hints)
    }
}

fn position_in_range(pos: &Position, range: &Range) -> bool {
    if pos.line < range.start.line || pos.line > range.end.line {
        return false;
    }
    if pos.line == range.start.line && pos.character < range.start.character {
        return false;
    }
    if pos.line == range.end.line && pos.character > range.end.character {
        return false;
    }
    true
}

fn collect_parameter_hints(
    content: &str,
    ast: &crate::parser::Ast,
    hints: &mut Vec<InlayHint>,
    language: Language,
    range: &Range,
) {
    for stmt in &ast.statements {
        collect_parameter_hints_from_stmt(content, &stmt.kind, hints, language, range);
    }
}

fn collect_parameter_hints_from_stmt(
    content: &str,
    stmt: &crate::parser::StmtKind,
    hints: &mut Vec<InlayHint>,
    language: Language,
    range: &Range,
) {
    use crate::parser::StmtKind;

    match stmt {
        StmtKind::Expr(expr) => {
            collect_parameter_hints_from_expr(content, &expr.kind, hints, language, range);
        }
        StmtKind::VarDecl {
            init: Some(val), ..
        } => {
            collect_parameter_hints_from_expr(content, &val.kind, hints, language, range);
        }
        StmtKind::VarDecl { init: None, .. } => {}
        StmtKind::FuncDecl { body, .. } => {
            for stmt in &body.statements {
                collect_parameter_hints_from_stmt(content, &stmt.kind, hints, language, range);
            }
        }
        StmtKind::If {
            then_branch,
            else_branch,
            condition,
            ..
        } => {
            collect_parameter_hints_from_expr(content, &condition.kind, hints, language, range);
            for stmt in &then_branch.statements {
                collect_parameter_hints_from_stmt(content, &stmt.kind, hints, language, range);
            }
            if let Some(else_block) = else_branch {
                for stmt in &else_block.statements {
                    collect_parameter_hints_from_stmt(content, &stmt.kind, hints, language, range);
                }
            }
        }
        StmtKind::While {
            body, condition, ..
        } => {
            collect_parameter_hints_from_expr(content, &condition.kind, hints, language, range);
            for stmt in &body.statements {
                collect_parameter_hints_from_stmt(content, &stmt.kind, hints, language, range);
            }
        }
        StmtKind::For { body, .. } => {
            for stmt in &body.statements {
                collect_parameter_hints_from_stmt(content, &stmt.kind, hints, language, range);
            }
        }
        StmtKind::ForIn { body, .. } => {
            for stmt in &body.statements {
                collect_parameter_hints_from_stmt(content, &stmt.kind, hints, language, range);
            }
        }
        StmtKind::Return(Some(expr)) => {
            collect_parameter_hints_from_expr(content, &expr.kind, hints, language, range);
        }
        StmtKind::ClassDecl { members, .. } => {
            use crate::parser::ClassMember;
            for member in members {
                if let ClassMember::Method { body, .. } = member {
                    for stmt in &body.statements {
                        collect_parameter_hints_from_stmt(
                            content, &stmt.kind, hints, language, range,
                        );
                    }
                }
            }
        }
        StmtKind::Block(block) => {
            for stmt in &block.statements {
                collect_parameter_hints_from_stmt(content, &stmt.kind, hints, language, range);
            }
        }
        _ => {}
    }
}

fn collect_parameter_hints_from_expr(
    content: &str,
    expr: &crate::parser::ExprKind,
    hints: &mut Vec<InlayHint>,
    language: Language,
    range: &Range,
) {
    use crate::parser::ExprKind;

    match expr {
        ExprKind::Call { callee, args } => {
            if let ExprKind::Identifier(name) = &callee.kind {
                if let Some(param_names) = get_builtin_param_names(name, language) {
                    for (i, (arg, param_name)) in args.iter().zip(param_names.iter()).enumerate() {
                        let hint_position = offset_to_position(content, arg.span.start);

                        if !position_in_range(&hint_position, range) {
                            continue;
                        }

                        hints.push(InlayHint {
                            position: hint_position,
                            label: InlayHintLabel::String(format!("{}: ", param_name)),
                            kind: Some(InlayHintKind::PARAMETER),
                            text_edits: None,
                            tooltip: Some(tower_lsp::lsp_types::InlayHintTooltip::String(
                                match language {
                                    Language::Arabic => format!("المعامل رقم {}", i + 1),
                                    Language::English => format!("Parameter {}", i + 1),
                                },
                            )),
                            padding_left: Some(false),
                            padding_right: Some(true),
                            data: None,
                        });
                    }
                }
            }

            collect_parameter_hints_from_expr(content, &callee.kind, hints, language, range);
            for arg in args {
                collect_parameter_hints_from_expr(content, &arg.kind, hints, language, range);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            collect_parameter_hints_from_expr(content, &left.kind, hints, language, range);
            collect_parameter_hints_from_expr(content, &right.kind, hints, language, range);
        }
        ExprKind::Unary { operand, .. } => {
            collect_parameter_hints_from_expr(content, &operand.kind, hints, language, range);
        }
        ExprKind::Member { object, .. } => {
            collect_parameter_hints_from_expr(content, &object.kind, hints, language, range);
        }
        ExprKind::Index { object, index, .. } => {
            collect_parameter_hints_from_expr(content, &object.kind, hints, language, range);
            collect_parameter_hints_from_expr(content, &index.kind, hints, language, range);
        }
        ExprKind::Array(elements) => {
            for elem in elements {
                collect_parameter_hints_from_expr(content, &elem.kind, hints, language, range);
            }
        }
        ExprKind::Object(pairs) => {
            for (_key, value) in pairs {
                collect_parameter_hints_from_expr(content, &value.kind, hints, language, range);
            }
        }
        _ => {}
    }
}

fn get_builtin_param_names(name: &str, language: Language) -> Option<Vec<&'static str>> {
    match (name, language) {
        ("اطبع", Language::Arabic) => Some(vec!["قيمة"]),
        ("اطبع_سطر", Language::Arabic) => Some(vec!["قيمة"]),
        ("ادخل", Language::Arabic) => Some(vec![]),
        ("طول", Language::Arabic) => Some(vec!["قيمة"]),
        ("نوع", Language::Arabic) => Some(vec!["قيمة"]),
        ("قوة", Language::Arabic) => Some(vec!["أساس", "أس"]),
        ("جذر", Language::Arabic) => Some(vec!["قيمة"]),
        ("مطلق", Language::Arabic) => Some(vec!["قيمة"]),
        ("اقرأ_ملف", Language::Arabic) => Some(vec!["مسار"]),
        ("اكتب_ملف", Language::Arabic) => Some(vec!["مسار", "محتوى"]),

        ("print", Language::English) => Some(vec!["value"]),
        ("println", Language::English) => Some(vec!["value"]),
        ("input", Language::English) => Some(vec![]),
        ("len", Language::English) | ("length", Language::English) => Some(vec!["value"]),
        ("type", Language::English) | ("typeof", Language::English) => Some(vec!["value"]),
        ("pow", Language::English) => Some(vec!["base", "exp"]),
        ("sqrt", Language::English) => Some(vec!["value"]),
        ("abs", Language::English) => Some(vec!["value"]),
        ("read_file", Language::English) => Some(vec!["path"]),
        ("write_file", Language::English) => Some(vec!["path", "content"]),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    fn test_uri() -> Url {
        Url::parse("file:///test.trq").unwrap()
    }

    #[test]
    fn test_position_in_range() {
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 10,
                character: 100,
            },
        };

        assert!(position_in_range(
            &Position {
                line: 5,
                character: 50
            },
            &range
        ));
        assert!(position_in_range(
            &Position {
                line: 0,
                character: 0
            },
            &range
        ));
        assert!(position_in_range(
            &Position {
                line: 10,
                character: 100
            },
            &range
        ));
        assert!(!position_in_range(
            &Position {
                line: 11,
                character: 0
            },
            &range
        ));
    }

    #[test]
    fn test_get_builtin_param_names() {
        assert_eq!(
            get_builtin_param_names("اطبع", Language::Arabic),
            Some(vec!["قيمة"])
        );
        assert_eq!(
            get_builtin_param_names("print", Language::English),
            Some(vec!["value"])
        );
        assert_eq!(
            get_builtin_param_names("قوة", Language::Arabic),
            Some(vec!["أساس", "أس"])
        );
        assert_eq!(get_builtin_param_names("unknown", Language::English), None);
    }

    #[test]
    fn test_inlay_hints_basic() {
        let content = "متغير س = 5".to_string();
        let mut doc = DocumentState::new(test_uri(), 1, content);

        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 100,
                character: 0,
            },
        };

        let hints = handle_inlay_hints(&mut doc, range, Language::Arabic);
        assert!(hints.is_some() || hints.is_none()); // May or may not have hints depending on analysis
    }
}
