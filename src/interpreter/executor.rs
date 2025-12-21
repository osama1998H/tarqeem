//! Core interpreter execution engine.
//!
//! This module implements the IR interpreter that executes Tarqeem programs
//! by walking through IR instructions and maintaining execution state.

use std::collections::HashMap;
use std::io::{self, Write};

use crate::ir::{
    BasicBlock, BinaryOp, BlockId, Constant, FuncId, Function, Instruction, IrType, Module,
    UnaryOp, VarId,
};

use super::error::{RuntimeError, RuntimeResult};
use super::value::Value;

/// Maximum call stack depth to prevent stack overflow.
const MAX_STACK_DEPTH: usize = 1000;

/// A call frame representing a function invocation.
#[derive(Debug)]
struct CallFrame {
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

impl CallFrame {
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

/// The Tarqeem interpreter.
///
/// Executes IR modules by interpreting instructions directly.
pub struct Interpreter {
    /// The IR module to execute
    module: Module,
    /// Call stack
    call_stack: Vec<CallFrame>,
    /// Global variables
    globals: HashMap<String, Value>,
    /// Current exception (if any)
    current_exception: Option<Value>,
    /// Output buffer (for testing)
    output: Vec<String>,
    /// Whether to capture output instead of printing
    capture_output: bool,
}

impl Interpreter {
    /// Create a new interpreter for the given module.
    pub fn new(module: Module) -> Self {
        Self {
            module,
            call_stack: Vec::new(),
            globals: HashMap::new(),
            current_exception: None,
            output: Vec::new(),
            capture_output: false,
        }
    }

    /// Enable output capture (useful for testing).
    pub fn capture_output(&mut self, capture: bool) {
        self.capture_output = capture;
    }

    /// Get captured output.
    pub fn get_output(&self) -> &[String] {
        &self.output
    }

    /// Run the program starting from the main function.
    ///
    /// Looks for a function named "main" or "رئيسية" and executes it.
    pub fn run(&mut self) -> RuntimeResult<Value> {
        // Initialize globals
        self.init_globals()?;

        // Find main function
        let main_func = self.find_main_function()?;

        // Execute main
        self.call_function(&main_func, vec![])
    }

    /// Initialize global variables.
    fn init_globals(&mut self) -> RuntimeResult<()> {
        for (name, _ty, init) in &self.module.globals {
            let value = match init {
                Some(c) => self.constant_to_value(c),
                None => Value::Null,
            };
            self.globals.insert(name.clone(), value);
        }
        Ok(())
    }

    /// Find the main function.
    fn find_main_function(&self) -> RuntimeResult<FuncId> {
        // Try common main function names
        // __main__ is the auto-generated wrapper for top-level statements
        let main_names = ["__main__", "main", "رئيسية", "البداية"];

        for name in main_names {
            let func_id = FuncId(name.to_string());
            if self.module.get_function(&func_id).is_some() {
                return Ok(func_id);
            }
        }

        // If no main, try to find a top-level function or execute all statements
        if let Some(func) = self.module.functions.first() {
            return Ok(func.id.clone());
        }

        Err(RuntimeError::undefined_function("main/رئيسية"))
    }

    /// Call a function with arguments.
    pub fn call_function(&mut self, func_id: &FuncId, args: Vec<Value>) -> RuntimeResult<Value> {
        // Check stack depth
        if self.call_stack.len() >= MAX_STACK_DEPTH {
            return Err(RuntimeError::stack_overflow());
        }

        // Get the function
        let func = self
            .module
            .get_function(func_id)
            .ok_or_else(|| RuntimeError::undefined_function(&func_id.0))?
            .clone();

        // Create a new call frame
        let mut frame = CallFrame::new(func_id.clone());

        // Bind parameters
        for (i, param) in func.params.iter().enumerate() {
            let value = args.get(i).cloned().unwrap_or(Value::Null);
            frame.locals.insert(param.id.0, value);
        }

        // Push frame
        self.call_stack.push(frame);

        // Execute function
        let result = self.execute_function(&func);

        // Pop frame
        self.call_stack.pop();

        result
    }

