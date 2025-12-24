---
paths: src/**/*.rs
---

# Rust Code Style (applies to src/**/*.rs)

This file defines Rust coding standards for the Tarqeem compiler.

## Naming Conventions

```rust
// Types: PascalCase
pub struct TokenKind { }
pub enum ExpressionType { }
pub trait Visitor { }

// Functions/methods: snake_case
fn parse_expression(&mut self) -> Result<Expr, ParseError> { }
fn check_types(&self) -> TypeCheckResult { }

// Variables: snake_case
let token_stream = lexer.tokenize();
let current_scope = self.scope_stack.last();

// Constants: SCREAMING_SNAKE_CASE
const MAX_NESTING_DEPTH: usize = 256;
const DEFAULT_BUFFER_SIZE: usize = 4096;
```

## Type Aliases for Clarity

```rust
// GOOD: Use type aliases for complex Result types
pub type ParseResult<T> = Result<T, ParseError>;
pub type TypeCheckResult<T> = Result<T, TypeError>;
pub type CodeGenResult<T> = Result<T, CodeGenError>;

// Usage
fn parse_stmt(&mut self) -> ParseResult<Stmt> { }
```

## Error Handling

### Never panic on user input

```rust
// BAD: Crashes on invalid input
let token = self.tokens.get(self.pos).unwrap();

// GOOD: Handle gracefully
let token = self.tokens.get(self.pos)
    .ok_or_else(|| ParseError::unexpected_eof(self.last_span()))?;
```

### Use the ? operator

```rust
// GOOD: Propagate errors with ?
fn compile(&mut self, source: &str) -> Result<Program, CompileError> {
    let tokens = self.lexer.tokenize(source)?;
    let ast = self.parser.parse(tokens)?;
    let typed_ast = self.analyzer.analyze(ast)?;
    Ok(typed_ast)
}
```

### Custom error types with thiserror

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected token: expected {expected}, found {found}")]
    UnexpectedToken {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Unexpected end of file")]
    UnexpectedEof { span: Span },
}
```

## Documentation

See `.claude/rules/comments.md` for the full commenting philosophy.

**Key principle**: Comments explain WHY, not WHAT. Don't translate code into English.

```rust
// BAD: Restates code
// Check for newlines
if c == '\n' { }

// GOOD: Explains design decision
// English letters explicitly rejected to enforce Arabic-only identifiers
fn is_identifier_start(&self, c: char) -> bool { }
```

## Struct Organization

```rust
pub struct Parser<'a> {
    // 1. Core state
    tokens: &'a [Token],
    current: usize,

    // 2. Auxiliary state
    errors: Vec<ParseError>,
    panic_mode: bool,

    // 3. Configuration
    config: ParserConfig,
}
```

## Visibility

```rust
// Prefer private by default
struct InternalHelper { }

// Only expose what's needed
pub struct PublicAPI { }

// Use pub(crate) for internal sharing
pub(crate) fn shared_utility() { }
```

## Imports

```rust
// Group imports:
// 1. Standard library
use std::collections::HashMap;
use std::fmt;

// 2. External crates
use inkwell::context::Context;
use thiserror::Error;

// 3. Crate modules
use crate::lexer::token::{Token, TokenKind};
use crate::error::Span;

// 4. Local module
use super::ast::{Expr, Stmt};
```

## Pattern Matching

```rust
// GOOD: Exhaustive matching
match token.kind {
    TokenKind::Let => self.parse_let()?,
    TokenKind::If => self.parse_if()?,
    TokenKind::While => self.parse_while()?,
    // ...all cases
    _ => return Err(ParseError::unexpected_token(token)),
}

// GOOD: Use if-let for single patterns
if let Some(expr) = self.try_parse_expr() {
    return Ok(expr);
}
```

## Lifetimes

```rust
// Name lifetimes meaningfully when not obvious
impl<'source, 'tokens> Parser<'source, 'tokens> { }

// Use 'a, 'b for simple cases
fn compare<'a>(a: &'a str, b: &'a str) -> &'a str { }
```

## Tests in the same file

```rust
// Place unit tests at the bottom of the file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() { }
}
```

## Avoid These Anti-Patterns

```rust
// BAD: Excessive cloning
let copy = expensive_value.clone();

// BAD: Ignoring Result
let _ = file.write(data);  // Error silently ignored!

// BAD: Nested match/if
match x {
    Some(y) => match y {
        Some(z) => // deeply nested...
    }
}

// GOOD: Use combinators
x.and_then(|y| y).map(|z| /* ... */)
```
