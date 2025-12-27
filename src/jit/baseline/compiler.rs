//! Baseline JIT Compiler Implementation
//!
//! This module implements the baseline JIT compiler using Cranelift.
//! It translates Tarqeem IR to Cranelift IR and compiles to native code.

use std::collections::HashMap;
use std::time::Instant;

use cranelift_codegen::ir::{
    types, AbiParam, Block, Function as CraneliftFunction, InstBuilder, Signature, UserFuncName,
};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module as CraneliftModule};

use crate::ir::{BinaryOp, Constant, Function, Instruction, IrType, Module, UnaryOp};
use crate::jit::cache::CompiledFunction;
use crate::jit::error::{JitError, JitResult};
use crate::jit::profile::CompilationTier;

/// Convert Tarqeem IR type to Cranelift type
fn ir_type_to_cranelift(ty: &IrType) -> JitResult<types::Type> {
    match ty {
        IrType::Void => Ok(types::INVALID),
        IrType::Bool => Ok(types::I8),
        IrType::Int => Ok(types::I64),
        IrType::Float => Ok(types::F64),
        IrType::String => Ok(types::I64), // Pointer to string struct
        IrType::Ptr(_) => Ok(types::I64), // Pointer
        IrType::Array(_, _) => Ok(types::I64), // Pointer to array
        IrType::Struct(_) => Ok(types::I64), // Pointer to struct
        IrType::Function { .. } => Ok(types::I64), // Function pointer
        IrType::Enum(_) => Ok(types::I64),          // Pointer to enum (tagged union)
    }
}

/// Baseline JIT compiler using Cranelift
pub struct BaselineCompiler {
    /// Cranelift JIT module
    jit_module: JITModule,

    /// Function builder context (reusable)
    builder_context: FunctionBuilderContext,

    /// Cranelift codegen context (reusable)
    ctx: Context,

    /// Map from Tarqeem function names to Cranelift function IDs
    func_ids: HashMap<String, FuncId>,

    /// Statistics
    functions_compiled: u64,
    total_compile_time_ms: u64,
}

impl BaselineCompiler {
    /// Create a new baseline compiler
    pub fn new() -> JitResult<Self> {
        // Configure Cranelift for the host target
        let mut flag_builder = settings::builder();
        flag_builder
            .set("use_colocated_libcalls", "false")
            .map_err(|e| {
                JitError::compilation(e.to_string(), format!("خطأ في إعداد Cranelift: {}", e))
            })?;
        flag_builder.set("is_pic", "false").map_err(|e| {
            JitError::compilation(e.to_string(), format!("خطأ في إعداد Cranelift: {}", e))
        })?;

        // Use the native target
        let isa_builder = cranelift_native::builder().map_err(|e| {
            JitError::compilation(
                e.to_string(),
                format!("فشل في الحصول على الهدف الأصلي: {}", e),
            )
        })?;

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| {
                JitError::compilation(e.to_string(), format!("فشل في إنشاء ISA: {}", e))
            })?;

        // Create JIT module
        let jit_builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let jit_module = JITModule::new(jit_builder);

