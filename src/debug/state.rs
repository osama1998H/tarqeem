//! Debug session state management
//!
//! This module defines the state machine for the debug session,
//! including pause reasons, step modes, and debug events.

use crate::interpreter::Value;
use crate::ir::{BlockId, FuncId, VarId};

use super::source_map::SourceLocation;
use super::BreakpointId;

/// The current state of a debug session
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugState {
    /// The debugger has not started execution
    NotStarted,

    /// The program is currently running
    Running,

    /// The program is paused
    Paused { reason: PauseReason },

    /// The program is stepping (executing one step at a time)
    Stepping { mode: StepMode },

    /// The program has terminated normally
    Terminated { exit_value: Option<String> },

    /// The program terminated due to an error
    Error { message: String, message_ar: String },
}

impl Default for DebugState {
    fn default() -> Self {
        Self::NotStarted
    }
}

impl DebugState {
    /// Check if the debugger is in a paused state
    pub fn is_paused(&self) -> bool {
        matches!(self, DebugState::Paused { .. })
    }

    /// Check if the debugger is running
    pub fn is_running(&self) -> bool {
        matches!(self, DebugState::Running | DebugState::Stepping { .. })
    }

    /// Check if execution has ended
    pub fn is_terminated(&self) -> bool {
        matches!(
            self,
            DebugState::Terminated { .. } | DebugState::Error { .. }
        )
    }

    /// Get a human-readable state description
    pub fn description(&self) -> &str {
        match self {
            DebugState::NotStarted => "Not started / لم يبدأ",
            DebugState::Running => "Running / جارٍ التنفيذ",
            DebugState::Paused { .. } => "Paused / متوقف",
            DebugState::Stepping { .. } => "Stepping / خطوة بخطوة",
            DebugState::Terminated { .. } => "Terminated / انتهى",
            DebugState::Error { .. } => "Error / خطأ",
        }
    }

    /// Get Arabic state description
    pub fn description_ar(&self) -> &str {
        match self {
            DebugState::NotStarted => "لم يبدأ",
            DebugState::Running => "جارٍ التنفيذ",
            DebugState::Paused { .. } => "متوقف",
            DebugState::Stepping { .. } => "خطوة بخطوة",
            DebugState::Terminated { .. } => "انتهى",
            DebugState::Error { .. } => "خطأ",
        }
    }
}

/// Reason for pausing execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PauseReason {
    /// Hit a breakpoint
    Breakpoint { id: BreakpointId },

    /// Completed a step operation
    Step,

    /// User requested pause
    UserRequest,

    /// Entry point reached (when starting in paused mode)
    Entry,

    /// Exception was thrown
    Exception { message: String },

    /// Data breakpoint/watchpoint hit
    DataBreakpoint {
        variable: String,
        old_value: String,
        new_value: String,
    },
}

impl PauseReason {
    /// Get a human-readable description
    pub fn description(&self) -> String {
        match self {
            PauseReason::Breakpoint { id } => {
                format!("Breakpoint {} hit / وصل إلى نقطة التوقف {}", id.0, id.0)
            }
            PauseReason::Step => "Step completed / اكتملت الخطوة".to_string(),
            PauseReason::UserRequest => "Paused by user / أوقفه المستخدم".to_string(),
            PauseReason::Entry => "Stopped at entry / توقف عند البداية".to_string(),
            PauseReason::Exception { message } => {
                format!("Exception: {} / استثناء: {}", message, message)
            }
            PauseReason::DataBreakpoint {
                variable,
                old_value,
                new_value,
            } => {
                format!(
                    "Variable '{}' changed: {} -> {} / تغير المتغير '{}': {} -> {}",
                    variable, old_value, new_value, variable, old_value, new_value
                )
            }
        }
    }

    /// Get Arabic description
    pub fn description_ar(&self) -> String {
        match self {
            PauseReason::Breakpoint { id } => format!("وصل إلى نقطة التوقف {}", id.0),
            PauseReason::Step => "اكتملت الخطوة".to_string(),
            PauseReason::UserRequest => "أوقفه المستخدم".to_string(),
            PauseReason::Entry => "توقف عند البداية".to_string(),
            PauseReason::Exception { message } => format!("استثناء: {}", message),
            PauseReason::DataBreakpoint {
                variable,
                old_value,
                new_value,
            } => {
                format!(
                    "تغير المتغير '{}': {} -> {}",
                    variable, old_value, new_value
                )
            }
        }
    }
}

/// Step execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    /// Step to next source line (step over function calls)
    /// خطوة فوق
    Over,

    /// Step into function calls
    /// خطوة داخل
    Into,

    /// Step out of current function
    /// خطوة خارج
    Out,

    /// Step one instruction (regardless of source line)
    /// خطوة تعليمة
    Instruction,
}

