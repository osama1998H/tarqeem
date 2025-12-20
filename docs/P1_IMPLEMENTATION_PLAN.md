# P1 Features Implementation Plan

**Date**: 2025-12-20
**Branch**: `claude/fix-stress-test-issues-XZuAG`
**Goal**: Complete array operations and enable full Game of Life stress test

---

## Executive Summary

Based on codebase analysis, 3 of 4 P1 features are **partially implemented** but have bugs preventing them from working. The fixes are relatively small (~100 lines total).

| Feature | Current State | Effort | Priority |
|---------|---------------|--------|----------|
| Empty Array Type Inference | 50% | ~30 lines | P1.1 |
| Array Indexing | 80% | ~20 lines | P1.2 |
| For-In Iteration | 90% | ~10 lines | P1.3 |
| Super() Calls | 10% | ~50 lines | P2 (defer) |

---

## Phase 1: Empty Array Type Inference (P1.1)

### Problem

```tarqeem
متغير شبكة: مصفوفة<منطقي> = [];
// Error: عدم تطابق الأنواع: متوقع مصفوفة، وُجد مصفوفة<مجهول>
```

The semantic analyzer doesn't use the declared type annotation to infer the element type of empty arrays.

### Root Cause

**File**: `src/semantic/analyzer.rs` (lines 993-1016)

```rust
ExprKind::Array(elements) => {
    if elements.is_empty() {
        Type::Array(Box::new(Type::Unknown))  // ← Always Unknown!
    } else {
        // ... infer from first element
    }
}
```

### Solution

Add "expected type" context propagation to the type inference system.

#### Step 1: Add expected_type field to SemanticAnalyzer

```rust
pub struct SemanticAnalyzer {
    // ... existing fields ...

    /// Expected type for context-aware inference (e.g., for empty arrays)
    expected_type: Option<Type>,
}
```

#### Step 2: Set expected type before checking initializers

In `check_var_decl()` or equivalent:

```rust
fn check_var_decl(&mut self, name: &str, ty: Option<&TypeAnnotation>, init: &Expr) {
    // Set expected type from declaration
    if let Some(declared_ty) = ty {
        self.expected_type = Some(self.resolve_type(declared_ty));
    }

    let inferred = self.infer_type(init);

    // Clear expected type
    self.expected_type = None;

    // ... rest of type checking
}
```

#### Step 3: Use expected type for empty arrays

```rust
ExprKind::Array(elements) => {
    if elements.is_empty() {
        // Check for expected type from context
        if let Some(Type::Array(elem_ty)) = &self.expected_type {
            Type::Array(elem_ty.clone())
        } else {
            Type::Array(Box::new(Type::Unknown))
        }
    } else {
        // ... existing code
    }
}
```

### Verification

```tarqeem
متغير شبكة: مصفوفة<منطقي> = [];  // Should compile
متغير أرقام: مصفوفة<عدد> = [];    // Should compile
```

---

## Phase 2: Fix Array Indexing Type Inference (P1.2)

### Problem

Array indexing works but type inference is fragile when element type is unknown.

### Current Code

**File**: `src/ir/builder.rs` (lines 1955-1977)

```rust
fn build_index(&mut self, object: &Expr, index: &Expr) -> Result<VarId> {
    let obj_type = self.infer_expr_type(object)?;
    // ...
    let elem_ty = if let IrType::Array(elem, _) = &obj_type {
        (**elem).clone()
    } else {
        IrType::Ptr(Box::new(IrType::Void))  // ← Fallback loses type info
    };
    // ...
}
```

### Solution

The fix depends on P1.1 being complete. Once empty arrays have proper types, this code will work correctly. However, we should also handle the case where we're indexing a variable that was declared with a type annotation.

#### Enhancement: Check var_types for array element type

```rust
fn build_index(&mut self, object: &Expr, index: &Expr) -> Result<VarId> {
    let obj_var = self.build_expr(object)?;
    let idx_var = self.build_expr(index)?;

    // Try to get element type from var_types first
    let elem_ty = if let Some(arr_ty) = self.var_types.get(&obj_var.0) {
        if let IrType::Array(elem, _) = arr_ty {
            (**elem).clone()
        } else {
            self.infer_element_type_from_expr(object)
        }
    } else {
        self.infer_element_type_from_expr(object)
    };

    let dest = self.new_var();
    self.emit(Instruction::ArrayGet {
        dest,
        array: obj_var,
        index: idx_var,
        elem_ty: elem_ty.clone(),
    });

    self.var_types.insert(dest.0, elem_ty);
    Ok(dest)
}

fn infer_element_type_from_expr(&self, object: &Expr) -> IrType {
    match self.infer_expr_type(object) {
        Ok(IrType::Array(elem, _)) => (*elem).clone(),
        _ => IrType::Ptr(Box::new(IrType::Void))
    }
}
```

### Verification

