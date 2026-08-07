//! On-Stack Replacement (OSR)
//!
//! OSR allows switching from interpreted to compiled code mid-execution,
//! essential for long-running loops that become hot during execution.
//!
//! الاستبدال على المكدس (OSR)
//! يسمح بالتحول من التفسير إلى الكود المترجم أثناء التنفيذ
//! ضروري للحلقات الطويلة التي تصبح ساخنة أثناء التشغيل

use std::collections::HashMap;

use crate::interpreter::Value;
use crate::ir::{BlockId, VarId};

/// OSR entry point for a compiled function
///
/// Represents a point in compiled code where execution can enter
/// from the interpreter (typically at loop headers).
#[derive(Debug, Clone)]
pub struct OsrEntry {
    /// Block ID where OSR can occur (usually a loop header)
    pub block_id: BlockId,

    /// Offset into compiled code for this entry point
    pub native_offset: usize,

    /// Mapping from IR variable to native stack slot offset
    /// Used to transfer interpreter state to native frame
    pub var_mapping: Vec<(VarId, i32)>,

    /// Size of the native stack frame at this entry point
    pub frame_size: usize,
}

impl OsrEntry {
    /// Create a new OSR entry point
    pub fn new(block_id: BlockId, native_offset: usize, frame_size: usize) -> Self {
        Self {
            block_id,
            native_offset,
            var_mapping: Vec::new(),
            frame_size,
        }
    }

    /// Add a variable mapping
    pub fn add_var_mapping(&mut self, var_id: VarId, stack_offset: i32) {
        self.var_mapping.push((var_id, stack_offset));
    }
}

/// State for managing OSR transitions
#[derive(Debug, Default)]
pub struct OsrState {
    /// OSR entry points indexed by function name and block ID
    entries: HashMap<String, Vec<OsrEntry>>,

    /// Number of loop iterations per (function, block) pair
    /// Used to determine when to trigger OSR
    loop_counters: HashMap<(String, u32), u64>,

    /// Threshold for triggering OSR compilation
    osr_threshold: u64,

    /// Statistics
    osr_triggers: u64,
    osr_successes: u64,
    osr_failures: u64,
}

impl OsrState {
    /// Create new OSR state with default threshold
    pub fn new() -> Self {
        Self::with_threshold(10_000)
    }

    /// Create OSR state with custom threshold
    pub fn with_threshold(threshold: u64) -> Self {
        Self {
            entries: HashMap::new(),
            loop_counters: HashMap::new(),
            osr_threshold: threshold,
            osr_triggers: 0,
            osr_successes: 0,
            osr_failures: 0,
        }
    }

    /// Register an OSR entry point for a function
    pub fn register_entry(&mut self, func_name: &str, entry: OsrEntry) {
        self.entries
            .entry(func_name.to_string())
            .or_default()
            .push(entry);
    }

    /// Record a loop iteration and check if OSR should trigger
    pub fn record_loop_iteration(&mut self, func_name: &str, block_id: u32) -> OsrTrigger {
        let key = (func_name.to_string(), block_id);
        let counter = self.loop_counters.entry(key).or_insert(0);
        *counter += 1;

        if *counter == self.osr_threshold {
            self.osr_triggers += 1;
            OsrTrigger::ShouldCompile
        } else if *counter > self.osr_threshold {
            // Check if we have a compiled entry point
            if let Some(entries) = self.entries.get(func_name) {
                if entries.iter().any(|e| e.block_id.0 == block_id) {
                    return OsrTrigger::ReadyToTransfer;
                }
            }
            OsrTrigger::WaitingForCompilation
        } else {
            OsrTrigger::Continue
        }
    }

    /// Get OSR entry point for a specific block
    pub fn get_entry(&self, func_name: &str, block_id: BlockId) -> Option<&OsrEntry> {
        self.entries
            .get(func_name)?
            .iter()
            .find(|e| e.block_id == block_id)
    }

