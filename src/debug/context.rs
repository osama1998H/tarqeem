//! Debug context and breakpoint management
//!
//! This module provides the DebugContext which manages all debug session state
//! including breakpoints, watch expressions, and configuration.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::ir::FuncId;

use super::source_map::{SourceLocation, SourceMap};
use super::state::{DebugState, PauseReason, StepMode};
use super::{DebugError, DebugResult};

/// Unique identifier for a breakpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BreakpointId(pub u32);

impl std::fmt::Display for BreakpointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A breakpoint in the source code
#[derive(Debug, Clone)]
pub struct Breakpoint {
    /// Unique identifier
    pub id: BreakpointId,
    /// Source file
    pub file: PathBuf,
    /// Line number (1-indexed)
    pub line: usize,
    /// Whether the breakpoint is enabled
    pub enabled: bool,
    /// Optional condition expression
    pub condition: Option<String>,
    /// Hit count (how many times this breakpoint should be skipped)
    pub hit_count: Option<u32>,
    /// Current hit count
    pub current_hits: u32,
    /// Log message instead of breaking
    pub log_message: Option<String>,
    /// Whether this breakpoint has been verified as valid
    pub verified: bool,
}

impl Breakpoint {
    /// Create a new breakpoint
    pub fn new(id: BreakpointId, file: PathBuf, line: usize) -> Self {
        Self {
            id,
            file,
            line,
            enabled: true,
            condition: None,
            hit_count: None,
            current_hits: 0,
            log_message: None,
            verified: false,
        }
    }

    /// Set a condition for the breakpoint
    pub fn with_condition(mut self, condition: String) -> Self {
        self.condition = Some(condition);
        self
    }

    /// Set a hit count for the breakpoint
    pub fn with_hit_count(mut self, count: u32) -> Self {
        self.hit_count = Some(count);
        self
    }

    /// Set a log message (logpoint)
    pub fn with_log_message(mut self, message: String) -> Self {
        self.log_message = Some(message);
        self
    }

    /// Enable or disable the breakpoint
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Mark the breakpoint as verified
    pub fn set_verified(&mut self, verified: bool) {
        self.verified = verified;
    }

    /// Check if this breakpoint should trigger
    pub fn should_trigger(&mut self) -> bool {
        if !self.enabled {
            return false;
        }

        self.current_hits += 1;

        if let Some(hit_count) = self.hit_count {
            if self.current_hits < hit_count {
                return false;
            }
        }

        true
    }

    /// Reset the hit counter
    pub fn reset_hits(&mut self) {
        self.current_hits = 0;
    }
}

/// A watch expression
#[derive(Debug, Clone)]
pub struct WatchExpression {
    /// Unique identifier
    pub id: u32,
    /// The expression to evaluate
    pub expression: String,
    /// Last known value
    pub last_value: Option<String>,
    /// Whether to break on value change
    pub break_on_change: bool,
}

impl WatchExpression {
    pub fn new(id: u32, expression: String) -> Self {
        Self {
            id,
            expression,
            last_value: None,
            break_on_change: false,
        }
    }

    pub fn with_break_on_change(mut self, enabled: bool) -> Self {
        self.break_on_change = enabled;
        self
    }
}

/// Debug configuration options
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// Stop at entry point
    pub stop_on_entry: bool,
    /// Maximum stack depth before warning
    pub max_stack_depth: usize,
    /// Timeout for expression evaluation (ms)
    pub eval_timeout_ms: u64,
    /// Show Arabic messages
    pub use_arabic: bool,
    /// Enable console output capture
    pub capture_output: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            stop_on_entry: false,
            max_stack_depth: 1000,
            eval_timeout_ms: 5000,
            use_arabic: true,
            capture_output: true,
        }
    }
}

