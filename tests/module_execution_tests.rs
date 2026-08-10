//! Execution-based tests for the module system (issue #182).
//!
//! Every other module test in this repo — `tests/phase3_criteria_tests.rs`
//! included — stops at `parses_ok` / `analyzes_ok`, i.e. parse plus type-check.
//! Imported symbols live in the symbol table long before they have a body in
//! any backend, so those tests stayed green for the whole v1.0 release while
//! `استورد` was broken end to end: calling an imported function failed at run
//! time with `دالة غير معرّفة`, and an imported constant with
//! `معرّف غير معرّف`. Issue #187 records that gap as the systemic reason ~40
//! broken features hid behind 1,300+ passing tests.
//!
//! These tests therefore *execute* programs and assert on real stdout, driving
//! the installed binary rather than the library, so the CLI's own module search
//! path and file-resolution logic are covered too.
//!
//! Two axes are swept for every fixture:
//!
//! * **All three backends** — `run` (interpreter), `run --jit`, and `compile`
//!   followed by executing the produced binary. All three consume the same
//!   linked IR, so a regression in `link_program` must surface in all three.
//!   The lone exception is the stdlib fixture, whose native leg is a documented
//!   pre-existing gap asserted explicitly rather than skipped.
//! * **Both working directories** — once with the CWD inside the fixture, and
//!   once from the repo root with an absolute path to the main file. Only the
//!   second catches the original defect: CWD-relative resolution used to
//!   succeed by accident and mask it.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::{fs, str};

use tempfile::TempDir;

const TARQEEM: &str = env!("CARGO_BIN_EXE_tarqeem");

/// The static runtime library every native binary links against, named as
/// `codegen::linker::find_runtime` names it.
const RUNTIME_LIB: &str = if cfg!(windows) { "trq.lib" } else { "libtrq.a" };

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Guarantees `libtrq.a` exists before the first native case links against it.
///
/// `libtrq.a` is produced by the *separate* `tarqeem-runtime` crate under
/// `runtime-rs/`, which a plain `cargo test` of this package never builds: the
/// workspace root is itself a package, so the default member set is just
/// `tarqeem`. The native leg therefore only ever passed on machines where some
/// earlier `cargo build --workspace` had left the library behind — a fresh
/// clone, and every CI job, got `undefined reference to 'trq_print_int'`
/// instead.
///
/// So build it on demand, at most once, however many test threads arrive here.
/// CI builds it explicitly (see `.github/workflows/ci.yml`), which makes this a
/// no-op there; the fallback is what keeps `git clone && cargo test` honest.
///
/// This never degrades to skipping the native backend. The premise of this file
/// is that all three backends run for every fixture, so a runtime that cannot be
/// prepared has to be a loud failure rather than a silent hole in the matrix.
fn ensure_runtime_library() {
    static RUNTIME: OnceLock<Result<(), String>> = OnceLock::new();

    // The Result is stored rather than unwrapped inside the closure: `OnceLock`
    // does not poison, so a panicking initialiser would let every other test
    // thread retry the same doomed build.
    if let Err(message) = RUNTIME.get_or_init(build_runtime_library) {
        panic!("{message}");
    }
}

/// The two `find_runtime` priorities a `cargo test` run can count on:
/// `TARQEEM_RUNTIME_PATH` (exported by `build.rs`, and stale often enough to
/// need the existence check) and the workspace target directory, release first.
///
/// `TARQEEM_HOME` and `~/.tarqeem/lib` are deliberately *not* consulted. They
/// hold whatever runtime was installed last, which need not match this
/// checkout, and the fixtures scrub or repoint `TARQEEM_HOME` anyway.
fn runtime_library_present() -> bool {
    if std::env::var("TARQEEM_RUNTIME_PATH").is_ok_and(|path| Path::new(&path).exists()) {
        return true;
    }

    ["release", "debug"].iter().any(|profile| {
        project_root()
            .join("target")
            .join(profile)
            .join(RUNTIME_LIB)
            .exists()
    })
}

