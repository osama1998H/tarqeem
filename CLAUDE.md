# CLAUDE.md - Development Guidelines for Tarqeem

This document provides context and guidelines for Claude (AI assistant) when working on the Tarqeem project.


# important notes hard line dont cross

- dont leave any claude marks in github issues / PR like this "🤖 Generated with Claude Code" or mention the model name i paid for claude subscription so its my work

## Imports

See @ARCHITECTURE.md for detailed technical architecture.
See @README.md for user documentation and syntax examples.
See @LANGUAGE_SPEC.md for the complete language specification.

---

# The Linguistic Philosophy of Tarqeem

This document outlines the intellectual and linguistic foundation of Tarqeem. It serves as the compass for language design, ensuring that every syntactic decision respects the structure, logic, and eloquence of the Arabic language.

---

## I. The Core Axiom: Authenticity over Translation

**Tarqeem is not an English programming language wearing an Arabic mask; it is an authentic Arabic computational system.**

The ultimate goal is **Cognitive Immediacy**: An Arabic speaker should be able to read, understand, and reason about the code directly in their native tongue, without performing a mental translation step into English. The code is not "translated"; it is "conceptualized" in Arabic.

---

## II. The Four Pillars of Design

### 1. Functional Description vs. Literal Translation

In Tarqeem, we do not translate words; we translate **meanings**.

* **The Principle:** If a programming term relies on an English metaphor that doesn't exist in Arabic, we discard the metaphor and describe the *function*.
* **The Logic:** Literal translation (e.g., translating "Interface" as "Face") creates confusion. Instead, we look at what the construct *does*. If it enforces a set of behaviors, it is a "Contract" or "Pact," not a "Face." If a construct refers to a resource, it is an "Identifier," not a "Handle."
* **Outcome:** The keyword must describe the runtime behavior or the architectural role of the element in the Arabic context.

### 2. Syntactic Integrity (Code as Literature)

Code in Tarqeem must adhere to the rules of Arabic Grammar (Nahw).

* **The Principle:** A line of code should be readable as a grammatically correct Arabic sentence.
* **The Logic:** Arabic has specific rules for descriptor placement (Adjectives follow Nouns). Therefore, type modifiers and attributes must follow the noun they modify, not precede it.
* **Outcome:** Reading the code aloud should sound natural to the ear, avoiding "broken" phrasing that mimics English word order.

### 3. Cognitive Alignment

The syntax must follow the logical flow of Arabic thought.

* **The Principle:** Syntactic structures that are redundant or alien to Arabic rhetoric are removed or reordered.
* **The Logic:** If a function returns nothing, Arabic logic dictates silence (absence of a return type) rather than explicitly stating "Void" or "Null." Absence implies non-existence; stating "Empty" is a redundancy.
* **Outcome:** A cleaner, more intuitive syntax that respects the economy of the language.

### 4. Epistemological Clarity (Self-Completeness)

Tarqeem rejects ambiguity in favor of explicit meaning.

* **The Principle:** No obscure abbreviations. Scientific and mathematical terms must use their full, historical Arabic names.
* **The Logic:** Abbreviations (like `sin`, `tan`) are barriers to entry. Arabic has a rich history in mathematics (e.g., Al-Khwarizmi, Al-Battani). We honor this by using the full, descriptive terms for mathematical functions rather than transliterating Western abbreviations.
* **Outcome:** Code that is self-documenting and pedagogically sound.

---

## III. Semantic Standards for the Standard Library

To ensure consistency across the ecosystem, naming follows a strict semantic structure based on parts of speech:

1. **Imperative Verbs for Actions:** Functions that perform a task or change a state must be named using the imperative form (Command).
2. **Nouns for Values:** Functions that purely return a value (without side effects) must use the noun describing that value.
3. **Interrogatives for States:** Boolean checks must be phrased as questions, mimicking natural human inquiry.
4. **Scientific Rooting:** Mathematical constants and functions return to their Arabic origins, using the terminology established during the Golden Age of Islamic Science, rather than modern transliterations.

---

## IV. The Verification Criterion: "The Native Reader Test"

Before any keyword or syntax rule is added to Tarqeem, it must pass the **Native Reader Test**. We ask four fundamental questions:

