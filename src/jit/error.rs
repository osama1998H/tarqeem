//! أخطاء الترجمة الفورية (JIT)
//!
//! ترقيم لغة برمجة عربية - جميع الرسائل بالعربية فقط

use std::fmt;

/// نتيجة عمليات JIT
pub type JitResult<T> = Result<T, JitError>;

/// أخطاء الترجمة الفورية والتنفيذ
#[derive(Debug, Clone)]
pub struct JitError {
    /// نوع الخطأ
    pub kind: JitErrorKind,

    /// رسالة الخطأ (بالعربية)
    pub message: String,

    /// اسم الدالة (اختياري)
    pub function: Option<String>,
}

impl JitError {
    /// إنشاء خطأ JIT جديد
    pub fn new(kind: JitErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            function: None,
        }
    }

    /// إضافة سياق الدالة للخطأ
    pub fn with_function(mut self, function: impl Into<String>) -> Self {
        self.function = Some(function.into());
        self
    }

    /// إنشاء خطأ ترجمة
    pub fn compilation(message: impl Into<String>) -> Self {
        Self::new(JitErrorKind::Compilation, message)
    }

    /// إنشاء خطأ تعليمة غير مدعومة
    pub fn unsupported_instruction(inst: impl Into<String>) -> Self {
        let inst = inst.into();
        Self::new(
            JitErrorKind::UnsupportedInstruction,
            format!("تعليمة غير مدعومة للترجمة الفورية: {}", inst),
        )
    }

    /// إنشاء خطأ توليد الكود
    pub fn codegen(message: impl Into<String>) -> Self {
        Self::new(JitErrorKind::CodeGeneration, message)
    }

    /// إنشاء خطأ تخصيص الذاكرة
    pub fn memory(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self::new(
            JitErrorKind::MemoryAllocation,
            format!("فشل في تخصيص الذاكرة: {}", msg),
        )
    }

    /// إنشاء خطأ ذاكرة التخزين ممتلئة
    pub fn cache_full() -> Self {
        Self::new(
            JitErrorKind::CacheFull,
            "ذاكرة التخزين ممتلئة، لا يمكن ترجمة المزيد من الدوال",
        )
    }

    /// إنشاء خطأ إعادة التفسير
    pub fn deoptimization(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self::new(
            JitErrorKind::Deoptimization,
            format!("مطلوب إعادة التفسير: {}", reason),
        )
    }

    /// إنشاء خطأ داخلي
    pub fn internal(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self::new(
            JitErrorKind::Internal,
            format!("خطأ داخلي في الترجمة الفورية: {}", msg),
        )
    }

    /// إنشاء خطأ LLVM
    pub fn llvm(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self::new(JitErrorKind::LlvmError, format!("خطأ LLVM: {}", msg))
    }

    /// إنشاء خطأ فشل الترقية
    pub fn tier_up_failed(from: &str, to: &str, reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self::new(
            JitErrorKind::TierUpFailed,
            format!("فشل الترقية من {} إلى {}: {}", from, to, reason),
        )
    }

    /// إنشاء خطأ ميزة غير متوفرة
    pub fn not_available(feature: impl Into<String>) -> Self {
        let feature = feature.into();
        Self::new(
            JitErrorKind::NotAvailable,
            format!("ميزة الترجمة الفورية غير متوفرة: {}", feature),
        )
    }
}

impl fmt::Display for JitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref func) = self.function {
            write!(f, "[{}] ", func)?;
        }
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for JitError {}

/// نوع خطأ JIT
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JitErrorKind {
    /// خطأ أثناء الترجمة
    Compilation,

    /// تعليمة غير مدعومة
    UnsupportedInstruction,

    /// خطأ توليد الكود
    CodeGeneration,

    /// فشل تخصيص الذاكرة
    MemoryAllocation,

    /// ذاكرة التخزين ممتلئة
    CacheFull,

    /// مطلوب إعادة التفسير
    Deoptimization,

    /// خطأ داخلي
    Internal,

    /// خطأ LLVM
    LlvmError,

    /// فشل الترقية
    TierUpFailed,

    /// ميزة غير متوفرة
    NotAvailable,
}

impl fmt::Display for JitErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JitErrorKind::Compilation => write!(f, "ترجمة"),
            JitErrorKind::UnsupportedInstruction => write!(f, "تعليمة غير مدعومة"),
            JitErrorKind::CodeGeneration => write!(f, "توليد الكود"),
            JitErrorKind::MemoryAllocation => write!(f, "تخصيص الذاكرة"),
            JitErrorKind::CacheFull => write!(f, "ذاكرة التخزين ممتلئة"),
            JitErrorKind::Deoptimization => write!(f, "إعادة التفسير"),
            JitErrorKind::Internal => write!(f, "داخلي"),
            JitErrorKind::LlvmError => write!(f, "خطأ LLVM"),
            JitErrorKind::TierUpFailed => write!(f, "فشل الترقية"),
            JitErrorKind::NotAvailable => write!(f, "غير متوفر"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_error_creation() {
        let err = JitError::compilation("خطأ تجريبي");

        assert_eq!(err.kind, JitErrorKind::Compilation);
        assert!(err.message.contains("خطأ تجريبي"));
    }

    #[test]
    fn test_jit_error_with_function() {
        let err = JitError::compilation("تجربة").with_function("my_func");

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
        let err = JitError::memory("نفاد الذاكرة");

        assert_eq!(err.kind, JitErrorKind::MemoryAllocation);
    }

    #[test]
    fn test_cache_full() {
        let err = JitError::cache_full();

        assert_eq!(err.kind, JitErrorKind::CacheFull);
    }

    #[test]
    fn test_error_display() {
        let err = JitError::internal("اختبار");
        let display = format!("{}", err);

        assert!(display.contains("خطأ داخلي"));
    }
}
