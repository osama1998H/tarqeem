# Tarqeem Architecture

This document describes the technical architecture of the Tarqeem compiler and runtime.

## Technology Choice: Rust

Tarqeem is implemented in **Rust** for the following reasons:

1. **Performance**: Rust produces native binaries with C/C++ level performance
2. **Memory Safety**: No garbage collector, yet memory-safe through ownership system
3. **Excellent Tooling**: Cargo for package management, great testing framework
4. **Parser Libraries**: Rich ecosystem (pest, nom, lalrpop) for building parsers
5. **LLVM Bindings**: Mature `inkwell` crate for LLVM code generation
6. **Unicode Support**: First-class Unicode/UTF-8 support essential for Arabic
7. **Error Handling**: Result types make compiler error handling robust

## Project Structure

```
tarqeem/
├── Cargo.toml                 # Rust package manifest
├── Cargo.lock                 # Dependency lock file
├── README.md                  # Project overview
├── ARCHITECTURE.md            # This file
├── CLAUDE.md                  # Development guidelines
│
├── src/
│   ├── main.rs               # CLI entry point
│   ├── lib.rs                # Library root
│   │
│   ├── lexer/                # Lexical analysis
│   │   ├── mod.rs
│   │   ├── token.rs          # Token definitions
│   │   ├── scanner.rs        # Character scanner
│   │   ├── lexer.rs          # Main lexer implementation
│   │   └── keywords.rs       # Arabic/English keyword maps
│   │
│   ├── parser/               # Syntax analysis
│   │   ├── mod.rs
│   │   ├── ast.rs            # AST node definitions
│   │   ├── parser.rs         # Recursive descent parser
│   │   ├── precedence.rs     # Operator precedence
│   │   └── error.rs          # Parse error types
│   │
│   ├── semantic/             # Semantic analysis
│   │   ├── mod.rs
│   │   ├── analyzer.rs       # Main semantic analyzer
│   │   ├── scope.rs          # Scope/symbol table
│   │   ├── types.rs          # Type system
│   │   ├── resolver.rs       # Name resolution
│   │   └── type_checker.rs   # Type checking
│   │
│   ├── ir/                   # Intermediate representation
│   │   ├── mod.rs
│   │   ├── instruction.rs    # IR instructions
│   │   ├── builder.rs        # IR builder
│   │   └── optimizer.rs      # IR-level optimizations
│   │
│   ├── codegen/              # Code generation
│   │   ├── mod.rs
│   │   ├── llvm.rs           # LLVM code generator
│   │   ├── target.rs         # Target machine config
│   │   └── linker.rs         # Linking utilities
│   │
│   ├── runtime/              # Runtime library
│   │   ├── mod.rs
│   │   ├── gc.rs             # Garbage collector (optional)
│   │   ├── string.rs         # String operations
│   │   └── io.rs             # I/O operations
│   │
│   ├── stdlib/               # Standard library
│   │   ├── mod.rs
│   │   ├── collections.rs    # Data structures
│   │   ├── math.rs           # Math functions
│   │   ├── io.rs             # I/O module
│   │   ├── string.rs         # String utilities
│   │   ├── net.rs            # Networking
│   │   └── fs.rs             # File system
│   │
│   ├── error/                # Error handling
│   │   ├── mod.rs
│   │   ├── diagnostic.rs     # Error diagnostics
│   │   ├── reporter.rs       # Error reporter (Arabic/English)
│   │   └── span.rs           # Source location spans
│   │
│   └── cli/                  # Command-line interface
│       ├── mod.rs
│       ├── commands.rs       # CLI commands
│       ├── repl.rs           # Interactive REPL
│       └── formatter.rs      # Code formatter
│
├── stdlib_trq/               # Standard library (Tarqeem source)
│   ├── مجموعات.ترقيم          # Collections (قائمة، قاموس، مجموعة)
│   ├── رياضيات.ترقيم          # Math functions
│   ├── نص.ترقيم               # String utilities
│   ├── ملفات.ترقيم            # File operations
│   └── شبكة.ترقيم             # Networking
│
├── tests/                    # Test suites
│   ├── lexer_tests.rs
│   ├── parser_tests.rs
│   ├── type_tests.rs
│   ├── codegen_tests.rs
│   └── integration/          # Integration tests
│       └── *.ترقيم
│
├── examples/                 # Example programs
│   ├── مرحبا.ترقيم             # Hello world
│   ├── حاسبة.ترقيم             # Calculator
│   ├── لعبة.ترقيم              # Simple game
│   └── خادم.ترقيم              # HTTP server
│
└── docs/                     # Documentation
    ├── language_spec.md      # Language specification
    ├── grammar.md            # Formal grammar
    └── stdlib.md             # Standard library docs
```

