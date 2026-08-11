//! Execution-based tests for instance properties (`خاصية`) — issue #239.
//!
//! Auto-property accessors are synthesized as real IR functions that read and
//! write a backing field `_{name}` by *index*. Both synthesis sites hardcoded
//! `index: 0`, so every auto-property on a class shared slot 0 — and a write
//! through one silently overwrote whatever real field occupied that slot.
//!
//! It stayed invisible because no test ever executed an instance property
//! through native codegen. The interpreter resolves `GetField`/`SetField` by
//! *name* and discards the index, so it is immune to a wrong index; the JIT
//! delegates to the interpreter for programs this short (Tier-0 never
//! promotes). Only the LLVM backend honours the index, and the one existing
//! property test — `oop_execution_tests.rs::test_static_auto_property` — is
//! both `مشترك` (an index-free global) and interpreter+JIT only.
//!
//! Hence the shape of this file: every fixture runs under **all three
//! backends** and is asserted against exact stdout. A test that does not link
//! natively cannot catch this class of bug.
//!
//! Fixtures always assign every property under test in the constructor.
//! Reading an unassigned auto-property exercises a *different*, still-open
//! defect (issue #251) — `build_new` emits no default initializers, so
//! `خاصية س: عدد = 7` reads as `لا_شيء` interpreted and `0` natively — which
//! would make these tests fail for the wrong reason.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::{fs, str};

use tempfile::TempDir;

const TARQEEM: &str = env!("CARGO_BIN_EXE_tarqeem");

/// The static runtime library native binaries link against, named as
/// `codegen::linker::find_runtime` names it.
const RUNTIME_LIB: &str = if cfg!(windows) { "trq.lib" } else { "libtrq.a" };

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Guarantees `libtrq.a` exists before the first native case links against it.
///
/// It is produced by the separate `tarqeem-runtime` crate, which a plain
/// `cargo test` of this package never builds — see the longer note in
/// `module_execution_tests.rs`. Never degrades to skipping the native backend:
/// that backend is the entire point of this file, so a runtime that cannot be
/// prepared has to fail loudly.
fn ensure_runtime_library() {
    static RUNTIME: OnceLock<Result<(), String>> = OnceLock::new();

    if let Err(message) = RUNTIME.get_or_init(build_runtime_library) {
        panic!("{message}");
    }
}

fn runtime_library_present() -> bool {
    if std::env::var("TARQEEM_RUNTIME_PATH").is_ok_and(|path| Path::new(&path).exists()) {
        return true;
    }

    runtime_archive().exists()
}

/// The profile this test binary was built with — the only one whose runtime
/// matches the compiler under test.
fn test_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

/// Deliberately the test binary's own profile only, unlike
/// `module_execution_tests.rs`, which accepts `target/release/libtrq.a` even
/// under a debug `cargo test`. A months-old release archive satisfying that
/// check would link the native leg against a runtime that does not correspond
/// to this checkout — hiding a runtime regression, or failing the link with an
/// `undefined reference` that reads like a compiler bug.
fn runtime_archive() -> PathBuf {
    project_root()
        .join("target")
        .join(test_profile())
        .join(RUNTIME_LIB)
}

