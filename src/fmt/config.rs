//! Formatter configuration
//!
//! Provides configurable formatting options with support for both
//! Arabic and English configuration keys in TOML files.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Formatter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FormatConfig {
    // === Indentation ===
    /// Number of spaces per indentation level
    #[serde(alias = "حجم_المسافة")]
    pub indent_size: usize,

    /// Use tabs instead of spaces
    #[serde(alias = "استخدم_تاب")]
    pub use_tabs: bool,

    // === Line length ===
    /// Maximum line length before wrapping
    #[serde(alias = "اقصى_طول_سطر")]
    pub max_line_length: usize,

    // === Braces ===
    /// Brace style (same_line or next_line)
    #[serde(alias = "نمط_الأقواس")]
    pub brace_style: BraceStyle,

    // === Spacing ===
    /// Add space after comma
    #[serde(alias = "مسافة_بعد_الفاصلة")]
    pub space_after_comma: bool,

    /// Add space around binary operators
    #[serde(alias = "مسافة_حول_العمليات")]
    pub space_around_operators: bool,

    /// Add space before opening brace
    #[serde(alias = "مسافة_قبل_القوس")]
    pub space_before_brace: bool,

    /// Add space inside parentheses
    #[serde(alias = "مسافة_داخل_الأقواس")]
    pub space_inside_parens: bool,

    /// Add space after colon in type annotations
    #[serde(alias = "مسافة_بعد_النقطتين")]
    pub space_after_colon: bool,

    // === Blank lines ===
    /// Number of blank lines after import statements
    #[serde(alias = "أسطر_فارغة_بعد_الاستيراد")]
    pub blank_lines_after_imports: usize,

    /// Number of blank lines between top-level declarations
    #[serde(alias = "أسطر_فارغة_بين_الدوال")]
    pub blank_lines_between_functions: usize,

    /// Maximum consecutive blank lines allowed inside blocks
    #[serde(alias = "اقصى_أسطر_فارغة_متتالية")]
    pub max_blank_lines: usize,

    // === Arabic-specific ===
    /// Use Arabic comma (،) instead of ASCII comma (,)
    #[serde(alias = "فاصلة_عربية")]
    pub arabic_comma: bool,

    /// Use Arabic semicolon (؛) instead of ASCII semicolon (;)
    #[serde(alias = "فاصلة_منقوطة_عربية")]
    pub arabic_semicolon: bool,

    // === Trailing ===
    /// Add trailing comma in multi-line constructs
    #[serde(alias = "فاصلة_نهائية")]
    pub trailing_comma: bool,

    /// Ensure file ends with newline
    #[serde(alias = "سطر_جديد_نهائي")]
    pub final_newline: bool,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            // Indentation
            indent_size: 4,
            use_tabs: false,

            // Line length
            max_line_length: 100,

            // Braces
            brace_style: BraceStyle::SameLine,

            // Spacing
            space_after_comma: true,
            space_around_operators: true,
            space_before_brace: true,
            space_inside_parens: false,
            space_after_colon: true,

            // Blank lines
            blank_lines_after_imports: 1,
            blank_lines_between_functions: 1,
            max_blank_lines: 1,

            // Arabic-specific
            arabic_comma: false,
            arabic_semicolon: false,

            // Trailing
            trailing_comma: false,
            final_newline: true,
        }
    }
}

