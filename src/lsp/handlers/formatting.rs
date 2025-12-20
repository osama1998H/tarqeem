//! Code formatting handler
//!
//! Provides basic code formatting support.

use tower_lsp::lsp_types::{Position, Range, TextEdit};

/// Handle document formatting request
///
/// For now, this provides basic formatting:
/// - Normalize indentation (4 spaces)
/// - Ensure consistent spacing around operators
/// - Trim trailing whitespace
pub fn handle_formatting(content: &str) -> Option<Vec<TextEdit>> {
    let formatted = format_content(content);

    if formatted == content {
        return None;
    }

    // Replace the entire document
    let lines: Vec<&str> = content.lines().collect();
    let end_line = lines.len().saturating_sub(1);
    let end_char = lines.last().map(|l| l.len()).unwrap_or(0);

    Some(vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: end_line as u32,
                character: end_char as u32,
            },
        },
        new_text: formatted,
    }])
}

/// Format the content
fn format_content(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut indent_level = 0u32;
    let indent_str = "    "; // 4 spaces

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            result.push('\n');
            continue;
        }

        // Adjust indent level based on closing braces at start of line
        if trimmed.starts_with('}') || trimmed.starts_with(')') || trimmed.starts_with(']') {
            indent_level = indent_level.saturating_sub(1);
        }

        // Handle else/catch/finally that start with closing brace
        if trimmed.starts_with('}') && (trimmed.contains("وإلا") || trimmed.contains("else")
            || trimmed.contains("التقط") || trimmed.contains("catch")
            || trimmed.contains("أخيراً") || trimmed.contains("finally"))
        {
            // Special case: } وإلا { should be on same line
            result.push_str(&indent_str.repeat(indent_level as usize));
            result.push_str(trimmed);
            result.push('\n');

            // Count opening braces to adjust indent
            let opens = trimmed.matches('{').count();
            let closes = trimmed.matches('}').count();
            if opens > closes {
                indent_level += 1;
            }
            continue;
        }

        // Add proper indentation
        result.push_str(&indent_str.repeat(indent_level as usize));

        // Format the line content
        let formatted_line = format_line(trimmed);
        result.push_str(&formatted_line);
        result.push('\n');

        // Adjust indent level for next line
        let opens = trimmed.matches('{').count() + trimmed.matches('(').count();
        let closes = trimmed.matches('}').count() + trimmed.matches(')').count();

        if opens > closes {
            indent_level += (opens - closes) as u32;
        } else if closes > opens && !trimmed.starts_with('}') && !trimmed.starts_with(')') {
            indent_level = indent_level.saturating_sub((closes - opens) as u32);
        }
    }

    // Remove trailing newline if the original didn't have one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Format a single line
fn format_line(line: &str) -> String {
    let mut result = String::with_capacity(line.len() * 2);
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut string_char = '"';

    while let Some(c) = chars.next() {
        // Handle string literals
        if (c == '"' || c == '\'' || c == '«') && !in_string {
            in_string = true;
            string_char = c;
            result.push(c);
            continue;
        }

        if in_string {
            result.push(c);
            let end_char = match string_char {
                '«' => '»',
                _ => string_char,
            };
            if c == end_char {
                in_string = false;
            }
            continue;
        }

        // Handle operators with spacing
        match c {
            '=' if chars.peek() == Some(&'=') => {
                ensure_space_before(&mut result);
                result.push_str("==");
                chars.next();
                ensure_space_after(&mut result, &mut chars);
            }
            '!' if chars.peek() == Some(&'=') => {
                ensure_space_before(&mut result);
                result.push_str("!=");
                chars.next();
                ensure_space_after(&mut result, &mut chars);
            }
            '<' if chars.peek() == Some(&'=') => {
                ensure_space_before(&mut result);
                result.push_str("<=");
                chars.next();
                ensure_space_after(&mut result, &mut chars);
            }
            '>' if chars.peek() == Some(&'=') => {
                ensure_space_before(&mut result);
                result.push_str(">=");
                chars.next();
                ensure_space_after(&mut result, &mut chars);
            }
            '+' if chars.peek() == Some(&'+') => {
                result.push_str("++");
                chars.next();
            }
            '-' if chars.peek() == Some(&'-') => {
                result.push_str("--");
                chars.next();
            }
            '-' if chars.peek() == Some(&'>') => {
                ensure_space_before(&mut result);
                result.push_str("->");
                chars.next();
                ensure_space_after(&mut result, &mut chars);
            }
            '=' if chars.peek() == Some(&'>') => {
                ensure_space_before(&mut result);
                result.push_str("=>");
                chars.next();
                ensure_space_after(&mut result, &mut chars);
            }
            '+' | '-' | '*' | '/' | '%' | '=' => {
                ensure_space_before(&mut result);
                result.push(c);
                ensure_space_after(&mut result, &mut chars);
            }
            ',' | '،' => {
                result.push(c);
                if chars.peek().map(|c| !c.is_whitespace()).unwrap_or(false) {
                    result.push(' ');
                }
            }
            ':' => {
                result.push(c);
                if chars.peek().map(|c| !c.is_whitespace()).unwrap_or(false) {
                    result.push(' ');
                }
            }
            '{' => {
                ensure_space_before(&mut result);
                result.push(c);
            }
            _ => {
                result.push(c);
            }
        }
    }

    result
}

/// Ensure there's a space before the current position
fn ensure_space_before(result: &mut String) {
    if !result.is_empty() && !result.ends_with(' ') && !result.ends_with('\t') {
        result.push(' ');
    }
}

/// Ensure there's a space after an operator
fn ensure_space_after(result: &mut String, chars: &mut std::iter::Peekable<std::str::Chars>) {
    if let Some(&next) = chars.peek() {
        if !next.is_whitespace() && next != '\n' {
            result.push(' ');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple() {
        let input = "متغير س=5";
        let result = format_content(input);
        assert_eq!(result, "متغير س = 5");
    }

    #[test]
    fn test_format_indentation() {
        let input = "دالة اختبار() {\nمتغير س = 5\n}";
        let result = format_content(input);
        assert!(result.contains("    متغير")); // Should be indented
    }

    #[test]
    fn test_format_operators() {
        let input = "متغير س=1+2*3";
        let result = format_content(input);
        assert!(result.contains("1 + 2"));
        assert!(result.contains("2 * 3"));
    }

    #[test]
    fn test_format_preserves_strings() {
        let input = "متغير س = \"hello world\"";
        let result = format_content(input);
        assert!(result.contains("\"hello world\"")); // String unchanged
    }

    #[test]
    fn test_format_comma_spacing() {
        let input = "دالة اختبار(أ:عدد،ب:عدد)";
        let result = format_content(input);
        assert!(result.contains("أ: عدد"));
        assert!(result.contains("، ب") || result.contains(", ب"));
    }
}
