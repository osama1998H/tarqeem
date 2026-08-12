//! CLI command implementations

mod compile;
mod debug;
mod explain;

pub use compile::{compile, CompileArgs};
pub use debug::{debug, DebugArgs};

use super::{Cli, Commands, PkgCommands};
use crate::doc::{DocExtractor, HtmlGenerator, JsonGenerator, MarkdownGenerator, OutputFormat};
use crate::error::Language;
use crate::interpreter::Interpreter;
use crate::ir::IrBuilder;
use crate::jit::{JitConfig, JitExecutor};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::semantic::Analyzer;
use crate::utils::{is_valid_source_extension, valid_source_extension_display, CompilerContext};
use colored::Colorize;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

/// Stands in for the source file a REPL line does not have, both as the name
/// diagnostics render against and as the anchor relative `استورد` specifiers
/// resolve from.
pub(super) const REPL_PSEUDO_FILE: &str = "<repl>";

// ============================================================================
// Helper Functions
// ============================================================================

pub(super) fn warn_invalid_extension(file: &Path) {
    // Arabic-only: ترقيم لغة برمجة عربية
    if !is_valid_source_extension(file) {
        eprintln!(
            "{}",
            format!(
                "تحذير: الملف لا يحمل امتداد ترقيم صالح ({})",
                valid_source_extension_display()
            )
            .yellow()
        );
    }
}

pub(super) fn find_runtime() -> Option<PathBuf> {
    // Build search paths in priority order
    let mut search_paths: Vec<PathBuf> = Vec::new();

    // 1. TARQEEM_HOME environment variable (highest priority)
    if let Ok(tarqeem_home) = std::env::var("TARQEEM_HOME") {
        search_paths.push(PathBuf::from(&tarqeem_home).join("lib/libtrq.a"));
    }

    // 2. Relative to executable (for development)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            search_paths.push(parent.join("runtime/libtrq.a"));
            search_paths.push(parent.join("../runtime/libtrq.a"));
            // Also check lib/ next to executable (installed layout)
            search_paths.push(parent.join("../lib/libtrq.a"));
        }
    }

    // 3. Relative to current directory (for development)
    search_paths.push(PathBuf::from("runtime/libtrq.a"));

    // 4. User-local path (~/.tarqeem/lib/libtrq.a)
    if let Some(home) = dirs::home_dir() {
        search_paths.push(home.join(".tarqeem/lib/libtrq.a"));
    }

    // 5. System paths (Unix)
    #[cfg(unix)]
    {
        search_paths.push(PathBuf::from("/usr/local/lib/tarqeem/libtrq.a"));
        search_paths.push(PathBuf::from("/usr/lib/tarqeem/libtrq.a"));
    }

    // 5. System paths (Windows)
    #[cfg(windows)]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            search_paths.push(PathBuf::from(&local_app_data).join("Tarqeem/lib/libtrq.a"));
        }
        search_paths.push(PathBuf::from("C:/Program Files/Tarqeem/lib/libtrq.a"));
    }

    for path in &search_paths {
        if path.exists() {
            return Some(path.clone());
        }
    }

    None
}

/// Find the WebAssembly runtime library
pub(super) fn find_wasm_runtime() -> Option<PathBuf> {
    let mut search_paths: Vec<PathBuf> = Vec::new();

    // 1. TARQEEM_HOME environment variable (highest priority)
    if let Ok(tarqeem_home) = std::env::var("TARQEEM_HOME") {
        search_paths.push(PathBuf::from(&tarqeem_home).join("lib/libtrq_wasm.a"));
    }

    // 2. Relative to executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            search_paths.push(parent.join("runtime/wasm/libtrq_wasm.a"));
            if let Some(grandparent) = parent.parent() {
                search_paths.push(grandparent.join("runtime/wasm/libtrq_wasm.a"));
                search_paths.push(grandparent.join("lib/libtrq_wasm.a"));
            }
        }
    }

    // 3. Relative to current directory
    search_paths.push(PathBuf::from("runtime/wasm/libtrq_wasm.a"));

    // 4. User-local path
    if let Some(home) = dirs::home_dir() {
        search_paths.push(home.join(".tarqeem/lib/libtrq_wasm.a"));
    }

    // 5. System paths (Unix)
    #[cfg(unix)]
    {
        search_paths.push(PathBuf::from("/usr/local/lib/tarqeem/libtrq_wasm.a"));
        search_paths.push(PathBuf::from("/usr/lib/tarqeem/libtrq_wasm.a"));
    }

    // 5. System paths (Windows)
    #[cfg(windows)]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            search_paths.push(PathBuf::from(&local_app_data).join("Tarqeem/lib/libtrq_wasm.a"));
        }
        search_paths.push(PathBuf::from("C:/Program Files/Tarqeem/lib/libtrq_wasm.a"));
    }

    search_paths.into_iter().find(|path| path.exists())
}

