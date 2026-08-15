//! Execution-based tests for **inherited instance members** — issues #249 and
//! #253.
//!
//! `FieldId` and `MethodId` mean *(defining class, index own-relative to that
//! class)*: `class_fields` holds a class's own fields only, and codegen adds
//! `inherited_field_count[defining_class]` to reach the flattened slot. The
//! accessor synthesis in `stmt_builder.rs` honours that. The member-access paths
//! in `expr_builder.rs` did not — they filled both ids with the *receiver's*
//! class, and when the member turned out to be inherited they silently
//! substituted `index: 0` and type `ptr`.
//!
//! Three wrong values in one instruction, with two distinct symptoms:
//!
//! * a subclass declaring no fields of its own GEP'd past the end of its struct
//!   (`inherited_count 1 + index 0` against a one-field type) — `clang` rejected
//!   the module with `invalid getelementptr indices`;
//! * a subclass with its own fields aliased every write onto one slot, *and* the
//!   lost type meant `اطبع` emitted `trq_print(ptr %x)` against an integer —
//!   SIGSEGV.
//!
//! Why nothing caught it: `oop_execution_tests.rs` covers inheritance but is
//! in-process with **no native leg**, and native codegen is the only consumer of
//! `field.index` (the interpreter keys on `field.name` and discards the index;
//! the JIT delegates to the interpreter for programs this short). CI's
//! `compare-backends` job would have caught it, but no example program reads an
//! inherited member through a subclass-typed reference.
//!
//! Issue #253 is the *method* half of the same confusion. A method declared on
//! an ancestor and called through a subclass-typed reference lowered
//! `MethodId.class` to the receiver rather than to the declaring class, so
//! native minted `@{subclass}::{method}` — a symbol nothing ever defines,
//! because the body was compiled as `@{ancestor}::{method}` — and clang rejected
//! the module. It carries a quieter symptom too: the callee's return type is
//! looked up as `method_return_types["{receiver}::{method}"]`, misses for the
//! same reason, and falls back to `Ptr(Void)`. So a fixture that merely links
//! proves only half of it, and one of the fixtures below prints a returned
//! `عدد` for that reason.
//!
//! So every fixture here runs under **all three backends** against exact stdout,
//! and reaches inherited members through the *subclass* — the shape that
//! `examples/أصناف.ترقيم` avoids by accident.
//!
//! One deliberate exclusion, so a failure here never means something else:
//!
//! * Fixtures assign every member they read. A declared default is parsed and
//!   dropped (issue #251), so `خاصية س: عدد = 7` still reads as `لا_شيء`
//!   interpreted and `0` natively.

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
/// `cargo test` of this package never builds. Never degrades to skipping the
/// native backend: that backend is the entire point of this file, so a runtime
/// that cannot be prepared has to fail loudly.
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

/// Deliberately this test binary's own profile only: a stale archive from the
/// other profile would link the native leg against a runtime that does not
/// correspond to this checkout.
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
    /// the program errored out and printed to stderr instead, and says nothing
    /// at all when the binary died on a signal and printed neither.
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
/// as-is so its diagnostics reach the assertion message — that is how the
/// `invalid getelementptr indices` half of #249 reports itself.
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
///   interpreter is not the reference — the literal is. That matters here more
///   than usual: the interpreter was *also* wrong for an inherited
///   auto-property, just wrong in a way that happened to agree with itself.
/// * raw stdout, byte for byte, against the first backend's, which `lines()`
///   cannot see past because it trims the stream and every line.
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

/// The #249 SIGSEGV repro, reduced: one inherited plain field, read through a
/// subclass-typed reference.
///
/// Before the fix the write resolved to `(class: فرع, index: 0)` and codegen
/// added `inherited_field_count[فرع]`, landing on slot 1 of a two-slot struct —
/// while the read came back typed `ptr`, so `اطبع` called `trq_print` on the
/// integer `6` and dereferenced address 6.
#[test]
fn test_inherited_plain_field_read_through_subclass() {
    assert_prints(
        r#"
صنف أصل {
    عام قيمة: عدد

    منشئ(ق: عدد) {
        هذا.قيمة = ق
    }
}

صنف فرع يرث أصل {
    عام إضافة: عدد

    منشئ(ق: عدد، إ: عدد) {
        هذا.قيمة = ق
        هذا.إضافة = إ
    }
}

متغير كائن = جديد فرع(5، 6)
اطبع(كائن.قيمة)
اطبع(كائن.إضافة)
"#,
        &["5", "6"],
    );
}

