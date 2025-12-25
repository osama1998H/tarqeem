//! JIT Error Types
//!
//! This module defines error types for the JIT compilation system.

use std::fmt;

/// Result type for JIT operations
pub type JitResult<T> = Result<T, JitError>;

/// JIT compilation and execution errors
#[derive(Debug, Clone)]
pub struct JitError {
    /// Error kind
    pub kind: JitErrorKind,

    /// Error message (English)
    pub message: String,

    /// Error message (Arabic)
    pub message_ar: String,

    /// Optional function name where the error occurred
    pub function: Option<String>,
}

impl JitError {
    /// Create a new JIT error
    pub fn new(
        kind: JitErrorKind,
        message: impl Into<String>,
        message_ar: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            message_ar: message_ar.into(),
            function: None,
        }
    }

    /// Add function context to the error
    pub fn with_function(mut self, function: impl Into<String>) -> Self {
        self.function = Some(function.into());
        self
    }

    /// Create a compilation error
    pub fn compilation(message: impl Into<String>, message_ar: impl Into<String>) -> Self {
        Self::new(JitErrorKind::Compilation, message, message_ar)
    }

    /// Create an unsupported instruction error
    pub fn unsupported_instruction(inst: impl Into<String>) -> Self {
        let inst = inst.into();
        Self::new(
            JitErrorKind::UnsupportedInstruction,
            format!("Unsupported instruction for JIT compilation: {}", inst),
            format!("تعليمة غير مدعومة للترجمة الفورية: {}", inst),
        )
    }

    /// Create a code generation error
    pub fn codegen(message: impl Into<String>, message_ar: impl Into<String>) -> Self {
        Self::new(JitErrorKind::CodeGeneration, message, message_ar)
    }

    /// Create a memory allocation error
    pub fn memory(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self::new(
            JitErrorKind::MemoryAllocation,
            format!("Memory allocation failed: {}", msg),
            format!("فشل في تخصيص الذاكرة: {}", msg),
        )
    }

    /// Create a code cache error
    pub fn cache_full() -> Self {
        Self::new(
            JitErrorKind::CacheFull,
            "Code cache is full, cannot compile more functions",
            "ذاكرة التخزين ممتلئة، لا يمكن ترجمة المزيد من الدوال",
        )
    }

    /// Create a deoptimization error
    pub fn deoptimization(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self::new(
            JitErrorKind::Deoptimization,
            format!("Deoptimization required: {}", reason),
            format!("مطلوب إعادة التفسير: {}", reason),
        )
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self::new(
            JitErrorKind::Internal,
            format!("Internal JIT error: {}", msg),
            format!("خطأ داخلي في الترجمة الفورية: {}", msg),
        )
    }

    /// Create an LLVM error
    pub fn llvm(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self::new(
            JitErrorKind::LlvmError,
            format!("LLVM error: {}", msg),
            format!("خطأ LLVM: {}", msg),
        )
    }

    /// Create a tier-up failure error
    pub fn tier_up_failed(from: &str, to: &str, reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self::new(
            JitErrorKind::TierUpFailed,
            format!("Failed to tier up from {} to {}: {}", from, to, reason),
            format!("فشل الترقية من {} إلى {}: {}", from, to, reason),
        )
    }
}

impl fmt::Display for JitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref func) = self.function {
            write!(f, "[{}] ", func)?;
        }
        write!(f, "{} / {}", self.message, self.message_ar)
    }
}

impl std::error::Error for JitError {}

/// Kind of JIT error
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JitErrorKind {
    /// Error during compilation
    Compilation,

    /// Unsupported instruction for JIT
    UnsupportedInstruction,

    /// Error generating native code
    CodeGeneration,

    /// Memory allocation failed
    MemoryAllocation,

    /// Code cache is full
    CacheFull,

    /// Deoptimization required
    Deoptimization,

    /// Internal JIT error
    Internal,

    /// LLVM-specific error
    LlvmError,

    /// Tier-up failed
    TierUpFailed,
}

impl fmt::Display for JitErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JitErrorKind::Compilation => write!(f, "Compilation / ترجمة"),
            JitErrorKind::UnsupportedInstruction => {
                write!(f, "Unsupported Instruction / تعليمة غير مدعومة")
            }
            JitErrorKind::CodeGeneration => write!(f, "Code Generation / توليد الكود"),
            JitErrorKind::MemoryAllocation => write!(f, "Memory Allocation / تخصيص الذاكرة"),
            JitErrorKind::CacheFull => write!(f, "Cache Full / ذاكرة التخزين ممتلئة"),
            JitErrorKind::Deoptimization => write!(f, "Deoptimization / إعادة التفسير"),
            JitErrorKind::Internal => write!(f, "Internal / داخلي"),
            JitErrorKind::LlvmError => write!(f, "LLVM Error / خطأ LLVM"),
            JitErrorKind::TierUpFailed => write!(f, "Tier-Up Failed / فشل الترقية"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_error_creation() {
        let err = JitError::compilation("Test error", "خطأ تجريبي");

        assert_eq!(err.kind, JitErrorKind::Compilation);
        assert!(err.message.contains("Test error"));
        assert!(err.message_ar.contains("خطأ تجريبي"));
    }

    #[test]
    fn test_jit_error_with_function() {
        let err = JitError::compilation("Test", "تجربة").with_function("my_func");

        assert_eq!(err.function, Some("my_func".to_string()));
        let display = format!("{}", err);
        assert!(display.contains("[my_func]"));
    }

    #[test]
    fn test_unsupported_instruction() {
        let err = JitError::unsupported_instruction("CallVirtual");

        assert_eq!(err.kind, JitErrorKind::UnsupportedInstruction);
        assert!(err.message.contains("CallVirtual"));
    }

    #[test]
    fn test_memory_error() {
        let err = JitError::memory("out of executable memory");

        assert_eq!(err.kind, JitErrorKind::MemoryAllocation);
    }

    #[test]
    fn test_cache_full() {
        let err = JitError::cache_full();

        assert_eq!(err.kind, JitErrorKind::CacheFull);
    }

    #[test]
    fn test_error_display() {
        let err = JitError::internal("test");
        let display = format!("{}", err);

        assert!(display.contains("Internal JIT error"));
        assert!(display.contains("خطأ داخلي"));
    }
}
