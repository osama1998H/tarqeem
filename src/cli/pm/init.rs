//! Package initialization command
//!
//! Creates a new Tarqeem package with proper directory structure.

use crate::package::{Manifest, PackageError, PackageResult};
use colored::*;
use std::fs;
use std::path::Path;

/// Run the init command
pub fn run(name: Option<String>, lib: bool) -> PackageResult<()> {
    // Check if already initialized
    if Path::new("حزمة.toml").exists() || Path::new("trq.toml").exists() {
        return Err(PackageError::AlreadyInitialized);
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

    // Create entry file
    let (entry_path, entry_content) = if lib {
        (
            "مصدر/lib.trq",
            r#"/// مكتبة ترقيم
/// Tarqeem library

/// دالة مرحبا - ترجع رسالة ترحيب
/// Hello function - returns a greeting message
صدّر دالة مرحبا() -> نص {
    أرجع "مرحباً من المكتبة!"
}

/// دالة رئيسية للمكتبة (للاختبار)
/// Main function for the library (for testing)
صدّر دالة اختبار() {
    اطبع(مرحبا())
}
"#,
        )
    } else {
        (
            "مصدر/رئيسي.trq",
            r#"/// برنامج ترقيم رئيسي
/// Main Tarqeem program

/// الدالة الرئيسية - نقطة بداية البرنامج
/// Main function - program entry point
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

    // Create test file
    let test_path = "اختبارات/test.trq";
    let test_content = if lib {
        r#"/// اختبارات المكتبة
/// Library tests

استورد { مرحبا } من "../مصدر/lib"

/// اختبار دالة مرحبا
دالة اختبار_مرحبا() {
    متغير نتيجة = مرحبا()
    تأكيد(نتيجة == "مرحباً من المكتبة!")
}
"#
    } else {
        r#"/// اختبارات البرنامج
/// Program tests

/// اختبار بسيط
دالة اختبار_أساسي() {
    تأكيد(1 + 1 == 2)
}
"#
    };

    if !Path::new(test_path).exists() {
        fs::write(test_path, test_content)?;
        println!("  {} {}", "→".cyan(), test_path);
    }

    // Save manifest
    manifest.save(Path::new("حزمة.toml"))?;
    println!("  {} حزمة.toml", "→".cyan());

    // Create .gitignore
    let gitignore_content = r#"# Tarqeem build artifacts / مخلفات البناء
/بناء/
/build/
/target/

# Lock file (optional: some prefer to commit this)
# .trqlock

# Packages directory
/packages/

# IDE and editor files
.vscode/
.idea/
*.swp
*.swo
*~

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
