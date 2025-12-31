//! # Tarqeem Debugger (trqdbg)
//!
//! This module provides interactive debugging capabilities for Tarqeem programs.
//! It implements the Debug Adapter Protocol (DAP) for IDE integration and
//! provides a CLI-based debugging interface.
//! ### Interactive Commands
//!
//! - `break <line>` / `توقف <سطر>` - Set breakpoint
//! - `continue` / `تابع` - Continue execution
//! - `step` / `خطوة` - Step to next line
//! - `next` / `التالي` - Step over function calls
//! - `out` / `خارج` - Step out of current function
//! - `print <expr>` / `اطبع <تعبير>` - Print expression value
//! - `locals` / `محليات` - Show local variables
//! - `stack` / `مكدس` - Show call stack
//! - `quit` / `اخرج` - Exit debugger

mod adapter;
mod commands;
mod context;
mod interpreter;
mod server;
mod source_map;
mod state;

#[cfg(test)]
mod tests;

pub use adapter::{DapAdapter, DapEvent, DapRequest, DapResponse};
pub use commands::{DebugCommand, DebugCommandParser};
pub use context::{Breakpoint, BreakpointId, DebugContext, WatchExpression};
pub use interpreter::{DebugInterpreter, StepResult};
pub use server::{DapMessage, DapProtocol, DapServer, TransportError, TransportResult};
pub use source_map::{SourceLocation, SourceMap};
pub use state::{
    DebugEvent, DebugState, DebugVariable, HeapAllocation, HeapChild, PauseReason, StackFrame,
    StepMode,
};

#[derive(Debug, Clone)]
pub struct DebugError {
    pub message: String,
}

impl DebugError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        Self {
            message: format!("خطأ داخلي في المصحح: {}", msg),
        }
    }

    pub fn breakpoint_not_found(id: BreakpointId) -> Self {
        Self {
            message: format!("نقطة التوقف {} غير موجودة", id.0),
        }
    }

    pub fn invalid_line(line: usize) -> Self {
        Self {
            message: format!("رقم السطر غير صالح: {}", line),
        }
    }

    pub fn no_source_mapping() -> Self {
        Self {
            message: "لا توجد خريطة مصدر متاحة للموقع الحالي".to_string(),
        }
    }
}

impl std::fmt::Display for DebugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DebugError {}

impl From<crate::interpreter::RuntimeError> for DebugError {
    fn from(err: crate::interpreter::RuntimeError) -> Self {
        Self {
            message: err.message.clone(),
        }
    }
}

pub type DebugResult<T> = Result<T, DebugError>;
