//! Class Resolver - Builds class hierarchy and validates OOP relationships
//!
//! This module handles:
//! - Building class inheritance hierarchy
//! - Validating interface implementations
//! - Constructing virtual method tables (vtables)
//! - Method override validation

use super::types::Type;
use crate::error::codes::{
    ERR_CIRCULAR_INHERITANCE, ERR_INTERFACE_NOT_IMPLEMENTED, ERR_PRIVATE_ACCESS,
};
use crate::error::{Diagnostic, Span};
use crate::parser::{ClassMember, MethodSignature, TypeAnnotation, Visibility};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: String,
    pub ty: Type,
    pub visibility: Visibility,
    pub is_static: bool,
    pub has_initializer: bool,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_async: bool,
    pub is_abstract: bool,
    pub vtable_index: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub type_params: Vec<String>,
    pub parent: Option<String>,
    pub interfaces: Vec<String>,
    pub fields: IndexMap<String, FieldInfo>,
    pub methods: IndexMap<String, MethodInfo>,
    pub constructor: Option<MethodInfo>,
    pub vtable: Vec<String>,
    pub span: Span,
}

impl ClassInfo {
    pub fn new(name: String, span: Span) -> Self {
        Self {
            name,
            type_params: Vec::new(),
            parent: None,
            interfaces: Vec::new(),
            fields: IndexMap::new(),
            methods: IndexMap::new(),
            constructor: None,
            vtable: Vec::new(),
            span,
        }
    }

    pub fn with_type_params(name: String, type_params: Vec<String>, span: Span) -> Self {
        Self {
            name,
            type_params,
            parent: None,
            interfaces: Vec::new(),
            fields: IndexMap::new(),
            methods: IndexMap::new(),
            constructor: None,
            vtable: Vec::new(),
            span,
        }
    }

    pub fn is_generic(&self) -> bool {
        !self.type_params.is_empty()
    }

