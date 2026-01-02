# Claude Code Hooks Implementation Plan for Tarqeem

## Overview

This plan outlines 7 Claude Code hooks designed specifically for the Tarqeem Arabic programming language compiler. Each hook is analyzed for its impact on both the AI agent's behavior and the project's development workflow.

---

## Hook 1: SessionStart - Project Context Injection

### Description
Automatically injects essential project status when a Claude Code session begins.

### Implementation
**File**: `.claude/hooks/session-start.sh`

```bash
#!/bin/bash
set -euo pipefail

cd /home/user/tarqeem

echo "=== ترقيم Project Status ==="
echo ""

# Git status
echo "📌 Git Branch: $(git rev-parse --abbrev-ref HEAD)"
echo "📊 Git Status:"
git status --short | head -10
if [ $(git status --short | wc -l) -gt 10 ]; then
  echo "   ... and $(( $(git status --short | wc -l) - 10 )) more files"
fi

# Last test summary
echo ""
echo "🧪 Test Status:"
if [ -f target/.last-test-result ]; then
  cat target/.last-test-result
else
  echo "   No recent test results. Run: cargo test"
fi

# TODOs in recently modified files
echo ""
echo "📝 Recent TODOs:"
git diff --name-only HEAD~5 2>/dev/null | xargs grep -l "TODO\|FIXME" 2>/dev/null | head -5 | while read f; do
  grep -n "TODO\|FIXME" "$f" | head -2
done || echo "   None found"

# Reminder
echo ""
echo "⚠️  Workflow: Explore → Plan → Implement → Verify"
echo "   Run 'cargo fmt && cargo clippy && cargo test' before committing"

exit 0
```

### Impact on AI Agent

| Aspect | Impact |
|--------|--------|
| **Context Awareness** | Agent starts with full project state visibility - knows what's changed, what's broken |
| **Decision Making** | Can prioritize fixing failing tests or addressing TODOs before new work |
| **Workflow Compliance** | Constant reminder of mandatory workflow reduces skipped steps |
| **Reduced Tool Calls** | No need to manually run `git status` or check test results at session start |

### Impact on Project

| Aspect | Impact |
|--------|--------|
| **Code Quality** | Broken tests are immediately visible, reducing "works on my machine" issues |
| **Technical Debt** | TODOs surface regularly, preventing accumulation |
| **Git Hygiene** | Uncommitted changes are highlighted, encouraging atomic commits |
| **Onboarding** | New sessions always start with consistent context |

### Estimated Overhead
- **Execution Time**: ~2-3 seconds
- **Added Context**: ~500-800 tokens per session

---

## Hook 2: PostToolUse (Write|Edit on `*.rs`) - Auto-Format & Lint

### Description
Automatically runs formatting and linting after any Rust file modification.

### Implementation
**File**: `.claude/hooks/post-rust-edit.sh`

```bash
#!/bin/bash
set -euo pipefail

input=$(cat)
tool_name=$(echo "$input" | jq -r '.tool_name')
file_path=$(echo "$input" | jq -r '.tool_input.file_path // .tool_input.path // empty')

# Only process Rust files
if [[ ! "$file_path" =~ \.rs$ ]]; then
  exit 0
fi

cd /home/user/tarqeem

# Run formatter on the specific file
if command -v rustfmt &> /dev/null; then
  rustfmt "$file_path" 2>/dev/null || true
fi

# Run clippy (quick check)
echo "🔍 Running clippy check..."
clippy_output=$(cargo clippy --message-format=short 2>&1 | grep -E "^src/" | head -5 || true)

if [ -n "$clippy_output" ]; then
  echo "⚠️  Clippy warnings in modified code:"
  echo "$clippy_output"
fi

# Check for bilingual messages in error-related files
if [[ "$file_path" =~ (error|diagnostic|reporter) ]]; then
  if ! grep -q "message.*:" "$file_path" 2>/dev/null; then
    echo "⚠️  Reminder: Error messages should be bilingual (Arabic + English)"
  fi
fi

exit 0
```

### Impact on AI Agent

| Aspect | Impact |
|--------|--------|
| **Code Quality Feedback** | Immediate feedback on formatting/linting issues after each edit |
| **Self-Correction** | Agent sees warnings and can fix them in the next edit |
| **Reduced Iterations** | Catches simple issues before manual review |
| **Learning** | Agent learns project's code style through consistent feedback |

### Impact on Project

