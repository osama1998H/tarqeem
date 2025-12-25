//! Comprehensive tests for the Interpreter execution engine
//!
//! These tests verify correct execution of IR instructions including
//! arithmetic, control flow, arrays, strings, objects, and built-in functions.

use super::*;
use crate::ir::{
    BasicBlock, BinaryOp, BlockId, Class, ClassId, Constant, FuncId, Function, Instruction, IrType,
    Module, Parameter, UnaryOp, VarId,
};

fn create_empty_module() -> Module {
    Module::new("test".to_string())
}

fn create_main_function() -> Function {
    Function::new(
        FuncId("main".to_string()),
        "main".to_string(),
        vec![],
        IrType::Int,
    )
}

fn run_module(module: Module) -> RuntimeResult<Value> {
    let mut interpreter = Interpreter::new(module);
    interpreter.run()
}

fn run_module_with_output(module: Module) -> (RuntimeResult<Value>, Vec<String>) {
    let mut interpreter = Interpreter::new(module);
    interpreter.capture_output(true);
    let result = interpreter.run();
    (result, interpreter.get_output().to_vec())
}

#[test]
fn test_add_integers() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(10),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(20),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Add,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(30));
}

#[test]
fn test_subtract_integers() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(50),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(30),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Sub,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(20));
}

#[test]
fn test_multiply_integers() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(7),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(8),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Mul,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(56));
}

#[test]
fn test_divide_integers() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(100),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(4),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Div,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(25));
}

#[test]
fn test_modulo_integers() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(17),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(5),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Mod,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(2));
}

#[test]
fn test_power_integers() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(2),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(10),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Pow,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(1024));
}

#[test]
fn test_division_by_zero_error() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(10),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(0),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Div,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    let result = run_module(module);
    assert!(result.is_err());
}

#[test]
fn test_negative_numbers() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(-5),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(3),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Mul,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(-15));
}

#[test]
fn test_add_floats() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Float;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Float(3.5),
        ty: IrType::Float,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Float(2.5),
        ty: IrType::Float,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Add,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Float,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Float(6.0));
}

#[test]
fn test_float_division() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Float;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Float(10.0),
        ty: IrType::Float,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Float(4.0),
        ty: IrType::Float,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Div,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Float,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Float(2.5));
}

#[test]
fn test_mixed_int_float_arithmetic() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Float;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(5),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Float(2.5),
        ty: IrType::Float,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Mul,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Float,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Float(12.5));
}

#[test]
fn test_equals_true() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Bool;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(42),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(42),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Eq,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Bool(true));
}

#[test]
fn test_equals_false() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Bool;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(42),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(43),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Eq,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Bool(false));
}

#[test]
fn test_less_than() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Bool;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(5),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(10),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Lt,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Bool(true));
}

#[test]
fn test_greater_than_or_equal() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Bool;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(10),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(10),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Ge,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Bool(true));
}

#[test]
fn test_logical_and_true() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Bool;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Bool(true),
        ty: IrType::Bool,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Bool(true),
        ty: IrType::Bool,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::And,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Bool,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Bool(true));
}

#[test]
fn test_logical_and_false() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Bool;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Bool(true),
        ty: IrType::Bool,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Bool(false),
        ty: IrType::Bool,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::And,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Bool,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Bool(false));
}

#[test]
fn test_logical_or() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Bool;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Bool(false),
        ty: IrType::Bool,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Bool(true),
        ty: IrType::Bool,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Or,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Bool,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Bool(true));
}

#[test]
fn test_bitwise_and() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(0b1010),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(0b1100),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::BitAnd,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(0b1000));
}

#[test]
fn test_bitwise_or() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(0b1010),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(0b1100),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::BitOr,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(0b1110));
}

#[test]
fn test_bitwise_xor() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(0b1010),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(0b1100),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::BitXor,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(0b0110));
}

#[test]
fn test_shift_left() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(4),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Shl,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(16));
}

#[test]
fn test_shift_right() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(32),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(3),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Shr,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(4));
}

