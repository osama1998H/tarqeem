# المرحلة الرابعة: الأدوات
# Phase 4: Tooling

## نظرة عامة | Overview

المرحلة الرابعة تحول ترقيم من مترجم سطر أوامر إلى نظام بيئي متكامل للتطوير مع أدوات احترافية. تغطي هذه المرحلة:

Phase 4 transforms Tarqeem from a command-line compiler into a complete development ecosystem with professional tooling. This phase covers:

1. **مدير الحزم** - Package Manager (حزم/trqpm)
2. **خادم LSP** - Language Server Protocol implementation
3. **إضافة VS Code** - VS Code Extension with full Arabic support
4. **مولد التوثيق** - Documentation Generator (توثيق/trqdoc)

---

## الحالة الحالية للتعليقات | Current Comment Status

> **ملاحظة مهمة**: ترقيم يدعم التعليقات حالياً (كل من `//` و `/* */` مع التداخل)، لكن التعليقات يتم تجاهلها بالكامل أثناء التحليل اللغوي. لا توجد بنية تحتية للتعليقات التوثيقية (docstrings) - هذه ميزة أدوات مخططة للمرحلة الرابعة.

> **Important Note**: Tarqeem has functional comment support (both `//` and `/* */` with nesting), but comments are purely discarded during lexing. There's no docstring or documentation comment infrastructure - this is a Phase 4 tooling feature.

### التغييرات المطلوبة للتعليقات التوثيقية | Changes Required for Doc Comments

1. **المحلل اللغوي (Lexer)**: تعديل لالتقاط تعليقات `///` و `/** */` كرموز خاصة بدلاً من تجاهلها
2. **المحلل النحوي (Parser)**: ربط التعليقات التوثيقية بالعناصر التالية (دوال، أصناف، إلخ)
3. **AST**: إضافة حقل `doc_comment: Option<String>` للعناصر القابلة للتوثيق
4. **التحليل الدلالي**: الحفاظ على التعليقات التوثيقية خلال التحليل
5. **مولد التوثيق**: تحليل واستخراج المعلومات من التعليقات التوثيقية

---

## المتطلبات الأساسية | Prerequisites

### من المراحل السابقة | From Previous Phases

| المتطلب | الحالة | ملاحظات |
|---------|--------|---------|
| بنية المترجم الكاملة | ✅ مكتمل | Lexer, Parser, Semantic, IR, Codegen |
| نظام الوحدات | ✅ مكتمل | استورد/صدّر working |
| المكتبة القياسية | ✅ مكتمل | مجموعات، نص، رياضيات، ملفات، شبكة، وقت، أخطاء |
| مكتبة التشغيل C | ✅ مكتمل | memory, strings, arrays, I/O, networking |
| المفسر | ✅ مكتمل | IR-based execution |
| 108+ اختبارات | ✅ مكتمل | All tests passing |

### متطلبات تقنية | Technical Requirements

| المتطلب | الغرض |
|---------|-------|
| Rust 1.70+ | Async support, latest features |
| tower-lsp | LSP server implementation |
| serde/serde_json | JSON serialization |
| toml | Package manifest parsing |
| reqwest | HTTP client for registry |
| tokio | Async runtime |

---

## معالم المرحلة الرابعة | Phase 4 Milestones

### المعلم 4.1: مدير الحزم (حزم)
### Milestone 4.1: Package Manager (trqpm)

**الهدف**: إنشاء نظام إدارة حزم كامل للغة ترقيم
**Goal**: Create a complete package management system for Tarqeem

#### 4.1.1: هيكل الحزمة | Package Structure

```
my-package/
├── حزمة.toml              # Package manifest (Arabic)
├── trq.toml               # Package manifest (English alias)
├── مصدر/                  # Source directory
│   ├── رئيسي.trq          # Main entry point
│   └── lib.trq            # Library entry point
├── اختبارات/              # Tests directory
│   └── *.trq
├── أمثلة/                 # Examples
│   └── *.trq
├── توثيق/                 # Documentation
│   └── *.md
└── .trqlock               # Lock file
```

#### 4.1.2: ملف الحزمة | Package Manifest (حزمة.toml)

```toml
[حزمة]
اسم = "مكتبتي"
نسخة = "0.1.0"
وصف = "مكتبة رائعة للغة ترقيم"
مؤلف = "أحمد محمد <ahmed@example.com>"
رخصة = "MIT"
مستودع = "https://github.com/user/my-package"
كلمات = ["أدوات", "مساعدة"]

# English alias section
[package]
name = "my-library"
version = "0.1.0"
description = "An awesome Tarqeem library"
author = "Ahmed Mohamed <ahmed@example.com>"
license = "MIT"

[اعتماديات]
# Dependencies
json = "1.0"
"مجموعات-إضافية" = "2.1"

[اعتماديات-تطوير]
# Dev dependencies
"اختبارات-إضافية" = "0.5"

[سكربتات]
# Scripts
بناء = "tarqeem compile مصدر/رئيسي.trq"
اختبار = "tarqeem test"
```

#### 4.1.3: أوامر مدير الحزم | Package Manager Commands

