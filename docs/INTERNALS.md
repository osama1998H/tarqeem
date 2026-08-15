# Tarqeem Compiler Internals

This document describes the internal architecture and design decisions of the Tarqeem compiler. It is intended for contributors and anyone interested in understanding how the compiler works.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Compilation Pipeline](#compilation-pipeline)
3. [Lexer](#lexer)
4. [Parser](#parser)
5. [Semantic Analysis](#semantic-analysis)
6. [Intermediate Representation (IR)](#intermediate-representation)
7. [Optimization Passes](#optimization-passes)
8. [Code Generation](#code-generation)
9. [Interpreter](#interpreter)
10. [Tools and Infrastructure](#tools-and-infrastructure)
11. [Design Patterns](#design-patterns)
12. [Adding New Features](#adding-new-features)

---

## Architecture Overview

Tarqeem is a **compiled, statically-typed Arabic programming language**. The compiler is written in Rust and targets LLVM for code generation.

### Directory Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library root
│
├── lexer/               # Lexical analysis
│   ├── mod.rs           # Module entry, re-exports
│   ├── lexer.rs         # Main lexer implementation
│   ├── scanner.rs       # Character scanner
│   ├── token.rs         # Token definitions
│   ├── keywords.rs      # Arabic/English keyword maps
│   └── position.rs      # Source position tracking
│
├── parser/              # Syntax analysis
│   ├── mod.rs           # Module entry
│   ├── parser.rs        # Core recursive descent parser
│   ├── expressions.rs   # Expression parsing (Pratt)
│   ├── statements.rs    # Statement parsing
│   ├── declarations.rs  # Declaration parsing
│   ├── ast.rs           # AST node definitions
│   └── precedence.rs    # Operator precedence
│
├── semantic/            # Semantic analysis
│   ├── mod.rs           # Module entry
│   ├── analyzer.rs      # Core semantic analyzer
│   ├── scope.rs         # Scope/symbol table
│   ├── types.rs         # Type system definitions
│   ├── resolver.rs      # Name resolution
│   └── type_checker.rs  # Type checking
│
├── ir/                  # Intermediate representation
│   ├── mod.rs           # Module entry
│   ├── instruction.rs   # IR instruction definitions
│   ├── builder.rs       # IR builder (orchestration)
│   ├── builder/         # Modular IR building
│   │   ├── mod.rs
│   │   ├── expressions.rs
│   │   ├── statements.rs
│   │   ├── operators.rs
│   │   ├── classes.rs
│   │   └── helpers.rs
│   └── opt/             # Optimization passes
│       ├── mod.rs       # Optimizer pipeline
│       ├── const_fold.rs
│       ├── dce.rs
│       ├── cse.rs
│       ├── inline.rs
│       └── loop_opt.rs
│
├── codegen/             # Code generation
│   ├── mod.rs
│   └── llvm/            # LLVM backend
│       ├── mod.rs
│       ├── codegen.rs   # LLVM IR generation
│       └── runtime.rs   # Runtime function declarations
│
├── interpreter/         # Tree-walking interpreter
│   ├── mod.rs
│   ├── executor.rs      # Core execution logic
│   ├── builtins.rs      # Builtin functions
│   ├── operators.rs     # Operator evaluation
│   └── value.rs         # Runtime values
│
├── cli/                 # Command-line interface
│   ├── mod.rs
│   └── commands/
│       ├── mod.rs       # Command dispatch
│       ├── compile.rs   # Compile command
│       └── debug.rs     # Debug command
│
├── lsp/                 # Language Server Protocol
├── debug/               # Debug Adapter Protocol
├── fmt/                 # Code formatter
├── doc/                 # Documentation generator
├── package/             # Package manager
└── utils/               # Utilities
    ├── interner.rs      # String interning
    └── context.rs       # Compiler context
```

---

## Compilation Pipeline

The compiler follows a classic multi-pass architecture:

```
Source (.ترقيم)
    │
    ▼
┌─────────┐
│  Lexer  │  Unicode-aware tokenization
└────┬────┘
     │ Token Stream
     ▼
┌─────────┐
│ Parser  │  Recursive descent + Pratt parsing
└────┬────┘
     │ AST (Abstract Syntax Tree)
     ▼
┌───────────┐
│ Semantic  │  Type checking, scope analysis
└────┬──────┘
     │ Typed AST + Symbol Tables
     ▼
┌──────────┐
│    IR    │  Three-address code generation
└────┬─────┘
     │ IR (SSA-style)
     ▼
┌───────────┐
│ Optimizer │  Constant folding, DCE, CSE, etc.
└────┬──────┘
     │ Optimized IR
     ▼
┌─────────┐
│ Codegen │  LLVM IR generation
└────┬────┘
     │ LLVM IR
     ▼
┌────────┐
│  LLVM  │  Native code generation
└────┬───┘
     │
     ▼
Executable Binary
```

### Layer Dependencies

**Critical Rule**: Each layer can only depend on layers BEFORE it.

```
Lexer → Parser → Semantic → IR → Codegen
                    ↓
              Interpreter
```

---

## Lexer

The lexer (`src/lexer/`) converts source code into a stream of tokens.

### Key Features

1. **Unicode-First**: Full UTF-8 support, NFC normalization for identifiers
2. **Bilingual Keywords**: Arabic primary, English aliases
3. **Bidirectional Text**: Proper handling of RTL Arabic mixed with LTR

### Token Structure

```rust
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub value: Option<TokenValue>,
}

pub enum TokenKind {
    // Literals
    IntLiteral,
    FloatLiteral,
    StringLiteral,

    // Keywords (Arabic primary)
    Let,        // متغير (mutaghayir)
    Const,      // ثابت (thabit)
    Function,   // دالة (dalah)
    // ...

    // Operators
    Plus, Minus, Star, Slash,
    // ...
}
```

### Scanner Algorithm

1. Skip whitespace (preserving newlines for statement termination — the parser
   ignores them while a `(`/`[` is still open, see `Parser::bracket_depth`)
2. Identify token start character
3. Dispatch to appropriate handler (number, string, identifier, operator)
4. Return token with span information

### Arabic Keyword Mapping

```rust
// keywords.rs
pub fn keyword_lookup(s: &str) -> Option<TokenKind> {
    match s {
        "متغير" | "let" | "var" => Some(TokenKind::Let),
        "ثابت" | "const" => Some(TokenKind::Const),
        "دالة" | "function" | "fn" => Some(TokenKind::Function),
        // ...
    }
}
```

---

## Parser

The parser (`src/parser/`) builds an Abstract Syntax Tree from tokens.

### Parsing Strategy

- **Recursive Descent**: Top-down parsing for statements and declarations
- **Pratt Parsing**: Operator precedence parsing for expressions

### AST Structure

```rust
pub enum Expr {
    Literal(Literal),
    Identifier(Ident),
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
    Unary { op: UnaryOp, operand: Box<Expr> },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    Member { object: Box<Expr>, property: Ident },
    // ...
}

pub enum Stmt {
    VarDecl { name: Ident, ty: Option<Type>, init: Option<Expr>, mutable: bool },
    FuncDecl { name: Ident, params: Vec<Param>, ret_ty: Option<Type>, body: Block },
    ClassDecl { name: Ident, extends: Option<Ident>, implements: Vec<Ident>, body: Vec<ClassMember> },
    If { cond: Expr, then_branch: Block, else_branch: Option<Block> },
    // ...
}
```

### Expression Parsing (Pratt)

The Pratt parser uses operator precedence:

```rust
fn parse_expression(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
    let mut left = self.parse_prefix()?;

    while let Some(prec) = self.peek_precedence() {
        if prec < min_prec { break; }
        left = self.parse_infix(left)?;
    }

    Ok(left)
}
```

---

## Semantic Analysis

Semantic analysis (`src/semantic/`) performs type checking and scope resolution.

### Phases

1. **Name Resolution**: Bind identifiers to declarations
2. **Type Checking**: Verify type compatibility
3. **Generic Resolution**: Instantiate generic types
4. **Inheritance Resolution**: Build class hierarchies

### Scope Management

```rust
pub struct Scope {
    parent: Option<ScopeId>,
    symbols: HashMap<InternedString, Symbol>,
    kind: ScopeKind,
}

pub enum ScopeKind {
    Global,
    Module,
    Function,
    Block,
    Class,
}
```

### Type System

```rust
pub enum Type {
    // Primitives
    Int,
    Float,
    String,
    Bool,
    Void,

    // Compound
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Optional(Box<Type>),

    // User-defined
    Class(ClassId),
    Interface(InterfaceId),
    Generic { name: String, constraints: Vec<Type> },

    // Special
    Any,
    Never,
    Unknown,
}
```

### Type Inference Algorithm

1. Collect all variable declarations with explicit types
2. For each expression:
   - If type is explicit, use it
   - If initialization exists, infer from expression type
   - Otherwise, mark as unknown and defer
3. Propagate types through the AST
4. Check for unresolved types and report errors

---

## Intermediate Representation

The IR (`src/ir/`) is a three-address code representation with SSA-style variables.

### IR Instruction Set

```rust
pub enum Instruction {
    // Constants
    Const { dest: VarId, value: Constant, ty: IrType },

    // Arithmetic
    Binary { dest: VarId, op: BinaryOp, left: VarId, right: VarId, ty: IrType },
    Unary { dest: VarId, op: UnaryOp, operand: VarId, ty: IrType },

    // Memory
    Load { dest: VarId, ptr: VarId },
    Store { ptr: VarId, value: VarId },

    // Control flow
    Jump { target: BlockId },
    Branch { cond: VarId, then_block: BlockId, else_block: BlockId },
    Return { value: Option<VarId> },

    // Functions
    Call { dest: Option<VarId>, func: FuncId, args: Vec<VarId> },

    // Objects
    NewObject { dest: VarId, class: ClassId },
    GetField { dest: VarId, object: VarId, field: FieldId },
    SetField { object: VarId, field: FieldId, value: VarId },
    CallMethod { dest: Option<VarId>, object: VarId, method: MethodId, args: Vec<VarId> },
}
```

### IR Builder Structure

```
src/ir/builder.rs          # Orchestrates IR building
src/ir/builder/
    ├── expressions.rs     # Expression IR generation
    ├── statements.rs      # Statement IR generation
    ├── operators.rs       # Operator IR generation
    ├── classes.rs         # Class/method IR generation
    └── helpers.rs         # Utility functions
```

---

## Optimization Passes

Optimizations (`src/ir/opt/`) are organized as a pipeline:

### Pipeline Configuration

| Level | Passes |
|-------|--------|
| O0 | None |
| O1 | Constant Folding, DCE |
| O2 | O1 + CSE, Loop Opts |
| O3 | O2 + Inlining, Unrolling |

### Constant Folding (`const_fold.rs`)

Evaluates constant expressions at compile time:

```
// Before
%1 = const 2
%2 = const 3
%3 = add %1, %2

// After
%3 = const 5
```

### Dead Code Elimination (`dce.rs`)

Removes unused instructions and unreachable blocks:

1. Mark all blocks reachable from entry
2. Mark all instructions with side effects as live
3. Propagate liveness backwards through uses
4. Remove unmarked instructions and blocks

### Common Subexpression Elimination (`cse.rs`)

Caches repeated computations:

```
// Before
%1 = add %a, %b
%2 = mul %1, %c
%3 = add %a, %b  // duplicate

// After
%1 = add %a, %b
%2 = mul %1, %c
// %3 references %1 instead
```

### Function Inlining (`inline.rs`)

Replaces function calls with function bodies for small functions.

### Loop Optimizations (`loop_opt.rs`)

- **LICM**: Hoist loop-invariant computations
- **Strength Reduction**: Replace expensive ops with cheaper equivalents
- **Unrolling**: Duplicate loop body (O3 only)

---

## Code Generation

Code generation (`src/codegen/`) produces LLVM IR using the `inkwell` crate.

### LLVM Type Mapping

| Tarqeem Type | LLVM Type |
|--------------|-----------|
| عدد (Int) | i64 |
| عدد_عشري (Float) | double |
| منطقي (Bool) | i1 |
| نص (String) | %String* |
| مصفوفة (Array) | %Array* |
| Object | %ClassName* |

### Class Layout

A class instance carries a vtable pointer at word 0, followed by its fields —
inherited ones first, then its own. Reference counting lives in the allocation
header `trq_alloc` prepends, *outside* the object pointer, so it is not a struct
field:

```
// trq_alloc returns a pointer past its own header
[ refcount: i64 | size: i64 ]   <- allocation header (runtime-rs/src/memory.rs)
┌──────────────────────────────┐ <- the object pointer
│ vtable: ptr                  │    word 0
│ inherited fields...          │
│ own fields...                │
└──────────────────────────────┘

@vtable.<Class> = internal constant [N x ptr] [ptr @<Class>::<method>, ...]
```

A subclass's vtable extends its parent's as a **prefix**: an override replaces the
inherited entry in place, and new members are appended. So a member's slot index
is the same in a class and all its descendants, which lets codegen take the index
from the receiver's static class and still call the runtime class's
implementation.

Classes with no virtually dispatchable member emit no vtable global; word 0 stays
zero and is never loaded. Object literals (`__anonymous__`) have no vtable slot at
all — they resolve fields by name and are never method receivers.

`الأصل.method()` is compiled as a direct call, not a vtable load: an override that
super-calls must reach the parent's body rather than resolving back into itself.

### Runtime Functions

The codegen declares external runtime functions:

- `trq_print_int`, `trq_print_string`, etc.
- `trq_string_concat`, `trq_string_length`
- `trq_array_new`, `trq_array_get`, `trq_array_set`
- Memory management: `trq_alloc`, `trq_retain`, `trq_release`

---

## Interpreter

The interpreter (`src/interpreter/`) is a tree-walking interpreter for debugging and REPL.

### Execution Model

```rust
pub struct Interpreter {
    globals: HashMap<String, Value>,
    call_stack: Vec<Frame>,
    builtins: BuiltinRegistry,
}

impl Interpreter {
    fn execute_stmt(&mut self, stmt: &Stmt) -> Result<(), RuntimeError>;
    fn evaluate_expr(&mut self, expr: &Expr) -> Result<Value, RuntimeError>;
}
```

### Value Representation

```rust
pub enum Value {
    Null,
    Int(i64),
    Float(f64),
    Bool(bool),
    String(Rc<String>),
    Array(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<ObjectInstance>>),
    Function(Rc<FunctionValue>),
}
```

---

## Tools and Infrastructure

### LSP Server (`src/lsp/`)

Provides IDE features:
- Diagnostics
- Go to definition
- Hover information
- Completions
- Semantic tokens

### DAP Server (`src/debug/`)

Debug Adapter Protocol implementation:
- Breakpoints
- Step in/over/out
- Variable inspection
- Call stack

### Formatter (`src/fmt/`)

Code formatter with RTL awareness.

### Package Manager (`src/package/`)

Handles dependencies with Arabic manifest format.

---

## Design Patterns

### Error Handling

All compiler errors are bilingual:

```rust
pub struct Diagnostic {
    pub level: Level,
    pub message: String,   // Arabic
    pub span: Span,
    pub notes: Vec<Note>,
}
```

### String Interning

Identifiers are interned for efficient comparison:

```rust
pub struct StringInterner {
    map: HashMap<String, InternedString>,
    strings: Vec<String>,
}
```

### Module Organization

Large modules are split into submodules:
- `mod.rs`: Module entry, re-exports, shared types
- `*.rs`: Implementation files for specific functionality

---

## Adding New Features

### Adding a New Keyword

1. Add token to `src/lexer/token.rs`
2. Add Arabic/English mapping in `src/lexer/keywords.rs`
3. Add parsing logic in `src/parser/`
4. Add semantic analysis in `src/semantic/`
5. Add IR generation in `src/ir/builder/`
6. Add codegen in `src/codegen/llvm/`
7. Add interpreter support in `src/interpreter/`
8. Add tests at each stage

### Adding a Standard Library Function

1. Implement in C in `runtime/`
2. Declare in `src/codegen/llvm/runtime.rs`
3. Register type in `src/semantic/scope.rs`
4. Add interpreter implementation in `src/interpreter/builtins.rs`
5. Create Tarqeem wrapper in `stdlib/`
6. Add documentation and tests

### Adding an Optimization Pass

1. Create pass file in `src/ir/opt/`
2. Implement `OptStats` tracking
3. Add to pipeline in `src/ir/opt/mod.rs`
4. Add tests in pass file
5. Add integration tests in `src/ir/opt/integration_tests.rs`

---

## Testing Strategy

### Unit Tests

Each module has inline tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specific_behavior() {
        // ...
    }
}
```

### Integration Tests

Full compilation tests in `tests/`:
- Parse and type-check example programs
- Compile and execute with expected output

### Benchmarks

Performance benchmarks in `benches/`:
- Lexer throughput
- Parser speed
- Type checker performance
- Optimizer timing
- End-to-end compilation

---

## Contributing

1. Read this document to understand the architecture
2. Follow the patterns established in existing code
3. Add tests for all new functionality
4. Ensure `cargo clippy` produces no warnings
5. Update documentation for public API changes
6. Add bilingual error messages for user-facing errors

---

**ترقيم ليست ترجمة - بل لغة برمجة عربية أصيلة**

Tarqeem is not a translation - it is an authentic Arabic programming language.
