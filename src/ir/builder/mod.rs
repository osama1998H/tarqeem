//! IR Builder - Converts typed AST to IR
//!
//! This module provides the `IrBuilder` which walks the AST and generates
//! the intermediate representation (IR) for code generation.
//!
//! # Module Organization
//!
//! The builder is split into submodules for maintainability:
//! - `expr_builder`: Expression building (literals, binary ops, calls, etc.)
//! - `stmt_builder`: Statement building (var decl, func decl, control flow, etc.)
//! - `type_helpers`: Type conversion and inference utilities

mod expr_builder;
mod stmt_builder;
mod type_helpers;

use crate::error::codes::ERR_ENTRY_POINT_CONFLICT;
use crate::parser::{
    Ast, ClassMember, ExportItems, Expr, ImportItems, Param, Stmt, StmtKind, TypeAnnotation,
};

use super::{
    BasicBlock, BlockId, Class, ClassId, Constant, FuncId, Function, Instruction, IrType, Module,
    Parameter, VarId,
};

use std::collections::{HashMap, HashSet};

/// Error type for IR building errors.
#[derive(Debug, Clone)]
pub struct IrError {
    pub message: String,
}

impl IrError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for IrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for IrError {}

/// Result type for IR building operations.
pub(crate) type Result<T> = std::result::Result<T, IrError>;

/// The IR builder converts typed AST to IR.
///
/// The builder maintains state for the current function being built,
/// variable scopes, and type information.
pub struct IrBuilder {
    /// The module being built.
    pub(crate) module: Module,
    /// The function currently being built.
    pub(crate) current_function: Option<Function>,
    /// The current basic block for instruction emission.
    pub(crate) current_block: BlockId,
    /// Counter for generating unique variable IDs.
    pub(crate) var_counter: u32,
    /// Counter for generating unique block IDs.
    pub(crate) block_counter: u32,
    /// Current scope's variable bindings.
    pub(crate) variables: HashMap<String, VarId>,
    /// Type information for each variable ID.
    pub(crate) var_types: HashMap<u32, IrType>,
    /// Stack of variable scopes for nested blocks.
    pub(crate) scope_stack: Vec<HashMap<String, VarId>>,
    /// Stack of loop blocks for break/continue: (continue_block, break_block).
    pub(crate) loop_stack: Vec<(BlockId, BlockId)>,
    /// Field information for each class: class_name -> [(field_name, field_type)].
    pub(crate) class_fields: HashMap<String, Vec<(String, IrType)>>,
    /// Return types for methods: "class::method" -> return_type.
    pub(crate) method_return_types: HashMap<String, IrType>,
    /// Property getters: "class::property" -> (getter_name, property_type).
    pub(crate) property_getters: HashMap<String, (String, IrType)>,
    /// Property setters: "class::property" -> setter_name.
    pub(crate) property_setters: HashMap<String, String>,
    /// Set of all function names for lookup.
    pub(crate) function_names: HashSet<String>,
    /// Return types for functions: function_name -> return_type.
    pub(crate) function_return_types: HashMap<String, IrType>,
    /// Parameter types for functions (declared functions and lifted
    /// lambdas): function_name -> [param_type]. Lets a call site recover a
    /// callee's full `IrType::Function{params, ret}` when it's used as a
    /// value (see issue #180).
    pub(crate) function_param_types: HashMap<String, Vec<IrType>>,
    /// Module-scoped counter for naming lifted lambda functions
    /// (`__lambda_N`). Deliberately never saved/restored around a nested
    /// `build_lambda` call (unlike `var_counter`, which is per-function) —
    /// reusing a per-function counter let two lambdas in different
    /// enclosing functions collide on the same lifted name.
    pub(crate) lambda_counter: u32,
    /// Set of parameter variable IDs for the current function.
    pub(crate) parameters: HashSet<u32>,
    /// Global constants: name -> (constant_value, type).
    pub(crate) global_constants: HashMap<String, (Constant, IrType)>,
    /// Set of global variable names.
    pub(crate) global_variables: HashSet<String>,
    /// Types for global variables: name -> type.
    pub(crate) global_var_types: HashMap<String, IrType>,
    /// Enum variant field types: (enum_name, variant_name) -> [field_types].
    pub(crate) enum_variant_fields: HashMap<(String, String), Vec<IrType>>,
    /// Every class declared in this module (populated in pass 2, before any
    /// function body is built, so `ClassName.member` resolves regardless of
    /// declaration order).
    pub(crate) class_names: HashSet<String>,
    /// Inheritance edges: child class name -> parent class name.
    pub(crate) class_parents: HashMap<String, String>,
    /// `مشترك` field globals: "Class::field" -> type.
    pub(crate) static_field_types: HashMap<String, IrType>,
    /// `مشترك` methods: "Class::method".
    pub(crate) static_methods: HashSet<String>,
    /// `مشترك خاصية` properties: "Class::property" (keys into
    /// `property_getters`/`property_setters`).
    pub(crate) static_properties: HashSet<String>,
    /// Static field/property initializers that are not compile-time
    /// constants, keyed by their "Class::member" global name.
    pub(crate) pending_static_inits: Vec<(String, Expr)>,
    /// Wildcard-import namespaces (`استورد * كـ رياض`). These name no value:
    /// they only qualify the bare names the linker merged.
    pub(crate) namespace_aliases: HashSet<String>,
    /// Named-import aliases: alias -> the original name the declaration was
    /// merged under (`استورد { ضاعف كـ اضعف }` records `اضعف` -> `ضاعف`).
    pub(crate) import_aliases: HashMap<String, String>,
}

