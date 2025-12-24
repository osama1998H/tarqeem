//! Build package command

use crate::package::{Manifest, PackageError, PackageResult};
use colored::*;
use std::process::Command;

pub fn run(release: bool) -> PackageResult<()> {
    // Find and parse manifest
    let (manifest, manifest_path) = Manifest::find_and_parse()?;
    let project_root = manifest_path.parent().unwrap();

    // Determine entry point
    let entry = manifest
        .package
        .entry
        .as_ref()
        .or(manifest.package.lib.as_ref())
        .ok_or_else(|| {
            PackageError::InvalidManifest(
                "No entry point specified (مدخل or entry) / لم يتم تحديد نقطة دخول".to_string(),
            )
        })?;

    let entry_path = project_root.join(entry);
    if !entry_path.exists() {
        return Err(PackageError::EntryPointNotFound(entry_path));
    }

    // Create output directory
    let output_dir = if release {
        project_root.join("بناء").join("إصدار")
    } else {
        project_root.join("بناء").join("تطوير")
    };
    std::fs::create_dir_all(&output_dir)?;

    // Determine output name
    let output_name = &manifest.package.name;
    let output_path = output_dir.join(output_name);

    println!(
        "{}",
        format!(
            "→ Building '{}' / جاري بناء '{}'...",
            manifest.package.name, manifest.package.name
        )
        .cyan()
    );

    if release {
        println!("  {} Release mode / وضع الإصدار", "→".cyan());
    } else {
        println!("  {} Debug mode / وضع التطوير", "→".cyan());
    }

    // Build command
    let mut cmd = Command::new("tarqeem");
    cmd.arg("compile");
    cmd.arg(&entry_path);
    cmd.arg("-o");
    cmd.arg(&output_path);

    if release {
        cmd.arg("-O2");
    }

    // Set current directory to project root for proper module resolution
    cmd.current_dir(project_root);

    println!(
        "  {} {} -> {}",
        "→".cyan(),
        entry_path.display(),
        output_path.display()
    );

    let output = cmd.output().map_err(|e| {
        PackageError::BuildFailed(format!(
            "Failed to run compiler: {} / فشل تشغيل المترجم: {}",
            e, e
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        if !stdout.is_empty() {
            println!("{}", stdout);
        }
        if !stderr.is_empty() {
            eprintln!("{}", stderr);
        }

        return Err(PackageError::BuildFailed(
            "Compilation failed / فشل الترجمة".to_string(),
        ));
    }

    // Print any output
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        println!("{}", stdout);
    }

    println!();
    println!(
        "{}",
        format!(
            "✓ Built '{}' -> {} / تم بناء '{}' -> {}",
            manifest.package.name,
            output_path.display(),
            manifest.package.name,
            output_path.display()
        )
        .green()
    );

    Ok(())
}
