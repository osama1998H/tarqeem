# Global Variables Implementation Plan

## Overview

This document outlines the implementation plan for adding full global variable support to Tarqeem. Currently, global constants are partially implemented (collected but inlined as constants), but mutable global variables and proper global storage are not supported in the LLVM codegen.

## Current State Analysis

### What Exists:
1. **Lexer**: `متغير`/`let` and `ثابت`/`const` tokens already defined
2. **Parser**: `VarDecl` AST node handles both mutable and immutable declarations
3. **Semantic**: Global scope exists, symbols can be defined at global level
4. **IR Module**: `globals: Vec<(String, IrType, Option<Constant>)>` field exists
5. **IR Builder**: Collects global constants in first pass (lines 119-141)
6. **Interpreter**: `init_globals()` properly initializes globals from `module.globals`

### What's Missing:
1. **IR Instructions**: No `GlobalLoad`/`GlobalStore` instructions
2. **IR Builder**: Doesn't populate `module.globals`, inlines constants instead
3. **IR Builder**: No tracking of which variables are global vs local
4. **LLVM Codegen**: No emission of global variable definitions
5. **LLVM Codegen**: No handling of global variable access

## Implementation Strategy

### Phase 1: IR Layer Changes

#### 1.1 Add New IR Instructions (`src/ir/instruction.rs`)

Add two new instructions for global variable access:

```rust
// In enum Instruction:

/// Load a value from a global variable
/// dest = load @global_name
GlobalLoad {
    dest: VarId,
    name: String,
    ty: IrType,
},

/// Store a value to a global variable
/// store value -> @global_name
GlobalStore {
    name: String,
    value: VarId,
},
```

#### 1.2 Update Instruction Display

Update the `Display` implementation to print these new instructions:
```
%5 = global_load @counter : i64
global_store @counter, %5
```

### Phase 2: IR Builder Changes (`src/ir/builder.rs`)

#### 2.1 Add Global Variable Tracking

Add a new field to track global variable names:
```rust
/// Names of global variables (to distinguish from locals)
global_variables: HashSet<String>,
```

#### 2.2 Modify First Pass - Collect ALL Globals

Modify the first pass to collect both mutable and immutable global variables:

```rust
// First pass: collect global variables (VarDecls at module level)
for stmt in &ast.statements {
    if let StmtKind::VarDecl { name, mutable, ty, init, .. } = &stmt.kind {
        // Determine type
        let ir_type = if let Some(t) = ty {
            self.convert_type(t)
        } else if let Some(init_expr) = init {
            if let Some(const_val) = self.try_evaluate_const(init_expr) {
                self.const_to_type(&const_val)
            } else {
                IrType::Ptr(Box::new(IrType::Void)) // Default
            }
        } else {
            IrType::Ptr(Box::new(IrType::Void))
        };

        // Determine initial value
        let init_val = init.as_ref().and_then(|e| self.try_evaluate_const(e));

        // Add to module globals
        self.module.globals.push((name.clone(), ir_type.clone(), init_val));

        // Track as global variable
        self.global_variables.insert(name.clone());

        // Also keep in global_constants for immutable optimization
        if !mutable {
            if let Some(const_val) = init.as_ref().and_then(|e| self.try_evaluate_const(e)) {
                self.global_constants.insert(name.clone(), (const_val, ir_type));
            }
        }
    }
}
```

#### 2.3 Skip Global VarDecl in Main Pass

Don't emit `Alloca` for global variables in `build_var_decl`:

```rust
fn build_var_decl(&mut self, name: &str, ty: Option<&TypeAnnotation>, init: Option<&Expr>) -> Result<()> {
    // Skip if this is a global variable (already handled)
    if self.global_variables.contains(name) {
        // For mutable globals, we need to emit initialization code
        if let Some(init_expr) = init {
            if !self.is_const_expr(init_expr) {
                // Non-constant initializer - emit at runtime
                let value = self.build_expr(init_expr)?;
                self.emit(Instruction::GlobalStore {
                    name: name.to_string(),
                    value,
                });
            }
        }
        return Ok(());
    }

    // ... existing local variable logic
}
```

#### 2.4 Modify Identifier Access

Update `build_identifier` to use `GlobalLoad` for global variables:

