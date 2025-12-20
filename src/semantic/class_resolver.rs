//! Class Resolver - Builds class hierarchy and validates OOP relationships
//!
//! This module handles:
//! - Building class inheritance hierarchy
//! - Validating interface implementations
//! - Constructing virtual method tables (vtables)
//! - Method override validation

use super::types::Type;
use crate::error::{Diagnostic, Span};
use crate::parser::{ClassMember, MethodSignature, TypeAnnotation, Visibility};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};

/// Information about a class field
#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: String,
    pub ty: Type,
    pub visibility: Visibility,
    pub is_static: bool,
    pub has_initializer: bool,
}

/// Information about a class method
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_async: bool,
    pub is_abstract: bool,
    /// Index in the vtable (None for static methods)
    pub vtable_index: Option<usize>,
}

/// Complete class information
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub parent: Option<String>,
    pub interfaces: Vec<String>,
    pub fields: IndexMap<String, FieldInfo>,
    pub methods: IndexMap<String, MethodInfo>,
    pub constructor: Option<MethodInfo>,
    /// Virtual method table: method names in order
    pub vtable: Vec<String>,
    pub span: Span,
}

impl ClassInfo {
    /// Create a new class info
    pub fn new(name: String, span: Span) -> Self {
        Self {
            name,
            parent: None,
            interfaces: Vec::new(),
            fields: IndexMap::new(),
            methods: IndexMap::new(),
            constructor: None,
            vtable: Vec::new(),
            span,
        }
    }

    /// Get a field by name, including inherited fields
    pub fn get_field<'a>(
        &'a self,
        name: &str,
        resolver: &'a ClassResolver,
    ) -> Option<&'a FieldInfo> {
        let mut visited = HashSet::new();
        self.get_field_with_cycle_check(name, resolver, &mut visited)
    }

    /// Internal helper to get field with cycle detection
    fn get_field_with_cycle_check<'a>(
        &'a self,
        name: &str,
        resolver: &'a ClassResolver,
        visited: &mut HashSet<String>,
    ) -> Option<&'a FieldInfo> {
        if let Some(field) = self.fields.get(name) {
            return Some(field);
        }

        // Check parent class with cycle detection
        if let Some(parent_name) = &self.parent {
            if visited.contains(parent_name) {
                // Cycle detected - prevent infinite recursion
                return None;
            }
            visited.insert(parent_name.clone());
            if let Some(parent) = resolver.get_class(parent_name) {
                return parent.get_field_with_cycle_check(name, resolver, visited);
            }
        }

        None
    }

    /// Get a method by name, including inherited methods
    pub fn get_method<'a>(
        &'a self,
        name: &str,
        resolver: &'a ClassResolver,
    ) -> Option<&'a MethodInfo> {
        let mut visited = HashSet::new();
        self.get_method_with_cycle_check(name, resolver, &mut visited)
    }

    /// Internal helper to get method with cycle detection
    fn get_method_with_cycle_check<'a>(
        &'a self,
        name: &str,
        resolver: &'a ClassResolver,
        visited: &mut HashSet<String>,
    ) -> Option<&'a MethodInfo> {
        if let Some(method) = self.methods.get(name) {
            return Some(method);
        }

        // Check parent class with cycle detection
        if let Some(parent_name) = &self.parent {
            if visited.contains(parent_name) {
                // Cycle detected - prevent infinite recursion
                return None;
            }
            visited.insert(parent_name.clone());
            if let Some(parent) = resolver.get_class(parent_name) {
                return parent.get_method_with_cycle_check(name, resolver, visited);
            }
        }

        None
    }

    /// Check if this class has a method (directly, not inherited)
    pub fn has_own_method(&self, name: &str) -> bool {
        self.methods.contains_key(name)
    }

    /// Get all fields including inherited ones
    pub fn all_fields<'a>(&'a self, resolver: &'a ClassResolver) -> Vec<(&'a str, &'a FieldInfo)> {
        let mut visited = HashSet::new();
        self.all_fields_with_cycle_check(resolver, &mut visited)
    }

    /// Internal helper to get all fields with cycle detection
    fn all_fields_with_cycle_check<'a>(
        &'a self,
        resolver: &'a ClassResolver,
        visited: &mut HashSet<String>,
    ) -> Vec<(&'a str, &'a FieldInfo)> {
        let mut fields = Vec::new();

        // First add parent fields with cycle detection
        if let Some(parent_name) = &self.parent {
            if !visited.contains(parent_name) {
                visited.insert(parent_name.clone());
                if let Some(parent) = resolver.get_class(parent_name) {
                    fields.extend(parent.all_fields_with_cycle_check(resolver, visited));
                }
            }
        }

        // Then add own fields
        for (name, field) in &self.fields {
            fields.push((name.as_str(), field));
        }

        fields
    }

    /// Get all methods including inherited ones
    pub fn all_methods<'a>(
        &'a self,
        resolver: &'a ClassResolver,
    ) -> Vec<(&'a str, &'a MethodInfo)> {
        let mut visited = HashSet::new();
        self.all_methods_with_cycle_check(resolver, &mut visited)
    }

    /// Internal helper to get all methods with cycle detection
    fn all_methods_with_cycle_check<'a>(
        &'a self,
        resolver: &'a ClassResolver,
        visited: &mut HashSet<String>,
    ) -> Vec<(&'a str, &'a MethodInfo)> {
        let mut methods: IndexMap<&str, &MethodInfo> = IndexMap::new();

        // First add parent methods with cycle detection
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

        // Then add/override with own methods
        for (name, method) in &self.methods {
            methods.insert(name.as_str(), method);
        }

        methods.into_iter().collect()
    }
}

