//! Code Generation Module
//!
//! This module handles the conversion of Tarqeem IR to native code.
//! The primary backend is LLVM IR, which can be compiled to native code
//! using llc/clang.
//!
//! # Architecture
//!
//! ```text
//! IR Module → LLVM IR Generator → .ll file → llc → .o file → linker → executable
//! ```
//!
//! # Targets
//!
//! Supported targets:
//! - x86_64-linux-gnu
//! - x86_64-apple-darwin
//! - aarch64-linux-gnu
//! - wasm32 (future)

pub mod linker;
pub mod llvm;
pub mod target;

pub use linker::Linker;
pub use llvm::LlvmCodegen;
pub use target::{Target, TargetTriple};
