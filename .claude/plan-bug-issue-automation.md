# Plan: Automatic GitHub Issue Creation for Bugs

## Overview

This plan combines **Claude Code rules** (`.claude/rules/`) with **hooks** to automatically create GitHub issues when bugs are detected during development.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Bug Detection & Issue Creation               │
└─────────────────────────────────────────────────────────────────┘

Detection Sources:
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
│  Test Failures   │  │  Clippy Warnings │  │  Compile Errors  │
│  (cargo test)    │  │  (cargo clippy)  │  │  (cargo check)   │
└────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘
         │                     │                     │
         └─────────────────────┼─────────────────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │  PostToolUse Hook    │
                    │  (Bash matcher)      │
                    │  Detects failures    │
                    └──────────┬───────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │  .claude/rules/      │
                    │  bug-tracking.md     │
                    │  (Instructs AI)      │
                    └──────────┬───────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │  gh issue create     │
                    │  (Creates issue)     │
                    └──────────────────────┘
```

---

## Component 1: Rule for Bug Tracking

**File**: `.claude/rules/bug-tracking.md`

This rule instructs the AI agent on when and how to create GitHub issues.

```markdown
---
description: Automatic bug tracking and GitHub issue creation
globs:
  - "**/*.rs"
  - "**/Cargo.toml"
---

# Bug Tracking and GitHub Issue Creation

## When to Create Issues

You MUST create a GitHub issue when:
1. **Test failures**: A test that was passing now fails
2. **Regression**: Previously working functionality breaks
3. **Compilation errors**: Code that was compiling now fails
4. **Clippy errors**: New clippy warnings introduced (not existing ones)

## When NOT to Create Issues

Do NOT create issues for:
- Intentional changes (refactoring that temporarily breaks tests)
- Work in progress (WIP commits)
- Known issues being actively worked on
- Issues that already exist in GitHub

## Issue Creation Process

### Step 1: Check for Existing Issues
Before creating, always check:
```bash
gh issue list --state open --search "keyword"
```

### Step 2: Create Issue with Structured Format
Use this template:
```bash
gh issue create \
  --title "[BUG] Brief description" \
  --body "## Description
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
- file1.rs
- file2.rs

## Suggested Fix
Brief suggestion if obvious.

## Context
- Detected by: Claude Code hook
- Branch: $(git branch --show-current)
- Commit: $(git rev-parse --short HEAD)" \
  --label "bug" \
  --label "auto-detected"
```

### Step 3: Log the Issue
After creation, inform the user:
```
Created GitHub issue #XXX for the detected bug.
```

## Issue Labels

Use these labels:
- `bug` - For bugs
- `auto-detected` - Created automatically by hooks
- `test-failure` - Test-related bugs
- `regression` - Previously working code broke
- `clippy` - Clippy warning issues
```

---

## Component 2: Enhanced Hook for Bug Detection

**File**: `.claude/hooks/bug-detector.sh`

This hook detects bugs from cargo commands and signals to the AI.

```bash
#!/bin/bash
# Hook: Bug Detector - Detects test failures, clippy errors, compilation errors
# Signals to AI agent to create GitHub issues for new bugs
set -euo pipefail

input=$(cat)
command=$(echo "$input" | jq -r '.tool_input.command // empty')
stdout=$(echo "$input" | jq -r '.tool_output.stdout // empty')
stderr=$(echo "$input" | jq -r '.tool_output.stderr // empty')
exit_code=$(echo "$input" | jq -r '.tool_output.exit_code // 0')

cd "${CLAUDE_PROJECT_DIR:-/home/user/tarqeem}"

# Only process cargo commands
if [[ ! "$command" =~ ^cargo ]]; then
  exit 0
fi

# Combine output for analysis
full_output="$stdout$stderr"

# Initialize bug detection
bug_detected=false
bug_type=""
bug_summary=""
error_output=""

# Detect test failures
if [[ "$command" =~ cargo\ test ]] && [[ "$exit_code" != "0" ]]; then
  failed_tests=$(echo "$full_output" | grep -E "^test .* FAILED$" | head -5 || true)
  if [ -n "$failed_tests" ]; then
    bug_detected=true
    bug_type="test-failure"
    bug_summary="Test failure detected"
    error_output="$failed_tests"
  fi
fi

# Detect clippy errors (not warnings)
if [[ "$command" =~ cargo\ clippy ]] && [[ "$full_output" =~ "error:" ]]; then
  clippy_errors=$(echo "$full_output" | grep -E "^error\[" | head -5 || true)
  if [ -n "$clippy_errors" ]; then
    bug_detected=true
    bug_type="clippy"
    bug_summary="Clippy error detected"
    error_output="$clippy_errors"
  fi
fi

# Detect compilation errors
if [[ "$command" =~ cargo\ (build|check) ]] && [[ "$exit_code" != "0" ]]; then
  compile_errors=$(echo "$full_output" | grep -E "^error\[" | head -5 || true)
  if [ -n "$compile_errors" ]; then
    bug_detected=true
    bug_type="compile"
    bug_summary="Compilation error detected"
    error_output="$compile_errors"
  fi
fi