| Aspect | Impact |
|--------|--------|
| **Consistency** | All code automatically formatted to project standards |
| **CI Alignment** | Pre-commit checks less likely to fail (same rules applied earlier) |
| **Bilingual Enforcement** | Error messages maintain Arabic + English requirement |
| **Warning Prevention** | Clippy warnings caught immediately, not in CI |

### Estimated Overhead
- **Execution Time**: ~3-5 seconds per file edit
- **Trade-off**: Slightly slower edits, but fewer CI failures

---

## Hook 3: PostToolUse (Bash `cargo test*`) - Test Summary Reporter

### Description
Parses test output and provides a concise summary with trend analysis.

### Implementation
**File**: `.claude/hooks/test-summary.sh`

```bash
#!/bin/bash
set -euo pipefail

input=$(cat)
command=$(echo "$input" | jq -r '.tool_input.command // empty')

# Only process cargo test commands
if [[ ! "$command" =~ ^cargo\ test ]]; then
  exit 0
fi

tool_output=$(echo "$input" | jq -r '.tool_output.stdout // empty')

cd /home/user/tarqeem

# Parse test results
passed=$(echo "$tool_output" | grep -oP '\d+(?= passed)' | tail -1 || echo "0")
failed=$(echo "$tool_output" | grep -oP '\d+(?= failed)' | tail -1 || echo "0")
ignored=$(echo "$tool_output" | grep -oP '\d+(?= ignored)' | tail -1 || echo "0")
total=$((passed + failed))

# Save for session start hook
echo "Last run: $(date '+%Y-%m-%d %H:%M')" > target/.last-test-result
echo "Results: $passed passed, $failed failed, $ignored ignored" >> target/.last-test-result

# Compare with baseline
baseline_file="target/.test-baseline"
if [ -f "$baseline_file" ]; then
  baseline_total=$(cat "$baseline_file")
  if [ "$total" -lt "$baseline_total" ]; then
    echo "⚠️  Test count dropped: $total (was $baseline_total) - possible missing tests?"
  fi
fi

# Update baseline if tests pass
if [ "$failed" -eq 0 ]; then
  echo "$total" > "$baseline_file"
fi

# Summary
echo ""
echo "📊 Test Summary:"
echo "   ✅ Passed: $passed"
if [ "$failed" -gt 0 ]; then
  echo "   ❌ Failed: $failed"
fi
if [ "$ignored" -gt 0 ]; then
  echo "   ⏭️  Ignored: $ignored"
fi

exit 0
```

### Impact on AI Agent

| Aspect | Impact |
|--------|--------|
| **Result Visibility** | Clear pass/fail summary without parsing verbose output |
| **Regression Detection** | Immediately knows if test count dropped (deleted tests?) |
| **Progress Tracking** | Can track if changes improved or broke tests |
| **Decision Support** | Knows when to stop making changes vs continue fixing |

### Impact on Project

| Aspect | Impact |
|--------|--------|
| **Test Coverage Preservation** | Alerts when tests are accidentally removed |
| **Historical Tracking** | Test trends stored for analysis |
| **Quality Gates** | Clear signal of project health |
| **Documentation** | Implicit record of test evolution |

### Estimated Overhead
- **Execution Time**: ~1 second (parsing only)
- **Storage**: ~100 bytes per test run

---

## Hook 4: PreToolUse (Bash) - Git Safety & Command Validation

### Description
Prevents dangerous git operations and validates commands before execution.

### Implementation
**File**: `.claude/hooks/pre-bash-safety.sh`

