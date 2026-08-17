<div dir="rtl" align="right">

# trqdoc - مولد توثيق ترقيم

مولد التوثيق الرسمي للغة ترقيم. يستخرج التعليقات التوثيقية من الكود المصدري ويولد توثيقاً بصيغ متعددة.

</div>

# trqdoc - Tarqeem Documentation Generator

The official documentation generator for the Tarqeem programming language. Extracts doc comments from source code and generates documentation in multiple formats.

## Features

- **Arabic-First Design**: Full RTL support with Arabic UI elements
- **Multiple Output Formats**: HTML, Markdown, and JSON
- **Doc Comment Tags**: Support for both Arabic and English documentation tags
- **LSP Integration**: Doc comments appear in IDE hover information
- **Recursive Processing**: Generate docs for entire directories

## Usage

### Command Line

```bash
# Generate HTML documentation for a single file
tarqeem doc source.ترقيم

# Arabic alias
tarqeem توثيق source.ترقيم

# Specify output directory
tarqeem doc source.ترقيم -o ./docs

# Generate Markdown format
tarqeem doc source.ترقيم -f markdown

# Generate JSON format
tarqeem doc source.ترقيم -f json

# Generate single combined file
tarqeem doc ./src -o docs.html --single-file

# Generate docs for entire directory
tarqeem doc ./src -o ./docs -f html
```

### Command Options

| Option | Short | Description |
|--------|-------|-------------|
| `--output` | `-o` | Output directory or file path |
| `--format` | `-f` | Output format: `html`, `markdown`, `json` (default: `html`) |
| `--single-file` | | Generate single file instead of directory |

## Doc Comment Syntax

### Module Doc Comments

A `///` block placed directly after `بسم_الله` documents the **file**, and becomes
`Documentation.description` — the module summary the HTML and Markdown generators
render at the top of the page.

```tarqeem
بسم_الله

/// وحدة الرياضيات
///
/// @منذ ١.٠.٠

/// القيمة المطلقة
صدّر دالة مطلق(س: عدد) -> عدد {
}

الحمد_لله
```

It is read as the file's documentation when a nearer comment follows it, when
nothing follows it, or when what follows is an `استورد` / `صدّر * من` /
`صدّر { … }` that carries no documentation of its own. A `///` block sitting
directly above a declaration documents *that declaration*, as it always has —
which is why the blank line and the `//` banner in the example above matter.

Tags in a module doc are parsed like any other (`@منذ`, `@ملاحظة`, …), but only
the description text is rendered; the tags are not.

### Single-line Doc Comments

```tarqeem
/// هذا تعليق توثيقي
/// يمكن أن يمتد على عدة أسطر
دالة مثال() {
}
```

### Block Doc Comments

```tarqeem
/**
 * هذا تعليق توثيقي كتلي
 * يمكن أن يمتد على عدة أسطر
 */
دالة مثال() {
}
```

## Documentation Tags

### Arabic Tags (الوسوم العربية)

| Tag | Description | Example |
|-----|-------------|---------|
| `@معامل` | Parameter documentation | `@معامل اسم وصف المعامل` |
| `@أرجع` / `@ارجع` | Return value documentation | `@أرجع وصف القيمة المُرجعة` |
| `@مثال` | Code example | `@مثال متغير س = جمع(1، 2)` |
| `@انظر` | Related items | `@انظر دالة_أخرى` |
| `@ملاحظة` | Additional notes | `@ملاحظة هذه الدالة غير متزامنة` |
| `@تحذير` | Warnings | `@تحذير لا تستخدم مع قيم سالبة` |
| `@منذ` | Version introduced | `@منذ 1.0.0` |

### English Tags

| Tag | Description | Example |
|-----|-------------|---------|
| `@param` | Parameter documentation | `@param name Description` |
| `@returns` | Return value documentation | `@returns Description` |
| `@example` | Code example | `@example let x = add(1, 2)` |
| `@see` | Related items | `@see other_function` |
| `@note` | Additional notes | `@note This function is async` |
| `@warning` | Warnings | `@warning Don't use with negative values` |
| `@since` | Version introduced | `@since 1.0.0` |

### Typed Parameters

You can specify parameter types in doc comments:

```tarqeem
/// @معامل أ {عدد} العدد الأول
/// @معامل ب {عدد} العدد الثاني
/// @أرجع {عدد} مجموع العددين
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}
```

## Complete Example

