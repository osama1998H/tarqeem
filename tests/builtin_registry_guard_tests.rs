//! Locks the builtin registry so it cannot grow, shrink, or drift silently.
//!
//! `Scope` holds the registry in three hand-maintained halves — `core_builtins()`,
//! the `get_stdlib_builtin()` match arms, and the `get_stdlib_module_exports()`
//! name lists — and until now nothing compared them. They happen to agree today;
//! that agreement was coincidence, not enforcement.
//!
//! These are **ratchet** tests. They pin the registry as it stands (29 core + 165
//! stdlib) while the builtin/stdlib boundary described in `docs/builtins-vs-stdlib.md`
//! is migrated. A name may only enter or leave the registry by editing the expected
//! list here, which is exactly the deliberate step the plan requires — a migration
//! commit deletes a builtin registration in the same commit that defines its stdlib
//! replacement, and this test is what refuses to let half of that land.
//!
//! What these tests deliberately do **not** assert: that every registered name has an
//! interpreter arm and a codegen mapping. 78 of them do not (see
//! `docs/builtins-inventory.md` §2.1), so that assertion belongs with the migration
//! that closes the hole, not with a guard that has to pass today.
//!
//! Like `runtime_symbols_tests.rs`, this builds nothing and runs nothing.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use tarqeem::semantic::Scope;

/// The complete set of globally-available builtins, sorted.
///
/// Adding a name here without an execution probe also fails
/// `builtins_execution_tests::test_every_core_builtin_agrees_across_backends`.
const CORE_BUILTINS: &[&str] = &[
    "ادخل",
    "ادخل_رسالة",
    "اطبع",
    "اطبع_خطأ",
    "اطبع_سطر",
    "الحق",
    "بتات_أو",
    "بتات_أو_حصري",
    "بتات_إزاحة_يسار",
    "بتات_إزاحة_يمين",
    "بتات_إزاحة_يمين_منطقية",
    "بتات_نفي",
    "بتات_و",
    "تأكد",
    "تأكد_رسالة",
    "توقف",
    "حرف_إلى_رمز",
    "رمز_إلى_حرف",
    "طباعة",
    "طول",
    "طول_مصفوفة",
    "قص_حروف",
    "عدد",
    "عدد_عشري",
    "منطقي",
    "نص",
    "نص_إلى_ثنائي",
    "ثنائي_إلى_نص",
    "نم",
    "نوع",
];

/// Names per stdlib module, as a ratchet on the size of each import surface.
const STDLIB_MODULE_SIZES: &[(&str, usize)] = &[
    ("رياضيات", 64),
    // 41 until #336, which took two names out for different reasons: `قص_حروف`
    // was promoted to the core tier (still callable, now with no import), and
    // `قص_نص` was removed outright — the primitive string surface is uniformly
    // codepoint-indexed, and byte work goes through `نص_إلى_ثنائي`/`ثنائي_إلى_نص`.
    ("نص", 39),
    ("ملفات", 21),
    ("وقت", 2),
    ("تشفير", 8),
    ("ضغط", 6),
    ("شبكة", 23),
];

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sorted(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut v: Vec<String> = names.into_iter().collect();
    v.sort();
    v
}

#[test]
fn core_builtin_set_is_locked() {
    let actual = sorted(Scope::core_builtin_names().into_iter().map(String::from));
    let expected = sorted(CORE_BUILTINS.iter().map(|s| s.to_string()));

    assert_eq!(
        actual, expected,
        "the core builtin set changed. Adding one is a deliberate act — it also needs an \
         IR lowering, an interpreter arm, a debug-interpreter arm, a codegen mapping and a \
         runtime symbol (see docs/builtins-inventory.md §1). Removing one is a breaking \
         change for user code. Update CORE_BUILTINS only alongside that work."
    );
}

#[test]
fn stdlib_module_list_is_locked() {
    let actual = sorted(Scope::get_stdlib_modules().iter().map(|s| s.to_string()));
    let expected = sorted(STDLIB_MODULE_SIZES.iter().map(|(m, _)| m.to_string()));

    assert_eq!(
        actual, expected,
        "the set of native-backed stdlib modules changed. This list is the short-circuit in \
         analyzer/stmt_analyzer.rs that stops an import from reaching disk, so adding a name \
         hides a stdlib/*.ترقيم file and removing one exposes it. The flip is all-or-nothing \
         per module: a module leaves this list only when its .ترقيم source answers every one \
         of its names."
    );
}

