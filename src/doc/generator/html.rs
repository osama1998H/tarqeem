//! HTML documentation generator with RTL support
//!
//! Generates HTML documentation with proper Arabic RTL text handling
//! and a bilingual interface.

use super::DocGenerator;
use crate::doc::model::{
    ClassDoc, Documentation, FieldDoc, FunctionDoc, InterfaceDoc, MethodDoc, VariableDoc,
};
use std::io::Write;

pub struct HtmlGenerator {
    pub include_nav: bool,
    pub include_search: bool,
    pub custom_css: Option<String>,
}

impl HtmlGenerator {
    pub fn new() -> Self {
        Self {
            include_nav: true,
            include_search: false,
            custom_css: None,
        }
    }
}

impl Default for HtmlGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl DocGenerator for HtmlGenerator {
    fn generate(&self, doc: &Documentation, writer: &mut dyn Write) -> std::io::Result<()> {
        self.write_header(doc, writer)?;

        if self.include_nav {
            self.write_nav(doc, writer)?;
        }

        writeln!(writer, "<main class=\"content\">")?;

        writeln!(writer, "<div class=\"module-header\">")?;
        writeln!(writer, "<h1>{}</h1>", html_escape(&doc.name))?;
        if let Some(desc) = &doc.description {
            writeln!(writer, "<p class=\"description\">{}</p>", html_escape(desc))?;
        }
        writeln!(writer, "</div>")?;

        let functions: Vec<_> = doc.functions().collect();
        if !functions.is_empty() {
            writeln!(writer, "<section id=\"functions\">")?;
            writeln!(writer, "<h2>الدوال / Functions</h2>")?;
            for func in functions {
                self.write_function(func, writer)?;
            }
            writeln!(writer, "</section>")?;
        }

        let classes: Vec<_> = doc.classes().collect();
        if !classes.is_empty() {
            writeln!(writer, "<section id=\"classes\">")?;
            writeln!(writer, "<h2>الأصناف / Classes</h2>")?;
            for class in classes {
                self.write_class(class, writer)?;
            }
            writeln!(writer, "</section>")?;
        }

        let interfaces: Vec<_> = doc.interfaces().collect();
        if !interfaces.is_empty() {
            writeln!(writer, "<section id=\"interfaces\">")?;
            writeln!(writer, "<h2>الواجهات / Interfaces</h2>")?;
            for interface in interfaces {
                self.write_interface(interface, writer)?;
            }
            writeln!(writer, "</section>")?;
        }

        let variables: Vec<_> = doc.variables().collect();
        if !variables.is_empty() {
            writeln!(writer, "<section id=\"variables\">")?;
            writeln!(writer, "<h2>المتغيرات / Variables</h2>")?;
            for var in variables {
                self.write_variable(var, writer)?;
            }
            writeln!(writer, "</section>")?;
        }

        writeln!(writer, "</main>")?;

        self.write_footer(writer)?;

        Ok(())
    }
}

impl HtmlGenerator {
    fn write_header(&self, doc: &Documentation, writer: &mut dyn Write) -> std::io::Result<()> {
        writeln!(writer, "<!DOCTYPE html>")?;
        writeln!(writer, "<html lang=\"ar\" dir=\"rtl\">")?;
        writeln!(writer, "<head>")?;
        writeln!(writer, "<meta charset=\"UTF-8\">")?;
        writeln!(
            writer,
            "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
        )?;
        writeln!(
            writer,
            "<title>{} - توثيق ترقيم</title>",
            html_escape(&doc.name)
        )?;
        writeln!(writer, "<style>{}</style>", self.get_css())?;
        if let Some(custom) = &self.custom_css {
            writeln!(writer, "<style>{}</style>", custom)?;
        }
        writeln!(writer, "</head>")?;
        writeln!(writer, "<body>")?;
        Ok(())
    }

    fn write_footer(&self, writer: &mut dyn Write) -> std::io::Result<()> {
        writeln!(writer, "<footer>")?;
        writeln!(writer, "<p>تم إنشاؤه بواسطة trqdoc - مولد توثيق ترقيم</p>")?;
        writeln!(writer, "</footer>")?;
        writeln!(writer, "</body>")?;
        writeln!(writer, "</html>")?;
        Ok(())
    }

