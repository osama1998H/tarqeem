//! JIT Performance Benchmarking and Tuning
//!
//! Provides utilities for measuring and tuning JIT compilation performance.
//!
//! أدوات قياس وضبط أداء الترجمة الفورية

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::profile::CompilationTier;

/// Benchmark results for a single function
#[derive(Debug, Clone)]
pub struct FunctionBenchmark {
    /// Function name
    pub name: String,

    /// Compilation tier
    pub tier: CompilationTier,

    /// Time to compile (if applicable)
    pub compile_time: Option<Duration>,

    /// Execution times for each run
    pub execution_times: Vec<Duration>,

    /// Average execution time
    pub avg_execution_time: Duration,

    /// Minimum execution time
    pub min_execution_time: Duration,

    /// Maximum execution time
    pub max_execution_time: Duration,

    /// Standard deviation of execution times
    pub std_dev: Duration,
}

impl FunctionBenchmark {
    /// Create a new benchmark result
    pub fn new(name: String, tier: CompilationTier, execution_times: Vec<Duration>) -> Self {
        let n = execution_times.len();
        let total: Duration = execution_times.iter().sum();
        let avg = if n > 0 {
            total / n as u32
        } else {
            Duration::ZERO
        };
        let min = execution_times
            .iter()
            .min()
            .copied()
            .unwrap_or(Duration::ZERO);
        let max = execution_times
            .iter()
            .max()
            .copied()
            .unwrap_or(Duration::ZERO);

        // Calculate standard deviation
        let variance = if n > 1 {
            let avg_nanos = avg.as_nanos() as f64;
            let sum_sq: f64 = execution_times
                .iter()
                .map(|t| {
                    let diff = t.as_nanos() as f64 - avg_nanos;
                    diff * diff
                })
                .sum();
            sum_sq / (n - 1) as f64
        } else {
            0.0
        };
        let std_dev = Duration::from_nanos(variance.sqrt() as u64);

        Self {
            name,
            tier,
            compile_time: None,
            execution_times,
            avg_execution_time: avg,
            min_execution_time: min,
            max_execution_time: max,
            std_dev,
        }
    }

    /// Set compile time
    pub fn with_compile_time(mut self, compile_time: Duration) -> Self {
        self.compile_time = Some(compile_time);
        self
    }

    /// Calculate speedup compared to another benchmark
    pub fn speedup_vs(&self, other: &FunctionBenchmark) -> f64 {
        if self.avg_execution_time.as_nanos() == 0 {
            return 0.0;
        }
        other.avg_execution_time.as_nanos() as f64 / self.avg_execution_time.as_nanos() as f64
    }
}

impl std::fmt::Display for FunctionBenchmark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Function: {} ({:?})", self.name, self.tier)?;
        if let Some(ct) = self.compile_time {
            writeln!(f, "  Compile time: {:?}", ct)?;
        }
        writeln!(f, "  Runs: {}", self.execution_times.len())?;
        writeln!(f, "  Avg: {:?}", self.avg_execution_time)?;
        writeln!(f, "  Min: {:?}", self.min_execution_time)?;
        writeln!(f, "  Max: {:?}", self.max_execution_time)?;
        writeln!(f, "  Std Dev: {:?}", self.std_dev)?;
        Ok(())
    }
}

/// JIT benchmark suite
#[derive(Debug, Default)]
pub struct JitBenchmark {
    /// Results for each function at each tier
    results: HashMap<String, Vec<FunctionBenchmark>>,

    /// Configuration
    warmup_runs: usize,
    measurement_runs: usize,
}