/// An inherited field and an own field must not alias, in either direction.
///
/// Distinct values are the load-bearing detail — the technique `docs/AI_NOTES.md`
/// records for #239. Both writes previously computed the same flattened slot, so
/// the second overwrote the first and the untouched slot read back as zeroed
/// memory: matching values would have hidden it, and an off-by-one predicts
/// garbage in the last slot instead of a clean swap.
#[test]
fn test_inherited_and_own_fields_keep_distinct_slots() {
    assert_prints(
        r#"
صنف قاعدة {
    عام أول: عدد

    منشئ() {
        هذا.أول = 11
    }
}

صنف مشتق يرث قاعدة {
    عام ثان: عدد
    عام ثالث: عدد

    منشئ() {
        الأصل()
        هذا.ثان = 22
        هذا.ثالث = 33
    }
}

متغير كائن = جديد مشتق()
اطبع(كائن.أول)
اطبع(كائن.ثان)
اطبع(كائن.ثالث)
"#,
        &["11", "22", "33"],
    );
}

/// The other direction of the same access: `هذا.<inherited>` from inside the
/// subclass's own code, where `هذا` is statically the subclass.
#[test]
fn test_inherited_field_read_from_subclass_method() {
    assert_prints(
        r#"
صنف أصل {
    عام قيمة: عدد

    منشئ(ق: عدد) {
        هذا.قيمة = ق
    }
}

صنف فرع يرث أصل {
    منشئ(ق: عدد) {
        الأصل(ق)
    }

    عام دالة اعرض() {
        اطبع(هذا.قيمة)
    }
}

متغير كائن = جديد فرع(7)
كائن.اعرض()
"#,
        &["7"],
    );
}

/// The #249 `invalid getelementptr indices` repro: a subclass that declares no
/// fields of its own, so the bogus `inherited_count + 0` ran past the end of a
/// struct with exactly one slot and clang rejected the whole module.
///
/// Also the shape that needs the *accessor* walk rather than the field walk: the
/// property is declared on the parent, so `{فرع}::__عيّن_قيمة` does not exist and
/// `MethodId.class` has to name `أصل`.
#[test]
fn test_inherited_auto_property_through_fieldless_subclass() {
    assert_prints(
        r#"
صنف أصل {
    خاصية قيمة: عدد = 0

    منشئ(ق: عدد) {
        هذا.قيمة = ق
    }
}

صنف فرع يرث أصل {
    منشئ(ق: عدد) {
        هذا.قيمة = ق
    }
}

متغير كائن = جديد فرع(5)
اطبع(كائن.قيمة)
"#,
        &["5"],
    );
}

/// Issue #250's exact repro, which turned out to share #249's root cause.
///
/// The write was never lost: inside `أصل::منشئ`, `هذا` is typed `أصل`, so it
/// routed through the setter and stored the backing field `_قيمة` correctly. The
/// *read* through a `فرع`-typed reference missed the accessor lookup, degraded to
/// a raw `GetField` named `قيمة` — a name no slot carries — and the interpreter
/// returned `Null`, printing `لا_شيء`.
#[test]
fn test_super_constructor_write_to_inherited_auto_property() {
    assert_prints(
        r#"
صنف أصل {
    خاصية قيمة: عدد = 0

    منشئ(ق: عدد) {
        هذا.قيمة = ق
    }
}

صنف فرع يرث أصل {
    منشئ(ق: عدد) {
        الأصل(ق)
    }
}

متغير كائن = جديد فرع(5)
اطبع(كائن.قيمة)
"#,
        &["5"],
    );
}

/// The same loss with `الأصل` removed from the write path entirely — the
/// discriminator proving #250 was the read-side accessor lookup and not
/// super-constructor lowering.
///
/// `اضبط` is declared on the subclass, so nothing here calls an inherited
/// method; the only inherited thing is the auto-property it writes.
#[test]
fn test_inherited_auto_property_written_outside_a_constructor() {
    assert_prints(
        r#"
صنف أصل {
    خاصية قيمة: عدد = 0

    منشئ() {
        هذا.قيمة = 1
    }
}

صنف فرع يرث أصل {
    منشئ() {
        الأصل()
    }

    عام دالة اضبط(ق: عدد) {
        هذا.قيمة = ق
    }
}

متغير كائن = جديد فرع()
اطبع(كائن.قيمة)
كائن.اضبط(5)
اطبع(كائن.قيمة)
"#,
        &["1", "5"],
    );
}

