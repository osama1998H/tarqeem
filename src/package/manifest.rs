//! Package manifest parsing (ترقيم.حزمة / حزمة.toml / trq.toml)
//!
//! Supports the new Arabic format (.حزمة) and TOML for backward compatibility.
//!
//! # صيغة الحزمة العربية
//!
//! ```text
//! حزمة:
//!     اسم: مكتبتي
//!     نسخة: ١.٠.٠
//!     رخصة: MIT
//!
//! اعتماديات:
//!     json: 2.0.0
//! ```

use super::error::{PackageError, PackageResult};
use super::format::{self, Value as FormatValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    #[serde(rename = "حزمة", alias = "package")]
    pub package: PackageInfo,

    #[serde(rename = "اعتماديات", alias = "dependencies", default)]
    pub dependencies: HashMap<String, DependencySpec>,

    #[serde(
        rename = "اعتماديات_تطوير",
        alias = "اعتماديات-تطوير",
        alias = "dev-dependencies",
        alias = "dev_dependencies",
        default
    )]
    pub dev_dependencies: HashMap<String, DependencySpec>,

    #[serde(rename = "سكربتات", alias = "scripts", default)]
    pub scripts: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageInfo {
    #[serde(rename = "اسم", alias = "name")]
    pub name: String,

    #[serde(rename = "نسخة", alias = "version")]
    pub version: String,

    #[serde(rename = "وصف", alias = "description", default)]
    pub description: Option<String>,

    #[serde(
        rename = "مؤلفون",
        alias = "مؤلف",
        alias = "authors",
        alias = "author",
        default
    )]
    pub authors: Authors,

    #[serde(rename = "رخصة", alias = "license", default)]
    pub license: Option<String>,

    #[serde(rename = "مستودع", alias = "repository", default)]
    pub repository: Option<String>,

    #[serde(rename = "موقع", alias = "homepage", default)]
    pub homepage: Option<String>,

    #[serde(rename = "كلمات", alias = "keywords", default)]
    pub keywords: Vec<String>,

    #[serde(rename = "مدخل", alias = "entry", alias = "main", default)]
    pub entry: Option<String>,

    #[serde(rename = "مكتبة", alias = "lib", default)]
    pub lib: Option<String>,

    #[serde(rename = "ترقيم", alias = "tarqeem", default)]
    pub tarqeem_version: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    Version(String),

    Detailed(DetailedDependency),
}

impl DependencySpec {
    pub fn version_req(&self) -> &str {
        match self {
            DependencySpec::Version(v) => v,
            DependencySpec::Detailed(d) => &d.version,
        }
    }

    pub fn is_path(&self) -> bool {
        matches!(self, DependencySpec::Detailed(d) if d.path.is_some())
    }

    pub fn is_git(&self) -> bool {
        matches!(self, DependencySpec::Detailed(d) if d.git.is_some())
    }

    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            DependencySpec::Detailed(d) => d.path.as_ref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedDependency {
    #[serde(rename = "نسخة", alias = "version")]
    pub version: String,

    #[serde(rename = "مسار", alias = "path", default)]
    pub path: Option<PathBuf>,

    #[serde(default)]
    pub git: Option<String>,

    #[serde(rename = "فرع", alias = "branch", default)]
    pub branch: Option<String>,

    #[serde(rename = "وسم", alias = "tag", default)]
    pub tag: Option<String>,

    #[serde(rename = "مراجعة", alias = "rev", default)]
    pub rev: Option<String>,

    #[serde(rename = "اختياري", alias = "optional", default)]
    pub optional: bool,

    #[serde(rename = "ميزات", alias = "features", default)]
    pub features: Vec<String>,
}

impl Manifest {
    pub const MANIFEST_NAMES: &'static [&'static str] = &[
        "ترقيم.حزمة", // الصيغة العربية الجديدة (أولوية قصوى)
        "حزمة.toml",  // TOML بالعربية (للتوافق)
        "trq.toml",   // TOML بالإنجليزية (للتوافق)
    ];

    pub const DEFAULT_MANIFEST_NAME: &'static str = "ترقيم.حزمة";

    pub fn find_and_parse() -> PackageResult<(Self, PathBuf)> {
        let current = std::env::current_dir()?;
        Self::find_and_parse_from(&current)
    }

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

