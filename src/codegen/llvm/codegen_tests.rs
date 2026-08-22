//! Comprehensive tests for LLVM code generation
//!
//! These tests verify that the LLVM IR code generator correctly converts
//! Tarqeem IR to valid LLVM IR text format.

#![allow(clippy::approx_constant)]

use super::codegen::*;
use crate::codegen::Target;
use crate::ir::*;

fn create_test_module(name: &str) -> Module {
    Module::new(name.to_string())
}

fn create_test_function(name: &str, params: Vec<Parameter>, return_type: IrType) -> Function {
    let mut func = Function::new(
        FuncId(name.to_string()),
        name.to_string(),
        params,
        return_type,
    );
    func.blocks
        .push(BasicBlock::with_label(BlockId(0), "entry".to_string()));
    func
}

fn create_codegen() -> LlvmCodegen {
    LlvmCodegen::new(Target::native())
}

#[test]
fn test_module_header_generation() {
    let mut codegen = create_codegen();
    let module = create_test_module("test_module");

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("; ModuleID = 'test_module'"));
    assert!(result.contains("source_filename = \"test_module\""));
    assert!(result.contains("target datalayout ="));
    assert!(result.contains("target triple ="));
}

#[test]
fn test_runtime_types_emitted() {
    let mut codegen = create_codegen();
    let module = create_test_module("test");

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("%struct.TrqString = type"));
    assert!(result.contains("%struct.TrqArray = type"));
}

#[test]
fn test_runtime_declarations_emitted() {
    let mut codegen = create_codegen();
    let module = create_test_module("test");

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("declare ptr @trq_alloc(i64)"));
    assert!(result.contains("declare void @trq_free(ptr)"));
    assert!(result.contains("declare ptr @trq_string_new(ptr, i64)"));
    assert!(result.contains("declare ptr @trq_array_new(i64, i64)"));
    assert!(result.contains("declare void @trq_print(ptr)"));
}

#[test]
fn test_const_int_generation() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function("test_fn", vec![], IrType::Void);
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(42),
        ty: IrType::Int,
    });
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("= add i64 42, 0"));
}

#[test]
fn test_const_float_generation() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function("test_fn", vec![], IrType::Void);
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Float(3.14),
        ty: IrType::Float,
    });
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("fadd double"));
    assert!(result.contains("0.0"));
}

#[test]
fn test_const_bool_true_generation() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function("test_fn", vec![], IrType::Void);
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Bool(true),
        ty: IrType::Bool,
    });
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("select i1 true, i1 true, i1 false"));
}

#[test]
fn test_const_bool_false_generation() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function("test_fn", vec![], IrType::Void);
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Bool(false),
        ty: IrType::Bool,
    });
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("select i1 false, i1 true, i1 false"));
}

#[test]
fn test_const_null_generation() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function("test_fn", vec![], IrType::Void);
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Null,
        ty: IrType::Ptr(Box::new(IrType::Int)),
    });
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("bitcast ptr null to ptr"));
}

#[test]
fn test_const_string_generation() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let str_idx = module.strings.add("مرحبا".to_string());

    let mut func = create_test_function("test_fn", vec![], IrType::Void);
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::String(str_idx),
        ty: IrType::String,
    });
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("@.str.0"));
    assert!(result.contains("call ptr @trq_string_new"));
}

#[test]
fn test_binary_add_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "add_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Add,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("add i64"));
}

#[test]
fn test_binary_sub_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "sub_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Sub,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("sub i64"));
}

#[test]
fn test_binary_mul_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "mul_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Mul,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("mul i64"));
}

#[test]
fn test_binary_div_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "div_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Div,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("sdiv i64"));
}

#[test]
fn test_binary_mod_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "mod_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Mod,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("srem i64"));
}

#[test]
fn test_binary_pow_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "pow_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Pow,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("call i64 @trq_pow_int"));
}

#[test]
fn test_binary_fadd() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "fadd_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Float,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Float,
            },
        ],
        IrType::Float,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Add,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Float,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("fadd double"));
}

#[test]
fn test_binary_fsub() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "fsub_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Float,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Float,
            },
        ],
        IrType::Float,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Sub,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Float,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("fsub double"));
}

