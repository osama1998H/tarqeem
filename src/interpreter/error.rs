//! Runtime error types for the interpreter.

use std::fmt;

/// Result type for interpreter operations.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Runtime error that can occur during interpretation.
#[derive(Debug, Clone)]
pub struct RuntimeError {
    /// Error message in English
    pub message: String,
    /// Error message in Arabic
    pub message_ar: String,
    /// Error kind
    pub kind: ErrorKind,
}

/// Categories of runtime errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Type mismatch during operation
    TypeError,
    /// Division by zero
    DivisionByZero,
    /// Array index out of bounds
    IndexOutOfBounds,
    /// Undefined variable access
    UndefinedVariable,
    /// Undefined function call
    UndefinedFunction,
    /// Null pointer dereference
    NullPointer,
    /// Stack overflow (too many nested calls)
    StackOverflow,
    /// Unhandled exception thrown
    UnhandledException,
    /// Invalid operation
    InvalidOperation,
    /// Internal interpreter error
    Internal,
}

impl RuntimeError {
    /// Create a new runtime error.
    pub fn new(kind: ErrorKind, message: impl Into<String>, message_ar: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            message_ar: message_ar.into(),
            kind,
        }
    }

    /// Create a type error.
    pub fn type_error(expected: &str, got: &str) -> Self {
        Self::new(
            ErrorKind::TypeError,
            format!("Type error: expected {}, got {}", expected, got),
            format!("خطأ في النوع: متوقع {}، وُجد {}", expected, got),
        )
    }

    /// Create a division by zero error.
    pub fn division_by_zero() -> Self {
        Self::new(
            ErrorKind::DivisionByZero,
            "Division by zero",
            "القسمة على صفر",
        )
    }

    /// Create an index out of bounds error.
    pub fn index_out_of_bounds(index: i64, len: usize) -> Self {
        Self::new(
            ErrorKind::IndexOutOfBounds,
            format!("Index {} out of bounds for array of length {}", index, len),
            format!("الفهرس {} خارج حدود المصفوفة ذات الطول {}", index, len),
        )
    }

    /// Create an undefined variable error.
    pub fn undefined_variable(name: &str) -> Self {
        Self::new(
            ErrorKind::UndefinedVariable,
            format!("Undefined variable: {}", name),
            format!("متغير غير معرّف: {}", name),
        )
    }

    /// Create an undefined function error.
    pub fn undefined_function(name: &str) -> Self {
        Self::new(
            ErrorKind::UndefinedFunction,
            format!("Undefined function: {}", name),
            format!("دالة غير معرّفة: {}", name),
        )
    }

    /// Create a null pointer error.
    pub fn null_pointer() -> Self {
        Self::new(
            ErrorKind::NullPointer,
            "Null pointer dereference",
            "محاولة الوصول لمؤشر فارغ",
        )
    }

    /// Create a stack overflow error.
    pub fn stack_overflow() -> Self {
        Self::new(
            ErrorKind::StackOverflow,
            "Stack overflow: too many nested function calls",
            "تجاوز المكدس: استدعاءات دالة متداخلة كثيرة جداً",
        )
    }

    /// Create an unhandled exception error.
    pub fn unhandled_exception(msg: &str) -> Self {
        Self::new(
            ErrorKind::UnhandledException,
            format!("Unhandled exception: {}", msg),
            format!("استثناء غير معالج: {}", msg),
        )
    }

    /// Create an invalid operation error.
    pub fn invalid_operation(msg: impl Into<String>, msg_ar: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidOperation, msg, msg_ar)
    }

    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        Self::new(
            ErrorKind::Internal,
            format!("Internal error: {}", msg),
            format!("خطأ داخلي: {}", msg),
        )
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} / {}", self.message, self.message_ar)
    }
}

impl std::error::Error for RuntimeError {}
