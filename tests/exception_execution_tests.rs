//! Execution-based tests for the exception system (issue #181).
//!
//! Before this, `ارمِ` could not be used in *any* program: the throwability
//! check accepts `استثناء` or a subclass, no `استثناء` was ever registered, and
//! the one name the standard library declared it under — `خطأ` — is the
//! boolean-false keyword, which the parser rejects as a class name. So the whole
//! of LANGUAGE_SPEC §11 was dead code while the existing tests stayed green:
//! every one of them either hand-declared its own `استثناء` (masking the missing
//! class) or stopped at `parses_ok` (`tests/phase3_criteria_tests.rs:885`,
//! `tests/runtime_rs_e2e_tests.rs:417`), so not one executed a *successful*
//! catch. That is the same systemic gap issue #187 records.
//!
//! These tests therefore execute programs and assert on real stdout, driving the
//! installed binary rather than the library — which also covers the prelude
//! reaching the CLI's own pipeline, not just `Analyzer::analyze` in isolation.
//!
//! Backend coverage is deliberately asymmetric, because the feature is:
//!
//! * **Interpreter and JIT** run exceptions fully, including propagation across
//!   call frames.
//! * **Native** refuses any program containing `ارمِ`, with ت٠٣٠٣. Codegen has no
//!   unwinding strategy at all — `TryBegin` lowers to an LLVM comment, the catch
//!   block is emitted with no predecessor, and `@trq_throw` is declared but
//!   defined nowhere in `runtime-rs`. `assert_rejected_natively` pins that
//!   refusal so it stays a diagnostic rather than reverting to the
//!   undefined-symbol link error it used to be.
//!
//! `test_try_catch_finally_without_throw_runs_natively` is the guard on the
//! other side of that line: `حاول`/`التقط`/`أخيراً` compiles and runs natively
//! today, so the block must key on `ارمِ` alone.
//!
//! This file mirrors `tests/module_execution_tests.rs`'s helper pattern verbatim
//! (this repo has no shared test-support module).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::{fs, str};

use tempfile::TempDir;

const TARQEEM: &str = env!("CARGO_BIN_EXE_tarqeem");

/// The static runtime library every native binary links against, named as
/// `codegen::linker::find_runtime` names it.
const RUNTIME_LIB: &str = if cfg!(windows) { "trq.lib" } else { "libtrq.a" };

/// The code native codegen reports for `ارمِ` (ت٠٣٠٣).
const NATIVE_EXCEPTIONS_CODE: &str = "ت٠٣٠٣";

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Guarantees `libtrq.a` exists before the first native case links against it.
/// See the fuller rationale on the twin in `tests/module_execution_tests.rs`:
/// `cargo test` of this package never builds the separate `tarqeem-runtime`
/// crate, so without this the native leg only passed where an earlier
/// `cargo build --workspace` had left the artifact behind.
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

    ["release", "debug"].iter().any(|profile| {
        project_root()
            .join("target")
            .join(profile)
            .join(RUNTIME_LIB)
            .exists()
    })
}

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

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut command = Command::new(&cargo);
    command
        .args(["build", "-p", "tarqeem-runtime"])
        .current_dir(project_root());
    if !cfg!(debug_assertions) {
        command.arg("--release");
    }

    match command.output() {
        Err(error) => Err(format!("تعذّر تشغيل «{cargo}»: {error}")),
        Ok(output) if !output.status.success() => Err(format!(
            "فشل بناء tarqeem-runtime (الحالة/status {:?})\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
        )),
        Ok(_) if !expected.exists() => Err(format!(
            "نجح البناء لكن {} غير موجودة (CARGO_TARGET_DIR؟)",
            expected.display()
        )),
        Ok(_) => Ok(()),
    }
}

/// The execution modes under test, named per `tarqeem` subcommand.
#[derive(Clone, Copy, Debug)]
enum Backend {
    Interpreter,
    Jit,
    Native,
}

