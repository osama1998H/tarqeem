//! نظام رموز الأخطاء العربية في ترقيم
//!
//! Arabic error code system for Tarqeem.
//! Each error code consists of an Arabic category letter followed by
//! a 4-digit number in Arabic-Indic numerals.
//!
//! Example: د٠٣٠١ (Semantic error 301 - break outside loop)

use std::fmt;

/// فئات الأخطاء - Error categories
///
/// Each category represents a phase in the compiler pipeline
/// or a specific type of error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// ق - قراءة (Lexer errors)
    Qiraa,
    /// ب - بناء (Parser/syntax errors)
    Binaa,
    /// د - دلالة (Semantic errors)
    Dalala,
    /// ن - نوع (Type errors)
    Naw,
    /// ص - صنف (Class/OOP errors)
    Sinf,
    /// و - وحدة (Module/import errors)
    Wahda,
    /// ت - توليد (Codegen errors)
    Tawleed,
    /// ح - تحذير (Warnings)
    Tahdheer,
    /// م - مهمل (Deprecation warnings)
    Muhmal,
}

impl ErrorCategory {
    /// الحصول على الحرف العربي للفئة
    /// Get the Arabic character for this category
    pub fn arabic_char(&self) -> char {
        match self {
            Self::Qiraa => 'ق',
            Self::Binaa => 'ب',
            Self::Dalala => 'د',
            Self::Naw => 'ن',
            Self::Sinf => 'ص',
            Self::Wahda => 'و',
            Self::Tawleed => 'ت',
            Self::Tahdheer => 'ح',
            Self::Muhmal => 'م',
        }
    }

    /// الحصول على الاسم الكامل للفئة بالعربية
    /// Get the full Arabic name for this category
    pub fn arabic_name(&self) -> &'static str {
        match self {
            Self::Qiraa => "قراءة",
            Self::Binaa => "بناء",
            Self::Dalala => "دلالة",
            Self::Naw => "نوع",
            Self::Sinf => "صنف",
            Self::Wahda => "وحدة",
            Self::Tawleed => "توليد",
            Self::Tahdheer => "تحذير",
            Self::Muhmal => "مهمل",
        }
    }

    /// الحصول على الاسم بالإنجليزية
    /// Get the English name for this category
    pub fn english_name(&self) -> &'static str {
        match self {
            Self::Qiraa => "Lexer",
            Self::Binaa => "Parser",
            Self::Dalala => "Semantic",
            Self::Naw => "Type",
            Self::Sinf => "Class",
            Self::Wahda => "Module",
            Self::Tawleed => "Codegen",
            Self::Tahdheer => "Warning",
            Self::Muhmal => "Deprecated",
        }
    }

    /// تحليل حرف عربي إلى فئة
    /// Parse an Arabic character into a category
    pub fn from_arabic_char(c: char) -> Option<Self> {
        match c {
            'ق' => Some(Self::Qiraa),
            'ب' => Some(Self::Binaa),
            'د' => Some(Self::Dalala),
            'ن' => Some(Self::Naw),
            'ص' => Some(Self::Sinf),
            'و' => Some(Self::Wahda),
            'ت' => Some(Self::Tawleed),
            'ح' => Some(Self::Tahdheer),
            'م' => Some(Self::Muhmal),
            _ => None,
        }
    }
}

/// رمز خطأ عربي - Arabic error code
///
/// Represents an error code in the format: `<category><4-digit-number>`
/// where number is in the range 0001-9999.
///
/// Example: د٠٣٠١ (category: Dalala, number: 301)
///
/// # Panics
///
/// `new()` panics if number is 0 or greater than 9999.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorCode {
    /// فئة الخطأ - Error category
    pub category: ErrorCategory,
    /// رقم الخطأ (0001-9999) - Error number (must be 1-9999)
    pub number: u16,
}

impl ErrorCode {
    /// إنشاء رمز خطأ جديد
    /// Create a new error code.
    ///
    /// # Panics
    ///
    /// Panics if `number` is 0 or greater than 9999.
    pub const fn new(category: ErrorCategory, number: u16) -> Self {
        assert!(
            number >= 1 && number <= 9999,
            "Error code number must be 1-9999"
        );
        Self { category, number }
    }

