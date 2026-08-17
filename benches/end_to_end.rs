//! End-to-end compilation benchmarks
//!
//! Measures full compilation pipeline time for various program sizes.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use tarqeem::ir::{OptLevel, Optimizer};
use tarqeem::{Analyzer, IrBuilder, Lexer, Parser};

/// Generate a representative "hello world" program
fn generate_hello_world() -> String {
    r#"بسم_الله

اطبع("مرحباً بالعالم!")

الحمد_لله
"#
    .to_string()
}

/// Generate a simple program with variables and expressions
fn generate_simple_program(vars: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    for i in 0..vars {
        source.push_str(&format!("متغير س{} = {} + {} * 2\n", i, i, i + 1));
    }

    source.push_str("الحمد_لله\n");
    source
}

/// Generate a program with functions
fn generate_function_program(funcs: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    for i in 0..funcs {
        source.push_str(&format!(
            r#"دالة دالة_{}(س: عدد) -> عدد {{
    متغير ن = س * 2
    أرجع ن + {}
}}

"#,
            i, i
        ));
    }

    // Call all functions
    for i in 0..funcs {
        source.push_str(&format!("متغير ن{} = دالة_{}({})\n", i, i, i));
    }

    source.push_str("الحمد_لله\n");
    source
}

/// Generate a program with classes
fn generate_class_program(classes: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    for i in 0..classes {
        source.push_str(&format!(
            r#"صنف صنف_{} {{
    خاص قيمة: عدد;

    منشئ(ق: عدد) {{
        هذا.قيمة = ق;
    }}

    عام دالة احصل_قيمة() -> عدد {{
        أرجع هذا.قيمة;
    }}
}}

"#,
            i
        ));
    }

    // Instantiate all classes
    for i in 0..classes {
        source.push_str(&format!("متغير ك{} = جديد صنف_{}({})\n", i, i, i * 10));
    }

    source.push_str("الحمد_لله\n");
    source
}

/// Generate a realistic mixed program
fn generate_mixed_program(scale: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    // Constants
    for i in 0..scale {
        source.push_str(&format!("ثابت ث{} = {} * 10\n", i, i));
    }

    // Functions
    for i in 0..(scale / 2) {
        source.push_str(&format!(
            r#"دالة حساب_{}(أ: عدد، ب: عدد) -> عدد {{
    متغير ج = أ + ب
    أرجع ج * 2
}}

"#,
            i
        ));
    }

    // Classes
    for i in 0..(scale / 4) {
        source.push_str(&format!(
            r#"صنف كائن_{} {{
    خاص بيانات: عدد;

    منشئ(ب: عدد) {{
        هذا.بيانات = ب;
    }}

    عام دالة معالجة() -> عدد {{
        أرجع هذا.بيانات + 1;
    }}
}}

"#,
            i
        ));
    }

    // Main logic
    source.push_str("متغير مجموع = 0\n");
    for i in 0..(scale / 2) {
        source.push_str(&format!(
            "مجموع = مجموع + حساب_{}({}, {})\n",
            i % (scale / 2),
            i,
            i + 1
        ));
    }

    source.push_str("اطبع(مجموع)\n");
    source.push_str("الحمد_لله\n");
    source
}

/// Benchmark just lexing phase
fn benchmark_lexer_phase(source: &str) {
    let mut lexer = Lexer::new(source);
    let _tokens = lexer.tokenize();
}

/// Benchmark lexing + parsing
fn benchmark_parse_phase(source: &str) {
    let mut parser = Parser::new(source);
    let _ast = parser.parse();
}

/// Benchmark lexing + parsing + semantic analysis
fn benchmark_semantic_phase(source: &str) {
    let mut parser = Parser::new(source);
    let ast = parser.parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let _result = analyzer.analyze(&ast);
}