    /// Execute a function.
    fn execute_function(&mut self, func: &Function) -> RuntimeResult<Value> {
        if func.blocks.is_empty() {
            return Ok(Value::Null);
        }

        // Start at entry block
        let mut block_idx = 0;

        loop {
            let block = &func.blocks[block_idx];
            let result = self.execute_block(block, func)?;

            match result {
                BlockResult::Continue => {
                    // Move to next block (shouldn't happen normally)
                    block_idx += 1;
                    if block_idx >= func.blocks.len() {
                        return Ok(Value::Null);
                    }
                }
                BlockResult::Jump(target) => {
                    // Update previous block for Phi resolution
                    if let Some(frame) = self.call_stack.last_mut() {
                        frame.prev_block = Some(block.id);
                    }
                    // Find target block
                    block_idx = self.find_block_index(func, target)?;
                }
                BlockResult::Return(value) => {
                    return Ok(value);
                }
                BlockResult::Throw(exception) => {
                    // Check for exception handler
                    if let Some(catch_block) = self.pop_try_block() {
                        self.current_exception = Some(exception);
                        block_idx = self.find_block_index(func, catch_block)?;
                    } else {
                        // Unhandled exception
                        let msg = exception.to_display_string();
                        return Err(RuntimeError::unhandled_exception(&msg));
                    }
                }
            }
        }
    }

    /// Find block index by ID.
    fn find_block_index(&self, func: &Function, block_id: BlockId) -> RuntimeResult<usize> {
        func.blocks
            .iter()
            .position(|b| b.id == block_id)
            .ok_or_else(|| RuntimeError::internal(format!("Block {} not found", block_id)))
    }

