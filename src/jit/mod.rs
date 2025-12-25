//! # Tarqeem JIT Compilation Module
//!
//! This module provides Just-In-Time compilation support for Tarqeem programs.
//! It implements a tiered execution model:
//!
//! - **Tier 0**: Interpreter (immediate execution, no compilation)
//! - **Tier 1**: Baseline JIT (fast compilation, moderate speedup)
//! - **Tier 2**: Optimizing JIT (slower compilation, maximum speedup)
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    JIT Execution Engine                      │
//! │  ┌──────────┐    ┌──────────┐    ┌──────────────┐           │
//! │  │  Tier 0  │ →  │  Tier 1  │ →  │   Tier 2     │           │
//! │  │Interpreter│   │Baseline  │    │ Optimizing   │           │
//! │  │ (cold)   │    │  JIT     │    │    JIT       │           │
//! │  └──────────┘    └──────────┘    └──────────────┘           │
//! │       ↑               ↑               ↑                     │
//! │       └───────────────┴───────────────┘                     │
//! │              Profiling & Tier-Up Decisions                  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use tarqeem::jit::{JitExecutor, JitConfig};
//! use tarqeem::ir::Module;
//!
//! let module: Module = /* build IR */;
//! let config = JitConfig::default();
//! let mut executor = JitExecutor::new(module, config);
//! let result = executor.run()?;
//! ```

pub mod baseline;
mod cache;
mod config;
mod error;
mod executor;
mod profile;

#[cfg(test)]
mod tests;

pub use baseline::{is_baseline_jit_available, BaselineCompiler};
pub use cache::{CacheStats, CodeCache, CompiledFunction, CompiledFunctionInfo};
pub use config::JitConfig;
pub use error::{JitError, JitResult};
pub use executor::JitExecutor;
pub use profile::{
    BranchProfile, CompilationTier, FunctionProfile, ObservedType, ProfileData, TierUpDecision,
};
