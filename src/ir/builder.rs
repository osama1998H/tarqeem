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

use std::collections::{HashMap, HashSet};

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

    /// Variable type tracking - maps VarId to its IrType
    var_types: HashMap<u32, IrType>,

    /// Stack of variable scopes
    scope_stack: Vec<HashMap<String, VarId>>,

    /// Loop context stack (continue_block, break_block)
    loop_stack: Vec<(BlockId, BlockId)>,

    /// Class field information
    class_fields: HashMap<String, Vec<(String, IrType)>>,

    /// Method return types: (ClassName::MethodName) -> IrType
    method_return_types: HashMap<String, IrType>,

    /// Known function names for identifier resolution
    function_names: HashSet<String>,

    /// Function return types: name -> IrType
    /// Used to properly type recursive calls and cross-function calls
    function_return_types: HashMap<String, IrType>,

    /// Track which VarIds are function parameters (not allocas)
    /// Parameters are passed by value and don't need Load instructions
    parameters: HashSet<u32>,

    /// Global constants: name -> (value, type)
    /// These are constants declared at module level that are visible in all functions
    global_constants: HashMap<String, (Constant, IrType)>,
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
            var_types: HashMap::new(),
            scope_stack: Vec::new(),
            loop_stack: Vec::new(),
            class_fields: HashMap::new(),
            method_return_types: HashMap::new(),
            function_names: HashSet::new(),
            function_return_types: HashMap::new(),
            parameters: HashSet::new(),
            global_constants: HashMap::new(),
        }
    }

    /// Build IR from an AST
    pub fn build(mut self, ast: &Ast) -> Result<Module> {
        // First pass: collect global constants (immutable VarDecls at module level)
        for stmt in &ast.statements {
            if let StmtKind::VarDecl { name, mutable: false, ty, init } = &stmt.kind {
                if let Some(init_expr) = init {
                    if let Some(const_val) = self.try_evaluate_const(init_expr) {
                        let ir_type = if let Some(t) = ty {
                            self.convert_type(t)
                        } else {
                            self.const_to_type(&const_val)
                        };
                        self.global_constants.insert(name.clone(), (const_val, ir_type));
                    }
                }
            }
        }

        // Second pass: collect class definitions
        for stmt in &ast.statements {
            if let StmtKind::ClassDecl { name, members, .. } = &stmt.kind {
                self.collect_class(name, members)?;
            }
        }

        // Third pass: collect function signatures
        for stmt in &ast.statements {
            if let StmtKind::FuncDecl { name, params, return_type, .. } = &stmt.kind {
                self.collect_function_signature(name, params, return_type)?;
            }
        }

        // Fourth pass: generate IR for all statements
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
            match member {
                ClassMember::Field { name: field_name, ty, .. } => {
                    let ir_type = ty
                        .as_ref()
                        .map(|t| self.convert_type(t))
                        .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                    fields.push((field_name.clone(), ir_type));
                }
                ClassMember::Method { name: method_name, return_type, .. } => {
                    // Store method return type
                    let ret_ty = return_type
                        .as_ref()
                        .map(|t| self.convert_type(t))
                        .unwrap_or(IrType::Void);
                    let full_name = format!("{}::{}", name, method_name);
                    self.method_return_types.insert(full_name, ret_ty);
                }
                _ => {}
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
        name: &str,
        _params: &[Param],
        return_type: &Option<TypeAnnotation>,
    ) -> Result<()> {
        // Register the function name for identifier resolution
        self.function_names.insert(name.to_string());

        // Store the return type for proper typing of recursive and cross-function calls
        let ret_ty = return_type
            .as_ref()
            .map(|t| self.convert_type(t))
            .unwrap_or(IrType::Void);
        self.function_return_types.insert(name.to_string(), ret_ty);

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
        // Clear variables to prevent leakage from previous scopes into function
        self.push_scope();
        self.variables.clear();
        self.parameters.clear();
        for param in &params {
            self.variables.insert(param.name.clone(), param.id);
            // Mark this VarId as a parameter (passed by value, not an alloca)
            self.parameters.insert(param.id.0);
            // Track the parameter's type
            self.var_types.insert(param.id.0, param.ty.clone());
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
            TypeKind::Generic { base, args } => {
                // Handle built-in generic types like مصفوفة<عدد> (Array<Int>)
                match base.as_str() {
                    "مصفوفة" | "array" | "Array" => {
                        if let Some(elem_type) = args.first() {
                            IrType::Array(Box::new(self.convert_type(elem_type)), 0)
                        } else {
                            IrType::Array(Box::new(IrType::Ptr(Box::new(IrType::Void))), 0)
                        }
                    }
                    "قاموس" | "map" | "Map" | "dict" | "Dict" => {
                        // Maps are represented as struct pointers for now
                        IrType::Ptr(Box::new(IrType::Void))
                    }
                    _ => {
                        // For other generics, treat as the base type
                        self.convert_simple_type(base)
                    }
                }
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

    /// Try to evaluate an expression as a compile-time constant
    fn try_evaluate_const(&mut self, expr: &Expr) -> Option<Constant> {
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
                    (Constant::Int(a), AstBinaryOp::Add, Constant::Int(b)) => Some(Constant::Int(a + b)),
                    (Constant::Int(a), AstBinaryOp::Sub, Constant::Int(b)) => Some(Constant::Int(a - b)),
                    (Constant::Int(a), AstBinaryOp::Mul, Constant::Int(b)) => Some(Constant::Int(a * b)),
                    (Constant::Int(a), AstBinaryOp::Div, Constant::Int(b)) if b != 0 => Some(Constant::Int(a / b)),
                    (Constant::Float(a), AstBinaryOp::Add, Constant::Float(b)) => Some(Constant::Float(a + b)),
                    (Constant::Float(a), AstBinaryOp::Sub, Constant::Float(b)) => Some(Constant::Float(a - b)),
                    (Constant::Float(a), AstBinaryOp::Mul, Constant::Float(b)) => Some(Constant::Float(a * b)),
                    (Constant::Float(a), AstBinaryOp::Div, Constant::Float(b)) if b != 0.0 => Some(Constant::Float(a / b)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Get the IR type for a constant value
    fn const_to_type(&self, constant: &Constant) -> IrType {
        match constant {
            Constant::Int(_) => IrType::Int,
            Constant::Float(_) => IrType::Float,
            Constant::Bool(_) => IrType::Bool,
            Constant::String(_) => IrType::String,
            Constant::Null => IrType::Ptr(Box::new(IrType::Void)),
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
        // Determine the type from annotation or infer from initializer
        let ir_type = if let Some(t) = ty {
            self.convert_type(t)
        } else if let Some(init_expr) = init {
            self.infer_expr_type(init_expr)
        } else {
            IrType::Ptr(Box::new(IrType::Void))
        };

        // Allocate space for the variable
        let ptr = self.new_var();
        self.emit(Instruction::Alloca {
            dest: ptr,
            ty: ir_type.clone(),
        });

        // Track the variable's type
        self.var_types.insert(ptr.0, ir_type.clone());

        // If there's an initializer, evaluate and store it
        if let Some(init_expr) = init {
            let value = self.build_expr(init_expr)?;
            self.emit(Instruction::Store { ptr, value });
        }

        // Record the variable location
        self.variables.insert(name.to_string(), ptr);

        Ok(())
    }

    /// Infer IR type from an expression (simplified)
    fn infer_expr_type(&self, expr: &Expr) -> IrType {
        match &expr.kind {
            ExprKind::Literal(lit) => match lit {
                Literal::Int(_) => IrType::Int,
                Literal::Float(_) => IrType::Float,
                Literal::String(_) => IrType::String,
                Literal::Bool(_) => IrType::Bool,
                Literal::Null => IrType::Ptr(Box::new(IrType::Void)),
            },
            ExprKind::Array(elements) => {
                // Infer element type from first element
                let elem_ty = if let Some(first) = elements.first() {
                    self.infer_expr_type(first)
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                };
                IrType::Array(Box::new(elem_ty), elements.len())
            }
            ExprKind::Binary { op, left, right } => match op {
                AstBinaryOp::Eq | AstBinaryOp::NotEq |
                AstBinaryOp::Lt | AstBinaryOp::LtEq |
                AstBinaryOp::Gt | AstBinaryOp::GtEq |
                AstBinaryOp::And | AstBinaryOp::Or => IrType::Bool,
                AstBinaryOp::Add => {
                    // Handle string concatenation
                    let left_ty = self.infer_expr_type(left);
                    let right_ty = self.infer_expr_type(right);
                    if matches!(left_ty, IrType::String) || matches!(right_ty, IrType::String) {
                        IrType::String
                    } else if matches!(left_ty, IrType::Float) || matches!(right_ty, IrType::Float) {
                        IrType::Float
                    } else {
                        IrType::Int
                    }
                }
                AstBinaryOp::Sub | AstBinaryOp::Mul | AstBinaryOp::Div | AstBinaryOp::Mod => {
                    // Float if either operand is float
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
            ExprKind::Unary { op, operand } => match op {
                AstUnaryOp::Not => IrType::Bool,
                AstUnaryOp::Neg | AstUnaryOp::PreInc | AstUnaryOp::PreDec |
                AstUnaryOp::PostInc | AstUnaryOp::PostDec => {
                    // Preserve operand type (Int or Float)
                    let operand_ty = self.infer_expr_type(operand);
                    match operand_ty {
                        IrType::Float => IrType::Float,
                        _ => IrType::Int,
                    }
                }
            },
            ExprKind::New { class, .. } => {
                // New returns a pointer to a heap-allocated object
                if let ExprKind::Identifier(name) = &class.kind {
                    IrType::Ptr(Box::new(IrType::Struct(ClassId(name.clone()))))
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Identifier(name) => {
                // Look up the variable's type
                if let Some(ptr) = self.lookup_var(name) {
                    self.var_types.get(&ptr.0).cloned().unwrap_or(IrType::Ptr(Box::new(IrType::Void)))
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Index { object, .. } => {
                // Get element type from array
                let obj_ty = self.infer_expr_type(object);
                if let IrType::Array(elem, _) = obj_ty {
                    (*elem).clone()
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Member { object, property } => {
                // Get field type from class if known
                let obj_ty = self.infer_expr_type(object);
                if let IrType::Struct(class_id) = obj_ty {
                    self.get_field_type(&class_id.0, property)
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Call { callee, .. } => {
                // Get function return type if known
                if let ExprKind::Identifier(name) = &callee.kind {
                    self.get_function_return_type(name)
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Ternary { then_expr, .. } => {
                // Return type of the 'then' branch
                self.infer_expr_type(then_expr)
            }
            ExprKind::This => {
                // Look up the type of 'this' from the first parameter (which is always 'this')
                if let Some(var_id) = self.lookup_var("هذا").or_else(|| self.lookup_var("this")) {
                    self.var_types.get(&var_id.0).cloned().unwrap_or(IrType::Ptr(Box::new(IrType::Void)))
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            _ => IrType::Ptr(Box::new(IrType::Void)),
        }
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
        let is_void_function = ret_type == IrType::Void;

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

        // Add implicit return if needed - check CURRENT block, not last block
        // After if-else, current block is the merge block which may need a return
        // Only add implicit return for void functions - non-void functions must have explicit returns
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
        let merge_block = self.new_block(Some("merge".to_string()));

        // Only create else block if there's an else branch
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

        // Determine element type from array type
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

        // Create alloca for the loop variable and store the element
        let elem_ptr = self.new_var();
        self.emit(Instruction::Alloca {
            dest: elem_ptr,
            ty: elem_ty.clone(),
        });
        self.emit(Instruction::Store {
            ptr: elem_ptr,
            value: elem,
        });

        // Track the element type
        self.var_types.insert(elem.0, elem_ty.clone());
        self.var_types.insert(elem_ptr.0, elem_ty);

        // Bind to variable (the alloca pointer, not the value)
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

        // Build condition chain with proper handling for multi-pattern arms
        // Each pattern gets its own check block to ensure proper control flow
        for (i, arm) in arms.iter().enumerate() {
            let patterns = &arm.patterns;

            for (p_idx, pattern) in patterns.iter().enumerate() {
                let pattern_val = self.build_expr(pattern)?;
                let cmp = self.new_var();
                self.emit(Instruction::Binary {
                    dest: cmp,
                    op: BinaryOp::Eq,
                    left: match_val,
                    right: pattern_val,
                    ty: IrType::Bool,
                });

                // Determine the else block (what to do if pattern doesn't match)
                let else_block = if p_idx + 1 < patterns.len() {
                    // More patterns in this arm - create block for next pattern check
                    self.new_block(Some(format!("match.arm{}.pat{}", i, p_idx + 1)))
                } else if i + 1 < arms.len() {
                    // No more patterns in this arm - go to next arm's first pattern
                    self.new_block(Some(format!("match.check{}", i + 1)))
                } else {
                    // Last pattern of last arm - go to exit
                    exit_block
                };

                self.emit(Instruction::Branch {
                    cond: cmp,
                    then_block: arm_blocks[i],
                    else_block,
                });

                // Switch to the else block for the next iteration
                self.switch_to_block(else_block);
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

        // Track the literal's type
        self.var_types.insert(dest.0, ty.clone());

        self.emit(Instruction::Const { dest, value, ty });
        Ok(dest)
    }

    /// Build IR for an identifier
    fn build_identifier(&mut self, name: &str) -> Result<VarId> {
        if let Some(var_id) = self.lookup_var(name) {
            // Check if this is a function parameter (passed by value)
            if self.parameters.contains(&var_id.0) {
                // Parameters are values, not pointers - return directly
                return Ok(var_id);
            }

            // Get the actual type from tracking (for local variables)
            let var_type = self.var_types.get(&var_id.0)
                .cloned()
                .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));

            // Load the value from the variable's location (alloca)
            let dest = self.new_var();
            self.emit(Instruction::Load {
                dest,
                ptr: var_id,
                ty: var_type.clone(),
            });

            // Track the loaded value's type
            self.var_types.insert(dest.0, var_type);

            Ok(dest)
        } else if self.function_names.contains(name) {
            // Function reference - emit a function pointer constant
            let dest = self.new_var();
            self.emit(Instruction::Const {
                dest,
                value: Constant::Null, // Will be replaced with actual function pointer in codegen
                ty: IrType::Ptr(Box::new(IrType::Void)),
            });
            Ok(dest)
        } else if let Some((const_val, const_ty)) = self.global_constants.get(name).cloned() {
            // Global constant - emit the constant value directly
            let dest = self.new_var();
            self.emit(Instruction::Const {
                dest,
                value: const_val,
                ty: const_ty.clone(),
            });
            self.var_types.insert(dest.0, const_ty);
            Ok(dest)
        } else {
            // Undefined identifier - report error
            Err(IrError::new(
                format!("Undefined identifier: '{}'", name),
                format!("معرّف غير معرّف: '{}'", name),
            ))
        }
    }

    /// Build IR for a binary expression
    fn build_binary(&mut self, left: &Expr, op: AstBinaryOp, right: &Expr) -> Result<VarId> {
        let left_var = self.build_expr(left)?;
        let right_var = self.build_expr(right)?;

        // Get operand types for better type inference
        let left_ty = self.var_types.get(&left_var.0).cloned().unwrap_or(IrType::Int);
        let right_ty = self.var_types.get(&right_var.0).cloned().unwrap_or(IrType::Int);

        // Handle string concatenation with type coercion
        if matches!(op, AstBinaryOp::Add) {
            let is_left_string = matches!(left_ty, IrType::String);
            let is_right_string = matches!(right_ty, IrType::String);

            if is_left_string || is_right_string {
                // Convert non-string operands to strings
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

                // Emit string concatenation
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

        // Determine result type based on operation and operand types
        let result_ty = match ir_op {
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt
            | BinaryOp::Ge | BinaryOp::And | BinaryOp::Or => IrType::Bool,
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod | BinaryOp::Pow => {
                // Promote to Float if either operand is Float
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

        // Track the result type
        self.var_types.insert(dest.0, result_ty);

        Ok(dest)
    }

    /// Convert a value to a string for concatenation
    fn convert_to_string(&mut self, var: VarId, ty: &IrType) -> Result<VarId> {
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

    /// Build IR for a unary expression
    fn build_unary(&mut self, op: AstUnaryOp, operand: &Expr) -> Result<VarId> {
        match op {
            AstUnaryOp::Neg => {
                // Negation: get operand type and value
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

    /// Build increment/decrement with store-back
    fn build_increment(&mut self, operand: &Expr, is_increment: bool, is_prefix: bool) -> Result<VarId> {
        // Get variable pointer - must be an lvalue
        let ptr = match &operand.kind {
            ExprKind::Identifier(name) => {
                self.lookup_var(name).ok_or_else(|| IrError::new(
                    format!("Cannot modify undefined variable '{}'", name),
                    format!("لا يمكن تعديل متغير غير معرّف '{}'", name),
                ))?
            }
            _ => return Err(IrError::new(
                "Increment/decrement requires a variable",
                "الزيادة/النقصان تتطلب متغيراً",
            )),
        };

        // Get variable type
        let var_type = self.var_types.get(&ptr.0).cloned().unwrap_or(IrType::Int);
        let result_ty = match var_type {
            IrType::Float => IrType::Float,
            _ => IrType::Int,
        };

        // Load current value
        let old_val = self.new_var();
        self.emit(Instruction::Load {
            dest: old_val,
            ptr,
            ty: result_ty.clone(),
        });
        self.var_types.insert(old_val.0, result_ty.clone());

        // Create constant 1
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

        // Compute new value: old_val +/- 1
        let new_val = self.new_var();
        let op = if is_increment { BinaryOp::Add } else { BinaryOp::Sub };
        self.emit(Instruction::Binary {
            dest: new_val,
            op,
            left: old_val,
            right: one,
            ty: result_ty.clone(),
        });
        self.var_types.insert(new_val.0, result_ty);

        // Store new value back to the variable
        self.emit(Instruction::Store {
            ptr,
            value: new_val,
        });

        // Return appropriate value: new_val for prefix, old_val for postfix
        Ok(if is_prefix { new_val } else { old_val })
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
                self.var_types.insert(dest.0, IrType::Void);
                return Ok(dest);
            }

            // Look up function return type if available
            let ret_ty = self.get_function_return_type(name);

            // Regular function call
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

        // Method call or indirect call
        if let ExprKind::Member { object, property } = &callee.kind {
            // Get object type to find class name
            let obj_type = self.infer_expr_type(object);
            let obj_var = self.build_expr(object)?;

            // Check for built-in array methods
            let is_array = match &obj_type {
                IrType::Array(_, _) => true,
                IrType::Ptr(inner) => matches!(inner.as_ref(), IrType::Array(_, _) | IrType::Void),
                _ => false,
            };

            if is_array {
                // Handle built-in array methods
                match property.as_str() {
                    "ألحق" | "push" | "أضف" | "add" => {
                        // Array push/append
                        if let Some(value_var) = arg_vars.first() {
                            // Get element type from array type or argument type
                            let elem_ty = match &obj_type {
                                IrType::Array(inner, _) => (**inner).clone(),
                                IrType::Ptr(inner) => match inner.as_ref() {
                                    IrType::Array(elem, _) => (**elem).clone(),
                                    _ => self.var_types.get(&value_var.0).cloned().unwrap_or(IrType::Int),
                                },
                                _ => self.var_types.get(&value_var.0).cloned().unwrap_or(IrType::Int),
                            };
                            self.emit(Instruction::ArrayPush {
                                array: obj_var,
                                value: *value_var,
                                elem_ty,
                            });
                            // Push returns the array for chaining
                            self.var_types.insert(obj_var.0, obj_type);
                            return Ok(obj_var);
                        }
                    }
                    "طول" | "length" | "len" => {
                        // Array length
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

            // Extract class ID from type (handle both Struct and Ptr(Struct))
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

            // Look up method return type
            let full_method_name = format!("{}::{}", class_id.0, property);
            let ret_ty = self.method_return_types.get(&full_method_name)
                .cloned()
                .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));

            // Method call
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
            });
            self.var_types.insert(dest.0, ret_ty);
            return Ok(dest);
        }

        // Generic indirect call
        let callee_var = self.build_expr(callee)?;
        let ret_ty = IrType::Ptr(Box::new(IrType::Void));
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

    /// Get the return type of a function by name
    fn get_function_return_type(&self, name: &str) -> IrType {
        // First check the pre-collected function signatures (for recursive/forward calls)
        if let Some(ret_ty) = self.function_return_types.get(name) {
            return ret_ty.clone();
        }

        // Then check module functions (for already-built functions)
        for func in &self.module.functions {
            if func.name == name || func.id.0 == name {
                return func.return_type.clone();
            }
        }

        // Default to void pointer for unknown functions
        IrType::Ptr(Box::new(IrType::Void))
    }

    /// Build IR for member access
    fn build_member(&mut self, object: &Expr, property: &str) -> Result<VarId> {
        let obj_type = self.infer_expr_type(object);
        let obj_var = self.build_expr(object)?;
        let dest = self.new_var();

        // Extract class ID from type (handle both Struct and Ptr(Struct))
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

        // Get field info from class fields if available
        let (field_ty, field_index, class_id) = if let Some(class_id) = class_id_opt {
            if let Some((idx, ty)) = self.get_field_info(&class_id.0, property) {
                (ty, idx, class_id)
            } else {
                (IrType::Ptr(Box::new(IrType::Void)), 0, class_id)
            }
        } else {
            (IrType::Ptr(Box::new(IrType::Void)), 0, ClassId("".to_string()))
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

    /// Get field type from class definition
    fn get_field_type(&self, class_name: &str, field_name: &str) -> IrType {
        if let Some(fields) = self.class_fields.get(class_name) {
            for (name, ty) in fields {
                if name == field_name {
                    return ty.clone();
                }
            }
        }
        IrType::Ptr(Box::new(IrType::Void))
    }

    /// Get field info (index, type) from class definition
    fn get_field_info(&self, class_name: &str, field_name: &str) -> Option<(u32, IrType)> {
        if let Some(fields) = self.class_fields.get(class_name) {
            for (idx, (name, ty)) in fields.iter().enumerate() {
                if name == field_name {
                    return Some((idx as u32, ty.clone()));
                }
            }
        }
        None
    }

    /// Build IR for index access
    fn build_index(&mut self, object: &Expr, index: &Expr) -> Result<VarId> {
        let obj_type = self.infer_expr_type(object);
        let obj_var = self.build_expr(object)?;
        let idx_var = self.build_expr(index)?;
        let dest = self.new_var();

        // Get element type from array type if available
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
                } else {
                    return Err(IrError::new(
                        format!("Cannot assign to undefined variable: '{}'", name),
                        format!("لا يمكن التعيين لمتغير غير معرّف: '{}'", name),
                    ));
                }
            }
            ExprKind::Member { object, property } => {
                // Get object type to find class info
                let obj_type = self.infer_expr_type(object);
                let obj_var = self.build_expr(object)?;

                // Extract class ID from type (handle both Struct and Ptr(Struct))
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

                // Look up class and field index
                let (class_id, field_index) = if let Some(class_id) = class_id_opt {
                    let index = self.get_field_info(&class_id.0, property)
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
                return Err(IrError::new(
                    "Unsupported assignment target",
                    "هدف التعيين غير مدعوم",
                ));
            }
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

        // Store back based on target type
        match &target.kind {
            ExprKind::Identifier(name) => {
                if let Some(ptr) = self.lookup_var(name) {
                    self.emit(Instruction::Store { ptr, value: result });
                } else {
                    return Err(IrError::new(
                        format!("Cannot assign to undefined variable: '{}'", name),
                        format!("لا يمكن التعيين لمتغير غير معرّف: '{}'", name),
                    ));
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
                return Err(IrError::new(
                    "Unsupported compound assignment target",
                    "هدف التعيين المركب غير مدعوم",
                ));
            }
        }

        Ok(result)
    }

    /// Build IR for array literal
    fn build_array(&mut self, elements: &[Expr]) -> Result<VarId> {
        // Infer element type from first element
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

    /// Build IR for object literal
    fn build_object(&mut self, fields: &[(String, Expr)]) -> Result<VarId> {
        // Objects are represented as anonymous structs for now
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

        let class_id = ClassId(class_name.clone());

        // Allocate object
        let dest = self.new_var();
        self.emit(Instruction::NewObject {
            dest,
            class: class_id.clone(),
        });

        // Track the object type
        self.var_types.insert(dest.0, IrType::Struct(class_id));

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
            Err(IrError::new(
                "'this' can only be used inside a method",
                "'هذا' يمكن استخدامه فقط داخل دالة",
            ))
        }
    }

    /// Build IR for 'super' reference
    fn build_super(&mut self) -> Result<VarId> {
        if let Some(var) = self.lookup_var("هذا").or_else(|| self.lookup_var("this")) {
            Ok(var)
        } else {
            Err(IrError::new(
                "'super' can only be used inside a method",
                "'الأصل' يمكن استخدامه فقط داخل دالة",
            ))
        }
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