    /// Prepare the native frame for OSR transfer
    ///
    /// Converts interpreter locals to native stack frame format
    pub fn prepare_transfer(
        &self,
        func_name: &str,
        block_id: BlockId,
        locals: &HashMap<VarId, Value>,
    ) -> Option<OsrTransfer> {
        let entry = self.get_entry(func_name, block_id)?;

        // Allocate native frame
        let mut native_frame = vec![0u8; entry.frame_size];

        // Copy each variable to its native stack location
        for (var_id, stack_offset) in &entry.var_mapping {
            if let Some(value) = locals.get(var_id) {
                let bytes = value_to_bytes(value);
                let offset = *stack_offset as usize;
                if offset + bytes.len() <= native_frame.len() {
                    native_frame[offset..offset + bytes.len()].copy_from_slice(&bytes);
                }
            }
        }

        Some(OsrTransfer {
            native_offset: entry.native_offset,
            native_frame,
        })
    }

    /// Record a successful OSR transfer
    pub fn record_success(&mut self) {
        self.osr_successes += 1;
    }

    /// Record a failed OSR transfer
    pub fn record_failure(&mut self) {
        self.osr_failures += 1;
    }

    /// Reset loop counter for a specific block
    pub fn reset_counter(&mut self, func_name: &str, block_id: u32) {
        let key = (func_name.to_string(), block_id);
        self.loop_counters.remove(&key);
    }

    /// Get OSR statistics
    pub fn stats(&self) -> OsrStats {
        OsrStats {
            entry_points: self.entries.values().map(|v| v.len()).sum(),
            triggers: self.osr_triggers,
            successes: self.osr_successes,
            failures: self.osr_failures,
            success_rate: if self.osr_triggers > 0 {
                self.osr_successes as f64 / self.osr_triggers as f64
            } else {
                0.0
            },
        }
    }

    /// Clear all OSR state
    pub fn clear(&mut self) {
        self.entries.clear();
        self.loop_counters.clear();
    }
}

/// Result of checking a loop iteration for OSR
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsrTrigger {
    /// Continue interpreting, threshold not reached
    Continue,

    /// Threshold reached, should compile this loop
    ShouldCompile,

    /// Waiting for compilation to complete
    WaitingForCompilation,

    /// Compiled code is ready, can transfer
    ReadyToTransfer,
}

/// Data needed to transfer execution to compiled code
#[derive(Debug)]
pub struct OsrTransfer {
    /// Offset into compiled code to jump to
    pub native_offset: usize,

    /// Native stack frame with transferred values
    pub native_frame: Vec<u8>,
}

/// OSR statistics
#[derive(Debug, Clone)]
pub struct OsrStats {
    /// Number of registered OSR entry points
    pub entry_points: usize,

    /// Number of OSR triggers (threshold reached)
    pub triggers: u64,

    /// Number of successful OSR transfers
    pub successes: u64,

    /// Number of failed OSR transfers
    pub failures: u64,

    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
}

impl std::fmt::Display for OsrStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "OSR Statistics / إحصائيات الاستبدال على المكدس")?;
        writeln!(f, "================================================")?;
        writeln!(f, "Entry points / نقاط الدخول: {}", self.entry_points)?;
        writeln!(f, "Triggers / التفعيلات: {}", self.triggers)?;
        writeln!(f, "Successes / النجاحات: {}", self.successes)?;
        writeln!(f, "Failures / الإخفاقات: {}", self.failures)?;
        writeln!(
            f,
            "Success rate / نسبة النجاح: {:.2}%",
            self.success_rate * 100.0
        )?;
        Ok(())
    }
}

