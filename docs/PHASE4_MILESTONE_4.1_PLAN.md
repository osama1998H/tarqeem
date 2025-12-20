# Milestone 4.1: Package Manager (مدير الحزم) - Implementation Plan

## Overview

This document provides a detailed implementation plan for Tarqeem's package manager (trqpm/حزم), the first milestone of Phase 4 (Tooling).

## Prerequisites

| Requirement | Status | Notes |
|-------------|--------|-------|
| Compiler working | ✅ | Phases 1-2 complete |
| Module system | ✅ | `استورد/import` functional |
| Standard library | ✅ | Phase 3 complete |
| 108+ tests passing | ✅ | All tests pass |

## Dependencies to Add

Add to `Cargo.toml`:

```toml
[dependencies]
# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Package management
semver = "1.0"
sha2 = "0.10"

# HTTP client (for registry)
reqwest = { version = "0.12", features = ["json", "blocking"] }

# Async runtime
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

# Compression
flate2 = "1.0"
tar = "0.4"

# File utilities
dirs = "5.0"
walkdir = "2.5"
```

## Implementation Phases

### Phase 4.1.1: Core Infrastructure (Days 1-3)

**Goal**: Create the package module structure and manifest parsing

#### Files to Create

```
src/
├── package/                       # NEW: Package management core
│   ├── mod.rs                     # Module exports
│   ├── manifest.rs                # حزمة.toml parsing
│   ├── lockfile.rs                # .trqlock file handling
│   ├── version.rs                 # Semver utilities
│   └── error.rs                   # Package-specific errors
```

#### 1. Package Manifest Structure (`src/package/manifest.rs`)

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Package manifest (حزمة.toml / trq.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Package metadata (Arabic section)
    #[serde(rename = "حزمة", alias = "package")]
    pub package: PackageInfo,

    /// Dependencies
    #[serde(rename = "اعتماديات", alias = "dependencies", default)]
    pub dependencies: HashMap<String, DependencySpec>,

    /// Dev dependencies
    #[serde(rename = "اعتماديات-تطوير", alias = "dev-dependencies", default)]
    pub dev_dependencies: HashMap<String, DependencySpec>,

    /// Build scripts
    #[serde(rename = "سكربتات", alias = "scripts", default)]
    pub scripts: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(rename = "مؤلف", alias = "author", alias = "authors", default)]
    pub authors: Vec<String>,

    /// License identifier
    #[serde(rename = "رخصة", alias = "license", default)]
    pub license: Option<String>,

    /// Repository URL
    #[serde(rename = "مستودع", alias = "repository", default)]
    pub repository: Option<String>,

    /// Keywords for search
    #[serde(rename = "كلمات", alias = "keywords", default)]
    pub keywords: Vec<String>,

    /// Entry point for binaries
    #[serde(rename = "مدخل", alias = "entry", default)]
    pub entry: Option<String>,

    /// Library entry point
    #[serde(rename = "مكتبة", alias = "lib", default)]
    pub lib: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    /// Simple version string: "1.0.0"
    Version(String),

    /// Detailed spec
    Detailed(DetailedDependency),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedDependency {
    #[serde(rename = "نسخة", alias = "version")]
    pub version: String,

    #[serde(rename = "مسار", alias = "path", default)]
    pub path: Option<PathBuf>,

    #[serde(rename = "git", default)]
    pub git: Option<String>,

    #[serde(rename = "فرع", alias = "branch", default)]
    pub branch: Option<String>,

    #[serde(rename = "اختياري", alias = "optional", default)]
    pub optional: bool,
}

