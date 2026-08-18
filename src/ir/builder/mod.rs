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

use crate::error::codes::{ERR_ENTRY_POINT_CONFLICT, ERR_NO_ENTRY_POINT};
use crate::parser::{
    Ast, ClassMember, ExportItems, Expr, ImportItems, Param, Stmt, StmtKind, TypeAnnotation,
};

use super::{
    BasicBlock, BlockId, Class, ClassId, Constant, FuncId, Function, Instruction, IrType, MethodId,
    Module, NativeBlock, Parameter, VarId,
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
    /// Names the semantic layer resolved to a user declaration, or `None` when
    /// no semantic pass fed the builder (unit tests, single-file paths).
    ///
    /// Only these may outrank a built-in. `function_names` cannot be used for
    /// that decision: it is collected from the *linked* AST, which carries
    /// every merged module declaration under its bare name whether the program
    /// imported it or not.
    visible_names: Option<HashSet<String>>,
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
    /// Each class's own virtually-dispatchable member names, in declaration
    /// order: instance methods and property accessors, never `مشترك` members or
    /// `منشئ`. Declaration order, not map order, because it fixes the vtable
    /// slot numbering — deriving it from `method_return_types` would renumber
    /// slots per run and break the prefix invariant `Class.vtable` relies on.
    pub(crate) class_own_virtuals: HashMap<String, Vec<String>>,
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

/// The builder state that belongs to one function's build and must survive a
/// nested one.
///
/// A class method, a constructor, a property accessor, a nested `دالة` and a
/// lifted lambda are all built by opening a second function in the middle of
/// the enclosing function's build. Every field here is per-function — block
/// and variable ids restart, and the loop targets name blocks of the function
/// that pushed them — so the enclosing build must get its own values back.
/// Module-wide tables (class layouts, function signatures, `lambda_counter`,
/// …) are deliberately absent: those accumulate across functions on purpose.
pub(crate) struct FunctionContext {
    function: Option<Function>,
    block: BlockId,
    var_counter: u32,
    block_counter: u32,
    variables: HashMap<String, VarId>,
    parameters: HashSet<u32>,
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
            visible_names: None,
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
            class_own_virtuals: HashMap::new(),
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

        // `قص_حروف` was mapped to a runtime symbol with no entry here for as long
        // as it existed (**B7**), and the omission is quiet like `ثنائي_إلى_نص`'s
        // below rather than loud like `جذر`'s above — `Ptr(Void)` and `String`
        // are both `ptr`, so it linked and printed correctly.
        //
        // Measured natively with this line deleted: `نوع` → `مؤشر`, `"X" + …` →
        // `X4341079168`, `== "رح"` → `خطأ`, and `طول` → **6 where 3 was right**.
        // That last one is the reason to care: the sentinel routes `ArrayLen` to
        // `trq_array_len`, which reads `TrqArray.len` at offset 0, and a
        // `TrqString`'s field at offset 0 is its *byte* length. So dropping this
        // entry makes the codepoint slicer count bytes — the one thing it exists
        // not to do, and invisible on ASCII.
        self.function_return_types
            .insert("قص_حروف".to_string(), IrType::String);

        // حرف_إلى_رمز lowers to a plain call, so this is the only thing that
        // types its result. Unregistered it would take the same `Ptr(Void)`
        // sentinel as `جذر` above and emit `call ptr` against a `declare i64`.
        self.function_return_types
            .insert("حرف_إلى_رمز".to_string(), IrType::Int);
        self.function_return_types
            .insert("رمز_إلى_حرف".to_string(), IrType::String);
        // Unregistered this one would not mismatch its `declare` — `Ptr(Void)`
        // and `Array` both map to `ptr` — so it would link and run, then read
        // the `TrqArray` as a `TrqString` in `اطبع` and `load ptr` out of an
        // i64 slot when indexed. Silent, unlike `جذر`'s segfault above.
        self.function_return_types.insert(
            "نص_إلى_ثنائي".to_string(),
            IrType::Array(Box::new(IrType::Int), 0),
        );
        // Quiet unregistered for the same reason as the line above, not `جذر`'s
        // loud one: `Ptr(Void)` and `String` are both `ptr`, so the module still
        // links and then reads a `TrqString` as whatever the sentinel implies.
        self.function_return_types
            .insert("ثنائي_إلى_نص".to_string(), IrType::String);

        // اقرأ_سطر (read_line) returns string
        self.function_return_types
            .insert("اقرأ_سطر".to_string(), IrType::String);
        self.function_return_types
            .insert("read_line".to_string(), IrType::String);

        // Date/time and base64 builtins (#241). Registering these is not
        // optional: defining the runtime symbols without a return type would
        // trade the link error for exactly the `جذر` segfault described above.
        for name in [
            "وقت_الآن",
            "وقت_أداء",
            "يوم_الأسبوع",
            "يوم_السنة",
            "رقم_الأسبوع",
            "أيام_الشهر",
            "فرق_أيام",
        ] {
            self.function_return_types
                .insert(name.to_string(), IrType::Int);
        }
        for name in [
            "نسّق_تاريخ",
            "نسّق_وقت",
            "نسّق_تاريخ_ووقت",
            "ترميز_أساس64",
            "فك_أساس64",
        ] {
            self.function_return_types
                .insert(name.to_string(), IrType::String);
        }
    }

    /// Build IR from an AST for an executable program.
    ///
    /// This is the main entry point for converting a parsed AST to IR. The
    /// AST must supply an entry point — top-level executable statements
    /// (Script mode) or `دالة رئيسية()` (Program mode) — or the build fails
    /// with ت٠٢٠٢. Use [`IrBuilder::build_library`] to lower a declarations-only
    /// module that is never meant to run on its own.
    pub fn build(self, ast: &Ast) -> Result<Module> {
        self.build_with_entry_point_policy(ast, true)
    }

    /// Supply the names the semantic layer resolved to user declarations
    /// (`Analyzer::visible_names`), which is what may outrank a built-in.
    ///
    /// Callers that link modules must set this. Without it the builder falls
    /// back to the linked AST's declarations, which include module declarations
    /// the program never imported.
    pub fn with_visible_names(mut self, names: HashSet<String>) -> Self {
        self.visible_names = Some(names);
        self
    }

    /// Build IR from an AST that is not required to define an entry point.
    ///
    /// A declarations-only file is a valid library module: it produces no
    /// `__main__`, and demanding one would be wrong. Everything else matches
    /// [`IrBuilder::build`].
    pub fn build_library(self, ast: &Ast) -> Result<Module> {
        self.build_with_entry_point_policy(ast, false)
    }

    fn build_with_entry_point_policy(
        mut self,
        ast: &Ast,
        require_entry_point: bool,
    ) -> Result<Module> {
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

        // Needs every class collected first: a slot's provider may be declared
        // after the class that inherits it.
        self.build_class_vtables();

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

        // Neither mode means no `__main__` is produced at all. Reported here,
        // off the same two flags the entry-point decision itself uses, because
        // downstream the absence is not recoverable and not explainable: the
        // linker fails on an undefined `___main__` C symbol, and the
        // interpreter just exits 0 having run nothing. A merged program keeps
        // its main file's entry point, so this can only fire on a file that
        // declares but never runs anything.
        if require_entry_point && !has_user_main && !has_top_level_executable {
            return Err(IrError::new(format!(
                "[{}] لا يحتوي الملف على نقطة دخول: لا جمل تنفيذية عليا ولا دالة رئيسية(). \
                     الملف الذي يحتوي تعريفات فقط هو وحدة تُستورَد، لا برنامج يُترجَم وحده. \
                     / File has no entry point: neither top-level executable statements nor \
                     دالة رئيسية(). A declarations-only file is a module to import, not a \
                     program to compile on its own.",
                ERR_NO_ENTRY_POINT
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

        self.module.shadowing_names = self
            .function_names
            .iter()
            .filter(|name| self.shadows_builtin(name))
            .cloned()
            .collect();

        Ok(self.module)
    }

    /// Whether a declaration of `name` outranks a same-named built-in.
    ///
    /// The single place this is decided. Backends read the answer off
    /// `Module::shadowing_names` rather than recomputing it, because the three
    /// sets they could recompute it from — the linked AST, `Module::functions`,
    /// and the semantic scope — do not agree.
    pub(crate) fn shadows_builtin(&self, name: &str) -> bool {
        if !self.function_names.contains(name) {
            return false;
        }
        match &self.visible_names {
            Some(visible) => visible.contains(name),
            None => true,
        }
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
        let mut own_virtuals = Vec::new();

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
                    } else {
                        own_virtuals.push(method_name.clone());
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
                        if !*is_static {
                            own_virtuals.push(format!("__احصل_{}", prop_name));
                        }
                    }

                    // Register setter
                    let has_setter = accessors
                        .iter()
                        .any(|a| matches!(a, crate::parser::PropertyAccessor::Set { .. }));
                    if has_setter || accessors.is_empty() {
                        let setter_name = format!("{}::__عيّن_{}", name, prop_name);
                        self.property_setters.insert(prop_key, setter_name);
                        if !*is_static {
                            own_virtuals.push(format!("__عيّن_{}", prop_name));
                        }
                    }
                }
                _ => {}
            }
        }

        self.class_fields.insert(name.to_string(), fields.clone());
        self.class_own_virtuals
            .insert(name.to_string(), own_virtuals);

        let class_id = ClassId(name.to_string());
        let mut class = Class::new(class_id, name.to_string());
        class.fields = fields;

        self.module.classes.push(class);
        Ok(())
    }

    /// Fills every class's `vtable` with one `MethodId` per virtually
    /// dispatchable member, naming the class that *provides* the implementation
    /// for that class.
    ///
    /// Runs after the class-collection pass, so forward references resolve.
    pub(crate) fn build_class_vtables(&mut self) {
        let vtables: Vec<Vec<MethodId>> = self
            .module
            .classes
            .iter()
            .map(|class| {
                let class_name = class.id.0.clone();
                self.vtable_slots(&class_name)
                    .into_iter()
                    .filter_map(|member| {
                        self.slot_provider(&class_name, &member)
                            .map(|owner| MethodId {
                                class: ClassId(owner),
                                name: member,
                            })
                    })
                    .collect()
            })
            .collect();

        for (class, vtable) in self.module.classes.iter_mut().zip(vtables) {
            class.vtable = vtable;
        }
    }

    /// The ordered slot names for `class`: its ancestors' slots first, in the
    /// same order, then its own new members appended.
    ///
    /// A subclass's table is therefore always a *prefix extension* of its
    /// parent's, which is what lets codegen read a slot index off the receiver's
    /// static class and still land on the runtime class's implementation. An
    /// override reuses the inherited slot rather than appending a second one.
    fn vtable_slots(&self, class: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        let mut current = Some(class.to_string());
        while let Some(name) = current {
            if !visited.insert(name.clone()) {
                break; // semantic analysis rejects cyclic inheritance; don't hang here
            }
            current = self.class_parents.get(&name).cloned();
            chain.push(name);
        }

        let mut slots: Vec<String> = Vec::new();
        for ancestor in chain.iter().rev() {
            for member in self.class_own_virtuals.get(ancestor).into_iter().flatten() {
                if !slots.iter().any(|slot| slot == member) {
                    slots.push(member.clone());
                }
            }
        }
        slots
    }

    /// The nearest class at or above `class` that declares `member` — the body
    /// this class's slot must point at.
    fn slot_provider(&self, class: &str, member: &str) -> Option<String> {
        let mut visited = HashSet::new();
        let mut current = Some(class.to_string());
        while let Some(name) = current {
            if !visited.insert(name.clone()) {
                break;
            }
            if self
                .class_own_virtuals
                .get(&name)
                .is_some_and(|members| members.iter().any(|m| m == member))
            {
                return Some(name);
            }
            current = self.class_parents.get(&name).cloned();
        }
        None
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
    /// native code (see `Function::native_block`).
    ///
    /// First reason wins among equals, so the earliest diagnosis is reported —
    /// except that an unsupported construct replaces an untyped-parameter
    /// reason, whose advice ("declare concrete types") cannot resolve it.
    pub(crate) fn block_native_lowering(&mut self, block: NativeBlock) {
        if let Some(func) = self.current_function.as_mut() {
            let replaces = match &func.native_block {
                None => true,
                Some(existing) => block.overrides_types && !existing.overrides_types,
            };
            if replaces {
                func.native_block = Some(block);
            }
        }
    }

    /// Detach the enclosing function's build state so a nested declaration
    /// can be built from a clean slate, and hand it back for
    /// [`IrBuilder::resume_function_context`].
    ///
    /// `begin_function` resets every field captured here and `end_function`
    /// restores none of them, so any site that builds a declaration *while*
    /// another function is open has to round-trip them itself. Doing that by
    /// hand is what broke: `build_class_decl` saved only `current_function`
    /// and `variables`, so a class method with more than one basic block left
    /// `current_block` pointing at a block id that exists only in the method.
    /// `emit` silently drops instructions aimed at a block the current
    /// function does not own, so every statement after a class declaration in
    /// Script mode vanished from the synthesized `__main__` — no diagnostic,
    /// no output, exit 0.
    ///
    /// `parameters` is captured for the same reason, and was the same bug one
    /// layer down: `begin_function` clears `parameters` and
    /// fills it with the nested declaration's own parameter ids, so on return
    /// the enclosing function saw *those* ids as direct values. In Script mode
    /// a class declared before a C-style `لكل` left id 0 (the method's `هذا`)
    /// marked as a parameter, so the loop variable — also id 0 in the
    /// synthesized `__main__` — stopped emitting its `Load` and the condition
    /// compared the raw alloca pointer: `متوقع comparable، وُجد ptr`.
    ///
    /// `var_types` is deliberately *not* captured: the lambda lift reads the
    /// types its nested build recorded after resuming, to thread a declared
    /// function-type annotation into the lifted body. Restoring it here makes
    /// `test_lambda_assigned_to_annotated_slot_threads_hint` fail with ت٠٣٠١.
    pub(crate) fn suspend_function_context(&mut self) -> FunctionContext {
        FunctionContext {
            function: self.current_function.take(),
            block: self.current_block,
            var_counter: self.var_counter,
            block_counter: self.block_counter,
            variables: std::mem::take(&mut self.variables),
            parameters: std::mem::take(&mut self.parameters),
        }
    }

    /// Put the enclosing function's build state back after a nested
    /// declaration finished (its `end_function` must already have run).
    pub(crate) fn resume_function_context(&mut self, saved: FunctionContext) {
        self.current_function = saved.function;
        self.current_block = saved.block;
        self.var_counter = saved.var_counter;
        self.block_counter = saved.block_counter;
        self.variables = saved.variables;
        self.parameters = saved.parameters;
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

    /// Whether the current block still needs a terminator — false once it ends
    /// in `Return`, `Jump`, `Branch` or `Throw`.
    ///
    /// Emitting past a terminator produces IR that LLVM rejects outright, so
    /// every branch-joining `Jump` has to be guarded by this.
    pub(crate) fn current_block_needs_terminator(&self) -> bool {
        self.current_function
            .as_ref()
            .and_then(|func| func.get_block(self.current_block))
            .map(|block| !block.has_terminator())
            .unwrap_or(false)
    }

    /// Close a body whose last block fell off the end, emitting the implicit
    /// return the source omitted.
    ///
    /// Keyed on `current_block`, never on `blocks.last()`: a statement that
    /// mints its exit block before its body blocks — `تطابق` always, the
    /// loops and `إذا` whenever the body branches — leaves the merge block
    /// buried mid-vector, so `blocks.last()` names an already-terminated arm
    /// and the check waves the real block through unterminated (#234).
    pub(crate) fn emit_implicit_return(&mut self, ret_ty: &IrType) {
        if !self.current_block_needs_terminator() {
            return;
        }

        if *ret_ty == IrType::Void {
            self.emit(Instruction::Return { value: None });
            return;
        }

        // Defensive default: semantic analysis is expected to guarantee every
        // path returns whenever a non-void return type was declared, so this
        // should be unreachable for valid programs — it exists only to avoid
        // an ill-typed `ret void` inside a non-void function if it ever is.
        let dest = self.new_var();
        let zero = match ret_ty {
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

    /// Generate a new unique variable ID.
    pub(crate) fn new_var(&mut self) -> VarId {
        let id = VarId(self.var_counter);
        self.var_counter += 1;
        id
    }

    /// Emit an instruction to the current block.
    pub(crate) fn emit(&mut self, inst: Instruction) {
        // Emitting into a block the current function does not own can only
        // mean its context was not restored after a nested build, and the
        // instruction is about to be dropped on the floor. That silent drop is
        // exactly how a whole `__main__` body went missing with no diagnostic
        // and exit 0; fail loudly in debug builds instead. Emitting with *no*
        // current function stays a legitimate no-op — top-level declarations
        // in Program mode take that path.
        debug_assert!(
            self.current_function
                .as_ref()
                .is_none_or(|func| func.get_block(self.current_block).is_some()),
            "تعليمة موجّهة إلى كتلة غير موجودة في الدالة الحالية / instruction \
             emitted into a block absent from the current function"
        );
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

    /// Most tests below assert on the *shape* of the emitted IR for a
    /// declaration, not on entry-point policy, so they lower as library
    /// modules — `build` would reject a bare `متغير س = ٥` for having no
    /// entry point. `build_program` covers that policy separately.
    fn build_ir(source: &str) -> Result<Module> {
        let wrapped = wrap_with_markers(source);
        let mut parser = Parser::new(&wrapped);
        let ast = parser.parse().expect("Failed to parse");
        let builder = IrBuilder::new("test".to_string());
        builder.build_library(&ast)
    }

    fn build_program(source: &str) -> Result<Module> {
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
    fn test_function_declaration_shadows_import_alias() {
        // An alias is module-scope naming layered over the merged AST; a
        // function the importing file declares itself outranks it. Guarding
        // only on locals let the alias silently redirect to the imported
        // ضاعف, so this printed 10 instead of 105 with no diagnostic.
        let source = r#"
            استورد { ضاعف كـ اضعف } من "./مكتبة"
            دالة ضاعف(س: عدد) -> عدد {
                أرجع س * 2
            }
            دالة اضعف(س: عدد) -> عدد {
                أرجع س + 100
            }
            دالة رئيسية() {
                اطبع(اضعف(5))
            }
        "#;
        let module = build_ir(source).expect("shadowed alias must build");
        assert!(
            calls_function(&module, "__main__", "اضعف"),
            "the file's own دالة اضعف must win over the import alias"
        );
        assert!(
            !calls_function(&module, "__main__", "ضاعف"),
            "the import alias must not hijack a declared function of that name"
        );
    }

    #[test]
    fn test_global_declaration_shadows_import_alias() {
        // Same precedence for a global holding a function value: reading
        // `اضعف` must load the file's own global, not redirect to ضاعف.
        let source = r#"
            استورد { ضاعف كـ اضعف } من "./مكتبة"
            متغير ضاعف = 1
            متغير اضعف = 7
            دالة رئيسية() {
                اطبع(اضعف)
            }
        "#;
        let module = build_ir(source).expect("shadowed alias must build");
        assert!(
            module
                .functions
                .iter()
                .filter(|f| f.name == "__main__")
                .flat_map(|f| &f.blocks)
                .flat_map(|b| &b.instructions)
                .any(|inst| matches!(inst, Instruction::GlobalLoad { name, .. } if name == "اضعف")),
            "the file's own global اضعف must win over the import alias"
        );
    }

    #[test]
    fn test_increment_through_import_alias_resolves_to_original_name() {
        // `ع++` never went through the import-alias rewrite that `ع = ع + ١`
        // and `ع += ١` already did, so an aliased global failed IR building
        // with "لا يمكن تعديل متغير غير معرّف". The linker merges the module's
        // declaration under its bare name, modelled here by declaring عداد.
        let source = r#"
            استورد { عداد كـ ع } من "./مكتبة"
            متغير عداد = 0
            دالة رئيسية() {
                ع++
                اطبع(ع)
            }
        "#;
        let module = build_ir(source).expect("aliased increment must build");
        assert!(
            module
                .functions
                .iter()
                .filter(|f| f.name == "__main__")
                .flat_map(|f| &f.blocks)
                .flat_map(|b| &b.instructions)
                .any(
                    |inst| matches!(inst, Instruction::GlobalStore { name, .. } if name == "عداد")
                ),
            "the alias ع++ must write back to the merged declaration عداد"
        );
    }

    #[test]
    fn test_declarations_only_program_reports_no_entry_point() {
        // Compiled as a program, a declarations-only file used to reach the C
        // linker and fail there on an undefined `___main__` symbol.
        let source = r#"
            صدّر دالة مساعدة() -> عدد {
                أرجع 5
            }
            صدّر صنف أداة { }
        "#;
        let err = build_program(source).expect_err("a file with no entry point must be rejected");
        assert!(
            err.message.contains("ت٠٢٠٢"),
            "the diagnostic must carry the ت٠٢٠٢ code, got: {}",
            err.message
        );
    }

    #[test]
    fn test_script_and_program_modes_still_supply_an_entry_point() {
        build_program("اطبع(5)").expect("script mode supplies an entry point");
        build_program("دالة رئيسية() { اطبع(5) }").expect("program mode supplies an entry point");
    }

    /// A method body that branches, so the method ends in a block whose id
    /// exists only inside that method. `طالما` in the reported repro; `إذا`
    /// does it just as well — the trigger is the extra basic block, not the
    /// keyword.
    const CLASS_WITH_BRANCHING_METHOD: &str = r#"
        صنف عداد {
            عام دالة عد() {
                متغير س = 0
                طالما (س < 3) {
                    س = س + 1
                }
            }
        }
    "#;

    /// How many `اطبع` calls actually landed in `func`'s blocks.
    fn print_count(module: &Module, func: &str) -> usize {
        module
            .functions
            .iter()
            .filter(|f| f.name == func)
            .flat_map(|f| &f.blocks)
            .flat_map(|b| &b.instructions)
            .filter(|inst| matches!(inst, Instruction::Print { .. }))
            .count()
    }

    #[test]
    fn test_script_code_after_a_class_declaration_reaches_main() {
        // Script mode holds __main__ open across the whole statement pass, so
        // the class methods are built *inside* it. build_class_decl restored
        // only current_function, leaving current_block pointing into the
        // method — and `emit` drops instructions aimed at a block the current
        // function does not own. Every top-level statement after the class
        // silently vanished: __main__ lowered to `entry: unreachable`, the
        // program printed nothing and exited 0.
        let source = format!("{}\nاطبع(7)", CLASS_WITH_BRANCHING_METHOD);
        let module = build_program(&source).expect("script with a class must build");
        assert_eq!(
            print_count(&module, "__main__"),
            1,
            "top-level code after a class declaration must still land in __main__"
        );
    }

    #[test]
    fn test_main_is_terminated_when_a_class_declaration_follows_script_code() {
        // The mirror case: the statements land (they precede the class), but
        // the end-of-__main__ terminator check looks up current_block, finds
        // no such block, and skips the Return — leaving __main__ falling off
        // the end into codegen's `unreachable`.
        let source = format!("اطبع(7)\n{}", CLASS_WITH_BRANCHING_METHOD);
        let module = build_program(&source).expect("script with a class must build");
        let main = module
            .functions
            .iter()
            .find(|f| f.name == "__main__")
            .expect("script mode synthesizes __main__");
        assert!(
            main.blocks.iter().all(|b| b.has_terminator()),
            "__main__ must not be left falling off the end of a block"
        );
    }

    #[test]
    fn test_class_declaration_does_not_leak_block_ids_into_the_enclosing_build() {
        // The state that must round-trip, asserted directly: after building a
        // class whose method opens extra blocks, the enclosing function keeps
        // emitting into blocks it owns.
        let source = format!(
            "{}\nمتغير ن = 1\nاطبع(ن)\nاطبع(ن + 1)",
            CLASS_WITH_BRANCHING_METHOD
        );
        let module = build_program(&source).expect("script with a class must build");
        assert_eq!(
            print_count(&module, "__main__"),
            2,
            "both statements after the class must reach __main__"
        );
    }

    /// `تطابق` mints its exit block *before* the arm blocks, so the merge
    /// block a method ends on is never `blocks.last()`. Any body-closing check
    /// that reads `blocks.last()` therefore inspects a terminated arm, passes,
    /// and leaves the real block bare (#234).
    const CLASS_WITH_MATCH_METHOD: &str = r#"
        صنف مثال {
            منشئ() { }
            عام دالة افحص(ق: عدد) {
                تطابق (ق) {
                    حالة 1 => اطبع(1)
                    غير_ذلك => اطبع(0)
                }
            }
        }
    "#;

    fn function<'a>(module: &'a Module, name: &str) -> &'a Function {
        module
            .functions
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("module must contain {}", name))
    }

    #[test]
    fn test_method_ending_in_match_is_terminated() {
        // Unterminated, the merge block falls through to match.arm0 in block
        // order, whose join jump goes back to it: the interpreter loops forever
        // and native codegen lands on `unreachable`.
        let module = build_ir(CLASS_WITH_MATCH_METHOD).expect("class must build");
        let method = function(&module, "مثال::افحص");
        assert!(
            method.blocks.iter().all(|b| b.has_terminator()),
            "a method ending in تطابق must not fall off the end of its merge block"
        );
    }

    #[test]
    fn test_constructor_ending_in_match_is_terminated() {
        let source = r#"
            صنف مثال {
                منشئ(ق: عدد) {
                    تطابق (ق) {
                        حالة 1 => اطبع(1)
                        غير_ذلك => اطبع(0)
                    }
                }
            }
        "#;
        let module = build_ir(source).expect("class must build");
        let ctor = function(&module, "مثال::منشئ");
        assert!(
            ctor.blocks.iter().all(|b| b.has_terminator()),
            "a constructor ending in تطابق must not fall off the end of its merge block"
        );
    }

    #[test]
    fn test_non_void_method_merge_block_returns_a_value() {
        // Every arm returns, so the merge block is dead — but it still gets
        // emitted, and closing it with a bare `Return { value: None }` would be
        // `ret void` inside an i64 function, which LLVM rejects.
        let source = r#"
            صنف مثال {
                منشئ() { }
                عام دالة افحص(ق: عدد) -> عدد {
                    تطابق (ق) {
                        حالة 1 => أرجع 1
                        غير_ذلك => أرجع 0
                    }
                }
            }
        "#;
        let module = build_ir(source).expect("class must build");
        let method = function(&module, "مثال::افحص");
        assert!(
            method.blocks.iter().all(|b| b.has_terminator()),
            "a non-void method ending in تطابق must still be terminated"
        );
        assert!(
            method
                .blocks
                .iter()
                .filter_map(|b| b.terminator())
                .all(|t| matches!(
                    t,
                    Instruction::Return { value: Some(_) }
                        | Instruction::Jump { .. }
                        | Instruction::Branch { .. }
                )),
            "a non-void method must never return void: {:?}",
            method
                .blocks
                .iter()
                .filter_map(|b| b.terminator())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_lambda_body_ending_in_match_is_terminated() {
        let source = r#"
            دالة رئيسية() {
                ثابت ف = (ق: عدد) => {
                    تطابق (ق) {
                        حالة 1 => اطبع(1)
                        غير_ذلك => اطبع(0)
                    }
                }
                ف(1)
            }
        "#;
        let module = build_program(source).expect("program must build");
        let lambda = function(&module, "__lambda_0");
        assert!(
            lambda.blocks.iter().all(|b| b.has_terminator()),
            "a lambda body ending in تطابق must not fall off the end of its merge block"
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