```rust
fn build_identifier(&mut self, name: &str) -> Result<VarId> {
    // Check local variables first
    if let Some(&var_id) = self.lookup_variable(name) {
        // ... existing local variable load logic
    }
    // Check if it's an immutable global constant - inline the value
    else if let Some((const_val, const_ty)) = self.global_constants.get(name).cloned() {
        let dest = self.new_var();
        self.emit(Instruction::Const {
            dest,
            value: const_val,
            ty: const_ty.clone(),
        });
        self.var_types.insert(dest.0, const_ty);
        Ok(dest)
    }
    // Check if it's a mutable global variable
    else if self.global_variables.contains(name) {
        let ir_type = self.get_global_type(name)?;
        let dest = self.new_var();
        self.emit(Instruction::GlobalLoad {
            dest,
            name: name.to_string(),
            ty: ir_type.clone(),
        });
        self.var_types.insert(dest.0, ir_type);
        Ok(dest)
    }
    // ... rest of existing logic
}
```

#### 2.5 Modify Assignment

Update assignment to use `GlobalStore` for global variables:

```rust
fn build_assignment(&mut self, target: &Expr, value: &Expr) -> Result<VarId> {
    match &target.kind {
        ExprKind::Identifier(name) => {
            let value_var = self.build_expr(value)?;

            if self.global_variables.contains(name) {
                // Global variable assignment
                self.emit(Instruction::GlobalStore {
                    name: name.clone(),
                    value: value_var,
                });
                Ok(value_var)
            } else if let Some(&ptr) = self.lookup_variable(name) {
                // Local variable assignment
                self.emit(Instruction::Store {
                    ptr,
                    value: value_var,
                });
                Ok(value_var)
            } else {
                Err(...)
            }
        }
        // ... other cases
    }
}
```

### Phase 3: LLVM Codegen Changes (`src/codegen/llvm/codegen.rs`)

#### 3.1 Add Global Variables Map

Add tracking for global variable LLVM names:
```rust
/// Global variable names
global_vars: HashMap<String, String>,
```

#### 3.2 Emit Global Variable Definitions

In the `generate` method, add emission of globals before functions:

```rust
pub fn generate(&mut self, module: &Module) -> Result<String, CodegenError> {
    // ... existing header, types, strings, classes code

    // Global variables
    self.emit_global_variables(module)?;

    // ... rest of existing code
}

fn emit_global_variables(&mut self, module: &Module) -> Result<(), CodegenError> {
    if module.globals.is_empty() {
        return Ok(());
    }

    writeln!(self.output, "; Global variables").unwrap();

    for (name, ty, init) in &module.globals {
        let llvm_type = self.type_mapper.to_llvm(ty);
        let llvm_name = mangle_name(name);

        let init_val = match init {
            Some(Constant::Int(n)) => n.to_string(),
            Some(Constant::Float(f)) => format!("{:e}", f),
            Some(Constant::Bool(b)) => if *b { "1" } else { "0" }.to_string(),
            Some(Constant::Null) => "null".to_string(),
            Some(Constant::String(idx)) => {
                // String global - initialize to pointer
                format!("@.str.{}", idx)
            }
            None => self.zero_initializer(ty),
        };

        writeln!(
            self.output,
            "@{} = global {} {}",
            llvm_name, llvm_type, init_val
        ).unwrap();

        self.global_vars.insert(name.clone(), llvm_name);
    }

    writeln!(self.output).unwrap();
    Ok(())
}
```

#### 3.3 Handle GlobalLoad Instruction

```rust
Instruction::GlobalLoad { dest, name, ty } => {
    let llvm_name = self.global_vars.get(name)
        .ok_or_else(|| CodegenError::UndefinedGlobal(name.clone()))?;
    let llvm_type = self.type_mapper.to_llvm(ty);
    let dest_name = self.get_or_create_var(*dest, ty.clone());

    writeln!(
        self.output,
        "    {} = load {}, ptr @{}",
        dest_name, llvm_type, llvm_name
    ).unwrap();
}
```

#### 3.4 Handle GlobalStore Instruction

```rust
Instruction::GlobalStore { name, value } => {
    let llvm_name = self.global_vars.get(name)
        .ok_or_else(|| CodegenError::UndefinedGlobal(name.clone()))?;
    let value_name = self.get_var(*value)?;
    let value_ty = self.var_types.get(&value.0)
        .cloned()
        .unwrap_or(IrType::Int);
    let llvm_type = self.type_mapper.to_llvm(&value_ty);

    writeln!(
        self.output,
        "    store {} {}, ptr @{}",
        llvm_type, value_name, llvm_name
    ).unwrap();
}
```

### Phase 4: Interpreter Updates (`src/interpreter/executor.rs`)

