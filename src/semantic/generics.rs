//! Generic Type Support
//!
//! This module handles generic type parameters and their resolution.
//! Tarqeem uses a monomorphization approach where generic types are
//! specialized at compile time.
//!
//! ## Example
//! ```tarqeem
//! صنف قائمة<ن> {
//!     خاص عناصر: مصفوفة<ن>
//!     عام دالة أضف(عنصر: ن) { ... }
//! }
//!
//! متغير أرقام = جديد قائمة<عدد>()
//! ```

use super::types::Type;
use crate::error::{Diagnostic, Span};
use crate::parser::{TypeAnnotation, TypeKind};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: String,
    pub constraint: Option<Type>,
    pub default: Option<Type>,
}

impl GenericParam {
    pub fn new(name: String) -> Self {
        Self {
            name,
            constraint: None,
            default: None,
        }
    }

    pub fn with_constraint(name: String, constraint: Type) -> Self {
        Self {
            name,
            constraint: Some(constraint),
            default: None,
        }
    }

    pub fn satisfies(&self, ty: &Type) -> bool {
        if let Some(ref constraint) = self.constraint {
            ty.is_compatible_with(constraint)
        } else {
            true
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GenericContext {
    substitutions: HashMap<String, Type>,
    parameters: Vec<GenericParam>,
}

impl GenericContext {
    pub fn new() -> Self {
        Self {
            substitutions: HashMap::new(),
            parameters: Vec::new(),
        }
    }

    pub fn with_parameters(params: Vec<GenericParam>) -> Self {
        Self {
            substitutions: HashMap::new(),
            parameters: params,
        }
    }

    pub fn substitute(&mut self, name: &str, ty: Type) {
        self.substitutions.insert(name.to_string(), ty);
    }

    pub fn get(&self, name: &str) -> Option<&Type> {
        self.substitutions.get(name)
    }

    pub fn is_parameter(&self, name: &str) -> bool {
        self.parameters.iter().any(|p| p.name == name)
    }

    pub fn get_parameter(&self, name: &str) -> Option<&GenericParam> {
        self.parameters.iter().find(|p| p.name == name)
    }

    pub fn apply(&self, ty: &Type) -> Type {
        match ty {
            Type::Generic(name) => self.substitutions.get(name).cloned().unwrap_or(ty.clone()),
            Type::Array(inner) => Type::Array(Box::new(self.apply(inner))),
            Type::Map(k, v) => Type::Map(Box::new(self.apply(k)), Box::new(self.apply(v))),
            Type::Optional(inner) => Type::Optional(Box::new(self.apply(inner))),
            Type::Function {
                params,
                return_type,
            } => Type::Function {
                params: params.iter().map(|p| self.apply(p)).collect(),
                return_type: Box::new(self.apply(return_type)),
            },
            _ => ty.clone(),
        }
    }

    pub fn is_fully_resolved(&self) -> bool {
        self.parameters
            .iter()
            .all(|p| self.substitutions.contains_key(&p.name))
    }

    pub fn unresolved_params(&self) -> Vec<&str> {
        self.parameters
            .iter()
            .filter(|p| !self.substitutions.contains_key(&p.name))
            .map(|p| p.name.as_str())
            .collect()
    }
}

pub struct GenericResolver {
    contexts: Vec<GenericContext>,
    diagnostics: Vec<Diagnostic>,
}

impl GenericResolver {
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn push_context(&mut self, context: GenericContext) {
        self.contexts.push(context);
    }

    pub fn pop_context(&mut self) -> Option<GenericContext> {
        self.contexts.pop()
    }

    pub fn current_context(&self) -> Option<&GenericContext> {
        self.contexts.last()
    }

    pub fn current_context_mut(&mut self) -> Option<&mut GenericContext> {
        self.contexts.last_mut()
    }

    pub fn is_generic_param(&self, name: &str) -> bool {
        self.contexts.iter().rev().any(|ctx| ctx.is_parameter(name))
    }

    pub fn resolve(&self, ty: &Type) -> Type {
        for ctx in self.contexts.iter().rev() {
            if let Type::Generic(name) = ty {
                if let Some(resolved) = ctx.get(name) {
                    return resolved.clone();
                }
            }
        }
        ty.clone()
    }

    pub fn apply_all(&self, ty: &Type) -> Type {
        let mut result = ty.clone();
        for ctx in self.contexts.iter().rev() {
            result = ctx.apply(&result);
        }
        result
    }

    pub fn instantiate(
        &mut self,
        params: &[GenericParam],
        args: &[Type],
        span: Span,
    ) -> Option<GenericContext> {
        if params.len() != args.len() {
            let min_args = params.iter().filter(|p| p.default.is_none()).count();
            if args.len() < min_args || args.len() > params.len() {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "Expected {} type arguments, got {}",
                        params.len(),
                        args.len()
                    ),
                    format!("متوقع {} معاملات نوع، وُجد {}", params.len(), args.len()),
                    span,
                ));
                return None;
            }
        }

        let mut context = GenericContext::with_parameters(params.to_vec());

        for (i, param) in params.iter().enumerate() {
            let arg = if i < args.len() {
                &args[i]
            } else if let Some(ref default) = param.default {
                default
            } else {
                self.diagnostics.push(Diagnostic::error(
                    format!("Missing type argument for '{}'", param.name),
                    format!("معامل نوع مفقود لـ '{}'", param.name),
                    span,
                ));
                return None;
            };

            if !param.satisfies(arg) {
                self.diagnostics.push(Diagnostic::error(
                    format!(
                        "Type '{}' does not satisfy constraint for '{}'",
                        arg, param.name
                    ),
                    format!(
                        "النوع '{}' لا يستوفي القيد لـ '{}'",
                        arg.arabic_name(),
                        param.name
                    ),
                    span,
                ));
                return None;
            }

            context.substitute(&param.name, arg.clone());
        }

        Some(context)
    }

