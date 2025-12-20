# Phase 2 Fixes Implementation Plan

This document outlines the detailed implementation plan for fixing the remaining issues identified during Phase 2 validation.

## Issues Overview

| Issue | Priority | Complexity | Estimated Impact |
|-------|----------|------------|------------------|
| 1. IR Builder Type Tracking | HIGH | Medium | Fixes variable examples, enables other fixes |
| 2. ++/-- Operators | MEDIUM | Low | Fixes control flow examples |
| 3. OOP Instantiation | HIGH | Medium | Fixes class examples |
| 4. String + Number Coercion | MEDIUM | Low | Improves usability |

---

## Issue 1: IR Builder Type Tracking

### Problem
The IR builder doesn't preserve type information through variable operations. When loading a variable, it uses `IrType::Ptr(Box::new(IrType::Void))` instead of the actual type, causing LLVM IR type mismatches.

### Root Cause
The `IrBuilder` struct has no mechanism to track types for allocated variables. When `build_var_decl` creates an `Alloca` instruction, it knows the type, but when `build_identifier` loads from that variable, the type information is lost.

### Solution

#### Step 1.1: Add `var_types` HashMap to IrBuilder
**File:** `src/ir/builder.rs` (struct definition ~line 45-75)

```rust
pub struct IrBuilder {
    // ... existing fields ...

    /// Variable type tracking - maps VarId to its IrType
    var_types: HashMap<VarId, IrType>,  // NEW FIELD
}
```

Initialize in `new()`:
```rust
var_types: HashMap::new(),
```

#### Step 1.2: Update `build_var_decl` to track types
**File:** `src/ir/builder.rs` (~line 413-440)

After creating the Alloca instruction:
```rust
// Track the type for this variable
self.var_types.insert(ptr, ir_type.clone());
```

#### Step 1.3: Add type inference helper
**File:** `src/ir/builder.rs` (new method)

```rust
/// Infer IR type from an expression
fn infer_ir_type(&self, expr: &Expr) -> IrType {
    match &expr.kind {
        ExprKind::Literal(lit) => match lit {
            Literal::Int(_) => IrType::Int,
            Literal::Float(_) => IrType::Float,
            Literal::String(_) => IrType::String,
            Literal::Bool(_) => IrType::Bool,
            Literal::Null => IrType::Ptr(Box::new(IrType::Void)),
        },
        ExprKind::Binary { op, .. } => match op {
            // Comparison operators return Bool
            BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt |
            BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq |
            BinaryOp::And | BinaryOp::Or => IrType::Bool,
            _ => IrType::Int,
        },
        ExprKind::New { class, .. } => {
            if let ExprKind::Identifier(name) = &class.kind {
                IrType::Struct(ClassId(name.clone()))
            } else {
                IrType::Ptr(Box::new(IrType::Void))
            }
        },
        _ => IrType::Ptr(Box::new(IrType::Void)),
    }
}
```

#### Step 1.4: Update `build_identifier` to use tracked types
**File:** `src/ir/builder.rs` (~line 1240-1266)

```rust
fn build_identifier(&mut self, name: &str) -> Result<VarId> {
    if let Some(var_ptr) = self.lookup_var(name) {
        // Get the actual type from tracking
        let var_type = self.var_types.get(&var_ptr)
            .cloned()
            .unwrap_or(IrType::Ptr(Box::new(IrType::Void)));

        let dest = self.new_var();
        self.emit(Instruction::Load {
            dest,
            ptr: var_ptr,
            ty: var_type.clone(),
        });

        // Track the loaded value's type
        self.var_types.insert(dest, var_type);
        Ok(dest)
    } else {
        // ... existing error handling ...
    }
}
```

#### Step 1.5: Update `build_literal` to track types
**File:** `src/ir/builder.rs` (~line 1222-1237)

Add after emitting the Const instruction:
```rust
self.var_types.insert(dest, ty);
```

#### Step 1.6: Update `build_binary` to track result types
**File:** `src/ir/builder.rs` (~line 1269-1307)

After emitting the Binary instruction:
```rust
// Determine result type based on operation
let result_ty = match ir_op {
    BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le |
    BinaryOp::Gt | BinaryOp::Ge | BinaryOp::And | BinaryOp::Or => IrType::Bool,
    _ => {
        // Use left operand type for arithmetic
        self.var_types.get(&left_var).cloned().unwrap_or(IrType::Int)
    }
};
self.var_types.insert(dest, result_ty);
```

