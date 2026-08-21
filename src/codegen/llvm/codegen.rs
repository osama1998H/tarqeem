//! LLVM IR Code Generator
//!
//! This module converts Tarqeem IR to LLVM IR text format.

use super::{mangle_name, TypeMapper};
use crate::codegen::Target;
use crate::error::codes::{ERR_LLVM_INTERNAL, ERR_UNTYPED_INDIRECT_CALL};
use crate::ir::{
    BasicBlock, BinaryOp, BlockId, Class, ClassId, Constant, Function, Instruction, IrType, Module,
    UnaryOp, VarId,
};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;

/// Struct slots a declared class reserves before its first field: the dispatch
/// pointer at word 0. Object literals (`__anonymous__`) reserve none.
const VTABLE_SLOT: u32 = 1;

macro_rules! emit {
    ($self:expr) => {
        writeln!($self.output).map_err(|e| CodegenError::with_code(
            format!("فشل في كتابة مخرجات LLVM: {}", e),
            ERR_LLVM_INTERNAL.to_string(),
        ))?
    };
    ($self:expr, $($arg:tt)*) => {
        writeln!($self.output, $($arg)*).map_err(|e| CodegenError::with_code(
            format!("فشل في كتابة مخرجات LLVM: {}", e),
            ERR_LLVM_INTERNAL.to_string(),
        ))?
    };
}

pub struct LlvmCodegen {
    target: Target,
    type_mapper: TypeMapper,
    output: String,
    current_func: Option<String>,
    var_map: HashMap<u32, String>,
    var_types: HashMap<u32, IrType>,
    block_map: HashMap<u32, String>,
    string_globals: HashMap<u32, (String, usize)>,
    name_counter: u32,
    class_defs: HashMap<String, Vec<(String, IrType)>>,
    vtable_globals: HashMap<String, String>,
    /// Dispatch slot of each virtual member, per class: "Class" -> member ->
    /// index. A subclass's table extends its parent's as a prefix, so the index
    /// found under the receiver's *static* class also addresses the right entry
    /// in the runtime class's table — that is what makes the load correct
    /// without codegen knowing the runtime type.
    vtable_slots: HashMap<String, HashMap<String, u32>>,
    current_return_type: IrType,
    /// Declared parameter types per mangled function name.
    ///
    /// A call site otherwise types each argument from the argument itself, which
    /// is wrong wherever the callee declares an optional: passing `0` to a
    /// `عدد?` parameter has to box, and only the callee's signature says so.
    fn_param_types: HashMap<String, Vec<IrType>>,
    global_vars: HashMap<String, String>,
    inherited_field_count: HashMap<String, usize>,
    /// Global string variables that need runtime initialization
    /// (mangled_global_name, string_constant_global, length)
    global_string_inits: Vec<(String, String, usize)>,
    /// Global optional scalars needing their initial value boxed at program
    /// start: (mangled_global_name, pointee_type, literal).
    global_optional_inits: Vec<(String, IrType, String)>,
    /// Declared type of each global, by source name. `GlobalStore` otherwise
    /// types the store from the value, which boxes nothing when a scalar is
    /// assigned to an optional global.
    global_types: HashMap<String, IrType>,
    /// Whether the module has a synthesized `__global_init__` function
    /// (non-constant global/`مشترك` initializers). Interpreter mode calls it
    /// explicitly; native codegen must call it from `__main__`'s prologue
    /// itself, or arrays/objects assigned to globals stay null.
    has_global_init: bool,
    /// Names the program declares as functions. A declared name keeps its own
    /// mangled symbol instead of adopting the runtime's, because built-ins are
    /// the last tier of the lookup order (#262). Without it a user's `مطلق` is
    /// *defined* as `@trq_abs_float`, which `emit_runtime_declarations` has
    /// already declared — an invalid redefinition LLVM rejects while parsing
    /// the IR (#257).
    user_function_names: HashSet<String>,
    /// The subset of `user_function_names` that outranks a same-named built-in,
    /// as decided by the IR builder (`Module::shadowing_names`).
    ///
    /// Definitions are mangled against `user_function_names` so *every* user
    /// function keeps a symbol of its own (#257); call sites are mangled against
    /// this narrower set, so a call the semantic layer bound to a built-in still
    /// reaches `trq_*` even when a merged module happens to declare that name.
    shadowing_names: HashSet<String>,
}

