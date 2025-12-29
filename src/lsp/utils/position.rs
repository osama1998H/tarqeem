//! Position and offset conversion utilities
//!
//! Handles conversion between:
//! - Tarqeem's `Span` (character indices from lexer)
//! - LSP's `Position` (line/character, 0-indexed, UTF-16 code units)
//! - LSP's `Range` (start/end positions)
//!
//! IMPORTANT: The lexer stores spans as character indices (not byte offsets)
//! because it uses Vec<char>. This module converts those indices to LSP positions.

use crate::error::Span;
use tower_lsp::lsp_types::{Position, Range};

/// Converts a character index to an LSP Position.
///
/// The `offset` parameter is a CHARACTER INDEX (not byte offset) because
/// the Tarqeem lexer uses Vec<char> and stores character positions in spans.
pub fn offset_to_position(content: &str, offset: usize) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;
    let mut char_index = 0usize;

    for c in content.chars() {
        if char_index >= offset {
            break;
        }

        if c == '\n' {
            line += 1;
            character = 0;
        } else {
            // LSP uses UTF-16 code units for character position
            character += c.len_utf16() as u32;
        }

        char_index += 1; // Count characters, not bytes
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

#[allow(dead_code)]
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
    fn test_offset_to_position_arabic_multiline() {
        // This tests the exact scenario from the inlay hints bug
        // Content has بسم_الله on line 0, اطبع("مرحبا") on line 1
        let content = "بسم_الله\nاطبع(\"مرحبا\")";

        // Character indices (what the lexer produces):
        // 0-7: بسم_الله (8 chars)
        // 8: \n
        // 9-12: اطبع (4 chars)
        // 13: (
        // 14: "

        // Char 0 -> line 0, char 0
        assert_eq!(
            offset_to_position(content, 0),
            Position {
                line: 0,
                character: 0
            }
        );

        // Char 8 (newline) -> line 0, char 8
        assert_eq!(
            offset_to_position(content, 8),
            Position {
                line: 0,
                character: 8
            }
        );

        // Char 9 (first char after newline) -> line 1, char 0
        assert_eq!(
            offset_to_position(content, 9),
            Position {
                line: 1,
                character: 0
            }
        );

        // Char 14 (opening quote of string arg) -> line 1, char 5
        assert_eq!(
            offset_to_position(content, 14),
            Position {
                line: 1,
                character: 5
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
