#!/bin/bash
# Hook 1: SessionStart - Injects project status at session start
set -euo pipefail

cd "${CLAUDE_PROJECT_DIR:-/home/user/tarqeem}"

echo "=== Tarqeem Project Status ==="
echo ""

# Git branch and status
echo "Git Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown')"
echo ""

# Show uncommitted changes
changes=$(git status --short 2>/dev/null | wc -l)
if [ "$changes" -gt 0 ]; then
  echo "Uncommitted Changes: $changes file(s)"
  git status --short 2>/dev/null | head -5
  if [ "$changes" -gt 5 ]; then
    echo "  ... and $((changes - 5)) more"
  fi
else
  echo "Uncommitted Changes: None (clean working tree)"
fi
echo ""

# Last test results
echo "Test Status:"
if [ -f target/.last-test-result ]; then
  cat target/.last-test-result | sed 's/^/  /'
else
  echo "  No recent test results. Run: cargo test"
fi
echo ""

# Recent TODOs in modified files
echo "Recent TODOs:"
todo_count=0
for f in $(git diff --name-only HEAD~5 2>/dev/null | grep "\.rs$" | head -10); do
  if [ -f "$f" ]; then
    todos=$(grep -n "TODO\|FIXME" "$f" 2>/dev/null | head -2 || true)
    if [ -n "$todos" ]; then
      echo "  $f:"
      echo "$todos" | sed 's/^/    /'
      todo_count=$((todo_count + 1))
    fi
  fi
done
if [ "$todo_count" -eq 0 ]; then
  echo "  None in recently modified files"
fi
echo ""

# Quick health check
echo "Build Status:"
if [ -f target/release/tarqeem ]; then
  echo "  Release binary: Present"
else
  echo "  Release binary: Not built (run: cargo build --release)"
fi

if [ -f target/debug/tarqeem ]; then
  echo "  Debug binary: Present"
else
  echo "  Debug binary: Not built"
fi
echo ""

# Workflow reminder
echo "Workflow: Explore -> Plan -> Implement -> Verify"
echo "Verify: cargo fmt && cargo clippy && cargo test"
echo ""

exit 0