```bash
# تهيئة مشروع جديد | Initialize new project
trqpm init اسم-المشروع
trqpm init project-name

# إضافة اعتمادية | Add dependency
trqpm add json
trqpm add "مجموعات-إضافية"@2.1

# إزالة اعتمادية | Remove dependency
trqpm remove json

# تثبيت الاعتماديات | Install dependencies
trqpm install

# تحديث الاعتماديات | Update dependencies
trqpm update

# بناء المشروع | Build project
trqpm build
trqpm build --release

# تشغيل المشروع | Run project
trqpm run

# تشغيل الاختبارات | Run tests
trqpm test

# نشر الحزمة | Publish package
trqpm publish

# البحث عن حزم | Search packages
trqpm search json
trqpm بحث json

# معلومات حزمة | Package info
trqpm info json
```

#### 4.1.4: هيكل التنفيذ | Implementation Structure

```
src/
├── cli/
│   └── commands/
│       └── pm/                    # Package manager commands
│           ├── mod.rs
│           ├── init.rs            # Project initialization
│           ├── add.rs             # Add dependencies
│           ├── remove.rs          # Remove dependencies
│           ├── install.rs         # Install dependencies
│           ├── update.rs          # Update dependencies
│           ├── build.rs           # Build project
│           ├── run.rs             # Run project
│           ├── test.rs            # Run tests
│           ├── publish.rs         # Publish to registry
│           └── search.rs          # Search registry
│
├── package/                       # Package management core
│   ├── mod.rs
│   ├── manifest.rs                # حزمة.toml parsing
│   ├── lockfile.rs                # Lock file management
│   ├── resolver.rs                # Dependency resolution
│   ├── registry.rs                # Registry client
│   ├── cache.rs                   # Package cache
│   └── builder.rs                 # Package builder
```

#### 4.1.5: سجل الحزم | Package Registry

**Registry API**:

```
GET  /api/v1/packages                    # List packages
GET  /api/v1/packages/:name              # Get package info
GET  /api/v1/packages/:name/:version     # Get specific version
POST /api/v1/packages                    # Publish package
GET  /api/v1/packages/:name/versions     # List versions
GET  /api/v1/search?q=:query             # Search packages
```

**Registry Data Model**:

```rust
struct Package {
    name: String,           // "json" or "مجموعات-إضافية"
    name_normalized: String, // NFC normalized for comparison
    versions: Vec<Version>,
    description: String,
    description_ar: Option<String>,
    author: String,
    repository: Option<String>,
    keywords: Vec<String>,
    downloads: u64,
}

struct Version {
    version: String,        // semver: "1.2.3"
    tarball_url: String,
    checksum: String,       // SHA256
    dependencies: HashMap<String, String>,
    published_at: DateTime<Utc>,
}
```

**المخرجات | Deliverables**:
- ✅ `trqpm init` يعمل بالعربية والإنجليزية
- ✅ حل الاعتماديات (dependency resolution)
- ✅ ملف القفل (.trqlock)
- ✅ التخزين المؤقت للحزم
- ✅ التكامل مع أمر `tarqeem`

---

### المعلم 4.2: خادم LSP
### Milestone 4.2: Language Server Protocol

**الهدف**: تمكين دعم IDE احترافي لترقيم
**Goal**: Enable professional IDE support for Tarqeem

#### 4.2.1: ميزات LSP | LSP Features

| الميزة | الوصف | الأولوية |
|--------|-------|----------|
| تشخيصات | أخطاء وتحذيرات فورية | P0 |
| إكمال تلقائي | اقتراحات أثناء الكتابة | P0 |
| انتقال للتعريف | Go to Definition | P0 |
| إظهار المعلومات | Hover information | P0 |
| إعادة التسمية | Rename symbol | P1 |
| البحث عن المراجع | Find References | P1 |
| تنسيق الكود | Code Formatting | P1 |
| إجراءات الكود | Code Actions (Quick Fix) | P1 |
| مخطط المستند | Document Outline | P2 |
| طي الكود | Code Folding | P2 |
| Inlay Hints | Parameter names, types | P2 |

#### 4.2.2: هيكل LSP | LSP Structure

```
src/
├── lsp/
│   ├── mod.rs                     # LSP server main
│   ├── server.rs                  # Server implementation
│   ├── capabilities.rs            # Server capabilities
│   ├── state.rs                   # Server state management
│   │
│   ├── handlers/                  # Request handlers
│   │   ├── mod.rs
│   │   ├── initialize.rs          # Initialize/shutdown
│   │   ├── diagnostics.rs         # Publish diagnostics
│   │   ├── completion.rs          # Auto-completion
│   │   ├── hover.rs               # Hover information
│   │   ├── definition.rs          # Go to definition
│   │   ├── references.rs          # Find references
│   │   ├── rename.rs              # Rename symbol
│   │   ├── formatting.rs          # Code formatting
│   │   ├── code_action.rs         # Quick fixes
│   │   ├── document_symbol.rs     # Document outline
│   │   └── folding.rs             # Code folding
│   │
│   ├── analysis/                  # Incremental analysis
│   │   ├── mod.rs
│   │   ├── document.rs            # Document management
│   │   ├── index.rs               # Symbol indexing
│   │   └── cache.rs               # Analysis cache
│   │
│   └── utils/
│       ├── mod.rs
│       ├── position.rs            # Position conversions
│       └── uri.rs                 # URI handling
```

#### 4.2.3: تحليل تزايدي | Incremental Analysis

