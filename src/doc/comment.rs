//! Doc comment parser for Tarqeem
//!
//! This module parses documentation comments and extracts structured information
//! like parameter descriptions, return values, examples, etc.
//!
//! ## Supported Tags (Arabic and English)
//!
//! | Arabic Tag | English Tag | Description |
//! |------------|-------------|-------------|
//! | @معامل     | @param      | Parameter description |
//! | @أرجع      | @returns    | Return value description |
//! | @مثال      | @example    | Example code |
//! | @انظر      | @see        | Related items |
//! | @ملاحظة    | @note       | Important note |
//! | @تحذير     | @warning    | Warning |
//! | @منذ       | @since      | Version when added |

use super::model::{ParamDoc, ReturnDoc};

/// Parsed documentation comment
#[derive(Debug, Clone, Default)]
pub struct ParsedDocComment {
    /// Main description text
    pub description: Option<String>,
    /// Parameter documentation (@معامل / @param)
    pub params: Vec<ParamDoc>,
    /// Return value documentation (@أرجع / @returns)
    pub returns: Option<ReturnDoc>,
    /// Example code blocks (@مثال / @example)
    pub examples: Vec<String>,
    /// See also references (@انظر / @see)
    pub see_also: Vec<String>,
    /// Notes (@ملاحظة / @note)
    pub notes: Vec<String>,
    /// Warnings (@تحذير / @warning)
    pub warnings: Vec<String>,
    /// Version added (@منذ / @since)
    pub since: Option<String>,
}

/// Parser for documentation comments
pub struct DocCommentParser;

impl DocCommentParser {
    /// Parse a doc comment string and extract structured information
    pub fn parse(comment: &str) -> ParsedDocComment {
        let mut result = ParsedDocComment::default();
        let mut description_lines = Vec::new();
        let mut current_tag: Option<(&str, String)> = None;
        let mut in_example = false;
        let mut example_lines = Vec::new();

        for line in comment.lines() {
            let trimmed = line.trim();

            // Check for tags
            if let Some((tag, content)) = Self::parse_tag(trimmed) {
                // First, save any pending tag
                if let Some((prev_tag, prev_content)) = current_tag.take() {
                    Self::save_tag(&mut result, prev_tag, &prev_content);
                }

                // Handle example tag specially (multi-line)
                if tag == "مثال" || tag == "example" {
                    in_example = true;
                    if !content.is_empty() {
                        example_lines.push(content);
                    }
                    continue;
                }

                // Start new tag
                current_tag = Some((tag, content));
            } else if in_example {
                // We're in an example block
                if trimmed.starts_with('@') {
                    // New tag starts, end example
                    in_example = false;
                    if !example_lines.is_empty() {
                        result.examples.push(example_lines.join("\n"));
                        example_lines.clear();
                    }
                    // Re-parse this line
                    if let Some((tag, content)) = Self::parse_tag(trimmed) {
                        current_tag = Some((tag, content));
                    }
                } else {
                    // Continue example
                    example_lines.push(trimmed.to_string());
                }
            } else if current_tag.is_some() {
                // Continue previous tag content
                if let Some((_, ref mut content)) = current_tag {
                    if !trimmed.is_empty() {
                        if !content.is_empty() {
                            content.push(' ');
                        }
                        content.push_str(trimmed);
                    }
                }
            } else {
                // Regular description line
                if !trimmed.is_empty() {
                    description_lines.push(trimmed.to_string());
                }
            }
        }

        // Save any remaining tag
        if let Some((tag, content)) = current_tag {
            Self::save_tag(&mut result, tag, &content);
        }

        // Save any remaining example
        if !example_lines.is_empty() {
            result.examples.push(example_lines.join("\n"));
        }

        // Set description
        if !description_lines.is_empty() {
            result.description = Some(description_lines.join("\n"));
        }

        result
    }

