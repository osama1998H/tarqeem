//! Debug-aware interpreter
//!
//! This module extends the standard interpreter with debugging capabilities
//! including breakpoints, stepping, and variable inspection.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::interpreter::{RuntimeError, RuntimeResult, Value};
use crate::ir::{
    BasicBlock, BinaryOp, BlockId, Constant, FuncId, Function, Instruction, IrType, Module,
    UnaryOp, VarId,
};

use super::context::{BreakpointId, DebugContext};
use super::source_map::SourceLocation;
use super::state::{
    DebugEvent, DebugScope, DebugState, DebugVariable, PauseReason, StackFrame, StepMode,
};
use super::{DebugError, DebugResult};

/// Maximum call stack depth to prevent stack overflow
const MAX_STACK_DEPTH: usize = 1000;

/// A call frame for debug execution
#[derive(Debug)]
struct DebugCallFrame {
    /// Function being executed
    func_id: FuncId,
    /// Current block index
    block_idx: usize,
    /// Current instruction index within the block
    inst_idx: usize,
    /// Local variables (VarId -> Value)
    locals: HashMap<u32, Value>,
    /// Previous block (for Phi resolution)
    prev_block: Option<BlockId>,
    /// Try block stack for exception handling
    try_stack: Vec<BlockId>,
}

impl DebugCallFrame {
    fn new(func_id: FuncId) -> Self {
        Self {
            func_id,
            block_idx: 0,
            inst_idx: 0,
            locals: HashMap::new(),
            prev_block: None,
            try_stack: Vec::new(),
        }
    }
}

/// Result of executing a single step
#[derive(Debug)]
pub enum StepResult {
    /// Continue to next instruction
    Continue,
    /// Execution paused
    Paused(PauseReason),
    /// Function returned a value
    Returned(Value),
    /// Exception was thrown
    Exception(Value),
    /// Execution completed
    Completed(Value),
}

/// Debug-aware interpreter for Tarqeem programs
pub struct DebugInterpreter {
    /// The IR module to execute
    module: Module,
    /// Call stack
    call_stack: Vec<DebugCallFrame>,
    /// Global variables
    globals: HashMap<String, Value>,
    /// Current exception (if any)
    current_exception: Option<Value>,
    /// Debug context (breakpoints, state, etc.)
    context: DebugContext,
    /// Event callback
    event_callback: Option<Box<dyn FnMut(DebugEvent)>>,
    /// Current source file
    current_file: PathBuf,
    /// Frame ID counter
    frame_id_counter: u32,
}

impl DebugInterpreter {
    /// Create a new debug interpreter for the given module
    pub fn new(module: Module, context: DebugContext) -> Self {
        Self {
            module,
            call_stack: Vec::new(),
            globals: HashMap::new(),
            current_exception: None,
            context,
            event_callback: None,
            current_file: PathBuf::from("unknown"),
            frame_id_counter: 0,
        }
    }

    /// Set the current source file
    pub fn set_source_file(&mut self, file: PathBuf) {
        self.current_file = file;
    }

    /// Set event callback
    pub fn set_event_callback<F>(&mut self, callback: F)
    where
        F: FnMut(DebugEvent) + 'static,
    {
        self.event_callback = Some(Box::new(callback));
    }

    /// Get the debug context
    pub fn context(&self) -> &DebugContext {
        &self.context
    }

    /// Get mutable debug context
    pub fn context_mut(&mut self) -> &mut DebugContext {
        &mut self.context
    }

    /// Emit a debug event
    fn emit_event(&mut self, event: DebugEvent) {
        if let Some(ref mut callback) = self.event_callback {
            callback(event);
        }
    }

    /// Initialize and start debugging
    pub fn start(&mut self) -> DebugResult<()> {
        // Initialize globals
        self.init_globals()?;

        // Set state to running or paused at entry
        if self.context.config().stop_on_entry {
            self.context.set_state(DebugState::Paused {
                reason: PauseReason::Entry,
            });
        } else {
            self.context.set_state(DebugState::Running);
        }

        self.emit_event(DebugEvent::Started);
        Ok(())
    }

    /// Run until a breakpoint, step completion, or termination
    pub fn run(&mut self) -> DebugResult<StepResult> {
        // Find main function
        let main_func = self.find_main_function()?;

        // Execute main
        match self.call_function(&main_func, vec![]) {
            Ok(value) => {
                self.context.set_state(DebugState::Terminated {
                    exit_value: Some(value.to_display_string()),
                });
                self.emit_event(DebugEvent::Exited { exit_code: 0 });
                Ok(StepResult::Completed(value))
            }
            Err(e) => {
                self.context.set_state(DebugState::Error {
                    message: e.message.clone(),
                    message_ar: e.message_ar.clone(),
                });
                Err(DebugError::new(e.message, e.message_ar))
            }
        }
    }

    /// Continue execution until next breakpoint or termination
    pub fn continue_execution(&mut self) -> DebugResult<StepResult> {
        self.context.set_state(DebugState::Running);
        self.emit_event(DebugEvent::Continued);
        self.run()
    }