    /// الحصول على الرمز كنص عربي
    /// Get the code as Arabic string (e.g., "د٠٣٠١")
    pub fn to_arabic_string(&self) -> String {
        format!(
            "{}{}",
            self.category.arabic_char(),
            to_arabic_numerals(self.number)
        )
    }

    /// تحليل نص عربي إلى رمز خطأ
    /// Parse an Arabic string into an error code.
    ///
    /// Returns `None` if the string is invalid or the number is outside 1-9999.
    pub fn from_arabic_string(s: &str) -> Option<Self> {
        let mut chars = s.chars();
        let category_char = chars.next()?;
        let category = ErrorCategory::from_arabic_char(category_char)?;

        let number_str: String = chars.collect();
        let number = from_arabic_numerals(&number_str)?;

        // Validate number is in valid range (1-9999)
        if number == 0 || number > 9999 {
            return None;
        }

        Some(Self { category, number })
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_arabic_string())
    }
}

/// تحويل رقم إلى أرقام عربية-هندية (٤ خانات)
/// Convert a number to 4-digit Arabic-Indic numerals
pub fn to_arabic_numerals(n: u16) -> String {
    const ARABIC_DIGITS: [char; 10] = ['٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨', '٩'];

    format!("{:04}", n)
        .chars()
        .map(|c| ARABIC_DIGITS[c.to_digit(10).unwrap_or(0) as usize])
        .collect()
}

/// تحويل أرقام عربية-هندية إلى رقم
/// Convert Arabic-Indic numerals to a number
pub fn from_arabic_numerals(s: &str) -> Option<u16> {
    let mut result: u16 = 0;
    for c in s.chars() {
        let digit = match c {
            '٠' => 0,
            '١' => 1,
            '٢' => 2,
            '٣' => 3,
            '٤' => 4,
            '٥' => 5,
            '٦' => 6,
            '٧' => 7,
            '٨' => 8,
            '٩' => 9,
            '0'..='9' => c.to_digit(10)? as u16,
            _ => return None,
        };
        result = result.checked_mul(10)?.checked_add(digit)?;
    }
    Some(result)
}

// ============================================================================
// رموز أخطاء القراءة (ق) - Lexer Error Codes
// ============================================================================

/// ق٠٠٠١: حرف غير معروف
pub const ERR_UNKNOWN_CHARACTER: ErrorCode = ErrorCode::new(ErrorCategory::Qiraa, 1);

/// ق٠٠٠٢: نهاية ملف غير متوقعة
pub const ERR_UNEXPECTED_EOF: ErrorCode = ErrorCode::new(ErrorCategory::Qiraa, 2);

/// ق٠١٠١: صيغة عدد غير صحيحة
pub const ERR_INVALID_NUMBER_FORMAT: ErrorCode = ErrorCode::new(ErrorCategory::Qiraa, 101);

/// ق٠٢٠١: نص غير مغلق
pub const ERR_UNCLOSED_STRING: ErrorCode = ErrorCode::new(ErrorCategory::Qiraa, 201);

/// ق٠٣٠١: تعليق غير مغلق
pub const ERR_UNCLOSED_COMMENT: ErrorCode = ErrorCode::new(ErrorCategory::Qiraa, 301);

// ============================================================================
// رموز أخطاء البناء (ب) - Parser Error Codes
// ============================================================================

/// ب٠٠٠١: تعبير غير متوقع
pub const ERR_UNEXPECTED_EXPRESSION: ErrorCode = ErrorCode::new(ErrorCategory::Binaa, 1);

/// ب٠٠٠٢: رمز غير متوقع
pub const ERR_UNEXPECTED_TOKEN: ErrorCode = ErrorCode::new(ErrorCategory::Binaa, 2);

/// ب٠١٠١: متوقع فاصلة منقوطة
pub const ERR_EXPECTED_SEMICOLON: ErrorCode = ErrorCode::new(ErrorCategory::Binaa, 101);

/// ب٠٢٠١: متوقع اسم المتغير
pub const ERR_EXPECTED_VARIABLE_NAME: ErrorCode = ErrorCode::new(ErrorCategory::Binaa, 201);

