//! IR generation benchmarks
//!
//! Measures IR instructions/second for various program structures.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use tarqeem::{Analyzer, IrBuilder, Parser};

/// Generate source with arithmetic expressions
fn generate_arithmetic(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");
    for i in 0..count {
        source.push_str(&format!(
            "متغير ن{} = {} + {} * {} - {} / 2\n",
            i,
            i,
            i + 1,
            i + 2,
            i + 3
        ));
    }
    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with function definitions
fn generate_functions(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    for i in 0..count {
        source.push_str(&format!(
            r#"دالة دالة_{}(أ: عدد، ب: عدد) -> عدد {{
    متغير ج = أ + ب
    متغير د = أ * ب
    متغير هـ = ج - د
    أرجع هـ
}}

"#,
            i
        ));
    }

    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with control flow
fn generate_control_flow(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    for i in 0..count {
        source.push_str(&format!(
            r#"متغير س{} = {}
إذا (س{} > 5) {{
    س{} = س{} * 2
}} وإلا {{
    س{} = س{} + 1
}}

"#,
            i, i, i, i, i, i, i
        ));
    }

    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with loops
fn generate_loops(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    for i in 0..count {
        source.push_str(&format!(
            r#"متغير مجموع_{} = 0
لكل (متغير ع = 0؛ ع < 10؛ ع++) {{
    مجموع_{} = مجموع_{} + ع
}}

"#,
            i, i, i
        ));
    }

    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with class instantiation
fn generate_classes(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    // Define a class
    source.push_str(
        r#"صنف نقطة {
    خاص س: عدد
    خاص ص: عدد

    منشئ(س: عدد، ص: عدد) {
        هذا.س = س
        هذا.ص = ص
    }

    عام دالة احصل_س() -> عدد {
        أرجع هذا.س
    }

    عام دالة احصل_ص() -> عدد {
        أرجع هذا.ص
    }
}

"#,
    );

    // Create instances
    for i in 0..count {
        source.push_str(&format!("متغير نقطة_{} = جديد نقطة({}, {})\n", i, i, i * 2));
    }

    source.push_str("الحمد_لله\n");
    source
}

fn parse_and_analyze(source: &str) -> tarqeem::parser::Ast {
    let mut parser = Parser::new(source);
    let ast = parser.parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze(&ast).expect("Analysis should succeed");
    ast
}

fn ir_arithmetic(c: &mut Criterion) {
    let mut group = c.benchmark_group("ir_arithmetic");

    for count in [10, 50, 100, 500].iter() {
        let source = generate_arithmetic(*count);
        group.throughput(Throughput::Bytes(source.len() as u64));

        let ast = parse_and_analyze(&source);

        group.bench_with_input(BenchmarkId::from_parameter(count), &ast, |b, ast| {
            b.iter(|| {
                let ir_builder = IrBuilder::new("benchmark".to_string());
                let result = ir_builder.build(black_box(ast));
                black_box(result)
            });
        });
    }

    group.finish();
}

fn ir_functions(c: &mut Criterion) {
    let mut group = c.benchmark_group("ir_functions");

    for count in [5, 10, 25, 50].iter() {
        let source = generate_functions(*count);
        group.throughput(Throughput::Bytes(source.len() as u64));

        let ast = parse_and_analyze(&source);

        group.bench_with_input(BenchmarkId::from_parameter(count), &ast, |b, ast| {
            b.iter(|| {
                let ir_builder = IrBuilder::new("benchmark".to_string());
                let result = ir_builder.build(black_box(ast));
                black_box(result)
            });
        });
    }

    group.finish();
}

fn ir_control_flow(c: &mut Criterion) {
    let mut group = c.benchmark_group("ir_control_flow");

    for count in [10, 25, 50, 100].iter() {
        let source = generate_control_flow(*count);
        group.throughput(Throughput::Bytes(source.len() as u64));

        let ast = parse_and_analyze(&source);

        group.bench_with_input(BenchmarkId::from_parameter(count), &ast, |b, ast| {
            b.iter(|| {
                let ir_builder = IrBuilder::new("benchmark".to_string());
                let result = ir_builder.build(black_box(ast));
                black_box(result)
            });
        });
    }

    group.finish();
}

fn ir_loops(c: &mut Criterion) {
    let mut group = c.benchmark_group("ir_loops");

    for count in [5, 10, 25, 50].iter() {
        let source = generate_loops(*count);
        group.throughput(Throughput::Bytes(source.len() as u64));

        let ast = parse_and_analyze(&source);

        group.bench_with_input(BenchmarkId::from_parameter(count), &ast, |b, ast| {
            b.iter(|| {
                let ir_builder = IrBuilder::new("benchmark".to_string());
                let result = ir_builder.build(black_box(ast));
                black_box(result)
            });
        });
    }

    group.finish();
}

fn ir_classes(c: &mut Criterion) {
    let mut group = c.benchmark_group("ir_classes");

    for count in [5, 10, 25, 50].iter() {
        let source = generate_classes(*count);
        group.throughput(Throughput::Bytes(source.len() as u64));

        let ast = parse_and_analyze(&source);

        group.bench_with_input(BenchmarkId::from_parameter(count), &ast, |b, ast| {
            b.iter(|| {
                let ir_builder = IrBuilder::new("benchmark".to_string());
                let result = ir_builder.build(black_box(ast));
                black_box(result)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    ir_arithmetic,
    ir_functions,
    ir_control_flow,
    ir_loops,
    ir_classes,
);
criterion_main!(benches);
