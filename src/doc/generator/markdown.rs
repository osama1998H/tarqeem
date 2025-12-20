//! Markdown documentation generator
//!
//! Generates Markdown documentation suitable for GitHub READMEs,
//! documentation sites, and other Markdown-compatible platforms.

use super::DocGenerator;
use crate::doc::model::{ClassDoc, Documentation, FunctionDoc, InterfaceDoc, VariableDoc};
use std::io::Write;

/// Markdown documentation generator
pub struct MarkdownGenerator {
    /// Include table of contents
    pub include_toc: bool,
    /// Use Arabic headers
    pub arabic_headers: bool,
}

impl MarkdownGenerator {
    pub fn new() -> Self {
        Self {
            include_toc: true,
            arabic_headers: true,
        }
    }
}

impl Default for MarkdownGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl DocGenerator for MarkdownGenerator {
    fn generate(&self, doc: &Documentation, writer: &mut dyn Write) -> std::io::Result<()> {
        // Title
        writeln!(writer, "# {}", &doc.name)?;
        writeln!(writer)?;

        // Description
        if let Some(desc) = &doc.description {
            writeln!(writer, "{}", desc)?;
            writeln!(writer)?;
        }

        // Table of contents
        if self.include_toc {
            self.write_toc(doc, writer)?;
        }

        // Functions
        let functions: Vec<_> = doc.functions().collect();
        if !functions.is_empty() {
            let header = if self.arabic_headers {
                "الدوال / Functions"
            } else {
                "Functions"
            };
            writeln!(writer, "## {}", header)?;
            writeln!(writer)?;
            for func in functions {
                self.write_function(func, writer)?;
            }
        }

        // Classes
        let classes: Vec<_> = doc.classes().collect();
        if !classes.is_empty() {
            let header = if self.arabic_headers {
                "الأصناف / Classes"
            } else {
                "Classes"
            };
            writeln!(writer, "## {}", header)?;
            writeln!(writer)?;
            for class in classes {
                self.write_class(class, writer)?;
            }
        }

        // Interfaces
        let interfaces: Vec<_> = doc.interfaces().collect();
        if !interfaces.is_empty() {
            let header = if self.arabic_headers {
                "الواجهات / Interfaces"
            } else {
                "Interfaces"
            };
            writeln!(writer, "## {}", header)?;
            writeln!(writer)?;
            for interface in interfaces {
                self.write_interface(interface, writer)?;
            }
        }

        // Variables
        let variables: Vec<_> = doc.variables().collect();
        if !variables.is_empty() {
            let header = if self.arabic_headers {
                "المتغيرات / Variables"
            } else {
                "Variables"
            };
            writeln!(writer, "## {}", header)?;
            writeln!(writer)?;
            for var in variables {
                self.write_variable(var, writer)?;
            }
        }

        writeln!(writer)?;
        writeln!(writer, "---")?;
        writeln!(writer, "*تم إنشاؤه بواسطة trqdoc*")?;

        Ok(())
    }
}

impl MarkdownGenerator {
    fn write_toc(&self, doc: &Documentation, writer: &mut dyn Write) -> std::io::Result<()> {
        let toc_header = if self.arabic_headers {
            "المحتويات"
        } else {
            "Table of Contents"
        };
        writeln!(writer, "## {}", toc_header)?;
        writeln!(writer)?;

        let functions: Vec<_> = doc.functions().collect();
        if !functions.is_empty() {
            let header = if self.arabic_headers { "الدوال" } else { "Functions" };
            writeln!(writer, "- [{}](#الدوال--functions)", header)?;
            for func in &functions {
                writeln!(writer, "  - [`{}`](#{})", &func.name, slug(&func.name))?;
            }
        }

        let classes: Vec<_> = doc.classes().collect();
        if !classes.is_empty() {
            let header = if self.arabic_headers { "الأصناف" } else { "Classes" };
            writeln!(writer, "- [{}](#الأصناف--classes)", header)?;
            for class in &classes {
                writeln!(writer, "  - [`{}`](#{})", &class.name, slug(&class.name))?;
            }
        }

        let interfaces: Vec<_> = doc.interfaces().collect();
        if !interfaces.is_empty() {
            let header = if self.arabic_headers { "الواجهات" } else { "Interfaces" };
            writeln!(writer, "- [{}](#الواجهات--interfaces)", header)?;
            for iface in &interfaces {
                writeln!(writer, "  - [`{}`](#{})", &iface.name, slug(&iface.name))?;
            }
        }

        writeln!(writer)?;
        Ok(())
    }

