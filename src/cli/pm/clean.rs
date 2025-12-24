//! Clean build artifacts command

use crate::package::{Manifest, PackageResult};
use colored::*;
use std::fs;

pub fn run() -> PackageResult<()> {
    let (manifest, manifest_path) = Manifest::find_and_parse()?;
    let project_root = manifest_path.parent().unwrap();

    println!(
        "{}",
        format!(
            "→ Cleaning build artifacts for '{}' / جاري تنظيف مخلفات البناء لـ '{}'...",
            manifest.package.name, manifest.package.name
        )
        .cyan()
    );

    let mut cleaned = 0;

    let build_dirs = ["بناء", "build", "target"];

    for dir_name in build_dirs {
        let dir = project_root.join(dir_name);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
            println!("  {} Removed / تمت إزالة: {}", "✓".green(), dir_name);
            cleaned += 1;
        }
    }

    let packages_dir = project_root.join("packages");
    if packages_dir.exists() {
        println!(
            "  {} Skipped packages/ (use --all to remove) / تم تخطي packages/",
            "→".cyan()
        );
    }

    if cleaned == 0 {
        println!("{}", "→ Nothing to clean / لا يوجد شيء للتنظيف".yellow());
    } else {
        println!();
        println!(
            "{}",
            format!(
                "✓ Cleaned {} directories / تم تنظيف {} مجلدات",
                cleaned, cleaned
            )
            .green()
        );
    }

    Ok(())
}
