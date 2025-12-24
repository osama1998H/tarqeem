//! Position and offset conversion utilities
//!
//! Handles conversion between:
//! - Tarqeem's `Span` (byte offsets + line/column)
//! - LSP's `Position` (line/character, 0-indexed)
//! - LSP's `Range` (start/end positions)

use crate::error::Span;
use tower_lsp::lsp_types::{Position, Range};

pub fn offset_to_position(content: &str, offset: usize) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;
    let mut current_offset = 0usize;

    for c in content.chars() {
        if current_offset >= offset {
            break;
        }

        if c == '\n' {
            line += 1;
            character = 0;
        } else {
            character += c.len_utf16() as u32;
        }

        current_offset += c.len_utf8();
    }

    Position { line, character }
}

pub fn position_to_offset(content: &str, position: Position) -> usize {
    let mut current_line = 0u32;
    let mut current_char = 0u32;
    let mut offset = 0usize;

    for c in content.chars() {
        if current_line == position.line && current_char >= position.character {
            break;
        }

        if current_line > position.line {
            break;
        }

        offset += c.len_utf8();

        if c == '\n' {
            current_line += 1;
            current_char = 0;
        } else {
            current_char += c.len_utf16() as u32;
        }
    }

    offset
}

pub fn span_to_range(content: &str, span: &Span) -> Range {
    if span.line > 0 {
        let start = Position {
            line: (span.line - 1) as u32,
            character: (span.column.saturating_sub(1)) as u32,
        };

        let end = offset_to_position(content, span.end);

        Range { start, end }
    } else {
        Range {
            start: offset_to_position(content, span.start),
            end: offset_to_position(content, span.end),
        }
    }
}

pub fn find_word_at_position(content: &str, position: Position) -> Option<(usize, usize, String)> {
    let offset = position_to_offset(content, position);

    let _bytes = content.as_bytes();

    let mut start = offset;
    while start > 0 {
        let c = content[..start].chars().last()?;
        if !is_identifier_char(c) {
            break;
        }
        start -= c.len_utf8();
    }

    let mut end = offset;
    for c in content[offset..].chars() {
        if !is_identifier_char(c) {
            break;
        }
        end += c.len_utf8();
    }

    if start == end {
        return None;
    }

    Some((start, end, content[start..end].to_string()))
}

fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || is_arabic_letter(c)
}

fn is_arabic_letter(c: char) -> bool {
    matches!(c, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}')
}

pub fn get_line_at_position(content: &str, line: u32) -> Option<&str> {
    content.lines().nth(line as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_to_position_simple() {
        let content = "hello\nworld";
        assert_eq!(
            offset_to_position(content, 0),
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            offset_to_position(content, 5),
            Position {
                line: 0,
                character: 5
            }
        );
        assert_eq!(
            offset_to_position(content, 6),
            Position {
                line: 1,
                character: 0
            }
        );
        assert_eq!(
            offset_to_position(content, 11),
            Position {
                line: 1,
                character: 5
            }
        );
    }

    #[test]
    fn test_offset_to_position_arabic() {
        let content = "متغير س = 5";
        assert_eq!(
            offset_to_position(content, 0),
            Position {
                line: 0,
                character: 0
            }
        );
    }

    #[test]
    fn test_position_to_offset() {
        let content = "hello\nworld";
        assert_eq!(
            position_to_offset(
                content,
                Position {
                    line: 0,
                    character: 0
                }
            ),
            0
        );
        assert_eq!(
            position_to_offset(
                content,
                Position {
                    line: 1,
                    character: 0
                }
            ),
            6
        );
        assert_eq!(
            position_to_offset(
                content,
                Position {
                    line: 1,
                    character: 5
                }
            ),
            11
        );
    }

    #[test]
    fn test_span_to_range() {
        let content = "متغير س = 5";
        let span = Span::new(0, 10, 1, 1);
        let range = span_to_range(content, &span);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
    }

    #[test]
    fn test_find_word_at_position() {
        let content = "متغير س = 5";
        let result = find_word_at_position(
            content,
            Position {
                line: 0,
                character: 2,
            },
        );
        assert!(result.is_some());
        let (_, _, word) = result.unwrap();
        assert_eq!(word, "متغير");
    }

    #[test]
    fn test_is_arabic_letter() {
        assert!(is_arabic_letter('م'));
        assert!(is_arabic_letter('ت'));
        assert!(is_arabic_letter('غ'));
        assert!(!is_arabic_letter('a'));
        assert!(!is_arabic_letter('1'));
    }
}
