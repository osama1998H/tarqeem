#![cfg(feature = "jit")]

use tarqeem::ir::{Function, Instruction, IrBuilder, IrType, Module};
use tarqeem::jit::baseline::BaselineCompiler;
use tarqeem::jit::optimizing::OptimizingCompiler;
use tarqeem::jit::JitError;
use tarqeem::parser::Parser;

fn push_module(statement: &str, value_type: &str) -> Module {
    let source = format!(
        "بسم_الله\n\
         دالة عدل(م: مصفوفة<عدد_عشري>، قيمة: {value_type}) {{ {statement} }}\n\
         اطبع(٠)\n\
         الحمد_لله"
    );
    let ast = Parser::new(&source).parse().unwrap();
    IrBuilder::new("إلحاق_كسور".to_string())
        .build(&ast)
        .unwrap()
}

fn push_function(module: &Module) -> &Function {
    module.functions.iter().find(|f| f.name == "عدل").unwrap()
}

fn push_instruction(function: &Function) -> &Instruction {
    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find(|instruction| matches!(instruction, Instruction::ArrayPush { .. }))
        .expect("يجب الاحتفاظ بتعليمة الإلحاق")
}

fn assert_unsupported(error: JitError, instruction: &Instruction) {
    assert!(matches!(
        instruction,
        Instruction::ArrayPush {
            elem_ty: IrType::Float,
            ..
        }
    ));
    assert_eq!(
        error.kind,
        JitError::unsupported_instruction(instruction.to_string()).kind
    );
    assert_eq!(
        error.message,
        format!("تعليمة غير مدعومة للترجمة الفورية: {instruction}")
    );
}

fn assert_baseline_rejects(value_type: &str) {
    for statement in ["م.ألحق(قيمة)", "م.أضف(قيمة)", "ألحق(م، قيمة)"] {
        let module = push_module(statement, value_type);
        let function = push_function(&module);
        let instruction = push_instruction(function);
        let mut compiler = BaselineCompiler::new().unwrap();
        let error = compiler.compile(&module, function).unwrap_err();
        assert_unsupported(error, instruction);
        assert_eq!(compiler.stats().functions_compiled, 0);
        assert!(compiler.get_function_ptr(&function.name).is_none());
    }
}

fn assert_optimizing_rejects(value_type: &str) {
    for statement in ["م.ألحق(قيمة)", "م.أضف(قيمة)", "ألحق(م، قيمة)"] {
        let module = push_module(statement, value_type);
        let function = push_function(&module);
        let instruction = push_instruction(function);
        let mut compiler = OptimizingCompiler::new().unwrap();
        let error = compiler
            .compile_with_profile(&module, function, None)
            .unwrap_err();
        assert_unsupported(error, instruction);
        assert_eq!(compiler.stats().functions_compiled, 0);
        assert!(compiler.get_function_ptr(&function.name).is_none());
    }
}

#[test]
fn test_baseline_rejects_float_array_push_from_integer_parameter() {
    assert_baseline_rejects("عدد");
}

#[test]
fn test_baseline_rejects_float_array_push_from_float_parameter() {
    assert_baseline_rejects("عدد_عشري");
}

#[test]
fn test_optimizing_rejects_float_array_push_from_integer_parameter() {
    assert_optimizing_rejects("عدد");
}

#[test]
fn test_optimizing_rejects_float_array_push_from_float_parameter() {
    assert_optimizing_rejects("عدد_عشري");
}