/// The main debug context managing the debug session
#[derive(Debug)]
pub struct DebugContext {
    /// Current debug state
    state: DebugState,
    /// Breakpoints by ID
    breakpoints: HashMap<BreakpointId, Breakpoint>,
    /// Breakpoints by location (file, line)
    breakpoints_by_location: HashMap<(PathBuf, usize), Vec<BreakpointId>>,
    /// Next breakpoint ID
    next_breakpoint_id: u32,
    /// Watch expressions
    watches: HashMap<u32, WatchExpression>,
    /// Next watch ID
    next_watch_id: u32,
    /// Source map for IR to source mapping
    source_map: SourceMap,
    /// Debug configuration
    config: DebugConfig,
    /// Current step mode (if stepping)
    step_mode: Option<StepMode>,
    /// Stack depth when step started (for step out)
    step_start_depth: usize,
    /// Line when step started (for step over)
    step_start_line: Option<usize>,
    /// Function when step started
    step_start_func: Option<String>,
    /// Output buffer
    output_buffer: Vec<String>,
}

impl Default for DebugContext {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugContext {
    /// Create a new debug context
    pub fn new() -> Self {
        Self {
            state: DebugState::NotStarted,
            breakpoints: HashMap::new(),
            breakpoints_by_location: HashMap::new(),
            next_breakpoint_id: 1,
            watches: HashMap::new(),
            next_watch_id: 1,
            source_map: SourceMap::new(),
            config: DebugConfig::default(),
            step_mode: None,
            step_start_depth: 0,
            step_start_line: None,
            step_start_func: None,
            output_buffer: Vec::new(),
        }
    }

    /// Create with configuration
    pub fn with_config(config: DebugConfig) -> Self {
        let mut ctx = Self::new();
        ctx.config = config;
        ctx
    }

    /// Get the current debug state
    pub fn state(&self) -> &DebugState {
        &self.state
    }

    /// Set the debug state
    pub fn set_state(&mut self, state: DebugState) {
        self.state = state;
    }

    /// Get the configuration
    pub fn config(&self) -> &DebugConfig {
        &self.config
    }

    /// Get mutable configuration
    pub fn config_mut(&mut self) -> &mut DebugConfig {
        &mut self.config
    }

    /// Set the source map
    pub fn set_source_map(&mut self, source_map: SourceMap) {
        self.source_map = source_map;
    }

    /// Get the source map
    pub fn source_map(&self) -> &SourceMap {
        &self.source_map
    }

    /// Get mutable source map
    pub fn source_map_mut(&mut self) -> &mut SourceMap {
        &mut self.source_map
    }

    /// Set source content for a file
    pub fn set_source(&mut self, file: PathBuf, content: String) {
        self.source_map.add_source(file, content);
    }

    // ==================== Breakpoint Management ====================

    /// Add a breakpoint at the specified location
    pub fn add_breakpoint(&mut self, file: PathBuf, line: usize) -> DebugResult<BreakpointId> {
        let id = BreakpointId(self.next_breakpoint_id);
        self.next_breakpoint_id += 1;

        let mut breakpoint = Breakpoint::new(id, file.clone(), line);

        // Try to verify the breakpoint using source map
        if !self.source_map.is_empty() {
            if let Some(valid_line) = self.source_map.find_nearest_breakpoint_line(&file, line) {
                if valid_line != line {
                    // Adjust to valid line
                    breakpoint.line = valid_line;
                }
                breakpoint.verified = true;
            }
        }

        let location_key = (file, breakpoint.line);
        self.breakpoints_by_location
            .entry(location_key)
            .or_default()
            .push(id);

        self.breakpoints.insert(id, breakpoint);

        Ok(id)
    }

    /// Add a conditional breakpoint
    pub fn add_conditional_breakpoint(
        &mut self,
        file: PathBuf,
        line: usize,
        condition: String,
    ) -> DebugResult<BreakpointId> {
        let id = self.add_breakpoint(file, line)?;
        if let Some(bp) = self.breakpoints.get_mut(&id) {
            bp.condition = Some(condition);
        }
        Ok(id)
    }

    /// Remove a breakpoint
    pub fn remove_breakpoint(&mut self, id: BreakpointId) -> DebugResult<()> {
        let bp = self
            .breakpoints
            .remove(&id)
            .ok_or_else(|| DebugError::breakpoint_not_found(id))?;

        let location_key = (bp.file, bp.line);
        if let Some(ids) = self.breakpoints_by_location.get_mut(&location_key) {
            ids.retain(|&bid| bid != id);
            if ids.is_empty() {
                self.breakpoints_by_location.remove(&location_key);
            }
        }

        Ok(())
    }

    /// Clear all breakpoints
    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
        self.breakpoints_by_location.clear();
    }