impl JitBenchmark {
    /// Create a new benchmark suite
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
            warmup_runs: 3,
            measurement_runs: 10,
        }
    }

    /// Set warmup runs
    pub fn with_warmup(mut self, runs: usize) -> Self {
        self.warmup_runs = runs;
        self
    }

    /// Set measurement runs
    pub fn with_runs(mut self, runs: usize) -> Self {
        self.measurement_runs = runs;
        self
    }

    /// Record a benchmark result
    pub fn record(&mut self, benchmark: FunctionBenchmark) {
        self.results
            .entry(benchmark.name.clone())
            .or_default()
            .push(benchmark);
    }

    /// Get results for a function
    pub fn get_results(&self, func_name: &str) -> Option<&Vec<FunctionBenchmark>> {
        self.results.get(func_name)
    }

    /// Get all results
    pub fn all_results(&self) -> &HashMap<String, Vec<FunctionBenchmark>> {
        &self.results
    }

    /// Generate a summary report
    pub fn summary(&self) -> BenchmarkSummary {
        let mut summary = BenchmarkSummary::default();

        for (name, benchmarks) in &self.results {
            // Find interpreter and optimized benchmarks
            let interp = benchmarks
                .iter()
                .find(|b| b.tier == CompilationTier::Interpreted);
            let baseline = benchmarks
                .iter()
                .find(|b| b.tier == CompilationTier::BaselineCompiled);
            let optimized = benchmarks
                .iter()
                .find(|b| b.tier == CompilationTier::Optimized);

            if let (Some(interp), Some(opt)) = (interp, optimized) {
                let speedup = opt.speedup_vs(interp);
                summary.functions.push(FunctionSpeedup {
                    name: name.clone(),
                    interpreter_time: interp.avg_execution_time,
                    baseline_time: baseline.map(|b| b.avg_execution_time),
                    optimized_time: opt.avg_execution_time,
                    speedup,
                });
                summary.total_speedup += speedup;
            }
        }

        if !summary.functions.is_empty() {
            summary.total_speedup /= summary.functions.len() as f64;
        }

        summary
    }

    /// Clear all results
    pub fn clear(&mut self) {
        self.results.clear();
    }
}

/// Summary of benchmark results
#[derive(Debug, Clone, Default)]
pub struct BenchmarkSummary {
    /// Per-function speedup data
    pub functions: Vec<FunctionSpeedup>,

    /// Average speedup across all functions
    pub total_speedup: f64,
}

/// Speedup data for a single function
#[derive(Debug, Clone)]
pub struct FunctionSpeedup {
    /// Function name
    pub name: String,

    /// Interpreter execution time
    pub interpreter_time: Duration,

    /// Baseline JIT execution time (optional)
    pub baseline_time: Option<Duration>,

    /// Optimized JIT execution time
    pub optimized_time: Duration,

    /// Speedup factor (interpreter_time / optimized_time)
    pub speedup: f64,
}

impl std::fmt::Display for BenchmarkSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "JIT Benchmark Summary / ملخص قياس أداء الترجمة الفورية")?;
        writeln!(f, "=======================================================")?;
        writeln!(f)?;

        for func in &self.functions {
            writeln!(f, "Function / الدالة: {}", func.name)?;
            writeln!(f, "  Interpreter: {:?}", func.interpreter_time)?;
            if let Some(bt) = func.baseline_time {
                writeln!(f, "  Baseline JIT: {:?}", bt)?;
            }
            writeln!(f, "  Optimized JIT: {:?}", func.optimized_time)?;
            writeln!(f, "  Speedup: {:.2}x", func.speedup)?;
            writeln!(f)?;
        }

        writeln!(
            f,
            "Average Speedup / متوسط التسريع: {:.2}x",
            self.total_speedup
        )?;
        Ok(())
    }
}

/// Timer utility for benchmarking
#[derive(Debug)]
pub struct Timer {
    start: Instant,
    name: String,
}

impl Timer {
    /// Start a new timer
    pub fn start(name: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            name: name.into(),
        }
    }

    /// Get elapsed time without stopping
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Stop and return elapsed time
    pub fn stop(self) -> Duration {
        self.start.elapsed()
    }

    /// Stop and print elapsed time
    pub fn stop_and_print(self) -> Duration {
        let elapsed = self.start.elapsed();
        eprintln!("[Timer] {} took {:?}", self.name, elapsed);
        elapsed
    }
}

/// Macro for timing a block of code
#[macro_export]
macro_rules! time_block {
    ($name:expr, $block:expr) => {{
        let timer = $crate::jit::bench::Timer::start($name);
        let result = $block;
        let elapsed = timer.stop();
        (result, elapsed)
    }};
}

