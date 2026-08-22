//! Debug-aware interpreter
//!
//! This module extends the standard interpreter with debugging capabilities
//! including breakpoints, stepping, and variable inspection.

mod builtins;
mod operations;

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::interpreter::{ErrorKind, RuntimeError, RuntimeResult, Value};
use crate::ir::{
    BasicBlock, BlockId, Constant, FuncId, Function, Instruction, IrType, MethodId, Module, VarId,
};
use crate::semantic::EXCEPTION_MESSAGE_FIELD;

use super::context::{BreakpointId, DebugContext};
use super::source_map::SourceLocation;
use super::state::{
    DebugEvent, DebugScope, DebugState, DebugVariable, HeapAllocation, HeapChild, PauseReason,
    StackFrame, StepMode,
};
use super::{DebugError, DebugResult};

const MAX_STACK_DEPTH: usize = 1000;

#[derive(Debug)]
struct DebugCallFrame {
    func_id: FuncId,
    block_idx: usize,
    inst_idx: usize,
    locals: HashMap<u32, Value>,
    prev_block: Option<BlockId>,
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

#[derive(Debug)]
pub enum StepResult {
    Continue,
    Paused(PauseReason),
    Returned(Value),
    Exception(Value),
    Completed(Value),
}

#[allow(dead_code)]
pub struct DebugInterpreter {
    module: Module,
    call_stack: Vec<DebugCallFrame>,
    globals: HashMap<String, Value>,
    current_exception: Option<Value>,
    pub(super) context: DebugContext,
    event_callback: Option<Box<dyn FnMut(DebugEvent)>>,
    current_file: PathBuf,
    frame_id_counter: u32,
    /// Mirrors `Interpreter::virtual_method_cache` — memoizes
    /// `resolve_virtual_method`'s runtime-class walk.
    virtual_method_cache: HashMap<(String, String), FuncId>,
}

impl DebugInterpreter {
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
            virtual_method_cache: HashMap::new(),
        }
    }

    pub fn set_source_file(&mut self, file: PathBuf) {
        self.current_file = file;
    }

    pub fn set_event_callback<F>(&mut self, callback: F)
    where
        F: FnMut(DebugEvent) + 'static,
    {
        self.event_callback = Some(Box::new(callback));
    }

    pub fn context(&self) -> &DebugContext {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut DebugContext {
        &mut self.context
    }

    pub fn request_pause(&mut self) {
        self.context.request_pause();
    }

    pub fn set_local_variable(&mut self, name: &str, value_str: &str) -> DebugResult<Value> {
        let func_id = {
            let Some(frame) = self.call_stack.last() else {
                return Err(DebugError::new("لا يوجد إطار استدعاء نشط"));
            };
            frame.func_id.clone()
        };

        let var_id = self
            .context
            .source_map()
            .find_variable_by_name(&func_id, name)
            .ok_or_else(|| DebugError::new(format!("المتغير '{}' غير موجود", name)))?;

        let new_value = self.parse_value_string(value_str)?;

        if let Some(frame) = self.call_stack.last_mut() {
            frame.locals.insert(var_id.0, new_value.clone());
        }

        Ok(new_value)
    }

    pub fn set_global_variable(&mut self, name: &str, value_str: &str) -> DebugResult<Value> {
        if !self.globals.contains_key(name) {
            return Err(DebugError::new(format!(
                "المتغير العام '{}' غير موجود",
                name
            )));
        }

        let new_value = self.parse_value_string(value_str)?;
        self.globals.insert(name.to_string(), new_value.clone());

        Ok(new_value)
    }

    pub fn set_variable(&mut self, name: &str, value_str: &str) -> DebugResult<Value> {
        if self.set_local_variable(name, value_str).is_ok() {
            return self.set_local_variable(name, value_str);
        }

        self.set_global_variable(name, value_str)
    }

    pub fn parse_value_string(&self, value_str: &str) -> DebugResult<Value> {
        let trimmed = value_str.trim();

        if let Ok(n) = trimmed.parse::<i64>() {
            return Ok(Value::Int(n));
        }

        if let Ok(f) = trimmed.parse::<f64>() {
            return Ok(Value::Float(f));
        }

        match trimmed {
            "true" | "صحيح" => return Ok(Value::Bool(true)),
            "false" | "خطأ" => return Ok(Value::Bool(false)),
            "null" | "لا_شيء" => return Ok(Value::Null),
            _ => {}
        }

        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            let s = &trimmed[1..trimmed.len() - 1];
            return Ok(Value::String(s.to_string().into()));
        }

        Ok(Value::String(trimmed.to_string().into()))
    }

    fn emit_event(&mut self, event: DebugEvent) {
        if let Some(ref mut callback) = self.event_callback {
            callback(event);
        }
    }

    pub fn start(&mut self) -> DebugResult<()> {
        self.init_globals()?;

        // Find and prepare the main function BEFORE pausing
        // This ensures stackTrace has data when client requests it at entry point
        let main_func_id = self.find_main_function()?;

        // Log diagnostic info about the main function
        if let Some(func) = self.module.get_function(&main_func_id) {
            eprintln!(
                "[DEBUG] Main function '{}' found with {} blocks",
                main_func_id.0,
                func.blocks.len()
            );
            if func.blocks.is_empty() {
                eprintln!("[DEBUG] WARNING: Main function has NO blocks - stepping will fail!");
            } else {
                eprintln!(
                    "[DEBUG] First block has {} instructions",
                    func.blocks[0].instructions.len()
                );
            }
        } else {
            eprintln!(
                "[DEBUG] ERROR: Main function '{}' not found in module!",
                main_func_id.0
            );
        }

        let frame = DebugCallFrame::new(main_func_id);
        self.call_stack.push(frame);

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

    pub fn run(&mut self) -> DebugResult<StepResult> {
        let main_func = self.find_main_function()?;

        // Check if frame was already pushed by start() (stop_on_entry case)
        let frame_already_pushed = !self.call_stack.is_empty();

        let result = if frame_already_pushed {
            // Frame exists from start(), just execute the function
            let func = self
                .module
                .get_function(&main_func)
                .ok_or_else(|| DebugError::new("الدالة الرئيسية غير موجودة"))?
                .clone();
            let exec_result = self.execute_function(&func);
            self.call_stack.pop();
            exec_result
        } else {
            // Normal path: call_function will push/pop frame
            self.call_function(&main_func, vec![])
        };

        match result {
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
                });
                Err(DebugError::new(e.message))
            }
        }
    }

    pub fn continue_execution(&mut self) -> DebugResult<StepResult> {
        self.context.set_state(DebugState::Running);
        self.emit_event(DebugEvent::Continued);
        self.run()
    }

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

    fn execute_one_instruction(&mut self) -> DebugResult<StepResult> {
        let Some(frame) = self.call_stack.last() else {
            eprintln!("[DEBUG] execute_one_instruction: call_stack is empty, returning Completed");
            return Ok(StepResult::Completed(Value::Null));
        };

        eprintln!(
            "[DEBUG] execute_one_instruction: func={}, block_idx={}, inst_idx={}",
            frame.func_id.0, frame.block_idx, frame.inst_idx
        );

        let func = self
            .module
            .get_function(&frame.func_id)
            .ok_or_else(|| DebugError::internal("Function not found"))?
            .clone();

        eprintln!(
            "[DEBUG] Function '{}' has {} blocks",
            frame.func_id.0,
            func.blocks.len()
        );

        if frame.block_idx >= func.blocks.len() {
            eprintln!(
                "[DEBUG] block_idx ({}) >= blocks.len() ({}), returning Completed",
                frame.block_idx,
                func.blocks.len()
            );
            return Ok(StepResult::Completed(Value::Null));
        }

        let block = &func.blocks[frame.block_idx];

        if self.context.check_and_clear_pause() {
            let reason = PauseReason::UserRequest;
            self.context.set_state(DebugState::Paused {
                reason: reason.clone(),
            });
            let location = self.get_current_location();
            self.emit_event(DebugEvent::Stopped {
                reason: reason.clone(),
                location,
            });
            return Ok(StepResult::Paused(reason));
        }

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

        let frame = self
            .call_stack
            .last()
            .ok_or_else(|| DebugError::internal("Call stack unexpectedly empty"))?;
        if frame.inst_idx >= block.instructions.len() {
            return Ok(StepResult::Continue);
        }

        let inst = block.instructions[frame.inst_idx].clone();

        match self.execute_instruction(&inst, &func)? {
            InstructionResult::Continue => {
                if let Some(frame) = self.call_stack.last_mut() {
                    frame.inst_idx += 1;
                }
                Ok(StepResult::Continue)
            }
            InstructionResult::Jump(target) => {
                let new_block_idx = self.find_block_index(&func, target)?;
                let current_frame = self
                    .call_stack
                    .last()
                    .ok_or_else(|| DebugError::internal("Call stack unexpectedly empty"))?;
                let prev_block_id = func.blocks[current_frame.block_idx].id;
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
                if let Some(catch_block) = self.pop_try_block() {
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

    fn check_breakpoint(&mut self, location: &SourceLocation) -> Option<BreakpointId> {
        let bp_ids: Vec<BreakpointId> = self
            .context
            .breakpoints_at(&location.file, location.line)
            .into_iter()
            .filter(|bp| bp.enabled)
            .map(|bp| bp.id)
            .collect();

        for id in bp_ids {
            if let Some(bp_mut) = self.context.get_breakpoint_mut(id) {
                if bp_mut.should_trigger() {
                    return Some(id);
                }
            }
        }
        None
    }

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

    /// Returns the current depth of the call stack
    pub fn call_stack_depth(&self) -> usize {
        self.call_stack.len()
    }

    pub fn get_stack_trace(&self) -> Vec<StackFrame> {
        eprintln!(
            "[DEBUG] get_stack_trace called - call_stack.len() = {}",
            self.call_stack.len()
        );
        for (i, frame) in self.call_stack.iter().enumerate() {
            eprintln!(
                "[DEBUG]   Frame {}: func='{}', block_idx={}, inst_idx={}",
                i, frame.func_id.0, frame.block_idx, frame.inst_idx
            );
        }

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

    pub fn get_locals(&self) -> Vec<DebugVariable> {
        let Some(frame) = self.call_stack.last() else {
            return Vec::new();
        };

        frame
            .locals
            .iter()
            .map(|(&var_id, value)| {
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

    pub fn get_globals(&self) -> Vec<DebugVariable> {
        self.globals
            .iter()
            .map(|(name, value)| DebugVariable::new(name.clone(), value.clone(), true))
            .collect()
    }

    pub fn get_scopes(&self) -> Vec<DebugScope> {
        vec![
            DebugScope::new("Locals / محليات".to_string()).with_variables(self.get_locals()),
            DebugScope::new("Globals / عالميات".to_string()).with_variables(self.get_globals()),
        ]
    }

    /// Returns all heap-allocated objects (strings, arrays, objects) for memory inspection
    pub fn get_heap_allocations(&self) -> Vec<HeapAllocation> {
        use std::collections::HashSet;
        use std::rc::Rc;

        let mut allocations = Vec::new();
        let mut seen_addresses: HashSet<usize> = HashSet::new();
        let mut allocation_id: u64 = 1;

        // Helper to estimate size of a value
        fn estimate_size(value: &Value) -> usize {
            match value {
                Value::String(s) => std::mem::size_of::<Rc<String>>() + s.len(),
                Value::Array(arr) => {
                    let arr_ref = arr.borrow();
                    std::mem::size_of_val(&*arr_ref)
                        + arr_ref.iter().map(estimate_size).sum::<usize>()
                }
                Value::Object(obj) => {
                    let obj_ref = obj.borrow();
                    std::mem::size_of_val(&*obj_ref)
                        + obj_ref
                            .fields
                            .iter()
                            .map(|(k, v)| k.len() + estimate_size(v))
                            .sum::<usize>()
                }
                Value::Int(_) => 8,
                Value::Float(_) => 8,
                Value::Bool(_) => 1,
                Value::Null => 0,
                _ => std::mem::size_of::<Value>(),
            }
        }

        // Process a value and add to allocations if heap-allocated
        let mut process_value = |name: &str, value: &Value, id: &mut u64| {
            match value {
                Value::String(s) => {
                    let addr = Rc::as_ptr(s) as usize;
                    if seen_addresses.insert(addr) {
                        let alloc = HeapAllocation::new(
                            *id,
                            "String".to_string(),
                            "نص".to_string(),
                            estimate_size(value),
                            format!("{:#x}", addr),
                            Rc::strong_count(s) as u32,
                            name.to_string(),
                        );
                        allocations.push(alloc);
                        *id += 1;
                    }
                }
                Value::Array(arr) => {
                    let addr = Rc::as_ptr(arr) as *const () as usize;
                    if seen_addresses.insert(addr) {
                        let arr_ref = arr.borrow();
                        let children: Vec<HeapChild> = arr_ref
                            .iter()
                            .enumerate()
                            .take(10) // Limit children for display
                            .map(|(i, v)| {
                                HeapChild::new(
                                    format!("[{}]", i),
                                    v.to_display_string(),
                                    v.type_name().to_string(),
                                    v.type_name_ar().to_string(),
                                )
                            })
                            .collect();

                        let mut alloc = HeapAllocation::new(
                            *id,
                            "Array".to_string(),
                            "مصفوفة".to_string(),
                            estimate_size(value),
                            format!("{:#x}", addr),
                            Rc::strong_count(arr) as u32,
                            name.to_string(),
                        );
                        if arr_ref.len() > 10 {
                            let mut children = children;
                            children.push(HeapChild::new(
                                "...".to_string(),
                                format!("{} عناصر أخرى", arr_ref.len() - 10),
                                "".to_string(),
                                "".to_string(),
                            ));
                            alloc = alloc.with_children(children);
                        } else {
                            alloc = alloc.with_children(children);
                        }
                        allocations.push(alloc);
                        *id += 1;
                    }
                }
                Value::Object(obj) => {
                    let addr = Rc::as_ptr(obj) as *const () as usize;
                    if seen_addresses.insert(addr) {
                        let obj_ref = obj.borrow();
                        let children: Vec<HeapChild> = obj_ref
                            .fields
                            .iter()
                            .take(20) // Limit fields for display
                            .map(|(field_name, field_value)| {
                                HeapChild::new(
                                    field_name.clone(),
                                    field_value.to_display_string(),
                                    field_value.type_name().to_string(),
                                    field_value.type_name_ar().to_string(),
                                )
                            })
                            .collect();

                        let alloc = HeapAllocation::new(
                            *id,
                            format!("Object ({})", obj_ref.class_id.0),
                            format!("كائن ({})", obj_ref.class_id.0),
                            estimate_size(value),
                            format!("{:#x}", addr),
                            Rc::strong_count(obj) as u32,
                            name.to_string(),
                        )
                        .with_children(children);
                        allocations.push(alloc);
                        *id += 1;
                    }
                }
                _ => {} // Primitive types are not heap-allocated
            }
        };

        // Process globals
        for (name, value) in &self.globals {
            process_value(name, value, &mut allocation_id);
        }

        // Process current frame locals
        if let Some(frame) = self.call_stack.last() {
            for (&var_id, value) in &frame.locals {
                let name = self
                    .context
                    .source_map()
                    .get_variable_info(&frame.func_id, VarId(var_id))
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| format!("%{}", var_id));
                process_value(&name, value, &mut allocation_id);
            }
        }

        allocations
    }

    pub fn evaluate(&self, expression: &str) -> DebugResult<Value> {
        let frame = self.call_stack.last();

        if let Some(frame) = frame {
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

            if let Some(stripped) = expression.strip_prefix('%') {
                if let Ok(var_id) = stripped.parse::<u32>() {
                    if let Some(value) = frame.locals.get(&var_id) {
                        return Ok(value.clone());
                    }
                }
            }
        }

        if let Some(value) = self.globals.get(expression) {
            return Ok(value.clone());
        }

        Err(DebugError::new(format!(
            "لا يمكن تقييم التعبير: {}",
            expression
        )))
    }

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

        Err(DebugError::new("الدالة الرئيسية غير موجودة"))
    }

    /// Mirrors `Interpreter::resolve_virtual_method` so stepping through an
    /// inherited or overridden method call in the debugger matches normal
    /// execution instead of diverging from it.
    fn resolve_virtual_method(&mut self, obj_val: &Value, method: &MethodId) -> FuncId {
        if let Value::Object(obj) = obj_val {
            let runtime_class = obj.borrow().class_id.clone();
            let cache_key = (runtime_class.0.clone(), method.name.clone());
            if let Some(cached) = self.virtual_method_cache.get(&cache_key) {
                return cached.clone();
            }

            let mut current = Some(runtime_class);
            let mut visited = std::collections::HashSet::new();
            while let Some(class_id) = current {
                if !visited.insert(class_id.0.clone()) {
                    break; // cyclic inheritance is rejected by semantic analysis; don't hang here
                }
                let candidate = FuncId(format!("{}::{}", class_id.0, method.name));
                if self.module.get_function(&candidate).is_some() {
                    self.virtual_method_cache
                        .insert(cache_key, candidate.clone());
                    return candidate;
                }
                current = self
                    .module
                    .get_class(&class_id)
                    .and_then(|c| c.parent.clone());
            }
        }
        FuncId(format!("{}::{}", method.class.0, method.name))
    }

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

    fn execute_function(&mut self, func: &Function) -> RuntimeResult<Value> {
        if func.blocks.is_empty() {
            return Ok(Value::Null);
        }

        loop {
            let frame = self
                .call_stack
                .last()
                .ok_or_else(|| RuntimeError::internal("call stack unexpectedly empty"))?;
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
                        let new_block_idx = self.find_block_index(func, catch_block)?;
                        self.current_exception = Some(exception);
                        if let Some(frame) = self.call_stack.last_mut() {
                            frame.block_idx = new_block_idx;
                            frame.inst_idx = 0;
                        }
                    } else {
                        // Parked so the caller's `Call` site can convert this
                        // `Err` back into a throw against its own handler stack —
                        // the same fix as `interpreter::executor` (issue #181).
                        let msg = Self::exception_message(&exception);
                        self.current_exception = Some(exception);
                        return Err(RuntimeError::unhandled_exception(&msg));
                    }
                }
            }
        }
    }

    fn find_block_index(&self, func: &Function, block_id: BlockId) -> RuntimeResult<usize> {
        func.blocks
            .iter()
            .position(|b| b.id == block_id)
            .ok_or_else(|| RuntimeError::internal(format!("Block {} not found", block_id)))
    }

    fn execute_block(&mut self, block: &BasicBlock, func: &Function) -> RuntimeResult<BlockResult> {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.inst_idx = 0;
        }

        for (idx, inst) in block.instructions.iter().enumerate() {
            if let Some(frame) = self.call_stack.last_mut() {
                frame.inst_idx = idx;
            }

            if let Some(location) = self.get_current_location() {
                if self
                    .context
                    .has_breakpoint_at(&location.file, location.line)
                {}
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

            Instruction::BoolToInt { dest, src } => {
                let val = self.get_local(*src)?;
                let result = Value::Int(if val.is_truthy() { 1 } else { 0 });
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

                // A declared function of the same name wins: built-ins are the
                // last tier of the lookup order (#262). Decided by the IR builder so all
                // backends answer identically.
                let outcome = if Self::is_builtin(&func.0) && !self.module.shadows_builtin(&func.0)
                {
                    self.call_builtin(&func.0, arg_values)
                } else {
                    self.call_function(func, arg_values)
                };

                self.finish_call(outcome, dest.as_ref())
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

                let outcome = self.call_function(&FuncId(func_name), arg_values);

                self.finish_call(outcome, dest.as_ref())
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
                virtual_dispatch,
                ..
            } => {
                let obj_val = self.get_local(*object)?;
                let mut arg_values = vec![obj_val.clone()];
                for arg in args {
                    arg_values.push(self.get_local(*arg)?);
                }

                let method_func_id = if *virtual_dispatch {
                    self.resolve_virtual_method(&obj_val, method)
                } else {
                    FuncId(format!("{}::{}", method.class.0, method.name))
                };
                let outcome = self.call_function(&method_func_id, arg_values);

                self.finish_call(outcome, dest.as_ref())
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
                let outcome = self.call_function(&method_func_id, arg_values);

                self.finish_call(outcome, dest.as_ref())
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

            Instruction::ArrayPop {
                dest,
                array,
                elem_ty,
            } => {
                let arr_val = self.get_local(*array)?;

                // Mirrors the executor's arm exactly; the two are hand-kept in
                // step and nothing tests them against each other.
                let result = match arr_val {
                    Value::Array(arr) => arr
                        .borrow_mut()
                        .pop()
                        .unwrap_or_else(|| Self::element_zero(elem_ty)),
                    Value::Null => Self::element_zero(elem_ty),
                    _ => return Err(RuntimeError::type_error("array", arr_val.type_name())),
                };

                self.set_local(*dest, result);
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

                self.context.add_output(output.clone());

                println!("{}", output);
                io::stdout().flush().ok();

                Ok(InstructionResult::Continue)
            }

            Instruction::Copy { dest, src, ty: _ } => {
                let value = self.get_local(*src)?.clone();
                self.set_local(*dest, value);
                Ok(InstructionResult::Continue)
            }

            Instruction::Nop => Ok(InstructionResult::Continue),

            Instruction::NewEnumVariant {
                dest,
                variant,
                fields,
            } => {
                let mut field_values = Vec::new();
                for f in fields {
                    field_values.push(self.get_local(*f)?.clone());
                }
                let enum_value = Value::Enum {
                    enum_name: variant.enum_id.0.clone(),
                    variant_name: variant.name.clone(),
                    discriminant: variant.discriminant as i64,
                    fields: field_values,
                };
                self.set_local(*dest, enum_value);
                Ok(InstructionResult::Continue)
            }
            Instruction::GetDiscriminant { dest, value } => {
                let val = self.get_local(*value)?;
                if let Value::Enum { discriminant, .. } = val {
                    self.set_local(*dest, Value::Int(discriminant));
                    Ok(InstructionResult::Continue)
                } else {
                    Err(RuntimeError::type_error("enum", val.type_name()))
                }
            }
            Instruction::GetVariantField {
                dest,
                value,
                field_index,
                ..
            } => {
                let val = self.get_local(*value)?;
                if let Value::Enum { fields, .. } = val {
                    if let Some(field_val) = fields.get(*field_index as usize) {
                        self.set_local(*dest, field_val.clone());
                        Ok(InstructionResult::Continue)
                    } else {
                        Err(RuntimeError::index_out_of_bounds(
                            *field_index as i64,
                            fields.len(),
                        ))
                    }
                } else {
                    Err(RuntimeError::type_error("enum", val.type_name()))
                }
            }
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
            Constant::Function(name) => Value::Function(name.clone()),
        }
    }

    /// What `احذف_آخر` answers when there is nothing to remove — the twin of
    /// `Interpreter::element_zero`, and of the eight zero bytes codegen loads.
    fn element_zero(elem_ty: &IrType) -> Value {
        match elem_ty {
            IrType::Int => Value::Int(0),
            IrType::Float => Value::Float(0.0),
            IrType::Bool => Value::Bool(false),
            IrType::String => Value::string(String::new()),
            _ => Value::Null,
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

    /// Bind a callee's result, or turn a throw the callee did not handle into one
    /// this frame's `try_stack` can catch.
    ///
    /// Kept identical to `interpreter::executor::Interpreter::finish_call`. This
    /// file duplicates the main interpreter's instruction handling (issue #223),
    /// so `tarqeem debug` would otherwise abort on an exception that
    /// `tarqeem run` catches — a debug-vs-run divergence (issue #181).
    fn finish_call(
        &mut self,
        outcome: RuntimeResult<Value>,
        dest: Option<&VarId>,
    ) -> RuntimeResult<InstructionResult> {
        match outcome {
            Ok(result) => {
                if let Some(d) = dest {
                    self.set_local(*d, result);
                }
                Ok(InstructionResult::Continue)
            }
            Err(err) => match self.take_propagating_exception(&err) {
                Some(exception) => Ok(InstructionResult::Throw(exception)),
                None => Err(err),
            },
        }
    }

    fn take_propagating_exception(&mut self, err: &RuntimeError) -> Option<Value> {
        if err.kind == ErrorKind::UnhandledException {
            self.current_exception.take()
        } else {
            None
        }
    }

    /// An uncaught exception's `رسالة`, rather than `<استثناء>`. Mirrors
    /// `interpreter::executor::Interpreter::exception_message`.
    ///
    /// `pub(crate)` because the DAP adapter renders `StepResult::Exception`
    /// itself on the step-driven path, which never passes through the recursive
    /// `execute` loop below (issue #181).
    pub(crate) fn exception_message(exception: &Value) -> String {
        if let Value::Object(obj) = exception {
            if let Some(message) = obj.borrow().fields.get(EXCEPTION_MESSAGE_FIELD) {
                return message.to_display_string();
            }
        }

        exception.to_display_string()
    }
}

enum BlockResult {
    Continue,
    Jump(BlockId),
    Return(Value),
    Throw(Value),
}

enum InstructionResult {
    Continue,
    Jump(BlockId),
    Return(Value),
    Throw(Value),
}
