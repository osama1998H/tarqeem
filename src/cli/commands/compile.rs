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

use super::{analyzer_file_path, configure_analyzer, warn_invalid_extension};

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
        println!("{}", "=== الرموز ===".cyan().bold());
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
        "خطأ في التحليل".to_string()
    })?;
    timing.parser = parser_start.elapsed();

    if args.dump_ast {
        println!("{}", "=== الشجرة النحوية ===".cyan().bold());
        println!("{:#?}", ast);
        timing.total = total_start.elapsed();
        if args.timing {
            println!("{}", timing.to_json());
        }
        return Ok(());
    }

    // Semantic analysis timing
    let semantic_start = Instant::now();
    let mut analyzer = Analyzer::for_file(analyzer_file_path(&args.file));
    configure_analyzer(&mut analyzer, args.verbose);
    if let Err(diagnostics) = analyzer.analyze(&ast) {
        for diag in &diagnostics {
            diag.emit(&source, &filename, lang);
        }
        return Err(format!("وُجد {} خطأ/أخطاء", diagnostics.len(),));
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
    // Imported module bodies only reach codegen through this merge; the IR
    // builder itself accepts a single Ast and drops `استورد`.
    let mut link_warnings = Vec::new();
    let linked = analyzer
        .linked_ast(&ast, &mut link_warnings)
        .map_err(|diagnostics| {
            for diag in &diagnostics {
                diag.emit(&source, &filename, lang);
            }
            format!("وُجد {} خطأ/أخطاء", diagnostics.len())
        })?;
    for diag in &link_warnings {
        diag.emit(&source, &filename, lang);
    }

    let ir_builder = IrBuilder::new(module_name).with_visible_names(analyzer.visible_names());
    let mut ir_module = ir_builder
        .build(&linked)
        .map_err(|e| format!("خطأ في توليد التمثيل الوسيط: {}", e.message))?;
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
        println!("{}", "=== إحصائيات التحسين ===".cyan().bold());
        println!("{}", optimizer.stats());
    }

    if args.dump_ir {
        println!("{}", "=== التمثيل الوسيط ===".cyan().bold());
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
            .ok_or_else(|| format!("هدف غير صالح: {}", triple_str))?
    } else if args.emit_wasm {
        // Default to wasm32-unknown-unknown for browser use
        Target::wasm32()
    } else {
        Target::native()
    };

    // Code generation timing
    let codegen_start = Instant::now();
    let mut codegen = LlvmCodegen::new(target_config.clone());
    // `CodegenError` has carried a code since ت٠٣٠١ but the CLI dropped it, so
    // no codegen code was ever printed — and one the user cannot see is one they
    // cannot pass to `tarqeem اشرح`.
    let llvm_ir = codegen.generate(&ir_module).map_err(|e| match &e.code {
        Some(code) => format!("خطأ في توليد الكود [{}]: {}", code, e.message),
        None => format!("خطأ في توليد الكود: {}", e.message),
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
        fs::write(&output_path, &llvm_ir).map_err(|e| format!("لا يمكن كتابة الملف: {}", e))?;
        println!(
            "{}",
            format!("تم كتابة التمثيل الوسيط إلى: {}", output_path.display(),).green()
        );
    } else if args.emit_asm || args.emit_obj {
        let linker = Linker::new(target_config)
            .optimization_level(args.opt_level as u32)
            .verbose(args.verbose);

        if !linker.is_available() {
            return Err(
                "لم يتم العثور على مترجم. ثبّت مترجم clang أو استخدم --emit-llvm".to_string(),
            );
        }

        if args.emit_asm {
            linker
                .compile_to_assembly(&llvm_ir, &output_path)
                .map_err(|e| format!("فشل توليد التجميع: {}", e.message))?;
            println!(
                "{}",
                format!("تم كتابة التجميع إلى: {}", output_path.display(),).green()
            );
        } else {
            linker
                .compile_to_object(&llvm_ir, &output_path)
                .map_err(|e| format!("فشل ترجمة الكائن: {}", e.message))?;
            println!(
                "{}",
                format!("تم كتابة ملف الكائن إلى: {}", output_path.display(),).green()
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
            return Err("ترجمة WebAssembly غير متاحة. ثبّت LLVM مع دعم WebAssembly.".to_string());
        }

        linker
            .compile_to_wasm(&llvm_ir, &output_path, None)
            .map_err(|e| format!(" ترجمة WebAssembly: {}", e))?;

        println!(
            "{}",
            format!("تم إنشاء WebAssembly: {}", output_path.display(),).green()
        );

        if args.wasm_js_bindings {
            let js_path = output_path.with_extension("js");
            println!(
                "{}",
                format!("روابط JavaScript: {}", js_path.display(),).green()
            );
        }

        if args.verbose {
            println!("الهدف: {}", target_config.llvm_triple(),);
            println!(
                "الذاكرة: {} صفحة ({} كيلوبايت)",
                args.wasm_memory_pages,
                args.wasm_memory_pages * 64
            );
        }
    } else {
        let linker = Linker::new(target_config)
            .optimization_level(args.opt_level as u32)
            .verbose(args.verbose);

        if linker.is_available() {
            // Discovery, the verbose report, and the ت٠١٠٢ refusal all live in
            // the linker, so there is one search rather than two disagreeing ones.
            linker
                .compile_to_executable(&llvm_ir, &output_path, None)
                .map_err(|e| e.to_string())?;
            println!(
                "{}",
                format!(" إنشاء الملف التنفيذي: {}", output_path.display(),).green()
            );
        } else {
            let ll_path = output_path.with_extension("ll");
            fs::write(&ll_path, &llvm_ir).map_err(|e| format!("لا يمكن كتابة الملف: {}", e))?;
            println!(
                "{}",
                "ملاحظة: لم يتم العثور على مترجم. تم كتابة التمثيل الوسيط بدلاً من ذلك.".yellow()
            );
            println!(
                "يمكنك الترجمة بـ: clang {} -o {}",
                ll_path.display(),
                output_path.display(),
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
        println!("{}", "تمت الترجمة بنجاح!".green().bold());
        println!("  الدوال: {}", ir_module.functions.len(),);
        println!("  الأصناف: {}", ir_module.classes.len(),);
        if opt != OptLevel::O0 {
            println!("  مستوى التحسين: {}", opt);
        }
    }

    Ok(())
}
