//! أنماط القيم لصيغة الحزمة
//! Value types for package format

use indexmap::IndexMap;
use std::fmt;

/// قيمة في صيغة الحزمة
/// A value in the package format
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// لا شيء / فارغ
    Null,
    /// قيمة منطقية (نعم/لا، صحيح/خطأ)
    Bool(bool),
    /// رقم (عدد صحيح أو عشري)
    Number(f64),
    /// نص
    String(String),
    /// قائمة من القيم
    Array(Vec<Value>),
    /// كائن (قاموس مرتب)
    Object(IndexMap<String, Value>),
}

impl Value {
    /// هل القيمة فارغة؟
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// هل القيمة منطقية؟
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// هل القيمة رقم؟
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    /// هل القيمة نص؟
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// هل القيمة قائمة؟
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// هل القيمة كائن؟
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }

    /// الحصول على القيمة كمنطقي
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// الحصول على القيمة كرقم
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// الحصول على القيمة كعدد صحيح
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(n) => Some(*n as i64),
            _ => None,
        }
    }

    /// الحصول على القيمة كنص
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// الحصول على القيمة كقائمة
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    /// الحصول على القيمة ككائن
    pub fn as_object(&self) -> Option<&IndexMap<String, Value>> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    /// الحصول على قيمة فرعية بالمفتاح
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(o) => o.get(key),
            _ => None,
        }
    }

    /// الحصول على قيمة فرعية بالفهرس
    pub fn get_index(&self, index: usize) -> Option<&Value> {
        match self {
            Value::Array(a) => a.get(index),
            _ => None,
        }
    }

    /// الحصول على اسم النوع بالعربية
    pub fn type_name_ar(&self) -> &'static str {
        match self {
            Value::Null => "فارغ",
            Value::Bool(_) => "منطقي",
            Value::Number(_) => "رقم",
            Value::String(_) => "نص",
            Value::Array(_) => "قائمة",
            Value::Object(_) => "كائن",
        }
    }

    /// الحصول على اسم النوع بالإنجليزية
    pub fn type_name_en(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "لا_شيء"),
            Value::Bool(true) => write!(f, "نعم"),
            Value::Bool(false) => write!(f, "لا"),
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Array(a) => {
                write!(f, "[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Object(o) => {
                write!(f, "{{")?;
                for (i, (k, v)) in o.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Number(n as f64)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n as f64)
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Number(n)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<Vec<Value>> for Value {
    fn from(a: Vec<Value>) -> Self {
        Value::Array(a)
    }
}

impl From<IndexMap<String, Value>> for Value {
    fn from(o: IndexMap<String, Value>) -> Self {
        Value::Object(o)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_types() {
        assert!(Value::Null.is_null());
        assert!(Value::Bool(true).is_bool());
        assert!(Value::Number(42.0).is_number());
        assert!(Value::String("مرحبا".to_string()).is_string());
        assert!(Value::Array(vec![]).is_array());
        assert!(Value::Object(IndexMap::new()).is_object());
    }

    #[test]
    fn test_value_as_methods() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Number(3.14).as_number(), Some(3.14));
        assert_eq!(Value::Number(42.0).as_i64(), Some(42));
        assert_eq!(Value::String("مرحبا".to_string()).as_str(), Some("مرحبا"));
    }

    #[test]
    fn test_value_display() {
        assert_eq!(format!("{}", Value::Null), "لا_شيء");
        assert_eq!(format!("{}", Value::Bool(true)), "نعم");
        assert_eq!(format!("{}", Value::Bool(false)), "لا");
        assert_eq!(format!("{}", Value::Number(42.0)), "42");
        assert_eq!(format!("{}", Value::Number(3.14)), "3.14");
        assert_eq!(
            format!("{}", Value::String("مرحبا".to_string())),
            "\"مرحبا\""
        );
    }

    #[test]
    fn test_value_type_names() {
        assert_eq!(Value::Null.type_name_ar(), "فارغ");
        assert_eq!(Value::Null.type_name_en(), "null");
        assert_eq!(Value::Bool(true).type_name_ar(), "منطقي");
        assert_eq!(Value::Number(1.0).type_name_ar(), "رقم");
    }
}