impl Manifest {
    /// Find and parse manifest from current directory or parents
    pub fn find_and_parse() -> Result<(Self, PathBuf), PackageError> {
        let manifest_names = ["حزمة.toml", "trq.toml"];
        let mut current = std::env::current_dir()?;

        loop {
            for name in &manifest_names {
                let manifest_path = current.join(name);
                if manifest_path.exists() {
                    let content = std::fs::read_to_string(&manifest_path)?;
                    let manifest: Manifest = toml::from_str(&content)?;
                    return Ok((manifest, manifest_path));
                }
            }

            if !current.pop() {
                return Err(PackageError::ManifestNotFound);
            }
        }
    }

    /// Parse manifest from a specific path
    pub fn parse(path: &Path) -> Result<Self, PackageError> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Manifest = toml::from_str(&content)?;
        Ok(manifest)
    }

    /// Create a new manifest with defaults
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            package: PackageInfo {
                name: name.to_string(),
                version: version.to_string(),
                description: None,
                authors: vec![],
                license: Some("MIT".to_string()),
                repository: None,
                keywords: vec![],
                entry: Some("مصدر/رئيسي.trq".to_string()),
                lib: None,
            },
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            scripts: HashMap::new(),
        }
    }

    /// Save manifest to file
    pub fn save(&self, path: &Path) -> Result<(), PackageError> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
```

#### 2. Lock File Structure (`src/package/lockfile.rs`)

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Lock file for deterministic builds (.trqlock)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFile {
    /// Lock file format version
    pub version: u32,

    /// Locked packages
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    /// Package name
    pub name: String,

    /// Exact version
    pub version: String,

    /// Source (registry, git, or path)
    pub source: String,

    /// SHA256 checksum
    pub checksum: String,

    /// Dependencies of this package
    pub dependencies: Vec<String>,
}

impl LockFile {
    pub const FILENAME: &'static str = ".trqlock";

    pub fn new() -> Self {
        Self {
            version: 1,
            packages: vec![],
        }
    }

    pub fn parse(path: &Path) -> Result<Self, PackageError> {
        let content = std::fs::read_to_string(path)?;
        let lockfile: LockFile = toml::from_str(&content)?;
        Ok(lockfile)
    }

    pub fn save(&self, path: &Path) -> Result<(), PackageError> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn get_package(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }
}
```

