//! Dependency resolution with semver support
//!
//! Resolves package dependencies using semantic versioning,
//! with cycle detection and topological sorting.

use super::cache::Cache;
use super::error::{PackageError, PackageResult};
use super::lockfile::{LockedPackage, PackageSource};
use super::manifest::{DependencySpec, Manifest};
use semver::{Version, VersionReq};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// A resolved package with all details needed for installation
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    /// Package name
    pub name: String,

    /// Exact version to use
    pub version: String,

    /// SHA256 checksum
    pub checksum: String,

    /// Where to get the package
    pub source: ResolvedSource,

    /// Dependencies of this package
    pub dependencies: Vec<String>,
}

/// Source of a resolved package
#[derive(Debug, Clone)]
pub enum ResolvedSource {
    /// From the package registry
    Registry { url: String, tarball_url: String },

    /// From a git repository
    Git {
        url: String,
        branch: Option<String>,
        tag: Option<String>,
        rev: Option<String>,
    },

    /// From a local path
    Path(PathBuf),
}

impl ResolvedPackage {
    /// Convert to a locked package
    pub fn to_locked(&self) -> LockedPackage {
        let source = match &self.source {
            ResolvedSource::Registry { url, .. } => PackageSource::Registry { url: url.clone() },
            ResolvedSource::Git {
                url,
                branch,
                tag,
                rev,
            } => {
                let reference = if let Some(branch) = branch {
                    super::lockfile::GitReference::Branch {
                        branch: branch.clone(),
                    }
                } else if let Some(tag) = tag {
                    super::lockfile::GitReference::Tag { tag: tag.clone() }
                } else if let Some(rev) = rev {
                    super::lockfile::GitReference::Rev { rev: rev.clone() }
                } else {
                    super::lockfile::GitReference::Branch {
                        branch: "main".to_string(),
                    }
                };
                PackageSource::Git {
                    url: url.clone(),
                    reference,
                }
            }
            ResolvedSource::Path(path) => PackageSource::Path {
                path: path.to_string_lossy().to_string(),
            },
        };

        let mut locked = LockedPackage {
            name: self.name.clone(),
            version: self.version.clone(),
            source,
            checksum: self.checksum.clone(),
            dependencies: HashMap::new(),
        };

        for dep in &self.dependencies {
            locked.dependencies.insert(dep.clone(), "*".to_string());
        }

        locked
    }
}

/// Package information from registry or cache
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackageInfo {
    /// Package name
    pub name: String,

    /// Available versions
    pub versions: Vec<VersionInfo>,

    /// Registry URL
    pub registry_url: String,
}

/// Version information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionInfo {
    /// Version string
    pub version: String,

    /// SHA256 checksum
    pub checksum: String,

    /// Tarball download URL
    pub tarball_url: String,

    /// Dependencies (name -> version requirement)
    pub dependencies: HashMap<String, String>,
}

/// Dependency resolver
pub struct Resolver<'a> {
    /// Package cache
    cache: &'a Cache,

    /// Already resolved packages
    resolved: HashMap<String, ResolvedPackage>,

    /// Currently resolving (for cycle detection)
    resolving: HashSet<String>,
}

impl<'a> Resolver<'a> {
    /// Create a new resolver
    pub fn new(cache: &'a Cache) -> Self {
        Self {
            cache,
            resolved: HashMap::new(),
            resolving: HashSet::new(),
        }
    }

    /// Resolve all dependencies from a manifest
    pub fn resolve(&mut self, manifest: &Manifest) -> PackageResult<Vec<ResolvedPackage>> {
        // Resolve regular dependencies
        for (name, spec) in &manifest.dependencies {
            self.resolve_package(name, spec)?;
        }

        // Resolve dev dependencies
        for (name, spec) in &manifest.dev_dependencies {
            self.resolve_package(name, spec)?;
        }

        // Return topologically sorted packages
        Ok(self.topological_sort())
    }

    /// Resolve a single package and its dependencies
    fn resolve_package(&mut self, name: &str, spec: &DependencySpec) -> PackageResult<()> {
        // Skip if already resolved
        if self.resolved.contains_key(name) {
            return Ok(());
        }

        // Detect cycles
        if self.resolving.contains(name) {
            return Err(PackageError::CircularDependency(name.to_string()));
        }
        self.resolving.insert(name.to_string());

        // Handle path dependencies specially
        if let Some(path) = spec.path() {
            let resolved = self.resolve_path_dependency(name, path, spec.version_req())?;
            self.resolved.insert(name.to_string(), resolved);
            self.resolving.remove(name);
            return Ok(());
        }

        // Parse version requirement
        let version_req = self.parse_version_req(spec.version_req())?;

        // Get package info from cache/registry
        let pkg_info = self.cache.get_package_info(name)?;

        // Find best matching version and clone it to avoid borrow issues
        let best_version = self.find_best_version(&pkg_info, &version_req)?;

        // Store resolved package
        let resolved = ResolvedPackage {
            name: name.to_string(),
            version: best_version.version.clone(),
            checksum: best_version.checksum.clone(),
            source: ResolvedSource::Registry {
                url: pkg_info.registry_url.clone(),
                tarball_url: best_version.tarball_url.clone(),
            },
            dependencies: best_version.dependencies.keys().cloned().collect(),
        };

        // Get dependencies before inserting
        let deps: Vec<_> = best_version
            .dependencies
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        self.resolved.insert(name.to_string(), resolved);

        // Resolve transitive dependencies after inserting current package
        for (dep_name, dep_version) in deps {
            let dep_spec = DependencySpec::Version(dep_version);
            self.resolve_package(&dep_name, &dep_spec)?;
        }

        self.resolving.remove(name);

        Ok(())
    }

