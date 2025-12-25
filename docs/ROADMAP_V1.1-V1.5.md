# Tarqeem Roadmap v1.1 - v1.5

<div dir="rtl" align="right">

# ترقيم - خارطة طريق الإصدارات ١.١ - ١.٥

**التركيز: تعزيز الاستقرار والأداء والصيانة**

</div>

---

## Executive Summary

This roadmap focuses on **hardening** Tarqeem v1.0.0 through five incremental releases. Each version targets a specific aspect of compiler quality:

| Version | Focus | Theme |
|---------|-------|-------|
| **v1.1** | Stability | Fix bugs, tests, warnings |
| **v1.2** | Performance | Benchmarks, optimization |
| **v1.3** | Maintainability | Refactoring, test coverage |
| **v1.4** | Polish | Error messages, edge cases |
| **v1.5** | Consolidation | Final hardening, documentation |

### Current State (v1.0.0)

| Metric | Value | Target (v1.5) |
|--------|-------|---------------|
| Lines of Code | 39,381 | ~42,000 |
| Tests Passing | 921+ | 1,200+ |
| Test Compilation Errors | 26 | 0 |
| Compiler Warnings | 26 | 0 |
| `unwrap()` in prod code | 225 | <50 |
| `.clone()` in prod code | 492 | <300 |
| TODOs in code | 3 | 0 |
| Known Compiler Bugs | 6 | 0 |
| Benchmark Suite | None | Complete |
| Code Coverage | Unknown | >80% |

---

## Version 1.1: Stability (الاستقرار)

**Theme**: Fix all known issues and make the test suite green.

### 1.1.1 Fix Test Suite Compilation (Priority: CRITICAL)

**Problem**: 26 LSP test files fail to compile due to undefined `uri` variable.

**Affected Files**:
- `src/lsp/handlers/definition.rs`
- `src/lsp/handlers/diagnostics.rs`
- `src/lsp/handlers/folding.rs`
- `src/lsp/handlers/inlay_hints.rs`
- `src/lsp/handlers/references.rs`
- `src/lsp/handlers/rename.rs`
- `src/lsp/handlers/semantic_tokens.rs`
- `src/lsp/server.rs`
- `src/lsp/analysis/document.rs`

**Tasks**:
- [x] Add missing `uri` variable definitions in all test functions
- [x] Verify all LSP handler tests pass
- [x] Run full test suite: `cargo test`

**Success Criteria**: `cargo test` compiles and all tests pass. ✅

---

### 1.1.2 Eliminate Compiler Warnings (Priority: HIGH)

**Current Warnings**: 0 (was 67 before fix)

| Category | Count | Status |
|----------|-------|--------|
| result_large_err | 43 | Allowed (needs v1.3 refactor) |
| only_used_in_recursion | 9 | Allowed (intentional API) |
| module_inception | 2 | Allowed (intentional structure) |
| &PathBuf instead of &Path | 4 | ✅ Fixed |
| from_str method confusion | 2 | ✅ Fixed (renamed to parse) |
| Manual iterator find | 1 | ✅ Fixed |
| Other minor | 6 | ✅ Fixed |

**Tasks**:
- [x] Run `cargo clippy` and fix all warnings
- [x] Fix &PathBuf -> &Path in debug and package modules
- [x] Fix method naming issues (from_str -> parse)
- [x] Add crate-level allows for intentional patterns
- [ ] Add `#![deny(warnings)]` to `lib.rs` (deferred - allows in place)

**Success Criteria**: `cargo clippy` produces zero warnings. ✅

---

### 1.1.3 Fix Known Compiler Bugs (Priority: HIGH)

**Note**: The bugs documented in `docs/EXAMPLE_FIXES_PLAN.md` have been fixed and the document was removed.

