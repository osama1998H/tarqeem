# Tarqeem Alpha Version Development Roadmap

<div dir="rtl" align="right">

# ترقيم - خارطة طريق نسخة ألفا

**أول لغة برمجة عربية مُترجَمة للأغراض العامة**

</div>

---

## Executive Summary

**Tarqeem** is a fully-featured, compiled, Arabic-first programming language with LLVM backend. This document consolidates the complete development roadmap from initial conception to V1 release and beyond.

| Metric | Value |
|--------|-------|
| Lines of Code | 39,300+ |
| Tests Passing | 921+ |
| V1 Readiness | 100% |
| Bilingual Support | Complete (Arabic/English) |
| LLVM Codegen | Working |
| Standard Library | Complete (10 modules) |

---

## Compiler Architecture

```
Source (.trq/.ترقيم) → Lexer → Parser → Semantic → IR → Optimizer → LLVM → Executable
```

### Layer Boundaries (CRITICAL)
Each layer can ONLY depend on layers before it:
- **Lexer**: Characters → Tokens (Arabic keywords, Unicode, NFC normalization)
- **Parser**: Tokens → AST (recursive descent + Pratt parsing)
- **Semantic**: AST → Typed AST (scope, types, generics, inheritance)
- **IR**: Typed AST → SSA IR (three-address code, CFG)
- **Optimizer**: IR → Optimized IR (const fold, DCE, CSE, inlining)
- **Codegen**: IR → LLVM IR → Native binary

---

## Development Phases

### Phase 1: Core Language ✅ COMPLETE
- [x] Language specification
- [x] Lexer with Arabic keyword tokenization
- [x] Parser (recursive descent + Pratt for expressions)
- [x] Type system (static typing with inference)
- [x] Scope and name resolution
- [x] Bilingual error messages (Arabic/English)
- [x] CLI commands (compile, run, check, repl)

### Phase 2: Code Generation ✅ COMPLETE
- [x] IR infrastructure (SSA form, basic blocks, CFG)
- [x] Type system completion (generics, inheritance, vtables)
- [x] Optimizer passes:
  - Constant folding
  - Dead code elimination (DCE)
  - Common subexpression elimination (CSE)
  - Function inlining
  - Loop optimizations
- [x] LLVM codegen (working executables)
- [x] Interpreter mode (IR-based execution)
- [x] C runtime library (1,700+ LOC)

### Phase 3: Standard Library ✅ COMPLETE
10 milestones implemented:

| Milestone | Module | Arabic Name | Status |
|-----------|--------|-------------|--------|
| 3.0 | P1 Bug Fixes | - | ✅ |
| 3.1 | Module System | استورد/صدّر | ✅ |
| 3.2 | Core Collections | مجموعات | ✅ |
| 3.3 | String Utilities | نص | ✅ |
| 3.4 | Math Library | رياضيات | ✅ |
| 3.5 | File System | ملفات | ✅ |
| 3.6 | I/O and Console | طرفية | ✅ |
| 3.7 | Networking | شبكة | ✅ |
| 3.8 | Date/Time | وقت | ✅ |
| 3.9 | Error Handling | أخطاء | ✅ |

**Standard Library Contents**:
- **مجموعات**: قائمة<ن>, مجموعة<ن>, خريطة<م،ق>, طابور<ن>, مكدس<ن>, متكرر
- **نص**: String manipulation, StringBuilder (باني_نص), formatting
- **رياضيات**: Basic math, trigonometry, random numbers, constants
- **ملفات**: File I/O (ملف), path handling (مسار), directories (مجلد)
- **طرفية**: Console I/O, ANSI colors, formatted output
- **شبكة**: TCP/UDP connections, HTTP client, servers
- **وقت**: Date (تاريخ), time (وقت), duration (مدة), formatting
- **أخطاء**: Error types, نتيجة<ن،خ> (Result), اختياري<ن> (Option)

