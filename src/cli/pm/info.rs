//! Show package info command

use crate::package::{Manifest, PackageResult};
use colored::*;

/// Run the info command
pub fn run() -> PackageResult<()> {
    // Find and parse manifest
    let (manifest, manifest_path) = Manifest::find_and_parse()?;

    println!();
    println!("{}", "Package Information / معلومات الحزمة".bold());
    println!("{}", "═".repeat(40));
    println!();

    // Basic info
    println!(
        "  {} {} {}",
        "Name / الاسم:".cyan(),
        manifest.package.name.bold(),
        format!("v{}", manifest.package.version).dimmed()
    );

    if let Some(ref desc) = manifest.package.description {
        println!("  {} {}", "Description / الوصف:".cyan(), desc);
    }

    if !manifest.package.authors.is_empty() {
        let authors = manifest.package.authors.as_vec().join(", ");
        println!("  {} {}", "Authors / المؤلفون:".cyan(), authors);
    }

    if let Some(ref license) = manifest.package.license {
        println!("  {} {}", "License / الرخصة:".cyan(), license);
    }

    if let Some(ref repo) = manifest.package.repository {
        println!("  {} {}", "Repository / المستودع:".cyan(), repo);
    }

    println!();

    // Entry points
    println!("{}", "Entry Points / نقاط الدخول".bold());
    println!("{}", "─".repeat(40));

    if let Some(ref entry) = manifest.package.entry {
        println!("  {} {}", "Binary / تنفيذي:".cyan(), entry);
    }

    if let Some(ref lib) = manifest.package.lib {
        println!("  {} {}", "Library / مكتبة:".cyan(), lib);
    }

    println!();

    // Dependencies
    if !manifest.dependencies.is_empty() {
        println!("{}", "Dependencies / الاعتماديات".bold());
        println!("{}", "─".repeat(40));

        for (name, spec) in &manifest.dependencies {
            println!("  {} {}", name.cyan(), spec.version_req().dimmed());
        }
        println!();
    }

    // Dev dependencies
    if !manifest.dev_dependencies.is_empty() {
        println!("{}", "Dev Dependencies / اعتماديات التطوير".bold());
        println!("{}", "─".repeat(40));

        for (name, spec) in &manifest.dev_dependencies {
            println!("  {} {}", name.cyan(), spec.version_req().dimmed());
        }
        println!();
    }

    // Manifest path
    println!(
        "  {} {}",
        "Manifest / ملف الحزمة:".dimmed(),
        manifest_path.display()
    );

    Ok(())
}
