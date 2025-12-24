//! Update dependencies command

use crate::package::{LockFile, Manifest, PackageResult};
use colored::*;

pub fn run(package: Option<String>) -> PackageResult<()> {
    // Find and parse manifest
    let (manifest, manifest_path) = Manifest::find_and_parse()?;
    let project_root = manifest_path.parent().unwrap();

    if let Some(pkg_name) = package {
        println!(
            "{}",
            format!("→ Updating '{}' / جاري تحديث '{}'...", pkg_name, pkg_name).cyan()
        );

        // Check if package exists in dependencies
        let in_deps = manifest.dependencies.contains_key(&pkg_name);
        let in_dev = manifest.dev_dependencies.contains_key(&pkg_name);

        if !in_deps && !in_dev {
            println!(
                "{}",
                format!(
                    "✗ Package '{}' not found in dependencies / الحزمة '{}' غير موجودة في الاعتماديات",
                    pkg_name, pkg_name
                )
                .red()
            );
            return Ok(());
        }

        // Remove from lockfile to force re-resolution
        let lockfile_path = project_root.join(LockFile::FILENAME);
        if lockfile_path.exists() {
            let mut lockfile = LockFile::parse(&lockfile_path)?;
            lockfile.remove_package(&pkg_name);
            lockfile.save(&lockfile_path)?;
        }
    } else {
        println!(
            "{}",
            "→ Updating all dependencies / جاري تحديث جميع الاعتماديات...".cyan()
        );

        // Remove lockfile to force full re-resolution
        let lockfile_path = project_root.join(LockFile::FILENAME);
        if lockfile_path.exists() {
            std::fs::remove_file(&lockfile_path)?;
        }
    }

    // Run install with force
    super::install::run(true)
}
