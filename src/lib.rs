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
pub mod codegen;
pub mod error;
pub mod interpreter;
pub mod ir;
pub mod lexer;
pub mod package;
pub mod parser;
pub mod semantic;
pub mod utils;

pub use codegen::{Linker, LlvmCodegen, Target};
pub use error::{Diagnostic, DiagnosticLevel, Span};
pub use ir::{IrBuilder, Module as IrModule};
pub use lexer::{Lexer, Token, TokenKind};
pub use package::{Cache, LockFile, Manifest, PackageError, Resolver};
pub use parser::{Ast, Parser};
pub use semantic::Analyzer;
pub use utils::{has_tarqeem_extension, is_valid_source_extension, FileExtension};
