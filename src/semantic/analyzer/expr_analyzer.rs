//! Expression analysis for the Tarqeem semantic analyzer.
//!
//! This module handles type inference and semantic analysis of expressions.

use super::super::generics::GenericParam;
use super::super::method_resolver::{MemberResolution, MethodResolver};
use super::super::scope::{Symbol, SymbolKind};
use super::super::types::Type;
use super::Analyzer;
use crate::error::codes::{
    ERR_CONST_ASSIGNMENT, ERR_NONSTATIC_VIA_CLASS, ERR_PRIVATE_ACCESS, ERR_PROPERTY_NOT_FOUND,
    ERR_PROTECTED_ACCESS, ERR_STATIC_VIA_INSTANCE, ERR_SUPER_OUTSIDE_CLASS, ERR_THIS_OUTSIDE_CLASS,
    ERR_TYPE_MISMATCH, ERR_UNDEFINED_CLASS, ERR_UNDEFINED_VARIABLE,
};
use crate::error::Span;
use crate::parser::*;

impl Analyzer {
    /// Analyze an expression and return its type.
    pub(crate) fn analyze_expr(&mut self, expr: &Expr) -> Type {
        self.infer_type(expr)
    }

    /// Infer the type of an expression.
    pub(crate) fn infer_type(&mut self, expr: &Expr) -> Type {
        match &expr.kind {
            ExprKind::Literal(lit) => match lit {
                Literal::Int(_) => Type::Int,
                Literal::Float(_) => Type::Float,
                Literal::String(_) => Type::String,
                Literal::Bool(_) => Type::Bool,
                Literal::Null => Type::Null,
            },

            ExprKind::Identifier(name) => {
                // Mark the symbol as used for unused variable warnings
                self.scope.mark_used(name);
                if let Some(symbol) = self.scope.lookup(name) {
                    symbol.ty.clone()
                } else {
                    let similar_names = self.find_similar_names(name, 3);
                    self.undefined_error(
                        "معرّف",
                        name,
                        expr.span,
                        &similar_names,
                        &ERR_UNDEFINED_VARIABLE.to_string(),
                    );
                    Type::Error
                }
            }

            ExprKind::Binary { left, op, right } => {
                self.infer_binary_expr(left, op, right, expr.span)
            }

            ExprKind::Unary { op, operand } => self.infer_unary_expr(op, operand, expr.span),

            ExprKind::Call { callee, args } => self.infer_call_expr(callee, args, expr.span),

            ExprKind::Member { object, property } => {
                let object_type = self.infer_type(object);
                let receiver_is_class = self.receiver_is_class_name(object);
                self.resolve_member_type(&object_type, property, expr.span, receiver_is_class)
            }

            ExprKind::Index { object, index } => self.infer_index_expr(object, index, expr.span),

            ExprKind::Assignment { target, value } => {
                self.infer_assignment_expr(target, value, expr.span)
            }

            ExprKind::CompoundAssignment {
                target,
                op: _,
                value,
            } => {
                self.infer_type(target);
                self.infer_type(value);
                self.infer_type(target)
            }

            ExprKind::Array(elements) => self.infer_array_expr(elements),

            ExprKind::Object(pairs) => self.infer_object_expr(pairs),

            ExprKind::Lambda { params, body } => self.infer_lambda_expr(params, body),

            ExprKind::New {
                class,
                type_args,
                args,
            } => self.infer_new_expr(class, type_args, args, expr.span),

            ExprKind::Await(inner) => self.infer_type(inner),

            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => self.infer_ternary_expr(condition, then_expr, else_expr, expr.span),

            ExprKind::Grouping(inner) => self.infer_type(inner),

            ExprKind::This => {
                if !self.scope.is_in_class() {
                    self.error_with_code(
                        "'هذا' يمكن استخدامها فقط داخل صنف",
                        expr.span,
                        &ERR_THIS_OUTSIDE_CLASS.to_string(),
                    );
                    Type::Error
                } else if let Some(ref class_name) = self.current_class {
                    Type::Class(class_name.clone())
                } else {
                    Type::Any
                }
            }

            ExprKind::Super => self.infer_super_expr(expr.span),

            ExprKind::EnumVariant {
                enum_name,
                variant_name,
                args,
                ..
            } => self.infer_enum_variant_expr(enum_name, variant_name, args, expr.span),
        }
    }

