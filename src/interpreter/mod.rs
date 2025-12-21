//! # Tarqeem Interpreter
//!
//! This module provides an IR-based interpreter for Tarqeem programs.
//! It executes IR instructions directly without compiling to native code,
//! enabling fast development iteration and REPL functionality.
//!
//! ## Architecture
//!
//! The interpreter uses a stack-based execution model with:
//! - `Value`: Runtime value representation
//! - `Environment`: Variable and function storage
//! - `Interpreter`: Core execution engine
//!
//! ## Usage
//!
//! ```ignore
//! use tarqeem::interpreter::Interpreter;
//! use tarqeem::ir::Module;
//!
//! let module: Module = /* build IR */;
//! let mut interpreter = Interpreter::new(module);
//! let result = interpreter.run()?;
//! ```

mod error;
mod executor;
mod value;

#[cfg(test)]
mod executor_tests;

pub use error::{RuntimeError, RuntimeResult};
pub use executor::Interpreter;
pub use value::Value;