    pub fn get_field<'a>(
        &'a self,
        name: &str,
        resolver: &'a ClassResolver,
    ) -> Option<&'a FieldInfo> {
        self.get_field_with_defining_class(name, resolver)
            .map(|(field, _)| field)
    }

    /// Get a field and the name of the class that defines it.
    /// Returns (field, defining_class_name).
    pub fn get_field_with_defining_class<'a>(
        &'a self,
        name: &str,
        resolver: &'a ClassResolver,
    ) -> Option<(&'a FieldInfo, &'a str)> {
        let mut visited = HashSet::new();
        self.get_field_with_class_cycle_check(name, resolver, &mut visited)
    }

    fn get_field_with_class_cycle_check<'a>(
        &'a self,
        name: &str,
        resolver: &'a ClassResolver,
        visited: &mut HashSet<String>,
    ) -> Option<(&'a FieldInfo, &'a str)> {
        if let Some(field) = self.fields.get(name) {
            return Some((field, &self.name));
        }

        if let Some(parent_name) = &self.parent {
            if visited.contains(parent_name) {
                return None;
            }
            visited.insert(parent_name.clone());
            if let Some(parent) = resolver.get_class(parent_name) {
                return parent.get_field_with_class_cycle_check(name, resolver, visited);
            }
        }

        None
    }

    pub fn get_method<'a>(
        &'a self,
        name: &str,
        resolver: &'a ClassResolver,
    ) -> Option<&'a MethodInfo> {
        self.get_method_with_defining_class(name, resolver)
            .map(|(method, _)| method)
    }

    /// Get a method and the name of the class that defines it.
    /// Returns (method, defining_class_name).
    pub fn get_method_with_defining_class<'a>(
        &'a self,
        name: &str,
        resolver: &'a ClassResolver,
    ) -> Option<(&'a MethodInfo, &'a str)> {
        let mut visited = HashSet::new();
        self.get_method_with_class_cycle_check(name, resolver, &mut visited)
    }

    fn get_method_with_class_cycle_check<'a>(
        &'a self,
        name: &str,
        resolver: &'a ClassResolver,
        visited: &mut HashSet<String>,
    ) -> Option<(&'a MethodInfo, &'a str)> {
        if let Some(method) = self.methods.get(name) {
            return Some((method, &self.name));
        }

        if let Some(parent_name) = &self.parent {
            if visited.contains(parent_name) {
                return None;
            }
            visited.insert(parent_name.clone());
            if let Some(parent) = resolver.get_class(parent_name) {
                return parent.get_method_with_class_cycle_check(name, resolver, visited);
            }
        }

        None
    }

    pub fn has_own_method(&self, name: &str) -> bool {
        self.methods.contains_key(name)
    }

    pub fn all_fields<'a>(&'a self, resolver: &'a ClassResolver) -> Vec<(&'a str, &'a FieldInfo)> {
        let mut visited = HashSet::new();
        self.all_fields_with_cycle_check(resolver, &mut visited)
    }

    fn all_fields_with_cycle_check<'a>(
        &'a self,
        resolver: &'a ClassResolver,
        visited: &mut HashSet<String>,
    ) -> Vec<(&'a str, &'a FieldInfo)> {
        let mut fields = Vec::new();

        if let Some(parent_name) = &self.parent {
            if !visited.contains(parent_name) {
                visited.insert(parent_name.clone());
                if let Some(parent) = resolver.get_class(parent_name) {
                    fields.extend(parent.all_fields_with_cycle_check(resolver, visited));
                }
            }
        }

        for (name, field) in &self.fields {
            fields.push((name.as_str(), field));
        }

        fields
    }

    pub fn all_methods<'a>(
        &'a self,
        resolver: &'a ClassResolver,
    ) -> Vec<(&'a str, &'a MethodInfo)> {
        let mut visited = HashSet::new();
        self.all_methods_with_cycle_check(resolver, &mut visited)
    }

    fn all_methods_with_cycle_check<'a>(
        &'a self,
        resolver: &'a ClassResolver,
        visited: &mut HashSet<String>,
    ) -> Vec<(&'a str, &'a MethodInfo)> {
        let mut methods: IndexMap<&str, &MethodInfo> = IndexMap::new();

        if let Some(parent_name) = &self.parent {
            if !visited.contains(parent_name) {
                visited.insert(parent_name.clone());
                if let Some(parent) = resolver.get_class(parent_name) {
                    for (name, method) in parent.all_methods_with_cycle_check(resolver, visited) {
                        methods.insert(name, method);
                    }
                }
            }
        }

        for (name, method) in &self.methods {
            methods.insert(name.as_str(), method);
        }

        methods.into_iter().collect()
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub methods: IndexMap<String, MethodSignatureInfo>,
    pub extends: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MethodSignatureInfo {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
}

impl InterfaceInfo {
    pub fn all_methods<'a>(
        &'a self,
        resolver: &'a ClassResolver,
    ) -> Vec<(&'a str, &'a MethodSignatureInfo)> {
        let mut visited = HashSet::new();
        self.all_methods_with_cycle_check(resolver, &mut visited)
    }

    fn all_methods_with_cycle_check<'a>(
        &'a self,
        resolver: &'a ClassResolver,
        visited: &mut HashSet<String>,
    ) -> Vec<(&'a str, &'a MethodSignatureInfo)> {
        let mut methods: IndexMap<&str, &MethodSignatureInfo> = IndexMap::new();

        for parent_name in &self.extends {
            if visited.contains(parent_name) {
                continue;
            }
            visited.insert(parent_name.clone());
            if let Some(parent) = resolver.get_interface(parent_name) {
                for (name, method) in parent.all_methods_with_cycle_check(resolver, visited) {
                    methods.insert(name, method);
                }
            }
        }

        for (name, method) in &self.methods {
            methods.insert(name.as_str(), method);
        }

        methods.into_iter().collect()
    }
}

pub struct ClassResolver {
    classes: HashMap<String, ClassInfo>,
    interfaces: HashMap<String, InterfaceInfo>,
    diagnostics: Vec<Diagnostic>,
}

impl ClassResolver {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            interfaces: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn get_class(&self, name: &str) -> Option<&ClassInfo> {
        self.classes.get(name)
    }

    pub fn get_class_mut(&mut self, name: &str) -> Option<&mut ClassInfo> {
        self.classes.get_mut(name)
    }

    pub fn get_interface(&self, name: &str) -> Option<&InterfaceInfo> {
        self.interfaces.get(name)
    }

    pub fn type_exists(&self, name: &str) -> bool {
        self.classes.contains_key(name) || self.interfaces.contains_key(name)
    }

