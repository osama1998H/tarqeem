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

pub mod memory;
pub mod types;

// Re-export all public types
pub use types::{RefCountHeader, TrqArray, TrqString, HEADER_SIZE};

// Re-export all public functions
pub use memory::{trq_alloc, trq_free, trq_realloc, trq_refcount, trq_release, trq_retain};
