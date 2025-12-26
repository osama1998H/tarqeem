//! JIT Configuration
//!
//! This module provides configuration options for the JIT compilation system.

use std::fmt;

/// Configuration for JIT compilation
///
/// Controls thresholds, behavior, and debugging options for the JIT compiler.
#[derive(Debug, Clone)]
pub struct JitConfig {
    /// Enable JIT compilation
    /// When false, always use interpreter
    pub enabled: bool,

    /// Threshold for Tier 0 → Tier 1 transition (baseline compilation)
    /// A function must be called this many times before baseline compilation
    pub baseline_threshold: u64,

    /// Threshold for Tier 1 → Tier 2 transition (optimizing compilation)
    /// A function must be called this many times before full optimization
    pub optimizing_threshold: u64,

    /// Enable background compilation
    /// When true, compilation happens in a separate thread
    pub background_compilation: bool,

    /// Maximum number of functions in the compile queue
    pub compile_queue_size: usize,

    /// Enable inline caching for method calls
    pub inline_caching: bool,

    /// Enable type specialization based on observed types
    pub type_specialization: bool,

    /// Enable On-Stack Replacement for long-running loops
    pub osr_enabled: bool,

    /// Loop iteration threshold for OSR
    pub osr_threshold: u64,

    /// Print verbose JIT compilation information
    pub verbose: bool,

    /// Dump generated native code
    pub dump_code: bool,

    /// Maximum size of compiled code cache in bytes
    pub code_cache_size: usize,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            baseline_threshold: 100,
            optimizing_threshold: 10_000,
            background_compilation: true,
            compile_queue_size: 64,
            inline_caching: true,
            type_specialization: true,
            osr_enabled: true,
            osr_threshold: 10_000,
            verbose: false,
            dump_code: false,
            code_cache_size: 64 * 1024 * 1024, // 64 MB
        }
    }
}

impl JitConfig {
    /// Create a new JIT configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration optimized for development (faster compilation, less optimization)
    pub fn development() -> Self {
        Self {
            enabled: true,
            baseline_threshold: 50,
            optimizing_threshold: 5_000,
            background_compilation: true,
            compile_queue_size: 32,
            inline_caching: true,
            type_specialization: false, // Faster without specialization
            osr_enabled: true,
            osr_threshold: 5_000,
            verbose: false,
            dump_code: false,
            code_cache_size: 32 * 1024 * 1024, // 32 MB
        }
    }

    /// Create a configuration optimized for production (maximum optimization)
    pub fn production() -> Self {
        Self {
            enabled: true,
            baseline_threshold: 100,
            optimizing_threshold: 10_000,
            background_compilation: true,
            compile_queue_size: 128,
            inline_caching: true,
            type_specialization: true,
            osr_enabled: true,
            osr_threshold: 10_000,
            verbose: false,
            dump_code: false,
            code_cache_size: 128 * 1024 * 1024, // 128 MB
        }
    }

    /// Create a configuration for debugging (verbose output)
    pub fn debug() -> Self {
        Self {
            enabled: true,
            baseline_threshold: 10, // Compile quickly to test JIT
            optimizing_threshold: 100,
            background_compilation: false, // Synchronous for predictable behavior
            compile_queue_size: 16,
            inline_caching: true,
            type_specialization: true,
            osr_enabled: true,
            osr_threshold: 100,
            verbose: true,
            dump_code: true,
            code_cache_size: 16 * 1024 * 1024, // 16 MB
        }
    }

    /// Create a configuration that disables JIT (pure interpretation)
    pub fn interpreter_only() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Builder method: set enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Builder method: set baseline threshold
    pub fn with_baseline_threshold(mut self, threshold: u64) -> Self {
        self.baseline_threshold = threshold;
        self
    }

    /// Builder method: set optimizing threshold
    pub fn with_optimizing_threshold(mut self, threshold: u64) -> Self {
        self.optimizing_threshold = threshold;
        self
    }

    /// Builder method: set background compilation
    pub fn with_background_compilation(mut self, enabled: bool) -> Self {
        self.background_compilation = enabled;
        self
    }

    /// Builder method: set verbose mode
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Builder method: set dump code mode
    pub fn with_dump_code(mut self, dump: bool) -> Self {
        self.dump_code = dump;
        self
    }

    /// Builder method: set OSR enabled
    pub fn with_osr(mut self, enabled: bool) -> Self {
        self.osr_enabled = enabled;
        self
    }

    /// Builder method: set inline caching
    pub fn with_inline_caching(mut self, enabled: bool) -> Self {
        self.inline_caching = enabled;
        self
    }

    /// Builder method: set type specialization
    pub fn with_type_specialization(mut self, enabled: bool) -> Self {
        self.type_specialization = enabled;
        self
    }

    /// Validate the configuration and return any warnings
    pub fn validate(&self) -> Vec<ConfigWarning> {
        let mut warnings = Vec::new();

        if self.baseline_threshold == 0 {
            warnings.push(ConfigWarning::ZeroBaseline);
        }

        if self.baseline_threshold >= self.optimizing_threshold {
            warnings.push(ConfigWarning::ThresholdOrder);
        }

        if self.compile_queue_size == 0 && self.background_compilation {
            warnings.push(ConfigWarning::EmptyQueue);
        }

        if self.code_cache_size < 1024 * 1024 {
            warnings.push(ConfigWarning::SmallCache);
        }

        warnings
    }
}

