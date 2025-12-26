//! Deoptimization Support
//!
//! Handles transitioning from compiled code back to the interpreter when
//! compiled code encounters a situation it cannot handle.
//!
//! دعم إلغاء التحسين
//! يتعامل مع التحول من الكود المترجم إلى المفسر عند حدوث حالة غير متوقعة

use std::collections::HashMap;

use crate::interpreter::Value;
use crate::ir::{FuncId, VarId};
use crate::jit::profile::CompilationTier;

/// Reasons for deoptimization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeoptReason {
    /// Type guard failed (observed different type than expected)
    TypeGuardFailed,

    /// Null pointer dereference
    NullPointer,

    /// Array bounds check failed
    ArrayBoundsCheck,

    /// Division by zero
    DivisionByZero,

    /// Stack overflow
    StackOverflow,

    /// Uncommon trap hit (rarely-taken branch)
    UncommonTrap,

    /// Inline cache miss (monomorphic → polymorphic)
    InlineCacheMiss,

    /// Class hierarchy changed (invalidates devirtualization)
    ClassHierarchyChanged,

    /// Debug breakpoint hit
    DebugBreakpoint,

    /// Explicit request to deoptimize
    Explicit,

    /// Unknown or unspecified reason
    Unknown,
}

impl std::fmt::Display for DeoptReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (en, ar) = match self {
            DeoptReason::TypeGuardFailed => ("Type guard failed", "فشل حارس النوع"),
            DeoptReason::NullPointer => ("Null pointer", "مؤشر فارغ"),
            DeoptReason::ArrayBoundsCheck => ("Array bounds check", "فحص حدود المصفوفة"),
            DeoptReason::DivisionByZero => ("Division by zero", "قسمة على صفر"),
            DeoptReason::StackOverflow => ("Stack overflow", "طفح المكدس"),
            DeoptReason::UncommonTrap => ("Uncommon trap", "مصيدة نادرة"),
            DeoptReason::InlineCacheMiss => ("Inline cache miss", "إخفاق التخزين المؤقت"),
            DeoptReason::ClassHierarchyChanged => {
                ("Class hierarchy changed", "تغير تسلسل الأصناف")
            }
            DeoptReason::DebugBreakpoint => ("Debug breakpoint", "نقطة توقف تصحيح"),
            DeoptReason::Explicit => ("Explicit deoptimization", "إلغاء تحسين صريح"),
            DeoptReason::Unknown => ("Unknown reason", "سبب غير معروف"),
        };
        write!(f, "{} / {}", en, ar)
    }
}

/// Information about a deoptimization point
#[derive(Debug, Clone)]
pub struct DeoptInfo {
    /// Function that was deoptimized
    pub func_id: FuncId,

    /// Reason for deoptimization
    pub reason: DeoptReason,

    /// Native code offset where deopt occurred
    pub native_offset: usize,

    /// Block ID to resume interpretation at
    pub resume_block: u32,

    /// Instruction index within the block
    pub resume_instruction: u32,

    /// Variable values to restore
    pub var_values: Vec<(VarId, Value)>,
}

impl DeoptInfo {
    /// Create a new deopt info
    pub fn new(func_id: FuncId, reason: DeoptReason) -> Self {
        Self {
            func_id,
            reason,
            native_offset: 0,
            resume_block: 0,
            resume_instruction: 0,
            var_values: Vec::new(),
        }
    }

    /// Set the native offset
    pub fn with_native_offset(mut self, offset: usize) -> Self {
        self.native_offset = offset;
        self
    }

    /// Set the resume point
    pub fn with_resume_point(mut self, block: u32, instruction: u32) -> Self {
        self.resume_block = block;
        self.resume_instruction = instruction;
        self
    }

    /// Add a variable value to restore
    pub fn add_var(&mut self, var_id: VarId, value: Value) {
        self.var_values.push((var_id, value));
    }
}

/// Manages deoptimization state and transitions
#[derive(Debug, Default)]
pub struct Deoptimizer {
    /// Deoptimization info indexed by function and native offset
    deopt_points: HashMap<String, Vec<DeoptInfo>>,

    /// Functions currently marked for deoptimization
    pending_deopts: Vec<FuncId>,

    /// Statistics per reason
    deopt_counts: HashMap<DeoptReason, u64>,

    /// Total deoptimizations
    total_deopts: u64,

