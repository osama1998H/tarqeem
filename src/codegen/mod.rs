//! Code Generation Module
//!
//! This module handles the conversion of Tarqeem IR to native code.
//! The primary backend is LLVM IR, which can be compiled to native code
//! using llc/clang, or to WebAssembly for web deployment.
//!
//! # Architecture
//!
//! ```text
//! IR Module → LLVM IR Generator → .ll file → llc → .o file → linker → executable
//!                                         ↓
//!                                    (WASM target)
//!                                         ↓
//!                              clang → .wasm file + .js bindings
//! ```
//!
//! # Targets
//!
//! Supported targets:
//! - x86_64-linux-gnu (native)
//! - x86_64-apple-darwin (native)
//! - aarch64-linux-gnu (native)
//! - aarch64-apple-darwin (native)
//! - wasm32-unknown-unknown (WebAssembly for browsers)
//! - wasm32-wasip1 (WebAssembly with WASI for standalone)

pub mod linker;
pub mod llvm;
pub mod target;

pub use linker::{EmitType, Linker, WasmConfig};
pub use llvm::LlvmCodegen;
pub use target::{Target, TargetTriple};
