//! Update dependencies command

use crate::package::{LockFile, Manifest, PackageResult};
use colored::*;

pub fn run(package: Option<String>) -> PackageResult<()> {
    let (manifest, manifest_path) = Manifest::find_and_parse()?;
    let project_root = manifest_path.parent().ok_or_else(|| {
        crate::package::PackageError::InvalidManifest(
            "Cannot determine project root from manifest path / لا يمكن تحديد جذر المشروع"
                .to_string(),
        )
    })?;

    if let Some(pkg_name) = package {
        println!(
            "{}",
            format!("→ Updating '{}' / جاري تحديث '{}'...", pkg_name, pkg_name).cyan()
        );

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

        let lockfile_path = project_root.join(LockFile::DEFAULT_LOCK_NAME);
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

        let lockfile_path = project_root.join(LockFile::DEFAULT_LOCK_NAME);
        if lockfile_path.exists() {
            std::fs::remove_file(&lockfile_path)?;
        }
    }

    super::install::run(true)
}
