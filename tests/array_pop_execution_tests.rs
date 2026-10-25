//! Array pop requires a known receiver layout and preserves typed-array behavior.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tempfile::TempDir;

const TARQEEM: &str = env!("CARGO_BIN_EXE_tarqeem");

#[derive(Debug)]
struct Output {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn ensure_runtime_library() {
    static RUNTIME: OnceLock<()> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let mut command = Command::new(cargo);
        command
            .args(["build", "-p", "tarqeem-runtime"])
            .current_dir(project_root());
        if !cfg!(debug_assertions) {
            command.arg("--release");
        }
        let output = command.output().expect("تعذّر بناء مكتبة وقت التشغيل");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let library = if cfg!(windows) { "trq.lib" } else { "libtrq.a" };
        assert!(project_root()
            .join("target")
            .join(profile)
            .join(library)
            .exists());
    });
}

fn run_command(program: &Path, args: &[&str], cwd: &Path) -> Output {
    let capture = TempDir::new().unwrap();
    let stdout = capture.path().join("stdout");
    let stderr = capture.path().join("stderr");
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env_remove("TARQEEM_HOME")
        .stdin(Stdio::null())
        .stdout(fs::File::create(&stdout).unwrap())
        .stderr(fs::File::create(&stderr).unwrap())
        .spawn()
        .unwrap_or_else(|e| panic!("تعذّر تشغيل {} {args:?}: {e}", program.display()));
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("انتهت مهلة {} {args:?}", program.display());
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Output {
        status: status.code(),
        stdout: fs::read_to_string(stdout).unwrap().replace("\r\n", "\n"),
        stderr: fs::read_to_string(stderr).unwrap(),
    }
}

fn write_program(dir: &Path, name: &str, body: &str) -> PathBuf {
    let source = dir.join(format!("{name}.ترقيم"));
    fs::write(&source, format!("بسم_الله\n{}\nالحمد_لله\n", body.trim())).unwrap();
    source
}

fn assert_prints(body: &str, expected: &str) {
    let dir = TempDir::new().unwrap();
    let source = write_program(dir.path(), "حذف_صالح", body);
    for args in [
        vec!["run", source.to_str().unwrap()],
        vec!["run", source.to_str().unwrap(), "--jit"],
    ] {
        let output = run_command(Path::new(TARQEEM), &args, dir.path());
        assert_eq!(output.status, Some(0), "{args:?}: {output:?}");
        assert_eq!(output.stdout, expected, "{args:?}: {output:?}");
    }

    ensure_runtime_library();
    for opt in ["0", "1", "2", "3"] {
        let executable = dir
            .path()
            .join(format!("native_O{opt}{}", std::env::consts::EXE_SUFFIX));
        let output = run_command(
            Path::new(TARQEEM),
            &[
                "compile",
                source.to_str().unwrap(),
                "-O",
                opt,
                "-o",
                executable.to_str().unwrap(),
            ],
            dir.path(),
        );
        assert_eq!(output.status, Some(0), "compile O{opt}: {output:?}");
        let output = run_command(&executable, &[], dir.path());
        assert_eq!(output.status, Some(0), "native O{opt}: {output:?}");
        assert_eq!(output.stdout, expected, "native O{opt}: {output:?}");
    }
}

