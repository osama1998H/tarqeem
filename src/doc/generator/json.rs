//! JSON documentation generator
//!
//! Generates machine-readable JSON documentation that can be consumed
//! by other tools, search indexes, or documentation browsers.

use super::DocGenerator;
use crate::doc::model::Documentation;
use std::io::Write;

/// JSON documentation generator
pub struct JsonGenerator {
    /// Pretty print the output
    pub pretty: bool,
}

impl JsonGenerator {
    pub fn new() -> Self {
        Self { pretty: true }
    }

    /// Create a minified JSON generator
    pub fn minified() -> Self {
        Self { pretty: false }
    }
}

impl Default for JsonGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl DocGenerator for JsonGenerator {
    fn generate(&self, doc: &Documentation, writer: &mut dyn Write) -> std::io::Result<()> {
        let json = if self.pretty {
            serde_json::to_string_pretty(doc)
        } else {
            serde_json::to_string(doc)
        };

        match json {
            Ok(s) => {
                writeln!(writer, "{}", s)?;
                Ok(())
            }
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::model::{DocItem, FunctionDoc, ParamDoc};

    #[test]
    fn test_generate_json() {
        let mut doc = Documentation::new("test".to_string(), "test.trq".to_string());

        let mut func = FunctionDoc::new("جمع".to_string());
        func.description = Some("جمع عددين".to_string());
        func.params.push(ParamDoc {
            name: "أ".to_string(),
            ty: Some("عدد".to_string()),
            description: Some("العدد الأول".to_string()),
            has_default: false,
        });
        doc.items.push(DocItem::Function(func));

        let generator = JsonGenerator::new();
        let mut output = Vec::new();
        generator.generate(&doc, &mut output).unwrap();
        let json = String::from_utf8(output).unwrap();

        assert!(json.contains("\"name\": \"test\""));
        assert!(json.contains("\"جمع\""));
        assert!(json.contains("جمع عددين")); // Part of the description value
    }

    #[test]
    fn test_generate_minified_json() {
        let doc = Documentation::new("test".to_string(), "test.trq".to_string());

        let generator = JsonGenerator::minified();
        let mut output = Vec::new();
        generator.generate(&doc, &mut output).unwrap();
        let json = String::from_utf8(output).unwrap();

        // Minified JSON should be on a single line (plus newline at end)
        assert_eq!(json.lines().count(), 1);
    }
}