### Test Cases
```rust
#[test]
fn test_type_tracking_int() {
    let source = "متغير س = 5\nمتغير ص = س + 1\nاطبع(ص)";
    // Should compile without type errors
}

#[test]
fn test_type_tracking_string() {
    let source = "متغير اسم = \"أحمد\"\nاطبع(اسم)";
    // Should print string, not garbage
}
```

---

## Issue 2: ++/-- Operators

### Problem
The increment/decrement operators are parsed but:
1. Not in the precedence table for postfix position
2. Don't store the updated value back to the variable

### Solution

#### Step 2.1: Add to precedence table
**File:** `src/parser/precedence.rs` (~line 24-57)

```rust
impl Precedence {
    pub fn of(kind: &TokenKind) -> Precedence {
        match kind {
            // ... existing cases ...

            // Postfix operators at high precedence
            TokenKind::PlusPlus | TokenKind::MinusMinus => Precedence::Call,

            // ... rest of cases ...
        }
    }
}
```

#### Step 2.2: Implement proper increment/decrement in IR builder
**File:** `src/ir/builder.rs` (~line 1310-1370)

Replace the existing unary handling with:

```rust
fn build_unary(&mut self, op: AstUnaryOp, operand: &Expr) -> Result<VarId> {
    match op {
        AstUnaryOp::Neg => { /* existing */ }
        AstUnaryOp::Not => { /* existing */ }
        AstUnaryOp::PreInc => self.build_increment(operand, true, true),
        AstUnaryOp::PreDec => self.build_increment(operand, false, true),
        AstUnaryOp::PostInc => self.build_increment(operand, true, false),
        AstUnaryOp::PostDec => self.build_increment(operand, false, false),
    }
}

/// Build increment/decrement with store-back
fn build_increment(&mut self, operand: &Expr, is_increment: bool, is_prefix: bool) -> Result<VarId> {
    // Get variable pointer
    let ptr = match &operand.kind {
        ExprKind::Identifier(name) => {
            self.lookup_var(name).ok_or_else(|| IrError::new(
                format!("Cannot modify undefined variable '{}'", name),
                format!("لا يمكن تعديل متغير غير معرّف '{}'", name),
            ))?
        }
        _ => return Err(IrError::new(
            "Increment/decrement requires a variable",
            "الزيادة/النقصان تتطلب متغيراً",
        )),
    };

    // Load current value
    let var_type = self.var_types.get(&ptr).cloned().unwrap_or(IrType::Int);
    let old_val = self.new_var();
    self.emit(Instruction::Load { dest: old_val, ptr, ty: var_type.clone() });
    self.var_types.insert(old_val, var_type.clone());

    // Create constant 1
    let one = self.new_var();
    self.emit(Instruction::Const { dest: one, value: Constant::Int(1), ty: IrType::Int });

    // Compute new value: old_val +/- 1
    let new_val = self.new_var();
    let op = if is_increment { BinaryOp::Add } else { BinaryOp::Sub };
    self.emit(Instruction::Binary {
        dest: new_val, op, left: old_val, right: one, ty: IrType::Int,
    });
    self.var_types.insert(new_val, IrType::Int);

    // Store new value back
    self.emit(Instruction::Store { ptr, value: new_val });

    // Return appropriate value
    Ok(if is_prefix { new_val } else { old_val })
}
```

### Test Cases
```rust
#[test]
fn test_prefix_increment() {
    let source = "متغير س = 5\nمتغير ص = ++س";
    // س = 6, ص = 6
}

#[test]
fn test_postfix_increment() {
    let source = "متغير س = 5\nمتغير ص = س++";
    // س = 6, ص = 5
}

#[test]
fn test_decrement_in_loop() {
    let source = "متغير عداد = 10\nطالما (عداد > 0) { عداد--; }";
}
```

---

## Issue 3: OOP Instantiation (`جديد`)

### Problem
When handling `جديد ClassName(args)`, the semantic analyzer calls `infer_type(class)` which looks up `ClassName` as a variable, not as a class name.

### Solution

#### Step 3.1: Rewrite `ExprKind::New` handler
**File:** `src/semantic/analyzer.rs` (~line 1060-1067)

