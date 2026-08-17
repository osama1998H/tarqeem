//! Execution coverage for the always-available core builtins (#222).
//!
//! Six of the core builtins type-checked and ran under the interpreter while
//! failing to compile natively, because `get_runtime_function_name` never knew
//! their names and codegen fell through to mangling the Arabic identifier — a
//! call to a symbol nothing defines. Two more (`طول_مصفوفة`, `الحق`) had the
//! mirror problem: a native mapping with no interpreter implementation.
//!
//! Nothing caught that, because no test ran a core builtin in more than one
//! backend. So every fixture here asserts *stdout*, under all three backends,
//! and `test_every_core_builtin_agrees_across_backends` walks the whole core
//! list so a newly added builtin cannot skip the matrix.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::{fs, str};

use tempfile::TempDir;

const TARQEEM: &str = env!("CARGO_BIN_EXE_tarqeem");

const RUNTIME_LIB: &str = if cfg!(windows) { "trq.lib" } else { "libtrq.a" };

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Guarantees `libtrq.a` exists before the first native case links against it.
///
/// This used to stage a copy into a `TARQEEM_HOME`-shaped directory, because the
/// CLI ranked `TARQEEM_HOME` above everything and never looked in
/// `target/<profile>/` — so merely building the runtime left the native leg
/// linking whatever stale archive sat in `~/.tarqeem/lib`. Since #285 the
/// compiler prefers the archive beside its own executable, which under
/// `cargo test` is the one built here, so building is now enough.
fn ensure_runtime_library() {
    static RUNTIME: OnceLock<Result<(), String>> = OnceLock::new();

    if let Err(message) = RUNTIME.get_or_init(build_runtime_library) {
        panic!("{message}");
    }
}

