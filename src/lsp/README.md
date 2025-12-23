# Tarqeem Language Server Protocol (LSP) Implementation

<div dir="rtl" align="right">

## خادم بروتوكول اللغة لترقيم

خادم LSP يوفر دعم IDE كامل للغة ترقيم، مما يتيح:
- عرض الأخطاء والتحذيرات في الوقت الفعلي
- الإكمال التلقائي الذكي
- الانتقال إلى التعريف
- معلومات عند التحويم
- البحث عن المراجع
- إعادة تسمية الرموز
- مخطط المستند

</div>

---

## Overview

The Tarqeem LSP server provides full IDE support through the Language Server Protocol. It enables modern editor features like real-time diagnostics, auto-completion, go-to-definition, and more.

## Features

### P0 - Must Have (Implemented)

| Feature | Description | Arabic |
|---------|-------------|--------|
| **Diagnostics** | Real-time error and warning display | أخطاء وتحذيرات |
| **Auto-completion** | Smart code completion suggestions | إكمال تلقائي |
| **Go to Definition** | Navigate to symbol definitions | انتقال للتعريف |
| **Hover Information** | Type info and documentation on hover | معلومات عند التحويم |

### P1 - Should Have (Implemented)

| Feature | Description | Arabic |
|---------|-------------|--------|
| **Find References** | Locate all usages of a symbol | البحث عن المراجع |
| **Rename Symbol** | Project-wide symbol renaming | إعادة التسمية |
| **Document Symbols** | Outline view of document structure | مخطط المستند |
| **Formatting** | Basic code formatting | تنسيق الكود |

### P2 - Nice to Have (Implemented)

| Feature | Description | Arabic |
|---------|-------------|--------|
| **Code Actions** | Quick fixes and refactorings | إجراءات سريعة |
| **Inlay Hints** | Inline type annotations | تلميحات مضمنة |
| **Semantic Tokens** | Enhanced syntax highlighting | رموز دلالية |
| **Code Folding** | Collapsible code regions | طي الكود |

## Usage

### Starting the Server

```bash
# Start the LSP server (communicates via stdin/stdout)
tarqeem lsp

# Arabic alias
tarqeem خادم

# With verbose output
tarqeem lsp -v
```

### Editor Integration

#### VS Code

Create or edit `.vscode/settings.json`:

```json
{
  "tarqeem.server.path": "/path/to/tarqeem",
  "tarqeem.server.args": ["lsp"]
}
```

#### Neovim (with nvim-lspconfig)

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

configs.tarqeem = {
  default_config = {
    cmd = { 'tarqeem', 'lsp' },
    filetypes = { 'tarqeem', 'trq' },
    root_dir = lspconfig.util.find_git_ancestor,
    settings = {},
  },
}

