//! Method Resolver - Resolves method calls and member access
//!
//! This module handles:
//! - Instance method lookup
//! - Static method lookup
//! - Super method calls
//! - Field access resolution
//! - Property getter/setter resolution

use super::class_resolver::{ClassResolver, FieldInfo, MethodInfo};
use super::types::Type;
use crate::error::codes::ERR_METHOD_NOT_FOUND;
use crate::error::{Diagnostic, Span};

#[derive(Debug, Clone)]
pub enum MemberResolution {
    /// A field from a class, with the name of the class that defines it.
    Field {
        field: FieldInfo,
        defining_class: String,
    },
    /// A method from a class, with the name of the class that defines it.
    Method {
        method: MethodInfo,
        defining_class: String,
    },
    BuiltinProperty {
        name: String,
        ty: Type,
    },
    NotFound,
}

#[derive(Debug, Clone)]
pub struct MethodCallResolution {
    pub method: MethodInfo,
    pub is_virtual: bool,
    pub defining_class: String,
    pub vtable_index: Option<usize>,
}

pub struct MethodResolver<'a> {
    class_resolver: &'a ClassResolver,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> MethodResolver<'a> {
    pub fn new(class_resolver: &'a ClassResolver) -> Self {
        Self {
            class_resolver,
            diagnostics: Vec::new(),
        }
    }

    pub fn resolve_member(&mut self, object_type: &Type, member_name: &str) -> MemberResolution {
        match object_type {
            Type::Class(class_name) => self.resolve_class_member(class_name, member_name),
            Type::Array(_) => self.resolve_array_member(member_name),
            Type::String => self.resolve_string_member(member_name),
            Type::Map(_, _) => self.resolve_map_member(member_name),
            Type::Any => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Any,
            },
            _ => MemberResolution::NotFound,
        }
    }

    fn resolve_class_member(&self, class_name: &str, member_name: &str) -> MemberResolution {
        if let Some(class) = self.class_resolver.get_class(class_name) {
            if let Some((method, defining_class)) =
                class.get_method_with_defining_class(member_name, self.class_resolver)
            {
                return MemberResolution::Method {
                    method: method.clone(),
                    defining_class: defining_class.to_string(),
                };
            }

            if let Some((field, defining_class)) =
                class.get_field_with_defining_class(member_name, self.class_resolver)
            {
                return MemberResolution::Field {
                    field: field.clone(),
                    defining_class: defining_class.to_string(),
                };
            }
        }

        MemberResolution::NotFound
    }

    fn resolve_array_member(&self, member_name: &str) -> MemberResolution {
        match member_name {
            "طول" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Int,
            },
            "ألحق" | "أضف" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Function {
                    params: vec![Type::Any],
                    return_type: Box::new(Type::Void),
                },
            },
            "احذف" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Function {
                    params: vec![],
                    return_type: Box::new(Type::Any),
                },
            },
            "اول" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Any,
            },
            "اخر" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Any,
            },
            _ => MemberResolution::NotFound,
        }
    }

    fn resolve_string_member(&self, member_name: &str) -> MemberResolution {
        match member_name {
            "طول" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Int,
            },
            "قص" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Function {
                    params: vec![Type::Int, Type::Int],
                    return_type: Box::new(Type::String),
                },
            },
            "كبير" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Function {
                    params: vec![],
                    return_type: Box::new(Type::String),
                },
            },
            "صغير" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Function {
                    params: vec![],
                    return_type: Box::new(Type::String),
                },
            },
            "استبدل" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Function {
                    params: vec![Type::String, Type::String],
                    return_type: Box::new(Type::String),
                },
            },
            "قسم" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Function {
                    params: vec![Type::String],
                    return_type: Box::new(Type::Array(Box::new(Type::String))),
                },
            },
            "يحتوي" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Function {
                    params: vec![Type::String],
                    return_type: Box::new(Type::Bool),
                },
            },
            "يبدأ_بـ" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Function {
                    params: vec![Type::String],
                    return_type: Box::new(Type::Bool),
                },
            },
            "ينتهي_بـ" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Function {
                    params: vec![Type::String],
                    return_type: Box::new(Type::Bool),
                },
            },
            _ => MemberResolution::NotFound,
        }
    }

    fn resolve_map_member(&self, member_name: &str) -> MemberResolution {
        match member_name {
            "طول" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Int,
            },
            "مفاتيح" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Array(Box::new(Type::Any)),
            },
            "قيم" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Array(Box::new(Type::Any)),
            },
            "يحتوي" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Function {
                    params: vec![Type::Any],
                    return_type: Box::new(Type::Bool),
                },
            },
            "احذف" => MemberResolution::BuiltinProperty {
                name: member_name.to_string(),
                ty: Type::Function {
                    params: vec![Type::Any],
                    return_type: Box::new(Type::Bool),
                },
            },
            _ => MemberResolution::NotFound,
        }
    }

    pub fn resolve_method_call(
        &mut self,
        object_type: &Type,
        method_name: &str,
        span: Span,
    ) -> Option<MethodCallResolution> {
        match object_type {
            Type::Class(class_name) => {
                self.resolve_class_method_call(class_name, method_name, span)
            }
            Type::Any => Some(MethodCallResolution {
                method: MethodInfo {
                    name: method_name.to_string(),
                    params: vec![],
                    return_type: Type::Any,
                    visibility: crate::parser::Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
                is_virtual: false,
                defining_class: String::new(),
                vtable_index: None,
            }),
            _ => None,
        }
    }

    fn resolve_class_method_call(
        &mut self,
        class_name: &str,
        method_name: &str,
        span: Span,
    ) -> Option<MethodCallResolution> {
        if let Some(class) = self.class_resolver.get_class(class_name) {
            if let Some(method) = class.get_method(method_name, self.class_resolver) {
                let defining_class = self.find_defining_class(class_name, method_name);

                return Some(MethodCallResolution {
                    method: method.clone(),
                    is_virtual: !method.is_static && method.vtable_index.is_some(),
                    defining_class,
                    vtable_index: method.vtable_index,
                });
            }
        }

        self.diagnostics.push(
            Diagnostic::error(
                format!(
                    "Method '{}' not found on type '{}'",
                    method_name, class_name
                ),
                format!(
                    "الدالة '{}' غير موجودة في النوع '{}'",
                    method_name, class_name
                ),
                span,
            )
            .with_code(ERR_METHOD_NOT_FOUND.to_string()),
        );

        None
    }

    fn find_defining_class(&self, class_name: &str, method_name: &str) -> String {
        if let Some(class) = self.class_resolver.get_class(class_name) {
            if class.has_own_method(method_name) {
                return class_name.to_string();
            }

            if let Some(parent_name) = &class.parent {
                return self.find_defining_class(parent_name, method_name);
            }
        }

        class_name.to_string()
    }

    pub fn resolve_super_call(
        &mut self,
        current_class: &str,
        method_name: &str,
        span: Span,
    ) -> Option<MethodCallResolution> {
        if let Some(class) = self.class_resolver.get_class(current_class) {
            if let Some(parent_name) = &class.parent {
                return self.resolve_class_method_call(parent_name, method_name, span);
            } else {
                self.diagnostics.push(Diagnostic::error(
                    format!("Class '{}' has no superclass", current_class),
                    format!("الصنف '{}' ليس له صنف أب", current_class),
                    span,
                ));
            }
        }

        None
    }

    pub fn check_method_args(
        &mut self,
        method: &MethodInfo,
        arg_types: &[Type],
        span: Span,
    ) -> bool {
        if arg_types.len() != method.params.len() {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "Method '{}' expects {} arguments, got {}",
                    method.name,
                    method.params.len(),
                    arg_types.len()
                ),
                format!(
                    "الدالة '{}' تتوقع {} معاملات، وُجد {}",
                    method.name,
                    method.params.len(),
                    arg_types.len()
                ),
                span,
            ));
            return false;
        }

        for (i, (arg_type, (_, param_type))) in
            arg_types.iter().zip(method.params.iter()).enumerate()
        {
            if !arg_type.is_compatible_with(param_type) {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "Argument {} has wrong type: expected {}, got {}",
                        i + 1,
                        param_type,
                        arg_type
                    ),
                    format!(
                        "المعامل {} نوعه خاطئ: متوقع {}، وُجد {}",
                        i + 1,
                        param_type.arabic_name(),
                        arg_type.arabic_name()
                    ),
                    span,
                ));
                return false;
            }
        }

        true
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Span;
    use crate::parser::Visibility;

    #[test]
    fn test_array_member_resolution() {
        let resolver = ClassResolver::new();
        let mut method_resolver = MethodResolver::new(&resolver);

        let array_type = Type::Array(Box::new(Type::Int));

        match method_resolver.resolve_member(&array_type, "طول") {
            MemberResolution::BuiltinProperty { ty, .. } => {
                assert_eq!(ty, Type::Int);
            }
            _ => panic!("Expected BuiltinProperty"),
        }
    }

    #[test]
    fn test_string_member_resolution() {
        let resolver = ClassResolver::new();
        let mut method_resolver = MethodResolver::new(&resolver);

        match method_resolver.resolve_member(&Type::String, "طول") {
            MemberResolution::BuiltinProperty { ty, .. } => {
                assert_eq!(ty, Type::Int);
            }
            _ => panic!("Expected BuiltinProperty"),
        }
    }

    #[test]
    fn test_class_method_resolution() {
        let mut class_resolver = ClassResolver::new();
        class_resolver.register_class("شخص", &[], None, &[], Span::empty());

        if let Some(class) = class_resolver.get_class_mut("شخص") {
            class.methods.insert(
                "تحية".to_string(),
                MethodInfo {
                    name: "تحية".to_string(),
                    params: vec![],
                    return_type: Type::String,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: Some(0),
                },
            );
        }

        class_resolver.build_vtables();

        let mut method_resolver = MethodResolver::new(&class_resolver);
        let person_type = Type::Class("شخص".to_string());

        match method_resolver.resolve_member(&person_type, "تحية") {
            MemberResolution::Method { method, .. } => {
                assert_eq!(method.name, "تحية");
                assert_eq!(method.return_type, Type::String);
            }
            _ => panic!("Expected Method"),
        }
    }
}
