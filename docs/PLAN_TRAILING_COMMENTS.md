# Plan: Add Trailing Comments Support

## Overview

Add support for preserving and formatting trailing comments - comments that appear after a statement on the same line.

**Example:**
```tarqeem
متغير س = 5  // هذا تعليق نهائي
ثابت ط = 3.14  // الثابت باي
```

## Current State

### How Comments Are Handled Now

1. **Lexer** (`src/lexer/lexer.rs:323-337`):
   - Tokenizes `//` comments as `TokenKind::LineComment(String)`
   - Content excludes the `//` prefix
   - Stops at newline

2. **Parser** (`src/parser/parser.rs`):
   - Filters out `Newline` tokens immediately (lines 23-24, 38-39)
   - Collects line comments via `collect_line_comments()` before parsing declarations
   - Attaches them as `leading_comments` to the next statement

3. **AST** (`src/parser/ast.rs:38-44`):
   - `Stmt` has `leading_comments: Vec<String>` field
   - No trailing comment support

4. **Formatter** (`src/fmt/formatter.rs:67-73`):
   - Outputs leading comments before statements
   - No trailing comment handling

### The Problem

```tarqeem
متغير س = 5  // تعليق على س
متغير ص = 10
```

Currently, the comment `// تعليق على س` is captured as a **leading** comment for `متغير ص = 10` instead of a **trailing** comment for `متغير س = 5`.

## Implementation Plan

### Phase 1: AST Changes

**File:** `src/parser/ast.rs`

1. Add `trailing_comment: Option<String>` field to `Stmt`:

```rust
#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
    /// Line comments that appear before this statement
    pub leading_comments: Vec<String>,
    /// Line comment that appears after this statement on the same line
    pub trailing_comment: Option<String>,
}
```

2. Update `Stmt::new()` and `Stmt::with_comments()` constructors

3. Add `Stmt::with_trailing_comment()` helper method

### Phase 2: Parser Changes

**File:** `src/parser/parser.rs`

The key insight: **We need to track newlines to know if a comment is trailing or leading**.

#### Option A: Keep Newline Tokens (Recommended)

1. **Remove Newline filtering** from constructor (lines 23-24, 38-39)
2. Add helper method `skip_newlines()` to skip newlines where needed
3. Modify `parse_declaration()` to:
   - Parse the statement
   - Check if next token is `LineComment` (without intervening newline)
   - If yes, capture as trailing comment
   - Then skip newlines

#### Option B: Use Span Line Numbers

1. After parsing a statement, check if there's a `LineComment` token
2. Compare the comment's span line with the statement's ending span line
3. If same line, it's a trailing comment

**Chosen: Option A** - More explicit and doesn't require accurate span tracking.

#### Implementation Steps:

1. Remove newline filtering from `Parser::new()` and `Parser::from_tokens()`

2. Add helper method:
```rust
fn skip_newlines(&mut self) {
    while self.check(&TokenKind::Newline) {
        self.advance();
    }
}
```

3. Update `collect_line_comments()` to stop at first newline:
```rust
fn collect_line_comments(&mut self) {
    // Skip leading newlines first
    self.skip_newlines();

    while let TokenKind::LineComment(content) = &self.peek().kind {
        self.pending_comments.push(content.clone());
        self.advance();
        self.skip_newlines(); // Skip newlines after each comment
    }
}
```

4. Add new method to capture trailing comment:
```rust
fn capture_trailing_comment(&mut self) -> Option<String> {
    // Check if next non-newline token on same line is a comment
    if let TokenKind::LineComment(content) = &self.peek().kind {
        let comment = content.clone();
        self.advance();
        Some(comment)
    } else {
        None
    }
}
```