    /// Execute a basic block.
    fn execute_block(&mut self, block: &BasicBlock, func: &Function) -> RuntimeResult<BlockResult> {
        for inst in &block.instructions {
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

    /// Execute a single instruction.
    fn execute_instruction(
        &mut self,
        inst: &Instruction,
        _func: &Function,
    ) -> RuntimeResult<InstructionResult> {
        match inst {
            // Constants
            Instruction::Const { dest, value, .. } => {
                let val = self.constant_to_value(value);
                self.set_local(*dest, val);
                Ok(InstructionResult::Continue)
            }

            // Binary operations
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

            // Unary operations
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

            // Type conversions
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
                // For interpreter, bitcast is a no-op (same value, different type view)
                let val = self.get_local(*src)?;
                self.set_local(*dest, val);
                Ok(InstructionResult::Continue)
            }

            // Memory operations
            Instruction::Alloca { dest, .. } => {
                // Create a pointer to a null value
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

                // For arrays, get element pointer
                match ptr_val {
                    Value::Array(arr) => {
                        let arr_ref = arr.borrow();
                        if idx < 0 || (idx as usize) >= arr_ref.len() {
                            return Err(RuntimeError::index_out_of_bounds(idx, arr_ref.len()));
                        }
                        // Return pointer to element (simplified as the value itself for now)
                        let elem = arr_ref[idx as usize].clone();
                        self.set_local(*dest, Value::ptr(elem));
                    }
                    _ => {
                        // For other pointers, just return the pointer offset (simplified)
                        self.set_local(*dest, ptr_val);
                    }
                }
                Ok(InstructionResult::Continue)
            }

            // Control flow
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

            // Function calls
            Instruction::Call {
                dest, func, args, ..
            } => {
                // Collect argument values
                let arg_values: Vec<Value> = args
                    .iter()
                    .map(|v| self.get_local(*v))
                    .collect::<RuntimeResult<Vec<_>>>()?;

                // Check for built-in functions first
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

            // Objects
            Instruction::NewObject { dest, class } => {
                let obj = Value::object(class.clone());
                // Initialize fields from class definition
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

                // Collect arguments (including self)
                let mut arg_values = vec![obj_val.clone()];
                for arg in args {
                    arg_values.push(self.get_local(*arg)?);
                }

                // Build method function ID
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

                // Get actual class from object
                let class_id = match &obj_val {
                    Value::Object(obj) => obj.borrow().class_id.clone(),
                    _ => return Err(RuntimeError::type_error("object", obj_val.type_name())),
                };

                // Look up method in vtable
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

                // Collect arguments
                let mut arg_values = vec![obj_val];
                for arg in args {
                    arg_values.push(self.get_local(*arg)?);
                }

                // Call method
                let method_func_id = FuncId(format!("{}::{}", method_id.class.0, method_id.name));
                let result = self.call_function(&method_func_id, arg_values)?;

                if let Some(d) = dest {
                    self.set_local(*d, result);
                }
                Ok(InstructionResult::Continue)
            }

            // Arrays
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

            // Strings
            Instruction::StringConcat { dest, left, right } => {
                let left_val = self.get_local(*left)?;
                let right_val = self.get_local(*right)?;

                let left_str = left_val.to_display_string();
                let right_str = right_val.to_display_string();

                self.set_local(*dest, Value::string(left_str + &right_str));
                Ok(InstructionResult::Continue)
            }

            // Exception handling
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

            // Phi functions
            Instruction::Phi { dest, incoming, .. } => {
                let prev_block = self
                    .call_stack
                    .last()
                    .and_then(|f| f.prev_block)
                    .unwrap_or(BlockId(0));

                // Find the value from the previous block
                let value = incoming
                    .iter()
                    .find(|(_, block)| *block == prev_block)
                    .map(|(var, _)| self.get_local(*var))
                    .transpose()?
                    .unwrap_or(Value::Null);

                self.set_local(*dest, value);
                Ok(InstructionResult::Continue)
            }

            // Print
            Instruction::Print { value } => {
                let val = self.get_local(*value)?;
                let output = val.to_display_string();

                if self.capture_output {
                    self.output.push(output);
                } else {
                    println!("{}", output);
                    io::stdout().flush().ok();
                }
                Ok(InstructionResult::Continue)
            }

            // Nop
            Instruction::Nop => Ok(InstructionResult::Continue),
        }
    }

    /// Execute a binary operation.
    fn execute_binary_op(
        &self,
        op: BinaryOp,
        left: Value,
        right: Value,
        _ty: &IrType,
    ) -> RuntimeResult<Value> {
        match op {
            // Arithmetic
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
            BinaryOp::Div => {
                // Check for division by zero
                match (&left, &right) {
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
                }
            }
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

            // Comparison
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

            // Logical
            BinaryOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            BinaryOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),

            // Bitwise
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

    /// Execute a unary operation.
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

    /// Convert a constant to a value.
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

    /// Get a local variable value.
    fn get_local(&self, var: VarId) -> RuntimeResult<Value> {
        self.call_stack
            .last()
            .and_then(|frame| frame.locals.get(&var.0))
            .cloned()
            .ok_or_else(|| RuntimeError::undefined_variable(&format!("%{}", var.0)))
    }

    /// Set a local variable value.
    fn set_local(&mut self, var: VarId, value: Value) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.locals.insert(var.0, value);
        }
    }

    /// Pop a try block from the current frame.
    fn pop_try_block(&mut self) -> Option<BlockId> {
        self.call_stack
            .last_mut()
            .and_then(|frame| frame.try_stack.pop())
    }

    /// Check if a function is a built-in.
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

    /// Call a built-in function.
    fn call_builtin(&mut self, name: &str, args: Vec<Value>) -> RuntimeResult<Value> {
        match name {
            "print" | "اطبع" | "println" => {
                let output = args
                    .iter()
                    .map(|v| v.to_display_string())
                    .collect::<Vec<_>>()
                    .join(" ");

                if self.capture_output {
                    self.output.push(output);
                } else {
                    println!("{}", output);
                    io::stdout().flush().ok();
                }
                Ok(Value::Null)
            }

            "input" | "ادخل" => {
                // Print prompt if provided
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

/// Result of executing a block.
enum BlockResult {
    /// Continue to next instruction (should not happen with proper terminators)
    Continue,
    /// Jump to target block
    Jump(BlockId),
    /// Return from function
    Return(Value),
    /// Throw exception
    Throw(Value),
}

/// Result of executing an instruction.
enum InstructionResult {
    /// Continue to next instruction
    Continue,
    /// Jump to target block
    Jump(BlockId),
    /// Return from function
    Return(Value),
    /// Throw exception
    Throw(Value),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BasicBlock, Function, Module, Parameter};

    fn create_simple_module() -> Module {
        let mut module = Module::new("test".to_string());

        // Create main function: return 42
        let mut main_func = Function::new(
            FuncId("main".to_string()),
            "main".to_string(),
            vec![],
            IrType::Int,
        );

        let mut entry_block = BasicBlock::new(BlockId(0));
        entry_block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Int(42),
            ty: IrType::Int,
        });
        entry_block.instructions.push(Instruction::Return {
            value: Some(VarId(0)),
        });

        main_func.blocks.push(entry_block);
        module.functions.push(main_func);

        module
    }

    #[test]
    fn test_simple_return() {
        let module = create_simple_module();
        let mut interpreter = Interpreter::new(module);
        let result = interpreter.run().unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn test_arithmetic() {
        let mut module = Module::new("test".to_string());

        // Create main: return 10 + 5
        let mut main_func = Function::new(
            FuncId("main".to_string()),
            "main".to_string(),
            vec![],
            IrType::Int,
        );

        let mut entry_block = BasicBlock::new(BlockId(0));
        entry_block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Int(10),
            ty: IrType::Int,
        });
        entry_block.instructions.push(Instruction::Const {
            dest: VarId(1),
            value: Constant::Int(5),
            ty: IrType::Int,
        });
        entry_block.instructions.push(Instruction::Binary {
            dest: VarId(2),
            op: BinaryOp::Add,
            left: VarId(0),
            right: VarId(1),
            ty: IrType::Int,
        });
        entry_block.instructions.push(Instruction::Return {
            value: Some(VarId(2)),
        });

        main_func.blocks.push(entry_block);
        module.functions.push(main_func);

        let mut interpreter = Interpreter::new(module);
        let result = interpreter.run().unwrap();
        assert_eq!(result, Value::Int(15));
    }

    #[test]
    fn test_print_builtin() {
        let mut module = Module::new("test".to_string());
        module.strings.add("Hello, World!".to_string());

        let mut main_func = Function::new(
            FuncId("main".to_string()),
            "main".to_string(),
            vec![],
            IrType::Void,
        );

        let mut entry_block = BasicBlock::new(BlockId(0));
        entry_block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::String(0),
            ty: IrType::String,
        });
        entry_block.instructions.push(Instruction::Call {
            dest: None,
            func: FuncId("اطبع".to_string()),
            args: vec![VarId(0)],
            ret_ty: IrType::Void,
        });
        entry_block
            .instructions
            .push(Instruction::Return { value: None });

