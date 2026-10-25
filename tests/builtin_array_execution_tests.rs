//! Builtin results keep their element types when stored in array literals.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tarqeem::ir::{Instruction, IrBuilder, IrType};
use tarqeem::parser::Parser;
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

fn assert_prints(body: &str, expected: &str) {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("عناصر_مدمجة.ترقيم");
    fs::write(&source, format!("بسم_الله\n{}\nالحمد_لله\n", body.trim())).unwrap();
    let output = run_command(
        Path::new(TARQEEM),
        &["run", source.to_str().unwrap()],
        dir.path(),
    );
    assert_eq!(output.status, Some(0), "interpreter: {}", output.stderr);
    assert_eq!(output.stdout, expected, "interpreter: {output:?}");

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
fn test_builtin_results_in_global_arrays() {
    assert_prints(
        r#"
ثابت أطوال = [طول_مصفوفة([١])، طول_مصفوفة([١، ٢])]
متغير بتات = [بتات_و(٣، ١)، بتات_أو(١٢، ١٠)]
متغير قيم_أو = [بتات_أو(١٢، ١٠)]
متغير حصري = [بتات_أو_حصري(١٢، ١٠)]
متغير نفي = [بتات_نفي(١٢)]
متغير يسار = [بتات_إزاحة_يسار(٣، ٢)]
متغير يمين = [بتات_إزاحة_يمين(-١٦، ٢)]
متغير قيم_منطقية = [بتات_إزاحة_يمين_منطقية(١٦، ٢)]
اطبع(أطوال[٠])
اطبع(أطوال[١])
اطبع(بتات[٠])
اطبع(بتات[١])
اطبع(قيم_أو[٠])
اطبع(حصري[٠])
اطبع(نفي[٠])
اطبع(يسار[٠])
اطبع(يمين[٠])
اطبع(قيم_منطقية[٠])
"#,
        "1\n2\n1\n14\n14\n6\n-13\n12\n-4\n4\n",
    );
}

#[test]
fn test_builtin_results_in_local_arrays_keep_parameter_values() {
    assert_prints(
        r#"
دالة اعرض(م: مصفوفة<عدد>، أ: عدد، ب: عدد) {
    متغير أطوال = [طول_مصفوفة(م)، طول(م)]
    متغير بتات = [بتات_و(أ، ب)، بتات_أو(أ، ب)، بتات_نفي(أ)]
    متغير قيم_أو = [بتات_أو(أ، ب)]
    متغير حصري = [بتات_أو_حصري(أ، ب)]
    متغير نفي = [بتات_نفي(أ)]
    متغير يسار = [بتات_إزاحة_يسار(أ، ٢)]
    متغير يمين = [بتات_إزاحة_يمين(أ، ٢)]
    متغير إزاحة = [بتات_إزاحة_يمين_منطقية(أ، ٢)]
    اطبع(أطوال[٠])
    اطبع(أطوال[١])
    اطبع(بتات[٠])
    اطبع(بتات[١])
    اطبع(بتات[٢])
    اطبع(قيم_أو[٠])
    اطبع(حصري[٠])
    اطبع(نفي[٠])
    اطبع(يسار[٠])
    اطبع(يمين[٠])
    اطبع(إزاحة[٠])
}
اعرض([٤، ٥، ٦]، ١٢، ١٠)
"#,
        "3\n3\n8\n14\n-13\n14\n6\n-13\n48\n3\n3\n",
    );
}

#[test]
fn test_conversion_results_and_nested_arrays_keep_element_types() {
    assert_prints(
        r#"
متغير عشريات = [عدد_عشري(٢)، عدد_عشري(٣)]
متغير نصوص = [نص(٧)، نوع(١)]
متغير صفوف = [[بتات_و(٣، ١)]، [طول([١، ٢])]]
اطبع(عشريات[٠] + ٠.٥)
اطبع(عشريات[١] + ٠.٥)
اطبع(نصوص[٠] + "!")
اطبع(نصوص[١])
اطبع(صفوف[٠][٠])
اطبع(صفوف[١][٠])
اطبع([بتات_أو(١، ٢)][٠])
"#,
        "2.5\n3.5\n7!\nعدد\n1\n2\n3\n",
    );
}

#[test]
fn test_user_function_array_elements_keep_declared_return_type() {
    assert_prints(
        r#"
دالة بتات_و(أ: عدد، ب: عدد) -> نص { أرجع "مظلل" }
دالة زد(س: عدد) -> عدد { أرجع س + ١ }
دالة اعرض() {
    ثابت بتات_أو: (عدد، عدد) -> نص = (أ، ب) => "قيمة"
    متغير قيم = [بتات_أو(٣، ١)]
    اطبع(قيم[٠] + "!")
}
متغير نصوص = [بتات_و(٣، ١)، بتات_و(٧، ٢)]
متغير أعداد = [زد(١)، زد(٢)]
اطبع(نصوص[٠] + "!")
اطبع(نصوص[١])
اطبع(أعداد[٠])
اطبع(أعداد[١])
اعرض()
"#,
        "مظلل!\nمظلل\n2\n3\nقيمة!\n",
    );
}

#[test]
fn test_builtin_array_literal_ir_uses_integer_elements() {
    let source = "بسم_الله\nمتغير م = [بتات_و(٣، ١)، بتات_أو(١٢، ١٠)]\nاطبع(م[٠])\nالحمد_لله";
    let ast = Parser::new(source).parse().unwrap();
    let module = IrBuilder::new("عناصر_مدمجة".into()).build(&ast).unwrap();
    let instructions = module
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions);
    let mut arrays = 0;
    let mut reads = 0;
    for instruction in instructions {
        match instruction {
            Instruction::NewArray { elem_ty, .. } => {
                assert_eq!(elem_ty, &IrType::Int);
                arrays += 1;
            }
            Instruction::ArrayGet { elem_ty, .. } => {
                assert_eq!(elem_ty, &IrType::Int);
                reads += 1;
            }
            _ => {}
        }
    }
    assert_eq!(arrays, 1);
    assert_eq!(reads, 1);
}
