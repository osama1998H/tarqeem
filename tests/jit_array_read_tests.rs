#![cfg(feature = "jit")]

use tarqeem::ir::{Function, Instruction, IrBuilder, Module};
use tarqeem::jit::baseline::BaselineCompiler;
use tarqeem::jit::optimizing::OptimizingCompiler;
use tarqeem::jit::JitError;
use tarqeem::parser::Parser;

fn read_module(expression: &str, return_result: bool) -> Module {
    let body = if return_result {
        format!("أرجع {expression}")
    } else {
        format!("{expression}\nأرجع ٠")
    };
    let source = format!(
        "بسم_الله\n\
         دالة اقرأ(م: مصفوفة<عدد>، فهرس: عدد) -> عدد {{ {body} }}\n\
         اطبع(٠)\n\
         الحمد_لله"
    );
    let ast = Parser::new(&source).parse().unwrap();
    // Keep discarded reads so silent skipping cannot masquerade as compilation support.
    IrBuilder::new("قراءة".to_string()).build(&ast).unwrap()
}

fn read_function(module: &Module) -> &Function {
    module.functions.iter().find(|f| f.name == "اقرأ").unwrap()
}

fn array_get_module(return_result: bool) -> Module {
    let module = read_module("م[فهرس]", return_result);
    assert!(matches!(
        read_function(&module).blocks[0].instructions[0],
        Instruction::ArrayGet { .. }
    ));
    module
}

fn array_len_module(expression: &str, return_result: bool) -> Module {
    let module = read_module(expression, return_result);
    assert!(matches!(
        read_function(&module).blocks[0].instructions[0],
        Instruction::ArrayLen { .. }
    ));
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

fn assert_baseline_rejects(module: Module) {
    let function = read_function(&module);
    let instruction = &function.blocks[0].instructions[0];
    let mut compiler = BaselineCompiler::new().unwrap();
    let error = compiler.compile(&module, function).unwrap_err();
    assert_unsupported(error, instruction);
    assert_eq!(compiler.stats().functions_compiled, 0);
    assert!(compiler.get_function_ptr(&function.name).is_none());
}

fn assert_optimizing_rejects(module: Module) {
    let function = read_function(&module);
    let instruction = &function.blocks[0].instructions[0];
    let mut compiler = OptimizingCompiler::new().unwrap();
    let error = compiler
        .compile_with_profile(&module, function, None)
        .unwrap_err();
    assert_unsupported(error, instruction);
    assert_eq!(compiler.stats().functions_compiled, 0);
    assert!(compiler.get_function_ptr(&function.name).is_none());
}

#[test]
fn test_baseline_rejects_array_get_before_its_result_is_read() {
    assert_baseline_rejects(array_get_module(true));
}

#[test]
fn test_baseline_rejects_discarded_array_get() {
    assert_baseline_rejects(array_get_module(false));
}

#[test]
fn test_optimizing_rejects_array_get_before_its_result_is_read() {
    assert_optimizing_rejects(array_get_module(true));
}

#[test]
fn test_optimizing_rejects_discarded_array_get() {
    assert_optimizing_rejects(array_get_module(false));
}

#[test]
fn test_baseline_rejects_array_len_before_its_result_is_read() {
    for expression in ["طول(م)", "م.طول()"] {
        assert_baseline_rejects(array_len_module(expression, true));
    }
}

#[test]
fn test_baseline_rejects_discarded_array_len() {
    for expression in ["طول(م)", "م.طول()"] {
        assert_baseline_rejects(array_len_module(expression, false));
    }
}

#[test]
fn test_optimizing_rejects_array_len_before_its_result_is_read() {
    for expression in ["طول(م)", "م.طول()"] {
        assert_optimizing_rejects(array_len_module(expression, true));
    }
}

#[test]
fn test_optimizing_rejects_discarded_array_len() {
    for expression in ["طول(م)", "م.طول()"] {
        assert_optimizing_rejects(array_len_module(expression, false));
    }
}