    /// Resolve a path dependency
    fn resolve_path_dependency(
        &self,
        name: &str,
        path: &PathBuf,
        _version_req: &str,
    ) -> PackageResult<ResolvedPackage> {
        // Read manifest from the path
        let manifest_path = path.join("حزمة.toml");
        let manifest = if manifest_path.exists() {
            Manifest::parse(&manifest_path)?
        } else {
            let alt_path = path.join("trq.toml");
            if alt_path.exists() {
                Manifest::parse(&alt_path)?
            } else {
                return Err(PackageError::ManifestNotFound);
            }
        };

        // Calculate checksum of the directory (simplified: hash of manifest)
        let checksum = self.cache.calculate_path_checksum(path)?;

        Ok(ResolvedPackage {
            name: name.to_string(),
            version: manifest.package.version,
            checksum,
            source: ResolvedSource::Path(path.clone()),
            dependencies: manifest.dependencies.keys().cloned().collect(),
        })
    }

    /// Parse a version requirement string
    fn parse_version_req(&self, version_str: &str) -> PackageResult<VersionReq> {
        if version_str == "latest" || version_str == "*" {
            return Ok(VersionReq::STAR);
        }

        VersionReq::parse(version_str)
            .map_err(|e| PackageError::InvalidVersion(format!("{}: {}", version_str, e)))
    }

    /// Find the best matching version for a requirement
    fn find_best_version<'b>(
        &self,
        pkg_info: &'b PackageInfo,
        version_req: &VersionReq,
    ) -> PackageResult<&'b VersionInfo> {
        pkg_info
            .versions
            .iter()
            .filter(|v| {
                Version::parse(&v.version)
                    .map(|ver| version_req.matches(&ver))
                    .unwrap_or(false)
            })
            .max_by(|a, b| {
                let va = Version::parse(&a.version).unwrap_or_else(|_| Version::new(0, 0, 0));
                let vb = Version::parse(&b.version).unwrap_or_else(|_| Version::new(0, 0, 0));
                va.cmp(&vb)
            })
            .ok_or_else(|| PackageError::VersionNotFound {
                name: pkg_info.name.clone(),
                version: version_req.to_string(),
            })
    }

    /// Topologically sort resolved packages
    fn topological_sort(&self) -> Vec<ResolvedPackage> {
        let mut result = vec![];
        let mut visited = HashSet::new();

        for name in self.resolved.keys() {
            self.visit_package(name, &mut visited, &mut result);
        }

        result
    }

    /// Visit a package for topological sort (DFS)
    fn visit_package(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        result: &mut Vec<ResolvedPackage>,
    ) {
        if visited.contains(name) {
            return;
        }
        visited.insert(name.to_string());

        if let Some(pkg) = self.resolved.get(name) {
            // Visit dependencies first
            for dep in &pkg.dependencies {
                self.visit_package(dep, visited, result);
            }
            result.push(pkg.clone());
        }
    }

    /// Get the number of resolved packages
    pub fn resolved_count(&self) -> usize {
        self.resolved.len()
    }

    /// Check if a package is resolved
    pub fn is_resolved(&self, name: &str) -> bool {
        self.resolved.contains_key(name)
    }

    /// Get a resolved package
    pub fn get_resolved(&self, name: &str) -> Option<&ResolvedPackage> {
        self.resolved.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_req() {
        let cache = Cache::new().unwrap();
        let resolver = Resolver::new(&cache);

        // Test various version requirements
        assert!(resolver.parse_version_req("1.0.0").is_ok());
        assert!(resolver.parse_version_req("^1.0").is_ok());
        assert!(resolver.parse_version_req("~1.0").is_ok());
        assert!(resolver.parse_version_req(">=1.0, <2.0").is_ok());
        assert!(resolver.parse_version_req("*").is_ok());
        assert!(resolver.parse_version_req("latest").is_ok());
    }

    #[test]
    fn test_resolved_package_to_locked() {
        let resolved = ResolvedPackage {
            name: "json".to_string(),
            version: "1.0.0".to_string(),
            checksum: "abc123".to_string(),
            source: ResolvedSource::Registry {
                url: "https://registry.tarqeem.dev".to_string(),
                tarball_url: "https://registry.tarqeem.dev/json/1.0.0.tar.gz".to_string(),
            },
            dependencies: vec!["utils".to_string()],
        };

        let locked = resolved.to_locked();
        assert_eq!(locked.name, "json");
        assert_eq!(locked.version, "1.0.0");
        assert!(locked.source.is_registry());
    }
}
