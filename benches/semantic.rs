//! Semantic analyzer benchmarks
//!
//! Measures type checking operations/second for various program structures.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tarqeem::{Analyzer, Parser};

/// Generate source with typed variable declarations
fn generate_typed_variables(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");
    for i in 0..count {
        let type_suffix = match i % 4 {
            0 => "عدد",
            1 => "عدد_عشري",
            2 => "نص",
            _ => "منطقي",
        };
        let value = match i % 4 {
            0 => format!("{}", i),
            1 => format!("{}.5", i),
            2 => format!("\"قيمة{}\"", i),
            _ => if i % 2 == 0 { "صحيح" } else { "خطأ" }.to_string(),
        };
        source.push_str(&format!("متغير س_{}: {} = {}\n", i, type_suffix, value));
    }
    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with function calls requiring type checking
fn generate_function_calls(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    // Define some functions first
    source.push_str(
        r#"دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}

دالة ضرب(أ: عدد، ب: عدد) -> عدد {
    أرجع أ * ب
}

دالة معالجة(نص_المدخل: نص) -> نص {
    أرجع نص_المدخل
}

"#,
    );

    // Generate calls
    for i in 0..count {
        match i % 3 {
            0 => source.push_str(&format!("متغير ن{} = جمع({}, {})\n", i, i, i + 1)),
            1 => source.push_str(&format!("متغير ن{} = ضرب({}, {})\n", i, i, i + 2)),
            _ => source.push_str(&format!("متغير ن{} = معالجة(\"اختبار{}\")\n", i, i)),
        }
    }

    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with class hierarchies for type resolution
fn generate_class_hierarchy(depth: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    // Base interface
    source.push_str(
        r#"ميثاق قابل_للطباعة {
    دالة اطبع()
}

"#,
    );

    // Generate class hierarchy
    for i in 0..depth {
        if i == 0 {
            source.push_str(&format!(
                r#"صنف صنف_{} يلتزم قابل_للطباعة {{
    خاص قيمة: عدد

    منشئ(ق: عدد) {{
        هذا.قيمة = ق
    }}

    عام دالة اطبع() {{
        اطبع(هذا.قيمة)
    }}

    عام دالة احصل_قيمة() -> عدد {{
        أرجع هذا.قيمة
    }}
}}

"#,
                i
            ));
        } else {
            source.push_str(&format!(
                r#"صنف صنف_{} يرث صنف_{} {{
    خاص قيمة_اضافية: عدد

    منشئ(ق: عدد، ق_ا: عدد) {{
        الأصل(ق)
        هذا.قيمة_اضافية = ق_ا
    }}

    عام دالة احصل_قيمة_اضافية() -> عدد {{
        أرجع هذا.قيمة_اضافية
    }}
}}

"#,
                i,
                i - 1
            ));
        }
    }

    source.push_str("الحمد_لله\n");
    source
}

/// Generate source with expressions requiring type inference
fn generate_type_inference(count: usize) -> String {
    let mut source = String::from("بسم_الله\n");

    for i in 0..count {
        // Mixed expressions that require type inference
        source.push_str(&format!(
            "متغير س{} = {} + {} * {} - {}\n",
            i,
            i,
            i + 1,
            i + 2,
            i / 2
        ));
        source.push_str(&format!(
            "متغير ص{} = {} > {} && {} < {}\n",
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

fn semantic_typed_variables(c: &mut Criterion) {
    let mut group = c.benchmark_group("semantic_typed_vars");

    for count in [10, 50, 100, 500].iter() {
        let source = generate_typed_variables(*count);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), &source, |b, source| {
            // Pre-parse the AST
            let mut parser = Parser::new(source);
            let ast = parser.parse().expect("Parse should succeed");

            b.iter(|| {
                let mut analyzer = Analyzer::new();
                let result = analyzer.analyze(black_box(&ast));
                black_box(result)
            });
        });
    }

    group.finish();
}

fn semantic_function_calls(c: &mut Criterion) {
    let mut group = c.benchmark_group("semantic_function_calls");

    for count in [10, 50, 100, 200].iter() {
        let source = generate_function_calls(*count);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), &source, |b, source| {
            // Pre-parse the AST
            let mut parser = Parser::new(source);
            let ast = parser.parse().expect("Parse should succeed");

            b.iter(|| {
                let mut analyzer = Analyzer::new();
                let result = analyzer.analyze(black_box(&ast));
                black_box(result)
            });
        });
    }

    group.finish();
}

fn semantic_class_hierarchy(c: &mut Criterion) {
    let mut group = c.benchmark_group("semantic_class_hierarchy");

    for depth in [3, 5, 10, 15].iter() {
        let source = generate_class_hierarchy(*depth);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(depth), &source, |b, source| {
            // Pre-parse the AST
            let mut parser = Parser::new(source);
            let ast = parser.parse().expect("Parse should succeed");

            b.iter(|| {
                let mut analyzer = Analyzer::new();
                let result = analyzer.analyze(black_box(&ast));
                black_box(result)
            });
        });
    }

    group.finish();
}

fn semantic_type_inference(c: &mut Criterion) {
    let mut group = c.benchmark_group("semantic_type_inference");

    for count in [10, 50, 100, 200].iter() {
        let source = generate_type_inference(*count);
        group.throughput(Throughput::Bytes(source.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), &source, |b, source| {
            // Pre-parse the AST
            let mut parser = Parser::new(source);
            let ast = parser.parse().expect("Parse should succeed");

            b.iter(|| {
                let mut analyzer = Analyzer::new();
                let result = analyzer.analyze(black_box(&ast));
                black_box(result)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    semantic_typed_variables,
    semantic_function_calls,
    semantic_class_hierarchy,
    semantic_type_inference,
);
criterion_main!(benches);
