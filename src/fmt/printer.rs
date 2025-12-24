//! Pretty printer for formatted output
//!
//! Handles indentation, line breaking, and text output generation.

use super::FormatConfig;

#[derive(Debug)]
pub struct Printer {
    buffer: String,

    indent_level: usize,

    config: FormatConfig,

    column: usize,

    at_line_start: bool,

    pending_blank_lines: usize,
}

impl Printer {
    pub fn new(config: FormatConfig) -> Self {
        Self {
            buffer: String::with_capacity(4096),
            indent_level: 0,
            config,
            column: 0,
            at_line_start: true,
            pending_blank_lines: 0,
        }
    }

    pub fn finish(mut self) -> String {
        // Handle final newline
        if self.config.final_newline && !self.buffer.ends_with('\n') && !self.buffer.is_empty() {
            self.buffer.push('\n');
        }

        self.buffer
    }

    pub fn write(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }

        self.flush_indent();
        self.buffer.push_str(s);
        self.column += s.chars().count();
        self.at_line_start = false;
    }

    pub fn write_char(&mut self, c: char) {
        self.flush_indent();
        self.buffer.push(c);
        self.column += 1;
        self.at_line_start = false;
    }

    pub fn write_space(&mut self) {
        if !self.at_line_start && !self.buffer.ends_with(' ') && !self.buffer.ends_with('\t') {
            self.write_char(' ');
        }
    }

    pub fn write_space_if(&mut self, condition: bool) {
        if condition {
            self.write_space();
        }
    }

    pub fn newline(&mut self) {
        self.buffer.push('\n');
        self.column = 0;
        self.at_line_start = true;
    }

    pub fn blank_line(&mut self) {
        self.pending_blank_lines = self.pending_blank_lines.max(1);
    }

    pub fn blank_lines(&mut self, count: usize) {
        self.pending_blank_lines = self.pending_blank_lines.max(count);
    }

    pub fn flush_blank_lines(&mut self) {
        if self.pending_blank_lines > 0 {
            let count = self.pending_blank_lines.min(self.config.max_blank_lines);
            for _ in 0..count {
                self.buffer.push('\n');
            }
            self.pending_blank_lines = 0;
            self.at_line_start = true;
            self.column = 0;
        }
    }

    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    pub fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }

    pub fn indent_level(&self) -> usize {
        self.indent_level
    }

    fn flush_indent(&mut self) {
        // First flush any pending blank lines
        self.flush_blank_lines();

        if self.at_line_start && self.indent_level > 0 {
            let indent = self.config.indent_str().repeat(self.indent_level);
            self.buffer.push_str(&indent);
            self.column += indent.len();
        }
    }

    pub fn write_open_brace(&mut self) {
        use super::BraceStyle;

        match self.config.brace_style {
            BraceStyle::SameLine => {
                self.write_space_if(self.config.space_before_brace);
                self.write_char('{');
            }
            BraceStyle::NextLine => {
                self.newline();
                self.write_char('{');
            }
        }
    }

    pub fn write_close_brace(&mut self) {
        self.write_char('}');
    }

    pub fn write_comma(&mut self) {
        self.write_char(self.config.comma());
        if self.config.space_after_comma {
            self.write_char(' ');
        }
    }

    pub fn write_semicolon(&mut self) {
        self.write_char(self.config.semicolon());
    }

    pub fn write_colon(&mut self) {
        self.write_char(':');
        if self.config.space_after_colon {
            self.write_char(' ');
        }
    }

    pub fn write_operator(&mut self, op: &str) {
        if self.config.space_around_operators {
            self.write_space();
            self.write(op);
            self.write_space();
        } else {
            self.write(op);
        }
    }

    pub fn write_arrow(&mut self) {
        self.write_operator("->");
    }

    pub fn write_fat_arrow(&mut self) {
        self.write_operator("=>");
    }

    pub fn write_parens<F>(&mut self, inner: F)
    where
        F: FnOnce(&mut Self),
    {
        self.write_char('(');
        if self.config.space_inside_parens {
            self.write_char(' ');
        }
        inner(self);
        if self.config.space_inside_parens {
            self.write_char(' ');
        }
        self.write_char(')');
    }

    pub fn write_brackets<F>(&mut self, inner: F)
    where
        F: FnOnce(&mut Self),
    {
        self.write_char('[');
        inner(self);
        self.write_char(']');
    }

    pub fn write_block<F>(&mut self, inner: F)
    where
        F: FnOnce(&mut Self),
    {
        self.write_open_brace();
        self.newline();
        self.indent();
        inner(self);
        self.dedent();
        self.write_close_brace();
    }

    pub fn would_exceed_line_length(&self, text: &str) -> bool {
        self.column + text.len() > self.config.max_line_length
    }

    pub fn config(&self) -> &FormatConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_output() {
        let config = FormatConfig::default();
        let mut printer = Printer::new(config);
        printer.write("متغير");
        printer.write_space();
        printer.write("س");
        printer.write_operator("=");
        printer.write("5");
        printer.newline();
        let result = printer.finish();
        assert_eq!(result, "متغير س = 5\n");
    }

    #[test]
    fn test_indentation() {
        let config = FormatConfig::default();
        let mut printer = Printer::new(config);
        printer.write("دالة");
        printer.write_open_brace();
        printer.newline();
        printer.indent();
        printer.write("س");
        printer.newline();
        printer.dedent();
        printer.write_close_brace();
        printer.newline();
        let result = printer.finish();
        assert!(result.contains("    س")); // 4-space indent
    }

    #[test]
    fn test_blank_lines() {
        let mut config = FormatConfig::default();
        config.max_blank_lines = 2;
        let mut printer = Printer::new(config);
        printer.write("أ");
        printer.newline();
        printer.blank_lines(3); // Request 3, but max is 2
        printer.write("ب");
        printer.newline();
        let result = printer.finish();
        // Should have exactly 2 blank lines (respecting max)
        assert!(result.contains("أ\n\n\nب")); // original newline + 2 blank
    }

    #[test]
    fn test_comma_spacing() {
        let config = FormatConfig::default();
        let mut printer = Printer::new(config);
        printer.write("أ");
        printer.write_comma();
        printer.write("ب");
        let result = printer.finish();
        assert_eq!(result, "أ, ب\n");
    }

    #[test]
    fn test_arabic_comma() {
        let mut config = FormatConfig::default();
        config.arabic_comma = true;
        let mut printer = Printer::new(config);
        printer.write("أ");
        printer.write_comma();
        printer.write("ب");
        let result = printer.finish();
        assert_eq!(result, "أ، ب\n");
    }
}