/// Two hops, so the walk is exercised past a single parent — and so a resolver
/// that stopped at the immediate parent (or that returned the *receiver* once it
/// found the member anywhere) would fail here while passing everything above.
#[test]
fn test_field_inherited_across_two_levels() {
    assert_prints(
        r#"
صنف مستوى_أول {
    عام أ: عدد

    منشئ() {
        هذا.أ = 1
    }
}

صنف مستوى_ثان يرث مستوى_أول {
    عام ب: عدد

    منشئ() {
        الأصل()
        هذا.ب = 2
    }
}

صنف مستوى_ثالث يرث مستوى_ثان {
    عام ج: عدد

    منشئ() {
        الأصل()
        هذا.ج = 3
    }
}

متغير كائن = جديد مستوى_ثالث()
اطبع(كائن.أ)
اطبع(كائن.ب)
اطبع(كائن.ج)
"#,
        &["1", "2", "3"],
    );
}

/// `+=` on an inherited member, both a plain field and an auto-property, from
/// inside the subclass and from outside it.
///
/// Compound assignment reads through `build_member` and writes through
/// `store_to_member`, so it inherits both halves of the fix — but it reaches them
/// by its own route, and it has its own history: `deb90d9` exists because `+=`
/// once bypassed the property setter entirely. `property_execution_tests.rs`
/// covers `+=` on a *same-class* property only.
#[test]
fn test_compound_assignment_to_inherited_members() {
    assert_prints(
        r#"
صنف أصل {
    عام عدد_موروث: عدد
    خاصية قيمة: عدد = 0

    منشئ() {
        هذا.عدد_موروث = 10
        هذا.قيمة = 20
    }
}

صنف فرع يرث أصل {
    عام خاص_به: عدد

    منشئ() {
        الأصل()
        هذا.خاص_به = 30
    }

    عام دالة زد() {
        هذا.عدد_موروث += 1
        هذا.قيمة += 1
    }
}

متغير كائن = جديد فرع()
كائن.زد()
كائن.عدد_موروث += 100
كائن.قيمة += 100
اطبع(كائن.عدد_موروث)
اطبع(كائن.قيمة)
اطبع(كائن.خاص_به)
"#,
        &["111", "121", "30"],
    );
}

/// Issue #253's exact repro: a method declared on the parent, reached through a
/// subclass-typed reference, where the subclass declares nothing but a
/// constructor.
///
/// `MethodId.class` named the receiver, so native emitted a call to
/// `@فرع::اعرض` while the body had been compiled as `@أصل::اعرض`, and clang
/// rejected the module with `use of undefined value`. The interpreter and the
/// JIT resolve from the object's runtime class, so neither ever saw it — which
/// is why this needs the native leg to mean anything.
#[test]
fn test_inherited_method_called_through_subclass() {
    assert_prints(
        r#"
صنف أصل {
    عام قيمة: عدد

    منشئ(ق: عدد) {
        هذا.قيمة = ق
    }

    عام دالة اعرض() {
        اطبع(هذا.قيمة)
    }
}

صنف فرع يرث أصل {
    منشئ(ق: عدد) {
        الأصل(ق)
    }
}

متغير كائن = جديد فرع(7)
كائن.اعرض()
"#,
        &["7"],
    );
}

/// The half a link-only fixture would miss: the *return type* of the inherited
/// callee.
///
/// It is looked up as `method_return_types["{receiver}::{method}"]`, misses for
/// the same reason the symbol does, and falls back to `Ptr(Void)` — visible in
/// `--dump-ir` as a `*void` result for a method whose declared return type is
/// `عدد`. Printing the returned value is what forces the type to matter: a fix
/// that repaired only the callee symbol would link cleanly and still be wrong
/// here.
#[test]
fn test_inherited_method_returning_number_through_subclass() {
    assert_prints(
        r#"
صنف أصل {
    عام قيمة: عدد

    منشئ(ق: عدد) {
        هذا.قيمة = ق
    }

    عام دالة مضاعف() -> عدد {
        أرجع هذا.قيمة * 2
    }
}

صنف فرع يرث أصل {
    منشئ(ق: عدد) {
        الأصل(ق)
    }
}

متغير كائن = جديد فرع(21)
اطبع(كائن.مضاعف())
"#,
        &["42"],
    );
}

