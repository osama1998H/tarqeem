<div dir="rtl" align="right">

# مدير الحزم ترقيم (trqpm / حزم)

**Tarqeem Package Manager**

</div>

## Overview / نظرة عامة

`trqpm` (Tarqeem Package Manager) is the official package manager for the Tarqeem programming language. It provides dependency management, project initialization, building, and testing capabilities with full bilingual (Arabic/English) support.

## Installation / التثبيت

The package manager is built into the Tarqeem CLI. Access it via:

```bash
tarqeem pkg <command>
# or Arabic alias
tarqeem حزم <command>
```

## Quick Start / البداية السريعة

```bash
# Initialize a new project / إنشاء مشروع جديد
tarqeem pkg init my-project
cd my-project

# Add a dependency / إضافة اعتمادية
tarqeem pkg add json@1.0

# Install dependencies / تثبيت الاعتماديات
tarqeem pkg install

# Build the project / بناء المشروع
tarqeem pkg build

# Run the project / تشغيل المشروع
tarqeem pkg run
```

## Commands / الأوامر

### `init` / `هيئ` / `أنشئ`

Initialize a new Tarqeem package.

```bash
# Create a new binary package
tarqeem pkg init [name]

# Create a new library package
tarqeem pkg init [name] --lib
```

**Creates:**
- `حزمة.toml` - Package manifest
- `مصدر/رئيسي.trq` - Main entry point (binary) or `مصدر/lib.trq` (library)
- `اختبارات/test.trq` - Test file
- `.gitignore` - Git ignore patterns
- `README.md` - Project readme

### `add` / `أضف`

Add a dependency to the project.

```bash
# Add a dependency
tarqeem pkg add json

# Add with specific version
tarqeem pkg add json@1.0.0

# Add as dev dependency
tarqeem pkg add json --dev

# Add local path dependency
tarqeem pkg add my-lib --path ../my-lib
```

### `remove` / `أزل`

Remove a dependency from the project.

```bash
tarqeem pkg remove json

# Remove from dev dependencies
tarqeem pkg remove json --dev
```

### `install` / `ثبت`

Install all dependencies listed in the manifest.

```bash
# Install dependencies
tarqeem pkg install

# Force reinstall (ignore lockfile)
tarqeem pkg install --force
```

### `update` / `حدث`

Update dependencies to their latest compatible versions.

```bash
# Update all dependencies
tarqeem pkg update

# Update specific package
tarqeem pkg update json
```

### `build` / `ابن`

Build the package.

```bash
# Debug build
tarqeem pkg build

# Release build with optimizations
tarqeem pkg build --release
```

**Output directories:**
- Debug: `بناء/تطوير/`
- Release: `بناء/إصدار/`

### `run` / `شغل`

Build and run the package.

```bash
# Run in debug mode
tarqeem pkg run

# Run in release mode
tarqeem pkg run --release

# Pass arguments to the program
tarqeem pkg run -- arg1 arg2
```

### `test` / `اختبر`

Run package tests.

```bash
# Run all tests
tarqeem pkg test

# Run tests matching a filter
tarqeem pkg test --filter my_test
```

**Test locations:** `اختبارات/` or `tests/`

### `info` / `معلومات`

Display package information.

```bash
tarqeem pkg info
```

### `clean` / `نظف`

Clean build artifacts.

```bash
tarqeem pkg clean
```

**Removes:** `بناء/`, `build/`, `target/`

## Manifest Format / صيغة ملف الحزمة

The package manifest (`حزمة.toml` or `trq.toml`) supports bilingual keys:

```toml
# Arabic format
["حزمة"]
"اسم" = "مشروعي"
"نسخة" = "0.1.0"
"وصف" = "وصف المشروع"
"مؤلفون" = ["أحمد"]
"رخصة" = "MIT"
"مدخل" = "مصدر/رئيسي.trq"

["اعتماديات"]
json = "1.0"
"مجموعات" = "^2.0"

["اعتماديات_تطوير"]
test-utils = "0.1"
```

```toml
# English format
[package]
name = "my-project"
version = "0.1.0"
description = "Project description"
authors = ["Ahmed"]
license = "MIT"
entry = "src/main.trq"

[dependencies]
json = "1.0"
collections = "^2.0"

[dev-dependencies]
test-utils = "0.1"
```

