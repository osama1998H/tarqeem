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
2024-12-20

### Project Phase
Phase 2: Code Generation (In Progress)
- IR infrastructure: Complete
- Type system: Complete
- Code optimizer: Complete
- LLVM codegen: Complete

### Known Issues
- None currently tracked

### In-Progress Work
- None currently tracked

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

---

## TODOs

Track follow-up items here:

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
