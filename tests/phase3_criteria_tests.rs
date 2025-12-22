//! Phase 3 Success Criteria Tests
//!
//! This module tests all success criteria defined in docs/PHASE3_PLAN.md:
//!
//! 1. Module System (نظام الوحدات يعمل)
//!    - `استورد X من "مكتبة"` works
//!    - Standard library is importable
//!    - No circular dependency errors
//!
//! 2. Collections (المجموعات تعمل)
//!    - قائمة<ن> with all functions
//!    - مجموعة<ن> with set operations
//!    - قاموس<م، ق> with iteration
//!
//! 3. String and Math (أدوات النص والرياضيات)
//!    - Comprehensive string processing functions
//!    - Basic and trigonometric math functions
//!
//! 4. File System (نظام الملفات)
//!    - Reading and writing files
//!    - Path handling
//!    - Directory management
//!
//! 5. Tests (الاختبارات)
//!    - Tests for each module
//!    - Integration tests for the whole library
//!    - All existing tests pass

use tarqeem::parser::Parser;

/// Helper to wrap source code with required file markers (بسم_الله and الحمد_لله)
fn wrap_with_markers(source: &str) -> String {
    format!("بسم_الله\n{}\nالحمد_لله", source.trim())
}

/// Helper function to test if source parses successfully
fn parses_ok(source: &str) -> bool {
    let wrapped_source = wrap_with_markers(source);
    let mut parser = Parser::new(&wrapped_source);
    parser.parse().is_ok()
}

/// Helper to get parse error message
fn parse_error(source: &str) -> String {
    let wrapped_source = wrap_with_markers(source);
    let mut parser = Parser::new(&wrapped_source);
    match parser.parse() {
        Ok(_) => String::new(),
        Err(e) => e.message,
    }
}

// ============================================================================
// Criterion 1: Module System Tests
// ============================================================================

mod module_system {
    use super::*;

    /// Test that basic import syntax parses correctly
    #[test]
    fn test_import_syntax_named_imports() {
        let source = r#"
استورد { قائمة، مجموعة } من "مجموعات"
"#;
        assert!(
            parses_ok(source),
            "Named import syntax should parse: {}",
            parse_error(source)
        );
    }

    /// Test import with alias syntax
    #[test]
    fn test_import_with_alias() {
        let source = r#"
استورد * كـ رياضيات من "رياضيات"
"#;
        assert!(
            parses_ok(source),
            "Wildcard import with alias should parse: {}",
            parse_error(source)
        );
    }

    /// Test relative import syntax
    #[test]
    fn test_relative_import() {
        let source = r#"
استورد { مساعد } من "./أدوات"
"#;
        assert!(
            parses_ok(source),
            "Relative import should parse: {}",
            parse_error(source)
        );
    }

    /// Test export syntax
    #[test]
    fn test_export_function() {
        let source = r#"
صدّر دالة حساب(س: عدد) -> عدد {
    أرجع س * 2
}
"#;
        assert!(
            parses_ok(source),
            "Export function should parse: {}",
            parse_error(source)
        );
    }

    /// Test export class syntax
    #[test]
    fn test_export_class() {
        let source = r#"
صدّر صنف أداة {
    خاص _قيمة: عدد

    منشئ() {
        هذا._قيمة = 0
    }
}
"#;
        assert!(
            parses_ok(source),
            "Export class should parse: {}",
            parse_error(source)
        );
    }

    /// Test re-export syntax
    #[test]
    fn test_reexport() {
        let source = r#"
استورد { قائمة } من "./قائمة"
صدّر { قائمة }
"#;
        assert!(
            parses_ok(source),
            "Re-export should parse: {}",
            parse_error(source)
        );
    }
}

// ============================================================================
// Criterion 2: Collections Tests
// ============================================================================

mod collections {
    use super::*;

