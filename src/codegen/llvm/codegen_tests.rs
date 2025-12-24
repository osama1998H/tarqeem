//! Comprehensive tests for LLVM code generation
//!
//! These tests verify that the LLVM IR code generator correctly converts
//! Tarqeem IR to valid LLVM IR text format.

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

// =============================================================================
// Module Header Tests
// =============================================================================

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

    // Should emit string and array struct types
    assert!(result.contains("%struct.TrqString = type"));
    assert!(result.contains("%struct.TrqArray = type"));
}

#[test]
fn test_runtime_declarations_emitted() {
    let mut codegen = create_codegen();
    let module = create_test_module("test");

    let result = codegen.generate(&module).unwrap();

    // Should declare runtime functions
    assert!(result.contains("declare ptr @trq_alloc(i64)"));
    assert!(result.contains("declare void @trq_free(ptr)"));
    assert!(result.contains("declare ptr @trq_string_new(ptr, i64)"));
    assert!(result.contains("declare ptr @trq_array_new(i64, i64)"));
    assert!(result.contains("declare void @trq_print(ptr)"));
}

// =============================================================================
// Constant Generation Tests
// =============================================================================

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

    // Integer constant uses add with 0
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

    // Float constant uses fadd with 0.0
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

    // Add string to string table
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

    // Should emit string literal and call trq_string_new
    assert!(result.contains("@.str.0"));
    assert!(result.contains("call ptr @trq_string_new"));
}

// =============================================================================
// Binary Operation Tests
// =============================================================================

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

    // Integer power uses runtime function
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

    // Float power uses LLVM intrinsic
    assert!(result.contains("call double @llvm.pow.f64"));
}

// =============================================================================
// Comparison Tests
// =============================================================================

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

// =============================================================================
// Logical Operation Tests
// =============================================================================

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

// =============================================================================
// Bitwise Operation Tests
// =============================================================================

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

// =============================================================================
// Unary Operation Tests
// =============================================================================

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

// =============================================================================
// Type Conversion Tests
// =============================================================================

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

// =============================================================================
// Control Flow Tests
// =============================================================================

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

// =============================================================================
// Memory Operation Tests
// =============================================================================

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

    // Alloca
    func.blocks[0].instructions.push(Instruction::Alloca {
        dest: VarId(0),
        ty: IrType::Int,
    });

    // Const value
    func.blocks[0].instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(42),
        ty: IrType::Int,
    });

    // Store
    func.blocks[0].instructions.push(Instruction::Store {
        ptr: VarId(0),
        value: VarId(1),
    });

    // Load
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

// =============================================================================
// Array Operation Tests
// =============================================================================

#[test]
fn test_new_array() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    let mut func = create_test_function("array_test", vec![], IrType::Void);

    // Create constants for array elements
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

    // Create array
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

// =============================================================================
// String Operation Tests
// =============================================================================

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

// =============================================================================
// Function Call Tests
// =============================================================================

#[test]
fn test_function_call() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    // Define a simple add function
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

    // Define a main function that calls add
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

    // Define a void function
    let mut void_func = create_test_function("do_nothing", vec![], IrType::Void);
    void_func.blocks[0]
        .instructions
        .push(Instruction::Return { value: None });
    module.functions.push(void_func);

    // Call it from main
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

// =============================================================================
// Class and Object Tests
// =============================================================================

#[test]
fn test_class_definition() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    // Define a simple class
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

    // Define class
    let mut class = Class::new(ClassId("Point".to_string()), "Point".to_string());
    class.fields.push(("x".to_string(), IrType::Int));
    class.fields.push(("y".to_string(), IrType::Int));
    module.classes.push(class);

    // Create instance
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

// =============================================================================
// Print Tests
// =============================================================================

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

// =============================================================================
// C Main Entry Point Tests
// =============================================================================

#[test]
fn test_c_main_entry_point() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    // Create __main__ function (the entry point)
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

    // Should emit C main that calls __main__
    assert!(result.contains("define i32 @main()"));
    assert!(result.contains("call void @__main__()"));
    assert!(result.contains("ret i32 0"));
}

// =============================================================================
// Arabic Name Mangling Tests
// =============================================================================

#[test]
fn test_arabic_function_name_mangling() {
    let mut codegen = create_codegen();
    let mut module = create_test_module("test");

    // Create function with Arabic name (non-builtin)
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

    // Non-builtin Arabic name should be mangled (not appear directly)
    assert!(!result.contains("define void @دالتي_الخاصة"));
    assert!(result.contains("_U")); // Mangled encoding
}

// =============================================================================
// Phi Function Tests
// =============================================================================

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

    // Entry block with branch
    func.blocks[0].instructions.push(Instruction::Branch {
        cond: VarId(0),
        then_block: BlockId(1),
        else_block: BlockId(2),
    });

    // Then block
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

    // Else block
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

    // Merge block with phi
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
