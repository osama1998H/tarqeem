# Phase 2: Code Generation & Full Language Features

## Overview

Phase 2 transforms Tarqeem from a compiler frontend into a fully functional compiled language. This phase covers:

1. **Intermediate Representation (IR)** - Three-address code SSA-based IR
2. **Code Optimization** - Constant folding, dead code elimination, inlining
3. **Native Code Generation** - LLVM backend for x86_64, ARM64, WebAssembly
4. **Complete OOP Semantics** - Full class/interface/generics type checking
5. **Runtime Library** - Memory management, built-in functions

---

## Phase 2 Milestones

### Milestone 2.1: IR Infrastructure (Foundation) ✅ COMPLETE
**Goal:** Create the intermediate representation layer

#### Tasks:
- [x] Define IR instruction set (`src/ir/instruction.rs`)
  - Constants, arithmetic, comparison operations
  - Control flow (jump, branch, return)
  - Function calls and returns
  - Memory operations (alloc, load, store)
  - Object operations (new, get/set field, call method)
  - Array operations (new, get, set, len)
  - Exception handling (try, catch, throw)
  - Phi nodes for SSA form

- [x] Implement IR builder (`src/ir/builder.rs`)
  - Convert typed AST to IR
  - Handle all expression types
  - Handle all statement types
  - Basic block management
  - SSA variable numbering

- [x] Create basic block infrastructure (`src/ir/instruction.rs`)
  - Basic block representation with predecessors/successors
  - Block terminators (jump, branch, return, throw)
  - Function and Module structures

- [x] IR printer/serializer (via Display traits)
  - Human-readable IR dump
  - Module, function, and instruction formatting

**Deliverables:**
- ✅ `--dump-ir` CLI flag working
- ✅ All example programs generate valid IR

---

### Milestone 2.2: Type System Completion ✅ COMPLETE
**Goal:** Complete semantic analysis for OOP features

#### Tasks:
- [x] Class type resolution (`src/semantic/class_resolver.rs`)
  - Build class hierarchy (inheritance tree)
  - Validate interface implementations
  - Check method overrides
  - Virtual method table (vtable) construction
  - Field inheritance and lookup
  - Circular inheritance detection

- [x] Generic type resolution (`src/semantic/generics.rs`)
  - Type parameter substitution
  - Generic constraint checking
  - Type inference from arguments
  - Nested generic context support

- [x] Method resolution (`src/semantic/method_resolver.rs`)
  - Instance method lookup
  - Static method lookup
  - Super method calls
  - Built-in type methods (array.طول, string.length, etc.)
  - Map/dictionary member access

- [ ] Module system (`src/semantic/modules.rs`) - *Deferred to Phase 3*
  - Module path resolution
  - Circular dependency detection
  - Public/private visibility checking
  - Export validation

**Deliverables:**
- ✅ All OOP examples type-check correctly
- ✅ Generic types fully resolved
- ⏳ Module imports deferred to Phase 3

---

### Milestone 2.3: Code Optimization ✅ COMPLETE
**Goal:** Implement IR-level optimizations

#### Tasks:
- [x] Constant folding (`src/ir/opt/const_fold.rs`)
  - Arithmetic on integer/float constants
  - Boolean simplification
  - Comparison folding
  - Branch condition folding
  - Constant propagation

- [x] Dead code elimination (`src/ir/opt/dce.rs`)
  - Remove unused variables
  - Remove unreachable blocks (reachability analysis)
  - Preserve side-effecting instructions

- [x] Common subexpression elimination (`src/ir/opt/cse.rs`)
  - Identify repeated computations
  - Replace with cached results
  - Handle commutative operations (a+b == b+a)
  - Variable substitution tracking

- [x] Function inlining (`src/ir/opt/inline.rs`)
  - Small function inlining
  - Call site counting
  - Configurable thresholds
  - Recursion detection
  - Variable/block renumbering

- [ ] Loop optimizations (`src/ir/opt/loop.rs`) - *Deferred*
  - Loop-invariant code motion
  - Strength reduction
  - Loop unrolling (optional)

- [x] Optimization pipeline (`src/ir/opt/mod.rs`)
  - Configurable optimization levels (-O0, -O1, -O2, -O3)
  - Fixed-point iteration for multi-pass optimization
  - Statistics collection (OptStats)

**Deliverables:**
- ✅ `-O` CLI flag with levels 0-3
- ✅ `--dump-opt-stats` flag for optimization statistics
- ✅ Four optimization passes working and tested

