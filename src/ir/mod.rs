//! Intermediate Representation (IR) Module
//!
//! This module provides the IR layer for the Tarqeem compiler. The IR is
//! a three-address code in SSA (Static Single Assignment) form, designed
//! to be:
//!
//! 1. Easy to optimize (constant folding, DCE, etc.)
//! 2. Easy to lower to LLVM IR
//! 3. Easy to interpret directly (for development mode)
//!
//! # Architecture
//!
//! ```text
//! Typed AST → IR Builder → IR Module → Optimizer → LLVM Codegen
//! ```
//!
//! # Example IR
//!
//! ```text
//! fn @add(%0: i64, %1: i64) -> i64 {
//! bb0:  ; entry
//!     %2: i64 = add %0, %1
//!     ret %2
//! }
//! ```

mod builder;
mod instruction;
pub mod opt;

pub use builder::IrBuilder;
pub use instruction::*;
pub use opt::{OptLevel, OptStats, Optimizer};
