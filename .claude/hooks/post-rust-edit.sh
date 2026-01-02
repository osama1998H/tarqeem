#!/bin/bash
# Hook 2: Post-Rust-Edit - Auto-format and lint after Rust file modifications
set -euo pipefail

input=$(cat)
tool_name=$(echo "$input" | jq -r '.tool_name')
file_path=$(echo "$input" | jq -r '.tool_input.file_path // .tool_input.path // empty')

# Only process Rust files
if [[ ! "$file_path" =~ \.rs$ ]]; then
  exit 0
fi

# Only process successful writes/edits
success=$(echo "$input" | jq -r '.tool_output.success // "true"')
if [ "$success" != "true" ]; then
  exit 0
fi

cd "${CLAUDE_PROJECT_DIR:-/home/user/tarqeem}"

echo ""
echo "Post-edit checks for: $(basename "$file_path")"

# Run formatter on the specific file if it exists
if [ -f "$file_path" ]; then
  if command -v rustfmt &> /dev/null; then
    if ! rustfmt --check "$file_path" 2>/dev/null; then
      echo "  Formatting: Applying rustfmt..."
      rustfmt "$file_path" 2>/dev/null || true
      echo "  Formatting: Applied"
    else
      echo "  Formatting: OK"
    fi
  fi
fi

# Run quick clippy check on the modified file's module
# Extract module from path (e.g., src/lexer/lexer.rs -> lexer)
module=$(echo "$file_path" | sed -n 's|.*/src/\([^/]*\)/.*|\1|p')

if [ -n "$module" ]; then
  echo "  Clippy: Checking $module module..."
  clippy_output=$(cargo clippy -p tarqeem --lib 2>&1 | grep -E "warning:|error:" | head -3 || true)

  if [ -n "$clippy_output" ]; then
    echo "  Clippy: Found issues:"
    echo "$clippy_output" | sed 's/^/    /'
  else
    echo "  Clippy: Clean"
  fi
fi

# Special checks for error-related files
if [[ "$file_path" =~ (error|diagnostic|reporter) ]]; then
  # Check for bilingual message pattern
  if [ -f "$file_path" ]; then
    if grep -q "Diagnostic\|DiagnosticLevel\|message" "$file_path" 2>/dev/null; then
      echo "  Reminder: Error messages should be bilingual (Arabic + English)"
    fi
  fi
fi

# Special checks for lexer files (NFC normalization)
if [[ "$file_path" =~ /lexer/ ]]; then
  if [ -f "$file_path" ]; then
    if grep -q "identifier\|Identifier" "$file_path" 2>/dev/null; then
      if ! grep -q "nfc\|NFC\|normalize" "$file_path" 2>/dev/null; then
        echo "  Reminder: Arabic identifiers require NFC normalization"
      fi
    fi
  fi
fi

exit 0
