//! JIT-Enabled Executor
//!
//! This module provides a JIT-enabled execution engine that combines
//! the interpreter with profiling and JIT compilation.

use std::sync::Arc;

use crate::interpreter::{Interpreter, RuntimeError, RuntimeResult, Value};
use crate::ir::{FuncId, Module};

#[cfg(feature = "jit")]
use super::baseline::BaselineCompiler;
use super::cache::CodeCache;
use super::config::JitConfig;
#[cfg(feature = "jit")]
use super::optimizing::OptimizingCompiler;
use super::profile::{
    CompilationTier, FunctionProfile, ProfileData, ProfileSummary, TierUpDecision,
};

/// Callback for JIT events (for verbose output)
pub type JitEventCallback = Box<dyn Fn(JitEvent) + Send + Sync>;

/// JIT-related events for monitoring and debugging
#[derive(Debug, Clone)]
pub enum JitEvent {
    /// A function was called
    FunctionCalled { name: String, call_count: u64 },

    /// A function is being considered for tier-up
    TierUpCandidate {
        name: String,
        from_tier: CompilationTier,
        to_tier: CompilationTier,
        call_count: u64,
    },

    /// A function was compiled (placeholder for Phase 2+)
    FunctionCompiled {
        name: String,
        tier: CompilationTier,
        #[allow(dead_code)]
        compile_time_ms: u64,
    },

    /// Execution started
    ExecutionStarted,

    /// Execution completed
    ExecutionCompleted {
        #[allow(dead_code)]
        total_calls: u64,
    },
}

impl std::fmt::Display for JitEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JitEvent::FunctionCalled { name, call_count } => {
                write!(f, "[JIT] Function '{}' called ({} times)", name, call_count)
            }
            JitEvent::TierUpCandidate {
                name,
                from_tier,
                to_tier,
                call_count,
            } => {
                write!(
                    f,
                    "[JIT] Function '{}' candidate for tier-up: {} → {} (calls: {})",
                    name, from_tier, to_tier, call_count
                )
            }
            JitEvent::FunctionCompiled {
                name,
                tier,
                compile_time_ms,
            } => {
                write!(
                    f,
                    "[JIT] Function '{}' compiled to {} in {}ms",
                    name, tier, compile_time_ms
                )
            }
            JitEvent::ExecutionStarted => {
                write!(f, "[JIT] Execution started")
            }
            JitEvent::ExecutionCompleted { total_calls } => {
                write!(f, "[JIT] Execution completed ({} total calls)", total_calls)
            }
        }
    }
}

/// JIT-enabled executor that combines interpretation with profiling and compilation
///
/// This executor wraps the interpreter and adds:
/// - Profiling for function call tracking
/// - Tier-up decisions based on call frequency
/// - JIT compilation of hot functions (when feature enabled)
pub struct JitExecutor {
    /// The IR module being executed
    module: Module,

    /// The underlying interpreter
    interpreter: Interpreter,

    /// JIT configuration
    config: JitConfig,

    /// Profiling data
    profile_data: ProfileData,

    /// Event callback for verbose output
    event_callback: Option<JitEventCallback>,

    /// Code cache for compiled functions
    code_cache: CodeCache,

    /// Baseline JIT compiler (when feature enabled)
    #[cfg(feature = "jit")]
    baseline_compiler: Option<BaselineCompiler>,

    /// Optimizing JIT compiler (when feature enabled)
    #[cfg(feature = "jit")]
    optimizing_compiler: Option<OptimizingCompiler>,
}

impl JitExecutor {
    /// Create a new JIT executor with the given module and configuration
    pub fn new(module: Module, config: JitConfig) -> Self {
        let interpreter = Interpreter::new(module.clone());

        // Initialize baseline compiler if JIT feature is enabled
        #[cfg(feature = "jit")]
        let baseline_compiler = if config.enabled {
            BaselineCompiler::new().ok()
        } else {
            None
        };

        // Initialize optimizing compiler if JIT feature is enabled
        #[cfg(feature = "jit")]
        let optimizing_compiler = if config.enabled {
            OptimizingCompiler::new().ok()
        } else {
            None
        };

        Self {
            module,
            interpreter,
            config,
            profile_data: ProfileData::new(),
            event_callback: None,
            code_cache: CodeCache::new(),
            #[cfg(feature = "jit")]
            baseline_compiler,
            #[cfg(feature = "jit")]
            optimizing_compiler,
        }
    }

