//! IR Builder - Converts typed AST to IR
//!
//! This module provides the `IrBuilder` which walks the AST and generates
//! the intermediate representation (IR) for code generation.

use crate::parser::{
    Ast, BinaryOp as AstBinaryOp, Block, CatchClause, ClassMember, Expr, ExprKind, LambdaBody,
    Literal, MatchArm, Param, Stmt, StmtKind, TypeAnnotation, TypeKind, UnaryOp as AstUnaryOp,
};

use super::{
    BasicBlock, BinaryOp, BlockId, Class, ClassId, Constant, FieldId, FuncId, Function,
    Instruction, IrType, MethodId, Module, Parameter, UnaryOp, VarId,
};

use std::collections::HashMap;

/// Error type for IR building
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

type Result<T> = std::result::Result<T, IrError>;

/// IR Builder for converting AST to IR
pub struct IrBuilder {
    /// The module being built
    module: Module,

    /// Current function being built
    current_function: Option<Function>,

    /// Current block ID
    current_block: BlockId,

    /// Variable counter for SSA naming
    var_counter: u32,

    /// Block counter
    block_counter: u32,

    /// Variable name to VarId mapping (for current scope)
    variables: HashMap<String, VarId>,

    /// Stack of variable scopes
    scope_stack: Vec<HashMap<String, VarId>>,

    /// Loop context stack (continue_block, break_block)
    loop_stack: Vec<(BlockId, BlockId)>,

    /// Class field information
    class_fields: HashMap<String, Vec<(String, IrType)>>,
}

impl IrBuilder {
    /// Create a new IR builder
    pub fn new(module_name: String) -> Self {
        Self {
            module: Module::new(module_name),
            current_function: None,
            current_block: BlockId(0),
            var_counter: 0,
            block_counter: 0,
            variables: HashMap::new(),
            scope_stack: Vec::new(),
            loop_stack: Vec::new(),
            class_fields: HashMap::new(),
        }
    }

    /// Build IR from an AST
    pub fn build(mut self, ast: &Ast) -> Result<Module> {
        // First pass: collect class definitions
        for stmt in &ast.statements {
            if let StmtKind::ClassDecl { name, members, .. } = &stmt.kind {
                self.collect_class(name, members)?;
            }
        }

        // Second pass: collect function signatures
        for stmt in &ast.statements {
            if let StmtKind::FuncDecl { name, params, return_type, .. } = &stmt.kind {
                self.collect_function_signature(name, params, return_type)?;
            }
        }

        // Third pass: generate IR for all statements
        // Create a main function to hold top-level code
        let mut has_top_level_code = false;
        for stmt in &ast.statements {
            match &stmt.kind {
                StmtKind::FuncDecl { .. } | StmtKind::ClassDecl { .. } | StmtKind::InterfaceDecl { .. } => {
                    // These are declarations, not executable code
                }
                _ => {
                    has_top_level_code = true;
                    break;
                }
            }
        }

        if has_top_level_code {
            self.begin_function("__main__".to_string(), vec![], IrType::Void)?;
        }

        for stmt in &ast.statements {
            self.build_stmt(stmt)?;
        }

        if has_top_level_code {
            // Add return if not already present
            if let Some(ref func) = self.current_function {
                if let Some(block) = func.blocks.last() {
                    if !block.has_terminator() {
                        self.emit(Instruction::Return { value: None });
                    }
                }
            }
            self.end_function()?;
        }

        Ok(self.module)
    }

    /// Collect class information for later use
    fn collect_class(&mut self, name: &str, members: &[ClassMember]) -> Result<()> {
        let mut fields = Vec::new();

        for member in members {
            if let ClassMember::Field { name: field_name, ty, .. } = member {
                let ir_type = ty
                    .as_ref()
                    .map(|t| self.convert_type(t))
                    .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                fields.push((field_name.clone(), ir_type));
            }
        }

        self.class_fields.insert(name.to_string(), fields.clone());

        // Create the class in the module
        let class_id = ClassId(name.to_string());
        let mut class = Class::new(class_id, name.to_string());
        class.fields = fields;

        self.module.classes.push(class);
        Ok(())
    }

    /// Collect function signature
    fn collect_function_signature(
        &mut self,
        _name: &str,
        _params: &[Param],
        _return_type: &Option<TypeAnnotation>,
    ) -> Result<()> {
        // For now, just a placeholder - full implementation would register the function
        Ok(())
    }

    /// Begin building a new function
    fn begin_function(
        &mut self,
        name: String,
        params: Vec<Parameter>,
        return_type: IrType,
    ) -> Result<()> {
        let func_id = FuncId(name.clone());
        let mut func = Function::new(func_id, name, params.clone(), return_type);

        // Initialize counters
        self.var_counter = params.len() as u32;
        self.block_counter = 0;

        // Create entry block
        let entry_block = BasicBlock::with_label(BlockId(0), "entry".to_string());
        func.blocks.push(entry_block);
        self.current_block = BlockId(0);
        self.block_counter = 1;

        // Add parameters to variable scope
        self.push_scope();
        for param in &params {
            self.variables.insert(param.name.clone(), param.id);
        }

        self.current_function = Some(func);
        Ok(())
    }

