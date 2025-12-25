//! Lexer throughput benchmarks
//!
//! Measures tokens/second for various source file sizes and complexity levels.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tarqeem::Lexer;

/// Generate a simple source file with repeated statements
fn generate_simple_source(lines: usize) -> String {
    let mut source = String::from("بسم_الله\n");
    for i in 0..lines {
        source.push_str(&format!("متغير س{} = {}\n", i, i * 2));
    }
    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with complex expressions
fn generate_complex_source(lines: usize) -> String {
    let mut source = String::from("بسم_الله\n");
    for i in 0..lines {
        source.push_str(&format!(
            "متغير نتيجة_{} = ({} + {}) * ({} - {}) / 2\n",
            i,
            i,
            i + 1,
            i + 2,
            i
        ));
    }
    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with class definitions
fn generate_class_source(classes: usize) -> String {
    let mut source = String::from("بسم_الله\n");
    for i in 0..classes {
        source.push_str(&format!(
            r#"صنف صنف_{} {{
    خاص حقل١: عدد
    خاص حقل٢: نص

    منشئ(ق١: عدد، ق٢: نص) {{
        هذا.حقل١ = ق١
        هذا.حقل٢ = ق٢
    }}

    عام دالة احصل() -> عدد {{
        أرجع هذا.حقل١
    }}
}}

"#,
            i
        ));
    }
    source.push_str("الحمد_لله\n");
    source
}

fn lexer_simple_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer_simple");

    for size in [10, 100, 500, 1000].iter() {
        let source = generate_simple_source(*size);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &source, |b, source| {
            b.iter(|| {
                let mut lexer = Lexer::new(black_box(source));
                let tokens = lexer.tokenize();
                black_box(tokens)
            });
        });
    }

    group.finish();
}

fn lexer_complex_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer_complex");

    for size in [10, 100, 500, 1000].iter() {
        let source = generate_complex_source(*size);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &source, |b, source| {
            b.iter(|| {
                let mut lexer = Lexer::new(black_box(source));
                let tokens = lexer.tokenize();
                black_box(tokens)
            });
        });
    }

    group.finish();
}

fn lexer_class_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer_classes");

    for size in [5, 10, 25, 50].iter() {
        let source = generate_class_source(*size);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &source, |b, source| {
            b.iter(|| {
                let mut lexer = Lexer::new(black_box(source));
                let tokens = lexer.tokenize();
                black_box(tokens)
            });
        });
    }

    group.finish();
}

fn lexer_real_world(c: &mut Criterion) {
    // Use actual example files if they exist
    let example_source = r#"بسم_الله

// مثال على الأصناف
صنف شخص {
    خاص اسم: نص
    خاص عمر: عدد

    منشئ(اسم: نص، عمر: عدد) {
        هذا.اسم = اسم
        هذا.عمر = عمر
    }

    عام دالة اطبع_معلومات() {
        اطبع("الاسم: " + هذا.اسم)
        اطبع("العمر: " + هذا.عمر)
    }
}

صنف موظف يرث شخص {
    خاص راتب: عدد_عشري

    منشئ(اسم: نص، عمر: عدد، راتب: عدد_عشري) {
        الأصل(اسم، عمر)
        هذا.راتب = راتب
    }
}

متغير ش = جديد شخص("أحمد"، 30)
ش.اطبع_معلومات()

الحمد_لله
"#;

    let mut group = c.benchmark_group("lexer_real_world");
    group.throughput(Throughput::Bytes(example_source.len() as u64));

    group.bench_function("class_example", |b| {
        b.iter(|| {
            let mut lexer = Lexer::new(black_box(example_source));
            let tokens = lexer.tokenize();
            black_box(tokens)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    lexer_simple_throughput,
    lexer_complex_throughput,
    lexer_class_throughput,
    lexer_real_world,
);
criterion_main!(benches);
