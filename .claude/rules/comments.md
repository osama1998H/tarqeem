---
paths: "{src,runtime-rs/src,benches}/**/*.rs"
---

# Code Comments Philosophy

This file defines when and how to use comments in the Tarqeem codebase.

## The Core Principle

**Comments should explain WHY, not WHAT.**

Just as the Arabic philosophy states "الوصف لا الترجمة" (description, not translation) - comments should not translate code into English. The code itself is the "what"; comments add context the code cannot express.

## When NOT to Comment

### 1. Self-Evident Code

```rust
// BAD: Restates what the code does
// Check for newlines
if c == '\n' {
    return self.make_token(TokenKind::Newline);
}

// GOOD: No comment needed - the code is clear
if c == '\n' {
    return self.make_token(TokenKind::Newline);
}
```

### 2. Self-Documenting Names

```rust
// BAD: Function name already says this
/// Checks if the character is a digit
fn is_digit(&self, c: char) -> bool { }

// GOOD: No doc needed - name is descriptive
fn is_digit(&self, c: char) -> bool { }
```

### 3. Obvious Enum Variants

```rust
// BAD: Just restates the type name
pub enum IrType {
    /// Boolean (1-bit integer)
    Bool,
    /// 64-bit signed integer
    Int,
    /// 64-bit floating point
    Float,
}

// GOOD: Only document if non-obvious
pub enum IrType {
    Bool,
    Int,
    Float,
    /// Pointer to heap-allocated UTF-8 string with length prefix
    String,
}
```

### 4. ASCII Art Section Headers

```rust
// BAD: Visual clutter
// ============ Helper Methods ============

// GOOD: Use well-named impls or modules for organization
impl Lexer {
    // Helper methods naturally grouped here
}
```

## When TO Comment

### 1. Design Decisions

```rust
// GOOD: Explains WHY this choice was made
// English letters are explicitly rejected to enforce Arabic-only identifiers
fn is_identifier_start(&self, c: char) -> bool {
    c == '_' || self.is_arabic_letter(c)
}
```

### 2. Non-Obvious Domain Knowledge

```rust
// GOOD: Explains domain-specific behavior
// NFC normalization ensures Arabic identifiers with different Unicode
// representations (composed vs decomposed) are treated as identical
fn normalize_name(name: &str) -> String {
    name.nfc().collect()
}
```

### 3. Complex Algorithms

```rust
// GOOD: Explains the approach
// Method resolution follows C3 linearization:
// 1. Check the class itself
// 2. Check implemented interfaces
// 3. Check parent class recursively
fn resolve_method(&self, class: &ClassType, name: &str) -> Option<&Method> {
```

### 4. Workarounds and Edge Cases

```rust
// GOOD: Explains why this unusual code exists
// LLVM requires opaque pointers in version 15+; we can't use typed pointers
let ptr_type = context.ptr_type(AddressSpace::default());
```

### 5. Public API Documentation

```rust
/// Parses a variable declaration.
///
/// # Grammar
/// ```text
/// متغير <name> [: <type>] = <expr>
/// ```
pub fn parse_var_decl(&mut self) -> ParseResult<Stmt> { }
```

## Comment Cleanup Checklist

Before committing, ask for each comment:

1. **Does the code already say this?** → Delete the comment
2. **Would renaming make this clear?** → Rename instead of comment
3. **Is this explaining "what" or "why"?** → Keep only "why"
4. **Is this a section header?** → Use code structure instead
5. **Would a new developer need this?** → Keep if truly non-obvious

## Bilingual Comments Exception

For Arabic keyword mappings, include both languages:

```rust
// GOOD: Helps developers unfamiliar with Arabic
TokenKind::Let        // متغير
TokenKind::Const      // ثابت
TokenKind::Function   // دالة
```

## Comment Budget When Writing Code

A comment block must not outgrow the code it documents. Concretely:

| Situation | Budget |
|-----------|--------|
| Public API doc comment | ≤3 lines, unless it carries a grammar block (see `parse_var_decl` above) |
| Inline comment inside a function | 1 line, explaining *why* |
| Any comment block | Never longer than the code it precedes |

Two habits that inflate a diff without adding meaning:

1. **Growing comments while editing.** Touching a function is not an occasion to
   expand its documentation. If the existing comment is still true, leave it.
2. **Documenting the obvious in bulk.** Ten lines describing a five-line `match`
   makes the `match` harder to find, not easier to understand.

A reviewer reads the diff. Every line of comment they must read is a line of code
they are not reading.

## Spotting Bloat: File It, Don't Fix It

When you notice an existing comment block that is too long, **do not fix it in the
change you are working on**. An unrelated comment refactor inflates the diff and
makes the PR harder to review — the exact problem this rule exists to prevent.

**Trigger** — any one of:
- A block of **≥10 comment lines that would lose nothing at ≤5**
- A doc comment duplicated from another declaration
- A comment that is now factually stale (describes behavior that changed)

**What to do:**

1. Check for a duplicate: `gh issue list --state open --search "<file name>"`
2. If none, file one short issue:
   ```bash
   gh issue create --title "[CODE-QUALITY] Over-long comment block in <path>" --label "code-quality" --label "documentation" --body "..."
   ```
3. Report the issue number to the user and **continue the original task**.

**Body format — file, line range, one sentence. Nothing more:**

```
`src/ir/builder/expr_builder.rs:120-141` — 22 comment lines restating the match arms below.
Reducible to ~4 lines stating why SSA temps are reused, without losing meaning.
No behavior change.
```

**Limits:**
- Cap at ~3 such issues per session. The tracker is for signal, not inventory.
- **Exception:** if the bloated comment sits inside lines you are already rewriting,
  delete it as part of that work. That is not a drive-by.
- No AI attribution in the issue — no "Generated with", no model name.

## Summary

| Comment Type | Action |
|--------------|--------|
| Restates code | Delete |
| Section headers (ASCII art) | Delete, use code structure |
| Obvious type/field docs | Delete |
| Design decisions | Keep |
| Domain knowledge | Keep |
| Complex algorithms | Keep |
| Workarounds/edge cases | Keep |
| Public API grammar | Keep |
| Arabic keyword mappings | Keep |
| Existing block ≥10 lines, reducible to ≤5 | File a `code-quality` issue, don't fix inline |
