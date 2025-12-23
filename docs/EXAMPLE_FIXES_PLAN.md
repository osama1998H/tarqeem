# Plan: Fix Example Issues and Compiler Bugs

## Executive Summary

After running all examples manually and researching the codebase, we identified **6 distinct issues** affecting example programs. This plan outlines fixes for both the compiler/runtime and the example files themselves.

| Issue | Type | Priority | Effort |
|-------|------|----------|--------|
| 1. Increment operator (++) fails for global variables | Compiler Bug | HIGH | Medium |
| 2. Method calls on class instances fail | Compiler Bug | HIGH | High |
| 3. Missing trq_int_to_string in interpreter | Compiler Bug | HIGH | Low |
| 4. Builtin function name collision | Design + Example | MEDIUM | Low |
| 5. Stdlib classes require imports | Example Fix | LOW | Low |
| 6. Main function name mismatch | Compiler + Example | MEDIUM | Low |

---

## Issue 1: Increment Operator (++) Fails for Global Variables

### Problem
```tarqeem
متغير عداد = 0;
طالما (عداد < 5) {
    اطبع(عداد);
    عداد++;  // ERROR: Cannot modify undefined variable 'عداد'
}
```

### Root Cause
**File**: `src/ir/builder.rs`, lines 1935-1955

The `build_increment()` function only checks local variables via `lookup_var()`, but doesn't check `self.global_variables` like `build_assignment()` and `build_compound_assignment()` do.

### Solution

**Step 1**: Modify `build_increment()` in `src/ir/builder.rs` to check both local and global variables:

```rust
fn build_increment(
    &mut self,
    operand: &Expr,
    is_increment: bool,
    is_prefix: bool,
) -> Result<VarId> {
    let ptr = match &operand.kind {
        ExprKind::Identifier(name) => {
            // Check local variables first
            if let Some(ptr) = self.lookup_var(name) {
                Some((ptr, false)) // false = is local
            } else if self.global_variables.contains(name) {
                None // Will handle as global
            } else {
                return Err(IrError::new(
                    format!("Cannot modify undefined variable '{}'", name),
                    format!("لا يمكن تعديل متغير غير معرّف '{}'", name),
                ));
            }
        }
        // ... rest of match arms
    };

    // Handle local vs global increment differently
    // For globals, use GlobalLoad/GlobalStore pattern
```

**Step 2**: Add global variable handling similar to `build_compound_assignment()`:
- Use `Instruction::GlobalLoad` to load the current value
- Perform the increment/decrement
- Use `Instruction::GlobalStore` to store the result

### Files to Modify
- `src/ir/builder.rs`: `build_increment()` function (lines 1935-2012)

### Test Cases to Add
```rust
#[test]
fn test_global_increment() {
    let source = r#"
        متغير س = 0;
        س++;
        اطبع(س);
    "#;
    // Should output: 1
}

#[test]
fn test_global_increment_in_while() {
    let source = r#"
        متغير عداد = 0;
        طالما (عداد < 3) {
            اطبع(عداد);
            عداد++;
        }
    "#;
    // Should output: 0, 1, 2
}
```

---

## Issue 2: Method Calls on Class Instances Fail

### Problem
```tarqeem
صنف شخص {
    عام دالة اطبع_معلومات() {
        اطبع("معلومات");
    }
}
متغير شخص١ = جديد شخص();
شخص١.اطبع_معلومات();  // ERROR: Undefined function: ::اطبع_معلومات
```

### Root Cause
**File**: `src/ir/builder.rs`, lines 2062-2150

When building a method call:
1. `infer_expr_type(object)` returns wrong type (e.g., `Ptr(Void)` instead of `Ptr(Struct("شخص"))`)
2. The ClassId extraction fails because the type doesn't contain the class name
3. An empty ClassId is created, resulting in function name `::methodName` instead of `ClassName::methodName`

The type inference fails because:
- Variable types are stored in `var_types` map
- When looking up a variable in `infer_expr_type()`, if VarId is not found in `var_types`, it defaults to `Ptr(Void)`

### Solution

**Step 1**: Fix type tracking in `build_var_decl()` (lines 628-683):
- Ensure that when a variable is initialized with `جديد ClassName()`, the correct type `Ptr(Struct(ClassName))` is stored in `var_types`

**Step 2**: Fix `infer_expr_type()` for `ExprKind::New`:
- Should return `Ptr(Struct(ClassName))` for new expressions

**Step 3**: Fix class ID extraction in `build_call()`:
- Add better error handling when class_id extraction fails
- Log or warn when falling back to empty ClassId

