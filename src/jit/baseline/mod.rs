//! Baseline JIT Compiler (Tier 1)
//!
//! This module implements a fast baseline JIT compiler using Cranelift.
//! The baseline compiler prioritizes compilation speed over optimization,
//! generating reasonably efficient code quickly.
//!
//! ## Design Goals
//!
//! - **Fast compilation**: < 5ms per function
//! - **Reasonable performance**: ~5x interpreter speed
//! - **Simple code generation**: No complex optimizations
//!
//! ## Architecture
//!
//! ```text
//! Tarqeem IR → BaselineCompiler → Cranelift IR → Native Code
//! ```

#[cfg(feature = "jit")]
mod compiler;

#[cfg(feature = "jit")]
pub use compiler::BaselineCompiler;

// Stub implementation when JIT feature is disabled
#[cfg(not(feature = "jit"))]
mod stub {
    use crate::ir::{Function, Module};
    use crate::jit::cache::CompiledFunction;
    use crate::jit::error::JitResult;

    /// Stub baseline compiler when JIT is disabled
    pub struct BaselineCompiler;

    impl BaselineCompiler {
        pub fn new() -> JitResult<Self> {
            Ok(Self)
        }

        pub fn compile(
            &mut self,
            _module: &Module,
            _func: &Function,
        ) -> JitResult<CompiledFunction> {
            Err(crate::jit::error::JitError::compilation(
                "JIT compilation is not enabled. Build with --features jit",
                "الترجمة الفورية غير مُفعَّلة. ابنِ مع --features jit",
            ))
        }

        pub fn is_available() -> bool {
            false
        }
    }

    impl Default for BaselineCompiler {
        fn default() -> Self {
            Self
        }
    }
}

#[cfg(not(feature = "jit"))]
pub use stub::BaselineCompiler;

/// Check if baseline JIT is available
pub fn is_baseline_jit_available() -> bool {
    #[cfg(feature = "jit")]
    {
        true
    }
    #[cfg(not(feature = "jit"))]
    {
        false
    }
}
