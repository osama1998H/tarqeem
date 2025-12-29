//! Compile command implementation.

use crate::codegen::{target::TargetTriple, Linker, LlvmCodegen, Target, WasmConfig};
use crate::error::Language;
use crate::ir::{IrBuilder, OptLevel, Optimizer};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::semantic::Analyzer;
use crate::utils::CompilerContext;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Timing information for each compilation stage (in microseconds)
#[derive(Default)]
pub struct CompilationTiming {
    pub lexer: Duration,
    pub parser: Duration,
    pub semantic: Duration,
    pub ir_build: Duration,
    pub optimize: Duration,
    pub codegen: Duration,
    pub total: Duration,
}

impl CompilationTiming {
    /// Output timing data as JSON (times in microseconds)
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"lexer":{},"parser":{},"semantic":{},"ir":{},"optimize":{},"codegen":{},"total":{}}}"#,
            self.lexer.as_micros(),
            self.parser.as_micros(),
            self.semantic.as_micros(),
            self.ir_build.as_micros(),
            self.optimize.as_micros(),
            self.codegen.as_micros(),
            self.total.as_micros()
        )
    }
}

use super::{configure_analyzer, find_runtime, find_wasm_runtime, warn_invalid_extension};

/// Arguments for the compile command
pub struct CompileArgs {
    pub file: PathBuf,
    pub output: Option<PathBuf>,
    pub opt_level: u8,
    pub emit_llvm: bool,
    pub emit_asm: bool,
    pub emit_obj: bool,
    pub emit_wasm: bool,
    pub wasm_js_bindings: bool,
    pub wasm_export_all: bool,
    pub wasm_memory_pages: u32,
    pub target: Option<String>,
    pub dump_tokens: bool,
    pub dump_ast: bool,
    pub dump_ir: bool,
    pub dump_opt_stats: bool,
    pub verbose: bool,
    /// Output compilation timing as JSON (for IDE integration)
    pub timing: bool,
}

