# CLAUDE.md - Development Guidelines for Tarqeem

This document provides context and guidelines for Claude (AI assistant) when working on the Tarqeem project.

## Project Overview

Tarqeem (ترقيم) is an Arabic programming language compiler written in Rust. The goal is to create a fully-featured, compiled, general-purpose programming language with native Arabic syntax support.

## Key Design Principles

### 1. Arabic-First, But Bilingual

- All keywords have Arabic primary forms with English aliases
- Error messages must be available in both Arabic and English
- Comments and documentation support both languages
- RTL text handling is a first-class concern

### 2. Best of Three Worlds

The syntax takes inspiration from:
- **Python**: Clean, readable syntax; indentation-aware formatting
- **PHP**: Practical, web-friendly standard library
- **JavaScript**: Modern async/await, arrow functions, destructuring

### 3. Compilation Target

- Primary target: Native machine code via LLVM
- Secondary targets: WebAssembly, JavaScript (transpilation)
- Development mode: Fast interpretation for rapid iteration

## Code Standards

### Rust Code Style

```rust
// Use descriptive names, even if long
fn parse_variable_declaration(&mut self) -> Result<Stmt, ParseError> {
    // ...
}

// Document complex logic
/// Parses a class declaration including inheritance and interface implementation.
///
/// Grammar:
/// صنف <name> [يرث <parent>] [يطبق <interfaces>] { <body> }
fn parse_class_declaration(&mut self) -> Result<Stmt, ParseError> {
    // ...
}

// Use type aliases for clarity
type ParseResult<T> = Result<T, ParseError>;
type TypeCheckResult<T> = Result<T, TypeError>;
```

### Error Messages

Always provide both Arabic and English error messages:

```rust
Diagnostic {
    message: "Cannot assign to immutable variable",
    message_ar: "لا يمكن تعيين قيمة لمتغير ثابت",
    // ...
}
```

### Testing

- Write tests for all new features
- Include Arabic source code in tests
- Test edge cases with mixed Arabic/English code

```rust
#[test]
fn test_mixed_language_identifiers() {
    let source = r#"
        متغير userName = "أحمد"
        متغير userAge = 25
        اطبع(userName + " عمره " + userAge)
    "#;
    // ...
}
```

## File Naming Conventions

- Rust source files: `snake_case.rs`
- Tarqeem source files: Arabic names with `.trq` extension
- Test files: `*_tests.rs` or `test_*.rs`
- Documentation: `*.md` in English

## Git Workflow - Gitflow Strategy

This project uses **Gitflow** as the branching strategy. Claude must follow these rules strictly.

### Main Branches

| Branch | Purpose | Protected |
|--------|---------|-----------|
| `main` | Production-ready code. Only receives merges from `release/*` or `hotfix/*` branches. | Yes |
| `develop` | Integration branch for features. All feature branches merge here. | Yes |

### Supporting Branches

| Branch Type | Naming Convention | Created From | Merges Into |
|-------------|-------------------|--------------|-------------|
| Feature | `feature/<description>` | `develop` | `develop` |
| Release | `release/<version>` | `develop` | `main` and `develop` |
| Hotfix | `hotfix/<description>` | `main` | `main` and `develop` |
| Bugfix | `bugfix/<description>` | `develop` | `develop` |

### Branch Naming Examples

- Features: `feature/lexer-arabic-support` or `feature/دعم-العربية`
- Releases: `release/v0.1.0`
- Hotfixes: `hotfix/critical-parser-bug`
- Bugfixes: `bugfix/unicode-normalization`

### Claude-Specific Git Rules

**IMPORTANT**: When Claude works on this repository, it MUST follow these rules:

1. **Never push directly to `main` or `develop`**
   - Always create a feature/bugfix branch first
   - Submit changes via Pull Request

2. **Branch Creation Workflow**:
   ```bash
   # For new features
   git checkout develop
   git pull origin develop
   git checkout -b feature/my-feature

   # For bugfixes
   git checkout develop
   git pull origin develop
   git checkout -b bugfix/my-fix

   # For hotfixes (critical production bugs only)
   git checkout main
   git pull origin main
   git checkout -b hotfix/critical-fix
   ```

3. **Before Starting Work**:
   - Always `git fetch` to get the latest state
   - Ensure you're branching from the correct base (`develop` for features, `main` for hotfixes)

4. **Pull Request Requirements**:
   - Feature branches → PR to `develop`
   - Release branches → PR to `main` (and back-merge to `develop`)
   - Hotfix branches → PR to `main` (and back-merge to `develop`)

5. **Merge Strategy**:
   - Use squash merges for features to keep history clean
   - Use regular merges for releases and hotfixes to preserve history

6. **Version Tagging**:
   - Tags are created only on `main` after release merges
   - Format: `v<major>.<minor>.<patch>` (e.g., `v0.1.0`)