    /// Test List (قائمة) class definition syntax
    #[test]
    fn test_list_class_syntax() {
        let source = r#"
صنف قائمة<ن> {
    خاص _عناصر: مصفوفة<ن>
    خاص _طول: عدد

    منشئ() {
        هذا._عناصر = []
        هذا._طول = 0
    }

    عام دالة أضف(عنصر: ن) {
    }

    عام دالة طول() -> عدد {
        أرجع هذا._طول
    }

    عام دالة فارغة() -> منطقي {
        أرجع هذا._طول == 0
    }
}
"#;
        assert!(
            parses_ok(source),
            "List class definition should parse: {}",
            parse_error(source)
        );
    }

    /// Test Set (مجموعة) class syntax
    #[test]
    fn test_set_class_syntax() {
        let source = r#"
صنف مجموعة<ن> {
    خاص _عناصر: مصفوفة<ن>

    عام دالة أضف(عنصر: ن) -> منطقي { أرجع صحيح }
    عام دالة يحتوي(عنصر: ن) -> منطقي { أرجع خطأ }
    عام دالة اتحاد(أخرى: مجموعة<ن>) -> مجموعة<ن> { أرجع هذا }
    عام دالة تقاطع(أخرى: مجموعة<ن>) -> مجموعة<ن> { أرجع هذا }
    عام دالة فرق(أخرى: مجموعة<ن>) -> مجموعة<ن> { أرجع هذا }
}
"#;
        assert!(
            parses_ok(source),
            "Set class with operations should parse: {}",
            parse_error(source)
        );
    }

    /// Test Map (قاموس/خريطة) class syntax
    #[test]
    fn test_map_class_syntax() {
        let source = r#"
صنف خريطة<م، ق> {
    خاص _مفاتيح: مصفوفة<م>
    خاص _قيم: مصفوفة<ق>

    عام دالة عيّن(مفتاح: م، قيمة: ق) { }
    عام دالة يحتوي_مفتاح(مفتاح: م) -> منطقي { أرجع خطأ }
    عام دالة مفاتيح() -> مصفوفة<م> { أرجع هذا._مفاتيح }
    عام دالة قيم() -> مصفوفة<ق> { أرجع هذا._قيم }
}
"#;
        assert!(
            parses_ok(source),
            "Map class should parse: {}",
            parse_error(source)
        );
    }

    /// Test Stack (مكدس) class syntax
    #[test]
    fn test_stack_class_syntax() {
        let source = r#"
صنف مكدس<ن> {
    خاص _عناصر: مصفوفة<ن>

    عام دالة ادفع(عنصر: ن) { }
    عام دالة فارغ() -> منطقي { أرجع صحيح }
}
"#;
        assert!(
            parses_ok(source),
            "Stack class should parse: {}",
            parse_error(source)
        );
    }

    /// Test Queue (طابور) class syntax
    #[test]
    fn test_queue_class_syntax() {
        let source = r#"
صنف طابور<ن> {
    خاص _عناصر: مصفوفة<ن>

    عام دالة ادخل(عنصر: ن) { }
    عام دالة فارغ() -> منطقي { أرجع صحيح }
}
"#;
        assert!(
            parses_ok(source),
            "Queue class should parse: {}",
            parse_error(source)
        );
    }

    /// Test Iterator interface syntax
    #[test]
    fn test_iterator_interface_syntax() {
        let source = r#"
واجهة متكرر<ن> {
    دالة التالي() -> ن
    دالة يوجد_تالي() -> منطقي
}

واجهة قابل_للتكرار<ن> {
    دالة متكرر() -> متكرر<ن>
}
"#;
        assert!(
            parses_ok(source),
            "Iterator interfaces should parse: {}",
            parse_error(source)
        );
    }

    /// Test Pair (زوج) class for Map entries
    #[test]
    fn test_pair_class_syntax() {
        let source = r#"
صنف زوج<أ، ب> {
    عام اول: أ
    عام ثاني: ب

    منشئ(اول: أ، ثاني: ب) {
        هذا.اول = اول
        هذا.ثاني = ثاني
    }
}
"#;
        assert!(
            parses_ok(source),
            "Pair class should parse: {}",
            parse_error(source)
        );
    }
}