```tarqeem
/// دالة لحساب مجموع عددين
///
/// تأخذ هذه الدالة عددين صحيحين وترجع مجموعهما.
///
/// @معامل أ العدد الأول
/// @معامل ب العدد الثاني
/// @أرجع مجموع العددين
/// @مثال
/// متغير نتيجة = جمع(5، 3)
/// اطبع(نتيجة)  // يطبع: 8
/// @انظر طرح، ضرب، قسمة
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}

/// صنف يمثل شخص
///
/// @مثال
/// متغير شخص = جديد شخص("أحمد"، 25)
/// اطبع(شخص.احصل_اسم())
صنف شخص {
    /// اسم الشخص
    خاص اسم: نص

    /// عمر الشخص
    خاص عمر: عدد

    /// إنشاء شخص جديد
    /// @معامل اسم اسم الشخص
    /// @معامل عمر عمر الشخص
    منشئ(اسم: نص، عمر: عدد) {
        هذا.اسم = اسم
        هذا.عمر = عمر
    }

    /// الحصول على اسم الشخص
    /// @أرجع اسم الشخص
    عام دالة احصل_اسم() -> نص {
        أرجع هذا.اسم
    }
}
```

## Output Formats

### HTML

- Full RTL support with Arabic styling
- Navigation sidebar
- Syntax-highlighted code examples
- Responsive design
- Index page for multi-file documentation

### Markdown

- GitHub-compatible format
- Table of contents
- Bilingual headers (Arabic/English)
- Code blocks with syntax highlighting hints

### JSON

- Machine-readable format
- Full documentation model
- Suitable for custom tooling
- Pretty-printed or minified output

## Module Architecture

```
src/doc/
├── mod.rs           # Module entry point
├── model.rs         # Documentation data structures
├── comment.rs       # Doc comment parser
├── extractor.rs     # AST to Documentation converter
└── generator/
    ├── mod.rs       # Generator traits and utilities
    ├── html.rs      # HTML generator with RTL support
    ├── markdown.rs  # Markdown generator
    └── json.rs      # JSON generator
```

## Programmatic Usage

```rust
use tarqeem::doc::{DocExtractor, HtmlGenerator, OutputFormat};
use tarqeem::doc::generator::DocGenerator;
use tarqeem::parser::Parser;

// Parse source code
let source = r#"
/// دالة للجمع
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}
"#;

let mut parser = Parser::new(source);
let ast = parser.parse().unwrap();

// Extract documentation
let extractor = DocExtractor::new("module".to_string(), "source.ترقيم".to_string());
let doc = extractor.extract(&ast);

// Generate HTML output
let generator = HtmlGenerator::new();
let mut output = Vec::new();
generator.generate(&doc, &mut output).unwrap();

let html = String::from_utf8(output).unwrap();
```

## LSP Integration

Doc comments are automatically extracted and displayed in IDE hover information when using the Tarqeem Language Server. Simply write doc comments above your declarations, and they will appear when hovering over symbols in your editor.

## Data Model

### Documentation
- `name`: Module/file name
- `description`: Module-level description, from the file's own `///` block
  (`Ast::module_doc`)
- `source_path`: Source file path
- `items`: List of documented items

### DocItem (enum)
- `Function(FunctionDoc)`
- `Class(ClassDoc)`
- `Interface(InterfaceDoc)`
- `Variable(VariableDoc)`

### FunctionDoc
- `name`: Function name
- `description`: Description text
- `params`: List of `ParamDoc`
- `returns`: Optional `ReturnDoc`
- `examples`: List of code examples
- `see_also`: List of related items
- `is_async`: Whether the function is async
- `is_exported`: Whether the function is exported
- `line`: Source line number

### ClassDoc
- `name`: Class name
- `description`: Description text
- `type_params`: Generic type parameters
- `extends`: Parent class (if any)
- `implements`: Implemented interfaces
- `fields`: List of `FieldDoc`
- `methods`: List of `MethodDoc`
- `constructor`: Optional `ConstructorDoc`
- `examples`: List of code examples

## Contributing

When adding new documentation features:

1. Update `model.rs` if new data structures are needed
2. Update `comment.rs` for new tag parsing
3. Update `extractor.rs` to extract new information from AST
4. Update all generators (`html.rs`, `markdown.rs`, `json.rs`)
5. Add tests for new functionality

## License

MIT License - See the main project LICENSE file.
