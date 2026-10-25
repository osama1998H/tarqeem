//! Array pushes retain their operands and effects through every optimization level.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tarqeem::codegen::{Linker, LlvmCodegen, Target};
use tarqeem::interpreter::Interpreter;
use tarqeem::ir::opt::LoopAnalysis;
use tarqeem::ir::{
    BasicBlock, BinaryOp, BlockId, Constant, FuncId, Function, Instruction, IrType, Module,
    OptLevel, Optimizer, VarId,
};
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

fn runtime_library() -> PathBuf {
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
    });
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let library = if cfg!(windows) { "trq.lib" } else { "libtrq.a" };
    let path = project_root().join("target").join(profile).join(library);
    assert!(path.exists(), "{}", path.display());
    path
}

fn write_program(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("إلحاق_مصفوفة.ترقيم");
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
    let args = ["run", source.to_str().unwrap()];
    let output = run_command(Path::new(TARQEEM), &args, dir.path());
    assert_eq!(output.status, Some(0), "interpreter: {}", output.stderr);
    assert_eq!(output.stdout, expected, "interpreter: {output:?}");

    runtime_library();
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

#[test]
fn test_array_push_loop_source_controls() {
    assert_prints(
        r#"
متغير م: مصفوفة<عدد> = []
لكل (متغير ي = ٠؛ ي < ٠؛ ي++) { ألحق(م، ٩٩) }
اطبع(طول(م))
لكل (متغير ي = ٠؛ ي < ٣؛ ي++) {
    ألحق(م، ٧)
    م.ألحق(ي)
}
اطبع(طول(م))
اطبع(م[٠])
اطبع(م[١])
اطبع(م[٢])
اطبع(م[٣])
اطبع(م[٤])
اطبع(م[٥])
"#,
        "0\n6\n7\n0\n7\n1\n7\n2\n",
    );
}

#[test]
fn test_array_push_rewrites_cse_value_operands() {
    assert_prints(
        r#"
دالة أضف(م: مصفوفة<عدد>، أ: عدد، ب: عدد) {
    اطبع(أ + ب)
    ألحق(م، أ + ب)
    م.ألحق(ب + أ)
    اطبع(-أ)
    ألحق(م، -أ)
    م.ألحق(-أ)
}
متغير م: مصفوفة<عدد> = []
أضف(م، ٣، ٤)
اطبع(طول(م))
اطبع(م[٠])
اطبع(م[١])
اطبع(م[٢])
اطبع(م[٣])
"#,
        "7\n-3\n4\n7\n7\n-3\n-3\n",
    );
}

// The source builder leaves CFG edges empty; explicit edges make this loop reach LICM.
fn loop_module(iterations: i64) -> Module {
    let mut entry = BasicBlock::new(BlockId(0));
    for (id, value) in [(0, 0), (1, 1), (3, 7), (4, 9)] {
        entry.instructions.push(Instruction::Const {
            dest: VarId(id),
            value: Constant::Int(value),
            ty: IrType::Int,
        });
    }
    let mut conditions = vec![VarId(0)];
    conditions.extend(std::iter::repeat_n(VarId(1), iterations as usize));
    entry.instructions.extend([
        Instruction::NewArray {
            dest: VarId(2),
            elem_ty: IrType::Int,
            elements: conditions,
        },
        Instruction::NewArray {
            dest: VarId(5),
            elem_ty: IrType::Int,
            elements: vec![],
        },
        Instruction::Jump { target: BlockId(1) },
    ]);
    entry.successors = vec![BlockId(1)];

    let mut header = BasicBlock::new(BlockId(1));
    header.predecessors = vec![BlockId(0), BlockId(2)];
    header.successors = vec![BlockId(2), BlockId(3)];
    header.instructions = vec![
        Instruction::ArrayPop {
            dest: VarId(6),
            array: VarId(2),
            elem_ty: IrType::Int,
        },
        Instruction::Binary {
            dest: VarId(7),
            op: BinaryOp::Gt,
            left: VarId(6),
            right: VarId(0),
            ty: IrType::Bool,
        },
        Instruction::Branch {
            cond: VarId(7),
            then_block: BlockId(2),
            else_block: BlockId(3),
        },
    ];

    let mut body = BasicBlock::new(BlockId(2));
    body.predecessors = vec![BlockId(1)];
    body.successors = vec![BlockId(1)];
    body.instructions = vec![
        Instruction::ArrayPush {
            array: VarId(5),
            value: VarId(3),
            elem_ty: IrType::Int,
        },
        Instruction::ArrayPush {
            array: VarId(5),
            value: VarId(4),
            elem_ty: IrType::Int,
        },
        Instruction::Jump { target: BlockId(1) },
    ];

    let mut exit = BasicBlock::new(BlockId(3));
    exit.predecessors = vec![BlockId(1)];
    exit.instructions = vec![
        Instruction::ArrayLen {
            dest: VarId(9),
            array: VarId(5),
        },
        Instruction::Print { value: VarId(9) },
    ];
    for index in 0..(iterations * 2) {
        let dest = 10 + index as u32 * 2;
        exit.instructions.extend([
            Instruction::Const {
                dest: VarId(dest),
                value: Constant::Int(index),
                ty: IrType::Int,
            },
            Instruction::ArrayGet {
                dest: VarId(dest + 1),
                array: VarId(5),
                index: VarId(dest),
                elem_ty: IrType::Int,
            },
            Instruction::Print {
                value: VarId(dest + 1),
            },
        ]);
    }
    exit.instructions.push(Instruction::Return { value: None });

    let mut function = Function::new(
        FuncId("__main__".into()),
        "رئيسية".into(),
        vec![],
        IrType::Void,
    );
    function.blocks = vec![entry, header, body, exit];
    function.var_counter = 10 + iterations as u32 * 4;
    function.block_counter = 4;
    let loops = LoopAnalysis::analyze(&function);
    assert_eq!(loops.loops.len(), 1);
    assert_eq!(loops.loops[0].preheader, Some(BlockId(0)));
    let mut module = Module::new("حلقة_إلحاق".into());
    module.functions.push(function);
    module
}

fn assert_optimized_loop(iterations: i64, expected: &str) {
    let module = loop_module(iterations);
    let mut interpreter = Interpreter::new(module.clone());
    interpreter.capture_output(true);
    interpreter.run().unwrap();
    assert_eq!(
        format!("{}\n", interpreter.get_output().join("\n")),
        expected
    );

    let runtime = runtime_library();
    let dir = TempDir::new().unwrap();
    for (number, level) in [OptLevel::O0, OptLevel::O1, OptLevel::O2, OptLevel::O3]
        .into_iter()
        .enumerate()
    {
        let mut optimized = module.clone();
        Optimizer::new(level).optimize(&mut optimized);
        let llvm = LlvmCodegen::new(Target::native())
            .generate(&optimized)
            .unwrap();
        let executable = dir
            .path()
            .join(format!("loop_O{number}{}", std::env::consts::EXE_SUFFIX));
        Linker::new(Target::native())
            .optimization_level(number as u32)
            .compile_to_executable(&llvm, &executable, Some(&runtime))
            .unwrap_or_else(|error| panic!("compile O{number}: {error}\n{optimized}"));
        let output = run_command(&executable, &[], dir.path());
        assert_eq!(output.status, Some(0), "native O{number}: {output:?}");
        assert_eq!(output.stdout, expected, "native O{number}: {output:?}");
    }
}

#[test]
fn test_array_push_does_not_run_before_zero_iteration_loop() {
    assert_optimized_loop(0, "0\n");
}

#[test]
fn test_array_push_runs_in_order_on_every_loop_iteration() {
    assert_optimized_loop(3, "6\n7\n9\n7\n9\n7\n9\n");
}
