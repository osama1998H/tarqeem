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
gh issue create --title "[BUG] Brief description" --label "bug" --label "auto-detected" --body "## Description
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
- Commit: commit-hash
- Detected by: Claude Code hook"
```

### Step 3: Report to User

After creating an issue, always inform the user:
```
Created GitHub issue #XXX: [issue title]
URL: https://github.com/osama1998H/tarqeem/issues/XXX
```

## Issue Labels

Use these labels appropriately:

| Label | When to Use |
|-------|-------------|
| `bug` | All bug issues |
| `auto-detected` | Issues created automatically by hooks |
| `test-failure` | Test-related bugs |
| `regression` | Previously working code broke |
| `clippy` | Clippy error issues |
| `compile-error` | Compilation failures |

## Issue Title Format

Use this format for issue titles:
- `[BUG] Brief description of the problem`
- `[REGRESSION] Feature X stopped working`
- `[TEST] Test name failing`

## Bug Detection Context

When the bug-detector hook fires, it will provide context like:

```
=== BUG DETECTED ===
Type: test-failure
Summary: Test failure detected
Bug file: /tmp/tarqeem-bug-TIMESTAMP.json
```

Use this information to create a detailed issue.

## Example Workflow

1. User runs `cargo test`
2. Hook detects test failure
3. You see the bug detection output
4. Check: `gh issue list --state open --search "test failure"`
5. If no existing issue, create one:
   ```bash
   gh issue create --title "[BUG] Test failure in lexer module" --label "bug" --label "auto-detected" --label "test-failure" --body "..."
   ```
6. Report: "Created GitHub issue #123 for the test failure"

## Important Notes

- Always use single-line `gh` commands (no backslash line continuations)
- Include the commit hash and branch in the issue body
- Link to relevant files when possible
- If unsure whether to create an issue, ASK the user first
