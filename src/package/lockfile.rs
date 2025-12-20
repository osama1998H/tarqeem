//! Lock file management (.trqlock)
//!
//! Ensures reproducible builds by locking exact versions.

use super::error::PackageResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Lock file format version
pub const LOCKFILE_VERSION: u32 = 1;

/// Lock file for deterministic builds (.trqlock)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFile {
    /// Lock file format version
    pub version: u32,

    /// Root package info
    #[serde(default)]
    pub root: Option<RootPackage>,

    /// Locked packages
    #[serde(default)]
    pub packages: Vec<LockedPackage>,
}

/// Root package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootPackage {
    /// Package name
    pub name: String,

    /// Package version
    pub version: String,
}

/// A locked package with exact version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    /// Package name
    pub name: String,

    /// Exact version
    pub version: String,

    /// Source type and location
    pub source: PackageSource,

    /// SHA256 checksum of the package
    pub checksum: String,

    /// Dependencies of this package (name = version)
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

/// Source of a package
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PackageSource {
    /// From the package registry
    #[serde(rename = "registry")]
    Registry {
        /// Registry URL
        url: String,
    },

    /// From a git repository
    #[serde(rename = "git")]
    Git {
        /// Git repository URL
        url: String,
        /// Branch, tag, or commit
        #[serde(flatten)]
        reference: GitReference,
    },

    /// From a local path
    #[serde(rename = "path")]
    Path {
        /// Local path
        path: String,
    },
}

/// Git reference type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GitReference {
    Branch { branch: String },
    Tag { tag: String },
    Rev { rev: String },
}

impl LockFile {
    /// Lock file name
    pub const FILENAME: &'static str = ".trqlock";

    /// Create a new empty lock file
    pub fn new() -> Self {
        Self {
            version: LOCKFILE_VERSION,
            root: None,
            packages: vec![],
        }
    }

    /// Create a lock file with root package info
    pub fn with_root(name: &str, version: &str) -> Self {
        Self {
            version: LOCKFILE_VERSION,
            root: Some(RootPackage {
                name: name.to_string(),
                version: version.to_string(),
            }),
            packages: vec![],
        }
    }

    /// Parse lock file from path
    pub fn parse(path: &Path) -> PackageResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let lockfile: LockFile = toml::from_str(&content)?;
        Ok(lockfile)
    }

    /// Try to parse lock file, returns None if not found
    pub fn try_parse(path: &Path) -> Option<Self> {
        Self::parse(path).ok()
    }

    /// Save lock file to path
    pub fn save(&self, path: &Path) -> PackageResult<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get a locked package by name
    pub fn get_package(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }

    /// Get a locked package by name (mutable)
    pub fn get_package_mut(&mut self, name: &str) -> Option<&mut LockedPackage> {
        self.packages.iter_mut().find(|p| p.name == name)
    }

    /// Check if a package is locked
    pub fn has_package(&self, name: &str) -> bool {
        self.packages.iter().any(|p| p.name == name)
    }

    /// Add or update a locked package
    pub fn upsert_package(&mut self, package: LockedPackage) {
        if let Some(existing) = self.get_package_mut(&package.name) {
            *existing = package;
        } else {
            self.packages.push(package);
        }
    }

    /// Remove a package from the lock file
    pub fn remove_package(&mut self, name: &str) -> Option<LockedPackage> {
        if let Some(idx) = self.packages.iter().position(|p| p.name == name) {
            Some(self.packages.remove(idx))
        } else {
            None
        }
    }

    /// Check if a package version matches the lock
    pub fn matches(&self, name: &str, version: &str) -> bool {
        self.get_package(name)
            .map(|p| p.version == version)
            .unwrap_or(false)
    }

    /// Get all package names
    pub fn package_names(&self) -> impl Iterator<Item = &str> {
        self.packages.iter().map(|p| p.name.as_str())
    }

    /// Sort packages by name for consistent output
    pub fn sort_packages(&mut self) {
        self.packages.sort_by(|a, b| a.name.cmp(&b.name));
    }

    /// Clear all packages
    pub fn clear(&mut self) {
        self.packages.clear();
    }
}

impl Default for LockFile {
    fn default() -> Self {
        Self::new()
    }
}