    /// Parse a tag from a line (returns tag name and content)
    fn parse_tag(line: &str) -> Option<(&'static str, String)> {
        let trimmed = line.trim();

        // Arabic tags
        if let Some(rest) = trimmed.strip_prefix("@معامل") {
            return Some(("param", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@أرجع") {
            return Some(("returns", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@ارجع") {
            return Some(("returns", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@مثال") {
            return Some(("مثال", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@انظر") {
            return Some(("see", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@ملاحظة") {
            return Some(("note", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@تحذير") {
            return Some(("warning", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@منذ") {
            return Some(("since", rest.trim().to_string()));
        }

        // English tags
        if let Some(rest) = trimmed.strip_prefix("@param") {
            return Some(("param", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@returns") {
            return Some(("returns", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@return") {
            return Some(("returns", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@example") {
            return Some(("مثال", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@see") {
            return Some(("see", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@note") {
            return Some(("note", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@warning") {
            return Some(("warning", rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix("@since") {
            return Some(("since", rest.trim().to_string()));
        }

        None
    }

    /// Save a tag to the result
    fn save_tag(result: &mut ParsedDocComment, tag: &str, content: &str) {
        match tag {
            "param" => {
                if let Some(param) = Self::parse_param(content) {
                    result.params.push(param);
                }
            }
            "returns" => {
                result.returns = Some(Self::parse_return(content));
            }
            "see" => {
                if !content.is_empty() {
                    result.see_also.push(content.to_string());
                }
            }
            "note" => {
                if !content.is_empty() {
                    result.notes.push(content.to_string());
                }
            }
            "warning" => {
                if !content.is_empty() {
                    result.warnings.push(content.to_string());
                }
            }
            "since" => {
                if !content.is_empty() {
                    result.since = Some(content.to_string());
                }
            }
            _ => {}
        }
    }

    /// Parse a @param tag content
    /// Format: @معامل اسم - وصف
    /// or: @معامل {نوع} اسم - وصف
    fn parse_param(content: &str) -> Option<ParamDoc> {
        let content = content.trim();
        if content.is_empty() {
            return None;
        }

        // Check for type in braces: {نوع} اسم - وصف
        let (ty, rest) = if content.starts_with('{') {
            if let Some(end) = content.find('}') {
                let ty = content[1..end].trim().to_string();
                (Some(ty), content[end + 1..].trim())
            } else {
                (None, content)
            }
        } else {
            (None, content)
        };

        // Split by - or : to get name and description
        let (name, description) = if let Some(sep) = rest.find(['-', ':', '–', '—']) {
            let name = rest[..sep].trim();
            let desc = rest[sep + 1..].trim();
            // Handle multi-byte separators
            let desc = if desc.starts_with(' ') {
                desc.trim_start()
            } else {
                desc
            };
            (name.to_string(), Some(desc.to_string()))
        } else {
            // Just a name, no description
            (
                rest.split_whitespace().next().unwrap_or(rest).to_string(),
                None,
            )
        };

        Some(ParamDoc {
            name,
            ty,
            description,
            has_default: false,
        })
    }

    /// Parse a @returns tag content
    /// Format: @أرجع وصف
    /// or: @أرجع {نوع} وصف
    fn parse_return(content: &str) -> ReturnDoc {
        let content = content.trim();

        // Check for type in braces: {نوع} وصف
        let (ty, description) = if content.starts_with('{') {
            if let Some(end) = content.find('}') {
                let ty = content[1..end].trim().to_string();
                let desc = content[end + 1..].trim();
                (
                    Some(ty),
                    if desc.is_empty() {
                        None
                    } else {
                        Some(desc.to_string())
                    },
                )
            } else {
                (None, Some(content.to_string()))
            }
        } else {
            (None, Some(content.to_string()))
        };

        ReturnDoc { ty, description }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_description() {
        let comment = "دالة لحساب مجموع عددين";
        let parsed = DocCommentParser::parse(comment);

        assert_eq!(
            parsed.description,
            Some("دالة لحساب مجموع عددين".to_string())
        );
        assert!(parsed.params.is_empty());
        assert!(parsed.returns.is_none());
    }

    #[test]
    fn test_parse_with_params_arabic() {
        let comment = r#"
دالة لحساب مجموع عددين
@معامل أ - العدد الأول
@معامل ب - العدد الثاني
@أرجع مجموع العددين
"#;
        let parsed = DocCommentParser::parse(comment);

        assert!(parsed.description.is_some());
        assert_eq!(parsed.params.len(), 2);
        assert_eq!(parsed.params[0].name, "أ");
        assert_eq!(parsed.params[0].description, Some("العدد الأول".to_string()));
        assert_eq!(parsed.params[1].name, "ب");
        assert!(parsed.returns.is_some());
    }

    #[test]
    fn test_parse_with_params_english() {
        let comment = r#"
Adds two numbers together
@param a - The first number
@param b - The second number
@returns The sum of both numbers
"#;
        let parsed = DocCommentParser::parse(comment);

        assert!(parsed.description.is_some());
        assert_eq!(parsed.params.len(), 2);
        assert_eq!(parsed.params[0].name, "a");
        assert_eq!(parsed.params[1].name, "b");
        assert!(parsed.returns.is_some());
    }

    #[test]
    fn test_parse_with_typed_params() {
        let comment = r#"
@معامل {عدد} س - الإحداثي السيني
@أرجع {منطقي} صحيح إذا كان موجباً
"#;
        let parsed = DocCommentParser::parse(comment);

        assert_eq!(parsed.params.len(), 1);
        assert_eq!(parsed.params[0].name, "س");
        assert_eq!(parsed.params[0].ty, Some("عدد".to_string()));
        assert_eq!(
            parsed.params[0].description,
            Some("الإحداثي السيني".to_string())
        );

        assert!(parsed.returns.is_some());
        let returns = parsed.returns.unwrap();
        assert_eq!(returns.ty, Some("منطقي".to_string()));
        assert_eq!(returns.description, Some("صحيح إذا كان موجباً".to_string()));
    }

    #[test]
    fn test_parse_with_example() {
        let comment = r#"
دالة للجمع
@مثال
متغير نتيجة = جمع(5، 3)
اطبع(نتيجة) // 8
"#;
        let parsed = DocCommentParser::parse(comment);

        assert_eq!(parsed.examples.len(), 1);
        assert!(parsed.examples[0].contains("متغير نتيجة = جمع(5، 3)"));
    }

    #[test]
    fn test_parse_with_see_also() {
        let comment = r#"
دالة للجمع
@انظر طرح
@انظر ضرب
"#;
        let parsed = DocCommentParser::parse(comment);

        assert_eq!(parsed.see_also.len(), 2);
        assert_eq!(parsed.see_also[0], "طرح");
        assert_eq!(parsed.see_also[1], "ضرب");
    }

    #[test]
    fn test_parse_with_notes_and_warnings() {
        let comment = r#"
دالة للقسمة
@ملاحظة يجب أن يكون المقسوم عليه غير صفر
@تحذير قد ترمي استثناء في حالة القسمة على صفر
"#;
        let parsed = DocCommentParser::parse(comment);

        assert_eq!(parsed.notes.len(), 1);
        assert!(parsed.notes[0].contains("غير صفر"));
        assert_eq!(parsed.warnings.len(), 1);
        assert!(parsed.warnings[0].contains("استثناء"));
    }
}