/// Unwraps `صدّر <declaration>` to the declaration it exports.
///
/// Every top-level scan in `build` classifies statements by `StmtKind`, so
/// without this an exported function/class/global stays invisible to those
/// scans even though `build_stmt` descends into it — mirrors
/// `hoist_func_decl`/`hoist_enum_decl` in the semantic analyzer.
fn as_top_level_decl(stmt: &Stmt) -> &Stmt {
    match &stmt.kind {
        StmtKind::Export(ExportItems::Declaration(inner)) => inner,
        _ => stmt,
    }
}

impl IrBuilder {
    /// Create a new IR builder for a module.
    pub fn new(module_name: String) -> Self {
        let mut builder = Self {
            module: Module::new(module_name),
            current_function: None,
            current_block: BlockId(0),
            var_counter: 0,
            block_counter: 0,
            variables: HashMap::new(),
            var_types: HashMap::new(),
            scope_stack: Vec::new(),
            loop_stack: Vec::new(),
            class_fields: HashMap::new(),
            method_return_types: HashMap::new(),
            property_getters: HashMap::new(),
            property_setters: HashMap::new(),
            function_names: HashSet::new(),
            function_return_types: HashMap::new(),
            function_param_types: HashMap::new(),
            lambda_counter: 0,
            parameters: HashSet::new(),
            global_constants: HashMap::new(),
            global_variables: HashSet::new(),
            global_var_types: HashMap::new(),
            enum_variant_fields: HashMap::new(),
            class_names: HashSet::new(),
            class_parents: HashMap::new(),
            static_field_types: HashMap::new(),
            static_methods: HashSet::new(),
            static_properties: HashSet::new(),
            pending_static_inits: Vec::new(),
            namespace_aliases: HashSet::new(),
            import_aliases: HashMap::new(),
        };
        builder.register_builtin_return_types();
        builder
    }

    /// Register return types for builtin functions.
    fn register_builtin_return_types(&mut self) {
        // نوع (type) function returns string
        self.function_return_types
            .insert("نوع".to_string(), IrType::String);

        // نص (string conversion) function returns string
        self.function_return_types
            .insert("نص".to_string(), IrType::String);

        // طول (length) function returns int
        self.function_return_types
            .insert("طول".to_string(), IrType::Int);

        // عدد (int conversion) function returns int
        self.function_return_types
            .insert("عدد".to_string(), IrType::Int);

        // عدد_عشري (float conversion) function returns float
        self.function_return_types
            .insert("عدد_عشري".to_string(), IrType::Float);

        // منطقي (bool conversion) function returns bool
        self.function_return_types
            .insert("منطقي".to_string(), IrType::Bool);

        // SHA-256 builtin functions
        self.function_return_types
            .insert("احسب_بصمة".to_string(), IrType::String);
        self.function_return_types
            .insert("بصمة_ملف".to_string(), IrType::String);
        self.function_return_types
            .insert("بصمة_ثنائي".to_string(), IrType::String);
        self.function_return_types
            .insert("طابق_بصمة".to_string(), IrType::Bool);

        // Hex encoding builtin functions
        self.function_return_types
            .insert("إلى_ست_عشري".to_string(), IrType::String);
        self.function_return_types
            .insert("من_ست_عشري".to_string(), IrType::String);
        self.function_return_types
            .insert("ثنائي_إلى_ست_عشري".to_string(), IrType::String);
        self.function_return_types.insert(
            "ست_عشري_إلى_ثنائي".to_string(),
            IrType::Array(Box::new(IrType::Int), 0),
        );

        // GZIP compression builtin functions
        self.function_return_types
            .insert("اضغط".to_string(), IrType::Array(Box::new(IrType::Int), 0));
        self.function_return_types
            .insert("فك_الضغط".to_string(), IrType::String);
        self.function_return_types.insert(
            "اضغط_ثنائي".to_string(),
            IrType::Array(Box::new(IrType::Int), 0),
        );
        self.function_return_types.insert(
            "فك_ضغط_ثنائي".to_string(),
            IrType::Array(Box::new(IrType::Int), 0),
        );
        self.function_return_types
            .insert("اضغط_ملف".to_string(), IrType::Bool);
        self.function_return_types
            .insert("فك_ضغط_ملف".to_string(), IrType::Bool);

        // اقرأ_ملف returns string
        self.function_return_types
            .insert("اقرأ_ملف".to_string(), IrType::String);

        // اكتب_ملف returns bool
        self.function_return_types
            .insert("اكتب_ملف".to_string(), IrType::Bool);

        // جذر (sqrt) returns float. Unregistered, its call result carried the
        // `Ptr(Void)` unknown sentinel, so `اطبع(جذر(١٦.٠))` natively emitted
        // `trq_print(ptr %x)` on a double and dereferenced it — a segfault in
        // every native binary using it, whether reached bare or through a
        // wildcard namespace. The sibling math builtins listed beside `جذر` in
        // codegen's `get_runtime_function_name` still lack return types and
        // remain affected.
        self.function_return_types
            .insert("جذر".to_string(), IrType::Float);

        // اقرأ_سطر (read_line) returns string
        self.function_return_types
            .insert("اقرأ_سطر".to_string(), IrType::String);
        self.function_return_types
            .insert("read_line".to_string(), IrType::String);
    }