fn build_runtime_library() -> Result<(), String> {
    if runtime_library_present() {
        return Ok(());
    }

    let expected = runtime_archive();

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut command = Command::new(&cargo);
    command
        .args(["build", "-p", "tarqeem-runtime"])
        .current_dir(project_root());
    if !cfg!(debug_assertions) {
        command.arg("--release");
    }

    match command.output() {
        Err(error) => Err(runtime_failure(&format!("تعذّر تشغيل «{cargo}»: {error}"))),
        Ok(output) if !output.status.success() => Err(runtime_failure(&format!(
            "فشل بناء tarqeem-runtime (الحالة/status {:?})\n--- stdout ---\n{}\n--- stderr ---\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ))),
        // A build that succeeds without producing the artifact sent it somewhere
        // the linker will not look — `CARGO_TARGET_DIR`, `--target-dir` or a
        // cross `--target` being the usual causes. Linking would fail next, with
        // a far less obvious message.
        Ok(_) if !expected.exists() => Err(runtime_failure(&format!(
            "نجح البناء لكن {} غير موجودة / build succeeded but the artifact is missing (CARGO_TARGET_DIR؟)",
            expected.display()
        ))),
        Ok(_) => Ok(()),
    }
}

/// Wraps a runtime-preparation failure with the command to run by hand, so the
/// panic reads as a missing prerequisite rather than as a broken test file.
fn runtime_failure(detail: &str) -> String {
    format!(
        "تعذّر تجهيز مكتبة وقت التشغيل {RUNTIME_LIB} اللازمة للربط الأصلي.\n\
         Could not prepare the runtime library required to link native binaries.\n\
         {detail}\n\
         نفّذها يدوياً / run it by hand: cargo build -p tarqeem-runtime{}",
        if cfg!(debug_assertions) {
            ""
        } else {
            " --release"
        }
    )
}

#[derive(Clone, Copy, Debug)]
enum Backend {
    Interpreter,
    Jit,
    Native,
}

impl Backend {
    const ALL: [Backend; 3] = [Backend::Interpreter, Backend::Jit, Backend::Native];
}

struct Output {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Output {
    fn succeeded(&self) -> bool {
        self.status == Some(0)
    }

    fn lines(&self) -> Vec<String> {
        self.stdout
            .trim()
            .lines()
            .map(|line| line.trim().to_string())
            .collect()
    }

    /// Both streams, labelled — a bare `assert_eq!` on stdout says nothing when
    /// the program errored out and printed to stderr instead.
    fn report(&self) -> String {
        format!(
            "الحالة/status: {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            self.status, self.stdout, self.stderr
        )
    }
}

/// Writes a fixture, adding the `بسم_الله` / `الحمد_لله` markers the parser
/// requires around every file.
fn write_main(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("رئيسي.ترقيم");
    fs::write(&path, format!("بسم_الله\n{}\nالحمد_لله\n", body.trim()))
        .expect("تعذّر كتابة الملف الرئيسي");
    path
}

/// A stale `TARQEEM_HOME` silently shadows this checkout; these fixtures import
/// nothing, so remove it outright.
fn tarqeem(args: &[&str], cwd: &Path) -> Output {
    let output = Command::new(TARQEEM)
        .args(args)
        .current_dir(cwd)
        .env_remove("TARQEEM_HOME")
        .output()
        .unwrap_or_else(|e| panic!("تعذّر تشغيل {} {:?}: {}", TARQEEM, args, e));

    Output {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn execute_binary(path: &Path) -> Output {
    let output = Command::new(path)
        .output()
        .unwrap_or_else(|e| panic!("تعذّر تشغيل الملف التنفيذي {}: {}", path.display(), e));

    Output {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Runs `main` under one backend. For `Native`, a failed compile is returned
/// as-is so its diagnostics reach the assertion message.
fn execute(backend: Backend, main: &Path, tag: &str) -> Output {
    let dir = main.parent().expect("للملف الرئيسي مجلد");
    let arg = main.to_str().expect("مسار صالح");

    match backend {
        Backend::Interpreter => tarqeem(&["run", arg], dir),
        Backend::Jit => tarqeem(&["run", "--jit", arg], dir),
        Backend::Native => {
            ensure_runtime_library();
            let exe = dir.join(format!("مخرج_{tag}"));
            let exe_arg = exe.to_str().expect("مسار صالح").to_string();
            let compiled = tarqeem(&["compile", arg, "-o", &exe_arg], dir);
            if !compiled.succeeded() {
                return compiled;
            }
            execute_binary(&exe)
        }
    }
}

/// Asserts `body` prints exactly `expected` under every backend.
///
/// Two assertions per backend, deliberately:
///
/// * `lines()` against a literal expectation. Comparing backends only against
///   each other would stay green if they all drifted together, so the
///   interpreter is not the reference — the literal is.
/// * raw stdout, byte for byte, against the first backend's. `lines()` trims
///   the whole stream and every line, so on its own it cannot see a backend
///   that prints `"3 "`, adds a blank line, or omits the trailing newline —
///   which is the very silent-divergence class this file exists to catch.
fn assert_prints(body: &str, expected: &[&str]) {
    let dir = TempDir::new().expect("تعذّر إنشاء مجلد مؤقت");
    let main = write_main(dir.path(), body);

    let mut reference: Option<(Backend, String)> = None;

    for backend in Backend::ALL {
        let output = execute(backend, &main, &format!("{backend:?}"));

        assert!(
            output.succeeded(),
            "فشل التنفيذ [{:?}]\nالمتوقع/expected: {:?}\n{}",
            backend,
            expected,
            output.report()
        );
        assert_eq!(
            output.lines(),
            expected,
            "خرج غير متطابق [{:?}]\nالمتوقع/expected: {:?}\n{}",
            backend,
            expected,
            output.report()
        );

        match &reference {
            None => reference = Some((backend, output.stdout.clone())),
            Some((first, first_stdout)) => assert_eq!(
                &output.stdout,
                first_stdout,
                "خرج غير متطابق حرفياً بين {:?} و{:?} / raw stdout differs byte for byte\n{}",
                first,
                backend,
                output.report()
            ),
        }
    }
}

/// `examples/خواص.ترقيم` in miniature: the reported symptom, where reading
/// `س` returned `ص`'s value because both accessors addressed slot 0.
#[test]
fn test_two_auto_properties_keep_distinct_slots() {
    assert_prints(
        r#"
صنف نقطة {
    خاصية س: عدد = 0
    خاصية ص: عدد = 0

    منشئ(س: عدد، ص: عدد) {
        هذا.س = س
        هذا.ص = ص
    }
}

متغير ن = جديد نقطة(3، 4)
اطبع(ن.س)
اطبع(ن.ص)
"#,
        &["3", "4"],
    );
}

/// Three properties, because at N=2 the last one is correct by coincidence:
/// the final write lands in the shared slot, so only the *earlier* properties
/// read back wrong. At N=3 the middle one is wrong too.
#[test]
fn test_three_auto_properties_keep_distinct_slots() {
    assert_prints(
        r#"
صنف ثلاثة {
    خاصية أ: عدد = 0
    خاصية ب: عدد = 0
    خاصية ج: عدد = 0

    منشئ(أ: عدد، ب: عدد، ج: عدد) {
        هذا.أ = أ
        هذا.ب = ب
        هذا.ج = ج
    }
}

متغير كائن = جديد ثلاثة(1، 2، 3)
اطبع(كائن.أ)
اطبع(كائن.ب)
اطبع(كائن.ج)
"#,
        &["1", "2", "3"],
    );
}

/// The damaging case: an auto-property write targeting slot 0 corrupted an
/// unrelated plain field that legitimately lives there. Interleaving the two
/// kinds is what pins the index rather than merely the aliasing.
#[test]
fn test_auto_property_write_does_not_clobber_plain_fields() {
    assert_prints(
        r#"
صنف مزيج {
    عام حقل_أول: عدد
    خاصية خ١: عدد = 0
    عام حقل_ثاني: عدد
    خاصية خ٢: عدد = 0

    منشئ() {
        هذا.حقل_أول = 11
        هذا.خ١ = 22
        هذا.حقل_ثاني = 33
        هذا.خ٢ = 44
    }
}

متغير كائن = جديد مزيج()
اطبع(كائن.حقل_أول)
اطبع(كائن.خ١)
اطبع(كائن.حقل_ثاني)
اطبع(كائن.خ٢)
"#,
        &["11", "22", "33", "44"],
    );
}

/// Properties with explicit `احصل`/`عيّن` lower their bodies as ordinary
/// statements over named backing fields, so they never used the hardcoded
/// index. Guards against the fix disturbing the path that already worked.
#[test]
fn test_full_accessor_properties_are_unaffected() {
    assert_prints(
        r#"
صنف شخص {
    خاص _اسم: نص
    خاص _عمر: عدد

    منشئ(اسم: نص، عمر: عدد) {
        هذا._اسم = اسم
        هذا._عمر = عمر
    }

    خاصية اسم: نص {
        احصل {
            أرجع هذا._اسم
        }
        عيّن(قيمة) {
            هذا._اسم = قيمة
        }
    }

    خاصية عمر: عدد {
        احصل {
            أرجع هذا._عمر
        }
    }
}

متغير ش = جديد شخص("أحمد"، 25)
اطبع(ش.اسم)
اطبع(ش.عمر)
ش.اسم = "محمد"
اطبع(ش.اسم)
"#,
        &["أحمد", "25", "محمد"],
    );
}

/// Compound assignment used to bypass the property setter entirely and emit a
/// bare `SetField` with the property's own name and `index: 0`. Two different
/// wrong answers resulted: the interpreter stored a by-name field no getter
/// reads (so `+=` vanished), while native wrote slot 0 and corrupted the *other*
/// property. Both paths now share `store_to_member`.
#[test]
fn test_compound_assignment_to_an_auto_property() {
    assert_prints(
        r#"
صنف نقطة {
    خاصية س: عدد = 0
    خاصية ص: عدد = 0

    منشئ(س: عدد، ص: عدد) {
        هذا.س = س
        هذا.ص = ص
    }
}

متغير ن = جديد نقطة(3، 4)
ن.ص += 10
ن.س -= 1
اطبع(ن.س)
اطبع(ن.ص)
"#,
        &["2", "14"],
    );
}

/// A class declared inside a function never reaches the top-level collection
/// pass, so its field layout does not exist when its accessors are synthesized
/// — which made `backing_field_index` abort the whole build for a program that
/// had run fine, and that `tarqeem check` still called clean. The layout is now
/// collected on demand.
///
/// The body only *declares* the class: `جديد محلي()` is rejected by the analyzer
/// with د٠٠٠٣ `صنف غير معروف`, a separate pre-existing limitation on
/// function-local classes. Declaring one must still not break the build.
#[test]
fn test_auto_properties_on_a_class_declared_inside_a_function() {
    assert_prints(
        r#"
دالة رئيسية() {
    صنف محلي {
        خاصية أ: عدد = 0
        خاصية ب: عدد = 0
    }
    اطبع("تم")
}
"#,
        &["تم"],
    );
}

/// Every backing-field index an auto-property accessor emits, keyed by
/// `Class::__accessor_prop`. Reads the IR directly, which is where the defect
/// lived — an execution test can only observe it through a backend that
/// honours the index, and only one of the three does.
///
/// Runs the analyzer before the IR builder, as every other test in `tests/`
/// does. Skipping it would let a fixture that does not type-check still assert
/// indices — describing a program the compiler rejects — and would break the
/// day `IrBuilder` requires analyzer-linked input for classes, as it already
/// does for the injected `استثناء` prelude.
fn accessor_field_indices(body: &str) -> Vec<(String, u32)> {
    use tarqeem::ir::{Instruction, IrBuilder};
    use tarqeem::parser::Parser;
    use tarqeem::semantic::Analyzer;

    let source = format!("بسم_الله\n{}\nالحمد_لله", body.trim());
    let ast = Parser::new(&source).parse().expect("تعذّر تحليل البرنامج");

    let mut analyzer = Analyzer::new();
    let stdlib_path = project_root().join("stdlib_trq");
    if stdlib_path.exists() {
        analyzer.add_search_path(stdlib_path);
    }
    if let Err(diagnostics) = analyzer.analyze(&ast) {
        panic!(
            "فشل التحليل الدلالي / semantic analysis failed: {}",
            diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    let module = IrBuilder::new("test".to_string())
        .build(&ast)
        .expect("تعذّر بناء التمثيل الوسيط");

    let mut found = Vec::new();
    for function in &module.functions {
        if !function.name.contains("__احصل_") && !function.name.contains("__عيّن_") {
            continue;
        }
        for block in &function.blocks {
            for instruction in &block.instructions {
                let index = match instruction {
                    Instruction::GetField { field, .. } | Instruction::SetField { field, .. } => {
                        field.index
                    }
                    _ => continue,
                };
                found.push((function.name.clone(), index));
            }
        }
    }
    found
}

/// Each auto-property's accessors must address that property's own slot. This
/// is the fix stated at the level it operates: `0, 0, 0` for three properties
/// is the bug, `0, 1, 2` is correct.
#[test]
fn test_accessors_address_their_own_backing_field_slot() {
    let indices = accessor_field_indices(
        r#"
صنف ثلاثة {
    خاصية أ: عدد = 0
    خاصية ب: عدد = 0
    خاصية ج: عدد = 0

    منشئ() {
        هذا.أ = 1
        هذا.ب = 2
        هذا.ج = 3
    }
}
اطبع(0)
"#,
    );

    for (property, expected) in [("أ", 0u32), ("ب", 1), ("ج", 2)] {
        for accessor in ["__احصل_", "__عيّن_"] {
            let name = format!("ثلاثة::{}{}", accessor, property);
            let actual = indices
                .iter()
                .find(|(function, _)| *function == name)
                .map(|(_, index)| *index);
            assert_eq!(
                actual,
                Some(expected),
                "الخاصية '{}' في {}: فهرس غير متوقع / unexpected slot\nالمرصود/observed: {:?}",
                property,
                name,
                indices
            );
        }
    }
}

/// Indices stay *own-class-relative* across an inheritance chain: codegen adds
/// `inherited_field_count` itself, so a subclass's first auto-property is slot
/// 0 of that subclass, not slot 1 of the flattened layout.
///
/// Asserted on the IR rather than by running the program: accessing an
/// inherited instance field natively is separately broken (issue #249) — the
/// subclass's LLVM struct type omits its parent's fields, so
/// `inherited_count + index` GEPs out of bounds (`invalid getelementptr
/// indices`, or a segfault). That reproduces with plain fields and no
/// properties at all, so it is not this issue. A super-constructor call that
/// writes an auto-property loses the write too (issue #250).
///
/// The parent deliberately carries two properties: the subclass's own property
/// is slot 0 own-relative but would be slot 2 under a flattened scheme, so this
/// fails in both directions rather than only one.
#[test]
fn test_accessor_indices_are_own_class_relative() {
    let indices = accessor_field_indices(
        r#"
صنف أصل {
    خاصية قيمة: عدد = 0
    خاصية ثانية: عدد = 0

    منشئ() {
        هذا.قيمة = 1
        هذا.ثانية = 2
    }
}

صنف فرع يرث أصل {
    خاصية إضافة: عدد = 0

    منشئ() {
        هذا.إضافة = 3
    }
}
اطبع(0)
"#,
    );

    for (name, expected) in [
        ("أصل::__احصل_قيمة", 0u32),
        ("أصل::__عيّن_قيمة", 0),
        ("أصل::__احصل_ثانية", 1),
        ("أصل::__عيّن_ثانية", 1),
        ("فرع::__احصل_إضافة", 0),
        ("فرع::__عيّن_إضافة", 0),
    ] {
        let actual = indices
            .iter()
            .find(|(function, _)| function == name)
            .map(|(_, index)| *index);
        assert_eq!(
            actual,
            Some(expected),
            "{} يجب أن يشير إلى فهرس صنفه {} / must address its own class's slot\nالمرصود/observed: {:?}",
            name,
            expected,
            indices
        );
    }
}