#[test]
fn test_binary_fmul() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "fmul_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Float,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Float,
            },
        ],
        IrType::Float,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Mul,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Float,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("fmul double"));
}

#[test]
fn test_binary_fdiv() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "fdiv_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Float,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Float,
            },
        ],
        IrType::Float,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Div,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Float,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("fdiv double"));
}

#[test]
fn test_binary_pow_float() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "fpow_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Float,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Float,
            },
        ],
        IrType::Float,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Pow,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Float,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("call double @llvm.pow.f64"));
}

#[test]
fn test_comparison_eq_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "eq_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Bool,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Eq,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("icmp eq i64"));
}

#[test]
fn test_comparison_ne_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "ne_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Bool,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Ne,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("icmp ne i64"));
}

#[test]
fn test_comparison_lt_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "lt_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Bool,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Lt,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("icmp slt i64"));
}

#[test]
fn test_comparison_le_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "le_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Bool,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Le,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("icmp sle i64"));
}

#[test]
fn test_comparison_gt_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "gt_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Bool,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Gt,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("icmp sgt i64"));
}

#[test]
fn test_comparison_ge_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "ge_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Bool,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Ge,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("icmp sge i64"));
}

#[test]
fn test_comparison_float_eq() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "feq_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Float,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Float,
            },
        ],
        IrType::Bool,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Eq,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Float,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("fcmp oeq double"));
}

#[test]
fn test_comparison_float_lt() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "flt_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Float,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Float,
            },
        ],
        IrType::Bool,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Lt,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Float,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("fcmp olt double"));
}

#[test]
fn test_logical_and() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "and_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Bool,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Bool,
            },
        ],
        IrType::Bool,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::And,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Bool,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("and i1"));
}

#[test]
fn test_logical_or() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "or_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Bool,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Bool,
            },
        ],
        IrType::Bool,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Or,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Bool,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("or i1"));
}

#[test]
fn test_bitwise_and() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "bitand_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::BitAnd,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("and i64"));
}

#[test]
fn test_bitwise_or() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "bitor_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::BitOr,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("or i64"));
}

#[test]
fn test_bitwise_xor() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "bitxor_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::BitXor,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("xor i64"));
}

#[test]
fn test_shift_left() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "shl_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Shl,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("shl i64"));
}

#[test]
fn test_shift_right() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "shr_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Shr,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("ashr i64"));
}

#[test]
fn test_unary_neg_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "neg_test",
        vec![Parameter {
            id: VarId(0),
            name: "a".to_string(),
            ty: IrType::Int,
        }],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Unary {
        dest: VarId(1),
        op: UnaryOp::Neg,
        operand: VarId(0),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("sub i64 0,"));
}

#[test]
fn test_unary_neg_float() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "fneg_test",
        vec![Parameter {
            id: VarId(0),
            name: "a".to_string(),
            ty: IrType::Float,
        }],
        IrType::Float,
    );
    func.blocks[0].instructions.push(Instruction::Unary {
        dest: VarId(1),
        op: UnaryOp::Neg,
        operand: VarId(0),
        ty: IrType::Float,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("fneg double"));
}

#[test]
fn test_unary_not() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "not_test",
        vec![Parameter {
            id: VarId(0),
            name: "a".to_string(),
            ty: IrType::Bool,
        }],
        IrType::Bool,
    );
    func.blocks[0].instructions.push(Instruction::Unary {
        dest: VarId(1),
        op: UnaryOp::Not,
        operand: VarId(0),
        ty: IrType::Bool,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("xor i1"));
    assert!(result.contains("true"));
}

#[test]
fn test_unary_bitnot() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "bitnot_test",
        vec![Parameter {
            id: VarId(0),
            name: "a".to_string(),
            ty: IrType::Int,
        }],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Unary {
        dest: VarId(1),
        op: UnaryOp::BitNot,
        operand: VarId(0),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("xor i64"));
    assert!(result.contains("-1"));
}

#[test]
fn test_int_to_float_conversion() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "convert_test",
        vec![Parameter {
            id: VarId(0),
            name: "a".to_string(),
            ty: IrType::Int,
        }],
        IrType::Float,
    );
    func.blocks[0].instructions.push(Instruction::IntToFloat {
        dest: VarId(1),
        src: VarId(0),
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("sitofp i64"));
    assert!(result.contains("to double"));
}