    /// Get all registered class names.
    pub fn all_class_names(&self) -> Vec<String> {
        self.classes.keys().cloned().collect()
    }

    /// Get all registered interface names.
    pub fn all_interface_names(&self) -> Vec<String> {
        self.interfaces.keys().cloned().collect()
    }

    pub fn register_class(
        &mut self,
        name: &str,
        type_params: &[String],
        parent: Option<&str>,
        interfaces: &[String],
        span: Span,
    ) {
        let mut class_info =
            ClassInfo::with_type_params(name.to_string(), type_params.to_vec(), span);
        class_info.parent = parent.map(|s| s.to_string());
        class_info.interfaces = interfaces.to_vec();
        self.classes.insert(name.to_string(), class_info);
    }

    pub fn register_interface(&mut self, name: &str, extends: &[String], span: Span) {
        let interface_info = InterfaceInfo {
            name: name.to_string(),
            methods: IndexMap::new(),
            extends: extends.to_vec(),
            span,
        };
        self.interfaces.insert(name.to_string(), interface_info);
    }

    pub fn add_interface_methods(
        &mut self,
        interface_name: &str,
        methods: &[MethodSignature],
        resolve_type: impl Fn(&TypeAnnotation) -> Type,
    ) {
        if let Some(interface) = self.interfaces.get_mut(interface_name) {
            for method in methods {
                let params: Vec<(String, Type)> = method
                    .params
                    .iter()
                    .map(|p| {
                        let ty = p.ty.as_ref().map(&resolve_type).unwrap_or(Type::Any);
                        (p.name.clone(), ty)
                    })
                    .collect();

                let return_type = method
                    .return_type
                    .as_ref()
                    .map(&resolve_type)
                    .unwrap_or(Type::Void);

                let sig = MethodSignatureInfo {
                    name: method.name.clone(),
                    params,
                    return_type,
                };

                interface.methods.insert(method.name.clone(), sig);
            }
        }
    }

    pub fn add_class_members(
        &mut self,
        class_name: &str,
        members: &[ClassMember],
        resolve_type: impl Fn(&TypeAnnotation) -> Type,
    ) {
        let mut fields = IndexMap::new();
        let mut methods = IndexMap::new();
        let mut constructor = None;

        for member in members {
            match member {
                ClassMember::Field {
                    visibility,
                    name,
                    ty,
                    init,
                    is_static,
                    ..
                } => {
                    let field_type = ty.as_ref().map(&resolve_type).unwrap_or(Type::Any);

                    let field_info = FieldInfo {
                        name: name.clone(),
                        ty: field_type,
                        visibility: *visibility,
                        is_static: *is_static,
                        has_initializer: init.is_some(),
                    };

                    fields.insert(name.clone(), field_info);
                }

                ClassMember::Method {
                    visibility,
                    name,
                    params,
                    return_type,
                    is_static,
                    is_async,
                    ..
                } => {
                    let param_types: Vec<(String, Type)> = params
                        .iter()
                        .map(|p| {
                            let ty = p.ty.as_ref().map(&resolve_type).unwrap_or(Type::Any);
                            (p.name.clone(), ty)
                        })
                        .collect();

                    let ret_type = return_type
                        .as_ref()
                        .map(&resolve_type)
                        .unwrap_or(Type::Void);

                    let method_info = MethodInfo {
                        name: name.clone(),
                        params: param_types,
                        return_type: ret_type,
                        visibility: *visibility,
                        is_static: *is_static,
                        is_async: *is_async,
                        is_abstract: false,
                        vtable_index: None,
                    };

                    methods.insert(name.clone(), method_info);
                }

                ClassMember::Constructor { params, .. } => {
                    let param_types: Vec<(String, Type)> = params
                        .iter()
                        .map(|p| {
                            let ty = p.ty.as_ref().map(&resolve_type).unwrap_or(Type::Any);
                            (p.name.clone(), ty)
                        })
                        .collect();

                    constructor = Some(MethodInfo {
                        name: "منشئ".to_string(),
                        params: param_types,
                        return_type: Type::Void,
                        visibility: Visibility::Public,
                        is_static: false,
                        is_async: false,
                        is_abstract: false,
                        vtable_index: None,
                    });
                }

                ClassMember::Property {
                    visibility,
                    name,
                    ty,
                    accessors,
                    default_value,
                    is_static,
                    ..
                } => {
                    let prop_type = resolve_type(ty);

                    let property_field_info = FieldInfo {
                        name: name.clone(),
                        ty: prop_type.clone(),
                        visibility: *visibility,
                        is_static: *is_static,
                        has_initializer: default_value.is_some(),
                    };
                    fields.insert(name.clone(), property_field_info);

                    let backing_field_name = format!("_{}", name);
                    let backing_field_info = FieldInfo {
                        name: backing_field_name.clone(),
                        ty: prop_type.clone(),
                        visibility: Visibility::Private,
                        is_static: *is_static,
                        has_initializer: default_value.is_some(),
                    };
                    fields.insert(backing_field_name, backing_field_info);

                    let has_getter = accessors
                        .iter()
                        .any(|a| matches!(a, crate::parser::PropertyAccessor::Get { .. }));
                    if has_getter || accessors.is_empty() {
                        let getter_name = format!("__احصل_{}", name);
                        let getter_info = MethodInfo {
                            name: getter_name.clone(),
                            params: vec![],
                            return_type: prop_type.clone(),
                            visibility: *visibility,
                            is_static: *is_static,
                            is_async: false,
                            is_abstract: false,
                            vtable_index: None,
                        };
                        methods.insert(getter_name, getter_info);
                    }

                    let has_setter = accessors
                        .iter()
                        .any(|a| matches!(a, crate::parser::PropertyAccessor::Set { .. }));
                    if has_setter || accessors.is_empty() {
                        let setter_name = format!("__عيّن_{}", name);
                        let setter_info = MethodInfo {
                            name: setter_name.clone(),
                            params: vec![("قيمة".to_string(), prop_type.clone())],
                            return_type: Type::Void,
                            visibility: *visibility,
                            is_static: *is_static,
                            is_async: false,
                            is_abstract: false,
                            vtable_index: None,
                        };
                        methods.insert(setter_name, setter_info);
                    }
                }
            }
        }

        if let Some(class) = self.classes.get_mut(class_name) {
            class.fields = fields;
            class.methods = methods;
            class.constructor = constructor;
        }
    }

