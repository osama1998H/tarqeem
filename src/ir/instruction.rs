//! IR Instruction Definitions
//!
//! This module defines the intermediate representation (IR) instructions used
//! by the Tarqeem compiler. The IR is a three-address code in SSA (Static Single
//! Assignment) form, designed to be lowered to LLVM IR or interpreted directly.

use std::fmt;

use crate::error::codes::{ErrorCode, ERR_UNTYPED_LAMBDA_PARAM};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(pub u32);

impl fmt::Display for VarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FuncId(pub String);

impl fmt::Display for FuncId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassId(pub String);

impl fmt::Display for ClassId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%class.{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumId(pub String);

impl fmt::Display for EnumId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%enum.{}", self.0)
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantId {
    pub enum_id: EnumId,
    pub name: String,
    pub discriminant: u32,
}

impl fmt::Display for VariantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.enum_id, self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrType {
    Void,
    Bool,
    Int,
    Float,
    String,
    Ptr(Box<IrType>),
    Array(Box<IrType>, usize),
    Function {
        params: Vec<IrType>,
        ret: Box<IrType>,
    },
    Struct(ClassId),
    Enum(EnumId),
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
            IrType::Enum(enum_id) => write!(f, "{}", enum_id),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(u32), // index into string table
    /// A reference to a function by name — a bare function *value*, mirroring
    /// the interpreter's `Value::Function(String)` exactly. This is what a
    /// lambda expression (or a named function used as a value) evaluates to,
    /// consumed by `Instruction::CallIndirect` (see issue #180).
    Function(String),
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Null => write!(f, "null"),
            Constant::Bool(b) => write!(f, "{}", b),
            Constant::Int(i) => write!(f, "{}", i),
            Constant::Float(fl) => write!(f, "{:.6}", fl),
            Constant::String(idx) => write!(f, "str#{}", idx),
            Constant::Function(name) => write!(f, "@{}", name),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    Not,
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

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Const {
        dest: VarId,
        value: Constant,
        ty: IrType,
    },

    Binary {
        dest: VarId,
        op: BinaryOp,
        left: VarId,
        right: VarId,
        ty: IrType,
    },

    Unary {
        dest: VarId,
        op: UnaryOp,
        operand: VarId,
        ty: IrType,
    },

    IntToFloat {
        dest: VarId,
        src: VarId,
    },
    FloatToInt {
        dest: VarId,
        src: VarId,
    },
    ToString {
        dest: VarId,
        src: VarId,
    },
    Bitcast {
        dest: VarId,
        src: VarId,
        to_ty: IrType,
    },

    Alloca {
        dest: VarId,
        ty: IrType,
    },

    Load {
        dest: VarId,
        ptr: VarId,
        ty: IrType,
    },

    Store {
        ptr: VarId,
        value: VarId,
    },

    GlobalLoad {
        dest: VarId,
        name: String,
        ty: IrType,
    },
    GlobalStore {
        name: String,
        value: VarId,
    },

    GetElementPtr {
        dest: VarId,
        ptr: VarId,
        index: VarId,
        elem_ty: IrType,
    },

    Jump {
        target: BlockId,
    },

    Branch {
        cond: VarId,
        then_block: BlockId,
        else_block: BlockId,
    },

    Return {
        value: Option<VarId>,
    },

    Call {
        dest: Option<VarId>,
        func: FuncId,
        args: Vec<VarId>,
        ret_ty: IrType,
    },

    CallIndirect {
        dest: Option<VarId>,
        func_ptr: VarId,
        args: Vec<VarId>,
        ret_ty: IrType,
    },

    NewObject {
        dest: VarId,
        class: ClassId,
    },

    GetField {
        dest: VarId,
        object: VarId,
        field: FieldId,
        ty: IrType,
    },

    SetField {
        object: VarId,
        field: FieldId,
        value: VarId,
    },

    CallMethod {
        dest: Option<VarId>,
        object: VarId,
        method: MethodId,
        args: Vec<VarId>,
        ret_ty: IrType,
        /// True for ordinary member calls (dispatch on the object's runtime
        /// class); false for `الأصل.method()` super calls, which must stay
        /// bound to `method.class` or an override calling its super would
        /// recurse into itself forever.
        virtual_dispatch: bool,
    },

    CallVirtual {
        dest: Option<VarId>,
        object: VarId,
        method_index: u32,
        args: Vec<VarId>,
        ret_ty: IrType,
    },

