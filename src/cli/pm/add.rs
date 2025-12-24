//! Add dependency command

use crate::package::{DependencySpec, DetailedDependency, Manifest, PackageResult};
use colored::*;
use std::path::PathBuf;

pub fn run(package: String, dev: bool, path: Option<PathBuf>) -> PackageResult<()> {
    // Find and parse manifest
    let (mut manifest, manifest_path) = Manifest::find_and_parse()?;

    // Parse package@version format
    let (name, version) = parse_package_spec(&package);

    // Check if already exists
    let dep_map = if dev {
        &manifest.dev_dependencies
    } else {
        &manifest.dependencies
    };

    if dep_map.contains_key(&name) {
        println!(
            "{}",
            format!(
                "⚠ Package '{}' already exists, updating... / الحزمة '{}' موجودة، جاري التحديث...",
                name, name
            )
            .yellow()
        );
    }

    // Create dependency spec
    let dep_spec = if let Some(p) = path {
        DependencySpec::Detailed(DetailedDependency {
            version: version.clone(),
            path: Some(p),
            git: None,
            branch: None,
            tag: None,
            rev: None,
            optional: false,
            features: vec![],
        })
    } else {
        DependencySpec::Version(version.clone())
    };

    // Add to appropriate section
    let dep_map = if dev {
        &mut manifest.dev_dependencies
    } else {
        &mut manifest.dependencies
    };

    dep_map.insert(name.clone(), dep_spec);

    // Save manifest
    manifest.save(&manifest_path)?;

    let section = if dev {
        "dev-dependencies / اعتماديات-تطوير"
    } else {
        "dependencies / اعتماديات"
    };

    println!(
        "{}",
        format!(
            "✓ Added {}@{} to {} / تمت إضافة {}@{} إلى {}",
            name, version, section, name, version, section
        )
        .green()
    );

    println!();
    println!(
        "{}",
        "→ Run 'tarqeem pkg install' to install dependencies / شغّل 'tarqeem pkg install' لتثبيت الاعتماديات"
            .cyan()
    );

    Ok(())
}

fn parse_package_spec(spec: &str) -> (String, String) {
    if let Some(idx) = spec.find('@') {
        let (name, version) = spec.split_at(idx);
        (name.to_string(), version[1..].to_string())
    } else {
        (spec.to_string(), "*".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_spec() {
        let (name, version) = parse_package_spec("json");
        assert_eq!(name, "json");
        assert_eq!(version, "*");

        let (name, version) = parse_package_spec("json@1.0.0");
        assert_eq!(name, "json");
        assert_eq!(version, "1.0.0");

        let (name, version) = parse_package_spec("مجموعات@^2.0");
        assert_eq!(name, "مجموعات");
        assert_eq!(version, "^2.0");
    }
}
