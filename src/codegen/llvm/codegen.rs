//! LLVM IR Code Generator
//!
//! This module converts Tarqeem IR to LLVM IR text format.

use super::TypeMapper;
use crate::codegen::Target;
use crate::ir::{
    BasicBlock, BinaryOp, BlockId, Class, Constant, Function,
    Instruction, IrType, Module, UnaryOp, VarId,
};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

/// LLVM IR Code Generator
pub struct LlvmCodegen {
    /// Target configuration
    target: Target,
    /// Type mapper
    type_mapper: TypeMapper,
    /// Output buffer
    output: String,
    /// Current function being generated
    current_func: Option<String>,
    /// Variable name mapping (IR VarId -> LLVM name)
    var_map: HashMap<u32, String>,
    /// Variable type mapping (IR VarId -> IrType)
    var_types: HashMap<u32, IrType>,
    /// Block label mapping (IR BlockId -> LLVM label)
    block_map: HashMap<u32, String>,
    /// String literal table (index -> global name)
    string_globals: HashMap<u32, String>,
    /// Counter for unique names
    name_counter: u32,
    /// Class definitions for field access
    class_defs: HashMap<String, Vec<(String, IrType)>>,
    /// VTable globals
    vtable_globals: HashMap<String, String>,
}

impl LlvmCodegen {
    /// Create a new LLVM code generator
    pub fn new(target: Target) -> Self {
        let pointer_bits = target.triple.pointer_bits();
        Self {
            target,
            type_mapper: TypeMapper::new(pointer_bits),
            output: String::new(),
            current_func: None,
            var_map: HashMap::new(),
            var_types: HashMap::new(),
            block_map: HashMap::new(),
            string_globals: HashMap::new(),
            name_counter: 0,
            class_defs: HashMap::new(),
            vtable_globals: HashMap::new(),
        }
    }

    /// Generate LLVM IR for a module
    pub fn generate(&mut self, module: &Module) -> Result<String, CodegenError> {
        self.output.clear();

        // Module header
        self.emit_header(&module.name);

        // Runtime type definitions
        self.emit_runtime_types();

        // String literals
        self.emit_string_table(module);

        // Class definitions
        for class in &module.classes {
            self.emit_class_definition(class)?;
        }

        // External declarations (runtime functions)
        self.emit_runtime_declarations();

        // Function definitions
        for func in &module.functions {
            self.emit_function(func)?;
        }

        Ok(self.output.clone())
    }

    /// Emit module header with target info
    fn emit_header(&mut self, name: &str) {
        writeln!(
            self.output,
            "; ModuleID = '{}'",
            name
        )
        .unwrap();
        writeln!(
            self.output,
            "source_filename = \"{}.trq\"",
            name
        )
        .unwrap();
        writeln!(
            self.output,
            "target datalayout = \"{}\"",
            self.target.llvm_data_layout()
        )
        .unwrap();
        writeln!(
            self.output,
            "target triple = \"{}\"",
            self.target.llvm_triple()
        )
        .unwrap();
        writeln!(self.output).unwrap();
    }

    /// Emit runtime type definitions
    fn emit_runtime_types(&mut self) {
        writeln!(self.output, "; Runtime types").unwrap();
        writeln!(self.output, "{}", TypeMapper::string_struct_type()).unwrap();
        writeln!(self.output, "{}", TypeMapper::array_struct_type()).unwrap();
        writeln!(self.output).unwrap();
    }

    /// Emit string literal table
    fn emit_string_table(&mut self, module: &Module) {
        if module.strings.iter().count() == 0 {
            return;
        }

        writeln!(self.output, "; String literals").unwrap();
        for (idx, s) in module.strings.iter() {
            let escaped = escape_llvm_string(s);
            let len = s.len();
            let global_name = format!("@.str.{}", idx);
            writeln!(
                self.output,
                "{} = private unnamed_addr constant [{} x i8] c\"{}\", align 1",
                global_name,
                len + 1,
                escaped
            )
            .unwrap();
            self.string_globals.insert(idx, global_name);
        }
        writeln!(self.output).unwrap();
    }

    /// Emit class/struct definition
    fn emit_class_definition(&mut self, class: &Class) -> Result<(), CodegenError> {
        writeln!(self.output, "; Class: {}", class.name).unwrap();

        // Store field information for later use
        self.class_defs
            .insert(class.id.0.clone(), class.fields.clone());

        // Generate struct type
        let type_def =
            self.type_mapper
                .generate_struct_type(&class.id, &class.fields);
        writeln!(self.output, "{}", type_def).unwrap();

        // Generate vtable if class has virtual methods
        if !class.vtable.is_empty() {
            self.emit_vtable(class)?;
        }

        writeln!(self.output).unwrap();
        Ok(())
    }

