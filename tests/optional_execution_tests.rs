//! Optional arguments and plain call controls must agree across interpreter and native optimization levels.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tempfile::TempDir;

const TARQEEM: &str = env!("CARGO_BIN_EXE_tarqeem");
const OPT_LEVELS: [&str; 4] = ["0", "1", "2", "3"];

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

fn write_program(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("معاملات_اختيارية.ترقيم");
    fs::write(&path, format!("بسم_الله\n{}\nالحمد_لله\n", body.trim()))
        .expect("تعذّر كتابة البرنامج");
    path
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

fn assert_prints(body: &str, expected: &str) {
    let dir = TempDir::new().unwrap();
    let source = write_program(dir.path(), body);
    // --jit currently falls back to interpretation; native coverage is the O0–O3 loop below.
    for args in [
        vec!["run", source.to_str().unwrap()],
        vec!["run", source.to_str().unwrap(), "--jit"],
    ] {
        let output = run_command(Path::new(TARQEEM), &args, dir.path());
        assert_eq!(output.status, Some(0), "{args:?}: {output:?}");
        assert_eq!(output.stdout, expected, "{args:?}: {output:?}");
    }

    ensure_runtime_library();
    for opt in OPT_LEVELS {
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

fn assert_type_error(output: &Output) {
    assert_eq!(output.status, Some(1), "{output:?}");
    assert!(output.stderr.contains("ن٠٠٠١"), "{output:?}");
    assert!(!output.stderr.contains("فشل clang"), "{output:?}");
}

fn assert_rejected(body: &str) {
    let dir = TempDir::new().unwrap();
    let source = write_program(dir.path(), body);
    for args in [
        vec!["check", source.to_str().unwrap()],
        vec!["run", source.to_str().unwrap()],
        vec!["run", source.to_str().unwrap(), "--jit"],
    ] {
        assert_type_error(&run_command(Path::new(TARQEEM), &args, dir.path()));
    }
    for opt in OPT_LEVELS {
        let executable = dir
            .path()
            .join(format!("rejected_O{opt}{}", std::env::consts::EXE_SUFFIX));
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
        // Never run an accidentally accepted negative fixture: its null argument can crash or hang.
        assert_type_error(&output);
        assert!(!executable.exists());
    }
}

#[test]
fn test_repeated_identity_calls_preserve_each_result() {
    assert_prints(
        r#"
دالة هوية(س: عدد) -> عدد { أرجع س }
اطبع(هوية(١٦٠٥))
اطبع(هوية(١٦٠٥))
اطبع(هوية(٧))
"#,
        "1605\n1605\n7\n",
    );
}

#[test]
fn test_repeated_small_calls_preserve_dependent_and_nested_results() {
    assert_prints(
        r#"
دالة هوية(س: عدد) -> عدد { أرجع س }
دالة زد(س: عدد) -> عدد { أرجع س + ١ }
دالة جمع_هويات(س: عدد) -> عدد { أرجع هوية(س) + هوية(س + ١) }
دالة رئيسية() {
    متغير أول = هوية(٥)
    متغير ثان = زد(أول)
    اطبع(أول + ثان)
    اطبع(هوية(زد(ثان)))
    اطبع(زد(هوية(٩)))
    اطبع(جمع_هويات(٣))
    اطبع(جمع_هويات(٤))
}
"#,
        "11\n7\n10\n7\n9\n",
    );
}

#[test]
fn test_repeated_narrowed_optional_calls_match_arithmetic_controls() {
    assert_prints(
        r#"
دالة هوية(س: عدد) -> عدد { أرجع س }
متغير موجود: عدد? = ١٦٠٥
إذا (موجود != لا_شيء) {
    اطبع(هوية(موجود))
    اطبع(هوية(موجود))
    اطبع(بتات_نفي(موجود))
    اطبع(موجود + ٠)
}
"#,
        "1605\n1605\n-1606\n1605\n",
    );
}

#[test]
fn test_narrowed_optional_call_matches_arithmetic_controls() {
    assert_prints(
        r#"
دالة هوية(س: عدد) -> عدد { أرجع س }
متغير موجود: عدد? = 1605
إذا (موجود != لا_شيء) {
    اطبع(هوية(موجود))
    اطبع(بتات_نفي(موجود))
    اطبع(موجود + 0)
}
"#,
        "1605\n-1606\n1605\n",
    );
}

#[test]
fn test_narrowed_scalar_arguments_keep_each_position_and_widening() {
    // Recursive callees retain their argument ABI at O3 instead of being inlined away.
    assert_prints(
        r#"
دالة سجل(أ: عدد، ب: عدد_عشري، ج: منطقي) {
    إذا (أ < 0) { سجل(-أ، ب، ج) } وإلا {
        اطبع(أ)
        اطبع(ب)
        اطبع(ج)
    }
}
دالة وسع(س: عدد_عشري) -> عدد_عشري {
    إذا (س < 0.0) { أرجع وسع(-س) }
    أرجع س + 0.5
}
دالة رئيسية() {
    متغير أ: عدد? = 7
    متغير ب: عدد_عشري? = 2.5
    متغير ج: منطقي? = صحيح
    متغير د: منطقي? = خطأ
    إذا (أ != لا_شيء) {
        إذا (ب != لا_شيء) {
            إذا (ج != لا_شيء) {
                إذا (د != لا_شيء) { سجل(أ، ب، د) }
                اطبع(وسع(أ))
                سجل(أ، ب، ج)
            }
        }
    }
}
"#,
        "7\n2.5\nخطأ\n7.5\n7\n2.5\nصحيح\n",
    );
}

#[test]
fn test_narrowed_runtime_arguments_use_declared_scalar_types() {
    assert_prints(
        r#"
استورد { جذر } من "رياضيات"
متغير رمز: عدد? = 1605
متغير بداية: عدد? = 1
متغير طول: عدد? = 2
متغير مربع: عدد_عشري? = 6.25
إذا (رمز != لا_شيء) { اطبع(رمز_إلى_حرف(رمز)) }
إذا (بداية != لا_شيء) {
    إذا (طول != لا_شيء) { اطبع(قص_حروف("مرحبا"، بداية، طول)) }
}
إذا (مربع != لا_شيء) { اطبع(جذر(مربع)) }
"#,
        "م\nرح\n2.5\n",
    );
}

#[test]
fn test_narrowed_method_and_indirect_call_arguments() {
    assert_prints(
        r#"
صنف جامع {
    عام قيمة: عدد
    منشئ() { هذا.قيمة = 3 }
    عام دالة اجمع(أ: عدد، ب: عدد) -> عدد { أرجع أ + ب + هذا.قيمة }
}
متغير ج = جديد جامع()
ثابت جمع: (عدد، عدد) -> عدد = (أ، ب) => أ + ب
متغير س: عدد? = 7
إذا (س != لا_شيء) {
    اطبع(ج.اجمع(3، س))
    اطبع(جمع(س، 4))
    اطبع(جمع(5، س))
}
"#,
        "13\n11\n12\n",
    );
}

#[test]
fn test_optional_parameters_keep_boxes_null_and_reference_values() {
    assert_prints(
        r#"
دالة افحص(س: عدد?) {
    إذا (س != لا_شيء) {
        اطبع("موجود")
        افحص(لا_شيء)
    } وإلا { اطبع("فارغ") }
}
دالة نصي(س: نص) { اطبع(س) }
متغير صفر: عدد? = 0
متغير غائب: عدد? = لا_شيء
متغير نصي_اختياري: نص? = "سلام"
افحص(صفر)
افحص(غائب)
افحص(0)
افحص(لا_شيء)
إذا (لا_شيء != صفر) { افحص(صفر) }
إذا (صفر == لا_شيء) { اطبع("خطأ") } وإلا { اطبع(صفر + 0) }
إذا (نصي_اختياري != لا_شيء) { نصي(نصي_اختياري) }
"#,
        "موجود\nفارغ\nفارغ\nموجود\nفارغ\nفارغ\nموجود\nفارغ\n0\nسلام\n",
    );
}

#[test]
fn test_indirect_optional_parameters_preserve_boxes_and_box_scalars() {
    assert_prints(
        r#"
ثابت موجود: (عدد?) -> منطقي = (س) => س != لا_شيء
متغير س: عدد? = 0
اطبع(موجود(س))
اطبع(موجود(لا_شيء))
اطبع(موجود(0))
"#,
        "صحيح\nخطأ\nصحيح\n",
    );
}

#[test]
fn test_unnarrowed_optional_scalars_are_rejected_as_concrete_arguments() {
    for (ty, present) in [("عدد", "7"), ("عدد_عشري", "2.5"), ("منطقي", "خطأ")] {
        for value in [present, "لا_شيء"] {
            assert_rejected(&format!(
                "دالة خذ(أ: عدد، ب: {ty}) {{ اطبع(ب) }}\nمتغير س: {ty}? = {value}\nخذ(1، س)"
            ));
        }
    }
    for value in ["0", "لا_شيء"] {
        assert_rejected(&format!("متغير س: عدد? = {value}\nنم(س)"));
        assert_rejected(&format!("متغير س: عدد? = {value}\nاطبع(بتات_نفي(س))"));
    }
}
