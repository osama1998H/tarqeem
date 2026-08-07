//! رسائل التشخيص - ترقيم لغة عربية فقط

use super::{Language, Span};
use colored::Colorize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
    Hint,
}

impl DiagnosticLevel {
    pub fn name(&self) -> &'static str {
        match self {
            DiagnosticLevel::Error => "خطأ",
            DiagnosticLevel::Warning => "تحذير",
            DiagnosticLevel::Info => "معلومة",
            DiagnosticLevel::Hint => "تلميح",
        }
    }
}

impl fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Clone)]
pub struct Note {
    pub message: String,
    pub span: Option<Span>,
}

impl Note {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub message: String,
    pub replacement: String,
    pub span: Span,
}

impl Suggestion {
    pub fn new(message: impl Into<String>, replacement: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            replacement: replacement.into(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub span: Span,
    pub notes: Vec<Note>,
    pub suggestions: Vec<Suggestion>,
    pub code: Option<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message: message.into(),
            span,
            notes: Vec::new(),
            suggestions: Vec::new(),
            code: None,
        }
    }

    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            message: message.into(),
            span,
            notes: Vec::new(),
            suggestions: Vec::new(),
            code: None,
        }
    }

    pub fn with_note(mut self, note: Note) -> Self {
        self.notes.push(note);
        self
    }

    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn emit(&self, source: &str, filename: &str, _lang: Language) {
        let level_str = self.level.name();

        let level_colored = match self.level {
            DiagnosticLevel::Error => level_str.red().bold(),
            DiagnosticLevel::Warning => level_str.yellow().bold(),
            DiagnosticLevel::Info => level_str.blue().bold(),
            DiagnosticLevel::Hint => level_str.cyan().bold(),
        };

        if let Some(code) = &self.code {
            eprintln!(
                "{}{}{}: {}",
                level_colored,
                "[".dimmed(),
                code.dimmed(),
                self.message.bold()
            );
        } else {
            eprintln!("{}: {}", level_colored, self.message.bold());
        }

        eprintln!(
            "  {} {}:{}:{}",
            "-->".blue().bold(),
            filename,
            self.span.line,
            self.span.column
        );

        if !source.is_empty() && self.span.line > 0 {
            let lines: Vec<&str> = source.lines().collect();
            if self.span.line <= lines.len() {
                let line = lines[self.span.line - 1];
                let line_num = format!("{}", self.span.line);
                let padding = " ".repeat(line_num.len());

                eprintln!("   {}|", padding.blue().bold());
                eprintln!(" {} | {}", line_num.blue().bold(), line);

                let col = self.span.column.saturating_sub(1);
                let underline_len = self.span.len().max(1);
                let underline = format!("{}{}", " ".repeat(col), "^".repeat(underline_len));
                eprintln!("   {}| {}", padding.blue().bold(), underline.red().bold());
            }
        }

        for note in &self.notes {
            eprintln!(
                "   {} {}: {}",
                "=".blue().bold(),
                "ملاحظة".cyan(),
                note.message
            );
        }

        for suggestion in &self.suggestions {
            eprintln!(
                "   {} {}: {}",
                "=".blue().bold(),
                "مساعدة".green(),
                suggestion.message
            );
            eprintln!(
                "   {} `{}`",
                "=".blue().bold(),
                suggestion.replacement.green()
            );
        }

        eprintln!();
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.level.name(), self.message)
    }
}

impl std::error::Error for Diagnostic {}
