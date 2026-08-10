//! Semantic and execution tests for arrow lambdas (issue #180).
//!
//! Arrow lambdas (`(س: عدد) => س * س`) are a flagship, README-advertised
//! feature that type-checks successfully today but fails in all three
//! execution modes (interpreter, JIT, native) — the root cause is that the
//! IR has no function *value* representation (`Constant` has no `Function`
//! variant), so `build_lambda` emits `Constant::Null` with a "will be
//! replaced with function pointer" comment that nothing ever fulfills.
//!
//! Like `tests/oop_execution_tests.rs`, these tests actually run the
//! program (interpreter and JIT) and assert on printed output, rather than
//! only asserting `analyze_diagnostics` succeeds — that gap is exactly how
//! #180 went unnoticed. This file mirrors `oop_execution_tests.rs`'s
//! file-local helper pattern verbatim (this repo has no shared test-support
//! module).
//!
//! `mod تحليل` covers the semantic layer (contextual param inference,
//! block-body return-type inference, capture detection); `mod تنفيذ`
//! executes programs end-to-end through the interpreter and JIT paths.

#![allow(dead_code)]

use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn wrap_with_markers(source: &str) -> String {
    format!("بسم_الله\n{}\nالحمد_لله", source.trim())
}

/// Runs `source` through the full pipeline (parse -> analyze -> build IR ->
/// interpret) and returns everything the program printed via `اطبع`.
fn interpret_stdout(source: &str) -> Result<Vec<String>, String> {
    use tarqeem::interpreter::Interpreter;
    use tarqeem::ir::IrBuilder;
    use tarqeem::parser::Parser;
    use tarqeem::semantic::Analyzer;

    let wrapped_source = wrap_with_markers(source);

    let mut parser = Parser::new(&wrapped_source);
    let ast = parser.parse().map_err(|e| e.message)?;

    let mut analyzer = Analyzer::new();
    let stdlib_path = project_root().join("stdlib_trq");
    if stdlib_path.exists() {
        analyzer.add_search_path(stdlib_path);
    }

    if let Err(diagnostics) = analyzer.analyze(&ast) {
        return Err(diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
            .join("\n"));
    }

    let ir_builder = IrBuilder::new("test".to_string());
    let ir_module = ir_builder.build(&ast).map_err(|e| e.message.to_string())?;

    let mut interpreter = Interpreter::new(ir_module);
    interpreter.capture_output(true);
    interpreter.run().map_err(|e| format!("{:?}", e))?;
    Ok(interpreter.get_output().to_vec())
}

/// Same pipeline, executed through `JitExecutor` instead of a bare
/// `Interpreter`. This guards against the `JitExecutor` API wrapper itself
/// diverging from the plain interpreter for these programs — it does
/// **not** prove Cranelift-compiled code is correct: with
/// `JitConfig::default()` (`baseline_threshold: 100`) none of these short
/// test bodies call a function enough times to promote past Tier-0
/// interpretation, so both `interpret_stdout` and `jit_stdout` currently run
/// the identical code path under the hood.
fn jit_stdout(source: &str) -> Result<Vec<String>, String> {
    use tarqeem::ir::IrBuilder;
    use tarqeem::jit::{JitConfig, JitExecutor};
    use tarqeem::parser::Parser;
    use tarqeem::semantic::Analyzer;

    let wrapped_source = wrap_with_markers(source);

    let mut parser = Parser::new(&wrapped_source);
    let ast = parser.parse().map_err(|e| e.message)?;

    let mut analyzer = Analyzer::new();
    let stdlib_path = project_root().join("stdlib_trq");
    if stdlib_path.exists() {
        analyzer.add_search_path(stdlib_path);
    }

    if let Err(diagnostics) = analyzer.analyze(&ast) {
        return Err(diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
            .join("\n"));
    }

    let ir_builder = IrBuilder::new("test".to_string());
    let ir_module = ir_builder.build(&ast).map_err(|e| e.message.to_string())?;

    let mut jit = JitExecutor::new(ir_module, JitConfig::default());
    jit.capture_output(true);
    jit.run().map_err(|e| format!("{:?}", e))?;
    Ok(jit.get_output().to_vec())
}

