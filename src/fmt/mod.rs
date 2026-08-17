//! Code Formatter (trqfmt) for Tarqeem
//!
//! Provides AST-based code formatting with configurable style options.
//!
//! ## Usage
//!
//! ```bash
//! # Format a file to stdout
//! tarqeem fmt file.ترقيم
//!
//! # Format and write back to file
//! tarqeem fmt -w file.ترقيم
//!
//! # Check if file is formatted (without changing)
//! tarqeem fmt --check file.ترقيم
//! ```

mod config;
mod formatter;
mod printer;

pub use config::{BraceStyle, FormatConfig};
pub use formatter::Formatter;
pub use printer::Printer;

use crate::parser::Parser;

pub fn format_source(source: &str, config: &FormatConfig) -> Result<String, FormatError> {
    let mut parser = Parser::new(source);
    let ast = parser.parse().map_err(|e| FormatError::ParseError {
        message: format!("{:?}", e),
    })?;

    let formatter = Formatter::new(config.clone());
    let formatted = formatter.format(&ast);

    // Refuse to hand back output the compiler cannot read. `fmt -w` overwrites
    // the user's file, so a formatter bug here destroys source (issue #201:
    // stripped `///` markers turned doc text into bare words). Verifying in
    // `format_source` rather than the CLI means `--check`, `--diff` and library
    // callers are all covered. This bounds *unparseable* output only — a
    // formatter that silently drops a comment still parses fine.
    if let Err(e) = Parser::new(&formatted).parse() {
        let mut message = format!("{} ({}:{})", e, e.span.line, e.span.column);
        if let Some(code) = &e.code {
            message = format!("[{}] {}", code, message);
        }
        // `brace_style = next_line` emits a newline before `{`, which no
        // declaration header accepts, so it fails on essentially every file.
        // Naming it keeps the guard from blaming the formatter in general when
        // one known-broken option is responsible (issue #226).
        if config.brace_style == BraceStyle::NextLine {
            message.push_str(
                " — likely cause: brace_style = next_line, which emits a newline \
                 before '{' that the parser does not accept; try brace_style = \
                 same_line / السبب المرجَّح: الإعداد brace_style = next_line",
            );
        }
        return Err(FormatError::OutputNotReparsable { message });
    }

    Ok(formatted)
}

pub fn check_formatted(source: &str, config: &FormatConfig) -> Result<bool, FormatError> {
    let formatted = format_source(source, config)?;
    Ok(source == formatted)
}

pub fn show_diff(source: &str, config: &FormatConfig) -> Result<String, FormatError> {
    let formatted = format_source(source, config)?;
    Ok(diff_of(source, &formatted))
}

/// Renders the diff for output a caller already has, so a caller that formatted
/// the source itself does not pay for a second format (and, with the re-parse
/// guard, a second pair of parses).
pub fn diff_of(source: &str, formatted: &str) -> String {
    if source == formatted {
        return String::new();
    }

    let mut diff = String::new();
    let original_lines: Vec<&str> = source.lines().collect();
    let formatted_lines: Vec<&str> = formatted.lines().collect();

    diff.push_str("--- original / الأصل\n");
    diff.push_str("+++ formatted / المنسق\n");

    let max_len = original_lines.len().max(formatted_lines.len());
    for i in 0..max_len {
        let orig = original_lines.get(i);
        let form = formatted_lines.get(i);

        match (orig, form) {
            (Some(o), Some(f)) if o != f => {
                diff.push_str(&format!("-{}\n", o));
                diff.push_str(&format!("+{}\n", f));
            }
            (Some(o), None) => {
                diff.push_str(&format!("-{}\n", o));
            }
            (None, Some(f)) => {
                diff.push_str(&format!("+{}\n", f));
            }
            _ => {}
        }
    }

    diff
}

#[derive(Debug, Clone)]
pub enum FormatError {
    ParseError {
        message: String,
    },
    IoError {
        message: String,
    },
    ConfigError {
        message: String,
    },
    /// The formatter produced output that no longer parses — always a bug in the
    /// formatter, never in the user's source, which parsed on the way in.
    OutputNotReparsable {
        message: String,
    },
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::ParseError { message } => {
                write!(f, "Parse error / خطأ في التحليل: {}", message)
            }
            FormatError::IoError { message } => {
                write!(f, "I/O error / خطأ في القراءة/الكتابة: {}", message)
            }
            FormatError::ConfigError { message } => {
                write!(f, "Config error / خطأ في الإعدادات: {}", message)
            }
            FormatError::OutputNotReparsable { message } => {
                write!(
                    f,
                    "Internal formatter bug: formatted output does not re-parse; \
                     the file was left unchanged / \
                     خطأ داخلي في المنسق: الناتج المنسق لا يمكن تحليله، \
                     ولم يُعدَّل الملف: {}",
                    message
                )
            }
        }
    }
}

impl std::error::Error for FormatError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap_with_markers(source: &str) -> String {
        format!("بسم_الله\n{}\nالحمد_لله", source.trim())
    }

    #[test]
    fn test_format_simple_variable() {
        let source = wrap_with_markers("متغير س=5");
        let config = FormatConfig::default();
        let result = format_source(&source, &config).unwrap();
        assert!(result.contains("متغير س = 5"));
    }

    #[test]
    fn test_format_function() {
        let source = wrap_with_markers("دالة اختبار(أ:عدد)->عدد{أرجع أ}");
        let config = FormatConfig::default();
        let result = format_source(&source, &config).unwrap();
        assert!(result.contains("دالة اختبار(أ: عدد) -> عدد"));
        assert!(result.contains("    أرجع أ"));
    }

    #[test]
    fn test_check_formatted() {
        let formatted = wrap_with_markers("متغير س = 5");
        let config = FormatConfig::default();
        let result = format_source(&formatted, &config).unwrap();
        assert!(result.contains("متغير س = 5"));
    }
}