/// `cargo test` of this package never builds the separate `tarqeem-runtime`
/// crate — the workspace root is itself a package, so the default member set is
/// just `tarqeem`. Build it on demand, at most once.
fn build_runtime_library() -> Result<(), String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut args = vec!["build", "-p", "tarqeem-runtime"];
    if !cfg!(debug_assertions) {
        args.push("--release");
    }

    let output = Command::new(&cargo)
        .args(&args)
        .current_dir(project_root())
        .output()
        .map_err(|e| format!("تعذّر تشغيل cargo لبناء مكتبة وقت التشغيل: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "فشل بناء مكتبة وقت التشغيل:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let built = project_root()
        .join("target")
        .join(profile)
        .join(RUNTIME_LIB);
    // A build that succeeds without producing the artifact means it went
    // somewhere the compiler will not look — `CARGO_TARGET_DIR` being the usual
    // cause. Linking would fail next, with a far less obvious message.
    if !built.exists() {
        return Err(format!(
            "مكتبة وقت التشغيل غير موجودة بعد البناء: {}",
            built.display()
        ));
    }

    Ok(())
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
    /// Set only when the native leg never produced a binary. A compile failure
    /// is indistinguishable from a runtime failure on status and stdout alone —
    /// both exit non-zero with nothing on stdout — so `assert_fails` would keep
    /// passing if a lowering regressed into an LLVM parse error, which is the
    /// exact breakage this suite exists to catch.
    compile_failed: bool,
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

    fn report(&self) -> String {
        format!(
            "الحالة/status: {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            self.status, self.stdout, self.stderr
        )
    }
}

fn write_program(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(format!("{name}.ترقيم"));
    fs::write(&path, format!("بسم_الله\n{}\nالحمد_لله\n", body.trim()))
        .expect("تعذّر كتابة البرنامج");
    path
}

fn tarqeem(args: &[&str], cwd: &Path) -> Output {
    let mut command = Command::new(TARQEEM);
    command.args(args).current_dir(cwd);

    // A stale `TARQEEM_HOME` silently shadows this checkout's `stdlib`, so it is
    // scrubbed on every backend. The runtime archive no longer needs it either:
    // the compiler finds the one built beside its own executable.
    command.env_remove("TARQEEM_HOME");

    let output = command
        .output()
        .unwrap_or_else(|e| panic!("تعذّر تشغيل {} {:?}: {}", TARQEEM, args, e));

    Output {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        compile_failed: false,
    }
}

fn execute(backend: Backend, main: &Path, tag: &str) -> Output {
    let cwd = main.parent().expect("للبرنامج مجلد");
    let arg = main.to_str().expect("مسار غير صالح");

    match backend {
        Backend::Interpreter => tarqeem(&["run", arg], cwd),
        Backend::Jit => tarqeem(&["run", "--jit", arg], cwd),
        Backend::Native => {
            // The only backend that links, so the only one needing libtrq.a.
            ensure_runtime_library();
            let exe = cwd.join(format!("مخرج_{tag}"));
            let exe_arg = exe.to_str().expect("مسار غير صالح").to_string();
            let mut compiled = tarqeem(&["compile", arg, "-o", &exe_arg], cwd);
            if !compiled.succeeded() {
                compiled.compile_failed = true;
                return compiled;
            }

            let run = Command::new(&exe)
                .output()
                .unwrap_or_else(|e| panic!("تعذّر تشغيل {}: {}", exe.display(), e));
            Output {
                status: run.status.code(),
                stdout: String::from_utf8_lossy(&run.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&run.stderr).into_owned(),
                compile_failed: false,
            }
        }
    }
}

/// Asserts `body` prints exactly `expected` under all three backends.
fn assert_prints(name: &str, body: &str, expected: &[&str]) {
    let temp = TempDir::new().unwrap();
    let main = write_program(temp.path(), name, body);

    for backend in Backend::ALL {
        let output = execute(backend, &main, &format!("{name}_{backend:?}"));

        assert!(
            output.succeeded(),
            "فشل التنفيذ [{backend:?}] لـ {name}\nالمتوقع/expected: {expected:?}\n{}",
            output.report()
        );
        assert_eq!(
            output.lines(),
            expected,
            "خرج غير متطابق [{backend:?}] لـ {name}\n{}",
            output.report()
        );
    }
}

/// Asserts `body` fails *at run time* under all three backends, printing
/// nothing on stdout.
///
/// Only the exit status and stdout are compared. `trq_assert` emits a bilingual
/// two-liner while the interpreter formats its own `RuntimeError`, so requiring
/// stderr equality would manufacture a divergence rather than detect one.
fn assert_fails(name: &str, body: &str) {
    let temp = TempDir::new().unwrap();
    let main = write_program(temp.path(), name, body);

    for backend in Backend::ALL {
        let output = execute(backend, &main, &format!("{name}_{backend:?}"));

        assert!(
            !output.compile_failed,
            "توقّعنا فشلاً وقت التنفيذ لا وقت الترجمة [{backend:?}] لـ {name}\n{}",
            output.report()
        );
        assert!(
            !output.succeeded(),
            "توقّعنا فشلاً [{backend:?}] لـ {name}\n{}",
            output.report()
        );
        assert!(
            output.lines().is_empty(),
            "توقّعنا ألا يُطبع شيء على stdout [{backend:?}] لـ {name}\n{}",
            output.report()
        );
    }
}

// ---------------------------------------------------------------------------
// تأكد / تأكد_رسالة
// ---------------------------------------------------------------------------

#[test]
fn test_assert_true_succeeds_in_every_backend() {
    // Natively this used to emit `@_U062A__U0623__U0643__U062F_`, which nothing
    // defines, so the LLVM module failed to parse.
    assert_prints("تأكد_ناجح", "تأكد(صحيح)\nاطبع(\"تم\")", &["تم"]);
}

#[test]
fn test_assert_false_fails_in_every_backend() {
    assert_fails("تأكد_فاشل", "تأكد(خطأ)\nاطبع(\"لا ينبغي طباعته\")");
}

#[test]
fn test_assert_with_message_succeeds_in_every_backend() {
    assert_prints(
        "تأكد_رسالة_ناجح",
        "تأكد_رسالة(صحيح، \"لن تظهر\")\nاطبع(\"تم\")",
        &["تم"],
    );
}

#[test]
fn test_assert_with_message_fails_in_every_backend() {
    assert_fails("تأكد_رسالة_فاشل", "تأكد_رسالة(خطأ، \"رسالتي\")");
}

// ---------------------------------------------------------------------------
// نوع
// ---------------------------------------------------------------------------

#[test]
fn test_type_name_matches_interpreter_for_every_primitive() {
    assert_prints(
        "نوع_الأنماط",
        "اطبع(نوع(5))\nاطبع(نوع(5.0))\nاطبع(نوع(\"س\"))\nاطبع(نوع(صحيح))\nاطبع(نوع([1، 2]))",
        &["عدد", "عدد_عشري", "نص", "منطقي", "مصفوفة"],
    );
}

// ---------------------------------------------------------------------------
// عدد / عدد_عشري / منطقي
// ---------------------------------------------------------------------------

#[test]
fn test_to_int_converts_every_source_type() {
    assert_prints(
        "تحويل_عدد",
        "اطبع(عدد(\"5\"))\nاطبع(عدد(7))\nاطبع(عدد(3.9))\nاطبع(عدد(صحيح))",
        &["5", "7", "3", "1"],
    );
}

#[test]
fn test_to_float_converts_every_source_type() {
    // `عدد_عشري(4)` is asserted through an addition rather than printed
    // directly: native `اطبع` renders a whole float as `4` where the
    // interpreter renders `4.0`, which is #185 and not this fix.
    assert_prints(
        "تحويل_عشري",
        "اطبع(عدد_عشري(\"5.5\"))\nاطبع(عدد_عشري(4) + 0.25)\nاطبع(عدد_عشري(2.5))",
        &["5.5", "4.25", "2.5"],
    );
}

#[test]
fn test_to_bool_follows_truthiness() {
    assert_prints(
        "تحويل_منطقي",
        "اطبع(منطقي(1))\nاطبع(منطقي(0))\nاطبع(منطقي(\"س\"))\nاطبع(منطقي(\"\"))",
        &["صحيح", "خطأ", "صحيح", "خطأ"],
    );
}

#[test]
fn test_to_string_of_a_string_is_the_string_itself() {
    // `convert_to_string` has no `String` arm and fell through to
    // `trq_int_to_string`, so native printed the pointer as `4318534576`.
    assert_prints("نص_من_نص", "اطبع(نص(\"مرحبا\"))", &["مرحبا"]);
}

#[test]
fn test_null_is_falsy_and_names_itself() {
    // `لا_شيء` is typed `Ptr(Void)`, so a type-only answer reports `مؤشر`, and
    // comparing it against `Int 0` made codegen emit `icmp ne ptr %a, %b` with
    // an i64 operand.
    assert_prints(
        "لا_شيء_منطقي",
        "اطبع(نوع(لا_شيء))\nاطبع(منطقي(لا_شيء))",
        &["لا_شيء", "خطأ"],
    );
}

#[test]
fn test_append_uses_the_array_element_type_not_the_value_type() {
    // `elem_ty` taken from the pushed value stored an i64 bit pattern into a
    // float array, which the reader decoded as a denormal double.
    assert_prints(
        "الحق_عشري",
        "متغير م = [1.5]\nالحق(م، 2)\nاطبع(م[1] + 0.5)",
        &["2.5"],
    );
}

#[test]
fn test_invalid_numeric_string_fails_rather_than_yielding_zero() {
    // The runtime's `trq_string_to_int` returns 0 via `unwrap_or`, so wiring it
    // up directly would have printed `0` natively where the interpreter errors —
    // the silent-divergence failure mode this whole file exists to catch.
    assert_fails("عدد_غير_صالح", "اطبع(عدد(\"أبجد\"))");
}

#[test]
fn test_invalid_float_string_fails_rather_than_yielding_zero() {
    assert_fails("عشري_غير_صالح", "اطبع(عدد_عشري(\"أبجد\"))");
}

// ---------------------------------------------------------------------------
// طول_مصفوفة / الحق — the mirror image: mapped natively, absent from the interpreter
// ---------------------------------------------------------------------------

#[test]
fn test_array_len_builtin_works_in_every_backend() {
    assert_prints("طول_مصفوفة_مباشر", "اطبع(طول_مصفوفة([1، 2، 3]))", &["3"]);
}

// ---------------------------------------------------------------------------
// بتات_و — the first bitwise primitive (#302)
// ---------------------------------------------------------------------------

#[test]
fn test_bitwise_and_masks_in_every_backend() {
    assert_prints(
        "بتات_و_قناع",
        "اطبع(بتات_و(12، 10))\nاطبع(بتات_و(255، 0))\nاطبع(بتات_و(7، 7))",
        &["8", "0", "7"],
    );
}

/// `عدد` is a signed i64 in every backend, so a negative left operand is the
/// case where a backend that widened or truncated differently would show it.
/// `-1` is all-ones, which makes the mask the identity.
#[test]
fn test_bitwise_and_agrees_on_negative_operands() {
    assert_prints(
        "بتات_و_سالب",
        "اطبع(بتات_و(-1، 255))\nاطبع(بتات_و(-2، 3))\nاطبع(بتات_و(-8، -3))",
        &["255", "2", "-8"],
    );
}

/// Printing a result proves only that *something* came back. A builtin whose
/// destination carries the `Ptr(Void)` sentinel instead of `عدد` prints
/// plausibly and then composes wrongly, which is the failure this whole
/// registry work exists to stop — so the result is added, compared and passed
/// on as an argument rather than only printed.
#[test]
fn test_bitwise_and_result_composes_as_an_integer() {
    assert_prints(
        "بتات_و_تركيب",
        concat!(
            "اطبع(بتات_و(12، 10) + 1)\n",
            "اطبع(بتات_و(12، 10) == 8)\n",
            "اطبع(نوع(بتات_و(1، 1)))\n",
            "دالة ضاعف(ن: عدد) -> عدد {\n    أرجع ن * 2\n}\n",
            "اطبع(ضاعف(بتات_و(6، 5)))",
        ),
        &["9", "صحيح", "عدد", "8"],
    );
}

/// Builtins are the last tier of the lookup order, so a user function of the
/// same name must win — in every backend at once. An earlier shadowing fix
/// changed only the IR builder and left native calling the user's function
/// while the interpreter still ran the builtin (#262).
#[test]
fn test_user_function_shadows_bitwise_and() {
    assert_prints(
        "بتات_و_مظلل",
        "دالة بتات_و(أ: عدد، ب: عدد) -> عدد {\n    أرجع 42\n}\nاطبع(بتات_و(12، 10))",
        &["42"],
    );
}

// ---------------------------------------------------------------------------
// بتات_أو — the second bitwise primitive (#306)
// ---------------------------------------------------------------------------

#[test]
fn test_bitwise_or_sets_bits_in_every_backend() {
    assert_prints(
        "بتات_أو_ضم",
        "اطبع(بتات_أو(12، 10))\nاطبع(بتات_أو(255، 0))\nاطبع(بتات_أو(0، 0))",
        &["14", "255", "0"],
    );
}

/// `عدد` is a signed i64 in every backend, so a negative left operand is the
/// case where a backend that widened or truncated differently would show it.
/// `-1` is all-ones, which makes it absorbing for OR — the mirror of its being
/// the identity for AND.
#[test]
fn test_bitwise_or_agrees_on_negative_operands() {
    assert_prints(
        "بتات_أو_سالب",
        "اطبع(بتات_أو(-1، 0))\nاطبع(بتات_أو(-2، 1))\nاطبع(بتات_أو(-16، 3))",
        &["-1", "-1", "-13"],
    );
}

/// Printing a result proves only that *something* came back. A builtin whose
/// destination carries the `Ptr(Void)` sentinel instead of `عدد` prints
/// plausibly and then composes wrongly, which is the failure this whole
/// registry work exists to stop — so the result is added, compared and passed
/// on as an argument rather than only printed.
#[test]
fn test_bitwise_or_result_composes_as_an_integer() {
    assert_prints(
        "بتات_أو_تركيب",
        concat!(
            "اطبع(بتات_أو(12، 10) + 1)\n",
            "اطبع(بتات_أو(12، 10) == 14)\n",
            "اطبع(نوع(بتات_أو(1، 2)))\n",
            "دالة ضاعف(ن: عدد) -> عدد {\n    أرجع ن * 2\n}\n",
            "اطبع(ضاعف(بتات_أو(4، 1)))",
        ),
        &["15", "صحيح", "عدد", "10"],
    );
}

/// Why OR is a separate primitive and not a convenience: AND can only inspect
/// bits, so replacing a packed field needs both. `-256` is the inverse of the
/// low-byte mask, which is how a field is cleared until `بتات_نفي` lands.
/// `0x1234` with its low byte replaced by `0xAB` is `0x12AB`.
#[test]
fn test_bitwise_or_and_and_replace_a_packed_field() {
    assert_prints(
        "بتات_أو_حقل",
        "اطبع(بتات_أو(بتات_و(4660، -256)، 171))",
        &["4779"],
    );
}

/// Builtins are the last tier of the lookup order, so a user function of the
/// same name must win — in every backend at once. An earlier shadowing fix
/// changed only the IR builder and left native calling the user's function
/// while the interpreter still ran the builtin (#262).
#[test]
fn test_user_function_shadows_bitwise_or() {
    assert_prints(
        "بتات_أو_مظلل",
        "دالة بتات_أو(أ: عدد، ب: عدد) -> عدد {\n    أرجع 42\n}\nاطبع(بتات_أو(12، 10))",
        &["42"],
    );
}

// ---------------------------------------------------------------------------
// بتات_أو_حصري — the third bitwise primitive (#309)
// ---------------------------------------------------------------------------

#[test]
fn test_bitwise_xor_toggles_bits_in_every_backend() {
    assert_prints(
        "بتات_أو_حصري_ضم",
        "اطبع(بتات_أو_حصري(12، 10))\nاطبع(بتات_أو_حصري(255، 0))\nاطبع(بتات_أو_حصري(7، 7))",
        &["6", "255", "0"],
    );
}

/// What makes XOR a primitive rather than a convenience: it is its own inverse,
/// so masking round-trips without a second operation. Every stream cipher and
/// every xorshift step rests on this identity, and a backend that widened or
/// truncated one operand would break it while a bare truth table still passed.
#[test]
fn test_bitwise_xor_is_self_inverse() {
    assert_prints(
        "بتات_أو_حصري_معكوس",
        concat!(
            "ثابت س = 4660\n",
            "ثابت قناع = 43981\n",
            "اطبع(بتات_أو_حصري(بتات_أو_حصري(س، قناع)، قناع) == س)\n",
            "اطبع(بتات_أو_حصري(بتات_أو_حصري(-7، قناع)، قناع) == -7)",
        ),
        &["صحيح", "صحيح"],
    );
}

/// `عدد` is a signed i64 in every backend, and `-1` is all-ones — so XOR against
/// it is the bitwise complement, the one operation neither `بتات_و` nor
/// `بتات_أو` can express. This is also the case a backend disagreeing about
/// operand width would fail first.
#[test]
fn test_bitwise_xor_against_all_ones_is_complement() {
    assert_prints(
        "بتات_أو_حصري_متمم",
        concat!(
            "اطبع(بتات_أو_حصري(0، -1))\n",
            "اطبع(بتات_أو_حصري(5، -1))\n",
            "اطبع(بتات_أو_حصري(-1، -1))\n",
            "اطبع(بتات_أو_حصري(-8، 3))",
        ),
        &["-1", "-6", "0", "-5"],
    );
}

/// Printing a result proves only that *something* came back. A builtin whose
/// destination carries the `Ptr(Void)` sentinel instead of `عدد` prints
/// plausibly and then composes wrongly, which is the failure this whole
/// registry work exists to stop — so the result is added, compared and passed
/// on as an argument rather than only printed.
#[test]
fn test_bitwise_xor_result_composes_as_an_integer() {
    assert_prints(
        "بتات_أو_حصري_تركيب",
        concat!(
            "اطبع(بتات_أو_حصري(12، 10) + 1)\n",
            "اطبع(بتات_أو_حصري(12، 10) == 6)\n",
            "اطبع(نوع(بتات_أو_حصري(1، 2)))\n",
            "دالة ضاعف(ن: عدد) -> عدد {\n    أرجع ن * 2\n}\n",
            "اطبع(ضاعف(بتات_أو_حصري(4، 1)))",
        ),
        &["7", "صحيح", "عدد", "10"],
    );
}

/// The three bitwise names lower through one IR-builder arm that picks its op by
/// name. A sibling added to the arm's pattern but not to its inner `match`
/// falls through un-intercepted; one added to the inner `match` under the wrong
/// op emits a plausible number. Composing all three in a single expression
/// pins each to its own operation.
#[test]
fn test_bitwise_family_ops_do_not_collide() {
    assert_prints(
        "بتات_عائلة",
        concat!(
            "اطبع(بتات_و(12، 10))\n",
            "اطبع(بتات_أو(12، 10))\n",
            "اطبع(بتات_أو_حصري(12، 10))\n",
            // XOR is OR minus AND: 14 - 8 = 6, so a family that collapsed onto
            // one op cannot satisfy this line.
            "اطبع(بتات_أو_حصري(12، 10) == بتات_أو(12، 10) - بتات_و(12، 10))",
        ),
        &["8", "14", "6", "صحيح"],
    );
}

/// Builtins are the last tier of the lookup order, so a user function of the
/// same name must win — in every backend at once. An earlier shadowing fix
/// changed only the IR builder and left native calling the user's function
/// while the interpreter still ran the builtin (#262).
#[test]
fn test_user_function_shadows_bitwise_xor() {
    assert_prints(
        "بتات_أو_حصري_مظلل",
        "دالة بتات_أو_حصري(أ: عدد، ب: عدد) -> عدد {\n    أرجع 42\n}\nاطبع(بتات_أو_حصري(12، 10))",
        &["42"],
    );
}

// ---------------------------------------------------------------------------
// بتات_نفي — the fourth bitwise primitive, and the first unary one (#312)
// ---------------------------------------------------------------------------

#[test]
fn test_bitwise_not_complements_in_every_backend() {
    assert_prints(
        "بتات_نفي_ضم",
        concat!(
            "اطبع(بتات_نفي(0))\n",
            "اطبع(بتات_نفي(-1))\n",
            "اطبع(بتات_نفي(5))\n",
            "اطبع(بتات_نفي(255))",
        ),
        &["-1", "0", "-6", "-256"],
    );
}

/// `عدد` is a signed i64, so complementing is `-س - ١` exactly. A backend that
/// complemented a narrower value — or that treated the operand as `منطقي`, which
/// the interpreter's `BitNot` arm also accepts — would still print a plausible
/// number while failing this.
#[test]
fn test_bitwise_not_matches_twos_complement_identity() {
    assert_prints(
        "بتات_نفي_متمم_اثنين",
        concat!(
            "اطبع(بتات_نفي(0) == -1)\n",
            "اطبع(بتات_نفي(42) == -43)\n",
            "اطبع(بتات_نفي(-43) == 42)\n",
            "اطبع(بتات_نفي(بتات_نفي(4660)) == 4660)",
        ),
        &["صحيح", "صحيح", "صحيح", "صحيح"],
    );
}

/// The operation this name spells was already reachable as
/// `بتات_أو_حصري(س، -1)` once #309 landed, so the primitive earns its slot only
/// if it agrees with that form everywhere. Asserting the two together is what
/// pins the new arm to `BitNot` rather than to some other unary op that happens
/// to return an integer.
#[test]
fn test_bitwise_not_agrees_with_xor_against_all_ones() {
    assert_prints(
        "بتات_نفي_يطابق_الحصري",
        concat!(
            "ثابت س = 4660\n",
            "اطبع(بتات_نفي(س) == بتات_أو_حصري(س، -1))\n",
            "اطبع(بتات_نفي(0) == بتات_أو_حصري(0، -1))\n",
            "اطبع(بتات_نفي(-7) == بتات_أو_حصري(-7، -1))",
        ),
        &["صحيح", "صحيح", "صحيح"],
    );
}

/// `بتات_نفي` is the first of the family to emit `Instruction::Unary`, whose
/// codegen path had never been reached from source before. Printing alone would
/// not distinguish a correct `عدد` from the `Ptr(Void)` sentinel, so the result
/// is added, compared and passed on.
#[test]
fn test_bitwise_not_result_composes_as_an_integer() {
    assert_prints(
        "بتات_نفي_تركيب",
        concat!(
            "اطبع(بتات_نفي(255) + 256)\n",
            "اطبع(بتات_نفي(255) == -256)\n",
            "اطبع(نوع(بتات_نفي(1)))\n",
            "دالة ضاعف(ن: عدد) -> عدد {\n    أرجع ن * 2\n}\n",
            "اطبع(ضاعف(بتات_نفي(-4)))",
        ),
        &["0", "صحيح", "عدد", "6"],
    );
}

/// The inverse mask is what a complement is *for*: clearing a packed field
/// before writing it. Until this name landed the example wrote `-256` by hand;
/// this asserts the computed form agrees, across all three backends.
#[test]
fn test_bitwise_not_builds_an_inverse_mask() {
    assert_prints(
        "بتات_نفي_قناع",
        concat!(
            "دالة استبدل_البايت_الأدنى(قيمة: عدد، بايت: عدد) -> عدد {\n",
            "    أرجع بتات_أو(بتات_و(قيمة، بتات_نفي(255))، بايت)\n",
            "}\n",
            "اطبع(استبدل_البايت_الأدنى(4660، 171))\n",
            "اطبع(بتات_نفي(255) == -256)",
        ),
        &["4779", "صحيح"],
    );
}

/// A narrowed optional is still a boxed pointer in codegen, and only
/// `emit_binary` unboxed it — so `بتات_و(س، 255)` compiled while `بتات_نفي(س)`
/// on the same `س` emitted `xor i64 %ptr, -1` and clang rejected the module, an
/// error neither the interpreter nor the JIT ever saw. Both calls sit in one
/// fixture so the asymmetry cannot come back unnoticed.
#[test]
fn test_bitwise_not_over_a_narrowed_optional() {
    assert_prints(
        "بتات_نفي_اختياري",
        concat!(
            "متغير س: عدد? = 255\n",
            "إذا (س != لا_شيء) {\n",
            "    اطبع(بتات_نفي(س))\n",
            "    اطبع(بتات_و(س، 255))\n",
            "    اطبع(بتات_نفي(س) + 1)\n",
            "}",
        ),
        &["-256", "255", "-255"],
    );
}

/// Builtins are the last tier of the lookup order, so a user function of the
/// same name must win — in every backend at once. An earlier shadowing fix
/// changed only the IR builder and left native calling the user's function
/// while the interpreter still ran the builtin (#262).
#[test]
fn test_user_function_shadows_bitwise_not() {
    assert_prints(
        "بتات_نفي_مظلل",
        "دالة بتات_نفي(س: عدد) -> عدد {\n    أرجع 42\n}\nاطبع(بتات_نفي(255))",
        &["42"],
    );
}

// ---------------------------------------------------------------------------
// بتات_إزاحة_يسار — the fifth bitwise primitive, and the first shift (#317)
// ---------------------------------------------------------------------------

#[test]
fn test_left_shift_moves_bits_in_every_backend() {
    assert_prints(
        "إزاحة_يسار_ضم",
        concat!(
            "اطبع(بتات_إزاحة_يسار(1، 0))\n",
            "اطبع(بتات_إزاحة_يسار(1، 1))\n",
            "اطبع(بتات_إزاحة_يسار(1، 10))\n",
            "اطبع(بتات_إزاحة_يسار(3، 4))\n",
            "اطبع(بتات_إزاحة_يسار(0، 5))",
        ),
        &["1", "2", "1024", "48", "0"],
    );
}

/// `عدد` is signed, so a shift is not a multiplication once the bit reaches the
/// sign position. Bit 63 is the boundary the guard has to admit — an
/// off-by-one that rejected it would zero this line while every smaller amount
/// still passed.
#[test]
fn test_left_shift_reaches_the_sign_bit_and_negative_operands() {
    assert_prints(
        "إزاحة_يسار_إشارة",
        concat!(
            "اطبع(بتات_إزاحة_يسار(1، 62))\n",
            "اطبع(بتات_إزاحة_يسار(1، 63))\n",
            "اطبع(بتات_إزاحة_يسار(-1، 4))\n",
            "اطبع(بتات_إزاحة_يسار(-3، 2))",
        ),
        &["4611686018427387904", "-9223372036854775808", "-16", "-12"],
    );
}

/// Below the overflow point a left shift *is* multiplication by a power of two,
/// and saying so in the language pins the direction: a backend that shifted
/// right instead would still print plausible integers.
#[test]
fn test_left_shift_agrees_with_multiplication_below_overflow() {
    assert_prints(
        "إزاحة_يسار_يطابق_الضرب",
        concat!(
            "اطبع(بتات_إزاحة_يسار(1، 8) == 1 * 2 ** 8)\n",
            "اطبع(بتات_إزاحة_يسار(7، 5) == 7 * 2 ** 5)\n",
            "اطبع(بتات_إزاحة_يسار(-6، 3) == -6 * 2 ** 3)",
        ),
        &["صحيح", "صحيح", "صحيح"],
    );
}

/// The contract, and the reason this primitive lowers to a guarded chain rather
/// than a bare `Shl`. Unguarded, this fixture would be a runtime error in the
/// two interpreters, `1` from the constant folder natively, and poison natively
/// once the amount stopped being a literal — the same call disagreeing with
/// itself across backends, which is what §11 rule 4 gates.
#[test]
fn test_left_shift_is_total_outside_the_valid_range() {
    assert_prints(
        "إزاحة_يسار_خارج_النطاق",
        concat!(
            "اطبع(بتات_إزاحة_يسار(1، 64))\n",
            "اطبع(بتات_إزاحة_يسار(1، 65))\n",
            "اطبع(بتات_إزاحة_يسار(255، 1000))\n",
            "اطبع(بتات_إزاحة_يسار(1، -1))\n",
            "اطبع(بتات_إزاحة_يسار(-1، -64))",
        ),
        &["0", "0", "0", "0", "0"],
    );
}

/// The literals above are folded away before native codegen ever sees a shift,
/// so they exercise the constant folder and nothing else. A variable amount is
/// the other half of the contract — it is the only path that reaches LLVM's
/// `shl i64`, whose out-of-range result is poison.
#[test]
fn test_left_shift_guards_a_runtime_amount_too() {
    assert_prints(
        "إزاحة_يسار_مقدار_متغير",
        concat!(
            "متغير مقدار = 3\n",
            "اطبع(بتات_إزاحة_يسار(5، مقدار))\n",
            "مقدار = 64\n",
            "اطبع(بتات_إزاحة_يسار(5، مقدار))\n",
            "مقدار = -1\n",
            "اطبع(بتات_إزاحة_يسار(5، مقدار))\n",
            "دالة أزح(قيمة: عدد، ن: عدد) -> عدد {\n    أرجع بتات_إزاحة_يسار(قيمة، ن)\n}\n",
            "اطبع(أزح(1، 4))\n",
            "اطبع(أزح(1، 99))",
        ),
        &["40", "0", "0", "16", "0"],
    );
}

/// The most extreme amount representable, reached by shifting rather than
/// written as a literal — negating `9223372036854775808` would not fit an `عدد`.
/// It is the one input where the guard's `٠ - (ن >> ٦)` could overflow if the
/// chain subtracted the amount itself instead of its shifted quotient.
#[test]
fn test_left_shift_handles_the_most_negative_amount() {
    assert_prints(
        "إزاحة_يسار_أصغر_مقدار",
        concat!(
            "ثابت الأصغر = بتات_إزاحة_يسار(1، 63)\n",
            "اطبع(الأصغر)\n",
            "اطبع(بتات_إزاحة_يسار(1، الأصغر))\n",
            "اطبع(بتات_إزاحة_يسار(255، بتات_نفي(الأصغر)))",
        ),
        &["-9223372036854775808", "0", "0"],
    );
}

/// The chain ends in a `BitAnd`, so its destination is what types the call.
/// Printing alone would not tell a real `عدد` from the `Ptr(Void)` sentinel a
/// missing type registration leaves behind.
#[test]
fn test_left_shift_result_composes_as_an_integer() {
    assert_prints(
        "إزاحة_يسار_تركيب",
        concat!(
            "اطبع(نوع(بتات_إزاحة_يسار(1، 3)))\n",
            "اطبع(بتات_إزاحة_يسار(1، 3) + 1)\n",
            "اطبع(بتات_إزاحة_يسار(1، 3) == 8)\n",
            "اطبع(بتات_و(بتات_إزاحة_يسار(1، 8)، 255))\n",
            "دالة ضاعف(ن: عدد) -> عدد {\n    أرجع ن * 2\n}\n",
            "اطبع(ضاعف(بتات_إزاحة_يسار(1، 3)))",
        ),
        &["عدد", "9", "صحيح", "0", "16"],
    );
}

/// A narrowed optional is still a boxed pointer in codegen, so it is covered in
/// both argument positions and in both at once. The amount position is the one
/// that matters: the guard reads that operand twice, and codegen unboxes only a
/// `VarId`'s first scalar use (#318) — which is why the lowering copies it once
/// before the chain. Without that copy this fixture emitted `and i64 %ptr, …`
/// and clang rejected the module, while the interpreter and JIT both answered
/// correctly.
#[test]
fn test_left_shift_over_a_narrowed_optional() {
    assert_prints(
        "إزاحة_يسار_اختياري",
        concat!(
            "متغير س: عدد? = 5\n",
            "إذا (س != لا_شيء) {\n",
            "    اطبع(بتات_إزاحة_يسار(س، 3))\n",
            "    اطبع(بتات_إزاحة_يسار(1، س))\n",
            "    اطبع(بتات_إزاحة_يسار(س، س))\n",
            "    اطبع(بتات_إزاحة_يسار(س، 3) + 1)\n",
            "}",
        ),
        &["40", "32", "160", "41"],
    );
}

/// Builtins are the last tier of the lookup order, so a user function of the
/// same name must win — in every backend at once (#262).
#[test]
fn test_user_function_shadows_left_shift() {
    assert_prints(
        "إزاحة_يسار_مظلل",
        "دالة بتات_إزاحة_يسار(س: عدد، ن: عدد) -> عدد {\n    أرجع 42\n}\nاطبع(بتات_إزاحة_يسار(1، 3))",
        &["42"],
    );
}

// ---------------------------------------------------------------------------
// بتات_إزاحة_يمين — the arithmetic right shift (#320)
// ---------------------------------------------------------------------------

#[test]
fn test_right_shift_moves_bits_in_every_backend() {
    assert_prints(
        "إزاحة_يمين_ضم",
        concat!(
            "اطبع(بتات_إزاحة_يمين(8، 1))\n",
            "اطبع(بتات_إزاحة_يمين(1، 0))\n",
            "اطبع(بتات_إزاحة_يمين(1، 1))\n",
            "اطبع(بتات_إزاحة_يمين(1024، 10))\n",
            "اطبع(بتات_إزاحة_يمين(48، 4))\n",
            "اطبع(بتات_إزاحة_يمين(0، 5))",
        ),
        &["4", "1", "0", "1", "3", "0"],
    );
}

/// The one property that separates this name from the logical shift it will sit
/// beside, and the only fixture that can catch a backend wired to `lshr`
/// instead of `ashr`: every line here would print a large positive number, all
/// of them plausible integers that no other fixture rejects.
#[test]
fn test_right_shift_propagates_the_sign() {
    assert_prints(
        "إزاحة_يمين_إشارة",
        concat!(
            "اطبع(بتات_إزاحة_يمين(-8، 1))\n",
            "اطبع(بتات_إزاحة_يمين(-16، 4))\n",
            "اطبع(بتات_إزاحة_يمين(-1، 1))\n",
            "اطبع(بتات_إزاحة_يمين(-1، 63))\n",
            "اطبع(بتات_إزاحة_يمين(بتات_إزاحة_يسار(1، 63)، 62))",
        ),
        &["-4", "-1", "-1", "-1", "-2"],
    );
}

/// A right shift floors and `/` truncates, so on a negative operand they are
/// genuinely different operations rather than two spellings of one. Pinning the
/// disagreement is what stops a later "simplification" into a division.
#[test]
fn test_right_shift_floors_where_division_truncates() {
    assert_prints(
        "إزاحة_يمين_تقريب",
        concat!(
            "اطبع(بتات_إزاحة_يمين(-7، 1))\n",
            "اطبع(-7 / 2)\n",
            "اطبع(بتات_إزاحة_يمين(-7، 1) == -7 / 2)\n",
            "اطبع(بتات_إزاحة_يمين(-1، 1))\n",
            "اطبع(-1 / 2)",
        ),
        &["-4", "-3", "خطأ", "-1", "0"],
    );
}

/// Where the operand is non-negative the two do agree, and saying so pins the
/// direction: a backend that shifted left instead would still print plausible
/// integers everywhere above.
#[test]
fn test_right_shift_agrees_with_division_when_non_negative() {
    assert_prints(
        "إزاحة_يمين_يطابق_القسمة",
        concat!(
            "اطبع(بتات_إزاحة_يمين(1024، 10) == 1024 / 2 ** 10)\n",
            "اطبع(بتات_إزاحة_يمين(255، 4) == 255 / 2 ** 4)\n",
            "اطبع(بتات_إزاحة_يمين(7، 5) == 7 / 2 ** 5)\n",
            "اطبع(بتات_إزاحة_يمين(7، 1) == 7 / 2)",
        ),
        &["صحيح", "صحيح", "صحيح", "صحيح"],
    );
}

/// The contract, and where it parts company with `بتات_إزاحة_يسار`. An
/// arithmetic shift vacates the high end and refills it from the sign, so
/// shifting everything out leaves the sign rather than zero. Zeroing instead
/// would put a cliff between the last two lines of
/// `test_right_shift_propagates_the_sign` and the first line here, with nothing
/// about the operand having changed.
#[test]
fn test_right_shift_is_total_outside_the_valid_range() {
    assert_prints(
        "إزاحة_يمين_خارج_النطاق",
        concat!(
            "اطبع(بتات_إزاحة_يمين(-1، 64))\n",
            "اطبع(بتات_إزاحة_يمين(-1، 65))\n",
            "اطبع(بتات_إزاحة_يمين(-1، 1000))\n",
            "اطبع(بتات_إزاحة_يمين(-1، -1))\n",
            "اطبع(بتات_إزاحة_يمين(255، 64))\n",
            "اطبع(بتات_إزاحة_يمين(255، -1))",
        ),
        &["-1", "-1", "-1", "-1", "0", "0"],
    );
}

/// The literals above are folded away before native codegen ever sees a shift,
/// so they exercise the constant folder and nothing else. A variable amount is
/// the other half of the contract — it is the only path that reaches LLVM's
/// `ashr i64`, whose out-of-range result is poison.
#[test]
fn test_right_shift_guards_a_runtime_amount_too() {
    assert_prints(
        "إزاحة_يمين_مقدار_متغير",
        concat!(
            "متغير مقدار = 2\n",
            "اطبع(بتات_إزاحة_يمين(-100، مقدار))\n",
            "مقدار = 64\n",
            "اطبع(بتات_إزاحة_يمين(-100، مقدار))\n",
            "اطبع(بتات_إزاحة_يمين(100، مقدار))\n",
            "مقدار = -1\n",
            "اطبع(بتات_إزاحة_يمين(-100، مقدار))\n",
            "دالة أزح_يميناً(قيمة: عدد، ن: عدد) -> عدد {\n",
            "    أرجع بتات_إزاحة_يمين(قيمة، ن)\n",
            "}\n",
            "اطبع(أزح_يميناً(-64، 3))\n",
            "اطبع(أزح_يميناً(-64، 99))\n",
            "اطبع(أزح_يميناً(64، 99))",
        ),
        &["-25", "-1", "0", "-1", "-8", "-1", "0"],
    );
}

/// The most extreme amount representable, reached by shifting rather than
/// written as a literal — negating `9223372036854775808` would not fit an `عدد`.
/// It is the one input where the guard's `٠ - (ن >> ٦)` could overflow if the
/// chain subtracted the amount itself instead of its shifted quotient. The last
/// two lines use it as the *value* rather than the amount, where `٦٣` is the
/// amount that saturation produces and so must already be correct.
#[test]
fn test_right_shift_handles_the_most_negative_amount() {
    assert_prints(
        "إزاحة_يمين_أصغر_مقدار",
        concat!(
            "ثابت الأصغر = بتات_إزاحة_يسار(1، 63)\n",
            "اطبع(بتات_إزاحة_يمين(-1، الأصغر))\n",
            "اطبع(بتات_إزاحة_يمين(255، الأصغر))\n",
            "اطبع(بتات_إزاحة_يمين(الأصغر، 63))\n",
            "اطبع(بتات_إزاحة_يمين(الأصغر، 62))",
        ),
        &["-1", "0", "-1", "-2"],
    );
}

/// The chain ends in the shift itself, so its destination is what types the
/// call. Printing alone would not tell a real `عدد` from the `Ptr(Void)`
/// sentinel a missing type registration leaves behind.
#[test]
fn test_right_shift_result_composes_as_an_integer() {
    assert_prints(
        "إزاحة_يمين_تركيب",
        concat!(
            "اطبع(نوع(بتات_إزاحة_يمين(8، 1)))\n",
            "اطبع(بتات_إزاحة_يمين(8، 1) + 1)\n",
            "اطبع(بتات_إزاحة_يمين(8، 1) == 4)\n",
            "اطبع(بتات_و(بتات_إزاحة_يمين(-1، 4)، 255))\n",
            "دالة ضاعف(ن: عدد) -> عدد {\n    أرجع ن * 2\n}\n",
            "اطبع(ضاعف(بتات_إزاحة_يمين(8، 1)))",
        ),
        &["عدد", "5", "صحيح", "255", "8"],
    );
}

/// The two shifts against each other, which is how they are actually used: a
/// left shift builds a mask at a position and a right shift brings the field it
/// selects back down to its own value. A direction swap in either lowering
/// breaks the round trip while leaving both names individually plausible.
#[test]
fn test_right_shift_undoes_the_left_shift() {
    assert_prints(
        "إزاحة_يمين_ذهاب_وإياب",
        concat!(
            "اطبع(بتات_إزاحة_يمين(بتات_إزاحة_يسار(1، 10)، 10))\n",
            "اطبع(بتات_إزاحة_يمين(بتات_إزاحة_يسار(-3، 4)، 4))\n",
            "اطبع(بتات_إزاحة_يمين(بتات_إزاحة_يسار(255، 8)، 8) == 255)\n",
            "اطبع(بتات_إزاحة_يمين(بتات_و(4660، بتات_إزاحة_يسار(255، 8))، 8))",
        ),
        &["1", "-3", "صحيح", "18"],
    );
}

/// A narrowed optional is still a boxed pointer in codegen, so it is covered in
/// both argument positions and in both at once. The amount position is the one
/// that matters: the guard reads that operand twice, and codegen unboxes only a
/// `VarId`'s first scalar use (#318) — which is why the shared guard copies it
/// once before the chain.
#[test]
fn test_right_shift_over_a_narrowed_optional() {
    assert_prints(
        "إزاحة_يمين_اختياري",
        concat!(
            "متغير س: عدد? = 5\n",
            "إذا (س != لا_شيء) {\n",
            "    اطبع(بتات_إزاحة_يمين(س، 1))\n",
            "    اطبع(بتات_إزاحة_يمين(64، س))\n",
            "    اطبع(بتات_إزاحة_يمين(س، س))\n",
            "    اطبع(بتات_إزاحة_يمين(س، 1) + 1)\n",
            "}",
        ),
        &["2", "2", "0", "3"],
    );
}

/// Builtins are the last tier of the lookup order, so a user function of the
/// same name must win — in every backend at once (#262).
#[test]
fn test_user_function_shadows_right_shift() {
    assert_prints(
        "إزاحة_يمين_مظلل",
        "دالة بتات_إزاحة_يمين(س: عدد، ن: عدد) -> عدد {\n    أرجع 42\n}\nاطبع(بتات_إزاحة_يمين(8، 1))",
        &["42"],
    );
}

// ---------------------------------------------------------------------------
// طول over a string — characters, not bytes (#185)
// ---------------------------------------------------------------------------

/// `ArrayLen` is polymorphic in every interpreting backend but a single symbol
/// in codegen, and picking the array symbol for a string is *silent*: both
/// `TrqString` and `TrqArray` are `#[repr(C)]` with `len` first, so the load
/// succeeds and returns the byte count. Five Arabic characters occupy ten
/// bytes, which is why this fixture is Arabic rather than ASCII — the same
/// assertion over "hello" would pass against the bug.
#[test]
fn test_string_len_counts_characters_not_bytes() {
    assert_prints("طول_نص_عربي", "اطبع(طول(\"مرحبا\"))", &["5"]);
}

/// A literal and a parameter reach `var_types` by different routes (constant
/// emission vs parameter registration), so covering only the literal would
/// leave half the dispatch untested.
#[test]
fn test_string_len_counts_characters_through_a_parameter() {
    assert_prints(
        "طول_نص_معامل",
        "دالة قِس(ن: نص) -> عدد {\n    أرجع طول(ن)\n}\nاطبع(قِس(\"مرحبا\"))",
        &["5"],
    );
}

/// Arrays must not regress: the same instruction serves both.
#[test]
fn test_array_len_still_counts_elements() {
    assert_prints("طول_مصفوفة_لم_يتغير", "اطبع(طول([1، 2، 3]))", &["3"]);
}

/// Indexing a string yields a one-character string, as in the interpreter.
/// Natively this reached `trq_array_get`, which read the string's byte pointer
/// as an element table and aborted.
#[test]
fn test_string_index_yields_one_character() {
    assert_prints(
        "فهرسة_نص",
        "متغير ن = \"مرحبا\"\nاطبع(ن[0])\nاطبع(ن[4])",
        &["م", "ا"],
    );
}

/// `لكل … في` shares `ArrayLen` for its trip count, so the fix changes native
/// string iteration from bytes to characters too.
#[test]
fn test_string_iteration_visits_characters_not_bytes() {
    assert_prints(
        "تكرار_نص",
        "متغير ع = 0\nلكل ح في \"مرحبا\" {\n    ع = ع + 1\n}\nاطبع(ع)",
        &["5"],
    );
}

// ---------------------------------------------------------------------------
// Float display — native must match the interpreter (#185)
// ---------------------------------------------------------------------------

/// A whole float keeps its `.0`. The runtime printed `value as i64` under a
/// `%g` convention no other backend followed.
#[test]
fn test_whole_float_prints_with_a_decimal_place() {
    assert_prints("عشري_صحيح", "اطبع(5.0)", &["5.0"]);
}

/// The runtime's guard also carried an `abs() < 1e15` clause the interpreter
/// never had, so the two agreed on nothing above that magnitude either.
#[test]
fn test_large_whole_float_prints_with_a_decimal_place() {
    assert_prints("عشري_كبير", "اطبع(1.0e20)", &["100000000000000000000.0"]);
}

/// A fractional float was always consistent — pinned so the fix cannot regress
/// the branch it did not touch.
#[test]
fn test_fractional_float_is_unchanged() {
    assert_prints("عشري_كسري", "اطبع(5.5)", &["5.5"]);
}

// ---------------------------------------------------------------------------
// Comparison operand dispatch, and optional representation (#185)
// ---------------------------------------------------------------------------
//
// These are not builtins, but they need exactly the harness above: a divergence
// that only appears in one backend is invisible to any single-backend test, and
// duplicating `assert_prints` into another file would be the more expensive
// mistake. Note the JIT leg proves less here than it looks — Cranelift compiles
// neither of these instruction shapes and delegates to the interpreter, so the
// JIT column agrees by fallback rather than by compiling anything (#215).

/// Comparison opcodes were chosen from the instruction's *result* type, which is
/// always Bool, so every non-Int operand hit an arm that spelled `i64`. Integers
/// were correct only by coincidence; booleans could not be compiled at all.
#[test]
fn test_booleans_compare_in_every_backend() {
    assert_prints(
        "مقارنة_منطقي",
        "متغير أ = صحيح\nمتغير ب = صحيح\nإذا (أ == ب) { اطبع(\"متساوي\") } وإلا { اطبع(\"مختلف\") }",
        &["متساوي"],
    );
}

#[test]
fn test_optional_compared_against_null_in_every_backend() {
    for (ty, value) in [
        ("نص", "\"أ\""),
        ("عدد", "5"),
        ("عدد_عشري", "2.5"),
        ("منطقي", "صحيح"),
    ] {
        assert_prints(
            &format!("اختياري_معيّن_{ty}"),
            &format!(
                "متغير س: {ty}? = {value}\nإذا (س != لا_شيء) {{ اطبع(\"موجود\") }} وإلا {{ اطبع(\"فارغ\") }}"
            ),
            &["موجود"],
        );
        assert_prints(
            &format!("اختياري_فارغ_{ty}"),
            &format!(
                "متغير س: {ty}? = لا_شيء\nإذا (س != لا_شيء) {{ اطبع(\"موجود\") }} وإلا {{ اطبع(\"فارغ\") }}"
            ),
            &["فارغ"],
        );
    }
}

/// The falsy scalars are the whole point of boxing.
///
/// An optional lowers to a pointer, and a scalar stored raw into that slot is
/// its own bit pattern — so `0`, `خطأ` and `0.0` were indistinguishable from a
/// null pointer. Fixing the comparison without fixing the representation would
/// have turned a build error into a silent wrong answer, which is strictly worse.
#[test]
fn test_falsy_scalar_optionals_are_not_null() {
    for (ty, value) in [("عدد", "0"), ("منطقي", "خطأ"), ("عدد_عشري", "0.0")] {
        assert_prints(
            &format!("اختياري_صفري_{ty}"),
            &format!(
                "متغير س: {ty}? = {value}\nإذا (س != لا_شيء) {{ اطبع(\"موجود\") }} وإلا {{ اطبع(\"فارغ\") }}"
            ),
            &["موجود"],
        );
    }
}

/// Boxing has to happen wherever `T` is implicitly widened to `T?`, not only at
/// a `متغير` declaration — an argument and a return value are coercion sites too.
#[test]
fn test_optionals_are_boxed_at_argument_and_return_positions() {
    assert_prints(
        "اختياري_معامل",
        "دالة افحص(س: عدد?) {\n    إذا (س != لا_شيء) { اطبع(\"موجود\") } وإلا { اطبع(\"فارغ\") }\n}\nافحص(0)\nافحص(لا_شيء)",
        &["موجود", "فارغ"],
    );
    assert_prints(
        "اختياري_إرجاع",
        "دالة اصنع() -> عدد? { أرجع 0 }\nمتغير س = اصنع()\nإذا (س != لا_شيء) { اطبع(\"موجود\") } وإلا { اطبع(\"فارغ\") }",
        &["موجود"],
    );
}

/// Fields and method parameters are coercion sites too.
///
/// These were found only by going looking for sites the first pass had missed —
/// both compiled cleanly and answered `فارغ` for a present `0`, which is the
/// silent wrong answer the boxing exists to prevent.
#[test]
fn test_optionals_are_boxed_in_fields_and_method_parameters() {
    assert_prints(
        "اختياري_حقل",
        "صنف حساب {\n    عام رصيد: عدد?\n    منشئ() { هذا.رصيد = 0 }\n    عام دالة افحص() {\n        إذا (هذا.رصيد != لا_شيء) { اطبع(\"موجود\") } وإلا { اطبع(\"فارغ\") }\n    }\n}\nمتغير ح = جديد حساب()\nح.افحص()",
        &["موجود"],
    );
    assert_prints(
        "اختياري_معامل_دالة_عضو",
        "صنف فاحص {\n    منشئ() { }\n    عام دالة افحص(س: عدد?) {\n        إذا (س != لا_شيء) { اطبع(\"موجود\") } وإلا { اطبع(\"فارغ\") }\n    }\n}\nمتغير ف = جديد فاحص()\nف.افحص(0)",
        &["موجود"],
    );
}

/// Every other fixture here is script mode, where a `متغير` is a *global* and
/// takes the global store path. Inside a function it is an alloca instead —
/// a different branch, and one that only manual runs had covered.
#[test]
fn test_falsy_scalar_optional_is_not_null_inside_a_function() {
    assert_prints(
        "اختياري_صفري_محلي",
        "دالة رئيسية() {\n    متغير س: عدد? = 0\n    إذا (س != لا_شيء) { اطبع(\"موجود\") } وإلا { اطبع(\"فارغ\") }\n}",
        &["موجود"],
    );
}

/// Printing a scalar optional segfaulted: the pointer went to `trq_print`, which
/// reads it as a `TrqString*`.
#[test]
fn test_printing_a_scalar_optional_matches_the_interpreter() {
    assert_prints("طباعة_اختياري_عدد", "متغير س: عدد? = 5\nاطبع(س)", &["5"]);
    assert_prints(
        "طباعة_اختياري_فارغ",
        "متغير س: عدد? = لا_شيء\nاطبع(س)",
        &["لا_شيء"],
    );
    assert_prints(
        "طباعة_اختياري_عشري",
        "متغير س: عدد_عشري? = 2.5\nاطبع(س)",
        &["2.5"],
    );
    assert_prints(
        "طباعة_اختياري_منطقي",
        "متغير س: منطقي? = صحيح\nاطبع(س)",
        &["صحيح"],
    );
}

/// Two optional strings compare by *value*, as the interpreter does. Routing all
/// pointer-typed operands to `icmp ptr` would have made this pointer identity —
/// trading a build error for a wrong answer, again.
#[test]
fn test_optional_strings_compare_by_value_not_identity() {
    assert_prints(
        "اختياري_نص_بالقيمة",
        "متغير أ: نص? = \"سلام\"\nمتغير ب: نص? = \"سلا\" + \"م\"\nإذا (أ == ب) { اطبع(\"متساوي\") } وإلا { اطبع(\"مختلف\") }",
        &["متساوي"],
    );
}

// ---------------------------------------------------------------------------
// Null-check narrowing — LANGUAGE_SPEC §13.4 (#185)
// ---------------------------------------------------------------------------

/// The example from the spec: after the check, the value is usable as its
/// unwrapped type.
#[test]
fn test_null_check_narrows_in_the_then_branch() {
    assert_prints(
        "تضييق_ثم",
        "متغير س: عدد? = 5\nإذا (س != لا_شيء) { اطبع(س + 1) }",
        &["6"],
    );
}

/// `لا_شيء != س` reads as naturally in Arabic as the reverse, so both operand
/// orders narrow.
#[test]
fn test_null_check_narrows_with_operands_reversed() {
    assert_prints(
        "تضييق_معكوس",
        "متغير س: عدد? = 5\nإذا (لا_شيء != س) { اطبع(س + 1) }",
        &["6"],
    );
}

/// `==` proves the opposite branch.
#[test]
fn test_null_check_narrows_the_else_branch() {
    assert_prints(
        "تضييق_وإلا",
        "متغير س: عدد? = 5\nإذا (س == لا_شيء) { اطبع(0) } وإلا { اطبع(س + 1) }",
        &["6"],
    );
}

/// Concatenating a narrowed optional printed the box's address.
///
/// The runtime's scalar-to-string conversions take the scalar itself, so the
/// pointer has to be loaded before the call. The example corpus caught this
/// after the unit tests above had all passed — worth keeping as a fixture.
#[test]
fn test_narrowed_optional_concatenates_its_value_not_its_address() {
    assert_prints(
        "تضييق_دمج",
        "متغير س: عدد? = 0\nإذا (س != لا_شيء) { اطبع(\"القيمة \" + س) }",
        &["القيمة 0"],
    );
}

/// A narrowed `نص?` is still a `TrqString*`, so it has to take the string path
/// rather than falling back to the array one and counting bytes again.
#[test]
fn test_narrowed_optional_string_measures_characters() {
    assert_prints(
        "تضييق_نص",
        "متغير س: نص? = \"مرحبا\"\nإذا (س != لا_شيء) { اطبع(طول(س)) }",
        &["5"],
    );
}

/// A float reads the same whether printed or concatenated.
///
/// These disagreed: `اطبع(5.0)` gave `5.0` while `اطبع("" + 5.0)` gave `5`,
/// because concatenation lowers through `trq_float_to_string` and that dropped
/// the fraction. Every backend agreed on the wrong answer, so no cross-backend
/// check could see it — only reading an example's output did.
#[test]
fn test_float_reads_the_same_printed_and_concatenated() {
    assert_prints(
        "عشري_دمج",
        "اطبع(5.0)\nاطبع(\"القيمة: \" + 5.0)\nاطبع(\"كسر: \" + 2.5)",
        &["5.0", "القيمة: 5.0", "كسر: 2.5"],
    );
}

#[test]
fn test_array_push_builtin_works_in_every_backend() {
    assert_prints(
        "الحق_مباشر",
        "متغير م = [1]\nالحق(م، 2)\nاطبع(طول_مصفوفة(م))",
        &["2"],
    );
}

// ---------------------------------------------------------------------------
// Shadowing — an imported name resolves to the import, in every backend
// ---------------------------------------------------------------------------

/// A same-named imported function wins over a core builtin, consistently.
///
/// This asserted the opposite until #262 was decided: imported names are tier 4
/// and builtins tier 5, so the import shadows. What has not changed is the part
/// that matters most — all three backends must answer the same way. The
/// interpreter consults `is_builtin` before user functions, so a lowering that
/// let only native bind the user's function would print a different answer
/// natively than interpreted, which is how the first attempt at shadowing was
/// caught and reverted.
#[test]
fn test_an_import_shadows_a_same_named_builtin_in_every_backend() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    write_program(
        dir,
        "وحدة",
        "صدّر دالة نوع(س: عدد) -> نص {\n    أرجع \"خاصتي\"\n}",
    );
    let main = write_program(
        dir,
        "رئيسي_تظليل",
        "استورد { نوع } من \"./وحدة\"\nاطبع(نوع(5))",
    );

    for backend in Backend::ALL {
        let output = execute(backend, &main, &format!("تظليل_{backend:?}"));
        assert!(
            output.succeeded(),
            "فشل التنفيذ [{backend:?}]\n{}",
            output.report()
        );
        assert_eq!(
            output.lines(),
            ["خاصتي"],
            "الدالة المستوردة يجب أن تظلّل المدمجة في كل الخلفيات [{backend:?}]\n{}",
            output.report()
        );
    }
}