#[test]
fn test_float_to_int_conversion() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "convert_test",
        vec![Parameter {
            id: VarId(0),
            name: "a".to_string(),
            ty: IrType::Float,
        }],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::FloatToInt {
        dest: VarId(1),
        src: VarId(0),
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("fptosi double"));
    assert!(result.contains("to i64"));
}

#[test]
fn test_unconditional_jump() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function("jump_test", vec![], IrType::Void);
    func.blocks[0]
        .instructions
        .push(Instruction::Jump { target: BlockId(1) });

    let mut block1 = BasicBlock::new(BlockId(1));
    block1
        .instructions
        .push(Instruction::Return { value: None });
    func.blocks.push(block1);

    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("br label %"));
}

#[test]
fn test_conditional_branch() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "branch_test",
        vec![Parameter {
            id: VarId(0),
            name: "cond".to_string(),
            ty: IrType::Bool,
        }],
        IrType::Void,
    );
    func.blocks[0].instructions.push(Instruction::Branch {
        cond: VarId(0),
        then_block: BlockId(1),
        else_block: BlockId(2),
    });

    let mut then_block = BasicBlock::with_label(BlockId(1), "then".to_string());
    then_block
        .instructions
        .push(Instruction::Return { value: None });
    func.blocks.push(then_block);

    let mut else_block = BasicBlock::with_label(BlockId(2), "else".to_string());
    else_block
        .instructions
        .push(Instruction::Return { value: None });
    func.blocks.push(else_block);

    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("br i1"));
    assert!(result.contains("label %"));
}

#[test]
fn test_return_void() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function("void_test", vec![], IrType::Void);
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("ret void"));
}

#[test]
fn test_return_value() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "ret_test",
        vec![Parameter {
            id: VarId(0),
            name: "x".to_string(),
            ty: IrType::Int,
        }],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(0)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("ret i64"));
}

#[test]
fn test_alloca_instruction() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function("alloca_test", vec![], IrType::Void);
    func.blocks[0].instructions.push(Instruction::Alloca {
        dest: VarId(0),
        ty: IrType::Int,
    });
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("alloca i64"));
}

#[test]
fn test_load_store_instructions() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function("loadstore_test", vec![], IrType::Int);

    func.blocks[0].instructions.push(Instruction::Alloca {
        dest: VarId(0),
        ty: IrType::Int,
    });

    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(42),
        ty: IrType::Int,
    });

    func.blocks[0].instructions.push(Instruction::Store {
        ptr: VarId(0),
        value: VarId(1),
    });

    func.blocks[0].instructions.push(Instruction::Load {
        dest: VarId(2),
        ptr: VarId(0),
        ty: IrType::Int,
    });

    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("alloca i64"));
    assert!(result.contains("store i64"));
    assert!(result.contains("load i64"));
}

#[test]
fn test_new_array() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function("array_test", vec![], IrType::Void);

    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(2),
        ty: IrType::Int,
    });

    func.blocks[0].instructions.push(Instruction::NewArray {
        dest: VarId(2),
        elem_ty: IrType::Int,
        elements: vec![VarId(0), VarId(1)],
    });

    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("call ptr @trq_array_new"));
}

#[test]
fn test_array_len() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "array_len_test",
        vec![Parameter {
            id: VarId(0),
            name: "arr".to_string(),
            ty: IrType::Array(Box::new(IrType::Int), 0),
        }],
        IrType::Int,
    );

    func.blocks[0].instructions.push(Instruction::ArrayLen {
        dest: VarId(1),
        array: VarId(0),
    });

    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("call i64 @trq_array_len"));
}

#[test]
fn test_array_get() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "array_get_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "arr".to_string(),
                ty: IrType::Array(Box::new(IrType::Int), 0),
            },
            Parameter {
                id: VarId(1),
                name: "idx".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );

    func.blocks[0].instructions.push(Instruction::ArrayGet {
        dest: VarId(2),
        array: VarId(0),
        index: VarId(1),
        elem_ty: IrType::Int,
    });

    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("call ptr @trq_array_get"));
    assert!(result.contains("load i64"));
}

