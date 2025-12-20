//! Runtime value representation for the interpreter.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::ir::ClassId;

/// Runtime value in the interpreter.
///
/// Values are immutable by default but arrays and objects use interior
/// mutability through `Rc<RefCell<...>>` for efficient sharing and mutation.
#[derive(Clone)]
pub enum Value {
    /// Null value (عدم)
    Null,

    /// Boolean value (منطقي)
    Bool(bool),

    /// 64-bit signed integer (عدد)
    Int(i64),

    /// 64-bit floating point (عدد_عشري)
    Float(f64),

    /// String value (نص)
    String(Rc<String>),

    /// Array value (مصفوفة)
    Array(Rc<RefCell<Vec<Value>>>),

    /// Object instance (كائن)
    Object(Rc<RefCell<TrqObject>>),

    /// Function reference (for first-class functions)
    Function(String),

    /// Pointer to a memory location (for Alloca/Load/Store)
    Ptr(Rc<RefCell<Value>>),
}

/// Object instance with class information and fields.
#[derive(Debug, Clone)]
pub struct TrqObject {
    /// Class identifier
    pub class_id: ClassId,
    /// Field values (by field name)
    pub fields: HashMap<String, Value>,
}

impl TrqObject {
    /// Create a new object of the given class.
    pub fn new(class_id: ClassId) -> Self {
        Self {
            class_id,
            fields: HashMap::new(),
        }
    }

    /// Get a field value.
    pub fn get_field(&self, name: &str) -> Option<&Value> {
        self.fields.get(name)
    }

    /// Set a field value.
    pub fn set_field(&mut self, name: String, value: Value) {
        self.fields.insert(name, value);
    }
}

impl Value {
    /// Create a null value.
    pub fn null() -> Self {
        Value::Null
    }

    /// Create a boolean value.
    pub fn bool(b: bool) -> Self {
        Value::Bool(b)
    }

    /// Create an integer value.
    pub fn int(i: i64) -> Self {
        Value::Int(i)
    }

    /// Create a float value.
    pub fn float(f: f64) -> Self {
        Value::Float(f)
    }

    /// Create a string value.
    pub fn string(s: impl Into<String>) -> Self {
        Value::String(Rc::new(s.into()))
    }

    /// Create an empty array.
    pub fn array() -> Self {
        Value::Array(Rc::new(RefCell::new(Vec::new())))
    }

    /// Create an array from a vector of values.
    pub fn array_from(values: Vec<Value>) -> Self {
        Value::Array(Rc::new(RefCell::new(values)))
    }

    /// Create a new object of the given class.
    pub fn object(class_id: ClassId) -> Self {
        Value::Object(Rc::new(RefCell::new(TrqObject::new(class_id))))
    }

    /// Create a pointer to a value.
    pub fn ptr(value: Value) -> Self {
        Value::Ptr(Rc::new(RefCell::new(value)))
    }

    /// Check if this value is truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.borrow().is_empty(),
            Value::Object(_) => true,
            Value::Function(_) => true,
            Value::Ptr(_) => true,
        }
    }

    /// Get the type name of this value (English).
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Function(_) => "function",
            Value::Ptr(_) => "ptr",
        }
    }

    /// Get the type name of this value (Arabic).
    pub fn type_name_ar(&self) -> &'static str {
        match self {
            Value::Null => "عدم",
            Value::Bool(_) => "منطقي",
            Value::Int(_) => "عدد",
            Value::Float(_) => "عدد_عشري",
            Value::String(_) => "نص",
            Value::Array(_) => "مصفوفة",
            Value::Object(_) => "كائن",
            Value::Function(_) => "دالة",
            Value::Ptr(_) => "مؤشر",
        }
    }

    /// Try to get as an integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as a float.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Try to get as a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get as a string reference.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Convert this value to a string representation.
    pub fn to_display_string(&self) -> String {
        match self {
            Value::Null => "عدم".to_string(),
            Value::Bool(true) => "صحيح".to_string(),
            Value::Bool(false) => "خطأ".to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{:.1}", f)
                } else {
                    format!("{}", f)
                }
            }
            Value::String(s) => s.to_string(),
            Value::Array(arr) => {
                let items: Vec<String> =
                    arr.borrow().iter().map(|v| v.to_display_string()).collect();
                format!("[{}]", items.join("، "))
            }
            Value::Object(obj) => {
                let obj = obj.borrow();
                format!("<{}>", obj.class_id.0)
            }
            Value::Function(name) => format!("<دالة {}>", name),
            Value::Ptr(_) => "<مؤشر>".to_string(),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "Null"),
            Value::Bool(b) => write!(f, "Bool({})", b),
            Value::Int(i) => write!(f, "Int({})", i),
            Value::Float(fl) => write!(f, "Float({})", fl),
            Value::String(s) => write!(f, "String({:?})", s),
            Value::Array(arr) => write!(f, "Array({:?})", arr.borrow()),
            Value::Object(obj) => write!(f, "Object({:?})", obj.borrow()),
            Value::Function(name) => write!(f, "Function({})", name),
            Value::Ptr(_) => write!(f, "Ptr(...)"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::Function(a), Value::Function(b)) => a == b,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_types() {
        assert_eq!(Value::null().type_name(), "null");
        assert_eq!(Value::bool(true).type_name(), "bool");
        assert_eq!(Value::int(42).type_name(), "int");
        assert_eq!(Value::float(3.14).type_name(), "float");
        assert_eq!(Value::string("hello").type_name(), "string");
        assert_eq!(Value::array().type_name(), "array");
    }

    #[test]
    fn test_truthiness() {
        assert!(!Value::null().is_truthy());
        assert!(!Value::bool(false).is_truthy());
        assert!(Value::bool(true).is_truthy());
        assert!(!Value::int(0).is_truthy());
        assert!(Value::int(1).is_truthy());
        assert!(Value::int(-1).is_truthy());
        assert!(!Value::string("").is_truthy());
        assert!(Value::string("hello").is_truthy());
    }

    #[test]
    fn test_display_string() {
        assert_eq!(Value::null().to_display_string(), "عدم");
        assert_eq!(Value::bool(true).to_display_string(), "صحيح");
        assert_eq!(Value::bool(false).to_display_string(), "خطأ");
        assert_eq!(Value::int(42).to_display_string(), "42");
        assert_eq!(Value::float(3.14).to_display_string(), "3.14");
        assert_eq!(Value::string("مرحبا").to_display_string(), "مرحبا");
    }

    #[test]
    fn test_equality() {
        assert_eq!(Value::int(42), Value::int(42));
        assert_ne!(Value::int(42), Value::int(43));
        assert_eq!(Value::string("hello"), Value::string("hello"));
        // Int and Float comparison
        assert_eq!(Value::int(42), Value::float(42.0));
    }
}
