//! Expression analysis for the Tarqeem semantic analyzer.
//!
//! This module handles type inference and semantic analysis of expressions.

use super::super::generics::GenericParam;
use super::super::method_resolver::{MemberResolution, MethodResolver};
use super::super::scope::Symbol;
use super::super::types::Type;
use super::Analyzer;
use crate::error::codes::{ERR_PRIVATE_ACCESS, ERR_PROPERTY_NOT_FOUND, ERR_PROTECTED_ACCESS};
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
                    use crate::error::codes::ERR_UNDEFINED_VARIABLE;
                    let similar_names = self.find_similar_names(name, 3);
                    self.undefined_error(
                        "identifier",
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
                self.resolve_member_type(&object_type, property, expr.span)
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
                    use crate::error::codes::ERR_THIS_OUTSIDE_CLASS;
                    self.error_with_code(
                        "'this' can only be used inside a class",
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
                    "Cannot apply operator '{}' to {} and {}",
                    op_str, left_type, right_type
                ),
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
                &format!("Cannot apply operator '{}' to {}", op_str, operand_type),
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
                        &format!("Expected {} arguments, got {}", params.len(), args.len()),
                        &format!("متوقع {} معاملات، وُجد {}", params.len(), args.len()),
                        span,
                    );
                }

                for (i, (arg, param_type)) in args.iter().zip(params.iter()).enumerate() {
                    let arg_type = self.infer_type(arg);
                    if !arg_type.is_compatible_with(param_type) {
                        use crate::error::codes::ERR_TYPE_MISMATCH;
                        self.type_mismatch_error(
                            param_type,
                            &arg_type,
                            arg.span,
                            &format!("argument {}", i + 1),
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
                    &format!("Cannot call non-function type {}", callee_type),
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
                    self.error(
                        "Array index must be an integer",
                        "فهرس المصفوفة يجب أن يكون عدداً صحيحاً",
                        index.span,
                    );
                }
                *inner
            }
            Type::Map(k, v) => {
                if !index_type.is_compatible_with(&k) {
                    use crate::error::codes::ERR_TYPE_MISMATCH;
                    self.type_mismatch_error(
                        &k,
                        &index_type,
                        index.span,
                        "map key",
                        "مفتاح القاموس",
                        &ERR_TYPE_MISMATCH.to_string(),
                    );
                }
                *v
            }
            Type::String => {
                if !index_type.is_compatible_with(&Type::Int) {
                    self.error(
                        "String index must be an integer",
                        "فهرس النص يجب أن يكون عدداً صحيحاً",
                        index.span,
                    );
                }
                Type::String
            }
            _ => {
                self.error(
                    &format!("Cannot index into {}", object_type),
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
                        use crate::error::codes::ERR_CONST_ASSIGNMENT;
                        self.error_with_code(
                            &format!("Cannot assign to immutable variable '{}'", name),
                            &format!("لا يمكن تعيين قيمة لمتغير ثابت '{}'", name),
                            target.span,
                            &ERR_CONST_ASSIGNMENT.to_string(),
                        );
                    }
                    if !value_type.is_compatible_with(&ty) {
                        use crate::error::codes::ERR_TYPE_MISMATCH;
                        self.type_mismatch_error(
                            &ty,
                            &value_type,
                            value.span,
                            "assignment",
                            "التعيين",
                            &ERR_TYPE_MISMATCH.to_string(),
                        );
                    }
                } else {
                    use crate::error::codes::ERR_UNDEFINED_VARIABLE;
                    let similar_names = self.find_similar_names(name, 3);
                    self.undefined_error(
                        "variable",
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
                self.error(
                    "Invalid assignment target",
                    "هدف تعيين غير صالح",
                    target.span,
                );
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
            let first_type = self.infer_type(&elements[0]);
            for elem in elements.iter().skip(1) {
                let elem_type = self.infer_type(elem);
                if !elem_type.is_compatible_with(&first_type) {
                    use crate::error::codes::ERR_TYPE_MISMATCH;
                    self.type_mismatch_error(
                        &first_type,
                        &elem_type,
                        elem.span,
                        "array element",
                        "عنصر المصفوفة",
                        &ERR_TYPE_MISMATCH.to_string(),
                    );
                }
            }
            Type::Array(Box::new(first_type))
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
            let first_value_type = self.infer_type(&pairs[0].1);

            let mut all_same = true;
            for (_, value) in pairs.iter().skip(1) {
                let value_type = self.infer_type(value);
                if !value_type.is_compatible_with(&first_value_type) {
                    all_same = false;
                    break;
                }
            }

            if all_same {
                Type::Map(Box::new(Type::String), Box::new(first_value_type))
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
                self.error(
                    "New expression requires a class name",
                    "تعبير جديد يتطلب اسم صنف",
                    class.span,
                );
                return Type::Error;
            }
        };

        let class_info = self.class_resolver.get_class(&class_name).cloned();

        if let Some(class_info) = class_info {
            if class_info.is_generic() {
                if type_args.is_empty() {
                    self.error(
                        &format!("Generic class '{}' requires type arguments", class_name),
                        &format!("الصنف المعمم '{}' يتطلب معاملات نوع", class_name),
                        span,
                    );
                } else if type_args.len() != class_info.type_params.len() {
                    self.error(
                        &format!(
                            "Wrong number of type arguments: expected {}, got {}",
                            class_info.type_params.len(),
                            type_args.len()
                        ),
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
                    &format!(
                        "Class '{}' is not generic but type arguments were provided",
                        class_name
                    ),
                    &format!("الصنف '{}' ليس معمماً لكن تم تقديم معاملات نوع", class_name),
                    span,
                );
            }

            if let Some(ref ctor) = class_info.constructor {
                let expected_params = &ctor.params;
                if args.len() != expected_params.len() {
                    self.error(
                        &format!(
                            "Constructor expects {} arguments, got {}",
                            expected_params.len(),
                            args.len()
                        ),
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
                    if !arg_type.is_compatible_with(param_type) {
                        use crate::error::codes::ERR_TYPE_MISMATCH;
                        self.type_mismatch_error(
                            param_type,
                            &arg_type,
                            arg.span,
                            &format!("constructor argument {}", i + 1),
                            &format!("معامل المنشئ {}", i + 1),
                            &ERR_TYPE_MISMATCH.to_string(),
                        );
                    }
                }
            } else if !args.is_empty() {
                self.error(
                    &format!("Class '{}' has no constructor", class_name),
                    &format!("الصنف '{}' ليس له منشئ", class_name),
                    span,
                );
            }

            Type::Class(class_name)
        } else {
            // Try to find similar class names
            use crate::error::codes::ERR_UNDEFINED_CLASS;
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
                "class",
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
            self.error(
                "Ternary condition must be boolean",
                "شرط العامل الثلاثي يجب أن يكون منطقياً",
                condition.span,
            );
        }

        let then_type = self.infer_type(then_expr);
        let else_type = self.infer_type(else_expr);

        if then_type.is_compatible_with(&else_type) {
            then_type
        } else {
            self.error(
                &format!(
                    "Ternary branches have incompatible types: {} and {}",
                    then_type, else_type
                ),
                &format!(
                    "فروع العامل الثلاثي غير متوافقة: {} و {}",
                    then_type.arabic_name(),
                    else_type.arabic_name()
                ),
                span,
            );
            Type::Error
        }
    }

    /// Infer super expression type.
    fn infer_super_expr(&mut self, span: Span) -> Type {
        if !self.scope.is_in_class() {
            use crate::error::codes::ERR_SUPER_OUTSIDE_CLASS;
            self.error_with_code(
                "'super' can only be used inside a class",
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
                    self.error(
                        "Cannot use 'super' in a class without a parent",
                        "لا يمكن استخدام 'الأصل' في صنف بدون أب",
                        span,
                    );
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
                            "Variant '{}::{}' expects {} argument(s), got {}",
                            enum_name,
                            variant_name,
                            variant.fields.len(),
                            args.len()
                        ),
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
                        if !arg_ty.is_compatible_with(expected_ty) {
                            use crate::error::codes::ERR_TYPE_MISMATCH;
                            self.type_mismatch_error(
                                expected_ty,
                                arg_ty,
                                args[i].span,
                                &format!(
                                    "enum variant '{}::{}' argument {}",
                                    enum_name,
                                    variant_name,
                                    i + 1
                                ),
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
                        "Variant '{}' not found in enum '{}'",
                        variant_name, enum_name
                    ),
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
    pub(crate) fn resolve_member_type(
        &mut self,
        object_type: &Type,
        property: &str,
        span: Span,
    ) -> Type {
        let mut method_resolver = MethodResolver::new(&self.class_resolver);

        match method_resolver.resolve_member(object_type, property) {
            MemberResolution::Field {
                field,
                defining_class,
            } => {
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
                                "Property '{}' not found on class '{}'",
                                property, class_name
                            ),
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
                        "Cannot access private member '{}' of class '{}'",
                        member_name, member_class
                    ),
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
                        "Cannot access protected member '{}' of class '{}' from outside its hierarchy",
                        member_name, member_class
                    ),
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
