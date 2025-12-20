# Testing Rules

This file defines testing requirements for all code changes.

## Test Requirements

### Every Change Must Have Tests

No code change is complete without appropriate tests:

| Change Type | Required Tests |
|------------|----------------|
| New keyword | Lexer test, parser test, integration test |
| New AST node | Parser test, semantic test |
| New type | Type checker test |
| Bug fix | Regression test proving the fix |
| Optimization | Correctness test + benchmark (optional) |
| Error message | Test that error is triggered correctly |

### Test File Locations

```
src/
├── lexer/
│   └── lexer.rs          # Unit tests: #[cfg(test)] mod tests
├── parser/
│   └── parser.rs         # Unit tests
├── semantic/
│   └── analyzer.rs       # Unit tests
├── ir/
│   └── builder.rs        # Unit tests
└── codegen/
    └── llvm/codegen.rs   # Unit tests

tests/                     # Integration tests
├── lexer_tests.rs
├── parser_tests.rs
├── type_tests.rs
├── codegen_tests.rs
└── integration/
    └── *.trq             # End-to-end tests
```

## Test Patterns

### Arabic Source Code in Tests

Always include Arabic source code in tests:

```rust
#[test]
fn test_arabic_variable() {
    let source = r#"متغير س = 5"#;
    let result = parse(source);
    assert!(result.is_ok());
}
```

### Mixed Language Tests

Test both Arabic and English keywords:

```rust
#[test]
fn test_bilingual_keywords() {
    // Arabic
    assert_tokens("متغير", &[TokenKind::Let]);

    // English alias
    assert_tokens("let", &[TokenKind::Let]);
}
```

### Error Path Tests

Test that errors are reported correctly:

```rust
#[test]
fn test_undefined_variable_error() {
    let source = r#"اطبع(غير_موجود)"#;
    let result = analyze(source);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("undefined"));
    assert!(err.message_ar.contains("غير معرف"));
}
```

### Edge Case Tests

Always test edge cases:

```rust
#[test]
fn test_empty_input() { /* ... */ }

#[test]
fn test_unicode_boundaries() { /* ... */ }

#[test]
fn test_maximum_nesting() { /* ... */ }

#[test]
fn test_arabic_numerals() { /* ... */ }
```

## Running Tests

### Standard Commands

```bash
# Run all tests
cargo test

# Run tests for specific module
cargo test lexer
cargo test parser
cargo test semantic

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_arabic_variable
```

### Before Committing

Run this sequence:

```bash
cargo fmt --check && cargo clippy && cargo test
```

## Test Naming Conventions

```rust
// Format: test_<what>_<condition>_<expected>
#[test]
fn test_lexer_arabic_keyword_returns_correct_token() { }

#[test]
fn test_parser_missing_semicolon_reports_error() { }

#[test]
fn test_type_checker_mismatched_types_fails() { }
```

## Integration Test Pattern

For `.trq` files in `tests/integration/`:

```rust
#[test]
fn test_integration_hello_world() {
    let result = compile_and_run("tests/integration/مرحبا.trq");
    assert_eq!(result.stdout, "مرحباً بالعالم!\n");
    assert_eq!(result.exit_code, 0);
}
```

## What NOT to Test

- Implementation details that may change
- Private helper functions (test through public API)
- Third-party library behavior

## Test Documentation

If a test is non-obvious, add a comment:

```rust
#[test]
fn test_arabic_comma_handling() {
    // Arabic uses '،' (U+060C) instead of ',' (U+002C)
    // The lexer must accept both
    let source = "جمع(1، 2)";  // Arabic comma
    assert!(parse(source).is_ok());
}
```