### Commit Messages

Format: `<type>(<scope>): <description>`

Types:
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code refactoring
- `docs`: Documentation
- `test`: Tests
- `chore`: Maintenance
- `perf`: Performance improvement
- `ci`: CI/CD changes

Examples:
```
feat(lexer): add Arabic keyword tokenization
fix(parser): handle RTL text in string literals
docs(readme): add syntax examples
refactor(ast): simplify node structure
```

### Gitflow Visual Summary

```
main:     ─────●─────────────────●─────────────●───→
               ↑                 ↑             ↑
release:       │    ─────●───────┘             │
               │         ↑                     │
hotfix:        │         │              ●──────┘
               │         │              ↑
develop: ──────┴─●───●───●───●───●──────┴───●───→
                 ↑   ↑       ↑
feature:    ─────┘   │   ────┘
                     │
bugfix:         ─────┘
```

## Common Tasks

### Adding a New Keyword

1. Add token to `src/lexer/token.rs`
2. Add Arabic mapping in `src/lexer/keywords.rs`
3. Add parsing logic in `src/parser/parser.rs`
4. Add AST node if needed in `src/parser/ast.rs`
5. Add semantic analysis in `src/semantic/`
6. Add code generation in `src/codegen/`
7. Add tests for each stage
8. Update documentation

### Adding a Standard Library Function

1. Implement in Rust in `src/stdlib/`
2. Create Tarqeem wrapper in `stdlib_trq/`
3. Add type definitions
4. Add documentation
5. Add tests

### Debugging the Compiler

```bash
# Verbose output
RUST_LOG=debug cargo run -- compile test.trq

# Dump tokens
cargo run -- compile test.trq --dump-tokens

# Dump AST
cargo run -- compile test.trq --dump-ast

# Dump IR
cargo run -- compile test.trq --dump-ir

# Dump LLVM IR
cargo run -- compile test.trq --emit-llvm
```

## Architecture Decisions

### Why Recursive Descent Parser?

- Easier to implement and understand
- Better error messages and recovery
- Sufficient for our grammar (not highly ambiguous)
- Can be extended to Pratt parsing for expressions

### Why LLVM?

- Mature, battle-tested backend
- Excellent optimization passes
- Cross-platform code generation
- Good Rust bindings via `inkwell`

### Why Reference Counting?

- Simpler than tracing GC
- Deterministic destruction
- Low latency (no GC pauses)
- Can be optimized by the compiler

## Known Challenges

### 1. RTL Text Rendering

- IDE/editor support varies
- Mixed Arabic/English in identifiers
- Solution: LSP server with proper bidirectional text handling

### 2. Unicode Identifiers

- Arabic letters have multiple forms (initial, medial, final)
- Normalization is important
- Solution: NFC normalization before comparison

### 3. Operator Characters

- Arabic has different quotation marks (« »)
- Comma is different (،)
- Solution: Accept both Arabic and ASCII punctuation

## Development Phases

### Phase 1: Core Language (Current)
- [x] Language specification
- [ ] Lexer with Arabic support
- [ ] Parser for basic syntax
- [ ] Type checker
- [ ] Simple code generator

### Phase 2: Full Language
- [ ] Classes and interfaces
- [ ] Generics
- [ ] Error handling
- [ ] Modules and imports

### Phase 3: Standard Library
- [ ] Core types
- [ ] Collections
- [ ] I/O
- [ ] Networking

### Phase 4: Tooling
- [ ] Package manager
- [ ] LSP server
- [ ] Debugger
- [ ] Formatter

## Useful Commands

```bash
# Build the compiler
cargo build --release

# Run tests
cargo test

# Run a specific test
cargo test lexer

# Check without building
cargo check

# Format code
cargo fmt

# Lint
cargo clippy

# Generate documentation
cargo doc --open

# Run the REPL
cargo run -- repl

# Compile a Tarqeem file
cargo run -- compile examples/مرحبا.trq

# Run a Tarqeem file
cargo run -- run examples/مرحبا.trq
```

## Resources

### Compiler Design
- "Crafting Interpreters" by Robert Nystrom
- "Engineering a Compiler" by Cooper & Torczon
- LLVM Language Reference Manual

### Rust
- The Rust Programming Language Book
- Rust API Guidelines
- `inkwell` documentation

### Unicode & Arabic
- Unicode Standard Annex #9 (Bidirectional Algorithm)
- Arabic Unicode Chart
- ICU documentation

## Contact

For questions about the project, refer to:
- README.md for user documentation
- ARCHITECTURE.md for technical details
- GitHub Issues for bugs and features

---

Remember: The goal is to make programming accessible to Arabic speakers while maintaining professional-grade compiler quality.