```rust
/// Manages document state for incremental compilation
struct DocumentState {
    uri: Url,
    version: i32,
    content: String,

    // Cached analysis results
    tokens: Option<Vec<Token>>,
    ast: Option<Program>,
    typed_ast: Option<TypedProgram>,
    diagnostics: Vec<Diagnostic>,

    // Dependencies
    imports: HashSet<ModuleId>,
    dependents: HashSet<Url>,
}

/// Incremental analysis engine
struct AnalysisEngine {
    documents: HashMap<Url, DocumentState>,
    global_symbols: SymbolIndex,

    /// Re-analyze only changed documents and dependents
    fn on_change(&mut self, uri: &Url, content: String) {
        // 1. Update document content
        // 2. Re-lex and re-parse
        // 3. Re-analyze semantics
        // 4. Propagate to dependents
        // 5. Publish diagnostics
    }
}
```

#### 4.2.4: اكتمال ذكي | Smart Completion

```rust
enum CompletionContext {
    TopLevel,           // Top-level declarations
    InFunction,         // Inside function body
    InClass,            // Inside class body
    AfterDot,           // Member access
    AfterColon,         // Type annotation
    InImport,           // Import statement
}

fn get_completions(context: CompletionContext, prefix: &str) -> Vec<CompletionItem> {
    match context {
        CompletionContext::TopLevel => {
            // Keywords: دالة، صنف، واجهة، متغير، ثابت، استورد
            // User-defined types
        }
        CompletionContext::AfterDot => {
            // Object members
            // Method calls
        }
        CompletionContext::InImport => {
            // Available modules
            // Exported symbols from modules
        }
        // ...
    }
}
```

#### 4.2.5: أمثلة الإكمال | Completion Examples

```tarqeem
// اكتمال الكلمات المفتاحية
دا|  // → دالة، داخلي
صن|  // → صنف

// اكتمال الأعضاء
متغير ق = جديد قائمة<عدد>()
ق.|  // → أضف()، احذف()، طول()، فارغة()، ...

// اكتمال الاستيراد
استورد { | } من "مجموعات"
// → قائمة، مجموعة، خريطة، طابور، مكدس

// اكتمال الأنماط
متغير س: |  // → عدد، عدد_عشري، نص، منطقي، مصفوفة، ...
```

**المخرجات | Deliverables**:
- ✅ خادم LSP يعمل مع VS Code
- ✅ تشخيصات فورية
- ✅ اكتمال تلقائي ذكي
- ✅ معلومات hover بالعربية
- ✅ الانتقال للتعريف
- ✅ البحث عن المراجع
- ✅ إعادة التسمية

---

### المعلم 4.3: إضافة VS Code
### Milestone 4.3: VS Code Extension

**الهدف**: توفير تجربة تطوير متكاملة في VS Code
**Goal**: Provide an integrated development experience in VS Code

#### 4.3.1: ميزات الإضافة | Extension Features

| الميزة | الوصف |
|--------|-------|
| تلوين الكود | Syntax highlighting for Tarqeem |
| دعم RTL | Right-to-left text support |
| دعم LSP | Full LSP client integration |
| مقاطع كود | Code snippets in Arabic |
| تشغيل وتصحيح | Run and debug support |
| عرض الأخطاء | Error lens integration |
| تنسيق تلقائي | Format on save |
| تشخيص مباشر | Inline diagnostics |

#### 4.3.2: هيكل الإضافة | Extension Structure

```
vscode-tarqeem/
├── package.json               # Extension manifest
├── tsconfig.json             # TypeScript config
├── webpack.config.js         # Build config
│
├── src/
│   ├── extension.ts          # Extension entry point
│   ├── client.ts             # LSP client
│   ├── commands.ts           # VS Code commands
│   ├── debug.ts              # Debug adapter
│   └── rtl.ts                # RTL support utilities
│
├── syntaxes/
│   └── tarqeem.tmLanguage.json  # TextMate grammar
│
├── snippets/
│   └── tarqeem.json          # Code snippets
│
├── language-configuration.json  # Language config
│
└── media/
    └── icon.png              # Extension icon
```

#### 4.3.3: تعريف القواعد | Grammar Definition

```json
{
  "name": "Tarqeem",
  "scopeName": "source.tarqeem",
  "patterns": [
    { "include": "#keywords" },
    { "include": "#strings" },
    { "include": "#comments" },
    { "include": "#numbers" },
    { "include": "#operators" }
  ],
  "repository": {
    "keywords": {
      "patterns": [
        {
          "name": "keyword.control.tarqeem",
          "match": "\\b(إذا|وإلا|طالما|لكل|في|تطابق|حالة|غير_ذلك|أرجع|توقف|تابع|حاول|التقط|أخيراً|ارمِ)\\b"
        },
        {
          "name": "keyword.declaration.tarqeem",
          "match": "\\b(دالة|صنف|واجهة|متغير|ثابت|عام|خاص|محمي|ثابت_صنف)\\b"
        },
        {
          "name": "keyword.other.tarqeem",
          "match": "\\b(استورد|صدّر|من|كـ|يرث|يطبق|منشئ|هذا|أساس|جديد|غير_متزامن|انتظر)\\b"
        },
        {
          "name": "constant.language.tarqeem",
          "match": "\\b(صحيح|خطأ|عدم|true|false|null|none)\\b"
        }
      ]
    },
    "types": {
      "patterns": [
        {
          "name": "support.type.tarqeem",
          "match": "\\b(عدد|عدد_عشري|نص|منطقي|مصفوفة|قاموس|فراغ|أي)\\b"
        }
      ]
    },
    "strings": {
      "patterns": [
        {
          "name": "string.quoted.double.tarqeem",
          "begin": "\"",
          "end": "\"",
          "patterns": [
            {
              "name": "constant.character.escape.tarqeem",
              "match": "\\\\."
            }
          ]
        }
      ]
    },
    "comments": {
      "patterns": [
        {
          "name": "comment.line.double-slash.tarqeem",
          "match": "//.*$"
        },
        {
          "name": "comment.block.tarqeem",
          "begin": "/\\*",
          "end": "\\*/"
        }
      ]
    }
  }
}
```

