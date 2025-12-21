//! Comprehensive tests for the CLI module
//!
//! These tests verify CLI argument parsing, command structure,
//! and command execution behavior.

use super::*;
use clap::Parser;
use std::path::PathBuf;

// =============================================================================
// CLI Argument Parsing Tests
// =============================================================================

#[test]
fn test_cli_parse_compile_basic() {
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.trq"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Compile { file, .. } => {
            assert_eq!(file, PathBuf::from("test.trq"));
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_parse_compile_with_output() {
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.trq", "-o", "output"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Compile { file, output, .. } => {
            assert_eq!(file, PathBuf::from("test.trq"));
            assert_eq!(output, Some(PathBuf::from("output")));
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_parse_compile_emit_llvm() {
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.trq", "--emit-llvm"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Compile { emit_llvm, .. } => {
            assert!(emit_llvm);
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_parse_compile_emit_asm() {
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.trq", "--emit-asm"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Compile { emit_asm, .. } => {
            assert!(emit_asm);
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_parse_compile_emit_obj() {
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.trq", "-c"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Compile { emit_obj, .. } => {
            assert!(emit_obj);
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_parse_compile_optimization_levels() {
    // Test O0
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.trq", "-O", "0"]);
    assert!(args.is_ok());
    match args.unwrap().command {
        Commands::Compile { opt_level, .. } => assert_eq!(opt_level, 0),
        _ => panic!("Expected Compile command"),
    }

    // Test O1
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.trq", "-O", "1"]);
    assert!(args.is_ok());
    match args.unwrap().command {
        Commands::Compile { opt_level, .. } => assert_eq!(opt_level, 1),
        _ => panic!("Expected Compile command"),
    }

    // Test O2
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.trq", "-O", "2"]);
    assert!(args.is_ok());
    match args.unwrap().command {
        Commands::Compile { opt_level, .. } => assert_eq!(opt_level, 2),
        _ => panic!("Expected Compile command"),
    }

    // Test O3
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.trq", "-O", "3"]);
    assert!(args.is_ok());
    match args.unwrap().command {
        Commands::Compile { opt_level, .. } => assert_eq!(opt_level, 3),
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_parse_compile_dump_flags() {
    let args = Cli::try_parse_from([
        "tarqeem",
        "compile",
        "test.trq",
        "--dump-tokens",
        "--dump-ast",
        "--dump-ir",
    ]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Compile {
            dump_tokens,
            dump_ast,
            dump_ir,
            ..
        } => {
            assert!(dump_tokens);
            assert!(dump_ast);
            assert!(dump_ir);
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_parse_compile_target() {
    let args = Cli::try_parse_from([
        "tarqeem",
        "compile",
        "test.trq",
        "--target",
        "x86_64-unknown-linux-gnu",
    ]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Compile { target, .. } => {
            assert_eq!(target, Some("x86_64-unknown-linux-gnu".to_string()));
        }
        _ => panic!("Expected Compile command"),
    }
}

// =============================================================================
// Run Command Tests
// =============================================================================

#[test]
fn test_cli_parse_run_basic() {
    let args = Cli::try_parse_from(["tarqeem", "run", "test.trq"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Run { file } => {
            assert_eq!(file, PathBuf::from("test.trq"));
        }
        _ => panic!("Expected Run command"),
    }
}

#[test]
fn test_cli_parse_run_arabic_alias() {
    let args = Cli::try_parse_from(["tarqeem", "شغل", "test.trq"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Run { file } => {
            assert_eq!(file, PathBuf::from("test.trq"));
        }
        _ => panic!("Expected Run command"),
    }
}

// =============================================================================
// Check Command Tests
// =============================================================================

#[test]
fn test_cli_parse_check_basic() {
    let args = Cli::try_parse_from(["tarqeem", "check", "test.trq"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Check { file } => {
            assert_eq!(file, PathBuf::from("test.trq"));
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn test_cli_parse_check_arabic_alias() {
    let args = Cli::try_parse_from(["tarqeem", "افحص", "test.trq"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Check { file } => {
            assert_eq!(file, PathBuf::from("test.trq"));
        }
        _ => panic!("Expected Check command"),
    }
}

// =============================================================================
// REPL Command Tests
// =============================================================================

#[test]
fn test_cli_parse_repl() {
    let args = Cli::try_parse_from(["tarqeem", "repl"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(matches!(cli.command, Commands::Repl));
}

#[test]
fn test_cli_parse_repl_arabic_alias() {
    let args = Cli::try_parse_from(["tarqeem", "تفاعلي"]);
    assert!(args.is_ok());

    assert!(matches!(args.unwrap().command, Commands::Repl));
}

// =============================================================================
// Fmt Command Tests
// =============================================================================

#[test]
fn test_cli_parse_fmt_basic() {
    let args = Cli::try_parse_from(["tarqeem", "fmt", "test.trq"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Fmt { path, .. } => {
            assert_eq!(path, Some(PathBuf::from("test.trq")));
        }
        _ => panic!("Expected Fmt command"),
    }
}

#[test]
fn test_cli_parse_fmt_write() {
    let args = Cli::try_parse_from(["tarqeem", "fmt", "test.trq", "-w"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Fmt { write, .. } => {
            assert!(write);
        }
        _ => panic!("Expected Fmt command"),
    }
}

#[test]
fn test_cli_parse_fmt_check() {
    let args = Cli::try_parse_from(["tarqeem", "fmt", "test.trq", "--check"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Fmt { check, .. } => {
            assert!(check);
        }
        _ => panic!("Expected Fmt command"),
    }
}

#[test]
fn test_cli_parse_fmt_diff() {
    let args = Cli::try_parse_from(["tarqeem", "fmt", "test.trq", "--diff"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Fmt { diff, .. } => {
            assert!(diff);
        }
        _ => panic!("Expected Fmt command"),
    }
}

#[test]
fn test_cli_parse_fmt_config() {
    let args = Cli::try_parse_from(["tarqeem", "fmt", "test.trq", "-c", "config.toml"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Fmt { config, .. } => {
            assert_eq!(config, Some(PathBuf::from("config.toml")));
        }
        _ => panic!("Expected Fmt command"),
    }
}

#[test]
fn test_cli_parse_fmt_sample_config() {
    let args = Cli::try_parse_from(["tarqeem", "fmt", "--sample-config"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Fmt { sample_config, .. } => {
            assert!(sample_config);
        }
        _ => panic!("Expected Fmt command"),
    }
}

// =============================================================================
// Lex Command Tests
// =============================================================================

#[test]
fn test_cli_parse_lex() {
    let args = Cli::try_parse_from(["tarqeem", "lex", "test.trq"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Lex { file } => {
            assert_eq!(file, PathBuf::from("test.trq"));
        }
        _ => panic!("Expected Lex command"),
    }
}

// =============================================================================
// Parse Command Tests
// =============================================================================

#[test]
fn test_cli_parse_parse() {
    let args = Cli::try_parse_from(["tarqeem", "parse", "test.trq"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Parse { file } => {
            assert_eq!(file, PathBuf::from("test.trq"));
        }
        _ => panic!("Expected Parse command"),
    }
}

// =============================================================================
// Package Manager Command Tests
// =============================================================================

#[test]
fn test_cli_parse_pkg_init() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "init"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Init { name, lib },
        } => {
            assert!(name.is_none());
            assert!(!lib);
        }
        _ => panic!("Expected Pkg Init command"),
    }
}

#[test]
fn test_cli_parse_pkg_init_with_name() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "init", "my-project"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Init { name, .. },
        } => {
            assert_eq!(name, Some("my-project".to_string()));
        }
        _ => panic!("Expected Pkg Init command"),
    }
}

#[test]
fn test_cli_parse_pkg_init_lib() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "init", "-l"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Init { lib, .. },
        } => {
            assert!(lib);
        }
        _ => panic!("Expected Pkg Init command"),
    }
}

#[test]
fn test_cli_parse_pkg_add() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "add", "json"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Add { package, dev, path },
        } => {
            assert_eq!(package, "json");
            assert!(!dev);
            assert!(path.is_none());
        }
        _ => panic!("Expected Pkg Add command"),
    }
}

#[test]
fn test_cli_parse_pkg_add_dev() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "add", "test-lib", "-d"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Add { dev, .. },
        } => {
            assert!(dev);
        }
        _ => panic!("Expected Pkg Add command"),
    }
}

#[test]
fn test_cli_parse_pkg_add_path() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "add", "local-lib", "-p", "../lib"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Add { path, .. },
        } => {
            assert_eq!(path, Some(PathBuf::from("../lib")));
        }
        _ => panic!("Expected Pkg Add command"),
    }
}

#[test]
fn test_cli_parse_pkg_remove() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "remove", "json"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Remove { package },
        } => {
            assert_eq!(package, "json");
        }
        _ => panic!("Expected Pkg Remove command"),
    }
}

#[test]
fn test_cli_parse_pkg_install() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "install"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Install { force },
        } => {
            assert!(!force);
        }
        _ => panic!("Expected Pkg Install command"),
    }
}

#[test]
fn test_cli_parse_pkg_install_force() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "install", "-f"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Install { force },
        } => {
            assert!(force);
        }
        _ => panic!("Expected Pkg Install command"),
    }
}