## Compiler Pipeline

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          TARQEEM COMPILER PIPELINE                       │
└─────────────────────────────────────────────────────────────────────────┘

Source Code (.ترقيم)
      │
      ▼
┌─────────────┐
│   LEXER     │  Tokenization: Characters → Tokens
│  (Scanner)  │  - Unicode-aware (Arabic + English)
│             │  - Handles RTL text properly
│             │  - Keyword mapping (Arabic ↔ English)
└─────────────┘
      │
      │  Token Stream
      ▼
┌─────────────┐
│   PARSER    │  Syntax Analysis: Tokens → AST
│ (Recursive  │  - Recursive descent parsing
│  Descent)   │  - Operator precedence parsing
│             │  - Error recovery
└─────────────┘
      │
      │  Abstract Syntax Tree
      ▼
┌─────────────┐
│  SEMANTIC   │  Semantic Analysis: AST → Typed AST
│  ANALYZER   │  - Name resolution
│             │  - Type checking
│             │  - Scope analysis
└─────────────┘
      │
      │  Typed AST
      ▼
┌─────────────┐
│     IR      │  IR Generation: Typed AST → IR
│  GENERATOR  │  - Three-address code style
│             │  - SSA form
│             │  - Control flow graph
└─────────────┘
      │
      │  Intermediate Representation
      ▼
┌─────────────┐
│  OPTIMIZER  │  Optimization: IR → Optimized IR
│             │  - Constant folding
│             │  - Dead code elimination
│             │  - Inlining
└─────────────┘
      │
      │  Optimized IR
      ▼
┌─────────────┐
│   CODEGEN   │  Code Generation: IR → LLVM IR
│   (LLVM)    │  - LLVM IR generation
│             │  - Target-specific optimization
│             │  - Native code emission
└─────────────┘
      │
      │  Object Files (.o)
      ▼
┌─────────────┐
│   LINKER    │  Linking: Objects → Executable
│             │  - Runtime library linking
│             │  - Standard library linking
└─────────────┘
      │
      ▼
Executable Binary
```

## Component Details

### 1. Lexer (المحلل اللغوي)

The lexer converts source code into tokens while handling:

- **Unicode Support**: Full UTF-8 support for Arabic characters
- **Bidirectional Text**: Proper handling of RTL Arabic mixed with LTR (numbers, English)
- **Keyword Mapping**: Dual Arabic/English keyword recognition

```rust
// Token types
pub enum TokenKind {
    // Literals
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),

    // Identifiers
    Identifier(String),

    // Keywords (Arabic)
    Mutaƣayir,      // متغير - let
    Thabit,         // ثابت - const
    Dalah,          // دالة - function
    Irjiƣ,          // أرجع - return
    Itha,           // إذا - if
    WaIlla,         // وإلا - else
    Talama,         // طالما - while
    Likul,          // لكل - for
    Sinf,           // صنف - class
    // ... more keywords

    // Operators
    Plus, Minus, Star, Slash,
    Equal, EqualEqual, BangEqual,
    Less, LessEqual, Greater, GreaterEqual,
    // ... more operators

    // Delimiters
    LeftParen, RightParen,
    LeftBrace, RightBrace,
    LeftBracket, RightBracket,
    Comma, Semicolon, Colon,
    Arrow,      // ->
    FatArrow,   // =>

    // Special
    Newline, Whitespace, Comment,
    EOF, Error,
}
```

### 2. Parser (المحلل النحوي)

Recursive descent parser with Pratt parsing for expressions:

```rust
// AST nodes
pub enum Expr {
    Literal(Literal),
    Identifier(String),
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
    Unary { op: UnaryOp, operand: Box<Expr> },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    Member { object: Box<Expr>, property: String },
    Index { object: Box<Expr>, index: Box<Expr> },
    Lambda { params: Vec<Param>, body: Box<Expr> },
    New { class: Box<Expr>, args: Vec<Expr> },
    Await(Box<Expr>),
}

