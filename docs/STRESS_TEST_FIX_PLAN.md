# Stress Test Issues Fix Plan

**Date**: 2025-12-20
**Branch**: `claude/fix-stress-test-issues-XZuAG`
**Goal**: Enable code execution by fixing critical bugs identified in the stress test

---

## Executive Summary

The stress test revealed that the Tarqeem compiler is ~50% complete. The frontend (lexer, parser, semantic analyzer) works well, but the backend (IR generation, LLVM codegen) has critical bugs preventing compilation and execution.

This plan addresses the issues in priority order to enable running the Conway's Game of Life stress test.

---

## Phase 1: Critical Fixes (P0) - Enables Compilation

### 1.1 Fix Empty Else Block Bug

**File**: `src/ir/builder.rs` (lines 822-885)

**Problem**: When an `if` statement has no `else` branch, an empty `else:` block is created in LLVM IR with no terminator instruction.

**Current behavior**:
```llvm
br i1 %cond, label %then, label %else
then:
  ret i64 %v4
else:           ; ← Empty block, no terminator!
merge:
  ret i64 %v0
```

**Root cause**: In `build_if()`, when `else_branch` is `None`, the code still creates an `else_block` but never adds a jump to it.

**Fix strategy**:
1. When `else_branch` is `None`, don't create an `else_block`
2. Branch directly to `merge_block` when condition is false
3. Only create `else_block` when there's actual else code

**Implementation**:
```rust
fn build_if(&mut self, condition: &Expr, then_branch: &Block, else_branch: Option<&Block>) -> Result<()> {
    let cond_var = self.build_expr(condition)?;
    let then_block = self.new_block(Some("then".to_string()));
    let merge_block = self.new_block(Some("merge".to_string()));

    // Only create else block if there's an else branch
    let else_target = if else_branch.is_some() {
        self.new_block(Some("else".to_string()))
    } else {
        merge_block  // Jump straight to merge
    };

    self.emit(Instruction::Branch {
        cond: cond_var,
        then_block,
        else_block: else_target,
    });

    // Build then block
    self.switch_to_block(then_block);
    self.build_block(then_branch)?;
    if !self.current_block_terminated() {
        self.emit(Instruction::Jump { target: merge_block });
    }

    // Build else block only if it exists
    if let Some(else_code) = else_branch {
        self.switch_to_block(else_target);
        self.build_block(else_code)?;
        if !self.current_block_terminated() {
            self.emit(Instruction::Jump { target: merge_block });
        }
    }

    self.switch_to_block(merge_block);
    Ok(())
}
```

**Verification**: `examples/اختبار_بسيط.trq` should compile without LLVM errors.

---

### 1.2 Implement Global Constants Visibility

**Files**:
- `src/ir/builder.rs` - Add globals to module
- `src/ir/instruction.rs` - Module already has `globals` field

**Problem**: Global constants declared at file level are not visible when referenced inside functions.

```tarqeem
ثابت عرض = 20;

دالة دالتي() {
    اطبع(عرض);  // Error: Undefined identifier: 'عرض'
}
```

**Root cause**:
1. IR builder never populates `module.globals`
2. Variable lookup only checks local scope

**Fix strategy**:
1. In `build_program()`, iterate over top-level statements
2. For `ConstDecl`, add to `module.globals` vector
3. In variable lookup, check globals after local scope fails
4. Generate LLVM global variable declarations

**Implementation steps**:

1. **Populate globals in `build_program()`**:
```rust
fn build_program(&mut self, program: &Program) -> Result<Module> {
    // First pass: collect global constants
    for stmt in &program.statements {
        if let StmtKind::ConstDecl { name, ty, init, .. } = &stmt.kind {
            let ir_type = self.convert_type(ty);
            let const_val = self.evaluate_const(init)?;
            self.module.globals.push((name.clone(), ir_type, Some(const_val)));
            self.global_symbols.insert(name.clone(), ir_type);
        }
    }

    // Second pass: build functions
    // ...
}
```

2. **Lookup globals in `get_var()`**:
```rust
fn get_var(&self, name: &str) -> Result<VarId> {
    // Check local scope first
    if let Some(id) = self.local_vars.get(name) {
        return Ok(*id);
    }
    // Check globals
    if let Some(ty) = self.global_symbols.get(name) {
        return Ok(self.emit_global_load(name, ty));
    }
    Err(format!("Undefined identifier: '{}'", name))
}
```

