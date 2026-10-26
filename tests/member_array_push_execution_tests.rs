//! Member append widens integer values to the float array element type.

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
    let source = write_program(dir.path(), "إلحاق_عنصر", body);
    for args in [
        vec!["run", source.to_str().unwrap()],
        vec!["run", source.to_str().unwrap(), "--jit"],
    ] {
        let output = run_command(Path::new(TARQEEM), &args, dir.path());
        assert_eq!(output.status, Some(0), "{args:?}: {}", output.stderr);
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

#[test]
fn test_member_append_widens_integer_literal_in_global_float_array() {
    assert_prints(
        "متغير م = [١.٥]\nم.ألحق(٢)\nاطبع(م[١] + ٠.٥)\nاطبع(طول(م))",
        "2.5\n2\n",
    );
}

#[test]
fn test_member_add_alias_widens_integer_literal_in_global_float_array() {
    assert_prints(
        "متغير م = [١.٥]\nم.أضف(٢)\nاطبع(م[١] + ٠.٥)\nاطبع(طول(م))",
        "2.5\n2\n",
    );
}

#[test]
fn test_member_append_widens_integer_parameters_and_repeated_loop_values() {
    assert_prints(
        r#"
دالة وسع(م: مصفوفة<عدد_عشري>، س: عدد) {
    م.ألحق(س)
    م.أضف(-س)
    لكل (متغير ي = ٠؛ ي < ٣؛ ي++) {
        م.ألحق(ي)
        م.أضف(ي - ١)
    }
}
دالة اعرض() {
    متغير م = [١.٥]
    وسع(م، ٤)
    اطبع(طول(م))
    لكل (متغير ي = ٠؛ ي < طول(م)؛ ي++) {
        اطبع(م[ي] + ٠.٥)
    }
}
اعرض()
"#,
        "9\n2.0\n4.5\n-3.5\n0.5\n-0.5\n1.5\n0.5\n2.5\n1.5\n",
    );
}

#[test]
fn test_member_append_preserves_field_receiver_and_single_evaluation() {
    assert_prints(
        r#"
صنف حامل {
    عام م: مصفوفة<عدد_عشري>
    منشئ() { هذا.م = [١.٥] }
}
متغير ح = جديد حامل()
متغير وصول = ٠
متغير قيم = ٠
دالة خذ() -> مصفوفة<عدد_عشري> {
    وصول = وصول + ١
    أرجع ح.م
}
دالة قيمة() -> عدد {
    قيم = قيم + ١
    أرجع قيم
}
خذ().ألحق(قيمة())
خذ().أضف(قيمة())
ح.م.ألحق(-٢)
ح.م.أضف(٠)
اطبع(وصول)
اطبع(قيم)
اطبع(طول(ح.م))
اطبع(ح.م[١] + ٠.٥)
اطبع(ح.م[٢] + ٠.٥)
اطبع(ح.م[٣] + ٠.٥)
اطبع(ح.م[٤] + ٠.٥)
"#,
        "2\n2\n5\n1.5\n2.5\n-1.5\n0.5\n",
    );
}

#[test]
fn test_global_widening_and_matching_member_element_types_remain_valid() {
    assert_prints(
        r#"
دالة وسع(م: مصفوفة<عدد_عشري>، س: عدد) {
    ألحق(م، س)
}
متغير كسور = [١.٥]
ألحق(كسور، ٢)
وسع(كسور، -٣)
كسور.ألحق(٤.٥)
كسور.أضف(٥.٥)
اطبع(طول(كسور))
اطبع(كسور[١] + ٠.٥)
اطبع(كسور[٢] + ٠.٥)
اطبع(كسور[٣] + ٠.٥)
اطبع(كسور[٤] + ٠.٥)
متغير أعداد = [١]
أعداد.ألحق(٢)
أعداد.أضف(-٣)
اطبع(أعداد[١])
اطبع(أعداد[٢])
متغير نصوص = ["أ"]
نصوص.ألحق("ب")
نصوص.أضف("ج")
اطبع(نصوص[١] + نصوص[٢])
اطبع(طول(نصوص))
"#,
        "5\n2.5\n-2.5\n5.0\n6.0\n2\n-3\nبج\n3\n",
    );
}

#[test]
fn test_member_append_aliases_preserve_non_array_class_methods() {
    assert_prints(
        r#"
صنف حامل {
    منشئ() {}
    عام دالة ألحق(س: نص) -> نص { أرجع س + "!" }
    عام دالة أضف(س: نص) -> نص { أرجع س + "؟" }
}
صنف أدوات {
    مشترك دالة ألحق(س: نص) -> نص { أرجع س + "!" }
    مشترك دالة أضف(س: نص) -> نص { أرجع س + "؟" }
}
متغير ح = جديد حامل()
اطبع(ح.ألحق("أ"))
اطبع(ح.أضف("ب"))
اطبع(أدوات.ألحق("ج"))
اطبع(أدوات.أضف("د"))
"#,
        "أ!\nب؟\nج!\nد؟\n",
    );
}
