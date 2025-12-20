//! Incremental analysis engine for the LSP server
//!
//! This module manages document state and provides incremental
//! analysis capabilities for real-time IDE features.

mod document;

pub use document::{AnalysisResult, DocumentState, SymbolInfo, SymbolKind};