### Package Metadata / بيانات الحزمة

| Arabic Key | English Key | Description |
|------------|-------------|-------------|
| `اسم` | `name` | Package name (required) |
| `نسخة` | `version` | Semantic version (required) |
| `وصف` | `description` | Package description |
| `مؤلفون` | `authors` | Author(s) |
| `رخصة` | `license` | License identifier |
| `مستودع` | `repository` | Repository URL |
| `موقع` | `homepage` | Homepage URL |
| `كلمات` | `keywords` | Search keywords |
| `مدخل` | `entry` | Binary entry point |
| `مكتبة` | `lib` | Library entry point |
| `ترقيم` | `tarqeem` | Minimum Tarqeem version |

### Dependency Specification / تحديد الاعتماديات

```toml
# Simple version
json = "1.0.0"

# Version constraints
json = "^1.0"    # Compatible with 1.x
json = "~1.0"    # Approximately 1.0.x
json = ">=1.0"   # At least 1.0
json = "*"       # Any version

# Detailed specification
["اعتماديات".my-lib]
"نسخة" = "1.0"
"مسار" = "../my-lib"  # Path dependency

# Git dependency
[dependencies.my-lib]
version = "1.0"
git = "https://github.com/user/repo"
branch = "main"  # or tag, rev
```

## Lock File / ملف القفل

The `.trqlock` file ensures reproducible builds by locking exact versions:

```toml
version = 1

[root]
name = "my-project"
version = "0.1.0"

[[packages]]
name = "json"
version = "1.2.3"
checksum = "sha256:abc123..."

[packages.source]
type = "registry"
url = "https://registry.tarqeem.dev"

[packages.dependencies]
utils = "0.5.0"
```

## Cache / التخزين المؤقت

Downloaded packages are cached at:
- Linux/macOS: `~/.cache/tarqeem/packages/`
- Windows: `%LOCALAPPDATA%\tarqeem\packages\`

To clear the cache:
```bash
rm -rf ~/.cache/tarqeem/packages
```

## Project Structure / هيكل المشروع

Recommended project layout:

```
my-project/
├── حزمة.toml          # Package manifest
├── .trqlock           # Lock file (auto-generated)
├── .gitignore
├── README.md
├── مصدر/              # Source code
│   └── رئيسي.trq      # Main entry point
├── اختبارات/          # Tests
│   └── test.trq
├── أمثلة/             # Examples (libraries)
│   └── example.trq
├── توثيق/             # Documentation
│   └── guide.md
├── packages/          # Installed dependencies
│   └── json/
└── بناء/              # Build output
    ├── تطوير/         # Debug builds
    └── إصدار/         # Release builds
```

## Environment Variables / متغيرات البيئة

| Variable | Description |
|----------|-------------|
| `TARQEEM_REGISTRY` | Custom registry URL |
| `TARQEEM_CACHE_DIR` | Custom cache directory |

## Error Messages / رسائل الخطأ

All error messages are provided in both Arabic and English:

```
Manifest not found (حزمة.toml or trq.toml) / لم يتم العثور على ملف الحزمة
Package not found: json / الحزمة غير موجودة: json
Version not found: json@2.0 / النسخة غير موجودة: json@2.0
Circular dependency detected: A -> B -> A / اعتمادية دائرية
```

## API / واجهة البرمجة

The package module can be used programmatically:

```rust
use tarqeem::package::{Manifest, Cache, Resolver};

// Parse manifest
let (manifest, path) = Manifest::find_and_parse()?;

// Resolve dependencies
let cache = Cache::new()?;
let mut resolver = Resolver::new(&cache);
let packages = resolver.resolve(&manifest)?;

// Install packages
for pkg in &packages {
    cache.download_and_verify(pkg)?;
}
```

## Versioning / الإصدارات

The package manager follows [Semantic Versioning](https://semver.org/):

- `MAJOR.MINOR.PATCH` (e.g., `1.2.3`)
- `^1.0` - Compatible updates (1.x.x)
- `~1.0` - Patch updates only (1.0.x)
- `*` or `latest` - Any version

## Contributing / المساهمة

See the main project [CLAUDE.md](../../../CLAUDE.md) for contribution guidelines.

## License / الرخصة

MIT License - see [LICENSE](../../../LICENSE) for details.