#### 3. Package Errors (`src/package/error.rs`)

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("Manifest not found (حزمة.toml or trq.toml) / لم يتم العثور على ملف الحزمة")]
    ManifestNotFound,

    #[error("Invalid manifest: {0} / ملف حزمة غير صالح: {0}")]
    InvalidManifest(String),

    #[error("Package not found: {0} / الحزمة غير موجودة: {0}")]
    PackageNotFound(String),

    #[error("Version not found: {0}@{1} / النسخة غير موجودة: {0}@{1}")]
    VersionNotFound(String, String),

    #[error("Dependency conflict: {0} / تعارض في الاعتماديات: {0}")]
    DependencyConflict(String),

    #[error("Circular dependency: {0} / اعتمادية دائرية: {0}")]
    CircularDependency(String),

    #[error("Network error: {0} / خطأ في الشبكة: {0}")]
    NetworkError(String),

    #[error("IO error: {0} / خطأ في الإدخال/الإخراج: {0}")]
    IoError(#[from] std::io::Error),

    #[error("TOML parse error: {0} / خطأ في تحليل TOML: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("Checksum mismatch for {0} / عدم تطابق في التحقق للحزمة {0}")]
    ChecksumMismatch(String),

    #[error("Already initialized / تمت التهيئة مسبقاً")]
    AlreadyInitialized,
}
```

---

### Phase 4.1.2: CLI Commands (Days 4-6)

**Goal**: Add package manager subcommands to the CLI

#### Files to Create/Modify

```
src/
├── cli/
│   ├── mod.rs                     # MODIFY: Add Pkg subcommand
│   ├── commands.rs                # MODIFY: Add pkg command handlers
│   └── pm/                        # NEW: Package manager CLI
│       ├── mod.rs
│       ├── init.rs
│       ├── add.rs
│       ├── remove.rs
│       ├── install.rs
│       ├── build.rs
│       └── run.rs
```

#### 1. Add to CLI enum (`src/cli/mod.rs`)

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing commands ...

    /// Package management commands
    #[command(aliases = ["حزم", "pm", "pkg"])]
    Pkg {
        #[command(subcommand)]
        command: PkgCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum PkgCommands {
    /// Initialize a new package
    #[command(aliases = ["هيئ", "أنشئ"])]
    Init {
        /// Package name
        name: Option<String>,

        /// Create library package
        #[arg(long, short = 'l', aliases = ["مكتبة"])]
        lib: bool,
    },

    /// Add a dependency
    #[command(aliases = ["أضف"])]
    Add {
        /// Package name with optional version (e.g., "json@1.0")
        package: String,

        /// Add as dev dependency
        #[arg(long, short = 'd', aliases = ["تطوير"])]
        dev: bool,
    },

    /// Remove a dependency
    #[command(aliases = ["احذف", "أزل"])]
    Remove {
        /// Package name
        package: String,
    },

    /// Install dependencies
    #[command(aliases = ["ثبت"])]
    Install {
        /// Force reinstall
        #[arg(long, short = 'f', aliases = ["أجبر"])]
        force: bool,
    },

    /// Update dependencies
    #[command(aliases = ["حدث"])]
    Update {
        /// Specific package to update
        package: Option<String>,
    },

    /// Build the package
    #[command(aliases = ["ابنِ"])]
    Build {
        /// Build in release mode
        #[arg(long, short = 'r', aliases = ["إصدار"])]
        release: bool,
    },

    /// Run the package
    #[command(aliases = ["شغل"])]
    Run {
        /// Arguments to pass to the program
        args: Vec<String>,
    },

    /// Run tests
    #[command(aliases = ["اختبر"])]
    Test {
        /// Test filter
        filter: Option<String>,
    },

    /// Search for packages
    #[command(aliases = ["ابحث"])]
    Search {
        /// Search query
        query: String,
    },

    /// Show package info
    #[command(aliases = ["معلومات"])]
    Info {
        /// Package name
        package: String,
    },
}
```

#### 2. Init Command (`src/cli/pm/init.rs`)

```rust
use crate::package::{Manifest, PackageError};
use colored::*;
use std::fs;
use std::path::Path;

pub fn run(name: Option<String>, lib: bool) -> Result<(), PackageError> {
    // Check if already initialized
    if Path::new("حزمة.toml").exists() || Path::new("trq.toml").exists() {
        return Err(PackageError::AlreadyInitialized);
    }

    // Get package name from argument or directory
    let pkg_name = name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "my-package".to_string())
    });

    // Create manifest
    let mut manifest = Manifest::new(&pkg_name, "0.1.0");

    if lib {
        manifest.package.lib = Some("مصدر/lib.trq".to_string());
        manifest.package.entry = None;
    }

    // Create directory structure
    let dirs = if lib {
        vec!["مصدر", "اختبارات", "أمثلة", "توثيق"]
    } else {
        vec!["مصدر", "اختبارات"]
    };

    for dir in &dirs {
        fs::create_dir_all(dir)?;
    }

    // Create entry file
    let entry_content = if lib {
        r#"/// مكتبة ترقيم
/// Tarqeem library

صدّر دالة مرحبا() -> نص {
    أرجع "مرحباً من المكتبة!"
}
"#
    } else {
        r#"/// برنامج ترقيم رئيسي
/// Main Tarqeem program

دالة رئيسي() {
    اطبع("مرحباً بالعالم!")
}
"#
    };

    let entry_path = if lib { "مصدر/lib.trq" } else { "مصدر/رئيسي.trq" };
    if !Path::new(entry_path).exists() {
        fs::write(entry_path, entry_content)?;
    }

    // Save manifest
    manifest.save(Path::new("حزمة.toml"))?;

    // Create .gitignore
    let gitignore = "# Tarqeem build artifacts\n/بناء/\n/build/\n/.trqlock\n\n# IDE\n.vscode/\n.idea/\n";
    fs::write(".gitignore", gitignore)?;

    println!("{}", format!("✓ تم إنشاء الحزمة '{}' / Created package '{}'", pkg_name).green());
    println!("  → {}", "مصدر/رئيسي.trq".cyan());
    println!("  → {}", "حزمة.toml".cyan());

    Ok(())
}
```