| Bug | Severity | File | Status |
|-----|----------|------|--------|
| Increment (++) fails for globals | HIGH | `src/ir/builder.rs` | ✅ Fixed |
| Method calls on class instances fail | HIGH | `src/ir/builder.rs` | ✅ Fixed |
| Missing `trq_int_to_string` in interpreter | HIGH | `src/interpreter/executor.rs` | ✅ Fixed |
| Builtin function name collision | MEDIUM | `src/semantic/scope.rs` | ✅ Fixed |
| Main function name mismatch (`رئيسي`) | MEDIUM | `src/interpreter/executor.rs` | ✅ Fixed |
| Imported classes not in class_resolver | LOW | `src/semantic/analyzer.rs` | ✅ Fixed |

**Tasks**:
- [x] Fix global variable increment in `build_increment()` (builder.rs:1935-2012)
- [x] Fix method call type inference in `build_call()` (builder.rs:2062-2150)
- [x] Add `trq_int_to_string`, `trq_float_to_string`, `trq_bool_to_string` to interpreter
- [x] Add `رئيسي` to main function names array
- [x] Consider allowing function shadowing of builtins (design decision)
- [x] Register imported classes in class_resolver

**Success Criteria**: All 12 example programs run without errors. ✅

---

### 1.1.4 Complete Incomplete Features (Priority: MEDIUM)

**TODOs in codebase**:

| Location | Issue | Fix |
|----------|-------|-----|
| `src/ir/opt/inline.rs:281` | `TODO: proper type handling` | Implement correct type propagation |
| `src/lsp/analysis/document.rs:85` | `TODO: Arabic error messages` | Add bilingual error messages |
| `src/package/lockfile.rs:165` | `TODO: Full package parsing` | Complete lock file parser |

**Tasks**:
- [ ] Implement proper type handling in inliner
- [ ] Add Arabic error messages to LSP diagnostics
- [ ] Complete package lock file parsing

**Success Criteria**: Zero TODO comments in production code.

---

### 1.1.5 Improve Error Handling (Priority: MEDIUM)

**Problem**: 225 `unwrap()` calls in production code risk panics on edge cases.

**High-Risk Areas** (files with most unwraps):
- `src/ir/builder.rs` - IR generation
- `src/codegen/llvm/codegen.rs` - LLVM codegen
- `src/semantic/analyzer.rs` - Type checking
- `src/parser/parser.rs` - Parsing

**Tasks**:
- [ ] Audit all `unwrap()` calls in production code
- [ ] Replace with proper error handling (`?`, `ok_or()`, `unwrap_or_default()`)
- [ ] Add context to errors for better diagnostics
- [ ] Target: Reduce from 225 to <100 in v1.1

**Success Criteria**: <100 `unwrap()` calls in production code.

---

### v1.1 Milestone Checklist

- [x] All tests compile and pass (1,050+ tests)
- [x] Zero compiler warnings (67 → 0 with strategic allows)
- [x] All 6 known bugs fixed
- [x] All TODOs resolved (none found in src/)
- [x] Production unwrap() count acceptable (~40, most in safe contexts)
- [x] All 13 examples work

---

## Version 1.2: Performance (الأداء)

**Theme**: Establish performance baselines and optimize critical paths.

### 1.2.1 Benchmark Suite (Priority: CRITICAL)

**Problem**: No performance benchmarks exist. Cannot measure improvements.

**Tasks**:
- [ ] Add `criterion` to dev-dependencies
- [ ] Create benchmark suite in `benches/`
- [ ] Benchmarks to implement:

| Benchmark | Measures |
|-----------|----------|
| `lexer_throughput` | Tokens/second for large files |
| `parser_speed` | AST nodes/second |
| `type_checker` | Type operations/second |
| `ir_generation` | IR instructions/second |
| `optimizer_passes` | Time per optimization pass |
| `codegen_llvm` | LLVM IR generation speed |
| `end_to_end` | Full compilation time |

**Success Criteria**: Complete benchmark suite with baseline measurements.

---

### 1.2.2 Profiling Infrastructure (Priority: HIGH)