/// Every name a module advertises must actually resolve.
///
/// `get_stdlib_module_exports` expands a wildcard import while `get_stdlib_builtin`
/// answers each name; a name in the first with no arm in the second imports cleanly
/// and then fails to resolve.
#[test]
fn every_exported_stdlib_name_resolves() {
    let mut unresolved = Vec::new();

    for module in Scope::get_stdlib_modules() {
        for name in Scope::get_stdlib_module_exports(module) {
            if Scope::get_stdlib_builtin(module, name).is_none() {
                unresolved.push(format!("{module}::{name}"));
            }
        }
    }

    assert!(
        unresolved.is_empty(),
        "names exported by a stdlib module with no signature arm behind them: {unresolved:?}"
    );
}

/// …and the converse: every arm must be advertised.
///
/// An arm with no export listing resolves under a named import and vanishes under a
/// wildcard one. `get_stdlib_builtin` is a `match` with no enumeration hook, so this
/// scans the source the way `runtime_symbols_tests.rs` scans codegen.
#[test]
fn every_stdlib_signature_arm_is_exported() {
    let mut unexported = Vec::new();

    for (module, arms) in scan_stdlib_builtin_arms() {
        let exported: BTreeSet<&str> = Scope::get_stdlib_module_exports(&module)
            .into_iter()
            .collect();

        for arm in arms {
            if !exported.contains(arm.as_str()) {
                unexported.push(format!("{module}::{arm}"));
            }
        }
    }

    assert!(
        unexported.is_empty(),
        "names with a signature arm in get_stdlib_builtin that get_stdlib_module_exports \
         does not list, so a wildcard import cannot see them: {unexported:?}"
    );
}

#[test]
fn stdlib_registry_size_is_locked() {
    let mut drifted = Vec::new();

    for (module, expected) in STDLIB_MODULE_SIZES {
        let actual = Scope::get_stdlib_module_exports(module).len();
        if actual != *expected {
            drifted.push(format!("{module}: expected {expected}, found {actual}"));
        }
    }

    assert!(
        drifted.is_empty(),
        "stdlib module sizes drifted: {drifted:?}. Migrating a name out of the registry into \
         self-hosted stdlib is expected to shrink these — update the count in the same commit \
         that deletes the registration and adds the .ترقيم implementation."
    );

    let total: usize = STDLIB_MODULE_SIZES.iter().map(|(_, n)| n).sum();
    // 194 until #336. A promotion is size-neutral — `قص_حروف` left `نص` and
    // joined `CORE_BUILTINS` — so the whole of this step is `قص_نص`'s removal,
    // and it is the first name the migration has actually dropped rather than
    // moved.
    assert_eq!(
        total + CORE_BUILTINS.len(),
        193,
        "total registry size changed; docs/builtins-vs-stdlib.md targets 40 primitives — reached \
         by migrating ~150 names out and adding 21 new ones, so this number moves in both \
         directions, but only ever deliberately"
    );
}

/// Module name → the names of its `Some(builtin(…))` arms, read from the source.
///
/// The first argument to `builtin(` is always the name literal, in both the
/// single-line and wrapped forms rustfmt produces, so the first string literal after
/// each occurrence is the name. Module blocks are delimited by `"<module>" => match name {`.
fn scan_stdlib_builtin_arms() -> Vec<(String, Vec<String>)> {
    let path = project_root().join("src/semantic/scope.rs");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));

    let mut blocks: Vec<(String, usize)> = Scope::get_stdlib_modules()
        .iter()
        .filter_map(|module| {
            let marker = format!("\"{module}\" => match name {{");
            source
                .find(&marker)
                .map(|start| ((*module).to_string(), start))
        })
        .collect();
    blocks.sort_by_key(|(_, start)| *start);

    assert_eq!(
        blocks.len(),
        Scope::get_stdlib_modules().len(),
        "a stdlib module has no `\"<name>\" => match name {{` block in scope.rs, so this scan \
         would silently skip it — the dispatch shape changed and this test needs updating"
    );

    let mut out = Vec::new();
    for (index, (module, start)) in blocks.iter().enumerate() {
        let end = blocks
            .get(index + 1)
            .map(|(_, next)| *next)
            .unwrap_or(source.len());

        let names = source[*start..end]
            .match_indices("Some(builtin(")
            .filter_map(|(at, _)| first_string_literal(&source[*start + at..]))
            .collect();

        out.push((module.clone(), names));
    }
    out
}

fn first_string_literal(text: &str) -> Option<String> {
    let open = text.find('"')?;
    let rest = &text[open + 1..];
    let close = rest.find('"')?;
    Some(rest[..close].to_string())
}