    /// Create a JIT executor with default configuration
    pub fn with_defaults(module: Module) -> Self {
        Self::new(module, JitConfig::default())
    }

    /// Set verbose mode with a callback for JIT events
    pub fn set_verbose(&mut self, verbose: bool) {
        if verbose {
            self.event_callback = Some(Box::new(|event| {
                eprintln!("{}", event);
            }));
        } else {
            self.event_callback = None;
        }
    }

    /// Set a custom event callback
    pub fn set_event_callback(&mut self, callback: JitEventCallback) {
        self.event_callback = Some(callback);
    }

    /// Get the JIT configuration
    pub fn config(&self) -> &JitConfig {
        &self.config
    }

    /// Get the profiling data
    pub fn profile_data(&self) -> &ProfileData {
        &self.profile_data
    }

    /// Get a summary of profiling data
    pub fn profile_summary(&self) -> ProfileSummary {
        self.profile_data.summary()
    }

    /// Emit a JIT event (if callback is set)
    fn emit_event(&self, event: JitEvent) {
        if let Some(ref callback) = self.event_callback {
            callback(event);
        }
    }

    /// Run the program
    pub fn run(&mut self) -> RuntimeResult<Value> {
        self.emit_event(JitEvent::ExecutionStarted);

        // Validate configuration
        let warnings = self.config.validate();
        for warning in warnings {
            eprintln!("JIT Warning: {}", warning);
        }

        // Run the interpreter (with profiling in the wrapper)
        let result = if self.config.enabled {
            self.run_with_profiling()
        } else {
            // JIT disabled, just run interpreter
            self.interpreter.run()
        };

        self.emit_event(JitEvent::ExecutionCompleted {
            total_calls: self.profile_data.total_calls(),
        });

        result
    }

    /// Run with profiling enabled
    fn run_with_profiling(&mut self) -> RuntimeResult<Value> {
        // For Phase 1, we run the interpreter but track function calls
        // through the profiling infrastructure
        //
        // In future phases, this will:
        // 1. Check if we have compiled code for the function
        // 2. Execute compiled code if available
        // 3. Otherwise interpret and check for tier-up

        // Find main function to record its name
        let main_func_id = self.find_main_function()?;

        // Record the call for profiling
        self.record_function_call(&main_func_id.0);

        // Run through the interpreter (handles init_globals internally)
        self.interpreter.run()
    }

    /// Record a function call for profiling
    pub fn record_function_call(&mut self, func_name: &str) {
        let profile = self.profile_data.record_call(func_name);
        let call_count = profile.call_count();

        if self.config.verbose {
            self.emit_event(JitEvent::FunctionCalled {
                name: func_name.to_string(),
                call_count,
            });
        }

        // Check for tier-up
        if self.config.enabled {
            self.check_tier_up(func_name, &profile);
        }
    }

    /// Check if a function should be tiered up
    fn check_tier_up(&mut self, func_name: &str, profile: &Arc<FunctionProfile>) {
        let decision = self.profile_data.should_tier_up(
            func_name,
            self.config.baseline_threshold,
            self.config.optimizing_threshold,
        );

        match decision {
            TierUpDecision::TierUp(target_tier) => {
                self.emit_event(JitEvent::TierUpCandidate {
                    name: func_name.to_string(),
                    from_tier: profile.tier(),
                    to_tier: target_tier,
                    call_count: profile.call_count(),
                });

                // Record the tier-up event
                self.profile_data.record_tier_up();
                profile.set_tier(target_tier);

                // Compile the function if JIT is available
                #[cfg(feature = "jit")]
                match target_tier {
                    CompilationTier::BaselineCompiled => {
                        self.compile_function_baseline(func_name);
                    }
                    CompilationTier::Optimized => {
                        self.compile_function_optimizing(func_name);
                    }
                    _ => {}
                }
            }
            TierUpDecision::Deoptimize => {
                // Reset to interpreted
                profile.set_tier(CompilationTier::Interpreted);
            }
            TierUpDecision::Stay => {
                // Nothing to do
            }
        }
    }

