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
mod bench;
mod cache;
mod config;
mod debug;
mod error;
mod executor;
mod memory;
pub mod optimizing;
mod profile;
pub mod runtime;

#[cfg(test)]
mod tests;

pub use baseline::{is_baseline_jit_available, BaselineCompiler};
pub use bench::{FunctionBenchmark, FunctionHints, JitBenchmark, PerformanceHint, Timer};
pub use cache::{CacheStats, CodeCache, CompiledFunction, CompiledFunctionInfo};
pub use config::JitConfig;
pub use debug::{
    CodeDumper, DebugVerbosity, DumpFormat, ExecutionTracer, FunctionDebugInfo, JitDebugInfo,
    LineMapping, LocalVarInfo, TraceData, TraceEntry,
};
pub use error::{JitError, JitResult};
pub use executor::JitExecutor;
pub use memory::{
    AllocationRecord, MemoryHint, MemoryPool, MemoryRegion, MemoryStats, MemoryTracker, PoolManager,
};
pub use optimizing::{is_optimizing_jit_available, OptimizingCompiler};
pub use profile::{
    BranchProfile, CompilationTier, FunctionProfile, ObservedType, ProfileData, TierUpDecision,
};
pub use runtime::{
    BackgroundCompiler, CompileRequest, CompileResult, DeoptInfo, DeoptReason, Deoptimizer,
    OsrEntry, OsrState, OsrTrigger,
};