    fn write_nav(&self, doc: &Documentation, writer: &mut dyn Write) -> std::io::Result<()> {
        writeln!(writer, "<nav class=\"sidebar\">")?;
        writeln!(writer, "<div class=\"nav-header\">")?;
        writeln!(writer, "<h2>المحتويات</h2>")?;
        writeln!(writer, "</div>")?;
        writeln!(writer, "<ul>")?;

        let functions: Vec<_> = doc.functions().collect();
        if !functions.is_empty() {
            writeln!(writer, "<li class=\"nav-section\">")?;
            writeln!(writer, "<a href=\"#functions\">الدوال</a>")?;
            writeln!(writer, "<ul>")?;
            for func in functions {
                writeln!(
                    writer,
                    "<li><a href=\"#func-{}\">{}</a></li>",
                    html_escape(&func.name),
                    html_escape(&func.name)
                )?;
            }
            writeln!(writer, "</ul>")?;
            writeln!(writer, "</li>")?;
        }

        let classes: Vec<_> = doc.classes().collect();
        if !classes.is_empty() {
            writeln!(writer, "<li class=\"nav-section\">")?;
            writeln!(writer, "<a href=\"#classes\">الأصناف</a>")?;
            writeln!(writer, "<ul>")?;
            for class in classes {
                writeln!(
                    writer,
                    "<li><a href=\"#class-{}\">{}</a></li>",
                    html_escape(&class.name),
                    html_escape(&class.name)
                )?;
            }
            writeln!(writer, "</ul>")?;
            writeln!(writer, "</li>")?;
        }

        let interfaces: Vec<_> = doc.interfaces().collect();
        if !interfaces.is_empty() {
            writeln!(writer, "<li class=\"nav-section\">")?;
            writeln!(writer, "<a href=\"#interfaces\">الواجهات</a>")?;
            writeln!(writer, "<ul>")?;
            for interface in interfaces {
                writeln!(
                    writer,
                    "<li><a href=\"#interface-{}\">{}</a></li>",
                    html_escape(&interface.name),
                    html_escape(&interface.name)
                )?;
            }
            writeln!(writer, "</ul>")?;
            writeln!(writer, "</li>")?;
        }

        writeln!(writer, "</ul>")?;
        writeln!(writer, "</nav>")?;
        Ok(())
    }

    fn write_function(&self, func: &FunctionDoc, writer: &mut dyn Write) -> std::io::Result<()> {
        writeln!(
            writer,
            "<article class=\"doc-item function\" id=\"func-{}\">",
            html_escape(&func.name)
        )?;
        writeln!(writer, "<header>")?;

        write!(writer, "<h3 class=\"name\">")?;
        if func.is_async {
            write!(writer, "<span class=\"modifier\">متوازي</span> ")?;
        }
        write!(writer, "<span class=\"keyword\">دالة</span> ")?;
        write!(
            writer,
            "<span class=\"func-name\">{}</span>(",
            html_escape(&func.name)
        )?;

        for (i, param) in func.params.iter().enumerate() {
            if i > 0 {
                write!(writer, "، ")?;
            }
            write!(writer, "{}", html_escape(&param.name))?;
            if let Some(ty) = &param.ty {
                write!(writer, ": <span class=\"type\">{}</span>", html_escape(ty))?;
            }
        }
        write!(writer, ")")?;

        if let Some(ret) = &func.returns {
            if let Some(ty) = &ret.ty {
                write!(
                    writer,
                    " -&gt; <span class=\"type\">{}</span>",
                    html_escape(ty)
                )?;
            }
        }
        writeln!(writer, "</h3>")?;

        if func.is_exported {
            writeln!(writer, "<span class=\"badge exported\">مُصدَّر</span>")?;
        }
        writeln!(writer, "</header>")?;

        if let Some(desc) = &func.description {
            writeln!(
                writer,
                "<div class=\"description\">{}</div>",
                html_escape(desc)
            )?;
        }

        if !func.params.is_empty() {
            writeln!(writer, "<div class=\"params\">")?;
            writeln!(writer, "<h4>المعاملات:</h4>")?;
            writeln!(writer, "<dl>")?;
            for param in &func.params {
                writeln!(writer, "<dt>")?;
                write!(writer, "<code>{}</code>", html_escape(&param.name))?;
                if let Some(ty) = &param.ty {
                    write!(writer, " <span class=\"type\">({})</span>", html_escape(ty))?;
                }
                writeln!(writer, "</dt>")?;
                if let Some(desc) = &param.description {
                    writeln!(writer, "<dd>{}</dd>", html_escape(desc))?;
                }
            }
            writeln!(writer, "</dl>")?;
            writeln!(writer, "</div>")?;
        }

        if let Some(ret) = &func.returns {
            if ret.description.is_some() || ret.ty.is_some() {
                writeln!(writer, "<div class=\"returns\">")?;
                writeln!(writer, "<h4>القيمة المُرجَعة:</h4>")?;
                if let Some(ty) = &ret.ty {
                    write!(writer, "<span class=\"type\">{}</span>", html_escape(ty))?;
                }
                if let Some(desc) = &ret.description {
                    write!(writer, " - {}", html_escape(desc))?;
                }
                writeln!(writer, "</div>")?;
            }
        }

        for example in &func.examples {
            writeln!(writer, "<div class=\"example\">")?;
            writeln!(writer, "<h4>مثال:</h4>")?;
            writeln!(writer, "<pre><code>{}</code></pre>", html_escape(example))?;
            writeln!(writer, "</div>")?;
        }

        writeln!(writer, "</article>")?;
        Ok(())
    }