/// `ArrayPop` lowers the way `ArrayGet` does — a call answering a borrowed
/// pointer, then an immediate load at the element type. The element type is
/// what the load reads, so a float array must emit `load double`, not `load i64`.
#[test]
fn test_array_pop() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "array_pop_test",
        vec![Parameter {
            id: VarId(0),
            name: "arr".to_string(),
            ty: IrType::Array(Box::new(IrType::Float), 0),
        }],
        IrType::Float,
    );

    func.blocks[0].instructions.push(Instruction::ArrayPop {
        dest: VarId(1),
        array: VarId(0),
        elem_ty: IrType::Float,
    });

    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("declare ptr @trq_array_pop(ptr)"));
    assert!(result.contains("call ptr @trq_array_pop"));
    assert!(result.contains("load double"));
}

#[test]
fn test_string_concat() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "concat_test",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::String,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::String,
            },
        ],
        IrType::String,
    );

    func.blocks[0].instructions.push(Instruction::StringConcat {
        dest: VarId(2),
        left: VarId(0),
        right: VarId(1),
    });

    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("call ptr @trq_string_concat"));
}

#[test]
fn test_function_call() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut add_func = create_test_function(
        "add",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    add_func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Add,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    add_func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(add_func);

    let mut main_func = create_test_function("main", vec![], IrType::Int);
    main_func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    main_func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(2),
        ty: IrType::Int,
    });
    main_func.blocks[0].instructions.push(Instruction::Call {
        dest: Some(VarId(2)),
        func: FuncId("add".to_string()),
        args: vec![VarId(0), VarId(1)],
        ret_ty: IrType::Int,
    });
    main_func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(main_func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("call i64 @add("));
}

#[test]
fn test_void_function_call() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut void_func = create_test_function("do_nothing", vec![], IrType::Void);
    void_func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(void_func);

    let mut main_func = create_test_function("main", vec![], IrType::Void);
    main_func.blocks[0].instructions.push(Instruction::Call {
        dest: None,
        func: FuncId("do_nothing".to_string()),
        args: vec![],
        ret_ty: IrType::Void,
    });
    main_func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(main_func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("call void @do_nothing()"));
}

#[test]
fn test_class_definition() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut class = Class::new(ClassId("Person".to_string()), "Person".to_string());
    class.fields.push(("name".to_string(), IrType::String));
    class.fields.push(("age".to_string(), IrType::Int));
    module.classes.push(class);

    let mut func = create_test_function("main", vec![], IrType::Void);
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("%class.Person = type"));
}

#[test]
fn test_new_object() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut class = Class::new(ClassId("Point".to_string()), "Point".to_string());
    class.fields.push(("x".to_string(), IrType::Int));
    class.fields.push(("y".to_string(), IrType::Int));
    module.classes.push(class);

    let mut func = create_test_function("main", vec![], IrType::Void);
    func.blocks[0].instructions.push(Instruction::NewObject {
        dest: VarId(0),
        class: ClassId("Point".to_string()),
    });
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("call ptr @trq_alloc"));
}

/// The allocation must cover the struct LLVM actually lays out, padding
/// included — asserted here rather than by running a program, because an
/// overrun this small lands in `trq_alloc`'s rounding slack and a fixture
/// reading the field back passes either way.
///
/// `{ ptr, i1, i64 }` puts the `i64` at offset 16, so the object needs 24
/// bytes. Summing bare field sizes asked for 8 + 1 + 8 = 17 and the
/// constructor's store then ran seven bytes past the end.
#[test]
fn test_new_object_allocation_covers_field_padding() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut class = Class::new(ClassId("Padded".to_string()), "Padded".to_string());
    class.fields.push(("flag".to_string(), IrType::Bool));
    class.fields.push(("count".to_string(), IrType::Int));
    module.classes.push(class);

    let mut func = create_test_function("main", vec![], IrType::Void);
    func.blocks[0].instructions.push(Instruction::NewObject {
        dest: VarId(0),
        class: ClassId("Padded".to_string()),
    });
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(
        result.contains("%class.Padded = type { ptr, i1, i64 }"),
        "layout changed; update the expected size below\n{result}"
    );
    assert!(
        result.contains("call ptr @trq_alloc(i64 24)"),
        "allocation does not cover the padded layout\n{result}"
    );
}

