# Plan: Fix V1 Critical Issues 1.3, 1.4, 1.5

**Date:** 2024-12-22
**Target Issues:** 1.3, 1.4, 1.5 from V1_RELEASE_AUDIT.md
**Estimated Effort:** ~150-200 LOC + tests

---

## Overview

This plan addresses the three remaining critical issues blocking v1 release:

1. **Issue 1.3:** No Unicode Normalization in Scope Lookups
2. **Issue 1.4:** Generics Framework Disconnected from Semantic Analysis
3. **Issue 1.5:** Method Override Parameter Contravariance Not Checked

---

## Issue 1.3: Unicode Normalization in Scope Lookups

### Problem
Arabic identifiers can have multiple Unicode representations (e.g., composed vs decomposed forms). The lexer applies NFC normalization at tokenization, but `Scope.lookup()` and `Scope.define()` use direct string comparison. If any code path bypasses lexer normalization, identifiers won't match correctly.

Per CLAUDE.md: "NFC normalization is a critical invariant"

### Location
- `src/semantic/scope.rs`

### Current Behavior
```rust
pub fn define(&mut self, symbol: Symbol) -> bool {
    if self.symbols.contains_key(&symbol.name) {  // Direct comparison
        false
    } else {
        self.symbols.insert(symbol.name.clone(), symbol);
        true
    }
}

pub fn lookup(&self, name: &str) -> Option<&Symbol> {
    if let Some(symbol) = self.symbols.get(name) {  // Direct comparison
        Some(symbol)
    } else if let Some(parent) = &self.parent {
        parent.lookup(name)
    } else {
        None
    }
}
```

### Fix Strategy
Add NFC normalization helper and apply it in all symbol operations.

### Implementation
1. Add import at top of `scope.rs`:
   ```rust
   use unicode_normalization::UnicodeNormalization;
   ```

2. Add normalization helper:
   ```rust
   /// Normalize a string to NFC form for consistent identifier comparison
   fn normalize_name(name: &str) -> String {
       name.nfc().collect()
   }
   ```

3. Modify `define()`:
   ```rust
   pub fn define(&mut self, symbol: Symbol) -> bool {
       let normalized = normalize_name(&symbol.name);
       if self.symbols.contains_key(&normalized) {
           false
       } else {
           let mut symbol = symbol;
           symbol.name = normalized.clone();
           self.symbols.insert(normalized, symbol);
           true
       }
   }
   ```

4. Modify `lookup()`:
   ```rust
   pub fn lookup(&self, name: &str) -> Option<&Symbol> {
       let normalized = normalize_name(name);
       if let Some(symbol) = self.symbols.get(&normalized) {
           Some(symbol)
       } else if let Some(parent) = &self.parent {
           parent.lookup(name)
       } else {
           None
       }
   }
   ```

5. Apply same pattern to `lookup_local()` and `lookup_mut()`.

### Tests to Add in `scope_tests.rs`
```rust
#[test]
fn test_unicode_normalization_lookup() {
    use unicode_normalization::UnicodeNormalization;

    let mut scope = Scope::new_global();

    // "أحمد" in NFC form (precomposed)
    let nfc_name: String = "أحمد".nfc().collect();

    // "أحمد" in NFD form (decomposed) - same visually but different bytes
    let nfd_name: String = "أحمد".nfd().collect();

    // They should have different byte representations
    assert_ne!(nfc_name.as_bytes(), nfd_name.as_bytes());

    // Define with one form
    scope.define(Symbol::variable(&nfc_name, Type::Int, true));

    // Lookup with the other form should still work
    assert!(scope.lookup(&nfd_name).is_some());
}

#[test]
fn test_unicode_normalization_define() {
    use unicode_normalization::UnicodeNormalization;

    let mut scope = Scope::new_global();

    let nfc_name: String = "متغير".nfc().collect();
    let nfd_name: String = "متغير".nfd().collect();

    // Define with NFD form
    scope.define(Symbol::variable(&nfd_name, Type::Int, true));

    // Lookup with NFC form should work
    assert!(scope.lookup(&nfc_name).is_some());
}

#[test]
fn test_unicode_normalization_prevents_duplicate() {
    use unicode_normalization::UnicodeNormalization;

    let mut scope = Scope::new_global();

    let nfc_name: String = "س".nfc().collect();
    let nfd_name: String = "س".nfd().collect();

    // Define with NFC form
    assert!(scope.define(Symbol::variable(&nfc_name, Type::Int, true)));

    // Attempt to define with NFD form should fail (same identifier)
    assert!(!scope.define(Symbol::variable(&nfd_name, Type::String, true)));
}
```

---

## Issue 1.4: Generics Framework Disconnected from Semantic Analysis

