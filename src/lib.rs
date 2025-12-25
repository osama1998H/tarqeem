//! # Tarqeem - ترقيم
//!
//! A compiled, general-purpose Arabic programming language.
//!
//! ## Overview
//!
//! Tarqeem provides full Arabic syntax support while maintaining compatibility
//! with English keywords. It combines the best features from Python, PHP, and
//! JavaScript into a cohesive, type-safe language.

// Allow large error variants for Diagnostic - boxing would require extensive refactoring.
// TODO(v1.3): Refactor to use Box<Diagnostic> for better performance.
#![allow(clippy::result_large_err)]
// Allow methods that only use self in recursion - these are intentional for API consistency.
#![allow(clippy::only_used_in_recursion)]
// Allow module naming pattern (e.g., lexer/lexer.rs) - intentional structure.
#![allow(clippy::module_inception)]

pub mod cli;
pub mod codegen;
pub mod debug;
pub mod doc;
pub mod error;
pub mod fmt;
pub mod interpreter;
pub mod ir;
pub mod lexer;
pub mod lsp;
pub mod package;
pub mod parser;
pub mod semantic;
pub mod utils;

pub use codegen::{Linker, LlvmCodegen, Target};
pub use debug::{DebugContext, DebugInterpreter, DebugState, SourceMap};
pub use error::{Diagnostic, DiagnosticLevel, Span};
pub use fmt::{format_source, FormatConfig, FormatError, Formatter};
pub use ir::{IrBuilder, Module as IrModule};
pub use lexer::{Lexer, Token, TokenKind};
pub use package::{Cache, LockFile, Manifest, PackageError, Resolver};
pub use parser::{Ast, Parser};
pub use semantic::Analyzer;
pub use utils::{has_tarqeem_extension, is_valid_source_extension, FileExtension, StringInterner, Symbol};