/// Builds `tarqeem-runtime` into the profile this test binary was built with,
/// so the artifact lands where `find_runtime` looks for it.
fn build_runtime_library() -> Result<(), String> {
    if runtime_library_present() {
        return Ok(());
    }

    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let expected = project_root()
        .join("target")
        .join(profile)
        .join(RUNTIME_LIB);

    eprintln!(
        "مكتبة وقت التشغيل مفقودة، جارٍ بناؤها / runtime library missing, building it: {}",
        expected.display()
    );

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
        // A build that succeeds without producing the artifact means it went
        // somewhere `find_runtime` will not look — `CARGO_TARGET_DIR` being the
        // usual cause. Linking would fail next, with a far less obvious message.
        Ok(_) if !expected.exists() => Err(runtime_failure(&format!(
            "نجح البناء لكن {} غير موجودة / build succeeded but the artifact is missing (CARGO_TARGET_DIR؟)",
            expected.display()
        ))),
        Ok(_) => Ok(()),
    }
}

/// Wraps a runtime-preparation failure with the command to run by hand.
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

/// Whether a fixture needs the standard library on the module search path.
#[derive(Clone, Copy)]
enum Stdlib {
    /// Local-file fixtures: no ambient `TARQEEM_HOME` at all.
    NotNeeded,
    /// Pins `TARQEEM_HOME` at this checkout. Without it the CLI falls back to
    /// `stdlib_trq` relative to the CWD (wrong for the temp-dir variant) and
    /// then to `~/.tarqeem/stdlib_trq` — a stale copy on developer machines,
    /// and absent in CI.
    PinnedToRepo,
}

/// The execution modes under test. Named per `tarqeem` subcommand rather than
/// per implementation, since that is what a user of the compiler picks.
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

    /// Everything the program printed, one entry per `اطبع` line.
    fn lines(&self) -> Vec<String> {
        self.stdout
            .trim()
            .lines()
            .map(|line| line.trim().to_string())
            .collect()
    }

    fn combined(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }

    /// Both streams, labelled — a bare `assert_eq!` on trimmed stdout says
    /// nothing when the binary errored out and printed to stderr instead.
    fn report(&self) -> String {
        format!(
            "الحالة/status: {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            self.status, self.stdout, self.stderr
        )
    }
}

/// Writes one `.ترقيم` module, adding the `بسم_الله` / `الحمد_لله` markers the
/// parser requires around every file.
fn write_module(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("تعذّر إنشاء مجلد الوحدة");
    }
    fs::write(&path, format!("بسم_الله\n{}\nالحمد_لله\n", body.trim())).expect("تعذّر كتابة الوحدة");
    path
}

