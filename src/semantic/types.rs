//! Type system for Tarqeem

use super::class_resolver::ClassResolver;
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

    /// A namespace bound by `استورد * كـ اسم`, carrying the specifier written
    /// in the `من` clause. The specifier — not the alias — is the key, because
    /// that is what identifies the module whose exports the alias stands for.
    Module(String),

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

    /// Two-argument-order type compatibility, shared by `is_compatible_with`
    /// (no subtyping — used for `==`, override variance, generic
    /// constraints, and every other pre-existing call site) and
    /// `is_assignable` (adds class subtyping, for assignment-position checks
    /// only). Keeping one recursive body means a fix to Optional/Array/Map/
    /// Function handling can't silently drift between the two callers.
    fn compat(&self, other: &Type, resolver: Option<&ClassResolver>) -> bool {
        match (self, other) {
            (a, b) if a == b => true,

            (Type::Any, _) | (_, Type::Any) => true,

            (Type::Unknown, _) | (_, Type::Unknown) => true,

            (Type::Int, Type::Float) => true,

            (Type::Null, Type::Optional(_)) | (Type::Optional(_), Type::Null) => true,

            // Kept as two arms, not one `(t, Optional(inner)) | (Optional(inner), t) => t.compat(inner, ...)`:
            // `compat` is directional (e.g. `(Int, Float) => true` but not the
            // reverse), so collapsing both orderings onto a single `t.compat(inner)`
            // call would silently swap which side is the value and which is the
            // slot whenever `self` (not `other`) is the Optional-wrapped one.
            (t, Type::Optional(inner)) => t.compat(inner, resolver),
            (Type::Optional(inner), t) => inner.compat(t, resolver),

            (Type::Array(a), Type::Array(b)) => a.compat(b, resolver),

            (Type::Map(ak, av), Type::Map(bk, bv)) => {
                ak.compat(bk, resolver) && av.compat(bv, resolver)
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
                    && p1.iter().zip(p2.iter()).all(|(a, b)| a.compat(b, resolver))
                    && r1.compat(r2, resolver)
            }

            // Upcast: a subclass value may be stored where its ancestor is
            // expected (issue #184). Interface-typed slots are deliberately
            // excluded: type annotations resolve `ميثاق` names to
            // `Type::Class`, not `Type::Interface` (see `parse_type_name`),
            // and member resolution through an interface name doesn't yet
            // type-check correctly, so allowing it here would trade a
            // compile error for a runtime crash. Add an
            // `implements_interface` arm only after that is fixed.
            (Type::Class(value_class), Type::Class(slot_class)) => {
                resolver.is_some_and(|r| r.is_subclass(value_class, slot_class))
            }

            _ => false,
        }
    }

    /// Value-to-value compatibility with no notion of class hierarchy —
    /// the pre-#184 semantics, unchanged. Used for `==`/`!=` operands,
    /// override parameter/return variance, generic constraints, and any
    /// other check that isn't deciding whether a value may be stored in a
    /// particular slot.
    pub fn is_compatible_with(&self, other: &Type) -> bool {
        self.compat(other, None)
    }

    /// Assignment-position compatibility: like `is_compatible_with`, plus
    /// upcasting a class to one of its ancestors. Use this (via
    /// `Analyzer::is_assignable`) for variable initialization, assignment,
    /// call/constructor arguments, and return values — not for `==`,
    /// override checks, or generic constraints, which must keep exact
    /// semantics.
    pub(crate) fn is_assignable(&self, slot: &Type, resolver: &ClassResolver) -> bool {
        self.compat(slot, Some(resolver))
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

            // Untyped lambda/function params resolve to `أي` (LANGUAGE_SPEC
            // §8.3); without these arms, spec-legal code like `(أ، ب) => أ + ب`
            // could never type-check its own body. Deliberately narrow: the
            // *other* operand must be something the operator could plausibly
            // accept, so `أي` stays an escape hatch for unknown-but-valid
            // values instead of switching off arithmetic checking wholesale
            // (e.g. `س ** "نص"` must still be rejected).
            (Type::Any, "+", other) | (other, "+", Type::Any)
                if matches!(
                    other,
                    Type::Int | Type::Float | Type::String | Type::Bool | Type::Any
                ) =>
            {
                Some(Type::Any)
            }

            (Type::Any, "-" | "*" | "/" | "%" | "**", other)
            | (other, "-" | "*" | "/" | "%" | "**", Type::Any)
                if matches!(other, Type::Int | Type::Float | Type::Any) =>
            {
                Some(Type::Any)
            }

            (Type::Any, "<" | "<=" | ">" | ">=", other)
            | (other, "<" | "<=" | ">" | ">=", Type::Any)
                if matches!(other, Type::Int | Type::Float | Type::String | Type::Any) =>
            {
                Some(Type::Bool)
            }

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
            Type::Module(specifier) => format!("وحدة '{}'", specifier),
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
            Type::Module(specifier) => write!(f, "module {}", specifier),
            Type::Any => write!(f, "any"),
            Type::Never => write!(f, "never"),
            Type::Unknown => write!(f, "unknown"),
            Type::Error => write!(f, "error"),
        }
    }
}

pub fn parse_type_name(name: &str) -> Type {
    match name {
        "عدد" => Type::Int,
        "عدد_عشري" => Type::Float,
        "نص" => Type::String,
        "منطقي" => Type::Bool,
        "لا_شيء" => Type::Null,
        "أي" | "اي" => Type::Any,
        _ => Type::Class(name.to_string()),
    }
}