    fn write_class(&self, class: &ClassDoc, writer: &mut dyn Write) -> std::io::Result<()> {
        writeln!(
            writer,
            "<article class=\"doc-item class\" id=\"class-{}\">",
            html_escape(&class.name)
        )?;
        writeln!(writer, "<header>")?;

        write!(writer, "<h3 class=\"name\">")?;
        write!(writer, "<span class=\"keyword\">صنف</span> ")?;
        write!(
            writer,
            "<span class=\"class-name\">{}</span>",
            html_escape(&class.name)
        )?;

        if !class.type_params.is_empty() {
            write!(writer, "&lt;")?;
            for (i, param) in class.type_params.iter().enumerate() {
                if i > 0 {
                    write!(writer, "، ")?;
                }
                write!(writer, "{}", html_escape(param))?;
            }
            write!(writer, "&gt;")?;
        }

        if let Some(extends) = &class.extends {
            write!(
                writer,
                " <span class=\"keyword\">يرث</span> {}",
                html_escape(extends)
            )?;
        }

        if !class.implements.is_empty() {
            write!(writer, " <span class=\"keyword\">يلتزم</span> ")?;
            for (i, iface) in class.implements.iter().enumerate() {
                if i > 0 {
                    write!(writer, "، ")?;
                }
                write!(writer, "{}", html_escape(iface))?;
            }
        }
        writeln!(writer, "</h3>")?;

        if class.is_exported {
            writeln!(writer, "<span class=\"badge exported\">مُصدَّر</span>")?;
        }
        writeln!(writer, "</header>")?;

        if let Some(desc) = &class.description {
            writeln!(
                writer,
                "<div class=\"description\">{}</div>",
                html_escape(desc)
            )?;
        }

        if let Some(ctor) = &class.constructor {
            writeln!(writer, "<div class=\"constructor\">")?;
            writeln!(writer, "<h4>المُنشئ:</h4>")?;
            if let Some(desc) = &ctor.description {
                writeln!(writer, "<p>{}</p>", html_escape(desc))?;
            }
            if !ctor.params.is_empty() {
                writeln!(writer, "<dl>")?;
                for param in &ctor.params {
                    writeln!(writer, "<dt><code>{}</code></dt>", html_escape(&param.name))?;
                    if let Some(desc) = &param.description {
                        writeln!(writer, "<dd>{}</dd>", html_escape(desc))?;
                    }
                }
                writeln!(writer, "</dl>")?;
            }
            writeln!(writer, "</div>")?;
        }

        if !class.fields.is_empty() {
            writeln!(writer, "<div class=\"fields\">")?;
            writeln!(writer, "<h4>الحقول:</h4>")?;
            writeln!(writer, "<table>")?;
            writeln!(
                writer,
                "<thead><tr><th>الاسم</th><th>النوع</th><th>الوصف</th></tr></thead>"
            )?;
            writeln!(writer, "<tbody>")?;
            for field in &class.fields {
                self.write_field_row(field, writer)?;
            }
            writeln!(writer, "</tbody></table>")?;
            writeln!(writer, "</div>")?;
        }

        if !class.methods.is_empty() {
            writeln!(writer, "<div class=\"methods\">")?;
            writeln!(writer, "<h4>الدوال:</h4>")?;
            for method in &class.methods {
                self.write_method(method, writer)?;
            }
            writeln!(writer, "</div>")?;
        }

        writeln!(writer, "</article>")?;
        Ok(())
    }