    pub fn infer_type_args(
        &mut self,
        params: &[GenericParam],
        param_types: &[Type],
        arg_types: &[Type],
    ) -> Option<GenericContext> {
        let mut context = GenericContext::with_parameters(params.to_vec());

        for (param_type, arg_type) in param_types.iter().zip(arg_types.iter()) {
            self.infer_from_type(&mut context, param_type, arg_type);
        }

        if context.is_fully_resolved() {
            Some(context)
        } else {
            None
        }
    }

    fn infer_from_type(&self, context: &mut GenericContext, param_type: &Type, arg_type: &Type) {
        match param_type {
            Type::Generic(name) => {
                if context.is_parameter(name) && !context.substitutions.contains_key(name) {
                    context.substitute(name, arg_type.clone());
                }
            }
            Type::Array(inner) => {
                if let Type::Array(arg_inner) = arg_type {
                    self.infer_from_type(context, inner, arg_inner);
                }
            }
            Type::Map(pk, pv) => {
                if let Type::Map(ak, av) = arg_type {
                    self.infer_from_type(context, pk, ak);
                    self.infer_from_type(context, pv, av);
                }
            }
            Type::Optional(inner) => {
                if let Type::Optional(arg_inner) = arg_type {
                    self.infer_from_type(context, inner, arg_inner);
                }
            }
            Type::Function {
                params,
                return_type,
            } => {
                if let Type::Function {
                    params: arg_params,
                    return_type: arg_ret,
                } = arg_type
                {
                    for (p, a) in params.iter().zip(arg_params.iter()) {
                        self.infer_from_type(context, p, a);
                    }
                    self.infer_from_type(context, return_type, arg_ret);
                }
            }
            _ => {}
        }
    }

    pub fn parse_generic_params(&self, type_ann: &TypeAnnotation) -> Vec<GenericParam> {
        if let TypeKind::Generic { args, .. } = &type_ann.kind {
            args.iter()
                .filter_map(|arg| {
                    if let TypeKind::Simple(name) = &arg.kind {
                        Some(GenericParam::new(name.clone()))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }
}

impl Default for GenericResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_param_creation() {
        let param = GenericParam::new("ن".to_string());
        assert_eq!(param.name, "ن");
        assert!(param.constraint.is_none());
    }

    #[test]
    fn test_generic_context_substitution() {
        let mut context = GenericContext::new();
        context.substitute("ن", Type::Int);

        assert_eq!(context.get("ن"), Some(&Type::Int));
    }

    #[test]
    fn test_apply_substitution() {
        let mut context = GenericContext::with_parameters(vec![GenericParam::new("ن".to_string())]);
        context.substitute("ن", Type::String);

        let generic = Type::Generic("ن".to_string());
        let resolved = context.apply(&generic);
        assert_eq!(resolved, Type::String);

        let array_generic = Type::Array(Box::new(Type::Generic("ن".to_string())));
        let resolved_array = context.apply(&array_generic);
        assert_eq!(resolved_array, Type::Array(Box::new(Type::String)));
    }

    #[test]
    fn test_instantiation() {
        let mut resolver = GenericResolver::new();
        let params = vec![GenericParam::new("ن".to_string())];
        let args = vec![Type::Int];

        let context = resolver.instantiate(&params, &args, Span::empty()).unwrap();
        assert_eq!(context.get("ن"), Some(&Type::Int));
    }

    #[test]
    fn test_type_inference() {
        let mut resolver = GenericResolver::new();
        let params = vec![GenericParam::new("ن".to_string())];
        let param_types = vec![Type::Array(Box::new(Type::Generic("ن".to_string())))];
        let arg_types = vec![Type::Array(Box::new(Type::String))];

        let context = resolver
            .infer_type_args(&params, &param_types, &arg_types)
            .unwrap();
        assert_eq!(context.get("ن"), Some(&Type::String));
    }

    #[test]
    fn test_nested_contexts() {
        let mut resolver = GenericResolver::new();

        let mut outer = GenericContext::with_parameters(vec![GenericParam::new("أ".to_string())]);
        outer.substitute("أ", Type::Int);
        resolver.push_context(outer);

        let mut inner = GenericContext::with_parameters(vec![GenericParam::new("ب".to_string())]);
        inner.substitute("ب", Type::String);
        resolver.push_context(inner);

        let outer_type = Type::Generic("أ".to_string());
        let inner_type = Type::Generic("ب".to_string());

        assert_eq!(resolver.resolve(&outer_type), Type::Int);
        assert_eq!(resolver.resolve(&inner_type), Type::String);

        resolver.pop_context();
        resolver.pop_context();
    }
}