    /// Build IR from an AST.
    ///
    /// This is the main entry point for converting a parsed AST to IR.
    pub fn build(mut self, ast: &Ast) -> Result<Module> {
        // Import-naming pre-pass, ahead of every name-collecting pass:
        // `src/semantic/linker.rs` merges each imported declaration into this
        // AST under its ORIGINAL bare name, and IR names are flat and
        // unmangled. A wildcard namespace (`* كـ رياض`) and a named-import
        // alias (`{ ضاعف كـ اضعف }`) are therefore compile-time naming only,
        // and must resolve back to that bare name at every lookup.
        for stmt in &ast.statements {
            if let StmtKind::Import { items, .. } = &stmt.kind {
                self.collect_import_names(items);
            }
        }

        // First pass: collect function signatures (needed for global variable type inference)
        for stmt in &ast.statements {
            if let StmtKind::FuncDecl {
                name,
                params,
                return_type,
                ..
            } = &as_top_level_decl(stmt).kind
            {
                self.collect_function_signature(name, params, return_type)?;
            }
        }

        // Second pass: collect class declarations
        for stmt in &ast.statements {
            if let StmtKind::ClassDecl {
                name,
                extends,
                members,
                ..
            } = &as_top_level_decl(stmt).kind
            {
                self.collect_class(name, extends.as_ref(), members)?;
            }
        }

        // Third pass: collect global variables (after functions, so we can infer types from calls)
        for stmt in &ast.statements {
            if let StmtKind::VarDecl {
                name,
                mutable,
                ty,
                init,
                ..
            } = &as_top_level_decl(stmt).kind
            {
                let ir_type = if let Some(t) = ty {
                    self.convert_type(t)
                } else if let Some(init_expr) = init {
                    if let Some(const_val) = self.try_evaluate_const(init_expr) {
                        self.const_to_type(&const_val)
                    } else {
                        self.infer_expr_type(init_expr)
                    }
                } else {
                    IrType::Int
                };

                // Implicit عدد → عدد_عشري conversion (spec §5.6): a float-typed
                // global with an integer constant initializer must store a float
                // constant, or codegen emits `global double 5` (invalid LLVM IR)
                let init_val =
                    init.as_ref()
                        .and_then(|e| self.try_evaluate_const(e))
                        .map(|c| match (&ir_type, c) {
                            (IrType::Float, Constant::Int(n)) => Constant::Float(n as f64),
                            (_, c) => c,
                        });

                self.module
                    .globals
                    .push((name.clone(), ir_type.clone(), init_val.clone()));

                self.global_variables.insert(name.clone());
                self.global_var_types.insert(name.clone(), ir_type.clone());

                if !mutable {
                    if let Some(const_val) = init_val {
                        self.global_constants
                            .insert(name.clone(), (const_val, ir_type));
                    }
                }
            }
        }

        // Check if user defined دالة رئيسية() (Program mode entry point)
        let has_user_main = ast.statements.iter().any(
            |stmt| matches!(&as_top_level_decl(stmt).kind, StmtKind::FuncDecl { name, .. } if name == "رئيسية"),
        );

        // Check if there's top-level EXECUTABLE code (Script mode entry point)
        // VarDecl is allowed (global variables), but other statements are executable code
        // Import statements are declarations, not executable code
        let has_top_level_executable = ast.statements.iter().any(|stmt| {
            !matches!(
                &as_top_level_decl(stmt).kind,
                StmtKind::FuncDecl { .. }
                    | StmtKind::ClassDecl { .. }
                    | StmtKind::InterfaceDecl { .. }
                    | StmtKind::EnumDecl { .. }
                    | StmtKind::VarDecl { .. }
                    | StmtKind::Import { .. }
                    // Named/wildcard/re-exports survive `as_top_level_decl`
                    // unwrapping; all are module metadata that emits no IR.
                    | StmtKind::Export(..)
            )
        });

        // ERROR: Cannot have both Script mode and Program mode in the same file
        if has_user_main && has_top_level_executable {
            return Err(IrError::new(format!(
                "[{}] لا يمكن وجود جمل تنفيذية عليا ودالة رئيسية() في نفس الملف. \
                     استخدم إما وضع السكربت (كود علوي) أو وضع البرنامج (دالة رئيسية).",
                ERR_ENTRY_POINT_CONFLICT
            )));
        }

        // Collect global variables that need runtime initialization (non-constant initializers)
        // This is needed for Program mode where VarDecl is processed outside function context
        let globals_needing_init: Vec<(String, Expr)> = ast
            .statements
            .iter()
            .filter_map(|stmt| {
                if let StmtKind::VarDecl { name, init, .. } = &as_top_level_decl(stmt).kind {
                    if self.global_variables.contains(name) {
                        if let Some(init_expr) = init {
                            // Only include if it's NOT a constant (arrays, objects, etc.)
                            if self.try_evaluate_const(init_expr).is_none() {
                                return Some((name.clone(), init_expr.clone()));
                            }
                        }
                    }
                }
                None
            })
            .collect();

        // In Program mode, create __global_init__ function for complex initializers
        // This ensures arrays, objects, etc. are properly initialized before main runs.
        // Static field/property initializers ride along here (must be merged in
        // *before* the emptiness check, or a module whose only non-const
        // initializer is a مشترك field would skip __global_init__ entirely).
        // Script mode drains them inline into the synthesized __main__ instead
        // (below); every other shape — Program mode, or a pure
        // declarations-only file with no top-level code and no دالة رئيسية —
        // has no __main__ to attach them to inline, so it needs
        // __global_init__ too, or a مشترك field with a non-const initializer
        // in a class-only file is silently left null forever.
        let is_script_mode = has_top_level_executable && !has_user_main;
        let mut globals_needing_init = globals_needing_init;
        if !is_script_mode {
            globals_needing_init.extend(std::mem::take(&mut self.pending_static_inits));
        }
        if !is_script_mode && !globals_needing_init.is_empty() {
            self.begin_function("__global_init__".to_string(), vec![], IrType::Void)?;

            for (name, init_expr) in &globals_needing_init {
                // Hint-aware shared path (not a bare build_expr): a global
                // lambda must pick up its declared function-type annotation
                // here, or it lifts with untyped params and native mode
                // rejects fully-annotated code with ت٠٣٠١.
                self.build_global_initializer(name, init_expr)?;
            }

            self.emit(Instruction::Return { value: None });
            self.end_function()?;
        }

        // Script mode: wrap top-level executable code in auto-generated __main__
        if has_top_level_executable && !has_user_main {
            self.begin_function("__main__".to_string(), vec![], IrType::Void)?;

            // Script mode has no __global_init__; run static initializers as
            // the first statements of the synthesized __main__ instead.
            for (key, init_expr) in std::mem::take(&mut self.pending_static_inits) {
                self.build_global_initializer(&key, &init_expr)?;
            }
        }

        // Fourth pass: build all statements
        for stmt in &ast.statements {
            self.build_stmt(stmt)?;
        }

        // Close the auto-generated __main__ if in Script mode
        if has_top_level_executable && !has_user_main {
            if let Some(ref func) = self.current_function {
                // Use current_block, not blocks.last(), because after control flow
                // statements (like match), current_block may be different from the
                // last block in the blocks vector
                if let Some(block) = func.get_block(self.current_block) {
                    if !block.has_terminator() {
                        self.emit(Instruction::Return { value: None });
                    }
                }
            }
            self.end_function()?;
        }

        // Program mode: rename دالة رئيسية() to __main__ to serve as entry point
        if has_user_main {
            for func in &mut self.module.functions {
                if func.name == "رئيسية" {
                    func.name = "__main__".to_string();
                    func.id = FuncId("__main__".to_string());
                    break;
                }
            }
        }

        Ok(self.module)
    }