#[test]
fn test_print_int() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function("main", vec![], IrType::Void);
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(42),
        ty: IrType::Int,
    });
    func.blocks[0]
        .instructions
        .push(Instruction::Print { value: VarId(0) });
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("call void @trq_print_int"));
    assert!(result.contains("call void @trq_print_newline"));
}

#[test]
fn test_print_string() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let str_idx = module.strings.add("Hello".to_string());

    let mut func = create_test_function("main", vec![], IrType::Void);
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::String(str_idx),
        ty: IrType::String,
    });
    func.blocks[0]
        .instructions
        .push(Instruction::Print { value: VarId(0) });
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("call void @trq_print(ptr"));
}

#[test]
fn test_main_entry_point() {
    // The C main() entry point is provided by the runtime library (builtins.c),
    // not by codegen. Codegen only generates __main__() which the runtime calls.
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut main_func = Function::new(
        FuncId("__main__".to_string()),
        "__main__".to_string(),
        vec![],
        IrType::Void,
    );
    main_func
        .blocks
        .push(BasicBlock::with_label(BlockId(0), "entry".to_string()));
    main_func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(main_func);

    let result = codegen.generate(&module).unwrap();

    // Verify __main__ is generated (called by runtime's main())
    assert!(result.contains("define void @__main__()"));
    // Verify we do NOT generate main() (runtime provides it)
    assert!(!result.contains("define i32 @main()"));
}

#[test]
fn test_arabic_function_name_mangling() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = Function::new(
        FuncId("دالتي_الخاصة".to_string()),
        "دالتي_الخاصة".to_string(),
        vec![],
        IrType::Void,
    );
    func.blocks
        .push(BasicBlock::with_label(BlockId(0), "entry".to_string()));
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(!result.contains("define void @دالتي_الخاصة"));
    assert!(result.contains("_U")); // Mangled encoding
}

#[test]
fn test_phi_function() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function(
        "phi_test",
        vec![Parameter {
            id: VarId(0),
            name: "cond".to_string(),
            ty: IrType::Bool,
        }],
        IrType::Int,
    );

    func.blocks[0].instructions.push(Instruction::Branch {
        cond: VarId(0),
        then_block: BlockId(1),
        else_block: BlockId(2),
    });

    let mut then_block = BasicBlock::with_label(BlockId(1), "then".to_string());
    then_block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    then_block
        .instructions
        .push(Instruction::Jump { target: BlockId(3) });
    func.blocks.push(then_block);

    let mut else_block = BasicBlock::with_label(BlockId(2), "else".to_string());
    else_block.instructions.push(Instruction::Const {
        dest: VarId(2),
        value: Constant::Int(0),
        ty: IrType::Int,
    });
    else_block
        .instructions
        .push(Instruction::Jump { target: BlockId(3) });
    func.blocks.push(else_block);

    let mut merge_block = BasicBlock::with_label(BlockId(3), "merge".to_string());
    merge_block.instructions.push(Instruction::Phi {
        dest: VarId(3),
        ty: IrType::Int,
        incoming: vec![(VarId(1), BlockId(1)), (VarId(2), BlockId(2))],
    });
    merge_block.instructions.push(Instruction::Return {
        value: Some(VarId(3)),
    });
    func.blocks.push(merge_block);

    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("phi i64"));
}

// =============================================================================
// WASM Code Generation Tests
// =============================================================================

fn create_wasm_codegen() -> LlvmCodegen {
    LlvmCodegen::new(Target::wasm32())
}

#[test]
fn test_wasm_target_module_header() {
    let mut codegen = create_wasm_codegen();
    let module = create_test_module("wasm_test");

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("; ModuleID = 'wasm_test'"));
    assert!(result.contains("target triple = \"wasm32-unknown-unknown\""));
    assert!(result.contains("p:32:32")); // WASM uses 32-bit pointers
}

#[test]
fn test_wasm_target_data_layout() {
    let mut codegen = create_wasm_codegen();
    let module = create_test_module("wasm_test");

    let result = codegen.generate(&module).unwrap();

    // WASM data layout should have little-endian and 32-bit pointers
    assert!(result.contains("e-m:e-p:32:32"));
}

