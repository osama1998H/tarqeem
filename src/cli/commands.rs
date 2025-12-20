//! CLI command implementations

use super::{Cli, Commands, PkgCommands};
use crate::codegen::{target::TargetTriple, Linker, LlvmCodegen, Target};
use crate::doc::generator::DocGenerator;
use crate::doc::{DocExtractor, HtmlGenerator, JsonGenerator, MarkdownGenerator, OutputFormat};
use crate::error::Language;
use crate::interpreter::Interpreter;
use crate::ir::{IrBuilder, OptLevel, Optimizer};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::semantic::Analyzer;
use crate::utils::{is_valid_source_extension, valid_source_extensions_display};
use colored::Colorize;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

/// Warn if file doesn't have a valid Tarqeem extension
fn warn_invalid_extension(file: &Path) {
    if !is_valid_source_extension(file) {
        eprintln!(
            "{}",
            format!(
                "تحذير: الملف لا يحمل امتداد ترقيم صالح ({})",
                valid_source_extensions_display()
            )
            .yellow()
        );
        eprintln!(
            "{}",
            format!(
                "Warning: File doesn't have a valid Tarqeem extension ({})",
                valid_source_extensions_display()
            )
            .yellow()
        );
    }
}

/// Find the Tarqeem runtime library (libtrq.a)
fn find_runtime() -> Option<PathBuf> {
    // Try several locations
    let search_paths = [
        // Relative to executable
        std::env::current_exe()
            .ok()?
            .parent()?
            .join("runtime/libtrq.a"),
        std::env::current_exe()
            .ok()?
            .parent()?
            .join("../runtime/libtrq.a"),
        // Relative to current directory
        PathBuf::from("runtime/libtrq.a"),
        // Standard install locations
        PathBuf::from("/usr/local/lib/tarqeem/libtrq.a"),
        PathBuf::from("/usr/lib/tarqeem/libtrq.a"),
    ];

    for path in &search_paths {
        if path.exists() {
            return Some(path.clone());
        }
    }

    None
}

/// Find the Tarqeem standard library directory (stdlib_trq/)
fn find_stdlib_path() -> Option<PathBuf> {
    // Try several locations in priority order
    let search_paths: Vec<Option<PathBuf>> = vec![
        // Relative to executable (installed)
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("stdlib_trq"))),
        // Relative to executable parent (development build)
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .and_then(|p| p.parent().map(|p| p.join("stdlib_trq")))
        }),
        // Relative to current directory (development)
        Some(PathBuf::from("stdlib_trq")),
        // Standard install locations
        Some(PathBuf::from("/usr/local/lib/tarqeem/stdlib_trq")),
        Some(PathBuf::from("/usr/lib/tarqeem/stdlib_trq")),
    ];

    for path in search_paths.into_iter().flatten() {
        if path.exists() && path.is_dir() {
            return Some(path);
        }
    }

    None
}

/// Configure the analyzer with stdlib search path
fn configure_analyzer(analyzer: &mut Analyzer, verbose: bool) {
    if let Some(stdlib_path) = find_stdlib_path() {
        if verbose {
            eprintln!(
                "{}",
                format!(
                    "Using stdlib: {} / المكتبة القياسية: {}",
                    stdlib_path.display(),
                    stdlib_path.display()
                )
                .dimmed()
            );
        }
        analyzer.add_search_path(stdlib_path);
    }
}