    /// Record the compile-time-only names one `استورد` introduces.
    fn collect_import_names(&mut self, items: &ImportItems) {
        match items {
            ImportItems::Wildcard(namespace) => {
                self.namespace_aliases.insert(namespace.clone());
            }
            ImportItems::Named(names) => {
                for item in names {
                    match &item.alias {
                        // A self-alias (`{ ضاعف كـ ضاعف }`) would map a name
                        // onto itself; there is nothing to redirect.
                        Some(alias) if *alias != item.name => {
                            self.import_aliases.insert(alias.clone(), item.name.clone());
                        }
                        _ => {}
                    }
                }
            }
            // A default import names the module's single default export
            // directly — there is no second name to redirect it to.
            ImportItems::Default(_) => {}
        }
    }

    /// Collect class information for later use.
    pub(crate) fn collect_class(
        &mut self,
        name: &str,
        extends: Option<&String>,
        members: &[ClassMember],
    ) -> Result<()> {
        self.class_names.insert(name.to_string());
        if let Some(parent) = extends {
            self.class_parents.insert(name.to_string(), parent.clone());
        }

        let mut fields = Vec::new();

        for member in members {
            match member {
                ClassMember::Field {
                    name: field_name,
                    ty,
                    init,
                    is_static,
                    ..
                } => {
                    let ir_type = ty
                        .as_ref()
                        .map(|t| self.convert_type(t))
                        .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                    if *is_static {
                        self.register_static_global(name, field_name, ir_type, init.as_ref());
                    } else {
                        fields.push((field_name.clone(), ir_type));
                    }
                }
                ClassMember::Method {
                    name: method_name,
                    return_type,
                    is_static,
                    ..
                } => {
                    let ret_ty = return_type
                        .as_ref()
                        .map(|t| self.convert_type(t))
                        .unwrap_or(IrType::Void);
                    let full_name = format!("{}::{}", name, method_name);
                    if *is_static {
                        self.static_methods.insert(full_name.clone());
                    }
                    self.method_return_types.insert(full_name, ret_ty);
                }
                ClassMember::Property {
                    name: prop_name,
                    ty,
                    accessors,
                    default_value,
                    is_static,
                    ..
                } => {
                    let prop_type = self.convert_type(ty);
                    let prop_key = format!("{}::{}", name, prop_name);
                    if *is_static {
                        self.static_properties.insert(prop_key.clone());
                    }

                    // For automatic properties (no custom accessors), add a backing field
                    if accessors.is_empty() {
                        let backing_field = format!("_{}", prop_name);
                        if *is_static {
                            self.register_static_global(
                                name,
                                &backing_field,
                                prop_type.clone(),
                                default_value.as_ref(),
                            );
                        } else {
                            fields.push((backing_field, prop_type.clone()));
                        }
                    }

                    // Register getter
                    let has_getter = accessors
                        .iter()
                        .any(|a| matches!(a, crate::parser::PropertyAccessor::Get { .. }));
                    if has_getter || accessors.is_empty() {
                        let getter_name = format!("{}::__احصل_{}", name, prop_name);
                        self.property_getters
                            .insert(prop_key.clone(), (getter_name, prop_type.clone()));
                    }

                    // Register setter
                    let has_setter = accessors
                        .iter()
                        .any(|a| matches!(a, crate::parser::PropertyAccessor::Set { .. }));
                    if has_setter || accessors.is_empty() {
                        let setter_name = format!("{}::__عيّن_{}", name, prop_name);
                        self.property_setters.insert(prop_key, setter_name);
                    }
                }
                _ => {}
            }
        }

        self.class_fields.insert(name.to_string(), fields.clone());

        let class_id = ClassId(name.to_string());
        let mut class = Class::new(class_id, name.to_string());
        class.fields = fields;

        self.module.classes.push(class);
        Ok(())
    }

