//! Optimizing JIT Compiler (Tier 2)
//!
//! This module implements the optimizing tier of the JIT compilation pipeline.
//! It uses Cranelift with aggressive optimizations and profile-guided compilation.
//!
//! المترجم الآني المُحسِّن (المستوى الثاني)
//! يستخدم Cranelift مع تحسينات قوية وقرارات موجهة بالملف الشخصي

#[cfg(feature = "jit")]
mod compiler;
#[cfg(feature = "jit")]
mod specialize;

#[cfg(feature = "jit")]
pub use compiler::OptimizingCompiler;
#[cfg(feature = "jit")]
pub use specialize::{SpecializationKey, TypeSpecializer};

/// Check if optimizing JIT is available
pub fn is_optimizing_jit_available() -> bool {
    cfg!(feature = "jit")
}

#[cfg(not(feature = "jit"))]
pub struct OptimizingCompiler;

#[cfg(not(feature = "jit"))]
impl OptimizingCompiler {
    pub fn new() -> Result<Self, crate::jit::error::JitError> {
        Err(crate::jit::error::JitError::not_available(
            "Optimizing JIT requires 'jit' feature",
        ))
    }
}
