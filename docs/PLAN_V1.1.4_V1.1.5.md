# Implementation Plan: v1.1.4 Complete Incomplete Features & v1.1.5 Improve Error Handling

## Executive Summary

This document provides a detailed implementation plan for two roadmap tasks:
- **1.1.4 Complete Incomplete Features** - Resolve 3 TODO comments in production code
- **1.1.5 Improve Error Handling** - Reduce `unwrap()` calls from 225 to <100

---

## Task 1.1.4: Complete Incomplete Features (Priority: MEDIUM)

### Overview

Three TODO comments exist in production code that need to be resolved:

| # | Location | Issue | Complexity |
|---|----------|-------|------------|
| 1 | `src/ir/opt/inline.rs:281` | Hardcoded type in function inlining | Medium |
| 2 | `src/lsp/analysis/document.rs:85` | Missing Arabic error messages | Low |
| 3 | `src/package/lockfile.rs:165` | Incomplete package array parsing | Medium |

---

### TODO #1: Proper Type Handling in Function Inliner

**File**: `src/ir/opt/inline.rs:275-282`

**Current Code**:
```rust
if let (Some(dest_var), Some(ret_var)) = (dest, return_var) {
    continuation_block.instructions.push(Instruction::Binary {
        dest: dest_var,
        op: crate::ir::BinaryOp::Add, // This is a hack - we need a Copy instruction
        left: ret_var,
        right: ret_var, // x + 0 would be better but we don't have the type info
        ty: IrType::Int, // TODO: proper type handling
    });
}
```

**Problem**:
- The inliner uses a binary add operation as a hack to "copy" a return value
- The type is hardcoded to `IrType::Int`, which is incorrect for Float, String, or Object returns
- This could cause incorrect code generation for non-integer functions

**Solution Approach**:

1. **Option A (Recommended)**: Add a `Copy` instruction to the IR instruction set
   ```rust
   // Add to src/ir/instruction.rs
   Instruction::Copy { dest: VarId, src: VarId, ty: IrType }
   ```
   This is the cleanest solution and aligns with how other IR systems work.

2. **Option B**: Track the callee's return type and use it
   ```rust
   // Get return type from the callee function
   let ret_ty = callee.return_type.clone();

   // Use proper type in the instruction
   continuation_block.instructions.push(Instruction::Binary {
       dest: dest_var,
       op: crate::ir::BinaryOp::Add,
       left: ret_var,
       right: self.create_zero_const(ret_ty.clone()), // Need to add zero const for type
       ty: ret_ty,
   });
   ```

**Recommended Implementation (Option A)**:

1. Add `Copy` instruction to `src/ir/instruction.rs`:
   ```rust
   /// Copy a value from one variable to another
   Copy { dest: VarId, src: VarId, ty: IrType },
   ```

2. Handle `Copy` in all instruction handlers:
   - `src/ir/opt/dce.rs` - Mark as having side effects
   - `src/ir/opt/const_fold.rs` - Pass through unchanged
   - `src/ir/opt/cse.rs` - Handle like a unary operation
   - `src/codegen/llvm/codegen.rs` - Emit LLVM load/store
   - `src/interpreter/executor.rs` - Copy value in interpreter

3. Update inliner to use `Copy`:
   ```rust
   if let (Some(dest_var), Some(ret_var)) = (dest, return_var) {
       let ret_ty = callee.return_type.clone();
       continuation_block.instructions.push(Instruction::Copy {
           dest: dest_var,
           src: ret_var,
           ty: ret_ty,
       });
   }
   ```

**Files to Modify**:
- `src/ir/instruction.rs` - Add Copy variant
- `src/ir/opt/inline.rs` - Use Copy instruction
- `src/ir/opt/dce.rs` - Handle Copy in liveness
- `src/ir/opt/const_fold.rs` - Pass through Copy
- `src/ir/opt/cse.rs` - Handle Copy in CSE
- `src/codegen/llvm/codegen.rs` - Generate LLVM code for Copy
- `src/interpreter/executor.rs` - Execute Copy