/// Asserts a program prints the same lines under both the interpreter and
/// the JIT executor, guarding against the mode-divergence class of bug
/// (issue #185) as a side effect of proving the fix itself.
fn assert_stdout_both_modes(source: &str, expected: &[&str]) {
    let interp = interpret_stdout(source).unwrap_or_else(|e| panic!("المفسّر فشل: {}", e));
    assert_eq!(interp, expected, "خرج المفسّر غير متطابق");

    let jit = jit_stdout(source).unwrap_or_else(|e| panic!("الترجمة الفورية فشلت: {}", e));
    assert_eq!(
        jit, expected,
        "اختلاف بين المفسّر والترجمة الفورية (راجع #185)"
    );
}

/// Runs `source` through parsing + semantic analysis only (no IR/execution)
/// and returns the diagnostics' error codes and messages on failure, for
/// asserting that a specific error code fires.
fn analyze_diagnostics(source: &str) -> Result<(), Vec<(Option<String>, String)>> {
    use tarqeem::parser::Parser;
    use tarqeem::semantic::Analyzer;

    let wrapped_source = wrap_with_markers(source);
    let mut parser = Parser::new(&wrapped_source);
    let ast = parser
        .parse()
        .map_err(|e| vec![(None, e.message.clone())])?;

    let mut analyzer = Analyzer::new();
    let stdlib_path = project_root().join("stdlib_trq");
    if stdlib_path.exists() {
        analyzer.add_search_path(stdlib_path);
    }

    analyzer.analyze(&ast).map(|_| ()).map_err(|diags| {
        diags
            .iter()
            .map(|d| (d.code.clone(), d.message.clone()))
            .collect()
    })
}

fn assert_analyze_error_code(source: &str, code: &str) {
    match analyze_diagnostics(source) {
        Ok(()) => panic!("توقعنا خطأ '{}' لكن التحليل نجح", code),
        Err(diags) => assert!(
            diags.iter().any(|(c, _)| c.as_deref() == Some(code)),
            "لم يظهر الرمز '{}' في: {:?}",
            code,
            diags
        ),
    }
}