#[test]
fn test_wasm_simple_function() {
    let mut codegen = create_wasm_codegen();
    let mut module = create_test_module("wasm_test");

    let mut func = create_test_function(
        "add",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Add,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    // WASM function definition should exist and contain the add operation
    assert!(result.contains("define i64 @add"));
    assert!(result.contains("i64 %arg.0")); // Parameters use arg.N format
    assert!(result.contains("i64 %arg.1"));
    assert!(result.contains("add i64"));
}

#[test]
fn test_wasm_arabic_function_name() {
    let mut codegen = create_wasm_codegen();
    let mut module = create_test_module("wasm_test");

    let mut func = Function::new(
        FuncId("جمع".to_string()),
        "جمع".to_string(),
        vec![
            Parameter {
                id: VarId(0),
                name: "أ".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "ب".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );
    func.blocks
        .push(BasicBlock::with_label(BlockId(0), "entry".to_string()));
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Add,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    // Should generate a function with mangled Arabic name
    assert!(result.contains("@_U"));
    assert!(result.contains("add i64"));
}

#[test]
fn test_wasm_runtime_declarations() {
    let mut codegen = create_wasm_codegen();
    let module = create_test_module("wasm_test");

    let result = codegen.generate(&module).unwrap();

    // WASM runtime should declare memory management functions
    assert!(result.contains("declare ptr @trq_alloc"));
    assert!(result.contains("declare void @trq_free"));
}

#[test]
fn test_wasm_arithmetic_operations() {
    let mut codegen = create_wasm_codegen();
    let mut module = create_test_module("wasm_test");

    // Create a function with multiple operations
    let mut func = create_test_function(
        "calculate",
        vec![
            Parameter {
                id: VarId(0),
                name: "x".to_string(),
                ty: IrType::Int,
            },
            Parameter {
                id: VarId(1),
                name: "y".to_string(),
                ty: IrType::Int,
            },
        ],
        IrType::Int,
    );

    // x * y
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Mul,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    // x + y
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(3),
        op: BinaryOp::Add,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    // (x * y) - (x + y)
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(4),
        op: BinaryOp::Sub,
        left: VarId(2),
        right: VarId(3),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(4)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("mul i64"));
    assert!(result.contains("add i64"));
    assert!(result.contains("sub i64"));
}

#[test]
fn test_wasm_float_operations() {
    let mut codegen = create_wasm_codegen();
    let mut module = create_test_module("wasm_test");

    let mut func = create_test_function(
        "float_calc",
        vec![
            Parameter {
                id: VarId(0),
                name: "a".to_string(),
                ty: IrType::Float,
            },
            Parameter {
                id: VarId(1),
                name: "b".to_string(),
                ty: IrType::Float,
            },
        ],
        IrType::Float,
    );
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Add,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Float,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("fadd double"));
}

#[test]
fn test_wasm_control_flow() {
    let mut codegen = create_wasm_codegen();
    let mut module = create_test_module("wasm_test");

    let mut func = create_test_function(
        "abs",
        vec![Parameter {
            id: VarId(0),
            name: "x".to_string(),
            ty: IrType::Int,
        }],
        IrType::Int,
    );

    // if x < 0 then -x else x
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(0),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Lt,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Branch {
        cond: VarId(2),
        then_block: BlockId(1),
        else_block: BlockId(2),
    });

    let mut neg_block = BasicBlock::with_label(BlockId(1), "negative".to_string());
    neg_block.instructions.push(Instruction::Unary {
        dest: VarId(3),
        op: UnaryOp::Neg,
        operand: VarId(0),
        ty: IrType::Int,
    });
    neg_block
        .instructions
        .push(Instruction::Jump { target: BlockId(3) });
    func.blocks.push(neg_block);

    let mut pos_block = BasicBlock::with_label(BlockId(2), "positive".to_string());
    pos_block
        .instructions
        .push(Instruction::Jump { target: BlockId(3) });
    func.blocks.push(pos_block);

    let mut merge_block = BasicBlock::with_label(BlockId(3), "merge".to_string());
    merge_block.instructions.push(Instruction::Phi {
        dest: VarId(4),
        ty: IrType::Int,
        incoming: vec![(VarId(3), BlockId(1)), (VarId(0), BlockId(2))],
    });
    merge_block.instructions.push(Instruction::Return {
        value: Some(VarId(4)),
    });
    func.blocks.push(merge_block);

    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("br i1"));
    assert!(result.contains("phi i64"));
}

