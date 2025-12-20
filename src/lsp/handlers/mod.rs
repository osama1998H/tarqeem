//! LSP request handlers
//!
//! This module contains handlers for all LSP requests and notifications.

mod completion;
mod definition;
mod diagnostics;
mod document_symbol;
mod formatting;
mod hover;
mod references;
mod rename;

pub use completion::handle_completion;
pub use definition::handle_definition;
pub use diagnostics::publish_diagnostics;
pub use document_symbol::handle_document_symbol;
pub use formatting::handle_formatting;
pub use hover::handle_hover;
pub use references::handle_references;
pub use rename::{handle_prepare_rename, handle_rename};