```rust
ExprKind::New { class, args } => {
    // Extract class name from identifier
    let class_name = match &class.kind {
        ExprKind::Identifier(name) => name.clone(),
        _ => {
            self.error(
                "New expression requires a class name",
                "تعبير جديد يتطلب اسم صنف",
                class.span,
            );
            return Type::Error;
        }
    };

    // Check class exists in class resolver
    if let Some(class_info) = self.class_resolver.get_class(&class_name) {
        // Validate constructor arguments
        if let Some(ref ctor) = class_info.constructor {
            let expected_params = &ctor.params;

            // Check argument count
            if args.len() != expected_params.len() {
                self.error(
                    &format!(
                        "Constructor expects {} arguments, got {}",
                        expected_params.len(), args.len()
                    ),
                    &format!(
                        "المنشئ يتوقع {} معاملات، وُجد {}",
                        expected_params.len(), args.len()
                    ),
                    expr.span,
                );
            }

            // Type-check each argument
            for (arg, (_, param_type)) in args.iter().zip(expected_params.iter()) {
                let arg_type = self.infer_type(arg);
                if !arg_type.is_compatible_with(param_type) {
                    self.error(
                        &format!("Wrong argument type: expected {}, got {}", param_type, arg_type),
                        &format!("نوع المعامل خاطئ: متوقع {}، وُجد {}",
                            param_type.arabic_name(), arg_type.arabic_name()),
                        arg.span,
                    );
                }
            }
        } else if !args.is_empty() {
            self.error(
                &format!("Class '{}' has no constructor", class_name),
                &format!("الصنف '{}' ليس له منشئ", class_name),
                expr.span,
            );
        }

        Type::Class(class_name)
    } else {
        self.error(
            &format!("Unknown class '{}'", class_name),
            &format!("صنف غير معروف '{}'", class_name),
            class.span,
        );
        Type::Error
    }
}
```

#### Step 3.2: Add `get_class` method to ClassResolver (if not exists)
**File:** `src/semantic/class_resolver.rs`

```rust
pub fn get_class(&self, name: &str) -> Option<&ClassInfo> {
    self.classes.get(name)
}
```

### Test Cases
```rust
#[test]
fn test_class_instantiation_valid() {
    let source = r#"
        صنف شخص {
            منشئ(اسم: نص) {}
        }
        متغير ش = جديد شخص("أحمد")
    "#;
    // Should compile
}

#[test]
fn test_class_instantiation_wrong_args() {
    let source = r#"
        صنف شخص {
            منشئ(اسم: نص) {}
        }
        متغير ش = جديد شخص(42)
    "#;
    // Should produce type error
}

#[test]
fn test_class_instantiation_unknown_class() {
    let source = "متغير ش = جديد غيرموجود()";
    // Should produce "unknown class" error
}
```

---

## Issue 4: String + Number Concatenation

### Problem
Only `String + String` is allowed. We need automatic type coercion for:
- `String + Int` → `String`
- `String + Float` → `String`
- `Int + String` → `String`
- etc.

### Solution

#### Step 4.1: Extend type rules
**File:** `src/semantic/types.rs` (~line 122-173)

Add to `binary_result_type`:
```rust
// String concatenation with implicit conversion
(Type::String, "+", Type::Int) => Some(Type::String),
(Type::String, "+", Type::Float) => Some(Type::String),
(Type::String, "+", Type::Bool) => Some(Type::String),
(Type::Int, "+", Type::String) => Some(Type::String),
(Type::Float, "+", Type::String) => Some(Type::String),
(Type::Bool, "+", Type::String) => Some(Type::String),
```

#### Step 4.2: Add type coercion in IR builder
**File:** `src/ir/builder.rs` (in `build_binary`)