impl Backend {
    const ALL: [Backend; 3] = [Backend::Interpreter, Backend::Jit, Backend::Native];
    /// The two backends that execute exceptions. Native is covered separately by
    /// `assert_rejected_natively`.
    const EXECUTING: [Backend; 2] = [Backend::Interpreter, Backend::Jit];
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

    fn combined(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }

    fn report(&self) -> String {
        format!(
            "الحالة/status: {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            self.status, self.stdout, self.stderr
        )
    }
}

/// One self-contained program in its own temp dir, with the `بسم_الله` /
/// `الحمد_لله` markers the parser requires.
struct Fixture {
    _dir: TempDir,
    path: PathBuf,
    out: PathBuf,
}

fn fixture(body: &str) -> Fixture {
    let dir = TempDir::new().expect("تعذّر إنشاء مجلد مؤقت");
    let path = dir.path().join("برنامج.ترقيم");
    fs::write(&path, format!("بسم_الله\n{}\nالحمد_لله\n", body.trim()))
        .expect("تعذّر كتابة البرنامج");
    let out = dir.path().to_path_buf();
    Fixture {
        _dir: dir,
        path,
        out,
    }
}

fn tarqeem(args: &[&str], cwd: &Path) -> Output {
    let mut command = Command::new(TARQEEM);
    command.args(args).current_dir(cwd);
    // None of these fixtures import anything; a stale `TARQEEM_HOME` would only
    // add an unrelated stdlib to the search path.
    command.env_remove("TARQEEM_HOME");

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

/// Runs the fixture under one backend. A failed native compile is returned
/// as-is so its diagnostics reach the assertion message.
fn execute(fixture: &Fixture, backend: Backend) -> Output {
    let arg = fixture.path.to_str().expect("مسار غير صالح");
    let cwd = fixture.path.parent().expect("للملف مجلد");

    match backend {
        Backend::Interpreter => tarqeem(&["run", arg], cwd),
        Backend::Jit => tarqeem(&["run", "--jit", arg], cwd),
        Backend::Native => {
            ensure_runtime_library();
            let exe = fixture.out.join("مخرج");
            let exe_arg = exe.to_str().expect("مسار غير صالح").to_string();
            let compiled = tarqeem(&["compile", arg, "-o", &exe_arg], cwd);
            if !compiled.succeeded() {
                return compiled;
            }
            execute_binary(&exe)
        }
    }
}

/// Asserts the program prints exactly `expected` under each of `backends`.
fn assert_prints(body: &str, expected: &[&str], backends: &[Backend]) {
    let fixture = fixture(body);

    for &backend in backends {
        let output = execute(&fixture, backend);

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
            "خرج غير متطابق [{:?}]\n{}",
            backend,
            output.report()
        );
    }
}

/// Asserts the program is rejected — non-zero exit, with `code` in the output —
/// under each of `backends`.
///
/// `backends` is a parameter rather than always `Backend::ALL` (as the module
/// tests' equivalent is) because a throwing program is *supposed* to succeed
/// under the interpreter and the JIT while native refuses it.
fn assert_rejected_with(body: &str, code: &str, backends: &[Backend]) {
    let fixture = fixture(body);

    for &backend in backends {
        let output = execute(&fixture, backend);

        assert!(
            !output.succeeded(),
            "توقعنا رفض البرنامج بالرمز {} [{:?}]\n{}",
            code,
            backend,
            output.report()
        );
        assert!(
            output.combined().contains(code),
            "الرمز {} غير موجود في التشخيص [{:?}]\n{}",
            code,
            backend,
            output.report()
        );
    }
}

/// The native half of an exception fixture: refused with ت٠٣٠٣, never linked.
fn assert_rejected_natively(body: &str) {
    assert_rejected_with(body, NATIVE_EXCEPTIONS_CODE, &[Backend::Native]);
}

// ==========================================================================
// The base class exists without being declared or imported
// ==========================================================================