### Problem
`GenericResolver` exists with full infrastructure but is marked as dead_code and never used:
```rust
#[allow(dead_code)]
generic_resolver: GenericResolver,
```

This means:
- Generic type parameters in classes/methods are NOT validated
- Type arguments at instantiation sites (e.g., `جديد قائمة<عدد>()`) are silently ignored

### Location
- `src/semantic/analyzer.rs`
- `src/semantic/generics.rs`

### Fix Strategy

**Phase 1: Basic Integration (v1 target)**
1. Remove `#[allow(dead_code)]` annotation
2. Add basic validation for type argument count at instantiation sites
3. Track generic type parameters when analyzing class/function declarations

**Phase 2: Full Integration (post-v1)**
- Full type argument substitution
- Generic constraint validation
- Inference from context

### Implementation (Phase 1)

1. Remove dead_code annotation from `analyzer.rs:21-22`:
   ```rust
   // Remove these lines:
   // #[allow(dead_code)]
   generic_resolver: GenericResolver,
   ```

2. Add method to push generic context when analyzing generic declarations:
   ```rust
   /// Enter a generic context for class/function with type parameters
   fn enter_generic_context(&mut self, type_params: &[String]) {
       let params: Vec<GenericParam> = type_params
           .iter()
           .map(|name| GenericParam::new(name.clone()))
           .collect();
       self.generic_resolver.push_context(GenericContext::with_parameters(params));
   }

   /// Exit the current generic context
   fn exit_generic_context(&mut self) {
       self.generic_resolver.pop_context();
   }
   ```

3. When analyzing class declarations with type parameters, push context:
   ```rust
   // In analyze_class_declaration or similar:
   if !class.type_params.is_empty() {
       self.enter_generic_context(&class.type_params);
   }
   // ... analyze class body ...
   if !class.type_params.is_empty() {
       self.exit_generic_context();
   }
   ```

4. When analyzing `New` expressions with type arguments, validate count:
   ```rust
   // In analyze_new_expression or similar:
   if let Some(class_info) = self.class_resolver.get_class(&class_name) {
       let expected_params = class_info.type_params.len();
       let provided_args = type_args.len();

       if provided_args != expected_params && provided_args != 0 {
           self.diagnostics.push(Diagnostic::error(
               &format!("Expected {} type arguments, got {}", expected_params, provided_args),
               &format!("متوقع {} معاملات نوع، وُجد {}", expected_params, provided_args),
               span,
           ));
       }
   }
   ```

### Tests to Add
```rust
#[test]
fn test_generic_class_instantiation_valid() {
    // Test: جديد قائمة<عدد>() with correct number of type args
    let source = r#"
        صنف قائمة<ن> {
            منشئ() {}
        }
        متغير ق = جديد قائمة<عدد>()
    "#;
    let result = analyze_source(source);
    assert!(result.is_ok());
}

#[test]
fn test_generic_class_instantiation_missing_args() {
    // Test: جديد قائمة() without type args (allowed - inferred)
    let source = r#"
        صنف قائمة<ن> {
            منشئ() {}
        }
        متغير ق = جديد قائمة()
    "#;
    let result = analyze_source(source);
    assert!(result.is_ok()); // Allowed for now - inference
}

#[test]
fn test_generic_class_instantiation_wrong_count() {
    // Test: جديد قائمة<عدد، نص>() with wrong number of args
    let source = r#"
        صنف قائمة<ن> {
            منشئ() {}
        }
        متغير ق = جديد قائمة<عدد، نص>()
    "#;
    let result = analyze_source(source);
    assert!(result.is_err());
}
```

---

## Issue 1.5: Method Override Parameter Contravariance Not Checked

### Problem
The `check_method_overrides()` function in `class_resolver.rs` only validates:
- Visibility restrictions
- Return type covariance

But it does NOT validate:
- Parameter count must match
- Parameter types must be contravariant (or at least compatible)

This allows unsound code like:
```tarqeem
صنف أ { دالة ف(x: عدد) {} }
صنف ب يرث أ { دالة ف(x: نص) {} }  // SHOULD ERROR
```

### Location
- `src/semantic/class_resolver.rs:694-748`

### Current Behavior (lines 694-748)
```rust
fn check_method_overrides(&mut self) {
    // ...
    // Check visibility is not more restrictive
    // Check return type is compatible
    // NO PARAMETER CHECKING
}
```

### Fix Strategy
Add parameter validation in `check_method_overrides()`:
1. Check parameter count matches
2. Check parameter types are compatible

### Implementation

Update `check_method_overrides()` at line 720 (after visibility check, before return type check):