3. **Generate LLVM globals in codegen**:
```llvm
@عرض = constant i64 20
```

**Verification**: `examples/لعبة_الحياة_بسيط.trq` should compile.

---

## Phase 2: High Priority Fixes (P1) - Enables Complex Programs

### 2.1 Fix Type Mismatch in Returns

**File**: `src/codegen/llvm/codegen.rs`

**Problem**: Functions returning `float` generate incorrect LLVM types.

```llvm
ret i64 %v5   ; but %v5 is actually double!
```

**Fix**: Ensure return type matches the declared function signature.

```rust
fn emit_return(&mut self, value: Option<VarId>) -> Result<()> {
    if let Some(var) = value {
        let var_type = self.get_var_type(var);
        let ret_type = self.current_function_return_type();

        // Cast if necessary
        let final_val = if var_type != ret_type {
            self.emit_type_cast(var, var_type, ret_type)?
        } else {
            var
        };

        writeln!(self.output, "  ret {} {}", ret_type.llvm_name(), self.get_var(final_val))
    } else {
        writeln!(self.output, "  ret void")
    }
}
```

---

### 2.2 Implement Type Inference for Empty Arrays

**File**: `src/semantic/analyzer.rs`

**Problem**: Empty array literal `[]` is typed as `Array(Unknown)` instead of inferring from context.

```tarqeem
متغير شبكة: مصفوفة<منطقي> = [];
// Error: عدم تطابق الأنواع: متوقع مصفوفة، وُجد مصفوفة<مجهول>
```

**Fix strategy**: Use declared type annotation to infer array element type.

```rust
fn infer_type(&mut self, expr: &Expr) -> Type {
    match &expr.kind {
        ExprKind::Array(elements) if elements.is_empty() => {
            // Check for expected type from context
            if let Some(expected) = self.expected_type.take() {
                if let Type::Array(elem_ty) = expected {
                    return Type::Array(elem_ty);
                }
            }
            Type::Array(Box::new(Type::Unknown))
        }
        // ...
    }
}

fn check_var_decl(&mut self, name: &str, ty: Option<&Type>, init: &Expr) {
    // Set expected type before inferring
    if let Some(declared_ty) = ty {
        self.expected_type = Some(declared_ty.clone());
    }
    let inferred = self.infer_type(init);
    // ...
}
```

---

### 2.3 Array Indexing in IR Builder

**File**: `src/ir/builder.rs`

**Problem**: Array indexing (`arr[i]`) needs IR generation support.

**Fix**: Add `Index` expression handling in `build_expr()`:

```rust
ExprKind::Index { object, index } => {
    let array_var = self.build_expr(object)?;
    let index_var = self.build_expr(index)?;

    // Get element pointer
    let elem_ptr = self.new_var(IrType::Ptr);
    self.emit(Instruction::ArrayGet {
        dest: elem_ptr,
        array: array_var,
        index: index_var,
    });

    // Load element
    let elem_type = self.get_array_element_type(array_var);
    let result = self.new_var(elem_type);
    self.emit(Instruction::Load {
        dest: result,
        ptr: elem_ptr,
    });

    Ok(result)
}
```

Add corresponding LLVM codegen:
```rust
Instruction::ArrayGet { dest, array, index } => {
    // Call runtime: trq_array_get(arr, index)
    writeln!(
        self.output,
        "  {} = call ptr @trq_array_get(ptr {}, i64 {})",
        self.get_var(*dest),
        self.get_var(*array),
        self.get_var(*index)
    )
}
```

---

### 2.4 For-In Iteration Over Arrays

**Files**:
- `src/ir/builder.rs` - Lower for-in to indexed loop
- `src/semantic/analyzer.rs` - Type check iterables

**Problem**: `لكل صف في شبكة { ... }` fails with "cannot iterate over array".

**Fix strategy**: Desugar for-in to a traditional for loop:

```tarqeem
// This:
لكل عنصر في مصفوفة { ... }

// Becomes:
{
    متغير __len = trq_array_len(مصفوفة);
    لكل (متغير __i = 0؛ __i < __len؛ __i++) {
        متغير عنصر = مصفوفة[__i];
        ...
    }
}
```

