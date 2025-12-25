//! Comprehensive tests for IR instructions and types

use super::instruction::*;

#[test]
fn test_var_id_creation() {
    let var = VarId(0);
    assert_eq!(var.0, 0);
}

#[test]
fn test_var_id_display() {
    let var = VarId(42);
    assert_eq!(format!("{}", var), "%42");
}

#[test]
fn test_var_id_display_zero() {
    let var = VarId(0);
    assert_eq!(format!("{}", var), "%0");
}

#[test]
fn test_var_id_equality() {
    let var1 = VarId(5);
    let var2 = VarId(5);
    let var3 = VarId(6);

    assert_eq!(var1, var2);
    assert_ne!(var1, var3);
}

#[test]
fn test_var_id_clone() {
    let var = VarId(10);
    let cloned = var;
    assert_eq!(var, cloned);
}

#[test]
fn test_var_id_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(VarId(1));
    set.insert(VarId(2));
    set.insert(VarId(1)); // Duplicate

    assert_eq!(set.len(), 2);
}

#[test]
fn test_block_id_creation() {
    let block = BlockId(0);
    assert_eq!(block.0, 0);
}

#[test]
fn test_block_id_display() {
    let block = BlockId(3);
    assert_eq!(format!("{}", block), "bb3");
}

#[test]
fn test_block_id_equality() {
    let block1 = BlockId(0);
    let block2 = BlockId(0);
    let block3 = BlockId(1);

    assert_eq!(block1, block2);
    assert_ne!(block1, block3);
}

#[test]
fn test_block_id_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(BlockId(0));
    set.insert(BlockId(1));
    set.insert(BlockId(0)); // Duplicate

    assert_eq!(set.len(), 2);
}

#[test]
fn test_func_id_creation() {
    let func = FuncId("main".to_string());
    assert_eq!(func.0, "main");
}

#[test]
fn test_func_id_display() {
    let func = FuncId("add".to_string());
    assert_eq!(format!("{}", func), "@add");
}

#[test]
fn test_func_id_arabic_name() {
    let func = FuncId("جمع".to_string());
    assert_eq!(format!("{}", func), "@جمع");
}

#[test]
fn test_func_id_equality() {
    let func1 = FuncId("foo".to_string());
    let func2 = FuncId("foo".to_string());
    let func3 = FuncId("bar".to_string());

    assert_eq!(func1, func2);
    assert_ne!(func1, func3);
}

#[test]
fn test_func_id_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(FuncId("a".to_string()));
    set.insert(FuncId("b".to_string()));
    set.insert(FuncId("a".to_string())); // Duplicate

    assert_eq!(set.len(), 2);
}

#[test]
fn test_class_id_creation() {
    let class = ClassId("Person".to_string());
    assert_eq!(class.0, "Person");
}

#[test]
fn test_class_id_display() {
    let class = ClassId("MyClass".to_string());
    assert_eq!(format!("{}", class), "%class.MyClass");
}

#[test]
fn test_class_id_arabic() {
    let class = ClassId("شخص".to_string());
    assert_eq!(format!("{}", class), "%class.شخص");
}

#[test]
fn test_class_id_equality() {
    let class1 = ClassId("A".to_string());
    let class2 = ClassId("A".to_string());
    let class3 = ClassId("B".to_string());

    assert_eq!(class1, class2);
    assert_ne!(class1, class3);
}

#[test]
fn test_field_id_creation() {
    let field = FieldId {
        class: ClassId("Person".to_string()),
        name: "age".to_string(),
        index: 0,
    };

    assert_eq!(field.name, "age");
    assert_eq!(field.index, 0);
}

#[test]
fn test_field_id_display() {
    let field = FieldId {
        class: ClassId("Person".to_string()),
        name: "name".to_string(),
        index: 1,
    };

    assert_eq!(format!("{}", field), "%class.Person.name");
}

#[test]
fn test_field_id_arabic() {
    let field = FieldId {
        class: ClassId("شخص".to_string()),
        name: "اسم".to_string(),
        index: 0,
    };

    assert_eq!(format!("{}", field), "%class.شخص.اسم");
}