    /// Compile a function using the baseline compiler
    #[cfg(feature = "jit")]
    fn compile_function_baseline(&mut self, func_name: &str) {
        // Check if already compiled at baseline or higher
        if let Some(compiled) = self.code_cache.get(func_name) {
            if compiled.info.tier >= CompilationTier::BaselineCompiled {
                return;
            }
        }

        // Get the function from the module
        let func_id = FuncId(func_name.to_string());
        let func = match self.module.get_function(&func_id) {
            Some(f) => f,
            None => return,
        };

        // Clone the function for compilation (to avoid borrow issues)
        let func_clone = func.clone();

        // Compile using baseline compiler
        if let Some(ref mut compiler) = self.baseline_compiler {
            match compiler.compile(&self.module, &func_clone) {
                Ok(compiled) => {
                    let compile_time_ms = compiled.info.compile_time_ms;
                    let tier = compiled.info.tier;

                    self.emit_event(JitEvent::FunctionCompiled {
                        name: func_name.to_string(),
                        tier,
                        compile_time_ms,
                    });

                    // Store in cache
                    self.code_cache.insert(compiled);
                }
                Err(e) => {
                    if self.config.verbose {
                        eprintln!(
                            "[JIT] Baseline compilation failed for '{}': {}",
                            func_name, e
                        );
                    }
                }
            }
        }
    }

    /// Compile a function using the optimizing compiler
    #[cfg(feature = "jit")]
    fn compile_function_optimizing(&mut self, func_name: &str) {
        // Get the function from the module
        let func_id = FuncId(func_name.to_string());
        let func = match self.module.get_function(&func_id) {
            Some(f) => f,
            None => return,
        };

        // Clone the function for compilation (to avoid borrow issues)
        let func_clone = func.clone();

        // Get profile for this function (for profile-guided optimization)
        let profile = self.profile_data.get(func_name);

        // Compile using optimizing compiler
        if let Some(ref mut compiler) = self.optimizing_compiler {
            let result = match profile {
                Some(p) => compiler.compile_with_profile(&self.module, &func_clone, Some(&p)),
                None => compiler.compile(&self.module, &func_clone),
            };

            match result {
                Ok(compiled) => {
                    let compile_time_ms = compiled.info.compile_time_ms;
                    let tier = compiled.info.tier;

                    self.emit_event(JitEvent::FunctionCompiled {
                        name: func_name.to_string(),
                        tier,
                        compile_time_ms,
                    });

                    // Store in cache (will replace baseline version)
                    self.code_cache.insert(compiled);
                }
                Err(e) => {
                    if self.config.verbose {
                        eprintln!(
                            "[JIT] Optimizing compilation failed for '{}': {}",
                            func_name, e
                        );
                    }
                }
            }
        }
    }

    /// Check if a function is compiled
    pub fn is_compiled(&self, func_name: &str) -> bool {
        self.code_cache.contains(func_name)
    }

    /// Get code cache statistics
    pub fn cache_stats(&self) -> super::cache::CacheStats {
        self.code_cache.stats()
    }

    /// Find the main function
    fn find_main_function(&self) -> RuntimeResult<FuncId> {
        let main_names = ["__main__", "main", "رئيسي", "رئيسية", "البداية"];

        for name in main_names {
            let func_id = FuncId(name.to_string());
            if self.module.get_function(&func_id).is_some() {
                return Ok(func_id);
            }
        }

        if let Some(func) = self.module.functions.first() {
            return Ok(func.id.clone());
        }

        Err(RuntimeError::undefined_function("main/رئيسي/رئيسية"))
    }

    /// Call a function with profiling
    pub fn call_function(&mut self, func_id: &FuncId, args: Vec<Value>) -> RuntimeResult<Value> {
        // Record the call
        self.record_function_call(&func_id.0);

        // Delegate to interpreter
        self.interpreter.call_function(func_id, args)
    }

    /// Get a reference to the underlying module
    pub fn module(&self) -> &Module {
        &self.module
    }

    /// Enable output capture on the underlying interpreter
    pub fn capture_output(&mut self, capture: bool) {
        self.interpreter.capture_output(capture);
    }

    /// Get captured output from the interpreter
    pub fn get_output(&self) -> &[String] {
        self.interpreter.get_output()
    }

    /// Reset profiling data
    pub fn reset_profile(&mut self) {
        self.profile_data.clear();
    }
}