5. Modify `parse_declaration()`:
```rust
fn parse_declaration(&mut self) -> Result<Stmt, Diagnostic> {
    self.skip_newlines();  // Skip leading newlines
    self.collect_line_comments();
    let leading_comments = self.take_pending_comments();

    let doc_comment = self.consume_doc_comment();

    let mut stmt = /* ... parse statement ... */;

    // Capture trailing comment before any newline
    stmt.trailing_comment = self.capture_trailing_comment();

    stmt.leading_comments = leading_comments;

    self.skip_newlines();  // Skip trailing newlines

    Ok(stmt)
}
```

### Phase 3: Formatter Changes

**File:** `src/fmt/formatter.rs`

1. Modify `format_stmt()` to output trailing comments:

```rust
fn format_stmt(&self, stmt: &Stmt, p: &mut Printer) {
    // Output leading line comments
    for comment in &stmt.leading_comments {
        p.write("//");
        p.write(comment);
        p.newline();
    }

    self.format_doc_comment_for_stmt(&stmt.kind, p);

    match &stmt.kind {
        // ... existing formatting logic ...
    }

    // Output trailing comment if present
    if let Some(trailing) = &stmt.trailing_comment {
        // Don't newline yet - add comment on same line
        p.write("  //");  // Two spaces before trailing comment
        p.write(trailing);
    }

    // Now add newline
    p.newline();
}
```

**Note:** Need to adjust the existing `p.newline()` calls in each match arm to not duplicate.

### Phase 4: Block-level Trailing Comments

For statements inside blocks (e.g., function bodies), the same logic applies. The `parse_block()` and `format_block()` functions also need updates.

### Phase 5: Tests

**File:** `tests/fmt_tests.rs` (or new file)

Add tests for:
1. Simple trailing comment on variable declaration
2. Trailing comment on function call
3. Trailing comment inside blocks
4. Mixed leading + trailing comments
5. Multiple statements with trailing comments
6. Preserving trailing comment content exactly

Example test:
```rust
#[test]
fn test_trailing_comment_preserved() {
    let source = r#"
بسم_الله
متغير س = 5  // هذا تعليق
الحمد_لله
"#;
    let ast = parse(source).unwrap();
    let formatted = format(&ast);
    assert!(formatted.contains("متغير س = 5  // هذا تعليق"));
}
```

## Files to Modify

| File | Changes |
|------|---------|
| `src/parser/ast.rs` | Add `trailing_comment` field to `Stmt`, update constructors |
| `src/parser/parser.rs` | Keep newlines, add `skip_newlines()`, modify comment collection |
| `src/fmt/formatter.rs` | Output trailing comments after statements |
| `tests/fmt_tests.rs` | Add tests for trailing comments |

## Edge Cases to Handle

1. **Multiple comments after statement**: Only first is trailing, rest are leading for next
2. **Comments inside multi-line expressions**: Not trailing (need careful span handling)
3. **Block comments as trailing**: Currently not supported (only line comments)
4. **Empty trailing comment**: `متغير س = 5  //` - preserve empty string
5. **Semicolon placement**: `متغير س = 5 // تعليق` vs `متغير س = 5  // تعليق` (with semicolon)

## Backward Compatibility

- Leading comments continue to work as before
- Existing code without trailing comments is unaffected
- Formatter output may change slightly (trailing comments now preserved)

## Implementation Order

1. ✅ Research current comment handling (DONE)
2. ⏳ Modify AST (`ast.rs`)
3. ⏳ Modify Parser (`parser.rs`)
4. ⏳ Modify Formatter (`formatter.rs`)
5. ⏳ Add tests
6. ⏳ Run full test suite
7. ⏳ Update documentation

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Breaking existing tests | Run tests after each change |
| Performance impact from keeping newlines | Minimal - newlines are simple tokens |
| Incorrect trailing comment detection | Use span line comparison as fallback |
| Formatter output changes | Add comprehensive tests first |

## Success Criteria

1. All existing tests pass
2. Trailing comments preserved in AST
3. Formatter outputs trailing comments correctly
4. Round-trip: parse → format → parse produces same AST
