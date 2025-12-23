//! File extension utilities for Tarqeem
//!
//! Provides constants and utilities for handling Tarqeem file extensions,
//! supporting both ASCII (.trq) and Arabic (.ترقيم) extensions.
//!
//! ## Supported Extensions
//!
//! | Type | Extensions |
//! |------|------------|
//! | Source | `.trq`, `.ترقيم` |
//! | Header | `.trqh`, `.ترقيم-ر` |
//! | Package | `.حزمة` |
//! | Lock | `.قفل` |

use std::ffi::OsStr;
use std::path::Path;

/// Valid source file extensions for Tarqeem
/// Supports both ASCII (.trq) and Arabic (.ترقيم) extensions
pub const SOURCE_EXTENSIONS: &[&str] = &["trq", "ترقيم"];

/// Valid header/interface file extensions
/// Supports both ASCII (.trqh) and Arabic (.ترقيم-ر) extensions
pub const HEADER_EXTENSIONS: &[&str] = &["trqh", "ترقيم-ر"];

/// Valid package manifest file extensions
/// Arabic-only extension for package configuration files
pub const PACKAGE_EXTENSIONS: &[&str] = &["حزمة"];

/// Valid lock file extensions
/// Arabic-only extension for dependency lock files
pub const LOCK_EXTENSIONS: &[&str] = &["قفل"];

/// Represents a Tarqeem file extension type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileExtension {
    /// Source file (.trq or .ترقيم)
    Source,
    /// Header/interface file (.trqh or .ترقيم-ر)
    Header,
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

        if SOURCE_EXTENSIONS.contains(&ext) {
            FileExtension::Source
        } else if HEADER_EXTENSIONS.contains(&ext) {
            FileExtension::Header
        } else if PACKAGE_EXTENSIONS.contains(&ext) {
            FileExtension::Package
        } else if LOCK_EXTENSIONS.contains(&ext) {
            FileExtension::Lock
        } else {
            FileExtension::Unknown
        }
    }

    /// Check if this is a valid Tarqeem extension (source or header)
    pub fn is_valid(&self) -> bool {
        matches!(self, FileExtension::Source | FileExtension::Header)
    }

    /// Check if this is a valid Tarqeem code extension (source or header)
    pub fn is_code(&self) -> bool {
        matches!(self, FileExtension::Source | FileExtension::Header)
    }

    /// Check if this is a valid package-related extension
    pub fn is_package_related(&self) -> bool {
        matches!(self, FileExtension::Package | FileExtension::Lock)
    }

    /// Get the primary extension for this type
    /// For Package and Lock, returns Arabic since they are Arabic-only
    pub fn primary_extension(&self) -> &'static str {
        match self {
            FileExtension::Source => "trq",
            FileExtension::Header => "trqh",
            FileExtension::Package => "حزمة",
            FileExtension::Lock => "قفل",
            FileExtension::Unknown => "",
        }
    }

    /// Get the Arabic extension for this type
    pub fn arabic_extension(&self) -> &'static str {
        match self {
            FileExtension::Source => "ترقيم",
            FileExtension::Header => "ترقيم-ر",
            FileExtension::Package => "حزمة",
            FileExtension::Lock => "قفل",
            FileExtension::Unknown => "",
        }
    }
}

/// Check if a path has a valid Tarqeem source extension (.trq or .ترقيم)
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use tarqeem::utils::is_valid_source_extension;
///
/// assert!(is_valid_source_extension(Path::new("program.trq")));
/// assert!(is_valid_source_extension(Path::new("برنامج.ترقيم")));
/// assert!(!is_valid_source_extension(Path::new("file.txt")));
/// ```
pub fn is_valid_source_extension(path: &Path) -> bool {
    matches!(FileExtension::from_path(path), FileExtension::Source)
}

/// Check if a path has a valid Tarqeem header extension (.trqh or .ترقيم-ر)
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use tarqeem::utils::is_valid_header_extension;
///
/// assert!(is_valid_header_extension(Path::new("types.trqh")));
/// assert!(is_valid_header_extension(Path::new("أنماط.ترقيم-ر")));
/// assert!(!is_valid_header_extension(Path::new("file.h")));
/// ```
pub fn is_valid_header_extension(path: &Path) -> bool {
    matches!(FileExtension::from_path(path), FileExtension::Header)
}

/// Check if a path has any valid Tarqeem extension (source or header)
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use tarqeem::utils::has_tarqeem_extension;
///
/// assert!(has_tarqeem_extension(Path::new("program.trq")));
/// assert!(has_tarqeem_extension(Path::new("برنامج.ترقيم")));
/// assert!(has_tarqeem_extension(Path::new("types.trqh")));
/// assert!(!has_tarqeem_extension(Path::new("file.rs")));
/// ```
pub fn has_tarqeem_extension(path: &Path) -> bool {
    FileExtension::from_path(path).is_valid()
}

/// Get a formatted string of valid source extensions for error messages
pub fn valid_source_extensions_display() -> String {
    SOURCE_EXTENSIONS
        .iter()
        .map(|e| format!(".{}", e))
        .collect::<Vec<_>>()
        .join(" أو / or ")
}

