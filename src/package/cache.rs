//! Package cache management
//!
//! Handles downloading, caching, and linking packages.

use super::error::{PackageError, PackageResult};
use super::resolver::{PackageInfo, ResolvedPackage, ResolvedSource, VersionInfo};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Default registry URL
pub const DEFAULT_REGISTRY: &str = "https://registry.tarqeem.dev";

/// Local package cache
pub struct Cache {
    /// Root cache directory (~/.cache/tarqeem/packages)
    root: PathBuf,

    /// Registry URL
    registry_url: String,

    /// In-memory package info cache
    package_cache: HashMap<String, PackageInfo>,
}

impl Cache {
    /// Create a new cache instance
    pub fn new() -> PackageResult<Self> {
        let root = Self::default_cache_dir()?;
        fs::create_dir_all(&root)?;

        Ok(Self {
            root,
            registry_url: DEFAULT_REGISTRY.to_string(),
            package_cache: HashMap::new(),
        })
    }

    /// Create cache with custom directory
    pub fn with_root(root: PathBuf) -> PackageResult<Self> {
        fs::create_dir_all(&root)?;

        Ok(Self {
            root,
            registry_url: DEFAULT_REGISTRY.to_string(),
            package_cache: HashMap::new(),
        })
    }

    /// Get the default cache directory
    pub fn default_cache_dir() -> PackageResult<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("tarqeem")
            .join("packages");

