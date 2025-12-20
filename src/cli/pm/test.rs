//! Run tests command

use crate::package::{Manifest, PackageError, PackageResult};
use colored::*;
use std::process::Command;
use walkdir::WalkDir;

/// Run the test command
pub fn run(filter: Option<String>) -> PackageResult<()> {
    // Find and parse manifest
    let (manifest, manifest_path) = Manifest::find_and_parse()?;
    let project_root = manifest_path.parent().unwrap();

    println!(
        "{}",
        format!(
            "→ Running tests for '{}' / جاري تشغيل اختبارات '{}'...",
            manifest.package.name, manifest.package.name
        )
        .cyan()
    );

    // Find test directory
    let test_dirs = ["اختبارات", "tests"];
    let test_dir = test_dirs
        .iter()
        .map(|d| project_root.join(d))
        .find(|p| p.exists());

    let test_dir = match test_dir {
        Some(d) => d,
        None => {
            println!(
                "{}",
                "→ No tests directory found (اختبارات/ or tests/) / لم يتم العثور على مجلد اختبارات"
                    .yellow()
            );
            return Ok(());
        }
    };

    // Find test files
    let test_files: Vec<_> = WalkDir::new(&test_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.is_file()
                && (path.extension().map_or(false, |ext| ext == "trq")
                    || path
                        .file_name()
                        .map_or(false, |n| n.to_string_lossy().ends_with(".ترقيم")))
        })
        .filter(|e| {
            if let Some(ref f) = filter {
                e.path().to_string_lossy().contains(f.as_str())
            } else {
                true
            }
        })
        .collect();

    if test_files.is_empty() {
        println!(
            "{}",
            "→ No test files found / لم يتم العثور على ملفات اختبار".yellow()
        );
        return Ok(());
    }

    println!(
        "{}",
        format!(
            "→ Found {} test files / تم العثور على {} ملف اختبار",
            test_files.len(),
            test_files.len()
        )
        .cyan()
    );

    let mut passed = 0;
    let mut failed = 0;

    for entry in test_files {
        let path = entry.path();
        let name = path
            .strip_prefix(project_root)
            .unwrap_or(path)
            .display()
            .to_string();

        print!("  {} {}...", "→".cyan(), name);

        // Run test file with interpreter
        let mut cmd = Command::new("tarqeem");
        cmd.arg("run");
        cmd.arg(path);
        cmd.current_dir(project_root);

        let output = cmd.output();

        match output {
            Ok(output) if output.status.success() => {
                println!(" {}", "✓".green());
                passed += 1;
            }
            Ok(output) => {
                println!(" {}", "✗".red());
                failed += 1;

                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() {
                    for line in stderr.lines() {
                        println!("    {}", line.red());
                    }
                }
            }
            Err(e) => {
                println!(" {} ({})", "✗".red(), e);
                failed += 1;
            }
        }
    }

    println!();
    if failed == 0 {
        println!(
            "{}",
            format!(
                "✓ All {} tests passed / نجحت جميع الـ {} اختبارات",
                passed, passed
            )
            .green()
        );
    } else {
        println!(
            "{}",
            format!(
                "✗ {} passed, {} failed / {} ناجح، {} فاشل",
                passed, failed, passed, failed
            )
            .red()
        );
    }

    if failed > 0 {
        Err(PackageError::BuildFailed(format!(
            "{} tests failed / {} اختبارات فشلت",
            failed, failed
        )))
    } else {
        Ok(())
    }
}
