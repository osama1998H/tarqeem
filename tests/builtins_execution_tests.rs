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

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
    tarqeem_with_env(args, cwd, &[])
}

/// `tarqeem` with extra variables in the child's environment.
///
/// `متغير_بيئة` is the first builtin whose answer depends on the environment, and
/// it cannot be tested by setting one here: cargo runs tests as threads in one
/// process, so `std::env::set_var` races every other test. Every backend leg is
/// already a child process, so the variable goes on the child instead.
fn tarqeem_with_env(args: &[&str], cwd: &Path, env: &[(&str, &str)]) -> Output {
    tarqeem_with_env_and_stdin(args, cwd, env, None)
}

/// The one place a `tarqeem` child is spawned.
///
/// `اقرأ_مجرى` is the first builtin whose answer depends on stdin, and — exactly
/// as with the environment above — a test cannot supply it in-process: cargo runs
/// tests as threads in one process, so there is no per-test stdin to redirect.
/// Every backend leg is already a child process, so the bytes go on the child.
/// `&[u8]` rather than `&str` on purpose: the primitive answers bytes, and one of
/// its contract rows is a byte sequence that is not text at all.
///
/// `stdin` of `None` keeps `Command::output`'s default, which is a **null** stdin
/// — an immediate EOF, not the parent's terminal. That is what lets the EOF row
/// of `اقرأ_مجرى`'s contract be asserted through the plain `assert_prints`.
fn tarqeem_with_env_and_stdin(
    args: &[&str],
    cwd: &Path,
    env: &[(&str, &str)],
    stdin: Option<&[u8]>,
) -> Output {
    let mut command = Command::new(TARQEEM);
    command.args(args).current_dir(cwd);

    // A stale `TARQEEM_HOME` silently shadows this checkout's `stdlib`, so it is
    // scrubbed on every backend. The runtime archive no longer needs it either:
    // the compiler finds the one built beside its own executable.
    command.env_remove("TARQEEM_HOME");
    command.envs(env.iter().copied());

    let output = match stdin {
        None => command
            .output()
            .unwrap_or_else(|e| panic!("تعذّر تشغيل {} {:?}: {}", TARQEEM, args, e)),
        Some(bytes) => spawn_with_stdin(&mut command, bytes, &format!("{TARQEEM} {args:?}")),
    };

    Output {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        compile_failed: false,
    }
}

/// Spawns `command` with `bytes` piped to its stdin, then closes the pipe.
///
/// Closing it is what produces EOF, so a program asking for more bytes than were
/// given gets a short answer instead of hanging. The payloads here are a handful
/// of bytes, well inside a pipe buffer, so writing before waiting cannot deadlock.
fn spawn_with_stdin(command: &mut Command, bytes: &[u8], what: &str) -> std::process::Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("تعذّر تشغيل {what}: {e}"));

    {
        let mut pipe = child.stdin.take().expect("للعملية مدخل قياسي");
        pipe.write_all(bytes)
            .unwrap_or_else(|e| panic!("تعذّر الكتابة في مدخل {what}: {e}"));
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("تعذّر انتظار {what}: {e}"))
}

fn execute(backend: Backend, main: &Path, tag: &str) -> Output {
    execute_with_env(backend, main, tag, &[])
}

/// `execute` with extra variables in the child's environment.
///
/// The native leg puts them on the **executed binary**, not only on `compile`:
/// the compiler reads no environment on that path, and the binary is what calls
/// `trq_env_get`.
fn execute_with_env(backend: Backend, main: &Path, tag: &str, env: &[(&str, &str)]) -> Output {
    execute_with_env_and_stdin(backend, main, tag, env, None)
}

/// `execute` with bytes on the child's standard input.
///
/// The native leg pipes them to the **executed binary**, never to `compile` — the
/// same split `execute_with_env` makes for the environment, and for the same
/// reason: the compiler reads neither on that path.
fn execute_with_stdin(backend: Backend, main: &Path, tag: &str, stdin: &[u8]) -> Output {
    execute_with_env_and_stdin(backend, main, tag, &[], Some(stdin))
}

/// `execute` with arguments for the **program**, not for `tarqeem`.
///
/// `معاملات_البرنامج` is the third builtin whose answer comes from outside the
/// source, after the environment (#338) and stdin (#350), and it needs the same
/// treatment for the same reason: a test cannot set its own process's argv any
/// more than it can `set_var`. The arguments go on the child.
///
/// The split the other two make applies here too, and is easy to get backwards:
/// on the native leg they belong to the **executed binary**, never to `compile`.
/// The interpreter and JIT legs take them after the file name, where clap's
/// `trailing_var_arg` collects them.
fn execute_with_args(backend: Backend, main: &Path, tag: &str, args: &[&str]) -> Output {
    execute_all(backend, main, tag, &[], None, args)
}

fn execute_with_env_and_stdin(
    backend: Backend,
    main: &Path,
    tag: &str,
    env: &[(&str, &str)],
    stdin: Option<&[u8]>,
) -> Output {
    execute_all(backend, main, tag, env, stdin, &[])
}