/// Issue #181's exact reproduction.
#[test]
fn test_throw_and_catch_base_exception_reads_message() {
    let program = r#"
حاول {
    ارمِ جديد استثناء("خطأ تجريبي")
} التقط (خ) {
    اطبع("التقطت: " + خ.رسالة)
}
"#;

    assert_prints(program, &["التقطت: خطأ تجريبي"], &Backend::EXECUTING);
    assert_rejected_natively(program);
}

/// The catch parameter used to fail on *any* use, not just field access: the IR
/// builder bound it without marking it a direct value, so reading the name
/// emitted a `Load` of a non-pointer (`متوقع ptr، وُجد object`).
#[test]
fn test_catch_parameter_is_usable_without_field_access() {
    let program = r#"
دالة صف(خ: استثناء) -> نص {
    أرجع خ.رسالة
}
حاول {
    ارمِ جديد استثناء("مباشر")
} التقط (خ) {
    اطبع(صف(خ))
}
"#;

    assert_prints(program, &["مباشر"], &Backend::EXECUTING);
    assert_rejected_natively(program);
}

#[test]
fn test_user_subclass_of_exception_is_throwable_and_catchable() {
    let program = r#"
صنف استثناء_قسمة يرث استثناء {
    منشئ(رسالة: نص) {
        الأصل(رسالة)
    }
}
حاول {
    ارمِ جديد استثناء_قسمة("لا يمكن القسمة على صفر")
} التقط (خ) {
    اطبع("فرعي: " + خ.رسالة)
}
"#;

    assert_prints(
        program,
        &["فرعي: لا يمكن القسمة على صفر"],
        &Backend::EXECUTING,
    );
    assert_rejected_natively(program);
}

// ==========================================================================
// Propagation across call frames
// ==========================================================================

/// `try_stack` lives on the `CallFrame`, so a callee that found no handler
/// returned a Rust `Err` that `?` carried past every enclosing `حاول`. One call
/// level was enough to defeat `التقط`.
#[test]
fn test_exception_from_callee_is_caught_by_caller() {
    let program = r#"
دالة يرمي() {
    ارمِ جديد استثناء("من الدالة")
}
حاول {
    يرمي()
} التقط (خ) {
    اطبع("التقطت: " + خ.رسالة)
}
"#;

    assert_prints(program, &["التقطت: من الدالة"], &Backend::EXECUTING);
    assert_rejected_natively(program);
}

/// Two frames deep, to prove the unwind is a loop rather than a single hop.
#[test]
fn test_exception_propagates_through_two_call_frames() {
    let program = r#"
دالة عميق() {
    ارمِ جديد استثناء("من الأعماق")
}
دالة وسيط() {
    عميق()
}
دالة رئيسية() {
    حاول {
        وسيط()
    } التقط (خ) {
        اطبع("التقطت: " + خ.رسالة)
    }
    اطبع("تم")
}
"#;

    assert_prints(program, &["التقطت: من الأعماق", "تم"], &Backend::EXECUTING);
    assert_rejected_natively(program);
}

#[test]
fn test_exception_from_method_is_caught_by_caller() {
    // The explicit `منشئ` is not incidental: `جديد` on a class with no
    // constructor at all fails with `دالة غير معرّفة: حاسبة::منشئ` (issue #211),
    // which has nothing to do with exceptions and would make this test fail for
    // the wrong reason.
    let program = r#"
صنف حاسبة {
    خاص الاسم: نص

    منشئ() {
        هذا.الاسم = "حاسبة"
    }

    عام دالة اقسم(أ: عدد، ب: عدد) -> عدد {
        إذا (ب == 0) {
            ارمِ جديد استثناء("قسمة على صفر")
        }
        أرجع أ / ب
    }
}
دالة رئيسية() {
    متغير ح = جديد حاسبة()
    حاول {
        اطبع(ح.اقسم(10، 0))
    } التقط (خ) {
        اطبع("التقطت: " + خ.رسالة)
    }
}
"#;

    assert_prints(program, &["التقطت: قسمة على صفر"], &Backend::EXECUTING);
    assert_rejected_natively(program);
}

