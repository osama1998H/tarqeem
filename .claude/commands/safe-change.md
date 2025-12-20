# Safe Change Workflow

Perform a safe, architecture-aware change: $ARGUMENTS

## Instructions

Follow this procedure EXACTLY. Do not skip steps.

### Step 1: EXPLORE (Read-Only)

Before writing ANY code:

1. **Identify the owner module** - Which part of the compiler pipeline owns this?
   - Lexer, Parser, Semantic, IR, Codegen, CLI, Error?

2. **Find existing patterns** - Search for 2-3 similar implementations:
   ```
   - Use Grep to find related code
   - Read the files to understand the pattern
   - List what you found
   ```

3. **List relevant files** - Name the 5-10 most important files and explain why.

4. **Check constraints** - Read these files for invariants:
   - `.claude/rules/architecture.md`
   - `.claude/rules/00-operating-procedure.md`
   - `ARCHITECTURE.md`

5. **State the impact** - What other modules/tests might be affected?

**STOP HERE. Do not proceed until exploration is complete.**

### Step 2: PLAN

Create a written plan:

1. **Steps** - Numbered list of what you'll do
2. **Files** - Exact paths you'll modify
3. **Tests** - What tests you'll run/add
4. **Risks** - What could go wrong

**Share the plan before proceeding to implementation.**

### Step 3: IMPLEMENT

Apply minimal, focused changes:

- Reuse existing patterns (don't invent new ones)
- Keep diff small
- Add bilingual error messages if user-facing
- Follow Rust style from `.claude/rules/rust-style.md`

### Step 4: VERIFY

Run these commands:

```bash
cargo fmt
cargo clippy
cargo test
```

Report results. If tests fail, diagnose before marking complete.

### Step 5: DOCUMENT

Update `docs/AI_NOTES.md` with:
- What was changed and why
- Decisions made
- Any risks or follow-ups
