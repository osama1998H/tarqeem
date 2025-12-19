//! Error handling and diagnostics for Tarqeem
//!
//! Provides comprehensive error reporting with support for both
//! Arabic and English messages.

mod diagnostic;
mod span;

pub use diagnostic::{Diagnostic, DiagnosticLevel, Note, Suggestion};
pub use span::Span;

/// Language for error messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Arabic,
    English,
}

/// Result type for Tarqeem operations
pub type TarqeemResult<T> = Result<T, Diagnostic>;