```rust
fn check_method_overrides(&mut self) {
    let mut violations = Vec::new();

    for (_class_name, class) in &self.classes {
        if let Some(parent_name) = &class.parent {
            if let Some(parent) = self.classes.get(parent_name) {
                for (method_name, method) in &class.methods {
                    if let Some(parent_method) = parent.get_method(method_name, self) {
                        // Check visibility is not more restrictive
                        if method.visibility == Visibility::Private
                            && parent_method.visibility != Visibility::Private
                        {
                            violations.push((
                                format!(
                                    "Cannot override {} method '{}' with private visibility",
                                    format_visibility(parent_method.visibility),
                                    method_name
                                ),
                                format!(
                                    "لا يمكن تجاوز الدالة {} '{}' بخصوصية خاصة",
                                    format_visibility_ar(parent_method.visibility),
                                    method_name
                                ),
                                class.span,
                            ));
                        }

                        // NEW: Check parameter count matches
                        if method.params.len() != parent_method.params.len() {
                            violations.push((
                                format!(
                                    "Override of '{}' has {} parameters, but parent has {}",
                                    method_name,
                                    method.params.len(),
                                    parent_method.params.len()
                                ),
                                format!(
                                    "تجاوز الدالة '{}' لديه {} معاملات، لكن الأب لديه {}",
                                    method_name,
                                    method.params.len(),
                                    parent_method.params.len()
                                ),
                                class.span,
                            ));
                        } else {
                            // NEW: Check parameter types are compatible
                            for (i, ((_, child_ty), (_, parent_ty))) in method
                                .params
                                .iter()
                                .zip(parent_method.params.iter())
                                .enumerate()
                            {
                                // For contravariance, child parameter should accept
                                // at least what parent accepts.
                                // For simplicity in v1, we require exact match or
                                // child being a supertype (Any accepts anything).
                                if !child_ty.is_compatible_with(parent_ty)
                                    && !parent_ty.is_compatible_with(child_ty)
                                {
                                    violations.push((
                                        format!(
                                            "Parameter {} of '{}' has incompatible type '{}', expected '{}'",
                                            i + 1, method_name, child_ty, parent_ty
                                        ),
                                        format!(
                                            "المعامل {} للدالة '{}' له نوع غير متوافق '{}', المتوقع '{}'",
                                            i + 1, method_name, child_ty.arabic_name(), parent_ty.arabic_name()
                                        ),
                                        class.span,
                                    ));
                                }
                            }
                        }

                        // Check return type is compatible (existing code)
                        if !method
                            .return_type
                            .is_compatible_with(&parent_method.return_type)
                        {
                            violations.push((
                                format!(
                                    "Return type of '{}' is not compatible with parent",
                                    method_name
                                ),
                                format!(
                                    "نوع الإرجاع للدالة '{}' غير متوافق مع الأب",
                                    method_name
                                ),
                                class.span,
                            ));
                        }
                    }
                }
            }
        }
    }

    for (msg, msg_ar, span) in violations {
        self.diagnostics
            .push(Diagnostic::error(&msg, &msg_ar, span));
    }
}
```

### Tests to Add at end of `class_resolver.rs`