/// The over-correction guard: an override must still win.
///
/// Resolving `MethodId.class` to the *declaring* class has to stop at the first
/// class in the chain that declares the method, not run to the topmost one —
/// otherwise `كائن.احسب()` on a `فرع` would execute `أصل::احسب`. The two bodies
/// return deliberately far-apart values so a wrong binding cannot read as an
/// arithmetic slip.
///
/// Green before the fix as well as after: naming the receiver happens to be
/// right exactly when the receiver declares the method. That is the point — it
/// is the constraint the fix must not trade away.
#[test]
fn test_subclass_override_wins_over_inherited_method() {
    assert_prints(
        r#"
صنف أصل {
    عام قيمة: عدد

    منشئ(ق: عدد) {
        هذا.قيمة = ق
    }

    عام دالة احسب() -> عدد {
        أرجع هذا.قيمة + 1
    }
}

صنف فرع يرث أصل {
    منشئ(ق: عدد) {
        الأصل(ق)
    }

    عام دالة احسب() -> عدد {
        أرجع هذا.قيمة + 100
    }
}

متغير كائن = جديد فرع(5)
اطبع(كائن.احسب())
"#,
        &["105"],
    );
}

/// Two hops, so a resolver that consults only the immediate parent fails here
/// while passing every single-level fixture above.
///
/// `اعرض` is declared on `مستوى_أول`, the receiver is `مستوى_ثالث`, and the
/// class in between declares nothing at all — the method half of what
/// `test_field_inherited_across_two_levels` does for fields.
#[test]
fn test_method_inherited_across_two_levels() {
    assert_prints(
        r#"
صنف مستوى_أول {
    عام أ: عدد

    منشئ(ق: عدد) {
        هذا.أ = ق
    }

    عام دالة اعرض() {
        اطبع(هذا.أ)
    }
}

صنف مستوى_ثان يرث مستوى_أول {
    منشئ(ق: عدد) {
        الأصل(ق)
    }
}

صنف مستوى_ثالث يرث مستوى_ثان {
    منشئ(ق: عدد) {
        الأصل(ق)
    }
}

متغير كائن = جديد مستوى_ثالث(9)
كائن.اعرض()
"#,
        &["9"],
    );
}

/// `الأصل.<method>()` where the method is declared on the *grandparent* — and
/// the one shape here that fails on **all three** backends, where the rest of
/// #253 is native-only.
///
/// `infer_expr_type`'s `ExprKind::Super` arm yields the immediate parent, so the
/// call is minted as `مستوى_ثان::اعرض`, a method that class does not declare
/// either. A super call dispatches non-virtually from that static id, so the
/// interpreter and the JIT cannot recover it the way they recover a member call:
/// both abort with `دالة غير معرّفة: مستوى_ثان::اعرض`, and native fails to link
/// the very same symbol.
///
/// Whether #253's fix reaches this depends on where it lands. A member call
/// mints the *receiver's* class; a super call mints the *immediate parent's*. A
/// fix at the shared resolution layer — walk the chain to the class that
/// declares the method — takes both; one confined to the member-access arm
/// leaves this red, and it belongs in an issue of its own.
#[test]
fn test_super_call_to_method_declared_on_grandparent() {
    assert_prints(
        r#"
صنف مستوى_أول {
    عام أ: عدد

    منشئ(ق: عدد) {
        هذا.أ = ق
    }

    عام دالة اعرض() {
        اطبع(هذا.أ)
    }
}

صنف مستوى_ثان يرث مستوى_أول {
    منشئ(ق: عدد) {
        الأصل(ق)
    }
}

صنف مستوى_ثالث يرث مستوى_ثان {
    منشئ(ق: عدد) {
        الأصل(ق)
    }

    عام دالة نفّذ() {
        الأصل.اعرض()
    }
}

متغير كائن = جديد مستوى_ثالث(4)
كائن.نفّذ()
"#,
        &["4"],
    );
}

/// Guards the strictness boundary of the fix rather than the fix itself.
///
/// Resolution failure on a class the builder has a layout for is now a hard
/// `IrError` instead of a silent `(index 0, type ptr)`. Object literals are typed
/// `Struct(__anonymous__)`, a class `collect_class` never registers because
/// codegen resolves its fields by name — so the receiver *looks* known while the
/// lookup necessarily fails. Gating on "has a field layout" rather than "has a
/// class" is what keeps this compiling.
///
/// Only the first member is read: a second one diverges natively for unrelated
/// reasons (see #185 on object/dictionary literals), which would make this fail
/// for the wrong reason.
#[test]
fn test_object_literal_member_read_stays_lenient() {
    assert_prints(
        r#"
متغير سجل = { اسم: "أحمد"، عمر: 30 }
اطبع(سجل.اسم)
"#,
        &["أحمد"],
    );
}
