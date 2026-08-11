//! Optimizer pass benchmarks
//!
//! Measures time per optimization pass and overall optimization effectiveness.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use tarqeem::ir::{OptLevel, Optimizer};
use tarqeem::{Analyzer, IrBuilder, Parser};

/// Generate source with constant expressions (good for constant folding)
fn generate_constant_folding_source(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");
    for i in 0..count {
        // These should be constant-folded
        source.push_str(&format!(
            "ثابت ث{} = {} + {} * {} - {} / 2\n",
            i,
            i * 10,
            i * 20,
            3,
            i * 6
        ));
    }
    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with dead code (good for DCE)
fn generate_dead_code_source(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    // Some live code
    source.push_str("متغير مستخدم = 0\n");

    // Dead assignments
    for i in 0..count {
        source.push_str(&format!("متغير ميت_{} = {} * 2\n", i, i));
    }

    // Use the live variable
    source.push_str("اطبع(مستخدم)\n");

    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with common subexpressions (good for CSE)
fn generate_cse_source(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");
    source.push_str("متغير أ = 10\n");
    source.push_str("متغير ب = 20\n");

    for i in 0..count {
        // Same expression repeated
        source.push_str(&format!("متغير س{} = أ + ب * 3\n", i));
        source.push_str(&format!("متغير ص{} = أ + ب * 3\n", i));
    }

    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with small functions (good for inlining)
fn generate_inlinable_source(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    // Define small functions
    for i in 0..count {
        source.push_str(&format!(
            r#"دالة صغيرة_{}(س: عدد) -> عدد {{
    أرجع س + {}
}}

"#,
            i, i
        ));
    }

    // Call them
    for i in 0..count {
        source.push_str(&format!("متغير ن{} = صغيرة_{}({})\n", i, i, i));
    }

    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with loops (good for loop optimizations)
fn generate_loop_source(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    for i in 0..count {
        source.push_str(&format!(
            r#"متغير مجموع_{} = 0
متغير ثابت_حلقة_{} = 10
لكل (متغير ع{} = 0؛ ع{} < 100؛ ع{}++) {{
    مجموع_{} = مجموع_{} + ثابت_حلقة_{}
}}

"#,
            i, i, i, i, i, i, i, i
        ));
    }

    source.push_str("الحمد_لله\n");
    source
}

/// Build IR module from source
fn build_ir_module(source: &str) -> tarqeem::ir::Module {
    let mut parser = Parser::new(source);
    let ast = parser.parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze(&ast).expect("Analysis should succeed");

    let ir_builder = IrBuilder::new("benchmark".to_string());
    ir_builder.build(&ast).expect("IR build should succeed")
}

fn optimizer_o0(c: &mut Criterion) {
    let source = generate_constant_folding_source(100);
    let mut group = c.benchmark_group("optimizer_O0");
    group.throughput(Throughput::Bytes(source.len() as u64));

    group.bench_function("no_optimization", |b| {
        b.iter(|| {
            let mut module = build_ir_module(&source);
            let mut optimizer = Optimizer::new(OptLevel::O0);
            optimizer.optimize(black_box(&mut module));
            black_box(module)
        });
    });

    group.finish();
}

fn optimizer_o1(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimizer_O1");

    // Constant folding benchmark
    let const_source = generate_constant_folding_source(100);
    group.bench_function("constant_folding", |b| {
        b.iter(|| {
            let mut module = build_ir_module(&const_source);
            let mut optimizer = Optimizer::new(OptLevel::O1);
            optimizer.optimize(black_box(&mut module));
            black_box(module)
        });
    });

    // DCE benchmark
    let dce_source = generate_dead_code_source(100);
    group.bench_function("dead_code_elimination", |b| {
        b.iter(|| {
            let mut module = build_ir_module(&dce_source);
            let mut optimizer = Optimizer::new(OptLevel::O1);
            optimizer.optimize(black_box(&mut module));
            black_box(module)
        });
    });

    group.finish();
}

fn optimizer_o2(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimizer_O2");

    // CSE benchmark
    let cse_source = generate_cse_source(50);
    group.bench_function("cse", |b| {
        b.iter(|| {
            let mut module = build_ir_module(&cse_source);
            let mut optimizer = Optimizer::new(OptLevel::O2);
            optimizer.optimize(black_box(&mut module));
            black_box(module)
        });
    });

    // Loop optimization benchmark
    let loop_source = generate_loop_source(20);
    group.bench_function("loop_optimization", |b| {
        b.iter(|| {
            let mut module = build_ir_module(&loop_source);
            let mut optimizer = Optimizer::new(OptLevel::O2);
            optimizer.optimize(black_box(&mut module));
            black_box(module)
        });
    });

    group.finish();
}

fn optimizer_o3(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimizer_O3");

    // Inlining benchmark
    let inline_source = generate_inlinable_source(20);
    group.bench_function("inlining", |b| {
        b.iter(|| {
            let mut module = build_ir_module(&inline_source);
            let mut optimizer = Optimizer::new(OptLevel::O3);
            optimizer.optimize(black_box(&mut module));
            black_box(module)
        });
    });

    group.finish();
}

fn optimizer_levels_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimizer_levels");

    // Use a representative mixed source
    let source = r#"بسم_الله

ثابت ث١ = 10 + 20 * 3
ثابت ث٢ = 100 / 4 - 5

دالة حساب(س: عدد، ص: عدد) -> عدد {
    متغير أ = س + ص
    متغير ب = س * ص
    أرجع أ + ب
}

متغير مجموع = 0
لكل (متغير ع = 0؛ ع < 10؛ ع++) {
    مجموع = مجموع + حساب(ع، ع + 1)
}

متغير غير_مستخدم = 999

اطبع(مجموع)

الحمد_لله
"#;

    for level in [OptLevel::O0, OptLevel::O1, OptLevel::O2, OptLevel::O3].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}", level)),
            level,
            |b, level| {
                b.iter(|| {
                    let mut module = build_ir_module(source);
                    let mut optimizer = Optimizer::new(*level);
                    optimizer.optimize(black_box(&mut module));
                    black_box(module)
                });
            },
        );
    }

    group.finish();
}

fn optimizer_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimizer_scalability");

    for size in [10, 50, 100, 200].iter() {
        let source = generate_constant_folding_source(*size);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &source, |b, source| {
            b.iter(|| {
                let mut module = build_ir_module(source);
                let mut optimizer = Optimizer::new(OptLevel::O2);
                optimizer.optimize(black_box(&mut module));
                black_box(module)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    optimizer_o0,
    optimizer_o1,
    optimizer_o2,
    optimizer_o3,
    optimizer_levels_comparison,
    optimizer_scalability,
);
criterion_main!(benches);