        Ok(cache_dir)
    }

    /// Set the registry URL
    pub fn set_registry(&mut self, url: &str) {
        self.registry_url = url.to_string();
    }

    /// Get package information from cache or registry
    pub fn get_package_info(&self, name: &str) -> PackageResult<PackageInfo> {
        // Check in-memory cache first
        if let Some(info) = self.package_cache.get(name) {
            return Ok(info.clone());
        }

        // Check local index cache
        let index_path = self.root.join("index").join(name).join("versions.json");
        if index_path.exists() {
            let content = fs::read_to_string(&index_path)?;
            if let Ok(info) = serde_json::from_str::<PackageInfo>(&content) {
                return Ok(info);
            }
        }

        // For now, return a "not found" error since we don't have a live registry
        // In a full implementation, we would fetch from the registry here
        Err(PackageError::PackageNotFound(name.to_string()))
    }

    /// Register a local package (for path dependencies and development)
    pub fn register_local_package(&mut self, name: &str, version: &str, path: &Path) {
        let info = PackageInfo {
            name: name.to_string(),
            versions: vec![VersionInfo {
                version: version.to_string(),
                checksum: self.calculate_path_checksum(path).unwrap_or_default(),
                tarball_url: format!("file://{}", path.display()),
                dependencies: HashMap::new(),
            }],
            registry_url: "local".to_string(),
        };

        self.package_cache.insert(name.to_string(), info);
    }

    /// Get path to cached package
    pub fn get_package_path(&self, name: &str, version: &str) -> PathBuf {
        self.root.join(name).join(version)
    }

    /// Check if package is cached
    pub fn is_cached(&self, name: &str, version: &str) -> bool {
        self.get_package_path(name, version).exists()
    }

    /// Download and verify a package
    pub fn download_and_verify(&self, pkg: &ResolvedPackage) -> PackageResult<PathBuf> {
        let pkg_path = self.get_package_path(&pkg.name, &pkg.version);

        // Skip if already cached
        if pkg_path.exists() {
            return Ok(pkg_path);
        }

        match &pkg.source {
            ResolvedSource::Registry { tarball_url, .. } => {
                self.download_from_url(tarball_url, &pkg.checksum, &pkg_path)?;
            }
            ResolvedSource::Git { url, branch, .. } => {
                self.clone_from_git(url, branch.as_deref(), &pkg_path)?;
            }
            ResolvedSource::Path(source_path) => {
                self.copy_from_path(source_path, &pkg_path)?;
            }
        }

        Ok(pkg_path)
    }

    /// Download a package from URL
    fn download_from_url(
        &self,
        url: &str,
        _expected_checksum: &str,
        target: &Path,
    ) -> PackageResult<()> {
        // For now, this is a stub. In production, we'd use reqwest
        // to download the tarball and verify the checksum.

        // Simulate download for local development
        if url.starts_with("file://") {
            let source = PathBuf::from(url.trim_start_matches("file://"));
            return self.copy_from_path(&source, target);
        }

        Err(PackageError::NetworkError(format!(
            "Registry download not implemented. URL: {}",
            url
        )))
    }

    /// Clone a git repository
    fn clone_from_git(&self, url: &str, branch: Option<&str>, target: &Path) -> PackageResult<()> {
        // Create parent directory
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        // Use git command to clone
        let mut cmd = std::process::Command::new("git");
        cmd.arg("clone");

        if let Some(b) = branch {
            cmd.args(["--branch", b]);
        }

        cmd.arg("--depth").arg("1");
        cmd.arg(url);
        cmd.arg(target);

        let output = cmd
            .output()
            .map_err(|e| PackageError::NetworkError(format!("Failed to run git: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PackageError::NetworkError(format!(
                "Git clone failed: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Copy a package from local path
    fn copy_from_path(&self, source: &Path, target: &Path) -> PackageResult<()> {
        if !source.exists() {
            return Err(PackageError::PackageNotFound(
                source.to_string_lossy().to_string(),
            ));
        }

        // Create parent directory
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        // Copy directory recursively
        self.copy_dir_recursive(source, target)?;

        Ok(())
    }

    /// Recursively copy a directory
    fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> PackageResult<()> {
        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                // Skip hidden directories and build artifacts
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with('.') || name_str == "target" || name_str == "بناء" {
                    continue;
                }
                self.copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    /// Link packages to project directory
    pub fn link_packages(&self, packages: &[ResolvedPackage], target: &Path) -> PackageResult<()> {
        fs::create_dir_all(target)?;

        for pkg in packages {
            let source = match &pkg.source {
                ResolvedSource::Path(p) => p.clone(),
                _ => self.get_package_path(&pkg.name, &pkg.version),
            };

            let link_path = target.join(&pkg.name);

            // Remove existing link/directory
            if link_path.exists() {
                if link_path.is_dir() {
                    fs::remove_dir_all(&link_path)?;
                } else {
                    fs::remove_file(&link_path)?;
                }
            }

            // Create symlink or copy
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&source, &link_path)?;
            }

            #[cfg(windows)]
            {
                // On Windows, copy instead of symlink for simplicity
                self.copy_dir_recursive(&source, &link_path)?;
            }
        }

        Ok(())
    }

    /// Calculate checksum of a directory
    pub fn calculate_path_checksum(&self, path: &Path) -> PackageResult<String> {
        let mut hasher = Sha256::new();

        // Hash all files in the directory
        self.hash_directory(path, &mut hasher)?;

        let hash = hasher.finalize();
        Ok(format!("sha256:{}", hex::encode(hash)))
    }

    /// Hash all files in a directory
    fn hash_directory(&self, path: &Path, hasher: &mut Sha256) -> PackageResult<()> {
        if path.is_file() {
            let mut file = fs::File::open(path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            hasher.update(&buffer);
            return Ok(());
        }

        // Get sorted entries for deterministic hashing
        let mut entries: Vec<_> = fs::read_dir(path)?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let entry_path = entry.path();

            // Skip hidden files and build directories
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str == "target" || name_str == "بناء" {
                continue;
            }

            // Hash the file name
            hasher.update(name_str.as_bytes());

            // Recursively hash
            self.hash_directory(&entry_path, hasher)?;
        }

        Ok(())
    }

    /// Extract a tarball to a directory
    pub fn extract_tarball(&self, tarball: &[u8], target: &Path) -> PackageResult<()> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        fs::create_dir_all(target)?;

        let decoder = GzDecoder::new(tarball);
        let mut archive = Archive::new(decoder);
        archive.unpack(target)?;

        Ok(())
    }

    /// Verify a checksum
    pub fn verify_checksum(&self, data: &[u8], expected: &str) -> bool {
        let hash = Sha256::digest(data);
        let computed = format!("sha256:{}", hex::encode(hash));

        // Support both with and without prefix
        if expected.starts_with("sha256:") {
            computed == expected
        } else {
            hex::encode(hash) == expected
        }
    }

    /// Clear cached package
    pub fn clear_package(&self, name: &str) -> PackageResult<()> {
        let pkg_dir = self.root.join(name);
        if pkg_dir.exists() {
            fs::remove_dir_all(&pkg_dir)?;
        }
        Ok(())
    }

    /// Clear entire cache
    pub fn clear_all(&self) -> PackageResult<()> {
        if self.root.exists() {
            fs::remove_dir_all(&self.root)?;
            fs::create_dir_all(&self.root)?;
        }
        Ok(())
    }

    /// Get cache size in bytes
    pub fn cache_size(&self) -> PackageResult<u64> {
        self.dir_size(&self.root)
    }

    /// Calculate directory size recursively
    fn dir_size(&self, path: &Path) -> PackageResult<u64> {
        let mut size = 0;

        if path.is_file() {
            return Ok(fs::metadata(path)?.len());
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                size += self.dir_size(&path)?;
            } else {
                size += fs::metadata(&path)?.len();
            }
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_creation() {
        let temp = tempdir().unwrap();
        let cache = Cache::with_root(temp.path().to_path_buf()).unwrap();
        assert!(cache.root.exists());
    }

    #[test]
    fn test_package_path() {
        let temp = tempdir().unwrap();
        let cache = Cache::with_root(temp.path().to_path_buf()).unwrap();

        let path = cache.get_package_path("json", "1.0.0");
        assert!(path.ends_with("json/1.0.0"));
    }

    #[test]
    fn test_is_cached() {
        let temp = tempdir().unwrap();
        let cache = Cache::with_root(temp.path().to_path_buf()).unwrap();

        assert!(!cache.is_cached("json", "1.0.0"));

        // Create the package directory
        let pkg_path = cache.get_package_path("json", "1.0.0");
        fs::create_dir_all(&pkg_path).unwrap();

        assert!(cache.is_cached("json", "1.0.0"));
    }

    #[test]
    fn test_verify_checksum() {
        let temp = tempdir().unwrap();
        let cache = Cache::with_root(temp.path().to_path_buf()).unwrap();

        let data = b"hello world";
        let hash = Sha256::digest(data);
        let checksum = format!("sha256:{}", hex::encode(hash));

        assert!(cache.verify_checksum(data, &checksum));
        assert!(!cache.verify_checksum(data, "sha256:wrong"));
    }

    #[test]
    fn test_copy_dir_recursive() {
        let temp = tempdir().unwrap();
        let cache = Cache::with_root(temp.path().join("cache")).unwrap();

        // Create source directory with files
        let src = temp.path().join("src");
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("file.txt"), "content").unwrap();
        fs::write(src.join("sub/nested.txt"), "nested").unwrap();

        // Copy
        let dst = temp.path().join("dst");
        cache.copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.join("file.txt").exists());
        assert!(dst.join("sub/nested.txt").exists());
    }

    #[test]
    fn test_register_local_package() {
        let temp = tempdir().unwrap();
        let mut cache = Cache::with_root(temp.path().to_path_buf()).unwrap();

        let pkg_path = temp.path().join("my-pkg");
        fs::create_dir_all(&pkg_path).unwrap();
        fs::write(pkg_path.join("lib.trq"), "// package").unwrap();

        cache.register_local_package("my-pkg", "0.1.0", &pkg_path);

        let info = cache.get_package_info("my-pkg").unwrap();
        assert_eq!(info.name, "my-pkg");
        assert_eq!(info.versions.len(), 1);
        assert_eq!(info.versions[0].version, "0.1.0");
    }
}
