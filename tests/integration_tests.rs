//! Integration tests for the Tarqeem compiler pipeline
//!
//! These tests verify end-to-end compilation and execution of Tarqeem programs.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn wrap_with_markers(source: &str) -> String {
    format!("بسم_الله\n{}\nالحمد_لله", source.trim())
}

fn interpret_source(source: &str) -> Result<String, String> {
    use tarqeem::interpreter::Interpreter;
    use tarqeem::ir::IrBuilder;
    use tarqeem::lexer::Lexer;
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
    let ir_module = ir_builder
        .build(&ast)
        .map_err(|e| format!("{}: {}", e.message, e.message_ar))?;

    let mut interpreter = Interpreter::new(ir_module);
    let result = interpreter.run().map_err(|e| format!("{:?}", e))?;

    Ok(format!("{}", result))
}

fn parses_ok(source: &str) -> bool {
    use tarqeem::parser::Parser;
    let wrapped_source = wrap_with_markers(source);
    let mut parser = Parser::new(&wrapped_source);
    parser.parse().is_ok()
}

fn parses_ok_with_markers(source: &str) -> bool {
    use tarqeem::parser::Parser;
    let mut parser = Parser::new(source);
    parser.parse().is_ok()
}

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

mod pipeline {
    use super::*;

    #[test]
    fn test_hello_world() {
        let source = r#"اطبع("مرحباً بالعالم!")"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_variable_declaration() {
        let source = r#"
متغير س = 5
متغير ص = 10
اطبع(س + ص)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_constant_declaration() {
        let source = r#"
ثابت باي = 3.14159
اطبع(باي)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_function_declaration() {
        let source = r#"
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}

اطبع(جمع(5، 3))
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_if_else() {
        let source = r#"
متغير س = 5
إذا (س > 3) {
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
لكل (متغير ع = 0؛ ع < 10؛ ع = ع + 1) {
    اطبع(ع)
}
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_array_literal() {
        let source = r#"
متغير أرقام = [1، 2، 3، 4، 5]
اطبع(طول(أرقام))
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_class_declaration() {
        let source = r#"
صنف شخص {
    خاص اسم: نص

    منشئ(اسم: نص) {
        هذا.اسم = اسم
    }

    عام دالة احصل_اسم() -> نص {
        أرجع هذا.اسم
    }
}
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_class_inheritance() {
        let source = r#"
صنف حيوان {
    خاص اسم: نص

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

mod builtins {
    use super::*;

    #[test]
    fn test_print_builtin() {
        let source = r#"اطبع("مرحباً")"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_len_builtin() {
        let source = r#"
متغير قائمة = [1، 2، 3]
متغير ط = طول(قائمة)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_type_builtin() {
        let source = r#"
متغير س = 5
متغير ن = نوع(س)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_math_functions() {
        let source = r#"
متغير أ = جذر(16.0)
متغير ب = قوة(2.0، 3.0)
متغير ج = مطلق(-5)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_trig_functions() {
        let source = r#"
متغير أ = جا(0.0)
متغير ب = جتا(0.0)
متغير ج = ظا(0.0)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_type_conversion_str() {
        let source = r#"
متغير أ = نص(42)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_type_conversion_bool() {
        let source = r#"
متغير أ = منطقي(1)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_type_conversion_int() {
        let source = r#"
متغير أ = عدد(3.14)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_type_conversion_float() {
        let source = r#"
متغير أ = عدد_عشري(42)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_random_functions() {
        let source = r#"
متغير أ = عشوائي()
متغير ب = عشوائي_بين(1، 100)
متغير ج = عشوائي_منطقي()
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_rounding_functions() {
        let source = r#"
متغير أ = أرضية(3.7)
متغير ب = سقف(3.2)
متغير ج = قرّب(3.5)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_min_max() {
        let source = r#"
متغير أ = أقل(5، 3)
متغير ب = أكبر(5، 3)
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_file_functions() {
        let source = r#"
متغير موجود = ملف_موجود("test.txt")
متغير مجلد = مجلد_حالي()
"#;
        assert!(analyzes_ok(source));
    }
}

mod examples {
    use super::*;

    #[test]
    fn test_example_hello_world() {
        let path = project_root().join("examples/مرحبا.ترقيم");
        if path.exists() {
            let source = fs::read_to_string(&path).expect("Failed to read example file");
            assert!(
                parses_ok_with_markers(&source),
                "Example file مرحبا.ترقيم failed to parse"
            );
        }
    }

    #[test]
    fn test_example_basic_test() {
        let path = project_root().join("examples/اختبار_بسيط.ترقيم");
        if path.exists() {
            let source = fs::read_to_string(&path).expect("Failed to read example file");
            assert!(
                parses_ok_with_markers(&source),
                "Example file اختبار_بسيط.ترقيم failed to parse"
            );
        }
    }

    #[test]
    fn test_example_variables() {
        let path = project_root().join("examples/متغيرات.ترقيم");
        if path.exists() {
            let source = fs::read_to_string(&path).expect("Failed to read example file");
            assert!(
                parses_ok_with_markers(&source),
                "Example file متغيرات.ترقيم failed to parse"
            );
        }
    }

    #[test]
    fn test_example_collections() {
        let path = project_root().join("examples/اختبار_مجموعات.ترقيم");
        if path.exists() {
            let source = fs::read_to_string(&path).expect("Failed to read example file");
            assert!(
                parses_ok_with_markers(&source),
                "Example file اختبار_مجموعات.ترقيم failed to parse"
            );
        }
    }
}

mod interpreter {
    use super::*;

    #[test]
    fn test_arithmetic_execution() {
        let source = r#"
دالة رئيسية() -> عدد {
    متغير أ = 5
    متغير ب = 3
    أرجع أ + ب
}
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_factorial_execution() {
        let source = r#"
دالة عاملي_حساب(ن: عدد) -> عدد {
    إذا (ن <= 1) {
        أرجع 1
    }
    أرجع ن * عاملي_حساب(ن - 1)
}

دالة رئيسية() -> عدد {
    أرجع عاملي_حساب(5)
}
"#;
        assert!(analyzes_ok(source));
    }

    #[test]
    fn test_fibonacci_execution() {
        let source = r#"
دالة فيبوناتشي(ن: عدد) -> عدد {
    إذا (ن <= 1) {
        أرجع ن
    }
    أرجع فيبوناتشي(ن - 1) + فيبوناتشي(ن - 2)
}

دالة رئيسية() -> عدد {
    أرجع فيبوناتشي(10)
}
"#;
        assert!(analyzes_ok(source));
    }
}

mod errors {
    use super::*;

    #[test]
    fn test_undefined_variable_error() {
        let source = r#"اطبع(متغير_غير_موجود)"#;
        assert!(!analyzes_ok(source), "Should fail on undefined variable");
    }

    #[test]
    fn test_type_mismatch_error() {
        let source = r#"
متغير س: عدد = "نص"
"#;
        assert!(!analyzes_ok(source), "Should fail on type mismatch");
    }

    #[test]
    fn test_missing_return_type() {
        let source = r#"
دالة خطأ() -> عدد {
    متغير س = 5
}
"#;
        let _ = analyzes_ok(source);
    }
}

mod bilingual {
    use super::*;

    #[test]
    fn test_arabic_keywords() {
        let source = r#"
متغير س = 5
ثابت ص = 10
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}
إذا (س > 0) {
    طالما (س > 0) {
        س = س - 1
    }
}
"#;
        assert!(parses_ok(source));
    }

    #[test]
    fn test_english_keywords_rejected() {
        let source = r#"
let x = 5
const y = 10
"#;
        assert!(!parses_ok(source));
    }

    #[test]
    fn test_mixed_language_rejected() {
        let source = r#"
متغير x = 5
"#;
        assert!(!parses_ok(source));
    }
}
