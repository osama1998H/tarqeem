//! Type conversion and inference helpers for the IR builder.
//!
//! This module provides utilities for converting AST types to IR types
//! and inferring expression types.

use crate::parser::{
    BinaryOp as AstBinaryOp, Expr, ExprKind, Literal, Param, TypeAnnotation, TypeKind,
};

use super::super::{ClassId, Constant, IrType};
use super::IrBuilder;

impl IrBuilder {
    /// Convert an AST type annotation to an IR type.
    pub(crate) fn convert_type(&self, ty: &TypeAnnotation) -> IrType {
        match &ty.kind {
            TypeKind::Simple(name) => self.convert_simple_type(name),
            TypeKind::Array(inner) => IrType::Array(Box::new(self.convert_type(inner)), 0),
            TypeKind::Map(_key, _value) => IrType::Ptr(Box::new(IrType::Void)),
            TypeKind::Function {
                params,
                return_type,
            } => IrType::Function {
                params: params.iter().map(|p| self.convert_type(p)).collect(),
                // Bare `()` (no return type) lowers to Void — the same
                // idiom `build_func_decl` uses for `دالة` with no `-> نوع`.
                ret: Box::new(
                    return_type
                        .as_ref()
                        .map(|t| self.convert_type(t))
                        .unwrap_or(IrType::Void),
                ),
            },
            TypeKind::Generic { base, args } => match base.as_str() {
                "مصفوفة" | "array" | "Array" => {
                    if let Some(elem_type) = args.first() {
                        IrType::Array(Box::new(self.convert_type(elem_type)), 0)
                    } else {
                        IrType::Array(Box::new(IrType::Ptr(Box::new(IrType::Void))), 0)
                    }
                }
                "قاموس" | "map" | "Map" | "dict" | "Dict" => {
                    IrType::Ptr(Box::new(IrType::Void))
                }
                _ => self.convert_simple_type(base),
            },
            TypeKind::Optional(inner) => IrType::Ptr(Box::new(self.convert_type(inner))),
        }
    }

    /// Convert a simple type name to an IR type.
    pub(crate) fn convert_simple_type(&self, name: &str) -> IrType {
        match name {
            "عدد" => IrType::Int,
            "عدد_عشري" => IrType::Float,
            "نص" => IrType::String,
            "منطقي" => IrType::Bool,
            _ => IrType::Struct(ClassId(name.to_string())),
        }
    }

    /// Convert a semantic type to an IR type.
    #[allow(dead_code)]
    pub(crate) fn semantic_to_ir_type(&self, ty: &crate::semantic::Type) -> IrType {
        use crate::semantic::Type as SemanticType;
        match ty {
            SemanticType::Int => IrType::Int,
            SemanticType::Float => IrType::Float,
            SemanticType::String => IrType::String,
            SemanticType::Bool => IrType::Bool,
            SemanticType::Void => IrType::Void,
            SemanticType::Null => IrType::Ptr(Box::new(IrType::Void)),
            SemanticType::Array(inner) => {
                IrType::Array(Box::new(self.semantic_to_ir_type(inner)), 0)
            }
            SemanticType::Class(name) => IrType::Struct(ClassId(name.clone())),
            SemanticType::Function {
                params,
                return_type,
            } => IrType::Function {
                params: params.iter().map(|p| self.semantic_to_ir_type(p)).collect(),
                ret: Box::new(self.semantic_to_ir_type(return_type)),
            },
            _ => IrType::Ptr(Box::new(IrType::Void)),
        }
    }