#[test]
fn test_unary_neg_int() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(42),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Unary {
        dest: VarId(1),
        op: UnaryOp::Neg,
        operand: VarId(0),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(-42));
}

#[test]
fn test_unary_neg_float() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Float;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Float(3.14),
        ty: IrType::Float,
    });
    block.instructions.push(Instruction::Unary {
        dest: VarId(1),
        op: UnaryOp::Neg,
        operand: VarId(0),
        ty: IrType::Float,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Float(-3.14));
}

#[test]
fn test_unary_not() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Bool;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Bool(true),
        ty: IrType::Bool,
    });
    block.instructions.push(Instruction::Unary {
        dest: VarId(1),
        op: UnaryOp::Not,
        operand: VarId(0),
        ty: IrType::Bool,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Bool(false));
}

#[test]
fn test_unary_bitnot() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(0),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Unary {
        dest: VarId(1),
        op: UnaryOp::BitNot,
        operand: VarId(0),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(-1));
}

#[test]
fn test_int_to_float() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Float;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(42),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::IntToFloat {
        dest: VarId(1),
        src: VarId(0),
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Float(42.0));
}

#[test]
fn test_float_to_int() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Float(3.7),
        ty: IrType::Float,
    });
    block.instructions.push(Instruction::FloatToInt {
        dest: VarId(1),
        src: VarId(0),
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(3));
}

#[test]
fn test_to_string() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::String;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(42),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::ToString {
        dest: VarId(1),
        src: VarId(0),
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    let result = run_module(module).unwrap();
    assert_eq!(result.to_display_string(), "42");
}

#[test]
fn test_string_concat() {
    let mut module = create_empty_module();
    module.strings.add("Hello, ".to_string());
    module.strings.add("World!".to_string());

    let mut func = create_main_function();
    func.return_type = IrType::String;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::String(0),
        ty: IrType::String,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::String(1),
        ty: IrType::String,
    });
    block.instructions.push(Instruction::StringConcat {
        dest: VarId(2),
        left: VarId(0),
        right: VarId(1),
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    let result = run_module(module).unwrap();
    assert_eq!(result.to_display_string(), "Hello, World!");
}

#[test]
fn test_string_arabic() {
    let mut module = create_empty_module();
    module.strings.add("مرحبا".to_string());

    let mut func = create_main_function();
    func.return_type = IrType::String;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::String(0),
        ty: IrType::String,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(0)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    let result = run_module(module).unwrap();
    assert_eq!(result.to_display_string(), "مرحبا");
}

#[test]
fn test_array_creation() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(2),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(2),
        value: Constant::Int(3),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::NewArray {
        dest: VarId(3),
        elem_ty: IrType::Int,
        elements: vec![VarId(0), VarId(1), VarId(2)],
    });
    block.instructions.push(Instruction::ArrayLen {
        dest: VarId(4),
        array: VarId(3),
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(4)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(3));
}