// ---------------------------------------------------------------------------
// A user function shadows a same-named builtin (#262, and #257 as a side effect)
// ---------------------------------------------------------------------------

/// A user's function wins over a same-named runtime builtin, in every backend.
///
/// `مطلق` is registered only on `استورد … من "رياضيات"`, so the semantic layer
/// always accepted the declaration — and codegen then applied
/// `get_runtime_function_name` to the *definition* as well as the call,
/// emitting `define @trq_abs_float` into a module that already carried
/// `declare @trq_abs_float`. LLVM rejected that pair while parsing the IR, so
/// the user saw a clang error naming a C symbol they never wrote (#257). Under
/// the shadowing rule the definition keeps its own mangled symbol, so the
/// collision cannot arise.
///
/// The body returns a constant rather than an absolute value on purpose. #257
/// reported this program as working under `run` because `مطلق(-٥)` and `|−5|`
/// both give `5` — a fixture that cannot tell which function ran. `٩٩٩` can.
#[test]
fn test_a_user_function_shadows_a_same_named_builtin_in_every_backend() {
    assert_prints(
        "تظليل_مطلق",
        "دالة مطلق(س: عدد) -> عدد {\n    أرجع ٩٩٩\n}\nاطبع(مطلق(-٥))",
        &["999"],
    );
}

