//! Diagnostic messages with Arabic/English support

use super::{Language, Span};
use colored::Colorize;
use std::fmt;

/// Severity level of a diagnostic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
    Hint,
}

impl DiagnosticLevel {
    /// Get the Arabic name for this level
    pub fn arabic(&self) -> &'static str {
        match self {
            DiagnosticLevel::Error => "خطأ",
            DiagnosticLevel::Warning => "تحذير",
            DiagnosticLevel::Info => "معلومة",
            DiagnosticLevel::Hint => "تلميح",
        }
    }

    /// Get the English name for this level
    pub fn english(&self) -> &'static str {
        match self {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Info => "info",
            DiagnosticLevel::Hint => "hint",
        }
    }
}

/// A note attached to a diagnostic
#[derive(Debug, Clone)]
pub struct Note {
    pub message: String,
    pub message_ar: String,
    pub span: Option<Span>,
}

impl Note {
    pub fn new(message: impl Into<String>, message_ar: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            message_ar: message_ar.into(),
            span: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

/// A fix suggestion for a diagnostic
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub message: String,
    pub message_ar: String,
    pub replacement: String,
    pub span: Span,
}

impl Suggestion {
    pub fn new(
        message: impl Into<String>,
        message_ar: impl Into<String>,
        replacement: impl Into<String>,
        span: Span,
    ) -> Self {
        Self {
            message: message.into(),
            message_ar: message_ar.into(),
            replacement: replacement.into(),
            span,
        }
    }
}

/// A compiler diagnostic with bilingual messages
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub message_ar: String,
    pub span: Span,
    pub notes: Vec<Note>,
    pub suggestions: Vec<Suggestion>,
    pub code: Option<String>,
}

impl Diagnostic {
    /// Create a new error diagnostic
    pub fn error(message: impl Into<String>, message_ar: impl Into<String>, span: Span) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message: message.into(),
            message_ar: message_ar.into(),
            span,
            notes: Vec::new(),
            suggestions: Vec::new(),
            code: None,
        }
    }

    /// Create a new warning diagnostic
    pub fn warning(message: impl Into<String>, message_ar: impl Into<String>, span: Span) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            message: message.into(),
            message_ar: message_ar.into(),
            span,
            notes: Vec::new(),
            suggestions: Vec::new(),
            code: None,
        }
    }

    /// Add a note to this diagnostic
    pub fn with_note(mut self, note: Note) -> Self {
        self.notes.push(note);
        self
    }

    /// Add a suggestion to this diagnostic
    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Set the error code
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Format and emit the diagnostic to stderr
    pub fn emit(&self, source: &str, filename: &str, lang: Language) {
        let level_str = match lang {
            Language::Arabic => self.level.arabic(),
            Language::English => self.level.english(),
        };

        let message = match lang {
            Language::Arabic => &self.message_ar,
            Language::English => &self.message,
        };

        let level_colored = match self.level {
            DiagnosticLevel::Error => level_str.red().bold(),
            DiagnosticLevel::Warning => level_str.yellow().bold(),
            DiagnosticLevel::Info => level_str.blue().bold(),
            DiagnosticLevel::Hint => level_str.cyan().bold(),
        };

        // Print header
        if let Some(code) = &self.code {
            eprintln!("{}{}{}: {}", level_colored, "[".dimmed(), code.dimmed(), message.bold());
        } else {
            eprintln!("{}: {}", level_colored, message.bold());
        }

        // Print location
        eprintln!(
            "  {} {}:{}:{}",
            "-->".blue().bold(),
            filename,
            self.span.line,
            self.span.column
        );

        // Print source context
        if !source.is_empty() && self.span.line > 0 {
            let lines: Vec<&str> = source.lines().collect();
            if self.span.line <= lines.len() {
                let line = lines[self.span.line - 1];
                let line_num = format!("{}", self.span.line);
                let padding = " ".repeat(line_num.len());

                eprintln!("   {}|", padding.blue().bold());
                eprintln!(" {} | {}", line_num.blue().bold(), line);

                // Print underline
                let col = self.span.column.saturating_sub(1);
                let underline_len = self.span.len().max(1);
                let underline = format!(
                    "{}{}",
                    " ".repeat(col),
                    "^".repeat(underline_len)
                );
                eprintln!(
                    "   {}| {}",
                    padding.blue().bold(),
                    underline.red().bold()
                );
            }
        }

        // Print notes
        for note in &self.notes {
            let note_msg = match lang {
                Language::Arabic => &note.message_ar,
                Language::English => &note.message,
            };
            eprintln!("   {} {}: {}", "=".blue().bold(), "note".cyan(), note_msg);
        }

        // Print suggestions
        for suggestion in &self.suggestions {
            let sugg_msg = match lang {
                Language::Arabic => &suggestion.message_ar,
                Language::English => &suggestion.message,
            };
            eprintln!(
                "   {} {}: {}",
                "=".blue().bold(),
                "help".green(),
                sugg_msg
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
        write!(
            f,
            "{}: {} | {}: {}",
            self.level.english(),
            self.message,
            self.level.arabic(),
            self.message_ar
        )
    }
}

impl std::error::Error for Diagnostic {}