/// ب٠٣٠١: متوقع اسم الدالة
pub const ERR_EXPECTED_FUNCTION_NAME: ErrorCode = ErrorCode::new(ErrorCategory::Binaa, 301);

/// ب٠٤٠١: متوقع اسم الصنف
pub const ERR_EXPECTED_CLASS_NAME: ErrorCode = ErrorCode::new(ErrorCategory::Binaa, 401);

// ============================================================================
// رموز أخطاء الدلالة (د) - Semantic Error Codes
// ============================================================================

/// د٠٠٠١: متغير غير معرف
pub const ERR_UNDEFINED_VARIABLE: ErrorCode = ErrorCode::new(ErrorCategory::Dalala, 1);

/// د٠٠٠٢: دالة غير معرفة
pub const ERR_UNDEFINED_FUNCTION: ErrorCode = ErrorCode::new(ErrorCategory::Dalala, 2);

/// د٠٠٠٣: صنف غير معرف
pub const ERR_UNDEFINED_CLASS: ErrorCode = ErrorCode::new(ErrorCategory::Dalala, 3);

/// د٠١٠١: إعادة تعريف متغير
pub const ERR_VARIABLE_REDEFINITION: ErrorCode = ErrorCode::new(ErrorCategory::Dalala, 101);

/// د٠١٠٢: تعيين قيمة لثابت
pub const ERR_CONST_ASSIGNMENT: ErrorCode = ErrorCode::new(ErrorCategory::Dalala, 102);

/// د٠٣٠١: استخدام 'أوقف' خارج حلقة
pub const ERR_BREAK_OUTSIDE_LOOP: ErrorCode = ErrorCode::new(ErrorCategory::Dalala, 301);

/// د٠٣٠٢: استخدام 'استمر' خارج حلقة
pub const ERR_CONTINUE_OUTSIDE_LOOP: ErrorCode = ErrorCode::new(ErrorCategory::Dalala, 302);

/// د٠٣٠٣: استخدام 'أرجع' خارج دالة
pub const ERR_RETURN_OUTSIDE_FUNCTION: ErrorCode = ErrorCode::new(ErrorCategory::Dalala, 303);

/// د٠٣٠٤: استخدام 'هذا' خارج صنف
pub const ERR_THIS_OUTSIDE_CLASS: ErrorCode = ErrorCode::new(ErrorCategory::Dalala, 304);

/// د٠٣٠٥: استخدام 'الأصل' خارج صنف
pub const ERR_SUPER_OUTSIDE_CLASS: ErrorCode = ErrorCode::new(ErrorCategory::Dalala, 305);

// ============================================================================
// رموز أخطاء الأنواع (ن) - Type Error Codes
// ============================================================================

/// ن٠٠٠١: عدم تطابق الأنواع
pub const ERR_TYPE_MISMATCH: ErrorCode = ErrorCode::new(ErrorCategory::Naw, 1);

/// ن٠٠٠٢: لا يمكن استنتاج النوع
pub const ERR_CANNOT_INFER_TYPE: ErrorCode = ErrorCode::new(ErrorCategory::Naw, 2);

/// ن٠١٠١: تحويل غير صحيح
pub const ERR_INVALID_CONVERSION: ErrorCode = ErrorCode::new(ErrorCategory::Naw, 101);

/// ن٠٢٠١: معامل نوع معمم غير محدد
pub const ERR_UNSPECIFIED_GENERIC: ErrorCode = ErrorCode::new(ErrorCategory::Naw, 201);

// ============================================================================
// رموز أخطاء الأصناف (ص) - Class Error Codes
// ============================================================================

/// ص٠٠٠١: صنف غير موجود
pub const ERR_CLASS_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Sinf, 1);

/// ص٠١٠١: وراثة دائرية
pub const ERR_CIRCULAR_INHERITANCE: ErrorCode = ErrorCode::new(ErrorCategory::Sinf, 101);

/// ص٠٢٠١: ميثاق غير منفذ
pub const ERR_INTERFACE_NOT_IMPLEMENTED: ErrorCode = ErrorCode::new(ErrorCategory::Sinf, 201);