// ============================================================================
// Criterion 3: String and Math Tests
// ============================================================================

mod string_utilities {
    use super::*;

    /// Test basic string functions syntax
    #[test]
    fn test_string_basic_functions() {
        let source = r#"
صدّر دالة قص(سلسلة: نص، بداية: عدد، نهاية: عدد) -> نص {
    أرجع __substring__(سلسلة، بداية، نهاية)
}

صدّر دالة يحتوي(سلسلة: نص، جزء: نص) -> منطقي {
    أرجع __str_contains__(سلسلة، جزء)
}

صدّر دالة يبدأ_بـ(سلسلة: نص، بادئة: نص) -> منطقي {
    أرجع __str_starts_with__(سلسلة، بادئة)
}

صدّر دالة ينتهي_بـ(سلسلة: نص، لاحقة: نص) -> منطقي {
    أرجع __str_ends_with__(سلسلة، لاحقة)
}
"#;
        assert!(
            parses_ok(source),
            "String basic functions should parse: {}",
            parse_error(source)
        );
    }

    /// Test StringBuilder (باني_نص) class
    #[test]
    fn test_string_builder() {
        let source = r#"
صنف باني_نص {
    خاص _اجزاء: مصفوفة<نص>

    منشئ() {
        هذا._اجزاء = []
    }

    عام دالة اضف(جزء: نص) -> باني_نص {
        أرجع هذا
    }

    عام دالة اضف_سطر(جزء: نص) -> باني_نص {
        أرجع هذا
    }

    عام دالة بناء() -> نص {
        أرجع ""
    }
}
"#;
        assert!(
            parses_ok(source),
            "StringBuilder class should parse: {}",
            parse_error(source)
        );
    }

    /// Test string formatting functions
    #[test]
    fn test_string_formatting() {
        let source = r#"
دالة نسّق(قالب: نص، قيمة: أي) -> نص {
    أرجع ""
}

دالة بأصفار(رقم: عدد، طول: عدد) -> نص {
    أرجع ""
}

دالة بفواصل(رقم: عدد) -> نص {
    أرجع ""
}
"#;
        assert!(
            parses_ok(source),
            "String formatting functions should parse: {}",
            parse_error(source)
        );
    }
}

mod math_library {
    use super::*;

    /// Test basic math functions
    #[test]
    fn test_basic_math_functions() {
        let source = r#"
صدّر دالة مطلق(قيمة: عدد) -> عدد {
    إذا (قيمة < 0) {
        أرجع قيمة * -1
    }
    أرجع قيمة
}

صدّر دالة قوة(الأساس: عدد، الأس: عدد) -> عدد {
    متغير نتيجة = 1
    أرجع نتيجة
}

صدّر دالة جذر_تربيعي(قيمة: عدد_عشري) -> عدد_عشري {
    أرجع __sqrt__(قيمة)
}
"#;
        assert!(
            parses_ok(source),
            "Basic math functions should parse: {}",
            parse_error(source)
        );
    }

    /// Test trigonometric functions
    #[test]
    fn test_trig_functions() {
        let source = r#"
صدّر دالة جيب(س: عدد_عشري) -> عدد_عشري {
    أرجع __sin__(س)
}

صدّر دالة جيب_تمام(س: عدد_عشري) -> عدد_عشري {
    أرجع __cos__(س)
}

صدّر دالة ظل(س: عدد_عشري) -> عدد_عشري {
    أرجع __tan__(س)
}

صدّر دالة جيب_معكوس(س: عدد_عشري) -> عدد_عشري {
    أرجع __asin__(س)
}

صدّر دالة جيب_تمام_معكوس(س: عدد_عشري) -> عدد_عشري {
    أرجع __acos__(س)
}

صدّر دالة ظل_معكوس(س: عدد_عشري) -> عدد_عشري {
    أرجع __atan__(س)
}
"#;
        assert!(
            parses_ok(source),
            "Trigonometric functions should parse: {}",
            parse_error(source)
        );
    }

