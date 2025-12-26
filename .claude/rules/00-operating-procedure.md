# Agent Operating Procedure (MANDATORY)

This file defines the mandatory workflow that the agent MUST follow for ALL code changes. Skipping these steps leads to bugs and architectural violations.

## The Golden Rule

**NEVER write code until you understand the system.**

The agent optimizes locally (current file/diff) while missing global constraints (architecture, invariants, cross-module contracts). This procedure prevents that failure mode.

## Mandatory Workflow: Explore → Plan → Implement → Verify

### Phase 1: EXPLORE (Read-Only)

Before ANY code changes, the agent MUST:

1. **Identify the owning module(s)** - Which part of the compiler pipeline owns this behavior?
   - Lexer (`src/lexer/`) - Tokenization
   - Parser (`src/parser/`) - AST generation
   - Semantic (`src/semantic/`) - Type checking, scope analysis
   - IR (`src/ir/`) - Intermediate representation
   - Codegen (`src/codegen/`) - LLVM code generation
   - CLI (`src/cli/`) - Command-line interface
   - Error (`src/error/`) - Diagnostics

2. **Locate existing patterns** - Find 2-3 similar implementations already in the codebase. Do NOT invent new patterns if one exists.

3. **List relevant files** - Name the 5-10 most relevant files and explain why each matters.

4. **Check dependencies** - What other modules depend on this code? What will break?

5. **Read the invariants** - Check `CLAUDE.md` and `ARCHITECTURE.md` for constraints.

**DO NOT PROCEED TO PHASE 2 UNTIL PHASE 1 IS COMPLETE.**

### Phase 2: PLAN

Create a written plan that includes:

1. **Steps** - Numbered list of implementation steps
2. **Files to modify** - Exact file paths
3. **Architectural impact** - How does this affect the compiler pipeline?
4. **Breaking changes** - Any backward compatibility concerns?
5. **Test strategy** - Which tests to run/write?
6. **Risks** - What could go wrong?

The plan should be shared with the user before proceeding (if significant).

**DO NOT PROCEED TO PHASE 3 UNTIL THE PLAN IS APPROVED (for significant changes).**

### Phase 3: IMPLEMENT

Execute the plan with these constraints:

1. **Minimal diff** - Only change what's necessary
2. **Reuse patterns** - Copy existing patterns, don't invent new ones
3. **Bilingual messages** - All user-facing strings need Arabic + English
4. **Type safety** - Use Rust's type system; avoid `unwrap()` in production code
5. **Document why** - Add comments for non-obvious logic

### Phase 4: VERIFY

After implementation:

1. **Run tests** - `cargo test` (at minimum)
2. **Run lints** - `cargo clippy`
3. **Check types** - `cargo check`
4. **Format code** - `cargo fmt`
5. **Manual test** - If possible, test with a `.ترقيم` file

### Phase 5: DOCUMENT

If the change introduces new patterns or modifies architecture:

1. **Update `docs/AI_NOTES.md`** - Record decisions, discoveries, risks
2. **Update CLAUDE.md** - If new guidelines are needed
3. **Update ARCHITECTURE.md** - If pipeline/structure changes

## Pre-Change Checklist

Before editing any file, answer these questions:

- [ ] What module owns this behavior?
- [ ] What are 2 existing examples of the same pattern in this codebase?
- [ ] What are the invariants I must not break?
- [ ] What are the expected side effects (other modules, tests)?
- [ ] What tests prove correctness?

## Post-Change Checklist

Before marking the task complete:

- [ ] What files changed and why (grouped by module)?
- [ ] What commands were run and what passed/failed?
- [ ] Is there any backward-compatibility risk?
- [ ] Does documentation need updating?
- [ ] Did I update `docs/AI_NOTES.md` with my decisions?

## When to Stop and Ask

Stop and ask the user if:

1. The change touches >3 modules
2. The change affects the public API
3. The change requires new dependencies
4. You're unsure about architectural implications
5. Tests are failing and you don't understand why

## Context Preservation

For long sessions or complex tasks:

1. **Write notes** - Update `docs/AI_NOTES.md` after each significant step
2. **Summarize progress** - Periodically summarize what's been done
3. **Track decisions** - Document WHY choices were made, not just WHAT