    fn write_function(&self, func: &FunctionDoc, writer: &mut dyn Write) -> std::io::Result<()> {
        // Function header
        writeln!(writer, "### `{}`", &func.name)?;
        writeln!(writer)?;

        // Signature
        write!(writer, "```tarqeem\n")?;
        if func.is_async {
            write!(writer, "غير_متزامن ")?;
        }
        write!(writer, "دالة {}(", &func.name)?;
        for (i, param) in func.params.iter().enumerate() {
            if i > 0 {
                write!(writer, "، ")?;
            }
            write!(writer, "{}", &param.name)?;
            if let Some(ty) = &param.ty {
                write!(writer, ": {}", ty)?;
            }
        }
        write!(writer, ")")?;
        if let Some(ret) = &func.returns {
            if let Some(ty) = &ret.ty {
                write!(writer, " -> {}", ty)?;
            }
        }
        writeln!(writer)?;
        writeln!(writer, "```")?;
        writeln!(writer)?;

        // Description
        if let Some(desc) = &func.description {
            writeln!(writer, "{}", desc)?;
            writeln!(writer)?;
        }

        // Parameters
        if !func.params.is_empty() {
            let header = if self.arabic_headers { "**المعاملات:**" } else { "**Parameters:**" };
            writeln!(writer, "{}", header)?;
            writeln!(writer)?;
            for param in &func.params {
                write!(writer, "- `{}`", &param.name)?;
                if let Some(ty) = &param.ty {
                    write!(writer, " ({})", ty)?;
                }
                if let Some(desc) = &param.description {
                    write!(writer, " - {}", desc)?;
                }
                writeln!(writer)?;
            }
            writeln!(writer)?;
        }

        // Return value
        if let Some(ret) = &func.returns {
            if ret.description.is_some() || ret.ty.is_some() {
                let header = if self.arabic_headers {
                    "**القيمة المُرجَعة:**"
                } else {
                    "**Returns:**"
                };
                write!(writer, "{} ", header)?;
                if let Some(ty) = &ret.ty {
                    write!(writer, "`{}`", ty)?;
                }
                if let Some(desc) = &ret.description {
                    write!(writer, " - {}", desc)?;
                }
                writeln!(writer)?;
                writeln!(writer)?;
            }
        }

        // Examples
        for example in &func.examples {
            let header = if self.arabic_headers { "**مثال:**" } else { "**Example:**" };
            writeln!(writer, "{}", header)?;
            writeln!(writer)?;
            writeln!(writer, "```tarqeem")?;
            writeln!(writer, "{}", example)?;
            writeln!(writer, "```")?;
            writeln!(writer)?;
        }

        // See also
        if !func.see_also.is_empty() {
            let header = if self.arabic_headers { "**انظر أيضاً:**" } else { "**See also:**" };
            write!(writer, "{} ", header)?;
            for (i, item) in func.see_also.iter().enumerate() {
                if i > 0 {
                    write!(writer, "، ")?;
                }
                write!(writer, "`{}`", item)?;
            }
            writeln!(writer)?;
            writeln!(writer)?;
        }

        writeln!(writer, "---")?;
        writeln!(writer)?;

        Ok(())
    }

    fn write_class(&self, class: &ClassDoc, writer: &mut dyn Write) -> std::io::Result<()> {
        // Class header
        writeln!(writer, "### `{}`", &class.name)?;
        writeln!(writer)?;

        // Signature
        write!(writer, "```tarqeem\n")?;
        write!(writer, "صنف {}", &class.name)?;
        if !class.type_params.is_empty() {
            write!(writer, "<")?;
            for (i, param) in class.type_params.iter().enumerate() {
                if i > 0 {
                    write!(writer, "، ")?;
                }
                write!(writer, "{}", param)?;
            }
            write!(writer, ">")?;
        }
        if let Some(extends) = &class.extends {
            write!(writer, " يرث {}", extends)?;
        }
        if !class.implements.is_empty() {
            write!(writer, " يطبق ")?;
            for (i, iface) in class.implements.iter().enumerate() {
                if i > 0 {
                    write!(writer, "، ")?;
                }
                write!(writer, "{}", iface)?;
            }
        }
        writeln!(writer)?;
        writeln!(writer, "```")?;
        writeln!(writer)?;

        // Description
        if let Some(desc) = &class.description {
            writeln!(writer, "{}", desc)?;
            writeln!(writer)?;
        }

        // Constructor
        if let Some(ctor) = &class.constructor {
            let header = if self.arabic_headers { "#### المُنشئ" } else { "#### Constructor" };
            writeln!(writer, "{}", header)?;
            writeln!(writer)?;
            if let Some(desc) = &ctor.description {
                writeln!(writer, "{}", desc)?;
                writeln!(writer)?;
            }
            if !ctor.params.is_empty() {
                for param in &ctor.params {
                    write!(writer, "- `{}`", &param.name)?;
                    if let Some(ty) = &param.ty {
                        write!(writer, " ({})", ty)?;
                    }
                    if let Some(desc) = &param.description {
                        write!(writer, " - {}", desc)?;
                    }
                    writeln!(writer)?;
                }
                writeln!(writer)?;
            }
        }

        // Fields
        if !class.fields.is_empty() {
            let header = if self.arabic_headers { "#### الحقول" } else { "#### Fields" };
            writeln!(writer, "{}", header)?;
            writeln!(writer)?;
            writeln!(writer, "| الاسم | النوع | الوصف |")?;
            writeln!(writer, "|-------|-------|-------|")?;
            for field in &class.fields {
                let ty = field.ty.as_deref().unwrap_or("-");
                let desc = field.description.as_deref().unwrap_or("-");
                let visibility = format!("({}) ", &field.visibility);
                writeln!(writer, "| {}`{}` | `{}` | {} |", visibility, &field.name, ty, desc)?;
            }
            writeln!(writer)?;
        }

        // Methods
        if !class.methods.is_empty() {
            let header = if self.arabic_headers { "#### الدوال" } else { "#### Methods" };
            writeln!(writer, "{}", header)?;
            writeln!(writer)?;
            for method in &class.methods {
                write!(writer, "- **`{}`** ", &method.name)?;
                write!(writer, "(")?;
                for (i, param) in method.params.iter().enumerate() {
                    if i > 0 {
                        write!(writer, "، ")?;
                    }
                    write!(writer, "{}", &param.name)?;
                    if let Some(ty) = &param.ty {
                        write!(writer, ": {}", ty)?;
                    }
                }
                write!(writer, ")")?;
                if let Some(ret) = &method.returns {
                    if let Some(ty) = &ret.ty {
                        write!(writer, " -> {}", ty)?;
                    }
                }
                writeln!(writer)?;
                if let Some(desc) = &method.description {
                    writeln!(writer, "  - {}", desc)?;
                }
            }
            writeln!(writer)?;
        }

        writeln!(writer, "---")?;
        writeln!(writer)?;

        Ok(())
    }