**Step 4**: Ensure `var_types` is populated correctly:
```rust
// In build_var_decl, when handling New expression:
if let ExprKind::New { class_name, .. } = &init_expr.kind {
    let class_type = IrType::Ptr(Box::new(IrType::Struct(ClassId(class_name.clone()))));
    self.var_types.insert(var_id, class_type);
}
```

### Files to Modify
- `src/ir/builder.rs`:
  - `build_var_decl()` - lines 628-683
  - `infer_expr_type()` - lines 686-816
  - `build_call()` - lines 2062-2150

### Test Cases to Add
```rust
#[test]
fn test_method_call_on_instance() {
    let source = r#"
        صنف اختبار {
            عام دالة قل_مرحبا() {
                اطبع("مرحبا");
            }
        }
        متغير ا = جديد اختبار();
        ا.قل_مرحبا();
    "#;
    // Should output: مرحبا
}
```

---

## Issue 3: Missing trq_int_to_string in Interpreter

### Problem
```tarqeem
اطبع("الجيل رقم: " + رقم_الجيل);  // ERROR: Undefined function: trq_int_to_string
```

### Root Cause
**File**: `src/interpreter/executor.rs`

The IR builder generates calls to `trq_int_to_string`, `trq_float_to_string`, and `trq_bool_to_string` for type coercion during string concatenation. These functions exist in the C runtime (`runtime/string.c`) for compiled code, but are **missing from the interpreter's builtin handler**.

### Solution

**Step 1**: Add the three conversion functions to `call_builtin()` in `src/interpreter/executor.rs`:

```rust
// Add after line ~1939 in the match statement:
"trq_int_to_string" => {
    let val = args.get(0).ok_or_else(|| RuntimeError::argument_error("trq_int_to_string", 1, 0))?;
    match val {
        Value::Int(n) => Ok(Value::String(n.to_string())),
        _ => Err(RuntimeError::type_error("int", val.type_name())),
    }
}
"trq_float_to_string" => {
    let val = args.get(0).ok_or_else(|| RuntimeError::argument_error("trq_float_to_string", 1, 0))?;
    match val {
        Value::Float(f) => Ok(Value::String(f.to_string())),
        Value::Int(n) => Ok(Value::String((*n as f64).to_string())),
        _ => Err(RuntimeError::type_error("float", val.type_name())),
    }
}
"trq_bool_to_string" => {
    let val = args.get(0).ok_or_else(|| RuntimeError::argument_error("trq_bool_to_string", 1, 0))?;
    match val {
        Value::Bool(b) => Ok(Value::String(if *b { "صحيح".to_string() } else { "خطأ".to_string() })),
        _ => Err(RuntimeError::type_error("bool", val.type_name())),
    }
}
```

### Files to Modify
- `src/interpreter/executor.rs`: Add three new match arms in `call_builtin()`

### Test Cases to Add
```rust
#[test]
fn test_int_to_string_concat() {
    let source = r#"
        متغير س = 42;
        اطبع("العدد: " + س);
    "#;
    // Should output: العدد: 42
}
```

---

## Issue 4: Builtin Function Name Collision

### Problem
```tarqeem
دالة عاملي(ن: عدد) -> عدد {  // ERROR: Function 'عاملي' is already defined
    // ...
}
```

### Root Cause
**File**: `src/semantic/scope.rs`

Builtin functions are registered in the global scope during initialization. User-defined functions with the same name cause a "duplicate definition" error because `Scope::define()` doesn't allow overwriting existing symbols.

### Analysis
This could be considered either:
1. **By Design**: Prevents accidental shadowing of important builtins
2. **A Bug**: Inconsistent with variable shadowing which IS allowed

### Solution (Two Options)

**Option A - Fix Examples Only (Recommended for now)**:
Rename conflicting functions in example files to avoid collisions.

**Option B - Allow Function Shadowing**:
Modify `Scope::define()` to allow user functions to shadow builtins (more complex, requires careful consideration).

### Example File Fixes (Option A)

**File**: `examples/اختبار_بسيط.ترقيم`
- Rename `عاملي` to `عاملي_محلي` or `احسب_عاملي`

```tarqeem
// Before:
دالة عاملي(ن: عدد) -> عدد { ... }

// After:
دالة احسب_عاملي(ن: عدد) -> عدد { ... }
```

### Files to Modify
- `examples/اختبار_بسيط.ترقيم`: Rename `عاملي` function

---

## Issue 5: Stdlib Classes Require Imports

### Problem
```tarqeem
متغير قائمتي = جديد قائمة<عدد>()  // ERROR: Unknown class 'قائمة'
```