**Testing**:
```rust
#[test]
fn test_inline_float_function() {
    // Test that float functions inline correctly
    let source = r#"
    بسم_الله
    دالة مربع(س: عدد_عشري) -> عدد_عشري {
        أرجع س * س
    }
    متغير ن = مربع(2.5)
    الحمد_لله
    "#;
    // Verify inlined type is Float, not Int
}

#[test]
fn test_inline_string_function() {
    // Similar test for string return type
}
```

---

### TODO #2: Arabic Error Messages in LSP

**File**: `src/lsp/analysis/document.rs:82-89`

**Current Code**:
```rust
for token in &tokens {
    if let TokenKind::Error(msg) = &token.kind {
        diagnostics.push(Diagnostic::error(
            msg.clone(),
            msg.clone(), // TODO: Arabic error messages
            token.span,
        ));
        has_errors = true;
    }
}
```

**Problem**: Lexer errors are displayed with the same English message for both languages.

**Solution**: Create a mapping function that translates common lexer errors to Arabic.

**Implementation**:

1. Add a translation function in `src/lsp/analysis/document.rs`:
   ```rust
   fn translate_lexer_error(msg: &str) -> String {
       // Common lexer error patterns and their Arabic translations
       match msg {
           m if m.contains("Unexpected character") => {
               m.replace("Unexpected character", "حرف غير متوقع")
           }
           m if m.contains("Unterminated string") => {
               "نص غير مُنهى".to_string()
           }
           m if m.contains("Invalid number") => {
               "رقم غير صالح".to_string()
           }
           m if m.contains("Invalid escape sequence") => {
               "تسلسل هروب غير صالح".to_string()
           }
           m if m.contains("Unterminated comment") => {
               "تعليق غير مُنهى".to_string()
           }
           m if m.contains("Invalid character in identifier") => {
               "حرف غير صالح في المعرّف".to_string()
           }
           m if m.contains("Number too large") => {
               "الرقم كبير جداً".to_string()
           }
           _ => format!("خطأ معجمي: {}", msg)
       }
   }
   ```

2. Update the error creation:
   ```rust
   for token in &tokens {
       if let TokenKind::Error(msg) = &token.kind {
           let arabic_msg = translate_lexer_error(msg);
           diagnostics.push(Diagnostic::error(
               msg.clone(),
               arabic_msg,
               token.span,
           ));
           has_errors = true;
       }
   }
   ```

**Files to Modify**:
- `src/lsp/analysis/document.rs` - Add translation function and use it

**Testing**:
```rust
#[test]
fn test_arabic_lexer_error_translation() {
    let content = "متغير س = \"نص غير مغلق";
    let mut doc = DocumentState::new(test_uri(), 1, wrap_with_markers(&content));
    let analysis = doc.get_analysis(Language::Arabic);

    // Verify the diagnostic has proper Arabic message
    assert!(analysis.diagnostics.iter().any(|d| d.message_ar.contains("غير")));
}
```

---

### TODO #3: Complete Lock File Package Parsing

**File**: `src/package/lockfile.rs:165`

**Current Code**:
```rust
pub fn parse_arabic_format(content: &str) -> PackageResult<Self> {
    // ... parsing logic for version and root ...

    let packages = Vec::new(); // TODO: Full package parsing

    Ok(Self {
        version,
        root,
        packages,
    })
}
```

**Problem**: The Arabic format parser doesn't parse the `حزم` (packages) array.

**Solution**: Implement full package array parsing following the Arabic format structure.

**Expected Arabic Format**:
```yaml
نسخة_الصيغة: ١

جذر:
    اسم: تطبيقي
    نسخة: ٠.١.٠

حزم:
    - اسم: json
      نسخة: ٢.٠.٠
      تحقق: sha256:abc123
      مصدر: سجل (https://registry.tarqeem.dev)
      اعتماديات:
          utils: ١.٠.٠
```

**Implementation**:

