//! IR Instruction Definitions
//!
//! This module defines the intermediate representation (IR) instructions used
//! by the Tarqeem compiler. The IR is a three-address code in SSA (Static Single
//! Assignment) form, designed to be lowered to LLVM IR or interpreted directly.

use std::fmt;

/// Unique identifier for a virtual register (SSA value)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(pub u32);

impl fmt::Display for VarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}

/// Unique identifier for a basic block
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

/// Unique identifier for a function
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FuncId(pub String);

impl fmt::Display for FuncId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

/// Unique identifier for a class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassId(pub String);

impl fmt::Display for ClassId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%class.{}", self.0)
    }
}

/// Unique identifier for a field within a class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldId {
    pub class: ClassId,
    pub name: String,
    pub index: u32,
}

impl fmt::Display for FieldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.class, self.name)
    }
}

/// Unique identifier for a method within a class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodId {
    pub class: ClassId,
    pub name: String,
}

impl fmt::Display for MethodId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.class, self.name)
    }
}

/// IR type representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrType {
    /// Void type (no value)
    Void,
    /// Boolean (1-bit integer)
    Bool,
    /// 64-bit signed integer
    Int,
    /// 64-bit floating point
    Float,
    /// Pointer to string data
    String,
    /// Pointer to a type
    Ptr(Box<IrType>),
    /// Fixed-size array
    Array(Box<IrType>, usize),
    /// Function type
    Function {
        params: Vec<IrType>,
        ret: Box<IrType>,
    },
    /// Struct/class type
    Struct(ClassId),
}

impl fmt::Display for IrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrType::Void => write!(f, "void"),
            IrType::Bool => write!(f, "bool"),
            IrType::Int => write!(f, "i64"),
            IrType::Float => write!(f, "f64"),
            IrType::String => write!(f, "str"),
            IrType::Ptr(inner) => write!(f, "*{}", inner),
            IrType::Array(elem, size) => write!(f, "[{} x {}]", size, elem),
            IrType::Function { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            IrType::Struct(class) => write!(f, "{}", class),
        }
    }
}

/// Constant values in IR
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    /// Null pointer
    Null,
    /// Boolean constant
    Bool(bool),
    /// Integer constant
    Int(i64),
    /// Float constant
    Float(f64),
    /// String constant (index into string table)
    String(u32),
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Null => write!(f, "null"),
            Constant::Bool(b) => write!(f, "{}", b),
            Constant::Int(i) => write!(f, "{}", i),
            Constant::Float(fl) => write!(f, "{:.6}", fl),
            Constant::String(idx) => write!(f, "str#{}", idx),
        }
    }
}

/// Binary operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,

    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Logical
    And,
    Or,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "add"),
            BinaryOp::Sub => write!(f, "sub"),
            BinaryOp::Mul => write!(f, "mul"),
            BinaryOp::Div => write!(f, "div"),
            BinaryOp::Mod => write!(f, "mod"),
            BinaryOp::Pow => write!(f, "pow"),
            BinaryOp::Eq => write!(f, "eq"),
            BinaryOp::Ne => write!(f, "ne"),
            BinaryOp::Lt => write!(f, "lt"),
            BinaryOp::Le => write!(f, "le"),
            BinaryOp::Gt => write!(f, "gt"),
            BinaryOp::Ge => write!(f, "ge"),
            BinaryOp::And => write!(f, "and"),
            BinaryOp::Or => write!(f, "or"),
            BinaryOp::BitAnd => write!(f, "bitand"),
            BinaryOp::BitOr => write!(f, "bitor"),
            BinaryOp::BitXor => write!(f, "bitxor"),
            BinaryOp::Shl => write!(f, "shl"),
            BinaryOp::Shr => write!(f, "shr"),
        }
    }
}

/// Unary operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    /// Arithmetic negation
    Neg,
    /// Logical not
    Not,
    /// Bitwise not
    BitNot,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Neg => write!(f, "neg"),
            UnaryOp::Not => write!(f, "not"),
            UnaryOp::BitNot => write!(f, "bitnot"),
        }
    }
}

