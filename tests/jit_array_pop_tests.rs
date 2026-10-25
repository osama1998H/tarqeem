#![cfg(feature = "jit")]

use tarqeem::ir::{Function, Instruction, IrBuilder, Module};
use tarqeem::jit::baseline::BaselineCompiler;
use tarqeem::jit::optimizing::OptimizingCompiler;
use tarqeem::jit::JitError;
use tarqeem::parser::Parser;

fn pop_module(expression: &str, return_result: bool) -> Module {
    let body = if return_result {
        format!("أرجع {expression}")
    } else {
        format!("{expression}\nأرجع ٠")
    };
    let source = format!(
        "بسم_الله\n\
         دالة أزل(م: مصفوفة<عدد>) -> عدد {{ {body} }}\n\
         اطبع(٠)\n\
         الحمد_لله"
    );
    let ast = Parser::new(&source).parse().unwrap();
    // Retain discarded mutations so compilation cannot silently drop their effects.
    IrBuilder::new("حذف_عنصر".to_string()).build(&ast).unwrap()
}

fn pop_function(module: &Module) -> &Function {
    module.functions.iter().find(|f| f.name == "أزل").unwrap()
}

fn assert_unsupported(error: JitError, instruction: &Instruction) {
    assert!(matches!(instruction, Instruction::ArrayPop { .. }));
    assert_eq!(
        error.kind,
        JitError::unsupported_instruction(instruction.to_string()).kind
    );
    assert_eq!(
        error.message,
        format!("تعليمة غير مدعومة للترجمة الفورية: {instruction}")
    );
}

fn assert_baseline_rejects(return_result: bool) {
    for expression in ["احذف_آخر(م)", "م.احذف_آخر()"] {
        let module = pop_module(expression, return_result);
        let function = pop_function(&module);
        let instruction = &function.blocks[0].instructions[0];
        let mut compiler = BaselineCompiler::new().unwrap();
        let error = compiler.compile(&module, function).unwrap_err();
        assert_unsupported(error, instruction);
        assert_eq!(compiler.stats().functions_compiled, 0);
        assert!(compiler.get_function_ptr(&function.name).is_none());
    }
}

fn assert_optimizing_rejects(return_result: bool) {
    for expression in ["احذف_آخر(م)", "م.احذف_آخر()"] {
        let module = pop_module(expression, return_result);
        let function = pop_function(&module);
        let instruction = &function.blocks[0].instructions[0];
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
fn test_baseline_rejects_array_pop_before_its_result_is_read() {
    assert_baseline_rejects(true);
}

#[test]
fn test_baseline_rejects_discarded_array_pop() {
    assert_baseline_rejects(false);
}

#[test]
fn test_optimizing_rejects_array_pop_before_its_result_is_read() {
    assert_optimizing_rejects(true);
}

#[test]
fn test_optimizing_rejects_discarded_array_pop() {
    assert_optimizing_rejects(false);
}