### Root Cause
Stdlib classes are NOT auto-imported. Users must explicitly import them:
```tarqeem
استورد { قائمة } من "مجموعات"
```

Additionally, there's a secondary bug: imported classes are added to scope but NOT to `class_resolver`, which is needed for `جديد` expressions.

### Solution

**Step 1 - Fix Example File**:
Add proper imports to `examples/اختبار_مجموعات.ترقيم`:

```tarqeem
بسم_الله

استورد { قائمة } من "مجموعات"
استورد { مكدس } من "مجموعات"
استورد { طابور } من "مجموعات"

// Rest of the code...
```

**Step 2 - Fix Typo**:
Line 80: Change `اطبار_قائمة()` to `اختبار_قائمة()`

**Step 3 - (Future) Fix Import Handler**:
In `src/semantic/analyzer.rs`, when importing a class, also register it in `class_resolver`.

### Files to Modify
- `examples/اختبار_مجموعات.ترقيم`: Add imports and fix typo
- `src/semantic/analyzer.rs` (future): Register imported classes in class_resolver

---

## Issue 6: Main Function Name Mismatch

### Problem
```tarqeem
دالة رئيسي() {  // This is NOT recognized as main
    // ...
}
```
Results in: `Type error: expected comparable, got null`

### Root Cause
**File**: `src/interpreter/executor.rs`, line 121

The interpreter searches for main functions with these names:
```rust
let main_names = ["__main__", "main", "رئيسية", "البداية"];
```

But `رئيسي` (masculine form) is NOT in the list. When not found, it falls back to executing the first function in the module, which may require parameters.

### Solution

**Step 1 - Add رئيسي to main function names**:
```rust
let main_names = ["__main__", "main", "رئيسي", "رئيسية", "البداية"];
```

**Step 2 - Fix Example File**:
Either rename function or add explicit call:

```tarqeem
// Option A: Rename to recognized name
دالة رئيسية() {
    // ...
}

// Option B: Add explicit call at end of file
رئيسي();
```

**Step 3 - Improve Fallback Behavior**:
When falling back to first function, check if it takes parameters:
```rust
if let Some(func) = self.module.functions.first() {
    if func.params.is_empty() {
        return Ok(func.id.clone());
    }
    // Don't execute functions that require parameters
}
```

### Files to Modify
- `src/interpreter/executor.rs`: Add `رئيسي` to main_names array
- `examples/حاسبة/اختبارات/test.trq`: Add `رئيسي();` at end or rename function

---

## Implementation Order

### Phase 1: Quick Wins (Low Effort, High Impact)
1. **Issue 3**: Add type conversion functions to interpreter (~30 lines)
2. **Issue 6**: Add `رئيسي` to main function names (~1 line)
3. **Issue 4**: Fix example file - rename `عاملي` function
4. **Issue 5**: Fix example file - add imports and fix typo

### Phase 2: Medium Effort Fixes
5. **Issue 1**: Fix increment operator for global variables (~50-100 lines)

### Phase 3: Complex Fixes
6. **Issue 2**: Fix method call type inference (~100-200 lines, needs careful testing)

---

## Testing Strategy

After each fix:
1. Run `cargo test` to ensure no regressions
2. Run the specific example that was failing
3. Run all examples to verify no new issues

### Example Test Commands
```bash
# Run all tests
cargo test

# Run specific example
./target/release/tarqeem run examples/تحكم.ترقيم
./target/release/tarqeem run examples/صنف.ترقيم
./target/release/tarqeem run examples/لعبة_الحياة.ترقيم

# Check syntax only
./target/release/tarqeem check examples/اختبار_بسيط.ترقيم
```

---

## Summary

| Issue | Fix Location | Complexity | Example Status After Fix |
|-------|--------------|------------|-------------------------|
| 1. Increment (++) | src/ir/builder.rs | Medium | تحكم.ترقيم will work |
| 2. Method calls | src/ir/builder.rs | High | صنف.ترقيم will work |
| 3. Int to string | src/interpreter/executor.rs | Low | لعبة_الحياة.ترقيم will work |
| 4. Name collision | examples/اختبار_بسيط.ترقيم | Low | اختبار_بسيط.ترقيم will work |
| 5. Stdlib imports | examples/اختبار_مجموعات.ترقيم | Low | اختبار_مجموعات.ترقيم will work |
| 6. Main function | src/interpreter/executor.rs + example | Low | test.trq will work |

**Expected Results After All Fixes**:
- 11/12 examples will pass (up from 5/12)
- اختبار_مجموعات.ترقيم may still need imported class registration fix