fn tarqeem(args: &[&str], cwd: &Path, stdlib: Stdlib) -> Output {
    let mut command = Command::new(TARQEEM);
    command.args(args).current_dir(cwd);

    match stdlib {
        // A stale `TARQEEM_HOME` silently shadows this checkout's `stdlib_trq`.
        Stdlib::NotNeeded => command.env_remove("TARQEEM_HOME"),
        Stdlib::PinnedToRepo => command.env("TARQEEM_HOME", project_root()),
    };

    let output = command
        .output()
        .unwrap_or_else(|e| panic!("تعذّر تشغيل {} {:?}: {}", TARQEEM, args, e));

    Output {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn execute_binary(path: &Path) -> Output {
    let output = path
        .to_str()
        .map(Command::new)
        .expect("مسار غير صالح")
        .output()
        .unwrap_or_else(|e| panic!("تعذّر تشغيل الملف التنفيذي {}: {}", path.display(), e));

    Output {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Runs `main_arg` under one backend. For `Native`, a failed compile is
/// returned as-is so its diagnostics reach the assertion message.
fn execute(
    backend: Backend,
    main_arg: &str,
    cwd: &Path,
    out_dir: &Path,
    tag: &str,
    stdlib: Stdlib,
) -> Output {
    match backend {
        Backend::Interpreter => tarqeem(&["run", main_arg], cwd, stdlib),
        Backend::Jit => tarqeem(&["run", "--jit", main_arg], cwd, stdlib),
        Backend::Native => {
            // The only backend that links, so the only one needing `libtrq.a`.
            ensure_runtime_library();
            let exe = out_dir.join(format!("مخرج_{tag}"));
            let exe_arg = exe.to_str().expect("مسار غير صالح").to_string();
            let compiled = tarqeem(&["compile", main_arg, "-o", &exe_arg], cwd, stdlib);
            if !compiled.succeeded() {
                return compiled;
            }
            execute_binary(&exe)
        }
    }
}

/// The two (label, working directory, file argument) pairs every case is run
/// under. The absolute-path variant is the one that used to fail.
fn variants(main: &Path) -> Vec<(&'static str, PathBuf, String)> {
    let dir = main.parent().expect("للملف الرئيسي مجلد").to_path_buf();
    let file_name = main
        .file_name()
        .and_then(|n| n.to_str())
        .expect("اسم ملف صالح")
        .to_string();
    let absolute = main.to_str().expect("مسار غير صالح").to_string();

    vec![
        ("مسار نسبي من مجلد الوحدات", dir, file_name),
        ("مسار مطلق من جذر المستودع", project_root(), absolute),
    ]
}

/// Asserts the program prints exactly `expected` under each of `backends`, from
/// both working directories. Callers pass `&Backend::ALL` unless a documented
/// backend gap forces a narrower set.
fn assert_prints(
    main: &Path,
    out_dir: &Path,
    expected: &[&str],
    stdlib: Stdlib,
    backends: &[Backend],
) {
    for (variant, (label, cwd, arg)) in variants(main).into_iter().enumerate() {
        for &backend in backends {
            let tag = format!("{:?}_{}", backend, variant);
            let output = execute(backend, &arg, &cwd, out_dir, &tag, stdlib);

            assert!(
                output.succeeded(),
                "فشل التنفيذ [{:?} / {}]\nالمتوقع/expected: {:?}\n{}",
                backend,
                label,
                expected,
                output.report()
            );
            assert_eq!(
                output.lines(),
                expected,
                "خرج غير متطابق [{:?} / {}]\nالمتوقع/expected: {:?}\n{}",
                backend,
                label,
                expected,
                output.report()
            );
        }
    }
}

/// Asserts the program is rejected — non-zero exit, with `code` somewhere in
/// the output — under all three backends, from both working directories.
fn assert_rejected_with(main: &Path, out_dir: &Path, code: &str, stdlib: Stdlib) {
    for (variant, (label, cwd, arg)) in variants(main).into_iter().enumerate() {
        for backend in Backend::ALL {
            let tag = format!("{:?}_{}", backend, variant);
            let output = execute(backend, &arg, &cwd, out_dir, &tag, stdlib);

            assert!(
                !output.succeeded(),
                "توقّعنا فشلاً برمز {} [{:?} / {}]\n{}",
                code,
                backend,
                label,
                output.report()
            );
            assert!(
                output.combined().contains(code),
                "الرمز {} غير موجود في المخرجات [{:?} / {}]\n{}",
                code,
                backend,
                label,
                output.report()
            );
        }
    }
}

#[test]
fn test_imported_function_with_arguments_returns_value() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    write_module(
        dir,
        "مكتبة.ترقيم",
        "صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد {\n    أرجع أ + ب\n}",
    );
    let main = write_module(
        dir,
        "رئيسي.ترقيم",
        "استورد { جمع } من \"./مكتبة\"\nاطبع(جمع(2، 3))",
    );

    // Before the fix the signature never crossed the module boundary, so this
    // failed type-checking with `متوقع 0 معاملات، وُجد 2`.
    assert_prints(&main, dir, &["5"], Stdlib::NotNeeded, &Backend::ALL);
}

#[test]
fn test_imported_constant_is_visible_in_main() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    write_module(dir, "ثوابت.ترقيم", "صدّر ثابت الحد = 7");
    let main = write_module(
        dir,
        "رئيسي.ترقيم",
        "استورد { الحد } من \"./ثوابت\"\nاطبع(الحد)",
    );

    // Previously `معرّف غير معرّف`: the constant had no initializer in the IR.
    assert_prints(&main, dir, &["7"], Stdlib::NotNeeded, &Backend::ALL);
}

#[test]
fn test_transitive_import_chain_links_every_module() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    write_module(dir, "ج.ترقيم", "صدّر دالة ج_دالة() -> عدد {\n    أرجع 11\n}");
    write_module(
        dir,
        "ب.ترقيم",
        "استورد { ج_دالة } من \"./ج\"\nصدّر دالة ب_دالة() -> عدد {\n    أرجع ج_دالة() + 1\n}",
    );
    let main = write_module(
        dir,
        "أ.ترقيم",
        "استورد { ب_دالة } من \"./ب\"\nاطبع(ب_دالة())",
    );

    // `ج` is never imported by main, and its symbols are deliberately not
    // visible there — but its *body* must still be linked in, or `ب_دالة`
    // calls a function that does not exist.
    assert_prints(&main, dir, &["12"], Stdlib::NotNeeded, &Backend::ALL);
}

#[test]
fn test_diamond_imports_do_not_duplicate_shared_module() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    write_module(dir, "د.ترقيم", "صدّر دالة د_دالة() -> عدد {\n    أرجع 4\n}");
    write_module(
        dir,
        "ب.ترقيم",
        "استورد { د_دالة } من \"./د\"\nصدّر دالة ب_دالة() -> عدد {\n    أرجع د_دالة()\n}",
    );
    write_module(
        dir,
        "ج.ترقيم",
        "استورد { د_دالة } من \"./د\"\nصدّر دالة ج_دالة() -> عدد {\n    أرجع د_دالة()\n}",
    );
    let main = write_module(
        dir,
        "أ.ترقيم",
        "استورد { ب_دالة } من \"./ب\"\nاستورد { ج_دالة } من \"./ج\"\nاطبع(ب_دالة() + ج_دالة())",
    );

    // `د` is reached twice. The loader caches by canonical path, so it must be
    // merged once — merging it twice is a و٠١٠١ duplicate-definition error.
    assert_prints(&main, dir, &["8"], Stdlib::NotNeeded, &Backend::ALL);
}

