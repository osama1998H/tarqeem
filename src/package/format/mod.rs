//! صيغة الحزمة العربية
//! Arabic package format
//!
//! صيغة تهيئة عربية بالكامل لحزم ترقيم، بديلة عن TOML.
//! A fully Arabic configuration format for Tarqeem packages, replacing TOML.
//!
//! # بنية الصيغة / Format Structure
//!
//! ```text
//! # تعليق
//!
//! حزمة:
//!     اسم: مكتبتي
//!     نسخة: ١.٠.٠
//!     وصف: "مكتبة رائعة"
//!     مؤلفون:
//!         - أحمد
//!         - محمد
//!     رخصة: MIT
//!
//! اعتماديات:
//!     json: 2.0.0
//! ```
//!
//! # المميزات / Features
//!
//! - مسافة بادئة بدلاً من الأقواس
//! - دعم الأرقام العربية (٠-٩)
//! - دعم المنطقيات العربية (نعم/لا)
//! - تعليقات بـ #
//!
//! # أمثلة / Examples
//!
//! ```
//! use tarqeem::package::format::parse;
//!
//! let source = r#"
//! اسم: مكتبتي
//! نسخة: 1.0.0
//! "#;
//!
//! let value = parse(source).unwrap();
//! let name = value.get("اسم").unwrap().as_str().unwrap();
//! assert_eq!(name, "مكتبتي");
//! ```

pub mod error;
pub mod lexer;
pub mod parser;
pub mod value;

pub use error::{FormatError, FormatErrorKind, FormatResult, Location};
pub use lexer::{Lexer, Token};
pub use parser::{parse, Parser};
pub use value::Value;

pub const DEFAULT_MANIFEST_NAME: &str = "ترقيم.حزمة";

pub const DEFAULT_LOCK_NAME: &str = "ترقيم.قفل";

pub const MANIFEST_NAMES: &[&str] = &[
    "ترقيم.حزمة", // الصيغة العربية الجديدة (أولوية قصوى)
];

pub const LOCK_NAMES: &[&str] = &[
    "ترقيم.قفل", // الصيغة العربية الجديدة (أولوية قصوى)
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let source = "اسم: مكتبتي\nنسخة: 1.0.0";
        let value = parse(source).unwrap();
        assert!(value.is_object());
    }

    #[test]
    fn test_default_names() {
        assert_eq!(DEFAULT_MANIFEST_NAME, "ترقيم.حزمة");
        assert_eq!(DEFAULT_LOCK_NAME, "ترقيم.قفل");
    }

    #[test]
    fn test_manifest_names_priority() {
        assert_eq!(MANIFEST_NAMES[0], "ترقيم.حزمة");
    }
}