#### 4.3.4: مقاطع الكود | Code Snippets

```json
{
  "دالة": {
    "prefix": ["دالة", "function", "fn"],
    "body": [
      "دالة ${1:اسم_الدالة}(${2:معاملات}) -> ${3:نوع_الإرجاع} {",
      "\t$0",
      "}"
    ],
    "description": "تعريف دالة جديدة"
  },
  "صنف": {
    "prefix": ["صنف", "class"],
    "body": [
      "صنف ${1:اسم_الصنف} {",
      "\tمنشئ(${2:معاملات}) {",
      "\t\t$0",
      "\t}",
      "}"
    ],
    "description": "تعريف صنف جديد"
  },
  "إذا": {
    "prefix": ["إذا", "if"],
    "body": [
      "إذا (${1:شرط}) {",
      "\t$0",
      "}"
    ],
    "description": "جملة شرطية"
  },
  "لكل": {
    "prefix": ["لكل", "for"],
    "body": [
      "لكل ${1:عنصر} في ${2:مجموعة} {",
      "\t$0",
      "}"
    ],
    "description": "حلقة لكل-في"
  },
  "حاول": {
    "prefix": ["حاول", "try"],
    "body": [
      "حاول {",
      "\t$0",
      "} التقط (${1:خطأ}) {",
      "\t",
      "}"
    ],
    "description": "معالجة الأخطاء"
  },
  "مرحبا": {
    "prefix": ["مرحبا", "hello"],
    "body": [
      "اطبع(\"مرحباً بالعالم!\")"
    ],
    "description": "برنامج مرحباً بالعالم"
  }
}
```

#### 4.3.5: دعم RTL | RTL Support

```typescript
// src/rtl.ts

import * as vscode from 'vscode';

export function configureRTLSupport(context: vscode.ExtensionContext) {
    // Register editor decoration for RTL hints
    const rtlDecorationType = vscode.window.createTextEditorDecorationType({
        // Visual hints for RTL content
    });

    // Configure editor settings for .trq files
    vscode.workspace.getConfiguration('editor', { languageId: 'tarqeem' })
        .update('unicodeHighlight.ambiguousCharacters', false);

    // Handle bidirectional text in completions
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider('tarqeem', {
            provideCompletionItems(document, position) {
                // Ensure completions display correctly in RTL
            }
        })
    );
}
```

#### 4.3.6: أوامر الإضافة | Extension Commands

```typescript
// package.json commands
{
  "contributes": {
    "commands": [
      {
        "command": "tarqeem.run",
        "title": "تشغيل الملف",
        "category": "ترقيم"
      },
      {
        "command": "tarqeem.compile",
        "title": "ترجمة الملف",
        "category": "ترقيم"
      },
      {
        "command": "tarqeem.format",
        "title": "تنسيق الكود",
        "category": "ترقيم"
      },
      {
        "command": "tarqeem.newProject",
        "title": "مشروع جديد",
        "category": "ترقيم"
      },
      {
        "command": "tarqeem.openRepl",
        "title": "فتح REPL",
        "category": "ترقيم"
      }
    ]
  }
}
```

**المخرجات | Deliverables**:
- ✅ تلوين كود كامل مع دعم RTL
- ✅ تكامل LSP كامل
- ✅ مقاطع كود عربية
- ✅ أوامر تشغيل وترجمة
- ✅ دعم تصحيح الأخطاء
- ✅ نشر على VS Code Marketplace

---

### المعلم 4.4: مولد التوثيق (توثيق)
### Milestone 4.4: Documentation Generator (trqdoc)

**الهدف**: إنشاء توثيق تلقائي من الكود المصدري
**Goal**: Generate documentation automatically from source code

#### 4.4.0: البنية التحتية للتعليقات التوثيقية | Doc Comment Infrastructure

> **متطلب أساسي**: حالياً التعليقات يتم تجاهلها في المحلل اللغوي. يجب تعديل المترجم لدعم التعليقات التوثيقية.

**التغييرات المطلوبة في المترجم | Compiler Changes Required**:

1. **تعديل المحلل اللغوي** (`src/lexer/lexer.rs`):
   ```rust
   // إضافة رمز جديد للتعليقات التوثيقية
   pub enum TokenKind {
       // ... existing tokens ...
       DocComment(String),      // /// تعليق توثيقي
       BlockDocComment(String), // /** تعليق توثيقي */
   }

   // في دالة scan_token():
   '/' => {
       if self.match_char('/') {
           if self.match_char('/') {
               // /// doc comment
               return self.scan_doc_comment();
           }
           self.skip_line_comment();
           return None; // Skip regular comments
       } else if self.match_char('*') {
           if self.match_char('*') {
               // /** block doc comment */
               return self.scan_block_doc_comment();
           }
           self.skip_block_comment();
           return None;
       }
       // ...
   }
   ```

