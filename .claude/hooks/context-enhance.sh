#!/bin/bash
# Hook 7: Context Enhance - Injects module context based on user prompt keywords
set -euo pipefail

input=$(cat)
user_prompt=$(echo "$input" | jq -r '.user_prompt // empty' | tr '[:upper:]' '[:lower:]')

# Exit if no prompt
if [ -z "$user_prompt" ]; then
  exit 0
fi

cd "${CLAUDE_PROJECT_DIR:-/home/user/tarqeem}"

context=""

# Detect module mentions and inject relevant context
# Lexer context
if [[ "$user_prompt" =~ (lexer|token|keyword|رمز|كلمة|تجزئة) ]]; then
  context+="Lexer Context (src/lexer/, 5 files):\n"
  context+="  - Arabic keywords in keywords.rs (phf perfect hash map)\n"
  context+="  - Invariant: NFC normalization required for identifiers\n"
  context+="  - Invariant: Every token must have accurate source location (Span)\n"
  context+="\n"
fi

# Parser context
if [[ "$user_prompt" =~ (parser|ast|parse|syntax|شجرة|تحليل|نحوي) ]]; then
  context+="Parser Context (src/parser/, 8 files):\n"
  context+="  - Recursive descent + Pratt parsing for expressions\n"
  context+="  - 11 precedence levels in precedence.rs\n"
  context+="  - AST nodes defined in ast.rs\n"
  context+="\n"
fi

# Semantic context
if [[ "$user_prompt" =~ (semantic|type|scope|symbol|نوع|نطاق|دلالي) ]]; then
  context+="Semantic Context (src/semantic/, 12 files):\n"
  context+="  - Type checking with generics support\n"
  context+="  - Scope management in scope.rs (839 lines)\n"
  context+="  - Class hierarchy in class_resolver.rs\n"
  context+="  - Invariant: Bilingual error messages required\n"
  context+="\n"
fi

# IR context
if [[ "$user_prompt" =~ (ir|intermediate|ssa|optimize|وسيط|تحسين) ]]; then
  context+="IR Context (src/ir/, 14 files):\n"
  context+="  - Three-address code, SSA form\n"
  context+="  - 5 optimization passes: const_fold, dce, cse, inline, loop_opt\n"
  context+="  - Builder in builder/mod.rs (933 lines)\n"
  context+="\n"
fi

# Codegen context
if [[ "$user_prompt" =~ (codegen|llvm|generate|compile|توليد|ترجم) ]]; then
  context+="Codegen Context (src/codegen/, 7 files):\n"
  context+="  - LLVM backend via inkwell\n"
  context+="  - Targets: x86_64, aarch64, WASM\n"
  context+="  - Main codegen in llvm/codegen.rs (600+ lines)\n"
  context+="\n"
fi

# Error handling context
if [[ "$user_prompt" =~ (error|diagnostic|message|خطأ|رسالة) ]]; then
  context+="Error Context (src/error/, 6 files):\n"
  context+="  - 9 error categories: ق،ب،د،ن،ص،و،ت،ح،م\n"
  context+="  - Format: [letter][4 digits] e.g., د٠٣٠١\n"
  context+="  - Invariant: All messages MUST be bilingual (Arabic + English)\n"
  context+="\n"
fi

# LSP context
if [[ "$user_prompt" =~ (lsp|language.server|completion|hover|definition) ]]; then
  context+="LSP Context (src/lsp/, 22 files):\n"
  context+="  - 13+ handlers in handlers/ directory\n"
  context+="  - Uses tower-lsp + tokio async runtime\n"
  context+="\n"
fi

# Testing context
if [[ "$user_prompt" =~ (test|اختبار|spec|verify) ]]; then
  context+="Testing Context:\n"
  context+="  - 921+ tests across unit and integration\n"
  context+="  - Unit tests: inline #[cfg(test)] mod tests\n"
  context+="  - Integration tests: tests/ directory\n"
  context+="  - Run all: cargo test\n"
  context+="  - Run specific: cargo test lexer\n"
  context+="\n"
fi

# CLI context
if [[ "$user_prompt" =~ (cli|command|argument|run|compile) ]]; then
  context+="CLI Context (src/cli/, 17 files):\n"
  context+="  - Clap derive-based argument parsing\n"
  context+="  - Commands: compile, run, check, repl, fmt, debug, doc, pkg\n"
  context+="  - Package manager subcommands in pm/ directory\n"
  context+="\n"
fi

# Architecture reminder for cross-layer work
if [[ "$user_prompt" =~ (refactor|move|restructure|architecture) ]]; then
  context+="Architecture Reminder:\n"
  context+="  - Layer order: Lexer -> Parser -> Semantic -> IR -> Codegen\n"
  context+="  - NO reverse dependencies allowed\n"
  context+="  - Interpreter is alternative to Codegen (parallel path)\n"
  context+="\n"
fi

# Output context if any was generated
if [ -n "$context" ]; then
  echo -e "$context"
fi

exit 0