    fn write_field_row(&self, field: &FieldDoc, writer: &mut dyn Write) -> std::io::Result<()> {
        writeln!(writer, "<tr>")?;
        write!(
            writer,
            "<td><span class=\"visibility {}\">{}</span> <code>{}</code>",
            field.visibility,
            field.visibility,
            html_escape(&field.name)
        )?;
        if field.is_static {
            write!(writer, " <span class=\"badge static\">مشترك</span>")?;
        }
        writeln!(writer, "</td>")?;
        writeln!(
            writer,
            "<td><span class=\"type\">{}</span></td>",
            field.ty.as_deref().map(html_escape).unwrap_or_default()
        )?;
        writeln!(
            writer,
            "<td>{}</td>",
            field
                .description
                .as_deref()
                .map(html_escape)
                .unwrap_or_default()
        )?;
        writeln!(writer, "</tr>")?;
        Ok(())
    }

    fn write_method(&self, method: &MethodDoc, writer: &mut dyn Write) -> std::io::Result<()> {
        writeln!(writer, "<div class=\"method\">")?;
        write!(writer, "<h5>")?;
        write!(
            writer,
            "<span class=\"visibility {}\">{}</span> ",
            method.visibility, method.visibility
        )?;
        if method.is_static {
            write!(writer, "<span class=\"modifier\">مشترك</span> ")?;
        }
        if method.is_async {
            write!(writer, "<span class=\"modifier\">متوازي</span> ")?;
        }
        write!(writer, "<span class=\"keyword\">دالة</span> ")?;
        write!(
            writer,
            "<span class=\"method-name\">{}</span>(",
            html_escape(&method.name)
        )?;

        for (i, param) in method.params.iter().enumerate() {
            if i > 0 {
                write!(writer, "، ")?;
            }
            write!(writer, "{}", html_escape(&param.name))?;
            if let Some(ty) = &param.ty {
                write!(writer, ": <span class=\"type\">{}</span>", html_escape(ty))?;
            }
        }
        write!(writer, ")")?;

        if let Some(ret) = &method.returns {
            if let Some(ty) = &ret.ty {
                write!(
                    writer,
                    " -&gt; <span class=\"type\">{}</span>",
                    html_escape(ty)
                )?;
            }
        }
        writeln!(writer, "</h5>")?;

        if let Some(desc) = &method.description {
            writeln!(writer, "<p>{}</p>", html_escape(desc))?;
        }
        writeln!(writer, "</div>")?;
        Ok(())
    }