2. **تعديل AST** (`src/parser/ast.rs`):
   ```rust
   pub struct FuncDecl {
       pub name: String,
       pub params: Vec<Param>,
       pub return_type: Option<Type>,
       pub body: Block,
       pub doc: Option<DocComment>,  // NEW
   }

   pub struct ClassDecl {
       pub name: String,
       // ... other fields ...
       pub doc: Option<DocComment>,  // NEW
   }

   pub struct DocComment {
       pub content: String,
       pub span: Span,
   }
   ```

3. **تعديل المحلل النحوي** (`src/parser/parser.rs`):
   ```rust
   fn parse_function_declaration(&mut self) -> ParseResult<Stmt> {
       // التقاط التعليق التوثيقي إن وجد
       let doc = self.consume_doc_comment();

       self.consume(&TokenKind::Keyword(Keyword::Dalah))?;
       // ... rest of function parsing ...

       Ok(Stmt::FuncDecl(FuncDecl {
           // ... other fields ...
           doc,
       }))
   }

   fn consume_doc_comment(&mut self) -> Option<DocComment> {
       if let Some(TokenKind::DocComment(content)) = self.peek_kind() {
           let span = self.current_span();
           self.advance();
           Some(DocComment { content, span })
       } else {
           None
       }
   }
   ```

4. **الحفاظ على التعليقات في التحليل الدلالي** (`src/semantic/analyzer.rs`):
   - التعليقات التوثيقية يجب أن تمر خلال التحليل بدون تعديل
   - إضافة حقل `doc` في `Symbol` للوصول إليها لاحقاً

**المخرجات | Deliverables**:
- ✅ المحلل اللغوي يلتقط `///` و `/** */`
- ✅ AST يحتوي على التعليقات التوثيقية
- ✅ التعليقات مرتبطة بالعناصر الصحيحة
- ✅ التعليقات متاحة لمولد التوثيق

---

#### 4.4.1: صيغة التعليقات التوثيقية | Doc Comment Format

```tarqeem
/// دالة لحساب مضروب العدد
///
/// @معامل ن - العدد المراد حساب مضروبه
/// @ارجاع - مضروب العدد
/// @مثال
/// ```
/// متغير نتيجة = عاملي(5)
/// اطبع(نتيجة)  // 120
/// ```
/// @راجع قوة، جذر
دالة عاملي(ن: عدد) -> عدد {
    إذا (ن <= 1) {
        أرجع 1
    }
    أرجع ن * عاملي(ن - 1)
}

/// صنف يمثل نقطة في المستوى الديكارتي
///
/// @مثال
/// ```
/// متغير نقطة = جديد نقطة(3، 4)
/// اطبع(نقطة.مسافة())  // 5
/// ```
صنف نقطة {
    /// الإحداثي الأفقي
    عام س: عدد_عشري

    /// الإحداثي الرأسي
    عام ص: عدد_عشري

    /// إنشاء نقطة جديدة
    /// @معامل س - الإحداثي الأفقي
    /// @معامل ص - الإحداثي الرأسي
    منشئ(س: عدد_عشري، ص: عدد_عشري) {
        هذا.س = س
        هذا.ص = ص
    }

    /// حساب المسافة من نقطة الأصل
    /// @ارجاع - المسافة من نقطة الأصل
    عام دالة مسافة() -> عدد_عشري {
        أرجع جذر(هذا.س * هذا.س + هذا.ص * هذا.ص)
    }
}
```

#### 4.4.2: علامات التوثيق | Documentation Tags

| العلامة | الوصف | English Alias |
|---------|-------|---------------|
| @معامل | وصف معامل الدالة | @param |
| @ارجاع | وصف القيمة المرجعة | @returns |
| @مثال | مثال على الاستخدام | @example |
| @راجع | إشارة إلى عناصر أخرى | @see |
| @تحذير | تحذير مهم | @warning |
| @ملاحظة | ملاحظة إضافية | @note |
| @منذ | الإصدار الذي أضيفت فيه | @since |
| @مهمل | علامة الإهمال | @deprecated |
| @خطأ | الأخطاء المحتملة | @throws |

#### 4.4.3: هيكل التنفيذ | Implementation Structure

```
src/
├── doc/
│   ├── mod.rs                 # Documentation module
│   ├── parser.rs              # Doc comment parser
│   ├── model.rs               # Documentation model
│   ├── generator/
│   │   ├── mod.rs
│   │   ├── html.rs            # HTML generator
│   │   ├── markdown.rs        # Markdown generator
│   │   └── json.rs            # JSON generator
│   ├── templates/             # HTML templates
│   │   ├── base.html
│   │   ├── module.html
│   │   ├── class.html
│   │   ├── function.html
│   │   └── styles.css
│   └── theme/
│       ├── light.css
│       └── dark.css
```

#### 4.4.4: أوامر التوثيق | Documentation Commands

```bash
# توليد التوثيق | Generate documentation
trqdoc مصدر/
trqdoc src/ --output docs/

# تحديد الصيغة | Specify format
trqdoc src/ --format html
trqdoc src/ --format markdown
trqdoc src/ --format json

# تحديد السمة | Specify theme
trqdoc src/ --theme dark

