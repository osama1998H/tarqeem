//! Runtime error types for the interpreter.

use std::fmt;

pub type RuntimeResult<T> = Result<T, RuntimeError>;

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
    pub kind: ErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    TypeError,
    DivisionByZero,
    IndexOutOfBounds,
    UndefinedVariable,
    UndefinedFunction,
    NullPointer,
    StackOverflow,
    UnhandledException,
    InvalidOperation,
    Internal,
}

impl RuntimeError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind,
        }
    }

    pub fn type_error(expected: &str, got: &str) -> Self {
        Self::new(
            ErrorKind::TypeError,
            format!("خطأ في النوع: متوقع {}، وُجد {}", expected, got),
        )
    }

    pub fn division_by_zero() -> Self {
        Self::new(
            ErrorKind::DivisionByZero,
            "القسمة على صفر",
        )
    }

    pub fn index_out_of_bounds(index: i64, len: usize) -> Self {
        Self::new(
            ErrorKind::IndexOutOfBounds,
            format!("الفهرس {} خارج حدود المصفوفة ذات الطول {}", index, len),
        )
    }

    pub fn undefined_variable(name: &str) -> Self {
        Self::new(
            ErrorKind::UndefinedVariable,
            format!("متغير غير معرّف: {}", name),
        )
    }

    pub fn undefined_function(name: &str) -> Self {
        Self::new(
            ErrorKind::UndefinedFunction,
            format!("دالة غير معرّفة: {}", name),
        )
    }

    pub fn null_pointer() -> Self {
        Self::new(
            ErrorKind::NullPointer,
            "محاولة الوصول لمؤشر فارغ",
        )
    }

    pub fn stack_overflow() -> Self {
        Self::new(
            ErrorKind::StackOverflow,
            "تجاوز المكدس: استدعاءات دالة متداخلة كثيرة جداً",
        )
    }

    pub fn unhandled_exception(msg: &str) -> Self {
        Self::new(
            ErrorKind::UnhandledException,
            format!("استثناء غير معالج: {}", msg),
        )
    }

    pub fn invalid_operation(msg: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidOperation, msg)
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        Self::new(
            ErrorKind::Internal,
            format!("خطأ داخلي: {}", msg),
        )
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RuntimeError {}