1. **Intuitiveness:** Can an Arabic speaker understand the concept without knowing the English equivalent?
2. **Fluency:** Does it read naturally in a sentence, or does it sound like a "translated text"?
3. **Accuracy:** Does the word describe what the code *actually does*, or is it just a literal translation of the English label?
4. **Grammar:** Is the syntactic order (Noun-Adjective, Verb-Subject) grammatically sound?

**Conclusion:** Tarqeem strives to prove that Arabic is not just a language of poetry and religion, but a fully capable, logical, and elegant medium for modern computation and software engineering.

## Project Map (READ FIRST)

**Architecture**: Compiled Arabic programming language with LLVM backend
**Language**: Rust (~40,000 lines)
**Version**: v1.0.0 (V1 Release Complete, in v1.1-v1.5 hardening phase)
**Core rule**: Preserve existing patterns; do not invent new abstractions if one already exists.

### Directory Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library root
├── lexer/               # Tokenization (Arabic keywords, Unicode)
├── parser/              # Recursive descent + Pratt parsing
├── semantic/            # Type checking, scope, generics, modules
├── ir/                  # Three-address code, SSA, optimizations
├── codegen/             # LLVM code generation
├── interpreter/         # Tree-walking interpreter for debugging
├── cli/                 # Commands (compile, run, repl, fmt, debug, doc, pkg)
├── lsp/                 # Language Server Protocol (20+ handlers)
├── debug/               # Debug Adapter Protocol (DAP) server
├── fmt/                 # Code formatter
├── doc/                 # Documentation generator
├── package/             # Package manager (trqpm)
├── error/               # Bilingual diagnostics
└── utils/               # String interning, extensions

runtime-rs/              # Rust runtime library (memory, string, array, I/O, crypto, network)
stdlib/              # Standard library (Tarqeem source)
tests/                   # Integration tests
benches/                 # Criterion benchmarks
examples/                # 10 example programs (one per language area)
docs/                    # Project documentation
```

### Compiler Pipeline (Layer Ordering)

```plain
Source → Lexer → Parser → Semantic → IR → Codegen → Binary
                    ↓
              Interpreter (for debugging/REPL)
```

**CRITICAL**: Each layer can ONLY depend on layers BEFORE it. See `.claude/rules/architecture.md`.

---

## Agent Operating Procedure (MANDATORY)

**The agent MUST follow this workflow for ALL code changes.** Skipping steps causes bugs.

### Workflow: Explore → Plan → Implement → Verify

1. **EXPLORE (Read-Only)**: Identify modules, find patterns, list files. **NO CODE**.
2. **PLAN**: Steps, files, impact, tests, risks. Get approval for significant changes.
3. **IMPLEMENT**: Minimal diff, reuse patterns, bilingual messages.
4. **VERIFY**: Run `cargo fmt && cargo clippy && cargo test`.
5. **DOCUMENT**: Update `docs/AI_NOTES.md` with decisions.

**Rule: Never write code until you understand the system.**

See `.claude/rules/00-operating-procedure.md` for complete workflow.

---

## Critical Invariants (DO NOT BREAK)

| Invariant | Rule |
|-----------|------|
| Layer boundaries | Lexer→Parser→Semantic→IR→Codegen (no reverse deps) |
| Bilingual messages | ALL user-facing strings need Arabic + English |
| NFC normalization | Arabic identifiers MUST be normalized before comparison |
| Error recovery | Never `panic!()` or `unwrap()` on user input |
| Token spans | Every token must have accurate source location |
| Arabic Philosophy | Use descriptive Arabic terms, not English transliterations |

---

## Modular Rules (.claude/rules/)

| File | Purpose |
|------|---------|
| `00-operating-procedure.md` | Mandatory workflow (MUST READ) |
| `architecture.md` | Layer boundaries and invariants |
| `testing.md` | Testing requirements |
| `rust-style.md` | Rust coding standards |
| `arabic-philosophy.md` | Arabic language philosophy, Unicode handling, and keyword design |
| `comments.md` | Comment budget; when to file a `code-quality` issue instead of fixing inline |
| `bug-tracking.md` | Issue creation and the Beta Roadmap board planning gate |
| `error-codes.md` | Arabic error code system (ق، ب، د، ن، ص، و، ت، ح، م) |
| `diagrams.md` | When a mermaid diagram is worth drawing, and how small |

---

## Skills (.claude/skills/)

| Skill | Purpose |
|-------|---------|
| `mermaid` | Mermaid syntax reference per diagram type (vendored, see `VENDORED.md`) |

---

## Standard Commands

```bash
# Build
cargo build --release

