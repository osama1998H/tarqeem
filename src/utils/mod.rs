//! Utility modules for Tarqeem
//!
//! This module contains helper utilities used across the compiler.

mod extensions;

pub use extensions::{
    has_tarqeem_extension, is_valid_lock_extension, is_valid_package_extension,
    is_valid_source_extension, valid_lock_extension_display, valid_package_extension_display,
    valid_source_extension_display, FileExtension, LOCK_EXTENSION, PACKAGE_EXTENSION,
    SOURCE_EXTENSION,
};
