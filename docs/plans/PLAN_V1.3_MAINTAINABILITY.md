# V1.3 Maintainability Implementation Plan

## Overview

Version 1.3 focuses on improving code maintainability through refactoring large modules, improving test coverage, adding documentation, and configuring code quality tools.

## Current State

| Module | Lines | Target | Reduction Needed |
|--------|-------|--------|------------------|
| `ir/builder.rs` | 3,154 | <1,800 | ~1,354 lines |
| `parser/parser.rs` | 2,678 | <1,500 | ~1,178 lines |
| `semantic/analyzer.rs` | 2,400 | <1,500 | ~900 lines |
| `interpreter/executor.rs` | 2,352 | <1,500 | ~852 lines |
| `cli/commands.rs` | 1,908 | <1,200 | ~708 lines |
| `debug/interpreter.rs` | 1,691 | <1,200 | ~491 lines |

## Implementation Strategy

### Phase 1: IR Builder Refactoring

**File: `src/ir/builder.rs` (3,154 lines → ~1,500 lines)**

Convert `builder.rs` to a module directory:

```
src/ir/builder/
├── mod.rs           # Core IrBuilder struct, infrastructure methods
├── expr_builder.rs  # Expression building methods (~800 lines)
├── stmt_builder.rs  # Statement building methods (~700 lines)
└── type_convert.rs  # Type conversion utilities (~150 lines)
```

**Method Distribution:**

`mod.rs` (Core - ~500 lines):
- `IrBuilder` struct definition
- `new()`, `build()` - Entry points
- `emit()`, `new_var()`, `new_block()` - Core utilities
- `push_scope()`, `pop_scope()`, `lookup_var()` - Scope management
- `collect_class()`, `collect_function_signature()` - Pre-processing
- `begin_function()`, `end_function()`, `switch_to_block()` - Function management

`expr_builder.rs` (~800 lines):
- `build_expr()` - Main expression dispatcher
- `build_literal()`, `build_identifier()` - Atoms
- `build_binary()`, `build_unary()` - Operators
- `build_call()`, `build_member()`, `build_index()` - Access
- `build_assignment()`, `build_compound_assignment()` - Assignment
- `build_array()`, `build_object()` - Collections
- `build_lambda()`, `build_new()`, `build_await()` - Complex
- `build_ternary()`, `build_this()`, `build_super()` - Special
- `build_increment()` - Increment/decrement
- `build_enum_variant()` - Enum handling

`stmt_builder.rs` (~700 lines):
- `build_stmt()` - Main statement dispatcher
- `build_var_decl()` - Variable declarations
- `build_func_decl()` - Function declarations
- `build_class_decl()` - Class declarations
- `build_if()` - If statements
- `build_while()`, `build_do_while()` - While loops
- `build_for()`, `build_for_in()` - For loops
- `build_match()`, `build_pattern_check()`, `add_pattern_bindings()` - Pattern matching
- `build_return()`, `build_break()`, `build_continue()` - Control flow
- `build_try()`, `build_throw()` - Exception handling
- `build_block()` - Block statements

`type_convert.rs` (~150 lines):
- `convert_type()` - AST type to IR type
- `convert_simple_type()` - Simple type names
- `semantic_to_ir_type()` - Semantic types to IR
- `infer_expr_type()` - Type inference
- `try_evaluate_const()` - Constant evaluation
- `const_to_type()`, `convert_to_string()` - Helpers

### Phase 2: Parser Refactoring

**File: `src/parser/parser.rs` (2,678 lines → ~1,300 lines)**

```
src/parser/
├── mod.rs           # Existing module exports
├── parser.rs        # Core Parser struct, infrastructure (~500 lines)
├── expr_parser.rs   # Expression parsing (~600 lines)
├── decl_parser.rs   # Declaration parsing (~500 lines)
├── stmt_parser.rs   # Statement parsing (~400 lines)
└── ... (existing files)
```

**Method Distribution:**

`parser.rs` (Core - ~500 lines):
- `Parser` struct definition
- `new()`, `from_tokens()`, `parse()` - Entry points
- Token manipulation: `advance()`, `peek()`, `previous()`, `check()`, etc.
- Error handling: `synchronize()`, `report_error()`
- Helpers: `expect()`, `consume_semicolon()`, `parse_block()`
- Comment handling: `consume_doc_comment()`, `collect_line_comments()`, etc.