```tarqeem
متغير أ: مصفوفة<عدد> = [1, 2, 3];
متغير أول = أ[0];  // Should infer أول as عدد
```

---

## Phase 3: Fix For-In Element Type (P1.3)

### Problem

For-in loops hardcode element type to void pointer.

### Current Code

**File**: `src/ir/builder.rs` (line 1186)

```rust
self.emit(Instruction::ArrayGet {
    dest: elem,
    array: array_var,
    index: index_val2,
    elem_ty: IrType::Ptr(Box::new(IrType::Void)),  // ← Wrong!
});
```

### Solution

Use the array's element type:

```rust
// Get element type from array
let elem_ty = if let Some(arr_ty) = self.var_types.get(&array_var.0) {
    if let IrType::Array(inner, _) = arr_ty {
        (**inner).clone()
    } else {
        IrType::Ptr(Box::new(IrType::Void))
    }
} else {
    // Try to infer from iterable expression
    match self.infer_expr_type(iterable) {
        Ok(IrType::Array(inner, _)) => (*inner).clone(),
        _ => IrType::Ptr(Box::new(IrType::Void))
    }
};

self.emit(Instruction::ArrayGet {
    dest: elem,
    array: array_var,
    index: index_val2,
    elem_ty: elem_ty.clone(),
});

// Track loop variable type
self.var_types.insert(elem.0, elem_ty);
```

### Verification

```tarqeem
متغير أرقام: مصفوفة<عدد> = [1, 2, 3];
لكل رقم في أرقام {
    اطبع(رقم * 2);  // Should work - رقم is عدد
}
```

---

## Phase 4: Super() Calls (P2 - Defer)

### Problem

`أساس(...)` currently just returns `this` instead of calling parent constructor.

### Current Code

**File**: `src/ir/builder.rs` (lines 2365-2375)

```rust
fn build_super(&mut self) -> Result<VarId> {
    if let Some(var) = self.lookup_var("هذا").or_else(|| self.lookup_var("this")) {
        Ok(var)  // ← Just returns this, doesn't call parent!
    }
    // ...
}
```

### Why Defer

This requires:
1. Class hierarchy tracking in IR builder
2. Parent class constructor resolution
3. vtable lookups for method calls
4. Changes to multiple files

**Recommendation**: Defer to P2 and focus on array features first.

### Future Implementation Outline

1. Track current class context in IR builder
2. In `build_call()`, detect when callee is `Super`
3. Look up parent class from class registry
4. Emit call to parent constructor/method with correct arguments
5. Handle `this` pointer adjustment for inheritance

---

## Implementation Order

```
┌─────────────────────────────────────────────────────────────┐
│ P1.1: Empty Array Type Inference                            │
│ File: src/semantic/analyzer.rs                              │
│ Est: 30 lines                                               │
├─────────────────────────────────────────────────────────────┤
│ P1.2: Array Indexing Type Fix                               │
│ File: src/ir/builder.rs                                     │
│ Est: 20 lines                                               │
│ Depends on: P1.1                                            │
├─────────────────────────────────────────────────────────────┤
│ P1.3: For-In Element Type Fix                               │
│ File: src/ir/builder.rs                                     │
│ Est: 10 lines                                               │
│ Depends on: P1.1                                            │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
    Test with لعبة_الحياة.trq
```

---

## Test Cases

### After P1.1 (Empty Array Inference)

```tarqeem
// test_empty_array.trq
متغير أرقام: مصفوفة<عدد> = [];
أرقام = [1, 2, 3];  // Should work
```

### After P1.2 (Array Indexing)

```tarqeem
// test_array_index.trq
متغير أ: مصفوفة<عدد> = [10, 20, 30];
متغير أول = أ[0];
اطبع(أول);  // Should print 10
```

### After P1.3 (For-In)

```tarqeem
// test_for_in.trq
متغير أرقام: مصفوفة<عدد> = [1, 2, 3];
لكل ر في أرقام {
    اطبع(ر);
}
```

### Full Integration (Game of Life)

```bash
cargo run -- compile examples/لعبة_الحياة.trq
./لعبة_الحياة
```

---

## Success Criteria

1. `cargo test` - All 84+ tests pass
2. `examples/لعبة_الحياة.trq` compiles without errors
3. Game of Life runs and shows expected output
4. No regressions in other examples

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Type inference changes break existing code | Run full test suite after each change |
| LLVM codegen issues with new array types | Test with --emit-llvm and verify IR |
| For-in type changes affect loop variable scope | Keep existing scope management |

---

## Files to Modify

| File | Changes |
|------|---------|
| `src/semantic/analyzer.rs` | Add expected_type field, use in array inference |
| `src/ir/builder.rs` | Fix element type in build_index and build_for_in |

**Total estimated: ~60 lines of changes**

---

## References

- `docs/STRESS_TEST_REPORT.md` - Original bug analysis
- `docs/STRESS_TEST_FIX_PLAN.md` - P0 implementation plan
- `docs/AI_NOTES.md` - Session log
