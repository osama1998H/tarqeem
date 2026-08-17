//! Statement building for the IR builder.
//!
//! This module handles conversion of AST statements to IR instructions.

use crate::error::codes::ERR_NATIVE_EXCEPTIONS;
use crate::parser::{
    Block, CatchClause, ClassMember, Expr, ExprKind, MatchArm, Param, Pattern, PatternKind, Stmt,
    StmtKind, TypeAnnotation,
};
use crate::semantic::EXCEPTION_CLASS;

use super::super::{
    BinaryOp, BlockId, ClassId, Constant, EnumId, FieldId, Instruction, IrType, NativeBlock,
    Parameter, VarId, VariantId,
};
use super::{IrBuilder, IrError, Result};

impl IrBuilder {
    /// Build IR for a statement.
    pub(crate) fn build_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match &stmt.kind {
            StmtKind::VarDecl { name, ty, init, .. } => {
                self.build_var_decl(name, ty.as_ref(), init.as_ref())
            }
            StmtKind::FuncDecl {
                name,
                params,
                return_type,
                body,
                is_async,
                ..
            } => self.build_func_decl(name, params, return_type.as_ref(), body, *is_async),
            StmtKind::ClassDecl {
                name,
                extends,
                implements,
                members,
                ..
            } => self.build_class_decl(name, extends.as_ref(), implements, members),
            StmtKind::InterfaceDecl { .. } => Ok(()),
            StmtKind::EnumDecl { name, variants, .. } => {
                // Store variant field types for pattern matching
                // Use NFC-normalized names for consistent lookup with Arabic identifiers
                let normalized_enum_name = Self::normalize_name(name);
                for variant in variants {
                    let normalized_variant_name = Self::normalize_name(&variant.name);
                    let field_types: Vec<IrType> = variant
                        .fields
                        .iter()
                        .map(|f| self.convert_type(&f.ty))
                        .collect();
                    self.enum_variant_fields.insert(
                        (normalized_enum_name.clone(), normalized_variant_name),
                        field_types,
                    );
                }
                Ok(())
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => self.build_if(condition, then_branch, else_branch.as_ref()),
            StmtKind::While { condition, body } => self.build_while(condition, body),
            StmtKind::DoWhile { body, condition } => self.build_do_while(body, condition),
            StmtKind::For {
                init,
                condition,
                update,
                body,
            } => self.build_for(init.as_deref(), condition.as_ref(), update.as_ref(), body),
            StmtKind::ForIn {
                variable,
                iterable,
                body,
            } => self.build_for_in(variable, iterable, body),
            StmtKind::Match { expr, arms } => self.build_match(expr, arms),
            StmtKind::Return(expr) => self.build_return(expr.as_ref()),
            StmtKind::Break => self.build_break(),
            StmtKind::Continue => self.build_continue(),
            StmtKind::Try {
                body,
                catch,
                finally,
            } => self.build_try(body, catch.as_ref(), finally.as_ref()),
            StmtKind::Throw(expr) => self.build_throw(expr),
            StmtKind::Import { .. } => Ok(()),
            StmtKind::Export(export_items) => {
                use crate::parser::ExportItems;
                match export_items {
                    ExportItems::Declaration(inner) => self.build_stmt(inner),
                    // Named exports, wildcards, and re-exports don't generate IR
                    // They're module-level metadata handled during semantic analysis
                    ExportItems::Named(_)
                    | ExportItems::Wildcard { .. }
                    | ExportItems::NamedReexport { .. } => Ok(()),
                }
            }
            StmtKind::Expr(expr) => {
                self.build_expr(expr)?;
                Ok(())
            }
            StmtKind::Block(block) => self.build_block(block),
        }
    }

    /// Build IR for a variable declaration.
    pub(crate) fn build_var_decl(
        &mut self,
        name: &str,
        ty: Option<&TypeAnnotation>,
        init: Option<&Expr>,
    ) -> Result<()> {
        if self.global_variables.contains(name) {
            if let Some(init_expr) = init {
                // Program mode: top-level initializers were already emitted
                // into __global_init__; the fourth pass revisits this decl
                // *outside* any function, so re-building it here would only
                // lift an orphaned duplicate lambda and drop the store on
                // the floor (`emit` has no current function to write to).
                if self.try_evaluate_const(init_expr).is_none() && self.current_function.is_some() {
                    self.build_global_initializer(name, init_expr)?;
                }
            }
            return Ok(());
        }

        let ir_type = if let Some(t) = ty {
            self.convert_type(t)
        } else if let Some(init_expr) = init {
            self.infer_expr_type(init_expr)
        } else {
            IrType::Ptr(Box::new(IrType::Void))
        };

        let ptr = self.new_var();
        self.emit(Instruction::Alloca {
            dest: ptr,
            ty: ir_type.clone(),
        });

        self.var_types.insert(ptr.0, ir_type.clone());

        if let Some(init_expr) = init {
            let value = self.build_init_expr(init_expr, Some(&ir_type))?;
            // See the identical correction in `build_global_initializer`.
            // The already-emitted `Alloca` keeps the provisional type, which
            // is harmless: every type this can re-tag is pointer-width, so
            // the slot is correctly sized either way, and `Load`/`Store` read
            // their type from `var_types` rather than from the `Alloca`.
            if Self::is_unknown_ir_type(Some(&ir_type)) {
                if let Some(real_ty) = self.var_types.get(&value.0).cloned() {
                    self.var_types.insert(ptr.0, real_ty);
                }
            }
            let value = self.coerce_int_to_float(value, Some(&ir_type), init_expr);
            self.emit(Instruction::Store { ptr, value });
        }

        self.variables.insert(name.to_string(), ptr);

        Ok(())
    }

