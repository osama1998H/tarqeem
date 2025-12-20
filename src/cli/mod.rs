//! Command Line Interface for Tarqeem

mod commands;
pub mod pm;

#[cfg(test)]
mod cli_tests;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub use commands::run;

/// Tarqeem - The Arabic Programming Language Compiler
#[derive(Parser, Debug)]
#[command(name = "tarqeem")]
#[command(author = "Tarqeem Contributors")]
#[command(version = "0.1.0")]
#[command(about = "ترقيم - أول لغة برمجة عربية مُترجَمة")]
#[command(
    long_about = "Tarqeem (ترقيم) is a compiled, general-purpose programming language with full Arabic syntax support."
)]
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

        /// Optimization level (0=none, 1=basic, 2=standard, 3=aggressive)
        /// مستوى التحسين (0=لا شيء، 1=أساسي، 2=قياسي، 3=متقدم)
        #[arg(short = 'O', value_name = "LEVEL", default_value = "0")]
        opt_level: u8,

        /// Emit LLVM IR instead of object code / إخراج LLVM IR
        #[arg(long)]
        emit_llvm: bool,

        /// Emit assembly instead of object code / إخراج التجميع
        #[arg(long)]
        emit_asm: bool,

        /// Emit object file only (don't link) / إخراج ملف كائن فقط
        #[arg(long, short = 'c')]
        emit_obj: bool,

        /// Target triple (e.g. x86_64-unknown-linux-gnu)
        #[arg(long)]
        target: Option<String>,

        /// Dump tokens (for debugging)
        #[arg(long)]
        dump_tokens: bool,

        /// Dump AST (for debugging)
        #[arg(long)]
        dump_ast: bool,

        /// Dump IR (for debugging)
        #[arg(long)]
        dump_ir: bool,

        /// Show optimization statistics
        #[arg(long)]
        dump_opt_stats: bool,
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
        /// Source file or directory to format (not required with --sample-config)
        path: Option<PathBuf>,

        /// Write changes to file (default is stdout)
        #[arg(long, short = 'w', aliases = ["in-place", "i"])]
        write: bool,

        /// Check if file is formatted (exit 1 if not)
        #[arg(long, aliases = ["افحص"])]
        check: bool,

        /// Show diff between original and formatted
        #[arg(long, aliases = ["فرق"])]
        diff: bool,

        /// Configuration file path
        #[arg(long, short = 'c', aliases = ["إعدادات"])]
        config: Option<PathBuf>,

        /// Generate sample config file
        #[arg(long, aliases = ["نموذج"])]
        sample_config: bool,
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

    /// Package management / إدارة الحزم
    #[command(aliases = ["حزم", "pm"])]
    Pkg {
        #[command(subcommand)]
        command: PkgCommands,
    },

    /// Start the Language Server Protocol server / بدء خادم LSP
    #[command(aliases = ["خادم"])]
    Lsp,

    /// Generate documentation / توليد التوثيق
    #[command(aliases = ["توثيق", "وثق"])]
    Doc {
        /// Source file or directory to document
        path: PathBuf,

        /// Output directory for generated docs
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,

        /// Documentation format (html, markdown, json) / صيغة التوثيق
        #[arg(long, short = 'f', default_value = "html")]
        format: String,

        /// Generate single file instead of directory / ملف واحد بدلاً من مجلد
        #[arg(long)]
        single_file: bool,
    },
}

/// Package manager subcommands / أوامر مدير الحزم
#[derive(Subcommand, Debug)]
pub enum PkgCommands {
    /// Initialize a new package / تهيئة حزمة جديدة
    #[command(aliases = ["هيئ", "أنشئ"])]
    Init {
        /// Package name (defaults to directory name)
        name: Option<String>,

        /// Create library package instead of binary / إنشاء مكتبة
        #[arg(long, short = 'l', aliases = ["مكتبة"])]
        lib: bool,
    },

    /// Add a dependency / إضافة اعتمادية
    #[command(aliases = ["أضف"])]
    Add {
        /// Package name with optional version (e.g., "json" or "json@1.0")
        package: String,

        /// Add as dev dependency / إضافة كاعتمادية تطوير
        #[arg(long, short = 'd', aliases = ["تطوير"])]
        dev: bool,

        /// Path to local package / مسار لحزمة محلية
        #[arg(long, short = 'p', aliases = ["مسار"])]
        path: Option<PathBuf>,
    },

    /// Remove a dependency / إزالة اعتمادية
    #[command(aliases = ["احذف", "أزل"])]
    Remove {
        /// Package name to remove
        package: String,
    },

    /// Install dependencies / تثبيت الاعتماديات
    #[command(aliases = ["ثبت"])]
    Install {
        /// Force reinstall / إعادة التثبيت
        #[arg(long, short = 'f', aliases = ["أجبر"])]
        force: bool,
    },

    /// Update dependencies / تحديث الاعتماديات
    #[command(aliases = ["حدث"])]
    Update {
        /// Specific package to update (updates all if not specified)
        package: Option<String>,
    },

    /// Build the package / بناء الحزمة
    #[command(aliases = ["ابنِ"])]
    Build {
        /// Build in release mode / بناء بوضع الإصدار
        #[arg(long, short = 'r', aliases = ["إصدار"])]
        release: bool,
    },

    /// Run the package / تشغيل الحزمة
    #[command(aliases = ["شغّل"])]
    Run {
        /// Arguments to pass to the program
        args: Vec<String>,
    },

    /// Run tests / تشغيل الاختبارات
    #[command(aliases = ["اختبر"])]
    Test {
        /// Test filter
        filter: Option<String>,
    },

    /// Show package info / عرض معلومات الحزمة
    #[command(aliases = ["معلومات"])]
    Info,

    /// Clean build artifacts / تنظيف مخلفات البناء
    #[command(aliases = ["نظف"])]
    Clean,
}
