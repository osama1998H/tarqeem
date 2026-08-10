//! Debug command implementation.

use crate::debug::{
    DebugCommand, DebugCommandParser, DebugContext, DebugInterpreter, DebugState, StepResult,
};
use crate::error::Language;
use crate::ir::IrBuilder;
use crate::parser::Parser;
use crate::semantic::Analyzer;
use crate::utils::CompilerContext;
use colored::Colorize;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use super::{analyzer_file_path, configure_analyzer, warn_invalid_extension};

/// Arguments for the debug command
pub struct DebugArgs {
    pub file: PathBuf,
    pub stop_on_entry: bool,
    pub dap_port: Option<u16>,
    pub dap_stdio: bool,
    pub arabic: bool,
    pub verbose: bool,
}

pub fn debug(args: DebugArgs, lang: Language) -> Result<(), String> {
    warn_invalid_extension(&args.file);

    // Arabic-only: ترقيم لغة برمجة عربية
    let source =
        fs::read_to_string(&args.file).map_err(|e| format!("لا يمكن قراءة الملف: {}", e))?;

    let filename = args.file.display().to_string();

    // Create compiler context with string interner
    let mut _ctx = CompilerContext::new();

    let mut parser = Parser::new(&source);
    let ast = parser.parse().map_err(|e| {
        e.emit(&source, &filename, lang);
        "خطأ في التحليل".to_string()
    })?;

    let mut analyzer = Analyzer::for_file(analyzer_file_path(&args.file));
    configure_analyzer(&mut analyzer, args.verbose);
    if let Err(diagnostics) = analyzer.analyze(&ast) {
        for diag in &diagnostics {
            diag.emit(&source, &filename, lang);
        }
        return Err(format!("وُجد {} خطأ/أخطاء", diagnostics.len()));
    }

    // Imported module bodies only reach the debug interpreter through this
    // merge; the IR builder itself accepts a single Ast and drops `استورد`.
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

    let ir_builder = IrBuilder::new(filename.clone());
    let ir_module = ir_builder
        .build(&linked)
        .map_err(|e| format!("خطأ بناء التمثيل الوسيط: {}", e.message))?;

    if args.dap_stdio || args.dap_port.is_some() {
        use crate::debug::DapServer;

        let server = DapServer::new();

        // Arabic-only: ترقيم لغة برمجة عربية
        if args.dap_stdio {
            server
                .run_stdio()
                .map_err(|e| format!("خطأ خادم التصحيح: {}", e.message))?;
        } else if let Some(port) = args.dap_port {
            server
                .run_tcp(port)
                .map_err(|e| format!("خطأ خادم التصحيح: {}", e.message))?;
        }

        return Ok(());
    }

    let mut context = DebugContext::new();
    context.config_mut().stop_on_entry = args.stop_on_entry;
    context.config_mut().use_arabic = args.arabic;
    context.set_source(args.file.clone(), source.clone());

    let mut debug_interpreter = DebugInterpreter::new(ir_module, context);

    debug_interpreter
        .start()
        .map_err(|e| format!("فشل بدء المصحح: {}", e.message))?;

    // Arabic-only: ترقيم لغة برمجة عربية
    println!("{}", "=== مصحح ترقيم ===".cyan().bold());
    println!("اكتب 'مساعدة' للحصول على قائمة الأوامر");
    println!();

    run_debug_loop(&mut debug_interpreter, &args.file, &source)
}