# تضمين التوثيق الخاص | Include private docs
trqdoc src/ --document-private

# فتح في المتصفح | Open in browser
trqdoc src/ --open

# خادم توثيق مباشر | Live documentation server
trqdoc serve src/
```

#### 4.4.5: نموذج التوثيق | Documentation Model

```rust
/// Parsed documentation for a symbol
struct Documentation {
    /// Main description (Arabic and/or English)
    description: String,

    /// Tagged sections
    params: Vec<ParamDoc>,
    returns: Option<String>,
    examples: Vec<Example>,
    see_also: Vec<String>,
    notes: Vec<String>,
    warnings: Vec<String>,
    since: Option<String>,
    deprecated: Option<String>,
    throws: Vec<String>,
}

struct ParamDoc {
    name: String,
    description: String,
    type_name: Option<String>,
}

struct Example {
    code: String,
    description: Option<String>,
}

/// Module documentation
struct ModuleDoc {
    name: String,
    path: PathBuf,
    description: Option<Documentation>,

    functions: Vec<FunctionDoc>,
    classes: Vec<ClassDoc>,
    interfaces: Vec<InterfaceDoc>,
    constants: Vec<ConstantDoc>,

    submodules: Vec<ModuleDoc>,
}

struct FunctionDoc {
    name: String,
    signature: String,
    doc: Option<Documentation>,
    visibility: Visibility,
}

struct ClassDoc {
    name: String,
    doc: Option<Documentation>,
    visibility: Visibility,

    fields: Vec<FieldDoc>,
    methods: Vec<MethodDoc>,
    constructor: Option<MethodDoc>,

    extends: Option<String>,
    implements: Vec<String>,
}
```

#### 4.4.6: مثال على الإخراج | Example Output

```html
<!DOCTYPE html>
<html dir="rtl" lang="ar">
<head>
    <meta charset="UTF-8">
    <title>توثيق مكتبة رياضيات</title>
    <link rel="stylesheet" href="styles.css">
</head>
<body>
    <nav class="sidebar">
        <h2>رياضيات</h2>
        <ul>
            <li><a href="#دوال">الدوال</a></li>
            <li><a href="#ثوابت">الثوابت</a></li>
        </ul>
    </nav>

    <main>
        <h1>مكتبة رياضيات</h1>
        <p>مكتبة الدوال الرياضية الأساسية.</p>

        <section id="دوال">
            <h2>الدوال</h2>

            <article class="function">
                <h3>عاملي</h3>
                <pre class="signature">دالة عاملي(ن: عدد) -> عدد</pre>
                <p>دالة لحساب مضروب العدد.</p>

                <h4>المعاملات</h4>
                <dl>
                    <dt>ن</dt>
                    <dd>العدد المراد حساب مضروبه</dd>
                </dl>

                <h4>القيمة المرجعة</h4>
                <p>مضروب العدد</p>

                <h4>مثال</h4>
                <pre class="example">
متغير نتيجة = عاملي(5)
اطبع(نتيجة)  // 120
                </pre>
            </article>
        </section>
    </main>
</body>
</html>
```

**المخرجات | Deliverables**:
- ✅ محلل تعليقات التوثيق
- ✅ توليد HTML مع دعم RTL
- ✅ توليد Markdown
- ✅ توليد JSON للتكامل
- ✅ سمات فاتحة وداكنة
- ✅ خادم توثيق مباشر

---

### المعلم 4.5: منسق الكود (تنسيق)
### Milestone 4.5: Code Formatter (trqfmt)

**الهدف**: تنسيق كود ترقيم تلقائياً بشكل موحد
**Goal**: Automatically format Tarqeem code consistently

#### 4.5.1: قواعد التنسيق | Formatting Rules

```rust
struct FormatConfig {
    // المسافات البادئة | Indentation
    indent_size: usize,              // Default: 4
    use_tabs: bool,                  // Default: false

    // طول السطر | Line length
    max_line_length: usize,          // Default: 100

    // الأقواس | Braces
    brace_style: BraceStyle,         // Same line vs new line

    // المسافات | Spacing
    space_after_comma: bool,         // Default: true
    space_around_operators: bool,    // Default: true
    space_before_brace: bool,        // Default: true

    // الأسطر الفارغة | Blank lines
    blank_lines_after_imports: usize,  // Default: 1
    blank_lines_between_functions: usize,  // Default: 1
    blank_lines_in_function: MaxLines,  // Default: 1

    // العربية | Arabic-specific
    arabic_comma: bool,              // Use ، instead of ,
    arabic_semicolon: bool,          // Use ؛ instead of ;
}

enum BraceStyle {
    SameLine,    // { على نفس السطر
    NextLine,    // { على سطر جديد
}
```

#### 4.5.2: أمثلة التنسيق | Formatting Examples

**قبل التنسيق | Before**:
```tarqeem
دالة حساب(أ:عدد،ب:عدد)->عدد{
متغير نتيجة=أ+ب
إذا(نتيجة>100){أرجع 100}
وإلا{أرجع نتيجة}
}
```

**بعد التنسيق | After**:
```tarqeem
دالة حساب(أ: عدد، ب: عدد) -> عدد {
    متغير نتيجة = أ + ب
    إذا (نتيجة > 100) {
        أرجع 100
    } وإلا {
        أرجع نتيجة
    }
}
```

#### 4.5.3: أوامر المنسق | Formatter Commands

```bash
# تنسيق ملف | Format file
trqfmt ملف.trq

