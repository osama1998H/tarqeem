//! Package management for Tarqeem
//!
//! This module provides package management functionality including:
//! - Manifest parsing (حزمة.toml / trq.toml)
//! - Lock file management (.trqlock)
//! - Dependency resolution
//! - Package caching
//!
//! # Example Manifest
//!
//! ```toml
//! [حزمة]
//! اسم = "مكتبتي"
//! نسخة = "0.1.0"
//! وصف = "مكتبة رائعة"
//!
//! [اعتماديات]
//! json = "1.0"
//! ```

pub mod cache;
pub mod error;
pub mod lockfile;
pub mod manifest;
pub mod resolver;

// Re-export commonly used types
pub use cache::Cache;
pub use error::{PackageError, PackageResult};
pub use lockfile::{GitReference, LockFile, LockedPackage, PackageSource};
pub use manifest::{Authors, DependencySpec, DetailedDependency, Manifest, PackageInfo};
pub use resolver::{ResolvedPackage, Resolver};