#[test]
fn test_method_id_creation() {
    let method = MethodId {
        class: ClassId("Calculator".to_string()),
        name: "add".to_string(),
    };

    assert_eq!(method.name, "add");
}

#[test]
fn test_method_id_display() {
    let method = MethodId {
        class: ClassId("Math".to_string()),
        name: "sqrt".to_string(),
    };

    assert_eq!(format!("{}", method), "%class.Math::sqrt");
}

#[test]
fn test_method_id_arabic() {
    let method = MethodId {
        class: ClassId("حاسبة".to_string()),
        name: "اجمع".to_string(),
    };

    assert_eq!(format!("{}", method), "%class.حاسبة::اجمع");
}

#[test]
fn test_ir_type_void() {
    let ty = IrType::Void;
    assert_eq!(format!("{}", ty), "void");
}

#[test]
fn test_ir_type_bool() {
    let ty = IrType::Bool;
    assert_eq!(format!("{}", ty), "bool");
}

#[test]
fn test_ir_type_int() {
    let ty = IrType::Int;
    assert_eq!(format!("{}", ty), "i64");
}

#[test]
fn test_ir_type_float() {
    let ty = IrType::Float;
    assert_eq!(format!("{}", ty), "f64");
}

#[test]
fn test_ir_type_string() {
    let ty = IrType::String;
    assert_eq!(format!("{}", ty), "str");
}

#[test]
fn test_ir_type_ptr() {
    let ty = IrType::Ptr(Box::new(IrType::Int));
    assert_eq!(format!("{}", ty), "*i64");
}

#[test]
fn test_ir_type_ptr_nested() {
    let ty = IrType::Ptr(Box::new(IrType::Ptr(Box::new(IrType::String))));
    assert_eq!(format!("{}", ty), "**str");
}

#[test]
fn test_ir_type_array() {
    let ty = IrType::Array(Box::new(IrType::Int), 10);
    assert_eq!(format!("{}", ty), "[10 x i64]");
}

#[test]
fn test_ir_type_function() {
    let ty = IrType::Function {
        params: vec![IrType::Int, IrType::Float],
        ret: Box::new(IrType::Bool),
    };
    assert_eq!(format!("{}", ty), "fn(i64, f64) -> bool");
}

#[test]
fn test_ir_type_function_no_params() {
    let ty = IrType::Function {
        params: vec![],
        ret: Box::new(IrType::Void),
    };
    assert_eq!(format!("{}", ty), "fn() -> void");
}

#[test]
fn test_ir_type_struct() {
    let ty = IrType::Struct(ClassId("Point".to_string()));
    assert_eq!(format!("{}", ty), "%class.Point");
}

#[test]
fn test_ir_type_equality() {
    assert_eq!(IrType::Int, IrType::Int);
    assert_eq!(IrType::Float, IrType::Float);
    assert_ne!(IrType::Int, IrType::Float);

    let arr1 = IrType::Array(Box::new(IrType::Int), 5);
    let arr2 = IrType::Array(Box::new(IrType::Int), 5);
    let arr3 = IrType::Array(Box::new(IrType::Int), 10);

    assert_eq!(arr1, arr2);
    assert_ne!(arr1, arr3);
}

#[test]
fn test_ir_type_clone() {
    let ty = IrType::Function {
        params: vec![IrType::Int],
        ret: Box::new(IrType::Float),
    };
    let cloned = ty.clone();
    assert_eq!(ty, cloned);
}

#[test]
fn test_constant_null() {
    let c = Constant::Null;
    assert_eq!(format!("{}", c), "null");
}

#[test]
fn test_constant_bool_true() {
    let c = Constant::Bool(true);
    assert_eq!(format!("{}", c), "true");
}

#[test]
fn test_constant_bool_false() {
    let c = Constant::Bool(false);
    assert_eq!(format!("{}", c), "false");
}