`expr_parser.rs` (~600 lines):
- `parse_expression()` - Main entry
- `parse_precedence()` - Pratt parsing
- `parse_prefix()` - Prefix expressions (literals, identifiers, unary, etc.)
- `parse_infix()` - Infix expressions (binary ops, calls, member access)
- `parse_arguments()` - Function arguments
- `try_parse_arrow_function()`, `try_parse_arrow_params()` - Lambdas
- Helpers: `token_to_binary_op()`, `compound_to_binary_op()`

`decl_parser.rs` (~500 lines):
- `parse_declaration()` - Main dispatcher
- `parse_var_declaration()` - Variable declarations
- `parse_function_declaration()` - Function declarations
- `parse_class_declaration()` - Class declarations
- `parse_class_members()`, `parse_class_member()` - Class body
- `parse_property_accessors()` - Property getters/setters
- `parse_interface_declaration()` - Interfaces
- `parse_enum_declaration()`, `parse_enum_variant()` - Enums
- `parse_import_statement()`, `parse_export_statement()` - Modules
- `parse_type_parameters()`, `parse_type_annotation()`, `parse_parameters()` - Types

`stmt_parser.rs` (~400 lines):
- `parse_statement()` - Main dispatcher
- `parse_if_statement()` - If statements
- `parse_while_statement()`, `parse_do_while_statement()` - While loops
- `parse_for_statement()` - For loops
- `parse_match_statement()`, `parse_match_arm()`, `parse_pattern()` - Pattern matching
- `parse_return_statement()`, `parse_break_statement()`, `parse_continue_statement()` - Control flow
- `parse_try_statement()`, `parse_throw_statement()` - Exception handling
- `parse_expression_statement()` - Expression statements

### Phase 3: Semantic Analyzer Refactoring

**File: `src/semantic/analyzer.rs` (2,400 lines → ~1,200 lines)**

```
src/semantic/
├── mod.rs              # Existing exports
├── analyzer.rs         # Core Analyzer struct (~600 lines)
├── type_inference.rs   # Type inference logic (~500 lines)
├── validators.rs       # Validation logic (~300 lines)
└── ... (existing files)
```

**Method Distribution:**

`analyzer.rs` (Core - ~600 lines):
- `Analyzer` struct definition
- `new()`, `analyze()` - Entry points
- `register_types()`, `add_type_members()` - Type registration
- `analyze_stmt()` - Statement analysis dispatcher
- Basic statement analysis: `analyze_var_decl()`, `analyze_func_decl()`
- Scope management: `push_scope()`, `pop_scope()`
- Error/warning helpers: `error()`, `warning()`, `warn()`

`type_inference.rs` (~500 lines):
- `analyze_expr()` - Expression analysis entry
- `infer_type()` - Main type inference
- `infer_pattern_type()` - Pattern type inference
- `resolve_member_type()` - Member access resolution
- `resolve_type()` - Type annotation resolution
- Type comparison and compatibility helpers

`validators.rs` (~300 lines):
- Class validation: `analyze_class_decl()`
- Interface validation: `analyze_interface_decl()`
- Control flow validation: `analyze_if()`, `analyze_while()`, `analyze_for()`, etc.
- `analyze_match()`, `add_pattern_bindings()`
- `analyze_return()`, `analyze_try()`, `analyze_throw()`
- `is_error_type()` - Type validation

### Phase 4: Interpreter Refactoring

**File: `src/interpreter/executor.rs` (2,352 lines → ~1,200 lines)**

```
src/interpreter/
├── mod.rs           # Existing exports
├── executor.rs      # Core Interpreter struct (~500 lines)
├── builtins.rs      # Builtin function handlers (~800 lines)
├── operators.rs     # Binary/unary operator execution (~200 lines)
└── ... (existing files)
```

**Method Distribution:**

`executor.rs` (Core - ~500 lines):
- `Interpreter` struct definition
- `new()`, `run()` - Entry points
- `call_function()`, `execute_function()` - Function execution
- `execute_block()`, `execute_instruction()` - Block/instruction execution
- `init_globals()`, `find_main_function()` - Initialization
- Local variable management: `get_local()`, `set_local()`
- Exception handling: `pop_try_block()`