    NewArray {
        dest: VarId,
        elem_ty: IrType,
        elements: Vec<VarId>,
    },
    ArrayLen {
        dest: VarId,
        array: VarId,
    },
    ArrayGet {
        dest: VarId,
        array: VarId,
        index: VarId,
        elem_ty: IrType,
    },
    ArraySet {
        array: VarId,
        index: VarId,
        value: VarId,
    },
    ArrayPush {
        array: VarId,
        value: VarId,
        elem_ty: IrType,
    },

    StringConcat {
        dest: VarId,
        left: VarId,
        right: VarId,
    },

    TryBegin {
        catch_block: BlockId,
    },
    TryEnd,
    Throw {
        exception: VarId,
    },
    GetException {
        dest: VarId,
    },

    Phi {
        dest: VarId,
        ty: IrType,
        incoming: Vec<(VarId, BlockId)>,
    },

    Print {
        value: VarId,
    }, // اطبع

    /// Copy a value from one variable to another.
    /// Used by the inliner to transfer return values with proper type tracking.
    Copy {
        dest: VarId,
        src: VarId,
        ty: IrType,
    },

    /// Create a new enum variant value
    NewEnumVariant {
        dest: VarId,
        variant: VariantId,
        fields: Vec<VarId>,
    },

    /// Get the discriminant (tag) of an enum value
    GetDiscriminant {
        dest: VarId,
        value: VarId,
    },

    /// Get a field from an enum variant
    GetVariantField {
        dest: VarId,
        value: VarId,
        variant: VariantId,
        field_index: u32,
        ty: IrType,
    },

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
                virtual_dispatch,
            } => {
                if let Some(d) = dest {
                    write!(f, "{}: {} = ", d, ret_ty)?;
                }
                let mnemonic = if *virtual_dispatch {
                    "call_method_virtual"
                } else {
                    "call_method"
                };
                write!(f, "{} {}, {}(", mnemonic, object, method)?;
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
            Instruction::Copy { dest, src, ty } => {
                write!(f, "{}: {} = copy {}", dest, ty, src)
            }
            Instruction::NewEnumVariant {
                dest,
                variant,
                fields,
            } => {
                write!(f, "{} = new_enum_variant {}", dest, variant)?;
                if !fields.is_empty() {
                    write!(f, "(")?;
                    for (i, field) in fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", field)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Instruction::GetDiscriminant { dest, value } => {
                write!(f, "{} = get_discriminant {}", dest, value)
            }
            Instruction::GetVariantField {
                dest,
                value,
                variant,
                field_index,
                ty,
            } => {
                write!(
                    f,
                    "{}: {} = get_variant_field {} {} [{}]",
                    dest, ty, value, variant, field_index
                )
            }
            Instruction::Nop => {
                write!(f, "nop")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub label: Option<String>,
    pub instructions: Vec<Instruction>,
    pub predecessors: Vec<BlockId>,
    pub successors: Vec<BlockId>,
}

impl BasicBlock {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            label: None,
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
        }
    }

    pub fn with_label(id: BlockId, label: String) -> Self {
        Self {
            id,
            label: Some(label),
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
        }
    }

    pub fn has_terminator(&self) -> bool {
        self.instructions.last().is_some_and(|inst| {
            matches!(
                inst,
                Instruction::Jump { .. }
                    | Instruction::Branch { .. }
                    | Instruction::Return { .. }
                    | Instruction::Throw { .. }
            )
        })
    }

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

#[derive(Debug, Clone)]
pub struct Parameter {
    pub id: VarId,
    pub name: String,
    pub ty: IrType,
}

/// A function's refusal to be lowered to native code, carrying the diagnostic
/// verbatim so codegen reports the right advice for the right cause.
///
/// The two constructors differ in more than wording: an untyped parameter is
/// something the *user* can fix by annotating, while an unsupported construct
/// can only be worked around by choosing another backend.
#[derive(Debug, Clone)]
pub struct NativeBlock {
    pub message: String,
    pub code: String,
}

impl NativeBlock {
    /// A type native codegen cannot pick an ABI for. `detail` names what.
    pub fn untyped(detail: impl fmt::Display) -> Self {
        Self {
            message: format!(
                "الترجمة الأصلية غير مدعومة هنا بعد: {} — صرّح بالأنواع المحددة أو شغّل البرنامج بالمفسّر (tarqeem run)",
                detail
            ),
            code: ERR_UNTYPED_LAMBDA_PARAM.to_string(),
        }
    }

    /// A construct with no native lowering at all, whatever its types. No
    /// annotation helps, so the advice is to pick a backend that supports it.
    pub fn unsupported(detail: impl fmt::Display, code: &ErrorCode) -> Self {
        Self {
            message: format!(
                "{} غير مدعوم في الترجمة الأصلية بعد — شغّل البرنامج بالمفسّر (tarqeem run) أو بالترجمة الفورية (tarqeem run --jit)",
                detail
            ),
            code: code.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub id: FuncId,
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: IrType,
    pub blocks: Vec<BasicBlock>,
    pub var_counter: u32,
    pub block_counter: u32,
    pub is_async: bool,
    /// Why this function cannot be lowered to native code, if it can't — an
    /// unannotated (or `أي`) parameter, returns whose types don't unify, or a
    /// construct with no native lowering such as `ارمِ`. A parameter's `IrType`
    /// alone cannot carry the first case: an unannotated parameter and an
    /// explicitly-annotated `قاموس<…>` both lower to `Ptr(Void)`, so the
    /// distinction has to be recorded when it is still known. `None` for every
    /// natively-lowerable function; the interpreter ignores this entirely
    /// (see ت٠٣٠١، ت٠٣٠٢، ت٠٣٠٣).
    pub native_block: Option<NativeBlock>,
}

impl Function {
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
            native_block: None,
        }
    }

    pub fn entry_block(&self) -> Option<&BasicBlock> {
        self.blocks.first()
    }

    pub fn entry_block_mut(&mut self) -> Option<&mut BasicBlock> {
        self.blocks.first_mut()
    }

    pub fn get_block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.iter().find(|b| b.id == id)
    }

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

#[derive(Debug, Clone)]
pub struct Class {
    pub id: ClassId,
    pub name: String,
    pub parent: Option<ClassId>,
    pub interfaces: Vec<ClassId>,
    pub fields: Vec<(String, IrType)>,
    pub methods: Vec<FuncId>,
    pub vtable: Vec<MethodId>,
}

impl Class {
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

    pub fn field_index(&self, name: &str) -> Option<u32> {
        self.fields
            .iter()
            .position(|(n, _)| n == name)
            .map(|i| i as u32)
    }
}

#[derive(Debug, Clone, Default)]
pub struct StringTable {
    strings: Vec<String>,
}

impl StringTable {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
        }
    }

