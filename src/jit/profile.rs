//! JIT Profiling Data Collection
//!
//! This module provides profiling infrastructure for JIT compilation decisions.
//! It tracks function call counts, branch behavior, and observed types to make
//! informed tier-up decisions.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Compilation tier for a function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CompilationTier {
    /// Tier 0: Interpreted execution (default)
    #[default]
    Interpreted,

    /// Tier 1: Baseline JIT compilation (fast compile, ~5x speedup)
    BaselineCompiled,

    /// Tier 2: Optimizing JIT compilation (slow compile, ~native speed)
    Optimized,
}

impl CompilationTier {
    /// Returns the next tier (for tier-up decisions)
    pub fn next_tier(&self) -> Option<CompilationTier> {
        match self {
            CompilationTier::Interpreted => Some(CompilationTier::BaselineCompiled),
            CompilationTier::BaselineCompiled => Some(CompilationTier::Optimized),
            CompilationTier::Optimized => None,
        }
    }

    /// Returns the Arabic name for this tier
    pub fn name_ar(&self) -> &'static str {
        match self {
            CompilationTier::Interpreted => "مُفسَّر",
            CompilationTier::BaselineCompiled => "ترجمة أساسية",
            CompilationTier::Optimized => "ترجمة مُحسَّنة",
        }
    }

    /// Returns the English name for this tier
    pub fn name_en(&self) -> &'static str {
        match self {
            CompilationTier::Interpreted => "Interpreted",
            CompilationTier::BaselineCompiled => "Baseline Compiled",
            CompilationTier::Optimized => "Optimized",
        }
    }
}

impl std::fmt::Display for CompilationTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name_en())
    }
}

/// Branch profiling data for a single conditional
#[derive(Debug, Default)]
pub struct BranchProfile {
    /// Number of times the branch was taken
    pub taken_count: AtomicU64,

    /// Number of times the branch was not taken
    pub not_taken_count: AtomicU64,
}

impl BranchProfile {
    /// Create a new empty branch profile
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a branch taken event
    pub fn record_taken(&self) {
        self.taken_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a branch not taken event
    pub fn record_not_taken(&self) {
        self.not_taken_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the total number of executions
    pub fn total(&self) -> u64 {
        self.taken_count.load(Ordering::Relaxed) + self.not_taken_count.load(Ordering::Relaxed)
    }

    /// Get the probability that the branch is taken (0.0 to 1.0)
    pub fn taken_probability(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            0.5 // Unknown, assume 50/50
        } else {
            self.taken_count.load(Ordering::Relaxed) as f64 / total as f64
        }
    }

    /// Returns true if this branch is biased (>90% one way)
    pub fn is_biased(&self) -> bool {
        let prob = self.taken_probability();
        prob > 0.9 || prob < 0.1
    }
}

impl Clone for BranchProfile {
    fn clone(&self) -> Self {
        Self {
            taken_count: AtomicU64::new(self.taken_count.load(Ordering::Relaxed)),
            not_taken_count: AtomicU64::new(self.not_taken_count.load(Ordering::Relaxed)),
        }
    }
}

/// Observed type information for type specialization
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ObservedType {
    /// Always observed as integer
    AlwaysInt,

    /// Always observed as float
    AlwaysFloat,

    /// Always observed as string
    AlwaysString,

    /// Always observed as boolean
    AlwaysBool,

    /// Always observed as null
    AlwaysNull,

    /// Always observed as a specific object type
    AlwaysObject(String),

    /// Always observed as an array
    AlwaysArray,

    /// Multiple different types observed (cannot specialize)
    Mixed,

    /// No observations yet
    Unknown,
}

impl ObservedType {
    /// Merge two observed types
    pub fn merge(&self, other: &ObservedType) -> ObservedType {
        if self == other {
            self.clone()
        } else if matches!(self, ObservedType::Unknown) {
            other.clone()
        } else if matches!(other, ObservedType::Unknown) {
            self.clone()
        } else {
            ObservedType::Mixed
        }
    }

