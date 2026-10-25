#![cfg(feature = "jit")]

use tarqeem::ir::opt::DeadCodeEliminator;
use tarqeem::ir::{Instruction, IrBuilder, Module};
use tarqeem::jit::baseline::BaselineCompiler;
use tarqeem::jit::optimizing::OptimizingCompiler;
use tarqeem::jit::JitError;
use tarqeem::parser::Parser;

fn mutation_module(statement: &str) -> Module {
    let source = format!(
        "بسم_الله\n\
         دالة عدل(م: مصفوفة<عدد>، قيمة: عدد) {{ {statement} }}\n\
         اطبع(٠)\n\
         الحمد_لله"
    );
    let ast = Parser::new(&source).parse().unwrap();
    let mut module = IrBuilder::new("تعديل".to_string()).build(&ast).unwrap();
    // Remove the unused void sentinel so only the mutation needs unsupported lowering.
    DeadCodeEliminator::new().run(&mut module);
    module
}

fn assert_unsupported(error: JitError, instruction: &Instruction) {
    assert_eq!(
        error.kind,
        JitError::unsupported_instruction(instruction.to_string()).kind
    );
    assert_eq!(
        error.message,
        format!("تعليمة غير مدعومة للترجمة الفورية: {instruction}")
    );
}

#[test]
fn test_baseline_rejects_array_push_instead_of_dropping_it() {
    for statement in ["ألحق(م، قيمة)", "م.ألحق(قيمة)"] {
        let module = mutation_module(statement);
        let function = module.functions.iter().find(|f| f.name == "عدل").unwrap();
        let instruction = &function.blocks[0].instructions[0];
        assert!(matches!(instruction, Instruction::ArrayPush { .. }));

        let mut compiler = BaselineCompiler::new().unwrap();
        let error = compiler.compile(&module, function).unwrap_err();
        assert_unsupported(error, instruction);
        assert_eq!(compiler.stats().functions_compiled, 0);
        assert!(compiler.get_function_ptr(&function.name).is_none());
    }
}

#[test]
fn test_optimizing_rejects_array_push_instead_of_dropping_it() {
    for statement in ["ألحق(م، قيمة)", "م.ألحق(قيمة)"] {
        let module = mutation_module(statement);
        let function = module.functions.iter().find(|f| f.name == "عدل").unwrap();
        let instruction = &function.blocks[0].instructions[0];
        assert!(matches!(instruction, Instruction::ArrayPush { .. }));

        let mut compiler = OptimizingCompiler::new().unwrap();
        let error = compiler.compile(&module, function).unwrap_err();
        assert_unsupported(error, instruction);
        assert_eq!(compiler.stats().functions_compiled, 0);
        assert!(compiler.get_function_ptr(&function.name).is_none());
    }
}

#[test]
fn test_baseline_still_rejects_array_pop() {
    let module = mutation_module("احذف_آخر(م)");
    let function = module.functions.iter().find(|f| f.name == "عدل").unwrap();
    let instruction = &function.blocks[0].instructions[0];
    assert!(matches!(instruction, Instruction::ArrayPop { .. }));

    let error = BaselineCompiler::new()
        .unwrap()
        .compile(&module, function)
        .unwrap_err();
    assert_unsupported(error, instruction);
}

#[test]
fn test_optimizing_still_rejects_array_pop() {
    let module = mutation_module("احذف_آخر(م)");
    let function = module.functions.iter().find(|f| f.name == "عدل").unwrap();
    let instruction = &function.blocks[0].instructions[0];
    assert!(matches!(instruction, Instruction::ArrayPop { .. }));

    let error = OptimizingCompiler::new()
        .unwrap()
        .compile(&module, function)
        .unwrap_err();
    assert_unsupported(error, instruction);
}
