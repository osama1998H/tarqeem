//! Formatter configuration
//!
//! Provides configurable formatting options with support for both
//! Arabic and English configuration keys in TOML files.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FormatConfig {
    #[serde(alias = "حجم_المسافة")]
    pub indent_size: usize,

    #[serde(alias = "استخدم_تاب")]
    pub use_tabs: bool,

    #[serde(alias = "اقصى_طول_سطر")]
    pub max_line_length: usize,

    #[serde(alias = "نمط_الأقواس")]
    pub brace_style: BraceStyle,

    #[serde(alias = "مسافة_بعد_الفاصلة")]
    pub space_after_comma: bool,

    #[serde(alias = "مسافة_حول_العمليات")]
    pub space_around_operators: bool,

    #[serde(alias = "مسافة_قبل_القوس")]
    pub space_before_brace: bool,

    #[serde(alias = "مسافة_داخل_الأقواس")]
    pub space_inside_parens: bool,

    #[serde(alias = "مسافة_بعد_النقطتين")]
    pub space_after_colon: bool,

    #[serde(alias = "أسطر_فارغة_بعد_الاستيراد")]
    pub blank_lines_after_imports: usize,

    #[serde(alias = "أسطر_فارغة_بين_الدوال")]
    pub blank_lines_between_functions: usize,

    #[serde(alias = "اقصى_أسطر_فارغة_متتالية")]
    pub max_blank_lines: usize,

    #[serde(alias = "فاصلة_عربية")]
    pub arabic_comma: bool,

    #[serde(alias = "فاصلة_منقوطة_عربية")]
    pub arabic_semicolon: bool,

    #[serde(alias = "فاصلة_نهائية")]
    pub trailing_comma: bool,

    #[serde(alias = "سطر_جديد_نهائي")]
    pub final_newline: bool,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            indent_size: 4,
            use_tabs: false,

            max_line_length: 100,

            brace_style: BraceStyle::SameLine,

            space_after_comma: true,
            space_around_operators: true,
            space_before_brace: true,
            space_inside_parens: false,
            space_after_colon: true,

            blank_lines_after_imports: 1,
            blank_lines_between_functions: 1,
            max_blank_lines: 1,

            arabic_comma: false,
            arabic_semicolon: false,

            trailing_comma: false,
            final_newline: true,
        }
    }
}

impl FormatConfig {
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).map_err(|e| ConfigError::IoError {
            message: format!("{}: {}", path.display(), e),
        })?;

        Self::from_toml(&content)
    }

    pub fn from_toml(content: &str) -> Result<Self, ConfigError> {
        toml::from_str(content).map_err(|e| ConfigError::ParseError {
            message: e.to_string(),
        })
    }

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

    pub fn indent_str(&self) -> String {
        if self.use_tabs {
            "\t".to_string()
        } else {
            " ".repeat(self.indent_size)
        }
    }

    pub fn comma(&self) -> char {
        if self.arabic_comma {
            '،'
        } else {
            ','
        }
    }

    pub fn semicolon(&self) -> char {
        if self.arabic_semicolon {
            '؛'
        } else {
            ';'
        }
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BraceStyle {
    #[default]
    #[serde(alias = "نفس_السطر")]
    SameLine,

    #[serde(alias = "سطر_جديد")]
    NextLine,
}

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
