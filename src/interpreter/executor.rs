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

const MAX_STACK_DEPTH: usize = 1000;

#[allow(dead_code)]
#[derive(Debug)]
struct CallFrame {
    func_id: FuncId,
    block_idx: usize,
    inst_idx: usize,
    locals: HashMap<u32, Value>,
    prev_block: Option<BlockId>,
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

pub struct Interpreter {
    module: Module,
    call_stack: Vec<CallFrame>,
    globals: HashMap<String, Value>,
    current_exception: Option<Value>,
    output: Vec<String>,
    capture_output: bool,
}

impl Interpreter {
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

    pub fn capture_output(&mut self, capture: bool) {
        self.capture_output = capture;
    }

    pub fn get_output(&self) -> &[String] {
        &self.output
    }

    pub fn run(&mut self) -> RuntimeResult<Value> {
        self.init_globals()?;

        let main_func = self.find_main_function()?;

        self.call_function(&main_func, vec![])
    }

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

    fn find_main_function(&self) -> RuntimeResult<FuncId> {
        let main_names = ["__main__", "main", "رئيسي", "رئيسية", "البداية"];

        for name in main_names {
            let func_id = FuncId(name.to_string());
            if self.module.get_function(&func_id).is_some() {
                return Ok(func_id);
            }
        }

        if let Some(func) = self.module.functions.first() {
            return Ok(func.id.clone());
        }

        Err(RuntimeError::undefined_function("main/رئيسي/رئيسية"))
    }

    pub fn call_function(&mut self, func_id: &FuncId, args: Vec<Value>) -> RuntimeResult<Value> {
        if self.call_stack.len() >= MAX_STACK_DEPTH {
            return Err(RuntimeError::stack_overflow());
        }

        let func = self
            .module
            .get_function(func_id)
            .ok_or_else(|| RuntimeError::undefined_function(&func_id.0))?
            .clone();

        let mut frame = CallFrame::new(func_id.clone());

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

        let mut block_idx = 0;

        loop {
            let block = &func.blocks[block_idx];
            let result = self.execute_block(block, func)?;

            match result {
                BlockResult::Continue => {
                    block_idx += 1;
                    if block_idx >= func.blocks.len() {
                        return Ok(Value::Null);
                    }
                }
                BlockResult::Jump(target) => {
                    if let Some(frame) = self.call_stack.last_mut() {
                        frame.prev_block = Some(block.id);
                    }
                    block_idx = self.find_block_index(func, target)?;
                }
                BlockResult::Return(value) => {
                    return Ok(value);
                }
                BlockResult::Throw(exception) => {
                    if let Some(catch_block) = self.pop_try_block() {
                        self.current_exception = Some(exception);
                        block_idx = self.find_block_index(func, catch_block)?;
                    } else {
                        let msg = exception.to_display_string();
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

                if self.capture_output {
                    self.output.push(output);
                } else {
                    println!("{}", output);
                    io::stdout().flush().ok();
                }
                Ok(InstructionResult::Continue)
            }

            Instruction::Copy { dest, src, ty: _ } => {
                // Copy the value from src to dest
                let value = self.get_local(*src)?.clone();
                self.set_local(*dest, value);
                Ok(InstructionResult::Continue)
            }

            Instruction::Nop => Ok(InstructionResult::Continue),
        }
    }

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
                | "طباعة"
                | "اطبع_سطر"
                | "input"
                | "ادخل"
                | "ادخل_رسالة"
                | "input_prompt"
                | "ادخل_عدد"
                | "input_int"
                | "ادخل_عشري"
                | "input_float"
                | "len"
                | "طول"
                | "length"
                | "type"
                | "نوع"
                | "typeof"
                | "int"
                | "عدد"
                | "float"
                | "عدد_عشري"
                | "str"
                | "نص"
                | "string"
                | "bool"
                | "منطقي"
                | "abs"
                | "مطلق"
                | "pow"
                | "قوة"
                | "sqrt"
                | "جذر"
                | "cbrt"
                | "جذر_تكعيبي"
                | "log"
                | "لوغاريتم"
                | "log10"
                | "لوغ10"
                | "لوغاريتم10"
                | "log2"
                | "لوغ2"
                | "exp"
                | "أس"
                | "أسي"
                | "floor"
                | "أرضية"
                | "ceil"
                | "سقف"
                | "round"
                | "قرب"
                | "تقريب"
                | "trunc"
                | "اقتطع"
                | "min"
                | "أقل"
                | "أدنى"
                | "max"
                | "أكبر"
                | "أقصى"
                | "clamp"
                | "حصر"
                | "sign"
                | "علامة"
                | "gcd"
                | "قاسم_مشترك"
                | "lcm"
                | "مضاعف_مشترك"
                | "factorial"
                | "عاملي"
                | "sin"
                | "جا"
                | "جيب"
                | "cos"
                | "جتا"
                | "جيب_التمام"
                | "tan"
                | "ظا"
                | "ظل"
                | "cot"
                | "ظتا"
                | "ظل_التمام"
                | "sec"
                | "قا"
                | "قاطع"
                | "csc"
                | "قتا"
                | "قاطع_التمام"
                | "asin"
                | "جا_عكسي"
                | "جيب_عكسي"
                | "acos"
                | "جتا_عكسي"
                | "جيب_تمام_عكسي"
                | "atan"
                | "ظا_عكسي"
                | "ظل_عكسي"
                | "atan2"
                | "ظا_عكسي2"
                | "ظل_عكسي2"
                | "sinh"
                | "جا_زائدي"
                | "جيب_زائدي"
                | "cosh"
                | "جتا_زائدي"
                | "جيب_تمام_زائدي"
                | "tanh"
                | "ظا_زائدي"
                | "ظل_زائدي"
                | "to_radians"
                | "الى_راديان"
                | "راديان"
                | "to_degrees"
                | "الى_درجات"
                | "درجات"
                | "random"
                | "عشوائي"
                | "random_int"
                | "random_range"
                | "عشوائي_بين"
                | "random_float"
                | "عشوائي_عشري"
                | "random_bool"
                | "عشوائي_منطقي"
                | "assert"
                | "تأكد"
                | "assert_msg"
                | "تأكد_رسالة"
                | "panic"
                | "توقف"
                | "sleep"
                | "نم"
                | "time_now"
                | "وقت_الآن"
                | "trq_int_to_string"
                | "trq_float_to_string"
                | "trq_bool_to_string"
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

                if self.capture_output {
                    self.output.push(output);
                } else {
                    println!("{}", output);
                    io::stdout().flush().ok();
                }
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

            "abs" | "مطلق" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "abs() requires 1 argument",
                        "مطلق() تتطلب معامل واحد",
                    )
                })?;