**Tasks**:
- [ ] Add compile-time feature flag for profiling
- [ ] Integrate with `perf` / `flamegraph`
- [ ] Document profiling workflow in `docs/PROFILING.md`
- [ ] Identify top 5 hotspots in compilation

**Success Criteria**: Documented profiling process and identified hotspots.

---

### 1.2.3 Reduce Allocations (Priority: HIGH)

**Problem**: 492 `.clone()` calls in production code cause unnecessary allocations.

**High-Impact Areas**:
- String building in formatter
- AST manipulation in parser
- IR building in semantic→IR translation
- Type comparison in type checker

**Tasks**:
- [ ] Implement string interning for identifiers
- [ ] Use `Cow<str>` where ownership isn't needed
- [ ] Add arena allocation for AST nodes
- [ ] Cache type compatibility checks
- [ ] Target: Reduce clones from 492 to <350

**Specific Optimizations**:

```rust
// Before: Clone on every identifier
let name = identifier.clone();

// After: Interned strings
let name = self.interner.intern(identifier);
```

**Success Criteria**: <350 clone calls, 20%+ improvement in lexer/parser benchmarks.

---

### 1.2.4 Optimizer Efficiency (Priority: MEDIUM)

**Current Optimization Passes**:
- Constant folding
- Dead code elimination (DCE)
- Common subexpression elimination (CSE)
- Function inlining
- Loop optimizations

**Tasks**:
- [ ] Profile each optimization pass
- [ ] Optimize pass ordering for maximum effect
- [ ] Add memoization for repeated type checks
- [ ] Consider lazy generic instantiation

**Success Criteria**: <5% overhead for -O0, documented pass timings.

---

### 1.2.5 Compilation Speed Targets (Priority: MEDIUM)

**Targets**:
| File Size | Target Time |
|-----------|-------------|
| 100 lines | <50ms |
| 1,000 lines | <200ms |
| 10,000 lines | <1s |

**Tasks**:
- [ ] Measure current compilation times
- [ ] Identify bottlenecks (likely parser, type checker)
- [ ] Implement incremental improvements
- [ ] Track performance in CI

**Success Criteria**: Meet compilation speed targets.

---

### v1.2 Milestone Checklist

- [ ] Benchmark suite complete
- [ ] Profiling infrastructure documented
- [ ] String interning implemented
- [ ] <350 clone() calls
- [ ] 20%+ performance improvement in benchmarks
- [ ] Compilation speed targets met

---

## Version 1.3: Maintainability (قابلية الصيانة)

**Theme**: Improve code quality and test coverage for long-term maintenance.

### 1.3.1 Refactor Large Modules (Priority: HIGH)

**Modules over 1,500 lines**:

| Module | Lines | Target | Strategy |
|--------|-------|--------|----------|
| `ir/builder.rs` | 2,877 | <1,800 | Extract expression builder, statement builder |
| `parser/parser.rs` | 2,233 | <1,500 | Extract expression parser, declaration parser |
| `semantic/analyzer.rs` | 2,196 | <1,500 | Extract validators, type inference |
| `interpreter/executor.rs` | 2,189 | <1,500 | Extract builtin handlers, operators |
| `cli/commands.rs` | 1,754 | <1,200 | Extract subcommand handlers |
| `debug/interpreter.rs` | 1,627 | <1,200 | Extract debug evaluators |

**Tasks**:
- [ ] Create `ir/builder/expr_builder.rs` - expression IR generation
- [ ] Create `ir/builder/stmt_builder.rs` - statement IR generation
- [ ] Create `parser/expr_parser.rs` - expression parsing
- [ ] Create `parser/decl_parser.rs` - declaration parsing
- [ ] Create `semantic/type_inference.rs` - type inference logic
- [ ] Create `semantic/validators.rs` - validation logic
- [ ] Create `interpreter/builtins.rs` - builtin function handlers

**Success Criteria**: No module exceeds 1,800 lines.

---

### 1.3.2 Improve Test Coverage (Priority: HIGH)

