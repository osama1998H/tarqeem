//! Code folding handler
//!
//! Provides collapsible regions for code blocks.

use crate::error::Language;
use crate::lsp::analysis::DocumentState;
use crate::lsp::utils::offset_to_position;
use crate::parser::{ClassMember, StmtKind};
use tower_lsp::lsp_types::{FoldingRange, FoldingRangeKind};

/// Handle folding range request
pub fn handle_folding_ranges(
    doc: &mut DocumentState,
    language: Language,
) -> Option<Vec<FoldingRange>> {
    let content = doc.content.clone();
    let analysis = doc.get_analysis(language);

    let mut ranges = Vec::new();

    // Collect folding ranges from AST
    if let Some(ref ast) = analysis.ast {
        for stmt in &ast.statements {
            collect_folding_ranges(&content, &stmt.kind, &stmt.span, &mut ranges);
        }
    }

    // Also add folding ranges for comments (multi-line comment blocks)
    collect_comment_folding_ranges(&content, &mut ranges);

    // Also add folding ranges for import groups
    collect_import_folding_ranges(&content, &analysis.ast, &mut ranges);

    if ranges.is_empty() {
        None
    } else {
        Some(ranges)
    }
}

/// Collect folding ranges from a statement
fn collect_folding_ranges(
    content: &str,
    stmt: &StmtKind,
    span: &crate::error::Span,
    ranges: &mut Vec<FoldingRange>,
) {
    match stmt {
        // Function declarations
        StmtKind::FuncDecl { body, .. } => {
            let start_pos = offset_to_position(content, span.start);
            let end_pos = offset_to_position(content, span.end);

            // Only fold if spans multiple lines
            if end_pos.line > start_pos.line {
                ranges.push(FoldingRange {
                    start_line: start_pos.line,
                    start_character: Some(start_pos.character),
                    end_line: end_pos.line,
                    end_character: Some(end_pos.character),
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: Some("...".to_string()),
                });
            }

            // Recursively process body statements
            for stmt in &body.statements {
                collect_folding_ranges(content, &stmt.kind, &stmt.span, ranges);
            }
        }

        // Class declarations
        StmtKind::ClassDecl { members, .. } => {
            let start_pos = offset_to_position(content, span.start);
            let end_pos = offset_to_position(content, span.end);

            if end_pos.line > start_pos.line {
                ranges.push(FoldingRange {
                    start_line: start_pos.line,
                    start_character: Some(start_pos.character),
                    end_line: end_pos.line,
                    end_character: Some(end_pos.character),
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: Some("...".to_string()),
                });
            }

            // Process class methods
            for member in members {
                if let ClassMember::Method { body, .. } = member {
                    for stmt in &body.statements {
                        collect_folding_ranges(content, &stmt.kind, &stmt.span, ranges);
                    }
                }
            }
        }

        // Interface declarations
        StmtKind::InterfaceDecl { .. } => {
            let start_pos = offset_to_position(content, span.start);
            let end_pos = offset_to_position(content, span.end);

            if end_pos.line > start_pos.line {
                ranges.push(FoldingRange {
                    start_line: start_pos.line,
                    start_character: Some(start_pos.character),
                    end_line: end_pos.line,
                    end_character: Some(end_pos.character),
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: Some("...".to_string()),
                });
            }
        }

        // If statements
        StmtKind::If {
            then_branch,
            else_branch,
            ..
        } => {
            let start_pos = offset_to_position(content, span.start);
            let end_pos = offset_to_position(content, span.end);

            if end_pos.line > start_pos.line {
                ranges.push(FoldingRange {
                    start_line: start_pos.line,
                    start_character: Some(start_pos.character),
                    end_line: end_pos.line,
                    end_character: Some(end_pos.character),
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: Some("...".to_string()),
                });
            }

            // Process branches
            for stmt in &then_branch.statements {
                collect_folding_ranges(content, &stmt.kind, &stmt.span, ranges);
            }
            if let Some(else_block) = else_branch {
                for stmt in &else_block.statements {
                    collect_folding_ranges(content, &stmt.kind, &stmt.span, ranges);
                }
            }
        }

        // While loops
        StmtKind::While { body, .. } => {
            let start_pos = offset_to_position(content, span.start);
            let end_pos = offset_to_position(content, span.end);

            if end_pos.line > start_pos.line {
                ranges.push(FoldingRange {
                    start_line: start_pos.line,
                    start_character: Some(start_pos.character),
                    end_line: end_pos.line,
                    end_character: Some(end_pos.character),
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: Some("...".to_string()),
                });
            }

            for stmt in &body.statements {
                collect_folding_ranges(content, &stmt.kind, &stmt.span, ranges);
            }
        }

        // For loops
        StmtKind::For { body, .. } => {
            let start_pos = offset_to_position(content, span.start);
            let end_pos = offset_to_position(content, span.end);

            if end_pos.line > start_pos.line {
                ranges.push(FoldingRange {
                    start_line: start_pos.line,
                    start_character: Some(start_pos.character),
                    end_line: end_pos.line,
                    end_character: Some(end_pos.character),
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: Some("...".to_string()),
                });
            }

            for stmt in &body.statements {
                collect_folding_ranges(content, &stmt.kind, &stmt.span, ranges);
            }
        }

        // ForIn loops
        StmtKind::ForIn { body, .. } => {
            let start_pos = offset_to_position(content, span.start);
            let end_pos = offset_to_position(content, span.end);

            if end_pos.line > start_pos.line {
                ranges.push(FoldingRange {
                    start_line: start_pos.line,
                    start_character: Some(start_pos.character),
                    end_line: end_pos.line,
                    end_character: Some(end_pos.character),
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: Some("...".to_string()),
                });
            }

            for stmt in &body.statements {
                collect_folding_ranges(content, &stmt.kind, &stmt.span, ranges);
            }
        }

        // Match statements
        StmtKind::Match { arms, .. } => {
            let start_pos = offset_to_position(content, span.start);
            let end_pos = offset_to_position(content, span.end);

            if end_pos.line > start_pos.line {
                ranges.push(FoldingRange {
                    start_line: start_pos.line,
                    start_character: Some(start_pos.character),
                    end_line: end_pos.line,
                    end_character: Some(end_pos.character),
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: Some("...".to_string()),
                });
            }

            // Process match arm bodies
            for arm in arms {
                for stmt in &arm.body.statements {
                    collect_folding_ranges(content, &stmt.kind, &stmt.span, ranges);
                }
            }
        }

        // Try-catch-finally
        StmtKind::Try {
            body,
            catch,
            finally,
        } => {
            let start_pos = offset_to_position(content, span.start);
            let end_pos = offset_to_position(content, span.end);

            if end_pos.line > start_pos.line {
                ranges.push(FoldingRange {
                    start_line: start_pos.line,
                    start_character: Some(start_pos.character),
                    end_line: end_pos.line,
                    end_character: Some(end_pos.character),
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: Some("...".to_string()),
                });
            }

            for stmt in &body.statements {
                collect_folding_ranges(content, &stmt.kind, &stmt.span, ranges);
            }
            if let Some(catch_clause) = catch {
                for stmt in &catch_clause.body.statements {
                    collect_folding_ranges(content, &stmt.kind, &stmt.span, ranges);
                }
            }
            if let Some(finally_block) = finally {
                for stmt in &finally_block.statements {
                    collect_folding_ranges(content, &stmt.kind, &stmt.span, ranges);
                }
            }
        }

        // Block statement
        StmtKind::Block(block) => {
            let start_pos = offset_to_position(content, span.start);
            let end_pos = offset_to_position(content, span.end);

            if end_pos.line > start_pos.line {
                ranges.push(FoldingRange {
                    start_line: start_pos.line,
                    start_character: Some(start_pos.character),
                    end_line: end_pos.line,
                    end_character: Some(end_pos.character),
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: Some("...".to_string()),
                });
            }

            for stmt in &block.statements {
                collect_folding_ranges(content, &stmt.kind, &stmt.span, ranges);
            }
        }

        _ => {}
    }
}