/// The same shape on a second name, so the rule cannot pass by way of one lucky
/// entry in a 197-name table.
#[test]
fn test_a_second_shadowed_builtin_name_behaves_the_same() {
    assert_prints(
        "تظليل_قاسم",
        "دالة قاسم_مشترك(أ: عدد، ب: عدد) -> عدد {\n    أرجع ٧٧٧\n}\nاطبع(قاسم_مشترك(١٢، ١٨))",
        &["777"],
    );
}

/// A *core* builtin — one pre-registered into the global scope — is shadowable
/// too.
///
/// This is the half that no backend change alone can deliver: `دالة طول(…)` was
/// a hard `د٠١٠١ معرّف مسبقاً` at hoist time (#262), because
/// `register_core_builtins` defines the 18 core names as ordinary global
/// symbols and `Scope::define` refuses a duplicate. So this fixture proves the
/// semantic half landed, and the differing signature (`نص` in, not `أي`) proves
/// the call type-checks against the user's declaration.
#[test]
fn test_a_core_builtin_is_shadowable_in_every_backend() {
    assert_prints(
        "تظليل_طول",
        "دالة طول(س: نص) -> عدد {\n    أرجع ٤٢\n}\nاطبع(طول(\"مرحبا\"))",
        &["42"],
    );
}