#[test]
fn test_array_get() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(10),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(20),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(2),
        value: Constant::Int(30),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::NewArray {
        dest: VarId(3),
        elem_ty: IrType::Int,
        elements: vec![VarId(0), VarId(1), VarId(2)],
    });

    block.instructions.push(Instruction::Const {
        dest: VarId(4),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::ArrayGet {
        dest: VarId(5),
        array: VarId(3),
        index: VarId(4),
        elem_ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(5)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(20));
}

#[test]
fn test_array_set() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(2),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(2),
        value: Constant::Int(3),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::NewArray {
        dest: VarId(3),
        elem_ty: IrType::Int,
        elements: vec![VarId(0), VarId(1), VarId(2)],
    });

    block.instructions.push(Instruction::Const {
        dest: VarId(4),
        value: Constant::Int(0),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(5),
        value: Constant::Int(100),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::ArraySet {
        array: VarId(3),
        index: VarId(4),
        value: VarId(5),
    });

    block.instructions.push(Instruction::ArrayGet {
        dest: VarId(6),
        array: VarId(3),
        index: VarId(4),
        elem_ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(6)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(100));
}

#[test]
fn test_array_push() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(2),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::NewArray {
        dest: VarId(2),
        elem_ty: IrType::Int,
        elements: vec![VarId(0), VarId(1)],
    });

    block.instructions.push(Instruction::Const {
        dest: VarId(3),
        value: Constant::Int(3),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::ArrayPush {
        array: VarId(2),
        value: VarId(3),
        elem_ty: IrType::Int,
    });

    block.instructions.push(Instruction::ArrayLen {
        dest: VarId(4),
        array: VarId(2),
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(4)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(3));
}

#[test]
fn test_array_index_out_of_bounds() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::NewArray {
        dest: VarId(1),
        elem_ty: IrType::Int,
        elements: vec![VarId(0)],
    });

    block.instructions.push(Instruction::Const {
        dest: VarId(2),
        value: Constant::Int(5),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::ArrayGet {
        dest: VarId(3),
        array: VarId(1),
        index: VarId(2),
        elem_ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(3)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    let result = run_module(module);
    assert!(result.is_err());
}

#[test]
fn test_branch_true() {
    let mut module = create_empty_module();
    let mut func = create_main_function();

    let mut entry = BasicBlock::new(BlockId(0));
    entry.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Bool(true),
        ty: IrType::Bool,
    });
    entry.instructions.push(Instruction::Branch {
        cond: VarId(0),
        then_block: BlockId(1),
        else_block: BlockId(2),
    });

    let mut then_block = BasicBlock::new(BlockId(1));
    then_block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    then_block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    let mut else_block = BasicBlock::new(BlockId(2));
    else_block.instructions.push(Instruction::Const {
        dest: VarId(2),
        value: Constant::Int(0),
        ty: IrType::Int,
    });
    else_block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(entry);
    func.blocks.push(then_block);
    func.blocks.push(else_block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(1));
}

#[test]
fn test_branch_false() {
    let mut module = create_empty_module();
    let mut func = create_main_function();

    let mut entry = BasicBlock::new(BlockId(0));
    entry.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Bool(false),
        ty: IrType::Bool,
    });
    entry.instructions.push(Instruction::Branch {
        cond: VarId(0),
        then_block: BlockId(1),
        else_block: BlockId(2),
    });

    let mut then_block = BasicBlock::new(BlockId(1));
    then_block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    then_block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    let mut else_block = BasicBlock::new(BlockId(2));
    else_block.instructions.push(Instruction::Const {
        dest: VarId(2),
        value: Constant::Int(0),
        ty: IrType::Int,
    });
    else_block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(entry);
    func.blocks.push(then_block);
    func.blocks.push(else_block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(0));
}

#[test]
fn test_loop_with_jump() {
    let mut module = create_empty_module();
    let mut func = create_main_function();

    let mut entry = BasicBlock::new(BlockId(0));
    entry.instructions.push(Instruction::Alloca {
        dest: VarId(0),
        ty: IrType::Int,
    });
    entry.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(0),
        ty: IrType::Int,
    });
    entry.instructions.push(Instruction::Store {
        ptr: VarId(0),
        value: VarId(1),
    });
    entry
        .instructions
        .push(Instruction::Jump { target: BlockId(1) });

    let mut loop_block = BasicBlock::new(BlockId(1));
    loop_block.instructions.push(Instruction::Load {
        dest: VarId(2),
        ptr: VarId(0),
        ty: IrType::Int,
    });
    loop_block.instructions.push(Instruction::Const {
        dest: VarId(3),
        value: Constant::Int(5),
        ty: IrType::Int,
    });
    loop_block.instructions.push(Instruction::Binary {
        dest: VarId(4),
        op: BinaryOp::Lt,
        left: VarId(2),
        right: VarId(3),
        ty: IrType::Int,
    });
    loop_block.instructions.push(Instruction::Branch {
        cond: VarId(4),
        then_block: BlockId(2),
        else_block: BlockId(3),
    });

    let mut body = BasicBlock::new(BlockId(2));
    body.instructions.push(Instruction::Load {
        dest: VarId(5),
        ptr: VarId(0),
        ty: IrType::Int,
    });
    body.instructions.push(Instruction::Const {
        dest: VarId(6),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    body.instructions.push(Instruction::Binary {
        dest: VarId(7),
        op: BinaryOp::Add,
        left: VarId(5),
        right: VarId(6),
        ty: IrType::Int,
    });
    body.instructions.push(Instruction::Store {
        ptr: VarId(0),
        value: VarId(7),
    });
    body.instructions
        .push(Instruction::Jump { target: BlockId(1) });

    let mut exit = BasicBlock::new(BlockId(3));
    exit.instructions.push(Instruction::Load {
        dest: VarId(8),
        ptr: VarId(0),
        ty: IrType::Int,
    });
    exit.instructions.push(Instruction::Return {
        value: Some(VarId(8)),
    });

    func.blocks.push(entry);
    func.blocks.push(loop_block);
    func.blocks.push(body);
    func.blocks.push(exit);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(5));
}

