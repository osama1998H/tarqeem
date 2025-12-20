# Code Formatter (trqfmt) / منسق الكود

The `fmt` module provides an AST-based code formatter for Tarqeem source files. It ensures consistent code style across projects.

## Features / الميزات

- **AST-based formatting**: Parses source code to AST for accurate, semantic-aware formatting
- **Configurable**: All formatting rules are configurable via `.trqfmt.toml`
- **Bilingual support**: Configuration keys support both Arabic and English
- **Arabic punctuation**: Optional support for Arabic comma (،) and semicolon (؛)
- **CLI integration**: Full CLI support with check, diff, and in-place editing modes

## Usage / الاستخدام

### Command Line / سطر الأوامر

```bash
# Format file and print to stdout / تنسيق ملف وطباعته
tarqeem fmt file.trq

# Format file in-place / تنسيق ملف في مكانه
tarqeem fmt -w file.trq
tarqeem fmt --write file.trq

# Check if files are formatted (for CI) / فحص التنسيق
tarqeem fmt --check file.trq

# Show diff of formatting changes / عرض الفرق
tarqeem fmt --diff file.trq

# Format entire directory / تنسيق مجلد كامل
tarqeem fmt src/

# Use custom config file / استخدام ملف إعدادات مخصص
tarqeem fmt --config .trqfmt.toml file.trq

# Generate sample config / توليد نموذج إعدادات
tarqeem fmt --sample-config
```

### Programmatic API / الواجهة البرمجية

```rust
use tarqeem::fmt::{format_source, check_formatted, FormatConfig};

// Format source code
let source = "دالة اختبار(أ:عدد)->عدد{أرجع أ}";
let config = FormatConfig::default();
let formatted = format_source(source, &config)?;

// Check if already formatted
let is_formatted = check_formatted(source, &config)?;
```

## Configuration / الإعدادات

Create a `.trqfmt.toml` file in your project root:

```toml
# Indentation / المسافات البادئة
indent_size = 4        # حجم_المسافة
use_tabs = false       # استخدم_تاب

# Line length / طول السطر
max_line_length = 100  # اقصى_طول_سطر

# Brace style: "same_line" or "next_line"
# نمط الأقواس: "same_line" أو "next_line"
brace_style = "same_line"  # نمط_الأقواس

# Spacing / المسافات
space_after_comma = true        # مسافة_بعد_الفاصلة
space_around_operators = true   # مسافة_حول_العمليات
space_before_brace = true       # مسافة_قبل_القوس
space_after_colon = true        # مسافة_بعد_النقطتين

# Blank lines / الأسطر الفارغة
blank_lines_after_imports = 1       # أسطر_فارغة_بعد_الاستيراد
blank_lines_between_functions = 1   # أسطر_فارغة_بين_الدوال
max_blank_lines = 1                 # اقصى_أسطر_فارغة_متتالية

# Arabic-specific / خاص بالعربية
arabic_comma = false      # فاصلة_عربية (use ، instead of ,)
arabic_semicolon = false  # فاصلة_منقوطة_عربية (use ؛ instead of ;)

# Trailing / النهايات
trailing_comma = false    # فاصلة_نهائية
final_newline = true      # سطر_جديد_نهائي
```

## Formatting Examples / أمثلة التنسيق

### Before / قبل

```tarqeem
دالة حساب(أ:عدد،ب:عدد)->عدد{
متغير نتيجة=أ+ب
إذا(نتيجة>100){أرجع 100}
وإلا{أرجع نتيجة}
}
```

### After / بعد

```tarqeem
دالة حساب(أ: عدد, ب: عدد) -> عدد {
    متغير نتيجة = أ + ب
    إذا (نتيجة > 100) {
        أرجع 100
    } وإلا {
        أرجع نتيجة
    }
}
```

## Module Structure / هيكل الوحدة

| File | Purpose |
|------|---------|
| `mod.rs` | Module root, public API functions |
| `config.rs` | `FormatConfig` struct and TOML parsing |
| `formatter.rs` | AST traversal and formatting logic |
| `printer.rs` | Output generation with indentation handling |

## Config File Discovery / اكتشاف ملف الإعدادات

The formatter searches for configuration in this order:
1. `.trqfmt.toml` in current directory
2. `trqfmt.toml` in current directory
3. `تنسيق.toml` in current directory
4. Parent directories (up to root)

If no config file is found, default settings are used.

## Integration / التكامل

### LSP Integration

The formatter integrates with the Tarqeem LSP server for editor formatting support. Use `textDocument/formatting` to format documents.

### CI/CD

Use `--check` flag in CI pipelines:

```bash
tarqeem fmt --check src/ || exit 1
```

## Error Handling / معالجة الأخطاء

The formatter returns `FormatError` for:
- `ParseError`: Source code has syntax errors
- `IoError`: File read/write failures
- `ConfigError`: Invalid configuration file

## Performance / الأداء

- Parses source to AST once, then formats in a single pass
- Minimal memory allocation through string builders
- Suitable for formatting large codebases

## See Also / انظر أيضاً

- [Phase 4 Plan](../../../docs/PHASE4_PLAN.md) - Milestone 4.5 specification
- [CLAUDE.md](../../../CLAUDE.md) - Development guidelines
