#!/bin/bash
# Hook 3: Test Summary - Parses and summarizes cargo test output
set -euo pipefail

input=$(cat)
command=$(echo "$input" | jq -r '.tool_input.command // empty')

# Only process cargo test commands
if [[ ! "$command" =~ ^cargo\ test ]]; then
  exit 0
fi

# Get test output from tool result
tool_output=$(echo "$input" | jq -r '.tool_output.stdout // empty')

cd "${CLAUDE_PROJECT_DIR:-/home/user/tarqeem}"

# Parse test results from output
# Cargo test output format: "test result: ok. X passed; Y failed; Z ignored"
result_line=$(echo "$tool_output" | grep -E "^test result:" | tail -1 || true)

if [ -z "$result_line" ]; then
  # Try alternative format for individual test runs
  passed=$(echo "$tool_output" | grep -cE "^test .* ok$" || echo "0")
  failed=$(echo "$tool_output" | grep -cE "^test .* FAILED$" || echo "0")
  ignored=$(echo "$tool_output" | grep -cE "^test .* ignored$" || echo "0")
else
  # Parse from result line
  passed=$(echo "$result_line" | grep -oP '\d+(?= passed)' || echo "0")
  failed=$(echo "$result_line" | grep -oP '\d+(?= failed)' || echo "0")
  ignored=$(echo "$result_line" | grep -oP '\d+(?= ignored)' || echo "0")
fi

# Ensure we have numbers
passed=${passed:-0}
failed=${failed:-0}
ignored=${ignored:-0}
total=$((passed + failed))

# Save results for session-start hook
mkdir -p target
cat > target/.last-test-result << EOF
Last run: $(date '+%Y-%m-%d %H:%M')
Results: $passed passed, $failed failed, $ignored ignored
Total: $total tests
EOF

# Compare with baseline to detect test count changes
baseline_file="target/.test-baseline"
if [ -f "$baseline_file" ]; then
  baseline_total=$(cat "$baseline_file" 2>/dev/null || echo "0")
  if [ "$total" -lt "$baseline_total" ]; then
    echo ""
    echo "Warning: Test count dropped: $total (was $baseline_total)"
    echo "  Possible causes: tests removed, compilation errors, or filter applied"
  elif [ "$total" -gt "$baseline_total" ]; then
    echo ""
    echo "Note: Test count increased: $total (was $baseline_total) - new tests added"
  fi
fi

# Update baseline only if all tests pass and no filter was applied
if [ "$failed" -eq 0 ] && [[ ! "$command" =~ :: ]]; then
  echo "$total" > "$baseline_file"
fi

# Summary output
echo ""
echo "=== Test Summary ==="
echo "  Passed:  $passed"
if [ "$failed" -gt 0 ]; then
  echo "  Failed:  $failed"
  # Extract failed test names
  echo ""
  echo "  Failed tests:"
  echo "$tool_output" | grep -E "^test .* FAILED$" | head -5 | sed 's/^/    /'
fi
if [ "$ignored" -gt 0 ]; then
  echo "  Ignored: $ignored"
fi
echo "  Total:   $total"

# Performance note
duration=$(echo "$tool_output" | grep -oP 'finished in \K[0-9.]+s' | tail -1 || true)
if [ -n "$duration" ]; then
  echo "  Duration: $duration"
fi

exit 0