    /// Step to next instruction
    pub fn step(&mut self, mode: StepMode) -> DebugResult<StepResult> {
        let current_depth = self.call_stack.len();
        let current_location = self.get_current_location();
        let current_func = self.call_stack.last().map(|f| f.func_id.0.as_str());

        self.context.start_stepping(
            mode,
            current_depth,
            current_location.as_ref().map(|l| l.line),
            current_func,
        );

        // Execute until step is complete
        loop {
            match self.execute_one_instruction()? {
                StepResult::Continue => {
                    let new_depth = self.call_stack.len();
                    let new_location = self.get_current_location();
                    let new_func = self.call_stack.last().map(|f| f.func_id.0.as_str());

                    if self.context.is_step_complete(
                        new_depth,
                        new_location.as_ref().map(|l| l.line),
                        new_func,
                    ) {
                        self.context.stop_stepping();
                        let reason = PauseReason::Step;
                        self.context.set_state(DebugState::Paused {
                            reason: reason.clone(),
                        });
                        self.emit_event(DebugEvent::Stopped {
                            reason,
                            location: new_location,
                        });
                        return Ok(StepResult::Paused(PauseReason::Step));
                    }
                }
                StepResult::Paused(reason) => {
                    self.context.stop_stepping();
                    return Ok(StepResult::Paused(reason));
                }
                result => {
                    self.context.stop_stepping();
                    return Ok(result);
                }
            }
        }
    }

    /// Execute a single instruction
    fn execute_one_instruction(&mut self) -> DebugResult<StepResult> {
        let Some(frame) = self.call_stack.last() else {
            return Ok(StepResult::Completed(Value::Null));
        };

        let func = self
            .module
            .get_function(&frame.func_id)
            .ok_or_else(|| DebugError::internal("Function not found"))?
            .clone();

        if frame.block_idx >= func.blocks.len() {
            return Ok(StepResult::Completed(Value::Null));
        }

        let block = &func.blocks[frame.block_idx];

        // Check for breakpoint before executing
        if let Some(location) = self.get_current_location() {
            if let Some(bp) = self.check_breakpoint(&location) {
                let reason = PauseReason::Breakpoint { id: bp };
                self.context.set_state(DebugState::Paused {
                    reason: reason.clone(),
                });
                self.emit_event(DebugEvent::BreakpointHit {
                    id: bp,
                    location: location.clone(),
                });
                return Ok(StepResult::Paused(reason));
            }
        }

        // Get current instruction
        let frame = self.call_stack.last().unwrap();
        if frame.inst_idx >= block.instructions.len() {
            // Move to next block or return
            return Ok(StepResult::Continue);
        }

        let inst = block.instructions[frame.inst_idx].clone();

        // Execute the instruction
        match self.execute_instruction(&inst, &func)? {
            InstructionResult::Continue => {
                // Move to next instruction
                if let Some(frame) = self.call_stack.last_mut() {
                    frame.inst_idx += 1;
                }
                Ok(StepResult::Continue)
            }
            InstructionResult::Jump(target) => {
                // Compute block index first to avoid borrow conflict
                let new_block_idx = self.find_block_index(&func, target)?;
                // Get current block ID before mutable borrow
                let current_frame = self.call_stack.last().unwrap();
                let prev_block_id = func.blocks[current_frame.block_idx].id;
                // Now update the frame
                if let Some(frame) = self.call_stack.last_mut() {
                    frame.prev_block = Some(prev_block_id);
                    frame.block_idx = new_block_idx;
                    frame.inst_idx = 0;
                }
                Ok(StepResult::Continue)
            }
            InstructionResult::Return(value) => {
                self.call_stack.pop();
                if self.call_stack.is_empty() {
                    Ok(StepResult::Returned(value))
                } else {
                    Ok(StepResult::Continue)
                }
            }
            InstructionResult::Throw(exception) => {
                // Check for exception handler
                if let Some(catch_block) = self.pop_try_block() {
                    // Compute block index first to avoid borrow conflict
                    let new_block_idx = self.find_block_index(&func, catch_block)?;
                    self.current_exception = Some(exception);
                    if let Some(frame) = self.call_stack.last_mut() {
                        frame.block_idx = new_block_idx;
                        frame.inst_idx = 0;
                    }
                    Ok(StepResult::Continue)
                } else {
                    Ok(StepResult::Exception(exception))
                }
            }
        }
    }

    /// Check if there's a breakpoint at the current location
    fn check_breakpoint(&mut self, location: &SourceLocation) -> Option<BreakpointId> {
        // Collect enabled breakpoint IDs first to avoid borrow conflict
        let bp_ids: Vec<BreakpointId> = self
            .context
            .breakpoints_at(&location.file, location.line)
            .into_iter()
            .filter(|bp| bp.enabled)
            .map(|bp| bp.id)
            .collect();

        // Now check each breakpoint with mutable access
        for id in bp_ids {
            if let Some(bp_mut) = self.context.get_breakpoint_mut(id) {
                if bp_mut.should_trigger() {
                    return Some(id);
                }
            }
        }
        None
    }

    /// Get current source location
    fn get_current_location(&self) -> Option<SourceLocation> {
        let frame = self.call_stack.last()?;
        let func = self.module.get_function(&frame.func_id)?;

        if frame.block_idx >= func.blocks.len() {
            return None;
        }

        let block = &func.blocks[frame.block_idx];

        self.context
            .source_map()
            .get_instruction_location(&frame.func_id, block.id, frame.inst_idx)
            .cloned()
    }

    /// Get the current call stack
    pub fn get_stack_trace(&self) -> Vec<StackFrame> {
        self.call_stack
            .iter()
            .enumerate()
            .rev()
            .map(|(idx, frame)| {
                let func = self.module.get_function(&frame.func_id);
                let name = func
                    .map(|f| f.name.clone())
                    .unwrap_or_else(|| frame.func_id.0.clone());

                let block_id = func
                    .and_then(|f| f.blocks.get(frame.block_idx))
                    .map(|b| b.id)
                    .unwrap_or(BlockId(0));

                let mut sf = StackFrame::new(
                    idx as u32,
                    name,
                    frame.func_id.clone(),
                    block_id,
                    frame.inst_idx,
                );

                // Add source location if available
                if let Some(loc) = self.context.source_map().get_instruction_location(
                    &frame.func_id,
                    block_id,
                    frame.inst_idx,
                ) {
                    sf = sf.with_location(loc.clone());
                }

                sf
            })
            .collect()
    }