The interpreter already handles globals correctly, but needs to support the new IR instructions.

#### 4.1 Handle GlobalLoad

```rust
Instruction::GlobalLoad { dest, name, ty: _ } => {
    let value = self.globals.get(name)
        .cloned()
        .unwrap_or(Value::Null);
    self.frame_mut().set(*dest, value);
}
```

#### 4.2 Handle GlobalStore

```rust
Instruction::GlobalStore { name, value } => {
    let val = self.frame().get(*value);
    self.globals.insert(name.clone(), val);
}
```

### Phase 5: Unit Tests

Create a new test file `tests/global_variables_tests.rs`:

```rust
#[test]
fn test_global_constant() {
    let source = r#"
        ثابت PI = 3.14159
        اطبع(PI)
    "#;
    assert_output!(source, "3.14159");
}

#[test]
fn test_global_mutable_variable() {
    let source = r#"
        متغير counter = 0

        دالة increment() {
            counter = counter + 1
        }

        increment()
        increment()
        اطبع(counter)
    "#;
    assert_output!(source, "2");
}

#[test]
fn test_global_variable_shadowing() {
    let source = r#"
        متغير x = 10

        دالة test() {
            متغير x = 20  // Local shadows global
            اطبع(x)
        }

        test()
        اطبع(x)
    "#;
    assert_output!(source, "20\n10");
}

#[test]
fn test_global_string_variable() {
    let source = r#"
        متغير message = "Hello"
        message = message + " World"
        اطبع(message)
    "#;
    assert_output!(source, "Hello World");
}

#[test]
fn test_arabic_global_variable() {
    let source = r#"
        متغير العداد = 0
        العداد = العداد + 5
        اطبع(العداد)
    "#;
    assert_output!(source, "5");
}

#[test]
fn test_const_cannot_be_reassigned() {
    let source = r#"
        ثابت x = 10
        x = 20
    "#;
    assert_error!(source, "immutable");
}
```

### Phase 6: Documentation Updates

#### 6.1 Update README.md

Add examples of global variables:

```markdown
### المتغيرات العامة (Global Variables)

يمكنك تعريف متغيرات على مستوى الملف:

```tarqeem
// متغير عام قابل للتعديل
متغير counter = 0

// ثابت عام
ثابت MAX_SIZE = 100

دالة increment() {
    counter = counter + 1  // الوصول للمتغير العام
}
```

#### 6.2 Update ARCHITECTURE.md

Add section on global variable handling in the compiler pipeline.

#### 6.3 Update AI_NOTES.md

Document the implementation decisions and any trade-offs.

## File Changes Summary

| File | Changes |
|------|---------|
| `src/ir/instruction.rs` | Add `GlobalLoad`, `GlobalStore` instructions |
| `src/ir/builder.rs` | Add global variable tracking, modify first pass, update identifier/assignment handling |
| `src/codegen/llvm/codegen.rs` | Add global variable emission, handle new instructions |
| `src/interpreter/executor.rs` | Handle new instructions |
| `src/debug/interpreter.rs` | Handle new instructions |
| `tests/global_variables_tests.rs` | New test file |
| `README.md` | Add global variable examples |
| `ARCHITECTURE.md` | Document global variable handling |
| `docs/AI_NOTES.md` | Implementation notes |

## Implementation Order

1. **IR Instructions** - Add new instructions first (no dependencies)
2. **IR Builder** - Update to use new instructions
3. **Interpreter** - Update to execute new instructions (for testing)
4. **LLVM Codegen** - Emit LLVM IR for globals
5. **Tests** - Add comprehensive tests
6. **Documentation** - Update all relevant docs

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking existing code | Run full test suite after each change |
| Complex initialization order | Globals with constant initializers first, then runtime init |
| Thread safety (future) | Document that globals are not thread-safe |
| Performance impact | Immutable globals still inlined as constants |

## Success Criteria

1. All existing tests pass
2. New global variable tests pass
3. Example programs with globals compile and run correctly
4. LLVM IR output shows proper global definitions
5. Both Arabic and English variable names work
6. Semantic analyzer catches immutable reassignment

## Estimated Complexity

- IR Instructions: Low (straightforward additions)
- IR Builder: Medium (requires careful scope handling)
- LLVM Codegen: Medium (new emission patterns)
- Interpreter: Low (simple execution logic)
- Tests: Low (clear test cases)
- Documentation: Low (examples and explanations)

**Total estimated effort**: Medium complexity feature