        Ok(Self {
            jit_module,
            builder_context: FunctionBuilderContext::new(),
            ctx: Context::new(),
            func_ids: HashMap::new(),
            functions_compiled: 0,
            total_compile_time_ms: 0,
        })
    }

    /// Check if baseline JIT is available
    pub fn is_available() -> bool {
        true
    }

    /// Compile a function to native code
    pub fn compile(&mut self, module: &Module, func: &Function) -> JitResult<CompiledFunction> {
        let start_time = Instant::now();

        // Create Cranelift function signature
        let sig = self.create_signature(func)?;

        // Declare the function in the JIT module
        let func_id = self
            .jit_module
            .declare_function(&func.name, Linkage::Local, &sig)
            .map_err(|e| {
                JitError::compilation(e.to_string(), format!("فشل في التصريح عن الدالة: {}", e))
            })?;

        // Clear and set up context
        self.ctx.func =
            CraneliftFunction::with_name_signature(UserFuncName::user(0, func_id.as_u32()), sig);

        // Build function body
        self.build_function_body(module, func)?;

        // Compile the function
        self.jit_module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| JitError::codegen(e.to_string(), format!("فشل في تعريف الدالة: {}", e)))?;

        // Clear context for reuse
        self.jit_module.clear_context(&mut self.ctx);

        // Finalize the function
        self.jit_module.finalize_definitions().map_err(|e| {
            JitError::codegen(e.to_string(), format!("فشل في إنهاء التعريفات: {}", e))
        })?;

        // Get code size (estimate)
        let code_size = self.estimate_code_size(func);

        // Calculate compile time
        let compile_time_ms = start_time.elapsed().as_millis() as u64;

        // Update statistics
        self.functions_compiled += 1;
        self.total_compile_time_ms += compile_time_ms;

        // Store function ID mapping
        self.func_ids.insert(func.name.clone(), func_id);

        Ok(CompiledFunction::with_func_id(
            func.name.clone(),
            CompilationTier::BaselineCompiled,
            code_size,
            compile_time_ms,
            func_id,
        ))
    }

    /// Create a Cranelift signature from a Tarqeem function
    fn create_signature(&self, func: &Function) -> JitResult<Signature> {
        let mut sig = Signature::new(CallConv::SystemV);

        // Add parameters
        for param in &func.params {
            let param_type = ir_type_to_cranelift(&param.ty)?;
            sig.params.push(AbiParam::new(param_type));
        }

        // Add return type
        let ret_type = ir_type_to_cranelift(&func.return_type)?;
        if ret_type != types::INVALID {
            sig.returns.push(AbiParam::new(ret_type));
        }

        Ok(sig)
    }

    /// Build the function body using Cranelift FunctionBuilder
    fn build_function_body(&mut self, module: &Module, func: &Function) -> JitResult<()> {
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);

        // Create entry block
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // Create blocks for all basic blocks
        let mut block_map: HashMap<u32, Block> = HashMap::new();
        block_map.insert(0, entry_block);

        for bb in &func.blocks {
            if bb.id.0 != 0 {
                let block = builder.create_block();
                block_map.insert(bb.id.0, block);
            }
        }

        // Variable mapping for SSA values
        let mut var_map: HashMap<u32, Variable> = HashMap::new();
        let mut var_counter = 0u32;

        // Declare variables for parameters
        for (i, param) in func.params.iter().enumerate() {
            let var = Variable::from_u32(var_counter);
            var_counter += 1;
            let ty = ir_type_to_cranelift(&param.ty)?;
            builder.declare_var(var, ty);
            let param_value = builder.block_params(entry_block)[i];
            builder.def_var(var, param_value);
            var_map.insert(param.id.0, var);
        }

        // Compile each basic block
        for (block_idx, bb) in func.blocks.iter().enumerate() {
            let cranelift_block = block_map[&bb.id.0];

            // Switch to this block (except entry which we're already in)
            if block_idx > 0 {
                builder.switch_to_block(cranelift_block);
            }

            // Compile each instruction
            for inst in &bb.instructions {
                compile_instruction(
                    &mut builder,
                    module,
                    func,
                    inst,
                    &mut var_map,
                    &mut var_counter,
                    &block_map,
                )?;
            }

            // Seal the block if all predecessors are known
            if block_idx > 0 {
                builder.seal_block(cranelift_block);
            }
        }

        builder.finalize();
        Ok(())
    }
}

/// Get or create a variable for a VarId (free function to avoid borrow issues)
fn get_or_create_var(
    builder: &mut FunctionBuilder,
    var_id: u32,
    ty: types::Type,
    var_map: &mut HashMap<u32, Variable>,
    var_counter: &mut u32,
) -> Variable {
    if let Some(var) = var_map.get(&var_id) {
        *var
    } else {
        let var = Variable::from_u32(*var_counter);
        *var_counter += 1;
        builder.declare_var(var, ty);
        var_map.insert(var_id, var);
        var
    }
}