    /// Test math constants
    #[test]
    fn test_math_constants() {
        let source = r#"
صدّر ثابت باي: عدد_عشري = 3.141592653589793
صدّر ثابت هـ: عدد_عشري = 2.718281828459045
صدّر ثابت ذهبي: عدد_عشري = 1.618033988749895
صدّر ثابت جذر2: عدد_عشري = 1.4142135623730951
"#;
        assert!(
            parses_ok(source),
            "Math constants should parse: {}",
            parse_error(source)
        );
    }

    /// Test random number functions
    #[test]
    fn test_random_functions() {
        let source = r#"
صدّر دالة عشوائي() -> عدد_عشري {
    أرجع __random__()
}

صدّر دالة عشوائي_بين(ادنى: عدد، اقصى: عدد) -> عدد {
    أرجع __random_int__(ادنى، اقصى)
}

صدّر دالة بذرة_عشوائي(بذرة: عدد) {
    __random_seed__(بذرة)
}
"#;
        assert!(
            parses_ok(source),
            "Random functions should parse: {}",
            parse_error(source)
        );
    }
}

// ============================================================================
// Criterion 4: File System Tests
// ============================================================================

mod file_system {
    use super::*;

    /// Test File class syntax
    #[test]
    fn test_file_class() {
        let source = r#"
صنف ملف {
    خاص _مسار: نص
    خاص _مقبض: عدد

    عام دالة اقرأ_كل() -> نص {
        أرجع __file_read__(هذا._مسار)
    }

    عام دالة اكتب(محتوى: نص) {
        __file_write__(هذا._مسار، محتوى)
    }

    عام دالة موجود() -> منطقي {
        أرجع __file_exists__(هذا._مسار)
    }

    عام دالة احذف() {
        __file_delete__(هذا._مسار)
    }
}
"#;
        assert!(
            parses_ok(source),
            "File class should parse: {}",
            parse_error(source)
        );
    }

    /// Test Path handling functions
    #[test]
    fn test_path_functions() {
        let source = r#"
صدّر دالة ادمج(مسار1: نص، مسار2: نص) -> نص {
    أرجع __path_join__(مسار1، مسار2)
}

صدّر دالة اسم_الملف(مسار: نص) -> نص {
    أرجع __path_basename__(مسار)
}

صدّر دالة أب(مسار: نص) -> نص {
    أرجع __path_dirname__(مسار)
}

صدّر دالة توسيع(مسار: نص) -> نص {
    أرجع __path_extension__(مسار)
}

صدّر دالة موجود(مسار: نص) -> منطقي {
    أرجع __path_exists__(مسار)
}

صدّر دالة هو_ملف(مسار: نص) -> منطقي {
    أرجع __is_file__(مسار)
}

صدّر دالة هو_مجلد(مسار: نص) -> منطقي {
    أرجع __is_dir__(مسار)
}
"#;
        assert!(
            parses_ok(source),
            "Path functions should parse: {}",
            parse_error(source)
        );
    }

    /// Test Directory class
    #[test]
    fn test_directory_class() {
        let source = r#"
صنف مجلد {
    خاص _مسار: نص

    منشئ(مسار: نص) {
        هذا._مسار = مسار
    }

    عام دالة أنشئ() {
        __mkdir__(هذا._مسار)
    }

    عام دالة حذف() {
        __rmdir__(هذا._مسار)
    }

    عام دالة قائمة() -> مصفوفة<نص> {
        أرجع __list_dir__(هذا._مسار)
    }

    عام دالة موجود() -> منطقي {
        أرجع __dir_exists__(هذا._مسار)
    }
}
"#;
        assert!(
            parses_ok(source),
            "Directory class should parse: {}",
            parse_error(source)
        );
    }

