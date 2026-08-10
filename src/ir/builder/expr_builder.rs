//! Expression building for the IR builder.
//!
//! This module handles conversion of AST expressions to IR instructions.

use crate::parser::{
    BinaryOp as AstBinaryOp, Expr, ExprKind, LambdaBody, Literal, Param, UnaryOp as AstUnaryOp,
};

use super::super::{
    BinaryOp, ClassId, Constant, EnumId, FieldId, FuncId, Instruction, IrType, MethodId, UnaryOp,
    VarId, VariantId,
};
use super::{IrBuilder, IrError, Result};

use unicode_normalization::UnicodeNormalization;

/// `الأصل.method()` must stay bound to the parent's implementation: since
/// `الأصل` resolves to the same `هذا` value, dispatching it dynamically on
/// the runtime object would call straight back into the overriding method
/// and recurse forever.
fn is_super_receiver(object: &Expr) -> bool {
    matches!(object.kind, ExprKind::Super)
}

impl IrBuilder {
    /// Build IR for an expression.
    pub(crate) fn build_expr(&mut self, expr: &Expr) -> Result<VarId> {
        match &expr.kind {
            ExprKind::Literal(lit) => self.build_literal(lit),
            ExprKind::Identifier(name) => self.build_identifier(name),
            ExprKind::Binary { left, op, right } => self.build_binary(left, *op, right),
            ExprKind::Unary { op, operand } => self.build_unary(*op, operand),
            ExprKind::Call { callee, args } => self.build_call(callee, args),
            ExprKind::Member { object, property } => self.build_member(object, property),
            ExprKind::Index { object, index } => self.build_index(object, index),
            ExprKind::Assignment { target, value } => self.build_assignment(target, value),
            ExprKind::CompoundAssignment { target, op, value } => {
                self.build_compound_assignment(target, *op, value)
            }
            ExprKind::Array(elements) => self.build_array(elements),
            ExprKind::Object(fields) => self.build_object(fields),
            ExprKind::Lambda { params, body } => self.build_lambda(params, body),
            ExprKind::New { class, args, .. } => self.build_new(class, args),
            ExprKind::Await(inner) => self.build_await(inner),
            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => self.build_ternary(condition, then_expr, else_expr),
            ExprKind::Grouping(inner) => self.build_expr(inner),
            ExprKind::This => self.build_this(),
            ExprKind::Super => self.build_super(),
            ExprKind::EnumVariant {
                enum_name,
                variant_name,
                args,
                ..
            } => self.build_enum_variant(enum_name, variant_name, args),
        }
    }

    /// Build IR for an enum variant instantiation.
    pub(crate) fn build_enum_variant(
        &mut self,
        enum_name: &str,
        variant_name: &str,
        args: &[Expr],
    ) -> Result<VarId> {
        // Build all field values first
        let mut field_vars = Vec::new();
        for arg in args {
            let var_id = self.build_expr(arg)?;
            field_vars.push(var_id);
        }

        // Create the variant ID with NFC-normalized names for Arabic consistency
        // Use FNV-1a hash of variant name as discriminant for consistent matching
        let normalized_enum = Self::normalize_name(enum_name);
        let normalized_variant = Self::normalize_name(variant_name);
        let variant_id = VariantId {
            enum_id: EnumId(normalized_enum),
            name: normalized_variant.clone(),
            discriminant: Self::calculate_discriminant(&normalized_variant),
        };

        // Create the enum value
        let dest = self.new_var();
        self.emit(Instruction::NewEnumVariant {
            dest,
            variant: variant_id,
            fields: field_vars,
        });

        Ok(dest)
    }

    /// Build IR for a literal value.
    pub(crate) fn build_literal(&mut self, lit: &Literal) -> Result<VarId> {
        let dest = self.new_var();
        let (value, ty) = match lit {
            Literal::Int(i) => (Constant::Int(*i), IrType::Int),
            Literal::Float(f) => (Constant::Float(*f), IrType::Float),
            Literal::String(s) => {
                let idx = self.add_string(s.clone());
                (Constant::String(idx), IrType::String)
            }
            Literal::Bool(b) => (Constant::Bool(*b), IrType::Bool),
            Literal::Null => (Constant::Null, IrType::Ptr(Box::new(IrType::Void))),
        };

        self.var_types.insert(dest.0, ty.clone());

        self.emit(Instruction::Const { dest, value, ty });
        Ok(dest)
    }

    /// Does a real declaration — a local, a global or a function — already
    /// claim `name`?
    ///
    /// Import naming (a wildcard namespace or a named-import alias) is
    /// compile-time-only sugar layered over the AST the linker merged, so any
    /// such declaration in the importing file must win over it. Guarding on
    /// locals alone let `استورد { ضاعف كـ اضعف }` silently hijack a `دالة
    /// اضعف` declared in the importing file — no diagnostic, just the wrong
    /// function called.
    fn is_declared_name(&self, name: &str) -> bool {
        self.lookup_var(name).is_some()
            || self.global_variables.contains(name)
            || self.function_names.contains(name)
    }

    /// Is `expr` a bare identifier naming a wildcard-import namespace
    /// (`استورد * كـ رياض`)? A declaration of the same name shadows it,
    /// mirroring the precedence `class_name_receiver` already applies to
    /// `ClassName.member`.
    fn is_namespace_receiver(&self, expr: &Expr) -> bool {
        let ExprKind::Identifier(name) = &expr.kind else {
            return false;
        };
        !self.is_declared_name(name) && self.namespace_aliases.contains(name)
    }

    /// The bare name a named-import alias redirects to (`اضعف` → `ضاعف`), or
    /// `None` when `name` is not an alias or a declaration of its own shadows
    /// it.
    fn alias_target(&self, name: &str) -> Option<String> {
        if self.is_declared_name(name) {
            return None;
        }
        self.import_aliases.get(name).cloned()
    }

    /// The class a `جديد <name>` refers to. `استورد { نقطة كـ إحداثية }` merges
    /// the class under `نقطة`, so the alias has to reach it; a class of the
    /// alias's own name shadows the import.
    ///
    /// Both the instantiation site and `infer_expr_type` must agree, or the
    /// object is allocated as one struct and its fields read through another —
    /// which codegen emits as a `getelementptr` on an undefined LLVM type.
    pub(super) fn resolve_class_name(&self, name: &str) -> String {
        if self.class_names.contains(name) {
            return name.to_string();
        }
        self.alias_target(name).unwrap_or_else(|| name.to_string())
    }

    /// Rewrites a reference that names an import binding to the bare name the
    /// linker merged the declaration under: a wildcard-namespace member
    /// (`رياض.جذر` → `جذر`) or a named-import alias (`اضعف` → `ضاعف`).
    /// `None` means `expr` is not such a reference and must be built as
    /// written.
    ///
    /// Resolution is a single hop, deliberately never chased transitively:
    /// two files that alias each other's names (`{ أ كـ ب }` in one and
    /// `{ ب كـ أ }` in the other) would otherwise loop the builder forever.
    fn resolve_import_ref(&self, expr: &Expr) -> Option<Expr> {
        let resolved = match &expr.kind {
            ExprKind::Member { object, property } if self.is_namespace_receiver(object) => {
                property.clone()
            }
            ExprKind::Identifier(name) => self.alias_target(name)?,
            _ => return None,
        };
        Some(Expr::new(ExprKind::Identifier(resolved), expr.span))
    }