    /// Returns true if this type can be specialized
    pub fn can_specialize(&self) -> bool {
        !matches!(self, ObservedType::Mixed | ObservedType::Unknown)
    }
}

impl Default for ObservedType {
    fn default() -> Self {
        ObservedType::Unknown
    }
}

/// Profile data for a single function
#[derive(Debug)]
pub struct FunctionProfile {
    /// Function name
    pub name: String,

    /// Number of times this function was called
    call_count: AtomicU64,

    /// Number of loop iterations executed in this function
    loop_iterations: AtomicU64,

    /// Current compilation tier
    tier: std::sync::atomic::AtomicU8,

    /// Observed argument types (for type specialization)
    pub arg_types: Vec<ObservedType>,

    /// Branch profiles indexed by block ID
    pub branch_profiles: HashMap<u32, BranchProfile>,
}

impl FunctionProfile {
    /// Create a new function profile
    pub fn new(name: String) -> Self {
        Self {
            name,
            call_count: AtomicU64::new(0),
            loop_iterations: AtomicU64::new(0),
            tier: std::sync::atomic::AtomicU8::new(0),
            arg_types: Vec::new(),
            branch_profiles: HashMap::new(),
        }
    }

    /// Get the current call count
    pub fn call_count(&self) -> u64 {
        self.call_count.load(Ordering::Relaxed)
    }

    /// Increment the call count and return the new value
    pub fn record_call(&self) -> u64 {
        self.call_count.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Get the current loop iteration count
    pub fn loop_iterations(&self) -> u64 {
        self.loop_iterations.load(Ordering::Relaxed)
    }

    /// Record loop iterations
    pub fn record_loop_iterations(&self, count: u64) {
        self.loop_iterations.fetch_add(count, Ordering::Relaxed);
    }

    /// Get the current compilation tier
    pub fn tier(&self) -> CompilationTier {
        match self.tier.load(Ordering::Relaxed) {
            0 => CompilationTier::Interpreted,
            1 => CompilationTier::BaselineCompiled,
            2 => CompilationTier::Optimized,
            _ => CompilationTier::Interpreted,
        }
    }

    /// Set the compilation tier
    pub fn set_tier(&self, tier: CompilationTier) {
        let tier_value = match tier {
            CompilationTier::Interpreted => 0,
            CompilationTier::BaselineCompiled => 1,
            CompilationTier::Optimized => 2,
        };
        self.tier.store(tier_value, Ordering::Relaxed);
    }

    /// Get or create a branch profile for the given block ID
    pub fn get_branch_profile(&mut self, block_id: u32) -> &BranchProfile {
        self.branch_profiles
            .entry(block_id)
            .or_insert_with(BranchProfile::new)
    }

    /// Reset all profiling data (useful for deoptimization)
    pub fn reset(&self) {
        self.call_count.store(0, Ordering::Relaxed);
        self.loop_iterations.store(0, Ordering::Relaxed);
    }
}

impl Default for FunctionProfile {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl Clone for FunctionProfile {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            call_count: AtomicU64::new(self.call_count.load(Ordering::Relaxed)),
            loop_iterations: AtomicU64::new(self.loop_iterations.load(Ordering::Relaxed)),
            tier: std::sync::atomic::AtomicU8::new(self.tier.load(Ordering::Relaxed)),
            arg_types: self.arg_types.clone(),
            branch_profiles: self.branch_profiles.clone(),
        }
    }
}

/// Tier-up decision result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TierUpDecision {
    /// Stay at current tier
    Stay,

    /// Tier up to the specified tier
    TierUp(CompilationTier),

    /// Deoptimize to interpreter (e.g., type guard failure)
    Deoptimize,
}

impl TierUpDecision {
    /// Returns true if this decision requires compilation
    pub fn requires_compilation(&self) -> bool {
        matches!(self, TierUpDecision::TierUp(_))
    }
}

/// Global profiling state for all functions
#[derive(Debug, Default)]
pub struct ProfileData {
    /// Function profiles indexed by function name
    functions: HashMap<String, Arc<FunctionProfile>>,

    /// Total number of function calls recorded
    total_calls: AtomicU64,