mod تحليل {
    use super::*;

    #[test]
    fn test_untyped_lambda_params_type_check() {
        // LANGUAGE_SPEC §8.3 exact syntax: untyped lambda params become
        // Type::Any, and binary_result_type carries arithmetic arms
        // for Any, so `أ + ب` fails to type-check.
        let source = r#"
            ثابت جمع = (أ، ب) => أ + ب؛
        "#;
        assert!(
            analyze_diagnostics(source).is_ok(),
            "توقعنا نجاح التحليل: {:?}",
            analyze_diagnostics(source).err()
        );
    }

    #[test]
    fn test_annotated_lambda_type_checks() {
        // The `(عدد، عدد) -> عدد` function-type annotation (spec §5.3).
        let source = r#"
            ثابت جمع: (عدد، عدد) -> عدد = (أ، ب) => أ + ب؛
        "#;
        assert!(
            analyze_diagnostics(source).is_ok(),
            "توقعنا نجاح التحليل: {:?}",
            analyze_diagnostics(source).err()
        );
    }

    #[test]
    fn test_annotated_lambda_params_are_int_not_any() {
        // Negative twin of test_annotated_lambda_type_checks: proves the
        // lambda's params really become Int, not silently Any (which would
        // make this pass vacuously): assigning جمع(1، 2): عدد to a نص
        // must surface a real type mismatch.
        let source = r#"
            ثابت جمع: (عدد، عدد) -> عدد = (أ، ب) => أ + ب؛
            ثابت س: نص = جمع(١، ٢)؛
        "#;
        assert!(
            analyze_diagnostics(source).is_err(),
            "توقعنا خطأ عدم تطابق الأنواع (جمع تُرجع عدد، لا نص)"
        );
    }

    #[test]
    fn test_lambda_argument_infers_param_type_from_callee_signature() {
        // Call-argument contextual inference: the lambda argument's
        // param types come from طبق's declared `ج: (عدد) -> عدد`.
        let source = r#"
            دالة طبق(ج: (عدد) -> عدد، ق: عدد) -> عدد {
                أرجع ج(ق)؛
            }
            ثابت ن: عدد = طبق((س) => س * ٢، ٥)؛
        "#;
        assert!(
            analyze_diagnostics(source).is_ok(),
            "توقعنا نجاح التحليل: {:?}",
            analyze_diagnostics(source).err()
        );
    }

    #[test]
    fn test_block_body_lambda_return_type_is_inferred_not_void() {
        // infer_lambda_expr must infer a block body's return type from
        // its أرجع statements (not hardcode Void), or assigning the
        // result to a عدد-typed variable is a spurious type mismatch.
        let source = r#"
            ثابت م = (س: عدد) => {
                أرجع س * 2؛
            }؛
            ثابت ن: عدد = م(3)؛
        "#;
        assert!(
            analyze_diagnostics(source).is_ok(),
            "توقعنا نجاح التحليل: {:?}",
            analyze_diagnostics(source).err()
        );
    }

    #[test]
    fn test_bare_return_inside_lambda_block_is_not_return_outside_function() {
        // Regression guard: is_in_function/get_function_return_type must
        // accept ScopeKind::Lambda, or a bare `أرجع؛` inside a lambda's
        // block body spuriously raises د٠٣٠٣ (return outside function).
        let source = r#"
            ثابت ف = (س: عدد) => {
                أرجع؛
            }؛
        "#;
        assert!(
            analyze_diagnostics(source).is_ok(),
            "لا يجب أن يظهر خطأ 'أرجع خارج دالة' (د٠٣٠٣) هنا: {:?}",
            analyze_diagnostics(source).err()
        );
    }

    #[test]
    fn test_lambda_capturing_outer_local_is_rejected() {
        // Non-capturing lambdas only (closures are #217): referencing an
        // outer local raises د٠٣٠٦ instead of a confusing IR error.
        let source = r#"
            دالة رئيسية() {
                متغير ع = 10؛
                ثابت أضف = (س: عدد) => س + ع؛
                اطبع(أضف(5))؛
            }
        "#;
        assert_analyze_error_code(source, "د٠٣٠٦");
    }

    #[test]
    fn test_this_inside_lambda_in_method_is_rejected() {
        // A lambda is lifted into a standalone function with no receiver,
        // so `هذا` inside one is a capture in disguise: it must be rejected
        // at the semantic stage (د٠٣٠٦) instead of dying later inside the
        // IR builder with a span-less internal error.
        let source = r#"
            صنف عداد {
                عام قيمة: عدد

                منشئ() {
                    هذا.قيمة = 10؛
                }

                عام دالة اصنع() {
                    ثابت ف = (س: عدد) => هذا.قيمة + س؛
                    اطبع(ف(5))؛
                }
            }
        "#;
        assert_analyze_error_code(source, "د٠٣٠٦");
    }

    #[test]
    fn test_break_inside_lambda_is_rejected_as_outside_loop() {
        // `أوقف` can only break a loop in the same function frame; a lambda
        // body is its own frame, so a break inside a lambda defined within
        // a loop must get the ordinary د٠٣٠١ diagnostic at the semantic
        // stage instead of a span-less IrError at build time.
        let source = r#"
            متغير ع = 0؛
            طالما (ع < 3) {
                ثابت ف = () => {
                    أوقف؛
                }؛
                ع++؛
            }
        "#;
        assert_analyze_error_code(source, "د٠٣٠١");
    }

    #[test]
    fn test_lambda_referencing_global_is_not_a_capture() {
        let source = r#"
            ثابت ثابت_عام = 10؛
            دالة رئيسية() {
                ثابت ف = (س: عدد) => س + ثابت_عام؛
                اطبع(ف(5))؛
            }
        "#;
        assert!(
            analyze_diagnostics(source).is_ok(),
            "توقعنا نجاح التحليل: {:?}",
            analyze_diagnostics(source).err()
        );
    }

    #[test]
    fn test_lambda_calling_declared_function_is_not_a_capture() {
        let source = r#"
            دالة ضاعف(س: عدد) -> عدد {
                أرجع س * 2؛
            }
            دالة رئيسية() {
                ثابت ف = (س: عدد) => ضاعف(س)؛
                اطبع(ف(5))؛
            }
        "#;
        assert!(
            analyze_diagnostics(source).is_ok(),
            "توقعنا نجاح التحليل: {:?}",
            analyze_diagnostics(source).err()
        );
    }

    #[test]
    fn test_lambda_own_local_variable_is_not_a_capture() {
        let source = r#"
            ثابت ف = (س: عدد) => {
                متغير ن = س * 2؛
                أرجع ن؛
            }؛
            اطبع(ف(5))؛
        "#;
        assert!(
            analyze_diagnostics(source).is_ok(),
            "توقعنا نجاح التحليل: {:?}",
            analyze_diagnostics(source).err()
        );
    }

    #[test]
    fn test_named_function_assigned_to_function_typed_variable() {
        // A named function is a first-class value assignable to a
        // function-typed variable.
        let source = r#"
            دالة مربع(س: عدد) -> عدد {
                أرجع س * س؛
            }
            ثابت ف: (عدد) -> عدد = مربع؛
        "#;
        assert!(
            analyze_diagnostics(source).is_ok(),
            "توقعنا نجاح التحليل: {:?}",
            analyze_diagnostics(source).err()
        );
    }
}

