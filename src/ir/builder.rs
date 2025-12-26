//! IR Builder - Converts typed AST to IR
//!
//! This module provides the `IrBuilder` which walks the AST and generates
//! the intermediate representation (IR) for code generation.

use crate::parser::{
    Ast, BinaryOp as AstBinaryOp, Block, CatchClause, ClassMember, Expr, ExprKind, LambdaBody,
    Literal, MatchArm, Param, Pattern, PatternKind, Stmt, StmtKind, TypeAnnotation, TypeKind,
    UnaryOp as AstUnaryOp,
};

use super::{
    BasicBlock, BinaryOp, BlockId, Class, ClassId, Constant, EnumId, FieldId, FuncId, Function,
    Instruction, IrType, MethodId, Module, Parameter, UnaryOp, VarId, VariantId,
};

use std::collections::{HashMap, HashSet};

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

pub struct IrBuilder {
    module: Module,
    current_function: Option<Function>,
    current_block: BlockId,
    var_counter: u32,
    block_counter: u32,
    variables: HashMap<String, VarId>,
    var_types: HashMap<u32, IrType>,
    scope_stack: Vec<HashMap<String, VarId>>,
    loop_stack: Vec<(BlockId, BlockId)>, // (continue_block, break_block)
    class_fields: HashMap<String, Vec<(String, IrType)>>,
    method_return_types: HashMap<String, IrType>,
    property_getters: HashMap<String, (String, IrType)>, // "class::property" -> (getter_name, type)
    property_setters: HashMap<String, String>,           // "class::property" -> setter_name
    function_names: HashSet<String>,
    function_return_types: HashMap<String, IrType>,
    parameters: HashSet<u32>,
    global_constants: HashMap<String, (Constant, IrType)>,
    global_variables: HashSet<String>,
    global_var_types: HashMap<String, IrType>,
}