#### 3. Add Command (`src/cli/pm/add.rs`)

```rust
use crate::package::{Manifest, DependencySpec, PackageError};
use colored::*;

pub fn run(package: String, dev: bool) -> Result<(), PackageError> {
    // Find and parse manifest
    let (mut manifest, manifest_path) = Manifest::find_and_parse()?;

    // Parse package@version format
    let (name, version) = if let Some(idx) = package.find('@') {
        let (n, v) = package.split_at(idx);
        (n.to_string(), v[1..].to_string())
    } else {
        // TODO: Fetch latest version from registry
        (package, "latest".to_string())
    };

    // Add to appropriate section
    let dep_map = if dev {
        &mut manifest.dev_dependencies
    } else {
        &mut manifest.dependencies
    };

    dep_map.insert(name.clone(), DependencySpec::Version(version.clone()));

    // Save manifest
    manifest.save(&manifest_path)?;

    let section = if dev { "dev-dependencies" } else { "dependencies" };
    println!(
        "{}",
        format!("✓ تمت إضافة {}@{} إلى {} / Added {}@{} to {}",
                name, version, section, name, version, section).green()
    );

    // Trigger install
    println!("{}", "→ جاري تثبيت الاعتماديات... / Installing dependencies...".cyan());
    super::install::run(false)?;

    Ok(())
}
```

#### 4. Install Command (`src/cli/pm/install.rs`)

```rust
use crate::package::{Manifest, LockFile, PackageError, resolver::Resolver, cache::Cache};
use colored::*;

pub fn run(force: bool) -> Result<(), PackageError> {
    // Find and parse manifest
    let (manifest, manifest_path) = Manifest::find_and_parse()?;
    let project_root = manifest_path.parent().unwrap();

    // Check lockfile
    let lockfile_path = project_root.join(LockFile::FILENAME);
    let mut lockfile = if lockfile_path.exists() && !force {
        LockFile::parse(&lockfile_path)?
    } else {
        LockFile::new()
    };

    // Initialize resolver and cache
    let cache = Cache::new()?;
    let mut resolver = Resolver::new(&cache);

    // Collect all dependencies
    let all_deps: Vec<_> = manifest.dependencies.iter()
        .chain(manifest.dev_dependencies.iter())
        .collect();

    if all_deps.is_empty() {
        println!("{}", "لا توجد اعتماديات / No dependencies".yellow());
        return Ok(());
    }

    println!("{}", format!("→ جاري تحليل {} اعتمادية... / Resolving {} dependencies...",
                           all_deps.len(), all_deps.len()).cyan());

    // Resolve dependencies
    let resolved = resolver.resolve(&manifest)?;

    // Download and install packages
    for pkg in &resolved {
        if let Some(locked) = lockfile.get_package(&pkg.name) {
            if locked.version == pkg.version && !force {
                println!("  {} {}@{}", "✓".green(), pkg.name, pkg.version);
                continue;
            }
        }

        println!("  {} {}@{}...", "↓".cyan(), pkg.name, pkg.version);
        cache.download_and_verify(pkg)?;

        // Update lockfile
        lockfile.packages.push(pkg.to_locked());
    }

    // Save lockfile
    lockfile.save(&lockfile_path)?;

    // Link packages to node_modules style directory
    let packages_dir = project_root.join("packages");
    cache.link_packages(&resolved, &packages_dir)?;

    println!("{}", format!("✓ تم تثبيت {} حزمة / Installed {} packages",
                           resolved.len(), resolved.len()).green());

    Ok(())
}
```