    pub fn add(&mut self, s: String) -> u32 {
        if let Some(idx) = self.strings.iter().position(|x| x == &s) {
            return idx as u32;
        }
        let idx = self.strings.len() as u32;
        self.strings.push(s);
        idx
    }

    pub fn get(&self, idx: u32) -> Option<&str> {
        self.strings.get(idx as usize).map(|s| s.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &str)> {
        self.strings
            .iter()
            .enumerate()
            .map(|(i, s)| (i as u32, s.as_str()))
    }
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub strings: StringTable,
    pub classes: Vec<Class>,
    pub functions: Vec<Function>,
    pub globals: Vec<(String, IrType, Option<Constant>)>,
}

impl Module {
    pub fn new(name: String) -> Self {
        Self {
            name,
            strings: StringTable::new(),
            classes: Vec::new(),
            functions: Vec::new(),
            globals: Vec::new(),
        }
    }

    pub fn get_function(&self, id: &FuncId) -> Option<&Function> {
        self.functions.iter().find(|f| &f.id == id)
    }

    pub fn get_class(&self, id: &ClassId) -> Option<&Class> {
        self.classes.iter().find(|c| &c.id == id)
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "; Module: {}", self.name)?;
        writeln!(f)?;

        if !self.strings.strings.is_empty() {
            writeln!(f, "; String table")?;
            for (idx, s) in self.strings.iter() {
                writeln!(f, ";   str#{} = {:?}", idx, s)?;
            }
            writeln!(f)?;
        }

        for class in &self.classes {
            writeln!(f, "; Class: {}", class.name)?;
            writeln!(f, "struct {} {{", class.id)?;
            for (name, ty) in &class.fields {
                writeln!(f, "    {}: {}", name, ty)?;
            }
            writeln!(f, "}}")?;
            writeln!(f)?;
        }

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
