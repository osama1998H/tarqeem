//! Error code explanation command
//!
//! Implements the `tarqeem اشرح [رمز]` command to display
//! documentation for Arabic error codes.

use crate::error::Language;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

/// Valid category letters for error codes
const VALID_CATEGORIES: &[char] = &['ق', 'ب', 'د', 'ن', 'ص', 'و', 'ت', 'ح', 'م'];

/// Arabic-Indic digits for validation
const ARABIC_DIGITS: &[char] = &['٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨', '٩'];

/// Execute the explain command
pub fn explain_command(code: String, verbose: bool, lang: Language) -> Result<(), String> {
    // Validate the error code format
    let (category, _number) = parse_error_code(&code).ok_or_else(|| {
        format_error(
            &format!(
                "Invalid error code format '{}'. Expected: [letter][4 digits], e.g., د٠٣٠١",
                code
            ),
            &format!(
                "صيغة رمز الخطأ '{}' غير صحيحة. المتوقع: [حرف][٤ أرقام]، مثال: د٠٣٠١",
                code
            ),
            lang,
        )
    })?;

    // Find the documentation file
    let doc_path = find_doc_file(category, &code)?;

    if verbose {
        let path_msg = match lang {
            Language::Arabic => format!("قراءة التوثيق من: {}", doc_path.display()),
            Language::English => format!("Reading documentation from: {}", doc_path.display()),
        };
        eprintln!("{}", path_msg.dimmed());
    }

    // Read the documentation
    let content = fs::read_to_string(&doc_path).map_err(|_| {
        format_error(
            &format!("Documentation for '{}' not available", code),
            &format!("توثيق '{}' غير متوفر", code),
            lang,
        )
    })?;

    // Display the documentation
    print_formatted_markdown(&content);

    Ok(())
}

/// Parse an error code string into (category, number)
fn parse_error_code(code: &str) -> Option<(char, String)> {
    let chars: Vec<char> = code.chars().collect();

    // Must be at least 5 characters: 1 letter + 4 digits
    if chars.len() != 5 {
        return None;
    }

    let category = chars[0];
    let number: String = chars[1..].iter().collect();

    // Validate category
    if !VALID_CATEGORIES.contains(&category) {
        return None;
    }

    // Validate all digits are Arabic-Indic numerals
    for c in chars[1..].iter() {
        if !ARABIC_DIGITS.contains(c) {
            return None;
        }
    }

    Some((category, number))
}

/// Find the documentation file for an error code
fn find_doc_file(category: char, code: &str) -> Result<PathBuf, String> {
    let filename = format!("{}.md", code);

    // Build search paths
    let mut search_paths: Vec<PathBuf> = Vec::new();

    // 1. CARGO_MANIFEST_DIR (development)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        search_paths.push(
            PathBuf::from(manifest_dir)
                .join("docs/رموز_الأخطاء")
                .join(category.to_string())
                .join(&filename),
        );
    }

    // 2. Relative to executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            search_paths.push(
                parent
                    .join("docs/رموز_الأخطاء")
                    .join(category.to_string())
                    .join(&filename),
            );
            search_paths.push(
                parent
                    .join("../docs/رموز_الأخطاء")
                    .join(category.to_string())
                    .join(&filename),
            );
        }
    }

    // 3. Current directory
    search_paths.push(
        PathBuf::from("docs/رموز_الأخطاء")
            .join(category.to_string())
            .join(&filename),
    );

    // 4. User-local path
    if let Some(home) = dirs::home_dir() {
        search_paths.push(
            home.join(".tarqeem/docs/رموز_الأخطاء")
                .join(category.to_string())
                .join(&filename),
        );
    }

    // Search for the file
    for path in &search_paths {
        if path.exists() {
            return Ok(path.clone());
        }
    }

    Err(format!(
        "Error code '{}' not found / رمز الخطأ '{}' غير موجود",
        code, code
    ))
}

/// Format error message based on language
fn format_error(en: &str, ar: &str, lang: Language) -> String {
    match lang {
        Language::Arabic => ar.to_string(),
        Language::English => en.to_string(),
    }
}

/// Print markdown content formatted for terminal
fn print_formatted_markdown(content: &str) {
    for line in content.lines() {
        if let Some(header) = line.strip_prefix("# ") {
            // Main header
            println!("{}", header.bold().cyan());
            println!("{}", "=".repeat(header.chars().count()).cyan());
        } else if let Some(header) = line.strip_prefix("## ") {
            // Section header
            println!();
            println!("{}", header.bold().yellow());
            println!("{}", "-".repeat(header.chars().count()).yellow());
        } else if let Some(header) = line.strip_prefix("### ") {
            // Subsection header
            println!();
            println!("{}", header.bold());
        } else if line.starts_with("```") {
            // Code block delimiter
            println!("{}", line.dimmed());
        } else if let Some(item) = line.strip_prefix("- ") {
            // List item
            println!("  {} {}", "•".green(), item);
        } else if let Some(item) = line.strip_prefix("* ") {
            // List item
            println!("  {} {}", "•".green(), item);
        } else if line.starts_with("| ") {
            // Table row
            println!("{}", line.dimmed());
        } else if line.trim().is_empty() {
            println!();
        } else {
            println!("{}", line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_error_code() {
        let result = parse_error_code("د٠٣٠١");
        assert!(result.is_some());
        let (cat, num) = result.unwrap();
        assert_eq!(cat, 'د');
        assert_eq!(num, "٠٣٠١");
    }

    #[test]
    fn test_parse_all_categories() {
        for cat in VALID_CATEGORIES {
            let code = format!("{}٠٠٠١", cat);
            let result = parse_error_code(&code);
            assert!(result.is_some(), "Failed to parse category: {}", cat);
        }
    }

    #[test]
    fn test_parse_invalid_category() {
        let result = parse_error_code("E0301");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_format_too_short() {
        let result = parse_error_code("د٠١");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_format_too_long() {
        let result = parse_error_code("د٠٠٠٠١");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_western_digits() {
        let result = parse_error_code("د0301");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_warning_code() {
        let result = parse_error_code("ح٠٠٠١");
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert_eq!(cat, 'ح');
    }

    #[test]
    fn test_parse_deprecated_code() {
        let result = parse_error_code("م٠٠٠١");
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert_eq!(cat, 'م');
    }
}
