//! أخطاء تحليل صيغة الحزمة
//! Package format parsing errors

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub line: usize,
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

#[derive(Debug, Clone, PartialEq)]
pub enum FormatErrorKind {
    UnexpectedChar(char),
    UnexpectedEof,
    InvalidIndentation,
    InvalidValue(String),
    InvalidNumber(String),
    UnterminatedString,
    ExpectedColon,
    ExpectedValue,
    ExpectedKey,
    DuplicateKey(String),
    UnknownSection(String),
    MissingField(String),
    TypeError { expected: String, found: String },
}

#[derive(Debug, Clone)]
pub struct FormatError {
    pub kind: FormatErrorKind,
    pub location: Location,
    pub message: String,
}

impl FormatError {
    pub fn new(kind: FormatErrorKind, location: Location) -> Self {
        let message = match &kind {
            FormatErrorKind::UnexpectedChar(c) => format!("حرف غير متوقع: '{}'", c),
            FormatErrorKind::UnexpectedEof => "نهاية ملف غير متوقعة".to_string(),
            FormatErrorKind::InvalidIndentation => "مسافة بادئة غير صالحة".to_string(),
            FormatErrorKind::InvalidValue(v) => format!("قيمة غير صالحة: '{}'", v),
            FormatErrorKind::InvalidNumber(n) => format!("رقم غير صالح: '{}'", n),
            FormatErrorKind::UnterminatedString => "نص غير مغلق - متوقع علامة اقتباس".to_string(),
            FormatErrorKind::ExpectedColon => "متوقع ':' بعد المفتاح".to_string(),
            FormatErrorKind::ExpectedValue => "متوقع قيمة".to_string(),
            FormatErrorKind::ExpectedKey => "متوقع مفتاح".to_string(),
            FormatErrorKind::DuplicateKey(k) => format!("مفتاح مكرر: '{}'", k),
            FormatErrorKind::UnknownSection(s) => format!("قسم غير معروف: '{}'", s),
            FormatErrorKind::MissingField(f) => format!("حقل مفقود: '{}'", f),
            FormatErrorKind::TypeError { expected, found } => {
                format!("نوع غير صحيح: متوقع {} وجد {}", expected, found)
            }
        };

        Self {
            kind,
            location,
            message,
        }
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.location, self.message)
    }
}

impl std::error::Error for FormatError {}

pub type FormatResult<T> = Result<T, FormatError>;