```rust
fn parse_arabic_format(content: &str) -> PackageResult<Self> {
    let value = format::parse(content)
        .map_err(|e| super::error::PackageError::InvalidManifest(format!("{}", e)))?;

    let obj = value.as_object().ok_or_else(|| {
        super::error::PackageError::InvalidManifest("القفل يجب أن يكون كائناً".to_string())
    })?;

    // Parse version
    let version = obj
        .get("نسخة_الصيغة")
        .or_else(|| obj.get("version"))
        .and_then(|v| v.as_i64())
        .unwrap_or(LOCKFILE_VERSION as i64) as u32;

    // Parse root
    let root = Self::parse_root_package(obj)?;

    // Parse packages array
    let packages = Self::parse_packages_array(obj)?;

    Ok(Self {
        version,
        root,
        packages,
    })
}

fn parse_root_package(obj: &serde_json::Map<String, serde_json::Value>)
    -> PackageResult<Option<RootPackage>>
{
    if let Some(root_obj) = obj.get("جذر").or_else(|| obj.get("root")) {
        if let Some(root_map) = root_obj.as_object() {
            let name = root_map
                .get("اسم")
                .or_else(|| root_map.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let ver = root_map
                .get("نسخة")
                .or_else(|| root_map.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return Ok(Some(RootPackage { name, version: ver }));
        }
    }
    Ok(None)
}

fn parse_packages_array(obj: &serde_json::Map<String, serde_json::Value>)
    -> PackageResult<Vec<LockedPackage>>
{
    let mut packages = Vec::new();

    if let Some(pkgs_val) = obj.get("حزم").or_else(|| obj.get("packages")) {
        if let Some(pkgs_arr) = pkgs_val.as_array() {
            for pkg_val in pkgs_arr {
                if let Some(pkg_obj) = pkg_val.as_object() {
                    let pkg = Self::parse_locked_package(pkg_obj)?;
                    packages.push(pkg);
                }
            }
        }
    }

    Ok(packages)
}

fn parse_locked_package(obj: &serde_json::Map<String, serde_json::Value>)
    -> PackageResult<LockedPackage>
{
    let name = obj
        .get("اسم")
        .or_else(|| obj.get("name"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            super::error::PackageError::InvalidManifest(
                "اسم الحزمة مطلوب / Package name required".to_string()
            )
        })?
        .to_string();

    let version = obj
        .get("نسخة")
        .or_else(|| obj.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();

    let checksum = obj
        .get("تحقق")
        .or_else(|| obj.get("checksum"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let source = Self::parse_package_source(obj)?;

    let dependencies = Self::parse_dependencies(obj)?;

    Ok(LockedPackage {
        name,
        version,
        source,
        checksum,
        dependencies,
    })
}

fn parse_package_source(obj: &serde_json::Map<String, serde_json::Value>)
    -> PackageResult<PackageSource>
{
    let source_str = obj
        .get("مصدر")
        .or_else(|| obj.get("source"))
        .and_then(|v| v.as_str())
        .unwrap_or("سجل (https://registry.tarqeem.dev)");

    // Parse source string format: "سجل (url)" or "git (url ref)" or "مسار (path)"
    if source_str.starts_with("سجل") || source_str.starts_with("registry") {
        let url = Self::extract_parenthesized(source_str);
        Ok(PackageSource::Registry { url })
    } else if source_str.starts_with("git") {
        let content = Self::extract_parenthesized(source_str);
        let parts: Vec<&str> = content.splitn(2, ' ').collect();
        let url = parts.first().unwrap_or(&"").to_string();
        let reference = Self::parse_git_reference(parts.get(1).unwrap_or(&""));
        Ok(PackageSource::Git { url, reference })
    } else if source_str.starts_with("مسار") || source_str.starts_with("path") {
        let path = Self::extract_parenthesized(source_str);
        Ok(PackageSource::Path { path })
    } else {
        Ok(PackageSource::Registry {
            url: "https://registry.tarqeem.dev".to_string()
        })
    }
}

fn extract_parenthesized(s: &str) -> String {
    s.find('(')
        .and_then(|start| s.rfind(')').map(|end| &s[start + 1..end]))
        .unwrap_or("")
        .to_string()
}

fn parse_git_reference(ref_str: &str) -> GitReference {
    if ref_str.starts_with("فرع=") || ref_str.starts_with("branch=") {
        let branch = ref_str.split('=').nth(1).unwrap_or("main").to_string();
        GitReference::Branch { branch }
    } else if ref_str.starts_with("وسم=") || ref_str.starts_with("tag=") {
        let tag = ref_str.split('=').nth(1).unwrap_or("").to_string();
        GitReference::Tag { tag }
    } else if ref_str.starts_with("مراجعة=") || ref_str.starts_with("rev=") {
        let rev = ref_str.split('=').nth(1).unwrap_or("").to_string();
        GitReference::Rev { rev }
    } else {
        GitReference::Branch { branch: "main".to_string() }
    }
}

fn parse_dependencies(obj: &serde_json::Map<String, serde_json::Value>)
    -> PackageResult<HashMap<String, String>>
{
    let mut deps = HashMap::new();

    if let Some(deps_val) = obj.get("اعتماديات").or_else(|| obj.get("dependencies")) {
        if let Some(deps_obj) = deps_val.as_object() {
            for (name, version_val) in deps_obj {
                if let Some(version) = version_val.as_str() {
                    deps.insert(name.clone(), version.to_string());
                }
            }
        }
    }

    Ok(deps)
}
```