#[test]
fn test_constant_int() {
    let c = Constant::Int(42);
    assert_eq!(format!("{}", c), "42");
}

#[test]
fn test_constant_int_negative() {
    let c = Constant::Int(-100);
    assert_eq!(format!("{}", c), "-100");
}

#[test]
fn test_constant_int_large() {
    let c = Constant::Int(i64::MAX);
    assert_eq!(format!("{}", c), i64::MAX.to_string());
}

#[test]
fn test_constant_float() {
    let c = Constant::Float(3.14);
    let display = format!("{}", c);
    assert!(display.starts_with("3.14"));
}

#[test]
fn test_constant_float_zero() {
    let c = Constant::Float(0.0);
    assert_eq!(format!("{}", c), "0.000000");
}

#[test]
fn test_constant_string() {
    let c = Constant::String(5);
    assert_eq!(format!("{}", c), "str#5");
}

#[test]
fn test_constant_equality() {
    assert_eq!(Constant::Int(5), Constant::Int(5));
    assert_ne!(Constant::Int(5), Constant::Int(6));
    assert_eq!(Constant::Bool(true), Constant::Bool(true));
    assert_ne!(Constant::Bool(true), Constant::Bool(false));
}

#[test]
fn test_constant_clone() {
    let c = Constant::Float(2.718);
    let cloned = c.clone();
    assert_eq!(c, cloned);
}

#[test]
fn test_binary_op_arithmetic_display() {
    assert_eq!(format!("{}", BinaryOp::Add), "add");
    assert_eq!(format!("{}", BinaryOp::Sub), "sub");
    assert_eq!(format!("{}", BinaryOp::Mul), "mul");
    assert_eq!(format!("{}", BinaryOp::Div), "div");
    assert_eq!(format!("{}", BinaryOp::Mod), "mod");
    assert_eq!(format!("{}", BinaryOp::Pow), "pow");
}

#[test]
fn test_binary_op_comparison_display() {
    assert_eq!(format!("{}", BinaryOp::Eq), "eq");
    assert_eq!(format!("{}", BinaryOp::Ne), "ne");
    assert_eq!(format!("{}", BinaryOp::Lt), "lt");
    assert_eq!(format!("{}", BinaryOp::Le), "le");
    assert_eq!(format!("{}", BinaryOp::Gt), "gt");
    assert_eq!(format!("{}", BinaryOp::Ge), "ge");
}

#[test]
fn test_binary_op_logical_display() {
    assert_eq!(format!("{}", BinaryOp::And), "and");
    assert_eq!(format!("{}", BinaryOp::Or), "or");
}

#[test]
fn test_binary_op_bitwise_display() {
    assert_eq!(format!("{}", BinaryOp::BitAnd), "bitand");
    assert_eq!(format!("{}", BinaryOp::BitOr), "bitor");
    assert_eq!(format!("{}", BinaryOp::BitXor), "bitxor");
    assert_eq!(format!("{}", BinaryOp::Shl), "shl");
    assert_eq!(format!("{}", BinaryOp::Shr), "shr");
}

#[test]
fn test_binary_op_equality() {
    assert_eq!(BinaryOp::Add, BinaryOp::Add);
    assert_ne!(BinaryOp::Add, BinaryOp::Sub);
}

#[test]
fn test_binary_op_copy() {
    let op = BinaryOp::Mul;
    let copied = op;
    assert_eq!(op, copied);
}

#[test]
fn test_binary_op_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(BinaryOp::Add);
    set.insert(BinaryOp::Sub);
    set.insert(BinaryOp::Add); // Duplicate

    assert_eq!(set.len(), 2);
}

#[test]
fn test_unary_op_display() {
    assert_eq!(format!("{}", UnaryOp::Neg), "neg");
    assert_eq!(format!("{}", UnaryOp::Not), "not");
    assert_eq!(format!("{}", UnaryOp::BitNot), "bitnot");
}

#[test]
fn test_unary_op_equality() {
    assert_eq!(UnaryOp::Neg, UnaryOp::Neg);
    assert_ne!(UnaryOp::Neg, UnaryOp::Not);
}