```bash
#!/bin/bash
set -euo pipefail

input=$(cat)
command=$(echo "$input" | jq -r '.tool_input.command // empty')

# === Git Safety Rules (from CLAUDE.md) ===

# Block force push to main/develop
if [[ "$command" =~ git\ push.*--force ]] || [[ "$command" =~ git\ push.*-f ]]; then
  if [[ "$command" =~ (main|master|develop) ]]; then
    echo '{"decision": "deny", "reason": "🚫 BLOCKED: Force push to protected branch (main/develop) is forbidden per CLAUDE.md"}'
    exit 0
  fi
fi

# Block hard reset without explicit user request
if [[ "$command" =~ git\ reset\ --hard ]]; then
  echo '{"decision": "ask", "reason": "⚠️ git reset --hard is destructive. Confirm this is intentional?"}'
  exit 0
fi

# Block --no-verify flags
if [[ "$command" =~ --no-verify ]] || [[ "$command" =~ --no-gpg-sign ]]; then
  echo '{"decision": "deny", "reason": "🚫 BLOCKED: Skipping hooks (--no-verify) is forbidden per CLAUDE.md"}'
  exit 0
fi

# Warn about direct commits to protected branches
if [[ "$command" =~ git\ commit ]] && [[ ! "$command" =~ --amend ]]; then
  current_branch=$(cd /home/user/tarqeem && git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
  if [[ "$current_branch" =~ ^(main|master|develop)$ ]]; then
    echo '{"decision": "ask", "reason": "⚠️ Committing directly to '"$current_branch"'. Create a feature branch instead?"}'
    exit 0
  fi
fi

# Block git config changes
if [[ "$command" =~ git\ config ]]; then
  echo '{"decision": "deny", "reason": "🚫 BLOCKED: Git config changes are forbidden per CLAUDE.md"}'
  exit 0
fi

# Block interactive git commands
if [[ "$command" =~ git.*\ -i ]] || [[ "$command" =~ git\ rebase\ -i ]] || [[ "$command" =~ git\ add\ -i ]]; then
  echo '{"decision": "deny", "reason": "🚫 BLOCKED: Interactive git commands (-i) are not supported"}'
  exit 0
fi

# Allow command
exit 0
```

### Impact on AI Agent

| Aspect | Impact |
|--------|--------|
| **Guardrails** | Prevents accidental destructive operations |
| **Policy Enforcement** | CLAUDE.md rules automatically enforced, no reliance on memory |
| **Reduced Risk** | Agent cannot accidentally force push or destroy history |
| **Workflow Guidance** | Prompts for feature branches instead of direct main commits |

### Impact on Project

| Aspect | Impact |
|--------|--------|
| **Repository Safety** | Protected branches stay protected |
| **Audit Trail** | Hooks always run, history preserved |
| **Team Collaboration** | No surprise force pushes affecting others |
| **CI/CD Integrity** | Pre-commit hooks can't be bypassed |

### Estimated Overhead
- **Execution Time**: <100ms (regex matching only)
- **False Positives**: Rare, but may need to adjust patterns

---

## Hook 5: PreToolUse (Write|Edit) - Architecture Layer Validation

### Description
Enforces the strict layer dependency rules: `Lexer → Parser → Semantic → IR → Codegen`

### Implementation
**File**: `.claude/hooks/architecture-check.sh`

```bash
#!/bin/bash
set -euo pipefail

input=$(cat)
file_path=$(echo "$input" | jq -r '.tool_input.file_path // .tool_input.path // empty')
content=$(echo "$input" | jq -r '.tool_input.content // .tool_input.new_string // empty')

# Only check Rust source files
if [[ ! "$file_path" =~ ^.*src/.*\.rs$ ]]; then
  exit 0
fi

# Determine which layer this file belongs to
layer=""
case "$file_path" in
  *"/lexer/"*)    layer="lexer" ;;
  *"/parser/"*)   layer="parser" ;;
  *"/semantic/"*) layer="semantic" ;;
  *"/ir/"*)       layer="ir" ;;
  *"/codegen/"*)  layer="codegen" ;;
  *"/interpreter/"*) layer="interpreter" ;;
  *)              exit 0 ;;  # Not a layer file
esac

# Define forbidden imports per layer (reverse dependencies)
case "$layer" in
  "lexer")
    # Lexer cannot import from any other compiler layer
    if echo "$content" | grep -qE "use (crate::|super::).*(parser|semantic|ir|codegen)"; then
      echo '{"decision": "deny", "reason": "🏗️ ARCHITECTURE VIOLATION: Lexer cannot depend on parser/semantic/ir/codegen layers"}'
      exit 0
    fi
    ;;
  "parser")
    # Parser can only import from lexer
    if echo "$content" | grep -qE "use (crate::|super::).*(semantic|ir|codegen)"; then
      echo '{"decision": "deny", "reason": "🏗️ ARCHITECTURE VIOLATION: Parser cannot depend on semantic/ir/codegen layers"}'
      exit 0
    fi
    ;;
  "semantic")
    # Semantic can import from lexer, parser
    if echo "$content" | grep -qE "use (crate::|super::).*(ir|codegen)"; then
      echo '{"decision": "deny", "reason": "🏗️ ARCHITECTURE VIOLATION: Semantic cannot depend on ir/codegen layers"}'
      exit 0
    fi
    ;;
  "ir")
    # IR can import from lexer, parser, semantic
    if echo "$content" | grep -qE "use (crate::|super::).*codegen"; then
      echo '{"decision": "deny", "reason": "🏗️ ARCHITECTURE VIOLATION: IR cannot depend on codegen layer"}'
      exit 0
    fi
    ;;
esac

exit 0
```