    /// Try to evaluate an expression as a compile-time constant.
    pub(crate) fn try_evaluate_const(&mut self, expr: &Expr) -> Option<Constant> {
        match &expr.kind {
            ExprKind::Literal(lit) => match lit {
                Literal::Int(i) => Some(Constant::Int(*i)),
                Literal::Float(f) => Some(Constant::Float(*f)),
                Literal::String(s) => {
                    let idx = self.add_string(s.clone());
                    Some(Constant::String(idx))
                }
                Literal::Bool(b) => Some(Constant::Bool(*b)),
                Literal::Null => Some(Constant::Null),
            },
            ExprKind::Unary { op, operand } => {
                use crate::parser::UnaryOp as AstUnaryOp;
                let val = self.try_evaluate_const(operand)?;
                match (op, val) {
                    (AstUnaryOp::Neg, Constant::Int(i)) => Some(Constant::Int(-i)),
                    (AstUnaryOp::Neg, Constant::Float(f)) => Some(Constant::Float(-f)),
                    (AstUnaryOp::Not, Constant::Bool(b)) => Some(Constant::Bool(!b)),
                    _ => None,
                }
            }
            ExprKind::Binary { left, op, right } => {
                let left_val = self.try_evaluate_const(left)?;
                let right_val = self.try_evaluate_const(right)?;
                match (left_val, op, right_val) {
                    (Constant::Int(a), AstBinaryOp::Add, Constant::Int(b)) => {
                        Some(Constant::Int(a + b))
                    }
                    (Constant::Int(a), AstBinaryOp::Sub, Constant::Int(b)) => {
                        Some(Constant::Int(a - b))
                    }
                    (Constant::Int(a), AstBinaryOp::Mul, Constant::Int(b)) => {
                        Some(Constant::Int(a * b))
                    }
                    (Constant::Int(a), AstBinaryOp::Div, Constant::Int(b)) if b != 0 => {
                        Some(Constant::Int(a / b))
                    }
                    (Constant::Float(a), AstBinaryOp::Add, Constant::Float(b)) => {
                        Some(Constant::Float(a + b))
                    }
                    (Constant::Float(a), AstBinaryOp::Sub, Constant::Float(b)) => {
                        Some(Constant::Float(a - b))
                    }
                    (Constant::Float(a), AstBinaryOp::Mul, Constant::Float(b)) => {
                        Some(Constant::Float(a * b))
                    }
                    (Constant::Float(a), AstBinaryOp::Div, Constant::Float(b)) if b != 0.0 => {
                        Some(Constant::Float(a / b))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Get the IR type for a constant value.
    pub(crate) fn const_to_type(&self, constant: &Constant) -> IrType {
        match constant {
            Constant::Int(_) => IrType::Int,
            Constant::Float(_) => IrType::Float,
            Constant::Bool(_) => IrType::Bool,
            Constant::String(_) => IrType::String,
            Constant::Null => IrType::Ptr(Box::new(IrType::Void)),
            // A function value is never itself the result of const-folding
            // another expression (this function evaluates compile-time
            // constant *expressions*, and `Constant::Function` is only ever
            // produced directly by `build_lambda`/`build_identifier`), so
            // this arm exists purely for exhaustiveness.
            Constant::Function(_) => IrType::Ptr(Box::new(IrType::Void)),
        }
    }

    /// Infer the IR type of an expression.
    pub(crate) fn infer_expr_type(&self, expr: &Expr) -> IrType {
        match &expr.kind {
            ExprKind::Literal(lit) => match lit {
                Literal::Int(_) => IrType::Int,
                Literal::Float(_) => IrType::Float,
                Literal::String(_) => IrType::String,
                Literal::Bool(_) => IrType::Bool,
                Literal::Null => IrType::Ptr(Box::new(IrType::Void)),
            },
            ExprKind::Array(elements) => {
                let elem_ty = if let Some(first) = elements.first() {
                    self.infer_expr_type(first)
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                };
                IrType::Array(Box::new(elem_ty), elements.len())
            }
            ExprKind::Binary { op, left, right } => match op {
                AstBinaryOp::Eq
                | AstBinaryOp::NotEq
                | AstBinaryOp::Lt
                | AstBinaryOp::LtEq
                | AstBinaryOp::Gt
                | AstBinaryOp::GtEq
                | AstBinaryOp::And
                | AstBinaryOp::Or => IrType::Bool,
                AstBinaryOp::Add => {
                    let left_ty = self.infer_expr_type(left);
                    let right_ty = self.infer_expr_type(right);
                    if matches!(left_ty, IrType::String) || matches!(right_ty, IrType::String) {
                        IrType::String
                    } else if matches!(left_ty, IrType::Float) || matches!(right_ty, IrType::Float)
                    {
                        IrType::Float
                    } else {
                        IrType::Int
                    }
                }
                AstBinaryOp::Sub | AstBinaryOp::Mul | AstBinaryOp::Div | AstBinaryOp::Mod => {
                    let left_ty = self.infer_expr_type(left);
                    let right_ty = self.infer_expr_type(right);
                    if matches!(left_ty, IrType::Float) || matches!(right_ty, IrType::Float) {
                        IrType::Float
                    } else {
                        IrType::Int
                    }
                }
                _ => IrType::Int, // Default for other operations
            },
            ExprKind::Unary { op, operand } => {
                use crate::parser::UnaryOp as AstUnaryOp;
                match op {
                    AstUnaryOp::Not => IrType::Bool,
                    AstUnaryOp::Neg
                    | AstUnaryOp::PreInc
                    | AstUnaryOp::PreDec
                    | AstUnaryOp::PostInc
                    | AstUnaryOp::PostDec => {
                        let operand_ty = self.infer_expr_type(operand);
                        match operand_ty {
                            IrType::Float => IrType::Float,
                            _ => IrType::Int,
                        }
                    }
                }
            }
            ExprKind::New { class, .. } => {
                if let ExprKind::Identifier(name) = &class.kind {
                    IrType::Ptr(Box::new(IrType::Struct(ClassId(
                        self.resolve_class_name(name),
                    ))))
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Identifier(name) => {
                if let Some(ptr) = self.lookup_var(name) {
                    self.var_types
                        .get(&ptr.0)
                        .cloned()
                        .unwrap_or(IrType::Ptr(Box::new(IrType::Void)))
                } else if let Some(global_ty) = self.global_var_types.get(name).cloned() {
                    global_ty
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Index { object, .. } => {
                let obj_ty = self.infer_expr_type(object);
                if let IrType::Array(elem, _) = obj_ty {
                    (*elem).clone()
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Member { object, property } => {
                if let Some(class) = self.class_name_receiver(object) {
                    if let Some((_, ty)) = self.resolve_static_field(&class, property) {
                        return ty;
                    }
                    if let Some((_, ty)) = self.resolve_static_property(&class, property) {
                        return ty;
                    }
                    return IrType::Ptr(Box::new(IrType::Void));
                }
                let obj_ty = self.infer_expr_type(object);
                if let IrType::Struct(class_id) = obj_ty {
                    // A property is read through its accessor, and its backing
                    // field is named `_{prop}` — so the field lookup below
                    // cannot see it under the name the source used.
                    if let Some((_, _, ty)) = self.resolve_instance_property(&class_id.0, property)
                    {
                        return ty;
                    }
                    self.get_field_type(&class_id.0, property)
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Call { callee, .. } => {
                if let ExprKind::Identifier(name) = &callee.kind {
                    // A call *through a function value* (a lambda or a named
                    // function held in a variable) returns that signature's
                    // `ret`, not the global function table's entry — there is
                    // no declared function of this name to look up. Missing
                    // this leaves the result slot typed `Ptr(Void)`, so a
                    // later `اطبع` natively emits `trq_print(ptr %x)` on what
                    // is really an integer and dereferences it.
                    if let Some(IrType::Function { ret, .. }) = self.callee_value_type(name) {
                        return (*ret).clone();
                    }
                    self.get_function_return_type(name)
                } else if let ExprKind::Member { object, property } = &callee.kind {
                    if let Some(class) = self.class_name_receiver(object) {
                        if let Some(full) = self.resolve_static_method(&class, property) {
                            return self
                                .method_return_types
                                .get(&full)
                                .cloned()
                                .unwrap_or(IrType::Void);
                        }
                    }
                    IrType::Ptr(Box::new(IrType::Void))
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Ternary { then_expr, .. } => self.infer_expr_type(then_expr),
            ExprKind::This => {
                if let Some(var_id) = self.lookup_var("هذا").or_else(|| self.lookup_var("this"))
                {
                    self.var_types
                        .get(&var_id.0)
                        .cloned()
                        .unwrap_or(IrType::Ptr(Box::new(IrType::Void)))
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Super => {
                if let Some(ref func) = self.current_function {
                    if let Some(idx) = func.name.find("::") {
                        let current_class_name = &func.name[..idx];
                        if let Some(parent_class_id) = self
                            .module
                            .classes
                            .iter()
                            .find(|c| c.name == current_class_name)
                            .and_then(|c| c.parent.as_ref())
                        {
                            return IrType::Ptr(Box::new(IrType::Struct(parent_class_id.clone())));
                        }
                    }
                }
                if let Some(var_id) = self.lookup_var("هذا").or_else(|| self.lookup_var("this"))
                {
                    self.var_types
                        .get(&var_id.0)
                        .cloned()
                        .unwrap_or(IrType::Ptr(Box::new(IrType::Void)))
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            _ => IrType::Ptr(Box::new(IrType::Void)),
        }
    }

    pub(crate) fn untyped_param_reason(param_name: &str) -> String {
        format!(
            "المعامل '{}' بدون نوع محدد (النوع 'أي' لا يكفي للترجمة الأصلية)",
            param_name
        )
    }

    /// Names of `params` that never resolved to a concrete type, so native
    /// codegen cannot pick an ABI for them: unannotated (and not covered by
    /// a contextual `hint`), or explicitly annotated `أي`. The interpreter is
    /// dynamically typed and unaffected — this only gates `tarqeem compile`.
    pub(crate) fn unlowerable_param_names(
        &self,
        params: &[Param],
        hint_params: &[IrType],
    ) -> Vec<String> {
        params
            .iter()
            .enumerate()
            .filter(|(i, p)| match &p.ty {
                None => hint_params.get(*i).is_none(),
                Some(ann) => matches!(&ann.kind, TypeKind::Simple(n) if n == "أي" || n == "اي"),
            })
            .map(|(_, p)| p.name.clone())
            .collect()
    }

    /// The recorded type of `name` when it is a local or global *variable*
    /// holding a value (as opposed to a declared function). Used to tell a
    /// call through a function value apart from a direct call.
    pub(crate) fn callee_value_type(&self, name: &str) -> Option<IrType> {
        if let Some(ptr) = self.lookup_var(name) {
            return self.var_types.get(&ptr.0).cloned();
        }
        self.global_var_types.get(name).cloned()
    }

    /// Get the return type of a function by name.
    pub(crate) fn get_function_return_type(&self, name: &str) -> IrType {
        if let Some(ret_ty) = self.function_return_types.get(name) {
            return ret_ty.clone();
        }

        for func in &self.module.functions {
            if func.name == name || func.id.0 == name {
                return func.return_type.clone();
            }
        }

        IrType::Ptr(Box::new(IrType::Void))
    }

    /// Get the type of a field in a class, or of any field it inherits.
    ///
    /// Walks the parent chain: an inherited field that resolved to `Ptr(Void)`
    /// here is how `اطبع(كائن.حقل_موروث)` ended up emitting `trq_print(ptr %x)`
    /// against an integer (issue #249).
    pub(crate) fn get_field_type(&self, class_name: &str, field_name: &str) -> IrType {
        self.resolve_instance_field(class_name, field_name)
            .map(|(_, _, ty)| ty)
            .unwrap_or(IrType::Ptr(Box::new(IrType::Void)))
    }

    /// Get field information (index and type) for a class field.
    pub(crate) fn get_field_info(
        &self,
        class_name: &str,
        field_name: &str,
    ) -> Option<(u32, IrType)> {
        if let Some(fields) = self.class_fields.get(class_name) {
            for (idx, (name, ty)) in fields.iter().enumerate() {
                if name == field_name {
                    return Some((idx as u32, ty.clone()));
                }
            }
        }
        None
    }

    /// Is `expr` a bare identifier naming a declared class, used as a
    /// namespace (`ClassName.member`)? Returns `None` if the identifier is
    /// shadowed by a local/global/function of the same name, mirroring the
    /// shadowing precedence `build_identifier` already uses (locals, then
    /// globals/functions, only then anything else).
    pub(crate) fn class_name_receiver(&self, expr: &Expr) -> Option<String> {
        let ExprKind::Identifier(name) = &expr.kind else {
            return None;
        };
        if self.lookup_var(name).is_some()
            || self.global_variables.contains(name)
            || self.global_constants.contains_key(name)
            || self.function_names.contains(name)
        {
            return None;
        }
        self.class_names.contains(name).then(|| name.clone())
    }

    /// Walk `class` up through `class_parents` looking for a `مشترك` field
    /// or static-property backing field, returning the *defining* class's
    /// global key so every subclass shares one storage slot.
    pub(crate) fn resolve_static_field(
        &self,
        class: &str,
        member: &str,
    ) -> Option<(String, IrType)> {
        let mut current = Some(class.to_string());
        let mut visited = std::collections::HashSet::new();
        while let Some(c) = current {
            if !visited.insert(c.clone()) {
                break; // cyclic inheritance is rejected by semantic analysis; don't hang here
            }
            let key = format!("{}::{}", c, member);
            if let Some(ty) = self.static_field_types.get(&key) {
                return Some((key, ty.clone()));
            }
            current = self.class_parents.get(&c).cloned();
        }
        None
    }

    /// Walk `class` up through `class_parents` looking for a `مشترك` method,
    /// returning the defining class's mangled function name.
    pub(crate) fn resolve_static_method(&self, class: &str, member: &str) -> Option<String> {
        let mut current = Some(class.to_string());
        let mut visited = std::collections::HashSet::new();
        while let Some(c) = current {
            if !visited.insert(c.clone()) {
                break;
            }
            let key = format!("{}::{}", c, member);
            if self.static_methods.contains(&key) {
                return Some(key);
            }
            current = self.class_parents.get(&c).cloned();
        }
        None
    }

    /// Walk `class` up through `class_parents` looking for a `مشترك خاصية`,
    /// returning the defining class's getter function name and its type.
    pub(crate) fn resolve_static_property(
        &self,
        class: &str,
        member: &str,
    ) -> Option<(String, IrType)> {
        let mut current = Some(class.to_string());
        let mut visited = std::collections::HashSet::new();
        while let Some(c) = current {
            if !visited.insert(c.clone()) {
                break;
            }
            let key = format!("{}::{}", c, member);
            if self.static_properties.contains(&key) {
                if let Some((getter_name, ty)) = self.property_getters.get(&key) {
                    return Some((getter_name.clone(), ty.clone()));
                }
            }
            current = self.class_parents.get(&c).cloned();
        }
        None
    }

    /// Same as `resolve_static_property`, but for the setter side of an
    /// assignment target.
    pub(crate) fn resolve_static_property_setter(
        &self,
        class: &str,
        member: &str,
    ) -> Option<String> {
        let mut current = Some(class.to_string());
        let mut visited = std::collections::HashSet::new();
        while let Some(c) = current {
            if !visited.insert(c.clone()) {
                break;
            }
            let key = format!("{}::{}", c, member);
            if self.static_properties.contains(&key) {
                if let Some(setter_name) = self.property_setters.get(&key) {
                    return Some(setter_name.clone());
                }
            }
            current = self.class_parents.get(&c).cloned();
        }
        None
    }

    /// Walk `class` up through `class_parents` looking for an instance field —
    /// a plain `عام`/`خاص`/`محمي` field, or an auto-property's `_`-prefixed
    /// backing field — and return the *defining* class alongside the slot's
    /// index **within that class**.
    ///
    /// `get_field_info` searches one class's own fields only, and
    /// `collect_class` never merges the parent chain into `class_fields`, so an
    /// inherited member misses there. Callers used to read that miss as "index
    /// 0, type `ptr`, owned by the receiver" — three wrong values at once, which
    /// codegen faithfully turned into an out-of-bounds GEP or a `trq_print(ptr)`
    /// on an integer (issue #249). Naming the definer is what keeps codegen's
    /// `inherited_field_count[definer] + index` correct: with single
    /// inheritance the flattened layout is `[ancestors…, own…]`, so a field
    /// declared in `D` at own-index `i` sits at that same offset in *every*
    /// subclass of `D`.
    pub(crate) fn resolve_instance_field(
        &self,
        class: &str,
        member: &str,
    ) -> Option<(String, u32, IrType)> {
        let mut current = Some(class.to_string());
        let mut visited = std::collections::HashSet::new();
        while let Some(c) = current {
            if !visited.insert(c.clone()) {
                break; // cyclic inheritance is rejected by semantic analysis; don't hang here
            }
            if let Some((index, ty)) = self.get_field_info(&c, member) {
                return Some((c, index, ty));
            }
            current = self.class_parents.get(&c).cloned();
        }
        None
    }

    /// Walk `class` up through `class_parents` looking for a non-`مشترك`
    /// `خاصية`, returning the defining class, its getter's bare method name,
    /// and the property's type.
    ///
    /// The class is returned separately because `MethodId` carries it, and it
    /// must be the *definer*: both backends mint the callee symbol from
    /// `MethodId.class` — natively in `CallMethod`'s static bind, interpreted in
    /// `resolve_virtual_method`'s fallback — and `{subclass}::__احصل_{prop}` is
    /// never synthesized. `property_getters` holds static and instance
    /// properties alike, so `static_properties` is what separates them; the
    /// static side has its own resolver above and reaches storage through
    /// globals rather than a receiver.
    pub(crate) fn resolve_instance_property(
        &self,
        class: &str,
        member: &str,
    ) -> Option<(ClassId, String, IrType)> {
        let mut current = Some(class.to_string());
        let mut visited = std::collections::HashSet::new();
        while let Some(c) = current {
            if !visited.insert(c.clone()) {
                break;
            }
            let key = format!("{}::{}", c, member);
            if !self.static_properties.contains(&key) {
                if let Some((getter_name, ty)) = self.property_getters.get(&key) {
                    return Some((ClassId(c), bare_method_name(getter_name), ty.clone()));
                }
            }
            current = self.class_parents.get(&c).cloned();
        }
        None
    }

    /// Same as `resolve_instance_property`, but for the setter side of an
    /// assignment target.
    pub(crate) fn resolve_instance_property_setter(
        &self,
        class: &str,
        member: &str,
    ) -> Option<(ClassId, String)> {
        let mut current = Some(class.to_string());
        let mut visited = std::collections::HashSet::new();
        while let Some(c) = current {
            if !visited.insert(c.clone()) {
                break;
            }
            let key = format!("{}::{}", c, member);
            if !self.static_properties.contains(&key) {
                if let Some(setter_name) = self.property_setters.get(&key) {
                    return Some((ClassId(c), bare_method_name(setter_name)));
                }
            }
            current = self.class_parents.get(&c).cloned();
        }
        None
    }

    /// Does the IR builder know a field layout for `class`? False for `أي`-typed
    /// and unresolved receivers, and for `__anonymous__` object literals, whose
    /// fields codegen resolves by name and which `collect_class` therefore never
    /// registers. Field resolution stays lenient for those; a *declared* class
    /// missing one of its own members is an internal invariant violation.
    pub(crate) fn has_field_layout(&self, class: &str) -> bool {
        self.class_fields.contains_key(class)
    }
}

/// Accessors are registered as `{Class}::{method}` but `MethodId` names the
/// method alone, with the class travelling in `MethodId.class`.
fn bare_method_name(qualified: &str) -> String {
    qualified
        .rsplit("::")
        .next()
        .unwrap_or(qualified)
        .to_string()
}