/// Compile a single instruction (free function to avoid borrow issues)
fn compile_instruction(
    builder: &mut FunctionBuilder,
    _module: &Module,
    _func: &Function,
    inst: &Instruction,
    var_map: &mut HashMap<u32, Variable>,
    var_counter: &mut u32,
    block_map: &HashMap<u32, Block>,
) -> JitResult<()> {
    match inst {
        Instruction::Const { dest, value, ty } => {
            let cranelift_ty = ir_type_to_cranelift(ty)?;
            let var = get_or_create_var(builder, dest.0, cranelift_ty, var_map, var_counter);

            let val = match value {
                Constant::Null => builder.ins().iconst(types::I64, 0),
                Constant::Bool(b) => builder.ins().iconst(types::I8, if *b { 1 } else { 0 }),
                Constant::Int(i) => builder.ins().iconst(types::I64, *i),
                Constant::Float(f) => builder.ins().f64const(*f),
                Constant::String(_) => {
                    // String constants are handled as pointers
                    // For now, just use a placeholder
                    builder.ins().iconst(types::I64, 0)
                }
            };

            builder.def_var(var, val);
        }

        Instruction::Binary {
            dest,
            op,
            left,
            right,
            ty,
        } => {
            let cranelift_ty = ir_type_to_cranelift(ty)?;
            let dest_var = get_or_create_var(builder, dest.0, cranelift_ty, var_map, var_counter);

            let left_var = var_map.get(&left.0).ok_or_else(|| {
                JitError::compilation(
                    format!("Undefined variable: {}", left),
                    format!("متغير غير معرّف: {}", left),
                )
            })?;
            let right_var = var_map.get(&right.0).ok_or_else(|| {
                JitError::compilation(
                    format!("Undefined variable: {}", right),
                    format!("متغير غير معرّف: {}", right),
                )
            })?;

            let left_val = builder.use_var(*left_var);
            let right_val = builder.use_var(*right_var);

            let result = match (op, ty) {
                (BinaryOp::Add, IrType::Int) => builder.ins().iadd(left_val, right_val),
                (BinaryOp::Sub, IrType::Int) => builder.ins().isub(left_val, right_val),
                (BinaryOp::Mul, IrType::Int) => builder.ins().imul(left_val, right_val),
                (BinaryOp::Div, IrType::Int) => builder.ins().sdiv(left_val, right_val),
                (BinaryOp::Mod, IrType::Int) => builder.ins().srem(left_val, right_val),

                (BinaryOp::Add, IrType::Float) => builder.ins().fadd(left_val, right_val),
                (BinaryOp::Sub, IrType::Float) => builder.ins().fsub(left_val, right_val),
                (BinaryOp::Mul, IrType::Float) => builder.ins().fmul(left_val, right_val),
                (BinaryOp::Div, IrType::Float) => builder.ins().fdiv(left_val, right_val),

                (BinaryOp::Eq, IrType::Int) | (BinaryOp::Eq, IrType::Bool) => {
                    let cmp = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::Equal,
                        left_val,
                        right_val,
                    );
                    builder.ins().uextend(types::I64, cmp)
                }
                (BinaryOp::Ne, IrType::Int) | (BinaryOp::Ne, IrType::Bool) => {
                    let cmp = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::NotEqual,
                        left_val,
                        right_val,
                    );
                    builder.ins().uextend(types::I64, cmp)
                }
                (BinaryOp::Lt, IrType::Int) => {
                    let cmp = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
                        left_val,
                        right_val,
                    );
                    builder.ins().uextend(types::I64, cmp)
                }
                (BinaryOp::Le, IrType::Int) => {
                    let cmp = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::SignedLessThanOrEqual,
                        left_val,
                        right_val,
                    );
                    builder.ins().uextend(types::I64, cmp)
                }
                (BinaryOp::Gt, IrType::Int) => {
                    let cmp = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThan,
                        left_val,
                        right_val,
                    );
                    builder.ins().uextend(types::I64, cmp)
                }
                (BinaryOp::Ge, IrType::Int) => {
                    let cmp = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThanOrEqual,
                        left_val,
                        right_val,
                    );
                    builder.ins().uextend(types::I64, cmp)
                }

                (BinaryOp::And, _) => builder.ins().band(left_val, right_val),
                (BinaryOp::Or, _) => builder.ins().bor(left_val, right_val),
                (BinaryOp::BitAnd, _) => builder.ins().band(left_val, right_val),
                (BinaryOp::BitOr, _) => builder.ins().bor(left_val, right_val),
                (BinaryOp::BitXor, _) => builder.ins().bxor(left_val, right_val),
                (BinaryOp::Shl, _) => builder.ins().ishl(left_val, right_val),
                (BinaryOp::Shr, _) => builder.ins().sshr(left_val, right_val),

                _ => {
                    return Err(JitError::unsupported_instruction(format!(
                        "Binary {:?} on {:?}",
                        op, ty
                    )));
                }
            };

            builder.def_var(dest_var, result);
        }

        Instruction::Unary {
            dest,
            op,
            operand,
            ty,
        } => {
            let cranelift_ty = ir_type_to_cranelift(ty)?;
            let dest_var = get_or_create_var(builder, dest.0, cranelift_ty, var_map, var_counter);

            let operand_var = var_map.get(&operand.0).ok_or_else(|| {
                JitError::compilation(
                    format!("Undefined variable: {}", operand),
                    format!("متغير غير معرّف: {}", operand),
                )
            })?;
            let operand_val = builder.use_var(*operand_var);

            let result = match (op, ty) {
                (UnaryOp::Neg, IrType::Int) => builder.ins().ineg(operand_val),
                (UnaryOp::Neg, IrType::Float) => builder.ins().fneg(operand_val),
                (UnaryOp::Not, _) => {
                    let one = builder.ins().iconst(types::I64, 1);
                    builder.ins().bxor(operand_val, one)
                }
                (UnaryOp::BitNot, _) => builder.ins().bnot(operand_val),
                _ => {
                    return Err(JitError::unsupported_instruction(format!(
                        "Unary {:?} on {:?}",
                        op, ty
                    )));
                }
            };

            builder.def_var(dest_var, result);
        }

        Instruction::Return { value } => {
            if let Some(var_id) = value {
                let var = var_map.get(&var_id.0).ok_or_else(|| {
                    JitError::compilation(
                        format!("Undefined variable: {}", var_id),
                        format!("متغير غير معرّف: {}", var_id),
                    )
                })?;
                let val = builder.use_var(*var);
                builder.ins().return_(&[val]);
            } else {
                builder.ins().return_(&[]);
            }
        }

        Instruction::Jump { target } => {
            let target_block = block_map.get(&target.0).ok_or_else(|| {
                JitError::compilation(
                    format!("Unknown block: {}", target),
                    format!("كتلة غير معروفة: {}", target),
                )
            })?;
            builder.ins().jump(*target_block, &[]);
        }

        Instruction::Branch {
            cond,
            then_block,
            else_block,
        } => {
            let cond_var = var_map.get(&cond.0).ok_or_else(|| {
                JitError::compilation(
                    format!("Undefined variable: {}", cond),
                    format!("متغير غير معرّف: {}", cond),
                )
            })?;
            let cond_val = builder.use_var(*cond_var);

            let then_blk = block_map.get(&then_block.0).ok_or_else(|| {
                JitError::compilation(
                    format!("Unknown block: {}", then_block),
                    format!("كتلة غير معروفة: {}", then_block),
                )
            })?;
            let else_blk = block_map.get(&else_block.0).ok_or_else(|| {
                JitError::compilation(
                    format!("Unknown block: {}", else_block),
                    format!("كتلة غير معروفة: {}", else_block),
                )
            })?;

            // Truncate to i8 for comparison
            let cond_i8 = builder.ins().ireduce(types::I8, cond_val);
            builder.ins().brif(cond_i8, *then_blk, &[], *else_blk, &[]);
        }

        // For now, emit a return for unsupported instructions
        // In a complete implementation, these would be handled properly
        Instruction::Call {
            dest: Some(d),
            func: _,
            args: _,
            ret_ty,
        } => {
            // TODO: Implement function calls
            // For now, just define a zero value for the destination
            let cranelift_ty = ir_type_to_cranelift(ret_ty)?;
            let dest_var = get_or_create_var(builder, d.0, cranelift_ty, var_map, var_counter);
            let zero = builder.ins().iconst(types::I64, 0);
            builder.def_var(dest_var, zero);
        }

        Instruction::Call { dest: None, .. } => {
            // Function call with no destination - nothing to do
        }

        _ => {
            // For other unsupported instructions, we skip them
            // A complete implementation would handle all instruction types
        }
    }

    Ok(())
}