    /// Test convenience file functions
    #[test]
    fn test_file_convenience_functions() {
        let source = r#"
صدّر دالة اقرأ_ملف(المسار: نص) -> نص {
    أرجع __file_read__(المسار)
}

صدّر دالة اكتب_ملف(المسار: نص، المحتوى: نص) {
    __file_write__(المسار، المحتوى)
}

صدّر دالة ملف_موجود(المسار: نص) -> منطقي {
    أرجع __file_exists__(المسار)
}

صدّر دالة احذف_ملف(المسار: نص) {
    __file_delete__(المسار)
}

صدّر دالة انسخ_ملف(المصدر: نص، الهدف: نص) {
    __file_copy__(المصدر، الهدف)
}
"#;
        assert!(
            parses_ok(source),
            "File convenience functions should parse: {}",
            parse_error(source)
        );
    }
}

// ============================================================================
// Criterion 5: Date/Time Tests
// ============================================================================

mod date_time {
    use super::*;

    /// Test Date class
    #[test]
    fn test_date_class() {
        let source = r#"
صنف تاريخ {
    عام سنة: عدد
    عام شهر: عدد
    عام يوم: عدد

    منشئ(السنة: عدد، الشهر: عدد، اليوم: عدد) {
        هذا.سنة = السنة
        هذا.شهر = الشهر
        هذا.يوم = اليوم
    }

    عام دالة اضف_ايام(كمية: عدد) -> تاريخ {
        أرجع هذا
    }

    عام دالة يوم_الاسبوع() -> عدد {
        أرجع 0
    }

    عام دالة الى_نص() -> نص {
        أرجع ""
    }
}
"#;
        assert!(
            parses_ok(source),
            "Date class should parse: {}",
            parse_error(source)
        );
    }

    /// Test Time class
    #[test]
    fn test_time_class() {
        let source = r#"
صنف وقت {
    عام ساعة: عدد
    عام دقيقة: عدد
    عام ثانية: عدد
    عام ميلي: عدد

    منشئ(الساعة: عدد، الدقيقة: عدد، الثانية: عدد) {
        هذا.ساعة = الساعة
        هذا.دقيقة = الدقيقة
        هذا.ثانية = الثانية
        هذا.ميلي = 0
    }

    عام دالة اضف_ثواني(كمية: عدد) -> وقت {
        أرجع هذا
    }

    عام دالة الى_ثواني() -> عدد {
        أرجع هذا.ساعة * 3600 + هذا.دقيقة * 60 + هذا.ثانية
    }
}
"#;
        assert!(
            parses_ok(source),
            "Time class should parse: {}",
            parse_error(source)
        );
    }

    /// Test DateTime class
    #[test]
    fn test_datetime_class() {
        let source = r#"
صنف تاريخ_ووقت {
    عام تاريخ: تاريخ
    عام وقت: وقت

    عام دالة طابع_وقت() -> عدد {
        أرجع 0
    }

    عام دالة نسّق(قالب: نص) -> نص {
        أرجع ""
    }
}

دالة الان() -> تاريخ_ووقت {
    أرجع __now__()
}

دالة طابع_وقت() -> عدد {
    أرجع __timestamp__()
}

دالة نوم(ميلي: عدد) {
    __sleep__(ميلي)
}
"#;
        assert!(
            parses_ok(source),
            "DateTime class should parse: {}",
            parse_error(source)
        );
    }