/// Rethrowing from a handler must reach the outer one, not the same one again.
#[test]
fn test_rethrow_from_catch_reaches_outer_handler() {
    let program = r#"
دالة داخلي() {
    حاول {
        ارمِ جديد استثناء("الأصلي")
    } التقط (خ) {
        اطبع("الداخلي رأى: " + خ.رسالة)
        ارمِ خ
    }
}
دالة رئيسية() {
    حاول {
        داخلي()
    } التقط (خ) {
        اطبع("الخارجي رأى: " + خ.رسالة)
    }
}
"#;

    assert_prints(
        program,
        &["الداخلي رأى: الأصلي", "الخارجي رأى: الأصلي"],
        &Backend::EXECUTING,
    );
    assert_rejected_natively(program);
}

// ==========================================================================
// Control flow around the handler
// ==========================================================================

#[test]
fn test_statements_after_throw_in_try_body_do_not_run() {
    let program = r#"
حاول {
    اطبع("قبل")
    ارمِ جديد استثناء("انفجار")
    اطبع("لن يُطبع")
} التقط (خ) {
    اطبع("التقطت: " + خ.رسالة)
} أخيراً {
    اطبع("أخيراً")
}
اطبع("بعد")
"#;

    assert_prints(
        program,
        &["قبل", "التقطت: انفجار", "أخيراً", "بعد"],
        &Backend::EXECUTING,
    );
    assert_rejected_natively(program);
}

/// A `حاول` with no `التقط` used to register a handler whose whole body was a
/// jump past the exception, so the throw was silently discarded and the program
/// carried on as if nothing had happened.
#[test]
fn test_try_without_catch_does_not_swallow_the_exception() {
    let program = r#"
حاول {
    ارمِ جديد استثناء("مُهمَل")
} أخيراً {
    اطبع("أخيراً")
}
اطبع("لا ينبغي الوصول هنا")
"#;

    let fixture = fixture(program);

    for backend in Backend::EXECUTING {
        let output = execute(&fixture, backend);

        assert!(
            !output.succeeded(),
            "استثناء بلا معالج يجب أن ينهي البرنامج [{:?}]\n{}",
            backend,
            output.report()
        );
        assert!(
            !output.combined().contains("لا ينبغي الوصول هنا"),
            "استمر التنفيذ بعد استثناء غير معالج [{:?}]\n{}",
            backend,
            output.report()
        );
        // The reason, not just `<استثناء>`: an uncaught exception's whole value
        // to the user is the message it carries.
        assert!(
            output.combined().contains("مُهمَل"),
            "رسالة الاستثناء غير المعالج مفقودة [{:?}]\n{}",
            backend,
            output.report()
        );
    }
}

/// The other side of the ت٠٣٠٣ line. `حاول`/`التقط`/`أخيراً` with nothing thrown
/// compiles and runs natively today, so the native block must key on `ارمِ`
/// alone — blocking the whole construct would regress working programs.
#[test]
fn test_try_catch_finally_without_throw_runs_natively() {
    assert_prints(
        r#"
حاول {
    اطبع("داخل حاول")
} التقط (خ) {
    اطبع("داخل التقط")
} أخيراً {
    اطبع("داخل أخيراً")
}
اطبع("بعد")
"#,
        &["داخل حاول", "داخل أخيراً", "بعد"],
        &Backend::ALL,
    );
}

// ==========================================================================
// Rejections
// ==========================================================================

#[test]
fn test_throwing_a_string_is_rejected() {
    assert_rejected_with(
        r#"
حاول {
    ارمِ "مجرد نص"
} التقط (خ) {
    اطبع("لن يحدث")
}
"#,
        "ص٠٦٠١",
        &Backend::ALL,
    );
}

#[test]
fn test_throwing_a_number_is_rejected() {
    assert_rejected_with("ارمِ 42", "ص٠٦٠١", &Backend::ALL);
}