impl BaselineCompiler {
    /// Estimate code size for a function
    fn estimate_code_size(&self, func: &Function) -> usize {
        // Rough estimate: 20 bytes per instruction on average
        let inst_count: usize = func.blocks.iter().map(|b| b.instructions.len()).sum();
        inst_count * 20
    }

    /// Get the function pointer for a compiled function
    pub fn get_function_ptr(&self, name: &str) -> Option<*const u8> {
        self.func_ids
            .get(name)
            .map(|id| self.jit_module.get_finalized_function(*id))
    }

    /// Get compilation statistics
    pub fn stats(&self) -> BaselineStats {
        BaselineStats {
            functions_compiled: self.functions_compiled,
            total_compile_time_ms: self.total_compile_time_ms,
            avg_compile_time_ms: if self.functions_compiled > 0 {
                self.total_compile_time_ms / self.functions_compiled
            } else {
                0
            },
        }
    }
}

impl Default for BaselineCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to create baseline compiler")
    }
}

/// Baseline compiler statistics
#[derive(Debug, Clone)]
pub struct BaselineStats {
    /// Number of functions compiled
    pub functions_compiled: u64,

    /// Total compilation time in milliseconds
    pub total_compile_time_ms: u64,

    /// Average compilation time per function
    pub avg_compile_time_ms: u64,
}