# تنسيق في المكان | Format in place
trqfmt -i ملف.trq
trqfmt --in-place ملف.trq

# تنسيق مجلد | Format directory
trqfmt مصدر/

# فحص التنسيق فقط | Check only
trqfmt --check ملف.trq

# استخدام ملف إعدادات | Use config file
trqfmt --config .trqfmt.toml ملف.trq

# إخراج الفرق | Show diff
trqfmt --diff ملف.trq
```

#### 4.5.4: ملف الإعدادات | Configuration File

```toml
# .trqfmt.toml

# المسافات البادئة
حجم_المسافة = 4
استخدم_تاب = false

# طول السطر
اقصى_طول_سطر = 100

# الأقواس
نمط_الأقواس = "نفس_السطر"

# المسافات
مسافة_بعد_الفاصلة = true
مسافة_حول_العمليات = true

# الفواصل العربية
فاصلة_عربية = false
فاصلة_منقوطة_عربية = false
```

**المخرجات | Deliverables**:
- ✅ منسق كود كامل
- ✅ ملف إعدادات قابل للتخصيص
- ✅ تكامل مع LSP
- ✅ تكامل مع VS Code
- ✅ دعم الفواصل العربية

---

### المعلم 4.6: مصحح الأخطاء (تصحيح)
### Milestone 4.6: Debugger (trqdbg)

**الهدف**: تمكين تصحيح الأخطاء التفاعلي لبرامج ترقيم
**Goal**: Enable interactive debugging for Tarqeem programs

#### 4.6.1: ميزات المصحح | Debugger Features

| الميزة | الوصف |
|--------|-------|
| نقاط التوقف | Breakpoints on lines |
| تنفيذ خطوة | Step over/into/out |
| فحص المتغيرات | Variable inspection |
| تتبع الاستدعاءات | Call stack trace |
| تقييم التعبيرات | Expression evaluation |
| نقاط المراقبة | Watch expressions |
| شروط التوقف | Conditional breakpoints |

#### 4.6.2: بروتوكول DAP | Debug Adapter Protocol

```rust
// Implement VS Code Debug Adapter Protocol

struct DebugAdapter {
    session: DebugSession,
    breakpoints: HashMap<PathBuf, Vec<Breakpoint>>,
    call_stack: Vec<StackFrame>,
    variables: HashMap<VariableId, Variable>,
}

impl DebugAdapter {
    fn handle_request(&mut self, request: Request) -> Response {
        match request.command.as_str() {
            "initialize" => self.initialize(request),
            "launch" => self.launch(request),
            "setBreakpoints" => self.set_breakpoints(request),
            "continue" => self.continue_execution(request),
            "next" => self.step_over(request),
            "stepIn" => self.step_in(request),
            "stepOut" => self.step_out(request),
            "threads" => self.threads(request),
            "stackTrace" => self.stack_trace(request),
            "scopes" => self.scopes(request),
            "variables" => self.variables(request),
            "evaluate" => self.evaluate(request),
            _ => self.unknown_command(request),
        }
    }
}
```

#### 4.6.3: وضع التصحيح في المفسر | Interpreter Debug Mode

```rust
struct DebugInterpreter {
    executor: Executor,
    breakpoints: HashSet<(ModuleId, usize)>,  // (module, line)
    state: DebugState,
}

enum DebugState {
    Running,
    Paused { reason: PauseReason },
    Stepping { mode: StepMode },
    Terminated,
}

enum StepMode {
    Over,   // خطوة فوق
    Into,   // خطوة داخل
    Out,    // خطوة خارج
}

impl DebugInterpreter {
    fn execute_with_debug(&mut self) -> DebugEvent {
        loop {
            // Check for breakpoint
            if self.at_breakpoint() {
                return DebugEvent::Paused { reason: PauseReason::Breakpoint };
            }

            // Check step completion
            if self.step_complete() {
                return DebugEvent::Paused { reason: PauseReason::Step };
            }

            // Execute one instruction
            match self.executor.step() {
                Ok(()) => continue,
                Err(e) => return DebugEvent::Exception(e),
            }
        }
    }
}
```

**المخرجات | Deliverables**:
- ✅ محول DAP كامل
- ✅ نقاط توقف وتنفيذ خطوة
- ✅ فحص المتغيرات
- ✅ تقييم التعبيرات
- ✅ تكامل مع VS Code

---

## هيكل الملفات بعد المرحلة الرابعة | File Structure After Phase 4

```
tarqeem/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   │
│   ├── cli/
│   │   └── commands/
│   │       └── pm/              # Package manager
│   │
│   ├── package/                 # Package management core
│   │   ├── mod.rs
│   │   ├── manifest.rs
│   │   ├── lockfile.rs
│   │   ├── resolver.rs
│   │   ├── registry.rs
│   │   └── cache.rs
│   │
│   ├── lsp/                     # Language Server
│   │   ├── mod.rs
│   │   ├── server.rs
│   │   ├── handlers/
│   │   └── analysis/
│   │
│   ├── doc/                     # Documentation generator
│   │   ├── mod.rs
│   │   ├── parser.rs
│   │   └── generator/
│   │
│   ├── fmt/                     # Code formatter
│   │   ├── mod.rs
│   │   └── rules.rs
│   │
│   └── debug/                   # Debugger
│       ├── mod.rs
│       └── adapter.rs
│
├── vscode-tarqeem/              # VS Code Extension
│   ├── package.json
│   ├── src/
│   ├── syntaxes/
│   └── snippets/
│
└── registry/                    # Package Registry (separate repo)
    ├── api/
    └── web/