    /// End the current function and add it to the module
    fn end_function(&mut self) -> Result<()> {
        if let Some(func) = self.current_function.take() {
            self.module.functions.push(func);
        }
        self.pop_scope();
        Ok(())
    }

    /// Create a new basic block
    fn new_block(&mut self, label: Option<String>) -> BlockId {
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

    /// Switch to a different block
    fn switch_to_block(&mut self, block_id: BlockId) {
        self.current_block = block_id;
    }

    /// Allocate a new SSA variable
    fn new_var(&mut self) -> VarId {
        let id = VarId(self.var_counter);
        self.var_counter += 1;
        id
    }

    /// Emit an instruction to the current block
    fn emit(&mut self, inst: Instruction) {
        if let Some(ref mut func) = self.current_function {
            if let Some(block) = func.get_block_mut(self.current_block) {
                block.instructions.push(inst);
            }
        }
    }

    /// Push a new variable scope
    fn push_scope(&mut self) {
        self.scope_stack.push(self.variables.clone());
    }

    /// Pop a variable scope
    fn pop_scope(&mut self) {
        if let Some(vars) = self.scope_stack.pop() {
            self.variables = vars;
        }
    }

    /// Look up a variable by name
    fn lookup_var(&self, name: &str) -> Option<VarId> {
        self.variables.get(name).copied()
    }

    /// Add a string to the string table
    fn add_string(&mut self, s: String) -> u32 {
        self.module.strings.add(s)
    }

    /// Convert AST type annotation to IR type
    fn convert_type(&self, ty: &TypeAnnotation) -> IrType {
        match &ty.kind {
            TypeKind::Simple(name) => self.convert_simple_type(name),
            TypeKind::Array(inner) => IrType::Array(Box::new(self.convert_type(inner)), 0),
            TypeKind::Map(_key, _value) => {
                // Maps are represented as struct pointers for now
                IrType::Ptr(Box::new(IrType::Void))
            }
            TypeKind::Function { params, return_type } => IrType::Function {
                params: params.iter().map(|p| self.convert_type(p)).collect(),
                ret: Box::new(self.convert_type(return_type)),
            },
            TypeKind::Generic { base, .. } => {
                // For now, treat generics as their base type
                self.convert_simple_type(base)
            }
            TypeKind::Optional(inner) => {
                // Optionals are represented as nullable pointers
                IrType::Ptr(Box::new(self.convert_type(inner)))
            }
        }
    }

    /// Convert a simple type name to IR type
    fn convert_simple_type(&self, name: &str) -> IrType {
        match name {
            "عدد" | "int" => IrType::Int,
            "عدد_عشري" | "float" => IrType::Float,
            "نص" | "string" => IrType::String,
            "منطقي" | "bool" => IrType::Bool,
            "فراغ" | "void" => IrType::Void,
            _ => IrType::Struct(ClassId(name.to_string())),
        }
    }

    /// Convert semantic type to IR type
    /// Note: Currently unused, will be used when integrating with typed AST
    #[allow(dead_code)]
    fn semantic_to_ir_type(&self, ty: &crate::semantic::Type) -> IrType {
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
            SemanticType::Function { params, return_type } => IrType::Function {
                params: params.iter().map(|p| self.semantic_to_ir_type(p)).collect(),
                ret: Box::new(self.semantic_to_ir_type(return_type)),
            },
            _ => IrType::Ptr(Box::new(IrType::Void)),
        }
    }

    // ==================== Statement Building ====================