impl std::fmt::Display for BaselineStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Baseline Compiler Statistics / إحصائيات المترجم الأساسي")?;
        writeln!(f, "=======================================================")?;
        writeln!(
            f,
            "Functions compiled / الدوال المُترجَمة: {}",
            self.functions_compiled
        )?;
        writeln!(
            f,
            "Total compile time / إجمالي وقت الترجمة: {}ms",
            self.total_compile_time_ms
        )?;
        writeln!(
            f,
            "Avg compile time / متوسط وقت الترجمة: {}ms",
            self.avg_compile_time_ms
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BasicBlock, BlockId, FuncId, Parameter, VarId};

    fn create_simple_function() -> Function {
        let mut func = Function::new(
            FuncId("test_add".to_string()),
            "test_add".to_string(),
            vec![
                Parameter {
                    id: VarId(0),
                    name: "a".to_string(),
                    ty: IrType::Int,
                },
                Parameter {
                    id: VarId(1),
                    name: "b".to_string(),
                    ty: IrType::Int,
                },
            ],
            IrType::Int,
        );

        let mut block = BasicBlock::new(BlockId(0));
        block.instructions = vec![
            Instruction::Binary {
                dest: VarId(2),
                op: BinaryOp::Add,
                left: VarId(0),
                right: VarId(1),
                ty: IrType::Int,
            },
            Instruction::Return {
                value: Some(VarId(2)),
            },
        ];

        func.blocks.push(block);
        func
    }

    #[test]
    fn test_baseline_compiler_creation() {
        let compiler = BaselineCompiler::new();
        assert!(compiler.is_ok());
    }

    #[test]
    fn test_baseline_compiler_stats() {
        let compiler = BaselineCompiler::new().unwrap();
        let stats = compiler.stats();
        assert_eq!(stats.functions_compiled, 0);
        assert_eq!(stats.total_compile_time_ms, 0);
    }

    #[test]
    fn test_simple_function_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let module = Module::new("test".to_string());
        let func = create_simple_function();

        let result = compiler.compile(&module, &func);
        assert!(result.is_ok());

        let compiled = result.unwrap();
        assert_eq!(compiled.info.name, "test_add");
        assert_eq!(compiled.info.tier, CompilationTier::BaselineCompiled);
    }
}
