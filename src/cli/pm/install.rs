//! Install dependencies command

use crate::package::{Cache, LockFile, Manifest, PackageResult, Resolver};
use colored::*;

/// Run the install command
pub fn run(force: bool) -> PackageResult<()> {
    // Find and parse manifest
    let (manifest, manifest_path) = Manifest::find_and_parse()?;
    let project_root = manifest_path.parent().unwrap();

    println!(
        "{}",
        format!(
            "→ Installing dependencies for '{}' / جاري تثبيت اعتماديات '{}'...",
            manifest.package.name, manifest.package.name
        )
        .cyan()
    );

    // Check if there are any dependencies
    let total_deps = manifest.dependencies.len() + manifest.dev_dependencies.len();
    if total_deps == 0 {
        println!(
            "{}",
            "→ No dependencies to install / لا توجد اعتماديات للتثبيت".yellow()
        );
        return Ok(());
    }

    // Check lockfile
    let lockfile_path = project_root.join(LockFile::FILENAME);
    let mut lockfile = if lockfile_path.exists() && !force {
        LockFile::parse(&lockfile_path).unwrap_or_else(|_| {
            LockFile::with_root(&manifest.package.name, &manifest.package.version)
        })
    } else {
        LockFile::with_root(&manifest.package.name, &manifest.package.version)
    };

    // Initialize cache and resolver
    let cache = Cache::new()?;
    let mut resolver = Resolver::new(&cache);

    println!(
        "{}",
        format!(
            "→ Resolving {} dependencies... / جاري تحليل {} اعتمادية...",
            total_deps, total_deps
        )
        .cyan()
    );

    // Resolve dependencies
    let resolved = match resolver.resolve(&manifest) {
        Ok(pkgs) => pkgs,
        Err(e) => {
            // For now, handle missing packages gracefully
            println!(
                "{}",
                format!(
                    "⚠ Some packages could not be resolved: {} / بعض الحزم لم يتم تحليلها: {}",
                    e, e
                )
                .yellow()
            );
            vec![]
        }
    };

    if resolved.is_empty() && total_deps > 0 {
        println!();
        println!(
            "{}",
            "ℹ No packages found in registry. For local development, use path dependencies:"
                .yellow()
        );
        println!("{}", "  اعتماديات:".cyan());
        println!(
            "{}",
            "    my-pkg = { نسخة = \"*\", مسار = \"../my-pkg\" }".cyan()
        );
        println!();

        // Still create empty packages directory and lockfile
        let packages_dir = project_root.join("packages");
        std::fs::create_dir_all(&packages_dir)?;
        lockfile.save(&lockfile_path)?;

        return Ok(());
    }

    // Download and install packages
    let packages_dir = project_root.join("packages");
    std::fs::create_dir_all(&packages_dir)?;

    for pkg in &resolved {
        // Check if already installed with correct version
        if let Some(locked) = lockfile.get_package(&pkg.name) {
            if locked.version == pkg.version && !force {
                println!("  {} {}@{}", "✓".green(), pkg.name, pkg.version);
                continue;
            }
        }

        println!("  {} {}@{}...", "↓".cyan(), pkg.name, pkg.version);

        // Download and verify
        match cache.download_and_verify(pkg) {
            Ok(_) => {
                // Update lockfile
                lockfile.upsert_package(pkg.to_locked());
                println!("  {} {}@{}", "✓".green(), pkg.name, pkg.version);
            }
            Err(e) => {
                println!("  {} {}@{}: {}", "✗".red(), pkg.name, pkg.version, e);
            }
        }
    }

    // Link packages
    if !resolved.is_empty() {
        cache.link_packages(&resolved, &packages_dir)?;
    }

    // Save lockfile
    lockfile.sort_packages();
    lockfile.save(&lockfile_path)?;

    println!();
    println!(
        "{}",
        format!(
            "✓ Installed {} packages / تم تثبيت {} حزمة",
            resolved.len(),
            resolved.len()
        )
        .green()
    );

    Ok(())
}
