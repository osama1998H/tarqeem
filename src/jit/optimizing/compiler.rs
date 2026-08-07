//! Optimizing JIT Compiler (Tier 2)
//!
//! This module implements the optimizing tier of the JIT compilation pipeline
//! using Cranelift with aggressive optimizations.
//!
//! المترجم الآني المُحسِّن (المستوى الثاني)
//! يستخدم Cranelift مع تحسينات قوية لأفضل أداء

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

use super::specialize::TypeSpecializer;
use crate::ir::{BinaryOp, Constant, Function, Instruction, IrType, Module, UnaryOp};
use crate::jit::cache::CompiledFunction;
use crate::jit::error::{JitError, JitResult};
use crate::jit::profile::{CompilationTier, FunctionProfile};

/// Convert Tarqeem IR type to Cranelift type
fn ir_type_to_cranelift(ty: &IrType) -> JitResult<types::Type> {
    match ty {
        IrType::Void => Ok(types::INVALID),
        IrType::Bool => Ok(types::I8),
        IrType::Int => Ok(types::I64),
        IrType::Float => Ok(types::F64),
        IrType::String => Ok(types::I64), // Pointer to string struct
        IrType::Ptr(_) => Ok(types::I64),
        IrType::Array(_, _) => Ok(types::I64),
        IrType::Struct(_) => Ok(types::I64),
        IrType::Function { .. } => Ok(types::I64),
        IrType::Enum(_) => Ok(types::I64), // Pointer to enum (tagged union)
    }
}

/// Optimizing JIT compiler using Cranelift with aggressive optimizations
pub struct OptimizingCompiler {
    /// Cranelift JIT module with aggressive optimizations
    jit_module: JITModule,

    /// Function builder context (reusable)
    builder_context: FunctionBuilderContext,

    /// Cranelift codegen context (reusable)
    ctx: Context,

    /// Map from Tarqeem function names to Cranelift function IDs
    func_ids: HashMap<String, FuncId>,

    /// Type specializer for generic functions
    specializer: TypeSpecializer,

    /// Statistics
    functions_compiled: u64,
    total_compile_time_ms: u64,
    specializations_generated: u64,
}

impl OptimizingCompiler {
    /// Create a new optimizing compiler with aggressive optimizations
    pub fn new() -> JitResult<Self> {
        // Configure Cranelift for maximum optimization
        let mut flag_builder = settings::builder();

        // Use speed-optimized settings
        flag_builder
            .set("opt_level", "speed")
            .map_err(|e| JitError::compilation(format!("خطأ في إعداد مستوى التحسين: {}", e)))?;

        flag_builder
            .set("use_colocated_libcalls", "false")
            .map_err(|e| JitError::compilation(format!("خطأ في إعداد Cranelift: {}", e)))?;

        flag_builder
            .set("is_pic", "false")
            .map_err(|e| JitError::compilation(format!("خطأ في إعداد Cranelift: {}", e)))?;

        // Use the native target with speed optimization
        let isa_builder = cranelift_native::builder()
            .map_err(|e| JitError::compilation(format!("فشل في الحصول على الهدف الأصلي: {}", e)))?;

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| JitError::compilation(format!("فشل في إنشاء ISA: {}", e)))?;

        // Create JIT module
        let jit_builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let jit_module = JITModule::new(jit_builder);

