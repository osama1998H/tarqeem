#!/bin/bash
# Hook 5: Architecture Check - Enforces layer dependencies
# Layers: Lexer -> Parser -> Semantic -> IR -> Codegen (no reverse deps)
set -euo pipefail

input=$(cat)
file_path=$(echo "$input" | jq -r '.tool_input.file_path // .tool_input.path // empty')
content=$(echo "$input" | jq -r '.tool_input.content // .tool_input.new_string // empty')

# Only check Rust source files in src/
if [[ ! "$file_path" =~ ^.*/src/.*\.rs$ ]]; then
  exit 0
fi

# Skip if no content to check (e.g., read operations)
if [ -z "$content" ]; then
  exit 0
fi

# Determine which layer this file belongs to
layer=""
case "$file_path" in
  *"/lexer/"*)       layer="lexer" ;;
  *"/parser/"*)      layer="parser" ;;
  *"/semantic/"*)    layer="semantic" ;;
  *"/ir/"*)          layer="ir" ;;
  *"/codegen/"*)     layer="codegen" ;;
  *"/interpreter/"*) layer="interpreter" ;;
  *"/jit/"*)         layer="jit" ;;
  *)                 exit 0 ;;  # Not a compiler layer file
esac

# Define forbidden imports per layer (reverse dependencies are not allowed)
# Layer order: Lexer -> Parser -> Semantic -> IR -> Codegen
case "$layer" in
  "lexer")
    # Lexer is the first layer - cannot import from any other compiler layer
    if echo "$content" | grep -qE "use (crate::|super::).*(parser|semantic|ir|codegen|interpreter|jit)::"; then
      cat << 'EOF'
{"decision": "deny", "reason": "ARCHITECTURE VIOLATION: Lexer cannot depend on parser/semantic/ir/codegen layers. Layer order: Lexer -> Parser -> Semantic -> IR -> Codegen"}
EOF
      exit 0
    fi
    ;;
  "parser")
    # Parser can only import from lexer
    if echo "$content" | grep -qE "use (crate::|super::).*(semantic|ir|codegen)::"; then
      cat << 'EOF'
{"decision": "deny", "reason": "ARCHITECTURE VIOLATION: Parser cannot depend on semantic/ir/codegen layers. Layer order: Lexer -> Parser -> Semantic -> IR -> Codegen"}
EOF
      exit 0
    fi
    ;;
  "semantic")
    # Semantic can import from lexer and parser
    if echo "$content" | grep -qE "use (crate::|super::).*(ir|codegen)::"; then
      cat << 'EOF'
{"decision": "deny", "reason": "ARCHITECTURE VIOLATION: Semantic cannot depend on ir/codegen layers. Layer order: Lexer -> Parser -> Semantic -> IR -> Codegen"}
EOF
      exit 0
    fi
    ;;
  "ir")
    # IR can import from lexer, parser, semantic
    if echo "$content" | grep -qE "use (crate::|super::).*codegen::"; then
      cat << 'EOF'
{"decision": "deny", "reason": "ARCHITECTURE VIOLATION: IR cannot depend on codegen layer. Layer order: Lexer -> Parser -> Semantic -> IR -> Codegen"}
EOF
      exit 0
    fi
    ;;
  "interpreter")
    # Interpreter can use lexer, parser, semantic, ir (but not codegen - it's an alternative path)
    if echo "$content" | grep -qE "use (crate::|super::).*codegen::"; then
      cat << 'EOF'
{"decision": "ask", "reason": "Interpreter importing from codegen is unusual - interpreter is typically an alternative to codegen. Is this intentional?"}
EOF
      exit 0
    fi
    ;;
esac

# Check for potential circular module dependencies within same layer
if echo "$content" | grep -qE "use super::super::"; then
  cat << 'EOF'
{"decision": "ask", "reason": "Deep parent references (super::super::) may indicate architectural issues. Consider restructuring imports."}
EOF
  exit 0
fi

# Allow the write/edit
exit 0
