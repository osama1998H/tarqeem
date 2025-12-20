# Phase 3 Preparation Plan

## Overview

This document outlines the prerequisites that must be completed before Phase 3 (Standard Library) can begin. These items were deferred from Phase 2 and are now blocking Phase 3 progress.

**Status as of 2025-12-20**:
- Phase 2: ✅ Complete (all 6 milestones, 101 tests passing)
- Prerequisites: 🚧 In Progress

---

## Executive Summary

### Prerequisites to Complete

| Priority | Item | Status | Effort |
|----------|------|--------|--------|
| P0 | Module System (استورد/صدّر) | 🔴 Partial | Large |
| P1 | Super Constructor Call (أساس()) | 🔴 Bug | Medium |
| P2 | Update AI_NOTES.md | 🔴 Outdated | Small |
| P3 | Create stdlib_trq/ Structure | 🔴 Not Started | Medium |

### What's Already Done

| Component | Status | Notes |
|-----------|--------|-------|
| Module tokens | ✅ Done | استورد, صدّر, من, كـ in lexer |
| Module parsing | ✅ Done | Full AST for import/export statements |
| Import stub | ✅ Done | Registers imported names as `Type::Any` |
| Array indexing | ✅ Done | IR + codegen + runtime |
| For-in iteration | ✅ Done | IR desugars to indexed loop |
| Empty array inference | ✅ Done | Uses expected_type context |
| Runtime library | ✅ Done | C runtime with all core functions |

### Previously Reported as Bugs (Now Fixed)

| Item | Was Reported As | Actual Status |
|------|-----------------|---------------|
| Array indexing (`arr[i]`) | Not implemented | ✅ Fixed - IR + codegen works |
| For-in iteration (`لكل x في arr`) | Not implemented | ✅ Fixed - Desugars to indexed loop |
| Empty array inference | Not working | ✅ Fixed - Uses expected_type context |

---

## P0: Module System Implementation

### Current State

**What exists**:
```
Lexer: استورد, صدّر, من, كـ → Tokens ✅
Parser: Full import/export parsing → AST ✅
Semantic: Stub that registers names as Type::Any ✅
IR/Codegen: Nothing ❌
```

**What's missing**:
1. Module path resolution
2. File loading and parsing
3. Symbol export/visibility tracking
4. Cross-module type checking
5. IR generation for modules
6. Linker support for multi-file programs

### Implementation Plan

#### Task M1: Module Infrastructure (Foundation)

**File**: `src/semantic/modules.rs` (new)

**Components**:

```rust
/// Module identifier (path-based)
pub struct ModuleId(PathBuf);

/// Loaded module with its symbols
pub struct Module {
    pub id: ModuleId,
    pub path: PathBuf,
    pub exports: HashMap<String, Symbol>,
    pub ast: Program,
}

/// Module resolution and loading
pub struct ModuleLoader {
    /// Search paths for modules
    search_paths: Vec<PathBuf>,
    /// Loaded modules (cached)
    modules: HashMap<ModuleId, Module>,
    /// Currently loading (for cycle detection)
    loading_stack: Vec<ModuleId>,
}
```

**Key functions**:
- `resolve_path(from: &Path, import: &str) -> Result<PathBuf>`
- `load_module(path: &Path) -> Result<Module>`
- `check_circular_dependency(module_id: &ModuleId) -> Result<()>`

#### Task M2: Export Tracking

**Changes to**: `src/semantic/analyzer.rs`

**New fields**:
```rust
struct Analyzer {
    // ... existing fields ...

    /// Exported symbols from current module
    exports: HashMap<String, (Symbol, Span)>,
    /// Whether current item has export visibility
    in_export: bool,
}
```

**New behavior**:
1. Track `صدّر` (export) declarations
2. Store exported symbols with their types
3. Validate all exports are defined

#### Task M3: Import Resolution

**Changes to**: `src/semantic/analyzer.rs`