/// Shadowing reached through an import — tier 4 beating tier 5.
///
/// The declaration lives in a separate file, so the name arrives through
/// `semantic::linker`'s merge rather than from main's own AST. Before this
/// change the import `define` was silently dropped and the builtin kept
/// winning.
#[test]
fn test_an_imported_function_shadows_a_builtin_in_every_backend() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    write_program(
        dir,
        "وحدة_تظليل",
        "صدّر دالة قاسم_مشترك(أ: عدد، ب: عدد) -> عدد {\n    أرجع ٧٧٧\n}",
    );
    let main = write_program(
        dir,
        "رئيسي_تظليل_مستورد",
        "استورد { قاسم_مشترك } من \"./وحدة_تظليل\"\nاطبع(قاسم_مشترك(١٢، ١٨))",
    );

    for backend in Backend::ALL {
        let output = execute(backend, &main, &format!("تظليل_مستورد_{backend:?}"));
        assert!(
            !output.compile_failed,
            "توقّعنا ترجمة ناجحة [{backend:?}]\n{}",
            output.report()
        );
        assert!(
            output.succeeded(),
            "فشل التنفيذ [{backend:?}]\n{}",
            output.report()
        );
        assert_eq!(
            output.lines(),
            ["777"],
            "الدالة المستوردة يجب أن تظلّل المدمجة في كل الخلفيات [{backend:?}]\n{}",
            output.report()
        );
    }
}