```rust
fn build_binary(&mut self, left: &Expr, op: AstBinaryOp, right: &Expr) -> Result<VarId> {
    let left_var = self.build_expr(left)?;
    let right_var = self.build_expr(right)?;

    let left_ty = self.var_types.get(&left_var).cloned().unwrap_or(IrType::Int);
    let right_ty = self.var_types.get(&right_var).cloned().unwrap_or(IrType::Int);

    // Handle string concatenation with type coercion
    if matches!(op, AstBinaryOp::Add) {
        let is_left_string = matches!(left_ty, IrType::String);
        let is_right_string = matches!(right_ty, IrType::String);

        if is_left_string || is_right_string {
            let left_str = if is_left_string {
                left_var
            } else {
                self.convert_to_string(left_var, &left_ty)?
            };

            let right_str = if is_right_string {
                right_var
            } else {
                self.convert_to_string(right_var, &right_ty)?
            };

            let dest = self.new_var();
            self.emit(Instruction::StringConcat { dest, left: left_str, right: right_str });
            self.var_types.insert(dest, IrType::String);
            return Ok(dest);
        }
    }

    // ... rest of existing handling ...
}

/// Convert value to string for concatenation
fn convert_to_string(&mut self, var: VarId, ty: &IrType) -> Result<VarId> {
    let dest = self.new_var();
    match ty {
        IrType::Int => self.emit(Instruction::Call {
            dest: Some(dest),
            func: FuncId("__int_to_string".to_string()),
            args: vec![var],
        }),
        IrType::Float => self.emit(Instruction::Call {
            dest: Some(dest),
            func: FuncId("__float_to_string".to_string()),
            args: vec![var],
        }),
        IrType::Bool => self.emit(Instruction::Call {
            dest: Some(dest),
            func: FuncId("__bool_to_string".to_string()),
            args: vec![var],
        }),
        _ => self.emit(Instruction::Call {
            dest: Some(dest),
            func: FuncId("__to_string".to_string()),
            args: vec![var],
        }),
    }
    self.var_types.insert(dest, IrType::String);
    Ok(dest)
}
```

#### Step 4.3: Handle in LLVM codegen
**File:** `src/codegen/llvm/codegen.rs`

The runtime already has `trq_int_to_string`, `trq_float_to_string`, `trq_bool_to_string` - just need to emit the correct calls.

### Test Cases
```rust
#[test]
fn test_string_int_concat() {
    let source = r#"متغير س = "العدد: " + 42"#;
    // Should produce "العدد: 42"
}

#[test]
fn test_int_string_concat() {
    let source = r#"متغير س = 42 + " هو الجواب""#;
    // Should produce "42 هو الجواب"
}

#[test]
fn test_string_float_concat() {
    let source = r#"متغير س = "باي = " + 3.14"#;
    // Should produce "باي = 3.14"
}
```

---

## Implementation Order

```
┌─────────────────────────────────────────────────────────────┐
│                    Phase 2 Fixes                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ┌─────────────────────────────────┐                       │
│   │ Issue 1: Type Tracking          │ ◄─── Foundation       │
│   │ - Add var_types HashMap         │                       │
│   │ - Track in all expr builders    │                       │
│   └─────────────────┬───────────────┘                       │
│                     │                                       │
│         ┌───────────┴───────────┐                           │
│         ▼                       ▼                           │
│   ┌─────────────────┐   ┌─────────────────┐                 │
│   │ Issue 2: ++/--  │   │ Issue 4: String │                 │
│   │ - Precedence    │   │ Coercion        │                 │
│   │ - Store-back    │   │ - Type rules    │                 │
│   └────────┬────────┘   │ - IR conversion │                 │
│            │            └────────┬────────┘                 │
│            │                     │                          │
│            └──────────┬──────────┘                          │
│                       ▼                                     │
│              ┌─────────────────┐                            │
│              │ Issue 3: OOP    │                            │
│              │ Instantiation   │                            │
│              │ (Independent)   │                            │
│              └─────────────────┘                            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Recommended sequence:**
1. **Issue 1** (Type Tracking) - 2-3 hours - Foundation for other fixes
2. **Issue 4** (String Coercion) - 1-2 hours - Depends on type tracking
3. **Issue 2** (++/-- Operators) - 1-2 hours - Uses type tracking
4. **Issue 3** (OOP Instantiation) - 1-2 hours - Independent semantic fix

---

## Files Summary

| File | Issues Affected | Changes |
|------|-----------------|---------|
| `src/ir/builder.rs` | 1, 2, 4 | var_types HashMap, increment, string coercion |
| `src/semantic/types.rs` | 4 | String + primitive type rules |
| `src/semantic/analyzer.rs` | 3 | New expression handling |
| `src/parser/precedence.rs` | 2 | ++/-- in precedence table |
| `src/codegen/llvm/codegen.rs` | 4 | String conversion calls |

---

## Success Criteria

After implementing these fixes:

1. ✅ `examples/متغيرات.trq` compiles and runs correctly
2. ✅ `examples/تحكم.trq` compiles (uses ++/--)
3. ✅ `examples/صنف.trq` compiles (uses classes)
4. ✅ `"text" + 42` produces `"text42"`
5. ✅ All 84+ tests continue to pass