#[test]
fn test_wasm_vs_native_target() {
    let wasm_target = Target::wasm32();
    let native_target = Target::native();

    // WASM should have 32-bit pointers
    assert_eq!(wasm_target.triple.pointer_bits(), 32);

    // Native is typically 64-bit (on modern systems)
    // Note: This may vary depending on the build machine
    assert!(native_target.triple.pointer_bits() >= 32);

    // WASM-specific checks
    assert!(wasm_target.is_wasm());
    assert!(!native_target.is_wasm());
}

#[test]
fn test_wasm_wasi_target() {
    let wasi_target = Target::wasm32_wasi();

    assert!(wasi_target.is_wasm());
    assert!(wasi_target.is_wasi());
    assert_eq!(wasi_target.triple.arch, "wasm32");
}

#[test]
fn test_wasm_string_operations() {
    let mut codegen = create_wasm_codegen();
    let mut module = create_test_module("wasm_test");

    let str_idx = module.strings.add("مرحبا بالعالم".to_string());

    let mut func = create_test_function("greet", vec![], IrType::String);
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::String(str_idx),
        ty: IrType::String,
    });
    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(0)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("@.str.0"));
    assert!(result.contains("call ptr @trq_string_new"));
}

#[test]
fn test_wasm_array_operations() {
    let mut codegen = create_wasm_codegen();
    let mut module = create_test_module("wasm_test");

    let mut func = create_test_function(
        "make_array",
        vec![],
        IrType::Array(Box::new(IrType::Int), 0),
    );

    // Create elements
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(2),
        ty: IrType::Int,
    });
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(2),
        value: Constant::Int(3),
        ty: IrType::Int,
    });

    func.blocks[0].instructions.push(Instruction::NewArray {
        dest: VarId(3),
        elem_ty: IrType::Int,
        elements: vec![VarId(0), VarId(1), VarId(2)],
    });

    func.blocks[0].instructions.push(Instruction::Return {
        value: Some(VarId(3)),
    });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(result.contains("call ptr @trq_array_new"));
}

// ─── Bool at the Rust FFI boundary must be zero-extended (#266 follow-up) ───

/// An `i1`'s upper byte bits are don't-care to LLVM, so `ليس س` lowers to
/// `xorb $-1, %al` on x86-64 and `false` reaches the runtime as 254. Rust's
/// `extern "C" fn(bool)` admits 0 and 1 only, and its branch arithmetic on an
/// invalid pattern walked into `.rodata`: `اطبع(ليس س)` printed DWARF strings
/// instead of `خطأ`, natively, on x86-64 only — aarch64 happened to produce 0/1.
///
/// `zeroext` is what makes LLVM emit the `andl $1` normalization, and it has to
/// be on the **call site**: that is where the ABI is taken from, so a declaration
/// carrying it alone would not fix the call.
#[test]
fn test_print_bool_passes_zeroext() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function("main", vec![], IrType::Void);
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Bool(false),
        ty: IrType::Bool,
    });
    func.blocks[0]
        .instructions
        .push(Instruction::Print { value: VarId(0) });
    func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(
        result.contains("declare void @trq_print_bool(i1 zeroext)"),
        "the runtime declaration must carry zeroext"
    );
    assert!(
        result.contains("call void @trq_print_bool(i1 zeroext"),
        "the call site must carry zeroext — LLVM takes the ABI from the call, not the declaration"
    );
}

/// `منطقي_لنص` reaches the runtime through the generic call path, which types its
/// arguments with `map_param_type` rather than the hand-written string above. The
/// same mapper spells `define` parameters, which is what keeps a signature and its
/// call sites from disagreeing — so a bool parameter must read `i1 zeroext` too.
#[test]
fn test_bool_parameter_signature_is_zeroext() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let func = create_test_function(
        "خذ_منطقي",
        vec![Parameter {
            id: VarId(0),
            name: "ق".to_string(),
            ty: IrType::Bool,
        }],
        IrType::Void,
    );
    module.functions.push(func);

    let result = codegen.generate(&module).unwrap();

    assert!(
        result.contains("i1 zeroext %arg.0"),
        "a bool parameter must be zero-extended, or a computed i1 argument \
         arrives with garbage in its upper bits"
    );
}