#[test]
fn test_builtin_len_array() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(2),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::NewArray {
        dest: VarId(2),
        elem_ty: IrType::Int,
        elements: vec![VarId(0), VarId(1)],
    });

    block.instructions.push(Instruction::Call {
        dest: Some(VarId(3)),
        func: FuncId("طول".to_string()),
        args: vec![VarId(2)],
        ret_ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(3)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(2));
}

#[test]
fn test_builtin_len_string() {
    let mut module = create_empty_module();
    module.strings.add("مرحبا".to_string());

    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::String(0),
        ty: IrType::String,
    });
    block.instructions.push(Instruction::Call {
        dest: Some(VarId(1)),
        func: FuncId("len".to_string()),
        args: vec![VarId(0)],
        ret_ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(5));
}

#[test]
fn test_builtin_abs() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(-42),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Call {
        dest: Some(VarId(1)),
        func: FuncId("abs".to_string()),
        args: vec![VarId(0)],
        ret_ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(42));
}

#[test]
fn test_builtin_sqrt() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    func.return_type = IrType::Float;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Float(16.0),
        ty: IrType::Float,
    });
    block.instructions.push(Instruction::Call {
        dest: Some(VarId(1)),
        func: FuncId("sqrt".to_string()),
        args: vec![VarId(0)],
        ret_ty: IrType::Float,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Float(4.0));
}

#[test]
fn test_builtin_int_conversion() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Float(3.9),
        ty: IrType::Float,
    });
    block.instructions.push(Instruction::Call {
        dest: Some(VarId(1)),
        func: FuncId("عدد".to_string()),
        args: vec![VarId(0)],
        ret_ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(3));
}

#[test]
fn test_builtin_print() {
    let mut module = create_empty_module();
    module.strings.add("Hello".to_string());

    let mut func = create_main_function();
    func.return_type = IrType::Void;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::String(0),
        ty: IrType::String,
    });
    block.instructions.push(Instruction::Call {
        dest: None,
        func: FuncId("اطبع".to_string()),
        args: vec![VarId(0)],
        ret_ty: IrType::Void,
    });
    block.instructions.push(Instruction::Return { value: None });

    func.blocks.push(block);
    module.functions.push(func);

    let (result, output) = run_module_with_output(module);
    assert!(result.is_ok());
    assert_eq!(output, vec!["Hello"]);
}

