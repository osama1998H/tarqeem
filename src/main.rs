//! Tarqeem CLI - Command Line Interface for the Tarqeem compiler

use clap::Parser;
use tarqeem::cli::Cli;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = tarqeem::cli::run(cli) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