    /// Get local variables for the current frame
    pub fn get_locals(&self) -> Vec<DebugVariable> {
        let Some(frame) = self.call_stack.last() else {
            return Vec::new();
        };

        frame
            .locals
            .iter()
            .map(|(&var_id, value)| {
                // Try to get variable name from source map
                let var_info = self
                    .context
                    .source_map()
                    .get_variable_info(&frame.func_id, VarId(var_id));

                let name = var_info
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| format!("%{}", var_id));

                let is_mutable = var_info.map(|i| i.is_mutable).unwrap_or(true);

                DebugVariable::new(name, value.clone(), is_mutable).with_var_id(VarId(var_id))
            })
            .collect()
    }

    /// Get global variables
    pub fn get_globals(&self) -> Vec<DebugVariable> {
        self.globals
            .iter()
            .map(|(name, value)| DebugVariable::new(name.clone(), value.clone(), true))
            .collect()
    }

    /// Get scopes for the current frame
    pub fn get_scopes(&self) -> Vec<DebugScope> {
        vec![
            DebugScope::new("Locals / محليات".to_string()).with_variables(self.get_locals()),
            DebugScope::new("Globals / عالميات".to_string()).with_variables(self.get_globals()),
        ]
    }

    /// Evaluate an expression in the current context
    pub fn evaluate(&self, expression: &str) -> DebugResult<Value> {
        // For now, just handle simple variable lookups
        let frame = self.call_stack.last();

        // Check if it's a simple variable name
        if let Some(frame) = frame {
            // Try to find by name in source map
            for (&var_id, value) in &frame.locals {
                if let Some(info) = self
                    .context
                    .source_map()
                    .get_variable_info(&frame.func_id, VarId(var_id))
                {
                    if info.name == expression {
                        return Ok(value.clone());
                    }
                }
            }

            // Try parsing as VarId (%n format)
            if let Some(stripped) = expression.strip_prefix('%') {
                if let Ok(var_id) = stripped.parse::<u32>() {
                    if let Some(value) = frame.locals.get(&var_id) {
                        return Ok(value.clone());
                    }
                }
            }
        }

        // Check globals
        if let Some(value) = self.globals.get(expression) {
            return Ok(value.clone());
        }

        Err(DebugError::new(
            format!("Cannot evaluate expression: {}", expression),
            format!("لا يمكن تقييم التعبير: {}", expression),
        ))
    }

    // ==================== Interpreter Core Logic ====================

    /// Initialize global variables
    fn init_globals(&mut self) -> DebugResult<()> {
        for (name, _ty, init) in &self.module.globals {
            let value = match init {
                Some(c) => self.constant_to_value(c),
                None => Value::Null,
            };
            self.globals.insert(name.clone(), value);
        }
        Ok(())
    }

    /// Find the main function
    fn find_main_function(&self) -> DebugResult<FuncId> {
        let main_names = ["__main__", "main", "رئيسية", "البداية"];

        for name in main_names {
            let func_id = FuncId(name.to_string());
            if self.module.get_function(&func_id).is_some() {
                return Ok(func_id);
            }
        }

        if let Some(func) = self.module.functions.first() {
            return Ok(func.id.clone());
        }

        Err(DebugError::new(
            "Main function not found",
            "الدالة الرئيسية غير موجودة",
        ))
    }

    /// Call a function with arguments
    fn call_function(&mut self, func_id: &FuncId, args: Vec<Value>) -> RuntimeResult<Value> {
        if self.call_stack.len() >= MAX_STACK_DEPTH {
            return Err(RuntimeError::stack_overflow());
        }

        let func = self
            .module
            .get_function(func_id)
            .ok_or_else(|| RuntimeError::undefined_function(&func_id.0))?
            .clone();

        let mut frame = DebugCallFrame::new(func_id.clone());

        for (i, param) in func.params.iter().enumerate() {
            let value = args.get(i).cloned().unwrap_or(Value::Null);
            frame.locals.insert(param.id.0, value);
        }

        self.call_stack.push(frame);

        let result = self.execute_function(&func);

        self.call_stack.pop();

        result
    }

    /// Execute a function
    fn execute_function(&mut self, func: &Function) -> RuntimeResult<Value> {
        if func.blocks.is_empty() {
            return Ok(Value::Null);
        }

        loop {
            let frame = self.call_stack.last().unwrap();
            let block_idx = frame.block_idx;

            if block_idx >= func.blocks.len() {
                return Ok(Value::Null);
            }

            let block = &func.blocks[block_idx];
            let result = self.execute_block(block, func)?;

            match result {
                BlockResult::Continue => {
                    if let Some(frame) = self.call_stack.last_mut() {
                        frame.block_idx += 1;
                        frame.inst_idx = 0;
                        if frame.block_idx >= func.blocks.len() {
                            return Ok(Value::Null);
                        }
                    }
                }
                BlockResult::Jump(target) => {
                    // Compute block index first to avoid borrow conflict
                    let new_block_idx = self.find_block_index(func, target)?;
                    if let Some(frame) = self.call_stack.last_mut() {
                        let current_block = &func.blocks[frame.block_idx];
                        frame.prev_block = Some(current_block.id);
                        frame.block_idx = new_block_idx;
                        frame.inst_idx = 0;
                    }
                }
                BlockResult::Return(value) => {
                    return Ok(value);
                }
                BlockResult::Throw(exception) => {
                    if let Some(catch_block) = self.pop_try_block() {
                        // Compute block index first to avoid borrow conflict
                        let new_block_idx = self.find_block_index(func, catch_block)?;
                        self.current_exception = Some(exception);
                        if let Some(frame) = self.call_stack.last_mut() {
                            frame.block_idx = new_block_idx;
                            frame.inst_idx = 0;
                        }
                    } else {
                        let msg = exception.to_display_string();
                        return Err(RuntimeError::unhandled_exception(&msg));
                    }
                }
            }
        }
    }

    /// Find block index by ID
    fn find_block_index(&self, func: &Function, block_id: BlockId) -> RuntimeResult<usize> {
        func.blocks
            .iter()
            .position(|b| b.id == block_id)
            .ok_or_else(|| RuntimeError::internal(format!("Block {} not found", block_id)))
    }

    /// Execute a basic block
    fn execute_block(&mut self, block: &BasicBlock, func: &Function) -> RuntimeResult<BlockResult> {
        // Update frame instruction index
        if let Some(frame) = self.call_stack.last_mut() {
            frame.inst_idx = 0;
        }

        for (idx, inst) in block.instructions.iter().enumerate() {
            if let Some(frame) = self.call_stack.last_mut() {
                frame.inst_idx = idx;
            }

            // Check for breakpoint
            if let Some(location) = self.get_current_location() {
                if self
                    .context
                    .has_breakpoint_at(&location.file, location.line)
                {
                    // Breakpoint handling is done in the step loop
                }
            }

            match self.execute_instruction(inst, func)? {
                InstructionResult::Continue => {}
                InstructionResult::Jump(target) => {
                    return Ok(BlockResult::Jump(target));
                }
                InstructionResult::Return(value) => {
                    return Ok(BlockResult::Return(value));
                }
                InstructionResult::Throw(exception) => {
                    return Ok(BlockResult::Throw(exception));
                }
            }
        }
        Ok(BlockResult::Continue)
    }

    /// Execute a single instruction
    fn execute_instruction(
        &mut self,
        inst: &Instruction,
        _func: &Function,
    ) -> RuntimeResult<InstructionResult> {
        match inst {
            Instruction::Const { dest, value, .. } => {
                let val = self.constant_to_value(value);
                self.set_local(*dest, val);
                Ok(InstructionResult::Continue)
            }

            Instruction::Binary {
                dest,
                op,
                left,
                right,
                ty,
            } => {
                let left_val = self.get_local(*left)?;
                let right_val = self.get_local(*right)?;
                let result = self.execute_binary_op(*op, left_val, right_val, ty)?;
                self.set_local(*dest, result);
                Ok(InstructionResult::Continue)
            }

            Instruction::Unary {
                dest,
                op,
                operand,
                ty,
            } => {
                let operand_val = self.get_local(*operand)?;
                let result = self.execute_unary_op(*op, operand_val, ty)?;
                self.set_local(*dest, result);
                Ok(InstructionResult::Continue)
            }

            Instruction::IntToFloat { dest, src } => {
                let val = self.get_local(*src)?;
                let result = match val {
                    Value::Int(i) => Value::Float(i as f64),
                    Value::Float(f) => Value::Float(f),
                    _ => return Err(RuntimeError::type_error("int", val.type_name())),
                };
                self.set_local(*dest, result);
                Ok(InstructionResult::Continue)
            }

            Instruction::FloatToInt { dest, src } => {
                let val = self.get_local(*src)?;
                let result = match val {
                    Value::Float(f) => Value::Int(f as i64),
                    Value::Int(i) => Value::Int(i),
                    _ => return Err(RuntimeError::type_error("float", val.type_name())),
                };
                self.set_local(*dest, result);
                Ok(InstructionResult::Continue)
            }

            Instruction::ToString { dest, src } => {
                let val = self.get_local(*src)?;
                let result = Value::string(val.to_display_string());
                self.set_local(*dest, result);
                Ok(InstructionResult::Continue)
            }

            Instruction::Bitcast { dest, src, .. } => {
                let val = self.get_local(*src)?;
                self.set_local(*dest, val);
                Ok(InstructionResult::Continue)
            }

            Instruction::Alloca { dest, .. } => {
                let ptr = Value::ptr(Value::Null);
                self.set_local(*dest, ptr);
                Ok(InstructionResult::Continue)
            }

            Instruction::Load { dest, ptr, .. } => {
                let ptr_val = self.get_local(*ptr)?;
                let result = match ptr_val {
                    Value::Ptr(p) => p.borrow().clone(),
                    _ => return Err(RuntimeError::type_error("ptr", ptr_val.type_name())),
                };
                self.set_local(*dest, result);
                Ok(InstructionResult::Continue)
            }

            Instruction::Store { ptr, value } => {
                let ptr_val = self.get_local(*ptr)?;
                let val = self.get_local(*value)?;
                match ptr_val {
                    Value::Ptr(p) => {
                        *p.borrow_mut() = val;
                    }
                    _ => return Err(RuntimeError::type_error("ptr", ptr_val.type_name())),
                }
                Ok(InstructionResult::Continue)
            }

            Instruction::GlobalLoad { dest, name, .. } => {
                let val = self.globals.get(name).cloned().unwrap_or(Value::Null);
                self.set_local(*dest, val);
                Ok(InstructionResult::Continue)
            }

            Instruction::GlobalStore { name, value } => {
                let val = self.get_local(*value)?;
                self.globals.insert(name.clone(), val);
                Ok(InstructionResult::Continue)
            }

            Instruction::GetElementPtr {
                dest, ptr, index, ..
            } => {
                let ptr_val = self.get_local(*ptr)?;
                let idx_val = self.get_local(*index)?;
                let idx = idx_val
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", idx_val.type_name()))?;

                match ptr_val {
                    Value::Array(arr) => {
                        let arr_ref = arr.borrow();
                        if idx < 0 || (idx as usize) >= arr_ref.len() {
                            return Err(RuntimeError::index_out_of_bounds(idx, arr_ref.len()));
                        }
                        let elem = arr_ref[idx as usize].clone();
                        self.set_local(*dest, Value::ptr(elem));
                    }
                    _ => {
                        self.set_local(*dest, ptr_val);
                    }
                }
                Ok(InstructionResult::Continue)
            }

            Instruction::Jump { target } => Ok(InstructionResult::Jump(*target)),

            Instruction::Branch {
                cond,
                then_block,
                else_block,
            } => {
                let cond_val = self.get_local(*cond)?;
                if cond_val.is_truthy() {
                    Ok(InstructionResult::Jump(*then_block))
                } else {
                    Ok(InstructionResult::Jump(*else_block))
                }
            }

            Instruction::Return { value } => {
                let result = match value {
                    Some(v) => self.get_local(*v)?,
                    None => Value::Null,
                };
                Ok(InstructionResult::Return(result))
            }

            Instruction::Call {
                dest, func, args, ..
            } => {
                let arg_values: Vec<Value> = args
                    .iter()
                    .map(|v| self.get_local(*v))
                    .collect::<RuntimeResult<Vec<_>>>()?;

                let result = if self.is_builtin(&func.0) {
                    self.call_builtin(&func.0, arg_values)?
                } else {
                    self.call_function(func, arg_values)?
                };

                if let Some(d) = dest {
                    self.set_local(*d, result);
                }
                Ok(InstructionResult::Continue)
            }

            Instruction::CallIndirect {
                dest,
                func_ptr,
                args,
                ..
            } => {
                let func_val = self.get_local(*func_ptr)?;
                let func_name = match func_val {
                    Value::Function(name) => name,
                    _ => return Err(RuntimeError::type_error("function", func_val.type_name())),
                };

                let arg_values: Vec<Value> = args
                    .iter()
                    .map(|v| self.get_local(*v))
                    .collect::<RuntimeResult<Vec<_>>>()?;

                let result = self.call_function(&FuncId(func_name), arg_values)?;

                if let Some(d) = dest {
                    self.set_local(*d, result);
                }
                Ok(InstructionResult::Continue)
            }

            Instruction::NewObject { dest, class } => {
                let obj = Value::object(class.clone());
                if let Some(class_def) = self.module.get_class(class) {
                    if let Value::Object(ref obj_rc) = obj {
                        let mut obj_mut = obj_rc.borrow_mut();
                        for (field_name, _) in &class_def.fields {
                            obj_mut.fields.insert(field_name.clone(), Value::Null);
                        }
                    }
                }
                self.set_local(*dest, obj);
                Ok(InstructionResult::Continue)
            }

            Instruction::GetField {
                dest,
                object,
                field,
                ..
            } => {
                let obj_val = self.get_local(*object)?;
                match obj_val {
                    Value::Object(obj) => {
                        let obj_ref = obj.borrow();
                        let value = obj_ref
                            .get_field(&field.name)
                            .cloned()
                            .unwrap_or(Value::Null);
                        self.set_local(*dest, value);
                    }
                    _ => return Err(RuntimeError::type_error("object", obj_val.type_name())),
                }
                Ok(InstructionResult::Continue)
            }

            Instruction::SetField {
                object,
                field,
                value,
            } => {
                let obj_val = self.get_local(*object)?;
                let new_value = self.get_local(*value)?;
                match obj_val {
                    Value::Object(obj) => {
                        let mut obj_mut = obj.borrow_mut();
                        obj_mut.set_field(field.name.clone(), new_value);
                    }
                    _ => return Err(RuntimeError::type_error("object", obj_val.type_name())),
                }
                Ok(InstructionResult::Continue)
            }

            Instruction::CallMethod {
                dest,
                object,
                method,
                args,
                ..
            } => {
                let obj_val = self.get_local(*object)?;
                let mut arg_values = vec![obj_val.clone()];
                for arg in args {
                    arg_values.push(self.get_local(*arg)?);
                }

                let method_func_id = FuncId(format!("{}::{}", method.class.0, method.name));
                let result = self.call_function(&method_func_id, arg_values)?;

                if let Some(d) = dest {
                    self.set_local(*d, result);
                }
                Ok(InstructionResult::Continue)
            }

            Instruction::CallVirtual {
                dest,
                object,
                method_index,
                args,
                ..
            } => {
                let obj_val = self.get_local(*object)?;

                let class_id = match &obj_val {
                    Value::Object(obj) => obj.borrow().class_id.clone(),
                    _ => return Err(RuntimeError::type_error("object", obj_val.type_name())),
                };

                let method_id = if let Some(class) = self.module.get_class(&class_id) {
                    class
                        .vtable
                        .get(*method_index as usize)
                        .cloned()
                        .ok_or_else(|| {
                            RuntimeError::internal(format!(
                                "vtable index {} out of bounds",
                                method_index
                            ))
                        })?
                } else {
                    return Err(RuntimeError::internal(format!(
                        "Class {} not found",
                        class_id.0
                    )));
                };

                let mut arg_values = vec![obj_val];
                for arg in args {
                    arg_values.push(self.get_local(*arg)?);
                }

                let method_func_id = FuncId(format!("{}::{}", method_id.class.0, method_id.name));
                let result = self.call_function(&method_func_id, arg_values)?;

                if let Some(d) = dest {
                    self.set_local(*d, result);
                }
                Ok(InstructionResult::Continue)
            }

            Instruction::NewArray { dest, elements, .. } => {
                let values: Vec<Value> = elements
                    .iter()
                    .map(|v| self.get_local(*v))
                    .collect::<RuntimeResult<Vec<_>>>()?;
                self.set_local(*dest, Value::array_from(values));
                Ok(InstructionResult::Continue)
            }

            Instruction::ArrayLen { dest, array } => {
                let arr_val = self.get_local(*array)?;
                match arr_val {
                    Value::Array(arr) => {
                        let len = arr.borrow().len() as i64;
                        self.set_local(*dest, Value::Int(len));
                    }
                    Value::String(s) => {
                        let len = s.chars().count() as i64;
                        self.set_local(*dest, Value::Int(len));
                    }
                    _ => return Err(RuntimeError::type_error("array", arr_val.type_name())),
                }
                Ok(InstructionResult::Continue)
            }

            Instruction::ArrayGet {
                dest, array, index, ..
            } => {
                let arr_val = self.get_local(*array)?;
                let idx_val = self.get_local(*index)?;
                let idx = idx_val
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", idx_val.type_name()))?;

                let result = match arr_val {
                    Value::Array(arr) => {
                        let arr_ref = arr.borrow();
                        if idx < 0 || (idx as usize) >= arr_ref.len() {
                            return Err(RuntimeError::index_out_of_bounds(idx, arr_ref.len()));
                        }
                        arr_ref[idx as usize].clone()
                    }
                    Value::String(s) => {
                        let chars: Vec<char> = s.chars().collect();
                        if idx < 0 || (idx as usize) >= chars.len() {
                            return Err(RuntimeError::index_out_of_bounds(idx, chars.len()));
                        }
                        Value::string(chars[idx as usize].to_string())
                    }
                    _ => return Err(RuntimeError::type_error("array", arr_val.type_name())),
                };

                self.set_local(*dest, result);
                Ok(InstructionResult::Continue)
            }

            Instruction::ArraySet {
                array,
                index,
                value,
            } => {
                let arr_val = self.get_local(*array)?;
                let idx_val = self.get_local(*index)?;
                let new_val = self.get_local(*value)?;
                let idx = idx_val
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", idx_val.type_name()))?;

                match arr_val {
                    Value::Array(arr) => {
                        let mut arr_mut = arr.borrow_mut();
                        if idx < 0 || (idx as usize) >= arr_mut.len() {
                            return Err(RuntimeError::index_out_of_bounds(idx, arr_mut.len()));
                        }
                        arr_mut[idx as usize] = new_val;
                    }
                    _ => return Err(RuntimeError::type_error("array", arr_val.type_name())),
                }
                Ok(InstructionResult::Continue)
            }

            Instruction::ArrayPush { array, value, .. } => {
                let arr_val = self.get_local(*array)?;
                let new_val = self.get_local(*value)?;

                match arr_val {
                    Value::Array(arr) => {
                        arr.borrow_mut().push(new_val);
                    }
                    _ => return Err(RuntimeError::type_error("array", arr_val.type_name())),
                }
                Ok(InstructionResult::Continue)
            }

            Instruction::StringConcat { dest, left, right } => {
                let left_val = self.get_local(*left)?;
                let right_val = self.get_local(*right)?;
                let left_str = left_val.to_display_string();
                let right_str = right_val.to_display_string();
                self.set_local(*dest, Value::string(left_str + &right_str));
                Ok(InstructionResult::Continue)
            }

            Instruction::TryBegin { catch_block } => {
                if let Some(frame) = self.call_stack.last_mut() {
                    frame.try_stack.push(*catch_block);
                }
                Ok(InstructionResult::Continue)
            }

            Instruction::TryEnd => {
                self.pop_try_block();
                Ok(InstructionResult::Continue)
            }

            Instruction::Throw { exception } => {
                let exc_val = self.get_local(*exception)?;
                Ok(InstructionResult::Throw(exc_val))
            }

            Instruction::GetException { dest } => {
                let exc = self.current_exception.take().unwrap_or(Value::Null);
                self.set_local(*dest, exc);
                Ok(InstructionResult::Continue)
            }

            Instruction::Phi { dest, incoming, .. } => {
                let prev_block = self
                    .call_stack
                    .last()
                    .and_then(|f| f.prev_block)
                    .unwrap_or(BlockId(0));

                let value = incoming
                    .iter()
                    .find(|(_, block)| *block == prev_block)
                    .map(|(var, _)| self.get_local(*var))
                    .transpose()?
                    .unwrap_or(Value::Null);

                self.set_local(*dest, value);
                Ok(InstructionResult::Continue)
            }

            Instruction::Print { value } => {
                let val = self.get_local(*value)?;
                let output = val.to_display_string();

                // Add to output buffer
                self.context.add_output(output.clone());

                // Also print to stdout
                println!("{}", output);
                io::stdout().flush().ok();

                Ok(InstructionResult::Continue)
            }

            Instruction::Nop => Ok(InstructionResult::Continue),
        }
    }

    // ==================== Helper Methods ====================

    fn execute_binary_op(
        &self,
        op: BinaryOp,
        left: Value,
        right: Value,
        _ty: &IrType,
    ) -> RuntimeResult<Value> {
        match op {
            BinaryOp::Add => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_add(*b))),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
                (Value::String(a), Value::String(b)) => Ok(Value::string(format!("{}{}", a, b))),
                _ => Err(RuntimeError::type_error(
                    "numeric or string",
                    &format!("{} and {}", left.type_name(), right.type_name()),
                )),
            },
            BinaryOp::Sub => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_sub(*b))),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
                _ => Err(RuntimeError::type_error("numeric", left.type_name())),
            },
            BinaryOp::Mul => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_mul(*b))),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
                _ => Err(RuntimeError::type_error("numeric", left.type_name())),
            },
            BinaryOp::Div => match (&left, &right) {
                (Value::Int(_), Value::Int(0)) => Err(RuntimeError::division_by_zero()),
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a / b)),
                (Value::Float(_), Value::Float(b)) if *b == 0.0 => {
                    Err(RuntimeError::division_by_zero())
                }
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                (Value::Int(_), Value::Float(b)) if *b == 0.0 => {
                    Err(RuntimeError::division_by_zero())
                }
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 / b)),
                (Value::Float(_), Value::Int(0)) => Err(RuntimeError::division_by_zero()),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a / *b as f64)),
                _ => Err(RuntimeError::type_error("numeric", left.type_name())),
            },
            BinaryOp::Mod => match (&left, &right) {
                (Value::Int(_), Value::Int(0)) => Err(RuntimeError::division_by_zero()),
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a % b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a % b)),
                _ => Err(RuntimeError::type_error("numeric", left.type_name())),
            },
            BinaryOp::Pow => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) if *b >= 0 => Ok(Value::Int(a.pow(*b as u32))),
                (Value::Int(a), Value::Int(b)) => Ok(Value::Float((*a as f64).powf(*b as f64))),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(*b))),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).powf(*b))),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.powf(*b as f64))),
                _ => Err(RuntimeError::type_error("numeric", left.type_name())),
            },
            BinaryOp::Eq => Ok(Value::Bool(left == right)),
            BinaryOp::Ne => Ok(Value::Bool(left != right)),
            BinaryOp::Lt => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a < b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) < *b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a < (*b as f64))),
                (Value::String(a), Value::String(b)) => Ok(Value::Bool(a < b)),
                _ => Err(RuntimeError::type_error("comparable", left.type_name())),
            },
            BinaryOp::Le => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a <= b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a <= b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) <= *b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a <= (*b as f64))),
                (Value::String(a), Value::String(b)) => Ok(Value::Bool(a <= b)),
                _ => Err(RuntimeError::type_error("comparable", left.type_name())),
            },
            BinaryOp::Gt => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a > b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) > *b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a > (*b as f64))),
                (Value::String(a), Value::String(b)) => Ok(Value::Bool(a > b)),
                _ => Err(RuntimeError::type_error("comparable", left.type_name())),
            },
            BinaryOp::Ge => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a >= b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a >= b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) >= *b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a >= (*b as f64))),
                (Value::String(a), Value::String(b)) => Ok(Value::Bool(a >= b)),
                _ => Err(RuntimeError::type_error("comparable", left.type_name())),
            },
            BinaryOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            BinaryOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
            BinaryOp::BitAnd => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a & b)),
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
                _ => Err(RuntimeError::type_error("int or bool", left.type_name())),
            },
            BinaryOp::BitOr => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a | b)),
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a || *b)),
                _ => Err(RuntimeError::type_error("int or bool", left.type_name())),
            },
            BinaryOp::BitXor => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a ^ b)),
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a ^ *b)),
                _ => Err(RuntimeError::type_error("int or bool", left.type_name())),
            },
            BinaryOp::Shl => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => {
                    if *b < 0 || *b >= 64 {
                        return Err(RuntimeError::invalid_operation(
                            format!("Shift amount {} is out of range (0-63)", b),
                            format!("مقدار الإزاحة {} خارج النطاق (0-63)", b),
                        ));
                    }
                    Ok(Value::Int(*a << *b))
                }
                _ => Err(RuntimeError::type_error("int", left.type_name())),
            },
            BinaryOp::Shr => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => {
                    if *b < 0 || *b >= 64 {
                        return Err(RuntimeError::invalid_operation(
                            format!("Shift amount {} is out of range (0-63)", b),
                            format!("مقدار الإزاحة {} خارج النطاق (0-63)", b),
                        ));
                    }
                    Ok(Value::Int(*a >> *b))
                }
                _ => Err(RuntimeError::type_error("int", left.type_name())),
            },
        }
    }

    fn execute_unary_op(&self, op: UnaryOp, operand: Value, _ty: &IrType) -> RuntimeResult<Value> {
        match op {
            UnaryOp::Neg => match operand {
                Value::Int(i) => Ok(Value::Int(-i)),
                Value::Float(f) => Ok(Value::Float(-f)),
                _ => Err(RuntimeError::type_error("numeric", operand.type_name())),
            },
            UnaryOp::Not => Ok(Value::Bool(!operand.is_truthy())),
            UnaryOp::BitNot => match operand {
                Value::Int(i) => Ok(Value::Int(!i)),
                Value::Bool(b) => Ok(Value::Bool(!b)),
                _ => Err(RuntimeError::type_error("int or bool", operand.type_name())),
            },
        }
    }

    fn constant_to_value(&self, constant: &Constant) -> Value {
        match constant {
            Constant::Null => Value::Null,
            Constant::Bool(b) => Value::Bool(*b),
            Constant::Int(i) => Value::Int(*i),
            Constant::Float(f) => Value::Float(*f),
            Constant::String(idx) => {
                let s = self.module.strings.get(*idx).unwrap_or("").to_string();
                Value::string(s)
            }
        }
    }

    fn get_local(&self, var: VarId) -> RuntimeResult<Value> {
        self.call_stack
            .last()
            .and_then(|frame| frame.locals.get(&var.0))
            .cloned()
            .ok_or_else(|| RuntimeError::undefined_variable(&format!("%{}", var.0)))
    }

    fn set_local(&mut self, var: VarId, value: Value) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.locals.insert(var.0, value);
        }
    }

    fn pop_try_block(&mut self) -> Option<BlockId> {
        self.call_stack
            .last_mut()
            .and_then(|frame| frame.try_stack.pop())
    }

    fn is_builtin(&self, name: &str) -> bool {
        matches!(
            name,
            "print"
                | "اطبع"
                | "println"
                | "input"
                | "ادخل"
                | "len"
                | "طول"
                | "type"
                | "نوع"
                | "int"
                | "عدد"
                | "float"
                | "عدد_عشري"
                | "str"
                | "نص"
                | "bool"
                | "منطقي"
                | "abs"
                | "sqrt"
                | "جذر"
                | "sin"
                | "cos"
                | "tan"
                | "floor"
                | "ceil"
                | "round"
        )
    }

    fn call_builtin(&mut self, name: &str, args: Vec<Value>) -> RuntimeResult<Value> {
        match name {
            "print" | "اطبع" | "println" => {
                let output = args
                    .iter()
                    .map(|v| v.to_display_string())
                    .collect::<Vec<_>>()
                    .join(" ");

                self.context.add_output(output.clone());
                println!("{}", output);
                io::stdout().flush().ok();
                Ok(Value::Null)
            }

            "input" | "ادخل" => {
                if let Some(prompt) = args.first() {
                    print!("{}", prompt.to_display_string());
                    io::stdout().flush().ok();
                }

                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| RuntimeError::internal(format!("Input error: {}", e)))?;

                Ok(Value::string(input.trim_end()))
            }

            "len" | "طول" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "len() requires 1 argument",
                        "طول() تتطلب معامل واحد",
                    )
                })?;

                match val {
                    Value::Array(arr) => Ok(Value::Int(arr.borrow().len() as i64)),
                    Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
                    _ => Err(RuntimeError::type_error("array or string", val.type_name())),
                }
            }

            "type" | "نوع" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "type() requires 1 argument",
                        "نوع() تتطلب معامل واحد",
                    )
                })?;
                Ok(Value::string(val.type_name_ar()))
            }

            "int" | "عدد" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "int() requires 1 argument",
                        "عدد() تتطلب معامل واحد",
                    )
                })?;

                match val {
                    Value::Int(i) => Ok(Value::Int(*i)),
                    Value::Float(f) => Ok(Value::Int(*f as i64)),
                    Value::String(s) => s
                        .parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| RuntimeError::type_error("numeric string", "invalid string")),
                    Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                    _ => Err(RuntimeError::type_error(
                        "convertible to int",
                        val.type_name(),
                    )),
                }
            }

            "float" | "عدد_عشري" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "float() requires 1 argument",
                        "عدد_عشري() تتطلب معامل واحد",
                    )
                })?;

                match val {
                    Value::Int(i) => Ok(Value::Float(*i as f64)),
                    Value::Float(f) => Ok(Value::Float(*f)),
                    Value::String(s) => s
                        .parse::<f64>()
                        .map(Value::Float)
                        .map_err(|_| RuntimeError::type_error("numeric string", "invalid string")),
                    _ => Err(RuntimeError::type_error(
                        "convertible to float",
                        val.type_name(),
                    )),
                }
            }

            "str" | "نص" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "str() requires 1 argument",
                        "نص() تتطلب معامل واحد",
                    )
                })?;
                Ok(Value::string(val.to_display_string()))
            }

            "bool" | "منطقي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "bool() requires 1 argument",
                        "منطقي() تتطلب معامل واحد",
                    )
                })?;
                Ok(Value::Bool(val.is_truthy()))
            }

            "abs" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "abs() requires 1 argument",
                        "abs() تتطلب معامل واحد",
                    )
                })?;

                match val {
                    Value::Int(i) => Ok(Value::Int(i.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => Err(RuntimeError::type_error("numeric", val.type_name())),
                }
            }

            "sqrt" | "جذر" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sqrt() requires 1 argument",
                        "جذر() تتطلب معامل واحد",
                    )
                })?;

                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sqrt()))
            }

            "sin" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sin() requires 1 argument",
                        "sin() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sin()))
            }

            "cos" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "cos() requires 1 argument",
                        "cos() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.cos()))
            }

            "tan" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "tan() requires 1 argument",
                        "tan() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.tan()))
            }

            "floor" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "floor() requires 1 argument",
                        "floor() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.floor()))
            }

            "ceil" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "ceil() requires 1 argument",
                        "ceil() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.ceil()))
            }

            "round" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "round() requires 1 argument",
                        "round() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.round()))
            }

            _ => Err(RuntimeError::undefined_function(name)),
        }
    }
}

/// Result of executing a block
enum BlockResult {
    Continue,
    Jump(BlockId),
    Return(Value),
    Throw(Value),
}

/// Result of executing an instruction
enum InstructionResult {
    Continue,
    Jump(BlockId),
    Return(Value),
    Throw(Value),
}