    /// Emit vtable for a class
    fn emit_vtable(&mut self, class: &Class) -> Result<(), CodegenError> {
        let vtable_name = format!("@vtable.{}", class.id.0);

        // VTable is an array of function pointers
        let vtable_entries: Vec<String> = class
            .vtable
            .iter()
            .map(|method| format!("ptr @{}_{}", method.class.0, method.name))
            .collect();

        writeln!(
            self.output,
            "{} = internal constant [{} x ptr] [{}]",
            vtable_name,
            vtable_entries.len(),
            vtable_entries.join(", ")
        )
        .unwrap();

        self.vtable_globals
            .insert(class.id.0.clone(), vtable_name);
        Ok(())
    }

    /// Emit runtime function declarations
    fn emit_runtime_declarations(&mut self) {
        writeln!(self.output, "; Runtime function declarations").unwrap();

        // Memory allocation
        writeln!(self.output, "declare ptr @trq_alloc(i64)").unwrap();
        writeln!(self.output, "declare void @trq_free(ptr)").unwrap();
        writeln!(self.output, "declare void @trq_retain(ptr)").unwrap();
        writeln!(self.output, "declare void @trq_release(ptr)").unwrap();

        // String operations
        writeln!(self.output, "declare ptr @trq_string_new(ptr, i64)").unwrap();
        writeln!(self.output, "declare ptr @trq_string_concat(ptr, ptr)").unwrap();
        writeln!(self.output, "declare i64 @trq_string_len(ptr)").unwrap();
        writeln!(self.output, "declare ptr @trq_int_to_string(i64)").unwrap();
        writeln!(self.output, "declare ptr @trq_float_to_string(double)").unwrap();
        writeln!(self.output, "declare ptr @trq_bool_to_string(i1)").unwrap();

        // Array operations
        writeln!(self.output, "declare ptr @trq_array_new(i64, i64)").unwrap();
        writeln!(self.output, "declare i64 @trq_array_len(ptr)").unwrap();
        writeln!(self.output, "declare ptr @trq_array_get(ptr, i64)").unwrap();
        writeln!(self.output, "declare void @trq_array_set(ptr, i64, ptr)").unwrap();

        // I/O operations
        writeln!(self.output, "declare void @trq_print(ptr)").unwrap();
        writeln!(self.output, "declare void @trq_print_int(i64)").unwrap();
        writeln!(self.output, "declare void @trq_print_float(double)").unwrap();
        writeln!(self.output, "declare void @trq_print_bool(i1)").unwrap();
        writeln!(self.output, "declare void @trq_print_newline()").unwrap();

        // Math operations
        writeln!(self.output, "declare double @llvm.pow.f64(double, double)").unwrap();
        writeln!(self.output, "declare i64 @trq_pow_int(i64, i64)").unwrap();

        // Exception handling
        writeln!(self.output, "declare void @trq_throw(ptr)").unwrap();
        writeln!(self.output, "declare ptr @trq_get_exception()").unwrap();

        // C standard library
        writeln!(self.output, "declare i64 @strlen(ptr)").unwrap();

        writeln!(self.output).unwrap();
    }

