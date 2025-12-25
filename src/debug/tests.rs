//! Tests for the debug module

#![allow(clippy::approx_constant)]

use super::*;
use crate::ir::{
    BasicBlock, BlockId, Constant, FuncId, Function, Instruction, IrType, Module, VarId,
};
use std::path::PathBuf;

fn create_test_module() -> Module {
    let mut module = Module::new("test".to_string());

    let hello_idx = module.strings.add("Hello, Debug!".to_string());

    let mut main_func = Function::new(
        FuncId("main".to_string()),
        "main".to_string(),
        vec![],
        IrType::Int,
    );

    let mut entry = BasicBlock::new(BlockId(0));

    entry.instructions.push(Instruction::Const {
        dest: VarId(0),
        value: Constant::Int(10),
        ty: IrType::Int,
    });

    entry.instructions.push(Instruction::Const {
        dest: VarId(1),
        value: Constant::Int(20),
        ty: IrType::Int,
    });

    entry.instructions.push(Instruction::Binary {
        dest: VarId(2),
        op: crate::ir::BinaryOp::Add,
        left: VarId(0),
        right: VarId(1),
        ty: IrType::Int,
    });

    entry.instructions.push(Instruction::Const {
        dest: VarId(3),
        value: Constant::String(hello_idx),
        ty: IrType::String,
    });
    entry
        .instructions
        .push(Instruction::Print { value: VarId(3) });

    entry.instructions.push(Instruction::Return {
        value: Some(VarId(2)),
    });

    main_func.blocks.push(entry);
    module.functions.push(main_func);

    module
}

fn create_branching_module() -> Module {
    let mut module = Module::new("test".to_string());

    let mut main_func = Function::new(
        FuncId("main".to_string()),
        "main".to_string(),
        vec![],
        IrType::Int,
    );

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

    main_func.blocks.push(entry);
    main_func.blocks.push(then_block);
    main_func.blocks.push(else_block);
    module.functions.push(main_func);

    module
}

#[test]
fn test_debug_interpreter_creation() {
    let module = create_test_module();
    let context = DebugContext::new();
    let interpreter = DebugInterpreter::new(module, context);

    assert!(interpreter.context().state() == &DebugState::NotStarted);
}

#[test]
fn test_debug_interpreter_start() {
    let module = create_test_module();
    let context = DebugContext::new();
    let mut interpreter = DebugInterpreter::new(module, context);

    interpreter.start().unwrap();

    assert!(interpreter.context().state() == &DebugState::Running);
}

#[test]
fn test_debug_interpreter_run() {
    let module = create_test_module();
    let context = DebugContext::new();
    let mut interpreter = DebugInterpreter::new(module, context);

    interpreter.start().unwrap();
    let result = interpreter.run();

    assert!(result.is_ok());
    match result.unwrap() {
        StepResult::Completed(value) => {
            assert_eq!(value, crate::interpreter::Value::Int(30));
        }
        other => panic!("Expected Completed, got {:?}", other),
    }
}

#[test]
fn test_debug_interpreter_with_stop_on_entry() {
    let module = create_test_module();
    let mut context = DebugContext::new();
    context.config_mut().stop_on_entry = true;

    let mut interpreter = DebugInterpreter::new(module, context);
    interpreter.start().unwrap();

    assert!(
        interpreter.context().state()
            == &DebugState::Paused {
                reason: PauseReason::Entry
            }
    );
}

#[test]
fn test_debug_context_breakpoints() {
    let mut context = DebugContext::new();
    let file = PathBuf::from("test.trq");

    let id = context.add_breakpoint(file.clone(), 10).unwrap();

    assert!(context.has_breakpoint_at(&file, 10));
    assert!(context.get_breakpoint(id).is_some());

    context.remove_breakpoint(id).unwrap();
    assert!(!context.has_breakpoint_at(&file, 10));
}