/// Benchmark full compilation to IR
fn benchmark_ir_phase(source: &str) {
    let mut parser = Parser::new(source);
    let ast = parser.parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze(&ast).expect("Analysis should succeed");

    let ir_builder = IrBuilder::new("benchmark".to_string());
    let _module = ir_builder.build(&ast);
}

/// Benchmark full compilation with optimization
fn benchmark_full_pipeline(source: &str, opt_level: OptLevel) {
    let mut parser = Parser::new(source);
    let ast = parser.parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze(&ast).expect("Analysis should succeed");

    let ir_builder = IrBuilder::new("benchmark".to_string());
    // Some corpora are declaration-only; they measure pipeline throughput,
    // not entry-point policy, so they lower as library modules.
    let mut module = ir_builder
        .build_library(&ast)
        .expect("IR build should succeed");

    let mut optimizer = Optimizer::new(opt_level);
    optimizer.optimize(&mut module);
}

fn end_to_end_hello_world(c: &mut Criterion) {
    let source = generate_hello_world();
    let mut group = c.benchmark_group("end_to_end_hello");
    group.throughput(Throughput::Bytes(source.len() as u64));

    group.bench_function("full_pipeline", |b| {
        b.iter(|| benchmark_full_pipeline(black_box(&source), OptLevel::O2));
    });

    group.finish();
}

fn end_to_end_phases(c: &mut Criterion) {
    let source = generate_mixed_program(20);
    let mut group = c.benchmark_group("end_to_end_phases");
    group.throughput(Throughput::Bytes(source.len() as u64));

    group.bench_function("1_lexer", |b| {
        b.iter(|| benchmark_lexer_phase(black_box(&source)));
    });

    group.bench_function("2_parser", |b| {
        b.iter(|| benchmark_parse_phase(black_box(&source)));
    });

    group.bench_function("3_semantic", |b| {
        b.iter(|| benchmark_semantic_phase(black_box(&source)));
    });

    group.bench_function("4_ir", |b| {
        b.iter(|| benchmark_ir_phase(black_box(&source)));
    });

    group.bench_function("5_full_O0", |b| {
        b.iter(|| benchmark_full_pipeline(black_box(&source), OptLevel::O0));
    });

    group.bench_function("6_full_O2", |b| {
        b.iter(|| benchmark_full_pipeline(black_box(&source), OptLevel::O2));
    });

    group.finish();
}

fn end_to_end_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end_scale");

    for scale in [10, 25, 50, 100].iter() {
        let source = generate_mixed_program(*scale);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("scale_{}", scale)),
            &source,
            |b, source| {
                b.iter(|| benchmark_full_pipeline(black_box(source), OptLevel::O2));
            },
        );
    }

    group.finish();
}

fn end_to_end_by_type(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end_type");

    // Simple variables
    let simple = generate_simple_program(50);
    group.bench_function("simple_vars", |b| {
        b.iter(|| benchmark_full_pipeline(black_box(&simple), OptLevel::O2));
    });

    // Functions
    let funcs = generate_function_program(20);
    group.bench_function("functions", |b| {
        b.iter(|| benchmark_full_pipeline(black_box(&funcs), OptLevel::O2));
    });

    // Classes
    let classes = generate_class_program(10);
    group.bench_function("classes", |b| {
        b.iter(|| benchmark_full_pipeline(black_box(&classes), OptLevel::O2));
    });

    group.finish();
}

fn end_to_end_opt_levels(c: &mut Criterion) {
    let source = generate_mixed_program(30);
    let mut group = c.benchmark_group("end_to_end_opt_levels");
    group.throughput(Throughput::Bytes(source.len() as u64));

    for level in [OptLevel::O0, OptLevel::O1, OptLevel::O2, OptLevel::O3].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}", level)),
            level,
            |b, level| {
                b.iter(|| benchmark_full_pipeline(black_box(&source), *level));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    end_to_end_hello_world,
    end_to_end_phases,
    end_to_end_scalability,
    end_to_end_by_type,
    end_to_end_opt_levels,
);
criterion_main!(benches);
