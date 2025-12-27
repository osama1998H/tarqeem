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

use super::{configure_analyzer, warn_invalid_extension};

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

    let source = fs::read_to_string(&args.file)
        .map_err(|e| format!("Could not read file: {} / لا يمكن قراءة الملف: {}", e, e))?;

    let filename = args.file.display().to_string();

    // Create compiler context with string interner
    let mut _ctx = CompilerContext::new();

    let mut parser = Parser::new(&source);
    let ast = parser.parse().map_err(|e| {
        e.emit(&source, &filename, lang);
        "Parse error / خطأ في التحليل".to_string()
    })?;

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

    let ir_builder = IrBuilder::new(filename.clone());
    let ir_module = ir_builder.build(&ast).map_err(|e| {
        format!(
            "IR build error: {} / خطأ بناء التمثيل الوسيط: {}",
            e.message, e.message_ar
        )
    })?;

    if args.dap_stdio || args.dap_port.is_some() {
        use crate::debug::DapServer;

        let server = DapServer::new();

        if args.dap_stdio {
            server.run_stdio().map_err(|e| {
                format!(
                    "DAP server error: {} / خطأ خادم التصحيح: {}",
                    e.message, e.message_ar
                )
            })?;
        } else if let Some(port) = args.dap_port {
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
    context.config_mut().stop_on_entry = args.stop_on_entry;
    context.config_mut().use_arabic = args.arabic;
    context.set_source(args.file.clone(), source.clone());

    let mut debug_interpreter = DebugInterpreter::new(ir_module, context);

    debug_interpreter.start().map_err(|e| {
        format!(
            "Failed to start debugger: {} / فشل بدء المصحح: {}",
            e.message, e.message_ar
        )
    })?;

    if args.arabic {
        println!("{}", "=== مصحح ترقيم ===".cyan().bold());
        println!("اكتب 'مساعدة' للحصول على قائمة الأوامر");
    } else {
        println!("{}", "=== Tarqeem Debugger (trqdbg) ===".cyan().bold());
        println!("Type 'help' for a list of commands");
    }
    println!();

    run_debug_loop(&mut debug_interpreter, &args.file, &source, args.arabic)
}

fn run_debug_loop(
    debug_interpreter: &mut DebugInterpreter,
    file: &Path,
    source: &str,
    arabic: bool,
) -> Result<(), String> {
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
                handle_debug_command(debug_interpreter, cmd, file, source, arabic)?;
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
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
    arabic: bool,
) -> Result<(), String> {
    match cmd {
        DebugCommand::Quit => {
            if arabic {
                println!("مع السلامة!");
            } else {
                println!("Goodbye!");
            }
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
                println!(
                    "{}",
                    format!("Error: {} / {}", e.message, e.message_ar).red()
                );
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
                        println!(
                            "{}",
                            format!("Error: {} / {}", e.message, e.message_ar).red()
                        );
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
                    if arabic {
                        println!(
                            "{}",
                            format!("نقطة توقف {} عند {}:{}", id.0, bp_path.display(), line)
                                .green()
                        );
                    } else {
                        println!(
                            "{}",
                            format!("Breakpoint {} at {}:{}", id.0, bp_path.display(), line)
                                .green()
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "{}",
                        format!("Error: {} / {}", e.message, e.message_ar).red()
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
                        format!("Error: {} / {}", e.message, e.message_ar).red()
                    );
                }
            }
        }
        DebugCommand::ListBreakpoints => {
            let breakpoints: Vec<_> = debug_interpreter.context().breakpoints().collect();
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
        DebugCommand::Print { expression } => match debug_interpreter.evaluate(&expression) {
            Ok(value) => {
                println!("{} = {}", expression, value.to_display_string().yellow());
            }
            Err(e) => {
                println!(
                    "{}",
                    format!("Error: {} / {}", e.message, e.message_ar).red()
                );
            }
        },
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
                            println!("{} {:>4} │ {}", marker, line_num.to_string().dimmed(), line);
                        }
                    }
                }
            }
        }
        DebugCommand::Run => {
            debug_interpreter
                .start()
                .map_err(|e| format!("Error: {} / {}", e.message, e.message_ar))?;
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
            let watches: Vec<_> = debug_interpreter.context().watches().collect();
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
                    println!("  {} {} = {}", watch.id, watch.expression, value.yellow());
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
            if let Some(bp) = debug_interpreter.context_mut().get_breakpoint_mut(id) {
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
            if let Some(bp) = debug_interpreter.context_mut().get_breakpoint_mut(id) {
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
                    format!("أمر غير معروف: {}. اكتب 'مساعدة' للمساعدة.", command).yellow()
                );
            } else {
                println!(
                    "{}",
                    format!("Unknown command: {}. Type 'help' for help.", command).yellow()
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

    Ok(())
}