# Test (921+ tests)
cargo test

# Lint (must be warning-free)
cargo clippy

# Format
cargo fmt

# Benchmarks
cargo bench

# CLI commands
cargo run -- compile file.ترقيم    # Compile to binary
cargo run -- run file.ترقيم        # Compile and execute
cargo run -- check file.ترقيم      # Type check only
cargo run -- repl                # Interactive REPL
cargo run -- fmt file.ترقيم        # Format code
cargo run -- debug file.ترقيم      # Debug with DAP
cargo run -- doc file.ترقيم        # Generate documentation
cargo run -- pkg init            # Initialize package
```

---

## Project Overview

Tarqeem (ترقيم) is the **first compiled Arabic programming language** for general-purpose programming. It is written in Rust and compiles to native machine code via LLVM.

## Current Status

| Component | Status |
|-----------|--------|
| Lexer | ✅ Complete |
| Parser | ✅ Complete |
| Semantic Analyzer | ✅ Complete |
| IR Generation | ✅ Complete |
| LLVM Codegen | ✅ Complete |
| Interpreter | ✅ Complete |
| Standard Library | ✅ Complete (8 modules) |
| LSP Server | ✅ Complete |
| DAP Server | ✅ Complete |
| Formatter | ✅ Complete |
| Package Manager | ✅ Complete |
| Doc Generator | ✅ Complete |

**Current Focus**: v1.1-v1.5 quality hardening (see `docs/ROADMAP_V1.1-V1.5.md`)

## Key Design Principles

### 1. Arabic-First Language

- All keywords and identifiers must be in Arabic (English is NOT supported)
- Error messages are available in both Arabic and English for accessibility
- Arabic philosophy: **descriptive terms, not English transliterations**
  - ✅ `ميثاق` (covenant/contract) instead of ~~`واجهة`~~ (literal translation of "interface")
  - ✅ `مشترك` (shared) instead of ~~`ثابت_صنف`~~ (literal translation of "static")
  - ✅ Functions without return type simply omit `-> نوع` (no `فراغ` keyword needed)

### 2. Best of Three Worlds

The syntax takes inspiration from:
- **Python**: Clean, readable syntax; indentation-aware formatting
- **PHP**: Practical, web-friendly standard library
- **JavaScript**: Modern async/await, arrow functions, destructuring

### 3. Compilation Target

- Primary target: Native machine code via LLVM
- Secondary targets: WebAssembly, JavaScript (planned for v2.0)
- Development mode: Tree-walking interpreter for debugging

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
/// صنف <name> [يرث <parent>] [يلتزم <interfaces>] { <body> }
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
    message: "لا يمكن تعيين قيمة لمتغير ثابت",
    // ...
}
```

### Testing

- Write tests for all new features
- Include Arabic source code in tests
- All 921+ tests must pass before committing

```rust
#[test]
fn test_arabic_identifiers() {
    let source = r#"
        متغير اسم_المستخدم = "أحمد"
        متغير عمر_المستخدم = 25
        اطبع(اسم_المستخدم + " عمره " + عمر_المستخدم)
    "#;
    // ...
}
```

## File Naming Conventions

- Rust source files: `snake_case.rs`
- Tarqeem source files: Arabic names with `.ترقيم` extension (or `.ترقيم`)
- Package manifest: `ترقيم.حزمة` (Arabic format)
- Lock file: `ترقيم.قفل`
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

3. **Commit Messages**:

   Format: `<type>(<scope>): <description>`

   Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `perf`, `ci`

   Examples:
   ```
   feat(lexer): add Arabic keyword tokenization
   fix(parser): handle RTL text in string literals
   docs(readme): add syntax examples
   refactor(ast): simplify node structure
   ```

## Common Tasks

### Adding a New Keyword