/// ص٠٣٠١: خاصية غير موجودة
pub const ERR_PROPERTY_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Sinf, 301);

/// ص٠٣٠٢: دالة غير موجودة في الصنف
pub const ERR_METHOD_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Sinf, 302);

/// ص٠٤٠١: الوصول لعضو خاص
pub const ERR_PRIVATE_ACCESS: ErrorCode = ErrorCode::new(ErrorCategory::Sinf, 401);

/// ص٠٤٠٢: الوصول لعضو محمي
pub const ERR_PROTECTED_ACCESS: ErrorCode = ErrorCode::new(ErrorCategory::Sinf, 402);

// ============================================================================
// رموز أخطاء الوحدات (و) - Module Error Codes
// ============================================================================

/// و٠٠٠١: وحدة غير موجودة
pub const ERR_MODULE_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Wahda, 1);

/// و٠٠٠٢: عنصر غير مُصدَّر
pub const ERR_NOT_EXPORTED: ErrorCode = ErrorCode::new(ErrorCategory::Wahda, 2);

/// و٠١٠١: تصدير مكرر
pub const ERR_DUPLICATE_EXPORT: ErrorCode = ErrorCode::new(ErrorCategory::Wahda, 101);

/// و٠٣٠١: تبعية دائرية
pub const ERR_CIRCULAR_DEPENDENCY: ErrorCode = ErrorCode::new(ErrorCategory::Wahda, 301);

// ============================================================================
// رموز أخطاء التوليد (ت) - Codegen Error Codes
// ============================================================================

/// ت٠٠٠١: خطأ داخلي في LLVM
pub const ERR_LLVM_INTERNAL: ErrorCode = ErrorCode::new(ErrorCategory::Tawleed, 1);

/// ت٠١٠١: فشل الربط
pub const ERR_LINKING_FAILED: ErrorCode = ErrorCode::new(ErrorCategory::Tawleed, 101);

/// ت٠٢٠١: تعارض وضع نقطة الدخول (وضع السكربت ووضع البرنامج معاً)
/// Entry point mode conflict: Cannot have both top-level executable code (Script mode)
/// and دالة رئيسية() (Program mode) in the same file.
pub const ERR_ENTRY_POINT_CONFLICT: ErrorCode = ErrorCode::new(ErrorCategory::Tawleed, 201);

// ============================================================================
// رموز التحذيرات (ح) - Warning Codes
// ============================================================================

/// ح٠٠٠١: متغير غير مستخدم
pub const WARN_UNUSED_VARIABLE: ErrorCode = ErrorCode::new(ErrorCategory::Tahdheer, 1);

/// ح٠٠٠٢: دالة غير مستخدمة
pub const WARN_UNUSED_FUNCTION: ErrorCode = ErrorCode::new(ErrorCategory::Tahdheer, 2);

/// ح٠٠٠٣: استيراد غير مستخدم
pub const WARN_UNUSED_IMPORT: ErrorCode = ErrorCode::new(ErrorCategory::Tahdheer, 3);

// ============================================================================
// رموز الصيغ المهملة (م) - Deprecation Codes
// ============================================================================