    fn write_interface(
        &self,
        interface: &InterfaceDoc,
        writer: &mut dyn Write,
    ) -> std::io::Result<()> {
        writeln!(
            writer,
            "<article class=\"doc-item interface\" id=\"interface-{}\">",
            html_escape(&interface.name)
        )?;
        writeln!(writer, "<header>")?;

        write!(writer, "<h3 class=\"name\">")?;
        write!(writer, "<span class=\"keyword\">ميثاق</span> ")?;
        write!(
            writer,
            "<span class=\"interface-name\">{}</span>",
            html_escape(&interface.name)
        )?;

        if !interface.type_params.is_empty() {
            write!(writer, "&lt;")?;
            for (i, param) in interface.type_params.iter().enumerate() {
                if i > 0 {
                    write!(writer, "، ")?;
                }
                write!(writer, "{}", html_escape(param))?;
            }
            write!(writer, "&gt;")?;
        }
        writeln!(writer, "</h3>")?;

        if interface.is_exported {
            writeln!(writer, "<span class=\"badge exported\">مُصدَّر</span>")?;
        }
        writeln!(writer, "</header>")?;

        if let Some(desc) = &interface.description {
            writeln!(
                writer,
                "<div class=\"description\">{}</div>",
                html_escape(desc)
            )?;
        }

        if !interface.methods.is_empty() {
            writeln!(writer, "<div class=\"methods\">")?;
            writeln!(writer, "<h4>الدوال:</h4>")?;
            for method in &interface.methods {
                writeln!(writer, "<div class=\"method-signature\">")?;
                write!(writer, "<code>دالة {}(", html_escape(&method.name))?;
                for (i, param) in method.params.iter().enumerate() {
                    if i > 0 {
                        write!(writer, "، ")?;
                    }
                    write!(writer, "{}", html_escape(&param.name))?;
                    if let Some(ty) = &param.ty {
                        write!(writer, ": {}", html_escape(ty))?;
                    }
                }
                write!(writer, ")")?;
                if let Some(ret_ty) = &method.return_type {
                    write!(writer, " -&gt; {}", html_escape(ret_ty))?;
                }
                writeln!(writer, "</code>")?;
                if let Some(desc) = &method.description {
                    writeln!(writer, "<p>{}</p>", html_escape(desc))?;
                }
                writeln!(writer, "</div>")?;
            }
            writeln!(writer, "</div>")?;
        }

        writeln!(writer, "</article>")?;
        Ok(())
    }

    fn write_variable(&self, var: &VariableDoc, writer: &mut dyn Write) -> std::io::Result<()> {
        writeln!(
            writer,
            "<article class=\"doc-item variable\" id=\"var-{}\">",
            html_escape(&var.name)
        )?;
        writeln!(writer, "<header>")?;

        write!(writer, "<h3 class=\"name\">")?;
        let keyword = if var.is_mutable {
            "متغير"
        } else {
            "ثابت"
        };
        write!(writer, "<span class=\"keyword\">{}</span> ", keyword)?;
        write!(
            writer,
            "<span class=\"var-name\">{}</span>",
            html_escape(&var.name)
        )?;
        if let Some(ty) = &var.ty {
            write!(writer, ": <span class=\"type\">{}</span>", html_escape(ty))?;
        }
        writeln!(writer, "</h3>")?;

        if var.is_exported {
            writeln!(writer, "<span class=\"badge exported\">مُصدَّر</span>")?;
        }
        writeln!(writer, "</header>")?;

        if let Some(desc) = &var.description {
            writeln!(
                writer,
                "<div class=\"description\">{}</div>",
                html_escape(desc)
            )?;
        }

        writeln!(writer, "</article>")?;
        Ok(())
    }

    fn get_css(&self) -> &'static str {
        r#"
:root {
    --primary-color: #1a73e8;
    --secondary-color: #34a853;
    --background: #ffffff;
    --surface: #f8f9fa;
    --text-primary: #202124;
    --text-secondary: #5f6368;
    --border-color: #dadce0;
    --code-bg: #f1f3f4;
}

* {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

body {
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
    font-family: 'Cairo', 'Tajawal', sans-serif;
    background: var(--background);
    color: var(--text-primary);
    line-height: 1.6;
    display: flex;
}

.sidebar {
    width: 280px;
    height: 100vh;
    position: fixed;
    right: 0;
    top: 0;
    background: var(--surface);
    border-left: 1px solid var(--border-color);
    overflow-y: auto;
    padding: 1rem;
}

.sidebar .nav-header h2 {
    font-size: 1.1rem;
    color: var(--text-secondary);
    margin-bottom: 1rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--border-color);
}

.sidebar ul {
    list-style: none;
}

.sidebar .nav-section > a {
    font-weight: 600;
    color: var(--text-primary);
    display: block;
    padding: 0.5rem 0;
}

.sidebar ul ul {
    margin-right: 1rem;
}

.sidebar ul ul li a {
    color: var(--text-secondary);
    text-decoration: none;
    display: block;
    padding: 0.25rem 0;
}

.sidebar ul ul li a:hover {
    color: var(--primary-color);
}

.content {
    margin-right: 300px;
    padding: 2rem;
    max-width: 900px;
}

.module-header {
    margin-bottom: 2rem;
    padding-bottom: 1rem;
    border-bottom: 2px solid var(--primary-color);
}

.module-header h1 {
    font-size: 2rem;
    color: var(--primary-color);
}

.module-header .description {
    color: var(--text-secondary);
    margin-top: 0.5rem;
}

section {
    margin-bottom: 3rem;
}

section h2 {
    font-size: 1.5rem;
    color: var(--text-primary);
    margin-bottom: 1.5rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--border-color);
}