#[test]
fn test_cli_parse_pkg_update() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "update"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Update { package },
        } => {
            assert!(package.is_none());
        }
        _ => panic!("Expected Pkg Update command"),
    }
}

#[test]
fn test_cli_parse_pkg_update_specific() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "update", "json"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Update { package },
        } => {
            assert_eq!(package, Some("json".to_string()));
        }
        _ => panic!("Expected Pkg Update command"),
    }
}

#[test]
fn test_cli_parse_pkg_build() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "build"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Build { release },
        } => {
            assert!(!release);
        }
        _ => panic!("Expected Pkg Build command"),
    }
}

#[test]
fn test_cli_parse_pkg_build_release() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "build", "-r"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Build { release },
        } => {
            assert!(release);
        }
        _ => panic!("Expected Pkg Build command"),
    }
}

#[test]
fn test_cli_parse_pkg_run() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "run"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Run { args },
        } => {
            assert!(args.is_empty());
        }
        _ => panic!("Expected Pkg Run command"),
    }
}

#[test]
fn test_cli_parse_pkg_run_with_args() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "run", "--", "arg1", "arg2"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Run { args },
        } => {
            assert_eq!(args, vec!["arg1", "arg2"]);
        }
        _ => panic!("Expected Pkg Run command"),
    }
}

