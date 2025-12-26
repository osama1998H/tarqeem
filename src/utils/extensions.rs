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

pub const SOURCE_EXTENSION: &str = "ترقيم";

pub const PACKAGE_EXTENSION: &str = "حزمة";

pub const LOCK_EXTENSION: &str = "قفل";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileExtension {
    Source,
    Package,
    Lock,
    Unknown,
}

impl FileExtension {
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

    pub fn is_valid(&self) -> bool {
        matches!(self, FileExtension::Source)
    }

    pub fn is_code(&self) -> bool {
        matches!(self, FileExtension::Source)
    }

    pub fn is_package_related(&self) -> bool {
        matches!(self, FileExtension::Package | FileExtension::Lock)
    }

    pub fn extension(&self) -> &'static str {
        match self {
            FileExtension::Source => SOURCE_EXTENSION,
            FileExtension::Package => PACKAGE_EXTENSION,
            FileExtension::Lock => LOCK_EXTENSION,
            FileExtension::Unknown => "",
        }
    }
}

pub fn is_valid_source_extension(path: &Path) -> bool {
    matches!(FileExtension::from_path(path), FileExtension::Source)
}

pub fn has_tarqeem_extension(path: &Path) -> bool {
    FileExtension::from_path(path).is_valid()
}

pub fn valid_source_extension_display() -> String {
    format!(".{}", SOURCE_EXTENSION)
}

pub fn is_valid_package_extension(path: &Path) -> bool {
    matches!(FileExtension::from_path(path), FileExtension::Package)
}

pub fn is_valid_lock_extension(path: &Path) -> bool {
    matches!(FileExtension::from_path(path), FileExtension::Lock)
}

pub fn valid_package_extension_display() -> String {
    format!(".{}", PACKAGE_EXTENSION)
}

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
        assert!(!is_valid_source_extension(Path::new("file.ترقيم")));
        assert!(!is_valid_source_extension(Path::new("noextension")));
    }

    #[test]
    fn test_has_tarqeem_extension() {
        assert!(has_tarqeem_extension(Path::new("file.ترقيم")));
        assert!(!has_tarqeem_extension(Path::new("file.txt")));
    }

    #[test]
    fn test_file_extension_enum() {
        assert_eq!(
            FileExtension::from_path(Path::new("file.ترقيم")),
            FileExtension::Source
        );
        assert_eq!(
            FileExtension::from_path(Path::new("file.ترقيم")),
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
        assert!(is_valid_package_extension(Path::new("ترقيم.حزمة")));
        assert!(is_valid_package_extension(Path::new("مكتبتي.حزمة")));
        assert!(is_valid_package_extension(Path::new("/مسار/مشروع.حزمة")));
    }

    #[test]
    fn test_lock_extension() {
        assert!(is_valid_lock_extension(Path::new("حزمة.قفل")));
        assert!(is_valid_lock_extension(Path::new("مشروع.قفل")));
    }

    #[test]
    fn test_file_extension_enum_package_lock() {
        assert_eq!(
            FileExtension::from_path(Path::new("ترقيم.حزمة")),
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