    pub fn build_vtables(&mut self) {
        let order = self.topological_sort_classes();

        for class_name in order {
            self.build_vtable_for_class(&class_name);
        }
    }

    fn build_vtable_for_class(&mut self, class_name: &str) {
        let parent_vtable: Vec<String> = if let Some(class) = self.classes.get(class_name) {
            if let Some(parent_name) = &class.parent {
                self.classes
                    .get(parent_name)
                    .map(|p| p.vtable.clone())
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            return;
        };

        let mut vtable = parent_vtable;

        if let Some(class) = self.classes.get(class_name) {
            let method_names: Vec<String> = class
                .methods
                .iter()
                .filter(|(_, m)| !m.is_static)
                .map(|(n, _)| n.clone())
                .collect();

            for method_name in method_names {
                if let Some(pos) = vtable.iter().position(|n| n == &method_name) {
                    if let Some(class) = self.classes.get_mut(class_name) {
                        if let Some(method) = class.methods.get_mut(&method_name) {
                            method.vtable_index = Some(pos);
                        }
                    }
                } else {
                    let new_index = vtable.len();
                    vtable.push(method_name.clone());
                    if let Some(class) = self.classes.get_mut(class_name) {
                        if let Some(method) = class.methods.get_mut(&method_name) {
                            method.vtable_index = Some(new_index);
                        }
                    }
                }
            }
        }

        if let Some(class) = self.classes.get_mut(class_name) {
            class.vtable = vtable;
        }
    }

    fn topological_sort_classes(&self) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();

        for name in self.classes.keys() {
            self.visit_class(name, &mut visited, &mut result);
        }

        result
    }

    fn visit_class(&self, name: &str, visited: &mut HashSet<String>, result: &mut Vec<String>) {
        if visited.contains(name) {
            return;
        }
        visited.insert(name.to_string());

        if let Some(class) = self.classes.get(name) {
            if let Some(parent_name) = &class.parent {
                self.visit_class(parent_name, visited, result);
            }
        }

        result.push(name.to_string());
    }