    /// Build IR for a statement
    fn build_stmt(&mut self, stmt: &Stmt) -> Result<()> {
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
            } => self.build_func_decl(name, params, return_type.as_ref(), body, *is_async),
            StmtKind::ClassDecl {
                name,
                extends,
                implements,
                members,
            } => self.build_class_decl(name, extends.as_ref(), implements, members),
            StmtKind::InterfaceDecl { .. } => {
                // Interfaces don't generate runtime code
                Ok(())
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => self.build_if(condition, then_branch, else_branch.as_ref()),
            StmtKind::While { condition, body } => self.build_while(condition, body),
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
            StmtKind::Import { .. } => {
                // Imports are handled at a different stage
                Ok(())
            }
            StmtKind::Export(inner) => self.build_stmt(inner),
            StmtKind::Expr(expr) => {
                self.build_expr(expr)?;
                Ok(())
            }
            StmtKind::Block(block) => self.build_block(block),
        }
    }

    /// Build IR for a variable declaration
    fn build_var_decl(
        &mut self,
        name: &str,
        ty: Option<&TypeAnnotation>,
        init: Option<&Expr>,
    ) -> Result<()> {
        let ir_type = ty
            .map(|t| self.convert_type(t))
            .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));

        // Allocate space for the variable
        let ptr = self.new_var();
        self.emit(Instruction::Alloca {
            dest: ptr,
            ty: ir_type.clone(),
        });

        // If there's an initializer, evaluate and store it
        if let Some(init_expr) = init {
            let value = self.build_expr(init_expr)?;
            self.emit(Instruction::Store { ptr, value });
        }

        // Record the variable location
        self.variables.insert(name.to_string(), ptr);

        Ok(())
    }

    /// Build IR for a function declaration
    fn build_func_decl(
        &mut self,
        name: &str,
        params: &[Param],
        return_type: Option<&TypeAnnotation>,
        body: &Block,
        is_async: bool,
    ) -> Result<()> {
        // Convert parameters
        let ir_params: Vec<Parameter> = params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let ty = p
                    .ty
                    .as_ref()
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

        // Save current function state
        let saved_function = self.current_function.take();
        let saved_block = self.current_block;
        let saved_var_counter = self.var_counter;
        let saved_block_counter = self.block_counter;
        let saved_variables = self.variables.clone();

        // Begin the new function
        self.begin_function(name.to_string(), ir_params, ret_type)?;

        if let Some(ref mut func) = self.current_function {
            func.is_async = is_async;
        }

        // Build the body
        for stmt in &body.statements {
            self.build_stmt(stmt)?;
        }

        // Add implicit return if needed
        if let Some(ref func) = self.current_function {
            if let Some(block) = func.blocks.last() {
                if !block.has_terminator() {
                    self.emit(Instruction::Return { value: None });
                }
            }
        }

        // End this function
        self.end_function()?;

        // Restore previous state
        self.current_function = saved_function;
        self.current_block = saved_block;
        self.var_counter = saved_var_counter;
        self.block_counter = saved_block_counter;
        self.variables = saved_variables;

        Ok(())
    }

    /// Build IR for a class declaration
    fn build_class_decl(
        &mut self,
        name: &str,
        extends: Option<&String>,
        _implements: &[String],
        members: &[ClassMember],
    ) -> Result<()> {
        // Update the class with parent info
        if let Some(parent) = extends {
            for class in &mut self.module.classes {
                if class.name == name {
                    class.parent = Some(ClassId(parent.clone()));
                    break;
                }
            }
        }

        // Generate methods
        for member in members {
            match member {
                ClassMember::Method {
                    name: method_name,
                    params,
                    return_type,
                    body,
                    is_async,
                    ..
                } => {
                    // Generate method as a function with mangled name
                    let mangled_name = format!("{}::{}", name, method_name);

                    // Add 'this' as first parameter
                    let mut method_params: Vec<Parameter> = vec![Parameter {
                        id: VarId(0),
                        name: "هذا".to_string(), // "this" in Arabic
                        ty: IrType::Struct(ClassId(name.to_string())),
                    }];

                    for (i, p) in params.iter().enumerate() {
                        let ty = p
                            .ty
                            .as_ref()
                            .map(|t| self.convert_type(t))
                            .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                        method_params.push(Parameter {
                            id: VarId((i + 1) as u32),
                            name: p.name.clone(),
                            ty,
                        });
                    }

                    let ret_type = return_type
                        .as_ref()
                        .map(|t| self.convert_type(t))
                        .unwrap_or(IrType::Void);

                    // Save state
                    let saved_function = self.current_function.take();
                    let saved_variables = self.variables.clone();

                    // Build the method
                    self.begin_function(mangled_name, method_params, ret_type)?;

                    if let Some(ref mut func) = self.current_function {
                        func.is_async = *is_async;
                    }

                    // Add 'this' to scope
                    self.variables.insert("هذا".to_string(), VarId(0));
                    self.variables.insert("this".to_string(), VarId(0));

                    for stmt in &body.statements {
                        self.build_stmt(stmt)?;
                    }

                    // Add implicit return
                    if let Some(ref func) = self.current_function {
                        if let Some(block) = func.blocks.last() {
                            if !block.has_terminator() {
                                self.emit(Instruction::Return { value: None });
                            }
                        }
                    }

                    self.end_function()?;

                    // Restore state
                    self.current_function = saved_function;
                    self.variables = saved_variables;
                }
                ClassMember::Constructor { params, body } => {
                    // Constructor is a special method
                    let mangled_name = format!("{}::منشئ", name);

                    let mut ctor_params: Vec<Parameter> = vec![Parameter {
                        id: VarId(0),
                        name: "هذا".to_string(),
                        ty: IrType::Struct(ClassId(name.to_string())),
                    }];

                    for (i, p) in params.iter().enumerate() {
                        let ty = p
                            .ty
                            .as_ref()
                            .map(|t| self.convert_type(t))
                            .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                        ctor_params.push(Parameter {
                            id: VarId((i + 1) as u32),
                            name: p.name.clone(),
                            ty,
                        });
                    }

                    // Save state
                    let saved_function = self.current_function.take();
                    let saved_variables = self.variables.clone();

                    // Build constructor
                    self.begin_function(mangled_name, ctor_params, IrType::Void)?;

                    self.variables.insert("هذا".to_string(), VarId(0));
                    self.variables.insert("this".to_string(), VarId(0));

                    for stmt in &body.statements {
                        self.build_stmt(stmt)?;
                    }

                    if let Some(ref func) = self.current_function {
                        if let Some(block) = func.blocks.last() {
                            if !block.has_terminator() {
                                self.emit(Instruction::Return { value: None });
                            }
                        }
                    }

                    self.end_function()?;

                    // Restore state
                    self.current_function = saved_function;
                    self.variables = saved_variables;
                }
                ClassMember::Field { .. } => {
                    // Fields are already collected
                }
            }
        }

        Ok(())
    }

    /// Build IR for an if statement
    fn build_if(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        else_branch: Option<&Block>,
    ) -> Result<()> {
        let cond_var = self.build_expr(condition)?;

        let then_block = self.new_block(Some("then".to_string()));
        let else_block = self.new_block(Some("else".to_string()));
        let merge_block = self.new_block(Some("merge".to_string()));

        self.emit(Instruction::Branch {
            cond: cond_var,
            then_block,
            else_block: if else_branch.is_some() {
                else_block
            } else {
                merge_block
            },
        });

        // Build then branch
        self.switch_to_block(then_block);
        self.push_scope();
        for stmt in &then_branch.statements {
            self.build_stmt(stmt)?;
        }
        self.pop_scope();

        // Jump to merge if no terminator
        if let Some(ref func) = self.current_function {
            if let Some(block) = func.get_block(self.current_block) {
                if !block.has_terminator() {
                    self.emit(Instruction::Jump {
                        target: merge_block,
                    });
                }
            }
        }

        // Build else branch if present
        if let Some(else_body) = else_branch {
            self.switch_to_block(else_block);
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

    /// Build IR for a while loop
    fn build_while(&mut self, condition: &Expr, body: &Block) -> Result<()> {
        let cond_block = self.new_block(Some("while.cond".to_string()));
        let body_block = self.new_block(Some("while.body".to_string()));
        let exit_block = self.new_block(Some("while.exit".to_string()));

        // Jump to condition
        self.emit(Instruction::Jump { target: cond_block });

        // Build condition
        self.switch_to_block(cond_block);
        let cond_var = self.build_expr(condition)?;
        self.emit(Instruction::Branch {
            cond: cond_var,
            then_block: body_block,
            else_block: exit_block,
        });

        // Push loop context
        self.loop_stack.push((cond_block, exit_block));

        // Build body
        self.switch_to_block(body_block);
        self.push_scope();
        for stmt in &body.statements {
            self.build_stmt(stmt)?;
        }
        self.pop_scope();

        // Jump back to condition
        if let Some(ref func) = self.current_function {
            if let Some(block) = func.get_block(self.current_block) {
                if !block.has_terminator() {
                    self.emit(Instruction::Jump { target: cond_block });
                }
            }
        }

        // Pop loop context
        self.loop_stack.pop();

        self.switch_to_block(exit_block);
        Ok(())
    }

    /// Build IR for a for loop
    fn build_for(
        &mut self,
        init: Option<&Stmt>,
        condition: Option<&Expr>,
        update: Option<&Expr>,
        body: &Block,
    ) -> Result<()> {
        self.push_scope();

        // Build init
        if let Some(init_stmt) = init {
            self.build_stmt(init_stmt)?;
        }

        let cond_block = self.new_block(Some("for.cond".to_string()));
        let body_block = self.new_block(Some("for.body".to_string()));
        let update_block = self.new_block(Some("for.update".to_string()));
        let exit_block = self.new_block(Some("for.exit".to_string()));

        // Jump to condition
        self.emit(Instruction::Jump { target: cond_block });

        // Build condition
        self.switch_to_block(cond_block);
        if let Some(cond_expr) = condition {
            let cond_var = self.build_expr(cond_expr)?;
            self.emit(Instruction::Branch {
                cond: cond_var,
                then_block: body_block,
                else_block: exit_block,
            });
        } else {
            // No condition means infinite loop (until break)
            self.emit(Instruction::Jump { target: body_block });
        }

        // Push loop context (continue goes to update, break goes to exit)
        self.loop_stack.push((update_block, exit_block));

        // Build body
        self.switch_to_block(body_block);
        for stmt in &body.statements {
            self.build_stmt(stmt)?;
        }

        // Jump to update
        if let Some(ref func) = self.current_function {
            if let Some(block) = func.get_block(self.current_block) {
                if !block.has_terminator() {
                    self.emit(Instruction::Jump {
                        target: update_block,
                    });
                }
            }
        }

        // Build update
        self.switch_to_block(update_block);
        if let Some(update_expr) = update {
            self.build_expr(update_expr)?;
        }
        self.emit(Instruction::Jump { target: cond_block });

        // Pop loop context
        self.loop_stack.pop();

        self.pop_scope();
        self.switch_to_block(exit_block);
        Ok(())
    }

    /// Build IR for a for-in loop
    fn build_for_in(&mut self, variable: &str, iterable: &Expr, body: &Block) -> Result<()> {
        // For now, implement as a simple indexed loop
        // Later this should use iterator protocol

        let array_var = self.build_expr(iterable)?;

        // Get array length
        let len_var = self.new_var();
        self.emit(Instruction::ArrayLen {
            dest: len_var,
            array: array_var,
        });

        // Create index variable
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

        // Condition: index < len
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

        // Push loop context
        self.loop_stack.push((update_block, exit_block));

        // Body
        self.switch_to_block(body_block);
        self.push_scope();

        // Get current element
        let index_val2 = self.new_var();
        self.emit(Instruction::Load {
            dest: index_val2,
            ptr: index_ptr,
            ty: IrType::Int,
        });
        let elem = self.new_var();
        self.emit(Instruction::ArrayGet {
            dest: elem,
            array: array_var,
            index: index_val2,
            elem_ty: IrType::Ptr(Box::new(IrType::Void)), // Generic element type
        });

        // Bind to variable
        self.variables.insert(variable.to_string(), elem);

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

        // Update: index++
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

        // Pop loop context
        self.loop_stack.pop();

        self.switch_to_block(exit_block);
        Ok(())
    }

    /// Build IR for a match statement
    fn build_match(&mut self, expr: &Expr, arms: &[MatchArm]) -> Result<()> {
        let match_val = self.build_expr(expr)?;
        let exit_block = self.new_block(Some("match.exit".to_string()));

        // Create blocks for each arm
        let arm_blocks: Vec<BlockId> = arms
            .iter()
            .enumerate()
            .map(|(i, _)| self.new_block(Some(format!("match.arm{}", i))))
            .collect();

        // Build condition chain
        for (i, arm) in arms.iter().enumerate() {
            // For each pattern in the arm
            for pattern in &arm.patterns {
                let pattern_val = self.build_expr(pattern)?;
                let cmp = self.new_var();
                self.emit(Instruction::Binary {
                    dest: cmp,
                    op: BinaryOp::Eq,
                    left: match_val,
                    right: pattern_val,
                    ty: IrType::Bool,
                });

                let next_check = if i + 1 < arms.len() {
                    self.new_block(Some(format!("match.check{}", i + 1)))
                } else {
                    exit_block
                };

                self.emit(Instruction::Branch {
                    cond: cmp,
                    then_block: arm_blocks[i],
                    else_block: next_check,
                });

                if i + 1 < arms.len() {
                    self.switch_to_block(next_check);
                }
            }
        }

        // Build arm bodies
        for (i, arm) in arms.iter().enumerate() {
            self.switch_to_block(arm_blocks[i]);
            self.push_scope();
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

    /// Build IR for a return statement
    fn build_return(&mut self, expr: Option<&Expr>) -> Result<()> {
        let value = if let Some(e) = expr {
            Some(self.build_expr(e)?)
        } else {
            None
        };

        self.emit(Instruction::Return { value });
        Ok(())
    }

    /// Build IR for a break statement
    fn build_break(&mut self) -> Result<()> {
        if let Some((_, exit_block)) = self.loop_stack.last() {
            self.emit(Instruction::Jump { target: *exit_block });
            Ok(())
        } else {
            Err(IrError::new(
                "break outside of loop",
                "كسر خارج حلقة",
            ))
        }
    }

    /// Build IR for a continue statement
    fn build_continue(&mut self) -> Result<()> {
        if let Some((continue_block, _)) = self.loop_stack.last() {
            self.emit(Instruction::Jump {
                target: *continue_block,
            });
            Ok(())
        } else {
            Err(IrError::new(
                "continue outside of loop",
                "استمر خارج حلقة",
            ))
        }
    }

    /// Build IR for a try statement
    fn build_try(
        &mut self,
        body: &Block,
        catch: Option<&CatchClause>,
        finally: Option<&Block>,
    ) -> Result<()> {
        let catch_block = self.new_block(Some("catch".to_string()));
        let finally_block = self.new_block(Some("finally".to_string()));
        let exit_block = self.new_block(Some("try.exit".to_string()));

        // Begin try region
        self.emit(Instruction::TryBegin { catch_block });

        // Build try body
        self.push_scope();
        for stmt in &body.statements {
            self.build_stmt(stmt)?;
        }
        self.pop_scope();

        // End try region
        self.emit(Instruction::TryEnd);

        // Jump to finally (or exit if no finally)
        if finally.is_some() {
            self.emit(Instruction::Jump {
                target: finally_block,
            });
        } else {
            self.emit(Instruction::Jump { target: exit_block });
        }

        // Build catch block
        self.switch_to_block(catch_block);
        if let Some(catch_clause) = catch {
            self.push_scope();

            // Get the exception
            let exception_var = self.new_var();
            self.emit(Instruction::GetException {
                dest: exception_var,
            });
            self.variables
                .insert(catch_clause.param.clone(), exception_var);

            for stmt in &catch_clause.body.statements {
                self.build_stmt(stmt)?;
            }
            self.pop_scope();
        }

        if finally.is_some() {
            self.emit(Instruction::Jump {
                target: finally_block,
            });
        } else {
            self.emit(Instruction::Jump { target: exit_block });
        }

        // Build finally block
        if let Some(finally_body) = finally {
            self.switch_to_block(finally_block);
            self.push_scope();
            for stmt in &finally_body.statements {
                self.build_stmt(stmt)?;
            }
            self.pop_scope();
            self.emit(Instruction::Jump { target: exit_block });
        }

        self.switch_to_block(exit_block);
        Ok(())
    }

    /// Build IR for a throw statement
    fn build_throw(&mut self, expr: &Expr) -> Result<()> {
        let exception = self.build_expr(expr)?;
        self.emit(Instruction::Throw { exception });
        Ok(())
    }

    /// Build IR for a block
    fn build_block(&mut self, block: &Block) -> Result<()> {
        self.push_scope();
        for stmt in &block.statements {
            self.build_stmt(stmt)?;
        }
        self.pop_scope();
        Ok(())
    }

    // ==================== Expression Building ====================

    /// Build IR for an expression, returning the result VarId
    fn build_expr(&mut self, expr: &Expr) -> Result<VarId> {
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
            ExprKind::New { class, args } => self.build_new(class, args),
            ExprKind::Await(inner) => self.build_await(inner),
            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => self.build_ternary(condition, then_expr, else_expr),
            ExprKind::Grouping(inner) => self.build_expr(inner),
            ExprKind::This => self.build_this(),
            ExprKind::Super => self.build_super(),
        }
    }

    /// Build IR for a literal
    fn build_literal(&mut self, lit: &Literal) -> Result<VarId> {
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

        self.emit(Instruction::Const { dest, value, ty });
        Ok(dest)
    }

    /// Build IR for an identifier
    fn build_identifier(&mut self, name: &str) -> Result<VarId> {
        if let Some(var_ptr) = self.lookup_var(name) {
            // Load the value from the variable's location
            let dest = self.new_var();
            self.emit(Instruction::Load {
                dest,
                ptr: var_ptr,
                ty: IrType::Ptr(Box::new(IrType::Void)), // Will be refined by type info
            });
            Ok(dest)
        } else {
            // Could be a function reference
            // For now, return a placeholder
            let dest = self.new_var();
            self.emit(Instruction::Const {
                dest,
                value: Constant::Null,
                ty: IrType::Ptr(Box::new(IrType::Void)),
            });
            Ok(dest)
        }
    }

    /// Build IR for a binary expression
    fn build_binary(&mut self, left: &Expr, op: AstBinaryOp, right: &Expr) -> Result<VarId> {
        let left_var = self.build_expr(left)?;
        let right_var = self.build_expr(right)?;

        let ir_op = match op {
            AstBinaryOp::Add => BinaryOp::Add,
            AstBinaryOp::Sub => BinaryOp::Sub,
            AstBinaryOp::Mul => BinaryOp::Mul,
            AstBinaryOp::Div => BinaryOp::Div,
            AstBinaryOp::Mod => BinaryOp::Mod,
            AstBinaryOp::Pow => {
                // Power needs special handling - for now treat as mul
                BinaryOp::Mul
            }
            AstBinaryOp::Eq => BinaryOp::Eq,
            AstBinaryOp::NotEq => BinaryOp::Ne,
            AstBinaryOp::Lt => BinaryOp::Lt,
            AstBinaryOp::LtEq => BinaryOp::Le,
            AstBinaryOp::Gt => BinaryOp::Gt,
            AstBinaryOp::GtEq => BinaryOp::Ge,
            AstBinaryOp::And => BinaryOp::And,
            AstBinaryOp::Or => BinaryOp::Or,
        };

        // Determine result type (simplified)
        let result_ty = match ir_op {
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt
            | BinaryOp::Ge | BinaryOp::And | BinaryOp::Or => IrType::Bool,
            _ => IrType::Int, // Default to int for arithmetic
        };

        let dest = self.new_var();
        self.emit(Instruction::Binary {
            dest,
            op: ir_op,
            left: left_var,
            right: right_var,
            ty: result_ty,
        });

        Ok(dest)
    }

    /// Build IR for a unary expression
    fn build_unary(&mut self, op: AstUnaryOp, operand: &Expr) -> Result<VarId> {
        let operand_var = self.build_expr(operand)?;

        match op {
            AstUnaryOp::Neg => {
                let dest = self.new_var();
                self.emit(Instruction::Unary {
                    dest,
                    op: UnaryOp::Neg,
                    operand: operand_var,
                    ty: IrType::Int,
                });
                Ok(dest)
            }
            AstUnaryOp::Not => {
                let dest = self.new_var();
                self.emit(Instruction::Unary {
                    dest,
                    op: UnaryOp::Not,
                    operand: operand_var,
                    ty: IrType::Bool,
                });
                Ok(dest)
            }
            AstUnaryOp::PreInc | AstUnaryOp::PostInc => {
                // x++ or ++x
                let one = self.new_var();
                self.emit(Instruction::Const {
                    dest: one,
                    value: Constant::Int(1),
                    ty: IrType::Int,
                });
                let dest = self.new_var();
                self.emit(Instruction::Binary {
                    dest,
                    op: BinaryOp::Add,
                    left: operand_var,
                    right: one,
                    ty: IrType::Int,
                });
                Ok(dest)
            }
            AstUnaryOp::PreDec | AstUnaryOp::PostDec => {
                // x-- or --x
                let one = self.new_var();
                self.emit(Instruction::Const {
                    dest: one,
                    value: Constant::Int(1),
                    ty: IrType::Int,
                });
                let dest = self.new_var();
                self.emit(Instruction::Binary {
                    dest,
                    op: BinaryOp::Sub,
                    left: operand_var,
                    right: one,
                    ty: IrType::Int,
                });
                Ok(dest)
            }
        }
    }

    /// Build IR for a function call
    fn build_call(&mut self, callee: &Expr, args: &[Expr]) -> Result<VarId> {
        // Build arguments
        let arg_vars: Vec<VarId> = args
            .iter()
            .map(|a| self.build_expr(a))
            .collect::<Result<Vec<_>>>()?;

        // Check if it's a built-in function
        if let ExprKind::Identifier(name) = &callee.kind {
            if name == "اطبع" || name == "print" {
                // Special handling for print
                if let Some(arg) = arg_vars.first() {
                    self.emit(Instruction::Print { value: *arg });
                }
                // Return void
                let dest = self.new_var();
                self.emit(Instruction::Const {
                    dest,
                    value: Constant::Null,
                    ty: IrType::Void,
                });
                return Ok(dest);
            }

            // Regular function call
            let dest = self.new_var();
            self.emit(Instruction::Call {
                dest: Some(dest),
                func: FuncId(name.clone()),
                args: arg_vars,
                ret_ty: IrType::Ptr(Box::new(IrType::Void)),
            });
            return Ok(dest);
        }

        // Method call or indirect call
        if let ExprKind::Member { object, property } = &callee.kind {
            let obj_var = self.build_expr(object)?;

            // Method call
            let dest = self.new_var();
            self.emit(Instruction::CallMethod {
                dest: Some(dest),
                object: obj_var,
                method: MethodId {
                    class: ClassId("".to_string()), // Will be resolved later
                    name: property.clone(),
                },
                args: arg_vars,
                ret_ty: IrType::Ptr(Box::new(IrType::Void)),
            });
            return Ok(dest);
        }

        // Generic indirect call
        let callee_var = self.build_expr(callee)?;
        let dest = self.new_var();
        self.emit(Instruction::CallIndirect {
            dest: Some(dest),
            func_ptr: callee_var,
            args: arg_vars,
            ret_ty: IrType::Ptr(Box::new(IrType::Void)),
        });
        Ok(dest)
    }

    /// Build IR for member access
    fn build_member(&mut self, object: &Expr, property: &str) -> Result<VarId> {
        let obj_var = self.build_expr(object)?;
        let dest = self.new_var();

        // For now, assume it's a field access
        // In a full implementation, we'd need type information
        self.emit(Instruction::GetField {
            dest,
            object: obj_var,
            field: FieldId {
                class: ClassId("".to_string()), // Will be resolved later
                name: property.to_string(),
                index: 0,
            },
            ty: IrType::Ptr(Box::new(IrType::Void)),
        });

        Ok(dest)
    }

    /// Build IR for index access
    fn build_index(&mut self, object: &Expr, index: &Expr) -> Result<VarId> {
        let obj_var = self.build_expr(object)?;
        let idx_var = self.build_expr(index)?;
        let dest = self.new_var();

        self.emit(Instruction::ArrayGet {
            dest,
            array: obj_var,
            index: idx_var,
            elem_ty: IrType::Ptr(Box::new(IrType::Void)),
        });

        Ok(dest)
    }

    /// Build IR for assignment
    fn build_assignment(&mut self, target: &Expr, value: &Expr) -> Result<VarId> {
        let value_var = self.build_expr(value)?;

        match &target.kind {
            ExprKind::Identifier(name) => {
                if let Some(ptr) = self.lookup_var(name) {
                    self.emit(Instruction::Store {
                        ptr,
                        value: value_var,
                    });
                }
            }
            ExprKind::Member { object, property } => {
                let obj_var = self.build_expr(object)?;
                self.emit(Instruction::SetField {
                    object: obj_var,
                    field: FieldId {
                        class: ClassId("".to_string()),
                        name: property.clone(),
                        index: 0,
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
            _ => {}
        }

        Ok(value_var)
    }

    /// Build IR for compound assignment (+=, -=, etc.)
    fn build_compound_assignment(
        &mut self,
        target: &Expr,
        op: AstBinaryOp,
        value: &Expr,
    ) -> Result<VarId> {
        // First load the current value
        let current = self.build_expr(target)?;
        let increment = self.build_expr(value)?;

        // Perform the operation
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

        // Store back
        if let ExprKind::Identifier(name) = &target.kind {
            if let Some(ptr) = self.lookup_var(name) {
                self.emit(Instruction::Store { ptr, value: result });
            }
        }

        Ok(result)
    }

    /// Build IR for array literal
    fn build_array(&mut self, elements: &[Expr]) -> Result<VarId> {
        let elem_vars: Vec<VarId> = elements
            .iter()
            .map(|e| self.build_expr(e))
            .collect::<Result<Vec<_>>>()?;

        let dest = self.new_var();
        self.emit(Instruction::NewArray {
            dest,
            elem_ty: IrType::Ptr(Box::new(IrType::Void)),
            elements: elem_vars,
        });

        Ok(dest)
    }

    /// Build IR for object literal
    fn build_object(&mut self, fields: &[(String, Expr)]) -> Result<VarId> {
        // Objects are represented as anonymous structs for now
        let dest = self.new_var();
        self.emit(Instruction::NewObject {
            dest,
            class: ClassId("__anonymous__".to_string()),
        });

        for (name, expr) in fields {
            let value = self.build_expr(expr)?;
            self.emit(Instruction::SetField {
                object: dest,
                field: FieldId {
                    class: ClassId("__anonymous__".to_string()),
                    name: name.clone(),
                    index: 0,
                },
                value,
            });
        }

        Ok(dest)
    }

    /// Build IR for lambda expression
    fn build_lambda(&mut self, params: &[Param], body: &LambdaBody) -> Result<VarId> {
        // Generate a unique name for the lambda
        let lambda_name = format!("__lambda_{}", self.var_counter);

        let ir_params: Vec<Parameter> = params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let ty = p
                    .ty
                    .as_ref()
                    .map(|t| self.convert_type(t))
                    .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                Parameter {
                    id: VarId(i as u32),
                    name: p.name.clone(),
                    ty,
                }
            })
            .collect();

        // Save state
        let saved_function = self.current_function.take();
        let saved_variables = self.variables.clone();

        // Build lambda as a function
        self.begin_function(
            lambda_name.clone(),
            ir_params,
            IrType::Ptr(Box::new(IrType::Void)),
        )?;

        match body {
            LambdaBody::Expr(expr) => {
                let result = self.build_expr(expr)?;
                self.emit(Instruction::Return {
                    value: Some(result),
                });
            }
            LambdaBody::Block(block) => {
                for stmt in &block.statements {
                    self.build_stmt(stmt)?;
                }
                // Add implicit return if needed
                if let Some(ref func) = self.current_function {
                    if let Some(blk) = func.blocks.last() {
                        if !blk.has_terminator() {
                            self.emit(Instruction::Return { value: None });
                        }
                    }
                }
            }
        }

        self.end_function()?;

        // Restore state
        self.current_function = saved_function;
        self.variables = saved_variables;

        // Return a reference to the lambda function
        let dest = self.new_var();
        self.emit(Instruction::Const {
            dest,
            value: Constant::Null, // Will be replaced with function pointer
            ty: IrType::Function {
                params: vec![],
                ret: Box::new(IrType::Void),
            },
        });

        Ok(dest)
    }

    /// Build IR for new expression
    fn build_new(&mut self, class: &Expr, args: &[Expr]) -> Result<VarId> {
        // Get class name
        let class_name = if let ExprKind::Identifier(name) = &class.kind {
            name.clone()
        } else {
            "__dynamic__".to_string()
        };

        // Allocate object
        let dest = self.new_var();
        self.emit(Instruction::NewObject {
            dest,
            class: ClassId(class_name.clone()),
        });

        // Build constructor arguments
        let arg_vars: Vec<VarId> = args
            .iter()
            .map(|a| self.build_expr(a))
            .collect::<Result<Vec<_>>>()?;

        // Call constructor
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

    /// Build IR for await expression
    fn build_await(&mut self, inner: &Expr) -> Result<VarId> {
        // For now, just evaluate the inner expression
        // Real async support would require more complex transformation
        self.build_expr(inner)
    }

    /// Build IR for ternary expression
    fn build_ternary(
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

        // Build then branch
        self.switch_to_block(then_block);
        let then_var = self.build_expr(then_expr)?;
        let then_exit_block = self.current_block;
        self.emit(Instruction::Jump {
            target: merge_block,
        });

        // Build else branch
        self.switch_to_block(else_block);
        let else_var = self.build_expr(else_expr)?;
        let else_exit_block = self.current_block;
        self.emit(Instruction::Jump {
            target: merge_block,
        });

        // Merge with phi
        self.switch_to_block(merge_block);
        let result = self.new_var();
        self.emit(Instruction::Phi {
            dest: result,
            ty: IrType::Ptr(Box::new(IrType::Void)),
            incoming: vec![(then_var, then_exit_block), (else_var, else_exit_block)],
        });

        Ok(result)
    }

    /// Build IR for 'this' reference
    fn build_this(&mut self) -> Result<VarId> {
        if let Some(var) = self.lookup_var("هذا").or_else(|| self.lookup_var("this")) {
            Ok(var)
        } else {
            // Return null if not in a method context
            let dest = self.new_var();
            self.emit(Instruction::Const {
                dest,
                value: Constant::Null,
                ty: IrType::Ptr(Box::new(IrType::Void)),
            });
            Ok(dest)
        }
    }

    /// Build IR for 'super' reference
    fn build_super(&mut self) -> Result<VarId> {
        // Super is essentially 'this' but for method resolution
        self.build_this()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn build_ir(source: &str) -> Result<Module> {
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("Failed to parse");
        let builder = IrBuilder::new("test".to_string());
        builder.build(&ast)
    }

    #[test]
    fn test_simple_var_decl() {
        let source = "متغير س = 5";
        let module = build_ir(source).expect("Failed to build IR");
        assert_eq!(module.functions.len(), 1); // __main__
        let main = &module.functions[0];
        assert!(main.blocks[0].instructions.len() >= 2);
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
        // Should have entry, then, and merge blocks
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
        // Should have entry, cond, body, and exit blocks
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
}
