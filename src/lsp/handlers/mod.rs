//! LSP request handlers
//!
//! This module contains handlers for all LSP requests and notifications.

mod code_actions;
mod completion;
mod definition;
mod diagnostics;
mod document_symbol;
mod folding;
mod formatting;
mod hover;
mod inlay_hints;
mod references;
mod rename;
mod semantic_tokens;

pub use code_actions::handle_code_actions;
pub use completion::handle_completion;
pub use definition::handle_definition;
pub use diagnostics::publish_diagnostics;
pub use document_symbol::handle_document_symbol;
pub use folding::handle_folding_ranges;
pub use formatting::handle_formatting;
pub use hover::handle_hover;
pub use inlay_hints::handle_inlay_hints;
pub use references::handle_references;
pub use rename::{handle_prepare_rename, handle_rename};
pub use semantic_tokens::{get_semantic_tokens_legend, handle_semantic_tokens_full};