/// IR Instructions
///
/// Each instruction operates on virtual registers (VarId) and produces
/// a result in SSA form.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // ==================== Constants ====================
    /// Load a constant value into a register
    /// dest = const value
    Const {
        dest: VarId,
        value: Constant,
        ty: IrType,
    },

    // ==================== Arithmetic ====================
    /// Binary operation: dest = left op right
    Binary {
        dest: VarId,
        op: BinaryOp,
        left: VarId,
        right: VarId,
        ty: IrType,
    },

    /// Unary operation: dest = op operand
    Unary {
        dest: VarId,
        op: UnaryOp,
        operand: VarId,
        ty: IrType,
    },

    // ==================== Type Conversion ====================
    /// Convert integer to float
    IntToFloat { dest: VarId, src: VarId },

    /// Convert float to integer
    FloatToInt { dest: VarId, src: VarId },

    /// Convert any value to string
    ToString { dest: VarId, src: VarId },

    /// Bitcast (reinterpret bits)
    Bitcast {
        dest: VarId,
        src: VarId,
        to_ty: IrType,
    },

    // ==================== Memory ====================
    /// Allocate stack space for a local variable
    /// dest = alloca ty
    Alloca { dest: VarId, ty: IrType },

    /// Load a value from memory
    /// dest = load ptr
    Load { dest: VarId, ptr: VarId, ty: IrType },

    /// Store a value to memory
    /// store value -> ptr
    Store { ptr: VarId, value: VarId },

    /// Load a value from a global variable
    /// dest = global_load @name
    GlobalLoad {
        dest: VarId,
        name: String,
        ty: IrType,
    },

    /// Store a value to a global variable
    /// global_store @name, value
    GlobalStore { name: String, value: VarId },

    /// Get pointer to array element
    /// dest = gep ptr, index
    GetElementPtr {
        dest: VarId,
        ptr: VarId,
        index: VarId,
        elem_ty: IrType,
    },

    // ==================== Control Flow ====================
    /// Unconditional jump
    Jump { target: BlockId },

    /// Conditional branch
    /// if cond goto then_block else goto else_block
    Branch {
        cond: VarId,
        then_block: BlockId,
        else_block: BlockId,
    },

    /// Return from function
    Return { value: Option<VarId> },

    // ==================== Function Calls ====================
    /// Call a function
    /// dest = call func(args...)
    Call {
        dest: Option<VarId>,
        func: FuncId,
        args: Vec<VarId>,
        ret_ty: IrType,
    },

    /// Call an indirect function (function pointer)
    CallIndirect {
        dest: Option<VarId>,
        func_ptr: VarId,
        args: Vec<VarId>,
        ret_ty: IrType,
    },

    // ==================== Objects ====================
    /// Allocate a new object
    /// dest = new class
    NewObject { dest: VarId, class: ClassId },

    /// Get field from object
    /// dest = obj.field
    GetField {
        dest: VarId,
        object: VarId,
        field: FieldId,
        ty: IrType,
    },

    /// Set field on object
    /// obj.field = value
    SetField {
        object: VarId,
        field: FieldId,
        value: VarId,
    },

    /// Call a method on an object
    /// dest = obj.method(args...)
    CallMethod {
        dest: Option<VarId>,
        object: VarId,
        method: MethodId,
        args: Vec<VarId>,
        ret_ty: IrType,
    },

    /// Call virtual method (through vtable)
    CallVirtual {
        dest: Option<VarId>,
        object: VarId,
        method_index: u32,
        args: Vec<VarId>,
        ret_ty: IrType,
    },

    // ==================== Arrays ====================
    /// Create a new array
    /// dest = array [elements...]
    NewArray {
        dest: VarId,
        elem_ty: IrType,
        elements: Vec<VarId>,
    },

    /// Get array length
    /// dest = len(array)
    ArrayLen { dest: VarId, array: VarId },

    /// Get array element
    /// dest = array[index]
    ArrayGet {
        dest: VarId,
        array: VarId,
        index: VarId,
        elem_ty: IrType,
    },

    /// Set array element
    /// array[index] = value
    ArraySet {
        array: VarId,
        index: VarId,
        value: VarId,
    },

    /// Push element to array (append)
    /// array.push(value)
    ArrayPush {
        array: VarId,
        value: VarId,
        elem_ty: IrType,
    },

    // ==================== Strings ====================
    /// Concatenate strings
    /// dest = left + right
    StringConcat {
        dest: VarId,
        left: VarId,
        right: VarId,
    },

    // ==================== Exception Handling ====================
    /// Begin try block (marks exception handling region)
    TryBegin { catch_block: BlockId },

    /// End try block
    TryEnd,

    /// Throw an exception
    Throw { exception: VarId },

    /// Get current exception in catch block
    GetException { dest: VarId },

    // ==================== Phi Functions (SSA) ====================
    /// Phi function for SSA form
    /// dest = phi [val1, block1], [val2, block2], ...
    Phi {
        dest: VarId,
        ty: IrType,
        incoming: Vec<(VarId, BlockId)>,
    },

    // ==================== Debug/Intrinsics ====================
    /// Print a value (for debugging/اطبع)
    Print { value: VarId },

    /// No operation (placeholder)
    Nop,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Const { dest, value, ty } => {
                write!(f, "{}: {} = const {}", dest, ty, value)
            }
            Instruction::Binary {
                dest,
                op,
                left,
                right,
                ty,
            } => {
                write!(f, "{}: {} = {} {}, {}", dest, ty, op, left, right)
            }
            Instruction::Unary {
                dest,
                op,
                operand,
                ty,
            } => {
                write!(f, "{}: {} = {} {}", dest, ty, op, operand)
            }
            Instruction::IntToFloat { dest, src } => {
                write!(f, "{}: f64 = int_to_float {}", dest, src)
            }
            Instruction::FloatToInt { dest, src } => {
                write!(f, "{}: i64 = float_to_int {}", dest, src)
            }
            Instruction::ToString { dest, src } => {
                write!(f, "{}: str = to_string {}", dest, src)
            }
            Instruction::Bitcast { dest, src, to_ty } => {
                write!(f, "{}: {} = bitcast {}", dest, to_ty, src)
            }
            Instruction::Alloca { dest, ty } => {
                write!(f, "{}: *{} = alloca {}", dest, ty, ty)
            }
            Instruction::Load { dest, ptr, ty } => {
                write!(f, "{}: {} = load {}", dest, ty, ptr)
            }
            Instruction::Store { ptr, value } => {
                write!(f, "store {}, {}", value, ptr)
            }
            Instruction::GlobalLoad { dest, name, ty } => {
                write!(f, "{}: {} = global_load @{}", dest, ty, name)
            }
            Instruction::GlobalStore { name, value } => {
                write!(f, "global_store @{}, {}", name, value)
            }
            Instruction::GetElementPtr {
                dest,
                ptr,
                index,
                elem_ty,
            } => {
                write!(f, "{}: *{} = gep {}, {}", dest, elem_ty, ptr, index)
            }
            Instruction::Jump { target } => {
                write!(f, "jump {}", target)
            }
            Instruction::Branch {
                cond,
                then_block,
                else_block,
            } => {
                write!(f, "branch {}, {}, {}", cond, then_block, else_block)
            }
            Instruction::Return { value } => match value {
                Some(v) => write!(f, "ret {}", v),
                None => write!(f, "ret void"),
            },
            Instruction::Call {
                dest,
                func,
                args,
                ret_ty,
            } => {
                if let Some(d) = dest {
                    write!(f, "{}: {} = ", d, ret_ty)?;
                }
                write!(f, "call {}(", func)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Instruction::CallIndirect {
                dest,
                func_ptr,
                args,
                ret_ty,
            } => {
                if let Some(d) = dest {
                    write!(f, "{}: {} = ", d, ret_ty)?;
                }
                write!(f, "call_indirect {}(", func_ptr)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Instruction::NewObject { dest, class } => {
                write!(f, "{}: {} = new {}", dest, class, class)
            }
            Instruction::GetField {
                dest,
                object,
                field,
                ty,
            } => {
                write!(f, "{}: {} = getfield {}, {}", dest, ty, object, field)
            }
            Instruction::SetField {
                object,
                field,
                value,
            } => {
                write!(f, "setfield {}, {}, {}", object, field, value)
            }
            Instruction::CallMethod {
                dest,
                object,
                method,
                args,
                ret_ty,
            } => {
                if let Some(d) = dest {
                    write!(f, "{}: {} = ", d, ret_ty)?;
                }
                write!(f, "call_method {}, {}(", object, method)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Instruction::CallVirtual {
                dest,
                object,
                method_index,
                args,
                ret_ty,
            } => {
                if let Some(d) = dest {
                    write!(f, "{}: {} = ", d, ret_ty)?;
                }
                write!(f, "call_virtual {}, #{}(", object, method_index)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Instruction::NewArray {
                dest,
                elem_ty,
                elements,
            } => {
                write!(f, "{}: [{}] = array [", dest, elem_ty)?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            Instruction::ArrayLen { dest, array } => {
                write!(f, "{}: i64 = array_len {}", dest, array)
            }
            Instruction::ArrayGet {
                dest,
                array,
                index,
                elem_ty,
            } => {
                write!(f, "{}: {} = array_get {}, {}", dest, elem_ty, array, index)
            }
            Instruction::ArraySet {
                array,
                index,
                value,
            } => {
                write!(f, "array_set {}, {}, {}", array, index, value)
            }
            Instruction::ArrayPush {
                array,
                value,
                elem_ty,
            } => {
                write!(f, "array_push {}, {}: {}", array, value, elem_ty)
            }
            Instruction::StringConcat { dest, left, right } => {
                write!(f, "{}: str = string_concat {}, {}", dest, left, right)
            }
            Instruction::TryBegin { catch_block } => {
                write!(f, "try_begin catch={}", catch_block)
            }
            Instruction::TryEnd => {
                write!(f, "try_end")
            }
            Instruction::Throw { exception } => {
                write!(f, "throw {}", exception)
            }
            Instruction::GetException { dest } => {
                write!(f, "{}: exception = get_exception", dest)
            }
            Instruction::Phi { dest, ty, incoming } => {
                write!(f, "{}: {} = phi ", dest, ty)?;
                for (i, (val, block)) in incoming.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "[{}, {}]", val, block)?;
                }
                Ok(())
            }
            Instruction::Print { value } => {
                write!(f, "print {}", value)
            }
            Instruction::Nop => {
                write!(f, "nop")
            }
        }
    }
}