**Files to Modify**:
- `src/package/lockfile.rs` - Add package parsing functions

**Testing**:
```rust
#[test]
fn test_parse_arabic_lockfile_with_packages() {
    let content = r#"
نسخة_الصيغة: 1

جذر:
    اسم: my-app
    نسخة: 0.1.0

حزم:
    - اسم: json
      نسخة: 2.0.0
      تحقق: sha256:abc123
      مصدر: سجل (https://registry.tarqeem.dev)
      اعتماديات:
          utils: 1.0.0
"#;
    let lockfile = LockFile::parse_arabic_format(content).unwrap();
    assert_eq!(lockfile.packages.len(), 1);
    assert_eq!(lockfile.packages[0].name, "json");
    assert_eq!(lockfile.packages[0].version, "2.0.0");
}
```

---

## Task 1.1.5: Improve Error Handling (Priority: MEDIUM)

### Overview

**Current State**: 527 total `unwrap()` calls across 58 files
**Production Code Estimate**: ~225 calls (excluding `#[cfg(test)]` modules)
**Target**: Reduce to <100 in production code

### Audit Results by File (Production Code Only)

| File | Production unwrap() | Risk Level | Notes |
|------|---------------------|------------|-------|
| `src/ir/builder.rs` | 3 | HIGH | Variable lookups at lines 1903, 1923, 1967 |
| `src/codegen/llvm/codegen.rs` | ~26 | MEDIUM | LLVM operations, needs context |
| `src/interpreter/executor.rs` | ~6 | MEDIUM | Runtime value access |
| `src/package/cache.rs` | ~26 | MEDIUM | File operations, network |
| `src/debug/context.rs` | ~12 | LOW | Debug-only code |
| `src/semantic/modules.rs` | ~3 | LOW | After existence checks |
| Others | ~149 | VARIES | Mixed criticality |

### High-Priority Fixes

#### 1. IR Builder Variable Lookups (CRITICAL)

**Location**: `src/ir/builder.rs:1903, 1923, 1967`

**Current Code**:
```rust
// Line 1903
let ptr = self.lookup_var(&name).unwrap();

// Line 1923
let ptr = self.lookup_var(&name).unwrap();

// Line 1967
let ptr = self.lookup_var(&name).unwrap();
```

**Problem**: These are called after checking `is_local`, but the unwrap could still fail if scope management has bugs.

**Solution**: Convert to proper error propagation.