    pub fn parse(path: &Path) -> PackageResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let manifest = if ext == "حزمة" {
            Self::parse_arabic_format(&content)?
        } else {
            toml::from_str(&content)?
        };

        manifest.validate()?;
        Ok(manifest)
    }

    pub fn parse_arabic_format(content: &str) -> PackageResult<Self> {
        let value =
            format::parse(content).map_err(|e| PackageError::InvalidManifest(format!("{}", e)))?;

        Self::from_format_value(&value)
    }

    fn from_format_value(value: &FormatValue) -> PackageResult<Self> {
        let root = value.as_object().ok_or_else(|| {
            PackageError::InvalidManifest(
                "الملف يجب أن يحتوي على كائن / File must contain an object".to_string(),
            )
        })?;

        let pkg_value = root.get("حزمة").or_else(|| root.get("package"));
        let package = if let Some(pkg) = pkg_value {
            Self::parse_package_info(pkg)?
        } else {
            Self::parse_package_info(value)?
        };

        let dependencies = if let Some(deps) =
            root.get("اعتماديات").or_else(|| root.get("dependencies"))
        {
            Self::parse_dependencies(deps)?
        } else {
            HashMap::new()
        };

        let dev_dependencies = if let Some(deps) = root
            .get("اعتماديات_تطوير")
            .or_else(|| root.get("dev-dependencies"))
        {
            Self::parse_dependencies(deps)?
        } else {
            HashMap::new()
        };

        let scripts = if let Some(s) = root.get("سكربتات").or_else(|| root.get("scripts")) {
            Self::parse_scripts(s)?
        } else {
            HashMap::new()
        };

        Ok(Self {
            package,
            dependencies,
            dev_dependencies,
            scripts,
        })
    }

    fn parse_package_info(value: &FormatValue) -> PackageResult<PackageInfo> {
        let obj = value.as_object().ok_or_else(|| {
            PackageError::InvalidManifest(
                "حزمة يجب أن يكون كائناً / package must be an object".to_string(),
            )
        })?;

        let name = obj
            .get("اسم")
            .or_else(|| obj.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let version = obj
            .get("نسخة")
            .or_else(|| obj.get("version"))
            .map(|v| Self::format_version_string(v))
            .unwrap_or_default();

        let description = obj
            .get("وصف")
            .or_else(|| obj.get("description"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let authors = if let Some(a) = obj.get("مؤلفون").or_else(|| obj.get("authors")) {
            Self::parse_authors(a)
        } else if let Some(a) = obj.get("مؤلف").or_else(|| obj.get("author")) {
            if let Some(s) = a.as_str() {
                Authors::Single(s.to_string())
            } else {
                Authors::None
            }
        } else {
            Authors::None
        };

        let license = obj
            .get("رخصة")
            .or_else(|| obj.get("license"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let repository = obj
            .get("مستودع")
            .or_else(|| obj.get("repository"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let homepage = obj
            .get("موقع")
            .or_else(|| obj.get("homepage"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let keywords = if let Some(kw) = obj.get("كلمات").or_else(|| obj.get("keywords")) {
            if let Some(arr) = kw.as_array() {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        let entry = obj
            .get("مدخل")
            .or_else(|| obj.get("entry"))
            .or_else(|| obj.get("main"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let lib = obj
            .get("مكتبة")
            .or_else(|| obj.get("lib"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tarqeem_version = obj
            .get("ترقيم")
            .or_else(|| obj.get("tarqeem"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(PackageInfo {
            name,
            version,
            description,
            authors,
            license,
            repository,
            homepage,
            keywords,
            entry,
            lib,
            tarqeem_version,
        })
    }

    fn format_version_string(value: &FormatValue) -> String {
        if let Some(s) = value.as_str() {
            s.to_string()
        } else if let Some(n) = value.as_number() {
            if n.fract() == 0.0 {
                format!("{}.0.0", n as i64)
            } else {
                let s = format!("{}", n);
                if s.matches('.').count() == 1 {
                    format!("{}.0", s)
                } else {
                    s
                }
            }
        } else {
            String::new()
        }
    }

    fn parse_authors(value: &FormatValue) -> Authors {
        if let Some(arr) = value.as_array() {
            let authors: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if authors.is_empty() {
                Authors::None
            } else if authors.len() == 1 {
                Authors::Single(authors.into_iter().next().unwrap())
            } else {
                Authors::Multiple(authors)
            }
        } else if let Some(s) = value.as_str() {
            Authors::Single(s.to_string())
        } else {
            Authors::None
        }
    }

    fn parse_dependencies(value: &FormatValue) -> PackageResult<HashMap<String, DependencySpec>> {
        let obj = value.as_object().ok_or_else(|| {
            PackageError::InvalidManifest(
                "اعتماديات يجب أن يكون كائناً / dependencies must be an object".to_string(),
            )
        })?;

        let mut deps = HashMap::new();
        for (name, val) in obj {
            let spec = if let Some(s) = val.as_str() {
                DependencySpec::Version(s.to_string())
            } else if let Some(n) = val.as_number() {
                DependencySpec::Version(Self::format_version_string(val))
            } else if let Some(obj) = val.as_object() {
                let version = obj
                    .get("نسخة")
                    .or_else(|| obj.get("version"))
                    .map(|v| Self::format_version_string(v))
                    .unwrap_or_else(|| "*".to_string());

                let path = obj
                    .get("مسار")
                    .or_else(|| obj.get("path"))
                    .and_then(|v| v.as_str())
                    .map(PathBuf::from);

                let git = obj
                    .get("git")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let branch = obj
                    .get("فرع")
                    .or_else(|| obj.get("branch"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let tag = obj
                    .get("وسم")
                    .or_else(|| obj.get("tag"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let rev = obj
                    .get("مراجعة")
                    .or_else(|| obj.get("rev"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let optional = obj
                    .get("اختياري")
                    .or_else(|| obj.get("optional"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let features = if let Some(f) = obj.get("ميزات").or_else(|| obj.get("features"))
                {
                    if let Some(arr) = f.as_array() {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                };

                DependencySpec::Detailed(DetailedDependency {
                    version,
                    path,
                    git,
                    branch,
                    tag,
                    rev,
                    optional,
                    features,
                })
            } else {
                DependencySpec::Version("*".to_string())
            };

            deps.insert(name.clone(), spec);
        }

        Ok(deps)
    }

    fn parse_scripts(value: &FormatValue) -> PackageResult<HashMap<String, String>> {
        let obj = value.as_object().ok_or_else(|| {
            PackageError::InvalidManifest(
                "سكربتات يجب أن يكون كائناً / scripts must be an object".to_string(),
            )
        })?;

        let mut scripts = HashMap::new();
        for (name, val) in obj {
            if let Some(s) = val.as_str() {
                scripts.insert(name.clone(), s.to_string());
            }
        }

        Ok(scripts)
    }

    pub fn validate(&self) -> PackageResult<()> {
        if self.package.name.is_empty() {
            return Err(PackageError::InvalidManifest(
                "Package name is required / اسم الحزمة مطلوب".to_string(),
            ));
        }

        if self.package.version.is_empty() {
            return Err(PackageError::InvalidManifest(
                "Package version is required / نسخة الحزمة مطلوبة".to_string(),
            ));
        }

        if semver::Version::parse(&self.package.version).is_err() {
            let with_patch = format!("{}.0", self.package.version);
            if semver::Version::parse(&with_patch).is_err() {
                return Err(PackageError::InvalidVersion(self.package.version.clone()));
            }
        }

        Ok(())
    }

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
                entry: Some("مصدر/رئيسي.ترقيم".to_string()),
                lib: None,
                tarqeem_version: None,
            },
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            scripts: HashMap::new(),
        }
    }

    pub fn new_lib(name: &str, version: &str) -> Self {
        let mut manifest = Self::new(name, version);
        manifest.package.entry = None;
        manifest.package.lib = Some("مصدر/مكتبة.ترقيم".to_string());
        manifest
    }

    pub fn save(&self, path: &Path) -> PackageResult<()> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let content = if ext == "حزمة" {
            self.to_arabic_format()
        } else {
            toml::to_string_pretty(self)?
        };

        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn to_arabic_format(&self) -> String {
        let mut output = String::new();

        output.push_str("# ملف تهيئة حزمة ترقيم\n\n");

        output.push_str("حزمة:\n");
        output.push_str(&format!("    اسم: {}\n", self.package.name));
        output.push_str(&format!("    نسخة: {}\n", self.package.version));

        if let Some(ref desc) = self.package.description {
            output.push_str(&format!("    وصف: \"{}\"\n", desc));
        }

        match &self.package.authors {
            Authors::Single(author) => {
                output.push_str(&format!("    مؤلف: {}\n", author));
            }
            Authors::Multiple(authors) if !authors.is_empty() => {
                output.push_str("    مؤلفون:\n");
                for author in authors {
                    output.push_str(&format!("        - {}\n", author));
                }
            }
            _ => {}
        }

        if let Some(ref license) = self.package.license {
            output.push_str(&format!("    رخصة: {}\n", license));
        }

        if let Some(ref repo) = self.package.repository {
            output.push_str(&format!("    مستودع: {}\n", repo));
        }

        if let Some(ref homepage) = self.package.homepage {
            output.push_str(&format!("    موقع: {}\n", homepage));
        }

        if !self.package.keywords.is_empty() {
            output.push_str("    كلمات:\n");
            for keyword in &self.package.keywords {
                output.push_str(&format!("        - {}\n", keyword));
            }
        }

        if let Some(ref entry) = self.package.entry {
            output.push_str(&format!("    مدخل: {}\n", entry));
        }

        if let Some(ref lib) = self.package.lib {
            output.push_str(&format!("    مكتبة: {}\n", lib));
        }

        if let Some(ref tarqeem) = self.package.tarqeem_version {
            output.push_str(&format!("    ترقيم: {}\n", tarqeem));
        }

        if !self.dependencies.is_empty() {
            output.push('\n');
            output.push_str("اعتماديات:\n");
            for (name, spec) in &self.dependencies {
                output.push_str(&Self::format_dependency(name, spec, 4));
            }
        }

        if !self.dev_dependencies.is_empty() {
            output.push('\n');
            output.push_str("اعتماديات_تطوير:\n");
            for (name, spec) in &self.dev_dependencies {
                output.push_str(&Self::format_dependency(name, spec, 4));
            }
        }

        if !self.scripts.is_empty() {
            output.push('\n');
            output.push_str("سكربتات:\n");
            for (name, cmd) in &self.scripts {
                output.push_str(&format!("    {}: \"{}\"\n", name, cmd));
            }
        }

        output
    }

    fn format_dependency(name: &str, spec: &DependencySpec, indent: usize) -> String {
        let spaces = " ".repeat(indent);
        match spec {
            DependencySpec::Version(v) => {
                format!("{}{}: {}\n", spaces, name, v)
            }
            DependencySpec::Detailed(d) => {
                let mut output = format!("{}{}:\n", spaces, name);
                let inner_spaces = " ".repeat(indent + 4);
                output.push_str(&format!("{}نسخة: {}\n", inner_spaces, d.version));

                if let Some(ref path) = d.path {
                    output.push_str(&format!("{}مسار: {}\n", inner_spaces, path.display()));
                }
                if let Some(ref git) = d.git {
                    output.push_str(&format!("{}git: {}\n", inner_spaces, git));
                }
                if let Some(ref branch) = d.branch {
                    output.push_str(&format!("{}فرع: {}\n", inner_spaces, branch));
                }
                if let Some(ref tag) = d.tag {
                    output.push_str(&format!("{}وسم: {}\n", inner_spaces, tag));
                }
                if let Some(ref rev) = d.rev {
                    output.push_str(&format!("{}مراجعة: {}\n", inner_spaces, rev));
                }
                if d.optional {
                    output.push_str(&format!("{}اختياري: نعم\n", inner_spaces));
                }
                if !d.features.is_empty() {
                    output.push_str(&format!("{}ميزات:\n", inner_spaces));
                    for feature in &d.features {
                        output.push_str(&format!("{}    - {}\n", inner_spaces, feature));
                    }
                }

                output
            }
        }
    }

    pub fn project_root(manifest_path: &Path) -> Option<&Path> {
        manifest_path.parent()
    }

    pub fn entry_path(&self, project_root: &Path) -> Option<PathBuf> {
        self.package
            .entry
            .as_ref()
            .or(self.package.lib.as_ref())
            .map(|e| project_root.join(e))
    }

    pub fn is_library(&self) -> bool {
        self.package.lib.is_some() && self.package.entry.is_none()
    }

    pub fn all_dependencies(&self) -> impl Iterator<Item = (&String, &DependencySpec)> {
        self.dependencies.iter().chain(self.dev_dependencies.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arabic_manifest() {
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
        assert_eq!(manifest.package.entry, Some("مصدر/رئيسي.ترقيم".to_string()));
    }

    #[test]
    fn test_new_lib_manifest() {
        let manifest = Manifest::new_lib("test-lib", "0.1.0");
        assert_eq!(manifest.package.name, "test-lib");
        assert!(manifest.package.entry.is_none());
        assert_eq!(manifest.package.lib, Some("مصدر/مكتبة.ترقيم".to_string()));
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


    #[test]
    fn test_parse_arabic_format_simple() {
        let content = r#"
حزمة:
    اسم: مكتبتي
    نسخة: 1.0.0
    رخصة: MIT
"#;
        let manifest = Manifest::parse_arabic_format(content).unwrap();
        assert_eq!(manifest.package.name, "مكتبتي");
        assert_eq!(manifest.package.version, "1.0.0");
        assert_eq!(manifest.package.license, Some("MIT".to_string()));
    }

    #[test]
    fn test_parse_arabic_format_with_dependencies() {
        let content = r#"
حزمة:
    اسم: مشروعي
    نسخة: 0.1.0

اعتماديات:
    json: 2.0.0
    http: 1.0.0
"#;
        let manifest = Manifest::parse_arabic_format(content).unwrap();
        assert_eq!(manifest.package.name, "مشروعي");
        assert!(manifest.dependencies.contains_key("json"));
        assert!(manifest.dependencies.contains_key("http"));
    }

    #[test]
    fn test_parse_arabic_format_with_authors() {
        let content = r#"
حزمة:
    اسم: مكتبتي
    نسخة: 1.0.0
    مؤلفون:
        - أحمد محمد
        - فاطمة علي
"#;
        let manifest = Manifest::parse_arabic_format(content).unwrap();
        assert_eq!(manifest.package.authors.as_vec().len(), 2);
    }

    #[test]
    fn test_parse_arabic_format_arabic_numbers() {
        let content = r#"
حزمة:
    اسم: مكتبتي
    نسخة: ١.٠.٠
"#;
        let manifest = Manifest::parse_arabic_format(content).unwrap();
        assert_eq!(manifest.package.name, "مكتبتي");
        assert!(
            manifest.package.version.contains("1.0.0") || manifest.package.version.contains("1")
        );
    }

    #[test]
    fn test_parse_arabic_format_detailed_dependency() {
        let content = r#"
حزمة:
    اسم: مشروعي
    نسخة: 0.1.0

اعتماديات:
    أدوات:
        نسخة: 1.0.0
        اختياري: نعم
"#;
        let manifest = Manifest::parse_arabic_format(content).unwrap();
        let dep = manifest.dependencies.get("أدوات").unwrap();
        assert_eq!(dep.version_req(), "1.0.0");
        if let DependencySpec::Detailed(d) = dep {
            assert!(d.optional);
        } else {
            panic!("Expected detailed dependency");
        }
    }

    #[test]
    fn test_to_arabic_format() {
        let manifest = Manifest::new("مشروعي", "0.1.0");
        let output = manifest.to_arabic_format();

        assert!(output.contains("حزمة:"));
        assert!(output.contains("اسم: مشروعي"));
        assert!(output.contains("نسخة: 0.1.0"));
    }

    #[test]
    fn test_arabic_format_roundtrip() {
        let original = Manifest::new("مكتبتي", "1.0.0");
        let arabic_content = original.to_arabic_format();
        let parsed = Manifest::parse_arabic_format(&arabic_content).unwrap();

        assert_eq!(parsed.package.name, original.package.name);
        assert_eq!(parsed.package.version, original.package.version);
    }

    #[test]
    fn test_manifest_names_priority() {
        assert_eq!(Manifest::MANIFEST_NAMES[0], "ترقيم.حزمة");
        assert_eq!(Manifest::MANIFEST_NAMES[1], "حزمة.toml");
        assert_eq!(Manifest::MANIFEST_NAMES[2], "trq.toml");
    }

    #[test]
    fn test_default_manifest_name() {
        assert_eq!(Manifest::DEFAULT_MANIFEST_NAME, "ترقيم.حزمة");
    }
}
