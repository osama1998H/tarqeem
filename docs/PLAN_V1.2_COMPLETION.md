# Plan: Complete v1.2 Implementation

**Objective**: Fix the three incomplete v1.2 items:
1. String interning integration (0% → 100%)
2. unwrap() reduction (522 → <100)
3. clone() reduction (500 → <350)

**Estimated Total Effort**: 3-4 days

---

## Phase 1: String Interning Integration

**Goal**: Integrate the existing `StringInterner` into the compiler pipeline.

### Step 1.1: Create CompilerContext (1 hour)

**File**: `src/context.rs` (NEW)

```rust
use crate::utils::StringInterner;

pub struct CompilerContext {
    pub interner: StringInterner,
}

impl CompilerContext {
    pub fn new() -> Self {
        Self {
            interner: StringInterner::with_capacity(10000),
        }
    }
}

impl Default for CompilerContext {
    fn default() -> Self {
        Self::new()
    }
}
```

**Update** `src/lib.rs`:
- Add `pub mod context;`
- Export `CompilerContext`

### Step 1.2: Integrate Interner into Lexer (2 hours)

**File**: `src/lexer/lexer.rs`

Changes:
1. Add lifetime parameter to `Lexer` struct
2. Add `interner: &'a mut StringInterner` field
3. Update `Lexer::new()` signature
4. In `scan_identifier()`, call `self.interner.intern(&ident)`

```rust
pub struct Lexer<'a> {
    source: Vec<char>,
    tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
    column: usize,
    interner: &'a mut StringInterner,  // NEW
}

impl<'a> Lexer<'a> {
    pub fn new(source: &str, interner: &'a mut StringInterner) -> Self {
        // ... existing code, add interner field
    }
}
```

### Step 1.3: Update CLI Commands (2 hours)

**Files**: `src/cli/commands.rs`, `src/cli/mod.rs`

For each command (compile, run, check, repl):
1. Create `CompilerContext` at entry point
2. Pass `&mut context.interner` to Lexer
3. Pass `&context.interner` to Analyzer (for future optimization)

### Step 1.4: Update Tests (1 hour)

**Files**: All lexer tests

Update test helpers to create and pass an interner:
```rust
fn parse_test(source: &str) -> Ast {
    let mut interner = StringInterner::new();
    let mut lexer = Lexer::new(source, &mut interner);
    // ...
}
```

---

## Phase 2: Critical unwrap() Reduction

**Goal**: Reduce from 522 to <100 unwrap() calls in production code.

### Step 2.1: Package Manager Path Operations (30 min)

**Files**:
- `src/cli/pm/build.rs:9`
- `src/cli/pm/clean.rs:9`
- `src/cli/pm/install.rs:8`
- `src/cli/pm/run.rs:9`
- `src/cli/pm/test.rs:10`
- `src/cli/pm/update.rs:8`

**Pattern**:
```rust
// Before
let project_root = manifest_path.parent().unwrap();

// After
let project_root = manifest_path
    .parent()
    .ok_or_else(|| PackageError::InvalidManifest("Cannot determine project root".into()))?;
```

### Step 2.2: Debug Interpreter Call Stack (30 min)

**File**: `src/debug/interpreter.rs`
**Lines**: 327, 343, 586

**Pattern**:
```rust
// Before
let frame = self.call_stack.last().unwrap();

// After
let frame = self.call_stack.last().ok_or(DebugError::EmptyCallStack)?;
```

### Step 2.3: Debug Adapter Breakpoint Lookup (15 min)

**File**: `src/debug/adapter.rs:525`

**Pattern**:
```rust
// Before
let bp = interpreter.context().get_breakpoint(id).unwrap();

// After
let bp = interpreter.context().get_breakpoint(id)
    .ok_or(DebugError::BreakpointNotFound(id))?;
```

### Step 2.4: LLVM Codegen writeln! Operations (1 hour)

**File**: `src/codegen/llvm/codegen.rs`
**Lines**: 631, 639, 792, 803, 814, 850, 859, 877, 899, 905, 923, 931, 964, 973, 1050, 1066, 1072, 1084, 1104, 1110

**Strategy**: Change function signatures to return `Result<(), std::fmt::Error>`:

```rust
// Before
fn emit_instruction(&mut self, inst: &Instruction) {
    writeln!(self.output, "...").unwrap();
}

// After
fn emit_instruction(&mut self, inst: &Instruction) -> std::fmt::Result {
    writeln!(self.output, "...")?;
    Ok(())
}
```

### Step 2.5: RwLock Operations in Interner (30 min)

**File**: `src/utils/interner.rs`
**Lines**: 178, 192, 197