---

### Phase 4.1.3: Dependency Resolution (Days 7-9)

**Goal**: Implement semver-based dependency resolution

#### Files to Create

```
src/package/
├── resolver.rs                    # Dependency resolver
├── cache.rs                       # Package cache
└── registry.rs                    # Registry client
```

#### 1. Dependency Resolver (`src/package/resolver.rs`)

```rust
use crate::package::{Manifest, DependencySpec, PackageError};
use semver::{Version, VersionReq};
use std::collections::{HashMap, HashSet};

/// Resolved package info
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: String,
    pub checksum: String,
    pub source: PackageSource,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum PackageSource {
    Registry(String),  // URL
    Git { url: String, branch: Option<String> },
    Path(PathBuf),
}

pub struct Resolver<'a> {
    cache: &'a Cache,
    resolved: HashMap<String, ResolvedPackage>,
    resolving: HashSet<String>,  // For cycle detection
}

impl<'a> Resolver<'a> {
    pub fn new(cache: &'a Cache) -> Self {
        Self {
            cache,
            resolved: HashMap::new(),
            resolving: HashSet::new(),
        }
    }

    pub fn resolve(&mut self, manifest: &Manifest) -> Result<Vec<ResolvedPackage>, PackageError> {
        // Resolve all dependencies
        for (name, spec) in &manifest.dependencies {
            self.resolve_package(name, spec)?;
        }

        for (name, spec) in &manifest.dev_dependencies {
            self.resolve_package(name, spec)?;
        }

        // Return topologically sorted packages
        Ok(self.topological_sort())
    }

    fn resolve_package(&mut self, name: &str, spec: &DependencySpec) -> Result<(), PackageError> {
        // Skip if already resolved
        if self.resolved.contains_key(name) {
            return Ok(());
        }

        // Detect cycles
        if self.resolving.contains(name) {
            return Err(PackageError::CircularDependency(name.to_string()));
        }
        self.resolving.insert(name.to_string());

        // Get version requirement
        let version_req = match spec {
            DependencySpec::Version(v) => {
                if v == "latest" {
                    VersionReq::STAR
                } else {
                    VersionReq::parse(v).map_err(|e|
                        PackageError::InvalidManifest(format!("Invalid version: {}", e)))?
                }
            }
            DependencySpec::Detailed(d) => {
                VersionReq::parse(&d.version).map_err(|e|
                    PackageError::InvalidManifest(format!("Invalid version: {}", e)))?
            }
        };

        // Fetch available versions from registry/cache
        let pkg_info = self.cache.get_package_info(name)?;

        // Find best matching version
        let best_version = pkg_info.versions.iter()
            .filter(|v| version_req.matches(&Version::parse(&v.version).unwrap()))
            .max_by_key(|v| Version::parse(&v.version).unwrap())
            .ok_or_else(|| PackageError::VersionNotFound(
                name.to_string(),
                version_req.to_string()
            ))?;

        // Resolve transitive dependencies
        for (dep_name, dep_spec) in &best_version.dependencies {
            self.resolve_package(dep_name, &DependencySpec::Version(dep_spec.clone()))?;
        }

        // Store resolved package
        self.resolved.insert(name.to_string(), ResolvedPackage {
            name: name.to_string(),
            version: best_version.version.clone(),
            checksum: best_version.checksum.clone(),
            source: PackageSource::Registry(pkg_info.registry_url.clone()),
            dependencies: best_version.dependencies.keys().cloned().collect(),
        });

        self.resolving.remove(name);
        Ok(())
    }

    fn topological_sort(&self) -> Vec<ResolvedPackage> {
        let mut result = vec![];
        let mut visited = HashSet::new();

        for name in self.resolved.keys() {
            self.visit_package(name, &mut visited, &mut result);
        }

        result
    }

    fn visit_package(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        result: &mut Vec<ResolvedPackage>
    ) {
        if visited.contains(name) {
            return;
        }
        visited.insert(name.to_string());

        if let Some(pkg) = self.resolved.get(name) {
            for dep in &pkg.dependencies {
                self.visit_package(dep, visited, result);
            }
            result.push(pkg.clone());
        }
    }
}
```