#[test]
fn test_circular_imports_report_error_code() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    write_module(
        dir,
        "ب.ترقيم",
        "استورد { أ_دالة } من \"./أ\"\nصدّر دالة ب_دالة() -> عدد {\n    أرجع أ_دالة() + 1\n}",
    );
    let main = write_module(
        dir,
        "أ.ترقيم",
        "استورد { ب_دالة } من \"./ب\"\nصدّر دالة أ_دالة() -> عدد {\n    أرجع 2\n}\nاطبع(ب_دالة())",
    );

    // The cycle is found by a *nested* load, and every module on it still lands
    // in the cache — so the third analysis pass sees them all present and never
    // re-reports it. `preload_imported_modules` therefore has to forward و٠٣٠١
    // specifically; until it did, this program compiled and ran silently.
    assert_rejected_with(&main, dir, "و٠٣٠١", Stdlib::NotNeeded);
}

#[test]
fn test_duplicate_export_collision_names_both_modules() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    let first = write_module(
        dir,
        "م1.ترقيم",
        "صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد {\n    أرجع أ + ب\n}",
    );
    let second = write_module(
        dir,
        "م2.ترقيم",
        "صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد {\n    أرجع أ * ب\n}",
    );
    let main = write_module(
        dir,
        "رئيسي.ترقيم",
        "استورد { جمع } من \"./م1\"\nاستورد { جمع } من \"./م2\"\nاطبع(جمع(2، 3))",
    );

    assert_rejected_with(&main, dir, "و٠١٠١", Stdlib::NotNeeded);

    // The message must name both origins, or the user cannot tell which pair of
    // modules collided.
    let checked = tarqeem(
        &["check", main.to_str().unwrap()],
        &project_root(),
        Stdlib::NotNeeded,
    );
    assert!(
        !checked.succeeded(),
        "توقّعنا فشل الفحص\n{}",
        checked.report()
    );

    let message = checked.combined();
    for module in [&first, &second] {
        let canonical = module.canonicalize().expect("مسار قابل للتقييس");
        assert!(
            message.contains(canonical.to_str().unwrap()),
            "مسار الوحدة {} غير مذكور في رسالة التصادم\n{}",
            canonical.display(),
            checked.report()
        );
    }
}

#[test]
fn test_subdirectory_import_resolves_index_file() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    write_module(dir, "حزمة/فهرس.ترقيم", "صدّر ثابت س = 9");
    let main = write_module(dir, "رئيسي.ترقيم", "استورد { س } من \"./حزمة\"\nاطبع(س)");

    // `./حزمة` is a directory: resolution must fall through to `فهرس.ترقيم`.
    assert_prints(&main, dir, &["9"], Stdlib::NotNeeded, &Backend::ALL);
}

