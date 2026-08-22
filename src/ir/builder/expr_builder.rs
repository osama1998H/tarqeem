//! Expression building for the IR builder.
//!
//! This module handles conversion of AST expressions to IR instructions.

use crate::parser::{
    BinaryOp as AstBinaryOp, Expr, ExprKind, LambdaBody, Literal, Param, UnaryOp as AstUnaryOp,
};

use super::super::{
    BinaryOp, ClassId, Constant, EnumId, FieldId, FuncId, Instruction, IrType, MethodId,
    NativeBlock, UnaryOp, VarId, VariantId,
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

/// A member that resolves to neither a field nor a property on a class whose
/// layout the builder knows — nor on any of its ancestors.
///
/// This used to be a silent fallback to `index: 0` with type `ptr`, which
/// codegen turned into an out-of-bounds GEP or a `trq_print(ptr)` against an
/// integer (issue #249). Semantic analysis rejects unknown members before the IR
/// builder runs, so reaching this means the analyzer and `collect_class` disagree
/// about a class's shape: an internal invariant, not a user diagnostic, hence no
/// ت/د error code — but still bilingual, since a contributor reading it is
/// exactly who it is for. Same reasoning as `backing_field_index`.
fn unknown_member_error(class: &str, member: &str) -> IrError {
    IrError::new(format!(
        "العضو '{member}' غير موجود في تخطيط الصنف '{class}' ولا في أي من أصوله \
         / member '{member}' is missing from the layout of class '{class}' and all of its ancestors"
    ))
}

/// What `emit_shift_range_guard` yields for one shift amount.
struct ShiftGuard {
    /// The amount masked into 0-63, safe to hand to any backend's shift.
    amount: VarId,
    /// All ones when the original amount was outside 0-63, zero otherwise.
    out_of_range: VarId,
    /// The `٦٣` the guard masked with, so an arm needing that constant reuses
    /// it rather than emitting a second one.
    range_mask: VarId,
}

/// A `خاصية` that exists but does not declare the accessor this access needs:
/// reading one that has only `عيّن`, or assigning to one that has only `احصل`.
///
/// Unlike `unknown_member_error` this *is* user-reachable — semantic analysis
/// does not yet reject either direction — so it must name what actually went
/// wrong rather than claim the member is missing from the layout. It stays an
/// `IrError` without a ص code because that check belongs in the analyzer, where
/// a span is still available; this is the backstop that keeps the builder from
/// silently reading slot 0 (or, before it errored at all, corrupting it).
fn missing_accessor_error(class: &str, member: &str, reading: bool) -> IrError {
    let (accessor, ar_action, en_action) = if reading {
        ("احصل", "قراءة", "read")
    } else {
        ("عيّن", "التعيين إلى", "assign to")
    };
    IrError::new(format!(
        "لا يمكن {ar_action} الخاصية '{member}' في الصنف '{class}': لا تملك مُلحق '{accessor}' \
         / cannot {en_action} property '{member}' on class '{class}': it declares no '{accessor}' accessor"
    ))
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

    /// The type of `var` as recorded during building, defaulting to `Int` like
    /// the other builtin lowerings do.
    fn arg_type(&self, var: VarId) -> IrType {
        self.var_types.get(&var.0).cloned().unwrap_or(IrType::Int)
    }

    fn emit_const(&mut self, value: Constant, ty: IrType) -> VarId {
        let dest = self.new_var();
        self.emit(Instruction::Const {
            dest,
            value,
            ty: ty.clone(),
        });
        self.var_types.insert(dest.0, ty);
        dest
    }

    /// A `void`-typed result, for builtins called as statements.
    fn emit_void(&mut self) -> VarId {
        self.emit_const(Constant::Null, IrType::Void)
    }

    fn emit_int_const(&mut self, value: i64) -> VarId {
        self.emit_const(Constant::Int(value), IrType::Int)
    }

    fn emit_int_binary(&mut self, op: BinaryOp, left: VarId, right: VarId) -> VarId {
        let dest = self.new_var();
        self.emit(Instruction::Binary {
            dest,
            op,
            left,
            right,
            ty: IrType::Int,
        });
        self.var_types.insert(dest.0, IrType::Int);
        dest
    }

    fn emit_int_unary(&mut self, op: UnaryOp, operand: VarId) -> VarId {
        let dest = self.new_var();
        self.emit(Instruction::Unary {
            dest,
            op,
            operand,
            ty: IrType::Int,
        });
        self.var_types.insert(dest.0, IrType::Int);
        dest
    }

    fn emit_call(&mut self, symbol: &str, args: Vec<VarId>, ret_ty: IrType) -> VarId {
        let dest = self.new_var();
        self.emit(Instruction::Call {
            dest: Some(dest),
            func: FuncId(symbol.to_string()),
            args,
            ret_ty: ret_ty.clone(),
        });
        self.var_types.insert(dest.0, ret_ty);
        dest
    }

    /// The Arabic type name `نوع` reports, mirroring `Value::type_name_ar` in
    /// the interpreter.
    fn type_name_ar(ty: &IrType) -> &'static str {
        match ty {
            IrType::Bool => "منطقي",
            IrType::Int => "عدد",
            IrType::Float => "عدد_عشري",
            IrType::String => "نص",
            IrType::Array(_, _) => "مصفوفة",
            IrType::Struct(_) => "كائن",
            IrType::Enum(_) => "تعداد",
            IrType::Function { .. } => "دالة",
            IrType::Void => "لا_شيء",
            IrType::Ptr(_) => "مؤشر",
        }
    }

    /// Lowers a core builtin that cannot be expressed as a name→symbol mapping.
    ///
    /// The conversion builtins are declared over `أي`, so the callee depends on
    /// the argument's type; `نوع` is answerable at build time because `IrType`
    /// has no dynamic variant; and `تأكد` needs a message operand its call site
    /// never supplies. Codegen's table is name-only and can express none of
    /// that, so an unmapped name used to fall through to a mangled Arabic symbol
    /// that nothing defined (#222).
    ///
    /// Returns `None` for anything that is not one of these, leaving the normal
    /// call path to handle it. Interception is deliberately unconditional — see
    /// the note at the call site on why a shadowing guard is wrong here.
    ///
    /// `arg_exprs` is the unlowered argument list: `نوع` answers from `IrType`,
    /// which cannot tell `لا_شيء` from any other untyped pointer, so the literal
    /// is read back off the AST.
    fn build_core_builtin_call(
        &mut self,
        name: &str,
        args: &[VarId],
        arg_exprs: &[Expr],
    ) -> Result<Option<VarId>> {
        let Some(&first) = args.first() else {
            return Ok(None);
        };
        let arg_ty = self.arg_type(first);

        let dest = match name {
            // `اطبع_سطر` shares the arm because the interpreter prints all three
            // with `println!`; codegen's table maps it to the newline-less
            // `trq_print`, which printed `أب` natively for two calls that give
            // `أ\nب\n` interpreted.
            "اطبع" | "طباعة" | "اطبع_سطر" => {
                self.emit(Instruction::Print { value: first });
                self.emit_void()
            }

            "طول" | "طول_مصفوفة" => {
                let dest = self.new_var();
                self.emit(Instruction::ArrayLen { dest, array: first });
                self.var_types.insert(dest.0, IrType::Int);
                dest
            }

            // A value already of type `نص` is its own conversion:
            // `convert_to_string` has no `String` arm and would fall through to
            // `trq_int_to_string`, printing the pointer as a decimal number.
            "نص" if arg_ty == IrType::String => first,
            "نص" => self.convert_to_string(first, &arg_ty)?,

            "نوع" => {
                let name_ar = match arg_exprs.first().map(|e| &e.kind) {
                    Some(ExprKind::Literal(Literal::Null)) => "لا_شيء",
                    _ => Self::type_name_ar(&arg_ty),
                };
                let idx = self.add_string(name_ar.to_string());
                self.emit_const(Constant::String(idx), IrType::String)
            }

            "عدد" => match arg_ty {
                IrType::Int => first,
                IrType::Bool => {
                    // صحيح/خطأ become 1/0, as the interpreter's `عدد` does.
                    let dest = self.new_var();
                    self.emit(Instruction::BoolToInt { dest, src: first });
                    self.var_types.insert(dest.0, IrType::Int);
                    dest
                }
                IrType::Float => {
                    let dest = self.new_var();
                    self.emit(Instruction::FloatToInt { dest, src: first });
                    self.var_types.insert(dest.0, IrType::Int);
                    dest
                }
                ref ty if Self::may_be_string(ty) => {
                    self.emit_call("trq_string_to_int_checked", vec![first], IrType::Int)
                }
                ref ty => return Err(Self::unconvertible(ty, "عدد")),
            },

            "عدد_عشري" => match arg_ty {
                IrType::Float => first,
                IrType::Int | IrType::Bool => {
                    // A bool widens through i64 first: `sitofp` takes an i64,
                    // and the interpreter's IntToFloat likewise rejects a bool.
                    let widened = if arg_ty == IrType::Bool {
                        let dest = self.new_var();
                        self.emit(Instruction::BoolToInt { dest, src: first });
                        self.var_types.insert(dest.0, IrType::Int);
                        dest
                    } else {
                        first
                    };
                    let dest = self.new_var();
                    self.emit(Instruction::IntToFloat { dest, src: widened });
                    self.var_types.insert(dest.0, IrType::Float);
                    dest
                }
                ref ty if Self::may_be_string(ty) => {
                    self.emit_call("trq_string_to_float_checked", vec![first], IrType::Float)
                }
                ref ty => return Err(Self::unconvertible(ty, "عدد_عشري")),
            },

            "منطقي" => self.build_truthiness(first, &arg_ty),

            "تأكد" | "تأكد_رسالة" => {
                // `trq_assert` treats a null message as "فشل التأكيد", which is
                // exactly what the interpreter reports for the one-argument form.
                let message = match args.get(1) {
                    Some(&msg) => msg,
                    None => self.emit_const(Constant::Null, IrType::Ptr(Box::new(IrType::Void))),
                };
                self.emit(Instruction::Call {
                    dest: None,
                    func: FuncId("trq_assert".to_string()),
                    args: vec![first, message],
                    ret_ty: IrType::Void,
                });
                self.emit_void()
            }

            "ألحق" => {
                let Some(&value) = args.get(1) else {
                    return Ok(None);
                };
                // The array's element type wins over the pushed value's, as the
                // member path below does: pushing `2` into `[1.5]` with
                // `elem_ty: Int` stores an i64 bit pattern the reader decodes as
                // a double.
                let elem_ty = Self::array_elem_ty(&arg_ty).unwrap_or_else(|| self.arg_type(value));
                // …and the value is widened to it, the same عدد → عدد_عشري
                // coercion call arguments get: storing a raw i64 into a
                // `double` slot is an LLVM type error.
                let value = self
                    .coerce_args_to_params(vec![value], std::slice::from_ref(&elem_ty))
                    .remove(0);
                self.emit(Instruction::ArrayPush {
                    array: first,
                    value,
                    elem_ty,
                });
                self.emit_void()
            }

            "احذف_آخر" => {
                // The array is the only place the element type can come from —
                // `ألحق` can fall back on the pushed value's type, and there is
                // no value here. Guessing `عدد` would read a `double` slot as a
                // raw bit pattern, silently, so a non-array is refused instead.
                let elem_ty = Self::array_elem_ty(&arg_ty)
                    .ok_or_else(|| Self::not_an_array(&arg_ty, "احذف_آخر"))?;
                let dest = self.new_var();
                self.emit(Instruction::ArrayPop {
                    dest,
                    array: first,
                    elem_ty: elem_ty.clone(),
                });
                self.var_types.insert(dest.0, elem_ty);
                dest
            }

            // Lowered here rather than mapped to a runtime symbol: `BinaryOp` is
            // already implemented by every backend and the constant folder, so
            // an `and`/`or`/`xor i64` costs no `runtime-rs` work and no call.
            "بتات_و" | "بتات_أو" | "بتات_أو_حصري" => {
                let Some(&right) = args.get(1) else {
                    return Ok(None);
                };
                // Exhaustive by name: a sibling added to the pattern above but
                // not here falls through uncalled instead of silently emitting OR.
                let op = match name {
                    "بتات_و" => BinaryOp::BitAnd,
                    "بتات_أو" => BinaryOp::BitOr,
                    "بتات_أو_حصري" => BinaryOp::BitXor,
                    _ => return Ok(None),
                };
                self.emit_int_binary(op, first, right)
            }

            // The family's only unary member, so it cannot share the arm above.
            "بتات_نفي" => self.emit_int_unary(UnaryOp::BitNot, first),

            // The first shift, so the first lowering that is a chain rather than
            // one op. The chain is a range guard, and it is not optional: a bare
            // `Shl` makes an amount outside 0-63 a runtime error interpreted, a
            // masked result folded, and poison natively — the same call
            // disagreeing with itself across backends.
            //
            // Out of range every bit leaves the word and nothing fills it, so
            // zeroing the result is the arithmetic answer.
            "بتات_إزاحة_يسار" => {
                let Some(&amount) = args.get(1) else {
                    return Ok(None);
                };
                let guard = self.emit_shift_range_guard(amount);

                // Zeroing masks the *result*, so what this arm needs is the
                // complement of the guard's flag.
                let keep = self.emit_int_unary(UnaryOp::BitNot, guard.out_of_range);
                let shifted = self.emit_int_binary(BinaryOp::Shl, first, guard.amount);
                self.emit_int_binary(BinaryOp::BitAnd, shifted, keep)
            }

            // The same guard, with the opposite answer out of range: an
            // arithmetic shift vacates the *high* end and refills it from the
            // sign, so shifting everything out leaves the sign rather than zero
            // — 0 for a non-negative operand and -1 for a negative one. Zeroing
            // here would break at the boundary, since `بتات_إزاحة_يمين(-١، ٦٣)`
            // is already -1.
            "بتات_إزاحة_يمين" => {
                let Some(&amount) = args.get(1) else {
                    return Ok(None);
                };
                let guard = self.emit_shift_range_guard(amount);

                // Saturating the amount to 63 is what produces that sign fill,
                // 63 being the amount at which the value is already fully
                // shifted out. `أ | ٦٣` saturates rather than needing a select
                // because `guard.amount` is masked to those same six bits.
                let saturate =
                    self.emit_int_binary(BinaryOp::BitAnd, guard.range_mask, guard.out_of_range);
                let clamped = self.emit_int_binary(BinaryOp::BitOr, guard.amount, saturate);
                self.emit_int_binary(BinaryOp::Shr, first, clamped)
            }

            // The zero-filling counterpart: `عدد` is signed and every backend's
            // `Shr` is arithmetic, so يمين refills from the sign at every
            // amount and no spelling of it fills with zeros.
            //
            // Separating the sign bit is what makes an arithmetic shift behave
            // logically — the remaining 63 bits are non-negative — and the bit
            // is then placed at its new position rather than dropped. That
            // needs no `ن == ٠` special case, unlike the sketch in §1.3.
            "بتات_إزاحة_يمين_منطقية" => {
                let Some(&amount) = args.get(1) else {
                    return Ok(None);
                };
                let guard = self.emit_shift_range_guard(amount);

                // Out of range every bit leaves the word and zeros fill behind
                // it, so zeroing the *value* zeroes every term below. Folding
                // that into one instruction also makes this the value's first
                // scalar use, which is where codegen unboxes a narrowed
                // optional (#318) — `keep` being a bare `Int` is what makes it
                // fire. An `x & -1 => x` peephole would silently restore that
                // bug; `test_logical_right_shift_over_a_narrowed_optional`
                // catches it, natively.
                let keep = self.emit_int_unary(UnaryOp::BitNot, guard.out_of_range);
                let value = self.emit_int_binary(BinaryOp::BitAnd, first, keep);

                let sign_cleared = self.emit_int_const(i64::MAX);
                let low = self.emit_int_binary(BinaryOp::BitAnd, value, sign_cleared);
                let shifted = self.emit_int_binary(BinaryOp::Shr, low, guard.amount);

                // `٦٣ - المقدار` reads the guard's *masked* amount, so it stays
                // in 0-63 and cannot overflow at `i64::MIN`.
                let sign = self.emit_int_binary(BinaryOp::Shr, value, guard.range_mask);
                let sign_position =
                    self.emit_int_binary(BinaryOp::Sub, guard.range_mask, guard.amount);
                let one = self.emit_int_const(1);
                let bit = self.emit_int_binary(BinaryOp::Shl, one, sign_position);
                let moved_sign = self.emit_int_binary(BinaryOp::BitAnd, sign, bit);

                self.emit_int_binary(BinaryOp::BitOr, shifted, moved_sign)
            }

            _ => return Ok(None),
        };

        Ok(Some(dest))
    }

    /// Brings an arbitrary shift amount into the range every backend accepts,
    /// and reports whether it started there.
    ///
    /// No backend can be handed a raw amount: outside 0-63 both interpreters
    /// raise «مقدار الإزاحة خارج النطاق», LLVM's shift is poison, and the
    /// constant folder's `wrapping_*` masks — four answers to one call. So the
    /// shift always sees `amount`, and `out_of_range` is what decides the
    /// result instead. Each shift applies it differently: see its own arm.
    fn emit_shift_range_guard(&mut self, raw_amount: VarId) -> ShiftGuard {
        let sixty_three = self.emit_int_const(63);
        let zero = self.emit_int_const(0);

        // Read the amount once, through this copy, because the chain needs it
        // twice and codegen unboxes a narrowed optional only on its *first* use
        // as a scalar operand — the second would emit the raw pointer and clang
        // would reject the module (#318). `أ | ٠` is free after either
        // optimizer, but it is load-bearing: an `x | 0 => x` peephole would fold
        // it away and silently restore that bug.
        // `test_left_shift_over_a_narrowed_optional` is what catches that,
        // natively.
        let raw_amount = self.emit_int_binary(BinaryOp::BitOr, raw_amount, zero);

        // `ن >> ٦` is zero exactly on 0-63 — a larger amount leaves a positive
        // quotient, a negative one leaves a negative quotient. `high | -high`
        // then carries the sign bit whenever `high` is non-zero, so an
        // arithmetic shift by 63 spreads it to a full -1/0 mask without a branch
        // or a Bool→Int widening (which the JIT tiers have no arm for).
        let six = self.emit_int_const(6);
        let high = self.emit_int_binary(BinaryOp::Shr, raw_amount, six);
        let negated = self.emit_int_binary(BinaryOp::Sub, zero, high);
        let either_sign = self.emit_int_binary(BinaryOp::BitOr, high, negated);
        let out_of_range = self.emit_int_binary(BinaryOp::Shr, either_sign, sixty_three);
        let amount = self.emit_int_binary(BinaryOp::BitAnd, raw_amount, sixty_three);

        ShiftGuard {
            amount,
            out_of_range,
            range_mask: sixty_three,
        }
    }

    /// Whether a value of this type may hold a `نص` at runtime, and so can be
    /// handed to the checked string parsers.
    ///
    /// `Ptr(Void)` is the builder's "unknown" — a lambda parameter or an
    /// untyped local. The reference types below are known *not* to be strings,
    /// and passing one to `trq_string_to_int_checked` reads a foreign pointer
    /// as a `TrqString`, which segfaults natively where the interpreter reports
    /// a type error.
    fn may_be_string(ty: &IrType) -> bool {
        !matches!(
            ty,
            IrType::Array(_, _)
                | IrType::Struct(_)
                | IrType::Enum(_)
                | IrType::Function { .. }
                | IrType::Void
        )
    }

    fn unconvertible(ty: &IrType, target: &str) -> IrError {
        IrError::new(format!(
            "لا يمكن تحويل قيمة من نوع '{}' إلى '{}' / cannot convert a value of type '{}' to '{}'",
            Self::type_name_ar(ty),
            target,
            Self::type_name_ar(ty),
            target
        ))
    }

    /// The element type of an array-shaped `IrType`, through one level of
    /// `Ptr`. `None` for anything else — each caller keeps its own fallback,
    /// because they deliberately differ (refuse, pushed value's type, `Int`).
    fn array_elem_ty(ty: &IrType) -> Option<IrType> {
        match ty {
            IrType::Array(elem, _) => Some((**elem).clone()),
            IrType::Ptr(inner) => match inner.as_ref() {
                IrType::Array(elem, _) => Some((**elem).clone()),
                _ => None,
            },
            _ => None,
        }
    }

    fn not_an_array(ty: &IrType, name: &str) -> IrError {
        IrError::new(format!(
            "'{}' تتطلب مصفوفة، ووُجد '{}' / '{}' requires an array, found '{}'",
            name,
            Self::type_name_ar(ty),
            name,
            Self::type_name_ar(ty)
        ))
    }

    /// `منطقي` follows the interpreter's `Value::is_truthy`: zero, an empty
    /// string and an empty array are false; everything else is true.
    fn build_truthiness(&mut self, var: VarId, ty: &IrType) -> VarId {
        let ptr_ty = IrType::Ptr(Box::new(IrType::Void));

        let (measured, zero, cmp_ty) = match ty {
            IrType::Bool => return var,
            IrType::Float => (
                var,
                self.emit_const(Constant::Float(0.0), IrType::Float),
                IrType::Float,
            ),
            IrType::String => {
                let len = self.emit_call("trq_string_len", vec![var], IrType::Int);
                (
                    len,
                    self.emit_const(Constant::Int(0), IrType::Int),
                    IrType::Int,
                )
            }
            IrType::Array(_, _) => {
                let len = self.new_var();
                self.emit(Instruction::ArrayLen {
                    dest: len,
                    array: var,
                });
                self.var_types.insert(len.0, IrType::Int);
                (
                    len,
                    self.emit_const(Constant::Int(0), IrType::Int),
                    IrType::Int,
                )
            }
            // Reference types compare against a null *pointer*, not `0`: an
            // object is truthy and `لا_شيء` is not, and an `i64 0` operand made
            // codegen emit `icmp ne ptr %a, %b` with mismatched operand types.
            IrType::Ptr(_) | IrType::Struct(_) | IrType::Function { .. } | IrType::Void => {
                (var, self.emit_const(Constant::Null, ptr_ty.clone()), ptr_ty)
            }
            _ => (
                var,
                self.emit_const(Constant::Int(0), IrType::Int),
                IrType::Int,
            ),
        };

        let dest = self.new_var();
        self.emit(Instruction::Binary {
            dest,
            op: BinaryOp::Ne,
            left: measured,
            right: zero,
            ty: cmp_ty,
        });
        self.var_types.insert(dest.0, IrType::Bool);
        dest
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

        // The core builtins below are lowered here rather than mapped by name in
        // codegen, because they take `أي` and so need the argument's type (or,
        // for `تأكد`, an argument codegen cannot synthesise).
        //
        // Any nearer binding of the same name suppresses the interception:
        // built-ins are the last tier of the lookup order (#262). The guard must
        // stay in step with the interpreter, the debug interpreter and codegen —
        // an earlier attempt changed only this site, so native called the user's
        // function while the interpreter still ran the builtin.
        if let ExprKind::Identifier(name) = &callee.kind {
            // `name` may be a local/global variable holding a function
            // value (a lambda, or a named function used as a value) rather
            // than a directly-callable declared function — dispatch through
            // CallIndirect in that case (issue #180). An untyped local still
            // counts as a function value as long as no *declared* function
            // of the same name exists, preserving today's behavior when a
            // local merely shadows an unrelated function name.
            let local_var = self.lookup_var(name);
            let local_ty = local_var.and_then(|v| self.var_types.get(&v.0).cloned());

            // A variable *known* to hold a function is tier 1/3 and outranks a
            // built-in as much as a declared function does; `ثابت طول = (س:
            // نص) => ٤٢` type-checks since #262 and must not still lower to
            // `ArrayLen`. Only the function-typed case counts here — an
            // untyped or non-function binding is a semantic error at the call,
            // and treating it as a shadow would hide the built-in behind it.
            let holds_function_value = matches!(local_ty, Some(IrType::Function { .. }))
                || (local_var.is_none()
                    && self.global_variables.contains(name)
                    && matches!(
                        self.global_var_types.get(name),
                        Some(IrType::Function { .. })
                    ));

            if !self.shadows_builtin(name) && !holds_function_value {
                if let Some(dest) = self.build_core_builtin_call(name, &arg_vars, args)? {
                    return Ok(dest);
                }
            }

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
                            let elem_ty = Self::array_elem_ty(&obj_type).unwrap_or_else(|| {
                                self.var_types
                                    .get(&value_var.0)
                                    .cloned()
                                    .unwrap_or(IrType::Int)
                            });
                            self.emit(Instruction::ArrayPush {
                                array: obj_var,
                                value: *value_var,
                                elem_ty,
                            });
                            self.var_types.insert(obj_var.0, obj_type);
                            return Ok(obj_var);
                        }
                    }
                    "احذف_آخر" => {
                        // `is_array` admits `Ptr(Void)`, an array whose element
                        // type was lost upstream, so this needs the fallback the
                        // global arm refuses on — there it means "not an array",
                        // here it means "an array we know less about".
                        let elem_ty = Self::array_elem_ty(&obj_type).unwrap_or(IrType::Int);
                        let dest = self.new_var();
                        self.emit(Instruction::ArrayPop {
                            dest,
                            array: obj_var,
                            elem_ty: elem_ty.clone(),
                        });
                        self.var_types.insert(dest.0, elem_ty);
                        return Ok(dest);
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

            // One lookup for both, so the declaring class and the return type
            // cannot disagree (issue #253). A miss keeps the receiver's own
            // class: this branch also carries `أي`-typed, interface-typed and
            // anonymous receivers, which have no entry to find.
            let (method_class, ret_ty) = self
                .resolve_instance_method(&class_id.0, property)
                .unwrap_or_else(|| (class_id.clone(), IrType::Ptr(Box::new(IrType::Void))));

            let dest = self.new_var();
            self.emit(Instruction::CallMethod {
                dest: Some(dest),
                object: obj_var,
                method: MethodId {
                    class: method_class,
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

        // A property is read through its accessor. The accessor may be declared
        // on an ancestor, and then it is the *ancestor* that must be named: both
        // backends mint the callee symbol from `MethodId.class`, and
        // `{subclass}::__احصل_{prop}` is never synthesized.
        if let Some(ref class_id) = class_id_opt {
            if let Some((defining_class, getter_name, prop_type)) =
                self.resolve_instance_property(&class_id.0, property)
            {
                self.emit(Instruction::CallMethod {
                    dest: Some(dest),
                    object: obj_var,
                    method: MethodId {
                        class: defining_class,
                        name: getter_name,
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
            match self.resolve_instance_field(&class_id.0, property) {
                Some((defining_class, idx, ty)) => (ty, idx, ClassId(defining_class)),
                None if self.declares_instance_property(&class_id.0, property) => {
                    return Err(missing_accessor_error(&class_id.0, property, true));
                }
                None if self.has_field_layout(&class_id.0) => {
                    return Err(unknown_member_error(&class_id.0, property));
                }
                // No layout to check against: `__anonymous__` object literals,
                // whose fields codegen resolves by name. Stay lenient.
                None => (IrType::Ptr(Box::new(IrType::Void)), 0, class_id),
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
                self.store_to_member(object, property, value_var)?;
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

    /// Stores `value` into `object.property`, routing through a property setter
    /// when the property has one and resolving a plain field's slot by name.
    ///
    /// Shared by plain and compound assignment. The compound path used to
    /// duplicate this with a bare `SetField` carrying an empty class, the
    /// property's own name rather than its `_`-prefixed backing field, and a
    /// hardcoded `index: 0` — so `ن.ص += 1` dropped the write silently in the
    /// interpreter (it stored a by-name field no getter reads) and corrupted
    /// slot 0 natively. That is the #239 defect one layer up, which is why the
    /// two paths are now one.
    fn store_to_member(&mut self, object: &Expr, property: &str, value: VarId) -> Result<()> {
        if let Some(class) = self.class_name_receiver(object) {
            if let Some((key, _)) = self.resolve_static_field(&class, property) {
                self.emit(Instruction::GlobalStore { name: key, value });
                return Ok(());
            }
            if let Some(setter) = self.resolve_static_property_setter(&class, property) {
                self.emit(Instruction::Call {
                    dest: None,
                    func: FuncId(setter),
                    args: vec![value],
                    ret_ty: IrType::Void,
                });
                return Ok(());
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

        // Mirrors the read path in `build_member`: the setter may be inherited,
        // and then `MethodId.class` must name the class that declares it.
        if let Some(ref class_id) = class_id_opt {
            if let Some((defining_class, setter_name)) =
                self.resolve_instance_property_setter(&class_id.0, property)
            {
                self.emit(Instruction::CallMethod {
                    dest: None,
                    object: obj_var,
                    method: MethodId {
                        class: defining_class,
                        name: setter_name,
                    },
                    args: vec![value],
                    ret_ty: IrType::Void,
                    virtual_dispatch: !is_super_receiver(object),
                });
                return Ok(());
            }
        }

        let (class_id, field_index) = if let Some(class_id) = class_id_opt {
            match self.resolve_instance_field(&class_id.0, property) {
                Some((defining_class, index, _)) => (ClassId(defining_class), index),
                None if self.declares_instance_property(&class_id.0, property) => {
                    return Err(missing_accessor_error(&class_id.0, property, false));
                }
                None if self.has_field_layout(&class_id.0) => {
                    return Err(unknown_member_error(&class_id.0, property));
                }
                None => (class_id, 0),
            }
        } else {
            (ClassId("".to_string()), 0)
        };

        self.emit(Instruction::SetField {
            object: obj_var,
            field: FieldId {
                class: class_id,
                name: property.to_string(),
                index: field_index,
            },
            value,
        });

        Ok(())
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
                self.store_to_member(object, property, result)?;
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
        // A lifted lambda is the one nested build that also needs its own
        // `parameters`/`var_types`: it re-numbers its parameters from zero, so
        // the enclosing function's entries for those same ids would otherwise
        // describe the lambda's slots. Declaration builds must NOT isolate
        // these — `build_func_decl` has always let them accumulate, and enum
        // payload types registered while lowering one function are read back
        // while lowering the next.
        let saved_parameters = std::mem::take(&mut self.parameters);
        let saved_var_types = std::mem::take(&mut self.var_types);

        let unlowerable = self.unlowerable_param_names(params, hint_params);
        self.begin_function(
            lambda_name.clone(),
            ir_params,
            IrType::Ptr(Box::new(IrType::Void)), // provisional; patched below
        )?;
        if let Some(name) = unlowerable.first() {
            self.block_native_lowering(NativeBlock::untyped(Self::untyped_param_reason(name)));
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
                    self.block_native_lowering(NativeBlock::untyped(
                        "قيم الإرجاع في الدالة السهمية ذات أنواع غير متوافقة",
                    ));
                    valued[0].clone()
                };

                if ret_ty != IrType::Void {
                    self.patch_block_lambda_returns(&ret_ty);
                }

                self.emit_implicit_return(&ret_ty);

                ret_ty
            }
        };

        if let Some(ref mut func) = self.current_function {
            func.return_type = real_ret_ty.clone();
        }

        self.end_function()?;

        self.resume_function_context(saved);
        self.parameters = saved_parameters;
        self.var_types = saved_var_types;

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