    pub fn validate(&mut self) -> Result<(), Vec<Diagnostic>> {
        self.check_circular_inheritance();
        self.check_interface_implementations();
        self.check_method_overrides();
        self.check_abstract_implementations();

        if self.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.diagnostics))
        }
    }

    fn check_circular_inheritance(&mut self) {
        for (name, class) in &self.classes {
            let mut visited = HashSet::new();
            let mut current = Some(name.as_str());

            while let Some(class_name) = current {
                if visited.contains(class_name) {
                    self.diagnostics.push(
                        Diagnostic::error(
                            format!("وراثة دائرية مكتشفة للصنف '{}'", name),
                            class.span,
                        )
                        .with_code(ERR_CIRCULAR_INHERITANCE.to_string()),
                    );
                    break;
                }
                visited.insert(class_name);

                current = self
                    .classes
                    .get(class_name)
                    .and_then(|c| c.parent.as_deref());
            }
        }
    }

    fn check_interface_implementations(&mut self) {
        let mut violations = Vec::new();

        for (class_name, class) in &self.classes {
            for interface_name in &class.interfaces {
                if let Some(interface) = self.interfaces.get(interface_name) {
                    for (method_name, sig) in interface.all_methods(self) {
                        let has_method = class.get_method(method_name, self).is_some();

                        if !has_method {
                            violations.push((
                                format!(
                                    "الصنف '{}' لا يُنفذ الدالة '{}' من الميثاق '{}'",
                                    class_name, method_name, interface_name
                                ),
                                class.span,
                            ));
                        } else if let Some(method) = class.get_method(method_name, self) {
                            if method.params.len() != sig.params.len() {
                                violations.push((
                                    format!(
                                        "الدالة '{}' في الصنف '{}' لديها عدد خاطئ من المعاملات (متوقع {}، وجد {})",
                                        method_name, class_name, sig.params.len(), method.params.len()
                                    ),
                                    class.span,
                                ));
                            } else {
                                for (i, ((_, expected_ty), (_, actual_ty))) in
                                    sig.params.iter().zip(method.params.iter()).enumerate()
                                {
                                    if expected_ty != actual_ty {
                                        violations.push((
                                            format!(
                                                "المعامل {} للدالة '{}' في الصنف '{}' له نوع خاطئ (متوقع '{}'، وجد '{}')",
                                                i + 1, method_name, class_name, expected_ty.arabic_name(), actual_ty.arabic_name()
                                            ),
                                            class.span,
                                        ));
                                    }
                                }
                            }

                            if method.return_type != sig.return_type {
                                violations.push((
                                    format!(
                                        "الدالة '{}' في الصنف '{}' لديها نوع إرجاع خاطئ (متوقع '{}'، وجد '{}')",
                                        method_name, class_name, sig.return_type.arabic_name(), method.return_type.arabic_name()
                                    ),
                                    class.span,
                                ));
                            }
                        }
                    }
                }
            }
        }

        for (msg_ar, span) in violations {
            self.diagnostics.push(
                Diagnostic::error(&msg_ar, span)
                    .with_code(ERR_INTERFACE_NOT_IMPLEMENTED.to_string()),
            );
        }
    }

    fn check_method_overrides(&mut self) {
        let mut violations = Vec::new();
        let mut private_override_violations = Vec::new();

        for class in self.classes.values() {
            if let Some(parent_name) = &class.parent {
                if let Some(parent) = self.classes.get(parent_name) {
                    for (method_name, method) in &class.methods {
                        if let Some(parent_method) = parent.get_method(method_name, self) {
                            // An override may keep or widen visibility, never narrow it —
                            // otherwise a caller holding an ancestor-typed reference could
                            // pass the parent's (wider) visibility check at compile time
                            // while virtual dispatch (#184) resolves at runtime to an
                            // override the same caller couldn't have called directly.
                            if visibility_rank(method.visibility)
                                < visibility_rank(parent_method.visibility)
                            {
                                private_override_violations.push((
                                    format!(
                                        "لا يمكن تجاوز الدالة {} '{}' بخصوصية {}",
                                        format_visibility_ar(parent_method.visibility),
                                        method_name,
                                        narrowed_visibility_ar(method.visibility)
                                    ),
                                    class.span,
                                ));
                            }

                            if method.params.len() != parent_method.params.len() {
                                violations.push((
                                    format!(
                                        "تجاوز الدالة '{}' لديه {} معاملات، لكن الأب لديه {}",
                                        method_name,
                                        method.params.len(),
                                        parent_method.params.len()
                                    ),
                                    class.span,
                                ));
                            } else {
                                for (i, ((_, child_ty), (_, parent_ty))) in method
                                    .params
                                    .iter()
                                    .zip(parent_method.params.iter())
                                    .enumerate()
                                {
                                    if !child_ty.is_compatible_with(parent_ty)
                                        && !parent_ty.is_compatible_with(child_ty)
                                    {
                                        violations.push((
                                            format!(
                                                "المعامل {} للدالة '{}' له نوع غير متوافق '{}', المتوقع '{}'",
                                                i + 1, method_name, child_ty.arabic_name(), parent_ty.arabic_name()
                                            ),
                                            class.span,
                                        ));
                                    }
                                }
                            }

                            if !method
                                .return_type
                                .is_compatible_with(&parent_method.return_type)
                            {
                                violations.push((
                                    format!(
                                        "نوع الإرجاع للدالة '{}' غير متوافق مع الأب",
                                        method_name
                                    ),
                                    class.span,
                                ));
                            }
                        }
                    }
                }
            }
        }

        for (msg_ar, span) in private_override_violations {
            self.diagnostics
                .push(Diagnostic::error(&msg_ar, span).with_code(ERR_PRIVATE_ACCESS.to_string()));
        }

        for (msg_ar, span) in violations {
            self.diagnostics.push(Diagnostic::error(&msg_ar, span));
        }
    }

    fn check_abstract_implementations(&mut self) {
        let mut violations = Vec::new();

        for (class_name, class) in &self.classes {
            let is_abstract_class = class.methods.values().any(|m| m.is_abstract);
            if is_abstract_class {
                continue;
            }

            let mut abstract_methods: Vec<(String, String)> = Vec::new(); // (method_name, defining_class)
            let mut current_parent = class.parent.clone();
            let mut visited_parents = HashSet::new();

            while let Some(parent_name) = current_parent {
                if visited_parents.contains(&parent_name) {
                    break;
                }
                visited_parents.insert(parent_name.clone());

                if let Some(parent_class) = self.classes.get(&parent_name) {
                    for (method_name, method) in &parent_class.methods {
                        if method.is_abstract
                            && !abstract_methods.iter().any(|(m, _)| m == method_name)
                        {
                            abstract_methods.push((method_name.clone(), parent_name.clone()));
                        }
                    }
                    current_parent = parent_class.parent.clone();
                } else {
                    break;
                }
            }

            for (method_name, defining_class) in abstract_methods {
                let is_implemented = class
                    .get_method(&method_name, self)
                    .is_some_and(|m| !m.is_abstract);
                if !is_implemented {
                    violations.push((
                        format!(
                            "الصنف '{}' يجب أن يُطبّق الدالة المجردة '{}' من '{}'",
                            class_name, method_name, defining_class
                        ),
                        class.span,
                    ));
                }
            }
        }

        for (msg_ar, span) in violations {
            self.diagnostics.push(Diagnostic::error(&msg_ar, span));
        }
    }

    pub fn is_subclass(&self, class_name: &str, potential_parent: &str) -> bool {
        let mut visited = HashSet::new();
        self.is_subclass_with_cycle_check(class_name, potential_parent, &mut visited)
    }

    fn is_subclass_with_cycle_check(
        &self,
        class_name: &str,
        potential_parent: &str,
        visited: &mut HashSet<String>,
    ) -> bool {
        if class_name == potential_parent {
            return true;
        }

        if visited.contains(class_name) {
            return false;
        }
        visited.insert(class_name.to_string());

        if let Some(class) = self.classes.get(class_name) {
            if let Some(parent_name) = &class.parent {
                return self.is_subclass_with_cycle_check(parent_name, potential_parent, visited);
            }
        }

        false
    }

    pub fn implements_interface(&self, class_name: &str, interface_name: &str) -> bool {
        let mut visited = HashSet::new();
        self.implements_interface_with_cycle_check(class_name, interface_name, &mut visited)
    }

    fn implements_interface_with_cycle_check(
        &self,
        class_name: &str,
        interface_name: &str,
        visited: &mut HashSet<String>,
    ) -> bool {
        if visited.contains(class_name) {
            return false;
        }
        visited.insert(class_name.to_string());

        if let Some(class) = self.classes.get(class_name) {
            if class.interfaces.contains(&interface_name.to_string()) {
                return true;
            }

            if let Some(parent_name) = &class.parent {
                return self.implements_interface_with_cycle_check(
                    parent_name,
                    interface_name,
                    visited,
                );
            }
        }

        false
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl Default for ClassResolver {
    fn default() -> Self {
        Self::new()
    }
}

fn format_visibility_ar(vis: Visibility) -> &'static str {
    match vis {
        Visibility::Public => "العامة",
        Visibility::Private => "الخاصة",
        Visibility::Protected => "المحمية",
    }
}