fn execute_all(
    backend: Backend,
    main: &Path,
    tag: &str,
    env: &[(&str, &str)],
    stdin: Option<&[u8]>,
    prog_args: &[&str],
) -> Output {
    let cwd = main.parent().expect("للبرنامج مجلد");
    let arg = main.to_str().expect("مسار غير صالح");

    // The program's arguments follow the file name, where clap's
    // `trailing_var_arg` collects them.
    fn cli_line<'a>(head: &[&'a str], prog_args: &[&'a str]) -> Vec<&'a str> {
        let mut all = head.to_vec();
        all.extend_from_slice(prog_args);
        all
    }

    match backend {
        Backend::Interpreter => {
            tarqeem_with_env_and_stdin(&cli_line(&["run", arg], prog_args), cwd, env, stdin)
        }
        Backend::Jit => tarqeem_with_env_and_stdin(
            &cli_line(&["run", "--jit", arg], prog_args),
            cwd,
            env,
            stdin,
        ),
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

            let mut runner = Command::new(&exe);
            runner.envs(env.iter().copied());
            runner.args(prog_args);
            let run = match stdin {
                None => runner
                    .output()
                    .unwrap_or_else(|e| panic!("تعذّر تشغيل {}: {}", exe.display(), e)),
                Some(bytes) => spawn_with_stdin(&mut runner, bytes, &exe.display().to_string()),
            };
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
    assert_prints_with_env(name, body, &[], expected)
}

/// Asserts `body` prints exactly `expected` under all three backends, with
/// `stdin` piped to each.
fn assert_prints_with_stdin(name: &str, body: &str, stdin: &[u8], expected: &[&str]) {
    let temp = TempDir::new().unwrap();
    let main = write_program(temp.path(), name, body);

    for backend in Backend::ALL {
        let output = execute_with_stdin(backend, &main, &format!("{name}_{backend:?}"), stdin);

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

/// What `assert_prints_with_tree` puts on disk for one fixture name.
///
/// `Symlink`'s target is another name in the same directory and **need not
/// exist** — a dangling link is a contract row for `احذف_مسار`, not a mistake.
/// A fixture that is the target of a later `Symlink` must come before it.
#[derive(Clone, Copy)]
enum Fixture<'a> {
    File(&'a str),
    EmptyDir,
    #[cfg(unix)]
    Symlink {
        to: &'a str,
    },
}

/// Builds the fixture tree from scratch, removing whatever the previous backend
/// leg left behind.
///
/// `remove_file` comes first because it is what unlinks a **symlink**;
/// `remove_dir_all` refuses one rather than following it.
fn materialize(dir: &Path, fixtures: &[(&str, Fixture)]) {
    for (fixture_name, _) in fixtures {
        let path = dir.join(fixture_name);
        if fs::remove_file(&path).is_err() {
            let _ = fs::remove_dir_all(&path);
        }
    }

    for (fixture_name, fixture) in fixtures {
        let path = dir.join(fixture_name);
        match fixture {
            Fixture::File(contents) => fs::write(&path, contents).expect("تعذّر كتابة ملف التجربة"),
            Fixture::EmptyDir => fs::create_dir(&path).expect("تعذّر إنشاء مجلد التجربة"),
            #[cfg(unix)]
            Fixture::Symlink { to } => {
                std::os::unix::fs::symlink(dir.join(to), &path).expect("تعذّر إنشاء وصلة التجربة")
            }
        }
    }
}

/// `assert_prints` with a fixture tree on disk, whose **absolute** paths are
/// substituted into `body`.
///
/// Each fixture is created inside the same `TempDir` the program lives in, and
/// every `{مسار}` in `body` becomes the absolute path of the first fixture,
/// `{مسار2}` the second, and so on.
///
/// Absolute, not relative: the native leg runs the compiled binary directly and
/// inherits no working directory from the source's location, so a relative
/// fixture name would resolve against wherever `cargo test` was invoked and the
/// three backends would disagree. That is the same lesson `_with_env` and
/// `_with_stdin` learned in their own currency — the fixture goes where the
/// child can reach it, not where the harness happens to stand.
///
/// A second fixture is `{مسار2}`, a third `{مسار3}`. Latin digits, because the
/// Tarqeem programs in this file use Latin digits throughout — the Arabic-Indic
/// ones here are all prose. `examples/` is where the other convention lives.
///
/// **The tree is rebuilt inside the backend loop, once per leg.** It used to be
/// built once, before the loop, which is invisible for a primitive that only
/// *reads* — and wrong for one that deletes: the interpreter leg would consume
/// the fixture and the JIT and native legs would then run against an absent
/// path, failing however correct the implementation was.
///
/// Additive, like `_with_env` and `_with_stdin`: `assert_prints_with_files` is
/// one line over this, so its existing callers are untouched — rewriting the
/// same file contents per leg is idempotent.
fn assert_prints_with_tree(
    name: &str,
    fixtures: &[(&str, Fixture)],
    body: &str,
    expected: &[&str],
) {
    assert_prints_with_tree_and_contents(name, fixtures, body, expected, &[])
}

/// `assert_prints_with_tree`, plus what the tree must **contain** once the
/// program has ended.
///
/// The sixth helper a primitive's contract has forced — env on the child (#338),
/// stdin on the child (#350), fixture files (#352), a tree restored per leg
/// (#355), arguments on the child (#360), and a file read back here.
///
/// `افتح_ملف` needs it because its durability row is invisible from inside the
/// program: bytes written to a handle sit in a `BufWriter` until program end, so
/// nothing the program can print distinguishes "flushed at exit" from "lost". The
/// check runs **inside** the backend loop, after the tree the leg rebuilt, so a
/// backend that drops the bytes fails on its own leg rather than on the next
/// one's fixture.
fn assert_prints_with_tree_and_contents(
    name: &str,
    fixtures: &[(&str, Fixture)],
    body: &str,
    expected: &[&str],
    after: &[(&str, &str)],
) {
    let temp = TempDir::new().unwrap();

    let mut resolved = body.to_string();
    for (index, (fixture_name, _)) in fixtures.iter().enumerate() {
        let path = temp.path().join(fixture_name);
        let placeholder = match index {
            0 => "{مسار}".to_string(),
            other => format!("{{مسار{}}}", other + 1),
        };
        resolved = resolved.replace(&placeholder, path.to_str().expect("مسار غير صالح"));
    }

    // An unsubstituted placeholder is silent otherwise: it reaches the program as
    // a literal path, which every path primitive reads as *absent* — so a row
    // asserting the absent answer would pass while testing nothing. Fail here,
    // where the mismatch is, rather than in an assertion that cannot see it.
    assert!(
        !resolved.contains("{مسار"),
        "بقي موضع مسار بلا استبدال في {name} / an unsubstituted path placeholder remains — \
         the fixture list has {} entries, so the placeholders are {{مسار}} then {{مسار2}}, …",
        fixtures.len()
    );

    let main = write_program(temp.path(), name, &resolved);

    for backend in Backend::ALL {
        materialize(temp.path(), fixtures);

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

        for (fixture_name, contents) in after {
            let path = temp.path().join(fixture_name);
            let found = fs::read(&path).unwrap_or_else(|error| {
                panic!("تعذّر قراءة {fixture_name} بعد التنفيذ [{backend:?}] لـ {name}: {error}")
            });
            assert_eq!(
                String::from_utf8_lossy(&found),
                *contents,
                "محتوى {fixture_name} بعد التنفيذ [{backend:?}] لـ {name}\n{}",
                output.report()
            );
        }
    }
}

/// `assert_prints_with_tree` where every fixture is a plain file.
fn assert_prints_with_files(name: &str, files: &[(&str, &str)], body: &str, expected: &[&str]) {
    let fixtures: Vec<(&str, Fixture)> = files
        .iter()
        .map(|(file_name, contents)| (*file_name, Fixture::File(contents)))
        .collect();

    assert_prints_with_tree(name, &fixtures, body, expected)
}

/// `assert_prints` with extra variables in each backend's environment.
fn assert_prints_with_env(name: &str, body: &str, env: &[(&str, &str)], expected: &[&str]) {
    let temp = TempDir::new().unwrap();
    let main = write_program(temp.path(), name, body);

    for backend in Backend::ALL {
        let output = execute_with_env(backend, &main, &format!("{name}_{backend:?}"), env);

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

/// Asserts `body` prints exactly `expected` under all three backends, with
/// `args` handed to the **program** rather than to `tarqeem`.
fn assert_prints_with_args(name: &str, body: &str, args: &[&str], expected: &[&str]) {
    let temp = TempDir::new().unwrap();
    let main = write_program(temp.path(), name, body);

    for backend in Backend::ALL {
        let output = execute_with_args(backend, &main, &format!("{name}_{backend:?}"), args);

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

/// Asserts `body` exits with `status` under all three backends, having printed
/// exactly `expected` and nothing on stderr.
///
/// `assert_prints` cannot express this — it demands status 0 — and `assert_fails`
/// cannot either: it demands *some* non-zero status and empty stdout, so it
/// passes for the wrong reasons on a program that deliberately chooses its own
/// status after printing.
///
/// The `compile_failed` guard is the load-bearing part. A native compile failure
/// exits non-zero with empty stdout, which is indistinguishable from
/// `أنهِ_البرنامج(1)` on status alone, so without it a lowering regressing into
/// an LLVM parse error would keep every non-zero case below green.
///
/// stderr is asserted empty because that is where the divergence would hide: the
/// native binary prints nothing, and an interpreter that reported the exit as a
/// runtime error would still satisfy both the status and stdout checks — and
/// `compare-backends` in CI diffs stdout only.
fn assert_exits_with(name: &str, body: &str, status: i32, expected: &[&str]) {
    let temp = TempDir::new().unwrap();
    let main = write_program(temp.path(), name, body);

    for backend in Backend::ALL {
        let output = execute(backend, &main, &format!("{name}_{backend:?}"));

        assert!(
            !output.compile_failed,
            "توقّعنا خروجاً وقت التنفيذ لا فشلاً وقت الترجمة [{backend:?}] لـ {name}\n{}",
            output.report()
        );
        assert_eq!(
            output.status,
            Some(status),
            "حالة خروج غير متوقعة [{backend:?}] لـ {name}\n{}",
            output.report()
        );
        assert_eq!(
            output.lines(),
            expected,
            "خرج غير متطابق [{backend:?}] لـ {name}\n{}",
            output.report()
        );
        assert!(
            output.stderr.is_empty(),
            "توقّعنا ألا يُطبع شيء على stderr [{backend:?}] لـ {name}\n{}",
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
// بتات_إزاحة_يمين_منطقية — the logical right shift (#322)
// ---------------------------------------------------------------------------

#[test]
fn test_logical_right_shift_moves_bits_in_every_backend() {
    assert_prints(
        "إزاحة_منطقية_ضم",
        concat!(
            "اطبع(بتات_إزاحة_يمين_منطقية(8، 1))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(1، 0))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(1، 1))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(1024، 10))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(48، 4))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(0، 5))",
        ),
        &["4", "1", "0", "1", "3", "0"],
    );
}

/// The one property this name exists for, and the only fixture that can catch a
/// lowering that lost the sign separation and became the arithmetic shift: every
/// line here would print a small negative number instead, all of them plausible.
///
/// The values are the operand read as an *unsigned* 64-bit word divided by the
/// power of two, computed by hand — `(٢**٦٤ - ١٦) / ٢**٤` for the fourth.
#[test]
fn test_logical_right_shift_fills_with_zeros() {
    assert_prints(
        "إزاحة_منطقية_أصفار",
        concat!(
            "اطبع(بتات_إزاحة_يمين_منطقية(-1، 1))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-1، 32))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-1، 63))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-16، 4))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-8، 1))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(بتات_إزاحة_يسار(1، 63)، 1))",
        ),
        &[
            "9223372036854775807",
            "4294967295",
            "1",
            "1152921504606846975",
            "9223372036854775804",
            "4611686018427387904",
        ],
    );
}

/// Against its sibling, in both directions. Agreement on a non-negative operand
/// is as load-bearing as the disagreement on a negative one: a lowering that
/// simply *was* the arithmetic shift would satisfy the first two lines, and one
/// that mangled the low bits would satisfy the last three.
///
/// The final pair is where the two converge again — out of range the arithmetic
/// shift keeps the sign and this one does not, which is the same family rule
/// producing two different numbers.
#[test]
fn test_logical_right_shift_diverges_from_the_arithmetic_one() {
    assert_prints(
        "إزاحة_منطقية_مقابل_حسابية",
        concat!(
            "اطبع(بتات_إزاحة_يمين_منطقية(48، 4) == بتات_إزاحة_يمين(48، 4))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-1، 0) == بتات_إزاحة_يمين(-1، 0))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-16، 4) == بتات_إزاحة_يمين(-16، 4))\n",
            "اطبع(بتات_إزاحة_يمين(-16، 4))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-16، 4))\n",
            "اطبع(بتات_إزاحة_يمين(-1، 64))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-1، 64))",
        ),
        &[
            "صحيح",
            "صحيح",
            "خطأ",
            "-1",
            "1152921504606846975",
            "-1",
            "0",
        ],
    );
}

/// A logical shift is *unsigned* division by a power of two, so it agrees with
/// `/` exactly where the operand is non-negative and parts company by the full
/// width of the word where it is not. Pinning both halves is what stops a later
/// "simplification" into a division, the same way the arithmetic shift's
/// floor-versus-truncation fixture does.
#[test]
fn test_logical_right_shift_is_unsigned_division() {
    assert_prints(
        "إزاحة_منطقية_قسمة",
        concat!(
            "اطبع(بتات_إزاحة_يمين_منطقية(1024، 10) == 1024 / 2 ** 10)\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(255، 4) == 255 / 2 ** 4)\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-1، 1) == -1 / 2)\n",
            "اطبع(-1 / 2)\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-1، 1))",
        ),
        &["صحيح", "صحيح", "خطأ", "0", "9223372036854775807"],
    );
}

/// The contract, reached by the family rule rather than inherited from
/// `بتات_إزاحة_يسار`: an amount outside 0-63 is a complete shift, and this shift
/// always fills with zeros, so the answer is `٠` for a negative operand too —
/// where its sibling answers `-١`. A negative amount is out of range like any
/// other, not a shift in the opposite direction.
#[test]
fn test_logical_right_shift_is_total_outside_the_valid_range() {
    assert_prints(
        "إزاحة_منطقية_خارج_النطاق",
        concat!(
            "اطبع(بتات_إزاحة_يمين_منطقية(-1، 64))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-1، 65))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-1، 1000))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-1، -1))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(255، 64))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(255، -1))",
        ),
        &["0", "0", "0", "0", "0", "0"],
    );
}

/// The literals above are folded away before native codegen ever sees a shift,
/// so they exercise the constant folder and nothing else. A variable amount is
/// the other half of the contract — it is the only path that reaches LLVM's
/// `ashr i64` and `shl i64`, whose out-of-range results are poison.
#[test]
fn test_logical_right_shift_guards_a_runtime_amount_too() {
    assert_prints(
        "إزاحة_منطقية_مقدار_متغير",
        concat!(
            "متغير مقدار = 4\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-16، مقدار))\n",
            "مقدار = 0\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-16، مقدار))\n",
            "مقدار = 63\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-16، مقدار))\n",
            "مقدار = 64\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-16، مقدار))\n",
            "مقدار = -1\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-16، مقدار))\n",
            "دالة أزح_منطقياً(قيمة: عدد، ن: عدد) -> عدد {\n",
            "    أرجع بتات_إزاحة_يمين_منطقية(قيمة، ن)\n",
            "}\n",
            "اطبع(أزح_منطقياً(-1، 1))\n",
            "اطبع(أزح_منطقياً(-1، 99))\n",
            "اطبع(أزح_منطقياً(64، 3))",
        ),
        &[
            "1152921504606846975",
            "-16",
            "1",
            "0",
            "0",
            "9223372036854775807",
            "0",
            "8",
        ],
    );
}

/// The most extreme amount representable, reached by shifting rather than
/// written as a literal — negating `9223372036854775808` would not fit an `عدد`.
/// It is the one input where the guard's `٠ - (ن >> ٦)` could overflow if the
/// chain subtracted the amount itself instead of its shifted quotient.
///
/// The last three use it as the *value*, which is this lowering's tightest spot:
/// clearing the sign bit leaves exactly zero, so the whole answer comes from the
/// sign term. At amount `٠` that term is `١ << ٦٣`, which must reproduce the
/// operand rather than saturate or drop.
#[test]
fn test_logical_right_shift_handles_the_most_negative_amount() {
    assert_prints(
        "إزاحة_منطقية_أصغر_مقدار",
        concat!(
            "ثابت الأصغر = بتات_إزاحة_يسار(1، 63)\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(-1، الأصغر))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(الأصغر، 0))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(الأصغر، 1))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(الأصغر، 63))",
        ),
        &["0", "-9223372036854775808", "4611686018427387904", "1"],
    );
}

/// The chain ends in a `BitOr`, so its destination is what types the call.
/// Printing alone would not tell a real `عدد` from the `Ptr(Void)` sentinel a
/// missing type registration leaves behind.
#[test]
fn test_logical_right_shift_result_composes_as_an_integer() {
    assert_prints(
        "إزاحة_منطقية_تركيب",
        concat!(
            "اطبع(نوع(بتات_إزاحة_يمين_منطقية(8، 1)))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(8، 1) + 1)\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(8، 1) == 4)\n",
            "اطبع(بتات_و(بتات_إزاحة_يمين_منطقية(-1، 4)، 255))\n",
            "دالة ضاعف(ن: عدد) -> عدد {\n    أرجع ن * 2\n}\n",
            "اطبع(ضاعف(بتات_إزاحة_يمين_منطقية(8، 1)))",
        ),
        &["عدد", "5", "صحيح", "255", "8"],
    );
}

/// Once `بتات_إزاحة_يمين` landed (#320) this operation became reachable by
/// composing the six existing names, so §1.3's criterion-(a) justification for
/// it has expired — the case is call-site readability and registry completeness,
/// as with `بتات_نفي` (#312).
///
/// This asserts the equivalence rather than leaving it as prose, in both
/// directions that matter: the sign bit at rest (`ن == ٠`), in flight, and out
/// of range, where the composition's two inner guards happen to produce the same
/// answer the single guard does. It is not a tautology — the composition emits
/// two range guards and five calls where the lowering emits one guard and no
/// call.
#[test]
fn test_logical_right_shift_matches_the_composition_it_names() {
    assert_prints(
        "إزاحة_منطقية_مركبة",
        concat!(
            "دالة يميناً_منطقية(قيمة: عدد، ن: عدد) -> عدد {\n",
            "    متغير بلا_إشارة = بتات_إزاحة_يمين(بتات_و(قيمة، 9223372036854775807)، ن)\n",
            "    متغير موضع = بتات_إزاحة_يسار(1، 63 - ن)\n",
            "    أرجع بتات_أو(بلا_إشارة، بتات_و(بتات_إزاحة_يمين(قيمة، 63)، موضع))\n",
            "}\n",
            "اطبع(يميناً_منطقية(-1، 0) == بتات_إزاحة_يمين_منطقية(-1، 0))\n",
            "اطبع(يميناً_منطقية(-1، 1) == بتات_إزاحة_يمين_منطقية(-1، 1))\n",
            "اطبع(يميناً_منطقية(-16، 4) == بتات_إزاحة_يمين_منطقية(-16، 4))\n",
            "اطبع(يميناً_منطقية(255، 4) == بتات_إزاحة_يمين_منطقية(255، 4))\n",
            "اطبع(يميناً_منطقية(-1، 64) == بتات_إزاحة_يمين_منطقية(-1، 64))",
        ),
        &["صحيح", "صحيح", "صحيح", "صحيح", "صحيح"],
    );
}

/// The two shifts against each other, which is how they are actually used: a
/// left shift builds a mask at a position and a right shift brings the field it
/// selects back down. Zero fill makes the round trip exact even where the
/// operand's top bit is set, which the arithmetic shift cannot do.
#[test]
fn test_logical_right_shift_undoes_the_left_shift() {
    assert_prints(
        "إزاحة_منطقية_ذهاب_وإياب",
        concat!(
            "اطبع(بتات_إزاحة_يمين_منطقية(بتات_إزاحة_يسار(1، 10)، 10))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(بتات_إزاحة_يسار(255، 56)، 56))\n",
            "اطبع(بتات_إزاحة_يمين(بتات_إزاحة_يسار(255، 56)، 56))\n",
            "اطبع(بتات_إزاحة_يمين_منطقية(بتات_و(4660، بتات_إزاحة_يسار(255، 8))، 8))",
        ),
        &["1", "255", "-1", "18"],
    );
}

/// A narrowed optional is still a boxed pointer in codegen, so it is covered in
/// both argument positions and in both at once. Unlike its siblings the *value*
/// position matters here too: the lowering reads the value twice, and folding
/// the out-of-range mask into that read is what unboxes it (#318). The negative
/// case is the one that would print a small negative number if the mask had been
/// applied to the result instead and the value left boxed.
#[test]
fn test_logical_right_shift_over_a_narrowed_optional() {
    assert_prints(
        "إزاحة_منطقية_اختياري",
        concat!(
            "متغير س: عدد? = 5\n",
            "إذا (س != لا_شيء) {\n",
            "    اطبع(بتات_إزاحة_يمين_منطقية(س، 1))\n",
            "    اطبع(بتات_إزاحة_يمين_منطقية(64، س))\n",
            "    اطبع(بتات_إزاحة_يمين_منطقية(س، س))\n",
            "    اطبع(بتات_إزاحة_يمين_منطقية(س، 1) + 1)\n",
            "}\n",
            "متغير ص: عدد? = -16\n",
            "إذا (ص != لا_شيء) {\n",
            "    اطبع(بتات_إزاحة_يمين_منطقية(ص، 4))\n",
            "    اطبع(بتات_إزاحة_يمين_منطقية(ص، 0))\n",
            "}",
        ),
        &["2", "2", "0", "3", "1152921504606846975", "-16"],
    );
}

/// Builtins are the last tier of the lookup order, so a user function of the
/// same name must win — in every backend at once (#262).
#[test]
fn test_user_function_shadows_logical_right_shift() {
    assert_prints(
        "إزاحة_منطقية_مظلل",
        "دالة بتات_إزاحة_يمين_منطقية(س: عدد، ن: عدد) -> عدد {\n    أرجع 42\n}\nاطبع(بتات_إزاحة_يمين_منطقية(-1، 1))",
        &["42"],
    );
}

// ---------------------------------------------------------------------------
// حرف_إلى_رمز — the codepoint accessor (#324)
// ---------------------------------------------------------------------------

/// One case per UTF-8 encoding width. The native leg reads the lead byte to
/// decide how many bytes belong to the first character while both interpreters
/// call `chars()`, so a width the byte walk gets wrong is the only way these two
/// implementations can disagree — and the wrong answer would still print.
#[test]
fn test_char_code_decodes_every_utf8_width_in_every_backend() {
    assert_prints(
        "رمز_عروض",
        concat!(
            "اطبع(حرف_إلى_رمز(\"A\"))\n",
            "اطبع(حرف_إلى_رمز(\"م\"))\n",
            "اطبع(حرف_إلى_رمز(\"﷽\"))\n",
            "اطبع(حرف_إلى_رمز(\"𞸀\"))\n",
            "اطبع(حرف_إلى_رمز(\"\"))",
        ),
        &["65", "1605", "65021", "126464", "-1"],
    );
}

/// A codepoint, not a grapheme: the fatha in "مَ" is a codepoint of its own and
/// the *second* one, so a first-grapheme reading would still answer 1605 here
/// but the bare-fatha line pins which unit is being counted.
#[test]
fn test_char_code_reads_the_first_codepoint_not_the_first_grapheme() {
    assert_prints(
        "رمز_تشكيل",
        concat!(
            "اطبع(حرف_إلى_رمز(\"مَرحبا\"))\n",
            "اطبع(حرف_إلى_رمز(\"َ\"))\n",
            "اطبع(حرف_إلى_رمز(\"مرحبا\"))",
        ),
        &["1605", "1614", "1605"],
    );
}

/// The load-bearing test. `حرف_إلى_رمز` lowers to a plain call, so nothing but
/// the `register_builtin_return_types` entry types its result; without it the
/// result carries the `Ptr(Void)` sentinel, printing keeps working, and
/// concatenation prints a pointer instead — which is exactly how
/// `"X" + حرف_في(س،١)` printed `X4377631856` natively.
#[test]
fn test_char_code_result_composes_as_an_integer() {
    assert_prints(
        "رمز_تركيب",
        concat!(
            "اطبع(نوع(حرف_إلى_رمز(\"م\")))\n",
            "اطبع(حرف_إلى_رمز(\"م\") + 1)\n",
            "اطبع(حرف_إلى_رمز(\"م\") == 1605)\n",
            "اطبع(\"الرمز: \" + حرف_إلى_رمز(\"ب\"))",
        ),
        &["عدد", "1606", "صحيح", "الرمز: 1576"],
    );
}

/// A literal argument can fold at build time; a variable and a concatenation
/// cannot, so these are the cases where the runtime call is actually made.
#[test]
fn test_char_code_over_a_computed_string() {
    assert_prints(
        "رمز_محسوب",
        concat!(
            "متغير كلمة = \"سلام\"\n",
            "اطبع(حرف_إلى_رمز(كلمة))\n",
            "اطبع(حرف_إلى_رمز(\"ن\" + \"ور\"))\n",
            "دالة أول_رمز(س: نص) -> عدد {\n",
            "    أرجع حرف_إلى_رمز(س)\n",
            "}\n",
            "اطبع(أول_رمز(\"دار\"))",
        ),
        &["1587", "1606", "1583"],
    );
}

/// `Type::compat` lets an un-narrowed `نص?` into a `نص` parameter, and native
/// lowers that to `ptr null` where the runtime's guard answers `-1`. Both
/// interpreters therefore need a `Null` arm; keying only on `Value::String`
/// would make them abort on source that native runs fine. The narrowed leg is
/// the string analogue of #318 — a narrowed `نص?` stays a bare `TrqString*`,
/// so it needs no unboxing.
#[test]
fn test_char_code_over_an_optional_string() {
    assert_prints(
        "رمز_اختياري",
        concat!(
            "متغير فارغ: نص? = لا_شيء\n",
            "اطبع(حرف_إلى_رمز(فارغ))\n",
            "متغير موجود: نص? = \"ه\"\n",
            "إذا (موجود != لا_شيء) {\n",
            "    اطبع(حرف_إلى_رمز(موجود))\n",
            "    اطبع(حرف_إلى_رمز(موجود) + 1)\n",
            "}",
        ),
        &["-1", "1607", "1608"],
    );
}

/// Builtins are the last tier of the lookup order, so a user function of the
/// same name must win — in every backend at once (#262). This is the first
/// symbol-mapped core builtin, so the shadow has to survive two independent
/// gates: `shadows_builtin` in the IR builder and the `user_functions` check in
/// codegen's `mangle_function_name`, which would otherwise emit a call to
/// `@trq_string_char_code` instead of the user's symbol.
#[test]
fn test_user_function_shadows_char_code() {
    assert_prints(
        "رمز_مظلل",
        "دالة حرف_إلى_رمز(س: نص) -> عدد {\n    أرجع 42\n}\nاطبع(حرف_إلى_رمز(\"م\"))",
        &["42"],
    );
}

// ---------------------------------------------------------------------------
// رمز_إلى_حرف — the codepoint constructor (#326)
// ---------------------------------------------------------------------------

/// One case per UTF-8 encoding width, read back through `حرف_إلى_رمز` rather
/// than compared against a literal: the two names are one contract, and a width
/// the encoder gets wrong would otherwise still print something plausible.
#[test]
fn test_char_from_code_encodes_every_utf8_width_in_every_backend() {
    assert_prints(
        "حرف_عروض",
        concat!(
            "اطبع(رمز_إلى_حرف(65))\n",
            "اطبع(رمز_إلى_حرف(1605))\n",
            "اطبع(رمز_إلى_حرف(65021))\n",
            "اطبع(رمز_إلى_حرف(126464))",
        ),
        &["A", "م", "﷽", "𞸀"],
    );
}

/// Asserted through `طول`, never by printing the result: `Output::lines()`
/// trims, so a printed empty string is indistinguishable from a printed newline
/// and the test could not fail.
///
/// The last case is why the range check is on the `i64` before any cast —
/// `4294967361 as u32` is 65, so an unguarded cast would answer "A".
#[test]
fn test_char_from_code_rejects_unrepresentable_code_points() {
    assert_prints(
        "حرف_مرفوض",
        concat!(
            "اطبع(طول(رمز_إلى_حرف(-1)))\n",
            "اطبع(طول(رمز_إلى_حرف(55296)))\n",
            "اطبع(طول(رمز_إلى_حرف(57343)))\n",
            "اطبع(طول(رمز_إلى_حرف(1114112)))\n",
            "اطبع(طول(رمز_إلى_حرف(4294967361)))",
        ),
        &["0", "0", "0", "0", "0"],
    );
}

/// The boundaries either side of each rejected range, so the guard cannot
/// quietly become one wider or one narrower than Unicode.
#[test]
fn test_char_from_code_accepts_the_range_boundaries() {
    assert_prints(
        "حرف_حدود",
        concat!(
            "اطبع(حرف_إلى_رمز(رمز_إلى_حرف(55295)))\n",
            "اطبع(حرف_إلى_رمز(رمز_إلى_حرف(57344)))\n",
            "اطبع(حرف_إلى_رمز(رمز_إلى_حرف(1114111)))",
        ),
        &["55295", "57344", "1114111"],
    );
}

/// The load-bearing test. `رمز_إلى_حرف` lowers to a plain call, so nothing but
/// the `register_builtin_return_types` entry types its result; without it the
/// result carries the `Ptr(Void)` sentinel, printing keeps working, and
/// concatenating or comparing prints a pointer instead — exactly how
/// `"X" + حرف_في(س،١)` printed `X4377631856` natively.
#[test]
fn test_char_from_code_result_composes_as_a_string() {
    assert_prints(
        "حرف_تركيب",
        concat!(
            "اطبع(نوع(رمز_إلى_حرف(65)))\n",
            "اطبع(\"س\" + رمز_إلى_حرف(65))\n",
            "اطبع(رمز_إلى_حرف(1605) == \"م\")\n",
            "اطبع(طول(رمز_إلى_حرف(65)))",
        ),
        &["نص", "سA", "صحيح", "1"],
    );
}

/// U+0000 is a one-character string, not the empty one — which is precisely why
/// `حرف_إلى_رمز` could not use `0` as its "no first character" sentinel.
/// Asserted through `طول` and the round trip rather than by printing, since a
/// NUL byte on stdout is a question about the terminal, not about the contract.
#[test]
fn test_char_from_code_builds_a_nul_character() {
    assert_prints(
        "حرف_صفر",
        concat!(
            "اطبع(طول(رمز_إلى_حرف(0)))\n",
            "اطبع(حرف_إلى_رمز(رمز_إلى_حرف(0)))",
        ),
        &["1", "0"],
    );
}

/// The pair is total in both directions: every valid code round-trips, and
/// every rejected one lands on `""`, whose code is `-1`. So the two sentinels
/// map onto each other instead of leaving a hole.
#[test]
fn test_char_code_and_char_from_code_round_trip() {
    assert_prints(
        "حرف_إياب",
        concat!(
            "اطبع(رمز_إلى_حرف(حرف_إلى_رمز(\"م\")))\n",
            "اطبع(حرف_إلى_رمز(رمز_إلى_حرف(126464)))\n",
            "اطبع(حرف_إلى_رمز(رمز_إلى_حرف(-1)))\n",
            "اطبع(حرف_إلى_رمز(رمز_إلى_حرف(1114112)))",
        ),
        &["م", "126464", "-1", "-1"],
    );
}

/// A literal argument can fold at build time; a variable, an arithmetic
/// expression and a parameter cannot, so these are the cases where the runtime
/// call is actually made.
#[test]
fn test_char_from_code_over_a_computed_argument() {
    assert_prints(
        "حرف_محسوب",
        concat!(
            "متغير رمز = 1605\n",
            "اطبع(رمز_إلى_حرف(رمز))\n",
            "اطبع(رمز_إلى_حرف(1600 + 5))\n",
            "دالة حرف_من(ر: عدد) -> نص {\n",
            "    أرجع رمز_إلى_حرف(ر)\n",
            "}\n",
            "اطبع(حرف_من(1583))",
        ),
        &["م", "م", "د"],
    );
}

/// Builtins are the last tier of the lookup order, so a user function of the
/// same name must win — in every backend at once (#262). A symbol-mapped
/// builtin has to survive two independent gates: `shadows_builtin` in the IR
/// builder, and the `user_functions` check in codegen's `mangle_function_name`,
/// which would otherwise emit `@trq_string_from_char_code` instead of the
/// user's own symbol.
#[test]
fn test_user_function_shadows_char_from_code() {
    assert_prints(
        "حرف_مظلل",
        "دالة رمز_إلى_حرف(ر: عدد) -> نص {\n    أرجع \"ظ\"\n}\nاطبع(رمز_إلى_حرف(65))",
        &["ظ"],
    );
}

// ---------------------------------------------------------------------------
// نص_إلى_ثنائي — the string→bytes bridge (#330)
// ---------------------------------------------------------------------------

/// Byte counts per UTF-8 width, and the point of the primitive: `طول` of the
/// result is the **byte** length while `طول` of the string is the character
/// length. Both are asserted side by side, since a result that silently counted
/// characters would agree with the string on every ASCII input.
#[test]
fn test_string_to_bytes_counts_bytes_in_every_backend() {
    assert_prints(
        "ثنائي_أطوال",
        concat!(
            "اطبع(طول(نص_إلى_ثنائي(\"A\")))\n",
            "اطبع(طول(نص_إلى_ثنائي(\"م\")))\n",
            "اطبع(طول(نص_إلى_ثنائي(\"﷽\")))\n",
            "اطبع(طول(نص_إلى_ثنائي(\"𞸀\")))\n",
            "اطبع(طول(نص_إلى_ثنائي(\"مرحبا\")))\n",
            "اطبع(طول(\"مرحبا\"))",
        ),
        &["1", "2", "3", "4", "10", "5"],
    );
}

/// The load-bearing test. `نص_إلى_ثنائي` lowers to a plain call, so nothing but
/// the `register_builtin_return_types` entry types its result — and an array is
/// the one return type whose absence does **not** show up as a signature
/// mismatch, because `Ptr(Void)` and `Array` both map to LLVM `ptr`. Dropping
/// the entry links and runs; the element load turns into `load ptr` on an i64
/// slot and `اطبع` reads the `TrqArray` as a `TrqString`. Indexing is therefore
/// the only thing that fails loudly, which is why it is asserted here.
#[test]
fn test_string_to_bytes_result_composes_as_an_integer_array() {
    assert_prints(
        "ثنائي_تركيب",
        concat!(
            "اطبع(نص_إلى_ثنائي(\"hi\"))\n",
            "اطبع(نص_إلى_ثنائي(\"A\")[0])\n",
            "اطبع(نص_إلى_ثنائي(\"A\")[0] + 1)\n",
            "اطبع(نص_إلى_ثنائي(\"A\")[0] == 65)\n",
            "اطبع(نوع(نص_إلى_ثنائي(\"A\")[0]))",
        ),
        &["[104، 105]", "65", "66", "صحيح", "عدد"],
    );
}

/// An empty string has no bytes, so the answer is an empty array — a value, not
/// a sentinel. Asserted through `طول` and by printing the array itself, never by
/// printing a bare `""`: `Output::lines()` trims, so that assertion could not
/// fail.
#[test]
fn test_string_to_bytes_of_an_empty_string_is_an_empty_array() {
    assert_prints(
        "ثنائي_فارغ",
        "اطبع(طول(نص_إلى_ثنائي(\"\")))\nاطبع(نص_إلى_ثنائي(\"\"))",
        &["0", "[]"],
    );
}

/// `Type::compat` admits an un-narrowed `نص?` into a `نص` parameter, so this
/// type-checks and native lowers it to `ptr null`, where the runtime guard
/// answers an empty array. Both interpreters need a `Value::Null` arm to agree;
/// keyed on `Value::String` alone they abort where native prints. The *narrowed*
/// shape is deliberately absent — that is #327, and it is still open.
#[test]
fn test_string_to_bytes_accepts_an_absent_optional_in_every_backend() {
    assert_prints(
        "ثنائي_غائب",
        "متغير غائب: نص? = لا_شيء\nاطبع(طول(نص_إلى_ثنائي(غائب)))",
        &["0"],
    );
}

/// A literal argument can fold at build time, so the argument is reached three
/// other ways: through a variable, through a concatenation, and through a
/// function parameter.
#[test]
fn test_string_to_bytes_accepts_a_computed_argument() {
    assert_prints(
        "ثنائي_محسوب",
        concat!(
            "متغير ن = \"مرحبا\"\n",
            "اطبع(طول(نص_إلى_ثنائي(ن)))\n",
            "اطبع(طول(نص_إلى_ثنائي(ن + \"!\")))\n",
            "دالة عدد_بايتات(س: نص) -> عدد {\n",
            "    أرجع طول(نص_إلى_ثنائي(س))\n",
            "}\n",
            "اطبع(عدد_بايتات(\"م\"))",
        ),
        &["10", "11", "2"],
    );
}

/// The result is a first-class array: it binds to a variable, survives the
/// `Load` that reading that variable emits, indexes, and iterates. `لكل … في`
/// matters on its own because it lowers through `ArrayLen`, which dispatches on
/// the operand's IR type and would pick the string symbol if that type were
/// wrong.
#[test]
fn test_string_to_bytes_result_binds_and_iterates() {
    assert_prints(
        "ثنائي_حلقة",
        concat!(
            "متغير ب = نص_إلى_ثنائي(\"Az\")\n",
            "اطبع(طول(ب))\n",
            "اطبع(ب[0])\n",
            "اطبع(ب[1])\n",
            "متغير مج = 0\n",
            "لكل ي في ب {\n",
            "    مج = مج + ي\n",
            "}\n",
            "اطبع(مج)",
        ),
        &["2", "65", "122", "187"],
    );
}

/// Appending to the **empty** result, which is the one array in the language
/// reachable with `cap == 0`: `helpers::allocate_array` sets `cap = len`, while
/// `trq_array_new` floors capacity at `ARRAY_INITIAL_CAP`. Growth doubles, and
/// `0 * 2` is `0`, so this hung the native binary forever while both
/// interpreters answered `1`.
///
/// `runtime-rs` has a unit test for the same shape, but **CI never runs it** —
/// every CI `cargo test` is root-package scoped, so this is the leg that
/// actually guards the fix. Note the failure mode if it regresses: the native
/// leg hangs rather than failing, so the signal is a stuck job, not a red
/// assertion.
#[test]
fn test_appending_to_an_empty_byte_array_grows_it_in_every_backend() {
    assert_prints(
        "ثنائي_إلحاق",
        concat!(
            "متغير ب = نص_إلى_ثنائي(\"\")\n",
            "الحق(ب، 5)\n",
            "اطبع(طول(ب))\n",
            "اطبع(ب[0])",
        ),
        &["1", "5"],
    );
}

/// Round trip against the landed inverse: an ASCII byte read out here is the
/// codepoint `رمز_إلى_حرف` builds the same character from. Restricted to ASCII
/// on purpose — a multi-byte character's *bytes* are not its codepoint, and
/// reassembling one needs `ثنائي_إلى_نص`, which does not exist yet.
#[test]
fn test_string_to_bytes_round_trips_ascii_through_char_from_code() {
    assert_prints(
        "ثنائي_ذهاب",
        concat!(
            "اطبع(رمز_إلى_حرف(نص_إلى_ثنائي(\"z\")[0]))\n",
            "اطبع(حرف_إلى_رمز(\"z\") == نص_إلى_ثنائي(\"z\")[0])",
        ),
        &["z", "صحيح"],
    );
}

/// Builtins are the last lookup tier, so a user function of the same name must
/// win in every backend (#262) — past both `shadows_builtin` in the IR builder
/// and the `user_functions` check in codegen's `mangle_function_name`, which
/// would otherwise emit `@trq_string_to_bytes` for the user's own definition.
#[test]
fn test_user_function_shadows_string_to_bytes() {
    assert_prints(
        "ثنائي_مظلل",
        concat!(
            "دالة نص_إلى_ثنائي(س: نص) -> مصفوفة<عدد> {\n",
            "    أرجع [7]\n",
            "}\n",
            "اطبع(طول(نص_إلى_ثنائي(\"مرحبا\")))\n",
            "اطبع(نص_إلى_ثنائي(\"مرحبا\")[0])",
        ),
        &["1", "7"],
    );
}

// ---------------------------------------------------------------------------
// ثنائي_إلى_نص — the bytes→string bridge (#333)
// ---------------------------------------------------------------------------

/// The pair is total in both directions at every UTF-8 width, which is what
/// closes blocker **B9**: before this, octets could be read out of a string and
/// never assembled back into one.
#[test]
fn test_bytes_to_string_round_trips_in_every_backend() {
    assert_prints(
        "نص_دورة",
        concat!(
            "اطبع(ثنائي_إلى_نص(نص_إلى_ثنائي(\"A\")))\n",
            "اطبع(ثنائي_إلى_نص(نص_إلى_ثنائي(\"م\")))\n",
            "اطبع(ثنائي_إلى_نص(نص_إلى_ثنائي(\"﷽\")))\n",
            "اطبع(ثنائي_إلى_نص(نص_إلى_ثنائي(\"𞸀\")))\n",
            "اطبع(ثنائي_إلى_نص(نص_إلى_ثنائي(\"مرحبا\")) == \"مرحبا\")",
        ),
        &["A", "م", "﷽", "𞸀", "صحيح"],
    );
}

/// Byte arrays written by hand, so the decoder cannot pass merely by agreeing
/// with the encoder it inverts. `[217، 133]` is the load-bearing line: those are
/// «م»'s two **octets**, so an implementation that read the array as codepoints
/// would answer «Ù…» — two characters — instead.
#[test]
fn test_bytes_to_string_decodes_arrays_written_by_hand() {
    assert_prints(
        "نص_مكتوب",
        concat!(
            "اطبع(ثنائي_إلى_نص([104، 105]))\n",
            "اطبع(ثنائي_إلى_نص([217، 133]))\n",
            "اطبع(ثنائي_إلى_نص([239، 183، 189]))\n",
            "اطبع(طول(ثنائي_إلى_نص([217، 133])))\n",
            "متغير ب = [65، 122]\n",
            "اطبع(ثنائي_إلى_نص(ب))",
        ),
        &["hi", "م", "﷽", "1", "Az"],
    );
}

/// The load-bearing test, and it catches more than its sibling's did. Deleting
/// the `register_builtin_return_types` entry leaves the module valid — `Ptr(Void)`
/// and `String` are both LLVM `ptr` — so `اطبع` still printed «مرحبا» correctly.
/// Measured with the entry removed: `نوع` answered `مؤشر`, concatenation printed
/// `X4340804192`, and `==` answered `خطأ`. Printing alone passes; these three do
/// not.
#[test]
fn test_bytes_to_string_result_composes_as_a_string() {
    assert_prints(
        "نص_تركيب",
        concat!(
            "اطبع(نوع(ثنائي_إلى_نص([65])))\n",
            "اطبع(\"X\" + ثنائي_إلى_نص([65])) \n",
            "اطبع(ثنائي_إلى_نص([65]) == \"A\")\n",
            "اطبع(طول(ثنائي_إلى_نص([104، 105])))\n",
            "اطبع(حرف_إلى_رمز(ثنائي_إلى_نص([217، 133])))",
        ),
        &["نص", "XA", "صحيح", "2", "1605"],
    );
}

/// One rule covers both rejections: an element that is not a byte, and bytes that
/// are not a UTF-8 encoding. Asserted through `طول`, never by expecting a printed
/// `""` — `Output::lines()` trims, so that assertion could never fail.
#[test]
fn test_bytes_to_string_rejects_what_is_not_an_encoding() {
    assert_prints(
        "نص_مرفوض",
        concat!(
            // Not bytes.
            "اطبع(طول(ثنائي_إلى_نص([300])))\n",
            "اطبع(طول(ثنائي_إلى_نص([-1])))\n",
            "اطبع(طول(ثنائي_إلى_نص([65، 256])))\n",
            // Bytes, but not an encoding: never a lead byte, a truncated
            // sequence, a stray continuation, an overlong U+0000, a surrogate.
            "اطبع(طول(ثنائي_إلى_نص([255])))\n",
            "اطبع(طول(ثنائي_إلى_نص([217])))\n",
            "اطبع(طول(ثنائي_إلى_نص([133])))\n",
            "اطبع(طول(ثنائي_إلى_نص([192، 128])))\n",
            "اطبع(طول(ثنائي_إلى_نص([237، 160، 128])))\n",
            // An empty array is a value, not a rejection: it has one decoding.
            "اطبع(طول(ثنائي_إلى_نص([])))",
        ),
        &["0", "0", "0", "0", "0", "0", "0", "0", "0"],
    );
}

/// `لا_شيء` reaches the arm through an `أي` holder rather than an optional
/// annotation — `مصفوفة<عدد>؟` does not parse (ب٠١٠١) and a bare `لا_شيء` is
/// refused at the argument. Without the `Value::Null` arm both interpreters raise
/// a type error here while native answers `""`, which is the divergence class
/// this whole increment exists to avoid.
#[test]
fn test_bytes_to_string_accepts_a_null_holder() {
    assert_prints(
        "نص_غائب",
        concat!(
            "متغير غائب: أي = لا_شيء\n",
            "اطبع(طول(ثنائي_إلى_نص(غائب)))\n",
            "اطبع(\"[\" + ثنائي_إلى_نص(غائب) + \"]\")",
        ),
        &["0", "[]"],
    );
}

/// `docs/builtins-vs-stdlib.md` §1.3 claims criterion (a) — inexpressible — for
/// this name, and that claim **expired** before it shipped: indexing over
/// `مصفوفة<عدد>` (#330), the seven bitwise names (increment A) and `رمز_إلى_حرف`
/// (#326) together make UTF-8 decoding writable in Tarqeem. So rather than repeat
/// a stale claim, this asserts the equivalence — the third row in that document to
/// need this treatment, after `بتات_نفي` and `بتات_إزاحة_يمين_منطقية`.
///
/// The scope is deliberately **valid input only**. The hand decoder reproduces
/// the decoding, not the validation: rejecting overlong forms and surrogates is
/// the half that stays materially harder to write by hand, and it is why the
/// primitive is still worth having.
#[test]
fn test_bytes_to_string_matches_the_decoder_it_names() {
    assert_prints(
        "نص_مكافئ",
        concat!(
            "دالة فكك(ب: مصفوفة<عدد>) -> نص {\n",
            "    متغير ناتج = \"\"\n",
            "    متغير ي = 0\n",
            "    طالما (ي < طول(ب)) {\n",
            "        ثابت بادئ = ب[ي]\n",
            "        متغير رمز = 0\n",
            "        متغير عرض = 1\n",
            "        إذا (بادئ < 128) {\n",
            "            رمز = بادئ\n",
            "        } وإلا إذا (بتات_و(بادئ، 224) == 192) {\n",
            "            رمز = بتات_و(بادئ، 31)\n",
            "            عرض = 2\n",
            "        } وإلا إذا (بتات_و(بادئ، 240) == 224) {\n",
            "            رمز = بتات_و(بادئ، 15)\n",
            "            عرض = 3\n",
            "        } وإلا {\n",
            "            رمز = بتات_و(بادئ، 7)\n",
            "            عرض = 4\n",
            "        }\n",
            "        متغير خطوة = 1\n",
            "        طالما (خطوة < عرض) {\n",
            "            رمز = بتات_أو(بتات_إزاحة_يسار(رمز، 6)، بتات_و(ب[ي + خطوة]، 63))\n",
            "            خطوة = خطوة + 1\n",
            "        }\n",
            "        ناتج = ناتج + رمز_إلى_حرف(رمز)\n",
            "        ي = ي + عرض\n",
            "    }\n",
            "    أرجع ناتج\n",
            "}\n",
            "لكل نص_مدخل في [\"A\"، \"م\"، \"﷽\"، \"𞸀\"، \"مرحبا\"، \"Az0\"، \"\"] {\n",
            "    ثابت بايتات = نص_إلى_ثنائي(نص_مدخل)\n",
            "    اطبع(ثنائي_إلى_نص(بايتات) == فكك(بايتات))\n",
            "}",
        ),
        &["صحيح", "صحيح", "صحيح", "صحيح", "صحيح", "صحيح", "صحيح"],
    );
}

/// Builtins are the last lookup tier, so a user definition must win in every
/// backend (#262) — including past codegen's `mangle_function_name`, which would
/// otherwise emit `@trq_string_from_bytes` for the user's own function.
#[test]
fn test_user_function_shadows_bytes_to_string() {
    assert_prints(
        "نص_مظلل",
        concat!(
            "دالة ثنائي_إلى_نص(ب: مصفوفة<عدد>) -> نص {\n",
            "    أرجع \"دالتي\"\n",
            "}\n",
            "اطبع(ثنائي_إلى_نص([104، 105]))",
        ),
        &["دالتي"],
    );
}

// ---------------------------------------------------------------------------
// قص_حروف — the codepoint slicer (#336)
// ---------------------------------------------------------------------------

/// The whole point of the name: it counts characters, not octets. Every fixture
/// here is multi-byte, because a byte slicer agrees with a codepoint slicer on
/// all of ASCII — that agreement is exactly how a regression would pass.
#[test]
fn test_substr_chars_slices_by_codepoint_in_every_backend() {
    assert_prints(
        "قص_حروف_أساسي",
        concat!(
            "اطبع(قص_حروف(\"مرحبا\"، 1، 3))\n",
            // One codepoint of each UTF-8 width in one string, so a slicer that
            // walked bytes would cut inside the 3- and 4-byte characters.
            "اطبع(قص_حروف(\"A﷽م𞸀ب\"، 1، 3))\n",
            "اطبع(قص_حروف(\"𞸀م\"، 0، 1))\n",
            "اطبع(قص_حروف(\"مرحبا\"، 0، 5))\n",
        ),
        &["رحب", "﷽م𞸀", "𞸀", "مرحبا"],
    );
}

/// The load-bearing test, and the one that fails if the
/// `register_builtin_return_types` entry is ever dropped. That was `قص_حروف`'s
/// state for two releases (**B7**): mapped to a runtime symbol with no registered
/// return type, so it *printed* correctly while carrying the `Ptr(Void)` sentinel.
///
/// Measured natively with the entry removed, exactly as #333 measured its own —
/// and **four** of the five assertions catch it, where #333's array caught one and
/// its string caught three. `نوع` answered `مؤشر`, `"X" + …` printed
/// `X4341079168`, `== "رح"` answered `خطأ`, and `طول` answered **6 instead of 3**:
/// the sentinel routes it to `trq_array_len`, which reads `TrqString.len` — the
/// byte count. So the failure mode of dropping the entry is that this name starts
/// counting bytes, which is the one thing it exists not to do. Only
/// `حرف_إلى_رمز` still answered correctly, and printing alone passed. The binary
/// exited 0 throughout.
#[test]
fn test_substr_chars_result_composes_as_a_string() {
    assert_prints(
        "قص_حروف_تركيب",
        concat!(
            "اطبع(نوع(قص_حروف(\"مرحبا\"، 1، 2)))\n",
            "اطبع(\"X\" + قص_حروف(\"مرحبا\"، 1، 2))\n",
            "اطبع(قص_حروف(\"مرحبا\"، 1، 2) == \"رح\")\n",
            "اطبع(طول(قص_حروف(\"مرحبا\"، 1، 3)))\n",
            "اطبع(حرف_إلى_رمز(قص_حروف(\"مرحبا\"، 0، 1)))\n",
        ),
        &["نص", "Xرح", "صحيح", "3", "1605"],
    );
}

/// Total, so no call site needs a range check before calling it. Each answer is
/// `trq_string_substr_chars`'s, copied rather than chosen — a negative start, a
/// start past the end and a non-positive length all give `""`, and a length past
/// the end clamps to what remains.
///
/// Asserted through `طول` rather than by printing, because an empty line and a
/// line that failed to print are the same line.
#[test]
fn test_substr_chars_is_total_out_of_range() {
    assert_prints(
        "قص_حروف_مدى",
        concat!(
            "اطبع(طول(قص_حروف(\"مرحبا\"، -1، 2)))\n",
            "اطبع(طول(قص_حروف(\"مرحبا\"، 9، 2)))\n",
            "اطبع(طول(قص_حروف(\"مرحبا\"، 5، 1)))\n",
            "اطبع(طول(قص_حروف(\"مرحبا\"، 1، 0)))\n",
            "اطبع(طول(قص_حروف(\"مرحبا\"، 1، -3)))\n",
            "اطبع(طول(قص_حروف(\"\"، 0، 1)))\n",
            // Clamping, not truncation to nothing: three characters remain.
            "اطبع(طول(قص_حروف(\"مرحبا\"، 2، 99)))\n",
        ),
        &["0", "0", "0", "0", "0", "0", "3"],
    );
}

/// `Type::compat` lets an un-narrowed `نص؟` into a `نص` parameter and native
/// lowers it to `ptr null`, where the runtime's guard answers `""`. Without the
/// interpreter's `Value::Null` arm this source aborts interpreted while running
/// fine natively — the divergence class #324 found and #330 confirmed.
///
/// The two `عدد` parameters get no such arm, and that is deliberate: there
/// native's `0` is an artifact of passing a null pointer in an i64 slot, not a
/// designed answer, so mirroring it would encode #327 as contract.
#[test]
fn test_substr_chars_accepts_a_null_holder() {
    assert_prints(
        "قص_حروف_غائب",
        concat!(
            "متغير غائب: نص? = لا_شيء\n",
            "اطبع(طول(قص_حروف(غائب، 0، 3)))\n",
            "اطبع(قص_حروف(غائب، 0، 3) == \"\")\n",
        ),
        &["0", "صحيح"],
    );
}

/// §1.3 of `docs/builtins-vs-stdlib.md` justifies this name under criterion (a),
/// and that claim **expired** before it shipped — the fourth row in that document
/// to need this treatment, after `بتات_نفي` (#312), `بتات_إزاحة_يمين_منطقية`
/// (#322) and `ثنائي_إلى_نص` (#333). `نص_إلى_ثنائي` (#330), indexing
/// over `مصفوفة<عدد>` and the bitwise family together make a codepoint slicer
/// writable in Tarqeem, so rather than repeat a stale claim this asserts the
/// equivalence.
///
/// It ships as a primitive anyway, on grounds that do not depend on
/// inexpressibility: §5.2 keeps a no-import name a builtin until **B12** is fixed,
/// so stdlib is not an available home, and it is a repair of an already-registered
/// name rather than a new one.
///
/// The accumulator is seeded with `نص_إلى_ثنائي("")` rather than a `[]` literal:
/// the literal route is refused by codegen for a typed array, and the empty
/// array's zero capacity is the shape that used to hang `الحق` natively.
#[test]
fn test_substr_chars_matches_the_slicer_it_names() {
    assert_prints(
        "قص_حروف_مكافئ",
        concat!(
            "دالة اسلخ(س: نص، بداية: عدد، عدد_أحرف: عدد) -> نص {\n",
            "    إذا (بداية < 0 || عدد_أحرف <= 0) {\n",
            "        أرجع \"\"\n",
            "    }\n",
            "    ثابت بايتات = نص_إلى_ثنائي(س)\n",
            "    متغير ناتج = نص_إلى_ثنائي(\"\")\n",
            "    متغير موضع = 0\n",
            "    متغير رقم_الحرف = 0\n",
            "    طالما (موضع < طول(بايتات)) {\n",
            "        ثابت بادئ = بايتات[موضع]\n",
            "        متغير عرض = 1\n",
            "        إذا (بتات_و(بادئ، 224) == 192) {\n",
            "            عرض = 2\n",
            "        } وإلا إذا (بتات_و(بادئ، 240) == 224) {\n",
            "            عرض = 3\n",
            "        } وإلا إذا (بتات_و(بادئ، 248) == 240) {\n",
            "            عرض = 4\n",
            "        }\n",
            "        إذا (رقم_الحرف >= بداية && رقم_الحرف < بداية + عدد_أحرف) {\n",
            "            متغير خطوة = 0\n",
            "            طالما (خطوة < عرض) {\n",
            "                الحق(ناتج، بايتات[موضع + خطوة])\n",
            "                خطوة = خطوة + 1\n",
            "            }\n",
            "        }\n",
            "        موضع = موضع + عرض\n",
            "        رقم_الحرف = رقم_الحرف + 1\n",
            "    }\n",
            "    أرجع ثنائي_إلى_نص(ناتج)\n",
            "}\n",
            "اطبع(اسلخ(\"مرحبا\"، 1، 3) == قص_حروف(\"مرحبا\"، 1، 3))\n",
            "اطبع(اسلخ(\"A﷽م𞸀ب\"، 1، 3) == قص_حروف(\"A﷽م𞸀ب\"، 1، 3))\n",
            "اطبع(اسلخ(\"مرحبا\"، 0، 9) == قص_حروف(\"مرحبا\"، 0، 9))\n",
            "اطبع(اسلخ(\"مرحبا\"، -1، 2) == قص_حروف(\"مرحبا\"، -1، 2))\n",
            "اطبع(اسلخ(\"مرحبا\"، 9، 2) == قص_حروف(\"مرحبا\"، 9، 2))\n",
        ),
        &["صحيح", "صحيح", "صحيح", "صحيح", "صحيح"],
    );
}

/// Builtins are the last lookup tier, so a user definition must win in every
/// backend (#262) — including past codegen's `mangle_function_name`, which would
/// otherwise emit `@trq_string_substr_chars` for the user's own function.
#[test]
fn test_user_function_shadows_substr_chars() {
    assert_prints(
        "قص_حروف_مظلل",
        concat!(
            "دالة قص_حروف(س: نص، ب: عدد، ط: عدد) -> نص {\n",
            "    أرجع \"دالتي\"\n",
            "}\n",
            "اطبع(قص_حروف(\"مرحبا\"، 1، 2))",
        ),
        &["دالتي"],
    );
}

// ---------------------------------------------------------------------------
// متغير_بيئة — قراءة متغيّرات البيئة (#338)
// ---------------------------------------------------------------------------

/// The name of every variable these tests inject. Latin, because environment
/// variable names conventionally are — Arabic reaches the runtime as ordinary
/// string data either way, and the absent-name test below relies on that.
const ENV_PROBE: &str = "TARQEEM_ENV_PROBE_338";
const ENV_EMPTY: &str = "TARQEEM_ENV_EMPTY_338";

/// The Arabic value is not decoration: it is what proves the value survives
/// `std::env::var` → `TrqString` → print, and it is also what makes `طول` able
/// to catch a missing return type below. On an ASCII value the byte count and
/// the character count agree, and the assertion would pass either way.
#[test]
fn test_env_var_reads_the_environment_in_every_backend() {
    assert_prints_with_env(
        "بيئة_قراءة",
        concat!(
            "اطبع(متغير_بيئة(\"TARQEEM_ENV_PROBE_338\"))\n",
            "اطبع(طول(متغير_بيئة(\"TARQEEM_ENV_PROBE_338\")))",
        ),
        &[(ENV_PROBE, "مرحبا")],
        &["مرحبا", "5"],
    );
}

/// The load-bearing test. `متغير_بيئة` lowers to a plain call, so nothing but the
/// `register_builtin_return_types` entry types its result.
///
/// Measured with that entry deleted: `نوع` → `مؤشر`, `"X" + …` → `X` followed by
/// a pointer in decimal, `== "مرحبا"` → `خطأ`, and `طول` → **10 where 5 was
/// right** — the sentinel routes `ArrayLen` to `trq_array_len`, which reads
/// `TrqArray.len` at offset 0, and a `TrqString`'s field at offset 0 is its
/// *byte* length. Four of five, like `قص_حروف`. Printing alone passed, as it has
/// every time.
#[test]
fn test_env_var_result_composes_as_a_string() {
    assert_prints_with_env(
        "بيئة_تركيب",
        concat!(
            "اطبع(نوع(متغير_بيئة(\"TARQEEM_ENV_PROBE_338\")))\n",
            "اطبع(\"X\" + متغير_بيئة(\"TARQEEM_ENV_PROBE_338\"))\n",
            "اطبع(متغير_بيئة(\"TARQEEM_ENV_PROBE_338\") == \"مرحبا\")\n",
            "اطبع(طول(متغير_بيئة(\"TARQEEM_ENV_PROBE_338\")))",
        ),
        &[(ENV_PROBE, "مرحبا")],
        &["نص", "Xمرحبا", "صحيح", "5"],
    );
}

/// `""` here is a value, not a sentinel: an absent variable and an empty name
/// both have one unambiguous answer, so there is nothing for a caller to tell
/// apart. Every one of these is `trq_env_get`'s answer rather than a chosen one.
#[test]
fn test_env_var_is_total_for_an_absent_name() {
    assert_prints(
        "بيئة_غائب",
        concat!(
            "اطبع(طول(متغير_بيئة(\"TARQEEM_ABSENT_338\")))\n",
            "اطبع(متغير_بيئة(\"TARQEEM_ABSENT_338\") == \"\")\n",
            "اطبع(طول(متغير_بيئة(\"\")))\n",
            "اطبع(طول(متغير_بيئة(\"لا_يوجد_متغير_بهذا_الاسم\")))",
        ),
        &["0", "صحيح", "0", "0"],
    );
}

/// Set-but-empty answers `""` too, so it is indistinguishable from unset. That is
/// `getenv(3)`'s contract and it is deliberate; a caller needing the distinction
/// has no route to it. Only reachable with an injected variable — hence the
/// separate test from the absent cases above.
#[test]
fn test_env_var_does_not_distinguish_set_but_empty_from_unset() {
    assert_prints_with_env(
        "بيئة_فارغ",
        concat!(
            "اطبع(طول(متغير_بيئة(\"TARQEEM_ENV_EMPTY_338\")))\n",
            "اطبع(متغير_بيئة(\"TARQEEM_ENV_EMPTY_338\") == متغير_بيئة(\"TARQEEM_ABSENT_338\"))",
        ),
        &[(ENV_EMPTY, "")],
        &["0", "صحيح"],
    );
}

/// The name is read raw. `trq_env_get` does its own null/len/UTF-8 checks rather
/// than going through `string.rs`'s `as_str`, which trims — so a trimming
/// interpreter arm would answer «مرحبا» where native answers `""`, on source that
/// looks like a typo rather than a bug (#324).
#[test]
fn test_env_var_does_not_trim_the_name() {
    assert_prints_with_env(
        "بيئة_فراغات",
        concat!(
            "اطبع(طول(متغير_بيئة(\" TARQEEM_ENV_PROBE_338 \")))\n",
            "اطبع(طول(متغير_بيئة(\"TARQEEM_ENV_PROBE_338\")))",
        ),
        &[(ENV_PROBE, "مرحبا")],
        &["0", "5"],
    );
}

/// The parameter is a pointer, so the runtime's null guard is a designed answer
/// rather than the integer-zero artifact an `عدد` parameter would face (#326,
/// #327). Both routes to a null are covered: an un-narrowed `نص?`, which
/// `Type::compat` lets into a `نص` parameter, and an `أي` holder.
#[test]
fn test_env_var_accepts_a_null_holder() {
    assert_prints(
        "بيئة_لا_شيء",
        concat!(
            "متغير غائب: نص? = لا_شيء\n",
            "اطبع(طول(متغير_بيئة(غائب)))\n",
            "متغير مجهول: أي = لا_شيء\n",
            "اطبع(طول(متغير_بيئة(مجهول)))",
        ),
        &["0", "0"],
    );
}

/// Builtins are the last lookup tier, so a user definition must win in every
/// backend (#262) — including past codegen's `mangle_function_name`, which would
/// otherwise emit `@trq_env_get` for the user's own function.
#[test]
fn test_user_function_shadows_env_var() {
    assert_prints(
        "بيئة_مظلل",
        concat!(
            "دالة متغير_بيئة(اسم: نص) -> نص {\n",
            "    أرجع \"دالتي\"\n",
            "}\n",
            "اطبع(متغير_بيئة(\"PATH\"))",
        ),
        &["دالتي"],
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
        // Negative for the same reason, in the other direction: `-١` shifted by
        // 63 is `١` under zero fill and `-١` under sign fill, so this one line
        // rejects the sibling's lowering.
        (
            "بتات_إزاحة_يمين_منطقية",
            "اطبع(بتات_إزاحة_يمين_منطقية(-1، 63))",
            &["1"],
        ),
        // Two-byte Arabic letters throughout, so this one line rejects the
        // degradation a byte slicer would give: `رح` is bytes 2-5, not 1-3.
        ("قص_حروف", "اطبع(قص_حروف(\"مرحبا\"، 1، 3))", &["رحب"]),
        // A two-byte Arabic letter, so this one line rejects both plausible
        // degradations: a byte read gives 217 and a `طول`-style count gives 1.
        ("حرف_إلى_رمز", "اطبع(حرف_إلى_رمز(\"مرحبا\"))", &["1605"]),
        // A two-byte Arabic letter, so this one line rejects a byte-wise write:
        // it would emit the lead byte alone and print a replacement character.
        ("رمز_إلى_حرف", "اطبع(رمز_إلى_حرف(1605))", &["م"]),
        // A two-byte Arabic letter again, so this one line rejects the one
        // degradation that would otherwise look right: counting characters
        // instead of octets, which agrees with the byte count on all ASCII.
        ("نص_إلى_ثنائي", "اطبع(طول(نص_إلى_ثنائي(\"م\")))", &["2"]),
        // «م»'s two octets, so this one line rejects the plausible degradation:
        // reading the array as codepoints answers «Ù…» instead.
        ("ثنائي_إلى_نص", "اطبع(ثنائي_إلى_نص([217، 133]))", &["م"]),
        // A stub answering `""` for every name passes every absent-variable
        // case, so this reads one that every process has. Its value is
        // machine-dependent; that it is non-empty is not.
        (
            "متغير_بيئة",
            "اطبع(طول(متغير_بيئة(\"PATH\")) > 0)",
            &["صحيح"],
        ),
        // Writes its bytes and answers their count, so one line proves both
        // halves: «م» is two octets, and a probe that only checked the count
        // would pass on a primitive that wrote nothing.
        (
            "اكتب_مجرى",
            "اطبع(اكتب_مجرى(1، نص_إلى_ثنائي(\"م\\n\")))",
            &["م", "3"],
        ),
        // The sweep's helper gives the child a null stdin, so descriptor `٠`
        // would answer nothing here and prove little. `١` is an output stream,
        // which is a refusal the primitive decides on its own — and `طول` over
        // the answer is what proves an array came back rather than a sentinel.
        ("اقرأ_مجرى", "اطبع(طول(اقرأ_مجرى(1، 4)))", &["0"]),
        // Status `٠` so the sweep's `assert_prints` still applies, and a line
        // before it so the probe distinguishes "exited cleanly" from "never
        // ran". The status half is covered by the dedicated tests below; both
        // spellings appear here because `core_builtin_names` lists both and this
        // sweep is what refuses to let a registered name go unexercised.
        ("أنهِ_البرنامج", "اطبع(\"قبل\")\nأنهِ_البرنامج(0)", &["قبل"]),
        ("أنه_البرنامج", "اطبع(\"قبل\")\nأنه_البرنامج(0)", &["قبل"]),
        // `"."` is the one path every backend can reach without the sweep
        // supplying anything: the program's own directory exists wherever it is
        // run, so the answer does not depend on a working directory the three
        // legs do not share. A *file* row would, which is why it lives in the
        // fixture-backed tests below.
        ("حالة_مسار", "اطبع(حالة_مسار(\".\"، 0))", &["2"]),
        (
            "احذف_مسار",
            "اطبع(احذف_مسار(\"لا_يوجد_هذا_المسار\"))",
            &["خطأ"],
        ),
        // The sweep gives the child no arguments, so the empty array is the only
        // row reachable here — and it is the one row all three legs agree on
        // without the harness supplying anything, since `argv[0]` is excluded.
        // Printing the array is safe *because* it is empty: a non-empty
        // `مصفوفة<نص>` prints its elements' addresses natively (#359).
        ("معاملات_البرنامج", "اطبع(طول(معاملات_البرنامج()))", &["0"]),
        // A refusal, so the sweep needs no fixture and creates nothing: the mode
        // is settled before the path, so an unknown one never reaches the
        // filesystem. The success rows are in the section below, where a tree
        // helper can supply a file.
        ("افتح_ملف", "اطبع(افتح_ملف(\".\", 9))", &["-1"]),
        // A refusal too, and this program has opened nothing, so `3` names no
        // handle. The success rows are in the section below, where a tree helper
        // can supply a file to write and read back.
        ("اغلق_ملف", "اطبع(اغلق_ملف(3))", &["خطأ"]),
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

// ---------------------------------------------------------------------------
// أنهِ_البرنامج — الإنهاء بحالة خروج صريحة (#342)
// ---------------------------------------------------------------------------
//
// The first builtin whose observable result is the process's **exit status**
// rather than a value, so every assertion here is on `status` and on what did
// *not* get printed. Before it, no Tarqeem program could report a status of its
// own choosing: `توقف` is always 1 and always writes to stderr, and the three
// `process::exit` calls in `runtime-rs` are all hardcoded.

/// Status `٠` prints what came before and nothing after, on every backend.
///
/// The empty-stderr half of `assert_exits_with` is what this pins in particular:
/// the interpreter reaches the exit as an `Err`, so a CLI that reported it before
/// honouring it would print a «Runtime error» line the native binary never
/// prints — invisible to the CI backend-diff, which compares stdout only.
#[test]
fn test_exit_zero_terminates_quietly_in_every_backend() {
    assert_exits_with(
        "إنهاء_صفر",
        "اطبع(\"قبل\")\nأنهِ_البرنامج(٠)\nاطبع(\"بعد\")",
        0,
        &["قبل"],
    );
}

/// A non-zero status survives to the caller, which is the whole point of the
/// name — and the one thing `توقف` cannot do.
#[test]
fn test_exit_reports_a_nonzero_status_in_every_backend() {
    assert_exits_with(
        "إنهاء_ثلاثة",
        "اطبع(\"قبل\")\nأنهِ_البرنامج(٣)\nاطبع(\"بعد\")",
        3,
        &["قبل"],
    );
}

/// The masking contract, at every edge, in all three backends.
///
/// A POSIX status is eight bits and `عدد` is signed 64-bit, so the language has
/// to answer for the other 56. Masking in both `trq_exit` and the interpreter's
/// arm — rather than handing the value to the OS — is what makes the answer the
/// same everywhere: POSIX truncates to the low byte, Windows keeps all 32. These
/// cases are what stop the two one-line implementations from drifting apart.
#[test]
fn test_exit_status_masks_to_the_low_byte_in_every_backend() {
    let cases: [(&str, i32); 6] = [
        ("0", 0),
        ("3", 3),
        // The top of the range, and the value one past it: a byte's worth beyond
        // the end wraps rather than saturating or erroring.
        ("255", 255),
        ("256", 0),
        // Negative amounts fold into the same clause instead of getting their
        // own, exactly as they do in the shift range contract.
        ("-1", 255),
        ("300", 44),
    ];

    for (index, (status, expected)) in cases.iter().enumerate() {
        assert_exits_with(
            &format!("إنهاء_قناع_{index}"),
            &format!("أنهِ_البرنامج({status})"),
            *expected,
            &[],
        );
    }
}

/// `حاول` does not intercept it, and that is structural rather than guarded.
///
/// `Executor::take_propagating_exception` routes only `ErrorKind::UnhandledException`
/// to a frame's `try_stack`, so the termination signal walks past every handler.
/// Worth pinning because the interpreter carries it *as* an `Err`: if it ever
/// became a catchable one, `التقط` would swallow the exit interpreted while the
/// native binary still terminated — silent wrong output, in the exact shape this
/// project keeps finding it.
#[test]
fn test_exit_is_not_catchable_in_every_backend() {
    assert_exits_with(
        "إنهاء_داخل_حاول",
        "حاول {\n    أنهِ_البرنامج(٣)\n} التقط (خ) {\n    اطبع(\"لا ينبغي طباعته\")\n}\nاطبع(\"ولا هذا\")",
        3,
        &[],
    );
}

/// Called from inside a function, the exit ends the program rather than the call.
#[test]
fn test_exit_from_inside_a_function_ends_the_program() {
    assert_exits_with(
        "إنهاء_من_دالة",
        "دالة تحقق(س: عدد) {\n    إذا (س < ٠) {\n        أنهِ_البرنامج(٢)\n    }\n    اطبع(\"سليم\")\n}\nتحقق(٥)\nتحقق(-١)\nاطبع(\"لا ينبغي طباعته\")",
        2,
        &["سليم"],
    );
}

/// The kasra-less spelling is the same primitive, not a near-miss.
///
/// `normalize_name` is NFC only and does not strip tashkeel, so the two names are
/// distinct identifiers reaching one runtime symbol — the arrangement the keyword
/// table already uses for `ارمِ`/`ارم`.
#[test]
fn test_exit_variant_spelling_behaves_identically() {
    assert_exits_with(
        "إنهاء_بلا_كسرة",
        "اطبع(\"قبل\")\nأنه_البرنامج(٧)",
        7,
        &["قبل"],
    );
}

/// Binding the result changes nothing: the program still exits when told to.
///
/// This replaces the composition gate every primitive since #324 has carried,
/// because a `فراغ` builtin has no result to compose. The first attempt at one
/// here asserted that `متغير س = أنهِ_البرنامج(٠)` is *rejected*, and it was
/// confounded twice over: the call exits 0 before anything can be observed, and —
/// measured — the analyzer does not reject a `فراغ` result bound to a variable at
/// all. `نم` reproduces that identically, so it is the whole `Type::Void` class
/// and not this name; filed as #343 rather than fixed here.
///
/// What is left worth pinning is the risk that binding introduces: the call now
/// carries a `dest`, so a regression in how a `Void` result is typed could bind
/// or discard the wrong thing. A non-zero status is what makes the assertion
/// two-sided — status 3 can only come from the call actually running.
#[test]
fn test_exit_bound_to_a_variable_still_terminates() {
    assert_exits_with(
        "إنهاء_مربوط",
        "اطبع(\"قبل\")\nمتغير س = أنهِ_البرنامج(٣)\nاطبع(\"بعد\")",
        3,
        &["قبل"],
    );
}

/// A user declaration of the name shadows the builtin, in every backend.
///
/// Built-ins are the last lookup tier (LANGUAGE_SPEC §4.9), and the guard in
/// `expr_builder` is what keeps native in step with the interpreter — an earlier
/// version of that guard changed only one site, so native called the builtin
/// while the interpreter ran the user's function. Mirrors the `متغير_بيئة` case.
#[test]
fn test_user_function_shadows_the_exit_builtin() {
    assert_prints(
        "تظليل_إنهاء",
        "دالة أنهِ_البرنامج(حالة: عدد) {\n    اطبع(\"دالتي \" + حالة)\n}\nأنهِ_البرنامج(٣)\nاطبع(\"وصلنا\")",
        &["دالتي 3", "وصلنا"],
    );
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

// ---------------------------------------------------------------------------
// اكتب_مجرى — the write(2) primitive (#347)
// ---------------------------------------------------------------------------

/// Writing bytes and counting them, in one program, because either half alone
/// admits a primitive that does not work: a count with no write, or a write
/// whose answer is invented.
///
/// The value is Arabic on purpose. `طول("مرحبا")` is 5 and the write is 10
/// octets, so the two numbers stand side by side and a primitive that counted
/// characters would be caught. On an ASCII value they agree and the assertion
/// could not fail — the lesson #338 recorded for a string length, which holds
/// for a count too.
#[test]
fn test_write_stream_writes_bytes_and_counts_them_in_every_backend() {
    assert_prints(
        "مجرى_كتابة",
        concat!(
            "اطبع(اكتب_مجرى(1، نص_إلى_ثنائي(\"مرحبا\\n\")))\n",
            "اطبع(طول(\"مرحبا\"))",
        ),
        &["مرحبا", "11", "5"],
    );
}

/// Raw bytes, no newline of its own: two writes with no line break between them
/// land on one line, and the count is what separates them.
///
/// This is the difference from `اطبع`, asserted rather than described — `اطبع`
/// terminates its output and this does not.
#[test]
fn test_write_stream_appends_no_newline_of_its_own() {
    assert_prints(
        "مجرى_بلا_سطر",
        concat!(
            "اكتب_مجرى(1، نص_إلى_ثنائي(\"أ\"))\n",
            "اكتب_مجرى(1، نص_إلى_ثنائي(\"ب\\n\"))\n",
            "اطبع(\"تمّ\")",
        ),
        &["أب", "تمّ"],
    );
}

/// The load-bearing test. `اكتب_مجرى` lowers to a plain `Call`, so nothing but
/// the `register_builtin_return_types` entry types its result.
///
/// Measured with that entry deleted, and it fails in two ways at once — neither
/// of them the wrong-number mode the array and string primitives produce. `== ٠`
/// and `+ ١` make **native compilation fail** (ت٠١٠١, clang: «'%v13' defined
/// with type 'i64' but expected 'ptr'»): a scalar has no struct for the
/// `Ptr(Void)` sentinel to misread, and `icmp`/`add` on a `ptr` is not valid IR
/// at all. `نوع` answers `مؤشر`, as it has for every name before this one. And
/// `اطبع` is **quieter** here than anywhere else — it prints *nothing* for the
/// count, taking the pointer path, where `ثنائي_إلى_نص` at least printed a
/// pointer in decimal and `قص_حروف` a wrong length.
///
/// So the prediction rule from #336 generalises past struct layouts: predict the
/// failure from the *return type's representation*. A scalar's missing entry is
/// caught at build time by any arithmetic, and printing catches nothing.
#[test]
fn test_write_stream_result_composes_as_an_integer() {
    assert_prints(
        "مجرى_تركيب",
        concat!(
            "اطبع(نوع(اكتب_مجرى(1، نص_إلى_ثنائي(\"م\\n\"))))\n",
            "اطبع(اكتب_مجرى(1، نص_إلى_ثنائي(\"م\\n\")) + 1)\n",
            "اطبع(اكتب_مجرى(1، نص_إلى_ثنائي(\"م\\n\")) == 3)",
        ),
        &["م", "عدد", "م", "4", "م", "صحيح"],
    );
}

/// `٢` is stderr, and this is the one contract row `assert_prints` cannot check:
/// it compares stdout. So the streams are read apart, and both halves are
/// asserted — the bytes are on stderr **and** absent from stdout.
///
/// Worth separating from `اطبع_خطأ`, which is excluded from the cross-backend
/// sweep precisely because it disagrees about the stream: the interpreter prints
/// it to stdout and native to stderr (#286). This primitive does not inherit
/// that, and this test is what says so.
#[test]
fn test_write_stream_sends_descriptor_two_to_stderr_in_every_backend() {
    let temp = TempDir::new().unwrap();
    let main = write_program(
        temp.path(),
        "مجرى_خطأ",
        concat!(
            "اطبع(\"مخرج\")\n",
            "اطبع(اكتب_مجرى(2، نص_إلى_ثنائي(\"خطأ\\n\")))",
        ),
    );

    for backend in Backend::ALL {
        let output = execute(backend, &main, &format!("مجرى_خطأ_{backend:?}"));

        assert!(
            output.succeeded(),
            "فشل التنفيذ [{backend:?}]\n{}",
            output.report()
        );
        // The count reaches stdout, the bytes do not.
        assert_eq!(
            output.lines(),
            &["مخرج", "7"],
            "المجرى ٢ كتب في المخرج القياسي [{backend:?}]\n{}",
            output.report()
        );
        assert!(
            output.stderr.contains("خطأ"),
            "المجرى ٢ لم يكتب في مجرى الخطأ [{backend:?}]\n{}",
            output.report()
        );
    }
}

/// Every descriptor that names nowhere to write, answering `-١`.
///
/// `٠` is stdin, so writing to it is an error rather than an alias for stdout.
/// `٣` upward names a file handle, and this program opens none, so the table is
/// empty — both interpreters and the runtime agree on `-١` here for the same
/// reason, not by coincidence. Since #362 that is a property of *this program*
/// rather than of the language: `افتح_ملف` can put a live handle at `٣`, and the
/// section for it asserts what happens then. A negative descriptor folds into the
/// same clause instead of getting its own.
#[test]
fn test_write_stream_refuses_a_descriptor_it_cannot_write_to() {
    assert_prints(
        "مجرى_معرّف",
        concat!(
            "اطبع(اكتب_مجرى(0، نص_إلى_ثنائي(\"س\")))\n",
            "اطبع(اكتب_مجرى(3، نص_إلى_ثنائي(\"س\")))\n",
            "اطبع(اكتب_مجرى(99، نص_إلى_ثنائي(\"س\")))\n",
            "اطبع(اكتب_مجرى(-1، نص_إلى_ثنائي(\"س\")))",
        ),
        &["-1", "-1", "-1", "-1"],
    );
}

/// An element outside `0..=255` is not a byte, and the whole call is refused
/// rather than the value truncated to its low byte.
///
/// Truncation is what makes this load-bearing rather than a taste: `٣٠٠` would
/// answer as `٤٤` — the comma — so a rejected array and an accepted one would
/// produce the same output, and there would be no way to tell them apart. Same
/// reasoning as `ثنائي_إلى_نص` (#333).
///
/// The rejection is total: nothing is written. `[٦٥، ٣٠٠]` is the case that
/// shows it — the `A` before the bad element does not reach the stream, so the
/// `-١` sits alone on its line.
///
/// The accepted boundaries go to **stderr**, not stdout, and that is not
/// squeamishness: `٢٥٥` and `٠` are not valid UTF-8 on their own, and this
/// primitive writes them raw where every print builtin would drop them
/// (`trq_print` is `if let Ok(text) = from_utf8`). Sending them to stdout would
/// make this test compare a line that is not text. The count still proves they
/// were accepted, and it arrives on stdout.
#[test]
fn test_write_stream_rejects_a_value_that_is_not_a_byte() {
    assert_prints(
        "مجرى_بايت",
        concat!(
            "اطبع(اكتب_مجرى(1، [300]))\n",
            "اطبع(اكتب_مجرى(1، [-1]))\n",
            "اطبع(اكتب_مجرى(1، [256]))\n",
            "اطبع(اكتب_مجرى(1، [65، 300]))\n",
            "اطبع(اكتب_مجرى(2، [255]))\n",
            "اطبع(اكتب_مجرى(2، [0]))",
        ),
        &["-1", "-1", "-1", "-1", "1", "1"],
    );
}

/// The bytes are written raw, valid UTF-8 or not — `write(2)` does not inspect
/// what it carries, and neither does this.
///
/// It is the one thing no print builtin can do: `trq_print` decodes first and
/// silently prints nothing when the decode fails, so a lone `٢٥٥` is
/// unreachable through `اطبع`. Asserted on stderr and read back as bytes,
/// because the moment it were on stdout the harness's own comparison would be
/// comparing something that is not a string.
#[test]
fn test_write_stream_writes_bytes_that_are_not_text() {
    let temp = TempDir::new().unwrap();
    let main = write_program(temp.path(), "مجرى_غير_نص", "اطبع(اكتب_مجرى(2، [255، 254]))");

    for backend in Backend::ALL {
        let output = execute(backend, &main, &format!("مجرى_غير_نص_{backend:?}"));

        assert!(
            output.succeeded(),
            "فشل التنفيذ [{backend:?}]\n{}",
            output.report()
        );
        assert_eq!(
            output.lines(),
            &["2"],
            "عدد البايتات [{backend:?}]\n{}",
            output.report()
        );
        // `from_utf8_lossy` turned each undecodable byte into U+FFFD, which is
        // proof they arrived: dropping them would leave stderr empty.
        assert_eq!(
            output.stderr.chars().filter(|c| *c == '\u{FFFD}').count(),
            2,
            "البايتان غير القابلين للترميز لم يصلا [{backend:?}]\n{}",
            output.report()
        );
    }
}

/// Nothing to write is a count of zero, not a failure — `٠` is a value here the
/// way an empty array is `نص_إلى_ثنائي("")`'s value.
///
/// `لا_شيء` answers the same, and that costs nothing: both mean nothing was
/// written, so giving them one answer loses no information a caller could use.
/// It is reached through an `أي` holder, since `مصفوفة<عدد>؟` does not parse
/// (ب٠١٠١) and a bare `لا_شيء` is refused at the argument — the route #333
/// found for the same shape.
#[test]
fn test_write_stream_of_nothing_answers_zero() {
    assert_prints(
        "مجرى_فارغ",
        concat!(
            "اطبع(اكتب_مجرى(1، []))\n",
            "اطبع(اكتب_مجرى(1، نص_إلى_ثنائي(\"\")))\n",
            "متغير غائب: أي = لا_شيء\n",
            "اطبع(اكتب_مجرى(1، غائب))",
        ),
        &["0", "0", "0"],
    );
}

/// The bytes survive the round trip: what `نص_إلى_ثنائي` produced is what the
/// stream received, for every UTF-8 width.
///
/// Asserted by reading the written bytes back as output rather than by trusting
/// the count — a primitive that wrote the right *number* of wrong bytes would
/// pass the count assertions above.
#[test]
fn test_write_stream_writes_the_bytes_it_was_given() {
    assert_prints(
        "مجرى_وفاء",
        concat!(
            "اكتب_مجرى(1، نص_إلى_ثنائي(\"A﷽م𞸀\\n\"))\n",
            "اطبع(اكتب_مجرى(1، [72، 105، 10]))",
        ),
        &["A﷽م𞸀", "Hi", "3"],
    );
}

/// `لا_شيء` as a descriptor is a type error, not a write to stream zero.
///
/// The parameter is an `عدد`, so there is no pointer for a runtime guard to
/// answer and codegen turns `لا_شيء` into `0` above the runtime; mirroring that
/// would encode the artifact as contract. #326's narrowing, and the same choice
/// `أنهِ_البرنامج` made — `رمز_إلى_حرف` and `نم` diverge identically on the same
/// source (#327).
#[test]
fn test_write_stream_refuses_an_absent_descriptor() {
    let temp = TempDir::new().unwrap();
    let main = write_program(
        temp.path(),
        "مجرى_معرّف_غائب",
        "متغير غائب: أي = لا_شيء\nاطبع(اكتب_مجرى(غائب، [65]))",
    );

    // Native refuses the `أي` parameter before it runs, so only the two
    // interpreters can be asked; both must refuse rather than write.
    for backend in [Backend::Interpreter, Backend::Jit] {
        let output = execute(backend, &main, &format!("مجرى_معرّف_غائب_{backend:?}"));
        assert!(
            !output.succeeded(),
            "توقّعنا خطأ نوع [{backend:?}]\n{}",
            output.report()
        );
    }
}

/// A user function named `اكتب_مجرى` shadows the builtin, like every other core
/// name: builtins are the last lookup tier, not reserved words
/// (LANGUAGE_SPEC §4.9).
#[test]
fn test_user_function_shadows_write_stream() {
    assert_prints(
        "مجرى_تظليل",
        concat!(
            "دالة اكتب_مجرى(مجرى: عدد، بايتات: مصفوفة<عدد>) -> عدد {\n",
            "    أرجع 42\n",
            "}\n",
            "اطبع(اكتب_مجرى(1، [65]))",
        ),
        &["42"],
    );
}

// ---------------------------------------------------------------------------
// اقرأ_مجرى — the read(2) primitive (#350)
// ---------------------------------------------------------------------------

/// Reads stdin and answers the bytes, in all three backends.
///
/// The Arabic value is not decoration: it is what separates the byte count from
/// the character count. «مرحبا» is five characters and ten octets, so a
/// primitive that answered characters — or a `طول` reading the wrong field —
/// would print `5` here. #338 measured that an ASCII value silently loses that
/// catcher, and a byte primitive tested on ASCII would lose it twice over.
#[test]
fn test_read_stream_reads_stdin_in_every_backend() {
    assert_prints_with_stdin(
        "قراءة_مجرى",
        concat!(
            "متغير بايتات = اقرأ_مجرى(0، 32)\n",
            "اطبع(طول(بايتات))\n",
            "اطبع(ثنائي_إلى_نص(بايتات))\n",
            "اطبع(طول(ثنائي_إلى_نص(بايتات)))",
        ),
        "مرحبا".as_bytes(),
        &["10", "مرحبا", "5"],
    );
}

/// The load-bearing test, and it runs over bytes that were actually read.
///
/// `اقرأ_مجرى` lowers to a plain `Call`, so nothing but the
/// `register_builtin_return_types` entry types its result. #330 measured that a
/// missing entry on an array return is **quiet** — "only `نوع` catches it".
/// Measured here with the entry deleted, that does not hold for this name, and
/// the reason is instructive: what catches it depends on what the caller does
/// with the **elements**, not on the return type.
///
/// `نوع` answers `مؤشر` in all three backends. `طول` answers `10` — right either
/// way, since `ArrayLen` routes to `trq_array_len` regardless — and printing the
/// whole array is *silent* wrong output, correct in the interpreters and empty
/// natively. But printing an indexed element **aborts** the native binary
/// («misaligned pointer dereference … 0x41»: with `Ptr(Void)` the element is a
/// pointer, so `trq_print` dereferences the byte value), and `+ ١` or `== ٦٨` on
/// one makes native **compilation fail** with ت٠١٠١.
///
/// Which is why this test indexes, adds and compares rather than printing the
/// array — and why it must not run over a refusal: an empty answer cannot be
/// indexed, and `طول` answers `0` with or without the entry, so every assertion
/// would pass on a sentinel.
#[test]
fn test_read_stream_result_composes_as_a_byte_array() {
    assert_prints_with_stdin(
        "قراءة_تركيب",
        concat!(
            "متغير بايتات = اقرأ_مجرى(0، 4)\n",
            "اطبع(نوع(بايتات))\n",
            "اطبع(بايتات[0])\n",
            "اطبع(بايتات[0] + 1)\n",
            "اطبع(بايتات[3] == 68)\n",
            "اطبع(نوع(بايتات[0]))",
        ),
        b"ABCD",
        &["مصفوفة", "65", "66", "صحيح", "عدد"],
    );
}

/// Successive calls continue where the last one stopped, and each answers
/// exactly what it asked for while there is that much left.
#[test]
fn test_read_stream_reads_the_stream_in_order() {
    assert_prints_with_stdin(
        "قراءة_تتابع",
        concat!(
            "متغير أول = اقرأ_مجرى(0، 2)\n",
            "متغير ثان = اقرأ_مجرى(0، 3)\n",
            "اطبع(ثنائي_إلى_نص(أول))\n",
            "اطبع(ثنائي_إلى_نص(ثان))\n",
            "اطبع(طول(اقرأ_مجرى(0، 9)))",
        ),
        b"abcdef",
        // Two, then three, then the single byte that was left.
        &["ab", "cde", "1"],
    );
}

/// The property §1.3's row exists for: a multi-byte character split across two
/// reads survives, because the primitive moves octets and never decodes.
///
/// «مرحبا» is `D9 85 D8 B1 …`, so three bytes cuts «ر» in half. The first read
/// ends on `216` and the second begins on `177` — the two halves of that
/// character, contiguous across the boundary with nothing dropped or substituted.
/// `ثنائي_إلى_نص` of the truncated piece answers `""`, which is the point:
/// decoding is the caller's business and happens once, after the bytes are whole.
#[test]
fn test_read_stream_splits_a_codepoint_without_losing_it() {
    assert_prints_with_stdin(
        "قراءة_حدود",
        concat!(
            "متغير أول = اقرأ_مجرى(0، 3)\n",
            "متغير بقية = اقرأ_مجرى(0، 7)\n",
            "اطبع(طول(أول))\n",
            "اطبع(أول[2])\n",
            "اطبع(بقية[0])\n",
            // The straddled character cannot be decoded from either piece.
            "اطبع(طول(ثنائي_إلى_نص(أول)))\n",
            "اطبع(طول(بقية))",
        ),
        "مرحبا".as_bytes(),
        &["3", "216", "177", "0", "7"],
    );
}

/// Asking for more than the stream holds answers what there was, and the next
/// call answers nothing. That is what "loops until the count or EOF" means, and
/// it is why a short answer has exactly one meaning.
#[test]
fn test_read_stream_stops_at_end_of_stream() {
    assert_prints_with_stdin(
        "قراءة_نهاية",
        concat!(
            "اطبع(طول(اقرأ_مجرى(0، 100)))\n",
            "اطبع(طول(اقرأ_مجرى(0، 100)))",
        ),
        b"abc",
        &["3", "0"],
    );
}

/// An empty stdin is EOF from the first call. Uses the plain `assert_prints`,
/// whose child gets a **null** stdin rather than the parent's terminal — which is
/// why this row needs no piping at all.
#[test]
fn test_read_stream_answers_nothing_on_an_empty_stream() {
    assert_prints("قراءة_فراغ", "اطبع(طول(اقرأ_مجرى(0، 4)))", &["0"]);
}

/// Reading `ن` bytes at once and `ن` times one byte answer the same thing.
///
/// The equivalence shape #322, #333 and #336 used. Here it is not a check against
/// a hand-written alternative but against the primitive's own contract: if the
/// read did not loop, the batched call could answer short and the two columns
/// would part.
#[test]
fn test_read_stream_matches_the_loop_it_names() {
    let expected = &["97", "98", "99", "100"];

    assert_prints_with_stdin(
        "قراءة_دفعة",
        concat!(
            "متغير كل = اقرأ_مجرى(0، 4)\n",
            "لكل (متغير ي = 0؛ ي < طول(كل)؛ ي++) {\n",
            "    اطبع(كل[ي])\n",
            "}",
        ),
        b"abcd",
        expected,
    );

    assert_prints_with_stdin(
        "قراءة_بايت_بايت",
        concat!(
            "لكل (متغير ي = 0؛ ي < 4؛ ي++) {\n",
            "    متغير واحد = اقرأ_مجرى(0، 1)\n",
            "    اطبع(واحد[0])\n",
            "}",
        ),
        b"abcd",
        expected,
    );
}

/// Bytes that are not text arrive intact, which no input path had before: `ادخل`
/// decodes to a `نص`, so a byte outside UTF-8 could not survive it. The mirror of
/// `اكتب_مجرى` putting a lone `٢٥٥` on stdout.
#[test]
fn test_read_stream_reads_bytes_that_are_not_text() {
    assert_prints_with_stdin(
        "قراءة_غير_نص",
        concat!(
            "متغير بايتات = اقرأ_مجرى(0، 3)\n",
            "اطبع(بايتات[0])\n",
            "اطبع(بايتات[1])\n",
            "اطبع(بايتات[2])\n",
            // Not an encoding, so decoding refuses rather than inventing text.
            "اطبع(طول(ثنائي_إلى_نص(بايتات)))",
        ),
        &[0xFF, 0x00, 0x41],
        &["255", "0", "65", "0"],
    );
}

/// The refusal rows. `١` and `٢` carry bytes the other way, `٣` upward names a
/// handle *this program* has not opened (#362 made that reachable, so it is no
/// longer a property of the language), and a negative descriptor names nothing.
///
/// All four answer the same empty array EOF does — an array return has no value
/// to spare for a sentinel, the way `اكتب_مجرى` has `-١`. The stdin bytes are
/// piped and deliberately left unread: they prove the refusals are decided on the
/// descriptor and do not fall through to stdin.
#[test]
fn test_read_stream_refuses_a_stream_it_cannot_read() {
    assert_prints_with_stdin(
        "قراءة_مرفوضة",
        concat!(
            "اطبع(طول(اقرأ_مجرى(1، 4)))\n",
            "اطبع(طول(اقرأ_مجرى(2، 4)))\n",
            "اطبع(طول(اقرأ_مجرى(3، 4)))\n",
            "اطبع(طول(اقرأ_مجرى(-1، 4)))\n",
            // Untouched, so it is all still there.
            "اطبع(طول(اقرأ_مجرى(0، 4)))",
        ),
        b"abcd",
        &["0", "0", "0", "0", "4"],
    );
}

/// A non-positive count answers nothing **and consumes nothing**, which the next
/// call proves: the bytes are still there.
#[test]
fn test_read_stream_of_nothing_consumes_nothing() {
    assert_prints_with_stdin(
        "قراءة_صفر",
        concat!(
            "اطبع(طول(اقرأ_مجرى(0، 0)))\n",
            "اطبع(طول(اقرأ_مجرى(0، -5)))\n",
            "اطبع(ثنائي_إلى_نص(اقرأ_مجرى(0، 3)))",
        ),
        b"abc",
        &["0", "0", "abc"],
    );
}

/// A user function named `اقرأ_مجرى` shadows the builtin, like every other core
/// name: builtins are the last lookup tier, not reserved words
/// (LANGUAGE_SPEC §4.9).
#[test]
fn test_user_function_shadows_read_stream() {
    assert_prints(
        "قراءة_تظليل",
        concat!(
            "دالة اقرأ_مجرى(مجرى: عدد، عدد_البايتات: عدد) -> مصفوفة<عدد> {\n",
            "    أرجع [42]\n",
            "}\n",
            "اطبع(اقرأ_مجرى(0، 4)[0])",
        ),
        &["42"],
    );
}

// ────────────────────────────────────────────────────────────────────────────
// حالة_مسار — `stat(2)`, one field per call (#352)
// ────────────────────────────────────────────────────────────────────────────
//
// Every row here is pinned in all three backends for a reason this family has
// not had before: the kind/size mapping exists **twice** — once in
// `trq_path_status` and once in `call_path_status` — because the compiler crate
// does not depend on `tarqeem-runtime` and an `extern "C"` function taking a
// `*const TrqString` could not read a `Value` anyway. Nothing but these tests
// stops the two copies from drifting apart.

/// A directory answers its kind and **no** size.
///
/// `"."` is the fixture the harness does not have to supply: the program's own
/// directory exists wherever the program runs, so the row does not depend on a
/// working directory the three legs would have to share.
///
/// The `-١` for the size is the deliberate delta from `trq_file_size`, which
/// answers the OS `st_size` here — 4096 on ext4, 64–96 on APFS. A number that
/// changes with the filesystem could not be asserted at all.
#[test]
fn test_path_status_answers_a_directory_without_a_size() {
    assert_prints(
        "حالة_مجلد",
        concat!("اطبع(حالة_مسار(\".\"، 0))\n", "اطبع(حالة_مسار(\".\"، 1))",),
        &["2", "-1"],
    );
}

/// A regular file: kind `١`, and the size in **bytes**.
///
/// The fixture's content is Arabic on purpose. «مرحبا» is five characters and
/// ten bytes, so an implementation that counted characters — or a missing
/// `register_builtin_return_types` entry routing `ArrayLen` at a `TrqString`
/// header — would pass this with an ASCII fixture and fail it here. The same
/// catcher #338 relied on.
#[test]
fn test_path_status_reads_a_regular_file_and_its_byte_size() {
    assert_prints_with_files(
        "حالة_ملف",
        &[("بيانات.نص", "مرحبا")],
        concat!(
            "اطبع(حالة_مسار(\"{مسار}\"، 0))\n",
            "اطبع(حالة_مسار(\"{مسار}\"، 1))",
        ),
        &["1", "10"],
    );
}

/// An absent path and an empty name are one answer, in both fields.
#[test]
fn test_path_status_reads_nothing_as_absent() {
    assert_prints(
        "حالة_معدوم",
        concat!(
            "اطبع(حالة_مسار(\"لا_يوجد_هذا_المسار\"، 0))\n",
            "اطبع(حالة_مسار(\"لا_يوجد_هذا_المسار\"، 1))\n",
            "اطبع(حالة_مسار(\"\"، 0))\n",
            "اطبع(حالة_مسار(\"\"، 1))",
        ),
        &["0", "-1", "0", "-1"],
    );
}

/// A field this function does not know has no answer, whatever the path holds —
/// `"."` exists and is readable, and every one of these is `-١`.
///
/// The field is settled before the path, so an unknown field never reaches the
/// filesystem; that ordering is invisible from here, which is why the two
/// implementations state it in the same words.
#[test]
fn test_path_status_has_no_answer_for_an_unknown_field() {
    assert_prints(
        "حالة_حقل_مجهول",
        concat!(
            "اطبع(حالة_مسار(\".\"، 2))\n",
            "اطبع(حالة_مسار(\".\"، 9))\n",
            "اطبع(حالة_مسار(\".\"، -1))",
        ),
        &["-1", "-1", "-1"],
    );
}

/// A null path reads as absent through **both** routes that can produce one.
///
/// An un-narrowed `نص?` gets in through `Type::compat` (#324) and native lowers
/// it to `ptr null`, where the runtime's guard answers; an `أي` holder is the
/// other route, and the one that still works where the optional syntax does not
/// (#333). The `عدد` field deliberately has no such arm — there native turns
/// `لا_شيء` into `0` as an artifact of the call path, not as a designed answer
/// (#326, #327).
#[test]
fn test_path_status_reads_a_null_path_as_absent() {
    assert_prints(
        "حالة_لا_شيء",
        concat!(
            "متغير غائب: نص? = لا_شيء\n",
            "اطبع(حالة_مسار(غائب، 0))\n",
            "اطبع(حالة_مسار(غائب، 1))\n",
            "متغير مجهول: أي = لا_شيء\n",
            "اطبع(حالة_مسار(مجهول، 0))\n",
            "اطبع(حالة_مسار(مجهول، 1))",
        ),
        &["0", "-1", "0", "-1"],
    );
}

/// **The load-bearing test.** `حالة_مسار` lowers to a plain call, so nothing but
/// the `register_builtin_return_types` entry types its result.
///
/// Measured with that entry deleted, and #347's prediction for a *scalar* return
/// held exactly — where #330's prediction for an array did not survive a second
/// array (#350):
///
/// | use | interpreters | native |
/// |---|---|---|
/// | `اطبع(…)` | `2` | prints **nothing**, exit 0 |
/// | `نوع(…)` | `مؤشر` | `مؤشر` |
/// | `… + ١` | `3` | **compile failure** ت٠١٠١ |
/// | `… == ٢` | `صحيح` | **compile failure** ت٠١٠١ |
///
/// So printing alone passes either way, and the two arithmetic rows are what
/// make the entry non-optional. Which is also why this runs over `"."` — a real
/// answer — rather than over a refusal: `-١` composes just as well as `٢`, but a
/// row that answers `0` would not distinguish a sentinel from a value (#350).
#[test]
fn test_path_status_result_composes_as_an_integer() {
    assert_prints(
        "حالة_تركيب",
        concat!(
            "اطبع(نوع(حالة_مسار(\".\"، 0)))\n",
            "اطبع(حالة_مسار(\".\"، 0) + 1)\n",
            "اطبع(حالة_مسار(\".\"، 0) == 2)",
        ),
        &["عدد", "3", "صحيح"],
    );
}

/// The name opens with `حالة`, which is `TokenKind::Case` — and unlike every
/// other keyword embedded in a builtin name, that one is reserved **only** inside
/// a `تطابق` block.
///
/// The lexer test pins that the name stays one token; this pins the half the
/// lexer cannot reach, which is the parser accepting it in the one construct
/// where the token it embeds *is* a keyword. Both the scrutinee and an arm body
/// call it, since those are the two positions inside `تطابق` where an expression
/// appears.
#[test]
fn test_path_status_is_callable_inside_a_match() {
    assert_prints(
        "حالة_داخل_تطابق",
        concat!(
            "تطابق (حالة_مسار(\".\"، 0)) {\n",
            "    حالة 1 => اطبع(\"ملف\")\n",
            "    حالة 2 => اطبع(حالة_مسار(\".\"، 1))\n",
            "    غير_ذلك => اطبع(\"غير ذلك\")\n",
            "}",
        ),
        &["-1"],
    );
}

/// The four names this one folds, written as the stdlib wrappers they become —
/// which is the whole case for the primitive, executed rather than asserted.
///
/// `هل_موجود` is `!= ٠` and not `== ١`, and that is the point of the fourth kind:
/// `ملف_موجود` is `Path::exists()`, true for a device, while `هل_ملف` is false
/// for the same path. Three values could not answer for both.
#[test]
fn test_path_status_folds_the_four_file_predicates() {
    assert_prints_with_files(
        "حالة_أغلفة",
        &[("بيانات.نص", "abc")],
        concat!(
            "دالة هل_موجود(م: نص) -> منطقي { أرجع حالة_مسار(م، 0) != 0 }\n",
            "دالة هل_ملف(م: نص) -> منطقي { أرجع حالة_مسار(م، 0) == 1 }\n",
            "دالة هل_مجلد(م: نص) -> منطقي { أرجع حالة_مسار(م، 0) == 2 }\n",
            "دالة حجم(م: نص) -> عدد { أرجع حالة_مسار(م، 1) }\n",
            "اطبع(هل_موجود(\"{مسار}\"))\n",
            "اطبع(هل_ملف(\"{مسار}\"))\n",
            "اطبع(هل_مجلد(\"{مسار}\"))\n",
            "اطبع(حجم(\"{مسار}\"))\n",
            "اطبع(هل_موجود(\".\"))\n",
            "اطبع(هل_ملف(\".\"))\n",
            "اطبع(هل_مجلد(\".\"))",
        ),
        &["صحيح", "صحيح", "خطأ", "3", "صحيح", "خطأ", "صحيح"],
    );
}

/// The fourth kind, from Tarqeem source: `/dev/null` exists and is neither a
/// file nor a directory.
///
/// Unix-only, and therefore not in `examples/مدمجات.ترقيم` — the golden file is
/// regenerated on a developer machine and a Windows contributor would produce a
/// different one.
#[test]
#[cfg(unix)]
fn test_path_status_marks_a_device_as_neither_file_nor_directory() {
    assert_prints(
        "حالة_جهاز",
        concat!(
            "اطبع(حالة_مسار(\"/dev/null\"، 0))\n",
            "اطبع(حالة_مسار(\"/dev/null\"، 1))",
        ),
        &["3", "-1"],
    );
}

/// A user function named `حالة_مسار` shadows the builtin, like every other core
/// name: builtins are the last lookup tier, not reserved words
/// (LANGUAGE_SPEC §4.9).
#[test]
fn test_user_function_shadows_path_status() {
    assert_prints(
        "حالة_تظليل",
        concat!(
            "دالة حالة_مسار(م: نص، حقل: عدد) -> عدد {\n",
            "    أرجع 99\n",
            "}\n",
            "اطبع(حالة_مسار(\".\"، 0))",
        ),
        &["99"],
    );
}

// ────────────────────────────────────────────────────────────────────────────
// احذف_مسار — `unlink(2)` for a file, `rmdir(2)` for an empty directory (#355)
// ────────────────────────────────────────────────────────────────────────────
//
// Pinned cross-backend for `حالة_مسار`'s reason and one more of its own. The
// lstat-then-choose mapping exists **twice** — in `trq_path_delete` and in
// `call_path_delete` — for the reason that block records. And this primitive has
// an *effect*, so a test that only reads the return value cannot tell a working
// implementation from one that answers `صحيح` and deletes nothing: every row that
// should remove something asks `حالة_مسار` afterwards.

/// The effect gate. A `منطقي` return composes weakly on its own, so the sibling
/// primitive supplies the proof that the file is actually gone — in all three
/// backends, not just in the one that happens to run first.
#[test]
fn test_path_delete_removes_a_file_and_the_sibling_sees_it_go() {
    assert_prints_with_files(
        "حذف_ملف",
        &[("بيانات.نص", "مرحبا")],
        concat!(
            "اطبع(حالة_مسار(\"{مسار}\"، 0))\n",
            "اطبع(احذف_مسار(\"{مسار}\"))\n",
            "اطبع(حالة_مسار(\"{مسار}\"، 0))",
        ),
        &["1", "صحيح", "0"],
    );
}

/// An empty directory goes the same way, through `rmdir` rather than `unlink`.
#[test]
fn test_path_delete_removes_an_empty_directory() {
    assert_prints_with_tree(
        "حذف_مجلد",
        &[("فارغ", Fixture::EmptyDir)],
        concat!(
            "اطبع(حالة_مسار(\"{مسار}\"، 0))\n",
            "اطبع(احذف_مسار(\"{مسار}\"))\n",
            "اطبع(حالة_مسار(\"{مسار}\"، 0))",
        ),
        &["2", "صحيح", "0"],
    );
}

/// **The contract decision, executed.** `حالة_مسار` follows the link and answers
/// `٢`, so a `stat`-based selector would call `rmdir` on it and fail. This unlinks
/// the link and leaves the directory it named — the row that would silently drift
/// if either copy of the kernel switched to `metadata`.
#[test]
#[cfg(unix)]
fn test_path_delete_unlinks_a_symlink_to_a_directory_and_spares_its_target() {
    assert_prints_with_tree(
        "حذف_وصلة",
        &[
            ("هدف", Fixture::EmptyDir),
            ("وصلة", Fixture::Symlink { to: "هدف" }),
        ],
        concat!(
            "اطبع(حالة_مسار(\"{مسار2}\"، 0))\n",
            "اطبع(احذف_مسار(\"{مسار2}\"))\n",
            "اطبع(حالة_مسار(\"{مسار2}\"، 0))\n",
            "اطبع(حالة_مسار(\"{مسار}\"، 0))",
        ),
        &["2", "صحيح", "0", "2"],
    );
}

/// A broken link is removable, and this is the row a `stat`-based selector could
/// never reach at all: `حالة_مسار` reads it as **absent**, so a selector that
/// asked would find nothing to delete and strand the link permanently.
#[test]
#[cfg(unix)]
fn test_path_delete_unlinks_a_broken_symlink() {
    assert_prints_with_tree(
        "حذف_وصلة_مقطوعة",
        &[(
            "معلقة",
            Fixture::Symlink {
                to: "لا_يوجد_هدف"
            },
        )],
        concat!(
            "اطبع(حالة_مسار(\"{مسار}\"، 0))\n",
            "اطبع(احذف_مسار(\"{مسار}\"))\n",
            "اطبع(احذف_مسار(\"{مسار}\"))",
        ),
        &["0", "صحيح", "خطأ"],
    );
}

/// `rmdir`, not `rm -r`. `"."` is a non-empty directory wherever a program runs,
/// and POSIX refuses `rmdir(".")` regardless — so the row needs no fixture and
/// cannot delete anything.
#[test]
fn test_path_delete_refuses_a_non_empty_directory() {
    assert_prints(
        "حذف_مجلد_عامر",
        concat!("اطبع(احذف_مسار(\".\"))\n", "اطبع(حالة_مسار(\".\"، 0))",),
        &["خطأ", "2"],
    );
}

/// An absent path and an empty name are one answer, as they are for `حالة_مسار`.
#[test]
fn test_path_delete_removes_nothing_that_is_not_there() {
    assert_prints(
        "حذف_معدوم",
        concat!(
            "اطبع(احذف_مسار(\"لا_يوجد_هذا_المسار\"))\n",
            "اطبع(احذف_مسار(\"\"))",
        ),
        &["خطأ", "خطأ"],
    );
}

/// `لا_شيء` answers `خطأ` rather than raising. Both spellings of a null reach the
/// arm: an un-narrowed `نص?` through `Type::compat`, and an `أي` holder (#333).
#[test]
fn test_path_delete_reads_a_null_path_as_nothing() {
    assert_prints(
        "حذف_لا_شيء",
        concat!(
            "متغير غائب: نص? = لا_شيء\n",
            "متغير مجهول: أي = لا_شيء\n",
            "اطبع(احذف_مسار(غائب))\n",
            "اطبع(احذف_مسار(مجهول))",
        ),
        &["خطأ", "خطأ"],
    );
}

/// The two names this folds, written as the stdlib wrappers they become.
///
/// Both carry a **documented delta** at one edge: the only kind they can ask for
/// comes from `حالة_مسار`, which follows symlinks, so a symlink-to-directory is
/// refused by `احذف_ملف` where `remove_file` succeeds today, and accepted by
/// `احذف_مجلد` where `remove_dir` fails. One edge, two faces. Blast radius is
/// nil: neither name has an interpreter arm today, so neither ever worked outside
/// native compilation.
#[test]
fn test_path_delete_folds_the_two_delete_names() {
    assert_prints_with_tree(
        "حذف_أغلفة",
        &[("و.نص", Fixture::File("x")), ("د", Fixture::EmptyDir)],
        concat!(
            "دالة احذف_ملف(م: نص) -> منطقي {\n",
            "    إذا (حالة_مسار(م، 0) == 2) { أرجع خطأ }\n",
            "    أرجع احذف_مسار(م)\n",
            "}\n",
            "دالة احذف_مجلد(م: نص) -> منطقي {\n",
            "    إذا (حالة_مسار(م، 0) != 2) { أرجع خطأ }\n",
            "    أرجع احذف_مسار(م)\n",
            "}\n",
            "اطبع(احذف_مجلد(\"{مسار}\"))\n",
            "اطبع(احذف_ملف(\"{مسار2}\"))\n",
            "اطبع(احذف_ملف(\"{مسار}\"))\n",
            "اطبع(احذف_مجلد(\"{مسار2}\"))\n",
            "اطبع(حالة_مسار(\"{مسار}\"، 0))\n",
            "اطبع(حالة_مسار(\"{مسار2}\"، 0))",
        ),
        &["خطأ", "خطأ", "صحيح", "صحيح", "0", "0"],
    );
}

/// The load-bearing test for the `register_builtin_return_types` entry. Printing
/// alone passes without it — natively it prints nothing at all, which `اطبع` has
/// done for every scalar since #347 — so the assertions that matter are `نوع` and
/// the comparison, which fails native compilation outright with ت٠١٠١.
#[test]
fn test_path_delete_result_composes_as_a_boolean() {
    assert_prints(
        "حذف_تركيب",
        concat!(
            "اطبع(نوع(احذف_مسار(\"لا_يوجد_هذا_المسار\")))\n",
            "اطبع(احذف_مسار(\"لا_يوجد_هذا_المسار\") == خطأ)\n",
            "إذا (ليس احذف_مسار(\"لا_يوجد_هذا_المسار\")) { اطبع(\"لم يُحذف\") }",
        ),
        &["منطقي", "صحيح", "لم يُحذف"],
    );
}

/// A user function named `احذف_مسار` shadows the builtin, like every other core
/// name (LANGUAGE_SPEC §4.9).
#[test]
fn test_user_function_shadows_path_delete() {
    assert_prints(
        "حذف_تظليل",
        concat!(
            "دالة احذف_مسار(م: نص) -> منطقي {\n",
            "    أرجع صحيح\n",
            "}\n",
            "اطبع(احذف_مسار(\"لا_يوجد_هذا_المسار\"))",
        ),
        &["صحيح"],
    );
}

// ────────────────────────────────────────────────────────────────────────────
// معاملات_البرنامج — the program's own command-line arguments (#360)
// ────────────────────────────────────────────────────────────────────────────
//
// The first builtin whose two implementations read genuinely **different
// sources**: `trq_program_args` reads the argv its own `main` was handed, while
// `call_program_args` reads what the CLI recorded. There is no shared kernel to
// keep in step — and no way to tell the two apart except by running both, which
// is what every test here does.
//
// One shape is deliberately absent: `اطبع` of the whole array. Printing a
// non-empty `مصفوفة<نص>` is wrong natively (#359) — `trq_print_array` reads every
// element as an `i64` — with or without this name. The empty array is unaffected,
// since the element loop is skipped, so the CI example may print it and these
// tests index instead.

/// No arguments answers an empty array — a **value**, not a sentinel, and the
/// only row the CI example can cover, since `examples.yml` runs every example
/// with no arguments.
#[test]
fn test_program_args_is_empty_when_none_were_given() {
    assert_prints(
        "معاملات_فارغة",
        concat!("متغير م = معاملات_البرنامج()\n", "اطبع(طول(م))\n", "اطبع(م)",),
        &["0", "[]"],
    );
}

/// Arguments arrive in order, and `argv[0]` is **not** among them.
///
/// The exclusion is what makes this test possible at all: natively `argv[0]` is
/// the compiled binary's path and interpreted it would be the `.ترقيم` source
/// path, so a run including it could never assert the same list on both.
#[test]
fn test_program_args_preserves_order_and_drops_the_program_name() {
    assert_prints_with_args(
        "معاملات_ترتيب",
        concat!(
            "متغير م = معاملات_البرنامج()\n",
            "اطبع(طول(م))\n",
            "اطبع(م[0])\n",
            "اطبع(م[2])",
        ),
        &["أول", "ثان", "ثالث"],
        &["3", "أول", "ثالث"],
    );
}

/// An argument the shell already split stays one element, spaces and all, and an
/// empty argument keeps its position rather than vanishing.
#[test]
fn test_program_args_keeps_spaces_and_empty_arguments() {
    assert_prints_with_args(
        "معاملات_فراغات",
        concat!(
            "متغير م = معاملات_البرنامج()\n",
            "اطبع(طول(م))\n",
            "اطبع(م[0])\n",
            "اطبع(طول(م[1]))",
        ),
        &["ثان ثالث", ""],
        &["2", "ثان ثالث", "0"],
    );
}

/// An Arabic argument survives the round trip through both argv paths.
///
/// `طول` on the element is the assertion that matters: it counts **characters**,
/// so a byte-level mangling on either side shows up here where printing alone
/// might not.
#[test]
fn test_program_args_carries_arabic_unchanged() {
    assert_prints_with_args(
        "معاملات_عربية",
        concat!(
            "متغير م = معاملات_البرنامج()\n",
            "اطبع(م[0])\n",
            "اطبع(طول(م[0]))\n",
            "اطبع(م[0] == \"مرحبا\")",
        ),
        &["مرحبا"],
        &["مرحبا", "5", "صحيح"],
    );
}

/// An argument that looks like a flag belongs to the program, not to `tarqeem`.
///
/// This is what `allow_hyphen_values` buys on the interpreter and JIT legs; the
/// native leg gets it for free, since the binary's argv is not parsed by clap at
/// all. Without it the two legs would disagree — `tarqeem` would reject `-س` as
/// an unknown flag while the compiled program accepted it.
#[test]
fn test_program_args_accepts_an_argument_that_looks_like_a_flag() {
    assert_prints_with_args(
        "معاملات_شرطة",
        concat!(
            "متغير م = معاملات_البرنامج()\n",
            "اطبع(طول(م))\n",
            "اطبع(م[0])\n",
            "اطبع(م[1])",
        ),
        &["-س", "--jit"],
        &["2", "-س", "--jit"],
    );
}

/// A `--` that is not the **first** program argument is carried verbatim, like
/// any other token.
///
/// The leading position is the one exception in this whole contract, and it is
/// not fixable: `tarqeem run` reaches the program through clap, which consumes a
/// leading bare `--` as its own escape marker, while a compiled binary has no
/// parser in front of it. So `tarqeem run ب.ترقيم -- أ` answers `["أ"]` where
/// `./مخرج -- أ` answers `["--"، "أ"]`. It is bounded and escapable — doubling it
/// (`-- -- أ`) reproduces the native answer exactly — and it is the convention
/// `cargo run --` already sets, so it is documented in LANGUAGE_SPEC rather than
/// worked around. This test pins the part that *does* agree, which is every other
/// position.
#[test]
fn test_program_args_carries_a_later_double_dash_verbatim() {
    assert_prints_with_args(
        "معاملات_شرطتان",
        concat!(
            "متغير م = معاملات_البرنامج()\n",
            "اطبع(طول(م))\n",
            "اطبع(م[1])",
        ),
        &["أ", "--", "ب"],
        &["3", "--"],
    );
}

/// The array is iterable and its elements concatenate, which is what a program
/// actually does with them.
#[test]
fn test_program_args_iterates_and_concatenates() {
    assert_prints_with_args(
        "معاملات_حلقة",
        concat!(
            "لكل س في معاملات_البرنامج() {\n",
            "    اطبع(\"معامل: \" + س)\n",
            "}",
        ),
        &["أ", "ب"],
        &["معامل: أ", "معامل: ب"],
    );
}

/// Calling it twice answers the same list — it is process state read, not
/// consumed, unlike `اقرأ_مجرى`, whose stream is drained by reading it.
#[test]
fn test_program_args_answers_the_same_list_every_time() {
    assert_prints_with_args(
        "معاملات_تكرار",
        concat!(
            "اطبع(طول(معاملات_البرنامج()))\n",
            "اطبع(طول(معاملات_البرنامج()))\n",
            "اطبع(معاملات_البرنامج()[0] == معاملات_البرنامج()[0])",
        ),
        &["أ", "ب"],
        &["2", "2", "صحيح"],
    );
}

/// **The load-bearing test for the `register_builtin_return_types` entry**, and
/// the measurement behind it is a third distinct mode for an array return.
///
/// #330 measured one caught assertion for `Array(Int)` and #350 measured three
/// modes at once for another; neither transfers. Measured here with the entry
/// deleted, on `مصفوفة<نص>`:
///
/// | use | interpreters | native |
/// |---|---|---|
/// | `طول(م)` | correct | correct — `ArrayLen` routes to `trq_array_len` regardless |
/// | `اطبع(م[0])` | correct | correct — the element survives being printed alone |
/// | `نوع(م)` | `مؤشر` — caught | `مؤشر` — caught |
/// | `م[0] + "!"` | **run-time type error**, exit 1 | **`4376042720!`**, exit 0 |
/// | `م[0] == "أول"` | *unreached* | **`خطأ`**, exit 0 |
///
/// So the two backends fail the *same* use site in opposite manners — the
/// interpreter loudly, native silently — which no previous name in this family
/// has done. Indexing and printing the element pass either way, so the
/// assertions that carry the test are `نوع`, `+` and `==`.
#[test]
fn test_program_args_result_composes_as_strings() {
    assert_prints_with_args(
        "معاملات_تركيب",
        concat!(
            "متغير م = معاملات_البرنامج()\n",
            "اطبع(نوع(م))\n",
            "اطبع(م[0] + \"!\")\n",
            "اطبع(م[0] == \"أول\")",
        ),
        &["أول"],
        &["مصفوفة", "أول!", "صحيح"],
    );
}

/// A user function of the same name shadows the builtin, per LANGUAGE_SPEC §4.9.
#[test]
fn test_user_function_shadows_program_args() {
    assert_prints_with_args(
        "معاملات_تظليل",
        concat!(
            "دالة معاملات_البرنامج() -> مصفوفة<نص> {\n",
            "    أرجع [\"دالتي\"]\n",
            "}\n",
            "اطبع(معاملات_البرنامج()[0])",
        ),
        &["أول"],
        &["دالتي"],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// افتح_ملف — open(2), and the first name that puts a live handle on a stream
// ═══════════════════════════════════════════════════════════════════════

/// A handle is always past the console streams, so it goes straight into
/// `اكتب_مجرى`/`اقرأ_مجرى` without colliding with one.
///
/// Asserted as `>= 3` rather than `== 3` on purpose: the number is observable, so
/// pinning it would be a promise about allocation order rather than about the
/// reserved range. Both backends do start at 3, which is what keeps the
/// comparison meaningful at all.
#[test]
fn test_file_open_answers_a_handle_past_the_console_streams() {
    assert_prints_with_tree(
        "فتح_معرّف",
        &[("مقروء.نص", Fixture::File("مرحبا"))],
        concat!(
            "متغير م = افتح_ملف(\"{مسار}\", 0)\n",
            "اطبع(م >= 3)\n",
            "اطبع(م != 0)"
        ),
        &["صحيح", "صحيح"],
    );
}

/// The round trip the opener exists for: a file's bytes reach a program through
/// the stream pair, which before this could only read stdin.
///
/// Arabic content deliberately — «مرحبا» is five characters and ten bytes, so a
/// read that counted characters would answer 5 here and pass on an ASCII fixture.
#[test]
fn test_file_open_round_trips_a_file_through_the_stream_pair() {
    assert_prints_with_tree(
        "فتح_دورة",
        &[("مقروء.نص", Fixture::File("مرحبا"))],
        concat!(
            "متغير م = افتح_ملف(\"{مسار}\", 0)\n",
            "متغير بايتات = اقرأ_مجرى(م, 64)\n",
            "اطبع(طول(بايتات))\n",
            "اطبع(ثنائي_إلى_نص(بايتات))\n",
            "اطبع(طول(ثنائي_إلى_نص(بايتات)))"
        ),
        &["10", "مرحبا", "5"],
    );
}

/// Write mode creates the file, and that is visible with no flush and no close —
/// `File::create` reaches the filesystem before any byte does. Append mode creates
/// it too.
///
/// The program deletes the fixture first, so the row being tested is genuinely
/// *creation* rather than truncation of something the harness put there.
#[test]
fn test_file_open_in_write_and_append_modes_create_the_file() {
    assert_prints_with_tree(
        "فتح_إنشاء",
        &[("مكتوب.نص", Fixture::File("قديم"))],
        concat!(
            "اطبع(احذف_مسار(\"{مسار}\"))\n",
            "اطبع(حالة_مسار(\"{مسار}\", 0))\n",
            "متغير كاتب = افتح_ملف(\"{مسار}\", 1)\n",
            "اطبع(كاتب >= 3)\n",
            "اطبع(حالة_مسار(\"{مسار}\", 0))\n",
            "اطبع(احذف_مسار(\"{مسار}\"))\n",
            "متغير ملحق = افتح_ملف(\"{مسار}\", 2)\n",
            "اطبع(ملحق >= 3)\n",
            "اطبع(حالة_مسار(\"{مسار}\", 0))"
        ),
        &["صحيح", "0", "صحيح", "1", "صحيح", "صحيح", "1"],
    );
}

/// The durability row, and the reason the flush at program end exists.
///
/// It cannot be asserted from inside the program: the bytes sit in a `BufWriter`
/// until the end, so nothing the program prints tells "flushed at exit" from
/// "lost". Hence the file is read back **after** the run, once per backend leg.
///
/// Without that flush the two backends part company invisibly — the interpreter is
/// an ordinary Rust binary whose thread-local destructors run, while native's
/// `main` is `extern "C"` and skips them, so the compiled program would write
/// nothing and still exit 0. The CI backend diff compares stdout, so it could not
/// see it.
#[test]
fn test_file_open_lands_written_bytes_by_the_end_of_the_program() {
    assert_prints_with_tree_and_contents(
        "فتح_ثبات",
        &[("مكتوب.نص", Fixture::File(""))],
        concat!(
            "متغير كاتب = افتح_ملف(\"{مسار}\", 1)\n",
            "اطبع(اكتب_مجرى(كاتب, نص_إلى_ثنائي(\"مرحبا\")))"
        ),
        &["10"],
        &[("مكتوب.نص", "مرحبا")],
    );
}

/// Append keeps what is there where write truncates it — the one thing the two
/// modes differ in, so one test covers both. The content check is what sees it.
#[test]
fn test_file_open_appends_where_write_truncates() {
    assert_prints_with_tree_and_contents(
        "فتح_إلحاق",
        &[("مكتوب.نص", Fixture::File("أ"))],
        concat!(
            "متغير ملحق = افتح_ملف(\"{مسار}\", 2)\n",
            "اطبع(اكتب_مجرى(ملحق, نص_إلى_ثنائي(\"ب\")))"
        ),
        &["2"],
        &[("مكتوب.نص", "أب")],
    );
}

#[test]
fn test_file_open_in_write_mode_truncates_what_was_there() {
    assert_prints_with_tree_and_contents(
        "فتح_اقتطاع",
        &[("مكتوب.نص", Fixture::File("قديم جداً"))],
        concat!(
            "متغير كاتب = افتح_ملف(\"{مسار}\", 1)\n",
            "اطبع(اكتب_مجرى(كاتب, نص_إلى_ثنائي(\"ج\")))"
        ),
        &["2"],
        &[("مكتوب.نص", "ج")],
    );
}

/// Bytes written to a handle a program **does not close** are not guaranteed
/// visible until it ends, because the write path never flushes.
///
/// Pinning current behaviour, not the rule: a payload larger than the
/// `BufWriter`'s capacity would flush mid-write and *would* be visible. Both
/// backends share the same buffer type, so they agree either way — which is the
/// property worth having. The contrast is
/// `test_file_close_lands_written_bytes_before_the_program_ends` below: the same
/// program with one `اغلق_ملف` reads its own bytes back.
#[test]
fn test_file_open_does_not_promise_bytes_before_the_program_ends() {
    assert_prints_with_tree(
        "فتح_قبل_الإفراغ",
        &[("مكتوب.نص", Fixture::File(""))],
        concat!(
            "متغير كاتب = افتح_ملف(\"{مسار}\", 1)\n",
            "اطبع(اكتب_مجرى(كاتب, نص_إلى_ثنائي(\"مرحبا\")))\n",
            "متغير قارئ = افتح_ملف(\"{مسار}\", 0)\n",
            "اطبع(طول(اقرأ_مجرى(قارئ, 64)))\n",
            "اطبع(حالة_مسار(\"{مسار}\", 0))"
        ),
        &["10", "0", "1"],
    );
}

/// A mode this primitive does not know is refused **before** the path, so a bad
/// mode creates nothing.
///
/// `3` is the row that makes this more than a range check: it is
/// `وضع_قراءة_كتابة` in `stdlib/ملفات/ملف.ترقيم`, and no handle direction here can
/// serve it, so it is refused with the rest rather than served by one half.
#[test]
fn test_file_open_refuses_an_unknown_mode_without_touching_the_path() {
    assert_prints_with_tree(
        "فتح_وضع",
        &[("مكتوب.نص", Fixture::File("قديم"))],
        concat!(
            "اطبع(احذف_مسار(\"{مسار}\"))\n",
            "اطبع(افتح_ملف(\"{مسار}\", 3))\n",
            "اطبع(افتح_ملف(\"{مسار}\", 9))\n",
            "اطبع(افتح_ملف(\"{مسار}\", 0 - 1))\n",
            "اطبع(حالة_مسار(\"{مسار}\", 0))"
        ),
        &["صحيح", "-1", "-1", "-1", "0"],
    );
}

/// `-1`, and never `0`: `0` names stdin in the stream pair, so a failed open
/// answering it would send a later `اقرأ_مجرى` to the keyboard and succeed.
///
/// An absent path, an empty name and `لا_شيء` are one answer. The `لا_شيء` row is
/// the **path**, which is a pointer, so an un-narrowed `نص?` reaches the runtime's
/// null guard; the mode is an `عدد` and has no such row (#327).
#[test]
fn test_file_open_answers_minus_one_for_nothing_to_open() {
    assert_prints_with_tree(
        "فتح_معدوم",
        &[("مجلد", Fixture::EmptyDir)],
        concat!(
            "متغير غائب_فتح: نص? = لا_شيء\n",
            "اطبع(افتح_ملف(\"{مسار}/لا_يوجد/ملف.نص\", 0))\n",
            "اطبع(افتح_ملف(\"\", 0))\n",
            "اطبع(افتح_ملف(غائب_فتح, 0))"
        ),
        &["-1", "-1", "-1"],
    );
}

/// Two opens are two handles, or a program holding both would write through the
/// one it meant to read.
#[test]
fn test_file_open_hands_out_distinct_handles() {
    assert_prints_with_tree(
        "فتح_تعدد",
        &[("مقروء.نص", Fixture::File("مرحبا"))],
        concat!(
            "متغير أول = افتح_ملف(\"{مسار}\", 0)\n",
            "متغير ثان = افتح_ملف(\"{مسار}\", 0)\n",
            "اطبع(أول >= 3)\n",
            "اطبع(ثان >= 3)\n",
            "اطبع(أول != ثان)"
        ),
        &["صحيح", "صحيح", "صحيح"],
    );
}

/// A handle carries a direction and the stream pair honours it: writing to a
/// reader fails, reading a writer answers nothing.
///
/// Both refusals existed for a handle that was never opened; this is the first
/// time a **live** handle can be the wrong kind, and the first time the two
/// backends have to agree about a handle rather than about its absence.
#[test]
fn test_file_open_handles_carry_their_direction() {
    assert_prints_with_tree(
        "فتح_اتجاه",
        &[("مقروء.نص", Fixture::File("مرحبا"))],
        concat!(
            "متغير قارئ = افتح_ملف(\"{مسار}\", 0)\n",
            "اطبع(اكتب_مجرى(قارئ, نص_إلى_ثنائي(\"س\")))\n",
            "متغير كاتب = افتح_ملف(\"{مسار}\", 2)\n",
            "اطبع(طول(اقرأ_مجرى(كاتب, 4)))"
        ),
        &["-1", "0"],
    );
}

/// A directory is refused in **every** mode, and that is a deliberate deviation
/// from `open(2)` rather than a faithful reading of it.
///
/// Found by running the CI example — the line `افتح_ملف(".", 0)` was written
/// expecting `-1` and answered a handle, because `File::open` succeeds on a
/// directory under POSIX. Left that way it would have been a **platform** split in
/// a contract row: Windows refuses the same open, since `CreateFile` needs a flag
/// `std` does not pass, and `cargo test` never runs there — the Windows CI job
/// only builds. So this test would have encoded a Unix-only answer with nothing
/// able to catch it, which is #355's review lesson inverted.
///
/// Refused on both sides instead, checked through the opened handle so there is no
/// window between the test and the open. One documented behaviour, one
/// implementation — the shape #355 chose over a `cfg(windows)` arm.
#[test]
fn test_file_open_refuses_a_directory_in_every_mode() {
    assert_prints_with_tree(
        "فتح_مجلد",
        &[("مجلد", Fixture::EmptyDir)],
        concat!(
            "اطبع(افتح_ملف(\"{مسار}\", 0))\n",
            "اطبع(افتح_ملف(\"{مسار}\", 1))\n",
            "اطبع(افتح_ملف(\"{مسار}\", 2))"
        ),
        &["-1", "-1", "-1"],
    );
}

/// The composition gate, per standing rule 5: printing a sentinel-typed result
/// passes while composing it is silently wrong.
///
/// A scalar return's missing-`register_builtin_return_types` mode is predictable
/// across names (#347, confirmed at #352 and #355), and these are the three rows
/// it shows up in: `نوع` answers `مؤشر`, and `+`/`==` fail native compilation with
/// ت٠١٠١ because an `add`/`icmp` on a `ptr` is not valid IR. `اطبع` alone would
/// print nothing natively and pass here either way, so it is not the gate.
#[test]
fn test_file_open_result_composes_as_an_integer() {
    assert_prints_with_tree(
        "فتح_تركيب",
        &[("مقروء.نص", Fixture::File("مرحبا"))],
        concat!(
            "متغير م = افتح_ملف(\"{مسار}\", 0)\n",
            "اطبع(نوع(م))\n",
            "اطبع(م - م)\n",
            "اطبع(م >= 3)\n",
            "متغير مرفوض = افتح_ملف(\"{مسار}\", 9)\n",
            "اطبع(مرفوض + 1)\n",
            "اطبع(مرفوض == 0 - 1)"
        ),
        &["عدد", "0", "صحيح", "0", "صحيح"],
    );
}

// ---------------------------------------------------------------------------
// اغلق_ملف — close(2), and the name that makes written bytes land early
// ---------------------------------------------------------------------------

/// The row the name exists for, and the capability `افتح_ملف` could not deliver:
/// a program writes a file and reads it back **within its own run**.
///
/// Its contrast is `test_file_open_does_not_promise_bytes_before_the_program_ends`
/// above — the identical program without the close reads `0` bytes. Arabic
/// content deliberately: five characters and ten bytes, so a wrong unit anywhere
/// on the path cannot pass.
#[test]
fn test_file_close_lands_written_bytes_before_the_program_ends() {
    assert_prints_with_tree(
        "إغلاق_يُنزل_البايتات",
        &[("مكتوب.نص", Fixture::File(""))],
        concat!(
            "متغير كاتب = افتح_ملف(\"{مسار}\", 1)\n",
            "اطبع(اكتب_مجرى(كاتب, نص_إلى_ثنائي(\"مرحبا\")))\n",
            "اطبع(اغلق_ملف(كاتب))\n",
            "متغير قارئ = افتح_ملف(\"{مسار}\", 0)\n",
            "اطبع(ثنائي_إلى_نص(اقرأ_مجرى(قارئ, 64)))"
        ),
        &["10", "صحيح", "مرحبا"],
    );
}

/// And the bytes are on disk when the run is over, not merely readable inside it.
///
/// Invisible from within the program, which is what `_and_contents` is for: it
/// reads the fixture back after each backend's leg.
#[test]
fn test_file_close_leaves_the_bytes_on_disk() {
    assert_prints_with_tree_and_contents(
        "إغلاق_يُثبت_البايتات",
        &[("مكتوب.نص", Fixture::File(""))],
        concat!(
            "متغير كاتب = افتح_ملف(\"{مسار}\", 1)\n",
            "اطبع(اكتب_مجرى(كاتب, نص_إلى_ثنائي(\"ج\")))\n",
            "اطبع(اغلق_ملف(كاتب))"
        ),
        &["2", "صحيح"],
        &[("مكتوب.نص", "ج")],
    );
}

/// A reader closes too, and answers the same `صحيح` — there is simply nothing to
/// flush.
#[test]
fn test_file_close_releases_a_reader() {
    assert_prints_with_tree(
        "إغلاق_قارئ",
        &[("مقروء.نص", Fixture::File("مرحبا"))],
        concat!(
            "متغير قارئ = افتح_ملف(\"{مسار}\", 0)\n",
            "اطبع(ثنائي_إلى_نص(اقرأ_مجرى(قارئ, 64)))\n",
            "اطبع(اغلق_ملف(قارئ))"
        ),
        &["مرحبا", "صحيح"],
    );
}

/// The handle leaves the table, so the second close is a miss like any other.
#[test]
fn test_file_close_refuses_a_handle_it_already_released() {
    assert_prints_with_tree(
        "إغلاق_مرتين",
        &[("مقروء.نص", Fixture::File("م"))],
        concat!(
            "متغير قارئ = افتح_ملف(\"{مسار}\", 0)\n",
            "اطبع(اغلق_ملف(قارئ))\n",
            "اطبع(اغلق_ملف(قارئ))"
        ),
        &["صحيح", "خطأ"],
    );
}

/// **The console streams are not closable**, deviating from `close(2)`, which
/// does close descriptor 1.
///
/// They need no special arm: handles start at `٣`, so `٠`/`١`/`٢` were never in
/// the table and a refusal falls out of the lookup. One documented behaviour and
/// one implementation, the shape #362 chose for its directory refusal — and here
/// it also keeps a program from closing the stream the harness reads its output
/// from.
#[test]
fn test_file_close_refuses_the_console_streams() {
    assert_prints(
        "إغلاق_المجاري_القياسية",
        concat!(
            "اطبع(اغلق_ملف(0))\n",
            "اطبع(اغلق_ملف(1))\n",
            "اطبع(اغلق_ملف(2))"
        ),
        &["خطأ", "خطأ", "خطأ"],
    );
}

/// A handle never opened, and a negative one. Total: no range check is needed
/// before the call, and nothing throws.
#[test]
fn test_file_close_refuses_a_handle_never_opened() {
    assert_prints(
        "إغلاق_معرِّف_مجهول",
        concat!(
            "اطبع(اغلق_ملف(3))\n",
            "اطبع(اغلق_ملف(99))\n",
            "اطبع(اغلق_ملف(0 - 1))"
        ),
        &["خطأ", "خطأ", "خطأ"],
    );
}

/// Closing frees the entry, never the number.
///
/// Both backends count up from 3 and neither recycles, so a program that prints a
/// handle prints the same sequence everywhere — which is the only reason the
/// numbers are comparable at all.
#[test]
fn test_file_close_does_not_recycle_the_number() {
    assert_prints_with_tree(
        "إغلاق_لا_يُعيد_الرقم",
        &[("مقروء.نص", Fixture::File("م"))],
        concat!(
            "متغير أول = افتح_ملف(\"{مسار}\", 0)\n",
            "اطبع(أول)\n",
            "اطبع(اغلق_ملف(أول))\n",
            "متغير ثان = افتح_ملف(\"{مسار}\", 0)\n",
            "اطبع(ثان)\n",
            "اطبع(ثان > أول)"
        ),
        &["3", "صحيح", "4", "صحيح"],
    );
}

/// The stream pair inherits the release with no arm of its own: a closed number
/// is absent from the table, which both halves already refuse.
///
/// The two answers differ because the two return types do — `اكتب_مجرى` has `-١`
/// to spare and an array return has nothing, so a closed handle reads exactly
/// like EOF.
#[test]
fn test_the_stream_pair_refuses_a_closed_handle() {
    assert_prints_with_tree(
        "إغلاق_ثم_المجاري",
        &[("مقروء.نص", Fixture::File("مرحبا"))],
        concat!(
            "متغير قارئ = افتح_ملف(\"{مسار}\", 0)\n",
            "اطبع(اغلق_ملف(قارئ))\n",
            "اطبع(طول(اقرأ_مجرى(قارئ, 64)))\n",
            "متغير كاتب = افتح_ملف(\"{مسار}\", 2)\n",
            "اطبع(اغلق_ملف(كاتب))\n",
            "اطبع(اكتب_مجرى(كاتب, نص_إلى_ثنائي(\"x\")))"
        ),
        &["صحيح", "0", "صحيح", "-1"],
    );
}

/// The composition gate — the three things a `منطقي` result must be able to do.
///
/// No arithmetic row: `منطقي + عدد` is refused by the **semantic** layer, which
/// never consults the IR return type, so the `+ ١` row every measurement since
/// #347 used is unwritable here. `ليس` is the substitute. Measured with the
/// `register_builtin_return_types` entry deleted, `نوع` answers `مؤشر` and
/// `== خطأ` fails native compilation with ت٠١٠١.
#[test]
fn test_file_close_result_composes_as_a_boolean() {
    assert_prints(
        "تركيب_الإغلاق",
        concat!(
            "اطبع(نوع(اغلق_ملف(3)))\n",
            "اطبع(اغلق_ملف(3) == خطأ)\n",
            "إذا (ليس اغلق_ملف(3)) { اطبع(\"لم يُغلق\") }"
        ),
        &["منطقي", "صحيح", "لم يُغلق"],
    );
}

/// A user function named `اغلق_ملف` shadows the builtin, like every other core
/// name: builtins are the last resort in name resolution, not reserved words.
#[test]
fn test_file_close_can_be_shadowed_by_a_user_function() {
    assert_prints(
        "تظليل_الإغلاق",
        concat!(
            "دالة اغلق_ملف(معرف: عدد) -> منطقي {\n",
            "    أرجع صحيح\n",
            "}\n",
            "اطبع(اغلق_ملف(3))"
        ),
        &["صحيح"],
    );
}
