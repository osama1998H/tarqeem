//! Consistency between the runtime symbols codegen declares and the ones
//! `runtime-rs` defines.
//!
//! Three separate bugs have now been the same drift: #185 (`طول`/`نوع` diverging
//! natively), #222 (names missing from the mapping table) and #241 (names in the
//! table with nothing defining them). Each surfaced as a link failure naming an
//! internal symbol — `undefined value '@trq_…'` — with no source location and no
//! Arabic diagnostic.
//!
//! No execution test can catch this class, because a symbol only reaches the
//! linker if some program happens to call it, and no `examples/*.ترقيم` calls
//! the date, time or base64 builtins. So this checks the two source trees
//! directly. It builds nothing and runs nothing, which is why it is not named
//! `*_execution_tests.rs`.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Symbols codegen declares that `runtime-rs` is not expected to define yet.
///
/// Entries are removed as their issues are fixed, and are **never** added to
/// silence fresh drift without filing one first — the same rule the
/// `KNOWN_DIVERGENT` list in `.github/workflows/examples.yml` carries.
const KNOWN_UNDEFINED: &[(&str, &str)] = &[
    // Native exception unwinding is designed but unimplemented; `build_throw`
    // refuses native lowering with ت٠٣٠٣ rather than emitting a call.
    ("trq_throw", "#238"),
    ("trq_get_exception", "#238"),
    // The nine date constructors return a field-bearing object (`.سنة`, `.ساعة`)
    // that has no representation below the semantic layer, so they cannot be
    // written until that return type is designed.
    ("trq_date_today", "#298"),
    ("trq_date_parse", "#298"),
    ("trq_date_from_timestamp", "#298"),
    ("trq_date_add_days", "#298"),
    ("trq_date_add_months", "#298"),
    ("trq_time_parse", "#298"),
    ("trq_datetime_now", "#298"),
    ("trq_datetime_from_timestamp", "#298"),
    ("trq_datetime_parse", "#298"),
];

/// Every `@trq_*` name codegen can emit a `declare` or a `call` for.
///
/// Codegen never assembles these names dynamically — every occurrence is a
/// literal — so scanning the source text has no blind spot.
fn declared_symbols() -> BTreeSet<String> {
    let path = project_root().join("src/codegen/llvm/codegen.rs");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));

    let mut symbols = BTreeSet::new();
    for (index, _) in source.match_indices("@trq_") {
        let name: String = source[index + 1..]
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
            .collect();
        // `@trq_*` appears in a doc comment; the wildcard stops the scan early
        // and leaves the bare prefix.
        if name != "trq_" {
            symbols.insert(name);
        }
    }
    symbols
}

/// Every symbol `runtime-rs` exports across the C ABI.
fn defined_symbols() -> BTreeSet<String> {
    let dir = project_root().join("runtime-rs/src");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", dir.display(), e))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("rs"))
        .collect();
    files.sort();

    assert!(
        !files.is_empty(),
        "no .rs files under {} — the scan would assert nothing",
        dir.display()
    );

    const MARKER: &str = "pub extern \"C\" fn ";
    let mut symbols = BTreeSet::new();
    for path in &files {
        let source = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
        for (index, _) in source.match_indices(MARKER) {
            let name: String = source[index + MARKER.len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if name.starts_with("trq_") {
                symbols.insert(name);
            }
        }
    }
    symbols
}

#[test]
fn test_every_declared_runtime_symbol_is_defined() {
    let declared = declared_symbols();
    let defined = defined_symbols();

    // A scan that silently found nothing would pass forever.
    assert!(
        declared.len() > 100,
        "only {} @trq_* symbols found in codegen — the scan is broken, not the code",
        declared.len()
    );

    let allowed: BTreeSet<&str> = KNOWN_UNDEFINED.iter().map(|(name, _)| *name).collect();
    let missing: Vec<String> = declared
        .iter()
        .filter(|name| !defined.contains(*name) && !allowed.contains(name.as_str()))
        .cloned()
        .collect();

    assert!(
        missing.is_empty(),
        "رموز يعلنها مولد الكود ولا يعرّفها وقت التشغيل، فيفشل الربط:\n  {}\n\
         Declared by codegen, defined nowhere in runtime-rs — these fail at link:\n  {}",
        missing.join("\n  "),
        missing.join("\n  "),
    );
}

#[test]
fn test_allow_list_does_not_outlive_its_issues() {
    let defined = defined_symbols();

    let stale: Vec<String> = KNOWN_UNDEFINED
        .iter()
        .filter(|(name, _)| defined.contains(*name))
        .map(|(name, issue)| format!("{} ({})", name, issue))
        .collect();

    assert!(
        stale.is_empty(),
        "these are defined now and must be dropped from KNOWN_UNDEFINED:\n  {}",
        stale.join("\n  "),
    );
}

#[test]
fn test_allow_list_entries_are_still_declared() {
    let declared = declared_symbols();

    // An entry naming a symbol codegen no longer declares is dead weight that
    // would mask a genuine regression if the name ever came back.
    let orphaned: Vec<String> = KNOWN_UNDEFINED
        .iter()
        .filter(|(name, _)| !declared.contains(*name))
        .map(|(name, issue)| format!("{} ({})", name, issue))
        .collect();

    assert!(
        orphaned.is_empty(),
        "these are no longer declared by codegen and must be dropped from KNOWN_UNDEFINED:\n  {}",
        orphaned.join("\n  "),
    );
}

#[test]
fn test_time_builtins_resolved_by_this_fix_are_defined() {
    let defined = defined_symbols();

    // The two symbols reachable from ordinary source (`استورد { وقت_الآن } من "وقت"`).
    // Guarding them by name keeps the regression explicit even if the broad
    // check above is ever relaxed.
    for name in [
        "trq_time_now",
        "trq_performance_now",
        "trq_day_of_week",
        "trq_base64_encode",
    ] {
        assert!(
            defined.contains(name),
            "{} is declared by codegen but not defined in runtime-rs",
            name
        );
    }
}