### Impact on AI Agent

| Aspect | Impact |
|--------|--------|
| **Architecture Awareness** | Cannot accidentally create circular dependencies |
| **Immediate Feedback** | Learns project structure through enforcement |
| **Reduced Refactoring** | Bad imports caught before they're written |
| **Design Guidance** | Understands module boundaries implicitly |

### Impact on Project

| Aspect | Impact |
|--------|--------|
| **Maintainability** | Layer boundaries preserved, clean architecture |
| **Compile Times** | No unnecessary recompilation from circular deps |
| **Code Review** | Fewer architecture discussions in PRs |
| **Long-term Health** | Prevents gradual architecture erosion |

### Estimated Overhead
- **Execution Time**: ~100-200ms (grep on content)
- **False Positives**: Possible with comments containing layer names (rare)

---

## Hook 6: Stop - Final Verification Check

### Description
Runs verification checks when Claude finishes responding to ensure code integrity.

### Implementation
**File**: `.claude/hooks/final-verify.sh`

```bash
#!/bin/bash
set -euo pipefail

cd /home/user/tarqeem

# Check if any Rust files were modified in this session
modified_rs=$(git diff --name-only HEAD 2>/dev/null | grep "\.rs$" || true)

if [ -z "$modified_rs" ]; then
  # No Rust changes, skip verification
  exit 0
fi

echo ""
echo "🔍 Final Verification (Rust files modified)..."

# Quick compile check
echo "   Checking compilation..."
if ! cargo check --quiet 2>/dev/null; then
  echo "   ❌ Compilation failed! Please fix errors before continuing."
  exit 0
fi
echo "   ✅ Compilation OK"

# Check formatting
echo "   Checking formatting..."
if ! cargo fmt --check --quiet 2>/dev/null; then
  echo "   ⚠️  Formatting issues found. Running cargo fmt..."
  cargo fmt --quiet
  echo "   ✅ Formatted"
fi

# Quick clippy check
echo "   Checking clippy..."
clippy_warnings=$(cargo clippy --quiet --message-format=short 2>&1 | grep -c "warning:" || echo "0")
if [ "$clippy_warnings" -gt 0 ]; then
  echo "   ⚠️  $clippy_warnings clippy warning(s) found"
else
  echo "   ✅ Clippy clean"
fi

# Uncommitted changes summary
echo ""
echo "📋 Uncommitted changes:"
git diff --stat HEAD | tail -5

exit 0
```

### Impact on AI Agent

| Aspect | Impact |
|--------|--------|
| **Quality Assurance** | Every response verified before user sees final state |
| **Self-Correction Loop** | Can see issues and offer to fix in next turn |
| **Confidence** | Knows code compiles before claiming "done" |
| **Completeness** | Summarizes what changed for user review |

### Impact on Project

| Aspect | Impact |
|--------|--------|
| **No Broken Commits** | Code always compiles at session end |
| **Consistent State** | Formatting applied automatically |
| **Visibility** | Clear summary of session's changes |
| **Reduced Churn** | Fewer "fix formatting" follow-up commits |

### Estimated Overhead
- **Execution Time**: 5-15 seconds (cargo check + clippy)
- **Trade-off**: Slight delay at response end, but catches issues early

---

## Hook 7: UserPromptSubmit - Context Enhancement

### Description
Automatically injects relevant context based on keywords in user's prompt.

### Implementation
**File**: `.claude/hooks/context-enhance.sh`