/// Indefinite-adjective form for "cannot override ... with N visibility" —
/// only ever used for the narrower of the two visibilities in that message,
/// so `Public` (the widest) never needs a form here.
fn narrowed_visibility_ar(vis: Visibility) -> &'static str {
    match vis {
        Visibility::Private => "خاصة",
        Visibility::Protected => "محمية",
        Visibility::Public => "عامة",
    }
}

/// Permissiveness ordering for override-narrowing checks: wider visibility
/// ranks higher, so an override is only valid if its rank is >= the
/// overridden method's rank.
fn visibility_rank(vis: Visibility) -> u8 {
    match vis {
        Visibility::Public => 2,
        Visibility::Protected => 1,
        Visibility::Private => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Span;

    #[test]
    fn test_class_registration() {
        let mut resolver = ClassResolver::new();
        resolver.register_class("شخص", &[], None, &[], Span::empty());
        resolver.register_class("موظف", &[], Some("شخص"), &[], Span::empty());

        assert!(resolver.get_class("شخص").is_some());
        assert!(resolver.get_class("موظف").is_some());
        assert!(resolver.is_subclass("موظف", "شخص"));
    }

    #[test]
    fn test_interface_registration() {
        let mut resolver = ClassResolver::new();
        resolver.register_interface("قابل_للطباعة", &[], Span::empty());

        assert!(resolver.get_interface("قابل_للطباعة").is_some());
    }

    #[test]
    fn test_vtable_building() {
        let mut resolver = ClassResolver::new();
        resolver.register_class("أ", &[], None, &[], Span::empty());
        resolver.register_class("ب", &[], Some("أ"), &[], Span::empty());

        if let Some(class) = resolver.get_class_mut("أ") {
            class.methods.insert(
                "دالة1".to_string(),
                MethodInfo {
                    name: "دالة1".to_string(),
                    params: vec![],
                    return_type: Type::Void,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
            );
        }

        if let Some(class) = resolver.get_class_mut("ب") {
            class.methods.insert(
                "دالة1".to_string(), // Override
                MethodInfo {
                    name: "دالة1".to_string(),
                    params: vec![],
                    return_type: Type::Void,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
            );
            class.methods.insert(
                "دالة2".to_string(), // New method
                MethodInfo {
                    name: "دالة2".to_string(),
                    params: vec![],
                    return_type: Type::Void,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
            );
        }

        resolver.build_vtables();

        let class_a = resolver.get_class("أ").unwrap();
        assert_eq!(class_a.vtable.len(), 1);
        assert_eq!(class_a.vtable[0], "دالة1");

        let class_b = resolver.get_class("ب").unwrap();
        assert_eq!(class_b.vtable.len(), 2);
        assert!(class_b.vtable.contains(&"دالة1".to_string()));
        assert!(class_b.vtable.contains(&"دالة2".to_string()));
    }

    #[test]
    fn test_circular_inheritance_detection() {
        let mut resolver = ClassResolver::new();
        resolver.register_class("أ", &[], Some("ب"), &[], Span::empty());
        resolver.register_class("ب", &[], Some("أ"), &[], Span::empty());

        let result = resolver.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_method_override_same_params_valid() {
        let mut resolver = ClassResolver::new();
        resolver.register_class("أ", &[], None, &[], Span::empty());
        resolver.register_class("ب", &[], Some("أ"), &[], Span::empty());

        if let Some(class) = resolver.get_class_mut("أ") {
            class.methods.insert(
                "دالة".to_string(),
                MethodInfo {
                    name: "دالة".to_string(),
                    params: vec![("س".to_string(), Type::Int)],
                    return_type: Type::Void,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
            );
        }

        if let Some(class) = resolver.get_class_mut("ب") {
            class.methods.insert(
                "دالة".to_string(),
                MethodInfo {
                    name: "دالة".to_string(),
                    params: vec![("س".to_string(), Type::Int)],
                    return_type: Type::Void,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
            );
        }

        let result = resolver.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_method_override_incompatible_param_type() {
        let mut resolver = ClassResolver::new();
        resolver.register_class("أ", &[], None, &[], Span::empty());
        resolver.register_class("ب", &[], Some("أ"), &[], Span::empty());

        if let Some(class) = resolver.get_class_mut("أ") {
            class.methods.insert(
                "دالة".to_string(),
                MethodInfo {
                    name: "دالة".to_string(),
                    params: vec![("س".to_string(), Type::Int)],
                    return_type: Type::Void,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
            );
        }

        if let Some(class) = resolver.get_class_mut("ب") {
            class.methods.insert(
                "دالة".to_string(),
                MethodInfo {
                    name: "دالة".to_string(),
                    params: vec![("س".to_string(), Type::String)],
                    return_type: Type::Void,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
            );
        }

        let result = resolver.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_method_override_wrong_param_count() {
        let mut resolver = ClassResolver::new();
        resolver.register_class("أ", &[], None, &[], Span::empty());
        resolver.register_class("ب", &[], Some("أ"), &[], Span::empty());

        if let Some(class) = resolver.get_class_mut("أ") {
            class.methods.insert(
                "دالة".to_string(),
                MethodInfo {
                    name: "دالة".to_string(),
                    params: vec![("س".to_string(), Type::Int)],
                    return_type: Type::Void,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
            );
        }

        if let Some(class) = resolver.get_class_mut("ب") {
            class.methods.insert(
                "دالة".to_string(),
                MethodInfo {
                    name: "دالة".to_string(),
                    params: vec![("س".to_string(), Type::Int), ("ص".to_string(), Type::Int)],
                    return_type: Type::Void,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
            );
        }

        let result = resolver.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_method_override_any_param_accepts_all() {
        let mut resolver = ClassResolver::new();
        resolver.register_class("أ", &[], None, &[], Span::empty());
        resolver.register_class("ب", &[], Some("أ"), &[], Span::empty());

        if let Some(class) = resolver.get_class_mut("أ") {
            class.methods.insert(
                "دالة".to_string(),
                MethodInfo {
                    name: "دالة".to_string(),
                    params: vec![("س".to_string(), Type::Int)],
                    return_type: Type::Void,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
            );
        }

        if let Some(class) = resolver.get_class_mut("ب") {
            class.methods.insert(
                "دالة".to_string(),
                MethodInfo {
                    name: "دالة".to_string(),
                    params: vec![("س".to_string(), Type::Any)],
                    return_type: Type::Void,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
            );
        }

        let result = resolver.validate();
        assert!(result.is_ok()); // Any accepts Int
    }

    #[test]
    fn test_method_override_fewer_params_invalid() {
        let mut resolver = ClassResolver::new();
        resolver.register_class("أ", &[], None, &[], Span::empty());
        resolver.register_class("ب", &[], Some("أ"), &[], Span::empty());

        if let Some(class) = resolver.get_class_mut("أ") {
            class.methods.insert(
                "دالة".to_string(),
                MethodInfo {
                    name: "دالة".to_string(),
                    params: vec![
                        ("س".to_string(), Type::Int),
                        ("ص".to_string(), Type::String),
                    ],
                    return_type: Type::Void,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
            );
        }

        if let Some(class) = resolver.get_class_mut("ب") {
            class.methods.insert(
                "دالة".to_string(),
                MethodInfo {
                    name: "دالة".to_string(),
                    params: vec![("س".to_string(), Type::Int)],
                    return_type: Type::Void,
                    visibility: Visibility::Public,
                    is_static: false,
                    is_async: false,
                    is_abstract: false,
                    vtable_index: None,
                },
            );
        }

        let result = resolver.validate();
        assert!(result.is_err());
    }
}