    /// Lowers a `مشترك` field/property backing field to a module-level
    /// global keyed on the *defining* class (`"{Class}::{member}"`), reusing
    /// the existing `GlobalLoad`/`GlobalStore` machinery instead of the
    /// per-instance struct layout. Mirrors the constant-folding the ordinary
    /// top-level `متغير`/`ثابت` pass applies (see pass 3, below).
    fn register_static_global(
        &mut self,
        class: &str,
        member: &str,
        ty: IrType,
        init: Option<&Expr>,
    ) {
        let key = format!("{}::{}", class, member);
        let const_init = init.and_then(|e| self.try_evaluate_const(e)).map(|c| {
            // Implicit عدد -> عدد_عشري conversion (spec §5.6), mirroring pass 3.
            match (&ty, c) {
                (IrType::Float, Constant::Int(n)) => Constant::Float(n as f64),
                (_, c) => c,
            }
        });
        if const_init.is_none() {
            if let Some(init_expr) = init {
                self.pending_static_inits
                    .push((key.clone(), init_expr.clone()));
            }
        }
        self.module
            .globals
            .push((key.clone(), ty.clone(), const_init));
        self.static_field_types.insert(key, ty);
    }

    /// Collect function signature for forward declaration.
    pub(crate) fn collect_function_signature(
        &mut self,
        name: &str,
        params: &[Param],
        return_type: &Option<TypeAnnotation>,
    ) -> Result<()> {
        self.function_names.insert(name.to_string());

        let param_tys: Vec<IrType> = params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(|t| self.convert_type(t))
                    .unwrap_or(IrType::Ptr(Box::new(IrType::Void)))
            })
            .collect();
        self.function_param_types
            .insert(name.to_string(), param_tys);

        let ret_ty = return_type
            .as_ref()
            .map(|t| self.convert_type(t))
            .unwrap_or(IrType::Void);
        self.function_return_types.insert(name.to_string(), ret_ty);

        Ok(())
    }

    /// Records why the function currently being built cannot be lowered to
    /// native code (see `Function::native_block_reason`). First reason wins,
    /// so the earliest/most specific diagnosis is the one reported.
    pub(crate) fn block_native_lowering(&mut self, reason: String) {
        if let Some(func) = self.current_function.as_mut() {
            if func.native_block_reason.is_none() {
                func.native_block_reason = Some(reason);
            }
        }
    }

    /// Begin building a new function.
    pub(crate) fn begin_function(
        &mut self,
        name: String,
        params: Vec<Parameter>,
        return_type: IrType,
    ) -> Result<()> {
        let func_id = FuncId(name.clone());
        let mut func = Function::new(func_id, name, params.clone(), return_type);

        self.var_counter = params.len() as u32;
        self.block_counter = 0;

        let entry_block = BasicBlock::with_label(BlockId(0), "entry".to_string());
        func.blocks.push(entry_block);
        self.current_block = BlockId(0);
        self.block_counter = 1;

        self.push_scope();
        self.variables.clear();
        self.parameters.clear();
        for param in &params {
            self.variables.insert(param.name.clone(), param.id);
            self.parameters.insert(param.id.0);
            self.var_types.insert(param.id.0, param.ty.clone());
        }

        self.current_function = Some(func);
        Ok(())
    }

    /// Finish building the current function and add it to the module.
    pub(crate) fn end_function(&mut self) -> Result<()> {
        if let Some(func) = self.current_function.take() {
            self.module.functions.push(func);
        }
        self.pop_scope();
        Ok(())
    }

    /// Create a new basic block with an optional label.
    pub(crate) fn new_block(&mut self, label: Option<String>) -> BlockId {
        let id = BlockId(self.block_counter);
        self.block_counter += 1;

        let block = if let Some(l) = label {
            BasicBlock::with_label(id, l)
        } else {
            BasicBlock::new(id)
        };

        if let Some(ref mut func) = self.current_function {
            func.blocks.push(block);
        }

        id
    }

    /// Switch the current block for instruction emission.
    pub(crate) fn switch_to_block(&mut self, block_id: BlockId) {
        self.current_block = block_id;
    }

    /// Generate a new unique variable ID.
    pub(crate) fn new_var(&mut self) -> VarId {
        let id = VarId(self.var_counter);
        self.var_counter += 1;
        id
    }

    /// Emit an instruction to the current block.
    pub(crate) fn emit(&mut self, inst: Instruction) {
        if let Some(ref mut func) = self.current_function {
            if let Some(block) = func.get_block_mut(self.current_block) {
                block.instructions.push(inst);
            }
        }
    }

    /// Push a new variable scope.
    pub(crate) fn push_scope(&mut self) {
        self.scope_stack.push(self.variables.clone());
    }

    /// Pop the current variable scope.
    pub(crate) fn pop_scope(&mut self) {
        if let Some(vars) = self.scope_stack.pop() {
            self.variables = vars;
        }
    }

    /// Look up a variable by name in the current scope.
    pub(crate) fn lookup_var(&self, name: &str) -> Option<VarId> {
        self.variables.get(name).copied()
    }

    /// Add a string to the module's string table and return its index.
    pub(crate) fn add_string(&mut self, s: String) -> u32 {
        self.module.strings.add(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn wrap_with_markers(source: &str) -> String {
        format!("بسم_الله\n{}\nالحمد_لله", source.trim())
    }

    fn build_ir(source: &str) -> Result<Module> {
        let wrapped = wrap_with_markers(source);
        let mut parser = Parser::new(&wrapped);
        let ast = parser.parse().expect("Failed to parse");
        let builder = IrBuilder::new("test".to_string());
        builder.build(&ast)
    }

    #[test]
    fn test_simple_var_decl() {
        let source = "متغير س = 5";
        let module = build_ir(source).expect("Failed to build IR");
        assert_eq!(module.globals.len(), 1);
        let (name, ty, init) = &module.globals[0];
        assert_eq!(name, "س");
        assert!(matches!(ty, IrType::Int));
        assert!(matches!(init, Some(Constant::Int(5))));
    }

    #[test]
    fn test_function_decl() {
        let source = r#"
            دالة جمع(أ: عدد، ب: عدد) -> عدد {
                أرجع أ + ب
            }
        "#;
        let module = build_ir(source).expect("Failed to build IR");
        assert!(module.functions.iter().any(|f| f.name == "جمع"));
    }

    #[test]
    fn test_if_statement() {
        let source = r#"
            متغير س = 5;
            إذا (س > 0) {
                اطبع("موجب");
            }
        "#;
        let module = build_ir(source).expect("Failed to build IR");
        let main = &module.functions[0];
        assert!(main.blocks.len() >= 3);
    }

    #[test]
    fn test_while_loop() {
        let source = r#"
            متغير ع = 0;
            طالما (ع < 5) {
                ع = ع + 1;
            }
        "#;
        let module = build_ir(source).expect("Failed to build IR");
        let main = &module.functions[0];
        assert!(main.blocks.len() >= 4);
    }

    #[test]
    fn test_class_decl() {
        let source = r#"
            صنف شخص {
                خاص اسم: نص;

                منشئ(اسم: نص) {
                    هذا.اسم = اسم;
                }
            }
        "#;
        let module = build_ir(source).expect("Failed to build IR");
        assert!(module.classes.iter().any(|c| c.name == "شخص"));
    }

    #[test]
    fn test_module_display() {
        let source = "متغير س = 42";
        let module = build_ir(source).expect("Failed to build IR");
        let output = format!("{}", module);
        assert!(output.contains("Module: test"));
    }

    #[test]
    fn test_global_constant() {
        let source = "ثابت باي = 3";
        let module = build_ir(source).expect("Failed to build IR");
        assert_eq!(module.globals.len(), 1);
        let (name, ty, init) = &module.globals[0];
        assert_eq!(name, "باي");
        assert!(matches!(ty, IrType::Int));
        assert!(matches!(init, Some(Constant::Int(3))));
    }

    #[test]
    fn test_global_mutable_variable() {
        let source = "متغير عداد = 0";
        let module = build_ir(source).expect("Failed to build IR");
        assert_eq!(module.globals.len(), 1);
        let (name, ty, init) = &module.globals[0];
        assert_eq!(name, "عداد");
        assert!(matches!(ty, IrType::Int));
        assert!(matches!(init, Some(Constant::Int(0))));
    }

    #[test]
    fn test_multiple_globals() {
        let source = r#"
            متغير س = 10
            ثابت ص = 20
            متغير ع = 30
        "#;
        let module = build_ir(source).expect("Failed to build IR");
        assert_eq!(module.globals.len(), 3);

        let names: Vec<&String> = module.globals.iter().map(|(n, _, _)| n).collect();
        assert!(names.contains(&&"س".to_string()));
        assert!(names.contains(&&"ص".to_string()));
        assert!(names.contains(&&"ع".to_string()));
    }

    #[test]
    fn test_global_access_in_function() {
        let source = r#"
            متغير عداد = 0

            دالة زد() {
                عداد = عداد + 1
            }
        "#;
        let module = build_ir(source).expect("Failed to build IR");

        assert_eq!(module.globals.len(), 1);

        let increment_fn = module.functions.iter().find(|f| f.name == "زد");
        assert!(increment_fn.is_some());

        let func = increment_fn.unwrap();
        let has_global_load = func.blocks.iter().any(|block| {
            block
                .instructions
                .iter()
                .any(|inst| matches!(inst, Instruction::GlobalLoad { name, .. } if name == "عداد"))
        });
        let has_global_store = func.blocks.iter().any(|block| {
            block
                .instructions
                .iter()
                .any(|inst| matches!(inst, Instruction::GlobalStore { name, .. } if name == "عداد"))
        });
        assert!(has_global_load, "Should have GlobalLoad for عداد");
        assert!(has_global_store, "Should have GlobalStore for عداد");
    }

    #[test]
    fn test_global_with_arabic_name() {
        let source = "متغير العداد = 100";
        let module = build_ir(source).expect("Failed to build IR");
        assert_eq!(module.globals.len(), 1);
        let (name, _, _) = &module.globals[0];
        assert_eq!(name, "العداد");
    }

    #[test]
    fn test_global_boolean() {
        let source = "متغير علامة = صحيح";
        let module = build_ir(source).expect("Failed to build IR");
        assert_eq!(module.globals.len(), 1);
        let (_, ty, init) = &module.globals[0];
        assert!(matches!(ty, IrType::Bool));
        assert!(matches!(init, Some(Constant::Bool(true))));
    }

    #[test]
    fn test_local_variable_in_function() {
        let source = r#"
            دالة اختبار() {
                متغير محلي = 5
                أرجع محلي
            }
        "#;
        let module = build_ir(source).expect("Failed to build IR");

        assert_eq!(module.globals.len(), 0);

        let test_fn = module.functions.iter().find(|f| f.name == "اختبار");
        assert!(test_fn.is_some());

        let func = test_fn.unwrap();
        let has_alloca = func.blocks.iter().any(|block| {
            block
                .instructions
                .iter()
                .any(|inst| matches!(inst, Instruction::Alloca { .. }))
        });
        assert!(has_alloca, "Local variable should use Alloca");
    }

    #[test]
    fn test_program_mode_main_function_renamed() {
        // Program mode: دالة رئيسية() is renamed to __main__ for linking
        let source = r#"
            دالة رئيسية() {
                اطبع("مرحبا")
            }
        "#;
        let module = build_ir(source).expect("Failed to build IR");

        // Should have __main__ function, not رئيسية
        assert!(
            module.functions.iter().any(|f| f.name == "__main__"),
            "Function رئيسية should be renamed to __main__"
        );
        assert!(
            !module.functions.iter().any(|f| f.name == "رئيسية"),
            "Function رئيسية should not exist after renaming"
        );
    }

    #[test]
    fn test_program_mode_with_globals() {
        // Program mode: globals + دالة رئيسية() should work
        let source = r#"
            متغير س = 5

            دالة رئيسية() {
                اطبع("مرحبا")
            }
        "#;
        let module = build_ir(source).expect("Failed to build IR");

        // Should have global and __main__
        assert_eq!(module.globals.len(), 1, "Should have one global variable");
        assert!(
            module.functions.iter().any(|f| f.name == "__main__"),
            "Function رئيسية should be renamed to __main__"
        );
        // Should NOT have duplicate __main__ functions
        let main_count = module
            .functions
            .iter()
            .filter(|f| f.name == "__main__")
            .count();
        assert_eq!(main_count, 1, "Should have exactly one __main__ function");
    }

    #[test]
    fn test_script_mode_top_level_code() {
        // Script mode: top-level executable code creates auto __main__
        let source = r#"
            اطبع("مرحبا")
        "#;
        let module = build_ir(source).expect("Failed to build IR");

        assert!(
            module.functions.iter().any(|f| f.name == "__main__"),
            "Script mode should create __main__ for top-level code"
        );
    }

    #[test]
    fn test_program_mode_global_array_init() {
        // Program mode: global variables with complex initializers (arrays)
        // should create __global_init__ function
        let source = r#"
            متغير قائمة = [1، 2، 3]

            دالة رئيسية() {
                اطبع(قائمة)
            }
        "#;
        let module = build_ir(source).expect("Failed to build IR");

        // Should have __global_init__ function for array initialization
        assert!(
            module.functions.iter().any(|f| f.name == "__global_init__"),
            "Program mode with array global should create __global_init__ function"
        );

        // __global_init__ should have GlobalStore instruction
        let init_func = module
            .functions
            .iter()
            .find(|f| f.name == "__global_init__")
            .expect("__global_init__ function should exist");

        let has_global_store = init_func.blocks.iter().any(|block| {
            block.instructions.iter().any(
                |inst| matches!(inst, Instruction::GlobalStore { name, .. } if name == "قائمة"),
            )
        });
        assert!(
            has_global_store,
            "__global_init__ should have GlobalStore for قائمة"
        );
    }

    #[test]
    fn test_program_mode_constant_global_no_init_func() {
        // Program mode: global variables with constant initializers (int, string, etc.)
        // should NOT create __global_init__ function
        let source = r#"
            متغير س = 5
            ثابت اسم = "احمد"

            دالة رئيسية() {
                اطبع(س)
            }
        "#;
        let module = build_ir(source).expect("Failed to build IR");

        // Should NOT have __global_init__ function (constants don't need it)
        assert!(
            !module.functions.iter().any(|f| f.name == "__global_init__"),
            "Program mode with only constant globals should NOT create __global_init__"
        );
    }

    #[test]
    fn test_script_and_program_mode_conflict_error() {
        // Conflict: both top-level executable code AND دالة رئيسية() should error
        let source = r#"
            اطبع("top level")

            دالة رئيسية() {
                اطبع("in main")
            }
        "#;
        let result = build_ir(source);

        assert!(
            result.is_err(),
            "Should error when both Script mode and Program mode are used"
        );
        let err = result.unwrap_err();
        assert!(
            err.message.contains("لا يمكن وجود"),
            "Error should mention the conflict"
        );
        // Verify error code is included
        assert!(
            err.message.contains("ت٠٢٠١"),
            "Error should include error code ت٠٢٠١"
        );
        assert!(
            err.message.contains("ت٠٢٠١"),
            "Arabic error should include error code ت٠٢٠١"
        );
    }

    #[test]
    fn test_global_float_annotation_int_initializer_coerced_to_float_constant() {
        // Implicit عدد → عدد_عشري (spec §5.6): keeping Constant::Int(5) on an
        // f64 global made codegen emit `global double 5`, invalid LLVM IR
        let source = "متغير أ: عدد_عشري = 5";
        let module = build_ir(source).expect("Failed to build IR");
        let (name, ty, init) = &module.globals[0];
        assert_eq!(name, "أ");
        assert!(matches!(ty, IrType::Float));
        assert!(
            matches!(init, Some(Constant::Float(f)) if *f == 5.0),
            "int initializer of a float global must be coerced, got {:?}",
            init
        );
    }

    #[test]
    fn test_local_float_annotation_int_initializer_emits_int_to_float() {
        // Without the coercion the i64 bit pattern was stored raw into an
        // alloca double, printing ~2.47e-323 in native binaries
        let source = r#"
            دالة رئيسية() {
                متغير أ: عدد_عشري = 5
                اطبع(أ)
            }
        "#;
        let module = build_ir(source).expect("Failed to build IR");
        let has_int_to_float = module
            .functions
            .iter()
            .flat_map(|f| &f.blocks)
            .flat_map(|b| &b.instructions)
            .any(|i| matches!(i, Instruction::IntToFloat { .. }));
        assert!(
            has_int_to_float,
            "float-typed local with int initializer must emit IntToFloat"
        );
    }

    #[test]
    fn test_exported_function_signature_reaches_return_types() {
        // `function_return_types` isn't observable after `build` consumes the
        // builder, so assert through the global whose type it drives. Program
        // mode is required: `__global_init__` is emitted before any function
        // body exists, so the global's type can only come from the first-pass
        // signature scan. (Script mode re-infers it once the body is built,
        // which masks a missed signature.)
        let source = r#"
            صدّر دالة تحية() -> نص {
                أرجع "مرحبا"
            }
            متغير رسالة = تحية()
            دالة رئيسية() {
                اطبع(رسالة)
            }
        "#;
        let module = build_ir(source).expect("Failed to build IR");
        let (name, ty, _) = &module.globals[0];
        assert_eq!(name, "رسالة");
        assert!(
            matches!(ty, IrType::String),
            "exported function's return type must reach function_return_types, got {:?}",
            ty
        );
    }

    #[test]
    fn test_exported_declarations_are_not_top_level_executable() {
        let source = r#"
            صدّر دالة مساعدة() -> عدد {
                أرجع 5
            }
            صدّر صنف أداة {}
            صدّر ثابت الإصدار = 1
            دالة رئيسية() {
                اطبع(مساعدة())
            }
        "#;
        assert!(
            build_ir(source).is_ok(),
            "a file of صدّر declarations plus دالة رئيسية must not trigger ت٠٢٠١"
        );
    }

    #[test]
    fn test_named_export_is_not_top_level_executable() {
        // Named/wildcard exports never unwrap to a declaration, so they need
        // their own exclusion from the executable-code scan.
        let source = r#"
            دالة مساعدة() -> عدد {
                أرجع 5
            }
            صدّر { مساعدة }
            دالة رئيسية() {
                اطبع(مساعدة())
            }
        "#;
        assert!(
            build_ir(source).is_ok(),
            "a named export must not count as top-level executable code"
        );
    }

    /// Does `func` call `name` directly (the shape the linker's flat, unmangled
    /// names require of every resolved import reference)?
    fn calls_function(module: &Module, func: &str, name: &str) -> bool {
        module
            .functions
            .iter()
            .filter(|f| f.name == func)
            .flat_map(|f| &f.blocks)
            .flat_map(|b| &b.instructions)
            .any(|inst| matches!(inst, Instruction::Call { func, .. } if func.0 == name))
    }

    #[test]
    fn test_wildcard_namespace_call_lowers_to_bare_function() {
        // Models the post-link AST: the module's declaration is merged in
        // under its bare name while main's `استورد` survives.
        let source = r#"
            استورد * كـ أدوات من "./مكتبة"
            دالة جمع(أ: عدد، ب: عدد) -> عدد {
                أرجع أ + ب
            }
            دالة رئيسية() {
                اطبع(أدوات.جمع(2، 3))
            }
        "#;
        let module = build_ir(source).expect("namespaced call must build");
        assert!(
            calls_function(&module, "__main__", "جمع"),
            "أدوات.جمع must lower to a direct call to جمع"
        );
    }

    #[test]
    fn test_named_import_alias_resolves_to_original_name() {
        let source = r#"
            استورد { ضاعف كـ اضعف } من "./مكتبة"
            دالة ضاعف(س: عدد) -> عدد {
                أرجع س * 2
            }
            دالة رئيسية() {
                اطبع(اضعف(5))
            }
        "#;
        let module = build_ir(source).expect("aliased import must build");
        assert!(
            calls_function(&module, "__main__", "ضاعف"),
            "the alias اضعف must call the merged declaration ضاعف"
        );
    }

    #[test]
    fn test_local_binding_shadows_import_alias() {
        // An alias is module-scope naming; a local of the same name wins, as
        // it does over any other module-level binding.
        let source = r#"
            استورد { ضاعف كـ اضعف } من "./مكتبة"
            دالة ضاعف(س: عدد) -> عدد {
                أرجع س * 2
            }
            دالة رئيسية() {
                متغير اضعف = 7
                اطبع(اضعف)
            }
        "#;
        let module = build_ir(source).expect("shadowed alias must build");
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "__main__")
            .expect("__main__ must exist");
        assert!(
            main.blocks
                .iter()
                .flat_map(|b| &b.instructions)
                .any(|inst| matches!(inst, Instruction::Alloca { .. })),
            "the local اضعف must still be allocated"
        );
        assert!(
            !calls_function(&module, "__main__", "ضاعف"),
            "a local must shadow the import alias, not resolve through it"
        );
    }

    #[test]
    fn test_pure_export_declarations_generate_no_main() {
        let source = r#"
            صدّر دالة مساعدة() -> عدد {
                أرجع 5
            }
        "#;
        let module = build_ir(source).expect("Failed to build IR");
        assert!(
            !module.functions.iter().any(|f| f.name == "__main__"),
            "a declarations-only module must not synthesize a script-mode __main__"
        );
    }
}