#[test]
fn test_debug_context_conditional_breakpoint() {
    let mut context = DebugContext::new();
    let file = PathBuf::from("test.trq");

    let id = context
        .add_conditional_breakpoint(file.clone(), 10, "x > 5".to_string())
        .unwrap();

    let bp = context.get_breakpoint(id).unwrap();
    assert_eq!(bp.condition, Some("x > 5".to_string()));
}

#[test]
fn test_debug_context_watch_expressions() {
    let mut context = DebugContext::new();

    let id = context.add_watch("x + y".to_string());
    assert!(context.watches().any(|w| w.id == id));

    context.remove_watch(id);
    assert!(!context.watches().any(|w| w.id == id));
}

#[test]
fn test_source_map() {
    let mut map = SourceMap::new();
    let file = PathBuf::from("test.trq");
    let func_id = FuncId("main".to_string());

    map.add_source(
        file.clone(),
        "دالة رئيسية() {\n  اطبع(\"مرحبا\")\n}".to_string(),
    );

    let span = crate::error::Span::new(0, 10, 2, 3);
    let loc = SourceLocation::new(file.clone(), span);
    map.add_instruction(&func_id, BlockId(0), 0, loc);

    assert!(!map.is_empty());
    let retrieved = map.get_instruction_location(&func_id, BlockId(0), 0);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().line, 2);
}

#[test]
fn test_debug_command_parsing() {
    use commands::DebugCommandParser;

    assert_eq!(DebugCommandParser::parse("c"), DebugCommand::Continue);
    assert_eq!(
        DebugCommandParser::parse("continue"),
        DebugCommand::Continue
    );
    assert_eq!(DebugCommandParser::parse("تابع"), DebugCommand::Continue);

    assert_eq!(DebugCommandParser::parse("n"), DebugCommand::StepOver);
    assert_eq!(DebugCommandParser::parse("s"), DebugCommand::StepInto);
    assert_eq!(DebugCommandParser::parse("o"), DebugCommand::StepOut);

    assert_eq!(
        DebugCommandParser::parse("b 10"),
        DebugCommand::Break {
            file: None,
            line: 10
        }
    );
    assert_eq!(
        DebugCommandParser::parse("break test.trq:20"),
        DebugCommand::Break {
            file: Some(PathBuf::from("test.trq")),
            line: 20
        }
    );

    assert_eq!(
        DebugCommandParser::parse("p x"),
        DebugCommand::Print {
            expression: "x".to_string()
        }
    );

    assert_eq!(DebugCommandParser::parse("h"), DebugCommand::Help);
    assert_eq!(DebugCommandParser::parse("q"), DebugCommand::Quit);
}

#[test]
fn test_step_mode() {
    let mut context = DebugContext::new();

    context.start_stepping(StepMode::Over, 1, Some(10), Some("main"));
    assert_eq!(context.step_mode(), Some(StepMode::Over));

    assert!(!context.is_step_complete(1, Some(10), Some("main")));

    assert!(context.is_step_complete(1, Some(11), Some("main")));

    assert!(!context.is_step_complete(2, Some(5), Some("other")));

    context.stop_stepping();
    assert_eq!(context.step_mode(), None);
}

#[test]
fn test_step_into() {
    let mut context = DebugContext::new();

    context.start_stepping(StepMode::Into, 1, Some(10), Some("main"));

    assert!(!context.is_step_complete(1, Some(10), Some("main")));

    assert!(context.is_step_complete(1, Some(11), Some("main")));

    assert!(context.is_step_complete(2, Some(10), Some("other")));
}

#[test]
fn test_step_out() {
    let mut context = DebugContext::new();

    context.start_stepping(StepMode::Out, 2, Some(10), Some("inner"));

    assert!(!context.is_step_complete(2, Some(15), Some("inner")));

    assert!(!context.is_step_complete(3, Some(5), Some("deeper")));

    assert!(context.is_step_complete(1, Some(20), Some("outer")));
}