**Options**:
1. Switch to `parking_lot::RwLock` (doesn't poison)
2. Handle poisoning with `.unwrap_or_else(|e| e.into_inner())`

```rust
// Option 1: Use parking_lot
use parking_lot::RwLock;

// Option 2: Handle poisoning
let strings = self.strings.read().unwrap_or_else(|e| e.into_inner());
```

### Step 2.6: LSP and Misc (30 min)

**Files**:
- `src/lsp/analysis/document.rs:120` - use `expect()` with message
- `src/lsp/utils/position.rs:225` - return Result
- `src/package/manifest.rs:400` - restructure to avoid check-then-unwrap

---

## Phase 3: High-Impact clone() Reduction

**Goal**: Reduce from 500 to <350 clone() calls.

### Step 3.1: Parser Token Cloning (1.5 hours)

**File**: `src/parser/parser.rs`

**Fix 1** - Return reference from peek():
```rust
// Before
fn peek(&self) -> Token { self.tokens[self.current].clone() }

// After
fn peek(&self) -> &Token { &self.tokens[self.current] }
```

**Fix 2** - Add `peek_kind()` helper:
```rust
fn peek_kind(&self) -> &TokenKind {
    &self.tokens[self.current].kind
}
```

**Fix 3** - Update call sites to use references.

### Step 3.2: Type System Cloning (2 hours)

**File**: `src/semantic/analyzer.rs`

**Fix 1** - Use `Rc<Type>` for expected_type:
```rust
// Before
expected_type: Option<Type>,

// After
expected_type: Option<Rc<Type>>,
```

**Fix 2** - Use `Arc<[Type]>` for function parameters:
```rust
// In Symbol::function
param_types: Arc<[Type]>,  // Instead of Vec<Type>
```

### Step 3.3: Generics Substitution (1.5 hours)

**File**: `src/semantic/generics.rs`

**Fix** - Return `Cow<Type>` from substitution:
```rust
use std::borrow::Cow;

fn apply(&self, ty: &Type) -> Cow<'_, Type> {
    match ty {
        Type::Generic(name) => {
            if let Some(sub) = self.substitutions.get(name) {
                Cow::Borrowed(sub)
            } else {
                Cow::Borrowed(ty)
            }
        }
        _ => Cow::Borrowed(ty),
    }
}
```

### Step 3.4: IR Builder Scope Stack (1 hour)

**File**: `src/ir/builder.rs:312`

**Fix** - Use reference-counted scopes:
```rust
// Before
scope_stack: Vec<HashMap<String, VarId>>,

// After
scope_stack: Vec<Rc<HashMap<String, VarId>>>,

// With copy-on-write when modifying
fn enter_scope(&mut self) {
    let new_scope = Rc::new((*self.current_scope()).clone());
    self.scope_stack.push(new_scope);
}
```

### Step 3.5: IR Instruction Cloning in Optimization Passes (1.5 hours)

**Files**:
- `src/ir/opt/const_fold.rs`
- `src/ir/opt/cse.rs`
- `src/ir/opt/inline.rs`

**Strategy**: Use `std::mem::take()` or references instead of cloning:

```rust
// Before
new_instructions.push(inst.clone());

// After (if inst is not needed after)
new_instructions.push(std::mem::take(inst));

// Or use reference if just reading
let Instruction::Call { dest, args, .. } = &instructions[idx];
```

### Step 3.6: Class Name Interning (30 min)

**File**: `src/semantic/analyzer.rs` and `src/semantic/class_resolver.rs`

Once interner is integrated:
```rust
// Before
Type::Class(class_name.clone())

// After
Type::Class(self.interner.intern(&class_name))
```

---

## Implementation Order

### Day 1: String Interning Foundation
1. [ ] Create `src/context.rs` with `CompilerContext`
2. [ ] Update `src/lib.rs` exports
3. [ ] Modify `Lexer` to accept interner
4. [ ] Update `src/cli/commands.rs` for compile command
5. [ ] Run tests, fix any breakage

### Day 2: Complete Interning + Start unwrap() Fixes
1. [ ] Update run, check, repl commands
2. [ ] Update all lexer tests
3. [ ] Fix 6 package manager unwrap() calls
4. [ ] Fix 3 debug interpreter unwrap() calls
5. [ ] Fix debug adapter unwrap() call

### Day 3: unwrap() Completion + Start clone() Fixes
1. [ ] Fix LLVM codegen writeln! unwrap() calls (20 lines)
2. [ ] Fix RwLock operations in interner
3. [ ] Fix LSP and misc unwrap() calls
4. [ ] Start parser token cloning fixes
5. [ ] Start type system Rc<Type> changes

### Day 4: clone() Completion + Testing
1. [ ] Complete parser token reference changes
2. [ ] Complete type system changes
3. [ ] Fix generics Cow<Type> changes
4. [ ] Fix IR builder scope stack
5. [ ] Fix IR optimization passes
6. [ ] Run full test suite
7. [ ] Run benchmarks to verify improvement

---

## Verification Checklist

After implementation, verify:

```bash
# All tests pass
cargo test

# No warnings
cargo clippy

# Count unwrap() - should be <100
grep -r "\.unwrap()" src --include="*.rs" | grep -v "_tests.rs" | grep -v "test" | wc -l

# Count clone() - should be <350
grep -r "\.clone()" src --include="*.rs" | grep -v "_tests.rs" | wc -l

# Benchmarks show improvement
cargo bench --bench lexer
cargo bench --bench parser
cargo bench --bench semantic
```

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Lifetime issues with interner | Start with `'static` lifetime or `Rc<RefCell>` if needed |
| Breaking API changes | Maintain backward-compatible `new()` that creates internal interner |
| Performance regression | Benchmark before/after each major change |
| Test failures | Run tests after each step, not just at end |

---

## Success Criteria

| Metric | Before | Target | Verified |
|--------|--------|--------|----------|
| String interner usage | 0 call sites | Lexer + Analyzer | [ ] |
| unwrap() count | 522 | <100 | [ ] |
| clone() count | 500 | <350 | [ ] |
| Tests passing | 1060 | 1060 | [ ] |
| Clippy warnings | 0 | 0 | [ ] |
