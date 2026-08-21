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
/// Shared so the `& ٢٥٥` masking cannot differ between the two interpreters.
pub(crate) use executor::builtins::call_exit_program;
/// Shared with the debug interpreter so `قص_حروف` is total in the same way in
/// both — the argument checks drift as easily as the slicing does.
pub(crate) use executor::builtins::call_substring_by_chars;
/// Shared so `اكتب_مجرى` refuses and counts identically in both: the descriptor
/// map, the byte-range rejection and the empty-versus-failed answers are all
/// contract, and all live in the one dispatch.
pub(crate) use executor::builtins::call_write_stream;

/// Shared with the debug interpreter for the same reason, and its sibling:
/// the read half of the byte-level stream pair.
pub(crate) use executor::builtins::call_read_stream;

pub(crate) use executor::builtins::call_path_delete;
/// Shared for a reason the others do not have: the kind/size mapping is already
/// duplicated once, in `trq_path_status`, because the compiler crate does not
/// depend on `tarqeem-runtime`. A third copy in the debug interpreter would give
/// it two ways to drift instead of one.
pub(crate) use executor::builtins::call_path_status;
/// Shared with the debug interpreter so an unset argument list answers the same
/// empty array in both. Unlike its neighbours this has no `runtime-rs` twin: the
/// runtime reads its own `main`'s argv while this reads what the CLI was handed,
/// so the two sides have nothing to share and nothing to drift.
pub(crate) use executor::builtins::call_program_args;
/// Recorded by the CLI before a program runs; read by `معاملات_البرنامج`.
pub use executor::builtins::set_program_args;
pub use executor::Interpreter;
pub use value::Value;

/// Shared with the debug interpreter so both time builtins agree (#241).
pub(crate) use executor::builtins::epoch_millis;
