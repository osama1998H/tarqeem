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
    /// `أنهِ_البرنامج` was called: end the program with this exit status.
    ///
    /// Not an error, and carried as one deliberately. The interpreter runs
    /// in-process, so the only faithful alternative would be `process::exit`
    /// inside the builtin arm — which would end the test binary for any
    /// in-process assertion and would let a builtin terminate a host process it
    /// does not own (the REPL, the DAP server). An `Err` instead propagates to
    /// whoever *does* own the process, and `src/cli/commands/mod.rs` turns it
    /// into the requested status.
    ///
    /// It is uncatchable by construction rather than by a guard:
    /// `Executor::take_propagating_exception` routes only `UnhandledException`
    /// to a frame's `try_stack`, so `حاول { أنهِ_البرنامج(٣) }` exits 3 in every
    /// backend instead of running its `التقط`.
    ProgramExit(i32),
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
        Self::new(ErrorKind::DivisionByZero, "القسمة على صفر")
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
        Self::new(ErrorKind::NullPointer, "محاولة الوصول لمؤشر فارغ")
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

    /// Terminate with `status`, which the caller has already reduced to a byte.
    ///
    /// The message is never shown on the normal path — the CLI honours the
    /// status before it prints any diagnostic — so it exists for the cases that
    /// print an `Err` generically, such as a `تطابق` over `Display`.
    pub fn program_exit(status: i32) -> Self {
        Self::new(
            ErrorKind::ProgramExit(status),
            format!("إنهاء البرنامج بالحالة {}", status),
        )
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        Self::new(ErrorKind::Internal, format!("خطأ داخلي: {}", msg))
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RuntimeError {}