/// A basic block containing a sequence of instructions
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Block identifier
    pub id: BlockId,
    /// Optional label (for debugging)
    pub label: Option<String>,
    /// Instructions in this block
    pub instructions: Vec<Instruction>,
    /// Predecessor blocks
    pub predecessors: Vec<BlockId>,
    /// Successor blocks (derived from terminator)
    pub successors: Vec<BlockId>,
}

impl BasicBlock {
    /// Create a new basic block
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            label: None,
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
        }
    }

    /// Create a new basic block with a label
    pub fn with_label(id: BlockId, label: String) -> Self {
        Self {
            id,
            label: Some(label),
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
        }
    }

    /// Check if this block has a terminator instruction
    pub fn has_terminator(&self) -> bool {
        self.instructions.last().map_or(false, |inst| {
            matches!(
                inst,
                Instruction::Jump { .. }
                    | Instruction::Branch { .. }
                    | Instruction::Return { .. }
                    | Instruction::Throw { .. }
            )
        })
    }

    /// Get the terminator instruction, if any
    pub fn terminator(&self) -> Option<&Instruction> {
        self.instructions.last().filter(|inst| {
            matches!(
                inst,
                Instruction::Jump { .. }
                    | Instruction::Branch { .. }
                    | Instruction::Return { .. }
                    | Instruction::Throw { .. }
            )
        })
    }
}