    /// Number of tier-up events
    tier_up_count: AtomicU64,
}

impl ProfileData {
    /// Create a new empty profile data store
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a profile for the given function
    pub fn get_or_create(&mut self, func_name: &str) -> Arc<FunctionProfile> {
        self.functions
            .entry(func_name.to_string())
            .or_insert_with(|| Arc::new(FunctionProfile::new(func_name.to_string())))
            .clone()
    }

    /// Get a profile for the given function (if it exists)
    pub fn get(&self, func_name: &str) -> Option<Arc<FunctionProfile>> {
        self.functions.get(func_name).cloned()
    }

    /// Record a function call and return the profile
    pub fn record_call(&mut self, func_name: &str) -> Arc<FunctionProfile> {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        let profile = self.get_or_create(func_name);
        profile.record_call();
        profile
    }

    /// Get the total number of function calls
    pub fn total_calls(&self) -> u64 {
        self.total_calls.load(Ordering::Relaxed)
    }

    /// Get the number of tier-up events
    pub fn tier_up_count(&self) -> u64 {
        self.tier_up_count.load(Ordering::Relaxed)
    }

    /// Record a tier-up event
    pub fn record_tier_up(&self) {
        self.tier_up_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Check if a function should tier up based on thresholds
    pub fn should_tier_up(
        &self,
        func_name: &str,
        baseline_threshold: u64,
        optimizing_threshold: u64,
    ) -> TierUpDecision {
        let Some(profile) = self.get(func_name) else {
            return TierUpDecision::Stay;
        };

        let call_count = profile.call_count();
        let current_tier = profile.tier();

        match current_tier {
            CompilationTier::Interpreted if call_count >= baseline_threshold => {
                TierUpDecision::TierUp(CompilationTier::BaselineCompiled)
            }
            CompilationTier::BaselineCompiled if call_count >= optimizing_threshold => {
                TierUpDecision::TierUp(CompilationTier::Optimized)
            }
            _ => TierUpDecision::Stay,
        }
    }

    /// Get all function names that have been profiled
    pub fn function_names(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }

    /// Get the number of profiled functions
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Clear all profiling data
    pub fn clear(&mut self) {
        self.functions.clear();
        self.total_calls.store(0, Ordering::Relaxed);
        self.tier_up_count.store(0, Ordering::Relaxed);
    }

    /// Get a summary of profiling data for display
    pub fn summary(&self) -> ProfileSummary {
        let mut by_tier = HashMap::new();
        let mut hottest_functions = Vec::new();

        for (name, profile) in &self.functions {
            let tier = profile.tier();
            *by_tier.entry(tier).or_insert(0usize) += 1;

            hottest_functions.push((name.clone(), profile.call_count()));
        }

        // Sort by call count descending
        hottest_functions.sort_by(|a, b| b.1.cmp(&a.1));
        hottest_functions.truncate(10);

        ProfileSummary {
            total_functions: self.functions.len(),
            total_calls: self.total_calls(),
            tier_up_count: self.tier_up_count(),
            by_tier,
            hottest_functions,
        }
    }
}

impl Clone for ProfileData {
    fn clone(&self) -> Self {
        Self {
            functions: self.functions.clone(),
            total_calls: AtomicU64::new(self.total_calls.load(Ordering::Relaxed)),
            tier_up_count: AtomicU64::new(self.tier_up_count.load(Ordering::Relaxed)),
        }
    }
}

/// Summary of profiling data
#[derive(Debug, Clone)]
pub struct ProfileSummary {
    /// Total number of profiled functions
    pub total_functions: usize,

    /// Total number of function calls
    pub total_calls: u64,

    /// Number of tier-up events
    pub tier_up_count: u64,

    /// Function count by tier
    pub by_tier: HashMap<CompilationTier, usize>,

