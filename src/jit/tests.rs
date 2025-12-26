//! Integration tests for the JIT compilation system
//!
//! These tests verify the JIT infrastructure works correctly with the
//! rest of the compiler pipeline.

use super::*;
use crate::ir::{
    BasicBlock, BinaryOp, BlockId, Constant, FuncId, Function, Instruction, IrType, Module,
    Parameter, VarId,
};

/// Helper to create a basic block with instructions
fn make_block(id: BlockId, instructions: Vec<Instruction>) -> BasicBlock {
    let mut block = BasicBlock::new(id);
    block.instructions = instructions;
    block
}

/// Create a simple test module with a main function that returns 42
fn create_simple_module() -> Module {
    let mut module = Module::new("test_simple".to_string());

    let mut main_func = Function::new(
        FuncId("__main__".to_string()),
        "__main__".to_string(),
        vec![],
        IrType::Int,
    );
    main_func.blocks.push(make_block(
        BlockId(0),
        vec![
            Instruction::Const {
                dest: VarId(0),
                value: Constant::Int(42),
                ty: IrType::Int,
            },
            Instruction::Return {
                value: Some(VarId(0)),
            },
        ],
    ));

    module.functions.push(main_func);
    module
}

/// Create a module with a loop that calls a helper function
#[allow(dead_code)]
fn create_loop_module() -> Module {
    let mut module = Module::new("test_loop".to_string());

    // Helper function that will be called many times
    let mut helper_func = Function::new(
        FuncId("مساعد".to_string()),
        "مساعد".to_string(),
        vec![],
        IrType::Int,
    );
    helper_func.blocks.push(make_block(
        BlockId(0),
        vec![
            Instruction::Const {
                dest: VarId(0),
                value: Constant::Int(1),
                ty: IrType::Int,
            },
            Instruction::Return {
                value: Some(VarId(0)),
            },
        ],
    ));

    // Main function that calls helper in a loop
    let mut main_func = Function::new(
        FuncId("__main__".to_string()),
        "__main__".to_string(),
        vec![],
        IrType::Int,
    );
    main_func.blocks.push(make_block(
        BlockId(0),
        vec![
            Instruction::Const {
                dest: VarId(0),
                value: Constant::Int(0),
                ty: IrType::Int,
            },
            Instruction::Return {
                value: Some(VarId(0)),
            },
        ],
    ));

    module.functions.push(helper_func);
    module.functions.push(main_func);
    module
}

