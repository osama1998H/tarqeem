#!/bin/bash
# Hook: Bug Detector - Detects test failures, clippy errors, compilation errors
# Provides context to AI agent for creating GitHub issues
set -euo pipefail

input=$(cat)
command=$(echo "$input" | jq -r '.tool_input.command // empty')
stdout=$(echo "$input" | jq -r '.tool_output.stdout // empty')
stderr=$(echo "$input" | jq -r '.tool_output.stderr // empty')
exit_code=$(echo "$input" | jq -r '.tool_output.exit_code // 0')

# Only process cargo commands
if [[ ! "$command" =~ ^cargo ]]; then
  exit 0
fi

# Only process failed commands (exit code != 0)
if [ "$exit_code" == "0" ]; then
  exit 0
fi

cd "${CLAUDE_PROJECT_DIR:-/home/user/tarqeem}"

# Combine output for analysis
full_output="$stdout$stderr"

# Initialize bug detection
bug_detected=false
bug_type=""
bug_summary=""
error_output=""

# Detect test failures
if [[ "$command" =~ cargo\ test ]]; then
  failed_tests=$(echo "$full_output" | grep -E "^test .* FAILED$" | head -5 || true)
  if [ -n "$failed_tests" ]; then
    bug_detected=true
    bug_type="test-failure"
    bug_summary="Test failure detected in cargo test"
    error_output="$failed_tests"
  fi
fi

# Detect clippy errors
if [[ "$command" =~ cargo\ clippy ]]; then
  clippy_errors=$(echo "$full_output" | grep -E "^error\[E" | head -5 || true)
  if [ -n "$clippy_errors" ]; then
    bug_detected=true
    bug_type="clippy"
    bug_summary="Clippy error detected"
    error_output="$clippy_errors"
  fi
fi

# Detect compilation errors
if [[ "$command" == "cargo build"* ]] || [[ "$command" == "cargo check"* ]]; then
  compile_errors=$(echo "$full_output" | grep -E "^error\[E" | head -5 || true)
  if [ -n "$compile_errors" ]; then
    bug_detected=true
    bug_type="compile-error"
    bug_summary="Compilation error detected"
    error_output="$compile_errors"
  fi
fi

# If bug detected, provide context to AI
if [ "$bug_detected" = true ]; then
  # Get git context
  branch=$(git branch --show-current 2>/dev/null || echo 'unknown')
  commit=$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')
  timestamp=$(date -Iseconds)

  # Save bug info for reference
  bug_file="/tmp/tarqeem-bug-$(date +%Y%m%d-%H%M%S).json"
  cat > "$bug_file" << EOFBUG
{
  "type": "$bug_type",
  "summary": "$bug_summary",
  "command": "$command",
  "branch": "$branch",
  "commit": "$commit",
  "timestamp": "$timestamp",
  "error_preview": $(echo "$error_output" | head -10 | jq -Rs .)
}
EOFBUG

  # Output context for AI
  echo ""
  echo "========================================"
  echo "          BUG DETECTED"
  echo "========================================"
  echo "Type:    $bug_type"
  echo "Summary: $bug_summary"
  echo "Branch:  $branch"
  echo "Commit:  $commit"
  echo "Command: $command"
  echo ""
  echo "Error Preview:"
  echo "$error_output" | head -10 | sed 's/^/  /'
  echo ""
  echo "Bug details saved to: $bug_file"
  echo ""
  echo "ACTION REQUIRED (per .claude/rules/bug-tracking.md):"
  echo "  1. Check existing issues: gh issue list --state open --search \"$bug_type\""
  echo "  2. If new bug, create issue with appropriate labels"
  echo "========================================"
fi

exit 0