#[test]
fn test_throwing_a_non_exception_class_is_rejected() {
    assert_rejected_with(
        r#"
صنف شخص {
    عام الاسم: نص
}
ارمِ جديد شخص()
"#,
        "ص٠٦٠١",
        &Backend::ALL,
    );
}

/// `register_class` is a `HashMap::insert`: a user class of this name would
/// replace the prelude's silently, and `link_program` would then merge two
/// declarations of it into the IR.
#[test]
fn test_redeclaring_the_base_exception_class_is_rejected() {
    assert_rejected_with(
        r#"
صنف استثناء {
    عام رسالة: نص
    منشئ(رسالة: نص) {
        هذا.رسالة = رسالة
    }
}
اطبع("لن يحدث")
"#,
        "ص٠٦٠٢",
        &Backend::ALL,
    );
}

/// A program with no exceptions at all must not pick up the prelude's costs or
/// its diagnostics — the prelude is now in every program's module cache.
#[test]
fn test_program_without_exceptions_is_unaffected() {
    assert_prints(
        r#"
دالة ضاعف(س: عدد) -> عدد {
    أرجع س * 2
}
اطبع(ضاعف(21))
"#,
        &["42"],
        &Backend::ALL,
    );
}

// ==========================================================================
// The debug interpreter (`tarqeem debug`)
// ==========================================================================