impl LockedPackage {
    /// Create a new locked package from registry
    pub fn from_registry(
        name: String,
        version: String,
        registry_url: String,
        checksum: String,
    ) -> Self {
        Self {
            name,
            version,
            source: PackageSource::Registry { url: registry_url },
            checksum,
            dependencies: HashMap::new(),
        }
    }

    /// Create a new locked package from path
    pub fn from_path(name: String, version: String, path: String, checksum: String) -> Self {
        Self {
            name,
            version,
            source: PackageSource::Path { path },
            checksum,
            dependencies: HashMap::new(),
        }
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, name: String, version: String) {
        self.dependencies.insert(name, version);
    }
}

impl PackageSource {
    /// Check if this is a registry source
    pub fn is_registry(&self) -> bool {
        matches!(self, PackageSource::Registry { .. })
    }

    /// Check if this is a git source
    pub fn is_git(&self) -> bool {
        matches!(self, PackageSource::Git { .. })
    }

    /// Check if this is a path source
    pub fn is_path(&self) -> bool {
        matches!(self, PackageSource::Path { .. })
    }

    /// Get a display string for the source
    pub fn display(&self) -> String {
        match self {
            PackageSource::Registry { url } => format!("registry: {}", url),
            PackageSource::Git { url, reference } => {
                let ref_str = match reference {
                    GitReference::Branch { branch } => format!("branch={}", branch),
                    GitReference::Tag { tag } => format!("tag={}", tag),
                    GitReference::Rev { rev } => format!("rev={}", rev),
                };
                format!("git: {} ({})", url, ref_str)
            }
            PackageSource::Path { path } => format!("path: {}", path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_lockfile() {
        let lockfile = LockFile::new();
        assert_eq!(lockfile.version, LOCKFILE_VERSION);
        assert!(lockfile.packages.is_empty());
    }

    #[test]
    fn test_lockfile_with_root() {
        let lockfile = LockFile::with_root("my-pkg", "1.0.0");
        assert!(lockfile.root.is_some());
        let root = lockfile.root.unwrap();
        assert_eq!(root.name, "my-pkg");
        assert_eq!(root.version, "1.0.0");
    }

    #[test]
    fn test_upsert_package() {
        let mut lockfile = LockFile::new();

        let pkg = LockedPackage::from_registry(
            "json".to_string(),
            "1.0.0".to_string(),
            "https://registry.tarqeem.dev".to_string(),
            "abc123".to_string(),
        );

        lockfile.upsert_package(pkg.clone());
        assert_eq!(lockfile.packages.len(), 1);

        // Update version
        let mut pkg2 = pkg;
        pkg2.version = "1.0.1".to_string();
        lockfile.upsert_package(pkg2);
        assert_eq!(lockfile.packages.len(), 1);
        assert_eq!(lockfile.get_package("json").unwrap().version, "1.0.1");
    }

    #[test]
    fn test_serialize_lockfile() {
        let mut lockfile = LockFile::with_root("my-app", "0.1.0");

        let mut pkg = LockedPackage::from_registry(
            "json".to_string(),
            "2.0.0".to_string(),
            "https://registry.tarqeem.dev".to_string(),
            "sha256:abc123def456".to_string(),
        );
        pkg.add_dependency("utils".to_string(), "1.0.0".to_string());

        lockfile.upsert_package(pkg);

        let toml_str = toml::to_string_pretty(&lockfile).unwrap();
        assert!(toml_str.contains("version = 1"));
        assert!(toml_str.contains("name = \"json\""));
        assert!(toml_str.contains("version = \"2.0.0\""));
    }

    #[test]
    fn test_parse_lockfile() {
        let toml = r#"
version = 1

[root]
name = "my-app"
version = "0.1.0"

[[packages]]
name = "json"
version = "2.0.0"
checksum = "sha256:abc123"

[packages.source]
type = "registry"
url = "https://registry.tarqeem.dev"

[packages.dependencies]
utils = "1.0.0"
"#;
        let lockfile: LockFile = toml::from_str(toml).unwrap();
        assert_eq!(lockfile.version, 1);
        assert!(lockfile.root.is_some());
        assert_eq!(lockfile.packages.len(), 1);
        assert_eq!(lockfile.packages[0].name, "json");
    }
}