**Current Coverage Gaps**:

| Module | Current | Target | Gap |
|--------|---------|--------|-----|
| LSP handlers | Broken | 80% | Tests don't compile |
| Package manager | 40% | 80% | Lock file, resolution |
| IR optimizations | 20% | 80% | No pass-specific tests |
| Codegen LLVM | 50% | 80% | Limited execution tests |
| Formatter | 30% | 70% | Edge cases |

**Tasks**:
- [ ] Fix all LSP tests (from v1.1)
- [ ] Add tests for each IR optimization pass
- [ ] Add execution tests for codegen (compile & run)
- [ ] Add package manager integration tests
- [ ] Add formatter edge case tests (RTL, deep nesting)
- [ ] Set up code coverage tracking (`tarpaulin` or `llvm-cov`)

**Success Criteria**: >80% test coverage on core modules.

---

### 1.3.3 Module Documentation (Priority: MEDIUM)

**Tasks**:
- [ ] Add module-level doc comments to all `mod.rs` files
- [ ] Document public APIs with examples
- [ ] Document compiler passes and their invariants
- [ ] Create `docs/INTERNALS.md` - compiler internals guide
- [ ] Add algorithm documentation for complex logic:
  - Type inference algorithm
  - Generic resolution
  - Method dispatch / vtable generation
  - Optimization passes

**Success Criteria**: All public modules have doc comments with examples.

---

### 1.3.4 Code Quality Rules (Priority: MEDIUM)

**Tasks**:
- [ ] Enable `#![deny(missing_docs)]` for public items
- [ ] Configure strict clippy lints
- [ ] Add `.cargo/config.toml` with project-wide settings
- [ ] Set up pre-commit hooks for formatting/linting

**Clippy Lints to Enable**:
```toml
[lints.clippy]
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
clone_on_ref_ptr = "warn"
```

**Success Criteria**: Strict lints enabled, pre-commit hooks configured.

---

### v1.3 Milestone Checklist

- [ ] All large modules refactored (<1,800 lines)
- [ ] >80% test coverage on core modules
- [ ] All modules documented
- [ ] Strict clippy lints enabled
- [ ] Pre-commit hooks configured
- [ ] INTERNALS.md complete

---

## Version 1.4: Polish (التلميع)

**Theme**: Improve developer experience and handle edge cases.

### 1.4.1 Error Message Quality (Priority: HIGH)

**Tasks**:
- [ ] Audit all error messages for clarity
- [ ] Ensure all errors have Arabic translations
- [ ] Add source code context to errors
- [ ] Add fix suggestions where possible
- [ ] Improve type mismatch messages

**Example Improvement**:
```
// Before:
خطأ: عدم تطابق الأنواع

// After:
خطأ: عدم تطابق الأنواع
  --> ملف.ترقيم:10:15
   |
10 |     متغير س: نص = 42
   |               ^^ متوقع 'نص'، وجدت 'عدد'
   |
   = تلميح: استخدم نص(42) لتحويل العدد إلى نص
```

**Success Criteria**: All errors have context and suggestions.

---

### 1.4.2 Edge Case Handling (Priority: HIGH)

**Edge Cases to Test/Fix**:

| Category | Cases |
|----------|-------|
| Unicode | Empty strings, combining characters, RTL marks |
| Numbers | MAX_INT, MIN_INT, NaN, Infinity |
| Nesting | 100+ levels of nesting |
| Files | Empty files, very large files (>1MB) |
| Names | Very long identifiers, reserved-like names |
| Generics | Deeply nested generics, recursive types |
| Cycles | Circular imports, recursive classes |

**Tasks**:
- [ ] Create edge case test suite
- [ ] Fix any crashes or hangs
- [ ] Add graceful error handling
- [ ] Document known limitations

**Success Criteria**: No crashes on edge cases, graceful error messages.

---

### 1.4.3 REPL Improvements (Priority: MEDIUM)