mod تنفيذ {
    use super::*;

    #[test]
    fn test_readme_square_lambda_executes() {
        let source = r#"
            ثابت مربع = (س: عدد) => س * س؛
            اطبع(مربع(5))؛
        "#;
        assert_stdout_both_modes(source, &["25"]);
    }

    #[test]
    fn test_spec_untyped_sum_lambda_executes() {
        // Digit-rendering style confirmed
        // against tests/oop_execution_tests.rs, which asserts printed
        // integers as ASCII digits (e.g. اطبع(م.مساحة()) -> "25"), not
        // Arabic-Indic — matched here accordingly.
        let source = r#"
            ثابت جمع = (أ، ب) => أ + ب؛
            اطبع(جمع(٣، ٤))؛
        "#;
        assert_stdout_both_modes(source, &["7"]);
    }

    #[test]
    fn test_block_body_lambda_executes() {
        let source = r#"
            ثابت م = (س: عدد) => {
                أرجع س * 2؛
            }؛
            اطبع(م(4))؛
        "#;
        assert_stdout_both_modes(source, &["8"]);
    }

    #[test]
    fn test_lambda_as_higher_order_function_argument_executes() {
        let source = r#"
            دالة طبق(ج: (عدد) -> عدد، ق: عدد) -> عدد {
                أرجع ج(ق)؛
            }
            دالة رئيسية() {
                اطبع(طبق((س) => س * 2، 5))؛
            }
        "#;
        assert_stdout_both_modes(source, &["10"]);
    }

    #[test]
    fn test_named_function_as_value_called_indirectly() {
        let source = r#"
            دالة مربع(س: عدد) -> عدد {
                أرجع س * س؛
            }
            ثابت ف = مربع؛
            اطبع(ف(5))؛
        "#;
        assert_stdout_both_modes(source, &["25"]);
    }

    #[test]
    fn test_two_lambdas_in_different_functions_do_not_clobber_each_other() {
        // Regression test for the
        // __lambda_N naming/state-clobber bugs found during design: without
        // a module-scoped lambda_counter and full builder-state save/
        // restore (current_block, var_counter, block_counter, parameters,
        // var_types, loop_stack), أ()'s and ب()'s lambdas could collide on
        // the same lifted function name or corrupt each other's state.
        let source = r#"
            دالة أ() -> عدد {
                ثابت ف = (س: عدد) => س + 1؛
                أرجع ف(1)؛
            }
            دالة ب() -> عدد {
                ثابت ف = (س: عدد) => س + 100؛
                أرجع ف(1)؛
            }
            دالة رئيسية() {
                اطبع(أ())؛
                اطبع(ب())؛
            }
        "#;
        assert_stdout_both_modes(source, &["2", "101"]);
    }

    #[test]
    fn test_function_type_annotated_lambda_executes() {
        // End-to-end annotated lambda (also exercises the parser
        // support for the `(عدد، عدد) -> عدد` annotation syntax).
        let source = r#"
            ثابت جمع: (عدد، عدد) -> عدد = (أ، ب) => أ + ب؛
            اطبع(جمع(3، 4))؛
        "#;
        assert_stdout_both_modes(source, &["7"]);
    }
}