impl LlvmCodegen {
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
            vtable_slots: HashMap::new(),
            fn_param_types: HashMap::new(),
            current_return_type: IrType::Void,
            global_vars: HashMap::new(),
            inherited_field_count: HashMap::new(),
            global_string_inits: Vec::new(),
            global_optional_inits: Vec::new(),
            global_types: HashMap::new(),
            has_global_init: false,
            user_function_names: HashSet::new(),
            shadowing_names: HashSet::new(),
        }
    }

    pub fn generate(&mut self, module: &Module) -> Result<String, CodegenError> {
        self.output.clear();
        self.has_global_init = module.functions.iter().any(|f| f.name == "__global_init__");
        // Must precede every `mangle_function_name` call below, definitions and
        // call sites alike, or the two disagree about one name.
        self.user_function_names = module.functions.iter().map(|f| f.id.0.clone()).collect();
        self.shadowing_names = module.shadowing_names.clone();

        self.emit_header(&module.name)?;

        self.emit_runtime_types()?;

        self.emit_string_table(module)?;

        self.emit_global_variables(module)?;

        // First, collect all class's own fields for inheritance lookup
        for class in &module.classes {
            self.class_defs
                .insert(class.id.0.clone(), class.fields.clone());
        }

        // Then emit class definitions with inherited fields
        for class in &module.classes {
            self.emit_class_definition(class, &module.classes)?;
        }

        // Collect and emit anonymous class definitions (for object literals)
        let anon_fields = self.collect_anonymous_class_fields(module);
        self.emit_anonymous_class_definition(&anon_fields)?;

        self.emit_runtime_declarations()?;

        for func in &module.functions {
            self.fn_param_types.insert(
                mangle_function_name(&func.id.0, &self.user_function_names),
                func.params.iter().map(|p| p.ty.clone()).collect(),
            );
        }

        for func in &module.functions {
            self.emit_function(func)?;
        }

        // Note: The C main() entry point is provided by the runtime library (builtins.c).
        // The runtime's main() calls __main__() which is generated by the Tarqeem compiler.
        // Global string initialization is emitted at the start of __main__ in emit_function().

        Ok(self.output.clone())
    }

    /// Emit initialization code for global string variables
    /// Converts raw char* string constants to TrqString* at program start
    fn emit_global_string_init(&mut self) -> Result<(), CodegenError> {
        if self.global_string_inits.is_empty() {
            return Ok(());
        }

        emit!(self, "  ; Initialize global string variables");
        for (global_name, str_global, len) in self.global_string_inits.clone() {
            let ptr_temp = self.new_temp();
            emit!(
                self,
                "  {} = getelementptr [0 x i8], ptr {}, i64 0, i64 0",
                ptr_temp,
                str_global
            );
            let str_temp = self.new_temp();
            emit!(
                self,
                "  {} = call ptr @trq_string_new(ptr {}, i64 {})",
                str_temp,
                ptr_temp,
                len
            );
            emit!(self, "  store ptr {}, ptr @{}", str_temp, global_name);
        }
        Ok(())
    }

    /// The literal text of a scalar constant, or `None` if it is not one.
    fn scalar_literal(init: &Constant) -> Option<String> {
        match init {
            Constant::Int(n) => Some(n.to_string()),
            Constant::Bool(b) => Some(if *b { "1" } else { "0" }.to_string()),
            Constant::Float(f) => {
                let s = format!("{:e}", f);
                Some(if s.contains('.') {
                    s
                } else {
                    s.replace('e', ".0e")
                })
            }
            _ => None,
        }
    }

    /// Box the initial value of each global optional scalar, at program start.
    fn emit_global_optional_init(&mut self) -> Result<(), CodegenError> {
        if self.global_optional_inits.is_empty() {
            return Ok(());
        }

        emit!(self, "  ; Initialize global optional scalars");
        for (global_name, ty, literal) in self.global_optional_inits.clone() {
            let llvm_ty = self.type_mapper.map_type(&ty);
            let boxed = self.new_temp();
            emit!(self, "  {} = call ptr @trq_alloc(i64 8)", boxed);
            emit!(self, "  store {} {}, ptr {}", llvm_ty, literal, boxed);
            emit!(self, "  store ptr {}, ptr @{}", boxed, global_name);
        }
        Ok(())
    }

    fn emit_header(&mut self, name: &str) -> Result<(), CodegenError> {
        emit!(self, "; ModuleID = '{}'", name);
        emit!(self, "source_filename = \"{}\"", name);
        emit!(
            self,
            "target datalayout = \"{}\"",
            self.target.llvm_data_layout()
        );
        emit!(self, "target triple = \"{}\"", self.target.llvm_triple());
        emit!(self);
        Ok(())
    }

    fn emit_runtime_types(&mut self) -> Result<(), CodegenError> {
        emit!(self, "; Runtime types");
        emit!(self, "{}", TypeMapper::string_struct_type());
        emit!(self, "{}", TypeMapper::array_struct_type());
        emit!(self);
        Ok(())
    }

    fn emit_string_table(&mut self, module: &Module) -> Result<(), CodegenError> {
        if module.strings.iter().count() == 0 {
            return Ok(());
        }

        emit!(self, "; String literals");
        for (idx, s) in module.strings.iter() {
            let escaped = escape_llvm_string(s);
            let len = s.len();
            let global_name = format!("@.str.{}", idx);
            emit!(
                self,
                "{} = private unnamed_addr constant [{} x i8] c\"{}\", align 1",
                global_name,
                len + 1,
                escaped
            );
            self.string_globals.insert(idx, (global_name, len));
        }
        emit!(self);
        Ok(())
    }

    fn emit_global_variables(&mut self, module: &Module) -> Result<(), CodegenError> {
        if module.globals.is_empty() {
            return Ok(());
        }

        emit!(self, "; Global variables");
        for (name, ty, init) in &module.globals {
            let llvm_type = self.type_mapper.map_type(ty);
            let global_name = mangle_name(name);
            self.global_types.insert(name.clone(), ty.clone());

            // A global optional scalar cannot carry its value in the initializer:
            // the slot is a pointer, so the value has to live in a box, and a box
            // needs an allocation. Defer it the way string globals already are —
            // start as null, fill in at program start (#185).
            if let IrType::Ptr(pointee) = ty {
                if matches!(**pointee, IrType::Int | IrType::Float | IrType::Bool) {
                    if let Some(literal) = init.as_ref().and_then(Self::scalar_literal) {
                        emit!(self, "@{} = global {} null", global_name, llvm_type);
                        self.global_optional_inits.push((
                            global_name.clone(),
                            (**pointee).clone(),
                            literal,
                        ));
                        self.global_vars.insert(name.clone(), global_name);
                        continue;
                    }
                }
            }

            let init_val = match init {
                Some(Constant::Int(n)) => n.to_string(),
                Some(Constant::Float(f)) => {
                    // LLVM requires a decimal point in float literals (1.0e4, not 1e4)
                    let s = format!("{:e}", f);
                    if !s.contains('.') {
                        // Insert .0 before 'e' if no decimal point present
                        s.replace("e", ".0e")
                    } else {
                        s
                    }
                }
                Some(Constant::Bool(b)) => {
                    if *b {
                        "1".to_string()
                    } else {
                        "0".to_string()
                    }
                }
                Some(Constant::Null) => "null".to_string(),
                Some(Constant::Function(name)) => {
                    format!("@{}", mangle_function_name(name, &self.shadowing_names))
                }
                Some(Constant::String(idx)) => {
                    // String globals store TrqString*, initialized at program start
                    // Store the init info for emit_global_string_init() to handle
                    if let Some((str_global, len)) = self.string_globals.get(idx) {
                        self.global_string_inits.push((
                            global_name.clone(),
                            str_global.clone(),
                            *len,
                        ));
                    }
                    "null".to_string()
                }
                None => self.zero_initializer(ty),
            };

            emit!(self, "@{} = global {} {}", global_name, llvm_type, init_val);

            self.global_vars.insert(name.clone(), global_name);
        }
        emit!(self);
        Ok(())
    }

    fn zero_initializer(&self, ty: &IrType) -> String {
        match ty {
            IrType::Int => "0".to_string(),
            IrType::Float => "0.0".to_string(),
            IrType::Bool => "0".to_string(),
            IrType::String => "null".to_string(),
            IrType::Ptr(_) => "null".to_string(),
            IrType::Array(_, _) => "zeroinitializer".to_string(),
            IrType::Struct(_) => "zeroinitializer".to_string(),
            IrType::Enum(_) => "zeroinitializer".to_string(),
            IrType::Void => "zeroinitializer".to_string(),
            IrType::Function { .. } => "null".to_string(),
        }
    }

    fn emit_class_definition(
        &mut self,
        class: &Class,
        all_classes: &[Class],
    ) -> Result<(), CodegenError> {
        emit!(self, "; Class: {}", class.name);

        // Collect all fields including inherited ones (parent fields come first)
        let all_fields = self.collect_class_fields(class, all_classes);

        // Calculate inherited field count (fields before this class's own fields)
        let inherited_count = all_fields.len() - class.fields.len();
        self.inherited_field_count
            .insert(class.id.0.clone(), inherited_count);

        // Update class_defs with full field list for correct field access
        self.class_defs
            .insert(class.id.0.clone(), all_fields.clone());

        let type_def = self
            .type_mapper
            .generate_struct_type(&class.id, &all_fields, true);
        emit!(self, "{}", type_def);

        if !class.vtable.is_empty() {
            self.vtable_slots.insert(
                class.id.0.clone(),
                class
                    .vtable
                    .iter()
                    .enumerate()
                    .map(|(slot, method)| (method.name.clone(), slot as u32))
                    .collect(),
            );
            self.emit_vtable(class)?;
        }

        emit!(self);
        Ok(())
    }

    /// Recursively collect all fields for a class including inherited fields
    fn collect_class_fields(&self, class: &Class, all_classes: &[Class]) -> Vec<(String, IrType)> {
        let mut fields = Vec::new();

        // First, add parent class fields (recursively)
        if let Some(parent_id) = &class.parent {
            if let Some(parent_class) = all_classes.iter().find(|c| c.id.0 == parent_id.0) {
                fields.extend(self.collect_class_fields(parent_class, all_classes));
            }
        }

        // Then add this class's own fields
        fields.extend(class.fields.iter().cloned());

        fields
    }

    /// Collect field information for anonymous classes from the IR.
    /// Scans all functions for NewObject instructions with __anonymous__ class
    /// and gathers field types from subsequent SetField instructions.
    fn collect_anonymous_class_fields(&self, module: &Module) -> Vec<(String, IrType)> {
        let mut fields: Vec<(String, IrType)> = Vec::new();
        let mut seen_fields: std::collections::HashSet<String> = std::collections::HashSet::new();

        for func in &module.functions {
            // Build a type map for variables in this function
            let var_types = self.infer_var_types(func);

            for block in &func.blocks {
                let mut anon_objects: std::collections::HashSet<u32> =
                    std::collections::HashSet::new();

                for instr in &block.instructions {
                    match instr {
                        Instruction::NewObject { dest, class } => {
                            if class.0 == "__anonymous__" {
                                anon_objects.insert(dest.0);
                            }
                        }
                        Instruction::SetField {
                            object,
                            field,
                            value,
                        } if anon_objects.contains(&object.0)
                            && field.class.0 == "__anonymous__"
                            && !seen_fields.contains(&field.name) =>
                        {
                            // Get the type of the value being set
                            let field_ty = var_types
                                .get(&value.0)
                                .cloned()
                                .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                            fields.push((field.name.clone(), field_ty));
                            seen_fields.insert(field.name.clone());
                        }
                        _ => {}
                    }
                }
            }
        }

        fields
    }

    /// Infer types for all variables in a function by analyzing instructions.
    fn infer_var_types(&self, func: &Function) -> HashMap<u32, IrType> {
        let mut types: HashMap<u32, IrType> = HashMap::new();

        // Add parameter types
        for param in &func.params {
            types.insert(param.id.0, param.ty.clone());
        }

        // Scan all instructions
        for block in &func.blocks {
            for instr in &block.instructions {
                match instr {
                    Instruction::Const { dest, ty, .. } => {
                        types.insert(dest.0, ty.clone());
                    }
                    Instruction::Binary { dest, ty, .. } => {
                        types.insert(dest.0, ty.clone());
                    }
                    Instruction::Unary { dest, ty, .. } => {
                        types.insert(dest.0, ty.clone());
                    }
                    Instruction::Call {
                        dest: Some(dest),
                        ret_ty,
                        ..
                    } => {
                        types.insert(dest.0, ret_ty.clone());
                    }
                    Instruction::NewObject { dest, class } => {
                        types.insert(dest.0, IrType::Ptr(Box::new(IrType::Struct(class.clone()))));
                    }
                    Instruction::GetField { dest, ty, .. } => {
                        types.insert(dest.0, ty.clone());
                    }
                    Instruction::NewArray { dest, elem_ty, .. } => {
                        types.insert(dest.0, IrType::Ptr(Box::new(elem_ty.clone())));
                    }
                    Instruction::GetElementPtr { dest, elem_ty, .. } => {
                        types.insert(dest.0, elem_ty.clone());
                    }
                    Instruction::Alloca { dest, ty, .. } => {
                        types.insert(dest.0, IrType::Ptr(Box::new(ty.clone())));
                    }
                    Instruction::Load { dest, ty, .. } => {
                        types.insert(dest.0, ty.clone());
                    }
                    Instruction::Phi { dest, ty, .. } => {
                        types.insert(dest.0, ty.clone());
                    }
                    Instruction::Bitcast { dest, to_ty, .. } => {
                        types.insert(dest.0, to_ty.clone());
                    }
                    Instruction::CallMethod {
                        dest: Some(dest),
                        ret_ty,
                        ..
                    } => {
                        types.insert(dest.0, ret_ty.clone());
                    }
                    _ => {}
                }
            }
        }

        types
    }

    /// Emit the struct type definition for anonymous classes.
    fn emit_anonymous_class_definition(
        &mut self,
        fields: &[(String, IrType)],
    ) -> Result<(), CodegenError> {
        if fields.is_empty() {
            return Ok(());
        }

        emit!(self, "; Anonymous class (object literals)");
        let class_id = ClassId("__anonymous__".to_string());
        let type_def = self
            .type_mapper
            .generate_struct_type(&class_id, fields, false);
        emit!(self, "{}", type_def);

        // Register the fields in class_defs for field access
        self.class_defs
            .insert("__anonymous__".to_string(), fields.to_vec());

        emit!(self);
        Ok(())
    }

    /// Calculate field access information for correct struct indexing with inheritance.
    ///
    /// Returns (struct_type_name, actual_index) where:
    /// - struct_type_name: The LLVM struct type name for the object's actual class
    /// - actual_index: The field index adjusted for inherited fields
    fn get_field_access_info(
        &self,
        field_class: &str,
        field_name: &str,
        field_index: u32,
        obj_ty: &Option<IrType>,
    ) -> (String, u32) {
        // Get the actual object class for struct type
        let actual_class = match obj_ty {
            Some(IrType::Ptr(inner)) => match inner.as_ref() {
                IrType::Struct(class_id) if !class_id.0.is_empty() => class_id.0.clone(),
                _ => {
                    // If the type is Ptr to non-struct or empty class, use field_class
                    // If field_class is also empty, default to __anonymous__ (for object literals)
                    if field_class.is_empty() {
                        "__anonymous__".to_string()
                    } else {
                        field_class.to_string()
                    }
                }
            },
            Some(IrType::Struct(class_id)) if !class_id.0.is_empty() => class_id.0.clone(),
            _ => {
                // If no type info or empty class name, default to __anonymous__ for object literals
                if field_class.is_empty() {
                    "__anonymous__".to_string()
                } else {
                    field_class.to_string()
                }
            }
        };

        let struct_ty = format!("%class.{}", mangle_class_name(&actual_class));

        // For anonymous classes, look up field index by name from class_defs
        let actual_index = if actual_class == "__anonymous__" {
            if let Some(fields) = self.class_defs.get("__anonymous__") {
                fields
                    .iter()
                    .position(|(name, _)| name == field_name)
                    .map(|i| i as u32)
                    .unwrap_or(field_index)
            } else {
                field_index
            }
        } else {
            // Calculate actual index: inherited fields + field index within defining class
            let lookup_class = if field_class.is_empty() {
                &actual_class
            } else {
                field_class
            };
            let inherited_count = self
                .inherited_field_count
                .get(lookup_class)
                .copied()
                .unwrap_or(0) as u32;
            // +1 for the vtable pointer every declared class carries at word 0
            // (`generate_struct_type`). Both object GEP sites — GetField and
            // SetField — reach the index through here, so this is the only
            // place the shift belongs.
            VTABLE_SLOT + inherited_count + field_index
        };

        (struct_ty, actual_index)
    }

    /// The dispatch slot for `member`, read off the receiver's *static* class.
    ///
    /// Sound because a subclass's table extends its parent's as a prefix: the
    /// index a static class assigns to a member is the index every descendant
    /// assigns it too, so the entry loaded from the object's own table is the
    /// implementation its runtime class provides. `None` means "no table for
    /// this receiver" — an untyped, interface-typed or anonymous receiver — and
    /// the caller falls back to a static bind.
    fn receiver_vtable_slot(&self, object: u32, member: &str) -> Option<u32> {
        let class = match self.var_types.get(&object)? {
            IrType::Ptr(inner) => match inner.as_ref() {
                IrType::Struct(class_id) => &class_id.0,
                _ => return None,
            },
            IrType::Struct(class_id) => &class_id.0,
            _ => return None,
        };

        self.vtable_slots.get(class)?.get(member).copied()
    }

    fn emit_vtable(&mut self, class: &Class) -> Result<(), CodegenError> {
        let mangled_class = mangle_class_name(&class.id.0);
        let vtable_name = format!("@vtable.{}", mangled_class);

        // Must match `emit_function`'s symbol exactly: bodies are emitted as
        // ordinary functions named `mangle_function_name("{Class}::{method}")`.
        // Spelling these `@{class}_{method}` instead would name symbols nothing
        // defines — invisible until a vtable was first populated.
        let vtable_entries: Vec<String> = class
            .vtable
            .iter()
            .map(|method| {
                let symbol = mangle_function_name(
                    &format!("{}::{}", method.class.0, method.name),
                    &self.user_function_names,
                );
                format!("ptr @{}", symbol)
            })
            .collect();

        emit!(
            self,
            "{} = internal constant [{} x ptr] [{}]",
            vtable_name,
            vtable_entries.len(),
            vtable_entries.join(", ")
        );

        self.vtable_globals.insert(class.id.0.clone(), vtable_name);
        Ok(())
    }

    fn emit_runtime_declarations(&mut self) -> Result<(), CodegenError> {
        emit!(self, "; Runtime function declarations");

        emit!(self, "declare ptr @trq_alloc(i64)");
        emit!(self, "declare void @trq_free(ptr)");
        emit!(self, "declare void @trq_retain(ptr)");
        emit!(self, "declare void @trq_release(ptr)");

        emit!(self, "declare ptr @trq_string_new(ptr, i64)");
        emit!(self, "declare ptr @trq_string_concat(ptr, ptr)");
        emit!(self, "declare i64 @trq_string_len(ptr)");
        emit!(self, "declare i64 @trq_string_len_chars(ptr)");
        emit!(self, "declare ptr @trq_string_substr_chars(ptr, i64, i64)");
        emit!(self, "declare ptr @trq_string_char_at(ptr, i64)");
        emit!(self, "declare i64 @trq_string_char_code(ptr)");
        emit!(self, "declare ptr @trq_string_from_char_code(i64)");
        emit!(self, "declare ptr @trq_string_to_bytes(ptr)");
        emit!(self, "declare ptr @trq_string_from_bytes(ptr)");
        emit!(self, "declare i1 @trq_string_contains(ptr, ptr)");
        emit!(self, "declare i1 @trq_string_starts_with(ptr, ptr)");
        emit!(self, "declare i1 @trq_string_ends_with(ptr, ptr)");
        emit!(self, "declare i64 @trq_string_index_of(ptr, ptr)");
        emit!(self, "declare i64 @trq_string_last_index_of(ptr, ptr)");
        emit!(self, "declare i64 @trq_string_count(ptr, ptr)");
        emit!(self, "declare ptr @trq_string_to_upper(ptr)");
        emit!(self, "declare ptr @trq_string_to_lower(ptr)");
        emit!(self, "declare ptr @trq_string_to_title(ptr)");
        emit!(self, "declare ptr @trq_string_reverse(ptr)");
        emit!(self, "declare ptr @trq_string_trim(ptr)");
        emit!(self, "declare ptr @trq_string_trim_left(ptr)");
        emit!(self, "declare ptr @trq_string_trim_right(ptr)");
        emit!(self, "declare ptr @trq_string_split(ptr, ptr)");
        emit!(self, "declare ptr @trq_string_join(ptr, ptr)");
        emit!(self, "declare ptr @trq_string_replace(ptr, ptr, ptr)");
        emit!(self, "declare ptr @trq_string_replace_all(ptr, ptr, ptr)");
        emit!(self, "declare ptr @trq_string_repeat(ptr, i64)");
        emit!(self, "declare ptr @trq_string_pad_left(ptr, i64, ptr)");
        emit!(self, "declare ptr @trq_string_pad_right(ptr, i64, ptr)");
        emit!(self, "declare i1 @trq_string_is_numeric(ptr)");
        emit!(self, "declare i1 @trq_string_is_alpha(ptr)");
        emit!(self, "declare i1 @trq_string_is_arabic(ptr)");
        emit!(self, "declare i64 @trq_string_compare(ptr, ptr)");
        emit!(self, "declare i1 @trq_string_equals(ptr, ptr)");

        emit!(self, "declare ptr @trq_int_to_string(i64)");
        emit!(self, "declare ptr @trq_float_to_string(double)");
        emit!(self, "declare ptr @trq_bool_to_string(i1 zeroext)");
        emit!(self, "declare i64 @trq_string_to_int(ptr)");
        emit!(self, "declare double @trq_string_to_float(ptr)");
        // The `عدد`/`عدد_عشري` builtins reject an unparsable string instead of
        // yielding 0, matching the interpreter (#222).
        emit!(self, "declare i64 @trq_string_to_int_checked(ptr)");
        emit!(self, "declare double @trq_string_to_float_checked(ptr)");
        // `i1 zeroext` per the FFI convention below: Rust's `bool` is UB for any
        // other bit pattern.
        emit!(self, "declare void @trq_assert(i1 zeroext, ptr)");

        emit!(self, "declare ptr @trq_array_new(i64, i64)");
        emit!(self, "declare i64 @trq_array_len(ptr)");
        emit!(self, "declare ptr @trq_array_get(ptr, i64)");
        emit!(self, "declare void @trq_array_set(ptr, i64, ptr)");
        emit!(self, "declare void @trq_array_push(ptr, ptr, i64)");
        emit!(self, "declare ptr @trq_array_pop(ptr)");

        emit!(self, "declare void @trq_print(ptr)");
        emit!(self, "declare void @trq_print_int(i64)");
        emit!(self, "declare void @trq_print_float(double)");
        emit!(self, "declare void @trq_print_optional_scalar(ptr, i64)");
        emit!(self, "declare void @trq_print_bool(i1 zeroext)");
        emit!(self, "declare void @trq_print_array(ptr)");
        emit!(self, "declare void @trq_print_newline()");
        emit!(self, "declare void @trq_print_error(ptr)");
        emit!(self, "declare ptr @trq_input()");
        emit!(self, "declare ptr @trq_input_prompt(ptr)");
        emit!(self, "declare i64 @trq_input_int()");
        emit!(self, "declare double @trq_input_float()");

        emit!(self, "declare double @llvm.pow.f64(double, double)");
        emit!(self, "declare i64 @trq_pow_int(i64, i64)");
        emit!(self, "declare double @trq_pow_float(double, double)");
        emit!(self, "declare i64 @trq_abs_int(i64)");
        emit!(self, "declare double @trq_abs_float(double)");
        emit!(self, "declare double @trq_sqrt(double)");
        emit!(self, "declare double @trq_cbrt(double)");
        emit!(self, "declare double @trq_nroot(double, i64)");
        emit!(self, "declare double @trq_log(double)");
        emit!(self, "declare double @trq_log10(double)");
        emit!(self, "declare double @trq_log2(double)");
        emit!(self, "declare double @trq_exp(double)");
        emit!(self, "declare double @trq_floor(double)");
        emit!(self, "declare double @trq_ceil(double)");
        emit!(self, "declare double @trq_round(double)");
        emit!(self, "declare double @trq_trunc(double)");
        emit!(self, "declare i64 @trq_min_int(i64, i64)");
        emit!(self, "declare i64 @trq_max_int(i64, i64)");
        emit!(self, "declare double @trq_min_float(double, double)");
        emit!(self, "declare double @trq_max_float(double, double)");
        emit!(self, "declare i64 @trq_clamp_int(i64, i64, i64)");
        emit!(
            self,
            "declare double @trq_clamp_float(double, double, double)"
        );
        emit!(self, "declare i64 @trq_sign(i64)");
        emit!(self, "declare i64 @trq_mod(i64, i64)");
        emit!(self, "declare i64 @trq_gcd(i64, i64)");
        emit!(self, "declare i64 @trq_lcm(i64, i64)");
        emit!(self, "declare i64 @trq_factorial(i64)");

        emit!(self, "declare double @trq_sin(double)");
        emit!(self, "declare double @trq_cos(double)");
        emit!(self, "declare double @trq_tan(double)");
        emit!(self, "declare double @trq_cot(double)");
        emit!(self, "declare double @trq_sec(double)");
        emit!(self, "declare double @trq_csc(double)");
        emit!(self, "declare double @trq_asin(double)");
        emit!(self, "declare double @trq_acos(double)");
        emit!(self, "declare double @trq_atan(double)");
        emit!(self, "declare double @trq_atan2(double, double)");
        emit!(self, "declare double @trq_sinh(double)");
        emit!(self, "declare double @trq_cosh(double)");
        emit!(self, "declare double @trq_tanh(double)");
        emit!(self, "declare double @trq_to_radians(double)");
        emit!(self, "declare double @trq_to_degrees(double)");

        emit!(self, "declare void @trq_random_seed(i64)");
        emit!(self, "declare i64 @trq_random_int()");
        emit!(self, "declare i64 @trq_random_int_range(i64, i64)");
        emit!(self, "declare double @trq_random_float()");
        emit!(
            self,
            "declare double @trq_random_float_range(double, double)"
        );
        emit!(self, "declare i1 @trq_random_bool()");

        emit!(self, "declare i1 @trq_file_exists(ptr)");
        emit!(self, "declare i1 @trq_file_is_file(ptr)");
        emit!(self, "declare i1 @trq_file_is_dir(ptr)");
        emit!(self, "declare ptr @trq_file_read(ptr)");
        emit!(self, "declare i1 @trq_file_write(ptr, ptr)");
        emit!(self, "declare i1 @trq_file_append(ptr, ptr)");
        emit!(self, "declare i1 @trq_file_delete(ptr)");
        emit!(self, "declare i1 @trq_file_copy(ptr, ptr)");
        emit!(self, "declare i1 @trq_file_move(ptr, ptr)");
        emit!(self, "declare i64 @trq_file_size(ptr)");
        emit!(self, "declare i64 @trq_file_open(ptr, i64)");
        emit!(self, "declare i64 @trq_path_status(ptr, i64)");
        emit!(self, "declare i1 @trq_path_delete(ptr)");
        emit!(self, "declare ptr @trq_program_args()");
        emit!(self, "declare i1 @trq_dir_create(ptr)");
        emit!(self, "declare i1 @trq_dir_create_all(ptr)");
        emit!(self, "declare i1 @trq_dir_delete(ptr)");
        emit!(self, "declare ptr @trq_dir_list(ptr)");
        emit!(self, "declare ptr @trq_dir_current()");
        emit!(self, "declare ptr @trq_dir_home()");
        emit!(self, "declare ptr @trq_dir_temp()");
        emit!(self, "declare ptr @trq_path_join(ptr, ptr)");
        emit!(self, "declare ptr @trq_path_parent(ptr)");
        emit!(self, "declare ptr @trq_path_filename(ptr)");
        emit!(self, "declare ptr @trq_path_extension(ptr)");
        emit!(self, "declare ptr @trq_path_stem(ptr)");
        emit!(self, "declare ptr @trq_path_absolute(ptr)");
        emit!(self, "declare i1 @trq_path_is_absolute(ptr)");
        emit!(self, "declare ptr @trq_path_separator()");

        // Beside the directory readers because they read the environment too.
        // `trq_dir_home` *is* this call with `HOME` hardcoded; `trq_dir_temp` is
        // not — it calls `std::env::temp_dir()`, which falls back to `/tmp` when
        // `TMPDIR` is unset and walks `TMP`/`TEMP`/`USERPROFILE` on Windows.
        emit!(self, "declare ptr @trq_env_get(ptr)");
        // `write(2)`. `i64` both ways: the descriptor and the byte count are
        // `عدد`, and the `ptr` in between is the `TrqArray` of bytes.
        emit!(self, "declare i64 @trq_write_stream(i64, ptr)");
        // `read(2)`. Two `i64` in — the descriptor and the count, both `عدد` —
        // and a `ptr` out, the `TrqArray` of bytes it answers.
        emit!(self, "declare ptr @trq_read_stream(i64, i64)");

        emit!(self, "declare ptr @trq_date_today()");
        emit!(self, "declare ptr @trq_date_parse(ptr)");
        emit!(self, "declare ptr @trq_date_from_timestamp(i64)");
        emit!(self, "declare ptr @trq_date_add_days(i64, i64, i64, i64)");
        emit!(self, "declare ptr @trq_date_add_months(i64, i64, i64, i64)");
        emit!(
            self,
            "declare i64 @trq_date_diff_days(i64, i64, i64, i64, i64, i64)"
        );
        emit!(self, "declare i64 @trq_day_of_week(i64, i64, i64)");
        emit!(self, "declare i64 @trq_day_of_year(i64, i64, i64)");
        emit!(self, "declare i64 @trq_week_number(i64, i64, i64)");
        emit!(self, "declare i64 @trq_days_in_month(i64, i64)");
        emit!(self, "declare ptr @trq_date_format(i64, i64, i64, ptr)");
        // i64, not ptr: the semantic layer types وقت_الآن as عدد and the
        // interpreter returns epoch milliseconds. The old `ptr` came from an
        // unbuilt struct-returning date API (#241).
        emit!(self, "declare i64 @trq_time_now()");
        emit!(self, "declare ptr @trq_time_parse(ptr)");
        emit!(
            self,
            "declare ptr @trq_time_format(i64, i64, i64, i64, ptr)"
        );
        emit!(self, "declare ptr @trq_datetime_now()");
        emit!(self, "declare ptr @trq_datetime_from_timestamp(i64)");
        emit!(self, "declare ptr @trq_datetime_parse(ptr)");
        emit!(
            self,
            "declare ptr @trq_datetime_format(i64, i64, i64, i64, i64, i64, ptr)"
        );
        emit!(self, "declare void @trq_sleep(i64)");
        // `أنهِ_البرنامج`. The call site emits `call ptr` against this `void`
        // declare, because the name deliberately has no return type registered —
        // clang accepts the mismatch under opaque pointers, and registering the
        // type would break `متغير س = أنهِ_البرنامج(٣)` natively while both
        // interpreters ran it. The reasoning and the measurements are on the
        // matching gap in `IrBuilder::register_builtin_return_types`; #343 is what
        // has to land before this becomes `call void`.
        emit!(self, "declare void @trq_exit(i64)");
        emit!(self, "declare i64 @trq_performance_now()");

        emit!(self, "declare i64 @trq_tcp_connect(ptr, i64, i64)");
        emit!(self, "declare void @trq_tcp_close(i64)");
        emit!(self, "declare i1 @trq_tcp_send(i64, ptr)");
        emit!(self, "declare i1 @trq_tcp_send_bytes(i64, ptr)");
        emit!(self, "declare ptr @trq_tcp_receive(i64, i64)");
        emit!(self, "declare ptr @trq_tcp_receive_bytes(i64, i64, i64)");
        emit!(self, "declare ptr @trq_tcp_receive_until(i64, ptr, i64)");
        emit!(self, "declare i1 @trq_tcp_available(i64)");
        emit!(self, "declare i64 @trq_tcp_listen(ptr, i64, i64)");
        emit!(self, "declare ptr @trq_tcp_accept(i64)");
        emit!(self, "declare ptr @trq_tcp_accept_timeout(i64, i64)");
        emit!(self, "declare ptr @trq_tcp_local_address(i64)");
        emit!(self, "declare i64 @trq_tcp_local_port(i64)");
        emit!(self, "declare i64 @trq_udp_bind(i64)");
        emit!(self, "declare void @trq_udp_close(i64)");
        emit!(self, "declare i1 @trq_udp_send_to(i64, ptr, i64, ptr)");
        emit!(
            self,
            "declare i1 @trq_udp_send_bytes_to(i64, ptr, i64, ptr)"
        );
        emit!(self, "declare ptr @trq_udp_receive(i64, i64)");
        emit!(self, "declare ptr @trq_udp_receive_bytes(i64, i64, i64)");
        emit!(self, "declare i1 @trq_udp_reply(i64, ptr)");
        emit!(self, "declare ptr @trq_resolve_hostname(ptr)");
        emit!(self, "declare ptr @trq_get_local_ip()");
        emit!(
            self,
            "declare ptr @trq_http_request(ptr, ptr, ptr, ptr, i64, i1)"
        );
        emit!(self, "declare i1 @trq_http_download(ptr, ptr)");
        emit!(self, "declare ptr @trq_http_get(ptr)");
        emit!(self, "declare ptr @trq_url_encode(ptr)");
        emit!(self, "declare ptr @trq_url_decode(ptr)");
        emit!(self, "declare ptr @trq_base64_encode(ptr)");
        emit!(self, "declare ptr @trq_base64_decode(ptr)");

        // Cryptography - SHA-256
        emit!(self, "declare ptr @trq_sha256_string(ptr)");
        emit!(self, "declare ptr @trq_sha256_file(ptr)");
        emit!(self, "declare ptr @trq_sha256_bytes(ptr)");
        emit!(self, "declare i1 @trq_sha256_compare(ptr, ptr)");

        // Cryptography - Hex encoding
        emit!(self, "declare ptr @trq_hex_encode(ptr)");
        emit!(self, "declare ptr @trq_hex_decode(ptr)");
        emit!(self, "declare ptr @trq_hex_encode_bytes(ptr)");
        emit!(self, "declare ptr @trq_hex_decode_to_bytes(ptr)");

        // Compression - gzip
        emit!(self, "declare ptr @trq_gzip_compress_string(ptr)");
        emit!(self, "declare ptr @trq_gzip_decompress_to_string(ptr)");
        emit!(self, "declare ptr @trq_gzip_compress_bytes(ptr)");
        emit!(self, "declare ptr @trq_gzip_decompress_bytes(ptr)");
        emit!(self, "declare i1 @trq_gzip_compress_file(ptr, ptr)");
        emit!(self, "declare i1 @trq_gzip_decompress_file(ptr, ptr)");

        emit!(self, "declare void @trq_throw(ptr)");
        emit!(self, "declare ptr @trq_get_exception()");
        emit!(self, "declare void @trq_panic(ptr)");

        emit!(self, "declare i64 @strlen(ptr)");

        emit!(self);
        Ok(())
    }

    fn emit_function(&mut self, func: &Function) -> Result<(), CodegenError> {
        // A lambda param that never resolved to a concrete type (no
        // annotation, no hint from context) lowers to `Ptr(Void)` ("ptr" in
        // LLVM). Native `call`/`call_indirect` sites carry their own
        // signature, so this alone wouldn't fail to link — but the callee
        // would silently reinterpret whatever bit pattern the caller passed
        // (issue #185's divergence class). The interpreter is dynamically
        // typed and unaffected; only native codegen needs this guard.
        // The message and code come from the block itself: the advice for an
        // untyped parameter ("declare concrete types") is wrong for a construct
        // with no native lowering at all, such as `ارمِ` (issue #181).
        if let Some(block) = &func.native_block {
            return Err(CodegenError::with_code(
                block.message.clone(),
                block.code.clone(),
            ));
        }

        self.var_map.clear();
        self.block_map.clear();
        self.name_counter = 0;

        let func_name = mangle_function_name(&func.id.0, &self.user_function_names);
        self.current_func = Some(func_name.clone());
        self.current_return_type = func.return_type.clone();

        for (i, param) in func.params.iter().enumerate() {
            let param_name = format!("%arg.{}", i);
            self.var_map.insert(param.id.0, param_name);
            self.var_types.insert(param.id.0, param.ty.clone());
        }

        for (i, block) in func.blocks.iter().enumerate() {
            let block_label = if i == 0 {
                "entry".to_string()
            } else if let Some(ref label) = block.label {
                format!("{}.{}", sanitize_label(label), block.id.0)
            } else {
                format!("bb{}", block.id.0)
            };
            self.block_map.insert(block.id.0, block_label);
        }

        let return_type = self.type_mapper.map_type(&func.return_type);
        let params: Vec<String> = func
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| format!("{} %arg.{}", self.type_mapper.map_param_type(&p.ty), i))
            .collect();

        emit!(
            self,
            "define {} @{}({}) {{",
            return_type,
            func_name,
            params.join(", ")
        );

        // Flag to track if we need to emit global string init and/or the
        // __global_init__ call
        let needs_global_init = func_name == "__main__"
            && (!self.global_string_inits.is_empty()
                || !self.global_optional_inits.is_empty()
                || self.has_global_init);

        for (i, block) in func.blocks.iter().enumerate() {
            // For __main__, emit global string initialization at the start of the first block
            if needs_global_init && i == 0 {
                let label = self.get_block(block.id)?;
                emit!(self, "{}:", label);
                self.emit_global_string_init()?;
                self.emit_global_optional_init()?;
                // String globals must be initialized first: __global_init__
                // may store string values into globals it depends on.
                if self.has_global_init {
                    emit!(self, "  call void @__global_init__()");
                }
                // Continue with block instructions
                for inst in &block.instructions {
                    self.emit_instruction(inst)?;
                }
                if !block.has_terminator() {
                    emit!(self, "  unreachable");
                }
            } else {
                self.emit_block(block)?;
            }
        }

        emit!(self, "}}");
        emit!(self);

        self.current_func = None;
        Ok(())
    }

    fn emit_block(&mut self, block: &BasicBlock) -> Result<(), CodegenError> {
        let label = self.get_block(block.id)?;
        emit!(self, "{}:", label);

        for inst in &block.instructions {
            self.emit_instruction(inst)?;
        }

        if !block.has_terminator() {
            emit!(self, "  unreachable");
        }

        Ok(())
    }

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
                let result_type = match op {
                    BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge => IrType::Bool,
                    BinaryOp::And | BinaryOp::Or => IrType::Bool,
                    _ => ty.clone(),
                };
                self.var_types.insert(dest.0, result_type);
            }

            Instruction::Unary {
                dest,
                op,
                operand,
                ty,
            } => {
                self.emit_unary(*dest, *op, *operand, ty)?;
                let result_type = match op {
                    UnaryOp::Not => IrType::Bool,
                    UnaryOp::Neg | UnaryOp::BitNot => ty.clone(),
                };
                self.var_types.insert(dest.0, result_type);
            }

            Instruction::IntToFloat { dest, src } => {
                let dest_name = self.get_or_create_var(*dest);
                let src_name = self.get_var(*src)?;
                emit!(self, "  {} = sitofp i64 {} to double", dest_name, src_name);
                self.var_types.insert(dest.0, IrType::Float);
            }

            Instruction::BoolToInt { dest, src } => {
                let dest_name = self.get_or_create_var(*dest);
                let src_name = self.get_var(*src)?;
                emit!(self, "  {} = zext i1 {} to i64", dest_name, src_name);
                self.var_types.insert(dest.0, IrType::Int);
            }

            Instruction::FloatToInt { dest, src } => {
                let dest_name = self.get_or_create_var(*dest);
                let src_name = self.get_var(*src)?;
                emit!(self, "  {} = fptosi double {} to i64", dest_name, src_name);
                self.var_types.insert(dest.0, IrType::Int);
            }

            Instruction::ToString { dest, src } => {
                let dest_name = self.get_or_create_var(*dest);
                let src_name = self.get_var(*src)?;
                let src_type = self.var_types.get(&src.0).cloned();
                match &src_type {
                    Some(IrType::Float) => {
                        writeln!(
                            self.output,
                            "  {} = call ptr @trq_float_to_string(double {})",
                            dest_name, src_name
                        )
                        .unwrap();
                    }
                    Some(IrType::Bool) => {
                        // Same `zeroext` requirement as @trq_print_bool: the
                        // Rust side is `extern "C" fn(bool)`, which admits only
                        // 0 or 1.
                        writeln!(
                            self.output,
                            "  {} = call ptr @trq_bool_to_string(i1 zeroext {})",
                            dest_name, src_name
                        )
                        .unwrap();
                    }
                    Some(IrType::String) | Some(IrType::Ptr(_)) => {
                        emit!(self, "  {} = bitcast ptr {} to ptr", dest_name, src_name);
                    }
                    _ => {
                        emit!(
                            self,
                            "  {} = call ptr @trq_int_to_string(i64 {})",
                            dest_name,
                            src_name
                        );
                    }
                }
                self.var_types.insert(dest.0, IrType::String);
            }

            Instruction::Bitcast { dest, src, to_ty } => {
                let dest_name = self.get_or_create_var(*dest);
                let src_name = self.get_var(*src)?;
                let to_type = self.type_mapper.map_type(to_ty);
                emit!(
                    self,
                    "  {} = bitcast ptr {} to {}",
                    dest_name,
                    src_name,
                    to_type
                );
            }

            Instruction::Alloca { dest, ty } => {
                let dest_name = self.get_or_create_var(*dest);
                let llvm_ty = self.type_mapper.map_type(ty);
                self.var_types
                    .insert(dest.0, IrType::Ptr(Box::new(ty.clone())));
                emit!(self, "  {} = alloca {}", dest_name, llvm_ty);
            }

            Instruction::Load { dest, ptr, ty } => {
                let dest_name = self.get_or_create_var(*dest);
                let ptr_name = self.get_var(*ptr)?;
                let llvm_ty = self.type_mapper.map_type(ty);
                self.var_types.insert(dest.0, ty.clone());
                emit!(self, "  {} = load {}, ptr {}", dest_name, llvm_ty, ptr_name);
            }

            Instruction::Store { ptr, value } => {
                let ptr_name = self.get_var(*ptr)?;
                let value_name = self.get_var(*value)?;
                let val_type = self
                    .var_types
                    .get(&value.0)
                    .cloned()
                    .or_else(|| {
                        self.var_types.get(&ptr.0).and_then(|t| {
                            if let IrType::Ptr(inner) = t {
                                Some((**inner).clone())
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));

                // An optional `T?` lowers to `Ptr(T)`, so a slot holding one is
                // `Ptr(Ptr(T))`. Storing a bare scalar into it wrote the raw i64
                // where a pointer belongs — valid LLVM under opaque pointers, so
                // nothing rejected it, and `عدد? = 0` then compared equal to
                // `لا_شيء` because both bit patterns are zero (#185). Box the
                // scalar so the null test is a genuine pointer test.
                let slot_ty = match self.var_types.get(&ptr.0) {
                    Some(IrType::Ptr(inner)) => Some((**inner).clone()),
                    _ => None,
                };
                if let Some(slot_ty) = slot_ty {
                    if Self::needs_boxing(&val_type, &slot_ty) {
                        let boxed = self.emit_boxed_scalar(&value_name, &val_type);
                        emit!(self, "  store ptr {}, ptr {}", boxed, ptr_name);
                        return Ok(());
                    }
                }

                let llvm_ty = self.type_mapper.map_type(&val_type);
                emit!(self, "  store {} {}, ptr {}", llvm_ty, value_name, ptr_name);
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
                emit!(
                    self,
                    "  {} = getelementptr inbounds {}, ptr {}, i64 {}",
                    dest_name,
                    llvm_ty,
                    ptr_name,
                    index_name
                );
            }

            Instruction::Jump { target } => {
                let target_label = self.get_block(*target)?;
                emit!(self, "  br label %{}", target_label);
            }

            Instruction::Branch {
                cond,
                then_block,
                else_block,
            } => {
                let cond_name = self.get_var(*cond)?;
                let then_label = self.get_block(*then_block)?;
                let else_label = self.get_block(*else_block)?;
                emit!(
                    self,
                    "  br i1 {}, label %{}, label %{}",
                    cond_name,
                    then_label,
                    else_label
                );
            }

            Instruction::Return { value } => {
                if let Some(val) = value {
                    let mut val_name = self.get_var(*val)?;
                    let declared = self.current_return_type.clone();
                    if let Some(val_ty) = self.var_types.get(&val.0).cloned() {
                        if Self::needs_boxing(&val_ty, &declared) {
                            val_name = self.emit_boxed_scalar(&val_name, &val_ty);
                        }
                    }
                    let ret_ty = self.type_mapper.map_type(&declared);
                    emit!(self, "  ret {} {}", ret_ty, val_name);
                } else {
                    emit!(self, "  ret void");
                }
            }

            Instruction::Call {
                dest,
                func,
                args,
                ret_ty,
            } => {
                let func_name = mangle_function_name(&func.0, &self.shadowing_names);

                // The callee's declared parameter types, where known: an argument
                // otherwise describes itself, which is wrong for a `عدد?`
                // parameter given a bare `0` — that has to be boxed first (#185).
                let declared_params = self.fn_param_types.get(&func_name).cloned();
                // Runtime conversions take the scalar itself, so a boxed optional
                // reaching one — as `"…" + مخزون` does inside a narrowed branch —
                // has to be loaded first, or the pointer is printed as the value.
                let runtime_scalar_param = match func_name.as_str() {
                    "trq_int_to_string" => Some(IrType::Int),
                    "trq_float_to_string" => Some(IrType::Float),
                    "trq_bool_to_string" => Some(IrType::Bool),
                    _ => None,
                };

                let mut args_str: Vec<String> = Vec::with_capacity(args.len());
                for (i, a) in args.iter().enumerate() {
                    let mut name = self.get_var(*a).unwrap_or("undef".to_string());
                    let mut arg_ty = self
                        .var_types
                        .get(&a.0)
                        .cloned()
                        .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));

                    if let Some(declared) = declared_params.as_ref().and_then(|p| p.get(i)) {
                        if Self::needs_boxing(&arg_ty, declared) {
                            name = self.emit_boxed_scalar(&name, &arg_ty);
                            arg_ty = declared.clone();
                        }
                    } else if let Some(scalar) = runtime_scalar_param.as_ref().filter(|_| i == 0) {
                        if matches!(&arg_ty, IrType::Ptr(inner) if **inner == *scalar) {
                            name = self.emit_unboxed_scalar(&name, scalar);
                            arg_ty = scalar.clone();
                        }
                    }

                    let llvm_ty = self.type_mapper.map_param_type(&arg_ty);
                    args_str.push(format!("{} {}", llvm_ty, name));
                }
                let ret_type = self.type_mapper.map_type(ret_ty);

                let is_void = matches!(ret_ty, IrType::Void);

                if let Some(d) = dest {
                    if is_void {
                        writeln!(
                            self.output,
                            "  call {} @{}({})",
                            ret_type,
                            func_name,
                            args_str.join(", ")
                        )
                        .unwrap();
                    } else {
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
                        self.var_types.insert(d.0, ret_ty.clone());
                    }
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
                // The call site's LLVM signature comes entirely from static
                // types. If the callee value's recorded type isn't a
                // function signature (e.g. a lambda reaching here through
                // an `أي`-typed slot), the emitted `call` would carry a
                // wrong return/argument signature and the callee would
                // reinterpret raw bits — silent corruption (issue #185's
                // divergence class). The interpreter dispatches on runtime
                // values and handles this fine; only native codegen must
                // reject it.
                if !matches!(
                    self.var_types.get(&func_ptr.0),
                    Some(IrType::Function { .. })
                ) {
                    return Err(CodegenError::with_code(
                        "الترجمة الأصلية لا تدعم استدعاء قيمة دالة بدون توقيع معروف (مثل قيمة من النوع 'أي'): صرّح بنوع الدالة، أو شغّل البرنامج بالمفسّر (tarqeem run)"
                            .to_string(),
                        ERR_UNTYPED_INDIRECT_CALL.to_string(),
                    ));
                }
                let func_ptr_name = self.get_var(*func_ptr)?;
                let args_str: Vec<String> = args
                    .iter()
                    .map(|a| {
                        let name = self.get_var(*a).unwrap_or("undef".to_string());
                        let arg_ty = self
                            .var_types
                            .get(&a.0)
                            .cloned()
                            .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));
                        let llvm_ty = self.type_mapper.map_param_type(&arg_ty);
                        format!("{} {}", llvm_ty, name)
                    })
                    .collect();
                let ret_type = self.type_mapper.map_type(ret_ty);
                let is_void = matches!(ret_ty, IrType::Void);

                if let Some(d) = dest {
                    if is_void {
                        writeln!(
                            self.output,
                            "  call {} {}({})",
                            ret_type,
                            func_ptr_name,
                            args_str.join(", ")
                        )
                        .unwrap();
                    } else {
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
                        self.var_types.insert(d.0, ret_ty.clone());
                    }
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
                // Sized off the registered struct layout, which pads each field to
                // its alignment: summing bare field sizes under-allocated, so
                // `{ ptr, i1, i64 }` asked for 17 bytes while its `i64` is stored
                // at offset 16 — a seven-byte overrun on every instance.
                let size = self.type_mapper.type_size(&IrType::Struct(class.clone()));

                writeln!(
                    self.output,
                    "  {} = call ptr @trq_alloc(i64 {})",
                    dest_name, size
                )
                .unwrap();

                // Installed before the constructor runs, so a constructor
                // calling a virtual method dispatches on the class being built —
                // matching the interpreter, which stamps `class_id` at
                // construction. A class with no virtual members emits no vtable
                // global; word 0 then stays `trq_alloc`'s zero, and no call site
                // can load it.
                if let Some(vtable) = self.vtable_globals.get(&class.0).cloned() {
                    writeln!(self.output, "  store ptr {}, ptr {}", vtable, dest_name).unwrap();
                }

                self.var_types
                    .insert(dest.0, IrType::Ptr(Box::new(IrType::Struct(class.clone()))));
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

                // Get the object's actual class type and calculate field offset
                // including inherited fields
                let obj_ty = self.var_types.get(&object.0).cloned();
                let (struct_ty, actual_index) =
                    self.get_field_access_info(&field.class.0, &field.name, field.index, &obj_ty);

                let ptr_name = self.fresh_name("field.ptr");
                writeln!(
                    self.output,
                    "  {} = getelementptr inbounds {}, ptr {}, i32 0, i32 {}",
                    ptr_name, struct_ty, obj_name, actual_index
                )
                .unwrap();
                writeln!(
                    self.output,
                    "  {} = load {}, ptr {}",
                    dest_name, llvm_ty, ptr_name
                )
                .unwrap();
                // Register the destination variable's type for correct argument passing
                self.var_types.insert(dest.0, ty.clone());
            }

            Instruction::SetField {
                object,
                field,
                value,
            } => {
                let obj_name = self.get_var(*object)?;
                let val_name = self.get_var(*value)?;

                // Get the object's actual class type and calculate field offset
                // including inherited fields
                let obj_ty = self.var_types.get(&object.0).cloned();
                let (struct_ty, actual_index) =
                    self.get_field_access_info(&field.class.0, &field.name, field.index, &obj_ty);

                let ptr_name = self.fresh_name("field.ptr");
                writeln!(
                    self.output,
                    "  {} = getelementptr inbounds {}, ptr {}, i32 0, i32 {}",
                    ptr_name, struct_ty, obj_name, actual_index
                )
                .unwrap();
                let mut val_name = val_name;
                let mut val_type = self.var_types.get(&value.0).cloned().unwrap_or(IrType::Int);

                // A field declared `عدد?` holds a boxed scalar like any other
                // optional, so assigning a bare scalar to it boxes here too —
                // otherwise `هذا.رصيد = 0` stores a raw 0 and the field then
                // compares equal to `لا_شيء` (#185).
                if let Some(declared) = self.declared_field_type(&field.class.0, &field.name) {
                    if Self::needs_boxing(&val_type, &declared) {
                        val_name = self.emit_boxed_scalar(&val_name, &val_type);
                        val_type = declared;
                    }
                }

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
                virtual_dispatch,
            } => {
                let obj_name = self.get_var(*object)?;

                // `الأصل.م()` carries `virtual_dispatch: false` and must keep it:
                // dispatching a super call virtually would resolve back to the
                // override that made it and recurse until the stack ends.
                // Receivers with no class table — `أي`, `ميثاق` (#209), object
                // literals — fall back to the definer's body as before.
                let slot = virtual_dispatch
                    .then(|| self.receiver_vtable_slot(object.0, &method.name))
                    .flatten();

                let method_symbol = mangle_function_name(
                    &format!("{}::{}", method.class.0, method.name),
                    &self.user_function_names,
                );

                let callee = match slot {
                    Some(index) => {
                        let vtable_ptr = self.fresh_name("vtable.ptr");
                        emit!(self, "  {} = load ptr, ptr {}", vtable_ptr, obj_name);

                        let method_ptr_ptr = self.fresh_name("method.ptr.ptr");
                        emit!(
                            self,
                            "  {} = getelementptr inbounds ptr, ptr {}, i32 {}",
                            method_ptr_ptr,
                            vtable_ptr,
                            index
                        );

                        let method_ptr = self.fresh_name("method.ptr");
                        emit!(self, "  {} = load ptr, ptr {}", method_ptr, method_ptr_ptr);
                        method_ptr
                    }
                    None => format!("@{}", method_symbol),
                };

                // Declared parameter 0 is the receiver, which is not in `args`, so
                // the argument at `i` is declared at `i + 1`. Without this, a
                // method taking `س: عدد?` called as `م.افحص(0)` receives a raw
                // scalar in a pointer slot (#185).
                let declared_params = self.fn_param_types.get(&method_symbol).cloned();

                let mut all_args = vec![format!("ptr {}", obj_name)];
                for (i, arg) in args.iter().enumerate() {
                    let mut arg_name = self.get_var(*arg)?;
                    let mut arg_ty = self.var_types.get(&arg.0).cloned().unwrap_or(IrType::Int);

                    if let Some(declared) = declared_params.as_ref().and_then(|p| p.get(i + 1)) {
                        if Self::needs_boxing(&arg_ty, declared) {
                            arg_name = self.emit_boxed_scalar(&arg_name, &arg_ty);
                            arg_ty = declared.clone();
                        }
                    }

                    let llvm_ty = self.type_mapper.map_param_type(&arg_ty);
                    all_args.push(format!("{} {}", llvm_ty, arg_name));
                }

                let ret_type = self.type_mapper.map_type(ret_ty);
                let is_void = matches!(ret_ty, IrType::Void);

                if let Some(d) = dest {
                    if is_void {
                        // Void calls cannot have a return value assigned
                        writeln!(
                            self.output,
                            "  call {} {}({})",
                            ret_type,
                            callee,
                            all_args.join(", ")
                        )
                        .unwrap();
                    } else {
                        let dest_name = self.get_or_create_var(*d);
                        writeln!(
                            self.output,
                            "  {} = call {} {}({})",
                            dest_name,
                            ret_type,
                            callee,
                            all_args.join(", ")
                        )
                        .unwrap();
                        self.var_types.insert(d.0, ret_ty.clone());
                    }
                } else {
                    writeln!(
                        self.output,
                        "  call {} {}({})",
                        ret_type,
                        callee,
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
                let obj_name = self.get_var(*object)?;

                let vtable_ptr = self.fresh_name("vtable.ptr");
                emit!(self, "  {} = load ptr, ptr {}", vtable_ptr, obj_name);

                let method_ptr_ptr = self.fresh_name("method.ptr.ptr");
                emit!(
                    self,
                    "  {} = getelementptr inbounds ptr, ptr {}, i32 {}",
                    method_ptr_ptr,
                    vtable_ptr,
                    method_index
                );

                let method_ptr = self.fresh_name("method.ptr");
                emit!(self, "  {} = load ptr, ptr {}", method_ptr, method_ptr_ptr);

                let mut all_args = vec![format!("ptr {}", obj_name)];
                for arg in args {
                    let arg_name = self.get_var(*arg)?;
                    let arg_ty = self.var_types.get(&arg.0).cloned().unwrap_or(IrType::Int);
                    let llvm_ty = self.type_mapper.map_param_type(&arg_ty);
                    all_args.push(format!("{} {}", llvm_ty, arg_name));
                }

                let ret_type = self.type_mapper.map_type(ret_ty);
                let is_void = matches!(ret_ty, IrType::Void);

                if let Some(d) = dest {
                    if is_void {
                        emit!(
                            self,
                            "  call {} {}({})",
                            ret_type,
                            method_ptr,
                            all_args.join(", ")
                        );
                    } else {
                        let dest_name = self.get_or_create_var(*d);
                        emit!(
                            self,
                            "  {} = call {} {}({})",
                            dest_name,
                            ret_type,
                            method_ptr,
                            all_args.join(", ")
                        );
                        self.var_types.insert(d.0, ret_ty.clone());
                    }
                } else {
                    emit!(
                        self,
                        "  call {} {}({})",
                        ret_type,
                        method_ptr,
                        all_args.join(", ")
                    );
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

                self.var_types.insert(
                    dest.0,
                    IrType::Array(Box::new(elem_ty.clone()), len as usize),
                );

                writeln!(
                    self.output,
                    "  {} = call ptr @trq_array_new(i64 {}, i64 {})",
                    dest_name, len, elem_size
                )
                .unwrap();

                for (i, elem) in elements.iter().enumerate() {
                    let elem_name = self.get_var(*elem)?;
                    let actual_elem_ty = self
                        .var_types
                        .get(&elem.0)
                        .cloned()
                        .unwrap_or(elem_ty.clone());
                    let llvm_elem_ty = self.type_mapper.map_type(&actual_elem_ty);
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
                // `ArrayLen` is polymorphic: every interpreting backend branches on
                // the runtime value and counts characters for a string. Codegen has
                // no runtime tag, so it must dispatch on the operand's IR type here
                // — and getting it wrong is silent, not loud, because `TrqString`
                // and `TrqArray` are both `#[repr(C)]` with `len` first, so reading
                // one as the other returns the *byte* count instead of trapping
                // (#185). An unknown operand type keeps the array symbol: that is
                // today's behaviour, and the catch-all arm is where type-directed
                // fixes break (#222).
                let symbol = if Self::is_string_operand(self.var_types.get(&array.0)) {
                    "trq_string_len_chars"
                } else {
                    "trq_array_len"
                };
                writeln!(
                    self.output,
                    "  {} = call i64 @{}(ptr {})",
                    dest_name, symbol, array_name
                )
                .unwrap();
                // ArrayLen returns i64 (Int type)
                self.var_types.insert(dest.0, IrType::Int);
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

                // Indexing a string yields a one-character string, exactly as the
                // interpreter's `Value::String` arm does. Routing it through
                // `trq_array_get` instead read the string's byte pointer as an
                // element table and aborted with "الوصول إلى مصفوفة فارغة" — the
                // element-access half of the same polymorphism gap as `ArrayLen`
                // (#185). `لكل ح في نص` is the common way to reach it.
                if Self::is_string_operand(self.var_types.get(&array.0)) {
                    writeln!(
                        self.output,
                        "  {} = call ptr @trq_string_char_at(ptr {}, i64 {})",
                        dest_name, array_name, index_name
                    )
                    .unwrap();
                    self.var_types.insert(dest.0, IrType::String);
                    return Ok(());
                }

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
                self.var_types.insert(dest.0, elem_ty.clone());
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
                let val_type = self.var_types.get(&value.0).cloned().unwrap_or(IrType::Int);
                let llvm_ty = self.type_mapper.map_type(&val_type);
                writeln!(
                    self.output,
                    "  store {} {}, ptr {}",
                    llvm_ty, value_name, elem_ptr
                )
                .unwrap();
            }

            Instruction::ArrayPush {
                array,
                value,
                elem_ty,
            } => {
                let array_name = self.get_var(*array)?;
                let value_name = self.get_var(*value)?;
                let llvm_ty = self.type_mapper.map_type(elem_ty);

                let elem_size = match elem_ty {
                    IrType::Bool => 1,
                    IrType::Int => 8,
                    IrType::Float => 8,
                    IrType::Ptr(_) | IrType::String | IrType::Array(_, _) | IrType::Struct(_) => 8,
                    _ => 8,
                };

                let temp_ptr = self.fresh_name("push.tmp");
                emit!(self, "  {} = alloca {}", temp_ptr, llvm_ty);

                emit!(self, "  store {} {}, ptr {}", llvm_ty, value_name, temp_ptr);

                emit!(
                    self,
                    "  call void @trq_array_push(ptr {}, ptr {}, i64 {})",
                    array_name,
                    temp_ptr,
                    elem_size
                );
            }

            Instruction::StringConcat { dest, left, right } => {
                let dest_name = self.get_or_create_var(*dest);
                let left_name = self.get_var(*left)?;
                let right_name = self.get_var(*right)?;
                emit!(
                    self,
                    "  {} = call ptr @trq_string_concat(ptr {}, ptr {})",
                    dest_name,
                    left_name,
                    right_name
                );
                self.var_types.insert(dest.0, IrType::String);
            }

            Instruction::TryBegin { catch_block } => {
                let catch_label = self.get_block(*catch_block)?;
                emit!(self, "  ; try_begin catch={}", catch_label);
            }

            Instruction::TryEnd => {
                emit!(self, "  ; try_end");
            }

            Instruction::Throw { exception } => {
                let exc_name = self.get_var(*exception)?;
                emit!(self, "  call void @trq_throw(ptr {})", exc_name);
                emit!(self, "  unreachable");
            }

            Instruction::GetException { dest } => {
                let dest_name = self.get_or_create_var(*dest);
                emit!(self, "  {} = call ptr @trq_get_exception()", dest_name);
                // Exception is returned as a pointer
                self.var_types
                    .insert(dest.0, IrType::Ptr(Box::new(IrType::Void)));
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

                emit!(
                    self,
                    "  {} = phi {} {}",
                    dest_name,
                    llvm_ty,
                    entries.join(", ")
                );
                // GlobalStore infers the stored type from var_types; without
                // this a ptr-typed phi (e.g. string ternary) falls back to i64
                self.var_types.insert(dest.0, ty.clone());
            }

            Instruction::Print { value } => {
                let val_name = self.get_var(*value)?;
                let var_type = self.var_types.get(&value.0).cloned();
                match &var_type {
                    // A boxed scalar optional is `Ptr(Int|Float|Bool)`. Handing it
                    // to `trq_print`, which expects a `TrqString*`, segfaulted
                    // (#185). Print `لا_شيء` for a null one and the pointee
                    // otherwise, matching the interpreter.
                    Some(IrType::Ptr(pointee))
                        if matches!(**pointee, IrType::Int | IrType::Float | IrType::Bool) =>
                    {
                        let kind = match **pointee {
                            IrType::Float => 1,
                            IrType::Bool => 2,
                            _ => 0,
                        };
                        emit!(
                            self,
                            "  call void @trq_print_optional_scalar(ptr {}, i64 {})",
                            val_name,
                            kind
                        );
                    }
                    Some(IrType::String) | Some(IrType::Ptr(_)) => {
                        emit!(self, "  call void @trq_print(ptr {})", val_name);
                    }
                    Some(IrType::Float) => {
                        emit!(self, "  call void @trq_print_float(double {})", val_name);
                    }
                    Some(IrType::Bool) => {
                        // `zeroext` is load-bearing, not decoration: an `i1` whose
                        // upper byte bits are don't-care (any NOT, lowered to
                        // `xorb $-1`) otherwise reaches Rust's `bool` as 254 or
                        // 255 — an invalid bit pattern whose branch arithmetic
                        // walks off into .rodata (#266 follow-up).
                        emit!(self, "  call void @trq_print_bool(i1 zeroext {})", val_name);
                    }
                    Some(IrType::Array(_, _)) => {
                        emit!(self, "  call void @trq_print_array(ptr {})", val_name);
                    }
                    _ => {
                        emit!(self, "  call void @trq_print_int(i64 {})", val_name);
                    }
                }
                emit!(self, "  call void @trq_print_newline()");
            }

            Instruction::GlobalLoad { dest, name, ty } => {
                let dest_name = self.get_or_create_var(*dest);
                let llvm_type = self.type_mapper.map_type(ty);
                let global_name = mangle_name(name);

                // String globals store TrqString* (initialized at program start or from assignments)
                // Just load the pointer directly - no wrapping needed
                emit!(
                    self,
                    "  {} = load {}, ptr @{}",
                    dest_name,
                    llvm_type,
                    global_name
                );
                self.var_types.insert(dest.0, ty.clone());
            }

            Instruction::GlobalStore { name, value } => {
                let mut value_name = self.get_var(*value)?;
                let mut value_ty = self.var_types.get(&value.0).cloned().unwrap_or(IrType::Int);

                // Assigning a scalar to an optional global boxes, exactly as it
                // does for a local (#185).
                if let Some(declared) = self.global_types.get(name).cloned() {
                    if Self::needs_boxing(&value_ty, &declared) {
                        value_name = self.emit_boxed_scalar(&value_name, &value_ty);
                        value_ty = declared;
                    }
                }

                let llvm_type = self.type_mapper.map_type(&value_ty);
                let global_name = mangle_name(name);
                writeln!(
                    self.output,
                    "  store {} {}, ptr @{}",
                    llvm_type, value_name, global_name
                )
                .unwrap();
            }

            Instruction::Copy { dest, src, ty } => {
                // Copy is a simple value transfer - ensure dest is registered
                // If source exists in var_map, alias it; otherwise create a new variable
                if let Some(src_name) = self.var_map.get(&src.0).cloned() {
                    self.var_map.insert(dest.0, src_name);
                } else {
                    // Source not in var_map - create destination variable
                    self.get_or_create_var(*dest);
                }
                // Also copy the type information - use provided type or try to get from source
                if let Some(src_ty) = self.var_types.get(&src.0).cloned() {
                    self.var_types.insert(dest.0, src_ty);
                } else {
                    self.var_types.insert(dest.0, ty.clone());
                }
            }

            Instruction::NewEnumVariant {
                dest,
                variant,
                fields,
            } => {
                // Enum layout: { i64 discriminant, field1, field2, ... }
                // Calculate total size: discriminant (8 bytes) + all field sizes
                let mut total_size = 8u64; // discriminant
                for field_var in fields.iter() {
                    if let Some(field_ty) = self.var_types.get(&field_var.0) {
                        total_size += self.type_mapper.type_size(field_ty);
                    } else {
                        total_size += 8; // default pointer size
                    }
                }
                let total_size = if total_size == 8 { 16 } else { total_size }; // Minimum 16 bytes

                // Allocate memory for enum
                let enum_ptr = self.new_temp();
                emit!(
                    self,
                    "  {} = call ptr @trq_alloc(i64 {})",
                    enum_ptr,
                    total_size
                );

                // Store discriminant at offset 0
                emit!(
                    self,
                    "  store i64 {}, ptr {}",
                    variant.discriminant,
                    enum_ptr
                );

                // Store field values at subsequent offsets
                // Use consistent 8-byte alignment for all fields to match GetVariantField
                let mut offset = 8u64; // Start after discriminant
                for field_var in fields.iter() {
                    let field_name = self.get_var(*field_var)?;
                    let field_ty = self
                        .var_types
                        .get(&field_var.0)
                        .cloned()
                        .unwrap_or(IrType::Int);
                    let llvm_ty = self.type_mapper.map_type(&field_ty);

                    // GEP to field offset
                    let field_ptr = self.new_temp();
                    emit!(
                        self,
                        "  {} = getelementptr i8, ptr {}, i64 {}",
                        field_ptr,
                        enum_ptr,
                        offset
                    );

                    // Store field value
                    emit!(
                        self,
                        "  store {} {}, ptr {}",
                        llvm_ty,
                        field_name,
                        field_ptr
                    );

                    // Use 8-byte alignment for all fields for consistency with GetVariantField
                    offset += 8;
                }

                // Result is the pointer to the enum
                self.var_map.insert(dest.0, enum_ptr.clone());
                self.var_types.insert(
                    dest.0,
                    IrType::Enum(crate::ir::EnumId(variant.enum_id.0.clone())),
                );
            }

            Instruction::GetDiscriminant { dest, value } => {
                let dest_name = self.get_or_create_var(*dest);
                let value_name = self.get_var(*value)?;

                // Load discriminant from offset 0 of enum pointer
                emit!(self, "  {} = load i64, ptr {}", dest_name, value_name);

                self.var_types.insert(dest.0, IrType::Int);
            }

            Instruction::GetVariantField {
                dest,
                value,
                variant: _,
                field_index,
                ty,
            } => {
                let dest_name = self.get_or_create_var(*dest);
                let value_name = self.get_var(*value)?;
                let llvm_ty = self.type_mapper.map_type(ty);

                // Calculate field offset: 8 (discriminant) + field_index * 8
                // All fields use 8-byte alignment for consistency with NewEnumVariant
                let offset = 8 + (*field_index as u64) * 8;

                // GEP to field offset
                let field_ptr = self.new_temp();
                emit!(
                    self,
                    "  {} = getelementptr i8, ptr {}, i64 {}",
                    field_ptr,
                    value_name,
                    offset
                );

                // Load field value
                emit!(
                    self,
                    "  {} = load {}, ptr {}",
                    dest_name,
                    llvm_ty,
                    field_ptr
                );

                self.var_types.insert(dest.0, ty.clone());
            }

            Instruction::Nop => {}
        }

        Ok(())
    }

    fn emit_const(
        &mut self,
        dest: VarId,
        value: &Constant,
        ty: &IrType,
    ) -> Result<(), CodegenError> {
        let dest_name = self.get_or_create_var(dest);
        self.var_types.insert(dest.0, ty.clone());

        match value {
            Constant::Null => {
                emit!(self, "  {} = bitcast ptr null to ptr", dest_name);
            }
            Constant::Function(name) => {
                let mangled = mangle_function_name(name, &self.shadowing_names);
                emit!(self, "  {} = bitcast ptr @{} to ptr", dest_name, mangled);
            }
            Constant::Bool(b) => {
                let val = if *b { "true" } else { "false" };
                emit!(
                    self,
                    "  {} = select i1 {}, i1 true, i1 false",
                    dest_name,
                    val
                );
            }
            Constant::Int(i) => {
                emit!(self, "  {} = add i64 {}, 0", dest_name, i);
            }
            Constant::Float(f) => {
                // LLVM requires a decimal point in float literals (1.0e4, not 1e4)
                let float_str = format!("{:e}", f);
                let float_str = if !float_str.contains('.') {
                    float_str.replace("e", ".0e")
                } else {
                    float_str
                };
                emit!(self, "  {} = fadd double {}, 0.0", dest_name, float_str);
            }
            Constant::String(idx) => {
                if let Some((global, len)) = self.string_globals.get(idx) {
                    let tmp_ptr = format!("%tmp_strptr_{}", self.name_counter);
                    self.name_counter += 1;
                    emit!(
                        self,
                        "  {} = getelementptr [0 x i8], ptr {}, i64 0, i64 0",
                        tmp_ptr,
                        global
                    );
                    emit!(
                        self,
                        "  {} = call ptr @trq_string_new(ptr {}, i64 {})",
                        dest_name,
                        tmp_ptr,
                        len
                    );
                } else {
                    emit!(
                        self,
                        "  {} = call ptr @trq_string_new(ptr null, i64 0)",
                        dest_name
                    );
                }
            }
        }

        Ok(())
    }

    fn emit_binary(
        &mut self,
        dest: VarId,
        op: BinaryOp,
        left: VarId,
        right: VarId,
        ty: &IrType,
    ) -> Result<(), CodegenError> {
        let dest_name = self.get_or_create_var(dest);
        let mut left_name = self.get_var(left)?;
        let mut right_name = self.get_var(right)?;

        // Once the analyzer narrows `س` after `إذا (س != لا_شيء)`, the branch may
        // use it as a plain scalar — but it is still a boxed pointer here, so the
        // value has to be loaded back out (#185). Unbox only against a bare
        // scalar operand: a comparison with `لا_شيء` has `Ptr(Void)` on the other
        // side and must stay a pointer test.
        let left_ty_raw = self.var_types.get(&left.0).cloned();
        let right_ty_raw = self.var_types.get(&right.0).cloned();
        let unboxes_against = |boxed: &Option<IrType>, other: &Option<IrType>| -> Option<IrType> {
            match (boxed, other) {
                (Some(IrType::Ptr(pointee)), Some(other_ty))
                    if **pointee == *other_ty
                        && matches!(**pointee, IrType::Int | IrType::Float | IrType::Bool) =>
                {
                    Some((**pointee).clone())
                }
                _ => None,
            }
        };

        if let Some(scalar) = unboxes_against(&left_ty_raw, &right_ty_raw) {
            left_name = self.emit_unboxed_scalar(&left_name, &scalar);
            self.var_types.insert(left.0, scalar);
        } else if let Some(scalar) = unboxes_against(&right_ty_raw, &left_ty_raw) {
            right_name = self.emit_unboxed_scalar(&right_name, &scalar);
            self.var_types.insert(right.0, scalar);
        }

        // Check operand type from var_types - more reliable than ty parameter
        // This handles cases where the IR builder passes incorrect type info
        let operand_ty = self.var_types.get(&left.0).cloned().unwrap_or(ty.clone());

        // For arithmetic operations, use the actual operand type
        let is_arithmetic = matches!(
            op,
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
        );

        if is_arithmetic && operand_ty == IrType::Float {
            let instruction = match op {
                BinaryOp::Add => "fadd double",
                BinaryOp::Sub => "fsub double",
                BinaryOp::Mul => "fmul double",
                BinaryOp::Div => "fdiv double",
                BinaryOp::Mod => "frem double",
                _ => unreachable!(),
            };
            writeln!(
                self.output,
                "  {} = {} {}, {}",
                dest_name, instruction, left_name, right_name
            )
            .unwrap();
            // Track the result type
            self.var_types.insert(dest.0, IrType::Float);
            return Ok(());
        }

        // For comparison operations, check operand type (not result type which is always Bool)
        let is_comparison = matches!(
            op,
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        );

        if is_comparison {
            // `Binary.ty` is the *result* type, always Bool for a comparison, so
            // the opcode table below cannot be trusted to describe the operands —
            // integer comparison only works there because that arm happens to
            // spell `i64`. Everything non-scalar has to be resolved here instead,
            // from the operand types codegen already tracks (#185).
            //
            // Two shapes need care. `لا_شيء` is emitted as `Ptr(Void)` and an
            // optional `T?` as `Ptr(T)`, so anything involving the null literal is
            // a pointer-identity test — but two optional *strings* compared with
            // each other must keep the value semantics the interpreter has, and so
            // route to the string path rather than to `icmp ptr`.
            let left_ty = self.var_types.get(&left.0).cloned();
            let right_ty = self.var_types.get(&right.0).cloned();

            let is_null_literal = |t: &Option<IrType>| matches!(t, Some(IrType::Ptr(inner)) if matches!(**inner, IrType::Void));
            let both_optional_strings = matches!(
                (&left_ty, &right_ty),
                (Some(IrType::Ptr(a)), Some(IrType::Ptr(b)))
                    if matches!(**a, IrType::String) && matches!(**b, IrType::String)
            );
            let is_reference = |t: &Option<IrType>| {
                matches!(
                    t,
                    Some(
                        IrType::Ptr(_)
                            | IrType::Struct(_)
                            | IrType::Enum(_)
                            | IrType::Function { .. }
                    )
                )
            };

            let operand_ty = if matches!(left_ty, Some(IrType::String)) || both_optional_strings {
                Some(IrType::String)
            } else if is_null_literal(&left_ty)
                || is_null_literal(&right_ty)
                || is_reference(&left_ty)
                || is_reference(&right_ty)
            {
                Some(IrType::Ptr(Box::new(IrType::Void)))
            } else {
                left_ty
            };

            if let Some(operand_ty) = operand_ty.as_ref() {
                match operand_ty {
                    IrType::String => {
                        // String comparison using runtime functions
                        match op {
                            BinaryOp::Eq => {
                                writeln!(
                                    self.output,
                                    "  {} = call i1 @trq_string_equals(ptr {}, ptr {})",
                                    dest_name, left_name, right_name
                                )
                                .unwrap();
                                return Ok(());
                            }
                            BinaryOp::Ne => {
                                let tmp = self.fresh_name("tmp.streq");
                                writeln!(
                                    self.output,
                                    "  {} = call i1 @trq_string_equals(ptr {}, ptr {})",
                                    tmp, left_name, right_name
                                )
                                .unwrap();
                                writeln!(self.output, "  {} = xor i1 {}, true", dest_name, tmp)
                                    .unwrap();
                                return Ok(());
                            }
                            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                                let cmp_result = self.fresh_name("tmp.strcmp");
                                writeln!(
                                    self.output,
                                    "  {} = call i64 @trq_string_compare(ptr {}, ptr {})",
                                    cmp_result, left_name, right_name
                                )
                                .unwrap();
                                let cmp_op = match op {
                                    BinaryOp::Lt => "icmp slt i64",
                                    BinaryOp::Le => "icmp sle i64",
                                    BinaryOp::Gt => "icmp sgt i64",
                                    BinaryOp::Ge => "icmp sge i64",
                                    _ => unreachable!(),
                                };
                                writeln!(
                                    self.output,
                                    "  {} = {} {}, 0",
                                    dest_name, cmp_op, cmp_result
                                )
                                .unwrap();
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                    IrType::Float => {
                        // Float comparison
                        let cmp_op = match op {
                            BinaryOp::Eq => "fcmp oeq double",
                            BinaryOp::Ne => "fcmp one double",
                            BinaryOp::Lt => "fcmp olt double",
                            BinaryOp::Le => "fcmp ole double",
                            BinaryOp::Gt => "fcmp ogt double",
                            BinaryOp::Ge => "fcmp oge double",
                            _ => unreachable!(),
                        };
                        writeln!(
                            self.output,
                            "  {} = {} {}, {}",
                            dest_name, cmp_op, left_name, right_name
                        )
                        .unwrap();
                        return Ok(());
                    }
                    // Pointer identity, which is what the interpreter gives for a
                    // class instance too. Enum values do not reach this arm — the
                    // IR types them as strings, so a direct `==` between them
                    // reads a discriminant as a `TrqString` header, before this
                    // change and after it.
                    IrType::Ptr(_)
                    | IrType::Struct(_)
                    | IrType::Enum(_)
                    | IrType::Function { .. } => {
                        let cmp_op = match op {
                            BinaryOp::Eq => "icmp eq ptr",
                            BinaryOp::Ne => "icmp ne ptr",
                            // Ordering two references is not meaningful; the
                            // analyzer rejects it, so reaching here is a bug in
                            // an earlier layer rather than user input.
                            _ => {
                                return Err(CodegenError::new(format!(
                                    "لا يمكن ترتيب مرجعين بالعامل {op:?} / cannot order two references with {op:?}"
                                )))
                            }
                        };
                        writeln!(
                            self.output,
                            "  {} = {} {}, {}",
                            dest_name, cmp_op, left_name, right_name
                        )
                        .unwrap();
                        return Ok(());
                    }
                    IrType::Bool => {
                        // Booleans are `i1`; the fallback table would compare them
                        // as `i64` and LLVM rejects the module outright.
                        let cmp_op = match op {
                            BinaryOp::Eq => "icmp eq i1",
                            BinaryOp::Ne => "icmp ne i1",
                            BinaryOp::Lt => "icmp ult i1",
                            BinaryOp::Le => "icmp ule i1",
                            BinaryOp::Gt => "icmp ugt i1",
                            BinaryOp::Ge => "icmp uge i1",
                            _ => unreachable!(),
                        };
                        writeln!(
                            self.output,
                            "  {} = {} {}, {}",
                            dest_name, cmp_op, left_name, right_name
                        )
                        .unwrap();
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        let instruction = match (op, ty) {
            (BinaryOp::Add, IrType::Int) => "add i64",
            (BinaryOp::Sub, IrType::Int) => "sub i64",
            (BinaryOp::Mul, IrType::Int) => "mul i64",
            (BinaryOp::Div, IrType::Int) => "sdiv i64",
            (BinaryOp::Mod, IrType::Int) => "srem i64",

            (BinaryOp::Add, IrType::Float) => "fadd double",
            (BinaryOp::Sub, IrType::Float) => "fsub double",
            (BinaryOp::Mul, IrType::Float) => "fmul double",
            (BinaryOp::Div, IrType::Float) => "fdiv double",
            (BinaryOp::Mod, IrType::Float) => "frem double",

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

            (BinaryOp::Eq, IrType::Bool) => "icmp eq i64",
            (BinaryOp::Ne, IrType::Bool) => "icmp ne i64",
            (BinaryOp::Lt, IrType::Bool) => "icmp slt i64",
            (BinaryOp::Le, IrType::Bool) => "icmp sle i64",
            (BinaryOp::Gt, IrType::Bool) => "icmp sgt i64",
            (BinaryOp::Ge, IrType::Bool) => "icmp sge i64",
            (BinaryOp::Eq, IrType::Int) => "icmp eq i64",
            (BinaryOp::Ne, IrType::Int) => "icmp ne i64",
            (BinaryOp::Lt, IrType::Int) => "icmp slt i64",
            (BinaryOp::Le, IrType::Int) => "icmp sle i64",
            (BinaryOp::Gt, IrType::Int) => "icmp sgt i64",
            (BinaryOp::Ge, IrType::Int) => "icmp sge i64",

            (BinaryOp::Eq, IrType::Float) => "fcmp oeq double",
            (BinaryOp::Ne, IrType::Float) => "fcmp one double",
            (BinaryOp::Lt, IrType::Float) => "fcmp olt double",
            (BinaryOp::Le, IrType::Float) => "fcmp ole double",
            (BinaryOp::Gt, IrType::Float) => "fcmp ogt double",
            (BinaryOp::Ge, IrType::Float) => "fcmp oge double",

            // String comparison using runtime functions
            (BinaryOp::Eq, IrType::String) => {
                writeln!(
                    self.output,
                    "  {} = call i1 @trq_string_equals(ptr {}, ptr {})",
                    dest_name, left_name, right_name
                )
                .unwrap();
                return Ok(());
            }
            (BinaryOp::Ne, IrType::String) => {
                // Not equals: negate the result of equals
                let tmp = self.fresh_name("tmp.streq");
                writeln!(
                    self.output,
                    "  {} = call i1 @trq_string_equals(ptr {}, ptr {})",
                    tmp, left_name, right_name
                )
                .unwrap();
                writeln!(self.output, "  {} = xor i1 {}, true", dest_name, tmp).unwrap();
                return Ok(());
            }
            (BinaryOp::Lt, IrType::String)
            | (BinaryOp::Le, IrType::String)
            | (BinaryOp::Gt, IrType::String)
            | (BinaryOp::Ge, IrType::String) => {
                // String comparison: compare < 0, <= 0, > 0, >= 0
                let cmp_result = self.fresh_name("tmp.strcmp");
                writeln!(
                    self.output,
                    "  {} = call i64 @trq_string_compare(ptr {}, ptr {})",
                    cmp_result, left_name, right_name
                )
                .unwrap();
                let cmp_op = match op {
                    BinaryOp::Lt => "icmp slt i64",
                    BinaryOp::Le => "icmp sle i64",
                    BinaryOp::Gt => "icmp sgt i64",
                    BinaryOp::Ge => "icmp sge i64",
                    _ => unreachable!(),
                };
                writeln!(
                    self.output,
                    "  {} = {} {}, 0",
                    dest_name, cmp_op, cmp_result
                )
                .unwrap();
                return Ok(());
            }

            // Pointer comparison (for object identity)
            (BinaryOp::Eq, IrType::Ptr(_)) => "icmp eq ptr",
            (BinaryOp::Ne, IrType::Ptr(_)) => "icmp ne ptr",

            (BinaryOp::And, IrType::Bool) => "and i1",
            (BinaryOp::Or, IrType::Bool) => "or i1",

            (BinaryOp::BitAnd, IrType::Int) => "and i64",
            (BinaryOp::BitOr, IrType::Int) => "or i64",
            (BinaryOp::BitXor, IrType::Int) => "xor i64",
            (BinaryOp::Shl, IrType::Int) => "shl i64",
            (BinaryOp::Shr, IrType::Int) => "ashr i64",

            _ => {
                return Err(CodegenError::with_code(
                    format!("عملية ثنائية غير مدعومة: {:?} على {:?}", op, ty),
                    ERR_LLVM_INTERNAL.to_string(),
                ));
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

    fn emit_unary(
        &mut self,
        dest: VarId,
        op: UnaryOp,
        operand: VarId,
        ty: &IrType,
    ) -> Result<(), CodegenError> {
        let dest_name = self.get_or_create_var(dest);
        let mut operand_name = self.get_var(operand)?;

        // A narrowed optional is still a boxed `Ptr(T)` here, so the value has to
        // be loaded back out before an integer opcode can touch it — the same
        // unboxing `emit_binary` does (#185). Without it, `بتات_نفي(س)` after
        // `إذا (س != لا_شيء)` emits `xor i64 %ptr, -1` and clang rejects the
        // module, while the interpreter and JIT both answer correctly.
        if let Some(IrType::Ptr(pointee)) = self.var_types.get(&operand.0).cloned() {
            let scalar = *pointee;
            if scalar == *ty && matches!(scalar, IrType::Int | IrType::Float | IrType::Bool) {
                operand_name = self.emit_unboxed_scalar(&operand_name, &scalar);
                self.var_types.insert(operand.0, scalar);
            }
        }

        match (op, ty) {
            (UnaryOp::Neg, IrType::Int) => {
                emit!(self, "  {} = sub i64 0, {}", dest_name, operand_name);
            }
            (UnaryOp::Neg, IrType::Float) => {
                emit!(self, "  {} = fneg double {}", dest_name, operand_name);
            }
            (UnaryOp::Not, IrType::Bool) => {
                emit!(self, "  {} = xor i1 {}, true", dest_name, operand_name);
            }
            (UnaryOp::BitNot, IrType::Int) => {
                emit!(self, "  {} = xor i64 {}, -1", dest_name, operand_name);
            }
            _ => {
                return Err(CodegenError::with_code(
                    format!("عملية أحادية غير مدعومة: {:?} على {:?}", op, ty),
                    ERR_LLVM_INTERNAL.to_string(),
                ));
            }
        }

        Ok(())
    }

    fn get_or_create_var(&mut self, var: VarId) -> String {
        if let Some(name) = self.var_map.get(&var.0) {
            name.clone()
        } else {
            let name = format!("%v{}", var.0);
            self.var_map.insert(var.0, name.clone());
            name
        }
    }

    /// Create a new unique temporary variable name
    fn new_temp(&mut self) -> String {
        self.name_counter += 1;
        format!("%t{}", self.name_counter)
    }

    fn get_var(&self, var: VarId) -> Result<String, CodegenError> {
        self.var_map.get(&var.0).cloned().ok_or_else(|| {
            CodegenError::with_code(
                format!("متغير غير معروف: {}", var),
                ERR_LLVM_INTERNAL.to_string(),
            )
        })
    }

    fn get_block(&self, block: BlockId) -> Result<String, CodegenError> {
        self.block_map.get(&block.0).cloned().ok_or_else(|| {
            CodegenError::with_code(
                format!("كتلة غير معروفة: {}", block),
                ERR_LLVM_INTERNAL.to_string(),
            )
        })
    }

    fn fresh_name(&mut self, prefix: &str) -> String {
        self.name_counter += 1;
        format!("%{}.{}", prefix, self.name_counter)
    }

    /// True when a value of type `from` needs boxing to satisfy a slot, parameter
    /// or return position declared `to`.
    ///
    /// An optional `T?` lowers to `Ptr(T)`, so a scalar reaching one has to be
    /// put behind a pointer — otherwise the raw bit pattern stands in for the
    /// pointer and `عدد? = 0` is indistinguishable from `لا_شيء` (#185).
    fn needs_boxing(from: &IrType, to: &IrType) -> bool {
        matches!(from, IrType::Int | IrType::Float | IrType::Bool)
            && matches!(to, IrType::Ptr(pointee) if **pointee == *from)
    }

    /// Whether an operand is a string at run time.
    ///
    /// `نص` is `String` and `نص?` is `Ptr(String)`, but both are a `TrqString*`
    /// once compiled — so a narrowed optional string must take the string path
    /// too, or `طول` counts its bytes again (#185).
    fn is_string_operand(ty: Option<&IrType>) -> bool {
        matches!(ty, Some(IrType::String))
            || matches!(ty, Some(IrType::Ptr(inner)) if matches!(**inner, IrType::String))
    }

    /// The declared type of `field` on `class`.
    ///
    /// `class_defs` holds each class's *own* fields, so an inherited field is not
    /// under the receiver's name. The fallback searches every class, but only
    /// accepts an unambiguous match — guessing between two same-named fields on
    /// unrelated classes would be worse than declining to box.
    fn declared_field_type(&self, class: &str, field: &str) -> Option<IrType> {
        let own = |c: &str| {
            self.class_defs.get(c).and_then(|fields| {
                fields
                    .iter()
                    .find(|(name, _)| name == field)
                    .map(|(_, ty)| ty.clone())
            })
        };

        own(class).or_else(|| {
            let mut found = self.class_defs.values().filter_map(|fields| {
                fields
                    .iter()
                    .find(|(name, _)| name == field)
                    .map(|(_, ty)| ty.clone())
            });
            let only = found.next()?;
            found.next().is_none().then_some(only)
        })
    }

    /// Load a boxed scalar back out of its cell.
    fn emit_unboxed_scalar(&mut self, value_name: &str, ty: &IrType) -> String {
        let llvm_ty = self.type_mapper.map_type(ty);
        let loaded = self.fresh_name("opt.unbox");
        let _ = writeln!(
            self.output,
            "  {} = load {}, ptr {}",
            loaded, llvm_ty, value_name
        );
        loaded
    }

    /// Allocate an 8-byte cell, store `value_name` in it, and yield the cell.
    fn emit_boxed_scalar(&mut self, value_name: &str, ty: &IrType) -> String {
        let llvm_ty = self.type_mapper.map_type(ty);
        let boxed = self.fresh_name("opt.box");
        let _ = writeln!(self.output, "  {} = call ptr @trq_alloc(i64 8)", boxed);
        let _ = writeln!(
            self.output,
            "  store {} {}, ptr {}",
            llvm_ty, value_name, boxed
        );
        boxed
    }
}

#[derive(Debug)]
pub struct CodegenError {
    pub message: String,
    pub code: Option<String>,
}

impl CodegenError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            code: None,
        }
    }

    pub fn with_code(message: String, code: String) -> Self {
        Self {
            message,
            code: Some(code),
        }
    }
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref code) = self.code {
            write!(f, "[{}] {}", code, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for CodegenError {}

/// The LLVM symbol a function name is emitted under.
///
/// `user_functions` is every name the program declares. A declared name keeps
/// its own mangled symbol; only an undeclared one falls through to the runtime
/// table. Applying the table to a *definition* is what made a user's `مطلق`
/// collide with the `declare @trq_abs_float` above it (#257).
fn mangle_function_name(name: &str, user_functions: &HashSet<String>) -> String {
    if !user_functions.contains(name) {
        if let Some(runtime_name) = get_runtime_function_name(name) {
            return runtime_name.to_string();
        }
    }
    mangle_name(name)
}

fn get_runtime_function_name(arabic_name: &str) -> Option<&'static str> {
    match arabic_name {
        // Core tier: reachable with no import, and its `IrType::String` return
        // type is registered in the IR builder. `trq_string_substr_chars` also
        // survives independently of any name — `trq_string_char_at` calls it, and
        // that is what the `س[ي]` operator lowers to.
        "قص_حروف" => Some("trq_string_substr_chars"),
        // The `نص` module tier, unlike the core names below it.
        "حرف_في" => Some("trq_string_char_at"),
        "حرف_إلى_رمز" => Some("trq_string_char_code"),
        "رمز_إلى_حرف" => Some("trq_string_from_char_code"),
        // Core tier too, and the first one returning an array: the `ptr` this
        // declares is a `TrqArray*`, told apart from a `TrqString*` only by the
        // `IrType::Array` registered in the IR builder.
        "نص_إلى_ثنائي" => Some("trq_string_to_bytes"),
        // The mirror: the `ptr` it takes is a `TrqArray*`, and the one it
        // returns is a `TrqString*`.
        "ثنائي_إلى_نص" => Some("trq_string_from_bytes"),
        "يحتوي" => Some("trq_string_contains"),
        "يبدأ_بـ" => Some("trq_string_starts_with"),
        "ينتهي_بـ" => Some("trq_string_ends_with"),
        "موضع" => Some("trq_string_index_of"),
        "موضع_اخير" => Some("trq_string_last_index_of"),
        "عدد_مرات" => Some("trq_string_count"),
        "كبير" => Some("trq_string_to_upper"),
        "صغير" => Some("trq_string_to_lower"),
        "عنوان" => Some("trq_string_to_title"),
        "اعكس_نص" => Some("trq_string_reverse"),
        "ازل_فراغات" => Some("trq_string_trim"),
        "ازل_فراغات_يسار" => Some("trq_string_trim_left"),
        "ازل_فراغات_يمين" => Some("trq_string_trim_right"),
        "قسّم" => Some("trq_string_split"),
        "ادمج" => Some("trq_string_join"),
        "استبدل" => Some("trq_string_replace"),
        "استبدل_كل" => Some("trq_string_replace_all"),
        "كرر_نص" => Some("trq_string_repeat"),
        "احشو_يسار" => Some("trq_string_pad_left"),
        "احشو_يمين" => Some("trq_string_pad_right"),
        "طول_نص" => Some("trq_string_len"),
        "طول_حروف" => Some("trq_string_len_chars"),
        "رقمي" => Some("trq_string_is_numeric"),
        "حروف_فقط" => Some("trq_string_is_alpha"),
        "عربي" => Some("trq_string_is_arabic"),
        "قارن_نص" => Some("trq_string_compare"),
        "نصوص_متساوية" => Some("trq_string_equals"),

        "عدد_لنص" => Some("trq_int_to_string"),
        "عشري_لنص" => Some("trq_float_to_string"),
        "منطقي_لنص" => Some("trq_bool_to_string"),
        "نص" => Some("trq_int_to_string"), // Generic to-string (primarily used with int)
        "نص_لعدد" => Some("trq_string_to_int"),
        "نص_لعشري" => Some("trq_string_to_float"),

        "ملف_موجود" => Some("trq_file_exists"),
        "هل_ملف" => Some("trq_file_is_file"),
        "هل_مجلد" => Some("trq_file_is_dir"),
        "اقرأ_ملف" => Some("trq_file_read"),
        "اكتب_ملف" => Some("trq_file_write"),
        "الحق_ملف" => Some("trq_file_append"),
        "احذف_ملف" => Some("trq_file_delete"),
        "انسخ_ملف" => Some("trq_file_copy"),
        "انقل_ملف" => Some("trq_file_move"),
        "حجم_ملف" => Some("trq_file_size"),
        "انشئ_مجلد" => Some("trq_dir_create"),
        "قائمة_مجلد" => Some("trq_dir_list"),
        "احذف_مجلد" => Some("trq_dir_delete"),
        "مجلد_حالي" => Some("trq_dir_current"),
        "مجلد_مستخدم" => Some("trq_dir_home"),
        "مجلد_مؤقت" => Some("trq_dir_temp"),
        // `مجلد_مستخدم` reduces to this one — `trq_dir_home` is `getenv("HOME")`
        // and nothing else — so it can become a stdlib wrapper in Increment G.
        // `مجلد_مؤقت` cannot: `trq_dir_temp` calls `std::env::temp_dir()`, whose
        // fallback to `/tmp` no single environment read reproduces. Both stay
        // live under their own symbols here regardless.
        "متغير_بيئة" => Some("trq_env_get"),
        // Core tier. Reached through the plain `Call` fall-through like its
        // neighbours, so the Arabic name arrives here and is mapped once.
        "اكتب_مجرى" => Some("trq_write_stream"),
        "اقرأ_مجرى" => Some("trq_read_stream"),
        // Core tier as well, and the one name that answers for four of the
        // `ملفات` names above: `ملف_موجود`، `هل_ملف`، `هل_مجلد` and `حجم_ملف` all
        // reduce to a field of this. They stay mapped here until Increment G
        // flips `ملفات` to disk — standing rule 3: a symbol is not deleted just
        // because a name that folds it landed.
        "حالة_مسار" => Some("trq_path_status"),
        // Core tier too, and the sibling that *acts* where that one asks. It
        // folds `احذف_ملف` and `احذف_مجلد`, which stay mapped above until
        // Increment G flips `ملفات` — standing rule 3.
        "افتح_ملف" => Some("trq_file_open"),
        "احذف_مسار" => Some("trq_path_delete"),
        // Core tier, and the only one of these whose answer the compiler crate
        // does not also compute: the runtime reads its own `main`'s argv while
        // the interpreters read what the CLI was handed, so there is no shared
        // kernel here and nothing for the two to drift on.
        "معاملات_البرنامج" => Some("trq_program_args"),
        // Both spellings of one primitive, mapped to one symbol — the kasra-less
        // variant is an orthographic pair like the keyword table's `ارمِ`/`ارم`,
        // not a second capability.
        "أنهِ_البرنامج" | "أنه_البرنامج" => Some("trq_exit"),
        "ادمج_مسار" => Some("trq_path_join"),
        "مسار_اب" => Some("trq_path_parent"),
        "اسم_ملف" => Some("trq_path_filename"),
        "امتداد_ملف" => Some("trq_path_extension"),
        "فاصل_مسار" => Some("trq_path_separator"),

        "بذرة_عشوائي" => Some("trq_random_seed"),
        "عشوائي_عدد" => Some("trq_random_int"),
        "عشوائي_عدد_بين" => Some("trq_random_int_range"),
        "عشوائي_عشري" => Some("trq_random_float"),
        "عشوائي_عشري_بين" => Some("trq_random_float_range"),
        "عشوائي_منطقي" => Some("trq_random_bool"),

        "تاريخ_اليوم" => Some("trq_date_today"),
        "حلل_تاريخ" => Some("trq_date_parse"),
        "تاريخ_من_طابع" => Some("trq_date_from_timestamp"),
        "أضف_أيام" => Some("trq_date_add_days"),
        "أضف_أشهر" => Some("trq_date_add_months"),
        "فرق_أيام" => Some("trq_date_diff_days"),
        "يوم_الأسبوع" => Some("trq_day_of_week"),
        "يوم_السنة" => Some("trq_day_of_year"),
        "رقم_الأسبوع" => Some("trq_week_number"),
        "أيام_الشهر" => Some("trq_days_in_month"),
        "نسّق_تاريخ" => Some("trq_date_format"),
        "وقت_الآن" => Some("trq_time_now"),
        "حلل_وقت" => Some("trq_time_parse"),
        "نسّق_وقت" => Some("trq_time_format"),
        "تاريخ_ووقت_من_طابع" => Some("trq_datetime_from_timestamp"),
        "حلل_تاريخ_ووقت" => Some("trq_datetime_parse"),
        "نسّق_تاريخ_ووقت" => Some("trq_datetime_format"),
        "نم" => Some("trq_sleep"),
        "وقت_أداء" => Some("trq_performance_now"),

        "نقل_اتصل" => Some("trq_tcp_connect"),
        "نقل_اغلق" => Some("trq_tcp_close"),
        "نقل_ارسل" => Some("trq_tcp_send"),
        "نقل_ارسل_بايتات" => Some("trq_tcp_send_bytes"),
        "نقل_استقبل" => Some("trq_tcp_receive"),
        "نقل_استقبل_بايتات" => Some("trq_tcp_receive_bytes"),
        "نقل_استقبل_حتى" => Some("trq_tcp_receive_until"),
        "نقل_بيانات_متاحة" => Some("trq_tcp_available"),
        "نقل_استمع" => Some("trq_tcp_listen"),
        "نقل_اقبل" => Some("trq_tcp_accept"),
        "نقل_اقبل_مع_مهلة" => Some("trq_tcp_accept_timeout"),
        "نقل_عنوان_محلي" => Some("trq_tcp_local_address"),
        "نقل_منفذ_محلي" => Some("trq_tcp_local_port"),
        "حزم_اربط" => Some("trq_udp_bind"),
        "حزم_اغلق" => Some("trq_udp_close"),
        "حزم_ارسل_الى" => Some("trq_udp_send_to"),
        "حزم_ارسل_بايتات_الى" => Some("trq_udp_send_bytes_to"),
        "حزم_استقبل" => Some("trq_udp_receive"),
        "حزم_استقبل_بايتات" => Some("trq_udp_receive_bytes"),
        "حزم_ارسل_رد" => Some("trq_udp_reply"),
        "حل_عنوان" => Some("trq_resolve_hostname"),
        "احصل_عنوان_محلي" => Some("trq_get_local_ip"),
        "طلب_ويب" => Some("trq_http_request"),
        "حمّل_ويب" => Some("trq_http_download"),
        "رمّز_رابط" => Some("trq_url_encode"),
        "فك_رمز_رابط" => Some("trq_url_decode"),
        "ترميز_أساس64" => Some("trq_base64_encode"),
        "فك_أساس64" => Some("trq_base64_decode"),

        // Cryptography - SHA-256 (بصمة = fingerprint)
        "احسب_بصمة" => Some("trq_sha256_string"),
        "بصمة_ملف" => Some("trq_sha256_file"),
        "بصمة_ثنائي" => Some("trq_sha256_bytes"),
        "طابق_بصمة" => Some("trq_sha256_compare"),

        // Hex encoding (ست_عشري = hexadecimal)
        "إلى_ست_عشري" => Some("trq_hex_encode"),
        "من_ست_عشري" => Some("trq_hex_decode"),
        "ثنائي_إلى_ست_عشري" => Some("trq_hex_encode_bytes"),
        "ست_عشري_إلى_ثنائي" => Some("trq_hex_decode_to_bytes"),

        // Compression (اضغط = compress, فك_الضغط = decompress)
        "اضغط" => Some("trq_gzip_compress_string"),
        "فك_الضغط" => Some("trq_gzip_decompress_to_string"),
        "اضغط_ثنائي" => Some("trq_gzip_compress_bytes"),
        "فك_ضغط_ثنائي" => Some("trq_gzip_decompress_bytes"),
        "اضغط_ملف" => Some("trq_gzip_compress_file"),
        "فك_ضغط_ملف" => Some("trq_gzip_decompress_file"),

        "اطبع" | "طباعة" => Some("trq_print"),
        "اطبع_سطر" => Some("trq_print"), // Will add newline in wrapper
        "اطبع_خطأ" => Some("trq_print_error"),
        "اطبع_منسق" => Some("trq_print"),
        "ادخل" => Some("trq_input"),
        "ادخل_رسالة" => Some("trq_input_prompt"),
        "ادخل_عدد" => Some("trq_input_int"),
        "ادخل_عشري" => Some("trq_input_float"),

        "جذر" => Some("trq_sqrt"),
        "جذر_تكعيبي" => Some("trq_cbrt"),
        "لوغاريتم" => Some("trq_log"),
        "لوغ10" | "لوغاريتم10" => Some("trq_log10"),
        "لوغ2" => Some("trq_log2"),
        "أسي" | "أس" => Some("trq_exp"),
        "أرضية" => Some("trq_floor"),
        "سقف" => Some("trq_ceil"),
        "قرّب" | "تقريب" => Some("trq_round"),
        "اقتطع" => Some("trq_trunc"),
        "مطلق" => Some("trq_abs_float"),
        "مطلق_عدد" => Some("trq_abs_int"),
        "أقل" | "أدنى" => Some("trq_min_float"),
        "أقل_عدد" => Some("trq_min_int"),
        "أكبر" | "أقصى" => Some("trq_max_float"),
        "أكبر_عدد" => Some("trq_max_int"),
        "حصر" => Some("trq_clamp_float"),
        "حصر_عدد" => Some("trq_clamp_int"),
        "علامة" => Some("trq_sign"),
        "باقي" => Some("trq_mod"),
        "قاسم_مشترك" => Some("trq_gcd"),
        "مضاعف_مشترك" => Some("trq_lcm"),
        "عاملي" => Some("trq_factorial"),
        "قوة" => Some("trq_pow_float"),
        "قوة_عدد" => Some("trq_pow_int"),

        "جا" | "جيب" => Some("trq_sin"),
        "جتا" | "جيب_التمام" => Some("trq_cos"),
        "ظا" | "ظل" => Some("trq_tan"),
        "ظتا" | "ظل_التمام" => Some("trq_cot"),
        "قا" | "قاطع" => Some("trq_sec"),
        "قتا" | "قاطع_التمام" => Some("trq_csc"),
        "جا_عكسي" | "جيب_عكسي" => Some("trq_asin"),
        "جتا_عكسي" | "جيب_تمام_عكسي" => Some("trq_acos"),
        "ظا_عكسي" => Some("trq_atan"),
        "ظا_عكسي2" => Some("trq_atan2"),
        "جا_زائدي" => Some("trq_sinh"),
        "جتا_زائدي" => Some("trq_cosh"),
        "ظا_زائدي" => Some("trq_tanh"),
        "الى_راديان" | "راديان" => Some("trq_to_radians"),
        "الى_درجات" | "درجات" => Some("trq_to_degrees"),

        "توقف" => Some("trq_panic"),
        "طول" => Some("trq_array_len"), // Generic length function for arrays
        "طول_مصفوفة" => Some("trq_array_len"),
        "الحق" => Some("trq_array_push"),

        // Networking - الشبكة
        // TCP Operations
        "اتصل_خادم" => Some("trq_tcp_connect"),
        "أغلق_اتصال" => Some("trq_tcp_close"),
        "أرسل" => Some("trq_tcp_send"),
        "أرسل_بايتات" => Some("trq_tcp_send_bytes"),
        "استقبل" => Some("trq_tcp_receive"),
        "استقبل_بايتات" => Some("trq_tcp_receive_bytes"),
        "استقبل_حتى" => Some("trq_tcp_receive_until"),
        "هل_متاح" => Some("trq_tcp_available"),
        "استمع" => Some("trq_tcp_listen"),
        "اقبل_اتصال" => Some("trq_tcp_accept"),
        "عنوان_محلي" => Some("trq_tcp_local_address"),
        "منفذ_محلي" => Some("trq_tcp_local_port"),

        // UDP Operations
        "ارتبط_منفذ" => Some("trq_udp_bind"),
        "أرسل_إلى" => Some("trq_udp_send_to"),
        "استقبل_من" => Some("trq_udp_receive"),
        "رد" => Some("trq_udp_reply"),

        // DNS and utilities
        "حل_اسم_نطاق" => Some("trq_resolve_hostname"),
        "عنوان_محلي_للجهاز" => Some("trq_get_local_ip"),

        // HTTP Operations
        "احصل_ويب" => Some("trq_http_get"),
        "حمّل_ملف" => Some("trq_http_download"),

        // URL encoding
        "فك_ترميز_رابط" => Some("trq_url_decode"),

        _ => None,
    }
}

fn mangle_class_name(name: &str) -> String {
    mangle_name(name)
}

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

fn escape_llvm_string(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        if byte == b'"' || byte == b'\\' || !(32..=126).contains(&byte) {
            result.push_str(&format!("\\{:02X}", byte));
        } else {
            result.push(byte as char);
        }
    }
    result.push_str("\\00");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    // No longer used by `generate` itself — the code now travels on the
    // `NativeBlock` — but these tests assert which code a given source produces.
    use crate::error::codes::ERR_UNTYPED_LAMBDA_PARAM;

    #[test]
    fn test_mangle_function_name() {
        let none = HashSet::new();
        assert_eq!(mangle_function_name("main", &none), "main");
        assert_eq!(mangle_function_name("add_numbers", &none), "add_numbers");
        assert_eq!(mangle_function_name("اطبع", &none), "trq_print");
        assert_eq!(mangle_function_name("طباعة", &none), "trq_print");
        let mangled = mangle_function_name("دالتي", &none);
        assert!(!mangled.contains("دالتي"));
        assert!(mangled.contains("_U"));
    }

    /// A name the program declares keeps its own symbol, so the definition can
    /// never collide with the `declare @trq_*` emitted for the same name (#257).
    #[test]
    fn test_a_declared_name_does_not_adopt_the_runtime_symbol() {
        let declared: HashSet<String> = ["اطبع", "مطلق"].iter().map(|s| s.to_string()).collect();

        for name in ["اطبع", "مطلق"] {
            let symbol = mangle_function_name(name, &declared);
            assert!(
                !symbol.starts_with("trq_"),
                "'{name}' يجب ألا يأخذ رمز وقت التشغيل، وُجد {symbol}"
            );
        }

        // An undeclared name still maps, so call sites reach the runtime.
        assert_eq!(mangle_function_name("طباعة", &declared), "trq_print");
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

    #[test]
    fn test_string_ternary_global_stores_ptr_not_i64() {
        // Phi results were not registered in var_types, so GlobalStore fell
        // back to i64 and emitted `store i64 <ptr-value>` — invalid LLVM IR
        use crate::ir::IrBuilder;
        use crate::parser::Parser;

        let source = "بسم_الله\nمتغير وضع = 1 >= 0 ? \"بالغ\" : \"قاصر\"؛\nاطبع(وضع)؛\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("Failed to parse");
        let ir_module = IrBuilder::new("test".to_string())
            .build(&ast)
            .expect("Failed to build IR");

        let mut codegen = LlvmCodegen::new(crate::codegen::Target::native());
        let llvm_ir = codegen.generate(&ir_module).expect("codegen failed");

        let global_store_lines: Vec<&str> = llvm_ir
            .lines()
            .filter(|l| l.contains("store") && l.contains("@_U0648__U0636__U0639_"))
            .collect();
        assert!(
            !global_store_lines.is_empty(),
            "expected a store to the وضع global in:\n{}",
            llvm_ir
        );
        for line in &global_store_lines {
            assert!(
                line.trim_start().starts_with("store ptr"),
                "string ternary must be stored as ptr, got: {}",
                line
            );
        }
    }

    #[test]
    fn test_lambda_call_indirect_uses_real_return_type() {
        // Regression test (issue #180): a call through a global holding a
        // lambda used to emit `call ptr %fn(...)` regardless of the
        // lambda's actual return type, because the global's recorded type
        // was a provisional `Ptr(Void)` guess from before the lambda body
        // was built. Against an i64-returning lambda this is an LLVM
        // call-site/callee signature mismatch that segfaults at runtime.
        use crate::ir::IrBuilder;
        use crate::parser::Parser;

        let source = "بسم_الله\nثابت مربع = (س: عدد) => س * س؛\nاطبع(مربع(5))؛\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("Failed to parse");
        let ir_module = IrBuilder::new("test".to_string())
            .build(&ast)
            .expect("Failed to build IR");

        let mut codegen = LlvmCodegen::new(crate::codegen::Target::native());
        let llvm_ir = codegen.generate(&ir_module).expect("codegen failed");

        let call_line = llvm_ir
            .lines()
            .find(|l| l.contains("call") && l.contains('%') && !l.contains("call void @trq"))
            .unwrap_or_else(|| panic!("expected an indirect call in:\n{}", llvm_ir));
        assert!(
            call_line.contains("call i64"),
            "indirect call must use the lambda's real i64 return type, got: {}",
            call_line
        );
    }

    #[test]
    fn test_lambda_untyped_param_rejected_in_native_codegen() {
        // Native codegen cannot safely lower an arrow-function parameter
        // that never resolved to a concrete type (issue #180); the
        // interpreter handles this dynamically (see
        // tests/lambda_execution_tests.rs::تنفيذ::test_spec_untyped_sum_lambda_executes),
        // but native mode needs a real LLVM type at the parameter.
        use crate::error::codes::ERR_UNTYPED_LAMBDA_PARAM;
        use crate::ir::IrBuilder;
        use crate::parser::Parser;

        let source = "بسم_الله\nثابت جمع = (أ، ب) => أ + ب؛\nاطبع(جمع(3، 4))؛\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("Failed to parse");
        let ir_module = IrBuilder::new("test".to_string())
            .build(&ast)
            .expect("Failed to build IR");

        let mut codegen = LlvmCodegen::new(crate::codegen::Target::native());
        let err = codegen
            .generate(&ir_module)
            .expect_err("native codegen must reject an untyped lambda parameter");

        assert_eq!(
            err.code.as_deref(),
            Some(ERR_UNTYPED_LAMBDA_PARAM.to_string().as_str())
        );
    }

    #[test]
    fn test_indirect_call_through_any_slot_rejected_in_native_codegen() {
        // A lambda passed through an `أي`-typed parameter has no static
        // signature at the call site; emitting the call anyway would
        // produce an ABI-mismatched `call ptr` against a `define i64`
        // callee — silent corruption instead of an error. Native codegen
        // must reject it (ت٠٣٠٢); the interpreter runs it fine.
        use crate::error::codes::ERR_UNTYPED_INDIRECT_CALL;
        use crate::ir::IrBuilder;
        use crate::parser::Parser;

        // The callee reaches the call site through an `أي`-typed *variable*,
        // so no function has an unlowerable parameter (the lambda's own `س`
        // is annotated) and the ت٠٣٠١ parameter guard does not fire first.
        let source = "بسم_الله\nمتغير ف: أي = (س: عدد) => س * س؛\nاطبع(ف(5))؛\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("Failed to parse");
        let ir_module = IrBuilder::new("test".to_string())
            .build(&ast)
            .expect("Failed to build IR");

        let mut codegen = LlvmCodegen::new(crate::codegen::Target::native());
        let err = codegen
            .generate(&ir_module)
            .expect_err("native codegen must reject an indirect call with no known signature");

        assert_eq!(
            err.code.as_deref(),
            Some(ERR_UNTYPED_INDIRECT_CALL.to_string().as_str())
        );
    }

    #[test]
    fn test_lambda_as_call_argument_compiles_natively() {
        // A lambda literal passed as a call argument picks up its param
        // types from the callee's declared function-type parameter — the
        // same contextual inference the semantic layer performs. Without
        // the hint it lifts with `Ptr(Void)` params and native compilation
        // rejects spec-legal code with ت٠٣٠١.
        use crate::ir::IrBuilder;
        use crate::parser::Parser;

        let source = "بسم_الله\nدالة طبق(ف: (عدد) -> عدد، ق: عدد) -> عدد {\nأرجع ف(ق)؛\n}\nاطبع(طبق((س) => س * 2، 5))؛\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("Failed to parse");
        let ir_module = IrBuilder::new("test".to_string())
            .build(&ast)
            .expect("Failed to build IR");

        let mut codegen = LlvmCodegen::new(crate::codegen::Target::native());
        let llvm_ir = codegen
            .generate(&ir_module)
            .expect("a lambda argument with an inferable signature must compile natively");

        assert!(
            llvm_ir.contains("define i64 @__lambda_0(i64"),
            "the argument lambda must lift with a concrete i64 param, in:\n{}",
            llvm_ir
        );
    }

    #[test]
    fn test_program_mode_annotated_global_lambda_compiles_natively() {
        // Program mode routes global initializers through __global_init__;
        // that path must thread the declared function-type annotation to
        // the lambda (and must not lift a second orphaned duplicate from
        // the statement pass, where the store would be silently dropped).
        use crate::ir::IrBuilder;
        use crate::parser::Parser;

        let source = "بسم_الله\nثابت جمع: (عدد، عدد) -> عدد = (أ، ب) => أ + ب؛\nدالة رئيسية() {\nاطبع(جمع(3، 4))؛\n}\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("Failed to parse");
        let ir_module = IrBuilder::new("test".to_string())
            .build(&ast)
            .expect("Failed to build IR");

        let lambda_count = ir_module
            .functions
            .iter()
            .filter(|f| f.name.starts_with("__lambda_"))
            .count();
        assert_eq!(
            lambda_count, 1,
            "the global initializer must lift exactly one lambda, not an orphaned duplicate"
        );

        let mut codegen = LlvmCodegen::new(crate::codegen::Target::native());
        let llvm_ir = codegen
            .generate(&ir_module)
            .expect("an annotated program-mode global lambda must compile natively");
        assert!(
            llvm_ir.contains("define i64 @__lambda_0(i64"),
            "the annotation must reach the lifted lambda's params, in:\n{}",
            llvm_ir
        );
    }

    #[test]
    fn test_mixed_bare_and_valued_lambda_returns_unify() {
        // Semantic analysis accepts mixed bare/valued returns in a block
        // lambda (legal early-return code, folded to أي). The lifted
        // function must patch bare returns to a zero-of-type return, or
        // native codegen emits `ret void` inside a non-void define —
        // invalid LLVM IR.
        use crate::ir::IrBuilder;
        use crate::parser::Parser;

        let source = "بسم_الله\nثابت ف = (س: عدد) => {\nإذا (س > 0) {\nأرجع؛\n}\nأرجع 1؛\n}؛\nف(5)؛\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("Failed to parse");
        let ir_module = IrBuilder::new("test".to_string())
            .build(&ast)
            .expect("Failed to build IR");

        let mut codegen = LlvmCodegen::new(crate::codegen::Target::native());
        let llvm_ir = codegen.generate(&ir_module).expect("codegen failed");

        let lambda_body: String = llvm_ir
            .lines()
            .skip_while(|l| !l.contains("@__lambda_0"))
            .take_while(|l| !l.starts_with('}'))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            lambda_body.contains("define i64"),
            "mixed returns must unify to the valued type, in:\n{}",
            llvm_ir
        );
        assert!(
            !lambda_body.contains("ret void"),
            "bare أرجع must be patched to a typed return inside a non-void lambda, in:\n{}",
            lambda_body
        );
    }

    #[test]
    fn test_indirect_call_coerces_int_arg_to_float() {
        // Spec §5.6's implicit عدد → عدد_عشري coercion must also apply at
        // indirect-call arguments: passing a raw i64 where the callee's
        // signature says double makes the callee reinterpret the bit
        // pattern (garbage output natively).
        use crate::ir::IrBuilder;
        use crate::parser::Parser;

        let source = "بسم_الله\nثابت ف: (عدد_عشري) -> عدد_عشري = (س) => س؛\nاطبع(ف(5))؛\nالحمد_لله";
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("Failed to parse");
        let ir_module = IrBuilder::new("test".to_string())
            .build(&ast)
            .expect("Failed to build IR");

        let mut codegen = LlvmCodegen::new(crate::codegen::Target::native());
        let llvm_ir = codegen.generate(&ir_module).expect("codegen failed");

        let call_line = llvm_ir
            .lines()
            .find(|l| l.contains("call double"))
            .unwrap_or_else(|| panic!("expected a double indirect call in:\n{}", llvm_ir));
        assert!(
            call_line.contains("double %") && !call_line.contains("i64 %"),
            "the int argument must be coerced to double before the call, got: {}",
            call_line
        );
    }

    /// Builds a module and returns its LLVM IR, or the CodegenError code.
    fn compile_or_code(source: &str) -> Result<String, Option<String>> {
        use crate::ir::IrBuilder;
        use crate::parser::Parser;

        let ast = Parser::new(source).parse().expect("Failed to parse");
        let ir_module = IrBuilder::new("test".to_string())
            .build(&ast)
            .expect("Failed to build IR");
        LlvmCodegen::new(crate::codegen::Target::native())
            .generate(&ir_module)
            .map_err(|e| e.code.clone())
    }

    #[test]
    fn test_call_through_lambda_variable_types_the_result_slot() {
        // The slot's type came from `infer_expr_type`, whose `Call` arm knew
        // only the global function table — a call *through a function value*
        // fell back to `Ptr(Void)`, so `اطبع` emitted `trq_print(ptr %x)` on
        // an integer and dereferenced it (segfault, exit 139).
        let ir = compile_or_code(
            "بسم_الله\nثابت مربع = (س: عدد) => س * س؛\nمتغير ن = مربع(5)؛\nاطبع(ن)؛\nالحمد_لله",
        )
        .expect("must compile");
        assert!(
            ir.contains("call void @trq_print_int"),
            "the call result must be typed i64, not printed as a pointer, in:\n{ir}"
        );
        assert!(
            !ir.contains("call void @trq_print(ptr"),
            "an integer must never reach the pointer-printing runtime call, in:\n{ir}"
        );
    }

    #[test]
    fn test_void_bodied_lambda_emits_bare_return() {
        // `(س) => اطبع(س)` is an idiomatic callback whose body is Void; the
        // lifted function must `ret void`, not `ret void %v` (invalid LLVM,
        // and the void Call never even names a dest).
        let ir = compile_or_code(
            "بسم_الله\nدالة رئيسية() {\nثابت اطبعه = (س: عدد) => اطبع(س)؛\nاطبعه(7)؛\n}\nالحمد_لله",
        )
        .expect("must compile");
        assert!(
            ir.contains("define void @__lambda_0"),
            "expected a void-returning lifted lambda in:\n{ir}"
        );
        assert!(
            !ir.contains("ret void %"),
            "`ret void` must not carry an operand, in:\n{ir}"
        );
    }

    #[test]
    fn test_any_annotated_param_is_rejected_natively() {
        // `أي` lowers to `Struct(ClassId("أي"))`, not `Ptr(Void)`, so a guard
        // keyed on the type shape missed it and emitted an ABI mismatch.
        assert_eq!(
            compile_or_code(
                "بسم_الله\nدالة رئيسية() {\nثابت ف = (س: أي) => س؛\nاطبع(ف(5))؛\n}\nالحمد_لله"
            )
            .unwrap_err()
            .as_deref(),
            Some(ERR_UNTYPED_LAMBDA_PARAM.to_string().as_str())
        );
    }

    #[test]
    fn test_declared_function_untyped_param_is_rejected_natively() {
        // Same hazard as the lambda case: keying the guard on the
        // `__lambda_` name prefix left this identical hole open.
        assert_eq!(
            compile_or_code("بسم_الله\nدالة ضاعف(س) { أرجع س * 2 }\nاطبع(ضاعف(5))؛\nالحمد_لله")
                .unwrap_err()
                .as_deref(),
            Some(ERR_UNTYPED_LAMBDA_PARAM.to_string().as_str())
        );
    }

    #[test]
    fn test_annotated_map_param_is_not_falsely_rejected() {
        // `قاموس<…>` legitimately lowers to `Ptr(Void)`, so the old
        // shape-keyed guard rejected fully-annotated code.
        compile_or_code(
            "بسم_الله\nثابت ف: (قاموس<نص، عدد>) -> عدد = (خ) => 1؛\nاطبع(ف({}))؛\nالحمد_لله",
        )
        .expect("an annotated قاموس parameter must compile natively");
    }

    #[test]
    fn test_curried_lambda_threads_hint_to_inner_lambda() {
        // LANGUAGE_SPEC §5.3's curried form: the inner lambda must inherit
        // its parameter type from the outer annotation's return signature,
        // or it lifts untyped and native codegen refuses documented syntax.
        let ir = compile_or_code(
            "بسم_الله\nثابت ف: (عدد) -> (عدد) -> عدد = (أ) => (ب) => ب * 2؛\nاطبع(ف(1)(21))؛\nالحمد_لله",
        )
        .expect("a curried annotated lambda must compile natively");
        assert!(
            ir.contains("(i64 %arg.0)"),
            "both lifted lambdas must take a concrete i64 param, in:\n{ir}"
        );
    }

    #[test]
    fn test_lambda_assigned_to_annotated_slot_threads_hint() {
        // Assignment position previously skipped the hint, so reassigning a
        // lambda to an already-annotated function-typed variable was
        // rejected with "declare a type" — which the user had declared.
        compile_or_code(
            "بسم_الله\nدالة رئيسية() {\nمتغير ف: (عدد) -> عدد = (س) => س + 1؛\nف = (س) => س * 10؛\nاطبع(ف(3))؛\n}\nالحمد_لله",
        )
        .expect("a lambda assigned to an annotated slot must compile natively");
    }

    #[test]
    fn test_non_unifiable_mixed_returns_give_a_tarqeem_diagnostic() {
        // Semantic analysis folds mixed return types to `أي` on purpose and
        // the interpreter dispatches dynamically, but native code needs one
        // ABI — the user must get a Tarqeem error, not a leaked clang one.
        assert_eq!(
            compile_or_code(
                "بسم_الله\nدالة رئيسية() {\nثابت ف = (س: عدد) => { إذا (س > 0) { أرجع \"نص\" } أرجع 1 }؛\nاطبع(ف(5))؛\n}\nالحمد_لله"
            )
            .unwrap_err()
            .as_deref(),
            Some(ERR_UNTYPED_LAMBDA_PARAM.to_string().as_str())
        );
    }
}