```rust
// Helper method to add to IrBuilder
fn get_local_var(&self, name: &str) -> Result<VarId> {
    self.lookup_var(name).ok_or_else(|| {
        IrError::new(
            format!("Internal error: variable '{}' not found in scope", name),
            format!("خطأ داخلي: المتغير '{}' غير موجود في النطاق", name),
        )
    })
}

// Then replace lines 1903, 1923, 1967:
let ptr = self.get_local_var(&name)?;
```

**Files to Modify**:
- `src/ir/builder.rs` - Add helper method and update 3 call sites

---

#### 2. Codegen LLVM Operations

**Location**: `src/codegen/llvm/codegen.rs` (multiple locations)

**Pattern Found**:
```rust
// Common patterns:
let value = self.values.get(&var).unwrap();
let func = self.module.get_function("name").unwrap();
```

**Solution Strategy**:
1. Add a `CodegenError` type with bilingual messages
2. Replace unwraps with proper error propagation
3. Use `ok_or_else` with context

**Example Fix**:
```rust
// Before:
let value = self.values.get(&var).unwrap();

// After:
let value = self.values.get(&var).ok_or_else(|| {
    CodegenError::new(
        format!("Variable {:?} not found during code generation", var),
        format!("المتغير {:?} غير موجود أثناء توليد الكود", var),
    )
})?;
```

---

#### 3. Package Cache Operations

**Location**: `src/package/cache.rs` (~26 unwraps)

**Pattern Found**:
```rust
// File operations
std::fs::create_dir_all(&path).unwrap();
std::fs::write(&path, content).unwrap();
```

**Solution**: Convert to proper `PackageResult` error handling.

```rust
// Before:
std::fs::create_dir_all(&path).unwrap();

// After:
std::fs::create_dir_all(&path).map_err(|e| {
    PackageError::IoError(format!("Failed to create cache directory: {}", e))
})?;
```

---

### Low-Priority Fixes (v1.2+)

#### 4. Interpreter Executor

The interpreter has ~6 unwraps that should be converted to RuntimeError.

#### 5. Debug Context

Debug-only code with ~12 unwraps, lower priority as it's for development.

---

### Implementation Strategy

#### Phase 1: Critical Path (v1.1)
1. Fix IR Builder variable lookups (3 unwraps) - HIGH IMPACT
2. Add helper functions for common patterns
3. Target: Reduce to <150 unwraps

#### Phase 2: Codegen Safety (v1.1)
1. Add CodegenError type
2. Fix LLVM operation unwraps (26 unwraps)
3. Target: Reduce to <125 unwraps

#### Phase 3: Package Manager (v1.1)
1. Fix cache operation unwraps (26 unwraps)
2. Add proper error messages for file operations
3. Target: Reduce to <100 unwraps

#### Phase 4: Remaining Cleanup (v1.2)
1. Interpreter unwraps
2. Debug context unwraps
3. Edge case handling
4. Target: Reduce to <75 unwraps

---

### New Error Types to Add

```rust
// src/ir/error.rs (or add to builder.rs)
#[derive(Debug, Clone)]
pub struct IrError {
    pub message: String,
    pub message_ar: String,
    pub span: Option<Span>,
}

impl IrError {
    pub fn internal(msg: impl Into<String>, msg_ar: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            message_ar: msg_ar.into(),
            span: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

// src/codegen/error.rs (new file)
#[derive(Debug, Clone)]
pub struct CodegenError {
    pub message: String,
    pub message_ar: String,
}

impl CodegenError {
    pub fn new(msg: impl Into<String>, msg_ar: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            message_ar: msg_ar.into(),
        }
    }
}
```

---

### Testing Strategy

1. **Unit Tests for Error Paths**
   ```rust
   #[test]
   fn test_ir_builder_undefined_var_error() {
       // Force an error condition and verify proper error type
   }
   ```

2. **Integration Tests**
   ```rust
   #[test]
   fn test_compilation_with_invalid_input() {
       // Ensure graceful error handling, no panics
   }
   ```

3. **Fuzz Testing** (optional, v1.3)
   - Use `cargo-fuzz` to find edge cases that trigger panics

---