#### 2. Package Cache (`src/package/cache.rs`)

```rust
use crate::package::{PackageError, ResolvedPackage};
use sha2::{Sha256, Digest};
use std::path::{Path, PathBuf};
use std::fs;

/// Local package cache
pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new() -> Result<Self, PackageError> {
        let root = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tarqeem")
            .join("packages");

        fs::create_dir_all(&root)?;

        Ok(Self { root })
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
    pub fn download_and_verify(&self, pkg: &ResolvedPackage) -> Result<PathBuf, PackageError> {
        let pkg_path = self.get_package_path(&pkg.name, &pkg.version);

        // Skip if already cached
        if pkg_path.exists() {
            return Ok(pkg_path);
        }

        // Download from source
        let tarball = match &pkg.source {
            PackageSource::Registry(url) => {
                self.download_from_registry(url, &pkg.name, &pkg.version)?
            }
            PackageSource::Git { url, branch } => {
                self.clone_from_git(url, branch.as_deref())?
            }
            PackageSource::Path(path) => {
                // Copy from local path
                fs::read(path)?
            }
        };

        // Verify checksum
        let computed_hash = hex::encode(Sha256::digest(&tarball));
        if computed_hash != pkg.checksum {
            return Err(PackageError::ChecksumMismatch(pkg.name.clone()));
        }

        // Extract to cache
        fs::create_dir_all(&pkg_path)?;
        self.extract_tarball(&tarball, &pkg_path)?;

        Ok(pkg_path)
    }

    /// Link packages to project directory
    pub fn link_packages(
        &self,
        packages: &[ResolvedPackage],
        target: &Path
    ) -> Result<(), PackageError> {
        fs::create_dir_all(target)?;

        for pkg in packages {
            let source = self.get_package_path(&pkg.name, &pkg.version);
            let link = target.join(&pkg.name);

            // Remove existing link
            if link.exists() {
                fs::remove_dir_all(&link)?;
            }

            // Create symlink or copy
            #[cfg(unix)]
            std::os::unix::fs::symlink(&source, &link)?;

            #[cfg(windows)]
            fs_extra::dir::copy(&source, &link, &Default::default())?;
        }

        Ok(())
    }

    fn download_from_registry(&self, url: &str, name: &str, version: &str) -> Result<Vec<u8>, PackageError> {
        let tarball_url = format!("{}/api/v1/packages/{}/{}/tarball", url, name, version);

        let response = reqwest::blocking::get(&tarball_url)
            .map_err(|e| PackageError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(PackageError::PackageNotFound(format!("{}@{}", name, version)));
        }

        response.bytes()
            .map(|b| b.to_vec())
            .map_err(|e| PackageError::NetworkError(e.to_string()))
    }

    fn extract_tarball(&self, data: &[u8], target: &Path) -> Result<(), PackageError> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let decoder = GzDecoder::new(data);
        let mut archive = Archive::new(decoder);
        archive.unpack(target)?;

        Ok(())
    }
}
```

---

### Phase 4.1.4: ModuleLoader Integration (Days 10-11)

**Goal**: Connect package manager with the existing module system

#### Files to Modify

```
src/
├── semantic/
│   ├── modules.rs                 # MODIFY: Add package search paths
│   └── analyzer.rs                # MODIFY: Configure package paths
├── cli/
│   └── commands.rs                # MODIFY: Initialize package paths
```

#### 1. Enhance ModuleLoader (`src/semantic/modules.rs`)

