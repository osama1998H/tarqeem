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

tests/                          # Integration tests
├── integration_tests.rs        # Full-pipeline compilation
├── error_codes_test.rs         # Arabic error code system
├── phase3_criteria_tests.rs
├── dap_integration_tests.rs    # Debugger
├── runtime_rs_e2e_tests.rs     # runtime-rs across the FFI boundary
└── *_execution_tests.rs        # Per-feature execution:
                                # exception, inheritance, lambda,
                                # module, oop, property

examples/                       # End-to-end .ترقيم programs,
└── *.ترقيم                      # run by .github/workflows/examples.yml
```

Add a new `tests/<feature>_execution_tests.rs` when a language feature needs
end-to-end coverage across interpreter, JIT, and native. Do not invent a new
directory layout.

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

### Arabic-Only Syntax Tests

Tarqeem syntax is Arabic. English may appear **only inside string literal content** —
never as a keyword, identifier, or alias. See `.claude/rules/arabic-philosophy.md`.

Every lexer or parser feature needs both halves: the Arabic form is accepted, and the
Latin form is rejected.

```rust
#[test]
fn test_arabic_keyword_accepted() {
    assert_tokens("متغير", &[TokenKind::Let]);
}

// Latin identifiers are rejected at lex time.
// Mirrors src/lexer/lexer.rs::test_english_identifiers_produce_errors
#[test]
fn test_english_identifiers_produce_errors() {
    let source = r#"متغير userName = "أحمد""#;
    let tokens: Vec<_> = Lexer::new(source).tokenize();
    assert_eq!(tokens[0].kind, TokenKind::Let);
    assert!(matches!(&tokens[1].kind, TokenKind::Error(msg) if msg.contains("English")));
}
```

English inside a string is fine — it is data, not syntax:

```rust
#[test]
fn test_english_string_content_is_allowed() {
    assert!(parse(r#"اطبع("hello")"#).is_ok());
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
    assert!(err.message.contains("غير معرف"));
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

End-to-end `.ترقيم` programs live in `examples/` and are executed by
`.github/workflows/examples.yml`. A program that must behave identically across
backends belongs there, because the workflow diffs interpreter, JIT, and native
output — this project's recurring failure mode is silent wrong output, where one
backend disagrees with another without erroring.

```rust
#[test]
fn test_integration_hello_world() {
    let result = compile_and_run("examples/مرحبا.ترقيم");
    assert_eq!(result.stdout, "مرحباً بالعالم!\n");
    assert_eq!(result.exit_code, 0);
}
```

Inline source in a `tests/*_execution_tests.rs` file suits a narrow feature; an
`examples/` program suits anything worth showing a user.

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