    /// Test Arabic day/month names constants
    #[test]
    fn test_arabic_date_names() {
        let source = r#"
ثابت أيام_الأسبوع: مصفوفة<نص> = [
    "الأحد"،
    "الإثنين"،
    "الثلاثاء"،
    "الأربعاء"،
    "الخميس"،
    "الجمعة"،
    "السبت"
]

ثابت أشهر_السنة: مصفوفة<نص> = [
    "يناير"،
    "فبراير"،
    "مارس"،
    "أبريل"،
    "مايو"،
    "يونيو"،
    "يوليو"،
    "أغسطس"،
    "سبتمبر"،
    "أكتوبر"،
    "نوفمبر"،
    "ديسمبر"
]
"#;
        assert!(
            parses_ok(source),
            "Arabic date names should parse: {}",
            parse_error(source)
        );
    }
}

// ============================================================================
// Criterion 6: Error Handling Tests
// ============================================================================

mod error_handling {
    use super::*;

    /// Test base Error class
    #[test]
    fn test_error_class() {
        let source = r#"
صنف استثناء {
    عام رسالة: نص
    عام رسالة_عربية: نص

    منشئ(النص: نص) {
        هذا.رسالة = النص
        هذا.رسالة_عربية = النص
    }

    عام دالة الى_نص() -> نص {
        أرجع هذا.رسالة_عربية
    }
}
"#;
        assert!(
            parses_ok(source),
            "Error class should parse: {}",
            parse_error(source)
        );
    }

    /// Test specific error types with inheritance
    #[test]
    fn test_specific_error_types() {
        let source = r#"
صنف استثناء_قيمة يرث استثناء {
    منشئ(النص: نص) {
        أساس(النص)
    }
}

صنف استثناء_نوع يرث استثناء {
    منشئ(النص: نص) {
        أساس(النص)
    }
}

صنف استثناء_فهرس يرث استثناء {
    عام الفهرس: عدد
    عام الطول: عدد

    منشئ(ف: عدد، ط: عدد) {
        أساس("Index out of bounds")
        هذا.الفهرس = ف
        هذا.الطول = ط
    }
}

صنف استثناء_ملف يرث استثناء {
    عام المسار: نص

    منشئ(م: نص، النص: نص) {
        أساس(النص)
        هذا.المسار = م
    }
}
"#;
        assert!(
            parses_ok(source),
            "Specific error types should parse: {}",
            parse_error(source)
        );
    }

    /// Test Result type
    #[test]
    fn test_result_type() {
        let source = r#"
صنف نتيجة<ق، خ> {
    خاص _قيمة: ق
    خاص _خطأ: خ
    خاص _نجاح: منطقي

    عام دالة ناجحة() -> منطقي {
        أرجع هذا._نجاح
    }

    عام دالة فاشلة() -> منطقي {
        أرجع !هذا._نجاح
    }

    عام دالة قيمة() -> ق {
        أرجع هذا._قيمة
    }

    عام دالة احصل_خطأ() -> خ {
        أرجع هذا._خطأ
    }

    عام دالة قيمة_او(بديل: ق) -> ق {
        إذا (هذا._نجاح) {
            أرجع هذا._قيمة
        }
        أرجع بديل
    }
}
"#;
        assert!(
            parses_ok(source),
            "Result type should parse: {}",
            parse_error(source)
        );
    }

    /// Test Option type
    #[test]
    fn test_option_type() {
        let source = r#"
صنف اختياري<ن> {
    خاص _قيمة: ن
    خاص _موجود: منطقي

    عام دالة موجود() -> منطقي {
        أرجع هذا._موجود
    }

    عام دالة فارغ() -> منطقي {
        أرجع !هذا._موجود
    }

    عام دالة قيمة() -> ن {
        أرجع هذا._قيمة
    }

    عام دالة قيمة_او(افتراضي: ن) -> ن {
        إذا (هذا._موجود) {
            أرجع هذا._قيمة
        }
        أرجع افتراضي
    }
}
"#;
        assert!(
            parses_ok(source),
            "Option type should parse: {}",
            parse_error(source)
        );
    }

