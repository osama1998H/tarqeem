//! # Tarqeem Debugger (trqdbg)
//!
//! This module provides interactive debugging capabilities for Tarqeem programs.
//! It implements the Debug Adapter Protocol (DAP) for IDE integration and
//! provides a CLI-based debugging interface.
//!
//! ## Features
//!
//! - **Breakpoints**: Set breakpoints on source lines
//! - **Step Execution**: Step over, into, and out of function calls
//! - **Variable Inspection**: Inspect local and global variables
//! - **Call Stack Tracing**: View the current call stack
//! - **Expression Evaluation**: Evaluate expressions in the current context
//! - **Watch Expressions**: Monitor variable values
//! - **Conditional Breakpoints**: Break only when conditions are met
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Debug Adapter Protocol                     │
//! │                         (DAP Server)                          │
//! └────────────────────────────┬────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      DebugController                          │
//! │  - Manages debug session state                               │
//! │  - Handles breakpoints and watches                           │
//! │  - Coordinates with DebugInterpreter                         │
//! └────────────────────────────┬────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    DebugInterpreter                           │
//! │  - Extends Interpreter with debug hooks                       │
//! │  - Supports step-by-step execution                           │
//! │  - Provides variable inspection                              │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ### CLI Debugging
//!
//! ```bash
//! tarqeem debug program.trq
//! ```
//!
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
mod source_map;
mod state;

#[cfg(test)]
mod tests;

pub use adapter::{DapAdapter, DapRequest, DapResponse};
pub use commands::{DebugCommand, DebugCommandParser};
pub use context::{Breakpoint, BreakpointId, DebugContext, WatchExpression};
pub use interpreter::{DebugInterpreter, StepResult};
pub use source_map::{SourceLocation, SourceMap};
pub use state::{DebugEvent, DebugState, DebugVariable, PauseReason, StackFrame, StepMode};

/// Error type for debugger operations
#[derive(Debug, Clone)]
pub struct DebugError {
    /// English error message
    pub message: String,
    /// Arabic error message
    pub message_ar: String,
}

impl DebugError {
    pub fn new(message: impl Into<String>, message_ar: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            message_ar: message_ar.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        Self {
            message: format!("Internal debugger error: {}", msg),
            message_ar: format!("خطأ داخلي في المصحح: {}", msg),
        }
    }

    pub fn breakpoint_not_found(id: BreakpointId) -> Self {
        Self {
            message: format!("Breakpoint {} not found", id.0),
            message_ar: format!("نقطة التوقف {} غير موجودة", id.0),
        }
    }

    pub fn invalid_line(line: usize) -> Self {
        Self {
            message: format!("Invalid line number: {}", line),
            message_ar: format!("رقم السطر غير صالح: {}", line),
        }
    }

    pub fn no_source_mapping() -> Self {
        Self {
            message: "No source mapping available for current location".to_string(),
            message_ar: "لا توجد خريطة مصدر متاحة للموقع الحالي".to_string(),
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
            message_ar: err.message_ar.clone(),
        }
    }
}

/// Result type for debugger operations
pub type DebugResult<T> = Result<T, DebugError>;