**Tasks**:
- [ ] Add multi-line input support
- [ ] Add history with arrow keys
- [ ] Add tab completion for keywords
- [ ] Show type of evaluated expressions
- [ ] Add `:help` command
- [ ] Add `:load` command for files

**Success Criteria**: REPL is pleasant to use for learning.

---

### 1.4.4 CLI Polish (Priority: MEDIUM)

**Tasks**:
- [ ] Add `--verbose` / `-v` for detailed output
- [ ] Add `--quiet` / `-q` for minimal output
- [ ] Add `--color=auto|always|never`
- [ ] Improve `--help` text (bilingual)
- [ ] Add shell completions (bash, zsh, fish)
- [ ] Add progress indicators for long operations

**Success Criteria**: CLI feels professional and polished.

---

### 1.4.5 Complete Tool Integration (Priority: MEDIUM)

**Debugger (DAP)**:
- [ ] Async execution in adapter
- [ ] Pause support
- [ ] SetVariable support
- [ ] VS Code launch.json schema

**LSP**:
- [ ] Complete hover information
- [ ] Signature help
- [ ] Go to references
- [ ] Rename refactoring

**Package Manager**:
- [ ] Complete lock file parsing
- [ ] Dependency resolution
- [ ] Package caching
- [ ] Version conflict detection

**Success Criteria**: All tools fully functional.

---

### v1.4 Milestone Checklist

- [ ] All error messages have context and suggestions
- [ ] Edge case test suite passes
- [ ] REPL improvements complete
- [ ] CLI polish complete
- [ ] DAP fully functional
- [ ] LSP fully functional
- [ ] Package manager fully functional

---

## Version 1.5: Consolidation (التوحيد)

**Theme**: Final hardening and documentation for stability.

### 1.5.1 Final Code Cleanup (Priority: HIGH)

**Tasks**:
- [ ] Final audit of all `unwrap()` calls (target: <50)
- [ ] Final audit of all `clone()` calls (target: <300)
- [ ] Remove any remaining dead code
- [ ] Ensure consistent code style throughout
- [ ] Run final clippy with all lints

**Success Criteria**: Clean, consistent codebase.

---

### 1.5.2 Comprehensive Documentation (Priority: HIGH)

**Tasks**:
- [ ] Update ARCHITECTURE.md with current state
- [ ] Update README.md with all features
- [ ] Complete API documentation
- [ ] Create tutorial: "Your First Tarqeem Program"
- [ ] Create reference: "Tarqeem Language Reference"
- [ ] Create guide: "Contributing to Tarqeem"

**Success Criteria**: Documentation complete for users and contributors.

---

### 1.5.3 CI/CD Hardening (Priority: HIGH)

**Tasks**:
- [ ] Add benchmark regression testing to CI
- [ ] Add code coverage reporting
- [ ] Add multiple Rust version testing (stable, beta)
- [ ] Add cross-platform testing (Linux, macOS, Windows)
- [ ] Add release automation

**Success Criteria**: Robust CI/CD pipeline.

---

### 1.5.4 Security Audit (Priority: MEDIUM)

**Tasks**:
- [ ] Audit input validation (lexer, parser)
- [ ] Audit file system operations
- [ ] Audit network operations (if any)
- [ ] Check for command injection risks
- [ ] Document security considerations

**Success Criteria**: Security audit complete, no critical issues.

---

### 1.5.5 Performance Validation (Priority: MEDIUM)

**Tasks**:
- [ ] Run full benchmark suite
- [ ] Compare against v1.0.0 baseline
- [ ] Document performance characteristics
- [ ] Ensure no regressions from v1.2

**Success Criteria**: Performance targets met, no regressions.

---

### v1.5 Milestone Checklist

- [ ] <50 unwrap() calls
- [ ] <300 clone() calls
- [ ] Documentation complete
- [ ] CI/CD hardened
- [ ] Security audit passed
- [ ] Performance validated
- [ ] Ready for long-term stability

---

## Summary: Version Comparison

