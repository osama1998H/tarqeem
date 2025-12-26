//! Lock file management (ترقيم.قفل)
//!
//! Ensures reproducible builds by locking exact versions.
//!
//! # أسماء الملفات المدعومة
//! - `ترقيم.قفل` (الصيغة العربية الجديدة)

use super::error::PackageResult;
use super::format;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub const LOCKFILE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFile {
    pub version: u32,

    #[serde(default)]
    pub root: Option<RootPackage>,

    #[serde(default)]
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootPackage {
    pub name: String,

    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    pub name: String,

    pub version: String,

    pub source: PackageSource,

    pub checksum: String,

    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PackageSource {
    #[serde(rename = "registry")]
    Registry { url: String },

    #[serde(rename = "git")]
    Git {
        url: String,
        #[serde(flatten)]
        reference: GitReference,
    },

    #[serde(rename = "path")]
    Path { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GitReference {
    Branch { branch: String },
    Tag { tag: String },
    Rev { rev: String },
}

impl LockFile {
    pub const LOCK_NAMES: &'static [&'static str] = &[
        "ترقيم.قفل", // الصيغة العربية الجديدة (أولوية قصوى)
    ];

    pub const DEFAULT_LOCK_NAME: &'static str = "ترقيم.قفل";

    pub fn new() -> Self {
        Self {
            version: LOCKFILE_VERSION,
            root: None,
            packages: vec![],
        }
    }

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

    pub fn find_in_dir(dir: &Path) -> Option<std::path::PathBuf> {
        for name in Self::LOCK_NAMES {
            let path = dir.join(name);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    pub fn parse(path: &Path) -> PackageResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if ext == "قفل" {
            Self::parse_arabic_format(&content)
        } else {
            let lockfile: LockFile = toml::from_str(&content)?;
            Ok(lockfile)
        }
    }

    pub fn parse_arabic_format(content: &str) -> PackageResult<Self> {
        let value = format::parse(content)
            .map_err(|e| super::error::PackageError::InvalidManifest(format!("{}", e)))?;

        let obj = value.as_object().ok_or_else(|| {
            super::error::PackageError::InvalidManifest("القفل يجب أن يكون كائناً".to_string())
        })?;

        let version = obj
            .get("نسخة_الصيغة")
            .or_else(|| obj.get("version"))
            .and_then(|v| v.as_i64())
            .unwrap_or(LOCKFILE_VERSION as i64) as u32;

        let root = if let Some(root_obj) = obj.get("جذر").or_else(|| obj.get("root")) {
            if let Some(root_map) = root_obj.as_object() {
                let name = root_map
                    .get("اسم")
                    .or_else(|| root_map.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let ver = root_map
                    .get("نسخة")
                    .or_else(|| root_map.get("version"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(RootPackage { name, version: ver })
            } else {
                None
            }
        } else {
            None
        };

        // Parse packages array
        let packages = Self::parse_packages_from_arabic(obj)?;

        Ok(Self {
            version,
            root,
            packages,
        })
    }

    fn parse_packages_from_arabic(
        obj: &indexmap::IndexMap<String, format::value::Value>,
    ) -> PackageResult<Vec<LockedPackage>> {
        let mut packages = Vec::new();

        if let Some(pkgs_val) = obj.get("حزم").or_else(|| obj.get("packages")) {
            if let Some(pkgs_arr) = pkgs_val.as_array() {
                for pkg_val in pkgs_arr {
                    if let Some(pkg_obj) = pkg_val.as_object() {
                        let pkg = Self::parse_single_package(pkg_obj)?;
                        packages.push(pkg);
                    }
                }
            }
        }

        Ok(packages)
    }

    fn parse_single_package(
        pkg_obj: &indexmap::IndexMap<String, format::value::Value>,
    ) -> PackageResult<LockedPackage> {
        let name = pkg_obj
            .get("اسم")
            .or_else(|| pkg_obj.get("name"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                super::error::PackageError::InvalidManifest(
                    "اسم الحزمة مطلوب / Package name required".to_string(),
                )
            })?
            .to_string();

        let version = pkg_obj
            .get("نسخة")
            .or_else(|| pkg_obj.get("version"))
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        let checksum = pkg_obj
            .get("تحقق")
            .or_else(|| pkg_obj.get("checksum"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let source = Self::parse_package_source(pkg_obj)?;

        let dependencies = Self::parse_package_dependencies(pkg_obj)?;

        Ok(LockedPackage {
            name,
            version,
            source,
            checksum,
            dependencies,
        })
    }

    fn parse_package_source(
        pkg_obj: &indexmap::IndexMap<String, format::value::Value>,
    ) -> PackageResult<PackageSource> {
        let source_str = pkg_obj
            .get("مصدر")
            .or_else(|| pkg_obj.get("source"))
            .and_then(|v| v.as_str())
            .unwrap_or("سجل (https://registry.tarqeem.dev)");

        // Parse source string format: "سجل (url)" or "git (url ref)" or "مسار (path)"
        if source_str.starts_with("سجل") || source_str.starts_with("registry") {
            let url = Self::extract_parenthesized(source_str);
            Ok(PackageSource::Registry {
                url: if url.is_empty() {
                    "https://registry.tarqeem.dev".to_string()
                } else {
                    url
                },
            })
        } else if source_str.starts_with("git") {
            let content = Self::extract_parenthesized(source_str);
            let parts: Vec<&str> = content.splitn(2, ' ').collect();
            let url = parts.first().unwrap_or(&"").to_string();
            let reference = Self::parse_git_reference(parts.get(1).unwrap_or(&""));
            Ok(PackageSource::Git { url, reference })
        } else if source_str.starts_with("مسار") || source_str.starts_with("path") {
            let path = Self::extract_parenthesized(source_str);
            Ok(PackageSource::Path { path })
        } else {
            // Default to registry
            Ok(PackageSource::Registry {
                url: "https://registry.tarqeem.dev".to_string(),
            })
        }
    }

    fn extract_parenthesized(s: &str) -> String {
        s.find('(')
            .and_then(|start| s.rfind(')').map(|end| s[start + 1..end].to_string()))
            .unwrap_or_default()
    }

    fn parse_git_reference(ref_str: &str) -> GitReference {
        if ref_str.starts_with("فرع=") || ref_str.starts_with("branch=") {
            let branch = ref_str.split('=').nth(1).unwrap_or("main").to_string();
            GitReference::Branch { branch }
        } else if ref_str.starts_with("وسم=") || ref_str.starts_with("tag=") {
            let tag = ref_str.split('=').nth(1).unwrap_or("").to_string();
            GitReference::Tag { tag }
        } else if ref_str.starts_with("مراجعة=") || ref_str.starts_with("rev=") {
            let rev = ref_str.split('=').nth(1).unwrap_or("").to_string();
            GitReference::Rev { rev }
        } else {
            GitReference::Branch {
                branch: "main".to_string(),
            }
        }
    }

    fn parse_package_dependencies(
        pkg_obj: &indexmap::IndexMap<String, format::value::Value>,
    ) -> PackageResult<HashMap<String, String>> {
        let mut deps = HashMap::new();

        if let Some(deps_val) = pkg_obj
            .get("اعتماديات")
            .or_else(|| pkg_obj.get("dependencies"))
        {
            if let Some(deps_obj) = deps_val.as_object() {
                for (name, version_val) in deps_obj {
                    if let Some(version) = version_val.as_str() {
                        deps.insert(name.clone(), version.to_string());
                    }
                }
            }
        }

        Ok(deps)
    }

    pub fn try_parse(path: &Path) -> Option<Self> {
        Self::parse(path).ok()
    }

    pub fn save(&self, path: &Path) -> PackageResult<()> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let content = if ext == "قفل" {
            self.to_arabic_format()
        } else {
            toml::to_string_pretty(self)?
        };

        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn to_arabic_format(&self) -> String {
        let mut output = String::new();

        output.push_str("# ملف قفل حزمة ترقيم\n");
        output.push_str("# لا تعدل هذا الملف يدوياً\n\n");

        output.push_str(&format!("نسخة_الصيغة: {}\n\n", self.version));

        if let Some(ref root) = self.root {
            output.push_str("جذر:\n");
            output.push_str(&format!("    اسم: {}\n", root.name));
            output.push_str(&format!("    نسخة: {}\n", root.version));
            output.push('\n');
        }

        if !self.packages.is_empty() {
            output.push_str("حزم:\n");
            for pkg in &self.packages {
                output.push_str(&format!("    - اسم: {}\n", pkg.name));
                output.push_str(&format!("      نسخة: {}\n", pkg.version));
                output.push_str(&format!("      تحقق: {}\n", pkg.checksum));
                output.push_str(&format!(
                    "      مصدر: {}\n",
                    Self::format_source(&pkg.source)
                ));
                if !pkg.dependencies.is_empty() {
                    output.push_str("      اعتماديات:\n");
                    for (name, ver) in &pkg.dependencies {
                        output.push_str(&format!("          {}: {}\n", name, ver));
                    }
                }
            }
        }

        output
    }

    fn format_source(source: &PackageSource) -> String {
        match source {
            PackageSource::Registry { url } => format!("سجل ({})", url),
            PackageSource::Git { url, reference } => {
                let ref_str = match reference {
                    GitReference::Branch { branch } => format!("فرع={}", branch),
                    GitReference::Tag { tag } => format!("وسم={}", tag),
                    GitReference::Rev { rev } => format!("مراجعة={}", rev),
                };
                format!("git ({} {})", url, ref_str)
            }
            PackageSource::Path { path } => format!("مسار ({})", path),
        }
    }

    pub fn get_package(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }

    pub fn get_package_mut(&mut self, name: &str) -> Option<&mut LockedPackage> {
        self.packages.iter_mut().find(|p| p.name == name)
    }

    pub fn has_package(&self, name: &str) -> bool {
        self.packages.iter().any(|p| p.name == name)
    }

    pub fn upsert_package(&mut self, package: LockedPackage) {
        if let Some(existing) = self.get_package_mut(&package.name) {
            *existing = package;
        } else {
            self.packages.push(package);
        }
    }

    pub fn remove_package(&mut self, name: &str) -> Option<LockedPackage> {
        if let Some(idx) = self.packages.iter().position(|p| p.name == name) {
            Some(self.packages.remove(idx))
        } else {
            None
        }
    }

    pub fn matches(&self, name: &str, version: &str) -> bool {
        self.get_package(name)
            .map(|p| p.version == version)
            .unwrap_or(false)
    }

    pub fn package_names(&self) -> impl Iterator<Item = &str> {
        self.packages.iter().map(|p| p.name.as_str())
    }

    pub fn sort_packages(&mut self) {
        self.packages.sort_by(|a, b| a.name.cmp(&b.name));
    }

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

    pub fn from_path(name: String, version: String, path: String, checksum: String) -> Self {
        Self {
            name,
            version,
            source: PackageSource::Path { path },
            checksum,
            dependencies: HashMap::new(),
        }
    }

    pub fn add_dependency(&mut self, name: String, version: String) {
        self.dependencies.insert(name, version);
    }
}

impl PackageSource {
    pub fn is_registry(&self) -> bool {
        matches!(self, PackageSource::Registry { .. })
    }

    pub fn is_git(&self) -> bool {
        matches!(self, PackageSource::Git { .. })
    }

    pub fn is_path(&self) -> bool {
        matches!(self, PackageSource::Path { .. })
    }

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
}