impl fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(label) = &self.label {
            writeln!(f, "{}:  ; {}", self.id, label)?;
        } else {
            writeln!(f, "{}:", self.id)?;
        }

        for inst in &self.instructions {
            writeln!(f, "    {}", inst)?;
        }

        Ok(())
    }
}

/// A function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    pub id: VarId,
    pub name: String,
    pub ty: IrType,
}

/// IR representation of a function
#[derive(Debug, Clone)]
pub struct Function {
    /// Function identifier
    pub id: FuncId,
    /// Original name (may be Arabic)
    pub name: String,
    /// Parameters
    pub params: Vec<Parameter>,
    /// Return type
    pub return_type: IrType,
    /// Basic blocks (first block is entry)
    pub blocks: Vec<BasicBlock>,
    /// Local variable count (for SSA numbering)
    pub var_counter: u32,
    /// Block counter
    pub block_counter: u32,
    /// Is this an async function?
    pub is_async: bool,
}

impl Function {
    /// Create a new function
    pub fn new(id: FuncId, name: String, params: Vec<Parameter>, return_type: IrType) -> Self {
        Self {
            id,
            name,
            params,
            return_type,
            blocks: Vec::new(),
            var_counter: 0,
            block_counter: 0,
            is_async: false,
        }
    }

    /// Get the entry block
    pub fn entry_block(&self) -> Option<&BasicBlock> {
        self.blocks.first()
    }