fn find_stdlib_path() -> Option<PathBuf> {
    let mut search_paths: Vec<PathBuf> = Vec::new();

    // 1. TARQEEM_HOME environment variable (highest priority)
    if let Ok(tarqeem_home) = std::env::var("TARQEEM_HOME") {
        search_paths.push(PathBuf::from(&tarqeem_home).join("stdlib"));
    }

    // 2. Relative to executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            search_paths.push(parent.join("stdlib"));
            if let Some(grandparent) = parent.parent() {
                search_paths.push(grandparent.join("stdlib"));
            }
        }
    }

    // 3. Relative to current directory
    search_paths.push(PathBuf::from("stdlib"));

    // 4. User-local path
    if let Some(home) = dirs::home_dir() {
        search_paths.push(home.join(".tarqeem/stdlib"));
    }

    // 5. System paths (Unix)
    #[cfg(unix)]
    {
        search_paths.push(PathBuf::from("/usr/local/lib/tarqeem/stdlib"));
        search_paths.push(PathBuf::from("/usr/lib/tarqeem/stdlib"));
    }

    // 5. System paths (Windows)
    #[cfg(windows)]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            search_paths.push(PathBuf::from(&local_app_data).join("Tarqeem/stdlib"));
        }
        search_paths.push(PathBuf::from("C:/Program Files/Tarqeem/stdlib"));
    }

    search_paths
        .into_iter()
        .find(|path| path.exists() && path.is_dir())
}

/// Absolute path to hand `Analyzer::for_file`.
///
/// Relative imports resolve against `path.parent()`, which is `""` — not the
/// file's directory — for a bare `برنامج.ترقيم` argument, so the path must be
/// made absolute first. Canonicalization can only fail on a file we already
/// read successfully (races, permissions); the raw path is then no worse than
/// the pre-existing behaviour of no path at all.
pub(super) fn analyzer_file_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub(super) fn configure_analyzer(analyzer: &mut Analyzer, verbose: bool) {
    if let Some(stdlib_path) = find_stdlib_path() {
        // Arabic-only: ترقيم لغة برمجة عربية
        if verbose {
            eprintln!(
                "{}",
                format!("المكتبة القياسية: {}", stdlib_path.display()).dimmed()
            );
        }
        analyzer.add_search_path(stdlib_path);
    }
}

fn collect_source_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    // Arabic-only: ترقيم لغة برمجة عربية
    let entries = fs::read_dir(dir).map_err(|e| format!("لا يمكن قراءة المجلد: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("لا يمكن قراءة المدخل: {}", e))?;
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

// ============================================================================
// Main Command Dispatcher
// ============================================================================

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
            emit_wasm,
            wasm_js_bindings,
            wasm_export_all,
            wasm_memory_pages,
            target,
            dump_tokens,
            dump_ast,
            dump_ir,
            dump_opt_stats,
            timing,
        } => {
            let args = CompileArgs {
                file,
                output,
                opt_level,
                emit_llvm,
                emit_asm,
                emit_obj,
                emit_wasm,
                wasm_js_bindings,
                wasm_export_all,
                wasm_memory_pages,
                target,
                dump_tokens,
                dump_ast,
                dump_ir,
                dump_opt_stats,
                verbose: cli.verbose,
                timing,
            };
            compile(args, lang)
        }

        Commands::Run { file, jit, profile } => run_command(file, jit, profile, cli.verbose, lang),

        Commands::Debug {
            file,
            stop_on_entry,
            dap_port,
            dap_stdio,
            arabic,
        } => {
            let args = DebugArgs {
                file,
                stop_on_entry,
                dap_port,
                dap_stdio,
                arabic,
                verbose: cli.verbose,
            };
            debug(args, lang)
        }

        Commands::Check { file } => check_command(file, cli.verbose, lang),

        Commands::Repl => repl_command(cli.verbose, lang),

        Commands::Fmt {
            path,
            write,
            check,
            diff,
            config,
            sample_config,
        } => fmt_command(path, write, check, diff, config, sample_config, cli.verbose),

        Commands::Lex { file } => lex_command(file),

        Commands::Parse { file, format } => parse_command(file, format, lang),

        Commands::Pkg { command } => pkg_command(command),

        Commands::Lsp => lsp_command(cli.verbose),

        Commands::Doc {
            path,
            output,
            format,
            single_file,
        } => doc_command(path, output, format, single_file, cli.verbose, lang),

        Commands::Explain { code } => explain::explain_command(code, cli.verbose, lang),
    }
}