#[test]
fn test_debug_state_transitions() {
    let state = DebugState::NotStarted;
    assert!(!state.is_paused());
    assert!(!state.is_running());
    assert!(!state.is_terminated());

    let state = DebugState::Running;
    assert!(!state.is_paused());
    assert!(state.is_running());

    let state = DebugState::Paused {
        reason: PauseReason::Step,
    };
    assert!(state.is_paused());
    assert!(!state.is_running());

    let state = DebugState::Stepping {
        mode: StepMode::Over,
    };
    assert!(!state.is_paused());
    assert!(state.is_running());

    let state = DebugState::Terminated { exit_value: None };
    assert!(state.is_terminated());
}

#[test]
fn test_breakpoint_hit_count() {
    let mut bp = Breakpoint::new(BreakpointId(1), PathBuf::from("test.trq"), 10);
    bp.hit_count = Some(3);

    assert!(!bp.should_trigger());
    assert!(!bp.should_trigger());

    assert!(bp.should_trigger());

    assert!(bp.should_trigger());

    bp.reset_hits();
    assert!(!bp.should_trigger());
}

#[test]
fn test_breakpoint_disabled() {
    let mut bp = Breakpoint::new(BreakpointId(1), PathBuf::from("test.trq"), 10);

    assert!(bp.should_trigger());

    bp.enabled = false;
    bp.reset_hits();

    assert!(!bp.should_trigger());
}