pub enum Stmt {
    VarDecl { name: String, mutable: bool, ty: Option<Type>, init: Option<Expr> },
    FuncDecl { name: String, params: Vec<Param>, ret_ty: Option<Type>, body: Block },
    ClassDecl { name: String, extends: Option<String>, implements: Vec<String>, body: Vec<ClassMember> },
    InterfaceDecl { name: String, methods: Vec<MethodSignature> },
    If { cond: Expr, then_branch: Block, else_branch: Option<Block> },
    While { cond: Expr, body: Block },
    For { init: Option<Box<Stmt>>, cond: Option<Expr>, update: Option<Expr>, body: Block },
    ForIn { var: String, iterable: Expr, body: Block },
    Match { expr: Expr, arms: Vec<MatchArm> },
    Return(Option<Expr>),
    Try { body: Block, catch: Option<CatchClause>, finally: Option<Block> },
    Throw(Expr),
    Import { items: ImportItems, from: String },
    Export(Box<Stmt>),
    Expr(Expr),
}
```

### 3. Type System (نظام الأنماط)

Strong static typing with inference:

```rust
pub enum Type {
    // Primitives
    Int,            // عدد
    Float,          // عدد_عشري
    String,         // نص
    Bool,           // منطقي
    Void,           // Internal: functions default to no return

    // Compound types
    Array(Box<Type>),               // مصفوفة<ن>
    Map(Box<Type>, Box<Type>),      // قاموس<م، ق>
    Function { params: Vec<Type>, ret: Box<Type> },

    // User-defined
    Class(String),
    Interface(String),
    Generic { name: String, constraints: Vec<Type> },

    // Special
    Any,            // أي
    Never,          // أبداً
    Unknown,        // Inference placeholder
}
```

### 4. Intermediate Representation (التمثيل الوسيط)

SSA-based IR for optimization:

```rust
pub enum IRInstruction {
    // Constants
    Const { dest: VarId, value: Constant },

    // Arithmetic
    Add { dest: VarId, left: VarId, right: VarId },
    Sub { dest: VarId, left: VarId, right: VarId },
    Mul { dest: VarId, left: VarId, right: VarId },
    Div { dest: VarId, left: VarId, right: VarId },

    // Comparison
    Eq { dest: VarId, left: VarId, right: VarId },
    Lt { dest: VarId, left: VarId, right: VarId },
    // ...

    // Control flow
    Jump { target: BlockId },
    Branch { cond: VarId, then_block: BlockId, else_block: BlockId },
    Return { value: Option<VarId> },

    // Functions
    Call { dest: Option<VarId>, func: FuncId, args: Vec<VarId> },

    // Memory
    Alloc { dest: VarId, ty: Type },
    Load { dest: VarId, ptr: VarId },
    Store { ptr: VarId, value: VarId },

    // Objects
    NewObject { dest: VarId, class: ClassId },
    GetField { dest: VarId, object: VarId, field: FieldId },
    SetField { object: VarId, field: FieldId, value: VarId },
    CallMethod { dest: Option<VarId>, object: VarId, method: MethodId, args: Vec<VarId> },
}
```

### 5. Code Generation (توليد الكود)

Using LLVM via the `inkwell` crate:

```rust
pub struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    execution_engine: ExecutionEngine<'ctx>,

    // Symbol tables
    variables: HashMap<String, PointerValue<'ctx>>,
    functions: HashMap<String, FunctionValue<'ctx>>,
    classes: HashMap<String, StructType<'ctx>>,
}

impl<'ctx> CodeGen<'ctx> {
    pub fn compile_program(&mut self, program: &Program) -> Result<(), CodeGenError> {
        // 1. Forward declare all functions and classes
        self.forward_declare(program)?;

        // 2. Generate function bodies
        for func in &program.functions {
            self.compile_function(func)?;
        }

        // 3. Generate class methods
        for class in &program.classes {
            self.compile_class(class)?;
        }

        // 4. Verify module
        self.module.verify()?;

        Ok(())
    }

    pub fn emit_object_file(&self, path: &Path) -> Result<(), CodeGenError> {
        let target_machine = Target::from_triple(&TargetTriple::create("x86_64-unknown-linux-gnu"))
            .create_target_machine(...);

        target_machine.write_to_file(&self.module, FileType::Object, path)?;
        Ok(())
    }
}
```

## Memory Management

Tarqeem uses a hybrid approach:

1. **Stack Allocation**: Primitives and small structs
2. **Reference Counting**: Default for heap objects
3. **Optional GC**: For cyclic data structures

```rust
// Runtime reference counting
pub struct TrqObject {
    ref_count: AtomicUsize,
    type_info: &'static TypeInfo,
    data: [u8],  // Flexible array member
}