/// A module declaration the program never imported must NOT shadow a builtin.
///
/// The linker merges every declaration of an imported module into one flat
/// namespace under its bare name, so the IR builder sees `اطبع` as declared
/// even though `analyze` — which only registers *imported* names — bound the
/// call to the builtin. Backends that re-derived "is this declared?" from the
/// linked AST therefore called a function the type-checker never checked the
/// arguments against: `اطبع(٥)` passed an i64 into a `نص` parameter, printing
/// the wrong thing interpreted and segfaulting natively.
///
/// This is the shape `stdlib/طرفية` has — it exports `اطبع`, `ادخل` and
/// `ادخل_رسالة`, all core builtins — so importing anything at all from it hit
/// this.
#[test]
fn test_an_unimported_module_declaration_does_not_shadow_a_builtin() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    write_program(
        dir,
        "وحدة_غير_مستوردة",
        "صدّر دالة شيء() -> عدد {\n    أرجع ١\n}\n\
         صدّر دالة اطبع(س: نص) {\n    طباعة(\"[وحدة] \")\n    طباعة(س)\n}",
    );
    let main = write_program(
        dir,
        "رئيسي_غير_مستورد",
        "استورد { شيء } من \"./وحدة_غير_مستوردة\"\nاطبع(شيء())",
    );

    for backend in Backend::ALL {
        let output = execute(backend, &main, &format!("غير_مستورد_{backend:?}"));
        assert!(
            !output.compile_failed,
            "توقّعنا ترجمة ناجحة [{backend:?}]\n{}",
            output.report()
        );
        assert!(
            output.succeeded(),
            "فشل التنفيذ [{backend:?}] — تعطّل محتمل\n{}",
            output.report()
        );
        assert_eq!(
            output.lines(),
            ["1"],
            "دالة وحدة لم تُستورَد يجب ألا تظلّل المدمجة [{backend:?}]\n{}",
            output.report()
        );
    }
}

