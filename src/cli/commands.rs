//! CLI command implementations

use super::{Cli, Commands};
use crate::codegen::{Linker, LlvmCodegen, Target, target::TargetTriple};
use crate::error::Language;
use crate::ir::{IrBuilder, OptLevel, Optimizer};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::semantic::Analyzer;
use colored::Colorize;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

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
                format!("IR generation error: {} / خطأ في توليد الـ IR: {}", e.message, e.message_ar)
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
                println!("{}", "=== Optimization Stats / إحصائيات التحسين ===".cyan().bold());
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
                    .ok_or_else(|| format!("Invalid target triple: {} / هدف غير صالح: {}", triple_str, triple_str))?
            } else {
                Target::native()
            };

            let mut codegen = LlvmCodegen::new(target_config.clone());
            let llvm_ir = codegen.generate(&ir_module).map_err(|e| {
                format!("Code generation error: {} / خطأ في توليد الكود: {}", e.message, e.message_ar)
            })?;

            // Determine output path
            let output_path = output.unwrap_or_else(|| {
                let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
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
                fs::write(&output_path, &llvm_ir)
                    .map_err(|e| format!("Could not write output: {} / لا يمكن كتابة الملف: {}", e, e))?;
                println!(
                    "{}",
                    format!("LLVM IR written to: {} / تم كتابة LLVM IR إلى: {}",
                        output_path.display(), output_path.display()).green()
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
                    linker.compile_to_assembly(&llvm_ir, &output_path)
                        .map_err(|e| format!("Assembly generation failed: {} / فشل توليد التجميع: {}", e.message, e.message_ar))?;
                    println!(
                        "{}",
                        format!("Assembly written to: {} / تم كتابة التجميع إلى: {}",
                            output_path.display(), output_path.display()).green()
                    );
                } else {
                    linker.compile_to_object(&llvm_ir, &output_path)
                        .map_err(|e| format!("Object compilation failed: {} / فشل ترجمة الكائن: {}", e.message, e.message_ar))?;
                    println!(
                        "{}",
                        format!("Object file written to: {} / تم كتابة ملف الكائن إلى: {}",
                            output_path.display(), output_path.display()).green()
                    );
                }
            } else {
                // Compile to executable
                let linker = Linker::new(target_config)
                    .optimization_level(opt_level as u32)
                    .verbose(cli.verbose);

                if linker.is_available() {
                    linker.compile_to_executable(&llvm_ir, &output_path, None)
                        .map_err(|e| format!("Linking failed: {} / فشل الربط: {}", e.message, e.message_ar))?;
                    println!(
                        "{}",
                        format!("Executable created: {} / تم إنشاء الملف التنفيذي: {}",
                            output_path.display(), output_path.display()).green()
                    );
                } else {
                    // Fallback: just write LLVM IR
                    let ll_path = output_path.with_extension("ll");
                    fs::write(&ll_path, &llvm_ir)
                        .map_err(|e| format!("Could not write output: {} / لا يمكن كتابة الملف: {}", e, e))?;
                    println!(
                        "{}",
                        "Note: No compiler found. LLVM IR written instead. / ملاحظة: لم يتم العثور على مترجم. تم كتابة LLVM IR بدلاً من ذلك.".yellow()
                    );
                    println!(
                        "  You can compile with: clang {} -o {} / يمكنك الترجمة بـ: clang {} -o {}",
                        ll_path.display(), output_path.display(),
                        ll_path.display(), output_path.display()
                    );
                }
            }

            if cli.verbose {
                println!("{}", "Compilation successful! / تمت الترجمة بنجاح!".green().bold());
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
                    println!(
                        "  Optimization level: {} / مستوى التحسين: {}",
                        opt, opt
                    );
                }
            }

            Ok(())
        }

        Commands::Run { file } => {
            let source = fs::read_to_string(&file)
                .map_err(|e| format!("Could not read file: {} / لا يمكن قراءة الملف: {}", e, e))?;

            let filename = file.display().to_string();

            // Parse and analyze
            let mut parser = Parser::new(&source);
            let ast = parser.parse().map_err(|e| {
                e.emit(&source, &filename, lang);
                format!("Parse error / خطأ في التحليل")
            })?;

            let mut analyzer = Analyzer::new();
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

            // TODO: Interpret/execute the program
            println!(
                "{}",
                "Note: Execution not yet implemented / ملاحظة: التنفيذ غير مُنفذ بعد"
                    .yellow()
                    .bold()
            );
            println!("{}", "Program parsed and analyzed successfully! / تم التحليل بنجاح!".green());

            Ok(())
        }

        Commands::Check { file } => {
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
            println!("{}", "=== Tarqeem REPL / الوضع التفاعلي لترقيم ===".cyan().bold());
            println!("Type 'exit' or 'خروج' to quit / اكتب 'exit' أو 'خروج' للخروج");
            println!();

            let stdin = io::stdin();
            let mut stdout = io::stdout();

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

                        // Parse and analyze the line
                        let mut parser = Parser::new(trimmed);
                        match parser.parse() {
                            Ok(ast) => {
                                let mut analyzer = Analyzer::new();
                                if let Err(diagnostics) = analyzer.analyze(&ast) {
                                    for diag in &diagnostics {
                                        diag.emit(trimmed, "<repl>", lang);
                                    }
                                } else {
                                    println!("{}", "OK".green());
                                    if cli.verbose {
                                        println!("{:#?}", ast);
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
    }
}