    /// Get a mutable reference to the entry block
    pub fn entry_block_mut(&mut self) -> Option<&mut BasicBlock> {
        self.blocks.first_mut()
    }

    /// Get a block by ID
    pub fn get_block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.iter().find(|b| b.id == id)
    }

    /// Get a mutable block by ID
    pub fn get_block_mut(&mut self, id: BlockId) -> Option<&mut BasicBlock> {
        self.blocks.iter_mut().find(|b| b.id == id)
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fn {}(", self.id)?;
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", param.id, param.ty)?;
        }
        writeln!(f, ") -> {} {{", self.return_type)?;

        for block in &self.blocks {
            write!(f, "{}", block)?;
        }

        writeln!(f, "}}")
    }
}

/// IR representation of a class
#[derive(Debug, Clone)]
pub struct Class {
    /// Class identifier
    pub id: ClassId,
    /// Original name
    pub name: String,
    /// Parent class (for inheritance)
    pub parent: Option<ClassId>,
    /// Implemented interfaces
    pub interfaces: Vec<ClassId>,
    /// Fields with their types
    pub fields: Vec<(String, IrType)>,
    /// Methods
    pub methods: Vec<FuncId>,
    /// Virtual table (method name -> vtable index)
    pub vtable: Vec<MethodId>,
}

impl Class {
    /// Create a new class
    pub fn new(id: ClassId, name: String) -> Self {
        Self {
            id,
            name,
            parent: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            vtable: Vec::new(),
        }
    }

    /// Get field index by name
    pub fn field_index(&self, name: &str) -> Option<u32> {
        self.fields
            .iter()
            .position(|(n, _)| n == name)
            .map(|i| i as u32)
    }
}

/// Global string table for string literals
#[derive(Debug, Clone, Default)]
pub struct StringTable {
    strings: Vec<String>,
}

impl StringTable {
    /// Create a new string table
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
        }
    }

    /// Add a string and return its index
    pub fn add(&mut self, s: String) -> u32 {
        // Check if string already exists
        if let Some(idx) = self.strings.iter().position(|x| x == &s) {
            return idx as u32;
        }
        let idx = self.strings.len() as u32;
        self.strings.push(s);
        idx
    }

    /// Get a string by index
    pub fn get(&self, idx: u32) -> Option<&str> {
        self.strings.get(idx as usize).map(|s| s.as_str())
    }

    /// Iterate over all strings
    pub fn iter(&self) -> impl Iterator<Item = (u32, &str)> {
        self.strings
            .iter()
            .enumerate()
            .map(|(i, s)| (i as u32, s.as_str()))
    }
}

