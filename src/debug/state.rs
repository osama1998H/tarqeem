//! Debug session state management
//!
//! This module defines the state machine for the debug session,
//! including pause reasons, step modes, and debug events.

use crate::interpreter::Value;
use crate::ir::{BlockId, FuncId, VarId};

use super::source_map::SourceLocation;
use super::BreakpointId;

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub enum DebugState {
    #[default]
    NotStarted,

    Running,

    Paused { reason: PauseReason },

    Stepping { mode: StepMode },

    Terminated { exit_value: Option<String> },

    Error { message: String, message_ar: String },
}


impl DebugState {
    pub fn is_paused(&self) -> bool {
        matches!(self, DebugState::Paused { .. })
    }

    pub fn is_running(&self) -> bool {
        matches!(self, DebugState::Running | DebugState::Stepping { .. })
    }

    pub fn is_terminated(&self) -> bool {
        matches!(
            self,
            DebugState::Terminated { .. } | DebugState::Error { .. }
        )
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PauseReason {
    Breakpoint { id: BreakpointId },

    Step,

    UserRequest,

    Entry,

    Exception { message: String },

    DataBreakpoint {
        variable: String,
        old_value: String,
        new_value: String,
    },
}

impl PauseReason {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    Over,

    Into,

    Out,

    Instruction,
}

impl StepMode {
    pub fn name(&self) -> &str {
        match self {
            StepMode::Over => "Step Over / خطوة فوق",
            StepMode::Into => "Step Into / خطوة داخل",
            StepMode::Out => "Step Out / خطوة خارج",
            StepMode::Instruction => "Step Instruction / خطوة تعليمة",
        }
    }

    pub fn name_ar(&self) -> &str {
        match self {
            StepMode::Over => "خطوة فوق",
            StepMode::Into => "خطوة داخل",
            StepMode::Out => "خطوة خارج",
            StepMode::Instruction => "خطوة تعليمة",
        }
    }
}

#[derive(Debug, Clone)]
pub enum DebugEvent {
    Started,

    Stopped {
        reason: PauseReason,
        location: Option<SourceLocation>,
    },

    Continued,

    BreakpointHit {
        id: BreakpointId,
        location: SourceLocation,
    },

    ExceptionOccurred {
        message: String,
        location: Option<SourceLocation>,
    },

    Output {
        text: String,
        category: OutputCategory,
    },

    Exited { exit_code: i32 },

    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputCategory {
    Stdout,
    Stderr,
    Console,
}

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub id: u32,
    pub name: String,
    pub location: Option<SourceLocation>,
    pub func_id: FuncId,
    pub block_id: BlockId,
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

#[derive(Debug, Clone)]
pub struct DebugVariable {
    pub name: String,
    pub value: String,
    pub type_name: String,
    pub var_id: Option<VarId>,
    pub is_mutable: bool,
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

#[derive(Debug, Clone)]
pub struct DebugScope {
    pub name: String,
    pub variables: Vec<DebugVariable>,
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