fn run_debug_loop(
    debug_interpreter: &mut DebugInterpreter,
    file: &Path,
    source: &str,
) -> Result<(), String> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Arabic-only: ترقيم لغة برمجة عربية
    loop {
        let state = debug_interpreter.context().state();
        match state {
            DebugState::Paused { reason } => {
                let reason_str = reason.description_ar();
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
                break;
            }
            DebugState::Error { message } => {
                println!("{}", format!("خطأ: {}", message).red().bold());
                break;
            }
            _ => {}
        }

        // Arabic-only prompt
        print!("{}", "ترقيم> ".green().bold());
        if stdout.flush().is_err() {
            eprintln!("خطأ في الإدخال والإخراج");
            break;
        }

        let mut input = String::new();
        match stdin.lock().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let cmd = DebugCommandParser::parse(input.trim());
                handle_debug_command(debug_interpreter, cmd, file, source)?;
            }
            Err(e) => {
                eprintln!("خطأ في قراءة المدخل: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn handle_debug_command(
    debug_interpreter: &mut DebugInterpreter,
    cmd: DebugCommand,
    file: &Path,
    source: &str,
) -> Result<(), String> {
    // Arabic-only: ترقيم لغة برمجة عربية
    match cmd {
        DebugCommand::Quit => {
            println!("مع السلامة!");
            std::process::exit(0);
        }
        DebugCommand::Help => {
            println!("{}", DebugCommandParser::help_text());
        }
        DebugCommand::Continue => match debug_interpreter.continue_execution() {
            Ok(StepResult::Completed(val)) => {
                debug_interpreter
                    .context_mut()
                    .set_state(DebugState::Terminated {
                        exit_value: Some(val.to_display_string()),
                    });
            }
            Ok(StepResult::Paused(reason)) => {
                debug_interpreter
                    .context_mut()
                    .set_state(DebugState::Paused { reason });
            }
            Ok(_) => {}
            Err(e) => {
                println!("{}", format!("خطأ: {}", e.message).red());
            }
        },
        DebugCommand::StepOver
        | DebugCommand::StepInto
        | DebugCommand::StepOut
        | DebugCommand::StepInstruction => {
            if let Some(mode) = cmd.as_step_mode() {
                match debug_interpreter.step(mode) {
                    Ok(StepResult::Completed(val)) => {
                        debug_interpreter
                            .context_mut()
                            .set_state(DebugState::Terminated {
                                exit_value: Some(val.to_display_string()),
                            });
                    }
                    Ok(StepResult::Paused(reason)) => {
                        debug_interpreter
                            .context_mut()
                            .set_state(DebugState::Paused { reason });
                    }
                    Ok(_) => {}
                    Err(e) => {
                        println!("{}", format!("خطأ: {}", e.message).red());
                    }
                }
            }
        }
        DebugCommand::Break {
            file: bp_file,
            line,
        } => {
            let bp_path = bp_file.unwrap_or_else(|| file.to_path_buf());
            match debug_interpreter
                .context_mut()
                .add_breakpoint(bp_path.clone(), line)
            {
                Ok(id) => {
                    println!(
                        "{}",
                        format!("نقطة توقف {} عند {}:{}", id.0, bp_path.display(), line).green()
                    );
                }
                Err(e) => {
                    println!("{}", format!("خطأ: {}", e.message).red());
                }
            }
        }
        DebugCommand::DeleteBreakpoint { id } => {
            match debug_interpreter.context_mut().remove_breakpoint(id) {
                Ok(()) => {
                    println!("تم حذف نقطة التوقف {}", id.0);
                }
                Err(e) => {
                    println!("{}", format!("خطأ: {}", e.message).red());
                }
            }
        }
        DebugCommand::ListBreakpoints => {
            let breakpoints: Vec<_> = debug_interpreter.context().breakpoints().collect();
            if breakpoints.is_empty() {
                println!("لا توجد نقاط توقف");
            } else {
                for bp in breakpoints {
                    let enabled = if bp.enabled { "" } else { " (معطلة)" };
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
                println!("لا توجد متغيرات محلية");
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
        DebugCommand::Print { expression } => match debug_interpreter.evaluate(&expression) {
            Ok(value) => {
                println!("{} = {}", expression, value.to_display_string().yellow());
            }
            Err(e) => {
                println!("{}", format!("خطأ: {}", e.message).red());
            }
        },
        DebugCommand::Backtrace => {
            let frames = debug_interpreter.get_stack_trace();
            if frames.is_empty() {
                println!("لا يوجد مكدس استدعاء");
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
            } else {
                println!("موقع غير معروف");
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
                            println!("{} {:>4} │ {}", marker, line_num.to_string().dimmed(), line);
                        }
                    }
                }
            }
        }
        DebugCommand::Run => {
            // Arabic-only: ترقيم لغة برمجة عربية
            debug_interpreter
                .start()
                .map_err(|e| format!("خطأ: {}", e.message))?;
        }
        DebugCommand::Watch { expression } => {
            // Arabic-only: ترقيم لغة برمجة عربية
            let id = debug_interpreter
                .context_mut()
                .add_watch(expression.clone());
            println!("مراقبة {} تمت إضافتها (معرف: {})", expression, id);
        }
        DebugCommand::Unwatch { id } => {
            // Arabic-only: ترقيم لغة برمجة عربية
            debug_interpreter.context_mut().remove_watch(id);
            println!("تم إزالة المراقبة {}", id);
        }
        DebugCommand::ListWatches => {
            // Arabic-only: ترقيم لغة برمجة عربية
            let watches: Vec<_> = debug_interpreter.context().watches().collect();
            if watches.is_empty() {
                println!("لا توجد تعبيرات مراقبة");
            } else {
                for watch in watches {
                    let value = debug_interpreter
                        .evaluate(&watch.expression)
                        .map(|v| v.to_display_string())
                        .unwrap_or_else(|_| "<error>".to_string());
                    println!("  {} {} = {}", watch.id, watch.expression, value.yellow());
                }
            }
        }
        DebugCommand::ClearBreakpoints => {
            debug_interpreter.context_mut().clear_breakpoints();
            println!("تم مسح جميع نقاط التوقف");
        }
        DebugCommand::EnableBreakpoint { id } => {
            if let Some(bp) = debug_interpreter.context_mut().get_breakpoint_mut(id) {
                bp.enabled = true;
                println!("تم تفعيل نقطة التوقف {}", id.0);
            } else {
                println!("{}", format!("نقطة التوقف {} غير موجودة", id.0).red());
            }
        }
        DebugCommand::DisableBreakpoint { id } => {
            if let Some(bp) = debug_interpreter.context_mut().get_breakpoint_mut(id) {
                bp.enabled = false;
                println!("تم تعطيل نقطة التوقف {}", id.0);
            } else {
                println!("{}", format!("نقطة التوقف {} غير موجودة", id.0).red());
            }
        }
        DebugCommand::Unknown { command } => {
            println!(
                "{}",
                format!("أمر غير معروف: {}. اكتب 'مساعدة' للمساعدة.", command).yellow()
            );
        }
        _ => {
            println!("الأمر غير مدعوم بعد");
        }
    }

    Ok(())
}