pub fn compile(args: CompileArgs, lang: Language) -> Result<(), String> {
    warn_invalid_extension(&args.file);

    // Start total timing
    let total_start = Instant::now();
    let mut timing = CompilationTiming::default();

    let source = fs::read_to_string(&args.file)
        .map_err(|e| format!("Could not read file: {} / لا يمكن قراءة الملف: {}", e, e))?;

    let filename = args.file.display().to_string();

    // Create compiler context with string interner
    let mut ctx = CompilerContext::new();

    if args.dump_tokens {
        let lexer_start = Instant::now();
        let mut lexer = Lexer::with_interner(&source, ctx.interner_mut());
        println!("{}", "=== Tokens / الرموز ===".cyan().bold());
        for token in lexer.tokenize() {
            println!("  {:?} @ {}", token.kind, token.span);
        }
        timing.lexer = lexer_start.elapsed();
        timing.total = total_start.elapsed();
        if args.timing {
            println!("{}", timing.to_json());
        }
        return Ok(());
    }

    // Lexer + Parser timing (parser includes lexer internally)
    let parser_start = Instant::now();
    let mut parser = Parser::new(&source);
    let ast = parser.parse().map_err(|e| {
        e.emit(&source, &filename, lang);
        "Parse error / خطأ في التحليل".to_string()
    })?;
    timing.parser = parser_start.elapsed();

    if args.dump_ast {
        println!("{}", "=== AST / الشجرة النحوية ===".cyan().bold());
        println!("{:#?}", ast);
        timing.total = total_start.elapsed();
        if args.timing {
            println!("{}", timing.to_json());
        }
        return Ok(());
    }

    // Semantic analysis timing
    let semantic_start = Instant::now();
    let mut analyzer = Analyzer::new();
    configure_analyzer(&mut analyzer, args.verbose);
    if let Err(diagnostics) = analyzer.analyze(&ast) {
        for diag in &diagnostics {
            diag.emit(&source, &filename, lang);
        }
        return Err(format!(
            "{} error(s) found / وُجد {} خطأ/أخطاء",
            diagnostics.len(),
            diagnostics.len()
        ));
    }
    timing.semantic = semantic_start.elapsed();

    let module_name = args
        .file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module")
        .to_string();

    // IR generation timing
    let ir_start = Instant::now();
    let ir_builder = IrBuilder::new(module_name);
    let mut ir_module = ir_builder.build(&ast).map_err(|e| {
        format!(
            "IR generation error: {} / خطأ في توليد التمثيل الوسيط: {}",
            e.message, e.message_ar
        )
    })?;
    timing.ir_build = ir_start.elapsed();

    let opt = match args.opt_level {
        0 => OptLevel::O0,
        1 => OptLevel::O1,
        2 => OptLevel::O2,
        _ => OptLevel::O3,
    };

    // Optimization timing
    let opt_start = Instant::now();
    let mut optimizer = Optimizer::new(opt);
    optimizer.optimize(&mut ir_module);
    timing.optimize = opt_start.elapsed();

    if args.dump_opt_stats && optimizer.stats().any_changes() {
        println!(
            "{}",
            "=== Optimization Stats / إحصائيات التحسين ==="
                .cyan()
                .bold()
        );
        println!("{}", optimizer.stats());
    }

    if args.dump_ir {
        println!("{}", "=== IR / التمثيل الوسيط ===".cyan().bold());
        println!("{}", ir_module);
        timing.total = total_start.elapsed();
        if args.timing {
            println!("{}", timing.to_json());
        }
        return Ok(());
    }

    // Determine target: use explicit target, WASM if requested, or native
    let target_config = if let Some(ref triple_str) = args.target {
        TargetTriple::parse(triple_str)
            .map(Target::from_triple)
            .ok_or_else(|| {
                format!(
                    "Invalid target triple: {} / هدف غير صالح: {}",
                    triple_str, triple_str
                )
            })?
    } else if args.emit_wasm {
        // Default to wasm32-unknown-unknown for browser use
        Target::wasm32()
    } else {
        Target::native()
    };

    // Code generation timing
    let codegen_start = Instant::now();
    let mut codegen = LlvmCodegen::new(target_config.clone());
    let llvm_ir = codegen.generate(&ir_module).map_err(|e| {
        format!(
            "Code generation error: {} / خطأ في توليد الكود: {}",
            e.message, e.message_ar
        )
    })?;
    timing.codegen = codegen_start.elapsed();

    let output_path = args.output.unwrap_or_else(|| {
        let stem = args
            .file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        if args.emit_llvm {
            PathBuf::from(format!("{}.ll", stem))
        } else if args.emit_asm {
            PathBuf::from(format!("{}.s", stem))
        } else if args.emit_obj {
            PathBuf::from(format!("{}.o", stem))
        } else if args.emit_wasm || target_config.is_wasm() {
            PathBuf::from(format!("{}.wasm", stem))
        } else {
            PathBuf::from(stem)
        }
    });

    if args.emit_llvm {
        fs::write(&output_path, &llvm_ir)
            .map_err(|e| format!("Could not write output: {} / لا يمكن كتابة الملف: {}", e, e))?;
        println!(
            "{}",
            format!(
                "LLVM IR written to: {} / تم كتابة التمثيل الوسيط إلى: {}",
                output_path.display(),
                output_path.display()
            )
            .green()
        );
    } else if args.emit_asm || args.emit_obj {
        let linker = Linker::new(target_config)
            .optimization_level(args.opt_level as u32)
            .verbose(args.verbose);

        if !linker.is_available() {
            return Err("No compiler (clang/llc) found. Install LLVM or use --emit-llvm / لم يتم العثور على مترجم. ثبّت مترجم clang أو استخدم --emit-llvm".to_string());
        }

        if args.emit_asm {
            linker
                .compile_to_assembly(&llvm_ir, &output_path)
                .map_err(|e| {
                    format!(
                        "Assembly generation failed: {} / فشل توليد التجميع: {}",
                        e.message, e.message_ar
                    )
                })?;
            println!(
                "{}",
                format!(
                    "Assembly written to: {} / تم كتابة التجميع إلى: {}",
                    output_path.display(),
                    output_path.display()
                )
                .green()
            );
        } else {
            linker
                .compile_to_object(&llvm_ir, &output_path)
                .map_err(|e| {
                    format!(
                        "Object compilation failed: {} / فشل ترجمة الكائن: {}",
                        e.message, e.message_ar
                    )
                })?;
            println!(
                "{}",
                format!(
                    "Object file written to: {} / تم كتابة ملف الكائن إلى: {}",
                    output_path.display(),
                    output_path.display()
                )
                .green()
            );
        }
    } else if args.emit_wasm || target_config.is_wasm() {
        // WebAssembly compilation
        let wasm_config = WasmConfig::new().with_memory_pages(args.wasm_memory_pages);
        let wasm_config = if args.wasm_js_bindings {
            wasm_config.with_js_bindings()
        } else {
            wasm_config
        };
        let wasm_config = if args.wasm_export_all {
            wasm_config.with_export_all()
        } else {
            wasm_config
        };

        let linker = Linker::new(target_config.clone())
            .optimization_level(args.opt_level as u32)
            .verbose(args.verbose)
            .wasm_config(wasm_config);

        if !linker.is_wasm_available() {
            return Err("WASM compilation not available. Install LLVM with WebAssembly support. / ترجمة WebAssembly غير متاحة. ثبّت LLVM مع دعم WebAssembly.".to_string());
        }

        // Find WASM runtime if available
        let wasm_runtime = find_wasm_runtime();

        linker
            .compile_to_wasm(&llvm_ir, &output_path, wasm_runtime.as_deref())
            .map_err(|e| {
                format!(
                    "WASM compilation failed: {} / فشل ترجمة WebAssembly: {}",
                    e.message, e.message_ar
                )
            })?;

        println!(
            "{}",
            format!(
                "WebAssembly created: {} / تم إنشاء WebAssembly: {}",
                output_path.display(),
                output_path.display()
            )
            .green()
        );

        if args.wasm_js_bindings {
            let js_path = output_path.with_extension("js");
            println!(
                "{}",
                format!(
                    "JavaScript bindings: {} / روابط JavaScript: {}",
                    js_path.display(),
                    js_path.display()
                )
                .green()
            );
        }

        if args.verbose {
            println!(
                "  Target: {} / الهدف: {}",
                target_config.llvm_triple(),
                target_config.llvm_triple()
            );
            println!(
                "  Memory: {} pages ({} KB) / الذاكرة: {} صفحة ({} كيلوبايت)",
                args.wasm_memory_pages,
                args.wasm_memory_pages * 64,
                args.wasm_memory_pages,
                args.wasm_memory_pages * 64
            );
        }
    } else {
        let linker = Linker::new(target_config)
            .optimization_level(args.opt_level as u32)
            .verbose(args.verbose);

        if linker.is_available() {
            let runtime_path = find_runtime();
            if runtime_path.is_none() && args.verbose {
                eprintln!("{}", "Warning: Runtime library not found. Executable may not link correctly. / تحذير: لم يتم العثور على مكتبة التشغيل.".yellow());
            }

            linker
                .compile_to_executable(&llvm_ir, &output_path, runtime_path.as_deref())
                .map_err(|e| {
                    format!(
                        "Linking failed: {} / فشل الربط: {}",
                        e.message, e.message_ar
                    )
                })?;
            println!(
                "{}",
                format!(
                    "Executable created: {} / تم إنشاء الملف التنفيذي: {}",
                    output_path.display(),
                    output_path.display()
                )
                .green()
            );
        } else {
            let ll_path = output_path.with_extension("ll");
            fs::write(&ll_path, &llvm_ir).map_err(|e| {
                format!("Could not write output: {} / لا يمكن كتابة الملف: {}", e, e)
            })?;
            println!(
                "{}",
                "Note: No compiler found. LLVM IR written instead. / ملاحظة: لم يتم العثور على مترجم. تم كتابة التمثيل الوسيط بدلاً من ذلك.".yellow()
            );
            println!(
                "  You can compile with: clang {} -o {} / يمكنك الترجمة بـ: clang {} -o {}",
                ll_path.display(),
                output_path.display(),
                ll_path.display(),
                output_path.display()
            );
        }
    }

    // Calculate total time
    timing.total = total_start.elapsed();

    // Output timing JSON if requested
    if args.timing {
        println!("{}", timing.to_json());
    }

    if args.verbose {
        println!(
            "{}",
            "Compilation successful! / تمت الترجمة بنجاح!"
                .green()
                .bold()
        );
        println!(
            "  Functions: {} / الدوال: {}",
            ir_module.functions.len(),
            ir_module.functions.len()
        );
        println!(
            "  Classes: {} / الأصناف: {}",
            ir_module.classes.len(),
            ir_module.classes.len()
        );
        if opt != OptLevel::O0 {
            println!("  Optimization level: {} / مستوى التحسين: {}", opt, opt);
        }
    }

    Ok(())
}