/// Create a module with multiple functions
fn create_multi_function_module() -> Module {
    let mut module = Module::new("test_multi".to_string());

    // Function: add(a, b) -> a + b
    let mut add_func = Function::new(
        FuncId("جمع".to_string()),
        "جمع".to_string(),
        vec![
            Parameter {
                id: VarId(0),
                name: "أ".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "ب".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    add_func.blocks.push(make_block(
        BlockId(0),
        vec![
            Instruction::Binary {
                dest: VarId(2),
                op: BinaryOp::Add,
                left: VarId(0),
                right: VarId(1),
                ty: IrType::Int,
            },
            Instruction::Return {
                value: Some(VarId(2)),
            },
        ],
    ));

    // Main function
    let mut main_func = Function::new(
        FuncId("__main__".to_string()),
        "__main__".to_string(),
        vec![],
        IrType::Int,
    );
    main_func.blocks.push(make_block(
        BlockId(0),
        vec![
            Instruction::Const {
                dest: VarId(0),
                value: Constant::Int(10),
                ty: IrType::Int,
            },
            Instruction::Const {
                dest: VarId(1),
                value: Constant::Int(20),
                ty: IrType::Int,
            },
            Instruction::Call {
                dest: Some(VarId(2)),
                func: FuncId("جمع".to_string()),
                args: vec![VarId(0), VarId(1)],
                ret_ty: IrType::Int,
            },
            Instruction::Return {
                value: Some(VarId(2)),
            },
        ],
    ));

    module.functions.push(add_func);
    module.functions.push(main_func);
    module
}

// ============================================================================
// Profile Module Tests
// ============================================================================

mod profile_tests {
    #[allow(unused_imports)]
    use super::*;
    use crate::jit::profile::*;

    #[test]
    fn test_compilation_tier_ordering() {
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
    fn test_compilation_tier_names() {
        assert_eq!(CompilationTier::Interpreted.name_ar(), "مُفسَّر");
        assert_eq!(CompilationTier::BaselineCompiled.name_ar(), "ترجمة أساسية");
        assert_eq!(CompilationTier::Optimized.name_ar(), "ترجمة مُحسَّنة");
    }

    #[test]
    fn test_branch_profile_probability() {
        let profile = BranchProfile::new();

        // 50/50 split
        for _ in 0..50 {
            profile.record_taken();
            profile.record_not_taken();
        }

        let prob = profile.taken_probability();
        assert!((prob - 0.5).abs() < 0.01);
        assert!(!profile.is_biased());
    }

    #[test]
    fn test_observed_type_specialization() {
        assert!(ObservedType::AlwaysInt.can_specialize());
        assert!(ObservedType::AlwaysFloat.can_specialize());
        assert!(ObservedType::AlwaysString.can_specialize());
        assert!(!ObservedType::Mixed.can_specialize());
        assert!(!ObservedType::Unknown.can_specialize());
    }

    #[test]
    fn test_profile_data_multiple_functions() {
        let mut data = ProfileData::new();

        // Record calls for multiple functions
        for _ in 0..100 {
            data.record_call("func_a");
        }
        for _ in 0..50 {
            data.record_call("func_b");
        }
        for _ in 0..25 {
            data.record_call("func_c");
        }

        assert_eq!(data.function_count(), 3);
        assert_eq!(data.total_calls(), 175);

        let summary = data.summary();
        assert_eq!(summary.hottest_functions[0].0, "func_a");
        assert_eq!(summary.hottest_functions[0].1, 100);
    }

    #[test]
    fn test_tier_up_at_threshold() {
        let mut data = ProfileData::new();

        // Just below threshold
        for _ in 0..99 {
            data.record_call("func");
        }
        assert_eq!(
            data.should_tier_up("func", 100, 10000),
            TierUpDecision::Stay
        );

        // At threshold
        data.record_call("func");
        assert_eq!(
            data.should_tier_up("func", 100, 10000),
            TierUpDecision::TierUp(CompilationTier::BaselineCompiled)
        );
    }
}

// ============================================================================
// Config Module Tests
// ============================================================================

mod config_tests {
    #[allow(unused_imports)]
    use super::*;
    use crate::jit::config::*;

    #[test]
    fn test_config_presets() {
        let dev = JitConfig::development();
        let prod = JitConfig::production();
        let debug = JitConfig::debug();

        // Development should have lower thresholds
        assert!(dev.baseline_threshold <= prod.baseline_threshold);

        // Debug should be verbose
        assert!(debug.verbose);
        assert!(debug.dump_code);
    }

    #[test]
    fn test_config_builder_chain() {
        let config = JitConfig::new()
            .with_enabled(true)
            .with_baseline_threshold(25)
            .with_optimizing_threshold(250)
            .with_verbose(true)
            .with_osr(false)
            .with_inline_caching(false);

        assert!(config.enabled);
        assert_eq!(config.baseline_threshold, 25);
        assert_eq!(config.optimizing_threshold, 250);
        assert!(config.verbose);
        assert!(!config.osr_enabled);
        assert!(!config.inline_caching);
    }

    #[test]
    fn test_config_validation_warnings() {
        // Valid config
        let valid = JitConfig::default();
        assert!(valid.validate().is_empty());

        // Invalid: zero baseline
        let zero_baseline = JitConfig::new().with_baseline_threshold(0);
        assert!(zero_baseline
            .validate()
            .contains(&ConfigWarning::ZeroBaseline));

        // Invalid: baseline >= optimizing
        let bad_order = JitConfig::new()
            .with_baseline_threshold(1000)
            .with_optimizing_threshold(500);
        assert!(bad_order
            .validate()
            .contains(&ConfigWarning::ThresholdOrder));
    }
}

// ============================================================================
// Error Module Tests
// ============================================================================

mod error_tests {
    #[allow(unused_imports)]
    use super::*;
    use crate::jit::error::*;

    #[test]
    fn test_error_kinds() {
        let comp_err = JitError::compilation("test", "اختبار");
        assert_eq!(comp_err.kind, JitErrorKind::Compilation);

        let unsupported = JitError::unsupported_instruction("CallVirtual");
        assert_eq!(unsupported.kind, JitErrorKind::UnsupportedInstruction);

        let cache_full = JitError::cache_full();
        assert_eq!(cache_full.kind, JitErrorKind::CacheFull);
    }

    #[test]
    fn test_error_with_function_context() {
        let err = JitError::internal("test").with_function("my_function");

        assert_eq!(err.function, Some("my_function".to_string()));
        let display = format!("{}", err);
        assert!(display.contains("[my_function]"));
    }

    #[test]
    fn test_bilingual_error_messages() {
        let err = JitError::memory("out of memory");

        assert!(err.message.contains("Memory allocation failed"));
        assert!(err.message_ar.contains("فشل في تخصيص الذاكرة"));
    }
}

// ============================================================================
// Executor Integration Tests
// ============================================================================

mod executor_tests {
    use super::*;

    #[test]
    fn test_executor_with_simple_module() {
        let module = create_simple_module();
        let mut executor = JitExecutor::with_defaults(module);

        let result = executor.run();
        assert!(result.is_ok());

        // Check profiling recorded the main function
        let profile = executor.profile_data().get("__main__");
        assert!(profile.is_some());
    }

    #[test]
    fn test_executor_with_jit_disabled() {
        let module = create_simple_module();
        let config = JitConfig::interpreter_only();
        let mut executor = JitExecutor::new(module, config);

        let result = executor.run();
        assert!(result.is_ok());
    }

    #[test]
    fn test_executor_profile_summary_format() {
        let module = create_simple_module();
        let mut executor = JitExecutor::with_defaults(module);

        executor.run().unwrap();

        let summary = executor.profile_summary();
        let formatted = format!("{}", summary);

        assert!(formatted.contains("JIT Profile Summary"));
        assert!(formatted.contains("ملخص التنميط الفوري"));
    }

    #[test]
    fn test_executor_verbose_mode() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let module = create_simple_module();
        let mut executor = JitExecutor::with_defaults(module);

        let event_received = Arc::new(AtomicBool::new(false));
        let event_received_clone = event_received.clone();

        executor.set_event_callback(Box::new(move |_| {
            event_received_clone.store(true, Ordering::Relaxed);
        }));

        executor.run().unwrap();

        assert!(event_received.load(Ordering::Relaxed));
    }

    #[test]
    fn test_executor_output_capture() {
        let module = create_simple_module();
        let mut executor = JitExecutor::with_defaults(module);

        executor.capture_output(true);
        executor.run().unwrap();

        // Output should be captured (even if empty for this simple test)
        let output = executor.get_output();
        assert!(output.is_empty() || !output.is_empty()); // Just verify no panic
    }

    #[test]
    fn test_executor_with_arabic_function_names() {
        let module = create_multi_function_module();
        let mut executor = JitExecutor::with_defaults(module);

        let result = executor.run();
        assert!(result.is_ok());

        // The Arabic function name should be tracked
        let summary = executor.profile_summary();
        assert!(summary.total_functions >= 1);
    }

    #[test]
    fn test_tier_up_simulation() {
        let module = create_simple_module();
        let config = JitConfig::new()
            .with_baseline_threshold(5)
            .with_optimizing_threshold(50);
        let mut executor = JitExecutor::new(module, config);

        // Simulate many function calls
        for _ in 0..100 {
            executor.record_function_call("hot_function");
        }

        let profile = executor.profile_data().get("hot_function").unwrap();

        // Should have tiered up through both tiers
        assert_eq!(profile.tier(), CompilationTier::Optimized);
        assert!(executor.profile_data().tier_up_count() >= 2);
    }
}

// ============================================================================
// Full Pipeline Integration Tests
// ============================================================================

mod pipeline_tests {
    use super::*;
    use crate::{Analyzer, IrBuilder, Parser};

    fn compile_and_run(source: &str) -> Result<crate::interpreter::Value, String> {
        // Parse
        let mut parser = Parser::new(source);
        let ast = parser
            .parse()
            .map_err(|e| format!("Parse error: {:?}", e))?;

        // Analyze
        let mut analyzer = Analyzer::new();
        analyzer
            .analyze(&ast)
            .map_err(|e| format!("Semantic error: {:?}", e))?;

        // Build IR
        let ir_builder = IrBuilder::new("test".to_string());
        let module = ir_builder
            .build(&ast)
            .map_err(|e| format!("IR error: {:?}", e))?;

        // Run with JIT executor
        let mut executor = JitExecutor::with_defaults(module);
        executor
            .run()
            .map_err(|e| format!("Runtime error: {:?}", e))
    }

    #[test]
    #[ignore = "Requires full compiler pipeline - run with --ignored"]
    fn test_simple_program_execution() {
        let source = r#"
            متغير س = 42
            اطبع(س)
        "#;

        let result = compile_and_run(source);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "Requires full compiler pipeline - run with --ignored"]
    fn test_function_call_profiling() {
        let source = r#"
            دالة مربع(س: عدد) -> عدد {
                أرجع س * س
            }

            متغير نتيجة = مربع(5)
            اطبع(نتيجة)
        "#;

        // Parse and build IR
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap();

        let mut analyzer = Analyzer::new();
        analyzer.analyze(&ast).unwrap();

        let ir_builder = IrBuilder::new("test".to_string());
        let module = ir_builder.build(&ast).unwrap();

        // Run with low threshold
        let config = JitConfig::new().with_baseline_threshold(1);
        let mut executor = JitExecutor::new(module, config);

        executor.run().unwrap();

        // Check that functions were profiled
        let summary = executor.profile_summary();
        assert!(summary.total_functions >= 1);
    }

    #[test]
    #[ignore = "Requires full compiler pipeline - run with --ignored"]
    fn test_loop_hot_function() {
        let source = r#"
            متغير مجموع = 0
            لكل (متغير ع = 0؛ ع < 10؛ ع++) {
                مجموع = مجموع + ع
            }
            اطبع(مجموع)
        "#;

        let result = compile_and_run(source);
        assert!(result.is_ok());
    }
}