---

### Milestone 2.4: LLVM Code Generation
**Goal:** Generate native code via LLVM

#### Dependencies:
- Add `inkwell` crate to Cargo.toml

#### Tasks:
- [ ] LLVM context setup (`src/codegen/llvm/context.rs`)
  - Module creation
  - Target machine configuration
  - Data layout setup

- [ ] Type mapping (`src/codegen/llvm/types.rs`)
  - Primitive types → LLVM types
  - Array types → LLVM array/pointer
  - Class types → LLVM struct
  - Function types → LLVM function types

- [ ] Expression codegen (`src/codegen/llvm/expr.rs`)
  - Literals → LLVM constants
  - Arithmetic → LLVM instructions
  - Comparisons → LLVM icmp/fcmp
  - Function calls → LLVM call

- [ ] Statement codegen (`src/codegen/llvm/stmt.rs`)
  - Variable declarations → alloca + store
  - Assignments → store
  - Control flow → br/switch
  - Returns → ret

- [ ] Function codegen (`src/codegen/llvm/function.rs`)
  - Function declaration
  - Parameter handling
  - Local variable allocation
  - Return value handling

- [ ] Class codegen (`src/codegen/llvm/class.rs`)
  - Struct type generation
  - VTable generation
  - Constructor generation
  - Method generation
  - Field access

- [ ] Object file emission (`src/codegen/llvm/emit.rs`)
  - Object file generation (.o)
  - Assembly output (--emit-asm)
  - LLVM IR output (--emit-llvm)

- [ ] Linker integration (`src/codegen/linker.rs`)
  - System linker invocation (ld/lld)
  - Runtime library linking
  - Executable generation

**Deliverables:**
- `tarqeem compile برنامج.trq -o برنامج` produces working executable
- Support for x86_64-linux target
- `--emit-llvm` and `--emit-asm` flags

---

### Milestone 2.5: Runtime Library
**Goal:** Implement core runtime functions

#### Tasks:
- [ ] Memory management (`src/runtime/memory.rs`)
  - Allocation functions (trq_alloc, trq_realloc, trq_free)
  - Reference counting (trq_retain, trq_release)
  - Cycle detection (optional)

- [ ] String runtime (`src/runtime/string.rs`)
  - String allocation with UTF-8 support
  - String concatenation
  - String comparison
  - String to number conversion
  - Unicode normalization

- [ ] Array runtime (`src/runtime/array.rs`)
  - Dynamic array allocation
  - Bounds checking
  - Array growth
  - Array iteration support

- [ ] I/O runtime (`src/runtime/io.rs`)
  - Print functions (اطبع/print)
  - Input functions (ادخل/input)
  - File operations (basic)

- [ ] Error runtime (`src/runtime/error.rs`)
  - Exception object structure
  - Stack trace capture
  - Exception throwing
  - Exception catching

- [ ] Built-in functions (`src/runtime/builtins.rs`)
  - `len()` / `طول()`
  - `type()` / `نوع()`
  - `str()` / `نص()`
  - `int()` / `عدد()`
  - `float()` / `عدد_عشري()`

**Deliverables:**
- All built-in functions callable from Tarqeem
- Memory-safe string operations
- Working exception handling

---

### Milestone 2.6: Interpreter Mode (Optional)
**Goal:** Enable fast development iteration without compilation

#### Tasks:
- [ ] Tree-walking interpreter (`src/interpreter/mod.rs`)
  - Direct AST execution
  - Variable environment
  - Function call stack
  - Object system

- [ ] REPL improvements (`src/cli/repl.rs`)
  - Multi-line input
  - History with arrow keys
  - Tab completion
  - Variable inspection

**Deliverables:**
- `tarqeem run برنامج.trq` executes immediately
- Interactive REPL with state preservation

---

## File Structure After Phase 2

