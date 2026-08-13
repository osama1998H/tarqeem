---
description: Automatic bug tracking and GitHub issue creation for Tarqeem
globs:
  - "**/*.rs"
  - "**/Cargo.toml"
  - "tests/**"
---

# Bug Tracking and GitHub Issue Creation

## Overview

When bugs are detected during development (test failures, compilation errors, clippy errors), you should create GitHub issues to track them. This ensures no bugs are lost and provides traceability.

## When to Create Issues

You MUST create a GitHub issue when:

1. **Test failures**: A test that was previously passing now fails
2. **Regression**: Previously working functionality breaks
3. **Compilation errors**: Code that was compiling now fails to compile
4. **Clippy errors**: New clippy errors (not warnings) are introduced

## When NOT to Create Issues

Do NOT create issues for:

- **Work in progress**: Intentional changes that temporarily break tests
- **Known issues**: Bugs already tracked in existing GitHub issues
- **Intentional changes**: Refactoring that is expected to break things temporarily
- **Warnings only**: Clippy warnings (not errors) unless specifically requested
- **User-caused errors**: Syntax errors the user is actively fixing

## Issue Creation Process

### Step 1: Check for Existing Issues

Before creating a new issue, ALWAYS check if one already exists:

```bash
gh issue list --state open --search "relevant keywords"
```

### Step 2: Create Issue with Structured Format

Use this command format (single line, no backslash continuations):

```bash
gh issue create --title "[BUG] Brief description" --label "bug" --body "## Description
What went wrong and when it was detected.

## Steps to Reproduce
1. Command that triggered the bug
2. Expected behavior
3. Actual behavior

## Error Output
\`\`\`
Paste error message here
\`\`\`

## Affected Files
- List affected files

## Environment
- Branch: branch-name
- Commit: commit-hash"
```

**No AI attribution.** Issues and pull requests must carry no trace of the tool that
wrote them — no "Generated with", no model name, no hook name, and no
`Co-Authored-By` trailer on the commits. This is a hard line in `CLAUDE.md`.

### Step 3: Report to User

After creating an issue, always inform the user:
```
Created GitHub issue #XXX: [issue title]
URL: https://github.com/osama1998H/tarqeem/issues/XXX
```

## Issue Labels

Only these labels exist. `gh issue create` **fails outright** if you pass a label
that does not exist, so never invent one — check with `gh label list` first.

| Label | When to Use |
|-------|-------------|
| `bug` | All bug issues |
| `test-failure` | A test that was passing now fails |
| `regression` | Previously working functionality broke |
| `enhancement` | Feature requests and chores |
| `code-quality` | Comment bloat, readability, maintainability (see `comments.md`) |
| `documentation` | Docs and comments |

Combine them: a broken test from a bad merge is `bug` + `test-failure` + `regression`.

## Issue Title Format

Use this format for issue titles:
- `[BUG] Brief description of the problem`
- `[REGRESSION] Feature X stopped working`
- `[TEST] Test name failing`

## Bug Detection Context

There is no detector hook. **You are the detector.** The trigger is you observing a
failure in output you just read — a failing `cargo test`, a `cargo clippy` error, a
build that stopped compiling — not an automated signal.

That means the judgment is yours: a failure you caused two minutes ago and are about
to fix is not an issue; a failure that was already there, or one you cannot fix in
this task, is.

## Example Workflow

1. You run `cargo test` and see a failure unrelated to your current change
2. Check: `gh issue list --state open --search "test failure lexer"`
3. If no existing issue, create one:
   ```bash
   gh issue create --title "[BUG] Test failure in lexer module" --label "bug" --label "test-failure" --body "..."
   ```
4. Add it to the roadmap board with status `Todo` (see below)
5. Report: "Created issue #123 for the test failure" — then resume the original task

---

# Roadmap Project Board

Issues alone do not show priority. The repository project board does. It is the
answer to "which of these 40 open issues matters?"

**Board**: `Tarqeem beta Roadmap` (owner `osama1998H`, currently project number 4).
When the beta closes it is replaced by the next one — Alpha, and so on. Never
hardcode the number or the field IDs; discover them.

**Statuses**: `Todo` → `Planning` → `In Progress` → `Done`

`Done` is handled by repository automation when an issue closes. **Never move an item
to `Done` by hand.**

If a future board uses different names, map by position rather than guessing:
first = backlog, second = planned, third = active, fourth = finished.

## Access

Board commands need the `project` scope. If a `gh project` call returns
`missing required scopes`, tell the user to run:

```
! gh auth refresh -s project
```

Note that `gh`'s own error text suggests `-s read:project`. That is **not enough** —
it grants read only, and the planning gate has to *move* items between columns. Ask
for `project`, which covers both.

Do not retry and do not work around it. But **a scope failure is not always fatal**:

| Situation | On scope failure |
|-----------|------------------|
| The planning gate (below) | Board access *is* the task → stop and wait |
| Filing an issue during other work | File the issue anyway, mention the missing scope once, skip the board add, continue |

A side task must never halt the primary one.

## Discovery

IDs are discovered at runtime so the rule survives the board being replaced:

```bash
gh project list --owner osama1998H
gh project field-list <number> --owner osama1998H --format json
gh project item-list <number> --owner osama1998H --format json
```

A newly filed issue is added to the board with status `Todo`.

## The Planning Gate

**This applies whenever anyone asks to start working on tickets.** Read the `Planning`
column *before* doing anything else.

### `Planning` has items → work from it

Take one, move it to `In Progress`, branch per Gitflow, one issue per PR. Do not
re-plan; the selection was already made and agreed.

### `Planning` is empty → select five, then stop

1. Choose **5** open issues
2. Move them `Todo` → `Planning`
3. Report them to the user: one line of rationale each, plus a suggested order
4. **Stop.** Let the user pick which to start. Do not begin coding in the same turn.

Selecting five and immediately writing code for one defeats the purpose — the point
is that the user sees the shortlist and its reasoning before work begins.

### Choosing the five

In priority order:

1. **Cluster over spread.** Five issues in one pipeline layer compound; five scattered
   across lexer, LSP, and codegen do not. Context earned on the first makes the rest cheaper.
2. **Unblockers first.** An issue that other issues depend on outranks a bigger isolated one.
3. **Silent wrong output over cosmetic.** A backend that quietly produces the wrong
   answer is this project's recurring failure mode and outranks a formatting nit.
4. **Prefer issues with a reproduction** over ones that still need investigation.

State which of these drove each pick. "Grouped with #253 — same MethodId naming
bug" is a rationale; "high impact" is not.

## Pull Requests

One issue per PR by default. If two genuinely must share one, say why in the body and
link both. Reference the issue so the board automation can close it (`Fixes #123`).

---

## Important Notes

- Always use single-line `gh` commands (no backslash line continuations)
- Include the commit hash and branch in the issue body
- Link to relevant files when possible
- If unsure whether to create an issue, ASK the user first
