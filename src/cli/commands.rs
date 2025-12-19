//! CLI command implementations

use super::{Cli, Commands};
use crate::error::Language;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::semantic::Analyzer;
use colored::Colorize;
use std::fs;
use std::io::{self, BufRead, Write};

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
            dump_tokens,
            dump_ast,
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

            // TODO: Code generation
            if cli.verbose {
                println!("{}", "Compilation successful! / تمت الترجمة بنجاح!".green().bold());
            }

            if let Some(output_path) = output {
                // For now, just write a placeholder
                fs::write(&output_path, "# Compiled Tarqeem program\n")
                    .map_err(|e| format!("Could not write output: {}", e))?;
                println!(
                    "Output written to: {} / تم الكتابة إلى: {}",
                    output_path.display(),
                    output_path.display()
                );
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
