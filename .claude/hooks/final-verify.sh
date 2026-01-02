#!/bin/bash
# Hook 6: Final Verify - Runs verification when Claude finishes responding
set -euo pipefail

cd "${CLAUDE_PROJECT_DIR:-/home/user/tarqeem}"

# Check if any Rust files were modified in this session
modified_rs=$(git diff --name-only HEAD 2>/dev/null | grep "\.rs$" || true)
staged_rs=$(git diff --cached --name-only 2>/dev/null | grep "\.rs$" || true)

# Combine modified and staged
all_modified="$modified_rs$staged_rs"

if [ -z "$all_modified" ]; then
  # No Rust changes, skip verification
  exit 0
fi

echo ""
echo "=== Final Verification (Rust files modified) ==="

# Quick compile check
echo "Checking compilation..."
if cargo check --quiet 2>/dev/null; then
  echo "  Compilation: OK"
else
  echo "  Compilation: FAILED - please fix errors"
  # Show first few errors
  cargo check 2>&1 | grep -E "^error" | head -3 | sed 's/^/  /'
  exit 0
fi

# Check formatting
echo "Checking formatting..."
if cargo fmt --check --quiet 2>/dev/null; then
  echo "  Formatting: OK"
else
  echo "  Formatting: Issues found, applying fixes..."
  cargo fmt --quiet 2>/dev/null || true
  echo "  Formatting: Fixed"
fi

# Quick clippy check
echo "Checking clippy..."
clippy_warnings=$(cargo clippy --quiet --message-format=short 2>&1 | grep -c "^warning:" || echo "0")
if [ "$clippy_warnings" -eq 0 ]; then
  echo "  Clippy: Clean"
else
  echo "  Clippy: $clippy_warnings warning(s) found"
  cargo clippy --quiet --message-format=short 2>&1 | grep "^warning:" | head -3 | sed 's/^/  /'
fi

# Show uncommitted changes summary
echo ""
echo "Uncommitted changes:"
git diff --stat HEAD 2>/dev/null | tail -5 | sed 's/^/  /'

# Check if tests were run (look for recent test results)
if [ -f target/.last-test-result ]; then
  last_test_time=$(stat -c %Y target/.last-test-result 2>/dev/null || echo "0")
  current_time=$(date +%s)
  age=$((current_time - last_test_time))

  # If tests are more than 10 minutes old, suggest running them
  if [ "$age" -gt 600 ]; then
    echo ""
    echo "Note: Tests haven't been run recently. Consider: cargo test"
  fi
fi

exit 0