#[test]
fn test_cli_parse_pkg_test() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "test"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Test { filter },
        } => {
            assert!(filter.is_none());
        }
        _ => panic!("Expected Pkg Test command"),
    }
}

#[test]
fn test_cli_parse_pkg_test_filter() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "test", "test_add"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Test { filter },
        } => {
            assert_eq!(filter, Some("test_add".to_string()));
        }
        _ => panic!("Expected Pkg Test command"),
    }
}

#[test]
fn test_cli_parse_pkg_info() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "info"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Info,
        } => {}
        _ => panic!("Expected Pkg Info command"),
    }
}

#[test]
fn test_cli_parse_pkg_clean() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "clean"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Pkg {
            command: PkgCommands::Clean,
        } => {}
        _ => panic!("Expected Pkg Clean command"),
    }
}

// =============================================================================
// LSP Command Tests
// =============================================================================

#[test]
fn test_cli_parse_lsp() {
    let args = Cli::try_parse_from(["tarqeem", "lsp"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(matches!(cli.command, Commands::Lsp));
}

// =============================================================================
// Doc Command Tests
// =============================================================================

#[test]
fn test_cli_parse_doc_basic() {
    let args = Cli::try_parse_from(["tarqeem", "doc", "src/"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Doc {
            path,
            output,
            format,
            single_file,
        } => {
            assert_eq!(path, PathBuf::from("src/"));
            assert!(output.is_none());
            assert_eq!(format, "html");
            assert!(!single_file);
        }
        _ => panic!("Expected Doc command"),
    }
}

#[test]
fn test_cli_parse_doc_with_output() {
    let args = Cli::try_parse_from(["tarqeem", "doc", "src/", "-o", "docs/"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Doc { output, .. } => {
            assert_eq!(output, Some(PathBuf::from("docs/")));
        }
        _ => panic!("Expected Doc command"),
    }
}

#[test]
fn test_cli_parse_doc_format_markdown() {
    let args = Cli::try_parse_from(["tarqeem", "doc", "src/", "-f", "markdown"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Doc { format, .. } => {
            assert_eq!(format, "markdown");
        }
        _ => panic!("Expected Doc command"),
    }
}

#[test]
fn test_cli_parse_doc_format_json() {
    let args = Cli::try_parse_from(["tarqeem", "doc", "src/", "-f", "json"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Doc { format, .. } => {
            assert_eq!(format, "json");
        }
        _ => panic!("Expected Doc command"),
    }
}

#[test]
fn test_cli_parse_doc_single_file() {
    let args = Cli::try_parse_from(["tarqeem", "doc", "src/", "--single-file"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Doc { single_file, .. } => {
            assert!(single_file);
        }
        _ => panic!("Expected Doc command"),
    }
}

// =============================================================================
// Global Flag Tests
// =============================================================================

#[test]
fn test_cli_english_flag() {
    let args = Cli::try_parse_from(["tarqeem", "-e", "compile", "test.trq"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(cli.english);
}

#[test]
fn test_cli_english_flag_long() {
    let args = Cli::try_parse_from(["tarqeem", "--english", "compile", "test.trq"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(cli.english);
}

#[test]
fn test_cli_verbose_flag() {
    let args = Cli::try_parse_from(["tarqeem", "-v", "compile", "test.trq"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(cli.verbose);
}

#[test]
fn test_cli_verbose_flag_long() {
    let args = Cli::try_parse_from(["tarqeem", "--verbose", "compile", "test.trq"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(cli.verbose);
}

#[test]
fn test_cli_multiple_global_flags() {
    let args = Cli::try_parse_from(["tarqeem", "-e", "-v", "compile", "test.trq"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(cli.english);
    assert!(cli.verbose);
}

// =============================================================================
// Alias Tests
// =============================================================================

#[test]
fn test_cli_compile_alias_c() {
    let args = Cli::try_parse_from(["tarqeem", "c", "test.trq"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Compile { file, .. } => {
            assert_eq!(file, PathBuf::from("test.trq"));
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_compile_alias_arabic() {
    let args = Cli::try_parse_from(["tarqeem", "ترجم", "test.trq"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Compile { file, .. } => {
            assert_eq!(file, PathBuf::from("test.trq"));
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_run_alias_r() {
    let args = Cli::try_parse_from(["tarqeem", "r", "test.trq"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Run { file } => {
            assert_eq!(file, PathBuf::from("test.trq"));
        }
        _ => panic!("Expected Run command"),
    }
}

#[test]
fn test_cli_check_alias_ch() {
    let args = Cli::try_parse_from(["tarqeem", "ch", "test.trq"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Check { file } => {
            assert_eq!(file, PathBuf::from("test.trq"));
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn test_cli_fmt_alias_f() {
    let args = Cli::try_parse_from(["tarqeem", "f", "test.trq"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Fmt { path, .. } => {
            assert_eq!(path, Some(PathBuf::from("test.trq")));
        }
        _ => panic!("Expected Fmt command"),
    }
}

#[test]
fn test_cli_pkg_alias_pm() {
    let args = Cli::try_parse_from(["tarqeem", "pm", "init"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Pkg {
            command: PkgCommands::Init { .. },
        } => {}
        _ => panic!("Expected Pkg Init command"),
    }
}

// =============================================================================
// Error Cases
// =============================================================================

#[test]
fn test_cli_missing_command() {
    let args = Cli::try_parse_from(["tarqeem"]);
    assert!(args.is_err());
}

#[test]
fn test_cli_unknown_command() {
    let args = Cli::try_parse_from(["tarqeem", "unknown"]);
    assert!(args.is_err());
}

#[test]
fn test_cli_compile_missing_file() {
    let args = Cli::try_parse_from(["tarqeem", "compile"]);
    assert!(args.is_err());
}

#[test]
fn test_cli_run_missing_file() {
    let args = Cli::try_parse_from(["tarqeem", "run"]);
    assert!(args.is_err());
}

#[test]
fn test_cli_check_missing_file() {
    let args = Cli::try_parse_from(["tarqeem", "check"]);
    assert!(args.is_err());
}

#[test]
fn test_cli_pkg_missing_subcommand() {
    let args = Cli::try_parse_from(["tarqeem", "pkg"]);
    assert!(args.is_err());
}

#[test]
fn test_cli_pkg_add_missing_package() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "add"]);
    assert!(args.is_err());
}

#[test]
fn test_cli_pkg_remove_missing_package() {
    let args = Cli::try_parse_from(["tarqeem", "pkg", "remove"]);
    assert!(args.is_err());
}

// =============================================================================
// Arabic File Extension Tests
// =============================================================================

#[test]
fn test_cli_parse_arabic_extension() {
    let args = Cli::try_parse_from(["tarqeem", "compile", "برنامج.ترقيم"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Compile { file, .. } => {
            assert_eq!(file, PathBuf::from("برنامج.ترقيم"));
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_run_arabic_file() {
    let args = Cli::try_parse_from(["tarqeem", "run", "مرحبا.ترقيم"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Run { file } => {
            assert_eq!(file, PathBuf::from("مرحبا.ترقيم"));
        }
        _ => panic!("Expected Run command"),
    }
}

// =============================================================================
// Complex Argument Combination Tests
// =============================================================================

#[test]
fn test_cli_compile_full_options() {
    let args = Cli::try_parse_from([
        "tarqeem",
        "--verbose",
        "--english",
        "compile",
        "program.trq",
        "-o",
        "output",
        "-O",
        "2",
        "--target",
        "aarch64-apple-darwin",
        "--dump-ir",
    ]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(cli.verbose);
    assert!(cli.english);

    match cli.command {
        Commands::Compile {
            file,
            output,
            opt_level,
            target,
            dump_ir,
            ..
        } => {
            assert_eq!(file, PathBuf::from("program.trq"));
            assert_eq!(output, Some(PathBuf::from("output")));
            assert_eq!(opt_level, 2);
            assert_eq!(target, Some("aarch64-apple-darwin".to_string()));
            assert!(dump_ir);
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_doc_full_options() {
    let args = Cli::try_parse_from([
        "tarqeem",
        "doc",
        "src/",
        "-o",
        "documentation/",
        "-f",
        "markdown",
        "--single-file",
    ]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Doc {
            path,
            output,
            format,
            single_file,
        } => {
            assert_eq!(path, PathBuf::from("src/"));
            assert_eq!(output, Some(PathBuf::from("documentation/")));
            assert_eq!(format, "markdown");
            assert!(single_file);
        }
        _ => panic!("Expected Doc command"),
    }
}