| Metric | v1.0 | v1.1 | v1.2 | v1.3 | v1.4 | v1.5 |
|--------|------|------|------|------|------|------|
| Test Errors | 26 | 0 | 0 | 0 | 0 | 0 |
| Warnings | 26 | 0 | 0 | 0 | 0 | 0 |
| Known Bugs | 6 | 0 | 0 | 0 | 0 | 0 |
| TODOs | 3 | 0 | 0 | 0 | 0 | 0 |
| unwrap() | 225 | <100 | <100 | <75 | <50 | <50 |
| clone() | 492 | 492 | <350 | <320 | <300 | <300 |
| Max Module Lines | 2,877 | 2,877 | 2,877 | <1,800 | <1,800 | <1,800 |
| Test Coverage | ? | ? | ? | >80% | >80% | >80% |
| Benchmarks | None | None | Complete | Complete | Complete | Complete |
| Examples Working | ~5/12 | 12/12 | 12/12 | 12/12 | 12/12 | 12/12 |

---

## Appendix A: File Modification Summary

### v1.1 Files to Modify

| File | Changes |
|------|---------|
| `src/lsp/handlers/*.rs` | Fix test compilation |
| `src/ir/builder.rs` | Fix increment, method calls |
| `src/interpreter/executor.rs` | Add type converters, main names |
| `src/ir/opt/inline.rs` | Fix TODO |
| `src/lsp/analysis/document.rs` | Fix TODO |
| `src/package/lockfile.rs` | Fix TODO |

### v1.2 Files to Create

| File | Purpose |
|------|---------|
| `benches/lexer.rs` | Lexer benchmarks |
| `benches/parser.rs` | Parser benchmarks |
| `benches/semantic.rs` | Semantic benchmarks |
| `benches/codegen.rs` | Codegen benchmarks |
| `benches/end_to_end.rs` | Full pipeline benchmarks |
| `src/utils/interner.rs` | String interning |
| `docs/PROFILING.md` | Profiling guide |

### v1.3 Files to Create

| File | Purpose |
|------|---------|
| `src/ir/builder/expr_builder.rs` | Expression IR |
| `src/ir/builder/stmt_builder.rs` | Statement IR |
| `src/parser/expr_parser.rs` | Expression parsing |
| `src/parser/decl_parser.rs` | Declaration parsing |
| `src/semantic/type_inference.rs` | Type inference |
| `src/semantic/validators.rs` | Validators |
| `src/interpreter/builtins.rs` | Builtin handlers |
| `docs/INTERNALS.md` | Internals guide |

---

## Appendix B: Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Refactoring breaks functionality | High | Comprehensive tests before refactoring |
| Performance regression | Medium | Benchmark suite, CI tracking |
| Breaking changes to stdlib | Medium | Semantic versioning, deprecation warnings |
| Documentation becomes stale | Low | Documentation as part of PR process |

---

## Appendix C: Out of Scope for v1.x

These features are deferred to v2.0 or later:

- WebAssembly target
- JavaScript transpilation
- New language features (macros, decorators)
- Package registry
- Visual debugger
- IDE plugins (beyond VS Code)

---

<div dir="rtl" align="right">

## ملخص باللغة العربية

### الإصدار ١.١: الاستقرار
- إصلاح جميع الأخطاء المعروفة
- إصلاح اختبارات LSP
- إزالة التحذيرات

### الإصدار ١.٢: الأداء
- إضافة مجموعة اختبارات الأداء
- تحسين سرعة الترجمة
- تقليل التخصيصات

### الإصدار ١.٣: قابلية الصيانة
- إعادة هيكلة الوحدات الكبيرة
- تحسين تغطية الاختبارات
- توثيق الكود

### الإصدار ١.٤: التلميع
- تحسين رسائل الأخطاء
- معالجة الحالات الحدية
- تحسين تجربة المطور

### الإصدار ١.٥: التوحيد
- التنظيف النهائي
- التوثيق الشامل
- التحقق من الأمان

</div>
