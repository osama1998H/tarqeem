//! Execution-based tests for core OOP features (issue #184).
//!
//! Unlike most of `tests/runtime_rs_e2e_tests.rs`, which only asserts that a
//! program parses/type-checks (`analyzes_ok`), these tests actually run the
//! program (interpreter and JIT) and assert on printed output. That gap is
//! exactly why the #184 defects — inherited method calls, `مشترك` static
//! access, and upcasting — went unnoticed through 1,300+ green tests (see
//! issue #187). These tests are red before the corresponding fix and green
//! after.

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
    let stdlib_path = project_root().join("stdlib");
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
/// **not** prove Cranelift-compiled code is correct for method dispatch:
/// with `JitConfig::default()` (`baseline_threshold: 100`) none of these
/// short test bodies call a method enough times to promote past Tier-0
/// interpretation, so both `interpret_stdout` and `jit_stdout` currently
/// run the identical code path under the hood. Cranelift's baseline/
/// optimizing compilers have no `Instruction::CallMethod` arm at all today
/// (tracked as a native/JIT dispatch gap alongside issue #185) — raising
/// the threshold here to force promotion would just trade this coverage
/// gap for spurious failures on a deliberately out-of-scope limitation.
fn jit_stdout(source: &str) -> Result<Vec<String>, String> {
    use tarqeem::ir::IrBuilder;
    use tarqeem::jit::{JitConfig, JitExecutor};
    use tarqeem::parser::Parser;
    use tarqeem::semantic::Analyzer;

    let wrapped_source = wrap_with_markers(source);

    let mut parser = Parser::new(&wrapped_source);
    let ast = parser.parse().map_err(|e| e.message)?;

    let mut analyzer = Analyzer::new();
    let stdlib_path = project_root().join("stdlib");
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
    let stdlib_path = project_root().join("stdlib");
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

mod terminators {
    use super::*;

    #[test]
    fn test_method_ending_in_match_returns_to_its_caller() {
        // The #234 repro. تطابق mints its exit block before the arm blocks, so
        // the merge block the method ends on was never blocks.last() — the
        // implicit-Return check inspected a terminated arm, passed, and left
        // the merge block bare. The interpreter then fell through in block
        // order into match.arm0, whose join jump goes back to the merge block:
        // "1" forever, and the caller never regained control.
        //
        // A regression here hangs rather than fails; the CI job timeout is the
        // backstop.
        let source = r#"
            صنف مثال {
                منشئ() { }
                عام دالة افحص(ق: عدد) {
                    تطابق (ق) {
                        حالة 1 => اطبع(1)
                        غير_ذلك => اطبع(0)
                    }
                }
            }

            دالة رئيسية() {
                متغير م = جديد مثال()
                م.افحص(1)
                اطبع(99)
            }
        "#;
        assert_stdout_both_modes(source, &["1", "99"]);
    }

    #[test]
    fn test_constructor_ending_in_match_returns_to_its_caller() {
        let source = r#"
            صنف مثال {
                منشئ(ق: عدد) {
                    تطابق (ق) {
                        حالة 1 => اطبع(1)
                        غير_ذلك => اطبع(0)
                    }
                }
            }

            دالة رئيسية() {
                متغير م = جديد مثال(1)
                اطبع(99)
            }
        "#;
        assert_stdout_both_modes(source, &["1", "99"]);
    }
}

mod dispatch {
    use super::*;

    #[test]
    fn test_inherited_method_call_dispatches_to_parent() {
        // The literal #184 repro: موظف inherits تحية from شخص without
        // overriding it. Before the fix this aborted with
        // "دالة غير معرّفة: موظف::تحية".
        let source = r#"
            صنف شخص {
                خاص اسم: نص

                منشئ(اسم: نص) {
                    هذا.اسم = اسم
                }

                عام دالة تحية() {
                    اطبع("مرحبا " + هذا.اسم)
                }
            }

            صنف موظف يرث شخص {
                منشئ(اسم: نص) {
                    الأصل(اسم)
                }
            }

            دالة رئيسية() {
                متغير م = جديد موظف("أحمد")
                م.تحية()
            }
        "#;
        assert_stdout_both_modes(source, &["مرحبا أحمد"]);
    }

    #[test]
    fn test_template_method_dispatches_to_override_via_this() {
        // شخص::قدم_نفسك calls هذا.تحية(); موظف overrides تحية. هذا's
        // *static* type inside قدم_نفسك is always شخص, so this only passes
        // if dispatch is genuinely dynamic (resolved from the runtime
        // object), not merely "static with a parent fallback".
        let source = r#"
            صنف شخص {
                منشئ() {}

                عام دالة قدم_نفسك() {
                    هذا.تحية()
                }

                عام دالة تحية() {
                    اطبع("أنا شخص")
                }
            }

            صنف موظف يرث شخص {
                منشئ() { الأصل() }

                عام دالة تحية() {
                    اطبع("أنا موظف")
                }
            }

            دالة رئيسية() {
                متغير م = جديد موظف()
                م.قدم_نفسك()
            }
        "#;
        assert_stdout_both_modes(source, &["أنا موظف"]);
    }

    #[test]
    fn test_super_call_does_not_recurse() {
        // موظف::تحية overrides تحية and calls الأصل.تحية(). If super calls
        // were (incorrectly) dispatched dynamically on the runtime object,
        // this would call straight back into موظف::تحية and recurse until
        // the interpreter's stack-depth guard aborts the program.
        let source = r#"
            صنف شخص {
                منشئ() {}

                عام دالة تحية() {
                    اطبع("أنا شخص")
                }
            }

            صنف موظف يرث شخص {
                منشئ() { الأصل() }

                عام دالة تحية() {
                    الأصل.تحية()
                    اطبع("أنا موظف")
                }
            }

            دالة رئيسية() {
                متغير م = جديد موظف()
                م.تحية()
            }
        "#;
        assert_stdout_both_modes(source, &["أنا شخص", "أنا موظف"]);
    }

    #[test]
    fn test_three_level_inheritance_chain() {
        // ج inherits اجلب from أ through ب, and ب overrides nothing —
        // proves the parent-chain walk isn't hardcoded to a single hop.
        let source = r#"
            صنف أ {
                منشئ() {}
                عام دالة اجلب() -> نص { أرجع "من أ" }
            }
            صنف ب يرث أ {
                منشئ() { الأصل() }
            }
            صنف ج يرث ب {
                منشئ() { الأصل() }
            }
            دالة رئيسية() {
                متغير س = جديد ج()
                اطبع(س.اجلب())
            }
        "#;
        assert_stdout_both_modes(source, &["من أ"]);
    }

    #[test]
    fn test_override_without_upcast_still_works() {
        // Regression guard: this already worked before the fix (no upcast,
        // no inheritance gap) and must keep working unchanged.
        let source = r#"
            صنف شكل {
                منشئ() {}
                عام دالة مساحة() -> عدد { أرجع 0 }
            }
            صنف مربع يرث شكل {
                خاص ض: عدد
                منشئ(ض: عدد) { هذا.ض = ض }
                عام دالة مساحة() -> عدد { أرجع هذا.ض * هذا.ض }
            }
            دالة رئيسية() {
                متغير م = جديد مربع(5)
                اطبع(م.مساحة())
            }
        "#;
        assert_stdout_both_modes(source, &["25"]);
    }
}

mod statics {
    use super::*;

    #[test]
    fn test_static_field_read() {
        let source = r#"
            صنف عدادات {
                مشترك المجموع: عدد = 5
            }
            دالة رئيسية() {
                اطبع(عدادات.المجموع)
            }
        "#;
        assert_stdout_both_modes(source, &["5"]);
    }

    #[test]
    fn test_static_field_write_then_read() {
        let source = r#"
            صنف عدادات {
                مشترك المجموع: عدد = 5
            }
            دالة رئيسية() {
                اطبع(عدادات.المجموع)
                عدادات.المجموع = 7
                اطبع(عدادات.المجموع)
            }
        "#;
        assert_stdout_both_modes(source, &["5", "7"]);
    }

    #[test]
    fn test_static_method_call_with_args_and_return() {
        // The literal #184 repro. The field and the method's result are
        // deliberately different values — if static-method dispatch ever
        // mis-resolved to the field's global instead of actually invoking
        // the function (both #184 fixes routed through the same
        // class_name_receiver machinery), a shared value would let that
        // bug pass unnoticed.
        let source = r#"
            صنف عدادات {
                مشترك المجموع: عدد = 7

                مشترك دالة اجمع(أ: عدد، ب: عدد) -> عدد {
                    أرجع أ + ب
                }
            }
            دالة رئيسية() {
                اطبع(عدادات.المجموع)
                اطبع(عدادات.اجمع(2، 3))
            }
        "#;
        assert_stdout_both_modes(source, &["7", "5"]);
    }

    #[test]
    fn test_static_field_mutated_from_static_method() {
        // The exact shape of the parser fixture that motivated this fix
        // (src/parser/parser_tests.rs::test_parse_class_static_members),
        // now actually executed.
        let source = r#"
            صنف عداد_م {
                مشترك قيمة: عدد = 0

                مشترك دالة زد() {
                    عداد_م.قيمة = عداد_م.قيمة + 1
                }
            }
            دالة رئيسية() {
                عداد_م.زد()
                عداد_م.زد()
                اطبع(عداد_م.قيمة)
            }
        "#;
        assert_stdout_both_modes(source, &["2"]);
    }

    #[test]
    fn test_inherited_static_shares_one_slot() {
        // موظف has no static field of its own: عداد is defined only on
        // شخص. Reading/writing through either name must hit the same slot.
        let source = r#"
            صنف شخص {
                مشترك عداد: عدد = 3
                منشئ() {}
            }
            صنف موظف يرث شخص {
                منشئ() { الأصل() }
            }
            دالة رئيسية() {
                اطبع(موظف.عداد)
                موظف.عداد = 9
                اطبع(شخص.عداد)
            }
        "#;
        assert_stdout_both_modes(source, &["3", "9"]);
    }

    #[test]
    fn test_static_auto_property() {
        let source = r#"
            صنف عدادات {
                مشترك خاصية س: عدد = 4
            }
            دالة رئيسية() {
                اطبع(عدادات.س)
                عدادات.س = 6
                اطبع(عدادات.س)
            }
        "#;
        assert_stdout_both_modes(source, &["4", "6"]);
    }

    #[test]
    fn test_static_non_const_initializer() {
        // Array literals aren't compile-time constants (try_evaluate_const
        // returns None for them), so this exercises the __global_init__
        // path rather than the constant-folded global slot.
        let source = r#"
            صنف عدادات {
                مشترك أسماء: مصفوفة<نص> = ["أ"، "ب"]
            }
            دالة رئيسية() {
                اطبع(طول(عدادات.أسماء))
            }
        "#;
        assert_stdout_both_modes(source, &["2"]);
    }

    #[test]
    fn test_local_variable_shadows_class_name() {
        // A local variable named like a declared class must win over the
        // class-name-as-namespace resolution (class_name_receiver checks
        // shadowing first).
        let source = r#"
            صنف عدادات {
                مشترك المجموع: عدد = 5
            }
            دالة رئيسية() {
                متغير عدادات = 3
                اطبع(عدادات)
            }
        "#;
        assert_stdout_both_modes(source, &["3"]);
    }

    #[test]
    fn test_nonstatic_member_via_class_name_is_rejected() {
        let source = r#"
            صنف صندوق {
                عام ح: عدد
                منشئ() { هذا.ح = 1 }
            }
            دالة رئيسية() {
                اطبع(صندوق.ح)
            }
        "#;
        assert_analyze_error_code(source, "ص٠٥٠١");
    }

    #[test]
    fn test_static_member_via_instance_is_rejected() {
        let source = r#"
            صنف صندوق {
                مشترك ح: عدد = 1
                منشئ() {}
            }
            دالة رئيسية() {
                متغير ن = جديد صندوق()
                اطبع(ن.ح)
            }
        "#;
        assert_analyze_error_code(source, "ص٠٥٠٢");
    }
}

mod upcasting {
    use super::*;

    #[test]
    fn test_upcast_dispatches_to_override() {
        // The literal #184 repro: assigning a مربع to a شكل-typed variable
        // used to be rejected at type-check (ن٠٠٠١). It must now compile
        // AND dispatch dynamically to the override (25), not statically to
        // the base implementation (0) — that's the interaction between the
        // dispatch fix (Part A) and this assignability fix.
        let source = r#"
            صنف شكل {
                منشئ() {}
                عام دالة مساحة() -> عدد { أرجع 0 }
            }
            صنف مربع يرث شكل {
                خاص ض: عدد
                منشئ(ض: عدد) { هذا.ض = ض }
                عام دالة مساحة() -> عدد { أرجع هذا.ض * هذا.ض }
            }
            دالة رئيسية() {
                متغير ش: شكل = جديد مربع(5)
                اطبع(ش.مساحة())
            }
        "#;
        assert_stdout_both_modes(source, &["25"]);
    }

    #[test]
    fn test_upcast_via_function_parameter() {
        let source = r#"
            صنف شكل {
                منشئ() {}
                عام دالة مساحة() -> عدد { أرجع 0 }
            }
            صنف مربع يرث شكل {
                خاص ض: عدد
                منشئ(ض: عدد) { هذا.ض = ض }
                عام دالة مساحة() -> عدد { أرجع هذا.ض * هذا.ض }
            }
            دالة اطبع_مساحة(ش: شكل) {
                اطبع(ش.مساحة())
            }
            دالة رئيسية() {
                اطبع_مساحة(جديد مربع(5))
            }
        "#;
        assert_stdout_both_modes(source, &["25"]);
    }

    #[test]
    fn test_upcast_via_optional_slot() {
        let source = r#"
            صنف شكل {
                منشئ() {}
                عام دالة مساحة() -> عدد { أرجع 0 }
            }
            صنف مربع يرث شكل {
                خاص ض: عدد
                منشئ(ض: عدد) { هذا.ض = ض }
                عام دالة مساحة() -> عدد { أرجع هذا.ض * هذا.ض }
            }
            دالة رئيسية() {
                متغير ش: شكل? = جديد مربع(5)
                إذا (ش != لا_شيء) {
                    اطبع(ش.مساحة())
                }
            }
        "#;
        assert_stdout_both_modes(source, &["25"]);
    }

    #[test]
    fn test_upcast_via_constructor_argument() {
        let source = r#"
            صنف شكل {
                منشئ() {}
                عام دالة مساحة() -> عدد { أرجع 0 }
            }
            صنف مربع يرث شكل {
                خاص ض: عدد
                منشئ(ض: عدد) { هذا.ض = ض }
                عام دالة مساحة() -> عدد { أرجع هذا.ض * هذا.ض }
            }
            صنف حاوية {
                خاص ش: شكل
                منشئ(ش: شكل) { هذا.ش = ش }
                عام دالة اعرض() { اطبع(هذا.ش.مساحة()) }
            }
            دالة رئيسية() {
                متغير ح = جديد حاوية(جديد مربع(5))
                ح.اعرض()
            }
        "#;
        assert_stdout_both_modes(source, &["25"]);
    }

    #[test]
    fn test_override_without_upcast_still_type_checks() {
        // Regression guard for the refactor itself (Type::compat sharing
        // one recursive body between is_compatible_with and is_assignable).
        let source = r#"
            صنف شكل {
                منشئ() {}
                عام دالة مساحة() -> عدد { أرجع 0 }
            }
            صنف مربع يرث شكل {
                خاص ض: عدد
                منشئ(ض: عدد) { هذا.ض = ض }
                عام دالة مساحة() -> عدد { أرجع هذا.ض * هذا.ض }
            }
            دالة رئيسية() {
                متغير م = جديد مربع(5)
                اطبع(م.مساحة())
            }
        "#;
        assert_stdout_both_modes(source, &["25"]);
    }

    #[test]
    fn test_int_to_float_widening_still_allowed() {
        // Locks the pre-existing Int->Float arm through the shared `compat`
        // recursion — must survive the is_compatible_with/is_assignable split.
        let source = r#"
            دالة رئيسية() {
                متغير س: عدد_عشري = 5
                اطبع(س)
            }
        "#;
        let interp = interpret_stdout(source).unwrap_or_else(|e| panic!("المفسّر فشل: {}", e));
        assert_eq!(interp, ["5.0"]);
    }

    #[test]
    fn test_downcast_is_rejected() {
        let source = r#"
            صنف شكل { منشئ() {} }
            صنف مربع يرث شكل { منشئ() { الأصل() } }
            دالة رئيسية() {
                متغير م: مربع = جديد شكل()
            }
        "#;
        assert_analyze_error_code(source, "ن٠٠٠١");
    }

    #[test]
    fn test_unrelated_classes_are_rejected() {
        let source = r#"
            صنف أ { منشئ() {} }
            صنف ب { منشئ() {} }
            دالة رئيسية() {
                متغير س: أ = جديد ب()
            }
        "#;
        assert_analyze_error_code(source, "ن٠٠٠١");
    }

    #[test]
    fn test_interface_slot_is_still_rejected() {
        // Locks the deliberate exclusion documented on the new arm in
        // Type::compat: interface-typed slots are NOT covered by this fix.
        // If a future change adds an implements_interface arm, this test
        // forces a conscious re-check of the prerequisites noted there.
        let source = r#"
            ميثاق قابل_للطباعة {
                دالة اطبع_معلومات()
            }
            صنف مربع يلتزم قابل_للطباعة {
                منشئ() {}
                عام دالة اطبع_معلومات() {
                    اطبع("مربع")
                }
            }
            دالة رئيسية() {
                متغير ط: قابل_للطباعة = جديد مربع()
            }
        "#;
        assert_analyze_error_code(source, "ن٠٠٠١");
    }

    #[test]
    fn test_string_to_int_is_rejected() {
        let source = r#"
            دالة رئيسية() {
                متغير س: عدد = "نص"
            }
        "#;
        assert_analyze_error_code(source, "ن٠٠٠١");
    }

    /// Code-review regression guard: the ternary widening join
    /// (`infer_ternary_expr`) must pick the ancestor type regardless of
    /// which branch is the subclass — a naive `is_assignable(then, else)`
    /// check alone would narrow to whichever branch happens to satisfy it
    /// first.
    #[test]
    fn test_ternary_widens_to_ancestor_regardless_of_branch_order() {
        // The join must resolve to شكل (the ancestor) no matter which side
        // of the ternary the subclass is on. A naive `is_assignable(then,
        // else)` check alone narrows to whichever branch satisfies it
        // first — accepting `مربع` here would then wrongly permit calling a
        // مربع-only member on a value that could actually be a plain شكل
        // at runtime, since the condition isn't known at compile time.
        let subclass_in_else = r#"
            صنف شكل { منشئ() {} }
            صنف مربع يرث شكل {
                منشئ() {}
                عام دالة فقط_مربع() -> عدد { أرجع 1 }
            }
            دالة رئيسية() {
                متغير م = صحيح ? جديد شكل() : جديد مربع()
                اطبع(م.فقط_مربع())
            }
        "#;
        assert_analyze_error_code(subclass_in_else, "ص٠٣٠١");

        let subclass_in_then = r#"
            صنف شكل { منشئ() {} }
            صنف مربع يرث شكل {
                منشئ() {}
                عام دالة فقط_مربع() -> عدد { أرجع 1 }
            }
            دالة رئيسية() {
                متغير م = صحيح ? جديد مربع() : جديد شكل()
                اطبع(م.فقط_مربع())
            }
        "#;
        assert_analyze_error_code(subclass_in_then, "ص٠٣٠١");
    }

    /// Code-review regression guard: a ternary with one Optional branch and
    /// one plain branch must stay Optional either order — the previous
    /// widening logic silently dropped the Optional annotation whenever the
    /// non-Optional branch happened to satisfy `is_assignable` first.
    #[test]
    fn test_ternary_preserves_optional_from_either_branch() {
        let source = r#"
            دالة قد_يفرغ(ب: منطقي) -> نص? {
                إذا (ب) { أرجع "قيمة" }
                أرجع لا_شيء
            }
            دالة رئيسية() {
                متغير م١ = صحيح ? قد_يفرغ(صحيح) : "افتراضي"
                م١ = لا_شيء
                اطبع("تم١")

                متغير م٢ = صحيح ? "افتراضي" : قد_يفرغ(صحيح)
                م٢ = لا_شيء
                اطبع("تم٢")
            }
        "#;
        assert_stdout_both_modes(source, &["تم١", "تم٢"]);
    }

    /// Code-review regression guard: array-literal element-type inference
    /// must not depend on which order related-class elements are listed in.
    #[test]
    fn test_array_literal_element_order_independent() {
        let source = r#"
            صنف شكل {
                منشئ() {}
                عام دالة مساحة() -> عدد { أرجع 0 }
            }
            صنف مربع يرث شكل {
                خاص ض: عدد
                منشئ(ض: عدد) { هذا.ض = ض }
                عام دالة مساحة() -> عدد { أرجع هذا.ض * هذا.ض }
            }
            دالة رئيسية() {
                متغير أ = [جديد مربع(5)، جديد شكل()]
                لكل عنصر في أ {
                    اطبع(عنصر.مساحة())
                }
            }
        "#;
        assert_stdout_both_modes(source, &["25", "0"]);
    }
}

/// `صدّر` on a declaration must change nothing but its visibility to other
/// modules (issue #259).
///
/// The analyzer registered an exported class's *name* but never its members,
/// so the compiler rejected correct code: `جديد` reported
/// `الصنف 'س' ليس له منشئ` and every field/method access reported ص٠٣٠١ —
/// even from inside the class's own method bodies.
///
/// Invisible until now because no test ever used the members of an exported
/// class declared in the **main file**. The fixtures that come closest each
/// miss it: `tests/phase3_criteria_tests.rs::test_export_class` stops at
/// `parses_ok`; `src/ir/builder`'s exported-class fixtures declare `{}` with no
/// members to lose; and `tests/module_execution_tests.rs::
/// test_imported_class_constructs_and_reads_field` exercises the *module* path,
/// which was already correct because `add_module_type_members` unwrapped `صدّر`
/// and `add_type_members` did not.
///
/// The invariant under test is that `صدّر صنف س` behaves exactly like `صنف س`.
/// `test_unexported_class_constructs_and_reads_member` is the explicit control
/// for that pairing; the remaining cases assert only the exported form, relying
/// on the `dispatch`, `statics` and `upcasting` modules above for un-exported
/// coverage of the same shapes.
mod exported_declarations {
    use super::*;

    #[test]
    fn test_exported_class_constructs_and_reads_member() {
        let source = r#"
            صدّر صنف نقطة {
                خاص س: عدد
                منشئ(س: عدد) { هذا.س = س }
                عام دالة اقرأ() -> عدد { أرجع هذا.س }
            }
            دالة رئيسية() {
                متغير ن = جديد نقطة(5)
                اطبع(ن.اقرأ())
            }
        "#;
        assert_stdout_both_modes(source, &["5"]);
    }

    #[test]
    fn test_unexported_class_constructs_and_reads_member() {
        // The control: identical but for `صدّر`. Passed throughout, which is
        // what made the exported form's failure a pure export-path defect.
        let source = r#"
            صنف نقطة {
                خاص س: عدد
                منشئ(س: عدد) { هذا.س = س }
                عام دالة اقرأ() -> عدد { أرجع هذا.س }
            }
            دالة رئيسية() {
                متغير ن = جديد نقطة(5)
                اطبع(ن.اقرأ())
            }
        "#;
        assert_stdout_both_modes(source, &["5"]);
    }

    #[test]
    fn test_exported_class_static_factory_constructs_own_class() {
        // The literal #259 repro, and the pattern `stdlib/` is built on:
        // وقت.الآن()، خادم_نقل.بعنوان(...)، طلب.برابط(...). The factory
        // constructs the very class that declares it, so it fails on both
        // counts — `جديد` and the following member call.
        let source = r#"
            صدّر صنف نقطة {
                خاص س: عدد
                منشئ(س: عدد) { هذا.س = س }
                مشترك دالة بقيمة(س: عدد) -> نقطة { أرجع جديد نقطة(س) }
                عام دالة اقرأ() -> عدد { أرجع هذا.س }
            }
            دالة رئيسية() {
                اطبع(نقطة.بقيمة(5).اقرأ())
            }
        "#;
        assert_stdout_both_modes(source, &["5"]);
    }

    #[test]
    fn test_exported_class_static_field_is_registered() {
        let source = r#"
            صدّر صنف عدادات {
                عام مشترك المجموع: عدد = 7
            }
            دالة رئيسية() {
                اطبع(عدادات.المجموع)
            }
        "#;
        assert_stdout_both_modes(source, &["7"]);
    }

    #[test]
    fn test_inherited_method_reaches_exported_parent() {
        // An exported parent contaminated its *un-exported* children: the
        // empty member table propagated into the subclass's vtable, so a
        // subclass that never mentions `صدّر` still failed.
        let source = r#"
            صدّر صنف أصل {
                عام قيمة: عدد
                منشئ(ق: عدد) { هذا.قيمة = ق }
                عام دالة اعرض() -> عدد { أرجع هذا.قيمة }
            }
            صنف فرع يرث أصل {
                منشئ(ق: عدد) { الأصل(ق) }
            }
            دالة رئيسية() {
                متغير كائن = جديد فرع(7)
                اطبع(كائن.اعرض())
            }
        "#;
        assert_stdout_both_modes(source, &["7"]);
    }

    #[test]
    fn test_exported_generic_class_constructs_and_reads() {
        // `type_params` were registered by pass 1 all along; only the members
        // went missing, so a generic exported class failed for the same reason
        // a plain one did.
        //
        // The constructor deliberately takes `عدد` rather than `ن`: a
        // type-parameter-typed constructor argument is rejected as
        // `ن٠٠٠١ متوقع ن، وُجد عدد` whether or not the class is exported — a
        // separate generic-substitution gap that would mask this case.
        let source = r#"
            صدّر صنف حاوية<ن> {
                خاص العدد: عدد
                منشئ(ع: عدد) { هذا.العدد = ع }
                عام دالة اجلب() -> عدد { أرجع هذا.العدد }
            }
            دالة رئيسية() {
                متغير ح = جديد حاوية<عدد>(9)
                اطبع(ح.اجلب())
            }
        "#;
        assert_stdout_both_modes(source, &["9"]);
    }

    #[test]
    fn test_exported_interface_implemented_completely_runs() {
        // `add_type_members` also feeds `add_interface_methods`, so an
        // exported ميثاق lost its methods too — and a class that *did*
        // implement it was reported ص٠٢٠١ anyway once the pairing was the
        // other way round (plain ميثاق + exported class).
        let source = r#"
            صدّر ميثاق شكل {
                دالة مساحة() -> عدد
            }
            صدّر صنف مربع يلتزم شكل {
                خاص ض: عدد
                منشئ(ض: عدد) { هذا.ض = ض }
                عام دالة مساحة() -> عدد { أرجع هذا.ض * هذا.ض }
            }
            دالة رئيسية() {
                متغير م = جديد مربع(5)
                اطبع(م.مساحة())
            }
        "#;
        assert_stdout_both_modes(source, &["25"]);
    }

    #[test]
    fn test_exported_generic_interface_accepts_concrete_implementation() {
        // Registering an exported ميثاق's methods activates the contract check,
        // which compared types by name. `ميثاق حاوية<ن>` resolves `ن` to
        // `Type::Class("ن")` — a name no implementation can match — so every
        // implementor of a generic contract was rejected with ص٠٢٠١. An
        // unsubstituted type parameter must impose no requirement.
        let source = r#"
            صدّر ميثاق حاوية<ن> {
                دالة ضع(عنصر: ن)
                دالة اجلب() -> ن
            }
            صنف صندوق يلتزم حاوية<عدد> {
                خاص ق: عدد
                منشئ() { هذا.ق = 0 }
                عام دالة ضع(عنصر: عدد) { هذا.ق = عنصر }
                عام دالة اجلب() -> عدد { أرجع هذا.ق }
            }
            دالة رئيسية() {
                متغير ص = جديد صندوق()
                ص.ضع(5)
                اطبع(ص.اجلب())
            }
        "#;
        assert_stdout_both_modes(source, &["5"]);
    }

    #[test]
    fn test_exported_interface_with_أي_parameter_accepts_concrete_type() {
        // LANGUAGE_SPEC §9.7's own `قابل_للمقارنة` example: `أي` is specified as
        // "يقبل أي نمط" (§5.5), so a contract parameter typed `أي` is satisfied
        // by any concrete type. Comparing by name rejected it.
        let source = r#"
            صدّر ميثاق قابل_للمقارنة {
                دالة قارن(آخر: أي) -> عدد
            }
            صنف مربع يلتزم قابل_للمقارنة {
                منشئ() {}
                عام دالة قارن(آخر: مربع) -> عدد { أرجع 0 }
            }
            دالة رئيسية() {
                متغير م = جديد مربع()
                اطبع(م.قارن(م))
            }
        "#;
        assert_stdout_both_modes(source, &["0"]);
    }

    #[test]
    fn test_override_of_exported_generic_parent_is_accepted() {
        // The same unsubstituted-type-parameter defect reached
        // `check_method_overrides` through an exported generic *parent*, so a
        // subclass narrowing `ن` to a concrete type was rejected — blocking the
        // very pattern the export fix unblocks.
        //
        // The override deliberately does not call `الأصل.أضف(عنصر)`: passing an
        // argument to a method whose parameter is a type parameter still fails
        // as `ن٠٠٠١ متوقع ن، وُجد عدد`, a separate call-site substitution gap
        // that has nothing to do with overriding and would mask this case.
        let source = r#"
            صدّر صنف مجموعة_عامة<ن> {
                خاص العدد: عدد
                منشئ() { هذا.العدد = 0 }
                عام دالة أضف(عنصر: ن) { هذا.العدد = هذا.العدد + 1 }
                عام دالة الحجم() -> عدد { أرجع هذا.العدد }
            }
            صنف أرقام يرث مجموعة_عامة {
                منشئ() { الأصل() }
                عام دالة أضف(عنصر: عدد) { اطبع(عنصر) }
            }
            دالة رئيسية() {
                متغير أ = جديد أرقام()
                أ.أضف(7)
                اطبع(أ.الحجم())
            }
        "#;
        assert_stdout_both_modes(source, &["7", "0"]);
    }

    #[test]
    fn test_refused_exception_redefinition_reports_only_its_own_error() {
        // `register_types` refuses a redefinition of the prelude's `استثناء`
        // and returns without registering it, so the entry under that name is
        // still the prelude's. `add_type_members` then wrote the user's members
        // over it, destroying `رسالة` and the single-string constructor — so the
        // one correct refusal came with two bogus errors on correct code.
        let source = r#"
            صدّر صنف استثناء {
                عام كود: عدد
                منشئ(كود: عدد) { هذا.كود = كود }
            }
            دالة رئيسية() {
                حاول {
                    ارمِ جديد استثناء("فشل")
                } التقط (خ) {
                    اطبع(خ.رسالة)
                }
            }
        "#;
        match analyze_diagnostics(source) {
            Ok(()) => panic!("توقعنا رفض إعادة تعريف 'استثناء'"),
            Err(diags) => {
                let errors: Vec<_> = diags
                    .iter()
                    .filter(|(code, _)| code.as_deref() != Some("ح٠٠٠١"))
                    .collect();
                assert!(
                    errors
                        .iter()
                        .any(|(code, _)| code.as_deref() == Some("ص٠٦٠٢")),
                    "لم يظهر رمز الرفض ص٠٦٠٢ في: {:?}",
                    diags
                );
                assert!(
                    !errors
                        .iter()
                        .any(|(code, _)| code.as_deref() == Some("ص٠٣٠١")),
                    "ظهر خطأ ص٠٣٠١ على كود صحيح — أعضاء 'استثناء' من البادئة أُتلفت: {:?}",
                    diags
                );
            }
        }
    }

    #[test]
    fn test_exported_interface_missing_method_is_rejected() {
        // The one *unsound* symptom: an exported ميثاق registered zero
        // methods, so `ClassResolver::validate` had nothing to require and a
        // class that ignored the contract compiled clean. This test therefore
        // failed in the opposite direction from the others — analysis
        // succeeded where it had to fail.
        let source = r#"
            صدّر ميثاق شكل {
                دالة مساحة() -> عدد
            }
            صنف مربع يلتزم شكل {
                منشئ() {}
            }
            دالة رئيسية() {
                متغير م = جديد مربع()
                اطبع(1)
            }
        "#;
        assert_analyze_error_code(source, "ص٠٢٠١");
    }
}