1. Add token to `src/lexer/token.rs`
2. Add Arabic mapping in `src/lexer/keywords.rs`
3. Add parsing logic in `src/parser/parser.rs`
4. Add AST node if needed in `src/parser/ast.rs`
5. Add semantic analysis in `src/semantic/`
6. Add IR generation in `src/ir/builder.rs`
7. Add code generation in `src/codegen/llvm/codegen.rs`
8. Add interpreter support in `src/interpreter/executor.rs`
9. Add tests for each stage
10. Update documentation

### Adding a Standard Library Function

1. Implement in Rust in `runtime-rs/src/` (for native functions)
2. Export with `#[no_mangle] extern "C"` for C ABI compatibility
3. Re-export from `runtime-rs/src/lib.rs`
4. Create Tarqeem wrapper in `stdlib/`
5. Register in `src/semantic/scope.rs` for type checking
6. Add codegen mapping if needed
7. Add documentation
8. Add tests

### Debugging the Compiler

```bash
# Verbose output
RUST_LOG=debug cargo run -- compile test.ترقيم

# Dump tokens
cargo run -- lex test.ترقيم

# Dump AST
cargo run -- parse test.ترقيم

# Dump LLVM IR
cargo run -- compile test.ترقيم --emit-llvm

# Run with interpreter (default for `run`)
cargo run -- run test.ترقيم

# Run with JIT (experimental)
cargo run -- run test.ترقيم --jit
```

## Architecture Decisions

### Why Recursive Descent Parser?

- Easier to implement and understand
- Better error messages and recovery
- Sufficient for our grammar (not highly ambiguous)
- Extended with Pratt parsing for expressions

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

### Why Tree-Walking Interpreter?

- Used for REPL and debugging
- Easier to implement source-level debugging
- Powers the DAP server for VS Code debugging

## Known Challenges

### 1. RTL Text Rendering

- IDE/editor support varies
- Arabic-only identifiers require proper RTL support
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

### Phase 1: Core Language ✅ Complete
- [x] Language specification
- [x] Lexer with Arabic support
- [x] Parser for all syntax
- [x] Type checker with generics
- [x] Semantic analysis

### Phase 2: Code Generation ✅ Complete
- [x] IR generation (three-address code, SSA)
- [x] Optimization passes (const folding, DCE, CSE, inlining, loop opt)
- [x] LLVM code generation
- [x] Object file linking

### Phase 3: Standard Library ✅ Complete
- [x] Collections (قائمة، مجموعة، خريطة، طابور، مكدس)
- [x] String utilities (نص)
- [x] Math library (رياضيات)
- [x] File I/O (ملفات)
- [x] Console I/O (طرفية)
- [x] Networking (شبكة)
- [x] Date/Time (وقت)
- [x] Error handling (أخطاء)

### Phase 4: Tooling ✅ Complete
- [x] Package manager (trqpm)
- [x] LSP server
- [x] DAP server (debugger)
- [x] Formatter
- [x] Documentation generator

### Phase 5: Quality Hardening 🔄 In Progress
See `docs/ROADMAP_V1.1-V1.5.md` for details:
- v1.1: Stability (test fixes, warnings) ✅
- v1.2: Performance (benchmarks, string interning) ✅
- v1.3: Maintainability (refactoring, coverage) ⏳
- v1.4: Polish (error messages, edge cases) ⏳
- v1.5: Consolidation (final hardening) ⏳

## Quality Metrics

| Metric | Current | Target (v1.5) |
|--------|---------|---------------|
| Tests | 921+ | 1,200+ |
| Warnings | 0 | 0 |
| Known Bugs | 0 | 0 |
| Code Coverage | Unknown | >80% |

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

# Run benchmarks
cargo bench

# Run the REPL
cargo run -- repl

# Compile a Tarqeem file
cargo run -- compile examples/مرحبا.ترقيم

# Run a Tarqeem file
cargo run -- run examples/مرحبا.ترقيم
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

## Project Documentation

| File | Purpose |
|------|---------|
| `CLAUDE.md` | This file - AI development guidelines |
| `ARCHITECTURE.md` | Technical architecture details |
| `LANGUAGE_SPEC.md` | Complete language specification |
| `README.md` | User documentation and examples |
| `GETTING_STARTED.md` | Quick start guide |
| `docs/AI_NOTES.md` | AI implementation decisions log |

---

Remember: **ترقيم ليست ترجمة - بل لغة برمجة عربية أصيلة**
(Tarqeem is not a translation - it is an authentic Arabic programming language)