    /// Emits the `GlobalStore` for one non-const global initializer,
    /// threading the registered global type as a lambda hint and correcting
    /// a provisionally-typed lambda global afterward. Shared by
    /// `build_var_decl` (script mode) and the program-mode
    /// `__global_init__`/`__main__` static-initializer loops in
    /// `IrBuilder::build` — the program-mode path previously used a bare
    /// `build_expr`, so a fully annotated global lambda still lifted with
    /// untyped params.
    pub(crate) fn build_global_initializer(&mut self, name: &str, init_expr: &Expr) -> Result<()> {
        let global_ty = self.global_var_types.get(name).cloned();
        let value = self.build_init_expr(init_expr, global_ty.as_ref())?;
        // An unannotated global's registered type is only a provisional
        // `infer_expr_type` guess made before any body was built, and that
        // pass has no arm for a lambda literal (nor for a call through one),
        // so it lands on the `Ptr(Void)` unknown sentinel. Now that the
        // initializer is actually built, its real type is known — adopt it,
        // or every later `GlobalLoad` reads the slot at the wrong type
        // (`load ptr` from an `i64` slot, which `اطبع` then dereferences and
        // segfaults natively; the interpreter is dynamically typed and
        // unaffected either way).
        if Self::is_unknown_ir_type(global_ty.as_ref()) {
            if let Some(real_ty) = self.var_types.get(&value.0).cloned() {
                self.set_global_type(name, real_ty);
            }
        }
        let value = self.coerce_int_to_float(value, global_ty.as_ref(), init_expr);
        self.emit(Instruction::GlobalStore {
            name: name.to_string(),
            value,
        });
        Ok(())
    }

    /// `Ptr(Void)` is the builder's "type not known yet" sentinel (see
    /// `infer_expr_type`'s fallthrough); a slot carrying it may be safely
    /// re-typed once the real value has been built.
    pub(crate) fn is_unknown_ir_type(ty: Option<&IrType>) -> bool {
        match ty {
            None => true,
            Some(IrType::Ptr(inner)) => **inner == IrType::Void,
            Some(_) => false,
        }
    }

    /// Re-type a global in both places the type is recorded: the builder's
    /// lookup table (read by `build_identifier` for `GlobalLoad`) and the
    /// module's global list (which codegen emits `@g = global <ty>` from).
    fn set_global_type(&mut self, name: &str, ty: IrType) {
        self.global_var_types.insert(name.to_string(), ty.clone());
        if let Some(entry) = self.module.globals.iter_mut().find(|(n, _, _)| n == name) {
            entry.1 = ty;
        }
    }

    /// Builds a variable's initializer expression, threading a declared
    /// function-type annotation through to `build_lambda_with_hint` when the
    /// initializer is a bare lambda literal (e.g.
    /// `ثابت جمع: (عدد، عدد) -> عدد = (أ، ب) => أ + ب؛`) — this is what lets
    /// an unannotated lambda param pick up a concrete type from the
    /// surrounding declaration instead of defaulting to `Ptr(Void)`.
    fn build_init_expr(&mut self, init_expr: &Expr, declared_ty: Option<&IrType>) -> Result<VarId> {
        if let ExprKind::Lambda { params, body } = &init_expr.kind {
            if matches!(declared_ty, Some(IrType::Function { .. })) {
                return self.build_lambda_with_hint(params, body, declared_ty);
            }
        }
        self.build_expr(init_expr)
    }

    /// Implicit عدد → عدد_عشري conversion (spec §5.6): storing an integer
    /// value into a float-typed slot must go through IntToFloat, otherwise
    /// native codegen stores the raw i64 bit pattern into a double slot.
    fn coerce_int_to_float(
        &mut self,
        value: VarId,
        target_ty: Option<&IrType>,
        init_expr: &Expr,
    ) -> VarId {
        if target_ty == Some(&IrType::Float) && self.infer_expr_type(init_expr) == IrType::Int {
            let coerced = self.new_var();
            self.emit(Instruction::IntToFloat {
                dest: coerced,
                src: value,
            });
            self.var_types.insert(coerced.0, IrType::Float);
            coerced
        } else {
            value
        }
    }

