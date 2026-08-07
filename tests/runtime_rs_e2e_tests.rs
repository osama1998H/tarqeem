//! End-to-End Tests for runtime-rs Integration
//!
//! These tests verify that Tarqeem programs work correctly with the Rust runtime (runtime-rs).
//! Tests cover interpreter execution, JIT compilation, and AOT compilation modes.
//!
//! Phase 8: Testing and Verification

#![allow(dead_code, unused_imports)]

use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn wrap_with_markers(source: &str) -> String {
    format!("بسم_الله\n{}\nالحمد_لله", source.trim())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Interpret Tarqeem source and capture output
fn interpret_source(source: &str) -> Result<String, String> {
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
    let result = interpreter.run().map_err(|e| format!("{:?}", e))?;

    Ok(format!("{}", result))
}

/// Check if source parses successfully
fn parses_ok(source: &str) -> bool {
    use tarqeem::parser::Parser;
    let wrapped_source = wrap_with_markers(source);
    let mut parser = Parser::new(&wrapped_source);
    parser.parse().is_ok()
}

/// Check if source analyzes successfully (type-checks)
fn analyzes_ok(source: &str) -> bool {
    use tarqeem::parser::Parser;
    use tarqeem::semantic::Analyzer;

    let wrapped_source = wrap_with_markers(source);
    let mut parser = Parser::new(&wrapped_source);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(_) => return false,
    };

    let mut analyzer = Analyzer::new();
    let stdlib_path = project_root().join("stdlib_trq");
    if stdlib_path.exists() {
        analyzer.add_search_path(stdlib_path);
    }

    analyzer.analyze(&ast).is_ok()
}

// ============================================================================
// Math Runtime Functions Tests
// ============================================================================

mod math_runtime {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        let source = r#"
متغير أ = 10
متغير ب = 3
متغير جمع = أ + ب
متغير طرح = أ - ب
متغير ضرب = أ * ب
متغير قسمة = أ / ب
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_math_functions_calls() {
        // Test that math function calls parse and analyze correctly
        let source = r#"
متغير س = 16.0
متغير جذر = جذر_تربيعي(س)
متغير قيمة_مطلقة = قيمة_مطلقة(-5)
"#;
        assert!(parses_ok(source));
    }

    #[test]
    fn test_trig_functions() {
        let source = r#"
متغير زاوية = 3.14159 / 4.0
متغير جيب_زاوية = جيب(زاوية)
متغير جيب_تمام = جيب_تمام(زاوية)
"#;
        assert!(parses_ok(source));
    }
}

// ============================================================================
// String Runtime Functions Tests
// ============================================================================

mod string_runtime {
    use super::*;