impl FormatConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).map_err(|e| ConfigError::IoError {
            message: format!("{}: {}", path.display(), e),
        })?;

        Self::from_toml(&content)
    }

    /// Parse configuration from TOML string
    pub fn from_toml(content: &str) -> Result<Self, ConfigError> {
        toml::from_str(content).map_err(|e| ConfigError::ParseError {
            message: e.to_string(),
        })
    }

    /// Find and load configuration from the filesystem
    ///
    /// Searches for configuration files in this order:
    /// 1. .trqfmt.toml in current directory
    /// 2. trqfmt.toml in current directory
    /// 3. تنسيق.toml in current directory
    /// 4. Parent directories (up to root)
    pub fn find_and_load() -> Option<Self> {
        let config_names = [".trqfmt.toml", "trqfmt.toml", "تنسيق.toml"];

        let mut current = std::env::current_dir().ok()?;

        loop {
            for name in &config_names {
                let config_path = current.join(name);
                if config_path.exists() {
                    return Self::from_file(&config_path).ok();
                }
            }

            if !current.pop() {
                break;
            }
        }

        None
    }

    /// Get the indent string based on configuration
    pub fn indent_str(&self) -> String {
        if self.use_tabs {
            "\t".to_string()
        } else {
            " ".repeat(self.indent_size)
        }
    }

    /// Get the comma character based on configuration
    pub fn comma(&self) -> char {
        if self.arabic_comma {
            '،'
        } else {
            ','
        }
    }

    /// Get the semicolon character based on configuration
    pub fn semicolon(&self) -> char {
        if self.arabic_semicolon {
            '؛'
        } else {
            ';'
        }
    }

    /// Generate a sample configuration file
    pub fn sample_config() -> String {
        r#"# Tarqeem Formatter Configuration / إعدادات منسق ترقيم
# .trqfmt.toml

# === Indentation / المسافات البادئة ===

# Number of spaces per indentation level
# حجم المسافة البادئة
indent_size = 4
# حجم_المسافة = 4

# Use tabs instead of spaces
# استخدام التاب بدلاً من المسافات
use_tabs = false
# استخدم_تاب = false

# === Line Length / طول السطر ===

# Maximum line length before wrapping
# أقصى طول للسطر قبل التفاف
max_line_length = 100
# اقصى_طول_سطر = 100

# === Braces / الأقواس ===

# Brace style: "same_line" or "next_line"
# نمط الأقواس: "same_line" أو "next_line"
brace_style = "same_line"
# نمط_الأقواس = "نفس_السطر"

# === Spacing / المسافات ===

# Add space after comma
space_after_comma = true
# مسافة_بعد_الفاصلة = true

# Add space around operators (+, -, *, /, =, etc.)
space_around_operators = true
# مسافة_حول_العمليات = true

# Add space before opening brace
space_before_brace = true
# مسافة_قبل_القوس = true

# Add space after colon in type annotations
space_after_colon = true
# مسافة_بعد_النقطتين = true

# === Blank Lines / الأسطر الفارغة ===

# Blank lines after import statements
blank_lines_after_imports = 1
# أسطر_فارغة_بعد_الاستيراد = 1

# Blank lines between top-level declarations
blank_lines_between_functions = 1
# أسطر_فارغة_بين_الدوال = 1

# Maximum consecutive blank lines allowed
max_blank_lines = 1
# اقصى_أسطر_فارغة_متتالية = 1

# === Arabic-specific / خاص بالعربية ===

# Use Arabic comma (،) instead of ASCII comma (,)
arabic_comma = false
# فاصلة_عربية = false

# Use Arabic semicolon (؛) instead of ASCII semicolon (;)
arabic_semicolon = false
# فاصلة_منقوطة_عربية = false

# === Trailing / النهايات ===

# Add trailing comma in multi-line constructs
trailing_comma = false
# فاصلة_نهائية = false

# Ensure file ends with newline
final_newline = true
# سطر_جديد_نهائي = true
"#
        .to_string()
    }
}

/// Brace style options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BraceStyle {
    /// Opening brace on same line as declaration
    /// `دالة اختبار() {`
    #[default]
    #[serde(alias = "نفس_السطر")]
    SameLine,

    /// Opening brace on next line
    /// ```text
    /// دالة اختبار()
    /// {
    /// ```
    #[serde(alias = "سطر_جديد")]
    NextLine,
}

/// Configuration error
#[derive(Debug, Clone)]
pub enum ConfigError {
    IoError { message: String },
    ParseError { message: String },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError { message } => {
                write!(f, "I/O error / خطأ في القراءة: {}", message)
            }
            ConfigError::ParseError { message } => {
                write!(f, "Parse error / خطأ في التحليل: {}", message)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = FormatConfig::default();
        assert_eq!(config.indent_size, 4);
        assert!(!config.use_tabs);
        assert_eq!(config.max_line_length, 100);
        assert!(config.space_after_comma);
        assert!(config.space_around_operators);
    }

    #[test]
    fn test_config_from_toml() {
        let toml = r#"
indent_size = 2
use_tabs = true
arabic_comma = true
"#;
        let config = FormatConfig::from_toml(toml).unwrap();
        assert_eq!(config.indent_size, 2);
        assert!(config.use_tabs);
        assert!(config.arabic_comma);
    }

    #[test]
    fn test_arabic_config_aliases() {
        // Note: TOML requires quoted keys for non-ASCII characters
        let toml = r#"
"حجم_المسافة" = 2
"استخدم_تاب" = true
"فاصلة_عربية" = true
"#;
        let config = FormatConfig::from_toml(toml).unwrap();
        assert_eq!(config.indent_size, 2);
        assert!(config.use_tabs);
        assert!(config.arabic_comma);
    }

    #[test]
    fn test_indent_str() {
        let mut config = FormatConfig::default();
        assert_eq!(config.indent_str(), "    ");

        config.use_tabs = true;
        assert_eq!(config.indent_str(), "\t");

        config.use_tabs = false;
        config.indent_size = 2;
        assert_eq!(config.indent_str(), "  ");
    }

    #[test]
    fn test_comma_char() {
        let mut config = FormatConfig::default();
        assert_eq!(config.comma(), ',');

        config.arabic_comma = true;
        assert_eq!(config.comma(), '،');
    }
}
