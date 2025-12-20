//! Remove dependency command

use crate::package::{Manifest, PackageError, PackageResult};
use colored::*;

/// Run the remove command
pub fn run(package: String) -> PackageResult<()> {
    // Find and parse manifest
    let (mut manifest, manifest_path) = Manifest::find_and_parse()?;

    // Try to remove from dependencies
    let removed_from_deps = manifest.dependencies.remove(&package).is_some();
    let removed_from_dev = manifest.dev_dependencies.remove(&package).is_some();

    if !removed_from_deps && !removed_from_dev {
        return Err(PackageError::PackageNotFound(package));
    }

    // Save manifest
    manifest.save(&manifest_path)?;

    let section = if removed_from_deps {
        "dependencies / اعتماديات"
    } else {
        "dev-dependencies / اعتماديات-تطوير"
    };

    println!(
        "{}",
        format!(
            "✓ Removed '{}' from {} / تمت إزالة '{}' من {}",
            package, section, package, section
        )
        .green()
    );

    // TODO: Run install to update packages directory

    Ok(())
}
