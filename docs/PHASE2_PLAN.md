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

- [x] Loop optimizations (`src/ir/opt/loop_opt.rs`)
  - Loop detection and analysis
  - Loop-invariant code motion (LICM)
  - Strength reduction
  - Loop unrolling (optional, O3 only)
  - Induction variable analysis

- [x] Optimization pipeline (`src/ir/opt/mod.rs`)
  - Configurable optimization levels (-O0, -O1, -O2, -O3)
  - Fixed-point iteration for multi-pass optimization
  - Statistics collection (OptStats)
  - Loop optimizations integrated at O2+

**Deliverables:**
- ✅ `-O` CLI flag with levels 0-3
- ✅ `--dump-opt-stats` flag for optimization statistics
- ✅ Five optimization passes working and tested (including loop optimizations)

---

### Milestone 2.4: LLVM Code Generation ✅ COMPLETE
**Goal:** Generate native code via LLVM

#### Implementation Notes:
- Uses LLVM IR text generation (no inkwell dependency required)
- Generated IR can be compiled with clang/llc to native code
- Supports x86_64 and aarch64 targets

#### Tasks:
- [x] Target configuration (`src/codegen/target.rs`)
  - Target triple handling
  - Data layout setup
  - Native platform detection

- [x] Type mapping (`src/codegen/llvm/types.rs`)
  - Primitive types → LLVM types (i64, double, i1, ptr)
  - Array types → LLVM array/pointer
  - Class types → LLVM struct
  - Function types → LLVM function types
  - Zero initializers

- [x] LLVM IR codegen (`src/codegen/llvm/codegen.rs`)
  - Module header generation
  - String literal table
  - Runtime type definitions
  - All IR instruction conversion
  - Function name mangling for Arabic

- [x] Function codegen (integrated in codegen.rs)
  - Function declaration with return types
  - Parameter handling
  - Local variable allocation (alloca)
  - Return value handling

- [x] Class codegen (integrated in codegen.rs)
  - Struct type generation
  - VTable generation
  - Field access (GEP instructions)
  - Method calls (direct and virtual)

- [x] Object file emission (`src/codegen/linker.rs`)
  - Object file generation via clang/llc (.o)
  - Assembly output (--emit-asm)
  - LLVM IR output (--emit-llvm)

- [x] Linker integration (`src/codegen/linker.rs`)
  - System linker invocation (clang/ld/lld)
  - Fallback to LLVM IR when no compiler available
  - Executable generation

**Deliverables:**
- ✅ `tarqeem compile برنامج.trq --emit-llvm` generates LLVM IR
- ✅ `tarqeem compile برنامج.trq --emit-asm` generates assembly (requires clang/llc)
- ✅ `tarqeem compile برنامج.trq -o برنامج` produces executable (requires clang)
- ✅ Support for x86_64-linux and other targets
- ✅ `--emit-llvm`, `--emit-asm`, and `--emit-obj` flags

---

### Milestone 2.5: Runtime Library ✅ COMPLETE
**Goal:** Implement core runtime functions

#### Implementation Notes:
- Runtime implemented in C (not Rust) for simpler ABI compatibility with LLVM IR
- Compiles to static library (libtrq.a) that links with generated code
- All functions use the `trq_` prefix

#### Tasks:
- [x] Memory management (`runtime/memory.c`)
  - Reference-counted allocation (trq_alloc, trq_realloc, trq_free)
  - Retain/release functions (trq_retain, trq_release)
  - Reference count header attached to all allocations

- [x] String runtime (`runtime/string.c`)
  - TrqString structure with UTF-8 support
  - String creation, concatenation, comparison
  - String to/from number conversion
  - Unicode code point counting

- [x] Array runtime (`runtime/array.c`)
  - TrqArray structure with dynamic sizing
  - Bounds-checked access (get/set)
  - Push/pop operations
  - Array slicing and cloning

- [x] I/O runtime (`runtime/io.c`)
  - Print functions (trq_print, trq_print_int, etc.)
  - Input functions (trq_input, trq_input_prompt)
  - File operations (open, read, write, close)

- [x] Built-in functions (`runtime/builtins.c`)
  - Math operations (pow, abs, sqrt, sin, cos, etc.)
  - Exception handling (trq_throw, trq_get_exception)
  - Runtime init/cleanup (trq_runtime_init/cleanup)
  - Program entry point (main wrapper)

- [x] Build system (`runtime/Makefile`)
  - Compiles to libtrq.a static library
  - Debug and release modes
  - Install target

**Deliverables:**
- ✅ Runtime library compiles with standard C compiler
- ✅ All runtime functions available for linking
- ✅ Bilingual error messages (Arabic/English)
- ✅ Memory-safe reference counting

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
├── ir/                          # Intermediate Representation ✅
│   ├── mod.rs
│   ├── instruction.rs           # IR instruction definitions
│   ├── builder.rs               # AST → IR conversion
│   └── opt/                     # Optimizations
│       ├── mod.rs               # Optimization pipeline
│       ├── const_fold.rs        # Constant folding
│       ├── dce.rs               # Dead code elimination
│       ├── cse.rs               # Common subexpression elimination
│       ├── inline.rs            # Function inlining
│       └── loop_opt.rs          # Loop optimizations
│
├── codegen/                     # Code Generation ✅
│   ├── mod.rs
│   ├── target.rs                # Target triple configuration
│   ├── linker.rs                # Linker integration (clang/llc)
│   └── llvm/                    # LLVM IR text generation
│       ├── mod.rs
│       ├── types.rs             # Type mapping (IR → LLVM)
│       └── codegen.rs           # Main IR → LLVM conversion
│
├── semantic/                    # Enhanced ✅
│   ├── mod.rs
│   ├── analyzer.rs              # Main semantic analyzer
│   ├── types.rs                 # Type definitions
│   ├── scope.rs                 # Scope management
│   ├── class_resolver.rs        # Class hierarchy resolution
│   ├── generics.rs              # Generic type resolution
│   └── method_resolver.rs       # Method lookup
│
runtime/                         # C Runtime Library ✅
├── tarqeem_rt.h                 # Header file with all declarations
├── memory.c                     # Reference-counted allocation
├── string.c                     # UTF-8 string operations
├── array.c                      # Dynamic array operations
├── io.c                         # I/O and file operations
├── builtins.c                   # Math, exceptions, entry point
└── Makefile                     # Builds libtrq.a

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