```

---

## الاعتماديات المطلوبة | Dependencies to Add

```toml
[dependencies]
# LSP
tower-lsp = "0.20"
async-trait = "0.1"

# Package management
semver = "1.0"
toml = "0.8"
reqwest = { version = "0.11", features = ["json"] }
sha2 = "0.10"
flate2 = "1.0"
tar = "0.4"

# Async
tokio = { version = "1", features = ["full"] }
futures = "0.3"

# Documentation
pulldown-cmark = "0.9"
syntect = "5.0"
tera = "1.19"  # Template engine

# Formatting
pretty = "0.12"  # Pretty printing

# Debug
dap = "0.5"  # Debug Adapter Protocol
```

---

## خطة التنفيذ | Implementation Plan

### الترتيب الموصى | Recommended Order

```
المرحلة 4.1: مدير الحزم (أسابيع 1-3)
├── هيكل الحزمة (حزمة.toml)
├── أوامر init، add، remove
├── حل الاعتماديات
├── ملف القفل
└── التخزين المؤقت

المرحلة 4.2: خادم LSP (أسابيع 4-6)
├── البنية الأساسية
├── التشخيصات
├── الإكمال التلقائي
├── الانتقال للتعريف
├── المعلومات عند التحويم
└── التحليل التزايدي

المرحلة 4.3: إضافة VS Code (أسابيع 7-8)
├── تلوين الكود
├── تكامل LSP
├── مقاطع الكود
├── دعم RTL
└── النشر

المرحلة 4.4: مولد التوثيق (أسابيع 9-10)
├── محلل التعليقات
├── نموذج التوثيق
├── توليد HTML
├── توليد Markdown
└── السمات

المرحلة 4.5: منسق الكود (أسبوع 11)
├── قواعد التنسيق
├── ملف الإعدادات
└── تكامل LSP

المرحلة 4.6: المصحح (أسابيع 12-14)
├── محول DAP
├── نقاط التوقف
├── تنفيذ خطوة
├── فحص المتغيرات
└── تكامل VS Code
```

---

## معايير النجاح | Success Criteria

### المرحلة الرابعة مكتملة عندما:

1. **مدير الحزم يعمل**
   - `trqpm init` ينشئ مشروع جديد
   - `trqpm add` يضيف اعتماديات
   - `trqpm install` يثبت الاعتماديات
   - `trqpm build` يبني المشروع

2. **خادم LSP يعمل**
   - التشخيصات الفورية تظهر
   - الإكمال التلقائي يعمل
   - الانتقال للتعريف يعمل
   - المعلومات عند التحويم تظهر

3. **إضافة VS Code تعمل**
   - تلوين الكود صحيح
   - تكامل LSP كامل
   - مقاطع الكود تعمل
   - منشورة على Marketplace

4. **مولد التوثيق يعمل**
   - `trqdoc` يولد HTML
   - دعم RTL كامل
   - السمات تعمل

5. **المنسق يعمل**
   - `trqfmt` ينسق الكود
   - ملف الإعدادات يعمل

6. **المصحح يعمل**
   - نقاط التوقف تعمل
   - تنفيذ خطوة يعمل
   - فحص المتغيرات يعمل

---

## المخاطر والتخفيف | Risks and Mitigation

### التحديات المحتملة:

1. **تعقيد LSP**
   - التخفيف: البدء بالميزات الأساسية (diagnostics، completion)
   - استخدام tower-lsp للتبسيط

2. **أداء التحليل التزايدي**
   - التخفيف: تخزين مؤقت ذكي
   - تحليل جزئي للملفات الكبيرة

3. **توافق RTL في VS Code**
   - التخفيف: اختبار مكثف
   - استخدام ميزات VS Code الموجودة

4. **أمان السجل**
   - التخفيف: توقيع الحزم
   - التحقق من الشهادات

5. **تعقيد DAP**
   - التخفيف: البدء بالمفسر فقط
   - إضافة دعم LLVM لاحقاً

---

## ملاحظات | Notes

- جميع واجهات المستخدم يجب أن تكون ثنائية اللغة (عربي/إنجليزي)
- دعم RTL في جميع الأدوات
- التوثيق بالعربية أولاً
- اختبارات شاملة لكل ميزة
- الحفاظ على التوافق مع المراحل السابقة

---

## المراجع | References

- [ARCHITECTURE.md](/home/user/tarqeem/ARCHITECTURE.md) - التفاصيل التقنية
- [PHASE2_PLAN.md](/home/user/tarqeem/docs/PHASE2_PLAN.md) - خطة المرحلة الثانية
- [PHASE3_PLAN.md](/home/user/tarqeem/docs/PHASE3_PLAN.md) - خطة المرحلة الثالثة
- [LSP Specification](https://microsoft.github.io/language-server-protocol/) - مواصفات LSP
- [DAP Specification](https://microsoft.github.io/debug-adapter-protocol/) - مواصفات DAP
- [VS Code Extension API](https://code.visualstudio.com/api) - واجهة إضافات VS Code