#[test]
fn test_imported_class_constructs_and_reads_field() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    write_module(
        dir,
        "شكل.ترقيم",
        "صدّر صنف نقطة {\n    عام س: عدد\n    منشئ(س: عدد) {\n        هذا.س = س\n    }\n}",
    );
    let main = write_module(
        dir,
        "رئيسي.ترقيم",
        "استورد { نقطة } من \"./شكل\"\nمتغير ن = جديد نقطة(7)\nاطبع(ن.س)",
    );

    // A class needs more than a symbol-table entry: the constructor body and
    // the field layout have to be merged before IR generation.
    assert_prints(&main, dir, &["7"], Stdlib::NotNeeded, &Backend::ALL);
}

#[test]
fn test_wildcard_alias_import_of_local_module_calls_namespaced_function() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    write_module(
        dir,
        "أدوات.ترقيم",
        "صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد {\n    أرجع أ + ب\n}",
    );
    let main = write_module(
        dir,
        "رئيسي.ترقيم",
        "استورد * كـ أدوات من \"./أدوات\"\nاطبع(أدوات.جمع(2، 3))",
    );

    assert_prints(&main, dir, &["5"], Stdlib::NotNeeded, &Backend::ALL);
}

#[test]
fn test_wildcard_alias_import_of_stdlib_calls_namespaced_function() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    // `مطلق` returns `عدد`, not `عدد_عشري`: floats still print as `4.0` under
    // the interpreter/JIT and `4` natively (issue #185), which would force a
    // weaker assertion here for no benefit to the module system.
    let main = write_module(
        dir,
        "رئيسي.ترقيم",
        "استورد * كـ رياض من \"رياضيات\"\nاطبع(رياض.مطلق(0 - 7))",
    );

    // Stdlib names short-circuit to a builtin table instead of being read from
    // disk, so this exercises a different path than the local-file wildcard.
    assert_prints(
        &main,
        dir,
        &["7"],
        Stdlib::PinnedToRepo,
        &[Backend::Interpreter, Backend::Jit],
    );

    // Native is excluded above because stdlib calls segfault once compiled: the
    // interpreter and JIT register the builtins at run time, but nothing links
    // a body for them into the object file, so LLVM emits a call to a symbol
    // that never gets defined. This is issue #185 item 3 ("importing any stdlib
    // function → native binary segfaults, exit 139, while interpreter/JIT print
    // correct results"), filed before this branch and unrelated to the #182
    // module merge: a plain `استورد { مطلق } من "رياضيات"` crashes identically,
    // while every local-file fixture in this file passes natively.
    //
    // Asserting the current failure rather than skipping it keeps the gap
    // visible: whoever fixes native stdlib linkage will see this assertion trip
    // and should then fold `Backend::Native` into the call above and delete
    // this block.
    for (variant, (label, cwd, arg)) in variants(&main).into_iter().enumerate() {
        let tag = format!("Native_gap_{}", variant);
        let output = execute(Backend::Native, &arg, &cwd, dir, &tag, Stdlib::PinnedToRepo);
        assert!(
            !output.succeeded(),
            "المكتبة القياسية تعمل الآن في الترجمة الأصلية [{}] — \
             أضف Backend::Native أعلاه واحذف هذه الكتلة\n{}",
            label,
            output.report()
        );
    }
}

#[test]
fn test_aliased_named_import_calls_function_under_alias() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    write_module(
        dir,
        "أدوات.ترقيم",
        "صدّر دالة ضاعف(س: عدد) -> عدد {\n    أرجع س * 2\n}",
    );
    let main = write_module(
        dir,
        "رئيسي.ترقيم",
        "استورد { ضاعف كـ اضعف } من \"./أدوات\"\nاطبع(اضعف(5))",
    );

    // The alias must bind to the imported body, not merely to a fresh symbol.
    assert_prints(&main, dir, &["10"], Stdlib::NotNeeded, &Backend::ALL);
}

#[test]
fn test_program_without_imports_still_executes() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    let main = write_module(
        dir,
        "رئيسي.ترقيم",
        "دالة جمع(أ: عدد، ب: عدد) -> عدد {\n    أرجع أ + ب\n}\nاطبع(جمع(2، 3))",
    );

    // Regression guard for `link_program`'s empty-cache fast path: a program
    // with nothing to merge must be left exactly as it was.
    assert_prints(&main, dir, &["5"], Stdlib::NotNeeded, &Backend::ALL);
}