## Implementation Checklist

### Task 1.1.4: Complete Incomplete Features
- [ ] TODO #1: Add Copy instruction to IR
  - [ ] Add to `src/ir/instruction.rs`
  - [ ] Update `src/ir/opt/inline.rs`
  - [ ] Update optimization passes
  - [ ] Update codegen
  - [ ] Update interpreter
  - [ ] Add tests
- [ ] TODO #2: Arabic LSP error messages
  - [ ] Add translation function
  - [ ] Update error creation
  - [ ] Add tests
- [ ] TODO #3: Complete lockfile parsing
  - [ ] Add package parsing functions
  - [ ] Handle all source types
  - [ ] Parse dependencies
  - [ ] Add tests

### Task 1.1.5: Improve Error Handling
- [ ] Phase 1: IR Builder (3 unwraps)
  - [ ] Add `get_local_var` helper
  - [ ] Update lines 1903, 1923, 1967
  - [ ] Add tests
- [ ] Phase 2: Codegen (26 unwraps)
  - [ ] Add CodegenError type
  - [ ] Convert critical unwraps
  - [ ] Add tests
- [ ] Phase 3: Package Cache (26 unwraps)
  - [ ] Convert file operation unwraps
  - [ ] Add proper error messages
  - [ ] Add tests
- [ ] Verification
  - [ ] Run `grep -r "\.unwrap()" src/ --include="*.rs" | grep -v "#\[cfg(test)\]" | wc -l`
  - [ ] Target: <100 unwraps in production code

---

## Success Criteria

### 1.1.4 Complete Incomplete Features
- [ ] Zero TODO comments in production code
- [ ] All tests pass
- [ ] `cargo clippy` passes

### 1.1.5 Improve Error Handling
- [ ] <100 `unwrap()` calls in production code
- [ ] No panics on malformed input
- [ ] All error paths tested
- [ ] `cargo clippy` passes

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Breaking IR optimization | Medium | High | Comprehensive test suite |
| Lockfile parsing incompatibility | Low | Medium | Support both formats |
| Performance regression from error handling | Low | Low | Only add overhead on error paths |

---

## Time Estimate

| Task | Estimated Effort |
|------|------------------|
| TODO #1 (Inliner type fix) | 4-6 hours |
| TODO #2 (Arabic LSP errors) | 1-2 hours |
| TODO #3 (Lockfile parsing) | 3-4 hours |
| Error handling Phase 1 | 2-3 hours |
| Error handling Phase 2 | 4-6 hours |
| Error handling Phase 3 | 3-4 hours |
| Testing & verification | 2-3 hours |
| **Total** | **19-28 hours** |

---

## Appendix A: All TODO Locations

```
src/ir/opt/inline.rs:281:        ty: IrType::Int, // TODO: proper type handling
src/lsp/analysis/document.rs:85:            msg.clone(), // TODO: Arabic error messages
src/package/lockfile.rs:165:    let packages = Vec::new(); // TODO: Full package parsing
```

## Appendix B: High-Risk unwrap() Locations

```
src/ir/builder.rs:1903:let ptr = self.lookup_var(&name).unwrap();
src/ir/builder.rs:1923:let ptr = self.lookup_var(&name).unwrap();
src/ir/builder.rs:1967:let ptr = self.lookup_var(&name).unwrap();
```

## Appendix C: File-by-File unwrap() Count

| File | Count | Test Code | Production |
|------|-------|-----------|------------|
| src/package/cache.rs | 26 | 0 | 26 |
| src/codegen/llvm/codegen.rs | 26 | 0 | 26 |
| src/debug/tests.rs | 25 | 25 | 0 |
| src/interpreter/executor_tests.rs | 45 | 45 | 0 |
| src/parser/parser_tests.rs | 82 | 82 | 0 |
| src/codegen/llvm/codegen_tests.rs | 60 | 60 | 0 |
| ... | ... | ... | ... |

---

**Document Version**: 1.0
**Created**: 2025-12-24
**Author**: Claude Code Assistant