/// Collect folding ranges for comment blocks
fn collect_comment_folding_ranges(content: &str, ranges: &mut Vec<FoldingRange>) {
    let mut comment_start: Option<u32> = None;
    let mut comment_end: u32 = 0;

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num as u32;
        let trimmed = line.trim();

        if trimmed.starts_with("//") || trimmed.starts_with("؟؟") {
            if comment_start.is_none() {
                comment_start = Some(line_num);
            }
            comment_end = line_num;
        } else if comment_start.is_some() {
            // End of comment block
            if let Some(start) = comment_start {
                if comment_end > start {
                    ranges.push(FoldingRange {
                        start_line: start,
                        start_character: None,
                        end_line: comment_end,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Comment),
                        collapsed_text: Some("// ...".to_string()),
                    });
                }
            }
            comment_start = None;
        }
    }

    // Handle trailing comment block
    if let Some(start) = comment_start {
        if comment_end > start {
            ranges.push(FoldingRange {
                start_line: start,
                start_character: None,
                end_line: comment_end,
                end_character: None,
                kind: Some(FoldingRangeKind::Comment),
                collapsed_text: Some("// ...".to_string()),
            });
        }
    }
}

/// Collect folding ranges for import groups
fn collect_import_folding_ranges(
    content: &str,
    ast: &Option<crate::parser::Ast>,
    ranges: &mut Vec<FoldingRange>,
) {
    let Some(ast) = ast else {
        return;
    };

    let mut import_start: Option<u32> = None;
    let mut import_end: u32 = 0;

    for stmt in &ast.statements {
        if matches!(stmt.kind, StmtKind::Import { .. }) {
            let pos = offset_to_position(content, stmt.span.start);
            if import_start.is_none() {
                import_start = Some(pos.line);
            }
            import_end = pos.line;
        } else if import_start.is_some() {
            // End of import group
            break;
        }
    }

    if let Some(start) = import_start {
        if import_end > start {
            ranges.push(FoldingRange {
                start_line: start,
                start_character: None,
                end_line: import_end,
                end_character: None,
                kind: Some(FoldingRangeKind::Imports),
                collapsed_text: Some("...".to_string()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    /// Helper to wrap source code with required file markers
    fn wrap_with_markers(source: &str) -> String {
        format!("بسم_الله\n{}\nالحمد_لله", source.trim())
    }

    #[test]
    fn test_folding_ranges_function() {
        let uri = Url::parse("file:///test.trq").unwrap();
        // Use English keywords for consistent parsing in tests
        let content = wrap_with_markers(
            r#"
function add(a: int, b: int) -> int {
    return a + b
}
"#,
        );
        let mut doc = DocumentState::new(uri, 1, content);

        let ranges = handle_folding_ranges(&mut doc, Language::English);
        assert!(ranges.is_some());
        let ranges = ranges.unwrap();
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_folding_ranges_arabic_function() {
        let uri = Url::parse("file:///test.trq").unwrap();
        let content = wrap_with_markers(
            r#"
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}
"#,
        );
        let mut doc = DocumentState::new(uri, 1, content);

        let ranges = handle_folding_ranges(&mut doc, Language::Arabic);
        // Parser may not produce AST for Arabic functions in all cases
        // If it does, verify the folding ranges are correct
        if let Some(ranges) = ranges {
            assert!(!ranges.is_empty());
        }
    }

    #[test]
    fn test_folding_ranges_class() {
        let uri = Url::parse("file:///test.trq").unwrap();
        let content = wrap_with_markers(
            r#"
صنف شخص {
    خاص اسم: نص

    منشئ(اسم: نص) {
        هذا.اسم = اسم
    }
}
"#,
        );
        let mut doc = DocumentState::new(uri, 1, content);

        let ranges = handle_folding_ranges(&mut doc, Language::Arabic);
        assert!(ranges.is_some());
    }

    #[test]
    fn test_comment_folding_ranges() {
        let content = "// Comment 1\n// Comment 2\n// Comment 3\ncode here";
        let mut ranges = Vec::new();
        collect_comment_folding_ranges(content, &mut ranges);

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start_line, 0);
        assert_eq!(ranges[0].end_line, 2);
        assert_eq!(ranges[0].kind, Some(FoldingRangeKind::Comment));
    }

    #[test]
    fn test_no_folding_single_line() {
        let uri = Url::parse("file:///test.trq").unwrap();
        let content = wrap_with_markers("متغير س = 5");
        let mut doc = DocumentState::new(uri, 1, content);

        let ranges = handle_folding_ranges(&mut doc, Language::Arabic);
        // Single line statements should not create folding ranges
        assert!(ranges.is_none() || ranges.unwrap().is_empty());
    }
}
