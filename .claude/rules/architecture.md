# Architecture Constraints

This file defines the architectural boundaries that MUST be respected. Violations will cause bugs.

## Compiler Pipeline Layering

The Tarqeem compiler follows a strict pipeline:

```
Source (.ترقيم) → Lexer → Parser → Semantic → IR → Codegen → Binary
```

### Layer Dependencies (ENFORCED)

| Layer | Can Import | Cannot Import |
|-------|-----------|---------------|
| `lexer` | `error` | parser, semantic, ir, codegen |
| `parser` | `lexer`, `error` | semantic, ir, codegen |
| `semantic` | `parser`, `lexer`, `error` | ir, codegen |
| `ir` | `semantic`, `parser`, `error` | codegen |
| `codegen` | `ir`, `semantic`, `error` | - |
| `cli` | ALL | - |
| `error` | NONE | all others |

**Rule: Each layer can only depend on layers that come BEFORE it in the pipeline.**

### Violation Examples (DO NOT DO)

```rust
// BAD: Lexer importing parser types
use crate::parser::ast::Expr;  // WRONG!

// BAD: Semantic importing codegen
use crate::codegen::llvm::Context;  // WRONG!

// BAD: IR depending on codegen decisions
use crate::codegen::target::TargetTriple;  // WRONG!
```

### Correct Examples

```rust
// GOOD: Parser using lexer types
use crate::lexer::token::{Token, TokenKind};

// GOOD: Semantic using parser types
use crate::parser::ast::{Expr, Stmt};

// GOOD: Codegen using IR
use crate::ir::instruction::IRInstruction;
```

## Module Ownership

Each concern has ONE owner:

| Concern | Owner Module | Files |
|---------|-------------|-------|
| Tokenization | `lexer` | `token.rs`, `lexer.rs`, `keywords.rs` |
| AST Definition | `parser` | `ast.rs` |
| Syntax Parsing | `parser` | `parser.rs`, `precedence.rs` |
| Type System | `semantic` | `types.rs` |
| Scope/Symbols | `semantic` | `scope.rs` |
| Type Checking | `semantic` | `analyzer.rs` |
| Generics | `semantic` | `generics.rs` |
| Class Resolution | `semantic` | `class_resolver.rs`, `method_resolver.rs` |
| IR Instructions | `ir` | `instruction.rs` |
| IR Building | `ir` | `builder.rs` |
| Optimizations | `ir/opt` | `const_fold.rs`, `dce.rs`, `cse.rs`, `inline.rs` |
| LLVM Codegen | `codegen/llvm` | `codegen.rs`, `types.rs` |
| Linking | `codegen` | `linker.rs` |
| Error Format | `error` | `diagnostic.rs`, `span.rs` |

**Rule: Do not add functionality to the wrong module. If unsure, ask.**

## Critical Invariants (DO NOT BREAK)

### 1. Token Span Accuracy

Every token MUST have accurate source location (`Span`). This is required for error reporting.

```rust
// Tokens must always have valid spans
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,  // REQUIRED, never None
    pub lexeme: String,
}
```

### 2. AST Immutability

The AST is immutable after parsing. Semantic analysis annotates via separate structures, not mutation.

```rust
// GOOD: Separate type map
struct TypedAST {
    ast: Program,
    types: HashMap<NodeId, Type>,
}

// BAD: Mutating AST nodes
node.resolved_type = Some(ty);  // WRONG
```

### 3. Error Recovery

The compiler MUST continue after errors when possible. Never `panic!()` or `unwrap()` on user input.

```rust
// GOOD: Return error, continue parsing
if !self.expect(TokenKind::Semicolon) {
    self.report_error("Expected ';'");
    self.synchronize();  // Skip to next statement
    continue;
}

// BAD: Crash on bad input
self.expect(TokenKind::Semicolon).unwrap();  // WRONG
```

### 4. Bilingual Messages

ALL user-facing messages need both Arabic and English versions.

```rust
// REQUIRED format
Diagnostic {
    message: "لا يمكن إيجاد المتغير 'x'",
    // ...
}
```

### 5. Unicode Normalization

All identifiers MUST be NFC-normalized before comparison.

```rust
// GOOD: Normalize before comparing
use unicode_normalization::UnicodeNormalization;
let normalized = identifier.nfc().collect::<String>();

// BAD: Direct comparison
if name1 == name2  // May fail for Arabic!
```

## When Changing Architecture

If a change requires violating these constraints:

1. **STOP** - Do not proceed
2. **Document** - Write up the proposed change
3. **Discuss** - Get approval before proceeding
4. **Update** - Modify this file if the constraint changes
