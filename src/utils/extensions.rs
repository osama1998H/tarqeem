//! File extension utilities for Tarqeem
//!
//! Provides constants and utilities for handling Tarqeem file extensions.
//! Tarqeem uses Arabic-only extensions to maintain language consistency.
//!
//! ## Supported Extensions
//!
//! | Type | Extension |
//! |------|-----------|
//! | Source | `.ترقيم` |
//! | Package | `.حزمة` |
//! | Lock | `.قفل` |

use std::ffi::OsStr;
use std::path::Path;

/// Valid source file extension for Tarqeem (Arabic only)
pub const SOURCE_EXTENSION: &str = "ترقيم";

/// Valid package manifest file extension (Arabic only)
pub const PACKAGE_EXTENSION: &str = "حزمة";

/// Valid lock file extension (Arabic only)
pub const LOCK_EXTENSION: &str = "قفل";

/// Represents a Tarqeem file extension type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileExtension {
    /// Source file (.ترقيم)
    Source,
    /// Package manifest file (.حزمة)
    Package,
    /// Lock file (.قفل)
    Lock,
    /// Unknown/unsupported extension
    Unknown,
}

impl FileExtension {
    /// Determine the file extension type from a path
    pub fn from_path(path: &Path) -> Self {
        let ext = path.extension().and_then(OsStr::to_str).unwrap_or("");

        if ext == SOURCE_EXTENSION {
            FileExtension::Source
        } else if ext == PACKAGE_EXTENSION {
            FileExtension::Package
        } else if ext == LOCK_EXTENSION {
            FileExtension::Lock
        } else {
            FileExtension::Unknown
        }
    }

    /// Check if this is a valid Tarqeem source extension
    pub fn is_valid(&self) -> bool {
        matches!(self, FileExtension::Source)
    }

    /// Check if this is a valid Tarqeem code extension (source files only)
    pub fn is_code(&self) -> bool {
        matches!(self, FileExtension::Source)
    }

    /// Check if this is a valid package-related extension
    pub fn is_package_related(&self) -> bool {
        matches!(self, FileExtension::Package | FileExtension::Lock)
    }

    /// Get the extension for this type
    pub fn extension(&self) -> &'static str {
        match self {
            FileExtension::Source => SOURCE_EXTENSION,
            FileExtension::Package => PACKAGE_EXTENSION,
            FileExtension::Lock => LOCK_EXTENSION,
            FileExtension::Unknown => "",
        }
    }
}

/// Check if a path has a valid Tarqeem source extension (.ترقيم)
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use tarqeem::utils::is_valid_source_extension;
///
/// assert!(is_valid_source_extension(Path::new("برنامج.ترقيم")));
/// assert!(!is_valid_source_extension(Path::new("file.txt")));
/// ```
pub fn is_valid_source_extension(path: &Path) -> bool {
    matches!(FileExtension::from_path(path), FileExtension::Source)
}

/// Check if a path has any valid Tarqeem source extension
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use tarqeem::utils::has_tarqeem_extension;
///
/// assert!(has_tarqeem_extension(Path::new("برنامج.ترقيم")));
/// assert!(!has_tarqeem_extension(Path::new("file.rs")));
/// ```
pub fn has_tarqeem_extension(path: &Path) -> bool {
    FileExtension::from_path(path).is_valid()
}

/// Get the valid source extension for error messages
pub fn valid_source_extension_display() -> String {
    format!(".{}", SOURCE_EXTENSION)
}

/// Check if a path has a valid package manifest extension (.حزمة)
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use tarqeem::utils::is_valid_package_extension;
///
/// assert!(is_valid_package_extension(Path::new("مكتبتي.حزمة")));
/// assert!(!is_valid_package_extension(Path::new("package.toml")));
/// ```
pub fn is_valid_package_extension(path: &Path) -> bool {
    matches!(FileExtension::from_path(path), FileExtension::Package)
}

