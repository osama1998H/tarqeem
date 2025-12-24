//! CLI command implementations

use super::{Cli, Commands, PkgCommands};
use crate::codegen::{target::TargetTriple, Linker, LlvmCodegen, Target};
use crate::debug::{
    DebugCommand, DebugCommandParser, DebugContext, DebugInterpreter, DebugState, StepResult,
};
use crate::doc::generator::DocGenerator;
use crate::doc::{DocExtractor, HtmlGenerator, JsonGenerator, MarkdownGenerator, OutputFormat};
use crate::error::Language;
use crate::interpreter::Interpreter;
use crate::ir::{IrBuilder, OptLevel, Optimizer};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::semantic::Analyzer;
use crate::utils::{is_valid_source_extension, valid_source_extension_display};
use colored::Colorize;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

fn warn_invalid_extension(file: &Path) {
    if !is_valid_source_extension(file) {
        eprintln!(
            "{}",
            format!(
                "تحذير: الملف لا يحمل امتداد ترقيم صالح ({})",
                valid_source_extension_display()
            )
            .yellow()
        );
        eprintln!(
            "{}",
            format!(
                "Warning: File doesn't have a valid Tarqeem extension ({})",
                valid_source_extension_display()
            )
            .yellow()
        );
    }
}

fn find_runtime() -> Option<PathBuf> {
    let search_paths = [
        std::env::current_exe()
            .ok()?
            .parent()?
            .join("runtime/libtrq.a"),
        std::env::current_exe()
            .ok()?
            .parent()?
            .join("../runtime/libtrq.a"),
        PathBuf::from("runtime/libtrq.a"),
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

fn find_stdlib_path() -> Option<PathBuf> {
    let search_paths: Vec<Option<PathBuf>> = vec![
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("stdlib_trq"))),
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .and_then(|p| p.parent().map(|p| p.join("stdlib_trq")))
        }),
        Some(PathBuf::from("stdlib_trq")),
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
            warn_invalid_extension(&file);

            let source = fs::read_to_string(&file)
                .map_err(|e| format!("Could not read file: {} / لا يمكن قراءة الملف: {}", e, e))?;

            let filename = file.display().to_string();

            if dump_tokens {
                let mut lexer = Lexer::new(&source);
                println!("{}", "=== Tokens / الرموز ===".cyan().bold());
                for token in lexer.tokenize() {
                    println!("  {:?} @ {}", token.kind, token.span);
                }
                return Ok(());
            }

            let mut parser = Parser::new(&source);
            let ast = parser.parse().map_err(|e| {
                e.emit(&source, &filename, lang);
                "Parse error / خطأ في التحليل".to_string()
            })?;

            if dump_ast {
                println!("{}", "=== AST / الشجرة النحوية ===".cyan().bold());
                println!("{:#?}", ast);
                return Ok(());
            }

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

            let module_name = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module")
                .to_string();

            let ir_builder = IrBuilder::new(module_name);
            let mut ir_module = ir_builder.build(&ast).map_err(|e| {
                format!(
                    "IR generation error: {} / خطأ في توليد التمثيل الوسيط: {}",
                    e.message, e.message_ar
                )
            })?;

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

            if emit_llvm {
                fs::write(&output_path, &llvm_ir).map_err(|e| {
                    format!("Could not write output: {} / لا يمكن كتابة الملف: {}", e, e)
                })?;
                println!(
                    "{}",
                    format!(
                        "LLVM IR written to: {} / تم كتابة التمثيل الوسيط إلى: {}",
                        output_path.display(),
                        output_path.display()
                    )
                    .green()
                );
            } else if emit_asm || emit_obj {
                let linker = Linker::new(target_config)
                    .optimization_level(opt_level as u32)
                    .verbose(cli.verbose);

                if !linker.is_available() {
                    return Err("No compiler (clang/llc) found. Install LLVM or use --emit-llvm / لم يتم العثور على مترجم. ثبّت مترجم clang أو استخدم --emit-llvm".to_string());
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
                let linker = Linker::new(target_config)
                    .optimization_level(opt_level as u32)
                    .verbose(cli.verbose);

                if linker.is_available() {
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
            warn_invalid_extension(&file);

            let source = fs::read_to_string(&file)
                .map_err(|e| format!("Could not read file: {} / لا يمكن قراءة الملف: {}", e, e))?;

            let filename = file.display().to_string();

            let mut parser = Parser::new(&source);
            let ast = parser.parse().map_err(|e| {
                e.emit(&source, &filename, lang);
                "Parse error / خطأ في التحليل".to_string()
            })?;

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

            let ir_builder = IrBuilder::new(filename.clone());
            let ir_module = ir_builder.build(&ast).map_err(|e| {
                format!(
                    "IR build error: {} / خطأ بناء التمثيل الوسيط: {}",
                    e.message, e.message_ar
                )
            })?;

            let mut interpreter = Interpreter::new(ir_module);
            match interpreter.run() {
                Ok(_result) => {
                    if cli.verbose {
                        println!(
                            "{}",
                            "Program completed successfully / اكتمل تنفيذ البرنامج بنجاح".green()
                        );
                    }
                }
                Err(e) => {
                    eprintln!("{} {}", "Runtime error / خطأ وقت التشغيل:".red().bold(), e);
                    return Err("Runtime error / خطأ وقت التشغيل".to_string());
                }
            }

            Ok(())
        }

        Commands::Debug {
            file,
            stop_on_entry,
            dap_port,
            dap_stdio,
            arabic,
        } => {
            warn_invalid_extension(&file);

            let source = fs::read_to_string(&file)
                .map_err(|e| format!("Could not read file: {} / لا يمكن قراءة الملف: {}", e, e))?;

            let filename = file.display().to_string();

            let mut parser = Parser::new(&source);
            let ast = parser.parse().map_err(|e| {
                e.emit(&source, &filename, lang);
                "Parse error / خطأ في التحليل".to_string()
            })?;

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

            let ir_builder = IrBuilder::new(filename.clone());
            let ir_module = ir_builder.build(&ast).map_err(|e| {
                format!(
                    "IR build error: {} / خطأ بناء التمثيل الوسيط: {}",
                    e.message, e.message_ar
                )
            })?;

            if dap_stdio || dap_port.is_some() {
                use crate::debug::DapServer;

                let server = DapServer::new();

                if dap_stdio {
                    server.run_stdio().map_err(|e| {
                        format!(
                            "DAP server error: {} / خطأ خادم التصحيح: {}",
                            e.message, e.message_ar
                        )
                    })?;
                } else if let Some(port) = dap_port {
                    server.run_tcp(port).map_err(|e| {
                        format!(
                            "DAP server error: {} / خطأ خادم التصحيح: {}",
                            e.message, e.message_ar
                        )
                    })?;
                }

                return Ok(());
            }

            let mut context = DebugContext::new();
            context.config_mut().stop_on_entry = stop_on_entry;
            context.config_mut().use_arabic = arabic;
            context.set_source(file.clone(), source.clone());

            let mut debug_interpreter = DebugInterpreter::new(ir_module, context);

            debug_interpreter.start().map_err(|e| {
                format!(
                    "Failed to start debugger: {} / فشل بدء المصحح: {}",
                    e.message, e.message_ar
                )
            })?;

            if arabic {
                println!("{}", "=== مصحح ترقيم ===".cyan().bold());
                println!("اكتب 'مساعدة' للحصول على قائمة الأوامر");
            } else {
                println!("{}", "=== Tarqeem Debugger (trqdbg) ===".cyan().bold());
                println!("Type 'help' for a list of commands");
            }
            println!();

            let stdin = io::stdin();
            let mut stdout = io::stdout();

            loop {
                let state = debug_interpreter.context().state();
                match state {
                    DebugState::Paused { reason } => {
                        let reason_str = if arabic {
                            reason.description_ar()
                        } else {
                            reason.description()
                        };
                        println!("{}", format!("[{}]", reason_str).yellow());

                        if let Some(frame) = debug_interpreter.get_stack_trace().first() {
                            if let Some(loc) = &frame.location {
                                println!(
                                    "  {} {}:{}:{}",
                                    "→".green().bold(),
                                    loc.file.display(),
                                    loc.line,
                                    loc.column
                                );

                                if let Some(line) = source.lines().nth(loc.line.saturating_sub(1)) {
                                    println!("    {}", line.dimmed());
                                }
                            } else {
                                println!(
                                    "  {} {} (block {}, instruction {})",
                                    "→".green().bold(),
                                    frame.name,
                                    frame.block_id.0,
                                    frame.inst_idx
                                );
                            }
                        }
                    }
                    DebugState::Terminated { exit_value } => {
                        if arabic {
                            println!(
                                "{}",
                                format!(
                                    "انتهى البرنامج{}",
                                    exit_value
                                        .as_ref()
                                        .map(|v| format!(" بقيمة: {}", v))
                                        .unwrap_or_default()
                                )
                                .green()
                                .bold()
                            );
                        } else {
                            println!(
                                "{}",
                                format!(
                                    "Program terminated{}",
                                    exit_value
                                        .as_ref()
                                        .map(|v| format!(" with value: {}", v))
                                        .unwrap_or_default()
                                )
                                .green()
                                .bold()
                            );
                        }
                        break;
                    }
                    DebugState::Error {
                        message,
                        message_ar,
                    } => {
                        let msg = if arabic { message_ar } else { message };
                        println!("{}", format!("Error: {}", msg).red().bold());
                        break;
                    }
                    _ => {}
                }

                print!("{}", "trqdbg> ".green().bold());
                stdout.flush().unwrap();

                let mut input = String::new();
                match stdin.lock().read_line(&mut input) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let cmd = DebugCommandParser::parse(input.trim());
                        match cmd {
                            DebugCommand::Quit => {
                                if arabic {
                                    println!("مع السلامة!");
                                } else {
                                    println!("Goodbye!");
                                }
                                break;
                            }
                            DebugCommand::Help => {
                                println!("{}", DebugCommandParser::help_text());
                            }
                            DebugCommand::Continue => {
                                match debug_interpreter.continue_execution() {
                                    Ok(StepResult::Completed(val)) => {
                                        debug_interpreter.context_mut().set_state(
                                            DebugState::Terminated {
                                                exit_value: Some(val.to_display_string()),
                                            },
                                        );
                                    }
                                    Ok(StepResult::Paused(reason)) => {
                                        debug_interpreter
                                            .context_mut()
                                            .set_state(DebugState::Paused { reason });
                                    }
                                    Ok(_) => {}
                                    Err(e) => {
                                        println!(
                                            "{}",
                                            format!("Error: {} / {}", e.message, e.message_ar)
                                                .red()
                                        );
                                    }
                                }
                            }
                            DebugCommand::StepOver
                            | DebugCommand::StepInto
                            | DebugCommand::StepOut
                            | DebugCommand::StepInstruction => {
                                if let Some(mode) = cmd.as_step_mode() {
                                    match debug_interpreter.step(mode) {
                                        Ok(StepResult::Completed(val)) => {
                                            debug_interpreter.context_mut().set_state(
                                                DebugState::Terminated {
                                                    exit_value: Some(val.to_display_string()),
                                                },
                                            );
                                        }
                                        Ok(StepResult::Paused(reason)) => {
                                            debug_interpreter
                                                .context_mut()
                                                .set_state(DebugState::Paused { reason });
                                        }
                                        Ok(_) => {}
                                        Err(e) => {
                                            println!(
                                                "{}",
                                                format!("Error: {} / {}", e.message, e.message_ar)
                                                    .red()
                                            );
                                        }
                                    }
                                }
                            }
                            DebugCommand::Break {
                                file: bp_file,
                                line,
                            } => {
                                let bp_path = bp_file.unwrap_or_else(|| file.clone());
                                match debug_interpreter
                                    .context_mut()
                                    .add_breakpoint(bp_path.clone(), line)
                                {
                                    Ok(id) => {
                                        if arabic {
                                            println!(
                                                "{}",
                                                format!(
                                                    "نقطة توقف {} عند {}:{}",
                                                    id.0,
                                                    bp_path.display(),
                                                    line
                                                )
                                                .green()
                                            );
                                        } else {
                                            println!(
                                                "{}",
                                                format!(
                                                    "Breakpoint {} at {}:{}",
                                                    id.0,
                                                    bp_path.display(),
                                                    line
                                                )
                                                .green()
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        println!(
                                            "{}",
                                            format!("Error: {} / {}", e.message, e.message_ar)
                                                .red()
                                        );
                                    }
                                }
                            }
                            DebugCommand::DeleteBreakpoint { id } => {
                                match debug_interpreter.context_mut().remove_breakpoint(id) {
                                    Ok(()) => {
                                        if arabic {
                                            println!("تم حذف نقطة التوقف {}", id.0);
                                        } else {
                                            println!("Deleted breakpoint {}", id.0);
                                        }
                                    }
                                    Err(e) => {
                                        println!(
                                            "{}",
                                            format!("Error: {} / {}", e.message, e.message_ar)
                                                .red()
                                        );
                                    }
                                }
                            }
                            DebugCommand::ListBreakpoints => {
                                let breakpoints: Vec<_> =
                                    debug_interpreter.context().breakpoints().collect();
                                if breakpoints.is_empty() {
                                    if arabic {
                                        println!("لا توجد نقاط توقف");
                                    } else {
                                        println!("No breakpoints");
                                    }
                                } else {
                                    for bp in breakpoints {
                                        let enabled = if bp.enabled { "" } else { " (disabled)" };
                                        println!(
                                            "  {} {}:{} {}{}",
                                            bp.id.0,
                                            bp.file.display(),
                                            bp.line,
                                            bp.condition
                                                .as_ref()
                                                .map(|c| format!("if {}", c))
                                                .unwrap_or_default(),
                                            enabled
                                        );
                                    }
                                }
                            }
                            DebugCommand::Locals => {
                                let locals = debug_interpreter.get_locals();
                                if locals.is_empty() {
                                    if arabic {
                                        println!("لا توجد متغيرات محلية");
                                    } else {
                                        println!("No local variables");
                                    }
                                } else {
                                    for var in locals {
                                        println!(
                                            "  {} {} = {} : {}",
                                            if var.is_mutable {
                                                "متغير"
                                            } else {
                                                "ثابت"
                                            },
                                            var.name,
                                            var.value.yellow(),
                                            var.type_name.dimmed()
                                        );
                                    }
                                }
                            }
                            DebugCommand::Print { expression } => {
                                match debug_interpreter.evaluate(&expression) {
                                    Ok(value) => {
                                        println!(
                                            "{} = {}",
                                            expression,
                                            value.to_display_string().yellow()
                                        );
                                    }
                                    Err(e) => {
                                        println!(
                                            "{}",
                                            format!("Error: {} / {}", e.message, e.message_ar)
                                                .red()
                                        );
                                    }
                                }
                            }
                            DebugCommand::Backtrace => {
                                let frames = debug_interpreter.get_stack_trace();
                                if frames.is_empty() {
                                    if arabic {
                                        println!("لا يوجد مكدس استدعاء");
                                    } else {
                                        println!("No call stack");
                                    }
                                } else {
                                    for (i, frame) in frames.iter().enumerate() {
                                        let loc = frame
                                            .location
                                            .as_ref()
                                            .map(|l| format!(" at {}:{}", l.file.display(), l.line))
                                            .unwrap_or_default();
                                        println!("  #{} {}{}", i, frame.name, loc);
                                    }
                                }
                            }
                            DebugCommand::Where => {
                                if let Some(frame) = debug_interpreter.get_stack_trace().first() {
                                    if let Some(loc) = &frame.location {
                                        println!(
                                            "{}:{}:{} in {}",
                                            loc.file.display(),
                                            loc.line,
                                            loc.column,
                                            frame.name
                                        );
                                    } else {
                                        println!(
                                            "{} (block {}, instruction {})",
                                            frame.name, frame.block_id.0, frame.inst_idx
                                        );
                                    }
                                } else if arabic {
                                    println!("موقع غير معروف");
                                } else {
                                    println!("Unknown location");
                                }
                            }
                            DebugCommand::List { lines } => {
                                let num_lines = lines.unwrap_or(10);
                                if let Some(frame) = debug_interpreter.get_stack_trace().first() {
                                    if let Some(loc) = &frame.location {
                                        let start = loc.line.saturating_sub(num_lines / 2);
                                        let end = start + num_lines;
                                        for (i, line) in source.lines().enumerate() {
                                            let line_num = i + 1;
                                            if line_num >= start && line_num <= end {
                                                let marker = if line_num == loc.line {
                                                    "→".green().bold()
                                                } else {
                                                    " ".normal()
                                                };
                                                println!(
                                                    "{} {:>4} │ {}",
                                                    marker,
                                                    line_num.to_string().dimmed(),
                                                    line
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            DebugCommand::Run => {
                                debug_interpreter.start().map_err(|e| {
                                    format!("Error: {} / {}", e.message, e.message_ar)
                                })?;
                            }
                            DebugCommand::Watch { expression } => {
                                let id = debug_interpreter
                                    .context_mut()
                                    .add_watch(expression.clone());
                                if arabic {
                                    println!("مراقبة {} تمت إضافتها (معرف: {})", expression, id);
                                } else {
                                    println!("Watch {} added (id: {})", expression, id);
                                }
                            }
                            DebugCommand::Unwatch { id } => {
                                debug_interpreter.context_mut().remove_watch(id);
                                if arabic {
                                    println!("تم إزالة المراقبة {}", id);
                                } else {
                                    println!("Watch {} removed", id);
                                }
                            }
                            DebugCommand::ListWatches => {
                                let watches: Vec<_> =
                                    debug_interpreter.context().watches().collect();
                                if watches.is_empty() {
                                    if arabic {
                                        println!("لا توجد تعبيرات مراقبة");
                                    } else {
                                        println!("No watch expressions");
                                    }
                                } else {
                                    for watch in watches {
                                        let value = debug_interpreter
                                            .evaluate(&watch.expression)
                                            .map(|v| v.to_display_string())
                                            .unwrap_or_else(|_| "<error>".to_string());
                                        println!(
                                            "  {} {} = {}",
                                            watch.id,
                                            watch.expression,
                                            value.yellow()
                                        );
                                    }
                                }
                            }
                            DebugCommand::ClearBreakpoints => {
                                debug_interpreter.context_mut().clear_breakpoints();
                                if arabic {
                                    println!("تم مسح جميع نقاط التوقف");
                                } else {
                                    println!("All breakpoints cleared");
                                }
                            }
                            DebugCommand::EnableBreakpoint { id } => {
                                if let Some(bp) =
                                    debug_interpreter.context_mut().get_breakpoint_mut(id)
                                {
                                    bp.enabled = true;
                                    if arabic {
                                        println!("تم تفعيل نقطة التوقف {}", id.0);
                                    } else {
                                        println!("Breakpoint {} enabled", id.0);
                                    }
                                } else {
                                    println!("{}", format!("Breakpoint {} not found", id.0).red());
                                }
                            }
                            DebugCommand::DisableBreakpoint { id } => {
                                if let Some(bp) =
                                    debug_interpreter.context_mut().get_breakpoint_mut(id)
                                {
                                    bp.enabled = false;
                                    if arabic {
                                        println!("تم تعطيل نقطة التوقف {}", id.0);
                                    } else {
                                        println!("Breakpoint {} disabled", id.0);
                                    }
                                } else {
                                    println!("{}", format!("Breakpoint {} not found", id.0).red());
                                }
                            }
                            DebugCommand::Unknown { command } => {
                                if arabic {
                                    println!(
                                        "{}",
                                        format!(
                                            "أمر غير معروف: {}. اكتب 'مساعدة' للمساعدة.",
                                            command
                                        )
                                        .yellow()
                                    );
                                } else {
                                    println!(
                                        "{}",
                                        format!(
                                            "Unknown command: {}. Type 'help' for help.",
                                            command
                                        )
                                        .yellow()
                                    );
                                }
                            }
                            _ => {
                                if arabic {
                                    println!("الأمر غير مدعوم بعد");
                                } else {
                                    println!("Command not yet supported");
                                }
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

        Commands::Check { file } => {
            warn_invalid_extension(&file);

            let source = fs::read_to_string(&file)
                .map_err(|e| format!("Could not read file: {} / لا يمكن قراءة الملف: {}", e, e))?;

            let filename = file.display().to_string();

            let mut parser = Parser::new(&source);
            let ast = parser.parse().map_err(|e| {
                e.emit(&source, &filename, lang);
                "Parse error / خطأ في التحليل".to_string()
            })?;

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
                                    let module_name = format!("<repl:{}>", line_count);
                                    let ir_builder = IrBuilder::new(module_name);
                                    match ir_builder.build(&ast) {
                                        Ok(ir_module) => {
                                            let mut interpreter = Interpreter::new(ir_module);
                                            match interpreter.run() {
                                                Ok(result) => {
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

        Commands::Fmt {
            path,
            write,
            check,
            diff,
            config,
            sample_config,
        } => {
            use crate::fmt::{self, FormatConfig};

            if sample_config {
                println!("{}", FormatConfig::sample_config());
                return Ok(());
            }

            let path = path.ok_or_else(|| {
                "Path is required. Use 'tarqeem fmt <file>' or 'tarqeem fmt --sample-config' / المسار مطلوب".to_string()
            })?;

            let format_config = if let Some(config_path) = config {
                FormatConfig::from_file(&config_path).map_err(|e| {
                    format!(
                        "Could not load config: {} / لا يمكن تحميل الإعدادات: {}",
                        e, e
                    )
                })?
            } else {
                FormatConfig::find_and_load().unwrap_or_default()
            };

            let files: Vec<PathBuf> = if path.is_dir() {
                collect_source_files(&path)?
            } else {
                warn_invalid_extension(&path);
                vec![path.clone()]
            };

            if files.is_empty() {
                return Err("No source files found / لم يتم العثور على ملفات مصدر".to_string());
            }

            let mut all_formatted = true;
            let mut files_changed = 0;

            for file in &files {
                let source = fs::read_to_string(file).map_err(|e| {
                    format!(
                        "Could not read file {}: {} / لا يمكن قراءة الملف {}: {}",
                        file.display(),
                        e,
                        file.display(),
                        e
                    )
                })?;

                let formatted = fmt::format_source(&source, &format_config).map_err(|e| {
                    format!(
                        "Format error in {}: {} / خطأ التنسيق في {}: {}",
                        file.display(),
                        e,
                        file.display(),
                        e
                    )
                })?;

                let is_changed = source != formatted;

                if check {
                    if is_changed {
                        all_formatted = false;
                        eprintln!(
                            "{}",
                            format!(
                                "Would reformat: {} / سيتم إعادة تنسيق: {}",
                                file.display(),
                                file.display()
                            )
                            .yellow()
                        );
                    }
                } else if diff {
                    if is_changed {
                        println!("{}", format!("--- {} (original)", file.display()).red());
                        println!("{}", format!("+++ {} (formatted)", file.display()).green());

                        let diff_output = fmt::show_diff(&source, &format_config)
                            .map_err(|e| format!("{}", e))?;
                        println!("{}", diff_output);
                    }
                } else if write {
                    if is_changed {
                        fs::write(file, &formatted).map_err(|e| {
                            format!(
                                "Could not write file {}: {} / لا يمكن كتابة الملف {}: {}",
                                file.display(),
                                e,
                                file.display(),
                                e
                            )
                        })?;
                        files_changed += 1;
                        if cli.verbose {
                            println!(
                                "{}",
                                format!(
                                    "Formatted: {} / تم تنسيق: {}",
                                    file.display(),
                                    file.display()
                                )
                                .green()
                            );
                        }
                    }
                } else {
                    print!("{}", formatted);
                }
            }

            if check {
                if all_formatted {
                    println!(
                        "{}",
                        "All files are formatted correctly / جميع الملفات منسقة بشكل صحيح"
                            .green()
                            .bold()
                    );
                    Ok(())
                } else {
                    Err("Some files need formatting / بعض الملفات تحتاج تنسيق".to_string())
                }
            } else if write && cli.verbose {
                println!(
                    "{}",
                    format!(
                        "{} file(s) formatted / تم تنسيق {} ملف(ات)",
                        files_changed, files_changed
                    )
                    .green()
                    .bold()
                );
                Ok(())
            } else {
                Ok(())
            }
        }

        Commands::Lex { file } => {
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
            if cli.verbose {
                eprintln!(
                    "{}",
                    "Starting Tarqeem Language Server... / جاري بدء خادم لغة ترقيم..."
                        .cyan()
                        .bold()
                );
            }

            let runtime = tokio::runtime::Runtime::new().map_err(|e| {
                format!(
                    "Failed to create runtime: {} / فشل إنشاء وقت التشغيل: {}",
                    e, e
                )
            })?;

            runtime.block_on(async {
                crate::lsp::run_server()
                    .await
                    .map_err(|e| format!("LSP server error: {} / خطأ خادم LSP: {}", e, e))
            })
        }

        Commands::Doc {
            path,
            output,
            format,
            single_file,
        } => {
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

            let source_files: Vec<PathBuf> = if path.is_dir() {
                collect_source_files(&path)?
            } else {
                warn_invalid_extension(&path);
                vec![path.clone()]
            };

            if source_files.is_empty() {
                return Err("No source files found / لم يتم العثور على ملفات مصدر".to_string());
            }

            let output_dir = output.unwrap_or_else(|| {
                if path.is_dir() {
                    path.join("docs")
                } else {
                    path.parent()
                        .map(|p| p.join("docs"))
                        .unwrap_or_else(|| PathBuf::from("docs"))
                }
            });

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

                let mut parser = Parser::new(&source);
                let ast = parser.parse().map_err(|e| {
                    e.emit(&source, &filename, lang);
                    format!(
                        "Parse error in {}: / خطأ في تحليل {}:",
                        source_file.display(),
                        source_file.display()
                    )
                })?;

                let extractor = DocExtractor::new(module_name.clone(), filename);
                let doc = extractor.extract(&ast);

                if cli.verbose {
                    let item_count = doc.items.len();
                    println!(
                        "  {} - {} items / {} عنصر",
                        module_name, item_count, item_count
                    );
                }

                all_docs.push((module_name, doc));
            }

            if single_file {
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

                    if let Some(parent) = output_file.parent() {
                        fs::create_dir_all(parent).map_err(|e| {
                            format!(
                                "Could not create output directory: {} / لا يمكن إنشاء مجلد الإخراج: {}",
                                e, e
                            )
                        })?;
                    }

                    fs::write(&output_file, output_buffer).map_err(|e| {
                        format!("Could not write output: {} / لا يمكن كتابة الإخراج: {}", e, e)
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
                        format!("Could not write output: {} / لا يمكن كتابة الإخراج: {}", e, e)
                    })?;
                }

                if output_format == OutputFormat::Html && all_docs.len() > 1 {
                    let index_content = generate_html_index(&all_docs);
                    let index_file = output_dir.join("index.html");
                    fs::write(&index_file, index_content).map_err(|e| {
                        format!("Could not write index: {} / لا يمكن كتابة الفهرس: {}", e, e)
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

fn collect_source_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    let entries = fs::read_dir(dir).map_err(|e| {
        format!(
            "Could not read directory: {} / لا يمكن قراءة المجلد: {}",
            e, e
        )
    })?;

    for entry in entries {
        let entry = entry
            .map_err(|e| format!("Could not read entry: {} / لا يمكن قراءة المدخل: {}", e, e))?;
        let path = entry.path();

        if path.is_file() && is_valid_source_extension(&path) {
            files.push(path);
        } else if path.is_dir() {
            files.extend(collect_source_files(&path)?);
        }
    }

    files.sort();

    Ok(files)
}

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
        let func_count = doc
            .items
            .iter()
            .filter(|i| matches!(i, crate::doc::model::DocItem::Function(_)))
            .count();
        let class_count = doc
            .items
            .iter()
            .filter(|i| matches!(i, crate::doc::model::DocItem::Class(_)))
            .count();
        let interface_count = doc
            .items
            .iter()
            .filter(|i| matches!(i, crate::doc::model::DocItem::Interface(_)))
            .count();

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
            doc.description
                .as_ref()
                .map(|d| format!("<p>{}</p>", d))
                .unwrap_or_default(),
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
