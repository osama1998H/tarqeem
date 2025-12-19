//! Semantic analysis for Tarqeem
//!
//! This module provides semantic analysis including:
//! - Name resolution
//! - Scope management
//! - Type checking
//! - Symbol table management

mod analyzer;
mod scope;
mod types;

pub use analyzer::Analyzer;
pub use scope::{Scope, Symbol, SymbolKind};
pub use types::Type;