`builtins.rs` (~800 lines):
- `is_builtin()` - Check if function is builtin
- `call_builtin()` - Main builtin dispatcher
- All builtin implementations:
  - Print/input: `trq_print`, `trq_read`, etc.
  - String operations: `trq_string_*`
  - Array operations: `trq_array_*`
  - Math functions: `trq_math_*`
  - Type conversions: `trq_*_to_string`
  - File I/O, networking, etc.

`operators.rs` (~200 lines):
- `execute_binary_op()` - Binary operator execution
- `execute_unary_op()` - Unary operator execution
- `constant_to_value()` - Constant conversion

### Phase 5: CLI Commands Refactoring

**File: `src/cli/commands.rs` (1,908 lines → ~800 lines)**

```
src/cli/
├── mod.rs           # Existing
├── commands.rs      # Core command handling (~400 lines)
├── compile_cmd.rs   # Compile command (~200 lines)
├── run_cmd.rs       # Run command (~200 lines)
├── tools_cmd.rs     # Tools (fmt, check, etc.) (~300 lines)
└── ... (existing)
```

### Phase 6: Debug Interpreter Refactoring

**File: `src/debug/interpreter.rs` (1,691 lines → ~900 lines)**

```
src/debug/
├── mod.rs           # Existing
├── interpreter.rs   # Core debug interpreter (~500 lines)
├── evaluators.rs    # Expression evaluators (~400 lines)
└── ... (existing)
```

## Phase 7: Test Coverage Improvements

### New Test Files to Create:

1. `tests/ir_optimization_tests.rs` - Tests for each optimization pass
2. `tests/codegen_execution_tests.rs` - Compile and run tests
3. `tests/package_manager_tests.rs` - Package manager integration tests
4. `tests/formatter_edge_cases.rs` - RTL, deep nesting, edge cases

### Coverage Configuration:

Add to `Cargo.toml`:
```toml
[dev-dependencies]
cargo-tarpaulin = "0.27"
```

Or use `llvm-cov` with rustup component.

## Phase 8: Documentation

### Files to Create:

1. `docs/INTERNALS.md` - Compiler internals guide
2. Update module-level doc comments in all `mod.rs` files

### Documentation Structure for INTERNALS.md:

```markdown
# Tarqeem Compiler Internals

## Compiler Pipeline
- Lexer → Parser → Semantic → IR → Codegen

## Type Inference Algorithm
- Bidirectional type inference
- Generic resolution

## Method Dispatch & VTable Generation
- Single inheritance vtables
- Interface implementation

## Optimization Passes
- Constant folding
- Dead code elimination
- Common subexpression elimination
- Function inlining
- Loop optimizations
```

## Phase 9: Code Quality Configuration

### Create `.cargo/config.toml`:

```toml
[build]
rustflags = ["-D", "warnings"]

[target.'cfg(all())']
rustflags = []
```

### Update `Cargo.toml` with clippy lints:

```toml
[lints.clippy]
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
clone_on_ref_ptr = "warn"
```

### Create pre-commit hook:

`.git/hooks/pre-commit`:
```bash
#!/bin/sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## Execution Order

1. **IR Builder Refactoring** (largest impact, most critical)
2. **Parser Refactoring** (second largest)
3. **Semantic Analyzer Refactoring**
4. **Interpreter Refactoring**
5. **CLI Commands Refactoring** (lower priority)
6. **Debug Interpreter Refactoring** (lower priority)
7. **Test Coverage**
8. **Documentation**
9. **Code Quality Configuration**

## Success Criteria

- [ ] No module exceeds 1,800 lines
- [ ] All 1,125+ tests pass
- [ ] All 13 example programs work (both compiled and JIT)
- [ ] `cargo clippy` produces zero warnings
- [ ] `cargo fmt --check` passes
- [ ] `docs/INTERNALS.md` complete
- [ ] Pre-commit hooks configured

## Risk Mitigation

1. **Run tests after each refactoring step** - Don't batch changes
2. **Keep public API stable** - Only move private methods
3. **Use `pub(crate)` for internal APIs** - Maintain encapsulation
4. **Commit frequently** - Enable easy rollback
