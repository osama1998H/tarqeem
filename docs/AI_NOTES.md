# AI Implementation Notes

This file serves as persistent memory for AI agents working on the Tarqeem codebase. Update this file after each significant change to maintain context across sessions.

---

## How to Use This File

AI agents MUST update this file:
1. After completing any significant implementation
2. When making architectural decisions
3. When discovering important patterns or constraints
4. When encountering and resolving issues

Each entry should include:
- Date and brief description
- What was changed/decided
- Why (rationale)
- Any follow-ups or risks

---

## Current State

### Last Updated
2025-12-20

### Project Phase
Phase 2: Code Generation (✅ Complete)
- IR infrastructure: Complete
- Type system: Complete (generics, inheritance, vtables)
- Code optimizer: Complete (const fold, DCE, CSE, inlining, loop opts)
- LLVM codegen: Complete (executables run)
- Interpreter mode: Complete (IR-based execution)

Phase 3 Prep: Module System (In Progress)
- Module loader infrastructure: Complete
- Export tracking: Complete
- Import resolution: Complete
- Super constructor call (أساس()): Fixed

### Known Issues
- None critical. All P0/P1 bugs resolved.

### In-Progress Work
- Standard library foundation (stdlib_trq/)

---

## Implementation Log

### 2024-12-20: Agent Context Awareness Implementation

**What**: Added comprehensive agent context engineering infrastructure to prevent bugs from context rot.

**Changes made**:
1. Created `.claude/rules/` with modular rules:
   - `00-operating-procedure.md` - Mandatory Explore→Plan→Implement→Verify workflow
   - `architecture.md` - Layer boundaries and invariants
   - `testing.md` - Testing requirements
   - `rust-style.md` - Path-scoped Rust coding standards
   - `arabic-support.md` - Arabic language handling rules

2. Created `.claude/commands/` with reusable workflows:
   - `safe-change.md` - Full safe change workflow
   - `explore.md` - Read-only exploration
   - `fix-issue.md` - Bug fix workflow
   - `add-feature.md` - New feature workflow
   - `review-code.md` - Code review checklist

3. Updated `CLAUDE.md`:
   - Added project map at the top
   - Added mandatory workflow section
   - Added critical invariants table
   - Added modular rules and commands reference
   - Added imports for ARCHITECTURE.md and README.md

4. Created `docs/AI_NOTES.md` (this file):
   - Persistent memory across sessions
   - Implementation log
   - Decision tracking

**Why**: AI agents optimize locally while missing global constraints. This causes bugs. The solution is:
- Mandatory workflow that forces exploration before coding
- Modular rules that are always loaded
- Structured notes that persist across sessions
- Slash commands for consistent workflows

**Risks**: None. This is additive documentation.

**Follow-ups**:
- Agents should follow the new workflow
- Update this file after each significant change

---

## Architectural Decisions

### Decision: Layer Boundaries
**Date**: Project inception
**Decision**: Compiler layers (Lexer→Parser→Semantic→IR→Codegen) can only depend on layers before them.
**Rationale**: Prevents circular dependencies and maintains clear separation of concerns.
**See**: `.claude/rules/architecture.md`

### Decision: Bilingual Error Messages
**Date**: Project inception
**Decision**: All user-facing messages must have both Arabic and English versions.
**Rationale**: Tarqeem is Arabic-first but needs to be accessible to English speakers.
**See**: `.claude/rules/arabic-support.md`

### Decision: NFC Normalization
**Date**: Project inception
**Decision**: All Arabic identifiers must be NFC-normalized before comparison.
**Rationale**: Arabic text can have multiple byte representations for the same visual characters.
**See**: `.claude/rules/arabic-support.md`

---

## Patterns Discovered

### Pattern: Token with Span
Every token must carry its source location. This is required for accurate error reporting.
```rust
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,  // REQUIRED
    pub lexeme: String,
}
```

### Pattern: Result Type Aliases
Use type aliases for complex Result types:
```rust
pub type ParseResult<T> = Result<T, ParseError>;
pub type TypeCheckResult<T> = Result<T, TypeError>;
```