```bash
#!/bin/bash
set -euo pipefail

input=$(cat)
user_prompt=$(echo "$input" | jq -r '.user_prompt // empty' | tr '[:upper:]' '[:lower:]')

cd /home/user/tarqeem

# Detect module mentions and inject relevant context
context=""

if [[ "$user_prompt" =~ (lexer|تجزئة|token|رمز) ]]; then
  context+="📚 Lexer Context: src/lexer/ (5 files, Arabic keyword mapping in keywords.rs)\n"
  context+="   Key invariant: NFC normalization required for identifiers\n"
fi

if [[ "$user_prompt" =~ (parser|تحليل|ast|شجرة) ]]; then
  context+="📚 Parser Context: src/parser/ (8 files, recursive descent + Pratt parsing)\n"
  context+="   Key invariant: Every token must have accurate source location\n"
fi

if [[ "$user_prompt" =~ (semantic|دلالي|type|نوع|scope|نطاق) ]]; then
  context+="📚 Semantic Context: src/semantic/ (12 files, type checking + generics)\n"
  context+="   Key invariant: Bilingual error messages required\n"
fi

if [[ "$user_prompt" =~ (ir|intermediate|وسيط) ]]; then
  context+="📚 IR Context: src/ir/ (14 files, SSA form, 5 optimization passes)\n"
fi

if [[ "$user_prompt" =~ (codegen|llvm|توليد) ]]; then
  context+="📚 Codegen Context: src/codegen/ (7 files, LLVM backend)\n"
  context+="   Targets: x86_64, aarch64, WASM\n"
fi

if [[ "$user_prompt" =~ (error|خطأ|diagnostic) ]]; then
  context+="📚 Error Context: src/error/ (6 files, 9 error categories)\n"
  context+="   Format: [حرف][٤ أرقام] e.g., د٠٣٠١\n"
  context+="   Key invariant: All messages must be bilingual\n"
fi

if [[ "$user_prompt" =~ (test|اختبار) ]]; then
  context+="📚 Testing Context: 921+ tests across unit/integration\n"
  context+="   Run: cargo test (all), cargo test --lib (unit only)\n"
  context+="   Locations: inline #[cfg(test)], tests/ directory\n"
fi

if [ -n "$context" ]; then
  echo -e "$context"
fi

exit 0
```

### Impact on AI Agent

| Aspect | Impact |
|--------|--------|
| **Automatic Context** | Relevant module info injected without manual lookup |
| **Faster Responses** | Less time spent exploring for basic facts |
| **Consistency** | Same context provided every time a module is mentioned |
| **Invariant Awareness** | Key rules surfaced at point of need |

### Impact on Project

| Aspect | Impact |
|--------|--------|
| **Knowledge Preservation** | Institutional knowledge encoded in hooks |
| **Onboarding** | New AI sessions have instant context |
| **Reduced Errors** | Invariants shown before code is written |
| **Efficiency** | Less back-and-forth for basic questions |

### Estimated Overhead
- **Execution Time**: <100ms (keyword matching)
- **Added Context**: 100-300 tokens per prompt (only when relevant)

---

## Settings Configuration

**File**: `.claude/settings.json`

```json
{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "permissions": {
    "allow": ["Skill"]
  },
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "/home/user/tarqeem/.claude/hooks/session-start.sh",
            "timeout": 10
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "/home/user/tarqeem/.claude/hooks/context-enhance.sh",
            "timeout": 5
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "/home/user/tarqeem/.claude/hooks/pre-bash-safety.sh",
            "timeout": 5
          }
        ]
      },
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "/home/user/tarqeem/.claude/hooks/architecture-check.sh",
            "timeout": 5
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "/home/user/tarqeem/.claude/hooks/post-rust-edit.sh",
            "timeout": 30
          }
        ]
      },
      {
        "matcher": "Bash(cargo test*)",
        "hooks": [
          {
            "type": "command",
            "command": "/home/user/tarqeem/.claude/hooks/test-summary.sh",
            "timeout": 10
          }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "/home/user/tarqeem/.claude/hooks/final-verify.sh",
            "timeout": 60
          }
        ]
      }
    ]
  }
}
```

---

## Impact Summary Table

| Hook | Agent Benefit | Project Benefit | Overhead |
|------|---------------|-----------------|----------|
| **SessionStart** | Full context at start | Visibility into project state | 2-3s |
| **Post-Rust-Edit** | Immediate quality feedback | Consistent formatting | 3-5s/edit |
| **Test Summary** | Clear pass/fail visibility | Test coverage protection | 1s |
| **Pre-Bash Safety** | Prevents destructive actions | Repository safety | <100ms |
| **Architecture Check** | Learns layer boundaries | Maintains clean architecture | 100-200ms |
| **Final Verify** | Confirms code compiles | No broken commits | 5-15s |
| **Context Enhance** | Relevant context auto-injected | Knowledge preservation | <100ms |

---

## Implementation Order

1. **Phase 1 (Critical Safety)**: Pre-Bash Safety + Architecture Check
2. **Phase 2 (Quality)**: Post-Rust-Edit + Final Verify
3. **Phase 3 (Context)**: SessionStart + Context Enhance
4. **Phase 4 (Insights)**: Test Summary

---

## Rollback Plan

If any hook causes issues:
1. Remove from `.claude/settings.json`
2. Delete script from `.claude/hooks/`
3. Restart Claude Code session

Each hook is independent - disabling one doesn't affect others.
