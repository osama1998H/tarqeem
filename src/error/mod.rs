//! Error handling and diagnostics for Tarqeem
//!
//! Provides comprehensive error reporting with support for both
//! Arabic and English messages.
//!
//! # رموز الأخطاء العربية - Arabic Error Codes
//!
//! This module includes an Arabic error code system with codes like:
//! - `د٠٣٠١` - استخدام 'أوقف' خارج حلقة
//! - `ن٠٠٠١` - عدم تطابق الأنواع
//!
//! See `docs/رموز_الأخطاء/` for full documentation.

pub mod codes;
mod diagnostic;
mod span;

#[cfg(test)]
mod diagnostic_tests;
#[cfg(test)]
mod span_tests;

pub use codes::{ErrorCategory, ErrorCode};
pub use diagnostic::{Diagnostic, DiagnosticLevel, Note, Suggestion};
pub use span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Arabic,
    English,
}

pub type TarqeemResult<T> = Result<T, Diagnostic>;