impl IrBuilder {
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
        }
    }

    pub fn build(mut self, ast: &Ast) -> Result<Module> {
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

        for stmt in &ast.statements {
            if let StmtKind::ClassDecl { name, members, .. } = &stmt.kind {
                self.collect_class(name, members)?;
            }
        }

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

        let mut has_top_level_code = false;
        for stmt in &ast.statements {
            match &stmt.kind {
                StmtKind::FuncDecl { .. }
                | StmtKind::ClassDecl { .. }
                | StmtKind::InterfaceDecl { .. } => {}
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

    fn collect_class(&mut self, name: &str, members: &[ClassMember]) -> Result<()> {
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

    fn collect_function_signature(
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

    fn begin_function(
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

    fn end_function(&mut self) -> Result<()> {
        if let Some(func) = self.current_function.take() {
            self.module.functions.push(func);
        }
        self.pop_scope();
        Ok(())
    }

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

    fn switch_to_block(&mut self, block_id: BlockId) {
        self.current_block = block_id;
    }

    fn new_var(&mut self) -> VarId {
        let id = VarId(self.var_counter);
        self.var_counter += 1;
        id
    }

    fn emit(&mut self, inst: Instruction) {
        if let Some(ref mut func) = self.current_function {
            if let Some(block) = func.get_block_mut(self.current_block) {
                block.instructions.push(inst);
            }
        }
    }

    fn push_scope(&mut self) {
        self.scope_stack.push(self.variables.clone());
    }

    fn pop_scope(&mut self) {
        if let Some(vars) = self.scope_stack.pop() {
            self.variables = vars;
        }
    }

    fn lookup_var(&self, name: &str) -> Option<VarId> {
        self.variables.get(name).copied()
    }

    fn add_string(&mut self, s: String) -> u32 {
        self.module.strings.add(s)
    }

    fn convert_type(&self, ty: &TypeAnnotation) -> IrType {
        match &ty.kind {
            TypeKind::Simple(name) => self.convert_simple_type(name),
            TypeKind::Array(inner) => IrType::Array(Box::new(self.convert_type(inner)), 0),
            TypeKind::Map(_key, _value) => IrType::Ptr(Box::new(IrType::Void)),
            TypeKind::Function {
                params,
                return_type,
            } => IrType::Function {
                params: params.iter().map(|p| self.convert_type(p)).collect(),
                ret: Box::new(self.convert_type(return_type)),
            },
            TypeKind::Generic { base, args } => match base.as_str() {
                "مصفوفة" | "array" | "Array" => {
                    if let Some(elem_type) = args.first() {
                        IrType::Array(Box::new(self.convert_type(elem_type)), 0)
                    } else {
                        IrType::Array(Box::new(IrType::Ptr(Box::new(IrType::Void))), 0)
                    }
                }
                "قاموس" | "map" | "Map" | "dict" | "Dict" => {
                    IrType::Ptr(Box::new(IrType::Void))
                }
                _ => self.convert_simple_type(base),
            },
            TypeKind::Optional(inner) => IrType::Ptr(Box::new(self.convert_type(inner))),
        }
    }

    fn convert_simple_type(&self, name: &str) -> IrType {
        match name {
            "عدد" | "int" => IrType::Int,
            "عدد_عشري" | "float" => IrType::Float,
            "نص" | "string" => IrType::String,
            "منطقي" | "bool" => IrType::Bool,
            "void" => IrType::Void, // فراغ eliminated - functions default to no return
            _ => IrType::Struct(ClassId(name.to_string())),
        }
    }

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
            SemanticType::Function {
                params,
                return_type,
            } => IrType::Function {
                params: params.iter().map(|p| self.semantic_to_ir_type(p)).collect(),
                ret: Box::new(self.semantic_to_ir_type(return_type)),
            },
            _ => IrType::Ptr(Box::new(IrType::Void)),
        }
    }

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
                    (Constant::Int(a), AstBinaryOp::Add, Constant::Int(b)) => {
                        Some(Constant::Int(a + b))
                    }
                    (Constant::Int(a), AstBinaryOp::Sub, Constant::Int(b)) => {
                        Some(Constant::Int(a - b))
                    }
                    (Constant::Int(a), AstBinaryOp::Mul, Constant::Int(b)) => {
                        Some(Constant::Int(a * b))
                    }
                    (Constant::Int(a), AstBinaryOp::Div, Constant::Int(b)) if b != 0 => {
                        Some(Constant::Int(a / b))
                    }
                    (Constant::Float(a), AstBinaryOp::Add, Constant::Float(b)) => {
                        Some(Constant::Float(a + b))
                    }
                    (Constant::Float(a), AstBinaryOp::Sub, Constant::Float(b)) => {
                        Some(Constant::Float(a - b))
                    }
                    (Constant::Float(a), AstBinaryOp::Mul, Constant::Float(b)) => {
                        Some(Constant::Float(a * b))
                    }
                    (Constant::Float(a), AstBinaryOp::Div, Constant::Float(b)) if b != 0.0 => {
                        Some(Constant::Float(a / b))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn const_to_type(&self, constant: &Constant) -> IrType {
        match constant {
            Constant::Int(_) => IrType::Int,
            Constant::Float(_) => IrType::Float,
            Constant::Bool(_) => IrType::Bool,
            Constant::String(_) => IrType::String,
            Constant::Null => IrType::Ptr(Box::new(IrType::Void)),
        }
    }

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
            StmtKind::EnumDecl { .. } => Ok(()),
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
            StmtKind::Export(inner) => self.build_stmt(inner),
            StmtKind::Expr(expr) => {
                self.build_expr(expr)?;
                Ok(())
            }
            StmtKind::Block(block) => self.build_block(block),
        }
    }

    fn build_var_decl(
        &mut self,
        name: &str,
        ty: Option<&TypeAnnotation>,
        init: Option<&Expr>,
    ) -> Result<()> {
        if self.global_variables.contains(name) {
            if let Some(init_expr) = init {
                if self.try_evaluate_const(init_expr).is_none() {
                    let value = self.build_expr(init_expr)?;
                    self.emit(Instruction::GlobalStore {
                        name: name.to_string(),
                        value,
                    });
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
            let value = self.build_expr(init_expr)?;
            self.emit(Instruction::Store { ptr, value });
        }

        self.variables.insert(name.to_string(), ptr);

        Ok(())
    }

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
                let elem_ty = if let Some(first) = elements.first() {
                    self.infer_expr_type(first)
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                };
                IrType::Array(Box::new(elem_ty), elements.len())
            }
            ExprKind::Binary { op, left, right } => match op {
                AstBinaryOp::Eq
                | AstBinaryOp::NotEq
                | AstBinaryOp::Lt
                | AstBinaryOp::LtEq
                | AstBinaryOp::Gt
                | AstBinaryOp::GtEq
                | AstBinaryOp::And
                | AstBinaryOp::Or => IrType::Bool,
                AstBinaryOp::Add => {
                    let left_ty = self.infer_expr_type(left);
                    let right_ty = self.infer_expr_type(right);
                    if matches!(left_ty, IrType::String) || matches!(right_ty, IrType::String) {
                        IrType::String
                    } else if matches!(left_ty, IrType::Float) || matches!(right_ty, IrType::Float)
                    {
                        IrType::Float
                    } else {
                        IrType::Int
                    }
                }
                AstBinaryOp::Sub | AstBinaryOp::Mul | AstBinaryOp::Div | AstBinaryOp::Mod => {
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
                AstUnaryOp::Neg
                | AstUnaryOp::PreInc
                | AstUnaryOp::PreDec
                | AstUnaryOp::PostInc
                | AstUnaryOp::PostDec => {
                    let operand_ty = self.infer_expr_type(operand);
                    match operand_ty {
                        IrType::Float => IrType::Float,
                        _ => IrType::Int,
                    }
                }
            },
            ExprKind::New { class, .. } => {
                if let ExprKind::Identifier(name) = &class.kind {
                    IrType::Ptr(Box::new(IrType::Struct(ClassId(name.clone()))))
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Identifier(name) => {
                if let Some(ptr) = self.lookup_var(name) {
                    self.var_types
                        .get(&ptr.0)
                        .cloned()
                        .unwrap_or(IrType::Ptr(Box::new(IrType::Void)))
                } else if let Some(global_ty) = self.global_var_types.get(name).cloned() {
                    global_ty
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Index { object, .. } => {
                let obj_ty = self.infer_expr_type(object);
                if let IrType::Array(elem, _) = obj_ty {
                    (*elem).clone()
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Member { object, property } => {
                let obj_ty = self.infer_expr_type(object);
                if let IrType::Struct(class_id) = obj_ty {
                    self.get_field_type(&class_id.0, property)
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Call { callee, .. } => {
                if let ExprKind::Identifier(name) = &callee.kind {
                    self.get_function_return_type(name)
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Ternary { then_expr, .. } => self.infer_expr_type(then_expr),
            ExprKind::This => {
                if let Some(var_id) = self.lookup_var("هذا").or_else(|| self.lookup_var("this"))
                {
                    self.var_types
                        .get(&var_id.0)
                        .cloned()
                        .unwrap_or(IrType::Ptr(Box::new(IrType::Void)))
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            ExprKind::Super => {
                if let Some(ref func) = self.current_function {
                    if let Some(idx) = func.name.find("::") {
                        let current_class_name = &func.name[..idx];
                        if let Some(parent_class_id) = self
                            .module
                            .classes
                            .iter()
                            .find(|c| c.name == current_class_name)
                            .and_then(|c| c.parent.as_ref())
                        {
                            return IrType::Ptr(Box::new(IrType::Struct(parent_class_id.clone())));
                        }
                    }
                }
                if let Some(var_id) = self.lookup_var("هذا").or_else(|| self.lookup_var("this"))
                {
                    self.var_types
                        .get(&var_id.0)
                        .cloned()
                        .unwrap_or(IrType::Ptr(Box::new(IrType::Void)))
                } else {
                    IrType::Ptr(Box::new(IrType::Void))
                }
            }
            _ => IrType::Ptr(Box::new(IrType::Void)),
        }
    }

    fn build_func_decl(
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

        let saved_function = self.current_function.take();
        let saved_block = self.current_block;
        let saved_var_counter = self.var_counter;
        let saved_block_counter = self.block_counter;
        let saved_variables = self.variables.clone();

        self.begin_function(name.to_string(), ir_params, ret_type)?;

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

        self.current_function = saved_function;
        self.current_block = saved_block;
        self.var_counter = saved_var_counter;
        self.block_counter = saved_block_counter;
        self.variables = saved_variables;

        Ok(())
    }

    fn build_class_decl(
        &mut self,
        name: &str,
        extends: Option<&String>,
        _implements: &[String],
        members: &[ClassMember],
    ) -> Result<()> {
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
                    is_async,
                    ..
                } => {
                    let mangled_name = format!("{}::{}", name, method_name);

                    let mut method_params: Vec<Parameter> = vec![Parameter {
                        id: VarId(0),
                        name: "هذا".to_string(), // "this" in Arabic
                        ty: IrType::Struct(ClassId(name.to_string())),
                    }];

                    for (i, p) in params.iter().enumerate() {
                        let ty =
                            p.ty.as_ref()
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

                    let saved_function = self.current_function.take();
                    let saved_variables = self.variables.clone();

                    self.begin_function(mangled_name, method_params, ret_type)?;

                    if let Some(ref mut func) = self.current_function {
                        func.is_async = *is_async;
                    }

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

                    self.current_function = saved_function;
                    self.variables = saved_variables;
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

                    let saved_function = self.current_function.take();
                    let saved_variables = self.variables.clone();

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

                    self.current_function = saved_function;
                    self.variables = saved_variables;
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
                        let getter_name = format!("{}::__احصل_{}", name, prop_name);
                        let getter_params = if *is_static {
                            vec![]
                        } else {
                            vec![Parameter {
                                id: VarId(0),
                                name: "هذا".to_string(),
                                ty: IrType::Struct(ClassId(name.to_string())),
                            }]
                        };

                        let saved_function = self.current_function.take();
                        let saved_variables = std::mem::take(&mut self.variables);

                        self.begin_function(getter_name, getter_params, prop_type.clone())?;

                        if !*is_static {
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
                            let this_var = VarId(0);
                            let backing_field = format!("_{}", prop_name);
                            let result = self.new_var();
                            self.emit(Instruction::GetField {
                                dest: result,
                                object: this_var,
                                field: FieldId {
                                    class: ClassId(name.to_string()),
                                    name: backing_field,
                                    index: 0,
                                },
                                ty: prop_type.clone(),
                            });
                            self.emit(Instruction::Return {
                                value: Some(result),
                            });
                        }

                        self.end_function()?;

                        self.current_function = saved_function;
                        self.variables = saved_variables;
                    }

                    let has_setter = accessors
                        .iter()
                        .any(|a| matches!(a, crate::parser::PropertyAccessor::Set { .. }));

                    if has_setter || accessors.is_empty() {
                        let setter_name = format!("{}::__عيّن_{}", name, prop_name);
                        let mut setter_params = if *is_static {
                            vec![]
                        } else {
                            vec![Parameter {
                                id: VarId(0),
                                name: "هذا".to_string(),
                                ty: IrType::Struct(ClassId(name.to_string())),
                            }]
                        };
                        setter_params.push(Parameter {
                            id: VarId(setter_params.len() as u32),
                            name: "قيمة".to_string(),
                            ty: prop_type.clone(),
                        });

                        let saved_function = self.current_function.take();
                        let saved_variables = std::mem::take(&mut self.variables);

                        self.begin_function(setter_name, setter_params, IrType::Void)?;

                        if !*is_static {
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
                                self.variables.insert(
                                    param_name.clone(),
                                    VarId(if *is_static { 0 } else { 1 }),
                                );
                                for stmt in &body.statements {
                                    self.build_stmt(stmt)?;
                                }
                                break;
                            }
                        }

                        if accessors.is_empty() {
                            let this_var = VarId(0);
                            let value_var = VarId(1);
                            let backing_field = format!("_{}", prop_name);
                            self.emit(Instruction::SetField {
                                object: this_var,
                                field: FieldId {
                                    class: ClassId(name.to_string()),
                                    name: backing_field,
                                    index: 0,
                                },
                                value: value_var,
                            });
                        }

                        self.emit(Instruction::Return { value: None });
                        self.end_function()?;

                        self.current_function = saved_function;
                        self.variables = saved_variables;
                    }
                }
            }
        }

        Ok(())
    }

    fn build_if(
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

    fn build_while(&mut self, condition: &Expr, body: &Block) -> Result<()> {
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

    fn build_do_while(&mut self, body: &Block, condition: &Expr) -> Result<()> {
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

    fn build_for(
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

    fn build_for_in(&mut self, variable: &str, iterable: &Expr, body: &Block) -> Result<()> {
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

    fn build_match(&mut self, expr: &Expr, arms: &[MatchArm]) -> Result<()> {
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

    /// Build pattern check and branch
    fn build_pattern_check(
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
            PatternKind::EnumVariant {
                variant_name, ..
            } => {
                // Check discriminant for enum variant match
                // Get discriminant from match value
                let disc = self.new_var();
                self.emit(Instruction::GetDiscriminant {
                    dest: disc,
                    value: match_val,
                });

                // Compare with expected discriminant (use hash of variant name)
                // Must match the calculation in build_enum_variant
                let expected = self.new_var();
                let disc_val = (variant_name.chars().map(|c| c as u32).sum::<u32>() % 256) as i64;
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

    /// Add pattern bindings to scope
    fn add_pattern_bindings(&mut self, pattern: &Pattern, match_val: VarId) -> Result<()> {
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
                // Extract variant fields and bind them
                for (i, binding) in bindings.iter().enumerate() {
                    let field_val = self.new_var();
                    self.emit(Instruction::GetVariantField {
                        dest: field_val,
                        value: match_val,
                        variant: VariantId {
                            enum_id: EnumId(enum_name.clone()),
                            name: variant_name.clone(),
                            discriminant: (variant_name.chars().map(|c| c as u32).sum::<u32>() % 256),
                        },
                        field_index: i as u32,
                        ty: IrType::Int, // Default to Int for now
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

    fn build_return(&mut self, expr: Option<&Expr>) -> Result<()> {
        let value = if let Some(e) = expr {
            Some(self.build_expr(e)?)
        } else {
            None
        };

        self.emit(Instruction::Return { value });
        Ok(())
    }

    fn build_break(&mut self) -> Result<()> {
        if let Some((_, exit_block)) = self.loop_stack.last() {
            self.emit(Instruction::Jump {
                target: *exit_block,
            });
            Ok(())
        } else {
            Err(IrError::new("break outside of loop", "كسر خارج حلقة"))
        }
    }

    fn build_continue(&mut self) -> Result<()> {
        if let Some((continue_block, _)) = self.loop_stack.last() {
            self.emit(Instruction::Jump {
                target: *continue_block,
            });
            Ok(())
        } else {
            Err(IrError::new("continue outside of loop", "استمر خارج حلقة"))
        }
    }

    fn build_try(
        &mut self,
        body: &Block,
        catch: Option<&CatchClause>,
        finally: Option<&Block>,
    ) -> Result<()> {
        let catch_block = self.new_block(Some("catch".to_string()));
        let finally_block = self.new_block(Some("finally".to_string()));
        let exit_block = self.new_block(Some("try.exit".to_string()));

        self.emit(Instruction::TryBegin { catch_block });

        self.push_scope();
        for stmt in &body.statements {
            self.build_stmt(stmt)?;
        }
        self.pop_scope();

        self.emit(Instruction::TryEnd);

        if finally.is_some() {
            self.emit(Instruction::Jump {
                target: finally_block,
            });
        } else {
            self.emit(Instruction::Jump { target: exit_block });
        }

        self.switch_to_block(catch_block);
        if let Some(catch_clause) = catch {
            self.push_scope();

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

    fn build_throw(&mut self, expr: &Expr) -> Result<()> {
        let exception = self.build_expr(expr)?;
        self.emit(Instruction::Throw { exception });
        Ok(())
    }

    fn build_block(&mut self, block: &Block) -> Result<()> {
        self.push_scope();
        for stmt in &block.statements {
            self.build_stmt(stmt)?;
        }
        self.pop_scope();
        Ok(())
    }

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

    fn build_enum_variant(
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

        // Create the variant ID
        // Use hash of variant name as discriminant for consistent matching
        let disc_val = (variant_name.chars().map(|c| c as u32).sum::<u32>()) % 256;
        let variant_id = VariantId {
            enum_id: EnumId(enum_name.to_string()),
            name: variant_name.to_string(),
            discriminant: disc_val,
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

        self.var_types.insert(dest.0, ty.clone());

        self.emit(Instruction::Const { dest, value, ty });
        Ok(dest)
    }

    fn build_identifier(&mut self, name: &str) -> Result<VarId> {
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
            let dest = self.new_var();
            self.emit(Instruction::Const {
                dest,
                value: Constant::Null, // Will be replaced with actual function pointer in codegen
                ty: IrType::Ptr(Box::new(IrType::Void)),
            });
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
            Err(IrError::new(
                format!("Undefined identifier: '{}'", name),
                format!("معرّف غير معرّف: '{}'", name),
            ))
        }
    }

    fn build_binary(&mut self, left: &Expr, op: AstBinaryOp, right: &Expr) -> Result<VarId> {
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

    fn build_unary(&mut self, op: AstUnaryOp, operand: &Expr) -> Result<VarId> {
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

    fn build_increment(
        &mut self,
        operand: &Expr,
        is_increment: bool,
        is_prefix: bool,
    ) -> Result<VarId> {
        let name = match &operand.kind {
            ExprKind::Identifier(name) => name.clone(),
            _ => {
                return Err(IrError::new(
                    "Increment/decrement requires a variable",
                    "الزيادة/النقصان تتطلب متغيراً",
                ))
            }
        };

        // Store the lookup result to avoid redundant lookups and unwrap() calls
        let local_ptr = self.lookup_var(&name);
        let is_global = self.global_variables.contains(&name);

        if local_ptr.is_none() && !is_global {
            return Err(IrError::new(
                format!("Cannot modify undefined variable '{}'", name),
                format!("لا يمكن تعديل متغير غير معرّف '{}'", name),
            ));
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

    fn build_call(&mut self, callee: &Expr, args: &[Expr]) -> Result<VarId> {
        if matches!(callee.kind, ExprKind::Super) {
            return self.build_super_constructor_call(args);
        }

        let arg_vars: Vec<VarId> = args
            .iter()
            .map(|a| self.build_expr(a))
            .collect::<Result<Vec<_>>>()?;

        if let ExprKind::Identifier(name) = &callee.kind {
            if name == "اطبع" || name == "print" {
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
            let obj_type = self.infer_expr_type(object);
            let obj_var = self.build_expr(object)?;

            let is_array = match &obj_type {
                IrType::Array(_, _) => true,
                IrType::Ptr(inner) => matches!(inner.as_ref(), IrType::Array(_, _) | IrType::Void),
                _ => false,
            };

            if is_array {
                match property.as_str() {
                    "ألحق" | "push" | "أضف" | "add" => {
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
                    "طول" | "length" | "len" => {
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
            });
            self.var_types.insert(dest.0, ret_ty);
            return Ok(dest);
        }

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

    fn get_function_return_type(&self, name: &str) -> IrType {
        if let Some(ret_ty) = self.function_return_types.get(name) {
            return ret_ty.clone();
        }

        for func in &self.module.functions {
            if func.name == name || func.id.0 == name {
                return func.return_type.clone();
            }
        }

        IrType::Ptr(Box::new(IrType::Void))
    }

    fn build_member(&mut self, object: &Expr, property: &str) -> Result<VarId> {
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

    fn build_index(&mut self, object: &Expr, index: &Expr) -> Result<VarId> {
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

    fn build_assignment(&mut self, target: &Expr, value: &Expr) -> Result<VarId> {
        let value_var = self.build_expr(value)?;

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
                    return Err(IrError::new(
                        format!("Cannot assign to undefined variable: '{}'", name),
                        format!("لا يمكن التعيين لمتغير غير معرّف: '{}'", name),
                    ));
                }
            }
            ExprKind::Member { object, property } => {
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
                return Err(IrError::new(
                    "Unsupported assignment target",
                    "هدف التعيين غير مدعوم",
                ));
            }
        }

        Ok(value_var)
    }

    fn build_compound_assignment(
        &mut self,
        target: &Expr,
        op: AstBinaryOp,
        value: &Expr,
    ) -> Result<VarId> {
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

    fn build_array(&mut self, elements: &[Expr]) -> Result<VarId> {
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

    fn build_object(&mut self, fields: &[(String, Expr)]) -> Result<VarId> {
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

    fn build_lambda(&mut self, params: &[Param], body: &LambdaBody) -> Result<VarId> {
        let lambda_name = format!("__lambda_{}", self.var_counter);

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

        let saved_function = self.current_function.take();
        let saved_variables = self.variables.clone();

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

        self.current_function = saved_function;
        self.variables = saved_variables;

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

    fn build_new(&mut self, class: &Expr, args: &[Expr]) -> Result<VarId> {
        let class_name = if let ExprKind::Identifier(name) = &class.kind {
            name.clone()
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

    fn build_await(&mut self, inner: &Expr) -> Result<VarId> {
        self.build_expr(inner)
    }

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

    fn build_super_constructor_call(&mut self, args: &[Expr]) -> Result<VarId> {
        let this_var = self
            .lookup_var("هذا")
            .or_else(|| self.lookup_var("this"))
            .ok_or_else(|| {
                IrError::new(
                    "'super()' can only be used inside a constructor",
                    "'الأصل()' يمكن استخدامه فقط داخل منشئ",
                )
            })?;

        let current_class_name = match &self.current_function {
            Some(func) => {
                if let Some(idx) = func.name.find("::") {
                    func.name[..idx].to_string()
                } else {
                    return Err(IrError::new(
                        "'super()' can only be used inside a class constructor",
                        "'الأصل()' يمكن استخدامه فقط داخل منشئ صنف",
                    ));
                }
            }
            None => {
                return Err(IrError::new(
                    "'super()' can only be used inside a function",
                    "'الأصل()' يمكن استخدامه فقط داخل دالة",
                ));
            }
        };

        let parent_class_name = self
            .module
            .classes
            .iter()
            .find(|c| c.name == current_class_name)
            .and_then(|c| c.parent.as_ref())
            .map(|p| p.0.clone())
            .ok_or_else(|| {
                IrError::new(
                    format!("Class '{}' has no parent class", current_class_name),
                    format!("الصنف '{}' ليس له صنف أب", current_class_name),
                )
            })?;

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
}