**Implementation**:
```rust
fn build_for_in(&mut self, var: &str, iterable: &Expr, body: &Block) -> Result<()> {
    let array_var = self.build_expr(iterable)?;

    // Get array length
    let len_var = self.new_var(IrType::I64);
    self.emit(Instruction::Call {
        dest: Some(len_var),
        func: "trq_array_len".into(),
        args: vec![array_var],
    });

    // Create index variable
    let idx_var = self.new_var(IrType::I64);
    self.emit(Instruction::Const { dest: idx_var, value: Constant::Int(0) });

    // Loop header
    let loop_header = self.new_block(Some("for_in_header".into()));
    let loop_body = self.new_block(Some("for_in_body".into()));
    let loop_end = self.new_block(Some("for_in_end".into()));

    self.emit(Instruction::Jump { target: loop_header });
    self.switch_to_block(loop_header);

    // Check i < len
    let cmp_var = self.new_var(IrType::Bool);
    self.emit(Instruction::Lt { dest: cmp_var, left: idx_var, right: len_var });
    self.emit(Instruction::Branch {
        cond: cmp_var,
        then_block: loop_body,
        else_block: loop_end,
    });

    // Loop body
    self.switch_to_block(loop_body);

    // Get current element: var = array[i]
    let elem_ptr = self.build_array_index(array_var, idx_var)?;
    let elem_var = self.load(elem_ptr)?;
    self.define_local(var, elem_var);

    // Build user body
    self.build_block(body)?;

    // Increment index
    let one = self.new_var(IrType::I64);
    self.emit(Instruction::Const { dest: one, value: Constant::Int(1) });
    self.emit(Instruction::Add { dest: idx_var, left: idx_var, right: one });
    self.emit(Instruction::Jump { target: loop_header });

    self.switch_to_block(loop_end);
    Ok(())
}
```

---

## Phase 3: Testing & Verification

### 3.1 Test Files

After each fix, test progressively:

1. **P0 fixes complete**: `cargo run -- compile examples/اختبار_بسيط.trq` should succeed
2. **P1 fixes complete**: `cargo run -- compile examples/لعبة_الحياة_بسيط.trq` should succeed
3. **All fixes complete**: `cargo run -- compile examples/لعبة_الحياة.trq` should succeed

### 3.2 Unit Tests

Add regression tests for each fix:

```rust
#[test]
fn test_if_without_else_generates_valid_ir() {
    let source = r#"
        دالة اختبار(س: عدد) -> عدد {
            إذا (س > 0) {
                أرجع 1;
            }
            أرجع 0;
        }
    "#;
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
}

#[test]
fn test_global_constant_visible_in_function() {
    let source = r#"
        ثابت عدد_أقصى = 100;

        دالة احصل() -> عدد {
            أرجع عدد_أقصى;
        }
    "#;
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
}

#[test]
fn test_array_indexing() {
    let source = r#"
        دالة رئيسية() {
            متغير أ: مصفوفة<عدد> = [1، 2، 3];
            متغير أول = أ[0];
        }
    "#;
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
}
```

---

## Implementation Order

| Order | Task | File(s) | Est. Lines |
|-------|------|---------|------------|
| 1 | Fix empty else block | `src/ir/builder.rs` | ~20 |
| 2 | Global constants visibility | `src/ir/builder.rs`, `src/ir/instruction.rs` | ~50 |
| 3 | LLVM global emission | `src/codegen/llvm/codegen.rs` | ~30 |
| 4 | Fix return type mismatch | `src/codegen/llvm/codegen.rs` | ~15 |
| 5 | Empty array type inference | `src/semantic/analyzer.rs` | ~20 |
| 6 | Array indexing IR | `src/ir/builder.rs` | ~30 |
| 7 | For-in desugaring | `src/ir/builder.rs` | ~50 |
| 8 | Integration tests | `tests/` | ~100 |

**Total estimated**: ~315 lines of code changes

---

## Success Criteria

1. `cargo test` passes (all 84+ tests)
2. `cargo clippy` has no errors
3. `examples/اختبار_بسيط.trq` compiles and runs
4. `examples/لعبة_الحياة_بسيط.trq` compiles and runs
5. `examples/لعبة_الحياة.trq` compiles (may need additional array fixes)

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking existing tests | Run `cargo test` after each change |
| LLVM version differences | Test with the same LLVM version as CI |
| Unicode edge cases | Use existing NFC normalization |
| Performance regression | Benchmark after for-in implementation |

---

## References

- `docs/STRESS_TEST_REPORT.md` - Detailed bug analysis
- `docs/AI_NOTES.md` - Implementation history
- `ARCHITECTURE.md` - Compiler pipeline design
- `runtime/tarqeem_rt.h` - Runtime API