/// Performance hints for JIT compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceHint {
    /// Function is a hot loop - prioritize for optimization
    HotLoop,

    /// Function is called frequently - good candidate for inlining
    FrequentlyCalled,

    /// Function has predictable types - good for specialization
    MonomorphicTypes,

    /// Function has polymorphic types - avoid specialization
    PolymorphicTypes,

    /// Function is recursive - consider tail call optimization
    Recursive,

    /// Function is small - good candidate for inlining
    Small,

    /// Function is large - may not benefit from inlining
    Large,
}

/// Collection of performance hints for a function
#[derive(Debug, Clone, Default)]
pub struct FunctionHints {
    /// Function name
    pub name: String,

    /// Performance hints
    pub hints: Vec<PerformanceHint>,
}

impl FunctionHints {
    /// Create new hints for a function
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            hints: Vec::new(),
        }
    }

    /// Add a hint
    pub fn add_hint(&mut self, hint: PerformanceHint) {
        if !self.hints.contains(&hint) {
            self.hints.push(hint);
        }
    }

    /// Check if a hint is present
    pub fn has_hint(&self, hint: PerformanceHint) -> bool {
        self.hints.contains(&hint)
    }

    /// Check if function should be prioritized for optimization
    pub fn should_prioritize(&self) -> bool {
        self.hints.contains(&PerformanceHint::HotLoop)
            || self.hints.contains(&PerformanceHint::FrequentlyCalled)
    }

    /// Check if function is good candidate for inlining
    pub fn should_inline(&self) -> bool {
        self.hints.contains(&PerformanceHint::Small)
            && self.hints.contains(&PerformanceHint::FrequentlyCalled)
            && !self.hints.contains(&PerformanceHint::Recursive)
    }

    /// Check if function should use type specialization
    pub fn should_specialize(&self) -> bool {
        self.hints.contains(&PerformanceHint::MonomorphicTypes)
            && !self.hints.contains(&PerformanceHint::PolymorphicTypes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_benchmark() {
        let times = vec![
            Duration::from_millis(10),
            Duration::from_millis(12),
            Duration::from_millis(11),
            Duration::from_millis(10),
            Duration::from_millis(13),
        ];

        let bench =
            FunctionBenchmark::new("test_func".to_string(), CompilationTier::Optimized, times);

        assert_eq!(bench.name, "test_func");
        assert_eq!(bench.tier, CompilationTier::Optimized);
        assert_eq!(bench.min_execution_time, Duration::from_millis(10));
        assert_eq!(bench.max_execution_time, Duration::from_millis(13));
    }

    #[test]
    fn test_speedup_calculation() {
        let interp_times = vec![Duration::from_millis(100)];
        let opt_times = vec![Duration::from_millis(10)];

        let interp = FunctionBenchmark::new(
            "test".to_string(),
            CompilationTier::Interpreted,
            interp_times,
        );
        let opt = FunctionBenchmark::new("test".to_string(), CompilationTier::Optimized, opt_times);

        let speedup = opt.speedup_vs(&interp);
        assert!((speedup - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_jit_benchmark_suite() {
        let mut suite = JitBenchmark::new();

        let bench = FunctionBenchmark::new(
            "func1".to_string(),
            CompilationTier::Optimized,
            vec![Duration::from_millis(5)],
        );
        suite.record(bench);

        assert!(suite.get_results("func1").is_some());
        assert!(suite.get_results("nonexistent").is_none());
    }

    #[test]
    fn test_timer() {
        let timer = Timer::start("test");
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.stop();
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[test]
    fn test_function_hints() {
        let mut hints = FunctionHints::new("test");

        hints.add_hint(PerformanceHint::Small);
        hints.add_hint(PerformanceHint::FrequentlyCalled);

        assert!(hints.has_hint(PerformanceHint::Small));
        assert!(hints.should_inline());
        assert!(!hints.should_specialize());
    }

    #[test]
    fn test_prioritization() {
        let mut hints = FunctionHints::new("test");
        assert!(!hints.should_prioritize());

        hints.add_hint(PerformanceHint::HotLoop);
        assert!(hints.should_prioritize());
    }
}