/// Convert interpreter Value to raw bytes for native frame
fn value_to_bytes(value: &Value) -> Vec<u8> {
    match value {
        Value::Null => vec![0u8; 8],
        Value::Bool(b) => {
            let v = if *b { 1i64 } else { 0i64 };
            v.to_le_bytes().to_vec()
        }
        Value::Int(i) => i.to_le_bytes().to_vec(),
        Value::Float(f) => f.to_le_bytes().to_vec(),
        Value::String(_) => {
            // Strings are pointers - for now just use 0
            // Real implementation would use string interning
            0i64.to_le_bytes().to_vec()
        }
        Value::Array(_) => {
            // Arrays are pointers
            0i64.to_le_bytes().to_vec()
        }
        Value::Object(_) => {
            // Objects are pointers
            0i64.to_le_bytes().to_vec()
        }
        Value::Function(_) => {
            // Function pointers
            0i64.to_le_bytes().to_vec()
        }
        Value::Ptr(_) => {
            // Pointer values
            0i64.to_le_bytes().to_vec()
        }
        Value::Enum { discriminant, .. } => {
            // Enums - store discriminant as i64
            discriminant.to_le_bytes().to_vec()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osr_entry_creation() {
        let mut entry = OsrEntry::new(BlockId(1), 0x100, 64);
        entry.add_var_mapping(VarId(0), 0);
        entry.add_var_mapping(VarId(1), 8);

        assert_eq!(entry.block_id, BlockId(1));
        assert_eq!(entry.native_offset, 0x100);
        assert_eq!(entry.frame_size, 64);
        assert_eq!(entry.var_mapping.len(), 2);
    }

    #[test]
    fn test_osr_state_loop_counting() {
        let mut state = OsrState::with_threshold(100);

        // First iterations should continue
        for _ in 0..99 {
            assert_eq!(state.record_loop_iteration("test", 0), OsrTrigger::Continue);
        }

        // 100th iteration should trigger compilation
        assert_eq!(
            state.record_loop_iteration("test", 0),
            OsrTrigger::ShouldCompile
        );

        // After threshold, wait for compilation
        assert_eq!(
            state.record_loop_iteration("test", 0),
            OsrTrigger::WaitingForCompilation
        );
    }

    #[test]
    fn test_osr_entry_registration() {
        let mut state = OsrState::new();

        let entry = OsrEntry::new(BlockId(1), 0x100, 64);
        state.register_entry("test_func", entry);

        assert!(state.get_entry("test_func", BlockId(1)).is_some());
        assert!(state.get_entry("test_func", BlockId(2)).is_none());
        assert!(state.get_entry("other_func", BlockId(1)).is_none());
    }

    #[test]
    fn test_osr_ready_to_transfer() {
        let mut state = OsrState::with_threshold(10);

        // Register entry first
        let entry = OsrEntry::new(BlockId(0), 0x100, 64);
        state.register_entry("test", entry);

        // Reach threshold
        for _ in 0..10 {
            state.record_loop_iteration("test", 0);
        }

        // After threshold with entry registered, should be ready
        assert_eq!(
            state.record_loop_iteration("test", 0),
            OsrTrigger::ReadyToTransfer
        );
    }

    #[test]
    fn test_osr_stats() {
        let mut state = OsrState::with_threshold(10);

        let entry = OsrEntry::new(BlockId(0), 0x100, 64);
        state.register_entry("test", entry);

        // Generate some stats
        for _ in 0..10 {
            state.record_loop_iteration("test", 0);
        }
        state.record_success();
        state.record_success();
        state.record_failure();

        let stats = state.stats();
        assert_eq!(stats.entry_points, 1);
        assert_eq!(stats.triggers, 1);
        assert_eq!(stats.successes, 2);
        assert_eq!(stats.failures, 1);
    }

    #[test]
    fn test_value_to_bytes() {
        assert_eq!(
            value_to_bytes(&Value::Int(42)),
            42i64.to_le_bytes().to_vec()
        );
        assert_eq!(
            value_to_bytes(&Value::Float(2.5)),
            2.5f64.to_le_bytes().to_vec()
        );
        assert_eq!(
            value_to_bytes(&Value::Bool(true)),
            1i64.to_le_bytes().to_vec()
        );
        assert_eq!(
            value_to_bytes(&Value::Bool(false)),
            0i64.to_le_bytes().to_vec()
        );
    }
}