// ---------------------------------------------------------------------------
// The sweep — every core builtin, so a new one cannot skip the matrix
// ---------------------------------------------------------------------------

/// One probe per core builtin, driven by `Scope::core_builtin_names()` so the
/// list cannot drift away from what the semantic layer registers.
///
/// A static check would not work here: the conversion builtins are lowered by
/// IR-builder special cases, invisible to `get_runtime_function_name`. Only
/// running the program proves a builtin reaches a real implementation.
#[test]
fn test_every_core_builtin_agrees_across_backends() {
    // Three names are deliberately outside this sweep, which compares stdout of
    // a program that runs to completion: `ادخل`/`ادخل_رسالة` block on stdin, and
    // `اطبع_خطأ` disagrees about the *stream* — the interpreter prints it to
    // stdout, native to stderr — which this harness cannot express. They are
    // *not* covered elsewhere — stated plainly rather than implied away, since a
    // guard test that overstates its reach is how the next drift hides. `توقف`
    // terminates, so it gets an exit-code fixture below instead.
    let probes: &[(&str, &str, &[&str])] = &[
        ("اطبع", "اطبع(1)", &["1"]),
        ("طباعة", "طباعة(1)", &["1"]),
        // Two calls, not one: `lines()` trims, so a single call cannot tell a
        // trailing newline from its absence — which is how native `اطبع_سطر`
        // printing through the newline-less `trq_print` stayed invisible.
        ("اطبع_سطر", "اطبع_سطر(\"أ\")\nاطبع_سطر(\"ب\")", &["أ", "ب"]),
        // A *string*, deliberately: this probe used an array while native `طول`
        // counted UTF-8 bytes and the interpreter counted characters (#185).
        ("طول", "اطبع(طول(\"مرحبا\"))", &["5"]),
        ("نوع", "اطبع(نوع(1))", &["عدد"]),
        ("عدد", "اطبع(عدد(\"5\"))", &["5"]),
        ("عدد_عشري", "اطبع(عدد_عشري(\"5.5\"))", &["5.5"]),
        ("نص", "اطبع(نص(5))", &["5"]),
        ("منطقي", "اطبع(منطقي(1))", &["صحيح"]),
        ("تأكد", "تأكد(صحيح)\nاطبع(\"تم\")", &["تم"]),
        (
            "تأكد_رسالة",
            "تأكد_رسالة(صحيح، \"س\")\nاطبع(\"تم\")",
            &["تم"],
        ),
        ("نم", "نم(0)\nاطبع(\"تم\")", &["تم"]),
        ("طول_مصفوفة", "اطبع(طول_مصفوفة([1، 2]))", &["2"]),
        (
            "الحق",
            "متغير م = [1]\nالحق(م، 2)\nاطبع(طول_مصفوفة(م))",
            &["2"],
        ),
        ("بتات_و", "اطبع(بتات_و(12، 10))", &["8"]),
        ("بتات_أو", "اطبع(بتات_أو(12، 10))", &["14"]),
        ("بتات_أو_حصري", "اطبع(بتات_أو_حصري(12، 10))", &["6"]),
        ("بتات_نفي", "اطبع(بتات_نفي(255))", &["-256"]),
        ("بتات_إزاحة_يسار", "اطبع(بتات_إزاحة_يسار(3، 4))", &["48"]),
        // Negative, so the sweep's one line already distinguishes the
        // arithmetic shift from a logical one.
        ("بتات_إزاحة_يمين", "اطبع(بتات_إزاحة_يمين(-48، 4))", &["-3"]),
    ];

    let covered: Vec<&str> = probes.iter().map(|(name, _, _)| *name).collect();
    let uncovered: Vec<&str> = tarqeem::semantic::Scope::core_builtin_names()
        .iter()
        .copied()
        .filter(|name| !covered.contains(name))
        .filter(|name| !matches!(*name, "ادخل" | "ادخل_رسالة" | "اطبع_خطأ" | "توقف"))
        .collect();
    assert!(
        uncovered.is_empty(),
        "دوال أساسية بلا تغطية تنفيذية: {uncovered:?}"
    );

    for (name, body, expected) in probes {
        assert_prints(&format!("أساسي_{name}"), body, expected);
    }
}

