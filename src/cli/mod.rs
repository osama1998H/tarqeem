//! Command Line Interface for Tarqeem

mod commands;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub use commands::run;

/// Tarqeem - The Arabic Programming Language Compiler
#[derive(Parser, Debug)]
#[command(name = "tarqeem")]
#[command(author = "Tarqeem Contributors")]
#[command(version = "0.1.0")]
#[command(about = "ترقيم - أول لغة برمجة عربية مُترجَمة")]
#[command(long_about = "Tarqeem (ترقيم) is a compiled, general-purpose programming language with full Arabic syntax support.")]
pub struct Cli {
    /// Use English for error messages (default is Arabic)
    #[arg(long, short = 'e', global = true)]
    pub english: bool,

    /// Verbose output
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Compile a Tarqeem source file / ترجمة ملف ترقيم
    #[command(aliases = ["ترجم", "c"])]
    Compile {
        /// Source file to compile
        file: PathBuf,

        /// Output file path
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,

        /// Dump tokens (for debugging)
        #[arg(long)]
        dump_tokens: bool,

        /// Dump AST (for debugging)
        #[arg(long)]
        dump_ast: bool,

        /// Dump IR (for debugging)
        #[arg(long)]
        dump_ir: bool,
    },

    /// Run a Tarqeem source file / تشغيل ملف ترقيم
    #[command(aliases = ["شغل", "r"])]
    Run {
        /// Source file to run
        file: PathBuf,
    },

    /// Check a Tarqeem source file for errors / فحص ملف ترقيم
    #[command(aliases = ["افحص", "ch"])]
    Check {
        /// Source file to check
        file: PathBuf,
    },

    /// Start the interactive REPL / بدء الوضع التفاعلي
    #[command(aliases = ["تفاعلي"])]
    Repl,

    /// Format a Tarqeem source file / تنسيق ملف ترقيم
    #[command(aliases = ["نسق", "f"])]
    Fmt {
        /// Source file to format
        file: PathBuf,

        /// Write changes to file (default is stdout)
        #[arg(long, short = 'w')]
        write: bool,
    },

    /// Tokenize a file and display tokens / تحليل الرموز
    Lex {
        /// Source file to tokenize
        file: PathBuf,
    },

    /// Parse a file and display AST / تحليل النحو
    Parse {
        /// Source file to parse
        file: PathBuf,
    },
}