/// Information about an interface
#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub methods: IndexMap<String, MethodSignatureInfo>,
    pub extends: Vec<String>,
    pub span: Span,
}

/// Method signature in an interface
#[derive(Debug, Clone)]
pub struct MethodSignatureInfo {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
}

impl InterfaceInfo {
    /// Get all methods including from extended interfaces
    pub fn all_methods<'a>(
        &'a self,
        resolver: &'a ClassResolver,
    ) -> Vec<(&'a str, &'a MethodSignatureInfo)> {
        let mut visited = HashSet::new();
        self.all_methods_with_cycle_check(resolver, &mut visited)
    }

    /// Internal helper to get all methods with cycle detection
    fn all_methods_with_cycle_check<'a>(
        &'a self,
        resolver: &'a ClassResolver,
        visited: &mut HashSet<String>,
    ) -> Vec<(&'a str, &'a MethodSignatureInfo)> {
        let mut methods: IndexMap<&str, &MethodSignatureInfo> = IndexMap::new();

        // First add extended interface methods with cycle detection
        for parent_name in &self.extends {
            if visited.contains(parent_name) {
                // Cycle detected - skip to prevent infinite recursion
                continue;
            }
            visited.insert(parent_name.clone());
            if let Some(parent) = resolver.get_interface(parent_name) {
                for (name, method) in parent.all_methods_with_cycle_check(resolver, visited) {
                    methods.insert(name, method);
                }
            }
        }

        // Then add own methods
        for (name, method) in &self.methods {
            methods.insert(name.as_str(), method);
        }

        methods.into_iter().collect()
    }
}

/// The class resolver - builds and validates class hierarchy
pub struct ClassResolver {
    /// All classes, keyed by name
    classes: HashMap<String, ClassInfo>,
    /// All interfaces, keyed by name
    interfaces: HashMap<String, InterfaceInfo>,
    /// Collected diagnostics
    diagnostics: Vec<Diagnostic>,
}

impl ClassResolver {
    /// Create a new class resolver
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            interfaces: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Get a class by name
    pub fn get_class(&self, name: &str) -> Option<&ClassInfo> {
        self.classes.get(name)
    }

    /// Get a mutable class by name
    pub fn get_class_mut(&mut self, name: &str) -> Option<&mut ClassInfo> {
        self.classes.get_mut(name)
    }

    /// Get an interface by name
    pub fn get_interface(&self, name: &str) -> Option<&InterfaceInfo> {
        self.interfaces.get(name)
    }

    /// Check if a type exists (class or interface)
    pub fn type_exists(&self, name: &str) -> bool {
        self.classes.contains_key(name) || self.interfaces.contains_key(name)
    }

    /// Register a class (first pass - just the declaration)
    pub fn register_class(
        &mut self,
        name: &str,
        parent: Option<&str>,
        interfaces: &[String],
        span: Span,
    ) {
        let mut class_info = ClassInfo::new(name.to_string(), span);
        class_info.parent = parent.map(|s| s.to_string());
        class_info.interfaces = interfaces.to_vec();
        self.classes.insert(name.to_string(), class_info);
    }

    /// Register an interface (first pass)
    pub fn register_interface(&mut self, name: &str, extends: &[String], span: Span) {
        let interface_info = InterfaceInfo {
            name: name.to_string(),
            methods: IndexMap::new(),
            extends: extends.to_vec(),
            span,
        };
        self.interfaces.insert(name.to_string(), interface_info);
    }