                match val {
                    Value::Int(i) => Ok(Value::Int(i.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => Err(RuntimeError::type_error("numeric", val.type_name())),
                }
            }

            "pow" | "قوة" => {
                let base = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "pow() requires 2 arguments",
                        "قوة() تتطلب معاملين",
                    )
                })?;
                let exp = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "pow() requires 2 arguments",
                        "قوة() تتطلب معاملين",
                    )
                })?;

                match (base, exp) {
                    (Value::Int(b), Value::Int(e)) if *e >= 0 => Ok(Value::Int(b.pow(*e as u32))),
                    (Value::Int(b), Value::Int(e)) => Ok(Value::Float((*b as f64).powf(*e as f64))),
                    _ => {
                        let b = base
                            .as_float()
                            .ok_or_else(|| RuntimeError::type_error("numeric", base.type_name()))?;
                        let e = exp
                            .as_float()
                            .ok_or_else(|| RuntimeError::type_error("numeric", exp.type_name()))?;
                        Ok(Value::Float(b.powf(e)))
                    }
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

            "cbrt" | "جذر_تكعيبي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "cbrt() requires 1 argument",
                        "جذر_تكعيبي() تتطلب معامل واحد",
                    )
                })?;

                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.cbrt()))
            }

            "log" | "لوغاريتم" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "log() requires 1 argument",
                        "لوغاريتم() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.ln()))
            }

            "log10" | "لوغ10" | "لوغاريتم10" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "log10() requires 1 argument",
                        "لوغ10() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.log10()))
            }

            "log2" | "لوغ2" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "log2() requires 1 argument",
                        "لوغ2() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.log2()))
            }

            "exp" | "أس" | "أسي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "exp() requires 1 argument",
                        "أس() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.exp()))
            }

            "floor" | "أرضية" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "floor() requires 1 argument",
                        "أرضية() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.floor()))
            }

            "ceil" | "سقف" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "ceil() requires 1 argument",
                        "سقف() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.ceil()))
            }

            "round" | "قرب" | "تقريب" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "round() requires 1 argument",
                        "قرب() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.round()))
            }

            "trunc" | "اقتطع" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "trunc() requires 1 argument",
                        "اقتطع() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.trunc()))
            }

            "min" | "أقل" | "أدنى" => {
                let a = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "min() requires 2 arguments",
                        "أقل() تتطلب معاملين",
                    )
                })?;
                let b = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "min() requires 2 arguments",
                        "أقل() تتطلب معاملين",
                    )
                })?;

                match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Int(*x.min(y))),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.min(*y))),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Float((*x as f64).min(*y))),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x.min(*y as f64))),
                    _ => Err(RuntimeError::type_error("numeric", a.type_name())),
                }
            }

            "max" | "أكبر" | "أقصى" => {
                let a = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "max() requires 2 arguments",
                        "أكبر() تتطلب معاملين",
                    )
                })?;
                let b = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "max() requires 2 arguments",
                        "أكبر() تتطلب معاملين",
                    )
                })?;

                match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Int(*x.max(y))),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.max(*y))),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Float((*x as f64).max(*y))),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x.max(*y as f64))),
                    _ => Err(RuntimeError::type_error("numeric", a.type_name())),
                }
            }

            "clamp" | "حصر" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "clamp() requires 3 arguments",
                        "حصر() تتطلب ثلاثة معاملات",
                    )
                })?;
                let min_val = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "clamp() requires 3 arguments",
                        "حصر() تتطلب ثلاثة معاملات",
                    )
                })?;
                let max_val = args.get(2).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "clamp() requires 3 arguments",
                        "حصر() تتطلب ثلاثة معاملات",
                    )
                })?;

                match (val, min_val, max_val) {
                    (Value::Int(v), Value::Int(mn), Value::Int(mx)) => {
                        Ok(Value::Int(*v.max(mn).min(mx)))
                    }
                    _ => {
                        let v = val
                            .as_float()
                            .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                        let mn = min_val.as_float().ok_or_else(|| {
                            RuntimeError::type_error("numeric", min_val.type_name())
                        })?;
                        let mx = max_val.as_float().ok_or_else(|| {
                            RuntimeError::type_error("numeric", max_val.type_name())
                        })?;
                        Ok(Value::Float(v.max(mn).min(mx)))
                    }
                }
            }

            "sign" | "علامة" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sign() requires 1 argument",
                        "علامة() تتطلب معامل واحد",
                    )
                })?;

                match val {
                    Value::Int(i) => Ok(Value::Int(i.signum())),
                    Value::Float(f) => {
                        if f.is_nan() {
                            Ok(Value::Float(f64::NAN))
                        } else if *f > 0.0 {
                            Ok(Value::Float(1.0))
                        } else if *f < 0.0 {
                            Ok(Value::Float(-1.0))
                        } else {
                            Ok(Value::Float(0.0))
                        }
                    }
                    _ => Err(RuntimeError::type_error("numeric", val.type_name())),
                }
            }

            "gcd" | "قاسم_مشترك" => {
                let a = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "gcd() requires 2 arguments",
                        "قاسم_مشترك() تتطلب معاملين",
                    )
                })?;
                let b = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "gcd() requires 2 arguments",
                        "قاسم_مشترك() تتطلب معاملين",
                    )
                })?;

                let x = a
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", a.type_name()))?;
                let y = b
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", b.type_name()))?;

                fn gcd(mut a: i64, mut b: i64) -> i64 {
                    a = a.abs();
                    b = b.abs();
                    while b != 0 {
                        let t = b;
                        b = a % b;
                        a = t;
                    }
                    a
                }

                Ok(Value::Int(gcd(x, y)))
            }

            "lcm" | "مضاعف_مشترك" => {
                let a = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "lcm() requires 2 arguments",
                        "مضاعف_مشترك() تتطلب معاملين",
                    )
                })?;
                let b = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "lcm() requires 2 arguments",
                        "مضاعف_مشترك() تتطلب معاملين",
                    )
                })?;

                let x = a
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", a.type_name()))?;
                let y = b
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", b.type_name()))?;

                fn gcd(mut a: i64, mut b: i64) -> i64 {
                    a = a.abs();
                    b = b.abs();
                    while b != 0 {
                        let t = b;
                        b = a % b;
                        a = t;
                    }
                    a
                }

                if x == 0 || y == 0 {
                    Ok(Value::Int(0))
                } else {
                    Ok(Value::Int((x.abs() / gcd(x, y)) * y.abs()))
                }
            }

            "factorial" | "عاملي" => {
                let n = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "factorial() requires 1 argument",
                        "عاملي() تتطلب معامل واحد",
                    )
                })?;

                let n = n
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", n.type_name()))?;

                if n < 0 {
                    return Err(RuntimeError::invalid_operation(
                        "factorial() requires non-negative argument",
                        "عاملي() تتطلب عدد غير سالب",
                    ));
                }

                let mut result: i64 = 1;
                for i in 2..=n {
                    result = result.saturating_mul(i);
                }
                Ok(Value::Int(result))
            }

            "sin" | "جا" | "جيب" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sin() requires 1 argument",
                        "جيب() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sin()))
            }

            "cos" | "جتا" | "جيب_التمام" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "cos() requires 1 argument",
                        "جيب_التمام() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.cos()))
            }

            "tan" | "ظا" | "ظل" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "tan() requires 1 argument",
                        "ظل() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.tan()))
            }

            "cot" | "ظتا" | "ظل_التمام" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "cot() requires 1 argument",
                        "ظل_التمام() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(1.0 / f.tan()))
            }

            "sec" | "قا" | "قاطع" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sec() requires 1 argument",
                        "قاطع() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(1.0 / f.cos()))
            }

            "csc" | "قتا" | "قاطع_التمام" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "csc() requires 1 argument",
                        "قاطع_التمام() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(1.0 / f.sin()))
            }

            "asin" | "جا_عكسي" | "جيب_عكسي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "asin() requires 1 argument",
                        "جيب_عكسي() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.asin()))
            }

            "acos" | "جتا_عكسي" | "جيب_تمام_عكسي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "acos() requires 1 argument",
                        "جيب_تمام_عكسي() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.acos()))
            }

            "atan" | "ظا_عكسي" | "ظل_عكسي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "atan() requires 1 argument",
                        "ظل_عكسي() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.atan()))
            }

            "atan2" | "ظا_عكسي2" | "ظل_عكسي2" => {
                let y = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "atan2() requires 2 arguments",
                        "ظل_عكسي2() تتطلب معاملين",
                    )
                })?;
                let x = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "atan2() requires 2 arguments",
                        "ظل_عكسي2() تتطلب معاملين",
                    )
                })?;

                let y = y
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", y.type_name()))?;
                let x = x
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", x.type_name()))?;

                Ok(Value::Float(y.atan2(x)))
            }

            "sinh" | "جا_زائدي" | "جيب_زائدي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sinh() requires 1 argument",
                        "جيب_زائدي() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.sinh()))
            }

            "cosh" | "جتا_زائدي" | "جيب_تمام_زائدي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "cosh() requires 1 argument",
                        "جيب_تمام_زائدي() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.cosh()))
            }

            "tanh" | "ظا_زائدي" | "ظل_زائدي" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "tanh() requires 1 argument",
                        "ظل_زائدي() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.tanh()))
            }

            "to_radians" | "الى_راديان" | "راديان" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "to_radians() requires 1 argument",
                        "الى_راديان() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.to_radians()))
            }

            "to_degrees" | "الى_درجات" | "درجات" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "to_degrees() requires 1 argument",
                        "الى_درجات() تتطلب معامل واحد",
                    )
                })?;
                let f = val
                    .as_float()
                    .ok_or_else(|| RuntimeError::type_error("numeric", val.type_name()))?;
                Ok(Value::Float(f.to_degrees()))
            }

            "random" | "عشوائي" | "random_int" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(12345);
                let random = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                Ok(Value::Int((random % (i64::MAX as u64 + 1)) as i64))
            }

            "random_range" | "عشوائي_بين" => {
                let min_val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "random_range() requires 2 arguments",
                        "عشوائي_بين() تتطلب معاملين",
                    )
                })?;
                let max_val = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "random_range() requires 2 arguments",
                        "عشوائي_بين() تتطلب معاملين",
                    )
                })?;

                let min = min_val
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", min_val.type_name()))?;
                let max = max_val
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", max_val.type_name()))?;

                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(12345);
                let random = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let range = (max - min + 1) as u64;
                let result = min + (random % range) as i64;
                Ok(Value::Int(result))
            }

            "random_float" | "عشوائي_عشري" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(12345);
                let random = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let result = (random as f64) / (u64::MAX as f64);
                Ok(Value::Float(result))
            }

            "random_bool" | "عشوائي_منطقي" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(12345);
                let random = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                Ok(Value::Bool(random.is_multiple_of(2)))
            }

            "assert" | "تأكد" => {
                let cond = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "assert() requires 1 argument",
                        "تأكد() تتطلب معامل واحد",
                    )
                })?;

                if !cond.is_truthy() {
                    return Err(RuntimeError::invalid_operation(
                        "Assertion failed",
                        "فشل التأكيد",
                    ));
                }
                Ok(Value::Null)
            }

            "assert_msg" | "تأكد_رسالة" => {
                let cond = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "assert_msg() requires 2 arguments",
                        "تأكد_رسالة() تتطلب معاملين",
                    )
                })?;
                let msg = args.get(1).ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "assert_msg() requires 2 arguments",
                        "تأكد_رسالة() تتطلب معاملين",
                    )
                })?;

                if !cond.is_truthy() {
                    let msg_str = msg.to_display_string();
                    return Err(RuntimeError::invalid_operation(
                        format!("Assertion failed: {}", msg_str),
                        format!("فشل التأكيد: {}", msg_str),
                    ));
                }
                Ok(Value::Null)
            }

            "panic" | "توقف" => {
                let msg = args
                    .first()
                    .map(|v| v.to_display_string())
                    .unwrap_or_else(|| "Panic!".to_string());

                Err(RuntimeError::invalid_operation(
                    format!("Panic: {}", msg),
                    format!("توقف: {}", msg),
                ))
            }

            "sleep" | "نم" => {
                let ms = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "sleep() requires 1 argument (milliseconds)",
                        "نم() تتطلب معامل واحد (ميلي ثانية)",
                    )
                })?;

                let ms = ms
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_error("int", ms.type_name()))?;

                if ms > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
                }
                Ok(Value::Null)
            }

            "time_now" | "وقت_الآن" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                Ok(Value::Int(now))
            }

            "ادخل_رسالة" | "input_prompt" => {
                let prompt = args
                    .first()
                    .map(|v| v.to_display_string())
                    .unwrap_or_default();

                print!("{}", prompt);
                io::stdout().flush().ok();

                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| RuntimeError::internal(format!("Input error: {}", e)))?;

                Ok(Value::string(input.trim_end()))
            }

            "ادخل_عدد" | "input_int" => {
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| RuntimeError::internal(format!("Input error: {}", e)))?;

                input
                    .trim()
                    .parse::<i64>()
                    .map(Value::Int)
                    .map_err(|_| RuntimeError::type_error("integer input", "invalid input"))
            }

            "ادخل_عشري" | "input_float" => {
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| RuntimeError::internal(format!("Input error: {}", e)))?;

                input
                    .trim()
                    .parse::<f64>()
                    .map(Value::Float)
                    .map_err(|_| RuntimeError::type_error("float input", "invalid input"))
            }

            "trq_int_to_string" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "trq_int_to_string requires 1 argument",
                        "trq_int_to_string تتطلب معامل واحد",
                    )
                })?;
                match val {
                    Value::Int(n) => Ok(Value::string(n.to_string())),
                    Value::Float(f) => Ok(Value::string((*f as i64).to_string())),
                    _ => Err(RuntimeError::type_error("int", val.type_name())),
                }
            }

            "trq_float_to_string" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "trq_float_to_string requires 1 argument",
                        "trq_float_to_string تتطلب معامل واحد",
                    )
                })?;
                match val {
                    Value::Float(f) => Ok(Value::string(f.to_string())),
                    Value::Int(n) => Ok(Value::string((*n as f64).to_string())),
                    _ => Err(RuntimeError::type_error("float", val.type_name())),
                }
            }

            "trq_bool_to_string" => {
                let val = args.first().ok_or_else(|| {
                    RuntimeError::invalid_operation(
                        "trq_bool_to_string requires 1 argument",
                        "trq_bool_to_string تتطلب معامل واحد",
                    )
                })?;
                match val {
                    Value::Bool(b) => Ok(Value::string(if *b {
                        "صحيح".to_string()
                    } else {
                        "خطأ".to_string()
                    })),
                    _ => Err(RuntimeError::type_error("bool", val.type_name())),
                }
            }

            _ => Err(RuntimeError::undefined_function(name)),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BasicBlock, Function, Module, Parameter};

    fn create_simple_module() -> Module {
        let mut module = Module::new("test".to_string());

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

        let mut main_func = Function::new(
            FuncId("main".to_string()),
            "main".to_string(),
            vec![],
            IrType::Int,
        );

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

        let mut main_func = Function::new(
            FuncId("main".to_string()),
            "main".to_string(),
            vec![],
            IrType::Int,
        );

        let mut entry_block = BasicBlock::new(BlockId(0));

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

        entry_block.instructions.push(Instruction::NewArray {
            dest: VarId(3),
            elem_ty: IrType::Int,
            elements: vec![VarId(0), VarId(1), VarId(2)],
        });

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