fn assert_pop_rejected(source: &Path) {
    let dir = source.parent().unwrap();
    let mut failures = Vec::new();
    let mut executables = Vec::new();
    let mut check = |args: &[&str]| {
        let output = run_command(Path::new(TARQEEM), args, dir);
        if output.status != Some(1)
            || !output.stderr.contains("نوع عنصر معلوم")
            || !output.stderr.contains("array with a known element type")
            || output.stderr.contains("مؤشر")
            || output.stderr.contains("clang")
            || output.stderr.contains("Runtime error")
        {
            failures.push(format!("{args:?}: {output:?}"));
        }
    };
    for args in [
        vec!["run", source.to_str().unwrap()],
        vec!["run", source.to_str().unwrap(), "--jit"],
    ] {
        check(&args);
    }
    for opt in ["0", "1", "2", "3"] {
        let executable = dir.join(format!("refused_O{opt}{}", std::env::consts::EXE_SUFFIX));
        check(&[
            "compile",
            source.to_str().unwrap(),
            "-O",
            opt,
            "-o",
            executable.to_str().unwrap(),
        ]);
        // Never execute a negative fixture even if compilation wrongly succeeds.
        executables.push(executable);
    }
    for executable in executables {
        if executable.exists() {
            failures.push(format!("unexpected executable: {}", executable.display()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn test_pop_rejects_dictionary_parameters_without_pointer_diagnostics() {
    for expression in ["م.احذف_آخر()", "احذف_آخر(م)"] {
        let dir = TempDir::new().unwrap();
        let source = write_program(
            dir.path(),
            "حذف_قاموس",
            &format!("دالة أزل(م: قاموس<نص، عدد>) {{ {expression} }}\nاطبع(٠)"),
        );
        assert_pop_rejected(&source);
    }
}

#[test]
fn test_pop_rejects_untyped_parameters_even_when_passed_arrays() {
    for expression in ["م.احذف_آخر()", "احذف_آخر(م)"] {
        for array in ["[١، ٢]", "[١.٥، ٢.٥]"] {
            let dir = TempDir::new().unwrap();
            let source = write_program(
                dir.path(),
                "حذف_غير_معنون",
                &format!("دالة أزل(م) {{ اطبع({expression}) }}\nأزل({array})"),
            );
            assert_pop_rejected(&source);
        }
    }
}

#[test]
fn test_imported_dictionary_pop_is_rejected_during_lowering() {
    for expression in ["م.احذف_آخر()", "احذف_آخر(م)"] {
        let dir = TempDir::new().unwrap();
        write_program(
            dir.path(),
            "مساعد",
            &format!("صدّر دالة أزل(م: قاموس<نص، عدد>) {{ {expression} }}"),
        );
        let source = write_program(
            dir.path(),
            "الرئيسي",
            "استورد { أزل } من \"./مساعد\"\nاطبع(٠)",
        );
        assert_pop_rejected(&source);
    }
}

#[test]
fn test_pop_preserves_global_typed_elements_and_empty_values() {
    assert_prints(
        r#"
متغير أعداد = [١، ٢]
اطبع(أعداد.احذف_آخر())
اطبع(احذف_آخر(أعداد))
اطبع(أعداد.احذف_آخر())
اطبع(طول(أعداد))
متغير كسور = [١.٥، ٢.٥]
اطبع(احذف_آخر(كسور) + ٠.٥)
اطبع(كسور.احذف_آخر() + ٠.٥)
اطبع(احذف_آخر(كسور) + ٠.٥)
اطبع(طول(كسور))
متغير نصوص = ["أ"، "ب"]
اطبع(نصوص.احذف_آخر() + "!")
اطبع(احذف_آخر(نصوص) + "!")
اطبع(نصوص.احذف_آخر() + "!")
اطبع(طول(نصوص))
"#,
        "2\n1\n0\n0\n3.0\n2.0\n0.5\n0\nب!\nأ!\n!\n0\n",
    );
}

#[test]
fn test_pop_preserves_local_arrays_and_typed_parameter_remedy() {
    assert_prints(
        r#"
دالة أزل_عدد(م: مصفوفة<عدد>) {
    اطبع(احذف_آخر(م))
    م.احذف_آخر()
    اطبع(طول(م))
}
دالة أزل_كسر(م: مصفوفة<عدد_عشري>) {
    اطبع(م.احذف_آخر() + ٠.٥)
    احذف_آخر(م)
    اطبع(طول(م))
}
دالة أزل_نص(م: مصفوفة<نص>) {
    اطبع(احذف_آخر(م) + "!")
    م.احذف_آخر()
    اطبع(طول(م))
}
دالة اعرض() {
    متغير أعداد = [١، ٢، ٣]
    اطبع(أعداد.احذف_آخر())
    أزل_عدد(أعداد)
    اطبع(طول(أعداد))
    متغير كسور = [١.٥، ٢.٥، ٣.٥]
    اطبع(احذف_آخر(كسور) + ٠.٥)
    أزل_كسر(كسور)
    اطبع(طول(كسور))
    متغير نصوص = ["أ"، "ب"، "ج"]
    اطبع(نصوص.احذف_آخر() + "!")
    أزل_نص(نصوص)
    اطبع(طول(نصوص))
}
اعرض()
"#,
        "3\n2\n0\n0\n4.0\n3.0\n0\n0\nج!\nب!\n0\n0\n",
    );
}

#[test]
fn test_pop_receiver_is_evaluated_once() {
    assert_prints(
        r#"
متغير مرات = ٠
متغير م = [١، ٢، ٣]
دالة خذ() -> مصفوفة<عدد> {
    مرات = مرات + ١
    أرجع م
}
اطبع(احذف_آخر(خذ()))
اطبع(خذ().احذف_آخر())
اطبع(مرات)
اطبع(طول(م))
"#,
        "3\n2\n2\n1\n",
    );
}

#[test]
fn test_pop_validation_preserves_user_functions_function_values_and_methods() {
    for body in [
        r#"دالة احذف_آخر(س: نص) -> نص { أرجع س + "!" }
اطبع(احذف_آخر("أ"))"#,
        r#"ثابت احذف_آخر: (نص) -> نص = (س) => س + "!"
اطبع(احذف_آخر("أ"))"#,
        r#"دالة اعرض() {
    ثابت احذف_آخر: (نص) -> نص = (س) => س + "!"
    اطبع(احذف_آخر("أ"))
}
اعرض()"#,
        r#"صنف حامل {
    منشئ() {}
    عام دالة احذف_آخر() -> نص { أرجع "أ!" }
}
متغير ح = جديد حامل()
اطبع(ح.احذف_آخر())"#,
        r#"صنف أدوات {
    مشترك دالة احذف_آخر(س: نص) -> نص { أرجع س + "!" }
}
اطبع(أدوات.احذف_آخر("أ"))"#,
    ] {
        assert_prints(body, "أ!\n");
    }
}