    /// Add methods to an interface
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
                        let ty = p.ty.as_ref().map(|t| resolve_type(t)).unwrap_or(Type::Any);
                        (p.name.clone(), ty)
                    })
                    .collect();

                let return_type = method
                    .return_type
                    .as_ref()
                    .map(|t| resolve_type(t))
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

    /// Add members to a class (second pass)
    pub fn add_class_members(
        &mut self,
        class_name: &str,
        members: &[ClassMember],
        resolve_type: impl Fn(&TypeAnnotation) -> Type,
    ) {
        // First collect all member info
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
                    let field_type = ty.as_ref().map(|t| resolve_type(t)).unwrap_or(Type::Any);

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
                            let ty = p.ty.as_ref().map(|t| resolve_type(t)).unwrap_or(Type::Any);
                            (p.name.clone(), ty)
                        })
                        .collect();

                    let ret_type = return_type
                        .as_ref()
                        .map(|t| resolve_type(t))
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
                            let ty = p.ty.as_ref().map(|t| resolve_type(t)).unwrap_or(Type::Any);
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
            }
        }

        // Update the class
        if let Some(class) = self.classes.get_mut(class_name) {
            class.fields = fields;
            class.methods = methods;
            class.constructor = constructor;
        }
    }

    /// Build vtables for all classes
    pub fn build_vtables(&mut self) {
        // We need to process classes in topological order (parents before children)
        let order = self.topological_sort_classes();

        for class_name in order {
            self.build_vtable_for_class(&class_name);
        }
    }

    /// Build vtable for a single class
    fn build_vtable_for_class(&mut self, class_name: &str) {
        // Get parent vtable if exists
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

        // Build the new vtable
        let mut vtable = parent_vtable;

        if let Some(class) = self.classes.get(class_name) {
            // Collect method names that need to be added or updated
            let method_names: Vec<String> = class
                .methods
                .iter()
                .filter(|(_, m)| !m.is_static)
                .map(|(n, _)| n.clone())
                .collect();

            for method_name in method_names {
                // Check if method overrides a parent method
                if let Some(pos) = vtable.iter().position(|n| n == &method_name) {
                    // Override: update vtable_index
                    if let Some(class) = self.classes.get_mut(class_name) {
                        if let Some(method) = class.methods.get_mut(&method_name) {
                            method.vtable_index = Some(pos);
                        }
                    }
                } else {
                    // New method: add to vtable
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

        // Store the vtable
        if let Some(class) = self.classes.get_mut(class_name) {
            class.vtable = vtable;
        }
    }

    /// Topological sort of classes (parents before children)
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

        // Visit parent first
        if let Some(class) = self.classes.get(name) {
            if let Some(parent_name) = &class.parent {
                self.visit_class(parent_name, visited, result);
            }
        }

        result.push(name.to_string());
    }

    /// Validate the class hierarchy
    pub fn validate(&mut self) -> Result<(), Vec<Diagnostic>> {
        self.check_circular_inheritance();
        self.check_interface_implementations();
        self.check_method_overrides();

        if self.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.diagnostics))
        }
    }

    /// Check for circular inheritance
    fn check_circular_inheritance(&mut self) {
        for (name, class) in &self.classes {
            let mut visited = HashSet::new();
            let mut current = Some(name.as_str());

            while let Some(class_name) = current {
                if visited.contains(class_name) {
                    self.diagnostics.push(Diagnostic::error(
                        &format!("Circular inheritance detected for class '{}'", name),
                        &format!("وراثة دائرية مكتشفة للصنف '{}'", name),
                        class.span,
                    ));
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

    /// Check that all interface methods are implemented
    fn check_interface_implementations(&mut self) {
        // Collect all violations first
        let mut violations = Vec::new();

        for (class_name, class) in &self.classes {
            for interface_name in &class.interfaces {
                if let Some(interface) = self.interfaces.get(interface_name) {
                    for (method_name, sig) in interface.all_methods(self) {
                        // Check if class has this method
                        let has_method = class.get_method(method_name, self).is_some();

                        if !has_method {
                            violations.push((
                                format!(
                                    "Class '{}' does not implement method '{}' from interface '{}'",
                                    class_name, method_name, interface_name
                                ),
                                format!(
                                    "الصنف '{}' لا يُنفذ الدالة '{}' من الواجهة '{}'",
                                    class_name, method_name, interface_name
                                ),
                                class.span,
                            ));
                        } else if let Some(method) = class.get_method(method_name, self) {
                            // Check parameter count matches
                            if method.params.len() != sig.params.len() {
                                violations.push((
                                    format!(
                                        "Method '{}' in class '{}' has wrong number of parameters (expected {}, got {})",
                                        method_name, class_name, sig.params.len(), method.params.len()
                                    ),
                                    format!(
                                        "الدالة '{}' في الصنف '{}' لديها عدد خاطئ من المعاملات (متوقع {}، وجد {})",
                                        method_name, class_name, sig.params.len(), method.params.len()
                                    ),
                                    class.span,
                                ));
                            } else {
                                // Check parameter types match
                                for (i, ((_, expected_ty), (_, actual_ty))) in
                                    sig.params.iter().zip(method.params.iter()).enumerate()
                                {
                                    if expected_ty != actual_ty {
                                        violations.push((
                                            format!(
                                                "Parameter {} of method '{}' in class '{}' has wrong type (expected {:?}, got {:?})",
                                                i + 1, method_name, class_name, expected_ty, actual_ty
                                            ),
                                            format!(
                                                "المعامل {} للدالة '{}' في الصنف '{}' له نوع خاطئ (متوقع {:?}، وجد {:?})",
                                                i + 1, method_name, class_name, expected_ty, actual_ty
                                            ),
                                            class.span,
                                        ));
                                    }
                                }
                            }

                            // Check return type matches
                            if method.return_type != sig.return_type {
                                violations.push((
                                    format!(
                                        "Method '{}' in class '{}' has wrong return type (expected {:?}, got {:?})",
                                        method_name, class_name, sig.return_type, method.return_type
                                    ),
                                    format!(
                                        "الدالة '{}' في الصنف '{}' لديها نوع إرجاع خاطئ (متوقع {:?}، وجد {:?})",
                                        method_name, class_name, sig.return_type, method.return_type
                                    ),
                                    class.span,
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Add all violations to diagnostics
        for (msg, msg_ar, span) in violations {
            self.diagnostics
                .push(Diagnostic::error(&msg, &msg_ar, span));
        }
    }

    /// Check method override validity
    fn check_method_overrides(&mut self) {
        let mut violations = Vec::new();

        for (_class_name, class) in &self.classes {
            if let Some(parent_name) = &class.parent {
                if let Some(parent) = self.classes.get(parent_name) {
                    for (method_name, method) in &class.methods {
                        if let Some(parent_method) = parent.get_method(method_name, self) {
                            // Check visibility is not more restrictive
                            if method.visibility == Visibility::Private
                                && parent_method.visibility != Visibility::Private
                            {
                                violations.push((
                                    format!(
                                        "Cannot override {} method '{}' with private visibility",
                                        format_visibility(parent_method.visibility),
                                        method_name
                                    ),
                                    format!(
                                        "لا يمكن تجاوز الدالة {} '{}' بخصوصية خاصة",
                                        format_visibility_ar(parent_method.visibility),
                                        method_name
                                    ),
                                    class.span,
                                ));
                            }

                            // Check return type is compatible
                            if !method
                                .return_type
                                .is_compatible_with(&parent_method.return_type)
                            {
                                violations.push((
                                    format!(
                                        "Return type of '{}' is not compatible with parent",
                                        method_name
                                    ),
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

        for (msg, msg_ar, span) in violations {
            self.diagnostics
                .push(Diagnostic::error(&msg, &msg_ar, span));
        }
    }

    /// Check if a class is a subclass of another
    pub fn is_subclass(&self, class_name: &str, potential_parent: &str) -> bool {
        if class_name == potential_parent {
            return true;
        }

        if let Some(class) = self.classes.get(class_name) {
            if let Some(parent_name) = &class.parent {
                return self.is_subclass(parent_name, potential_parent);
            }
        }

        false
    }

    /// Check if a class implements an interface
    pub fn implements_interface(&self, class_name: &str, interface_name: &str) -> bool {
        if let Some(class) = self.classes.get(class_name) {
            // Check direct implementation
            if class.interfaces.contains(&interface_name.to_string()) {
                return true;
            }

            // Check parent
            if let Some(parent_name) = &class.parent {
                return self.implements_interface(parent_name, interface_name);
            }
        }

        false
    }

    /// Get all diagnostics
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl Default for ClassResolver {
    fn default() -> Self {
        Self::new()
    }
}

fn format_visibility(vis: Visibility) -> &'static str {
    match vis {
        Visibility::Public => "public",
        Visibility::Private => "private",
        Visibility::Protected => "protected",
    }
}

fn format_visibility_ar(vis: Visibility) -> &'static str {
    match vis {
        Visibility::Public => "العامة",
        Visibility::Private => "الخاصة",
        Visibility::Protected => "المحمية",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Span;

    #[test]
    fn test_class_registration() {
        let mut resolver = ClassResolver::new();
        resolver.register_class("شخص", None, &[], Span::empty());
        resolver.register_class("موظف", Some("شخص"), &[], Span::empty());

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
        resolver.register_class("أ", None, &[], Span::empty());
        resolver.register_class("ب", Some("أ"), &[], Span::empty());

        // Add a method to class أ
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

        // Add methods to class ب
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

        // Check vtables
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
        resolver.register_class("أ", Some("ب"), &[], Span::empty());
        resolver.register_class("ب", Some("أ"), &[], Span::empty());

        let result = resolver.validate();
        assert!(result.is_err());
    }
}