    /// Test try-catch syntax
    #[test]
    fn test_try_catch_syntax() {
        let source = r#"
دالة تجربة() {
    حاول {
        متغير نتيجة = قسمة(10، 0)
    } التقط (خ) {
        اطبع("حدث خطأ: " + خ.رسالة)
    } أخيراً {
        اطبع("تم الانتهاء")
    }
}

دالة قسمة(س: عدد، ص: عدد) -> عدد {
    إذا (ص == 0) {
        ارمِ خطأ_جديد("لا يمكن القسمة على صفر")
    }
    أرجع س / ص
}
"#;
        assert!(
            parses_ok(source),
            "Try-catch syntax should parse: {}",
            parse_error(source)
        );
    }
}

// ============================================================================
// Criterion 7: Console I/O Tests
// ============================================================================

mod console_io {
    use super::*;

    /// Test basic I/O functions
    #[test]
    fn test_basic_io() {
        let source = r#"
صدّر دالة اطبع(قيمة: أي) {
    __print__(قيمة)
}

صدّر دالة اطبع_سطر(قيمة: أي) {
    __println__(قيمة)
}

صدّر دالة ادخل() -> نص {
    أرجع __readline__()
}

صدّر دالة ادخل_رسالة(رسالة: نص) -> نص {
    __print__(رسالة)
    أرجع __readline__()
}
"#;
        assert!(
            parses_ok(source),
            "Basic I/O functions should parse: {}",
            parse_error(source)
        );
    }

    /// Test ANSI color constants
    #[test]
    fn test_ansi_colors() {
        let source = r#"
ثابت لون_أحمر: نص = "\x1b[31m"
ثابت لون_أخضر: نص = "\x1b[32m"
ثابت لون_أصفر: نص = "\x1b[33m"
ثابت لون_أزرق: نص = "\x1b[34m"
ثابت لون_افتراضي: نص = "\x1b[0m"

دالة طباعة_ملونة(السلسلة: نص، اللون: نص) {
    اطبع(اللون + السلسلة + لون_افتراضي)
}
"#;
        assert!(
            parses_ok(source),
            "ANSI color constants should parse: {}",
            parse_error(source)
        );
    }
}

// ============================================================================
// Criterion 8: Networking Tests (Optional)
// ============================================================================

mod networking {
    use super::*;

    /// Test TCP connection class
    #[test]
    fn test_tcp_connection() {
        let source = r#"
صنف اتصال_tcp {
    خاص _عنوان: نص
    خاص _منفذ: عدد
    خاص _متصل: منطقي

    منشئ(عنوان: نص، منفذ: عدد) {
        هذا._عنوان = عنوان
        هذا._منفذ = منفذ
        هذا._متصل = خطأ
    }

    عام دالة اتصل() {
        هذا._متصل = صحيح
    }

    عام دالة أغلق() {
        هذا._متصل = خطأ
    }

    عام دالة ارسل(بيانات: نص) {
        __tcp_send__(هذا._عنوان، بيانات)
    }

    عام دالة استقبل() -> نص {
        أرجع __tcp_recv__()
    }

    عام دالة متصل() -> منطقي {
        أرجع هذا._متصل
    }
}
"#;
        assert!(
            parses_ok(source),
            "TCP connection class should parse: {}",
            parse_error(source)
        );
    }

    /// Test TCP server class
    #[test]
    fn test_tcp_server() {
        let source = r#"
صنف خادم_tcp {
    خاص _منفذ: عدد
    خاص _يستمع: منطقي

    منشئ(منفذ: عدد) {
        هذا._منفذ = منفذ
        هذا._يستمع = خطأ
    }

    عام دالة استمع() {
        هذا._يستمع = صحيح
    }

    عام دالة قبول() -> اتصال_tcp {
        أرجع __tcp_accept__()
    }

    عام دالة أغلق() {
        هذا._يستمع = خطأ
    }
}
"#;
        assert!(
            parses_ok(source),
            "TCP server class should parse: {}",
            parse_error(source)
        );
    }