#[test]
fn test_unary_op_copy() {
    let op = UnaryOp::BitNot;
    let copied = op;
    assert_eq!(op, copied);
}

#[test]
fn test_unary_op_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(UnaryOp::Neg);
    set.insert(UnaryOp::Not);
    set.insert(UnaryOp::Neg); // Duplicate

    assert_eq!(set.len(), 2);
}

#[test]
fn test_instruction_const() {
    let inst = Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(42),
        ty: IrType::Int,
    };

    match inst {
        Instruction::Const { dest, value, ty } => {
            assert_eq!(dest, VarId(0));
            assert_eq!(value, Constant::Int(42));
            assert_eq!(ty, IrType::Int);
        }
        _ => panic!("Expected Const instruction"),
    }
}

#[test]
fn test_instruction_binary() {
    let inst = Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Add,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    };

    match inst {
        Instruction::Binary {
            dest,
            op,
            left,
            right,
            ty,
        } => {
            assert_eq!(dest, VarId(2));
            assert_eq!(op, BinaryOp::Add);
            assert_eq!(left, VarId(0));
            assert_eq!(right, VarId(1));
            assert_eq!(ty, IrType::Int);
        }
        _ => panic!("Expected Binary instruction"),
    }
}

#[test]
fn test_instruction_unary() {
    let inst = Instruction::Unary {
        dest: VarId(1),
        op: UnaryOp::Neg,
        operand: VarId(0),
        ty: IrType::Int,
    };

    match inst {
        Instruction::Unary {
            dest,
            op,
            operand,
            ty,
        } => {
            assert_eq!(dest, VarId(1));
            assert_eq!(op, UnaryOp::Neg);
            assert_eq!(operand, VarId(0));
            assert_eq!(ty, IrType::Int);
        }
        _ => panic!("Expected Unary instruction"),
    }
}

#[test]
fn test_instruction_int_to_float() {
    let inst = Instruction::IntToFloat {
        dest: VarId(1),
        src: VarId(0),
    };

    match inst {
        Instruction::IntToFloat { dest, src } => {
            assert_eq!(dest, VarId(1));
            assert_eq!(src, VarId(0));
        }
        _ => panic!("Expected IntToFloat instruction"),
    }
}

#[test]
fn test_instruction_float_to_int() {
    let inst = Instruction::FloatToInt {
        dest: VarId(1),
        src: VarId(0),
    };

    match inst {
        Instruction::FloatToInt { dest, src } => {
            assert_eq!(dest, VarId(1));
            assert_eq!(src, VarId(0));
        }
        _ => panic!("Expected FloatToInt instruction"),
    }
}

#[test]
fn test_instruction_to_string() {
    let inst = Instruction::ToString {
        dest: VarId(1),
        src: VarId(0),
    };

    match inst {
        Instruction::ToString { dest, src } => {
            assert_eq!(dest, VarId(1));
            assert_eq!(src, VarId(0));
        }
        _ => panic!("Expected ToString instruction"),
    }
}

#[test]
fn test_instruction_alloca() {
    let inst = Instruction::Alloca {
        dest: VarId(0),
        ty: IrType::Int,
    };

    match inst {
        Instruction::Alloca { dest, ty } => {
            assert_eq!(dest, VarId(0));
            assert_eq!(ty, IrType::Int);
        }
        _ => panic!("Expected Alloca instruction"),
    }
}

#[test]
fn test_instruction_load() {
    let inst = Instruction::Load {
        dest: VarId(1),
        ptr: VarId(0),
        ty: IrType::Int,
    };

    match inst {
        Instruction::Load { dest, ptr, ty } => {
            assert_eq!(dest, VarId(1));
            assert_eq!(ptr, VarId(0));
            assert_eq!(ty, IrType::Int);
        }
        _ => panic!("Expected Load instruction"),
    }
}

