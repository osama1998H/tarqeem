//! Package manager errors
//!
//! Provides bilingual error messages for package management operations.

use std::path::PathBuf;
use thiserror::Error;

/// Package manager errors with bilingual messages
#[derive(Debug, Error)]
pub enum PackageError {
    #[error("Manifest not found (حزمة.toml or trq.toml) / لم يتم العثور على ملف الحزمة")]
    ManifestNotFound,

    #[error("Invalid manifest: {0} / ملف حزمة غير صالح: {0}")]
    InvalidManifest(String),

    #[error("Package not found: {0} / الحزمة غير موجودة: {0}")]
    PackageNotFound(String),

    #[error("Version not found: {name}@{version} / النسخة غير موجودة: {name}@{version}")]
    VersionNotFound { name: String, version: String },

    #[error("Dependency conflict: {0} / تعارض في الاعتماديات: {0}")]
    DependencyConflict(String),

    #[error("Circular dependency detected: {0} / اعتمادية دائرية: {0}")]
    CircularDependency(String),

    #[error("Network error: {0} / خطأ في الشبكة: {0}")]
    NetworkError(String),

    #[error("IO error: {0} / خطأ في الإدخال/الإخراج: {0}")]
    IoError(#[from] std::io::Error),

    #[error("TOML parse error: {0} / خطأ في تحليل TOML: {0}")]
    TomlParseError(#[from] toml::de::Error),

    #[error("TOML serialize error: {0} / خطأ في تسلسل TOML: {0}")]
    TomlSerializeError(#[from] toml::ser::Error),

    #[error("Invalid version: {0} / نسخة غير صالحة: {0}")]
    InvalidVersion(String),

    #[error("Checksum mismatch for {0} / عدم تطابق في التحقق للحزمة {0}")]
    ChecksumMismatch(String),

    #[error("Already initialized / تمت التهيئة مسبقاً")]
    AlreadyInitialized,

    #[error("Not in a package directory / ليس مجلد حزمة")]
    NotInPackage,

    #[error("Entry point not found: {0} / نقطة الدخول غير موجودة: {0}")]
    EntryPointNotFound(PathBuf),

    #[error("Build failed: {0} / فشل البناء: {0}")]
    BuildFailed(String),

    #[error("Package already exists: {0} / الحزمة موجودة مسبقاً: {0}")]
    PackageAlreadyExists(String),

    #[error("Invalid package name: {0} / اسم حزمة غير صالح: {0}")]
    InvalidPackageName(String),

    #[error("Cache error: {0} / خطأ في التخزين المؤقت: {0}")]
    CacheError(String),
}

/// Result type for package operations
pub type PackageResult<T> = Result<T, PackageError>;