// ============================================================================
// Individual Command Implementations
// ============================================================================

fn run_command(
    file: PathBuf,
    jit: bool,
    profile: bool,
    verbose: bool,
    lang: Language,
) -> Result<(), String> {
    warn_invalid_extension(&file);

    // Arabic-only: ترقيم لغة برمجة عربية
    let source = fs::read_to_string(&file).map_err(|e| format!("لا يمكن قراءة الملف: {}", e))?;

    let filename = file.display().to_string();

    // Create compiler context with string interner
    let mut _ctx = CompilerContext::new();

    let mut parser = Parser::new(&source);
    let ast = parser.parse().map_err(|e| {
        e.emit(&source, &filename, lang);
        "خطأ في التحليل".to_string()
    })?;

    let mut analyzer = Analyzer::for_file(analyzer_file_path(&file));
    configure_analyzer(&mut analyzer, verbose);
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

    // Imported module bodies only reach the backends through this merge; the IR
    // builder itself accepts a single Ast and drops `استورد`.
    let mut link_warnings = Vec::new();
    let linked = analyzer
        .linked_ast(&ast, &mut link_warnings)
        .map_err(|diagnostics| {
            for diag in &diagnostics {
                diag.emit(&source, &filename, lang);
            }
            format!(
                "{} error(s) found / وُجد {} خطأ/أخطاء",
                diagnostics.len(),
                diagnostics.len()
            )
        })?;
    for diag in &link_warnings {
        diag.emit(&source, &filename, lang);
    }

    let ir_builder = IrBuilder::new(filename.clone());
    let ir_module = ir_builder.build(&linked).map_err(|e| {
        format!(
            "IR build error: {} / خطأ بناء التمثيل الوسيط: {}",
            e.message, e.message
        )
    })?;

    // When --profile is used, always use JIT for profiling data
    let use_jit = jit || profile;

    if use_jit {
        // Use JIT compilation
        if verbose && !profile {
            println!(
                "{}",
                "Using JIT compilation / استخدام الترجمة الفورية".cyan()
            );
        }

        let config = JitConfig::default().with_verbose(verbose && !profile);
        let mut executor = JitExecutor::new(ir_module, config);

        match executor.run() {
            Ok(_result) => {
                if profile {
                    // Output profiling data as JSON for IDE integration
                    println!("{}", executor.profile_summary().to_json());
                } else if verbose {
                    println!(
                        "{}",
                        "Program completed successfully (JIT) / اكتمل تنفيذ البرنامج بنجاح (ترجمة فورية)".green()
                    );
                    // Print JIT statistics
                    println!("\n{}", executor.profile_summary());
                }
            }
            Err(e) => {
                eprintln!(
                    "{} {}",
                    "JIT runtime error / خطأ وقت التشغيل (ترجمة فورية):"
                        .red()
                        .bold(),
                    e
                );
                return Err("JIT runtime error / خطأ وقت التشغيل (ترجمة فورية)".to_string());
            }
        }
    } else {
        // Use interpreter
        let mut interpreter = Interpreter::new(ir_module);
        match interpreter.run() {
            Ok(_result) => {
                if verbose {
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
    }

    Ok(())
}

fn check_command(file: PathBuf, verbose: bool, lang: Language) -> Result<(), String> {
    warn_invalid_extension(&file);

    // Arabic-only: ترقيم لغة برمجة عربية
    let source = fs::read_to_string(&file).map_err(|e| format!("لا يمكن قراءة الملف: {}", e))?;

    let filename = file.display().to_string();

    // Create compiler context with string interner
    let mut _ctx = CompilerContext::new();

    let mut parser = Parser::new(&source);
    let ast = parser.parse().map_err(|e| {
        e.emit(&source, &filename, lang);
        "خطأ في التحليل".to_string()
    })?;

    let mut analyzer = Analyzer::for_file(analyzer_file_path(&file));
    configure_analyzer(&mut analyzer, verbose);
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

    // `analyze` returns Ok when only warnings were raised, so a warnings-only
    // run reported success while silently discarding them — the module
    // not-found warning in particular was invisible. Warnings still must not
    // fail `check`.
    for diag in analyzer.diagnostics() {
        diag.emit(&source, &filename, lang);
    }

    // A top-level name collision between two merged modules is only detected
    // by the link step, so without running it `check` reported success on a
    // program that both `run` and `compile` reject (issue #182). The merged
    // AST itself is not needed here — `check` builds no IR.
    let mut link_warnings = Vec::new();
    analyzer
        .linked_ast(&ast, &mut link_warnings)
        .map_err(|diagnostics| {
            for diag in &diagnostics {
                diag.emit(&source, &filename, lang);
            }
            format!(
                "{} error(s) found / وُجد {} خطأ/أخطاء",
                diagnostics.len(),
                diagnostics.len()
            )
        })?;
    for diag in &link_warnings {
        diag.emit(&source, &filename, lang);
    }

    println!(
        "{}",
        "No errors found! / لم يتم العثور على أخطاء!".green().bold()
    );

    Ok(())
}

/// Analyze one REPL line and fold every module it imports into a single `Ast`.
///
/// Split out of `repl_command` because the REPL was the one pipeline entry
/// point that never ran the linker: it handed the raw AST straight to the IR
/// builder, so an imported function had a symbol-table entry but no body and
/// calling it failed at run time with `دالة غير معرّفة` — the exact issue #182
/// symptom (`run`, `compile`, `check` and the debugger were all wired).
/// A terminal session is not reachable from a test; this is.
///
/// `current_file` is what relative `استورد` specifiers resolve against. A REPL
/// line has no file of its own, so the caller names one in the working
/// directory. `warnings` is an out-parameter, matching `Analyzer::linked_ast`.
/// Did the build produce something to run?
///
/// Both entry-point modes converge on `__main__`: script mode synthesizes it,
/// program mode renames `دالة رئيسية()` to it. A module without one declares
/// and nothing more.
pub(super) fn has_entry_point(module: &crate::ir::Module) -> bool {
    module
        .get_function(&crate::ir::FuncId("__main__".to_string()))
        .is_some()
}

pub(super) fn link_repl_line(
    ast: &crate::parser::Ast,
    current_file: PathBuf,
    verbose: bool,
    warnings: &mut Vec<crate::error::Diagnostic>,
) -> Result<crate::parser::Ast, Vec<crate::error::Diagnostic>> {
    let mut analyzer = Analyzer::for_file(current_file);
    configure_analyzer(&mut analyzer, verbose);
    analyzer.analyze(ast)?;
    analyzer.linked_ast(ast, warnings)
}

fn repl_command(verbose: bool, lang: Language) -> Result<(), String> {
    println!(
        "{}",
        // Arabic-only: ترقيم لغة برمجة عربية
        "=== الوضع التفاعلي لترقيم ===".cyan().bold()
    );
    println!("اكتب 'خروج' للخروج");
    println!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut line_count = 0u32;

    // Relative imports resolve against the working directory, as they did when
    // the REPL passed no file at all; naming a file inside it just states that
    // explicitly, since module resolution works from a file's parent directory.
    let repl_file = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(REPL_PSEUDO_FILE);

    // Create compiler context - reused across all REPL lines for deduplication
    let mut _ctx = CompilerContext::new();

    loop {
        print!("{}", "ترقيم> ".green().bold());
        if stdout.flush().is_err() {
            eprintln!("I/O error / خطأ في الإدخال والإخراج");
            break;
        }

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                // Arabic-only: ترقيم لغة برمجة عربية
                if trimmed == "خروج" {
                    println!("مع السلامة!");
                    break;
                }

                if trimmed.is_empty() {
                    continue;
                }

                line_count += 1;

                let source = wrap_repl_input(trimmed);
                let mut parser = Parser::new(&source);
                match parser.parse() {
                    Ok(ast) => {
                        let mut link_warnings = Vec::new();
                        // A rejected line must not end the session, so the
                        // diagnostics are printed here rather than propagated
                        // the way `run_command` propagates them.
                        match link_repl_line(&ast, repl_file.clone(), verbose, &mut link_warnings) {
                            Err(diagnostics) => {
                                for diag in &diagnostics {
                                    diag.emit(&source, REPL_PSEUDO_FILE, lang);
                                }
                            }
                            Ok(linked) => {
                                for diag in &link_warnings {
                                    diag.emit(&source, REPL_PSEUDO_FILE, lang);
                                }

                                let module_name = format!("<repl:{}>", line_count);
                                let ir_builder = IrBuilder::new(module_name);
                                // A line that only declares (`متغير س = ٥`) is a
                                // library, not a program: `build` would reject
                                // it for having no entry point (ت٠٢٠٢).
                                match ir_builder.build_library(&linked) {
                                    Ok(ir_module) if !has_entry_point(&ir_module) => {
                                        // Nothing to execute, so executing
                                        // anything would be wrong: the
                                        // interpreter's fallback runs whatever
                                        // function it finds first, which for
                                        // `دالة ف(س)` means calling it with no
                                        // arguments. A declaration is silent.
                                    }
                                    Ok(ir_module) => {
                                        let mut interpreter = Interpreter::new(ir_module);
                                        match interpreter.run() {
                                            Ok(result) => {
                                                if verbose {
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
                    }
                    Err(e) => {
                        e.emit(&source, REPL_PSEUDO_FILE, lang);
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

fn fmt_command(
    path: Option<PathBuf>,
    write: bool,
    check: bool,
    diff: bool,
    config: Option<PathBuf>,
    sample_config: bool,
    verbose: bool,
) -> Result<(), String> {
    use crate::fmt::{self, FormatConfig};

    if sample_config {
        println!("{}", FormatConfig::sample_config());
        return Ok(());
    }

    let path = path.ok_or_else(|| {
        "Path is required. Use 'tarqeem fmt <file>' or 'tarqeem fmt --sample-config' / المسار مطلوب"
            .to_string()
    })?;

    let format_config = if let Some(config_path) = config {
        FormatConfig::from_file(&config_path)
            .map_err(|e| format!("لا يمكن تحميل الإعدادات: {}", e))?
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
        // Arabic-only: ترقيم لغة برمجة عربية
        let source = fs::read_to_string(file)
            .map_err(|e| format!("لا يمكن قراءة الملف {}: {}", file.display(), e))?;

        let formatted = fmt::format_source(&source, &format_config)
            .map_err(|e| format!("خطأ التنسيق في {}: {}", file.display(), e))?;

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

                println!("{}", fmt::diff_of(&source, &formatted));
            }
        } else if write {
            if is_changed {
                // Arabic-only: ترقيم لغة برمجة عربية
                fs::write(file, &formatted)
                    .map_err(|e| format!("لا يمكن كتابة الملف {}: {}", file.display(), e))?;
                files_changed += 1;
                if verbose {
                    println!("{}", format!("تم تنسيق: {}", file.display()).green());
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
    } else if write && verbose {
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

fn lex_command(file: PathBuf) -> Result<(), String> {
    warn_invalid_extension(&file);

    // Arabic-only: ترقيم لغة برمجة عربية
    let source = fs::read_to_string(&file).map_err(|e| format!("لا يمكن قراءة الملف: {}", e))?;

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();

    println!("{}", "=== الرموز ===".cyan().bold());
    for token in tokens {
        println!(
            "  [{:>4}:{:<3}] {:?} '{}'",
            token.span.line, token.span.column, token.kind, token.lexeme
        );
    }

    Ok(())
}

fn parse_command(file: PathBuf, format: String, lang: Language) -> Result<(), String> {
    warn_invalid_extension(&file);

    // Arabic-only: ترقيم لغة برمجة عربية
    let source = fs::read_to_string(&file).map_err(|e| format!("لا يمكن قراءة الملف: {}", e))?;

    let filename = file.display().to_string();

    let mut parser = Parser::new(&source);
    match parser.parse() {
        Ok(ast) => {
            match format.as_str() {
                "json" => {
                    // Output AST as JSON for IDE integration
                    let json = serde_json::to_string_pretty(&ast)
                        .map_err(|e| format!("خطأ في تحويل JSON: {}", e))?;
                    println!("{}", json);
                }
                _ => {
                    // Default: debug format
                    println!("{}", "=== الشجرة النحوية ===".cyan().bold());
                    println!("{:#?}", ast);
                }
            }
        }
        Err(e) => {
            e.emit(&source, &filename, lang);
            return Err("خطأ في التحليل".to_string());
        }
    }

    Ok(())
}

fn pkg_command(command: PkgCommands) -> Result<(), String> {
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

fn lsp_command(verbose: bool) -> Result<(), String> {
    // Arabic-only: ترقيم لغة برمجة عربية
    if verbose {
        eprintln!("{}", "جاري بدء خادم لغة ترقيم...".cyan().bold());
    }

    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| format!("فشل إنشاء وقت التشغيل: {}", e))?;

    runtime.block_on(async {
        crate::lsp::run_server()
            .await
            .map_err(|e| format!("خطأ خادم LSP: {}", e))
    })
}

fn doc_command(
    path: PathBuf,
    output: Option<PathBuf>,
    format: String,
    single_file: bool,
    verbose: bool,
    lang: Language,
) -> Result<(), String> {
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

    // Arabic-only: ترقيم لغة برمجة عربية
    if !single_file {
        fs::create_dir_all(&output_dir).map_err(|e| format!("لا يمكن إنشاء مجلد الإخراج: {}", e))?;
    }

    if verbose {
        println!(
            "{}",
            format!("جاري توليد التوثيق لـ {} ملف(ات)...", source_files.len()).cyan()
        );
    }

    // Create compiler context with string interner
    let mut _ctx = CompilerContext::new();

    let mut all_docs = Vec::new();
    for source_file in &source_files {
        let source = fs::read_to_string(source_file)
            .map_err(|e| format!("لا يمكن قراءة الملف {}: {}", source_file.display(), e))?;

        let filename = source_file.display().to_string();
        let module_name = source_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module")
            .to_string();

        let mut parser = Parser::new(&source);
        let ast = parser.parse().map_err(|e| {
            e.emit(&source, &filename, lang);
            format!("خطأ في تحليل {}", source_file.display())
        })?;

        let extractor = DocExtractor::new(module_name.clone(), filename);
        let doc = extractor.extract(&ast);

        if verbose {
            let item_count = doc.items.len();
            println!("  {} - {} عنصر", module_name, item_count);
        }

        all_docs.push((module_name, doc));
    }

    if single_file {
        generate_single_doc_file(&output_dir, &all_docs, output_format)?;
    } else {
        generate_multi_doc_files(&output_dir, &all_docs, output_format)?;
    }

    if verbose {
        println!("{}", "اكتمل توليد التوثيق!".green().bold());
    }

    Ok(())
}

fn generate_single_doc_file(
    output_dir: &Path,
    all_docs: &[(String, crate::doc::model::Documentation)],
    output_format: OutputFormat,
) -> Result<(), String> {
    use crate::doc::generator::DocGenerator;

    let output_file = if output_dir.is_dir() || !output_dir.exists() {
        let ext = match output_format {
            OutputFormat::Html => "html",
            OutputFormat::Markdown => "md",
            OutputFormat::Json => "json",
        };
        output_dir.join(format!("documentation.{}", ext))
    } else {
        output_dir.to_path_buf()
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
        .map_err(|e| format!("فشل توليد التوثيق: {}", e))?;

        if let Some(parent) = output_file.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("لا يمكن إنشاء مجلد الإخراج: {}", e))?;
        }

        fs::write(&output_file, output_buffer)
            .map_err(|e| format!("لا يمكن كتابة الإخراج: {}", e))?;

        // Arabic-only: ترقيم لغة برمجة عربية
        println!(
            "{}",
            format!("تم توليد التوثيق: {}", output_file.display()).green()
        );
    }

    Ok(())
}

fn generate_multi_doc_files(
    output_dir: &Path,
    all_docs: &[(String, crate::doc::model::Documentation)],
    output_format: OutputFormat,
) -> Result<(), String> {
    use crate::doc::generator::DocGenerator;

    for (module_name, doc) in all_docs {
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
        .map_err(|e| format!("فشل توليد التوثيق: {}", e))?;

        fs::write(&output_file, output_buffer)
            .map_err(|e| format!("لا يمكن كتابة الإخراج: {}", e))?;
    }

    if output_format == OutputFormat::Html && all_docs.len() > 1 {
        let index_content = generate_html_index(all_docs);
        let index_file = output_dir.join("index.html");
        fs::write(&index_file, index_content).map_err(|e| format!("لا يمكن كتابة الفهرس: {}", e))?;
    }

    // Arabic-only: ترقيم لغة برمجة عربية
    println!(
        "{}",
        format!("تم توليد التوثيق في: {}", output_dir.display()).green()
    );

    Ok(())
}

/// REPL input is a bare statement, but the parser requires every program
/// to open with بسم_الله and close with الحمد_لله. Wrap the input so
/// users can type plain statements interactively.
fn wrap_repl_input(input: &str) -> String {
    if input.trim_start().starts_with("بسم_الله") {
        input.to_string()
    } else {
        format!("بسم_الله\n{}\nالحمد_لله", input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_repl_input_makes_bare_statement_parseable() {
        let wrapped = wrap_repl_input("اطبع(٥)");
        let mut parser = Parser::new(&wrapped);
        assert!(
            parser.parse().is_ok(),
            "wrapped REPL input should parse as a full program"
        );
    }

    #[test]
    fn test_wrap_repl_input_keeps_explicit_markers() {
        let src = "بسم_الله\nاطبع(٥)\nالحمد_لله";
        assert_eq!(wrap_repl_input(src), src);
    }

    #[test]
    fn test_wrap_repl_input_variable_declaration() {
        let wrapped = wrap_repl_input("متغير س = ٥");
        let mut parser = Parser::new(&wrapped);
        assert!(parser.parse().is_ok());
    }

    /// Lower one REPL line exactly as `repl_command` does, against a scratch
    /// directory standing in for the working directory.
    ///
    /// The `TempDir` travels back in the tuple because dropping it deletes the
    /// module fixtures the linker is about to read.
    fn lower_repl_line(
        modules: &[(&str, &str)],
        line: &str,
    ) -> (Result<crate::ir::Module, String>, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();

        for (name, body) in modules {
            let wrapped = format!("بسم_الله\n{}\nالحمد_لله", body.trim());
            fs::write(dir.path().join(name), wrapped).unwrap();
        }

        let source = wrap_repl_input(line);
        let ast = Parser::new(&source).parse().expect("REPL line must parse");

        let mut warnings = Vec::new();
        let result = link_repl_line(
            &ast,
            dir.path().join(REPL_PSEUDO_FILE),
            false,
            &mut warnings,
        )
        .map_err(|diagnostics| {
            diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
                .join("; ")
        })
        .and_then(|linked| {
            IrBuilder::new("<repl:1>".to_string())
                .build_library(&linked)
                .map_err(|e| e.message)
        });

        (result, dir)
    }

    /// The REPL was the one pipeline entry point that never ran the linker, so
    /// an imported function reached the interpreter as a name with no body and
    /// calling it failed with `دالة غير معرّفة` (issue #182).
    #[test]
    fn test_repl_line_links_the_body_of_an_imported_function() {
        let (result, _dir) = lower_repl_line(
            &[(
                "أدوات.ترقيم",
                "صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد {\n أرجع أ + ب\n}",
            )],
            "استورد { جمع } من \"./أدوات\"؛ اطبع(جمع(2، 3))",
        );

        let module = result.expect("REPL line importing a module must lower");
        assert!(
            module
                .functions
                .iter()
                .any(|f| f.name.contains("جمع") || f.id.0.contains("جمع")),
            "the imported function's body must reach the IR, got {:?}",
            module.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
    }

    /// A REPL line that only declares is a library, not a program: `build`
    /// rejects it for having no entry point (ت٠٢٠٢), which turned a previously
    /// silent line into an error. It must lower — and then not be run, or the
    /// interpreter reports the entry point it cannot find instead.
    #[test]
    fn test_repl_line_declaring_only_a_variable_lowers_and_is_not_run() {
        let (result, _dir) = lower_repl_line(&[], "متغير س = ٥");

        let module = result.expect("a declaration-only REPL line must lower");
        assert!(
            !has_entry_point(&module),
            "a declaration is not a program and must not be executed"
        );
    }

    #[test]
    fn test_repl_line_with_top_level_code_still_has_an_entry_point() {
        let (result, _dir) = lower_repl_line(&[], "اطبع(٥)");

        let module = result.expect("an executable REPL line must lower");
        assert!(has_entry_point(&module), "top-level code must still be run");
    }
}
