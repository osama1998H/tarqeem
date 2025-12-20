//! Utility functions for the LSP server

mod position;

pub use position::{find_word_at_position, position_to_offset, span_to_range};
