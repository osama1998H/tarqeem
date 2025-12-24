//! Run package command

use crate::package::{Manifest, PackageError, PackageResult};
use colored::*;
use std::process::Command;

pub fn run(args: Vec<String>) -> PackageResult<()> {
    // Find and parse manifest
    let (manifest, manifest_path) = Manifest::find_and_parse()?;
    let project_root = manifest_path.parent().unwrap();

    // Check if this is a library
    if manifest.is_library() {
        return Err(PackageError::BuildFailed(
            "Cannot run a library package. Use 'tarqeem pkg test' instead. / لا يمكن تشغيل حزمة مكتبة. استخدم 'tarqeem pkg test' بدلاً من ذلك."
                .to_string(),
        ));
    }

    // Check if built
    let output_path = project_root
        .join("بناء")
        .join("تطوير")
        .join(&manifest.package.name);

    if !output_path.exists() {
        println!(
            "{}",
            "→ Building package first... / جاري بناء الحزمة أولاً...".cyan()
        );
        super::build::run(false)?;
    }

    println!(
        "{}",
        format!(
            "→ Running '{}' / جاري تشغيل '{}'...",
            manifest.package.name, manifest.package.name
        )
        .cyan()
    );
    println!();

    // Run the executable
    let mut cmd = Command::new(&output_path);
    cmd.args(&args);
    cmd.current_dir(project_root);

    let status = cmd.status().map_err(|e| {
        PackageError::BuildFailed(format!(
            "Failed to run executable: {} / فشل تشغيل الملف التنفيذي: {}",
            e, e
        ))
    })?;

    if !status.success() {
        if let Some(code) = status.code() {
            println!();
            println!(
                "{}",
                format!(
                    "Process exited with code {} / انتهت العملية برمز الخروج {}",
                    code, code
                )
                .yellow()
            );
        }
    }

    Ok(())
}