    /// Build IR for an identifier reference.
    pub(crate) fn build_identifier(&mut self, name: &str) -> Result<VarId> {
        // An aliased named import declares nothing of its own: `استورد
        // { ضاعف كـ اضعف }` leaves the body merged under `ضاعف`, so the alias
        // has to reach it. A declaration of the same name still shadows the
        // import, matching the semantic analyzer's scoping.
        let alias_target = self.alias_target(name);
        let name = alias_target.as_deref().unwrap_or(name);

        if let Some(var_id) = self.lookup_var(name) {
            if self.parameters.contains(&var_id.0) {
                return Ok(var_id);
            }

            let var_type = self
                .var_types
                .get(&var_id.0)
                .cloned()
                .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));

            let dest = self.new_var();
            self.emit(Instruction::Load {
                dest,
                ptr: var_id,
                ty: var_type.clone(),
            });

            self.var_types.insert(dest.0, var_type);

            Ok(dest)
        } else if self.function_names.contains(name) {
            // A bare reference to a declared function's name used as a
            // value (e.g. `ثابت ف = مربع؛`) — mirrors build_lambda's
            // Constant::Function so a named function is just as usable as a
            // lambda via CallIndirect (issue #180).
            let params = self
                .function_param_types
                .get(name)
                .cloned()
                .unwrap_or_default();
            let ret = self
                .function_return_types
                .get(name)
                .cloned()
                .unwrap_or(IrType::Void);
            let fn_ty = IrType::Function {
                params,
                ret: Box::new(ret),
            };
            let dest = self.new_var();
            self.emit(Instruction::Const {
                dest,
                value: Constant::Function(name.to_string()),
                ty: fn_ty.clone(),
            });
            self.var_types.insert(dest.0, fn_ty);
            Ok(dest)
        } else if let Some((const_val, const_ty)) = self.global_constants.get(name).cloned() {
            let dest = self.new_var();
            self.emit(Instruction::Const {
                dest,
                value: const_val,
                ty: const_ty.clone(),
            });
            self.var_types.insert(dest.0, const_ty);
            Ok(dest)
        } else if let Some(var_ty) = self.global_var_types.get(name).cloned() {
            let dest = self.new_var();
            self.emit(Instruction::GlobalLoad {
                dest,
                name: name.to_string(),
                ty: var_ty.clone(),
            });
            self.var_types.insert(dest.0, var_ty);
            Ok(dest)
        } else {
            Err(IrError::new(format!("معرّف غير معرّف: '{}'", name)))
        }
    }

    /// Build IR for a binary operation.
    pub(crate) fn build_binary(
        &mut self,
        left: &Expr,
        op: AstBinaryOp,
        right: &Expr,
    ) -> Result<VarId> {
        let left_var = self.build_expr(left)?;
        let right_var = self.build_expr(right)?;

        let left_ty = self
            .var_types
            .get(&left_var.0)
            .cloned()
            .unwrap_or(IrType::Int);
        let right_ty = self
            .var_types
            .get(&right_var.0)
            .cloned()
            .unwrap_or(IrType::Int);

        if matches!(op, AstBinaryOp::Add) {
            let is_left_string = matches!(left_ty, IrType::String);
            let is_right_string = matches!(right_ty, IrType::String);

            if is_left_string || is_right_string {
                let left_str = if is_left_string {
                    left_var
                } else {
                    self.convert_to_string(left_var, &left_ty)?
                };

                let right_str = if is_right_string {
                    right_var
                } else {
                    self.convert_to_string(right_var, &right_ty)?
                };

                let dest = self.new_var();
                self.emit(Instruction::StringConcat {
                    dest,
                    left: left_str,
                    right: right_str,
                });
                self.var_types.insert(dest.0, IrType::String);
                return Ok(dest);
            }
        }

        let ir_op = match op {
            AstBinaryOp::Add => BinaryOp::Add,
            AstBinaryOp::Sub => BinaryOp::Sub,
            AstBinaryOp::Mul => BinaryOp::Mul,
            AstBinaryOp::Div => BinaryOp::Div,
            AstBinaryOp::Mod => BinaryOp::Mod,
            AstBinaryOp::Pow => BinaryOp::Pow,
            AstBinaryOp::Eq => BinaryOp::Eq,
            AstBinaryOp::NotEq => BinaryOp::Ne,
            AstBinaryOp::Lt => BinaryOp::Lt,
            AstBinaryOp::LtEq => BinaryOp::Le,
            AstBinaryOp::Gt => BinaryOp::Gt,
            AstBinaryOp::GtEq => BinaryOp::Ge,
            AstBinaryOp::And => BinaryOp::And,
            AstBinaryOp::Or => BinaryOp::Or,
        };

        let result_ty = match ir_op {
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::And
            | BinaryOp::Or => IrType::Bool,
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::Pow => {
                if matches!(left_ty, IrType::Float) || matches!(right_ty, IrType::Float) {
                    IrType::Float
                } else {
                    IrType::Int
                }
            }
            _ => IrType::Int,
        };

        let dest = self.new_var();
        self.emit(Instruction::Binary {
            dest,
            op: ir_op,
            left: left_var,
            right: right_var,
            ty: result_ty.clone(),
        });

        self.var_types.insert(dest.0, result_ty);

        Ok(dest)
    }

    /// Convert a value to string for concatenation.
    pub(crate) fn convert_to_string(&mut self, var: VarId, ty: &IrType) -> Result<VarId> {
        let dest = self.new_var();
        let func_name = match ty {
            IrType::Int => "trq_int_to_string".to_string(),
            IrType::Float => "trq_float_to_string".to_string(),
            IrType::Bool => "trq_bool_to_string".to_string(),
            _ => "trq_int_to_string".to_string(), // Default fallback
        };

        self.emit(Instruction::Call {
            dest: Some(dest),
            func: FuncId(func_name),
            args: vec![var],
            ret_ty: IrType::String,
        });
        self.var_types.insert(dest.0, IrType::String);
        Ok(dest)
    }

    /// Build IR for a unary operation.
    pub(crate) fn build_unary(&mut self, op: AstUnaryOp, operand: &Expr) -> Result<VarId> {
        match op {
            AstUnaryOp::Neg => {
                let operand_type = self.infer_expr_type(operand);
                let operand_var = self.build_expr(operand)?;

                let result_ty = match operand_type {
                    IrType::Float => IrType::Float,
                    _ => IrType::Int,
                };
                let dest = self.new_var();
                self.emit(Instruction::Unary {
                    dest,
                    op: UnaryOp::Neg,
                    operand: operand_var,
                    ty: result_ty.clone(),
                });
                self.var_types.insert(dest.0, result_ty);
                Ok(dest)
            }
            AstUnaryOp::Not => {
                let operand_var = self.build_expr(operand)?;
                let dest = self.new_var();
                self.emit(Instruction::Unary {
                    dest,
                    op: UnaryOp::Not,
                    operand: operand_var,
                    ty: IrType::Bool,
                });
                self.var_types.insert(dest.0, IrType::Bool);
                Ok(dest)
            }
            AstUnaryOp::PreInc => self.build_increment(operand, true, true),
            AstUnaryOp::PreDec => self.build_increment(operand, false, true),
            AstUnaryOp::PostInc => self.build_increment(operand, true, false),
            AstUnaryOp::PostDec => self.build_increment(operand, false, false),
        }
    }

    /// Build IR for increment/decrement operations.
    pub(crate) fn build_increment(
        &mut self,
        operand: &Expr,
        is_increment: bool,
        is_prefix: bool,
    ) -> Result<VarId> {
        // `ع++` on an import binding names the merged declaration, exactly as
        // `ع = ع + ١` and `ع += ١` already do — without this rewrite the alias
        // reached the read-modify-write path unresolved and failed there.
        let resolved_operand = self.resolve_import_ref(operand);
        let operand = resolved_operand.as_ref().unwrap_or(operand);

        let name = match &operand.kind {
            ExprKind::Identifier(name) => name.clone(),
            _ => return Err(IrError::new("الزيادة/النقصان تتطلب متغيراً")),
        };

        // Store the lookup result to avoid redundant lookups and unwrap() calls
        let local_ptr = self.lookup_var(&name);
        let is_global = self.global_variables.contains(&name);

        if local_ptr.is_none() && !is_global {
            return Err(IrError::new(format!(
                "لا يمكن تعديل متغير غير معرّف '{}'",
                name
            )));
        }

        let result_ty = if let Some(ptr) = local_ptr {
            let var_type = self.var_types.get(&ptr.0).cloned().unwrap_or(IrType::Int);
            match var_type {
                IrType::Float => IrType::Float,
                _ => IrType::Int,
            }
        } else {
            let var_type = self
                .global_var_types
                .get(&name)
                .cloned()
                .unwrap_or(IrType::Int);
            match var_type {
                IrType::Float => IrType::Float,
                _ => IrType::Int,
            }
        };

        let old_val = self.new_var();
        if let Some(ptr) = local_ptr {
            self.emit(Instruction::Load {
                dest: old_val,
                ptr,
                ty: result_ty.clone(),
            });
        } else {
            self.emit(Instruction::GlobalLoad {
                dest: old_val,
                name: name.clone(),
                ty: result_ty.clone(),
            });
        }
        self.var_types.insert(old_val.0, result_ty.clone());

        let one = self.new_var();
        let const_val = if matches!(result_ty, IrType::Float) {
            Constant::Float(1.0)
        } else {
            Constant::Int(1)
        };
        self.emit(Instruction::Const {
            dest: one,
            value: const_val,
            ty: result_ty.clone(),
        });
        self.var_types.insert(one.0, result_ty.clone());

        let new_val = self.new_var();
        let op = if is_increment {
            BinaryOp::Add
        } else {
            BinaryOp::Sub
        };
        self.emit(Instruction::Binary {
            dest: new_val,
            op,
            left: old_val,
            right: one,
            ty: result_ty.clone(),
        });
        self.var_types.insert(new_val.0, result_ty);

        if let Some(ptr) = local_ptr {
            self.emit(Instruction::Store {
                ptr,
                value: new_val,
            });
        } else {
            self.emit(Instruction::GlobalStore {
                name: name.clone(),
                value: new_val,
            });
        }

        Ok(if is_prefix { new_val } else { old_val })
    }

    /// Build IR for a function/method call.
    pub(crate) fn build_call(&mut self, callee: &Expr, args: &[Expr]) -> Result<VarId> {
        if matches!(callee.kind, ExprKind::Super) {
            return self.build_super_constructor_call(args);
        }

        // Import naming is resolved before any dispatch decision, so
        // `رياض.جذر(...)` becomes a plain call to `جذر` rather than a method
        // dispatch, and an alias calls the original name — the linker merged
        // every imported declaration under its bare, unmangled name. This
        // deliberately runs before the `ClassName.member` static-call case,
        // so a namespace wins over a class of the same name.
        let resolved_callee = self.resolve_import_ref(callee);
        let callee = resolved_callee.as_ref().unwrap_or(callee);

        // Callee parameter types, resolved *before* the arguments are
        // built, so a lambda literal passed as an argument picks up its
        // param types from the callee's signature — the same contextual
        // inference the semantic layer performs via `expected_type`.
        // Without this, `طبق((س) => س * ٢، ٥)` lifts the lambda with
        // untyped (`Ptr(Void)`) params and native compilation rejects
        // spec-legal code with ت٠٣٠١.
        let expected_params: Option<Vec<IrType>> = match &callee.kind {
            ExprKind::Identifier(name) => self.callee_param_types(name),
            _ => None,
        };

        let arg_vars = self.build_call_args(args, expected_params.as_deref())?;
        let arg_vars = match expected_params.as_deref() {
            Some(params) => self.coerce_args_to_params(arg_vars, params),
            None => arg_vars,
        };

        if let ExprKind::Identifier(name) = &callee.kind {
            if name == "اطبع" {
                if let Some(arg) = arg_vars.first() {
                    self.emit(Instruction::Print { value: *arg });
                }
                let dest = self.new_var();
                self.emit(Instruction::Const {
                    dest,
                    value: Constant::Null,
                    ty: IrType::Void,
                });
                self.var_types.insert(dest.0, IrType::Void);
                return Ok(dest);
            }

            // Handle طول (length) as a function call - generates ArrayLen instruction
            if name == "طول" {
                if let Some(array_var) = arg_vars.first() {
                    let dest = self.new_var();
                    self.emit(Instruction::ArrayLen {
                        dest,
                        array: *array_var,
                    });
                    self.var_types.insert(dest.0, IrType::Int);
                    return Ok(dest);
                }
            }

            // Handle نص (to-string conversion) with type dispatch
            // This function accepts Type::Any but needs to dispatch to the correct
            // runtime function based on the actual argument type
            if name == "نص" {
                if let Some(arg_var) = arg_vars.first() {
                    let arg_ty = self
                        .var_types
                        .get(&arg_var.0)
                        .cloned()
                        .unwrap_or(IrType::Int);
                    return self.convert_to_string(*arg_var, &arg_ty);
                }
            }

            // `name` may be a local/global variable holding a function
            // value (a lambda, or a named function used as a value) rather
            // than a directly-callable declared function — dispatch through
            // CallIndirect in that case (issue #180). An untyped local still
            // counts as a function value as long as no *declared* function
            // of the same name exists, preserving today's behavior when a
            // local merely shadows an unrelated function name.
            let local_var = self.lookup_var(name);
            let local_ty = local_var.and_then(|v| self.var_types.get(&v.0).cloned());
            let is_callable_value = if local_var.is_some() {
                matches!(local_ty, Some(IrType::Function { .. }))
                    || !self.function_names.contains(name)
            } else if self.global_variables.contains(name) {
                let global_ty = self.global_var_types.get(name).cloned();
                matches!(global_ty, Some(IrType::Function { .. }))
                    || !self.function_names.contains(name)
            } else {
                false
            };

            if is_callable_value {
                let callee_var = self.build_identifier(name)?;
                let ret_ty = match self.var_types.get(&callee_var.0) {
                    Some(IrType::Function { ret, .. }) => (**ret).clone(),
                    _ => IrType::Ptr(Box::new(IrType::Void)),
                };
                let dest = self.new_var();
                self.emit(Instruction::CallIndirect {
                    dest: Some(dest),
                    func_ptr: callee_var,
                    args: arg_vars,
                    ret_ty: ret_ty.clone(),
                });
                self.var_types.insert(dest.0, ret_ty);
                return Ok(dest);
            }

            let ret_ty = self.get_function_return_type(name);

            let dest = self.new_var();
            self.emit(Instruction::Call {
                dest: Some(dest),
                func: FuncId(name.clone()),
                args: arg_vars,
                ret_ty: ret_ty.clone(),
            });
            self.var_types.insert(dest.0, ret_ty);
            return Ok(dest);
        }

        if let ExprKind::Member { object, property } = &callee.kind {
            if let Some(class) = self.class_name_receiver(object) {
                if let Some(full) = self.resolve_static_method(&class, property) {
                    let ret_ty = self
                        .method_return_types
                        .get(&full)
                        .cloned()
                        .unwrap_or(IrType::Void);
                    let dest = self.new_var();
                    self.emit(Instruction::Call {
                        dest: Some(dest),
                        func: FuncId(full),
                        args: arg_vars,
                        ret_ty: ret_ty.clone(),
                    });
                    self.var_types.insert(dest.0, ret_ty);
                    return Ok(dest);
                }
                return Err(IrError::new(format!(
                    "الدالة '{}' ليست دالة مشتركة في الصنف '{}'",
                    property, class
                )));
            }

            let obj_type = self.infer_expr_type(object);
            let obj_var = self.build_expr(object)?;

            let is_array = match &obj_type {
                IrType::Array(_, _) => true,
                IrType::Ptr(inner) => matches!(inner.as_ref(), IrType::Array(_, _) | IrType::Void),
                _ => false,
            };

            if is_array {
                match property.as_str() {
                    "ألحق" | "أضف" => {
                        if let Some(value_var) = arg_vars.first() {
                            let elem_ty = match &obj_type {
                                IrType::Array(inner, _) => (**inner).clone(),
                                IrType::Ptr(inner) => match inner.as_ref() {
                                    IrType::Array(elem, _) => (**elem).clone(),
                                    _ => self
                                        .var_types
                                        .get(&value_var.0)
                                        .cloned()
                                        .unwrap_or(IrType::Int),
                                },
                                _ => self
                                    .var_types
                                    .get(&value_var.0)
                                    .cloned()
                                    .unwrap_or(IrType::Int),
                            };
                            self.emit(Instruction::ArrayPush {
                                array: obj_var,
                                value: *value_var,
                                elem_ty,
                            });
                            self.var_types.insert(obj_var.0, obj_type);
                            return Ok(obj_var);
                        }
                    }
                    "طول" => {
                        let dest = self.new_var();
                        self.emit(Instruction::ArrayLen {
                            dest,
                            array: obj_var,
                        });
                        self.var_types.insert(dest.0, IrType::Int);
                        return Ok(dest);
                    }
                    _ => {}
                }
            }

            let class_id = match &obj_type {
                IrType::Struct(class_id) => class_id.clone(),
                IrType::Ptr(inner) => {
                    if let IrType::Struct(class_id) = inner.as_ref() {
                        class_id.clone()
                    } else {
                        ClassId("".to_string())
                    }
                }
                _ => ClassId("".to_string()),
            };

            let full_method_name = format!("{}::{}", class_id.0, property);
            let ret_ty = self
                .method_return_types
                .get(&full_method_name)
                .cloned()
                .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));

            let dest = self.new_var();
            self.emit(Instruction::CallMethod {
                dest: Some(dest),
                object: obj_var,
                method: MethodId {
                    class: class_id,
                    name: property.clone(),
                },
                args: arg_vars,
                ret_ty: ret_ty.clone(),
                virtual_dispatch: !is_super_receiver(object),
            });
            self.var_types.insert(dest.0, ret_ty);
            return Ok(dest);
        }

        let callee_var = self.build_expr(callee)?;
        let (arg_vars, ret_ty) = match self.var_types.get(&callee_var.0).cloned() {
            Some(IrType::Function { params, ret }) => (
                self.coerce_args_to_params(arg_vars, &params),
                (*ret).clone(),
            ),
            _ => (arg_vars, IrType::Ptr(Box::new(IrType::Void))),
        };
        let dest = self.new_var();
        self.emit(Instruction::CallIndirect {
            dest: Some(dest),
            func_ptr: callee_var,
            args: arg_vars,
            ret_ty: ret_ty.clone(),
        });
        self.var_types.insert(dest.0, ret_ty);
        Ok(dest)
    }

    /// The callee's parameter types when statically known — from a
    /// function-typed local/global variable (indirect call) or a declared
    /// function's collected signature (direct call). `None` when the callee
    /// has no recoverable signature (builtins, untyped values).
    fn callee_param_types(&self, name: &str) -> Option<Vec<IrType>> {
        if let Some(v) = self.lookup_var(name) {
            return match self.var_types.get(&v.0) {
                Some(IrType::Function { params, .. }) => Some(params.clone()),
                // An untyped local shadowing a declared function name still
                // dispatches to the declared function (see build_call's
                // shadowing rule), so its signature applies.
                _ if self.function_names.contains(name) => {
                    self.function_param_types.get(name).cloned()
                }
                _ => None,
            };
        }
        if let Some(IrType::Function { params, .. }) = self.global_var_types.get(name) {
            return Some(params.clone());
        }
        self.function_param_types.get(name).cloned()
    }

    /// Builds call arguments, threading the callee's parameter type as a
    /// hint into any bare lambda literal argument (mirrors what
    /// `build_init_expr` does for annotated variable declarations).
    fn build_call_args(
        &mut self,
        args: &[Expr],
        expected: Option<&[IrType]>,
    ) -> Result<Vec<VarId>> {
        args.iter()
            .enumerate()
            .map(|(i, a)| {
                if let ExprKind::Lambda { params, body } = &a.kind {
                    if let Some(hint @ IrType::Function { .. }) = expected.and_then(|e| e.get(i)) {
                        return self.build_lambda_with_hint(params, body, Some(hint));
                    }
                }
                self.build_expr(a)
            })
            .collect()
    }

    /// Implicit عدد → عدد_عشري coercion (spec §5.6) at call arguments:
    /// passing an integer where the callee's signature expects a float must
    /// go through `IntToFloat`, or native codegen emits a call whose raw
    /// i64 bit pattern the callee reinterprets as a double.
    fn coerce_args_to_params(&mut self, arg_vars: Vec<VarId>, params: &[IrType]) -> Vec<VarId> {
        arg_vars
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                if params.get(i) == Some(&IrType::Float)
                    && self.var_types.get(&v.0) == Some(&IrType::Int)
                {
                    let coerced = self.new_var();
                    self.emit(Instruction::IntToFloat {
                        dest: coerced,
                        src: v,
                    });
                    self.var_types.insert(coerced.0, IrType::Float);
                    coerced
                } else {
                    v
                }
            })
            .collect()
    }

    /// Build IR for a member access.
    pub(crate) fn build_member(&mut self, object: &Expr, property: &str) -> Result<VarId> {
        // A wildcard namespace holds no value to read a field out of:
        // `رياض.ط` is the bare `ط` the linker merged (or, for stdlib, the
        // builtin already registered under that bare name).
        if self.is_namespace_receiver(object) {
            return self.build_identifier(property);
        }

        if let Some(class) = self.class_name_receiver(object) {
            if let Some((key, ty)) = self.resolve_static_field(&class, property) {
                let dest = self.new_var();
                self.emit(Instruction::GlobalLoad {
                    dest,
                    name: key,
                    ty: ty.clone(),
                });
                self.var_types.insert(dest.0, ty);
                return Ok(dest);
            }
            if let Some((getter, ty)) = self.resolve_static_property(&class, property) {
                let dest = self.new_var();
                self.emit(Instruction::Call {
                    dest: Some(dest),
                    func: FuncId(getter),
                    args: vec![],
                    ret_ty: ty.clone(),
                });
                self.var_types.insert(dest.0, ty);
                return Ok(dest);
            }
            return Err(IrError::new(format!(
                "العضو '{}' ليس عضواً مشتركاً في الصنف '{}'",
                property, class
            )));
        }

        let obj_type = self.infer_expr_type(object);
        let obj_var = self.build_expr(object)?;
        let dest = self.new_var();

        let class_id_opt = match &obj_type {
            IrType::Struct(class_id) => Some(class_id.clone()),
            IrType::Ptr(inner) => {
                if let IrType::Struct(class_id) = inner.as_ref() {
                    Some(class_id.clone())
                } else {
                    None
                }
            }
            _ => None,
        };

        // Check if this is a property with a getter
        if let Some(ref class_id) = class_id_opt {
            let prop_key = format!("{}::{}", class_id.0, property);
            if let Some((getter_name, prop_type)) = self.property_getters.get(&prop_key).cloned() {
                // Extract just the method name part (e.g., "__احصل_اسم" from "شخص::__احصل_اسم")
                let method_name_only = getter_name
                    .split("::")
                    .last()
                    .unwrap_or(&getter_name)
                    .to_string();
                // Emit a method call to the getter instead of GetField
                self.emit(Instruction::CallMethod {
                    dest: Some(dest),
                    object: obj_var,
                    method: MethodId {
                        class: class_id.clone(),
                        name: method_name_only,
                    },
                    args: vec![],
                    ret_ty: prop_type.clone(),
                    virtual_dispatch: !is_super_receiver(object),
                });
                self.var_types.insert(dest.0, prop_type);
                return Ok(dest);
            }
        }

        let (field_ty, field_index, class_id) = if let Some(class_id) = class_id_opt {
            if let Some((idx, ty)) = self.get_field_info(&class_id.0, property) {
                (ty, idx, class_id)
            } else {
                (IrType::Ptr(Box::new(IrType::Void)), 0, class_id)
            }
        } else {
            (
                IrType::Ptr(Box::new(IrType::Void)),
                0,
                ClassId("".to_string()),
            )
        };

        self.emit(Instruction::GetField {
            dest,
            object: obj_var,
            field: FieldId {
                class: class_id,
                name: property.to_string(),
                index: field_index,
            },
            ty: field_ty.clone(),
        });

        self.var_types.insert(dest.0, field_ty);
        Ok(dest)
    }

    /// Build IR for an index operation.
    pub(crate) fn build_index(&mut self, object: &Expr, index: &Expr) -> Result<VarId> {
        let obj_type = self.infer_expr_type(object);
        let obj_var = self.build_expr(object)?;
        let idx_var = self.build_expr(index)?;
        let dest = self.new_var();

        let elem_ty = if let IrType::Array(elem, _) = &obj_type {
            (**elem).clone()
        } else {
            IrType::Ptr(Box::new(IrType::Void))
        };

        self.emit(Instruction::ArrayGet {
            dest,
            array: obj_var,
            index: idx_var,
            elem_ty: elem_ty.clone(),
        });

        self.var_types.insert(dest.0, elem_ty);
        Ok(dest)
    }

    /// Build IR for an assignment expression.
    pub(crate) fn build_assignment(&mut self, target: &Expr, value: &Expr) -> Result<VarId> {
        // Writing through an import binding (`أدوات.عداد = ٥`, or an aliased
        // `اضعف = ...`) targets the merged declaration's bare name, exactly
        // as reading through one does.
        let resolved_target = self.resolve_import_ref(target);
        let target = resolved_target.as_ref().unwrap_or(target);

        // Assigning a bare lambda to an already-annotated function-typed
        // slot must thread that slot's signature in as a hint, exactly as a
        // declaration does — otherwise the lambda lifts with untyped params
        // and native codegen rejects code whose type the user *did* declare.
        let value_var = match (&target.kind, &value.kind) {
            (ExprKind::Identifier(name), ExprKind::Lambda { params, body }) => {
                let slot_ty = self.callee_value_type(name);
                if matches!(slot_ty, Some(IrType::Function { .. })) {
                    self.build_lambda_with_hint(params, body, slot_ty.as_ref())?
                } else {
                    self.build_expr(value)?
                }
            }
            _ => self.build_expr(value)?,
        };

        match &target.kind {
            ExprKind::Identifier(name) => {
                if let Some(ptr) = self.lookup_var(name) {
                    self.emit(Instruction::Store {
                        ptr,
                        value: value_var,
                    });
                } else if self.global_variables.contains(name) {
                    self.emit(Instruction::GlobalStore {
                        name: name.clone(),
                        value: value_var,
                    });
                } else {
                    return Err(IrError::new(format!(
                        "لا يمكن التعيين لمتغير غير معرّف: '{}'",
                        name
                    )));
                }
            }
            ExprKind::Member { object, property } => {
                if let Some(class) = self.class_name_receiver(object) {
                    if let Some((key, _)) = self.resolve_static_field(&class, property) {
                        self.emit(Instruction::GlobalStore {
                            name: key,
                            value: value_var,
                        });
                        return Ok(value_var);
                    }
                    if let Some(setter) = self.resolve_static_property_setter(&class, property) {
                        self.emit(Instruction::Call {
                            dest: None,
                            func: FuncId(setter),
                            args: vec![value_var],
                            ret_ty: IrType::Void,
                        });
                        return Ok(value_var);
                    }
                    return Err(IrError::new(format!(
                        "العضو '{}' ليس عضواً مشتركاً في الصنف '{}'",
                        property, class
                    )));
                }

                let obj_type = self.infer_expr_type(object);
                let obj_var = self.build_expr(object)?;

                let class_id_opt = match &obj_type {
                    IrType::Struct(class_id) => Some(class_id.clone()),
                    IrType::Ptr(inner) => {
                        if let IrType::Struct(class_id) = inner.as_ref() {
                            Some(class_id.clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                // Check if this is a property with a setter
                if let Some(ref class_id) = class_id_opt {
                    let prop_key = format!("{}::{}", class_id.0, property);
                    if let Some(setter_name) = self.property_setters.get(&prop_key).cloned() {
                        // Extract just the method name part (e.g., "__عيّن_اسم" from "شخص::__عيّن_اسم")
                        let method_name_only = setter_name
                            .split("::")
                            .last()
                            .unwrap_or(&setter_name)
                            .to_string();
                        // Emit a method call to the setter instead of SetField
                        self.emit(Instruction::CallMethod {
                            dest: None,
                            object: obj_var,
                            method: MethodId {
                                class: class_id.clone(),
                                name: method_name_only,
                            },
                            args: vec![value_var],
                            ret_ty: IrType::Void,
                            virtual_dispatch: !is_super_receiver(object),
                        });
                        return Ok(value_var);
                    }
                }

                let (class_id, field_index) = if let Some(class_id) = class_id_opt {
                    let index = self
                        .get_field_info(&class_id.0, property)
                        .map(|(idx, _)| idx)
                        .unwrap_or(0);
                    (class_id, index)
                } else {
                    (ClassId("".to_string()), 0)
                };

                self.emit(Instruction::SetField {
                    object: obj_var,
                    field: FieldId {
                        class: class_id,
                        name: property.clone(),
                        index: field_index,
                    },
                    value: value_var,
                });
            }
            ExprKind::Index { object, index } => {
                let obj_var = self.build_expr(object)?;
                let idx_var = self.build_expr(index)?;
                self.emit(Instruction::ArraySet {
                    array: obj_var,
                    index: idx_var,
                    value: value_var,
                });
            }
            _ => {
                return Err(IrError::new("هدف التعيين غير مدعوم"));
            }
        }

        Ok(value_var)
    }

    /// Build IR for a compound assignment (+=, -=, etc.).
    pub(crate) fn build_compound_assignment(
        &mut self,
        target: &Expr,
        op: AstBinaryOp,
        value: &Expr,
    ) -> Result<VarId> {
        // See `build_assignment`: an import binding on the left of `+=` names
        // the merged declaration, not a member of a namespace object.
        let resolved_target = self.resolve_import_ref(target);
        let target = resolved_target.as_ref().unwrap_or(target);

        let current = self.build_expr(target)?;
        let increment = self.build_expr(value)?;

        let ir_op = match op {
            AstBinaryOp::Add => BinaryOp::Add,
            AstBinaryOp::Sub => BinaryOp::Sub,
            AstBinaryOp::Mul => BinaryOp::Mul,
            AstBinaryOp::Div => BinaryOp::Div,
            AstBinaryOp::Mod => BinaryOp::Mod,
            _ => BinaryOp::Add,
        };

        let result = self.new_var();
        self.emit(Instruction::Binary {
            dest: result,
            op: ir_op,
            left: current,
            right: increment,
            ty: IrType::Int,
        });

        match &target.kind {
            ExprKind::Identifier(name) => {
                if let Some(ptr) = self.lookup_var(name) {
                    self.emit(Instruction::Store { ptr, value: result });
                } else if self.global_variables.contains(name) {
                    self.emit(Instruction::GlobalStore {
                        name: name.clone(),
                        value: result,
                    });
                } else {
                    return Err(IrError::new(format!(
                        "لا يمكن التعيين لمتغير غير معرّف: '{}'",
                        name
                    )));
                }
            }
            ExprKind::Member { object, property } => {
                if let Some(class) = self.class_name_receiver(object) {
                    if let Some((key, _)) = self.resolve_static_field(&class, property) {
                        self.emit(Instruction::GlobalStore {
                            name: key,
                            value: result,
                        });
                        return Ok(result);
                    }
                    if let Some(setter) = self.resolve_static_property_setter(&class, property) {
                        self.emit(Instruction::Call {
                            dest: None,
                            func: FuncId(setter),
                            args: vec![result],
                            ret_ty: IrType::Void,
                        });
                        return Ok(result);
                    }
                    return Err(IrError::new(format!(
                        "العضو '{}' ليس عضواً مشتركاً في الصنف '{}'",
                        property, class
                    )));
                }

                let obj_var = self.build_expr(object)?;
                self.emit(Instruction::SetField {
                    object: obj_var,
                    field: FieldId {
                        class: ClassId("".to_string()),
                        name: property.clone(),
                        index: 0,
                    },
                    value: result,
                });
            }
            ExprKind::Index { object, index } => {
                let obj_var = self.build_expr(object)?;
                let idx_var = self.build_expr(index)?;
                self.emit(Instruction::ArraySet {
                    array: obj_var,
                    index: idx_var,
                    value: result,
                });
            }
            _ => {
                return Err(IrError::new("هدف التعيين المركب غير مدعوم"));
            }
        }

        Ok(result)
    }

    /// Build IR for an array literal.
    pub(crate) fn build_array(&mut self, elements: &[Expr]) -> Result<VarId> {
        let elem_ty = if let Some(first) = elements.first() {
            self.infer_expr_type(first)
        } else {
            IrType::Ptr(Box::new(IrType::Void))
        };

        let elem_vars: Vec<VarId> = elements
            .iter()
            .map(|e| self.build_expr(e))
            .collect::<Result<Vec<_>>>()?;

        let dest = self.new_var();
        let array_ty = IrType::Array(Box::new(elem_ty.clone()), elem_vars.len());
        self.emit(Instruction::NewArray {
            dest,
            elem_ty,
            elements: elem_vars,
        });

        self.var_types.insert(dest.0, array_ty);
        Ok(dest)
    }

    /// Build IR for an object literal.
    pub(crate) fn build_object(&mut self, fields: &[(String, Expr)]) -> Result<VarId> {
        let dest = self.new_var();
        let class_id = ClassId("__anonymous__".to_string());
        self.emit(Instruction::NewObject {
            dest,
            class: class_id.clone(),
        });

        for (name, expr) in fields {
            let value = self.build_expr(expr)?;
            self.emit(Instruction::SetField {
                object: dest,
                field: FieldId {
                    class: class_id.clone(),
                    name: name.clone(),
                    index: 0,
                },
                value,
            });
        }

        self.var_types.insert(dest.0, IrType::Struct(class_id));
        Ok(dest)
    }

    /// Build IR for a lambda expression with no type hint from context.
    pub(crate) fn build_lambda(&mut self, params: &[Param], body: &LambdaBody) -> Result<VarId> {
        self.build_lambda_with_hint(params, body, None)
    }

    /// Build IR for a lambda expression, optionally hinted by a declared
    /// function type from context (e.g. `ثابت ف: (عدد) -> عدد = (س) => ...`).
    /// The hint fills in a concrete type for any unannotated parameter that
    /// would otherwise default to `Ptr(Void)`.
    ///
    /// Lifts the body into a real module-level function (`__lambda_N`) and
    /// leaves behind a `Constant::Function` value referencing it by name —
    /// the missing piece that made lambdas type-check but never execute
    /// (issue #180).
    pub(crate) fn build_lambda_with_hint(
        &mut self,
        params: &[Param],
        body: &LambdaBody,
        hint: Option<&IrType>,
    ) -> Result<VarId> {
        use super::super::Parameter;

        let hint_params: &[IrType] = match hint {
            Some(IrType::Function { params, .. }) => params.as_slice(),
            _ => &[],
        };

        let lambda_name = format!("__lambda_{}", self.lambda_counter);
        self.lambda_counter += 1;

        // Defense in depth: the module-scoped counter above should make a
        // collision unreachable, but a user program can't otherwise declare
        // a function named `__lambda_N`, so this is a one-line guard rather
        // than a real expected path.
        if self
            .module
            .get_function(&FuncId(lambda_name.clone()))
            .is_some()
        {
            return Err(IrError::new(format!(
                "الدالة الداخلية '{}' معرّفة مسبقاً",
                lambda_name
            )));
        }

        let ir_params: Vec<Parameter> = params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let ty =
                    p.ty.as_ref()
                        .map(|t| self.convert_type(t))
                        .or_else(|| hint_params.get(i).cloned())
                        .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                Parameter {
                    id: VarId(i as u32),
                    name: p.name.clone(),
                    ty,
                }
            })
            .collect();
        let real_param_tys: Vec<IrType> = ir_params.iter().map(|p| p.ty.clone()).collect();

        let saved = self.suspend_function_context();

        let unlowerable = self.unlowerable_param_names(params, hint_params);
        self.begin_function(
            lambda_name.clone(),
            ir_params,
            IrType::Ptr(Box::new(IrType::Void)), // provisional; patched below
        )?;
        if let Some(name) = unlowerable.first() {
            self.block_native_lowering(Self::untyped_param_reason(name));
        }

        // Read whatever the lambda's own build produces (return type) from
        // its own var_types/current_function *before* the outer state below
        // is restored.
        let real_ret_ty = match body {
            LambdaBody::Expr(expr) => {
                // A curried lambda (`(أ) => (ب) => ...`, spec §5.3) must pass
                // the outer hint's *return* signature down, or the inner
                // lambda's params stay untyped and native codegen rejects the
                // very syntax the annotation spelled out.
                let inner_hint = match (&expr.kind, hint) {
                    (ExprKind::Lambda { .. }, Some(IrType::Function { ret, .. }))
                        if matches!(**ret, IrType::Function { .. }) =>
                    {
                        Some((**ret).clone())
                    }
                    _ => None,
                };
                let result = match (&expr.kind, &inner_hint) {
                    (ExprKind::Lambda { params, body }, Some(h)) => {
                        self.build_lambda_with_hint(params, body, Some(h))?
                    }
                    _ => self.build_expr(expr)?,
                };
                let ret_ty = self
                    .var_types
                    .get(&result.0)
                    .cloned()
                    .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                // A Void-valued body (`(س) => اطبع(س)` — an idiomatic
                // callback) has no value to return: `ret void %v` is invalid
                // LLVM, and a void `Call` never even names a dest for `%v`
                // to refer to.
                if ret_ty == IrType::Void {
                    self.emit(Instruction::Return { value: None });
                } else {
                    self.emit(Instruction::Return {
                        value: Some(result),
                    });
                }
                ret_ty
            }
            LambdaBody::Block(block) => {
                for stmt in &block.statements {
                    self.build_stmt(stmt)?;
                }

                // Scan ALL returns, not just the first: semantic analysis
                // accepts mixed bare/valued and عدد/عدد_عشري returns (folded
                // to أي), so the lifted function must be patched to one
                // consistent return type or native codegen emits invalid
                // LLVM IR (`ret void` inside `define i64`, or `ret i64` of
                // a double).
                let valued: Vec<IrType> = self
                    .current_function
                    .as_ref()
                    .map(|func| {
                        func.blocks
                            .iter()
                            .flat_map(|b| &b.instructions)
                            .filter_map(|inst| match inst {
                                Instruction::Return { value: Some(v) } => Some(
                                    self.var_types
                                        .get(&v.0)
                                        .cloned()
                                        .unwrap_or(IrType::Ptr(Box::new(IrType::Void))),
                                ),
                                _ => None,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let ret_ty = if valued.is_empty() {
                    IrType::Void
                } else if valued.iter().all(|t| *t == valued[0]) {
                    valued[0].clone()
                } else if valued
                    .iter()
                    .all(|t| matches!(t, IrType::Int | IrType::Float))
                {
                    // Mixed عدد/عدد_عشري returns widen to عدد_عشري
                    // (spec §5.6's implicit coercion, applied per-return in
                    // the patch pass below).
                    IrType::Float
                } else {
                    // Non-unifiable mix (e.g. `أرجع "نص"` and `أرجع ١` in one
                    // lambda) — semantic analysis folds this to `أي` and the
                    // interpreter dispatches on runtime values, but native
                    // code needs one concrete ABI. Keep the first type so the
                    // IR stays well-formed for the interpreter, and block
                    // native lowering so the user gets a Tarqeem diagnostic
                    // instead of a leaked clang type error.
                    self.block_native_lowering(
                        "قيم الإرجاع في الدالة السهمية ذات أنواع غير متوافقة".to_string(),
                    );
                    valued[0].clone()
                };

                if ret_ty != IrType::Void {
                    self.patch_block_lambda_returns(&ret_ty);
                }

                let needs_terminator = self
                    .current_function
                    .as_ref()
                    .and_then(|func| func.blocks.last())
                    .map(|blk| !blk.has_terminator())
                    .unwrap_or(false);

                if needs_terminator {
                    if ret_ty == IrType::Void {
                        self.emit(Instruction::Return { value: None });
                    } else {
                        // Defensive default: semantic analysis is expected to
                        // guarantee every path returns whenever a non-void
                        // return type was inferred, so this should be
                        // unreachable for valid programs — it exists only to
                        // avoid an ill-typed `ret void` inside a non-void
                        // function if it ever is.
                        let dest = self.new_var();
                        let zero = match &ret_ty {
                            IrType::Float => Constant::Float(0.0),
                            IrType::Bool => Constant::Bool(false),
                            IrType::Int => Constant::Int(0),
                            _ => Constant::Null,
                        };
                        self.emit(Instruction::Const {
                            dest,
                            value: zero,
                            ty: ret_ty.clone(),
                        });
                        self.var_types.insert(dest.0, ret_ty.clone());
                        self.emit(Instruction::Return { value: Some(dest) });
                    }
                }

                ret_ty
            }
        };

        if let Some(ref mut func) = self.current_function {
            func.return_type = real_ret_ty.clone();
        }

        self.end_function()?;

        self.resume_function_context(saved);

        self.function_param_types
            .insert(lambda_name.clone(), real_param_tys.clone());
        self.function_return_types
            .insert(lambda_name.clone(), real_ret_ty.clone());

        let fn_ty = IrType::Function {
            params: real_param_tys,
            ret: Box::new(real_ret_ty),
        };

        let dest = self.new_var();
        self.emit(Instruction::Const {
            dest,
            value: Constant::Function(lambda_name),
            ty: fn_ty.clone(),
        });
        self.var_types.insert(dest.0, fn_ty);

        Ok(dest)
    }

    /// Rewrites every `أرجع` in the (still-current) lifted lambda to match
    /// the unified non-void return type: bare `Return {None}` becomes a
    /// zero-of-type return, and an `Int`-valued return in a `Float`-typed
    /// lambda gains an `IntToFloat` (spec §5.6). Without this, mixed
    /// returns — legal early-return code the semantic layer folds to أي —
    /// lower to invalid LLVM IR (`ret void` inside `define i64`).
    fn patch_block_lambda_returns(&mut self, ret_ty: &IrType) {
        // Plan first (immutable scan), then apply — each patch allocates a
        // fresh VarId, which needs `&mut self`.
        let mut plan: Vec<(usize, usize, Option<VarId>)> = Vec::new();
        let Some(func) = self.current_function.as_ref() else {
            return;
        };
        for (bi, blk) in func.blocks.iter().enumerate() {
            for (ii, inst) in blk.instructions.iter().enumerate() {
                match inst {
                    Instruction::Return { value: None } => plan.push((bi, ii, None)),
                    Instruction::Return { value: Some(v) }
                        if *ret_ty == IrType::Float
                            && self.var_types.get(&v.0) == Some(&IrType::Int) =>
                    {
                        plan.push((bi, ii, Some(*v)));
                    }
                    _ => {}
                }
            }
        }
        // Reverse order keeps earlier indices valid while each patch
        // replaces one instruction with two.
        for (bi, ii, src) in plan.into_iter().rev() {
            let dest = self.new_var();
            let (replacement, dest_ty) = match src {
                None => {
                    let zero = match ret_ty {
                        IrType::Float => Constant::Float(0.0),
                        IrType::Bool => Constant::Bool(false),
                        IrType::Int => Constant::Int(0),
                        _ => Constant::Null,
                    };
                    (
                        vec![
                            Instruction::Const {
                                dest,
                                value: zero,
                                ty: ret_ty.clone(),
                            },
                            Instruction::Return { value: Some(dest) },
                        ],
                        ret_ty.clone(),
                    )
                }
                Some(v) => (
                    vec![
                        Instruction::IntToFloat { dest, src: v },
                        Instruction::Return { value: Some(dest) },
                    ],
                    IrType::Float,
                ),
            };
            self.var_types.insert(dest.0, dest_ty);
            if let Some(func) = self.current_function.as_mut() {
                func.blocks[bi].instructions.splice(ii..=ii, replacement);
            }
        }
    }

    /// Build IR for a new object instantiation.
    pub(crate) fn build_new(&mut self, class: &Expr, args: &[Expr]) -> Result<VarId> {
        let class_name = if let ExprKind::Identifier(name) = &class.kind {
            self.resolve_class_name(name)
        } else {
            "__dynamic__".to_string()
        };

        let class_id = ClassId(class_name.clone());

        let dest = self.new_var();
        self.emit(Instruction::NewObject {
            dest,
            class: class_id.clone(),
        });

        self.var_types.insert(dest.0, IrType::Struct(class_id));

        let arg_vars: Vec<VarId> = args
            .iter()
            .map(|a| self.build_expr(a))
            .collect::<Result<Vec<_>>>()?;

        let ctor_name = format!("{}::منشئ", class_name);
        let mut ctor_args = vec![dest];
        ctor_args.extend(arg_vars);

        self.emit(Instruction::Call {
            dest: None,
            func: FuncId(ctor_name),
            args: ctor_args,
            ret_ty: IrType::Void,
        });

        Ok(dest)
    }

    /// Build IR for an await expression.
    pub(crate) fn build_await(&mut self, inner: &Expr) -> Result<VarId> {
        self.build_expr(inner)
    }

    /// Build IR for a ternary expression.
    pub(crate) fn build_ternary(
        &mut self,
        condition: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
    ) -> Result<VarId> {
        let cond_var = self.build_expr(condition)?;

        let then_block = self.new_block(Some("ternary.then".to_string()));
        let else_block = self.new_block(Some("ternary.else".to_string()));
        let merge_block = self.new_block(Some("ternary.merge".to_string()));

        self.emit(Instruction::Branch {
            cond: cond_var,
            then_block,
            else_block,
        });

        self.switch_to_block(then_block);
        let then_var = self.build_expr(then_expr)?;
        let then_exit_block = self.current_block;
        self.emit(Instruction::Jump {
            target: merge_block,
        });

        self.switch_to_block(else_block);
        let else_var = self.build_expr(else_expr)?;
        let else_exit_block = self.current_block;
        self.emit(Instruction::Jump {
            target: merge_block,
        });

        self.switch_to_block(merge_block);
        let result = self.new_var();

        let phi_type = self
            .var_types
            .get(&then_var.0)
            .cloned()
            .or_else(|| self.var_types.get(&else_var.0).cloned())
            .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));

        self.var_types.insert(result.0, phi_type.clone());

        self.emit(Instruction::Phi {
            dest: result,
            ty: phi_type,
            incoming: vec![(then_var, then_exit_block), (else_var, else_exit_block)],
        });

        Ok(result)
    }

    /// Build IR for 'this' reference.
    pub(crate) fn build_this(&mut self) -> Result<VarId> {
        if let Some(var) = self.lookup_var("هذا").or_else(|| self.lookup_var("this")) {
            Ok(var)
        } else {
            Err(IrError::new("'هذا' يمكن استخدامه فقط داخل دالة"))
        }
    }

    /// Build IR for 'super' reference.
    pub(crate) fn build_super(&mut self) -> Result<VarId> {
        if let Some(var) = self.lookup_var("هذا").or_else(|| self.lookup_var("this")) {
            Ok(var)
        } else {
            Err(IrError::new("'الأصل' يمكن استخدامه فقط داخل دالة"))
        }
    }

    /// Build IR for a super constructor call.
    pub(crate) fn build_super_constructor_call(&mut self, args: &[Expr]) -> Result<VarId> {
        let this_var = self
            .lookup_var("هذا")
            .or_else(|| self.lookup_var("this"))
            .ok_or_else(|| IrError::new("'الأصل()' يمكن استخدامه فقط داخل منشئ"))?;

        let current_class_name = match &self.current_function {
            Some(func) => {
                if let Some(idx) = func.name.find("::") {
                    func.name[..idx].to_string()
                } else {
                    return Err(IrError::new("'الأصل()' يمكن استخدامه فقط داخل منشئ صنف"));
                }
            }
            None => {
                return Err(IrError::new("'الأصل()' يمكن استخدامه فقط داخل دالة"));
            }
        };

        let parent_class_name = self
            .module
            .classes
            .iter()
            .find(|c| c.name == current_class_name)
            .and_then(|c| c.parent.as_ref())
            .map(|p| p.0.clone())
            .ok_or_else(|| IrError::new(format!("الصنف '{}' ليس له صنف أب", current_class_name)))?;

        let arg_vars: Vec<VarId> = args
            .iter()
            .map(|a| self.build_expr(a))
            .collect::<Result<Vec<_>>>()?;

        let parent_ctor_name = format!("{}::منشئ", parent_class_name);

        let mut call_args = vec![this_var];
        call_args.extend(arg_vars);

        let dest = self.new_var();
        self.emit(Instruction::Call {
            dest: Some(dest),
            func: FuncId(parent_ctor_name),
            args: call_args,
            ret_ty: IrType::Void,
        });
        self.var_types.insert(dest.0, IrType::Void);

        Ok(dest)
    }

    /// NFC-normalize a string for consistent comparison of Arabic identifiers.
    pub(crate) fn normalize_name(name: &str) -> String {
        name.nfc().collect()
    }

    /// Calculate a discriminant hash for an enum variant name.
    /// Uses FNV-1a hash for better distribution and collision resistance.
    /// The name is NFC-normalized first to handle Arabic identifier variations.
    pub(crate) fn calculate_discriminant(variant_name: &str) -> u32 {
        // NFC normalize to handle Arabic identifier variations
        let normalized: String = variant_name.nfc().collect();

        // FNV-1a hash constants for 32-bit
        const FNV_OFFSET: u32 = 2166136261;
        const FNV_PRIME: u32 = 16777619;

        let mut hash = FNV_OFFSET;
        for byte in normalized.bytes() {
            hash ^= byte as u32;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }
}