    /// Get a breakpoint by ID
    pub fn get_breakpoint(&self, id: BreakpointId) -> Option<&Breakpoint> {
        self.breakpoints.get(&id)
    }

    /// Get mutable breakpoint by ID
    pub fn get_breakpoint_mut(&mut self, id: BreakpointId) -> Option<&mut Breakpoint> {
        self.breakpoints.get_mut(&id)
    }

    /// Get all breakpoints
    pub fn breakpoints(&self) -> impl Iterator<Item = &Breakpoint> {
        self.breakpoints.values()
    }

    /// Get breakpoints at a specific location
    pub fn breakpoints_at(&self, file: &PathBuf, line: usize) -> Vec<&Breakpoint> {
        let key = (file.clone(), line);
        self.breakpoints_by_location
            .get(&key)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.breakpoints.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if there's a breakpoint at the given location
    pub fn has_breakpoint_at(&self, file: &PathBuf, line: usize) -> bool {
        let key = (file.clone(), line);
        self.breakpoints_by_location
            .get(&key)
            .map(|ids| !ids.is_empty())
            .unwrap_or(false)
    }

    /// Enable or disable a breakpoint
    pub fn set_breakpoint_enabled(&mut self, id: BreakpointId, enabled: bool) -> DebugResult<()> {
        self.breakpoints
            .get_mut(&id)
            .ok_or_else(|| DebugError::breakpoint_not_found(id))?
            .enabled = enabled;
        Ok(())
    }

    /// Toggle a breakpoint
    pub fn toggle_breakpoint(&mut self, id: BreakpointId) -> DebugResult<bool> {
        let bp = self
            .breakpoints
            .get_mut(&id)
            .ok_or_else(|| DebugError::breakpoint_not_found(id))?;
        bp.enabled = !bp.enabled;
        Ok(bp.enabled)
    }

    // ==================== Watch Expression Management ====================

    /// Add a watch expression
    pub fn add_watch(&mut self, expression: String) -> u32 {
        let id = self.next_watch_id;
        self.next_watch_id += 1;

        let watch = WatchExpression::new(id, expression);
        self.watches.insert(id, watch);

        id
    }

    /// Add a data breakpoint (watch with break on change)
    pub fn add_data_breakpoint(&mut self, expression: String) -> u32 {
        let id = self.next_watch_id;
        self.next_watch_id += 1;

        let watch = WatchExpression::new(id, expression).with_break_on_change(true);
        self.watches.insert(id, watch);

        id
    }

    /// Remove a watch expression
    pub fn remove_watch(&mut self, id: u32) -> bool {
        self.watches.remove(&id).is_some()
    }

    /// Get watch expressions
    pub fn watches(&self) -> impl Iterator<Item = &WatchExpression> {
        self.watches.values()
    }

    /// Get mutable watch expression
    pub fn get_watch_mut(&mut self, id: u32) -> Option<&mut WatchExpression> {
        self.watches.get_mut(&id)
    }

    // ==================== Stepping Control ====================

    /// Start stepping
    pub fn start_stepping(
        &mut self,
        mode: StepMode,
        current_depth: usize,
        current_line: Option<usize>,
        current_func: Option<&str>,
    ) {
        self.step_mode = Some(mode);
        self.step_start_depth = current_depth;
        self.step_start_line = current_line;
        self.step_start_func = current_func.map(|s| s.to_string());
        self.state = DebugState::Stepping { mode };
    }

    /// Stop stepping
    pub fn stop_stepping(&mut self) {
        self.step_mode = None;
        self.step_start_depth = 0;
        self.step_start_line = None;
        self.step_start_func = None;
    }

    /// Get current step mode
    pub fn step_mode(&self) -> Option<StepMode> {
        self.step_mode
    }

    /// Check if a step is complete based on current state
    pub fn is_step_complete(
        &self,
        current_depth: usize,
        current_line: Option<usize>,
        current_func: Option<&str>,
    ) -> bool {
        let Some(mode) = self.step_mode else {
            return false;
        };

        match mode {
            StepMode::Instruction => true,
            StepMode::Into => {
                // Complete when we're at a different line or different function
                current_line != self.step_start_line
                    || current_func.map(|s| s.to_string()) != self.step_start_func
            }
            StepMode::Over => {
                // Complete when we're at a different line at same or lower depth
                current_depth <= self.step_start_depth && current_line != self.step_start_line
            }
            StepMode::Out => {
                // Complete when we've returned to a lower depth
                current_depth < self.step_start_depth
            }
        }
    }

    // ==================== Output Management ====================

    /// Add output to the buffer
    pub fn add_output(&mut self, output: String) {
        self.output_buffer.push(output);
    }

    /// Get and clear output buffer
    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.output_buffer)
    }

    /// Get output buffer
    pub fn output(&self) -> &[String] {
        &self.output_buffer
    }

    // ==================== Source Lookup ====================

    /// Get source line content
    pub fn get_source_line(&self, file: &PathBuf, line: usize) -> Option<&str> {
        self.source_map.get_source_line(file, line)
    }

    /// Get source location for current execution point
    pub fn get_source_location(
        &self,
        func_id: &FuncId,
        block_id: crate::ir::BlockId,
        inst_idx: usize,
    ) -> Option<&SourceLocation> {
        self.source_map
            .get_instruction_location(func_id, block_id, inst_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breakpoint_creation() {
        let mut ctx = DebugContext::new();
        let file = PathBuf::from("test.trq");

        let id = ctx.add_breakpoint(file.clone(), 10).unwrap();
        assert!(ctx.has_breakpoint_at(&file, 10));

        let bp = ctx.get_breakpoint(id).unwrap();
        assert_eq!(bp.line, 10);
        assert!(bp.enabled);
    }

    #[test]
    fn test_breakpoint_removal() {
        let mut ctx = DebugContext::new();
        let file = PathBuf::from("test.trq");

        let id = ctx.add_breakpoint(file.clone(), 10).unwrap();
        assert!(ctx.has_breakpoint_at(&file, 10));

        ctx.remove_breakpoint(id).unwrap();
        assert!(!ctx.has_breakpoint_at(&file, 10));
    }

    #[test]
    fn test_breakpoint_toggle() {
        let mut ctx = DebugContext::new();
        let file = PathBuf::from("test.trq");

        let id = ctx.add_breakpoint(file, 10).unwrap();

        assert!(ctx.get_breakpoint(id).unwrap().enabled);
        ctx.toggle_breakpoint(id).unwrap();
        assert!(!ctx.get_breakpoint(id).unwrap().enabled);
        ctx.toggle_breakpoint(id).unwrap();
        assert!(ctx.get_breakpoint(id).unwrap().enabled);
    }

    #[test]
    fn test_conditional_breakpoint() {
        let mut ctx = DebugContext::new();
        let file = PathBuf::from("test.trq");

        let id = ctx
            .add_conditional_breakpoint(file, 10, "x > 5".to_string())
            .unwrap();

        let bp = ctx.get_breakpoint(id).unwrap();
        assert_eq!(bp.condition, Some("x > 5".to_string()));
    }

    #[test]
    fn test_watch_expression() {
        let mut ctx = DebugContext::new();

        let id = ctx.add_watch("x + y".to_string());
        assert!(ctx.watches().any(|w| w.id == id));

        ctx.remove_watch(id);
        assert!(!ctx.watches().any(|w| w.id == id));
    }

    #[test]
    fn test_stepping() {
        let mut ctx = DebugContext::new();

        ctx.start_stepping(StepMode::Over, 1, Some(10), Some("main"));
        assert_eq!(ctx.step_mode(), Some(StepMode::Over));

        // Same line - not complete
        assert!(!ctx.is_step_complete(1, Some(10), Some("main")));
        // Different line at same depth - complete
        assert!(ctx.is_step_complete(1, Some(11), Some("main")));
        // Same line at deeper depth - not complete (inside function)
        assert!(!ctx.is_step_complete(2, Some(20), Some("inner")));

        ctx.stop_stepping();
        assert_eq!(ctx.step_mode(), None);
    }

    #[test]
    fn test_step_out() {
        let mut ctx = DebugContext::new();

        ctx.start_stepping(StepMode::Out, 2, Some(10), Some("inner"));

        // Same depth - not complete
        assert!(!ctx.is_step_complete(2, Some(15), Some("inner")));
        // Lower depth - complete (returned)
        assert!(ctx.is_step_complete(1, Some(5), Some("outer")));
    }
}
