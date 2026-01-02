//! Tarqeem Runtime Library (Rust Implementation)
//!
//! This crate provides the runtime library for Tarqeem programs, implementing
//! memory management, string operations, array handling, and other core functionality.
//!
//! # ABI Compatibility
//!
//! All exported functions use the C calling convention (`extern "C"`) and are
//! marked with `#[no_mangle]` to ensure they can be linked with compiled
//! Tarqeem programs, JIT-compiled code, and the interpreter.
//!
//! # Module Structure
//!
//! - `types`: FFI-compatible type definitions (TrqString, TrqArray, RefCountHeader)
//! - `memory`: Reference-counted memory allocation
//! - `string`: String operations (creation, manipulation, conversion)
//! - `array`: Array operations (creation, access, modification)
//!
//! # Example Usage (from C)
//!
//! ```c
//! #include <stdint.h>
//!
//! // Link with libtrq.a or libtrq.so
//! extern void* trq_alloc(int64_t size);
//! extern void trq_retain(void* ptr);
//! extern void trq_release(void* ptr);
//! extern int64_t trq_refcount(void* ptr);
//!
//! int main() {
//!     void* ptr = trq_alloc(100);
//!     // refcount is 1
//!
//!     trq_retain(ptr);
//!     // refcount is 2
//!
//!     trq_release(ptr);
//!     // refcount is 1
//!
//!     trq_release(ptr);
//!     // refcount is 0, memory freed
//!
//!     return 0;
//! }
//! ```

#![allow(non_snake_case)]

pub mod array;
pub mod memory;
pub mod string;
pub mod types;

// Re-export all public types
pub use types::{RefCountHeader, TrqArray, TrqString, HEADER_SIZE};

// Re-export all memory functions
pub use memory::{trq_alloc, trq_free, trq_realloc, trq_refcount, trq_release, trq_retain};

// Re-export all string functions
pub use string::{
    trq_bool_to_string, trq_float_to_string, trq_int_to_string, trq_string_char_at,
    trq_string_clone, trq_string_compare, trq_string_concat, trq_string_contains,
    trq_string_count, trq_string_ends_with, trq_string_equals, trq_string_free_data,
    trq_string_from_cstr, trq_string_index_of, trq_string_is_alpha, trq_string_is_arabic,
    trq_string_is_numeric, trq_string_join, trq_string_last_index_of, trq_string_len,
    trq_string_len_chars, trq_string_new, trq_string_pad_left, trq_string_pad_right,
    trq_string_repeat, trq_string_replace, trq_string_replace_all, trq_string_reverse,
    trq_string_split, trq_string_starts_with, trq_string_substr, trq_string_substr_chars,
    trq_string_to_float, trq_string_to_int, trq_string_to_lower, trq_string_to_title,
    trq_string_to_upper, trq_string_trim, trq_string_trim_left, trq_string_trim_right,
};

// Re-export all array functions
pub use array::{
    trq_array_clone, trq_array_concat, trq_array_ensure_capacity, trq_array_free_data,
    trq_array_get, trq_array_len, trq_array_new, trq_array_pop, trq_array_push, trq_array_set,
    trq_array_slice,
};
