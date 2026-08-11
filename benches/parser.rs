//! Parser speed benchmarks
//!
//! Measures AST nodes/second for various source file sizes and complexity levels.

// Bench closures return Result<Ast, Diagnostic> straight into black_box;
// boxing the error type here would only distort the measurement.
#![allow(clippy::result_large_err)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use tarqeem::Parser;

/// Generate a simple source file with repeated variable declarations
fn generate_simple_source(lines: usize) -> String {
    let mut source = String::from("بسم_الله\n");
    for i in 0..lines {
        source.push_str(&format!("متغير س{} = {}\n", i, i * 2));
    }
    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with complex expressions
fn generate_complex_expressions(lines: usize) -> String {
    let mut source = String::from("بسم_الله\n");
    for i in 0..lines {
        source.push_str(&format!(
            "متغير نتيجة_{} = ({} + {} * 3) / ({} - 1) + ({} % 7)\n",
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

/// Generate source with nested control flow
fn generate_control_flow(depth: usize) -> String {
    let mut source = String::from("بسم_الله\n");
    source.push_str("متغير س = 0\n");

    for _ in 0..depth {
        source.push_str("إذا (س < 10) {\n");
        source.push_str("    س = س + 1\n");
    }

    for _ in 0..depth {
        source.push_str("}\n");
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
    أرجع ج + د
}}

"#,
            i
        ));
    }

    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with class definitions
fn generate_classes(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    for i in 0..count {
        source.push_str(&format!(
            r#"صنف صنف_{} {{
    خاص حقل١: عدد
    خاص حقل٢: نص

    منشئ(ق١: عدد، ق٢: نص) {{
        هذا.حقل١ = ق١
        هذا.حقل٢ = ق٢
    }}

    عام دالة احصل_قيمة() -> عدد {{
        أرجع هذا.حقل١
    }}

    عام دالة عيّن_قيمة(قيمة: عدد) {{
        هذا.حقل١ = قيمة
    }}
}}

"#,
            i
        ));
    }

    source.push_str("الحمد_لله\n");
    source
}

fn parser_simple_statements(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_simple");

    for size in [10, 100, 500, 1000].iter() {
        let source = generate_simple_source(*size);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &source, |b, source| {
            b.iter(|| {
                let mut parser = Parser::new(black_box(source));
                let ast = parser.parse();
                black_box(ast)
            });
        });
    }

    group.finish();
}

fn parser_complex_expressions(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_expressions");

    for size in [10, 100, 500, 1000].iter() {
        let source = generate_complex_expressions(*size);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &source, |b, source| {
            b.iter(|| {
                let mut parser = Parser::new(black_box(source));
                let ast = parser.parse();
                black_box(ast)
            });
        });
    }

    group.finish();
}

fn parser_control_flow(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_control_flow");

    for depth in [5, 10, 25, 50].iter() {
        let source = generate_control_flow(*depth);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(depth), &source, |b, source| {
            b.iter(|| {
                let mut parser = Parser::new(black_box(source));
                let ast = parser.parse();
                black_box(ast)
            });
        });
    }

    group.finish();
}

fn parser_functions(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_functions");

    for count in [5, 10, 25, 50].iter() {
        let source = generate_functions(*count);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), &source, |b, source| {
            b.iter(|| {
                let mut parser = Parser::new(black_box(source));
                let ast = parser.parse();
                black_box(ast)
            });
        });
    }

    group.finish();
}

fn parser_classes(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_classes");

    for count in [5, 10, 25, 50].iter() {
        let source = generate_classes(*count);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), &source, |b, source| {
            b.iter(|| {
                let mut parser = Parser::new(black_box(source));
                let ast = parser.parse();
                black_box(ast)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    parser_simple_statements,
    parser_complex_expressions,
    parser_control_flow,
    parser_functions,
    parser_classes,
);
criterion_main!(benches);
