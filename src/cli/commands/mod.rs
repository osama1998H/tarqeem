//! CLI command implementations

mod compile;
mod debug;

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
    // Build search paths, filtering out None values
    let mut search_paths: Vec<PathBuf> = Vec::new();

    // Relative to executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            search_paths.push(parent.join("runtime/libtrq.a"));
            search_paths.push(parent.join("../runtime/libtrq.a"));
        }
    }

    // Relative to current directory
    search_paths.push(PathBuf::from("runtime/libtrq.a"));

    // User-local path (~/.tarqeem/lib/libtrq.a)
    if let Some(home) = dirs::home_dir() {
        search_paths.push(home.join(".tarqeem/lib/libtrq.a"));
    }

    // System paths
    search_paths.push(PathBuf::from("/usr/local/lib/tarqeem/libtrq.a"));
    search_paths.push(PathBuf::from("/usr/lib/tarqeem/libtrq.a"));

    for path in &search_paths {
        if path.exists() {
            return Some(path.clone());
        }
    }

    None
}

/// Find the WebAssembly runtime library
pub(super) fn find_wasm_runtime() -> Option<PathBuf> {
    let search_paths: Vec<Option<PathBuf>> = vec![
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("runtime/wasm/libtrq_wasm.a"))),
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .and_then(|p| p.parent().map(|p| p.join("runtime/wasm/libtrq_wasm.a")))
        }),
        Some(PathBuf::from("runtime/wasm/libtrq_wasm.a")),
        Some(PathBuf::from("/usr/local/lib/tarqeem/libtrq_wasm.a")),
        Some(PathBuf::from("/usr/lib/tarqeem/libtrq_wasm.a")),
    ];

    search_paths
        .into_iter()
        .flatten()
        .find(|path| path.exists())
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

    search_paths
        .into_iter()
        .flatten()
        .find(|path| path.exists() && path.is_dir())
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

        Commands::Run { file, jit } => run_command(file, jit, cli.verbose, lang),

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
    }
}

// ============================================================================
// Individual Command Implementations
// ============================================================================

fn run_command(file: PathBuf, jit: bool, verbose: bool, lang: Language) -> Result<(), String> {
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

    let mut analyzer = Analyzer::new();
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

    let ir_builder = IrBuilder::new(filename.clone());
    let ir_module = ir_builder.build(&ast).map_err(|e| {
        format!(
            "IR build error: {} / خطأ بناء التمثيل الوسيط: {}",
            e.message, e.message_ar
        )
    })?;

    if jit {
        // Use JIT compilation
        if verbose {
            println!(
                "{}",
                "Using JIT compilation / استخدام الترجمة الفورية".cyan()
            );
        }

        let config = JitConfig::default().with_verbose(verbose);
        let mut executor = JitExecutor::new(ir_module, config);

        match executor.run() {
            Ok(_result) => {
                if verbose {
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

    let mut analyzer = Analyzer::new();
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

    println!(
        "{}",
        "No errors found! / لم يتم العثور على أخطاء!".green().bold()
    );

    Ok(())
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

                let mut parser = Parser::new(trimmed);
                match parser.parse() {
                    Ok(ast) => {
                        let mut analyzer = Analyzer::new();
                        configure_analyzer(&mut analyzer, verbose);
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
                                            if verbose {
                                                println!(
                                                    "{} {}",
                                                    "=>".cyan(),
                                                    format!("{}", result).yellow()
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("{} {}", "Runtime error:".red().bold(), e);
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

                let diff_output =
                    fmt::show_diff(&source, &format_config).map_err(|e| format!("{}", e))?;
                println!("{}", diff_output);
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
