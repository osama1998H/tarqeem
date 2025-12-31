//! Type system for Tarqeem

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Void,
    Null,

    Array(Box<Type>),

    Map(Box<Type>, Box<Type>),

    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },

    Class(String),

    Interface(String),

    Enum(String),

    Generic(String),

    Optional(Box<Type>),

    Any,

    Never,

    Unknown,

    Error,
}

impl Type {
    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }

    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            Type::Int | Type::Float | Type::String | Type::Bool | Type::Null | Type::Void
        )
    }

    pub fn is_compatible_with(&self, other: &Type) -> bool {
        match (self, other) {
            (a, b) if a == b => true,

            (Type::Any, _) | (_, Type::Any) => true,

            (Type::Unknown, _) | (_, Type::Unknown) => true,

            (Type::Int, Type::Float) => true,

            (Type::Null, Type::Optional(_)) | (Type::Optional(_), Type::Null) => true,

            (t, Type::Optional(inner)) | (Type::Optional(inner), t) => t.is_compatible_with(inner),

            (Type::Array(a), Type::Array(b)) => a.is_compatible_with(b),

            (Type::Map(ak, av), Type::Map(bk, bv)) => {
                ak.is_compatible_with(bk) && av.is_compatible_with(bv)
            }

            (
                Type::Function {
                    params: p1,
                    return_type: r1,
                },
                Type::Function {
                    params: p2,
                    return_type: r2,
                },
            ) => {
                p1.len() == p2.len()
                    && p1
                        .iter()
                        .zip(p2.iter())
                        .all(|(a, b)| a.is_compatible_with(b))
                    && r1.is_compatible_with(r2)
            }

            _ => false,
        }
    }

    pub fn binary_result_type(&self, op: &str, other: &Type) -> Option<Type> {
        match (self, op, other) {
            (Type::Int, "+", Type::Int)
            | (Type::Int, "-", Type::Int)
            | (Type::Int, "*", Type::Int)
            | (Type::Int, "/", Type::Int)
            | (Type::Int, "%", Type::Int) => Some(Type::Int),

            (Type::Float, "+", Type::Float)
            | (Type::Float, "-", Type::Float)
            | (Type::Float, "*", Type::Float)
            | (Type::Float, "/", Type::Float) => Some(Type::Float),

            (Type::Int, "+", Type::Float)
            | (Type::Float, "+", Type::Int)
            | (Type::Int, "-", Type::Float)
            | (Type::Float, "-", Type::Int)
            | (Type::Int, "*", Type::Float)
            | (Type::Float, "*", Type::Int)
            | (Type::Int, "/", Type::Float)
            | (Type::Float, "/", Type::Int) => Some(Type::Float),

            (Type::Int, "**", Type::Int) => Some(Type::Int),
            (Type::Float, "**", _) | (_, "**", Type::Float) => Some(Type::Float),

            (Type::String, "+", Type::String) => Some(Type::String),
            (Type::String, "+", Type::Int) => Some(Type::String),
            (Type::String, "+", Type::Float) => Some(Type::String),
            (Type::String, "+", Type::Bool) => Some(Type::String),
            (Type::Int, "+", Type::String) => Some(Type::String),
            (Type::Float, "+", Type::String) => Some(Type::String),
            (Type::Bool, "+", Type::String) => Some(Type::String),

            (Type::Int, "<", Type::Int)
            | (Type::Int, "<=", Type::Int)
            | (Type::Int, ">", Type::Int)
            | (Type::Int, ">=", Type::Int)
            | (Type::Float, "<", Type::Float)
            | (Type::Float, "<=", Type::Float)
            | (Type::Float, ">", Type::Float)
            | (Type::Float, ">=", Type::Float)
            | (Type::String, "<", Type::String)
            | (Type::String, "<=", Type::String)
            | (Type::String, ">", Type::String)
            | (Type::String, ">=", Type::String) => Some(Type::Bool),

            (a, "==", b) | (a, "!=", b) if a.is_compatible_with(b) => Some(Type::Bool),

            (Type::Bool, "&&", Type::Bool) | (Type::Bool, "||", Type::Bool) => Some(Type::Bool),

            _ => None,
        }
    }

    pub fn unary_result_type(&self, op: &str) -> Option<Type> {
        match (self, op) {
            (Type::Int, "-") => Some(Type::Int),
            (Type::Float, "-") => Some(Type::Float),
            (Type::Bool, "!") => Some(Type::Bool),
            (Type::Int, "++") | (Type::Int, "--") => Some(Type::Int),
            _ => None,
        }
    }

    pub fn arabic_name(&self) -> String {
        match self {
            Type::Int => "عدد".to_string(),
            Type::Float => "عدد_عشري".to_string(),
            Type::String => "نص".to_string(),
            Type::Bool => "منطقي".to_string(),
            Type::Void => "(لا_إرجاع)".to_string(), // Internal: functions default to no return
            Type::Null => "لا_شيء".to_string(),
            Type::Array(inner) => format!("مصفوفة<{}>", inner.arabic_name()),
            Type::Map(k, v) => format!("قاموس<{}، {}>", k.arabic_name(), v.arabic_name()),
            Type::Function {
                params,
                return_type,
            } => {
                let params_str: Vec<_> = params.iter().map(|p| p.arabic_name()).collect();
                format!(
                    "({}) -> {}",
                    params_str.join("، "),
                    return_type.arabic_name()
                )
            }
            Type::Class(name) => name.clone(),
            Type::Interface(name) => name.clone(),
            Type::Enum(name) => name.clone(),
            Type::Generic(name) => name.clone(),
            Type::Optional(inner) => format!("{}?", inner.arabic_name()),
            Type::Any => "أي".to_string(),
            Type::Never => "أبداً".to_string(),
            Type::Unknown => "مجهول".to_string(),
            Type::Error => "خطأ".to_string(),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::String => write!(f, "string"),
            Type::Bool => write!(f, "bool"),
            Type::Void => write!(f, "void"),
            Type::Null => write!(f, "null"),
            Type::Array(inner) => write!(f, "array<{}>", inner),
            Type::Map(k, v) => write!(f, "map<{}, {}>", k, v),
            Type::Function {
                params,
                return_type,
            } => {
                let params_str: Vec<_> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "({}) -> {}", params_str.join(", "), return_type)
            }
            Type::Class(name) => write!(f, "{}", name),
            Type::Interface(name) => write!(f, "{}", name),
            Type::Enum(name) => write!(f, "{}", name),
            Type::Generic(name) => write!(f, "{}", name),
            Type::Optional(inner) => write!(f, "{}?", inner),
            Type::Any => write!(f, "any"),
            Type::Never => write!(f, "never"),
            Type::Unknown => write!(f, "unknown"),
            Type::Error => write!(f, "error"),
        }
    }
}

pub fn parse_type_name(name: &str) -> Type {
    match name {
        "عدد" | "int" => Type::Int,
        "عدد_عشري" | "float" => Type::Float,
        "نص" | "string" => Type::String,
        "منطقي" | "bool" => Type::Bool,
        "void" => Type::Void, // فراغ eliminated - functions default to no return
        "لا_شيء" | "null" | "none" => Type::Null,
        "أي" | "اي" | "any" => Type::Any,
        _ => Type::Class(name.to_string()),
    }
}