/// Run the CLI
pub fn run(cli: Cli) -> Result<(), String> {
    let lang = if cli.english {
        Language::English
    } else {
        Language::Arabic
    };

    match cli.command {
        Commands::Compile {
            file,
            output,
            opt_level,
            emit_llvm,
            emit_asm,
            emit_obj,
            target,
            dump_tokens,
            dump_ast,
            dump_ir,
            dump_opt_stats,
        } => {
            // Warn if file extension is not recognized
            warn_invalid_extension(&file);

            let source = fs::read_to_string(&file)
                .map_err(|e| format!("Could not read file: {} / لا يمكن قراءة الملف: {}", e, e))?;

            let filename = file.display().to_string();

            // Lexing
            if dump_tokens {
                let mut lexer = Lexer::new(&source);
                println!("{}", "=== Tokens / الرموز ===".cyan().bold());
                for token in lexer.tokenize() {
                    println!("  {:?} @ {}", token.kind, token.span);
                }
                return Ok(());
            }

            // Parsing
            let mut parser = Parser::new(&source);
            let ast = parser.parse().map_err(|e| {
                e.emit(&source, &filename, lang);
                format!("Parse error / خطأ في التحليل")
            })?;

            if dump_ast {
                println!("{}", "=== AST / الشجرة النحوية ===".cyan().bold());
                println!("{:#?}", ast);
                return Ok(());
            }

            // Semantic analysis
            let mut analyzer = Analyzer::new();
            configure_analyzer(&mut analyzer, cli.verbose);
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

            // IR generation
            let module_name = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module")
                .to_string();

            let ir_builder = IrBuilder::new(module_name);
            let mut ir_module = ir_builder.build(&ast).map_err(|e| {
                format!(
                    "IR generation error: {} / خطأ في توليد الـ IR: {}",
                    e.message, e.message_ar
                )
            })?;

            // Run optimization passes
            let opt = match opt_level {
                0 => OptLevel::O0,
                1 => OptLevel::O1,
                2 => OptLevel::O2,
                _ => OptLevel::O3,
            };

            let mut optimizer = Optimizer::new(opt);
            optimizer.optimize(&mut ir_module);

            if dump_opt_stats && optimizer.stats().any_changes() {
                println!(
                    "{}",
                    "=== Optimization Stats / إحصائيات التحسين ==="
                        .cyan()
                        .bold()
                );
                println!("{}", optimizer.stats());
            }

            if dump_ir {
                println!("{}", "=== IR / التمثيل الوسيط ===".cyan().bold());
                println!("{}", ir_module);
                return Ok(());
            }

            // LLVM Code Generation
            let target_config = if let Some(ref triple_str) = target {
                TargetTriple::parse(triple_str)
                    .map(Target::from_triple)
                    .ok_or_else(|| {
                        format!(
                            "Invalid target triple: {} / هدف غير صالح: {}",
                            triple_str, triple_str
                        )
                    })?
            } else {
                Target::native()
            };

            let mut codegen = LlvmCodegen::new(target_config.clone());
            let llvm_ir = codegen.generate(&ir_module).map_err(|e| {
                format!(
                    "Code generation error: {} / خطأ في توليد الكود: {}",
                    e.message, e.message_ar
                )
            })?;

            // Determine output path
            let output_path = output.unwrap_or_else(|| {
                let stem = file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output");
                if emit_llvm {
                    PathBuf::from(format!("{}.ll", stem))
                } else if emit_asm {
                    PathBuf::from(format!("{}.s", stem))
                } else if emit_obj {
                    PathBuf::from(format!("{}.o", stem))
                } else {
                    PathBuf::from(stem)
                }
            });

            // Handle different emit modes
            if emit_llvm {
                // Write LLVM IR directly
                fs::write(&output_path, &llvm_ir).map_err(|e| {
                    format!("Could not write output: {} / لا يمكن كتابة الملف: {}", e, e)
                })?;
                println!(
                    "{}",
                    format!(
                        "LLVM IR written to: {} / تم كتابة LLVM IR إلى: {}",
                        output_path.display(),
                        output_path.display()
                    )
                    .green()
                );
            } else if emit_asm || emit_obj {
                // Use linker to compile
                let linker = Linker::new(target_config)
                    .optimization_level(opt_level as u32)
                    .verbose(cli.verbose);

                if !linker.is_available() {
                    return Err("No compiler (clang/llc) found. Install LLVM or use --emit-llvm / لم يتم العثور على مترجم. ثبّت LLVM أو استخدم --emit-llvm".to_string());
                }

                if emit_asm {
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
            } else {
                // Compile to executable
                let linker = Linker::new(target_config)
                    .optimization_level(opt_level as u32)
                    .verbose(cli.verbose);

                if linker.is_available() {
                    // Find runtime library
                    let runtime_path = find_runtime();
                    if runtime_path.is_none() && cli.verbose {
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
                    // Fallback: just write LLVM IR
                    let ll_path = output_path.with_extension("ll");
                    fs::write(&ll_path, &llvm_ir).map_err(|e| {
                        format!("Could not write output: {} / لا يمكن كتابة الملف: {}", e, e)
                    })?;
                    println!(
                        "{}",
                        "Note: No compiler found. LLVM IR written instead. / ملاحظة: لم يتم العثور على مترجم. تم كتابة LLVM IR بدلاً من ذلك.".yellow()
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

            if cli.verbose {
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

        Commands::Run { file } => {
            // Warn if file extension is not recognized
            warn_invalid_extension(&file);

            let source = fs::read_to_string(&file)
                .map_err(|e| format!("Could not read file: {} / لا يمكن قراءة الملف: {}", e, e))?;

            let filename = file.display().to_string();

            // Parse
            let mut parser = Parser::new(&source);
            let ast = parser.parse().map_err(|e| {
                e.emit(&source, &filename, lang);
                format!("Parse error / خطأ في التحليل")
            })?;

            // Semantic analysis
            let mut analyzer = Analyzer::new();
            configure_analyzer(&mut analyzer, cli.verbose);
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

            // Build IR
            let ir_builder = IrBuilder::new(filename.clone());
            let ir_module = ir_builder
                .build(&ast)
                .map_err(|e| format!("IR build error: {} / خطأ بناء التمثيل الوسيط: {}", e, e))?;

            // Run interpreter
            let mut interpreter = Interpreter::new(ir_module);
            match interpreter.run() {
                Ok(_result) => {
                    // Program executed successfully
                    if cli.verbose {
                        println!(
                            "{}",
                            "Program completed successfully / اكتمل تنفيذ البرنامج بنجاح".green()
                        );
                    }
                }
                Err(e) => {
                    eprintln!("{} {}", "Runtime error / خطأ وقت التشغيل:".red().bold(), e);
                    return Err(format!("Runtime error / خطأ وقت التشغيل"));
                }
            }

            Ok(())
        }

        Commands::Check { file } => {
            // Warn if file extension is not recognized
            warn_invalid_extension(&file);

            let source = fs::read_to_string(&file)
                .map_err(|e| format!("Could not read file: {} / لا يمكن قراءة الملف: {}", e, e))?;

            let filename = file.display().to_string();

            // Parse
            let mut parser = Parser::new(&source);
            let ast = parser.parse().map_err(|e| {
                e.emit(&source, &filename, lang);
                format!("Parse error / خطأ في التحليل")
            })?;

            // Analyze
            let mut analyzer = Analyzer::new();
            configure_analyzer(&mut analyzer, cli.verbose);
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

            println!(
                "{}",
                "No errors found! / لم يتم العثور على أخطاء!".green().bold()
            );

            Ok(())
        }

        Commands::Repl => {
            println!(
                "{}",
                "=== Tarqeem REPL / الوضع التفاعلي لترقيم ===".cyan().bold()
            );
            println!("Type 'exit' or 'خروج' to quit / اكتب 'exit' أو 'خروج' للخروج");
            println!();

            let stdin = io::stdin();
            let mut stdout = io::stdout();
            let mut line_count = 0u32;

            loop {
                print!("{}", "ترقيم> ".green().bold());
                stdout.flush().unwrap();

                let mut line = String::new();
                match stdin.lock().read_line(&mut line) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed == "exit" || trimmed == "خروج" {
                            println!("Goodbye! / مع السلامة!");
                            break;
                        }

                        if trimmed.is_empty() {
                            continue;
                        }

                        line_count += 1;

                        // Parse and analyze the line
                        let mut parser = Parser::new(trimmed);
                        match parser.parse() {
                            Ok(ast) => {
                                let mut analyzer = Analyzer::new();
                                configure_analyzer(&mut analyzer, cli.verbose);
                                if let Err(diagnostics) = analyzer.analyze(&ast) {
                                    for diag in &diagnostics {
                                        diag.emit(trimmed, "<repl>", lang);
                                    }
                                } else {
                                    // Build IR and execute
                                    let module_name = format!("<repl:{}>", line_count);
                                    let ir_builder = IrBuilder::new(module_name);
                                    match ir_builder.build(&ast) {
                                        Ok(ir_module) => {
                                            let mut interpreter = Interpreter::new(ir_module);
                                            match interpreter.run() {
                                                Ok(result) => {
                                                    // Print result if not null and verbose
                                                    if cli.verbose {
                                                        println!(
                                                            "{} {}",
                                                            "=>".cyan(),
                                                            format!("{}", result).yellow()
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!(
                                                        "{} {}",
                                                        "Runtime error:".red().bold(),
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("{} {}", "IR error:".red().bold(), e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                e.emit(trimmed, "<repl>", lang);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading input: {}", e);
                        break;
                    }
                }
            }

            Ok(())
        }

        Commands::Fmt { file, write } => {
            // Warn if file extension is not recognized
            warn_invalid_extension(&file);

            let source = fs::read_to_string(&file)
                .map_err(|e| format!("Could not read file: {} / لا يمكن قراءة الملف: {}", e, e))?;

            // TODO: Implement formatter
            // For now, just output the source as-is
            if write {
                println!(
                    "{}",
                    "Note: Formatter not yet implemented / ملاحظة: المُنسق غير مُنفذ بعد"
                        .yellow()
                        .bold()
                );
            } else {
                println!("{}", source);
            }

            Ok(())
        }

        Commands::Lex { file } => {
            // Warn if file extension is not recognized
            warn_invalid_extension(&file);

            let source = fs::read_to_string(&file)
                .map_err(|e| format!("Could not read file: {} / لا يمكن قراءة الملف: {}", e, e))?;

            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();

            println!("{}", "=== Tokens / الرموز ===".cyan().bold());
            for token in tokens {
                println!(
                    "  [{:>4}:{:<3}] {:?} '{}'",
                    token.span.line, token.span.column, token.kind, token.lexeme
                );
            }

            Ok(())
        }

        Commands::Parse { file } => {
            // Warn if file extension is not recognized
            warn_invalid_extension(&file);

            let source = fs::read_to_string(&file)
                .map_err(|e| format!("Could not read file: {} / لا يمكن قراءة الملف: {}", e, e))?;

            let filename = file.display().to_string();

            let mut parser = Parser::new(&source);
            match parser.parse() {
                Ok(ast) => {
                    println!("{}", "=== AST / الشجرة النحوية ===".cyan().bold());
                    println!("{:#?}", ast);
                }
                Err(e) => {
                    e.emit(&source, &filename, lang);
                    return Err("Parse error / خطأ في التحليل".to_string());
                }
            }

            Ok(())
        }

        Commands::Pkg { command } => {
            use super::pm;

            let result = match command {
                PkgCommands::Init { name, lib } => pm::init(name, lib),
                PkgCommands::Add { package, dev, path } => pm::add(package, dev, path),
                PkgCommands::Remove { package } => pm::remove(package),
                PkgCommands::Install { force } => pm::install(force),
                PkgCommands::Update { package } => pm::update(package),
                PkgCommands::Build { release } => pm::build(release),
                PkgCommands::Run { args } => pm::pkg_run(args),
                PkgCommands::Test { filter } => pm::test(filter),
                PkgCommands::Info => pm::info(),
                PkgCommands::Clean => pm::clean(),
            };

            result.map_err(|e| format!("{}", e))
        }

        Commands::Lsp => {
            // Start the LSP server
            if cli.verbose {
                eprintln!(
                    "{}",
                    "Starting Tarqeem Language Server... / جاري بدء خادم لغة ترقيم..."
                        .cyan()
                        .bold()
                );
            }

            // Create and run the async runtime
            let runtime = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Failed to create runtime: {} / فشل إنشاء وقت التشغيل: {}", e, e))?;

            runtime.block_on(async {
                crate::lsp::run_server().await.map_err(|e| {
                    format!(
                        "LSP server error: {} / خطأ خادم LSP: {}",
                        e, e
                    )
                })
            })
        }

        Commands::Doc {
            path,
            output,
            format,
            single_file,
        } => {
            // Determine output format
            let output_format = match format.to_lowercase().as_str() {
                "html" => OutputFormat::Html,
                "markdown" | "md" => OutputFormat::Markdown,
                "json" => OutputFormat::Json,
                _ => {
                    return Err(format!(
                        "Unknown format: {}. Use html, markdown, or json / صيغة غير معروفة: {}. استخدم html أو markdown أو json",
                        format, format
                    ));
                }
            };

            // Collect source files
            let source_files: Vec<PathBuf> = if path.is_dir() {
                // Find all .trq and .ترقيم files in the directory
                collect_source_files(&path)?
            } else {
                warn_invalid_extension(&path);
                vec![path.clone()]
            };

            if source_files.is_empty() {
                return Err(
                    "No source files found / لم يتم العثور على ملفات مصدر".to_string()
                );
            }

            // Determine output directory
            let output_dir = output.unwrap_or_else(|| {
                if path.is_dir() {
                    path.join("docs")
                } else {
                    path.parent()
                        .map(|p| p.join("docs"))
                        .unwrap_or_else(|| PathBuf::from("docs"))
                }
            });

            // Create output directory if it doesn't exist
            if !single_file {
                fs::create_dir_all(&output_dir).map_err(|e| {
                    format!(
                        "Could not create output directory: {} / لا يمكن إنشاء مجلد الإخراج: {}",
                        e, e
                    )
                })?;
            }

            if cli.verbose {
                println!(
                    "{}",
                    format!(
                        "Generating documentation for {} file(s)... / جاري توليد التوثيق لـ {} ملف(ات)...",
                        source_files.len(),
                        source_files.len()
                    )
                    .cyan()
                );
            }

            // Process each source file
            let mut all_docs = Vec::new();
            for source_file in &source_files {
                let source = fs::read_to_string(source_file).map_err(|e| {
                    format!(
                        "Could not read file {}: {} / لا يمكن قراءة الملف {}: {}",
                        source_file.display(),
                        e,
                        source_file.display(),
                        e
                    )
                })?;

                let filename = source_file.display().to_string();
                let module_name = source_file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("module")
                    .to_string();

                // Parse the file
                let mut parser = Parser::new(&source);
                let ast = parser.parse().map_err(|e| {
                    e.emit(&source, &filename, lang);
                    format!(
                        "Parse error in {}: / خطأ في تحليل {}:",
                        source_file.display(),
                        source_file.display()
                    )
                })?;

                // Extract documentation
                let extractor = DocExtractor::new(module_name.clone(), filename);
                let doc = extractor.extract(&ast);

                if cli.verbose {
                    let item_count = doc.items.len();
                    println!(
                        "  {} - {} items / {} عنصر",
                        module_name,
                        item_count,
                        item_count
                    );
                }

                all_docs.push((module_name, doc));
            }

            // Generate output
            if single_file {
                // Generate a single combined documentation file
                let output_file = if output_dir.is_dir() || !output_dir.exists() {
                    let ext = match output_format {
                        OutputFormat::Html => "html",
                        OutputFormat::Markdown => "md",
                        OutputFormat::Json => "json",
                    };
                    output_dir.join(format!("documentation.{}", ext))
                } else {
                    output_dir.clone()
                };

                // For now, generate first doc (TODO: combine all docs)
                if let Some((_name, doc)) = all_docs.first() {
                    let mut output_buffer = Vec::new();

                    match output_format {
                        OutputFormat::Html => {
                            let generator = HtmlGenerator::new();
                            generator.generate(doc, &mut output_buffer)
                        }
                        OutputFormat::Markdown => {
                            let generator = MarkdownGenerator::new();
                            generator.generate(doc, &mut output_buffer)
                        }
                        OutputFormat::Json => {
                            let generator = JsonGenerator::new();
                            generator.generate(doc, &mut output_buffer)
                        }
                    }
                    .map_err(|e| {
                        format!(
                            "Failed to generate documentation: {} / فشل توليد التوثيق: {}",
                            e, e
                        )
                    })?;

                    // Create parent directory if needed
                    if let Some(parent) = output_file.parent() {
                        fs::create_dir_all(parent).map_err(|e| {
                            format!(
                                "Could not create output directory: {} / لا يمكن إنشاء مجلد الإخراج: {}",
                                e, e
                            )
                        })?;
                    }

                    fs::write(&output_file, output_buffer).map_err(|e| {
                        format!(
                            "Could not write output: {} / لا يمكن كتابة الإخراج: {}",
                            e, e
                        )
                    })?;

                    println!(
                        "{}",
                        format!(
                            "Documentation generated: {} / تم توليد التوثيق: {}",
                            output_file.display(),
                            output_file.display()
                        )
                        .green()
                    );
                }
            } else {
                // Generate separate documentation files
                for (module_name, doc) in &all_docs {
                    let ext = match output_format {
                        OutputFormat::Html => "html",
                        OutputFormat::Markdown => "md",
                        OutputFormat::Json => "json",
                    };
                    let output_file = output_dir.join(format!("{}.{}", module_name, ext));

                    let mut output_buffer = Vec::new();

                    match output_format {
                        OutputFormat::Html => {
                            let generator = HtmlGenerator::new();
                            generator.generate(doc, &mut output_buffer)
                        }
                        OutputFormat::Markdown => {
                            let generator = MarkdownGenerator::new();
                            generator.generate(doc, &mut output_buffer)
                        }
                        OutputFormat::Json => {
                            let generator = JsonGenerator::new();
                            generator.generate(doc, &mut output_buffer)
                        }
                    }
                    .map_err(|e| {
                        format!(
                            "Failed to generate documentation: {} / فشل توليد التوثيق: {}",
                            e, e
                        )
                    })?;

                    fs::write(&output_file, output_buffer).map_err(|e| {
                        format!(
                            "Could not write output: {} / لا يمكن كتابة الإخراج: {}",
                            e, e
                        )
                    })?;
                }

                // Generate index file for HTML
                if output_format == OutputFormat::Html && all_docs.len() > 1 {
                    let index_content = generate_html_index(&all_docs);
                    let index_file = output_dir.join("index.html");
                    fs::write(&index_file, index_content).map_err(|e| {
                        format!(
                            "Could not write index: {} / لا يمكن كتابة الفهرس: {}",
                            e, e
                        )
                    })?;
                }

                println!(
                    "{}",
                    format!(
                        "Documentation generated in: {} / تم توليد التوثيق في: {}",
                        output_dir.display(),
                        output_dir.display()
                    )
                    .green()
                );
            }

            if cli.verbose {
                println!(
                    "{}",
                    "Documentation generation complete! / اكتمل توليد التوثيق!"
                        .green()
                        .bold()
                );
            }

            Ok(())
        }
    }
}

/// Collect all Tarqeem source files from a directory
fn collect_source_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    let entries = fs::read_dir(dir).map_err(|e| {
        format!(
            "Could not read directory: {} / لا يمكن قراءة المجلد: {}",
            e, e
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            format!(
                "Could not read entry: {} / لا يمكن قراءة المدخل: {}",
                e, e
            )
        })?;
        let path = entry.path();

        if path.is_file() && is_valid_source_extension(&path) {
            files.push(path);
        } else if path.is_dir() {
            // Recursively collect from subdirectories
            files.extend(collect_source_files(&path)?);
        }
    }

    // Sort files for consistent output
    files.sort();

    Ok(files)
}

/// Generate an HTML index page for multiple documentation files
fn generate_html_index(docs: &[(String, crate::doc::model::Documentation)]) -> String {
    let mut html = String::from(
        r#"<!DOCTYPE html>
<html lang="ar" dir="rtl">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>توثيق ترقيم | Tarqeem Documentation</title>
    <style>
        :root {
            --primary-color: #2563eb;
            --secondary-color: #1e40af;
            --background-color: #f8fafc;
            --card-background: #ffffff;
            --text-color: #1e293b;
            --border-color: #e2e8f0;
        }

        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            direction: rtl;
            text-align: right;
            background-color: var(--background-color);
            color: var(--text-color);
            margin: 0;
            padding: 2rem;
            line-height: 1.6;
        }

        .container {
            max-width: 1200px;
            margin: 0 auto;
        }

        h1 {
            color: var(--primary-color);
            text-align: center;
            margin-bottom: 2rem;
            font-size: 2.5rem;
        }

        .module-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
            gap: 1.5rem;
        }

        .module-card {
            background: var(--card-background);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 1.5rem;
            transition: box-shadow 0.2s, transform 0.2s;
        }

        .module-card:hover {
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
            transform: translateY(-2px);
        }

        .module-card h2 {
            margin: 0 0 1rem 0;
            color: var(--secondary-color);
        }

        .module-card a {
            color: var(--primary-color);
            text-decoration: none;
            font-weight: bold;
        }

        .module-card a:hover {
            text-decoration: underline;
        }

        .module-stats {
            color: #64748b;
            font-size: 0.9rem;
            margin-top: 0.5rem;
        }

        footer {
            text-align: center;
            margin-top: 3rem;
            padding-top: 1rem;
            border-top: 1px solid var(--border-color);
            color: #64748b;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>توثيق ترقيم<br><small style="font-size: 0.5em; color: #64748b;">Tarqeem Documentation</small></h1>

        <div class="module-grid">
"#,
    );

    for (name, doc) in docs {
        let func_count = doc.items.iter().filter(|i| matches!(i, crate::doc::model::DocItem::Function(_))).count();
        let class_count = doc.items.iter().filter(|i| matches!(i, crate::doc::model::DocItem::Class(_))).count();
        let interface_count = doc.items.iter().filter(|i| matches!(i, crate::doc::model::DocItem::Interface(_))).count();

        html.push_str(&format!(
            r#"            <div class="module-card">
                <h2><a href="{}.html">{}</a></h2>
                {}
                <div class="module-stats">
                    {} دوال | {} أصناف | {} واجهات
                </div>
            </div>
"#,
            name,
            name,
            doc.description.as_ref().map(|d| format!("<p>{}</p>", d)).unwrap_or_default(),
            func_count,
            class_count,
            interface_count
        ));
    }

    html.push_str(
        r#"        </div>

        <footer>
            <p>تم توليده بواسطة trqdoc | Generated by trqdoc</p>
        </footer>
    </div>
</body>
</html>
"#,
    );

    html
}