        main_func.blocks.push(entry_block);
        module.functions.push(main_func);

        let mut interpreter = Interpreter::new(module);
        interpreter.capture_output(true);
        interpreter.run().unwrap();

        assert_eq!(interpreter.get_output(), &["Hello, World!"]);
    }

    #[test]
    fn test_branch() {
        let mut module = Module::new("test".to_string());

        // Create main: if true { return 1 } else { return 0 }
        let mut main_func = Function::new(
            FuncId("main".to_string()),
            "main".to_string(),
            vec![],
            IrType::Int,
        );

        // Entry block
        let mut entry_block = BasicBlock::new(BlockId(0));
        entry_block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Bool(true),
            ty: IrType::Bool,
        });
        entry_block.instructions.push(Instruction::Branch {
            cond: VarId(0),
            then_block: BlockId(1),
            else_block: BlockId(2),
        });

        // Then block
        let mut then_block = BasicBlock::new(BlockId(1));
        then_block.instructions.push(Instruction::Const {
            dest: VarId(1),
            value: Constant::Int(1),
            ty: IrType::Int,
        });
        then_block.instructions.push(Instruction::Return {
            value: Some(VarId(1)),
        });

        // Else block
        let mut else_block = BasicBlock::new(BlockId(2));
        else_block.instructions.push(Instruction::Const {
            dest: VarId(2),
            value: Constant::Int(0),
            ty: IrType::Int,
        });
        else_block.instructions.push(Instruction::Return {
            value: Some(VarId(2)),
        });