    /// Functions that have been deoptimized
    deoptimized_functions: HashMap<String, u64>,

    /// Max deopts before bailing out (stop trying to compile)
    max_deopts_per_function: u64,
}

impl Deoptimizer {
    /// Create a new deoptimizer
    pub fn new() -> Self {
        Self {
            deopt_points: HashMap::new(),
            pending_deopts: Vec::new(),
            deopt_counts: HashMap::new(),
            total_deopts: 0,
            deoptimized_functions: HashMap::new(),
            max_deopts_per_function: 10, // Stop compiling after 10 deopts
        }
    }

    /// Create with custom max deopts
    pub fn with_max_deopts(max: u64) -> Self {
        let mut d = Self::new();
        d.max_deopts_per_function = max;
        d
    }

    /// Register a deoptimization point in compiled code
    pub fn register_deopt_point(&mut self, func_name: &str, info: DeoptInfo) {
        self.deopt_points
            .entry(func_name.to_string())
            .or_default()
            .push(info);
    }

    /// Find deopt info for a given function and native offset
    pub fn find_deopt_info(&self, func_name: &str, native_offset: usize) -> Option<&DeoptInfo> {
        self.deopt_points.get(func_name)?.iter().find(|info| {
            // Find the deopt point closest to (but not after) the offset
            info.native_offset <= native_offset
        })
    }

    /// Record a deoptimization event
    pub fn record_deopt(&mut self, func_name: &str, reason: DeoptReason) {
        self.total_deopts += 1;
        *self.deopt_counts.entry(reason).or_insert(0) += 1;
        *self
            .deoptimized_functions
            .entry(func_name.to_string())
            .or_insert(0) += 1;
    }

    /// Check if a function should be recompiled after deopt
    ///
    /// Returns the appropriate tier, or None if we should give up
    pub fn should_recompile(&self, func_name: &str) -> Option<CompilationTier> {
        let count = self.deoptimized_functions.get(func_name).unwrap_or(&0);

        if *count >= self.max_deopts_per_function {
            // Too many deopts, give up on this function
            None
        } else if *count >= 3 {
            // Several deopts, stay at baseline
            Some(CompilationTier::BaselineCompiled)
        } else {
            // Few deopts, try optimizing again
            Some(CompilationTier::Optimized)
        }
    }

    /// Mark a function for deoptimization
    pub fn mark_for_deopt(&mut self, func_id: FuncId) {
        if !self.pending_deopts.contains(&func_id) {
            self.pending_deopts.push(func_id);
        }
    }

    /// Take all pending deoptimizations
    pub fn take_pending(&mut self) -> Vec<FuncId> {
        std::mem::take(&mut self.pending_deopts)
    }

    /// Check if there are pending deoptimizations
    pub fn has_pending(&self) -> bool {
        !self.pending_deopts.is_empty()
    }

    /// Clear deopt points for a function (e.g., when recompiling)
    pub fn clear_function(&mut self, func_name: &str) {
        self.deopt_points.remove(func_name);
    }

    /// Get statistics
    pub fn stats(&self) -> DeoptStats {
        DeoptStats {
            total_deopts: self.total_deopts,
            by_reason: self.deopt_counts.clone(),
            functions_affected: self.deoptimized_functions.len(),
            functions_bailed_out: self
                .deoptimized_functions
                .values()
                .filter(|&count| *count >= self.max_deopts_per_function)
                .count(),
        }
    }

    /// Reset all statistics
    pub fn reset_stats(&mut self) {
        self.deopt_counts.clear();
        self.total_deopts = 0;
        self.deoptimized_functions.clear();
    }
}

/// Deoptimization statistics
#[derive(Debug, Clone)]
pub struct DeoptStats {
    /// Total number of deoptimizations
    pub total_deopts: u64,

    /// Deoptimizations by reason
    pub by_reason: HashMap<DeoptReason, u64>,

    /// Number of functions that have been deoptimized
    pub functions_affected: usize,

    /// Number of functions that bailed out (stopped being compiled)
    pub functions_bailed_out: usize,
}

