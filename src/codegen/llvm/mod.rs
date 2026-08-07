//! LLVM IR Code Generation
//!
//! This module generates LLVM IR text format from Tarqeem IR.
//! The generated IR can be compiled using llc or clang.

mod codegen;
mod types;

#[cfg(test)]
mod codegen_tests;

pub use codegen::LlvmCodegen;
pub use types::{mangle_name, TypeMapper};