### Pattern: Bilingual Diagnostic
```rust
Diagnostic {
    message: "English message",
    message_ar: "رسالة بالعربية",
    span: Span,
    level: DiagnosticLevel,
}
```

---

## Session Summaries

Use this section to summarize what was accomplished in each session.

### Session: 2024-12-20 - Agent Context Awareness
- Researched best practices from Anthropic documentation
- Implemented modular rules system
- Implemented slash commands
- Updated CLAUDE.md with project map and workflow
- Created this notes file

### Session: 2025-12-20 - Conway's Game of Life Stress Test

**Goal**: Stress test the Tarqeem compiler with a non-trivial program (Conway's Game of Life)

**Test Files Created**:
1. `examples/لعبة_الحياة.trq` - Full implementation (33 errors, exposed type system gaps)
2. `examples/لعبة_الحياة_بسيط.trq` - Simplified without arrays (global constant issues)
3. `examples/اختبار_بسيط.trq` - Minimal test (LLVM codegen bugs)

**Findings**:
| Component | Status |
|-----------|--------|
| Lexer | ✅ Working |
| Parser | ✅ Working |
| Semantic Analysis | ⚠️ Partial (arrays/generics broken) |
| IR Generation | ⚠️ Partial (simple cases work) |
| LLVM Codegen | ❌ Bugs (empty else blocks, type mismatches) |
| Execution | ❌ Not implemented |

**Critical Bugs Discovered**:
1. Empty else block in LLVM IR causes clang to fail
2. Global constants not visible in function scope during IR gen
3. Type mismatch: double returned as i64
4. Array indexing not implemented in type checker
5. For-in iteration over arrays not implemented
6. Generic types (مصفوفة<مصفوفة<منطقي>>) not fully supported

**See**: `docs/STRESS_TEST_REPORT.md` for detailed analysis

### Session: 2025-12-20 - Stress Test Bug Fixes (P0 Complete)

**Goal**: Fix the critical P0 bugs identified in the stress test to enable code execution

**Bugs Fixed**:

1. **Empty else block bug** (`src/ir/builder.rs:build_if`)
   - When `else_branch` is `None`, branch directly to `merge_block` instead of creating empty `else_block`
   - Prevents LLVM error about blocks without terminators

2. **Global constants visibility** (`src/ir/builder.rs`)
   - Added `global_constants: HashMap<String, (Constant, IrType)>` field
   - Added first pass in `build()` to collect global constants before processing functions
   - Modified `build_identifier()` to check globals after local scope

3. **C main entry point** (`src/codegen/llvm/codegen.rs`)
   - Added `emit_c_main_entry()` function to generate C `main()` that calls `__main__()`
   - Fixes "undefined reference to main" linking error

4. **Void function call destination** (`src/codegen/llvm/codegen.rs`)
   - Modified Call instruction handling to skip destination assignment for void returns
   - Fixes type mismatch errors in LLVM IR

5. **Unique block labels** (`src/codegen/llvm/codegen.rs`)
   - Added block ID to label names: `format!("{}.{}", label, block.id.0)`
   - Prevents duplicate label errors

6. **Return type tracking** (`src/codegen/llvm/codegen.rs`)
   - Added `current_return_type` field to track function return type
   - Fixed Return instruction to use proper type instead of defaulting to i64

7. **Variable type tracking** (`src/codegen/llvm/codegen.rs`)
   - Added `var_types.insert()` for Binary, Unary, IntToFloat, FloatToInt operations
   - Prevents "void parameter" errors in LLVM IR

8. **Implicit return for void functions** (`src/ir/builder.rs:build_func_decl`)
   - Changed from checking `func.blocks.last()` to checking current block
   - Only adds implicit return for void functions
   - Fixes functions ending with if-else that had unreachable merge blocks

**Test Results**:
- All 84 unit tests pass
- 5/8 example files compile and run correctly:
  - ✅ اختبار_بسيط.trq - arithmetic, factorial, recursion
  - ✅ دوال.trq - function calls
  - ✅ لعبة_الحياة_بسيط.trq - Game of Life (simplified)
  - ✅ متغيرات.trq - variables and arrays
  - ✅ مرحبا.trq - hello world
- 3/8 examples need P1 features (array indexing, for-in, empty array inference)

**Remaining P1 Work**:
- Empty array type inference (`متغير arr: مصفوفة<عدد> = []`)
- Array indexing IR generation (`arr[i]`)
- For-in iteration over arrays (`لكل x في arr`)

**See**: `docs/STRESS_TEST_FIX_PLAN.md` for detailed implementation plan

### Session: 2025-12-20 - Phase 3 Prerequisites

**Goal**: Prepare for Phase 3 (Standard Library) by implementing module system infrastructure

**What was validated**:
- Array indexing: Already working (confirmed in test cases)
- For-in iteration: Already working (confirmed in test cases)
- Only real remaining issue: super constructor call (أساس())

**Bugs Fixed**:

1. **Super constructor call (أساس())** (`src/semantic/analyzer.rs`, `src/ir/builder.rs`)
   - Added `analyze_super_constructor_call()` method to Analyzer
   - Added special-case handling in `ExprKind::Call` for `ExprKind::Super` callee
   - Added `build_super_constructor_call()` method to IrBuilder
   - Validates: in class context, parent exists, correct argument count

**Module System Implementation**:

1. **Module Infrastructure (M1)** (`src/semantic/modules.rs`)
   - `ModuleId`: Unique identifier based on canonical path
   - `ModuleLoader`: Handles loading, caching, and cycle detection
   - `LoadedModule`: Parsed module with AST and exports
   - `ExportedSymbol`: Tracks exported functions, classes, interfaces, variables
   - Path resolution supports: relative (`./`), absolute, search paths
   - File extensions: `.trq` and `.ترقيم`
   - Index files: `index.trq` and `فهرس.ترقيم`

2. **Export Tracking (M2)** (`src/semantic/modules.rs`)
   - `collect_exports()` scans AST for exported declarations
   - Tracks export kind (Function, Class, Interface, Variable, Constant)

3. **Import Resolution (M3)** (`src/semantic/analyzer.rs`)
   - Full `analyze_import()` implementation
   - Resolves module paths relative to current file
   - Loads modules and imports symbols with correct types
   - Handles named imports, aliases, and wildcard imports
   - Circular dependency detection with helpful error messages

**Files Changed**:
- `src/semantic/analyzer.rs` - super constructor + module integration
- `src/ir/builder.rs` - super constructor IR generation
- `src/semantic/modules.rs` - new module system infrastructure
- `src/semantic/mod.rs` - export new module types
- `src/parser/parser.rs` - `from_tokens()` constructor

**Test Results**: 108 tests passing (up from 101)

**See**: `docs/PHASE3_PREP_PLAN.md` for detailed implementation plan

---

## TODOs

Track follow-up items here:

**Completed (2025-12-20)**:
- [x] Fix empty else block bug in LLVM codegen
- [x] Fix type mismatch in return statements
- [x] Make global constants visible in IR generation
- [x] Add C main entry point for executables
- [x] Fix void function call destination
- [x] Fix implicit return for void functions after if-else

**Completed (2025-12-20 Phase 3 Prep)**:
- [x] Fix super constructor call (أساس()) semantic issues
- [x] Implement module system infrastructure
- [x] Implement export tracking
- [x] Implement import resolution

**Pending**:
- [ ] Create stdlib_trq/ directory structure
- [ ] Implement core standard library modules
- [ ] Add more path-scoped rules as patterns emerge
- [ ] Create integration tests for the compiler pipeline
- [ ] Document common error patterns and solutions

---

## Template for New Entries

```markdown
### YYYY-MM-DD: Brief Title

**What**: One-line description of the change.

**Changes made**:
1. First change
2. Second change

**Why**: Rationale for the change.

**Risks**: Any potential issues.

**Follow-ups**: Future work needed.
```