/// م٠٠٠١: استخدام كلمة مفتاحية مهملة
pub const WARN_DEPRECATED_KEYWORD: ErrorCode = ErrorCode::new(ErrorCategory::Muhmal, 1);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(ERR_BREAK_OUTSIDE_LOOP.to_string(), "د٠٣٠١");
        assert_eq!(ERR_CONTINUE_OUTSIDE_LOOP.to_string(), "د٠٣٠٢");
        assert_eq!(ERR_RETURN_OUTSIDE_FUNCTION.to_string(), "د٠٣٠٣");
        assert_eq!(ERR_THIS_OUTSIDE_CLASS.to_string(), "د٠٣٠٤");
        assert_eq!(ERR_SUPER_OUTSIDE_CLASS.to_string(), "د٠٣٠٥");
    }

    #[test]
    fn test_error_code_categories() {
        assert_eq!(ERR_UNKNOWN_CHARACTER.to_string(), "ق٠٠٠١");
        assert_eq!(ERR_UNEXPECTED_TOKEN.to_string(), "ب٠٠٠٢");
        assert_eq!(ERR_UNDEFINED_VARIABLE.to_string(), "د٠٠٠١");
        assert_eq!(ERR_TYPE_MISMATCH.to_string(), "ن٠٠٠١");
        assert_eq!(ERR_CLASS_NOT_FOUND.to_string(), "ص٠٠٠١");
        assert_eq!(ERR_MODULE_NOT_FOUND.to_string(), "و٠٠٠١");
        assert_eq!(ERR_LLVM_INTERNAL.to_string(), "ت٠٠٠١");
        assert_eq!(ERR_ENTRY_POINT_CONFLICT.to_string(), "ت٠٢٠١");
    }

    #[test]
    fn test_arabic_numerals_conversion() {
        assert_eq!(to_arabic_numerals(0), "٠٠٠٠");
        assert_eq!(to_arabic_numerals(1), "٠٠٠١");
        assert_eq!(to_arabic_numerals(301), "٠٣٠١");
        assert_eq!(to_arabic_numerals(9999), "٩٩٩٩");
    }

    #[test]
    fn test_from_arabic_numerals() {
        assert_eq!(from_arabic_numerals("٠٠٠٠"), Some(0));
        assert_eq!(from_arabic_numerals("٠٠٠١"), Some(1));
        assert_eq!(from_arabic_numerals("٠٣٠١"), Some(301));
        assert_eq!(from_arabic_numerals("٩٩٩٩"), Some(9999));
        // Also accepts western digits
        assert_eq!(from_arabic_numerals("0301"), Some(301));
    }

    #[test]
    fn test_error_code_parsing() {
        let code = ErrorCode::from_arabic_string("د٠٣٠١");
        assert!(code.is_some());
        let code = code.unwrap();
        assert_eq!(code.category, ErrorCategory::Dalala);
        assert_eq!(code.number, 301);

        // Round-trip test
        assert_eq!(code.to_string(), "د٠٣٠١");
    }

    #[test]
    fn test_category_names() {
        assert_eq!(ErrorCategory::Qiraa.arabic_char(), 'ق');
        assert_eq!(ErrorCategory::Qiraa.arabic_name(), "قراءة");
        assert_eq!(ErrorCategory::Qiraa.english_name(), "Lexer");

        assert_eq!(ErrorCategory::Dalala.arabic_char(), 'د');
        assert_eq!(ErrorCategory::Dalala.arabic_name(), "دلالة");
        assert_eq!(ErrorCategory::Dalala.english_name(), "Semantic");
    }

    #[test]
    fn test_warning_codes() {
        assert_eq!(WARN_UNUSED_VARIABLE.to_string(), "ح٠٠٠١");
        assert_eq!(WARN_DEPRECATED_KEYWORD.to_string(), "م٠٠٠١");
    }

    #[test]
    fn test_error_code_boundary_values() {
        // Valid boundary: 1 (minimum)
        let code_min = ErrorCode::new(ErrorCategory::Dalala, 1);
        assert_eq!(code_min.to_string(), "د٠٠٠١");

        // Valid boundary: 9999 (maximum)
        let code_max = ErrorCode::new(ErrorCategory::Dalala, 9999);
        assert_eq!(code_max.to_string(), "د٩٩٩٩");
    }

    #[test]
    #[should_panic(expected = "Error code number must be 1-9999")]
    fn test_error_code_zero_panics() {
        let _ = ErrorCode::new(ErrorCategory::Dalala, 0);
    }

    #[test]
    #[should_panic(expected = "Error code number must be 1-9999")]
    fn test_error_code_over_9999_panics() {
        let _ = ErrorCode::new(ErrorCategory::Dalala, 10000);
    }

    #[test]
    fn test_from_arabic_string_rejects_invalid_numbers() {
        // Number 0 should be rejected
        assert!(ErrorCode::from_arabic_string("د٠٠٠٠").is_none());

        // Number > 9999 should be rejected
        assert!(ErrorCode::from_arabic_string("د١٠٠٠٠").is_none());

        // Valid numbers should work
        assert!(ErrorCode::from_arabic_string("د٠٠٠١").is_some());
        assert!(ErrorCode::from_arabic_string("د٩٩٩٩").is_some());
    }
}
