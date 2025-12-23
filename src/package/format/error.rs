//! أخطاء تحليل صيغة الحزمة
//! Package format parsing errors

use std::fmt;

/// موقع في الملف المصدري
/// Location in source file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    /// رقم السطر (يبدأ من 1)
    pub line: usize,
    /// رقم العمود (يبدأ من 1)
    pub column: usize,
}

impl Location {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// أنواع أخطاء التحليل
/// Parse error types
#[derive(Debug, Clone, PartialEq)]
pub enum FormatErrorKind {
    /// حرف غير متوقع
    UnexpectedChar(char),
    /// نهاية ملف غير متوقعة
    UnexpectedEof,
    /// مسافة بادئة غير صالحة
    InvalidIndentation,
    /// قيمة غير صالحة
    InvalidValue(String),
    /// رقم غير صالح
    InvalidNumber(String),
    /// نص غير مغلق
    UnterminatedString,
    /// متوقع نقطتان
    ExpectedColon,
    /// متوقع قيمة
    ExpectedValue,
    /// متوقع مفتاح
    ExpectedKey,
    /// مفتاح مكرر
    DuplicateKey(String),
    /// قسم غير معروف
    UnknownSection(String),
    /// حقل مفقود
    MissingField(String),
    /// نوع غير صحيح
    TypeError { expected: String, found: String },
}

/// خطأ تحليل صيغة الحزمة
/// Package format parse error
#[derive(Debug, Clone)]
pub struct FormatError {
    /// نوع الخطأ
    pub kind: FormatErrorKind,
    /// الموقع في الملف
    pub location: Location,
    /// الرسالة بالعربية
    pub message_ar: String,
    /// الرسالة بالإنجليزية
    pub message_en: String,
}

impl FormatError {
    pub fn new(kind: FormatErrorKind, location: Location) -> Self {
        let (message_ar, message_en) = match &kind {
            FormatErrorKind::UnexpectedChar(c) => (
                format!("حرف غير متوقع: '{}'", c),
                format!("Unexpected character: '{}'", c),
            ),
            FormatErrorKind::UnexpectedEof => (
                "نهاية ملف غير متوقعة".to_string(),
                "Unexpected end of file".to_string(),
            ),
            FormatErrorKind::InvalidIndentation => (
                "مسافة بادئة غير صالحة".to_string(),
                "Invalid indentation".to_string(),
            ),
            FormatErrorKind::InvalidValue(v) => (
                format!("قيمة غير صالحة: '{}'", v),
                format!("Invalid value: '{}'", v),
            ),
            FormatErrorKind::InvalidNumber(n) => (
                format!("رقم غير صالح: '{}'", n),
                format!("Invalid number: '{}'", n),
            ),
            FormatErrorKind::UnterminatedString => (
                "نص غير مغلق - متوقع علامة اقتباس".to_string(),
                "Unterminated string - expected closing quote".to_string(),
            ),
            FormatErrorKind::ExpectedColon => (
                "متوقع ':' بعد المفتاح".to_string(),
                "Expected ':' after key".to_string(),
            ),
            FormatErrorKind::ExpectedValue => {
                ("متوقع قيمة".to_string(), "Expected value".to_string())
            }
            FormatErrorKind::ExpectedKey => ("متوقع مفتاح".to_string(), "Expected key".to_string()),
            FormatErrorKind::DuplicateKey(k) => (
                format!("مفتاح مكرر: '{}'", k),
                format!("Duplicate key: '{}'", k),
            ),
            FormatErrorKind::UnknownSection(s) => (
                format!("قسم غير معروف: '{}'", s),
                format!("Unknown section: '{}'", s),
            ),
            FormatErrorKind::MissingField(f) => (
                format!("حقل مفقود: '{}'", f),
                format!("Missing field: '{}'", f),
            ),
            FormatErrorKind::TypeError { expected, found } => (
                format!("نوع غير صحيح: متوقع {} وجد {}", expected, found),
                format!("Type error: expected {} found {}", expected, found),
            ),
        };

        Self {
            kind,
            location,
            message_ar,
            message_en,
        }
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} / {}",
            self.location, self.message_ar, self.message_en
        )
    }
}

impl std::error::Error for FormatError {}

/// نتيجة تحليل الصيغة
pub type FormatResult<T> = Result<T, FormatError>;
