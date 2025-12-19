//! # Tarqeem - ترقيم
//!
//! A compiled, general-purpose Arabic programming language.
//!
//! ## Overview
//!
//! Tarqeem provides full Arabic syntax support while maintaining compatibility
//! with English keywords. It combines the best features from Python, PHP, and
//! JavaScript into a cohesive, type-safe language.

pub mod cli;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod semantic;

pub use error::{Diagnostic, DiagnosticLevel, Span};
pub use lexer::{Lexer, Token, TokenKind};
pub use parser::{Ast, Parser};
pub use semantic::Analyzer;