impl StepMode {
    /// Get human-readable name
    pub fn name(&self) -> &str {
        match self {
            StepMode::Over => "Step Over / خطوة فوق",
            StepMode::Into => "Step Into / خطوة داخل",
            StepMode::Out => "Step Out / خطوة خارج",
            StepMode::Instruction => "Step Instruction / خطوة تعليمة",
        }
    }

    /// Get Arabic name
    pub fn name_ar(&self) -> &str {
        match self {
            StepMode::Over => "خطوة فوق",
            StepMode::Into => "خطوة داخل",
            StepMode::Out => "خطوة خارج",
            StepMode::Instruction => "خطوة تعليمة",
        }
    }
}

/// Events emitted by the debugger
#[derive(Debug, Clone)]
pub enum DebugEvent {
    /// Debug session started
    Started,

    /// Execution paused
    Stopped {
        reason: PauseReason,
        location: Option<SourceLocation>,
    },

    /// Execution continued
    Continued,

    /// A breakpoint was hit
    BreakpointHit {
        id: BreakpointId,
        location: SourceLocation,
    },

    /// An exception occurred
    ExceptionOccurred {
        message: String,
        location: Option<SourceLocation>,
    },

    /// Output was produced
    Output {
        text: String,
        category: OutputCategory,
    },

    /// Program terminated
    Exited { exit_code: i32 },

    /// Debug session ended
    Terminated,
}

/// Category of debug output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputCategory {
    /// Standard output
    Stdout,
    /// Standard error
    Stderr,
    /// Debug console output
    Console,
}

/// A stack frame in the call stack
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Frame ID (unique within session)
    pub id: u32,
    /// Function name
    pub name: String,
    /// Source location (if available)
    pub location: Option<SourceLocation>,
    /// Function ID
    pub func_id: FuncId,
    /// Current block
    pub block_id: BlockId,
    /// Current instruction index
    pub inst_idx: usize,
}

impl StackFrame {
    pub fn new(id: u32, name: String, func_id: FuncId, block_id: BlockId, inst_idx: usize) -> Self {
        Self {
            id,
            name,
            location: None,
            func_id,
            block_id,
            inst_idx,
        }
    }

    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }
}

/// A variable in the debug context
#[derive(Debug, Clone)]
pub struct DebugVariable {
    /// Variable name
    pub name: String,
    /// Variable value (formatted for display)
    pub value: String,
    /// Variable type
    pub type_name: String,
    /// VarId if this is a local variable
    pub var_id: Option<VarId>,
    /// Whether the value can be modified
    pub is_mutable: bool,
    /// Number of child variables (for expandable types)
    pub children_count: usize,
}

impl DebugVariable {
    pub fn new(name: String, value: Value, is_mutable: bool) -> Self {
        let type_name = value.type_name().to_string();
        let formatted_value = value.to_display_string();
        let children_count = match &value {
            Value::Array(arr) => arr.borrow().len(),
            Value::Object(obj) => obj.borrow().fields.len(),
            _ => 0,
        };

        Self {
            name,
            value: formatted_value,
            type_name,
            var_id: None,
            is_mutable,
            children_count,
        }
    }

    pub fn with_var_id(mut self, var_id: VarId) -> Self {
        self.var_id = Some(var_id);
        self
    }
}

/// A scope containing variables
#[derive(Debug, Clone)]
pub struct DebugScope {
    /// Scope name
    pub name: String,
    /// Variables in this scope
    pub variables: Vec<DebugVariable>,
    /// Whether this is an expensive scope to evaluate
    pub expensive: bool,
}

impl DebugScope {
    pub fn new(name: String) -> Self {
        Self {
            name,
            variables: Vec::new(),
            expensive: false,
        }
    }

    pub fn with_variables(mut self, variables: Vec<DebugVariable>) -> Self {
        self.variables = variables;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_state() {
        let state = DebugState::NotStarted;
        assert!(!state.is_paused());
        assert!(!state.is_running());
        assert!(!state.is_terminated());

        let state = DebugState::Running;
        assert!(!state.is_paused());
        assert!(state.is_running());

        let state = DebugState::Paused {
            reason: PauseReason::Step,
        };
        assert!(state.is_paused());
        assert!(!state.is_running());

        let state = DebugState::Terminated { exit_value: None };
        assert!(state.is_terminated());
    }

    #[test]
    fn test_pause_reason() {
        let reason = PauseReason::Breakpoint {
            id: BreakpointId(1),
        };
        assert!(reason.description().contains("1"));

        let reason = PauseReason::Exception {
            message: "Error".to_string(),
        };
        assert!(reason.description().contains("Error"));
    }

    #[test]
    fn test_step_mode() {
        assert!(StepMode::Over.name().contains("Over"));
        assert!(StepMode::Into.name_ar().contains("داخل"));
    }
}
