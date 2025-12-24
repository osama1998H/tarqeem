//! Remove dependency command

use crate::package::{Manifest, PackageError, PackageResult};
use colored::*;

pub fn run(package: String) -> PackageResult<()> {
    let (mut manifest, manifest_path) = Manifest::find_and_parse()?;

    let removed_from_deps = manifest.dependencies.remove(&package).is_some();
    let removed_from_dev = manifest.dev_dependencies.remove(&package).is_some();

    if !removed_from_deps && !removed_from_dev {
        return Err(PackageError::PackageNotFound(package));
    }

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

    println!("{}", "→ Updating packages... / جاري تحديث الحزم...".cyan());
    super::install::run(false)?;

    Ok(())
}