impl fmt::Display for JitConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "JIT Configuration / إعدادات الترجمة الفورية")?;
        writeln!(f, "============================================")?;
        writeln!(
            f,
            "Enabled / مُفعَّل: {}",
            if self.enabled {
                "yes / نعم"
            } else {
                "no / لا"
            }
        )?;
        writeln!(
            f,
            "Baseline threshold / عتبة الترجمة الأساسية: {}",
            self.baseline_threshold
        )?;
        writeln!(
            f,
            "Optimizing threshold / عتبة التحسين: {}",
            self.optimizing_threshold
        )?;
        writeln!(
            f,
            "Background compilation / ترجمة في الخلفية: {}",
            if self.background_compilation {
                "yes / نعم"
            } else {
                "no / لا"
            }
        )?;
        writeln!(
            f,
            "Inline caching / تخزين مؤقت للاستدعاءات: {}",
            if self.inline_caching {
                "yes / نعم"
            } else {
                "no / لا"
            }
        )?;
        writeln!(
            f,
            "Type specialization / تخصص الأنماط: {}",
            if self.type_specialization {
                "yes / نعم"
            } else {
                "no / لا"
            }
        )?;
        writeln!(
            f,
            "OSR enabled / الاستبدال على المكدس: {}",
            if self.osr_enabled {
                "yes / نعم"
            } else {
                "no / لا"
            }
        )?;
        writeln!(
            f,
            "Code cache size / حجم ذاكرة التخزين: {} MB",
            self.code_cache_size / (1024 * 1024)
        )?;
        Ok(())
    }
}

/// Configuration validation warning
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigWarning {
    /// Baseline threshold is zero (will compile everything immediately)
    ZeroBaseline,

    /// Baseline threshold >= optimizing threshold
    ThresholdOrder,

    /// Compile queue size is zero but background compilation is enabled
    EmptyQueue,

    /// Code cache size is very small
    SmallCache,
}

impl fmt::Display for ConfigWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigWarning::ZeroBaseline => write!(
                f,
                "Baseline threshold is 0 - all functions will be compiled immediately / عتبة الترجمة الأساسية صفر - ستُترجم جميع الدوال فوراً"
            ),
            ConfigWarning::ThresholdOrder => write!(
                f,
                "Baseline threshold >= optimizing threshold - tier 2 may never trigger / عتبة الأساسية أكبر من عتبة التحسين - قد لا يعمل المستوى الثاني"
            ),
            ConfigWarning::EmptyQueue => write!(
                f,
                "Compile queue size is 0 but background compilation is enabled / حجم طابور الترجمة صفر لكن الترجمة في الخلفية مُفعَّلة"
            ),
            ConfigWarning::SmallCache => write!(
                f,
                "Code cache size is very small - may cause frequent evictions / حجم ذاكرة التخزين صغير جداً - قد يسبب إخلاء متكرر"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = JitConfig::default();

        assert!(config.enabled);
        assert_eq!(config.baseline_threshold, 100);
        assert_eq!(config.optimizing_threshold, 10_000);
        assert!(config.background_compilation);
        assert!(config.inline_caching);
        assert!(config.type_specialization);
        assert!(!config.verbose);
    }

    #[test]
    fn test_development_config() {
        let config = JitConfig::development();

        assert!(config.enabled);
        assert!(config.baseline_threshold < JitConfig::default().baseline_threshold);
        assert!(!config.type_specialization); // Disabled for speed
    }

    #[test]
    fn test_production_config() {
        let config = JitConfig::production();

        assert!(config.enabled);
        assert!(config.type_specialization);
        assert!(config.code_cache_size > JitConfig::default().code_cache_size);
    }

    #[test]
    fn test_debug_config() {
        let config = JitConfig::debug();

        assert!(config.verbose);
        assert!(config.dump_code);
        assert!(!config.background_compilation); // Synchronous for debugging
    }

    #[test]
    fn test_interpreter_only() {
        let config = JitConfig::interpreter_only();

        assert!(!config.enabled);
    }

    #[test]
    fn test_builder_pattern() {
        let config = JitConfig::new()
            .with_baseline_threshold(50)
            .with_optimizing_threshold(5000)
            .with_verbose(true)
            .with_osr(false);

        assert_eq!(config.baseline_threshold, 50);
        assert_eq!(config.optimizing_threshold, 5000);
        assert!(config.verbose);
        assert!(!config.osr_enabled);
    }

    #[test]
    fn test_validation_zero_baseline() {
        let config = JitConfig::new().with_baseline_threshold(0);
        let warnings = config.validate();

        assert!(warnings.contains(&ConfigWarning::ZeroBaseline));
    }

    #[test]
    fn test_validation_threshold_order() {
        let config = JitConfig::new()
            .with_baseline_threshold(1000)
            .with_optimizing_threshold(100);
        let warnings = config.validate();

        assert!(warnings.contains(&ConfigWarning::ThresholdOrder));
    }

    #[test]
    fn test_validation_valid_config() {
        let config = JitConfig::default();
        let warnings = config.validate();

        assert!(warnings.is_empty());
    }

    #[test]
    fn test_config_display() {
        let config = JitConfig::default();
        let display = format!("{}", config);

        assert!(display.contains("JIT Configuration"));
        assert!(display.contains("Baseline threshold"));
        assert!(display.contains("100"));
    }
}
