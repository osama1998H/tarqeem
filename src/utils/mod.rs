//! Utility modules for Tarqeem
//!
//! This module contains helper utilities used across the compiler.

mod extensions;

pub use extensions::{
    has_tarqeem_extension, is_valid_header_extension, is_valid_source_extension,
    valid_header_extensions_display, valid_source_extensions_display, FileExtension,
    HEADER_EXTENSIONS, SOURCE_EXTENSIONS,
};