.doc-item {
    background: var(--surface);
    border-radius: 8px;
    padding: 1.5rem;
    margin-bottom: 1.5rem;
    border: 1px solid var(--border-color);
}

.doc-item header {
    margin-bottom: 1rem;
}

.doc-item h3.name {
    font-size: 1.2rem;
    font-family: 'Fira Code', monospace;
    word-break: break-word;
}

.keyword {
    color: #d73a49;
    font-weight: 600;
}

.modifier {
    color: #6f42c1;
}

.type {
    color: #22863a;
}

.func-name, .class-name, .interface-name, .method-name, .var-name {
    color: #0366d6;
    font-weight: 600;
}

.badge {
    display: inline-block;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
    margin-right: 0.5rem;
}

.badge.exported {
    background: #e8f5e9;
    color: #2e7d32;
}

.badge.static {
    background: #e3f2fd;
    color: #1565c0;
}

.description {
    color: var(--text-primary);
    margin-bottom: 1rem;
}

.params, .returns, .example {
    margin-top: 1rem;
}

.params h4, .returns h4, .example h4, .fields h4, .methods h4, .constructor h4 {
    font-size: 0.9rem;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
}

dl {
    margin: 0;
}

dt {
    font-weight: 600;
    margin-top: 0.5rem;
}

dd {
    margin-right: 1rem;
    color: var(--text-secondary);
}

pre {
    background: var(--code-bg);
    padding: 1rem;
    border-radius: 4px;
    overflow-x: auto;
    direction: ltr;
    text-align: left;
}

code {
    font-family: 'Fira Code', monospace;
    font-size: 0.9em;
}

table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 0.5rem;
}

th, td {
    padding: 0.75rem;
    text-align: right;
    border-bottom: 1px solid var(--border-color);
}

th {
    background: var(--background);
    font-weight: 600;
}

.visibility {
    font-size: 0.85em;
    font-weight: 600;
}

.visibility.عام { color: #22863a; }
.visibility.خاص { color: #d73a49; }
.visibility.محمي { color: #e36209; }

.method {
    padding: 1rem;
    background: var(--background);
    border-radius: 4px;
    margin-bottom: 0.5rem;
}

.method h5 {
    font-family: 'Fira Code', monospace;
    font-size: 0.95rem;
    margin-bottom: 0.5rem;
}

.method p {
    color: var(--text-secondary);
    font-size: 0.9rem;
}

.method-signature {
    padding: 0.75rem;
    background: var(--background);
    border-radius: 4px;
    margin-bottom: 0.5rem;
}

.method-signature code {
    display: block;
    direction: ltr;
}

.method-signature p {
    margin-top: 0.5rem;
    color: var(--text-secondary);
}

footer {
    margin-right: 300px;
    padding: 2rem;
    text-align: center;
    color: var(--text-secondary);
    border-top: 1px solid var(--border-color);
}

@media (max-width: 768px) {
    .sidebar {
        display: none;
    }
    .content, footer {
        margin-right: 0;
    }
}
"#
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_generate_empty_doc() {
        let doc = Documentation::new("test".to_string(), "test.ترقيم".to_string());
        let generator = HtmlGenerator::new();
        let mut output = Vec::new();
        generator.generate(&doc, &mut output).unwrap();
        let html = String::from_utf8(output).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("lang=\"ar\""));
        assert!(html.contains("dir=\"rtl\""));
        assert!(html.contains("test"));
    }
}
