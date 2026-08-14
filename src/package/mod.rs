//! Package management for Tarqeem
//!
//! This module provides package management functionality including:
//! - Manifest parsing (ترقيم.حزمة)
//! - Lock file management (ترقيم.قفل)
//! - Dependency resolution
//! - Package caching
//!
//! # صيغة الحزمة العربية
//!
//! ```text
//! # ملف تهيئة حزمة ترقيم
//!
//! حزمة:
//!     اسم: مكتبتي
//!     نسخة: ٠.١.٠
//!     وصف: "مكتبة رائعة"
//!     رخصة: MIT
//!
//! اعتماديات:
//!     json: 1.0
//! ```

pub mod cache;
pub mod error;
pub mod format;
pub mod lockfile;
pub mod manifest;
pub mod resolver;

pub use cache::Cache;
pub use error::{PackageError, PackageResult};
pub use format::{parse as parse_format, FormatError, Value as FormatValue};
pub use lockfile::{GitReference, LockFile, LockedPackage, PackageSource};
pub use manifest::{Authors, DependencySpec, DetailedDependency, Manifest, PackageInfo};
pub use resolver::{ResolvedPackage, Resolver};
