//! Code Formatter (trqfmt) for Tarqeem
//!
//! Provides AST-based code formatting with configurable style options.
//!
//! ## Usage
//!
//! ```bash
//! # Format a file to stdout
//! tarqeem fmt file.trq
//!
//! # Format and write back to file
//! tarqeem fmt -w file.trq
//!
//! # Check if file is formatted (without changing)
//! tarqeem fmt --check file.trq
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
    Ok(formatter.format(&ast))
}

pub fn check_formatted(source: &str, config: &FormatConfig) -> Result<bool, FormatError> {
    let formatted = format_source(source, config)?;
    Ok(source == formatted)
}

pub fn show_diff(source: &str, config: &FormatConfig) -> Result<String, FormatError> {
    let formatted = format_source(source, config)?;

    if source == formatted {
        return Ok(String::new());
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

    Ok(diff)
}

#[derive(Debug, Clone)]
pub enum FormatError {
    ParseError { message: String },
    IoError { message: String },
    ConfigError { message: String },
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