```
src/
├── ir/                          # NEW: Intermediate Representation
│   ├── mod.rs
│   ├── instruction.rs           # IR instruction definitions
│   ├── builder.rs               # AST → IR conversion
│   ├── cfg.rs                   # Control flow graph
│   ├── printer.rs               # IR pretty-printing
│   └── opt/                     # Optimizations
│       ├── mod.rs               # Optimization pipeline
│       ├── const_fold.rs        # Constant folding
│       ├── dce.rs               # Dead code elimination
│       ├── cse.rs               # Common subexpression elimination
│       ├── inline.rs            # Function inlining
│       └── loop.rs              # Loop optimizations
│
├── codegen/                     # NEW: Code Generation
│   ├── mod.rs
│   ├── llvm/                    # LLVM backend
│   │   ├── mod.rs
│   │   ├── context.rs           # LLVM context/module
│   │   ├── types.rs             # Type mapping
│   │   ├── expr.rs              # Expression codegen
│   │   ├── stmt.rs              # Statement codegen
│   │   ├── function.rs          # Function codegen
│   │   ├── class.rs             # Class/OOP codegen
│   │   └── emit.rs              # Object file emission
│   ├── linker.rs                # Linker integration
│   └── target.rs                # Target configuration
│
├── runtime/                     # NEW: Runtime Library
│   ├── mod.rs
│   ├── memory.rs                # Memory management
│   ├── string.rs                # String operations
│   ├── array.rs                 # Array operations
│   ├── io.rs                    # I/O operations
│   ├── error.rs                 # Exception handling
│   └── builtins.rs              # Built-in functions
│
├── semantic/                    # ENHANCED
│   ├── mod.rs
│   ├── analyzer.rs              # (existing)
│   ├── types.rs                 # (existing)
│   ├── scope.rs                 # (existing)
│   ├── class_resolver.rs        # NEW: Class hierarchy
│   ├── generics.rs              # NEW: Generic resolution
│   ├── method_resolver.rs       # NEW: Method lookup
│   └── modules.rs               # NEW: Module system
│
├── interpreter/                 # NEW: Optional interpreter
│   ├── mod.rs
│   ├── evaluator.rs
│   └── environment.rs
│
└── ... (existing modules unchanged)
```

---

## Dependencies to Add

```toml
[dependencies]
# LLVM bindings
inkwell = { version = "0.4", features = ["llvm17-0"] }

# For interpreter (optional)
rustyline = "14.0"  # REPL line editing

# For optimization analysis
petgraph = "0.6"    # Graph algorithms for CFG
```

---

## Implementation Order

### Recommended Sequence:

```
Week 1-2: IR Infrastructure (Milestone 2.1)
├── Define IR instructions
├── Implement IR builder for expressions
├── Implement IR builder for statements
└── Add --dump-ir flag

Week 3-4: Type System Completion (Milestone 2.2)
├── Class hierarchy resolution
├── Interface implementation checking
├── Method resolution
└── Basic generics support

Week 5-6: Basic LLVM Codegen (Milestone 2.4 - partial)
├── LLVM setup and type mapping
├── Expression codegen
├── Statement codegen
├── Simple function codegen

Week 7-8: OOP Codegen + Runtime (Milestones 2.4 + 2.5)
├── Class/method codegen
├── Runtime library implementation
├── Memory management
├── String operations

Week 9-10: Optimization + Polish (Milestone 2.3)
├── Constant folding
├── Dead code elimination
├── Optimization pipeline
└── Testing and bug fixes
```

---

## Success Criteria

### Phase 2 is complete when:

1. **Compilation Works**
   - `tarqeem compile examples/مرحبا.trq -o مرحبا` produces executable
   - Executable runs and prints "مرحباً بالعالم!"

2. **All Examples Compile**
   - All 5 example programs compile without errors
   - Generated executables run correctly

3. **OOP Features Work**
   - Classes can be instantiated
   - Methods can be called
   - Inheritance works correctly
   - Interfaces are enforced

4. **Error Handling Works**
   - Try/catch blocks work
   - Exceptions propagate correctly
   - Stack traces are available

5. **Performance**
   - Compilation < 1 second for small programs
   - Generated code runs within 5x of C equivalent

6. **Tests Pass**
   - All existing tests continue to pass
   - New codegen tests added and passing
   - Integration tests with compiled executables

---

## Risk Mitigation

### Potential Challenges:

1. **LLVM Complexity**
   - Mitigation: Start with simple expressions, incremental testing
   - Fallback: Can implement simpler C backend first

2. **Generic Type Resolution**
   - Mitigation: Start with monomorphization (simple but works)
   - Consider type erasure as alternative

3. **Memory Management**
   - Mitigation: Start with simple reference counting
   - Add cycle detection only if needed

4. **Arabic in Runtime Errors**
   - Mitigation: Embed string tables in executable
   - Consider ICU library for complex i18n

---

## Notes

- Keep all existing Phase 1 code working throughout Phase 2
- Add feature flags to enable/disable incomplete features
- Document all new public APIs
- Maintain bilingual error messages for new errors
- Update README.md roadmap as milestones complete
