//! Comprehensive tests for the CLI module
//!
//! These tests verify CLI argument parsing, command structure,
//! and command execution behavior.

use super::*;
use clap::Parser;
use std::path::PathBuf;

#[test]
fn test_cli_parse_compile_basic() {
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.ترقيم"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Compile { file, .. } => {
            assert_eq!(file, PathBuf::from("test.ترقيم"));
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_parse_compile_with_output() {
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.ترقيم", "-o", "output"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Compile { file, output, .. } => {
            assert_eq!(file, PathBuf::from("test.ترقيم"));
            assert_eq!(output, Some(PathBuf::from("output")));
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_parse_compile_emit_llvm() {
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.ترقيم", "--emit-llvm"]);
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
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.ترقيم", "--emit-asm"]);
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
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.ترقيم", "-c"]);
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
    let args = Cli::try_parse_from(["tarqeem", "compile", "test.ترقيم", "-O", "0"]);
    assert!(args.is_ok());
    match args.unwrap().command {
        Commands::Compile { opt_level, .. } => assert_eq!(opt_level, 0),
        _ => panic!("Expected Compile command"),
    }

    let args = Cli::try_parse_from(["tarqeem", "compile", "test.ترقيم", "-O", "1"]);
    assert!(args.is_ok());
    match args.unwrap().command {
        Commands::Compile { opt_level, .. } => assert_eq!(opt_level, 1),
        _ => panic!("Expected Compile command"),
    }

    let args = Cli::try_parse_from(["tarqeem", "compile", "test.ترقيم", "-O", "2"]);
    assert!(args.is_ok());
    match args.unwrap().command {
        Commands::Compile { opt_level, .. } => assert_eq!(opt_level, 2),
        _ => panic!("Expected Compile command"),
    }

    let args = Cli::try_parse_from(["tarqeem", "compile", "test.ترقيم", "-O", "3"]);
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
        "test.ترقيم",
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
        "test.ترقيم",
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

#[test]
fn test_cli_parse_run_basic() {
    let args = Cli::try_parse_from(["tarqeem", "run", "test.ترقيم"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Run { file, .. } => {
            assert_eq!(file, PathBuf::from("test.ترقيم"));
        }
        _ => panic!("Expected Run command"),
    }
}

#[test]
fn test_cli_parse_run_arabic_alias() {
    let args = Cli::try_parse_from(["tarqeem", "شغل", "test.ترقيم"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Run { file, .. } => {
            assert_eq!(file, PathBuf::from("test.ترقيم"));
        }
        _ => panic!("Expected Run command"),
    }
}

#[test]
fn test_cli_parse_check_basic() {
    let args = Cli::try_parse_from(["tarqeem", "check", "test.ترقيم"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Check { file } => {
            assert_eq!(file, PathBuf::from("test.ترقيم"));
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn test_cli_parse_check_arabic_alias() {
    let args = Cli::try_parse_from(["tarqeem", "افحص", "test.ترقيم"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Check { file } => {
            assert_eq!(file, PathBuf::from("test.ترقيم"));
        }
        _ => panic!("Expected Check command"),
    }
}

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

#[test]
fn test_cli_parse_fmt_basic() {
    let args = Cli::try_parse_from(["tarqeem", "fmt", "test.ترقيم"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Fmt { path, .. } => {
            assert_eq!(path, Some(PathBuf::from("test.ترقيم")));
        }
        _ => panic!("Expected Fmt command"),
    }
}

#[test]
fn test_cli_parse_fmt_write() {
    let args = Cli::try_parse_from(["tarqeem", "fmt", "test.ترقيم", "-w"]);
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
    let args = Cli::try_parse_from(["tarqeem", "fmt", "test.ترقيم", "--check"]);
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
    let args = Cli::try_parse_from(["tarqeem", "fmt", "test.ترقيم", "--diff"]);
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
    let args = Cli::try_parse_from(["tarqeem", "fmt", "test.ترقيم", "-c", "config.toml"]);
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

#[test]
fn test_cli_parse_lex() {
    let args = Cli::try_parse_from(["tarqeem", "lex", "test.ترقيم"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Lex { file } => {
            assert_eq!(file, PathBuf::from("test.ترقيم"));
        }
        _ => panic!("Expected Lex command"),
    }
}

#[test]
fn test_cli_parse_parse() {
    let args = Cli::try_parse_from(["tarqeem", "parse", "test.ترقيم"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Parse { file, format } => {
            assert_eq!(file, PathBuf::from("test.ترقيم"));
            assert_eq!(format, "debug"); // Default format
        }
        _ => panic!("Expected Parse command"),
    }
}

#[test]
fn test_cli_parse_parse_json() {
    let args = Cli::try_parse_from(["tarqeem", "parse", "test.ترقيم", "--format", "json"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Parse { file, format } => {
            assert_eq!(file, PathBuf::from("test.ترقيم"));
            assert_eq!(format, "json");
        }
        _ => panic!("Expected Parse command"),
    }
}

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

#[test]
fn test_cli_parse_lsp() {
    let args = Cli::try_parse_from(["tarqeem", "lsp"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(matches!(cli.command, Commands::Lsp));
}

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

#[test]
fn test_cli_english_flag() {
    let args = Cli::try_parse_from(["tarqeem", "-e", "compile", "test.ترقيم"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(cli.english);
}

#[test]
fn test_cli_english_flag_long() {
    let args = Cli::try_parse_from(["tarqeem", "--english", "compile", "test.ترقيم"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(cli.english);
}

#[test]
fn test_cli_verbose_flag() {
    let args = Cli::try_parse_from(["tarqeem", "-v", "compile", "test.ترقيم"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(cli.verbose);
}

#[test]
fn test_cli_verbose_flag_long() {
    let args = Cli::try_parse_from(["tarqeem", "--verbose", "compile", "test.ترقيم"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(cli.verbose);
}

#[test]
fn test_cli_multiple_global_flags() {
    let args = Cli::try_parse_from(["tarqeem", "-e", "-v", "compile", "test.ترقيم"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    assert!(cli.english);
    assert!(cli.verbose);
}

#[test]
fn test_cli_compile_alias_c() {
    let args = Cli::try_parse_from(["tarqeem", "c", "test.ترقيم"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Compile { file, .. } => {
            assert_eq!(file, PathBuf::from("test.ترقيم"));
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_compile_alias_arabic() {
    let args = Cli::try_parse_from(["tarqeem", "ترجم", "test.ترقيم"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Compile { file, .. } => {
            assert_eq!(file, PathBuf::from("test.ترقيم"));
        }
        _ => panic!("Expected Compile command"),
    }
}

#[test]
fn test_cli_run_alias_r() {
    let args = Cli::try_parse_from(["tarqeem", "r", "test.ترقيم"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Run { file, .. } => {
            assert_eq!(file, PathBuf::from("test.ترقيم"));
        }
        _ => panic!("Expected Run command"),
    }
}

#[test]
fn test_cli_check_alias_ch() {
    let args = Cli::try_parse_from(["tarqeem", "ch", "test.ترقيم"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Check { file } => {
            assert_eq!(file, PathBuf::from("test.ترقيم"));
        }
        _ => panic!("Expected Check command"),
    }
}

#[test]
fn test_cli_fmt_alias_f() {
    let args = Cli::try_parse_from(["tarqeem", "f", "test.ترقيم"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Fmt { path, .. } => {
            assert_eq!(path, Some(PathBuf::from("test.ترقيم")));
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
        Commands::Run { file, .. } => {
            assert_eq!(file, PathBuf::from("مرحبا.ترقيم"));
        }
        _ => panic!("Expected Run command"),
    }
}

#[test]
fn test_cli_compile_full_options() {
    let args = Cli::try_parse_from([
        "tarqeem",
        "--verbose",
        "--english",
        "compile",
        "program.ترقيم",
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
            assert_eq!(file, PathBuf::from("program.ترقيم"));
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

use crate::ir::IrBuilder;
use crate::lexer::Lexer;
use crate::parser::Parser as TarqeemParser;
use crate::semantic::Analyzer;

fn wrap_source(source: &str) -> String {
    format!("بسم_الله\n{}\nالحمد_لله", source.trim())
}

fn check_source(source: &str) -> Result<(), Vec<String>> {
    let wrapped = wrap_source(source);
    let mut parser = TarqeemParser::new(&wrapped);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(e) => return Err(vec![e.message]),
    };

    let mut analyzer = Analyzer::new();
    if let Err(errors) = analyzer.analyze(&ast) {
        return Err(errors.iter().map(|e| e.message.clone()).collect());
    }

    Ok(())
}

fn compile_to_ir(source: &str) -> Result<crate::ir::Module, String> {
    let wrapped = wrap_source(source);
    let mut parser = TarqeemParser::new(&wrapped);
    let ast = parser.parse().map_err(|e| e.message)?;

    let mut analyzer = Analyzer::new();
    analyzer.analyze(&ast).map_err(|errors| {
        errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join("; ")
    })?;

    let ir_builder = IrBuilder::new("test_module".to_string());
    let module = ir_builder.build(&ast).map_err(|e| e.to_string())?;
    Ok(module)
}

#[test]
fn test_check_valid_hello_world() {
    let source = r#"
        اطبع("مرحبا بالعالم")
    "#;

    let result = check_source(source);
    assert!(result.is_ok(), "Hello world should be valid: {:?}", result);
}

#[test]
fn test_check_valid_variable_declaration() {
    let source = r#"
        متغير س = 5
        متغير ص: عدد = 10
        ثابت ز = 15
    "#;

    let result = check_source(source);
    assert!(
        result.is_ok(),
        "Variable declarations should be valid: {:?}",
        result
    );
}

#[test]
fn test_check_valid_function() {
    let source = r#"
        دالة جمع(أ: عدد، ب: عدد) -> عدد {
            أرجع أ + ب
        }
    "#;

    let result = check_source(source);
    assert!(result.is_ok(), "Function should be valid: {:?}", result);
}

#[test]
fn test_check_valid_class() {
    let source = r#"
        صنف شخص {
            خاص اسم: نص

            منشئ(اسم: نص) {
                هذا.اسم = اسم
            }

            عام دالة اطبع_اسم() {
                اطبع(هذا.اسم)
            }
        }
    "#;

    let result = check_source(source);
    assert!(result.is_ok(), "Class should be valid: {:?}", result);
}

#[test]
fn test_check_syntax_error_missing_variable_name() {
    let source = r#"
        متغير = 5
    "#;

    let result = check_source(source);
    assert!(result.is_err(), "Missing variable name should be an error");
}

#[test]
fn test_check_semantic_error_type_mismatch() {
    let source = r#"
        متغير س: عدد = "نص"
    "#;

    let result = check_source(source);
    assert!(result.is_err(), "Type mismatch should be an error");
}

#[test]
fn test_check_semantic_error_undefined_variable() {
    let source = r#"
        اطبع(غير_موجود)
    "#;

    let result = check_source(source);
    assert!(result.is_err(), "Undefined variable should be an error");
}

#[test]
fn test_compile_generates_ir() {
    let source = r#"
        متغير س = 42
        اطبع(س)
    "#;

    let result = compile_to_ir(source);
    assert!(result.is_ok(), "Should generate IR: {:?}", result);

    let module = result.unwrap();
    assert!(!module.functions.is_empty(), "Module should have functions");
}

#[test]
fn test_compile_function_ir() {
    let source = r#"
        دالة مضاعفة(س: عدد) -> عدد {
            أرجع س * 2
        }
    "#;

    let result = compile_to_ir(source);
    assert!(
        result.is_ok(),
        "Should generate IR for function: {:?}",
        result
    );

    let module = result.unwrap();
    let has_func = module
        .functions
        .iter()
        .any(|f| f.name.contains("مضاعفة") || f.id.0.contains("مضاعفة"));
    assert!(has_func, "Module should have the مضاعفة function");
}

#[test]
fn test_parser_error_recovery_collects_multiple_errors() {
    let source = r#"
        متغير = 1
        متغير ص = 2
        ثابت = 3
        ثابت ع = 4
    "#;

    let wrapped = wrap_source(source);
    let mut parser = TarqeemParser::new(&wrapped);
    let _ = parser.parse();

    let errors = parser.get_errors();
    assert!(
        errors.len() >= 2,
        "Parser should collect multiple errors, got {}",
        errors.len()
    );
}

#[test]
fn test_error_messages_are_bilingual() {
    let source = r#"
        متغير = 5
    "#;

    let wrapped = wrap_source(source);
    let mut parser = TarqeemParser::new(&wrapped);
    let result = parser.parse();

    assert!(result.is_err());
    let err = result.unwrap_err();

    assert!(
        !err.message.is_empty(),
        "English message should not be empty"
    );
    assert!(
        !err.message_ar.is_empty(),
        "Arabic message should not be empty"
    );
}

#[test]
fn test_lexer_tokenizes_arabic_keywords() {
    let source = "متغير س = 5";
    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();

    assert!(!tokens.is_empty());
    assert!(
        matches!(tokens[0].kind, crate::lexer::TokenKind::Let),
        "First token should be Let (متغير)"
    );
}

#[test]
fn test_lexer_tokenizes_arabic_identifiers() {
    let source = "اسم_المستخدم";
    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();

    assert!(!tokens.is_empty());
    match &tokens[0].kind {
        crate::lexer::TokenKind::Identifier(name) => {
            assert_eq!(name, "اسم_المستخدم");
        }
        _ => panic!("Expected Identifier token"),
    }
}

// ============================================================================
// Explain Command Tests
// ============================================================================

#[test]
fn test_cli_parse_explain_basic() {
    let args = Cli::try_parse_from(["tarqeem", "explain", "د٠٣٠١"]);
    assert!(args.is_ok());

    let cli = args.unwrap();
    match cli.command {
        Commands::Explain { code } => {
            assert_eq!(code, "د٠٣٠١");
        }
        _ => panic!("Expected Explain command"),
    }
}

#[test]
fn test_cli_parse_explain_arabic_alias() {
    let args = Cli::try_parse_from(["tarqeem", "اشرح", "د٠٣٠١"]);
    assert!(args.is_ok());

    match args.unwrap().command {
        Commands::Explain { code } => {
            assert_eq!(code, "د٠٣٠١");
        }
        _ => panic!("Expected Explain command"),
    }
}

#[test]
fn test_cli_explain_missing_code() {
    let args = Cli::try_parse_from(["tarqeem", "explain"]);
    assert!(args.is_err());
}

#[test]
fn test_cli_parse_explain_various_categories() {
    // Test different error code categories
    let categories = [
        "ق٠٠٠١",
        "ب٠٠٠١",
        "د٠٠٠١",
        "ن٠٠٠١",
        "ص٠٠٠١",
        "و٠٠٠١",
        "ت٠٠٠١",
        "ح٠٠٠١",
        "م٠٠٠١",
    ];

    for code in categories {
        let args = Cli::try_parse_from(["tarqeem", "explain", code]);
        assert!(
            args.is_ok(),
            "Should parse explain command for code: {}",
            code
        );

        match args.unwrap().command {
            Commands::Explain { code: parsed_code } => {
                assert_eq!(parsed_code, code);
            }
            _ => panic!("Expected Explain command for code: {}", code),
        }
    }
}