    /// Infer binary expression type.
    fn infer_binary_expr(&mut self, left: &Expr, op: &BinaryOp, right: &Expr, span: Span) -> Type {
        let left_type = self.infer_type(left);
        let right_type = self.infer_type(right);

        let op_str = match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Pow => "**",
            BinaryOp::Eq => "==",
            BinaryOp::NotEq => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
        };

        if let Some(result_type) = left_type.binary_result_type(op_str, &right_type) {
            result_type
        } else {
            self.error(
                &format!(
                    "لا يمكن تطبيق العامل '{}' على {} و {}",
                    op_str,
                    left_type.arabic_name(),
                    right_type.arabic_name()
                ),
                span,
            );
            Type::Error
        }
    }

    /// Infer unary expression type.
    fn infer_unary_expr(&mut self, op: &UnaryOp, operand: &Expr, span: Span) -> Type {
        let operand_type = self.infer_type(operand);

        let op_str = match op {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
            UnaryOp::PreInc | UnaryOp::PostInc => "++",
            UnaryOp::PreDec | UnaryOp::PostDec => "--",
        };

        if let Some(result_type) = operand_type.unary_result_type(op_str) {
            result_type
        } else {
            self.error(
                &format!(
                    "لا يمكن تطبيق العامل '{}' على {}",
                    op_str,
                    operand_type.arabic_name()
                ),
                span,
            );
            Type::Error
        }
    }

    /// Infer call expression type.
    fn infer_call_expr(&mut self, callee: &Expr, args: &[Expr], span: Span) -> Type {
        if matches!(callee.kind, ExprKind::Super) {
            return self.analyze_super_constructor_call(args, span);
        }

        let callee_type = self.infer_type(callee);

        match callee_type {
            Type::Function {
                params,
                return_type,
            } => {
                if args.len() != params.len() {
                    self.error(
                        &format!("متوقع {} معاملات، وُجد {}", params.len(), args.len()),
                        span,
                    );
                }

                for (i, (arg, param_type)) in args.iter().zip(params.iter()).enumerate() {
                    let arg_type = self.infer_type(arg);
                    if !self.is_assignable(&arg_type, param_type) {
                        self.type_mismatch_error(
                            param_type,
                            &arg_type,
                            arg.span,
                            &format!("المعامل {}", i + 1),
                            &ERR_TYPE_MISMATCH.to_string(),
                        );
                    }
                }

                *return_type
            }
            Type::Any => {
                for arg in args {
                    self.infer_type(arg);
                }
                Type::Any
            }
            _ => {
                self.error(
                    &format!("لا يمكن استدعاء نوع غير دالة {}", callee_type.arabic_name()),
                    callee.span,
                );
                Type::Error
            }
        }
    }

    /// Infer index expression type.
    fn infer_index_expr(&mut self, object: &Expr, index: &Expr, _span: Span) -> Type {
        let object_type = self.infer_type(object);
        let index_type = self.infer_type(index);

        match object_type {
            Type::Array(inner) => {
                if !index_type.is_compatible_with(&Type::Int) {
                    self.error("فهرس المصفوفة يجب أن يكون عدداً صحيحاً", index.span);
                }
                *inner
            }
            Type::Map(k, v) => {
                if !index_type.is_compatible_with(&k) {
                    self.type_mismatch_error(
                        &k,
                        &index_type,
                        index.span,
                        "مفتاح القاموس",
                        &ERR_TYPE_MISMATCH.to_string(),
                    );
                }
                *v
            }
            Type::String => {
                if !index_type.is_compatible_with(&Type::Int) {
                    self.error("فهرس النص يجب أن يكون عدداً صحيحاً", index.span);
                }
                Type::String
            }
            _ => {
                self.error(
                    &format!("لا يمكن الفهرسة في {}", object_type.arabic_name()),
                    object.span,
                );
                Type::Error
            }
        }
    }

    /// Infer assignment expression type.
    fn infer_assignment_expr(&mut self, target: &Expr, value: &Expr, _span: Span) -> Type {
        let value_type = self.infer_type(value);

        match &target.kind {
            ExprKind::Identifier(name) => {
                let symbol_info = self.scope.lookup(name).map(|s| (s.mutable, s.ty.clone()));

                if let Some((mutable, ty)) = symbol_info {
                    if !mutable {
                        self.error_with_code(
                            &format!("لا يمكن تعيين قيمة لمتغير ثابت '{}'", name),
                            target.span,
                            &ERR_CONST_ASSIGNMENT.to_string(),
                        );
                    }
                    if !self.is_assignable(&value_type, &ty) {
                        self.type_mismatch_error(
                            &ty,
                            &value_type,
                            value.span,
                            "التعيين",
                            &ERR_TYPE_MISMATCH.to_string(),
                        );
                    }
                } else {
                    let similar_names = self.find_similar_names(name, 3);
                    self.undefined_error(
                        "متغير",
                        name,
                        target.span,
                        &similar_names,
                        &ERR_UNDEFINED_VARIABLE.to_string(),
                    );
                }
            }
            ExprKind::Member { object, .. } | ExprKind::Index { object, .. } => {
                self.infer_type(object);
            }
            _ => {
                self.error("هدف تعيين غير صالح", target.span);
            }
        }

        value_type
    }

    /// Infer array expression type.
    fn infer_array_expr(&mut self, elements: &[Expr]) -> Type {
        if elements.is_empty() {
            if let Some(Type::Array(elem_ty)) = &self.expected_type {
                Type::Array(elem_ty.clone())
            } else {
                Type::Array(Box::new(Type::Unknown))
            }
        } else {
            // Fold to the widest type seen so far rather than anchoring on
            // the first element — otherwise `[جديد مربع()، جديد شكل()]`
            // (subclass listed before its ancestor) is rejected while the
            // reverse order compiles, purely because of element order.
            let mut widest_type = self.infer_type(&elements[0]);
            for elem in elements.iter().skip(1) {
                let elem_type = self.infer_type(elem);
                if self.is_assignable(&elem_type, &widest_type) {
                    // Already fits — keep the current widest type.
                } else if self.is_assignable(&widest_type, &elem_type) {
                    widest_type = elem_type;
                } else {
                    self.type_mismatch_error(
                        &widest_type,
                        &elem_type,
                        elem.span,
                        "عنصر المصفوفة",
                        &ERR_TYPE_MISMATCH.to_string(),
                    );
                }
            }
            Type::Array(Box::new(widest_type))
        }
    }

    /// Infer object expression type.
    fn infer_object_expr(&mut self, pairs: &[(String, Expr)]) -> Type {
        if pairs.is_empty() {
            if let Some(Type::Map(key_ty, val_ty)) = &self.expected_type {
                Type::Map(key_ty.clone(), val_ty.clone())
            } else {
                Type::Map(Box::new(Type::String), Box::new(Type::Any))
            }
        } else {
            // Same order-independent widening fold as infer_array_expr, so
            // an ancestor-typed value listed after a subclass value doesn't
            // spuriously fall back to `أي` just because of pair order.
            let mut widest_type = self.infer_type(&pairs[0].1);
            let mut all_compatible = true;
            for (_, value) in pairs.iter().skip(1) {
                let value_type = self.infer_type(value);
                if self.is_assignable(&value_type, &widest_type) {
                    // Already fits — keep the current widest type.
                } else if self.is_assignable(&widest_type, &value_type) {
                    widest_type = value_type;
                } else {
                    all_compatible = false;
                    break;
                }
            }

            if all_compatible {
                Type::Map(Box::new(Type::String), Box::new(widest_type))
            } else {
                Type::Map(Box::new(Type::String), Box::new(Type::Any))
            }
        }
    }

    /// Infer lambda expression type.
    fn infer_lambda_expr(&mut self, params: &[Param], body: &LambdaBody) -> Type {
        self.push_function_scope(Type::Any);

        let expected_param_types: Option<Vec<Type>> = match &self.expected_type {
            Some(Type::Function {
                params: expected_params,
                ..
            }) => Some(expected_params.clone()),
            _ => None,
        };

        let param_types: Vec<Type> = params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let ty = if let Some(type_ann) = &p.ty {
                    self.resolve_type(type_ann)
                } else {
                    expected_param_types
                        .as_ref()
                        .and_then(|expected| expected.get(i).cloned())
                        .unwrap_or(Type::Any)
                };
                self.scope
                    .define(Symbol::variable(&p.name, ty.clone(), false, p.span));
                ty
            })
            .collect();

        let return_type = match body {
            LambdaBody::Expr(expr) => self.infer_type(expr),
            LambdaBody::Block(block) => {
                for stmt in &block.statements {
                    self.analyze_stmt(stmt);
                }
                Type::Void
            }
        };

        self.pop_scope();

        Type::Function {
            params: param_types,
            return_type: Box::new(return_type),
        }
    }

    /// Infer new expression type.
    fn infer_new_expr(
        &mut self,
        class: &Expr,
        type_args: &[TypeAnnotation],
        args: &[Expr],
        span: Span,
    ) -> Type {
        let class_name = match &class.kind {
            ExprKind::Identifier(name) => name.clone(),
            _ => {
                self.error("تعبير جديد يتطلب اسم صنف", class.span);
                return Type::Error;
            }
        };

        let class_info = self.class_resolver.get_class(&class_name).cloned();

        if let Some(class_info) = class_info {
            if class_info.is_generic() {
                if type_args.is_empty() {
                    self.error(
                        &format!("الصنف المعمم '{}' يتطلب معاملات نوع", class_name),
                        span,
                    );
                } else if type_args.len() != class_info.type_params.len() {
                    self.error(
                        &format!(
                            "عدد خاطئ لمعاملات النوع: متوقع {}، وُجد {}",
                            class_info.type_params.len(),
                            type_args.len()
                        ),
                        span,
                    );
                } else {
                    let resolved_args: Vec<Type> =
                        type_args.iter().map(|ta| self.resolve_type(ta)).collect();

                    let params: Vec<GenericParam> = class_info
                        .type_params
                        .iter()
                        .map(|name| GenericParam::new(name.clone()))
                        .collect();

                    if let Some(context) =
                        self.generic_resolver
                            .instantiate(&params, &resolved_args, span)
                    {
                        drop(context);
                    }

                    let diagnostics = self.generic_resolver.take_diagnostics();
                    for diag in diagnostics {
                        self.diagnostics.push(diag);
                    }
                }
            } else if !type_args.is_empty() {
                self.error(
                    &format!("الصنف '{}' ليس معمماً لكن تم تقديم معاملات نوع", class_name),
                    span,
                );
            }

            if let Some(ref ctor) = class_info.constructor {
                let expected_params = &ctor.params;
                if args.len() != expected_params.len() {
                    self.error(
                        &format!(
                            "المنشئ يتوقع {} معاملات، وُجد {}",
                            expected_params.len(),
                            args.len()
                        ),
                        span,
                    );
                }

                for (i, (arg, (_, param_type))) in
                    args.iter().zip(expected_params.iter()).enumerate()
                {
                    let arg_type = self.infer_type(arg);
                    if !self.is_assignable(&arg_type, param_type) {
                        self.type_mismatch_error(
                            param_type,
                            &arg_type,
                            arg.span,
                            &format!("معامل المنشئ {}", i + 1),
                            &ERR_TYPE_MISMATCH.to_string(),
                        );
                    }
                }
            } else if !args.is_empty() {
                self.error(&format!("الصنف '{}' ليس له منشئ", class_name), span);
            }

            Type::Class(class_name)
        } else {
            // Try to find similar class names
            let all_classes = self.class_resolver.all_class_names();
            let similar: Vec<String> = all_classes
                .iter()
                .filter(|name| {
                    Self::levenshtein_distance(&class_name, name)
                        <= (class_name.chars().count() / 2 + 2)
                })
                .filter(|name| *name != &class_name)
                .take(3)
                .cloned()
                .collect();
            self.undefined_error(
                "صنف",
                &class_name,
                class.span,
                &similar,
                &ERR_UNDEFINED_CLASS.to_string(),
            );
            Type::Error
        }
    }

    /// Infer ternary expression type.
    fn infer_ternary_expr(
        &mut self,
        condition: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
        span: Span,
    ) -> Type {
        let cond_type = self.infer_type(condition);
        if !cond_type.is_compatible_with(&Type::Bool) {
            self.error("شرط العامل الثلاثي يجب أن يكون منطقياً", condition.span);
        }

        let then_type = self.infer_type(then_expr);
        let else_type = self.infer_type(else_expr);

        // Widen to whichever branch's type the other is assignable to
        // (rather than always taking then_type), so e.g. a مربع/شكل ternary
        // types as شكل — the supertype — not the more specific subtype one
        // branch happens to produce.
        let joined = if self.is_assignable(&then_type, &else_type) {
            else_type.clone()
        } else if self.is_assignable(&else_type, &then_type) {
            then_type.clone()
        } else {
            self.error(
                &format!(
                    "فروع العامل الثلاثي غير متوافقة: {} و {}",
                    then_type.arabic_name(),
                    else_type.arabic_name()
                ),
                span,
            );
            return Type::Error;
        };

        // `is_assignable` unwraps `?` in either direction (a plain value fits
        // an Optional slot, and vice versa), so it alone can't tell "join is
        // Optional" from "join is plain" — pick whichever branch happens to
        // satisfy the assignability check first, which may drop the other
        // branch's nullability. Re-add it explicitly: if either branch could
        // be لا_شيء, the ternary as a whole can still evaluate to لا_شيء.
        let either_optional =
            matches!(then_type, Type::Optional(_)) || matches!(else_type, Type::Optional(_));
        if either_optional && !matches!(joined, Type::Optional(_)) {
            Type::Optional(Box::new(joined))
        } else {
            joined
        }
    }

    /// Infer super expression type.
    fn infer_super_expr(&mut self, span: Span) -> Type {
        if !self.scope.is_in_class() {
            self.error_with_code(
                "'الأصل' يمكن استخدامها فقط داخل صنف",
                span,
                &ERR_SUPER_OUTSIDE_CLASS.to_string(),
            );
            Type::Error
        } else if let Some(ref class_name) = self.current_class {
            if let Some(class) = self.class_resolver.get_class(class_name) {
                if let Some(ref parent_name) = class.parent {
                    Type::Class(parent_name.clone())
                } else {
                    self.error("لا يمكن استخدام 'الأصل' في صنف بدون أب", span);
                    Type::Error
                }
            } else {
                Type::Any
            }
        } else {
            Type::Any
        }
    }

    /// Infer enum variant expression type.
    fn infer_enum_variant_expr(
        &mut self,
        enum_name: &str,
        variant_name: &str,
        args: &[Expr],
        span: Span,
    ) -> Type {
        // Analyze all arguments first
        let arg_types: Vec<Type> = args.iter().map(|a| self.analyze_expr(a)).collect();

        // Look up the enum in our registry
        if let Some(enum_info) = self.enums.get(enum_name).cloned() {
            // Find the variant
            if let Some(variant) = enum_info.variants.iter().find(|v| v.name == variant_name) {
                // Check argument count matches
                if args.len() != variant.fields.len() {
                    self.error(
                        &format!(
                            "الحالة '{}::{}' تتوقع {} معامل(ات)، وُجد {}",
                            enum_name,
                            variant_name,
                            variant.fields.len(),
                            args.len()
                        ),
                        span,
                    );
                } else {
                    // Check argument types match
                    for (i, (arg_ty, expected_ty)) in
                        arg_types.iter().zip(&variant.fields).enumerate()
                    {
                        if !self.is_assignable(arg_ty, expected_ty) {
                            self.type_mismatch_error(
                                expected_ty,
                                arg_ty,
                                args[i].span,
                                &format!(
                                    "معامل {} للحالة '{}::{}'",
                                    i + 1,
                                    enum_name,
                                    variant_name
                                ),
                                &ERR_TYPE_MISMATCH.to_string(),
                            );
                        }
                    }
                }
                Type::Enum(enum_name.to_string())
            } else {
                // Variant not found
                self.error(
                    &format!(
                        "الحالة '{}' غير موجودة في التعداد '{}'",
                        variant_name, enum_name
                    ),
                    span,
                );
                Type::Error
            }
        } else {
            // Enum not found - might be defined later or doesn't exist
            // For now, just return the type and let it be resolved later
            Type::Enum(enum_name.to_string())
        }
    }

    /// Resolve member type from an object.
    /// Is `object` a bare identifier naming a declared class (`اسم_الصنف.عضو`
    /// form), as opposed to an instance? Used to reject `ClassName.member`
    /// for non-static members and `instance.member` for static ones —
    /// otherwise a `مشترك` member would silently behave like an instance one
    /// (or vice versa) instead of raising a clear diagnostic.
    fn receiver_is_class_name(&mut self, object: &Expr) -> bool {
        matches!(&object.kind, ExprKind::Identifier(name)
            if self.scope.lookup(name).map(|s| s.kind == SymbolKind::Class).unwrap_or(false))
    }

    pub(crate) fn resolve_member_type(
        &mut self,
        object_type: &Type,
        property: &str,
        span: Span,
        receiver_is_class: bool,
    ) -> Type {
        let mut method_resolver = MethodResolver::new(&self.class_resolver);

        match method_resolver.resolve_member(object_type, property) {
            MemberResolution::Field {
                field,
                defining_class,
            } => {
                if !self.check_static_access(receiver_is_class, field.is_static, property, span) {
                    return Type::Error;
                }
                // Check visibility for field access using the defining class
                if !self.check_member_visibility(&defining_class, field.visibility, property, span)
                {
                    return Type::Error;
                }
                field.ty.clone()
            }
            MemberResolution::Method {
                method,
                defining_class,
            } => {
                if !self.check_static_access(receiver_is_class, method.is_static, property, span) {
                    return Type::Error;
                }
                // Check visibility for method access using the defining class
                if !self.check_member_visibility(&defining_class, method.visibility, property, span)
                {
                    return Type::Error;
                }
                Type::Function {
                    params: method.params.iter().map(|(_, ty)| ty.clone()).collect(),
                    return_type: Box::new(method.return_type.clone()),
                }
            }
            MemberResolution::BuiltinProperty { ty, .. } => ty,
            MemberResolution::NotFound => {
                if let Type::Class(class_name) = object_type {
                    if self.class_resolver.get_class(class_name).is_some() {
                        self.error_with_code(
                            &format!(
                                "الخاصية '{}' غير موجودة في الصنف '{}'",
                                property, class_name
                            ),
                            span,
                            &ERR_PROPERTY_NOT_FOUND.to_string(),
                        );
                    }
                }
                Type::Any
            }
        }
    }

    /// Rejects the mismatched access forms: a non-static member reached
    /// through a class name (`ClassName.field`), or a static (`مشترك`)
    /// member reached through an instance (`instance.staticField`). Checked
    /// before visibility, since "wrong access form" is the more actionable
    /// diagnostic when both would otherwise apply.
    fn check_static_access(
        &mut self,
        receiver_is_class: bool,
        member_is_static: bool,
        property: &str,
        span: Span,
    ) -> bool {
        if receiver_is_class && !member_is_static {
            self.error_with_code(
                &format!(
                    "لا يمكن الوصول للعضو '{}' عبر اسم الصنف؛ العضو ليس مشتركاً (مشترك)",
                    property
                ),
                span,
                &ERR_NONSTATIC_VIA_CLASS.to_string(),
            );
            return false;
        }
        if !receiver_is_class && member_is_static {
            self.error_with_code(
                &format!("العضو '{}' مشترك؛ يُستخدم عبر اسم الصنف مباشرة", property),
                span,
                &ERR_STATIC_VIA_INSTANCE.to_string(),
            );
            return false;
        }
        true
    }

    /// Check if member access is allowed based on visibility rules.
    ///
    /// Visibility rules:
    /// - `عام` (Public): Accessible everywhere
    /// - `خاص` (Private): Accessible only within the same class
    /// - `محمي` (Protected): Accessible in the class and its subclasses
    ///
    /// Returns `true` if access is allowed, `false` if denied (and reports error).
    fn check_member_visibility(
        &mut self,
        member_class: &str,
        visibility: Visibility,
        member_name: &str,
        span: Span,
    ) -> bool {
        match visibility {
            Visibility::Public => true,
            Visibility::Private => {
                // Private: only accessible within the same class
                if let Some(ref current_class) = self.current_class {
                    if current_class == member_class {
                        return true;
                    }
                }
                self.error_with_code(
                    &format!(
                        "لا يمكن الوصول للعضو الخاص '{}' من الصنف '{}'",
                        member_name, member_class
                    ),
                    span,
                    &ERR_PRIVATE_ACCESS.to_string(),
                );
                false
            }
            Visibility::Protected => {
                // Protected: accessible in the class and its subclasses
                if let Some(ref current_class) = self.current_class {
                    // Same class - allowed
                    if current_class == member_class {
                        return true;
                    }
                    // Subclass - allowed
                    if self.class_resolver.is_subclass(current_class, member_class) {
                        return true;
                    }
                }
                self.error_with_code(
                    &format!(
                        "لا يمكن الوصول للعضو المحمي '{}' من الصنف '{}' من خارج تسلسله الهرمي",
                        member_name, member_class
                    ),
                    span,
                    &ERR_PROTECTED_ACCESS.to_string(),
                );
                false
            }
        }
    }
}