    /// Emit a function definition
    fn emit_function(&mut self, func: &Function) -> Result<(), CodegenError> {
        // Reset per-function state
        self.var_map.clear();
        self.block_map.clear();
        self.name_counter = 0;

        let func_name = mangle_function_name(&func.id.0);
        self.current_func = Some(func_name.clone());

        // Map parameters
        for (i, param) in func.params.iter().enumerate() {
            let param_name = format!("%arg.{}", i);
            self.var_map.insert(param.id.0, param_name);
        }

        // Map blocks
        for (i, block) in func.blocks.iter().enumerate() {
            let block_label = if i == 0 {
                "entry".to_string()
            } else if let Some(ref label) = block.label {
                sanitize_label(label)
            } else {
                format!("bb{}", block.id.0)
            };
            self.block_map.insert(block.id.0, block_label);
        }

        // Function signature
        let return_type = self.type_mapper.map_type(&func.return_type);
        let params: Vec<String> = func
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                format!("{} %arg.{}", self.type_mapper.map_param_type(&p.ty), i)
            })
            .collect();

        writeln!(
            self.output,
            "define {} @{}({}) {{",
            return_type,
            func_name,
            params.join(", ")
        )
        .unwrap();

        // Emit blocks
        for block in &func.blocks {
            self.emit_block(block)?;
        }

        writeln!(self.output, "}}").unwrap();
        writeln!(self.output).unwrap();

        self.current_func = None;
        Ok(())
    }

    /// Emit a basic block
    fn emit_block(&mut self, block: &BasicBlock) -> Result<(), CodegenError> {
        let label = self.block_map.get(&block.id.0).unwrap().clone();
        writeln!(self.output, "{}:", label).unwrap();

        for inst in &block.instructions {
            self.emit_instruction(inst)?;
        }

        Ok(())
    }

    /// Emit an instruction
    fn emit_instruction(&mut self, inst: &Instruction) -> Result<(), CodegenError> {
        match inst {
            Instruction::Const { dest, value, ty } => {
                self.emit_const(*dest, value, ty)?;
            }

            Instruction::Binary {
                dest,
                op,
                left,
                right,
                ty,
            } => {
                self.emit_binary(*dest, *op, *left, *right, ty)?;
            }

            Instruction::Unary {
                dest,
                op,
                operand,
                ty,
            } => {
                self.emit_unary(*dest, *op, *operand, ty)?;
            }

            Instruction::IntToFloat { dest, src } => {
                let dest_name = self.get_or_create_var(*dest);
                let src_name = self.get_var(*src)?;
                writeln!(
                    self.output,
                    "  {} = sitofp i64 {} to double",
                    dest_name, src_name
                )
                .unwrap();
            }

            Instruction::FloatToInt { dest, src } => {
                let dest_name = self.get_or_create_var(*dest);
                let src_name = self.get_var(*src)?;
                writeln!(
                    self.output,
                    "  {} = fptosi double {} to i64",
                    dest_name, src_name
                )
                .unwrap();
            }

            Instruction::ToString { dest, src } => {
                // For now, just use a placeholder - needs runtime support
                let dest_name = self.get_or_create_var(*dest);
                let src_name = self.get_var(*src)?;
                writeln!(
                    self.output,
                    "  {} = call ptr @trq_int_to_string(i64 {})",
                    dest_name, src_name
                )
                .unwrap();
            }

            Instruction::Bitcast { dest, src, to_ty } => {
                let dest_name = self.get_or_create_var(*dest);
                let src_name = self.get_var(*src)?;
                let to_type = self.type_mapper.map_type(to_ty);
                writeln!(
                    self.output,
                    "  {} = bitcast ptr {} to {}",
                    dest_name, src_name, to_type
                )
                .unwrap();
            }

            Instruction::Alloca { dest, ty } => {
                let dest_name = self.get_or_create_var(*dest);
                let llvm_ty = self.type_mapper.map_type(ty);
                // Track the allocated pointer type
                self.var_types.insert(dest.0, IrType::Ptr(Box::new(ty.clone())));
                writeln!(self.output, "  {} = alloca {}", dest_name, llvm_ty).unwrap();
            }

            Instruction::Load { dest, ptr, ty } => {
                let dest_name = self.get_or_create_var(*dest);
                let ptr_name = self.get_var(*ptr)?;
                let llvm_ty = self.type_mapper.map_type(ty);
                // Track the loaded value type
                self.var_types.insert(dest.0, ty.clone());
                writeln!(
                    self.output,
                    "  {} = load {}, ptr {}",
                    dest_name, llvm_ty, ptr_name
                )
                .unwrap();
            }

            Instruction::Store { ptr, value } => {
                let ptr_name = self.get_var(*ptr)?;
                let value_name = self.get_var(*value)?;
                // Look up the type of the value
                let val_type = self.var_types.get(&value.0).cloned().unwrap_or(IrType::Int);
                let llvm_ty = self.type_mapper.map_type(&val_type);
                writeln!(
                    self.output,
                    "  store {} {}, ptr {}",
                    llvm_ty, value_name, ptr_name
                )
                .unwrap();
            }

            Instruction::GetElementPtr {
                dest,
                ptr,
                index,
                elem_ty,
            } => {
                let dest_name = self.get_or_create_var(*dest);
                let ptr_name = self.get_var(*ptr)?;
                let index_name = self.get_var(*index)?;
                let llvm_ty = self.type_mapper.map_type(elem_ty);
                writeln!(
                    self.output,
                    "  {} = getelementptr inbounds {}, ptr {}, i64 {}",
                    dest_name, llvm_ty, ptr_name, index_name
                )
                .unwrap();
            }

            Instruction::Jump { target } => {
                let target_label = self.get_block(*target)?;
                writeln!(self.output, "  br label %{}", target_label).unwrap();
            }

            Instruction::Branch {
                cond,
                then_block,
                else_block,
            } => {
                let cond_name = self.get_var(*cond)?;
                let then_label = self.get_block(*then_block)?;
                let else_label = self.get_block(*else_block)?;
                writeln!(
                    self.output,
                    "  br i1 {}, label %{}, label %{}",
                    cond_name, then_label, else_label
                )
                .unwrap();
            }

            Instruction::Return { value } => {
                if let Some(val) = value {
                    let val_name = self.get_var(*val)?;
                    // TODO: Need to know the return type
                    writeln!(self.output, "  ret i64 {}", val_name).unwrap();
                } else {
                    writeln!(self.output, "  ret void").unwrap();
                }
            }

            Instruction::Call {
                dest,
                func,
                args,
                ret_ty,
            } => {
                let func_name = mangle_function_name(&func.0);
                let args_str: Vec<String> = args
                    .iter()
                    .map(|a| {
                        let name = self.get_var(*a).unwrap_or("undef".to_string());
                        format!("i64 {}", name) // TODO: Proper types
                    })
                    .collect();
                let ret_type = self.type_mapper.map_type(ret_ty);

                if let Some(d) = dest {
                    let dest_name = self.get_or_create_var(*d);
                    writeln!(
                        self.output,
                        "  {} = call {} @{}({})",
                        dest_name,
                        ret_type,
                        func_name,
                        args_str.join(", ")
                    )
                    .unwrap();
                } else {
                    writeln!(
                        self.output,
                        "  call {} @{}({})",
                        ret_type,
                        func_name,
                        args_str.join(", ")
                    )
                    .unwrap();
                }
            }

            Instruction::CallIndirect {
                dest,
                func_ptr,
                args,
                ret_ty,
            } => {
                let func_ptr_name = self.get_var(*func_ptr)?;
                let args_str: Vec<String> = args
                    .iter()
                    .map(|a| {
                        let name = self.get_var(*a).unwrap_or("undef".to_string());
                        format!("i64 {}", name)
                    })
                    .collect();
                let ret_type = self.type_mapper.map_type(ret_ty);

                if let Some(d) = dest {
                    let dest_name = self.get_or_create_var(*d);
                    writeln!(
                        self.output,
                        "  {} = call {} {}({})",
                        dest_name,
                        ret_type,
                        func_ptr_name,
                        args_str.join(", ")
                    )
                    .unwrap();
                } else {
                    writeln!(
                        self.output,
                        "  call {} {}({})",
                        ret_type,
                        func_ptr_name,
                        args_str.join(", ")
                    )
                    .unwrap();
                }
            }

            Instruction::NewObject { dest, class } => {
                let dest_name = self.get_or_create_var(*dest);
                // Get size of class struct
                let fields = self.class_defs.get(&class.0).cloned().unwrap_or_default();
                let size: u64 = fields
                    .iter()
                    .map(|(_, ty)| self.type_mapper.type_size(ty))
                    .sum();
                let size = if size == 0 { 8 } else { size }; // Minimum 8 bytes

                writeln!(
                    self.output,
                    "  {} = call ptr @trq_alloc(i64 {})",
                    dest_name, size
                )
                .unwrap();
            }

            Instruction::GetField {
                dest,
                object,
                field,
                ty,
            } => {
                let dest_name = self.get_or_create_var(*dest);
                let obj_name = self.get_var(*object)?;
                let llvm_ty = self.type_mapper.map_type(ty);
                let struct_ty = format!("%class.{}", field.class.0);

                // Get element pointer then load
                let ptr_name = self.fresh_name("field.ptr");
                writeln!(
                    self.output,
                    "  {} = getelementptr inbounds {}, ptr {}, i32 0, i32 {}",
                    ptr_name, struct_ty, obj_name, field.index
                )
                .unwrap();
                writeln!(
                    self.output,
                    "  {} = load {}, ptr {}",
                    dest_name, llvm_ty, ptr_name
                )
                .unwrap();
            }

            Instruction::SetField {
                object,
                field,
                value,
            } => {
                let obj_name = self.get_var(*object)?;
                let val_name = self.get_var(*value)?;
                let struct_ty = format!("%class.{}", field.class.0);

                let ptr_name = self.fresh_name("field.ptr");
                writeln!(
                    self.output,
                    "  {} = getelementptr inbounds {}, ptr {}, i32 0, i32 {}",
                    ptr_name, struct_ty, obj_name, field.index
                )
                .unwrap();
                // Look up the type of the value
                let val_type = self.var_types.get(&value.0).cloned().unwrap_or(IrType::Int);
                let llvm_ty = self.type_mapper.map_type(&val_type);
                writeln!(
                    self.output,
                    "  store {} {}, ptr {}",
                    llvm_ty, val_name, ptr_name
                )
                .unwrap();
            }

            Instruction::CallMethod {
                dest,
                object,
                method,
                args,
                ret_ty,
            } => {
                // Direct method call (non-virtual)
                let obj_name = self.get_var(*object)?;
                let method_name = format!("{}_{}", method.class.0, method.name);

                let mut all_args = vec![format!("ptr {}", obj_name)];
                for arg in args {
                    let arg_name = self.get_var(*arg)?;
                    all_args.push(format!("i64 {}", arg_name));
                }

                let ret_type = self.type_mapper.map_type(ret_ty);
                if let Some(d) = dest {
                    let dest_name = self.get_or_create_var(*d);
                    writeln!(
                        self.output,
                        "  {} = call {} @{}({})",
                        dest_name,
                        ret_type,
                        method_name,
                        all_args.join(", ")
                    )
                    .unwrap();
                } else {
                    writeln!(
                        self.output,
                        "  call {} @{}({})",
                        ret_type,
                        method_name,
                        all_args.join(", ")
                    )
                    .unwrap();
                }
            }

            Instruction::CallVirtual {
                dest,
                object,
                method_index,
                args,
                ret_ty,
            } => {
                // Virtual method call through vtable
                let obj_name = self.get_var(*object)?;

                // Load vtable pointer from object (assume it's at offset 0)
                let vtable_ptr = self.fresh_name("vtable.ptr");
                writeln!(
                    self.output,
                    "  {} = load ptr, ptr {}",
                    vtable_ptr, obj_name
                )
                .unwrap();

                // Get method pointer from vtable
                let method_ptr_ptr = self.fresh_name("method.ptr.ptr");
                writeln!(
                    self.output,
                    "  {} = getelementptr inbounds ptr, ptr {}, i32 {}",
                    method_ptr_ptr, vtable_ptr, method_index
                )
                .unwrap();

                let method_ptr = self.fresh_name("method.ptr");
                writeln!(
                    self.output,
                    "  {} = load ptr, ptr {}",
                    method_ptr, method_ptr_ptr
                )
                .unwrap();

                // Call through function pointer
                let mut all_args = vec![format!("ptr {}", obj_name)];
                for arg in args {
                    let arg_name = self.get_var(*arg)?;
                    all_args.push(format!("i64 {}", arg_name));
                }

                let ret_type = self.type_mapper.map_type(ret_ty);
                if let Some(d) = dest {
                    let dest_name = self.get_or_create_var(*d);
                    writeln!(
                        self.output,
                        "  {} = call {} {}({})",
                        dest_name,
                        ret_type,
                        method_ptr,
                        all_args.join(", ")
                    )
                    .unwrap();
                } else {
                    writeln!(
                        self.output,
                        "  call {} {}({})",
                        ret_type,
                        method_ptr,
                        all_args.join(", ")
                    )
                    .unwrap();
                }
            }

            Instruction::NewArray {
                dest,
                elem_ty,
                elements,
            } => {
                let dest_name = self.get_or_create_var(*dest);
                let elem_size = self.type_mapper.type_size(elem_ty);
                let len = elements.len() as i64;

                // Track the array type - it returns a pointer
                self.var_types.insert(dest.0, IrType::Array(Box::new(elem_ty.clone()), len as usize));

                // Allocate array
                writeln!(
                    self.output,
                    "  {} = call ptr @trq_array_new(i64 {}, i64 {})",
                    dest_name, len, elem_size
                )
                .unwrap();

                // Initialize elements
                for (i, elem) in elements.iter().enumerate() {
                    let elem_name = self.get_var(*elem)?;
                    // Use the actual element type from var_types if available
                    let actual_elem_ty = self.var_types.get(&elem.0).cloned().unwrap_or(elem_ty.clone());
                    let llvm_elem_ty = self.type_mapper.map_type(&actual_elem_ty);
                    // Get element pointer
                    let elem_ptr = self.fresh_name("elem.ptr");
                    writeln!(
                        self.output,
                        "  {} = call ptr @trq_array_get(ptr {}, i64 {})",
                        elem_ptr, dest_name, i
                    )
                    .unwrap();
                    writeln!(
                        self.output,
                        "  store {} {}, ptr {}",
                        llvm_elem_ty, elem_name, elem_ptr
                    )
                    .unwrap();
                }
            }

            Instruction::ArrayLen { dest, array } => {
                let dest_name = self.get_or_create_var(*dest);
                let array_name = self.get_var(*array)?;
                writeln!(
                    self.output,
                    "  {} = call i64 @trq_array_len(ptr {})",
                    dest_name, array_name
                )
                .unwrap();
            }

            Instruction::ArrayGet {
                dest,
                array,
                index,
                elem_ty,
            } => {
                let dest_name = self.get_or_create_var(*dest);
                let array_name = self.get_var(*array)?;
                let index_name = self.get_var(*index)?;
                let llvm_ty = self.type_mapper.map_type(elem_ty);

                let elem_ptr = self.fresh_name("elem.ptr");
                writeln!(
                    self.output,
                    "  {} = call ptr @trq_array_get(ptr {}, i64 {})",
                    elem_ptr, array_name, index_name
                )
                .unwrap();
                writeln!(
                    self.output,
                    "  {} = load {}, ptr {}",
                    dest_name, llvm_ty, elem_ptr
                )
                .unwrap();
            }

            Instruction::ArraySet {
                array,
                index,
                value,
            } => {
                let array_name = self.get_var(*array)?;
                let index_name = self.get_var(*index)?;
                let value_name = self.get_var(*value)?;

                let elem_ptr = self.fresh_name("elem.ptr");
                writeln!(
                    self.output,
                    "  {} = call ptr @trq_array_get(ptr {}, i64 {})",
                    elem_ptr, array_name, index_name
                )
                .unwrap();
                // Look up the type of the value
                let val_type = self.var_types.get(&value.0).cloned().unwrap_or(IrType::Int);
                let llvm_ty = self.type_mapper.map_type(&val_type);
                writeln!(
                    self.output,
                    "  store {} {}, ptr {}",
                    llvm_ty, value_name, elem_ptr
                )
                .unwrap();
            }

            Instruction::StringConcat { dest, left, right } => {
                let dest_name = self.get_or_create_var(*dest);
                let left_name = self.get_var(*left)?;
                let right_name = self.get_var(*right)?;
                writeln!(
                    self.output,
                    "  {} = call ptr @trq_string_concat(ptr {}, ptr {})",
                    dest_name, left_name, right_name
                )
                .unwrap();
            }

            Instruction::TryBegin { catch_block } => {
                // Exception handling - for now just emit a comment
                let catch_label = self.get_block(*catch_block)?;
                writeln!(
                    self.output,
                    "  ; try_begin catch={}",
                    catch_label
                )
                .unwrap();
            }

            Instruction::TryEnd => {
                writeln!(self.output, "  ; try_end").unwrap();
            }

            Instruction::Throw { exception } => {
                let exc_name = self.get_var(*exception)?;
                writeln!(
                    self.output,
                    "  call void @trq_throw(ptr {})",
                    exc_name
                )
                .unwrap();
                writeln!(self.output, "  unreachable").unwrap();
            }

            Instruction::GetException { dest } => {
                let dest_name = self.get_or_create_var(*dest);
                writeln!(
                    self.output,
                    "  {} = call ptr @trq_get_exception()",
                    dest_name
                )
                .unwrap();
            }

            Instruction::Phi { dest, ty, incoming } => {
                let dest_name = self.get_or_create_var(*dest);
                let llvm_ty = self.type_mapper.map_type(ty);

                let entries: Vec<String> = incoming
                    .iter()
                    .map(|(var, block)| {
                        let var_name = self.get_var(*var).unwrap_or("undef".to_string());
                        let block_label = self.get_block(*block).unwrap_or("entry".to_string());
                        format!("[ {}, %{} ]", var_name, block_label)
                    })
                    .collect();

                writeln!(
                    self.output,
                    "  {} = phi {} {}",
                    dest_name,
                    llvm_ty,
                    entries.join(", ")
                )
                .unwrap();
            }

            Instruction::Print { value } => {
                let val_name = self.get_var(*value)?;
                // Dispatch based on type
                let var_type = self.var_types.get(&value.0).cloned();
                match &var_type {
                    Some(IrType::String) | Some(IrType::Ptr(_)) => {
                        // For strings/pointers, create a TrqString from the raw ptr
                        // and then call trq_print
                        let str_var = format!("%tmp_str_{}", self.name_counter);
                        let len_var = format!("%tmp_len_{}", self.name_counter);
                        self.name_counter += 1;
                        // Get string length using strlen
                        writeln!(
                            self.output,
                            "  {} = call i64 @strlen(ptr {})",
                            len_var, val_name
                        ).unwrap();
                        // Create TrqString
                        writeln!(
                            self.output,
                            "  {} = call ptr @trq_string_new(ptr {}, i64 {})",
                            str_var, val_name, len_var
                        ).unwrap();
                        // Print the TrqString
                        writeln!(
                            self.output,
                            "  call void @trq_print(ptr {})",
                            str_var
                        ).unwrap();
                    }
                    Some(IrType::Float) => {
                        writeln!(
                            self.output,
                            "  call void @trq_print_float(double {})",
                            val_name
                        ).unwrap();
                    }
                    Some(IrType::Bool) => {
                        writeln!(
                            self.output,
                            "  call void @trq_print_bool(i1 {})",
                            val_name
                        ).unwrap();
                    }
                    Some(IrType::Array(_, _)) => {
                        // For arrays, print the array reference
                        writeln!(
                            self.output,
                            "  call void @trq_print_int(i64 ptrtoint (ptr {} to i64))",
                            val_name
                        ).unwrap();
                    }
                    _ => {
                        // Default to int
                        writeln!(
                            self.output,
                            "  call void @trq_print_int(i64 {})",
                            val_name
                        ).unwrap();
                    }
                }
                writeln!(self.output, "  call void @trq_print_newline()").unwrap();
            }

            Instruction::Nop => {
                // No operation - emit nothing or a comment
            }
        }

        Ok(())
    }

    /// Emit a constant instruction
    fn emit_const(
        &mut self,
        dest: VarId,
        value: &Constant,
        ty: &IrType,
    ) -> Result<(), CodegenError> {
        let dest_name = self.get_or_create_var(dest);
        // Track the type for later use (e.g., in Print)
        self.var_types.insert(dest.0, ty.clone());

        match value {
            Constant::Null => {
                writeln!(self.output, "  {} = bitcast ptr null to ptr", dest_name).unwrap();
            }
            Constant::Bool(b) => {
                let val = if *b { "true" } else { "false" };
                // For booleans, we can't use add, so we use a select trick
                writeln!(
                    self.output,
                    "  {} = select i1 {}, i1 true, i1 false",
                    dest_name, val
                )
                .unwrap();
            }
            Constant::Int(i) => {
                // Can't just assign in LLVM - need an instruction
                // Use add with 0 as a workaround
                writeln!(
                    self.output,
                    "  {} = add i64 {}, 0",
                    dest_name, i
                )
                .unwrap();
            }
            Constant::Float(f) => {
                writeln!(
                    self.output,
                    "  {} = fadd double {:e}, 0.0",
                    dest_name, f
                )
                .unwrap();
            }
            Constant::String(idx) => {
                // Get pointer to string literal
                if let Some(global) = self.string_globals.get(idx) {
                    writeln!(
                        self.output,
                        "  {} = getelementptr [0 x i8], ptr {}, i64 0, i64 0",
                        dest_name, global
                    )
                    .unwrap();
                } else {
                    writeln!(
                        self.output,
                        "  {} = bitcast ptr null to ptr",
                        dest_name
                    )
                    .unwrap();
                }
            }
        }

        Ok(())
    }

    /// Emit a binary operation
    fn emit_binary(
        &mut self,
        dest: VarId,
        op: BinaryOp,
        left: VarId,
        right: VarId,
        ty: &IrType,
    ) -> Result<(), CodegenError> {
        let dest_name = self.get_or_create_var(dest);
        let left_name = self.get_var(left)?;
        let right_name = self.get_var(right)?;

        let instruction = match (op, ty) {
            // Integer arithmetic
            (BinaryOp::Add, IrType::Int) => "add i64",
            (BinaryOp::Sub, IrType::Int) => "sub i64",
            (BinaryOp::Mul, IrType::Int) => "mul i64",
            (BinaryOp::Div, IrType::Int) => "sdiv i64",
            (BinaryOp::Mod, IrType::Int) => "srem i64",

            // Float arithmetic
            (BinaryOp::Add, IrType::Float) => "fadd double",
            (BinaryOp::Sub, IrType::Float) => "fsub double",
            (BinaryOp::Mul, IrType::Float) => "fmul double",
            (BinaryOp::Div, IrType::Float) => "fdiv double",
            (BinaryOp::Mod, IrType::Float) => "frem double",

            // Power is a special case - needs runtime function
            (BinaryOp::Pow, IrType::Int) => {
                writeln!(
                    self.output,
                    "  {} = call i64 @trq_pow_int(i64 {}, i64 {})",
                    dest_name, left_name, right_name
                )
                .unwrap();
                return Ok(());
            }
            (BinaryOp::Pow, IrType::Float) => {
                writeln!(
                    self.output,
                    "  {} = call double @llvm.pow.f64(double {}, double {})",
                    dest_name, left_name, right_name
                )
                .unwrap();
                return Ok(());
            }

            // Integer comparisons (result type is Bool)
            (BinaryOp::Eq, IrType::Bool) => "icmp eq i64",
            (BinaryOp::Ne, IrType::Bool) => "icmp ne i64",
            (BinaryOp::Lt, IrType::Bool) => "icmp slt i64",
            (BinaryOp::Le, IrType::Bool) => "icmp sle i64",
            (BinaryOp::Gt, IrType::Bool) => "icmp sgt i64",
            (BinaryOp::Ge, IrType::Bool) => "icmp sge i64",
            // Also handle Int type for legacy IR
            (BinaryOp::Eq, IrType::Int) => "icmp eq i64",
            (BinaryOp::Ne, IrType::Int) => "icmp ne i64",
            (BinaryOp::Lt, IrType::Int) => "icmp slt i64",
            (BinaryOp::Le, IrType::Int) => "icmp sle i64",
            (BinaryOp::Gt, IrType::Int) => "icmp sgt i64",
            (BinaryOp::Ge, IrType::Int) => "icmp sge i64",

            // Float comparisons
            (BinaryOp::Eq, IrType::Float) => "fcmp oeq double",
            (BinaryOp::Ne, IrType::Float) => "fcmp one double",
            (BinaryOp::Lt, IrType::Float) => "fcmp olt double",
            (BinaryOp::Le, IrType::Float) => "fcmp ole double",
            (BinaryOp::Gt, IrType::Float) => "fcmp ogt double",
            (BinaryOp::Ge, IrType::Float) => "fcmp oge double",

            // Logical (on booleans represented as i1)
            (BinaryOp::And, IrType::Bool) => "and i1",
            (BinaryOp::Or, IrType::Bool) => "or i1",

            // Bitwise
            (BinaryOp::BitAnd, IrType::Int) => "and i64",
            (BinaryOp::BitOr, IrType::Int) => "or i64",
            (BinaryOp::BitXor, IrType::Int) => "xor i64",
            (BinaryOp::Shl, IrType::Int) => "shl i64",
            (BinaryOp::Shr, IrType::Int) => "ashr i64",

            _ => {
                return Err(CodegenError {
                    message: format!(
                        "Unsupported binary operation: {:?} on {:?}",
                        op, ty
                    ),
                    message_ar: format!(
                        "عملية ثنائية غير مدعومة: {:?} على {:?}",
                        op, ty
                    ),
                });
            }
        };

        writeln!(
            self.output,
            "  {} = {} {}, {}",
            dest_name, instruction, left_name, right_name
        )
        .unwrap();

        Ok(())
    }

    /// Emit a unary operation
    fn emit_unary(
        &mut self,
        dest: VarId,
        op: UnaryOp,
        operand: VarId,
        ty: &IrType,
    ) -> Result<(), CodegenError> {
        let dest_name = self.get_or_create_var(dest);
        let operand_name = self.get_var(operand)?;

        match (op, ty) {
            (UnaryOp::Neg, IrType::Int) => {
                writeln!(
                    self.output,
                    "  {} = sub i64 0, {}",
                    dest_name, operand_name
                )
                .unwrap();
            }
            (UnaryOp::Neg, IrType::Float) => {
                writeln!(
                    self.output,
                    "  {} = fneg double {}",
                    dest_name, operand_name
                )
                .unwrap();
            }
            (UnaryOp::Not, IrType::Bool) => {
                writeln!(
                    self.output,
                    "  {} = xor i1 {}, true",
                    dest_name, operand_name
                )
                .unwrap();
            }
            (UnaryOp::BitNot, IrType::Int) => {
                writeln!(
                    self.output,
                    "  {} = xor i64 {}, -1",
                    dest_name, operand_name
                )
                .unwrap();
            }
            _ => {
                return Err(CodegenError {
                    message: format!("Unsupported unary operation: {:?} on {:?}", op, ty),
                    message_ar: format!(
                        "عملية أحادية غير مدعومة: {:?} على {:?}",
                        op, ty
                    ),
                });
            }
        }

        Ok(())
    }

    /// Get or create a variable name
    fn get_or_create_var(&mut self, var: VarId) -> String {
        if let Some(name) = self.var_map.get(&var.0) {
            name.clone()
        } else {
            let name = format!("%v{}", var.0);
            self.var_map.insert(var.0, name.clone());
            name
        }
    }

    /// Get variable name (must exist)
    fn get_var(&self, var: VarId) -> Result<String, CodegenError> {
        self.var_map.get(&var.0).cloned().ok_or_else(|| CodegenError {
            message: format!("Unknown variable: {}", var),
            message_ar: format!("متغير غير معروف: {}", var),
        })
    }

    /// Get block label
    fn get_block(&self, block: BlockId) -> Result<String, CodegenError> {
        self.block_map
            .get(&block.0)
            .cloned()
            .ok_or_else(|| CodegenError {
                message: format!("Unknown block: {}", block),
                message_ar: format!("كتلة غير معروفة: {}", block),
            })
    }

    /// Generate a fresh unique name
    fn fresh_name(&mut self, prefix: &str) -> String {
        self.name_counter += 1;
        format!("%{}.{}", prefix, self.name_counter)
    }
}