    /// Build IR for a function declaration.
    pub(crate) fn build_func_decl(
        &mut self,
        name: &str,
        params: &[Param],
        return_type: Option<&TypeAnnotation>,
        body: &Block,
        is_async: bool,
    ) -> Result<()> {
        let ir_params: Vec<Parameter> = params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let ty =
                    p.ty.as_ref()
                        .map(|t| self.convert_type(t))
                        .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                Parameter {
                    id: VarId(i as u32),
                    name: p.name.clone(),
                    ty,
                }
            })
            .collect();

        let ret_type = return_type
            .map(|t| self.convert_type(t))
            .unwrap_or(IrType::Void);
        let is_void_function = ret_type == IrType::Void;

        let saved = self.suspend_function_context();

        let unlowerable = self.unlowerable_param_names(params, &[]);
        self.begin_function(name.to_string(), ir_params, ret_type)?;
        // A declared function's unannotated parameter lowers to `Ptr(Void)`
        // exactly as a lambda's does, so it is the same native ABI hazard —
        // record it here too rather than keying the codegen guard on the
        // `__lambda_` name prefix.
        if let Some(p) = unlowerable.first() {
            self.block_native_lowering(NativeBlock::untyped(Self::untyped_param_reason(p)));
        }

        if let Some(ref mut func) = self.current_function {
            func.is_async = is_async;
        }

        for stmt in &body.statements {
            self.build_stmt(stmt)?;
        }

        let needs_return = if is_void_function {
            if let Some(ref func) = self.current_function {
                let current_block_id = self.current_block;
                func.blocks
                    .iter()
                    .find(|b| b.id == current_block_id)
                    .map(|b| !b.has_terminator())
                    .unwrap_or(false)
            } else {
                false
            }
        } else {
            false
        };
        if needs_return {
            self.emit(Instruction::Return { value: None });
        }

        self.end_function()?;

        self.resume_function_context(saved);

        Ok(())
    }

    /// Build IR for a class declaration.
    pub(crate) fn build_class_decl(
        &mut self,
        name: &str,
        extends: Option<&String>,
        _implements: &[String],
        members: &[ClassMember],
    ) -> Result<()> {
        // A class declared inside a function or block never reaches the
        // collection pass, which walks top-level statements only
        // (`IrBuilder::build`). Collect it on demand so its field layout — and
        // its entry in `module.classes` — exist before any member is lowered.
        // Without this, `backing_field_index` has no layout to consult.
        if !self.class_fields.contains_key(name) {
            self.collect_class(name, extends, members)?;
        }

        if let Some(parent) = extends {
            for class in &mut self.module.classes {
                if class.name == name {
                    class.parent = Some(ClassId(parent.clone()));
                    break;
                }
            }
        }

        for member in members {
            match member {
                ClassMember::Method {
                    name: method_name,
                    params,
                    return_type,
                    body,
                    is_static,
                    is_async,
                    ..
                } => {
                    let mangled_name = format!("{}::{}", name, method_name);

                    // مشترك methods have no receiver: omit هذا entirely rather
                    // than binding it to a parameter the caller never passes
                    // (mirrors the static branch of build_property_getter/setter).
                    let mut method_params: Vec<Parameter> = if *is_static {
                        Vec::new()
                    } else {
                        vec![Parameter {
                            id: VarId(0),
                            name: "هذا".to_string(), // "this" in Arabic
                            ty: IrType::Struct(ClassId(name.to_string())),
                        }]
                    };
                    let base_index = method_params.len() as u32;

                    for (i, p) in params.iter().enumerate() {
                        let ty =
                            p.ty.as_ref()
                                .map(|t| self.convert_type(t))
                                .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                        method_params.push(Parameter {
                            id: VarId(base_index + i as u32),
                            name: p.name.clone(),
                            ty,
                        });
                    }

                    let ret_type = return_type
                        .as_ref()
                        .map(|t| self.convert_type(t))
                        .unwrap_or(IrType::Void);

                    let saved = self.suspend_function_context();

                    self.begin_function(mangled_name, method_params, ret_type.clone())?;

                    if let Some(ref mut func) = self.current_function {
                        func.is_async = *is_async;
                    }

                    if !*is_static {
                        self.variables.insert("هذا".to_string(), VarId(0));
                        self.variables.insert("this".to_string(), VarId(0));
                    }

                    for stmt in &body.statements {
                        self.build_stmt(stmt)?;
                    }

                    self.emit_implicit_return(&ret_type);

                    self.end_function()?;

                    self.resume_function_context(saved);
                }
                ClassMember::Constructor { params, body, .. } => {
                    let mangled_name = format!("{}::منشئ", name);

                    let mut ctor_params: Vec<Parameter> = vec![Parameter {
                        id: VarId(0),
                        name: "هذا".to_string(),
                        ty: IrType::Struct(ClassId(name.to_string())),
                    }];

                    for (i, p) in params.iter().enumerate() {
                        let ty =
                            p.ty.as_ref()
                                .map(|t| self.convert_type(t))
                                .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                        ctor_params.push(Parameter {
                            id: VarId((i + 1) as u32),
                            name: p.name.clone(),
                            ty,
                        });
                    }

                    let saved = self.suspend_function_context();

                    self.begin_function(mangled_name, ctor_params, IrType::Void)?;

                    self.variables.insert("هذا".to_string(), VarId(0));
                    self.variables.insert("this".to_string(), VarId(0));

                    for stmt in &body.statements {
                        self.build_stmt(stmt)?;
                    }

                    self.emit_implicit_return(&IrType::Void);

                    self.end_function()?;

                    self.resume_function_context(saved);
                }
                ClassMember::Field { .. } => {}

                ClassMember::Property {
                    name: prop_name,
                    ty,
                    accessors,
                    is_static,
                    ..
                } => {
                    let prop_type = self.convert_type(ty);

                    let has_getter = accessors
                        .iter()
                        .any(|a| matches!(a, crate::parser::PropertyAccessor::Get { .. }));

                    if has_getter || accessors.is_empty() {
                        self.build_property_getter(
                            name, prop_name, &prop_type, accessors, *is_static,
                        )?;
                    }

                    let has_setter = accessors
                        .iter()
                        .any(|a| matches!(a, crate::parser::PropertyAccessor::Set { .. }));

                    if has_setter || accessors.is_empty() {
                        self.build_property_setter(
                            name, prop_name, &prop_type, accessors, *is_static,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Slot of an auto-property's backing field inside its declaring class.
    ///
    /// `collect_class` pushes `_{prop}` into `class_fields` in declaration
    /// order and never merges parent fields, so the index this returns is
    /// own-class-relative — the convention codegen expects before it adds
    /// `inherited_field_count`.
    ///
    /// Both accessor sites used to hardcode `0` (issue #239). Because the
    /// interpreter resolves `GetField`/`SetField` by name and drops the index,
    /// only natively compiled code noticed: every auto-property on a class
    /// shared slot 0, so they aliased each other *and* a write through one
    /// overwrote whatever real field occupied that slot. A missing backing
    /// field is an error rather than a fallback to 0 — falling back is the bug.
    ///
    /// Reaching the error means `collect_class` and this synthesis disagree
    /// about the class's layout, which no source program should be able to
    /// cause: it is an internal invariant, not a user diagnostic, so it carries
    /// no ت/د error code — but it is still bilingual, since a contributor
    /// reading it is exactly who it is for.
    fn backing_field_index(&self, class_name: &str, backing_field: &str) -> Result<u32> {
        self.get_field_info(class_name, backing_field)
            .map(|(index, _)| index)
            .ok_or_else(|| {
                IrError::new(format!(
                    "الحقل الداعم '{backing_field}' للخاصية غير موجود في تخطيط الصنف '{class_name}' \
                     / property backing field '{backing_field}' is missing from the layout of class '{class_name}'"
                ))
            })
    }

    /// Build IR for a property getter.
    fn build_property_getter(
        &mut self,
        class_name: &str,
        prop_name: &str,
        prop_type: &IrType,
        accessors: &[crate::parser::PropertyAccessor],
        is_static: bool,
    ) -> Result<()> {
        let getter_name = format!("{}::__احصل_{}", class_name, prop_name);
        let getter_params = if is_static {
            vec![]
        } else {
            vec![Parameter {
                id: VarId(0),
                name: "هذا".to_string(),
                ty: IrType::Struct(ClassId(class_name.to_string())),
            }]
        };

        let saved = self.suspend_function_context();

        self.begin_function(getter_name, getter_params, prop_type.clone())?;

        if !is_static {
            self.variables.insert("هذا".to_string(), VarId(0));
        }

        for accessor in accessors {
            if let crate::parser::PropertyAccessor::Get { body, .. } = accessor {
                match body {
                    crate::parser::PropertyAccessorBody::Block(block) => {
                        for stmt in &block.statements {
                            self.build_stmt(stmt)?;
                        }
                    }
                    crate::parser::PropertyAccessorBody::Expr(expr) => {
                        let result = self.build_expr(expr)?;
                        self.emit(Instruction::Return {
                            value: Some(result),
                        });
                    }
                }
                break;
            }
        }

        if accessors.is_empty() {
            let result = self.new_var();
            if is_static {
                // مشترك خاصية has no هذا: the backing field lives as a
                // global, not an instance slot (see register_static_global).
                self.emit(Instruction::GlobalLoad {
                    dest: result,
                    name: format!("{}::_{}", class_name, prop_name),
                    ty: prop_type.clone(),
                });
            } else {
                let this_var = VarId(0);
                let backing_field = format!("_{}", prop_name);
                let index = self.backing_field_index(class_name, &backing_field)?;
                self.emit(Instruction::GetField {
                    dest: result,
                    object: this_var,
                    field: FieldId {
                        class: ClassId(class_name.to_string()),
                        name: backing_field,
                        index,
                    },
                    ty: prop_type.clone(),
                });
            }
            self.emit(Instruction::Return {
                value: Some(result),
            });
        }

        self.end_function()?;

        self.resume_function_context(saved);

        Ok(())
    }

    /// Build IR for a property setter.
    fn build_property_setter(
        &mut self,
        class_name: &str,
        prop_name: &str,
        prop_type: &IrType,
        accessors: &[crate::parser::PropertyAccessor],
        is_static: bool,
    ) -> Result<()> {
        let setter_name = format!("{}::__عيّن_{}", class_name, prop_name);
        let mut setter_params = if is_static {
            vec![]
        } else {
            vec![Parameter {
                id: VarId(0),
                name: "هذا".to_string(),
                ty: IrType::Struct(ClassId(class_name.to_string())),
            }]
        };
        setter_params.push(Parameter {
            id: VarId(setter_params.len() as u32),
            name: "قيمة".to_string(),
            ty: prop_type.clone(),
        });

        let saved = self.suspend_function_context();

        self.begin_function(setter_name, setter_params, IrType::Void)?;

        if !is_static {
            self.variables.insert("هذا".to_string(), VarId(0));
            self.variables.insert("قيمة".to_string(), VarId(1));
        } else {
            self.variables.insert("قيمة".to_string(), VarId(0));
        }

        for accessor in accessors {
            if let crate::parser::PropertyAccessor::Set {
                param_name, body, ..
            } = accessor
            {
                self.variables
                    .insert(param_name.clone(), VarId(if is_static { 0 } else { 1 }));
                for stmt in &body.statements {
                    self.build_stmt(stmt)?;
                }
                break;
            }
        }

        if accessors.is_empty() {
            if is_static {
                // See build_property_getter: the backing field is a global.
                self.emit(Instruction::GlobalStore {
                    name: format!("{}::_{}", class_name, prop_name),
                    value: VarId(0), // قيمة is the only (and first) parameter
                });
            } else {
                let this_var = VarId(0);
                let value_var = VarId(1);
                let backing_field = format!("_{}", prop_name);
                let index = self.backing_field_index(class_name, &backing_field)?;
                self.emit(Instruction::SetField {
                    object: this_var,
                    field: FieldId {
                        class: ClassId(class_name.to_string()),
                        name: backing_field,
                        index,
                    },
                    value: value_var,
                });
            }
        }

        self.emit(Instruction::Return { value: None });
        self.end_function()?;

        self.resume_function_context(saved);

        Ok(())
    }

    /// Build IR for an if statement.
    pub(crate) fn build_if(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        else_branch: Option<&Block>,
    ) -> Result<()> {
        let cond_var = self.build_expr(condition)?;

        let then_block = self.new_block(Some("then".to_string()));
        let merge_block = self.new_block(Some("merge".to_string()));

        let else_target = if else_branch.is_some() {
            self.new_block(Some("else".to_string()))
        } else {
            merge_block
        };

        self.emit(Instruction::Branch {
            cond: cond_var,
            then_block,
            else_block: else_target,
        });

        self.switch_to_block(then_block);
        self.push_scope();
        for stmt in &then_branch.statements {
            self.build_stmt(stmt)?;
        }
        self.pop_scope();

        if let Some(ref func) = self.current_function {
            if let Some(block) = func.get_block(self.current_block) {
                if !block.has_terminator() {
                    self.emit(Instruction::Jump {
                        target: merge_block,
                    });
                }
            }
        }

        if let Some(else_body) = else_branch {
            self.switch_to_block(else_target);
            self.push_scope();
            for stmt in &else_body.statements {
                self.build_stmt(stmt)?;
            }
            self.pop_scope();

            if let Some(ref func) = self.current_function {
                if let Some(block) = func.get_block(self.current_block) {
                    if !block.has_terminator() {
                        self.emit(Instruction::Jump {
                            target: merge_block,
                        });
                    }
                }
            }
        }

        self.switch_to_block(merge_block);
        Ok(())
    }

    /// Build IR for a while loop.
    pub(crate) fn build_while(&mut self, condition: &Expr, body: &Block) -> Result<()> {
        let cond_block = self.new_block(Some("while.cond".to_string()));
        let body_block = self.new_block(Some("while.body".to_string()));
        let exit_block = self.new_block(Some("while.exit".to_string()));

        self.emit(Instruction::Jump { target: cond_block });

        self.switch_to_block(cond_block);
        let cond_var = self.build_expr(condition)?;
        self.emit(Instruction::Branch {
            cond: cond_var,
            then_block: body_block,
            else_block: exit_block,
        });

        self.loop_stack.push((cond_block, exit_block));

        self.switch_to_block(body_block);
        self.push_scope();
        for stmt in &body.statements {
            self.build_stmt(stmt)?;
        }
        self.pop_scope();

        if let Some(ref func) = self.current_function {
            if let Some(block) = func.get_block(self.current_block) {
                if !block.has_terminator() {
                    self.emit(Instruction::Jump { target: cond_block });
                }
            }
        }

        self.loop_stack.pop();

        self.switch_to_block(exit_block);
        Ok(())
    }

    /// Build IR for a do-while loop.
    pub(crate) fn build_do_while(&mut self, body: &Block, condition: &Expr) -> Result<()> {
        let body_block = self.new_block(Some("dowhile.body".to_string()));
        let cond_block = self.new_block(Some("dowhile.cond".to_string()));
        let exit_block = self.new_block(Some("dowhile.exit".to_string()));

        self.emit(Instruction::Jump { target: body_block });

        self.loop_stack.push((cond_block, exit_block));

        self.switch_to_block(body_block);
        self.push_scope();
        for stmt in &body.statements {
            self.build_stmt(stmt)?;
        }
        self.pop_scope();

        if let Some(ref func) = self.current_function {
            if let Some(block) = func.get_block(self.current_block) {
                if !block.has_terminator() {
                    self.emit(Instruction::Jump { target: cond_block });
                }
            }
        }

        self.switch_to_block(cond_block);
        let cond_var = self.build_expr(condition)?;
        self.emit(Instruction::Branch {
            cond: cond_var,
            then_block: body_block,
            else_block: exit_block,
        });

        self.loop_stack.pop();

        self.switch_to_block(exit_block);
        Ok(())
    }

    /// Build IR for a for loop.
    pub(crate) fn build_for(
        &mut self,
        init: Option<&Stmt>,
        condition: Option<&Expr>,
        update: Option<&Expr>,
        body: &Block,
    ) -> Result<()> {
        self.push_scope();

        if let Some(init_stmt) = init {
            self.build_stmt(init_stmt)?;
        }

        let cond_block = self.new_block(Some("for.cond".to_string()));
        let body_block = self.new_block(Some("for.body".to_string()));
        let update_block = self.new_block(Some("for.update".to_string()));
        let exit_block = self.new_block(Some("for.exit".to_string()));

        self.emit(Instruction::Jump { target: cond_block });

        self.switch_to_block(cond_block);
        if let Some(cond_expr) = condition {
            let cond_var = self.build_expr(cond_expr)?;
            self.emit(Instruction::Branch {
                cond: cond_var,
                then_block: body_block,
                else_block: exit_block,
            });
        } else {
            self.emit(Instruction::Jump { target: body_block });
        }

        self.loop_stack.push((update_block, exit_block));

        self.switch_to_block(body_block);
        for stmt in &body.statements {
            self.build_stmt(stmt)?;
        }

        if let Some(ref func) = self.current_function {
            if let Some(block) = func.get_block(self.current_block) {
                if !block.has_terminator() {
                    self.emit(Instruction::Jump {
                        target: update_block,
                    });
                }
            }
        }

        self.switch_to_block(update_block);
        if let Some(update_expr) = update {
            self.build_expr(update_expr)?;
        }
        self.emit(Instruction::Jump { target: cond_block });

        self.loop_stack.pop();

        self.pop_scope();
        self.switch_to_block(exit_block);
        Ok(())
    }

    /// Build IR for a for-in loop.
    pub(crate) fn build_for_in(
        &mut self,
        variable: &str,
        iterable: &Expr,
        body: &Block,
    ) -> Result<()> {
        let array_var = self.build_expr(iterable)?;

        let len_var = self.new_var();
        self.emit(Instruction::ArrayLen {
            dest: len_var,
            array: array_var,
        });

        let index_ptr = self.new_var();
        self.emit(Instruction::Alloca {
            dest: index_ptr,
            ty: IrType::Int,
        });
        let zero = self.new_var();
        self.emit(Instruction::Const {
            dest: zero,
            value: Constant::Int(0),
            ty: IrType::Int,
        });
        self.emit(Instruction::Store {
            ptr: index_ptr,
            value: zero,
        });

        let cond_block = self.new_block(Some("forin.cond".to_string()));
        let body_block = self.new_block(Some("forin.body".to_string()));
        let update_block = self.new_block(Some("forin.update".to_string()));
        let exit_block = self.new_block(Some("forin.exit".to_string()));

        self.emit(Instruction::Jump { target: cond_block });

        self.switch_to_block(cond_block);
        let index_val = self.new_var();
        self.emit(Instruction::Load {
            dest: index_val,
            ptr: index_ptr,
            ty: IrType::Int,
        });
        let cond = self.new_var();
        self.emit(Instruction::Binary {
            dest: cond,
            op: BinaryOp::Lt,
            left: index_val,
            right: len_var,
            ty: IrType::Bool,
        });
        self.emit(Instruction::Branch {
            cond,
            then_block: body_block,
            else_block: exit_block,
        });

        self.loop_stack.push((update_block, exit_block));

        self.switch_to_block(body_block);
        self.push_scope();

        let index_val2 = self.new_var();
        self.emit(Instruction::Load {
            dest: index_val2,
            ptr: index_ptr,
            ty: IrType::Int,
        });

        let elem_ty = if let Some(array_ty) = self.var_types.get(&array_var.0) {
            match array_ty {
                IrType::Array(inner, _) => (**inner).clone(),
                IrType::Ptr(inner) => match &**inner {
                    IrType::Array(elem, _) => (**elem).clone(),
                    _ => IrType::Ptr(Box::new(IrType::Void)),
                },
                _ => IrType::Ptr(Box::new(IrType::Void)),
            }
        } else {
            IrType::Ptr(Box::new(IrType::Void))
        };

        let elem = self.new_var();
        self.emit(Instruction::ArrayGet {
            dest: elem,
            array: array_var,
            index: index_val2,
            elem_ty: elem_ty.clone(),
        });

        let elem_ptr = self.new_var();
        self.emit(Instruction::Alloca {
            dest: elem_ptr,
            ty: elem_ty.clone(),
        });
        self.emit(Instruction::Store {
            ptr: elem_ptr,
            value: elem,
        });

        self.var_types.insert(elem.0, elem_ty.clone());
        self.var_types.insert(elem_ptr.0, elem_ty);

        self.variables.insert(variable.to_string(), elem_ptr);

        for stmt in &body.statements {
            self.build_stmt(stmt)?;
        }

        self.pop_scope();

        if let Some(ref func) = self.current_function {
            if let Some(block) = func.get_block(self.current_block) {
                if !block.has_terminator() {
                    self.emit(Instruction::Jump {
                        target: update_block,
                    });
                }
            }
        }

        self.switch_to_block(update_block);
        let index_val3 = self.new_var();
        self.emit(Instruction::Load {
            dest: index_val3,
            ptr: index_ptr,
            ty: IrType::Int,
        });
        let one = self.new_var();
        self.emit(Instruction::Const {
            dest: one,
            value: Constant::Int(1),
            ty: IrType::Int,
        });
        let new_index = self.new_var();
        self.emit(Instruction::Binary {
            dest: new_index,
            op: BinaryOp::Add,
            left: index_val3,
            right: one,
            ty: IrType::Int,
        });
        self.emit(Instruction::Store {
            ptr: index_ptr,
            value: new_index,
        });
        self.emit(Instruction::Jump { target: cond_block });

        self.loop_stack.pop();

        self.switch_to_block(exit_block);
        Ok(())
    }

    /// Build IR for a match statement.
    pub(crate) fn build_match(&mut self, expr: &Expr, arms: &[MatchArm]) -> Result<()> {
        let match_val = self.build_expr(expr)?;
        let exit_block = self.new_block(Some("match.exit".to_string()));

        let arm_blocks: Vec<BlockId> = arms
            .iter()
            .enumerate()
            .map(|(i, _)| self.new_block(Some(format!("match.arm{}", i))))
            .collect();

        for (i, arm) in arms.iter().enumerate() {
            let patterns = &arm.patterns;

            for (p_idx, pattern) in patterns.iter().enumerate() {
                let else_block = if p_idx + 1 < patterns.len() {
                    self.new_block(Some(format!("match.arm{}.pat{}", i, p_idx + 1)))
                } else if i + 1 < arms.len() {
                    self.new_block(Some(format!("match.check{}", i + 1)))
                } else {
                    exit_block
                };

                // Build pattern comparison based on pattern kind
                self.build_pattern_check(pattern, match_val, arm_blocks[i], else_block)?;
                self.switch_to_block(else_block);
            }
        }

        for (i, arm) in arms.iter().enumerate() {
            self.switch_to_block(arm_blocks[i]);
            self.push_scope();

            // Add pattern bindings to scope before building arm body
            for pattern in &arm.patterns {
                self.add_pattern_bindings(pattern, match_val)?;
            }

            for stmt in &arm.body.statements {
                self.build_stmt(stmt)?;
            }
            self.pop_scope();

            if let Some(ref func) = self.current_function {
                if let Some(block) = func.get_block(self.current_block) {
                    if !block.has_terminator() {
                        self.emit(Instruction::Jump { target: exit_block });
                    }
                }
            }
        }

        self.switch_to_block(exit_block);
        Ok(())
    }

    /// Build pattern check and branch.
    pub(crate) fn build_pattern_check(
        &mut self,
        pattern: &Pattern,
        match_val: VarId,
        then_block: BlockId,
        else_block: BlockId,
    ) -> Result<()> {
        match &pattern.kind {
            PatternKind::Literal(expr) => {
                // Compare with literal value
                let pattern_val = self.build_expr(expr)?;
                let cmp = self.new_var();
                self.emit(Instruction::Binary {
                    dest: cmp,
                    op: BinaryOp::Eq,
                    left: match_val,
                    right: pattern_val,
                    ty: IrType::Bool,
                });
                self.emit(Instruction::Branch {
                    cond: cmp,
                    then_block,
                    else_block,
                });
            }
            PatternKind::Identifier(_) | PatternKind::Wildcard => {
                // Always matches - unconditional jump
                self.emit(Instruction::Jump { target: then_block });
            }
            PatternKind::EnumVariant { variant_name, .. } => {
                // Check discriminant for enum variant match
                // Get discriminant from match value
                let disc = self.new_var();
                self.emit(Instruction::GetDiscriminant {
                    dest: disc,
                    value: match_val,
                });

                // Compare with expected discriminant (use FNV-1a hash of variant name)
                // Must match the calculation in build_enum_variant
                let expected = self.new_var();
                let disc_val = Self::calculate_discriminant(variant_name) as i64;
                self.emit(Instruction::Const {
                    dest: expected,
                    value: Constant::Int(disc_val),
                    ty: IrType::Int,
                });

                let cmp = self.new_var();
                self.emit(Instruction::Binary {
                    dest: cmp,
                    op: BinaryOp::Eq,
                    left: disc,
                    right: expected,
                    ty: IrType::Bool,
                });

                self.emit(Instruction::Branch {
                    cond: cmp,
                    then_block,
                    else_block,
                });
            }
        }
        Ok(())
    }

    /// Add pattern bindings to scope.
    pub(crate) fn add_pattern_bindings(
        &mut self,
        pattern: &Pattern,
        match_val: VarId,
    ) -> Result<()> {
        match &pattern.kind {
            PatternKind::Identifier(name) => {
                // Bind the identifier to the match value
                // Mark as parameter so it's treated as a direct value, not a pointer
                self.variables.insert(name.clone(), match_val);
                self.parameters.insert(match_val.0);
            }
            PatternKind::EnumVariant {
                enum_name,
                variant_name,
                bindings,
            } => {
                // NFC-normalize names for consistent Arabic identifier handling
                let normalized_enum = Self::normalize_name(enum_name);
                let normalized_variant = Self::normalize_name(variant_name);

                // Look up field types from enum declaration
                let field_types = self
                    .enum_variant_fields
                    .get(&(normalized_enum.clone(), normalized_variant.clone()))
                    .cloned()
                    .unwrap_or_default();

                // Extract variant fields and bind them
                for (i, binding) in bindings.iter().enumerate() {
                    // Get the actual field type, defaulting to Int if not found
                    let field_ty = field_types.get(i).cloned().unwrap_or(IrType::Int);

                    let field_val = self.new_var();
                    self.emit(Instruction::GetVariantField {
                        dest: field_val,
                        value: match_val,
                        variant: VariantId {
                            enum_id: EnumId(normalized_enum.clone()),
                            name: normalized_variant.clone(),
                            discriminant: Self::calculate_discriminant(&normalized_variant),
                        },
                        field_index: i as u32,
                        ty: field_ty,
                    });
                    // Mark as parameter so it's treated as a direct value, not a pointer
                    self.variables.insert(binding.clone(), field_val);
                    self.parameters.insert(field_val.0);
                }
            }
            PatternKind::Literal(_) | PatternKind::Wildcard => {
                // No bindings for literals or wildcards
            }
        }
        Ok(())
    }

    /// Build IR for a return statement.
    pub(crate) fn build_return(&mut self, expr: Option<&Expr>) -> Result<()> {
        let value = if let Some(e) = expr {
            Some(self.build_expr(e)?)
        } else {
            None
        };

        self.emit(Instruction::Return { value });
        Ok(())
    }

    /// Build IR for a break statement.
    pub(crate) fn build_break(&mut self) -> Result<()> {
        if let Some((_, exit_block)) = self.loop_stack.last() {
            self.emit(Instruction::Jump {
                target: *exit_block,
            });
            Ok(())
        } else {
            Err(IrError::new("'أوقف' خارج حلقة"))
        }
    }

    /// Build IR for a continue statement.
    pub(crate) fn build_continue(&mut self) -> Result<()> {
        if let Some((continue_block, _)) = self.loop_stack.last() {
            self.emit(Instruction::Jump {
                target: *continue_block,
            });
            Ok(())
        } else {
            Err(IrError::new("'استمر' خارج حلقة"))
        }
    }

    /// Build IR for a try-catch-finally statement.
    pub(crate) fn build_try(
        &mut self,
        body: &Block,
        catch: Option<&CatchClause>,
        finally: Option<&Block>,
    ) -> Result<()> {
        // Blocks exist only for the clauses that were written. A `حاول` with no
        // `التقط` used to get a catch block anyway, whose whole body was a jump
        // past the exception — so `TryBegin` registered a handler that silently
        // discarded whatever was thrown. With no handler pushed, the throw
        // propagates instead (issue #181). The unconditional `finally` block was
        // likewise emitted empty and terminator-less, which LLVM rejects.
        let catch_block = catch.map(|_| self.new_block(Some("catch".to_string())));
        let finally_block = finally.map(|_| self.new_block(Some("finally".to_string())));
        let exit_block = self.new_block(Some("try.exit".to_string()));

        let after_clause = finally_block.unwrap_or(exit_block);

        if let Some(catch_block) = catch_block {
            self.emit(Instruction::TryBegin { catch_block });
        }

        self.push_scope();
        for stmt in &body.statements {
            self.build_stmt(stmt)?;
        }
        self.pop_scope();

        // `ارمِ` is a terminator (`Instruction::has_terminator`), so a body
        // ending in one must not be followed by `TryEnd` and a jump: the
        // interpreter never reaches them and native codegen emits instructions
        // after a terminator. The handler also has to stay pushed for the throw
        // that just happened to find it.
        if self.current_block_needs_terminator() {
            if catch_block.is_some() {
                self.emit(Instruction::TryEnd);
            }
            self.emit(Instruction::Jump {
                target: after_clause,
            });
        }

        if let (Some(catch_block), Some(catch_clause)) = (catch_block, catch) {
            self.switch_to_block(catch_block);
            self.push_scope();

            let exception_var = self.new_var();
            self.emit(Instruction::GetException {
                dest: exception_var,
            });
            self.variables
                .insert(catch_clause.param.clone(), exception_var);
            // `GetException` writes the exception itself, not a slot holding it.
            // Without this the name resolves as an alloca and every use of the
            // catch parameter lowers to a `Load` of a non-pointer, failing with
            // `متوقع ptr، وُجد object` (issue #181). Same reason
            // `add_pattern_bindings` marks its `تطابق` bindings.
            self.parameters.insert(exception_var.0);
            // The analyzer types the catch parameter as `استثناء`; the IR needs
            // the same, or member access falls into the unknown-class branch of
            // `build_member` and `خ.رسالة` comes back as `Ptr(Void)` — enough to
            // make `"…" + خ.رسالة` lower to integer addition.
            self.var_types.insert(
                exception_var.0,
                IrType::Struct(ClassId(EXCEPTION_CLASS.to_string())),
            );

            for stmt in &catch_clause.body.statements {
                self.build_stmt(stmt)?;
            }
            self.pop_scope();

            if self.current_block_needs_terminator() {
                self.emit(Instruction::Jump {
                    target: after_clause,
                });
            }
        }

        if let (Some(finally_block), Some(finally_body)) = (finally_block, finally) {
            self.switch_to_block(finally_block);
            self.push_scope();
            for stmt in &finally_body.statements {
                self.build_stmt(stmt)?;
            }
            self.pop_scope();

            if self.current_block_needs_terminator() {
                self.emit(Instruction::Jump { target: exit_block });
            }
        }

        self.switch_to_block(exit_block);
        Ok(())
    }

    /// Build IR for a throw statement.
    pub(crate) fn build_throw(&mut self, expr: &Expr) -> Result<()> {
        let exception = self.build_expr(expr)?;
        self.emit(Instruction::Throw { exception });

        // Native codegen has no unwinding strategy: it lowers `TryBegin` to a
        // comment, leaves the catch block without a predecessor, and calls a
        // `@trq_throw` the runtime never defines — so this used to surface as an
        // undefined-symbol link error naming an internal symbol. Refuse here
        // instead, and only for `ارمِ`: a `حاول`/`التقط` that never throws
        // lowers correctly today and must keep compiling (issue #181).
        self.block_native_lowering(NativeBlock::unsupported(
            "رمي الاستثناءات بـ «ارمِ»",
            &ERR_NATIVE_EXCEPTIONS,
        ));

        Ok(())
    }

    /// Build IR for a block statement.
    pub(crate) fn build_block(&mut self, block: &Block) -> Result<()> {
        self.push_scope();
        for stmt in &block.statements {
            self.build_stmt(stmt)?;
        }
        self.pop_scope();
        Ok(())
    }
}