/// The complete IR module (compilation unit)
#[derive(Debug, Clone)]
pub struct Module {
    /// Module name
    pub name: String,
    /// String literal table
    pub strings: StringTable,
    /// Classes defined in this module
    pub classes: Vec<Class>,
    /// Functions defined in this module
    pub functions: Vec<Function>,
    /// Global variables
    pub globals: Vec<(String, IrType, Option<Constant>)>,
}

impl Module {
    /// Create a new module
    pub fn new(name: String) -> Self {
        Self {
            name,
            strings: StringTable::new(),
            classes: Vec::new(),
            functions: Vec::new(),
            globals: Vec::new(),
        }
    }

    /// Get a function by ID
    pub fn get_function(&self, id: &FuncId) -> Option<&Function> {
        self.functions.iter().find(|f| &f.id == id)
    }

    /// Get a class by ID
    pub fn get_class(&self, id: &ClassId) -> Option<&Class> {
        self.classes.iter().find(|c| &c.id == id)
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "; Module: {}", self.name)?;
        writeln!(f)?;

        // Print string table
        if !self.strings.strings.is_empty() {
            writeln!(f, "; String table")?;
            for (idx, s) in self.strings.iter() {
                writeln!(f, ";   str#{} = {:?}", idx, s)?;
            }
            writeln!(f)?;
        }

        // Print classes
        for class in &self.classes {
            writeln!(f, "; Class: {}", class.name)?;
            writeln!(f, "struct {} {{", class.id)?;
            for (name, ty) in &class.fields {
                writeln!(f, "    {}: {}", name, ty)?;
            }
            writeln!(f, "}}")?;
            writeln!(f)?;
        }

        // Print globals
        for (name, ty, init) in &self.globals {
            if let Some(val) = init {
                writeln!(f, "global @{}: {} = {}", name, ty, val)?;
            } else {
                writeln!(f, "global @{}: {}", name, ty)?;
            }
        }
        if !self.globals.is_empty() {
            writeln!(f)?;
        }

        // Print functions
        for func in &self.functions {
            write!(f, "{}", func)?;
            writeln!(f)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_display() {
        let inst = Instruction::Const {
            dest: VarId(0),
            value: Constant::Int(42),
            ty: IrType::Int,
        };
        assert_eq!(format!("{}", inst), "%0: i64 = const 42");

        let inst = Instruction::Binary {
            dest: VarId(2),
            op: BinaryOp::Add,
            left: VarId(0),
            right: VarId(1),
            ty: IrType::Int,
        };
        assert_eq!(format!("{}", inst), "%2: i64 = add %0, %1");
    }

    #[test]
    fn test_basic_block() {
        let mut block = BasicBlock::new(BlockId(0));
        block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Int(42),
            ty: IrType::Int,
        });
        block.instructions.push(Instruction::Return {
            value: Some(VarId(0)),
        });

        assert!(block.has_terminator());
        assert!(matches!(
            block.terminator(),
            Some(Instruction::Return { .. })
        ));
    }

    #[test]
    fn test_function_display() {
        let mut func = Function::new(
            FuncId("main".to_string()),
            "main".to_string(),
            vec![],
            IrType::Int,
        );

        let mut entry = BasicBlock::with_label(BlockId(0), "entry".to_string());
        entry.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Int(0),
            ty: IrType::Int,
        });
        entry.instructions.push(Instruction::Return {
            value: Some(VarId(0)),
        });
        func.blocks.push(entry);

        let output = format!("{}", func);
        assert!(output.contains("fn @main()"));
        assert!(output.contains("const 0"));
        assert!(output.contains("ret %0"));
    }

    #[test]
    fn test_string_table() {
        let mut table = StringTable::new();
        let idx1 = table.add("hello".to_string());
        let idx2 = table.add("world".to_string());
        let idx3 = table.add("hello".to_string()); // Duplicate

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Should return existing index

        assert_eq!(table.get(0), Some("hello"));
        assert_eq!(table.get(1), Some("world"));
    }
}
