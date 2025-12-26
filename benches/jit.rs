//! JIT Compilation Benchmarks
//!
//! Measures JIT configuration and infrastructure performance.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tarqeem::ir::{
    BasicBlock, BinaryOp, BlockId, Constant, FuncId, Function, Instruction, IrType, Module, VarId,
};
use tarqeem::jit::{
    is_baseline_jit_available, is_optimizing_jit_available, BaselineCompiler, JitConfig,
    OptimizingCompiler,
};

/// Generate a simple IR module with arithmetic operations
fn generate_simple_module(func_count: usize, instr_count: usize) -> Module {
    let mut module = Module::new("benchmark_module".to_string());

    for f in 0..func_count {
        let func_name = format!("دالة_{}", f);
        let func_id = FuncId(func_name.clone());
        let mut func = Function::new(func_id, func_name, vec![], IrType::Int);

        // Create entry block
        let mut block = BasicBlock::new(BlockId(0));

        // Add constant instructions to the block
        for i in 0..instr_count {
            let var_id = VarId(i as u32);
            block.instructions.push(Instruction::Const {
                dest: var_id,
                value: Constant::Int(i as i64),
                ty: IrType::Int,
            });
        }

        // Add a return instruction
        block.instructions.push(Instruction::Return {
            value: Some(VarId(0)),
        });

        func.blocks.push(block);
        module.functions.push(func);
    }

    module
}

/// Generate an IR module with binary operations
fn generate_binary_module(op_count: usize) -> Module {
    let mut module = Module::new("binary_benchmark".to_string());

    let func_name = "دالة_حسابات".to_string();
    let func_id = FuncId(func_name.clone());
    let mut func = Function::new(func_id, func_name, vec![], IrType::Int);

    let mut block = BasicBlock::new(BlockId(0));

    // Initialize constants
    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(0),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(1),
        ty: IrType::Int,
    });

    // Add binary operations
    for i in 0..op_count {
        let dest = VarId(2 + i as u32);
        block.instructions.push(Instruction::Binary {
            dest,
            op: BinaryOp::Add,
            left: VarId(0),
            right: VarId(1),
            ty: IrType::Int,
        });
    }

    block.instructions.push(Instruction::Return {
        value: Some(VarId(0)),
    });

    func.blocks.push(block);
    module.functions.push(func);
    module
}

fn benchmark_config_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("jit_config_creation");

    group.bench_function("default", |b| b.iter(|| black_box(JitConfig::default())));

    group.bench_function("debug", |b| b.iter(|| black_box(JitConfig::debug())));

    group.bench_function("production", |b| {
        b.iter(|| black_box(JitConfig::production()))
    });

    group.finish();
}

fn benchmark_baseline_compilation(c: &mut Criterion) {
    if !is_baseline_jit_available() {
        eprintln!("Baseline JIT not available, skipping benchmarks");
        return;
    }

    let mut group = c.benchmark_group("jit_baseline_compile");

    for (funcs, instrs) in [(1, 10), (1, 50), (1, 100), (5, 20)].iter() {
        let module = generate_simple_module(*funcs, *instrs);
        let id = format!("{}funcs_{}instrs", funcs, instrs);

        group.bench_with_input(BenchmarkId::from_parameter(&id), &module, |b, module| {
            b.iter(|| {
                if let Ok(mut compiler) = BaselineCompiler::new() {
                    for func in &module.functions {
                        let _ = black_box(compiler.compile(module, func));
                    }
                }
            });
        });
    }

    group.finish();
}

fn benchmark_optimizing_compilation(c: &mut Criterion) {
    if !is_optimizing_jit_available() {
        eprintln!("Optimizing JIT not available, skipping benchmarks");
        return;
    }

    let mut group = c.benchmark_group("jit_optimizing_compile");

    for (funcs, instrs) in [(1, 10), (1, 50), (1, 100)].iter() {
        let module = generate_simple_module(*funcs, *instrs);
        let id = format!("{}funcs_{}instrs", funcs, instrs);

        group.bench_with_input(BenchmarkId::from_parameter(&id), &module, |b, module| {
            b.iter(|| {
                if let Ok(mut compiler) = OptimizingCompiler::new() {
                    for func in &module.functions {
                        let _ = black_box(compiler.compile(module, func));
                    }
                }
            });
        });
    }

    group.finish();
}

fn benchmark_binary_ops(c: &mut Criterion) {
    if !is_baseline_jit_available() {
        eprintln!("Baseline JIT not available, skipping binary ops benchmarks");
        return;
    }

    let mut group = c.benchmark_group("jit_binary_ops");

    for op_count in [10, 50, 100].iter() {
        let module = generate_binary_module(*op_count);

        group.bench_with_input(
            BenchmarkId::from_parameter(op_count),
            &module,
            |b, module| {
                b.iter(|| {
                    if let Ok(mut compiler) = BaselineCompiler::new() {
                        for func in &module.functions {
                            let _ = black_box(compiler.compile(module, func));
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

fn benchmark_module_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("jit_module_generation");

    for (funcs, instrs) in [(1, 10), (5, 20), (10, 50)].iter() {
        let id = format!("{}funcs_{}instrs", funcs, instrs);

        group.bench_function(BenchmarkId::from_parameter(&id), |b| {
            b.iter(|| black_box(generate_simple_module(*funcs, *instrs)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_config_creation,
    benchmark_module_generation,
    benchmark_baseline_compilation,
    benchmark_optimizing_compilation,
    benchmark_binary_ops,
);
criterion_main!(benches);