# If bug detected, provide context to AI
if [ "$bug_detected" = true ]; then
  # Save bug info for AI to use
  bug_file="/tmp/tarqeem-bug-$(date +%Y%m%d-%H%M%S).json"
  cat > "$bug_file" << EOF
{
  "type": "$bug_type",
  "summary": "$bug_summary",
  "command": "$command",
  "branch": "$(git branch --show-current 2>/dev/null || echo 'unknown')",
  "commit": "$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')",
  "timestamp": "$(date -Iseconds)",
  "error_preview": $(echo "$error_output" | head -10 | jq -Rs .)
}
EOF

  # Output context for AI
  echo ""
  echo "=== BUG DETECTED ==="
  echo "Type: $bug_type"
  echo "Summary: $bug_summary"
  echo "Bug file: $bug_file"
  echo ""
  echo "Per .claude/rules/bug-tracking.md, consider creating a GitHub issue:"
  echo "  1. Check if issue already exists: gh issue list --state open --search \"$bug_summary\""
  echo "  2. If new bug, create issue with: gh issue create --title \"[BUG] $bug_summary\" --label bug --label auto-detected --label $bug_type"
  echo "===================="
fi

exit 0
```

---

## Component 3: Custom Command for Issue Creation

**File**: `.claude/commands/create-bug-issue.md`

```markdown
# Create Bug Issue

Create a GitHub issue for a detected bug.

## Usage
/project:create-bug-issue <bug_type> <summary>

## Instructions

1. Read the bug detection file if it exists:
   - Look in /tmp/tarqeem-bug-*.json for recent bug info

2. Check for existing issues:
```bash
gh issue list --state open --search "$ARGUMENTS"
```

3. If no existing issue, create one using this template:
```bash
gh issue create \
  --title "[BUG] $ARGUMENTS" \
  --body "## Description
Bug automatically detected by Claude Code hooks.

## Detection Details
- Type: [bug type from detection]
- Branch: $(git branch --show-current)
- Commit: $(git rev-parse --short HEAD)
- Detected: $(date)

## Error Output
\`\`\`
[paste error output]
\`\`\`

## Files Affected
[list affected files]

## Suggested Fix
[suggestion if obvious]
" \
  --label "bug" \
  --label "auto-detected"
```

4. Report the created issue number to the user.
```

---

## Component 4: Updated settings.json

Add the bug detector hook to the existing configuration:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Bash(cargo *)",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/bug-detector.sh",
            "timeout": 10
          }
        ]
      }
    ]
  }
}
```

---

## Component 5: Permission Configuration

Allow gh commands in settings:

```json
{
  "permissions": {
    "allow": [
      "Skill",
      "Bash(gh:*)",
      "Bash(gh issue:*)",
      "Bash(gh pr:*)"
    ]
  }
}
```

---

## How It Works Together

### Flow 1: Test Failure Detection

```
User runs: cargo test
    │
    ▼
PostToolUse hook (bug-detector.sh) fires
    │
    ▼
Detects test failure (exit code != 0, FAILED in output)
    │
    ▼
Creates /tmp/tarqeem-bug-*.json with bug info
    │
    ▼
Outputs context: "BUG DETECTED - consider creating GitHub issue"
    │
    ▼
AI reads .claude/rules/bug-tracking.md
    │
    ▼
AI checks: gh issue list --state open --search "test failure"
    │
    ▼
If no existing issue → AI creates issue with gh issue create
    │
    ▼
Reports to user: "Created GitHub issue #XXX"
```

### Flow 2: AI Decision Making (Rule-Guided)

The rule file teaches the AI:
- **When** to create issues (test failures, regressions, clippy errors)
- **When NOT** to create issues (WIP, existing issues, intentional changes)
- **How** to format issues (structured template)
- **What labels** to use (bug, auto-detected, test-failure, etc.)

---

## Impact Analysis

### On AI Agent

| Aspect | Impact |
|--------|--------|
| **Awareness** | Knows when bugs are detected via hook output |
| **Decision Making** | Rules guide when to create vs skip issues |
| **Consistency** | Same issue format every time |
| **Autonomy** | Can create issues without user prompting |

### On Project

| Aspect | Impact |
|--------|--------|
| **Traceability** | All bugs get tracked in GitHub |
| **No Lost Bugs** | Automatic detection prevents forgetting |
| **Audit Trail** | Issues show detection context |
| **Team Visibility** | Everyone sees bugs via GitHub |

---

## Implementation Steps

1. Create `.claude/rules/bug-tracking.md` - AI instructions
2. Create `.claude/hooks/bug-detector.sh` - Detection hook
3. Create `.claude/commands/create-bug-issue.md` - Manual command
4. Update `.claude/settings.json` - Add hook and permissions
5. Test with intentional bug introduction
6. Verify issue creation works

---

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `.claude/rules/bug-tracking.md` | Create | AI instructions |
| `.claude/hooks/bug-detector.sh` | Create | Detection logic |
| `.claude/commands/create-bug-issue.md` | Create | Manual command |
| `.claude/settings.json` | Modify | Add hook + permissions |

---

## Sources

- [Claude Code Best Practices](https://www.anthropic.com/engineering/claude-code-best-practices)
- [GitHub Tasks Output Style](https://gist.github.com/johnlindquist/333aae98681b7cb7d6137abf72a2a218)
- [Claude Code Rules Directory](https://claudefa.st/blog/guide/mechanics/rules-directory)
- [GitHub CLI Integration](https://www.claudecode101.com/en/tutorial/configuration/github-cli)