**Replace the stub**:
```rust
fn analyze_import(&mut self, items: &ImportItems, from: &str, span: Span) {
    // Current: Just registers names as Type::Any
    // New: Actually loads and resolves the module

    // 1. Resolve module path
    let module_path = self.module_loader.resolve_path(&self.current_file, from)?;

    // 2. Load module (if not cached)
    let module = self.module_loader.load_module(&module_path)?;

    // 3. Import symbols based on items
    match items {
        ImportItems::Named(imports) => {
            for import in imports {
                if let Some(symbol) = module.exports.get(&import.name) {
                    let name = import.alias.as_ref().unwrap_or(&import.name);
                    self.scope.define(symbol.with_name(name));
                } else {
                    self.error(format!("Module '{}' has no export '{}'", from, import.name));
                }
            }
        }
        ImportItems::Wildcard(alias) => {
            // Create namespace object with all exports
        }
        ImportItems::Default(name) => {
            // Import default export
        }
    }
}
```

#### Task M4: IR Generation for Modules

**Changes to**: `src/ir/builder.rs`

**New behavior**:
- Generate IR for each module separately
- Handle cross-module function references
- Support forward declarations

**IR Module structure**:
```rust
pub struct IrProgram {
    pub modules: Vec<IrModule>,
    pub main_module: ModuleId,
}

pub struct IrModule {
    pub id: ModuleId,
    pub functions: Vec<Function>,
    pub classes: Vec<ClassDef>,
    pub exports: HashSet<String>,
}
```

#### Task M5: Multi-File Compilation

**Changes to**: `src/codegen/` and `src/cli/`

**New workflow**:
1. Parse entry file
2. Recursively load all imports
3. Topologically sort modules (detect cycles)
4. Generate IR for each module
5. Generate LLVM IR with proper linkage
6. Link all object files together

**CLI changes**:
```bash
# Single file (existing)
tarqeem compile main.trq -o main

# With modules (new behavior)
tarqeem compile main.trq -o main
# Automatically finds and compiles: stdlib_trq/مجموعات.trq, etc.
```

### Module System Test Cases

```tarqeem
// ===== test_modules/math.trq =====
صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}

صدّر ثابت باي = 3.14159

// ===== test_modules/main.trq =====
استورد { جمع، باي } من "./math"

اطبع(جمع(1، 2))  // 3
اطبع(باي)         // 3.14159
```

### Estimated Effort

| Task | Complexity | Estimated Time |
|------|------------|----------------|
| M1: Module Infrastructure | Medium | 4-6 hours |
| M2: Export Tracking | Low | 2-3 hours |
| M3: Import Resolution | Medium | 4-6 hours |
| M4: IR Generation | Medium | 4-6 hours |
| M5: Multi-File Compilation | High | 6-8 hours |
| Testing & Debug | Medium | 4-6 hours |
| **Total** | | **24-35 hours** |

---

## P1: Super Constructor Call Bug

### Issue

Super constructor calls (`أساس(args)`) fail with error:
```
خطأ: لا يمكن استدعاء نوع غير دالة شخص
Error: Cannot call non-function type شخص
```

**Reproduction** (`examples/صنف.trq` line 39):
```tarqeem
صنف موظف يرث شخص {
    منشئ(اسم: نص، عمر: عدد، راتب: عدد_عشري، منصب: نص) {
        أساس(اسم، عمر);  // ❌ Fails here
        هذا.راتب = راتب;
    }
}
```

### Root Cause

In `src/semantic/analyzer.rs:1189-1211`, the `ExprKind::Super` case returns `Type::Class(parent_name)`. When the parser then sees `أساس(args)`, it becomes a call expression where the callee has type `Class`, not `Function`.

The semantic analyzer's `analyze_call()` function checks if the callee is callable, but it doesn't recognize `Type::Class` as a valid callee for constructor calls.

### Fix Required

**Option A: Special-case super calls in analyze_call()**
```rust
// In analyze_call():
if matches!(callee_expr.kind, ExprKind::Super) {
    // This is a super constructor call
    // Look up parent class constructor and validate args
    return self.analyze_super_constructor_call(callee_expr, args);
}
```

**Option B: Return Type::Function from ExprKind::Super when followed by call**
This requires lookahead which is more complex.

**Recommended**: Option A - cleaner and matches how other languages handle super().

### Implementation Steps