/// Check if a path has a valid lock file extension (.قفل)
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use tarqeem::utils::is_valid_lock_extension;
///
/// assert!(is_valid_lock_extension(Path::new("حزمة.قفل")));
/// assert!(!is_valid_lock_extension(Path::new("package.lock")));
/// ```
pub fn is_valid_lock_extension(path: &Path) -> bool {
    matches!(FileExtension::from_path(path), FileExtension::Lock)
}

/// Get the valid package extension for error messages
pub fn valid_package_extension_display() -> String {
    format!(".{}", PACKAGE_EXTENSION)
}

/// Get the valid lock extension for error messages
pub fn valid_lock_extension_display() -> String {
    format!(".{}", LOCK_EXTENSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_extension_arabic() {
        assert!(is_valid_source_extension(Path::new("برنامج.ترقيم")));
        assert!(is_valid_source_extension(Path::new("مرحبا.ترقيم")));
        assert!(is_valid_source_extension(Path::new("/مسار/ملف.ترقيم")));
    }

    #[test]
    fn test_invalid_extensions() {
        assert!(!is_valid_source_extension(Path::new("file.txt")));
        assert!(!is_valid_source_extension(Path::new("file.rs")));
        assert!(!is_valid_source_extension(Path::new("file.py")));
        assert!(!is_valid_source_extension(Path::new("file.trq")));
        assert!(!is_valid_source_extension(Path::new("noextension")));
    }

    #[test]
    fn test_has_tarqeem_extension() {
        assert!(has_tarqeem_extension(Path::new("file.ترقيم")));
        assert!(!has_tarqeem_extension(Path::new("file.trq")));
        assert!(!has_tarqeem_extension(Path::new("file.txt")));
    }

    #[test]
    fn test_file_extension_enum() {
        assert_eq!(
            FileExtension::from_path(Path::new("file.ترقيم")),
            FileExtension::Source
        );
        assert_eq!(
            FileExtension::from_path(Path::new("file.trq")),
            FileExtension::Unknown
        );
        assert_eq!(
            FileExtension::from_path(Path::new("file.txt")),
            FileExtension::Unknown
        );
    }

    #[test]
    fn test_extension_display() {
        let display = valid_source_extension_display();
        assert_eq!(display, ".ترقيم");
    }

    #[test]
    fn test_package_extension() {
        assert!(is_valid_package_extension(Path::new("حزمة.حزمة")));
        assert!(is_valid_package_extension(Path::new("مكتبتي.حزمة")));
        assert!(is_valid_package_extension(Path::new("/مسار/مشروع.حزمة")));
        assert!(!is_valid_package_extension(Path::new("package.toml")));
    }

    #[test]
    fn test_lock_extension() {
        assert!(is_valid_lock_extension(Path::new("حزمة.قفل")));
        assert!(is_valid_lock_extension(Path::new("مشروع.قفل")));
        assert!(!is_valid_lock_extension(Path::new("package.lock")));
    }

    #[test]
    fn test_file_extension_enum_package_lock() {
        assert_eq!(
            FileExtension::from_path(Path::new("حزمة.حزمة")),
            FileExtension::Package
        );
        assert_eq!(
            FileExtension::from_path(Path::new("مشروع.قفل")),
            FileExtension::Lock
        );
    }

    #[test]
    fn test_is_package_related() {
        assert!(FileExtension::Package.is_package_related());
        assert!(FileExtension::Lock.is_package_related());
        assert!(!FileExtension::Source.is_package_related());
        assert!(!FileExtension::Unknown.is_package_related());
    }

    #[test]
    fn test_is_code() {
        assert!(FileExtension::Source.is_code());
        assert!(!FileExtension::Package.is_code());
        assert!(!FileExtension::Lock.is_code());
        assert!(!FileExtension::Unknown.is_code());
    }

    #[test]
    fn test_package_extension_display() {
        let display = valid_package_extension_display();
        assert_eq!(display, ".حزمة");
    }

    #[test]
    fn test_lock_extension_display() {
        let display = valid_lock_extension_display();
        assert_eq!(display, ".قفل");
    }
}