#[test]
fn test_recursive_factorial() {
    let mut module = create_empty_module();

    let mut factorial = Function::new(
        FuncId("factorial".to_string()),
        "factorial".to_string(),
        vec![Parameter {
            id: VarId(0),
            name: "n".to_string(),
            ty: IrType::Int,
        }],
        IrType::Int,
    );

    let mut entry = BasicBlock::new(BlockId(0));
    entry.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(1),
        ty: IrType::Int,
    });
    entry.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Le,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    entry.instructions.push(Instruction::Branch {
        cond: VarId(2),
        then_block: BlockId(1),
        else_block: BlockId(2),
    });

    let mut base = BasicBlock::new(BlockId(1));
    base.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    let mut recursive = BasicBlock::new(BlockId(2));
    recursive.instructions.push(Instruction::Binary {
        dest: VarId(3),
        op: BinaryOp::Sub,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });
    recursive.instructions.push(Instruction::Call {
        dest: Some(VarId(4)),
        func: FuncId("factorial".to_string()),
        args: vec![VarId(3)],
        ret_ty: IrType::Int,
    });
    recursive.instructions.push(Instruction::Binary {
        dest: VarId(5),
        op: BinaryOp::Mul,
        left: VarId(0),
        right: VarId(4),
        ty: IrType::Int,
    });
    recursive.instructions.push(Instruction::Return {
        value: Some(VarId(5)),
    });

    factorial.blocks.push(entry);
    factorial.blocks.push(base);
    factorial.blocks.push(recursive);
    module.functions.push(factorial);

    let mut main_func = create_main_function();
    let mut main_block = BasicBlock::new(BlockId(0));
    main_block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(5),
        ty: IrType::Int,
    });
    main_block.instructions.push(Instruction::Call {
        dest: Some(VarId(1)),
        func: FuncId("factorial".to_string()),
        args: vec![VarId(0)],
        ret_ty: IrType::Int,
    });
    main_block.instructions.push(Instruction::Return {
        value: Some(VarId(1)),
    });

    main_func.blocks.push(main_block);
    module.functions.push(main_func);

    assert_eq!(run_module(module).unwrap(), Value::Int(120));
}

#[test]
fn test_new_object() {
    let mut module = create_empty_module();

    let mut class = Class::new(ClassId("Point".to_string()), "Point".to_string());
    class.fields.push(("x".to_string(), IrType::Int));
    class.fields.push(("y".to_string(), IrType::Int));
    module.classes.push(class);

    let mut func = create_main_function();
    func.return_type = IrType::Void;
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::NewObject {
        dest: VarId(0),
        class: ClassId("Point".to_string()),
    });
    block.instructions.push(Instruction::Return { value: None });

    func.blocks.push(block);
    module.functions.push(func);

    let result = run_module(module);
    assert!(result.is_ok());
}

#[test]
fn test_throw_exception() {
    let mut module = create_empty_module();
    module.strings.add("Error message".to_string());

    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::String(0),
        ty: IrType::String,
    });
    block.instructions.push(Instruction::Throw {
        exception: VarId(0),
    });
    block.instructions.push(Instruction::Return { value: None });

    func.blocks.push(block);
    module.functions.push(func);

    let result = run_module(module);
    assert!(result.is_err());
}

#[test]
fn test_alloca_load_store() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Alloca {
        dest: VarId(0),
        ty: IrType::Int,
    });

    block.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(42),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Store {
        ptr: VarId(0),
        value: VarId(1),
    });

    block.instructions.push(Instruction::Load {
        dest: VarId(2),
        ptr: VarId(0),
        ty: IrType::Int,
    });
    block.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    assert_eq!(run_module(module).unwrap(), Value::Int(42));
}

#[test]
fn test_undefined_variable_error() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Return {
        value: Some(VarId(999)),
    });

    func.blocks.push(block);
    module.functions.push(func);

    let result = run_module(module);
    assert!(result.is_err());
}

#[test]
fn test_undefined_function_error() {
    let mut module = create_empty_module();
    let mut func = create_main_function();
    let mut block = BasicBlock::new(BlockId(0));

    block.instructions.push(Instruction::Call {
        dest: Some(VarId(0)),
        func: FuncId("undefined_function".to_string()),
        args: vec![],
        ret_ty: IrType::Void,
    });
    block.instructions.push(Instruction::Return { value: None });

    func.blocks.push(block);
    module.functions.push(func);

    let result = run_module(module);
    assert!(result.is_err());
}