    /// Top 10 hottest functions (name, call count)
    pub hottest_functions: Vec<(String, u64)>,
}

impl std::fmt::Display for ProfileSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "JIT Profile Summary / ملخص التنميط الفوري")?;
        writeln!(f, "========================================")?;
        writeln!(
            f,
            "Total functions / إجمالي الدوال: {}",
            self.total_functions
        )?;
        writeln!(f, "Total calls / إجمالي الاستدعاءات: {}", self.total_calls)?;
        writeln!(f, "Tier-up events / أحداث الترقية: {}", self.tier_up_count)?;
        writeln!(f)?;
        writeln!(f, "Functions by tier / الدوال حسب المستوى:")?;
        for (tier, count) in &self.by_tier {
            writeln!(f, "  {}: {}", tier, count)?;
        }
        writeln!(f)?;
        writeln!(f, "Hottest functions / أكثر الدوال استدعاءً:")?;
        for (name, count) in &self.hottest_functions {
            writeln!(f, "  {} - {} calls", name, count)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compilation_tier_next() {
        assert_eq!(
            CompilationTier::Interpreted.next_tier(),
            Some(CompilationTier::BaselineCompiled)
        );
        assert_eq!(
            CompilationTier::BaselineCompiled.next_tier(),
            Some(CompilationTier::Optimized)
        );
        assert_eq!(CompilationTier::Optimized.next_tier(), None);
    }

    #[test]
    fn test_branch_profile() {
        let profile = BranchProfile::new();

        profile.record_taken();
        profile.record_taken();
        profile.record_not_taken();

        assert_eq!(profile.total(), 3);
        assert!((profile.taken_probability() - 0.666).abs() < 0.01);
        assert!(!profile.is_biased());
    }

    #[test]
    fn test_branch_profile_biased() {
        let profile = BranchProfile::new();

        for _ in 0..95 {
            profile.record_taken();
        }
        for _ in 0..5 {
            profile.record_not_taken();
        }

        assert!(profile.is_biased());
        assert!((profile.taken_probability() - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_observed_type_merge() {
        assert_eq!(
            ObservedType::AlwaysInt.merge(&ObservedType::AlwaysInt),
            ObservedType::AlwaysInt
        );
        assert_eq!(
            ObservedType::AlwaysInt.merge(&ObservedType::AlwaysFloat),
            ObservedType::Mixed
        );
        assert_eq!(
            ObservedType::Unknown.merge(&ObservedType::AlwaysInt),
            ObservedType::AlwaysInt
        );
    }

    #[test]
    fn test_function_profile() {
        let profile = FunctionProfile::new("test_func".to_string());

        assert_eq!(profile.call_count(), 0);
        assert_eq!(profile.tier(), CompilationTier::Interpreted);

        profile.record_call();
        profile.record_call();

        assert_eq!(profile.call_count(), 2);

        profile.set_tier(CompilationTier::BaselineCompiled);
        assert_eq!(profile.tier(), CompilationTier::BaselineCompiled);
    }

    #[test]
    fn test_profile_data() {
        let mut data = ProfileData::new();

        data.record_call("func1");
        data.record_call("func1");
        data.record_call("func2");

        assert_eq!(data.function_count(), 2);
        assert_eq!(data.total_calls(), 3);

        let profile = data.get("func1").unwrap();
        assert_eq!(profile.call_count(), 2);
    }

    #[test]
    fn test_tier_up_decision() {
        let mut data = ProfileData::new();

        // Record 50 calls - below baseline threshold
        for _ in 0..50 {
            data.record_call("func1");
        }

        assert_eq!(
            data.should_tier_up("func1", 100, 10000),
            TierUpDecision::Stay
        );

        // Record 50 more calls - now at threshold
        for _ in 0..50 {
            data.record_call("func1");
        }

        assert_eq!(
            data.should_tier_up("func1", 100, 10000),
            TierUpDecision::TierUp(CompilationTier::BaselineCompiled)
        );
    }

    #[test]
    fn test_profile_summary() {
        let mut data = ProfileData::new();

        for _ in 0..100 {
            data.record_call("hot_func");
        }
        for _ in 0..10 {
            data.record_call("cold_func");
        }

        let summary = data.summary();
        assert_eq!(summary.total_functions, 2);
        assert_eq!(summary.total_calls, 110);
        assert_eq!(summary.hottest_functions[0].0, "hot_func");
    }
}