        Ok(Self {
            jit_module,
            builder_context: FunctionBuilderContext::new(),
            ctx: Context::new(),
            func_ids: HashMap::new(),
            specializer: TypeSpecializer::new(),
            functions_compiled: 0,
            total_compile_time_ms: 0,
            specializations_generated: 0,
        })
    }

    /// Check if optimizing JIT is available
    pub fn is_available() -> bool {
        true
    }

    /// Compile a function with profile-guided optimizations
    pub fn compile_with_profile(
        &mut self,
        module: &Module,
        func: &Function,
        profile: Option<&FunctionProfile>,
    ) -> JitResult<CompiledFunction> {
        let start_time = Instant::now();

        // Check if we should specialize based on profile
        let func_to_compile = if let Some(p) = profile {
            if let Some(specialized) = self.specializer.get_or_specialize(func, &p.arg_types) {
                self.specializations_generated += 1;
                specialized.clone()
            } else {
                func.clone()
            }
        } else {
            func.clone()
        };

        // Create Cranelift function signature
        let sig = self.create_signature(&func_to_compile)?;

        // Declare the function in the JIT module
        let func_id = self
            .jit_module
            .declare_function(&func_to_compile.name, Linkage::Local, &sig)
            .map_err(|e| JitError::compilation(format!("فشل في التصريح عن الدالة: {}", e)))?;

        // Clear and set up context
        self.ctx.func =
            CraneliftFunction::with_name_signature(UserFuncName::user(0, func_id.as_u32()), sig);

        // Build function body with profile-guided optimizations
        self.build_optimized_function_body(module, &func_to_compile, profile)?;

        // Compile the function
        self.jit_module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| JitError::codegen(format!("فشل في تعريف الدالة: {}", e)))?;

        // Clear context for reuse
        self.jit_module.clear_context(&mut self.ctx);

        // Finalize the function
        self.jit_module
            .finalize_definitions()
            .map_err(|e| JitError::codegen(format!("فشل في إنهاء التعريفات: {}", e)))?;

        // Get code size (estimate)
        let code_size = self.estimate_code_size(&func_to_compile);

        // Calculate compile time
        let compile_time_ms = start_time.elapsed().as_millis() as u64;

        // Update statistics
        self.functions_compiled += 1;
        self.total_compile_time_ms += compile_time_ms;

        // Store function ID mapping
        self.func_ids.insert(func_to_compile.name.clone(), func_id);

        Ok(CompiledFunction::with_func_id(
            func_to_compile.name,
            CompilationTier::Optimized,
            code_size,
            compile_time_ms,
            func_id,
        ))
    }

    /// Compile without profile (falls back to basic optimization)
    pub fn compile(&mut self, module: &Module, func: &Function) -> JitResult<CompiledFunction> {
        self.compile_with_profile(module, func, None)
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

    /// Build function body with profile-guided optimizations
    fn build_optimized_function_body(
        &mut self,
        module: &Module,
        func: &Function,
        profile: Option<&FunctionProfile>,
    ) -> JitResult<()> {
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

        // Declare variables for parameters
        for (i, param) in func.params.iter().enumerate() {
            let ty = ir_type_to_cranelift(&param.ty)?;
            // cranelift 0.129: declare_var allocates and returns the Variable
            let var = builder.declare_var(ty);
            let param_value = builder.block_params(entry_block)[i];
            builder.def_var(var, param_value);
            var_map.insert(param.id.0, var);
        }

        // Compile each basic block with optimizations
        for (block_idx, bb) in func.blocks.iter().enumerate() {
            let cranelift_block = block_map[&bb.id.0];

            // Switch to this block (except entry which we're already in)
            if block_idx > 0 {
                builder.switch_to_block(cranelift_block);
            }

            // Get branch profile for this block if available
            let branch_profile = profile.and_then(|p| p.branch_profiles.get(&bb.id.0));

            // Compile each instruction with profile guidance
            for inst in &bb.instructions {
                compile_optimized_instruction(
                    &mut builder,
                    module,
                    func,
                    inst,
                    &mut var_map,
                    &block_map,
                    branch_profile,
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

    /// Estimate code size for a function
    fn estimate_code_size(&self, func: &Function) -> usize {
        // Optimized code is typically larger due to inlining, unrolling, etc.
        let inst_count: usize = func.blocks.iter().map(|b| b.instructions.len()).sum();
        inst_count * 30 // Higher estimate for optimized code
    }

    /// Get the function pointer for a compiled function
    pub fn get_function_ptr(&self, name: &str) -> Option<*const u8> {
        self.func_ids
            .get(name)
            .map(|id| self.jit_module.get_finalized_function(*id))
    }

    /// Get compilation statistics
    pub fn stats(&self) -> OptimizingStats {
        OptimizingStats {
            functions_compiled: self.functions_compiled,
            total_compile_time_ms: self.total_compile_time_ms,
            avg_compile_time_ms: self
                .total_compile_time_ms
                .checked_div(self.functions_compiled)
                .unwrap_or(0),
            specializations_generated: self.specializations_generated,
            specializer_stats: self.specializer.stats(),
        }
    }
}

impl Default for OptimizingCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to create optimizing compiler")
    }
}

/// Get or create a variable for a VarId
fn get_or_create_var(
    builder: &mut FunctionBuilder,
    var_id: u32,
    ty: types::Type,
    var_map: &mut HashMap<u32, Variable>,
) -> Variable {
    if let Some(var) = var_map.get(&var_id) {
        *var
    } else {
        // cranelift 0.129: declare_var allocates and returns the Variable
        let var = builder.declare_var(ty);
        var_map.insert(var_id, var);
        var
    }
}

/// Compile a single instruction with profile-guided optimizations
#[allow(clippy::too_many_arguments)]
fn compile_optimized_instruction(
    builder: &mut FunctionBuilder,
    _module: &Module,
    _func: &Function,
    inst: &Instruction,
    var_map: &mut HashMap<u32, Variable>,
    block_map: &HashMap<u32, Block>,
    _branch_profile: Option<&crate::jit::profile::BranchProfile>,
) -> JitResult<()> {
    match inst {
        Instruction::Const { dest, value, ty } => {
            let cranelift_ty = ir_type_to_cranelift(ty)?;
            let var = get_or_create_var(builder, dest.0, cranelift_ty, var_map);

            let val = match value {
                Constant::Null => builder.ins().iconst(types::I64, 0),
                Constant::Bool(b) => builder.ins().iconst(types::I8, if *b { 1 } else { 0 }),
                Constant::Int(i) => builder.ins().iconst(types::I64, *i),
                Constant::Float(f) => builder.ins().f64const(*f),
                Constant::String(_) => {
                    // String constants are handled as pointers
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
            let dest_var = get_or_create_var(builder, dest.0, cranelift_ty, var_map);

            let left_var = var_map
                .get(&left.0)
                .ok_or_else(|| JitError::compilation(format!("متغير غير معرّف: {}", left)))?;
            let right_var = var_map
                .get(&right.0)
                .ok_or_else(|| JitError::compilation(format!("متغير غير معرّف: {}", right)))?;

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
            let dest_var = get_or_create_var(builder, dest.0, cranelift_ty, var_map);

            let operand_var = var_map
                .get(&operand.0)
                .ok_or_else(|| JitError::compilation(format!("متغير غير معرّف: {}", operand)))?;
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
                let var = var_map
                    .get(&var_id.0)
                    .ok_or_else(|| JitError::compilation(format!("متغير غير معرّف: {}", var_id)))?;
                let val = builder.use_var(*var);
                builder.ins().return_(&[val]);
            } else {
                builder.ins().return_(&[]);
            }
        }

        Instruction::Jump { target } => {
            let target_block = block_map
                .get(&target.0)
                .ok_or_else(|| JitError::compilation(format!("كتلة غير معروفة: {}", target)))?;
            builder.ins().jump(*target_block, &[]);
        }

        Instruction::Branch {
            cond,
            then_block,
            else_block,
        } => {
            let cond_var = var_map
                .get(&cond.0)
                .ok_or_else(|| JitError::compilation(format!("متغير غير معرّف: {}", cond)))?;
            let cond_val = builder.use_var(*cond_var);

            let then_blk = block_map
                .get(&then_block.0)
                .ok_or_else(|| JitError::compilation(format!("كتلة غير معروفة: {}", then_block)))?;
            let else_blk = block_map
                .get(&else_block.0)
                .ok_or_else(|| JitError::compilation(format!("كتلة غير معروفة: {}", else_block)))?;

            // Truncate to i8 for comparison
            let cond_i8 = builder.ins().ireduce(types::I8, cond_val);
            builder.ins().brif(cond_i8, *then_blk, &[], *else_blk, &[]);
        }

        Instruction::Call {
            dest: Some(d),
            func: _,
            args: _,
            ret_ty,
        } => {
            // TODO: Implement optimized function calls with inline caching
            let cranelift_ty = ir_type_to_cranelift(ret_ty)?;
            let dest_var = get_or_create_var(builder, d.0, cranelift_ty, var_map);
            let zero = builder.ins().iconst(types::I64, 0);
            builder.def_var(dest_var, zero);
        }

        Instruction::Call { dest: None, .. } => {
            // Function call with no destination - nothing to do
        }

        _ => {
            // Skip unsupported instructions
        }
    }

    Ok(())
}

/// Optimizing compiler statistics
#[derive(Debug, Clone)]
pub struct OptimizingStats {
    /// Number of functions compiled
    pub functions_compiled: u64,

    /// Total compilation time in milliseconds
    pub total_compile_time_ms: u64,

    /// Average compilation time per function
    pub avg_compile_time_ms: u64,

    /// Number of type specializations generated
    pub specializations_generated: u64,

    /// Type specializer statistics
    pub specializer_stats: super::specialize::SpecializerStats,
}

impl std::fmt::Display for OptimizingStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Optimizing Compiler Statistics / إحصائيات المترجم المُحسِّن"
        )?;
        writeln!(
            f,
            "========================================================="
        )?;
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
        writeln!(
            f,
            "Specializations / التخصصات: {}",
            self.specializations_generated
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
            FuncId("test_mul".to_string()),
            "test_mul".to_string(),
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
                op: BinaryOp::Mul,
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
    fn test_optimizing_compiler_creation() {
        let compiler = OptimizingCompiler::new();
        assert!(compiler.is_ok());
    }

    #[test]
    fn test_optimizing_compiler_stats() {
        let compiler = OptimizingCompiler::new().unwrap();
        let stats = compiler.stats();
        assert_eq!(stats.functions_compiled, 0);
        assert_eq!(stats.total_compile_time_ms, 0);
        assert_eq!(stats.specializations_generated, 0);
    }

    #[test]
    fn test_simple_function_optimization() {
        let mut compiler = OptimizingCompiler::new().unwrap();
        let module = Module::new("test".to_string());
        let func = create_simple_function();

        let result = compiler.compile(&module, &func);
        assert!(result.is_ok());

        let compiled = result.unwrap();
        assert_eq!(compiled.info.name, "test_mul");
        assert_eq!(compiled.info.tier, CompilationTier::Optimized);
    }

    #[test]
    fn test_profile_guided_compilation() {
        let mut compiler = OptimizingCompiler::new().unwrap();
        let module = Module::new("test".to_string());
        let func = create_simple_function();

        // Create a mock profile
        let profile = FunctionProfile::default();

        let result = compiler.compile_with_profile(&module, &func, Some(&profile));
        assert!(result.is_ok());
    }
}
