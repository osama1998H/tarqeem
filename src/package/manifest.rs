//! Package manifest parsing (حزمة.toml / trq.toml)
//!
//! Supports bilingual keys with Arabic primary and English aliases.

use super::error::{PackageError, PackageResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Package manifest (حزمة.toml / trq.toml)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    /// Package metadata
    #[serde(rename = "حزمة", alias = "package")]
    pub package: PackageInfo,

    /// Dependencies
    #[serde(rename = "اعتماديات", alias = "dependencies", default)]
    pub dependencies: HashMap<String, DependencySpec>,

    /// Dev dependencies
    #[serde(
        rename = "اعتماديات_تطوير",
        alias = "اعتماديات-تطوير",
        alias = "dev-dependencies",
        alias = "dev_dependencies",
        default
    )]
    pub dev_dependencies: HashMap<String, DependencySpec>,

    /// Build scripts
    #[serde(rename = "سكربتات", alias = "scripts", default)]
    pub scripts: HashMap<String, String>,
}

/// Package metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageInfo {
    /// Package name (Arabic or English)
    #[serde(rename = "اسم", alias = "name")]
    pub name: String,

    /// Semantic version
    #[serde(rename = "نسخة", alias = "version")]
    pub version: String,

    /// Package description
    #[serde(rename = "وصف", alias = "description", default)]
    pub description: Option<String>,

    /// Author(s)
    #[serde(
        rename = "مؤلفون",
        alias = "مؤلف",
        alias = "authors",
        alias = "author",
        default
    )]
    pub authors: Authors,

    /// License identifier
    #[serde(rename = "رخصة", alias = "license", default)]
    pub license: Option<String>,

    /// Repository URL
    #[serde(rename = "مستودع", alias = "repository", default)]
    pub repository: Option<String>,

    /// Homepage URL
    #[serde(rename = "موقع", alias = "homepage", default)]
    pub homepage: Option<String>,

    /// Keywords for search
    #[serde(rename = "كلمات", alias = "keywords", default)]
    pub keywords: Vec<String>,

    /// Entry point for binaries
    #[serde(rename = "مدخل", alias = "entry", alias = "main", default)]
    pub entry: Option<String>,

    /// Library entry point
    #[serde(rename = "مكتبة", alias = "lib", default)]
    pub lib: Option<String>,

    /// Minimum Tarqeem version required
    #[serde(rename = "ترقيم", alias = "tarqeem", default)]
    pub tarqeem_version: Option<String>,
}

/// Author(s) can be a single string or array
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum Authors {
    #[default]
    None,
    Single(String),
    Multiple(Vec<String>),
}

impl Authors {
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            Authors::None => vec![],
            Authors::Single(s) => vec![s.clone()],
            Authors::Multiple(v) => v.clone(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Authors::None => true,
            Authors::Single(s) => s.is_empty(),
            Authors::Multiple(v) => v.is_empty(),
        }
    }
}

/// Dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    /// Simple version string: "1.0.0" or "^1.0" or "*"
    Version(String),

    /// Detailed dependency specification
    Detailed(DetailedDependency),
}

impl DependencySpec {
    /// Get the version requirement string
    pub fn version_req(&self) -> &str {
        match self {
            DependencySpec::Version(v) => v,
            DependencySpec::Detailed(d) => &d.version,
        }
    }

    /// Check if this is a path dependency
    pub fn is_path(&self) -> bool {
        matches!(self, DependencySpec::Detailed(d) if d.path.is_some())
    }

    /// Check if this is a git dependency
    pub fn is_git(&self) -> bool {
        matches!(self, DependencySpec::Detailed(d) if d.git.is_some())
    }

    /// Get path if this is a path dependency
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            DependencySpec::Detailed(d) => d.path.as_ref(),
            _ => None,
        }
    }
}

/// Detailed dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedDependency {
    /// Version requirement
    #[serde(rename = "نسخة", alias = "version")]
    pub version: String,

    /// Local path (for development)
    #[serde(rename = "مسار", alias = "path", default)]
    pub path: Option<PathBuf>,

    /// Git repository URL
    #[serde(default)]
    pub git: Option<String>,

    /// Git branch
    #[serde(rename = "فرع", alias = "branch", default)]
    pub branch: Option<String>,

    /// Git tag
    #[serde(rename = "وسم", alias = "tag", default)]
    pub tag: Option<String>,

    /// Git revision (commit hash)
    #[serde(rename = "مراجعة", alias = "rev", default)]
    pub rev: Option<String>,

    /// Optional dependency
    #[serde(rename = "اختياري", alias = "optional", default)]
    pub optional: bool,

    /// Features to enable
    #[serde(rename = "ميزات", alias = "features", default)]
    pub features: Vec<String>,
}