impl TrqObject {
    pub fn retain(&self) {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn release(&self) {
        if self.ref_count.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.drop_contents();
            // Deallocate
        }
    }
}
```

## Error Handling

Comprehensive error reporting with Arabic support:

```rust
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub message_ar: String,  // Arabic translation
    pub span: Span,
    pub notes: Vec<Note>,
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    pub fn emit(&self, source: &str, lang: Language) {
        let msg = match lang {
            Language::Arabic => &self.message_ar,
            Language::English => &self.message,
        };

        // Pretty print with source context
        eprintln!("خطأ: {}", msg);
        eprintln!("  --> {}:{}:{}", self.span.file, self.span.line, self.span.col);
        // ...
    }
}
```

## Concurrency Model

Async/await with an event loop:

```rust
// Runtime async executor
pub struct Executor {
    ready_queue: VecDeque<Task>,
    io_reactor: IoReactor,
    timer_wheel: TimerWheel,
}

impl Executor {
    pub fn spawn(&mut self, future: impl Future<Output = ()>) {
        let task = Task::new(future);
        self.ready_queue.push_back(task);
    }

    pub fn run(&mut self) {
        loop {
            // 1. Poll ready tasks
            while let Some(task) = self.ready_queue.pop_front() {
                if task.poll().is_pending() {
                    // Re-queue when ready
                }
            }

            // 2. Wait for I/O events
            self.io_reactor.poll();

            // 3. Check timers
            self.timer_wheel.tick();
        }
    }
}
```

## Standard Library Architecture

The standard library is a mix of Rust runtime and Tarqeem source:

```
stdlib/
├── core/           # Rust: Low-level primitives
│   ├── memory      # Memory allocation
│   ├── string      # String internals
│   └── io          # I/O primitives
│
└── std/            # Tarqeem: High-level APIs
    ├── مجموعات     # Collections (List, Map, Set)
    ├── رياضيات     # Math functions
    ├── ملفات       # File system
    ├── شبكة        # Networking
    └── متزامن      # Async utilities
```

## Build System

```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release

# Run tests
cargo test

# Run specific test
cargo test lexer

# Generate docs
cargo doc --open

# Format code
cargo fmt

# Lint
cargo clippy
```

## Dependencies

```toml
[dependencies]
# CLI
clap = { version = "4.0", features = ["derive"] }
colored = "2.0"

# Parsing
logos = "0.13"          # Lexer generator
pest = "2.7"            # Alternative: PEG parser

# LLVM
inkwell = "0.2"         # LLVM bindings

# Unicode
unicode-segmentation = "1.10"
unicode-bidi = "0.3"

# Error handling
thiserror = "1.0"
miette = "5.0"          # Pretty error reporting

# Async runtime
tokio = { version = "1.0", features = ["full"] }

# Utilities
once_cell = "1.18"
indexmap = "2.0"
```

## Testing Strategy

1. **Unit Tests**: Each module has inline tests
2. **Integration Tests**: Full compilation of `.ترقيم` files
3. **Snapshot Tests**: AST/IR output comparison
4. **Fuzzing**: Property-based testing with `proptest`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_arabic_keywords() {
        let source = "متغير س = 5";
        let tokens = Lexer::new(source).collect::<Vec<_>>();

        assert_eq!(tokens[0].kind, TokenKind::Mutaƣayir);
        assert_eq!(tokens[1].kind, TokenKind::Identifier("س".into()));
        assert_eq!(tokens[2].kind, TokenKind::Equal);
        assert_eq!(tokens[3].kind, TokenKind::IntLiteral(5));
    }
}
```

## Performance Targets

- **Compilation Speed**: < 100ms for 10K lines
- **Runtime Performance**: Within 2x of equivalent C code
- **Memory Usage**: < 100MB for typical programs
- **Startup Time**: < 10ms for hello world

## Future Considerations

1. **JIT Compilation**: Optional JIT for REPL and hot code
2. **WebAssembly**: Compile to WASM for web deployment
3. **LSP Server**: IDE support with full Arabic localization
4. **Debugger**: Native debugging with Arabic variable names
5. **Package Manager**: Similar to Cargo/npm with Arabic package names