#[test]
fn test_debug_variable() {
    use crate::interpreter::Value;

    let var = DebugVariable::new("x".to_string(), Value::Int(42), true);
    assert_eq!(var.name, "x");
    assert_eq!(var.value, "42");
    assert_eq!(var.type_name, "int");
    assert!(var.is_mutable);
    assert_eq!(var.children_count, 0);

    let arr = Value::array_from(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    let var = DebugVariable::new("arr".to_string(), arr, true);
    assert_eq!(var.children_count, 3);
}

#[test]
fn test_stack_frame() {
    let frame = StackFrame::new(
        0,
        "main".to_string(),
        FuncId("main".to_string()),
        BlockId(0),
        0,
    );

    assert_eq!(frame.id, 0);
    assert_eq!(frame.name, "main");
    assert!(frame.location.is_none());

    let loc = SourceLocation::from_position(PathBuf::from("test.trq"), 10, 1);
    let frame = frame.with_location(loc);
    assert!(frame.location.is_some());
    assert_eq!(frame.location.as_ref().unwrap().line, 10);
}

#[test]
fn test_interpreter_get_locals() {
    let module = create_test_module();
    let context = DebugContext::new();
    let mut interpreter = DebugInterpreter::new(module, context);

    interpreter.start().unwrap();
    interpreter.run().unwrap();

    let locals = interpreter.get_locals();
    assert!(locals.is_empty());
}

#[test]
fn test_interpreter_branching() {
    let module = create_branching_module();
    let context = DebugContext::new();
    let mut interpreter = DebugInterpreter::new(module, context);

    interpreter.start().unwrap();
    let result = interpreter.run();

    assert!(result.is_ok());
    match result.unwrap() {
        StepResult::Completed(value) => {
            assert_eq!(value, crate::interpreter::Value::Int(1)); // true branch
        }
        other => panic!("Expected Completed, got {:?}", other),
    }
}

#[test]
fn test_pause_reason_description() {
    let reason = PauseReason::Breakpoint {
        id: BreakpointId(1),
    };
    assert!(reason.description().contains("1"));

    let reason = PauseReason::Step;
    assert!(reason.description().contains("Step"));

    let reason = PauseReason::Exception {
        message: "Error".to_string(),
    };
    assert!(reason.description().contains("Error"));
}

#[test]
fn test_output_capture() {
    let module = create_test_module();
    let mut context = DebugContext::new();
    context.config_mut().capture_output = true;

    let mut interpreter = DebugInterpreter::new(module, context);
    interpreter.start().unwrap();
    interpreter.run().unwrap();

    let output = interpreter.context().output();
    assert!(!output.is_empty());
    assert!(output[0].contains("Hello, Debug!"));
}

#[test]
fn test_pause_request() {
    let mut context = DebugContext::new();

    assert!(!context.is_pause_requested());

    context.request_pause();
    assert!(context.is_pause_requested());

    assert!(context.check_and_clear_pause());
    assert!(!context.is_pause_requested());

    assert!(!context.check_and_clear_pause());
}

#[test]
fn test_exception_breakpoints_config() {
    let mut context = DebugContext::new();

    assert!(!context.config().break_on_all_exceptions);
    assert!(context.config().break_on_uncaught_exceptions);

    assert!(!context.should_break_on_exception(true));
    assert!(context.should_break_on_exception(false));

    context.set_exception_breakpoints(true, false);
    assert!(context.should_break_on_exception(true));
    assert!(context.should_break_on_exception(false));

    context.set_exception_breakpoints(false, false);
    assert!(!context.should_break_on_exception(true));
    assert!(!context.should_break_on_exception(false));
}

#[test]
fn test_parse_value_string() {
    let module = create_test_module();
    let context = DebugContext::new();
    let interpreter = DebugInterpreter::new(module, context);

    let value = interpreter.parse_value_string("42").unwrap();
    assert_eq!(value, crate::interpreter::Value::Int(42));

    let value = interpreter.parse_value_string("3.14").unwrap();
    if let crate::interpreter::Value::Float(f) = value {
        assert!((f - 3.14).abs() < 0.001);
    } else {
        panic!("Expected Float");
    }

    let value = interpreter.parse_value_string("true").unwrap();
    assert_eq!(value, crate::interpreter::Value::Bool(true));

    let value = interpreter.parse_value_string("صحيح").unwrap();
    assert_eq!(value, crate::interpreter::Value::Bool(true));

    let value = interpreter.parse_value_string("false").unwrap();
    assert_eq!(value, crate::interpreter::Value::Bool(false));

    let value = interpreter.parse_value_string("null").unwrap();
    assert_eq!(value, crate::interpreter::Value::Null);

    let value = interpreter.parse_value_string("لا_شيء").unwrap();
    assert_eq!(value, crate::interpreter::Value::Null);

    let value = interpreter.parse_value_string("\"hello\"").unwrap();
    assert_eq!(
        value,
        crate::interpreter::Value::String("hello".to_string().into())
    );

    let value = interpreter.parse_value_string("hello").unwrap();
    assert_eq!(
        value,
        crate::interpreter::Value::String("hello".to_string().into())
    );
}

#[test]
fn test_source_map_find_variable_by_name() {
    use super::source_map::{SourceMap, VariableInfo};
    use crate::ir::{FuncId, VarId};

    let mut source_map = SourceMap::new();

    let func_id = FuncId("main".to_string());
    let var_id = VarId(1);

    let mut var_info = VariableInfo::new("متغير".to_string(), "عدد".to_string(), true);
    var_info.name_ar = Some("متغير".to_string());
    source_map.add_variable(&func_id, var_id, var_info);

    let found = source_map.find_variable_by_name(&func_id, "متغير");
    assert_eq!(found, Some(VarId(1)));

    let not_found = source_map.find_variable_by_name(&func_id, "غير_موجود");
    assert_eq!(not_found, None);

    let wrong_func = FuncId("other".to_string());
    let not_found = source_map.find_variable_by_name(&wrong_func, "متغير");
    assert_eq!(not_found, None);
}

#[test]
fn test_user_request_pause_reason() {
    let reason = PauseReason::UserRequest;
    assert!(reason.description().contains("user") || reason.description().contains("User"));
    assert!(reason.description_ar().contains("المستخدم"));
}
