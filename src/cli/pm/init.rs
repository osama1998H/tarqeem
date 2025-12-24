//! Package initialization command
//!
//! Creates a new Tarqeem package with proper directory structure.
//! Uses the new Arabic format (ترقيم.حزمة) by default.

use crate::package::{Manifest, PackageError, PackageResult};
use colored::*;
use std::fs;
use std::path::Path;

pub fn run(name: Option<String>, lib: bool) -> PackageResult<()> {
    // Check if already initialized - check all manifest names
    for manifest_name in Manifest::MANIFEST_NAMES {
        if Path::new(manifest_name).exists() {
            return Err(PackageError::AlreadyInitialized);
        }
    }

    // Get package name from argument or directory
    let pkg_name = name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "my-package".to_string())
    });

    // Validate package name
    if pkg_name.is_empty() {
        return Err(PackageError::InvalidPackageName(
            "Package name cannot be empty".to_string(),
        ));
    }

    // Create manifest
    let manifest = if lib {
        Manifest::new_lib(&pkg_name, "0.1.0")
    } else {
        Manifest::new(&pkg_name, "0.1.0")
    };

    // Create directory structure
    let dirs = if lib {
        vec!["مصدر", "اختبارات", "أمثلة", "توثيق"]
    } else {
        vec!["مصدر", "اختبارات"]
    };

    for dir in &dirs {
        fs::create_dir_all(dir)?;
        println!("  {} {}/", "→".cyan(), dir);
    }

    // Create entry file - use Arabic extension by default
    let (entry_path, entry_content) = if lib {
        (
            "مصدر/مكتبة.ترقيم",
            r#"# مكتبة ترقيم

# دالة مرحبا - ترجع رسالة ترحيب
صدّر دالة مرحبا() -> نص {
    أرجع "مرحباً من المكتبة!"
}

# دالة رئيسية للمكتبة (للاختبار)
صدّر دالة اختبار() {
    اطبع(مرحبا())
}
"#,
        )
    } else {
        (
            "مصدر/رئيسي.ترقيم",
            r#"# برنامج ترقيم رئيسي

# الدالة الرئيسية - نقطة بداية البرنامج
دالة رئيسي() {
    اطبع("مرحباً بالعالم!")
}
"#,
        )
    };

    if !Path::new(entry_path).exists() {
        fs::write(entry_path, entry_content)?;
        println!("  {} {}", "→".cyan(), entry_path);
    }

    // Create test file - use Arabic extension
    let test_path = "اختبارات/اختبار.ترقيم";
    let test_content = if lib {
        r#"# اختبارات المكتبة

استورد { مرحبا } من "../مصدر/مكتبة"

# اختبار دالة مرحبا
دالة اختبار_مرحبا() {
    متغير نتيجة = مرحبا()
    تأكيد(نتيجة == "مرحباً من المكتبة!")
}
"#
    } else {
        r#"# اختبارات البرنامج

# اختبار بسيط
دالة اختبار_أساسي() {
    تأكيد(١ + ١ == ٢)
}
"#
    };

    if !Path::new(test_path).exists() {
        fs::write(test_path, test_content)?;
        println!("  {} {}", "→".cyan(), test_path);
    }

    // Save manifest in Arabic format
    manifest.save(Path::new("ترقيم.حزمة"))?;
    println!("  {} ترقيم.حزمة", "→".cyan());

    // Create .gitignore
    let gitignore_content = r#"# مخلفات البناء / Tarqeem build artifacts
/بناء/
/build/
/target/

# ملف القفل (اختياري: البعض يفضل تضمينه)
# Lock file (optional: some prefer to commit this)
# ترقيم.قفل
# .trqlock

# مجلد الحزم
# Packages directory
/packages/

# ملفات المحررات
# IDE and editor files
.vscode/
.idea/
*.swp
*.swo
*~

# ملفات النظام
# OS files
.DS_Store
Thumbs.db
"#;

    if !Path::new(".gitignore").exists() {
        fs::write(".gitignore", gitignore_content)?;
        println!("  {} .gitignore", "→".cyan());
    }

    // Create README.md
    let readme_content = format!(
        r#"# {}

{} package created with Tarqeem.

## البناء والتشغيل / Build and Run

```bash
# تثبيت الاعتماديات / Install dependencies
tarqeem pkg install

# بناء الحزمة / Build package
tarqeem pkg build

# تشغيل البرنامج / Run program
tarqeem pkg run
```

## الاختبارات / Tests

```bash
tarqeem pkg test
```

## الرخصة / License

MIT
"#,
        pkg_name,
        if lib { "Library" } else { "Application" }
    );

    if !Path::new("README.md").exists() {
        fs::write("README.md", readme_content)?;
        println!("  {} README.md", "→".cyan());
    }

    println!();
    println!(
        "{}",
        format!(
            "✓ Created package '{}' / تم إنشاء الحزمة '{}'",
            pkg_name, pkg_name
        )
        .green()
    );
    println!();
    println!("Next steps / الخطوات التالية:");
    println!("  {} tarqeem pkg build  # بناء الحزمة", "→".cyan());
    println!("  {} tarqeem pkg run    # تشغيل البرنامج", "→".cyan());

    Ok(())
}
