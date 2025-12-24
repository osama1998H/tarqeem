//! Error handling and diagnostics for Tarqeem
//!
//! Provides comprehensive error reporting with support for both
//! Arabic and English messages.

mod diagnostic;
mod span;

#[cfg(test)]
mod diagnostic_tests;
#[cfg(test)]
mod span_tests;

pub use diagnostic::{Diagnostic, DiagnosticLevel, Note, Suggestion};
pub use span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Arabic,
    English,
}

pub type TarqeemResult<T> = Result<T, Diagnostic>;