// Factory functions for Phase 2 CLI integration
#[allow(dead_code)]
/// Create a JitExecutor with the given configuration
pub fn create_executor(module: Module, config: JitConfig) -> JitExecutor {
    JitExecutor::new(module, config)
}

#[allow(dead_code)]
/// Create a JitExecutor for development mode
pub fn create_dev_executor(module: Module) -> JitExecutor {
    JitExecutor::new(module, JitConfig::development())
}

#[allow(dead_code)]
/// Create a JitExecutor for production mode
pub fn create_prod_executor(module: Module) -> JitExecutor {
    JitExecutor::new(module, JitConfig::production())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BasicBlock, BlockId, Constant, Function, Instruction, IrType, Module, VarId};

    fn create_test_module() -> Module {
        let mut module = Module::new("test".to_string());

        // Create a simple main function
        let mut main_func = Function::new(
            FuncId("__main__".to_string()),
            "__main__".to_string(),
            vec![],
            IrType::Void,
        );
        let mut block = BasicBlock::new(BlockId(0));
        block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Int(42),
            ty: IrType::Int,
        });
        block.instructions.push(Instruction::Return {
            value: Some(VarId(0)),
        });
        main_func.blocks.push(block);

        module.functions.push(main_func);
        module
    }

    #[test]
    fn test_executor_creation() {
        let module = create_test_module();
        let executor = JitExecutor::with_defaults(module);

        assert!(executor.config().enabled);
    }

    #[test]
    fn test_executor_disabled_jit() {
        let module = create_test_module();
        let config = JitConfig::interpreter_only();
        let executor = JitExecutor::new(module, config);

        assert!(!executor.config().enabled);
    }

    #[test]
    fn test_executor_run() {
        let module = create_test_module();
        let mut executor = JitExecutor::with_defaults(module);

        let result = executor.run();
        assert!(result.is_ok());
    }

    #[test]
    fn test_profiling_records_calls() {
        let module = create_test_module();
        let mut executor = JitExecutor::with_defaults(module);

        executor.run().unwrap();

        // The main function should have been called at least once
        let profile = executor.profile_data().get("__main__");
        assert!(profile.is_some());
        assert!(profile.unwrap().call_count() >= 1);
    }

    #[test]
    fn test_profile_summary() {
        let module = create_test_module();
        let mut executor = JitExecutor::with_defaults(module);

        executor.run().unwrap();

        let summary = executor.profile_summary();
        assert!(summary.total_functions >= 1);
        assert!(summary.total_calls >= 1);
    }

    #[test]
    fn test_tier_up_detection() {
        let module = create_test_module();
        let config = JitConfig::new()
            .with_baseline_threshold(5)
            .with_optimizing_threshold(20);
        let mut executor = JitExecutor::new(module, config);

        // Simulate calls to reach baseline tier but not optimizing tier
        for _ in 0..10 {
            executor.record_function_call("test_func");
        }

        let profile = executor.profile_data().get("test_func").unwrap();
        assert_eq!(profile.tier(), CompilationTier::BaselineCompiled);
    }

    #[test]
    fn test_tier_up_to_optimized() {
        let module = create_test_module();
        let config = JitConfig::new()
            .with_baseline_threshold(5)
            .with_optimizing_threshold(15);
        let mut executor = JitExecutor::new(module, config);

        // Simulate many calls
        for _ in 0..20 {
            executor.record_function_call("test_func");
        }

        let profile = executor.profile_data().get("test_func").unwrap();
        assert_eq!(profile.tier(), CompilationTier::Optimized);
    }

    #[test]
    fn test_reset_profile() {
        let module = create_test_module();
        let mut executor = JitExecutor::with_defaults(module);

        executor.run().unwrap();
        assert!(executor.profile_data().function_count() > 0);

        executor.reset_profile();
        assert_eq!(executor.profile_data().function_count(), 0);
    }

    #[test]
    fn test_event_callback() {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::Arc;

        let module = create_test_module();
        let mut executor = JitExecutor::with_defaults(module);

        let event_count = Arc::new(AtomicU64::new(0));
        let event_count_clone = event_count.clone();

        executor.set_event_callback(Box::new(move |_event| {
            event_count_clone.fetch_add(1, Ordering::Relaxed);
        }));

        executor.run().unwrap();

        // Should have at least ExecutionStarted and ExecutionCompleted
        assert!(event_count.load(Ordering::Relaxed) >= 2);
    }
}