        main_func.blocks.push(entry_block);
        main_func.blocks.push(then_block);
        main_func.blocks.push(else_block);
        module.functions.push(main_func);

        let mut interpreter = Interpreter::new(module);
        let result = interpreter.run().unwrap();
        assert_eq!(result, Value::Int(1));
    }

    #[test]
    fn test_function_call() {
        let mut module = Module::new("test".to_string());

        // Create add function: fn add(a, b) { return a + b }
        let mut add_func = Function::new(
            FuncId("add".to_string()),
            "add".to_string(),
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

        let mut add_entry = BasicBlock::new(BlockId(0));
        add_entry.instructions.push(Instruction::Binary {
            dest: VarId(2),
            op: BinaryOp::Add,
            left: VarId(0),
            right: VarId(1),
            ty: IrType::Int,
        });
        add_entry.instructions.push(Instruction::Return {
            value: Some(VarId(2)),
        });
        add_func.blocks.push(add_entry);

        // Create main function: return add(10, 20)
        let mut main_func = Function::new(
            FuncId("main".to_string()),
            "main".to_string(),
            vec![],
            IrType::Int,
        );

        let mut main_entry = BasicBlock::new(BlockId(0));
        main_entry.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Int(10),
            ty: IrType::Int,
        });
        main_entry.instructions.push(Instruction::Const {
            dest: VarId(1),
            value: Constant::Int(20),
            ty: IrType::Int,
        });
        main_entry.instructions.push(Instruction::Call {
            dest: Some(VarId(2)),
            func: FuncId("add".to_string()),
            args: vec![VarId(0), VarId(1)],
            ret_ty: IrType::Int,
        });
        main_entry.instructions.push(Instruction::Return {
            value: Some(VarId(2)),
        });
        main_func.blocks.push(main_entry);

        module.functions.push(add_func);
        module.functions.push(main_func);

        let mut interpreter = Interpreter::new(module);
        let result = interpreter.run().unwrap();
        assert_eq!(result, Value::Int(30));
    }

    #[test]
    fn test_array_operations() {
        let mut module = Module::new("test".to_string());

        // Create main: arr = [1, 2, 3]; return arr[1]
        let mut main_func = Function::new(
            FuncId("main".to_string()),
            "main".to_string(),
            vec![],
            IrType::Int,
        );

        let mut entry_block = BasicBlock::new(BlockId(0));

        // Create array elements
        entry_block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Int(1),
            ty: IrType::Int,
        });
        entry_block.instructions.push(Instruction::Const {
            dest: VarId(1),
            value: Constant::Int(2),
            ty: IrType::Int,
        });
        entry_block.instructions.push(Instruction::Const {
            dest: VarId(2),
            value: Constant::Int(3),
            ty: IrType::Int,
        });

        // Create array
        entry_block.instructions.push(Instruction::NewArray {
            dest: VarId(3),
            elem_ty: IrType::Int,
            elements: vec![VarId(0), VarId(1), VarId(2)],
        });

        // Get arr[1]
        entry_block.instructions.push(Instruction::Const {
            dest: VarId(4),
            value: Constant::Int(1),
            ty: IrType::Int,
        });
        entry_block.instructions.push(Instruction::ArrayGet {
            dest: VarId(5),
            array: VarId(3),
            index: VarId(4),
            elem_ty: IrType::Int,
        });

        entry_block.instructions.push(Instruction::Return {
            value: Some(VarId(5)),
        });

        main_func.blocks.push(entry_block);
        module.functions.push(main_func);

        let mut interpreter = Interpreter::new(module);
        let result = interpreter.run().unwrap();
        assert_eq!(result, Value::Int(2));
    }
}