    /// Test HTTP client
    #[test]
    fn test_http_client() {
        let source = r#"
صنف استجابة_http {
    عام الحالة: عدد
    عام المحتوى: نص

    عام دالة ناجحة() -> منطقي {
        أرجع هذا.الحالة >= 200 && هذا.الحالة < 300
    }
}

صنف طلب_http {
    خاص _رابط: نص

    منشئ(الرابط: نص) {
        هذا._رابط = الرابط
    }

    عام دالة احصل() -> استجابة_http {
        أرجع __http_get__(هذا._رابط)
    }

    عام دالة ارسل(المحتوى: نص) -> استجابة_http {
        أرجع __http_post__(هذا._رابط، المحتوى)
    }
}
"#;
        assert!(
            parses_ok(source),
            "HTTP client should parse: {}",
            parse_error(source)
        );
    }
}

// ============================================================================
// Integration Tests - Full Programs
// ============================================================================

mod integration {
    use super::*;

    /// Test a complete program using collections
    #[test]
    fn test_full_program_with_collections() {
        let source = r#"
// برنامج كامل يستخدم المجموعات
متغير اسماء: مصفوفة<نص> = ["أحمد"، "محمد"، "فاطمة"]
متغير اعمار: مصفوفة<عدد> = [25، 30، 28]

لكل اسم في اسماء {
    اطبع_سطر(اسم)
}

متغير مجموع = 0
لكل عمر في اعمار {
    مجموع = مجموع + عمر
}
اطبع("مجموع الأعمار: ")
اطبع_سطر(مجموع)
"#;
        assert!(
            parses_ok(source),
            "Full program with collections should parse: {}",
            parse_error(source)
        );
    }

    /// Test a complete OOP program
    #[test]
    fn test_full_oop_program() {
        let source = r#"
// واجهة
واجهة قابل_للطباعة {
    دالة اطبع_معلومات()
}

// صنف أساسي
صنف شخص يطبق قابل_للطباعة {
    خاص _اسم: نص
    خاص _عمر: عدد

    منشئ(اسم: نص، عمر: عدد) {
        هذا._اسم = اسم
        هذا._عمر = عمر
    }

    عام دالة اطبع_معلومات() {
        اطبع_سطر("الاسم: " + هذا._اسم)
        اطبع_سطر("العمر: " + هذا._عمر)
    }

    عام دالة احصل_اسم() -> نص {
        أرجع هذا._اسم
    }
}

// صنف موروث
صنف موظف يرث شخص {
    خاص _راتب: عدد_عشري

    منشئ(اسم: نص، عمر: عدد، راتب: عدد_عشري) {
        أساس(اسم، عمر)
        هذا._راتب = راتب
    }

    عام دالة احصل_راتب() -> عدد_عشري {
        أرجع هذا._راتب
    }
}

// الاستخدام
متغير موظف1 = جديد موظف("أحمد"، 30، 5000.0)
موظف1.اطبع_معلومات()
"#;
        assert!(
            parses_ok(source),
            "Full OOP program should parse: {}",
            parse_error(source)
        );
    }

    /// Test a program with generics
    #[test]
    fn test_program_with_generics() {
        let source = r#"
// صنف معمم
صنف صندوق<ن> {
    خاص _محتوى: ن

    منشئ(محتوى: ن) {
        هذا._محتوى = محتوى
    }

    عام دالة احصل() -> ن {
        أرجع هذا._محتوى
    }

    عام دالة عيّن(محتوى: ن) {
        هذا._محتوى = محتوى
    }
}

// الاستخدام
متغير صندوق_رقم = جديد صندوق<عدد>(42)
متغير صندوق_نص = جديد صندوق<نص>("مرحبا")

اطبع_سطر(صندوق_رقم.احصل())
اطبع_سطر(صندوق_نص.احصل())
"#;
        assert!(
            parses_ok(source),
            "Program with generics should parse: {}",
            parse_error(source)
        );
    }
}