1. Add `analyze_super_constructor_call()` method to `Analyzer`
2. Look up parent class constructor parameters
3. Validate argument types match
4. Return `Type::Void` (constructors don't return values)
5. Generate appropriate IR for super constructor call
6. Add tests for:
   - Valid super() call with correct args
   - Super() call with wrong arg count
   - Super() call with wrong arg types
   - Super() call in class without parent

### Estimated Effort

| Task | Complexity | Estimated Time |
|------|------------|----------------|
| Semantic analysis fix | Medium | 2-3 hours |
| IR generation fix | Medium | 2-3 hours |
| Codegen support | Low | 1-2 hours |
| Testing | Low | 1-2 hours |
| **Total** | | **6-10 hours** |

---

## P2: Update AI_NOTES.md (Documentation)

### Issue

The `docs/AI_NOTES.md` file has outdated information in the "Known Issues" and "Pending" sections:

**Currently says** (WRONG):
```markdown
### Known Issues
- Array indexing not implemented in IR builder
- For-in iteration over arrays not implemented
- Empty array type inference needs work

### Pending
- [ ] Implement array indexing in IR builder
- [ ] Implement for-in iteration over collections
- [ ] Support generic array types properly (empty array type inference)
```

**Reality**:
- Array indexing: ✅ Implemented in `src/ir/builder.rs:2144-2166`
- For-in iteration: ✅ Implemented in `src/ir/builder.rs:1179`
- These were fixed in recent commits (3e4f72b, a669929)

### Fix Required

Update `docs/AI_NOTES.md` to:
1. Move "Array indexing" and "For-in iteration" to Completed
2. Update "Current State" section to reflect Phase 2 completion
3. Update "Project Phase" to mention Phase 3 preparation

---

## P3: Create stdlib_trq/ Directory Structure

### Purpose

The `stdlib_trq/` directory contains the Tarqeem standard library written in Tarqeem itself. This is distinct from `runtime/` which is the C runtime library.

### Directory Structure

```
stdlib_trq/
├── README.md              # Standard library documentation
├── مجموعات.trq            # Collections (List, Map, Set)
├── رياضيات.trq            # Math functions
├── نص.trq                 # String utilities
├── ملفات.trq              # File operations
└── شبكة.trq              # Networking
```

### File Contents (Initial Skeletons)

#### stdlib_trq/مجموعات.trq (Collections)
```tarqeem
// مجموعات - Collections Module
// مجموعات - وحدة المجموعات

// قائمة - List class wrapper around arrays
صدّر صنف قائمة<ن> {
    خاص عناصر: مصفوفة<ن>

    منشئ() {
        هذا.عناصر = []
    }

    عام دالة أضف(عنصر: ن) {
        هذا.عناصر.ألحق(عنصر)
    }

    عام دالة طول() -> عدد {
        أرجع هذا.عناصر.طول
    }

    عام دالة احصل(فهرس: عدد) -> ن {
        أرجع هذا.عناصر[فهرس]
    }
}

// قاموس - Dictionary wrapper
// TODO: Requires Map type support in runtime

// مجموعة - Set wrapper
// TODO: Requires Set type support in runtime
```

#### stdlib_trq/رياضيات.trq (Math)
```tarqeem
// رياضيات - Math Module
// رياضيات - وحدة الرياضيات

صدّر ثابت باي = 3.141592653589793
صدّر ثابت هـ = 2.718281828459045  // Euler's number

صدّر دالة مطلق(س: عدد) -> عدد {
    إذا (س < 0) {
        أرجع -س
    }
    أرجع س
}

صدّر دالة أقصى(أ: عدد، ب: عدد) -> عدد {
    إذا (أ > ب) {
        أرجع أ
    }
    أرجع ب
}

صدّر دالة أدنى(أ: عدد، ب: عدد) -> عدد {
    إذا (أ < ب) {
        أرجع أ
    }
    أرجع ب
}

// Built-in math functions from runtime
// جذر_تربيعي، جا، جتا، ظا، etc. are provided by runtime
```

#### stdlib_trq/نص.trq (String Utilities)
```tarqeem
// نص - String Utilities Module
// نص - وحدة معالجة النصوص

صدّر دالة فارغ(نص: نص) -> منطقي {
    أرجع نص.طول == 0
}

صدّر دالة يحتوي(نص: نص، جزء: نص) -> منطقي {
    // TODO: Implement string contains
    أرجع خطأ
}

صدّر دالة يبدأ_بـ(نص: نص، بادئة: نص) -> منطقي {
    // TODO: Implement starts_with
    أرجع خطأ
}

صدّر دالة ينتهي_بـ(نص: نص، لاحقة: نص) -> منطقي {
    // TODO: Implement ends_with
    أرجع خطأ
}
```

#### stdlib_trq/ملفات.trq (File Operations)
```tarqeem
// ملفات - File Operations Module
// ملفات - وحدة عمليات الملفات

// Note: File I/O requires runtime support via trq_file_* functions

صدّر دالة اقرأ_ملف(مسار: نص) -> نص {
    // TODO: Implement via runtime
    أرجع ""
}

صدّر دالة اكتب_ملف(مسار: نص، محتوى: نص) -> منطقي {
    // TODO: Implement via runtime
    أرجع خطأ
}

صدّر دالة موجود(مسار: نص) -> منطقي {
    // TODO: Implement via runtime
    أرجع خطأ
}
```

### Dependencies

Creating stdlib_trq/ **depends on** the module system (P0) being complete:
- Standard library files use `صدّر` to export symbols
- User code uses `استورد` to import from stdlib

### Implementation Order

1. ✅ Complete module system (P0)
2. Create stdlib_trq/ directory structure
3. Implement skeleton files
4. Add runtime bindings for advanced functions
5. Add tests for each module

---

## Implementation Order

### Recommended Sequence

**Note**: P1 (Super Constructor Bug) can be done in parallel with Module System since they're independent.

```
Step 1a: Module Infrastructure (M1)        Step 1b: Super Constructor Fix (P1)
├── Create src/semantic/modules.rs         ├── Fix analyze_call() for super()
├── Add ModuleLoader, Module types         ├── Add analyze_super_constructor_call()
├── Implement path resolution              ├── Update IR generation
└── Add to semantic mod.rs                 └── Add tests

Step 2: Export Tracking (M2)
├── Add exports field to Analyzer
├── Track exported symbols
└── Validate exports

Step 3: Import Resolution (M3)
├── Replace import stub in analyzer
├── Actually load and parse modules
├── Import symbols with proper types

Step 4: IR Generation for Modules (M4)
├── Handle multi-module IR
├── Add cross-module references
├── Support forward declarations

Step 5: Multi-File Compilation (M5)
├── Update CLI compile command
├── Topological sort modules
├── Generate LLVM IR with linkage
├── Link object files

Step 6: Update Documentation (P2)
├── Update AI_NOTES.md (correct bug status)
├── Update PHASE2_PLAN.md

Step 7: Create Standard Library (P3)
├── Create stdlib_trq/ directory
├── Create skeleton files
├── Add basic implementations
└── Add tests
```

---

## Success Criteria

Phase 3 preparation is complete when:

1. **Super constructor calls work**:
   ```tarqeem
   صنف موظف يرث شخص {
       منشئ(اسم: نص، عمر: عدد) {
           أساس(اسم، عمر);  // ✅ Works
       }
   }
   ```

2. **Module imports work end-to-end**:
   ```tarqeem
   // math.trq
   صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد { أرجع أ + ب }

   // main.trq
   استورد { جمع } من "./math"
   اطبع(جمع(1، 2))  // Prints: 3
   ```

3. **Circular dependencies are detected**:
   ```
   Error: Circular dependency detected: a.trq -> b.trq -> a.trq
   خطأ: تم اكتشاف اعتماد دائري: a.trq -> b.trq -> a.trq
   ```

4. **Standard library can be imported**:
   ```tarqeem
   استورد { قائمة } من "مجموعات"
   متغير أرقام = جديد قائمة<عدد>()
   ```

5. **Class example compiles and runs**:
   ```bash
   cargo run -- check examples/صنف.trq  # ✅ No errors
   ```

6. **All 101+ tests still pass**

7. **Documentation is up to date**

---

## Risk Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Module system complexity | High | Start with simple case (named imports), add features incrementally |
| Cross-module type checking | Medium | Reuse existing type system, add module context |
| Linker issues | Medium | Test with simple 2-file programs first |
| Standard library depends on module system | High | Complete module system before stdlib |
| Circular dependency detection | Low | Use loading stack approach |

---

## Notes

- Keep all existing Phase 2 tests passing
- Add tests for each new module feature
- Document new error messages in both Arabic and English
- Update CLAUDE.md if new patterns emerge