lspconfig.tarqeem.setup{}
```

#### Emacs (with lsp-mode)

```elisp
(require 'lsp-mode)

(add-to-list 'lsp-language-id-configuration '(tarqeem-mode . "tarqeem"))

(lsp-register-client
 (make-lsp-client
  :new-connection (lsp-stdio-connection '("tarqeem" "lsp"))
  :major-modes '(tarqeem-mode)
  :server-id 'tarqeem-ls))
```

## Architecture

```
src/lsp/
├── mod.rs              # Module entry point, server runner
├── server.rs           # Main LSP server (LanguageServer trait impl)
├── state.rs            # Server state management
├── capabilities.rs     # LSP capabilities configuration
├── analysis/
│   ├── mod.rs          # Analysis module
│   └── document.rs     # Document state and incremental analysis
├── handlers/
│   ├── mod.rs          # Handler exports
│   ├── code_actions.rs # Quick fixes and refactorings
│   ├── completion.rs   # Auto-completion
│   ├── definition.rs   # Go to definition
│   ├── diagnostics.rs  # Error/warning publishing
│   ├── document_symbol.rs # Document outline
│   ├── folding.rs      # Code folding ranges
│   ├── formatting.rs   # Code formatting
│   ├── hover.rs        # Hover information
│   ├── inlay_hints.rs  # Inline type/parameter hints
│   ├── references.rs   # Find references
│   ├── rename.rs       # Symbol renaming
│   └── semantic_tokens.rs # Semantic syntax highlighting
└── utils/
    ├── mod.rs          # Utility exports
    └── position.rs     # Position/offset conversion
```

## Protocol Details

### Document Synchronization

The server uses **full document sync** mode, meaning the entire document content is sent on each change. This simplifies implementation while providing reliable analysis.

### Capabilities

```json
{
  "textDocumentSync": "full",
  "hoverProvider": true,
  "completionProvider": {
    "triggerCharacters": [".", ":", "\"", "/"],
    "resolveProvider": true
  },
  "definitionProvider": true,
  "referencesProvider": true,
  "renameProvider": {
    "prepareProvider": true
  },
  "documentSymbolProvider": true,
  "documentFormattingProvider": true,
  "codeActionProvider": true,
  "foldingRangeProvider": true,
  "inlayHintProvider": true,
  "semanticTokensProvider": {
    "full": true,
    "legend": {
      "tokenTypes": ["namespace", "type", "class", ...],
      "tokenModifiers": ["declaration", "readonly", ...]
    }
  }
}
```

## Completion Types

The server provides completions for:

### Keywords (Arabic/English)

```tarqeem
// Arabic keywords
متغير، ثابت، دالة، صنف، واجهة، إذا، وإلا، طالما، لكل، أرجع

// English aliases
let, const, function, class, interface, if, else, while, for, return
```

### Types

```tarqeem
// Arabic types (فراغ/void eliminated - functions without return type default to void)
عدد، عدد_عشري، نص، منطقي، مصفوفة، قاموس

// English types
int, float, string, bool, array, map
```

### Built-in Functions

```tarqeem
// I/O
اطبع (print), ادخل (input)

// Introspection
طول (len), نوع (type)

// Type conversion
عدد (int), نص (str), منطقي (bool)

// Math
جذر (sqrt), مطلق (abs), قوة (pow)

// File I/O
اقرأ_ملف (read_file), اكتب_ملف (write_file)
```

## Diagnostics

The server reports diagnostics for:

- **Lexer errors**: Invalid tokens, unterminated strings
- **Parser errors**: Syntax errors, unexpected tokens
- **Semantic errors**: Type mismatches, undefined variables, scope issues

Diagnostics are available in both Arabic and English based on the client's locale setting.

## Position Handling

The server properly handles:

- **UTF-16 positions**: LSP uses UTF-16 code units
- **Arabic text**: Full RTL and bidirectional text support
- **Mixed content**: Arabic and English in the same file

## Dependencies

```toml
tower-lsp = "0.20"      # LSP protocol implementation
tokio = "1"             # Async runtime
dashmap = "6.0"         # Concurrent hash map
async-trait = "0.1"     # Async traits
futures = "0.3"         # Futures utilities
```

## Testing

Run LSP tests:

```bash
# Run all LSP tests
cargo test lsp

# Run specific handler tests
cargo test lsp::handlers::completion
cargo test lsp::handlers::hover
cargo test lsp::handlers::code_actions
cargo test lsp::handlers::folding
cargo test lsp::handlers::inlay_hints
cargo test lsp::handlers::semantic_tokens
cargo test lsp::utils::position
```

## Extending

### Adding a New Handler

1. Create a new file in `src/lsp/handlers/`:

```rust
// src/lsp/handlers/my_feature.rs
use crate::error::Language;
use crate::lsp::analysis::DocumentState;

pub fn handle_my_feature(
    doc: &mut DocumentState,
    language: Language,
) -> Option<MyResult> {
    // Implementation
}
```

2. Export from `src/lsp/handlers/mod.rs`:

```rust
mod my_feature;
pub use my_feature::handle_my_feature;
```

3. Add the handler call in `src/lsp/server.rs`:

```rust
async fn my_feature(&self, params: MyParams) -> Result<Option<MyResult>> {
    // Call handler
}
```

4. Update capabilities in `src/lsp/capabilities.rs`

### Adding New Completions

Edit `src/lsp/handlers/completion.rs` and add items to the appropriate function:
- `get_keyword_completions()` for keywords
- `get_builtin_completions()` for built-in functions
- `get_type_completions()` for types

## Troubleshooting

### Server Not Starting

```bash
# Check if the server starts correctly
tarqeem lsp -v 2>&1 | head -5
```

### Editor Not Connecting

1. Verify the server path is correct
2. Check that `tarqeem` is in PATH
3. Look for errors in the editor's LSP log

### Diagnostics Not Appearing

1. Ensure the file has a `.trq` or `.ترقيم` extension
2. Check that document sync is working (look for `didOpen` events)
3. Verify the analyzer is running without panics

## Contributing

When contributing to the LSP server:

1. Follow the existing code patterns
2. Maintain bilingual support (Arabic/English)
3. Add tests for new features
4. Update this README for new capabilities

## License

Same as the main Tarqeem project - MIT License.