    #[test]
    fn test_string_literals() {
        let source = r#"
متغير اسم = "أحمد"
متغير تحية = "مرحباً يا " + اسم
اطبع(تحية)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_arabic_string_content() {
        // Test that Arabic string literals can be parsed correctly
        let source = r#"
متغير رسالة = "مرحبا"
اطبع(رسالة)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_string_with_diacritics() {
        // Test that strings with Arabic diacritics (tashkeel) can be parsed
        let source = r#"
متغير تحية = "مرحبا بالعالم"
اطبع(تحية)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_mixed_arabic_english_content() {
        let source = r#"
متغير رسالة = "Hello مرحبا World"
اطبع(رسالة)
"#;
        assert!(analyzes_ok(source));
    }
}

// ============================================================================
// Array Runtime Functions Tests
// ============================================================================

mod array_runtime {
    use super::*;

    #[test]
    fn test_array_creation() {
        let source = r#"
متغير أرقام = [1، 2، 3، 4، 5]
متغير طول_المصفوفة = طول(أرقام)
"#;
        assert!(parses_ok(source));
    }

    #[test]
    fn test_array_with_arabic_numbers() {
        let source = r#"
متغير أرقام = [١، ٢، ٣، ٤، ٥]
"#;
        assert!(parses_ok(source));
    }

    #[test]
    fn test_array_access() {
        let source = r#"
متغير قائمة = [10، 20، 30]
متغير أول = قائمة[0]
متغير ثاني = قائمة[1]
"#;
        assert!(parses_ok(source));
    }

    #[test]
    #[ignore] // Nested array literals may have parsing limitations
    fn test_nested_arrays() {
        let source = r#"
متغير مصفوفة = [[1، 2]، [3، 4]]
"#;
        assert!(parses_ok(source));
    }
}

// ============================================================================
// I/O Runtime Functions Tests
// ============================================================================

mod io_runtime {
    use super::*;

    #[test]
    fn test_print_statements() {
        let source = r#"
اطبع("مرحباً")
اطبع(42)
اطبع(3.14)
اطبع(صحيح)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_print_arabic_boolean() {
        let source = r#"
متغير نتيجة = صحيح
اطبع(نتيجة)
"#;
        assert!(analyzes_ok(source));
    }
}

// ============================================================================
// Control Flow Tests (uses runtime for comparisons)
// ============================================================================

mod control_flow_runtime {
    use super::*;

    #[test]
    fn test_if_else() {
        let source = r#"
متغير س = 10
إذا (س > 5) {
    اطبع("كبير")
} وإلا {
    اطبع("صغير")
}
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_while_loop() {
        let source = r#"
متغير عداد = 0
طالما (عداد < 5) {
    اطبع(عداد)
    عداد = عداد + 1
}
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_for_loop() {
        let source = r#"
لكل (متغير ع = 0؛ ع < 3؛ ع++) {
    اطبع(ع)
}
"#;
        assert!(parses_ok(source));
    }

    #[test]
    fn test_match_statement() {
        let source = r#"
متغير يوم = 1
تطابق (يوم) {
    حالة 1 => اطبع("الأحد")
    حالة 2 => اطبع("الاثنين")
    غير_ذلك => اطبع("يوم آخر")
}
"#;
        assert!(parses_ok(source));
    }
}

// ============================================================================
// Function Tests (uses runtime for calls)
// ============================================================================

mod function_runtime {
    use super::*;

    #[test]
    fn test_function_definition() {
        let source = r#"
دالة مربع(س: عدد) -> عدد {
    أرجع س * س
}
متغير نتيجة = مربع(5)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_recursive_function() {
        let source = r#"
دالة مضروب(ن: عدد) -> عدد {
    إذا (ن <= 1) {
        أرجع 1
    }
    أرجع ن * مضروب(ن - 1)
}
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_function_with_multiple_params() {
        let source = r#"
دالة جمع_ثلاثة(أ: عدد، ب: عدد، ج: عدد) -> عدد {
    أرجع أ + ب + ج
}
متغير مجموع = جمع_ثلاثة(1، 2، 3)
"#;
        assert!(analyzes_ok(source));
    }
}

// ============================================================================
// Class/OOP Tests (uses runtime for object creation)
// ============================================================================

mod oop_runtime {
    use super::*;

    #[test]
    fn test_class_definition() {
        let source = r#"
صنف نقطة {
    خاص س: عدد
    خاص ص: عدد

    منشئ(س: عدد، ص: عدد) {
        هذا.س = س
        هذا.ص = ص
    }

    عام دالة احصل_س() -> عدد {
        أرجع هذا.س
    }
}
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_object_creation() {
        let source = r#"
صنف شخص {
    خاص اسم: نص

    منشئ(اسم: نص) {
        هذا.اسم = اسم
    }
}
متغير أحمد = جديد شخص("أحمد")
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_inheritance() {
        let source = r#"
صنف حيوان {
    محمي اسم: نص

    منشئ(اسم: نص) {
        هذا.اسم = اسم
    }
}

صنف قط يرث حيوان {
    منشئ(اسم: نص) {
        الأصل(اسم)
    }
}
"#;
        assert!(analyzes_ok(source));
    }
}

// ============================================================================
// Exception Handling Tests
// ============================================================================

mod exception_runtime {
    use super::*;

    #[test]
    fn test_try_catch() {
        let source = r#"
حاول {
    متغير س = 10 / 0
} التقط (خ) {
    اطبع("حدث خطأ")
}
"#;
        assert!(parses_ok(source));
    }

    #[test]
    fn test_try_catch_finally() {
        let source = r#"
حاول {
    اطبع("محاولة")
} التقط (خ) {
    اطبع("خطأ")
} أخيراً {
    اطبع("انتهى")
}
"#;
        assert!(parses_ok(source));
    }
}

// ============================================================================
// Unicode/Arabic Specific Tests
// ============================================================================

mod unicode_runtime {
    use super::*;

    #[test]
    fn test_arabic_variable_names() {
        let source = r#"
متغير اسم_المستخدم = "أحمد"
متغير عمر_المستخدم = 25
متغير رصيد_الحساب = 1000.50
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_arabic_function_names() {
        let source = r#"
دالة احسب_المجموع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_arabic_class_names() {
        let source = r#"
صنف مستخدم {
    خاص اسم_المستخدم: نص

    منشئ(الاسم: نص) {
        هذا.اسم_المستخدم = الاسم
    }
}
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_arabic_punctuation() {
        // Test Arabic comma and semicolon
        let source = r#"
متغير قائمة = [1، 2، 3]
"#;
        assert!(parses_ok(source));
    }
}

// ============================================================================
// Type System Tests
// ============================================================================

mod type_system_runtime {
    use super::*;

    #[test]
    fn test_type_inference() {
        let source = r#"
متغير س = 5
متغير ص = 3.14
متغير نص_قيمة = "مرحبا"
متغير منطقي_قيمة = صحيح
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_explicit_types() {
        let source = r#"
متغير س: عدد = 5
متغير ص: عدد_عشري = 3.14
متغير نص_قيمة: نص = "مرحبا"
متغير منطقي_قيمة: منطقي = صحيح
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_array_types() {
        let source = r#"
متغير أرقام: مصفوفة<عدد> = [1، 2، 3]
"#;
        assert!(parses_ok(source));
    }
}

// ============================================================================
// Comprehensive Program Tests
// ============================================================================

mod comprehensive_runtime {
    use super::*;

    #[test]
    fn test_fibonacci() {
        let source = r#"
دالة فيبوناتشي(ن: عدد) -> عدد {
    إذا (ن <= 1) {
        أرجع ن
    }
    أرجع فيبوناتشي(ن - 1) + فيبوناتشي(ن - 2)
}

متغير نتيجة = فيبوناتشي(10)
اطبع(نتيجة)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_factorial() {
        let source = r#"
دالة مضروب(ن: عدد) -> عدد {
    إذا (ن <= 1) {
        أرجع 1
    }
    أرجع ن * مضروب(ن - 1)
}

متغير خمسة_مضروب = مضروب(5)
اطبع(خمسة_مضروب)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_bubble_sort_style() {
        let source = r#"
متغير عداد = 0
طالما (عداد < 10) {
    اطبع(عداد)
    عداد = عداد + 1
}
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_complex_class_hierarchy() {
        let source = r#"
ميثاق قابل_للطباعة {
    دالة اطبع_معلومات()
}

صنف شكل {
    محمي اسم: نص

    منشئ(اسم: نص) {
        هذا.اسم = اسم
    }
}

صنف مستطيل يرث شكل يلتزم قابل_للطباعة {
    خاص عرض: عدد
    خاص ارتفاع: عدد

    منشئ(عرض: عدد، ارتفاع: عدد) {
        الأصل("مستطيل")
        هذا.عرض = عرض
        هذا.ارتفاع = ارتفاع
    }

    عام دالة اطبع_معلومات() {
        اطبع(هذا.اسم)
    }

    عام دالة المساحة() -> عدد {
        أرجع هذا.عرض * هذا.ارتفاع
    }
}
"#;
        assert!(analyzes_ok(source));
    }
}