```rust
#[test]
fn test_method_override_same_params_valid() {
    let mut resolver = ClassResolver::new();
    resolver.register_class("أ", None, &[], Span::empty());
    resolver.register_class("ب", Some("أ"), &[], Span::empty());

    // Add method to parent
    if let Some(class) = resolver.get_class_mut("أ") {
        class.methods.insert(
            "دالة".to_string(),
            MethodInfo {
                name: "دالة".to_string(),
                params: vec![("س".to_string(), Type::Int)],
                return_type: Type::Void,
                visibility: Visibility::Public,
                is_static: false,
                is_async: false,
                is_abstract: false,
                vtable_index: None,
            },
        );
    }

    // Override with same parameter type
    if let Some(class) = resolver.get_class_mut("ب") {
        class.methods.insert(
            "دالة".to_string(),
            MethodInfo {
                name: "دالة".to_string(),
                params: vec![("س".to_string(), Type::Int)],
                return_type: Type::Void,
                visibility: Visibility::Public,
                is_static: false,
                is_async: false,
                is_abstract: false,
                vtable_index: None,
            },
        );
    }

    let result = resolver.validate();
    assert!(result.is_ok());
}

#[test]
fn test_method_override_incompatible_param_type() {
    let mut resolver = ClassResolver::new();
    resolver.register_class("أ", None, &[], Span::empty());
    resolver.register_class("ب", Some("أ"), &[], Span::empty());

    // Parent method with Int parameter
    if let Some(class) = resolver.get_class_mut("أ") {
        class.methods.insert(
            "دالة".to_string(),
            MethodInfo {
                name: "دالة".to_string(),
                params: vec![("س".to_string(), Type::Int)],
                return_type: Type::Void,
                visibility: Visibility::Public,
                is_static: false,
                is_async: false,
                is_abstract: false,
                vtable_index: None,
            },
        );
    }

    // Override with String parameter (incompatible!)
    if let Some(class) = resolver.get_class_mut("ب") {
        class.methods.insert(
            "دالة".to_string(),
            MethodInfo {
                name: "دالة".to_string(),
                params: vec![("س".to_string(), Type::String)],
                return_type: Type::Void,
                visibility: Visibility::Public,
                is_static: false,
                is_async: false,
                is_abstract: false,
                vtable_index: None,
            },
        );
    }

    let result = resolver.validate();
    assert!(result.is_err());
}

#[test]
fn test_method_override_wrong_param_count() {
    let mut resolver = ClassResolver::new();
    resolver.register_class("أ", None, &[], Span::empty());
    resolver.register_class("ب", Some("أ"), &[], Span::empty());

    // Parent method with 1 parameter
    if let Some(class) = resolver.get_class_mut("أ") {
        class.methods.insert(
            "دالة".to_string(),
            MethodInfo {
                name: "دالة".to_string(),
                params: vec![("س".to_string(), Type::Int)],
                return_type: Type::Void,
                visibility: Visibility::Public,
                is_static: false,
                is_async: false,
                is_abstract: false,
                vtable_index: None,
            },
        );
    }

    // Override with 2 parameters (wrong count!)
    if let Some(class) = resolver.get_class_mut("ب") {
        class.methods.insert(
            "دالة".to_string(),
            MethodInfo {
                name: "دالة".to_string(),
                params: vec![
                    ("س".to_string(), Type::Int),
                    ("ص".to_string(), Type::Int),
                ],
                return_type: Type::Void,
                visibility: Visibility::Public,
                is_static: false,
                is_async: false,
                is_abstract: false,
                vtable_index: None,
            },
        );
    }

    let result = resolver.validate();
    assert!(result.is_err());
}

#[test]
fn test_method_override_any_param_accepts_all() {
    let mut resolver = ClassResolver::new();
    resolver.register_class("أ", None, &[], Span::empty());
    resolver.register_class("ب", Some("أ"), &[], Span::empty());

    // Parent with Int parameter
    if let Some(class) = resolver.get_class_mut("أ") {
        class.methods.insert(
            "دالة".to_string(),
            MethodInfo {
                name: "دالة".to_string(),
                params: vec![("س".to_string(), Type::Int)],
                return_type: Type::Void,
                visibility: Visibility::Public,
                is_static: false,
                is_async: false,
                is_abstract: false,
                vtable_index: None,
            },
        );
    }

    // Override with Any parameter (valid - Any is contravariant supertype)
    if let Some(class) = resolver.get_class_mut("ب") {
        class.methods.insert(
            "دالة".to_string(),
            MethodInfo {
                name: "دالة".to_string(),
                params: vec![("س".to_string(), Type::Any)],
                return_type: Type::Void,
                visibility: Visibility::Public,
                is_static: false,
                is_async: false,
                is_abstract: false,
                vtable_index: None,
            },
        );
    }

    let result = resolver.validate();
    assert!(result.is_ok()); // Any accepts Int
}
```

---

## Implementation Order

1. **Issue 1.3 (Unicode Normalization)** - Simplest, standalone fix
   - Modify `scope.rs`
   - Add tests to `scope_tests.rs`
   - ~20 LOC changes + ~30 LOC tests

2. **Issue 1.5 (Parameter Contravariance)** - Self-contained in one file
   - Modify `class_resolver.rs`
   - Add tests to `class_resolver.rs`
   - ~40 LOC changes + ~80 LOC tests

3. **Issue 1.4 (Generics Integration)** - Most complex, Phase 1 only
   - Modify `analyzer.rs`
   - Add helper methods
   - ~50 LOC changes + ~40 LOC tests
   - Note: Full integration deferred to v1.1

---

## Verification

After each issue is fixed:
```bash
cargo fmt
cargo clippy
cargo test
```

Final verification:
```bash
cargo build --release
cargo test --release
```

---

## Risk Assessment

| Issue | Risk | Mitigation |
|-------|------|------------|
| 1.3 Unicode | Low - additive change | Comprehensive tests |
| 1.4 Generics | Medium - Phase 1 only | Clear scope limitation |
| 1.5 Override | Low - isolated change | Tests for edge cases |

---

## Summary

| Issue | Effort | Files Changed | Tests Added |
|-------|--------|---------------|-------------|
| 1.3 | ~20 LOC | 1 | 3 |
| 1.4 | ~50 LOC | 1-2 | 3 |
| 1.5 | ~40 LOC | 1 | 4 |
| **Total** | **~110 LOC** | **3-4** | **10** |