#[test]
fn test_instruction_store() {
    let inst = Instruction::Store {
        ptr: VarId(0),
        value: VarId(1),
    };

    match inst {
        Instruction::Store { ptr, value } => {
            assert_eq!(ptr, VarId(0));
            assert_eq!(value, VarId(1));
        }
        _ => panic!("Expected Store instruction"),
    }
}

#[test]
fn test_instruction_jump() {
    let inst = Instruction::Jump { target: BlockId(1) };

    match inst {
        Instruction::Jump { target } => {
            assert_eq!(target, BlockId(1));
        }
        _ => panic!("Expected Jump instruction"),
    }
}

#[test]
fn test_instruction_branch() {
    let inst = Instruction::Branch {
        cond: VarId(0),
        then_block: BlockId(1),
        else_block: BlockId(2),
    };

    match inst {
        Instruction::Branch {
            cond,
            then_block,
            else_block,
        } => {
            assert_eq!(cond, VarId(0));
            assert_eq!(then_block, BlockId(1));
            assert_eq!(else_block, BlockId(2));
        }
        _ => panic!("Expected Branch instruction"),
    }
}

#[test]
fn test_instruction_return_value() {
    let inst = Instruction::Return {
        value: Some(VarId(0)),
    };

    match inst {
        Instruction::Return { value } => {
            assert_eq!(value, Some(VarId(0)));
        }
        _ => panic!("Expected Return instruction"),
    }
}

#[test]
fn test_instruction_return_void() {
    let inst = Instruction::Return { value: None };

    match inst {
        Instruction::Return { value } => {
            assert!(value.is_none());
        }
        _ => panic!("Expected Return instruction"),
    }
}

#[test]
fn test_instruction_call() {
    let inst = Instruction::Call {
        dest: Some(VarId(2)),
        func: FuncId("add".to_string()),
        args: vec![VarId(0), VarId(1)],
        ret_ty: IrType::Int,
    };

    match inst {
        Instruction::Call {
            dest,
            func,
            args,
            ret_ty,
        } => {
            assert_eq!(dest, Some(VarId(2)));
            assert_eq!(func, FuncId("add".to_string()));
            assert_eq!(args.len(), 2);
            assert_eq!(ret_ty, IrType::Int);
        }
        _ => panic!("Expected Call instruction"),
    }
}

#[test]
fn test_instruction_call_void() {
    let inst = Instruction::Call {
        dest: None,
        func: FuncId("print".to_string()),
        args: vec![VarId(0)],
        ret_ty: IrType::Void,
    };

    match inst {
        Instruction::Call {
            dest,
            func,
            args,
            ret_ty,
        } => {
            assert!(dest.is_none());
            assert_eq!(func, FuncId("print".to_string()));
            assert_eq!(args.len(), 1);
            assert_eq!(ret_ty, IrType::Void);
        }
        _ => panic!("Expected Call instruction"),
    }
}

#[test]
fn test_instruction_new_object() {
    let inst = Instruction::NewObject {
        dest: VarId(0),
        class: ClassId("Person".to_string()),
    };

    match inst {
        Instruction::NewObject { dest, class } => {
            assert_eq!(dest, VarId(0));
            assert_eq!(class, ClassId("Person".to_string()));
        }
        _ => panic!("Expected NewObject instruction"),
    }
}

#[test]
fn test_instruction_equality() {
    let inst1 = Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(5),
        ty: IrType::Int,
    };
    let inst2 = Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(5),
        ty: IrType::Int,
    };
    let inst3 = Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(10),
        ty: IrType::Int,
    };

    assert_eq!(inst1, inst2);
    assert_ne!(inst1, inst3);
}

#[test]
fn test_instruction_clone() {
    let inst = Instruction::Binary {
        dest: VarId(2),
        op: BinaryOp::Mul,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Float,
    };

    let cloned = inst.clone();
    assert_eq!(inst, cloned);
}

#[test]
fn test_instruction_debug() {
    let inst = Instruction::Jump { target: BlockId(5) };
    let debug = format!("{:?}", inst);
    assert!(debug.contains("Jump"));
    assert!(debug.contains("5"));
}
