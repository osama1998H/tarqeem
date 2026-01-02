#!/bin/bash
# Hook 4: Pre-Bash Safety - Prevents dangerous git operations
# Enforces CLAUDE.md git safety rules
set -euo pipefail

input=$(cat)
command=$(echo "$input" | jq -r '.tool_input.command // empty')

# Exit early if no command
if [ -z "$command" ]; then
  exit 0
fi

# === Git Safety Rules (from CLAUDE.md) ===

# Block force push to main/develop/master
if [[ "$command" == *"git push"*"--force"* ]] || [[ "$command" == *"git push"*"-f "* ]]; then
  if [[ "$command" == *"main"* ]] || [[ "$command" == *"master"* ]] || [[ "$command" == *"develop"* ]]; then
    cat << 'EOF'
{"decision": "deny", "reason": "BLOCKED: Force push to protected branch (main/develop) is forbidden per CLAUDE.md"}
EOF
    exit 0
  fi
fi

# Block hard reset without explicit user request
if [[ "$command" == *"git reset --hard"* ]]; then
  cat << 'EOF'
{"decision": "ask", "reason": "git reset --hard is destructive and may lose uncommitted work. Confirm this is intentional?"}
EOF
  exit 0
fi

# Block --no-verify flags (skipping pre-commit hooks)
if [[ "$command" == *"--no-verify"* ]]; then
  cat << 'EOF'
{"decision": "deny", "reason": "BLOCKED: Skipping hooks (--no-verify) is forbidden per CLAUDE.md"}
EOF
  exit 0
fi

# Block --no-gpg-sign (per CLAUDE.md)
if [[ "$command" == *"--no-gpg-sign"* ]]; then
  cat << 'EOF'
{"decision": "deny", "reason": "BLOCKED: Skipping GPG signing (--no-gpg-sign) is forbidden per CLAUDE.md"}
EOF
  exit 0
fi

# Warn about direct commits to protected branches
if [[ "$command" == *"git commit"* ]] && [[ "$command" != *"--amend"* ]]; then
  current_branch=$(cd "${CLAUDE_PROJECT_DIR:-/home/user/tarqeem}" && git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
  if [[ "$current_branch" == "main" ]] || [[ "$current_branch" == "master" ]] || [[ "$current_branch" == "develop" ]]; then
    cat << EOF
{"decision": "ask", "reason": "Committing directly to '$current_branch' branch. Per CLAUDE.md, consider creating a feature branch first. Proceed anyway?"}
EOF
    exit 0
  fi
fi

# Block git config changes (per CLAUDE.md: NEVER update git config)
if [[ "$command" == *"git config"* ]]; then
  cat << 'EOF'
{"decision": "deny", "reason": "BLOCKED: Git config changes are forbidden per CLAUDE.md"}
EOF
  exit 0
fi

# Block interactive git commands (not supported in non-interactive mode)
if [[ "$command" == *"git rebase -i"* ]] || [[ "$command" == *"git add -i"* ]] || [[ "$command" == *"git add --interactive"* ]]; then
  cat << 'EOF'
{"decision": "deny", "reason": "BLOCKED: Interactive git commands (-i) are not supported in Claude Code"}
EOF
  exit 0
fi

# Block amend without checking authorship first (CLAUDE.md requirement)
if [[ "$command" == *"git commit"*"--amend"* ]]; then
  # Check if we've verified authorship recently
  last_log=$(cd "${CLAUDE_PROJECT_DIR:-/home/user/tarqeem}" && git log -1 --format='%an <%ae>' 2>/dev/null || echo "")
  if [ -z "$last_log" ]; then
    cat << 'EOF'
{"decision": "ask", "reason": "Before amending, verify this is your commit. Run 'git log -1 --format=%an %ae' first. Proceed?"}
EOF
    exit 0
  fi
fi

# Allow the command
exit 0
