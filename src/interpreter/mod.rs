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

pub use error::{ErrorKind, RuntimeError, RuntimeResult};
pub(crate) use executor::builtins::bytes_to_string;
/// Shared for the same reason: `متغير_بيئة`'s contract is its argument checks,
/// and the name must be read raw in both (#324).
pub(crate) use executor::builtins::call_env_var;
/// Shared with the debug interpreter so `قص_حروف` is total in the same way in
/// both — the argument checks drift as easily as the slicing does.
pub(crate) use executor::builtins::call_substring_by_chars;
pub use executor::Interpreter;
pub use value::Value;

/// Shared with the debug interpreter so both time builtins agree (#241).
pub(crate) use executor::builtins::epoch_millis;