```rust
impl ModuleLoader {
    /// Add package directory to search path
    pub fn add_package_path(&mut self, packages_dir: &Path) {
        if packages_dir.exists() {
            for entry in fs::read_dir(packages_dir).unwrap_or_else(|_| panic!()) {
                if let Ok(entry) = entry {
                    if entry.path().is_dir() {
                        self.add_search_path(entry.path());
                    }
                }
            }
        }
    }

    /// Resolve module with package awareness
    pub fn resolve_path_with_packages(
        &self,
        import_path: &str,
        from_file: &Path,
        packages_dir: Option<&Path>
    ) -> Option<PathBuf> {
        // Try relative/absolute first
        if let Some(path) = self.resolve_path(import_path, from_file) {
            return Some(path);
        }

        // Try packages directory
        if let Some(pkg_dir) = packages_dir {
            // Parse package name from import
            let parts: Vec<&str> = import_path.split('/').collect();
            if !parts.is_empty() {
                let pkg_name = parts[0];
                let pkg_path = pkg_dir.join(pkg_name);

                if pkg_path.exists() {
                    let sub_path = if parts.len() > 1 {
                        parts[1..].join("/")
                    } else {
                        "mod.trq".to_string()
                    };

                    return self.try_extensions(&pkg_path.join(sub_path));
                }
            }
        }

        None
    }
}
```

#### 2. Update CLI Commands (`src/cli/commands.rs`)

```rust
fn setup_analyzer(project_root: Option<&Path>) -> Analyzer {
    let mut analyzer = Analyzer::new();

    // Add stdlib path
    if let Some(stdlib_path) = find_stdlib_path() {
        analyzer.add_search_path(stdlib_path);
    }

    // Add packages from project
    if let Some(root) = project_root {
        let packages_dir = root.join("packages");
        if packages_dir.exists() {
            analyzer.module_loader().add_package_path(&packages_dir);
        }
    }

    analyzer
}
```

---

### Phase 4.1.5: Build & Run Commands (Days 12-14)

**Goal**: Complete build and run commands with package awareness

#### Build Command (`src/cli/pm/build.rs`)

```rust
use crate::package::{Manifest, PackageError};
use crate::cli::commands;
use colored::*;
use std::path::Path;

pub fn run(release: bool) -> Result<(), PackageError> {
    let (manifest, manifest_path) = Manifest::find_and_parse()?;
    let project_root = manifest_path.parent().unwrap();

    // Determine entry point
    let entry = manifest.package.entry
        .as_ref()
        .or(manifest.package.lib.as_ref())
        .ok_or_else(|| PackageError::InvalidManifest(
            "No entry point specified (مدخل or entry)".to_string()
        ))?;

    let entry_path = project_root.join(entry);
    if !entry_path.exists() {
        return Err(PackageError::InvalidManifest(
            format!("Entry point not found: {}", entry_path.display())
        ));
    }

    // Create output directory
    let output_dir = project_root.join(if release { "بناء/إصدار" } else { "بناء/تطوير" });
    std::fs::create_dir_all(&output_dir)?;

    // Determine output name
    let output_name = manifest.package.name.clone();
    let output_path = output_dir.join(&output_name);

    println!("{}", format!("→ جاري بناء {}... / Building {}...", output_name, output_name).cyan());

    // Call compile command
    let compile_args = if release {
        vec!["compile", "-O", entry_path.to_str().unwrap(), "-o", output_path.to_str().unwrap()]
    } else {
        vec!["compile", entry_path.to_str().unwrap(), "-o", output_path.to_str().unwrap()]
    };

    commands::compile_with_args(&compile_args, Some(project_root))?;

    println!("{}", format!("✓ تم البناء: {} / Built: {}",
                           output_path.display(), output_path.display()).green());

    Ok(())
}
```

---

## Testing Plan

### Unit Tests