/// `src/debug/interpreter/` duplicates the main interpreter's instruction
/// handling and must be edited in lockstep (issue #223) — it had its own copy of
/// the per-frame `try_stack`, so the cross-frame fix in
/// `interpreter::executor` did not reach it. Without the port, a program whose
/// exception `tarqeem run` catches would abort under `tarqeem debug`: a
/// debug-vs-run divergence, which is the same silent-difference class this whole
/// issue is about.
///
/// This drives `DebugInterpreter` directly rather than the DAP wire protocol,
/// which needs a client on the other end.
#[test]
fn test_debug_interpreter_catches_exception_from_callee() {
    use tarqeem::debug::{DebugContext, DebugInterpreter, StepResult};
    use tarqeem::ir::IrBuilder;
    use tarqeem::parser::Parser;
    use tarqeem::semantic::Analyzer;

    let source = "بسم_الله
دالة يرمي() {
    ارمِ جديد استثناء(\"من الدالة\")
}
دالة رئيسية() {
    حاول {
        يرمي()
    } التقط (خ) {
        اطبع(\"التقطت: \" + خ.رسالة)
    }
}
الحمد_لله";

    let ast = Parser::new(source).parse().expect("يجب أن يُحلَّل البرنامج");

    let mut analyzer = Analyzer::new();
    analyzer
        .analyze(&ast)
        .unwrap_or_else(|d| panic!("فشل التحليل الدلالي: {:?}", d));

    let mut warnings = Vec::new();
    let linked = analyzer
        .linked_ast(&ast, &mut warnings)
        .expect("يجب أن يُدمَج التمهيد");

    let module = IrBuilder::new("تنقيح".to_string())
        .build(&linked)
        .expect("يجب أن يُبنى التمثيل الوسيط");

    let mut debugger = DebugInterpreter::new(module, DebugContext::new());
    let result = debugger.run().expect("يجب أن يكتمل التنفيذ");

    assert!(
        !matches!(result, StepResult::Exception(_)),
        "المنقح أبلغ عن استثناء غير معالج مع وجود «التقط»: {:?}",
        debugger.context().output()
    );
    assert_eq!(
        debugger.context().output(),
        ["التقطت: من الدالة"],
        "خرج المنقح غير متطابق"
    );
}

// ==========================================================================
// Regressions found by review of this change
// ==========================================================================

/// The prelude puts a class declaration in front of every program, which turned
/// a narrow pre-existing bug into a universal one: in Script mode a class built
/// while the synthesized `__main__` is open left the *method's* parameter ids in
/// `IrBuilder::parameters`, so a later variable reusing id 0 — the loop variable
/// of a C-style `لكل` — stopped emitting its `Load` and the condition compared
/// the raw alloca pointer (`متوقع comparable، وُجد ptr`).
///
/// This reproduces on `develop` with a hand-written class, and on this branch
/// with no class at all, since the prelude supplies one.
#[test]
fn test_c_style_loop_at_top_level_still_loads_its_variable() {
    assert_prints(
        r#"
لكل (متغير ع = 0؛ ع < 3؛ ع++) {
    اطبع("دورة")
}
اطبع("انتهى")
"#,
        &["دورة", "دورة", "دورة", "انتهى"],
        &Backend::ALL,
    );
}

#[test]
fn test_break_inside_try_inside_c_style_loop() {
    assert_prints(
        r#"
لكل (متغير ع = 0؛ ع < 3؛ ع++) {
    حاول {
        اطبع("دورة")
        أوقف
    } التقط (خ) {
        اطبع("معالج")
    }
}
اطبع("انتهى")
"#,
        &["دورة", "انتهى"],
        &Backend::ALL,
    );
}

#[test]
fn test_continue_inside_try_inside_c_style_loop() {
    assert_prints(
        r#"
لكل (متغير ع = 0؛ ع < 3؛ ع++) {
    حاول {
        إذا (ع == 1) {
            استمر
        }
        اطبع("دورة")
    } التقط (خ) {
        اطبع("معالج")
    }
}
اطبع("انتهى")
"#,
        &["دورة", "دورة", "انتهى"],
        &Backend::ALL,
    );
}

/// A class declared at top level in Script mode, followed by a C-style loop —
/// the shape that fails on `develop`. Kept separate from the prelude-only case
/// so a future change that makes the prelude lazy cannot silently drop coverage.
#[test]
fn test_user_class_before_c_style_loop_at_top_level() {
    assert_prints(
        r#"
صنف نقطة {
    عام س: عدد
    منشئ(س: عدد) {
        هذا.س = س
    }
}
لكل (متغير ع = 0؛ ع < 2؛ ع++) {
    اطبع("دورة")
}
متغير ن = جديد نقطة(7)
اطبع(ن.س)
"#,
        &["دورة", "دورة", "7"],
        &Backend::ALL,
    );
}

/// Every shape that collides with the prelude must name the real cause. Before
/// the fix only a bare `صنف استثناء` did; `صدّر صنف`, a function, and a module's
/// declaration all fell through to و٠١٠١ naming the pseudo-path `<تمهيد ترقيم>`,
/// which appears nowhere in the user's source.
#[test]
fn test_function_named_after_the_base_exception_class_is_rejected_actionably() {
    assert_rejected_with(
        r#"
دالة استثناء(س: عدد) -> عدد {
    أرجع س
}
اطبع(استثناء(5))
"#,
        "ص٠٦٠٢",
        &Backend::ALL,
    );
}

#[test]
fn test_exported_redeclaration_of_the_base_exception_class_is_rejected() {
    assert_rejected_with(
        r#"
صدّر صنف استثناء {
    عام رسالة: نص
    منشئ(رسالة: نص) {
        هذا.رسالة = رسالة
    }
}
"#,
        "ص٠٦٠٢",
        &Backend::ALL,
    );
}

/// `ارمِ` admits no annotation, so it must outrank an untyped-parameter reason
/// recorded earlier in the *same* function — otherwise the user is told to
/// declare types, does so, and hits the same wall.
///
/// The untyped parameter is on `ف` itself, not on a lambda: a lambda is lifted
/// into its own function, so its ت٠٣٠١ would be reported against that function
/// and never meet the throw's reason at all.
#[test]
fn test_throw_outranks_untyped_param_in_the_native_diagnostic() {
    assert_rejected_with(
        r#"
دالة ف(س) {
    ارمِ جديد استثناء("داخل ف")
}
دالة رئيسية() {
    ف(1)
}
"#,
        NATIVE_EXCEPTIONS_CODE,
        &[Backend::Native],
    );
}