/// `توقف` aborts the program, so it is asserted on exit status rather than
/// stdout — the sweep above only covers builtins that run to completion.
#[test]
fn test_halt_builtin_aborts_in_every_backend() {
    assert_fails("توقف_مباشر", "توقف(\"انتهى\")\nاطبع(\"لا ينبغي طباعته\")");
}

/// The two `وقت` builtins read a clock, so their value cannot be pinned the way
/// `assert_prints` requires. Assert the shape instead: one line holding a
/// positive integer of epoch-millisecond magnitude.
///
/// Both were declared by codegen with nothing defining them, so every program
/// importing either failed to link (#241). The range below is what makes this a
/// real assertion — it rejects the `0` a stubbed call yields and the huge value
/// a pointer printed as an integer would give, which is how the missing IR
/// return type would have surfaced.
///
/// Two backends, not four, and neither omission is an oversight:
/// - **JIT** stubs every call to the constant `0` (#262), so it would assert
///   nothing here.
/// - **`tarqeem debug`** has no leg in this harness; it launches a DAP server
///   rather than running to completion. The debug interpreter's own copy of
///   these builtins is covered by a unit test beside it.
#[test]
fn test_time_builtins_read_a_real_clock_in_interpreter_and_native() {
    // Wide enough never to expire in practice (through the year 5138), tight
    // enough that a pointer, a zero, or a byte count all fall outside.
    const PLAUSIBLE_MILLIS: std::ops::Range<i64> = 1_000_000_000_000..100_000_000_000_000;

    for name in ["وقت_الآن", "وقت_أداء"] {
        let temp = TempDir::new().unwrap();
        let main = write_program(
            temp.path(),
            "ساعة",
            &format!("استورد {{ {name} }} من \"وقت\"\nاطبع({name}())"),
        );

        for backend in [Backend::Interpreter, Backend::Native] {
            let output = execute(backend, &main, &format!("ساعة_{backend:?}"));

            assert!(
                output.succeeded(),
                "فشل تنفيذ {name} [{backend:?}]\n{}",
                output.report()
            );

            let lines = output.lines();
            assert_eq!(
                lines.len(),
                1,
                "توقّعنا سطراً واحداً من {name} [{backend:?}]\n{}",
                output.report()
            );

            let millis: i64 = lines[0].parse().unwrap_or_else(|_| {
                panic!(
                    "لم يُرجع {name} عدداً [{backend:?}]: {:?}\n{}",
                    lines[0],
                    output.report()
                )
            });
            assert!(
                PLAUSIBLE_MILLIS.contains(&millis),
                "{name} أرجع {millis} [{backend:?}]، وهو ليس توقيتاً بالميلي ثانية\n{}",
                output.report()
            );
        }
    }
}
