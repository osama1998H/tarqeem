<div dir="rtl" align="right">

# إضافة ترقيم لـ VS Code

**دعم كامل للغة البرمجة العربية ترقيم في VS Code**

</div>

# Tarqeem VS Code Extension

**Full support for the Tarqeem Arabic programming language in VS Code**

---

## Features | الميزات

### Syntax Highlighting | تلوين الكود
- Full syntax highlighting for all Tarqeem keywords (Arabic & English)
- Support for Arabic identifiers and comments
- Proper handling of Arabic numerals (٠-٩)

### IntelliSense | الإكمال الذكي
- Auto-completion for keywords, types, and user symbols
- Context-aware suggestions
- Built-in function documentation

### Diagnostics | التشخيصات
- Real-time error and warning display
- Bilingual error messages (Arabic/English)
- Quick fixes and code actions

### Navigation | التنقل
- Go to Definition
- Find All References
- Document Outline (symbols)

### Code Editing | تحرير الكود
- Code formatting
- Symbol renaming
- Code snippets in Arabic and English
- Inlay hints for types

### Commands | الأوامر
- **تشغيل / Run** (`Ctrl+Shift+R`): Execute current file
- **ترجمة / Compile** (`Ctrl+Shift+B`): Compile to executable
- **تنسيق / Format** (`Shift+Alt+F`): Format code
- **فحص / Check**: Check for errors
- **REPL**: Open interactive console
- **مشروع جديد / New Project**: Create new Tarqeem project

---

## Installation | التثبيت

### From VS Code Marketplace
1. Open VS Code
2. Go to Extensions (Ctrl+Shift+X)
3. Search for "Tarqeem" or "ترقيم"
4. Click Install

### Manual Installation
```bash
# Clone the repository
git clone https://github.com/osama1998H/tarqeem.git
cd tarqeem/vscode-tarqeem

# Install dependencies
npm install

# Build the extension
npm run compile

# Package the extension
npm run package

# Install the .vsix file
code --install-extension tarqeem-0.1.0.vsix
```

---

## Requirements | المتطلبات

- VS Code 1.85.0 or later
- Tarqeem compiler installed and in PATH
  - Or configure `tarqeem.server.path` setting

### Installing Tarqeem Compiler
```bash
# From source
git clone https://github.com/osama1998H/tarqeem.git
cd tarqeem
cargo build --release
cargo install --path .

# Verify installation
tarqeem --version
```

---

## Configuration | الإعدادات

Configure the extension in VS Code settings:

| Setting | Description | Default |
|---------|-------------|---------|
| `tarqeem.server.path` | Path to Tarqeem executable | `"tarqeem"` |
| `tarqeem.language` | Error message language (ar/en/auto) | `"auto"` |
| `tarqeem.formatting.enabled` | Enable code formatting | `true` |
| `tarqeem.formatting.formatOnSave` | Format on save | `false` |
| `tarqeem.inlayHints.enabled` | Show type hints | `true` |
| `tarqeem.diagnostics.enabled` | Show diagnostics | `true` |
| `tarqeem.completion.enabled` | Enable auto-completion | `true` |

### Example settings.json
```json
{
  "tarqeem.language": "ar",
  "tarqeem.formatting.formatOnSave": true,
  "tarqeem.inlayHints.enabled": true,
  "[tarqeem]": {
    "editor.formatOnSave": true,
    "editor.unicodeHighlight.ambiguousCharacters": false
  }
}
```

---

## File Extensions | امتدادات الملفات

| Extension | Description |
|-----------|-------------|
| `.trq` | Tarqeem source files |
| `.ترقيم` | Tarqeem source files (Arabic) |
| `.trqh` | Tarqeem header files |
| `.ترقيم-ر` | Tarqeem header files (Arabic) |

---

## Snippets | مقاطع الكود

Type these prefixes to insert code snippets:

### Arabic Prefixes
| Prefix | Description |
|--------|-------------|
| `دالة` | Function definition |
| `صنف` | Class definition |
| `واجهة` | Interface definition |
| `متغير` | Variable declaration |
| `ثابت` | Constant declaration |
| `إذا` | If statement |
| `طالما` | While loop |
| `لكل` | For loop |
| `حاول` | Try-catch block |
| `استورد` | Import statement |
| `اطبع` | Print statement |
| `مرحبا` | Hello World |

### English Prefixes
| Prefix | Description |
|--------|-------------|
| `function`, `fn` | Function definition |
| `class` | Class definition |
| `interface` | Interface definition |
| `let`, `var` | Variable declaration |
| `const` | Constant declaration |
| `if` | If statement |
| `while` | While loop |
| `for` | For loop |
| `try` | Try-catch block |
| `import` | Import statement |
| `print` | Print statement |
| `hello` | Hello World |

---

## Keyboard Shortcuts | اختصارات لوحة المفاتيح

| Shortcut | Command |
|----------|---------|
| `Ctrl+Shift+R` | Run file |
| `Ctrl+Shift+B` | Compile file |
| `Shift+Alt+F` | Format document |
| `F12` | Go to Definition |
| `Shift+F12` | Find All References |
| `F2` | Rename Symbol |
| `Ctrl+Space` | Trigger Suggestions |
| `Ctrl+.` | Quick Fix |

---

## Example Code | مثال على الكود

```tarqeem
// مرحباً بالعالم
اطبع("مرحباً بالعالم!")

// متغيرات وأنماط
متغير اسم: نص = "أحمد"
ثابت عمر: عدد = 25

// دالة
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}

// صنف
صنف شخص {
    خاص اسم: نص
    خاص عمر: عدد

    منشئ(اسم: نص، عمر: عدد) {
        هذا.اسم = اسم
        هذا.عمر = عمر
    }

    عام دالة قدم_نفسك() {
        اطبع("أنا " + هذا.اسم + " وعمري " + هذا.عمر)
    }
}

// الاستخدام
متغير نتيجة = جمع(5، 3)
اطبع("النتيجة: " + نتيجة)

متغير شخص١ = جديد شخص("أحمد"، 30)
شخص١.قدم_نفسك()
```

---

## Troubleshooting | حل المشاكل

### Language server fails to start
1. Verify Tarqeem is installed: `tarqeem --version`
2. Check the `tarqeem.server.path` setting
3. Check the Output panel (View > Output > ترقيم - Tarqeem)

### No syntax highlighting
1. Ensure the file has `.trq` or `.ترقيم` extension
2. Check if the language mode is set to "Tarqeem" (bottom-right of VS Code)

### Arabic text displays incorrectly
1. Use a font with good Arabic support (e.g., Cascadia Code, Fira Code)
2. Set `editor.unicodeHighlight.ambiguousCharacters: false`

---

## Contributing | المساهمة

Contributions are welcome! Please see the [main repository](https://github.com/osama1998H/tarqeem) for guidelines.

---

## License | الرخصة

MIT License - see [LICENSE](LICENSE) for details.

---

<div align="center">

**ترقيم** - لغة البرمجة العربية

Made with ❤️ for Arabic developers

</div>
