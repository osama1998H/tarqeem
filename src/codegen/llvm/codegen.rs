//! LLVM IR Code Generator
//!
//! This module converts Tarqeem IR to LLVM IR text format.

use super::{mangle_name, TypeMapper};
use crate::codegen::Target;
use crate::ir::{
    BasicBlock, BinaryOp, BlockId, Class, Constant, Function, Instruction, IrType, Module, UnaryOp,
    VarId,
};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

macro_rules! emit {
    ($self:expr) => {
        writeln!($self.output).map_err(|e| CodegenError {
            message: format!("Failed to write LLVM output: {}", e),
            message_ar: format!("فشل في كتابة مخرجات LLVM: {}", e),
        })?
    };
    ($self:expr, $($arg:tt)*) => {
        writeln!($self.output, $($arg)*).map_err(|e| CodegenError {
            message: format!("Failed to write LLVM output: {}", e),
            message_ar: format!("فشل في كتابة مخرجات LLVM: {}", e),
        })?
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
    current_return_type: IrType,
    global_vars: HashMap<String, String>,
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
            current_return_type: IrType::Void,
            global_vars: HashMap::new(),
        }
    }

    pub fn generate(&mut self, module: &Module) -> Result<String, CodegenError> {
        self.output.clear();

        self.emit_header(&module.name)?;

        self.emit_runtime_types()?;

        self.emit_string_table(module)?;

        self.emit_global_variables(module)?;

        for class in &module.classes {
            self.emit_class_definition(class)?;
        }

        self.emit_runtime_declarations()?;

        for func in &module.functions {
            self.emit_function(func)?;
        }

        let has_main = module.functions.iter().any(|f| f.name == "__main__");
        if has_main {
            self.emit_c_main_entry()?;
        }

        Ok(self.output.clone())
    }

    fn emit_c_main_entry(&mut self) -> Result<(), CodegenError> {
        emit!(self);
        emit!(self, "; C entry point");
        emit!(self, "define i32 @main() {{");
        emit!(self, "entry:");
        emit!(self, "  call void @__main__()");
        emit!(self, "  ret i32 0");
        emit!(self, "}}");
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

            let init_val = match init {
                Some(Constant::Int(n)) => n.to_string(),
                Some(Constant::Float(f)) => {
                    format!("{:e}", f)
                }
                Some(Constant::Bool(b)) => {
                    if *b {
                        "1".to_string()
                    } else {
                        "0".to_string()
                    }
                }
                Some(Constant::Null) => "null".to_string(),
                Some(Constant::String(idx)) => {
                    if let Some((str_global, len)) = self.string_globals.get(idx) {
                        format!(
                            "getelementptr ([{} x i8], ptr {}, i64 0, i64 0)",
                            len + 1,
                            str_global
                        )
                    } else {
                        "null".to_string()
                    }
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

    fn emit_class_definition(&mut self, class: &Class) -> Result<(), CodegenError> {
        emit!(self, "; Class: {}", class.name);

        self.class_defs
            .insert(class.id.0.clone(), class.fields.clone());

        let type_def = self
            .type_mapper
            .generate_struct_type(&class.id, &class.fields);
        emit!(self, "{}", type_def);

        if !class.vtable.is_empty() {
            self.emit_vtable(class)?;
        }

        emit!(self);
        Ok(())
    }

    fn emit_vtable(&mut self, class: &Class) -> Result<(), CodegenError> {
        let mangled_class = mangle_class_name(&class.id.0);
        let vtable_name = format!("@vtable.{}", mangled_class);

        let vtable_entries: Vec<String> = class
            .vtable
            .iter()
            .map(|method| {
                let mangled_method_class = mangle_class_name(&method.class.0);
                let mangled_method_name = mangle_function_name(&method.name);
                format!("ptr @{}_{}", mangled_method_class, mangled_method_name)
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
        emit!(self, "declare ptr @trq_string_substr(ptr, i64, i64)");
        emit!(self, "declare ptr @trq_string_substr_chars(ptr, i64, i64)");
        emit!(self, "declare ptr @trq_string_char_at(ptr, i64)");
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
        emit!(self, "declare ptr @trq_bool_to_string(i1)");
        emit!(self, "declare i64 @trq_string_to_int(ptr)");
        emit!(self, "declare double @trq_string_to_float(ptr)");

        emit!(self, "declare ptr @trq_array_new(i64, i64)");
        emit!(self, "declare i64 @trq_array_len(ptr)");
        emit!(self, "declare ptr @trq_array_get(ptr, i64)");
        emit!(self, "declare void @trq_array_set(ptr, i64, ptr)");
        emit!(self, "declare void @trq_array_push(ptr, ptr, i64)");
        emit!(self, "declare ptr @trq_array_pop(ptr)");

        emit!(self, "declare void @trq_print(ptr)");
        emit!(self, "declare void @trq_print_int(i64)");
        emit!(self, "declare void @trq_print_float(double)");
        emit!(self, "declare void @trq_print_bool(i1)");
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
        emit!(self, "declare ptr @trq_time_now()");
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
        self.var_map.clear();
        self.block_map.clear();
        self.name_counter = 0;

        let func_name = mangle_function_name(&func.id.0);
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

        for block in &func.blocks {
            self.emit_block(block)?;
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
                        writeln!(
                            self.output,
                            "  {} = call ptr @trq_bool_to_string(i1 {})",
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
                    let val_name = self.get_var(*val)?;
                    let ret_ty = self.type_mapper.map_type(&self.current_return_type);
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
                let func_name = mangle_function_name(&func.0);

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
                let struct_ty = format!("%class.{}", mangle_class_name(&field.class.0));

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
                let struct_ty = format!("%class.{}", mangle_class_name(&field.class.0));

                let ptr_name = self.fresh_name("field.ptr");
                writeln!(
                    self.output,
                    "  {} = getelementptr inbounds {}, ptr {}, i32 0, i32 {}",
                    ptr_name, struct_ty, obj_name, field.index
                )
                .unwrap();
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
                let obj_name = self.get_var(*object)?;
                let full_method_name = format!("{}::{}", method.class.0, method.name);
                let method_name = mangle_function_name(&full_method_name);

                let mut all_args = vec![format!("ptr {}", obj_name)];
                for arg in args {
                    let arg_name = self.get_var(*arg)?;
                    let arg_ty = self.var_types.get(&arg.0).cloned().unwrap_or(IrType::Int);
                    let llvm_ty = self.type_mapper.map_param_type(&arg_ty);
                    all_args.push(format!("{} {}", llvm_ty, arg_name));
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
                if let Some(d) = dest {
                    let dest_name = self.get_or_create_var(*d);
                    emit!(
                        self,
                        "  {} = call {} {}({})",
                        dest_name,
                        ret_type,
                        method_ptr,
                        all_args.join(", ")
                    );
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
            }

            Instruction::Print { value } => {
                let val_name = self.get_var(*value)?;
                let var_type = self.var_types.get(&value.0).cloned();
                match &var_type {
                    Some(IrType::String) | Some(IrType::Ptr(_)) => {
                        emit!(self, "  call void @trq_print(ptr {})", val_name);
                    }
                    Some(IrType::Float) => {
                        emit!(self, "  call void @trq_print_float(double {})", val_name);
                    }
                    Some(IrType::Bool) => {
                        emit!(self, "  call void @trq_print_bool(i1 {})", val_name);
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
                let value_name = self.get_var(*value)?;
                let value_ty = self.var_types.get(&value.0).cloned().unwrap_or(IrType::Int);
                let llvm_type = self.type_mapper.map_type(&value_ty);
                let global_name = mangle_name(name);
                writeln!(
                    self.output,
                    "  store {} {}, ptr @{}",
                    llvm_type, value_name, global_name
                )
                .unwrap();
            }

            Instruction::Copy { dest, src, ty: _ } => {
                // Copy is a simple value transfer - just map the source variable name to the destination
                if let Some(src_name) = self.var_map.get(&src.0).cloned() {
                    self.var_map.insert(dest.0, src_name);
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

                    offset += self.type_mapper.type_size(&field_ty);
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

                // Calculate field offset: 8 (discriminant) + sum of previous field sizes
                // For simplicity, assume all fields are 8 bytes (pointer-sized)
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
                emit!(self, "  {} = fadd double {:e}, 0.0", dest_name, f);
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
        let left_name = self.get_var(left)?;
        let right_name = self.get_var(right)?;

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

            (BinaryOp::And, IrType::Bool) => "and i1",
            (BinaryOp::Or, IrType::Bool) => "or i1",

            (BinaryOp::BitAnd, IrType::Int) => "and i64",
            (BinaryOp::BitOr, IrType::Int) => "or i64",
            (BinaryOp::BitXor, IrType::Int) => "xor i64",
            (BinaryOp::Shl, IrType::Int) => "shl i64",
            (BinaryOp::Shr, IrType::Int) => "ashr i64",

            _ => {
                return Err(CodegenError {
                    message: format!("Unsupported binary operation: {:?} on {:?}", op, ty),
                    message_ar: format!("عملية ثنائية غير مدعومة: {:?} على {:?}", op, ty),
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
                return Err(CodegenError {
                    message: format!("Unsupported unary operation: {:?} on {:?}", op, ty),
                    message_ar: format!("عملية أحادية غير مدعومة: {:?} على {:?}", op, ty),
                });
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
        self.var_map
            .get(&var.0)
            .cloned()
            .ok_or_else(|| CodegenError {
                message: format!("Unknown variable: {}", var),
                message_ar: format!("متغير غير معروف: {}", var),
            })
    }

    fn get_block(&self, block: BlockId) -> Result<String, CodegenError> {
        self.block_map
            .get(&block.0)
            .cloned()
            .ok_or_else(|| CodegenError {
                message: format!("Unknown block: {}", block),
                message_ar: format!("كتلة غير معروفة: {}", block),
            })
    }

    fn fresh_name(&mut self, prefix: &str) -> String {
        self.name_counter += 1;
        format!("%{}.{}", prefix, self.name_counter)
    }
}

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

fn mangle_function_name(name: &str) -> String {
    if let Some(runtime_name) = get_runtime_function_name(name) {
        return runtime_name.to_string();
    }
    mangle_name(name)
}

fn get_runtime_function_name(arabic_name: &str) -> Option<&'static str> {
    match arabic_name {
        "قص_نص" => Some("trq_string_substr"),
        "قص_حروف" => Some("trq_string_substr_chars"),
        "حرف_في" => Some("trq_string_char_at"),
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
        "طول_مصفوفة" => Some("trq_array_len"),
        "الحق" => Some("trq_array_push"),

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

    #[test]
    fn test_mangle_function_name() {
        assert_eq!(mangle_function_name("main"), "main");
        assert_eq!(mangle_function_name("add_numbers"), "add_numbers");
        assert_eq!(mangle_function_name("اطبع"), "trq_print");
        assert_eq!(mangle_function_name("طباعة"), "trq_print");
        let mangled = mangle_function_name("دالتي");
        assert!(!mangled.contains("دالتي"));
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