    fn write_interface(
        &self,
        interface: &InterfaceDoc,
        writer: &mut dyn Write,
    ) -> std::io::Result<()> {
        // Interface header
        writeln!(writer, "### `{}`", &interface.name)?;
        writeln!(writer)?;

        // Signature
        write!(writer, "```tarqeem\n")?;
        write!(writer, "واجهة {}", &interface.name)?;
        if !interface.type_params.is_empty() {
            write!(writer, "<")?;
            for (i, param) in interface.type_params.iter().enumerate() {
                if i > 0 {
                    write!(writer, "، ")?;
                }
                write!(writer, "{}", param)?;
            }
            write!(writer, ">")?;
        }
        writeln!(writer)?;
        writeln!(writer, "```")?;
        writeln!(writer)?;

        // Description
        if let Some(desc) = &interface.description {
            writeln!(writer, "{}", desc)?;
            writeln!(writer)?;
        }

        // Methods
        if !interface.methods.is_empty() {
            let header = if self.arabic_headers { "#### الدوال" } else { "#### Methods" };
            writeln!(writer, "{}", header)?;
            writeln!(writer)?;
            for method in &interface.methods {
                write!(writer, "- `دالة {}(", &method.name)?;
                for (i, param) in method.params.iter().enumerate() {
                    if i > 0 {
                        write!(writer, "، ")?;
                    }
                    write!(writer, "{}", &param.name)?;
                    if let Some(ty) = &param.ty {
                        write!(writer, ": {}", ty)?;
                    }
                }
                write!(writer, ")")?;
                if let Some(ret_ty) = &method.return_type {
                    write!(writer, " -> {}", ret_ty)?;
                }
                writeln!(writer, "`")?;
                if let Some(desc) = &method.description {
                    writeln!(writer, "  - {}", desc)?;
                }
            }
            writeln!(writer)?;
        }

        writeln!(writer, "---")?;
        writeln!(writer)?;

        Ok(())
    }

    fn write_variable(&self, var: &VariableDoc, writer: &mut dyn Write) -> std::io::Result<()> {
        writeln!(writer, "### `{}`", &var.name)?;
        writeln!(writer)?;

        let keyword = if var.is_mutable { "متغير" } else { "ثابت" };
        write!(writer, "```tarqeem\n")?;
        write!(writer, "{} {}", keyword, &var.name)?;
        if let Some(ty) = &var.ty {
            write!(writer, ": {}", ty)?;
        }
        writeln!(writer)?;
        writeln!(writer, "```")?;
        writeln!(writer)?;

        if let Some(desc) = &var.description {
            writeln!(writer, "{}", desc)?;
            writeln!(writer)?;
        }

        writeln!(writer, "---")?;
        writeln!(writer)?;

        Ok(())
    }
}

/// Create a slug from text for anchor links
fn slug(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || c.is_whitespace())
        .map(|c| if c.is_whitespace() { '-' } else { c })
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug() {
        assert_eq!(slug("Hello World"), "hello-world");
        assert_eq!(slug("مرحبا"), "مرحبا");
    }

    #[test]
    fn test_generate_empty_doc() {
        let doc = Documentation::new("test".to_string(), "test.trq".to_string());
        let generator = MarkdownGenerator::new();
        let mut output = Vec::new();
        generator.generate(&doc, &mut output).unwrap();
        let md = String::from_utf8(output).unwrap();

        assert!(md.contains("# test"));
        assert!(md.contains("trqdoc"));
    }
}