/// Code generation error
#[derive(Debug)]
pub struct CodegenError {
    pub message: String,
    pub message_ar: String,
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CodegenError {}

/// Mangle a function name to be valid for LLVM
fn mangle_function_name(name: &str) -> String {
    // For now, just replace non-ASCII with hex encoding
    let mut result = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            result.push(ch);
        } else {
            // Encode as _U followed by hex codepoint
            result.push_str(&format!("_U{:04X}_", ch as u32));
        }
    }
    result
}

/// Sanitize a block label
fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Escape a string for LLVM IR
fn escape_llvm_string(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        if byte == b'"' || byte == b'\\' || byte < 32 || byte > 126 {
            result.push_str(&format!("\\{:02X}", byte));
        } else {
            result.push(byte as char);
        }
    }
    // Null terminator
    result.push_str("\\00");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mangle_function_name() {
        assert_eq!(mangle_function_name("main"), "main");
        assert_eq!(mangle_function_name("add_numbers"), "add_numbers");
        // Arabic name
        let mangled = mangle_function_name("اطبع");
        assert!(!mangled.contains("اطبع"));
        assert!(mangled.contains("_U"));
    }

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_llvm_string("hello"), "hello\\00");
        assert_eq!(escape_llvm_string("a\"b"), "a\\22b\\00");
        assert_eq!(escape_llvm_string("a\\b"), "a\\5Cb\\00");
    }

    #[test]
    fn test_sanitize_label() {
        assert_eq!(sanitize_label("entry"), "entry");
        assert_eq!(sanitize_label("then.block"), "then.block");
        assert_eq!(sanitize_label("مدخل"), "____");
    }
}
