# AI Implementation Notes

Persistent memory for AI agents working on Tarqeem. Update after significant changes.

---

## Current State

**Last Updated**: 2025-12-21
**Phase**: V1 Release Complete
**Tests**: 921+ passing
**Known Issues**: None critical

---

## Architectural Decisions

- **Layer Boundaries**: Lexer→Parser→Semantic→IR→Codegen (no reverse deps)
- **Bilingual Messages**: All user-facing strings have Arabic + English
- **NFC Normalization**: Arabic identifiers normalized before comparison
- **Error Recovery**: Never panic/unwrap on user input

---

## Implementation Log (Outline)

### 2024-12-20: Agent Context Awareness
- Created `.claude/rules/` modular rules system
- Created `.claude/commands/` reusable workflows
- Updated CLAUDE.md with project map

### 2025-12-20: Stress Test & Fixes
- Created Conway's Game of Life stress test
- Fixed empty else block bug
- Fixed global constants visibility
- Fixed C main entry point
- Fixed void function handling
- Fixed implicit returns

### 2025-12-20: Phase 3 Standard Library
- Implemented module system (استورد/صدّر)
- Created مجموعات (Collections): قائمة, مجموعة, خريطة, طابور, مكدس
- Created نص (String utilities)
- Created رياضيات (Math library)
- Created ملفات (File system)
- Created طرفية (Console I/O)
- Created شبكة (Networking)
- Created وقت (Date/Time)
- Created أخطاء (Error handling)

### 2025-12-20: Parser Updates
- Added generic type parameters for classes/interfaces
- Added automatic semicolon insertion
- Added generic type arguments in `new` expressions

### 2025-12-20: Phase 4 Planning
- Created comprehensive Phase 4 plan (Tooling)
- Designed package manager (trqpm)
- Planned LSP server, VS Code extension

### 2025-12-21: Global Variables
- Added GlobalLoad/GlobalStore IR instructions
- Updated LLVM codegen for globals
- Updated interpreter for globals

---

## Key Patterns

- **Token with Span**: Every token carries source location
- **Result Type Aliases**: `ParseResult<T>`, `TypeCheckResult<T>`
- **Bilingual Diagnostic**: `message` + `message_ar` fields

---

## TODOs

**Completed**:
- [x] Phase 1-3 complete
- [x] V1 release ready
- [x] All critical bugs fixed

**Pending**:
- [ ] Phase 4: Tooling implementation
- [ ] Package manager (trqpm)
- [ ] LSP server
- [ ] VS Code extension

---

### 2025-12-23: Phase 1 Arabic Philosophy Audit

**Task**: Remove `__xxx__` pattern from 170+ builtin functions

**Architecture Insight**:
- scope.rs: Registers builtin functions with Arabic names (type checking)
- runtime/: C library with `trq_*` functions (actual implementation)
- stdlib_trq/: Uses `__xxx__` pattern to call internal functions
- codegen: Generates LLVM IR, needs mapping Arabic→trq_*

**Solution Design**:
1. Add function name mapping in codegen (Arabic → `trq_*`)
2. Register all runtime functions in scope.rs
3. Update stdlib_trq to use clean Arabic function names
4. Update examples and tests

**Files Modified**:
- src/codegen/llvm/codegen.rs - Add runtime function mapping
- src/semantic/scope.rs - Register additional builtin functions
- stdlib_trq/**/*.ترقيم - Remove __xxx__ pattern
- examples/ - Update function calls
- tests/ - Update test code

---

## Session Template

```markdown
### YYYY-MM-DD: Title
- What was changed
- Why
- Files modified
- Test results
```