impl Manifest {
    /// Manifest file names to search for (in order of preference)
    pub const MANIFEST_NAMES: &'static [&'static str] = &["حزمة.toml", "trq.toml"];

    /// Find and parse manifest from current directory or parents
    pub fn find_and_parse() -> PackageResult<(Self, PathBuf)> {
        let current = std::env::current_dir()?;
        Self::find_and_parse_from(&current)
    }

    /// Find and parse manifest starting from a specific directory
    pub fn find_and_parse_from(start: &Path) -> PackageResult<(Self, PathBuf)> {
        let mut current = start.to_path_buf();

        loop {
            for name in Self::MANIFEST_NAMES {
                let manifest_path = current.join(name);
                if manifest_path.exists() {
                    let manifest = Self::parse(&manifest_path)?;
                    return Ok((manifest, manifest_path));
                }
            }

            if !current.pop() {
                return Err(PackageError::ManifestNotFound);
            }
        }
    }

    /// Parse manifest from a specific path
    pub fn parse(path: &Path) -> PackageResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Manifest = toml::from_str(&content)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate manifest contents
    pub fn validate(&self) -> PackageResult<()> {
        // Validate package name
        if self.package.name.is_empty() {
            return Err(PackageError::InvalidManifest(
                "Package name is required / اسم الحزمة مطلوب".to_string(),
            ));
        }

        // Validate version
        if self.package.version.is_empty() {
            return Err(PackageError::InvalidManifest(
                "Package version is required / نسخة الحزمة مطلوبة".to_string(),
            ));
        }

        // Try to parse version as semver
        if semver::Version::parse(&self.package.version).is_err() {
            // Allow non-strict versions like "0.1" by adding .0
            let with_patch = format!("{}.0", self.package.version);
            if semver::Version::parse(&with_patch).is_err() {
                return Err(PackageError::InvalidVersion(self.package.version.clone()));
            }
        }

        Ok(())
    }

    /// Create a new manifest with defaults
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            package: PackageInfo {
                name: name.to_string(),
                version: version.to_string(),
                description: None,
                authors: Authors::None,
                license: Some("MIT".to_string()),
                repository: None,
                homepage: None,
                keywords: vec![],
                entry: Some("مصدر/رئيسي.trq".to_string()),
                lib: None,
                tarqeem_version: None,
            },
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            scripts: HashMap::new(),
        }
    }

    /// Create a new library manifest
    pub fn new_lib(name: &str, version: &str) -> Self {
        let mut manifest = Self::new(name, version);
        manifest.package.entry = None;
        manifest.package.lib = Some("مصدر/lib.trq".to_string());
        manifest
    }

    /// Save manifest to file
    pub fn save(&self, path: &Path) -> PackageResult<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the project root (directory containing manifest)
    pub fn project_root(manifest_path: &Path) -> Option<&Path> {
        manifest_path.parent()
    }

    /// Get the entry point path
    pub fn entry_path(&self, project_root: &Path) -> Option<PathBuf> {
        self.package
            .entry
            .as_ref()
            .or(self.package.lib.as_ref())
            .map(|e| project_root.join(e))
    }

    /// Check if this is a library package
    pub fn is_library(&self) -> bool {
        self.package.lib.is_some() && self.package.entry.is_none()
    }

    /// Get all dependencies (regular + dev)
    pub fn all_dependencies(&self) -> impl Iterator<Item = (&String, &DependencySpec)> {
        self.dependencies.iter().chain(self.dev_dependencies.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arabic_manifest() {
        // Note: TOML requires quoting non-ASCII section names
        let toml = r#"
["حزمة"]
"اسم" = "مكتبتي"
"نسخة" = "1.0.0"
"وصف" = "مكتبة رائعة"
"رخصة" = "MIT"

["اعتماديات"]
json = "2.0"
"أدوات-إضافية" = { "نسخة" = "1.0", "اختياري" = true }
"#;
        let manifest: Manifest = toml::from_str(toml).unwrap();
        assert_eq!(manifest.package.name, "مكتبتي");
        assert_eq!(manifest.package.version, "1.0.0");
        assert_eq!(
            manifest.package.description,
            Some("مكتبة رائعة".to_string())
        );
        assert!(manifest.dependencies.contains_key("json"));
        assert!(manifest.dependencies.contains_key("أدوات-إضافية"));
    }

    #[test]
    fn test_parse_english_manifest() {
        let toml = r#"
[package]
name = "my-lib"
version = "1.0.0"
description = "A great library"

[dependencies]
json = "2.0"
"#;
        let manifest: Manifest = toml::from_str(toml).unwrap();
        assert_eq!(manifest.package.name, "my-lib");
        assert_eq!(manifest.package.version, "1.0.0");
    }

    #[test]
    fn test_parse_mixed_manifest() {
        // Note: TOML requires quoting non-ASCII section names and keys
        let toml = r#"
["حزمة"]
name = "mixed-pkg"
"نسخة" = "0.1.0"

[dependencies]
"مجموعات" = "1.0"
utils = "0.5"
"#;
        let manifest: Manifest = toml::from_str(toml).unwrap();
        assert_eq!(manifest.package.name, "mixed-pkg");
        assert_eq!(manifest.package.version, "0.1.0");
        assert!(manifest.dependencies.contains_key("مجموعات"));
        assert!(manifest.dependencies.contains_key("utils"));
    }

    #[test]
    fn test_dependency_spec_version() {
        let spec = DependencySpec::Version("1.0.0".to_string());
        assert_eq!(spec.version_req(), "1.0.0");
        assert!(!spec.is_path());
        assert!(!spec.is_git());
    }

    #[test]
    fn test_dependency_spec_detailed() {
        let spec = DependencySpec::Detailed(DetailedDependency {
            version: "1.0.0".to_string(),
            path: Some(PathBuf::from("../local-pkg")),
            git: None,
            branch: None,
            tag: None,
            rev: None,
            optional: false,
            features: vec![],
        });
        assert_eq!(spec.version_req(), "1.0.0");
        assert!(spec.is_path());
        assert!(!spec.is_git());
    }

    #[test]
    fn test_new_manifest() {
        let manifest = Manifest::new("test-pkg", "0.1.0");
        assert_eq!(manifest.package.name, "test-pkg");
        assert_eq!(manifest.package.version, "0.1.0");
        assert_eq!(manifest.package.entry, Some("مصدر/رئيسي.trq".to_string()));
    }

    #[test]
    fn test_new_lib_manifest() {
        let manifest = Manifest::new_lib("test-lib", "0.1.0");
        assert_eq!(manifest.package.name, "test-lib");
        assert!(manifest.package.entry.is_none());
        assert_eq!(manifest.package.lib, Some("مصدر/lib.trq".to_string()));
        assert!(manifest.is_library());
    }

    #[test]
    fn test_authors_single() {
        let authors = Authors::Single("أحمد".to_string());
        assert_eq!(authors.as_vec(), vec!["أحمد".to_string()]);
        assert!(!authors.is_empty());
    }

    #[test]
    fn test_authors_multiple() {
        let authors = Authors::Multiple(vec!["أحمد".to_string(), "محمد".to_string()]);
        assert_eq!(authors.as_vec().len(), 2);
        assert!(!authors.is_empty());
    }

    #[test]
    fn test_authors_none() {
        let authors = Authors::None;
        assert!(authors.as_vec().is_empty());
        assert!(authors.is_empty());
    }

    #[test]
    fn test_manifest_validation_empty_name() {
        let mut manifest = Manifest::new("test", "1.0.0");
        manifest.package.name = String::new();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_manifest_validation_empty_version() {
        let mut manifest = Manifest::new("test", "1.0.0");
        manifest.package.version = String::new();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_manifest_validation_valid() {
        let manifest = Manifest::new("test-pkg", "1.0.0");
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_manifest_all_dependencies() {
        let toml = r#"
[package]
name = "test"
version = "1.0.0"

[dependencies]
dep1 = "1.0"
dep2 = "2.0"

[dev-dependencies]
dev1 = "0.1"
"#;
        let manifest: Manifest = toml::from_str(toml).unwrap();
        let all_deps: Vec<_> = manifest.all_dependencies().collect();
        assert_eq!(all_deps.len(), 3);
    }
}
