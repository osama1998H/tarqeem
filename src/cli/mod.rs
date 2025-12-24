//! Command Line Interface for Tarqeem

mod commands;
pub mod pm;

#[cfg(test)]
mod cli_tests;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub use commands::run;

#[derive(Parser, Debug)]
#[command(name = "tarqeem")]
#[command(author = "Tarqeem Contributors")]
#[command(version = "0.1.0")]
#[command(about = "ترقيم - أول لغة برمجة عربية مُترجَمة")]
#[command(
    long_about = "Tarqeem (ترقيم) is a compiled, general-purpose programming language with full Arabic syntax support."
)]
pub struct Cli {
    #[arg(long, short = 'e', global = true)]
    pub english: bool,

    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(aliases = ["ترجم", "c"])]
    Compile {
        file: PathBuf,
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,

        #[arg(short = 'O', value_name = "LEVEL", default_value = "0")]
        opt_level: u8,

        #[arg(long)]
        emit_llvm: bool,

        #[arg(long)]
        emit_asm: bool,

        #[arg(long, short = 'c')]
        emit_obj: bool,

        #[arg(long)]
        target: Option<String>,

        #[arg(long)]
        dump_tokens: bool,

        #[arg(long)]
        dump_ast: bool,

        #[arg(long)]
        dump_ir: bool,

        #[arg(long)]
        dump_opt_stats: bool,
    },

    #[command(aliases = ["شغل", "r"])]
    Run {
        file: PathBuf,
    },

    #[command(aliases = ["صحح", "dbg"])]
    Debug {
        file: PathBuf,

        #[arg(long, short = 's', aliases = ["توقف"])]
        stop_on_entry: bool,

        #[arg(long, short = 'p', aliases = ["منفذ"])]
        dap_port: Option<u16>,

        #[arg(long, aliases = ["إدخال_قياسي"])]
        dap_stdio: bool,

        #[arg(long, aliases = ["عربي"])]
        arabic: bool,
    },

    #[command(aliases = ["افحص", "ch"])]
    Check {
        file: PathBuf,
    },

    #[command(aliases = ["تفاعلي"])]
    Repl,

    #[command(aliases = ["نسق", "f"])]
    Fmt {
        path: Option<PathBuf>,

        #[arg(long, short = 'w', aliases = ["in-place", "i"])]
        write: bool,

        #[arg(long, aliases = ["افحص"])]
        check: bool,

        #[arg(long, aliases = ["فرق"])]
        diff: bool,

        #[arg(long, short = 'c', aliases = ["إعدادات"])]
        config: Option<PathBuf>,

        #[arg(long, aliases = ["نموذج"])]
        sample_config: bool,
    },

    Lex {
        file: PathBuf,
    },

    Parse {
        file: PathBuf,
    },

    #[command(aliases = ["حزم", "pm"])]
    Pkg {
        #[command(subcommand)]
        command: PkgCommands,
    },

    #[command(aliases = ["خادم"])]
    Lsp,

    #[command(aliases = ["توثيق", "وثق"])]
    Doc {
        path: PathBuf,

        #[arg(long, short = 'o')]
        output: Option<PathBuf>,

        #[arg(long, short = 'f', default_value = "html")]
        format: String,

        #[arg(long)]
        single_file: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum PkgCommands {
    #[command(aliases = ["هيئ", "أنشئ"])]
    Init {
        name: Option<String>,

        #[arg(long, short = 'l', aliases = ["مكتبة"])]
        lib: bool,
    },

    #[command(aliases = ["أضف"])]
    Add {
        package: String,

        #[arg(long, short = 'd', aliases = ["تطوير"])]
        dev: bool,

        #[arg(long, short = 'p', aliases = ["مسار"])]
        path: Option<PathBuf>,
    },

    #[command(aliases = ["احذف", "أزل"])]
    Remove { package: String },

    #[command(aliases = ["ثبت"])]
    Install {
        #[arg(long, short = 'f', aliases = ["أجبر"])]
        force: bool,
    },

    #[command(aliases = ["حدث"])]
    Update { package: Option<String> },

    #[command(aliases = ["ابنِ"])]
    Build {
        #[arg(long, short = 'r', aliases = ["إصدار"])]
        release: bool,
    },

    #[command(aliases = ["شغّل"])]
    Run { args: Vec<String> },

    #[command(aliases = ["اختبر"])]
    Test { filter: Option<String> },

    #[command(aliases = ["معلومات"])]
    Info,

    #[command(aliases = ["نظف"])]
    Clean,
}
