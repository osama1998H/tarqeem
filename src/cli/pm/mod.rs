//! Package manager CLI commands
//!
//! Implements all package management subcommands.

mod add;
mod build;
mod clean;
mod info;
mod init;
mod install;
mod remove;
mod run;
mod test;
mod update;

pub use add::run as add;
pub use build::run as build;
pub use clean::run as clean;
pub use info::run as info;
pub use init::run as init;
pub use install::run as install;
pub use remove::run as remove;
pub use run::run as pkg_run;
pub use test::run as test;
pub use update::run as update;

use crate::package::{Manifest, PackageResult};
use colored::*;
use std::path::Path;
pub fn print_success(en: &str, ar: &str) {
    println!("{} {} / {}", "✓".green(), en.green(), ar.green());
}
pub fn print_info(en: &str, ar: &str) {
    println!("{} {} / {}", "→".cyan(), en.cyan(), ar.cyan());
}

pub fn print_warning(en: &str, ar: &str) {
    println!("{} {} / {}", "⚠".yellow(), en.yellow(), ar.yellow());
}

pub fn print_error(en: &str, ar: &str) {
    eprintln!("{} {} / {}", "✗".red(), en.red(), ar.red());
}

pub fn find_project_root() -> PackageResult<(Manifest, std::path::PathBuf)> {
    Manifest::find_and_parse()
}

pub fn is_in_package() -> bool {
    Path::new("حزمة.toml").exists() || Path::new("trq.toml").exists()
}
