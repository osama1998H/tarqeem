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
use crate::parser::{Ast, ClassMember, Param, StmtKind, TypeAnnotation};

use super::{
    BasicBlock, BlockId, Class, ClassId, Constant, FuncId, Function, Instruction, IrType, Module,
    Parameter, VarId,
};

use std::collections::{HashMap, HashSet};

/// Error type for IR building errors.
#[derive(Debug, Clone)]
pub struct IrError {
    pub message: String,
    pub message_ar: String,
}

impl IrError {
    pub fn new(message: impl Into<String>, message_ar: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            message_ar: message_ar.into(),
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
}

impl IrBuilder {
    /// Create a new IR builder for a module.
    pub fn new(module_name: String) -> Self {
        Self {
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
            parameters: HashSet::new(),
            global_constants: HashMap::new(),
            global_variables: HashSet::new(),
            global_var_types: HashMap::new(),
            enum_variant_fields: HashMap::new(),
        }
    }

    /// Build IR from an AST.
    ///
    /// This is the main entry point for converting a parsed AST to IR.
    pub fn build(mut self, ast: &Ast) -> Result<Module> {
        // First pass: collect function signatures (needed for global variable type inference)
        for stmt in &ast.statements {
            if let StmtKind::FuncDecl {
                name,
                params,
                return_type,
                ..
            } = &stmt.kind
            {
                self.collect_function_signature(name, params, return_type)?;
            }
        }

        // Second pass: collect class declarations
        for stmt in &ast.statements {
            if let StmtKind::ClassDecl { name, members, .. } = &stmt.kind {
                self.collect_class(name, members)?;
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
            } = &stmt.kind
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

                let init_val = init.as_ref().and_then(|e| self.try_evaluate_const(e));

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
        let has_user_main = ast
            .statements
            .iter()
            .any(|stmt| matches!(&stmt.kind, StmtKind::FuncDecl { name, .. } if name == "رئيسية"));

        // Check if there's top-level EXECUTABLE code (Script mode entry point)
        // VarDecl is allowed (global variables), but other statements are executable code
        let has_top_level_executable = ast.statements.iter().any(|stmt| {
            !matches!(
                &stmt.kind,
                StmtKind::FuncDecl { .. }
                    | StmtKind::ClassDecl { .. }
                    | StmtKind::InterfaceDecl { .. }
                    | StmtKind::VarDecl { .. }
            )
        });

        // ERROR: Cannot have both Script mode and Program mode in the same file
        if has_user_main && has_top_level_executable {
            return Err(IrError::new(
                format!(
                    "[{}] Cannot have both top-level executable statements and دالة رئيسية() in the same file. \
                     Use either Script mode (top-level code) or Program mode (دالة رئيسية).",
                    ERR_ENTRY_POINT_CONFLICT
                ),
                format!(
                    "[{}] لا يمكن وجود جمل تنفيذية عليا ودالة رئيسية() في نفس الملف. \
                     استخدم إما وضع السكربت (كود علوي) أو وضع البرنامج (دالة رئيسية).",
                    ERR_ENTRY_POINT_CONFLICT
                ),
            ));
        }

        // Script mode: wrap top-level executable code in auto-generated __main__
        if has_top_level_executable && !has_user_main {
            self.begin_function("__main__".to_string(), vec![], IrType::Void)?;
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

    /// Collect class information for later use.
    pub(crate) fn collect_class(&mut self, name: &str, members: &[ClassMember]) -> Result<()> {
        let mut fields = Vec::new();

        for member in members {
            match member {
                ClassMember::Field {
                    name: field_name,
                    ty,
                    ..
                } => {
                    let ir_type = ty
                        .as_ref()
                        .map(|t| self.convert_type(t))
                        .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                    fields.push((field_name.clone(), ir_type));
                }
                ClassMember::Method {
                    name: method_name,
                    return_type,
                    ..
                } => {
                    let ret_ty = return_type
                        .as_ref()
                        .map(|t| self.convert_type(t))
                        .unwrap_or(IrType::Void);
                    let full_name = format!("{}::{}", name, method_name);
                    self.method_return_types.insert(full_name, ret_ty);
                }
                ClassMember::Property {
                    name: prop_name,
                    ty,
                    accessors,
                    default_value,
                    ..
                } => {
                    let prop_type = self.convert_type(ty);
                    let prop_key = format!("{}::{}", name, prop_name);

                    // For automatic properties (no custom accessors), add a backing field
                    if accessors.is_empty() {
                        let backing_field = format!("_{}", prop_name);
                        fields.push((backing_field, prop_type.clone()));
                    }

                    // Also add the property itself as a field for properties with default values
                    if default_value.is_some() && accessors.is_empty() {
                        // Already added as backing field above
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

    /// Collect function signature for forward declaration.
    pub(crate) fn collect_function_signature(
        &mut self,
        name: &str,
        _params: &[Param],
        return_type: &Option<TypeAnnotation>,
    ) -> Result<()> {
        self.function_names.insert(name.to_string());

        let ret_ty = return_type
            .as_ref()
            .map(|t| self.convert_type(t))
            .unwrap_or(IrType::Void);
        self.function_return_types.insert(name.to_string(), ret_ty);

        Ok(())
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
            err.message.contains("Cannot have both"),
            "Error should mention the conflict"
        );
        // Verify error code is included
        assert!(
            err.message.contains("ت٠٢٠١"),
            "Error should include error code ت٠٢٠١"
        );
        assert!(
            err.message_ar.contains("ت٠٢٠١"),
            "Arabic error should include error code ت٠٢٠١"
        );
    }
}