impl std::fmt::Display for DeoptStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Deoptimization Statistics / إحصائيات إلغاء التحسين")?;
        writeln!(f, "=====================================================")?;
        writeln!(
            f,
            "Total deopts / إجمالي الإلغاءات: {}",
            self.total_deopts
        )?;
        writeln!(
            f,
            "Functions affected / الدوال المتأثرة: {}",
            self.functions_affected
        )?;
        writeln!(
            f,
            "Functions bailed out / الدوال المتوقفة: {}",
            self.functions_bailed_out
        )?;

        if !self.by_reason.is_empty() {
            writeln!(f, "\nBy reason / حسب السبب:")?;
            for (reason, count) in &self.by_reason {
                writeln!(f, "  {}: {}", reason, count)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deopt_reason_display() {
        let reason = DeoptReason::TypeGuardFailed;
        let display = format!("{}", reason);
        assert!(display.contains("Type guard failed"));
        assert!(display.contains("فشل حارس النوع"));
    }

    #[test]
    fn test_deopt_info_creation() {
        let mut info = DeoptInfo::new(FuncId("test".to_string()), DeoptReason::TypeGuardFailed)
            .with_native_offset(0x100)
            .with_resume_point(2, 5);

        info.add_var(VarId(0), Value::Int(42));

        assert_eq!(info.func_id.0, "test");
        assert_eq!(info.reason, DeoptReason::TypeGuardFailed);
        assert_eq!(info.native_offset, 0x100);
        assert_eq!(info.resume_block, 2);
        assert_eq!(info.resume_instruction, 5);
        assert_eq!(info.var_values.len(), 1);
    }

    #[test]
    fn test_deoptimizer_registration() {
        let mut deopt = Deoptimizer::new();

        let info = DeoptInfo::new(FuncId("test".to_string()), DeoptReason::NullPointer)
            .with_native_offset(0x50);

        deopt.register_deopt_point("test", info);

        assert!(deopt.find_deopt_info("test", 0x50).is_some());
        assert!(deopt.find_deopt_info("test", 0x100).is_some()); // Should find closest before
        assert!(deopt.find_deopt_info("other", 0x50).is_none());
    }

    #[test]
    fn test_deopt_recording() {
        let mut deopt = Deoptimizer::new();

        deopt.record_deopt("func1", DeoptReason::TypeGuardFailed);
        deopt.record_deopt("func1", DeoptReason::TypeGuardFailed);
        deopt.record_deopt("func2", DeoptReason::NullPointer);

        let stats = deopt.stats();
        assert_eq!(stats.total_deopts, 3);
        assert_eq!(stats.functions_affected, 2);
        assert_eq!(stats.by_reason.get(&DeoptReason::TypeGuardFailed), Some(&2));
        assert_eq!(stats.by_reason.get(&DeoptReason::NullPointer), Some(&1));
    }

    #[test]
    fn test_recompile_decision() {
        let mut deopt = Deoptimizer::with_max_deopts(5);

        // First few deopts: try optimizing
        deopt.record_deopt("func1", DeoptReason::TypeGuardFailed);
        assert_eq!(
            deopt.should_recompile("func1"),
            Some(CompilationTier::Optimized)
        );

        // After 3 deopts: stay at baseline
        deopt.record_deopt("func1", DeoptReason::TypeGuardFailed);
        deopt.record_deopt("func1", DeoptReason::TypeGuardFailed);
        assert_eq!(
            deopt.should_recompile("func1"),
            Some(CompilationTier::BaselineCompiled)
        );

        // After max deopts: give up
        deopt.record_deopt("func1", DeoptReason::TypeGuardFailed);
        deopt.record_deopt("func1", DeoptReason::TypeGuardFailed);
        assert_eq!(deopt.should_recompile("func1"), None);
    }

    #[test]
    fn test_pending_deopts() {
        let mut deopt = Deoptimizer::new();

        assert!(!deopt.has_pending());

        deopt.mark_for_deopt(FuncId("func1".to_string()));
        deopt.mark_for_deopt(FuncId("func2".to_string()));
        deopt.mark_for_deopt(FuncId("func1".to_string())); // Duplicate

        assert!(deopt.has_pending());

        let pending = deopt.take_pending();
        assert_eq!(pending.len(), 2);
        assert!(!deopt.has_pending());
    }

    #[test]
    fn test_clear_function() {
        let mut deopt = Deoptimizer::new();

        let info = DeoptInfo::new(FuncId("test".to_string()), DeoptReason::NullPointer);
        deopt.register_deopt_point("test", info);

        assert!(deopt.find_deopt_info("test", 0).is_some());

        deopt.clear_function("test");

        assert!(deopt.find_deopt_info("test", 0).is_none());
    }
}