### Phase 4: Tooling 📋 PLANNED
| Milestone | Tool | Arabic Name | Status |
|-----------|------|-------------|--------|
| 4.1 | Package Manager | مدير الحزم (trqpm) | Planned |
| 4.2 | LSP Server | خادم LSP | Planned |
| 4.3 | VS Code Extension | إضافة VS Code | Planned |
| 4.4 | Documentation Generator | مولد التوثيق | Planned |
| 4.5 | Code Formatter | منسق الكود | Planned |
| 4.6 | Debugger (DAP) | مصحح الأخطاء | Planned |

---

## V1 Release Audit Summary

### Critical Issues Fixed ✅
| Issue | Category | Status |
|-------|----------|--------|
| Parser error recovery | Parser | Fixed |
| Codegen unwrap safety | Codegen | Fixed |
| Empty else block bug | IR/LLVM | Fixed |
| Global constants visibility | IR | Fixed |
| Type mismatch in returns | Codegen | Fixed |
| Void function call handling | Codegen | Fixed |
| Unique block labels | Codegen | Fixed |
| Implicit void returns | IR | Fixed |
| Super constructor calls | Semantic | Fixed |
| Unicode NFC normalization | Lexer | Fixed |

### Test Coverage
- **Unit Tests**: 857+ tests
- **Integration Tests**: 64+ tests
- **Example Files**: 8 working examples
- **Stress Test**: Conway's Game of Life

---

## Key Invariants

| Invariant | Rule |
|-----------|------|
| Layer Boundaries | Lexer→Parser→Semantic→IR→Codegen (no reverse deps) |
| Bilingual Messages | ALL user-facing strings need Arabic + English |
| NFC Normalization | Arabic identifiers MUST be normalized before comparison |
| Error Recovery | Never `panic!()` or `unwrap()` on user input |
| Token Spans | Every token must have accurate source location |

---

## Sahari Editor (Future)

**صحاري** - Arabic-First Code Editor (VS Code fork)

### Status
- Phase 1 (Foundation): ✅ Complete
- Phase 2 (RTL Enhancement): ✅ Complete
- Phase 3 (Tarqeem Integration): 📋 Planned

### Key Features
- RTL support via Monaco Editor PR #255455
- Auto-detect Arabic content for RTL mode
- Bundled Tarqeem extension
- Arabic fonts (Amiri, Noto Sans Arabic)
- Bilingual UI

---

## File Extensions

| Type | ASCII | Arabic |
|------|-------|--------|
| Source | `.trq` | `.ترقيم` |
| Header | `.trqh` | `.ترقيم-ر` |
| Package Manifest | `حزمة.toml` | - |

---

## Commands Reference

```bash
# Build
cargo build --release

# Run tests
cargo test

# Compile Tarqeem file
cargo run -- compile examples/مرحبا.trq

# Run Tarqeem file
cargo run -- run examples/مرحبا.trq

# Check syntax
cargo run -- check examples/مرحبا.trq

# Interactive REPL
cargo run -- repl

# Lint
cargo clippy

# Format
cargo fmt
```

---

## Known Limitations

1. **Doc comments**: `///` before declarations (infrastructure added, full support in Phase 4.4)
2. **Function types in generics**: Not yet supported in type annotations
3. **WebAssembly target**: Planned for future release

---

## Project Timeline Summary

| Phase | Duration | Status |
|-------|----------|--------|
| Phase 1: Core Language | Complete | ✅ |
| Phase 2: Code Generation | Complete | ✅ |
| Phase 3: Standard Library | Complete | ✅ |
| Phase 4: Tooling | ~14 weeks est. | 📋 |
| V1 Release | Ready | ✅ |

---

## Resources

- **Repository**: https://github.com/osama1998H/tarqeem
- **Architecture**: See `ARCHITECTURE.md`
- **Developer Guide**: See `CLAUDE.md`
- **README**: See `README.md`

---

<div dir="rtl" align="right">

## عن ترقيم

**ترقيم** - أول لغة برمجة عربية مُترجَمة للأغراض العامة. صُممت لجعل البرمجة متاحة للناطقين بالعربية مع الحفاظ على جودة المترجمات الاحترافية.

</div>