/// Get a formatted string of valid header extensions for error messages
pub fn valid_header_extensions_display() -> String {
    HEADER_EXTENSIONS
        .iter()
        .map(|e| format!(".{}", e))
        .collect::<Vec<_>>()
        .join(" أو / or ")
}

/// Check if a path has a valid package manifest extension (.حزمة)
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use tarqeem::utils::is_valid_package_extension;
///
/// assert!(is_valid_package_extension(Path::new("حزمة.حزمة")));
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
/// assert!(!is_valid_lock_extension(Path::new(".trqlock")));
/// ```
pub fn is_valid_lock_extension(path: &Path) -> bool {
    matches!(FileExtension::from_path(path), FileExtension::Lock)
}

/// Get a formatted string of valid package extensions for error messages
pub fn valid_package_extensions_display() -> String {
    PACKAGE_EXTENSIONS
        .iter()
        .map(|e| format!(".{}", e))
        .collect::<Vec<_>>()
        .join(" أو ")
}

/// Get a formatted string of valid lock extensions for error messages
pub fn valid_lock_extensions_display() -> String {
    LOCK_EXTENSIONS
        .iter()
        .map(|e| format!(".{}", e))
        .collect::<Vec<_>>()
        .join(" أو ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_extension_trq() {
        assert!(is_valid_source_extension(Path::new("program.trq")));
        assert!(is_valid_source_extension(Path::new("مرحبا.trq")));
        assert!(is_valid_source_extension(Path::new("/path/to/file.trq")));
    }

    #[test]
    fn test_source_extension_arabic() {
        assert!(is_valid_source_extension(Path::new("برنامج.ترقيم")));
        assert!(is_valid_source_extension(Path::new("program.ترقيم")));
        assert!(is_valid_source_extension(Path::new("/مسار/ملف.ترقيم")));
    }

    #[test]
    fn test_header_extensions() {
        assert!(is_valid_header_extension(Path::new("types.trqh")));
        assert!(is_valid_header_extension(Path::new("أنماط.ترقيم-ر")));
    }

    #[test]
    fn test_invalid_extensions() {
        assert!(!is_valid_source_extension(Path::new("file.txt")));
        assert!(!is_valid_source_extension(Path::new("file.rs")));
        assert!(!is_valid_source_extension(Path::new("file.py")));
        assert!(!is_valid_source_extension(Path::new("noextension")));
    }

    #[test]
    fn test_has_tarqeem_extension() {
        assert!(has_tarqeem_extension(Path::new("file.trq")));
        assert!(has_tarqeem_extension(Path::new("file.ترقيم")));
        assert!(has_tarqeem_extension(Path::new("file.trqh")));
        assert!(has_tarqeem_extension(Path::new("file.ترقيم-ر")));
        assert!(!has_tarqeem_extension(Path::new("file.txt")));
    }

    #[test]
    fn test_file_extension_enum() {
        assert_eq!(
            FileExtension::from_path(Path::new("file.trq")),
            FileExtension::Source
        );
        assert_eq!(
            FileExtension::from_path(Path::new("file.ترقيم")),
            FileExtension::Source
        );
        assert_eq!(
            FileExtension::from_path(Path::new("file.trqh")),
            FileExtension::Header
        );
        assert_eq!(
            FileExtension::from_path(Path::new("file.txt")),
            FileExtension::Unknown
        );
    }

    #[test]
    fn test_extension_display() {
        let display = valid_source_extensions_display();
        assert!(display.contains(".trq"));
        assert!(display.contains(".ترقيم"));
    }

    #[test]
    fn test_package_extension() {
        assert!(is_valid_package_extension(Path::new("حزمة.حزمة")));
        assert!(is_valid_package_extension(Path::new("مكتبتي.حزمة")));
        assert!(is_valid_package_extension(Path::new("/مسار/مشروع.حزمة")));
        assert!(!is_valid_package_extension(Path::new("package.toml")));
        assert!(!is_valid_package_extension(Path::new("حزمة.toml")));
    }

    #[test]
    fn test_lock_extension() {
        assert!(is_valid_lock_extension(Path::new("حزمة.قفل")));
        assert!(is_valid_lock_extension(Path::new("مشروع.قفل")));
        assert!(!is_valid_lock_extension(Path::new(".trqlock")));
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
        assert!(!FileExtension::Header.is_package_related());
        assert!(!FileExtension::Unknown.is_package_related());
    }

    #[test]
    fn test_is_code() {
        assert!(FileExtension::Source.is_code());
        assert!(FileExtension::Header.is_code());
        assert!(!FileExtension::Package.is_code());
        assert!(!FileExtension::Lock.is_code());
        assert!(!FileExtension::Unknown.is_code());
    }

    #[test]
    fn test_package_extension_display() {
        let display = valid_package_extensions_display();
        assert!(display.contains(".حزمة"));
    }

    #[test]
    fn test_lock_extension_display() {
        let display = valid_lock_extensions_display();
        assert!(display.contains(".قفل"));
    }
}