```rust
// tests/package/manifest_tests.rs
#[test]
fn test_parse_arabic_manifest() {
    let toml = r#"
[حزمة]
اسم = "مكتبتي"
نسخة = "1.0.0"
وصف = "مكتبة رائعة"

[اعتماديات]
json = "2.0"
"#;
    let manifest: Manifest = toml::from_str(toml).unwrap();
    assert_eq!(manifest.package.name, "مكتبتي");
    assert_eq!(manifest.package.version, "1.0.0");
    assert!(manifest.dependencies.contains_key("json"));
}

#[test]
fn test_parse_english_manifest() {
    let toml = r#"
[package]
name = "my-lib"
version = "1.0.0"

[dependencies]
json = "2.0"
"#;
    let manifest: Manifest = toml::from_str(toml).unwrap();
    assert_eq!(manifest.package.name, "my-lib");
}
```

### Integration Tests

```rust
// tests/package/integration_tests.rs
#[test]
fn test_init_creates_structure() {
    let temp = tempfile::tempdir().unwrap();
    std::env::set_current_dir(&temp).unwrap();

    super::init::run(Some("test-pkg".to_string()), false).unwrap();

    assert!(Path::new("حزمة.toml").exists());
    assert!(Path::new("مصدر/رئيسي.trq").exists());
}

#[test]
fn test_add_updates_manifest() {
    // Setup
    init_test_project();

    super::add::run("json@1.0".to_string(), false).unwrap();

    let manifest = Manifest::find_and_parse().unwrap().0;
    assert!(manifest.dependencies.contains_key("json"));
}
```

---

## Success Criteria

### Milestone 4.1 Complete When:

1. **`trqpm init`** works:
   - Creates `حزمة.toml` with Arabic keys
   - Creates `مصدر/رئيسي.trq` or `مصدر/lib.trq`
   - Creates directory structure

2. **`trqpm add`** works:
   - Parses `package@version` format
   - Updates manifest dependencies
   - Triggers install

3. **`trqpm install`** works:
   - Resolves dependency tree
   - Downloads packages (or uses local cache)
   - Creates `.trqlock` file
   - Links packages to `packages/` directory

4. **`trqpm build`** works:
   - Finds and reads manifest
   - Compiles entry point
   - Outputs to `بناء/` directory

5. **`trqpm run`** works:
   - Builds if necessary
   - Executes the built binary

6. **Module integration** works:
   - `استورد { X } من "package-name"` finds packages
   - Packages in `packages/` are searchable

---

## Risk Assessment

| Risk | Mitigation |
|------|-----------|
| Registry not available | Implement offline mode with local cache |
| TOML parsing issues with Arabic | Use serde aliases for bilingual support |
| Path issues on Windows | Use `PathBuf` and proper separators |
| Symlink issues on Windows | Fall back to copy on Windows |
| Circular dependencies | Early detection with resolving stack |

---

## Timeline Summary

| Phase | Days | Description |
|-------|------|-------------|
| 4.1.1 | 1-3 | Core infrastructure (manifest, lockfile) |
| 4.1.2 | 4-6 | CLI commands |
| 4.1.3 | 7-9 | Dependency resolution |
| 4.1.4 | 10-11 | ModuleLoader integration |
| 4.1.5 | 12-14 | Build/run commands, testing |

**Total**: ~14 days / 2-3 weeks

---

## Next Steps After 4.1

After completing the package manager, proceed to:

1. **Milestone 4.2**: LSP Server
2. **Milestone 4.3**: VS Code Extension
3. **Milestone 4.4**: Documentation Generator

---

## References

- [PHASE4_PLAN.md](./PHASE4_PLAN.md) - Full Phase 4 roadmap
- [ARCHITECTURE.md](../ARCHITECTURE.md) - Project architecture
- [Cargo.toml](../Cargo.toml) - Dependencies reference
- [src/semantic/modules.rs](../src/semantic/modules.rs) - Existing module loader
